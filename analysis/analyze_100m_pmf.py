
#!/usr/bin/env python3
"""
Compute and plot the *exact* PMF of the final score for Knizia's Decathlon — 100 Metres,
assuming optimal play throughout. Uses the precomputed SQLite policy DB produced by the C++ precompute step.

Usage:
  python analyze_100m_pmf.py --db ../solvers/100m_policy.db --out 100m_pmf.png --csv 100m_pmf.csv

Notes:
- We reconstruct the PMF by recursion:
    pmf(state) = pmf after taking the policy's best action at that state,
    expanding one roll (over all 4-dice outcomes) when the action involves a roll.
- Terminal when stage=2 and the best action is FREEZE → degenerate distribution at set1 + score(current).
- We work entirely with non-decreasing (sorted) 4-die tuples and multinomial weights to cut work by ~10x.
"""

import argparse
import sqlite3
import math
from collections import Counter
from functools import lru_cache

def sorted_4dice_outcomes_with_weights():
    """Yield (d1<=d2<=d3<=d4, probability) for fair 6-sided dice."""
    sides = range(1,7)
    total = 6**4
    # Enumerate nondecreasing quadruples and compute multiplicity
    outs = []
    for d1 in sides:
        for d2 in range(d1,7):
            for d3 in range(d2,7):
                for d4 in range(d3,7):
                    tup = (d1,d2,d3,d4)
                    cnt = Counter(tup)
                    mult = math.factorial(4)
                    for k in cnt.values():
                        mult //= math.factorial(k)
                    p = mult / total
                    outs.append((tup, p))
    return outs

FOUR_OUTS = sorted_4dice_outcomes_with_weights()

def score_set(dice):
    return sum(v if v < 6 else -6 for v in dice)

def fetch_policy(cur, stage, rerolls, dice_sorted, set1_score):
    """Return tuple (best_action, ev_freeze, sd_freeze, ev_reroll, sd_reroll). set1_score=None allowed for stage 1."""
    d1,d2,d3,d4 = dice_sorted
    if stage == 1:
        row = cur.execute(
            """SELECT best, ev_freeze, sd_freeze, ev_reroll, sd_reroll
               FROM states100m
              WHERE stage=? AND rerolls=? AND d1=? AND d2=? AND d3=? AND d4=? AND set1_score IS NULL""",
            (1, rerolls, d1, d2, d3, d4)
        ).fetchone()
    else:
        row = cur.execute(
            """SELECT best, ev_freeze, sd_freeze, ev_reroll, sd_reroll
               FROM states100m
              WHERE stage=? AND rerolls=? AND d1=? AND d2=? AND d3=? AND d4=? AND set1_score=?""",
            (2, rerolls, d1, d2, d3, d4, set1_score)
        ).fetchone()
    if row is None:
        raise RuntimeError(f"Policy row not found for state: stage={stage}, rerolls={rerolls}, dice={dice_sorted}, set1={set1_score}")
    return row  # (best, evF, sdF, evR, sdR)

def pmf_add(pmf_a, pmf_b, w=1.0):
    """Add pmf_b into pmf_a with weight w."""
    for x, p in pmf_b.items():
        pmf_a[x] = pmf_a.get(x, 0.0) + w*p
    return pmf_a

def pmf_scale(pmf, s):
    return {x: p*s for x,p in pmf.items()}

def pmf_mean_sd(pmf):
    mu = sum(x*p for x,p in pmf.items())
    var = sum((x-mu)**2 * p for x,p in pmf.items())
    return mu, math.sqrt(max(0.0, var))

def reconstruct_pmf(db_path, verbose=False):
    conn = sqlite3.connect(db_path)
    cur = conn.cursor()

    @lru_cache(maxsize=None)
    def pmf_state(stage, rerolls, d1,d2,d3,d4, set1):
        dice = (d1,d2,d3,d4)
        best, evF, sdF, evR, sdR = fetch_policy(cur, stage, rerolls, dice, set1)
        if best == "freeze" and stage == 2:
            total = set1 + score_set(dice)
            return {total: 1.0}

        pmf = {}
        if best == "freeze":
            # stage 1 -> roll initial set 2 and continue
            assert stage == 1
            s1 = score_set(dice)
            for (t, p) in FOUR_OUTS:
                sub = pmf_state(2, rerolls, t[0],t[1],t[2],t[3], s1)
                pmf_add(pmf, pmf_scale(sub, p))
        else:
            # reroll (stage 1 or 2): consume one reroll and redraw 4 dice
            assert rerolls > 0
            for (t, p) in FOUR_OUTS:
                sub = pmf_state(stage, rerolls-1, t[0],t[1],t[2],t[3], set1)
                pmf_add(pmf, pmf_scale(sub, p))

        return pmf

    # Starting distribution: average over initial roll of set 1
    start_pmf = {}
    for (t, p) in FOUR_OUTS:
        sub = pmf_state(1, 5, t[0],t[1],t[2],t[3], None)
        pmf_add(start_pmf, pmf_scale(sub, p))

    mu, sd = pmf_mean_sd(start_pmf)
    if verbose:
        print(f"Final-score EV = {mu:.6f}, SD = {sd:.6f}, support size = {len(start_pmf)}")
        # sanity: probabilities sum to 1
        print(f"Total probability = {sum(start_pmf.values()):.6f}")

    conn.close()
    return start_pmf, mu, sd

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", required=True, help="Path to 100m_policy.db")
    ap.add_argument("--out", default="100m_pmf.png", help="Output image path (PNG)")
    ap.add_argument("--csv", default=None, help="Optional CSV dump of (score,prob)")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    pmf, mu, sd = reconstruct_pmf(args.db, verbose=args.verbose)

    # Plot
    import matplotlib.pyplot as plt
    xs = sorted(pmf.keys())
    ps = [pmf[x] for x in xs]
    plt.figure()
    plt.bar(xs, ps)
    plt.xlabel("Final score")
    plt.ylabel("Probability")
    plt.title(f"100m optimal-policy PMF (EV={mu:.3f}, SD={sd:.3f})")
    plt.tight_layout()
    plt.savefig(args.out, dpi=150)
    print(f"Wrote plot to {args.out}")

    if args.csv:
        import csv
        with open(args.csv, "w", newline="") as f:
            w = csv.writer(f)
            w.writerow(["score","probability"])
            for x in xs:
                w.writerow([x, pmf[x]])
        print(f"Wrote CSV to {args.csv}")

if __name__ == "__main__":
    main()
