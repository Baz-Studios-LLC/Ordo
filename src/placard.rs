//! A placard: a card pinned over a thing in the WORLD.
//!
//! Divus Factus grew this shape twice in one week — nameplates over
//! villagers' heads, an occupancy call over a knocked house — and its
//! speech bubbles were already a third, hand-rolled before the kit
//! existed. The shape is always the same: project an entity's seat onto
//! the screen, hold a card centered over that point growing upward, put it
//! away when the thing walks off screen or out of reading range, and
//! sometimes let it die of old age ([`crate::overlay::Lifetime`] already
//! does the dying).
//!
//! The kit owns the PINNING and the card's chrome; the game owns the
//! content. A game that wants its own border color — Divus Factus dyes a
//! nameplate's border by the wearer's faith — removes the [`Edge`] tag
//! from the card and writes [`BorderColor`] itself; the repaint pass only
//! ever touches what is tagged.

use bevy::prelude::*;

use crate::overlay::Layer;
use crate::theme::{Edge, Fill, Role};

/// Pins a UI node over a world entity.
///
/// Lives on an absolute, zero-sized node — the projected point — whose
/// children are centered on it and grow upward. [`placard`] builds exactly
/// that shape; this component only does the following-around.
#[derive(Component, Debug, Clone, Copy)]
pub struct Placard {
    /// Whose head (or roof) this floats over.
    pub over: Entity,
    /// World units above the anchor's origin.
    pub lift: f32,
    /// Beyond this many world units from the camera, the placard hides:
    /// a card that has shrunk past reading is clutter, not information.
    /// `None` never hides.
    pub reach: Option<f32>,
    /// Inside this many world units the card is full-sized; past it, it
    /// shrinks with distance down to [`SCALE_FLOOR`]. Depth on a flat
    /// screen, and the answer to a crowd: near cards read, far cards
    /// register as presence without shouting.
    pub full_within: f32,
}

/// How small a placard may shrink before it stops being information.
const SCALE_FLOOR: f32 = 0.5;

/// The screen scale of something `distance` away that reads full-size
/// within `full_within`. The one authority on depth scaling — anything
/// world-anchored that is not yet a placard (Divus Factus's speech
/// bubbles, mid-migration) borrows it from here rather than growing its
/// own curve.
pub fn depth_scale(distance: f32, full_within: f32) -> f32 {
    (full_within / distance.max(1.0)).clamp(SCALE_FLOOR, 1.0)
}

/// The pieces a placard is made of.
pub struct PlacardParts {
    /// The pinned point. Despawn THIS to take the placard down.
    pub root: Entity,
    /// The card. Content goes in here.
    pub card: Entity,
}

/// A themed card floating over `over`, `lift` world units up.
///
/// The card comes painted as a panel with a card border and sits under
/// every piece of interface chrome — a window dragged across it must
/// cover it, because the placard belongs to the world, not the interface.
pub fn placard(
    commands: &mut Commands,
    over: Entity,
    lift: f32,
    reach: Option<f32>,
    full_within: f32,
) -> PlacardParts {
    let root = pin(commands, over, lift, reach, full_within);
    let card = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                // The parent is a point with no width of its own; without
                // this the card would be squeezed to nothing.
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            Fill(Role::PanelBg),
            BorderColor::all(Color::NONE),
            Edge(Role::CardBorder),
            ChildOf(root),
        ))
        .id();
    PlacardParts { root, card }
}

/// A bare pinned point with no card: for floating marks and one-glyph
/// callouts that would drown in a panel. Content goes in as children,
/// centered on the point and growing upward, exactly as a placard's card
/// does — a placard IS a pin wearing one.
pub fn pin(
    commands: &mut Commands,
    over: Entity,
    lift: f32,
    reach: Option<f32>,
    full_within: f32,
) -> Entity {
    commands
        .spawn((
            Placard {
                over,
                lift,
                reach,
                full_within,
            },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(0.0),
                height: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            UiTransform::default(),
            Visibility::Hidden,
            Layer::World,
        ))
        .id()
}

/// Climbs while it lives. Put beside a [`Placard`] and the pin rises off
/// its anchor — the shape of every floating mark a game ever wants: a
/// "+" of belief gained, a damage number, a coin earned. World units per
/// second.
#[derive(Component, Debug, Clone, Copy)]
pub struct Rising(pub f32);

pub(crate) fn raise_placards(time: Res<Time>, mut rising: Query<(&mut Placard, &Rising)>) {
    for (mut placard, rate) in &mut rising {
        placard.lift += rate.0 * time.delta_secs();
    }
}

/// Follows every placard's anchor: projected through the world camera,
/// hidden off screen, out of reach, or the moment the anchor is gone.
///
/// The world camera is the active one drawn first (`order == 0`), which
/// is what it means in every game this kit serves. A placard does not
/// despawn itself when its anchor dies — whether the card outlives the
/// thing is the game's decision, and the game knows how to despawn.
pub(crate) fn place_placards(
    cameras: Query<(&Camera, &GlobalTransform)>,
    anchors: Query<&GlobalTransform>,
    mut placards: Query<(&Placard, &mut Node, &mut Visibility, &mut UiTransform)>,
) {
    let Some((camera, camera_at)) = cameras
        .iter()
        .find(|(camera, _)| camera.is_active && camera.order == 0)
    else {
        return;
    };
    for (placard, mut node, mut visibility, mut ui) in &mut placards {
        let Ok(anchor) = anchors.get(placard.over) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let seat = anchor.translation() + Vec3::Y * placard.lift;
        let distance = seat.distance(camera_at.translation());
        let near = placard.reach.is_none_or(|reach| distance < reach);
        match camera.world_to_viewport(camera_at, seat) {
            Ok(spot) if near => {
                node.left = Val::Px(spot.x);
                node.top = Val::Px(spot.y);
                // Depth on a flat screen: the whole card shrinks toward
                // its pinned point as its anchor recedes.
                let scale = depth_scale(distance, placard.full_within);
                if ui.scale != Vec2::splat(scale) {
                    ui.scale = Vec2::splat(scale);
                }
                *visibility = Visibility::Inherited;
            }
            _ => *visibility = Visibility::Hidden,
        }
    }
}
