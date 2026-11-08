//! Random numbers: one run seed, split into named streams.
//!
//! REF R25: runs are seeded and the generator is a Mersenne Twister. We implement MT19937 here (rather than
//! the `rand_mt` crate the board suggested) so its whole state is plain data that saves, loads and hashes
//! with the rest of the run.
//! REF R44: levels come from the seed, so one seed is one dungeon.
//!
//! OURS: the seed is any text (trimmed, case-folded, hashed with FNV-1a 64), so runs can be shared by name.
//! Each system draws from its own stream, keyed by the run seed and the stream's name, so adding a draw in
//! one system never shifts another.

use serde::{Deserialize, Serialize};

const N: usize = 624;
const M: usize = 397;

/// MT19937, the 32-bit Mersenne Twister of Matsumoto and Nishimura (1998).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mt19937 {
    mt: Vec<u32>,
    idx: usize,
}

impl Mt19937 {
    /// `init_genrand` of the reference implementation.
    pub fn new(seed: u32) -> Self {
        let mut mt = vec![0u32; N];
        mt[0] = seed;
        for i in 1..N {
            mt[i] = 1_812_433_253u32
                .wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        Mt19937 { mt, idx: N }
    }

    /// `init_by_array` of the reference implementation.
    pub fn from_key(key: &[u32]) -> Self {
        let mut s = Mt19937::new(19_650_218);
        let mt = &mut s.mt;
        let (mut i, mut j) = (1usize, 0usize);
        let len = key.len().max(1);
        for _ in 0..N.max(len) {
            let prev = mt[i - 1] ^ (mt[i - 1] >> 30);
            mt[i] = (mt[i] ^ prev.wrapping_mul(1_664_525))
                .wrapping_add(*key.get(j).unwrap_or(&0))
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                mt[0] = mt[N - 1];
                i = 1;
            }
            if j >= len {
                j = 0;
            }
        }
        for _ in 0..N - 1 {
            let prev = mt[i - 1] ^ (mt[i - 1] >> 30);
            mt[i] = (mt[i] ^ prev.wrapping_mul(1_566_083_941)).wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                mt[0] = mt[N - 1];
                i = 1;
            }
        }
        mt[0] = 0x8000_0000;
        s
    }

    fn twist(&mut self) {
        for i in 0..N {
            let y = (self.mt[i] & 0x8000_0000) | (self.mt[(i + 1) % N] & 0x7fff_ffff);
            let mut v = self.mt[(i + M) % N] ^ (y >> 1);
            if y & 1 != 0 {
                v ^= 0x9908_b0df;
            }
            self.mt[i] = v;
        }
        self.idx = 0;
    }

    pub fn next_u32(&mut self) -> u32 {
        if self.idx >= N {
            self.twist();
        }
        let mut y = self.mt[self.idx];
        self.idx += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// Uniform in `0..n` without modulo bias. `n` must be above 0.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0, "below(0)");
        let zone = u32::MAX - (u32::MAX % n);
        loop {
            let r = self.next_u32();
            if r < zone {
                return r % n;
            }
        }
    }

    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + self.below((hi - lo + 1) as u32) as i32
    }

    /// True `pct` times in 100.
    pub fn chance(&mut self, pct: u32) -> bool {
        self.below(100) < pct
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.below(items.len() as u32) as usize])
        }
    }

    /// Index into `weights` chosen by weight; `None` if all are zero.
    pub fn weighted(&mut self, weights: &[u32]) -> Option<usize> {
        let total: u32 = weights.iter().sum();
        if total == 0 {
            return None;
        }
        let mut r = self.below(total);
        for (i, w) in weights.iter().enumerate() {
            if r < *w {
                return Some(i);
            }
            r -= w;
        }
        None
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u32 + 1) as usize;
            items.swap(i, j);
        }
    }
}

/// FNV-1a, 64 bit: the hash we use for seeds, stream names and the replay state hash.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The run seed from its text. OURS: case and surrounding spaces don't matter.
pub fn seed_from_text(text: &str) -> u64 {
    fnv1a64(text.trim().to_uppercase().as_bytes())
}

/// The stream `name` of the run with this seed.
pub fn stream(seed: u64, name: &str) -> Mt19937 {
    let h = fnv1a64(name.as_bytes());
    Mt19937::from_key(&[seed as u32, (seed >> 32) as u32, h as u32, (h >> 32) as u32])
}

/// The named streams a run keeps. Level generation makes a fresh stream per depth (`levelgen.<depth>`)
/// so the layout of depth 3 never depends on what happened on depth 2.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Streams {
    /// Loot in chests and on floors, and the run's unidentified appearances.
    pub loot: Mt19937,
    /// Monster choices: wandering, targets, spawns after the level is made.
    pub ai: Mt19937,
    /// To-hit and damage rolls, both ways.
    pub combat: Mt19937,
    /// Everything else in the world: traps, fountains, searching, lock picking, teleports.
    pub world: Mt19937,
}

impl Streams {
    pub fn new(seed: u64) -> Self {
        Streams {
            loot: stream(seed, "loot"),
            ai: stream(seed, "ai"),
            combat: stream(seed, "combat"),
            world: stream(seed, "world"),
        }
    }
}

/// Random seed text made of two words and a number, for "roll a seed" in the menu. The caller gives the
/// entropy (the app uses the clock); the sim itself never reads a clock.
pub fn seed_words(entropy: u64) -> String {
    const A: [&str; 16] = [
        "AMBER", "BRASS", "COLD", "DUSK", "EMBER", "FERN", "GRIM", "HOLLOW", "IRON", "JADE", "LOW", "MOSS",
        "PALE", "RUST", "SILT", "WAX",
    ];
    const B: [&str; 16] = [
        "ADDER", "BELL", "CAIRN", "DOOR", "EEL", "FLASK", "GATE", "HINGE", "KEY", "LAMP", "MOTH", "NAIL",
        "OWL", "RAVEN", "SPOOL", "TOMB",
    ];
    let mut r = Mt19937::from_key(&[entropy as u32, (entropy >> 32) as u32]);
    format!(
        "{}-{}-{}",
        A[r.below(16) as usize],
        B[r.below(16) as usize],
        r.range(10, 99)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_reference_init_genrand() {
        // The reference implementation's default seed 5489 gives 3499211612 first.
        let mut r = Mt19937::new(5489);
        assert_eq!(r.next_u32(), 3_499_211_612);
        assert_eq!(r.next_u32(), 581_869_302);
        assert_eq!(r.next_u32(), 3_890_346_734);
    }

    #[test]
    fn matches_reference_init_by_array() {
        // mt19937ar.out: init_by_array({0x123, 0x234, 0x345, 0x456}) gives 1067595299 955945823 477289528.
        let mut r = Mt19937::from_key(&[0x123, 0x234, 0x345, 0x456]);
        assert_eq!(r.next_u32(), 1_067_595_299);
        assert_eq!(r.next_u32(), 955_945_823);
        assert_eq!(r.next_u32(), 477_289_528);
    }

    #[test]
    fn streams_are_independent_and_fixed() {
        let seed = seed_from_text("  amber-door-42 ");
        assert_eq!(seed, seed_from_text("AMBER-DOOR-42"));
        let mut a = stream(seed, "loot");
        let mut b = stream(seed, "ai");
        let a1 = a.next_u32();
        assert_ne!(a1, b.next_u32());
        // Drawing from `ai` never moves `loot`.
        let mut a2 = stream(seed, "loot");
        let mut b2 = stream(seed, "ai");
        for _ in 0..100 {
            b2.next_u32();
        }
        assert_eq!(a2.next_u32(), a1);
    }

    #[test]
    fn below_stays_in_range() {
        let mut r = Mt19937::new(1);
        for n in 1..50 {
            for _ in 0..50 {
                assert!(r.below(n) < n);
            }
        }
        assert_eq!(r.range(3, 3), 3);
    }
}
