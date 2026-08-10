# Auditing every event's state space

## TL;DR

- Went through all ten events in rulebook order and audited the two-player state count for
  each. **1.42e9 → 1.13e9**, and more importantly every number now rests on an enumerated
  reachable set rather than a product of ranges.
- **The diff-axis formula was wrong in both directions.** I capped by the remaining swing
  and *then* widened, instead of widening and then capping — and I widened by the wrong
  quantity. Early events were under-counted (100m by 1.54×), late events over-counted
  (1500m by 1.5×).
- **Every event's min/max was independently confirmed.** Two do not follow "dice × 6":
  110m hurdles has a *floor of 5*, and javelin caps at 6 × **5** = 30 because only odd
  faces freeze.
- **`RULES-CHECKLIST.md` is wrong about skipping heights** under the two-player objective.
  It is dead code under solo EV and becomes live — often dominant — under win probability.
  The rulebook's own pole vault example opens by skipping.
- **Pole vault and high jump store no dice state at all.** Both reduce to a ladder of
  Bernoulli trials with precomputed probabilities; pole vault's entire dice engine is a
  20-float table.
- Long jump is still 61% of the count and still carries the one boxed product left.

---

## Details

### Score ranges

Every event's reachable score range, derived and then independently confirmed:

| event | min | max | span | note |
|---|---:|---:|---:|---|
| 100m | −48 | 40 | 88 | 8 dice; a 6 subtracts 6, else 1-5 |
| Long Jump | 0 | 30 | 30 | foul = 0; the jump uses at most 5 dice |
| Shot Put | 0 | 48 | 48 | foul = 0; a 1 fouls, so survivors are 2-6 |
| High Jump | 0 | 30 | 30 | bar capped where five dice can still reach |
| 400m | −48 | 40 | 88 | same eight dice / six-penalty as 100m |
| 110m Hurdles | **5** | 30 | 25 | no foul rule and all five dice must freeze |
| Discus | 0 | 30 | 30 | foul = 0; even faces, max 6 each |
| Pole Vault | 0 | 48 | 48 | foul = 0; ones-free dice, up to eight |
| Javelin | 0 | **30** | 30 | 6 dice but **only odd freeze**, so max face 5 |
| 1500m | −48 | 40 | 88 | same eight dice / six-penalty |

Sum of spans = **505**, the largest `|d|` reachable after all ten events.

The two exceptions are the ones worth remembering. Hurdles is the only event that cannot
score zero. Javelin has the most dice of any throwing event and still ties discus's
maximum, because a javelin die is worth at most 5.

Reachable score *sets* have holes — 100m is `−48 ∪ [−41,−37] ∪ [−34,40]`, javelin is
`[0,26] ∪ 28 ∪ 30`, discus is even-only — but the **diff sets are 100% dense at every
event boundary**, verified for all ten. Flat arrays indexed by `d`, no sparsity handling.

### The diff axis was wrong in both directions

The axis is the number of distinct `d = my_total − their_total` values a decision node
inside an event can see. I had been computing `d_start + (span − 1)`. Two errors:

**Widened by the wrong quantity.** The largest mid-event contribution is not one player's
partial score; it is what the *trailer* sees, `leader's finished score − trailer's
partial`. For 100m that reaches 68, not 44.

**Widened after capping instead of before.** Once `|d|` exceeds the swing left in the
remaining events, the outcome is decided and the states are dead. Capping `d_start` at
`rem` and then adding the widening pushes the axis back into that dead zone. 1500m has
only 88 of swing left, so its axis is 177 regardless of what folds mid-event; the 265 I
had was 88 values of pure dead zone.

Correct form, widen then cap:

```
axis(e) = 2 * min(acc(e) + W(e), rem(e)) + 1
  acc(e) = sum of spans of completed events      (how far d can already have drifted)
  rem(e) = sum of spans of events e..9           (how much can still overturn it)
  W(e)   = largest |contribution to d| at any decision node inside event e
```

`W` depends on what folds into `d` during the event: partial sums for the additive events,
the eliminated player's best for the ladders, and the opponent's banked best in the final
phase of a best-of-3.

| event | acc | rem | W | audited | I had | |
|---|---:|---:|---:|---:|---:|---|
| 100m | 0 | 505 | 68 | **137** | 89 | 1.54× |
| Long Jump | 88 | 417 | 30 | **237** | 177 | 1.34× |
| Shot Put | 118 | 387 | 48 | **333** | 237 | 1.41× |
| High Jump | 166 | 339 | 30 | **393** | 333 | 1.18× |
| 400m | 196 | 309 | 78 | **549** | 481 | 1.14× |
| 110m Hurdles | 284 | 221 | 30 | **443** | 468 | 0.95× |
| Discus | 309 | 196 | 30 | 393 | 393 | 1.00× |
| Pole Vault | 339 | 166 | 48 | 333 | 333 | 1.00× |
| Javelin | 387 | 118 | 30 | 237 | 237 | 1.00× |
| 1500m | 417 | 88 | 83 | **177** | 265 | 0.67× |

From discus onward `acc + W > rem` every time, so the widening is entirely absorbed and
the axis is pinned at `2·rem + 1`. **The last four events do not accumulate at all** — they
are governed by the dead zone, not by how far the score has drifted.

### Per-event findings

**The four rethrow events (100m, 400m, 110mh, 1500m).** A rethrow picks up *all* the dice
in the set, so the only choice is freeze-all versus rethrow-all, and two rolls with the
same score are the same state. Face-count vectors collapse to distinct set scores: 100m
126 → 37, 400m 21 → 15, hurdles 252 → **26** (9.7×). 1500m gains nothing — with one die
the score *is* the face. Group total 1,160,928 → 362,004.

`rerolls = 0` forces a freeze, after which the remaining sets are thrown with no choices
at all, so that subtree is a fixed convolution independent of `d`. 208 of 1,248 control
states could be precomputed away; not worth doing at this size, but it is the same trick
as shot put's forced-region kernel.

**Shot put.** A foul costs only the current attempt — the banked best survives. So while
`cur ≤ b_own`, throwing *weakly dominates* stopping, because a foul returns you to `b_own`,
which is exactly what stopping gives. That region is policy-free and independent of `d`
and `b_opp`, so it precomputes as an entry kernel per `b_own`. With the dead region
(`cur + 6(8−t) ≤ b_own`) and the forced stop at `t = 8`, **only 30% of `(b_own, t, cur)`
triples carry a real decision** — 2,233 of 7,344.

**High jump and pole vault store no dice state whatsoever.** Clearing gives exactly the bar
height, so each jump is binary; binary means maximise `P(clear)`, which fixes pole vault's
die count as a function of the height alone (3 dice at 10, rising to 8 from 26); and three
independent trials at the same bar fold to `1 − (1−p)³`. Nothing carries between jumps.
Pole vault's entire dice engine is a **20-float lookup table**. The state is just
`(h, best_L, best_T, alive flags, turn)`.

Elimination is absorbing and freezes your score, so an eliminated player's best folds
straight into `d`. High jump 673,992 → 194,436; pole vault 3,822,840 → 1,025,640.

**Discus and javelin.** Both node counts turned out reachability-tight, which is the first
time a number of mine survived an audit unchanged: discus has 25 reachable `(u, fs)` pairs
giving exactly 367 post-throw nodes, javelin has 36 giving exactly 701.

The dead test differs between them and copying one to the other would be a bug: discus can
still gain 6 per remaining die, javelin only **5**, because only odd faces freeze. Dead
regions are 17.5% and 14.2%.

Neither has a *guaranteed* region. A foul needs only that every remaining die shows the
wrong parity, probability `(1/2)^u > 0` for any `u ≥ 1`, so the foul branch always returns
to `b_own` and that axis never drops out. Long jump's jump phase was special in having no
downside at all.

The shot-put forced-continue argument applies to both but buys nothing: it removes the stop
branch, not the state, because discus and javelin still have a live freeze-subset choice at
every node. Shot put collapsed only because stop-or-throw was its *only* decision.

**Long jump.** Two phases, two different reductions. The run-up freezes the `k` smallest,
so the roll is observed only through its sorted prefix sums — and since those increase in
`k`, once one exceeds the budget every larger `k` busts identically. The canonical form is
the *feasible prefix only*: at `(nf=0, fs=0)` the 252 five-dice multisets collapse to 53,
because with a budget of 8, `{1,2,3,6,6}` and `{1,2,3,4,5}` are the same position. Run-up
roll-nodes 3,824 → **264**.

The jump phase has no foul, which uniquely enables the guaranteed region: if
`acc + r > b_own` the attempt will overwrite the banked best whatever happens, so `b_own`
drops out entirely.

### Where it landed

| event | states | share |
|---|---:|---:|
| Long Jump | 688,769,916 | 60.8% |
| Javelin | 301,919,751 | 26.6% |
| Discus | 77,131,359 | 6.8% |
| Shot Put | 63,878,314 | 5.6% |
| Pole Vault | 1,025,640 | 0.1% |
| 400m | 197,640 | |
| High Jump | 194,436 | |
| 110m Hurdles | 69,108 | |
| 100m | 60,828 | |
| 1500m | 50,976 | |
| **total** | **1,133,297,968** | |

Feasibility is unchanged and comfortable: one `f32` per state, `d`-slices independent and
parallel, peak RAM the size of one long-jump slice.

**1500m is the right first thing to build**, not hurdles as the previous entry suggested.
Being the last event, its continuation is *literally* the terminal step function — no
chaining, nothing upstream that can be wrong — so it is a complete self-contained
two-player solve at 50,976 states. It exercises the `reroll_sets` engine, the sequential
leader-then-trailer structure, the diff axis with dead-zone pruning, and the
`V(d = 0) = ½` assertion. Everything else then plugs into a chain already known good.

### The recurring mistake

Every wrong number in this project so far, mine included, has been the same move:
**multiplying out a box where the reachable set is constrained.** The counter is always to
enumerate the reachable set instead. It has now been worth between 1.2× and 14× every time
it has been applied, and the two numbers that survived unchanged (discus 367, javelin 701)
were the two that had been enumerated rather than boxed in the first place.

The one boxed product left is long jump's `31 (b_L) × 31 (b_T) × acc` — three axes all
measuring points, charged as independent when `acc ≤ 6(5−r)` and `b_own` only enters
through `max(0, acc − b_own)`.

---

## Carried forward

Sweeping `2026-08-09-02-sizing-the-two-player-state-space.md`.

**Done since**

- ~~**Audit javelin and discus.**~~ *Closed:* both node counts verified reachability-tight
  (367 and 701), dead regions applied, javelin's max-face-5 ceiling caught.
- ~~All four remaining events unaudited.~~ *Closed:* all ten now audited in rulebook order.
- ~~**Housekeeping** — delete the legacy pipeline.~~ *Closed in entry 02.*

**Still live**

- **Read the Championship scoring rules properly and quote them.** Untouched. The estimate
  that medals bound the coupling axis to ~21 values instead of ~400 — and therefore shrink
  the whole problem by an order of magnitude — still rests entirely on my reasoning rather
  than on the rulebook text. This is the highest-leverage unread paragraph in the project.
- **Play a full game in a browser by hand.** Still only the opening position has ever been
  drawn.
- **Add the remaining nine interactive engines.** Unchanged.
- **Decathlon mode.** Unchanged, and still the harness a two-player AI would plug into.
- **AI opponents — store policies or recompute?** Unchanged and still the open human
  decision. Values-only makes this an analysis exercise; storing policies (~1.1 GB, or
  recompute-on-demand per `d`-slice) makes it an opponent.

---

## Next steps

**Immediate**

- **Push on long jump's `(b_L, b_T, acc)` box.** 61% of the count and the last boxed
  product. Every previous attempt at this class of fix has paid.
- **Update `RULES-CHECKLIST.md`** for the two items below, which this session contradicts
  or adds.

**Then**

- **Build 1500m end to end.** Self-contained, 50,976 states, no chaining.
- **Then the other three rethrow events**, which come free from the same engine, followed
  by the two ladders (no dice state at all), then shot put, then long jump last.

**Assertions to build in from the start**, none of which cost anything:

- `V(d = 0, event boundary) = ½` exactly, by symmetry.
- `V` monotone nondecreasing in `d`, everywhere.
- Under a lead large enough that win probability is locally linear in `d`, induced play must
  converge to the EV-optimal policy, cross-checking against the ten values already pinned in
  `tests/disciplines.rs`.
- **The ladder closed form.** Last event, opponent finished, you need height `X`: clearing
  below `X` is worthless, clearing above it is no better than `X`, `p` is monotone
  decreasing, and skipping is free — so skip to the lowest sufficient bar and
  `P(win) = 1 − (1 − p(X))³`. For high jump needing 18 that is exactly 0.875000; for pole
  vault needing 30, 0.428140. One number that exercises the ladder, the fold-on-elimination
  and the `d`-dependence at once.
