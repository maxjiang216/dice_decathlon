
#!/usr/bin/env python3
"""
Compute and plot the PMF and CDF of the final score for Knizia's Decathlon — 100 Metres,
assuming optimal play throughout, using the precomputed SQLite policy DB.

Outputs:
- PMF plot (PNG) and optional CSV
- CDF plot (PNG) and optional TXT table

Usage:
  python analyze_100m_pmf_cdf.py \
      --db solvers/100m_policy.db \
      --pmf-out 100m_pmf.png --pmf-csv 100m_pmf.csv \
      --cdf-out 100m_cdf.png --cdf-txt 100m_cdf.txt \
      --verbose
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

    start_pmf = {}
    for (t, p) in FOUR_OUTS:
        sub = pmf_state(1, 5, t[0],t[1],t[2],t[3], None)
        pmf_add(start_pmf, pmf_scale(sub, p))

    mu, sd = pmf_mean_sd(start_pmf)
    if verbose:
        print(f"Final-score EV = {mu:.6f}, SD = {sd:.6f}, support size = {len(start_pmf)}")
        print(f"Total probability = {sum(start_pmf.values()):.6f}")

    conn.close()
    return start_pmf, mu, sd

def pmf_to_cdf(pmf):
    xs = sorted(pmf.keys())
    c = 0.0
    xs_out = []
    cdf = []
    for x in xs:
        c += pmf[x]
        xs_out.append(x)
        cdf.append(c)
    # fix any rounding wobble
    if abs(cdf[-1] - 1.0) < 1e-12:
        cdf[-1] = 1.0
    return xs_out, cdf

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", required=True, help="Path to 100m_policy.db")
    ap.add_argument("--pmf-out", default="100m_pmf.png", help="PMF plot (PNG)")
    ap.add_argument("--pmf-csv", default=None, help="Optional PMF CSV")
    ap.add_argument("--cdf-out", default="100m_cdf.png", help="CDF plot (PNG)")
    ap.add_argument("--cdf-txt", default="100m_cdf.txt", help="CDF text table")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    pmf, mu, sd = reconstruct_pmf(args.db, verbose=args.verbose)

    # --- PMF plot and CSV ---
    import matplotlib.pyplot as plt
    xs = sorted(pmf.keys())
    ps = [pmf[x] for x in xs]
    plt.figure()
    plt.bar(xs, ps)
    plt.xlabel("Final score")
    plt.ylabel("Probability")
    plt.title(f"100m optimal-policy PMF (EV={mu:.3f}, SD={sd:.3f})")
    plt.tight_layout()
    plt.savefig(args.pmf_out, dpi=150)
    print(f"Wrote PMF plot to {args.pmf_out}")

    if args.pmf_csv:
        import csv
        with open(args.pmf_csv, "w", newline="") as f:
            w = csv.writer(f)
            w.writerow(["score","probability"])
            for x in xs:
                w.writerow([x, pmf[x]])
        print(f"Wrote PMF CSV to {args.pmf_csv}")

    # --- CDF (values and plot) ---
    xs_cdf, cdf_vals = pmf_to_cdf(pmf)

    # text table
    if args.cdf_txt:
        with open(args.cdf_txt, "w") as f:
            f.write("# score\tcdf\n")
            for x, c in zip(xs_cdf, cdf_vals):
                f.write(f"{x}\t{c:.12f}\n")
        print(f"Wrote CDF text table to {args.cdf_txt}")

    # plot
    plt.figure()
    plt.plot(xs_cdf, cdf_vals, drawstyle="steps-post")
    plt.xlabel("Final score")
    plt.ylabel("Cumulative probability")
    plt.title("100m optimal-policy CDF")
    plt.tight_layout()
    plt.savefig(args.cdf_out, dpi=150)
    print(f"Wrote CDF plot to {args.cdf_out}")

if __name__ == "__main__":
    main()
