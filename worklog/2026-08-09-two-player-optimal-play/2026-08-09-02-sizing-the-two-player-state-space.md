# Sizing the two-player state space

## TL;DR

- **A two-player optimal-play DP is ~1.4e9 states and comfortably feasible** — minutes of
  core time, ~200 MB peak RAM, embarrassingly parallel. No approximation needed.
- The whole thing is **backward induction on a single boundary vector `V_e(d)`**, `d` being
  the point difference from completed events. Ten events chained through ~200 floats each.
- **The rulebook's turn order is free; its attempt interleaving is expensive.** "The
  leading player always starts" makes turn order a function of `sign(d)`, already in the
  state. "All first attempts are played first" forces *both* players' banked bests into the
  state at once, which is where 96% of the count lives.
- **Folding rule:** in-event progress merges into `d` exactly when the aggregator is
  addition. The four rethrow events fold completely (2.3M states total, 0.4%); the six
  `max`-scored events do not.
- Found five independent reductions, taking the count **3.02e9 → 1.42e9**, and corrected
  four over-counts of my own along the way. Long jump alone went 2.24e9 → 819M.
- **The Championship variant looks an order of magnitude cheaper** and is the plausible
  route to 3-4 players. Medals bound the cross-event coupling variable to ~21 values
  instead of ~400.
- No solver code written. This entry is the sizing argument that says the code is worth
  writing.
- Separately, **the legacy C++/Python pipeline was deleted** and a bug found in
  `scripts/lint.sh` that had been silently stopping it after its first check — so
  `cargo clippy` had never run locally. Fixed here and upstream.

---

## Details

### The question

Whether an exact optimal-play solver for the two-player game is buildable, or whether
multiplayer has to be Monte Carlo. The existing ten solvers answer "maximise my own
expected score for this event"; the two-player game asks "maximise the probability I
finish the decathlon ahead", which is a different objective and couples all ten events.

### What the rulebook settles

Two sentences determine the entire shape of the model, and they pull in opposite
directions.

> Players decide the order of play in the first discipline by the roll of a die. From the
> second discipline onwards, **the leading player always starts**, followed by the player
> with the second highest running total and so on. Ties are resolved by the throw of a die.

With two players, "who is leading" *is* the sign of `d`, which the state already carries.
Turn order costs nothing — no extra bit, no extra axis. The first discipline and exact
ties contribute a 50/50 mix over the two orderings and nothing more.

Note the direction: the leader moves **first**, so the *trailing* player has the
information advantage — they play knowing exactly what they need. That asymmetry is one
of the more interesting things a solve would quantify.

> If a discipline consists of several attempts, **all first attempts are played first, then
> all second attempts**, and so on. Similarly, if a discipline goes over several heights, all
> players have three consecutive jumps at the first height, then all players jump the next
> height and so on.

This is the expensive one. The four best-of-3 events are **not** "A takes all three
attempts, then B" — they run `L₁ T₁ L₂ T₂ L₃ T₃`. Mid-event, neither player's result has
been decided, so both banked bests must sit in the state simultaneously. That is a
`(hi+1)²` axis where a sequential reading would have needed `(hi+1)`.

Also settled, and worth stating because it drives the region analysis below: a foul costs
you **only the current attempt**. Shot put's "If you throw a one you suffer an invalid
attempt" ends that attempt at 0; previously banked attempts stand.

Sets *within* one attempt are not attempts, so 100m/400m/1500m do not interleave — one
player plays their whole event before the other starts.

### The factorisation

```
state = (d, within-event state, phase)
V₁₀(d) = 1 if d>0, ½ if d=0, 0 if d<0
V_e = solve_event_e(terminal payoff = V_{e+1})
```

Nothing else crosses an event boundary — the rulebook's "The ten disciplines are
independent of each other" is literal. Each event emits a ~200-float vector.

**`d` is a parameter, not a variable, inside an event.** It is the difference from
*completed* events; in-event progress lives in the banked-best axes. So an event is not one
large problem but ~200 independent ones, one per `d`, sharing only the read-only dice
tables. That is what makes this parallel and small-memory.

**The folding rule.** Partial progress merges into `d` iff `score = acc + future`. It fails
two ways here:

| aggregator | turn structure | extra axes |
|---|---|---:|
| `+` (100m, 400m, 1500m, 110mh) | sequential **or** interleaved | 0 |
| `max` (best-of-3) | sequential | 1 (own best) |
| `max` (best-of-3) | **interleaved** ← actual rule | 2 (both bests) |

So interleaving is not what is expensive — `max` is. Interleaving doubles damage that
`max` already causes. Had Knizia scored the throwing events as the *sum* of three
attempts, the whole game would be ~2M states and turn order would not touch the state
space at all. High jump and pole vault escape despite being `max`-scored, because heights
only rise and clearing `h` overwrites the best with a known value.

**Symmetry.** `σ: (d, b_L, b_T, mover) → (−d, b_T, b_L, other)` with `V(σ(s)) = 1 − V(s)`.
Orbits are size 2, so half the space is free. Realised as solving `d ≥ 0` only and
mirroring. This holds *because* the game is zero-sum with both sides optimal — against a
fixed suboptimal opponent it breaks and you need two value functions.

`d = 0` at an event boundary is the fixed point, so `V = ½` exactly. Free assertion for
the solver.

### Reductions found

In the order they turned up. Every one came from the same question: what does the state
actually need to distinguish?

1. **All-or-nothing rethrow collapses the roll to a scalar.** In 100m/400m/1500m/110mh the
   only choice is freeze-all vs rethrow-all, so two rolls with equal score are the same
   state. Face-count vectors → distinct set scores: 100m 126→37, hurdles 252→26 (9.7×).
   The score supports have holes for the same reason the event totals do — a four-dice set
   scores `−24` or `[−17,−13]`, nothing between.

2. **Discus scores are even-only.** Only even faces may be frozen, so the banked-best axis
   is 16 values, not 31. 3.75× on discus. Javelin 31→29 (27 and 29 are unreachable: odd
   totals need an odd die count, capped at 5 dice → 25), shot put 49→48 (no score of 1).

3. **Long-jump run-up: truncate at the first bust.** The run-up freezes the `k` smallest,
   so the roll is observed only through its sorted prefix sums; and since `s_k` increases
   in `k`, once `s_k` exceeds the budget every larger `k` busts identically. The canonical
   form is *the feasible prefix only* — the tail above it is one undifferentiated lump.
   Run-up roll-nodes 3,824 → 264 (14.5×). At `(nf=0, fs=0)` the 252 five-dice multisets
   collapse to 53: with a budget of 8, `{1,2,3,6,6}` and `{1,2,3,4,5}` are the same
   position.

   A weaker version — clamping each face to `min(face, B+1)` — only got 1.94×, because at
   `B=8` the clamp is the identity.

4. **Region analysis on best-of-3 attempts.** Three tests, each collapsing states:
   - **dead** — `acc + max_remaining ≤ b_own`: the attempt cannot beat your own best, so
     it is inert regardless of how it plays out. One shared value.
   - **forced** — no real choice. In shot put, while `cur ≤ b_own` throwing *weakly
     dominates* stopping, because a foul returns you to `b_own`, exactly what stopping
     gives. That region is policy-free **and** independent of `d` and `b_opp`, so it
     precomputes once per `b_own` as an entry kernel into the decision region.
     `t = 8` is a forced stop.
   - **guaranteed** — `acc + r > b_own` in the long-jump *jump* phase: the attempt will
     overwrite the banked best whatever happens, so `b_own` stops mattering and its whole
     31-value axis collapses to one entry. This works only because the jump phase has no
     foul; it will not port to discus/javelin, where a foul returns you to `b_own`.

   Shot put: only **30% of `(b_own, t, cur)` triples carry a real decision** (25.3% dead,
   30.4% forced, 13.9% forced at `t=8`). 3.9× overall. Long jump jump-phase 1.78×; its
   run-up barely moves (1.05×) because it retains too much optionality to be inert.

5. **Last-phase opponent fold.** In the last mover's last attempt the opponent has no
   moves left, so `b_opp` is frozen and folds into `d`. Applies to 1 of 6 phases.

### Corrections to my own counts

Recorded because the same mistake happened four times and will happen again: **multiplying
out a box where the reachable set is constrained.**

| what | wrong | right | |
|---|---:|---:|---|
| Charged in-attempt `acc` as its own axis *and* folded it into `d` | — | — | double count |
| Long-jump jump nodes: blanket `× 31` for `acc` | 14,291 | 2,411 | `acc ≤ 6(5−r)` |
| Long-jump run-up: allowed `fs` up to 7 with `nf = 0` | 1,970 | 1,000 | `nf = 0 ⟹ fs = 0` |
| Widened the diff axis by the event span *and* kept `b_L`/`b_T` axes | 207 | 177 | double count |

The `acc`/foul interaction deserves its own note, since the naive fix is also wrong. `acc`
cannot simply fold into `d`, because a foul reverts to `b_own` rather than keeping `acc`.
The clean parametrisation carries two numbers instead of three:

```
A = d + b_own − b_opp      (diff-scale; the floor / foul value)
e = acc − b_own            (small, −hi..hi)
value = V(A + max(0, e + future)) ,  foul → V(A)
```

### Where it landed

Diff axes are **100% dense** at every event boundary — verified, `|D| == hi−lo+1` for all
ten — even though individual event score sets have holes. Flat arrays indexed by `d`, no
sparsity handling.

| event | states | share |
|---|---:|---:|
| long jump | 819,445,041 | 57.8% |
| javelin | 419,163,651 | 29.5% |
| discus | 110,769,408 | 7.8% |
| shot put | 63,824,722 | 4.5% |
| pole vault | 3,822,840 | 0.3% |
| 110m hurdles | 707,616 | |
| high jump | 673,992 | |
| 400m | 242,424 | |
| 100m | 134,568 | |
| 1500m | 76,320 | |
| **total** | **1,418,860,582** | |

For scale, the same events solved solo with every reduction applied are ~350K control
states. Two-player costs `2 movers × opponent-best axis × diff axis ÷ 2 symmetry` on top —
for long jump exactly `2 × 31 × 177 / 2 = 5,487×`.

### Feasibility

| | |
|---|---|
| work | ~1e10 ops (~10-30 per state) |
| single core | ~2-5 minutes |
| parallel over `d` | seconds |
| peak RAM | ~200 MB (one `d`-slice of long jump) |
| output, values only | ~10 KB |
| output, full policy | ~1.4 GB |

Far cheaper per state than the existing solvers, which propagate a full `Dist` hashmap;
the two-player DP carries **one `f32` per state** (a win probability), so backward
induction over phases needs no distribution mixing and no allocation in the hot loop.

The only sizing decision is whether to store the policy. Values-only is a toy. Playing
against it means either ~1.4 GB on disk or recomputing the current event's `d`-slice on
demand (~200 MB, sub-second).

`d`-slices are perfectly independent — no locks, no shared mutable state; hoist the dice
tables and share them read-only. Expect **bad static load balance**: slices near
`|d| = max` are nearly decided and collapse, slices near `d = 0` cost several times more.
Use work stealing over individual `d` rather than pre-chunking.

Deliberately *not* doing: sharing work across `d` by exploiting that the optimal policy is
piecewise-constant in `d`. Worth maybe 2-3×, costs the trivial parallelism, and is a rich
source of subtle bugs against a problem already this cheap.

### Three and four players

Out of reach exactly. Three players needs two independent diffs and three banked bests;
long jump alone goes to roughly 1e13-1e14. The plan for 3-4 players is heuristic
opponents — target-based (play the two-player policy against the current leader, which
reuses `V(d)` directly at zero extra cost) or plain EV-maximisers, which the existing ten
solvers already are.

### The Championship variant may be the better target

Under points, the cross-event coupling variable is the *score* difference, range ±417,
which is why the diff axes run 177-835 and dominate everything. Under medals (3/2/1
honours) the coupling variable is the *honours* difference, bounded by ten events × a
small per-event delta — roughly **21 values instead of ~400**. Within an event both
players' scores are still needed to decide placement, but those are the `b_L`/`b_T` axes
we already carry.

Rough effect on the two-player numbers: long jump 819M → ~97M, javelin → ~37M, total
**1.42e9 → ~1.5e8**. An order of magnitude, entirely from the axis that has been the
bottleneck throughout. It is also the variant where three players might come within reach
rather than needing Monte Carlo. Not worked out properly yet — the per-event honours split
for two players needs reading off the rulebook first.

---

## Carried forward

Sweeping `2026-08-09-01-auditing-ten-solvers-against-two-pages.md`.

**Done since**

- ~~**The Championship variant is a different objective.**~~ Still true and still unbuilt,
  but no longer just a caveat: sized above, and it looks like the *cheaper* target rather
  than an extra one. Restated under Next steps as real work.

**Still live**

- **Play a full game in a browser by hand.** The opening position renders in both colour
  schemes; every position reached by clicking is still unseen. Untouched this session.
- **Add the remaining nine interactive engines.** 400m/1500m/110mh are `reroll_sets`
  reshapes; discus, javelin and long jump need per-die subset selection in the UI; high
  jump and pole vault need a height ladder, and pole vault a die-count control.
- **Decathlon mode** — ten events, running total, one score sheet. Now has a second reason
  to exist: it is the harness a two-player AI would plug into, and the running total *is*
  the `d` this entry is about.
- **AI opponents** — unchanged in substance, but this entry answers the open question in
  it. The decision was framed as "in-browser or precomputed, since long jump's state space
  is too large to solve on load". For the *solo* policies that stands. For the two-player
  policy the answer is neither: a single `d`-slice is ~200 MB and sub-second, so the
  current event can be solved on demand at the point the diff is known.
- ~~**Housekeeping** — untracked artifacts in the repo root and 16 MB of `.db` files under
  `solvers/`; gitignored, but whether to delete the superseded (and measurably wrong) C++/
  Python pipeline outright is still open.~~ *Closed later the same day:* deleted. See
  below.

### Cleanup, same session

The legacy pipeline is gone — `solvers/`, `players/`, `analysis/` and `setup_env.sh`,
plus the untracked plot artifacts, the two compiled C++ binaries and the two 16 MB policy
databases. The repository is now Rust only. `output/` was kept: it is written by
`cargo run --release -- analyze`, so it is solver output rather than legacy residue, and
it is the only place the ten distributions can be read without running anything.

`.venv/` went with them. It held matplotlib for the deleted plotting scripts, and it was
the reason `lint.sh` ran its Python block at all — the block's `find . -name "*.py"` was
matching 1,587 files inside the virtualenv even after every `.py` in the project was gone.

**`scripts/lint.sh` had a bug that hid all of this.** `((passed++))` returns exit status 1
when the counter is 0 — post-increment evaluates to the old value, and `((expr))` fails
when `expr` is zero — which under the script's own `set -euo pipefail` aborts it. So
lint.sh has **never run more than one check** in this repo: the Python block came first,
died, and `cargo clippy` was never reached locally. It stayed invisible because the first
check was failing anyway on the legacy Python, so the abort looked like an ordinary
failure. Fixed here, and upstream in
[standard-linter#4](https://github.com/maxjiang216/standard-linter/pull/4) — **merged**,
so the vendored copy here is byte-identical to upstream `main` and nothing diverges. That
mattered: the file is copied in rather than fetched, so a fix left only here would have
been silently reverted the next time it was refreshed. CI was never
affected — it calls the standard-linter workflow directly rather than this script, which
is why the clippy error in `270a7bd` was still caught.

`bash scripts/lint.sh` now exits 0 with rustfmt and clippy both green.

---

## Next steps

**Before writing any solver code**

- **Audit javelin and discus.** Javelin is 29.5% of the count and the least examined thing
  left — its `(u, fs)` node count of 701 came from a raw enumeration never sanity-checked,
  and neither event has had the dead-region test applied. Discus got its parity reduction
  but nothing else. Given that every audit so far has found 1.5-14×, treat both numbers as
  upper bounds.
- **Read the Championship scoring rules properly** and quote them here. The 1.5e8 estimate
  above assumes a per-event honours delta small enough to bound the coupling axis at ~21;
  that assumption is doing all the work and has not been checked against the text.

**Writing it**

- **Start with 110m hurdles.** 3,024 control states, single attempt, no fouls, no
  interleaving, additive folding. Milliseconds to solve, and it exercises the entire
  event-chaining machinery. 100m/400m/1500m then come free from the same engine.
- **Then shot put** — interleaving and fouls, but only 153 attempt nodes.
- **Long jump last.** 58% of the cost and most of the complexity.
- **Assertions to build in from the start**, because the encoding is where this goes wrong
  and none of these cost anything: `V(d=0, event boundary) = ½` exactly; `V` monotone
  nondecreasing in `d` everywhere; and under a lead large enough that win probability is
  locally linear in `d`, the induced play must converge to the EV-optimal policy — giving a
  direct cross-check against the ten expected values already pinned in
  `tests/disciplines.rs`.

**Open decision for a human**

- **Whether to store policies at all.** Values-only makes this an analysis exercise
  answering "how big is the first-mover disadvantage" and "how does optimal play change
  when trailing". Storing policies (~1.4 GB, or recompute-on-demand) is what makes it an
  opponent. These want different amounts of engineering and it is worth picking before
  starting.
