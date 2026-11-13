use eframe::egui::{self, Color32, CornerRadius, RichText, Sense};
use crate::api::format_commas;
use crate::app::RustBlox;
use crate::theme::{compact, pal, GREEN, RED};
use crate::types::{Action, Game, GameTab, ServerItem, ServerSort};
use crate::ui::widgets::*;

impl RustBlox {
    pub fn page_game(&mut self, ui: &mut egui::Ui, g: &Game, acts: &mut Vec<Action>) {
        let p = pal(ui);
        let page_w = ui.available_width().clamp(520.0, 980.0);
        let pad_x = ((ui.available_width() - page_w) * 0.5).max(0.0);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(pad_x);
            ui.vertical(|ui| {
                ui.set_width(page_w);
                ui.set_max_width(page_w);

                ui.horizontal(|ui| {
                    if ui.button("â† Back").clicked() {
                        acts.push(Action::Go(self.back.clone()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.link("View on roblox.com â†—").clicked() {
                            acts.push(Action::OpenUrl(format!(
                                "https://www.roblox.com/games/{}",
                                g.place_id
                            )));
                        }
                    });
                });
                ui.add_space(12.0);

                let ex = self
                    .game_extra
                    .as_ref()
                    .filter(|e| e.universe_id == g.universe_id);
                let media_urls: Vec<String> = ex
                    .map(|e| e.media.clone())
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| self.wide.get(&g.universe_id).cloned().into_iter().collect());
                let media_len = media_urls.len();
                let active_idx = if media_len > 0 {
                    self.game_media_idx % media_len
                } else {
                    0
                };
                let hero_img = media_urls.get(active_idx);

                // Compact Roblox-proportioned Hero Row (max 980px total)
                let media_w = (page_w * 0.59).clamp(320.0, 580.0);
                let media_h = media_w * 9.0 / 16.0;
                let info_w = (page_w - media_w - 16.0).clamp(260.0, 384.0);

                ui.horizontal_top(|ui| {
                    // Left media gallery
                    ui.vertical(|ui| {
                        ui.set_width(media_w);
                        let preview = self.current_card_preview();
                        let (video_tex, is_loading) = match preview.filter(|p| p.universe_id == g.universe_id) {
                            Some(p) => (p.texture, p.is_loading),
                            None => (None, false),
                        };
                        let img_resp = thumb_styled_with_video(
                            ui,
                            hero_img,
                            video_tex,
                            is_loading,
                            egui::vec2(media_w, media_h),
                            12,
                            false,
                        );
                        if img_resp.hovered() {
                            acts.push(Action::HoverGame(g.universe_id));
                        }
                        if media_len > 1 {
                            let y_mid = img_resp.rect.center().y - 16.0;
                            let left_rect = egui::Rect::from_min_size(
                                egui::pos2(img_resp.rect.left() + 10.0, y_mid),
                                egui::vec2(32.0, 32.0),
                            );
                            let right_rect = egui::Rect::from_min_size(
                                egui::pos2(img_resp.rect.right() - 42.0, y_mid),
                                egui::vec2(32.0, 32.0),
                            );
                            for (r, is_left, step) in [
                                (left_rect, true, -1),
                                (right_rect, false, 1),
                            ] {
                                let icon = if is_left {
                                    egui::include_image!("../../assets/icon_arrow_left.png")
                                } else {
                                    egui::include_image!("../../assets/icon_arrow_right.png")
                                };
                                let resp = ui
                                    .put(
                                        r,
                                        egui::Button::image(
                                            egui::Image::new(icon)
                                                .fit_to_exact_size(egui::vec2(16.0, 16.0))
                                                .tint(Color32::WHITE),
                                        )
                                        .fill(Color32::from_rgba_unmultiplied(15, 18, 24, 190))
                                        .corner_radius(16),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                                if resp.clicked() {
                                    acts.push(Action::GameMediaStep(step));
                                }
                            }

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.add_space(((media_w - media_len as f32 * 14.0) * 0.5).max(0.0));
                                for i in 0..media_len {
                                    let is_cur = i == active_idx;
                                    let (dot_rect, dot_resp) = ui.allocate_exact_size(
                                        egui::vec2(if is_cur { 18.0 } else { 8.0 }, 8.0),
                                        Sense::click(),
                                    );
                                    ui.painter().rect_filled(
                                        dot_rect,
                                        CornerRadius::same(4),
                                        if is_cur { GREEN } else { p.border },
                                    );
                                    if dot_resp
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        acts.push(Action::GameMediaSelect(i));
                                    }
                                }
                            });
                        }
                    });

                    ui.add_space(16.0);

                    // Right game info & Play / Favorite / Vote box (compact, never oversized)
                    avatar_glass_frame(ui, 18).show(ui, |ui| {
                        let card_inner = (info_w - 38.0).max(220.0);
                        ui.set_width(card_inner);
                        ui.set_max_width(card_inner);
                        ui.set_min_height((media_h - 38.0).max(200.0));
                        ui.vertical(|ui| {
                            ui.set_width(card_inner);
                            ui.add(
                                egui::Label::new(RichText::new(&g.name).size(24.0).strong()).wrap(),
                            );
                            ui.add_space(3.0);
                            if !g.creator.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("By {}", g.creator))
                                            .size(13.0)
                                            .color(p.muted),
                                    );
                                    if ex.is_some_and(|e| e.verified_creator) || g.creator_verified {
                                        verified_badge_tip(ui, 14.0, "Verified Creator");
                                    }
                                });
                            }
                            if let Some(m) =
                                ex.map(|e| e.maturity_short.as_str()).filter(|s| !s.is_empty())
                            {
                                ui.add_space(2.0);
                                ui.label(RichText::new(m).size(12.0).color(p.muted));
                            }

                            ui.add_space(16.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(card_inner, 44.0),
                                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                                |ui| {
                                    let play = egui::Button::image_and_text(
                                        egui::Image::new(egui::include_image!(
                                            "../../assets/icon_play.png"
                                        ))
                                        .fit_to_exact_size(egui::vec2(18.0, 18.0))
                                        .tint(Color32::WHITE),
                                        RichText::new("Play")
                                            .size(17.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(GREEN)
                                    .corner_radius(10);
                                    if ui.add(play).clicked() {
                                        acts.push(Action::Play(g.clone()));
                                    }
                                },
                            );
                            ui.add_space(10.0);

                            // Action & Vote strip (Favorite + Upvote/Downvote + Ratio Bar)
                            let fav = ex
                                .and_then(|e| e.is_favorited_api)
                                .unwrap_or_else(|| self.fav_ids.contains(&g.place_id));
                            let user_vote = ex.and_then(|e| e.user_vote);
                            let can_vote = ex.is_some_and(|e| e.can_vote);
                            let up_votes = g.up;
                            let down_votes = g.down;

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let third_w = ((card_inner - 12.0) / 3.0).max(58.0);
                                let fav_btn = egui::Button::image_and_text(
                                    egui::Image::new(egui::include_image!(
                                        "../../assets/icon_bookmark.png"
                                    ))
                                    .fit_to_exact_size(egui::vec2(13.0, 13.0))
                                    .tint(if fav { RED } else { p.muted }),
                                    RichText::new(if fav { "Favorited" } else { "Favorite" })
                                        .size(12.0)
                                        .color(if fav { RED } else { p.text }),
                                )
                                .fill(p.surface_bg)
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    if fav { RED } else { p.border },
                                ))
                                .corner_radius(8)
                                .min_size(egui::vec2(third_w, 32.0));
                                if ui.add(fav_btn).clicked() {
                                    acts.push(Action::ToggleFav(g.clone()));
                                }

                                // Upvote button
                                let up_sel = user_vote == Some(true);
                                let up_btn = egui::Button::image_and_text(
                                    egui::Image::new(egui::include_image!(
                                        "../../assets/icon_like.png"
                                    ))
                                    .fit_to_exact_size(egui::vec2(13.0, 13.0))
                                    .tint(if up_sel { Color32::WHITE } else { p.muted }),
                                    RichText::new(compact(up_votes))
                                        .size(12.0)
                                        .color(if up_sel { Color32::WHITE } else { p.text }),
                                )
                                .fill(if up_sel { GREEN } else { p.surface_bg })
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    if up_sel { GREEN } else { p.border },
                                ))
                                .corner_radius(8)
                                .min_size(egui::vec2(third_w, 32.0));
                                let up_resp = ui.add(up_btn).on_hover_text(if can_vote {
                                    "Upvote on Roblox"
                                } else {
                                    "Play or sign in to vote"
                                });
                                if up_resp.clicked() {
                                    let next = if up_sel { None } else { Some(true) };
                                    acts.push(Action::VoteGame(g.universe_id, next));
                                }

                                // Downvote button
                                let down_sel = user_vote == Some(false);
                                let down_btn = egui::Button::image_and_text(
                                    egui::Image::new(egui::include_image!(
                                        "../../assets/icon_dislike.png"
                                    ))
                                    .fit_to_exact_size(egui::vec2(13.0, 13.0))
                                    .tint(if down_sel { Color32::WHITE } else { p.muted }),
                                    RichText::new(compact(down_votes))
                                        .size(12.0)
                                        .color(if down_sel { Color32::WHITE } else { p.text }),
                                )
                                .fill(if down_sel { RED } else { p.surface_bg })
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    if down_sel { RED } else { p.border },
                                ))
                                .corner_radius(8)
                                .min_size(egui::vec2(third_w, 32.0));
                                let down_resp = ui.add(down_btn).on_hover_text(if can_vote {
                                    "Downvote on Roblox"
                                } else {
                                    "Play or sign in to vote"
                                });
                                if down_resp.clicked() {
                                    let next = if down_sel { None } else { Some(false) };
                                    acts.push(Action::VoteGame(g.universe_id, next));
                                }
                            });

                            let total_votes = up_votes + down_votes;
                            if total_votes > 0 {
                                ui.add_space(6.0);
                                let ratio = up_votes as f32 / total_votes as f32;
                                let (bar_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(card_inner, 4.0),
                                    Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    bar_rect,
                                    CornerRadius::same(2),
                                    p.border,
                                );
                                let up_rect = egui::Rect::from_min_max(
                                    bar_rect.min,
                                    egui::pos2(
                                        bar_rect.left() + bar_rect.width() * ratio,
                                        bar_rect.max.y,
                                    ),
                                );
                                ui.painter().rect_filled(up_rect, CornerRadius::same(2), GREEN);
                            }
                        });
                    });
                });

                ui.add_space(18.0);

                // About | Store | Servers underline tab bar matching Roblox page
                let tabs = [GameTab::About, GameTab::Store, GameTab::Servers];
                let tab_w = (page_w / 3.0).floor().max(110.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for tab in tabs {
                let selected = self.game_tab == tab;
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(tab_w, 36.0), Sense::click());
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.hovered() && !selected {
                    ui.painter().rect_filled(rect, CornerRadius::ZERO, p.hover_bg);
                }
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    tab.label(),
                    egui::FontId::proportional(14.5),
                    if selected { p.text } else { p.muted },
                );
                if selected {
                    let line = egui::Rect::from_min_max(
                        egui::pos2(rect.left(), rect.bottom() - 2.5),
                        egui::pos2(rect.right(), rect.bottom()),
                    );
                    ui.painter().rect_filled(line, CornerRadius::ZERO, GREEN);
                }
                if resp.clicked() && !selected {
                    acts.push(Action::SelectGameTab(tab));
                }
            }
        });
        ui.add_space(12.0);

        match self.game_tab {
            GameTab::About => {
                avatar_glass_frame(ui, 18).show(ui, |ui| {
                    ui.set_width((ui.available_width() - 4.0).max(260.0));
                    ui.label(RichText::new("Description").size(17.0).strong());
                    ui.add_space(8.0);
                    if g.description.trim().is_empty() {
                        ui.label(
                            RichText::new("No description provided.").color(p.muted),
                        );
                    } else {
                        ui.add(egui::Label::new(RichText::new(&g.description).size(13.5)).wrap());
                    }

                    if let Some(ex) = ex {
                        if !ex.maturity_short.is_empty() {
                            ui.add_space(14.0);
                            egui::Frame::new()
                                .fill(p.surface_bg)
                                .stroke(egui::Stroke::new(1.0, p.border))
                                .corner_radius(10)
                                .inner_margin(egui::Margin::symmetric(14, 10))
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            RichText::new(&ex.maturity_short)
                                                .size(13.0)
                                                .strong(),
                                        );
                                        if !ex.maturity_detail.is_empty() {
                                            ui.label(
                                                RichText::new(&ex.maturity_detail)
                                                    .size(13.0)
                                                    .color(p.muted),
                                            );
                                        }
                                    });
                                });
                        }
                    }

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(14.0);

                    // Full 10-field live Roblox stats row
                    let voice_str = match ex.and_then(|e| e.voice_chat) {
                        Some(true) => "Supported".to_string(),
                        Some(false) => "Not Supported".to_string(),
                        None => "--".to_string(),
                    };
                    let cam_str = match ex.and_then(|e| e.camera) {
                        Some(true) => "Supported".to_string(),
                        Some(false) => "Not Supported".to_string(),
                        None => "--".to_string(),
                    };
                    let stats = [
                        ("Active", compact(g.playing)),
                        (
                            "Favorites",
                            ex.map(|e| format_commas(e.favorites))
                                .unwrap_or_else(|| "--".to_string()),
                        ),
                        (
                            "Visits",
                            ex.map(|e| format!("{}+", compact(e.visits)))
                                .unwrap_or_else(|| "--".to_string()),
                        ),
                        ("Voice Chat", voice_str),
                        ("Camera", cam_str),
                        (
                            "Created",
                            ex.map(|e| e.created.clone())
                                .unwrap_or_else(|| "--".to_string()),
                        ),
                        (
                            "Updated",
                            ex.map(|e| e.updated.clone())
                                .unwrap_or_else(|| "--".to_string()),
                        ),
                        (
                            "Server Size",
                            ex.map(|e| e.max_players.to_string())
                                .unwrap_or_else(|| "--".to_string()),
                        ),
                        (
                            "Genre",
                            ex.map(|e| e.genre.clone())
                                .unwrap_or_else(|| "--".to_string()),
                        ),
                        (
                            "Subgenre",
                            ex.map(|e| {
                                if e.subgenre.is_empty() {
                                    "--".to_string()
                                } else {
                                    e.subgenre.clone()
                                }
                            })
                            .unwrap_or_else(|| "--".to_string()),
                        ),
                    ];
                    let avail = ui.available_width().max(260.0);
                    let cols = ((avail / 108.0).floor() as usize).clamp(2, 10);
                    let cell_w = ((avail - (cols as f32 - 1.0) * 10.0) / cols as f32).floor();
                    for chunk in stats.chunks(cols) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            for (k, v) in chunk {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(cell_w, 46.0),
                                    egui::Layout::top_down(egui::Align::Center),
                                    |ui| {
                                        ui.label(RichText::new(*k).size(11.5).color(p.muted));
                                        ui.add_space(2.0);
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(v.as_str()).size(13.5).strong(),
                                            )
                                            .truncate(),
                                        );
                                    },
                                );
                            }
                        });
                        ui.add_space(8.0);
                    }
                });
            }
            GameTab::Store => {
                avatar_glass_frame(ui, 18).show(ui, |ui| {
                    ui.set_width((ui.available_width() - 4.0).max(260.0));
                    ui.label(RichText::new("Passes").size(17.0).strong());
                    ui.add_space(10.0);
                    if self.game_passes_loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading passes from Roblox...");
                        });
                    } else if let Some(err) = &self.game_passes_err {
                        ui.colored_label(RED, format!("Could not load passes: {err}"));
                    } else if let Some(ex) = ex {
                        if ex.passes.is_empty() {
                            ui.label(
                                RichText::new("This experience has no game passes.")
                                    .color(p.muted),
                            );
                        } else {
                            let avail = (ui.available_width() - 10.0).max(220.0);
                            let gap = 12.0;
                            let cols = (((avail + gap) / 168.0).floor() as usize).max(2);
                            let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32)
                                .floor()
                                .clamp(130.0, 190.0);
                            let inner_w = card_w - 20.0;
                            for row in ex.passes.chunks(cols) {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = gap;
                                    for pass in row {
                                        egui::Frame::new()
                                            .fill(p.surface_bg)
                                            .stroke(egui::Stroke::new(
                                                1.0,
                                                if pass.is_owned { GREEN } else { p.border },
                                            ))
                                            .corner_radius(10)
                                            .inner_margin(8)
                                            .show(ui, |ui| {
                                                ui.set_width(inner_w);
                                                ui.vertical_centered(|ui| {
                                                    let u = (!pass.thumb.is_empty())
                                                        .then_some(&pass.thumb);
                                                    thumb(
                                                        ui,
                                                        u,
                                                        egui::vec2(inner_w, inner_w),
                                                        8,
                                                    )
                                                    .on_hover_text(&pass.description);
                                                    ui.add_space(6.0);
                                                    ui.add_sized(
                                                        [inner_w, 18.0],
                                                        egui::Label::new(
                                                            RichText::new(&pass.name)
                                                                .size(13.0)
                                                                .strong(),
                                                        )
                                                        .truncate(),
                                                    )
                                                    .on_hover_text(&pass.description);
                                                    ui.add_space(2.0);
                                                    let price_txt = if pass.is_owned {
                                                        "Owned âœ“".to_string()
                                                    } else if !pass.is_for_sale {
                                                        "Off Sale".to_string()
                                                    } else if let Some(pr) = pass.price {
                                                        format!("R$ {}", format_commas(pr))
                                                    } else {
                                                        "Off Sale".to_string()
                                                    };
                                                    ui.label(
                                                        RichText::new(price_txt).size(12.5).color(
                                                            if pass.is_owned { GREEN } else { p.muted },
                                                        ),
                                                    );
                                                    ui.add_space(5.0);
                                                    let btn = egui::Button::new(
                                                        RichText::new(if pass.is_owned {
                                                            "View Pass â†—"
                                                        } else {
                                                            "Buy Pass â†—"
                                                        })
                                                        .size(12.0)
                                                        .strong()
                                                        .color(Color32::WHITE),
                                                    )
                                                    .fill(if pass.is_owned { p.border } else { GREEN })
                                                    .corner_radius(7)
                                                    .min_size(egui::vec2(inner_w, 26.0));
                                                    if ui.add(btn).clicked() {
                                                        acts.push(Action::OpenUrl(format!(
                                                            "https://www.roblox.com/game-pass/{}",
                                                            pass.id
                                                        )));
                                                    }
                                                });
                                            });
                                    }
                                });
                                ui.add_space(gap);
                            }
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading store...");
                        });
                    }
                });
            }
            GameTab::Servers => {
                avatar_glass_frame(ui, 18).show(ui, |ui| {
                    ui.set_width((ui.available_width() - 4.0).max(260.0));

                    let (total_count, visible_servers) = if let Some(ex) = ex {
                        let total = ex.servers.len();
                        let mut list: Vec<(usize, &ServerItem)> = ex.servers
                            .iter()
                            .enumerate()
                            .filter(|(_, s)| {
                                if self.server_exclude_full {
                                    s.playing < s.max_players
                                } else {
                                    true
                                }
                            })
                            .collect();

                        match self.server_sort {
                            ServerSort::Default => {}
                            ServerSort::LowestPing => {
                                list.sort_by(|(_, a), (_, b)| {
                                    let pa = a.ping.unwrap_or(u64::MAX);
                                    let pb = b.ping.unwrap_or(u64::MAX);
                                    pa.cmp(&pb)
                                });
                            }
                            ServerSort::FewestPlayers => {
                                list.sort_by(|(_, a), (_, b)| {
                                    a.playing.cmp(&b.playing)
                                });
                            }
                            ServerSort::MostPlayers => {
                                list.sort_by(|(_, a), (_, b)| {
                                    b.playing.cmp(&a.playing)
                                });
                            }
                            ServerSort::HighestFps => {
                                list.sort_by(|(_, a), (_, b)| {
                                    let fa = a.fps.unwrap_or(0.0);
                                    let fb = b.fps.unwrap_or(0.0);
                                    fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
                                });
                            }
                        }
                        (total, list)
                    } else {
                        (0, Vec::new())
                    };

                    let title_text = if total_count > 0 && self.server_exclude_full {
                        format!("Public Servers ({}/{} open)", visible_servers.len(), total_count)
                    } else if total_count > 0 {
                        format!("Public Servers ({total_count})")
                    } else {
                        "Public Servers".to_string()
                    };

                    let avail_w = ui.available_width();
                    let is_wide = avail_w >= 560.0;

                    if is_wide {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&title_text).size(17.0).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("â†» Refresh").clicked() {
                                    acts.push(Action::RefreshServers(g.universe_id, g.place_id));
                                }

                                ui.add_space(6.0);

                                let exclude_full = self.server_exclude_full;
                                let pill_txt = if exclude_full { "âœ“ Open Slots Only" } else { "Exclude Full" };
                                let pill_bg = if exclude_full {
                                    if ui.visuals().dark_mode {
                                        Color32::from_rgba_unmultiplied(34, 197, 94, 38)
                                    } else {
                                        Color32::from_rgba_unmultiplied(34, 197, 94, 28)
                                    }
                                } else {
                                    p.card_bg
                                };
                                let pill_stroke = if exclude_full {
                                    egui::Stroke::new(1.0, GREEN)
                                } else {
                                    egui::Stroke::new(1.0, p.border)
                                };
                                let pill_color = if exclude_full {
                                    if ui.visuals().dark_mode { Color32::from_rgb(134, 239, 172) } else { GREEN }
                                } else {
                                    p.muted
                                };

                                let pill_btn = egui::Button::new(
                                    RichText::new(pill_txt).size(12.0).strong().color(pill_color),
                                )
                                .fill(pill_bg)
                                .stroke(pill_stroke)
                                .corner_radius(6);
                                if ui
                                    .add(pill_btn)
                                    .on_hover_text("Hide full servers (12/12) to show only joinable games")
                                    .clicked()
                                {
                                    self.server_exclude_full = !self.server_exclude_full;
                                }

                                ui.add_space(6.0);

                                egui::ComboBox::from_id_salt("servers_sort_combo")
                                    .selected_text(self.server_sort.label())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.server_sort, ServerSort::Default, "Default Order");
                                        ui.selectable_value(&mut self.server_sort, ServerSort::LowestPing, "âš¡ Lowest Ping");
                                        ui.selectable_value(&mut self.server_sort, ServerSort::FewestPlayers, "ðŸ‘¤ Fewest Players (Small)");
                                        ui.selectable_value(&mut self.server_sort, ServerSort::MostPlayers, "ðŸ‘¥ Most Players (Full)");
                                        ui.selectable_value(&mut self.server_sort, ServerSort::HighestFps, "ðŸŽ¯ Best FPS");
                                    });

                                ui.add_space(4.0);

                                let sort_icon = egui::Image::new(egui::include_image!("../../assets/icon_sort.png"))
                                    .fit_to_exact_size(egui::vec2(14.0, 14.0))
                                    .tint(p.muted);
                                ui.add(sort_icon);
                            });
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&title_text).size(17.0).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("â†» Refresh").clicked() {
                                    acts.push(Action::RefreshServers(g.universe_id, g.place_id));
                                }
                            });
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let sort_icon = egui::Image::new(egui::include_image!("../../assets/icon_sort.png"))
                                .fit_to_exact_size(egui::vec2(14.0, 14.0))
                                .tint(p.muted);
                            ui.add(sort_icon);

                            egui::ComboBox::from_id_salt("servers_sort_combo_narrow")
                                .selected_text(self.server_sort.label())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.server_sort, ServerSort::Default, "Default Order");
                                    ui.selectable_value(&mut self.server_sort, ServerSort::LowestPing, "âš¡ Lowest Ping");
                                    ui.selectable_value(&mut self.server_sort, ServerSort::FewestPlayers, "ðŸ‘¤ Fewest Players (Small)");
                                    ui.selectable_value(&mut self.server_sort, ServerSort::MostPlayers, "ðŸ‘¥ Most Players (Full)");
                                    ui.selectable_value(&mut self.server_sort, ServerSort::HighestFps, "ðŸŽ¯ Best FPS");
                                });

                            ui.add_space(6.0);

                            let exclude_full = self.server_exclude_full;
                            let pill_txt = if exclude_full { "âœ“ Open Slots Only" } else { "Exclude Full" };
                            let pill_bg = if exclude_full {
                                if ui.visuals().dark_mode {
                                    Color32::from_rgba_unmultiplied(34, 197, 94, 38)
                                } else {
                                    Color32::from_rgba_unmultiplied(34, 197, 94, 28)
                                }
                            } else {
                                p.card_bg
                            };
                            let pill_stroke = if exclude_full {
                                egui::Stroke::new(1.0, GREEN)
                            } else {
                                egui::Stroke::new(1.0, p.border)
                            };
                            let pill_color = if exclude_full {
                                if ui.visuals().dark_mode { Color32::from_rgb(134, 239, 172) } else { GREEN }
                            } else {
                                p.muted
                            };

                            let pill_btn = egui::Button::new(
                                RichText::new(pill_txt).size(12.0).strong().color(pill_color),
                            )
                            .fill(pill_bg)
                            .stroke(pill_stroke)
                            .corner_radius(6);
                            if ui
                                .add(pill_btn)
                                .on_hover_text("Hide full servers (12/12) to show only joinable games")
                                .clicked()
                            {
                                self.server_exclude_full = !self.server_exclude_full;
                            }
                        });
                    }
                    ui.add_space(10.0);

                    if let Some(err) = &self.game_servers_err {
                        ui.colored_label(RED, format!("Could not load servers: {err}"));
                    } else if let Some(ex) = ex {
                        if total_count == 0 && !self.game_servers_loading {
                            ui.label(
                                RichText::new("No active public servers found right now.")
                                    .color(p.muted),
                            );
                        } else if visible_servers.is_empty() && !self.game_servers_loading {
                            egui::Frame::new()
                                .fill(p.surface_bg)
                                .stroke(egui::Stroke::new(1.0, p.border))
                                .corner_radius(10)
                                .inner_margin(16)
                                .show(ui, |ui| {
                                    ui.set_width((ui.available_width() - 8.0).max(200.0));
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new(format!(
                                                "All {total_count} loaded servers are currently full."
                                            ))
                                            .size(14.0)
                                            .strong(),
                                        );
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new("Turn off 'Exclude Full' or load more servers to find an open slot.")
                                                .size(12.5)
                                                .color(p.muted),
                                        );
                                        ui.add_space(10.0);
                                        ui.horizontal(|ui| {
                                            if ui.button("Show Full Servers").clicked() {
                                                self.server_exclude_full = false;
                                            }
                                            if ex.servers_cursor.is_some() && !self.game_servers_loading {
                                                if ui.button("Load More Servers").clicked() {
                                                    acts.push(Action::LoadMoreServers(g.universe_id, g.place_id));
                                                }
                                            }
                                        });
                                        ui.add_space(4.0);
                                    });
                                });
                        } else {
                            let avail = (ui.available_width() - 10.0).max(240.0);
                            let gap = 10.0;
                            let cols = ((avail / 250.0).floor() as usize).clamp(1, 4);
                            let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32)
                                .floor()
                                .max(200.0);
                            let inner_w = card_w - 24.0;
                            for row in visible_servers.chunks(cols) {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = gap;
                                    for &(orig_idx, srv) in row {
                                        let num = orig_idx + 1;
                                        let is_full = srv.playing >= srv.max_players;
                                        let open_slots = srv.max_players.saturating_sub(srv.playing);

                                        egui::Frame::new()
                                            .fill(p.surface_bg)
                                            .stroke(egui::Stroke::new(1.0, p.border))
                                            .corner_radius(10)
                                            .inner_margin(10)
                                            .show(ui, |ui| {
                                                ui.set_width(inner_w);
                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        let title_resp = ui.label(
                                                            RichText::new(format!(
                                                                "Server #{num}"
                                                            ))
                                                            .size(14.0)
                                                            .strong(),
                                                        );
                                                        title_resp.on_hover_text(format!("Instance #{num}\nServer ID: {}", srv.id));
                                                        ui.with_layout(
                                                            egui::Layout::right_to_left(
                                                                egui::Align::Center,
                                                            ),
                                                            |ui| {
                                                                if is_full {
                                                                    ui.label(
                                                                        RichText::new(format!(
                                                                            "{} / {}",
                                                                            srv.playing, srv.max_players
                                                                        ))
                                                                        .size(12.5)
                                                                        .color(p.muted)
                                                                        .strong(),
                                                                    );
                                                                } else {
                                                                    let count_color = if open_slots <= 2 {
                                                                        Color32::from_rgb(234, 179, 8)
                                                                    } else {
                                                                        GREEN
                                                                    };
                                                                    ui.label(
                                                                        RichText::new(format!(
                                                                            "{} / {}",
                                                                            srv.playing, srv.max_players
                                                                        ))
                                                                        .size(12.5)
                                                                        .color(count_color)
                                                                        .strong(),
                                                                    );
                                                                }
                                                            },
                                                        );
                                                    });
                                                    ui.add_space(4.0);
                                                    let (dot_color, ping_txt) = match srv.ping {
                                                        Some(ms) if ms < 60 => (GREEN, format!("{ms} ms")),
                                                        Some(ms) if ms < 120 => (Color32::from_rgb(234, 179, 8), format!("{ms} ms")),
                                                        Some(ms) => (Color32::from_rgb(239, 68, 68), format!("{ms} ms")),
                                                        None => (p.muted, "--".to_string()),
                                                    };
                                                    let fps_txt = srv
                                                        .fps
                                                        .map(|f| format!("{f:.0} FPS"))
                                                        .unwrap_or_else(|| "--".to_string());
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.label(RichText::new("â—").size(8.0).color(dot_color));
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "Ping: {ping_txt}  â€¢  {fps_txt}"
                                                            ))
                                                            .size(12.0)
                                                            .color(p.muted),
                                                        );
                                                    });
                                                    ui.add_space(6.0);
                                                    let fill_ratio = if srv.max_players > 0 {
                                                        (srv.playing as f32
                                                            / srv.max_players as f32)
                                                            .clamp(0.0, 1.0)
                                                    } else {
                                                        0.0
                                                    };
                                                    let (bar_r, _) = ui.allocate_exact_size(
                                                        egui::vec2(inner_w, 5.0),
                                                        Sense::hover(),
                                                    );
                                                    ui.painter().rect_filled(
                                                        bar_r,
                                                        CornerRadius::same(2),
                                                        p.border,
                                                    );
                                                    let fill_r = egui::Rect::from_min_max(
                                                        bar_r.min,
                                                        egui::pos2(
                                                            bar_r.left()
                                                                + bar_r.width() * fill_ratio,
                                                            bar_r.max.y,
                                                        ),
                                                    );
                                                    let bar_color = if is_full {
                                                        Color32::from_rgb(156, 163, 175)
                                                    } else {
                                                        GREEN
                                                    };
                                                    ui.painter().rect_filled(
                                                        fill_r,
                                                        CornerRadius::same(2),
                                                        bar_color,
                                                    );
                                                    ui.add_space(8.0);
                                                    let (btn_txt, btn_fill, btn_color) = if is_full {
                                                        ("Join (Full)", p.card_bg, p.muted)
                                                    } else {
                                                        ("Join Server", GREEN, Color32::WHITE)
                                                    };
                                                    let join_btn = egui::Button::new(
                                                        RichText::new(btn_txt)
                                                            .size(12.5)
                                                            .strong()
                                                            .color(btn_color),
                                                    )
                                                    .fill(btn_fill)
                                                    .stroke(if is_full { egui::Stroke::new(1.0, p.border) } else { egui::Stroke::NONE })
                                                    .corner_radius(7)
                                                    .min_size(egui::vec2(inner_w, 28.0));
                                                    if ui.add(join_btn).clicked() {
                                                        acts.push(Action::JoinServer(
                                                            g.place_id,
                                                            srv.id.clone(),
                                                        ));
                                                    }
                                                });
                                            });
                                    }
                                });
                                ui.add_space(gap);
                            }
                            if ex.servers_cursor.is_some() && !self.game_servers_loading {
                                ui.add_space(8.0);
                                if ui.button("Load more servers").clicked() {
                                    acts.push(Action::LoadMoreServers(g.universe_id, g.place_id));
                                }
                            }
                        }
                    }
                    if self.game_servers_loading {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading live servers from Roblox...");
                        });
                    }
                });
            }
        }
            });
        });
        ui.add_space(20.0);
    }

}
