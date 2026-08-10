# Policy compaction, and what the 1500m probe measured

## TL;DR

- Prototyped the two-player 1500m solver in Python. It **reproduces the audited state count
  exactly** (288 × 177 = 50,976) and the EV-optimal expected score to nine decimals, so the
  audit method and the rules model both check out independently.
- **Playing the EV-optimal policy instead of the win-probability one costs up to 0.086 win
  probability.** That is the justification for this whole project in one number.
- **Only 3.27% of states genuinely deviate from the EV-optimal action.** An earlier count of
  32.9% was wrong: it counted ties, where both actions are equally optimal and the choice is
  free.
- **Every deviating control state deviates on exactly one contiguous interval** of the
  margin axis. Run histogram is `{1 run: 128, 3 runs: 160}` — never 2, never 4+. So the
  storage primitive is an *interval*, not a threshold.
- That makes policy storage **O(control states)** rather than O(control × diff), projecting
  the full game to **~14 MB** against 480 MB bit-packed or 1.08 GB raw.
- Settled: **f64 throughout, clamp to 0/1 at 1e-9, tie epsilon 1e-9, EV-max as the default
  action.**

---

## Details

### What the probe is

1500m is the only event solvable standalone — being last, its continuation is literally the
terminal step function, with nothing upstream that can be wrong. It is also the
**adversarial case** for every compaction idea: `V` is a pure step, so curvature is maximal
and the central-limit smoothing that makes earlier events well-behaved does not apply. Any
bound measured here is pessimistic for the other nine.

Two independent checks fell out for free:

- EV-optimal expected score **26.528791924**, matching `tests/disciplines.rs` exactly.
- State count **288 control × 177 margin = 50,976**, matching entry 03's audit exactly. The
  audit derived that from combinatorics; the solver derived it from actually enumerating
  reachable states. They agree.

### The margin axis

The prototype initially carried the accumulated score and the opponent's target as separate
axes, which is the same box-multiplication mistake this project keeps making. Only their
difference matters:

```
m = (what I must beat) - (what I have banked)     "margin still needed"
freezing a die worth s  ->  m - s
at the last die: win iff m < 0, tie iff m == 0
```

One axis, and it is exactly the `2 * min(acc + W, rem) + 1 = 177` the audit predicted.

### How much the two-player policy is actually worth

| margin needed | P(win) optimal | P(win) playing EV | regret |
|---:|---:|---:|---:|
| −16 | 0.999994 | 0.999982 | 0.000013 |
| 0 | 0.999622 | 0.998692 | 0.000930 |
| 8 | 0.997875 | 0.991885 | 0.005990 |
| 16 | 0.987230 | 0.955966 | 0.031264 |
| 27 | 0.598000 | 0.527666 | 0.070334 |
| **32** | **0.188029** | **0.112225** | **0.075803** |
| 40 | 0.000173 | 0.000011 | 0.000161 |

Peak regret **0.086**. The gap is concentrated where the event is genuinely contested —
needing roughly 16 to 32 more points than the opponent. Deviation rate follows the same
shape: ~0% when saturated, peaking at **10.6% around margin 16**.

Read plainly: a decathlete who plays every event EV-optimally is giving away up to eight
points of win probability per event, precisely in the situations that decide matches.

### Ties are not deviations

The first measurement said 32.9% of states differ from the EV policy, roughly flat across
all margins. The flatness was the tell. At margin −48 you win whatever you do, so both
actions are valued 1.0, the argmax is an arbitrary tie, and "differs" was counting
don't-care states.

The right question is not *does the argmax differ* but *would using the EV action cost
anything*:

```
deviation  iff  max(V_a) - V[ev_action] > eps
```

That drops it to **3.27%**, and the deviation profile becomes the sensible hump above
rather than a flat band.

### Ties and dead zones resolve to EV-max

Decided in discussion; it does three jobs at once and none of them are obvious from the
code, so:

1. **It minimises the deviation set.** The tie-break rule and the compression scheme are
   the same decision — every tie resolved toward EV is a state costing zero storage.
2. **It keeps the bot sane once the match is settled.** At win probability exactly 0 or 1
   every action is win-prob-optimal, so a naive argmax picks arbitrarily. The AI would
   freeze a six for −6 in the 100m while hopelessly behind and look broken.
3. **It is more robust against a fallible opponent**, which is the real reason. Two
   different dead conditions:

   | | recoverable? | why EV-max |
   |---|---|---|
   | hard dead zone, `\|d\| > remaining swing` | no, combinatorially | cosmetic only |
   | soft saturation, `V` within 1e-9 of 0/1 | **yes**, just unlikely | genuinely better play |

   The soft band is far wider than the hard one, and its probabilities assume the *opponent
   plays optimally*. Against a human who does not, a bot behaving randomly at `V = 0.9999`
   throws away real equity. Banking points there keeps you alive when they err.

Note this is deliberately **not** a lexicographic `(win_prob, EV)` dynamic program. That
would carry EV as a second value through the solve and yield "EV-optimal among
win-prob-optimal actions", which can differ from the solo-EV action. The chosen scheme is
better for our purpose: it never sacrifices win probability (the EV action is only used
where it is already win-prob-optimal), needs no second value function, and makes the
deviation set *smaller* rather than larger.

### The deviation set is one interval, not one threshold

Run-length encoding the action sequence along the margin axis, with ties filled by the EV
action:

```
1 run  : 128 control states    never deviates; EV is optimal everywhere
3 runs : 160 control states    EV | deviation | EV
2 runs : 0        4+ runs : 0
```

Unanimous, and robust to the tie epsilon across four orders of magnitude. The shape makes
sense in hindsight: at very negative margin you are already winning, at very positive you
are hopeless, both saturated — the deviation lives entirely in the contested band between.

We had guessed a *threshold* (one breakpoint, which would show as 2 runs). That is the wrong
primitive. It is an **interval**.

### Storage

Because a deviating control state needs one interval regardless of how long the diff axis
is, storage is O(control states):

| scheme | full game |
|---|---:|
| raw, 1 byte per state | 1.08 GB |
| bit-packed by action arity | 480 MB |
| **EV baseline + deviation intervals** | **~14 MB** |
| store nothing, recompute per event | 0 |

The ~14 MB is 1.7 MB of `d`-independent EV baseline (which `src/disciplines/` already
computes) plus ~12.6 MB of intervals at roughly 55% of control states × 5 bytes.

1500m alone only shows 3.4× against bit-packing because its axis is short at 177; the ratio
grows with axis length and most events are wider.

**Recompute-per-event remains the best option for the web UI**, since a slice is ~11 MB at
f32 or 22 MB at f64 and solves in milliseconds. The 14 MB matters only if a precomputed
policy needs shipping.

### Numerics

- **f64 throughout**, and everywhere, not only where the tie threshold forces it. A 1e-9
  comparison is below f32 resolution — near 1.0 the f32 spacing is ~6e-8, so it would be
  reading noise — but the general reason is accuracy across ten chained events, and the cost
  is irrelevant at these sizes. The largest slice (long jump) is 22 MB at f64, so sixteen
  threads hold ~355 MB. This also matches the existing crate, which contains no `f32` at
  all: `dp.rs` and `analysis.rs` are already f64 end to end, so there is nothing to migrate.
- **Clamp to exactly 0 and 1 within 1e-9.** Stops drift compounding across ten chained
  events and makes dead zones exact rather than approximate. It also removed 7 spurious
  deviations that were pure float noise.
- **Tie epsilon 1e-9.** The result is insensitive: 1e-12 and 1e-9 give byte-identical
  output, and 1e-6 shifts the deviation count by only 0.25 points.

---

## Carried forward

Sweeping `2026-08-09-03-auditing-every-event-state-space.md`.

**Done since**

- ~~Decide whether to store policies or recompute.~~ *Closed:* recompute per event for
  interactive play; if a policy must ship, EV baseline plus deviation intervals at ~14 MB.
  This was the open human decision carried through three entries.

**Still live**

- **Read the Championship scoring rules properly and quote them.** Untouched across four
  entries now, and still the highest-leverage unread paragraph in the project — the claim
  that medals shrink the coupling axis from ~400 values to ~21 is entirely my reasoning.
- **Push on long jump's `(b_L, b_T, acc)` box.** Still 61% of the count and still the only
  boxed product left.
- **Play a full game in a browser by hand;** only the opening position has ever been drawn.
- **Add the remaining nine interactive engines.**
- **Decathlon mode** — ten events, running total, one score sheet.

---

## Next steps

**Immediate**

- **Test the interval structure on a multi-action event.** 1500m is binary. Javelin has 24
  actions and discus 18, so there are more ways to differ and possibly more than one
  interval. Testable without solving the chain: run a single best-of-3 attempt engine
  against a synthetic continuation — a step function for the adversarial case, a Gaussian
  CDF for a realistic mid-game one — and count runs. This is the one measurement that could
  break the 14 MB projection.
- **Port 1500m to Rust** and pin both cross-checks as tests: the EV score to nine decimals,
  and the 50,976 state count.

**Then**, in reverse rulebook order because backward induction demands it: javelin, pole
vault, discus, 110mh, 400m, high jump, shot put, long jump, 100m.

**Assertions to carry from the start**, all free:

- `V(d = 0, event boundary) = ½` exactly, by symmetry.
- `V` monotone nondecreasing in `d`, everywhere.
- Ladder closed form: opponent finished and needing height `X`, skip to the lowest
  sufficient bar and `P(win) = 1 − (1 − p(X))³` — 0.875000 for high jump needing 18,
  0.428140 for pole vault needing 30.
- Large-lead limit: where win probability is locally linear in `d`, induced play must
  converge to the EV-optimal policy, cross-checking the ten values in `tests/disciplines.rs`.
