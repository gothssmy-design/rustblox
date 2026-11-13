use std::path::Path;
use std::sync::atomic::Ordering;
use eframe::egui::{self, Color32, CornerRadius, RichText, Sense};
use crate::app::RustBlox;
use crate::auth::{data_dir, save_settings, write_ids};
use crate::roblox_process::{
    apply_fps_setting, close_tray_roblox_processes, find_player, handler_status, register, restore,
};
use crate::theme::{
    accent_color, apply_theme, card_frame, effective_dark_mode, is_dark_color, pal, Palette,
    BLUE, GREEN, ORANGE, RED,
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

const BORDER_QUICK_SWATCHES: [([u8; 3], &str); 7] = [
    ([0, 196, 132], "Emerald"),
    ([64, 172, 255], "Sky Blue"),
    ([242, 92, 148], "Rose Pink"),
    ([164, 132, 255], "Violet"),
    ([245, 188, 66], "Amber Gold"),
    ([235, 240, 248], "Frost White"),
    ([86, 94, 110], "Slate"),
];

#[derive(Clone, Copy)]
struct ThemePack {
    name: &'static str,
    subtitle: &'static str,
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

const THEME_PACKS: [ThemePack; 8] = [
    ThemePack {
        name: "Classic Light",
        subtitle: "Clean & bright",
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
        name: "Midnight",
        subtitle: "Neutral dark",
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
        name: "Cyber Mint",
        subtitle: "Neon emerald",
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
        name: "Velvet Rose",
        subtitle: "Deep rose glow",
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
        name: "Deep Ocean",
        subtitle: "Cool azure",
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
        name: "Royal Indigo",
        subtitle: "Soft violet",
        dark: true,
        bg_mode: BgMode::SolidColor,
        bg_color: [26, 22, 50],
        text_mode: TextColorMode::Auto,
        borders: Some([142, 116, 245]),
        profile: Some([168, 140, 255]),
        friends: Some([182, 156, 255]),
        games: Some([128, 102, 230]),
        text_border: Some([158, 134, 255]),
        preview_accent: [168, 140, 255],
    },
    ThemePack {
        name: "Soft Blossom",
        subtitle: "Pastel rose",
        dark: false,
        bg_mode: BgMode::SolidColor,
        bg_color: [248, 218, 230],
        text_mode: TextColorMode::Auto,
        borders: Some([198, 92, 136]),
        profile: Some([184, 62, 114]),
        friends: Some([204, 82, 134]),
        games: Some([188, 84, 128]),
        text_border: Some([176, 68, 114]),
        preview_accent: [184, 62, 114],
    },
    ThemePack {
        name: "Warm Sand",
        subtitle: "Cozy daylight",
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
        let accent = accent_color(ui);
        let accent_fg = if is_dark_color([accent.r(), accent.g(), accent.b()]) {
            Color32::WHITE
        } else {
            Color32::from_rgb(16, 19, 26)
        };
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
                        .stroke(egui::Stroke::new(1.0, p.border))
                        .corner_radius(10)
                        .inner_margin(egui::Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            ui.label(RichText::new(tag).size(11.5).color(p.muted));
                        });
                });
            });
            ui.add_space(4.0);
            small_text(
                ui,
                "Pick a 1-click theme preset below, or fine-tune backdrop and border colors."
                    .to_string(),
            );
            ui.add_space(10.0);

            // 1-Click Visual Theme Packs (4 cards per row on desktop)
            let avail_w = ui.available_width();
            let pack_gap = 8.0;
            let cols = if avail_w >= 620.0 { 4 } else { 2 };
            let pack_w = ((avail_w - pack_gap * (cols as f32 - 1.0)) / cols as f32).max(140.0);

            for row in THEME_PACKS.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = pack_gap;
                    for pack in row {
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

                        if render_theme_pack_card(ui, pack_w, p, accent, pack, is_active) {
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
                ui.add_space(8.0);
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(10.0);

            // Mode & Backdrop style controls
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
                                .color(if selected { accent_fg } else { p.text }),
                        )
                        .fill(if selected { accent } else { p.surface_bg })
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
                    ui.label(RichText::new("Backdrop").color(p.muted));
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
                                .color(if selected { accent_fg } else { p.text }),
                        )
                        .fill(if selected { accent } else { p.surface_bg })
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

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(10.0);
            if self.settings_border_colors_section(ui, p, accent_color(ui)) {
                changed = true;
            }
        });

        if changed {
            save_settings(&self.settings);
            apply_theme(ui.ctx(), &self.settings);
        }
    }

    fn settings_border_colors_section(
        &mut self,
        ui: &mut egui::Ui,
        p: Palette,
        accent: Color32,
    ) -> bool {
        let mut changed = false;
        let custom_count = [
            self.settings.borders_color,
            self.settings.profile_color,
            self.settings.friends_color,
            self.settings.games_border_color,
            self.settings.text_border_color,
        ]
        .into_iter()
        .filter(|c| c.is_some())
        .count();

        let toggle_id = egui::Id::NULL.with("settings_borders_drawer_open");
        let mut is_open = ui
            .ctx()
            .data(|d| d.get_temp::<bool>(toggle_id))
            .unwrap_or(custom_count > 0);

        ui.horizontal(|ui| {
            let arrow = if is_open { "\u{25BE}" } else { "\u{25B8}" };
            let header_resp = ui
                .horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(RichText::new(arrow).size(13.0).color(p.muted));
                    ui.label(RichText::new("Customize Borders & Elements").size(14.5).strong());
                    if custom_count > 0 {
                        egui::Frame::new()
                            .fill(p.surface_bg)
                            .stroke(egui::Stroke::new(1.0, accent))
                            .corner_radius(8)
                            .inner_margin(egui::Margin::symmetric(7, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("{custom_count} custom"))
                                        .size(11.0)
                                        .color(accent),
                                );
                            });
                    }
                })
                .response
                .interact(Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if header_resp.clicked() {
                is_open = !is_open;
                ui.ctx().data_mut(|d| d.insert_temp(toggle_id, is_open));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if custom_count > 0
                    && ui
                        .add(
                            egui::Button::new(RichText::new("Reset All").size(11.5).color(p.text))
                                .fill(p.surface_bg)
                                .stroke(egui::Stroke::new(1.0, p.border))
                                .corner_radius(7),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                {
                    self.settings.borders_color = None;
                    self.settings.profile_color = None;
                    self.settings.friends_color = None;
                    self.settings.games_border_color = None;
                    self.settings.text_border_color = None;
                    changed = true;
                }
            });
        });

        if !is_open {
            return changed;
        }

        ui.add_space(6.0);
        // Quick 1-click border tint strip
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(7.0, 6.0);
            ui.label(RichText::new("Apply to all:").size(12.5).color(p.muted));
            for (rgb, name) in BORDER_QUICK_SWATCHES {
                let col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), Sense::click());
                if ui.is_rect_visible(rect) {
                    ui.painter().circle_filled(rect.center(), 9.0, col);
                    let stroke_col = if resp.hovered() { p.text } else { p.border };
                    ui.painter()
                        .circle_stroke(rect.center(), 9.5, egui::Stroke::new(1.2, stroke_col));
                }
                if resp
                    .on_hover_text(format!("Apply {name} to all borders"))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.settings.borders_color = Some(rgb);
                    self.settings.profile_color = Some(rgb);
                    self.settings.friends_color = Some(rgb);
                    self.settings.games_border_color = Some(rgb);
                    self.settings.text_border_color = Some(rgb);
                    changed = true;
                }
            }
            if self.settings.bg_mode == BgMode::SolidColor
                && ui
                    .add(
                        egui::Button::new(
                            RichText::new("Match Theme").size(11.5).color(Color32::WHITE),
                        )
                        .fill(accent)
                        .corner_radius(7),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
            {
                let rgb = [accent.r(), accent.g(), accent.b()];
                self.settings.borders_color = Some(rgb);
                self.settings.profile_color = Some(rgb);
                self.settings.friends_color = Some(rgb);
                self.settings.games_border_color = Some(rgb);
                self.settings.text_border_color = Some(rgb);
                changed = true;
            }
        });

        ui.add_space(10.0);

        let fallback_border = [p.border.r(), p.border.g(), p.border.b()];
        let fallback_profile = [p.profile_border.r(), p.profile_border.g(), p.profile_border.b()];
        let fallback_friends = [p.friends_border.r(), p.friends_border.g(), p.friends_border.b()];
        let fallback_games = [p.games_border.r(), p.games_border.g(), p.games_border.b()];
        let fallback_text = [p.text_border.r(), p.text_border.g(), p.text_border.b()];

        let avail_w = ui.available_width();
        let gap = 10.0;
        let col_w = ((avail_w - gap) * 0.5).max(200.0);

        let items: Vec<(&str, &str, Option<[u8; 3]>, [u8; 3], u8)> = vec![
            ("General Borders", "Cards, panels & dividers", self.settings.borders_color, fallback_border, 0),
            ("Profiles", "Profile cards & avatar frame", self.settings.profile_color, fallback_profile, 1),
            ("Friend List", "Friend avatars & search cards", self.settings.friends_color, fallback_friends, 2),
            ("Games Border", "Game thumbnail outlines", self.settings.games_border_color, fallback_games, 3),
            ("Text Boxes", "Search bars & bio input", self.settings.text_border_color, fallback_text, 4),
        ];

        for pair in items.chunks(2) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &(title, hint, cur_opt, fallback, slot_id) in pair {
                    let mut next_val = cur_opt;
                    let row_changed = render_color_slot_card(ui, col_w, p, title, hint, &mut next_val, fallback);
                    if row_changed {
                        match slot_id {
                            0 => self.settings.borders_color = next_val,
                            1 => self.settings.profile_color = next_val,
                            2 => self.settings.friends_color = next_val,
                            3 => self.settings.games_border_color = next_val,
                            _ => self.settings.text_border_color = next_val,
                        }
                        changed = true;
                    }
                }
            });
            ui.add_space(8.0);
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
}

fn render_color_slot_card(
    ui: &mut egui::Ui,
    width: f32,
    p: Palette,
    title: &str,
    hint: &str,
    slot: &mut Option<[u8; 3]>,
    fallback: [u8; 3],
) -> bool {
    let mut changed = false;
    let is_custom = slot.is_some();
    let mut rgb = slot.unwrap_or(fallback);
    let preview_border = if is_custom {
        Color32::from_rgb(rgb[0], rgb[1], rgb[2])
    } else {
        p.border
    };

    egui::Frame::new()
        .fill(p.surface_bg)
        .stroke(egui::Stroke::new(1.0, preview_border))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width((width - 26.0).max(160.0));
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.label(RichText::new(title).size(13.0).strong().color(p.text));
                    ui.label(RichText::new(hint).size(11.0).color(p.muted));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if ui
                        .color_edit_button_srgb(&mut rgb)
                        .on_hover_text(format!("Pick {title} color"))
                        .changed()
                    {
                        *slot = Some(rgb);
                        changed = true;
                    }

                    if is_custom {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Auto").size(11.0).color(p.muted))
                                    .fill(p.card_bg)
                                    .stroke(egui::Stroke::new(1.0, p.border))
                                    .corner_radius(6),
                            )
                            .on_hover_text("Reset to theme Auto")
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            *slot = None;
                            changed = true;
                        }
                    } else {
                        ui.label(RichText::new("Auto").size(11.0).color(p.muted));
                    }
                });
            });
        });

    changed
}

fn render_theme_pack_card(
    ui: &mut egui::Ui,
    width: f32,
    p: Palette,
    active_accent: Color32,
    pack: &ThemePack,
    is_active: bool,
) -> bool {
    let size = egui::vec2(width, 62.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hovered = resp.hovered();

    let border_col = if is_active {
        active_accent
    } else if hovered {
        p.text
    } else {
        p.border
    };
    let card_fill = if is_active { p.selected_bg } else { p.surface_bg };

    if ui.is_rect_visible(rect) {
        ui.painter().rect(
            rect,
            CornerRadius::same(10),
            card_fill,
            egui::Stroke::new(if is_active { 1.8 } else { 1.0 }, border_col),
            egui::StrokeKind::Inside,
        );

        // Mini live preview box on the left
        let pv = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
            egui::vec2(46.0, 46.0),
        );
        let bg_c = Color32::from_rgb(pack.bg_color[0], pack.bg_color[1], pack.bg_color[2]);
        let ac_c = Color32::from_rgb(
            pack.preview_accent[0],
            pack.preview_accent[1],
            pack.preview_accent[2],
        );
        let inner_fill = if pack.dark {
            Color32::from_rgba_unmultiplied(16, 19, 27, 210)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 224)
        };
        let pv_border = pack
            .borders
            .map(|[r, g, b]| Color32::from_rgb(r, g, b))
            .unwrap_or(ac_c);

        ui.painter().rect_filled(pv, CornerRadius::same(7), bg_c);
        ui.painter().rect(
            pv.shrink(6.0),
            CornerRadius::same(5),
            inner_fill,
            egui::Stroke::new(1.2, pv_border),
            egui::StrokeKind::Inside,
        );
        // Tiny accent bar + friend dot inside preview
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(pv.left() + 10.0, pv.top() + 12.0),
            egui::vec2(16.0, 4.0),
        );
        ui.painter().rect_filled(bar_rect, CornerRadius::same(2), ac_c);
        ui.painter().circle_filled(
            egui::pos2(pv.right() - 13.0, pv.bottom() - 13.0),
            3.5,
            ac_c,
        );

        // Name & subtitle on the right
        let text_left = pv.right() + 9.0;
        ui.painter().text(
            egui::pos2(text_left, rect.top() + 21.0),
            egui::Align2::LEFT_CENTER,
            pack.name,
            egui::FontId::new(13.0, egui::FontFamily::Name("Bold".into())),
            p.text,
        );
        ui.painter().text(
            egui::pos2(text_left, rect.top() + 39.0),
            egui::Align2::LEFT_CENTER,
            pack.subtitle,
            egui::FontId::proportional(11.0),
            p.muted,
        );
    }

    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}
