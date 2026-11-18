#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod app;
mod auth;
mod avatar_3d;
mod discord_rpc;
mod fastflags;
mod graphics;
mod roblox_process;
mod telemetry;
mod theme;
mod tray;
mod types;
mod ui;
mod updater;
mod video_preview;

use std::env;

use eframe::egui;

use app::RustBlox;
use auth::log;
use roblox_process::launch_from_uri;

fn main() -> eframe::Result<()> {
    graphics::install_panic_and_log_hook();
    let args: Vec<String> = env::args().collect();
    if updater::try_run_quiet_apply_cli(&args) {
        return Ok(());
    }
    if let Some(u_args) = updater::try_parse_updater_cli(&args) {
        let res = updater::run_updater_window(u_args);
        if let Err(ref e) = res {
            log(&format!("Updater window failed: {e}"));
        }
        return res;
    }
    updater::cleanup_old_binary();
    // Started by a roblox-player: link from the website? Launch the game and exit.
    if let Some(uri) = args.get(1) {
        if uri.starts_with("roblox-player:") {
            if let Err(e) = launch_from_uri(uri, false) {
                log(&format!("Launch failed: {e}"));
            }
            return Ok(());
        }
    }

    let boot_settings = auth::load_settings();
    let start_in_tray = tray::should_start_in_tray(&args, boot_settings.tray_enabled);
    if !start_in_tray && boot_settings.tray_enabled && tray::try_focus_existing_instance() {
        return Ok(());
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 780.0])
        .with_min_inner_size([760.0, 520.0]);
    if start_in_tray {
        viewport = viewport.with_visible(false);
    }
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/logo.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let res = graphics::run_native_with_fallback("rustblox", options, &args, move |cc| {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Ok(Box::new(RustBlox::new(cc, start_in_tray)))
    });
    if let Err(ref e) = res {
        log(&format!("Startup aborted: {e}"));
    }
    res
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::roblox_process::{
        extract_active_place_id_from_log, extract_player_exe_from_command,
        inspect_player_log_text, parse_crash_handler_attachment, LogSessionState,
    };

    #[test]
    fn parses_crash_handler_pid_and_log() {
        let cmd = r#"RobloxCrashHandler.exe --attachment=attachment_0.747_Player_E6E63_last.log=C:\Users\Test User\AppData\Local\Roblox\logs\0.747.0_Player_E6E63_last.log --database=C:\logs --annotation=PID=4208 --annotation=PlatformId=2"#;
        let parsed = parse_crash_handler_attachment(cmd);
        assert_eq!(
            parsed,
            Some((
                4208,
                PathBuf::from(
                    r"C:\Users\Test User\AppData\Local\Roblox\logs\0.747.0_Player_E6E63_last.log"
                )
            ))
        );
    }

    #[test]
    fn detects_roblox_log_lifecycle_states() {
        let tray_log = "AppState/TrayMode\nInfo [FLog::Systray] Systray initialized in sync mode\nInfo [FLog::WndProcessCheck] waitForNewPlayerProcess";
        assert_eq!(
            inspect_player_log_text(tray_log, tray_log.len() as u64),
            (LogSessionState::TrayOnly, true)
        );

        let playing_teleport = "[FLog::SingleSurfaceApp] launchUGCGame: (stage:Native).\n[FLog::SessionTransitionFSM] Entered play session.\n[FLog::SingleSurfaceApp] setStage: (stage:UGCGame)\n[FLog::SessionTransitionFSM] Teleported.\n[FLog::SessionTransitionFSM] Entered play session.";
        assert_eq!(
            inspect_player_log_text(playing_teleport, playing_teleport.len() as u64),
            (LogSessionState::Playing, false)
        );

        let finished_play = format!(
            "{playing_teleport}\n[FLog::SingleSurfaceApp] returnToLuaApp: (stage:UGCGame).\n[FLog::SingleSurfaceApp] setStage: (stage:LuaApp)\n[FLog::SessionTransitionFSM] Entered app session."
        );
        assert_eq!(
            inspect_player_log_text(&finished_play, finished_play.len() as u64),
            (LogSessionState::AppAfterPlay, false)
        );

        let resumed_play = format!("{finished_play}\n[FLog::SessionTransitionFSM] Entered play session.");
        assert_eq!(
            inspect_player_log_text(&resumed_play, resumed_play.len() as u64),
            (LogSessionState::Playing, false)
        );

        let app_only = "[FLog::SingleSurfaceApp] setStage: (stage:LuaApp)\n[FLog::SessionTransitionFSM] Entered app session.";
        assert_eq!(
            inspect_player_log_text(app_only, app_only.len() as u64),
            (LogSessionState::AppOnly, false)
        );
    }

    #[test]
    fn extracts_latest_joined_place_id_from_player_log() {
        let log = "2026-06-16T12:28:19Z,28.4,a9,6 [FLog::Output] ! Joining game 'cf9da149-3ee6' place 1818 at 128.116.1.1\n\
                   2026-06-16T12:35:00Z,42.1,b1,6 [FLog::Output] ! Joining game '11111111-2222' place 2753915549 at 128.116.2.2";
        assert_eq!(extract_active_place_id_from_log(log), Some(2753915549));
        assert_eq!(extract_active_place_id_from_log("random text"), None);
    }

    #[test]
    fn extracts_roblox_player_exe_from_registry_command() {
        let dir = std::env::temp_dir().join(format!(
            "rustblox_player_cmd_test_{}",
            std::process::id()
        ));
        let spaced = dir.join("Roblox Player");
        let _ = std::fs::create_dir_all(&spaced);
        let exe = spaced.join("RobloxPlayerBeta.exe");
        let other = spaced.join("rustblox.exe");
        std::fs::write(&exe, b"mz").expect("create test player exe");
        std::fs::write(&other, b"mz").expect("create test non-player exe");

        let quoted_cmd = format!("\"{}\" \"%1\"", exe.display());
        assert_eq!(
            extract_player_exe_from_command(&quoted_cmd),
            Some(exe.clone())
        );

        let non_player_cmd = format!("\"{}\" \"%1\"", other.display());
        assert_eq!(extract_player_exe_from_command(&non_player_cmd), None);

        let missing_cmd = format!("\"{}\" \"%1\"", spaced.join("missing\\RobloxPlayerBeta.exe").display());
        assert_eq!(extract_player_exe_from_command(&missing_cmd), None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
