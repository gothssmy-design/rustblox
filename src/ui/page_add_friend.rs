use std::collections::HashSet;
use eframe::egui::{self, Color32, CornerRadius, RichText, Sense};
use crate::app::RustBlox;
use crate::theme::{
    accent_color, card_frame_colored, is_dark_color, pal, BLUE, GREEN, ORANGE, RED,
};
use crate::types::Action;
use crate::ui::widgets::*;

impl RustBlox {
    pub fn page_add_friend(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let p = pal(ui);
        let accent = accent_color(ui);
        let accent_fg = if is_dark_color([accent.r(), accent.g(), accent.b()]) {
            Color32::WHITE
        } else {
            Color32::from_rgb(16, 19, 26)
        };
        let max_w = ui.available_width().min(940.0);

        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            ui.set_max_width(max_w);
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                let mut do_search = false;

                // Unified header & search card
                card_frame_colored(20, p.friends_card_bg, p.friends_border).show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        let back_btn = egui::Button::image_and_text(
                            egui::Image::new(egui::include_image!(
                                "../../assets/icon_arrow_left.png"
                            ))
                            .fit_to_exact_size(egui::vec2(14.0, 14.0))
                            .tint(p.text),
                            RichText::new("Back").size(13.0).strong().color(p.text),
                        )
                        .fill(p.surface_bg)
                        .stroke(egui::Stroke::new(1.0, p.friends_border))
                        .corner_radius(8)
                        .min_size(egui::vec2(0.0, 30.0));
                        if ui
                            .add(back_btn)
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            acts.push(Action::Go(self.back.clone()));
                        }

                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(
                                RichText::new("Find & Add Friends")
                                    .size(21.0)
                                    .strong()
                                    .color(p.text),
                            );
                            ui.label(
                                RichText::new(
                                    "Search Roblox players by display name or username to connect.",
                                )
                                .size(12.5)
                                .color(p.muted),
                            );
                        });
                    });

                    ui.add_space(14.0);

                    // Edge-to-edge search bar with aligned Search button
                    egui::Frame::new()
                        .fill(p.surface_bg)
                        .stroke(egui::Stroke::new(1.2, p.text_border))
                        .corner_radius(10)
                        .inner_margin(egui::Margin::symmetric(12, 6))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                ui.add(
                                    egui::Image::new(egui::include_image!(
                                        "../../assets/icon_search.png"
                                    ))
                                    .fit_to_exact_size(egui::vec2(15.0, 15.0))
                                    .tint(p.muted),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.spacing_mut().item_spacing.x = 8.0;
                                        let sbtn = egui::Button::new(
                                            RichText::new("Search")
                                                .size(13.0)
                                                .strong()
                                                .color(accent_fg),
                                        )
                                        .fill(accent)
                                        .corner_radius(8)
                                        .min_size(egui::vec2(78.0, 30.0));
                                        if ui
                                            .add(sbtn)
                                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            do_search = true;
                                        }

                                        if !self.new_friend.is_empty() {
                                            let clear_btn = ui
                                                .add(
                                                    egui::Label::new(
                                                        RichText::new("\u{00D7}")
                                                            .size(14.0)
                                                            .color(p.muted),
                                                    )
                                                    .sense(Sense::click()),
                                                )
                                                .on_hover_cursor(egui::CursorIcon::PointingHand);
                                            if clear_btn.clicked() {
                                                self.new_friend.clear();
                                            }
                                        }

                                        let edit_w = ui.available_width().max(120.0);
                                        let search_edit = ui.add(
                                            egui::TextEdit::singleline(&mut self.new_friend)
                                                .frame(egui::Frame::NONE)
                                                .hint_text("Enter @username or display name...")
                                                .desired_width(edit_w),
                                        );
                                        if search_edit.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        {
                                            do_search = true;
                                        }
                                    },
                                );
                            });
                        });
                });

                if do_search && !self.new_friend.trim().is_empty() {
                    acts.push(Action::SearchUsers(self.new_friend.clone()));
                }

                ui.add_space(16.0);

                // Results / States
                if self.friend_searching {
                    card_frame_colored(24, p.friends_card_bg, p.friends_border).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                RichText::new(format!(
                                    "Searching Roblox players for \"{}\"...",
                                    self.friend_searched
                                ))
                                .size(14.0)
                                .color(p.muted),
                            );
                        });
                    });
                } else if let Some(res) = &self.friend_search_results {
                    match res {
                        Err(e) => {
                            card_frame_colored(18, p.friends_card_bg, p.friends_border).show(
                                ui,
                                |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.colored_label(RED, format!("Search failed: {e}"));
                                    ui.add_space(6.0);
                                    if ui.button("Retry").clicked() {
                                        acts.push(Action::SearchUsers(
                                            self.friend_searched.clone(),
                                        ));
                                    }
                                },
                            );
                        }
                        Ok(list) if list.is_empty() => {
                            card_frame_colored(28, p.friends_card_bg, p.friends_border).show(
                                ui,
                                |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(6.0);
                                        ui.label(
                                            RichText::new(format!(
                                                "No players found for \"{}\"",
                                                self.friend_searched
                                            ))
                                            .size(16.0)
                                            .strong(),
                                        );
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new(
                                                "Check the spelling or try searching for their exact @username.",
                                            )
                                            .size(13.0)
                                            .color(p.muted),
                                        );
                                        ui.add_space(6.0);
                                    });
                                },
                            );
                        }
                        Ok(list) => {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "Results for \"{}\"",
                                        self.friend_searched
                                    ))
                                    .size(16.5)
                                    .strong(),
                                );
                                ui.label(
                                    RichText::new(format!("({} players)", list.len()))
                                        .size(13.0)
                                        .color(p.muted),
                                );
                            });
                            ui.add_space(12.0);

                            let my_id = self.me.as_ref().map(|prof| prof.user.id).unwrap_or(0);
                            let existing_ids: HashSet<u64> =
                                self.friends.iter().map(|f| f.id).collect();

                            let avail = ui.available_width();
                            let gap = 12.0;
                            let cols = ((avail + gap) / 310.0).floor().max(1.0) as usize;
                            let outer_w = ((avail - gap * (cols.saturating_sub(1) as f32))
                                / cols as f32)
                                .max(220.0);
                            let inner_w = (outer_w - 28.0).max(180.0);

                            for row in list.chunks(cols) {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = gap;
                                    for u in row {
                                        let is_self = u.id == my_id;
                                        let is_friend = existing_ids.contains(&u.id);
                                        let is_sent = self.friend_sent.contains(&u.id);
                                        let is_verified = u.verified
                                            || self.friend_search_verified.contains(&u.id);

                                        card_frame_colored(13, p.friends_card_bg, p.friends_border)
                                            .show(ui, |ui| {
                                                ui.set_width(inner_w);
                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        let av = thumb_styled(
                                                            ui,
                                                            Some(&u.headshot),
                                                            egui::vec2(56.0, 56.0),
                                                            28,
                                                            false,
                                                        )
                                                        .on_hover_text("View profile");
                                                        ui.painter().rect_stroke(
                                                            av.rect,
                                                            CornerRadius::same(28),
                                                            egui::Stroke::new(
                                                                1.5,
                                                                p.friends_border,
                                                            ),
                                                            egui::StrokeKind::Inside,
                                                        );
                                                        if av.clicked() {
                                                            acts.push(Action::OpenFriend(
                                                                u.clone(),
                                                            ));
                                                        }
                                                        ui.add_space(10.0);
                                                        ui.vertical(|ui| {
                                                            ui.spacing_mut().item_spacing.y = 2.0;
                                                            ui.horizontal(|ui| {
                                                                ui.spacing_mut().item_spacing.x =
                                                                    4.0;
                                                                let dlabel = ui.add(
                                                                    egui::Label::new(
                                                                        RichText::new(&u.display)
                                                                            .size(14.5)
                                                                            .strong(),
                                                                    )
                                                                    .truncate()
                                                                    .sense(Sense::click()),
                                                                );
                                                                if dlabel.clicked() {
                                                                    acts.push(Action::OpenFriend(
                                                                        u.clone(),
                                                                    ));
                                                                }
                                                                if is_verified {
                                                                    verified_badge_tip(
                                                                        ui,
                                                                        13.5,
                                                                        "Verified Account",
                                                                    );
                                                                }
                                                            });
                                                            ui.add(
                                                                egui::Label::new(
                                                                    RichText::new(format!(
                                                                        "@{}",
                                                                        u.name
                                                                    ))
                                                                    .size(12.0)
                                                                    .color(p.muted),
                                                                )
                                                                .truncate(),
                                                            );
                                                            let (dot_color, pres_text) = match u
                                                                .presence
                                                            {
                                                                2 => (
                                                                    GREEN,
                                                                    if u.location.is_empty() {
                                                                        "Playing Game".to_string()
                                                                    } else {
                                                                        format!(
                                                                            "Playing {}",
                                                                            u.location
                                                                        )
                                                                    },
                                                                ),
                                                                3 => (
                                                                    ORANGE,
                                                                    "In Studio".to_string(),
                                                                ),
                                                                1 => (BLUE, "Online".to_string()),
                                                                _ => (
                                                                    p.muted,
                                                                    "Offline".to_string(),
                                                                ),
                                                            };
                                                            ui.horizontal(|ui| {
                                                                let (dot_rect, _) = ui
                                                                    .allocate_exact_size(
                                                                        egui::vec2(8.0, 10.0),
                                                                        Sense::hover(),
                                                                    );
                                                                ui.painter().circle_filled(
                                                                    dot_rect.center(),
                                                                    3.5,
                                                                    dot_color,
                                                                );
                                                                ui.add(
                                                                    egui::Label::new(
                                                                        RichText::new(pres_text)
                                                                            .size(11.0)
                                                                            .color(if u.presence > 0 {
                                                                                dot_color
                                                                            } else {
                                                                                p.muted
                                                                            }),
                                                                    )
                                                                    .truncate(),
                                                                );
                                                            });
                                                        });
                                                    });

                                                    ui.add_space(10.0);
                                                    ui.horizontal(|ui| {
                                                        if is_self {
                                                            ui.label(
                                                                RichText::new("You")
                                                                    .size(12.0)
                                                                    .color(p.muted),
                                                            );
                                                        } else if is_friend {
                                                            ui.label(
                                                                RichText::new("Friends \u{2713}")
                                                                    .size(12.0)
                                                                    .color(GREEN)
                                                                    .strong(),
                                                            );
                                                        } else if is_sent {
                                                            let sent_badge = egui::Button::new(
                                                                RichText::new(
                                                                    "Request Sent \u{2713}",
                                                                )
                                                                .size(11.5)
                                                                .strong()
                                                                .color(GREEN),
                                                            )
                                                            .fill(p.surface_bg)
                                                            .stroke(egui::Stroke::new(
                                                                1.0,
                                                                p.friends_border,
                                                            ))
                                                            .corner_radius(7);
                                                            ui.add_enabled(false, sent_badge);
                                                        } else {
                                                            let add_btn = egui::Button::new(
                                                                RichText::new("+ Add Friend")
                                                                    .size(11.5)
                                                                    .strong()
                                                                    .color(accent_fg),
                                                            )
                                                            .fill(accent)
                                                            .corner_radius(7);
                                                            if ui
                                                                .add(add_btn)
                                                                .on_hover_cursor(
                                                                    egui::CursorIcon::PointingHand,
                                                                )
                                                                .clicked()
                                                            {
                                                                acts.push(
                                                                    Action::SendFriendRequest(
                                                                        u.id,
                                                                        u.name.clone(),
                                                                    ),
                                                                );
                                                            }
                                                        }

                                                        ui.with_layout(
                                                            egui::Layout::right_to_left(
                                                                egui::Align::Center,
                                                            ),
                                                            |ui| {
                                                                let prof_btn = egui::Button::new(
                                                                    RichText::new("View Profile")
                                                                        .size(11.5)
                                                                        .color(p.text),
                                                                )
                                                                .fill(p.surface_bg)
                                                                .stroke(egui::Stroke::new(
                                                                    1.0,
                                                                    p.friends_border,
                                                                ))
                                                                .corner_radius(7);
                                                                if ui
                                                                    .add(prof_btn)
                                                                    .on_hover_cursor(
                                                                        egui::CursorIcon::PointingHand,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    acts.push(Action::OpenFriend(
                                                                        u.clone(),
                                                                    ));
                                                                }
                                                            },
                                                        );
                                                    });
                                                });
                                            });
                                    }
                                });
                                ui.add_space(12.0);
                            }

                            if self.friend_search_cursor.is_some() {
                                ui.add_space(8.0);
                                ui.vertical_centered(|ui| {
                                    if self.friend_search_loading_more {
                                        ui.horizontal(|ui| {
                                            ui.spinner();
                                            ui.label(
                                                RichText::new("Loading more players...")
                                                    .color(p.muted),
                                            );
                                        });
                                    } else {
                                        let more_btn = egui::Button::new(
                                            RichText::new("Load More Players").size(13.0).strong(),
                                        )
                                        .fill(p.surface_bg)
                                        .stroke(egui::Stroke::new(1.0, p.friends_border))
                                        .corner_radius(8);
                                        if ui.add(more_btn).clicked() {
                                            acts.push(Action::LoadMoreUsers);
                                        }
                                    }
                                });
                            }
                        }
                    }
                } else {
                    // Clean centered empty state before searching
                    card_frame_colored(32, p.friends_card_bg, p.friends_border).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            let (badge_rect, _) =
                                ui.allocate_exact_size(egui::vec2(48.0, 48.0), Sense::hover());
                            ui.painter().circle_filled(
                                badge_rect.center(),
                                24.0,
                                p.surface_bg,
                            );
                            ui.painter().circle_stroke(
                                badge_rect.center(),
                                24.0,
                                egui::Stroke::new(1.0, p.friends_border),
                            );
                            egui::Image::new(egui::include_image!(
                                "../../assets/icon_search.png"
                            ))
                            .tint(accent)
                            .paint_at(
                                ui,
                                egui::Rect::from_center_size(
                                    badge_rect.center(),
                                    egui::vec2(20.0, 20.0),
                                ),
                            );

                            ui.add_space(12.0);
                            ui.label(
                                RichText::new("Find friends to play with on Roblox")
                                    .size(17.5)
                                    .strong()
                                    .color(p.text),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(
                                    "Search by @username or display name above, or try a popular account:",
                                )
                                .size(13.0)
                                .color(p.muted),
                            );
                            ui.add_space(16.0);

                            // Centered quick suggestion pills
                            ui.allocate_ui_with_layout(
                                egui::vec2(264.0, 30.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;
                                    for suggestion in ["builderman", "Roblox", "noob"] {
                                        let pill = egui::Button::new(
                                            RichText::new(suggestion).size(12.5).color(p.text),
                                        )
                                        .fill(p.surface_bg)
                                        .stroke(egui::Stroke::new(1.0, p.friends_border))
                                        .corner_radius(14)
                                        .min_size(egui::vec2(80.0, 28.0));
                                        if ui
                                            .add(pill)
                                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            self.new_friend = suggestion.to_string();
                                            acts.push(Action::SearchUsers(
                                                suggestion.to_string(),
                                            ));
                                        }
                                    }
                                },
                            );
                            ui.add_space(4.0);
                        });
                    });
                }

                ui.add_space(20.0);
            });
        });
    }
}
