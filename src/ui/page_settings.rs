use std::path::Path;
use std::sync::atomic::Ordering;
use eframe::egui::{self, Color32, CornerRadius, RichText, Sense};
use crate::app::RustBlox;
use crate::auth::{data_dir, save_settings, write_ids};
use crate::roblox_process::{
    apply_fps_setting, close_tray_roblox_processes, find_player, handler_status, register, restore,
};
use crate::theme::{
    apply_theme, card_frame, effective_dark_mode, is_dark_color, pal, BLUE, GREEN, ORANGE, RED,
};
use crate::types::{Action, BgMode, TextColorMode};
use crate::ui::widgets::*;

const COLOR_PRESETS: [([u8; 3], &str); 10] = [
    ([17, 19, 24], "Midnight Slate"),
    ([16, 30, 48], "Deep Ocean"),
    ([28, 24, 52], "Royal Indigo"),
    ([16, 38, 32], "Forest Pine"),
    ([184, 50, 95], "Deep Rose"),
    ([222, 74, 122], "Vibrant Rose"),
    ([248, 212, 226], "Soft Rose"),
    ([242, 246, 252], "Soft Cloud"),
    ([224, 237, 250], "Sky Mist"),
    ([245, 239, 230], "Warm Sand"),
];

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
                    ui.label(RichText::new("Not signed in.").color(p.muted));
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

            // Advanced collapsible section (Roblox Links + Data & Cache)
            self.settings_advanced_card(ui);
        });
        ui.add_space(20.0);
    }

    fn settings_theme_card(&mut self, ui: &mut egui::Ui) {
        let p = pal(ui);
        let mut changed = false;

        card_frame(ui, 18).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("Theme & Background").size(17.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let tag = match self.settings.bg_mode {
                        BgMode::ThemeDefault => "Default",
                        BgMode::SolidColor => "Custom Color",
                        BgMode::Wallpaper => "Wallpaper",
                    };
                    egui::Frame::new()
                        .fill(p.surface_bg)
                        .corner_radius(10)
                        .inner_margin(egui::Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            ui.label(RichText::new(tag).size(11.5).color(p.muted));
                        });
                });
            });
            ui.add_space(10.0);

            // Theme mode & Background style controls
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 18.0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(RichText::new("Mode").color(p.muted));
                    let active_dark = effective_dark_mode(&self.settings);
                    for (dark_opt, label) in [(false, "Light"), (true, "Dark")] {
                        let selected = active_dark == dark_opt;
                        let btn = egui::Button::new(
                            RichText::new(label)
                                .size(13.0)
                                .strong()
                                .color(if selected { Color32::WHITE } else { p.text }),
                        )
                        .fill(if selected { GREEN } else { p.surface_bg })
                        .corner_radius(8);
                        if ui
                            .add(btn)
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                            && !selected
                        {
                            self.settings.dark = dark_opt;
                            if self.settings.bg_mode == BgMode::SolidColor {
                                self.settings.text_mode = if dark_opt {
                                    TextColorMode::White
                                } else {
                                    TextColorMode::Dark
                                };
                            }
                            changed = true;
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(RichText::new("Background").color(p.muted));
                    for (mode_opt, label) in [
                        (BgMode::ThemeDefault, "Default"),
                        (BgMode::SolidColor, "Color"),
                        (BgMode::Wallpaper, "Wallpaper"),
                    ] {
                        let selected = self.settings.bg_mode == mode_opt;
                        let btn = egui::Button::new(
                            RichText::new(label)
                                .size(13.0)
                                .strong()
                                .color(if selected { Color32::WHITE } else { p.text }),
                        )
                        .fill(if selected { BLUE } else { p.surface_bg })
                        .corner_radius(8);
                        if ui
                            .add(btn)
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                            && !selected
                        {
                            self.settings.bg_mode = mode_opt;
                            if mode_opt == BgMode::SolidColor
                                && self.settings.text_mode == TextColorMode::Auto
                            {
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
            });

            match self.settings.bg_mode {
                BgMode::ThemeDefault => {
                    ui.add_space(6.0);
                    small_text(
                        ui,
                        "Using clean default Light or Dark theme colors. Pick Color or Wallpaper to customize the backdrop."
                            .to_string(),
                    );
                }
                BgMode::SolidColor => {
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                        ui.label(RichText::new("Presets:").size(13.0).color(p.muted));
                        for (rgb, name) in COLOR_PRESETS {
                            let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                            let selected = self.settings.bg_color == rgb;
                            let (rect, resp) = ui.allocate_exact_size(
                                egui::vec2(26.0, 26.0),
                                Sense::click(),
                            );
                            if ui.is_rect_visible(rect) {
                                ui.painter().rect_filled(rect, CornerRadius::same(7), color);
                                let stroke = if selected {
                                    egui::Stroke::new(2.2, GREEN)
                                } else if resp.hovered() {
                                    egui::Stroke::new(1.5, p.text)
                                } else {
                                    egui::Stroke::new(1.0, p.border)
                                };
                                ui.painter().rect_stroke(
                                    rect,
                                    CornerRadius::same(7),
                                    stroke,
                                    egui::StrokeKind::Inside,
                                );
                            }
                            if resp
                                .on_hover_text(name)
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.settings.bg_color = rgb;
                                if self.settings.text_mode == TextColorMode::Auto {
                                    self.settings.dark = is_dark_color(rgb);
                                }
                                changed = true;
                            }
                        }
                    });

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        ui.label("Custom color:");
                        if ui.color_edit_button_srgb(&mut self.settings.bg_color).changed() {
                            if self.settings.text_mode == TextColorMode::Auto {
                                self.settings.dark = is_dark_color(self.settings.bg_color);
                            }
                            changed = true;
                        }
                        let [r, g, b] = self.settings.bg_color;
                        ui.label(
                            RichText::new(format!("#{r:02X}{g:02X}{b:02X}"))
                                .monospace()
                                .size(12.5)
                                .color(p.muted),
                        );
                        if ui.button("Reset").clicked() {
                            self.settings.bg_mode = BgMode::ThemeDefault;
                            self.settings.text_mode = TextColorMode::Auto;
                            changed = true;
                        }
                    });

                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        ui.label(RichText::new("Text contrast:").size(13.0).color(p.muted));
                        let auto_tag = if is_dark_color(self.settings.bg_color) {
                            "Auto (White)"
                        } else {
                            "Auto (Dark)"
                        };
                        for (t_mode, label) in [
                            (TextColorMode::Auto, auto_tag),
                            (TextColorMode::White, "White"),
                            (TextColorMode::Green, "Green"),
                            (TextColorMode::Dark, "Dark"),
                        ] {
                            let selected = self.settings.text_mode == t_mode;
                            let btn = egui::Button::new(
                                RichText::new(label)
                                    .size(12.5)
                                    .strong()
                                    .color(if selected { Color32::WHITE } else { p.text }),
                            )
                            .fill(if selected { GREEN } else { p.surface_bg })
                            .corner_radius(7);
                            if ui
                                .add(btn)
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                                && !selected
                            {
                                self.settings.text_mode = t_mode;
                                match t_mode {
                                    TextColorMode::Auto => {
                                        self.settings.dark = is_dark_color(self.settings.bg_color);
                                    }
                                    TextColorMode::White | TextColorMode::Green => {
                                        self.settings.dark = true;
                                    }
                                    TextColorMode::Dark => {
                                        self.settings.dark = false;
                                    }
                                }
                                changed = true;
                            }
                        }
                    });

                    let wants_dark = is_dark_color(self.settings.bg_color);
                    let active_dark = effective_dark_mode(&self.settings);
                    if wants_dark != active_dark {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let label = if wants_dark {
                                "Dark text may be hard to read on this color."
                            } else {
                                "Light text may be hard to read on this color."
                            };
                            ui.label(RichText::new(label).size(12.0).color(ORANGE));
                            if ui.small_button("Use Auto contrast").clicked() {
                                self.settings.text_mode = TextColorMode::Auto;
                                self.settings.dark = wants_dark;
                                changed = true;
                            }
                        });
                    }
                }
                BgMode::Wallpaper => {
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 14.0;
                        let thumb_size = egui::vec2(144.0, 81.0);
                        let (rect, _) = ui.allocate_exact_size(thumb_size, Sense::hover());
                        if ui.is_rect_visible(rect) {
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::same(8),
                                p.surface_bg,
                            );
                            if let Some((_, tex)) = &self.wallpaper_tex {
                                egui::Image::new(tex)
                                    .corner_radius(8)
                                    .paint_at(ui, rect);
                            } else {
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "No image",
                                    egui::FontId::proportional(12.0),
                                    p.muted,
                                );
                            }
                            ui.painter().rect_stroke(
                                rect,
                                CornerRadius::same(8),
                                egui::Stroke::new(1.0, p.border),
                                egui::StrokeKind::Inside,
                            );
                        }

                        ui.vertical(|ui| {
                            if let Some((path, tex)) = &self.wallpaper_tex {
                                let name = Path::new(path)
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(path);
                                let [w, h] = tex.size();
                                ui.label(RichText::new(name).strong());
                                small_text(ui, format!("{w} x {h} px"));
                            } else if self.wallpaper_loading.is_some() {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Loading wallpaper...");
                                });
                            } else if !self.settings.bg_wallpaper.is_empty() {
                                ui.label(&self.settings.bg_wallpaper);
                            } else {
                                ui.label(
                                    RichText::new("Choose an image file to use as background.")
                                        .color(p.muted),
                                );
                            }

                            if let Some(err) = &self.wallpaper_err {
                                ui.label(RichText::new(err).size(12.0).color(RED));
                            }

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                let browse = egui::Button::new(
                                    RichText::new("Browse Image...")
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(BLUE)
                                .corner_radius(8);
                                if ui
                                    .add(browse)
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    self.open_wallpaper_picker();
                                }
                                if !self.settings.bg_wallpaper.is_empty()
                                    && ui.button("Clear").clicked()
                                {
                                    self.settings.bg_wallpaper.clear();
                                    self.settings.bg_mode = BgMode::ThemeDefault;
                                    self.wallpaper_tex = None;
                                    self.wallpaper_err = None;
                                    changed = true;
                                }
                            });
                        });
                    });

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label("Dim / Tint overlay:");
                        let mut pct = (self.settings.bg_overlay * 100.0).round();
                        if ui
                            .add(egui::Slider::new(&mut pct, 0.0..=85.0).suffix("%"))
                            .changed()
                        {
                            self.settings.bg_overlay = (pct / 100.0).clamp(0.0, 0.85);
                            changed = true;
                        }
                    });
                    ui.add_space(4.0);
                    small_text(
                        ui,
                        "Supports PNG, JPG, WEBP, BMP - or drag & drop an image onto this window."
                            .to_string(),
                    );
                }
            }
        });

        if changed {
            save_settings(&self.settings);
            apply_theme(ui.ctx(), &self.settings);
        }
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
}
