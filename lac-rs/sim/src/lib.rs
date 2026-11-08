//! `lac-sim`: the rules of Chest and Lever, deterministic and free of Bevy (AGENTS.md section 13).
//!
//! Same seed + same actions = the same run, bit for bit. No wall clock, no threads, no unseeded randomness,
//! no iteration over hash maps: every random draw comes from the run's seed through a named stream (`rng`).

pub mod content;
pub mod dice;
pub mod geom;
pub mod item;
pub mod levelgen;
pub mod monster;
pub mod party;
pub mod rng;
pub mod sim;
pub mod world;

pub use content::Content;
pub use geom::{Dir, IVec3};
pub use sim::{Action, Event, Outcome, Recording, RunOptions, Sim, State};
pub use world::{Cell, Feature, Level, Theme};
