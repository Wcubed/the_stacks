use crate::card::{Card, CardStack, CardVisualSize, STACK_DRAG_Z};
use crate::card_types::CardType::Worker;
use crate::card_types::{APPLE, LOG, PLANK};
use crate::stack_utils::{delete_card, StackCreation};
use crate::{card_types, GameState};
use bevy::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

/// Progress bars are located just underneath the dragged stacks on the z order.
/// Contrary to dragged stacks, progress bar Z's are relative to their parent,
/// so any dragged stack will have it's progress bar
/// visible above the others.
const RECIPE_PROGRESS_BAR_Z: f32 = STACK_DRAG_Z - 10.;

const RECIPE_PROGRESS_BAR_HEIGHT: f32 = 20.;
const RECIPE_PROGRESS_BAR_COLOR: Color = Color::WHITE;

/// Handles recipes on card stacks
/// Requires [CardPlugin].
pub struct RecipePlugin;

impl Plugin for RecipePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RecipeReadyMarker>().add_system_set(
            SystemSet::on_update(GameState::Run)
                .with_system(recipe_check_system)
                .with_system(recipe_timer_update_system)
                .with_system(recipe_timer_graphics_system)
                .with_system(recipe_finished_exclusive_system.exclusive_system().at_end()),
        );

        let world = &mut app.world;

        let recipes = RecipesBuilder::new(world)
            .with(
                "Cutting tree",
                2.,
                |cards| {
                    // Exactly 1 of type Worker, and the rest trees.
                    cards.iter().any(|c| c.card_type == Worker)
                        && cards.iter().filter(|c| c == &&&card_types::TREE).count()
                            == cards.len() - 1
                        && cards.len() > 1
                },
                |mut commands: Commands,
                 recipe_stack_query: Query<
                    (Entity, &CardStack, &GlobalTransform),
                    With<FinishRecipeMarker>,
                >,
                 card_query: Query<&Card>,
                 creation: Res<StackCreation>| {
                    for (root, stack, global_transform) in recipe_stack_query.iter() {
                        creation.spawn_stack(
                            &mut commands,
                            global_transform.translation.truncate(),
                            &[LOG, LOG],
                        );
                        creation.spawn_stack(
                            &mut commands,
                            global_transform.translation.truncate(),
                            &[APPLE],
                        );

                        for &card_entity in stack.iter() {
                            let card = card_query.get(card_entity).unwrap();
                            if card == &card_types::TREE {
                                // The recipe consumes a single tree.
                                delete_card(&mut commands, card_entity, root, stack);
                                break;
                            }
                        }
                    }
                },
            )
            .with(
                "Making plank",
                3.,
                |cards| {
                    cards.len() == 2
                        && cards.contains(&&LOG)
                        && cards.iter().any(|c| c.card_type == Worker)
                },
                |mut commands: Commands,
                 recipe_stack_query: Query<
                    (Entity, &CardStack, &GlobalTransform),
                    With<FinishRecipeMarker>,
                >,
                 card_query: Query<&Card>,
                 creation: Res<StackCreation>| {
                    for (root, stack, global_transform) in recipe_stack_query.iter() {
                        creation.spawn_stack(
                            &mut commands,
                            global_transform.translation.truncate(),
                            &[PLANK],
                        );

                        for &card_entity in stack.iter() {
                            let card = card_query.get(card_entity).unwrap();
                            if card == &card_types::LOG {
                                // The recipe consumes a single log.
                                delete_card(&mut commands, card_entity, root, stack);
                                break;
                            }
                        }
                    }
                },
            )
            .build();

        app.insert_resource(recipes);
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct RecipeId(&'static str);

#[derive(Component)]
pub struct OngoingRecipe {
    id: RecipeId,
    timer: Timer,
}

/// Component indicating a progress bar hovering over a currently ongoing recipe.
#[derive(Component)]
pub struct RecipeProgressBar;

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
        time: f32,
        valid_callback: fn(&Vec<&Card>) -> bool,
        finished_system: impl IntoSystem<(), (), Params> + 'static,
    ) -> Self {
        let mut boxed_system = Box::new(IntoSystem::into_system(finished_system));
        boxed_system.initialize(self.world);

        let id = RecipeId(name);

        let new_recipe = Recipe {
            time,
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
    /// Time the recipe takes, in seconds.
    time: f32,
    /// This callback is called when cards are added or removed from stacks.
    /// Should return `true` if the given stack contents are valid for this recipe.
    valid_callback: fn(&Vec<&Card>) -> bool,
    /// System that applies the effects of a recipe.
    /// Only called a maximum of once per frame.
    /// The stacks that need to be handled will be indicated by a [FinishRecipeMarker].
    /// If there are multiple stacks ready with this recipe, they need to be handled all at once.
    /// Any commands given to a [Commands] object will be applied immediately afterwards.
    ///
    /// Do not worry about leaving a [FinishRecipeMarker] lying around,
    /// it will be cleaned up automatically.
    finish_system: Box<dyn System<In = (), Out = ()>>,
}

/// Checks whether stacks are valid recipes or not.
pub fn recipe_check_system(
    mut commands: Commands,
    stacks: Query<(Entity, &CardStack, Option<&OngoingRecipe>), Changed<CardStack>>,
    cards: Query<&Card>,
    recipes: Res<Recipes>,
) {
    for (root, stack, maybe_ongoing_recipe) in stacks.iter() {
        let cards_in_stack = stack.iter().map(|&e| cards.get(e).unwrap()).collect();

        let mut recipe_found = false;

        if let Some(ongoing_recipe) = maybe_ongoing_recipe {
            if let Some(recipe) = recipes.get(&ongoing_recipe.id) {
                if (recipe.valid_callback)(&cards_in_stack) {
                    recipe_found = true;
                }
            }
        }

        if !recipe_found {
            for (&id, recipe) in recipes.iter() {
                if (recipe.valid_callback)(&cards_in_stack) {
                    commands.entity(root).insert(OngoingRecipe {
                        id,
                        // TODO (Wybe 2022-05-25): Allow recipes to have differing durations.
                        timer: Timer::new(Duration::from_secs_f32(recipe.time), false),
                    });

                    // Stop at the first recipe found (best not to have overlapping recipes)
                    recipe_found = true;
                    break;
                }
            }
        }

        if !recipe_found {
            commands.entity(root).remove::<OngoingRecipe>();
        }
    }
}

pub fn recipe_timer_update_system(
    mut commands: Commands,
    mut ongoing_recipes: Query<(Entity, &mut OngoingRecipe), With<CardStack>>,
    time: Res<Time>,
) {
    // TODO (Wybe 2022-05-22): Add an actual timer.
    for (root, mut recipe) in ongoing_recipes.iter_mut() {
        recipe.timer.tick(time.delta());

        if recipe.timer.finished() {
            commands
                .entity(root)
                .insert(RecipeReadyMarker(recipe.id))
                .remove::<OngoingRecipe>();
        }
    }
}

pub fn recipe_timer_graphics_system(
    mut commands: Commands,
    mut recipe_progress_bars: Query<
        (Entity, &Parent, &mut Sprite, &mut Transform),
        With<RecipeProgressBar>,
    >,
    ongoing_recipes: Query<&OngoingRecipe, (With<CardStack>, Changed<OngoingRecipe>)>,
    stacks_with_new_recipes: Query<Entity, (With<CardStack>, Added<OngoingRecipe>)>,
    card_visual_size: Res<CardVisualSize>,
) {
    // Create new progress bars
    for root in stacks_with_new_recipes.iter() {
        let progress_bar = commands
            .spawn_bundle(SpriteBundle {
                transform: Transform::from_xyz(
                    0.,
                    card_visual_size.y * 0.5 + RECIPE_PROGRESS_BAR_HEIGHT,
                    RECIPE_PROGRESS_BAR_Z,
                ),
                sprite: Sprite {
                    color: RECIPE_PROGRESS_BAR_COLOR,
                    custom_size: Some(Vec2::new(0., RECIPE_PROGRESS_BAR_HEIGHT)),
                    ..default()
                },
                ..default()
            })
            .insert(RecipeProgressBar)
            .id();

        commands.entity(root).add_child(progress_bar);
    }

    // Update existing progress bars
    for (entity, root, mut sprite, mut transform) in recipe_progress_bars.iter_mut() {
        if let Ok(recipe) = ongoing_recipes.get(root.0) {
            let new_width = recipe.timer.percent() * card_visual_size.x;

            sprite.custom_size = Some(Vec2::new(new_width, RECIPE_PROGRESS_BAR_HEIGHT));
            transform.translation.x = (-card_visual_size.x * 0.5) + (new_width * 0.5);
        } else {
            // Recipe no longer ongoing. Remove progress bar.
            commands.entity(entity).despawn_recursive();
        }
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
