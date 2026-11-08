//! The HUD (board row X1) and the automap (G12). REF R30: the four portraits sit on the right of the screen.
//! REF R35: a minimap of what the party has seen. REF R34: an inventory with item icons. OURS: the layout,
//! and the overlays (inventory, spells, pause, map, debug: board row E11).

use crate::game::{spell_list, Game, Overlay, ViewImage};
use crate::icons;
use crate::palette::{self, ix, BLUE, BONE, BROWN, FIRE, GREY, RED, VIOLET};
use crate::settings::{Command, Settings};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::FontSize;
use lac_sim::content::{ItemKind, Slot};
use lac_sim::world::Feature;
use lac_sim::{Cell, IVec3 as SIVec3, Sim};

pub const LOG_LINES: usize = 6;
pub const MAP: usize = 48;

#[derive(Component)]
pub struct HudRoot;
#[derive(Component)]
pub struct TopBar;
#[derive(Component)]
pub struct LogText(pub usize);
#[derive(Component)]
pub struct Card(pub usize);
#[derive(Component)]
pub struct CardPortrait(pub usize);
#[derive(Component)]
pub struct CardName(pub usize);
#[derive(Component)]
pub struct CardStatus(pub usize);
#[derive(Component)]
pub struct HpFill(pub usize);
#[derive(Component)]
pub struct MpFill(pub usize);
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

#[derive(Resource)]
pub struct HudImages {
    pub map: Handle<Image>,
    pub big_map: Handle<Image>,
    pub portraits: Vec<Handle<Image>>,
}

pub fn font(px: f32) -> TextFont {
    TextFont { font_size: FontSize::Px(px), ..default() }
}

pub const INK: Color = Color::srgb(0.86, 0.82, 0.72);
pub const DIM: Color = Color::srgb(0.55, 0.52, 0.47);
pub const PANEL: Color = Color::srgb(0.07, 0.065, 0.075);
pub const GOLD: Color = Color::srgb(0.95, 0.78, 0.35);

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

pub fn spawn_hud(mut commands: Commands, mut images: ResMut<Assets<Image>>, view: Res<ViewImage>, game: Res<Game>, settings: Res<Settings>) {
    let vision = settings.file.vision;
    let map = images.add(blank(MAP, MAP));
    let big_map = images.add(blank(MAP, MAP));
    let portraits: Vec<Handle<Image>> = game.sim.state.party.iter().map(|m| images.add(icons::portrait(m.class, m.portrait, vision))).collect();
    commands.insert_resource(HudImages { map: map.clone(), big_map: big_map.clone(), portraits: portraits.clone() });

    commands
        .spawn((
            Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Row, padding: UiRect::all(px(8)), column_gap: px(8), ..default() },
            BackgroundColor(Color::srgb(0.02, 0.02, 0.025)),
            HudRoot,
        ))
        .with_children(|root| {
            // left: top bar, the view, the message log
            root.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, row_gap: px(6), min_width: px(0), ..default() }).with_children(|left| {
                left.spawn((Text::new(""), font(15.0), TextColor(GOLD), TopBar));
                left.spawn(Node { flex_grow: 1.0, min_height: px(0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, overflow: Overflow::clip(), ..default() })
                    .with_children(|vc| {
                        vc.spawn((
                            ImageNode::new(view.0.clone()),
                            Node { height: percent(100), max_width: percent(100), aspect_ratio: Some(16.0 / 9.0), ..default() },
                        ))
                        .with_children(|v| {
                            v.spawn((
                                Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() },
                                BackgroundColor(Color::NONE),
                                HurtFlash,
                            ));
                            v.spawn((
                                Node { position_type: PositionType::Absolute, left: px(8), top: px(8), ..default() },
                                Text::new(""),
                                font(13.0),
                                TextColor(Color::srgb(0.6, 1.0, 0.6)),
                                DebugText,
                            ));
                            v.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: percent(6),
                                    right: percent(6),
                                    top: percent(6),
                                    bottom: percent(6),
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::all(px(14)),
                                    row_gap: px(2),
                                    display: Display::None,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.04, 0.035, 0.045, 0.93)),
                                OverlayPanel,
                            ));
                        });
                    });
                left.spawn((Node { flex_direction: FlexDirection::Column, height: px(LOG_LINES as f32 * 18.0 + 8.0), padding: UiRect::all(px(4)), ..default() }, BackgroundColor(PANEL)))
                    .with_children(|log| {
                        for i in 0..LOG_LINES {
                            log.spawn((Text::new(""), font(14.0), TextColor(INK), LogText(i)));
                        }
                    });
            });
            // right: the party (R30), then the map
            root.spawn(Node { width: px(300), flex_direction: FlexDirection::Column, row_gap: px(6), ..default() }).with_children(|right| {
                for (i, p) in portraits.iter().enumerate() {
                    right
                        .spawn((Node { flex_direction: FlexDirection::Row, column_gap: px(8), padding: UiRect::all(px(5)), ..default() }, BackgroundColor(PANEL), Card(i)))
                        .with_children(|card| {
                            card.spawn((ImageNode::new(p.clone()), Node { width: px(66), height: px(66), ..default() }, CardPortrait(i)));
                            card.spawn(Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, row_gap: px(3), ..default() }).with_children(|col| {
                                col.spawn((Text::new(""), font(14.0), TextColor(INK), CardName(i)));
                                bar(col, Color::srgb(0.75, 0.16, 0.14), HpFill(i));
                                bar(col, Color::srgb(0.22, 0.38, 0.85), MpFill(i));
                                col.spawn((Text::new(""), font(12.0), TextColor(DIM), CardStatus(i)));
                            });
                        });
                }
                right.spawn((Node { padding: UiRect::all(px(6)), justify_content: JustifyContent::Center, ..default() }, BackgroundColor(PANEL))).with_children(|m| {
                    m.spawn((ImageNode::new(map.clone()), Node { width: px(192), height: px(192), ..default() }, Minimap));
                });
                right.spawn((Text::new(""), font(12.0), TextColor(DIM), Hints));
            });
        });
}

fn bar(parent: &mut ChildSpawnerCommands, colour: Color, marker: impl Component) {
    parent.spawn((Node { height: px(8), width: percent(100), ..default() }, BackgroundColor(Color::srgb(0.15, 0.13, 0.14)))).with_children(|b| {
        b.spawn((Node { height: percent(100), width: percent(100), ..default() }, BackgroundColor(colour), marker));
    });
}

pub fn despawn_hud(mut commands: Commands, q: Query<Entity, With<HudRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_hud(
    mut commands: Commands,
    mut game: ResMut<Game>,
    settings: Res<Settings>,
    hud: Res<HudImages>,
    mut images: ResMut<Assets<Image>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<TopBar>>,
        Query<(&mut Text, &mut TextColor, &LogText)>,
        Query<(&mut Text, &CardName)>,
        Query<(&mut Text, &CardStatus)>,
        Query<&mut Text, With<DebugText>>,
        Query<&mut Text, With<Hints>>,
    )>,
    mut fills: ParamSet<(Query<(&mut Node, &HpFill)>, Query<(&mut Node, &MpFill)>)>,
    mut cards: Query<(&Card, &mut BackgroundColor), Without<HurtFlash>>,
    mut portraits: Query<(&CardPortrait, &mut ImageNode)>,
    mut flash: Query<&mut BackgroundColor, With<HurtFlash>>,
    mut panel: Query<(Entity, &mut Node), (With<OverlayPanel>, Without<HpFill>, Without<MpFill>)>,
) {
    // always: flashes, the log fading, the debug overlay
    for mut bg in &mut flash {
        bg.0 = Color::srgba(0.8, 0.05, 0.02, game.hurt_flash * 0.35);
    }
    for (c, mut bg) in &mut cards {
        let f = game.member_flash[c.0];
        let sel = c.0 == game.selected;
        let base = if sel { Color::srgb(0.14, 0.12, 0.10) } else { PANEL };
        bg.0 = base.mix(&Color::srgb(0.55, 0.08, 0.05), f * 0.7);
    }
    {
        let s = &game.sim.state;
        let mut q = texts.p4();
        if let Ok(mut t) = q.single_mut() {
            t.0 = if game.debug {
                format!(
                    "seed {}  tick {}  queue {}\ntile ({}, {}, {}) facing {:?}\ndepth {}  monsters {}\nframe {:.1} ms  hash {:016x}",
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
    let sim = &game.sim;
    let s = &sim.state;
    {
        let mut q = texts.p0();
        if let Ok(mut t) = q.single_mut() {
            let mut fx = Vec::new();
            if s.haste > 0 {
                fx.push(format!("quick {}", s.haste));
            }
            if s.levitating(&sim.content) {
                fx.push("floating".to_string());
            }
            if s.airborne {
                fx.push("in the air".to_string());
            }
            t.0 = format!(
                "Depth {} - {}    Gold {}    Tick {}{}{}",
                s.level.depth,
                s.level.theme.name(),
                s.gold,
                s.tick,
                if fx.is_empty() { String::new() } else { format!("    [{}]", fx.join(", ")) },
                if game.sim.queued() > 0 { format!("    queued {}", game.sim.queued()) } else { String::new() }
            );
        }
    }
    {
        let mut q = texts.p2();
        for (mut t, n) in q.iter_mut() {
            if let Some(m) = s.party.get(n.0) {
                let row = if n.0 < 2 { "front" } else { "back" };
                t.0 = format!("{}. {}  {} {} ({row})", n.0 + 1, m.name, m.class.name(), m.level);
            }
        }
    }
    {
        let mut q = texts.p3();
        for (mut t, n) in q.iter_mut() {
            if let Some(m) = s.party.get(n.0) {
                t.0 = if !m.alive() {
                    "dead".into()
                } else {
                    let mut st = format!("HP {}/{}", m.hp, m.max_hp);
                    if m.max_mp > 0 {
                        st.push_str(&format!("  MP {}/{}", m.mp, m.max_mp));
                    }
                    st.push_str(&format!("  AC {}", m.ac(&sim.content)));
                    if m.poison > 0 {
                        st.push_str("  poisoned");
                    }
                    st
                };
            }
        }
    }
    {
        let mut q = texts.p5();
        if let Ok(mut t) = q.single_mut() {
            let k = |c: Command| settings.describe(c);
            t.0 = format!(
                "move {}/{}/{}/{}  turn {}/{}\njump {}  long jump {}\nuse {}  attack {}  wait {}\npick up {}  pack {}  spells {}\nmap {}  menu {}  members 1-4",
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
                k(Command::Menu),
            );
        }
    }
    {
        let mut q = fills.p0();
        for (mut node, f) in q.iter_mut() {
            if let Some(m) = s.party.get(f.0) {
                node.width = percent(100.0 * m.hp.max(0) as f32 / m.max_hp.max(1) as f32);
            }
        }
    }
    {
        let mut q = fills.p1();
        for (mut node, f) in q.iter_mut() {
            if let Some(m) = s.party.get(f.0) {
                node.width = percent(if m.max_mp > 0 { 100.0 * m.mp as f32 / m.max_mp as f32 } else { 0.0 });
            }
        }
    }
    for (p, mut img) in &mut portraits {
        if let Some(m) = s.party.get(p.0) {
            img.color = if m.alive() { Color::WHITE } else { Color::srgb(0.25, 0.2, 0.2) };
            // portraits follow the members when they swap places
            if let Some(h) = hud.portraits.get(m.portrait as usize) {
                img.image = h.clone();
            }
        }
    }
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
            let selected = game.selected;
            let cursor = game.cursor;
            let pause = game.pause_cursor;
            commands.entity(panel_e).with_children(|p| match game.overlay {
                Overlay::Inventory => inventory(p, sim, selected, cursor, vision, &mut images),
                Overlay::Spells => spells(p, sim),
                Overlay::Pause => pause_menu(p, pause),
                Overlay::Map => {
                    p.spawn((Text::new(format!("Map of depth {}  (close: {})", s.level.depth, settings.describe(Command::Map))), font(15.0), TextColor(GOLD)));
                    p.spawn(Node { flex_grow: 1.0, justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }).with_children(|c| {
                        c.spawn((ImageNode::new(hud.big_map.clone()), Node { height: percent(100), aspect_ratio: Some(1.0), ..default() }));
                    });
                }
                Overlay::None => {}
            });
        }
    }
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
    let half = MAP as i32 / 2;
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
    for my in 0..MAP {
        for mx in 0..MAP {
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
            let k = (my * MAP + mx) * 4;
            data[k..k + 4].copy_from_slice(&c);
        }
    }
    // monsters the party can see right now
    for m in &lvl.monsters {
        if m.hp <= 0 || m.pos.flat_dist(s.pos) > 7 || !lac_sim::monster::line_of_sight(lvl, s.pos, m.pos) {
            continue;
        }
        let (mx, my) = to_map(m.pos.x, m.pos.z);
        if (0..MAP as i32).contains(&mx) && (0..MAP as i32).contains(&my) {
            let k = (my as usize * MAP + mx as usize) * 4;
            data[k..k + 4].copy_from_slice(&col(ix(RED, 6)));
        }
    }
    let (px_, pz) = to_map(s.pos.x, s.pos.z);
    let d = s.facing.delta();
    let (fx, fz) = to_map(s.pos.x + d.x, s.pos.z + d.z);
    for (x, z, c) in [(px_, pz, [255, 255, 255, 255]), (fx, fz, col(ix(BONE, 5)))] {
        if (0..MAP as i32).contains(&x) && (0..MAP as i32).contains(&z) {
            let k = (z as usize * MAP + x as usize) * 4;
            data[k..k + 4].copy_from_slice(&c);
        }
    }
}
