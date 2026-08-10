//! A small seeded PRNG.
//!
//! The engine is compiled to WebAssembly, where the `rand` crate needs a
//! `getrandom` backend wired up to the host. Rather than take that
//! dependency we carry a PCG-XSH-RR generator and let the caller supply
//! the seed (the browser passes one from `crypto.getRandomValues`). A
//! caller-supplied seed also makes any game exactly reproducible, which
//! is what lets a replay be pasted into a bug report.

/// PCG-XSH-RR 64/32: 64-bit state, 32-bit output.
pub struct Rng {
    state: u64,
    inc: u64,
}

const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

impl Rng {
    /// Create a generator from a seed. Any seed is valid.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: (seed << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULTIPLIER).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// A uniform integer in `0..n`. Rejection sampling keeps the
    /// distribution exact rather than biasing the low faces via `%`.
    fn below(&mut self, n: u32) -> u32 {
        let threshold = n.wrapping_neg() % n;
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return r % n;
            }
        }
    }

    /// Roll one six-sided die, returning a face in `1..=6`.
    pub fn roll(&mut self) -> u8 {
        (self.below(6) + 1) as u8
    }

    /// Roll `n` dice.
    pub fn roll_n(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.roll()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_are_in_range() {
        let mut rng = Rng::new(12345);
        for _ in 0..10_000 {
            let f = rng.roll();
            assert!((1..=6).contains(&f));
        }
    }

    #[test]
    fn same_seed_gives_same_sequence() {
        let a: Vec<u8> = Rng::new(7).roll_n(64);
        let b: Vec<u8> = Rng::new(7).roll_n(64);
        assert_eq!(a, b);
    }

    #[test]
    fn faces_are_roughly_uniform() {
        let mut rng = Rng::new(99);
        let mut counts = [0u32; 7];
        for _ in 0..60_000 {
            counts[rng.roll() as usize] += 1;
        }
        // 10_000 expected per face; a 15% band is far outside noise but
        // still catches a modulo bias or an off-by-one in `below`.
        for c in &counts[1..=6] {
            assert!((8_500..11_500).contains(c), "face counts {counts:?}");
        }
    }
}
