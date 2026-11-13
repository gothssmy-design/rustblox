use std::collections::HashMap;
use eframe::egui::{self, Color32, CornerRadius, Sense, Vec2, RichText};
use crate::theme::{compact, pal, BLUE, GREEN, ORANGE};
use crate::types::{Action, CardPreview, Game};

pub fn avatar_glass_frame(ui: &egui::Ui, pad: i8) -> egui::Frame {
    let p = pal(ui);
    egui::Frame::new()
        .fill(p.card_bg)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(14)
        .inner_margin(pad)
}

pub fn thumb_styled_with_video(
    ui: &mut egui::Ui,
    url: Option<&String>,
    video_tex: Option<&egui::TextureHandle>,
    is_video_loading: bool,
    size: Vec2,
    radius: u8,
    bordered: bool,
) -> egui::Response {
    let p = pal(ui);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let r = CornerRadius::same(radius);
        ui.painter().rect_filled(rect, r, p.placeholder);
        if let Some(tex) = video_tex {
            egui::Image::new(tex)
                .corner_radius(radius)
                .paint_at(ui, rect);

            let badge_w = 68.0;
            let badge_h = 18.0;
            let badge_rect = egui::Rect::from_min_size(
                egui::pos2(rect.max.x - badge_w - 6.0, rect.max.y - badge_h - 6.0),
                egui::vec2(badge_w, badge_h),
            );
            ui.painter().rect_filled(
                badge_rect,
                CornerRadius::same(5),
                Color32::from_black_alpha(190),
            );
            let icon_rect = egui::Rect::from_center_size(
                badge_rect.center() - egui::vec2(25.0, 0.0),
                egui::vec2(10.0, 10.0),
            );
            egui::Image::new(egui::include_image!("../../assets/icon_play.png"))
                .tint(Color32::WHITE)
                .paint_at(ui, icon_rect);
            ui.painter().text(
                badge_rect.center() + egui::vec2(6.0, 0.0),
                egui::Align2::CENTER_CENTER,
                "PREVIEW",
                egui::FontId::new(10.0, egui::FontFamily::Name("Bold".into())),
                Color32::WHITE,
            );
        } else {
            if let Some(u) = url.filter(|u| !u.is_empty()) {
                egui::Image::new(u.as_str())
                    .corner_radius(radius)
                    .paint_at(ui, rect);
            }
            if is_video_loading {
                let badge_w = 64.0;
                let badge_h = 18.0;
                let badge_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.max.x - badge_w - 6.0, rect.max.y - badge_h - 6.0),
                    egui::vec2(badge_w, badge_h),
                );
                ui.painter().rect_filled(
                    badge_rect,
                    CornerRadius::same(5),
                    Color32::from_black_alpha(175),
                );
                ui.painter().text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Loading...",
                    egui::FontId::new(9.5, egui::FontFamily::Proportional),
                    Color32::from_rgb(225, 225, 225),
                );
            }
        }
        if bordered {
            ui.painter().rect_stroke(
                rect,
                r,
                egui::Stroke::new(1.0, p.border),
                egui::StrokeKind::Inside,
            );
        }
        if resp.hovered() && video_tex.is_none() {
            ui.painter()
                .rect_filled(rect, r, Color32::from_black_alpha(24));
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn thumb_styled(
    ui: &mut egui::Ui,
    url: Option<&String>,
    size: Vec2,
    radius: u8,
    bordered: bool,
) -> egui::Response {
    thumb_styled_with_video(ui, url, None, false, size, radius, bordered)
}

pub fn thumb(ui: &mut egui::Ui, url: Option<&String>, size: Vec2, radius: u8) -> egui::Response {
    thumb_styled(ui, url, size, radius, false)
}

pub fn small_text(ui: &mut egui::Ui, text: String) {
    ui.add(egui::Label::new(RichText::new(text).size(12.0).color(pal(ui).muted)).truncate());
}

pub fn title_text(ui: &mut egui::Ui, text: &str, size: f32) -> egui::Response {
    let p = pal(ui);
    let font_id = egui::FontId::new(size, egui::FontFamily::Name("Bold".into()));
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .font(font_id)
                .color(p.text)
                .strong(),
        )
        .truncate()
        .sense(Sense::click()),
    )
    .on_hover_text(text)
    .on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn game_stats_row(
    ui: &mut egui::Ui,
    g: &Game,
    icon_size: f32,
    text_size: f32,
    compact_stats: bool,
) {
    let p = pal(ui);
    let custom_bg = ui.visuals().panel_fill == Color32::TRANSPARENT;
    let dark = ui.visuals().dark_mode;
    let (stat_color, sub_color) = if custom_bg {
        (p.text, p.muted)
    } else if dark {
        (
            Color32::from_rgb(226, 231, 240),
            Color32::from_rgb(190, 198, 210),
        )
    } else {
        (
            Color32::from_rgb(45, 50, 58),
            Color32::from_rgb(80, 84, 92),
        )
    };

    ui.horizontal(|ui| {
        ui.set_min_height(20.0);
        ui.spacing_mut().item_spacing.x = 5.0;
        ui.add(
            egui::Image::new(egui::include_image!("../../assets/icon_like.png"))
                .fit_to_exact_size(egui::vec2(icon_size, icon_size))
                .tint(stat_color),
        );
        ui.label(
            RichText::new(g.rating())
                .size(text_size)
                .color(stat_color)
                .strong(),
        );
        let tail = if g.playing > 0 {
            if compact_stats {
                format!("•  {}", compact(g.playing))
            } else {
                format!("•  {} playing", compact(g.playing))
            }
        } else if !compact_stats {
            "•  Rating".to_string()
        } else {
            String::new()
        };
        if !tail.is_empty() {
            ui.add(
                egui::Label::new(RichText::new(tail).size(text_size).color(sub_color)).truncate(),
            );
        }
    });
}

#[allow(dead_code)]
pub fn verified_badge(ui: &mut egui::Ui, size: f32) -> egui::Response {
    verified_badge_tip(ui, size, "Verified")
}

pub fn verified_badge_tip(ui: &mut egui::Ui, size: f32, tip: &str) -> egui::Response {
    let img = egui::Image::new(egui::include_image!("../../assets/icon_verified.png"))
        .fit_to_exact_size(egui::vec2(size, size));
    ui.add(img).on_hover_text(tip)
}

pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(16.0);
    ui.label(RichText::new(text).size(19.0).strong());
    ui.add_space(6.0);
}

pub fn section_link(ui: &mut egui::Ui, text: &str) -> bool {
    let p = pal(ui);
    ui.add_space(16.0);
    let resp = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let l = ui.add(
                egui::Label::new(RichText::new(text).size(19.0).strong().color(p.text))
                    .sense(Sense::click()),
            );
            let arrow = egui::Image::new(egui::include_image!("../../assets/icon_arrow_right.png"))
                .fit_to_exact_size(egui::vec2(13.0, 13.0))
                .tint(p.muted);
            let a = ui.add(arrow.sense(Sense::click()));
            let clicked = l.clicked() || a.clicked();
            if l.hovered() || a.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            clicked
        })
        .inner;
    ui.add_space(6.0);
    resp
}

pub fn sync_refresh_control(
    ui: &mut egui::Ui,
    syncing: bool,
    tip: &str,
    acts: &mut Vec<Action>,
) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let p = pal(ui);
        let btn = egui::Button::new(RichText::new("Refresh").size(13.0).color(p.text))
            .fill(p.surface_bg)
            .stroke(egui::Stroke::new(1.0, p.border))
            .corner_radius(16);
        if ui
            .add_enabled(!syncing, btn)
            .on_hover_text(tip)
            .clicked()
        {
            acts.push(Action::RefreshGames);
        }
        if syncing {
            ui.add(egui::Spinner::new().size(13.0));
        }
    });
}



#[allow(clippy::too_many_arguments)]
pub fn game_card(
    ui: &mut egui::Ui,
    g: &Game,
    thumb_url: Option<&String>,
    preview: Option<CardPreview<'_>>,
    thumb_size: Vec2,
    title_size: f32,
    badge_size: f32,
    stat_icon_size: f32,
    stat_text_size: f32,
    compact_stats: bool,
    acts: &mut Vec<Action>,
) {
    let w = thumb_size.x;
    let card_h = thumb_size.y + 58.0;
    let card_resp = ui.allocate_ui_with_layout(
        egui::vec2(w, card_h),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(w);
            ui.set_max_width(w);
            // Reset item spacing locally so card height stays deterministic regardless of grid row spacing
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);

            let (video_tex, is_loading) = match preview.filter(|p| p.universe_id == g.universe_id) {
                Some(p) => (p.texture, p.is_loading),
                None => (None, false),
            };

            let a = thumb_styled_with_video(ui, thumb_url, video_tex, is_loading, thumb_size, 8, false);
            let gp = pal(ui);
            ui.painter().rect_stroke(
                a.rect,
                CornerRadius::same(8),
                egui::Stroke::new(1.2, gp.games_border),
                egui::StrokeKind::Inside,
            );
            ui.add_space(3.0);

            let max_title_w = if g.creator_verified {
                (w - badge_size - 6.0).max(40.0)
            } else {
                w
            };
            let b = ui
                .horizontal(|ui| {
                    ui.set_min_height(24.0);
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let t = ui
                        .scope(|ui| {
                            ui.set_max_width(max_title_w);
                            title_text(ui, &g.name, title_size)
                        })
                        .inner;
                    if g.creator_verified {
                        verified_badge_tip(ui, badge_size, "Verified Creator");
                    }
                    t
                })
                .inner;

            game_stats_row(ui, g, stat_icon_size, stat_text_size, compact_stats);

            (a.hovered() || b.hovered(), a.clicked() || b.clicked())
        },
    );

    let card_rect = card_resp.response.rect;
    let pointer_in_card = ui.rect_contains_pointer(card_rect);
    let is_hovered = pointer_in_card || card_resp.inner.0 || card_resp.response.hovered();
    let is_clicked = card_resp.inner.1 || (pointer_in_card && ui.input(|i| i.pointer.primary_clicked()));

    if is_hovered {
        acts.push(Action::HoverGame(g.universe_id));
    }
    if is_clicked {
        acts.push(Action::Open(g.clone()));
    }
}

pub fn wide_card(
    ui: &mut egui::Ui,
    g: &Game,
    wide: &HashMap<u64, String>,
    preview: Option<CardPreview<'_>>,
    w: f32,
    acts: &mut Vec<Action>,
) {
    let h = (w * 9.0 / 16.0).floor();
    game_card(
        ui,
        g,
        wide.get(&g.universe_id),
        preview,
        egui::vec2(w, h),
        15.5,
        14.0,
        16.0,
        13.5,
        false,
        acts,
    );
}

pub fn icon_card(
    ui: &mut egui::Ui,
    g: &Game,
    icons: &HashMap<u64, String>,
    preview: Option<CardPreview<'_>>,
    w: f32,
    acts: &mut Vec<Action>,
) {
    game_card(
        ui,
        g,
        icons.get(&g.universe_id),
        preview,
        egui::vec2(w, w),
        14.5,
        13.0,
        14.5,
        12.5,
        true,
        acts,
    );
}

pub fn grid_card_width(ui: &egui::Ui) -> f32 {
    ((ui.available_width() - 5.0 * 14.0) / 6.0).clamp(150.0, 280.0).floor()
}

pub fn wide_grid_with_width(
    ui: &mut egui::Ui,
    games: &[Game],
    wide: &HashMap<u64, String>,
    preview: Option<CardPreview<'_>>,
    w: f32,
    acts: &mut Vec<Action>,
) {
    let gap = 14.0;
    let avail = ui.available_width();
    let cols = (((avail + gap + 0.5) / (w + gap)).floor() as usize).max(1);
    for (idx, row) in games.chunks(cols).enumerate() {
        if idx > 0 {
            ui.add_space(8.0);
        }
        // horizontal_top (Align::Min) keeps every card aligned cleanly along its top edge
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for g in row {
                wide_card(ui, g, wide, preview, w, acts);
            }
        });
    }
}

pub fn wide_grid(
    ui: &mut egui::Ui,
    games: &[Game],
    wide: &HashMap<u64, String>,
    preview: Option<CardPreview<'_>>,
    acts: &mut Vec<Action>,
) {
    let w = grid_card_width(ui);
    wide_grid_with_width(ui, games, wide, preview, w, acts);
}

pub fn wide_row(
    ui: &mut egui::Ui,
    id: &str,
    games: &[Game],
    wide: &HashMap<u64, String>,
    preview: Option<CardPreview<'_>>,
    acts: &mut Vec<Action>,
) {
    let w = grid_card_width(ui);
    egui::ScrollArea::horizontal()
        .id_salt(id)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                for g in games {
                    wide_card(ui, g, wide, preview, w, acts);
                }
            });
        });
}

pub fn icon_row(
    ui: &mut egui::Ui,
    id: &str,
    games: &[Game],
    icons: &HashMap<u64, String>,
    preview: Option<CardPreview<'_>>,
    acts: &mut Vec<Action>,
) {
    egui::ScrollArea::horizontal()
        .id_salt(id)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                for g in games {
                    icon_card(ui, g, icons, preview, 128.0, acts);
                }
            });
        });
}

// Round avatar with optional status dot and sub-label under it
pub fn bubble(
    ui: &mut egui::Ui,
    url: Option<&String>,
    label: &str,
    sub: &str,
    presence: u8,
    plus: bool,
    verified: bool,
) -> egui::Response {
    let p = pal(ui);
    let mut resp = None;
    ui.allocate_ui_with_layout(
        egui::vec2(108.0, 148.0),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.set_width(108.0);
            ui.spacing_mut().item_spacing.y = 3.0;
            let r = thumb_styled(ui, url, egui::vec2(96.0, 96.0), 48, false);
            if plus {
                ui.painter()
                    .circle_filled(r.rect.center(), 46.5, p.friends_card_bg);
            }
            ui.painter().rect_stroke(
                r.rect,
                CornerRadius::same(48),
                egui::Stroke::new(1.5, p.friends_border),
                egui::StrokeKind::Inside,
            );
            if plus {
                let c = r.rect.center();
                let st = egui::Stroke::new(2.8, GREEN);
                ui.painter().line_segment([c - egui::vec2(13.0, 0.0), c + egui::vec2(13.0, 0.0)], st);
                ui.painter().line_segment([c - egui::vec2(0.0, 13.0), c + egui::vec2(0.0, 13.0)], st);
            } else if presence > 0 {
                let dot_color = match presence {
                    2 => GREEN,
                    3 => ORANGE,
                    _ => BLUE,
                };
                let center = r.rect.right_bottom() - egui::vec2(12.0, 12.0);
                ui.painter().circle_filled(center, 11.0, p.bg);
                ui.painter().circle_filled(center, 8.0, dot_color);
            }
            ui.add_space(4.0);
            let display_label = if label.is_empty() { "Player" } else { label };
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.add(egui::Label::new(RichText::new(display_label).size(13.0).strong()).truncate());
                if verified {
                    verified_badge_tip(ui, 12.0, "Verified");
                }
            });
            if !sub.is_empty() {
                ui.add(
                    egui::Label::new(RichText::new(sub).size(11.5).color(p.muted)).truncate(),
                );
            }
            resp = Some(r);
        },
    );
    resp.unwrap()
}

pub fn draw_robux_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    size: f32,
    color: Color32,
) {
    let r = size * 0.43;
    let mut pts = Vec::with_capacity(6);
    for i in 0..6 {
        let angle = -std::f32::consts::FRAC_PI_2 + (i as f32) * std::f32::consts::FRAC_PI_3;
        pts.push(egui::pos2(
            center.x + r * angle.cos(),
            center.y + r * angle.sin(),
        ));
    }
    let stroke_w = (size * 0.13).clamp(1.3, 2.4);
    painter.add(egui::Shape::closed_line(pts, egui::Stroke::new(stroke_w, color)));
    let inner_half = (size * 0.145).max(1.8);
    let inner = egui::Rect::from_center_size(center, egui::vec2(inner_half * 2.0, inner_half * 2.0));
    painter.rect_filled(inner, (size * 0.04).max(0.5), color);
}


