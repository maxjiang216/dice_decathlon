# Knizia Decathlon Optimal Policy Precomputation

This repository contains tools to compute and analyze **optimal strategies** for the events in *Reiner Knizia's Decathlon* dice game.  
It currently includes a full implementation for the **Long Jump** event (best-of-three attempts), with plans to extend to all ten events.

## 🎯 Overview

The core idea is:
1. **Precompute** the optimal policy for each event via exhaustive search / dynamic programming in **C++**.
2. **Store** the policy and metadata in a compact **SQLite** database.
3. **Analyze** the resulting probability distributions (PMF/CDF) and expected values using **Python**.

This makes gameplay simulation and interactive play instantaneous — all heavy computation is done **once** at precomputation time.

---

## 📂 Structure

```

.
├── analysis/
│   ├── analyze\_longjump\_pmf\_cdf.py   # PMF/CDF analysis and visualization
│   └── ... (future event analysis scripts)
├── players/
│   └── longjump\_player.py            # Python interactive player using precomputed policy
├── solvers/
│   └── longjump\_precompute\_best3\_simple.cpp  # C++ solver (freeze-count version)
│   └── ... (future event solvers)
└── README.md

````

---

## ⚙️ How It Works

### Long Jump Rules Recap
- **5 dice**, 3 attempts per game.
- Each attempt has:
  - **Run-up phase**: Roll all remaining dice, freeze ≥1 die each roll, keeping total frozen sum ≤ 8.  
    If sum would exceed 8, the attempt scores **0** (foul).
  - **Jump phase**: Roll the frozen dice from the run-up. Freeze ≥1 die per roll until all are frozen.  
    Score = sum of all frozen dice in jump.
- **Final event score** = **best** of the three attempt scores.

### Precomputation Process
- Enumerates all possible post-roll states.
- Uses **dynamic programming** to find the optimal number of dice to freeze at each step:
  - In **run-up**, freeze smallest dice possible.
  - In **jump**, freeze largest dice possible.
- For best-of-three:
  - Adjusts risk dynamically — later attempts play riskier if a good score already exists.
- Saves the policy and EV/SD into a **SQLite** database.

---

## 🚀 Usage

### 1. Build the solver
```bash
g++ -O3 -std=c++20 solvers/longjump_precompute_best3_simple.cpp -lsqlite3 -o longjump_precompute
````

### 2. Run the solver

```bash
./longjump_precompute solvers/longjump_policy.db
```

This creates `longjump_policy.db` with:

* **`lj_post`** — optimal decisions for each post-roll state
* **`lj_meta`** — attempt EV and standard deviation

### 3. Analyze the policy

```bash
# Activate Python environment (see below)
python3 analysis/analyze_longjump_pmf_cdf.py \
  --db solvers/longjump_policy.db \
  --attempt-pmf longjump_attempt_pmf.png \
  --attempt-cdf longjump_attempt_cdf.png --attempt-cdf-txt longjump_attempt_cdf.txt \
  --final-pmf longjump_final_pmf.png \
  --final-cdf longjump_final_cdf.png --final-cdf-txt longjump_final_cdf.txt \
  --verbose
```

Generates:

* PMF and CDF plots (`.png`)
* Tabulated CDF values (`.txt`)
* Summary statistics (EV, SD, support size)

---

## 🐍 Python Environment Setup

For plotting (`matplotlib`) and CSV export (`pandas`):

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install matplotlib pandas sqlite3
```

---

## 📊 Example Outputs

**Final Score PMF** — distribution of final event scores under optimal policy.
**Final Score CDF** — probability of achieving at least a given score.

*(Screenshots/plots will go here once generated.)*

---

## 🔮 Roadmap

* [X] 100m Sprint
* [] Long Jump
* [ ] Shot Put
* [ ] High Jump
* [ ] 400m
* [ ] 110m Hurdles
* [ ] Discus
* [ ] Pole Vault
* [ ] Javelin
* [ ] 1500m