use crate::card_types::{CardCategory, CardType, TREE};
use crate::stack::{Card, CardStack, HoveredCard};
use crate::stack_utils::{delete_cards, StackCreation};
use crate::UpdateStage;
use bevy::prelude::*;

pub(crate) const BUY_FOREST_PACK: CardType = CardType {
    title: "Forest",
    value: Some(3),
    category: CardCategory::SystemCard,
    description: "Buy a Forest Pack",
    on_spawn: None,
};

pub(crate) const FOREST_PACK: CardType = CardType {
    title: "Forest",
    value: None,
    category: CardCategory::CardPack,
    description: "Right click to open",
    on_spawn: Some(|commands: &mut Commands, card: Entity| {
        // TODO (Wybe 2022-05-29): Implement random pack contents.
        commands
            .entity(card)
            .insert(CardPack(vec![TREE, TREE, TREE]));
    }),
};

pub struct CardPackPlugin;

impl Plugin for CardPackPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set_to_stage(
            UpdateStage::SystemsThatDeleteCards.as_str(),
            SystemSet::new().with_system(card_pack_open_system),
        );
    }
}

/// Keeps the cards contained in a card pack.
#[derive(Component, Deref, DerefMut)]
pub struct CardPack(Vec<CardType>);

/// This system has to go in a system stage that isn't [CoreStage::Update].
/// This is because it is allowed to remove cards / stacks, which can break other systems
/// which add components to them.
/// TODO (Wybe 2022-05-29): Maybe add a new system stage which has all the systems in it that allow adding or removing cards/stacks.
pub fn card_pack_open_system(
    mut commands: Commands,
    mut card_pack_query: Query<(&mut CardPack, &GlobalTransform, &Parent), With<Card>>,
    hovered_card_query: Query<Entity, With<HoveredCard>>,
    stacks_query: Query<&CardStack>,
    creation: Res<StackCreation>,
    mouse_input: Res<Input<MouseButton>>,
) {
    if mouse_input.just_pressed(MouseButton::Right) {
        for hovered in hovered_card_query.iter() {
            if let Ok((mut pack, global_transform, root)) = card_pack_query.get_mut(hovered) {
                // Spawn one card from the card pack.
                if let Some(new_card) = pack.pop() {
                    creation.spawn_stack(
                        &mut commands,
                        global_transform.translation.truncate(),
                        new_card,
                        1,
                        true,
                    );
                }

                // Delete card pack when empty.
                if pack.is_empty() {
                    if let Ok(stack) = stacks_query.get(root.0) {
                        delete_cards(&mut commands, &[hovered], root.0, &stack.0);
                    }
                }
            }
        }
    }
}
