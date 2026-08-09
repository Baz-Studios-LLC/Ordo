//! The one path unit tests cannot reach: a file on disk, through the asset
//! loader, through ramp resolution, into `Theme`, and out onto a node's
//! `BackgroundColor`. Headless, so it runs anywhere.

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use ordo::Fill;
use ordo::prelude::*;

#[test]
fn a_theme_file_reaches_a_nodes_colour() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(OrdoPlugin::with_theme("theme.ordo.toml"));

    // A ramp that could not be mistaken for the built-in default accent, which
    // is a cool blue. If the file never arrives, this never turns up.
    app.world_mut()
        .resource_mut::<Ramps>()
        .register("cloth_gold", |_| Color::srgb(1.0, 0.0, 0.0));

    let swatch = app
        .world_mut()
        .spawn((Fill(Role::Accent), BackgroundColor(Color::NONE)))
        .id();

    // Asset loading is asynchronous, so pump until it lands rather than
    // assuming a fixed number of frames.
    let mut arrived = false;
    for _ in 0..400 {
        app.update();
        let accent = app.world().resource::<Theme>().color(Role::Accent);
        if accent.to_srgba().red == 1.0 {
            arrived = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        arrived,
        "theme.ordo.toml never reached Theme — check the loader's extension \
         matching and that the asset root resolves to ./assets"
    );

    // One more turn, so the repaint pass certainly runs after the change.
    app.update();

    let painted = app
        .world()
        .entity(swatch)
        .get::<BackgroundColor>()
        .expect("swatch kept its BackgroundColor");
    assert_eq!(
        painted.0.to_srgba().red,
        1.0,
        "the repaint pass did not carry the resolved role onto the node"
    );
}

/// The metrics half of the same journey.
///
/// The shipped file agrees with the built-in defaults on every metric — it is
/// meant to read as a sensible starting point, not as a contrast — so simply
/// asserting `112.0` would pass with no file at all. The value is poisoned
/// before the first frame instead, and only an arriving file can put it right.
#[test]
fn a_stated_metric_overrides_what_is_already_there() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(OrdoPlugin::with_theme("theme.ordo.toml"));

    app.world_mut()
        .resource_mut::<Theme>()
        .set_metric(Metric::LabelWidth, -1.0);

    let mut arrived = false;
    for _ in 0..400 {
        app.update();
        if app.world().resource::<Theme>().metric(Metric::LabelWidth) == 112.0 {
            arrived = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        arrived,
        "metrics never arrived from theme.ordo.toml — label_width stayed poisoned"
    );
}

/// A notice posted before a shelf exists must wait, not vanish. Games post
/// during loading, and a dropped notice is impossible to notice being dropped.
#[test]
fn notices_wait_for_a_shelf_rather_than_being_dropped() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(OrdoPlugin::new());

    app.world_mut().resource_mut::<Notices>().say("held");
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<Notices>().pending(),
        1,
        "the notice should still be queued with no shelf on screen"
    );

    app.world_mut().spawn(toast_shelf(Anchor::TopRight));
    app.update();
    app.update();

    assert_eq!(app.world().resource::<Notices>().pending(), 0);
    assert_eq!(count::<ordo::Toast>(&mut app), 1);
}

/// Eviction lives in its own system for a reason: spawns are deferred, so the
/// toasts are not among the shelf's children until the frame after they are
/// posted — at which point the queue is empty. A cap checked inside
/// `show_notices` would return early and never fire.
#[test]
fn the_shelf_evicts_down_to_its_cap() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(OrdoPlugin::new());

    app.world_mut().spawn(toast_shelf(Anchor::TopRight));
    {
        let mut notices = app.world_mut().resource_mut::<Notices>();
        for i in 0..(ordo::overlay::TOAST_CAP + 4) {
            notices.say(format!("notice {i}"));
        }
    }

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        count::<ordo::Toast>(&mut app),
        ordo::overlay::TOAST_CAP,
        "the shelf should have evicted the oldest down to the cap"
    );
}

fn count<C: Component>(app: &mut App) -> usize {
    let mut query = app.world_mut().query_filtered::<Entity, With<C>>();
    query.iter(app.world()).count()
}
