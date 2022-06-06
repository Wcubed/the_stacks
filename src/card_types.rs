use crate::procedural::SeededHasherResource;
use crate::recipe::RecipeUses;
use crate::stack::{Card, CardDescription, IsExclusiveBottomCard};
use bevy::prelude::*;
use std::hash::{Hash, Hasher};

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum CardCategory {
    /// Cards which are integral to the game system, such as the market to sell things.
    SystemCard,
    CardPack,
    Worker,
    Nature,
    Resource,
    Valuable,
    Food,
    Gem,
}

impl CardCategory {
    pub fn background_color(&self) -> Color {
        match self {
            CardCategory::SystemCard => Color::PURPLE,
            CardCategory::CardPack => Color::BLACK,
            CardCategory::Worker => Color::hsl(25., 0.8, 0.2),
            CardCategory::Nature => Color::DARK_GREEN,
            CardCategory::Resource => Color::BLUE,
            CardCategory::Food => Color::ORANGE,
            CardCategory::Valuable => Color::YELLOW,
            CardCategory::Gem => Color::MAROON,
        }
    }

    /// Gets the color which has the most contrast with the background color.
    pub fn text_color(&self) -> Color {
        let back = self.background_color();
        // Factors based on how strong the human eye perceives each color.
        if back.r() * 0.299 + back.g() * 0.587 + back.b() * 0.114 > 0.729 {
            Color::BLACK
        } else {
            Color::WHITE
        }
    }
}

pub struct CardType {
    // TODO (Wybe 2022-06-05): Make this title an index into some kind of translation system instead.
    pub title: &'static str,
    pub category: CardCategory,
    /// Base cost of this card when sold.
    /// `None` means the card cannot be sold.
    pub value: Option<usize>,
    pub description: &'static str,
    /// Function that is ran on spawn of a card.
    /// Use this to add additional components.
    pub on_spawn: Option<fn(&mut Commands, Entity)>,
}

impl CardType {
    pub fn get_card_components(&self) -> (Card, CardDescription) {
        (
            Card {
                title: self.title,
                category: self.category,
                value: self.value,
            },
            CardDescription(self.description),
        )
    }
}

impl PartialEq<Self> for CardType {
    /// Only checks the card title, because non-equal cards should be identifiable by their title.
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
    }
}

impl Eq for CardType {}

impl Hash for CardType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.title.as_ref())
    }
}

pub(crate) const MARKET: CardType = CardType {
    title: "Market",
    value: None,
    category: CardCategory::SystemCard,
    description: "Sell cards here for coins.",
    on_spawn: Some(|commands: &mut Commands, card: Entity| {
        commands.entity(card).insert(IsExclusiveBottomCard);
    }),
};

pub(crate) const TREE: CardType = CardType {
    title: "Tree",
    value: Some(0),
    category: CardCategory::Nature,
    description: "A source of logs.",
    on_spawn: Some(|commands: &mut Commands, card: Entity| {
        commands.entity(card).insert(RecipeUses(3));
    }),
};

pub(crate) const CLAY_PATCH: CardType = CardType {
    title: "Clay Patch",
    value: Some(0),
    category: CardCategory::Nature,
    description: "A slippery piece of ground.",
    on_spawn: Some(|commands: &mut Commands, card: Entity| {
        commands.entity(card).insert(RecipeUses(3));
    }),
};

pub(crate) const CLAY: CardType = CardType {
    title: "Clay",
    value: Some(1),
    category: CardCategory::Resource,
    description: "Very moldable.\nSome say people are made of this...",
    on_spawn: None,
};

pub(crate) const LOG: CardType = CardType {
    title: "Log",
    value: Some(1),
    category: CardCategory::Resource,
    description: "A long piece of wood, with the bark still on.",
    on_spawn: None,
};

pub(crate) const PLANK: CardType = CardType {
    title: "Plank",
    value: Some(2),
    category: CardCategory::Resource,
    description: "Might have splinters.",
    on_spawn: None,
};

pub(crate) const VILLAGER: CardType = CardType {
    title: "Villager",
    value: None,
    category: CardCategory::Worker,
    description: "A strong worker.",
    on_spawn: None,
};

pub(crate) const COIN: CardType = CardType {
    title: "Coin",
    value: None,
    category: CardCategory::Valuable,
    description: "Lifeblood of the village. Literally...",
    on_spawn: None,
};

pub(crate) const APPLE: CardType = CardType {
    title: "Apple",
    value: Some(1),
    category: CardCategory::Food,
    description: "Rumored to scare doctors",
    on_spawn: None,
};
