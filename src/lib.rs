//! Optimal expected-value dynamic-programming solvers for the ten
//! disciplines of Reiner Knizia's Decathlon dice game.
//!
//! Each discipline is solved as a single-player game whose objective is
//! to maximise the expected value of that event's own score (the
//! multiplayer turn order and championship medals are out of scope).
//! Every solver returns the exact distribution of the final score under
//! optimal play, from which expected value, standard deviation and the
//! CDF are derived.

// Dice counts and scores are tiny integers, so the numeric casts in the
// probability arithmetic are always exact in practice; the pedantic cast
// lints add noise without catching real problems here.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::must_use_candidate
)]
// Indexing dice arrays by face value (1..=6) reads more clearly than an
// iterator; building small SVG/CSV strings with `push_str(&format!())`
// and plain float arithmetic is clearer than `write!`/`mul_add` here.
#![allow(
    clippy::needless_range_loop,
    clippy::format_push_string,
    clippy::suboptimal_flops
)]

pub mod analysis;
pub mod dice;
pub mod disciplines;
pub mod dp;
pub mod policy;
