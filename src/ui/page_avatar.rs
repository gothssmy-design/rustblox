use std::collections::HashSet;
use eframe::egui::{self, Color32, CornerRadius, Sense, RichText};
use crate::app::RustBlox;
use crate::avatar_3d::render_avatar_viewport;
use crate::theme::{pal, GREEN, RED};
use crate::types::{Action, AvatarAsset, AvatarTab, BodyColorTarget, OutfitItem};
use crate::ui::widgets::*;

fn avatar_assets_grid(
    ui: &mut egui::Ui,
    items: &[AvatarAsset],
    worn_ids: &HashSet<u64>,
    acts: &mut Vec<Action>,
) {
    let pal_colors = pal(ui);
    let avail = (ui.available_width() - 16.0).max(200.0);
    let gap = 12.0;
    let target_w = 160.0;
    let cols = (((avail + gap) / target_w).floor() as usize).max(2);
    let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32)
        .floor()
        .clamp(140.0, 195.0);
    let thumb_s = (card_w - 20.0).max(118.0);
    let item_fill = if ui.visuals().dark_mode {
        Color32::from_rgba_unmultiplied(28, 33, 44, 220)
    } else {
        pal_colors.surface_bg
    };
    for row in items.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for item in row {
                let is_worn = worn_ids.contains(&item.id);
                let card_fill = if is_worn {
                    if ui.visuals().dark_mode {
                        Color32::from_rgba_unmultiplied(22, 44, 34, 240)
                    } else {
                        Color32::from_rgb(232, 249, 238)
                    }
                } else {
                    item_fill
                };
                let card_stroke = egui::Stroke::new(
                    if is_worn { 2.0 } else { 1.0 },
                    if is_worn { GREEN } else { pal_colors.border },
                );

                let mut card_clicked = false;
                let frame_res = egui::Frame::new()
                    .fill(card_fill)
                    .stroke(card_stroke)
                    .corner_radius(12)
                    .inner_margin(8)
                    .show(ui, |ui| {
                        ui.set_width(thumb_s);
                        ui.vertical_centered(|ui| {
                            let u = (!item.thumb.is_empty()).then_some(&item.thumb);
                            let (thumb_rect, thumb_resp) = ui.allocate_exact_size(
                                egui::vec2(thumb_s, thumb_s),
                                egui::Sense::click(),
                            );
                            if thumb_resp.clicked() {
                                card_clicked = true;
                            }
                            if thumb_resp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            let p = ui.painter_at(thumb_rect);
                            p.rect_filled(
                                thumb_rect,
                                10.0,
                                if ui.visuals().dark_mode {
                                    Color32::from_rgb(18, 22, 30)
                                } else {
                                    Color32::from_rgb(240, 242, 246)
                                },
                            );
                            if let Some(url) = u {
                                let img = egui::Image::new(url)
                                    .fit_to_exact_size(egui::vec2(thumb_s, thumb_s))
                                    .corner_radius(10);
                                img.paint_at(ui, thumb_rect);
                            }

                            if is_worn {
                                let badge_size = egui::vec2(58.0, 20.0);
                                let badge_rect = egui::Rect::from_min_size(
                                    egui::pos2(thumb_rect.right() - 62.0, thumb_rect.top() + 6.0),
                                    badge_size,
                                );
                                p.rect_filled(badge_rect, 10.0, GREEN);
                                p.text(
                                    badge_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "âœ“ Worn",
                                    egui::FontId::proportional(11.0),
                                    Color32::WHITE,
                                );
                            }

                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 3.0;
                                let name_w = if item.creator_verified { (thumb_s - 18.0).max(60.0) } else { thumb_s };
                                ui.add_sized(
                                    [name_w, 18.0],
                                    egui::Label::new(RichText::new(&item.name).size(13.0).strong())
                                        .truncate(),
                                )
                                .on_hover_text(&item.name);
                                if item.creator_verified {
                                    let tip = if item.creator_name.is_empty() {
                                        "Verified Creator".to_string()
                                    } else {
                                        format!("By {} (Verified)", item.creator_name)
                                    };
                                    verified_badge_tip(ui, 13.0, &tip);
                                }
                            });
                            ui.add_sized(
                                [thumb_s, 16.0],
                                egui::Label::new(
                                    RichText::new(&item.asset_type_name)
                                        .size(11.0)
                                        .color(if is_worn { GREEN } else { pal_colors.muted }),
                                )
                                .truncate(),
                            );
                        });
                    });

                let resp = frame_res.response.interact(egui::Sense::click());
                if resp.clicked() || card_clicked {
                    acts.push(Action::ToggleWearAsset(item.clone()));
                }
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let tip = if is_worn {
                    format!("Click to unequip {}", item.name)
                } else {
                    format!("Click to equip {}", item.name)
                };
                resp.on_hover_text(tip);
            }
        });
        ui.add_space(gap);
    }
}


impl RustBlox {
    pub fn page_avatar(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let pal_colors = pal(ui);
        ui.add_space(10.0);
        let Some(avatar_url) = self.me.as_ref().map(|p| p.avatar.clone()) else {
            avatar_glass_frame(ui, 24).show(ui, |ui| {
                if let Some(e) = &self.me_err {
                    ui.colored_label(RED, format!("Could not load profile: {e}"));
                    ui.add_space(6.0);
                }
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading your avatar...");
                });
            });
            return;
        };

        let syncing = self.avatar_pending > 0 || self.avatar_loading;
        let current_tab = self.avatar_tab;
        let current_rig = self
            .avatar_state
            .as_ref()
            .map(|s| s.avatar_type.clone())
            .unwrap_or_else(|| "R15".to_string());

        if self.avatar_3d.model.is_none() && !self.avatar_3d.loading && self.avatar_3d.err.is_none() {
            self.load_avatar_3d();
        }

        let pedestal_fill = if ui.visuals().dark_mode {
            Color32::from_rgba_unmultiplied(15, 18, 25, 220)
        } else {
            pal_colors.surface_bg
        };

        let win_h = ui.ctx().input(|i| i.content_rect().height());
        let left_panel_w = 440.0f32;
        let left_h = (win_h - 75.0).clamp(580.0, 860.0);

        ui.horizontal_top(|ui| {
            // 1. Left Avatar Panel: ONLY the character viewport, large and centered, just like in Roblox!
            ui.vertical(|ui| {
                ui.set_width(left_panel_w);
                render_avatar_viewport(
                    ui,
                    &mut self.avatar_3d,
                    Some(&avatar_url),
                    egui::vec2(left_panel_w, left_h),
                    ui.visuals().dark_mode,
                );
            });

            ui.add_space(16.0);

            // 2. Right customization workspace (strictly bounded to viewport width, never dims or resets scroll)
            let right_w = (ui.available_width() - 4.0).max(300.0);
            let inner_w = (right_w - 36.0).max(260.0);

            ui.allocate_ui_with_layout(
                egui::vec2(right_w, left_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(right_w);
                    ui.set_max_width(right_w);
                    if let Some(e) = &self.avatar_err {
                        avatar_glass_frame(ui, 12).show(ui, |ui| {
                            ui.set_width((right_w - 28.0).max(260.0));
                            ui.horizontal(|ui| {
                                ui.colored_label(RED, format!("Avatar error: {e}"));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("Dismiss").clicked() {
                                            acts.push(Action::DismissAvatarErr);
                                        }
                                    },
                                );
                            });
                        });
                        ui.add_space(8.0);
                    }

                    avatar_glass_frame(ui, 16).show(ui, |ui| {
                        ui.set_width(inner_w);
                        ui.set_max_width(inner_w);

                        // Header row with title, Rig Type (R6 / R15), and Reload
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Customize Avatar").size(20.0).strong());
                            ui.add_space(12.0);
                            // Sleek Rig Type switch in header
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                ui.label(RichText::new("Rig:").size(12.0).color(pal_colors.muted));
                                for rig in ["R6", "R15"] {
                                    let selected = current_rig == rig;
                                    let btn = egui::Button::new(
                                        RichText::new(rig).size(12.0).strong().color(if selected {
                                            Color32::WHITE
                                        } else {
                                            pal_colors.muted
                                        }),
                                    )
                                    .fill(if selected { GREEN } else { pedestal_fill })
                                    .stroke(egui::Stroke::new(1.0, if selected { GREEN } else { pal_colors.border }))
                                    .corner_radius(6)
                                    .min_size(egui::vec2(44.0, 24.0));
                                    if ui.add(btn).clicked() && !selected {
                                        acts.push(Action::SetAvatarType(rig.to_string()));
                                    }
                                }
                            });

                            if syncing {
                                ui.add_space(8.0);
                                ui.spinner();
                                ui.label(
                                    RichText::new("Syncing...")
                                        .size(11.5)
                                        .color(pal_colors.muted),
                                );
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let reload_btn = egui::Button::new("â†» Reload")
                                    .fill(pedestal_fill)
                                    .stroke(egui::Stroke::new(1.0, pal_colors.border))
                                    .corner_radius(8);
                                if ui.add(reload_btn).clicked() {
                                    acts.push(Action::RefreshAvatar);
                                    acts.push(Action::RefreshAvatarTab(current_tab));
                                }
                            });
                        });
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        let worn_count = self
                            .avatar_state
                            .as_ref()
                            .map(|s| s.wearing.len())
                            .unwrap_or(0);
                        let cat_w = 152.0;
                        let content_w = (inner_w - cat_w - 18.0).max(200.0);
                        let grid_scroll_h = if matches!(current_tab, AvatarTab::Body) {
                            (win_h - 175.0).max(400.0)
                        } else {
                            (win_h - 220.0).max(380.0)
                        };

                        ui.horizontal_top(|ui| {
                            // Vertical category list on the left inside Customize Avatar
                            egui::Frame::new()
                                .fill(pedestal_fill)
                                .stroke(egui::Stroke::new(1.0, pal_colors.border))
                                .corner_radius(12)
                                .inner_margin(6)
                                .show(ui, |ui| {
                                    ui.set_width(cat_w - 14.0);
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 4.0;
                                        for tab in AvatarTab::ALL {
                                            let selected = current_tab == tab;
                                            let text =
                                                if tab == AvatarTab::Wearing && worn_count > 0 {
                                                    format!("Wearing ({worn_count})")
                                                } else {
                                                    tab.label().to_string()
                                                };
                                            let btn = egui::Button::new(
                                                RichText::new(text).size(12.5).strong().color(
                                                    if selected {
                                                        Color32::WHITE
                                                    } else {
                                                        pal_colors.text
                                                    },
                                                ),
                                            )
                                            .fill(if selected {
                                                GREEN
                                            } else {
                                                Color32::TRANSPARENT
                                            })
                                            .corner_radius(8)
                                            .min_size(egui::vec2(cat_w - 14.0, 31.0));
                                            if ui.add(btn).clicked() && !selected {
                                                acts.push(Action::SelectAvatarTab(tab));
                                            }
                                        }
                                    });
                                });

                            ui.add_space(12.0);

                            // Right catalog area: search bar + scrollable grid
                            ui.allocate_ui_with_layout(
                                egui::vec2(content_w, (win_h - 165.0).max(420.0)),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_width(content_w);
                                    ui.set_max_width(content_w);

                                    if !matches!(current_tab, AvatarTab::Body) {
                                        let has_accessories = self
                                            .avatar_state
                                            .as_ref()
                                            .map(|s| {
                                                s.wearing
                                                    .iter()
                                                    .any(|a| matches!(a.asset_type_id, 8 | 41..=47))
                                            })
                                            .unwrap_or(false);
                                        ui.horizontal(|ui| {
                                            let extra_w = if current_tab == AvatarTab::Wearing
                                                && has_accessories
                                            {
                                                164.0
                                            } else {
                                                4.0
                                            };
                                            let bar_w = (content_w - extra_w).max(140.0);
                                            egui::Frame::new()
                                                .fill(pedestal_fill)
                                                .stroke(egui::Stroke::new(1.0, pal_colors.border))
                                                .corner_radius(14)
                                                .inner_margin(egui::Margin::symmetric(12, 6))
                                                .show(ui, |ui| {
                                                    ui.set_width((bar_w - 26.0).max(100.0));
                                                    ui.horizontal(|ui| {
                                                        ui.add(
                                                            egui::Image::new(egui::include_image!(
                                                                "../../assets/icon_search.png"
                                                            ))
                                                            .fit_to_exact_size(egui::vec2(
                                                                13.0, 13.0,
                                                            ))
                                                            .tint(pal_colors.muted),
                                                        );
                                                        ui.add(
                                                            egui::TextEdit::singleline(
                                                                &mut self.avatar_search,
                                                            )
                                                            .frame(egui::Frame::NONE)
                                                            .hint_text("Filter items by name...")
                                                            .desired_width(
                                                                (bar_w - 72.0).max(60.0),
                                                            ),
                                                        );
                                                        if !self.avatar_search.is_empty()
                                                            && ui
                                                                .add(
                                                                    egui::Label::new(
                                                                        RichText::new("âœ•")
                                                                            .size(12.0)
                                                                            .color(
                                                                                pal_colors.muted,
                                                                            ),
                                                                    )
                                                                    .sense(Sense::click()),
                                                                )
                                                                .on_hover_cursor(
                                                                    egui::CursorIcon::PointingHand,
                                                                )
                                                                .clicked()
                                                        {
                                                            self.avatar_search.clear();
                                                        }
                                                    });
                                                });
                                            if current_tab == AvatarTab::Wearing && has_accessories
                                            {
                                                let strip_btn = egui::Button::new(
                                                    RichText::new("Unequip accessories").size(12.0),
                                                )
                                                .fill(pedestal_fill)
                                                .stroke(egui::Stroke::new(1.0, pal_colors.border))
                                                .corner_radius(12);
                                                if ui.add(strip_btn).clicked() {
                                                    acts.push(Action::UnequipAccessories);
                                                }
                                            }
                                        });
                                        ui.add_space(10.0);
                                    }

                                    let filter = self.avatar_search.trim().to_lowercase();
                                    let worn_ids: HashSet<u64> = self
                                        .avatar_state
                                        .as_ref()
                                        .map(|s| s.wearing.iter().map(|a| a.id).collect())
                                        .unwrap_or_default();

                                    // Dedicated scrollable grid so categories & search stay pinned
                                    egui::ScrollArea::vertical()
                                        .id_salt(("avatar_grid_scroll", current_tab as u8))
                                        .min_scrolled_height(grid_scroll_h)
                                        .max_height(grid_scroll_h)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            ui.set_max_width(content_w);
                                            match current_tab {
                                    AvatarTab::Wearing => {
                                        let items: Vec<AvatarAsset> = self
                                            .avatar_state
                                            .as_ref()
                                            .map(|s| {
                                                s.wearing
                                                    .iter()
                                                    .filter(|a| {
                                                        filter.is_empty()
                                                            || a.name
                                                                .to_lowercase()
                                                                .contains(&filter)
                                                            || a.asset_type_name
                                                                .to_lowercase()
                                                                .contains(&filter)
                                                    })
                                                    .cloned()
                                                    .collect()
                                            })
                                            .unwrap_or_default();
                                        if self.avatar_loading && self.avatar_state.is_none() {
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.label("Loading worn items...");
                                            });
                                        } else if items.is_empty() {
                                            ui.label(
                                                RichText::new("No worn items match this filter.")
                                                    .color(pal_colors.muted),
                                            );
                                        } else {
                                            avatar_assets_grid(ui, &items, &worn_ids, acts);
                                        }
                                    }
                                    AvatarTab::Outfits => {
                                        if let Some(err) = &self.avatar_outfits_err {
                                            ui.colored_label(
                                                RED,
                                                format!("Could not load outfits: {err}"),
                                            );
                                        } else if let Some(outfits) = &self.avatar_outfits {
                                            let filtered: Vec<&OutfitItem> = outfits
                                                .iter()
                                                .filter(|o| {
                                                    filter.is_empty()
                                                        || o.name.to_lowercase().contains(&filter)
                                                })
                                                .collect();
                                            if filtered.is_empty() && !self.avatar_outfits_loading {
                                                ui.label(
                                                    RichText::new("No saved outfits found.")
                                                        .color(pal_colors.muted),
                                                );
                                            } else if !filtered.is_empty() {
                                                let avail = (ui.available_width() - 16.0).max(200.0);
                                                let gap = 12.0;
                                                let target_w = 160.0;
                                                let cols = (((avail + gap) / target_w).floor() as usize).max(2);
                                                let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32)
                                                    .floor()
                                                    .clamp(140.0, 195.0);
                                                let thumb_s = (card_w - 20.0).max(118.0);
                                                for row in filtered.chunks(cols) {
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = gap;
                                                        for outfit in row {
                                                            let mut clicked = false;
                                                            let frame_res = egui::Frame::new()
                                                                .fill(pedestal_fill)
                                                                .stroke(egui::Stroke::new(
                                                                    1.0,
                                                                    pal_colors.border,
                                                                ))
                                                                .corner_radius(12)
                                                                .inner_margin(8)
                                                                .show(ui, |ui| {
                                                                    ui.set_width(thumb_s);
                                                                    ui.vertical_centered(|ui| {
                                                                        let u = (!outfit.thumb.is_empty())
                                                                            .then_some(&outfit.thumb);
                                                                        let pic = thumb_styled(
                                                                            ui,
                                                                            u,
                                                                            egui::vec2(thumb_s, thumb_s),
                                                                            10,
                                                                            true,
                                                                        )
                                                                        .on_hover_text(&outfit.name);
                                                                        if pic.clicked() {
                                                                            clicked = true;
                                                                        }
                                                                        ui.add_space(8.0);
                                                                        ui.add_sized(
                                                                            [thumb_s, 20.0],
                                                                            egui::Label::new(
                                                                                RichText::new(&outfit.name)
                                                                                    .size(13.0)
                                                                                    .strong(),
                                                                            )
                                                                            .truncate(),
                                                                        )
                                                                        .on_hover_text(&outfit.name);
                                                                        ui.add_space(4.0);
                                                                        ui.label(
                                                                            RichText::new("Outfit")
                                                                                .size(11.0)
                                                                                .color(pal_colors.muted),
                                                                        );
                                                                    });
                                                                });
                                                            let resp = frame_res.response.interact(egui::Sense::click());
                                                            if resp.clicked() || clicked {
                                                                acts.push(Action::WearOutfit((*outfit).clone()));
                                                            }
                                                            if resp.hovered() {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                            }
                                                            resp.on_hover_text(format!("Click to wear outfit \"{}\"", outfit.name));
                                                        }
                                                    });
                                                    ui.add_space(gap);
                                                }
                                                if self.avatar_outfits_more
                                                    && !self.avatar_outfits_loading
                                                {
                                                    ui.add_space(12.0);
                                                    if ui.button("Load more outfits").clicked() {
                                                        acts.push(Action::LoadMoreAvatarTab(
                                                            AvatarTab::Outfits,
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                        if self.avatar_outfits_loading {
                                            ui.add_space(8.0);
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.label("Loading saved outfits...");
                                            });
                                        }
                                    }
                                    AvatarTab::Body => {
                                        if let Some(st) = &mut self.avatar_state {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("Rig Type:").size(14.0).strong());
                                                ui.add_space(8.0);
                                                for rig in ["R6", "R15"] {
                                                    let selected = current_rig == rig;
                                                    let btn = egui::Button::new(
                                                        RichText::new(rig).size(13.0).strong().color(if selected {
                                                            Color32::WHITE
                                                        } else {
                                                            pal_colors.muted
                                                        }),
                                                    )
                                                    .fill(if selected { GREEN } else { pedestal_fill })
                                                    .stroke(egui::Stroke::new(1.0, if selected { GREEN } else { pal_colors.border }))
                                                    .corner_radius(6)
                                                    .min_size(egui::vec2(52.0, 26.0));
                                                    if ui.add(btn).clicked() && !selected {
                                                        acts.push(Action::SetAvatarType(rig.to_string()));
                                                    }
                                                }
                                            });
                                            ui.add_space(10.0);
                                            ui.label(
                                                RichText::new("Body Scales (R15)")
                                                    .size(16.0)
                                                    .strong(),
                                            );
                                            ui.add_space(4.0);
                                            if st.avatar_type != "R15" {
                                                ui.label(
                                                    RichText::new("Tip: switch Rig Type to R15 above for scales to apply visually.")
                                                        .size(12.0)
                                                        .color(pal_colors.muted),
                                                );
                                                ui.add_space(4.0);
                                            }
                                            let sliders: [(
                                                &str,
                                                &mut f32,
                                                std::ops::RangeInclusive<f32>,
                                            ); 6] = [
                                                ("Height", &mut st.scales.height, 0.90..=1.05),
                                                ("Width", &mut st.scales.width, 0.70..=1.00),
                                                ("Head", &mut st.scales.head, 0.95..=1.00),
                                                ("Depth", &mut st.scales.depth, 0.50..=1.00),
                                                (
                                                    "Proportion",
                                                    &mut st.scales.proportion,
                                                    0.00..=1.00,
                                                ),
                                                (
                                                    "Body Type",
                                                    &mut st.scales.body_type,
                                                    0.00..=1.00,
                                                ),
                                            ];
                                            for (name, val, range) in sliders {
                                                ui.horizontal(|ui| {
                                                    ui.add_sized(
                                                        [92.0, 20.0],
                                                        egui::Label::new(name),
                                                    );
                                                    ui.add(
                                                        egui::Slider::new(val, range).step_by(0.01),
                                                    );
                                                });
                                            }
                                            ui.add_space(8.0);
                                            ui.horizontal(|ui| {
                                                let apply_btn = egui::Button::new(
                                                    RichText::new("Apply Scales")
                                                        .strong()
                                                        .color(Color32::WHITE),
                                                )
                                                .fill(GREEN)
                                                .corner_radius(7);
                                                if ui.add(apply_btn).clicked() {
                                                    acts.push(Action::ApplyAvatarScales);
                                                }
                                                let reset_btn = egui::Button::new("Reset Default")
                                                    .fill(pedestal_fill)
                                                    .stroke(egui::Stroke::new(
                                                        1.0,
                                                        pal_colors.border,
                                                    ))
                                                    .corner_radius(7);
                                                if ui.add(reset_btn).clicked() {
                                                    acts.push(Action::ResetAvatarScales);
                                                }
                                            });

                                            ui.add_space(16.0);
                                            ui.separator();
                                            ui.add_space(12.0);

                                            ui.label(
                                                RichText::new("Skin Tone").size(16.0).strong(),
                                            );
                                            ui.add_space(6.0);
                                            let color_of = |id: u64| -> Color32 {
                                                self.avatar_palette
                                                    .iter()
                                                    .find(|c| c.id == id)
                                                    .map(|c| c.color)
                                                    .unwrap_or(Color32::from_rgb(163, 162, 165))
                                            };
                                            ui.horizontal_wrapped(|ui| {
                                                ui.spacing_mut().item_spacing =
                                                    egui::vec2(6.0, 6.0);
                                                for target in BodyColorTarget::ALL {
                                                    let sel = self.avatar_color_target == target;
                                                    let part_color = match target {
                                                        BodyColorTarget::All => None,
                                                        BodyColorTarget::Head => {
                                                            Some(color_of(st.colors.head))
                                                        }
                                                        BodyColorTarget::Torso => {
                                                            Some(color_of(st.colors.torso))
                                                        }
                                                        BodyColorTarget::LeftArm => {
                                                            Some(color_of(st.colors.left_arm))
                                                        }
                                                        BodyColorTarget::RightArm => {
                                                            Some(color_of(st.colors.right_arm))
                                                        }
                                                        BodyColorTarget::LeftLeg => {
                                                            Some(color_of(st.colors.left_leg))
                                                        }
                                                        BodyColorTarget::RightLeg => {
                                                            Some(color_of(st.colors.right_leg))
                                                        }
                                                    };
                                                    let mut btn = egui::Button::new(
                                                        RichText::new(target.label())
                                                            .size(12.0)
                                                            .color(if sel {
                                                                Color32::WHITE
                                                            } else {
                                                                pal_colors.text
                                                            }),
                                                    )
                                                    .fill(if sel {
                                                        GREEN
                                                    } else {
                                                        pedestal_fill
                                                    })
                                                    .corner_radius(12);
                                                    if let Some(c) = part_color {
                                                        btn = btn.stroke(egui::Stroke::new(
                                                            1.5,
                                                            if sel { GREEN } else { c },
                                                        ));
                                                    }
                                                    if ui.add(btn).clicked() {
                                                        self.avatar_color_target = target;
                                                    }
                                                }
                                            });
                                            ui.add_space(10.0);

                                            if self.avatar_palette.is_empty() {
                                                ui.label(
                                                    RichText::new("Loading Roblox skin palette...")
                                                        .color(pal_colors.muted),
                                                );
                                            } else {
                                                let active_id = match self.avatar_color_target {
                                                    BodyColorTarget::All
                                                    | BodyColorTarget::Head => st.colors.head,
                                                    BodyColorTarget::Torso => st.colors.torso,
                                                    BodyColorTarget::LeftArm => st.colors.left_arm,
                                                    BodyColorTarget::RightArm => {
                                                        st.colors.right_arm
                                                    }
                                                    BodyColorTarget::LeftLeg => st.colors.left_leg,
                                                    BodyColorTarget::RightLeg => {
                                                        st.colors.right_leg
                                                    }
                                                };
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.spacing_mut().item_spacing =
                                                        egui::vec2(6.0, 6.0);
                                                    for swatch in &self.avatar_palette {
                                                        let is_cur = swatch.id == active_id;
                                                        let (rect, resp) = ui.allocate_exact_size(
                                                            egui::vec2(30.0, 30.0),
                                                            Sense::click(),
                                                        );
                                                        let resp = resp
                                                            .on_hover_text(&swatch.name)
                                                            .on_hover_cursor(
                                                                egui::CursorIcon::PointingHand,
                                                            );
                                                        ui.painter().rect_filled(
                                                            rect,
                                                            CornerRadius::same(6),
                                                            swatch.color,
                                                        );
                                                        if is_cur {
                                                            ui.painter().rect_stroke(
                                                                rect.expand(1.5),
                                                                CornerRadius::same(7),
                                                                egui::Stroke::new(2.5, GREEN),
                                                                egui::StrokeKind::Outside,
                                                            );
                                                        }
                                                        if resp.clicked() {
                                                            acts.push(Action::SetBodyColorId(
                                                                swatch.id,
                                                            ));
                                                        }
                                                    }
                                                });
                                            }
                                        } else {
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.label("Loading body settings...");
                                            });
                                        }
                                    }
                                    tab => {
                                        if let Some(err) = &self.avatar_items_err {
                                            ui.colored_label(
                                                RED,
                                                format!("Could not load inventory: {err}"),
                                            );
                                        } else if let Some(list) = self.avatar_items.get(&tab) {
                                            let filtered: Vec<AvatarAsset> = list
                                                .iter()
                                                .filter(|a| {
                                                    filter.is_empty()
                                                        || a.name.to_lowercase().contains(&filter)
                                                        || a.asset_type_name
                                                            .to_lowercase()
                                                            .contains(&filter)
                                                })
                                                .cloned()
                                                .collect();
                                            if filtered.is_empty()
                                                && self.avatar_items_loading != Some(tab)
                                            {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "No {} items found in your inventory.",
                                                        tab.label()
                                                    ))
                                                    .color(pal_colors.muted),
                                                );
                                            } else if !filtered.is_empty() {
                                                avatar_assets_grid(
                                                    ui,
                                                    &filtered,
                                                    &worn_ids,
                                                    acts,
                                                );
                                                let has_more = self
                                                    .avatar_cursors
                                                    .get(&tab)
                                                    .and_then(|c| c.as_ref())
                                                    .is_some();
                                                if has_more && self.avatar_items_loading != Some(tab)
                                                {
                                                    ui.add_space(12.0);
                                                    if ui.button("Load more items").clicked() {
                                                        acts.push(Action::LoadMoreAvatarTab(tab));
                                                    }
                                                }
                                            }
                                        }
                                        if self.avatar_items_loading == Some(tab) {
                                            ui.add_space(8.0);
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.label(format!(
                                                    "Loading {} from your inventory...",
                                                    tab.label()
                                                ));
                                            });
                                        }
                                    }
                                }
                                        });
                                },
                            );
                        });
                    });
                },
            );
        });
        ui.add_space(20.0);
    }

}
