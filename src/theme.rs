use std::fs;
use std::sync::Arc;
use eframe::egui::{self, Color32, CornerRadius, Sense};

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color32,
    pub card_bg: Color32,
    pub surface_bg: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub placeholder: Color32,
    pub selected_bg: Color32,
    pub hover_bg: Color32,
    pub extreme_bg: Color32,
    pub widget_inactive: Color32,
    pub widget_hovered: Color32,
}

impl Palette {
    pub fn of(dark: bool) -> Self {
        if dark {
            Self {
                bg: Color32::from_rgb(17, 19, 24),
                card_bg: Color32::from_rgb(25, 28, 35),
                surface_bg: Color32::from_rgb(33, 37, 46),
                border: Color32::from_rgba_unmultiplied(255, 255, 255, 14),
                text: Color32::from_rgb(240, 242, 245),
                muted: Color32::from_rgb(156, 163, 175),
                placeholder: Color32::from_rgb(38, 42, 51),
                selected_bg: Color32::from_rgb(44, 49, 60),
                hover_bg: Color32::from_rgb(31, 35, 43),
                extreme_bg: Color32::from_rgb(26, 29, 36),
                widget_inactive: Color32::from_rgb(40, 45, 55),
                widget_hovered: Color32::from_rgb(56, 62, 75),
            }
        } else {
            Self {
                bg: Color32::from_rgb(255, 255, 255),
                card_bg: Color32::from_rgb(248, 249, 250),
                surface_bg: Color32::from_rgb(241, 243, 245),
                border: Color32::from_rgb(225, 228, 232),
                text: Color32::from_rgb(25, 25, 25),
                muted: Color32::from_rgb(96, 97, 98),
                placeholder: Color32::from_rgb(233, 236, 239),
                selected_bg: Color32::from_rgb(232, 235, 239),
                hover_bg: Color32::from_rgb(243, 245, 247),
                extreme_bg: Color32::from_rgb(241, 243, 245),
                widget_inactive: Color32::from_rgb(235, 238, 241),
                widget_hovered: Color32::from_rgb(222, 226, 230),
            }
        }
    }
}

pub fn pal(ui: &egui::Ui) -> Palette {
    Palette::of(ui.visuals().dark_mode)
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

pub fn apply_theme(ctx: &egui::Context, dark: bool) {
    let p = Palette::of(dark);
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.bg;
    v.window_fill = p.bg;
    v.extreme_bg_color = p.extreme_bg;
    v.faint_bg_color = p.card_bg;
    v.override_text_color = Some(p.text);
    v.text_options.font_hinting = true;
    v.text_options.subpixel_binning = false;
    v.window_corner_radius = CornerRadius::same(12);
    v.window_stroke = egui::Stroke::new(1.0, p.border);

    let r8 = CornerRadius::same(8);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.noninteractive.corner_radius = r8;
    v.widgets.inactive.weak_bg_fill = p.widget_inactive;
    v.widgets.inactive.bg_fill = p.widget_inactive;
    v.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    v.widgets.inactive.corner_radius = r8;
    v.widgets.hovered.weak_bg_fill = p.widget_hovered;
    v.widgets.hovered.bg_fill = p.widget_hovered;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.hovered.corner_radius = r8;
    v.widgets.active.weak_bg_fill = p.selected_bg;
    v.widgets.active.bg_fill = p.selected_bg;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.active.corner_radius = r8;
    v.widgets.open.corner_radius = r8;
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0, 176, 111, 55);
    v.selection.stroke = egui::Stroke::new(1.0, GREEN);

    let theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(theme);
    ctx.style_mut_of(theme, |s| {
        s.visuals = v;
        s.spacing.button_padding = egui::vec2(14.0, 7.0);
        s.spacing.item_spacing = egui::vec2(10.0, 8.0);
        s.spacing.scroll = egui::style::ScrollStyle::floating();
    });
}

pub fn card_frame(ui: &egui::Ui, margin: i8) -> egui::Frame {
    let p = pal(ui);
    egui::Frame::new()
        .fill(p.card_bg)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(12)
        .inner_margin(margin)
}
