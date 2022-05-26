use crate::card::{Card, HoveredCard};
use bevy::prelude::*;
use bevy_egui::*;
use bevy_egui::{EguiContext, EguiPlugin};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin).add_system(card_info_ui);
    }
}

fn card_info_ui(
    mut context: ResMut<EguiContext>,
    hovered_card_query: Query<&Card, With<HoveredCard>>,
) {
    let info_window_size = egui::Vec2::new(200.0, 100.0);
    let info_window_offset = egui::Vec2::new(10.0, -10.0);

    if let Some(hovered_card) = hovered_card_query.iter().next() {
        egui::Window::new(hovered_card.title)
            .id(egui::Id::new("Info window"))
            .fixed_size(info_window_size)
            .anchor(egui::Align2::LEFT_BOTTOM, info_window_offset)
            .collapsible(false)
            .show(context.ctx_mut(), |ui| {
                ui.label(hovered_card.description);

                if hovered_card.value.is_none() {
                    ui.label("Cannot be sold.");
                }

                ui.allocate_space(ui.available_size())
            });
    }
}
