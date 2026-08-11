//! A stepper: `<  Normal  >`.
//!
//! The control a settings window is mostly made of. A value with a nudge either side of it,
//! for a choice that has a handful of useful positions rather than a continuous range — and
//! for a great many settings that is the truth, whatever a slider would imply.
//!
//! Two reasons it's here rather than in a game. It is the same three nodes every time, and
//! getting it to *read* right is fiddly in a way that has nothing to do with any particular
//! game: the value has to hold a fixed width or the arrows twitch inward every time a
//! shorter word comes up, and it has to be centred in that width or the whole control
//! wobbles as the text changes.
//!
//! The arrows are `<` and `>`, plain ASCII, and that is deliberate. Bevy's embedded font
//! carries U+0020..U+007E and nothing else, so `‹` and `›` — or any of the nicer arrows —
//! draw as missing-glyph boxes in any game that hasn't shipped a font of its own. A kit
//! cannot make that assumption on a game's behalf.

use bevy::prelude::*;

use crate::theme::Metric;
use crate::widgets::{body, button};

/// The pieces of a stepper a caller wires into: the two nudges to observe, and the value to
/// rewrite.
pub struct StepperParts {
    pub root: Entity,
    pub down: Entity,
    pub value: Entity,
    pub up: Entity,
}

/// Marks a stepper's value box, so its width tracks the theme.
#[derive(Component)]
pub struct StepperValue;

/// `<  value  >`, laid out so the arrows don't move when the value's length changes.
pub fn stepper(commands: &mut Commands, parent: Entity, value: &str) -> StepperParts {
    let root = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(parent),
        ))
        .id();

    let down = commands.spawn((button("<"), ChildOf(root))).id();
    let value_entity = commands
        .spawn((
            StepperValue,
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(root),
            children![body(value)],
        ))
        .id();
    let up = commands.spawn((button(">"), ChildOf(root))).id();

    StepperParts { root, down, value: value_entity, up }
}

/// Sizes a stepper's value box from the theme.
///
/// [`Metric::LabelWidth`] rather than a metric of its own: a stepper's value and a row's
/// label are the two columns of the same table, and a window where they disagree reads as
/// two windows.
pub(crate) fn size_steppers(
    theme: Res<crate::theme::Theme>,
    mut values: Query<&mut Node, With<StepperValue>>,
    fresh: Query<(), Added<StepperValue>>,
) {
    if !theme.is_changed() && fresh.is_empty() {
        return;
    }
    let width = theme.px(Metric::LabelWidth);
    for mut node in &mut values {
        node.width = width;
    }
}
