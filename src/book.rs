//! The book: a full-screen reading scaffold.
//!
//! One grand surface instead of a floating panel — a left rail of chapters,
//! a content region that owns the rest of the screen, and a footer band the
//! game may dock its own furniture into (time controls, status lines). The
//! kit owns the architecture and the material; what the chapters ARE stays
//! the game's.
//!
//! Material: the night-ledger — layered dark surfaces with light falling
//! down them ([`Sheen`]), an embossed frame, and cornerwork. Vivid colour
//! is the CONTENT's job (liveries, schools, faith bands); the book itself
//! stays quiet so the colour reads.

use bevy::prelude::*;
use bevy::ui::BackgroundGradient;

use crate::overlay::Layer;
use crate::theme::{Edge, Face, Fill, FontRole, Metric, Role, Sheen, TextSize};
use crate::widgets::{Panel, dim, heading};

/// The pieces of a book a caller wires into: the root (to show and hide),
/// the rail (to fill with chapters), the content (to fill with pages), the
/// footer (to dock furniture), and the two title texts.
pub struct BookParts {
    pub root: Entity,
    pub rail: Entity,
    pub content: Entity,
    pub footer: Entity,
    pub title: Entity,
    pub subtitle: Entity,
}

/// A chapter button in the rail. The kit styles hover; the ACTIVE chapter
/// is the game's to mark, since only the game knows which page is open.
#[derive(Component, Debug, Clone, Copy)]
pub struct ChapterButton;

/// Raises the book, hidden. Full-screen, above the HUD, under tooltips.
pub fn book(commands: &mut Commands, title: &str, subtitle: &str) -> BookParts {
    let root = commands
        .spawn((
            Panel,
            Layer::Window,
            Interaction::default(),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                padding: UiRect::all(Val::Px(14.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            // The world dims behind the page but stays visible: a god
            // reads while the world turns.
            BackgroundColor(Color::BLACK.with_alpha(0.42)),
        ))
        .id();

    // The frame: the book's own body, an embossed surface inside the dim.
    let frame = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundGradient::default(),
            Sheen::new(Role::TitleBg, 1.0, Role::PanelBg, 1.0),
            Edge(Role::Accent),
            crate::theme::Opacity(0.55),
            ChildOf(root),
        ))
        .id();
    adorn(commands, frame);

    // The rail: chapters down the left, under the book's own name.
    let rail = commands
        .spawn((
            Node {
                width: Val::Px(236.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(14.0)),
                row_gap: Val::Px(4.0),
                border: UiRect::right(Val::Px(1.0)),
                ..default()
            },
            BackgroundGradient::default(),
            Sheen::new(Role::PanelBg, 1.0, Role::TitleBg, 0.9),
            Edge(Role::PanelBorder),
            ChildOf(frame),
        ))
        .id();
    let title_entity = commands.spawn((heading(title), ChildOf(rail))).id();
    let subtitle_entity = commands.spawn((dim(subtitle), ChildOf(rail))).id();
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(10.0)),
            ..default()
        },
        Fill(Role::Accent),
        crate::theme::Opacity(0.35),
        ChildOf(rail),
    ));

    // The main column: content above, footer band below.
    let main = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ChildOf(frame),
        ))
        .id();
    let content = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            ChildOf(main),
        ))
        .id();
    let footer = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(46.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                border: UiRect::top(Val::Px(1.0)),
                ..default()
            },
            BackgroundGradient::default(),
            Sheen::new(Role::TitleBg, 0.9, Role::PanelBg, 1.0),
            Edge(Role::PanelBorder),
            ChildOf(main),
        ))
        .id();

    BookParts {
        root,
        rail,
        content,
        footer,
        title: title_entity,
        subtitle: subtitle_entity,
    }
}

/// Adds a chapter to the rail: a quiet wide button the game tags and
/// styles as its own. Returns the button; its label rides inside.
pub fn chapter(commands: &mut Commands, rail: Entity, label: &str) -> Entity {
    let button = commands
        .spawn((
            ChapterButton,
            crate::widgets::OrdoButton,
            Interaction::default(),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            ChildOf(rail),
        ))
        .id();
    let text = commands.spawn(dim(label)).id();
    commands.entity(text).insert((
        TextSize(Metric::BodySize),
        Face(FontRole::Display),
        ChildOf(button),
    ));
    button
}

/// The cornerwork: a pair of fine lines lapping each corner of a frame,
/// the difference between a rectangle and a made thing.
pub fn adorn(commands: &mut Commands, frame: Entity) {
    for (h, v) in [
        (
            (Val::Px(6.0), Val::Auto, Val::Px(6.0), Val::Auto),
            (Val::Px(6.0), Val::Auto, Val::Px(6.0), Val::Auto),
        ),
        (
            (Val::Px(6.0), Val::Auto, Val::Auto, Val::Px(6.0)),
            (Val::Px(6.0), Val::Auto, Val::Auto, Val::Px(6.0)),
        ),
        (
            (Val::Auto, Val::Px(6.0), Val::Px(6.0), Val::Auto),
            (Val::Auto, Val::Px(6.0), Val::Px(6.0), Val::Auto),
        ),
        (
            (Val::Auto, Val::Px(6.0), Val::Auto, Val::Px(6.0)),
            (Val::Auto, Val::Px(6.0), Val::Auto, Val::Px(6.0)),
        ),
    ] {
        let (top, bottom, left, right) = h;
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                top,
                bottom,
                left,
                right,
                width: Val::Px(26.0),
                height: Val::Px(2.0),
                ..default()
            },
            Fill(Role::Accent),
            crate::theme::Opacity(0.65),
            ChildOf(frame),
        ));
        let (top, bottom, left, right) = v;
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                top,
                bottom,
                left,
                right,
                width: Val::Px(2.0),
                height: Val::Px(26.0),
                ..default()
            },
            Fill(Role::Accent),
            crate::theme::Opacity(0.65),
            ChildOf(frame),
        ));
    }
}

/// A count chip: a small colour-edged pill carrying a label and a live
/// numeral — "HUNTERS 3" in the hunter's own leather-brown. The caller
/// writes the numeral; the colour is the caller's language.
pub struct ChipParts {
    pub root: Entity,
    pub value: Entity,
}

pub fn chip(commands: &mut Commands, parent: Entity, label: &str, color: Color) -> ChipParts {
    let root = commands
        .spawn((
            Interaction::default(),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Baseline,
                column_gap: Val::Px(6.0),
                padding: UiRect::axes(Val::Px(9.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(999.0)),
                ..default()
            },
            BackgroundColor(color.with_alpha(0.13)),
            BorderColor::all(color.with_alpha(0.8)),
            ChildOf(parent),
        ))
        .id();
    let label_entity = commands.spawn(dim(label)).id();
    commands.entity(label_entity).insert(ChildOf(root));
    let value = commands.spawn(crate::widgets::body("")).id();
    commands
        .entity(value)
        .insert((TextColor(color), ChildOf(root)));
    ChipParts { root, value }
}

/// A great numeral over a small-caps label — the ledger's plates, made a
/// widget: SOULS over 10, HUNGRY over 2. Returns the numeral to write.
pub fn plate(commands: &mut Commands, parent: Entity, label: &str) -> Entity {
    let well = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(2.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundGradient::default(),
            Sheen::new(Role::CardBg, 1.0, Role::PanelBg, 0.8),
            Edge(Role::CardBorder),
            ChildOf(parent),
        ))
        .id();
    let value = commands.spawn(heading("")).id();
    commands.entity(value).insert(ChildOf(well));
    let label_entity = commands.spawn(dim(label)).id();
    commands.entity(label_entity).insert(ChildOf(well));
    value
}
