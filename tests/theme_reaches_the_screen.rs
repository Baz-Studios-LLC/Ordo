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

/// Tabs, end to end: the open one is marked, its pane is the only one showing, and both
/// swap when the selection moves.
///
/// The swap is the half worth testing. Closing a tab *removes* a component, which no change
/// filter can see, so a naive painter leaves the tab you left still wearing the pressed face
/// while the one you opened takes one too — two open tabs, which is not a state that exists.
#[test]
fn opening_a_tab_shows_its_pane_and_closes_the_last_one() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(OrdoPlugin::new());

    let strip = app.world_mut().spawn(tab_strip()).id();
    let video = app.world_mut().spawn((tab("Video", 0), ChildOf(strip))).id();
    let audio = app.world_mut().spawn((tab("Audio", 1), ChildOf(strip))).id();
    let video_pane = app.world_mut().spawn(pane(strip, 0)).id();
    let audio_pane = app.world_mut().spawn(pane(strip, 1)).id();
    app.update();

    let open = |app: &App, e: Entity| app.world().entity(e).contains::<Selected>();
    let shown = |app: &App, e: Entity| {
        *app.world().entity(e).get::<Visibility>().expect("a pane keeps its Visibility")
            != Visibility::Hidden
    };

    assert!(open(&app, video), "a strip should open on its first tab");
    assert!(!open(&app, audio));
    assert!(shown(&app, video_pane) && !shown(&app, audio_pane));

    app.world_mut().get_mut::<Tabs>(strip).unwrap().selected = 1;
    app.update();

    assert!(!open(&app, video), "the tab left behind is still marked open");
    assert!(open(&app, audio));
    assert!(shown(&app, audio_pane) && !shown(&app, video_pane));
}

/// A button's chrome is the one colour painted outside the repaint pass, because
/// it depends on what the pointer is doing. That makes it the one colour that
/// can forget [`Opacity`] — and it did, so a game fading a menu out faded the
/// labels and left the boxes behind them at full strength.
///
/// Worth an integration test rather than a unit one: the bug was as much about
/// *when* `paint_buttons` runs as what it writes. It repaints only buttons the
/// pointer has touched, so a changed opacity that isn't in its trigger produces
/// a button that fades the moment you wave the mouse at it and not before.
#[test]
fn a_faded_button_fades_its_chrome_and_not_only_its_label() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(OrdoPlugin::new());

    let button = app.world_mut().spawn(button("Continue")).id();
    app.update();

    let opaque = app
        .world()
        .entity(button)
        .get::<BackgroundColor>()
        .expect("the button kept its BackgroundColor")
        .0
        .alpha();
    assert!(opaque > 0.0, "an untouched button should still get painted");

    // Half faded, and the pointer never goes near it.
    app.world_mut().entity_mut(button).insert(Opacity(0.5));
    app.update();

    let faded = app.world().entity(button).get::<BackgroundColor>().unwrap().0.alpha();
    assert!(
        (faded - opaque * 0.5).abs() < 0.001,
        "the fill should be scaled to {} but is {faded}",
        opaque * 0.5,
    );

    let edge = app.world().entity(button).get::<BorderColor>().unwrap();
    assert!(
        edge.top.alpha() < 1.0,
        "the border should have faded with the fill, and is at {}",
        edge.top.alpha(),
    );
}
