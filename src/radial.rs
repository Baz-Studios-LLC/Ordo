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
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::math::Rot2;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::ui::UiTransform;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

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

/// One labelled wedge: a centred box holding a single line of text.
///
/// A box rather than a bare text node, because a bare one has no width to centre on and
/// wraps at whatever the layout gives it — "Ant Kit 1" came out as three stacked lines
/// spilling over the edge of its slice. The box is sized and placed by [`place_wedges`];
/// colour comes from [`paint_wedges`].
pub fn wedge(index: usize, content: &str) -> impl Bundle {
    (
        Wedge(index),
        Node {
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            overflow: Overflow::visible(),
            ..default()
        },
        children![(
            Text::new(content.to_string()),
            TextFont::default(),
            TextSize(Metric::BodySize),
            Face(FontRole::Body),
            // One line, always. A slice is not wide enough to wrap inside.
            TextLayout::no_wrap().with_justify(Justify::Center),
            // Owned by `paint_wedges` from here on — see the module note.
            TextColor(Color::WHITE),
        )],
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
    // A box wide enough for a short label and tall enough for one line of it. Centring a
    // known box beats guessing at text metrics, which aren't known until layout has run.
    let box_w = radius * LABEL_BOX_W;
    let box_h = theme.metric(Metric::BodySize) * LABEL_BOX_H;

    for (Wedge(index), parent, mut node) in &mut wedges {
        let Ok(hub) = hubs.get(parent.parent()) else {
            continue;
        };
        let at = hub.direction(*index) * radius;
        node.width = px(box_w);
        node.height = px(box_h);
        node.left = px(at.x - box_w * 0.5);
        node.top = px(at.y - box_h * 0.5);
    }
}

/// Paint every wedge from the hub's selection.
///
/// Sole writer of a wedge's `TextColor`, which is why `wedge` ships no `Ink`.
pub(crate) fn paint_wedges(
    theme: Res<Theme>,
    hubs: Query<&Radial>,
    wedges: Query<(&Wedge, &ChildOf, Has<Spent>, &Children)>,
    mut inks: Query<&mut TextColor>,
    changed: Query<(), Or<(Changed<Radial>, Added<Wedge>)>>,
) {
    if !theme.is_changed() && changed.is_empty() {
        return;
    }

    for (Wedge(index), parent, spent, children) in &wedges {
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
        for &child in children {
            if let Ok(mut colour) = inks.get_mut(child) {
                colour.0 = theme.color(role);
            }
        }
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

// ---------------------------------------------------------------------------
// The wheel
// ---------------------------------------------------------------------------

/// Marks the ring drawn under a hub's wedges.
#[derive(Component)]
pub struct RadialDisc;

/// Marks the single slice that lights up under the chosen wedge.
#[derive(Component)]
pub struct RadialHighlight;

/// Wheel art, generated once per wedge count and kept.
///
/// Drawn rather than shipped, for the same reason Ordo ships no palette: a menu with
/// three wedges and one with six want different art, and shipping PNGs would either
/// dictate the count or need a file per count. These are alpha masks, so the theme
/// tints them and the wheel follows the palette like everything else.
#[derive(Resource, Default)]
pub struct RadialArt {
    rings: HashMap<usize, Handle<Image>>,
    slices: HashMap<usize, Handle<Image>>,
}

/// Resolution of the generated art. It is scaled to whatever the radius metric says, so
/// this only decides how crisp the edges are.
const ART_SIZE: u32 = 256;
/// The ring straddles the wedge radius, so labels sit on it rather than beside it.
const RING_OUTER: f32 = 1.46;
const RING_INNER: f32 = 0.34;
/// The wheel sits over a live world, so it is always translucent regardless of how
/// opaque the role it borrows happens to be — a menu that hid what you were pointing at
/// would be working against itself.
const RING_ALPHA: f32 = 0.62;
const HIGHLIGHT_ALPHA: f32 = 1.0;
/// Gap between slices, as a fraction of one slice.
const SLICE_GAP: f32 = 0.03;
/// Thickness of the highlight rim, as a fraction of the outer radius.
const RIM_THICKNESS: f32 = 0.11;

/// The label box, as a multiple of the radius and of the body text size.
const LABEL_BOX_W: f32 = 1.05;
const LABEL_BOX_H: f32 = 2.0;
/// How much the pointed-at slice grows. Small on purpose: enough to feel like the wheel
/// answered, not enough to move the label out from under the cursor.
const HIGHLIGHT_GROW: f32 = 1.04;

/// Which piece of the wheel a mask draws.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    /// The whole band, cut into every slice — the wheel itself.
    Ring,
    /// A thin arc along the outer edge of one slice, pointing up.
    ///
    /// The highlight used to be the whole filled slice, which buried its own label: at
    /// any alpha strong enough to read as "this one", the text underneath was gone. A rim
    /// says the same thing at the edge of the slice, where there is nothing to cover.
    Rim,
}

/// One alpha mask, for either piece.
///
/// Bucketed with the same measured-from-up, clockwise convention as [`Radial::pick`], so
/// the slice that lights up is always the one being pointed at.
fn ring_mask(count: usize, part: Part) -> Image {
    let n = count.max(1) as f32;
    let step = core::f32::consts::TAU / n;
    let half = ART_SIZE as f32 * 0.5;
    let outer = half;
    let inner = half * (RING_INNER / RING_OUTER);

    let mut data = vec![0u8; (ART_SIZE * ART_SIZE * 4) as usize];

    for py in 0..ART_SIZE {
        for px in 0..ART_SIZE {
            let dx = px as f32 + 0.5 - half;
            let dy = py as f32 + 0.5 - half;
            let r = (dx * dx + dy * dy).sqrt();

            // Feathered edges — a hard-edged circle at this size reads as a jaggy polygon.
            let edge = 1.5;
            // The rim occupies only the outermost sliver of the band.
            let band_inner = match part {
                Part::Ring => inner,
                Part::Rim => outer - half * RIM_THICKNESS,
            };
            let mut alpha =
                ((outer - r) / edge).clamp(0.0, 1.0) * ((r - band_inner) / edge).clamp(0.0, 1.0);

            if alpha > 0.0 {
                let slot = (dy.atan2(dx) + core::f32::consts::FRAC_PI_2) / step;
                let index = slot.round().rem_euclid(n);
                // Distance from this slice's centre line, as a fraction of a slice.
                if (slot - slot.round()).abs() > 0.5 - SLICE_GAP {
                    alpha = 0.0;
                } else if part == Part::Rim && index != 0.0 {
                    alpha = 0.0;
                }
            }

            let i = ((py * ART_SIZE + px) * 4) as usize;
            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = (alpha * 255.0) as u8;
        }
    }

    Image::new(
        Extent3d { width: ART_SIZE, height: ART_SIZE, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Give every fresh hub its ring and its highlight.
pub(crate) fn dress_radials(
    mut commands: Commands,
    mut art: ResMut<RadialArt>,
    mut images: ResMut<Assets<Image>>,
    theme: Res<Theme>,
    fresh: Query<(Entity, &Radial), Added<Radial>>,
) {
    for (hub, radial) in &fresh {
        let count = radial.count.max(1);
        let ring = art
            .rings
            .entry(count)
            .or_insert_with(|| images.add(ring_mask(count, Part::Ring)))
            .clone();
        let slice = art
            .slices
            .entry(count)
            .or_insert_with(|| images.add(ring_mask(count, Part::Rim)))
            .clone();

        let radius = theme.metric(Metric::RadialRadius);
        let size = radius * RING_OUTER * 2.0;
        let corner = -radius * RING_OUTER;

        let plate = |image: Handle<Image>, tint: Color| {
            (
                ImageNode::new(image).with_color(tint),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(corner),
                    top: px(corner),
                    width: px(size),
                    height: px(size),
                    ..default()
                },
                // Behind the labels: siblings otherwise draw in spawn order, and the art
                // is attached after the game has already added its wedges.
                ZIndex(-1),
            )
        };

        let fade = |role: Role, alpha: f32| theme.color(role).with_alpha(alpha);

        commands.spawn((
            RadialDisc,
            plate(ring, fade(Role::CardBg, RING_ALPHA)),
            ChildOf(hub),
        ));
        commands.spawn((
            RadialHighlight,
            plate(slice, fade(Role::Accent, HIGHLIGHT_ALPHA)),
            Visibility::Hidden,
            ChildOf(hub),
        ));
    }
}

/// Point the lit slice at whatever the hand is pointing at, and grow it a little so the
/// wheel visibly answers.
pub(crate) fn aim_highlight(
    hubs: Query<&Radial>,
    mut highlights: Query<(&ChildOf, &mut UiTransform, &mut Visibility), With<RadialHighlight>>,
) {
    for (parent, mut transform, mut visibility) in &mut highlights {
        let Ok(hub) = hubs.get(parent.parent()) else {
            continue;
        };
        match hub.selected {
            Some(index) => {
                let step = core::f32::consts::TAU / hub.count.max(1) as f32;
                // The mask points up and UiTransform rotates clockwise — the same
                // direction the wedges are numbered in.
                transform.rotation = Rot2::radians(index as f32 * step);
                // Scaled about the hub, so the slice grows outward from the centre.
                transform.scale = Vec2::splat(HIGHLIGHT_GROW);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
