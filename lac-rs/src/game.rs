//! The run on screen: queued actions played back as animation (board row E2), the first-person camera (E5),
//! light and fog (E8), and the dungeon's entities kept in step with the sim.
//!
//! REF R27: the sim ticks only when an action is queued. REF R28: the more actions wait in the queue, the
//! faster each one plays, so the player can spam actions or watch them. OURS: the durations and the curve.
//! REF R14: the step and the 90° turn are tweened, as in Legend of Grimrock.

use crate::audio::{self, Sfx, Sounds};
use crate::models::{self, ModelCache};
use crate::palette::{self};
use crate::settings::{self, Settings};
use crate::voxel::{self, VoxelAssets};
use crate::{icons, Screen};
use bevy::camera::RenderTarget;
use bevy::pbr::{DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use lac_sim::sim::MoveKind;
use lac_sim::world::Feature;
use lac_sim::{Action, Dir, Event, IVec3 as SIVec3, Outcome, Recording, Sim};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;

/// The internal resolution. F8 set 480×270; ed-slopper asked for a roomier view, so it is 4:3 now, the full
/// height of the window.
pub const VIEW_W: u32 = 480;
pub const VIEW_H: u32 = 360;
/// OURS: the camera stands this far behind the centre of the party's tile, so the walls beside the party are in
/// view (the usual trick of tile-based crawlers, R14).
pub const BACK: f32 = 0.42 * TILE_W;
/// OURS: eye height above the floor of the party's tile.
/// OURS: the party stands tall (ed-slopper): three quarters of a tile.
pub const EYE: f32 = 0.75 * CELL_H;
/// Light, in Bevy's physical units (lumens, cd/m²) at its default camera exposure. OURS: the party's lantern is
/// bright near and gone by ten tiles; torches light a room; a little ambient keeps the far dark readable.
pub const PARTY_LIGHT: f32 = 100_000.0;
pub const TORCH_LIGHT: f32 = 140_000.0;
pub const AMBIENT: f32 = 1100.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Overlay {
    #[default]
    None,
    Inventory,
    Spells,
    Pause,
    Map,
    Help,
    /// Choosing what goes into a member's slot.
    Picker,
}

pub struct LogLine {
    pub text: String,
    pub age: f32,
}

#[derive(Default)]
pub struct Dirty {
    pub all_chunks: bool,
    pub chunks: BTreeSet<SIVec3>,
    pub features: bool,
    pub hud: bool,
}

/// The run in progress.
#[derive(Resource)]
pub struct Game {
    pub sim: Sim,
    pub record: Option<PathBuf>,
    pub replay: VecDeque<Action>,
    pub taken: Vec<Action>,
    pub log: Vec<LogLine>,
    pub dirty: Dirty,
    pub overlay: Overlay,
    pub selected: usize,
    pub cursor: usize,
    pub pause_cursor: usize,
    pub hurt_flash: f32,
    /// A melee swing on screen: time left, and whether it hit.
    pub slash: f32,
    pub slash_hit: bool,
    /// Resting: waiting until healed, stopped by danger.
    pub resting: bool,
    /// The slot the picker is filling.
    pub picker: Option<(usize, lac_sim::content::Slot)>,
    pub member_flash: [f32; 4],
    pub end_timer: Option<f32>,
    pub debug: bool,
    pub frame_ms: f32,
}

impl Game {
    pub fn new(sim: Sim) -> Self {
        let mut g = Game {
            sim,
            record: None,
            replay: VecDeque::new(),
            taken: Vec::new(),
            log: Vec::new(),
            dirty: Dirty { all_chunks: true, features: true, hud: true, ..default() },
            overlay: Overlay::None,
            selected: 0,
            cursor: 0,
            pause_cursor: 0,
            hurt_flash: 0.0,
            slash: 0.0,
            slash_hit: false,
            resting: false,
            picker: None,
            member_flash: [0.0; 4],
            end_timer: None,
            debug: false,
            frame_ms: 16.0,
        };
        let depth = g.sim.state.level.depth;
        let theme = g.sim.state.level.theme.name();
        g.say(format!("Depth {depth}: {theme}. Seed {}.", g.sim.state.seed_text));
        g.say("Find the stairs down. At the bottom waits the great chest.");
        g
    }
    /// An awake monster within ten tiles, seen or not (they may be coming).
    pub fn danger(&self) -> bool {
        let s = &self.sim.state;
        s.level.monsters.iter().any(|m| m.hp > 0 && m.awake && m.pos.flat_dist(s.pos) <= 10)
    }
    /// Everyone alive has full health and mana.
    pub fn rested(&self) -> bool {
        self.sim.state.party.iter().filter(|m| m.alive()).all(|m| m.hp >= m.max_hp && m.mp >= m.max_mp && m.poison == 0)
    }

    /// Who drinks, reads and wears: the selected member, or the first one alive if they have fallen.
    pub fn actor(&self) -> usize {
        let p = &self.sim.state.party;
        if p.get(self.selected).is_some_and(|m| m.alive()) {
            return self.selected;
        }
        p.iter().position(|m| m.alive()).unwrap_or(self.selected)
    }
    pub fn say(&mut self, s: impl Into<String>) {
        self.log.push(LogLine { text: s.into(), age: 0.0 });
        if self.log.len() > 60 {
            self.log.remove(0);
        }
        self.dirty.hud = true;
    }
    /// Queue an action, unless the run is over.
    pub fn push(&mut self, a: Action) {
        if self.sim.state.outcome == Outcome::Playing && self.sim.queued() < 32 {
            self.sim.push(a);
        }
    }
    pub fn write_recording(&self) {
        if let Some(p) = &self.record {
            let rec = Recording { seed: self.sim.state.seed_text.clone(), options: self.sim.state.options, actions: self.taken.clone(), expect: None };
            if let Ok(t) = ron::ser::to_string(&rec) {
                let _ = std::fs::write(p, t);
            }
        }
    }
}

/// The camera's path for the current tick, and every monster's.
#[derive(Resource, Default)]
pub struct Anim {
    pub active: bool,
    pub t: f32,
    pub dur: f32,
    /// Eye positions, with the height of the arc to the next (jumps).
    pub path: Vec<(Vec3, f32)>,
    pub yaw_from: f32,
    pub yaw_to: f32,
    pub lunge: f32,
    pub bob: bool,
}

#[derive(Component)]
pub struct PartyCam;

#[derive(Component)]
pub struct PartyLight;

#[derive(Component)]
pub struct WorldEntity;

#[derive(Component)]
pub struct ChunkView;

#[derive(Component)]
pub struct FeatureView;

#[derive(Component)]
pub struct TorchLight {
    pub phase: f32,
}

#[derive(Component)]
pub struct MonsterView {
    pub id: u32,
    pub from: Vec3,
    pub to: Vec3,
    pub yaw_from: f32,
    pub yaw_to: f32,
    pub dying: Option<f32>,
    pub flash: f32,
    pub lunge: f32,
    pub flies: bool,
    /// The depth it belongs to: views of an old level are dropped.
    pub depth: u8,
}

#[derive(Component)]
pub struct Bolt {
    pub from: Vec3,
    pub to: Vec3,
    pub fire: bool,
    pub arrow: bool,
}

// ------------------------------------------------------------------ particles (OURS)

/// Colours of glowing voxel particles.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Glow {
    Fire,
    Ember,
    Frost,
    White,
    Green,
    Violet,
    Gold,
    Blood,
    /// Plain grey dust: an arrow's flight and its splinters.
    Dust,
}

/// (colour, sRGB, glow strength; 0 is a plain lit cube)
const GLOWS: [(Glow, [f32; 3], f32); 9] = [
    (Glow::Fire, [1.0, 0.45, 0.12], 6.0),
    (Glow::Ember, [1.0, 0.85, 0.35], 6.0),
    (Glow::Frost, [0.55, 0.9, 1.0], 5.0),
    (Glow::White, [1.0, 1.0, 1.0], 5.0),
    (Glow::Green, [0.4, 1.0, 0.45], 4.0),
    (Glow::Violet, [0.75, 0.45, 1.0], 4.0),
    (Glow::Gold, [1.0, 0.85, 0.3], 5.0),
    (Glow::Blood, [0.55, 0.12, 0.10], 0.0),
    (Glow::Dust, [0.62, 0.56, 0.48], 0.0),
];

#[derive(Resource)]
pub struct ParticleAssets {
    pub cube: Handle<Mesh>,
    pub mats: Vec<Handle<StandardMaterial>>,
}

#[derive(Component)]
pub struct Particle {
    pub vel: Vec3,
    pub life: f32,
    pub max: f32,
    pub size: f32,
    pub gravity: f32,
}

/// Burst lights fade with their particles.
#[derive(Component)]
pub struct BurstLight {
    pub life: f32,
    pub max: f32,
    pub power: f32,
}

pub fn setup_particles(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let mats = GLOWS
        .iter()
        .map(|(_, c, e)| {
            let col = Color::srgb(c[0], c[1], c[2]);
            mats.add(StandardMaterial { base_color: col, emissive: LinearRgba::from(col) * (*e * 200.0), unlit: *e > 0.0, ..default() })
        })
        .collect();
    commands.insert_resource(ParticleAssets { cube, mats });
}

/// A small app-side noise for particle scatter (not the sim's: effects needn't replay).
fn jitter(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed as f32 / u32::MAX as f32) * 2.0 - 1.0
}

/// A cloud of glowing voxel cubes flying out from `at`, with a fading light if `light`.
pub fn burst(commands: &mut Commands, pa: &ParticleAssets, at: Vec3, glows: &[Glow], n: usize, speed: f32, light: bool) {
    let mut seed = ((at.x * 977.0 + at.z * 131.0 + at.y * 57.0).abs() as u32).wrapping_mul(2_654_435_761) | 1;
    for k in 0..n {
        let g = glows[k % glows.len()];
        let mi = GLOWS.iter().position(|(gg, _, _)| *gg == g).unwrap_or(0);
        let dir = Vec3::new(jitter(&mut seed), jitter(&mut seed) * 0.8 + 0.3, jitter(&mut seed)).normalize_or_zero();
        let life = 0.45 + 0.4 * jitter(&mut seed).abs();
        // fine voxel dust, a few tiles wide in the low-resolution view
        let size = 0.018 + 0.022 * jitter(&mut seed).abs();
        let vel = dir * speed * (0.4 + 0.6 * jitter(&mut seed).abs());
        commands.spawn((
            Mesh3d(pa.cube.clone()),
            MeshMaterial3d(pa.mats[mi].clone()),
            Transform::from_translation(at).with_scale(Vec3::splat(size)),
            Particle { vel, life, max: life, size, gravity: if matches!(g, Glow::Blood | Glow::Dust) { 4.0 } else { 0.6 } },
            WorldEntity,
        ));
    }
    if light {
        let c = GLOWS.iter().find(|(gg, _, _)| *gg == glows[0]).map(|(_, c, _)| *c).unwrap_or([1.0; 3]);
        commands.spawn((
            PointLight { color: Color::srgb(c[0], c[1], c[2]), intensity: 0.0, range: 5.0, ..default() },
            Transform::from_translation(at),
            BurstLight { life: 0.6, max: 0.6, power: 120_000.0 },
            WorldEntity,
        ));
    }
}

/// REF R27, R64: time stands still between actions, and so do particles: they hang in the air until the next
/// action plays (ed-slopper).
pub fn particles(
    mut commands: Commands,
    time: Res<Time>,
    anim: Res<Anim>,
    mut q: Query<(Entity, &mut Particle, &mut Transform)>,
    mut lights: Query<(Entity, &mut BurstLight, &mut PointLight)>,
) {
    let dt = if anim.active { time.delta_secs() } else { 0.0 };
    for (e, mut p, mut tr) in &mut q {
        p.life -= dt;
        if p.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        p.vel.y -= p.gravity * dt;
        let v = p.vel;
        p.vel *= 1.0 - 1.8 * dt;
        tr.translation += v * dt;
        tr.scale = Vec3::splat(p.size * (p.life / p.max).sqrt());
    }
    for (e, mut b, mut l) in &mut lights {
        b.life -= dt;
        if b.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        l.intensity = b.power * (b.life / b.max);
    }
}


#[derive(Resource, Default)]
pub struct ChunkMap(pub BTreeMap<SIVec3, Entity>);

/// How far open each door is (0 shut, 1 open), kept across rebuilds so doors swing instead of snapping.
#[derive(Resource, Default)]
pub struct DoorAngles(pub BTreeMap<(u8, SIVec3), f32>);

/// A door on screen. OURS: wooden doors swing 90° on a hinge at one edge; portcullises slide up.
#[derive(Component, Clone, Copy)]
pub struct DoorView {
    pub key: (u8, SIVec3),
    pub closed: Transform,
    pub portcullis: bool,
    pub open: bool,
    /// Tiles high: two where the doorway is open above, one under a lintel.
    pub high: f32,
}

impl DoorView {
    /// The door's transform when `frac` open.
    pub fn at(&self, frac: f32) -> Transform {
        let mut tr = self.closed;
        if self.portcullis {
            tr.translation.y += 0.92 * CELL_H * self.high * frac;
            return tr;
        }
        // the hinge sits at the door's left edge; the panel turns about it
        let hinge = Vec3::new(-0.47 * TILE_W, 0.0, 0.0);
        let turn = Quat::from_rotation_y(frac * std::f32::consts::FRAC_PI_2);
        let centre = hinge + turn * Vec3::new(0.47 * TILE_W, 0.0, 0.0);
        tr.translation = self.closed.translation + self.closed.rotation * centre;
        tr.rotation = self.closed.rotation * turn;
        tr
    }
}

/// Swing doors towards where the sim says they are.
pub fn animate_doors(time: Res<Time>, mut doors: ResMut<DoorAngles>, mut q: Query<(&DoorView, &mut Transform)>) {
    let step = time.delta_secs() * 3.0;
    for (v, mut tr) in &mut q {
        let f = doors.0.entry(v.key).or_insert(if v.open { 1.0 } else { 0.0 });
        let target = if v.open { 1.0 } else { 0.0 };
        if (*f - target).abs() > f32::EPSILON {
            *f = if *f < target { (*f + step).min(target) } else { (*f - step).max(target) };
            *tr = v.at(*f);
        }
    }
}

#[derive(Resource, Clone)]
pub struct ViewImage(pub Handle<Image>);

/// OURS: a tile is drawn this much wider than it is tall, so corridors and rooms feel roomy (the sim's grid is
/// unchanged).
pub const TILE_W: f32 = 1.5;
/// Scale from the sim's cells to the world.
/// OURS: and 1.25 times as tall as the sim's unit, so blocks and doorways aren't squat (ed-slopper: "squished").
pub const CELL_H: f32 = 1.25;
pub const CELL: Vec3 = Vec3::new(TILE_W, CELL_H, TILE_W);

/// The world position of the floor centre of a tile.
pub fn tile_pos(p: SIVec3) -> Vec3 {
    Vec3::new((p.x as f32 + 0.5) * TILE_W, p.y as f32 * CELL_H, (p.z as f32 + 0.5) * TILE_W)
}

/// A horizontal offset in tiles, in the world.
fn across(d: Dir, k: f32) -> Vec3 {
    Vec3::new(d.delta().x as f32, 0.0, d.delta().z as f32) * k * TILE_W
}

pub fn yaw(d: Dir) -> f32 {
    use std::f32::consts::{FRAC_PI_2, PI};
    match d {
        Dir::North => 0.0,
        Dir::West => FRAC_PI_2,
        Dir::South => PI,
        Dir::East => -FRAC_PI_2,
    }
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut d = (b - a) % TAU;
    if d > PI {
        d -= TAU;
    }
    if d < -PI {
        d += TAU;
    }
    a + d * t
}

/// REF R28: playback speeds up with the queue. OURS: each waiting action makes playback 80% faster.
pub fn speed(queued: usize) -> f32 {
    1.0 + queued as f32 * 0.8
}


// ------------------------------------------------------------------ setting up the 3D side

pub fn spawn_world(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut vmat: ResMut<Assets<voxel::VoxelMaterial>>,
    mut smat: ResMut<Assets<StandardMaterial>>,
    settings: Res<Settings>,
    game: Res<Game>,
) {
    let scale = settings.file.res_scale.clamp(1, 2) as u32;
    let mut target = Image::new_target_texture(VIEW_W * scale, VIEW_H * scale, TextureFormat::Bgra8UnormSrgb, None);
    target.sampler = bevy::image::ImageSampler::nearest();
    let view = images.add(target);
    commands.insert_resource(ViewImage(view.clone()));

    let va = voxel::make_material(&mut images, &mut vmat, settings.file.vision);
    commands.insert_resource(va);
    let mut cache = ModelCache {
        material: smat.add(StandardMaterial { base_color: Color::WHITE, perceptual_roughness: 0.9, reflectance: 0.05, ..default() }),
        ..default()
    };
    cache.meshes.clear();
    commands.insert_resource(cache);
    commands.insert_resource(ChunkMap::default());
    commands.insert_resource(DoorAngles::default());
    let s = &game.sim.state;
    let eye0 = tile_pos(s.pos) + Vec3::Y * EYE;
    commands.insert_resource(Anim { path: vec![(eye0, 0.0)], yaw_from: yaw(s.facing), yaw_to: yaw(s.facing), t: 1.0, dur: 1.0, ..default() });
    let fog = palette::fog(settings.file.vision, s.level.depth);
    let eye = tile_pos(s.pos) + Vec3::Y * EYE;
    let mut cam = commands.spawn((
        Camera3d::default(),
        Camera { order: -1, clear_color: ClearColorConfig::Custom(fog), ..default() },
        RenderTarget::Image(view.into()),
        Projection::Perspective(PerspectiveProjection { fov: 80f32.to_radians(), near: 0.05, ..default() }),
        Transform::from_translation(eye + Quat::from_rotation_y(yaw(s.facing)) * Vec3::new(0.0, 0.0, BACK)).with_rotation(Quat::from_rotation_y(yaw(s.facing))),
        Msaa::Off,
        // OURS: the brightness option shifts the exposure; 1.0 is Bevy's default (EV100 9.7)
        bevy::camera::Exposure { ev100: bevy::camera::Exposure::EV100_BLENDER - settings.file.brightness.clamp(0.25, 8.0).log2() },
        DistanceFog { color: fog, falloff: FogFalloff::Linear { start: 4.0, end: 16.0 }, ..default() },
        // OURS: a soft bloom, so flames and spells glow into the dark around them
        bevy::post_process::bloom::Bloom { intensity: 0.12, ..bevy::post_process::bloom::Bloom::NATURAL },
        PartyCam,
        WorldEntity,
    ));
    if settings.file.ssao {
        // REF R50, R65: screen-space ambient occlusion on the dungeon (Bevy GTAO: noise-dithered samples and an
        // edge-aware blur); shaders/palette.wesl remaps it and lays an ordered dither on top
        cam.insert(ScreenSpaceAmbientOcclusion::default());
    }
    cam.with_children(|c| {
        // OURS: the party carries a warm light (R50-R53 say nothing of light sources)
        c.spawn((
            PointLight { color: Color::srgb(0.92, 0.9, 0.8), intensity: PARTY_LIGHT, range: 14.0, ..default() },
            // a tile behind and a little above the eye: walls right in front don't burn white, while rooms
            // ahead stay lit (OURS)
            Transform::from_xyz(0.0, 0.25, 1.1),
            PartyLight,
        ));
    });
    commands.insert_resource(GlobalAmbientLight { color: Color::srgb(0.6, 0.62, 0.8), brightness: AMBIENT, ..default() });
}

pub fn despawn_world(mut commands: Commands, q: Query<Entity, Or<(With<WorldEntity>, With<ChunkView>, With<FeatureView>, With<MonsterView>, With<Bolt>)>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<Game>();
}

// ------------------------------------------------------------------ input

pub fn input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    settings: Res<Settings>,
    mut game: ResMut<Game>,
    mut next: ResMut<NextState<Screen>>,
    mut commands: Commands,
    sounds: Res<Sounds>,
) {
    use settings::Command as C;
    let pad = settings::gamepad_commands(&pads, settings.file.gamepad);
    let pressed = |c: C| settings.just(c, &keys) || pad.contains(&c);
    if game.end_timer.is_some() || !game.replay.is_empty() {
        if pressed(C::Menu) {
            game.replay.clear();
        }
        return;
    }
    for (k, i) in [(KeyCode::Digit1, 0), (KeyCode::Digit2, 1), (KeyCode::Digit3, 2), (KeyCode::Digit4, 3)] {
        if keys.just_pressed(k) && game.overlay != Overlay::Spells {
            game.selected = i;
            game.dirty.hud = true;
        }
    }
    if pressed(C::Debug) {
        game.debug = !game.debug;
    }
    let vol = settings.file.volume;
    // any key stops a rest
    if game.resting && (keys.get_just_pressed().next().is_some() || !pad.is_empty()) {
        game.resting = false;
        game.sim.clear_queue();
        game.say("You stop resting.");
        return;
    }
    match game.overlay {
        Overlay::None => {
            for c in C::ALL {
                if pressed(c) {
                    if let Some(a) = c.action() {
                        game.push(a);
                    }
                }
            }
            if pressed(C::Inventory) {
                game.overlay = Overlay::Inventory;
                game.cursor = 7;
                game.dirty.hud = true;
            } else if pressed(C::Spells) {
                game.overlay = Overlay::Spells;
                game.dirty.hud = true;
            } else if pressed(C::Map) {
                game.overlay = Overlay::Map;
                game.dirty.hud = true;
            } else if pressed(C::Rest) {
                if game.danger() {
                    game.say("You can't rest with enemies near.");
                } else if game.rested() {
                    game.say("Everyone is rested.");
                } else {
                    game.resting = true;
                    game.say("You rest...");
                }
            } else if pressed(C::Note) {
                // R60: marks on the map: each press cycles the mark here, the fifth takes it away
                let s = &game.sim.state;
                let cur = s.level.notes.get(&(s.pos.x, s.pos.z)).copied().unwrap_or(0);
                game.push(Action::Note { mark: (cur + 1) % 5 });
                game.dirty.hud = true;
            } else if pressed(C::Help) {
                game.overlay = Overlay::Help;
                game.dirty.hud = true;
            } else if pressed(C::Menu) {
                game.overlay = Overlay::Pause;
                game.pause_cursor = 0;
                game.dirty.hud = true;
            }
        }
        Overlay::Inventory => {
            let rows = 7 + game.sim.state.pack.len();
            let up = keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW);
            let down = keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS);
            if up {
                game.cursor = (game.cursor + rows - 1) % rows;
                game.dirty.hud = true;
            }
            if down {
                game.cursor = (game.cursor + 1) % rows;
                game.dirty.hud = true;
            }
            if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
                game.selected = (game.selected + 3) % 4;
                game.dirty.hud = true;
            }
            if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
                game.selected = (game.selected + 1) % 4;
                game.dirty.hud = true;
            }
            let member = game.actor() as u8;
            if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::KeyF) {
                if game.cursor < 7 {
                    let slot = lac_sim::content::Slot::ALL[game.cursor];
                    game.push(Action::Unequip { member, slot });
                } else {
                    let index = (game.cursor - 7) as u16;
                    game.push(Action::UseItem { member, index });
                }
            }
            if keys.just_pressed(KeyCode::KeyX) && game.cursor >= 7 {
                let index = (game.cursor - 7) as u16;
                game.push(Action::Drop { index });
            }
            if keys.just_pressed(KeyCode::KeyG) {
                game.push(Action::PickUp);
            }
            if keys.just_pressed(KeyCode::KeyR) {
                // swap the selected member with the next, to change the rows
                let b = ((game.selected + 1) % 4) as u8;
                game.push(Action::Swap { a: member, b });
            }
            if pressed(C::Inventory) || pressed(C::Menu) {
                game.overlay = Overlay::None;
                game.dirty.hud = true;
            }
        }
        Overlay::Spells => {
            let list = spell_list(&game.sim);
            for (k, key) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9].iter().enumerate() {
                if keys.just_pressed(*key) {
                    if let Some((m, sp)) = list.get(k) {
                        game.push(Action::Cast { member: *m as u8, spell: *sp });
                        game.overlay = Overlay::None;
                        game.dirty.hud = true;
                    }
                }
            }
            if pressed(C::Spells) || pressed(C::Menu) {
                game.overlay = Overlay::None;
                game.dirty.hud = true;
            }
        }
        Overlay::Picker => {
            if pressed(C::Menu) || pressed(C::Inventory) {
                game.overlay = Overlay::None;
                game.picker = None;
                game.dirty.hud = true;
            }
        }
        Overlay::Help => {
            if pressed(C::Help) || pressed(C::Menu) {
                game.overlay = Overlay::None;
                game.dirty.hud = true;
            }
        }
        Overlay::Map => {
            if pressed(C::Map) || pressed(C::Menu) {
                game.overlay = Overlay::None;
                game.dirty.hud = true;
            }
        }
        Overlay::Pause => {
            let n = 3;
            if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
                game.pause_cursor = (game.pause_cursor + n - 1) % n;
                game.dirty.hud = true;
            }
            if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
                game.pause_cursor = (game.pause_cursor + 1) % n;
                game.dirty.hud = true;
            }
            if pressed(C::Menu) {
                game.overlay = Overlay::None;
                game.dirty.hud = true;
            }
            if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
                match game.pause_cursor {
                    0 => {
                        game.overlay = Overlay::None;
                        game.dirty.hud = true;
                    }
                    1 => {
                        // X3: one save per run
                        save_run(&game);
                        game.write_recording();
                        audio::play(&mut commands, &sounds, Sfx::Door, vol);
                        next.set(Screen::Title);
                    }
                    _ => {
                        let _ = std::fs::remove_file(settings::save_path());
                        game.write_recording();
                        next.set(Screen::Title);
                    }
                }
            }
        }
    }
}

/// Every spell the living casters can cast now: (member, spell).
pub fn spell_list(sim: &Sim) -> Vec<(usize, lac_sim::party::Spell)> {
    let mut out = Vec::new();
    for (i, m) in sim.state.party.iter().enumerate() {
        if !m.alive() {
            continue;
        }
        for sp in lac_sim::party::Spell::of(m.class) {
            if m.level >= sp.min_level() {
                out.push((i, sp));
            }
        }
    }
    out
}

pub fn save_run(game: &Game) {
    let p = settings::save_path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if game.sim.state.outcome == Outcome::Playing {
        let _ = std::fs::write(p, game.sim.save());
    }
}

// ------------------------------------------------------------------ playback

#[allow(clippy::too_many_arguments)]
pub fn playback(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut anim: ResMut<Anim>,
    cam: Single<&Transform, With<PartyCam>>,
    mut monsters: Query<(Entity, &mut MonsterView)>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    time: Res<Time>,
    mut next: ResMut<NextState<Screen>>,
    mut summary: ResMut<crate::menu::EndSummary>,
    particles: Res<ParticleAssets>,
) {
    let vol = settings.file.volume;
    if anim.active {
        return;
    }
    if game.sim.queued() == 0 {
        if let Some(a) = game.replay.pop_front() {
            game.sim.push(a);
        }
    }
    // OURS: resting is waiting, fast, until healed or until danger shows
    if game.resting {
        if game.danger() {
            game.resting = false;
            game.sim.clear_queue();
            game.say("Something stirs. You stop resting.");
        } else if game.rested() || game.sim.state.outcome != Outcome::Playing {
            game.resting = false;
            game.say("Rested.");
        } else if game.sim.queued() == 0 {
            for _ in 0..4 {
                game.sim.push(Action::Wait);
            }
        }
    }
    // the end of the run, once the last animation has played
    if game.sim.state.outcome != Outcome::Playing && game.sim.queued() == 0 {
        let t = game.end_timer.get_or_insert(2.2);
        *t -= time.delta_secs();
        if *t <= 0.0 {
            *summary = crate::menu::EndSummary::from(&game.sim);
            let _ = std::fs::remove_file(settings::save_path());
            game.write_recording();
            next.set(Screen::End);
        }
        return;
    }
    let queued = game.sim.queued();
    let Some(&action) = game.sim.peek() else { return };
    let Some(events) = game.sim.step() else { return };
    game.taken.push(action);
    if game.taken.len() % 50 == 0 {
        game.write_recording();
    }
    let factor = speed(queued.saturating_sub(1));
    // the path runs through tile centres; the camera sits BACK behind them
    let start = cam.translation - cam.rotation * Vec3::new(0.0, 0.0, BACK);
    let start_yaw = anim.yaw_to;
    let mut path = vec![(start, 0.0)];
    let mut yaw_to = start_yaw;
    let mut base = 0.0f32;
    let mut lunge = 0.0;
    let mut snap = false;
    let mut bob = false;
    let content = game.sim.content.clone();
    let facing = Vec3::new(0.0, 0.0, -1.0);
    let fwd = Quat::from_rotation_y(start_yaw) * facing;
    for e in &events {
        match e {
            Event::PartyMoved { to, kind, from } => {
                let eye = tile_pos(*to) + Vec3::Y * EYE;
                match kind {
                    MoveKind::Step => {
                        path.push((eye, 0.0));
                        base = base.max(0.22);
                        bob = true;
                        audio::play(&mut commands, &sounds, Sfx::Step, vol * 0.6);
                    }
                    MoveKind::Jump => {
                        path.push((eye, 0.0));
                        base = base.max(0.24);
                        audio::play(&mut commands, &sounds, Sfx::Jump, vol);
                    }
                    MoveKind::LongJump => {
                        path.push((eye, 0.45));
                        base = base.max(0.34);
                        audio::play(&mut commands, &sounds, Sfx::Jump, vol);
                    }
                    MoveKind::Fall => {
                        path.push((eye, 0.0));
                        base += 0.08 * (from.y - to.y).max(1) as f32;
                        let water = game.sim.state.level.world.get(*to) == lac_sim::Cell::Water;
                        audio::play(&mut commands, &sounds, if water { Sfx::Splash } else { Sfx::Land }, vol);
                    }
                    MoveKind::Teleport => {
                        // every teleport goes in the log, with where from and to (the message line says why)
                        info!("teleport: {:?} -> {:?} (depth {})", from, to, game.sim.state.level.depth);
                        snap = true;
                        audio::play(&mut commands, &sounds, Sfx::Spell, vol);
                    }
                }
            }
            Event::PartyTurned { to, .. } => {
                yaw_to = yaw(*to);
                base = base.max(0.17);
            }
            Event::Bump => {
                path.push((start + fwd * 0.12, 0.0));
                path.push((start, 0.0));
                base = base.max(0.14);
                audio::play(&mut commands, &sounds, Sfx::Bump, vol);
            }
            Event::MonsterMoved { id, from, to } => {
                for (_, mut mv) in monsters.iter_mut() {
                    if mv.id == *id {
                        mv.from = monster_pos(*from, mv.flies);
                        mv.to = monster_pos(*to, mv.flies);
                        if let Some(d) = Dir::between(*from, *to) {
                            mv.yaw_from = mv.yaw_to;
                            mv.yaw_to = yaw(d);
                        }
                    }
                }
                base = base.max(0.2);
            }
            Event::MonsterHurt { id, .. } => {
                for (_, mut mv) in monsters.iter_mut() {
                    if mv.id == *id {
                        mv.flash = 1.0;
                        // OURS: a puff of dark debris where the blow lands
                        burst(&mut commands, &particles, mv.to + Vec3::Y * 0.7, &[Glow::Blood], 10, 1.3, false);
                    }
                }
            }
            Event::MonsterDied { id, .. } => {
                for (_, mut mv) in monsters.iter_mut() {
                    if mv.id == *id {
                        mv.dying = Some(0.0);
                    }
                }
                audio::play(&mut commands, &sounds, Sfx::Death, vol);
            }
            Event::MonsterSpawned { .. } => game.dirty.features = true,
            Event::MonsterAttack { id, hit, ranged, .. } => {
                for (_, mut mv) in monsters.iter_mut() {
                    if mv.id == *id && !ranged {
                        mv.lunge = 1.0;
                    }
                }
                if !hit {
                    audio::play(&mut commands, &sounds, Sfx::Miss, vol * 0.7);
                }
                base = base.max(0.22);
            }
            Event::PartyAttack { hit, target, ranged, .. } => {
                if !ranged {
                    lunge = 1.0;
                    game.slash = 0.2;
                    game.slash_hit = *hit;
                }
                base = base.max(0.22);
                if target.is_some() {
                    audio::play(&mut commands, &sounds, if *hit { Sfx::Hit } else { Sfx::Miss }, vol);
                } else {
                    audio::play(&mut commands, &sounds, Sfx::Miss, vol);
                }
            }
            Event::CharacterHurt { member, .. } => {
                if game.resting {
                    game.resting = false;
                    game.sim.clear_queue();
                    game.say("Hurt! You stop resting.");
                }
                game.hurt_flash = 1.0;
                if let Some(f) = game.member_flash.get_mut(*member as usize) {
                    *f = 1.0;
                }
                audio::play(&mut commands, &sounds, Sfx::Hurt, vol);
            }
            Event::CharacterHealed { .. } => {}
            Event::CharacterDied { .. } => audio::play(&mut commands, &sounds, Sfx::Death, vol),
            Event::Spell { spell, .. } => {
                base = base.max(0.25);
                audio::play(&mut commands, &sounds, Sfx::Spell, vol);
                use lac_sim::party::Spell as Sp;
                let s = &game.sim.state;
                let at = tile_pos(s.pos) + across(s.facing, 0.35) + Vec3::Y * (EYE - 0.15);
                let glows: &[Glow] = match spell {
                    Sp::Spark => &[Glow::White, Glow::Frost],
                    Sp::Fireball => &[Glow::Fire, Glow::Ember],
                    Sp::Quicken => &[Glow::Violet, Glow::White],
                    Sp::Mend | Sp::Raise => &[Glow::Green, Glow::White],
                    Sp::Purify => &[Glow::White, Glow::Frost],
                    Sp::Rebuke => &[Glow::Gold, Glow::White],
                };
                burst(&mut commands, &particles, at, glows, 48, 1.6, true);
            }
            Event::Bolt { from, to, fire, arrow } => {
                let a = tile_pos(*from) + Vec3::Y * EYE * 0.85;
                let b = tile_pos(*to) + Vec3::Y * EYE * 0.85;
                if *arrow {
                    // an arrow: a thin shaft flying along the line, no glow
                    let dir = (b - a).normalize_or_zero();
                    commands.spawn((
                        Bolt { from: a, to: b, fire: false, arrow: true },
                        Mesh3d(particles.cube.clone()),
                        MeshMaterial3d(particles.mats[GLOWS.iter().position(|(g, _, _)| *g == Glow::Dust).unwrap_or(0)].clone()),
                        Transform::from_translation(a).looking_to(if dir == Vec3::ZERO { Vec3::Z } else { dir }, Vec3::Y).with_scale(Vec3::new(0.025, 0.025, 0.45)),
                        WorldEntity,
                    ));
                    audio::play(&mut commands, &sounds, Sfx::Miss, vol * 0.8);
                } else {
                    commands.spawn((
                        Bolt { from: a, to: b, fire: *fire, arrow: false },
                        Transform::from_translation(a),
                        Visibility::default(),
                        PointLight { color: if *fire { Color::srgb(1.0, 0.5, 0.2) } else { Color::srgb(0.5, 0.7, 1.0) }, intensity: TORCH_LIGHT, range: 4.0, ..default() },
                    ));
                }
                base = base.max(0.25);
                if *fire {
                    audio::play(&mut commands, &sounds, Sfx::Fire, vol);
                }
            }
            Event::CellsChanged(cells) => {
                for c in cells {
                    for d in [SIVec3::ZERO, SIVec3::new(1, 0, 0), SIVec3::new(-1, 0, 0), SIVec3::new(0, 1, 0), SIVec3::new(0, -1, 0), SIVec3::new(0, 0, 1), SIVec3::new(0, 0, -1)] {
                        game.dirty.chunks.insert(lac_sim::world::World::chunk_of(*c + d));
                    }
                }
            }
            Event::FeatureChanged(p) => {
                game.dirty.features = true;
                match game.sim.state.level.features.get(p) {
                    Some(Feature::Lever { .. }) => audio::play(&mut commands, &sounds, Sfx::Lever, vol),
                    Some(Feature::Door { .. }) => audio::play(&mut commands, &sounds, Sfx::Door, vol),
                    Some(Feature::Chest { .. }) => audio::play(&mut commands, &sounds, Sfx::Chest, vol),
                    _ => {}
                }
            }
            Event::ItemsChanged(_) => game.dirty.features = true,
            Event::PackChanged => {
                game.dirty.hud = true;
                let rows = 7 + game.sim.state.pack.len();
                if game.cursor >= rows {
                    game.cursor = rows - 1;
                }
            }
            Event::Message(m) => {
                if m.contains("reaches level") {
                    audio::play(&mut commands, &sounds, Sfx::LevelUp, vol);
                }
                if m.starts_with("Got ") || m.ends_with(" gold.") {
                    audio::play(&mut commands, &sounds, Sfx::Pickup, vol * 0.8);
                }
                let m = m.clone();
                game.say(m);
            }
            Event::LevelChanged { .. } => {
                snap = true;
                game.dirty.all_chunks = true;
                game.dirty.features = true;
                audio::play(&mut commands, &sounds, Sfx::Stairs, vol);
                save_run(&game);
                game.write_recording();
            }
            Event::Won => audio::play(&mut commands, &sounds, Sfx::Win, vol),
            Event::Lost => audio::play(&mut commands, &sounds, Sfx::Lose, vol),
        }
    }
    let _ = content;
    game.dirty.hud = true;
    let s = &game.sim.state;
    let end_eye = tile_pos(s.pos) + Vec3::Y * EYE;
    if snap {
        path = vec![(end_eye, 0.0)];
        yaw_to = yaw(s.facing);
        anim.yaw_from = yaw_to;
    } else {
        anim.yaw_from = start_yaw;
        // make sure the path ends where the sim says the party is
        if path.last().map(|p| p.0.distance(end_eye) > 0.001).unwrap_or(true) && path.len() > 1 {
            path.push((end_eye, 0.0));
        }
    }
    if base == 0.0 {
        base = 0.08;
    }
    anim.path = path;
    anim.yaw_to = yaw_to;
    anim.t = 0.0;
    anim.dur = (base / factor).max(0.035);
    anim.lunge = lunge;
    anim.bob = bob;
    anim.active = true;
    // monsters that didn't move hold still
    for (_, mut mv) in monsters.iter_mut() {
        if !events.iter().any(|e| matches!(e, Event::MonsterMoved { id, .. } if *id == mv.id)) {
            mv.from = mv.to;
            mv.yaw_from = mv.yaw_to;
        }
    }
}

fn monster_pos(p: SIVec3, flies: bool) -> Vec3 {
    tile_pos(p) + if flies { Vec3::Y * 0.22 } else { Vec3::ZERO }
}

/// Advance the camera, the monsters and the bolts along this tick's animation.
pub fn animate(
    mut commands: Commands,
    time: Res<Time>,
    mut anim: ResMut<Anim>,
    mut cam: Single<&mut Transform, (With<PartyCam>, Without<MonsterView>, Without<Bolt>)>,
    mut monsters: Query<(Entity, &mut MonsterView, &mut Transform), (Without<PartyCam>, Without<Bolt>)>,
    mut bolts: Query<(Entity, &Bolt, &mut Transform), (Without<PartyCam>, Without<MonsterView>)>,
    mut game: ResMut<Game>,
    pa: Res<ParticleAssets>,
) {
    let dt = time.delta_secs();
    game.frame_ms = game.frame_ms * 0.9 + dt * 1000.0 * 0.1;
    game.hurt_flash = (game.hurt_flash - dt * 2.5).max(0.0);
    game.slash = (game.slash - dt).max(0.0);
    for f in &mut game.member_flash {
        *f = (*f - dt * 2.0).max(0.0);
    }
    for l in &mut game.log {
        l.age += dt;
    }
    if anim.active {
        anim.t += dt / anim.dur.max(0.001);
        if anim.t >= 1.0 {
            anim.t = 1.0;
        }
    }
    let t = anim.t;
    let ease = t * t * (3.0 - 2.0 * t);
    // camera along the path
    if !anim.path.is_empty() {
        let segs = anim.path.len().saturating_sub(1).max(1);
        let f = ease * segs as f32;
        let i = (f.floor() as usize).min(segs - 1);
        let local = f - i as f32;
        let (a, arc) = anim.path[i.min(anim.path.len() - 1)];
        let b = anim.path.get(i + 1).map(|p| p.0).unwrap_or(a);
        let mut p = a.lerp(b, local);
        p.y += arc * 4.0 * local * (1.0 - local);
        if anim.bob {
            // OURS: a small head bob on steps
            p.y += (local * std::f32::consts::PI).sin() * 0.035;
        }
        let yaw_now = lerp_angle(anim.yaw_from, anim.yaw_to, ease);
        let rot = Quat::from_rotation_y(yaw_now);
        let lunge = anim.lunge * (t * std::f32::consts::PI).sin() * 0.12;
        cam.translation = p + rot * Vec3::new(0.0, 0.0, BACK - lunge);
        cam.rotation = rot;
    }
    let time_s = time.elapsed_secs();
    for (e, mut mv, mut tr) in monsters.iter_mut() {
        let mut p = mv.from.lerp(mv.to, ease);
        if mv.flies {
            p.y += (time_s * 2.0 + mv.id as f32).sin() * 0.06;
        }
        let rot = Quat::from_rotation_y(lerp_angle(mv.yaw_from, mv.yaw_to, ease));
        let lunge = mv.lunge * (t * std::f32::consts::PI).sin() * 0.25;
        tr.translation = p + rot * Vec3::new(0.0, 0.0, -lunge);
        tr.rotation = rot;
        mv.flash = (mv.flash - dt * 4.0).max(0.0);
        let shake = (time_s * 60.0).sin() * 0.04 * mv.flash;
        tr.translation.x += shake;
        tr.scale = Vec3::splat(1.0 + 0.08 * mv.flash);
        if t >= 1.0 {
            mv.lunge = 0.0;
        }
        if let Some(d) = mv.dying.as_mut() {
            *d += dt * 2.5;
            let k = (1.0 - *d).max(0.0);
            tr.scale = Vec3::new(1.0 + (1.0 - k) * 0.3, k, 1.0 + (1.0 - k) * 0.3);
            if *d >= 1.0 {
                commands.entity(e).despawn();
            }
        }
    }
    for (e, b, mut tr) in bolts.iter_mut() {
        tr.translation = b.from.lerp(b.to, t);
        if b.arrow {
            // an arrow leaves no sparks; it splinters where it lands
            if t >= 1.0 {
                burst(&mut commands, &pa, b.to, &[Glow::Dust], 8, 1.0, false);
                commands.entity(e).despawn();
            }
            continue;
        }
        let trail: &[Glow] = if b.fire { &[Glow::Fire, Glow::Ember] } else { &[Glow::White, Glow::Frost] };
        // a light trail of sparks behind the bolt, then a burst where it lands
        if anim.active {
            burst(&mut commands, &pa, tr.translation, trail, 2, 0.3, false);
        }
        if t >= 1.0 {
            burst(&mut commands, &pa, b.to, trail, 60, 2.2, true);
            commands.entity(e).despawn();
        }
    }
    if anim.active && anim.t >= 1.0 {
        anim.active = false;
    }
}

// ------------------------------------------------------------------ keeping the world in step

pub fn rebuild_chunks(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut map: ResMut<ChunkMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    va: Res<VoxelAssets>,
    mut cam: Single<(&mut Camera, &mut DistanceFog), With<PartyCam>>,
    settings: Res<Settings>,
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    let all = std::mem::take(&mut game.dirty.all_chunks);
    let some: BTreeSet<SIVec3> = std::mem::take(&mut game.dirty.chunks);
    if !all && some.is_empty() {
        return;
    }
    let world = &game.sim.state.level.world;
    let keys: Vec<SIVec3> = if all {
        for (_, e) in std::mem::take(&mut map.0) {
            commands.entity(e).despawn();
        }
        let fog = palette::fog(settings.file.vision, game.sim.state.level.depth);
        cam.0.clear_color = ClearColorConfig::Custom(fog);
        cam.1.color = fog;
        // OURS: each theme's ambient light has its own tint
        ambient.color = match game.sim.state.level.theme {
            lac_sim::Theme::Crypt => Color::srgb(0.64, 0.72, 0.7),
            lac_sim::Theme::Caves => Color::srgb(0.58, 0.74, 0.62),
            lac_sim::Theme::Tombs => Color::srgb(0.62, 0.62, 0.86),
            lac_sim::Theme::Deep => Color::srgb(0.76, 0.52, 0.8),
        };
        world.chunk_keys().collect()
    } else {
        some.into_iter().filter(|k| world.chunk_keys().any(|c| c == *k)).collect()
    };
    for k in keys {
        if let Some(e) = map.0.remove(&k) {
            commands.entity(e).despawn();
        }
        if let Some(mesh) = voxel::chunk_mesh(world, k) {
            let e = commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(va.material.clone()), Transform::from_scale(CELL), ChunkView)).id();
            map.0.insert(k, e);
        }
    }
}

/// Features, floor items and monsters: rebuilt whenever something about them changes.
#[allow(clippy::too_many_arguments)]
pub fn rebuild_features(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    views: Query<Entity, With<FeatureView>>,
    monsters: Query<(Entity, &MonsterView)>,
    settings: Res<Settings>,
    mut doors: ResMut<DoorAngles>,
) {
    if !std::mem::take(&mut game.dirty.features) {
        return;
    }
    let vision = settings.file.vision;
    for e in &views {
        commands.entity(e).despawn();
    }
    let s = &game.sim.state;
    let c = &game.sim.content;
    let lvl = &s.level;
    let mat = cache.material.clone();
    for (p, f) in &lvl.features {
        let base = tile_pos(*p);
        let mut door_view: Option<DoorView> = None;
        let mut arch: Option<Transform> = None;
        let (key, model, tr): (String, fn() -> (models::VoxModel, f32), Transform) = match f {
            Feature::Chest { open, facing, .. } => (
                format!("chest{open}"),
                if *open { || (models::chest_model(true, false), 1.0) } else { || (models::chest_model(false, false), 1.0) },
                Transform::from_translation(base).with_rotation(Quat::from_rotation_y(yaw(*facing))),
            ),
            Feature::Lever { on, face, .. } => {
                let wall = tile_pos(*p) + across(*face, 0.5) + Vec3::Y * 0.3 * CELL_H;
                (
                    format!("lever{on}"),
                    if *on { || (models::lever_model(true), 1.0) } else { || (models::lever_model(false), 1.0) },
                    Transform::from_translation(wall).with_rotation(Quat::from_rotation_y(yaw(*face))),
                )
            }
            Feature::Torch { face } => {
                let wall = tile_pos(*p) + across(*face, 0.5) + Vec3::Y * 0.45 * CELL_H;
                let tr = Transform::from_translation(wall).with_rotation(Quat::from_rotation_y(yaw(*face)));
                // the flame's light
                commands.spawn((
                    // close to the wall, so it throws a bright halo round the torch and a soft glow beyond
                    PointLight { color: Color::srgb(1.0, 0.66, 0.36), intensity: TORCH_LIGHT, range: 6.0, ..default() },
                    Transform::from_translation(wall + across(*face, 0.3) + Vec3::Y * 0.45 * CELL_H),
                    TorchLight { phase: (p.x * 7 + p.z * 13) as f32 },
                    FeatureView,
                ));
                ("torch".into(), || (models::torch_model(), 1.0), tr)
            }
            Feature::Door { open, portcullis, .. } => {
                let walls_x = lvl.world.solid(*p + SIVec3::new(1, 0, 0)) && lvl.world.solid(*p + SIVec3::new(-1, 0, 0));
                let closed = Transform::from_translation(base)
                    .with_rotation(Quat::from_rotation_y(if walls_x { 0.0 } else { std::f32::consts::FRAC_PI_2 }))
                    .with_scale(Vec3::new(TILE_W, CELL_H, 1.0));
                let key = (lvl.depth, *p);
                let frac = *doors.0.entry(key).or_insert(if *open { 1.0 } else { 0.0 });
                let tall = !lvl.world.solid(p.up());
                let high = if tall { 2.0 } else { 1.0 };
                let view = DoorView { key, closed, portcullis: *portcullis, open: *open, high };
                let tr = view.at(frac);
                door_view = Some(view);
                if tall {
                    // the stone arch over a two-high doorway
                    arch = Some(Transform::from_translation(base + Vec3::Y * CELL_H).with_rotation(closed.rotation).with_scale(CELL));
                }
                (
                    format!("door{portcullis}{tall}"),
                    match (*portcullis, tall) {
                        (true, true) => || (models::door_model(true, true), 1.0),
                        (true, false) => || (models::door_model(true, false), 1.0),
                        (false, true) => || (models::door_model(false, true), 1.0),
                        (false, false) => || (models::door_model(false, false), 1.0),
                    },
                    tr,
                )
            }
            Feature::Stairs { facing } => (
                "stairs".into(),
                || (models::stairs_model(), 1.0),
                Transform::from_translation(base + Vec3::Y * 0.002).with_rotation(Quat::from_rotation_y(yaw(*facing))).with_scale(CELL),
            ),
            Feature::Plate { pressed, .. } => (
                format!("plate{pressed}"),
                if *pressed { || (models::plate_model(true), 1.0) } else { || (models::plate_model(false), 1.0) },
                Transform::from_translation(base + Vec3::Y * 0.001).with_scale(CELL),
            ),
            Feature::Spikes => ("spikes".into(), || (models::spikes_model(), 1.0), Transform::from_translation(base).with_scale(CELL)),
            Feature::Fountain { .. } => ("fountain".into(), || (models::fountain_model(), 1.0), Transform::from_translation(base)),
            Feature::Tomb => ("tomb".into(), || (models::tomb_model(), 1.0), Transform::from_translation(base).with_rotation(Quat::from_rotation_y(((p.x + p.z) % 2) as f32 * 0.3 - 0.15))),
            Feature::SecretWall => continue,
        };
        let mesh = cache.get(&key, &mut meshes, vision, model);
        let mut e = commands.spawn((Mesh3d(mesh), MeshMaterial3d(mat.clone()), tr, FeatureView));
        if let Some(v) = door_view.take() {
            e.insert(v);
        }
        if let Some(at) = arch {
            let mesh = cache.get("arch", &mut meshes, vision, || (models::arch_model(), 1.0));
            commands.spawn((Mesh3d(mesh), MeshMaterial3d(mat.clone()), at, FeatureView));
        }
    }
    for (p, items) in &lvl.items {
        for (k, it) in items.iter().enumerate().take(6) {
            let def = c.item(&it.def);
            let liquid = s.knowledge.looks.get(&it.def).map(|l| icons::look_colour(l)).unwrap_or((palette::ix(palette::RED, 4), palette::ix(palette::RED, 6)));
            let key = format!("item:{}:{}", def.icon, liquid.0);
            let icon_name = def.icon.clone();
            let ic = icons::icon(&icon_name, liquid);
            let mesh = if let Some(h) = cache.meshes.get(&key) {
                h.clone()
            } else {
                let h = meshes.add(models::item_model(&ic).mesh(vision, 0.7));
                cache.meshes.insert(key, h.clone());
                h
            };
            let off = Vec3::new(((k % 3) as f32 - 1.0) * 0.22, 0.003 + k as f32 * 0.004, ((k / 3) as f32 - 0.5) * 0.25);
            let spin = ((p.x * 31 + p.z * 17 + k as i32 * 7) % 12) as f32 * 0.5;
            commands.spawn((Mesh3d(mesh), MeshMaterial3d(mat.clone()), Transform::from_translation(tile_pos(*p) + off).with_rotation(Quat::from_rotation_y(spin)), FeatureView));
        }
    }
    // monsters: add the new, drop the gone (the dying finish their animation on their own)
    let live: BTreeMap<u32, &lac_sim::monster::Monster> = lvl.monsters.iter().filter(|m| m.hp > 0).map(|m| (m.id, m)).collect();
    let mut have = BTreeSet::new();
    for (e, mv) in &monsters {
        if mv.depth != lvl.depth || (mv.dying.is_none() && !live.contains_key(&mv.id)) {
            commands.entity(e).despawn();
            continue;
        }
        have.insert(mv.id);
    }
    for (id, m) in live {
        if have.contains(&id) {
            continue;
        }
        let def = c.monster(&m.kind);
        let model_name = def.model.clone();
        let mesh = cache.get(&format!("mon:{model_name}"), &mut meshes, vision, || models::monster_model(&model_name));
        let pos = monster_pos(m.pos, def.flies);
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw(m.facing))),
            MonsterView { id, from: pos, to: pos, yaw_from: yaw(m.facing), yaw_to: yaw(m.facing), dying: None, flash: 0.0, lunge: 0.0, flies: def.flies, depth: lvl.depth },
        ));
    }
}

/// Brightness keys in play: `[` darker, `]` brighter (saved with the settings).
pub fn brightness_keys(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<Settings>, mut game: ResMut<Game>) {
    let up = keys.just_pressed(KeyCode::BracketRight);
    let down = keys.just_pressed(KeyCode::BracketLeft);
    if !up && !down {
        return;
    }
    let b = &mut settings.file.brightness;
    *b = if up { (*b * 1.25).min(8.0) } else { (*b / 1.25).max(0.25) };
    let pct = (*b * 100.0).round();
    settings.save();
    game.say(format!("Brightness {pct}%"));
}

pub fn apply_exposure(settings: Res<Settings>, mut cam: Query<&mut bevy::camera::Exposure, With<PartyCam>>) {
    if !settings.is_changed() {
        return;
    }
    for mut e in &mut cam {
        e.ev100 = bevy::camera::Exposure::EV100_BLENDER - settings.file.brightness.clamp(0.25, 8.0).log2();
    }
}

/// A health bar over a hurt monster (R61), turned to the camera. OURS: shown once a monster is hurt.
#[derive(Component)]
pub struct HpBar {
    pub id: u32,
    pub fill: bool,
}

#[derive(Resource)]
pub struct BarAssets {
    pub mesh: Handle<Mesh>,
    pub back: Handle<StandardMaterial>,
    pub front: Handle<StandardMaterial>,
}

pub fn setup_bars(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(BarAssets {
        mesh: meshes.add(Rectangle::new(1.0, 1.0)),
        back: mats.add(StandardMaterial { base_color: Color::srgb(0.08, 0.06, 0.06), unlit: true, ..default() }),
        front: mats.add(StandardMaterial { base_color: Color::srgb(0.35, 0.85, 0.25), unlit: true, ..default() }),
    });
}

pub fn monster_bars(
    mut commands: Commands,
    game: Res<Game>,
    assets: Res<BarAssets>,
    cam: Single<&Transform, (With<PartyCam>, Without<HpBar>, Without<MonsterView>)>,
    views: Query<(&MonsterView, &Transform), Without<HpBar>>,
    mut bars: Query<(Entity, &HpBar, &mut Transform, &mut Visibility), Without<MonsterView>>,
) {
    let lvl = &game.sim.state.level;
    let hp = |id: u32| lvl.monsters.iter().find(|m| m.id == id).map(|m| (m.hp, m.max_hp));
    let mut have = std::collections::BTreeSet::new();
    for (e, bar, mut tr, mut vis) in &mut bars {
        let view = views.iter().find(|(v, _)| v.id == bar.id && v.depth == lvl.depth);
        let (Some((v, vt)), Some((h, max))) = (view, hp(bar.id)) else {
            commands.entity(e).despawn();
            continue;
        };
        have.insert((bar.id, bar.fill));
        if h >= max || h <= 0 || v.dying.is_some() {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;
        let def = game.sim.content.monster(&lvl.monsters.iter().find(|m| m.id == bar.id).unwrap().kind);
        let top = if def.special && def.model == "warden" { 2.3 } else { 1.45 };
        let frac = h as f32 / max as f32;
        let w = 0.5;
        let toward = (cam.translation - vt.translation).with_y(0.0).normalize_or_zero();
        let rot = Quat::from_rotation_arc(Vec3::Z, if toward == Vec3::ZERO { Vec3::Z } else { toward });
        let right = rot * Vec3::X;
        let base = vt.translation + Vec3::Y * top;
        if bar.fill {
            tr.translation = base - right * (w * (1.0 - frac) / 2.0) + toward * 0.01;
            tr.scale = Vec3::new(w * frac, 0.045, 1.0);
        } else {
            tr.translation = base;
            tr.scale = Vec3::new(w + 0.03, 0.07, 1.0);
        }
        tr.rotation = rot;
    }
    for m in &lvl.monsters {
        if m.hp <= 0 || m.hp >= m.max_hp {
            continue;
        }
        for fill in [false, true] {
            if !have.contains(&(m.id, fill)) {
                commands.spawn((
                    Mesh3d(assets.mesh.clone()),
                    MeshMaterial3d(if fill { assets.front.clone() } else { assets.back.clone() }),
                    Transform::from_scale(Vec3::ZERO),
                    Visibility::Hidden,
                    HpBar { id: m.id, fill },
                    WorldEntity,
                ));
            }
        }
    }
}

pub fn flicker(time: Res<Time>, mut torches: Query<(&TorchLight, &mut PointLight)>, mut party: Query<&mut PointLight, (With<PartyLight>, Without<TorchLight>)>) {
    let t = time.elapsed_secs();
    for (tl, mut l) in &mut torches {
        let f = (t * 9.0 + tl.phase).sin() * 0.5 + (t * 23.0 + tl.phase * 1.7).sin() * 0.3;
        l.intensity = TORCH_LIGHT * (1.0 + f * 0.18);
    }
    for mut l in &mut party {
        let f = (t * 7.0).sin() * 0.5 + (t * 17.3).sin() * 0.5;
        l.intensity = PARTY_LIGHT * (1.0 + f * 0.06);
    }
}
