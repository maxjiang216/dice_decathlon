# python -m players.100m
import sqlite3, random
from pathlib import Path

DB_PATH = Path(__file__).resolve().parents[1] / "solvers" / "100m_policy.db"

def score_set(d):  # for printing only
    return sum(v if v < 6 else -6 for v in d)

def lookup(cur, stage, rerolls, dice, set1_score):
    d = tuple(sorted(dice))
    row = cur.execute(
        """SELECT ev_freeze, sd_freeze, ev_reroll, sd_reroll, best
             FROM states100m
            WHERE stage=? AND rerolls=? AND d1=? AND d2=? AND d3=? AND d4=? AND
                  (set1_score IS ? OR set1_score = ?)""",
        (stage, rerolls, d[0], d[1], d[2], d[3],
         None if stage==1 else None, None if stage==1 else set1_score)
    ).fetchone()
    if row is None:
        raise RuntimeError(f"State not found: stage={stage}, rerolls={rerolls}, dice={d}, set1={set1_score}")
    evF, sdF, evR, sdR, best = row
    acts = {"freeze": (evF, sdF)}
    if evR is not None:
        acts["reroll"] = (evR, sdR)
    return best, acts

def fmt_acts(best, acts):
    lines=[]
    for k,(ev,sd) in acts.items():
        star = " <-- best" if k==best else ""
        lines.append(f"  {k.upper():6s}: EV={ev:6.3f}, SD={sd:6.3f}{star}")
    return "\n".join(lines)

def interactive(play_auto=False, seed=None):
    rng = random.Random(seed)
    with sqlite3.connect(DB_PATH) as conn:
        cur = conn.cursor()
        rerolls = 5
        # stage 1 initial roll
        dice = tuple(sorted(rng.choices([1,2,3,4,5,6], k=4)))
        stage = 1
        set1 = None

        print("=== 100 Metres — precomputed policy ===")
        print("(1..5 = face value; 6 = -6) | 5 shared rerolls\n")

        while True:
            if stage == 1:
                print(f"[SET 1] Dice: {dice} | score-if-freeze={score_set(dice):2d} | rerolls={rerolls}")
                best, acts = lookup(cur, 1, rerolls, dice, None)
            else:
                print(f"[SET 2] Dice: {dice} | score-if-freeze={score_set(dice):2d} | rerolls={rerolls} | set1={set1}")
                best, acts = lookup(cur, 2, rerolls, dice, set1)

            print(fmt_acts(best, acts))

            if play_auto:
                choice = best
                print(f"Engine chooses: {choice}")
            else:
                ch = input("Choose [f]reeze / [r]eroll / [b]est / [q]uit: ").strip().lower()
                if ch == "q": print("Bye"); return
                if ch == "b": choice = best
                elif ch == "f": choice = "freeze"
                elif ch == "r": choice = "reroll"
                else: print("Invalid -> best"); choice = best

            if choice == "freeze":
                if stage == 1:
                    set1 = score_set(dice)
                    dice = tuple(sorted(rng.choices([1,2,3,4,5,6], k=4)))
                    stage = 2
                else:
                    total = set1 + score_set(dice)
                    print(f"\nFINAL: set1={set1}, set2={score_set(dice)}, total={total}")
                    return
            else:
                if rerolls == 0:
                    print("No rerolls left; forced freeze.")
                    if stage == 1:
                        set1 = score_set(dice)
                        dice = tuple(sorted(rng.choices([1,2,3,4,5,6], k=4)))
                        stage = 2
                    else:
                        total = set1 + score_set(dice)
                        print(f"\nFINAL: set1={set1}, set2={score_set(dice)}, total={total}")
                        return
                else:
                    rerolls -= 1
                    dice = tuple(sorted(rng.choices([1,2,3,4,5,6], k=4)))

if __name__ == "__main__":
    interactive(play_auto=False)
