use std::sync::atomic::Ordering;
use eframe::egui::{self, Color32, RichText};
use crate::app::RustBlox;
use crate::auth::{data_dir, save_settings, write_ids};
use crate::roblox_process::{
    apply_fps_setting, close_tray_roblox_processes, find_player, handler_status, register, restore,
};
use crate::theme::{apply_theme, card_frame, pal, GREEN, ORANGE, RED};
use crate::types::Action;
use crate::ui::widgets::*;

impl RustBlox {
    pub fn page_settings(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let p = pal(ui);
        ui.add_space(12.0);
        ui.label(RichText::new("Settings").size(26.0).strong());
        ui.add_space(12.0);

        ui.vertical(|ui| {
            ui.set_max_width(740.0);

            // Account card
            card_frame(ui, 18).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Account").size(17.0).strong());
                ui.add_space(6.0);
                let mut do_sign_out = false;
                let mut do_refresh_auth = false;
                if self.cookie.is_some() {
                    if let Some(prof) = &self.me {
                        ui.label(format!("Signed in via Quick Login as @{}", prof.user.name));
                    } else {
                        ui.label("Signed in via Quick Login");
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Refresh profile & friends").clicked() {
                            do_refresh_auth = true;
                        }
                        let signout_btn = egui::Button::new(
                            RichText::new("Sign out").strong().color(Color32::WHITE),
                        )
                        .fill(RED);
                        if ui.add(signout_btn).clicked() {
                            do_sign_out = true;
                        }
                    });
                } else {
                    ui.label(
                        RichText::new("Not signed in.").color(p.muted),
                    );
                    ui.add_space(8.0);
                    let btn = egui::Button::new(
                        RichText::new("Sign in with Quick Login")
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(GREEN)
                    .corner_radius(8);
                    if ui.add(btn).clicked() {
                        acts.push(Action::StartQuickLogin);
                    }
                }
                if do_refresh_auth {
                    self.sync_auth_session(true);
                    self.status = "Refreshing profile & friends...".to_string();
                }
                if do_sign_out {
                    self.sign_out();
                }
            });

            ui.add_space(12.0);

            // Appearance card
            card_frame(ui, 18).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Appearance").size(17.0).strong());
                ui.add_space(8.0);
                let prev_dark = self.settings.dark;
                ui.horizontal(|ui| {
                    ui.label("Theme:");
                    for (dark_opt, label) in [(false, "Light"), (true, "Dark")] {
                        let selected = self.settings.dark == dark_opt;
                        let btn = egui::Button::new(
                            RichText::new(label)
                                .strong()
                                .color(if selected { Color32::WHITE } else { p.text }),
                        )
                        .fill(if selected { GREEN } else { p.surface_bg })
                        .corner_radius(8);
                        if ui.add(btn).clicked() {
                            self.settings.dark = dark_opt;
                        }
                    }
                });
                if self.settings.dark != prev_dark {
                    save_settings(&self.settings);
                    apply_theme(ui.ctx(), self.settings.dark);
                }
            });

            ui.add_space(12.0);

            // Roblox Client & Performance card
            card_frame(ui, 18).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Roblox Client & Performance").size(17.0).strong());
                ui.add_space(8.0);
                let prev_fps = self.settings.target_fps;
                let fps_label = |fps: Option<u32>| match fps {
                    None => "Default".to_string(),
                    Some(999) => "999 FPS (Unlimited)".to_string(),
                    Some(n) => format!("{n} FPS"),
                };
                ui.horizontal(|ui| {
                    ui.label("Target FPS limit:");
                    egui::ComboBox::from_id_salt("target_fps")
                        .selected_text(fps_label(self.settings.target_fps))
                        .show_ui(ui, |ui| {
                            for opt in [None, Some(60), Some(120), Some(144), Some(240), Some(999)] {
                                ui.selectable_value(&mut self.settings.target_fps, opt, fps_label(opt));
                            }
                        });
                });
                if self.settings.target_fps != prev_fps {
                    save_settings(&self.settings);
                    self.player_path = find_player();
                    self.status = match apply_fps_setting(self.settings.target_fps) {
                        Ok(()) => format!("Target FPS set to {}", fps_label(self.settings.target_fps)),
                        Err(e) => format!("Saved preference, but {e}"),
                    };
                }
                ui.add_space(4.0);
                small_text(
                    ui,
                    "Writes DFIntTaskSchedulerTargetFps to ClientSettings\\ClientAppSettings.json (applied on launch)."
                        .to_string(),
                );

                ui.add_space(10.0);
                let prev_auto_close = self.settings.auto_close_roblox;
                ui.checkbox(
                    &mut self.settings.auto_close_roblox,
                    "Auto-close Roblox app after leaving a game (also closes Roblox tray)",
                );
                if self.settings.auto_close_roblox != prev_auto_close {
                    save_settings(&self.settings);
                    self.auto_close_flag
                        .store(self.settings.auto_close_roblox, Ordering::Relaxed);
                    if self.settings.auto_close_roblox {
                        close_tray_roblox_processes();
                        self.status = "Auto-close Roblox app enabled.".to_string();
                    } else {
                        self.status = "Auto-close Roblox app disabled.".to_string();
                    }
                }
                small_text(
                    ui,
                    "Closes the Roblox desktop/tray app after leaving a game. Active only while rustblox is running â€” closing rustblox stops it completely so the official Roblox app runs normally."
                        .to_string(),
                );

                ui.add_space(10.0);
                if let Some(path) = &self.player_path.clone() {
                    let ver = path
                        .parent()
                        .and_then(|d| d.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown");
                    ui.label(format!("Detected Roblox version: {ver}"));
                    small_text(ui, path.display().to_string());
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Open Roblox folder").clicked() {
                            if let Some(dir) = path.parent() {
                                let _ = open::that(dir);
                            }
                        }
                        if ui.button("Re-scan").clicked() {
                            self.player_path = find_player();
                            self.status = "Checked Roblox installation".to_string();
                        }
                    });
                } else {
                    ui.colored_label(
                        ORANGE,
                        "RobloxPlayerBeta.exe was not found. Install Roblox once from roblox.com first.",
                    );
                    ui.add_space(4.0);
                    if ui.button("Re-scan").clicked() {
                        self.player_path = find_player();
                    }
                }
            });

            ui.add_space(12.0);

            // Roblox Links card
            card_frame(ui, 18).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Roblox Links").size(17.0).strong());
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Make rustblox open when you click Play on roblox.com.")
                        .color(p.muted),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Active handler:").color(p.muted));
                    ui.label(RichText::new(&self.handler).strong());
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Use rustblox for Roblox links").clicked() {
                        self.status = register().unwrap_or_else(|e| format!("Error: {e}"));
                        self.handler = handler_status();
                    }
                    if ui.button("Restore original").clicked() {
                        self.status = restore().unwrap_or_else(|e| format!("Error: {e}"));
                        self.handler = handler_status();
                    }
                });
            });

            ui.add_space(12.0);

            // Data card
            card_frame(ui, 18).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Data & Cache").size(17.0).strong());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Refresh games").clicked() {
                        self.force_refresh_games();
                    }
                    if ui.button("Clear Continue list").clicked() {
                        self.just_launched.clear();
                        self.recent_ids.clear();
                        self.recent.clear();
                        write_ids("recent.txt", &self.recent_ids);
                    }
                    if ui.button("Open data folder").clicked() {
                        let _ = open::that(data_dir());
                    }
                });
                ui.add_space(6.0);
                small_text(ui, format!("Saved in {}", data_dir().display()));
            });
        });
        ui.add_space(20.0);
    }
}
