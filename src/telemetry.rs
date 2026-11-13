use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind};

use crate::theme::{accent_color, is_dark_color, pal, BLUE, GREEN, ORANGE, RED};

const DOCK_WIDTH: f32 = 64.0;
const SAMPLE_INTERVAL: Duration = Duration::from_millis(450);
const FPS_HISTORY_INTERVAL: Duration = Duration::from_millis(160);
const FPS_HISTORY_CAP: usize = 28;
const CPU_HISTORY_CAP: usize = 24;

#[derive(Clone)]
struct TelemetryHandle(Arc<Mutex<TelemetryState>>);

#[derive(Clone, Copy)]
struct CpuRawTimes {
    idle: u64,
    kernel: u64,
    user: u64,
    proc_kernel: u64,
    proc_user: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TelemetrySnapshot {
    pub fps: f32,
    pub frame_ms: f32,
    pub ram_pct: f32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub proc_ram_mb: f32,
    pub cpu_pct: f32,
    pub proc_cpu_pct: f32,
    pub cpu_cores: usize,
}

impl Default for TelemetrySnapshot {
    fn default() -> Self {
        Self {
            fps: 60.0,
            frame_ms: 16.67,
            ram_pct: 0.0,
            ram_used_gb: 0.0,
            ram_total_gb: 0.0,
            proc_ram_mb: 0.0,
            cpu_pct: 0.0,
            proc_cpu_pct: 0.0,
            cpu_cores: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        }
    }
}

struct TelemetryState {
    started_at: Instant,
    last_frame: Option<Instant>,
    last_sample: Option<Instant>,
    last_fps_push: Option<Instant>,
    prev_cpu: Option<CpuRawTimes>,
    frame_times: VecDeque<Instant>,
    fps_history: VecDeque<f32>,
    cpu_history: VecDeque<f32>,
    snapshot: TelemetrySnapshot,
    anim_fps: f32,
    anim_ram: f32,
    anim_cpu: f32,
    rotor_phase: f32,
    show_gb_mode: bool,
}

impl Default for TelemetryState {
    fn default() -> Self {
        let now = Instant::now();
        let mut fps_history = VecDeque::with_capacity(FPS_HISTORY_CAP);
        for _ in 0..FPS_HISTORY_CAP {
            fps_history.push_back(60.0);
        }
        let mut cpu_history = VecDeque::with_capacity(CPU_HISTORY_CAP);
        for _ in 0..CPU_HISTORY_CAP {
            cpu_history.push_back(0.0);
        }
        let mut s = Self {
            started_at: now,
            last_frame: None,
            last_sample: None,
            last_fps_push: None,
            prev_cpu: read_raw_cpu_times(),
            frame_times: VecDeque::with_capacity(160),
            fps_history,
            cpu_history,
            snapshot: TelemetrySnapshot::default(),
            anim_fps: 60.0,
            anim_ram: 0.0,
            anim_cpu: 0.0,
            rotor_phase: 0.0,
            show_gb_mode: false,
        };
        s.refresh_os_metrics();
        s.anim_ram = s.snapshot.ram_pct;
        s.anim_cpu = s.snapshot.cpu_pct;
        s
    }
}

impl TelemetryState {
    fn tick(&mut self, now: Instant) -> f32 {
        let raw_dt = self
            .last_frame
            .map(|prev| now.saturating_duration_since(prev).as_secs_f32())
            .unwrap_or(1.0 / 60.0);
        self.last_frame = Some(now);

        if raw_dt > 0.9 {
            self.frame_times.clear();
        }
        self.frame_times.push_back(now);
        while let Some(&front) = self.frame_times.front() {
            if now.saturating_duration_since(front) > Duration::from_secs(1) {
                self.frame_times.pop_front();
            } else {
                break;
            }
        }

        let inst_fps = if raw_dt > 0.0002 && raw_dt < 0.5 {
            (1.0 / raw_dt).clamp(1.0, 999.0)
        } else {
            self.snapshot.fps
        };

        let measured_fps = if let Some(&oldest) = self.frame_times.front() {
            let span = now.saturating_duration_since(oldest).as_secs_f32();
            if self.frame_times.len() >= 4 && span >= 0.15 {
                let rolling = ((self.frame_times.len() - 1) as f32 / span).clamp(1.0, 999.0);
                rolling * 0.72 + inst_fps * 0.28
            } else {
                inst_fps
            }
        } else {
            inst_fps
        };

        self.snapshot.fps = self.snapshot.fps * 0.82 + measured_fps * 0.18;
        self.snapshot.frame_ms = 1000.0 / self.snapshot.fps.max(1.0);

        let anim_dt = raw_dt.clamp(0.001, 0.05);
        let smooth_k = (anim_dt * 9.0).clamp(0.05, 0.45);
        self.anim_fps += (self.snapshot.fps - self.anim_fps) * smooth_k;
        self.anim_ram += (self.snapshot.ram_pct - self.anim_ram) * smooth_k;
        self.anim_cpu += (self.snapshot.cpu_pct - self.anim_cpu) * smooth_k;

        let load = (self.anim_cpu / 100.0).clamp(0.0, 1.0);
        self.rotor_phase = (self.rotor_phase + anim_dt * (1.8 + load * 6.5)) % std::f32::consts::TAU;

        if self
            .last_fps_push
            .map(|t| now.saturating_duration_since(t) >= FPS_HISTORY_INTERVAL)
            .unwrap_or(true)
        {
            self.last_fps_push = Some(now);
            if self.fps_history.len() >= FPS_HISTORY_CAP {
                self.fps_history.pop_front();
            }
            self.fps_history.push_back(self.anim_fps);
        }

        if self
            .last_sample
            .map(|t| now.saturating_duration_since(t) >= SAMPLE_INTERVAL)
            .unwrap_or(true)
        {
            self.last_sample = Some(now);
            self.refresh_os_metrics();
        }

        raw_dt
    }

    fn refresh_os_metrics(&mut self) {
        if let Some((pct, used_gb, total_gb)) = read_system_memory() {
            self.snapshot.ram_pct = pct;
            self.snapshot.ram_used_gb = used_gb;
            self.snapshot.ram_total_gb = total_gb;
        }
        if let Some(mb) = read_process_memory_mb() {
            self.snapshot.proc_ram_mb = mb;
        }
        if let Some(cur) = read_raw_cpu_times() {
            if let Some(prev) = self.prev_cpu {
                let (sys_pct, proc_pct) = compute_cpu_percentages(prev, cur);
                self.snapshot.cpu_pct = sys_pct;
                self.snapshot.proc_cpu_pct = proc_pct;
                if self.cpu_history.len() >= CPU_HISTORY_CAP {
                    self.cpu_history.pop_front();
                }
                self.cpu_history.push_back(sys_pct);
            }
            self.prev_cpu = Some(cur);
        }
    }
}

fn compute_cpu_percentages(prev: CpuRawTimes, cur: CpuRawTimes) -> (f32, f32) {
    let d_idle = cur.idle.saturating_sub(prev.idle);
    let d_kernel = cur.kernel.saturating_sub(prev.kernel);
    let d_user = cur.user.saturating_sub(prev.user);
    let d_total = d_kernel.saturating_add(d_user);
    if d_total == 0 {
        return (0.0, 0.0);
    }
    // On Windows GetSystemTimes, kernel time includes idle time.
    let busy = d_total.saturating_sub(d_idle);
    let sys_pct = ((busy as f64 / d_total as f64) * 100.0).clamp(0.0, 100.0) as f32;

    let d_proc = cur
        .proc_kernel
        .saturating_sub(prev.proc_kernel)
        .saturating_add(cur.proc_user.saturating_sub(prev.proc_user));
    let proc_pct = ((d_proc as f64 / d_total as f64) * 100.0).clamp(0.0, 100.0) as f32;
    (sys_pct, proc_pct)
}

#[cfg(windows)]
#[inline]
fn filetime_to_u64(ft: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

#[cfg(windows)]
fn read_raw_cpu_times() -> Option<CpuRawTimes> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes, GetSystemTimes};

    unsafe {
        let mut idle = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut kernel = idle;
        let mut user = idle;
        if GetSystemTimes(&mut idle, &mut kernel, &mut user) == 0 {
            return None;
        }

        let mut p_create = idle;
        let mut p_exit = idle;
        let mut p_kernel = idle;
        let mut p_user = idle;
        let got_proc = GetProcessTimes(
            GetCurrentProcess(),
            &mut p_create,
            &mut p_exit,
            &mut p_kernel,
            &mut p_user,
        ) != 0;

        Some(CpuRawTimes {
            idle: filetime_to_u64(idle),
            kernel: filetime_to_u64(kernel),
            user: filetime_to_u64(user),
            proc_kernel: if got_proc { filetime_to_u64(p_kernel) } else { 0 },
            proc_user: if got_proc { filetime_to_u64(p_user) } else { 0 },
        })
    }
}

#[cfg(not(windows))]
fn read_raw_cpu_times() -> Option<CpuRawTimes> {
    None
}

#[cfg(windows)]
fn read_system_memory() -> Option<(f32, f32, f32)> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    unsafe {
        let mut mem: MEMORYSTATUSEX = std::mem::zeroed();
        mem.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut mem) == 0 || mem.ullTotalPhys == 0 {
            return None;
        }
        let total = mem.ullTotalPhys as f64;
        let avail = mem.ullAvailPhys as f64;
        let used = (total - avail).max(0.0);
        let gb = 1024.0 * 1024.0 * 1024.0;
        let pct = ((used / total) * 100.0).clamp(0.0, 100.0) as f32;
        Some((pct, (used / gb) as f32, (total / gb) as f32))
    }
}

#[cfg(not(windows))]
fn read_system_memory() -> Option<(f32, f32, f32)> {
    None
}

#[cfg(windows)]
fn read_process_memory_mb() -> Option<f32> {
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    unsafe {
        let mut pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb) == 0 {
            return None;
        }
        Some((pmc.WorkingSetSize as f64 / (1024.0 * 1024.0)) as f32)
    }
}

#[cfg(not(windows))]
fn read_process_memory_mb() -> Option<f32> {
    None
}

fn mix_c32(a: Color32, b: Color32, t: f32, alpha: u8) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 * inv + b.r() as f32 * t).round().clamp(0.0, 255.0) as u8,
        (a.g() as f32 * inv + b.g() as f32 * t).round().clamp(0.0, 255.0) as u8,
        (a.b() as f32 * inv + b.b() as f32 * t).round().clamp(0.0, 255.0) as u8,
        alpha,
    )
}

fn readable_tint(color: Color32, bg: Color32) -> Color32 {
    let bg_dark = is_dark_color([bg.r(), bg.g(), bg.b()]);
    let c_dark = is_dark_color([color.r(), color.g(), color.b()]);
    if bg_dark && c_dark {
        mix_c32(color, Color32::WHITE, 0.55, 255)
    } else if !bg_dark && !c_dark {
        mix_c32(color, Color32::from_rgb(16, 20, 28), 0.36, 255)
    } else {
        color
    }
}

fn fps_status_color(fps: f32, accent: Color32, bg: Color32) -> Color32 {
    let base = if fps >= 45.0 {
        // Blend slightly with theme accent when healthy so the dock belongs to any palette.
        mix_c32(GREEN, accent, 0.25, 255)
    } else if fps >= 25.0 {
        ORANGE
    } else {
        RED
    };
    readable_tint(base, bg)
}

fn load_status_color(pct: f32, healthy: Color32, bg: Color32) -> Color32 {
    let base = if pct >= 88.0 {
        RED
    } else if pct >= 72.0 {
        ORANGE
    } else {
        healthy
    };
    readable_tint(base, bg)
}

fn format_gb_short(gb: f32) -> String {
    if gb >= 10.0 {
        format!("{gb:.0}G")
    } else {
        format!("{gb:.1}G")
    }
}

fn paint_module_header(
    painter: &egui::Painter,
    x_left: f32,
    x_right: f32,
    y_center: f32,
    label_x_offset: f32,
    label: &str,
    value: &str,
    muted: Color32,
    val_color: Color32,
) {
    painter.text(
        Pos2::new(x_left + label_x_offset, y_center),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::new(8.8, egui::FontFamily::Name("Bold".into())),
        muted,
    );
    painter.text(
        Pos2::new(x_right, y_center),
        egui::Align2::RIGHT_CENTER,
        value,
        FontId::new(10.5, egui::FontFamily::Name("Bold".into())),
        val_color,
    );
}

fn paint_fps_sparkline(
    painter: &egui::Painter,
    well: Rect,
    history: &VecDeque<f32>,
    color: Color32,
    surface_bg: Color32,
    border: Color32,
    t: f32,
) {
    painter.rect_filled(well, CornerRadius::same(5), surface_bg);
    painter.rect_stroke(
        well,
        CornerRadius::same(5),
        Stroke::new(0.8, border),
        StrokeKind::Inside,
    );

    if history.len() < 2 {
        return;
    }

    let inner = well.shrink2(egui::vec2(2.5, 2.5));
    let max_val = history
        .iter()
        .copied()
        .fold(60.0_f32, f32::max)
        .max(60.0)
        * 1.15;

    let n = history.len();
    let mut pts = Vec::with_capacity(n);
    for (i, &v) in history.iter().enumerate() {
        let x = inner.left() + (i as f32 / (n - 1) as f32) * inner.width();
        let norm = (v / max_val).clamp(0.06, 0.96);
        let y = inner.bottom() - norm * inner.height();
        pts.push(Pos2::new(x, y));
    }

    // Soft vertical gradient fill beneath the FPS curve.
    let top_col = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 68);
    let bot_col = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 4);
    let mut mesh = egui::Mesh::default();
    for i in 0..(pts.len() - 1) {
        let p0 = pts[i];
        let p1 = pts[i + 1];
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(p0, top_col);
        mesh.colored_vertex(p1, top_col);
        mesh.colored_vertex(Pos2::new(p1.x, inner.bottom()), bot_col);
        mesh.colored_vertex(Pos2::new(p0.x, inner.bottom()), bot_col);
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    painter.add(egui::Shape::mesh(mesh));

    // Subtle soft glow stroke under the sharp line.
    painter.add(egui::Shape::line(
        pts.clone(),
        Stroke::new(
            2.8,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 42),
        ),
    ));
    painter.add(egui::Shape::line(pts.clone(), Stroke::new(1.4, color)));

    if let Some(&last) = pts.last() {
        let pulse = 0.5 + 0.5 * (t * 4.5).sin();
        painter.circle_filled(
            last,
            2.4 + pulse * 1.9,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 65),
        );
        painter.circle_filled(last, 1.9, mix_c32(color, Color32::WHITE, 0.45, 255));
    }
}

fn paint_ram_meter(
    painter: &egui::Painter,
    well: Rect,
    ram_pct: f32,
    color: Color32,
    surface_bg: Color32,
    border: Color32,
    t: f32,
) {
    painter.rect_filled(well, CornerRadius::same(4), surface_bg);
    painter.rect_stroke(
        well,
        CornerRadius::same(4),
        Stroke::new(0.8, border),
        StrokeKind::Inside,
    );

    // Subtle gauge quartile tick marks inside the track.
    let tick_col = Color32::from_rgba_unmultiplied(border.r(), border.g(), border.b(), 95);
    for q in [0.25_f32, 0.5, 0.75] {
        let tx = well.left() + well.width() * q;
        painter.line_segment(
            [Pos2::new(tx, well.top() + 2.0), Pos2::new(tx, well.bottom() - 2.0)],
            Stroke::new(0.7, tick_col),
        );
    }

    let frac = (ram_pct / 100.0).clamp(0.05, 1.0);
    let fill_w = (well.width() - 2.0) * frac;
    let fill_rect = Rect::from_min_size(
        Pos2::new(well.left() + 1.0, well.top() + 1.0),
        egui::vec2(fill_w.max(4.0), (well.height() - 2.0).max(4.0)),
    );

    let fill_dark = mix_c32(color, Color32::from_rgb(12, 16, 24), 0.25, 225);
    painter.rect_filled(fill_rect, CornerRadius::same(3), fill_dark);

    let top_half = Rect::from_min_max(
        fill_rect.min,
        Pos2::new(fill_rect.max.x, fill_rect.min.y + fill_rect.height() * 0.58),
    );
    painter.rect_filled(
        top_half,
        CornerRadius::same(3),
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 230),
    );

    // Traveling liquid shimmer wave clipped cleanly to the filled bar.
    let clipped = painter.with_clip_rect(fill_rect);
    let span = fill_rect.width() + 28.0;
    let phase = (t * 0.72).fract();
    let shimmer_cx = fill_rect.left() - 14.0 + phase * span;
    for (offset, w, alpha) in [(-4.0_f32, 14.0_f32, 32_u8), (0.0, 6.0, 95)] {
        let band = Rect::from_center_size(
            Pos2::new(shimmer_cx + offset, fill_rect.center().y),
            egui::vec2(w, fill_rect.height()),
        );
        clipped.rect_filled(
            band,
            CornerRadius::same(2),
            Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
        );
    }

    // Crisp leading highlight cap at the current RAM level.
    if fill_rect.width() > 6.0 {
        let cap_x = fill_rect.max.x - 1.2;
        clipped.line_segment(
            [
                Pos2::new(cap_x, fill_rect.top() + 1.5),
                Pos2::new(cap_x, fill_rect.bottom() - 1.5),
            ],
            Stroke::new(1.5, mix_c32(color, Color32::WHITE, 0.6, 230)),
        );
    }
}

fn paint_cpu_equalizer(
    painter: &egui::Painter,
    well: Rect,
    cpu_pct: f32,
    history: &VecDeque<f32>,
    color: Color32,
    surface_bg: Color32,
    border: Color32,
    t: f32,
) {
    painter.rect_filled(well, CornerRadius::same(5), surface_bg);
    painter.rect_stroke(
        well,
        CornerRadius::same(5),
        Stroke::new(0.8, border),
        StrokeKind::Inside,
    );

    let inner = well.shrink2(egui::vec2(3.0, 2.5));
    let bars = 8usize;
    let gap = 2.0_f32;
    let bar_w = ((inner.width() - gap * (bars - 1) as f32) / bars as f32).max(2.0);
    let load = (cpu_pct / 100.0).clamp(0.0, 1.0);
    let wave_speed = 2.4 + load * 6.0;

    for i in 0..bars {
        // Blend live historical CPU samples with an animated traveling harmonic wave.
        let hist_idx = if history.is_empty() {
            0
        } else {
            let start = history.len().saturating_sub(bars);
            (start + i).min(history.len() - 1)
        };
        let hist_frac = history
            .get(hist_idx)
            .map(|v| (*v / 100.0).clamp(0.0, 1.0))
            .unwrap_or(load);

        let wave1 = 0.5 + 0.5 * (t * wave_speed - i as f32 * 0.72).sin();
        let wave2 = 0.5 + 0.5 * (t * wave_speed * 1.45 + i as f32 * 1.15).cos();
        let harmonic = wave1 * 0.65 + wave2 * 0.35;

        let active_level = (load * 0.65 + hist_frac * 0.35).clamp(0.04, 1.0);
        let h_frac = (0.14 + active_level * (0.42 + 0.44 * harmonic)).clamp(0.14, 1.0);

        let x = inner.left() + i as f32 * (bar_w + gap);
        let bar_h = (inner.height() * h_frac).max(2.5);
        let bar_rect = Rect::from_min_max(
            Pos2::new(x, inner.bottom() - bar_h),
            Pos2::new(x + bar_w, inner.bottom()),
        );

        let alpha = (130.0 + h_frac * 125.0).round().clamp(110.0, 255.0) as u8;
        let bar_col = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
        painter.rect_filled(bar_rect, CornerRadius::same(2), bar_col);

        // Bright top crown on energetic peaks.
        if h_frac > 0.35 {
            let cap = Rect::from_min_max(
                bar_rect.min,
                Pos2::new(bar_rect.max.x, (bar_rect.min.y + 2.0).min(bar_rect.max.y)),
            );
            painter.rect_filled(
                cap,
                CornerRadius::same(2),
                mix_c32(color, Color32::WHITE, 0.45, 220),
            );
        }
    }
}

fn paint_cpu_rotor(painter: &egui::Painter, center: Pos2, radius: f32, phase: f32, color: Color32) {
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 80),
        ),
    );
    for k in 0..3 {
        let ang = phase + k as f32 * (std::f32::consts::TAU / 3.0);
        let p_in = Pos2::new(center.x + ang.cos() * 1.1, center.y + ang.sin() * 1.1);
        let p_out = Pos2::new(
            center.x + ang.cos() * radius,
            center.y + ang.sin() * radius,
        );
        painter.line_segment([p_in, p_out], Stroke::new(1.3, color));
    }
    painter.circle_filled(center, 1.3, color);
}

fn paint_ram_icon(painter: &egui::Painter, left_center: Pos2, color: Color32) {
    let r = Rect::from_center_size(Pos2::new(left_center.x + 3.0, left_center.y), egui::vec2(6.5, 6.5));
    painter.rect_stroke(r, CornerRadius::same(1), Stroke::new(1.0, color), StrokeKind::Inside);
    painter.line_segment(
        [
            Pos2::new(r.left() + 1.5, r.center().y),
            Pos2::new(r.right() - 1.5, r.center().y),
        ],
        Stroke::new(1.0, color),
    );
}

pub fn draw_sidebar_telemetry(ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
    let id = egui::Id::NULL.with("rustblox_sidebar_telemetry");
    let handle = ctx.data_mut(|d| {
        d.get_temp_mut_or_insert_with(id, || TelemetryHandle(Arc::new(Mutex::new(TelemetryState::default()))))
            .clone()
    });

    let now = Instant::now();
    let (snap, anim_fps, anim_ram, anim_cpu, rotor_phase, t, fps_hist, cpu_hist, show_gb, raw_dt) = {
        let mut guard = handle.0.lock().unwrap();
        let raw_dt = guard.tick(now);
        (
            guard.snapshot,
            guard.anim_fps,
            guard.anim_ram,
            guard.anim_cpu,
            guard.rotor_phase,
            guard.started_at.elapsed().as_secs_f32(),
            guard.fps_history.clone(),
            guard.cpu_history.clone(),
            guard.show_gb_mode,
            raw_dt,
        )
    };

    // Pace smooth animation cleanly without pegging uncapped >500Hz setups at 100% CPU.
    if minimized {
        ctx.request_repaint_after(Duration::from_millis(500));
    } else if raw_dt < 0.002 {
        ctx.request_repaint_after(Duration::from_millis(8));
    } else {
        ctx.request_repaint();
    }

    let avail_h = ui.available_height();
    if avail_h < 108.0 {
        return;
    }

    let compact = avail_h < 166.0;
    let dock_h = if compact { 126.0 } else { 166.0 };

    let p = pal(ui);
    let accent = accent_color(ui);

    let (rect, resp) = ui.allocate_exact_size(egui::vec2(DOCK_WIDTH, dock_h), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

    if resp.clicked() {
        let mut guard = handle.0.lock().unwrap();
        guard.show_gb_mode = !guard.show_gb_mode;
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let hovered = resp.hovered();

        let frame_stroke_col = if hovered {
            mix_c32(p.border, accent, 0.55, 220)
        } else {
            p.border
        };

        // Subtle outer card fill & border matched to active theme palette.
        painter.rect_filled(rect, CornerRadius::same(12), p.card_bg);
        painter.rect_stroke(
            rect,
            CornerRadius::same(12),
            Stroke::new(if hovered { 1.3 } else { 1.0 }, frame_stroke_col),
            StrokeKind::Inside,
        );

        let pad_x = 6.0;
        let x0 = rect.left() + pad_x;
        let x1 = rect.right() - pad_x;
        let inner_w = x1 - x0;

        let fps_col = fps_status_color(anim_fps, accent, p.card_bg);
        let ram_col = load_status_color(anim_ram, mix_c32(BLUE, accent, 0.30, 255), p.card_bg);
        let cpu_col = load_status_color(anim_cpu, accent, p.card_bg);

        let div_col = Color32::from_rgba_unmultiplied(p.border.r(), p.border.g(), p.border.b(), 100);
        let slot_h = dock_h / 3.0;

        // ---------------------------------------------------------------------
        // Slot 1: FPS + Live Scrolling Sparkline & Pulsing Beacon
        // ---------------------------------------------------------------------
        let s1_top = rect.top() + if compact { 4.0 } else { 6.0 };
        let pulse = 0.5 + 0.5 * (t * 3.6).sin();
        let dot_center = Pos2::new(x0 + 3.0, s1_top + 6.0);
        painter.circle_filled(
            dot_center,
            2.2 + pulse * 1.4,
            Color32::from_rgba_unmultiplied(fps_col.r(), fps_col.g(), fps_col.b(), 60),
        );
        painter.circle_filled(dot_center, 2.0, fps_col);

        let fps_val_str = if show_gb {
            format!("{:.0}ms", snap.frame_ms)
        } else {
            format!("{}", anim_fps.round().clamp(0.0, 999.0) as u32)
        };
        paint_module_header(
            painter,
            x0,
            x1,
            s1_top + 6.0,
            8.5,
            "FPS",
            &fps_val_str,
            p.muted,
            fps_col,
        );

        let spark_y = s1_top + 14.0;
        let spark_h = if compact { 17.0 } else { 22.0 };
        paint_fps_sparkline(
            painter,
            Rect::from_min_size(Pos2::new(x0, spark_y), egui::vec2(inner_w, spark_h)),
            &fps_hist,
            fps_col,
            p.surface_bg,
            p.border,
            t,
        );
        if !compact {
            painter.text(
                Pos2::new(x0, spark_y + spark_h + 6.0),
                egui::Align2::LEFT_CENTER,
                format!("{:.1} ms", snap.frame_ms),
                FontId::new(8.2, egui::FontFamily::Proportional),
                p.muted,
            );
        }

        // Divider 1
        let d1_y = rect.top() + slot_h;
        painter.line_segment(
            [Pos2::new(x0, d1_y), Pos2::new(x1, d1_y)],
            Stroke::new(0.8, div_col),
        );

        // ---------------------------------------------------------------------
        // Slot 2: RAM + Animated Liquid Shimmer Gauge
        // ---------------------------------------------------------------------
        let s2_top = d1_y + if compact { 4.0 } else { 6.0 };
        paint_ram_icon(painter, Pos2::new(x0, s2_top + 6.0), ram_col);
        let ram_val_str = if show_gb && snap.ram_total_gb > 0.0 {
            format_gb_short(snap.ram_used_gb)
        } else {
            format!("{}%", anim_ram.round().clamp(0.0, 100.0) as u32)
        };
        paint_module_header(
            painter,
            x0,
            x1,
            s2_top + 6.0,
            8.5,
            "RAM",
            &ram_val_str,
            p.muted,
            ram_col,
        );

        let bar_y = s2_top + if compact { 16.0 } else { 18.0 };
        let bar_h = if compact { 12.0 } else { 14.0 };
        paint_ram_meter(
            painter,
            Rect::from_min_size(Pos2::new(x0, bar_y), egui::vec2(inner_w, bar_h)),
            anim_ram,
            ram_col,
            p.surface_bg,
            p.border,
            t,
        );
        if !compact && snap.ram_total_gb > 0.0 {
            painter.text(
                Pos2::new(x0, bar_y + bar_h + 7.5),
                egui::Align2::LEFT_CENTER,
                format!("{}/{}", format_gb_short(snap.ram_used_gb), format_gb_short(snap.ram_total_gb)),
                FontId::new(8.2, egui::FontFamily::Proportional),
                p.muted,
            );
        }

        // Divider 2
        let d2_y = rect.top() + slot_h * 2.0;
        painter.line_segment(
            [Pos2::new(x0, d2_y), Pos2::new(x1, d2_y)],
            Stroke::new(0.8, div_col),
        );

        // ---------------------------------------------------------------------
        // Slot 3: CPU + Rotating Core Spinner & Animated Wave Equalizer
        // ---------------------------------------------------------------------
        let s3_top = d2_y + if compact { 4.0 } else { 6.0 };
        paint_cpu_rotor(
            painter,
            Pos2::new(x0 + 3.2, s3_top + 6.0),
            3.3,
            rotor_phase,
            cpu_col,
        );
        let cpu_val_str = format!("{}%", anim_cpu.round().clamp(0.0, 100.0) as u32);
        paint_module_header(
            painter,
            x0,
            x1,
            s3_top + 6.0,
            8.5,
            "CPU",
            &cpu_val_str,
            p.muted,
            cpu_col,
        );

        let eq_y = s3_top + 14.0;
        let eq_h = if compact { 16.0 } else { 20.0 };
        paint_cpu_equalizer(
            painter,
            Rect::from_min_size(Pos2::new(x0, eq_y), egui::vec2(inner_w, eq_h)),
            anim_cpu,
            &cpu_hist,
            cpu_col,
            p.surface_bg,
            p.border,
            t,
        );
        if !compact {
            painter.text(
                Pos2::new(x0, eq_y + eq_h + 6.5),
                egui::Align2::LEFT_CENTER,
                format!("App {:.1}%", snap.proc_cpu_pct),
                FontId::new(8.2, egui::FontFamily::Proportional),
                p.muted,
            );
        }
    }

    resp.on_hover_ui(|ui| {
        ui.set_min_width(190.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.label(
                RichText::new("System Performance")
                    .size(12.5)
                    .strong()
                    .color(p.text),
            );
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(RichText::new("FPS").size(11.5).color(p.muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{:.0} FPS ({:.1} ms)", snap.fps, snap.frame_ms))
                            .size(11.5)
                            .strong()
                            .color(fps_status_color(snap.fps, accent, p.card_bg)),
                    );
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("RAM").size(11.5).color(p.muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{:.1} / {:.1} GB ({:.0}%)",
                            snap.ram_used_gb, snap.ram_total_gb, snap.ram_pct
                        ))
                        .size(11.5)
                        .strong()
                        .color(p.text),
                    );
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Processor").size(11.5).color(p.muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{:.1}% ({} threads)", snap.cpu_pct, snap.cpu_cores))
                            .size(11.5)
                            .strong()
                            .color(p.text),
                    );
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("RustBlox").size(11.0).color(p.muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{:.0} MB RAM  •  {:.1}% CPU",
                            snap.proc_ram_mb, snap.proc_cpu_pct
                        ))
                        .size(11.0)
                        .color(p.muted),
                    );
                });
            });
            ui.add_space(2.0);
            ui.label(
                RichText::new("Click dock to toggle % / GB / ms")
                    .size(10.0)
                    .color(p.muted),
            );
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_percentage_accounts_for_idle_inside_kernel_time() {
        let t0 = CpuRawTimes {
            idle: 800,
            kernel: 900,
            user: 100,
            proc_kernel: 20,
            proc_user: 10,
        };
        let t1 = CpuRawTimes {
            idle: 1500,
            kernel: 1700,
            user: 300,
            proc_kernel: 45,
            proc_user: 35,
        };
        // d_kernel = 800, d_user = 200 -> total = 1000; d_idle = 700 -> busy = 300 -> 30%
        // d_proc = 25 + 25 = 50 -> 5%
        let (sys_pct, proc_pct) = compute_cpu_percentages(t0, t1);
        assert!((sys_pct - 30.0).abs() < 0.01);
        assert!((proc_pct - 5.0).abs() < 0.01);
    }

    #[test]
    fn telemetry_state_initializes_valid_ranges() {
        let mut s = TelemetryState::default();
        let now = Instant::now() + Duration::from_millis(16);
        let _ = s.tick(now);
        assert!(s.snapshot.fps.is_finite() && s.snapshot.fps > 0.0);
        assert!((0.0..=100.0).contains(&s.snapshot.ram_pct));
        assert!((0.0..=100.0).contains(&s.snapshot.cpu_pct));
        assert!(s.snapshot.cpu_cores >= 1);
    }
}
