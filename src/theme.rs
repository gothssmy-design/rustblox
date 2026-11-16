use std::fs;
use std::sync::Arc;
use eframe::egui::{self, Color32, CornerRadius, Sense};
use crate::types::{BgMode, Settings, TextColorMode};

const THEME_PALETTE_KEY: &str = "rustblox_theme_palette";

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color32,
    pub card_bg: Color32,
    pub surface_bg: Color32,
    pub border: Color32,
    pub profile_border: Color32,
    pub profile_card_bg: Color32,
    pub friends_border: Color32,
    pub friends_card_bg: Color32,
    pub games_border: Color32,
    pub text_border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub placeholder: Color32,
    pub selected_bg: Color32,
    pub hover_bg: Color32,
    pub extreme_bg: Color32,
    pub widget_inactive: Color32,
    pub widget_hovered: Color32,
    pub glass_edition: bool,
    pub glass_highlight: f32,
    pub corner_radius: u8,
}

fn mix_color(a: Color32, b: [u8; 3], t: f32, alpha: u8) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 * inv + b[0] as f32 * t).round().clamp(0.0, 255.0) as u8,
        (a.g() as f32 * inv + b[1] as f32 * t).round().clamp(0.0, 255.0) as u8,
        (a.b() as f32 * inv + b[2] as f32 * t).round().clamp(0.0, 255.0) as u8,
        alpha,
    )
}

fn tint_card_surface(base_card: Color32, accent_rgb: [u8; 3], dark: bool) -> Color32 {
    mix_color(
        base_card,
        accent_rgb,
        if dark { 0.16 } else { 0.11 },
        base_card.a(),
    )
}

fn harmonize_auto_border(bg_rgb: [u8; 3], dark: bool) -> Color32 {
    if dark {
        mix_color(Color32::from_rgb(bg_rgb[0], bg_rgb[1], bg_rgb[2]), [238, 242, 250], 0.48, 84)
    } else {
        mix_color(Color32::from_rgb(bg_rgb[0], bg_rgb[1], bg_rgb[2]), [34, 40, 54], 0.42, 86)
    }
}

impl Palette {
    pub fn of(dark: bool) -> Self {
        if dark {
            let border = Color32::from_rgba_unmultiplied(255, 255, 255, 22);
            let card_bg = Color32::from_rgb(25, 28, 35);
            Self {
                bg: Color32::from_rgb(17, 19, 24),
                card_bg,
                surface_bg: Color32::from_rgb(33, 37, 46),
                border,
                profile_border: border,
                profile_card_bg: card_bg,
                friends_border: border,
                friends_card_bg: card_bg,
                games_border: border,
                text_border: Color32::from_rgba_unmultiplied(255, 255, 255, 36),
                text: Color32::from_rgb(240, 242, 245),
                muted: Color32::from_rgb(156, 163, 175),
                placeholder: Color32::from_rgb(38, 42, 51),
                selected_bg: Color32::from_rgb(44, 49, 60),
                hover_bg: Color32::from_rgb(31, 35, 43),
                extreme_bg: Color32::from_rgb(26, 29, 36),
                widget_inactive: Color32::from_rgb(40, 45, 55),
                widget_hovered: Color32::from_rgb(56, 62, 75),
                glass_edition: false,
                glass_highlight: 0.0,
                corner_radius: 12,
            }
        } else {
            let border = Color32::from_rgb(220, 224, 232);
            let card_bg = Color32::from_rgb(248, 249, 250);
            Self {
                bg: Color32::from_rgb(255, 255, 255),
                card_bg,
                surface_bg: Color32::from_rgb(241, 243, 245),
                border,
                profile_border: border,
                profile_card_bg: card_bg,
                friends_border: border,
                friends_card_bg: card_bg,
                games_border: border,
                text_border: Color32::from_rgb(204, 210, 222),
                text: Color32::from_rgb(25, 25, 25),
                muted: Color32::from_rgb(96, 97, 98),
                placeholder: Color32::from_rgb(233, 236, 239),
                selected_bg: Color32::from_rgb(232, 235, 239),
                hover_bg: Color32::from_rgb(243, 245, 247),
                extreme_bg: Color32::from_rgb(241, 243, 245),
                widget_inactive: Color32::from_rgb(235, 238, 241),
                widget_hovered: Color32::from_rgb(222, 226, 230),
                glass_edition: false,
                glass_highlight: 0.0,
                corner_radius: 12,
            }
        }
    }

    pub fn from_settings(settings: &Settings) -> Self {
        let dark = effective_dark_mode(settings);
        let text = effective_text_color(settings);
        let custom_bg = settings.bg_mode != BgMode::ThemeDefault;
        let radius = settings.corner_roundness.clamp(6, 22);
        let muted = Color32::from_rgba_unmultiplied(
            text.r(),
            text.g(),
            text.b(),
            if dark { 208 } else { 188 },
        );

        let mut p = if !custom_bg {
            let mut base = Palette::of(dark);
            base.text = text;
            base.muted = Color32::from_rgba_unmultiplied(
                text.r(),
                text.g(),
                text.b(),
                if dark { 185 } else { 172 },
            );
            base
        } else if settings.bg_mode == BgMode::SolidColor {
            let bg_rgb = settings.bg_color;
            let (card_bg, surface_bg, extreme_bg, selected_bg, hover_bg, placeholder, widget_inactive, widget_hovered) =
                if dark {
                    (
                        mix_color(Color32::from_rgb(15, 17, 24), bg_rgb, 0.20, 218),
                        mix_color(Color32::from_rgb(255, 255, 255), bg_rgb, 0.22, 34),
                        mix_color(Color32::from_rgb(13, 15, 21), bg_rgb, 0.16, 216),
                        mix_color(Color32::from_rgb(255, 255, 255), bg_rgb, 0.25, 48),
                        mix_color(Color32::from_rgb(255, 255, 255), bg_rgb, 0.20, 26),
                        mix_color(Color32::from_rgb(24, 28, 38), bg_rgb, 0.25, 180),
                        mix_color(Color32::from_rgb(255, 255, 255), bg_rgb, 0.20, 32),
                        mix_color(Color32::from_rgb(255, 255, 255), bg_rgb, 0.26, 52),
                    )
                } else {
                    (
                        mix_color(Color32::from_rgb(255, 255, 255), bg_rgb, 0.14, 232),
                        mix_color(Color32::from_rgb(16, 20, 30), bg_rgb, 0.34, 26),
                        mix_color(Color32::from_rgb(255, 255, 255), bg_rgb, 0.10, 228),
                        mix_color(Color32::from_rgb(16, 20, 30), bg_rgb, 0.30, 38),
                        mix_color(Color32::from_rgb(16, 20, 30), bg_rgb, 0.25, 20),
                        mix_color(Color32::from_rgb(232, 236, 242), bg_rgb, 0.28, 210),
                        mix_color(Color32::from_rgb(16, 20, 30), bg_rgb, 0.30, 22),
                        mix_color(Color32::from_rgb(16, 20, 30), bg_rgb, 0.36, 38),
                    )
                };
            let auto_border = harmonize_auto_border(bg_rgb, dark);
            Self {
                bg: Color32::TRANSPARENT,
                card_bg,
                surface_bg,
                border: auto_border,
                profile_border: auto_border,
                profile_card_bg: card_bg,
                friends_border: auto_border,
                friends_card_bg: card_bg,
                games_border: auto_border,
                text_border: mix_color(
                    auto_border,
                    if dark { [255, 255, 255] } else { [24, 28, 38] },
                    0.22,
                    118,
                ),
                text,
                muted,
                placeholder,
                selected_bg,
                hover_bg,
                extreme_bg,
                widget_inactive,
                widget_hovered,
                glass_edition: false,
                glass_highlight: 0.0,
                corner_radius: radius,
            }
        } else if dark {
            let border = Color32::from_rgba_unmultiplied(255, 255, 255, 42);
            let card_bg = Color32::from_rgba_unmultiplied(16, 19, 27, 214);
            Self {
                bg: Color32::TRANSPARENT,
                card_bg,
                surface_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 28),
                border,
                profile_border: border,
                profile_card_bg: card_bg,
                friends_border: border,
                friends_card_bg: card_bg,
                games_border: border,
                text_border: Color32::from_rgba_unmultiplied(255, 255, 255, 62),
                text,
                muted,
                placeholder: Color32::from_rgba_unmultiplied(255, 255, 255, 22),
                selected_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 42),
                hover_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 22),
                extreme_bg: Color32::from_rgba_unmultiplied(14, 17, 24, 210),
                widget_inactive: Color32::from_rgba_unmultiplied(255, 255, 255, 28),
                widget_hovered: Color32::from_rgba_unmultiplied(255, 255, 255, 46),
                glass_edition: false,
                glass_highlight: 0.0,
                corner_radius: radius,
            }
        } else {
            let border = Color32::from_rgba_unmultiplied(16, 19, 26, 42);
            let card_bg = Color32::from_rgba_unmultiplied(255, 255, 255, 226);
            Self {
                bg: Color32::TRANSPARENT,
                card_bg,
                surface_bg: Color32::from_rgba_unmultiplied(16, 19, 26, 20),
                border,
                profile_border: border,
                profile_card_bg: card_bg,
                friends_border: border,
                friends_card_bg: card_bg,
                games_border: border,
                text_border: Color32::from_rgba_unmultiplied(16, 19, 26, 62),
                text,
                muted,
                placeholder: Color32::from_rgba_unmultiplied(16, 19, 26, 22),
                selected_bg: Color32::from_rgba_unmultiplied(16, 19, 26, 32),
                hover_bg: Color32::from_rgba_unmultiplied(16, 19, 26, 16),
                extreme_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                widget_inactive: Color32::from_rgba_unmultiplied(16, 19, 26, 18),
                widget_hovered: Color32::from_rgba_unmultiplied(16, 19, 26, 34),
                glass_edition: false,
                glass_highlight: 0.0,
                corner_radius: radius,
            }
        };

        if settings.glass_edition {
            let opacity = settings.glass_opacity.clamp(0.20, 0.92);
            let highlight = settings.glass_highlight.clamp(0.0, 1.0);
            let card_alpha = (opacity * 165.0 + 38.0).round().clamp(52.0, 224.0) as u8;
            let surf_alpha = (opacity * 92.0 + 24.0).round().clamp(26.0, 136.0) as u8;
            let rim_alpha = (58.0 + highlight * 118.0).round().clamp(36.0, 200.0) as u8;

            if dark {
                let card_base = if settings.bg_mode == BgMode::SolidColor {
                    mix_color(Color32::from_rgb(20, 22, 28), settings.bg_color, 0.16, card_alpha)
                } else {
                    Color32::from_rgba_unmultiplied(26, 28, 34, card_alpha)
                };
                p.card_bg = card_base;
                p.surface_bg = Color32::from_rgba_unmultiplied(255, 255, 255, surf_alpha);
                p.extreme_bg = Color32::from_rgba_unmultiplied(
                    18,
                    20,
                    25,
                    card_alpha.saturating_add(24).min(242),
                );
                p.selected_bg = Color32::from_rgba_unmultiplied(
                    255,
                    255,
                    255,
                    surf_alpha.saturating_add(26).min(165),
                );
                p.hover_bg = Color32::from_rgba_unmultiplied(
                    255,
                    255,
                    255,
                    surf_alpha.saturating_sub(6).max(20),
                );
                p.placeholder = Color32::from_rgba_unmultiplied(255, 255, 255, 24);
                p.widget_inactive = p.surface_bg;
                p.widget_hovered = Color32::from_rgba_unmultiplied(
                    255,
                    255,
                    255,
                    surf_alpha.saturating_add(28).min(170),
                );
            } else {
                let card_base = if settings.bg_mode == BgMode::SolidColor {
                    mix_color(Color32::from_rgb(255, 255, 255), settings.bg_color, 0.10, card_alpha)
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, card_alpha)
                };
                p.card_bg = card_base;
                p.surface_bg =
                    Color32::from_rgba_unmultiplied(18, 20, 26, (surf_alpha / 2).max(16));
                p.extreme_bg = Color32::from_rgba_unmultiplied(
                    255,
                    255,
                    255,
                    card_alpha.saturating_add(20).min(242),
                );
                p.selected_bg = Color32::from_rgba_unmultiplied(18, 20, 26, 40);
                p.hover_bg = Color32::from_rgba_unmultiplied(18, 20, 26, 22);
                p.placeholder = Color32::from_rgba_unmultiplied(18, 20, 26, 26);
                p.widget_inactive = p.surface_bg;
                p.widget_hovered = Color32::from_rgba_unmultiplied(18, 20, 26, 44);
            }
            let glass_rim = if dark {
                Color32::from_rgba_unmultiplied(255, 255, 255, rim_alpha)
            } else {
                mix_color(
                    Color32::from_rgb(198, 204, 214),
                    [255, 255, 255],
                    highlight * 0.40,
                    rim_alpha.saturating_add(28).min(235),
                )
            };
            p.border = glass_rim;
            p.profile_border = glass_rim;
            p.friends_border = glass_rim;
            p.games_border = glass_rim;
            p.text_border = glass_rim;
            p.profile_card_bg = p.card_bg;
            p.friends_card_bg = p.card_bg;
            p.glass_edition = true;
            p.glass_highlight = highlight;
            p.corner_radius = radius;
        } else {
            p.corner_radius = radius;
        }

        let to_c32 = |[r, g, b]: [u8; 3]| Color32::from_rgb(r, g, b);
        if let Some(rgb) = settings.borders_color {
            let c = to_c32(rgb);
            p.border = c;
            p.profile_border = c;
            p.friends_border = c;
            p.games_border = c;
            p.text_border = c;
        }
        if let Some(rgb) = settings.profile_color {
            p.profile_border = to_c32(rgb);
            p.profile_card_bg = tint_card_surface(p.card_bg, rgb, dark);
        }
        if let Some(rgb) = settings.friends_color {
            p.friends_border = to_c32(rgb);
            p.friends_card_bg = tint_card_surface(p.card_bg, rgb, dark);
        }
        if let Some(rgb) = settings.games_border_color {
            p.games_border = to_c32(rgb);
        }
        if let Some(rgb) = settings.text_border_color {
            p.text_border = to_c32(rgb);
        }

        p
    }
}

pub fn pal(ui: &egui::Ui) -> Palette {
    ui.ctx()
        .data(|d| d.get_temp::<Palette>(egui::Id::NULL.with(THEME_PALETTE_KEY)))
        .unwrap_or_else(|| Palette::of(ui.visuals().dark_mode))
}

pub const GREEN: Color32 = Color32::from_rgb(0, 176, 111);
pub const BLUE: Color32 = Color32::from_rgb(0, 162, 255);
pub const ORANGE: Color32 = Color32::from_rgb(246, 136, 2);
pub const RED: Color32 = Color32::from_rgb(210, 50, 50);

pub fn compact(n: u64) -> String {
    let (v, s) = if n >= 1_000_000 {
        (n as f64 / 1_000_000.0, "M")
    } else if n >= 1_000 {
        (n as f64 / 1_000.0, "K")
    } else {
        return n.to_string();
    };
    let t = format!("{v:.1}");
    format!("{}{s}", t.trim_end_matches(".0"))
}

pub fn crisp_font_tweak() -> egui::FontTweak {
    egui::FontTweak {
        hinting: Some(true),
        hinting_target: egui::epaint::text::HintingTarget::Smooth(
            egui::epaint::text::SmoothHinting {
                light: false,
                symmetric_rendering: false,
                preserve_linear_metrics: false,
            },
        ),
        subpixel_binning: Some(false),
        ..Default::default()
    }
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let tweak = crisp_font_tweak();

    if let Ok(bytes) = fs::read(r"C:\Windows\Fonts\segoeui.ttf")
        .or_else(|_| fs::read(r"C:\Windows\Fonts\arial.ttf"))
    {
        let mut data = egui::FontData::from_owned(bytes);
        data.tweak = tweak.clone();
        fonts.font_data.insert("segoe_ui".to_owned(), Arc::new(data));
        if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            prop.insert(0, "segoe_ui".to_owned());
        }
    }

    if let Ok(bytes) = fs::read(r"C:\Windows\Fonts\seguisym.ttf") {
        let mut data = egui::FontData::from_owned(bytes);
        data.tweak = tweak.clone();
        fonts.font_data.insert("segoe_sym".to_owned(), Arc::new(data));
        if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            let pos = usize::from(prop.first().is_some_and(|s| s == "segoe_ui"));
            prop.insert(pos, "segoe_sym".to_owned());
        }
    }

    let bold_bytes = fs::read(r"C:\Windows\Fonts\segoeuib.ttf")
        .or_else(|_| fs::read(r"C:\Windows\Fonts\arialbd.ttf"));
    let mut bold_chain = Vec::new();
    if let Ok(bytes) = bold_bytes {
        let mut data = egui::FontData::from_owned(bytes);
        data.tweak = tweak.clone();
        fonts.font_data.insert("segoe_bold".to_owned(), Arc::new(data));
        bold_chain.push("segoe_bold".to_owned());
    }
    if fonts.font_data.contains_key("segoe_sym") {
        bold_chain.push("segoe_sym".to_owned());
    }
    if let Some(prop) = fonts.families.get(&egui::FontFamily::Proportional) {
        bold_chain.extend(prop.iter().cloned());
    }
    fonts
        .families
        .insert(egui::FontFamily::Name("Bold".into()), bold_chain);

    let logo_bytes = fs::read(r"C:\Windows\Fonts\seguibl.ttf")
        .or_else(|_| fs::read(r"C:\Windows\Fonts\ariblk.ttf"))
        .or_else(|_| fs::read(r"C:\Windows\Fonts\segoeuib.ttf"));
    let mut logo_chain = Vec::new();
    if let Ok(bytes) = logo_bytes {
        let mut data = egui::FontData::from_owned(bytes);
        data.tweak = tweak;
        fonts.font_data.insert("logo_black".to_owned(), Arc::new(data));
        logo_chain.push("logo_black".to_owned());
    }
    if let Some(prop) = fonts.families.get(&egui::FontFamily::Proportional) {
        logo_chain.extend(prop.iter().cloned());
    }
    fonts
        .families
        .insert(egui::FontFamily::Name("LogoBlack".into()), logo_chain);

    ctx.set_fonts(fonts);
}

pub fn brand_title(ui: &mut egui::Ui, size: f32, clickable: bool) -> egui::Response {
    let mut job = egui::text::LayoutJob::default();
    let font_id = egui::FontId::new(size, egui::FontFamily::Name("LogoBlack".into()));
    let rust_red = Color32::from_rgb(220, 34, 34);
    let blox_color = if ui.visuals().dark_mode {
        Color32::WHITE
    } else {
        Color32::BLACK
    };
    job.append(
        "RUST",
        0.0,
        egui::TextFormat {
            font_id: font_id.clone(),
            color: rust_red,
            ..Default::default()
        },
    );
    job.append(
        "BLOX",
        0.0,
        egui::TextFormat {
            font_id,
            color: blox_color,
            ..Default::default()
        },
    );
    let r = ui.add(
        egui::Label::new(job).sense(if clickable { Sense::click() } else { Sense::hover() }),
    );
    if clickable {
        r.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        r
    }
}

fn relative_luminance(rgb: [u8; 3]) -> f32 {
    let ch = |c: u8| {
        let s = c as f32 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(rgb[0]) + 0.7152 * ch(rgb[1]) + 0.0722 * ch(rgb[2])
}

pub fn is_dark_color(rgb: [u8; 3]) -> bool {
    let lum = relative_luminance(rgb);
    // Warm/rose hues stay cleaner with crisp white text up to slightly higher luminance
    if rgb[0] > rgb[1].saturating_add(35) {
        lum < 0.28
    } else {
        lum < 0.20
    }
}

pub fn effective_dark_mode(settings: &Settings) -> bool {
    settings.dark
}

pub fn effective_text_color(settings: &Settings) -> Color32 {
    let dark = effective_dark_mode(settings);
    let custom_bg = settings.bg_mode != BgMode::ThemeDefault;
    match settings.text_mode {
        TextColorMode::Green => {
            if dark {
                Color32::from_rgb(126, 248, 172)
            } else {
                Color32::from_rgb(12, 118, 68)
            }
        }
        TextColorMode::Auto | TextColorMode::White | TextColorMode::Dark => {
            if custom_bg {
                if dark {
                    Color32::from_rgb(252, 253, 255)
                } else {
                    Color32::from_rgb(16, 19, 26)
                }
            } else {
                Palette::of(dark).text
            }
        }
    }
}

pub fn panel_chrome_fill(ui: &egui::Ui, settings: &Settings) -> Color32 {
    let dark = effective_dark_mode(settings);
    if settings.glass_edition {
        let alpha = (settings.glass_opacity.clamp(0.20, 0.92) * 145.0 + 36.0)
            .round()
            .clamp(44.0, 212.0) as u8;
        return match settings.bg_mode {
            BgMode::SolidColor => {
                if dark {
                    mix_color(Color32::from_rgb(18, 20, 25), settings.bg_color, 0.16, alpha)
                } else {
                    mix_color(Color32::from_rgb(255, 255, 255), settings.bg_color, 0.10, alpha)
                }
            }
            BgMode::ThemeDefault | BgMode::Wallpaper => {
                if dark {
                    Color32::from_rgba_unmultiplied(20, 22, 27, alpha)
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, alpha)
                }
            }
        };
    }
    match settings.bg_mode {
        BgMode::ThemeDefault => pal(ui).bg,
        BgMode::SolidColor => {
            if dark {
                mix_color(Color32::from_rgb(13, 15, 21), settings.bg_color, 0.16, 200)
            } else {
                mix_color(Color32::from_rgb(255, 255, 255), settings.bg_color, 0.12, 214)
            }
        }
        BgMode::Wallpaper => {
            if dark {
                Color32::from_rgba_unmultiplied(15, 18, 24, 194)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 206)
            }
        }
    }
}

pub fn paint_glass_sheen(
    painter: &egui::Painter,
    rect: egui::Rect,
    radius: u8,
    p: Palette,
) {
    if !p.glass_edition || p.glass_highlight <= 0.01 || rect.width() < 12.0 || rect.height() < 10.0 {
        return;
    }
    let h = (rect.height() * 0.28).clamp(8.0, 26.0);
    let band = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.top() + h));
    let top_alpha = (18.0 * p.glass_highlight).round().clamp(4.0, 26.0) as u8;
    painter.rect_filled(
        band,
        CornerRadius {
            nw: radius,
            ne: radius,
            sw: 0,
            se: 0,
        },
        Color32::from_rgba_unmultiplied(255, 255, 255, top_alpha),
    );
    let inset = (radius as f32 * 0.45).clamp(4.0, rect.width() * 0.25);
    let rim_alpha = (55.0 + 95.0 * p.glass_highlight).round().clamp(30.0, 170.0) as u8;
    painter.line_segment(
        [
            egui::pos2(rect.left() + inset, rect.top() + 0.8),
            egui::pos2(rect.right() - inset, rect.top() + 0.8),
        ],
        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, rim_alpha)),
    );
}

fn rgb_to_hsv(rgb: [u8; 3]) -> (f32, f32, f32) {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d < 1e-5 {
        0.0
    } else if (max - r).abs() < 1e-5 {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if (max - g).abs() < 1e-5 {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max < 1e-5 { 0.0 } else { d / max };
    (h, s, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color32 {
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = match (h / 60.0).floor() as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color32::from_rgb(
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

pub fn theme_accent(settings: &Settings) -> Color32 {
    let dark = effective_dark_mode(settings);
    if settings.bg_mode == BgMode::SolidColor {
        let (h, s, _) = rgb_to_hsv(settings.bg_color);
        if s >= 0.14 {
            return hsv_to_rgb(h, s.clamp(0.68, 0.90), if dark { 0.88 } else { 0.68 });
        }
    }
    if settings.text_mode == TextColorMode::Green {
        GREEN
    } else if dark {
        GREEN
    } else {
        BLUE
    }
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    mix_color(a, [b.r(), b.g(), b.b()], t, a.a())
}

pub fn accent_color(ui: &egui::Ui) -> Color32 {
    ui.visuals().selection.stroke.color
}

pub fn apply_theme(ctx: &egui::Context, settings: &Settings) {
    let dark = effective_dark_mode(settings);
    let custom_bg = settings.bg_mode != BgMode::ThemeDefault;
    let p = Palette::from_settings(settings);
    let text_color = p.text;
    ctx.data_mut(|d| d.insert_temp(egui::Id::NULL.with(THEME_PALETTE_KEY), p));
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = if custom_bg {
        Color32::TRANSPARENT
    } else {
        p.bg
    };
    v.window_fill = if settings.glass_edition {
        Color32::from_rgba_unmultiplied(
            p.card_bg.r(),
            p.card_bg.g(),
            p.card_bg.b(),
            p.card_bg.a().saturating_add(24).min(240),
        )
    } else if custom_bg {
        if dark {
            mix_color(Color32::from_rgb(18, 22, 30), settings.bg_color, 0.14, 244)
        } else {
            mix_color(Color32::from_rgb(252, 253, 255), settings.bg_color, 0.08, 246)
        }
    } else {
        p.bg
    };
    v.extreme_bg_color = p.extreme_bg;
    v.faint_bg_color = p.card_bg;
    v.override_text_color = Some(text_color);
    v.text_options.font_hinting = true;
    v.text_options.subpixel_binning = false;
    v.window_corner_radius = CornerRadius::same(p.corner_radius);
    v.window_stroke = egui::Stroke::new(1.0, p.border);
    if settings.glass_edition {
        v.window_shadow = egui::Shadow {
            offset: [0, 8],
            blur: 22,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 86 } else { 26 }),
        };
    }

    let r8 = CornerRadius::same(p.corner_radius.saturating_sub(4).clamp(6, 16));
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.noninteractive.corner_radius = r8;
    v.widgets.noninteractive.fg_stroke.color = text_color;
    v.widgets.inactive.weak_bg_fill = p.widget_inactive;
    v.widgets.inactive.bg_fill = p.widget_inactive;
    v.widgets.inactive.bg_stroke = if settings.text_border_color.is_some() || settings.glass_edition {
        egui::Stroke::new(1.0, p.text_border)
    } else {
        egui::Stroke::NONE
    };
    v.widgets.inactive.corner_radius = r8;
    v.widgets.inactive.fg_stroke.color = text_color;
    v.widgets.hovered.weak_bg_fill = p.widget_hovered;
    v.widgets.hovered.bg_fill = p.widget_hovered;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.hovered.corner_radius = r8;
    v.widgets.hovered.fg_stroke.color = text_color;
    v.widgets.active.weak_bg_fill = p.selected_bg;
    v.widgets.active.bg_fill = p.selected_bg;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.2, p.text_border);
    v.widgets.active.corner_radius = r8;
    v.widgets.active.fg_stroke.color = text_color;
    v.widgets.open.corner_radius = r8;
    let accent = theme_accent(settings);
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 55);
    v.selection.stroke = egui::Stroke::new(1.0, accent);

    let theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(theme);
    ctx.style_mut_of(theme, |s| {
        s.visuals = v;
        s.animation_time = 0.16;
        s.spacing.button_padding = egui::vec2(14.0, 7.0);
        s.spacing.item_spacing = egui::vec2(10.0, 8.0);
        s.spacing.scroll = egui::style::ScrollStyle::floating();
    });
}

pub fn card_frame_colored(margin: i8, fill: Color32, border: Color32) -> egui::Frame {
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(12)
        .inner_margin(margin)
}

pub fn card_frame(ui: &egui::Ui, margin: i8) -> egui::Frame {
    let p = pal(ui);
    let mut frame = egui::Frame::new()
        .fill(p.card_bg)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(p.corner_radius)
        .inner_margin(margin);
    if p.glass_edition {
        let dark = ui.visuals().dark_mode;
        frame = frame.shadow(egui::Shadow {
            offset: [0, 5],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 58 } else { 20 }),
        });
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::{is_dark_color, theme_accent, Palette, BLUE, GREEN};
    use crate::types::{BgMode, Settings};
    use eframe::egui::Color32;

    #[test]
    fn detects_rose_and_light_dark_contrast_correctly() {
        // Deep rose & vibrant rose should trigger light/white text
        assert!(is_dark_color([184, 50, 95]));
        assert!(is_dark_color([222, 74, 122]));
        // Soft pastel rose & warm sand should trigger dark text
        assert!(!is_dark_color([248, 212, 226]));
        assert!(!is_dark_color([245, 239, 230]));
        // Deep navy should trigger light text
        assert!(is_dark_color([16, 30, 48]));
    }

    #[test]
    fn theme_accent_tracks_theme_mode_and_color() {
        let mut s = Settings::default();
        s.dark = true;
        s.bg_mode = BgMode::ThemeDefault;
        assert_eq!(theme_accent(&s), GREEN);

        s.dark = false;
        assert_eq!(theme_accent(&s), BLUE);

        s.bg_mode = BgMode::SolidColor;
        s.bg_color = [184, 50, 95]; // rose theme -> red-dominant accent
        let rose_accent = theme_accent(&s);
        assert!(rose_accent.r() > rose_accent.g() && rose_accent.r() > rose_accent.b());
    }

    #[test]
    fn custom_borders_and_harmonized_background_apply_cleanly() {
        let mut s = Settings::default();
        s.bg_mode = BgMode::SolidColor;
        s.bg_color = [184, 50, 95];
        s.borders_color = Some([240, 210, 220]);
        s.profile_color = Some([255, 130, 175]);
        s.friends_color = Some([110, 220, 170]);
        s.games_border_color = Some([120, 185, 255]);
        s.text_border_color = Some([255, 235, 130]);

        let p = Palette::from_settings(&s);
        assert_eq!(p.border, Color32::from_rgb(240, 210, 220));
        assert_eq!(p.profile_border, Color32::from_rgb(255, 130, 175));
        assert_eq!(p.friends_border, Color32::from_rgb(110, 220, 170));
        assert_eq!(p.games_border, Color32::from_rgb(120, 185, 255));
        assert_eq!(p.text_border, Color32::from_rgb(255, 235, 130));
        // Card surface should pick up warm rose harmony
        assert!(p.card_bg.r() > p.card_bg.g());
    }

    #[test]
    fn glass_liquid_edition_applies_neutral_glass_and_roundness() {
        let mut s = Settings::default();
        s.dark = true;
        s.glass_edition = true;
        s.glass_opacity = 0.45;
        s.glass_highlight = 0.85;
        s.corner_roundness = 18;

        let p = Palette::from_settings(&s);
        assert!(p.glass_edition);
        // Keeps the normal background color untouched while making UI cards/surfaces frosted liquid glass.
        assert_eq!(p.bg, Palette::of(true).bg);
        assert_eq!(p.corner_radius, 18);
        assert!(p.card_bg.a() < 200);
        assert!(p.surface_bg.a() < 120);
        assert!(p.border.a() > 100);
    }
}
