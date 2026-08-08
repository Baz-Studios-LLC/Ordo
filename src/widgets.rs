//! The handful of pieces that turned up in every game.
//!
//! Each one returns an `impl Bundle` where it can, so it drops straight into
//! `children![..]` and composes without a `Commands` in hand. The pieces that
//! genuinely produce more than one entity — a panel with a title bar — hand
//! back the entities the caller will want instead.
//!
//! Nothing here reads the theme. Widgets carry role and metric *tags*; the
//! passes in [`crate::theme`] fill in the colour and the spacing, which is why
//! a running game answers an edit to the theme file.

use crate::theme::{Edge, Face, Fill, FontRole, Ink, Metric, Role, TextSize, Theme};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Pressed;
// The headless one. `bevy::prelude::Button` is the older marker; this is the
// widget that also fires `Activate` on Enter or Space while focused.
use bevy::ui_widgets::Button as UiButton;

// ---------------------------------------------------------------------------
// Layout tags — the metric half of the theme
// ---------------------------------------------------------------------------

/// Padding, gaps, border width and corner radius follow the theme's metrics.
#[derive(Component, Debug, Clone, Copy)]
pub struct Padded;

/// Sits in a corner of the screen, a [`Metric::Margin`] off both edges.
#[derive(Component, Debug, Clone, Copy)]
pub struct Anchored(pub Anchor);

/// Holds the shared label column width, so stacked rows read as a table
/// instead of as a list of sentences.
#[derive(Component, Debug, Clone, Copy)]
pub struct LabelColumn;

/// Where a panel sits. Corners rather than free placement: a HUD is furniture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Anchor {
    fn offsets(self, margin: Val) -> (Val, Val, Val, Val) {
        let auto = Val::Auto;
        match self {
            Anchor::TopLeft => (margin, auto, auto, margin),
            Anchor::TopRight => (margin, margin, auto, auto),
            Anchor::BottomLeft => (auto, auto, margin, margin),
            Anchor::BottomRight => (auto, margin, margin, auto),
        }
    }
}

/// Re-applies every metric-driven layout value when the theme moves.
///
/// One query rather than three: a panel is `Anchored` *and* `Padded`, and two
/// systems both reaching for its `Node` is a conflict Bevy will refuse.
pub(crate) fn relayout(
    theme: Res<Theme>,
    mut nodes: Query<
        (&mut Node, Has<Padded>, Option<&Anchored>, Has<LabelColumn>),
        Or<(With<Padded>, With<Anchored>, With<LabelColumn>)>,
    >,
    fresh: Query<(), Or<(Added<Padded>, Added<Anchored>, Added<LabelColumn>)>>,
) {
    if !theme.is_changed() && fresh.is_empty() {
        return;
    }

    let pad = theme.px(Metric::Pad);
    let gap = theme.px(Metric::Gap);
    let border = theme.px(Metric::Border);
    let radius = theme.px(Metric::Radius);
    let margin = theme.px(Metric::Margin);
    let label_width = theme.px(Metric::LabelWidth);

    for (mut node, padded, anchored, label) in &mut nodes {
        if padded {
            node.padding = UiRect::all(pad);
            node.row_gap = gap;
            node.column_gap = gap;
            node.border = UiRect::all(border);
            node.border_radius = BorderRadius::all(radius);
        }
        if let Some(Anchored(anchor)) = anchored {
            let (top, right, bottom, left) = anchor.offsets(margin);
            node.position_type = PositionType::Absolute;
            node.top = top;
            node.right = right;
            node.bottom = bottom;
            node.left = left;
        }
        if label {
            node.width = label_width;
        }
    }
}

// ---------------------------------------------------------------------------
// Panel
// ---------------------------------------------------------------------------

/// Marks a kit-made panel — useful to the game for telling "the cursor is over
/// the interface" from "the cursor is over the world".
#[derive(Component, Debug, Clone, Copy)]
pub struct Panel;

/// An anchored frame: translucent fill, hairline border, content stacked down
/// the column. `min_width` stops a panel whose contents change from breathing.
pub fn panel(anchor: Anchor, min_width: Option<f32>) -> impl Bundle {
    (
        Panel,
        Anchored(anchor),
        Padded,
        Node {
            flex_direction: FlexDirection::Column,
            min_width: min_width.map(px).unwrap_or(Val::Auto),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Fill(Role::PanelBg),
        BorderColor::all(Color::NONE),
        Edge(Role::PanelBorder),
        // Lets the picking pass track hover, which is how the game knows the
        // pointer has left the world.
        Interaction::default(),
    )
}

/// A full-screen dimmer to sit a modal on. Not `Padded`: a backdrop has no
/// inside.
pub fn backdrop() -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
        Fill(Role::Scrim),
        crate::overlay::Layer::Modal,
    )
}

/// A well: a second material inside a panel, for detail panes that should read
/// as their own surface.
pub fn card() -> impl Bundle {
    (
        Padded,
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::NONE),
        Fill(Role::CardBg),
        BorderColor::all(Color::NONE),
        Edge(Role::CardBorder),
    )
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

fn text_in(content: &str, role: Role, size: Metric, face: FontRole) -> impl Bundle {
    (
        Text::new(content.to_string()),
        TextFont::default(),
        TextSize(size),
        Face(face),
        TextColor(Color::WHITE),
        Ink(role),
    )
}

/// A title. Accent-coloured and set in the display face, because the eye should
/// find it first.
pub fn heading(content: &str) -> impl Bundle {
    text_in(
        content,
        Role::Accent,
        Metric::TitleSize,
        FontRole::Display,
    )
}

/// Body copy.
pub fn body(content: &str) -> impl Bundle {
    text_in(content, Role::Ink, Metric::BodySize, FontRole::Body)
}

/// Secondary text — labels, hints, readings. Found second, but still found.
pub fn dim(content: &str) -> impl Bundle {
    text_in(content, Role::InkDim, Metric::SmallSize, FontRole::Body)
}

// ---------------------------------------------------------------------------
// Rows
// ---------------------------------------------------------------------------

/// A horizontal line of content, at least one theme row high.
pub fn row() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        },
        RowHeight,
    )
}

/// Marks a row so its height tracks [`Metric::RowHeight`].
#[derive(Component, Debug, Clone, Copy)]
pub struct RowHeight;

/// A row's metric is a *floor*, not a fixed height.
///
/// Fixing the height looks right for a row of text and is wrong the moment a
/// row holds anything taller — a button is padded to nearly twice a text row,
/// and with `align_items: Center` it overflows the box in both directions and
/// lands on top of whatever sits above and below it.
pub(crate) fn resize_rows(
    theme: Res<Theme>,
    mut rows: Query<&mut Node, With<RowHeight>>,
    fresh: Query<(), Added<RowHeight>>,
) {
    if !theme.is_changed() && fresh.is_empty() {
        return;
    }
    let height = theme.px(Metric::RowHeight);
    let gap = theme.px(Metric::Gap);
    for mut node in &mut rows {
        node.min_height = height;
        node.column_gap = gap;
    }
}

/// The label half of a labelled row: fixed-width, dim, left-aligned.
pub fn label(content: &str) -> impl Bundle {
    (
        LabelColumn,
        Node::default(),
        children![dim(content)],
    )
}

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

/// Marks a button Ordo paints. Separate from [`UiButton`] so a game can put an
/// unpainted widget button on screen without the kit reaching for it.
#[derive(Component, Debug, Clone, Copy)]
pub struct OrdoButton;

/// A button.
///
/// Built on `bevy::ui_widgets::Button`, which fires `Activate` on click *and*
/// on Enter or Space while focused — keyboard and pad activation for free, and
/// a good reason not to write a state machine here. Observe `Activate` on the
/// returned entity to do something with it.
pub fn button(content: &str) -> impl Bundle {
    (
        UiButton,
        OrdoButton,
        Hovered::default(),
        Padded,
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        children![body(content)],
    )
}

/// Paints idle, hover and pressed from the theme.
///
/// A *removed* component is not a change any filter can watch for, so releases
/// have to be collected separately or every clicked button stays lit. When the
/// theme itself moves, every button repaints rather than only the touched one.
pub(crate) fn paint_buttons(
    theme: Res<Theme>,
    touched: Query<Entity, (With<OrdoButton>, Or<(Changed<Hovered>, Added<Pressed>)>)>,
    every: Query<Entity, With<OrdoButton>>,
    mut released: RemovedComponents<Pressed>,
    mut buttons: Query<
        (&Hovered, Has<Pressed>, &mut BackgroundColor, &mut BorderColor),
        With<OrdoButton>,
    >,
) {
    // Drained first either way: leaving releases unread would have them turn up
    // again next frame as a phantom repaint.
    let just_released: Vec<Entity> = released.read().collect();
    let candidates: Vec<Entity> = if theme.is_changed() {
        every.iter().collect()
    } else {
        touched.iter().chain(just_released).collect()
    };

    for entity in candidates {
        let Ok((hovered, pressed, mut background, mut border)) = buttons.get_mut(entity) else {
            continue;
        };
        let (fill, edge) = match (pressed, hovered.get()) {
            (true, _) => (Role::ButtonPressed, Role::Accent),
            (false, true) => (Role::ButtonHover, Role::Accent),
            (false, false) => (Role::ButtonIdle, Role::PanelBorder),
        };
        *background = BackgroundColor(theme.color(fill));
        *border = BorderColor::all(theme.color(edge));
    }
}
