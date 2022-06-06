use crate::card_types::{CardCategory, CardType, CLAY, TREE};
use crate::procedural::SeededHasherResource;
use crate::stack::stack_utils::delete_cards;
use crate::stack::{Card, CardStack, CreateStackEvent, HoveredCard, IsExclusiveBottomCard};
use crate::UpdateStage;
use bevy::prelude::*;

const FOREST_PACK_CONTENT_OPTIONS: &[CardType] = &[TREE, CLAY];

pub(crate) const BUY_FOREST_PACK: CardType = CardType {
    title: "Forest",
    value: Some(3),
    category: CardCategory::SystemCard,
    description: "Buy a Forest Pack",
    on_spawn: Some(|commands: &mut Commands, card: Entity| {
        commands.entity(card).insert(IsExclusiveBottomCard);
    }),
};

pub(crate) const FOREST_PACK: CardType = CardType {
    title: "Forest",
    value: None,
    category: CardCategory::CardPack,
    description: "Right click to open",
    on_spawn: Some(|commands: &mut Commands, card: Entity| {
        commands.entity(card).insert(CardPack { cards: 3 });
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

/// Marks card packs.
#[derive(Component)]
pub struct CardPack {
    cards: usize,
}

/// This system has to go in a system stage that isn't [CoreStage::Update].
/// This is because it is allowed to remove cards / stacks, which can break other systems
/// which add components to them.
/// TODO (Wybe 2022-05-29): Maybe add a new system stage which has all the systems in it that allow adding or removing cards/stacks.
pub fn card_pack_open_system(
    mut commands: Commands,
    mut card_pack_query: Query<(&Card, &mut CardPack, &GlobalTransform, &Parent)>,
    hovered_card_query: Query<Entity, With<HoveredCard>>,
    stacks_query: Query<&CardStack>,
    mouse_input: Res<Input<MouseButton>>,
    seeded_hasing: Res<SeededHasherResource>,
    mut creation: EventWriter<CreateStackEvent>,
) {
    if mouse_input.just_pressed(MouseButton::Right) {
        for hovered in hovered_card_query.iter() {
            if let Ok((card, mut pack, global_transform, root)) = card_pack_query.get_mut(hovered) {
                if pack.cards > 0 {
                    let mut rng = seeded_hasing.with(hovered);
                    rng.with(pack.cards);

                    let new_card = if card.is_type(&FOREST_PACK) {
                        // TODO (Wybe 2022-06-05): randomize.
                        let card = &FOREST_PACK_CONTENT_OPTIONS
                            [rng.value_in_range(0..FOREST_PACK_CONTENT_OPTIONS.len())];
                        Some(card)
                    } else {
                        None
                    };

                    // Spawn one card from the card pack.
                    if let Some(new_card) = new_card {
                        creation.send(CreateStackEvent {
                            position: global_transform.translation.truncate(),
                            card_type: new_card,
                            amount: 1,
                        });
                        pack.cards -= 1;
                    }
                }

                // Delete card pack when empty.
                if pack.cards == 0 {
                    if let Ok(stack) = stacks_query.get(root.0) {
                        delete_cards(&mut commands, &[hovered], root.0, &stack.0);
                    }
                }
            }
        }
    }
}
