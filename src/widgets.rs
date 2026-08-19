//! The handful of pieces that turned up in every game.
//!
//! Each one returns an `impl Bundle` where it can, so it drops straight into
//! `children![..]` and composes without a `Commands` in hand. The pieces that
//! genuinely produce more than one entity — a panel with a title bar — hand
//! back the entities the caller will want instead.
//!
//! Nothing here reads the theme. Widgets carry role and metric *tags*; the
//! passes in [`crate::theme`] fill in the color and the spacing, which is why
//! a running game answers an edit to the theme file.

use crate::tabs::Selected;
use crate::theme::{
    Edge, Face, Fill, FontRole, Ink, Metric, Opacity, Role, TextSize, Theme, tinted,
};
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

/// Where a panel sits. Corners and edge-midpoints rather than free
/// placement: a HUD is furniture, and furniture stands against the walls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    /// Centered on an edge — a quest tracker's home, halfway up the left.
    Left,
    Right,
    Top,
    Bottom,
    /// Dead center — nothing lives here long.
    Center,
    /// High and centered, about a third of the way down: where a herald
    /// stands. Dead center is where the eye already is and where the
    /// world's own business happens, so a card there covers the very
    /// thing it is announcing; a third down clears the ground and reads
    /// as a proclamation over the scene rather than a box in front of
    /// it.
    Herald,
}

impl Anchor {
    fn offsets(self, margin: Val) -> (Val, Val, Val, Val) {
        let auto = Val::Auto;
        let half = Val::Percent(50.0);
        match self {
            Anchor::TopLeft => (margin, auto, auto, margin),
            Anchor::TopRight => (margin, margin, auto, auto),
            Anchor::BottomLeft => (auto, auto, margin, margin),
            Anchor::BottomRight => (auto, margin, margin, auto),
            // The centered axis pins at fifty percent; `centering` hands the
            // matching half-size pullback, applied once at spawn.
            Anchor::Left => (half, auto, auto, margin),
            Anchor::Right => (half, margin, auto, auto),
            Anchor::Top => (margin, auto, auto, half),
            Anchor::Bottom => (auto, auto, margin, half),
            Anchor::Center => (half, auto, auto, half),
            Anchor::Herald => (Val::Percent(30.0), auto, auto, half),
        }
    }

    /// The translation that turns an edge pin into an edge CENTRE — minus
    /// half the panel's own size along the pinned axis. `None` for corners,
    /// which sit where their offsets put them.
    pub fn centering(self) -> Option<Val2> {
        match self {
            Anchor::Left | Anchor::Right => Some(Val2::new(Val::Px(0.0), Val::Percent(-50.0))),
            Anchor::Top | Anchor::Bottom => Some(Val2::new(Val::Percent(-50.0), Val::Px(0.0))),
            Anchor::Center => Some(Val2::new(Val::Percent(-50.0), Val::Percent(-50.0))),
            Anchor::Herald => Some(Val2::new(Val::Percent(-50.0), Val::Percent(-50.0))),
            _ => None,
        }
    }

    /// How a stack against this wall lines its cards up: hugging the edge
    /// it stands on.
    pub fn ranks(self) -> AlignItems {
        match self {
            Anchor::TopLeft | Anchor::BottomLeft | Anchor::Left => AlignItems::Start,
            Anchor::TopRight | Anchor::BottomRight | Anchor::Right => AlignItems::End,
            Anchor::Top | Anchor::Bottom | Anchor::Center | Anchor::Herald => AlignItems::Center,
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
        (
            &mut Node,
            Has<Padded>,
            Option<&Anchored>,
            Has<LabelColumn>,
            Has<Hanging>,
        ),
        Or<(With<Padded>, With<Anchored>, With<LabelColumn>)>,
    >,
    fresh: Query<
        (),
        Or<(
            Added<Padded>,
            Added<Anchored>,
            Added<LabelColumn>,
            Added<Hanging>,
        )>,
    >,
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

    for (mut node, padded, anchored, label, hanging) in &mut nodes {
        if padded {
            node.padding = UiRect::all(pad);
            node.row_gap = gap;
            node.column_gap = gap;
            node.border = UiRect::all(border);
            node.border_radius = BorderRadius::all(radius);
        }
        // Undone AFTER the padding, not instead of it: a hanging panel is
        // an ordinary panel with one edge given away to the screen.
        if hanging {
            node.padding.top = pad * 2.0;
            node.border.top = px(0.0);
            node.border_radius = BorderRadius {
                top_left: px(0.0),
                top_right: px(0.0),
                bottom_left: radius,
                bottom_right: radius,
            };
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

/// Marks a compact, edge-anchored inspection surface. Games use this for
/// people, buildings, and other live objects that deserve detail without
/// replacing the world beneath them.
#[derive(Component, Debug, Clone, Copy)]
pub struct InspectorPanel;

/// A right-side panel for inspecting a living part of the game world.
///
/// The caller owns its final width and height; Ordo owns the anchored panel
/// language so inspectors remain recognisable between games.
pub fn inspector_panel(min_width: f32) -> impl Bundle {
    (InspectorPanel, panel(Anchor::TopRight, Some(min_width)))
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

/// A hairline divider, full width.
///
/// The cheapest thing in the kit and the one that does most for a dense window: a settings
/// dialog is a stack of unrelated rows, and a rule is how the eye is told where one group
/// ends. Border rather than background, so it lands on the same hairline as every frame.
pub fn rule() -> impl Bundle {
    (
        Node {
            width: percent(100),
            border: UiRect::top(px(1.0)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        Edge(Role::PanelBorder),
    )
}

/// A titled divider: a short run, the words, a run to the edge.
///
/// THE ONE WAY TO OPEN A SECTION. Three of these grew up in one game - a centered
/// one, a plain dim line, and a tick-and-rule - and which a surface got came
/// down to who wrote it.
///
/// This is the tick, with the weights made equal, which is what Brett was
/// missing from it: "I like on the centered ones how the line looks like it is
/// the same weight on both sides of the word. I think I would like the tick
/// headers if the line was the same on both sides." The old tick differed on
/// three counts at once - 1.5px against 1px, accent against border ink, and 70%
/// alpha against 30% - so one side read as a bold gold dash and the other as a
/// faint gray hairline. ONE INK, ONE WEIGHT, and only the LENGTH differs now.
///
/// Left-aligned rather than centered because sections stack: down a tall window
/// every label starts at the same x, and a centered label lands somewhere new on
/// each line depending on how long its words are.
///
/// A `Commands` helper rather than a bundle, because it is three entities and a
/// bundle is one. Returns the row, so a caller may hide the whole heading when
/// its section has nothing to say - which is most of what a section header is
/// for in a live window.
pub fn section(commands: &mut Commands, parent: Entity, label: &str) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8.0),
                margin: UiRect::top(px(4.0)),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();
    // The two runs differ in LENGTH and in nothing else.
    let hairline = |commands: &mut Commands, lead: bool| {
        commands.spawn((
            Node {
                width: if lead { px(14.0) } else { Val::Auto },
                flex_grow: if lead { 0.0 } else { 1.0 },
                flex_shrink: 0.0,
                height: px(1.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Fill(Role::PanelBorder),
            ChildOf(row),
        ));
    };
    hairline(commands, true);
    commands.spawn((
        text_in(label, Role::Accent, Metric::SmallSize, FontRole::Display),
        ChildOf(row),
    ));
    hairline(commands, false);
    row
}

/// A stat tile: a quiet label over a value said loudly, in its own bordered card.
///
/// LIFTED OUT OF A GAME, which is the only way a kit earns a widget. Divus
/// Factus grew these for a villager's vitals - hunger, rest, health, spirits -
/// and they are the reason that window reads as designed rather than dumped:
/// four facts as four objects the eye can count, instead of four rows it has to
/// read. They tile two-up by default and wrap, so a section of them needs no
/// grid arithmetic.
///
/// The VALUE entity comes back, because a live window writes values every frame
/// and the kit never does. Its color is the caller's too - a value that goes
/// red when it matters is the whole point of a tile over a row.
pub fn stat_tile(commands: &mut Commands, parent: Entity, label: &str, value: &str) -> Entity {
    let tile = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: percent(45.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(10.0), px(7.0)),
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(px(3.0)),
                row_gap: px(2.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Fill(Role::TitleBg),
            BorderColor::all(Color::NONE),
            Edge(Role::PanelBorder),
            ChildOf(parent),
        ))
        .id();
    commands.spawn((
        text_in(label, Role::InkDim, Metric::SmallSize, FontRole::Display),
        ChildOf(tile),
    ));
    commands
        .spawn((
            text_in(value, Role::Ink, Metric::BodySize, FontRole::Body),
            ChildOf(tile),
        ))
        .id()
}

/// A well: the part of a tall window that scrolls, everything else staying put.
///
/// Also lifted rather than invented. A profile window with a dozen sections has
/// to scroll SOMETHING, and scrolling the whole panel takes the title and the
/// tabs away with it - so the frame holds still and the well moves. `min_height`
/// of nought is the part everybody forgets: without it a flex child refuses to
/// shrink below its content and the scroll never engages.
pub fn well() -> impl Bundle {
    (
        Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0.0),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            ..default()
        },
        bevy::ui::ScrollPosition::default(),
    )
}

/// A spacer that eats whatever room is left, so what follows it sits at the far edge.
///
/// A row of `[label] [gap] [value]` is the whole shape of a settings line, and doing it with
/// a fixed label width instead leaves the value stranded in the middle of a wide window.
pub fn spring() -> impl Bundle {
    Node {
        flex_grow: 1.0,
        ..default()
    }
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
    text_in(content, Role::Accent, Metric::TitleSize, FontRole::Display)
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
    (LabelColumn, Node::default(), children![dim(content)])
}

/// A readout: one small boxed figure, for a rail of them.
///
/// The shape the bench's own tool bar wears - a hairline box, a dark well,
/// one width for every cell in the rail. Brett, of that bar: "I like the
/// way the toolbar looks in Op, can we do something like that?"
///
/// One width for all of them, not each sized to its contents: a row of
/// figures that steps in and out as the eye runs along it reads as several
/// unrelated things rather than as one instrument. `wide` is that width.
///
/// It is not a [`button`] - nothing here is pressed - so it takes no
/// `Hovered` and no interaction, and its chrome is painted from the theme
/// like any other panel rather than from what a pointer is doing.
pub fn readout(wide: f32) -> impl Bundle {
    (
        Padded,
        Node {
            width: px(wide),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: px(6.0),
            border: UiRect::all(px(1.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Fill(Role::CardBg),
        BorderColor::all(Color::NONE),
        Edge(Role::CardBorder),
    )
}

/// Marks a panel FIXED TO THE TOP EDGE rather than floating over the view.
///
/// It is padded like any other panel, and then two things are undone: the
/// border and the rounding across the top, because a line drawn along an
/// edge the panel is already flush with reads as a seam; and the top
/// padding is doubled, because a hanging panel is slid up past its own
/// corners and the part of that padding above the screen is not padding at
/// all - it is off-screen. Sixteen pixels of air with fourteen of it
/// hidden leaves two, and the title sits on the edge it hangs from.
///
/// Its own component rather than the game reaching in, because
/// [`relayout`] rewrites padding and border wholesale for everything
/// `Padded` - so anything set from outside is overwritten on the next
/// theme change, quietly and a frame later.
#[derive(Component, Debug, Clone, Copy)]
pub struct Hanging;

/// A rail of readouts that hangs from the top edge of the screen.
///
/// Slide it up past its own corner radius and it reads as a tab fixed to
/// the edge; leave it where it lands and it is a box floating near one.
pub fn hanging_rail() -> impl Bundle {
    (
        Panel,
        Padded,
        Hanging,
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
        Fill(Role::PanelBg),
        BorderColor::all(Color::NONE),
        Edge(Role::PanelBorder),
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
///
/// A button's chrome is painted here rather than through `Fill` and `Edge`,
/// because its color depends on what the pointer is doing and the repaint pass
/// knows nothing about that. That makes this the one place a color is set
/// outside `theme::repaint`, so it carries the same obligation: honor
/// [`Opacity`]. Without it a fading menu fades its labels and leaves the boxes
/// behind them at full strength — and worse, only until the pointer moves,
/// since a button that isn't touched isn't repainted at all.
pub(crate) fn paint_buttons(
    theme: Res<Theme>,
    touched: Query<
        Entity,
        (
            With<OrdoButton>,
            Or<(
                Changed<Hovered>,
                Added<Pressed>,
                Changed<Opacity>,
                Added<Selected>,
            )>,
        ),
    >,
    every: Query<Entity, With<OrdoButton>>,
    mut released: RemovedComponents<Pressed>,
    mut deselected: RemovedComponents<Selected>,
    mut buttons: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<Selected>,
            Option<&Opacity>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<OrdoButton>,
    >,
) {
    // Drained first either way: leaving releases unread would have them turn up
    // again next frame as a phantom repaint. `Selected` needs the same treatment
    // for the same reason — a tab closing is a *removed* component, which no
    // filter can watch for, and without this the tab you just left keeps its
    // pressed face while the one you opened takes one too.
    let just_released: Vec<Entity> = released.read().collect();
    let just_closed: Vec<Entity> = deselected.read().collect();
    let candidates: Vec<Entity> = if theme.is_changed() {
        every.iter().collect()
    } else {
        touched
            .iter()
            .chain(just_released)
            .chain(just_closed)
            .collect()
    };

    for entity in candidates {
        let Ok((hovered, pressed, selected, opacity, mut background, mut border)) =
            buttons.get_mut(entity)
        else {
            continue;
        };
        // Pressed beats open beats hovered. An open tab still lights under the
        // pointer, or the thing you are already looking at stops answering you.
        let (fill, edge) = match (pressed, hovered.get(), crate::tabs::is_selected(selected)) {
            (true, _, _) => (Role::ButtonPressed, Role::Accent),
            (false, true, _) => (Role::ButtonHover, Role::Accent),
            (false, false, Some(open)) => open,
            (false, false, None) => (Role::ButtonIdle, Role::PanelBorder),
        };
        *background = BackgroundColor(tinted(&theme, fill, opacity));
        *border = BorderColor::all(tinted(&theme, edge, opacity));
    }
}
