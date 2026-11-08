//! The simulation: the action queue, the tick, and every rule that runs inside a tick.
//!
//! REF R27: time stands still while no action is queued. The sim never advances on its own; the app pushes
//! actions and steps it. REF R13: turn-based. REF R24: actions, not keys, are what the sim sees.

use crate::content::{Content, Effect, Element, ItemKind, OnHit, Slot};
use crate::dice::Dice;
use crate::geom::{Dir, IVec3};
use crate::item::{Item, Knowledge};
use crate::levelgen;
use crate::monster;
use crate::party::{self, Attr, Character, Class, Spell};
use crate::rng::{self, Mt19937, Streams};
use crate::world::{Cell, Feature, KeyKind, Level, LinkEffect, Trap};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// The deepest level; its great chest ends the run. OURS: R77 (how a run is won) is a gap.
pub const FINAL_DEPTH: u8 = 8;
/// OURS: one shared pack of 24 items.
pub const PACK_SIZE: usize = 24;
/// OURS: a monster acts each time its energy reaches this.
const ACT_COST: i32 = 12;

/// What the player asks the party to do. REF R24: decoupled from raw input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    StepForward,
    StepBack,
    StepLeft,
    StepRight,
    TurnLeft,
    TurnRight,
    /// REF R21: up one tile, with one tick of grace before the fall.
    Jump,
    /// REF R22: across a one-tile gap.
    LongJump,
    /// Whatever is in front: lever, door, chest, fountain, stairs; or search the wall.
    Use,
    /// The whole party strikes (G9a).
    Attack,
    Wait,
    Cast { member: u8, spell: Spell },
    /// Drink, read or zap the pack item at `index`; weapons, armour and jewellery are put on.
    UseItem { member: u8, index: u16 },
    Equip { member: u8, index: u16 },
    Unequip { member: u8, slot: Slot },
    /// Free: drop onto the party's tile.
    Drop { index: u16 },
    /// Free: pick up everything on the party's tile.
    PickUp,
    /// Free: swap two members (front row is 0 and 1).
    Swap { a: u8, b: u8 },
    /// Free: put a mark on the automap at the party's tile (0 takes it away). R60: a minimap the player can
    /// make notes on.
    Note { mark: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveKind {
    Step,
    Jump,
    LongJump,
    Fall,
    Teleport,
}

/// What happened in a tick, for the app to show. The app reads positions from the state; events say how
/// things got there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    PartyMoved { from: IVec3, to: IVec3, kind: MoveKind },
    PartyTurned { from: Dir, to: Dir },
    /// A move that went nowhere.
    Bump,
    MonsterMoved { id: u32, from: IVec3, to: IVec3 },
    MonsterSpawned { id: u32 },
    MonsterHurt { id: u32, amount: i32 },
    MonsterDied { id: u32, pos: IVec3 },
    /// `target` is the party member.
    MonsterAttack { id: u32, target: u8, hit: bool, damage: i32, ranged: bool },
    PartyAttack { member: u8, target: Option<u32>, hit: bool, damage: i32, ranged: bool },
    CharacterHurt { member: u8, amount: i32 },
    CharacterHealed { member: u8, amount: i32 },
    CharacterDied { member: u8 },
    Spell { member: u8, spell: Spell },
    Bolt { from: IVec3, to: IVec3, fire: bool },
    CellsChanged(Vec<IVec3>),
    FeatureChanged(IVec3),
    ItemsChanged(IVec3),
    PackChanged,
    Message(String),
    LevelChanged { depth: u8 },
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunOptions {
    /// REF R43: spiders become beetles (board row X4).
    pub spider_free: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Playing,
    Won,
    Lost { cause: String },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub kills: u32,
    pub deepest: u8,
    pub gold_found: u32,
    pub chests: u32,
    pub levers: u32,
}

/// Everything a run is: saved, loaded, hashed for replays.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub seed_text: String,
    pub seed: u64,
    pub options: RunOptions,
    pub tick: u64,
    pub level: Level,
    pub party: Vec<Character>,
    pub pos: IVec3,
    pub facing: Dir,
    pub pack: Vec<Item>,
    pub gold: u32,
    pub knowledge: Knowledge,
    pub rng: Streams,
    /// Ticks of haste left.
    pub haste: u16,
    /// Ticks of levitation left (from potions; rings levitate while worn).
    pub levitate: u16,
    /// REF R21: true for the tick after a jump, while the party hangs in the air.
    pub airborne: bool,
    pub outcome: Outcome,
    pub stats: Stats,
}

/// A run in progress: the state, the content it runs on, and the queue of actions not yet taken.
#[derive(Clone, Debug)]
pub struct Sim {
    pub state: State,
    pub content: Content,
    queue: VecDeque<Action>,
}

fn say(ev: &mut Vec<Event>, s: impl Into<String>) {
    ev.push(Event::Message(s.into()));
}

impl Sim {
    /// A new run from seed text.
    pub fn new(seed_text: &str, options: RunOptions, content: Content) -> Sim {
        let seed = rng::seed_from_text(seed_text);
        let mut streams = Streams::new(seed);
        let knowledge = Knowledge::deal(&content, &mut streams.loot);
        let level = levelgen::generate(&content, seed, 1, options);
        let mut pack = vec![Item::new("potion_healing"), Item::new("potion_healing"), Item::new("scroll_identify")];
        pack.retain(|i| content.items.contains_key(&i.def));
        let state = State {
            seed_text: seed_text.trim().to_uppercase(),
            seed,
            options,
            tick: 0,
            pos: level.start,
            facing: level.start_facing,
            level,
            party: party::starting_party(),
            pack,
            gold: 0,
            knowledge,
            rng: streams,
            haste: 0,
            levitate: 0,
            airborne: false,
            outcome: Outcome::Playing,
            stats: Stats { deepest: 1, ..Stats::default() },
        };
        let mut sim = Sim { state, content, queue: VecDeque::new() };
        // the starting healing potions are known; the rest of the run is a mystery
        sim.state.knowledge.identified.insert("potion_healing".into());
        sim.state.update_explored();
        sim
    }

    /// A run on a given level (tests, the debug test level).
    pub fn on_level(level: Level, content: Content) -> Sim {
        let mut sim = Sim::new("TEST", RunOptions::default(), content);
        sim.state.pos = level.start;
        sim.state.facing = level.start_facing;
        sim.state.level = level;
        sim.state.update_explored();
        sim
    }

    pub fn from_state(state: State, content: Content) -> Sim {
        Sim { state, content, queue: VecDeque::new() }
    }

    /// Queue an action. REF R28: the app may queue many; it plays them back faster the more wait.
    pub fn push(&mut self, a: Action) {
        self.queue.push_back(a);
    }
    /// The next action in the queue.
    pub fn peek(&self) -> Option<&Action> {
        self.queue.front()
    }
    pub fn queued(&self) -> usize {
        self.queue.len()
    }
    pub fn clear_queue(&mut self) {
        self.queue.clear();
    }

    /// Take the next queued action, if any. REF R27: no action, no tick.
    pub fn step(&mut self) -> Option<Vec<Event>> {
        let a = self.queue.pop_front()?;
        Some(self.tick(a))
    }

    /// Take one action and, if it takes time, advance the world by one turn.
    pub fn tick(&mut self, a: Action) -> Vec<Event> {
        let mut ev = Vec::new();
        if self.state.outcome != Outcome::Playing {
            return ev;
        }
        let c = &self.content;
        let s = &mut self.state;
        let depth_before = s.level.depth;
        let took_time = s.party_act(c, a, &mut ev);
        if !took_time || s.outcome != Outcome::Playing {
            return ev;
        }
        s.tick += 1;
        if s.level.depth != depth_before {
            // a new level: the monsters there haven't seen us yet
            s.update_explored();
            return ev;
        }
        if a != Action::Jump {
            s.gravity(c, &mut ev);
            if s.level.depth != depth_before {
                s.update_explored();
                return ev;
            }
        }
        s.status(c, &mut ev);
        if s.outcome == Outcome::Playing {
            s.monsters_act(c, &mut ev);
        }
        s.level.monsters.retain(|m| m.hp > 0);
        s.check_lost(&mut ev);
        s.update_explored();
        ev
    }

    /// The hash of the whole state: the replay test compares it (board row F4).
    pub fn state_hash(&self) -> u64 {
        rng::fnv1a64(ron::ser::to_string(&self.state).expect("state serialises").as_bytes())
    }

    pub fn save(&self) -> String {
        ron::ser::to_string(&self.state).expect("state serialises")
    }

    pub fn load(text: &str, content: Content) -> Result<Sim, String> {
        let state: State = ron::from_str(text).map_err(|e| e.to_string())?;
        Ok(Sim::from_state(state, content))
    }

    pub fn item_name(&self, it: &Item) -> String {
        self.state.knowledge.name(&self.content, it)
    }
}

impl State {
    pub fn levitating(&self, c: &Content) -> bool {
        self.levitate > 0 || self.party_effect(c, |e| matches!(e, Effect::Levitate(_)))
    }

    pub fn jumping_boots(&self, c: &Content) -> bool {
        self.party_effect(c, |e| matches!(e, Effect::Jumping))
    }

    /// Does any living member wear something with this effect?
    pub fn party_effect(&self, c: &Content, f: impl Fn(&Effect) -> bool) -> bool {
        self.party.iter().filter(|m| m.alive()).any(|m| member_effect(c, m, &f))
    }

    fn front(&self) -> IVec3 {
        self.pos + self.facing.delta()
    }

    /// Is something holding the party up: the floor, a chest, or (OURS) a monster it came down on?
    fn standing(&self) -> bool {
        self.level.supported(self.pos) || self.level.monster_at(self.pos.down()).is_some()
    }

    fn can_enter(&self, p: IVec3) -> bool {
        self.level.open(p) && self.level.monster_at(p).is_none()
    }

    fn living(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.party.len()).filter(|&i| self.party[i].alive())
    }

    fn best_rogue(&self) -> Option<usize> {
        self.living().find(|&i| self.party[i].class == Class::Rogue)
    }

    // ---------------------------------------------------------------- the party's turn

    /// Do the party's action; returns whether it took time.
    fn party_act(&mut self, c: &Content, a: Action, ev: &mut Vec<Event>) -> bool {
        match a {
            Action::StepForward => self.step(c, self.facing, ev),
            Action::StepBack => self.step(c, self.facing.opposite(), ev),
            Action::StepLeft => self.step(c, self.facing.ccw(), ev),
            Action::StepRight => self.step(c, self.facing.cw(), ev),
            Action::TurnLeft | Action::TurnRight => {
                let from = self.facing;
                self.facing = if a == Action::TurnLeft { from.ccw() } else { from.cw() };
                ev.push(Event::PartyTurned { from, to: self.facing });
                true
            }
            Action::Jump => self.jump(c, ev),
            Action::LongJump => self.long_jump(c, ev),
            Action::Use => self.use_front(c, ev),
            Action::Attack => {
                self.attack(c, ev);
                true
            }
            Action::Wait => true,
            Action::Cast { member, spell } => self.cast(c, member as usize, spell, ev),
            Action::UseItem { member, index } => self.use_item(c, member as usize, index as usize, ev),
            Action::Equip { member, index } => self.equip(c, member as usize, index as usize, ev),
            Action::Unequip { member, slot } => self.unequip(c, member as usize, slot, ev),
            Action::Drop { index } => {
                if (index as usize) < self.pack.len() {
                    let it = self.pack.remove(index as usize);
                    say(ev, format!("Dropped {}.", self.knowledge.name(c, &it)));
                    self.level.items.entry(self.pos).or_default().push(it);
                    ev.push(Event::ItemsChanged(self.pos));
                    ev.push(Event::PackChanged);
                }
                false
            }
            Action::PickUp => {
                self.pick_up(c, ev);
                false
            }
            Action::Note { mark } => {
                let key = (self.pos.x, self.pos.z);
                if mark == 0 {
                    self.level.notes.remove(&key);
                } else {
                    self.level.notes.insert(key, mark);
                }
                false
            }
            Action::Swap { a, b } => {
                let (a, b) = (a as usize, b as usize);
                if a < self.party.len() && b < self.party.len() && a != b {
                    self.party.swap(a, b);
                    say(ev, format!("{} and {} change places.", self.party[b].name, self.party[a].name));
                }
                false
            }
        }
    }

    /// One tile along `dir`. REF R20: tile-based. REF R23: walls block. OURS: a blocked step takes no time.
    fn step(&mut self, c: &Content, dir: Dir, ev: &mut Vec<Event>) -> bool {
        let to = self.pos + dir.delta();
        if !self.can_enter(to) {
            if let Some(i) = self.level.monster_at(to) {
                say(ev, format!("The {} is in the way.", c.monster(&self.level.monsters[i].kind).name));
            } else if self.level.world.solid(to) && !self.level.world.solid(to.up()) && self.level.open(to.up()) {
                say(ev, "A step up: jump first.");
            }
            ev.push(Event::Bump);
            return false;
        }
        let from = self.pos;
        self.pos = to;
        self.airborne = false;
        ev.push(Event::PartyMoved { from, to, kind: MoveKind::Step });
        self.entered(c, ev);
        true
    }

    /// REF R21: jump = up one tile, then one tick of grace before falling, so a step queued right after it
    /// lands on the ledge. OURS: boots of jumping make it two tiles.
    fn jump(&mut self, c: &Content, ev: &mut Vec<Event>) -> bool {
        if self.levitating(c) {
            say(ev, "You can't jump while floating.");
            return false;
        }
        if !self.standing() {
            return false;
        }
        let height = if self.jumping_boots(c) { 2 } else { 1 };
        let mut to = self.pos;
        for _ in 0..height {
            if !self.can_enter(to.up()) {
                break;
            }
            to = to.up();
        }
        if to == self.pos {
            say(ev, "No room to jump.");
            ev.push(Event::Bump);
            return false;
        }
        let from = self.pos;
        self.pos = to;
        self.airborne = true;
        ev.push(Event::PartyMoved { from, to, kind: MoveKind::Jump });
        true
    }

    /// REF R22: a long jump crosses a one-tile gap. OURS: it flies as far as it can up to two tiles (three
    /// with boots of jumping) and lands where it stops; the fall, if any, follows.
    fn long_jump(&mut self, c: &Content, ev: &mut Vec<Event>) -> bool {
        if !self.standing() && !self.levitating(c) {
            return false;
        }
        let dist = if self.jumping_boots(c) { 3 } else { 2 };
        let mut to = self.pos;
        for _ in 0..dist {
            let next = to + self.facing.delta();
            if !self.can_enter(next) {
                break;
            }
            to = next;
        }
        if to == self.pos {
            ev.push(Event::Bump);
            return false;
        }
        let from = self.pos;
        self.pos = to;
        self.airborne = false;
        ev.push(Event::PartyMoved { from, to, kind: MoveKind::LongJump });
        // if nothing holds us here, gravity at the end of the tick brings us down and checks again
        self.entered(c, ev);
        true
    }

    /// Fall until something holds the party up. OURS: falls of three tiles or more hurt, (tiles - 2)d4 each,
    /// unless the party lands in water.
    fn gravity(&mut self, c: &Content, ev: &mut Vec<Event>) {
        self.airborne = false;
        if self.levitating(c) {
            return;
        }
        let from = self.pos;
        loop {
            if self.level.supported(self.pos) || self.pos.y <= 0 {
                break;
            }
            let below = self.pos.down();
            // OURS: falling onto a monster shoves it aside if it can go anywhere; else the party lands on it
            if let Some(i) = self.level.monster_at(below) {
                let spots: Vec<IVec3> = Dir::ALL.iter().map(|d| below + d.delta()).filter(|q| self.can_enter(*q) && self.level.supported(*q)).collect();
                match self.rng.ai.pick(&spots).copied() {
                    Some(q) => {
                        let id = self.level.monsters[i].id;
                        self.level.monsters[i].pos = q;
                        ev.push(Event::MonsterMoved { id, from: below, to: q });
                        say(ev, format!("You land beside the {}, knocking it aside.", c.monster(&self.level.monsters[i].kind).name));
                    }
                    None => break,
                }
            }
            if !self.can_enter(below) {
                break;
            }
            self.pos = below;
        }
        if self.pos == from {
            return;
        }
        ev.push(Event::PartyMoved { from, to: self.pos, kind: MoveKind::Fall });
        let tiles = from.y - self.pos.y;
        if tiles >= 3 && self.level.world.get(self.pos) != Cell::Water {
            say(ev, "You land hard.");
            let d = Dice::new((tiles - 2) as u8, 4, 0);
            for i in self.living().collect::<Vec<_>>() {
                let dmg = d.roll(&mut self.rng.combat);
                self.hurt(i, dmg, "a fall", ev);
            }
        }
        self.entered(c, ev);
    }

    /// The party has arrived on a tile.
    fn entered(&mut self, c: &Content, ev: &mut Vec<Event>) {
        let p = self.pos;
        if !self.level.supported(p) && !self.levitating(c) {
            // still in the air: gravity will bring us down at the end of the tick
            return;
        }
        match self.level.features.get(&p).cloned() {
            Some(Feature::Stairs { .. }) => {
                if self.levitating(c) {
                    say(ev, "You float above the stairs and can't reach them.");
                } else {
                    self.descend(c, ev);
                }
                return;
            }
            Some(Feature::Plate { pressed: false, link }) => {
                self.level.features.insert(p, Feature::Plate { pressed: true, link });
                ev.push(Event::FeatureChanged(p));
                say(ev, "Click. Something gives under your feet.");
                self.fire_link(c, link, ev);
            }
            Some(Feature::Spikes) => {
                say(ev, "Spikes!");
                for i in self.living().collect::<Vec<_>>() {
                    let dmg = Dice::new(1, 6, 0).roll(&mut self.rng.combat);
                    self.hurt(i, dmg, "spikes", ev);
                }
            }
            _ => {}
        }
        if let Some(items) = self.level.items.get(&p)
            && !items.is_empty() {
                let names: Vec<String> = items.iter().take(4).map(|i| self.knowledge.name(c, i)).collect();
                let more = if items.len() > 4 { ", ..." } else { "" };
                say(ev, format!("Here: {}{more}", names.join(", ")));
            }
        if self.party_effect(c, |e| matches!(e, Effect::SeeSecrets)) {
            for d in Dir::ALL {
                let q = p + d.delta();
                if self.level.features.get(&q) == Some(&Feature::SecretWall) {
                    self.reveal(q, ev);
                    say(ev, "Your amulet glows: a hidden passage!");
                }
            }
        }
    }

    fn descend(&mut self, c: &Content, ev: &mut Vec<Event>) {
        let depth = self.level.depth + 1;
        self.level = levelgen::generate(c, self.seed, depth, self.options);
        self.pos = self.level.start;
        self.facing = self.level.start_facing;
        self.airborne = false;
        self.stats.deepest = self.stats.deepest.max(depth);
        ev.push(Event::LevelChanged { depth });
        let msg = if depth == FINAL_DEPTH {
            format!("Depth {depth}: {}. The air hums. Something old guards the great chest.", self.level.theme.name())
        } else {
            format!("You descend to depth {depth}: {}.", self.level.theme.name())
        };
        say(ev, msg);
    }

    fn pick_up(&mut self, c: &Content, ev: &mut Vec<Event>) {
        let Some(items) = self.level.items.remove(&self.pos) else {
            say(ev, "Nothing here to pick up.");
            return;
        };
        let mut left = Vec::new();
        for it in items {
            match c.item(&it.def).kind {
                ItemKind::Gold { .. } => {
                    self.gold += it.amount;
                    self.stats.gold_found += it.amount;
                    say(ev, format!("{} gold.", it.amount));
                }
                _ if self.pack.len() < PACK_SIZE => {
                    say(ev, format!("Got {}.", self.knowledge.name(c, &it)));
                    self.pack.push(it);
                }
                _ => left.push(it),
            }
        }
        if !left.is_empty() {
            say(ev, "The pack is full.");
            self.level.items.insert(self.pos, left);
        }
        ev.push(Event::ItemsChanged(self.pos));
        ev.push(Event::PackChanged);
    }

    fn give(&mut self, c: &Content, it: Item, ev: &mut Vec<Event>) {
        if let ItemKind::Gold { .. } = c.item(&it.def).kind {
            self.gold += it.amount;
            self.stats.gold_found += it.amount;
            say(ev, format!("{} gold.", it.amount));
        } else if self.pack.len() < PACK_SIZE {
            say(ev, format!("Got {}.", self.knowledge.name(c, &it)));
            self.pack.push(it);
            ev.push(Event::PackChanged);
        } else {
            say(ev, format!("No room: {} drops at your feet.", self.knowledge.name(c, &it)));
            self.level.items.entry(self.pos).or_default().push(it);
            ev.push(Event::ItemsChanged(self.pos));
        }
    }

    // ---------------------------------------------------------------- using things

    fn take_key(&mut self, c: &Content, kind: KeyKind) -> bool {
        let idx = self.pack.iter().position(|it| matches!(c.item(&it.def).kind, ItemKind::Key { key } if key == kind));
        match idx {
            Some(i) => {
                self.pack.remove(i);
                true
            }
            None => false,
        }
    }

    /// Try to open a lock: a key, else the rogue's picks, else brute force. OURS.
    fn try_lock(&mut self, c: &Content, kind: KeyKind, what: &str, ev: &mut Vec<Event>) -> bool {
        if self.take_key(c, kind) {
            say(ev, format!("You unlock the {what} with the {} key.", kind.name()));
            ev.push(Event::PackChanged);
            return true;
        }
        if let Some(r) = self.best_rogue() {
            let m = &self.party[r];
            let roll = self.rng.world.range(1, 20) + m.mod_of(Attr::Dex) + m.level as i32;
            if roll >= 15 {
                say(ev, format!("{} picks the lock of the {what}.", m.name));
                return true;
            }
            say(ev, format!("{} fails to pick the {} lock of the {what}.", m.name, kind.name()));
            return false;
        }
        // OURS: with no rogue alive, the strongest tries to force it
        let strongest = self.living().max_by_key(|&i| (self.party[i].attr(Attr::Str), std::cmp::Reverse(i)));
        if let Some(i) = strongest {
            let m = &self.party[i];
            if self.rng.world.range(1, 20) + m.mod_of(Attr::Str) >= 16 {
                say(ev, format!("{} forces the {what} open.", m.name));
                return true;
            }
            say(ev, format!("{} strains at the locked {what}, but it holds.", m.name));
        }
        false
    }

    fn use_front(&mut self, c: &Content, ev: &mut Vec<Event>) -> bool {
        let f = self.front();
        match self.level.features.get(&f).cloned() {
            Some(Feature::Lever { on, face, link }) => {
                if face != self.facing.opposite() {
                    say(ev, "You can't reach the lever from here.");
                    return false;
                }
                self.level.features.insert(f, Feature::Lever { on: !on, face, link });
                self.stats.levers += 1;
                ev.push(Event::FeatureChanged(f));
                say(ev, "Clunk. The lever moves.");
                self.fire_link(c, link, ev);
                true
            }
            Some(Feature::Door { open, lock, portcullis }) => {
                if portcullis {
                    say(ev, if open { "The portcullis is up." } else { "A portcullis. It won't lift by hand." });
                    return false;
                }
                if let Some(k) = lock {
                    if !self.try_lock(c, k, "door", ev) {
                        return true;
                    }
                    self.level.features.insert(f, Feature::Door { open: true, lock: None, portcullis });
                    ev.push(Event::FeatureChanged(f));
                    return true;
                }
                if open && (self.level.monster_at(f).is_some() || self.level.items.contains_key(&f)) {
                    say(ev, "Something is in the doorway.");
                    return false;
                }
                self.level.features.insert(f, Feature::Door { open: !open, lock: None, portcullis });
                ev.push(Event::FeatureChanged(f));
                true
            }
            Some(Feature::Chest { .. }) => self.open_chest(c, f, ev),
            Some(Feature::Fountain { uses }) => {
                self.fountain(c, f, uses, ev);
                true
            }
            Some(Feature::Stairs { .. }) => {
                let from = self.pos;
                self.pos = f;
                ev.push(Event::PartyMoved { from, to: f, kind: MoveKind::Step });
                self.descend(c, ev);
                true
            }
            Some(Feature::Tomb) => {
                say(ev, "An old tomb. The name on it is worn away.");
                false
            }
            Some(Feature::SecretWall) => {
                if self.search_roll() {
                    self.reveal(f, ev);
                    say(ev, "You find a hidden passage!");
                } else {
                    say(ev, "You search the wall and find nothing.");
                }
                true
            }
            _ => {
                if let Some(i) = self.level.monster_at(f) {
                    say(ev, format!("The {} won't be used. Attack it.", c.monster(&self.level.monsters[i].kind).name));
                    return false;
                }
                if self.level.world.solid(f) {
                    say(ev, "You search the wall and find nothing.");
                    return true;
                }
                say(ev, "There is nothing to use there.");
                false
            }
        }
    }

    /// OURS: 20% + 5% per level and INT bonus of the best searcher; rogues add 25%.
    fn search_roll(&mut self) -> bool {
        let mut best = 0;
        for i in self.living().collect::<Vec<_>>() {
            let m = &self.party[i];
            let mut v = 20 + 5 * (m.level as i32 + m.mod_of(Attr::Int));
            if m.class == Class::Rogue {
                v += 25;
            }
            best = best.max(v);
        }
        self.rng.world.below(100) < best.clamp(5, 95) as u32
    }

    fn reveal(&mut self, p: IVec3, ev: &mut Vec<Event>) {
        if self.level.features.get(&p) == Some(&Feature::SecretWall) {
            self.level.features.remove(&p);
            self.level.world.set(p, Cell::Air);
            ev.push(Event::FeatureChanged(p));
            ev.push(Event::CellsChanged(vec![p]));
        }
    }

    fn open_chest(&mut self, c: &Content, f: IVec3, ev: &mut Vec<Event>) -> bool {
        let Some(Feature::Chest { open, lock, trap, mimic, items, facing, great }) = self.level.features.get(&f).cloned() else {
            return false;
        };
        if open {
            if items.is_empty() {
                say(ev, "The chest is empty.");
                return false;
            }
        } else {
            if mimic {
                self.level.features.remove(&f);
                ev.push(Event::FeatureChanged(f));
                let id = levelgen::spawn(&mut self.level, c, "mimic", f, &mut self.rng.ai, self.options);
                if let Some(m) = self.level.monsters.iter_mut().find(|m| m.id == id) {
                    m.awake = true;
                    m.facing = self.facing.opposite();
                }
                ev.push(Event::MonsterSpawned { id });
                say(ev, "The chest grows teeth. A mimic!");
                return true;
            }
            if let Some(k) = lock
                && !self.try_lock(c, k, "chest", ev) {
                    return true;
                }
            if let Some(t) = trap {
                let disarmed = match self.best_rogue() {
                    Some(r) => {
                        let m = &self.party[r];
                        self.rng.world.range(1, 20) + m.mod_of(Attr::Dex) + m.level as i32 >= 12
                    }
                    None => false,
                };
                if disarmed {
                    let name = self.party[self.best_rogue().unwrap()].name.clone();
                    say(ev, format!("{name} finds a trap on the chest and disarms it."));
                } else {
                    self.spring_trap(c, t, ev);
                }
            }
            self.stats.chests += 1;
            if great {
                self.level.features.insert(f, Feature::Chest { open: true, lock: None, trap: None, mimic: false, items: vec![], facing, great });
                ev.push(Event::FeatureChanged(f));
                say(ev, "The great chest opens. Inside, on old velvet: the Sunken Crown. You have won!");
                self.outcome = Outcome::Won;
                ev.push(Event::Won);
                return true;
            }
        }
        self.level.features.insert(f, Feature::Chest { open: true, lock: None, trap: None, mimic: false, items: vec![], facing, great });
        ev.push(Event::FeatureChanged(f));
        if items.is_empty() {
            say(ev, "The chest is empty.");
        }
        for it in items {
            self.give(c, it, ev);
        }
        true
    }

    fn spring_trap(&mut self, c: &Content, t: Trap, ev: &mut Vec<Event>) {
        match t {
            Trap::Needle => {
                let who: Vec<usize> = self.living().collect();
                if let Some(&i) = self.rng.world.pick(&who) {
                    say(ev, format!("A needle pricks {}. Poison!", self.party[i].name));
                    if self.party[i].resist(c, Element::Poison) < 100 {
                        self.party[i].poison += 10;
                    }
                    self.harm(c, i, 1, Element::Poison, "a poisoned needle", ev);
                }
            }
            Trap::Gas => {
                say(ev, "Green gas fills the air!");
                for i in self.living().collect::<Vec<_>>() {
                    self.party[i].poison += 5;
                }
            }
            Trap::Fire => {
                say(ev, "The chest bursts into flame!");
                for i in self.living().collect::<Vec<_>>() {
                    let d = Dice::new(2, 4, 0).roll(&mut self.rng.combat);
                    self.harm(c, i, d, Element::Fire, "a fire trap", ev);
                }
            }
            Trap::Alarm => {
                say(ev, "A bell rings out across the level!");
                for m in &mut self.level.monsters {
                    m.awake = true;
                }
            }
            Trap::Teleport => {
                say(ev, "A glyph flares and the world lurches.");
                self.teleport(c, ev);
            }
        }
    }

    fn fountain(&mut self, c: &Content, f: IVec3, uses: u8, ev: &mut Vec<Event>) {
        let who: Vec<usize> = self.living().collect();
        let Some(&i) = self.rng.world.pick(&who) else { return };
        let name = self.party[i].name.clone();
        // OURS: what a fountain does, by percent
        let r = self.rng.world.below(100);
        match r {
            0..=29 => {
                say(ev, "Cool, clear water. Everyone feels better.");
                for j in who.clone() {
                    let d = Dice::new(2, 6, 0).roll(&mut self.rng.world);
                    self.heal(j, d, ev);
                }
            }
            30..=44 => {
                say(ev, format!("{name} feels magic tingling."));
                let m = &mut self.party[i];
                m.mp = (m.mp + 6).min(m.max_mp);
            }
            45..=59 => {
                say(ev, format!("The water is foul. {name} is poisoned."));
                self.party[i].poison += 8;
            }
            60..=69 => {
                say(ev, "Something glitters at the bottom of the basin.");
                let it = levelgen::roll_loot(c, "floor", self.level.depth, &mut self.rng.loot);
                for it in it {
                    self.give(c, it, ev);
                }
            }
            70..=79 => {
                say(ev, "The water churns. Something climbs out!");
                let spot = Dir::ALL.into_iter().map(|d| f + d.delta()).find(|&p| self.can_enter(p) && p != self.pos && self.level.supported(p));
                if let Some(p) = spot {
                    let id = levelgen::spawn(&mut self.level, c, "ooze", p, &mut self.rng.ai, self.options);
                    if let Some(m) = self.level.monsters.iter_mut().find(|m| m.id == id) {
                        m.awake = true;
                    }
                    ev.push(Event::MonsterSpawned { id });
                }
            }
            80..=84 => {
                say(ev, format!("{name} feels wiser. The water tastes of old secrets."));
                self.apply_effect(c, i, &Effect::Identify, ev);
            }
            85..=86 => {
                say(ev, format!("{name} feels strong!"));
                self.apply_effect(c, i, &Effect::Strength, ev);
            }
            _ => say(ev, "The water tastes of stone. Nothing happens."),
        }
        if uses <= 1 {
            self.level.features.remove(&f);
            say(ev, "The fountain dries up.");
        } else {
            self.level.features.insert(f, Feature::Fountain { uses: uses - 1 });
        }
        ev.push(Event::FeatureChanged(f));
    }

    /// Fire every effect of a link. OURS: R72 is a gap; links are our design.
    fn fire_link(&mut self, c: &Content, link: u32, ev: &mut Vec<Event>) {
        let effects = self.level.links.get(&link).cloned().unwrap_or_default();
        for e in effects {
            match e {
                LinkEffect::ToggleDoor(p) => {
                    if let Some(Feature::Door { open, lock, portcullis }) = self.level.features.get(&p).cloned() {
                        if open && self.level.monster_at(p).is_some() {
                            continue;
                        }
                        self.level.features.insert(p, Feature::Door { open: !open, lock, portcullis });
                        ev.push(Event::FeatureChanged(p));
                        say(ev, if open { "Somewhere, a gate slams down." } else { "Somewhere, a gate grinds open." });
                    }
                }
                LinkEffect::ToggleBridge { cells, cell } => {
                    let raising = cells.first().is_some_and(|p| self.level.world.get(*p) != cell);
                    for p in &cells {
                        self.level.world.set(*p, if raising { cell } else { Cell::Air });
                    }
                    ev.push(Event::CellsChanged(cells.clone()));
                    say(ev, if raising { "With a rumble, a bridge slides out." } else { "The bridge slides back into the wall." });
                }
                LinkEffect::Reveal(p) => {
                    self.reveal(p, ev);
                    say(ev, "Stone grinds on stone: a wall slides away.");
                }
                LinkEffect::Spawn { kind, at } => {
                    for p in at {
                        if self.can_enter(p) && p != self.pos {
                            let id = levelgen::spawn(&mut self.level, c, &kind, p, &mut self.rng.ai, self.options);
                            if let Some(m) = self.level.monsters.iter_mut().find(|m| m.id == id) {
                                m.awake = true;
                            }
                            ev.push(Event::MonsterSpawned { id });
                        }
                    }
                    say(ev, "You hear something stir.");
                }
                LinkEffect::Flood(cells) => {
                    for p in &cells {
                        if !self.level.world.solid(*p) {
                            self.level.world.set(*p, Cell::Water);
                        }
                    }
                    ev.push(Event::CellsChanged(cells));
                    say(ev, "Water gushes in!");
                }
                LinkEffect::Darts => {
                    say(ev, "Darts hiss out of the walls!");
                    for i in self.living().collect::<Vec<_>>() {
                        if self.rng.combat.chance(50) {
                            let d = Dice::new(1, 4, 0).roll(&mut self.rng.combat);
                            self.hurt(i, d, "a dart trap", ev);
                        }
                    }
                }
                LinkEffect::Wake => {
                    for m in &mut self.level.monsters {
                        m.awake = true;
                    }
                    say(ev, "A deep gong sounds. The level wakes.");
                }
            }
        }
    }

    // ---------------------------------------------------------------- combat (G9a: d20 against AC)

    /// The first living monster along the party's facing, within `range`, with nothing solid between.
    fn first_in_line(&self, range: i32) -> Option<usize> {
        let mut p = self.pos;
        for _ in 0..range {
            p += self.facing.delta();
            if let Some(i) = self.level.monster_at(p) {
                return Some(i);
            }
            if !self.level.open(p) {
                return None;
            }
        }
        None
    }

    fn all_in_line(&self, range: i32) -> Vec<usize> {
        let mut out = Vec::new();
        let mut p = self.pos;
        for _ in 0..range {
            p += self.facing.delta();
            if let Some(i) = self.level.monster_at(p) {
                out.push(i);
            }
            if !self.level.open(p) {
                break;
            }
        }
        out
    }

    /// G9a: one Attack makes the whole party strike. Front row with any weapon at the monster in front; back
    /// row only with reach (at the monster in front) or ranged weapons (the first monster in line).
    fn attack(&mut self, c: &Content, ev: &mut Vec<Event>) {
        let front = self.level.monster_at(self.front());
        let mut anyone = false;
        for i in self.living().collect::<Vec<_>>() {
            let (to_hit, dmg, bonus, reach, ranged) = self.party[i].attack_profile(c);
            let target = if ranged {
                self.first_in_line(6)
            } else if i < 2 || reach {
                front
            } else {
                None
            };
            let Some(t) = target else { continue };
            if self.level.monsters[t].hp <= 0 {
                continue;
            }
            anyone = true;
            let def = c.monster(&self.level.monsters[t].kind);
            let roll = self.rng.combat.range(1, 20);
            let hit = roll == 20 || (roll != 1 && roll + to_hit >= def.ac as i32);
            let mut damage = 0;
            if hit {
                damage = (dmg.roll(&mut self.rng.combat) + bonus).max(1);
                if roll == 20 {
                    damage *= 2;
                }
            }
            let id = self.level.monsters[t].id;
            ev.push(Event::PartyAttack { member: i as u8, target: Some(id), hit, damage, ranged });
            let name = def.name.clone();
            let who = self.party[i].name.clone();
            if hit {
                say(ev, format!("{who} hits the {name} for {damage}{}.", if roll == 20 { " (critical!)" } else { "" }));
                self.damage_monster(c, t, damage, ev);
            } else {
                say(ev, format!("{who} misses the {name}."));
                self.level.monsters[t].awake = true;
            }
        }
        if !anyone {
            ev.push(Event::PartyAttack { member: 0, target: None, hit: false, damage: 0, ranged: false });
            say(ev, "You swing at empty air.");
        }
    }

    fn damage_monster(&mut self, c: &Content, i: usize, amount: i32, ev: &mut Vec<Event>) {
        let m = &mut self.level.monsters[i];
        if m.hp <= 0 {
            return;
        }
        m.hp -= amount;
        m.awake = true;
        m.sleep = 0;
        ev.push(Event::MonsterHurt { id: m.id, amount });
        if m.hp <= 0 {
            let (id, pos, kind) = (m.id, m.pos, m.kind.clone());
            let def = c.monster(&kind);
            ev.push(Event::MonsterDied { id, pos });
            say(ev, format!("The {} dies.", def.name));
            self.stats.kills += 1;
            let xp = def.xp;
            self.share_xp(xp, ev);
            // OURS: a quarter of monsters drop something; mimics drop a chest's worth
            let table = if kind == "mimic" { Some("chest") } else if self.rng.loot.chance(25) { Some("floor") } else { None };
            if let Some(t) = table {
                let drop_at = if self.level.supported(pos) { pos } else { self.pos };
                let items = levelgen::roll_loot(c, t, self.level.depth, &mut self.rng.loot);
                if !items.is_empty() {
                    self.level.items.entry(drop_at).or_default().extend(items);
                    ev.push(Event::ItemsChanged(drop_at));
                }
            }
        }
    }

    fn share_xp(&mut self, xp: u32, ev: &mut Vec<Event>) {
        let alive: Vec<usize> = self.living().collect();
        if alive.is_empty() {
            return;
        }
        let each = (xp / alive.len() as u32).max(1);
        for i in alive {
            self.party[i].xp += each;
            while self.party[i].xp >= party::xp_for_next(self.party[i].level) && self.party[i].level < 20 {
                let m = &mut self.party[i];
                m.level += 1;
                let gain = (1 + self.rng.combat.below(m.class.hit_die() as u32) as i32 + m.mod_of(Attr::Con)).max(1);
                m.max_hp += gain;
                m.hp += gain;
                let caster = match m.class {
                    Class::Mage => m.mod_of(Attr::Int),
                    Class::Priest => m.mod_of(Attr::Wis),
                    _ => 0,
                };
                if m.class.mana_per_level() > 0 {
                    let mg = m.class.mana_per_level() + caster.max(0);
                    m.max_mp += mg;
                    m.mp += mg;
                }
                say(ev, format!("{} reaches level {}!", m.name, m.level));
            }
        }
    }

    /// Harm of a kind: resistance takes its percent off, and past 100 the harm heals instead (R60: exploitable
    /// stats, on purpose).
    fn harm(&mut self, c: &Content, i: usize, amount: i32, el: Element, cause: &str, ev: &mut Vec<Event>) {
        if el == Element::Physical {
            return self.hurt(i, amount, cause, ev);
        }
        let r = self.party[i].resist(c, el);
        let left = amount * (100 - r) / 100;
        if left < 0 {
            if self.party[i].alive() {
                say(ev, format!("{} drinks in the {}!", self.party[i].name, el.name()));
            }
            self.heal(i, -left, ev);
        } else {
            self.hurt(i, left, cause, ev);
        }
    }

    fn hurt(&mut self, i: usize, amount: i32, cause: &str, ev: &mut Vec<Event>) {
        let m = &mut self.party[i];
        if !m.alive() || amount <= 0 {
            return;
        }
        m.hp -= amount;
        ev.push(Event::CharacterHurt { member: i as u8, amount });
        if m.hp <= 0 {
            m.hp = 0;
            m.poison = 0;
            say(ev, format!("{} dies!", m.name));
            ev.push(Event::CharacterDied { member: i as u8 });
            if self.party.iter().all(|m| !m.alive()) {
                self.outcome = Outcome::Lost { cause: cause.to_string() };
            }
        }
    }

    /// OURS (board row G15): the dead stand up at a quarter of their hit points, and lose a level's worth of
    /// nothing else; the price is the scroll or the spell.
    fn raise(&mut self, ev: &mut Vec<Event>) {
        let mut any = false;
        for i in 0..self.party.len() {
            if !self.party[i].alive() {
                let m = &mut self.party[i];
                m.hp = (m.max_hp / 4).max(1);
                any = true;
                say(ev, format!("{} gasps and stands up!", m.name));
                ev.push(Event::CharacterHealed { member: i as u8, amount: m.hp });
            }
        }
        if !any {
            say(ev, "A warm wind passes. Nobody needed it.");
        }
    }

    fn heal(&mut self, i: usize, amount: i32, ev: &mut Vec<Event>) {
        let m = &mut self.party[i];
        if !m.alive() {
            return;
        }
        let before = m.hp;
        m.hp = (m.hp + amount).min(m.max_hp);
        if m.hp > before {
            ev.push(Event::CharacterHealed { member: i as u8, amount: m.hp - before });
        }
    }

    fn check_lost(&mut self, ev: &mut Vec<Event>) {
        if let Outcome::Lost { .. } = self.outcome
            && !ev.contains(&Event::Lost) {
                say(ev, "The whole party has fallen. The dungeon keeps them.");
                ev.push(Event::Lost);
            }
    }

    // ---------------------------------------------------------------- spells (G10)

    fn cast(&mut self, c: &Content, i: usize, spell: Spell, ev: &mut Vec<Event>) -> bool {
        let Some(m) = self.party.get(i) else { return false };
        if !m.alive() {
            return false;
        }
        if m.class != spell.class() {
            say(ev, format!("{} doesn't know {}.", m.name, spell.name()));
            return false;
        }
        if m.level < spell.min_level() {
            say(ev, format!("{} needs level {} for {}.", m.name, spell.min_level(), spell.name()));
            return false;
        }
        if m.mp < spell.cost() {
            say(ev, format!("{} is out of mana.", m.name));
            return false;
        }
        let (lvl, name) = (m.level as i32, m.name.clone());
        let wis = m.mod_of(Attr::Wis);
        self.party[i].mp -= spell.cost();
        ev.push(Event::Spell { member: i as u8, spell });
        match spell {
            Spell::Spark => {
                let dmg = Dice::new(1, 6, (lvl / 2) as i16).roll(&mut self.rng.combat);
                self.bolt(c, 8, dmg, false, false, ev);
            }
            Spell::Fireball => {
                if let Some(t) = self.first_in_line(8) {
                    let at = self.level.monsters[t].pos;
                    ev.push(Event::Bolt { from: self.pos, to: at, fire: true });
                    say(ev, format!("{name} hurls a fireball!"));
                    let main = Dice::new(3, 6, lvl as i16).roll(&mut self.rng.combat);
                    self.damage_monster(c, t, main, ev);
                    for j in 0..self.level.monsters.len() {
                        let p = self.level.monsters[j].pos;
                        if j != t && p.manhattan(at) == 1 {
                            let d = Dice::new(2, 6, 0).roll(&mut self.rng.combat);
                            self.damage_monster(c, j, d, ev);
                        }
                    }
                } else {
                    say(ev, "The fireball bursts against nothing.");
                }
            }
            Spell::Quicken => {
                self.haste += 20;
                say(ev, "Time thins around the party. You are quickened.");
            }
            Spell::Mend => {
                let target = self.living().min_by_key(|&j| (self.party[j].hp * 100) / self.party[j].max_hp.max(1));
                if let Some(t) = target {
                    let d = Dice::new(1, 8, (lvl + wis) as i16).roll(&mut self.rng.combat).max(1);
                    say(ev, format!("{name} mends {}.", self.party[t].name));
                    self.heal(t, d, ev);
                }
            }
            Spell::Purify => {
                for j in self.living().collect::<Vec<_>>() {
                    self.party[j].poison = 0;
                }
                say(ev, "A clean light washes over the party.");
            }
            Spell::Raise => {
                say(ev, format!("{name} calls the fallen back."));
                self.raise(ev);
            }
            Spell::Rebuke => {
                say(ev, format!("{name} raises a holy sign!"));
                let mut any = false;
                for j in 0..self.level.monsters.len() {
                    let m = &self.level.monsters[j];
                    if m.hp > 0 && c.monster(&m.kind).undead && m.pos.manhattan(self.pos) <= 3 && monster::line_of_sight(&self.level, self.pos, m.pos) {
                        any = true;
                        let d = Dice::new(2, 6, lvl as i16).roll(&mut self.rng.combat);
                        self.damage_monster(c, j, d, ev);
                    }
                }
                if !any {
                    say(ev, "No undead are near.");
                }
            }
        }
        true
    }

    /// A bolt down the facing line: the first monster (or every one, if `pierce`) takes `dmg`.
    fn bolt(&mut self, c: &Content, range: i32, dmg: i32, fire: bool, pierce: bool, ev: &mut Vec<Event>) {
        let targets = if pierce { self.all_in_line(range) } else { self.first_in_line(range).into_iter().collect() };
        let mut end = self.pos;
        for _ in 0..range {
            let n = end + self.facing.delta();
            if !self.level.open(n) {
                break;
            }
            end = n;
        }
        if let Some(&t) = targets.last()
            && !pierce {
                end = self.level.monsters[t].pos;
            }
        ev.push(Event::Bolt { from: self.pos, to: end, fire });
        if targets.is_empty() {
            say(ev, "The bolt hits nothing.");
        }
        for t in targets {
            self.damage_monster(c, t, dmg, ev);
        }
    }

    // ---------------------------------------------------------------- items

    fn use_item(&mut self, c: &Content, i: usize, index: usize, ev: &mut Vec<Event>) -> bool {
        if index >= self.pack.len() || i >= self.party.len() {
            return false;
        }
        if !self.party[i].alive() {
            say(ev, format!("{} is dead.", self.party[i].name));
            return false;
        }
        let it = self.pack[index].clone();
        let def = c.item(&it.def).clone();
        let name = self.party[i].name.clone();
        match &def.kind {
            ItemKind::Potion { effect } => {
                self.pack.remove(index);
                ev.push(Event::PackChanged);
                say(ev, format!("{name} drinks the {}.", self.knowledge.name(c, &it)));
                self.knowledge.identified.insert(it.def.clone());
                self.apply_effect(c, i, effect, ev);
                say(ev, format!("It was a {}.", def.name));
                true
            }
            ItemKind::Scroll { effect } => {
                self.pack.remove(index);
                ev.push(Event::PackChanged);
                say(ev, format!("{name} reads the {}.", self.knowledge.name(c, &it)));
                self.knowledge.identified.insert(it.def.clone());
                self.apply_effect(c, i, effect, ev);
                say(ev, format!("It was a {}.", def.name));
                true
            }
            ItemKind::Wand { effect, .. } => {
                if it.charges <= 0 {
                    say(ev, format!("{name} waves the wand. Nothing: it is empty."));
                    self.pack[index].known = true;
                    return true;
                }
                self.pack[index].charges -= 1;
                say(ev, format!("{name} zaps the {}.", self.knowledge.name(c, &it)));
                self.knowledge.identified.insert(it.def.clone());
                self.apply_effect(c, i, effect, ev);
                ev.push(Event::PackChanged);
                true
            }
            ItemKind::Weapon { .. } | ItemKind::Armor { .. } | ItemKind::Ring { .. } | ItemKind::Amulet { .. } => self.equip(c, i, index, ev),
            ItemKind::Key { .. } => {
                say(ev, "Face a locked door or chest and use it.");
                false
            }
            ItemKind::Crown => {
                say(ev, "The crown is heavy and cold.");
                false
            }
            ItemKind::Gold { .. } => false,
        }
    }

    fn identify_some(&mut self, c: &Content, n: usize, ev: &mut Vec<Event>) {
        let mut done = 0;
        for idx in 0..self.pack.len() {
            if done >= n {
                break;
            }
            let it = &self.pack[idx];
            let unknown_kind = !self.knowledge.kind_known(c, &it.def);
            let unknown_enchant = !it.known;
            if unknown_kind || unknown_enchant {
                let def = it.def.clone();
                self.knowledge.identified.insert(def);
                self.pack[idx].known = true;
                let nm = self.knowledge.name(c, &self.pack[idx]);
                say(ev, format!("Identified: {nm}."));
                done += 1;
            }
        }
        for m in 0..self.party.len() {
            for slot in Slot::ALL {
                if done >= n {
                    break;
                }
                if let Some(it) = self.party[m].equip.get_mut(&slot)
                    && !it.known {
                        it.known = true;
                        done += 1;
                    }
            }
        }
        if done == 0 {
            say(ev, "You already know everything you carry.");
        }
        ev.push(Event::PackChanged);
    }

    fn apply_effect(&mut self, c: &Content, i: usize, effect: &Effect, ev: &mut Vec<Event>) {
        let name = self.party[i].name.clone();
        match effect {
            Effect::Heal(d) => {
                let v = d.roll(&mut self.rng.world);
                self.heal(i, v, ev);
                self.party[i].poison = 0;
            }
            Effect::Mana(d) => {
                let v = d.roll(&mut self.rng.world);
                let m = &mut self.party[i];
                m.mp = (m.mp + v).min(m.max_mp);
                say(ev, if m.max_mp > 0 { format!("{name}'s mind clears.") } else { format!("{name} feels a buzz, and nothing more.") });
            }
            Effect::Cure => {
                self.party[i].poison = 0;
                say(ev, format!("{name} feels clean."));
            }
            Effect::Poison(d) => {
                let v = d.roll(&mut self.rng.world).max(1);
                self.party[i].poison += (v * 2) as u16;
                say(ev, format!("{name} feels very sick."));
            }
            Effect::Strength => {
                let m = &mut self.party[i];
                m.attrs[Attr::Str as usize] = (m.attrs[Attr::Str as usize] + 1).min(25);
                say(ev, format!("{name} feels stronger."));
            }
            Effect::Haste(t) => {
                self.haste += t;
                say(ev, "Everything around you slows down.");
            }
            Effect::Levitate(t) => {
                self.levitate += (*t).max(10);
                say(ev, "The party floats up off the floor.");
            }
            Effect::Identify => {
                let n = self.rng.world.range(1, 3) as usize;
                self.identify_some(c, n, ev);
            }
            Effect::RemoveCurse => {
                for m in &mut self.party {
                    for it in m.equip.values_mut() {
                        if it.cursed {
                            it.cursed = false;
                            if it.enchant < 0 {
                                it.enchant = 0;
                            }
                        }
                    }
                }
                for it in &mut self.pack {
                    it.cursed = false;
                }
                say(ev, "You feel as if someone is watching over you.");
                ev.push(Event::PackChanged);
            }
            Effect::Teleport => self.teleport(c, ev),
            Effect::Mapping => {
                let s = self.level.world.size;
                for x in 0..s.x {
                    for z in 0..s.z {
                        if (1..s.y - 1).any(|y| self.level.open(IVec3::new(x, y, z)) && self.level.supported(IVec3::new(x, y, z))) {
                            self.level.explored.insert((x, z));
                        }
                    }
                }
                say(ev, "A map of the level unfolds in your mind.");
            }
            Effect::EnchantWeapon | Effect::EnchantArmor => {
                let slots: &[Slot] = if *effect == Effect::EnchantWeapon {
                    &[Slot::Weapon]
                } else {
                    &[Slot::Body, Slot::Shield, Slot::Head, Slot::Feet]
                };
                let slot = slots.iter().copied().find(|s| self.party[i].equip.contains_key(s));
                let Some(slot) = slot else {
                    say(ev, format!("{name}'s hands tingle, then nothing."));
                    return;
                };
                let it = self.party[i].equip.get_mut(&slot).unwrap();
                // REF R12: broken on purpose. OURS: past +5 the item can't hold any more and is gone.
                if it.enchant >= 5 {
                    self.party[i].equip.remove(&slot);
                    say(ev, format!("{name}'s gear glows white-hot and evaporates!"));
                    return;
                }
                it.enchant += 1;
                it.cursed = false;
                it.known = true;
                say(ev, format!("{name}'s {} glows blue.", slot.name()));
            }
            Effect::Fire(d) => {
                let v = d.roll(&mut self.rng.combat);
                self.bolt(c, 8, v, true, false, ev);
            }
            Effect::Lightning(d) => {
                let v = d.roll(&mut self.rng.combat);
                self.bolt(c, 8, v, false, true, ev);
            }
            Effect::Dig => {
                let mut p = self.pos;
                let mut changed = Vec::new();
                for _ in 0..5 {
                    p += self.facing.delta();
                    if self.level.features.get(&p) == Some(&Feature::SecretWall) {
                        self.reveal(p, ev);
                        continue;
                    }
                    let cell = self.level.world.get(p);
                    if cell == Cell::Bedrock || !self.level.world.in_bounds(p) || p.x <= 0 || p.z <= 0 || p.x >= self.level.world.size.x - 1 || p.z >= self.level.world.size.z - 1 {
                        break;
                    }
                    if cell.is_solid() {
                        if self.level.features.contains_key(&p) {
                            self.level.features.remove(&p);
                            ev.push(Event::FeatureChanged(p));
                        }
                        self.level.world.set(p, Cell::Air);
                        changed.push(p);
                    }
                }
                if changed.is_empty() {
                    say(ev, "The rock shivers, but holds.");
                } else {
                    say(ev, "The rock crumbles away before you!");
                    ev.push(Event::CellsChanged(changed));
                }
            }
            Effect::Sleep => {
                let ts = self.all_in_line(6);
                for t in &ts {
                    let d = Dice::new(2, 6, 5).roll(&mut self.rng.combat) as u16;
                    self.level.monsters[*t].sleep = d;
                }
                say(ev, if ts.is_empty() { "A drowsy mist drifts away." } else { "A drowsy mist rolls ahead. Something yawns." });
            }
            Effect::Raise => self.raise(ev),
            Effect::Nothing => say(ev, "Nothing seems to happen."),
            Effect::Protection(_) | Effect::Regeneration | Effect::Teleportitis | Effect::Aggravate | Effect::Jumping | Effect::SeeSecrets | Effect::Resist(..) => {
                say(ev, "Nothing seems to happen.")
            }
        }
    }

    fn teleport(&mut self, c: &Content, ev: &mut Vec<Event>) {
        let s = self.level.world.size;
        for _ in 0..500 {
            let x = self.rng.world.range(1, s.x - 2);
            let z = self.rng.world.range(1, s.z - 2);
            if let Some(y) = self.level.ground_below(x, z, s.y - 2) {
                let p = IVec3::new(x, y, z);
                if self.can_enter(p) && !self.level.features.contains_key(&p) && self.level.world.get(p) != Cell::Lava {
                    let from = self.pos;
                    self.pos = p;
                    ev.push(Event::PartyMoved { from, to: p, kind: MoveKind::Teleport });
                    say(ev, "You are somewhere else.");
                    self.entered(c, ev);
                    return;
                }
            }
        }
        say(ev, "You feel a wrench, but stay put.");
    }

    fn equip(&mut self, c: &Content, i: usize, index: usize, ev: &mut Vec<Event>) -> bool {
        if index >= self.pack.len() || i >= self.party.len() || !self.party[i].alive() {
            return false;
        }
        let it = self.pack[index].clone();
        let kind = c.item(&it.def).kind.clone();
        let Some(slot) = kind.slot() else {
            say(ev, "That can't be worn or wielded.");
            return false;
        };
        let name = self.party[i].name.clone();
        let two_handed = matches!(kind, ItemKind::Weapon { two_handed: true, .. });
        // what has to come off first
        let mut off = vec![slot];
        if two_handed {
            off.push(Slot::Shield);
        }
        if slot == Slot::Shield
            && let Some(w) = self.party[i].equip.get(&Slot::Weapon)
                && matches!(c.item(&w.def).kind, ItemKind::Weapon { two_handed: true, .. }) {
                    off.push(Slot::Weapon);
                }
        for s in &off {
            if let Some(w) = self.party[i].equip.get(s)
                && w.cursed {
                    let wn = self.knowledge.name(c, w);
                    self.party[i].equip.get_mut(s).unwrap().known = true;
                    say(ev, format!("{name}'s {wn} is cursed and won't come off!"));
                    return true;
                }
        }
        let needed = off.iter().filter(|s| self.party[i].equip.contains_key(s)).count();
        if self.pack.len() - 1 + needed > PACK_SIZE {
            say(ev, "No room in the pack for what comes off.");
            return false;
        }
        let mut new = self.pack.remove(index);
        for s in &off {
            if let Some(old) = self.party[i].equip.remove(s) {
                self.pack.push(old);
            }
        }
        new.known = true;
        if matches!(kind, ItemKind::Ring { .. } | ItemKind::Amulet { .. }) {
            // OURS: you learn what jewellery is once it's on
            self.knowledge.identified.insert(new.def.clone());
        }
        let nm = self.knowledge.name(c, &new);
        say(ev, format!("{name} puts on the {nm}."));
        if new.cursed {
            say(ev, "It tightens. It is cursed!");
        }
        self.party[i].equip.insert(slot, new);
        ev.push(Event::PackChanged);
        true
    }

    fn unequip(&mut self, c: &Content, i: usize, slot: Slot, ev: &mut Vec<Event>) -> bool {
        let Some(m) = self.party.get(i) else { return false };
        let Some(it) = m.equip.get(&slot) else { return false };
        if it.cursed {
            let nm = self.knowledge.name(c, it);
            say(ev, format!("The {nm} is cursed and won't come off!"));
            self.party[i].equip.get_mut(&slot).unwrap().known = true;
            return false;
        }
        if self.pack.len() >= PACK_SIZE {
            say(ev, "The pack is full.");
            return false;
        }
        let it = self.party[i].equip.remove(&slot).unwrap();
        say(ev, format!("{} takes off the {}.", self.party[i].name, self.knowledge.name(c, &it)));
        self.pack.push(it);
        ev.push(Event::PackChanged);
        true
    }

    // ---------------------------------------------------------------- the world's turn

    /// Poison, regeneration, lava, curses, and timers running out.
    fn status(&mut self, c: &Content, ev: &mut Vec<Event>) {
        let t = self.tick;
        if self.haste > 0 {
            self.haste -= 1;
            if self.haste == 0 {
                say(ev, "You feel yourself slow down.");
            }
        }
        if self.levitate > 0 {
            self.levitate -= 1;
            if self.levitate == 0 && !self.levitating(c) {
                say(ev, "You float gently down.");
                self.gravity(c, ev);
            }
        }
        for i in 0..self.party.len() {
            if !self.party[i].alive() {
                continue;
            }
            if self.party[i].poison > 0 {
                self.party[i].poison -= 1;
                if t.is_multiple_of(2) {
                    self.harm(c, i, 1, Element::Poison, "poison", ev);
                }
                if self.party[i].poison == 0 && self.party[i].alive() {
                    say(ev, format!("{} feels the poison pass.", self.party[i].name));
                }
            }
            let m = &self.party[i];
            let regen = member_effect(c, m, |e| matches!(e, Effect::Regeneration));
            // OURS: everyone heals 1 every 20 ticks; casters regain 1 mana every 8
            if (regen && t.is_multiple_of(3)) || t.is_multiple_of(20) {
                self.heal(i, 1, ev);
            }
            let m = &mut self.party[i];
            if m.max_mp > 0 && t.is_multiple_of(8) {
                m.mp = (m.mp + 1).min(m.max_mp);
            }
        }
        if self.level.world.get(self.pos) == Cell::Lava {
            say(ev, "You are burning!");
            for i in self.living().collect::<Vec<_>>() {
                let d = Dice::new(2, 6, 0).roll(&mut self.rng.combat);
                self.harm(c, i, d, Element::Fire, "lava", ev);
            }
        }
        if self.party_effect(c, |e| matches!(e, Effect::Teleportitis)) && self.rng.world.chance(3) {
            self.teleport(c, ev);
        }
    }

    fn monsters_act(&mut self, c: &Content, ev: &mut Vec<Event>) {
        let aggravate = self.party_effect(c, |e| matches!(e, Effect::Aggravate));
        let hasted = self.haste > 0;
        for idx in 0..self.level.monsters.len() {
            if self.level.monsters[idx].hp <= 0 {
                continue;
            }
            let def = c.monster(&self.level.monsters[idx].kind);
            let gain = if hasted { def.speed as i32 / 2 } else { def.speed as i32 };
            self.level.monsters[idx].energy += gain;
            while self.level.monsters[idx].energy >= ACT_COST && self.level.monsters[idx].hp > 0 && self.outcome == Outcome::Playing {
                self.level.monsters[idx].energy -= ACT_COST;
                self.monster_act(c, idx, aggravate, ev);
            }
        }
    }

    fn monster_act(&mut self, c: &Content, idx: usize, aggravate: bool, ev: &mut Vec<Event>) {
        let def = c.monster(&self.level.monsters[idx].kind).clone();
        let m = &mut self.level.monsters[idx];
        if m.sleep > 0 {
            m.sleep -= 1;
            return;
        }
        let pos = m.pos;
        let dist = pos.flat_dist(self.pos) + (pos.y - self.pos.y).abs();
        let sees = dist <= def.sight as i32 && monster::line_of_sight(&self.level, pos, self.pos);
        let m = &mut self.level.monsters[idx];
        if !m.awake {
            if aggravate || (sees && self.rng.ai.chance(50)) {
                m.awake = true;
            } else {
                return;
            }
        }
        let adjacent = pos.y == self.pos.y && pos.flat_manhattan(self.pos) == 1;
        // OURS: the living flee when nearly dead, half the time
        let fleeing = !def.undead && !def.special && m.hp * 4 < m.max_hp && self.rng.ai.chance(50);
        if fleeing {
            let options = monster::moves_from(&self.level, &def, pos);
            let best = options
                .into_iter()
                .filter(|&t| self.level.monster_at(t).is_none() && t != self.pos)
                .max_by_key(|t| (t.flat_manhattan(self.pos), std::cmp::Reverse(*t)));
            if let Some(t) = best
                && t.flat_manhattan(self.pos) > pos.flat_manhattan(self.pos) {
                    self.move_monster(idx, t, ev);
                    return;
                }
        }
        if adjacent {
            if let Some(d) = Dir::between(pos, self.pos) {
                self.level.monsters[idx].facing = d;
            }
            self.monster_attack(c, idx, false, ev);
            return;
        }
        if let Some(range) = def.ranged {
            let aligned = pos.y == self.pos.y && (pos.x == self.pos.x || pos.z == self.pos.z);
            if aligned && dist <= range as i32 && sees {
                if let Some(d) = Dir::between(pos, pos + IVec3::new((self.pos.x - pos.x).signum(), 0, (self.pos.z - pos.z).signum())) {
                    self.level.monsters[idx].facing = d;
                }
                ev.push(Event::Bolt { from: pos, to: self.pos, fire: false });
                self.monster_attack(c, idx, true, ev);
                return;
            }
        }
        if (sees || aggravate || dist <= 3)
            && let Some(t) = monster::step_towards(&self.level, &def, pos, self.pos, 14) {
                if t != self.pos && self.level.monster_at(t).is_none() {
                    self.move_monster(idx, t, ev);
                }
                return;
            }
        // wander
        if self.rng.ai.chance(40) {
            let options: Vec<IVec3> = monster::moves_from(&self.level, &def, pos)
                .into_iter()
                .filter(|&t| self.level.monster_at(t).is_none() && t != self.pos)
                .collect();
            if let Some(&t) = self.rng.ai.pick(&options) {
                self.move_monster(idx, t, ev);
            }
        }
    }

    fn move_monster(&mut self, idx: usize, to: IVec3, ev: &mut Vec<Event>) {
        let m = &mut self.level.monsters[idx];
        let from = m.pos;
        if let Some(d) = Dir::between(from, to) {
            m.facing = d;
        }
        m.pos = to;
        ev.push(Event::MonsterMoved { id: m.id, from, to });
    }

    fn monster_attack(&mut self, c: &Content, idx: usize, ranged: bool, ev: &mut Vec<Event>) {
        let (id, kind) = (self.level.monsters[idx].id, self.level.monsters[idx].kind.clone());
        let def = c.monster(&kind);
        // OURS: monsters strike the front row; the back row only once the front has fallen, or by arrows
        let front: Vec<usize> = self.living().filter(|&i| i < 2).collect();
        let pool: Vec<usize> = if front.is_empty() || ranged { self.living().collect() } else { front };
        let Some(&target) = self.rng.ai.pick(&pool) else { return };
        let ac = self.party[target].ac(c);
        let roll = self.rng.combat.range(1, 20);
        let hit = roll == 20 || (roll != 1 && roll + def.to_hit as i32 >= ac);
        let mut damage = 0;
        let tname = self.party[target].name.clone();
        if hit {
            damage = def.damage.roll(&mut self.rng.combat).max(1);
            if roll == 20 {
                damage *= 2;
            }
        }
        ev.push(Event::MonsterAttack { id, target: target as u8, hit, damage, ranged });
        if hit {
            say(ev, format!("The {} {} {tname} for {damage}.", def.name, if ranged { "shoots" } else { "hits" }));
            let cause = format!("a {}", def.name);
            self.harm(c, target, damage, def.element, &cause, ev);
            match def.on_hit {
                Some(OnHit::Poison) if self.party[target].alive() && self.rng.combat.chance(40) => {
                    self.party[target].poison += 6;
                    say(ev, format!("{tname} is poisoned!"));
                }
                Some(OnHit::DrainMana) => {
                    let m = &mut self.party[target];
                    if m.mp > 0 {
                        m.mp = (m.mp - 3).max(0);
                        say(ev, format!("{tname} feels their magic drain away."));
                    }
                }
                _ => {}
            }
        } else {
            say(ev, format!("The {} misses {tname}.", def.name));
        }
    }

    /// The automap (R35): mark the columns the party can see from where it stands. OURS: radius 7, along
    /// lines at the party's height.
    pub fn update_explored(&mut self) {
        let r = 7;
        let p = self.pos;
        for dz in -r..=r {
            for dx in -r..=r {
                if dx * dx + dz * dz > r * r + 1 {
                    continue;
                }
                let q = IVec3::new(p.x + dx, p.y, p.z + dz);
                if !self.level.world.in_bounds(q) {
                    continue;
                }
                if monster::line_of_sight(&self.level, p, q) {
                    self.level.explored.insert((q.x, q.z));
                }
            }
        }
    }
}

fn member_effect(c: &Content, m: &Character, f: impl Fn(&Effect) -> bool) -> bool {
    m.equip.values().any(|it| match &c.item(&it.def).kind {
        ItemKind::Ring { effect } | ItemKind::Amulet { effect } => f(effect),
        ItemKind::Armor { effect: Some(effect), .. } => f(effect),
        _ => false,
    })
}

/// A whole run from a seed and a list of actions: what a recording holds (board row F4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recording {
    pub seed: String,
    #[serde(default)]
    pub options: RunOptions,
    pub actions: Vec<Action>,
    /// The state hash after the last action, in hex; checked by the replay test.
    #[serde(default)]
    pub expect: Option<String>,
}

impl Recording {
    pub fn play(&self, content: Content) -> Sim {
        let mut sim = Sim::new(&self.seed, self.options, content);
        for a in &self.actions {
            sim.tick(*a);
        }
        sim
    }
}

/// Keep the rng module's type in the public API for the app's seed roller.
pub type Rng = Mt19937;
