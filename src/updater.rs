use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, CornerRadius, RichText, Sense};
use serde_json::Value;

use crate::auth::load_settings;
use crate::theme::{apply_theme, install_fonts, pal, theme_accent};

pub const RUSTBLOX_REPO: &str = "gothssmy-design/rustblox";
pub const RUSTBLOX_RELEASES_URL: &str = "https://github.com/gothssmy-design/rustblox/releases";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateKind {
    Patch,
    Full,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustBloxRelease {
    pub version: String,
    pub tag_name: String,
    pub kind: UpdateKind,
    pub title: String,
    pub notes: String,
    pub html_url: String,
    pub exe_url: Option<String>,
    pub exe_name: Option<String>,
    pub exe_size: u64,
    pub hotfix_url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LiveHotfix {
    pub fastflag_overrides: HashMap<String, Value>,
    pub auto_close_roblox: Option<bool>,
    pub status_note: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UpdaterArgs {
    pub target_exe: PathBuf,
    pub from_ver: String,
    pub to_ver: String,
    pub exe_url: String,
    pub exe_size: u64,
}

pub fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    let core = trimmed.split(['-', '+']).next().unwrap_or("").trim();
    if core.is_empty() {
        return None;
    }
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn is_newer_version(remote: &str, current: &str) -> bool {
    match (parse_semver(remote), parse_semver(current)) {
        (Some(r), Some(c)) => r > c,
        _ => false,
    }
}

pub fn classify_update_kind(
    remote: &str,
    current: &str,
    title: &str,
    notes: &str,
) -> UpdateKind {
    let lower_meta = format!("{title}\n{notes}").to_ascii_lowercase();
    if lower_meta.contains("[full]") || lower_meta.contains("[restart]") {
        return UpdateKind::Full;
    }
    if lower_meta.contains("[patch]") || lower_meta.contains("[hotfix]") {
        return UpdateKind::Patch;
    }
    match (parse_semver(remote), parse_semver(current)) {
        (Some((rmaj, rmin, rpat)), Some((cmaj, cmin, cpat))) => {
            if rmaj == cmaj && rmin == cmin && rpat > cpat {
                UpdateKind::Patch
            } else {
                UpdateKind::Full
            }
        }
        _ => UpdateKind::Full,
    }
}

pub fn parse_github_release_update(v: &Value, current_version: &str) -> Option<RustBloxRelease> {
    if v["draft"].as_bool().unwrap_or(false) {
        return None;
    }
    let tag_name = v["tag_name"].as_str()?.trim().to_string();
    if !is_newer_version(&tag_name, current_version) {
        return None;
    }
    let normalized_ver = parse_semver(&tag_name)
        .map(|(a, b, c)| format!("{a}.{b}.{c}"))
        .unwrap_or_else(|| tag_name.trim_start_matches(['v', 'V']).to_string());
    let title = v["name"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&tag_name)
        .to_string();
    let notes = v["body"].as_str().map(str::trim).unwrap_or("").to_string();
    let html_url = v["html_url"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(RUSTBLOX_RELEASES_URL)
        .to_string();
    let kind = classify_update_kind(&tag_name, current_version, &title, &notes);

    let mut chosen_exe: Option<(String, String, u64, bool)> = None;
    let mut hotfix_url: Option<String> = None;
    if let Some(assets) = v["assets"].as_array() {
        for a in assets {
            let Some(name) = a["name"].as_str() else {
                continue;
            };
            let Some(url) = a["browser_download_url"].as_str() else {
                continue;
            };
            let lower = name.to_ascii_lowercase();
            if lower == "hotfix.json" || lower.ends_with(".hotfix.json") {
                hotfix_url = Some(url.to_string());
                continue;
            }
            if !lower.ends_with(".exe") {
                continue;
            }
            let preferred = lower.contains("rustblox");
            let size = a["size"].as_u64().unwrap_or(0);
            let replace = match &chosen_exe {
                None => true,
                Some((_, _, _, was_pref)) => !was_pref && preferred,
            };
            if replace {
                chosen_exe = Some((url.to_string(), name.to_string(), size, preferred));
            }
        }
    }

    let (exe_url, exe_name, exe_size) = match chosen_exe {
        Some((u, n, sz, _)) => (Some(u), Some(n), sz),
        None => (None, None, 0),
    };

    Some(RustBloxRelease {
        version: normalized_ver,
        tag_name,
        kind,
        title,
        notes,
        html_url,
        exe_url,
        exe_name,
        exe_size,
        hotfix_url,
    })
}

pub fn check_rustblox_update() -> Result<Option<RustBloxRelease>, String> {
    let url = format!("https://api.github.com/repos/{RUSTBLOX_REPO}/releases/latest");
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(6))
        .timeout_read(Duration::from_secs(12))
        .build();
    let resp = match agent
        .get(&url)
        .set("User-Agent", concat!("rustblox/", env!("CARGO_PKG_VERSION")))
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => return Ok(None),
        Err(ureq::Error::Status(code, _)) => {
            return Err(format!("GitHub returned HTTP {code}"));
        }
        Err(e) => return Err(format!("Could not reach update server: {e}")),
    };
    let text = resp.into_string().map_err(|e| e.to_string())?;
    let json: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(parse_github_release_update(&json, env!("CARGO_PKG_VERSION")))
}

pub fn parse_live_hotfix_str(text: &str) -> Result<LiveHotfix, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("Invalid hotfix JSON: {e}"))?;
    let mut fastflag_overrides = HashMap::new();
    if let Some(obj) = v["fastflags"].as_object() {
        for (k, val) in obj {
            fastflag_overrides.insert(k.clone(), val.clone());
        }
    }
    let auto_close_roblox = v["auto_close_roblox"].as_bool();
    let status_note = v["note"]
        .as_str()
        .or_else(|| v["message"].as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Ok(LiveHotfix {
        fastflag_overrides,
        auto_close_roblox,
        status_note,
    })
}

pub fn download_hotfix(url: &str) -> Result<LiveHotfix, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(6))
        .timeout_read(Duration::from_secs(12))
        .build();
    let resp = agent
        .get(url)
        .set("User-Agent", concat!("rustblox/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| format!("Failed fetching hotfix: {e}"))?;
    let text = resp.into_string().map_err(|e| e.to_string())?;
    parse_live_hotfix_str(&text)
}

fn companion_path(exe: &Path, suffix: &str) -> Result<PathBuf, String> {
    let dir = exe
        .parent()
        .ok_or_else(|| "Could not determine application folder.".to_string())?;
    let fname = exe
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Could not determine executable name.".to_string())?;
    Ok(dir.join(format!("{fname}{suffix}")))
}

fn temp_updater_path() -> PathBuf {
    std::env::temp_dir().join("rustblox_updater.exe")
}

pub fn cleanup_old_binary() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    for suffix in [".old", ".new"] {
        if let Ok(path) = companion_path(&exe, suffix) {
            if path.exists() && fs::remove_file(&path).is_err() {
                let path_bg = path.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(550));
                    let _ = fs::remove_file(path_bg);
                });
            }
        }
    }
    let tmp_updater = temp_updater_path();
    if exe != tmp_updater && tmp_updater.exists() && fs::remove_file(&tmp_updater).is_err() {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(700));
            let _ = fs::remove_file(tmp_updater);
        });
    }
}

fn stream_download_exe(
    exe_url: &str,
    dest_path: &Path,
    expected_size: u64,
    downloaded: &AtomicU64,
    total: &AtomicU64,
    cancelled: Option<&AtomicBool>,
    ctx: &egui::Context,
) -> Result<(), String> {
    downloaded.store(0, Ordering::Relaxed);
    total.store(expected_size, Ordering::Relaxed);
    ctx.request_repaint();

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(60))
        .build();
    let resp = agent
        .get(exe_url)
        .set("User-Agent", concat!("rustblox/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    if let Some(len) = resp
        .header("content-length")
        .and_then(|s| s.parse::<u64>().ok())
    {
        if len > 0 {
            total.store(len, Ordering::Relaxed);
        }
    }

    let mut file = fs::File::create(dest_path).map_err(|e| {
        format!(
            "Cannot write to {} ({e}). Move rustblox.exe to a writable folder or update manually.",
            dest_path
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "app folder".to_string())
        )
    })?;

    let mut reader = resp.into_reader();
    let mut buf = [0u8; 64 * 1024];
    let mut bytes_written: u64 = 0;
    let mut header = [0u8; 2];
    let mut header_len = 0usize;

    loop {
        if cancelled.is_some_and(|c| c.load(Ordering::Relaxed)) {
            drop(file);
            let _ = fs::remove_file(dest_path);
            return Err("Update cancelled.".to_string());
        }
        let n = reader.read(&mut buf).map_err(|e| {
            let _ = fs::remove_file(dest_path);
            format!("Download interrupted: {e}")
        })?;
        if n == 0 {
            break;
        }
        if header_len < 2 {
            let take = (2 - header_len).min(n);
            header[header_len..header_len + take].copy_from_slice(&buf[..take]);
            header_len += take;
        }
        file.write_all(&buf[..n]).map_err(|e| {
            let _ = fs::remove_file(dest_path);
            format!("Failed writing update file: {e}")
        })?;
        bytes_written += n as u64;
        downloaded.store(bytes_written, Ordering::Relaxed);
        ctx.request_repaint();
    }
    file.flush().map_err(|e| {
        let _ = fs::remove_file(dest_path);
        format!("Failed flushing update file: {e}")
    })?;
    drop(file);

    if bytes_written < 64 * 1024 || header != *b"MZ" {
        let _ = fs::remove_file(dest_path);
        return Err("Downloaded file is not a valid Windows executable.".to_string());
    }
    Ok(())
}

fn swap_binary_with_retries(target_exe: &Path, staged_exe: &Path) -> Result<(), String> {
    let backup_path = companion_path(target_exe, ".old")?;
    if backup_path.exists() {
        let _ = fs::remove_file(&backup_path);
    }

    let mut last_err = None;
    let mut moved_old = false;
    for _ in 0..45 {
        if !target_exe.exists() {
            moved_old = false;
            last_err = None;
            break;
        }
        match fs::rename(target_exe, &backup_path) {
            Ok(()) => {
                moved_old = true;
                last_err = None;
                break;
            }
            Err(e) => {
                last_err = Some(e);
                thread::sleep(Duration::from_millis(140));
            }
        }
    }
    if let Some(e) = last_err {
        return Err(format!("Could not prepare executable swap: {e}"));
    }

    if let Err(e) = fs::rename(staged_exe, target_exe) {
        if moved_old {
            let _ = fs::rename(&backup_path, target_exe);
        }
        return Err(format!("Could not install updated executable: {e}"));
    }
    Ok(())
}

pub fn apply_small_patch_without_restart(
    release: &RustBloxRelease,
    downloaded: &AtomicU64,
    total: &AtomicU64,
    ctx: &egui::Context,
) -> Result<(Option<LiveHotfix>, Option<PathBuf>), String> {
    let hotfix = match &release.hotfix_url {
        Some(url) => Some(download_hotfix(url)?),
        None => None,
    };
    let staged = if let Some(exe_url) = &release.exe_url {
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Could not locate current executable: {e}"))?;
        let staged_path = companion_path(&current_exe, ".staged")?;
        stream_download_exe(
            exe_url,
            &staged_path,
            release.exe_size,
            downloaded,
            total,
            None,
            ctx,
        )?;
        Some(staged_path)
    } else {
        None
    };
    if hotfix.is_none() && staged.is_none() {
        return Err("This patch release has no downloadable patch or hotfix attached.".to_string());
    }
    Ok((hotfix, staged))
}

pub fn launch_updater_process(release: &RustBloxRelease) -> Result<(), String> {
    let exe_url = release
        .exe_url
        .as_deref()
        .ok_or_else(|| "This release has no Windows .exe download attached.".to_string())?;
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Could not locate current executable: {e}"))?;
    let helper_exe = temp_updater_path();
    fs::copy(&current_exe, &helper_exe)
        .map_err(|e| format!("Could not start updater helper: {e}"))?;

    Command::new(&helper_exe)
        .arg("--updater")
        .arg(&current_exe)
        .arg(env!("CARGO_PKG_VERSION"))
        .arg(&release.version)
        .arg(exe_url)
        .arg(release.exe_size.to_string())
        .spawn()
        .map_err(|e| format!("Could not launch updater window: {e}"))?;
    Ok(())
}

pub fn launch_quiet_apply_on_exit(staged_exe: &Path) {
    if !staged_exe.exists() {
        return;
    }
    let Ok(current_exe) = std::env::current_exe() else {
        return;
    };
    let helper_exe = temp_updater_path();
    if fs::copy(&current_exe, &helper_exe).is_err() {
        return;
    }
    let mut cmd = Command::new(&helper_exe);
    cmd.arg("--quiet-apply").arg(&current_exe).arg(staged_exe);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let _ = cmd.spawn();
}

pub fn try_parse_updater_cli(args: &[String]) -> Option<UpdaterArgs> {
    if args.get(1).map(|s| s.as_str()) != Some("--updater") || args.len() < 7 {
        return None;
    }
    Some(UpdaterArgs {
        target_exe: PathBuf::from(&args[2]),
        from_ver: args[3].clone(),
        to_ver: args[4].clone(),
        exe_url: args[5].clone(),
        exe_size: args[6].parse::<u64>().unwrap_or(0),
    })
}

pub fn try_run_quiet_apply_cli(args: &[String]) -> bool {
    if args.get(1).map(|s| s.as_str()) != Some("--quiet-apply") || args.len() < 4 {
        return false;
    }
    let target_exe = PathBuf::from(&args[2]);
    let staged_exe = PathBuf::from(&args[3]);
    if staged_exe.exists() {
        if swap_binary_with_retries(&target_exe, &staged_exe).is_err() {
            let _ = fs::remove_file(&staged_exe);
        }
    }
    true
}

#[derive(Clone, Debug)]
enum UpdaterPhase {
    Downloading,
    Installing,
    Launching,
    Failed(String),
}

struct UpdaterShared {
    phase: UpdaterPhase,
}

struct RustBloxUpdaterApp {
    args: UpdaterArgs,
    downloaded: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
    cancelled: Arc<AtomicBool>,
    shared: Arc<Mutex<UpdaterShared>>,
    smooth_progress: f32,
    started_at: Instant,
    worker_started: bool,
}

impl RustBloxUpdaterApp {
    fn new(args: UpdaterArgs) -> Self {
        let total = args.exe_size;
        Self {
            args,
            downloaded: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(total)),
            cancelled: Arc::new(AtomicBool::new(false)),
            shared: Arc::new(Mutex::new(UpdaterShared {
                phase: UpdaterPhase::Downloading,
            })),
            smooth_progress: 0.0,
            started_at: Instant::now(),
            worker_started: false,
        }
    }

    fn start_worker(&mut self, ctx: &egui::Context) {
        self.worker_started = true;
        self.cancelled.store(false, Ordering::Relaxed);
        self.downloaded.store(0, Ordering::Relaxed);
        self.total.store(self.args.exe_size, Ordering::Relaxed);
        if let Ok(mut guard) = self.shared.lock() {
            guard.phase = UpdaterPhase::Downloading;
        }

        let args = self.args.clone();
        let downloaded = Arc::clone(&self.downloaded);
        let total = Arc::clone(&self.total);
        let cancelled = Arc::clone(&self.cancelled);
        let shared = Arc::clone(&self.shared);
        let ctx_bg = ctx.clone();

        thread::spawn(move || {
            let staging = match companion_path(&args.target_exe, ".new") {
                Ok(p) => p,
                Err(e) => {
                    if let Ok(mut g) = shared.lock() {
                        g.phase = UpdaterPhase::Failed(e);
                    }
                    ctx_bg.request_repaint();
                    return;
                }
            };

            if let Err(e) = stream_download_exe(
                &args.exe_url,
                &staging,
                args.exe_size,
                &downloaded,
                &total,
                Some(&cancelled),
                &ctx_bg,
            ) {
                if let Ok(mut g) = shared.lock() {
                    g.phase = UpdaterPhase::Failed(e);
                }
                ctx_bg.request_repaint();
                return;
            }

            if let Ok(mut g) = shared.lock() {
                g.phase = UpdaterPhase::Installing;
            }
            ctx_bg.request_repaint();

            if let Err(e) = swap_binary_with_retries(&args.target_exe, &staging) {
                let _ = fs::remove_file(&staging);
                if let Ok(mut g) = shared.lock() {
                    g.phase = UpdaterPhase::Failed(e);
                }
                ctx_bg.request_repaint();
                return;
            }

            if let Ok(mut g) = shared.lock() {
                g.phase = UpdaterPhase::Launching;
            }
            ctx_bg.request_repaint();
            thread::sleep(Duration::from_millis(420));

            if let Err(e) = Command::new(&args.target_exe).spawn() {
                if let Ok(mut g) = shared.lock() {
                    g.phase = UpdaterPhase::Failed(format!(
                        "Update installed, but relaunch failed: {e}"
                    ));
                }
                ctx_bg.request_repaint();
                return;
            }

            ctx_bg.send_viewport_cmd(egui::ViewportCommand::Close);
        });
    }
}

impl eframe::App for RustBloxUpdaterApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.worker_started {
            self.start_worker(ui.ctx());
        }

        let p = pal(ui);
        let settings = load_settings();
        let accent = theme_accent(&settings);
        let phase = self
            .shared
            .lock()
            .map(|g| g.phase.clone())
            .unwrap_or(UpdaterPhase::Downloading);

        let screen = ui.ctx().input(|i| i.viewport_rect());
        let dt = ui.ctx().input(|i| i.unstable_dt).min(0.05);
        let t = self.started_at.elapsed().as_secs_f32();

        let target_frac = match &phase {
            UpdaterPhase::Downloading => {
                let done = self.downloaded.load(Ordering::Relaxed);
                let total = self.total.load(Ordering::Relaxed);
                if total > 0 {
                    (done as f32 / total as f32).clamp(0.03, 0.90)
                } else {
                    0.22
                }
            }
            UpdaterPhase::Installing => 0.95,
            UpdaterPhase::Launching => 1.0,
            UpdaterPhase::Failed(_) => self.smooth_progress,
        };
        self.smooth_progress += (target_frac - self.smooth_progress) * (dt * 9.0).clamp(0.0, 1.0);

        let card_rect = screen.shrink(4.0);
        let painter = ui.painter();
        painter.rect_filled(card_rect, CornerRadius::same(14), p.bg);
        painter.rect_stroke(
            card_rect,
            CornerRadius::same(14),
            egui::Stroke::new(1.0, p.border),
            egui::StrokeKind::Inside,
        );

        let drag_resp = ui.interact(
            card_rect,
            ui.scope_id().with("updater_drag"),
            Sense::drag(),
        );
        if drag_resp.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::same(22)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close_btn = ui.add(
                            egui::Button::new(RichText::new("×").size(14.0).color(p.muted))
                                .fill(Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE),
                        );
                        if close_btn.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            self.cancelled.store(true, Ordering::Relaxed);
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });

                ui.vertical_centered(|ui| {
                    let (logo_wrap, _) =
                        ui.allocate_exact_size(egui::vec2(62.0, 62.0), Sense::hover());
                    let pulse = 0.5 + 0.5 * (t * 2.4).sin();
                    let halo_alpha = (18.0 + pulse * 18.0) as u8;
                    ui.painter().circle_filled(
                        logo_wrap.center(),
                        30.0,
                        Color32::from_rgba_unmultiplied(
                            accent.r(),
                            accent.g(),
                            accent.b(),
                            halo_alpha,
                        ),
                    );
                    let logo_rect =
                        egui::Rect::from_center_size(logo_wrap.center(), egui::vec2(46.0, 46.0));
                    egui::Image::new(egui::include_image!("../assets/logo.png"))
                        .corner_radius(10)
                        .paint_at(ui, logo_rect);

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Updating RustBlox")
                            .size(18.0)
                            .strong()
                            .color(p.text),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!("v{}  ->  v{}", self.args.from_ver, self.args.to_ver))
                            .size(12.0)
                            .color(p.muted),
                    );

                    ui.add_space(16.0);

                    match &phase {
                        UpdaterPhase::Failed(err) => {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(err).size(11.5).color(Color32::from_rgb(248, 113, 113)),
                                )
                                .wrap(),
                            );
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if ui.button("Retry").clicked() {
                                    self.start_worker(ui.ctx());
                                }
                                if ui.button("Close").clicked() {
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            });
                        }
                        _ => {
                            let (bar_rect, _) =
                                ui.allocate_exact_size(egui::vec2(280.0, 7.0), Sense::hover());
                            ui.painter().rect_filled(
                                bar_rect,
                                CornerRadius::same(4),
                                p.surface_bg,
                            );
                            let fill_w = (bar_rect.width() * self.smooth_progress).max(8.0);
                            let fill_rect = egui::Rect::from_min_size(
                                bar_rect.min,
                                egui::vec2(fill_w.min(bar_rect.width()), bar_rect.height()),
                            );
                            ui.painter().rect_filled(fill_rect, CornerRadius::same(4), accent);

                            ui.add_space(8.0);
                            let status_txt = match phase {
                                UpdaterPhase::Downloading => {
                                    let done = self.downloaded.load(Ordering::Relaxed);
                                    let total = self.total.load(Ordering::Relaxed);
                                    if total > 0 {
                                        let pct = ((done as f64 / total as f64) * 100.0).min(100.0);
                                        format!(
                                            "Downloading update · {:.1} / {:.1} MB ({:.0}%)",
                                            done as f64 / (1024.0 * 1024.0),
                                            total as f64 / (1024.0 * 1024.0),
                                            pct
                                        )
                                    } else {
                                        "Downloading update...".to_string()
                                    }
                                }
                                UpdaterPhase::Installing => "Installing update...".to_string(),
                                UpdaterPhase::Launching => "Starting RustBlox...".to_string(),
                                UpdaterPhase::Failed(_) => String::new(),
                            };
                            ui.label(RichText::new(status_txt).size(11.5).color(p.muted));
                        }
                    }
                });
            });

        if !matches!(phase, UpdaterPhase::Failed(_)) {
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
    }
}

pub fn run_updater_window(args: UpdaterArgs) -> eframe::Result<()> {
    let settings = load_settings();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([392.0, 246.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/logo.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Updating RustBlox",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            install_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx, &settings);
            Ok(Box::new(RustBloxUpdaterApp::new(args)))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_parsing_and_comparison_work() {
        assert_eq!(parse_semver("v0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_semver("0.2.5-beta.1"), Some((0, 2, 5)));
        assert_eq!(parse_semver("1.4"), Some((1, 4, 0)));
        assert_eq!(parse_semver(""), None);

        assert!(is_newer_version("v0.1.1", "0.1.0"));
        assert!(is_newer_version("v0.2.0", "0.1.9"));
        assert!(is_newer_version("1.0.0", "0.99.99"));
        assert!(!is_newer_version("v0.1.0", "0.1.0"));
        assert!(!is_newer_version("v0.0.9", "0.1.0"));

        assert_eq!(
            classify_update_kind("v0.1.1", "0.1.0", "Bug fixes", "Fixed crash"),
            UpdateKind::Patch
        );
        assert_eq!(
            classify_update_kind("v0.2.0", "0.1.0", "New UI", "Redesigned cards"),
            UpdateKind::Full
        );
        assert_eq!(
            classify_update_kind("v0.1.1", "0.1.0", "Update [full]", "Major change"),
            UpdateKind::Full
        );
    }

    #[test]
    fn parses_github_release_and_picks_exe_and_hotfix() {
        let json = serde_json::json!({
            "tag_name": "v0.1.2",
            "name": "Small Bugfix Patch",
            "body": "- Fixed friend sync\n- Live FastFlag hotfix",
            "draft": false,
            "html_url": "https://github.com/gothssmy-design/rustblox/releases/tag/v0.1.2",
            "assets": [
                {
                    "name": "hotfix.json",
                    "size": 512,
                    "browser_download_url": "https://example.com/hotfix.json"
                },
                {
                    "name": "rustblox.exe",
                    "size": 8500000,
                    "browser_download_url": "https://example.com/rustblox.exe"
                }
            ]
        });
        let rel = parse_github_release_update(&json, "0.1.0").expect("should detect newer release");
        assert_eq!(rel.version, "0.1.2");
        assert_eq!(rel.kind, UpdateKind::Patch);
        assert_eq!(rel.exe_name.as_deref(), Some("rustblox.exe"));
        assert_eq!(
            rel.exe_url.as_deref(),
            Some("https://example.com/rustblox.exe")
        );
        assert_eq!(
            rel.hotfix_url.as_deref(),
            Some("https://example.com/hotfix.json")
        );
        assert_eq!(rel.exe_size, 8_500_000);
        assert!(parse_github_release_update(&json, "0.1.2").is_none());
    }

    #[test]
    fn parses_live_hotfix_payload() {
        let raw = r#"{
            "fastflags": {"DFIntTaskSchedulerTargetFps": 240, "FFlagDebugDisableTelemetry": true},
            "auto_close_roblox": true,
            "note": "Applied FPS & telemetry patch live."
        }"#;
        let hf = parse_live_hotfix_str(raw).expect("hotfix json should parse");
        assert_eq!(hf.fastflag_overrides.len(), 2);
        assert_eq!(hf.auto_close_roblox, Some(true));
        assert_eq!(
            hf.status_note.as_deref(),
            Some("Applied FPS & telemetry patch live.")
        );
    }

    #[test]
    fn streams_download_validates_pe_and_swaps_target_exe() {
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().unwrap();

        let mut fake_pe = vec![0x90u8; 80 * 1024];
        fake_pe[0] = b'M';
        fake_pe[1] = b'Z';
        fake_pe[2..6].copy_from_slice(b"V011");

        let server_payload = fake_pe.clone();
        let server = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req = [0u8; 1024];
                let _ = stream.read(&mut req);
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    server_payload.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&server_payload);
            }
        });

        let dir = std::env::temp_dir().join(format!(
            "rustblox_updater_test_{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        let target_exe = dir.join("rustblox.exe");
        let staged_exe = dir.join("rustblox.exe.new");
        fs::write(&target_exe, b"OLD_VERSION_BYTES").unwrap();

        let dl = AtomicU64::new(0);
        let total = AtomicU64::new(0);
        let ctx = egui::Context::default();
        let url = format!("http://{addr}/rustblox.exe");
        stream_download_exe(
            &url,
            &staged_exe,
            fake_pe.len() as u64,
            &dl,
            &total,
            None,
            &ctx,
        )
        .expect("stream download should succeed");

        assert_eq!(dl.load(Ordering::Relaxed), fake_pe.len() as u64);
        assert_eq!(total.load(Ordering::Relaxed), fake_pe.len() as u64);

        swap_binary_with_retries(&target_exe, &staged_exe).expect("binary swap should succeed");
        let swapped = fs::read(&target_exe).unwrap();
        assert_eq!(swapped.len(), fake_pe.len());
        assert_eq!(&swapped[0..6], b"MZV011");
        let backup = fs::read(dir.join("rustblox.exe.old")).unwrap();
        assert_eq!(backup, b"OLD_VERSION_BYTES");

        let _ = server.join();
        let _ = fs::remove_dir_all(&dir);
    }
}
