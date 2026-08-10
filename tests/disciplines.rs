//! Integration tests pinning each discipline's distribution and a few
//! independently checkable expected values.

#![allow(clippy::cast_precision_loss)]

use dice_decathlon::disciplines;

/// Every discipline's final-score distribution must be a valid PMF.
#[test]
fn all_pmfs_sum_to_one() {
    for (key, solve) in disciplines::registry() {
        let solved = solve();
        let total: f64 = solved.dist.mass.values().sum();
        assert!((total - 1.0).abs() < 1e-6, "{key} PMF sums to {total}");
    }
}

/// Hurdles is a 1-D optimal-stopping problem: keep the current sum of
/// five dice or rethrow all five, up to five times. The value can be
/// recomputed directly as W(r) = E[max(s, W(r-1))], W(0) = E[s].
#[test]
fn hurdles_matches_independent_dp() {
    // Distribution of the sum of five dice.
    let mut counts = vec![0u64; 31];
    for a in 1..=6 {
        for b in 1..=6 {
            for c in 1..=6 {
                for d in 1..=6 {
                    for e in 1..=6 {
                        counts[a + b + c + d + e] += 1;
                    }
                }
            }
        }
    }
    let total = 6f64.powi(5);
    let dist: Vec<(f64, f64)> = counts
        .iter()
        .enumerate()
        .filter(|(_, &n)| n > 0)
        .map(|(s, &n)| (s as f64, n as f64 / total))
        .collect();

    let mut w: f64 = dist.iter().map(|&(s, p)| s * p).sum();
    for _ in 0..5 {
        w = dist.iter().map(|&(s, p)| s.max(w) * p).sum();
    }

    let solved = disciplines::hurdles::solve();
    assert!(
        (solved.dist.mean() - w).abs() < 1e-9,
        "hurdles EV {} != independent {w}",
        solved.dist.mean()
    );
}

/// Our optimal Long Jump must beat the old heuristic solver's reported
/// start value of ~20.41.
#[test]
fn longjump_beats_old_heuristic() {
    let ev = disciplines::longjump::solve().dist.mean();
    assert!(ev >= 20.41, "long jump EV {ev} below heuristic 20.41");
}

/// Every discipline's expected value under optimal play, pinned against
/// an independent brute force written from the rulebook alone (see
/// `worklog/2026-08-09-rules-fidelity-and-web-ui/`). These are not
/// snapshots of our own output: each was reproduced to nine decimal
/// places by a separate implementation that shares no code with the
/// solvers, so a regression here means a real behaviour change.
#[test]
fn expected_values_match_independent_brute_force() {
    let expected: &[(&str, f64)] = &[
        ("100m", 23.997_512_229),
        ("longjump", 22.394_956_800),
        ("shotput", 18.634_491_985),
        ("highjump", 19.263_445_441),
        ("400m", 25.667_993_300),
        ("110mh", 21.375_403_087),
        ("discus", 22.317_089_285),
        ("polevault", 17.277_634_238),
        ("javelin", 22.251_507_501),
        ("1500m", 26.528_791_924),
    ];

    for &(key, want) in expected {
        let solver = disciplines::find(key).expect("registered discipline");
        let got = solver().dist.mean();
        assert!(
            (got - want).abs() < 1e-8,
            "{key} EV {got} != independently derived {want}"
        );
    }
}

/// 100m spans every set of eight dice: all sixes (-48) to all fives.
#[test]
fn hundred_metres_score_range() {
    let solved = disciplines::m100::solve();
    assert_eq!(*solved.dist.mass.keys().next().unwrap(), -48);
    assert_eq!(*solved.dist.mass.keys().next_back().unwrap(), 40);
}
