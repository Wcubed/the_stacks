use crate::recipe::RecipeUses;
use crate::stack::{Card, IsExclusiveBottomCard};
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
    pub id: &'static str,
    pub category: CardCategory,
    /// Base cost of this card when sold.
    /// `None` means the card cannot be sold.
    pub value: Option<usize>,
    /// Function that is ran on spawn of a card.
    /// Use this to add additional components.
    pub on_spawn: Option<fn(&mut Commands, Entity)>,
}

impl CardType {
    pub fn get_card_component(&self) -> Card {
        Card {
            type_id: self.id,
            category: self.category,
            value: self.value,
        }
    }
}

impl PartialEq<Self> for CardType {
    /// Only checks the card id, because non-equal cards should be identifiable by their id.
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for CardType {}

impl Hash for CardType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.id.as_ref())
    }
}

pub(crate) const MARKET: CardType = CardType {
    id: "market",
    value: None,
    category: CardCategory::SystemCard,
    on_spawn: Some(|commands: &mut Commands, card: Entity| {
        commands.entity(card).insert(IsExclusiveBottomCard);
    }),
};

pub(crate) const TREE: CardType = CardType {
    id: "tree",
    value: Some(0),
    category: CardCategory::Nature,
    on_spawn: Some(|commands: &mut Commands, card: Entity| {
        commands.entity(card).insert(RecipeUses(3));
    }),
};

pub(crate) const CLAY_PATCH: CardType = CardType {
    id: "clay_patch",
    value: Some(0),
    category: CardCategory::Nature,
    on_spawn: Some(|commands: &mut Commands, card: Entity| {
        commands.entity(card).insert(RecipeUses(3));
    }),
};

pub(crate) const CLAY: CardType = CardType {
    id: "clay",
    value: Some(1),
    category: CardCategory::Resource,
    on_spawn: None,
};

pub(crate) const LOG: CardType = CardType {
    id: "log",
    value: Some(1),
    category: CardCategory::Resource,
    on_spawn: None,
};

pub(crate) const PLANK: CardType = CardType {
    id: "plank",
    value: Some(2),
    category: CardCategory::Resource,
    on_spawn: None,
};

pub(crate) const VILLAGER: CardType = CardType {
    id: "villager",
    value: None,
    category: CardCategory::Worker,
    on_spawn: None,
};

pub(crate) const COIN: CardType = CardType {
    id: "coin",
    value: None,
    category: CardCategory::Valuable,
    on_spawn: None,
};

pub(crate) const APPLE: CardType = CardType {
    id: "apple",
    value: Some(1),
    category: CardCategory::Food,
    on_spawn: None,
};
