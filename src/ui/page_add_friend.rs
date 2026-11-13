use std::collections::HashSet;
use eframe::egui::{self, Color32, RichText, Sense};
use crate::app::RustBlox;
use crate::theme::{pal, BLUE, GREEN, ORANGE, RED};
use crate::types::Action;
use crate::ui::widgets::*;

impl RustBlox {
    pub fn page_add_friend(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let p = pal(ui);
        let avail_w = ui.available_width();
        let page_w = avail_w.clamp(520.0, 1080.0);
        let pad_x = ((avail_w - page_w) * 0.5).max(0.0);

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(pad_x);
            ui.vertical(|ui| {
                ui.set_width(page_w);
                ui.set_max_width(page_w);

                // Top navigation bar
                ui.horizontal(|ui| {
                    let back_btn = egui::Button::new(RichText::new("â† Back").size(13.5).strong())
                        .corner_radius(8);
                    if ui.add(back_btn).clicked() {
                        acts.push(Action::Go(self.back.clone()));
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Find & Add Friends")
                            .size(24.0)
                            .strong(),
                    );
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Search Roblox players by display name or username to connect and send friend requests.")
                        .size(13.0)
                        .color(p.muted),
                );
                ui.add_space(14.0);

                // Prominent frosted search bar
                let mut do_search = false;
                avatar_glass_frame(ui, 12).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        ui.add(
                            egui::Image::new(egui::include_image!("../../assets/icon_search.png"))
                                .fit_to_exact_size(egui::vec2(16.0, 16.0))
                                .tint(p.muted),
                        );
                        let search_edit = ui.add(
                            egui::TextEdit::singleline(&mut self.new_friend)
                                .frame(egui::Frame::NONE)
                                .hint_text("Search players by username or display name (press Enter)...")
                                .desired_width((ui.available_width() - 150.0).max(120.0)),
                        );
                        if search_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            do_search = true;
                        }
                        if !self.new_friend.is_empty() {
                            let clear_btn = ui.add(
                                egui::Label::new(RichText::new("âœ•").size(13.0).color(p.muted))
                                    .sense(Sense::click()),
                            );
                            if clear_btn.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if clear_btn.clicked() {
                                self.new_friend.clear();
                            }
                        }
                        let sbtn = egui::Button::new(
                            RichText::new("Search").size(13.0).strong().color(Color32::WHITE),
                        )
                        .fill(GREEN)
                        .corner_radius(8);
                        if ui.add(sbtn).clicked() {
                            do_search = true;
                        }
                    });
                });

                if do_search && !self.new_friend.trim().is_empty() {
                    acts.push(Action::SearchUsers(self.new_friend.clone()));
                }

                ui.add_space(16.0);

                // Results / States
                if self.friend_searching {
                    avatar_glass_frame(ui, 24).show(ui, |ui| {
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
                            avatar_glass_frame(ui, 18).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.colored_label(RED, format!("Search failed: {e}"));
                                ui.add_space(6.0);
                                if ui.button("Retry").clicked() {
                                    acts.push(Action::SearchUsers(self.friend_searched.clone()));
                                }
                            });
                        }
                        Ok(list) if list.is_empty() => {
                            avatar_glass_frame(ui, 24).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.vertical_centered(|ui| {
                                    ui.add_space(8.0);
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
                                        RichText::new("Check the spelling or try searching for their exact @username.")
                                            .size(13.0)
                                            .color(p.muted),
                                    );
                                    ui.add_space(8.0);
                                });
                            });
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
                            let cols = ((avail + 12.0) / 330.0).floor().max(1.0) as usize;
                            let card_w = ((avail - (cols.saturating_sub(1) as f32 * 12.0)) / cols as f32).max(240.0);

                            for row in list.chunks(cols) {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 12.0;
                                    for u in row {
                                        let is_self = u.id == my_id;
                                        let is_friend = existing_ids.contains(&u.id);
                                        let is_sent = self.friend_sent.contains(&u.id);
                                        let is_verified = u.verified || self.friend_search_verified.contains(&u.id);

                                        avatar_glass_frame(ui, 12).show(ui, |ui| {
                                            ui.set_width(card_w);
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    let av = thumb_styled(
                                                        ui,
                                                        Some(&u.headshot),
                                                        egui::vec2(60.0, 60.0),
                                                        30,
                                                        true,
                                                    )
                                                    .on_hover_text("View profile");
                                                    if av.clicked() {
                                                        acts.push(Action::OpenFriend(u.clone()));
                                                    }
                                                    ui.add_space(10.0);
                                                    ui.vertical(|ui| {
                                                        ui.horizontal(|ui| {
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
                                                                acts.push(Action::OpenFriend(u.clone()));
                                                            }
                                                            if is_verified {
                                                                verified_badge_tip(ui, 14.0, "Verified Account");
                                                            }
                                                        });
                                                        ui.add(
                                                            egui::Label::new(
                                                                RichText::new(format!("@{}", u.name))
                                                                    .size(12.0)
                                                                    .color(p.muted),
                                                            )
                                                            .truncate(),
                                                        );
                                                        ui.add_space(3.0);
                                                        let (dot_color, pres_text) = match u.presence {
                                                            2 => (
                                                                GREEN,
                                                                if u.location.is_empty() {
                                                                    "Playing Game".to_string()
                                                                } else {
                                                                    format!("Playing {}", u.location)
                                                                },
                                                            ),
                                                            3 => (ORANGE, "In Studio".to_string()),
                                                            1 => (BLUE, "Online".to_string()),
                                                            _ => (p.muted, "Offline".to_string()),
                                                        };
                                                        ui.horizontal(|ui| {
                                                            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), Sense::hover());
                                                            ui.painter().circle_filled(dot_rect.center(), 3.5, dot_color);
                                                            ui.add(
                                                                egui::Label::new(
                                                                    RichText::new(pres_text)
                                                                        .size(11.0)
                                                                        .color(if u.presence > 0 { dot_color } else { p.muted }),
                                                                )
                                                                .truncate(),
                                                            );
                                                        });
                                                    });
                                                });

                                                ui.add_space(10.0);
                                                ui.horizontal(|ui| {
                                                    if is_self {
                                                        ui.label(RichText::new("You").size(12.0).color(p.muted));
                                                    } else if is_friend {
                                                        ui.label(RichText::new("Friends âœ“").size(12.0).color(GREEN).strong());
                                                    } else if is_sent {
                                                        let sent_badge = egui::Button::new(
                                                            RichText::new("Request Sent âœ“").size(11.5).strong().color(GREEN),
                                                        )
                                                        .fill(p.surface_bg)
                                                        .corner_radius(6);
                                                        ui.add_enabled(false, sent_badge);
                                                    } else {
                                                        let add_btn = egui::Button::new(
                                                            RichText::new("+ Add Friend").size(11.5).strong().color(Color32::WHITE),
                                                        )
                                                        .fill(GREEN)
                                                        .corner_radius(6);
                                                        if ui.add(add_btn).clicked() {
                                                            acts.push(Action::SendFriendRequest(u.id, u.name.clone()));
                                                        }
                                                    }

                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        let prof_btn = egui::Button::new(
                                                            RichText::new("View Profile").size(11.5).color(p.muted),
                                                        )
                                                        .fill(p.surface_bg)
                                                        .corner_radius(6);
                                                        if ui.add(prof_btn).clicked() {
                                                            acts.push(Action::OpenFriend(u.clone()));
                                                        }
                                                    });
                                                });
                                            });
                                        });
                                    }
                                });
                                ui.add_space(12.0);
                            }

                            if self.friend_search_cursor.is_some() {
                                ui.add_space(10.0);
                                ui.vertical_centered(|ui| {
                                    if self.friend_search_loading_more {
                                        ui.horizontal(|ui| {
                                            ui.spinner();
                                            ui.label(RichText::new("Loading more players...").color(p.muted));
                                        });
                                    } else {
                                        let more_btn = egui::Button::new(
                                            RichText::new("Load More Players").size(13.0).strong(),
                                        )
                                        .fill(p.surface_bg)
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
                    // Empty state before any search
                    avatar_glass_frame(ui, 24).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new("Find friends to play with on Roblox")
                                    .size(17.0)
                                    .strong(),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("Search by username or display name above to find any Roblox player.")
                                    .size(13.0)
                                    .color(p.muted),
                            );
                            ui.add_space(14.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Suggestions:").size(12.0).color(p.muted));
                                for suggestion in ["builderman", "Roblox", "noob"] {
                                    let pill = egui::Button::new(
                                        RichText::new(suggestion).size(12.0),
                                    )
                                    .fill(p.surface_bg)
                                    .corner_radius(12);
                                    if ui.add(pill).clicked() {
                                        self.new_friend = suggestion.to_string();
                                        acts.push(Action::SearchUsers(suggestion.to_string()));
                                    }
                                }
                            });
                            ui.add_space(10.0);
                        });
                    });
                }

                ui.add_space(20.0);
            });
            ui.add_space(pad_x);
        });
    }

}
