use serde::{Deserialize, Serialize};

/// 決定性虛擬亂數產生器（SplitMix64 初始化 + xoshiro256**）。
/// 不依賴外部 RNG crate，確保跨平台與跨版本輸出完全一致。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    s: [u64; 4],
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

impl Rng {
    /// 從 64 位元種子初始化 xoshiro256** 狀態。
    pub fn new(seed: u64) -> Self {
        let mut sm = seed;
        let s = [
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
        ];
        Self { s }
    }

    /// 產生下一個 64 位元無號整數。
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;

        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];

        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);

        result
    }

    /// 產生 `[0, n)` 範圍內的整數。
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0, "upper bound must be > 0");
        self.next_u64() % n
    }

    /// 以 Fisher-Yates 演算法就地洗牌。
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        let len = v.len();
        if len <= 1 {
            return;
        }
        for i in (1..len).rev() {
            let j = self.below(i as u64 + 1) as usize;
            v.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;
    use proptest::prelude::{prop_assert, proptest};

    #[test]
    fn same_seed_produces_identical_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_produce_different_sequences() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn seed_zero_regression_first_three_values() {
        let mut rng = Rng::new(0);
        assert_eq!(rng.next_u64(), 0x99ec_5f36_cb75_f2b4);
        assert_eq!(rng.next_u64(), 0xbf6e_1f78_4956_452a);
        assert_eq!(rng.next_u64(), 0x1a5f_849d_4933_e6e0);
    }

    #[test]
    fn shuffle_preserves_elements_and_is_deterministic() {
        let mut a: Vec<u32> = (0..52).collect();
        let mut b = a.clone();
        let mut r1 = Rng::new(123);
        let mut r2 = Rng::new(123);
        r1.shuffle(&mut a);
        r2.shuffle(&mut b);
        assert_eq!(a, b);
        assert_ne!(a, (0..52).collect::<Vec<_>>());
        a.sort_unstable();
        assert_eq!(a, (0..52).collect::<Vec<_>>());
    }

    proptest! {
        #[test]
        fn below_always_in_range(seed: u64, n in 1u64..1_000_000) {
            let mut rng = Rng::new(seed);
            for _ in 0..20 {
                prop_assert!(rng.below(n) < n);
            }
        }
    }
}
