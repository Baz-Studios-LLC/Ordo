//! A radial menu — choices arranged around the point you pressed.
//!
//! Earns its place because the gesture is one motion: press where the thing
//! should go, flick toward what it should be, release. The press point *is* the
//! target, so there is no second "now click the destination" step and nothing
//! to cancel. Please Don't Shake stocks its ant farm this way, and the same
//! shape suits any game that places something at a spot.
//!
//! Ordo owns the two parts that are the same in every game: where the wedges
//! sit, and which one an offset is pointing at. What opens the menu stays with
//! the game, because the gesture that summons it differs — a hold, a modifier,
//! a bound key — and that is not a thing a kit should decide.
//!
//! Colour here is *state*, not theme, so it follows [`crate::widgets`]'s button
//! precedent: one pass writes `TextColor` directly from the theme's roles.
//! Wedge labels therefore carry [`TextSize`] and [`Face`] — typography still
//! answers the theme file — but deliberately **no [`Ink`]**, because `repaint`
//! also writes `TextColor` and two writers over one colour is exactly how a
//! highlight ends up flickering.
//!
//! [`Ink`]: crate::theme::Ink

use crate::theme::{Face, FontRole, Metric, Role, TextSize, Theme};
use bevy::prelude::*;

/// The hub of a radial menu. Holds the state its wedges are painted from.
#[derive(Component, Debug, Clone)]
pub struct Radial {
    /// How many wedges hang off this hub. Sets their spacing, so it has to
    /// agree with the number actually spawned.
    pub count: usize,
    /// Which wedge the hand is pointing at, or `None` inside the dead zone.
    /// The game writes this; the paint pass reads it.
    pub selected: Option<usize>,
}

impl Radial {
    pub fn new(count: usize) -> Self {
        Self { count, selected: None }
    }

    /// Which wedge an offset from the hub points at.
    ///
    /// Wedge zero sits straight up and the rest follow clockwise, which is the
    /// order they read in on screen. `offset` is in UI coordinates, so `y`
    /// grows *downward*.
    ///
    /// Inside `dead_zone` the answer is `None` rather than the nearest wedge —
    /// opening a menu shouldn't instantly commit to whichever way your hand
    /// happened to drift on the way down.
    pub fn pick(&self, offset: Vec2, dead_zone: f32) -> Option<usize> {
        if self.count == 0 || offset.length() < dead_zone {
            return None;
        }
        let step = std::f32::consts::TAU / self.count as f32;
        // Measured from straight up, which is -y here.
        let from_up = offset.y.atan2(offset.x) + std::f32::consts::FRAC_PI_2;
        let index = (from_up / step).round().rem_euclid(self.count as f32) as usize;
        Some(index % self.count)
    }

    /// Where wedge `index` sits relative to the hub, before the radius is
    /// applied. Shared with [`place_wedges`] so the geometry that draws a wedge
    /// and the geometry that picks it can never disagree.
    pub fn direction(&self, index: usize) -> Vec2 {
        let step = std::f32::consts::TAU / self.count.max(1) as f32;
        let a = index as f32 * step;
        Vec2::new(a.sin(), -a.cos())
    }
}

/// One choice on the hub. The index sets where it sits and what
/// [`Radial::selected`] is compared against.
#[derive(Component, Debug, Clone, Copy)]
pub struct Wedge(pub usize);

/// A wedge that is shown but cannot be chosen — out of stock, not yet unlocked.
///
/// Present-but-dimmed rather than absent, so the menu keeps the same shape every
/// time it opens and stays learnable as muscle memory.
#[derive(Component, Debug, Clone, Copy)]
pub struct Spent;

/// A hub, placed at a point in UI space — usually wherever the pointer was.
///
/// Zero-sized on purpose: it is an origin, not a surface. Give it [`Wedge`]
/// children and [`place_wedges`] arranges them.
pub fn radial(at: Vec2, count: usize) -> impl Bundle {
    (
        Radial::new(count),
        Node {
            position_type: PositionType::Absolute,
            left: px(at.x),
            top: px(at.y),
            ..default()
        },
    )
}

/// One labelled wedge. Position comes from [`place_wedges`], colour from
/// [`paint_wedges`].
pub fn wedge(index: usize, content: &str) -> impl Bundle {
    (
        Wedge(index),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        Text::new(content.to_string()),
        TextFont::default(),
        TextSize(Metric::BodySize),
        Face(FontRole::Body),
        // Owned by `paint_wedges` from here on — see the module note.
        TextColor(Color::WHITE),
    )
}

/// Arrange wedges around their hub at [`Metric::RadialRadius`].
///
/// Runs on a theme change or a newly spawned wedge, so the radius is live in the
/// theme file like every other metric.
pub(crate) fn place_wedges(
    theme: Res<Theme>,
    hubs: Query<&Radial>,
    mut wedges: Query<(&Wedge, &ChildOf, &mut Node)>,
    fresh: Query<(), Added<Wedge>>,
) {
    if !theme.is_changed() && fresh.is_empty() {
        return;
    }

    let radius = theme.metric(Metric::RadialRadius);

    for (Wedge(index), parent, mut node) in &mut wedges {
        let Ok(hub) = hubs.get(parent.parent()) else {
            continue;
        };
        let at = hub.direction(*index) * radius;
        // Nudged back by half a line so a label reads as centred on its point
        // rather than hanging off it. Text has no measured size until layout has
        // run, so a fraction of the body size is the honest approximation.
        let half_line = theme.metric(Metric::BodySize) * 0.5;
        node.left = px(at.x - half_line * 2.0);
        node.top = px(at.y - half_line);
    }
}

/// Paint every wedge from the hub's selection.
///
/// Sole writer of a wedge's `TextColor`, which is why `wedge` ships no `Ink`.
pub(crate) fn paint_wedges(
    theme: Res<Theme>,
    hubs: Query<&Radial>,
    mut wedges: Query<(&Wedge, &ChildOf, Has<Spent>, &mut TextColor)>,
    changed: Query<(), Or<(Changed<Radial>, Added<Wedge>)>>,
) {
    if !theme.is_changed() && changed.is_empty() {
        return;
    }

    for (Wedge(index), parent, spent, mut colour) in &mut wedges {
        let Ok(hub) = hubs.get(parent.parent()) else {
            continue;
        };
        let role = if spent {
            // Found second, but still found — the wedge is meant to be legible.
            Role::InkDim
        } else if hub.selected == Some(*index) {
            Role::Accent
        } else {
            Role::Ink
        };
        colour.0 = theme.color(role);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wedge_zero_is_straight_up_and_the_rest_go_clockwise() {
        let hub = Radial::new(4);
        // UI space: y grows downward, so "up" is negative.
        assert_eq!(hub.pick(Vec2::new(0.0, -50.0), 10.0), Some(0));
        assert_eq!(hub.pick(Vec2::new(50.0, 0.0), 10.0), Some(1));
        assert_eq!(hub.pick(Vec2::new(0.0, 50.0), 10.0), Some(2));
        assert_eq!(hub.pick(Vec2::new(-50.0, 0.0), 10.0), Some(3));
    }

    #[test]
    fn the_dead_zone_answers_nothing() {
        let hub = Radial::new(4);
        assert_eq!(hub.pick(Vec2::new(0.0, -4.0), 10.0), None);
        assert_eq!(hub.pick(Vec2::ZERO, 10.0), None);
    }

    #[test]
    fn picking_and_placing_agree() {
        // The two halves of the geometry have to describe the same wedge, or a
        // menu highlights one thing and places another.
        for count in [2usize, 3, 4, 5, 6, 8] {
            let hub = Radial::new(count);
            for index in 0..count {
                let at = hub.direction(index) * 60.0;
                assert_eq!(
                    hub.pick(at, 10.0),
                    Some(index),
                    "count {count}, wedge {index}"
                );
            }
        }
    }

    #[test]
    fn an_empty_hub_picks_nothing() {
        assert_eq!(Radial::new(0).pick(Vec2::new(0.0, -50.0), 10.0), None);
    }
}
