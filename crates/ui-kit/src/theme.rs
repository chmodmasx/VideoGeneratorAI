use egui::{CornerRadius, Stroke, Style, Visuals};

use crate::tokens;

pub fn install(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = tokens::PANEL;
    visuals.window_fill = tokens::PANEL;
    visuals.extreme_bg_color = tokens::BACKGROUND;
    visuals.faint_bg_color = tokens::RAISED;
    visuals.selection.bg_fill = tokens::ACCENT;
    visuals.widgets.inactive.bg_fill = tokens::RAISED;
    visuals.widgets.inactive.corner_radius =
        CornerRadius::same(tokens::CORNER_RADIUS);
    visuals.widgets.hovered.corner_radius =
        CornerRadius::same(tokens::CORNER_RADIUS);
    visuals.widgets.active.corner_radius =
        CornerRadius::same(tokens::CORNER_RADIUS);
    visuals.widgets.noninteractive.bg_stroke =
        Stroke::new(1.0, tokens::BORDER);

    let mut style = Style::default();
    style.visuals = visuals;
    style.spacing.item_spacing = tokens::ITEM_SPACING;
    style.spacing.button_padding = tokens::BUTTON_PADDING;
    ctx.set_style(style);
}
