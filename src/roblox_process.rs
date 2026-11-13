use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde_json::Value;
use windows_sys::Win32::Foundation::{
    CloseHandle, FALSE, FILETIME, HANDLE, HWND, INVALID_HANDLE_VALUE, LPARAM, TRUE, WPARAM,
};
use windows_sys::core::BOOL;
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    TerminateProcess,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE,
};
use winreg::enums::*;
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;
use crate::api::{enc, post_csrf};
use crate::auth::{data_dir, load_settings};
use crate::types::Msg;
use eframe::egui;

pub const CMD_KEY: &str = r"Software\Classes\roblox-player\shell\open\command";

pub fn find_player() -> Option<PathBuf> {
    let base = PathBuf::from(env::var("LOCALAPPDATA").ok()?)
        .join("Roblox")
        .join("Versions");
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(base).ok()?.flatten() {
        let exe = entry.path().join("RobloxPlayerBeta.exe");
        if let Some(time) = fs::metadata(&exe).ok().and_then(|m| m.modified().ok()) {
            if best.as_ref().map_or(true, |(t, _)| time > *t) {
                best = Some((time, exe));
            }
        }
    }
    best.map(|(_, path)| path)
}

pub fn apply_fps_setting(target_fps: Option<u32>) -> Result<(), String> {
    let player = find_player()
        .ok_or_else(|| "Roblox is not installed. Install it once from roblox.com first.".to_string())?;
    let version_dir = player
        .parent()
        .ok_or_else(|| "Could not determine Roblox version directory".to_string())?;
    let cs_dir = version_dir.join("ClientSettings");
    let cs_file = cs_dir.join("ClientAppSettings.json");

    let mut map = if let Ok(existing) = fs::read_to_string(&cs_file) {
        serde_json::from_str::<serde_json::Map<String, Value>>(&existing).unwrap_or_default()
    } else {
        serde_json::Map::new()
    };

    if let Some(fps) = target_fps {
        map.insert("DFIntTaskSchedulerTargetFps".to_string(), Value::from(fps));
        fs::create_dir_all(&cs_dir).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(&Value::Object(map)).map_err(|e| e.to_string())?;
        fs::write(&cs_file, text).map_err(|e| e.to_string())?;
    } else {
        map.remove("DFIntTaskSchedulerTargetFps");
        if cs_file.exists() {
            if map.is_empty() {
                let _ = fs::remove_file(&cs_file);
            } else {
                let text =
                    serde_json::to_string_pretty(&Value::Object(map)).map_err(|e| e.to_string())?;
                fs::write(&cs_file, text).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

#[repr(C)]
struct NtUnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *const u16,
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQueryInformationProcess(
        process_handle: HANDLE,
        process_information_class: u32,
        process_information: *mut std::ffi::c_void,
        process_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

fn read_process_cmdline(handle: HANDLE) -> String {
    let mut qwords = vec![0u64; 512];
    let mut ret_len: u32 = 0;
    let mut status = unsafe {
        NtQueryInformationProcess(
            handle,
            60,
            qwords.as_mut_ptr() as *mut std::ffi::c_void,
            (qwords.len() * 8) as u32,
            &mut ret_len,
        )
    };
    if status < 0 && ret_len > (qwords.len() * 8) as u32 && ret_len <= 131_072 {
        qwords.resize((ret_len as usize).div_ceil(8), 0);
        status = unsafe {
            NtQueryInformationProcess(
                handle,
                60,
                qwords.as_mut_ptr() as *mut std::ffi::c_void,
                (qwords.len() * 8) as u32,
                &mut ret_len,
            )
        };
    }
    if status < 0 || qwords.len() * 8 < std::mem::size_of::<NtUnicodeString>() {
        return String::new();
    }
    unsafe {
        let us = &*(qwords.as_ptr() as *const NtUnicodeString);
        let start = qwords.as_ptr() as usize;
        let end = start + qwords.len() * 8;
        let buf_addr = us.buffer as usize;
        let byte_len = us.length as usize;
        if us.buffer.is_null()
            || byte_len == 0
            || byte_len % 2 != 0
            || buf_addr < start
            || buf_addr.saturating_add(byte_len) > end
        {
            return String::new();
        }
        let slice = std::slice::from_raw_parts(us.buffer, byte_len / 2);
        String::from_utf16_lossy(slice)
    }
}

fn filetime_to_unix_ms(ft: FILETIME) -> u64 {
    let ticks = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
    ticks.saturating_sub(116_444_736_000_000_000) / 10_000
}

fn read_process_start_ms(handle: HANDLE) -> u64 {
    let mut created = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut zero = created;
    let ok = unsafe { GetProcessTimes(handle, &mut created, &mut zero, &mut zero, &mut zero) };
    if ok != 0 {
        filetime_to_unix_ms(created)
    } else {
        0
    }
}

pub fn parse_crash_handler_attachment(cmd: &str) -> Option<(u32, PathBuf)> {
    let lower = cmd.to_ascii_lowercase();
    let pid_tag = "--annotation=pid=";
    let pid_pos = lower.find(pid_tag)? + pid_tag.len();
    let pid_digits: String = cmd[pid_pos..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let pid = pid_digits.parse::<u32>().ok()?;

    let att_tag = "--attachment=";
    let att_start = lower.find(att_tag)? + att_tag.len();
    let eq_rel = cmd[att_start..].find('=')?;
    let path_start = att_start + eq_rel + 1;
    let rest = &cmd[path_start..];
    let rest_lower = &lower[path_start..];
    let log_end = rest_lower.find(".log")? + 4;
    let raw = rest[..log_end].trim_matches('"');
    if raw.is_empty() {
        None
    } else {
        Some((pid, PathBuf::from(raw)))
    }
}

pub struct RbxProc {
    pub pid: u32,
    pub start_ms: u64,
    pub cmdline: String,
}

pub fn enumerate_roblox_processes() -> (Vec<RbxProc>, HashMap<u32, PathBuf>) {
    let mut players = Vec::new();
    let mut crash_logs = HashMap::new();
    let self_pid = std::process::id();
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snap == INVALID_HANDLE_VALUE {
        return (players, crash_logs);
    }
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    if unsafe { Process32FirstW(snap, &mut entry) } != FALSE {
        loop {
            let pid = entry.th32ProcessID;
            if pid != 0 && pid != self_pid {
                let nul = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe = String::from_utf16_lossy(&entry.szExeFile[..nul]);
                let is_player = exe.eq_ignore_ascii_case("RobloxPlayerBeta.exe");
                let is_crash = exe.eq_ignore_ascii_case("RobloxCrashHandler.exe");
                if is_player || is_crash {
                    let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid) };
                    if !h.is_null() {
                        let cmdline = read_process_cmdline(h);
                        let start_ms = if is_player {
                            read_process_start_ms(h)
                        } else {
                            0
                        };
                        unsafe { CloseHandle(h) };
                        if is_player {
                            players.push(RbxProc {
                                pid,
                                start_ms,
                                cmdline,
                            });
                        } else if let Some((target_pid, log_path)) =
                            parse_crash_handler_attachment(&cmdline)
                        {
                            crash_logs.insert(target_pid, log_path);
                        }
                    } else if is_player {
                        players.push(RbxProc {
                            pid,
                            start_ms: 0,
                            cmdline: String::new(),
                        });
                    }
                }
            }
            if unsafe { Process32NextW(snap, &mut entry) } == FALSE {
                break;
            }
        }
    }
    unsafe { CloseHandle(snap) };
    (players, crash_logs)
}

struct CloseWinCtx {
    pid: u32,
    posted: bool,
}

unsafe extern "system" fn enum_close_pid_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let ctx = &mut *(lparam as *mut CloseWinCtx);
        let mut win_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut win_pid);
        if win_pid == ctx.pid && PostMessageW(hwnd, WM_CLOSE, 0 as WPARAM, 0) != FALSE {
            ctx.posted = true;
        }
    }
    TRUE
}

pub fn terminate_roblox_process(pid: u32) -> bool {
    let h = unsafe { OpenProcess(PROCESS_TERMINATE, FALSE, pid) };
    if !h.is_null() {
        let ok = unsafe { TerminateProcess(h, 0) } != FALSE;
        unsafe { CloseHandle(h) };
        if ok {
            return true;
        }
    }
    let mut ctx = CloseWinCtx { pid, posted: false };
    unsafe {
        EnumWindows(
            Some(enum_close_pid_window),
            (&raw mut ctx) as LPARAM,
        );
    }
    ctx.posted
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSessionState {
    TrayOnly,
    Playing,
    AppAfterPlay,
    AppOnly,
    Unknown,
}

fn last_pattern_offset(text: &str, patterns: &[&str]) -> Option<usize> {
    patterns.iter().filter_map(|p| text.rfind(p)).max()
}

pub fn inspect_player_log_text(text: &str, total_len: u64) -> (LogSessionState, bool) {
    let is_handoff = text.contains("waitForNewPlayerProcess");

    let play_markers = [
        "[FLog::SessionTransitionFSM] Entered play session.",
        "[FLog::SingleSurfaceApp] setStage: (stage:UGCGame)",
        "[FLog::SingleSurfaceApp] launchUGCGame:",
        "[FLog::SessionTransitionFSM] Teleported.",
    ];
    let app_markers = [
        "[FLog::SessionTransitionFSM] Entered app session.",
        "[FLog::SingleSurfaceApp] setStage: (stage:LuaApp)",
    ];
    let last_play = last_pattern_offset(text, &play_markers);
    let last_app = last_pattern_offset(text, &app_markers);
    let has_play =
        last_play.is_some() || text.contains("returnToLuaApp: (stage:UGCGame)") || total_len > 32_768;

    let state = if let Some(app_pos) = last_app {
        if last_play.is_none_or(|play_pos| app_pos > play_pos) {
            if has_play {
                LogSessionState::AppAfterPlay
            } else {
                LogSessionState::AppOnly
            }
        } else {
            LogSessionState::Playing
        }
    } else if last_play.is_some() {
        LogSessionState::Playing
    } else if text.contains("AppState/TrayMode") && !text.contains("User open main window") {
        LogSessionState::TrayOnly
    } else {
        LogSessionState::Unknown
    };
    (state, is_handoff)
}

pub fn inspect_player_log(path: &Path) -> (LogSessionState, bool) {
    let Ok(mut f) = fs::File::open(path) else {
        return (LogSessionState::Unknown, false);
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    const MAX_TAIL: u64 = 512 * 1024;
    if len > MAX_TAIL {
        let _ = f.seek(SeekFrom::Start(len - MAX_TAIL));
    }
    let mut buf = Vec::with_capacity(len.min(MAX_TAIL) as usize);
    if f.read_to_end(&mut buf).is_err() {
        return (LogSessionState::Unknown, false);
    }
    let text = String::from_utf8_lossy(&buf);
    inspect_player_log_text(&text, len)
}

pub fn roblox_logs_dir() -> Option<PathBuf> {
    Some(PathBuf::from(env::var("LOCALAPPDATA").ok()?).join("Roblox").join("logs"))
}

pub fn find_player_log_for_proc(
    proc: &RbxProc,
    crash_logs: &HashMap<u32, PathBuf>,
    logs_dir: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(p) = crash_logs.get(&proc.pid) {
        if p.exists() {
            return Some(p.clone());
        }
    }
    let dir = logs_dir?;
    let mut best: Option<(u64, u64, PathBuf)> = None;
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.contains("_Player_")
            || !name_str.ends_with("_last.log")
            || name_str.contains("CrashHandler")
        {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let created_ms = meta
            .created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .and_then(|d| u64::try_from(d.as_millis()).ok())
            .unwrap_or(0);
        let modified_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .and_then(|d| u64::try_from(d.as_millis()).ok())
            .unwrap_or(0);
        if proc.start_ms > 0 {
            if modified_ms + 2_000 < proc.start_ms {
                continue;
            }
            if created_ms > 0 && created_ms + 4_000 < proc.start_ms {
                continue;
            }
        }
        let dist = if proc.start_ms > 0 && created_ms > 0 {
            created_ms.abs_diff(proc.start_ms)
        } else {
            0
        };
        if best
            .as_ref()
            .is_none_or(|(b_dist, b_mod, _)| dist < *b_dist || (dist == *b_dist && modified_ms > *b_mod))
        {
            best = Some((dist, modified_ms, entry.path()));
        }
    }
    best.map(|(_, _, p)| p)
}

pub fn close_tray_roblox_processes() -> usize {
    let (procs, crash_logs) = enumerate_roblox_processes();
    let logs_dir = roblox_logs_dir();
    let mut closed = 0;
    for p in procs {
        let lower = p.cmdline.to_ascii_lowercase();
        let is_tray_cmd = lower.contains("--launch-to-tray");
        let is_tray_log = if !is_tray_cmd && lower.is_empty() {
            find_player_log_for_proc(&p, &crash_logs, logs_dir.as_deref())
                .is_some_and(|log| inspect_player_log(&log).0 == LogSessionState::TrayOnly)
        } else {
            false
        };
        if (is_tray_cmd || is_tray_log) && terminate_roblox_process(p.pid) {
            closed += 1;
        }
    }
    closed
}

pub fn start_roblox_watcher(
    tx: Sender<Msg>,
    ctx: egui::Context,
    enabled: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    last_launch_ms: Arc<AtomicU64>,
) {
    thread::spawn(move || {
        let mut seen_app_since: HashMap<u32, Instant> = HashMap::new();
        let mut recently_closed: HashMap<u32, Instant> = HashMap::new();
        while !stop.load(Ordering::Relaxed) {
            if !enabled.load(Ordering::Relaxed) {
                seen_app_since.clear();
                thread::sleep(Duration::from_millis(1500));
                continue;
            }
            let (procs, crash_logs) = enumerate_roblox_processes();
            if procs.is_empty() {
                seen_app_since.clear();
                recently_closed.retain(|_, t| t.elapsed() < Duration::from_secs(10));
                thread::sleep(Duration::from_millis(1000));
                continue;
            }

            let now_ms = now_unix_ms();
            let launch_age_ms = now_ms.saturating_sub(last_launch_ms.load(Ordering::Relaxed));
            let active_pids: HashSet<u32> = procs.iter().map(|p| p.pid).collect();
            seen_app_since.retain(|pid, _| active_pids.contains(pid));
            recently_closed.retain(|pid, t| active_pids.contains(pid) && t.elapsed() < Duration::from_secs(10));

            let logs_dir = roblox_logs_dir();
            for p in procs {
                if recently_closed.contains_key(&p.pid) {
                    continue;
                }
                let lower = p.cmdline.to_ascii_lowercase();
                let proc_age_ms = if p.start_ms > 0 {
                    now_ms.saturating_sub(p.start_ms)
                } else {
                    0
                };

                // Always close lingering background tray process
                if lower.contains("--launch-to-tray") {
                    if terminate_roblox_process(p.pid) {
                        recently_closed.insert(p.pid, Instant::now());
                    }
                    continue;
                }

                let log_state = find_player_log_for_proc(&p, &crash_logs, logs_dir.as_deref())
                    .map(|log| inspect_player_log(&log));

                let explicit_app = !lower.is_empty()
                    && (lower.contains("--app") || lower.contains("launchmode:app"))
                    && !lower.contains("launchmode:play")
                    && !lower.contains("placelauncher");

                let should_close = match log_state {
                    Some((LogSessionState::AppAfterPlay, _)) => {
                        let first = seen_app_since.entry(p.pid).or_insert_with(Instant::now);
                        if first.elapsed() >= Duration::from_millis(800) {
                            true
                        } else {
                            false
                        }
                    }
                    Some((LogSessionState::TrayOnly, _)) => proc_age_ms >= 2_000,
                    Some((LogSessionState::AppOnly, false)) => {
                        if launch_age_ms >= 15_000 || explicit_app {
                            let first = seen_app_since.entry(p.pid).or_insert_with(Instant::now);
                            first.elapsed() >= Duration::from_millis(2_500)
                        } else {
                            seen_app_since.remove(&p.pid);
                            false
                        }
                    }
                    Some((LogSessionState::Playing, _)) => {
                        seen_app_since.remove(&p.pid);
                        false
                    }
                    _ => {
                        if explicit_app && proc_age_ms >= 3_000 {
                            let first = seen_app_since.entry(p.pid).or_insert_with(Instant::now);
                            first.elapsed() >= Duration::from_millis(1_500)
                        } else {
                            false
                        }
                    }
                };

                if should_close {
                    seen_app_since.remove(&p.pid);
                    if terminate_roblox_process(p.pid) {
                        recently_closed.insert(p.pid, Instant::now());
                        let _ = tx.send(Msg::RobloxAppClosed);
                        ctx.request_repaint();
                    }
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}

pub fn launch_from_uri(uri: &str, preclean_tray: bool) -> Result<(), String> {
    let settings = load_settings();
    let _ = apply_fps_setting(settings.target_fps);
    if preclean_tray && settings.auto_close_roblox && close_tray_roblox_processes() > 0 {
        thread::sleep(Duration::from_millis(100));
    }
    let player = find_player()
        .ok_or("Roblox is not installed. Install it once from roblox.com first.")?;
    Command::new(player)
        .arg(uri)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn launch_authenticated(place_id: u64, cookie: &str) -> Result<String, String> {
    launch_authenticated_server(place_id, None, cookie)
}

pub fn launch_authenticated_server(
    place_id: u64,
    job_id: Option<&str>,
    cookie: &str,
) -> Result<String, String> {
    let resp = post_csrf(
        "https://auth.roblox.com/v1/authentication-ticket",
        serde_json::json!({}),
        Some(cookie),
        Some("https://www.roblox.com/"),
    )?;
    let ticket = resp
        .header("rbx-authentication-ticket")
        .ok_or_else(|| "Session expired — please sign in again".to_string())?;
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let launcher_url = match job_id {
        Some(jid) if !jid.is_empty() => format!(
            "https://assetgame.roblox.com/game/PlaceLauncher.ashx?request=RequestGameJob&browserTrackerId=0&placeId={place_id}&gameId={jid}&isPlayTogetherGame=false"
        ),
        _ => format!(
            "https://assetgame.roblox.com/game/PlaceLauncher.ashx?request=RequestGame&browserTrackerId=0&placeId={place_id}&isPlayTogetherGame=false"
        ),
    };
    let uri = format!(
        "roblox-player:1+launchmode:play+gameinfo:{ticket}+launchtime:{ms}+placelauncherurl:{}+robloxLocale:en_us+gameLocale:en_us",
        enc(&launcher_url)
    );
    launch_from_uri(&uri, true)?;
    Ok(if job_id.is_some() {
        "Joining selected server...".to_string()
    } else {
        "Launching Roblox (signed in)...".to_string()
    })
}

pub fn handler_status() -> String {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(key) = hkcu.open_subkey_with_flags(CMD_KEY, KEY_READ) else {
        return "Not registered (Roblox not installed)".to_string();
    };
    let Ok(current) = key.get_value::<String, _>("") else {
        return "Unknown".to_string();
    };
    if let Ok(me) = env::current_exe() {
        let mine = format!("\"{}\" \"%1\"", me.display());
        if current.eq_ignore_ascii_case(&mine) {
            return "rustblox (this app)".to_string();
        }
    }
    if current.to_ascii_lowercase().contains("rustblox") {
        "rustblox (another path)".to_string()
    } else {
        "Default Roblox launcher".to_string()
    }
}

pub fn register() -> Result<String, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(CMD_KEY, KEY_READ | KEY_WRITE)
        .map_err(|e| format!("Roblox handler not found: {e}"))?;
    let current: String = key.get_value("").map_err(|e| e.to_string())?;
    let me = env::current_exe().map_err(|e| e.to_string())?;
    let mine = format!("\"{}\" \"%1\"", me.display());
    if current == mine {
        return Ok("Already registered".to_string());
    }
    fs::write(data_dir().join("original_command.txt"), &current).map_err(|e| e.to_string())?;
    key.set_value("", &mine).map_err(|e| e.to_string())?;
    Ok("rustblox now opens Roblox links".to_string())
}

pub fn restore() -> Result<String, String> {
    let original = fs::read_to_string(data_dir().join("original_command.txt"))
        .map_err(|_| "No backup found".to_string())?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(CMD_KEY, KEY_WRITE)
        .map_err(|e| e.to_string())?;
    key.set_value("", &original).map_err(|e| e.to_string())?;
    Ok("Original Roblox launcher restored".to_string())
}
