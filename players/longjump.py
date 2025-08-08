#!/usr/bin/env python3
"""
Interactive player for Long Jump in Knizia's Dice Decathlon.

Reads optimal policy from solvers/longjump_policy.db (from longjump_precompute.cpp).
Lets the human roll/freeze or stop, with optional hints from the engine.

Usage:
    python3 -m players.longjump [--db solvers/longjump_policy.db] [--hint]

Controls:
    On run-up: choose dice to freeze or 'stop' to end run-up.
    On jump: choose dice to freeze until all are frozen.
    You can enter faces separated by spaces (e.g. "5 6" to freeze a 5 and 6),
    or 'stop' (run-up only), or 'all' to freeze everything.
"""
import argparse
import sqlite3
import random

RUNUP_PRE=0; RUNUP_POST=1; JUMP_PRE=2; JUMP_POST=3

def roll_dice(n):
    return [random.randint(1, 6) for _ in range(n)]

def counts_from_dice(dice):
    cnt = {i: 0 for i in range(1, 7)}
    for d in dice:
        cnt[d] += 1
    return cnt

def fetch_decision(cur, phase, sum_frozen, cnt):
    row = cur.execute(
        """SELECT f1,f2,f3,f4,f5,f6 FROM lj_post
           WHERE phase=? AND (sum_frozen IS ? OR sum_frozen=?) AND
                 n1=? AND n2=? AND n3=? AND n4=? AND n5=? AND n6=?""",
        (phase,
         None if phase == JUMP_POST else None,
         None if phase == JUMP_POST else sum_frozen,
         cnt[1], cnt[2], cnt[3], cnt[4], cnt[5], cnt[6])
    ).fetchone()
    if row is None:
        raise RuntimeError(f"No policy for phase={phase}, s={sum_frozen}, cnt={cnt}")
    return {i: row[i-1] for i in range(1, 7)}

def freeze_dice(dice, freeze_faces, freeze_counts):
    """Freeze according to chosen counts per face."""
    frozen = []
    remaining = []
    counts_needed = freeze_counts.copy()
    for d in dice:
        if counts_needed.get(d, 0) > 0:
            frozen.append(d)
            counts_needed[d] -= 1
        else:
            remaining.append(d)
    return frozen, remaining

def run_longjump(cur, show_hint=False):
    # Run-up phase
    frozen_runup = []
    sum_runup = 0
    remaining = 5
    while True:
        dice = roll_dice(remaining)
        print(f"Run-up roll: {dice}  (sum frozen={sum_runup})")
        cnt = counts_from_dice(dice)
        if show_hint:
            hint = fetch_decision(cur, RUNUP_POST, sum_runup, cnt)
            print(f"[HINT] Freeze: {hint}")
        choice = input("Freeze dice (faces) or 'stop': ").strip().lower()
        if choice == 'stop':
            break
        if choice == 'all':
            to_freeze = dice.copy()
        else:
            try:
                to_freeze = list(map(int, choice.split()))
            except ValueError:
                print("Invalid input.")
                continue
        fc = {i: 0 for i in range(1, 7)}
        for v in to_freeze:
            if v in fc:
                fc[v] += 1
        frozen, remaining_dice = freeze_dice(dice, to_freeze, fc)
        frozen_runup.extend(frozen)
        sum_runup += sum(frozen)
        remaining = len(remaining_dice)
        if sum_runup > 8:
            print("Foul! Sum in run-up exceeded 8.")
            return 0
        if remaining == 0:
            break

    # Jump phase
    jump_score = 0
    remaining = len(frozen_runup)  # jump dice count = # frozen in run-up
    print(f"Jump phase: rolling {remaining} dice")
    while remaining > 0:
        dice = roll_dice(remaining)
        print(f"Jump roll: {dice}")
        cnt = counts_from_dice(dice)
        if show_hint:
            hint = fetch_decision(cur, JUMP_POST, None, cnt)
            print(f"[HINT] Freeze: {hint}")
        choice = input("Freeze dice (faces) or 'all': ").strip().lower()
        if choice == 'all':
            to_freeze = dice.copy()
        else:
            try:
                to_freeze = list(map(int, choice.split()))
            except ValueError:
                print("Invalid input.")
                continue
        fc = {i: 0 for i in range(1, 7)}
        for v in to_freeze:
            if v in fc:
                fc[v] += 1
        frozen, remaining_dice = freeze_dice(dice, to_freeze, fc)
        jump_score += sum(frozen)
        remaining = len(remaining_dice)

    total_score = sum_runup + jump_score
    print(f"Attempt score: {total_score}")
    return total_score

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", default="solvers/longjump_policy.db")
    ap.add_argument("--hint", action="store_true", help="Show engine's optimal freeze each step")
    args = ap.parse_args()
    conn = sqlite3.connect(args.db)
    cur = conn.cursor()
    print("=== Long Jump ===")
    scores = []
    for attempt in range(1, 4):  # best of 3
        print(f"\nAttempt {attempt} of 3")
        score = run_longjump(cur, show_hint=args.hint)
        scores.append(score)
    best = max(scores)
    print(f"\nYour scores: {scores}, Best: {best}")
    conn.close()

if __name__ == "__main__":
    main()
