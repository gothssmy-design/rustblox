use eframe::egui::{self, Color32, RichText};
use crate::app::RustBlox;
use crate::theme::{brand_title, card_frame, pal, GREEN, RED};
use crate::types::Action;

impl RustBlox {
    pub fn page_login(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let logged_in = self.cookie.is_some();
        ui.add_space(10.0);
        let mut cancel_clicked = false;
        let mut regen_clicked = false;

        if logged_in {
            if ui.button("â† Back").clicked() {
                cancel_clicked = true;
            }
            ui.add_space(16.0);
        } else {
            ui.add_space(28.0);
        }

        ui.vertical_centered(|ui| {
            let p = pal(ui);
            card_frame(ui, 28).show(ui, |ui| {
                ui.set_max_width(480.0);
                ui.vertical_centered(|ui| {
                    ui.add(
                        egui::Image::new(egui::include_image!("../../assets/logo.png"))
                            .fit_to_exact_size(egui::vec2(48.0, 48.0)),
                    );
                    ui.add_space(6.0);
                    brand_title(ui, 26.0, false);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Connect your Roblox account with Quick Login \u{2014} no password needed, encrypted locally with Windows DPAPI.")
                            .color(p.muted)
                            .size(13.0),
                    );
                    ui.add_space(18.0);

                    if let Some(ql) = &self.quick_login {
                        if let Some(err) = &ql.error {
                            ui.colored_label(RED, err);
                            ui.add_space(12.0);
                        } else if ql.code.is_empty() {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(&ql.status_text);
                            });
                        } else {
                            ui.label("1. Open the Roblox app or roblox.com \u{2192} Settings \u{2192} Quick Login");
                            ui.label("2. Enter this 6-character code (or scan the QR code):");
                            ui.add_space(12.0);

                            let spaced: String = ql
                                .code
                                .chars()
                                .map(|c| c.to_string())
                                .collect::<Vec<_>>()
                                .join("  ");
                            egui::Frame::new()
                                .fill(p.surface_bg)
                                .stroke(egui::Stroke::new(1.0, p.border))
                                .corner_radius(10)
                                .inner_margin(egui::Margin::symmetric(24, 12))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(spaced)
                                            .size(34.0)
                                            .strong()
                                            .monospace()
                                            .color(p.text),
                                    );
                                });
                            ui.add_space(8.0);
                            if ui.button("Copy code").clicked() {
                                ui.ctx().copy_text(ql.code.clone());
                                self.status = "Quick Login code copied".to_string();
                            }

                            if let Some(bytes) = &ql.qr_bytes {
                                ui.add_space(14.0);
                                egui::Frame::new()
                                    .fill(Color32::WHITE)
                                    .corner_radius(10)
                                    .inner_margin(8)
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::Image::from_bytes(ql.qr_uri.clone(), bytes.clone())
                                                .fit_to_exact_size(egui::vec2(180.0, 180.0)),
                                        );
                                    });
                            }

                            ui.add_space(14.0);
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(&ql.status_text);
                            });
                        }
                    } else {
                        ui.label(
                            RichText::new("Sign in with Roblox Quick Login (QR code or 6-character code) to access rustblox.")
                                .color(p.muted),
                        );
                    }

                    ui.add_space(18.0);
                    ui.horizontal(|ui| {
                        let btn_text = if self.quick_login.is_some() {
                            "New QR & code"
                        } else {
                            "Sign in with QR / Quick Login"
                        };
                        let start_btn = egui::Button::new(
                            RichText::new(btn_text).strong().color(Color32::WHITE),
                        )
                        .fill(GREEN);
                        if ui.add(start_btn).clicked() {
                            regen_clicked = true;
                        }
                        if self.quick_login.is_some() && ui.button("Cancel").clicked() {
                            cancel_clicked = true;
                        }
                    });
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
