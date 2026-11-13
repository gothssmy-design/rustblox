#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod app;
mod auth;
mod avatar_3d;
mod fastflags;
mod roblox_process;
mod telemetry;
mod theme;
mod types;
mod ui;
mod video_preview;

use std::env;

use eframe::egui;

use app::RustBlox;
use auth::log;
use roblox_process::launch_from_uri;

fn main() -> eframe::Result<()> {
    // Started by a roblox-player: link from the website? Launch the game and exit.
    let args: Vec<String> = env::args().collect();
    if let Some(uri) = args.get(1) {
        if uri.starts_with("roblox-player:") {
            if let Err(e) = launch_from_uri(uri, false) {
                log(&format!("Launch failed: {e}"));
            }
            return Ok(());
        }
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 780.0])
        .with_min_inner_size([760.0, 520.0]);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/logo.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "rustblox",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(RustBlox::new(cc)))
        }),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::roblox_process::{
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
}
