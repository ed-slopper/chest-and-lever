//! Dice: `NdS+P`, written as text in content files ("2d6+1", "1d4", "3").
//!
//! G9a (decided by ed-slopper, 2026-11-08): combat uses dice in the D&D way, after the early pitch's
//! "D20s" (R16).

use crate::rng::Mt19937;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Dice {
    pub n: u8,
    pub sides: u16,
    pub plus: i16,
}

impl Dice {
    pub const fn new(n: u8, sides: u16, plus: i16) -> Self {
        Dice { n, sides, plus }
    }
    pub const fn flat(v: i16) -> Self {
        Dice { n: 0, sides: 0, plus: v }
    }
    pub fn roll(&self, r: &mut Mt19937) -> i32 {
        let mut t = self.plus as i32;
        if self.sides > 0 {
            for _ in 0..self.n {
                t += 1 + r.below(self.sides as u32) as i32;
            }
        }
        t
    }
    pub fn max(&self) -> i32 {
        self.n as i32 * self.sides as i32 + self.plus as i32
    }
    pub fn average(&self) -> f32 {
        self.n as f32 * (self.sides as f32 + 1.0) / 2.0 + self.plus as f32
    }
}

impl TryFrom<String> for Dice {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        Dice::parse(&s)
    }
}

impl From<Dice> for String {
    fn from(d: Dice) -> String {
        d.to_string()
    }
}

impl Dice {
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        let err = || format!("bad dice {s:?}, want like 2d6+1");
        let Some((n, rest)) = s.split_once('d') else {
            return s.parse::<i16>().map(Dice::flat).map_err(|_| err());
        };
        let n: u8 = if n.is_empty() { 1 } else { n.parse().map_err(|_| err())? };
        let (sides, plus) = if let Some((a, b)) = rest.split_once('+') {
            (a, b.parse::<i16>().map_err(|_| err())?)
        } else if let Some((a, b)) = rest.split_once('-') {
            (a, -b.parse::<i16>().map_err(|_| err())?)
        } else {
            (rest, 0)
        };
        let sides: u16 = sides.parse().map_err(|_| err())?;
        Ok(Dice { n, sides, plus })
    }
}

impl fmt::Display for Dice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.n == 0 || self.sides == 0 {
            return write!(f, "{}", self.plus);
        }
        write!(f, "{}d{}", self.n, self.sides)?;
        match self.plus.cmp(&0) {
            std::cmp::Ordering::Greater => write!(f, "+{}", self.plus),
            std::cmp::Ordering::Less => write!(f, "{}", self.plus),
            std::cmp::Ordering::Equal => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_print() {
        for s in ["2d6+1", "1d4", "3", "1d8-1", "-2"] {
            assert_eq!(Dice::parse(s).unwrap().to_string(), s);
        }
        assert_eq!(Dice::parse("d6").unwrap(), Dice::new(1, 6, 0));
        assert!(Dice::parse("xd6").is_err());
    }

    #[test]
    fn rolls_in_range() {
        let mut r = Mt19937::new(7);
        let d = Dice::new(3, 6, 2);
        for _ in 0..200 {
            let v = d.roll(&mut r);
            assert!((5..=20).contains(&v));
        }
    }
}
