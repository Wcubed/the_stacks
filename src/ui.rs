use crate::card::{Card, CardStack, HoveredCard};
use crate::recipe::{OngoingRecipe, Recipes};
use bevy::prelude::*;
use bevy_egui::*;
use bevy_egui::{EguiContext, EguiPlugin};

/// Title height is a guess. Needed to calculate window positions,
/// because setting the window size does not include the title.
/// TODO (Wybe 2022-05-26): Can we get this from egui?
const TITLE_HEIGHT: f32 = 50.0;

const CARD_INFO_SIZE: egui::Vec2 = egui::vec2(200.0, 100.0);
const CARD_INFO_WINDOW_OFFSET: egui::Vec2 = egui::vec2(10.0, -10.0);
const RECIPE_INFO_SIZE: egui::Vec2 = egui::vec2(CARD_INFO_SIZE.x, 20.0);
const RECIPE_INFO_WINDOW_OFFSET: egui::Vec2 = egui::vec2(
    CARD_INFO_WINDOW_OFFSET.x,
    -(CARD_INFO_SIZE.y + TITLE_HEIGHT) + CARD_INFO_WINDOW_OFFSET.y,
);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
            .add_system(card_info_ui)
            .add_system(card_crafting_info_ui);
    }
}

fn card_info_ui(
    mut context: ResMut<EguiContext>,
    hovered_card_query: Query<&Card, With<HoveredCard>>,
) {
    if let Some(hovered_card) = hovered_card_query.iter().next() {
        egui::Window::new(hovered_card.title)
            .id(egui::Id::new("Card info window"))
            .fixed_size(CARD_INFO_SIZE)
            .anchor(egui::Align2::LEFT_BOTTOM, CARD_INFO_WINDOW_OFFSET)
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

fn card_crafting_info_ui(
    mut context: ResMut<EguiContext>,
    hovered_card_query: Query<&Parent, With<HoveredCard>>,
    stack_recipe_query: Query<&OngoingRecipe, With<CardStack>>,
    recipes: Res<Recipes>,
) {
    if let Some(hovered_card) = hovered_card_query.iter().next() {
        if let Ok(recipe) = stack_recipe_query.get(hovered_card.0) {
            egui::Window::new(recipe.id.0)
                .id(egui::Id::new("Recipe window"))
                .fixed_size(RECIPE_INFO_SIZE)
                .anchor(egui::Align2::LEFT_BOTTOM, RECIPE_INFO_WINDOW_OFFSET)
                .collapsible(false)
                .show(context.ctx_mut(), |ui| {
                    ui.label(format!(
                        "{:.1} seconds",
                        (recipe.timer.duration() - recipe.timer.elapsed()).as_secs_f32()
                    ));

                    ui.allocate_space(ui.available_size())
                });
        }
    }
}
