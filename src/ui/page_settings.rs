use std::sync::atomic::Ordering;
use eframe::egui::{self, Color32, CornerRadius, RichText, Sense};
use crate::app::RustBlox;
use crate::auth::{data_dir, save_settings, write_ids};
use crate::fastflags::{
    export_effective_flags_json, format_flag_value, import_custom_flags_json, parse_flag_value,
    save_fastflags, sync_client_fastflags, LightingMode, RendererMode,
};
use crate::roblox_process::{
    apply_fps_setting, close_tray_roblox_processes, find_player, handler_status, register, restore,
};
use crate::theme::{
    accent_color, apply_theme, card_frame, effective_dark_mode, is_dark_color, pal, Palette,
    BLUE, GREEN, ORANGE, RED,
};
use crate::types::{Action, BgMode, TextColorMode};
use crate::ui::page_login::draw_login_marquee;
use crate::ui::widgets::*;

#[derive(Clone, Copy)]
struct ThemePack {
    name: &'static str,
    dark: bool,
    bg_mode: BgMode,
    bg_color: [u8; 3],
    text_mode: TextColorMode,
    borders: Option<[u8; 3]>,
    profile: Option<[u8; 3]>,
    friends: Option<[u8; 3]>,
    games: Option<[u8; 3]>,
    text_border: Option<[u8; 3]>,
    preview_accent: [u8; 3],
}

const THEME_PACKS: [ThemePack; 6] = [
    ThemePack {
        name: "Light",
        dark: false,
        bg_mode: BgMode::ThemeDefault,
        bg_color: [248, 249, 250],
        text_mode: TextColorMode::Auto,
        borders: None,
        profile: None,
        friends: None,
        games: None,
        text_border: None,
        preview_accent: [0, 148, 255],
    },
    ThemePack {
        name: "Dark",
        dark: true,
        bg_mode: BgMode::ThemeDefault,
        bg_color: [17, 19, 24],
        text_mode: TextColorMode::Auto,
        borders: None,
        profile: None,
        friends: None,
        games: None,
        text_border: None,
        preview_accent: [0, 176, 111],
    },
    ThemePack {
        name: "Mint",
        dark: true,
        bg_mode: BgMode::SolidColor,
        bg_color: [14, 22, 30],
        text_mode: TextColorMode::Auto,
        borders: Some([34, 197, 148]),
        profile: Some([0, 214, 143]),
        friends: Some([56, 224, 168]),
        games: Some([28, 170, 132]),
        text_border: Some([48, 210, 160]),
        preview_accent: [0, 214, 143],
    },
    ThemePack {
        name: "Rose",
        dark: true,
        bg_mode: BgMode::SolidColor,
        bg_color: [56, 18, 34],
        text_mode: TextColorMode::Auto,
        borders: Some([232, 92, 142]),
        profile: Some([255, 118, 168]),
        friends: Some([248, 138, 180]),
        games: Some([212, 76, 128]),
        text_border: Some([244, 122, 168]),
        preview_accent: [255, 118, 168],
    },
    ThemePack {
        name: "Ocean",
        dark: true,
        bg_mode: BgMode::SolidColor,
        bg_color: [14, 28, 48],
        text_mode: TextColorMode::Auto,
        borders: Some([62, 162, 240]),
        profile: Some([76, 194, 255]),
        friends: Some([96, 210, 255]),
        games: Some([52, 146, 224]),
        text_border: Some([86, 188, 255]),
        preview_accent: [76, 194, 255],
    },
    ThemePack {
        name: "Sand",
        dark: false,
        bg_mode: BgMode::SolidColor,
        bg_color: [244, 237, 226],
        text_mode: TextColorMode::Auto,
        borders: Some([178, 156, 130]),
        profile: Some([152, 124, 94]),
        friends: Some([134, 156, 118]),
        games: Some([168, 144, 118]),
        text_border: Some([154, 132, 106]),
        preview_accent: [152, 124, 94],
    },
];

impl RustBlox {
    pub fn page_settings(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let p = pal(ui);
        ui.add_space(12.0);
        ui.label(RichText::new("Settings").size(26.0).strong());
        ui.add_space(12.0);

        ui.vertical(|ui| {
            ui.set_max_width(740.0);

            // Accounts card
            card_frame(ui, 18).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Accounts").size(17.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("+ Add account").clicked() {
                            acts.push(Action::AddAccount);
                        }
                        if self.cookie.is_some() {
                            if ui.button("Sign out").clicked() {
                                self.sign_out();
                            }
                            if ui.button("Refresh").clicked() {
                                self.sync_auth_session(true);
                                self.status = "Refreshing profile & friends...".to_string();
                            }
                        }
                    });
                });
                ui.add_space(6.0);

                if !self.saved_accounts.is_empty() {
                    let active_id = self.me.as_ref().map(|m| m.user.id);
                    let saved = self.saved_accounts.clone();
                    for acc in &saved {
                        let is_active = active_id == Some(acc.id)
                            || self.cookie.as_deref() == Some(acc.cookie.as_str());
                        egui::Frame::new()
                            .fill(if is_active {
                                p.selected_bg.gamma_multiply(0.35)
                            } else {
                                p.surface_bg
                            })
                            .stroke(egui::Stroke::new(1.0, p.border))
                            .corner_radius(8)
                            .inner_margin(egui::Margin::symmetric(10, 7))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let _ = thumb_styled(
                                        ui,
                                        Some(&acc.headshot),
                                        egui::vec2(28.0, 28.0),
                                        14,
                                        true,
                                    );
                                    ui.add_space(4.0);
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(&acc.display).size(13.0).strong());
                                            if acc.verified {
                                                verified_badge_tip(ui, 12.5, "Verified Account");
                                            }
                                            if is_active {
                                                ui.label(RichText::new("• Active").size(11.5).color(GREEN));
                                            }
                                        });
                                        small_text(ui, format!("@{}", acc.name));
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button(RichText::new("Remove").size(11.5).color(RED)).clicked() {
                                                acts.push(Action::RemoveAccount(acc.id));
                                            }
                                            if !is_active && ui.button("Switch").clicked() {
                                                acts.push(Action::SwitchAccount(acc.id));
                                            }
                                        },
                                    );
                                });
                            });
                        ui.add_space(5.0);
                    }
                } else if self.cookie.is_none() {
                    ui.label(RichText::new("Not signed in.").color(p.muted));
                    ui.add_space(6.0);
                    if ui.button("Sign in with Quick Login").clicked() {
                        acts.push(Action::StartQuickLogin);
                    }
                }
            });

            ui.add_space(12.0);

            // Theme & Background card
            self.settings_theme_card(ui);

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
                    "Closes the Roblox desktop/tray app after leaving a game. Active only while rustblox is running - closing rustblox stops it completely so the official Roblox app runs normally."
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

            // Built-in FastFlags card
            self.settings_fastflags_card(ui);

            ui.add_space(12.0);

            // Advanced collapsible section (Roblox Links + Data & Cache)
            self.settings_advanced_card(ui);
        });
        ui.add_space(20.0);
    }

    fn settings_fastflags_card(&mut self, ui: &mut egui::Ui) {
        let p = pal(ui);

        card_frame(ui, 18).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let mut changed = false;
            let mut removed_keys: Vec<String> = Vec::new();

            ui.horizontal(|ui| {
                ui.label(RichText::new("FastFlags").size(17.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reset").clicked() {
                        for k in self.fastflags.custom_flags.keys() {
                            removed_keys.push(k.clone());
                        }
                        self.fastflags.reset_all();
                        changed = true;
                    }
                    if ui.button("Quality").clicked() {
                        self.fastflags.apply_quality_preset();
                        changed = true;
                    }
                    if ui.button("Max FPS").clicked() {
                        self.fastflags.apply_competitive_preset();
                        changed = true;
                    }
                });
            });
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                ui.label("Renderer:");
                let prev_ren = self.fastflags.renderer;
                egui::ComboBox::from_id_salt("ff_renderer")
                    .selected_text(self.fastflags.renderer.label())
                    .show_ui(ui, |ui| {
                        for mode in RendererMode::ALL {
                            ui.selectable_value(&mut self.fastflags.renderer, mode, mode.label());
                        }
                    });
                if self.fastflags.renderer != prev_ren {
                    changed = true;
                }

                ui.add_space(12.0);
                ui.label("Lighting:");
                let prev_light = self.fastflags.lighting;
                egui::ComboBox::from_id_salt("ff_lighting")
                    .selected_text(self.fastflags.lighting.label())
                    .show_ui(ui, |ui| {
                        for mode in LightingMode::ALL {
                            ui.selectable_value(&mut self.fastflags.lighting, mode, mode.label());
                        }
                    });
                if self.fastflags.lighting != prev_light {
                    changed = true;
                }
            });

            ui.add_space(8.0);
            let prev = (
                self.fastflags.disable_shadows,
                self.fastflags.low_quality_textures,
                self.fastflags.disable_post_fx,
                self.fastflags.disable_grass,
                self.fastflags.disable_telemetry,
            );
            ui.checkbox(&mut self.fastflags.disable_shadows, "Disable player & world shadows");
            ui.checkbox(&mut self.fastflags.low_quality_textures, "Low texture quality");
            ui.checkbox(&mut self.fastflags.disable_post_fx, "Disable post-processing (bloom/blur)");
            ui.checkbox(&mut self.fastflags.disable_grass, "Disable 3D grass");
            ui.checkbox(&mut self.fastflags.disable_telemetry, "Disable telemetry");
            if prev
                != (
                    self.fastflags.disable_shadows,
                    self.fastflags.low_quality_textures,
                    self.fastflags.disable_post_fx,
                    self.fastflags.disable_grass,
                    self.fastflags.disable_telemetry,
                )
            {
                changed = true;
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Custom Flags").size(14.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(if self.ff_json_open { "Hide JSON" } else { "JSON" })
                        .clicked()
                    {
                        self.ff_json_open = !self.ff_json_open;
                        if self.ff_json_open && self.ff_json_buf.trim().is_empty() {
                            self.ff_json_buf = export_effective_flags_json(
                                &self.fastflags,
                                self.settings.target_fps,
                            );
                        }
                    }
                });
            });
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.ff_new_key)
                        .hint_text("FFlag name")
                        .desired_width(230.0),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.ff_new_val)
                        .hint_text("Value")
                        .desired_width(120.0),
                );
                if ui.button("Add").clicked() {
                    let key = self.ff_new_key.trim().to_string();
                    if key.is_empty() {
                        self.ff_err = Some("Enter a FastFlag name.".to_string());
                    } else {
                        let val = if self.ff_new_val.trim().is_empty() {
                            serde_json::Value::Bool(true)
                        } else {
                            parse_flag_value(&self.ff_new_val)
                        };
                        self.fastflags.custom_flags.insert(key, val);
                        self.ff_new_key.clear();
                        self.ff_new_val.clear();
                        self.ff_err = None;
                        changed = true;
                    }
                }
            });

            if !self.fastflags.custom_flags.is_empty() {
                ui.add_space(6.0);
                let entries: Vec<(String, serde_json::Value)> = self
                    .fastflags
                    .custom_flags
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                for (k, v) in entries {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&k).monospace().size(12.0));
                        ui.label(RichText::new("=").color(p.muted));
                        ui.label(RichText::new(format_flag_value(&v)).monospace().size(12.0).color(p.muted));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new("Remove").size(11.0).color(RED)).clicked() {
                                self.fastflags.custom_flags.remove(&k);
                                removed_keys.push(k.clone());
                                changed = true;
                            }
                        });
                    });
                }
            }

            if self.ff_json_open {
                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.ff_json_buf)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(6)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("Import JSON").clicked() {
                        match import_custom_flags_json(&self.ff_json_buf, &mut self.fastflags) {
                            Ok(n) => {
                                self.ff_err = None;
                                self.status = format!("Imported {n} FastFlags.");
                                changed = true;
                            }
                            Err(e) => self.ff_err = Some(e),
                        }
                    }
                    if ui.button("Load Current").clicked() {
                        self.ff_json_buf = export_effective_flags_json(
                            &self.fastflags,
                            self.settings.target_fps,
                        );
                        self.ff_err = None;
                    }
                    if ui.button("Copy").clicked() {
                        ui.ctx().copy_text(export_effective_flags_json(
                            &self.fastflags,
                            self.settings.target_fps,
                        ));
                        self.status = "Copied FastFlags JSON.".to_string();
                    }
                });
            }

            if let Some(err) = &self.ff_err {
                ui.add_space(4.0);
                ui.colored_label(RED, err);
            }

            if changed {
                if let Err(e) = save_fastflags(&self.fastflags) {
                    self.ff_err = Some(format!("Failed saving fastflags.json: {e}"));
                } else {
                    self.player_path = find_player();
                    let removed_refs: Vec<&str> =
                        removed_keys.iter().map(String::as_str).collect();
                    match sync_client_fastflags(
                        &self.fastflags,
                        self.settings.target_fps,
                        &removed_refs,
                    ) {
                        Ok(()) => {
                            self.ff_err = None;
                            self.status = "FastFlags updated.".to_string();
                        }
                        Err(e) => {
                            self.ff_err = Some(e);
                        }
                    }
                }
            }
        });
    }

    fn settings_theme_card(&mut self, ui: &mut egui::Ui) {
        let p = pal(ui);
        let accent = accent_color(ui);
        let accent_fg = if is_dark_color([accent.r(), accent.g(), accent.b()]) {
            Color32::WHITE
        } else {
            Color32::from_rgb(16, 19, 26)
        };
        let mut changed = false;

        card_frame(ui, 18).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let has_custom = self.settings.bg_mode != BgMode::ThemeDefault
                || self.settings.text_mode != TextColorMode::Auto
                || self.settings.borders_color.is_some()
                || self.settings.profile_color.is_some()
                || self.settings.friends_color.is_some()
                || self.settings.games_border_color.is_some()
                || self.settings.text_border_color.is_some();

            ui.horizontal(|ui| {
                ui.label(RichText::new("Theme").size(17.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if has_custom && ui.button("Reset to Default").clicked() {
                        self.settings.bg_mode = BgMode::ThemeDefault;
                        self.settings.text_mode = TextColorMode::Auto;
                        self.settings.borders_color = None;
                        self.settings.profile_color = None;
                        self.settings.friends_color = None;
                        self.settings.games_border_color = None;
                        self.settings.text_border_color = None;
                        changed = true;
                    }
                });
            });
            ui.add_space(8.0);

            // 1. Simple 1-click theme chips
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                ui.label(RichText::new("Style:").color(p.muted));
                for pack in THEME_PACKS {
                    let is_active = self.settings.bg_mode == pack.bg_mode
                        && self.settings.borders_color == pack.borders
                        && self.settings.profile_color == pack.profile
                        && self.settings.friends_color == pack.friends
                        && self.settings.games_border_color == pack.games
                        && self.settings.text_border_color == pack.text_border
                        && if pack.bg_mode == BgMode::ThemeDefault {
                            effective_dark_mode(&self.settings) == pack.dark
                        } else {
                            self.settings.bg_color == pack.bg_color
                        };

                    let dot = Color32::from_rgb(
                        pack.preview_accent[0],
                        pack.preview_accent[1],
                        pack.preview_accent[2],
                    );
                    let resp = egui::Frame::new()
                        .fill(if is_active { p.selected_bg } else { p.surface_bg })
                        .stroke(egui::Stroke::new(
                            if is_active { 1.5 } else { 1.0 },
                            if is_active { accent } else { p.border },
                        ))
                        .corner_radius(8)
                        .inner_margin(egui::Margin::symmetric(10, 5))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (dr, _) = ui.allocate_exact_size(
                                    egui::vec2(10.0, 10.0),
                                    Sense::hover(),
                                );
                                ui.painter().circle_filled(dr.center(), 4.5, dot);
                                ui.label(RichText::new(pack.name).size(12.5).strong());
                            });
                        })
                        .response
                        .interact(Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);

                    if resp.clicked() {
                        self.settings.dark = pack.dark;
                        self.settings.bg_mode = pack.bg_mode;
                        if pack.bg_mode == BgMode::SolidColor {
                            self.settings.bg_color = pack.bg_color;
                        }
                        self.settings.text_mode = pack.text_mode;
                        self.settings.borders_color = pack.borders;
                        self.settings.profile_color = pack.profile;
                        self.settings.friends_color = pack.friends;
                        self.settings.games_border_color = pack.games;
                        self.settings.text_border_color = pack.text_border;
                        changed = true;
                    }
                }
            });

            ui.add_space(10.0);

            // 2. Simple Mode + Background switcher
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                ui.label(RichText::new("Mode:").color(p.muted));
                let active_dark = effective_dark_mode(&self.settings);
                for (dark_opt, label) in [(false, "Light"), (true, "Dark")] {
                    let selected = active_dark == dark_opt;
                    let btn = egui::Button::new(
                        RichText::new(label)
                            .size(12.5)
                            .strong()
                            .color(if selected { accent_fg } else { p.text }),
                    )
                    .fill(if selected { accent } else { p.surface_bg })
                    .corner_radius(7);
                    if ui.add(btn).clicked() && !selected {
                        self.settings.dark = dark_opt;
                        self.settings.text_mode = TextColorMode::Auto;
                        changed = true;
                    }
                }

                ui.add_space(10.0);
                ui.label(RichText::new("Background:").color(p.muted));
                for (mode_opt, label) in [
                    (BgMode::ThemeDefault, "Default"),
                    (BgMode::SolidColor, "Solid Color"),
                    (BgMode::Wallpaper, "Wallpaper"),
                ] {
                    let selected = self.settings.bg_mode == mode_opt;
                    let btn = egui::Button::new(
                        RichText::new(label)
                            .size(12.5)
                            .strong()
                            .color(if selected { accent_fg } else { p.text }),
                    )
                    .fill(if selected { accent } else { p.surface_bg })
                    .corner_radius(7);
                    if ui.add(btn).clicked() && !selected {
                        self.settings.bg_mode = mode_opt;
                        self.settings.text_mode = TextColorMode::Auto;
                        if mode_opt == BgMode::SolidColor {
                            self.settings.dark = is_dark_color(self.settings.bg_color);
                        }
                        changed = true;
                        if mode_opt == BgMode::Wallpaper
                            && self.wallpaper_tex.is_none()
                            && !self.settings.bg_wallpaper.trim().is_empty()
                        {
                            let path = self.settings.bg_wallpaper.clone();
                            self.request_wallpaper_load(path);
                        }
                    }
                }
            });

            if self.settings.bg_mode == BgMode::SolidColor {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("Background color:");
                    if ui.color_edit_button_srgb(&mut self.settings.bg_color).changed() {
                        self.settings.dark = is_dark_color(self.settings.bg_color);
                        self.settings.text_mode = TextColorMode::Auto;
                        changed = true;
                    }
                    let [r, g, b] = self.settings.bg_color;
                    ui.label(
                        RichText::new(format!("#{r:02X}{g:02X}{b:02X}"))
                            .monospace()
                            .size(12.0)
                            .color(p.muted),
                    );
                });
            } else if self.settings.bg_mode == BgMode::Wallpaper {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Choose Image...").clicked() {
                        self.open_wallpaper_picker();
                    }
                    if !self.settings.bg_wallpaper.is_empty() && ui.button("Clear").clicked() {
                        self.settings.bg_wallpaper.clear();
                        self.settings.bg_mode = BgMode::ThemeDefault;
                        self.wallpaper_tex = None;
                        self.wallpaper_err = None;
                        changed = true;
                    }
                    ui.add_space(8.0);
                    ui.label("Dim:");
                    let mut pct = (self.settings.bg_overlay * 100.0).round();
                    if ui.add(egui::Slider::new(&mut pct, 0.0..=85.0).suffix("%")).changed() {
                        self.settings.bg_overlay = (pct / 100.0).clamp(0.0, 0.85);
                        changed = true;
                    }
                });
                if let Some(err) = &self.wallpaper_err {
                    ui.colored_label(RED, err);
                }
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // 3. Clean tabbed Preview (Home | Profile | Login)
            self.settings_theme_showcase(ui, p, accent);

            ui.add_space(10.0);

            // 4. Optional Borders drawer (closed by default so it stays simple!)
            if self.settings_border_colors_section(ui, p) {
                changed = true;
            }
        });

        if changed {
            save_settings(&self.settings);
            apply_theme(ui.ctx(), &self.settings);
        }
    }

    fn settings_border_colors_section(&mut self, ui: &mut egui::Ui, p: Palette) -> bool {
        let mut changed = false;
        let toggle_id = egui::Id::NULL.with("settings_borders_drawer_open");
        let mut is_open = ui
            .ctx()
            .data(|d| d.get_temp::<bool>(toggle_id))
            .unwrap_or(false);

        ui.horizontal(|ui| {
            let arrow = if is_open { "▾" } else { "▸" };
            let r = ui.add(
                egui::Label::new(
                    RichText::new(format!("{arrow} Border Colors (optional)"))
                        .size(13.0)
                        .color(p.muted),
                )
                .sense(Sense::click()),
            );
            if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                is_open = !is_open;
                ui.ctx().data_mut(|d| d.insert_temp(toggle_id, is_open));
            }
        });

        if !is_open {
            return false;
        }

        ui.add_space(6.0);
        let rows = [
            ("Borders", self.settings.borders_color, p.border, 0u8),
            ("Profile", self.settings.profile_color, p.profile_border, 1),
            ("Friends", self.settings.friends_color, p.friends_border, 2),
            ("Games", self.settings.games_border_color, p.games_border, 3),
            ("Text boxes", self.settings.text_border_color, p.text_border, 4),
        ];
        for (label, cur, fallback, slot_id) in rows {
            ui.horizontal(|ui| {
                ui.add_sized([90.0, 20.0], egui::Label::new(label));
                let mut rgb = cur.unwrap_or([fallback.r(), fallback.g(), fallback.b()]);
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    match slot_id {
                        0 => self.settings.borders_color = Some(rgb),
                        1 => self.settings.profile_color = Some(rgb),
                        2 => self.settings.friends_color = Some(rgb),
                        3 => self.settings.games_border_color = Some(rgb),
                        _ => self.settings.text_border_color = Some(rgb),
                    }
                    changed = true;
                }
                if cur.is_some() && ui.small_button("Reset").clicked() {
                    match slot_id {
                        0 => self.settings.borders_color = None,
                        1 => self.settings.profile_color = None,
                        2 => self.settings.friends_color = None,
                        3 => self.settings.games_border_color = None,
                        _ => self.settings.text_border_color = None,
                    }
                    changed = true;
                }
            });
        }
        changed
    }

    fn settings_advanced_card(&mut self, ui: &mut egui::Ui) {
        let p = pal(ui);
        card_frame(ui, 18).show(ui, |ui| {
            ui.set_width(ui.available_width());

            let header = ui
                .horizontal(|ui| {
                    let angle = if self.settings_advanced_open {
                        std::f32::consts::FRAC_PI_2
                    } else {
                        0.0
                    };
                    let chevron = egui::Image::new(egui::include_image!(
                        "../../assets/icon_arrow_right.png"
                    ))
                    .fit_to_exact_size(egui::vec2(12.0, 12.0))
                    .rotate(angle, egui::Vec2::splat(0.5))
                    .tint(p.muted);
                    let arrow_resp = ui.add(chevron.sense(Sense::click()));
                    let title_resp = ui.add(
                        egui::Label::new(RichText::new("Advanced").size(17.0).strong())
                            .sense(Sense::click()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let summary = if self.settings_advanced_open {
                            "Hide"
                        } else {
                            "Roblox Links  •  Data & Cache"
                        };
                        ui.add(
                            egui::Label::new(RichText::new(summary).size(12.0).color(p.muted))
                                .sense(Sense::click()),
                        )
                    })
                    .inner
                        .clicked()
                        || arrow_resp.clicked()
                        || title_resp.clicked()
                });

            if header.response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if header.inner {
                self.settings_advanced_open = !self.settings_advanced_open;
            }

            if self.settings_advanced_open {
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Roblox Links subsection
                ui.label(RichText::new("Roblox Links").size(15.0).strong());
                ui.add_space(4.0);
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

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(10.0);

                // Data & Cache subsection
                ui.label(RichText::new("Data & Cache").size(15.0).strong());
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
            }
        });
    }

    fn settings_theme_showcase(&self, ui: &mut egui::Ui, p: Palette, accent: Color32) {
        let tab_id = egui::Id::NULL.with("settings_preview_tab");
        let mut tab: u8 = ui.ctx().data(|d| d.get_temp(tab_id)).unwrap_or(0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Preview:").color(p.muted));
            for (idx, label) in [(0u8, "Home"), (1, "Profile"), (2, "Login")] {
                if ui.selectable_label(tab == idx, label).clicked() {
                    tab = idx;
                    ui.ctx().data_mut(|d| d.insert_temp(tab_id, tab));
                }
            }
        });
        ui.add_space(6.0);

        let card_w = ui.available_width().min(360.0);
        let preview_h = 136.0_f32;
        let dark = effective_dark_mode(&self.settings);
        let t = ui.input(|i| i.time as f32);
        if tab == 2 {
            ui.ctx().request_repaint();
        }

        let paint_base = |ui: &mut egui::Ui, rect: egui::Rect| {
            let painter = ui.painter();
            painter.rect_filled(rect, CornerRadius::same(10), p.bg);
            if self.settings.bg_mode == BgMode::Wallpaper {
                if let Some((_, tex)) = &self.wallpaper_tex {
                    let mut clipped = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                    clipped.set_clip_rect(rect.intersect(ui.clip_rect()));
                    egui::Image::new(tex)
                        .corner_radius(CornerRadius::same(10))
                        .paint_at(&clipped, rect);
                    let overlay_col = if dark {
                        Color32::from_black_alpha((self.settings.bg_overlay * 255.0).round() as u8)
                    } else {
                        Color32::from_white_alpha((self.settings.bg_overlay * 255.0).round() as u8)
                    };
                    clipped
                        .painter()
                        .rect_filled(rect, CornerRadius::same(10), overlay_col);
                }
            }
        };

        let draw_home = |ui: &mut egui::Ui, w: f32| {
            ui.vertical(|ui| {
                ui.set_width(w);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(w, preview_h),
                    egui::Sense::hover(),
                );
                paint_base(ui, rect);
                let painter = ui.painter_at(rect);

                // Top bar
                let top_h = 20.0;
                let top_rect = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.right(), rect.top() + top_h),
                );
                painter.rect_filled(top_rect, CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 }, p.card_bg);
                painter.line_segment(
                    [top_rect.left_bottom(), top_rect.right_bottom()],
                    egui::Stroke::new(1.0, p.border),
                );
                // Search pill (text_border)
                let search_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 48.0, rect.top() + 4.5),
                    egui::vec2((w - 86.0).max(36.0), 11.0),
                );
                painter.rect_filled(search_rect, CornerRadius::same(5), p.surface_bg);
                painter.rect_stroke(
                    search_rect,
                    CornerRadius::same(5),
                    egui::Stroke::new(1.0, p.text_border),
                    egui::StrokeKind::Inside,
                );
                painter.circle_filled(
                    egui::pos2(rect.right() - 12.0, top_rect.center().y),
                    5.0,
                    accent,
                );

                // Left sidebar
                let side_w = 38.0;
                let side_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), top_rect.bottom()),
                    egui::pos2(rect.left() + side_w, rect.bottom()),
                );
                painter.rect_filled(side_rect, CornerRadius { nw: 0, ne: 0, sw: 10, se: 0 }, p.card_bg);
                painter.line_segment(
                    [side_rect.right_top(), side_rect.right_bottom()],
                    egui::Stroke::new(1.0, p.border),
                );
                for idx in 0..4 {
                    let y = side_rect.top() + 8.0 + idx as f32 * 15.0;
                    let pill = egui::Rect::from_min_size(
                        egui::pos2(side_rect.left() + 5.0, y),
                        egui::vec2(side_w - 5.0, 10.0),
                    );
                    if idx == 0 {
                        painter.rect_filled(
                            pill,
                            CornerRadius { nw: 4, sw: 4, ne: 0, se: 0 },
                            accent.gamma_multiply(0.26),
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(pill.right() - 2.5, pill.top()),
                                egui::vec2(2.5, pill.height()),
                            ),
                            CornerRadius::ZERO,
                            accent,
                        );
                    } else {
                        painter.rect_filled(
                            pill.shrink2(egui::vec2(2.0, 2.0)),
                            CornerRadius::same(3),
                            p.muted.gamma_multiply(0.28),
                        );
                    }
                }

                // Body: Friends row + Game cards
                let bx = side_rect.right() + 10.0;
                let by = top_rect.bottom() + 8.0;
                painter.rect_filled(
                    egui::Rect::from_min_size(egui::pos2(bx, by), egui::vec2(36.0, 5.0)),
                    CornerRadius::same(2),
                    p.text.gamma_multiply(0.85),
                );
                for f in 0..3 {
                    let fc = egui::Rect::from_min_size(
                        egui::pos2(bx + f as f32 * 30.0, by + 9.0),
                        egui::vec2(24.0, 26.0),
                    );
                    if fc.right() < rect.right() - 6.0 {
                        painter.rect_filled(fc, CornerRadius::same(5), p.friends_card_bg);
                        painter.rect_stroke(
                            fc,
                            CornerRadius::same(5),
                            egui::Stroke::new(1.1, p.friends_border),
                            egui::StrokeKind::Inside,
                        );
                        painter.circle_filled(
                            egui::pos2(fc.center().x, fc.top() + 10.0),
                            5.5,
                            p.muted.gamma_multiply(0.45),
                        );
                        painter.circle_filled(
                            egui::pos2(fc.right() - 5.0, fc.top() + 5.0),
                            2.5,
                            GREEN,
                        );
                    }
                }

                let gy = by + 43.0;
                painter.rect_filled(
                    egui::Rect::from_min_size(egui::pos2(bx, gy), egui::vec2(44.0, 5.0)),
                    CornerRadius::same(2),
                    p.text.gamma_multiply(0.85),
                );
                let gcard_w = ((rect.right() - 10.0 - bx - 8.0) / 2.0).max(40.0);
                for g in 0..2 {
                    let gc = egui::Rect::from_min_size(
                        egui::pos2(bx + g as f32 * (gcard_w + 8.0), gy + 9.0),
                        egui::vec2(gcard_w, 46.0),
                    );
                    painter.rect_filled(gc, CornerRadius::same(6), p.card_bg);
                    painter.rect_stroke(
                        gc,
                        CornerRadius::same(6),
                        egui::Stroke::new(1.1, p.games_border),
                        egui::StrokeKind::Inside,
                    );
                    let thumb = egui::Rect::from_min_max(
                        gc.min,
                        egui::pos2(gc.right(), gc.top() + 26.0),
                    );
                    let tcol = if g == 0 { BLUE } else { accent };
                    painter.rect_filled(
                        thumb,
                        CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 },
                        tcol.gamma_multiply(0.55),
                    );
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(gc.left() + 5.0, thumb.bottom() + 5.0),
                            egui::vec2((gcard_w - 14.0).max(16.0), 4.0),
                        ),
                        CornerRadius::same(2),
                        p.text.gamma_multiply(0.85),
                    );
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(gc.left() + 5.0, thumb.bottom() + 12.0),
                            egui::vec2((gcard_w * 0.55).max(12.0), 3.0),
                        ),
                        CornerRadius::same(2),
                        p.muted.gamma_multiply(0.7),
                    );
                }

                // Outer border
                painter.rect_stroke(
                    rect,
                    CornerRadius::same(10),
                    egui::Stroke::new(1.2, p.border),
                    egui::StrokeKind::Inside,
                );
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Home").size(12.0).strong().color(p.muted));
                });
            });
        };

        let draw_profile = |ui: &mut egui::Ui, w: f32| {
            ui.vertical(|ui| {
                ui.set_width(w);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(w, preview_h),
                    egui::Sense::hover(),
                );
                paint_base(ui, rect);
                let painter = ui.painter_at(rect);

                // Top bar
                let top_h = 20.0;
                let top_rect = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.right(), rect.top() + top_h),
                );
                painter.rect_filled(top_rect, CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 }, p.card_bg);
                painter.line_segment(
                    [top_rect.left_bottom(), top_rect.right_bottom()],
                    egui::Stroke::new(1.0, p.border),
                );

                // Left sidebar
                let side_w = 38.0;
                let side_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), top_rect.bottom()),
                    egui::pos2(rect.left() + side_w, rect.bottom()),
                );
                painter.rect_filled(side_rect, CornerRadius { nw: 0, ne: 0, sw: 10, se: 0 }, p.card_bg);
                painter.line_segment(
                    [side_rect.right_top(), side_rect.right_bottom()],
                    egui::Stroke::new(1.0, p.border),
                );
                for idx in 0..4 {
                    let y = side_rect.top() + 8.0 + idx as f32 * 15.0;
                    let pill = egui::Rect::from_min_size(
                        egui::pos2(side_rect.left() + 5.0, y),
                        egui::vec2(side_w - 5.0, 10.0),
                    );
                    if idx == 2 {
                        painter.rect_filled(
                            pill,
                            CornerRadius { nw: 4, sw: 4, ne: 0, se: 0 },
                            accent.gamma_multiply(0.26),
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(pill.right() - 2.5, pill.top()),
                                egui::vec2(2.5, pill.height()),
                            ),
                            CornerRadius::ZERO,
                            accent,
                        );
                    } else {
                        painter.rect_filled(
                            pill.shrink2(egui::vec2(2.0, 2.0)),
                            CornerRadius::same(3),
                            p.muted.gamma_multiply(0.28),
                        );
                    }
                }

                // Profile Header Card (profile_card_bg + profile_border)
                let bx = side_rect.right() + 10.0;
                let bw = (rect.right() - 10.0 - bx).max(60.0);
                let prof_card = egui::Rect::from_min_size(
                    egui::pos2(bx, top_rect.bottom() + 8.0),
                    egui::vec2(bw, 44.0),
                );
                painter.rect_filled(prof_card, CornerRadius::same(7), p.profile_card_bg);
                painter.rect_stroke(
                    prof_card,
                    CornerRadius::same(7),
                    egui::Stroke::new(1.1, p.profile_border),
                    egui::StrokeKind::Inside,
                );
                // Round avatar inside profile
                let av_center = egui::pos2(prof_card.left() + 20.0, prof_card.center().y);
                painter.circle_filled(av_center, 13.0, p.surface_bg);
                painter.circle_stroke(av_center, 13.0, egui::Stroke::new(1.2, p.profile_border));
                painter.circle_filled(av_center, 5.5, p.muted.gamma_multiply(0.55));
                // Name + @handle + verified dot
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(prof_card.left() + 40.0, prof_card.top() + 12.0),
                        egui::vec2((bw * 0.45).max(28.0), 5.5),
                    ),
                    CornerRadius::same(2),
                    p.text.gamma_multiply(0.9),
                );
                painter.circle_filled(
                    egui::pos2(
                        prof_card.left() + 44.0 + (bw * 0.45).max(28.0),
                        prof_card.top() + 14.5,
                    ),
                    3.0,
                    BLUE,
                );
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(prof_card.left() + 40.0, prof_card.top() + 22.0),
                        egui::vec2((bw * 0.30).max(20.0), 3.5),
                    ),
                    CornerRadius::same(2),
                    p.muted.gamma_multiply(0.75),
                );

                // Bio / About card (text_border)
                let bio_card = egui::Rect::from_min_size(
                    egui::pos2(bx, prof_card.bottom() + 7.0),
                    egui::vec2(bw, 26.0),
                );
                painter.rect_filled(bio_card, CornerRadius::same(6), p.surface_bg);
                painter.rect_stroke(
                    bio_card,
                    CornerRadius::same(6),
                    egui::Stroke::new(1.1, p.text_border),
                    egui::StrokeKind::Inside,
                );
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(bio_card.left() + 7.0, bio_card.top() + 7.0),
                        egui::vec2((bw - 24.0).max(20.0), 3.5),
                    ),
                    CornerRadius::same(2),
                    p.text.gamma_multiply(0.75),
                );
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(bio_card.left() + 7.0, bio_card.top() + 14.0),
                        egui::vec2((bw * 0.55).max(16.0), 3.5),
                    ),
                    CornerRadius::same(2),
                    p.muted.gamma_multiply(0.65),
                );

                // Favorite / Badge chips (games_border)
                let chip_y = bio_card.bottom() + 7.0;
                let chip_w = ((bw - 8.0) / 2.0).max(26.0);
                for c in 0..2 {
                    let chip = egui::Rect::from_min_size(
                        egui::pos2(bx + c as f32 * (chip_w + 8.0), chip_y),
                        egui::vec2(chip_w, 16.0),
                    );
                    painter.rect_filled(chip, CornerRadius::same(5), p.card_bg);
                    painter.rect_stroke(
                        chip,
                        CornerRadius::same(5),
                        egui::Stroke::new(1.0, p.games_border),
                        egui::StrokeKind::Inside,
                    );
                }

                painter.rect_stroke(
                    rect,
                    CornerRadius::same(10),
                    egui::Stroke::new(1.2, p.border),
                    egui::StrokeKind::Inside,
                );
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Profile").size(12.0).strong().color(p.muted));
                });
            });
        };

        let draw_login = |ui: &mut egui::Ui, w: f32| {
            ui.vertical(|ui| {
                ui.set_width(w);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(w, preview_h),
                    egui::Sense::hover(),
                );
                let painter = ui.painter();
                let login_bg = if dark {
                    Color32::from_rgb(17, 19, 25)
                } else {
                    Color32::from_rgb(245, 246, 250)
                };
                painter.rect_filled(rect, CornerRadius::same(10), login_bg);

                // Real live miniature sideways-scrolling Ferris & Roblox backdrop!
                draw_login_marquee(ui, rect, t, dark, 0.36);

                // Mini centered Login card
                let card_rect = egui::Rect::from_center_size(
                    rect.center(),
                    egui::vec2((w * 0.60).clamp(96.0, 146.0), 88.0),
                );
                let card_bg = if dark {
                    Color32::from_rgba_unmultiplied(21, 25, 34, 242)
                } else {
                    Color32::from_rgba_unmultiplied(255, 253, 250, 245)
                };
                let card_border = if dark {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 26)
                } else {
                    Color32::from_rgba_unmultiplied(255, 130, 72, 70)
                };
                let painter = ui.painter_at(rect);
                painter.rect_filled(
                    card_rect.translate(egui::vec2(0.0, 3.0)),
                    CornerRadius::same(9),
                    Color32::from_black_alpha(if dark { 80 } else { 28 }),
                );
                painter.rect_filled(card_rect, CornerRadius::same(9), card_bg);
                painter.rect_stroke(
                    card_rect,
                    CornerRadius::same(9),
                    egui::Stroke::new(1.0, card_border),
                    egui::StrokeKind::Inside,
                );

                // Mini static logo
                let logo_rect = egui::Rect::from_center_size(
                    egui::pos2(card_rect.center().x, card_rect.top() + 14.0),
                    egui::vec2(14.0, 14.0),
                );
                egui::Image::new(egui::include_image!("../../assets/logo.png"))
                    .paint_at(ui, logo_rect);

                // Mini title + subtitle bars
                painter.rect_filled(
                    egui::Rect::from_center_size(
                        egui::pos2(card_rect.center().x, card_rect.top() + 27.0),
                        egui::vec2(46.0, 4.5),
                    ),
                    CornerRadius::same(2),
                    p.text.gamma_multiply(0.88),
                );
                painter.rect_filled(
                    egui::Rect::from_center_size(
                        egui::pos2(card_rect.center().x, card_rect.top() + 35.0),
                        egui::vec2(64.0, 3.0),
                    ),
                    CornerRadius::same(1),
                    p.muted.gamma_multiply(0.7),
                );

                // Mini white QR square hint + tactile orange button
                let qr_mini = egui::Rect::from_center_size(
                    egui::pos2(card_rect.center().x, card_rect.top() + 52.0),
                    egui::vec2(18.0, 18.0),
                );
                painter.rect_filled(qr_mini, CornerRadius::same(3), Color32::WHITE);
                painter.rect_filled(
                    qr_mini.shrink(4.0),
                    CornerRadius::same(1),
                    Color32::from_rgb(28, 32, 40),
                );

                let btn_rect = egui::Rect::from_center_size(
                    egui::pos2(card_rect.center().x, card_rect.bottom() - 14.0),
                    egui::vec2(card_rect.width() - 28.0, 14.0),
                );
                painter.rect_filled(
                    btn_rect.translate(egui::vec2(0.0, 2.0)),
                    CornerRadius::same(4),
                    Color32::from_rgb(168, 70, 28),
                );
                painter.rect_filled(
                    btn_rect,
                    CornerRadius::same(4),
                    Color32::from_rgb(255, 107, 43),
                );

                painter.rect_stroke(
                    rect,
                    CornerRadius::same(10),
                    egui::Stroke::new(1.2, p.border),
                    egui::StrokeKind::Inside,
                );
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Login").size(12.0).strong().color(p.muted));
                });
            });
        };

        match tab {
            0 => draw_home(ui, card_w),
            1 => draw_profile(ui, card_w),
            _ => draw_login(ui, card_w),
        }
    }
}
