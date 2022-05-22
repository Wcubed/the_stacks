use crate::card::{Card, CardsInStack, IsBottomCardOfStack};
use crate::card_types::CardType::Worker;
use crate::{card_types, GameState};
use bevy::prelude::*;
use std::collections::HashMap;

/// Handles recipes on card stacks
/// Requires [CardPlugin].
pub struct RecipePlugin;

impl Plugin for RecipePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RecipeReadyMarker>().add_system_set(
            SystemSet::on_update(GameState::Run)
                .with_system(recipe_check_system)
                .with_system(recipe_cleanup_system)
                .with_system(recipe_timer_update_system)
                .with_system(recipe_finished_exclusive_system.exclusive_system().at_end()),
        );

        let recipes = Recipes::default().with(Recipe {
            name: "Cutting tree".to_owned(),
            valid_callback: |cards| {
                // Exactly 1 of type Worker, and the rest trees.
                cards.iter().any(|c| c.card_type == Worker)
                    && cards.iter().filter(|c| c == &&&card_types::TREE).count() == cards.len() - 1
                    && cards.len() > 1
            },
        });

        app.insert_resource(recipes);
    }
}

/// Component indicating the id of an ongoing recipe.
#[derive(Component, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub struct RecipeId(usize);

/// Component that marks a stack with a recipe ready to be finished.
/// This is read by [recipe_finished_exclusive_system], which will finish the recipe.
#[derive(Component)]
pub struct RecipeReadyMarker(RecipeId);

/// Resource listing all the possible recipes.
#[derive(Default)]
pub struct Recipes {
    pub recipes: HashMap<RecipeId, Recipe>,
    next_id: RecipeId,
}

impl Recipes {
    pub fn with(mut self, new_recipe: Recipe) -> Self {
        self.recipes.insert(self.next_id, new_recipe);
        self.next_id.0 += 1;
        self
    }
}

pub struct Recipe {
    name: String,
    valid_callback: fn(&Vec<&Card>) -> bool,
}

/// Checks whether stacks are valid recipes or not.
pub fn recipe_check_system(
    mut commands: Commands,
    stacks: Query<(&CardsInStack, Option<&RecipeId>), Changed<CardsInStack>>,
    cards: Query<&Card>,
    recipes: Res<Recipes>,
) {
    for (stack, maybe_ongoing_recipe) in stacks.iter() {
        let root_card = stack[0];

        let cards_in_stack = stack.iter().map(|&e| cards.get(e).unwrap()).collect();

        let mut recipe_found = false;

        if let Some(ongoing_recipe_id) = maybe_ongoing_recipe {
            if let Some(ongoing_recipe) = recipes.recipes.get(ongoing_recipe_id) {
                if (ongoing_recipe.valid_callback)(&cards_in_stack) {
                    recipe_found = true;
                }
            }
        }

        if !recipe_found {
            for (&id, recipe) in recipes.recipes.iter() {
                if (recipe.valid_callback)(&cards_in_stack) {
                    commands.entity(root_card).insert(id);

                    // Stop at the first recipe found (best not to have overlapping recipes)
                    recipe_found = true;
                    break;
                }
            }
        }

        if !recipe_found {
            commands.entity(root_card).remove::<RecipeId>();
        }
    }
}

/// Removes recipe markers from cards that are no longer the root of a stack.
pub fn recipe_cleanup_system(
    mut commands: Commands,
    cards_that_are_no_longer_stacks: Query<Entity, (With<RecipeId>, Without<CardsInStack>)>,
) {
    for card in cards_that_are_no_longer_stacks.iter() {
        commands.entity(card).remove::<RecipeId>();
    }
}

pub fn recipe_timer_update_system(
    mut commands: Commands,
    ongoing_recipes: Query<(Entity, &RecipeId), With<CardsInStack>>,
) {
    // TODO (Wybe 2022-05-22): Add an actual timer.
    for (root_card, &recipe_id) in ongoing_recipes.iter() {
        commands
            .entity(root_card)
            .insert(RecipeReadyMarker(recipe_id))
            .remove::<RecipeId>();
    }
}

/// System that handles recipes that finish.
/// Has full mutable access to the `World`, so there shouldn't be too many limits on what
/// recipes can do when they complete.
pub fn recipe_finished_exclusive_system(world: &mut World) {
    let mut ready_recipes = world.query::<(&CardsInStack, &RecipeReadyMarker)>();
    let recipes = world.get_resource::<Recipes>().unwrap();

    let mut finished_recipe_roots = Vec::new();

    for (stack, RecipeReadyMarker(id)) in ready_recipes.iter(world) {
        if let Some(recipe) = recipes.recipes.get(&id) {
            // TODO (Wybe 2022-05-22): Put a check in to see if the recipe is still valid?
            println!("Recipe finished: {}", recipe.name);

            finished_recipe_roots.push(stack[0])
        }
    }

    for root_card in finished_recipe_roots {
        world
            .get_entity_mut(root_card)
            .unwrap()
            .remove::<RecipeReadyMarker>();
    }
}
