use std::f32::consts::TAU;

use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::app::RustBlox;
use crate::theme::{brand_title, effective_dark_mode, pal, GREEN, RED};
use crate::types::Action;
use crate::ui::widgets::{thumb_styled, verified_badge_tip};

const RR_ORANGE: Color32 = Color32::from_rgb(255, 107, 43);
const RR_SKY: Color32 = Color32::from_rgb(56, 189, 248);

fn darken(c: Color32, factor: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (c.r() as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (c.g() as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (c.b() as f32 * factor).round().clamp(0.0, 255.0) as u8,
        c.a(),
    )
}

fn lighten(c: Color32, amount: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (c.r() as f32 + (255.0 - c.r() as f32) * amount).round().clamp(0.0, 255.0) as u8,
        (c.g() as f32 + (255.0 - c.g() as f32) * amount).round().clamp(0.0, 255.0) as u8,
        (c.b() as f32 + (255.0 - c.b() as f32) * amount).round().clamp(0.0, 255.0) as u8,
        c.a(),
    )
}

/// Tactile Rec Room-style chunky 3D button with a darker bottom lip that compresses on click.
fn rr_button(
    ui: &mut egui::Ui,
    label: &str,
    fill: Color32,
    text_color: Color32,
    size: Vec2,
    font_size: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let pressed = resp.is_pointer_button_down_on();
        let press_offset = if pressed { 3.0 } else if hovered { -0.5 } else { 0.0 };
        let lip_h = 4.0;

        let face_col = if pressed {
            darken(fill, 0.94)
        } else if hovered {
            lighten(fill, 0.10)
        } else {
            fill
        };
        let lip_col = darken(fill, 0.66);

        let painter = ui.painter();
        let radius = CornerRadius::same(12);

        // Soft drop shadow underneath
        painter.rect_filled(
            rect.translate(egui::vec2(0.0, 3.0)),
            CornerRadius::same(13),
            Color32::from_black_alpha(if hovered { 60 } else { 42 }),
        );

        // 3D bottom lip base
        let lip_rect = Rect::from_min_max(
            Pos2::new(rect.left(), rect.top() + lip_h),
            Pos2::new(rect.right(), rect.bottom()),
        );
        painter.rect_filled(lip_rect, radius, lip_col);

        // Main clickable face shifting cleanly onto the lip when pressed
        let face_rect = Rect::from_min_max(
            Pos2::new(rect.left(), rect.top() + press_offset),
            Pos2::new(rect.right(), rect.bottom() - lip_h + press_offset),
        );
        painter.rect_filled(face_rect, radius, face_col);

        // Crisp subtle top inner gloss
        let gloss_rect = Rect::from_min_max(
            Pos2::new(face_rect.left() + 6.0, face_rect.top() + 2.0),
            Pos2::new(face_rect.right() - 6.0, face_rect.top() + 5.0),
        );
        painter.rect_filled(
            gloss_rect,
            CornerRadius::same(3),
            Color32::from_white_alpha(if hovered { 48 } else { 30 }),
        );

        painter.text(
            face_rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(font_size),
            text_color,
        );
    }
    resp
}

pub fn draw_login_marquee(
    ui: &mut egui::Ui,
    rect: Rect,
    t: f32,
    dark: bool,
    tile_scale: f32,
) {
    let mut clipped_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    clipped_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
    let painter = clipped_ui.painter();
    let w = rect.width().max(1.0);
    let h = rect.height().max(1.0);

    // 1. Multiple stacked sideways-scrolling rows of Ferris & Roblox icons
    let row_gap = (78.0 * tile_scale).max(20.0);
    let col_gap = (104.0 * tile_scale).max(26.0);
    let row_count = ((h / row_gap).ceil() as usize + 2).min(24);
    let tile_alpha = if dark { 52 } else { 70 };
    let tint = Color32::from_white_alpha(tile_alpha);
    let ferris_img = egui::Image::new(egui::include_image!("../../assets/bg_ferris.png")).tint(tint);
    let roblox_img = egui::Image::new(egui::include_image!("../../assets/bg_roblox.png"))
        .corner_radius(CornerRadius::same((10.0 * tile_scale).round().clamp(3.0, 10.0) as u8))
        .tint(tint);

    for row in 0..row_count {
        let dir = if row % 2 == 0 { 1.0_f32 } else { -1.0_f32 };
        let speed = (24.0 + (row % 3) as f32 * 7.0) * tile_scale;
        let base_y = rect.top() + 26.0 * tile_scale + row as f32 * row_gap;
        let row_stagger = if row % 2 == 1 { col_gap * 0.5 } else { 0.0 };
        let span = w + col_gap * 2.0;
        let scroll = t * speed * dir + row_stagger;
        let count = (span / col_gap).ceil() as usize + 1;
        let wrap_cols = ((count + 1) / 2) * 2;
        let full_period = wrap_cols as f32 * col_gap;

        for idx in 0..wrap_cols {
            let raw_x = idx as f32 * col_gap + scroll;
            let wrapped_x = raw_x.rem_euclid(full_period) - col_gap;
            let cx = rect.left() + wrapped_x;
            if cx < rect.left() - col_gap || cx > rect.right() + col_gap {
                continue;
            }
            let bob = (t * 1.5 + row as f32 * 0.9 + idx as f32 * 0.7).sin() * 2.5 * tile_scale;
            let center = Pos2::new(cx, base_y + bob);
            let is_ferris = (idx + row) % 2 == 0;
            if is_ferris {
                let sz = Vec2::new(56.0 * tile_scale, 37.5 * tile_scale);
                ferris_img.clone().paint_at(&clipped_ui, Rect::from_center_size(center, sz));
            } else {
                let sz = Vec2::new(42.0 * tile_scale, 42.0 * tile_scale);
                roblox_img.clone().paint_at(&clipped_ui, Rect::from_center_size(center, sz));
            }
        }
    }

    // 2. Soft rolling horizon waves at the bottom
    for (wave_idx, (amp, freq, speed, col, a)) in [
        (20.0_f32, 1.8_f32, 0.42_f32, RR_ORANGE, if dark { 0.08 } else { 0.12 }),
        (15.0_f32, 2.5_f32, -0.58_f32, RR_SKY, if dark { 0.06 } else { 0.09 }),
    ]
    .iter()
    .enumerate()
    {
        let steps = 40;
        let base_y = rect.bottom() - (30.0 + wave_idx as f32 * 13.0) * tile_scale;
        let mut pts = Vec::with_capacity(steps + 3);
        for s in 0..=steps {
            let u = s as f32 / steps as f32;
            let x = rect.left() + u * w;
            let y = base_y + (u * TAU * *freq + t * *speed).sin() * (*amp * tile_scale);
            pts.push(Pos2::new(x, y));
        }
        pts.push(Pos2::new(rect.right(), rect.bottom() + 4.0));
        pts.push(Pos2::new(rect.left(), rect.bottom() + 4.0));
        painter.add(egui::Shape::convex_polygon(
            pts,
            col.gamma_multiply(*a),
            Stroke::NONE,
        ));
    }
}

fn draw_recroom_backdrop(ui: &mut egui::Ui, rect: Rect, t: f32, dark: bool) {
    draw_login_marquee(ui, rect, t, dark, 1.0);
}

impl RustBlox {
    pub fn page_login(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let logged_in = self.cookie.is_some();
        let dark = effective_dark_mode(&self.settings);
        let p = pal(ui);
        let t = ui.input(|i| i.time as f32);

        // Paint continuous Rec Room animated backdrop across the full viewport
        let full_rect = ui.clip_rect();
        draw_recroom_backdrop(ui, full_rect, t, dark);
        ui.ctx().request_repaint();

        let mut cancel_clicked = false;
        let mut regen_clicked = false;

        if logged_in {
            ui.add_space(6.0);
            if ui.button("← Back").clicked() {
                cancel_clicked = true;
            }
        }

        // Vertically center the card smoothly based on current content height
        let avail_h = ui.available_height();
        let approx_card_h = if self.quick_login.is_some() {
            520.0
        } else if !logged_in && !self.saved_accounts.is_empty() {
            390.0
        } else {
            305.0
        };
        let top_pad = ((avail_h - approx_card_h) * 0.44).max(16.0);
        ui.add_space(top_pad);

        ui.vertical_centered(|ui| {
            let card_w = 450.0_f32.min(ui.available_width() - 24.0).max(300.0);
            let card_bg = if dark {
                Color32::from_rgba_unmultiplied(21, 25, 34, 240)
            } else {
                Color32::from_rgba_unmultiplied(255, 253, 250, 244)
            };
            let card_border = if dark {
                Color32::from_rgba_unmultiplied(255, 255, 255, 22)
            } else {
                Color32::from_rgba_unmultiplied(255, 130, 72, 65)
            };

            egui::Frame::new()
                .fill(card_bg)
                .stroke(Stroke::new(1.5, card_border))
                .corner_radius(CornerRadius::same(22))
                .shadow(egui::Shadow {
                    offset: [0, 14],
                    blur: 36,
                    spread: 0,
                    color: Color32::from_black_alpha(if dark { 110 } else { 36 }),
                })
                .inner_margin(egui::Margin::symmetric(28, 26))
                .show(ui, |ui| {
                    ui.set_width(card_w - 56.0);
                    ui.vertical_centered(|ui| {
                        let pulse = (t * 2.8).sin() * 0.5 + 0.5;
                        ui.add(
                            egui::Image::new(egui::include_image!("../../assets/logo.png"))
                                .fit_to_exact_size(egui::vec2(48.0, 48.0)),
                        );

                        ui.add_space(6.0);
                        brand_title(ui, 26.0, false);
                        ui.add_space(6.0);
                        let subtitle = if logged_in {
                            "Add another Roblox account using official Quick Login"
                        } else {
                            "Sign in securely via Official Roblox Quick Login"
                        };
                        ui.label(RichText::new(subtitle).color(p.muted).size(13.0));
                        ui.add_space(16.0);

                        if let Some(ql) = &self.quick_login {
                            if let Some(err) = &ql.error {
                                egui::Frame::new()
                                    .fill(RED.gamma_multiply(0.12))
                                    .stroke(Stroke::new(1.0, RED.gamma_multiply(0.45)))
                                    .corner_radius(10)
                                    .inner_margin(egui::Margin::symmetric(14, 10))
                                    .show(ui, |ui| {
                                        ui.colored_label(RED, err);
                                    });
                                ui.add_space(12.0);
                            } else if ql.code.is_empty() {
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(RichText::new(&ql.status_text).size(13.5));
                                });
                                ui.add_space(10.0);
                            } else {
                                // Perfect QR card with animated corner viewfinder brackets
                                if let Some(bytes) = &ql.qr_bytes {
                                    let qr_dim = 176.0_f32;
                                    let pad = 14.0_f32;
                                    let outer_sz = egui::vec2(qr_dim + pad * 2.0, qr_dim + pad * 2.0);
                                    let (qr_box, _) = ui.allocate_exact_size(outer_sz, Sense::hover());

                                    // Drop shadow + pure white crisp QR backing
                                    ui.painter().rect_filled(
                                        qr_box.translate(egui::vec2(0.0, 5.0)),
                                        CornerRadius::same(18),
                                        Color32::from_black_alpha(if dark { 85 } else { 28 }),
                                    );
                                    ui.painter().rect_filled(
                                        qr_box,
                                        CornerRadius::same(16),
                                        Color32::WHITE,
                                    );
                                    ui.painter().rect_stroke(
                                        qr_box,
                                        CornerRadius::same(16),
                                        Stroke::new(1.5, RR_ORANGE.gamma_multiply(0.32)),
                                        egui::StrokeKind::Inside,
                                    );

                                    let qr_img_rect =
                                        Rect::from_center_size(qr_box.center(), egui::vec2(qr_dim, qr_dim));
                                    egui::Image::from_bytes(ql.qr_uri.clone(), bytes.clone())
                                        .texture_options(egui::TextureOptions::NEAREST)
                                        .paint_at(ui, qr_img_rect);

                                    // Animated Rec Room viewfinder corner brackets outside QR
                                    let bracket_inset = 5.0 + pulse * 1.5;
                                    let br = qr_box.expand(bracket_inset);
                                    let blen = 16.0;
                                    let bstroke = Stroke::new(2.5, RR_ORANGE.gamma_multiply(0.85));
                                    for (cx, cy, dx, dy) in [
                                        (br.left(), br.top(), 1.0_f32, 1.0_f32),
                                        (br.right(), br.top(), -1.0, 1.0),
                                        (br.left(), br.bottom(), 1.0, -1.0),
                                        (br.right(), br.bottom(), -1.0, -1.0),
                                    ] {
                                        let corner = Pos2::new(cx, cy);
                                        ui.painter().line_segment(
                                            [corner, Pos2::new(cx + blen * dx, cy)],
                                            bstroke,
                                        );
                                        ui.painter().line_segment(
                                            [corner, Pos2::new(cx, cy + blen * dy)],
                                            bstroke,
                                        );
                                    }

                                    ui.add_space(16.0);
                                }

                                // Individual spaced 6-character code tiles
                                let chars: Vec<char> = ql.code.chars().collect();
                                ui.horizontal(|ui| {
                                    let tile_w = 34.0_f32;
                                    let gap = 6.0_f32;
                                    let total_w =
                                        chars.len() as f32 * tile_w + (chars.len().saturating_sub(1)) as f32 * gap
                                            + if chars.len() == 6 { 10.0 } else { 0.0 };
                                    let extra = ((ui.available_width() - total_w) * 0.5).max(0.0);
                                    ui.add_space(extra);
                                    ui.spacing_mut().item_spacing.x = gap;
                                    for (idx, ch) in chars.iter().enumerate() {
                                        if chars.len() == 6 && idx == 3 {
                                            ui.add_space(4.0);
                                        }
                                        let (trect, _) = ui.allocate_exact_size(
                                            egui::vec2(tile_w, 44.0),
                                            Sense::hover(),
                                        );
                                        ui.painter().rect_filled(
                                            trect,
                                            CornerRadius::same(9),
                                            p.surface_bg,
                                        );
                                        ui.painter().rect_stroke(
                                            trect,
                                            CornerRadius::same(9),
                                            Stroke::new(1.5, p.border),
                                            egui::StrokeKind::Inside,
                                        );
                                        ui.painter().text(
                                            trect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            ch.to_string(),
                                            FontId::monospace(22.0),
                                            p.text,
                                        );
                                    }
                                });

                                ui.add_space(10.0);
                                let copy_id = ui.scope_id().with("ql_copy_anim");
                                let copied_until: f64 =
                                    ui.data(|d| d.get_temp(copy_id)).unwrap_or(0.0);
                                let now = ui.input(|i| i.time);
                                let is_copied = now < copied_until;
                                let copy_label = if is_copied { "Copied!" } else { "Copy Code" };
                                if rr_button(
                                    ui,
                                    copy_label,
                                    if is_copied { GREEN } else { p.surface_bg },
                                    if is_copied { Color32::WHITE } else { p.text },
                                    egui::vec2(124.0, 34.0),
                                    12.5,
                                )
                                .clicked()
                                {
                                    ui.ctx().copy_text(ql.code.clone());
                                    ui.data_mut(|d| d.insert_temp(copy_id, now + 1.6));
                                    self.status = "Quick Login code copied".to_string();
                                }

                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new("Approve this code inside the official Roblox app or roblox.com/crosslogin.")
                                        .size(11.5)
                                        .color(p.muted),
                                );
                                ui.add_space(8.0);
                                let linked = ql.status_text.contains("Linked")
                                    || ql.status_text.contains("Confirm");
                                ui.horizontal(|ui| {
                                    let dot_col = if linked { GREEN } else { RR_ORANGE };
                                    let (dot_r, _) =
                                        ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
                                    ui.painter().circle_filled(
                                        dot_r.center(),
                                        3.5 + pulse * 1.5,
                                        dot_col,
                                    );
                                    ui.label(
                                        RichText::new(&ql.status_text)
                                            .size(12.5)
                                            .color(if linked { GREEN } else { p.text }),
                                    );
                                });
                            }
                        }

                        ui.add_space(if self.quick_login.is_some() { 16.0 } else { 6.0 });

                        // Primary tactile Rec Room button row
                        if let Some(_) = &self.quick_login {
                            ui.horizontal(|ui| {
                                let w = 155.0_f32;
                                let gap = 10.0_f32;
                                let extra = ((ui.available_width() - (w + 100.0 + gap)) * 0.5).max(0.0);
                                ui.add_space(extra);
                                if rr_button(
                                    ui,
                                    "New QR & Code",
                                    RR_ORANGE,
                                    Color32::WHITE,
                                    egui::vec2(w, 40.0),
                                    13.5,
                                )
                                .clicked()
                                {
                                    regen_clicked = true;
                                }
                                ui.add_space(gap);
                                if rr_button(
                                    ui,
                                    "Cancel",
                                    p.surface_bg,
                                    p.text,
                                    egui::vec2(100.0, 40.0),
                                    13.0,
                                )
                                .clicked()
                                {
                                    cancel_clicked = true;
                                }
                            });
                        } else if rr_button(
                            ui,
                            "Sign in with Quick Login",
                            RR_ORANGE,
                            Color32::WHITE,
                            egui::vec2(250.0, 46.0),
                            15.0,
                        )
                        .clicked()
                        {
                            regen_clicked = true;
                        }

                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Direct connection to roblox.com  •  Session encrypted on this PC")
                                .size(11.0)
                                .color(p.muted),
                        );

                        // Saved accounts quick switch list when signed out
                        if !logged_in && !self.saved_accounts.is_empty() {
                            ui.add_space(18.0);
                            ui.separator();
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Saved Accounts")
                                    .size(12.0)
                                    .strong()
                                    .color(p.muted),
                            );
                            ui.add_space(6.0);
                            let saved = self.saved_accounts.clone();
                            for acc in &saved {
                                let (r_rect, r_resp) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 40.0),
                                    Sense::click(),
                                );
                                ui.painter().rect_filled(
                                    r_rect,
                                    CornerRadius::same(10),
                                    if r_resp.hovered() {
                                        p.hover_bg
                                    } else {
                                        p.surface_bg
                                    },
                                );
                                ui.painter().rect_stroke(
                                    r_rect,
                                    CornerRadius::same(10),
                                    Stroke::new(1.0, p.border),
                                    egui::StrokeKind::Inside,
                                );
                                let mut row_ui = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(r_rect.shrink2(egui::vec2(10.0, 5.0))),
                                );
                                row_ui.horizontal(|ui| {
                                    let _ = thumb_styled(
                                        ui,
                                        Some(&acc.headshot),
                                        egui::vec2(28.0, 28.0),
                                        14,
                                        true,
                                    );
                                    ui.label(RichText::new(&acc.display).strong().size(13.0));
                                    if acc.verified {
                                        verified_badge_tip(ui, 13.0, "Verified Account");
                                    }
                                    ui.label(
                                        RichText::new(format!("@{}", acc.name))
                                            .size(11.5)
                                            .color(p.muted),
                                    );
                                });
                                if r_resp.clicked() {
                                    acts.push(Action::SwitchAccount(acc.id));
                                }
                                ui.add_space(4.0);
                            }
                        }
                    });
                });
        });

        if regen_clicked {
            acts.push(Action::StartQuickLogin);
        } else if cancel_clicked {
            self.cancel_quick_login();
            if logged_in {
                acts.push(Action::Go(self.back.clone()));
            }
        }
    }
}
