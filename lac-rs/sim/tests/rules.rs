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
    assert_eq!(s.state.tick, 2 * lac_sim::sim::action_ticks(&Action::Wait));
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

#[test]
fn standing_on_a_chest_holds_the_party_up() {
    // a long jump that comes down on a chest leaves the party standing on it, free to move on
    let mut s = test_sim();
    let chest = IVec3::new(5, 1, 3);
    s.state.level.world.set(chest, lac_sim::Cell::Air);
    s.state.level.features.insert(chest, Feature::Chest { open: false, lock: None, trap: None, mimic: false, items: vec![], facing: Dir::South, great: false });
    // stand on the ledge row beside it, one up
    s.state.pos = IVec3::new(4, 2, 3);
    s.state.facing = Dir::East;
    s.tick(Action::StepForward);
    assert_eq!(s.state.pos, IVec3::new(5, 2, 3), "on top of the chest");
    s.tick(Action::Wait);
    assert_eq!(s.state.pos, IVec3::new(5, 2, 3));
    let t = s.state.tick;
    s.tick(Action::Jump);
    assert_eq!(s.state.tick, t + lac_sim::sim::action_ticks(&Action::Jump), "can jump off a chest");
}

#[test]
fn falling_onto_a_monster_shoves_it_aside() {
    let mut s = test_sim();
    let c = Content::builtin();
    // the pit at z = 7 is two deep: a rat waits at the bottom
    lac_sim::levelgen::spawn(&mut s.state.level, &c, "rat", IVec3::new(5, 0, 7), &mut lac_sim::rng::Mt19937::new(1), RunOptions::default());
    s.state.pos = IVec3::new(5, 1, 6);
    s.state.facing = Dir::South;
    s.tick(Action::StepForward);
    assert_eq!(s.state.pos, IVec3::new(5, 0, 7), "landed in the pit");
    assert!(s.state.level.monsters.iter().all(|m| m.pos != s.state.pos), "the rat was pushed out of the way");
}

#[test]
fn raise_dead_brings_back_the_fallen() {
    let mut s = test_sim();
    s.state.party[1].hp = 0;
    s.state.pack.push(lac_sim::item::Item::new("scroll_raise"));
    let i = s.state.pack.len() as u16 - 1;
    s.tick(Action::UseItem { member: 0, index: i });
    assert!(s.state.party[1].alive());
    assert_eq!(s.state.party[1].hp, (s.state.party[1].max_hp / 4).max(1));
}

#[test]
fn more_than_full_fire_ward_makes_lava_heal() {
    // R60: exploitable stats; two fire wards (60 + 120) pass 100% and lava heals
    let mut s = test_sim();
    let m = &mut s.state.party[0];
    m.equip.insert(lac_sim::content::Slot::Ring, lac_sim::item::Item::new("ring_fire_ward"));
    m.equip.insert(lac_sim::content::Slot::Amulet, lac_sim::item::Item::new("emberheart"));
    m.hp = 5;
    for i in 1..4 {
        s.state.party[i].hp = 1000;
        s.state.party[i].max_hp = 1000;
    }
    s.state.level.world.set(IVec3::new(5, 1, 5), lac_sim::Cell::Lava);
    s.tick(Action::Wait);
    assert!(s.state.party[0].hp > 5, "lava healed the warded fighter, hp {}", s.state.party[0].hp);
    assert!(s.state.party[1].hp < 1000, "and burned the others");
}

#[test]
fn set_pieces_add_armour() {
    let c = Content::builtin();
    let mut s = test_sim();
    let m = &mut s.state.party[0];
    m.equip.clear();
    let bare = m.ac(&c);
    m.equip.insert(lac_sim::content::Slot::Head, lac_sim::item::Item::new("ward_helm"));
    let one = m.ac(&c);
    m.equip.insert(lac_sim::content::Slot::Body, lac_sim::item::Item::new("ward_mail"));
    let two = m.ac(&c);
    assert_eq!(one, bare + 2);
    assert_eq!(two, one + 5 + 2, "the second piece adds its own AC and the set bonus");
}

#[test]
fn a_unique_appears_at_most_once_per_run() {
    // board row G16
    let c = Content::builtin();
    for seed in 0..30u64 {
        let mut seen = std::collections::BTreeMap::new();
        for depth in 1..=8u8 {
            let l = lac_sim::levelgen::generate(&c, seed, depth, RunOptions::default());
            let mut items: Vec<&lac_sim::item::Item> = l.items.values().flatten().collect();
            for f in l.features.values() {
                if let Feature::Chest { items: inside, .. } = f {
                    items.extend(inside.iter());
                }
            }
            for it in items {
                if c.item(&it.def).unique {
                    *seen.entry(it.def.clone()).or_insert(0) += 1;
                    assert_eq!(lac_sim::levelgen::unique_depth(&c, seed, &it.def), depth);
                }
            }
        }
        assert!(seen.values().all(|n| *n == 1), "seed {seed}: {seen:?}");
    }
}

#[test]
fn nobody_jumps_over_a_closed_door() {
    let mut s = test_sim();
    let door = IVec3::new(5, 1, 4);
    s.state.level.features.insert(door, Feature::Door { open: false, lock: None, portcullis: false });
    s.tick(Action::Jump);
    let ev = s.tick(Action::StepForward);
    assert!(ev.contains(&Event::Bump), "the space over the door is shut");
    s.tick(Action::Wait);
    assert_eq!(s.state.pos, IVec3::new(5, 1, 5));
    // opened, it lets the party through
    s.tick(Action::Use);
    s.tick(Action::StepForward);
    assert_eq!(s.state.pos, door);
}

#[test]
fn one_member_can_strike_alone() {
    let c = Content::builtin();
    let mut s = test_sim();
    lac_sim::levelgen::spawn(&mut s.state.level, &c, "shambler", IVec3::new(5, 1, 4), &mut lac_sim::rng::Mt19937::new(3), RunOptions::default());
    let ev = s.tick(Action::AttackWith { member: 0 });
    let strikers: Vec<u8> = ev.iter().filter_map(|e| if let Event::PartyAttack { member, .. } = e { Some(*member) } else { None }).collect();
    assert_eq!(strikers, vec![0], "only Brann struck");
}

#[test]
fn turning_takes_no_time() {
    // REF R64: a turn is animated but moves no ticks, so monsters don't get to act while the party looks round
    let mut s = test_sim();
    s.tick(Action::TurnLeft);
    s.tick(Action::TurnLeft);
    assert_eq!(s.state.tick, 0);
    assert_eq!(s.state.facing, Dir::South);
    s.tick(Action::StepForward);
    assert_eq!(s.state.tick, lac_sim::sim::action_ticks(&Action::StepForward));
}

#[test]
fn every_generated_door_has_level_floor_both_sides() {
    // a lintel over a door next to a step would block the jump up it
    let c = Content::builtin();
    for seed in 0..15u64 {
        for depth in 1..=8u8 {
            let l = lac_sim::levelgen::generate(&c, seed, depth, RunOptions::default());
            for (p, f) in &l.features {
                if !matches!(f, Feature::Door { .. }) || !l.world.solid(p.up()) {
                    continue;
                }
                for d in Dir::ALL {
                    let q = *p + d.delta();
                    if l.open(q) || matches!(l.features.get(&q), Some(Feature::Door { .. })) {
                        assert!(l.supported(q) || l.world.get(q).is_liquid(), "seed {seed} depth {depth}: door {p:?} opens onto a drop at {q:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn a_member_can_hold_a_potion_and_drink_it() {
    // R63: items held on the character, used by clicking them
    let mut s = test_sim();
    s.state.party[2].equip.clear();
    s.state.party[2].hp = 2;
    let k = s.state.pack.iter().position(|i| i.def == "potion_healing").unwrap() as u16;
    let before = s.state.pack.len();
    s.tick(Action::Hold { member: 2, index: k });
    assert_eq!(s.state.pack.len(), before - 1);
    assert_eq!(s.state.tick, 0, "taking it in hand is free");
    s.tick(Action::UseHeld { member: 2 });
    assert!(s.state.party[2].hp > 2, "Odile drank it");
    assert!(!s.state.party[2].equip.contains_key(&lac_sim::content::Slot::Shield));
}
