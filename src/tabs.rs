//! Tabs, and the panes behind them.
//!
//! The kit already had the furniture for this — a window, buttons, a book with a rail of
//! chapters — and was missing the only part that isn't furniture: **which one is open**.
//! Every game that wanted tabs therefore wrote the same three things, and wrote them
//! slightly differently: a selected index, the plumbing that shows one pane and hides the
//! rest, and a paint pass for the tab that is currently on. So the kit owns them now.
//!
//! A tab is an ordinary Ordo [`button`], so it hovers and presses and answers Enter and
//! Space like everything else. What it gains is a [`Tab`] index, and what its strip gains is
//! a [`Tabs`] with the selection in it. Panes are marked with [`Pane`] and Ordo shows
//! exactly one.
//!
//! The selected tab is painted through [`Selected`], which is deliberately *not* specific to
//! tabs: it means "this button is currently on", which is equally what a toggle or a radio
//! row needs.
//!
//! ```ignore
//! let strip = commands.spawn(tab_strip()).id();
//! for (i, name) in ["Video", "Audio", "Gameplay"].iter().enumerate() {
//!     commands.spawn((tab(name, i), ChildOf(strip)));
//! }
//! // Panes go wherever the game likes; they only have to name their index and know
//! // which strip they belong to.
//! commands.spawn((Pane { strip, index: 0 }, Node::default(), children![body("...")]));
//! ```

use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use crate::theme::{Metric, Role, Theme};
use crate::widgets::button;

/// A strip of tabs, and which of them is open.
#[derive(Component, Debug, Clone, Copy)]
pub struct Tabs {
    pub selected: usize,
}

impl Default for Tabs {
    fn default() -> Self {
        Self { selected: 0 }
    }
}

/// Which tab this is, within its strip.
///
/// Stated rather than counted from sibling order, for the same reason a wedge states its
/// own index: a game that adds, hides or reorders tabs must not have the meaning of its
/// panes shift underneath it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tab(pub usize);

/// Content belonging to one tab. Ordo shows the selected one and hides the others.
#[derive(Component, Debug, Clone, Copy)]
pub struct Pane {
    /// The strip this pane answers to, so one screen can hold more than one set of tabs.
    pub strip: Entity,
    pub index: usize,
}

/// This button is currently *on*.
///
/// Read by the button painter, which is why it lives here rather than being a private detail
/// of tabs: a toggle or a radio row wants exactly the same thing, and neither should have to
/// reach for a tab to get it.
#[derive(Component, Debug, Clone, Copy)]
pub struct Selected;

/// A strip to hang tabs off. A row, spaced by the theme's gap.
pub fn tab_strip() -> impl Bundle {
    (
        Tabs::default(),
        StripSpacing,
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::End,
            ..default()
        },
    )
}

/// Marks a strip so its spacing tracks the theme.
#[derive(Component)]
pub struct StripSpacing;

/// One tab. An ordinary button that knows its own index.
pub fn tab(label: &str, index: usize) -> impl Bundle {
    (Tab(index), button(label))
}

pub(crate) fn space_strips(
    theme: Res<Theme>,
    mut strips: Query<&mut Node, With<StripSpacing>>,
    fresh: Query<(), Added<StripSpacing>>,
) {
    if !theme.is_changed() && fresh.is_empty() {
        return;
    }
    let gap = theme.px(Metric::Gap);
    for mut node in &mut strips {
        node.column_gap = gap;
    }
}

/// Clicking a tab opens it.
///
/// An observer rather than a polled query, for the reason the whole kit uses `Activate`:
/// Ordo's buttons carry no `Interaction`, and this way a tab answers the keyboard too.
pub fn open_tab(
    activate: On<Activate>,
    tabs: Query<(&Tab, &ChildOf)>,
    mut strips: Query<&mut Tabs>,
) {
    let Ok((Tab(index), parent)) = tabs.get(activate.entity) else {
        return;
    };
    if let Ok(mut strip) = strips.get_mut(parent.parent())
        && strip.selected != *index
    {
        strip.selected = *index;
    }
}

/// Marks the open tab and shows its pane.
///
/// `Selected` is a component rather than a colour written here, because the repaint pass owns
/// colours — see [`crate::theme::repaint`]. Adding and removing it is what makes the button
/// painter draw a tab as on, and it means a faded window fades its open tab correctly too.
pub(crate) fn show_selected_pane(
    mut commands: Commands,
    strips: Query<(Entity, &Tabs)>,
    tabs: Query<(Entity, &Tab, &ChildOf, Has<Selected>)>,
    mut panes: Query<(&Pane, &mut Node)>,
    changed: Query<(), Or<(Changed<Tabs>, Added<Tab>, Added<Pane>)>>,
) {
    if changed.is_empty() {
        return;
    }

    for (entity, Tab(index), parent, marked) in &tabs {
        let Ok((_, strip)) = strips.get(parent.parent()) else {
            continue;
        };
        let open = strip.selected == *index;
        if open && !marked {
            commands.entity(entity).insert(Selected);
        } else if !open && marked {
            commands.entity(entity).remove::<Selected>();
        }
    }

    // `Display::None` rather than `Visibility::Hidden`, and this is the whole difference
    // between tabs that work and tabs that look broken. Hiding a pane stops it being drawn
    // and leaves it holding its space, so a window with three panes reserves room for all
    // three and the open one sits in a column of gaps where the closed ones are standing.
    for (pane, mut node) in &mut panes {
        let Ok((_, strip)) = strips.get(pane.strip) else {
            continue;
        };
        let want = if strip.selected == pane.index {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != want {
            node.display = want;
        }
    }
}

/// The roles a tab's chrome takes when it is open.
///
/// Kept beside the tab rather than inside the button painter so the two states are legible
/// together: an open tab wears the pressed face and an accent edge, which is the same
/// language a held-down button already speaks.
pub(crate) const SELECTED_FACE: (Role, Role) = (Role::ButtonPressed, Role::Accent);

/// Dresses a tab's pane. A plain column, so a game only has to fill it.
pub fn pane(strip: Entity, index: usize) -> impl Bundle {
    (
        Pane { strip, index },
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
    )
}

/// Is this button an open tab? Used by the button painter.
pub(crate) fn is_selected(marked: bool) -> Option<(Role, Role)> {
    marked.then_some(SELECTED_FACE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_strip_opens_on_its_first_tab() {
        assert_eq!(Tabs::default().selected, 0);
    }

    /// An open tab wears the pressed face. Stated as a test because the button painter reads
    /// it, and a change there that lost the accent edge would be invisible in a unit test of
    /// either half alone.
    #[test]
    fn an_open_tab_wears_the_pressed_face_and_an_accent_edge() {
        assert_eq!(is_selected(true), Some((Role::ButtonPressed, Role::Accent)));
        assert_eq!(is_selected(false), None);
    }
}
