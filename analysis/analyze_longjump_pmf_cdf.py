#!/usr/bin/env python3
"""
Reconstruct and plot PMF and CDF for Long Jump final score under optimal play,
using the precomputed SQLite DB produced by longjump_precompute.cpp.

- First reconstruct the *attempt* distribution exactly (one attempt: run-up then jump).
- Then compute the *event* distribution for "best of k attempts" (iid) via CDF^k.

Outputs:
  --attempt-pmf, --attempt-cdf, --attempt-cdf-txt
  --final-pmf, --final-cdf, --final-cdf-txt   (final = best of k)

DB schema (from the C++ precompute):
  lj_post(phase, sum_frozen, n1..n6, f1..f6)
    phase: 1=RUNUP_POST, 3=JUMP_POST
    sum_frozen: only for RUNUP_POST (NULL for JUMP_POST)
    n1..n6: counts of pips among rolled dice (post-roll state)
    f1..f6: how many of each pip to freeze now (optimal)
"""
import argparse, sqlite3, math
from functools import lru_cache
from collections import Counter

RUNUP_PRE=0; RUNUP_POST=1; JUMP_PRE=2; JUMP_POST=3

def score_from_counts(cnt):
    return sum((i*cnt[i]) for i in range(1,7))

def outcomes_counts(n):
    """List of (counts_dict, probability) for rolling n fair dice, using nondecreasing multiset + multinomial weights."""
    if n == 0:
        return [({1:0,2:0,3:0,4:0,5:0,6:0}, 1.0)]
    total = 6**n
    outs=[]
    def rec(left, start_face, cur):
        if left==0:
            m = math.factorial(n)
            for k in cur.values():
                m //= math.factorial(k)
            p = m / total
            cnt = {i:cur.get(i,0) for i in range(1,7)}
            outs.append((cnt, p))
            return
        for face in range(start_face, 7):
            cur[face] = cur.get(face,0)+1
            rec(left-1, face, cur)
            cur[face] -= 1
            if cur[face]==0: del cur[face]
    rec(n,1,{})
    return outs

# cache outcomes
_OUT = {n: outcomes_counts(n) for n in range(0,6)}

def fetch_decision(cur, phase, sum_frozen, cnt):
    row = cur.execute(
        """SELECT f1,f2,f3,f4,f5,f6 FROM lj_post
           WHERE phase=? AND (sum_frozen IS ? OR sum_frozen=?) AND
                 n1=? AND n2=? AND n3=? AND n4=? AND n5=? AND n6=?""",
        (phase,
         None if phase==JUMP_POST else None,
         None if phase==JUMP_POST else sum_frozen,
         cnt[1],cnt[2],cnt[3],cnt[4],cnt[5],cnt[6])
    ).fetchone()
    if row is None:
        raise RuntimeError(f"No policy row for phase={phase}, s={sum_frozen}, cnt={cnt}")
    f = {i: row[i-1] for i in range(1,7)}
    return f

def pmf_add(a, b, w=1.0):
    for x,p in b.items():
        a[x] = a.get(x,0.0) + w*p
    return a

def pmf_scale(a, s):
    return {x: p*s for x,p in a.items()}

def pmf_mean_sd(pmf):
    mu = sum(x*p for x,p in pmf.items())
    var = sum((x-mu)**2*p for x,p in pmf.items())
    return mu, math.sqrt(max(0.0, var))

def reconstruct_attempt_pmf(db_path, verbose=False):
    conn = sqlite3.connect(db_path)
    cur = conn.cursor()

    @lru_cache(maxsize=None)
    def pmf_jump_pre(n_rem):
        # Base: no dice left to roll in jump -> score 0 with prob 1
        if n_rem <= 0:
            return {0:1.0}
        pmf = {}
        for cnt, p in _OUT[n_rem]:
            f = fetch_decision(cur, JUMP_POST, None, cnt)
            add = sum(i*f[i] for i in range(1,7))
            fcnt = sum(f[i] for i in range(1,7))
            sub = pmf_jump_pre(n_rem - fcnt)
            shifted = {x+add: q for x,q in sub.items()}
            pmf_add(pmf, pmf_scale(shifted, p))
        return pmf

    @lru_cache(maxsize=None)
    def pmf_runup_pre(n_rem, s):
        # Base: if we've frozen all 5 dice in the run-up (n_rem==0),
        # jump uses k = 5 - n_rem dice (i.e., 5)
        if s > 8:
            # shouldn't be called, but treat as invalid attempt = 0
            return {0:1.0}
        if n_rem == 0:
            k = 5 - n_rem
            return pmf_jump_pre(k)

        # STOP option -> jump with k = 5 - n_rem dice
        k = 5 - n_rem
        pmf_stop = pmf_jump_pre(k)

        # ROLL option -> expand outcomes of n_rem dice; at POST freeze per optimal policy
        pmf_roll = {}
        for cnt, p in _OUT[n_rem]:
            f = fetch_decision(cur, RUNUP_POST, s, cnt)
            add = sum(i*f[i] for i in range(1,7))
            total_frozen = sum(f[i] for i in range(1,7))
            if add <= 0 or s + add > 8 or total_frozen <= 0:
                # No legal freeze -> invalid attempt for this outcome: contributes mass at 0
                pmf_add(pmf_roll, {0: p})
                continue
            sub = pmf_runup_pre(n_rem - total_frozen, s + add)
            pmf_add(pmf_roll, pmf_scale(sub, p))

        # Choose better branch by EV (tie → lower SD → prefer STOP)
        mu_stop, sd_stop = pmf_mean_sd(pmf_stop)
        mu_roll, sd_roll = pmf_mean_sd(pmf_roll) if pmf_roll else (0.0, 0.0)
        if (mu_roll > mu_stop) or (abs(mu_roll - mu_stop) < 1e-12 and sd_roll < sd_stop):
            return pmf_roll
        else:
            return pmf_stop

    pmf_attempt = pmf_runup_pre(5, 0)
    mu, sd = pmf_mean_sd(pmf_attempt)
    if verbose:
        print(f"Attempt EV={mu:.6f}, SD={sd:.6f}, support size={len(pmf_attempt)}, total prob={sum(pmf_attempt.values()):.6f}")
    conn.close()
    return pmf_attempt, mu, sd

def pmf_to_cdf(pmf):
    xs = sorted(pmf.keys())
    c=0.0; X=[]; F=[]
    for x in xs:
        c += pmf[x]; X.append(x); F.append(c)
    if X and abs(F[-1]-1.0) < 1e-12: F[-1]=1.0
    return X,F

def cdf_power_to_pmf(xs, F, k):
    """Given CDF of one attempt, compute PMF of max of k iid attempts."""
    Fmax = [f**k for f in F]
    pmf = {}
    prev = 0.0
    for x,fk in zip(xs, Fmax):
        pmf[x] = fk - prev
        prev = fk
    return pmf

def dump_txt(path, xs, ys, header):
    with open(path, "w") as f:
        f.write(header + "\n")
        for a,b in zip(xs, ys):
            f.write(f"{a}\t{b:.12f}\n")

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", required=True, help="Path to longjump_policy.db")
    ap.add_argument("--attempt-pmf", default="longjump_attempt_pmf.png")
    ap.add_argument("--attempt-cdf", default="longjump_attempt_cdf.png")
    ap.add_argument("--attempt-cdf-txt", default="longjump_attempt_cdf.txt")
    ap.add_argument("--final-pmf", default="longjump_final_pmf.png")
    ap.add_argument("--final-cdf", default="longjump_final_cdf.png")
    ap.add_argument("--final-cdf-txt", default="longjump_final_cdf.txt")
    ap.add_argument("--k", type=int, default=3, help="best of k attempts (default 3)")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    pmf_attempt, mu_a, sd_a = reconstruct_attempt_pmf(args.db, verbose=args.verbose)

    # PMF/CDF plots (attempt)
    import matplotlib.pyplot as plt
    xs = sorted(pmf_attempt.keys())
    ps = [pmf_attempt[x] for x in xs]
    plt.figure()
    plt.bar(xs, ps)
    plt.xlabel("Attempt score")
    plt.ylabel("Probability")
    plt.title(f"Long Jump (attempt) PMF  EV={mu_a:.3f}, SD={sd_a:.3f}")
    plt.tight_layout()
    plt.savefig(args.attempt_pmf, dpi=150); print(f"Wrote {args.attempt_pmf}")

    Xa, Fa = pmf_to_cdf(pmf_attempt)
    plt.figure()
    plt.plot(Xa, Fa, drawstyle="steps-post")
    plt.xlabel("Attempt score")
    plt.ylabel("Cumulative probability")
    plt.title("Long Jump (attempt) CDF")
    plt.tight_layout()
    plt.savefig(args.attempt_cdf, dpi=150); print(f"Wrote {args.attempt_cdf}")
    if args.attempt_cdf_txt:
        dump_txt(args.attempt_cdf_txt, Xa, Fa, "# score\tcdf")
        print(f"Wrote {args.attempt_cdf_txt}")

    # Final event = best of k attempts
    F_final_pmf = cdf_power_to_pmf(Xa, Fa, args.k)
    Xf = sorted(F_final_pmf.keys())
    Pf = [F_final_pmf[x] for x in Xf]
    muf = sum(x*p for x,p in F_final_pmf.items())
    sdf = math.sqrt(max(0.0, sum((x-muf)**2*p for x,p in F_final_pmf.items())))

    plt.figure()
    plt.bar(Xf, Pf)
    plt.xlabel("Final (best of k) score")
    plt.ylabel("Probability")
    plt.title(f"Long Jump (best of {args.k}) PMF  EV={muf:.3f}, SD={sdf:.3f}")
    plt.tight_layout()
    plt.savefig(args.final_pmf, dpi=150); print(f"Wrote {args.final_pmf}")

    # CDF for final
    acc=0.0; Fc=[]
    for x in Xf:
        acc += F_final_pmf[x]; Fc.append(acc)
    plt.figure()
    plt.plot(Xf, Fc, drawstyle="steps-post")
    plt.xlabel("Final (best of k) score")
    plt.ylabel("Cumulative probability")
    plt.title(f"Long Jump (best of {args.k}) CDF")
    plt.tight_layout()
    plt.savefig(args.final_cdf, dpi=150); print(f"Wrote {args.final_cdf}")
    if args.final_cdf_txt:
        dump_txt(args.final_cdf_txt, Xf, Fc, "# score\tcdf")
        print(f"Wrote {args.final_cdf_txt}")

if __name__ == "__main__":
    main()
