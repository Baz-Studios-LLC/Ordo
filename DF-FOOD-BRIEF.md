# Brief: the food loop

For the Divus Factus thread. This was worked out in the **Ordo** thread from
screenshots, so none of it is in your history — that's the only reason it's
written down. Everything is anchored to `file:line` so you can verify rather
than trust; I was reading DF cold and got at least one thing wrong on the way
(I claimed drop-absorption didn't exist; it does).

---

## What started it

A screenshot: **33 villagers praying for food beside a granary reading 165
food**, panel saying "food: holding steady". The prayer notices flooded the
whole right edge of the screen.

## What is actually happening

The prayers are **truthful, not a UI bug**. `belief.rs` has:

```rust
let dying_regardless = needs.hunger >= 0.92;
```

which fires even when the larder reads full, deliberately — its comment says
*"Six villagers died in VisitingStore at hunger 1.00 with the larder reading
full, and not one prayer rose."* So those 33 souls really are at ≥ 0.92 with
food in town.

**Hypothesis, unconfirmed.** `work/stores.rs:82` requires a villager to get
within **4.0 units** of `table` to actually eat, and that radius doesn't scale
with how many are queuing. Thirty-three converging villagers cannot fit inside
a 4-unit circle; they jostle outside it, never trigger the meal, climb past
0.92, and all pray. The tight pile in the middle of the screenshot is the
signature, and it also explains the column of prayer bubbles — they're all
standing in the same spot, so the bubbles de-overlap upward.

Verify by watching `hunger` and `Activity` on a few souls through a mass meal
before building anything on top of it.

---

## Order of work

### 0. Eating must never deadlock — the live bug

Express the eat decision as a total order whose last branch always succeeds, so
no configuration of the world can leave someone starving beside food. This
depends on nothing else and should land first.

There are hunger fixtures at `belief.rs:1692` onward; the assertion that stops
this being reinvented a third time belongs beside them — *a hungry villager
with food anywhere in reach always finds a way to eat*.

### 1. `Home(Entity)` on the villager

Doesn't exist today. `BedSlot(pub u8)` is a bare slot number and `MemberOf`
gives the **town**, not the building; the house is resolved by querying
`(&ChildOf, &Bed)` and matching slots (`home.rs:301–345`).

Small, unblocks items 4 and 5, and probably simplifies that existing
resolution.

### 2. Larder capacity

`Stockpile { larder: Larder }` (`stores.rs:224`) has `food()` but no cap I could
find. Two separate features below need something for "full" to mean — the drop
behaviour and the taker chain's fall-through — so this is a prerequisite twice
over.

House capacity from bed count via `shelter_capacity` (`home.rs`) gets you "a
longhouse holds more than a hut" for free.

### 3. Storehouse and granary as offering takers

`receive_offerings` (`work.rs:308`) already resolves a `taker` as a **preference
chain**: bonfire (`ONTO_THE_FLAMES`) → construction site (`OFFERING_REACH`) →
town. The town branch tests:

```rust
ground.woodpile.distance(here) < OFFERING_REACH
    || ground.foodpile.distance(here) < OFFERING_REACH
```

Those are bare **ground coordinates** on `SettlementGround`. The storehouse
*building* was never a target — which is exactly why dropping on the flag or a
half-built house works and dropping on the storehouse does nothing.

**Open question, possibly the whole bug:** `stores.rs` has a `Rehouse` pass that
moves piles indoors once there's a roof (`stores.rs:971`). Does
`ground.foodpile` follow it? If not, the spot that accepts an offering drifts
away from the sacks the player can see, which would make this feel arbitrary
rather than merely missing.

**Question already resolved:** the pickup comment at `hand/mod.rs:957` about
*"paying back exactly what was drawn when offered"* **is** honoured — the
`Goods` parcel arm in `receive_offerings` pays back exact kind and amount, and
deliberately runs before the generic `Matter` arm that used to misprice clay as
masonry.

### 4. Household larders

Put `Stockpile` on house entities. The component already exists and works; it
simply only ever lives on the settlement today.

This is the structural fix rather than a mitigation, because it **scales with
population**: more villagers means more houses means more places to eat. Seat
counts and granary slots are fixed numbers you re-tune against a growing town;
this isn't.

- **Save.** `save.rs` reads and writes `Stockpile` off `settlement_entity` by
  name (`416, 419, 429, 432, 1070`). Per-house larders need persistence added or
  households silently reset to empty on load.
- **A hauling errand**, granary → house. That is also what makes a household run
  *out* in an interesting way: nobody free to fetch.
- Then insert "the house you dropped it on" into item 3's chain, **ahead of the
  town**. That's the "unless they're full" behaviour expressed as chain order
  rather than a special case.
- **UI.** `ui/town.rs:151,241` already reads `Stockpile` for the panel — natural
  place to show "165 in the granary, 80 in homes".
- **The payoff worth building it for.** `miracles.rs:1290,1398` and
  `hand/mod.rs:927` write the *town* `Stockpile`, so divine generosity is a
  town-wide number going up. Per-house larders let you feed **one family**, and
  `belief.rs` opens by calling itself "the game's thesis made mechanical".

### 5. Ownership as a preference chain, not a gate

Only residents eat from their own house — but a locked larder is the original
bug wearing a hat, and it has obvious victims: travellers, colonists on the
road, an orphan, anyone whose house burned. So:

1. Own house larder
2. The commons — tavern, then granary
3. A neighbour's charity, if some household has surplus
4. Past death's door, take it anyway

Both valves feed machinery that already exists: `gossip.rs:1559` has a quarrel
that "knows which of them is wronged", and `colony.rs:173`
`fractured(grudges, boldness)` splits towns. So famine theft isn't a special
case to write — the household resents the thief, gossip carries it, enough
grudges fracture the settlement.

Charity is the more interesting half, and the same cost: it gives generosity a
village-level face mirroring what the god does at town level, so a well-fed
village visibly *behaves* differently rather than just having a bigger number.

### 6. Seats, then the tavern

`Tavern` already exists as a kind (`buildings.rs:59`) but is scored purely on
morale — `buildings.rs:533`, `Tavern => (0.75 - needs.avg_spirits).max(0.0) *
2.2` — and has nothing to do with food.

Claim-and-release seating for people **already exists** for beds: `Bed { slot,
double }` as a physical child of the building, `BedSlot(u8)` on the occupant,
and the genuinely fiddly half — whether someone should give up the bed they
have — at `home.rs:100`. Seats would be the third consumer of that pattern, so
it's probably worth lifting `Bed`/`BedSlot` into a generic claimable slot rather
than writing `Seat`/`Seated` alongside it. Work stations and pews want the same
thing later.

Then the granary gets a few eat slots and the tavern gets many: one mechanism,
different counts. Add a hunger term to that want-score so a *hungry* village
wants one, not only a cheerless one.

Note that slots alone convert a jostling deadlock into an orderly one — twelve
seats and thirty-three hungry villagers means they queue politely and starve on
schedule. Item 0's fallback is what keeps that safe, and it's also what makes
the tavern a genuine upgrade rather than a prerequisite: a spring-1 village eats
badly in the mud but survives, and building the tavern visibly improves it.

---

## Why this order

- **0 first** — it's the live bug and depends on nothing.
- **1 and 2** are both small and each unblocks two later items.
- **3** needs neither 1 nor 2, and it's the most immediately satisfying thing on
  the list, so it's cheap to land early.
- **4 before 5** — you can't write ownership rules before there's something owned.
- **6 last** — only worth it once eating is decentralised, or you'll be tuning
  seat counts against a growing town forever.
