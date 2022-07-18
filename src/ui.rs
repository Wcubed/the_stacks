use crate::localization::Localizer;
use crate::recipe::{OngoingRecipe, RECIPE_TITLE_LOCALIZATION_PREFIX};
use crate::stack::{Card, CardStack, HoveredCard};
use crate::{GameState, LengthOfDay, Speed, TimeOfDay, TimeSpeed};
use bevy::prelude::*;
use bevy_egui::egui::ProgressBar;
use bevy_egui::*;
use bevy_egui::{EguiContext, EguiPlugin};
use unic_langid::LanguageIdentifier;

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
const OPEN_MENU_WINDOW_OFFSET: egui::Vec2 = egui::vec2(OFFSETS.x, OFFSETS.y);

const DAY_PROGRESS_BAR_WIDTH: f32 = 400.;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
            .insert_resource(UiClaimsMouse(false))
            .add_system_set(
                SystemSet::on_update(GameState::Run)
                    .with_system(ui_mouse_claim_system)
                    .with_system(card_info_ui)
                    .with_system(card_crafting_info_ui)
                    .with_system(game_speed_ui)
                    .with_system(open_pause_menu_ui),
            )
            .add_system_set(SystemSet::on_update(GameState::PauseMenu).with_system(pause_menu_ui));
    }
}

pub struct UiClaimsMouse(pub bool);

/// Keeps track of whether the ui is currently claiming the mouse or not.
/// If the ui is not claiming the mouse, the game world can use it.
/// TODO (Wybe 2022-07-18): Add a similar system for the keyboard input.
fn ui_mouse_claim_system(mut context: ResMut<EguiContext>, mut claims_mouse: ResMut<UiClaimsMouse>) {
    claims_mouse.0 = context.ctx_mut().wants_pointer_input();
}

fn card_info_ui(
    mut context: ResMut<EguiContext>,
    hovered_card_query: Query<&Card, With<HoveredCard>>,
    localizer: Res<Localizer>,
) {
    if let Some(hovered_card) = hovered_card_query.iter().next() {
        egui::Window::new(hovered_card.localize_title(&localizer))
            .id(egui::Id::new("Card info window"))
            .fixed_size(CARD_INFO_SIZE)
            .anchor(egui::Align2::LEFT_BOTTOM, CARD_INFO_WINDOW_OFFSET)
            .collapsible(false)
            .show(context.ctx_mut(), |ui| {
                ui.label(hovered_card.localize_description(&localizer));

                if hovered_card.value.is_none() {
                    ui.label(localizer.localize("ui_cannot_be_sold"));
                }

                ui.allocate_space(ui.available_size())
            });
    }
}

fn card_crafting_info_ui(
    mut context: ResMut<EguiContext>,
    hovered_card_query: Query<&Parent, With<HoveredCard>>,
    stack_recipe_query: Query<&OngoingRecipe, With<CardStack>>,
    localizer: Res<Localizer>,
) {
    if let Some(hovered_card) = hovered_card_query.iter().next() {
        if let Ok(recipe) = stack_recipe_query.get(hovered_card.0) {
            let title_localization_id = RECIPE_TITLE_LOCALIZATION_PREFIX.to_string() + recipe.id.0;
            // TODO (Wybe 2022-06-19): Cache this string?
            let title = localizer.localize(&title_localization_id);

            let seconds_left = (recipe.timer.duration() - recipe.timer.elapsed()).as_secs_f32();
            let seconds_left_string = localizer.localize_with_args(
                "ui_seconds_left_in_recipe",
                &[("seconds", &format!("{:.1}", seconds_left))],
            );

            egui::Window::new(title)
                .id(egui::Id::new("Recipe window"))
                .fixed_size(RECIPE_INFO_SIZE)
                .anchor(egui::Align2::LEFT_BOTTOM, RECIPE_INFO_WINDOW_OFFSET)
                .collapsible(false)
                .show(context.ctx_mut(), |ui| {
                    ui.label(seconds_left_string);

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
    localizer: Res<Localizer>,
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
                let day_string = localizer
                    .localize_with_args("ui_current_day", &[("day", &time_of_day.day.to_string())]);
                let seconds_left_in_day_string = localizer.localize_with_args(
                    "ui_seconds_left_in_day",
                    &[("seconds", &format!("{:.0}", seconds_left_in_day))],
                );

                let day_progress = ProgressBar::new(time_of_day.time_of_day)
                    .desired_width(DAY_PROGRESS_BAR_WIDTH)
                    .text(day_string);
                ui.add(day_progress)
                    .on_hover_text(seconds_left_in_day_string);
            });
        });
}

fn open_pause_menu_ui(mut context: ResMut<EguiContext>, mut app_state: ResMut<State<GameState>>) {
    egui::Window::new("open_menu")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_TOP, OPEN_MENU_WINDOW_OFFSET)
        .show(context.ctx_mut(), |ui| {
            if ui.button("☰").clicked() {
                app_state.push(GameState::PauseMenu);
            }
        });
}

fn pause_menu_ui(
    mut context: ResMut<EguiContext>,
    mut app_state: ResMut<State<GameState>>,
    mut localizer: ResMut<Localizer>,
) {
    egui::Window::new(localizer.localize("ui_pause_menu_title"))
        .id(egui::Id::new("pause_menu"))
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .title_bar(true)
        .resizable(false)
        .collapsible(false)
        .show(context.ctx_mut(), |ui| {
            ui.vertical_centered_justified(|ui| {
                let mut selected = &localizer.current_language();

                let language_options = localizer.language_options();
                let mut sorted_language_identifiers: Vec<&LanguageIdentifier> =
                    language_options.keys().collect();
                sorted_language_identifiers.sort();

                egui::ComboBox::from_label(localizer.localize("ui_pause_menu_language_label"))
                    .selected_text(language_options[selected])
                    .show_ui(ui, |ui| {
                        for identifier in sorted_language_identifiers {
                            ui.selectable_value(
                                &mut selected,
                                identifier,
                                language_options[identifier],
                            );
                        }
                    });

                if selected != &localizer.current_language() {
                    // TODO (Wybe 2022-06-07): Update the titles of cards.
                    localizer.select_language(selected.clone());
                }

                if ui
                    .button(localizer.localize("ui_pause_menu_resume"))
                    .clicked()
                {
                    app_state.pop();
                }
            });
        });
}
