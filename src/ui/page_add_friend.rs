use std::collections::HashSet;
use eframe::egui::{self, Color32, CornerRadius, RichText, Sense};
use crate::app::RustBlox;
use crate::theme::{
    accent_color, card_frame_colored, is_dark_color, pal, Palette, BLUE, GREEN, ORANGE, RED,
};
use crate::types::{Action, SocialTab, User};
use crate::ui::widgets::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum UserCardMode {
    SearchOrFollow,
    IncomingRequest,
    Friend,
}

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

                // Header & search card
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
                                RichText::new("Friends & Requests")
                                    .size(21.0)
                                    .strong()
                                    .color(p.text),
                            );
                            ui.label(
                                RichText::new(
                                    "Search players, accept pending friend requests, or browse friends, followers, and following.",
                                )
                                .size(12.5)
                                .color(p.muted),
                            );
                        });
                    });

                    ui.add_space(14.0);

                    // Search bar
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

                                        if !self.new_friend.is_empty()
                                            || self.friend_search_results.is_some()
                                        {
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
                                                self.friend_searched.clear();
                                                self.friend_search_results = None;
                                                self.friend_search_cursor = None;
                                                self.friend_searching = false;
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

                ui.add_space(14.0);

                // Show Search results when active, otherwise show Requests / Friends / Followers / Following tabs
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
                    let mut clear_search_results = false;
                    match res {
                        Err(e) => {
                            card_frame_colored(18, p.friends_card_bg, p.friends_border).show(
                                ui,
                                |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.colored_label(RED, format!("Search failed: {e}"));
                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        if ui.button("Retry").clicked() {
                                            acts.push(Action::SearchUsers(
                                                self.friend_searched.clone(),
                                            ));
                                        }
                                        if ui.button("Back to Lists").clicked() {
                                            clear_search_results = true;
                                        }
                                    });
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
                                                "Check the spelling or try their exact @username.",
                                            )
                                            .size(13.0)
                                            .color(p.muted),
                                        );
                                        ui.add_space(10.0);
                                        if ui.button("Back to Lists").clicked() {
                                            clear_search_results = true;
                                        }
                                    });
                                },
                            );
                        }
                        Ok(list) => {
                            let list_clone = list.clone();
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
                                    RichText::new(format!("({} players)", list_clone.len()))
                                        .size(13.0)
                                        .color(p.muted),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("Back to Lists")
                                                        .size(12.0)
                                                        .color(p.text),
                                                )
                                                .fill(p.surface_bg)
                                                .stroke(egui::Stroke::new(1.0, p.friends_border))
                                                .corner_radius(7),
                                            )
                                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            clear_search_results = true;
                                        }
                                    },
                                );
                            });
                            if !clear_search_results {
                                ui.add_space(12.0);
                                self.render_social_grid(
                                    ui,
                                    &list_clone,
                                    UserCardMode::SearchOrFollow,
                                    p,
                                    accent,
                                    accent_fg,
                                    acts,
                                );

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
                                        } else if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("Load More Players")
                                                        .size(13.0)
                                                        .strong(),
                                                )
                                                .fill(p.surface_bg)
                                                .stroke(egui::Stroke::new(1.0, p.friends_border))
                                                .corner_radius(8),
                                            )
                                            .clicked()
                                        {
                                            acts.push(Action::LoadMoreUsers);
                                        }
                                    });
                                }
                            }
                        }
                    }
                    if clear_search_results {
                        self.friend_search_results = None;
                        self.friend_search_cursor = None;
                        self.friend_searched.clear();
                    }
                } else {
                    self.render_social_tabs_panel(ui, p, accent, accent_fg, acts);
                }

                ui.add_space(20.0);
            });
        });
    }

    fn render_social_tabs_panel(
        &mut self,
        ui: &mut egui::Ui,
        p: Palette,
        accent: Color32,
        accent_fg: Color32,
        acts: &mut Vec<Action>,
    ) {
        let active_tab = self.social_tab;
        let is_loading = self.social_loading.contains(&active_tab);

        // Tab switcher + Refresh button
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            for tab in SocialTab::ALL {
                let selected = active_tab == tab;
                let count_suffix = match tab {
                    SocialTab::Friends => format!(" ({})", self.friends.len()),
                    _ => match self.social_lists.get(&tab) {
                        Some(list) => {
                            let more = self
                                .social_cursors
                                .get(&tab)
                                .and_then(|c| c.as_ref())
                                .is_some();
                            if more {
                                format!(" ({}+)", list.len())
                            } else {
                                format!(" ({})", list.len())
                            }
                        }
                        None => String::new(),
                    },
                };
                let label = format!("{}{}", tab.label(), count_suffix);
                let btn = egui::Button::new(
                    RichText::new(label)
                        .size(13.0)
                        .strong()
                        .color(if selected { accent_fg } else { p.text }),
                )
                .fill(if selected { accent } else { p.surface_bg })
                .stroke(if selected {
                    egui::Stroke::NONE
                } else {
                    egui::Stroke::new(1.0, p.friends_border)
                })
                .corner_radius(8)
                .min_size(egui::vec2(0.0, 32.0));

                if ui
                    .add(btn)
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                    && !selected
                {
                    acts.push(Action::SelectSocialTab(tab));
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let rbtn = egui::Button::new(
                    RichText::new(if is_loading { "Refreshing..." } else { "Refresh" })
                        .size(12.5)
                        .color(p.text),
                )
                .fill(p.surface_bg)
                .stroke(egui::Stroke::new(1.0, p.friends_border))
                .corner_radius(8)
                .min_size(egui::vec2(0.0, 32.0));
                if ui
                    .add_enabled(!is_loading, rbtn)
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    acts.push(Action::RefreshSocialTab(active_tab));
                }
            });
        });

        ui.add_space(12.0);

        if is_loading
            && (active_tab == SocialTab::Friends
                || self
                    .social_lists
                    .get(&active_tab)
                    .map(|l| l.is_empty())
                    .unwrap_or(true))
        {
            card_frame_colored(24, p.friends_card_bg, p.friends_border).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new(format!("Loading {}...", active_tab.label()))
                            .size(14.0)
                            .color(p.muted),
                    );
                });
            });
            return;
        }

        if let Some(err) = self.social_errs.get(&active_tab) {
            card_frame_colored(20, p.friends_card_bg, p.friends_border).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.colored_label(
                    RED,
                    format!("Could not load {}: {err}", active_tab.label()),
                );
                ui.add_space(8.0);
                if ui.button("Retry").clicked() {
                    acts.push(Action::RefreshSocialTab(active_tab));
                }
            });
            return;
        }

        let users: Vec<User> = if active_tab == SocialTab::Friends {
            self.friends.clone()
        } else {
            self.social_lists
                .get(&active_tab)
                .cloned()
                .unwrap_or_default()
        };

        if users.is_empty() {
            let empty_msg = match active_tab {
                SocialTab::Requests => "No pending friend requests waiting to accept.",
                SocialTab::Friends => "No friends found yet.",
                SocialTab::Followers => "No followers yet.",
                SocialTab::Following => "You are not following anyone yet.",
            };
            card_frame_colored(28, p.friends_card_bg, p.friends_border).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(empty_msg)
                            .size(15.0)
                            .strong()
                            .color(p.text),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Use the search bar above to find and add Roblox players.")
                            .size(12.5)
                            .color(p.muted),
                    );
                    ui.add_space(6.0);
                });
            });
            return;
        }

        let mode = match active_tab {
            SocialTab::Requests => UserCardMode::IncomingRequest,
            SocialTab::Friends => UserCardMode::Friend,
            SocialTab::Followers | SocialTab::Following => UserCardMode::SearchOrFollow,
        };

        self.render_social_grid(ui, &users, mode, p, accent, accent_fg, acts);

        let has_more = active_tab != SocialTab::Friends
            && self
                .social_cursors
                .get(&active_tab)
                .and_then(|c| c.as_ref())
                .is_some();
        if has_more {
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                if self.social_loading_more.contains(&active_tab) {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Loading more...").color(p.muted));
                    });
                } else if ui
                    .add(
                        egui::Button::new(RichText::new("Load More").size(13.0).strong())
                            .fill(p.surface_bg)
                            .stroke(egui::Stroke::new(1.0, p.friends_border))
                            .corner_radius(8),
                    )
                    .clicked()
                {
                    acts.push(Action::LoadMoreSocialTab(active_tab));
                }
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_social_grid(
        &self,
        ui: &mut egui::Ui,
        list: &[User],
        mode: UserCardMode,
        p: Palette,
        accent: Color32,
        accent_fg: Color32,
        acts: &mut Vec<Action>,
    ) {
        let my_id = self.me.as_ref().map(|prof| prof.user.id).unwrap_or(0);
        let existing_ids: HashSet<u64> = self.friends.iter().map(|f| f.id).collect();
        let incoming_ids: HashSet<u64> = self
            .social_lists
            .get(&SocialTab::Requests)
            .map(|r| r.iter().map(|u| u.id).collect())
            .unwrap_or_default();

        let avail = ui.available_width();
        let gap = 12.0;
        let cols = ((avail + gap) / 310.0).floor().max(1.0) as usize;
        let outer_w =
            ((avail - gap * (cols.saturating_sub(1) as f32)) / cols as f32).max(220.0);
        let inner_w = (outer_w - 28.0).max(180.0);

        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for u in row {
                    let is_self = u.id == my_id;
                    let is_friend = existing_ids.contains(&u.id);
                    let is_incoming = incoming_ids.contains(&u.id);
                    let is_sent = self.friend_sent.contains(&u.id);
                    let is_responding = self.social_responding.contains(&u.id);
                    let is_verified =
                        u.verified || self.friend_search_verified.contains(&u.id);

                    card_frame_colored(13, p.friends_card_bg, p.friends_border).show(ui, |ui| {
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
                                    egui::Stroke::new(1.5, p.friends_border),
                                    egui::StrokeKind::Inside,
                                );
                                if av.clicked() {
                                    acts.push(Action::OpenFriend(u.clone()));
                                }
                                ui.add_space(10.0);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        let disp_text = if u.display.is_empty() {
                                            u.name.as_str()
                                        } else {
                                            u.display.as_str()
                                        };
                                        let dlabel = ui.add(
                                            egui::Label::new(
                                                RichText::new(disp_text).size(14.5).strong(),
                                            )
                                            .truncate()
                                            .sense(Sense::click()),
                                        );
                                        if dlabel.clicked() {
                                            acts.push(Action::OpenFriend(u.clone()));
                                        }
                                        if is_verified {
                                            verified_badge_tip(ui, 13.5, "Verified Account");
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
                                        let (dot_rect, _) = ui.allocate_exact_size(
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
                                                RichText::new(pres_text).size(11.0).color(
                                                    if u.presence > 0 {
                                                        dot_color
                                                    } else {
                                                        p.muted
                                                    },
                                                ),
                                            )
                                            .truncate(),
                                        );
                                    });
                                });
                            });

                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                match mode {
                                    UserCardMode::IncomingRequest => {
                                        let acc_btn = egui::Button::new(
                                            RichText::new(if is_responding {
                                                "Saving..."
                                            } else {
                                                "Accept"
                                            })
                                            .size(11.5)
                                            .strong()
                                            .color(Color32::WHITE),
                                        )
                                        .fill(GREEN)
                                        .corner_radius(7);
                                        if ui
                                            .add_enabled(!is_responding, acc_btn)
                                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            acts.push(Action::AcceptFriendRequest(
                                                u.id,
                                                u.name.clone(),
                                            ));
                                        }

                                        let dec_btn = egui::Button::new(
                                            RichText::new("Decline").size(11.5).color(p.text),
                                        )
                                        .fill(p.surface_bg)
                                        .stroke(egui::Stroke::new(1.0, p.friends_border))
                                        .corner_radius(7);
                                        if ui
                                            .add_enabled(!is_responding, dec_btn)
                                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            acts.push(Action::DeclineFriendRequest(
                                                u.id,
                                                u.name.clone(),
                                            ));
                                        }
                                    }
                                    UserCardMode::Friend => {
                                        if u.presence == 2 {
                                            if let Some(pid) = u.place_id {
                                                let join_btn = egui::Button::new(
                                                    RichText::new("Join")
                                                        .size(11.5)
                                                        .strong()
                                                        .color(Color32::WHITE),
                                                )
                                                .fill(GREEN)
                                                .corner_radius(7);
                                                if ui
                                                    .add(join_btn)
                                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                                    .clicked()
                                                {
                                                    acts.push(Action::JoinPlace(pid));
                                                }
                                            } else {
                                                ui.label(
                                                    RichText::new("Friends \u{2713}")
                                                        .size(12.0)
                                                        .color(GREEN)
                                                        .strong(),
                                                );
                                            }
                                        } else {
                                            ui.label(
                                                RichText::new("Friends \u{2713}")
                                                    .size(12.0)
                                                    .color(GREEN)
                                                    .strong(),
                                            );
                                        }
                                    }
                                    UserCardMode::SearchOrFollow => {
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
                                        } else if is_incoming {
                                            let acc_btn = egui::Button::new(
                                                RichText::new("Accept")
                                                    .size(11.5)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(GREEN)
                                            .corner_radius(7);
                                            if ui
                                                .add_enabled(!is_responding, acc_btn)
                                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                                .clicked()
                                            {
                                                acts.push(Action::AcceptFriendRequest(
                                                    u.id,
                                                    u.name.clone(),
                                                ));
                                            }
                                        } else if is_sent {
                                            let sent_badge = egui::Button::new(
                                                RichText::new("Request Sent \u{2713}")
                                                    .size(11.5)
                                                    .strong()
                                                    .color(GREEN),
                                            )
                                            .fill(p.surface_bg)
                                            .stroke(egui::Stroke::new(1.0, p.friends_border))
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
                                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                                .clicked()
                                            {
                                                acts.push(Action::SendFriendRequest(
                                                    u.id,
                                                    u.name.clone(),
                                                ));
                                            }
                                        }
                                    }
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let prof_label = if mode == UserCardMode::IncomingRequest {
                                            "Profile"
                                        } else {
                                            "View Profile"
                                        };
                                        let prof_btn = egui::Button::new(
                                            RichText::new(prof_label).size(11.5).color(p.text),
                                        )
                                        .fill(p.surface_bg)
                                        .stroke(egui::Stroke::new(1.0, p.friends_border))
                                        .corner_radius(7);
                                        if ui
                                            .add(prof_btn)
                                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            acts.push(Action::OpenFriend(u.clone()));
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
    }
}
