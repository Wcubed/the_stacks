use crate::card::Card;
use crate::recipe::RecipeUses;
use bevy::prelude::*;

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum CardCategory {
    /// Cards which are integral to the game system, such as the market to sell things.
    SystemCard,
    Worker,
    Nature,
    Resource,
    Valuable,
    Food,
}

impl CardCategory {
    pub fn background_color(&self) -> Color {
        match self {
            CardCategory::SystemCard => Color::PURPLE,
            CardCategory::Worker => Color::hsl(25., 0.8, 0.2),
            CardCategory::Nature => Color::DARK_GREEN,
            CardCategory::Resource => Color::BLUE,
            CardCategory::Food => Color::OLIVE,
            CardCategory::Valuable => Color::YELLOW,
        }
    }
}

pub struct CardType {
    pub title: &'static str,
    pub category: CardCategory,
    /// Function that is ran on spawn of a card.
    /// Use this to add additional components.
    pub on_spawn: Option<fn(&mut Commands, Entity)>,
}

impl CardType {
    pub fn get_card_component(&self) -> Card {
        Card {
            title: self.title,
            category: self.category,
        }
    }
}

pub(crate) const MARKET: CardType = CardType {
    title: "Market",
    category: CardCategory::SystemCard,
    on_spawn: None,
};

pub(crate) const TREE: CardType = CardType {
    title: "Tree",
    category: CardCategory::Nature,
    on_spawn: Some(|commands: &mut Commands, card: Entity| {
        commands.entity(card).insert(RecipeUses(3));
    }),
};

pub(crate) const LOG: CardType = CardType {
    title: "Log",
    category: CardCategory::Resource,
    on_spawn: None,
};

pub(crate) const PLANK: CardType = CardType {
    title: "Plank",
    category: CardCategory::Resource,
    on_spawn: None,
};

pub(crate) const WORKER: CardType = CardType {
    title: "Worker",
    category: CardCategory::Worker,
    on_spawn: None,
};

pub(crate) const COIN: CardType = CardType {
    title: "Coin",
    category: CardCategory::Valuable,
    on_spawn: None,
};

pub(crate) const APPLE: CardType = CardType {
    title: "Apple",
    category: CardCategory::Food,
    on_spawn: None,
};
