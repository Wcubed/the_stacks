use bevy::prelude::Color;

pub enum CardType {
    Nature,
    Resource,
}

impl CardType {
    pub fn background_color(&self) -> Color {
        match self {
            CardType::Nature => Color::DARK_GREEN,
            CardType::Resource => Color::BLUE,
        }
    }
}
