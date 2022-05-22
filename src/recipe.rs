use crate::card::Card;
use crate::card_types;
use crate::card_types::CardType::Worker;
use bevy::prelude::*;
use std::collections::HashMap;

/// Handles recipes on card stacks
/// Requires [CardPlugin].
pub struct RecipePlugin;

impl Plugin for RecipePlugin {
    fn build(&self, app: &mut App) {
        let recipes = Recipes::default().with(Recipe {
            name: "Cutting tree".to_owned(),
            valid_callback: |cards| {
                // Exactly 1 of type Worker, and the rest trees.
                cards.iter().any(|c| c.card_type == Worker)
                    && cards.iter().filter(|c| c == &&card_types::TREE).count() == cards.len() - 1
            },
        });

        app.insert_resource(recipes);
    }
}

/// Component indicating the id of an ongoing recipe.
#[derive(Component, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub struct RecipeId(usize);

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
    valid_callback: fn(Vec<Card>) -> bool,
}
