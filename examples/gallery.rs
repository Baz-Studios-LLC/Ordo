//! Everything Ordo currently does, on one screen.
//!
//! Every colour role has a swatch, every metric has a live readout, and all
//! four anchors are occupied. Edit `assets/theme.ordo.toml` and save: colours
//! change, the numbers in the metrics panel change, and so do the padding,
//! border width, corner radius and label column of the panels showing them.
//! Nothing rebuilds.
//!
//!     cargo run --example gallery

use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use ordo::prelude::*;
use ordo::{Edge, Fill};

const ROLES: &[(Role, &str)] = &[
    (Role::PanelBg, "panel_bg"),
    (Role::TitleBg, "title_bg"),
    (Role::CardBg, "card_bg"),
    (Role::CardBorder, "card_border"),
    (Role::PanelBorder, "panel_border"),
    (Role::Scrim, "scrim"),
    (Role::Ink, "ink"),
    (Role::InkDim, "ink_dim"),
    (Role::Accent, "accent"),
    (Role::ButtonIdle, "button_idle"),
    (Role::ButtonHover, "button_hover"),
    (Role::ButtonPressed, "button_pressed"),
];

const METRICS: &[(Metric, &str)] = &[
    (Metric::TitleSize, "title_size"),
    (Metric::BodySize, "body_size"),
    (Metric::SmallSize, "small_size"),
    (Metric::Pad, "pad"),
    (Metric::Gap, "gap"),
    (Metric::Margin, "margin"),
    (Metric::LabelWidth, "label_width"),
    (Metric::RowHeight, "row_height"),
    (Metric::Border, "border"),
    (Metric::Radius, "radius"),
];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(OrdoPlugin::with_theme("theme.ordo.toml"))
        .add_systems(Startup, (lend_ramps, build).chain())
        .add_systems(Update, (arm_buttons, show_metrics))
        .run();
}

/// Stands in for a game's palette. Divus Factus hands over the real thing:
/// `|t| palette::shade(&palette::CLOTH_GOLD, t)`.
fn lend_ramps(mut ramps: ResMut<Ramps>) {
    ramps.register("cloth_gold", |t| {
        Color::srgb(0.10 + 0.85 * t, 0.07 + 0.66 * t, 0.03 + 0.28 * t)
    });
}

fn build(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Top left — every role, so a colour that never changes is obvious.
    commands
        .spawn(panel(Anchor::TopLeft, Some(230.0)))
        .with_children(|panel| {
            panel.spawn(heading("Roles"));
            for (role, name) in ROLES {
                panel.spawn((row(), children![label(name), swatch(*role)]));
            }
        });

    // Top right — every metric, read back live from the resolved theme.
    commands
        .spawn(panel(Anchor::TopRight, Some(230.0)))
        .with_children(|panel| {
            panel.spawn(heading("Metrics"));
            for (metric, name) in METRICS {
                panel.spawn((row(), children![label(name), (body("—"), Readout(*metric))]));
            }
        });

    // Bottom left — the widgets, plus the three text sizes side by side.
    commands
        .spawn(panel(Anchor::BottomLeft, Some(300.0)))
        .with_children(|panel| {
            panel.spawn(heading("Heading, at title_size"));
            panel.spawn(body("Body copy, at body_size."));
            panel.spawn(dim("Dim text, at small_size."));

            panel.spawn((
                card(),
                children![
                    dim("A card is a second material."),
                    (row(), children![label("Nested row"), body("aligns too")]),
                ],
            ));

            panel.spawn((
                row(),
                children![
                    (
                        button("Bless"),
                        Says("Blessed."),
                        Fanfare,
                        Tooltip::new("Bless", "Posts a notice with fanfare — accent ink."),
                    ),
                    (
                        button("Smite"),
                        Says("Smitten."),
                        Tooltip::new("Smite", "Posts a plain notice."),
                    ),
                    (
                        button("Flood"),
                        Says("Six at once."),
                        Flood,
                        Tooltip::new(
                            "Flood",
                            "Posts eight, so the shelf evicts down to its cap of six."
                        ),
                    ),
                ],
            ));

            panel.spawn((body("Hover and click the buttons."), Status));
            panel.spawn((
                dim("Edit assets/theme.ordo.toml and save. Rest on a line to see a hint."),
                Tooltip::new(
                    "Hot reload",
                    "Colours, text sizes, padding, border width, corner radius and the label \
                     column all follow the file. Nothing rebuilds.",
                ),
            ));
        });

    // Bottom right is left clear for the toast shelf — a corner cannot hold a
    // panel and a stack of notices at once without them landing on each other.
    commands.spawn(toast_shelf(Anchor::BottomRight));
}

/// A colour chip. Built here rather than in the kit, which shows the other way
/// in: a game can tag its own nodes with a role and join the repaint pass.
fn swatch(role: Role) -> impl Bundle {
    (
        Node {
            width: px(56.0),
            height: px(13.0),
            border: UiRect::all(px(1.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Fill(role),
        BorderColor::all(Color::NONE),
        Edge(Role::PanelBorder),
    )
}

/// Marks a text node that mirrors one metric's current value.
#[derive(Component)]
struct Readout(Metric);

fn show_metrics(theme: Res<Theme>, mut readouts: Query<(&Readout, &mut Text)>) {
    if !theme.is_changed() {
        return;
    }
    for (readout, mut text) in &mut readouts {
        *text = Text::new(format!("{:.1}", theme.metric(readout.0)));
    }
}

/// What a button says when it is pressed.
#[derive(Component, Clone, Copy)]
struct Says(&'static str);

/// Posts its notice with emphasis.
#[derive(Component)]
struct Fanfare;

/// Posts more notices than the shelf will hold, so eviction is visible.
#[derive(Component)]
struct Flood;

#[derive(Component)]
struct Status;

/// Observers are added after the tree exists, so the buttons are found by their
/// marker rather than threaded back out of the builder.
fn arm_buttons(mut commands: Commands, buttons: Query<Entity, (With<Says>, Added<Says>)>) {
    for entity in &buttons {
        commands.entity(entity).observe(announce);
    }
}

fn announce(
    activated: On<Activate>,
    pressed: Query<(&Says, Has<Fanfare>, Has<Flood>)>,
    mut notices: ResMut<Notices>,
    mut status: Query<&mut Text, With<Status>>,
) {
    let Ok((Says(line), fanfare, flood)) = pressed.get(activated.entity) else {
        return;
    };
    for mut text in &mut status {
        *text = Text::new(*line);
    }

    if flood {
        for i in 1..=8 {
            notices.push(Notice::new(format!("Notice {i} of eight")));
        }
    } else if fanfare {
        notices.push(Notice::fanfare(*line));
    } else {
        notices.push(Notice::new(*line));
    }
}
