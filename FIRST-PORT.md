# Brief: first port of Ordo into Divus Factus

For whoever is working in Divus Factus. Ordo is a UI kit at `../Ordo`, extracted
from the UI code DF, Flat Earth Simulator and wriftheart each wrote separately.
It has never been used by a real game. This is that first contact, and the point
is to find out what is wrong with it — not to convert DF's interface.

**Scope: one panel, built alongside the existing one, not replacing it.** Put the
two side by side and compare. If Ordo's design is broken, one panel is a cheap
way to find out.

---

## Setup

```toml
# Divus Factus/Cargo.toml
ordo = { path = "../Ordo" }
```

A path dependency on purpose. Ordo's own README argues for a pinned git rev, and
that is right once a game ships against it — it stops a library change from
silently altering a released game. During a spike where both move together,
a path is what you want. Switch when this stops being a spike.

Ordo's manifest enables `bevy_ui_widgets` and `ui_picking`. Cargo unifies
features, so DF's `bevy` picks these up with no change to DF's own dependency
line.

```rust
app.add_plugins(ordo::OrdoPlugin::with_theme("theme.ordo.toml"));
```

The theme file lives at `Divus Factus/assets/theme.ordo.toml`. It hot-reloads,
which needs bevy's `file_watcher` feature — worth turning on for the duration.

## Lend Ordo the palette

Ordo names colour *roles* and never ships colours. DF's UI is tinted from the
same ramps its villagers' clothes are dyed from, and duplicating those into a
kit would mean keeping two sets in step by hand forever. So DF hands over its
own sampling function:

```rust
fn lend_ramps(mut ramps: ResMut<ordo::Ramps>) {
    ramps.register("cloth_gold", |t| palette::shade(&palette::CLOTH_GOLD, t));
    ramps.register("bone",       |t| palette::shade(&palette::BONE, t));
}
```

Add it to `Startup`. Order does not matter — Ordo re-resolves the theme whenever
`Ramps` changes, so registering after the file has loaded is fine.

**Note the quantisation.** `palette::shade` rounds to one of five stops, so a
`shade` value in the theme file has only five distinct outcomes (0.0, 0.25, 0.5,
0.75, 1.0). That is correct — it is what DF's UI does today — but it makes the
"tune the shade live" idea much less useful than it sounds. If you want a
continuous knob, register a second name against `shade_smooth`:

```rust
ramps.register("cloth_gold_smooth", |t| palette::shade_smooth(&palette::CLOTH_GOLD, t));
```

## The theme file

This is DF's current `ui::theme` module translated one-for-one. It should
reproduce today's appearance exactly. Drop it in and adjust from there.

```toml
[color]
panel_bg     = { srgb = [0.045, 0.050, 0.062], alpha = 0.985 }
title_bg     = { srgb = [0.075, 0.082, 0.102], alpha = 0.990 }
card_bg      = { srgb = [0.058, 0.062, 0.078] }
card_border  = { ramp = "cloth_gold", shade = 0.60, alpha = 0.55 }
panel_border = { ramp = "cloth_gold", shade = 0.55, alpha = 0.35 }
ink          = { ramp = "bone", shade = 0.97 }
ink_dim      = { ramp = "bone", shade = 0.78 }
accent       = { ramp = "cloth_gold", shade = 0.85 }

# DF's theme module has no equivalents for these four. Ordo's cool-slate
# defaults will stand unless you give them gold ones, which is probably what
# a gold interface wants.
# scrim          = { srgb = [0.020, 0.030, 0.050], alpha = 0.720 }
# button_idle    = { ... }
# button_hover   = { ... }
# button_pressed = { ... }

[font]
display      = { path = "fonts/Cinzel.ttf" }
display_bold = { path = "fonts/CinzelDecorative-Bold.ttf" }
body         = { path = "fonts/EBGaramond.ttf" }

[metric]
title_size  = 13.0
body_size   = 13.0
small_size  = 12.0
pad         = 12.0
gap         = 5.0
margin      = 10.0
label_width = 112.0
row_height  = 22.0
border      = 1.0
radius      = 5.0
```

An unknown key is a hard error rather than a silent no-op, so a typo will tell
you rather than waste twenty minutes.

## What Ordo gives you

Nothing reads the theme at spawn time. A widget carries *tags* saying which role
and metric it follows, and a repaint pass fills them in — that indirection is
why the file can be edited with the game running.

```rust
use ordo::prelude::*;

panel(Anchor::TopLeft, Some(260.0))   // anchored frame, fill + border + padding
card()                                 // a well: second material inside a panel
backdrop()                             // full-screen scrim, Layer::Modal
row()                                  // horizontal line, at least RowHeight tall
label("Believers")                     // fixed-width label column
heading("The Village")                 // accent ink, Display face, TitleSize
body("1,204")                          // Ink, Body face, BodySize
dim("light")                           // InkDim, Body face, SmallSize
button("Dismiss")                      // bevy_ui_widgets::Button, themed states
toast_shelf(Anchor::BottomRight)       // spawn once; notices stack into it
Tooltip::new("Title", "A line.")       // add to any node; requires nothing else
```

Post a notice from anywhere:

```rust
notices.push(Notice::fanfare("A shrine is finished."));
```

Tag your own nodes to join the repaint pass — this is how DF's bespoke pieces
keep in step with the theme without becoming Ordo widgets:

```rust
(Node { .. }, BackgroundColor(Color::NONE), Fill(Role::CardBg), Edge(Role::CardBorder))
(Text::new(".."), TextSize(Metric::BodySize), Face(FontRole::Display), Ink(Role::Accent))
```

DF's existing `DisplayBoldFace` and `SerifFace` markers map straight onto
`Face(FontRole::DisplayBold)` and `Face(FontRole::Body)` — DF already invented
this system, which is a good sign for the design.

## Known gaps — do not spend time discovering these

- **No binding.** A value that changes still needs a hand-written system, exactly
  as `update_date_card` does today. This is the next thing to build and the port
  should inform its shape.
- **No gauge or bar.** DF's `Gauge` has no equivalent yet.
- **No window system.** No drag, z-order focus, close or scroll. DF's is far
  ahead of Ordo here; it is the largest single thing still to lift.
- **No icons or nine-slice.**
- **The metric set is a closed enum** — `title_size`, `body_size`, `small_size`
  and the spacing values, nothing else. DF's 36px date text has no home in it.
  Use raw `TextFont` for one-offs and tell me about it.
- **Tooltip placement follows the cursor** and flips near an edge, rather than
  using `bevy_ui_widgets::Popover`. Good enough; not final.
- **`Anchored` only does the four corners**, with one shared `margin`. No
  offsets, no stacking two panels down one edge.

## What to report back

Screenshots side by side with the original, and then:

1. **Does it look identical?** If not, where does it drift — colour, weight,
   spacing, type?
2. **Did lending `shade` as a closure work**, or did the quantisation or the
   sRGB handling produce something unexpected?
3. **Do three font roles cover DF**, or does it need more?
4. **Px or Rem?** Ordo emits `FontSize::Px` throughout. Bevy also offers `Rem`
   against a `RemSize` resource, and `VMin`. At DF's real render resolution, does
   pixel-fixed text hold up, or does the interface need a scale knob? This is
   much cheaper to change now than after the numbers are calibrated.
5. **Did corner `Anchored` fit**, or did you immediately want offsets?
6. **What was missing or awkward** in the first ten minutes. That list is worth
   more than the port itself.
