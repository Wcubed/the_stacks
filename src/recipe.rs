use crate::card::{Card, CardCreation, CardStack};
use crate::card_types::CardType::Worker;
use crate::card_types::{LOG, PLANK};
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

        let world = &mut app.world;

        let recipes = RecipesBuilder::new(world)
            .with(
                "Cutting tree",
                |cards| {
                    // Exactly 1 of type Worker, and the rest trees.
                    cards.iter().any(|c| c.card_type == Worker)
                        && cards.iter().filter(|c| c == &&&card_types::TREE).count()
                            == cards.len() - 1
                        && cards.len() > 1
                },
                |mut commands: Commands,
                 recipe_stack_query: Query<&GlobalTransform, With<FinishRecipeMarker>>,
                 creation: Res<CardCreation>| {
                    // TODO (Wybe 2022-05-23): Implement removing card from stack.
                    // TODO (Wybe 2022-05-23): Make a stack a separate entity, with the first card as a child. That way, any components that apply to the full stack, stay when you remove something from a stack.
                    for global_transform in recipe_stack_query.iter() {
                        creation.spawn_card(
                            &mut commands,
                            LOG,
                            global_transform.translation.truncate(),
                        );
                    }
                },
            )
            .with(
                "Making plank",
                |cards| {
                    cards.len() == 2
                        && cards.contains(&&LOG)
                        && cards.iter().any(|c| c.card_type == Worker)
                },
                |mut commands: Commands,
                 recipe_stack_query: Query<&GlobalTransform, With<FinishRecipeMarker>>,
                 creation: Res<CardCreation>| {
                    for global_transform in recipe_stack_query.iter() {
                        creation.spawn_card(
                            &mut commands,
                            PLANK,
                            global_transform.translation.truncate(),
                        );
                    }
                },
            )
            .build();

        app.insert_resource(recipes);
    }
}

/// Component indicating the id of an ongoing recipe.
#[derive(Component, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub struct RecipeId(&'static str);

/// Component that marks a stack with a recipe ready to be finished.
/// This is read by [recipe_finished_exclusive_system], which will finish the recipe.
#[derive(Component)]
pub struct RecipeReadyMarker(RecipeId);

/// Marker that indicates which stacks should have their recipe finished when a [Recipe]'s `finish_system` is called.
/// Are placed by [recipe_finished_exclusive_system], and any stray ones are automatically cleaned up.
#[derive(Component)]
pub struct FinishRecipeMarker;

pub struct RecipesBuilder<'a> {
    world: &'a mut World,
    recipes: HashMap<RecipeId, Recipe>,
}

impl<'a> RecipesBuilder<'a> {
    fn new(world: &'a mut bevy::prelude::World) -> Self {
        RecipesBuilder {
            world,
            recipes: HashMap::new(),
        }
    }

    pub fn with<Params>(
        mut self,
        name: &'static str,
        valid_callback: fn(&Vec<&Card>) -> bool,
        finished_system: impl IntoSystem<(), (), Params> + 'static,
    ) -> Self {
        let mut boxed_system = Box::new(IntoSystem::into_system(finished_system));
        boxed_system.initialize(&mut self.world);

        let id = RecipeId(name);

        let new_recipe = Recipe {
            valid_callback,
            finish_system: boxed_system,
        };

        self.recipes.insert(id, new_recipe);
        self
    }

    pub fn build(self) -> Recipes {
        Recipes(self.recipes)
    }
}

/// Resource listing all the possible recipes.
#[derive(Default, Deref, DerefMut)]
pub struct Recipes(HashMap<RecipeId, Recipe>);

pub struct Recipe {
    /// This callback is called when cards are added or removed from stacks.
    /// Should return `true` if the given stack contents are valid for this recipe.
    valid_callback: fn(&Vec<&Card>) -> bool,
    /// System that applies the effects of a recipe.
    /// Only called a maximum of once per frame.
    /// The stacks that need to be handled will be indicated by a [FinishRecipeMarker].
    /// If there are multiple stacks ready with this recipe, they need to be handled all at once.
    ///
    /// Do not worry about leaving a [FinishRecipeMarker] lying around,
    /// it will be cleaned up automatically.
    finish_system: Box<dyn System<In = (), Out = ()>>,
}

/// Checks whether stacks are valid recipes or not.
pub fn recipe_check_system(
    mut commands: Commands,
    stacks: Query<(&CardStack, Option<&RecipeId>), Changed<CardStack>>,
    cards: Query<&Card>,
    recipes: Res<Recipes>,
) {
    for (stack, maybe_ongoing_recipe) in stacks.iter() {
        let root_card = stack[0];

        let cards_in_stack = stack.iter().map(|&e| cards.get(e).unwrap()).collect();

        let mut recipe_found = false;

        if let Some(ongoing_recipe_id) = maybe_ongoing_recipe {
            if let Some(ongoing_recipe) = recipes.get(ongoing_recipe_id) {
                if (ongoing_recipe.valid_callback)(&cards_in_stack) {
                    recipe_found = true;
                }
            }
        }

        if !recipe_found {
            for (&id, recipe) in recipes.iter() {
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
    cards_that_are_no_longer_stacks: Query<Entity, (With<RecipeId>, Without<CardStack>)>,
) {
    for card in cards_that_are_no_longer_stacks.iter() {
        commands.entity(card).remove::<RecipeId>();
    }
}

pub fn recipe_timer_update_system(
    mut commands: Commands,
    ongoing_recipes: Query<(Entity, &RecipeId), With<CardStack>>,
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
/// TODO (Wybe 2022-05-24): test finishing multiple different recipes, each for multiple stacks, at the same time.
pub fn recipe_finished_exclusive_system(world: &mut World) {
    let mut ready_recipes = world.query::<(Entity, &RecipeReadyMarker)>();

    // See which recipes have been finished, and the stacks that they apply to.
    let mut finished_recipes: HashMap<RecipeId, Vec<Entity>> = HashMap::new();
    for (root, RecipeReadyMarker(id)) in ready_recipes.iter(world) {
        if let Some(roots) = finished_recipes.get_mut(&id) {
            roots.push(root);
        } else {
            finished_recipes.insert(*id, vec![root]);
        }
    }

    // Clear all the `RecipeReadyMarker` components.
    for roots in finished_recipes.values() {
        for &root in roots {
            world
                .get_entity_mut(root)
                .and_then(|mut e| e.remove::<RecipeReadyMarker>());
        }
    }

    // Apply all the recipes.
    world.resource_scope(|world, mut recipes: Mut<Recipes>| {
        for (id, stack_roots) in finished_recipes {
            if let Some(recipe) = recipes.get_mut(&id) {
                // Mark the stacks that this recipe applies to.
                for &root in stack_roots.iter() {
                    if let Some(mut e) = world.get_entity_mut(root) {
                        e.insert(FinishRecipeMarker);
                    }
                }

                // TODO (Wybe 2022-05-22): Put a check in to see if the recipe is still valid?
                recipe.finish_system.run((), world);
                // Apply any generated commands.
                recipe.finish_system.apply_buffers(world);

                // Remove any marks that are still around
                for &root in stack_roots.iter() {
                    if let Some(mut e) = world.get_entity_mut(root) {
                        e.remove::<FinishRecipeMarker>();
                    }
                }
            }
        }
    });
}
