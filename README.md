# Knizia Decathlon

Optimal strategies for the ten events of **Reiner Knizia's Decathlon**, a 1990 dice
game played with eight dice and a score sheet. The [rules are two pages][pdf] and are
the specification this repository is checked against.

Two things live here:

- **Solvers** — an exact dynamic program per event, giving the full distribution of the
  final score under optimal play. All ten events are covered.
- **A web UI** — play an event yourself in the browser, against the same rules engine.
  100 Metres so far.

[pdf]: https://www.knizia.de/wp-content/uploads/reiner/freebies/Website-Decathlon.pdf

---

## Optimal expected values

Each event is solved as a single-player game maximising the expected value of that
event's own score. Every number below has been **independently reproduced to nine
decimal places** by a separate brute force written from the rulebook alone, and is
pinned in `tests/disciplines.rs`.

| Event | Key | EV | SD | Range |
|-------|-----|---:|---:|---|
| 100 Metres | `100m` | 23.998 | 6.19 | −48 … 40 |
| Long Jump | `longjump` | 22.395 | 3.97 | 0 … 30 |
| Shot Put | `shotput` | 18.634 | 12.03 | 0 … 48 |
| High Jump | `highjump` | 19.263 | 2.27 | 0 … 30 |
| 400 Metres | `400m` | 25.668 | 5.46 | −48 … 40 |
| 110m Hurdles | `110mh` | 21.375 | 2.96 | 5 … 30 |
| Discus | `discus` | 22.317 | 4.54 | 0 … 30 |
| Pole Vault | `polevault` | 17.278 | 11.14 | 0 … 48 |
| Javelin | `javelin` | 22.252 | 4.30 | 0 … 30 |
| 1500 Metres | `1500m` | 26.529 | 5.25 | −48 … 40 |

Multiplayer turn order and the Championship (medals) variant are out of scope — see
`worklog/RULES-CHECKLIST.md` for why the latter is a genuinely different objective and
not just a rescaling.

---

## Play it

```bash
bash scripts/serve_web.sh          # builds the wasm bundle, serves on :8000
```

Then open <http://localhost:8000>. Requires [`wasm-pack`][wp].

The rules engine is Rust compiled to WebAssembly (`src/game/`), not a JavaScript
reimplementation. That is deliberate: a second copy of the rulebook is a second thing
to keep honest, and this repository has already been bitten once by a paraphrase
drifting away from the rules it summarised. The page renders; it decides nothing.

[wp]: https://rustwasm.github.io/wasm-pack/installer/

---

## Solve it

```bash
cargo run --release -- list                 # list event keys
cargo run --release -- solve 100m           # EV/SD for one event
cargo run --release -- analyze              # solve all, write output/<key>/
cargo run --release -- analyze longjump --out output
cargo test                                  # unit + integration tests
```

`analyze` writes `pmf.csv`, `cdf.txt`, `summary.json` and SVG charts per event under
`output/<key>/`.

---

## How the solvers work

Dice outcomes are enumerated as face-count vectors weighted by their **multinomial
multiplicity** (`src/dice.rs`), so expectations use correct probabilities. Three shared
engines cover the ten events:

- **Reroll sets** (`disciplines/reroll_sets.rs`) — 100m, 400m, 1500m, 110m Hurdles.
  Dice are split into groups; each group is thrown once for free and may be rethrown
  from a shared pool of five. Freezing a group locks its score.
- **Best of N attempts** (`disciplines/best_of_n.rs`) — Shot Put, Long Jump, Discus,
  Javelin. Play *within* an attempt is conditioned on the best score banked so far via
  a value function `v(k, b)`: with a strong score in hand you gamble, otherwise you
  play safe.
- **Rising bar** (`disciplines/heights.rs`) — High Jump, Pole Vault. The bar starts at
  10 and rises by 2; three failures at a height end the event.

Each returns an exact `Dist` over integer scores, from which EV, SD, PMF and CDF follow.

### Event rules, in brief

Summaries drift — [read the rules][pdf] before changing a solver.

| Event | Dice | Shape | Scoring |
|---|---|---|---|
| 100 Metres | 8 | two sets of 4, +5 shared rethrows; a rethrow picks up **all four** | 1–5 add, **6 subtracts 6** |
| Long Jump | 5 | 3 attempts; run-up (freeze low, foul above 8) then jump | sum of the jump dice |
| Shot Put | 8 | 3 attempts; throw dice one at a time, stop any time, a **1 fouls** | sum of thrown dice |
| High Jump | 5 | 3 jumps per height; clear if all five total ≥ the bar | highest height cleared |
| 400 Metres | 8 | four sets of 2, +5 shared rethrows | 1–5 add, **6 subtracts 6** |
| 110m Hurdles | 5 | one set of 5, up to 5 rethrows of all five | plain sum, **no six penalty** |
| Discus | 5 | 3 attempts; freeze **even** faces only, ≥1 per throw | sum of frozen dice |
| Pole Vault | 8 | 3 jumps per height; choose the die count, any **1 fails** | highest height cleared |
| Javelin | 6 | 3 attempts; freeze **odd** faces only, ≥1 per throw | sum of frozen dice |
| 1500 Metres | 8 | eight sets of 1, +5 shared rethrows | sum, **6 subtracts 6** |

---

## Layout

```
src/
  dice.rs             dice enumeration with multinomial weights
  dp.rs               Dist (exact PMF over integer scores) + the optimal-action rule
  disciplines/        the ten solvers and their three shared engines
  game/               interactive rules engines for playing by hand
  analysis.rs         PMF/CDF/SVG output
  wasm.rs             browser bindings (feature `wasm`)
web/
  index.html          the UI; renders only
  pkg/                wasm-pack output (generated)
output/               per-event pmf.csv, cdf.txt, summary.json and SVGs
worklog/              why things are the way they are; start with README.md
tests/                integration tests, including independently derived values
```

The repository is Rust only. A legacy C++/Python pipeline for 100m and Long Jump
lived under `solvers/`, `players/` and `analysis/`; it was superseded by this crate
and removed. Its 100m solver averaged 126 sorted dice patterns uniformly instead of
by multiplicity, overstating the event by ~1.45 points, so any number quoted from it
elsewhere is wrong. See `worklog/2026-08-09-rules-fidelity-and-web-ui/` for the
measurement.

---

## Contributing

Run `bash scripts/lint.sh --fix` then `bash scripts/lint.sh` before committing
(rustfmt + clippy, via [standard-linter][sl]).

Anything that rests on an inference rather than a rulebook quotation belongs in
`worklog/RULES-CHECKLIST.md`. Anything independently derived belongs in a test.

[sl]: https://github.com/maxjiang216/standard-linter
