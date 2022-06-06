use crate::recipe::OngoingRecipe;
use crate::stack::{Card, CardDescription, CardStack, HoveredCard};
use crate::{LengthOfDay, Speed, TimeOfDay, TimeSpeed};
use bevy::prelude::*;
use bevy_egui::egui::ProgressBar;
use bevy_egui::*;
use bevy_egui::{EguiContext, EguiPlugin};

/// Title height is a guess. Needed to calculate window positions,
/// because setting the window size does not include the title.
/// TODO (Wybe 2022-05-26): Can we get this from egui?
const TITLE_HEIGHT: f32 = 50.0;

const OFFSETS: egui::Vec2 = egui::vec2(10.0, 10.0);

const CARD_INFO_SIZE: egui::Vec2 = egui::vec2(200.0, 100.0);
const CARD_INFO_WINDOW_OFFSET: egui::Vec2 = egui::vec2(OFFSETS.x, -OFFSETS.y);
const RECIPE_INFO_SIZE: egui::Vec2 = egui::vec2(CARD_INFO_SIZE.x, 20.0);
const RECIPE_INFO_WINDOW_OFFSET: egui::Vec2 = egui::vec2(
    CARD_INFO_WINDOW_OFFSET.x,
    -(CARD_INFO_SIZE.y + TITLE_HEIGHT) + CARD_INFO_WINDOW_OFFSET.y,
);
const GAME_SPEED_WINDOW_OFFSET: egui::Vec2 = egui::vec2(-OFFSETS.x, OFFSETS.y);

const DAY_PROGRESS_BAR_WIDTH: f32 = 400.;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
            .add_system(card_info_ui)
            .add_system(card_crafting_info_ui)
            .add_system(game_speed_ui);
    }
}

fn card_info_ui(
    mut context: ResMut<EguiContext>,
    hovered_card_query: Query<(&Card, &CardDescription), With<HoveredCard>>,
) {
    if let Some((hovered_card, description)) = hovered_card_query.iter().next() {
        egui::Window::new(hovered_card.title)
            .id(egui::Id::new("Card info window"))
            .fixed_size(CARD_INFO_SIZE)
            .anchor(egui::Align2::LEFT_BOTTOM, CARD_INFO_WINDOW_OFFSET)
            .collapsible(false)
            .show(context.ctx_mut(), |ui| {
                ui.label(description.0);

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

fn game_speed_ui(
    mut context: ResMut<EguiContext>,
    mut speed: ResMut<TimeSpeed>,
    time_of_day: Res<TimeOfDay>,
    length_of_day: Res<LengthOfDay>,
) {
    egui::Window::new("speed_window")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_TOP, GAME_SPEED_WINDOW_OFFSET)
        .show(context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                let mut paused = !speed.running;
                ui.toggle_value(&mut paused, "||").on_hover_text("[space]");
                speed.running = !paused;

                // TODO (Wybe 2022-05-28): Use a key mapping plugin, instead of hardcoding.
                ui.selectable_value(&mut speed.speed, Speed::NORMAL, ">")
                    .on_hover_text("[1]");
                ui.selectable_value(&mut speed.speed, Speed::DOUBLE, ">>")
                    .on_hover_text("[2]");
                ui.selectable_value(&mut speed.speed, Speed::TRIPLE, ">>>")
                    .on_hover_text("[3]");

                let seconds_left_in_day = (1.0 - time_of_day.time_of_day) * length_of_day.0;

                let day_progress = ProgressBar::new(time_of_day.time_of_day)
                    .desired_width(DAY_PROGRESS_BAR_WIDTH)
                    .text(format!("Day {}", time_of_day.day));
                ui.add(day_progress)
                    .on_hover_text(format!("{:.1} seconds left", seconds_left_in_day));
            });
        });
}
