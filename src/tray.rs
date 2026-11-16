use std::cell::RefCell;
use std::env;
use std::path::Path;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use eframe::egui;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject, HBITMAP};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIIF_NOSOUND, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyIcon, DestroyMenu, DestroyWindow, DispatchMessageW, FindWindowW, GetCursorPos,
    GetMessageW, GetSystemMetrics, HICON, ICONINFO, IDI_APPLICATION, LoadIconW, MF_CHECKED,
    MF_GRAYED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG, PostMessageW, PostQuitMessage,
    RegisterClassW, RegisterWindowMessageW, SM_CXSMICON, SMTO_ABORTIFHUNG, SendMessageTimeoutW,
    SetForegroundWindow, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage,
    WM_APP, WM_CANCELMODE, WM_CLOSE, WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONDBLCLK,
    WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

use crate::types::Msg;

const RUN_REG_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const STARTUP_APPROVED_REG_PATH: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const STARTUP_VALUE_NAME: &str = "RustBlox";

const TRAY_WINDOW_CLASS: &str = "RustBloxTrayHostWindow";
const TRAY_WINDOW_TITLE: &str = "RustBloxTrayHost";
const TRAY_ICON_UID: u32 = 1;

const WM_APP_TRAY: u32 = WM_APP + 1;
const WM_APP_SYNC: u32 = WM_APP + 2;
const WM_APP_QUIT: u32 = WM_APP + 3;
const WM_APP_RESTORE: u32 = WM_APP + 4;

const CMD_SHOW_WINDOW: usize = 1001;
const CMD_HIDE_WINDOW: usize = 1002;
const CMD_OPEN_SETTINGS: usize = 1003;
const CMD_TOGGLE_DISCORD_RPC: usize = 1004;
const CMD_CHECK_UPDATES: usize = 1005;
const CMD_QUIT_APP: usize = 1006;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayEvent {
    ShowAndFocus,
    HideToTray,
    OpenSettings,
    ToggleDiscordRpc,
    CheckUpdates,
    QuitApp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrayState {
    pub enabled: bool,
    pub window_visible: bool,
    pub status_line: String,
    pub tooltip: String,
    pub discord_rpc_enabled: bool,
}

impl Default for TrayState {
    fn default() -> Self {
        Self {
            enabled: true,
            window_visible: true,
            status_line: "Idle".to_string(),
            tooltip: "RustBlox".to_string(),
            discord_rpc_enabled: true,
        }
    }
}

struct TrayShared {
    desired: TrayState,
    pending_balloon: Option<(String, String)>,
    shutdown: bool,
}

pub struct TrayController {
    hwnd: Arc<AtomicIsize>,
    shared: Arc<Mutex<TrayShared>>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl TrayController {
    pub fn new(initial: TrayState, tx: Sender<Msg>, ctx: egui::Context) -> Self {
        let hwnd = Arc::new(AtomicIsize::new(0));
        let shared = Arc::new(Mutex::new(TrayShared {
            desired: initial,
            pending_balloon: None,
            shutdown: false,
        }));
        let hwnd_bg = Arc::clone(&hwnd);
        let shared_bg = Arc::clone(&shared);
        let join = thread::Builder::new()
            .name("rustblox-tray".to_string())
            .spawn(move || run_tray_loop(hwnd_bg, shared_bg, tx, ctx))
            .ok();
        Self {
            hwnd,
            shared,
            join: Mutex::new(join),
        }
    }

    pub fn sync(&self, mut next: TrayState) {
        next.tooltip = clamp_tray_tooltip(&next.tooltip);
        let changed = {
            let Ok(mut guard) = self.shared.lock() else {
                return;
            };
            if guard.shutdown || guard.desired == next {
                false
            } else {
                guard.desired = next;
                true
            }
        };
        if changed {
            self.post_msg(WM_APP_SYNC);
        }
    }

    pub fn show_balloon(&self, title: &str, body: &str) {
        {
            let Ok(mut guard) = self.shared.lock() else {
                return;
            };
            if guard.shutdown || !guard.desired.enabled {
                return;
            }
            guard.pending_balloon = Some((title.to_string(), body.to_string()));
        }
        self.post_msg(WM_APP_SYNC);
    }

    pub fn shutdown(&self) {
        {
            if let Ok(mut guard) = self.shared.lock() {
                guard.shutdown = true;
                guard.desired.enabled = false;
                guard.pending_balloon = None;
            }
        }
        let raw_hwnd = self.hwnd.load(Ordering::Acquire);
        if raw_hwnd != 0 {
            unsafe {
                let h = raw_hwnd as HWND;
                PostMessageW(h, WM_CANCELMODE, 0, 0);
                PostMessageW(h, WM_APP_QUIT, 0, 0);
            }
        }
        if let Ok(mut slot) = self.join.lock() {
            if let Some(handle) = slot.take() {
                let _ = handle.join();
            }
        }
    }

    fn post_msg(&self, msg: u32) {
        let raw_hwnd = self.hwnd.load(Ordering::Acquire);
        if raw_hwnd != 0 {
            unsafe {
                PostMessageW(raw_hwnd as HWND, msg, 0, 0);
            }
        }
    }
}

impl Drop for TrayController {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn clamp_tray_tooltip(raw: &str) -> String {
    let trimmed = raw.trim();
    let src = if trimmed.is_empty() {
        "RustBlox"
    } else {
        trimmed
    };
    let mut out = String::with_capacity(src.len().min(127));
    let mut u16_len = 0usize;
    for ch in src.chars() {
        let units = ch.len_utf16();
        if u16_len + units > 127 {
            break;
        }
        out.push(ch);
        u16_len += units;
    }
    out
}

pub fn should_start_in_tray(args: &[String], tray_enabled: bool) -> bool {
    tray_enabled
        && args.iter().skip(1).any(|a| {
            matches!(
                a.trim().to_ascii_lowercase().as_str(),
                "--tray" | "--minimized" | "--start-minimized"
            )
        })
}

pub fn build_startup_command(exe_path: &Path, start_minimized: bool) -> String {
    if start_minimized {
        format!("\"{}\" --tray", exe_path.display())
    } else {
        format!("\"{}\"", exe_path.display())
    }
}

pub fn parse_startup_command(cmd: &str) -> Option<(String, bool)> {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (exe, rest) = if let Some(after_quote) = trimmed.strip_prefix('"') {
        let end = after_quote.find('"')?;
        (&after_quote[..end], &after_quote[end + 1..])
    } else {
        match trimmed.find(char::is_whitespace) {
            Some(idx) => (&trimmed[..idx], &trimmed[idx..]),
            None => (trimmed, ""),
        }
    };
    if exe.is_empty() {
        return None;
    }
    let minimized = rest
        .split_whitespace()
        .any(|flag| matches!(flag.to_ascii_lowercase().as_str(), "--tray" | "--minimized" | "--start-minimized"));
    Some((exe.to_string(), minimized))
}

pub fn is_startup_approved_bytes_enabled(bytes: &[u8]) -> bool {
    match bytes.first() {
        Some(&b) => b == 0x02 || b == 0x06,
        None => true,
    }
}

pub fn is_windows_startup_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(run_key) = hkcu.open_subkey_with_flags(RUN_REG_PATH, KEY_READ) else {
        return false;
    };
    let Ok(cmd) = run_key.get_value::<String, _>(STARTUP_VALUE_NAME) else {
        return false;
    };
    if parse_startup_command(&cmd).is_none() {
        return false;
    }
    if let Ok(approved_key) = hkcu.open_subkey_with_flags(STARTUP_APPROVED_REG_PATH, KEY_READ) {
        if let Ok(val) = approved_key.get_raw_value(STARTUP_VALUE_NAME) {
            return is_startup_approved_bytes_enabled(&val.bytes);
        }
    }
    true
}

pub fn set_windows_startup(enabled: bool, start_minimized: bool) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if enabled {
        let exe = env::current_exe()
            .map_err(|e| format!("Could not determine RustBlox executable path: {e}"))?;
        let (run_key, _) = hkcu
            .create_subkey_with_flags(RUN_REG_PATH, KEY_WRITE)
            .map_err(|e| format!("Could not open Windows Startup registry key: {e}"))?;
        let cmd = build_startup_command(&exe, start_minimized);
        run_key
            .set_value(STARTUP_VALUE_NAME, &cmd)
            .map_err(|e| format!("Could not write Windows Startup entry: {e}"))?;
        if let Ok(approved_key) = hkcu.open_subkey_with_flags(STARTUP_APPROVED_REG_PATH, KEY_WRITE) {
            let _ = approved_key.delete_value(STARTUP_VALUE_NAME);
        }
    } else {
        if let Ok(run_key) = hkcu.open_subkey_with_flags(RUN_REG_PATH, KEY_WRITE) {
            let _ = run_key.delete_value(STARTUP_VALUE_NAME);
        }
        if let Ok(approved_key) = hkcu.open_subkey_with_flags(STARTUP_APPROVED_REG_PATH, KEY_WRITE) {
            let _ = approved_key.delete_value(STARTUP_VALUE_NAME);
        }
    }
    Ok(())
}

pub fn sync_windows_startup_path_if_enabled(start_minimized: bool) {
    if !is_windows_startup_enabled() {
        return;
    }
    let Ok(exe) = env::current_exe() else {
        return;
    };
    let expected = build_startup_command(&exe, start_minimized);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(run_key) = hkcu.open_subkey_with_flags(RUN_REG_PATH, KEY_READ) else {
        return;
    };
    let current = run_key.get_value::<String, _>(STARTUP_VALUE_NAME).ok();
    if current.as_deref() != Some(expected.as_str()) {
        let _ = set_windows_startup(true, start_minimized);
    }
}

/// If another RustBlox instance is already running with Tray Mode active, ask it to
/// restore & focus its window cleanly instead of opening a duplicate window.
pub fn try_focus_existing_instance() -> bool {
    let cls = wide_nul(TRAY_WINDOW_CLASS);
    let title = wide_nul(TRAY_WINDOW_TITLE);
    unsafe {
        let existing = FindWindowW(cls.as_ptr(), title.as_ptr());
        if existing.is_null() {
            return false;
        }
        let mut ack: usize = 0;
        let sent = SendMessageTimeoutW(
            existing,
            WM_APP_RESTORE,
            0,
            0,
            SMTO_ABORTIFHUNG,
            400,
            &mut ack,
        );
        sent != 0 && ack == 1
    }
}

struct TrayRuntime {
    hwnd_slot: Arc<AtomicIsize>,
    shared: Arc<Mutex<TrayShared>>,
    tx: Sender<Msg>,
    ctx: egui::Context,
    hicon: HICON,
    owned_icon: bool,
    icon_added: bool,
    taskbar_created_msg: u32,
}

thread_local! {
    static TRAY_RUNTIME: RefCell<Option<TrayRuntime>> = const { RefCell::new(None) };
}

fn run_tray_loop(
    hwnd_slot: Arc<AtomicIsize>,
    shared: Arc<Mutex<TrayShared>>,
    tx: Sender<Msg>,
    ctx: egui::Context,
) {
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let cls_name = wide_nul(TRAY_WINDOW_CLASS);
        let wnd_title = wide_nul(TRAY_WINDOW_TITLE);
        let wc = WNDCLASSW {
            lpfnWndProc: Some(tray_wnd_proc),
            hInstance: hinstance,
            lpszClassName: cls_name.as_ptr(),
            ..std::mem::zeroed()
        };
        let _ = RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            0,
            cls_name.as_ptr(),
            wnd_title.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }

        let (hicon, owned_icon) = load_tray_hicon();
        let taskbar_created_msg = RegisterWindowMessageW(wide_nul("TaskbarCreated").as_ptr());
        let early_shutdown = shared.lock().map(|g| g.shutdown).unwrap_or(false);
        if early_shutdown {
            if owned_icon && !hicon.is_null() {
                DestroyIcon(hicon);
            }
            DestroyWindow(hwnd);
            return;
        }

        hwnd_slot.store(hwnd as isize, Ordering::Release);
        TRAY_RUNTIME.with(|cell| {
            *cell.borrow_mut() = Some(TrayRuntime {
                hwnd_slot: Arc::clone(&hwnd_slot),
                shared,
                tx,
                ctx,
                hicon,
                owned_icon,
                icon_added: false,
                taskbar_created_msg,
            });
        });

        apply_tray_sync(hwnd, false);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        TRAY_RUNTIME.with(|cell| {
            if let Some(mut rt) = cell.borrow_mut().take() {
                remove_tray_icon(hwnd, &mut rt.icon_added);
                rt.hwnd_slot.store(0, Ordering::Release);
                if rt.owned_icon && !rt.hicon.is_null() {
                    DestroyIcon(rt.hicon);
                }
            }
        });
    }
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_APP_TRAY => {
            let mouse_msg = (lparam as u32) & 0xFFFF;
            match mouse_msg {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                    emit_tray_event(TrayEvent::ShowAndFocus);
                }
                WM_RBUTTONUP | WM_CONTEXTMENU => {
                    unsafe { show_context_menu(hwnd); }
                }
                _ => {}
            }
            0
        }
        WM_APP_SYNC => {
            unsafe { apply_tray_sync(hwnd, true); }
            0
        }
        WM_APP_RESTORE => {
            let enabled = TRAY_RUNTIME.with(|cell| {
                cell.borrow()
                    .as_ref()
                    .and_then(|rt| rt.shared.lock().ok().map(|g| g.desired.enabled && !g.shutdown))
                    .unwrap_or(false)
            });
            if enabled {
                emit_tray_event(TrayEvent::ShowAndFocus);
                1
            } else {
                0
            }
        }
        WM_APP_QUIT | WM_CLOSE => {
            TRAY_RUNTIME.with(|cell| {
                if let Some(rt) = cell.borrow_mut().as_mut() {
                    unsafe {
                        remove_tray_icon(hwnd, &mut rt.icon_added);
                        rt.hwnd_slot.store(0, Ordering::Release);
                        DestroyWindow(hwnd);
                    }
                }
            });
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0); }
            0
        }
        other => {
            let is_taskbar_created = TRAY_RUNTIME.with(|cell| {
                cell.borrow()
                    .as_ref()
                    .is_some_and(|rt| rt.taskbar_created_msg != 0 && rt.taskbar_created_msg == other)
            });
            if is_taskbar_created {
                TRAY_RUNTIME.with(|cell| {
                    if let Some(rt) = cell.borrow_mut().as_mut() {
                        rt.icon_added = false;
                    }
                });
                unsafe { apply_tray_sync(hwnd, false); }
                0
            } else {
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
        }
    }
}

fn emit_tray_event(ev: TrayEvent) {
    TRAY_RUNTIME.with(|cell| {
        if let Some(rt) = cell.borrow().as_ref() {
            let _ = rt.tx.send(Msg::TrayAction(ev));
            rt.ctx.request_repaint();
        }
    });
}

unsafe fn apply_tray_sync(hwnd: HWND, allow_balloon: bool) {
    TRAY_RUNTIME.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(rt) = borrow.as_mut() else {
            return;
        };
        let (desired, balloon, shutdown) = match rt.shared.lock() {
            Ok(mut guard) => (
                guard.desired.clone(),
                if allow_balloon {
                    guard.pending_balloon.take()
                } else {
                    None
                },
                guard.shutdown,
            ),
            Err(_) => return,
        };
        if shutdown || !desired.enabled {
            unsafe { remove_tray_icon(hwnd, &mut rt.icon_added); }
            return;
        }
        unsafe { upsert_tray_icon(hwnd, rt, &desired.tooltip, balloon); }
    });
}

unsafe fn upsert_tray_icon(
    hwnd: HWND,
    rt: &mut TrayRuntime,
    tooltip: &str,
    balloon: Option<(String, String)>,
) {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_UID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_APP_TRAY,
        hIcon: rt.hicon,
        ..Default::default()
    };
    copy_wide_fixed(&mut nid.szTip, &clamp_tray_tooltip(tooltip));
    if let Some((title, body)) = balloon {
        nid.uFlags |= NIF_INFO;
        nid.dwInfoFlags = NIIF_INFO | NIIF_NOSOUND;
        copy_wide_fixed(&mut nid.szInfoTitle, &title);
        copy_wide_fixed(&mut nid.szInfo, &body);
    }

    let action = if rt.icon_added { NIM_MODIFY } else { NIM_ADD };
    if unsafe { Shell_NotifyIconW(action, &nid) } != 0 {
        rt.icon_added = true;
    } else if rt.icon_added {
        if unsafe { Shell_NotifyIconW(NIM_ADD, &nid) } != 0 {
            rt.icon_added = true;
        }
    }
}

unsafe fn remove_tray_icon(hwnd: HWND, icon_added: &mut bool) {
    if !*icon_added {
        return;
    }
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_UID,
        ..Default::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
    *icon_added = false;
}

unsafe fn show_context_menu(hwnd: HWND) {
    let desired = TRAY_RUNTIME.with(|cell| {
        cell.borrow()
            .as_ref()
            .and_then(|rt| rt.shared.lock().ok().map(|g| g.desired.clone()))
    });
    let Some(state) = desired else {
        return;
    };

    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }

        if state.window_visible {
            AppendMenuW(menu, MF_STRING, CMD_SHOW_WINDOW, wide_nul("Focus RustBlox").as_ptr());
            AppendMenuW(menu, MF_STRING, CMD_HIDE_WINDOW, wide_nul("Minimize to Tray").as_ptr());
        } else {
            AppendMenuW(menu, MF_STRING, CMD_SHOW_WINDOW, wide_nul("Open RustBlox").as_ptr());
        }

        let status_label = if state.status_line.trim().is_empty() {
            "Status: Idle".to_string()
        } else {
            state.status_line.clone()
        };
        AppendMenuW(menu, MF_GRAYED, 0, wide_nul(&status_label).as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());

        let rpc_flag = if state.discord_rpc_enabled {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING | MF_UNCHECKED
        };
        AppendMenuW(
            menu,
            rpc_flag,
            CMD_TOGGLE_DISCORD_RPC,
            wide_nul("Discord Rich Presence").as_ptr(),
        );
        AppendMenuW(menu, MF_STRING, CMD_OPEN_SETTINGS, wide_nul("Settings").as_ptr());
        AppendMenuW(
            menu,
            MF_STRING,
            CMD_CHECK_UPDATES,
            wide_nul("Check for Updates").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(menu, MF_STRING, CMD_QUIT_APP, wide_nul("Quit RustBlox").as_ptr());

        let mut pt = POINT { x: 0, y: 0 };
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        let _ = PostMessageW(hwnd, WM_NULL, 0, 0);
        let _ = DestroyMenu(menu);

        if chosen > 0 {
            match chosen as usize {
                CMD_SHOW_WINDOW => emit_tray_event(TrayEvent::ShowAndFocus),
                CMD_HIDE_WINDOW => emit_tray_event(TrayEvent::HideToTray),
                CMD_OPEN_SETTINGS => emit_tray_event(TrayEvent::OpenSettings),
                CMD_TOGGLE_DISCORD_RPC => emit_tray_event(TrayEvent::ToggleDiscordRpc),
                CMD_CHECK_UPDATES => emit_tray_event(TrayEvent::CheckUpdates),
                CMD_QUIT_APP => emit_tray_event(TrayEvent::QuitApp),
                _ => {}
            }
        }
    }
}

unsafe fn load_tray_hicon() -> (HICON, bool) {
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.clamp(16, 64) as u32;
    if let Ok(img) = image::load_from_memory(include_bytes!("../assets/logo.png")) {
        let resized = img
            .resize_exact(size, size, image::imageops::FilterType::Triangle)
            .to_rgba8();
        let mut bgra = Vec::with_capacity((size * size * 4) as usize);
        for px in resized.pixels() {
            bgra.push(px[2]);
            bgra.push(px[1]);
            bgra.push(px[0]);
            bgra.push(px[3]);
        }
        let mask_stride = ((size as usize + 15) / 16) * 2;
        let mask = vec![0u8; mask_stride * size as usize];
        unsafe {
            let color_bmp: HBITMAP =
                CreateBitmap(size as i32, size as i32, 1, 32, bgra.as_ptr() as *const _);
            let mask_bmp: HBITMAP =
                CreateBitmap(size as i32, size as i32, 1, 1, mask.as_ptr() as *const _);
            if !color_bmp.is_null() && !mask_bmp.is_null() {
                let info = ICONINFO {
                    fIcon: 1,
                    xHotspot: 0,
                    yHotspot: 0,
                    hbmMask: mask_bmp,
                    hbmColor: color_bmp,
                };
                let icon = CreateIconIndirect(&info);
                let _ = DeleteObject(color_bmp as _);
                let _ = DeleteObject(mask_bmp as _);
                if !icon.is_null() {
                    return (icon, true);
                }
            } else {
                if !color_bmp.is_null() {
                    let _ = DeleteObject(color_bmp as _);
                }
                if !mask_bmp.is_null() {
                    let _ = DeleteObject(mask_bmp as _);
                }
            }
        }
    }
    let fallback = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };
    (fallback, false)
}

fn wide_nul(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn copy_wide_fixed<const N: usize>(buf: &mut [u16; N], s: &str) {
    buf.fill(0);
    if N == 0 {
        return;
    }
    for (idx, code) in s.encode_utf16().take(N - 1).enumerate() {
        buf[idx] = code;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_parses_windows_startup_command() {
        let p = Path::new(r"C:\Program Files\RustBlox\rustblox.exe");
        let cmd_tray = build_startup_command(p, true);
        assert_eq!(
            cmd_tray,
            r#""C:\Program Files\RustBlox\rustblox.exe" --tray"#
        );
        assert_eq!(
            parse_startup_command(&cmd_tray),
            Some((r"C:\Program Files\RustBlox\rustblox.exe".to_string(), true))
        );

        let cmd_normal = build_startup_command(p, false);
        assert_eq!(cmd_normal, r#""C:\Program Files\RustBlox\rustblox.exe""#);
        assert_eq!(
            parse_startup_command(&cmd_normal),
            Some((r"C:\Program Files\RustBlox\rustblox.exe".to_string(), false))
        );
    }

    #[test]
    fn detects_start_in_tray_cli_flag() {
        let args = vec!["rustblox.exe".to_string(), "--tray".to_string()];
        assert!(should_start_in_tray(&args, true));
        assert!(!should_start_in_tray(&args, false));

        let args_normal = vec!["rustblox.exe".to_string()];
        assert!(!should_start_in_tray(&args_normal, true));

        let args_min = vec!["rustblox.exe".to_string(), "--start-minimized".to_string()];
        assert!(should_start_in_tray(&args_min, true));
    }

    #[test]
    fn checks_startup_approved_enabled_byte() {
        assert!(is_startup_approved_bytes_enabled(&[0x02, 0, 0, 0]));
        assert!(is_startup_approved_bytes_enabled(&[0x06, 0, 0, 0]));
        assert!(!is_startup_approved_bytes_enabled(&[0x03, 0, 0, 0]));
        assert!(!is_startup_approved_bytes_enabled(&[0x01, 0, 0, 0]));
        assert!(is_startup_approved_bytes_enabled(&[]));
    }

    #[test]
    fn clamps_tray_tooltip_length_cleanly() {
        assert_eq!(clamp_tray_tooltip("  "), "RustBlox");
        let long = "a".repeat(200);
        let clamped = clamp_tray_tooltip(&long);
        assert_eq!(clamped.encode_utf16().count(), 127);
    }
}
