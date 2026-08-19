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

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use crate::theme::{Edge, Fill, Metric, Role, Theme};
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
/// `Selected` is a component rather than a color written here, because the repaint pass owns
/// colors — see [`crate::theme::repaint`]. Adding and removing it is what makes the button
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

/// How much thicker the accent bar on an open tab is than an ordinary hairline.
const BAR_WEIGHT: f32 = 3.0;

/// Gives a tab a tab's *shape*: square-shouldered at the bottom, rounded on top, a heavy top
/// edge to carry the accent, and no bottom edge at all.
///
/// Without this a tab is a button, and reads as one however it is coloured. A tab is a card
/// with one side missing — the side it shares with the pane below it — and the shape is what
/// says so before any color does.
pub(crate) fn shape_tabs(
    theme: Res<Theme>,
    mut tabs: Query<&mut Node, With<Tab>>,
    fresh: Query<(), Added<Tab>>,
) {
    if !theme.is_changed() && fresh.is_empty() {
        return;
    }
    let hairline = theme.metric(Metric::Border);
    let radius = theme.px(Metric::Radius);
    for mut node in &mut tabs {
        node.border = UiRect {
            left: px(hairline),
            right: px(hairline),
            top: px(hairline * BAR_WEIGHT),
            bottom: px(0.0),
        };
        node.border_radius = BorderRadius {
            top_left: radius,
            top_right: radius,
            bottom_left: px(0.0),
            bottom_right: px(0.0),
        };
        // Down over whatever rule the game has drawn beneath the strip, by exactly a
        // hairline. An open tab's fill then covers that rule and joins the pane; a closed
        // one's own bottom edge is the rule at that point. This is the join that makes a
        // strip of tabs read as one piece of furniture with the content.
        node.margin = UiRect::bottom(px(-hairline));
    }
}

/// Paints a tab's chrome, per edge.
///
/// Runs *after* `paint_buttons`, and overwrites it for tabs. That pass has to write
/// `BorderColor::all` — every other button wants one color all the way round — and a tab is
/// the one widget that wants a different color on one edge, which is the whole effect: the
/// accent sits on the top of the open tab and nowhere else.
pub(crate) fn paint_tabs(
    theme: Res<Theme>,
    mut tabs: Query<
        (
            &Hovered,
            Has<Selected>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Tab>,
    >,
) {
    let accent = theme.color(Role::Accent);
    let edge = theme.color(Role::PanelBorder);
    let content = theme.color(Role::CardBg);
    let closed = theme.color(Role::ButtonIdle);
    let hovered_fill = theme.color(Role::ButtonHover);

    for (hovered, open, mut background, mut border) in &mut tabs {
        // An open tab is filled with the *content's* color, which is what joins it to the
        // pane below. A closed one keeps a button's face, so the strip reads as a row of
        // things you can press with one of them already open.
        *background = BackgroundColor(match (open, hovered.get()) {
            (true, _) => content,
            (false, true) => hovered_fill,
            (false, false) => closed,
        });
        *border = BorderColor {
            top: if open { accent } else { edge },
            left: edge,
            right: edge,
            // Nothing at the foot of an open tab: it is continuous with the pane.
            bottom: if open { content } else { edge },
        };
    }
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

// ---------------------------------------------------------------------------
// Folder tabs: the strip that joins the page it opens.
// ---------------------------------------------------------------------------

/// A tab drawn as a folder: rounded at the top, open at the bottom when it is
/// the selected one, so the strip and its page read as one object.
#[derive(Component, Debug, Clone, Copy)]
pub struct FolderTab;

/// The board a folder's tabs open into.
#[derive(Component, Debug, Clone, Copy)]
pub struct FolderPane;

/// Builds a folder: the strip, its tabs, and one pane each. Returns the panes,
/// for the caller to fill.
///
/// A `Commands` helper rather than bundles, and that is the whole point of it.
/// A folder tab needs THREE things and any two of them look like a bug:
///
/// 1. The strip must OVERLAP the page by a pixel, so the selected tab's bottom
///    border - painted in the page's own fill - erases the line beneath it.
/// 2. The strip must be DRAWN OVER the page, because the page is a later sibling
///    and will otherwise paint its top border straight back over that opening.
/// 3. The strip and the page must sit in a wrapper WITH NO GAP, because their
///    parent's `row_gap` otherwise pushes the tab away from the line it opens -
///    and the margin in (1) can only claw back one pixel of it.
///
/// The third is the one that hides. It was written by hand for one window, left
/// out of the shared widget, and reported again two hours later on another
/// screen: "Settings page tab has th same connection problem." A caller cannot
/// get it wrong from here, which is what a kit is for.
pub fn folder(commands: &mut Commands, parent: Entity, labels: &[&str]) -> Vec<Entity> {
    let wrapper = commands
        .spawn((
            Node {
                width: percent(100.0),
                flex_grow: 1.0,
                min_height: px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ChildOf(parent),
        ))
        .id();
    let strip = commands
        .spawn((
            tab_strip(),
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::End,
                // (1) and (2).
                margin: UiRect::bottom(px(1.0) * -1.0),
                ..default()
            },
            ZIndex(1),
            ChildOf(wrapper),
        ))
        .id();

    let mut panes = Vec::with_capacity(labels.len());
    for (index, label) in labels.iter().enumerate() {
        commands.spawn((
            tab(label, index),
            FolderTab,
            Node {
                padding: UiRect::axes(px(22.0), px(8.0)),
                border: UiRect {
                    top: px(if index == 0 { 3.0 } else { 1.0 }),
                    left: px(1.0),
                    right: px(1.0),
                    bottom: px(1.0),
                },
                border_radius: BorderRadius {
                    top_left: px(3.0),
                    top_right: px(3.0),
                    bottom_left: px(0.0),
                    bottom_right: px(0.0),
                },
                ..default()
            },
            ChildOf(strip),
        ));
        let pane = commands
            .spawn((
                Pane { strip, index },
                FolderPane,
                Node {
                    width: percent(100.0),
                    flex_grow: 1.0,
                    min_height: px(0.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(px(12.0)),
                    border: UiRect::all(px(1.0)),
                    // Square where the first tab meets it, rounded elsewhere.
                    border_radius: BorderRadius {
                        top_left: px(0.0),
                        top_right: px(3.0),
                        bottom_left: px(3.0),
                        bottom_right: px(3.0),
                    },
                    display: if index == 0 {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Fill(Role::CardBg),
                BorderColor::all(Color::NONE),
                Edge(Role::CardBorder),
                ChildOf(wrapper),
            ))
            .id();
        panes.push(pane);
    }
    panes
}

/// Paints a folder tab's four edges, which are four different things.
///
/// Not `Edge(Role)` like everything else in the kit, because that paints one
/// color on all four sides and a folder is defined by its sides DIFFERING: gold
/// along the top of the open one, and a bottom in the page's own fill, which is
/// what makes the line appear to open there.
pub(crate) fn paint_folder_tabs(
    theme: Res<Theme>,
    mut tabs: Query<(&mut BorderColor, &mut BackgroundColor, &mut Node, Has<Selected>), With<FolderTab>>,
) {
    for (mut border, mut fill, mut node, open) in &mut tabs {
        *border = if open {
            BorderColor {
                top: theme.color(Role::Accent),
                left: theme.color(Role::CardBorder),
                right: theme.color(Role::CardBorder),
                bottom: theme.color(Role::CardBg),
            }
        } else {
            BorderColor {
                top: theme.color(Role::PanelBorder).with_alpha(0.25),
                left: theme.color(Role::PanelBorder).with_alpha(0.25),
                right: theme.color(Role::PanelBorder).with_alpha(0.25),
                bottom: theme.color(Role::CardBorder),
            }
        };
        fill.0 = if open {
            theme.color(Role::CardBg)
        } else {
            theme.color(Role::PanelBg).with_alpha(0.55)
        };
        // The open tab's top is thicker, so the width moves with the color.
        node.border.top = px(if open { 3.0 } else { 1.0 });
    }
}
