//! Every generated level can be walked from the start to the stairs (or the great chest) with the sim's own
//! moves, once its gates are open: a check that the generator's solver and the real rules agree.

use lac_sim::world::{Feature, LinkEffect};
use lac_sim::{levelgen, Action, Content, Dir, IVec3, Level, RunOptions, Sim};
use std::collections::{BTreeSet, VecDeque};

/// Open every door and raise every bridge, as a party with all the keys and levers would.
fn open_gates(l: &mut Level) {
    for f in l.features.values_mut() {
        if let Feature::Door { open, lock, .. } = f {
            *open = true;
            *lock = None;
        }
    }
    for effects in l.links.clone().values() {
        for e in effects {
            if let LinkEffect::ToggleBridge { cells, cell } = e {
                for p in cells {
                    l.world.set(*p, *cell);
                }
            }
        }
    }
    l.monsters.clear();
}

fn reaches_goal(c: &Content, seed: u64, depth: u8) -> bool {
    let mut l = levelgen::generate(c, seed, depth, RunOptions::default());
    open_gates(&mut l);
    let goal_chest: Option<IVec3> = l.features.iter().find(|(_, f)| matches!(f, Feature::Chest { great: true, .. })).map(|(p, _)| *p);
    let start = l.start;
    let mut base = Sim::on_level(l, c.clone());
    for m in &mut base.state.party {
        m.hp = 100_000;
        m.max_hp = 100_000;
    }
    let mut seen = BTreeSet::new();
    let mut q = VecDeque::new();
    seen.insert(start);
    q.push_back(start);
    let tries: [&[Action]; 3] = [&[Action::StepForward], &[Action::Jump, Action::StepForward], &[Action::LongJump]];
    while let Some(p) = q.pop_front() {
        if let Some(g) = goal_chest
            && p.y == g.y && p.flat_manhattan(g) == 1 {
                return true;
            }
        for d in Dir::ALL {
            for seq in tries {
                let mut s = base.clone();
                s.state.pos = p;
                s.state.facing = d;
                s.state.airborne = false;
                for a in seq.iter() {
                    s.tick(*a);
                }
                s.tick(Action::Wait);
                if s.state.level.depth != depth {
                    return true;
                }
                let to = s.state.pos;
                if seen.insert(to) {
                    q.push_back(to);
                }
            }
        }
    }
    false
}

#[test]
fn levels_can_be_walked_to_the_goal() {
    let c = Content::builtin();
    let extra = std::env::var("LAC_SEEDS").unwrap_or_default();
    let mut seeds: Vec<String> = vec!["BOT-1".into(), "BOT-2".into(), "PLAY-3".into()];
    seeds.extend(extra.split(',').filter(|s| !s.is_empty()).map(String::from));
    for seed in seeds {
        let n = lac_sim::rng::seed_from_text(&seed);
        for depth in 1..=8u8 {
            assert!(reaches_goal(&c, n, depth), "seed {seed} depth {depth}: the goal can't be reached with real moves");
        }
    }
}
