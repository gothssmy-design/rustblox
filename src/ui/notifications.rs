use eframe::egui::{self, Color32, CornerRadius, RichText, Sense};
use crate::app::RustBlox;
use crate::auth::save_settings;
use crate::roblox_process::enumerate_roblox_processes;
use crate::theme::{
    card_frame, is_dark_color, pal, theme_accent, Palette, BLUE, GREEN, ORANGE, RED,
};
use crate::types::{Action, NotificationSource, Page, SocialTab, ToastAction, ToastKind};
use crate::ui::widgets::thumb_styled;

const TOAST_WIDTH: f32 = 344.0;
const ENTER_SECS: f64 = 0.28;
const EXIT_SECS: f64 = 0.20;

fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.25_f32;
    let c3 = c1 + 1.0;
    let x = t - 1.0;
    1.0 + c3 * x.powi(3) + c1 * x.powi(2)
}

fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

fn kind_accent(kind: ToastKind, accent: Color32) -> Color32 {
    match kind {
        ToastKind::FriendRequest => GREEN,
        ToastKind::UpdateAvailable => accent,
        ToastKind::Success => GREEN,
        ToastKind::Error => ORANGE,
    }
}

fn fire_toast_action(app: &mut RustBlox, act: ToastAction, acts: &mut Vec<Action>) {
    match act {
        ToastAction::AcceptFriendRequest { uid, uname } => {
            acts.push(Action::AcceptFriendRequest(uid, uname));
        }
        ToastAction::OpenRequests => {
            app.social_tab = SocialTab::Requests;
            app.friend_search_results = None;
            app.friend_search_cursor = None;
            app.friend_searched.clear();
            acts.push(Action::OpenAddFriend);
        }
        ToastAction::OpenSettings => {
            acts.push(Action::Go(Page::Settings));
        }
        ToastAction::OpenUrl(url) => {
            acts.push(Action::OpenUrl(url));
        }
        ToastAction::KillRoblox => {
            acts.push(Action::KillRobloxProcesses);
        }
    }
}

fn format_age(now: f64, created: f64) -> String {
    let secs = (now - created).max(0.0) as u64;
    if secs < 5 {
        "just now".to_string()
    } else if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

impl RustBlox {
    pub fn render_notifications(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        if self.settings.notifications_silent {
            if !self.toasts.is_empty() {
                self.toasts.clear();
            }
            return;
        }
        if self.toasts.is_empty() {
            return;
        }

        let now = ctx.input(|i| i.time);
        let dt = ctx.input(|i| i.stable_dt as f64).clamp(0.0, 0.1);
        let screen = ctx.input(|i| i.content_rect());
        if !screen.is_positive() {
            return;
        }

        for toast in &mut self.toasts {
            if toast.paused_at.is_some() && toast.dismissed_at.is_none() {
                toast.created_at += dt;
            }
            if toast.dismissed_at.is_none()
                && toast.duration > 0.0
                && (now - toast.created_at) >= toast.duration
            {
                toast.dismissed_at = Some(now);
            }
        }

        self.toasts.retain(|t| {
            t.dismissed_at
                .map(|d| (now - d) < EXIT_SECS)
                .unwrap_or(true)
        });
        if self.toasts.is_empty() {
            return;
        }

        ctx.request_repaint();
        let accent = theme_accent(&self.settings);

        let center_x = screen.center().x - TOAST_WIDTH * 0.5;
        let mut cursor_y = if self.cookie.is_some() { 58.0 } else { 16.0 };

        let ids: Vec<u64> = self.toasts.iter().rev().map(|t| t.id).collect();
        for tid in ids {
            let Some(idx) = self.toasts.iter().position(|t| t.id == tid) else {
                continue;
            };
            let toast = self.toasts[idx].clone();

            let enter_raw = ((now - toast.created_at) / ENTER_SECS).clamp(0.0, 1.0) as f32;
            let enter_ease = ease_out_back(enter_raw);

            let (exit_raw, alpha) = if let Some(d) = toast.dismissed_at {
                let r = ((now - d) / EXIT_SECS).clamp(0.0, 1.0) as f32;
                (r, (1.0 - ease_in_cubic(r)).clamp(0.0, 1.0))
            } else {
                (0.0, enter_raw.clamp(0.0, 1.0))
            };

            let y_slide = (1.0 - enter_ease) * -72.0 - ease_in_cubic(exit_raw) * 72.0;

            let height_id = egui::Id::NULL.with(("toast_height", toast.id));
            let smooth_y = ctx.animate_value_with_time(
                egui::Id::NULL.with(("toast_y", toast.id)),
                cursor_y,
                0.16,
            );

            let kind_color = kind_accent(toast.kind, accent);
            let primary_fg = if is_dark_color([kind_color.r(), kind_color.g(), kind_color.b()]) {
                Color32::WHITE
            } else {
                Color32::from_rgb(16, 19, 26)
            };

            let mut hovered_now = false;
            let mut dismiss_clicked = false;
            let mut fired_action: Option<ToastAction> = None;

            let area_pos = egui::pos2(center_x, smooth_y + y_slide);
            let area_resp = egui::Area::new(egui::Id::NULL.with(("toast_popup", toast.id)))
                .order(egui::Order::Foreground)
                .fixed_pos(area_pos)
                .show(ctx, |ui| {
                    let p = pal(ui);
                    ui.set_opacity(alpha);
                    let border_c = Color32::from_rgba_unmultiplied(
                        kind_color.r(),
                        kind_color.g(),
                        kind_color.b(),
                        170,
                    );
                    let card_fill = if self.settings.dark {
                        Color32::from_rgba_unmultiplied(18, 22, 30, 248)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 250)
                    };

                    let frame_resp = egui::Frame::new()
                        .fill(card_fill)
                        .stroke(egui::Stroke::new(1.2, border_c))
                        .corner_radius(11)
                        .shadow(egui::Shadow {
                            offset: [0, 8],
                            blur: 24,
                            spread: 2,
                            color: Color32::from_rgba_unmultiplied(0, 0, 0, 96),
                        })
                        .inner_margin(egui::Margin {
                            left: 14,
                            right: 11,
                            top: 9,
                            bottom: 10,
                        })
                        .show(ui, |ui| {
                            ui.set_width(TOAST_WIDTH - 25.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 10.0;

                                if let Some(av_url) = &toast.avatar {
                                    let av = thumb_styled(
                                        ui,
                                        Some(av_url),
                                        egui::vec2(34.0, 34.0),
                                        17,
                                        false,
                                    );
                                    ui.painter().rect_stroke(
                                        av.rect,
                                        CornerRadius::same(17),
                                        egui::Stroke::new(1.5, kind_color),
                                        egui::StrokeKind::Inside,
                                    );
                                } else {
                                    let (badge_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(28.0, 28.0),
                                        Sense::hover(),
                                    );
                                    ui.painter().circle_filled(
                                        badge_rect.center(),
                                        14.0,
                                        Color32::from_rgba_unmultiplied(
                                            kind_color.r(),
                                            kind_color.g(),
                                            kind_color.b(),
                                            34,
                                        ),
                                    );
                                    let glyph = match toast.kind {
                                        ToastKind::FriendRequest => "+",
                                        ToastKind::UpdateAvailable => "\u{2B06}",
                                        ToastKind::Success => "\u{2713}",
                                        ToastKind::Error => "!",
                                    };
                                    ui.painter().text(
                                        badge_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        glyph,
                                        egui::FontId::new(
                                            13.5,
                                            egui::FontFamily::Name("Bold".into()),
                                        ),
                                        kind_color,
                                    );
                                }

                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    ui.set_width((TOAST_WIDTH - 76.0).max(170.0));

                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&toast.title)
                                                    .size(13.0)
                                                    .strong()
                                                    .color(p.text),
                                            )
                                            .truncate(),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let close_resp = ui
                                                    .add(
                                                        egui::Label::new(
                                                            RichText::new("\u{00D7}")
                                                                .size(14.5)
                                                                .color(p.muted),
                                                        )
                                                        .sense(Sense::click()),
                                                    )
                                                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                                                if close_resp.clicked() {
                                                    dismiss_clicked = true;
                                                }
                                            },
                                        );
                                    });

                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&toast.body)
                                                .size(11.5)
                                                .color(p.muted),
                                        )
                                        .wrap(),
                                    );

                                    if toast.primary_action.is_some()
                                        || toast.secondary_action.is_some()
                                    {
                                        ui.add_space(4.0);
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 6.0;
                                            if let Some((label, act)) = &toast.primary_action {
                                                let btn = egui::Button::new(
                                                    RichText::new(label)
                                                        .size(11.0)
                                                        .strong()
                                                        .color(primary_fg),
                                                )
                                                .fill(kind_color)
                                                .corner_radius(6)
                                                .min_size(egui::vec2(0.0, 22.0));
                                                if ui
                                                    .add(btn)
                                                    .on_hover_cursor(
                                                        egui::CursorIcon::PointingHand,
                                                    )
                                                    .clicked()
                                                {
                                                    fired_action = Some(act.clone());
                                                    dismiss_clicked = true;
                                                }
                                            }
                                            if let Some((label, act)) = &toast.secondary_action {
                                                let btn = egui::Button::new(
                                                    RichText::new(label)
                                                        .size(11.0)
                                                        .color(p.text),
                                                )
                                                .fill(p.surface_bg)
                                                .stroke(egui::Stroke::new(1.0, p.border))
                                                .corner_radius(6)
                                                .min_size(egui::vec2(0.0, 22.0));
                                                if ui
                                                    .add(btn)
                                                    .on_hover_cursor(
                                                        egui::CursorIcon::PointingHand,
                                                    )
                                                    .clicked()
                                                {
                                                    fired_action = Some(act.clone());
                                                    dismiss_clicked = true;
                                                }
                                            }
                                        });
                                    }
                                });
                            });
                        });

                    let card_rect = frame_resp.response.rect;
                    hovered_now = frame_resp.response.hovered()
                        || ctx.input(|i| {
                            i.pointer
                                .hover_pos()
                                .map(|pos| card_rect.contains(pos))
                                .unwrap_or(false)
                        });

                    let strip = egui::Rect::from_min_max(
                        egui::pos2(card_rect.left() + 2.0, card_rect.top() + 8.0),
                        egui::pos2(card_rect.left() + 5.0, card_rect.bottom() - 8.0),
                    );
                    ui.painter().rect_filled(strip, CornerRadius::same(2), kind_color);

                    if toast.duration > 0.0 && toast.dismissed_at.is_none() {
                        let rem = (1.0 - ((now - toast.created_at) / toast.duration))
                            .clamp(0.0, 1.0) as f32;
                        let bar_w = (card_rect.width() - 24.0) * rem;
                        if bar_w > 1.0 {
                            let bar = egui::Rect::from_min_size(
                                egui::pos2(card_rect.left() + 12.0, card_rect.bottom() - 3.0),
                                egui::vec2(bar_w, 2.0),
                            );
                            ui.painter().rect_filled(
                                bar,
                                CornerRadius::same(2),
                                Color32::from_rgba_unmultiplied(
                                    kind_color.r(),
                                    kind_color.g(),
                                    kind_color.b(),
                                    195,
                                ),
                            );
                        }
                    }
                });

            let actual_h = area_resp.response.rect.height().max(52.0);
            ctx.data_mut(|d| d.insert_temp(height_id, actual_h));
            cursor_y += actual_h + 8.0;

            if let Some(t) = self.toasts.get_mut(idx) {
                t.paused_at = if hovered_now { Some(now) } else { None };
                if dismiss_clicked && t.dismissed_at.is_none() {
                    t.dismissed_at = Some(now);
                }
            }

            if let Some(act) = fired_action {
                fire_toast_action(self, act, acts);
            }
        }
    }

    pub fn page_notifications(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        for item in &mut self.notification_history {
            item.read = true;
        }

        let p = pal(ui);
        let accent = theme_accent(&self.settings);
        let accent_fg = if is_dark_color([accent.r(), accent.g(), accent.b()]) {
            Color32::WHITE
        } else {
            Color32::from_rgb(16, 19, 26)
        };
        let now = ui.ctx().input(|i| i.time);
        let content_w = ui.available_width().min(780.0);
        let pad = ((ui.available_width() - content_w) * 0.5).max(0.0);

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            if pad > 0.0 {
                ui.add_space(pad);
            }
            ui.vertical(|ui| {
                ui.set_width(content_w);
                ui.set_max_width(content_w);
                // Minimalist header bar
                card_frame(ui, 16).show(ui, |ui| {
                    ui.set_width((content_w - 34.0).max(200.0));
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Notifications")
                                .size(20.0)
                                .strong()
                                .color(p.text),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            if !self.notification_history.is_empty() && ui.button("Clear").clicked()
                            {
                                self.notification_history.clear();
                            }
                            if ui.button("Check Now").clicked() {
                                self.check_updates(true);
                                self.check_app_updates(true);
                                if !self.social_loading.contains(&SocialTab::Requests) {
                                    self.load_social_tab(SocialTab::Requests, false);
                                }
                            }
                            let silent = self.settings.notifications_silent;
                            let silent_btn = egui::Button::new(
                                RichText::new(if silent { "Silent: On" } else { "Silent: Off" })
                                    .size(12.0)
                                    .strong()
                                    .color(if silent { Color32::WHITE } else { p.text }),
                            )
                            .fill(if silent { ORANGE } else { p.surface_bg })
                            .stroke(egui::Stroke::new(
                                1.0,
                                if silent { ORANGE } else { p.border },
                            ))
                            .corner_radius(7);
                            if ui
                                .add(silent_btn)
                                .on_hover_text("Mute top-center popups while still logging alerts here")
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.settings.notifications_silent = !silent;
                                save_settings(&self.settings);
                                if self.settings.notifications_silent {
                                    self.toasts.clear();
                                }
                            }
                        });
                    });
                });

                ui.add_space(10.0);

                // Minimalist 2-card status summary: Roblox API + RustBlox & Background Process
                let (procs, _) = enumerate_roblox_processes();
                let req_count = self
                    .social_lists
                    .get(&SocialTab::Requests)
                    .map(|l| l.len())
                    .unwrap_or(0);
                let card_outer_w = (content_w - 10.0) * 0.5;
                let card_inner_w = (card_outer_w - 30.0).max(150.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;

                    card_frame(ui, 14).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(card_inner_w);
                            ui.set_max_width(card_inner_w);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Roblox API")
                                        .size(13.5)
                                        .strong()
                                        .color(BLUE),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if req_count > 0 {
                                            if ui
                                                .small_button(format!("{req_count} Requests"))
                                                .clicked()
                                            {
                                                self.social_tab = SocialTab::Requests;
                                                acts.push(Action::OpenAddFriend);
                                            }
                                        } else {
                                            ui.label(
                                                RichText::new("0 pending")
                                                    .size(11.5)
                                                    .color(p.muted),
                                            );
                                        }
                                    },
                                );
                            });
                            ui.add_space(4.0);
                            let api_msg = if let Some((ver, _)) = &self.roblox_update_available {
                                format!("Roblox update v{ver} ready")
                            } else if let Some(prof) = &self.me {
                                format!("Connected as @{} · Client up to date", prof.user.name)
                            } else {
                                "Signed out · Checking updates".to_string()
                            };
                            ui.add(egui::Label::new(RichText::new(api_msg).size(12.0).color(p.muted)).wrap());
                            if let Some((ver, _)) = &self.roblox_update_available {
                                ui.add_space(6.0);
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(format!("Download v{ver}"))
                                                .size(11.5)
                                                .strong()
                                                .color(accent_fg),
                                        )
                                        .fill(accent)
                                        .corner_radius(6),
                                    )
                                    .clicked()
                                {
                                    acts.push(Action::OpenUrl(
                                        "https://www.roblox.com/download/client".to_string(),
                                    ));
                                }
                            }
                        });
                    });

                    card_frame(ui, 14).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(card_inner_w);
                            ui.set_max_width(card_inner_w);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "RustBlox v{}",
                                        env!("CARGO_PKG_VERSION")
                                    ))
                                    .size(13.5)
                                    .strong()
                                    .color(GREEN),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if !procs.is_empty() {
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new("Kill Roblox")
                                                            .size(11.5)
                                                            .strong()
                                                            .color(Color32::WHITE),
                                                    )
                                                    .fill(RED)
                                                    .corner_radius(6),
                                                )
                                                .clicked()
                                            {
                                                acts.push(Action::KillRobloxProcesses);
                                            }
                                        } else {
                                            ui.label(
                                                RichText::new(if self.settings.auto_close_roblox {
                                                    "Auto-kill: On"
                                                } else {
                                                    "Auto-kill: Off"
                                                })
                                                .size(11.5)
                                                .color(p.muted),
                                            );
                                        }
                                    },
                                );
                            });
                            ui.add_space(4.0);
                            let proc_msg = if procs.is_empty() {
                                "No background Roblox process running".to_string()
                            } else {
                                let pids: Vec<String> =
                                    procs.iter().map(|pr| pr.pid.to_string()).collect();
                                format!(
                                    "{} Roblox process running (PID {})",
                                    procs.len(),
                                    pids.join(", ")
                                )
                            };
                            ui.add(egui::Label::new(RichText::new(proc_msg).size(12.0).color(p.muted)).wrap());
                            if let Some(rel) = self.app_update_available.clone() {
                                ui.add_space(6.0);
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(format!("Update to v{}", rel.version))
                                                .size(11.5)
                                                .strong()
                                                .color(accent_fg),
                                        )
                                        .fill(accent)
                                        .corner_radius(6),
                                    )
                                    .clicked()
                                {
                                    acts.push(Action::Go(Page::Settings));
                                }
                            }
                        });
                    });
                });

                ui.add_space(12.0);

                // Minimalist message feed
                if self.notification_history.is_empty() {
                    card_frame(ui, 24).show(ui, |ui| {
                        ui.set_width((content_w - 50.0).max(200.0));
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("No notifications yet.")
                                    .size(14.5)
                                    .strong()
                                    .color(p.text),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(
                                    "Friend requests, updates, and background Roblox process alerts appear here.",
                                )
                                .size(12.0)
                                .color(p.muted),
                            );
                        });
                    });
                } else {
                    let items = self.notification_history.clone();
                    for item in items.iter().rev() {
                        render_history_row(self, ui, item, now, p, accent, accent_fg, acts);
                        ui.add_space(6.0);
                    }
                }
                ui.add_space(20.0);
            });
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn render_history_row(
    app: &mut RustBlox,
    ui: &mut egui::Ui,
    item: &crate::types::NotificationItem,
    now: f64,
    p: Palette,
    accent: Color32,
    accent_fg: Color32,
    acts: &mut Vec<Action>,
) {
    let kcol = kind_accent(item.kind, accent);
    let src_col = match item.source {
        NotificationSource::RobloxApi => BLUE,
        NotificationSource::RustBlox => GREEN,
        NotificationSource::Process => ORANGE,
    };

    ui.push_id(item.id, |ui| {
        card_frame(ui, 12).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                if let Some(av) = &item.avatar {
                    thumb_styled(ui, Some(av), egui::vec2(32.0, 32.0), 16, false);
                } else {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(8.0, 32.0), Sense::hover());
                    ui.painter().rect_filled(
                        egui::Rect::from_center_size(r.center(), egui::vec2(4.0, 22.0)),
                        CornerRadius::same(2),
                        kcol,
                    );
                }

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(item.source.label())
                                .size(10.5)
                                .strong()
                                .color(src_col),
                        );
                        ui.label(RichText::new("·").size(10.5).color(p.muted));
                        ui.label(
                            RichText::new(&item.title)
                                .size(13.0)
                                .strong()
                                .color(p.text),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format_age(now, item.created_secs))
                                    .size(11.0)
                                    .color(p.muted),
                            );
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&item.body).size(12.0).color(p.muted));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some((label, act)) = &item.secondary_action {
                                if ui.small_button(label).clicked() {
                                    fire_toast_action(app, act.clone(), acts);
                                }
                            }
                            if let Some((label, act)) = &item.primary_action {
                                let btn = egui::Button::new(
                                    RichText::new(label)
                                        .size(11.0)
                                        .strong()
                                        .color(accent_fg),
                                )
                                .fill(accent)
                                .corner_radius(6);
                                if ui.add(btn).clicked() {
                                    fire_toast_action(app, act.clone(), acts);
                                }
                            }
                        });
                    });
                });
            });
        });
    });
}
