# Auditing ten solvers against two pages

## TL;DR

- Read the [rulebook PDF][pdf] properly for the first time and checked all ten solvers
  against it line by line. **No rules bugs in the Rust solvers.** All ten play the game
  the rulebook describes.
- **Independently reproduced all ten expected values** with Python brute forces written
  from the rulebook alone, sharing no code with the solvers. Every one matches to nine
  decimal places. They are now pinned in `tests/disciplines.rs`.
- **Proved the three heuristic move-set restrictions exact** (long-jump smallest-`k` /
  largest-`j`, parity `(count, sum)` collapse) by full subset enumeration. Pinned in a new
  `tests/attempt_engines.rs`.
- **The README's 100 Metres rules recap describes a different game.** The prose is wrong;
  both solvers are right. Rewritten.
- **The legacy C++ 100m solver is wrong by +1.45 points** — it averages 126 sorted dice
  patterns uniformly instead of weighting by multiplicity. Quantified below.
- Added `src/game/`, an interactive rules engine, and a wasm web UI for playing 100
  Metres. One implementation of the rulebook, not two.
- Started this worklog and `RULES-CHECKLIST.md`.

[pdf]: https://www.knizia.de/wp-content/uploads/reiner/freebies/Website-Decathlon.pdf

---

## Details

### The audit

The interesting result is a negative one: the Rust solvers were already right. Every
event's dice count, attempt count, reroll budget, foul condition and scoring rule matches
the rulebook. Worth recording *because* it is negative — the next person to suspect a
rules bug should suspect something else first.

Two events deserve their citations written down, because they are the ones where a
Yahtzee reflex produces a plausible-looking wrong implementation.

**100 Metres** — the rulebook, in full:

> Divide the eight dice into two sets of four. Throw the first four dice. If you are not
> satisfied with the result, pick up all four dice and rethrow them. This can be repeated
> several times until you freeze the first set. Then throw the other four dice and proceed
> in the same manner. [...] You have a maximum of seven throws, one initial throw for each
> set and up to five rethrows which may be divided between the sets as desired.
>
> **Scoring:** Total the value of the dice for numbers one to five, but subtract any sixes
> from the result.

Two things this settles that a Yahtzee player would get wrong. A rethrow picks up **all
four** dice — there is no keeping two and rerolling two. And the sets are played **in
order**: once set 1 is frozen you cannot return to it, so a rethrow saved from set 1 is
only worth whatever set 2 can still do with it. `reroll_sets::solve(2, 4, 5,
score_six_penalty)` is exactly this, and the shared `rerolls` counter carried across the
set boundary is what makes the "divided between the sets as desired" clause work.

**110 Metre Hurdles** scores `sum`, not `score_six_penalty`. It is the only rethrow-style
event whose scoring line does not mention sixes, and it is easy to assume otherwise given
that 100m, 400m and 1500m all subtract them. `hurdles.rs` gets this right.

### Independent verification

An audit that only reads code proves nothing about arithmetic, so every event got a second
implementation — Python, written from the rulebook text, sharing no code with the crate.
Scratch scripts, not committed; the numbers they produced are what got committed.

| Event | Rust | Independent | |
|---|---:|---:|---|
| 100 Metres | 23.997512229 | 23.997512229 | ✓ |
| Long Jump | 22.394956800 | 22.394956800 | ✓ |
| Shot Put | 18.634491985 | 18.634491985 | ✓ |
| High Jump | 19.263445441 | 19.263445441 | ✓ |
| 400 Metres | 25.667993300 | 25.667993300 | ✓ |
| 110m Hurdles | 21.375403087 | 21.375403087 | ✓ |
| Discus | 22.317089285 | 22.317089285 | ✓ |
| Pole Vault | 17.277634238 | 17.277634238 | ✓ |
| Javelin | 22.251507501 | 22.251507501 | ✓ |
| 1500 Metres | 26.528791924 | 26.528791924 | ✓ |

Two things about how this went that are worth not re-learning.

**The first attempt at long jump, discus and javelin did not finish.** Solving best-of-3
with full subset enumeration means 31 banked-best values × 3 attempt levels ≈ 93 complete
attempt solves, each enumerating every subset of every roll. It was killed after fifteen
minutes with no output. Splitting it in two is what made it tractable: verify the *attempt*
engine under the identity payoff with full enumeration (cheap, one solve), then verify the
*best-of-3 layer* separately with the reduced move sets (also cheap). The reduced sets
were fair to use in the second half only because the first half had just proved them
lossless.

**Two of the pinned numbers were briefly wrong, in the direction that matters.** The first
version of the table copied long jump's `22.3950` off four-decimal CLI output and called
it "independently derived" — it was a rounded snapshot of our own solver, exactly the kind
of number the checklist exists to keep out of tests. Then the discus and javelin brute
force returned 5309710.64 and 50709965.17, which is what a probability mass function looks
like when the foul branch adds `1.0` instead of `1.0 / 6**u`. Both were caught by the
numbers being visibly absurd, which is luck; a subtler normalisation bug would have
produced a plausible wrong value and been pinned as truth.

### The heuristic restrictions are exact

Three solvers deliberately consider fewer moves than the rules allow:

- long-jump run-up freezes only "the `k` smallest dice" for each `k`,
- long-jump jump freezes only "the `j` largest" for each `j`,
- discus and javelin collapse freezable subsets to distinct `(count, value-sum)` pairs.

Each is a dominance argument (see `RULES-CHECKLIST.md` for the arguments themselves), and
each was previously just an argument. A brute force enumerating *every* non-empty subset
of every roll now confirms all three to nine decimals: 16.942648747 for a long-jump
attempt, 14.885438946 discus, 15.218094840 javelin. `tests/attempt_engines.rs` pins them,
so widening or narrowing a move set moves a number and fails a test.

### The README described a different game

`README.md` said, under "100m Sprint Rules Recap":

> - Roll 5 dice up to 3 times.
> - After each roll, choose which dice to keep ("freeze") and which to reroll.
> - Score is **sum of frozen dice** after the third roll.

Five dice, three rolls, per-die freezing, no six penalty. That is not the 100 Metres; it is
roughly a Yahtzee turn. Every clause is wrong — the event uses eight dice in two sets of
four, up to seven throws, all-or-nothing rethrows, and sixes subtract.

What makes this worth an entry rather than a one-line fix: **both solvers implement the
correct rules.** The C++ from 2025 and the Rust from last week independently play the real
game. Only the prose drifted, and it drifted in the file that a newcomer reads first and
that an agent working from the repo would take as the specification. The rule about
quoting rather than paraphrasing in `worklog/README.md` is written for this.

### The legacy C++ 100m solver is wrong

Not a rules bug — an arithmetic one, and the Rust README already noted it in passing.
Quantifying it since it was never measured:

```cpp
// dedup to 126 patterns
sort(FOUR_OUTS.begin(), FOUR_OUTS.end());
FOUR_OUTS.erase(unique(FOUR_OUTS.begin(), FOUR_OUTS.end()), FOUR_OUTS.end());
```

...then `combine_avg` averages over them with uniform weight `1.0/ms.size()`. The 126
sorted patterns of four dice are not equally likely — `[1 2 3 4]` has multiplicity 24 and
`[3 3 3 3]` has 1. Averaging them uniformly overweights the flat patterns.

Reproducing that weighting exactly gives **25.447684** against the correct
**23.997512**: the legacy solver overstates 100m by about **1.45 points**, and its
`100m_policy.db` encodes decisions made under those wrong values. The Rust `dice.rs`
enumerates face-count vectors with their multinomial multiplicity, which is the fix.

The C++ and Python solvers, their `.db` files and the matplotlib analysis scripts are now
fully superseded — the Rust crate covers all ten events with correct weighting. They are
left in the tree for now but nothing should be read off them.

### `src/game/` — an interactive rules engine

The solvers answer "what is optimal"; they collapse an event to a score distribution and
never have to name a legal move. Playing needs the opposite: every intermediate position,
spelled out. So `src/game/` is a separate module, not a reuse of `disciplines/`.

The load-bearing decision is that it is **in Rust, compiled to wasm**, rather than
reimplemented in the page's JavaScript. A JS implementation would be quicker to write and
would be a second copy of the rulebook to keep honest — in a project whose central failure
mode is exactly a second copy of the rules drifting (see the README above), that trade is
not worth making. The page renders and nothing else; `Session::apply` decides what is
legal.

Shape: `Game` is a trait (`view()`, `apply()`), `Action` is a serde-tagged enum, and `View`
is everything needed to draw a position. Both cross the wasm boundary as JSON, so adding an
event means adding a `Game` impl and new `Action` variants, with no change to the
boundary. Only 100 Metres exists so far; `game::start` returns `None` for the other nine.

Two smaller choices:

- **Own PRNG (`game/rng.rs`), seeded by the caller.** `rand` on wasm needs a `getrandom`
  backend wired to the host, which is a dependency and a build wrinkle for something a
  40-line PCG does. The seed comes from the browser's `crypto.getRandomValues`. The real
  payoff is reproducibility: a seed replays a game exactly, so a bug report can be a
  number.
- **`crate-type = ["cdylib", "rlib"]` with an optional `wasm` feature**, rather than a
  separate crate. The CLI build never sees `wasm-bindgen`; the wasm build never sees a
  workspace restructure.

Verified end-to-end by loading the wasm module under Node: legal-move lists, rethrow
exhaustion across the set boundary (spend all five in set 1, confirm set 2 is offered only
"Freeze"), set ordering, terminal state rejecting further moves, and the unimplemented-event
error path. Plus five unit tests in `game/m100.rs`.

Rendered under headless Chrome in both colour schemes to check the page itself. Pips match
the logged faces, the six carries its warning outline, and both palettes hold up. What a
screenshot cannot do is click, so only the opening position has ever been *drawn* — the
frozen-set summary, the one-button exhausted-rethrow state and the result panel are
verified as data but not as pixels.

`scripts/serve_web.sh` builds the bundle and serves it — the page loads wasm as an ES
module, which browsers refuse over `file://`, so local play still needs an HTTP origin.

---

## Carried forward

First entry in this worklog; nothing to sweep.

---

## Next steps

**Immediate**

- **Play a full game in a browser by hand.** The opening position renders correctly in
  both colour schemes, but every position reached by clicking is still unseen.
- **Add the remaining nine interactive engines.** In rulebook order the cheap ones first:
  400m and 1500m are the same `reroll_sets` shape as 100m with different geometry, and
  110m hurdles is one set of five. Discus, javelin and long jump need subset selection in
  the UI (clicking individual dice), which is the first real interaction change. High jump
  and pole vault need a height ladder instead of dice groups; pole vault also needs a
  "how many dice" control.

**After that**

- **Decathlon mode** — ten events in order, running total, one score sheet. Needs an event
  picker and a session that outlives a single event, neither of which exists.
- **AI opponents** — the solvers already produce optimal policies but only expose
  distributions, not decisions. Playing against one means exporting the *policy* (which
  action at which state), not just the value. This is the largest piece of work here and
  it is worth deciding early whether the policy is computed in-browser or precomputed and
  shipped: 100m's state space is small enough to solve on load, and long jump's is not.
- **The Championship variant is a different objective.** Medals (3 / 2 / 1 honours) are not
  expected points, and optimal play under them is not EV-maximising — with a rival's score
  already posted you play to beat that number, which changes long jump and shot put most.
  Any AI opponent for Championship mode needs its own solve, not a reuse of these ten.

**Housekeeping**

- Untracked artifacts are sitting in the repo root (`100m_cdf.png`, `*.csv`, `*.txt`) plus
  16 MB of `.db` files under `solvers/`. Gitignored this session, but the question of
  whether the legacy C++/Python pipeline should be deleted outright is still open — it is
  superseded and, as measured above, wrong.
