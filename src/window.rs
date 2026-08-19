//! Windows: titled, movable, closable, and stacked by whoever touched one last.
//!
//! Lifted out of Divus Factus, which had all of this and Ordo had none of it.
//!
//! # A window is not a bundle
//!
//! Every other widget here is `-> impl Bundle`, spawned with its content in a
//! `children!`. A window cannot be, because it has chrome the caller does not
//! build - a title bar, a close button - and a body the caller fills, and two
//! `children!` on one entity is not a thing.
//!
//! So the chrome is not spawned with the window. It is put on afterward by
//! [`dress_windows`], which is the same move Ordo already makes with paint:
//! nothing is painted at spawn either, a tag says what a thing wants and a pass
//! gives it. A window carries [`Titled`] and gets a title bar the same way a
//! node carries [`Fill`] and gets a color.
//!
//! ```ignore
//! commands.spawn((
//!     window("The Ledger", Anchor::BottomLeft, 420.0),
//!     children![heading("Stores"), row_of("Timber", "86")],
//! ));
//! ```
//!
//! The caller's children are the body. The chrome arrives above them.

use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use crate::theme::{Edge, Face, FontRole, Ink, Metric, Role, TextSize};
use crate::widgets::Padded;

/// A movable, closable window. Open and shut is its own `Visibility`, so a game
/// keeps whatever it already uses to decide that.
#[derive(Component)]
pub struct Window;

/// What a window is called. Read once by [`dress_windows`] and left alone
/// after, so retitling means editing the title's own `Text`.
#[derive(Component)]
pub struct Titled(pub String);

/// Set on a window once its chrome is on, so the pass does not do it twice.
#[derive(Component)]
pub struct Dressed;

/// A window's title bar; points at the window it moves.
#[derive(Component)]
pub struct DragHandle(pub Entity);

/// A window's close button; points at the window it shuts.
#[derive(Component)]
pub struct CloseButton(pub Entity);

/// The window a title bar is being dragged by, and where inside it the pointer
/// took hold. Holding the grip is what stops a window snapping its corner to
/// the cursor on the first frame.
#[derive(Resource, Default)]
pub struct Dragging {
    held: Option<(Entity, Vec2)>,
}

/// A titled window, to be spawned with its body as `children!`.
///
/// It opens at the corner asked for and goes absolute the moment it is dragged,
/// which is what lets a window open somewhere sensible and still end up
/// anywhere.
///
/// The anchor is not a nicety. A window carries its own `Node`, so a caller
/// cannot add one to place it - two `Node`s in one bundle is a panic, not a
/// merge - and without an anchor every window in a game opens in the same
/// corner on top of the last. Found by doing exactly that.
pub fn window(title: &str, anchor: crate::widgets::Anchor, min_width: f32) -> impl Bundle {
    (
        Window,
        Titled(title.to_string()),
        crate::widgets::Anchored(anchor),
        Node {
            flex_direction: FlexDirection::Column,
            min_width: Val::Px(min_width),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        crate::theme::Fill(Role::PanelBg),
        BorderColor::all(Color::NONE),
        Edge(Role::CardBorder),
        Interaction::default(),
    )
}

/// Puts the chrome on a window that has not got it yet.
///
/// The title bar goes in as the FIRST child, before whatever the caller spawned
/// as the body - which is why this can run a frame late and still be right.
pub fn dress_windows(
    mut commands: Commands,
    bare: Query<(Entity, &Titled), (With<Window>, Without<Dressed>)>,
) {
    for (window, titled) in &bare {
        let bar = commands
            .spawn((
                DragHandle(window),
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                crate::theme::Fill(Role::TitleBg),
                Padded,
                Interaction::default(),
            ))
            .id();
        commands.spawn((
            Text::new(titled.0.clone()),
            TextColor(Color::WHITE),
            Ink(Role::Accent),
            Face(FontRole::Display),
            TextSize(Metric::TitleSize),
            ChildOf(bar),
        ));
        commands.spawn((
            CloseButton(window),
            bevy::ui::widget::Button,
            Interaction::default(),
            Text::new("x"),
            TextColor(Color::WHITE),
            Ink(Role::InkDim),
            Face(FontRole::Body),
            TextSize(Metric::SmallSize),
            ChildOf(bar),
        ));

        // FIRST, ahead of the body the caller spawned. `insert_children` at
        // index nought is the whole reason the chrome can arrive late.
        commands.entity(window).insert_children(0, &[bar]);
        commands.entity(window).insert(Dressed);
    }
}

/// Carries a window by its title bar.
pub fn drag_windows(
    primary: Query<&bevy::window::Window, With<bevy::window::PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut dragging: ResMut<Dragging>,
    handles: Query<(&DragHandle, &Interaction)>,
    mut windows: Query<(&mut Node, &ComputedNode, &UiGlobalTransform)>,
) {
    let Ok(primary) = primary.single() else {
        return;
    };
    let Some(cursor) = primary.cursor_position() else {
        return;
    };
    if !buttons.pressed(MouseButton::Left) {
        dragging.held = None;
        return;
    }

    // Taking hold: the grip is where in the window the pointer landed, so the
    // window keeps its offset instead of snapping its corner to the cursor.
    if dragging.held.is_none() {
        for (handle, interaction) in &handles {
            if *interaction != Interaction::Pressed {
                continue;
            }
            let Ok((_, computed, at)) = windows.get(handle.0) else {
                continue;
            };
            let scale = computed.inverse_scale_factor();
            let middle = Vec2::new(at.translation.x, at.translation.y) * scale;
            let corner = middle - computed.size() * scale * 0.5;
            dragging.held = Some((handle.0, cursor - corner));
            break;
        }
    }

    let Some((window, grip)) = dragging.held else {
        return;
    };
    let Ok((mut node, ..)) = windows.get_mut(window) else {
        return;
    };
    // Absolute from the first drag on. Until then a window sits wherever the
    // layout put it, which is what lets it open somewhere sensible.
    node.position_type = PositionType::Absolute;
    node.left = Val::Px(cursor.x - grip.x);
    node.top = Val::Px(cursor.y - grip.y);
    node.right = Val::Auto;
    node.bottom = Val::Auto;
}

/// Shuts a window when its close button is pressed.
pub fn close_windows(
    buttons: Query<(&CloseButton, &Interaction), Changed<Interaction>>,
    mut windows: Query<&mut Visibility, With<Window>>,
) {
    for (close, interaction) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Ok(mut showing) = windows.get_mut(close.0) {
            *showing = Visibility::Hidden;
        }
    }
}

/// The window last touched is the window in front.
///
/// By `GlobalZIndex` rather than by re-ordering children, so a window's place
/// in whatever laid it out is left alone - and a window that has never been
/// touched keeps the order it was spawned in.
pub fn focus_windows(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    primary: Query<&bevy::window::Window, With<bevy::window::PrimaryWindow>>,
    windows: Query<
        (
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
        ),
        With<Window>,
    >,
    shown: Query<Entity, (With<Window>, Changed<Visibility>)>,
    mut top: Local<i32>,
) {
    // A window that has just been opened comes to the front without being
    // clicked: opening one IS reaching for it.
    for window in &shown {
        *top += 1;
        commands.entity(window).insert(GlobalZIndex(*top));
    }

    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(primary) = primary.single() else {
        return;
    };
    let Some(cursor) = primary.cursor_position() else {
        return;
    };

    // Hit-tested by geometry rather than by hover, so a button inside a window
    // does not swallow the raise - clicking anything in a window is touching
    // the window.
    let mut hit: Option<Entity> = None;
    for (window, computed, at, visible) in &windows {
        if !visible.get() {
            continue;
        }
        let scale = computed.inverse_scale_factor();
        let middle = Vec2::new(at.translation.x, at.translation.y) * scale;
        let half = computed.size() * scale * 0.5;
        if (cursor.x - middle.x).abs() <= half.x && (cursor.y - middle.y).abs() <= half.y {
            hit = Some(window);
        }
    }
    if let Some(window) = hit {
        *top += 1;
        commands.entity(window).insert(GlobalZIndex(*top));
    }
}
