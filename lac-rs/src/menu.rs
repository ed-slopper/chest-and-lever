//! The screens around a run (board rows X2, X4, G11): the splash, the title, a new run with a seed, options
//! and key rebinding, and the end screen.
//!
//! REF R55: the splash screen has a stylised, pixelated LCD-display look (ours: green dot-matrix letters).
//! REF R25: runs are seeded, so the seed is shown and can be typed, to share a dungeon. REF R43: the
//! spider-free option.

use crate::hud::{font, DIM, GOLD, INK};
use crate::palette::Vision;
use crate::settings::{self, Command, Settings};
use crate::{icons, Screen, StartRun};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use lac_sim::{Outcome, Sim};

#[derive(Component)]
pub struct MenuRoot;

#[derive(Resource, Default)]
pub struct Menu {
    pub cursor: usize,
    pub seed: String,
    pub rebinding: Option<Command>,
    pub timer: f32,
    pub dirty: bool,
}

#[derive(Resource, Default, Clone)]
pub struct EndSummary {
    pub won: bool,
    pub cause: String,
    pub seed: String,
    pub depth: u8,
    pub ticks: u64,
    pub kills: u32,
    pub gold: u32,
    pub chests: u32,
    pub levers: u32,
    pub levels: Vec<String>,
}

impl EndSummary {
    pub fn from(sim: &Sim) -> Self {
        let s = &sim.state;
        EndSummary {
            won: s.outcome == Outcome::Won,
            cause: match &s.outcome {
                Outcome::Lost { cause } => cause.clone(),
                _ => String::new(),
            },
            seed: s.seed_text.clone(),
            depth: s.stats.deepest,
            ticks: s.tick,
            kills: s.stats.kills,
            gold: s.gold,
            chests: s.stats.chests,
            levers: s.stats.levers,
            levels: s.party.iter().map(|m| format!("{} the {} reached level {}{}", m.name, m.class.name(), m.level, if m.alive() { "" } else { " (fallen)" })).collect(),
        }
    }
}

pub fn clear(mut commands: Commands, q: Query<Entity, With<MenuRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn root(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(10),
                ..default()
            },
            BackgroundColor(Color::srgb(0.02, 0.025, 0.02)),
            MenuRoot,
        ))
        .id()
}

fn logo(commands: &mut Commands, images: &mut Assets<Image>, vision: Vision, height: f32) -> Entity {
    let img = icons::lcd_text("CHEST AND LEVER", vision);
    let (w, h) = (img.width() as f32, img.height() as f32);
    let handle = images.add(img);
    commands.spawn((ImageNode::new(handle), Node { width: px(height * w / h), height: px(height), ..default() })).id()
}

fn text(commands: &mut Commands, t: impl Into<String>, size: f32, colour: Color) -> Entity {
    commands.spawn((Text::new(t.into()), font(size), TextColor(colour))).id()
}

// ------------------------------------------------------------------ splash (R55)

pub fn splash_enter(mut commands: Commands, mut images: ResMut<Assets<Image>>, settings: Res<Settings>, mut menu: ResMut<Menu>) {
    menu.timer = 0.0;
    let r = root(&mut commands);
    let l = logo(&mut commands, &mut images, settings.file.vision, 150.0);
    let sub = text(&mut commands, "a first-person dungeon crawl for a party of four", 16.0, Color::srgb(0.45, 0.8, 0.45));
    let fan = text(&mut commands, "an unofficial fan game, made in the open", 12.0, Color::srgb(0.3, 0.5, 0.3));
    commands.entity(r).add_children(&[l, sub, fan]);
}

pub fn splash_update(time: Res<Time>, keys: Res<ButtonInput<KeyCode>>, mut menu: ResMut<Menu>, mut next: ResMut<NextState<Screen>>) {
    menu.timer += time.delta_secs();
    if menu.timer > 2.8 || (menu.timer > 0.3 && keys.get_just_pressed().next().is_some()) {
        next.set(Screen::Title);
    }
}

// ------------------------------------------------------------------ title

fn title_items() -> Vec<&'static str> {
    let mut v = Vec::new();
    if settings::save_path().exists() {
        v.push("Continue");
    }
    v.extend(["New run", "Options", "Quit"]);
    v
}

pub fn title_enter(mut menu: ResMut<Menu>) {
    menu.cursor = 0;
    menu.dirty = true;
}

#[allow(clippy::too_many_arguments)]
pub fn title_update(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    mut menu: ResMut<Menu>,
    mut next: ResMut<NextState<Screen>>,
    mut start: ResMut<StartRun>,
    old: Query<Entity, With<MenuRoot>>,
    mut exit: MessageWriter<AppExit>,
) {
    let items = title_items();
    if nav(&keys, &mut menu.cursor, items.len()) {
        menu.dirty = true;
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        match items[menu.cursor] {
            "Continue" => {
                start.load = true;
                next.set(Screen::Playing);
            }
            "New run" => next.set(Screen::NewRun),
            "Options" => next.set(Screen::Options),
            _ => {
                exit.write(AppExit::Success);
            }
        }
        return;
    }
    if !std::mem::take(&mut menu.dirty) {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let r = root(&mut commands);
    let l = logo(&mut commands, &mut images, settings.file.vision, 90.0);
    let gap = commands.spawn(Node { height: px(24), ..default() }).id();
    commands.entity(r).add_children(&[l, gap]);
    for (k, it) in items.iter().enumerate() {
        let sel = k == menu.cursor;
        let t = text(&mut commands, format!("{} {it} {}", if sel { ">" } else { " " }, if sel { "<" } else { " " }), 22.0, if sel { GOLD } else { INK });
        commands.entity(r).add_child(t);
    }
    let gap = commands.spawn(Node { height: px(24), ..default() }).id();
    let help = text(&mut commands, "Up/Down and Enter.  Descend eight levels and open the great chest.", 13.0, DIM);
    commands.entity(r).add_children(&[gap, help]);
}

fn nav(keys: &ButtonInput<KeyCode>, cursor: &mut usize, n: usize) -> bool {
    if n == 0 {
        return false;
    }
    if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
        *cursor = (*cursor + n - 1) % n;
        return true;
    }
    if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
        *cursor = (*cursor + 1) % n;
        return true;
    }
    if *cursor >= n {
        *cursor = 0;
        return true;
    }
    false
}

// ------------------------------------------------------------------ a new run: the seed (R25)

fn roll(time: &Time) -> String {
    let e = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0) ^ time.elapsed().as_nanos() as u64;
    lac_sim::rng::seed_words(e)
}

pub fn newrun_enter(mut menu: ResMut<Menu>, time: Res<Time>) {
    menu.seed = roll(&time);
    menu.dirty = true;
}

#[allow(clippy::too_many_arguments)]
pub fn newrun_update(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    time: Res<Time>,
    settings: Res<Settings>,
    mut menu: ResMut<Menu>,
    mut next: ResMut<NextState<Screen>>,
    mut start: ResMut<StartRun>,
    old: Query<Entity, With<MenuRoot>>,
) {
    for ev in typed.read() {
        if !ev.state.is_pressed() {
            continue;
        }
        match &ev.logical_key {
            Key::Character(c) => {
                for ch in c.chars() {
                    if (ch.is_ascii_alphanumeric() || ch == '-' || ch == ' ') && menu.seed.len() < 32 {
                        menu.seed.push(ch.to_ascii_uppercase());
                        menu.dirty = true;
                    }
                }
            }
            Key::Space if menu.seed.len() < 32 => {
                menu.seed.push(' ');
                menu.dirty = true;
            }
            Key::Backspace => {
                menu.seed.pop();
                menu.dirty = true;
            }
            _ => {}
        }
    }
    if keys.just_pressed(KeyCode::Tab) {
        menu.seed = roll(&time);
        menu.dirty = true;
    }
    if keys.just_pressed(KeyCode::Escape) {
        next.set(Screen::Title);
        return;
    }
    if keys.just_pressed(KeyCode::Enter) && !menu.seed.trim().is_empty() {
        start.seed = Some(menu.seed.trim().to_string());
        start.load = false;
        next.set(Screen::Playing);
        return;
    }
    if !std::mem::take(&mut menu.dirty) {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let r = root(&mut commands);
    let a = text(&mut commands, "A new run", 26.0, GOLD);
    let b = text(&mut commands, "Seed (type your own, or share this one: the same seed is the same dungeon)", 14.0, DIM);
    let c = text(&mut commands, format!("[ {}_ ]", menu.seed), 28.0, INK);
    let d = text(&mut commands, format!("Spider-free mode: {}  (change in Options)", if settings.file.spider_free { "on" } else { "off" }), 13.0, DIM);
    let e = text(&mut commands, "Enter: begin    Tab: roll another    Esc: back", 14.0, DIM);
    commands.entity(r).add_children(&[a, b, c, d, e]);
}

// ------------------------------------------------------------------ options (X4) and keys (E6)

const OPTION_ROWS: usize = 8;

pub fn options_enter(mut menu: ResMut<Menu>) {
    menu.cursor = 0;
    menu.rebinding = None;
    menu.dirty = true;
}

#[allow(clippy::too_many_arguments)]
pub fn options_update(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<Settings>,
    mut menu: ResMut<Menu>,
    mut next: ResMut<NextState<Screen>>,
    old: Query<Entity, With<MenuRoot>>,
    mut volume: ResMut<GlobalVolume>,
) {
    if nav(&keys, &mut menu.cursor, OPTION_ROWS) {
        menu.dirty = true;
    }
    let left = keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA);
    let right = keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD);
    let enter = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);
    let step = if right { 0.1 } else if left { -0.1 } else { 0.0 };
    let f = &mut settings.file;
    let mut changed = false;
    match menu.cursor {
        0 if left || right || enter => {
            f.spider_free = !f.spider_free;
            changed = true;
        }
        1 if step != 0.0 => {
            f.volume = (f.volume + step).clamp(0.0, 1.0);
            changed = true;
        }
        2 if step != 0.0 => {
            f.music = (f.music + step).clamp(0.0, 1.0);
            changed = true;
        }
        3 if left || right || enter => {
            f.vision = f.vision.next();
            changed = true;
        }
        4 if left || right || enter => {
            f.res_scale = if f.res_scale == 1 { 2 } else { 1 };
            changed = true;
        }
        5 if left || right || enter => {
            f.ssao = !f.ssao;
            changed = true;
        }
        6 if enter => {
            next.set(Screen::Keys);
            return;
        }
        7 if enter => {
            next.set(Screen::Title);
            return;
        }
        _ => {}
    }
    if changed {
        volume.volume = bevy::audio::Volume::Linear(1.0);
        settings.save();
        menu.dirty = true;
    }
    if keys.just_pressed(KeyCode::Escape) {
        next.set(Screen::Title);
        return;
    }
    if !std::mem::take(&mut menu.dirty) {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let f = &settings.file;
    let rows = [
        format!("Spider-free mode (spiders become beetles): {}", if f.spider_free { "on" } else { "off" }),
        format!("Sound volume: {:.0}%", f.volume * 100.0),
        format!("Music volume: {:.0}%", f.music * 100.0),
        format!("Colours: {}", f.vision.name()),
        format!("Resolution: {}x{}", 480 * f.res_scale as u32, 270 * f.res_scale as u32),
        format!("Ambient occlusion (SSAO): {}", if f.ssao { "on" } else { "off" }),
        "Key bindings...".to_string(),
        "Back".to_string(),
    ];
    let r = root(&mut commands);
    let t = text(&mut commands, "Options", 26.0, GOLD);
    commands.entity(r).add_child(t);
    for (k, row) in rows.iter().enumerate() {
        let sel = k == menu.cursor;
        let e = text(&mut commands, format!("{} {row}", if sel { ">" } else { " " }), 18.0, if sel { GOLD } else { INK });
        commands.entity(r).add_child(e);
    }
    let h = text(&mut commands, "Left/Right to change. The look takes effect on the next run you start or continue.", 13.0, DIM);
    commands.entity(r).add_child(h);
}

pub fn keys_update(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<Settings>,
    mut menu: ResMut<Menu>,
    mut next: ResMut<NextState<Screen>>,
    old: Query<Entity, With<MenuRoot>>,
) {
    let n = Command::ALL.len() + 2;
    if let Some(c) = menu.rebinding {
        if let Some(k) = keys.get_just_pressed().next() {
            if settings::key_name(*k) != "?" {
                settings.rebind(c, *k);
                settings.save();
            }
            menu.rebinding = None;
            menu.dirty = true;
        }
    } else {
        if nav(&keys, &mut menu.cursor, n) {
            menu.dirty = true;
        }
        if keys.just_pressed(KeyCode::Enter) {
            if menu.cursor < Command::ALL.len() {
                menu.rebinding = Some(Command::ALL[menu.cursor]);
            } else if menu.cursor == Command::ALL.len() {
                settings.file.bindings = settings::SettingsFile::default().bindings;
                settings.save();
            } else {
                next.set(Screen::Options);
                return;
            }
            menu.dirty = true;
        }
        if keys.just_pressed(KeyCode::Escape) {
            next.set(Screen::Options);
            return;
        }
    }
    if !std::mem::take(&mut menu.dirty) {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let r = root(&mut commands);
    let t = text(&mut commands, "Key bindings", 24.0, GOLD);
    commands.entity(r).add_child(t);
    for (k, c) in Command::ALL.iter().enumerate() {
        let sel = k == menu.cursor;
        let val = if menu.rebinding == Some(*c) { "press a key...".to_string() } else { settings.describe(*c) };
        let e = text(&mut commands, format!("{} {:<16} {val}", if sel { ">" } else { " " }, c.name()), 15.0, if sel { GOLD } else { INK });
        commands.entity(r).add_child(e);
    }
    for (k, label) in ["Reset to defaults", "Back"].iter().enumerate() {
        let sel = menu.cursor == Command::ALL.len() + k;
        let e = text(&mut commands, format!("{} {label}", if sel { ">" } else { " " }), 15.0, if sel { GOLD } else { INK });
        commands.entity(r).add_child(e);
    }
    let h = text(&mut commands, "Enter: rebind (the key moves off any other command).  1-4 always pick a party member.", 12.0, DIM);
    commands.entity(r).add_child(h);
}

pub fn keys_enter(mut menu: ResMut<Menu>) {
    menu.cursor = 0;
    menu.rebinding = None;
    menu.dirty = true;
}

// ------------------------------------------------------------------ the end (G11)

pub fn end_enter(mut commands: Commands, summary: Res<EndSummary>) {
    let r = root(&mut commands);
    let s = &*summary;
    let head = if s.won { "The Sunken Crown is yours." } else { "The dungeon keeps them." };
    let a = text(&mut commands, head, 28.0, if s.won { GOLD } else { Color::srgb(0.85, 0.35, 0.3) });
    let b = text(&mut commands, if s.won { "You opened the great chest at the bottom of the dungeon.".to_string() } else { format!("Fell at depth {}, killed by {}.", s.depth, s.cause) }, 16.0, INK);
    commands.entity(r).add_children(&[a, b]);
    for row in [
        format!("Seed: {}", s.seed),
        format!("Deepest: {}    Ticks: {}    Monsters slain: {}", s.depth, s.ticks, s.kills),
        format!("Gold: {}    Chests opened: {}    Levers pulled: {}", s.gold, s.chests, s.levers),
    ] {
        let e = text(&mut commands, row, 15.0, DIM);
        commands.entity(r).add_child(e);
    }
    for l in &s.levels {
        let e = text(&mut commands, l.clone(), 14.0, INK);
        commands.entity(r).add_child(e);
    }
    let e = text(&mut commands, "Enter: back to the title", 14.0, DIM);
    commands.entity(r).add_child(e);
}

pub fn end_update(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<Screen>>) {
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Escape) {
        next.set(Screen::Title);
    }
}
