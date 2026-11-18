use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Once};

use eframe::egui;
use eframe::egui_wgpu::WgpuSetup;
use eframe::wgpu::{self, Backend, Backends, DeviceType, Dx12Compiler};
use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};

use crate::auth::{data_dir, log};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsTier {
    WgpuDx12Hardware,
    #[cfg(feature = "glow")]
    GlowOpenGL,
    WgpuOpenGL,
    WgpuDx12SafeWarp,
    WgpuAllBackends,
}

impl GraphicsTier {
    pub fn label(self) -> &'static str {
        match self {
            Self::WgpuDx12Hardware => "DirectX 12 (Hardware)",
            #[cfg(feature = "glow")]
            Self::GlowOpenGL => "OpenGL (Glow)",
            Self::WgpuOpenGL => "OpenGL (WGPU)",
            Self::WgpuDx12SafeWarp => "DirectX 12 Safe / Software (WARP)",
            Self::WgpuAllBackends => "WGPU Auto (Vulkan / DX12 / GL)",
        }
    }
}

pub fn install_panic_and_log_hook() {
    static HOOK_ONCE: Once = Once::new();
    HOOK_ONCE.call_once(|| {
        let prev = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let loc = info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "unknown".to_string());
            let payload = panic_payload_str(info.payload());
            log(&format!("[panic] {loc}: {payload}"));
            prev(info);
        }));
    });
}

fn panic_payload_str(payload: &(dyn Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

pub fn parse_renderer_override(args: &[String], env_val: Option<&str>) -> Option<GraphicsTier> {
    let raw = args
        .iter()
        .find_map(|a| a.strip_prefix("--renderer="))
        .or_else(|| {
            args.windows(2)
                .find(|w| w[0] == "--renderer")
                .map(|w| w[1].as_str())
        })
        .or(env_val)?;

    match raw.trim().to_ascii_lowercase().as_str() {
        "dx12" | "wgpu-dx12" => Some(GraphicsTier::WgpuDx12Hardware),
        #[cfg(feature = "glow")]
        "glow" => Some(GraphicsTier::GlowOpenGL),
        "gl" | "opengl" | "glow" => Some(GraphicsTier::WgpuOpenGL),
        "warp" | "software" | "dx12-warp" | "safe" => Some(GraphicsTier::WgpuDx12SafeWarp),
        "vulkan" | "wgpu" | "auto" => Some(GraphicsTier::WgpuAllBackends),
        _ => None,
    }
}

pub fn launch_tiers(override_tier: Option<GraphicsTier>) -> Vec<GraphicsTier> {
    let default_order = [
        GraphicsTier::WgpuDx12Hardware,
        #[cfg(feature = "glow")]
        GraphicsTier::GlowOpenGL,
        GraphicsTier::WgpuOpenGL,
        GraphicsTier::WgpuDx12SafeWarp,
        GraphicsTier::WgpuAllBackends,
    ];
    let Some(first) = override_tier else {
        return default_order.to_vec();
    };
    let mut tiers = vec![first];
    for t in default_order {
        if t != first {
            tiers.push(t);
        }
    }
    tiers
}

pub fn score_wgpu_adapter(
    backend: Backend,
    device_type: DeviceType,
    allow_software: bool,
) -> Option<u8> {
    let dev_score = match device_type {
        DeviceType::DiscreteGpu => 50,
        DeviceType::IntegratedGpu => 40,
        DeviceType::VirtualGpu => 30,
        DeviceType::Other => 20,
        DeviceType::Cpu => {
            if allow_software {
                10
            } else {
                return None;
            }
        }
    };
    let api_score = match backend {
        Backend::Dx12 => 4,
        Backend::Vulkan => 3,
        Backend::Gl => 2,
        _ => 1,
    };
    Some(dev_score + api_score)
}

fn pick_best_wgpu_adapter(
    adapters: &[wgpu::Adapter],
    compatible_surface: Option<&wgpu::Surface<'_>>,
    allow_software: bool,
) -> Result<wgpu::Adapter, String> {
    let mut best: Option<(u8, &wgpu::Adapter)> = None;
    let mut descriptions = Vec::with_capacity(adapters.len());

    for adapter in adapters {
        let info = adapter.get_info();
        let surf_ok = compatible_surface.is_none_or(|s| adapter.is_surface_supported(s));
        descriptions.push(format!(
            "{} ({:?}/{:?}, surface_ok={})",
            info.name, info.backend, info.device_type, surf_ok
        ));
        if !surf_ok {
            continue;
        }
        if let Some(score) = score_wgpu_adapter(info.backend, info.device_type, allow_software) {
            if best.is_none_or(|(best_score, _)| score > best_score) {
                best = Some((score, adapter));
            }
        }
    }

    if let Some((_, adapter)) = best {
        Ok(adapter.clone())
    } else if descriptions.is_empty() {
        Err("No graphics adapters reported for backend".to_string())
    } else {
        Err(format!(
            "No suitable adapter (allow_software={allow_software}): [{}]",
            descriptions.join("; ")
        ))
    }
}

fn configure_options_for_tier(base: &eframe::NativeOptions, tier: GraphicsTier) -> eframe::NativeOptions {
    let mut opts = base.clone();
    opts.run_and_return = true;
    match tier {
        GraphicsTier::WgpuDx12Hardware => {
            opts.renderer = eframe::Renderer::Wgpu;
            if let WgpuSetup::CreateNew(ref mut create_new) = opts.wgpu_options.wgpu_setup {
                create_new.instance_descriptor.backends = Backends::DX12;
                create_new.native_adapter_selector = Some(Arc::new(|adapters, surface| {
                    pick_best_wgpu_adapter(adapters, surface, false)
                }));
            }
        }
        #[cfg(feature = "glow")]
        GraphicsTier::GlowOpenGL => {
            opts.renderer = eframe::Renderer::Glow;
        }
        GraphicsTier::WgpuOpenGL => {
            opts.renderer = eframe::Renderer::Wgpu;
            if let WgpuSetup::CreateNew(ref mut create_new) = opts.wgpu_options.wgpu_setup {
                create_new.instance_descriptor.backends = Backends::GL;
                create_new.native_adapter_selector = Some(Arc::new(|adapters, surface| {
                    pick_best_wgpu_adapter(adapters, surface, true)
                }));
            }
        }
        GraphicsTier::WgpuDx12SafeWarp => {
            opts.renderer = eframe::Renderer::Wgpu;
            if let WgpuSetup::CreateNew(ref mut create_new) = opts.wgpu_options.wgpu_setup {
                create_new.instance_descriptor.backends = Backends::DX12;
                create_new.instance_descriptor.backend_options.dx12.shader_compiler =
                    Dx12Compiler::Fxc;
                create_new.power_preference = wgpu::PowerPreference::LowPower;
                create_new.native_adapter_selector = Some(Arc::new(|adapters, surface| {
                    pick_best_wgpu_adapter(adapters, surface, true)
                }));
            }
        }
        GraphicsTier::WgpuAllBackends => {
            opts.renderer = eframe::Renderer::Wgpu;
            if let WgpuSetup::CreateNew(ref mut create_new) = opts.wgpu_options.wgpu_setup {
                create_new.instance_descriptor.backends = Backends::PRIMARY | Backends::GL;
                create_new.native_adapter_selector = Some(Arc::new(|adapters, surface| {
                    pick_best_wgpu_adapter(adapters, surface, true)
                }));
            }
        }
    }
    opts
}

struct ResilientApp {
    inner: Box<dyn eframe::App>,
    entered_cycle: Arc<AtomicBool>,
}

impl eframe::App for ResilientApp {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.entered_cycle.store(true, Ordering::Release);
        self.inner.logic(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.entered_cycle.store(true, Ordering::Release);
        self.inner.ui(ui, frame);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.inner.save(storage);
    }

    #[cfg(feature = "glow")]
    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.inner.on_exit(gl);
    }

    #[cfg(not(feature = "glow"))]
    fn on_exit(&mut self) {
        self.inner.on_exit();
    }

    fn auto_save_interval(&self) -> std::time::Duration {
        self.inner.auto_save_interval()
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        self.inner.clear_color(visuals)
    }

    fn persist_egui_memory(&self) -> bool {
        self.inner.persist_egui_memory()
    }
}

pub fn run_native_with_fallback<F>(
    app_name: &str,
    base_options: eframe::NativeOptions,
    args: &[String],
    mut make_app: F,
) -> eframe::Result<()>
where
    F: FnMut(&eframe::CreationContext<'_>) -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>,
{
    let env_override = std::env::var("RUSTBLOX_RENDERER").ok();
    let tiers = launch_tiers(parse_renderer_override(args, env_override.as_deref()));
    let mut errors: Vec<String> = Vec::new();
    let mut last_err: Option<eframe::Error> = None;

    for tier in tiers {
        let tier_opts = configure_options_for_tier(&base_options, tier);
        let entered_cycle = Arc::new(AtomicBool::new(false));
        let entered_flag = Arc::clone(&entered_cycle);

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            eframe::run_native(
                app_name,
                tier_opts,
                Box::new(|cc| {
                    let app = make_app(cc)?;
                    Ok(Box::new(ResilientApp {
                        inner: app,
                        entered_cycle: entered_flag,
                    }))
                }),
            )
        }));

        match result {
            Ok(Ok(())) => {
                if entered_cycle.load(Ordering::Acquire) {
                    return Ok(());
                }
                let msg = format!("{}: exited before rendering first frame", tier.label());
                log(&format!("[startup] {msg}"));
                errors.push(msg);
            }
            Ok(Err(err)) => {
                let msg = format!("{}: {err}", tier.label());
                log(&format!("[startup] {msg}"));
                errors.push(msg);
                last_err = Some(err);
            }
            Err(panic_val) => {
                let msg = format!("{}: panicked ({})", tier.label(), panic_payload_str(&*panic_val));
                log(&format!("[startup] {msg}"));
                errors.push(msg);
            }
        }
    }

    let log_path = data_dir().join("log.txt");
    let details = errors.join("\n- ");
    show_startup_error_dialog(app_name, &details, &log_path.display().to_string());
    Err(last_err.unwrap_or_else(|| {
        eframe::Error::AppCreation(Box::from(format!(
            "All graphics backends failed:\n- {details}"
        )))
    }))
}

pub fn show_startup_error_dialog(app_name: &str, details: &str, log_path: &str) {
    let title = format!("{app_name} — Unable to Start");
    let body = format!(
        "RustBlox could not initialize any graphics backend on this PC.\n\n\
         Tried renderers:\n- {details}\n\n\
         Tip: Make sure your GPU drivers are installed/enabled, or try updating DirectX / OpenGL drivers.\n\n\
         Diagnostic log saved to:\n{log_path}"
    );
    let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let body_w: Vec<u16> = body.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            body_w.as_ptr(),
            title_w.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_renderer_overrides_and_tier_order() {
        let args = vec!["rustblox.exe".to_string(), "--renderer=opengl".to_string()];
        let ov = parse_renderer_override(&args, None);
        #[cfg(feature = "glow")]
        let expected_gl = GraphicsTier::GlowOpenGL;
        #[cfg(not(feature = "glow"))]
        let expected_gl = GraphicsTier::WgpuOpenGL;
        assert_eq!(ov, Some(expected_gl));

        let tiers = launch_tiers(ov);
        assert_eq!(tiers[0], expected_gl);
        assert!(tiers.contains(&GraphicsTier::WgpuDx12Hardware));
        assert!(tiers.contains(&GraphicsTier::WgpuDx12SafeWarp));

        let args_space = vec![
            "rustblox.exe".to_string(),
            "--renderer".to_string(),
            "warp".to_string(),
        ];
        assert_eq!(
            parse_renderer_override(&args_space, None),
            Some(GraphicsTier::WgpuDx12SafeWarp)
        );
        assert_eq!(
            parse_renderer_override(&[], Some("opengl")),
            Some(GraphicsTier::WgpuOpenGL)
        );
    }

    #[test]
    fn scores_hardware_and_software_adapters_properly() {
        assert_eq!(
            score_wgpu_adapter(Backend::Dx12, DeviceType::Cpu, false),
            None
        );
        assert!(
            score_wgpu_adapter(Backend::Dx12, DeviceType::DiscreteGpu, false)
                > score_wgpu_adapter(Backend::Dx12, DeviceType::IntegratedGpu, false)
        );
        assert!(
            score_wgpu_adapter(Backend::Dx12, DeviceType::IntegratedGpu, true)
                > score_wgpu_adapter(Backend::Dx12, DeviceType::Cpu, true)
        );
        assert!(score_wgpu_adapter(Backend::Dx12, DeviceType::Cpu, true).is_some());
    }
}
