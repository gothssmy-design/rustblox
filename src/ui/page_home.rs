use eframe::egui::{self, Sense, RichText};
use crate::app::RustBlox;
use crate::theme::{pal, RED};
use crate::types::Action;
use crate::ui::widgets::*;

impl RustBlox {
    pub fn page_home(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        // Friends
        let count = self
            .me
            .as_ref()
            .map(|p| p.friends)
            .unwrap_or(self.friends.len() as u64);
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Friends ({count})")).size(19.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let find_btn = ui.add(
                    egui::Label::new(
                        RichText::new("Find Friends  â€º")
                            .size(13.0)
                            .color(pal(ui).muted),
                    )
                    .sense(Sense::click()),
                );
                if find_btn.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if find_btn.clicked() {
                    acts.push(Action::OpenAddFriend);
                }
            });
        });
        ui.add_space(10.0);

        egui::ScrollArea::horizontal()
            .id_salt("friends")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    let add_bubble = bubble(ui, None, "Add Friend", "", 0, true, false)
                        .on_hover_text("Search and add friends on Roblox");
                    if add_bubble.clicked() {
                        acts.push(Action::OpenAddFriend);
                    }
                    for f in &self.friends {
                        let sub = if f.presence == 2 && !f.location.is_empty() {
                            f.location.as_str()
                        } else if f.presence == 1 {
                            "Online"
                        } else if f.presence == 3 {
                            "In Studio"
                        } else {
                            ""
                        };
                        let label = if !f.display.is_empty() {
                            f.display.as_str()
                        } else if !f.name.is_empty() {
                            f.name.as_str()
                        } else {
                            "Player"
                        };
                        let tip = if f.name.is_empty() {
                            format!("User #{}", f.id)
                        } else if f.place_id.is_some() {
                            format!("@{} â€” Playing {}", f.name, f.location)
                        } else {
                            format!("@{}", f.name)
                        };
                        let r = bubble(ui, Some(&f.headshot), label, sub, f.presence, false, f.verified)
                            .on_hover_text(tip);
                        if r.clicked() {
                            acts.push(Action::OpenFriend(f.clone()));
                        }
                        r.context_menu(|ui| {
                            if ui.button("View profile").clicked() {
                                acts.push(Action::OpenFriend(f.clone()));
                                ui.close();
                            }
                            if let Some(pid) = f.place_id {
                                if ui.button("â–¶ Join game").clicked() {
                                    acts.push(Action::JoinPlace(pid));
                                    ui.close();
                                }
                            }
                        });
                    }
                    if self.friends.is_empty() {
                        ui.label(RichText::new("No friends loaded yet.").color(pal(ui).muted));
                    }
                });
            });
        if let Some(e) = &self.friends_err {
            ui.colored_label(RED, format!("Could not load friends: {e}"));
        }

        // Explore sorts
        let syncing = Self::is_sync_active(&self.sorts_sync_started)
            || Self::is_sync_active(&self.saved_sync_started);
        let mut retry = false;
        match &self.sorts {
            None => {
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading games from Roblox...");
                });
            }
            Some(Err(e)) => {
                ui.add_space(20.0);
                ui.colored_label(RED, format!("Could not reach Roblox: {e}"));
                retry = ui.button("Retry").clicked();
            }
            Some(Ok(sorts)) => {
                if let Some(first) = sorts.first() {
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        let link = ui
                            .add(
                                egui::Label::new(
                                    RichText::new(format!("{}  â€º", first.title))
                                        .size(19.0)
                                        .strong(),
                                )
                                .sense(Sense::click()),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if link.clicked() {
                            acts.push(Action::GoChart(0));
                        }
                        sync_refresh_control(
                            ui,
                            syncing,
                            "Topics & played games auto-sync in background (click to sync now)",
                            acts,
                        );
                    });
                    ui.add_space(6.0);
                    let n = first.games.len().min(12);
                    wide_grid(ui, &first.games[..n], &self.wide, self.current_card_preview(), acts);
                }
            }
        }
        if retry {
            self.sorts = None;
            self.force_refresh_games();
        }

        if !self.recent.is_empty() {
            section(ui, "Continue");
            icon_row(ui, "continue", &self.recent, &self.icon, self.current_card_preview(), acts);
        }
        if !self.favs.is_empty() {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("../../assets/icon_bookmark.png"))
                        .fit_to_exact_size(egui::vec2(15.0, 15.0))
                        .tint(pal(ui).text),
                );
                ui.label(RichText::new("Favorites").size(20.0).strong());
            });
            ui.add_space(8.0);
            icon_row(ui, "favorites", &self.favs, &self.icon, self.current_card_preview(), acts);
        }

        if let Some(Ok(sorts)) = &self.sorts {
            for (i, s) in sorts.iter().enumerate().skip(1) {
                if section_link(ui, &s.title) {
                    acts.push(Action::GoChart(i));
                }
                let n = s.games.len().min(24);
                wide_row(ui, &format!("sort{i}"), &s.games[..n], &self.wide, self.current_card_preview(), acts);
            }
        }
        ui.add_space(20.0);
    }

}
