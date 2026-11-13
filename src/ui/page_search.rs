use eframe::egui::{self, RichText};
use crate::app::RustBlox;
use crate::theme::RED;
use crate::types::Action;
use crate::ui::widgets::*;

impl RustBlox {
    pub fn page_search(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(10.0);
        ui.label(
            RichText::new(format!("Results for \"{}\"", self.searched))
                .size(24.0)
                .strong(),
        );
        ui.add_space(10.0);
        if self.searching {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Searching...");
            });
        }
        match &self.results {
            Some(Err(e)) => {
                ui.colored_label(RED, e);
            }
            Some(Ok(v)) if v.is_empty() => {
                ui.label("No games found.");
            }
            Some(Ok(v)) => {
                let preview = self.current_card_preview();
                wide_grid(ui, v, &self.wide, preview, acts);
            }
            None => {}
        }
        ui.add_space(20.0);
    }

}
