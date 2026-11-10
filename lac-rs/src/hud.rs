//! The HUD (board row X1) and the automap (G12). REF R30: the four portraits sit on the right of the screen.
//! REF R35: a minimap of what the party has seen. REF R34: an inventory with item icons.
//!
//! OURS: the layout, after ed-slopper's direction (2026-11-08): the view fills the height on the left; on the
//! right a stone panel holds the party as a 2×2 grid of cards (name, four equipment slots, portrait, health and
//! mana bars) and the pack as a 6×4 grid of icons, then the map. Everything there can be clicked. The overlays
//! (character sheet, spells, pause, map, help, debug: board row E11) sit on the view.

use crate::game::{spell_list, Game, Overlay, ViewImage};
use crate::icons;
use crate::palette::{self, ix, BLUE, BONE, BROWN, FIRE, GREEN, GREY, RED, VIOLET};
use crate::settings::{Command, Settings};
use bevy::asset::RenderAssetUsages;
use bevy::picking::events::PointerClick;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::FontSize;
use lac_sim::content::{ItemKind, Slot};
use lac_sim::world::Feature;
use lac_sim::{Action, Cell, IVec3 as SIVec3, Sim};
use std::collections::BTreeMap;

pub const LOG_LINES: usize = 6;
pub const MAP: usize = 48;
/// The small map shows this many tiles across, centred on the party.
pub const SMALL_MAP: usize = 24;
const PANEL_W: f32 = 452.0;
const SLOT: f32 = 34.0;
const PACK_SLOT: f32 = 58.0;
/// The slots a card shows; the rest are on the character sheet.
const CARD_SLOTS: [Slot; 7] = [Slot::Weapon, Slot::Shield, Slot::Head, Slot::Body, Slot::Feet, Slot::Ring, Slot::Amulet];

/// The faint glyph an empty card slot shows (a hand for the hands, a ring, a gem for the amulet).
fn slot_glyph(slot: Slot) -> &'static str {
    match slot {
        Slot::Weapon | Slot::Shield => "hand",
        Slot::Head => "helm",
        Slot::Body => "armor",
        Slot::Feet => "boots",
        Slot::Ring => "ring",
        Slot::Amulet => "gem",
    }
}

/// Where an item for a slot can come from, in the picker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PickFrom {
    Pack(usize),
    /// A potion, scroll or wand from the pack, to hold in the second hand.
    Hold(usize),
    Member(usize),
    TakeOff,
    Close,
}

#[derive(Component)]
pub struct HudRoot;
#[derive(Component)]
pub struct TopBar;
#[derive(Component)]
pub struct LogText(pub usize);
#[derive(Component)]
pub struct PartyPanel;
#[derive(Component)]
pub struct Minimap;
#[derive(Component)]
pub struct OverlayPanel;
#[derive(Component)]
pub struct DebugText;
#[derive(Component)]
pub struct HurtFlash;
#[derive(Component)]
pub struct Hints;
/// The 3D view in the HUD; sized each frame to the biggest 4:3 that fits.
#[derive(Component)]
pub struct ViewNode;

/// What a click on this node does.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum Clickable {
    Member(usize),
    Equip(usize, Slot),
    Pack(usize),
    /// The 3D view: use what is in front, or attack.
    View,
    /// A portrait: that member strikes alone.
    Portrait(usize),
    /// A row of the slot picker.
    Pick(PickFrom),
}

/// What the pointer is over, for the tooltip line.
#[derive(Resource, Default)]
pub struct Hover(pub Option<Clickable>);

#[derive(Component)]
pub struct Tooltip;

/// The white streak of a melee swing over the view.
#[derive(Component)]
pub struct Slash;

/// A diagonal streak, thick in the middle, made in code.
fn slash_image() -> Image {
    let n = 64usize;
    let mut img = blank(n, n);
    let data = img.data.as_mut().unwrap();
    for y in 0..n {
        for x in 0..n {
            // distance from the line x + y = n, along a gentle arc
            let t = x as f32 / n as f32;
            let arc = (t * std::f32::consts::PI).sin() * 6.0;
            let d = ((x + y) as f32 - n as f32 + arc).abs();
            let width = 1.0 + 3.5 * (t * std::f32::consts::PI).sin();
            if d < width {
                let a = (255.0 * (1.0 - d / width)).clamp(0.0, 255.0) as u8;
                let k = (y * n + x) * 4;
                data[k..k + 4].copy_from_slice(&[255, 250, 235, a]);
            }
        }
    }
    img
}

pub fn update_slash(game: Res<Game>, mut q: Query<(&mut ImageNode, &mut Visibility), With<Slash>>) {
    for (mut img, mut vis) in &mut q {
        if game.slash > 0.0 {
            *vis = Visibility::Visible;
            let a = (game.slash / 0.2).clamp(0.0, 1.0);
            img.color = if game.slash_hit { Color::srgba(1.0, 0.9, 0.8, a) } else { Color::srgba(0.7, 0.7, 0.7, a * 0.6) };
            img.flip_x = !game.slash_hit;
        } else if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

pub fn on_over(mut over: On<bevy::picking::events::PointerOver>, q: Query<&Clickable>, mut hover: ResMut<Hover>) {
    if let Ok(c) = q.get(over.entity) {
        hover.0 = Some(*c);
        over.propagate(false);
    }
}

pub fn on_out(mut out: On<bevy::picking::events::PointerOut>, q: Query<&Clickable>, mut hover: ResMut<Hover>) {
    if let Ok(c) = q.get(out.entity) {
        if hover.0 == Some(*c) {
            hover.0 = None;
        }
        out.propagate(false);
    }
}

/// The tooltip line: what the pointer is over.
pub fn update_tooltip(hover: Res<Hover>, game: Res<Game>, mut q: Query<&mut Text, With<Tooltip>>) {
    let Ok(mut t) = q.single_mut() else { return };
    let sim = &game.sim;
    let s = &sim.state;
    let text = match hover.0 {
        Some(Clickable::Pack(k)) => match s.pack.get(k) {
            Some(it) => {
                let who = &s.party[target_for(&game, k)].name;
                let drink = format!("click: {who} drinks");
                let note = match sim.content.item(&it.def).kind {
                    ItemKind::Potion { .. } => drink.as_str(),
                    ItemKind::Scroll { .. } => "click: read",
                    ItemKind::Wand { .. } => "click: zap ahead",
                    ItemKind::Weapon { .. } | ItemKind::Armor { .. } | ItemKind::Ring { .. } | ItemKind::Amulet { .. } => "click: wear",
                    _ => "",
                };
                format!("{}   {note}   right-click: drop", sim.item_name(it))
            }
            None => String::new(),
        },
        Some(Clickable::Equip(i, slot)) => match s.party[i].equip.get(&slot) {
            Some(it) if lac_sim::State::holdable(&sim.content, it) => format!("{} holds {}   click: use it   right-click: to the pack", s.party[i].name, sim.item_name(it)),
            Some(it) => format!("{}'s {}: {}   click: change   right-click: to the pack", s.party[i].name, slot.name(), sim.item_name(it)),
            None => format!("{}'s {}: nothing", s.party[i].name, slot.name()),
        },
        Some(Clickable::Member(i)) => {
            let m = &s.party[i];
            format!(
                "{} the {} {}   STR {} DEX {} CON {} INT {} WIS {}   click: acts",
                m.name, m.class.name(), m.level, m.attrs[0], m.attrs[1], m.attrs[2], m.attrs[3], m.attrs[4]
            )
        }
        Some(Clickable::View) => "click: use what is in front   right-click: attack".to_string(),
        Some(Clickable::Portrait(i)) => format!("click: {} attacks   right-click: {} acts", s.party[i].name, s.party[i].name),
        Some(Clickable::Pick(_)) => "click to choose".to_string(),
        None => String::new(),
    };
    if t.0 != text {
        t.0 = text;
    }
}

#[derive(Resource)]
pub struct HudImages {
    pub map: Handle<Image>,
    pub big_map: Handle<Image>,
    pub portraits: Vec<Handle<Image>>,
    /// Item icons by (icon, liquid colour).
    pub icons: BTreeMap<(String, u8), Handle<Image>>,
}

pub fn font(px: f32) -> TextFont {
    TextFont { font_size: FontSize::Px(px), ..default() }
}

pub const INK: Color = Color::srgb(0.88, 0.85, 0.78);
pub const DIM: Color = Color::srgb(0.58, 0.56, 0.52);
pub const PANEL: Color = Color::srgb(0.07, 0.065, 0.075);
pub const GOLD: Color = Color::srgb(0.95, 0.78, 0.35);
const CARD: Color = Color::srgba(0.10, 0.10, 0.11, 0.82);
const CARD_SEL: Color = Color::srgba(0.26, 0.21, 0.12, 0.92);
const SLOT_EDGE: Color = Color::srgb(0.33, 0.33, 0.35);
const SLOT_FILL: Color = Color::srgb(0.13, 0.13, 0.15);

fn blank(w: usize, h: usize) -> Image {
    let mut img = Image::new(
        Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![0; w * h * 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    img
}

/// A rough grey stone for the side panel, made in code (OURS).
fn stone_image(vision: palette::Vision) -> Image {
    let (w, h) = (113usize, 200usize);
    let cols = palette::colours(vision);
    let mut img = blank(w, h);
    let data = img.data.as_mut().unwrap();
    let hash = |x: usize, y: usize, s: u32| -> u32 {
        let mut v = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77) ^ s.wrapping_mul(0xC2B2_AE3D);
        v ^= v >> 15;
        v = v.wrapping_mul(0x2C1B_3C6D);
        v ^ (v >> 12)
    };
    for y in 0..h {
        for x in 0..w {
            let blob = (hash(x / 6, y / 6, 1) % 3) as u8;
            let grain = (hash(x, y, 2) % 7) as u8;
            let shade = 3 + blob + if grain == 0 { 1 } else { 0 } - if grain == 6 { 1 } else { 0 };
            let c = cols[ix(GREY, shade.min(6)) as usize];
            let k = (y * w + x) * 4;
            data[k..k + 4].copy_from_slice(&[(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 255]);
        }
    }
    img
}

pub fn spawn_hud(mut commands: Commands, mut images: ResMut<Assets<Image>>, view: Res<ViewImage>, game: Res<Game>, settings: Res<Settings>) {
    let vision = settings.file.vision;
    let map = images.add(blank(SMALL_MAP, SMALL_MAP));
    let big_map = images.add(blank(MAP, MAP));
    let stone = images.add(stone_image(vision));
    let slash = images.add(slash_image());
    let portraits: Vec<Handle<Image>> = game.sim.state.party.iter().map(|m| images.add(icons::portrait(m.class, m.portrait, vision))).collect();
    commands.insert_resource(HudImages { map: map.clone(), big_map, portraits, icons: BTreeMap::new() });

    commands
        .spawn((Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Row, ..default() }, BackgroundColor(Color::srgb(0.02, 0.02, 0.025)), HudRoot))
        .with_children(|root| {
            // left: the view, the full height
            root.spawn(Node { flex_grow: 1.0, min_width: px(0), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, overflow: Overflow::clip(), ..default() })
                .with_children(|vc| {
                    vc.spawn((ImageNode::new(view.0.clone()), Node { width: px(640), height: px(480), ..default() }, ViewNode, Clickable::View)).with_children(|v| {
                        v.spawn((Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() }, BackgroundColor(Color::NONE), HurtFlash));
                        v.spawn((
                            ImageNode::new(slash.clone()),
                            Node { position_type: PositionType::Absolute, left: percent(25), top: percent(15), width: percent(50), height: percent(66), ..default() },
                            Visibility::Hidden,
                            Slash,
                        ));
                        v.spawn((Node { position_type: PositionType::Absolute, left: px(10), top: px(8), ..default() }, Text::new(""), font(15.0), TextColor(GOLD), TopBar));
                        v.spawn((Node { position_type: PositionType::Absolute, right: px(10), top: px(8), ..default() }, Text::new(""), font(13.0), TextColor(Color::srgb(0.6, 1.0, 0.6)), DebugText));
                        // the message log, over the bottom of the view
                        v.spawn((
                            Node { position_type: PositionType::Absolute, left: px(0), right: px(0), bottom: px(0), flex_direction: FlexDirection::Column, padding: UiRect::axes(px(10), px(6)), ..default() },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
                        ))
                        .with_children(|log| {
                            for i in 0..LOG_LINES {
                                log.spawn((Text::new(""), font(14.0), TextColor(INK), LogText(i)));
                            }
                        });
                        v.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: percent(5),
                                right: percent(5),
                                top: percent(5),
                                bottom: percent(22),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(px(14)),
                                row_gap: px(2),
                                display: Display::None,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.04, 0.035, 0.045, 0.94)),
                            OverlayPanel,
                        ));
                    });
                });
            // right: the stone panel
            root.spawn((
                ImageNode::new(stone.clone()),
                Node { width: px(PANEL_W), height: percent(100), flex_direction: FlexDirection::Column, padding: UiRect::all(px(10)), row_gap: px(10), ..default() },
            ))
            .with_children(|right| {
                right.spawn((Node { flex_direction: FlexDirection::Column, row_gap: px(8), ..default() }, PartyPanel));
                right.spawn((Node { min_height: px(18), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)))).with_children(|t| {
                    t.spawn((Text::new(""), font(13.0), TextColor(GOLD), Tooltip));
                });
                right.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(10), align_items: AlignItems::FlexStart, ..default() }).with_children(|row| {
                    row.spawn((Node { padding: UiRect::all(px(4)), ..default() }, BackgroundColor(PANEL))).with_children(|m| {
                        m.spawn((ImageNode::new(map.clone()), Node { width: px(150), height: px(150), ..default() }, Minimap));
                    });
                    row.spawn((Node { flex_grow: 1.0, padding: UiRect::all(px(6)), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)))).with_children(|h| {
                        h.spawn((Text::new(""), font(12.0), TextColor(INK), Hints));
                    });
                });
            });
        });
}

/// Keep the view the biggest 4:3 that fits beside the panel.
pub fn fit_view(windows: Query<&Window>, mut view: Query<&mut Node, With<ViewNode>>) {
    let Ok(w) = windows.single() else { return };
    let (ww, wh) = (w.width() - PANEL_W, w.height());
    let width = ww.min(wh * 4.0 / 3.0).max(64.0).floor();
    let height = (width * 3.0 / 4.0).floor();
    for mut n in &mut view {
        if n.width != px(width) || n.height != px(height) {
            n.width = px(width);
            n.height = px(height);
        }
    }
}

pub fn despawn_hud(mut commands: Commands, q: Query<Entity, With<HudRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn icon_handle(hud: &mut HudImages, images: &mut Assets<Image>, sim: &Sim, it: &lac_sim::item::Item, vision: palette::Vision) -> Handle<Image> {
    let def = sim.content.item(&it.def);
    let liquid = sim.state.knowledge.looks.get(&it.def).map(|l| icons::look_colour(l)).unwrap_or((ix(RED, 4), ix(RED, 6)));
    hud.icons.entry((def.icon.clone(), liquid.0)).or_insert_with(|| images.add(icons::icon_image(&icons::icon(&def.icon, liquid), vision))).clone()
}

/// OURS: how rare and strong an item looks, from what the party knows of it (ed-slopper: grey, blue, purple,
/// orange). 0 common; 1 magic (a known +1 or +2, a ring, amulet or wand, or a find from depth 4 down); 2 rare (a
/// set piece, or a known +3 or better); 3 unique. A hidden enchantment never shows through the colour, and a
/// known curse makes anything common.
pub fn tier_of(sim: &Sim, it: &lac_sim::item::Item) -> u8 {
    let def = sim.content.item(&it.def);
    if it.known && it.cursed {
        return 0;
    }
    if def.unique {
        return 3;
    }
    let plus = if it.known { it.enchant } else { 0 };
    if def.set.is_some() || plus >= 3 {
        return 2;
    }
    let magic = matches!(def.kind, ItemKind::Ring { .. } | ItemKind::Amulet { .. } | ItemKind::Wand { .. });
    if plus >= 1 || magic || def.depth.0 >= 4 {
        return 1;
    }
    0
}

/// The colour of a tier.
pub fn tier_colour(tier: u8) -> Color {
    match tier {
        0 => Color::srgb(0.55, 0.55, 0.58),
        1 => Color::srgb(0.30, 0.56, 1.0),
        2 => Color::srgb(0.70, 0.36, 0.98),
        _ => Color::srgb(1.0, 0.56, 0.14),
    }
}

/// A square slot: a light edge, a dark well, the item's icon if any (or a faint glyph if empty). An item's
/// tier colours the edge, tints the well and, above common, makes a soft glow round the slot.
fn slot_node(p: &mut ChildSpawnerCommands, size: f32, icon: Option<(Handle<Image>, u8)>, glyph: Option<Handle<Image>>, highlight: bool, click: Clickable) {
    let tier = icon.as_ref().map(|i| i.1);
    let icon = icon.map(|i| i.0);
    let tc = tier.map(tier_colour);
    let edge = if highlight {
        GOLD
    } else {
        match (tier, tc) {
            (Some(t), Some(c)) if t > 0 => c,
            _ => SLOT_EDGE,
        }
    };
    let fill = match (tier, tc) {
        (Some(t), Some(c)) if t > 0 => {
            let (a, b) = (SLOT_FILL.to_srgba(), c.to_srgba());
            let k = 0.24;
            Color::srgb(a.red + (b.red - a.red) * k, a.green + (b.green - a.green) * k, a.blue + (b.blue - a.blue) * k)
        }
        _ => SLOT_FILL,
    };
    let mut outer = p.spawn((Node { width: px(size), height: px(size), padding: UiRect::all(px(2)), ..default() }, BackgroundColor(edge), click));
    if let (Some(t), Some(c)) = (tier, tc) {
        if t > 0 {
            outer.insert(bevy::ui::BoxShadow::new(c.with_alpha(0.35 + 0.15 * t as f32), Val::ZERO, Val::ZERO, px(1), px(4.0 + 2.0 * t as f32)));
        }
    }
    outer.with_children(|s| {
        s.spawn((Node { width: percent(100), height: percent(100), padding: UiRect::all(px(4)), ..default() }, BackgroundColor(fill))).with_children(|w| {
            if let Some(h) = icon {
                w.spawn((ImageNode::new(h), Node { width: percent(100), height: percent(100), ..default() }));
            } else if let Some(g) = glyph {
                w.spawn((ImageNode { color: Color::srgba(1.0, 1.0, 1.0, 0.28), ..ImageNode::new(g) }, Node { width: percent(100), height: percent(100), ..default() }));
            }
        });
    });
}

fn glyph_handle(hud: &mut HudImages, images: &mut Assets<Image>, name: &str, vision: palette::Vision) -> Handle<Image> {
    hud.icons.entry((format!("glyph:{name}"), 0)).or_insert_with(|| images.add(icons::icon_image(&icons::icon(name, (ix(GREY, 5), ix(GREY, 7))), vision))).clone()
}

fn bar(p: &mut ChildSpawnerCommands, frac: f32, colour: Color) {
    p.spawn((Node { height: px(7), width: percent(100), ..default() }, BackgroundColor(Color::srgb(0.12, 0.11, 0.12)))).with_children(|b| {
        b.spawn((Node { height: percent(100), width: percent(100.0 * frac.clamp(0.0, 1.0)), ..default() }, BackgroundColor(colour)));
    });
}

/// The party cards and the pack: rebuilt whenever something in them changes.
fn party_panel(p: &mut ChildSpawnerCommands, game: &Game, hud: &mut HudImages, images: &mut Assets<Image>, vision: palette::Vision) {
    let sim = &game.sim;
    let s = &sim.state;
    let actor = game.actor();
    for row in 0..2 {
        p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(8), ..default() }).with_children(|r| {
            for col in 0..2 {
                let i = row * 2 + col;
                let Some(m) = s.party.get(i) else { continue };
                let flash = game.member_flash[i];
                let base = if i == actor { CARD_SEL } else { CARD };
                let bg = base.mix(&Color::srgb(0.55, 0.08, 0.05), flash * 0.7);
                r.spawn((Node { width: px((PANEL_W - 28.0) / 2.0), flex_direction: FlexDirection::Column, padding: UiRect::all(px(6)), row_gap: px(4), ..default() }, BackgroundColor(bg), Clickable::Member(i)))
                    .with_children(|card| {
                        let row_name = if i < 2 { "front" } else { "back" };
                        card.spawn((Text::new(format!("{}  {} {}", m.name, m.class.name(), m.level)), font(14.0), TextColor(if m.alive() { INK } else { DIM })));
                        card.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(6), ..default() }).with_children(|body| {
                            body.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(4), ..default() }).with_children(|slots| {
                                for pair in CARD_SLOTS.chunks(2) {
                                    slots.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(4), ..default() }).with_children(|sr| {
                                        for slot in pair {
                                            let icon = m.equip.get(slot).map(|it| (icon_handle(hud, images, sim, it, vision), tier_of(sim, it)));
                                            let glyph = Some(glyph_handle(hud, images, slot_glyph(*slot), vision));
                                            let open = game.picker == Some((i, *slot));
                                            slot_node(sr, SLOT, icon, glyph, open, Clickable::Equip(i, *slot));
                                        }
                                    });
                                }
                            });
                            let portrait = hud.portraits.get(m.portrait as usize).cloned().unwrap_or_default();
                            body.spawn((
                                ImageNode { color: if m.alive() { Color::WHITE } else { Color::srgb(0.25, 0.2, 0.2) }, ..ImageNode::new(portrait) },
                                Node { width: px(96), height: px(96), ..default() },
                                Clickable::Portrait(i),
                            ));
                        });
                        bar(card, m.hp.max(0) as f32 / m.max_hp.max(1) as f32, Color::srgb(0.30, 0.78, 0.25));
                        bar(card, if m.max_mp > 0 { m.mp as f32 / m.max_mp as f32 } else { 0.0 }, Color::srgb(0.62, 0.30, 0.85));
                        let status = if !m.alive() {
                            format!("dead  ({row_name})")
                        } else {
                            let mut st = format!("HP {}/{}", m.hp, m.max_hp);
                            if m.max_mp > 0 {
                                st.push_str(&format!("  MP {}/{}", m.mp, m.max_mp));
                            }
                            st.push_str(&format!("  AC {}", m.ac(&sim.content)));
                            if m.poison > 0 {
                                st.push_str("  poison");
                            }
                            st
                        };
                        card.spawn((Text::new(status), font(12.0), TextColor(if m.poison > 0 { Color::srgb(0.5, 0.9, 0.4) } else { DIM })));
                    });
            }
        });
    }
    // the pack: 6 × 4
    let cursor_pack = if game.overlay == Overlay::Inventory && game.cursor >= 7 { Some(game.cursor - 7) } else { None };
    p.spawn((Node { flex_direction: FlexDirection::Column, row_gap: px(4), padding: UiRect::all(px(6)), ..default() }, BackgroundColor(CARD))).with_children(|pk| {
        pk.spawn((
            Text::new(format!("Pack {}/{}   gold {}   acting: {}", s.pack.len(), lac_sim::sim::PACK_SIZE, s.gold, s.party[actor].name)),
            font(13.0),
            TextColor(GOLD),
        ));
        for row in 0..4 {
            pk.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(4), ..default() }).with_children(|r| {
                for col in 0..6 {
                    let k = row * 6 + col;
                    let icon = s.pack.get(k).map(|it| (icon_handle(hud, images, sim, it, vision), tier_of(sim, it)));
                    slot_node(r, PACK_SLOT, icon, None, cursor_pack == Some(k), Clickable::Pack(k));
                }
            });
        }
        pk.spawn((Text::new("pack: click uses, right-click drops   slots: click to choose, right-click to pack   portrait: attack"), font(11.0), TextColor(DIM)));
    });
}

/// Who should drink or read pack item `k`: a known healing potion goes to whoever is most hurt, mana to the caster
/// lowest on it, purging to the poisoned; anything else to the acting member.
fn target_for(game: &Game, k: usize) -> usize {
    use lac_sim::content::Effect;
    let s = &game.sim.state;
    let c = &game.sim.content;
    let Some(it) = s.pack.get(k) else { return game.actor() };
    if !s.knowledge.kind_known(c, &it.def) {
        return game.actor();
    }
    let alive = || s.party.iter().enumerate().filter(|(_, m)| m.alive());
    let pick = match &c.item(&it.def).kind {
        ItemKind::Potion { effect: Effect::Heal(_) } => alive().filter(|(_, m)| m.hp < m.max_hp).min_by_key(|(_, m)| m.hp * 100 / m.max_hp.max(1)).map(|(i, _)| i),
        ItemKind::Potion { effect: Effect::Mana(_) } => alive().filter(|(_, m)| m.max_mp > 0 && m.mp < m.max_mp).min_by_key(|(_, m)| m.mp * 100 / m.max_mp.max(1)).map(|(i, _)| i),
        ItemKind::Potion { effect: Effect::Cure } => alive().find(|(_, m)| m.poison > 0).map(|(i, _)| i),
        _ => None,
    };
    pick.unwrap_or_else(|| game.actor())
}

/// Clicks on the panel: use and drop items, take things off, pick who acts.
pub fn on_click(mut click: On<PointerClick>, q: Query<&Clickable>, game: Option<ResMut<Game>>) {
    let Ok(c) = q.get(click.entity) else { return };
    click.propagate(false);
    let Some(mut game) = game else { return };
    if game.end_timer.is_some() {
        return;
    }
    if game.resting {
        game.resting = false;
        game.sim.clear_queue();
        game.say("You stop resting.");
        return;
    }
    let primary = click.button == PointerButton::Primary;
    match *c {
        Clickable::Member(i) => {
            game.selected = i;
            game.dirty.hud = true;
        }
        Clickable::Equip(i, slot) => {
            let held = game.sim.state.party[i].equip.get(&slot).is_some_and(|it| lac_sim::State::holdable(&game.sim.content, it));
            if primary && held {
                // a held potion, scroll or wand: use it, for this member (R63)
                game.push(Action::UseHeld { member: i as u8, main: slot == Slot::Weapon });
            } else if primary {
                // a list of what can go here: from the pack, or from someone else (R63)
                game.picker = Some((i, slot));
                game.overlay = Overlay::Picker;
            } else if game.sim.state.party[i].equip.contains_key(&slot) {
                // right-click: back to the pack
                game.push(Action::Unequip { member: i as u8, slot });
            }
            game.dirty.hud = true;
        }
        Clickable::Portrait(i) => {
            if primary {
                game.push(Action::AttackWith { member: i as u8 });
            } else {
                game.selected = i;
                game.dirty.hud = true;
            }
        }
        Clickable::Pick(from) => {
            let Some((i, slot)) = game.picker else { return };
            let member = i as u8;
            match from {
                PickFrom::Pack(k) => game.push(Action::Equip { member, index: k as u16 }),
                PickFrom::Hold(k) => game.push(Action::Hold { member, index: k as u16, main: slot == Slot::Weapon }),
                PickFrom::Member(j) => {
                    // off them into the pack (it lands at the end), then onto this member
                    let at = game.sim.state.pack.len() as u16;
                    game.push(Action::Unequip { member: j as u8, slot });
                    game.push(Action::Equip { member, index: at });
                }
                PickFrom::TakeOff => game.push(Action::Unequip { member, slot }),
                PickFrom::Close => {}
            }
            game.picker = None;
            game.overlay = Overlay::None;
            game.dirty.hud = true;
        }
        Clickable::View => {
            if game.overlay == Overlay::None {
                game.push(if primary { Action::Use } else { Action::Attack });
            }
        }
        Clickable::Pack(k) => {
            if k >= game.sim.state.pack.len() {
                return;
            }
            if primary {
                let member = target_for(&game, k) as u8;
                game.push(Action::UseItem { member, index: k as u16 });
            } else {
                game.push(Action::Drop { index: k as u16 });
            }
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_hud(
    mut commands: Commands,
    mut game: ResMut<Game>,
    settings: Res<Settings>,
    mut hud: ResMut<HudImages>,
    mut images: ResMut<Assets<Image>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<TopBar>>,
        Query<(&mut Text, &mut TextColor, &LogText)>,
        Query<&mut Text, With<DebugText>>,
        Query<&mut Text, With<Hints>>,
    )>,
    mut flash: Query<&mut BackgroundColor, With<HurtFlash>>,
    party: Query<Entity, With<PartyPanel>>,
    mut panel: Query<(Entity, &mut Node), With<OverlayPanel>>,
    mut last_flash: Local<bool>,
) {
    for mut bg in &mut flash {
        bg.0 = Color::srgba(0.8, 0.05, 0.02, game.hurt_flash * 0.35);
    }
    // cards flash red when hurt: keep rebuilding while a flash fades
    let flashing = game.member_flash.iter().any(|f| *f > 0.0);
    if flashing || *last_flash {
        game.dirty.hud = true;
    }
    *last_flash = flashing;
    {
        let s = &game.sim.state;
        let mut q = texts.p2();
        if let Ok(mut t) = q.single_mut() {
            t.0 = if game.debug {
                format!(
                    "seed {}  tick {}  queue {}\ntile ({}, {}, {}) facing {:?}\ndepth {}  monsters {}\nframe {:.1} ms\nhash {:016x}",
                    s.seed_text,
                    s.tick,
                    game.sim.queued(),
                    s.pos.x,
                    s.pos.y,
                    s.pos.z,
                    s.facing,
                    s.level.depth,
                    s.level.monsters.len(),
                    game.frame_ms,
                    game.sim.state_hash()
                )
            } else {
                String::new()
            };
        }
    }
    {
        let n = game.log.len();
        let mut q = texts.p1();
        for (mut t, mut col, line) in q.iter_mut() {
            let k = n as isize - LOG_LINES as isize + line.0 as isize;
            if k >= 0 {
                let l = &game.log[k as usize];
                t.0 = l.text.clone();
                let fresh = (1.0 - l.age / 6.0).clamp(0.0, 1.0);
                col.0 = DIM.mix(&INK, 0.35 + 0.65 * fresh);
            } else {
                t.0.clear();
            }
        }
    }
    if !std::mem::take(&mut game.dirty.hud) {
        return;
    }
    let vision = settings.file.vision;
    {
        let sim = &game.sim;
        let s = &sim.state;
        let mut q = texts.p0();
        if let Ok(mut t) = q.single_mut() {
            let mut fx = Vec::new();
            if s.haste > 0 {
                fx.push(format!("quick {}", s.haste));
            }
            if s.levitate > 0 {
                fx.push(format!("floating: {} more", s.levitate));
            } else if s.levitating(&sim.content) {
                // a worn item does it: name it, so the player knows what to take off
                let mut by = String::from("something worn");
                for m in s.party.iter().filter(|m| m.alive()) {
                    for it in m.equip.values() {
                        let lev = match &sim.content.item(&it.def).kind {
                            ItemKind::Ring { effect } | ItemKind::Amulet { effect } => matches!(effect, lac_sim::content::Effect::Levitate(_)),
                            ItemKind::Armor { effect: Some(effect), .. } => matches!(effect, lac_sim::content::Effect::Levitate(_)),
                            _ => false,
                        };
                        if lev {
                            by = format!("{}'s {}", m.name, sim.item_name(it));
                        }
                    }
                }
                fx.push(format!("floating: {by}, take it off to land"));
            }
            if s.airborne {
                fx.push("in the air".to_string());
            }
            t.0 = format!(
                "Depth {} - {}{}{}",
                s.level.depth,
                s.level.theme.name(),
                if fx.is_empty() { String::new() } else { format!("    [{}]", fx.join(", ")) },
                if sim.queued() > 0 { format!("    queued {}", sim.queued()) } else { String::new() }
            );
        }
    }
    {
        let mut q = texts.p3();
        if let Ok(mut t) = q.single_mut() {
            let k = |c: Command| settings.main_key(c);
            t.0 = format!(
                "move {} {} {} {}\nturn {} {}\njump {}  long {}\nuse {}  attack {}\nwait {}  pick up {}\nsheet {}  spells {}\nmap {}  mark {}\nhelp {}  menu {}\nmembers 1-4  light [ ]",
                k(Command::Forward),
                k(Command::StrafeLeft),
                k(Command::Back),
                k(Command::StrafeRight),
                k(Command::TurnLeft),
                k(Command::TurnRight),
                k(Command::Jump),
                k(Command::LongJump),
                k(Command::Use),
                k(Command::Attack),
                k(Command::Wait),
                k(Command::PickUp),
                k(Command::Inventory),
                k(Command::Spells),
                k(Command::Map),
                k(Command::Note),
                k(Command::Help),
                k(Command::Menu),
            );
        }
    }
    // the party and the pack
    if let Ok(pe) = party.single() {
        commands.entity(pe).despawn_related::<Children>();
        let g = &*game;
        let hud = &mut *hud;
        let images = &mut *images;
        commands.entity(pe).with_children(|p| party_panel(p, g, hud, images, vision));
    }
    let sim = &game.sim;
    if let Some(mut img) = images.get_mut(&hud.map) {
        draw_map(&mut img, sim, vision, false);
    }
    if let Some(mut img) = images.get_mut(&hud.big_map) {
        draw_map(&mut img, sim, vision, true);
    }
    // overlays
    if let Ok((panel_e, mut node)) = panel.single_mut() {
        commands.entity(panel_e).despawn_related::<Children>();
        if game.overlay == Overlay::None {
            node.display = Display::None;
        } else {
            node.display = Display::Flex;
            let selected = game.actor();
            let cursor = game.cursor;
            let pause = game.pause_cursor;
            let s = &sim.state;
            let big_map = hud.big_map.clone();
            commands.entity(panel_e).with_children(|p| match game.overlay {
                Overlay::Inventory => inventory(p, sim, selected, cursor, vision, &mut images),
                Overlay::Spells => spells(p, sim),
                Overlay::Pause => pause_menu(p, pause),
                Overlay::Help => help(p, &settings),
                Overlay::Picker => picker(p, &game, &mut hud, &mut images, vision),
                Overlay::Map => {
                    p.spawn((Text::new(format!("Map of depth {}  (close: {})", s.level.depth, settings.describe(Command::Map))), font(15.0), TextColor(GOLD)));
                    p.spawn(Node { flex_grow: 1.0, justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }).with_children(|c| {
                        c.spawn((ImageNode::new(big_map), Node { height: percent(100), aspect_ratio: Some(1.0), ..default() }));
                    });
                }
                Overlay::None => {}
            });
        }
    }
}

/// The slot picker (R63): what can go into a member's slot, from the pack and from the others, labelled.
fn picker(p: &mut ChildSpawnerCommands, game: &Game, hud: &mut HudImages, images: &mut Assets<Image>, vision: palette::Vision) {
    let Some((i, slot)) = game.picker else { return };
    let sim = &game.sim;
    let s = &sim.state;
    let m = &s.party[i];
    let fits = |it: &lac_sim::item::Item| sim.content.item(&it.def).kind.slot() == Some(slot);
    line(p, format!("{}'s {}", m.name, slot.name()), GOLD, 18.0);
    // each item row wears its rarity, as in the pack: a tinted, glowing frame round the icon and the name in
    // the tier's colour (grey, blue, purple, orange)
    let row = |p: &mut ChildSpawnerCommands, icon: Option<(Handle<Image>, u8)>, text: String, colour: Color, from: PickFrom| {
        let tier = icon.as_ref().map(|i| i.1).unwrap_or(0);
        let tc = tier_colour(tier);
        let colour = if tier > 0 { tc } else { colour };
        p.spawn((Node { flex_direction: FlexDirection::Row, column_gap: px(8), align_items: AlignItems::Center, padding: UiRect::all(px(3)), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.04)), Clickable::Pick(from)))
            .with_children(|r| {
                match icon {
                    Some((h, _)) => {
                        let (edge, fill) = if tier > 0 {
                            let (a, b) = (SLOT_FILL.to_srgba(), tc.to_srgba());
                            let k = 0.3;
                            (tc, Color::srgb(a.red + (b.red - a.red) * k, a.green + (b.green - a.green) * k, a.blue + (b.blue - a.blue) * k))
                        } else {
                            (SLOT_EDGE, SLOT_FILL)
                        };
                        let mut frame = r.spawn((Node { width: px(34), height: px(34), padding: UiRect::all(px(2)), ..default() }, BackgroundColor(edge)));
                        if tier > 0 {
                            frame.insert(bevy::ui::BoxShadow::new(tc.with_alpha(0.35 + 0.15 * tier as f32), Val::ZERO, Val::ZERO, px(1), px(4.0 + 2.0 * tier as f32)));
                        }
                        frame.with_children(|f| {
                            f.spawn((Node { width: percent(100), height: percent(100), padding: UiRect::all(px(1)), ..default() }, BackgroundColor(fill))).with_children(|w| {
                                w.spawn((ImageNode::new(h), Node { width: percent(100), height: percent(100), ..default() }));
                            });
                        });
                    }
                    None => {
                        r.spawn(Node { width: px(34), height: px(34), ..default() });
                    }
                }
                r.spawn((Text::new(text), font(14.0), TextColor(colour)));
            });
    };
    let with_tier = |h: Handle<Image>, it: &lac_sim::item::Item| Some((h, tier_of(sim, it)));
    if let Some(it) = m.equip.get(&slot) {
        let ic = icon_handle(hud, images, sim, it, vision);
        row(p, with_tier(ic, it), format!("Take off {} (to the pack)", sim.item_name(it)), INK, PickFrom::TakeOff);
    }
    line(p, "From the pack", GOLD, 14.0);
    let mut any = false;
    for (k, it) in s.pack.iter().enumerate() {
        if fits(it) {
            any = true;
            let ic = icon_handle(hud, images, sim, it, vision);
            row(p, with_tier(ic, it), sim.item_name(it), INK, PickFrom::Pack(k));
        }
    }
    if !any {
        line(p, "  nothing that fits", DIM, 13.0);
    }
    if slot == Slot::Shield || slot == Slot::Weapon {
        // either hand can also hold a potion, scroll or wand, to use with a click (R63); a wand in the main hand
        // also fires when the member attacks (OURS)
        let tip = if slot == Slot::Weapon { "To hold in hand (click it to use it; a wand also fires when they attack)" } else { "To hold in hand (click it on the card to use it)" };
        line(p, tip, GOLD, 14.0);
        let mut any = false;
        for (k, it) in s.pack.iter().enumerate() {
            if lac_sim::State::holdable(&sim.content, it) {
                any = true;
                let ic = icon_handle(hud, images, sim, it, vision);
                row(p, with_tier(ic, it), sim.item_name(it), INK, PickFrom::Hold(k));
            }
        }
        if !any {
            line(p, "  no potions, scrolls or wands", DIM, 13.0);
        }
    }
    line(p, "Worn by the others", GOLD, 14.0);
    let mut any = false;
    for (j, other) in s.party.iter().enumerate() {
        if j == i {
            continue;
        }
        if let Some(it) = other.equip.get(&slot) {
            any = true;
            let ic = icon_handle(hud, images, sim, it, vision);
            row(p, with_tier(ic, it), format!("{} (from {})", sim.item_name(it), other.name), INK, PickFrom::Member(j));
        }
    }
    if !any {
        line(p, "  nobody else wears one", DIM, 13.0);
    }
    row(p, None, "Close".to_string(), DIM, PickFrom::Close);
}

fn line(p: &mut ChildSpawnerCommands, text: impl Into<String>, colour: Color, size: f32) {
    p.spawn((Text::new(text.into()), font(size), TextColor(colour)));
}

fn inventory(p: &mut ChildSpawnerCommands, sim: &Sim, member: usize, cursor: usize, vision: palette::Vision, images: &mut Assets<Image>) {
    let s = &sim.state;
    let c = &sim.content;
    let m = &s.party[member];
    line(p, format!("{} the {} - level {} ({} xp, next {})", m.name, m.class.name(), m.level, m.xp, lac_sim::party::xp_for_next(m.level)), GOLD, 16.0);
    line(
        p,
        format!(
            "STR {}  DEX {}  CON {}  INT {}  WIS {}   AC {}   pack {}/{}",
            m.attrs[0],
            m.attrs[1],
            m.attrs[2],
            m.attrs[3],
            m.attrs[4],
            m.ac(c),
            s.pack.len(),
            lac_sim::sim::PACK_SIZE
        ),
        DIM,
        13.0,
    );
    line(p, "Left/Right: member   Up/Down: choose   Enter: use, wear or take off   X: drop   G: pick up   R: swap rows", DIM, 12.0);
    let row = |p: &mut ChildSpawnerCommands, sel: bool, icon: Option<Handle<Image>>, text: String, colour: Color| {
        p.spawn((Node { flex_direction: FlexDirection::Row, column_gap: px(6), align_items: AlignItems::Center, ..default() }, BackgroundColor(if sel { Color::srgb(0.2, 0.17, 0.12) } else { Color::NONE })))
            .with_children(|r| {
                match icon {
                    Some(h) => {
                        r.spawn((ImageNode::new(h), Node { width: px(18), height: px(18), ..default() }));
                    }
                    None => {
                        r.spawn(Node { width: px(18), height: px(18), ..default() });
                    }
                }
                r.spawn((Text::new(text), font(13.0), TextColor(colour)));
            });
    };
    let icon_of = |it: &lac_sim::item::Item, images: &mut Assets<Image>| {
        let def = c.item(&it.def);
        let liquid = s.knowledge.looks.get(&it.def).map(|l| icons::look_colour(l)).unwrap_or((ix(RED, 4), ix(RED, 6)));
        images.add(icons::icon_image(&icons::icon(&def.icon, liquid), vision))
    };
    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(24), flex_grow: 1.0, min_height: px(0), ..default() }).with_children(|cols| {
        cols.spawn(Node { flex_direction: FlexDirection::Column, width: percent(38), ..default() }).with_children(|eq| {
            line(eq, "Worn", GOLD, 14.0);
            for (k, slot) in Slot::ALL.iter().enumerate() {
                match m.equip.get(slot) {
                    Some(it) => {
                        let ic = icon_of(it, images);
                        row(eq, cursor == k, Some(ic), format!("{:<7} {}", slot.name(), sim.item_name(it)), INK);
                    }
                    None => row(eq, cursor == k, None, format!("{:<7} -", slot.name()), DIM),
                }
            }
            let (to_hit, dmg, bonus, reach, ranged) = m.attack_profile(c);
            let kind = if ranged { "ranged" } else if reach { "reach" } else { "melee" };
            line(eq, format!("attack +{to_hit}, {dmg}{bonus:+} ({kind})"), DIM, 12.0);
            if member >= 2 && !reach && !ranged {
                line(eq, "In the back row, only reach or ranged weapons strike.", Color::srgb(0.85, 0.55, 0.4), 12.0);
            }
        });
        cols.spawn(Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, overflow: Overflow::clip(), ..default() }).with_children(|pk| {
            line(pk, format!("Pack (gold {})", s.gold), GOLD, 14.0);
            if s.pack.is_empty() {
                line(pk, "empty", DIM, 13.0);
            }
            // keep the cursor in view
            let first = cursor.saturating_sub(7).saturating_sub(14);
            for (k, it) in s.pack.iter().enumerate().skip(first).take(18) {
                let ic = icon_of(it, images);
                let def = c.item(&it.def);
                let note = match &def.kind {
                    ItemKind::Potion { .. } => " (drink)",
                    ItemKind::Scroll { .. } => " (read)",
                    ItemKind::Wand { .. } => " (zap ahead)",
                    ItemKind::Weapon { .. } | ItemKind::Armor { .. } | ItemKind::Ring { .. } | ItemKind::Amulet { .. } => " (wear)",
                    _ => "",
                };
                row(pk, cursor == 7 + k, Some(ic), format!("{}{note}", sim.item_name(it)), INK);
            }
        });
    });
}

fn spells(p: &mut ChildSpawnerCommands, sim: &Sim) {
    line(p, "Spells - press a number to cast, Esc to close", GOLD, 16.0);
    let list = spell_list(sim);
    if list.is_empty() {
        line(p, "Nobody here can cast anything right now.", DIM, 14.0);
    }
    for (k, (m, sp)) in list.iter().enumerate() {
        let mem = &sim.state.party[*m];
        let colour = if mem.mp >= sp.cost() { INK } else { DIM };
        let what = match sp {
            lac_sim::party::Spell::Spark => "a bolt at the first foe ahead",
            lac_sim::party::Spell::Fireball => "fire on the foe ahead and those beside it",
            lac_sim::party::Spell::Quicken => "the party acts twice as often for a while",
            lac_sim::party::Spell::Mend => "heal the most hurt member",
            lac_sim::party::Spell::Purify => "cure everyone's poison",
            lac_sim::party::Spell::Rebuke => "burn every undead within three tiles",
            lac_sim::party::Spell::Raise => "the fallen stand up again",
        };
        line(p, format!("{}. {} - {} ({} MP): {what}", k + 1, mem.name, sp.name(), sp.cost()), colour, 14.0);
    }
    line(p, "Mages learn Fireball at level 3 and Quicken at 5; priests Purify at 2, Rebuke at 3 and Raise at 5.", DIM, 12.0);
}

fn help(p: &mut ChildSpawnerCommands, settings: &Settings) {
    line(p, format!("How to play  (close: {})", settings.describe(Command::Help)), GOLD, 18.0);
    for t in [
        "Lead four adventurers down eight levels and open the great chest at the bottom.",
        "Time stands still until you act. Queue several moves and they play back faster.",
        "Jump goes up one tile and hangs for a moment: step right after it to land on a ledge.",
        "Long jump crosses a one-tile gap. Two-tile chasms need the bridge lever nearby.",
        "Use faces what is in front: levers, doors, chests, fountains, stairs. Use a wall to search it.",
        "Attack: the front row (members 1 and 2) strikes; the back row only with spears, slings and bows.",
        "Potions, scrolls, wands and rings are unknown until tried. Some gear is cursed and won't come off.",
        "Locked doors and chests take a key; the rogue can pick locks and find traps.",
        "The map on the right turns with you: up is always forward. The big map has north up.",
    ] {
        line(p, t, INK, 14.0);
    }
    line(p, "Keys", GOLD, 15.0);
    let mut row = String::new();
    for (k, c) in Command::ALL.iter().enumerate() {
        row.push_str(&format!("{:<14} {:<14}", c.name(), settings.describe(*c)));
        if k % 3 == 2 {
            line(p, std::mem::take(&mut row), DIM, 13.0);
        }
    }
    if !row.is_empty() {
        line(p, row, DIM, 13.0);
    }
    line(p, "1-4 pick a member (inventory, spells). Change keys in Options > Key bindings.", DIM, 13.0);
}

fn pause_menu(p: &mut ChildSpawnerCommands, cursor: usize) {
    line(p, "Paused", GOLD, 20.0);
    for (k, t) in ["Resume", "Save and quit to title", "Abandon this run"].iter().enumerate() {
        line(p, format!("{} {t}", if k == cursor { ">" } else { " " }), if k == cursor { GOLD } else { INK }, 16.0);
    }
    line(p, "Saving keeps one save per run; it is deleted when the run ends.", DIM, 12.0);
}

/// The automap (R35): seen columns, by height; what stands in them; the party.
fn draw_map(img: &mut Image, sim: &Sim, vision: palette::Vision, big: bool) {
    let s = &sim.state;
    let lvl = &s.level;
    let cols = palette::colours(vision);
    let col = |i: u8| {
        let c = cols[i as usize];
        [(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 255]
    };
    let Some(data) = img.data.as_mut() else { return };
    let size = lvl.world.size;
    // the small map is centred on the party and turns with it, so forward is always up; the big one shows the
    // whole level with north up
    let n = if big { MAP } else { SMALL_MAP };
    let half = n as i32 / 2;
    let up = s.facing.delta();
    let right = s.facing.cw().delta();
    let to_world = |mx: i32, my: i32| -> (i32, i32) {
        if big {
            (mx, my)
        } else {
            let (sx, sy) = (mx - half, my - half);
            (s.pos.x + right.x * sx - up.x * sy, s.pos.z + right.z * sx - up.z * sy)
        }
    };
    let to_map = |x: i32, z: i32| -> (i32, i32) {
        if big {
            (x, z)
        } else {
            let (dx, dz) = (x - s.pos.x, z - s.pos.z);
            // invert to_world: sx = dot(d, right), sy = -dot(d, up)
            (half + dx * right.x + dz * right.z, half - (dx * up.x + dz * up.z))
        }
    };
    for my in 0..n {
        for mx in 0..n {
            let (x, z) = to_world(mx as i32, my as i32);
            let mut c = [6, 5, 7, 255];
            if x >= 0 && z >= 0 && x < size.x && z < size.z && lvl.explored.contains(&(x, z)) {
                match lvl.ground_below(x, z, (s.pos.y + 2).min(size.y - 2)) {
                    Some(y) => {
                        let p = SIVec3::new(x, y, z);
                        let cell = lvl.world.get(p);
                        let below = lvl.world.get(p.down());
                        c = if cell == Cell::Water || below == Cell::Water {
                            col(ix(BLUE, 4))
                        } else if cell == Cell::Lava || below == Cell::Lava {
                            col(ix(FIRE, 5))
                        } else {
                            let shade = (3 + y - s.pos.y).clamp(1, 6) as u8;
                            col(ix(GREY, shade))
                        };
                        for fy in y - 1..=y + 1 {
                            if let Some(f) = lvl.features.get(&SIVec3::new(x, fy, z)) {
                                c = match f {
                                    Feature::Stairs { .. } => col(ix(FIRE, 7)),
                                    Feature::Door { portcullis: true, .. } => col(ix(GREY, 6)),
                                    Feature::Door { .. } => col(ix(BROWN, 4)),
                                    Feature::Chest { open: false, .. } => col(ix(FIRE, 6)),
                                    Feature::Fountain { .. } => col(ix(BLUE, 6)),
                                    Feature::Spikes | Feature::Plate { .. } => col(ix(RED, 4)),
                                    Feature::Tomb => col(ix(BONE, 4)),
                                    _ => c,
                                };
                            }
                        }
                        if lvl.items.contains_key(&p) {
                            c = col(ix(VIOLET, 6));
                        }
                    }
                    None => c = col(ix(BROWN, 1)),
                }
            }
            let k = (my * n + mx) * 4;
            data[k..k + 4].copy_from_slice(&c);
        }
    }
    // the player's marks (R60)
    for (&(x, z), &mark) in &lvl.notes {
        let (mx, my) = to_map(x, z);
        if (0..n as i32).contains(&mx) && (0..n as i32).contains(&my) {
            let c = match mark {
                1 => col(ix(FIRE, 7)),
                2 => col(ix(RED, 5)),
                3 => col(ix(BLUE, 6)),
                _ => col(ix(GREEN, 6)),
            };
            let k = (my as usize * n + mx as usize) * 4;
            data[k..k + 4].copy_from_slice(&c);
        }
    }
    // monsters the party can see right now
    for m in &lvl.monsters {
        if m.hp <= 0 || m.pos.flat_dist(s.pos) > 7 || !lac_sim::monster::line_of_sight(lvl, s.pos, m.pos) {
            continue;
        }
        let (mx, my) = to_map(m.pos.x, m.pos.z);
        if (0..n as i32).contains(&mx) && (0..n as i32).contains(&my) {
            let k = (my as usize * n + mx as usize) * 4;
            data[k..k + 4].copy_from_slice(&col(ix(RED, 6)));
        }
    }
    let (px_, pz) = to_map(s.pos.x, s.pos.z);
    let d = s.facing.delta();
    let (fx, fz) = to_map(s.pos.x + d.x, s.pos.z + d.z);
    for (x, z, c) in [(px_, pz, [255, 255, 255, 255]), (fx, fz, col(ix(BONE, 5)))] {
        if (0..n as i32).contains(&x) && (0..n as i32).contains(&z) {
            let k = (z as usize * n + x as usize) * 4;
            data[k..k + 4].copy_from_slice(&c);
        }
    }
}
