//! Sound (board row C10): every effect and the ambient loop are synthesised in code at start-up, so there are
//! no sound files to credit. All ours (F7: our assets are CC BY 4.0).

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;

const RATE: u32 = 22_050;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sfx {
    Step,
    Bump,
    Jump,
    Land,
    Hit,
    Miss,
    Hurt,
    Death,
    Door,
    Lever,
    Chest,
    Pickup,
    Spell,
    Fire,
    LevelUp,
    Stairs,
    Splash,
    Win,
    Lose,
}

#[derive(Resource, Default)]
pub struct Sounds {
    pub fx: BTreeMap<Sfx, Handle<AudioSource>>,
    pub ambient: Handle<AudioSource>,
}

#[derive(Component)]
pub struct Ambient;

/// A tiny noise source (the app's, not the sim's: sounds needn't be reproducible, but this keeps them the same).
struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32_000.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Render `secs` of sound from a function of (time, progress 0..1, noise).
fn synth(secs: f32, seed: u32, f: impl Fn(f32, f32, &mut Noise) -> f32) -> Vec<f32> {
    let n = (secs * RATE as f32) as usize;
    let mut noise = Noise(seed | 1);
    let mut out = Vec::with_capacity(n);
    let mut lp = 0.0f32;
    for i in 0..n {
        let t = i as f32 / RATE as f32;
        let p = i as f32 / n as f32;
        let v = f(t, p, &mut noise);
        // a gentle low-pass keeps everything soft and old
        lp += (v - lp) * 0.45;
        out.push(lp);
    }
    out
}

fn env(p: f32, attack: f32) -> f32 {
    if p < attack {
        p / attack
    } else {
        (1.0 - (p - attack) / (1.0 - attack)).max(0.0).powf(2.0)
    }
}

fn sine(t: f32, f: f32) -> f32 {
    (t * f * std::f32::consts::TAU).sin()
}

fn square(t: f32, f: f32) -> f32 {
    if (t * f).fract() < 0.5 {
        0.6
    } else {
        -0.6
    }
}

fn make(s: Sfx) -> Vec<f32> {
    match s {
        Sfx::Step => synth(0.09, 3, |t, p, n| (n.next() * 0.5 + sine(t, 70.0)) * env(p, 0.05) * 0.5),
        Sfx::Bump => synth(0.12, 5, |t, p, n| (sine(t, 55.0) + n.next() * 0.3) * env(p, 0.02) * 0.7),
        Sfx::Jump => synth(0.18, 7, |t, p, n| (n.next() * 0.3 + sine(t, 180.0 + p * 260.0) * 0.4) * env(p, 0.2) * 0.5),
        Sfx::Land => synth(0.15, 9, |t, p, n| (n.next() * 0.6 + sine(t, 60.0)) * env(p, 0.02) * 0.7),
        Sfx::Hit => synth(0.16, 11, |t, p, n| (n.next() * 0.8 + square(t, 90.0 - p * 40.0) * 0.4) * env(p, 0.01) * 0.7),
        Sfx::Miss => synth(0.14, 13, |_, p, n| n.next() * env(p, 0.4) * 0.35),
        Sfx::Hurt => synth(0.25, 15, |t, p, n| (square(t, 140.0 - p * 60.0) * 0.5 + n.next() * 0.2) * env(p, 0.05) * 0.5),
        Sfx::Death => synth(0.6, 17, |t, p, n| (square(t, 160.0 - p * 120.0) * 0.4 + n.next() * 0.25) * env(p, 0.03) * 0.5),
        Sfx::Door => synth(0.45, 19, |t, p, n| (square(t, 110.0 + sine(t, 7.0) * 30.0) * 0.3 + n.next() * 0.15) * env(p, 0.1) * 0.5),
        Sfx::Lever => synth(0.3, 21, |t, p, n| {
            let click = if !(0.08..=0.55).contains(&p) { n.next() } else { 0.0 };
            (click + sine(t, 90.0) * 0.4) * env(p, 0.02) * 0.7
        }),
        Sfx::Chest => synth(0.6, 23, |t, p, n| (square(t, 95.0 + p * 40.0) * 0.2 * (1.0 - p) + sine(t, 880.0) * 0.25 * (p > 0.6) as u8 as f32 + n.next() * 0.1) * env(p, 0.05) * 0.6),
        Sfx::Pickup => synth(0.15, 25, |t, p, _| (sine(t, 660.0) + sine(t, 990.0) * 0.5 * (p > 0.4) as u8 as f32) * env(p, 0.05) * 0.35),
        Sfx::Spell => synth(0.5, 27, |t, p, _| {
            let f = [523.0, 659.0, 784.0, 1046.0][((p * 4.0) as usize).min(3)];
            (sine(t, f) * 0.5 + sine(t, f * 2.01) * 0.2) * env(p, 0.05) * 0.4
        }),
        Sfx::Fire => synth(0.5, 29, |t, p, n| (n.next() * 0.9 + sine(t, 70.0) * 0.3) * env(p, 0.08) * 0.6),
        Sfx::LevelUp => synth(0.8, 31, |t, p, _| {
            let f = [392.0, 523.0, 659.0, 784.0, 1046.0][((p * 5.0) as usize).min(4)];
            (square(t, f) * 0.3 + sine(t, f) * 0.3) * env(p, 0.02) * 0.4
        }),
        Sfx::Stairs => synth(0.9, 33, |t, p, n| {
            let step = (p * 6.0).fract() < 0.25;
            (if step { n.next() * 0.6 } else { 0.0 } + sine(t, 60.0 - p * 20.0) * 0.4) * env(p, 0.02) * 0.6
        }),
        Sfx::Splash => synth(0.35, 35, |_, p, n| n.next() * env(p, 0.05) * 0.5),
        Sfx::Win => synth(2.2, 37, |t, p, _| {
            let f = [262.0, 330.0, 392.0, 523.0, 659.0, 784.0][((p * 6.0) as usize).min(5)];
            (sine(t, f) * 0.4 + sine(t, f * 1.5) * 0.2) * env(p, 0.02) * 0.5
        }),
        Sfx::Lose => synth(2.0, 39, |t, p, _| (sine(t, 196.0 - p * 80.0) * 0.4 + sine(t, 147.0 - p * 60.0) * 0.3) * env(p, 0.02) * 0.5),
    }
}

/// OURS: the ambient loop, sixteen seconds of a low drone, slow breathing noise and a far-off drip.
fn ambient() -> Vec<f32> {
    let secs = 16.0;
    synth(secs, 41, |t, _, n| {
        let swell = 0.6 + 0.4 * sine(t, 1.0 / secs);
        let drone = sine(t, 55.0) * 0.25 + sine(t, 82.5) * 0.12 + sine(t, 110.0 + sine(t, 0.25) * 0.8) * 0.08;
        let air = n.next() * 0.04 * (0.5 + 0.5 * sine(t, 1.0 / 8.0));
        let dt = (t * 0.75).fract();
        let drip = if (t as u32) % 5 == 2 { sine(t, 1200.0 - dt * 600.0) * (-dt * 30.0).exp() * 0.25 } else { 0.0 };
        (drone * swell + air + drip) * 0.8
    })
}

pub fn build(audio: &mut Assets<AudioSource>) -> Sounds {
    let mut s = Sounds::default();
    for fx in [
        Sfx::Step, Sfx::Bump, Sfx::Jump, Sfx::Land, Sfx::Hit, Sfx::Miss, Sfx::Hurt, Sfx::Death, Sfx::Door, Sfx::Lever,
        Sfx::Chest, Sfx::Pickup, Sfx::Spell, Sfx::Fire, Sfx::LevelUp, Sfx::Stairs, Sfx::Splash, Sfx::Win, Sfx::Lose,
    ] {
        let bytes: Arc<[u8]> = wav(&make(fx)).into();
        s.fx.insert(fx, audio.add(AudioSource { bytes }));
    }
    let bytes: Arc<[u8]> = wav(&ambient()).into();
    s.ambient = audio.add(AudioSource { bytes });
    s
}

pub fn play(commands: &mut Commands, sounds: &Sounds, fx: Sfx, volume: f32) {
    if volume <= 0.0 {
        return;
    }
    if let Some(h) = sounds.fx.get(&fx) {
        commands.spawn((AudioPlayer::new(h.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume))));
    }
}

pub fn start_ambient(commands: &mut Commands, sounds: &Sounds, volume: f32) {
    commands.spawn((AudioPlayer::new(sounds.ambient.clone()), PlaybackSettings::LOOP.with_volume(Volume::Linear(volume)), Ambient));
}
