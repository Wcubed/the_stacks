use crate::card_packs::{BUY_FOREST_PACK, FOREST_PACK};
use crate::card_types::{CardCategory, APPLE};
use crate::card_types::{CLAY, COIN, LOG, MARKET, PLANK, TREE, VILLAGER};
use crate::procedural::SeededHasherResource;
use crate::recipe::{FinishRecipeMarker, RecipeUses, Recipes, RecipesBuilder};
use crate::stack::stack_utils::delete_cards;
use crate::stack::{Card, CardStack, CreateStackEvent};
use bevy::prelude::*;

pub fn build_recipes(world: &mut World) -> Recipes {
    RecipesBuilder::new(world)
        .with_recipe(
            "Cutting tree",
            2.,
            |cards| {
                // Contains only trees and workers
                cards.contains_exactly_one_of_category(CardCategory::Worker)
                    && cards.contains_n_of_type(TREE, cards.len() - 1)
                    && cards.len() > 1
            },
            |mut commands: Commands,
             recipe_stack_query: Query<
                (Entity, &CardStack, &GlobalTransform),
                With<FinishRecipeMarker>,
            >,
             mut card_query: Query<(&Card, &mut RecipeUses)>,
             seeded_hash: Res<SeededHasherResource>,
             mut creation: EventWriter<CreateStackEvent>| {
                for (root, stack, global_transform) in recipe_stack_query.iter() {
                    for &card_entity in stack.iter() {
                        if let Ok((card, mut uses)) = card_query.get_mut(card_entity) {
                            if card.is_type(&TREE) {
                                // The recipe consumes 1 use of a tree.
                                if uses.0 == 1 {
                                    delete_cards(&mut commands, &[card_entity], root, stack);
                                } else {
                                    uses.0 -= 1;
                                }

                                creation.send(CreateStackEvent {
                                    position: global_transform.translation.truncate(),
                                    card_type: &LOG,
                                    amount: 1,
                                });

                                // Sometimes, a tree also drops an apple.
                                let apple_percentage = 25;
                                let mut rng = seeded_hash.with(card_entity);
                                rng.with(uses.0);
                                if rng.value_in_range(0..100) < apple_percentage {
                                    creation.send(CreateStackEvent {
                                        position: global_transform.translation.truncate(),
                                        card_type: &APPLE,
                                        amount: 1,
                                    });
                                }
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
                    && cards.contains_exactly_one_of_type(LOG)
                    && cards.contains_exactly_one_of_category(CardCategory::Worker)
            },
            |mut commands: Commands,
             recipe_stack_query: Query<
                (Entity, &CardStack, &GlobalTransform),
                With<FinishRecipeMarker>,
            >,
             card_query: Query<&Card>,
             mut creation: EventWriter<CreateStackEvent>| {
                for (root, stack, global_transform) in recipe_stack_query.iter() {
                    for &card_entity in stack.iter() {
                        if let Ok(card) = card_query.get(card_entity) {
                            if card.is_type(&LOG) {
                                // The recipe consumes a single log.
                                delete_cards(&mut commands, &[card_entity], root, stack);

                                creation.send(CreateStackEvent {
                                    position: global_transform.translation.truncate(),
                                    card_type: &PLANK,
                                    amount: 1,
                                });
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
                cards.bottom_card_is_type(MARKET)
                    && cards
                        .iter()
                        .any(|c| c.value.is_some() && c.category != CardCategory::SystemCard)
            },
            |mut commands: Commands,
             recipe_stack_query: Query<
                (Entity, &CardStack, &GlobalTransform),
                With<FinishRecipeMarker>,
            >,
             card_query: Query<&Card>,
             mut creation: EventWriter<CreateStackEvent>| {
                for (root, stack, global_transform) in recipe_stack_query.iter() {
                    let mut total_value = 0;

                    // The recipe consumes all the cards that have a value.
                    let cards_with_value: Vec<Entity> = stack
                        .iter()
                        .filter_map(|&entity| {
                            if let Ok(card) = card_query.get(entity) {
                                if card.category == CardCategory::SystemCard {
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
                        creation.send(CreateStackEvent {
                            position: global_transform.translation.truncate(),
                            card_type: &COIN,
                            amount: total_value,
                        });
                    }
                }
            },
        )
        .with_instant_recipe(
            "Buy card pack",
            |cards| {
                // Bottom card is one of the card pack buy cards, and there are enough coins.

                let bottom_card = cards.first().unwrap();
                let cost = if bottom_card.is_type(&BUY_FOREST_PACK) {
                    bottom_card.value.unwrap()
                } else {
                    // Card is not one of the cards that allow buying packs.
                    return false;
                };
                // Enough coins?
                cards.iter().filter(|c| c.is_type(&COIN)).count() >= cost
            },
            |mut commands: Commands,
             recipe_stack_query: Query<
                (Entity, &CardStack, &GlobalTransform),
                With<FinishRecipeMarker>,
            >,
             card_query: Query<&Card>,
             mut creation: EventWriter<CreateStackEvent>| {
                for (root, stack, global_transform) in recipe_stack_query.iter() {
                    let pack_cost = card_query.get(stack[0]).unwrap().value.unwrap();

                    let coins_to_delete: Vec<Entity> = stack
                        .iter()
                        .filter_map(|&entity| {
                            if let Ok(card) = card_query.get(entity) {
                                if card.is_type(&COIN) {
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
                    creation.send(CreateStackEvent {
                        position: global_transform.translation.truncate(),
                        card_type: &FOREST_PACK,
                        amount: 1,
                    });
                }
            },
        )
        .with_recipe(
            "Creating Villager",
            5.0,
            |cards| {
                // 1 of a worker category, 2 clay and 2 coins
                cards.len() == 5
                    && cards.contains_exactly_one_of_category(CardCategory::Worker)
                    && cards.contains_n_of_type(CLAY, 2)
                    && cards.contains_n_of_type(COIN, 2)
            },
            |mut commands: Commands,
             recipe_stack_query: Query<
                (Entity, &CardStack, &GlobalTransform),
                With<FinishRecipeMarker>,
            >,
             card_query: Query<&Card>,
             mut creation: EventWriter<CreateStackEvent>| {
                for (root, stack, global_transform) in recipe_stack_query.iter() {
                    let cards_to_be_deleted: Vec<Entity> = stack
                        .iter()
                        .filter(|&&e| {
                            card_query
                                .get(e)
                                .ok()
                                .map(|c| c.is_type(&CLAY) || c.is_type(&COIN))
                                .unwrap_or(false)
                        })
                        .copied()
                        .collect();

                    delete_cards(&mut commands, &cards_to_be_deleted, root, stack);

                    creation.send(CreateStackEvent {
                        position: global_transform.translation.truncate(),
                        card_type: &VILLAGER,
                        amount: 1,
                    });
                }
            },
        )
        .build()
}
