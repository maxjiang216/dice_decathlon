# Solving all ten, and making them playable

## TL;DR

- **All ten events are solved for two players.** Whole competition in 28 seconds across
  all cores. `twoplayer::chain` answers what any lead is worth entering any event.
- **A bug had the second mover playing to help the first.** The value table is the first
  mover's win probability, and the best-of-three and ladder drivers maximised it on every
  turn. Every deviation and storage figure in entry 04 is superseded.
- **Policy storage is ~14 MB**, not the 38 MB the broken numbers implied. Optimal play
  departs from expected-value play far less often than they suggested — long jump in 1.62%
  of states, not 24.89%.
- **All ten events are now playable in the browser**, from four engines, with a menu, a
  decathlon mode, clickable dice and an explicit button for compulsory throws.
- **Seats and turn order exist for two to four players**, but nothing is wired to the page
  and **no computer chooses a move yet**.
- A `RULES-CHECKLIST.md` claim turned out false: a solo expected-value player *does* skip
  pole vault bars.

---

## Details

### The second mover was playing to help the first

The value table holds the **first mover's** win probability throughout. Both the
best-of-three driver and the ladder driver maximised it on every turn, so the opponent was
maximising *your* chances. Javelin as a last event returned 0.99999 at a level score.

It hid for three reasons, each worth remembering:

- The four additive events handle both movers explicitly through `1.0 - second[..]`, so
  they were right, and the symptom read as "the non-additive events all saturate".
- **Every pinned test still passed.** A solo expected-value solve has no second mover at
  all, so the values in `tests/disciplines.rs` are blind to it.
- Nothing structural catches it: the state counts, monotonicity and symmetry all still
  held.

What exposed it was building `chain` and looking at a number that should obviously have
been about a half. Corrected, javelin at a level score gives **0.4774** — below a half,
matching the first-mover disadvantage the 1500m already showed at 0.4489.

Superseded figures from entry 04:

| event | deviate was | now | sparse was | now |
|---|---:|---:|---:|---:|
| javelin | 23.84% | **9.88%** | 7.0 MB | **4.45 MB** |
| long jump | 24.89% | **1.62%** | 51.1 MB | **7.21 MB** |
| discus | 11.00% | 6.47% | 1.2 MB | 599 KB |
| shot put | 37.30% | 30.67% | 5.1 MB | 2.05 MB |
| 100m | 10.84% | 1.41% | 1,412 | 368 |

### All ten solved

```
event        states       packed B   dense B  sparse B  deviate   best
1500m        50,976          6,372       612       546    6.32%  11.7x
javelin 222,940,788        27.9 MB    6.8 MB    4.45 MB   9.88%   6.3x
polevault  1,917,160        239,645    24,395     4,303    1.50%  55.7x
discus    54,563,287        6.8 MB    1.0 MB     599 KB   6.47%  11.4x
110mh        69,108          8,639       644       140    1.43%  61.7x
400m        197,640         24,705     1,485       757    3.18%  32.6x
highjump    398,728         49,841     4,301       253    0.00% 197.0x
shotput  72,572,689         9.1 MB    1.6 MB    2.05 MB  30.67%   5.8x
longjump 943,425,812      117.9 MB   28.7 MB    7.21 MB   1.62%  16.3x
100m         60,828          7,604       944       368    1.41%  20.7x
```

Every event is checked against a value pinned from an independent implementation before
anything else is believed of it — long jump against **both** its single attempt
(16.942648747) and its best of three (22.394956800), so a bug in either phase fails loudly.

**High jump deviates in no state at all**, and that is not a compression result: its only
choice is attempt-or-skip, attempting always wins, so the policy is a constant. Two things
compound — skipping is never the expected-value play there, and the event sits six
disciplines from the end, so the value function is near-linear across the ±30 it can move.
Deviation grows as the game shortens: 0.00% for the high jump, 1.03% for the pole vault
with two events left, 6.32% for the 1500m.

### Reductions that mattered

Applied in order, each verified exact by the pinned values continuing to hold:

| | javelin states |
|---|---:|
| baseline | 958M |
| player symmetry | 481M |
| reachable banked bests | 421M |
| dead subtrees | 361M |
| reachable phases + forced nodes | **223M** |

The last is the largest and came from asking which states play actually *reaches*: before
the first mover has thrown, both banked bests are still zero, so the 29×29 grid does not
exist until the third phase. And a throw showing no freezable die has nothing to decide,
while one whose single freeze uses the last die in hand leaves no stop-or-continue choice
either.

**Player symmetry is not the identity it looks like.** A first-mover value function is
genuinely asymmetric, because moving second carries an information advantage. The halving
comes from the rulebook instead: the leading player starts, so a first mover is never
behind and the function is never *queried* at a negative difference. It applies fully to
the best-of-three events, where the difference is a fixed parameter, but only to the
*entry* value of the running events, where the difference is a state variable that drifts
as sets freeze.

### Equivalence classes carry their multiplicity

Collapsing rolls into classes is only sound if each class is weighted by how many ordered
rolls it stands for, and getting it wrong would be invisible: state counts, monotonicity
and symmetry all still hold while every probability is quietly off. Three enumerations now
assert they reconstruct exactly `6^n`, joining the freeze engine which already did:

- reroll-set scores — a 4-dice set collapses **1296 ordered rolls into 37 scores**, so most
  classes stand for many rolls; the extremes (four sixes, four fives) for one each
- long jump multisets and jump throws, weighted by multinomial coefficient
- long jump run-up throws, which additionally *group* multisets by feasible prefix, so the
  grouping must preserve the total

The freeze classes are the subtle case: wrong-parity dice collapse to a single symbol, so
the weight is `multinomial × 3^wrong` rather than a plain multinomial.

### The game side: all ten playable

Four engines cover the ten, because they group by shape rather than by name:

| engine | events |
|---|---|
| `running` | 100m, 400m, 110mh, 1500m |
| `ladder` | high jump, pole vault |
| `freeze` | discus, javelin |
| `shotput`, `longjump` | one each |

`m100.rs` was deleted rather than left beside the general engine — two implementations of
one rulebook page is the drift this project keeps warning about.

Reductions proved for the solver were reused in the UI: freeze choices are deduplicated on
`(count, sum)`, which is both correct and what makes the list short enough to read. Dice
are clicked rather than picked from a list of buttons, and a selection is legal exactly
when some offered freeze agrees with it on count and sum — so the page can let you click
anything and still only ever send a move the engine offered.

**Compulsory throws are now a button.** Opening a set, starting an attempt, the shot put's
compulsory first die — all of these used to fire inside a constructor or on the way out of
the previous move, so their result was history before anything was drawn. That matters
most exactly where it is most surprising: a shot put attempt can be voided by a die the
player never saw thrown. It is also why a shot put turn can be **over before it starts**,
which `meet` had to learn to skip past.

### Two rulebook findings

**A solo expected-value player does skip pole vault bars.** `RULES-CHECKLIST.md` claimed
the skip branch was dead code. At height 34 holding 32, attempting is worth 32.5197 and
skipping 32.5326: attempting clears only 22.9% of the time and the other 77.1% ends the
event, forfeiting every higher bar. The dominance argument behind the claim ignored that a
failed attempt is *absorbing*. The solvers were always right — `heights.rs` evaluates
`better(skip, attempt)` — so every published expected value already included skipping; only
the entry was wrong. The same question does **not** carry over to the high jump, where the
ladder declines nothing, and a test now asserts zero rather than merely allowing it.

**`V(d = 0) = ½` is false as stated**, and had been proposed as a free assertion in three
earlier entries. It holds only after averaging over the rulebook's die roll for turn order,
not conditional on who starts — the player who moves first in the 1500m at a level score
wins 44.9%, not 50%.

---

## Carried forward

Sweeping `2026-08-09-04-policy-compaction-and-the-1500m-probe.md`.

**Done since**

- ~~Test the interval structure on a multi-action event.~~ *Closed:* javelin's 24 actions
  did not break it; it compresses to 4.45 MB, in line with the projection.
- ~~Port 1500m to Rust, then the rest in reverse order.~~ *Closed:* all ten solved.
- ~~`V(d=0) = ½` as an assertion.~~ *Closed by disproof;* see above.
- ~~The ladder closed form as an assertion.~~ Superseded: the high jump turns out to have
  no decisions at all, so the closed form is about a branch that never runs.

**Still live**

- **Read the Championship scoring rules properly and quote them.** Untouched across five
  entries now, and still the highest-leverage unread paragraph in the project: the claim
  that medals shrink the coupling axis from ~400 values to ~21 is entirely my reasoning.
- **Long jump's 943M is loose.** Its `live_nodes` prune is cruder than the others — no
  phase-reachability pruning, which is what cut javelin hardest — so its 28.7 MB dense
  figure is an upper bound rather than a tight one.
- **Play a full game in a browser by hand.** Now much more worth doing: ten events, a
  decathlon, and every interaction is new.

---

## Next steps

**To finish the browser game** — the spec is: a menu, any single event or a full decathlon,
two to four players, a choice of which AI each computer uses (defaulting to the two-player
optimum against the current leader, or the runner-up when the computer *is* the leader),
and a hint that shows the human the best move.

1. **Make the advisor a table reader.** It currently re-solves the event on every query,
   which is both slow and throws away the compression work. It should be a waterfall: look
   up the deviation interval, and fall through to the difference-independent
   expected-value baseline when none covers this difference. Two lookups, no solving —
   which is what makes it viable in wasm, where the 28-second parallel solve is a
   non-starter. Needs the solver to *emit* the compressed tables, not just measure them.
2. **A query shape per engine.** The advisor answers reroll-set positions only. The ladder,
   freeze, shot put and long jump shapes are still to write, and both the computer players
   and the hint need all of them — they are the same call.
3. **Bind `meet` to the page**: seats, the AI selector, and a scoreboard.
4. **Pace the computers.** A short delay per move, with each computer's move in the log
   alongside the rival it measured itself against and the difference it was defending.
   Resolving a whole turn instantly would hide the play exactly as the compulsory throws
   used to.
5. **The hint**, outlining the dice it would freeze rather than naming a move in text.

**Before trusting the computer players**

- **Interleave attempts and heights.** The rulebook plays all first attempts, then all
  second; `meet` currently has each player finish their event before the next begins. Exact
  for the four single-attempt events, an approximation for the other six — and it hands the
  later player *more* information than the rules allow. This matters more than a fidelity
  nicety: the two-player solver models the interleaved game, so a policy lifted from it
  would be playing a slightly different game than it was solved for. The fix is for the
  four best-of-three engines to yield at attempt boundaries instead of starting the next
  attempt themselves.
