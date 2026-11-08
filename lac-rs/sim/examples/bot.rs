//! A simple bot that plays whole runs through `lac-sim`, for balance (board row G14):
//! `cargo run --release -p lac-sim --example bot -- <runs> [first seed number]`.
//!
//! It fights what stands next to it, heals when hurt, picks things up, opens doors, flips levers when the way
//! down is shut, and otherwise walks the shortest way to the stairs (or the great chest) with the party's own
//! moves. It plays much worse than a person: it never identifies, equips or retreats.

use lac_sim::content::ItemKind;
use lac_sim::party::{Class, Spell};
use lac_sim::world::Feature;
use lac_sim::{Action, Content, Dir, IVec3, Outcome, RunOptions, Sim};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Copy)]
enum Move {
    Step(Dir),
    JumpStep(Dir),
    LongJump(Dir),
}

fn open(sim: &Sim, p: IVec3) -> bool {
    let l = &sim.state.level;
    if l.world.solid(p) || l.world.get(p) == lac_sim::Cell::Lava {
        return false;
    }
    match l.features.get(&p) {
        // the bot walks into closed doors and opens them; locked ones only with a key or a rogue
        Some(Feature::Door { open, portcullis, lock }) => *open || (!portcullis && (lock.is_none() || has_key(sim) || has_rogue(sim))),
        Some(f) => !f.blocks(),
        None => true,
    }
}

fn door_closed(sim: &Sim, p: IVec3) -> bool {
    matches!(sim.state.level.features.get(&p), Some(Feature::Door { open: false, .. }))
}

fn has_key(sim: &Sim) -> bool {
    sim.state.pack.iter().any(|i| matches!(sim.content.item(&i.def).kind, ItemKind::Key { .. }))
}

fn has_rogue(sim: &Sim) -> bool {
    sim.state.party.iter().any(|m| m.alive() && m.class == Class::Rogue)
}

fn fall(sim: &Sim, mut p: IVec3) -> Option<IVec3> {
    let l = &sim.state.level;
    while !l.supported(p) {
        if p.y <= 1 || !open(sim, p.down()) {
            return None;
        }
        p = p.down();
    }
    Some(p)
}

/// Shortest moves from the party to the nearest tile where `goal` holds.
fn path(sim: &Sim, goal: impl Fn(IVec3) -> bool) -> Option<Vec<Move>> {
    let start = sim.state.pos;
    let mut prev: BTreeMap<IVec3, (IVec3, Move)> = BTreeMap::new();
    let mut q = VecDeque::new();
    q.push_back(start);
    prev.insert(start, (start, Move::Step(Dir::North)));
    while let Some(p) = q.pop_front() {
        if p != start && goal(p) {
            let mut out = Vec::new();
            let mut c = p;
            while c != start {
                let (b, m) = prev[&c];
                out.push(m);
                c = b;
            }
            out.reverse();
            return Some(out);
        }
        for d in Dir::ALL {
            let t = p + d.delta();
            let mut cands = Vec::new();
            if open(sim, t) {
                if let Some(l) = fall(sim, t) {
                    cands.push((l, Move::Step(d)));
                }
                let t2 = t + d.delta();
                if open(sim, t2) && !door_closed(sim, t) && !door_closed(sim, t2) {
                    if let Some(l) = fall(sim, t2) {
                        cands.push((l, Move::LongJump(d)));
                    }
                }
            }
            if open(sim, p.up()) {
                let u = p.up() + d.delta();
                if open(sim, u) && !door_closed(sim, u) && sim.state.level.supported(u) {
                    cands.push((u, Move::JumpStep(d)));
                }
            }
            for (to, m) in cands {
                if !prev.contains_key(&to) {
                    prev.insert(to, (p, m));
                    q.push_back(to);
                }
            }
        }
    }
    None
}

fn face(sim: &Sim, d: Dir) -> Option<Action> {
    let f = sim.state.facing;
    if f == d {
        None
    } else if f.cw() == d {
        Some(Action::TurnRight)
    } else {
        Some(Action::TurnLeft)
    }
}

fn decide(sim: &Sim, stuck: &mut u32) -> Vec<Action> {
    let s = &sim.state;
    let c = &sim.content;
    // heal
    for (i, m) in s.party.iter().enumerate() {
        if m.alive() && m.hp * 10 < m.max_hp * 4 {
            if let Some(k) = s.pack.iter().position(|it| it.def.starts_with("potion_") && s.knowledge.identified.contains(&it.def) && matches!(c.item(&it.def).kind, ItemKind::Potion { effect: lac_sim::content::Effect::Heal(_) })) {
                return vec![Action::UseItem { member: i as u8, index: k as u16 }];
            }
            if let Some(p) = s.party.iter().position(|q| q.alive() && q.class == Class::Priest && q.mp >= Spell::Mend.cost()) {
                return vec![Action::Cast { member: p as u8, spell: Spell::Mend }];
            }
        }
    }
    // fight whatever is next to us
    for d in Dir::ALL {
        if s.level.monster_at(s.pos + d.delta()).is_some() {
            if let Some(t) = face(sim, d) {
                return vec![t];
            }
            if let Some(m) = s.party.iter().position(|q| q.alive() && q.class == Class::Mage && q.mp >= Spell::Spark.cost()) {
                if s.tick % 2 == 0 {
                    return vec![Action::Cast { member: m as u8, spell: Spell::Spark }];
                }
            }
            return vec![Action::Attack];
        }
    }
    if s.level.items.contains_key(&s.pos) && s.pack.len() < lac_sim::sim::PACK_SIZE {
        return vec![Action::PickUp];
    }
    // the goal: stairs, or a tile next to the great chest
    let goal = |p: IVec3| match s.level.features.get(&p) {
        Some(Feature::Stairs { .. }) => true,
        _ => Dir::ALL.iter().any(|d| matches!(s.level.features.get(&(p + d.delta())), Some(Feature::Chest { great: true, .. })) && p.y == (p + d.delta()).y),
    };
    let route = path(sim, goal).or_else(|| {
        // the way is shut: go to a lever not yet flipped, or a closed chest
        path(sim, |p| {
            Dir::ALL.iter().any(|d| match s.level.features.get(&(p + d.delta())) {
                Some(Feature::Lever { on: false, face, .. }) => *face == d.opposite(),
                _ => false,
            })
        })
    });
    let Some(route) = route else {
        *stuck += 1;
        return vec![Action::TurnLeft, Action::StepForward];
    };
    if route.is_empty() {
        // standing at the goal: use what is in front (lever, chest)
        for d in Dir::ALL {
            match s.level.features.get(&(s.pos + d.delta())) {
                Some(Feature::Lever { on: false, face, .. }) if *face == d.opposite() => return face_then(sim, d, Action::Use),
                Some(Feature::Chest { great: true, .. }) => return face_then(sim, d, Action::Use),
                _ => {}
            }
        }
    }
    match route.first() {
        None => {
            for d in Dir::ALL {
                if let Some(Feature::Lever { on: false, face, .. }) = s.level.features.get(&(s.pos + d.delta())) {
                    if *face == d.opposite() {
                        return face_then(sim, d, Action::Use);
                    }
                }
            }
            vec![Action::Wait]
        }
        Some(Move::Step(d)) => {
            // a closed door ahead: open it first
            if let Some(Feature::Door { open: false, .. }) = s.level.features.get(&(s.pos + d.delta())) {
                return face_then(sim, *d, Action::Use);
            }
            face_then(sim, *d, Action::StepForward)
        }
        Some(Move::JumpStep(d)) => match face(sim, *d) {
            Some(t) => vec![t],
            None => vec![Action::Jump, Action::StepForward],
        },
        Some(Move::LongJump(d)) => face_then(sim, *d, Action::LongJump),
    }
}

fn face_then(sim: &Sim, d: Dir, a: Action) -> Vec<Action> {
    match face(sim, d) {
        Some(t) => vec![t],
        None => vec![a],
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let runs: u32 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(20);
    let first: u32 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(1);
    let content = Content::builtin();
    let mut depths = BTreeMap::new();
    let mut wins = 0;
    for k in first..first + runs {
        let seed = format!("BOT-{k}");
        let mut sim = Sim::new(&seed, RunOptions::default(), content.clone());
        let mut stuck = 0;
        let mut ticks_on_level = 0;
        let mut depth = 1;
        while sim.state.outcome == Outcome::Playing && sim.state.tick < 30_000 && stuck < 400 {
            let trace = std::env::var("BOT_TRACE").is_ok() && ticks_on_level < 400;
            for a in decide(&sim, &mut stuck) {
                let ev = sim.tick(a);
                if trace {
                    let msgs: Vec<String> = ev.iter().filter_map(|e| if let lac_sim::Event::Message(m) = e { Some(m.clone()) } else { None }).collect();
                    println!("  {:?} {:?} {a:?} {msgs:?}", sim.state.pos, sim.state.facing);
                }
            }
            ticks_on_level += 1;
            if sim.state.level.depth != depth {
                depth = sim.state.level.depth;
                ticks_on_level = 0;
            }
            if ticks_on_level > 4000 {
                break;
            }
        }
        let s = &sim.state;
        let end = match &s.outcome {
            Outcome::Won => {
                wins += 1;
                "WON".to_string()
            }
            Outcome::Lost { cause } => format!("died ({cause})"),
            Outcome::Playing => if stuck >= 400 { "stuck".into() } else { "timed out".into() },
        };
        let levels: Vec<String> = s.party.iter().map(|m| format!("{}{}", m.level, if m.alive() { "" } else { "+" })).collect();
        println!("{seed:>8}: depth {} tick {:>5} kills {:>3} gold {:>4} levels {}  {end}", s.stats.deepest, s.tick, s.stats.kills, s.gold, levels.join("/"));
        *depths.entry(s.stats.deepest).or_insert(0) += 1;
    }
    println!("deepest reached: {depths:?}; wins {wins}/{runs}");
}
