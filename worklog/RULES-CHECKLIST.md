# Rules checklist

Everything the code decides that the [rulebook][pdf] does not settle, plus every number
that is believed rather than derived. See `worklog/README.md` for what belongs here and
what belongs in a test instead.

[pdf]: https://www.knizia.de/wp-content/uploads/reiner/freebies/Website-Decathlon.pdf

Status key: `[ ]` open · `[x]` closed, with what closed it.

---

## Modelling shortcuts

- [x] **Long-jump run-up freezes only "the `k` smallest dice"**, and the jump only "the
  `j` largest", instead of every subset. The argument: the run-up state is `(frozen sum,
  frozen count)`, the continuation is weakly decreasing in the sum and the jump depends
  only on the count, so for each `k` the minimum-sum choice dominates; symmetrically the
  jump's continuation is increasing in the banked total, so the maximum-sum choice
  dominates. *Closed 2026-08-09:* a Python brute force enumerating every subset gives
  16.942648747 for a single attempt, matching to nine decimals. Pinned in
  `tests/attempt_engines.rs`.

- [x] **Discus and javelin collapse freezable subsets to `(count, value-sum)` pairs.** Two
  different subsets with the same count and sum leave identical states, so only the
  distinct pairs need exploring. *Closed 2026-08-09:* full-subset brute force matches to
  nine decimals (14.885438946 discus, 15.218094840 javelin). Pinned in
  `tests/attempt_engines.rs`.

- [x] **High jump and pole vault fold the three jumps at a height into `1 - (1-p)³`**
  rather than simulating them. Valid because the three jumps are independent trials with
  the same optimal die count, and clearing on jump 1 gains nothing over clearing on jump
  3. *Closed 2026-08-09:* independent brute force reproduces 19.263445441 and
  17.277634238.

- [ ] **Pole vault picks the die count that maximises the single-jump clear probability,
  once, and reuses it for all three jumps.** This is right when each jump is judged only
  on whether it clears the current bar — which it is, since overshooting scores nothing
  extra and the bar does not move within a height. *Watch for:* this argument breaks
  the moment anything makes the three jumps non-identical (a variant that carries dice
  over, or a rule about ones accumulating).

- [ ] **Skipping a height is modelled but is never chosen.** `p_clear` is monotone
  decreasing in the height, so attempting `h` weakly dominates skipping to `h+2`; the
  skip branch is dead code under the solo-EV objective. It is kept because the rulebook
  example explicitly skips heights, and because a championship objective (medals, not
  points) could make skipping live. *Watch for:* anyone "simplifying" the skip branch
  away before the multiplayer objective lands.

## Inferences the rulebook does not settle

- [ ] **Shot put forces the first die.** "Throw one die after the other. At any point you
  can stop" does not say whether you may stop before throwing anything. We require at
  least one throw. Harmless under the current objective — an empty attempt would score 0,
  which is also what a foul scores — but it would matter if a variant ever distinguished
  "no attempt" from "invalid attempt".

- [ ] **High jump and pole vault stop at 30 and 48 respectively.** The rulebook never caps
  the bar; it rises by 2 forever. Five dice cannot exceed 30 and eight ones-free dice
  cannot exceed 48, so the clear probability is exactly 0 above those and truncating is
  exact. *Watch for:* a variant that adds dice.

- [ ] **The objective is each event's own expected score.** The rulebook has a
  points-total game and a Championship variant scored in medals (3 / 2 / 1 honours). Under
  medals the optimal policy is not EV-maximising — you play to beat a specific rival's
  posted score, which changes long jump and shot put most. Nothing in the solver is wrong
  here; it just answers a different question than the Championship variant asks.

- [ ] **Turn order is ignored.** The rulebook orders play by running total and interleaves
  attempts across players ("all first attempts are played first, then all second"). This
  is information: in a best-of-3 event, a later player knows what they have to beat.
  Out of scope for the current solvers, and it is the substance of eventual AI opponents.

## Unverified numbers

*(None. All ten events' expected values were independently reproduced on 2026-08-09 and
now live in `tests/disciplines.rs`.)*

## Untested paths

- [x] **The web UI has never been opened in a real browser.** *Closed 2026-08-09:*
  rendered under headless Chrome in both colour schemes. Pip layouts match the logged
  faces, the six is flagged, and both palettes hold up.

- [ ] **No interactive state past the opening position has been rendered.** The headless
  screenshots only capture a fresh game, because a screenshot cannot click. The frozen-set
  summary, the exhausted-rethrow state (one button, not two) and the final-result panel
  are all verified through the wasm module under Node but have never been *drawn*.

- [ ] **Nine of the ten events have no interactive engine.** `src/game/` implements 100m
  only; `game::start` returns `None` for the rest and the UI has no event picker.
