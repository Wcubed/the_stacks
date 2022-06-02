#![cfg(test)]

use crate::card_types::CardCategory;
use crate::stack::{
    card_hover_system, Card, CardVisualSize, HoveredCard, IsCardHoverOverlay, MouseWorldPos,
};
use bevy::prelude::*;

#[test]
fn test_hover_system() {
    let mut world = World::default();

    let mut update_stage = SystemStage::parallel().with_system(card_hover_system);

    let card_size = Vec2::new(100.0, 200.0);
    world.insert_resource(MouseWorldPos(None));
    world.insert_resource(CardVisualSize(card_size));

    // Running it without any cards shouldn't fail.
    update_stage.run(&mut world);

    let hover_overlay = world
        .spawn()
        .insert(IsCardHoverOverlay)
        .insert(Visibility { is_visible: false })
        .id();
    // Spawn the card at (0, 0)
    let card = world
        .spawn()
        .insert_bundle(TransformBundle::default())
        .insert(Card {
            title: "Test card",
            category: CardCategory::Resource,
            description: "",
            value: None,
        })
        .push_children(&[hover_overlay])
        .id();

    // Put the mouse right at the middle of the card.
    let mouse_pos = Vec2::ZERO;
    world.insert_resource(MouseWorldPos(Some(mouse_pos)));

    update_stage.run(&mut world);

    // The card at (0, 0) should now be hovered.
    let hovered_card_component = world.get::<HoveredCard>(card);
    assert_eq!(
        hovered_card_component,
        Some(&HoveredCard {
            relative_hover_pos: mouse_pos
        })
    );
    let visible = world.get::<Visibility>(hover_overlay).unwrap().is_visible;
    assert!(visible);

    // Move the mouse to somewhere else in the card.
    let mouse_pos = Vec2::new(10.0, 40.0);
    world.insert_resource(MouseWorldPos(Some(mouse_pos)));

    update_stage.run(&mut world);

    // The card at (0, 0) should still be hovered. But now at the new location.
    let hovered_card_component = world.get::<HoveredCard>(card);
    assert_eq!(
        hovered_card_component,
        Some(&HoveredCard {
            relative_hover_pos: mouse_pos
        })
    );
    let visible = world.get::<Visibility>(hover_overlay).unwrap().is_visible;
    assert!(visible);

    // Move the mouse to somewhere outside the card.
    let mouse_pos = card_size + Vec2::new(10., 0.);
    world.insert_resource(MouseWorldPos(Some(mouse_pos)));

    update_stage.run(&mut world);

    // The card at (0, 0) should no longer be hovered.
    let hovered_card_component = world.get::<HoveredCard>(card);
    assert_eq!(hovered_card_component, None,);
    let visible = world.get::<Visibility>(hover_overlay).unwrap().is_visible;
    assert!(!visible);
}
