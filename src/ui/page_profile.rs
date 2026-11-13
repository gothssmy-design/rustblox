use eframe::egui::{self, Color32, CornerRadius, RichText, Sense};

use crate::app::RustBlox;
use crate::theme::{
    accent_color, card_frame_colored, compact, pal, BLUE, GREEN, ORANGE, RED,
};
use crate::types::{Action, FriendProfile, Page, User};
use crate::ui::widgets::*;

impl RustBlox {
    fn render_profile_view(
        &mut self,
        ui: &mut egui::Ui,
        profile: &FriendProfile,
        is_self: bool,
        acts: &mut Vec<Action>,
    ) {
        let max_w = ui.available_width().min(980.0);
        ui.vertical_centered(|ui| {
            ui.set_max_width(max_w);
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                self.render_profile_header_card(ui, profile, is_self, acts);
                ui.add_space(16.0);

                if profile.playing_game.is_some() {
                    self.render_profile_playing_card(ui, profile, acts);
                    ui.add_space(16.0);
                }

                self.render_profile_wearing_section(ui, profile, acts);
                ui.add_space(18.0);
                self.render_profile_favorites_section(ui, profile, acts);
                ui.add_space(18.0);
                self.render_profile_badges_section(ui, profile, acts);

                if !profile.experiences.is_empty() {
                    ui.add_space(18.0);
                    let p = pal(ui);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(RichText::new("Creations").size(16.5).strong());
                        ui.label(
                            RichText::new(format!("{}", profile.experiences.len()))
                                .size(12.5)
                                .color(p.muted),
                        );
                    });
                    ui.add_space(8.0);
                    let preview = self.current_card_preview();
                    wide_grid_with_width(ui, &profile.experiences, &self.wide, preview, 220.0, acts);
                }

                ui.add_space(28.0);
            });
        });
    }

    fn render_profile_header_card(
        &mut self,
        ui: &mut egui::Ui,
        profile: &FriendProfile,
        is_self: bool,
        acts: &mut Vec<Action>,
    ) {
        let p = pal(ui);
        card_frame_colored(20, p.profile_card_bg, p.profile_border).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 22.0;

                self.render_profile_avatar_box(ui, profile);

                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 8.0;

                    // Top row: Display name + action buttons
                    ui.horizontal(|ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.label(RichText::new(&profile.user.display).size(24.0).strong());
                            if profile.verified || profile.user.verified {
                                verified_badge_tip(ui, 17.0, "Verified Account");
                            }
                            render_presence_pill(ui, profile);
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            self.render_profile_actions(ui, profile, is_self, acts);
                        });
                    });

                    // Subtitle row: @username + join date + user ID
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(
                            RichText::new(format!("@{}", profile.user.name))
                                .size(13.5)
                                .color(p.muted),
                        );
                        if !profile.created.is_empty() {
                            ui.label(RichText::new("•").size(12.0).color(p.muted));
                            ui.label(
                                RichText::new(format!("Joined {}", profile.created))
                                    .size(12.5)
                                    .color(p.muted),
                            );
                        }
                        ui.label(RichText::new("•").size(12.0).color(p.muted));
                        let id_lbl = ui
                            .add(
                                egui::Label::new(
                                    RichText::new(format!("ID {}", profile.user.id))
                                        .size(12.0)
                                        .color(p.muted),
                                )
                                .sense(Sense::click()),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Click to copy user ID");
                        if id_lbl.clicked() {
                            ui.ctx().copy_text(profile.user.id.to_string());
                            self.status = "Copied user ID!".to_string();
                        }
                    });

                    // Clean metric row
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        for (count, label) in [
                            (profile.friends, "Friends"),
                            (profile.followers, "Followers"),
                            (profile.following, "Following"),
                        ] {
                            egui::Frame::new()
                                .fill(p.surface_bg)
                                .stroke(egui::Stroke::new(1.0, p.profile_border))
                                .corner_radius(10)
                                .inner_margin(egui::Margin::symmetric(12, 6))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 5.0;
                                        ui.label(
                                            RichText::new(compact(count))
                                                .size(13.5)
                                                .strong()
                                                .color(p.text),
                                        );
                                        ui.label(
                                            RichText::new(label).size(12.5).color(p.muted),
                                        );
                                    });
                                })
                                .response
                                .on_hover_text(format!(
                                    "{} {}",
                                    crate::api::format_commas(count),
                                    label
                                ));
                        }
                    });

                    ui.add_space(2.0);
                    let div_rect = ui.allocate_space(egui::vec2(ui.available_width(), 1.0)).1;
                    ui.painter()
                        .rect_filled(div_rect, 0.0, p.profile_border.linear_multiply(0.75));
                    ui.add_space(2.0);

                    self.render_profile_bio(ui, profile, is_self, acts);
                });
            });
        });
    }

    fn render_profile_avatar_box(&mut self, ui: &mut egui::Ui, profile: &FriendProfile) {
        let p = pal(ui);
        let box_size = egui::vec2(156.0, 156.0);
        let (outer_rect, _) = ui.allocate_exact_size(box_size, Sense::hover());
        let center = outer_rect.center();
        let outer_radius = box_size.x * 0.5;

        ui.painter().circle_filled(center, outer_radius, p.surface_bg);

        let img_url = if !profile.user.headshot.is_empty() {
            profile.user.headshot.as_str()
        } else {
            profile.avatar.as_str()
        };

        let inner_size = egui::vec2(148.0, 148.0);
        let inner_rect = egui::Rect::from_center_size(center, inner_size);
        if !img_url.is_empty() {
            egui::Image::new(img_url)
                .corner_radius(74)
                .paint_at(ui, inner_rect);
        } else {
            let initial = profile
                .user
                .display
                .chars()
                .next()
                .unwrap_or('?')
                .to_string();
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                initial,
                egui::FontId::proportional(42.0),
                p.muted,
            );
        }

        ui.painter().circle_stroke(
            center,
            outer_radius - 1.0,
            egui::Stroke::new(2.0, p.profile_border),
        );

        let dot_color = match profile.user.presence {
            1 => Some(BLUE),
            2 => Some(GREEN),
            3 => Some(ORANGE),
            _ => None,
        };
        if let Some(c) = dot_color {
            let dot_pos = center + egui::vec2(52.0, 52.0);
            ui.painter().circle_filled(dot_pos, 11.5, p.profile_card_bg);
            ui.painter().circle_filled(dot_pos, 8.0, c);
        }
    }

    fn render_profile_actions(
        &mut self,
        ui: &mut egui::Ui,
        profile: &FriendProfile,
        is_self: bool,
        acts: &mut Vec<Action>,
    ) {
        let p = pal(ui);
        ui.spacing_mut().item_spacing.x = 6.0;

        let more_btn = egui::Button::new(RichText::new("•••").size(12.0).color(p.text))
            .fill(p.surface_bg)
            .stroke(egui::Stroke::new(1.0, p.border))
            .corner_radius(8)
            .min_size(egui::vec2(32.0, 30.0));
        let more_resp = ui
            .add(more_btn)
            .on_hover_text("More options")
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        more_resp.context_menu(|ui| {
            self.render_profile_more_menu(ui, profile, is_self, acts);
        });
        if more_resp.clicked() {
            egui::Popup::toggle_id(ui.ctx(), more_resp.id.with("menu"));
        }
        egui::Popup::from_response(&more_resp)
            .id(more_resp.id.with("menu"))
            .show(|ui| {
                ui.set_min_width(160.0);
                self.render_profile_more_menu(ui, profile, is_self, acts);
            });

        if is_self {
            let edit_label = if self.profile_editing {
                "Close editor"
            } else {
                "Edit bio"
            };
            let edit_btn = egui::Button::new(RichText::new(edit_label).size(12.5).color(p.text))
                .fill(p.surface_bg)
                .stroke(egui::Stroke::new(1.0, p.border))
                .corner_radius(8)
                .min_size(egui::vec2(0.0, 30.0));
            if ui
                .add(edit_btn)
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                self.profile_editing = !self.profile_editing;
                if self.profile_editing {
                    self.profile_edit_bio = profile.description.clone();
                }
            }

            let avatar_btn = egui::Button::new(
                RichText::new("Customize Avatar")
                    .size(12.5)
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(BLUE)
            .corner_radius(8)
            .min_size(egui::vec2(0.0, 30.0));
            if ui
                .add(avatar_btn)
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                acts.push(Action::Go(Page::Avatar));
            }
        } else {
            let is_friend = self.friends.iter().any(|f| f.id == profile.user.id);
            let is_sent = self.friend_sent.contains(&profile.user.id);

            if is_friend {
                egui::Frame::new()
                    .fill(p.surface_bg)
                    .stroke(egui::Stroke::new(1.0, p.border))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .show(ui, |ui| {
                        ui.label(RichText::new("Friends").size(12.5).strong().color(GREEN));
                    });
            } else if is_sent {
                egui::Frame::new()
                    .fill(p.surface_bg)
                    .stroke(egui::Stroke::new(1.0, p.border))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .show(ui, |ui| {
                        ui.label(RichText::new("Request Sent").size(12.5).color(p.muted));
                    });
            } else {
                let add_btn = egui::Button::new(
                    RichText::new("Add Friend").size(12.5).strong().color(p.text),
                )
                .fill(p.surface_bg)
                .stroke(egui::Stroke::new(1.0, p.border))
                .corner_radius(8)
                .min_size(egui::vec2(0.0, 30.0));
                if ui
                    .add(add_btn)
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    acts.push(Action::SendFriendRequest(
                        profile.user.id,
                        profile.user.name.clone(),
                    ));
                }
            }

            if let Some(pid) = profile.user.place_id {
                let join_btn = egui::Button::image_and_text(
                    egui::Image::new(egui::include_image!("../../assets/icon_play.png"))
                        .fit_to_exact_size(egui::vec2(12.0, 12.0))
                        .tint(Color32::WHITE),
                    RichText::new("Join Game")
                        .size(12.5)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(GREEN)
                .corner_radius(8)
                .min_size(egui::vec2(0.0, 30.0));
                if ui
                    .add(join_btn)
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    acts.push(Action::JoinPlace(pid));
                }
            }
        }
    }

    fn render_profile_more_menu(
        &mut self,
        ui: &mut egui::Ui,
        profile: &FriendProfile,
        is_self: bool,
        acts: &mut Vec<Action>,
    ) {
        if ui.button("Copy profile link").clicked() {
            ui.ctx().copy_text(format!(
                "https://www.roblox.com/users/{}/profile",
                profile.user.id
            ));
            self.status = "Copied profile link!".to_string();
            ui.close();
        }
        if ui.button("Copy user ID").clicked() {
            ui.ctx().copy_text(profile.user.id.to_string());
            self.status = "Copied user ID!".to_string();
            ui.close();
        }
        if ui.button("Open in browser").clicked() {
            acts.push(Action::OpenUrl(format!(
                "https://www.roblox.com/users/{}/profile",
                profile.user.id
            )));
            ui.close();
        }
        if is_self {
            ui.separator();
            if ui.button("Refresh profile").clicked() {
                acts.push(Action::RefreshMyProfile);
                ui.close();
            }
        }
    }

    fn render_profile_bio(
        &mut self,
        ui: &mut egui::Ui,
        profile: &FriendProfile,
        is_self: bool,
        acts: &mut Vec<Action>,
    ) {
        let p = pal(ui);
        if self.profile_editing && is_self {
            ui.vertical(|ui| {
                egui::Frame::new()
                    .fill(p.surface_bg)
                    .stroke(egui::Stroke::new(1.2, p.text_border))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.profile_edit_bio)
                                .frame(egui::Frame::NONE)
                                .desired_width(ui.available_width())
                                .desired_rows(3)
                                .hint_text("Write something about yourself..."),
                        );
                    });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let save_label = if self.profile_saving_bio {
                        "Saving..."
                    } else {
                        "Save bio"
                    };
                    let save_btn = egui::Button::new(
                        RichText::new(save_label)
                            .size(12.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(BLUE)
                    .corner_radius(8);
                    if ui
                        .add_enabled(!self.profile_saving_bio, save_btn)
                        .clicked()
                    {
                        acts.push(Action::SaveBio(self.profile_edit_bio.clone()));
                    }
                    let cancel_btn =
                        egui::Button::new(RichText::new("Cancel").size(12.5).color(p.text))
                            .fill(p.surface_bg)
                            .stroke(egui::Stroke::new(1.0, p.border))
                            .corner_radius(8);
                    if ui.add(cancel_btn).clicked() {
                        self.profile_editing = false;
                    }
                    if let Some(e) = &self.profile_save_err {
                        ui.label(RichText::new(e).size(12.0).color(RED));
                    }
                });
            });
            return;
        }

        if !profile.description.is_empty() {
            let bio = &profile.description;
            let char_count = bio.chars().count();
            let line_count = bio.lines().count();
            let is_long = char_count > 220 || line_count > 3;

            let shown = if is_long && !self.profile_bio_expanded {
                let mut preview: String = bio.lines().take(3).collect::<Vec<_>>().join("\n");
                if preview.chars().count() > 220 {
                    preview = preview.chars().take(217).collect::<String>() + "...";
                } else if line_count > 3 {
                    preview.push_str("...");
                }
                preview
            } else {
                bio.clone()
            };

            ui.add(
                egui::Label::new(RichText::new(shown).size(13.0).color(p.text)).wrap(),
            );

            if is_long {
                let link_text = if self.profile_bio_expanded {
                    "Show less"
                } else {
                    "Show more"
                };
                if ui
                    .add(
                        egui::Label::new(
                            RichText::new(link_text).size(12.0).strong().color(BLUE),
                        )
                        .sense(Sense::click()),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.profile_bio_expanded = !self.profile_bio_expanded;
                }
            }
        } else {
            let empty_msg = if is_self {
                "No bio set yet."
            } else {
                "This user has no bio."
            };
            ui.label(RichText::new(empty_msg).size(12.5).color(p.muted));
        }
    }

    fn render_profile_playing_card(
        &mut self,
        ui: &mut egui::Ui,
        profile: &FriendProfile,
        acts: &mut Vec<Action>,
    ) {
        let Some(game) = &profile.playing_game else {
            return;
        };
        let p = pal(ui);

        ui.horizontal(|ui| {
            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 16.0), Sense::hover());
            ui.painter().circle_filled(dot_rect.center(), 4.5, GREEN);
            ui.label(RichText::new("Currently Playing").size(16.5).strong());
        });
        ui.add_space(6.0);

        card_frame_colored(14, p.profile_card_bg, p.profile_border).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                let t = thumb(
                    ui,
                    self.wide.get(&game.universe_id),
                    egui::vec2(176.0, 100.0),
                    10,
                );
                if t.clicked() {
                    acts.push(Action::Open(game.clone()));
                }

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let title = ui
                        .add(
                            egui::Label::new(RichText::new(&game.name).size(16.5).strong())
                                .sense(Sense::click())
                                .wrap(),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if title.clicked() {
                        acts.push(Action::Open(game.clone()));
                    }
                    ui.label(
                        RichText::new(format!("By {}", game.creator))
                            .size(12.5)
                            .color(p.muted),
                    );
                    if game.playing > 0 {
                        ui.label(
                            RichText::new(format!("{} active", compact(game.playing)))
                                .size(12.0)
                                .color(GREEN),
                        );
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(pid) = profile.user.place_id {
                        let join_btn = egui::Button::image_and_text(
                            egui::Image::new(egui::include_image!("../../assets/icon_play.png"))
                                .fit_to_exact_size(egui::vec2(13.0, 13.0))
                                .tint(Color32::WHITE),
                            RichText::new("Join")
                                .size(13.5)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(GREEN)
                        .corner_radius(10)
                        .min_size(egui::vec2(92.0, 36.0));
                        if ui
                            .add(join_btn)
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            acts.push(Action::JoinPlace(pid));
                        }
                    }
                });
            });
        });
    }

    fn render_profile_wearing_section(
        &mut self,
        ui: &mut egui::Ui,
        profile: &FriendProfile,
        acts: &mut Vec<Action>,
    ) {
        let p = pal(ui);
        let dark = self.settings.dark;
        let rb_gold = if dark {
            Color32::from_rgb(245, 194, 66)
        } else {
            Color32::from_rgb(184, 126, 8)
        };

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(RichText::new("Currently Wearing").size(16.5).strong());
            if !profile.currently_wearing.is_empty() {
                ui.label(
                    RichText::new(format!("{}", profile.currently_wearing.len()))
                        .size(12.5)
                        .color(p.muted),
                );
            }
        });
        ui.add_space(8.0);

        if profile.currently_wearing.is_empty() {
            card_frame_colored(16, p.profile_card_bg, p.profile_border).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("Not wearing any visible items.")
                        .size(13.0)
                        .color(p.muted),
                );
            });
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("wearing_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for item in &profile.currently_wearing {
                        let card_size = egui::vec2(134.0, 192.0);
                        let (card_rect, resp) = ui.allocate_exact_size(card_size, Sense::click());
                        let hovered = resp.hovered();

                        let border_color = if hovered {
                            accent_color(ui)
                        } else {
                            p.profile_border
                        };
                        ui.painter().rect(
                            card_rect,
                            CornerRadius::same(12),
                            p.profile_card_bg,
                            egui::Stroke::new(if hovered { 1.5 } else { 1.0 }, border_color),
                            egui::StrokeKind::Inside,
                        );

                        let thumb_rect = egui::Rect::from_min_size(
                            egui::pos2(card_rect.left() + 12.0, card_rect.top() + 10.0),
                            egui::vec2(110.0, 110.0),
                        );
                        ui.painter()
                            .rect_filled(thumb_rect, CornerRadius::same(8), p.surface_bg);
                        if !item.thumb.is_empty() {
                            egui::Image::new(&item.thumb)
                                .corner_radius(8)
                                .paint_at(ui, thumb_rect.shrink(3.0));
                        }

                        let body_rect = egui::Rect::from_min_max(
                            egui::pos2(card_rect.left() + 10.0, thumb_rect.bottom() + 8.0),
                            egui::pos2(card_rect.right() - 10.0, card_rect.bottom() - 8.0),
                        );
                        let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
                        body_ui.set_width(body_rect.width());
                        body_ui.spacing_mut().item_spacing.y = 2.0;

                        body_ui.add(
                            egui::Label::new(
                                RichText::new(&item.name)
                                    .size(12.5)
                                    .strong()
                                    .color(p.text),
                            )
                            .truncate(),
                        );
                        body_ui.add(
                            egui::Label::new(
                                RichText::new(&item.asset_type_name)
                                    .size(11.0)
                                    .color(p.muted),
                            )
                            .truncate(),
                        );

                        body_ui.add_space(2.0);
                        if let Some(pr) = item.price {
                            body_ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                let (ic_rect, _) =
                                    ui.allocate_exact_size(egui::vec2(12.0, 13.0), Sense::hover());
                                draw_robux_icon(ui.painter(), ic_rect.center(), 12.0, rb_gold);
                                ui.label(
                                    RichText::new(crate::api::format_commas(pr))
                                        .size(11.5)
                                        .strong()
                                        .color(p.text),
                                );
                            });
                        } else {
                            body_ui.label(RichText::new("Free").size(11.5).strong().color(GREEN));
                        }

                        let resp = resp
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_ui(|ui| {
                                ui.set_max_width(240.0);
                                ui.label(RichText::new(&item.name).size(13.0).strong());
                                ui.label(
                                    RichText::new(&item.asset_type_name)
                                        .size(11.5)
                                        .color(p.muted),
                                );
                                if !item.creator_name.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        ui.label(
                                            RichText::new(format!("By {}", item.creator_name))
                                                .size(11.5)
                                                .color(p.muted),
                                        );
                                        if item.creator_verified {
                                            verified_badge_tip(ui, 11.0, "Verified Creator");
                                        }
                                    });
                                }
                                if let Some(pr) = item.price {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        let (ic_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(12.0, 12.0),
                                            Sense::hover(),
                                        );
                                        draw_robux_icon(
                                            ui.painter(),
                                            ic_rect.center(),
                                            12.0,
                                            rb_gold,
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{} Robux",
                                                crate::api::format_commas(pr)
                                            ))
                                            .size(12.0),
                                        );
                                    });
                                } else {
                                    ui.label(RichText::new("Free").size(12.0).color(GREEN));
                                }
                            });

                        if resp.clicked() {
                            acts.push(Action::OpenUrl(format!(
                                "https://www.roblox.com/catalog/{}",
                                item.id
                            )));
                        }
                    }
                });
                ui.add_space(6.0);
            });
    }

    fn render_profile_section_header(ui: &mut egui::Ui, title: &str, count: usize) {
        let p = pal(ui);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(RichText::new(title).size(16.5).strong());
            if count > 0 {
                ui.label(RichText::new(format!("{count}")).size(12.5).color(p.muted));
            }
        });
        ui.add_space(8.0);
    }

    fn render_profile_favorites_section(
        &mut self,
        ui: &mut egui::Ui,
        profile: &FriendProfile,
        acts: &mut Vec<Action>,
    ) {
        let p = pal(ui);
        Self::render_profile_section_header(ui, "Favorites", profile.favorites.len());

        if profile.favorites.is_empty() {
            card_frame_colored(16, p.profile_card_bg, p.profile_border).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("No public favorite experiences visible.")
                        .size(13.0)
                        .color(p.muted),
                );
            });
            return;
        }

        let preview = self.current_card_preview();
        wide_grid_with_width(ui, &profile.favorites, &self.wide, preview, 220.0, acts);
    }

    fn render_profile_badges_section(
        &mut self,
        ui: &mut egui::Ui,
        profile: &FriendProfile,
        acts: &mut Vec<Action>,
    ) {
        let p = pal(ui);
        Self::render_profile_section_header(ui, "Badges", profile.badges.len());

        if profile.badges.is_empty() {
            card_frame_colored(16, p.profile_card_bg, p.profile_border).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("No public badges visible.")
                        .size(13.0)
                        .color(p.muted),
                );
            });
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt(("badges_scroll", profile.user.id))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for badge in &profile.badges {
                        let card_size = egui::vec2(134.0, 168.0);
                        let (card_rect, resp) = ui.allocate_exact_size(card_size, Sense::click());
                        let hovered = resp.hovered();

                        let border_color = if hovered {
                            accent_color(ui)
                        } else {
                            p.profile_border
                        };
                        ui.painter().rect(
                            card_rect,
                            CornerRadius::same(12),
                            p.profile_card_bg,
                            egui::Stroke::new(if hovered { 1.5 } else { 1.0 }, border_color),
                            egui::StrokeKind::Inside,
                        );

                        let icon_center = egui::pos2(card_rect.center().x, card_rect.top() + 58.0);
                        ui.painter().circle_filled(icon_center, 46.0, p.surface_bg);
                        if !badge.icon.is_empty() {
                            let icon_rect =
                                egui::Rect::from_center_size(icon_center, egui::vec2(86.0, 86.0));
                            egui::Image::new(&badge.icon)
                                .corner_radius(43)
                                .paint_at(ui, icon_rect);
                        }
                        ui.painter().circle_stroke(
                            icon_center,
                            46.0,
                            egui::Stroke::new(1.0, p.profile_border),
                        );

                        let body_rect = egui::Rect::from_min_max(
                            egui::pos2(card_rect.left() + 10.0, card_rect.top() + 114.0),
                            egui::pos2(card_rect.right() - 10.0, card_rect.bottom() - 8.0),
                        );
                        let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
                        body_ui.set_width(body_rect.width());
                        body_ui.spacing_mut().item_spacing.y = 3.0;

                        body_ui.vertical_centered(|ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&badge.name)
                                        .size(12.5)
                                        .strong()
                                        .color(p.text),
                                )
                                .truncate(),
                            );
                            let sub = if !badge.awarded_date.is_empty() {
                                badge.awarded_date.clone()
                            } else {
                                "Badge".to_string()
                            };
                            ui.label(RichText::new(sub).size(11.0).color(p.muted));
                        });

                        let resp = resp
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_ui(|ui| {
                                ui.set_max_width(240.0);
                                ui.label(RichText::new(&badge.name).size(13.0).strong());
                                if !badge.description.is_empty() {
                                    ui.add_space(2.0);
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&badge.description)
                                                .size(11.5)
                                                .color(p.muted),
                                        )
                                        .wrap(),
                                    );
                                }
                                if !badge.awarded_date.is_empty() {
                                    ui.add_space(3.0);
                                    ui.label(
                                        RichText::new(format!("Earned {}", badge.awarded_date))
                                            .size(11.0)
                                            .color(p.muted),
                                    );
                                }
                            });

                        if resp.clicked() {
                            acts.push(Action::OpenUrl(format!(
                                "https://www.roblox.com/badges/{}",
                                badge.id
                            )));
                        }
                    }
                });
                ui.add_space(6.0);
            });
    }

    pub fn page_my_profile(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if self.my_profile.is_none() && !self.my_profile_loading {
            self.ensure_my_profile_loaded();
        }

        let loading = self.my_profile_loading;
        let err = self.my_profile_err.clone();
        let profile = self.my_profile.clone();

        if let Some(ref prof) = profile {
            self.render_profile_view(ui, prof, true, acts);
        } else if loading {
            let p = pal(ui);
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label(RichText::new("Loading your profile...").size(14.0).color(p.muted));
            });
        } else if let Some(err) = err {
            let p = pal(ui);
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(RichText::new(&err).size(14.0).color(RED));
                ui.add_space(10.0);
                if ui
                    .button(RichText::new("Retry").size(13.0).color(p.text))
                    .clicked()
                {
                    acts.push(Action::RefreshMyProfile);
                }
            });
        } else {
            let p = pal(ui);
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(RichText::new("No profile loaded.").size(14.0).color(p.muted));
            });
        }
    }

    pub fn page_friend(&mut self, ui: &mut egui::Ui, u: &User, acts: &mut Vec<Action>) {
        let p = pal(ui);
        let back_btn = egui::Button::image_and_text(
            egui::Image::new(egui::include_image!("../../assets/icon_arrow_left.png"))
                .fit_to_exact_size(egui::vec2(14.0, 14.0))
                .tint(p.text),
            RichText::new("Back").size(13.0).color(p.text),
        )
        .fill(p.surface_bg)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(8);
        if ui
            .add(back_btn)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            acts.push(Action::Go(self.back.clone()));
        }
        ui.add_space(10.0);

        let fp = self
            .friend_profile
            .as_ref()
            .filter(|fp| fp.user.id == u.id)
            .cloned();
        if let Some(ref prof) = fp {
            self.render_profile_view(ui, prof, false, acts);
        } else if self.friend_loading {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!("Loading {}'s profile...", u.display))
                        .size(14.0)
                        .color(p.muted),
                );
            });
        } else if let Some(err) = &self.friend_err {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new(err).size(14.0).color(RED));
            });
        }
    }
}

fn render_presence_pill(ui: &mut egui::Ui, profile: &FriendProfile) {
    let p = pal(ui);
    let (dot_color, label) = match profile.user.presence {
        2 => (
            GREEN,
            if profile.user.location.is_empty() {
                "In Game".to_string()
            } else {
                format!("In Game • {}", profile.user.location)
            },
        ),
        3 => (ORANGE, "In Studio".to_string()),
        1 => (BLUE, "Online".to_string()),
        _ => (p.muted, "Offline".to_string()),
    };

    egui::Frame::new()
        .fill(p.surface_bg)
        .stroke(egui::Stroke::new(1.0, p.profile_border))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 12.0), Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 3.8, dot_color);
                ui.label(RichText::new(label).size(11.5).color(p.text));
            });
        });
}
