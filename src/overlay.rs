//! Things that float above the interface.
//!
//! Divus Factus writes this pattern three times in one file — toasts, speech
//! bubbles, hover hints — and each time it is the same shape: put something on
//! top, cap how many can exist, age it out. Here it is once.
//!
//! Everything transient fades through [`Opacity`] rather than by writing
//! colours, so the repaint pass in [`crate::theme`] stays the only thing that
//! ever sets a colour.

use crate::theme::{Edge, Face, Fill, FontRole, Ink, Metric, Opacity, Role, TextSize, Theme};
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
    /// Pinned to things in the WORLD — nameplates, placards, bubbles.
    /// Under everything: interface chrome dragged across a world-anchored
    /// card must cover it, because the card belongs to the world.
    World,
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
            Layer::World => -10,
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
/// A shelf: an edge-docked stack of cards. Quest trackers, buff bars,
/// objective pins, toast trays — every game wants at least one wall of
/// small cards, and this is the wall. Dock it to a corner or an edge
/// centre; it stacks its children downward, ranked against the edge it
/// stands on, and leaves what the cards ARE entirely to the game.
pub fn shelf(anchor: Anchor) -> impl Bundle {
    (
        Anchored(anchor),
        Layer::Panel,
        Node {
            flex_direction: FlexDirection::Column,
            align_items: anchor.ranks(),
            row_gap: px(6.0),
            ..default()
        },
        UiTransform {
            translation: anchor
                .centering()
                .unwrap_or(Val2::new(Val::Px(0.0), Val::Px(0.0))),
            ..default()
        },
    )
}

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

// ---------------------------------------------------------------------------
// Proclamations
// ---------------------------------------------------------------------------

/// The trumpet, not the doorbell: a centre-stage card for the events a
/// session gets a handful of — a birth, a founding, a hall raised. It fades
/// and swells in, holds while its confetti falls, and bows out; when two
/// great things happen at once, the second WAITS. Ceremony that queues is
/// ceremony; ceremony that stacks is noise.
///
/// The kit owns the stage, the card, the choreography and the confetti.
/// What the moments are, what they say, what colour they wear and what
/// sound they make stay the game's.
#[derive(Debug, Clone)]
pub struct Proclamation {
    pub title: String,
    pub line: String,
    /// The card's ink — border, title and confetti. Colour is the KIND
    /// here, part of a game's own language, so it arrives as paint rather
    /// than as a theme role.
    pub color: Color,
    /// An opaque token handed back on the card's press — a game packs
    /// whatever it needs to answer a click (an entity's bits, an index).
    pub token: Option<u64>,
}

/// The waiting line. Push from anywhere; the stage drains it one at a time.
#[derive(Resource, Default)]
pub struct Proclamations(pub Vec<Proclamation>);

impl Proclamations {
    pub fn push(&mut self, proclamation: Proclamation) {
        self.0.push(proclamation);
    }
}

/// Where proclamations play. Spawn one with [`proclamation_stage`]; without
/// it, pushed proclamations wait patiently and nothing is lost.
#[derive(Component, Debug, Clone, Copy)]
pub struct ProclamationStage;

/// The card on stage right now.
#[derive(Component)]
pub struct Proclaimed {
    age: f32,
    /// Seconds of entrance, hold, and exit.
    rise: f32,
    hold: f32,
    fall: f32,
    ink: Color,
}

/// The token of the card on stage, for the game's press handler.
#[derive(Component, Debug, Clone, Copy)]
pub struct ProclaimedToken(pub u64);

/// One fleck of card-coloured confetti, living entirely in the interface.
#[derive(Component)]
pub(crate) struct Confetti {
    velocity: Vec2,
    spin: f32,
    age: f32,
    life: f32,
}

/// Centre stage, above the furniture with the toasts.
pub fn proclamation_stage() -> impl Bundle {
    (
        ProclamationStage,
        Anchored(Anchor::Center),
        Layer::Toast,
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        UiTransform {
            translation: Val2::new(Val::Percent(-50.0), Val::Percent(-50.0)),
            ..default()
        },
    )
}

/// Puts the next waiting proclamation on an empty stage.
pub(crate) fn stage_proclamations(
    mut commands: Commands,
    mut queue: ResMut<Proclamations>,
    theme: Res<Theme>,
    stages: Query<Entity, With<ProclamationStage>>,
    playing: Query<(), With<Proclaimed>>,
) {
    if queue.0.is_empty() || !playing.is_empty() {
        return;
    }
    let Ok(stage) = stages.single() else {
        return;
    };
    let Proclamation {
        title,
        line,
        color,
        token,
    } = queue.0.remove(0);

    let card = commands
        .spawn((
            Proclaimed {
                age: 0.0,
                rise: 0.35,
                hold: 2.8,
                fall: 0.7,
                ink: color,
            },
            Interaction::default(),
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: theme.px(Metric::Gap),
                padding: UiRect::axes(Val::Px(26.0), Val::Px(16.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(theme.color(Role::PanelBg)),
            BorderColor::all(color),
            UiTransform::default(),
            ChildOf(stage),
        ))
        .id();
    if let Some(token) = token {
        commands.entity(card).insert(ProclaimedToken(token));
    }
    let title_entity = commands.spawn(heading(&title)).id();
    commands
        .entity(title_entity)
        .insert((TextColor(color), ChildOf(card)));
    let line_entity = commands.spawn(dim(&line)).id();
    commands.entity(line_entity).insert(ChildOf(card));

    // The confetti: a dozen flecks of the card's own colour, thrown from
    // the top corners, falling under interface gravity.
    for i in 0..14 {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let throw = 30.0 + (i as f32 * 7.3) % 60.0;
        commands.spawn((
            Confetti {
                velocity: Vec2::new(side * throw, -90.0 - (i as f32 * 11.0) % 70.0),
                spin: side * (60.0 + (i as f32 * 23.0) % 120.0),
                age: 0.0,
                life: 1.3 + (i as f32 * 0.13) % 0.8,
            },
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(8.0),
                left: Val::Percent(if side < 0.0 { 6.0 } else { 94.0 }),
                width: Val::Px(5.0),
                height: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(color),
            UiTransform::default(),
            ChildOf(card),
        ));
    }
}

/// Plays the card: swell in, hold, bow out — and rains the confetti.
pub(crate) fn play_proclamations(
    mut commands: Commands,
    time: Res<Time>,
    mut cards: Query<(Entity, &mut Proclaimed, &mut UiTransform, &mut BorderColor)>,
    mut flecks: Query<
        (
            Entity,
            &mut Confetti,
            &mut UiTransform,
            &mut BackgroundColor,
        ),
        Without<Proclaimed>,
    >,
) {
    let dt = time.delta_secs();
    for (card, mut played, mut pose, mut border) in &mut cards {
        played.age += dt;
        let (rise, hold, fall) = (played.rise, played.hold, played.fall);
        let scale = if played.age < rise {
            // Swell: most of the size arrives early, the last sliver eases.
            let t = (played.age / rise).clamp(0.0, 1.0);
            0.86 + 0.14 * (1.0 - (1.0 - t) * (1.0 - t))
        } else {
            1.0
        };
        pose.scale = Vec2::splat(scale);
        // The border breathes while the card holds.
        let breathing = 0.75 + 0.25 * (played.age * 5.0).sin().abs();
        *border = BorderColor::all(played.ink.with_alpha(breathing));
        if played.age > rise + hold + fall {
            commands.entity(card).despawn();
        }
    }
    // Interface gravity: down is +y in screen space.
    for (fleck, mut confetti, mut pose, mut paint) in &mut flecks {
        confetti.age += dt;
        if confetti.age > confetti.life {
            commands.entity(fleck).despawn();
            continue;
        }
        confetti.velocity.y += 240.0 * dt;
        let x = confetti.velocity.x * confetti.age;
        let y = confetti.velocity.y * confetti.age * 0.5;
        pose.translation = Val2::new(Val::Px(x), Val::Px(y));
        pose.rotation = Rot2::degrees(confetti.spin * confetti.age);
        let fade = 1.0 - (confetti.age / confetti.life);
        let color = paint.0.with_alpha(fade);
        paint.0 = color;
    }
}
