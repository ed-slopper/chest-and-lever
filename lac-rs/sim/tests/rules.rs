//! The rules against the record: R20-R23 and R27 on the code-built test level, plus whole-run properties.

use lac_sim::sim::{MoveKind, PACK_SIZE};
use lac_sim::world::Feature;
use lac_sim::{levelgen, Action, Content, Dir, Event, IVec3, Level, Outcome, RunOptions, Sim};

fn test_sim() -> Sim {
    let mut s = Sim::on_level(Level::test_level(), Content::builtin());
    // no monsters on the test level
    s.state.level.monsters.clear();
    s
}

#[test]
fn time_stands_still_without_actions() {
    // REF R27
    let mut s = test_sim();
    assert!(s.step().is_none());
    assert_eq!(s.state.tick, 0);
    s.push(Action::Wait);
    s.push(Action::Wait);
    assert_eq!(s.queued(), 2);
    assert!(s.step().is_some());
    assert!(s.step().is_some());
    assert!(s.step().is_none());
    assert_eq!(s.state.tick, 2);
}

#[test]
fn one_tile_per_step_and_walls_block() {
    // REF R20, R23
    let mut s = test_sim();
    assert_eq!(s.state.pos, IVec3::new(5, 1, 5));
    s.tick(Action::StepForward);
    assert_eq!(s.state.pos, IVec3::new(5, 1, 4));
    // the ledge is one tile up: a plain step bumps and takes no time
    let t = s.state.tick;
    let ev = s.tick(Action::StepForward);
    assert!(ev.contains(&Event::Bump));
    assert_eq!(s.state.pos, IVec3::new(5, 1, 4));
    assert_eq!(s.state.tick, t);
    s.tick(Action::TurnRight);
    assert_eq!(s.state.facing, Dir::East);
    // facing east, a step left is north (into the ledge), a step right is south
    assert!(s.tick(Action::StepLeft).contains(&Event::Bump));
    s.tick(Action::StepRight);
    assert_eq!(s.state.pos, IVec3::new(5, 1, 5));
}

#[test]
fn jump_then_step_lands_on_the_ledge() {
    // REF R21: up one tile, one tick of grace, a step queued right after lands on the ledge
    let mut s = test_sim();
    s.tick(Action::StepForward);
    let ev = s.tick(Action::Jump);
    assert!(ev.iter().any(|e| matches!(e, Event::PartyMoved { kind: MoveKind::Jump, .. })));
    assert_eq!(s.state.pos, IVec3::new(5, 2, 4));
    s.tick(Action::StepForward);
    assert_eq!(s.state.pos, IVec3::new(5, 2, 3), "landed on the ledge");
    // and stays there
    s.tick(Action::Wait);
    assert_eq!(s.state.pos, IVec3::new(5, 2, 3));
}

#[test]
fn jump_without_a_move_falls_back() {
    // REF R21: the grace is one tick only
    let mut s = test_sim();
    s.tick(Action::Jump);
    assert_eq!(s.state.pos, IVec3::new(5, 2, 5));
    let ev = s.tick(Action::Wait);
    assert!(ev.iter().any(|e| matches!(e, Event::PartyMoved { kind: MoveKind::Fall, .. })));
    assert_eq!(s.state.pos, IVec3::new(5, 1, 5));
}

#[test]
fn long_jump_crosses_a_one_tile_gap_and_a_step_falls_in() {
    // REF R22
    let mut s = test_sim();
    s.state.pos = IVec3::new(5, 1, 6);
    s.state.facing = Dir::South;
    s.tick(Action::LongJump);
    assert_eq!(s.state.pos, IVec3::new(5, 1, 8));
    // back, and this time step into the gap
    s.state.pos = IVec3::new(5, 1, 6);
    s.tick(Action::StepForward);
    assert_eq!(s.state.pos, IVec3::new(5, 0, 7), "fell into the gap");
    // and climb out: jump, then step
    s.tick(Action::Jump);
    s.tick(Action::StepForward);
    assert_eq!(s.state.pos, IVec3::new(5, 1, 8));
}

#[test]
fn a_two_high_wall_cannot_be_climbed() {
    let mut s = test_sim();
    s.state.pos = IVec3::new(8, 1, 9);
    s.state.facing = Dir::East;
    s.tick(Action::Jump);
    let ev = s.tick(Action::StepForward);
    assert!(ev.contains(&Event::Bump));
    s.tick(Action::Wait);
    assert_eq!(s.state.pos, IVec3::new(8, 1, 9));
}

#[test]
fn same_seed_same_actions_same_state() {
    let actions = [
        Action::StepForward,
        Action::TurnLeft,
        Action::StepForward,
        Action::Attack,
        Action::Jump,
        Action::StepForward,
        Action::Use,
        Action::Wait,
        Action::TurnRight,
        Action::LongJump,
    ];
    let run = |seed: &str| {
        let mut s = Sim::new(seed, RunOptions::default(), Content::builtin());
        for a in actions.iter().cycle().take(300) {
            s.tick(*a);
        }
        s.state_hash()
    };
    assert_eq!(run("AMBER-DOOR-42"), run("AMBER-DOOR-42"));
    assert_eq!(run("AMBER-DOOR-42"), run("amber-door-42"));
    assert_ne!(run("AMBER-DOOR-42"), run("AMBER-DOOR-43"));
}

#[test]
fn every_level_generates_with_a_way_down() {
    let c = Content::builtin();
    for seed in 0..12u64 {
        for depth in 1..=8u8 {
            let l = levelgen::generate(&c, seed * 7919 + 1, depth, RunOptions::default());
            assert_eq!(l.world.size, levelgen::SIZE, "seed {seed} depth {depth} fell back to the test level");
            assert!(l.supported(l.start) && l.open(l.start), "start must be a plain tile");
            let goal = l.features.values().any(|f| match f {
                Feature::Stairs { .. } => depth < 8,
                Feature::Chest { great, .. } => *great && depth == 8,
                _ => false,
            });
            assert!(goal, "seed {seed} depth {depth} has no stairs (or great chest)");
            assert!(!l.monsters.is_empty());
        }
    }
}

#[test]
fn spider_free_mode_has_no_spiders() {
    // REF R43
    let c = Content::builtin();
    for seed in 0..20u64 {
        for depth in 2..=7u8 {
            let l = levelgen::generate(&c, seed, depth, RunOptions { spider_free: true });
            assert!(l.monsters.iter().all(|m| !c.monster(&m.kind).spider));
        }
    }
}

#[test]
fn save_and_load_round_trip() {
    let mut s = Sim::new("SAVE-ME", RunOptions::default(), Content::builtin());
    for _ in 0..20 {
        s.tick(Action::StepForward);
        s.tick(Action::TurnRight);
    }
    let text = s.save();
    let back = Sim::load(&text, Content::builtin()).unwrap();
    assert_eq!(back.state, s.state);
    assert_eq!(back.state_hash(), s.state_hash());
}

/// Random play for many ticks: nothing panics and the party is always somewhere it may be.
#[test]
fn random_play_keeps_invariants() {
    let c = Content::builtin();
    for seed in ["FUZZ-1", "FUZZ-2", "FUZZ-3"] {
        let mut s = Sim::new(seed, RunOptions::default(), c.clone());
        let mut r = lac_sim::rng::stream(1, seed);
        let pool = [
            Action::StepForward,
            Action::StepForward,
            Action::StepForward,
            Action::StepBack,
            Action::StepLeft,
            Action::StepRight,
            Action::TurnLeft,
            Action::TurnRight,
            Action::Jump,
            Action::LongJump,
            Action::Use,
            Action::Attack,
            Action::Attack,
            Action::Wait,
            Action::PickUp,
        ];
        for _ in 0..4000 {
            let a = match r.below(30) {
                0 => Action::UseItem { member: r.below(4) as u8, index: r.below(PACK_SIZE as u32) as u16 },
                1 => Action::Cast { member: r.below(4) as u8, spell: lac_sim::party::Spell::ALL[r.below(6) as usize] },
                2 => Action::Drop { index: r.below(PACK_SIZE as u32) as u16 },
                3 => Action::Swap { a: r.below(4) as u8, b: r.below(4) as u8 },
                _ => *r.pick(&pool).unwrap(),
            };
            s.tick(a);
            let st = &s.state;
            assert!(!st.level.world.solid(st.pos), "{seed}: party inside rock at {:?}", st.pos);
            assert!(st.pack.len() <= PACK_SIZE);
            if st.outcome != Outcome::Playing {
                break;
            }
            // keep the fuzz going through deaths, to reach more code
            if st.party.iter().all(|m| m.hp < 5) {
                for m in &mut s.state.party {
                    m.hp = m.max_hp;
                }
            }
        }
    }
}
