# Knizia Decathlon Optimal Policy Precomputation

This repository contains tools to compute and analyze **optimal strategies** for the events in *Reiner Knizia's Decathlon* dice game.  
It currently includes:

- **100m Sprint** — complete solver, database, analysis scripts, and interactive player.
- **Long Jump** — solver and analysis in progress, policy precomputation implemented.

The goal is to extend this framework to all ten Decathlon events.

---

## 🎯 Overview

The workflow for each event:

1. **Precompute** the optimal policy for every possible game state via exhaustive search / dynamic programming in **C++**.
2. **Store** the optimal decisions, expected values, and standard deviations in a compact **SQLite** database.
3. **Analyze** the resulting probability distributions (PMF and CDF) and summary statistics using **Python**.
4. **Interactively play** against the optimal policy engine.

This separation means:
- **C++ solver** runs once per event to generate the database.
- **Python tools** can instantly load and use that database without recomputing.

---

## 📂 Structure

```

.
├── analysis/
│   ├── analyze_100m_pmf.py         # Simple EV/SD analysis for 100m
│   ├── analyze_100m_pmf_cdf.py     # Full PMF + CDF plots/tables for 100m
│   ├── analyze_longjump_pmf_cdf.py # Full PMF + CDF plots/tables for Long Jump
│
├── players/
│   ├── 100m.py     # Interactive player for 100m
│   ├── longjump.py # Interactive player for Long Jump (WIP)
│
├── solvers/
│   ├── decathlon_100m_precompute.cpp # 100m C++ solver
│   ├── decathlon_100m_solver.py      # 100m pure-Python solver
│   ├── 100m_precompute               # compiled binary (ignored in git)
│   ├── 100m_policy.db                 # SQLite DB for 100m
│   ├── longjump_precompute.cpp        # Long Jump C++ solver
│   ├── longjump_precompute            # compiled binary (ignored in git)
│   ├── longjump_policy.db             # SQLite DB for Long Jump
│
├── setup_env.sh  # Quick setup script for Python venv
├── README.md

````

---

## ⚙️ Event Details

### 100m Sprint Rules Recap
- Roll 5 dice up to 3 times.
- After each roll, choose which dice to keep (“freeze”) and which to reroll.
- Score is **sum of frozen dice** after the third roll.
- Goal: maximize the total while minimizing variance from bad rolls.

**Solver approach**:
- Enumerates all possible (roll, frozen) states.
- Calculates the optimal choice at each state to maximize expected final score.
- Stores EV and SD for each state.

---

### Long Jump Rules Recap
- **5 dice**, 3 attempts per event.
- Each attempt:
  - **Run-up phase**: Roll remaining dice, freeze ≥1 die each roll, total frozen sum ≤ 8 or foul (0).
  - **Jump phase**: Roll frozen dice from run-up, freeze ≥1 die per roll until all are frozen.
- **Final score** = best of the three attempts.

**Solver approach**:
- Enumerates all states.
- In run-up: freeze smallest dice possible.
- In jump: freeze largest dice possible.
- Best-of-three logic adjusts strategy based on previous attempts.

---

## 🚀 Usage

### 1. Build a solver
Example for Long Jump:
```bash
g++ -O3 -std=c++20 solvers/longjump_precompute.cpp -lsqlite3 -o solvers/longjump_precompute
````

Example for 100m:

```bash
g++ -O3 -std=c++20 solvers/decathlon_100m_precompute.cpp -lsqlite3 -o solvers/100m_precompute
```

### 2. Generate policy database

```bash
./solvers/longjump_precompute solvers/longjump_policy.db
./solvers/100m_precompute solvers/100m_policy.db
```

### 3. Analyze distributions

Example for 100m:

```bash
python3 analysis/analyze_100m_pmf_cdf.py \
  --db solvers/100m_policy.db \
  --pmf-out 100m_pmf.png --pmf-csv 100m_pmf.csv \
  --cdf-out 100m_cdf.png --cdf-txt 100m_cdf.txt \
  --verbose
```

Example for Long Jump:

```bash
python3 analysis/analyze_longjump_pmf_cdf.py \
  --db solvers/longjump_policy.db \
  --attempt-pmf longjump_attempt_pmf.png \
  --attempt-cdf longjump_attempt_cdf.png --attempt-cdf-txt longjump_attempt_cdf.txt \
  --final-pmf longjump_final_pmf.png \
  --final-cdf longjump_final_cdf.png --final-cdf-txt longjump_final_cdf.txt \
  --verbose
```

---

## 🐍 Python Environment Setup

```bash
bash setup_env.sh
source .venv/bin/activate
```

`setup_env.sh` creates a Python virtual environment and installs:

* `matplotlib`
* `pandas`

---

## 📊 Example Outputs

* **PMF plots**: show probability of each score under optimal play.
* **CDF plots**: show probability of reaching at least a given score.
* **CSV/TXT tables**: numeric values for analysis or reference.

---

## 🔮 Roadmap

* [x] 100m Sprint — complete
* [ ] Long Jump — solver/analysis mostly done
* [ ] Shot Put
* [ ] High Jump
* [ ] 400m
* [ ] 110m Hurdles
* [ ] Discus
* [ ] Pole Vault
* [ ] Javelin
* [ ] 1500m