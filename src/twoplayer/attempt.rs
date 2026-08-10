//! What a best-of-three driver needs from one attempt.
//!
//! The four throwing events differ only *inside* an attempt: discus and
//! javelin freeze dice of one parity, the shot put throws one die at a
//! time, the long jump has a run-up then a jump. All four then keep the
//! best of three interleaved attempts, so that part is written once
//! against this trait.

/// One attempt of a throwing event, solved against an arbitrary payoff.
///
/// `payoff` is indexed by the attempt's score, `0..=max_score`, with a
/// foul reading `payoff[0]`. `floor` is the mover's banked best: the
/// payoff is flat at or below it, which lets an implementation prune
/// positions that can no longer improve on what is already in hand.
pub trait AttemptEngine: Sync {
    /// Highest score the attempt can produce.
    fn max_score(&self) -> usize;
    /// Decision nodes a policy must address.
    fn nodes(&self) -> usize;
    /// Which scores the attempt can produce, indexed by score.
    fn reachable_scores(&self) -> &[bool];
    /// Nodes still worth storing given a banked best of `floor`.
    fn live_nodes(&self, floor: usize) -> usize;
    /// Best achievable `E[payoff(score)]`.
    fn expected(&self, payoff: &[f64], floor: usize) -> f64;
    /// As [`AttemptEngine::expected`], recording the action at each node.
    fn expected_with_policy(
        &self,
        payoff: &[f64],
        floor: usize,
        policy: &mut [u8],
    ) -> f64;
    /// As [`AttemptEngine::expected`], also flagging where following
    /// `baseline` would cost more than [`super::EPS`]. Where it would
    /// not, `chosen` copies the baseline, so ties cost nothing to store.
    fn expected_vs_baseline(
        &self,
        payoff: &[f64],
        floor: usize,
        baseline: &[u8],
        chosen: &mut [u8],
        deviates: &mut [bool],
    ) -> f64;
}
