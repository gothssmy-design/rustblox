use eframe::egui::{self, Color32, RichText};
use crate::app::RustBlox;
use crate::theme::{pal, GREEN, RED};
use crate::types::Action;
use crate::ui::widgets::*;

impl RustBlox {
    pub fn page_charts(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let p = pal(ui);
        let syncing = Self::is_sync_active(&self.sorts_sync_started);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Charts").size(26.0).strong());
            sync_refresh_control(
                ui,
                syncing,
                "Charts auto-sync in background (click to sync now)",
                acts,
            );
        });
        ui.add_space(10.0);
        match &self.sorts {
            None => {
                ui.spinner();
            }
            Some(Err(e)) => {
                ui.colored_label(RED, e);
            }
            Some(Ok(sorts)) => {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                    for (i, s) in sorts.iter().enumerate() {
                        let active = self.chart == i;
                        let btn = egui::Button::new(
                            RichText::new(&s.title)
                                .size(14.0)
                                .strong()
                                .color(if active { Color32::WHITE } else { p.text }),
                        )
                        .fill(if active { GREEN } else { p.surface_bg })
                        .corner_radius(18);
                        if ui.add(btn).clicked() {
                            self.chart = i;
                        }
                    }
                });
                ui.add_space(14.0);
                if let Some(s) = sorts.get(self.chart) {
                    let preview = self.current_card_preview();
                    wide_grid(ui, &s.games, &self.wide, preview, acts);
                }
            }
        }
        ui.add_space(20.0);
    }

}
