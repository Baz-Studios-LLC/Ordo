//! The vertical slice: a panel, a heading, labelled rows, a well, a button.
//!
//! Run it, then edit `assets/theme.ordo.toml` and save. The window answers
//! without a rebuild — colours, text sizes, padding, corner radius and the
//! label column all follow. That is the whole argument for the kit.
//!
//!     cargo run --example slice

use bevy::prelude::*;
use ordo::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(OrdoPlugin::with_theme("theme.ordo.toml"))
        .add_systems(Startup, (lend_ramps, build))
        .run();
}

/// Stands in for a game's own palette. Divus Factus would hand over the real
/// thing here — `|t| palette::shade(&palette::CLOTH_GOLD, t)` — so the theme
/// file resolves through exactly the code the villagers' clothes go through,
/// with no second interpolation to drift out of step with the first.
fn lend_ramps(mut ramps: ResMut<Ramps>) {
    ramps.register("cloth_gold", |t| {
        Color::srgb(0.10 + 0.85 * t, 0.07 + 0.66 * t, 0.03 + 0.28 * t)
    });
}

fn build(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        panel(Anchor::TopLeft, Some(260.0)),
        children![
            heading("The Village"),
            stat("Believers", "1,204"),
            stat("Offerings", "37 this week"),
            stat("Rainfall", "light"),
            (
                card(),
                children![
                    dim("A well reads as its own surface."),
                    stat("Mood", "content")
                ]
            ),
            button("Dismiss"),
        ],
    ));

    commands.spawn((
        panel(Anchor::BottomRight, None),
        children![dim("Edit assets/theme.ordo.toml and save.")],
    ));
}

/// A labelled row: fixed label column, reading on the right.
fn stat(name: &str, reading: &str) -> impl Bundle {
    (row(), children![label(name), body(reading)])
}
