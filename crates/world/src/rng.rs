use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn rng_for(world_seed: u64, tick: u64, salt: u64) -> ChaCha8Rng {
    let s = world_seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(tick.wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(salt);
    ChaCha8Rng::seed_from_u64(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn same_inputs_same_sequence() {
        let mut a = rng_for(42, 100, 7);
        let mut b = rng_for(42, 100, 7);
        for _ in 0..16 {
            let x: u64 = a.gen();
            let y: u64 = b.gen();
            assert_eq!(x, y);
        }
    }

    #[test]
    fn different_salt_differs() {
        let mut a = rng_for(42, 100, 7);
        let mut b = rng_for(42, 100, 8);
        let x: u64 = a.gen();
        let y: u64 = b.gen();
        assert_ne!(x, y);
    }
}
