# Ordo

A small UI kit for Bevy games. Mine, for my games.

This is not a general-purpose library and it is deliberately not trying to
become one. It exists because the same pieces got hand-written three separate
times — in Divus Factus, in Flat Earth Simulator, in wriftheart — and writing
them a fourth time is the only thing worth avoiding here.

Because it serves known games rather than strangers, it can make choices a
public library cannot: one Bevy version, one layout idiom, no semver anxiety,
no deprecation cycle, and freedom to break between games because every game
pins its own rev.

## The two ideas

**The game owns the palette.** Ordo names *roles* — `PanelBg`, `CardBorder`,
`InkDim` — and the game says what colour those are. Divus Factus tints its
panels from the very ramps its villagers' clothes are dyed from; a kit that
shipped its own palette would force it to keep two sets of colours in step by
hand forever. So Ordo owns the vocabulary and the game supplies the pigment.

**Nothing is painted at spawn.** A widget carries tags saying which role and
which metric it follows, and a pass fills them in afterwards. That indirection
is what lets the theme file be edited with the game running — which is the
single largest difference between using this and hand-rolling it again.

## Quickstart

```rust
use bevy::prelude::*;
use ordo::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(OrdoPlugin::with_theme("theme.ordo.toml"))
        .add_systems(Startup, build)
        .run();
}

fn build(mut commands: Commands) {
    commands.spawn((
        panel(Anchor::TopLeft, Some(260.0)),
        children![
            heading("The Village"),
            (row(), children![label("Believers"), body("1,204")]),
            button("Dismiss"),
        ],
    ));
}
```

```bash
cargo run --example slice
```

Then edit `assets/theme.ordo.toml` and save. Colours, text sizes, padding,
corner radius and the label column all follow, without a rebuild.

## Seeing all of it

```bash
cargo run --example gallery
```

Every colour role gets a swatch, every metric gets a live readout, and all four
anchors are occupied. If a theme edit does nothing, the gallery is where that
is obvious — a swatch that will not change or a number that will not move
points straight at the role or metric that is not wired up.

## Tests

```bash
cargo test
```

Unit tests cover the theme schema: that the shipped file parses, that a typo in
it is a hard error, that a partial file inherits the rest, and that ramps
resolve through the game's own closure. `tests/theme_reaches_the_screen.rs`
covers the part unit tests cannot — a file on disk, through the loader, through
ramp resolution, into `Theme`, and out onto a node's `BackgroundColor`, headless.

Note that the shipped theme file agrees with the built-in defaults on every
metric, deliberately: it is a sensible starting point, not a contrast. That
makes it easy to write a test that passes with no file loaded at all, so the
integration test poisons the value first and requires the file to put it right.

## Lending Ordo a palette

A ramp is registered as the game's *own* sampling function, not as a list of
stops, so the theme file resolves through exactly the code the rest of the game
uses. There is no second interpolation to drift out of step with the first.

```rust
fn lend_ramps(mut ramps: ResMut<Ramps>) {
    ramps.register("cloth_gold", |t| palette::shade(&palette::CLOTH_GOLD, t));
}
```

```toml
accent = { ramp = "cloth_gold", shade = 0.85 }
```

Those shade factors are the numbers actually worth fiddling with, and here they
cost a save rather than a rebuild.

## Type

Ordo ships no typeface, for the same reason it ships no palette. It names three
roles — `Display`, `DisplayBold`, `Body` — which is Divus Factus's own set
(Cinzel, CinzelDecorative-Bold, EB Garamond), and the game says what they are.

Bevy 0.19 runs text through Parley, so a face can be named three ways:

```toml
[font]
display      = { path = "fonts/Cinzel.ttf" }              # an asset you ship
display_bold = { family = "EB Garamond", italic = true }  # from the font database
body         = { generic = "sans-serif", weight = 400 }   # let the platform decide
```

A role you leave out is simply not set, and Bevy's embedded default stands —
which is what a game with no opinion about type wants. Flat Earth Simulator
ships no fonts at all and looks no worse for it.

## Consuming it

Git dependency, **pinned per game**:

```toml
ordo = { git = "…/Ordo", rev = "abc1234" }
```

Not a path dependency. With a path dep, improving Ordo for the game in front of
you silently changes a game you shipped last year. With a pinned rev, each game
upgrades when it chooses — which is the whole difference between a kit that
stays useful across projects and one that becomes a liability the moment there
are two live games.

## Scope

**In, and proven in two or more games:** theme roles and metrics, hot-reloaded
theme file, `panel` / `card` / `backdrop`, `row` / `label`, `button`, text
roles, `Layer`, `Opacity` / `Lifetime`, toasts, tooltips.

**Next, proven in Divus Factus:** `window` (drag, z-order focus, close,
scroll), `bar` / gauge, a real tween beyond `Lifetime`, and a **binding** —
the shared prerequisite for sliders, checkboxes and any data-driven layout.

**Then, cheap because upstream ships the behaviour:** slider, checkbox, radio
and dropdown, skinned over `bevy_ui_widgets`. Icon slots are the exception —
genuinely Ordo's own work, and substrate for toolbars and inventory cells.

**Deferred until a second game asks:** settings rows, keybind capture, tabs,
dialog, list navigation, a reopenable notification history.

**Out:** sprite-canvas UI. wriftheart draws its interface as sprites at explicit
coordinates through a `Pen`, which is a different rendering model; supporting
both would roughly double the design for one consumer. Its `ListNav` logic is
worth harvesting later as render-agnostic code, but the drawing is not.

**Also out, on evidence:** nine-slice and `UiMaterial`. No game here uses them —
the look is solid fills and hairline borders — so there is no custom shader
work and `bevy_ui` is sufficient.

## The rule

Only add what a game needs *right now*, on the game's schedule. Never a
"library week". The failure mode for a kit like this is quietly becoming the
project while the games stop moving.
