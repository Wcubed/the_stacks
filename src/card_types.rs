use crate::card::Card;
use bevy::prelude::{Color, Component};

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum CardCategory {
    Worker,
    Nature,
    Resource,
    Food,
}

impl CardCategory {
    pub fn background_color(&self) -> Color {
        match self {
            CardCategory::Worker => Color::hsl(25., 0.8, 0.2),
            CardCategory::Nature => Color::DARK_GREEN,
            CardCategory::Resource => Color::BLUE,
            CardCategory::Food => Color::OLIVE,
        }
    }
}

pub struct CardType {
    pub title: &'static str,
    pub category: CardCategory,
}

impl CardType {
    pub fn get_card_component(&self) -> Card {
        Card {
            title: self.title,
            category: self.category,
        }
    }
}
pub(crate) const TREE: CardType = CardType {
    title: "Tree",
    category: CardCategory::Nature,
};

pub(crate) const LOG: CardType = CardType {
    title: "Log",
    category: CardCategory::Resource,
};

pub(crate) const APPLE: CardType = CardType {
    title: "Apple",
    category: CardCategory::Food,
};

pub(crate) const PLANK: CardType = CardType {
    title: "Plank",
    category: CardCategory::Resource,
};

pub(crate) const WORKER: CardType = CardType {
    title: "Worker",
    category: CardCategory::Worker,
};
