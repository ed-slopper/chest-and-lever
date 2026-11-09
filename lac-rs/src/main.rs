//! Chest and Lever (F5): a first-person, tile-based party dungeon crawler and old-school roguelike.
//!
//! The rules live in `lac-sim` (deterministic, no Bevy). This app is the host: window, GPU, input, sound,
//! menus, and drawing the sim's state. Run: `cargo run --release`. Options:
//! `--seed <text>` start a run at once, `--record <file>` write the run's actions as a recording,
//! `--replay <file>` play a recording back (board row F4), `--test-level` the code-built test level (R47).
//! For testing: `--do "[StepForward, Jump]"` queue actions (RON) once the run starts, `--shot <file>@<secs>`
//! save a screenshot after that many seconds (repeatable), `--quit-after <secs>`, `--spawn a,b,c` stand monsters
//! of those kinds in a row on the test level (to look at their models).

mod audio;
mod game;
mod hud;
mod icons;
mod menu;
mod models;
mod palette;
mod settings;
mod textures;
mod voxel;

use bevy::asset::{io::Reader, AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use lac_sim::{Content, Recording, RunOptions, Sim};
use std::path::PathBuf;

#[derive(States, Default, Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum Screen {
    #[default]
    Splash,
    Title,
    NewRun,
    Options,
    Keys,
    Loading,
    Playing,
    End,
}

/// How the next run starts.
#[derive(Resource, Default)]
pub struct StartRun {
    pub seed: Option<String>,
    pub load: bool,
    pub record: Option<PathBuf>,
    pub replay: Option<Recording>,
    pub test_level: bool,
    /// Start the run on this depth (for testing).
    pub depth: Option<u8>,
    /// Monsters to stand in a row on the test level, for looking at their models.
    pub spawn: Vec<String>,
    pub script: Vec<lac_sim::Action>,
}

/// Screenshots and a timed exit, for checking the game without a person at the keyboard.
#[derive(Resource, Default)]
struct Capture {
    t: f32,
    shots: Vec<(f32, PathBuf)>,
    quit: Option<f32>,
}

fn capture(mut commands: Commands, time: Res<Time>, mut cap: ResMut<Capture>, mut exit: MessageWriter<AppExit>) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    cap.t += time.delta_secs();
    let t = cap.t;
    let due: Vec<PathBuf> = cap.shots.iter().filter(|(at, _)| *at <= t).map(|(_, p)| p.clone()).collect();
    cap.shots.retain(|(at, _)| *at > t);
    for p in due {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(p));
    }
    if cap.quit.is_some_and(|q| t >= q) {
        exit.write(AppExit::Success);
    }
}

// ------------------------------------------------------------------ content as Bevy assets (E10)

#[derive(Asset, TypePath, Debug)]
pub struct ContentText(pub String);

#[derive(Default, TypePath)]
struct ContentLoader;

impl AssetLoader for ContentLoader {
    type Asset = ContentText;
    type Settings = ();
    type Error = std::io::Error;
    async fn load(&self, reader: &mut dyn Reader, _: &(), _: &mut LoadContext<'_>) -> Result<ContentText, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ContentText(String::from_utf8_lossy(&bytes).into_owned()))
    }
    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

/// The content in use. Starts as the copy built into `lac-sim`, so a run can start before any file loads;
/// replaced when the files in `assets/content/` load or change (hot reload in debug builds).
#[derive(Resource)]
pub struct ContentRes(pub Content);

#[derive(Resource)]
struct ContentFiles(Vec<Handle<ContentText>>);

fn load_content(mut commands: Commands, server: Res<AssetServer>) {
    let handles = lac_sim::content::FILES.iter().map(|f| server.load(format!("content/{f}"))).collect();
    commands.insert_resource(ContentFiles(handles));
}

fn reload_content(
    mut events: MessageReader<AssetEvent<ContentText>>,
    files: Res<ContentFiles>,
    texts: Res<Assets<ContentText>>,
    mut content: ResMut<ContentRes>,
    game: Option<ResMut<game::Game>>,
) {
    let changed = events.read().any(|e| matches!(e, AssetEvent::Modified { .. } | AssetEvent::LoadedWithDependencies { .. }));
    if !changed {
        return;
    }
    let all: Option<Vec<&str>> = files.0.iter().map(|h| texts.get(h).map(|t| t.0.as_str())).collect();
    let Some(all) = all else { return };
    let arr: [&str; 5] = [all[0], all[1], all[2], all[3], all[4]];
    match Content::from_texts(&arr) {
        Ok(c) => {
            if c != content.0 {
                content.0 = c.clone();
                if let Some(mut g) = game {
                    g.sim.content = c;
                    g.say("(content files reloaded)");
                }
                info!("content reloaded");
            }
        }
        Err(e) => warn!("content files have an error, keeping the old content: {e}"),
    }
}

// ------------------------------------------------------------------ starting and ending a run

fn start_run(mut commands: Commands, mut start: ResMut<StartRun>, content: Res<ContentRes>, settings: Res<settings::Settings>, time: Res<Time>) {
    let options = RunOptions { spider_free: settings.file.spider_free };
    let c = content.0.clone();
    let mut replay = std::collections::VecDeque::new();
    let sim = if let Some(rec) = start.replay.take() {
        replay = rec.actions.iter().copied().collect();
        Sim::new(&rec.seed, rec.options, c)
    } else if start.test_level {
        start.test_level = false;
        let mut sim = Sim::on_level(lac_sim::Level::test_level(), c);
        sim.state.level.monsters.clear();
        // a row of monsters across the room, facing the party, asleep so they hold still
        let kinds: Vec<String> = start.spawn.drain(..).filter(|k| sim.content.monsters.contains_key(k)).collect();
        for (k, kind) in kinds.iter().enumerate() {
            let at = lac_sim::IVec3::new(2 + (k as i32 % 4) * 2, 1, 8 + (k as i32 / 4) * 2);
            let content = sim.content.clone();
            let id = lac_sim::levelgen::spawn(&mut sim.state.level, &content, kind, at, &mut lac_sim::rng::Mt19937::new(k as u32 + 1), RunOptions::default());
            if let Some(m) = sim.state.level.monsters.iter_mut().find(|m| m.id == id) {
                m.facing = lac_sim::Dir::North;
                m.sleep = 60_000;
            }
        }
        sim
    } else if start.load {
        start.load = false;
        match std::fs::read_to_string(settings::save_path()).ok().and_then(|t| Sim::load(&t, c.clone()).ok()) {
            Some(s) => s,
            None => Sim::new(&lac_sim::rng::seed_words(time.elapsed().as_nanos() as u64), options, c),
        }
    } else {
        let seed = start.seed.take().unwrap_or_else(|| lac_sim::rng::seed_words(time.elapsed().as_nanos() as u64 ^ 0x5eed));
        Sim::new(&seed, options, c)
    };
    let mut sim = sim;
    if let Some(d) = start.depth.take() {
        // testing: start deeper
        let lvl = lac_sim::levelgen::generate(&sim.content, sim.state.seed, d, sim.state.options);
        sim.state.pos = lvl.start;
        sim.state.facing = lvl.start_facing;
        sim.state.level = lvl;
        sim.state.update_explored();
    }
    let mut g = game::Game::new(sim);
    g.record = start.record.clone();
    g.replay = replay;
    g.replay.extend(start.script.drain(..));
    if !g.replay.is_empty() {
        g.say("Playing back a recording. Esc stops it.");
    }
    commands.insert_resource(g);
}

fn start_ambient(mut commands: Commands, sounds: Res<audio::Sounds>, settings: Res<settings::Settings>) {
    audio::start_ambient(&mut commands, &sounds, settings.file.music);
}

fn stop_ambient(mut commands: Commands, q: Query<Entity, With<audio::Ambient>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn setup(mut commands: Commands, mut sources: ResMut<Assets<bevy::audio::AudioSource>>) {
    // the window's own camera, for the UI; the dungeon renders to a low-resolution image (E9)
    commands.spawn((Camera2d, Camera { order: 1, ..default() }));
    commands.insert_resource(audio::build(&mut sources));
}

fn parse_args(start: &mut StartRun, cap: &mut Capture) -> Option<Screen> {
    let args: Vec<String> = std::env::args().collect();
    let mut first = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" if i + 1 < args.len() => {
                start.seed = Some(args[i + 1].clone());
                first = Some(Screen::Playing);
                i += 1;
            }
            "--record" if i + 1 < args.len() => {
                start.record = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--replay" if i + 1 < args.len() => {
                match std::fs::read_to_string(&args[i + 1]).map_err(|e| e.to_string()).and_then(|t| ron::from_str::<Recording>(&t).map_err(|e| e.to_string())) {
                    Ok(r) => {
                        start.replay = Some(r);
                        first = Some(Screen::Playing);
                    }
                    Err(e) => eprintln!("can't read recording {}: {e}", args[i + 1]),
                }
                i += 1;
            }
            "--do" if i + 1 < args.len() => {
                match ron::from_str::<Vec<lac_sim::Action>>(&args[i + 1]) {
                    Ok(a) => start.script = a,
                    Err(e) => eprintln!("can't read actions: {e}"),
                }
                i += 1;
            }
            "--shot" if i + 1 < args.len() => {
                let (p, t) = args[i + 1].rsplit_once('@').unwrap_or((&args[i + 1], "3"));
                cap.shots.push((t.parse().unwrap_or(3.0), PathBuf::from(p)));
                i += 1;
            }
            "--quit-after" if i + 1 < args.len() => {
                cap.quit = args[i + 1].parse().ok();
                i += 1;
            }
            "--test-level" => {
                start.test_level = true;
                first = Some(Screen::Playing);
            }
            "--depth" if i + 1 < args.len() => {
                start.depth = args[i + 1].parse().ok();
                i += 1;
            }
            "--spawn" if i + 1 < args.len() => {
                start.spawn = args[i + 1].split(',').map(|s| s.trim().to_string()).collect();
                start.test_level = true;
                first = Some(Screen::Playing);
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    first
}

fn jump_to(first: Option<Screen>) -> impl FnMut(ResMut<NextState<Screen>>) {
    let mut done = false;
    move |mut next: ResMut<NextState<Screen>>| {
        if !done {
            done = true;
            if let Some(s) = first {
                next.set(s);
            }
        }
    }
}

fn main() {
    let mut start = StartRun::default();
    let mut cap = Capture::default();
    let first = parse_args(&mut start, &mut cap);
    let settings = settings::Settings::load();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window { title: "Chest and Lever".into(), resolution: WindowResolution::new(1560, 900), ..default() }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(MaterialPlugin::<voxel::VoxelMaterial>::default())
        .init_asset::<ContentText>()
        .register_asset_loader(ContentLoader)
        .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.025)))
        .insert_resource(settings)
        .insert_resource(start)
        .insert_resource(cap)
        .insert_resource(ContentRes(Content::builtin()))
        .init_resource::<menu::Menu>()
        .init_resource::<menu::EndSummary>()
        .init_state::<Screen>()
        .add_systems(Startup, (setup, load_content))
        .add_systems(Update, (jump_to(first), reload_content, capture))
        .add_systems(OnEnter(Screen::Splash), menu::splash_enter)
        .add_systems(Update, menu::splash_update.run_if(in_state(Screen::Splash)))
        .add_systems(OnExit(Screen::Splash), menu::clear)
        .add_systems(OnEnter(Screen::Title), menu::title_enter)
        .add_systems(Update, menu::title_update.run_if(in_state(Screen::Title)))
        .add_systems(OnExit(Screen::Title), menu::clear)
        .add_systems(OnEnter(Screen::NewRun), menu::newrun_enter)
        .add_systems(Update, menu::newrun_update.run_if(in_state(Screen::NewRun)))
        .add_systems(OnExit(Screen::NewRun), menu::clear)
        .add_systems(OnEnter(Screen::Options), menu::options_enter)
        .add_systems(Update, menu::options_update.run_if(in_state(Screen::Options)))
        .add_systems(OnExit(Screen::Options), menu::clear)
        .add_systems(OnEnter(Screen::Keys), menu::keys_enter)
        .add_systems(Update, menu::keys_update.run_if(in_state(Screen::Keys)))
        .add_systems(OnExit(Screen::Keys), menu::clear)
        .add_systems(OnEnter(Screen::End), menu::end_enter)
        .add_systems(Update, menu::end_update.run_if(in_state(Screen::End)))
        .add_systems(OnExit(Screen::End), menu::clear)
        .add_systems(OnEnter(Screen::Loading), menu::loading_enter)
        .add_systems(Update, menu::loading_update.run_if(in_state(Screen::Loading)))
        .add_systems(OnExit(Screen::Loading), menu::clear)
        .add_observer(hud::on_click)
        .add_observer(hud::on_over)
        .add_observer(hud::on_out)
        .init_resource::<hud::Hover>()
        .add_systems(Update, (hud::update_tooltip, hud::update_slash).run_if(in_state(Screen::Playing)))
        .add_systems(OnEnter(Screen::Playing), (start_run, game::spawn_world, hud::spawn_hud, start_ambient).chain())
        .add_systems(
            Update,
            (game::input, game::playback, game::rebuild_chunks, game::rebuild_features, game::animate, game::flicker, hud::update_hud)
                .chain()
                .run_if(in_state(Screen::Playing)),
        )
        .add_systems(Update, (game::brightness_keys, game::apply_exposure, game::animate_doors).chain().run_if(in_state(Screen::Playing)))
        .add_systems(Update, game::monster_bars.after(game::animate).run_if(in_state(Screen::Playing)))
        .add_systems(Startup, (game::setup_bars, game::setup_particles))
        .add_systems(Update, game::particles.run_if(in_state(Screen::Playing)))
        .add_systems(Update, hud::fit_view.run_if(in_state(Screen::Playing)))
        .add_systems(OnExit(Screen::Playing), (hud::despawn_hud, stop_ambient, game::despawn_world).chain())
        .run();
}
