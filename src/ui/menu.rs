use eframe::egui::{self, CornerRadius, Sense};
use crate::theme::{lerp_color, pal, GREEN};
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
    let id = ui.scope_id().with(("nav_item", label));
    let hover_t = ui.ctx().animate_bool(id.with("hover"), resp.hovered());
    let sel_t = ui.ctx().animate_bool(id.with("sel"), selected);
    let bg_t = hover_t.max(sel_t);
    if bg_t > 0.01 {
        let base_fill = lerp_color(p.hover_bg, p.selected_bg, sel_t);
        ui.painter().rect_filled(
            rect.shrink(3.0),
            CornerRadius::same(10),
            base_fill.gamma_multiply(bg_t),
        );
    }
    if selected {
        let bar_y = ui.ctx().animate_value_with_time(
            egui::Id::NULL.with("nav_active_bar_y"),
            rect.center().y,
            0.16,
        );
        let bar = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 2.0, bar_y - 11.0),
            egui::pos2(rect.left() + 5.0, bar_y + 11.0),
        );
        ui.painter().rect_filled(bar, CornerRadius::same(2), GREEN);
    }
    let color = lerp_color(p.muted, p.text, sel_t.max(hover_t * 0.75));
    let lift = hover_t * 1.5;
    let icon_rect = egui::Rect::from_center_size(
        rect.center() - egui::vec2(0.0, 8.0 + lift),
        egui::vec2(20.0, 20.0),
    );
    egui::Image::new(nav_icon_source(icon))
        .tint(color)
        .paint_at(ui, icon_rect);
    ui.painter().text(
        rect.center() + egui::vec2(0.0, 15.0 - lift * 0.35),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11.0),
        color,
    );
    resp.clicked()
}

