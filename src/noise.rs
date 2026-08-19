//! Deterministic seed-driven jitter — one integer hash, no RNG state
//! and no allocation, so the same seed gives the same offset on every
//! frame and every run. Not a general-purpose PRNG and not
//! cryptographic: it exists to place stars and nudge glyphs.

/// Deterministically maps `seed` to an offset in `-spread/2 ..
/// spread/2` — the jitter four bundled apps use to place stars and
/// scatter glyphs without storing per-item randomness.
///
/// Same seed, same offset, every call: callers derive the seed from
/// something stable (an index, a grid position) and get a fixed layout
/// for free. The hash and its constants are load-bearing — changing
/// either moves every star in every app that already uses it.
pub fn scatter(seed: u32, spread: f32) -> f32 {
    let h = (seed.wrapping_mul(2_654_435_761)) ^ (seed.wrapping_mul(40_503).rotate_left(13));
    ((h % 10_000) as f32 / 10_000.0 - 0.5) * spread
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn scatter_is_deterministic_for_the_same_seed() {
        for seed in 0..1_000u32 {
            assert_eq!(
                scatter(seed, 10.0),
                scatter(seed, 10.0),
                "seed {seed} must map to one offset"
            );
        }
    }

    #[test]
    fn scatter_stays_within_half_the_spread() {
        for seed in 0..1_000u32 {
            let v = scatter(seed, 10.0);
            assert!(v.abs() <= 5.0, "seed {seed} gave {v}, outside +/- spread/2");
        }
    }

    /// The hash quantises to 10,000 buckets, so a large sample cannot
    /// be *fully* distinct — 256 samples collide about three times by
    /// the birthday bound. What matters is that distinct seeds spread
    /// out rather than clumping onto a handful of offsets.
    #[test]
    fn distinct_seeds_give_distinct_offsets() {
        let distinct: HashSet<u32> = (0..256u32).map(|s| scatter(s, 1_000.0).to_bits()).collect();
        assert!(
            distinct.len() >= 240,
            "only {} distinct offsets from 256 seeds",
            distinct.len()
        );
    }

    #[test]
    fn scatter_scales_linearly_with_spread() {
        for seed in 0..100u32 {
            assert_eq!(scatter(seed, 2.0), scatter(seed, 1.0) * 2.0);
        }
    }
}
