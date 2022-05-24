use crate::card::Card;
use bevy::prelude::Color;

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum CardType {
    Worker,
    Nature,
    Resource,
    Food,
}

impl CardType {
    pub fn background_color(&self) -> Color {
        match self {
            CardType::Worker => Color::hsl(25., 0.8, 0.2),
            CardType::Nature => Color::DARK_GREEN,
            CardType::Resource => Color::BLUE,
            CardType::Food => Color::OLIVE,
        }
    }
}

pub(crate) const TREE: Card = Card {
    title: "Tree",
    card_type: CardType::Nature,
};

pub(crate) const LOG: Card = Card {
    title: "Log",
    card_type: CardType::Resource,
};

pub(crate) const APPLE: Card = Card {
    title: "Apple",
    card_type: CardType::Food,
};

pub(crate) const PLANK: Card = Card {
    title: "Plank",
    card_type: CardType::Resource,
};

pub(crate) const WORKER: Card = Card {
    title: "Worker",
    card_type: CardType::Worker,
};
