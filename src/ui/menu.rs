use eframe::egui::{self, CornerRadius, Sense};
use crate::theme::{pal, GREEN};
use crate::types::NavIcon;


pub fn nav_icon_source(icon: NavIcon) -> egui::ImageSource<'static> {
    match icon {
        NavIcon::Home => egui::include_image!("../../assets/nav_home.png"),
        NavIcon::Charts => egui::include_image!("../../assets/nav_charts.png"),
        NavIcon::Profile => egui::include_image!("../../assets/nav_profile.png"),
        NavIcon::Avatar => egui::include_image!("../../assets/nav_avatar.png"),
        NavIcon::Settings => egui::include_image!("../../assets/nav_settings.png"),
    }
}

pub fn nav_item(ui: &mut egui::Ui, icon: NavIcon, label: &str, selected: bool) -> bool {
    let p = pal(ui);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(66.0, 58.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if selected || resp.hovered() {
        ui.painter().rect_filled(
            rect.shrink(3.0),
            CornerRadius::same(10),
            if selected { p.selected_bg } else { p.hover_bg },
        );
    }
    if selected {
        let bar = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 2.0, rect.center().y - 11.0),
            egui::pos2(rect.left() + 5.0, rect.center().y + 11.0),
        );
        ui.painter().rect_filled(bar, CornerRadius::same(2), GREEN);
    }
    let color = if selected { p.text } else { p.muted };
    let icon_rect = egui::Rect::from_center_size(
        rect.center() - egui::vec2(0.0, 8.0),
        egui::vec2(20.0, 20.0),
    );
    egui::Image::new(nav_icon_source(icon))
        .tint(color)
        .paint_at(ui, icon_rect);
    ui.painter().text(
        rect.center() + egui::vec2(0.0, 15.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11.0),
        color,
    );
    resp.clicked()
}

