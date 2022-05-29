use crate::card_packs::{BUY_FOREST_PACK, FOREST_PACK};
use crate::card_types::CardCategory::{SystemCard, Worker};
use crate::card_types::{COIN, LOG, MARKET, PLANK, TREE};
use crate::recipe::{FinishRecipeMarker, RecipeUses, Recipes, RecipesBuilder};
use crate::stack::{Card, CardStack};
use crate::stack_utils::{delete_cards, StackCreation};
use bevy::prelude::*;

pub fn build_recipes(world: &mut World) -> Recipes {
    RecipesBuilder::new(world)
        .with_recipe(
            "Cutting tree",
            2.,
            |cards| {
                // Exactly 1 of type Worker, and the rest trees.
                cards.iter().any(|c| c.category == Worker)
                    && cards.iter().filter(|c| c.is_type(TREE)).count() == cards.len() - 1
                    && cards.len() > 1
            },
            |mut commands: Commands,
             recipe_stack_query: Query<
                (Entity, &CardStack, &GlobalTransform),
                With<FinishRecipeMarker>,
            >,
             mut card_query: Query<(&Card, &mut RecipeUses)>,
             creation: Res<StackCreation>| {
                for (root, stack, global_transform) in recipe_stack_query.iter() {
                    for &card_entity in stack.iter() {
                        if let Ok((card, mut uses)) = card_query.get_mut(card_entity) {
                            if card.is_type(TREE) {
                                // The recipe consumes 1 use of a tree.
                                if uses.0 == 1 {
                                    delete_cards(&mut commands, &[card_entity], root, stack);
                                } else {
                                    uses.0 -= 1;
                                }

                                creation.spawn_stack(
                                    &mut commands,
                                    global_transform.translation.truncate(),
                                    LOG,
                                    1,
                                    true,
                                );
                                break;
                            }
                        }
                    }
                }
            },
        )
        .with_recipe(
            "Making plank",
            3.,
            |cards| {
                cards.len() == 2
                    && cards.iter().any(|c| c.is_type(LOG))
                    && cards.iter().any(|c| c.category == Worker)
            },
            |mut commands: Commands,
             recipe_stack_query: Query<
                (Entity, &CardStack, &GlobalTransform),
                With<FinishRecipeMarker>,
            >,
             card_query: Query<&Card>,
             creation: Res<StackCreation>| {
                for (root, stack, global_transform) in recipe_stack_query.iter() {
                    for &card_entity in stack.iter() {
                        if let Ok(card) = card_query.get(card_entity) {
                            if card.is_type(LOG) {
                                // The recipe consumes a single log.
                                delete_cards(&mut commands, &[card_entity], root, stack);

                                creation.spawn_stack(
                                    &mut commands,
                                    global_transform.translation.truncate(),
                                    PLANK,
                                    1,
                                    true,
                                );
                                break;
                            }
                        }
                    }
                }
            },
        )
        .with_instant_recipe(
            "Sell cards",
            |cards| {
                // Bottom card is a market, and there are sellable cards.
                // SystemCards are never sellable.
                cards.first().filter(|c| c.is_type(MARKET)).is_some()
                    && cards
                        .iter()
                        .any(|c| c.value.is_some() && c.category != SystemCard)
            },
            |mut commands: Commands,
             recipe_stack_query: Query<
                (Entity, &CardStack, &GlobalTransform),
                With<FinishRecipeMarker>,
            >,
             card_query: Query<&Card>,
             creation: Res<StackCreation>| {
                for (root, stack, global_transform) in recipe_stack_query.iter() {
                    let mut total_value = 0;

                    // The recipe consumes all the cards that have a value.
                    let cards_with_value: Vec<Entity> = stack
                        .iter()
                        .filter_map(|&entity| {
                            if let Ok(card) = card_query.get(entity) {
                                if card.category == SystemCard {
                                    // System cards can never be sold.
                                    return None;
                                }

                                if let Some(value) = card.value {
                                    total_value += value;
                                    Some(entity)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect();

                    if !cards_with_value.is_empty() {
                        delete_cards(&mut commands, &cards_with_value, root, stack);
                    }

                    if total_value > 0 {
                        creation.spawn_stack(
                            &mut commands,
                            global_transform.translation.truncate(),
                            COIN,
                            total_value,
                            true,
                        );
                    }
                }
            },
        )
        .with_instant_recipe(
            "Buy card pack",
            |cards| {
                // Bottom card is one of the card pack buy cards, and there are enough coins.

                let bottom_card = cards.first().unwrap();
                let cost = if bottom_card.is_type(BUY_FOREST_PACK) {
                    bottom_card.value.unwrap()
                } else {
                    // Card is not one of the cards that allow buying packs.
                    return false;
                };
                // Enough coins?
                cards.iter().filter(|c| c.is_type(COIN)).count() >= cost
            },
            |mut commands: Commands,
             recipe_stack_query: Query<
                (Entity, &CardStack, &GlobalTransform),
                With<FinishRecipeMarker>,
            >,
             card_query: Query<&Card>,
             creation: Res<StackCreation>| {
                for (root, stack, global_transform) in recipe_stack_query.iter() {
                    let pack_cost = card_query.get(stack[0]).unwrap().value.unwrap();

                    let coins_to_delete: Vec<Entity> = stack
                        .iter()
                        .filter_map(|&entity| {
                            if let Ok(card) = card_query.get(entity) {
                                if card.is_type(COIN) {
                                    Some(entity)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .take(pack_cost)
                        .collect();

                    if coins_to_delete.len() != pack_cost {
                        // Not enough coins.
                        return;
                    }

                    if !coins_to_delete.is_empty() {
                        delete_cards(&mut commands, &coins_to_delete, root, stack);
                    }

                    // Spawn pack.
                    creation.spawn_stack(
                        &mut commands,
                        global_transform.translation.truncate(),
                        FOREST_PACK,
                        1,
                        true,
                    );
                }
            },
        )
        .build()
}
