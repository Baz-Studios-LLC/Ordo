//! Things that float above the interface.
//!
//! Divus Factus writes this pattern three times in one file — toasts, speech
//! bubbles, hover hints — and each time it is the same shape: put something on
//! top, cap how many can exist, age it out. Here it is once.
//!
//! Everything transient fades through [`Opacity`] rather than by writing
//! colours, so the repaint pass in [`crate::theme`] stays the only thing that
//! ever sets a colour.

use crate::theme::{Edge, Face, Fill, FontRole, Ink, Metric, Opacity, Role, TextSize};
use crate::widgets::{Anchor, Anchored, Padded, dim, heading};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Layering
// ---------------------------------------------------------------------------

/// Where something sits in the stack.
///
/// One vocabulary rather than scattered `GlobalZIndex` literals: DF already
/// manages window z-order by hand in `focus_windows`, and toasts, bubbles and
/// hints each picked their own number. Four consumers, one ladder.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Anchored furniture — the HUD.
    Panel,
    /// Movable windows, which sort among themselves above the furniture.
    Window,
    /// A modal and its scrim.
    Modal,
    /// Transient notices.
    Toast,
    /// Hover hints, which must clear everything or they are pointless.
    Tooltip,
}

impl Layer {
    pub const fn z(self) -> i32 {
        match self {
            Layer::Panel => 0,
            Layer::Window => 10,
            Layer::Modal => 100,
            Layer::Toast => 200,
            Layer::Tooltip => 300,
        }
    }
}

pub(crate) fn apply_layers(
    mut commands: Commands,
    layered: Query<(Entity, &Layer), Changed<Layer>>,
) {
    for (entity, layer) in &layered {
        commands.entity(entity).insert(GlobalZIndex(layer.z()));
    }
}

// ---------------------------------------------------------------------------
// Ageing
// ---------------------------------------------------------------------------

/// Fades in, dwells, fades out, despawns. The whole tween system, for now.
#[derive(Component, Debug, Clone, Copy)]
pub struct Lifetime {
    pub total: f32,
    pub fade: f32,
    elapsed: f32,
}

impl Lifetime {
    pub fn new(total: f32, fade: f32) -> Self {
        Self {
            total,
            fade: fade.min(total * 0.5),
            elapsed: 0.0,
        }
    }

    fn opacity(&self) -> f32 {
        let rising = self.elapsed / self.fade.max(f32::EPSILON);
        let falling = (self.total - self.elapsed) / self.fade.max(f32::EPSILON);
        smoothstep(rising.min(falling))
    }
}

/// Smoothstep. Linear fades read as mechanical; the ends want to be soft.
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Ages everything transient and writes the result down each subtree.
///
/// The subtree walk is here rather than in a propagation system because a
/// separate one would need `Opacity` both immutably (to find what changed) and
/// mutably (to write descendants), which Bevy will not allow in one system.
pub(crate) fn age(
    time: Res<Time>,
    mut commands: Commands,
    mut lives: Query<(Entity, &mut Lifetime)>,
    kids: Query<&Children>,
    mut opacities: Query<&mut Opacity>,
) {
    for (root, mut life) in &mut lives {
        life.elapsed += time.delta_secs();
        if life.elapsed >= life.total {
            commands.entity(root).despawn();
            continue;
        }
        let want = life.opacity();

        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if let Ok(mut opacity) = opacities.get_mut(node)
                && opacity.0 != want
            {
                opacity.0 = want;
            }
            if let Ok(children) = kids.get(node) {
                for i in 0..children.len() {
                    stack.push(children[i]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Notices
// ---------------------------------------------------------------------------

/// Something worth telling the player, briefly.
#[derive(Debug, Clone)]
pub struct Notice {
    pub text: String,
    /// Emphasis. DF distinguishes a routine notice from an event that deserves
    /// the accent colour, and it is the right distinction to keep.
    pub fanfare: bool,
}

impl Notice {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            fanfare: false,
        }
    }

    pub fn fanfare(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            fanfare: true,
        }
    }
}

/// Push notices here; the shelf drains it.
///
/// A plain queue rather than an event, so a game can post one from anywhere
/// without caring whether a shelf exists yet.
#[derive(Resource, Default)]
pub struct Notices(Vec<Notice>);

impl Notices {
    pub fn push(&mut self, notice: Notice) {
        self.0.push(notice);
    }

    pub fn say(&mut self, text: impl Into<String>) {
        self.push(Notice::new(text));
    }

    /// How many are still waiting for a shelf.
    pub fn pending(&self) -> usize {
        self.0.len()
    }
}

/// The corner toasts stack into.
#[derive(Component, Debug, Clone, Copy)]
pub struct ToastShelf;

#[derive(Component, Debug, Clone, Copy)]
pub struct Toast;

/// Past this many, the oldest goes. A shelf that grows without limit stops
/// being a notification and becomes a log nobody reads.
pub const TOAST_CAP: usize = 6;

/// How long a toast lives, and how much of that is spent fading.
pub const TOAST_LIFE: f32 = 4.5;
pub const TOAST_FADE: f32 = 0.45;

/// Somewhere for toasts to stack. Spawn one; the rest is automatic.
pub fn toast_shelf(anchor: Anchor) -> impl Bundle {
    (
        ToastShelf,
        Anchored(anchor),
        Layer::Toast,
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::End,
            row_gap: px(6.0),
            ..default()
        },
    )
}

pub(crate) fn show_notices(
    mut commands: Commands,
    mut notices: ResMut<Notices>,
    shelves: Query<Entity, With<ToastShelf>>,
) {
    if notices.0.is_empty() {
        return;
    }
    let Ok(shelf) = shelves.single() else {
        // No shelf on screen. Hold them rather than throwing them away: a
        // notice posted during loading should still arrive.
        return;
    };

    for notice in notices.0.drain(..) {
        let role = if notice.fanfare {
            Role::Accent
        } else {
            Role::Ink
        };
        commands.spawn((
            Toast,
            Padded,
            Lifetime::new(TOAST_LIFE, TOAST_FADE),
            Opacity(0.0),
            Node::default(),
            BackgroundColor(Color::NONE),
            Fill(Role::CardBg),
            BorderColor::all(Color::NONE),
            Edge(Role::CardBorder),
            ChildOf(shelf),
            children![(
                Text::new(notice.text),
                TextFont::default(),
                TextSize(Metric::BodySize),
                Face(FontRole::Body),
                TextColor(Color::WHITE),
                Ink(role),
                Opacity(0.0),
            )],
        ));
    }
}

/// Evicts from the front — `Children` keeps spawn order, so the front is the
/// oldest.
///
/// Its own system, watching `Children`, rather than a tail on [`show_notices`].
/// Spawns are deferred, so a toast is not among the shelf's children until the
/// frame after it is posted — by which time the queue is empty and a check
/// living inside `show_notices` has already returned early. The cap would never
/// have fired.
pub(crate) fn cap_shelf(
    mut commands: Commands,
    shelves: Query<&Children, (With<ToastShelf>, Changed<Children>)>,
) {
    for children in &shelves {
        let over = children.len().saturating_sub(TOAST_CAP);
        for i in 0..over {
            commands.entity(children[i]).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Tooltips
// ---------------------------------------------------------------------------

/// A hover hint. Two parts, as DF has it: a title and a line beneath.
///
/// This matters more here than in most kits — Divus Factus is mouse-only with
/// no focus ring, so the cursor and what it reveals carry the whole affordance
/// load.
/// `Hovered` is required rather than expected: picking only tracks entities
/// that carry it, and a plain line of text does not have one the way a button
/// does. Forgetting it would make the hint silently never appear.
#[derive(Component, Debug, Clone)]
#[require(Hovered)]
pub struct Tooltip {
    pub title: String,
    pub line: String,
}

impl Tooltip {
    pub fn new(title: impl Into<String>, line: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            line: line.into(),
        }
    }
}

/// The floating hint itself.
#[derive(Component, Debug, Clone, Copy)]
pub struct TooltipView;

/// How long the pointer must rest before a hint appears. Without a delay,
/// crossing a dense panel sets off every hint on the way past.
pub const TOOLTIP_DELAY: f32 = 0.35;

#[derive(Resource, Default)]
pub(crate) struct HoverClock {
    resting_on: Option<Entity>,
    elapsed: f32,
}

pub(crate) fn track_hover(
    time: Res<Time>,
    mut clock: ResMut<HoverClock>,
    hovered: Query<(Entity, &Hovered), With<Tooltip>>,
) {
    let now = hovered
        .iter()
        .find(|(_, hovered)| hovered.get())
        .map(|(entity, _)| entity);

    if now != clock.resting_on {
        clock.resting_on = now;
        clock.elapsed = 0.0;
    } else if now.is_some() {
        clock.elapsed += time.delta_secs();
    }
}

pub(crate) fn show_tooltips(
    mut commands: Commands,
    clock: Res<HoverClock>,
    hints: Query<&Tooltip>,
    shown: Query<Entity, With<TooltipView>>,
) {
    let wanted = clock
        .resting_on
        .filter(|_| clock.elapsed >= TOOLTIP_DELAY)
        .and_then(|entity| hints.get(entity).ok());

    match wanted {
        None => {
            for view in &shown {
                commands.entity(view).despawn();
            }
        }
        Some(hint) => {
            if !shown.is_empty() {
                return;
            }
            commands.spawn((
                TooltipView,
                Padded,
                Layer::Tooltip,
                Opacity(0.0),
                Node {
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Column,
                    max_width: px(280.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Fill(Role::CardBg),
                BorderColor::all(Color::NONE),
                Edge(Role::CardBorder),
                children![
                    (heading(&hint.title), Opacity(0.0)),
                    (dim(&hint.line), Opacity(0.0)),
                ],
            ));
        }
    }
}

/// Follows the pointer, and fades in once placed.
///
/// Placement flips to the other side of the cursor near an edge. This is the
/// job `bevy_ui_widgets::Popover` exists to do properly, with real anchor
/// alignment — worth moving to once the rest of the kit settles.
pub(crate) fn place_tooltips(
    time: Res<Time>,
    windows: Query<&Window>,
    mut views: Query<(&mut Node, &mut Opacity), With<TooltipView>>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let (width, height) = (window.width(), window.height());

    for (mut node, mut opacity) in &mut views {
        // A guess at the hint's extent is enough to decide which side of the
        // cursor it belongs on; being exact would need a frame of layout, and
        // the hint would visibly jump.
        if cursor.x > width - 300.0 {
            node.left = Val::Auto;
            node.right = px(width - cursor.x + 16.0);
        } else {
            node.right = Val::Auto;
            node.left = px(cursor.x + 16.0);
        }
        if cursor.y > height - 120.0 {
            node.top = Val::Auto;
            node.bottom = px(height - cursor.y + 16.0);
        } else {
            node.bottom = Val::Auto;
            node.top = px(cursor.y + 20.0);
        }

        let want = (opacity.0 + time.delta_secs() / TOOLTIP_DELAY).min(1.0);
        if opacity.0 != want {
            opacity.0 = want;
        }
    }
}

/// Tooltips are spawned without children carrying [`Opacity`], so the text
/// fades with the frame. Same subtree walk as [`age`], same reason.
pub(crate) fn fade_tooltip_text(
    views: Query<(Entity, &Opacity), With<TooltipView>>,
    kids: Query<&Children>,
    mut opacities: Query<&mut Opacity, Without<TooltipView>>,
) {
    for (root, opacity) in &views {
        let want = opacity.0;
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if let Ok(mut child) = opacities.get_mut(node)
                && child.0 != want
            {
                child.0 = want;
            }
            if let Ok(children) = kids.get(node) {
                for i in 0..children.len() {
                    stack.push(children[i]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lifetime_rises_dwells_and_falls() {
        let mut life = Lifetime::new(4.0, 1.0);
        assert_eq!(life.opacity(), 0.0, "starts invisible");

        life.elapsed = 0.5;
        let rising = life.opacity();
        assert!(rising > 0.0 && rising < 1.0, "mid-rise, got {rising}");

        life.elapsed = 2.0;
        assert_eq!(life.opacity(), 1.0, "full through the dwell");

        life.elapsed = 4.0;
        assert_eq!(life.opacity(), 0.0, "gone at the end");
    }

    /// Otherwise the rise and the fall overlap and the toast never reaches full
    /// opacity — it just swells and shrinks, which reads as a glitch.
    #[test]
    fn a_fade_can_never_exceed_half_the_life() {
        assert_eq!(Lifetime::new(1.0, 10.0).fade, 0.5);
    }

    #[test]
    fn the_layers_are_strictly_ordered() {
        let ladder = [
            Layer::Panel,
            Layer::Window,
            Layer::Modal,
            Layer::Toast,
            Layer::Tooltip,
        ];
        for rung in ladder.windows(2) {
            assert!(
                rung[0].z() < rung[1].z(),
                "{:?} should sit below {:?}",
                rung[0],
                rung[1]
            );
        }
    }
}
