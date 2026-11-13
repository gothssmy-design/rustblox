use eframe::egui::{self, Color32, CornerRadius, RichText, Sense};
use crate::app::RustBlox;
use crate::avatar_3d::render_avatar_viewport;
use crate::theme::{card_frame, compact, pal, BLUE, GREEN, ORANGE, RED};
use crate::types::{Action, FriendProfile, Page, User};
use crate::ui::widgets::*;

impl RustBlox {
    pub fn paint_pastel_gradient(&self, painter: &egui::Painter, rect: egui::Rect, is_dark: bool) {
        let mut mesh = egui::Mesh::default();
        let tl = rect.left_top();
        let tc = egui::pos2(rect.center().x, rect.top());
        let tr = rect.right_top();
        let bl = rect.left_bottom();
        let bc = egui::pos2(rect.center().x, rect.bottom());
        let br = rect.right_bottom();

        let (c_tl, c_tc, c_tr, c_bl, c_bc, c_br) = if is_dark {
            (
                Color32::from_rgb(175, 215, 205),
                Color32::from_rgb(240, 225, 180),
                Color32::from_rgb(210, 190, 230),
                Color32::from_rgb(190, 215, 230),
                Color32::from_rgb(235, 210, 200),
                Color32::from_rgb(180, 210, 230),
            )
        } else {
            (
                Color32::from_rgb(199, 235, 222),
                Color32::from_rgb(250, 238, 185),
                Color32::from_rgb(222, 198, 238),
                Color32::from_rgb(212, 233, 245),
                Color32::from_rgb(248, 230, 220),
                Color32::from_rgb(205, 232, 245),
            )
        };

        let idx = mesh.vertices.len() as u32;
        let w_uv = egui::epaint::WHITE_UV;
        mesh.vertices.push(egui::epaint::Vertex { pos: tl, uv: w_uv, color: c_tl });
        mesh.vertices.push(egui::epaint::Vertex { pos: tc, uv: w_uv, color: c_tc });
        mesh.vertices.push(egui::epaint::Vertex { pos: tr, uv: w_uv, color: c_tr });
        mesh.vertices.push(egui::epaint::Vertex { pos: bl, uv: w_uv, color: c_bl });
        mesh.vertices.push(egui::epaint::Vertex { pos: bc, uv: w_uv, color: c_bc });
        mesh.vertices.push(egui::epaint::Vertex { pos: br, uv: w_uv, color: c_br });

        mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 4, idx, idx + 4, idx + 3]);
        mesh.indices.extend_from_slice(&[idx + 1, idx + 2, idx + 5, idx + 1, idx + 5, idx + 4]);

        painter.add(egui::Shape::Mesh(mesh.into()));
    }

    pub fn render_profile_view(
        &mut self,
        ui: &mut egui::Ui,
        profile: &FriendProfile,
        is_self: bool,
        acts: &mut Vec<Action>,
    ) {
        let p = pal(ui);
        let win_w = ui.available_width();
        let is_dark = ui.visuals().dark_mode;

        egui::ScrollArea::vertical()
            .id_salt(if is_self { "my_profile_scroll" } else { "user_profile_scroll" })
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(win_w);

                // 1. Top Panoramic Hero Banner with Pastel Iridescent Gradient
                let banner_h = (win_w * 0.30).clamp(240.0, 310.0);
                let (banner_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), banner_h),
                    Sense::hover(),
                );

                // Paint pastel gradient
                self.paint_pastel_gradient(ui.painter(), banner_rect, is_dark);
                ui.painter().rect_stroke(
                    banner_rect,
                    CornerRadius::same(16),
                    egui::Stroke::new(1.0, p.border),
                    egui::StrokeKind::Inside,
                );

                // Center avatar inside banner
                let avatar_w = (banner_h * 0.95).min(banner_rect.width() * 0.5);
                let avatar_rect = egui::Rect::from_center_size(
                    banner_rect.center(),
                    egui::vec2(avatar_w, banner_h * 0.95),
                );

                if self.profile_3d_mode {
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(avatar_rect));
                    render_avatar_viewport(
                        &mut child,
                        &mut self.avatar_3d,
                        Some(&profile.avatar),
                        avatar_rect.size(),
                        is_dark,
                    );
                } else if !profile.avatar.is_empty() {
                    egui::Image::new(&profile.avatar)
                        .fit_to_exact_size(avatar_rect.size())
                        .paint_at(ui, avatar_rect);
                }

                // 3D / 2D toggle pill in the top-right corner of banner
                let toggle_size = egui::vec2(44.0, 26.0);
                let toggle_rect = egui::Rect::from_min_size(
                    egui::pos2(banner_rect.right() - 56.0, banner_rect.top() + 14.0),
                    toggle_size,
                );
                let toggle_resp = ui.allocate_rect(toggle_rect, Sense::click());
                let toggle_hovered = toggle_resp.hovered();
                ui.painter().rect_filled(
                    toggle_rect,
                    CornerRadius::same(6),
                    if toggle_hovered {
                        Color32::from_black_alpha(170)
                    } else {
                        Color32::from_black_alpha(120)
                    },
                );
                ui.painter().rect_stroke(
                    toggle_rect,
                    CornerRadius::same(6),
                    egui::Stroke::new(1.0, Color32::from_white_alpha(45)),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    toggle_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    if self.profile_3d_mode { "2D" } else { "3D" },
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );
                if toggle_resp.clicked() {
                    self.profile_3d_mode = !self.profile_3d_mode;
                    if self.profile_3d_mode && self.avatar_3d.model.is_none() {
                        self.load_avatar_3d();
                    }
                }

                ui.add_space(16.0);

                // 2. Identity Row (Headshot circle + Names + Stats + Action Buttons)
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 18.0;

                    // Avatar headshot circle with presence status badge
                    let headshot_size = 96.0;
                    let (headshot_rect, _) = ui.allocate_exact_size(
                        egui::vec2(headshot_size, headshot_size),
                        Sense::hover(),
                    );

                    let headshot_url = if !profile.user.headshot.is_empty() {
                        &profile.user.headshot
                    } else {
                        &profile.avatar
                    };
                    if !headshot_url.is_empty() {
                        egui::Image::new(headshot_url)
                            .corner_radius(CornerRadius::same(headshot_size as u8 / 2))
                            .paint_at(ui, headshot_rect);
                    }
                    ui.painter().circle_stroke(
                        headshot_rect.center(),
                        headshot_size * 0.5,
                        egui::Stroke::new(3.0, p.card_bg),
                    );

                    // Presence indicator dot at bottom-right
                    let (dot_color, presence_label) = match profile.user.presence {
                        2 => (
                            GREEN,
                            if profile.user.location.is_empty() {
                                "In Game".to_string()
                            } else {
                                format!("Playing {}", profile.user.location)
                            },
                        ),
                        3 => (ORANGE, "In Studio".to_string()),
                        1 => (BLUE, "Online".to_string()),
                        _ => (p.muted, "Offline".to_string()),
                    };
                    let dot_center = headshot_rect.center()
                        + egui::vec2(headshot_size * 0.35, headshot_size * 0.35);
                    ui.painter().circle_filled(dot_center, 8.5, p.card_bg);
                    ui.painter().circle_filled(dot_center, 6.0, dot_color);
                    let dot_rect = egui::Rect::from_center_size(dot_center, egui::vec2(16.0, 16.0));
                    ui.allocate_rect(dot_rect, Sense::hover()).on_hover_text(presence_label);

                    // User Names and Stats
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&profile.user.display).size(26.0).strong());
                            if profile.verified || profile.user.verified {
                                verified_badge_tip(ui, 20.0, "Verified Account");
                            }
                        });
                        ui.label(
                            RichText::new(format!("@{}", profile.user.name))
                                .size(14.0)
                                .color(p.muted),
                        );
                        ui.add_space(6.0);

                        // Stat Chips
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            for (count, label) in [
                                (profile.friends, "Friends"),
                                (profile.followers, "Followers"),
                                (profile.following, "Following"),
                            ] {
                                egui::Frame::new()
                                    .fill(p.surface_bg)
                                    .stroke(egui::Stroke::new(1.0, p.border))
                                    .corner_radius(12)
                                    .inner_margin(egui::Margin::symmetric(12, 5))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(format!("{} {}", compact(count), label))
                                                .size(12.5)
                                                .strong(),
                                        );
                                    });
                            }
                            if !profile.created.is_empty() {
                                egui::Frame::new()
                                    .fill(p.surface_bg)
                                    .stroke(egui::Stroke::new(1.0, p.border))
                                    .corner_radius(12)
                                    .inner_margin(egui::Margin::symmetric(12, 5))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(format!("Joined {}", profile.created))
                                                .size(12.5)
                                                .color(p.muted),
                                        );
                                    });
                            }
                        });
                    });

                    // Action buttons (right aligned)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;

                        // More options popup
                        let more_btn = egui::Button::new(RichText::new("Â·Â·Â·").size(15.0).strong())
                            .fill(p.hover_bg)
                            .stroke(egui::Stroke::new(1.0, p.border))
                            .corner_radius(8)
                            .min_size(egui::vec2(36.0, 32.0));
                        let more_resp = ui.add(more_btn);
                        more_resp.context_menu(|ui| {
                            if ui.button("ðŸ“‹ Copy Profile Link").clicked() {
                                ui.ctx().copy_text(format!(
                                    "https://www.roblox.com/users/{}/profile",
                                    profile.user.id
                                ));
                                self.status = "Copied profile link!".to_string();
                                ui.close();
                            }
                            if ui.button("ðŸ†” Copy User ID").clicked() {
                                ui.ctx().copy_text(profile.user.id.to_string());
                                self.status = "Copied user ID!".to_string();
                                ui.close();
                            }
                            if ui.button("ðŸŒ Open on Roblox.com").clicked() {
                                acts.push(Action::OpenUrl(format!(
                                    "https://www.roblox.com/users/{}/profile",
                                    profile.user.id
                                )));
                                ui.close();
                            }
                            if is_self {
                                if ui.button("â†» Reload Profile").clicked() {
                                    acts.push(Action::RefreshMyProfile);
                                    ui.close();
                                }
                            }
                        });

                        if is_self {
                            // Edit avatar button -> goes to Avatar customization!
                            let edit_avatar_btn = egui::Button::new(
                                RichText::new("Edit avatar")
                                    .size(13.0)
                                    .strong()
                                    .color(p.text),
                            )
                            .fill(p.hover_bg)
                            .stroke(egui::Stroke::new(1.0, p.border))
                            .corner_radius(8)
                            .min_size(egui::vec2(86.0, 32.0));
                            if ui.add(edit_avatar_btn).clicked() {
                                acts.push(Action::Go(Page::Avatar));
                            }

                            // Edit profile button -> toggles inline bio editing
                            let edit_prof_btn = egui::Button::new(
                                RichText::new("Edit profile")
                                    .size(13.0)
                                    .strong()
                                    .color(p.text),
                            )
                            .fill(p.hover_bg)
                            .stroke(egui::Stroke::new(1.0, p.border))
                            .corner_radius(8)
                            .min_size(egui::vec2(86.0, 32.0));
                            if ui.add(edit_prof_btn).clicked() {
                                self.profile_editing = !self.profile_editing;
                                if self.profile_editing {
                                    self.profile_edit_bio = profile.description.clone();
                                }
                            }
                        } else {
                            let is_friend = self.friends.iter().any(|f| f.id == profile.user.id);
                            let is_sent = self.friend_sent.contains(&profile.user.id);

                            if is_friend {
                                let friend_pill = egui::Button::new(
                                    RichText::new("âœ“ Friends").size(13.0).strong().color(p.muted),
                                )
                                .fill(p.hover_bg)
                                .stroke(egui::Stroke::new(1.0, p.border))
                                .corner_radius(8);
                                ui.add_enabled(false, friend_pill);
                            } else if is_sent {
                                let sent_pill = egui::Button::new(
                                    RichText::new("Request Sent").size(13.0).color(p.muted),
                                )
                                .fill(p.hover_bg)
                                .stroke(egui::Stroke::new(1.0, p.border))
                                .corner_radius(8);
                                ui.add_enabled(false, sent_pill);
                            } else {
                                let add_btn = egui::Button::new(
                                    RichText::new("+ Add Friend")
                                        .size(13.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(GREEN)
                                .corner_radius(8);
                                if ui.add(add_btn).clicked() {
                                    acts.push(Action::SendFriendRequest(
                                        profile.user.id,
                                        profile.user.name.clone(),
                                    ));
                                }
                            }

                            if let Some(pid) = profile.user.place_id {
                                let join_btn = egui::Button::new(
                                    RichText::new("â–¶  Join Game")
                                        .size(13.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(GREEN)
                                .corner_radius(8);
                                if ui.add(join_btn).clicked() {
                                    acts.push(Action::JoinPlace(pid));
                                }
                            }
                        }
                    });
                });

                ui.add_space(14.0);

                // 3. About / Bio Section
                if self.profile_editing && is_self {
                    card_frame(ui, 12).show(ui, |ui| {
                        ui.label(RichText::new("Edit Bio / Description").strong().size(14.0));
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::multiline(&mut self.profile_edit_bio)
                                .desired_rows(3)
                                .desired_width(f32::INFINITY)
                                .hint_text("Write something about yourself..."),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if self.profile_saving_bio {
                                ui.spinner();
                                ui.label("Saving...");
                            } else {
                                let save_btn = egui::Button::new(
                                    RichText::new("Save").strong().color(Color32::WHITE),
                                )
                                .fill(GREEN)
                                .corner_radius(6);
                                if ui.add(save_btn).clicked() {
                                    acts.push(Action::SaveBio(self.profile_edit_bio.clone()));
                                }
                                if ui.button("Cancel").clicked() {
                                    self.profile_editing = false;
                                }
                            }
                            if let Some(e) = &self.profile_save_err {
                                ui.colored_label(RED, e);
                            }
                        });
                    });
                } else if !profile.description.is_empty() {
                    let bio = &profile.description;
                    let is_long = bio.len() > 180;
                    ui.horizontal_wrapped(|ui| {
                        if is_long && !self.profile_bio_expanded {
                            let truncated: String = bio.chars().take(180).collect();
                            ui.label(RichText::new(format!("{truncated}...")).size(13.5));
                            if ui.link(RichText::new("more").color(BLUE)).clicked() {
                                self.profile_bio_expanded = true;
                            }
                        } else {
                            ui.label(RichText::new(bio).size(13.5));
                            if is_long {
                                if ui.link(RichText::new("less").color(BLUE)).clicked() {
                                    self.profile_bio_expanded = false;
                                }
                            }
                        }
                    });
                } else if is_self {
                    ui.label(
                        RichText::new("No bio yet. Click 'Edit profile' to add one!")
                            .size(13.0)
                            .italics()
                            .color(p.muted),
                    );
                }

                // 4. Currently Wearing Carousel
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Currently Wearing").size(18.0).strong());
                    if !profile.currently_wearing.is_empty() {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!("({})", profile.currently_wearing.len()))
                                .size(13.0)
                                .color(p.muted),
                        );
                    }
                });
                ui.add_space(8.0);

                if profile.currently_wearing.is_empty() {
                    ui.label(RichText::new("No worn items to display.").size(13.0).color(p.muted));
                } else {
                    egui::ScrollArea::horizontal()
                        .id_salt(if is_self { "my_wearing_scroll" } else { "user_wearing_scroll" })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 10.0;
                                for item in &profile.currently_wearing {
                                    let card_w = 126.0;
                                    let card_h = 176.0;
                                    let (card_rect, card_resp) = ui.allocate_exact_size(
                                        egui::vec2(card_w, card_h),
                                        Sense::click(),
                                    );
                                    let card_resp = card_resp
                                        .on_hover_cursor(egui::CursorIcon::PointingHand);

                                    let hovered = card_resp.hovered();
                                    let bg_color =
                                        if hovered { p.hover_bg } else { p.surface_bg };
                                    let stroke_color = if hovered { GREEN } else { p.border };

                                    ui.painter().rect_filled(
                                        card_rect,
                                        CornerRadius::same(10),
                                        bg_color,
                                    );
                                    ui.painter().rect_stroke(
                                        card_rect,
                                        CornerRadius::same(10),
                                        egui::Stroke::new(
                                            if hovered { 1.5 } else { 1.0 },
                                            stroke_color,
                                        ),
                                        egui::StrokeKind::Inside,
                                    );

                                    // Thumbnail 110x110
                                    let thumb_rect = egui::Rect::from_min_size(
                                        egui::pos2(card_rect.left() + 8.0, card_rect.top() + 8.0),
                                        egui::vec2(110.0, 110.0),
                                    );
                                    ui.painter().rect_filled(
                                        thumb_rect,
                                        CornerRadius::same(8),
                                        if hovered { p.selected_bg } else { p.hover_bg },
                                    );
                                    if !item.thumb.is_empty() {
                                        egui::Image::new(&item.thumb)
                                            .corner_radius(CornerRadius::same(8))
                                            .fit_to_exact_size(thumb_rect.size())
                                            .paint_at(ui, thumb_rect);
                                    }

                                    // Item title
                                    let title_pos =
                                        egui::pos2(card_rect.left() + 8.0, card_rect.top() + 124.0);
                                    let clean_title: String = if item.name.len() > 18 {
                                        format!(
                                            "{}...",
                                            item.name.chars().take(16).collect::<String>()
                                        )
                                    } else {
                                        item.name.clone()
                                    };
                                    ui.painter().text(
                                        title_pos,
                                        egui::Align2::LEFT_TOP,
                                        clean_title,
                                        egui::FontId::proportional(12.0),
                                        p.text,
                                    );

                                    // Price row
                                    let price_pos =
                                        egui::pos2(card_rect.left() + 8.0, card_rect.top() + 148.0);
                                    let price_str = if let Some(pr) = item.price {
                                        format!("â—Ž {}", compact(pr))
                                    } else {
                                        "Free".to_string()
                                    };
                                    ui.painter().text(
                                        price_pos,
                                        egui::Align2::LEFT_TOP,
                                        price_str,
                                        egui::FontId::proportional(11.5),
                                        if item.price.is_some() { p.muted } else { GREEN },
                                    );

                                    // Tooltip
                                    let mut card_resp = card_resp;
                                    card_resp = card_resp.on_hover_ui(|ui| {
                                        ui.label(RichText::new(&item.name).size(13.0).strong());
                                        ui.label(
                                            RichText::new(format!("Type: {}", item.asset_type_name))
                                                .size(11.5)
                                                .color(p.muted),
                                        );
                                        if !item.creator_name.is_empty() {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "By {}",
                                                        item.creator_name
                                                    ))
                                                    .size(11.5)
                                                    .color(p.muted),
                                                );
                                                if item.creator_verified {
                                                    verified_badge_tip(ui, 12.0, "Verified Creator");
                                                }
                                            });
                                        }
                                        if let Some(pr) = item.price {
                                            ui.label(
                                                RichText::new(format!("Price: â—Ž {} Robux", pr))
                                                    .size(12.0)
                                                    .color(GREEN)
                                                    .strong(),
                                            );
                                        }
                                    });

                                    if card_resp.clicked() {
                                        acts.push(Action::OpenUrl(format!(
                                            "https://www.roblox.com/catalog/{}",
                                            item.id
                                        )));
                                    }
                                }
                            });
                        });
                }

                // 5. Currently Playing (if in game)
                if let Some(game) = &profile.playing_game {
                    ui.add_space(24.0);
                    ui.label(RichText::new("Currently Playing").size(18.0).strong());
                    ui.add_space(8.0);
                    card_frame(ui, 14).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let t = thumb(
                                ui,
                                self.wide.get(&game.universe_id),
                                egui::vec2(192.0, 108.0),
                                8,
                            );
                            ui.add_space(12.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&game.name).size(18.0).strong());
                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new(format!("By {}", game.creator))
                                        .size(13.0)
                                        .color(p.muted),
                                );
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "ðŸ‘¥ {} Playing",
                                            compact(game.playing)
                                        ))
                                        .size(13.0)
                                        .color(GREEN)
                                        .strong(),
                                    );
                                    if let Some(pid) = profile.user.place_id {
                                        ui.add_space(14.0);
                                        let join_btn = egui::Button::new(
                                            RichText::new("â–¶   Join Game")
                                                .size(14.0)
                                                .strong()
                                                .color(Color32::WHITE),
                                        )
                                        .fill(GREEN)
                                        .corner_radius(8);
                                        if ui.add(join_btn).clicked() {
                                            acts.push(Action::JoinPlace(pid));
                                        }
                                    }
                                });
                            });
                            if t.clicked() {
                                acts.push(Action::Open(game.clone()));
                            }
                        });
                    });
                }

                // 6. Experiences / Creations
                if !profile.experiences.is_empty() {
                    ui.add_space(24.0);
                    ui.label(RichText::new("Creations").size(18.0).strong());
                    ui.add_space(8.0);
                    let preview = self.current_card_preview();
                    wide_grid_with_width(ui, &profile.experiences, &self.wide, preview, 220.0, acts);
                }

                ui.add_space(30.0);
            });
    }

    pub fn page_my_profile(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if self.my_profile.is_none() && !self.my_profile_loading {
            self.ensure_my_profile_loaded();
        }
        let p = pal(ui);
        let loading = self.my_profile_loading;
        let err = self.my_profile_err.clone();
        let profile = self.my_profile.clone();

        if let Some(prof) = &profile {
            self.render_profile_view(ui, prof, true, acts);
        } else if loading {
            ui.add_space(40.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Loading profile...").size(16.0).color(p.muted));
            });
        } else if let Some(e) = &err {
            ui.add_space(20.0);
            card_frame(ui, 16).show(ui, |ui| {
                ui.colored_label(RED, format!("Could not load profile: {e}"));
                ui.add_space(8.0);
                if ui.button("Retry").clicked() {
                    acts.push(Action::RefreshMyProfile);
                }
            });
        } else {
            ui.add_space(40.0);
            ui.label(RichText::new("Please sign in to view your profile.").color(p.muted));
        }
    }

    pub fn page_friend(&mut self, ui: &mut egui::Ui, u: &User, acts: &mut Vec<Action>) {
        let p = pal(ui);
        ui.add_space(6.0);
        if ui.button("â† Back").clicked() {
            acts.push(Action::Go(self.back.clone()));
        }
        ui.add_space(8.0);

        let fp = self.friend_profile.as_ref().filter(|fp| fp.user.id == u.id).cloned();
        if let Some(prof) = &fp {
            self.render_profile_view(ui, prof, false, acts);
        } else if self.friend_loading {
            ui.add_space(30.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new(format!("Loading @{}'s profile...", u.name))
                        .size(15.0)
                        .color(p.muted),
                );
            });
        } else if let Some(err) = &self.friend_err {
            ui.colored_label(RED, format!("Could not load profile: {err}"));
        } else {
            ui.spinner();
        }
    }

}
