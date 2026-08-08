# Report: first port of Ordo into Divus Factus

Answering `FIRST-PORT.md`. Written from the Divus Factus side, 2026-08-07.

**Outcome: adopted.** Brett's call after seeing it: "we will use ordo from now
on… We should migrate everything to it. Also if we need to extend it, we should
add to ordo instead of creating our own solution." So this is no longer a spike,
and the rest of this document is written for whoever picks Ordo up next.

---

## What was done

To the brief's scope: one panel in Ordo beside one hand-built the same way every
DF panel is built, same content, same frame. Behind `DIVUS_FACTUS_ORDO=1`, in
`src/debug/ordo_trial.rs`, touching nothing else.

Then, past the brief's scope and because of the decision above, **the window
system was lifted into Ordo** — see "What changed in Ordo" below.

Setup was exactly as written and took minutes: path dependency, theme file,
`lend_ramps`, `OrdoPlugin::with_theme`. Cargo's feature unification did what the
brief promised; DF's own bevy line needed only `file_watcher`. No unknown-key
error fired, so every key in the brief's theme file is real.

## The six questions

**1. Does it look identical?** *Unanswered.* I cannot see the screen, and I am
not going to claim a visual match I have not looked at. Brett has both panels in
one frame. What I can say is that no value was silently dropped on the way in.

**2. Did lending `shade` as a closure work?** Yes, mechanically — and the
quantisation is worse than the brief suggests. `palette::shade` snaps to five
stops, so in the theme file as written:

```toml
card_border  = { ramp = "cloth_gold", shade = 0.60, alpha = 0.55 }
panel_border = { ramp = "cloth_gold", shade = 0.55, alpha = 0.35 }
```

`0.60` and `0.55` are **the same colour**. Those two roles differ only in alpha,
which is not what the file looks like it says. Registering a `_smooth` twin works,
but the sampler is chosen by ramp *name*, so every ramp needs two entries and the
theme file has to know which flavour it wants baked into the string. Consider
`{ ramp = "cloth_gold", sampler = "smooth", shade = 0.58 }` instead, so the game
registers one ramp and the file chooses how to walk it.

**3. Do three font roles cover DF?** Yes. DF has exactly three faces. Two notes:
DF's resource fields are `display`/`display_bold`/`text` against Ordo's
`Display`/`DisplayBold`/`Body`, so the brief's `fonts.body` does not compile —
harmless, but the mapping is not quite the one-for-one the brief implies.

**4. Px or Rem?** *Unanswered* — needs eyes on it at real resolution.

**5. Did corner `Anchored` fit?** **No, and it was the first thing I wanted.**
Two reasons, immediately:

- The trial's whole purpose is two panels side by side. Two panels down one edge
  is not expressible, so they went into opposite corners — which is a worse
  comparison than the brief asked for.
- `window()` (new, below) needed one at once. Without it every window in a game
  opens in the same corner on top of the last.

Offsets and stacking are the gap. One shared `margin` and four corners is not
enough for a game with more than four things on screen.

**6. What was missing or awkward in the first ten minutes.**

- **A widget owns its `Node`, so a caller cannot place or nudge it.** Adding a
  `Node` alongside `panel()` or `window()` is a *panic* — "has duplicate
  components" — not a merge. This is the sharpest edge in the kit. Either every
  widget takes the layout it might need as arguments, or there is a documented
  escape hatch. I hit it within a minute of writing the second widget.
- **Bundles plus `children!` give the caller no handle to anything inside.**
  Fine for a panel. Not fine for anything with chrome *and* a body — see the
  window note below for how that was resolved, which may be the general answer.
- **Ordo won the first real disagreement.** The hand-built panel died at
  `Startup` on `Res<Fonts>`, because DF loads fonts a beat later; it now needs
  `run_if(resource_exists::<Fonts>)` and a once-guard. Ordo's widgets did not
  care — a `Face` tag is resolved whenever the font arrives. The central claim
  paid off by killing the *other* panel, which is the best evidence the port
  produced.

## What changed in Ordo

Committed in DF's tree only as a consumer; **the Ordo source changes are
uncommitted in `../Ordo` and need review.**

`src/window.rs`, new. `Window`, `Titled`, `Dressed`, `DragHandle`,
`CloseButton`, `Dragging`, and `window(title, anchor, min_width)`, with
`dress_windows`, `drag_windows`, `close_windows`, `focus_windows`. Registered in
`OrdoPlugin` and the prelude.

**The design decision worth arguing with:** a window is not a bundle. It has
chrome the caller does not build and a body the caller fills, and two `children!`
on one entity is not a thing. So the chrome is **not spawned with it** —
`dress_windows` puts the title bar in as child 0 afterwards. That is the same
move Ordo already makes with paint: a tag says what a thing wants and a pass
gives it. A window carries `Titled` and gets a title bar the way a node carries
`Fill` and gets a colour, and the caller writes only the body:

```rust
commands.spawn((
    window("The Ledger", Anchor::BottomLeft, 300.0),
    children![row_of("Founded", "Spring 1"), row_of("Souls", "14")],
));
```

If that generalises, it is the answer to the "no handle to anything inside"
problem above, and probably how `card()` should grow a header.

`focus_windows` raises by `GlobalZIndex` rather than by re-ordering children, so
a window's place in whatever laid it out is left alone.

**Also changed:** the window systems run behind a `the_pointer_exists`
condition. They want a mouse and a primary window, Ordo's own suite is headless,
and a system asking for a missing resource is an *error*, not a no-op — growing
windows broke four tests that had nothing to do with windows. A kit that cannot
be tested headless is a kit nobody tests. Worth applying to anything else that
reaches for input.

## Still to lift, in the order DF needs them

1. **Binding.** Still the next thing to build, and the port did not change that.
2. **Scroll**, and a warning from DF's own code. Scrolling there needs *two*
   independent things: `Scrollable` + `ScrollPosition`, **and**
   `Overflow::scroll_y()` on the node. DF had one pane with each half and
   neither scrolled — the one with `Scrollable` alone consumed the wheel and sat
   still, which reads as *broken* rather than absent. If Ordo grows scrolling,
   make it one thing that cannot be half-applied.
3. **Gauge.**
4. **A metric for one-offs.** DF's 36px date text still has no home, as the brief
   predicted.

## Housekeeping

`../Ordo` is not a git repository, so the brief's "pin a rev once a game ships
against it" cannot be done yet. DF depends on it by path and is 59 commits ahead
of its own origin. Worth settling before either ships.
