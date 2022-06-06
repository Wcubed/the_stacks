mod recipe_defines;

use crate::card_types::{CardCategory, CardType};
use crate::stack::{Card, CardStack, CardVisualSize, DELTA_Z, STACK_DRAG_Z};
use crate::{is_time_running, GameState, TimeSpeed};
use bevy::ecs::event::Events;
use bevy::prelude::*;
use recipe_defines::build_recipes;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Progress bars are located just underneath the dragged stacks on the z order.
/// Contrary to dragged stacks, progress bar Z's are relative to their parent,
/// so any dragged stack will have it's progress bar
/// visible above the others.
const RECIPE_PROGRESS_BAR_Z: f32 = STACK_DRAG_Z - 10.;

const RECIPE_PROGRESS_BAR_HEIGHT: f32 = 20.;
const RECIPE_PROGRESS_BAR_FOREGROUND: Color = Color::WHITE;
const RECIPE_PROGRESS_BAR_BACKGROUND: Color = Color::rgb(0.1, 0.1, 0.1);

/// Handles recipes on card stacks
/// Requires [CardPlugin].
pub struct RecipePlugin;

impl Plugin for RecipePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FinishedRecipeEvent>()
            .add_system_set(
                SystemSet::on_update(GameState::Run)
                    .with_system(recipe_check_system)
                    .with_system(recipe_timer_graphics_system)
                    .with_system(recipe_finished_exclusive_system.exclusive_system().at_end()),
            )
            .add_system_set(
                SystemSet::on_update(GameState::Run)
                    .with_run_criteria(is_time_running)
                    .with_system(recipe_timer_update_system),
            );

        let recipes = build_recipes(&mut app.world);
        app.insert_resource(recipes);
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct RecipeId(pub(crate) &'static str);

#[derive(Component, Clone)]
pub struct OngoingRecipe {
    pub id: RecipeId,
    pub timer: Timer,
}

/// Component indicating a progress bar hovering over a currently ongoing recipe.
#[derive(Component)]
pub struct RecipeProgressBar;

/// Component indicating the background of a progress bar hovering over a currently ongoing recipe.
#[derive(Component)]
pub struct RecipeProgressBarBackground;

/// Component that marks a stack with a recipe ready to be finished.
/// This is read by [recipe_finished_exclusive_system], which will finish the recipe.
#[derive(Component)]
pub struct RecipeReadyMarker(RecipeId);

/// Marker that indicates which stacks should have their recipe finished when a [Recipe]'s `finish_system` is called.
/// Are placed by [recipe_finished_exclusive_system], and any stray ones are automatically cleaned up.
#[derive(Component)]
pub struct FinishRecipeMarker;

/// Generic component that can be used by recipes to track how many "uses" there are left in a card.
/// For example: A worker could chop wood from a tree multiple times, before the tree is deleted.
#[derive(Component)]
pub struct RecipeUses(pub u32);

/// Event that happens when a recipe has finished.
/// Contains the relevant recipe id, and the root entity of the relevant stack.
pub struct FinishedRecipeEvent(RecipeId, Entity);

pub struct RecipesBuilder<'a> {
    world: &'a mut World,
    recipes: HashMap<RecipeId, Recipe>,
}

impl<'a> RecipesBuilder<'a> {
    pub fn new(world: &'a mut bevy::prelude::World) -> Self {
        RecipesBuilder {
            world,
            recipes: HashMap::new(),
        }
    }

    /// Instant recipes can be done while in-game time is paused.
    pub fn add_instant_recipe<Params>(
        &mut self,
        name: &'static str,
        valid_callback: fn(&StackCheck) -> bool,
        finished_system: impl IntoSystem<(), (), Params> + 'static,
    ) {
        self.new_recipe(name, None, valid_callback, finished_system);
    }

    pub fn add_recipe<Params>(
        &mut self,
        name: &'static str,
        seconds: f32,
        valid_callback: fn(&StackCheck) -> bool,
        finished_system: impl IntoSystem<(), (), Params> + 'static,
    ) {
        self.new_recipe(name, Some(seconds), valid_callback, finished_system);
    }

    fn new_recipe<Params>(
        &mut self,
        // TODO (Wybe 2022-06-05): Use these names as indexes into some sort of translation file.
        name: &'static str,
        seconds: Option<f32>,
        valid_callback: fn(&StackCheck) -> bool,
        finished_system: impl IntoSystem<(), (), Params> + 'static,
    ) {
        let mut boxed_system = Box::new(IntoSystem::into_system(finished_system));
        boxed_system.initialize(self.world);

        let id = RecipeId(name);

        let new_recipe = Recipe {
            seconds,
            is_valid: valid_callback,
            finish_system: boxed_system,
        };

        self.recipes.insert(id, new_recipe);
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
    /// When `None` the recipe is instant, and can be done even if the in-game time is paused.
    pub seconds: Option<f32>,
    /// This callback is called when cards are added or removed from stacks.
    /// Should return `true` if the given stack contents are valid for this recipe.
    pub is_valid: fn(&StackCheck) -> bool,
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

/// Convenience structure passed to the `is_valid` function of [Recipe]s, for checking various info about a stack.
#[derive(Deref)]
pub struct StackCheck(pub Vec<Card>);

impl StackCheck {
    fn bottom_card_is_type(&self, card_type: CardType) -> bool {
        if let Some(card) = self.0.get(0) {
            card.is_type(&card_type)
        } else {
            false
        }
    }

    fn contains_exactly_one_of_type(&self, card_type: CardType) -> bool {
        self.0.iter().filter(|&c| c.is_type(&card_type)).count() == 1
    }

    fn contains_n_of_type(&self, card_type: CardType, amount: usize) -> bool {
        self.0.iter().filter(|&c| c.is_type(&card_type)).count() == amount
    }

    fn contains_exactly_one_of_category(&self, category: CardCategory) -> bool {
        self.0.iter().filter(|&c| c.category == category).count() == 1
    }
}

/// Checks whether stacks are valid recipes or not.
pub fn recipe_check_system(
    mut commands: Commands,
    changed_stacks: Query<(
        Entity,
        &CardStack,
        Option<&OngoingRecipe>,
        ChangeTrackers<CardStack>,
    )>,
    cards: Query<&Card>,
    recipes: Res<Recipes>,
    mut finished_recipe_events: EventReader<FinishedRecipeEvent>,
) {
    let finished_recipe_roots: HashSet<Entity> = finished_recipe_events
        .iter()
        .map(|FinishedRecipeEvent(_, root)| *root)
        .collect();

    for (root, stack, maybe_ongoing_recipe, stack_changed) in changed_stacks.iter() {
        if !stack_changed.is_changed() && !finished_recipe_roots.contains(&root) {
            // This stack didn't change, nor did it have a recipe finish.
            // No need to check it again.
            continue;
        }

        let cards_in_stack: Vec<Card> = stack
            .iter()
            .filter_map(|&e| cards.get(e).ok())
            .copied()
            .collect();
        let stack_check = StackCheck(cards_in_stack);

        let mut recipe_found =
            is_ongoing_recipe_valid_for_stack(maybe_ongoing_recipe, &stack_check, &recipes);

        if !recipe_found {
            for (&id, recipe) in recipes.iter() {
                if (recipe.is_valid)(&stack_check) {
                    if let Some(seconds) = recipe.seconds {
                        commands.entity(root).insert(OngoingRecipe {
                            id,
                            timer: Timer::new(Duration::from_secs_f32(seconds), false),
                        });
                    } else {
                        // This recipe is instant, so it is immediately ready.
                        commands.entity(root).insert(RecipeReadyMarker(id));
                    }

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

pub fn is_ongoing_recipe_valid_for_stack(
    maybe_ongoing: Option<&OngoingRecipe>,
    stack_check: &StackCheck,
    recipes: &Res<Recipes>,
) -> bool {
    if let Some(recipe) = maybe_ongoing.and_then(|r| recipes.get(&r.id)) {
        (recipe.is_valid)(stack_check)
    } else {
        false
    }
}

pub fn recipe_timer_update_system(
    mut commands: Commands,
    mut ongoing_recipes: Query<(Entity, &mut OngoingRecipe), With<CardStack>>,
    time: Res<Time>,
    speed: Res<TimeSpeed>,
) {
    for (root, mut recipe) in ongoing_recipes.iter_mut() {
        let progress = time.delta_seconds() * speed.speed_as_factor();
        recipe.timer.tick(Duration::from_secs_f32(progress));

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
    recipe_progress_bar_backgrounds: Query<(Entity, &Parent), With<RecipeProgressBarBackground>>,
    ongoing_recipes: Query<&OngoingRecipe, With<CardStack>>,
    stacks_with_new_recipes: Query<Entity, (With<CardStack>, Added<OngoingRecipe>)>,
    card_visual_size: Res<CardVisualSize>,
) {
    // Create new progress bars
    for root in stacks_with_new_recipes.iter() {
        let bar_y_pos = card_visual_size.y * 0.5 + RECIPE_PROGRESS_BAR_HEIGHT;

        let progress_bar = commands
            .spawn_bundle(SpriteBundle {
                transform: Transform::from_xyz(0., bar_y_pos, RECIPE_PROGRESS_BAR_Z),
                sprite: Sprite {
                    color: RECIPE_PROGRESS_BAR_FOREGROUND,
                    custom_size: Some(Vec2::new(0., RECIPE_PROGRESS_BAR_HEIGHT)),
                    ..default()
                },
                ..default()
            })
            .insert(RecipeProgressBar)
            .id();
        let progress_bar_background = commands
            .spawn_bundle(SpriteBundle {
                transform: Transform::from_xyz(0., bar_y_pos, RECIPE_PROGRESS_BAR_Z - DELTA_Z),
                sprite: Sprite {
                    color: RECIPE_PROGRESS_BAR_BACKGROUND,
                    custom_size: Some(Vec2::new(card_visual_size.x, RECIPE_PROGRESS_BAR_HEIGHT)),
                    ..default()
                },
                ..default()
            })
            .insert(RecipeProgressBarBackground)
            .id();

        commands
            .entity(root)
            .add_child(progress_bar)
            .add_child(progress_bar_background);
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

            // TODO (Wybe 2022-05-28): there is probably a better way, instead of searching.
            if let Some((background, _)) = recipe_progress_bar_backgrounds
                .iter()
                .find(|(_, &parent)| parent == *root)
            {
                commands.entity(background).despawn_recursive();
            }
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

    world.resource_scope(
        |world, mut recipe_finished_events: Mut<Events<FinishedRecipeEvent>>| {
            // Clear all the `RecipeReadyMarker` components.
            for (&recipe, roots) in finished_recipes.iter() {
                for &root in roots {
                    if let Some(mut root_mut) = world.get_entity_mut(root) {
                        root_mut.remove::<RecipeReadyMarker>();
                        // Let other systems know which recipes were finished.
                        recipe_finished_events.send(FinishedRecipeEvent(recipe, root));
                    }
                }
            }
        },
    );

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
