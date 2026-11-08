//! Item icons (R34: the inventory shows item icons) and character portraits (R30, R32; board row C7), drawn in
//! code as palette pixel art. All ours: the record's portraits and icons were placeholders we never copy.

use crate::palette::{self, ix, Vision, BLUE, BONE, BROWN, FIRE, GREEN, GREY, RED, VIOLET};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use lac_sim::party::Class;

pub const ICON: usize = 12;

#[derive(Clone)]
pub struct Icon {
    pub size: usize,
    pub px: Vec<Option<u8>>,
}

fn art(name: &str) -> [&'static str; 12] {
    match name {
        "sword" => [
            "..........kk", ".........kSk", "........kSsk", ".......kSsk.", "......kSsk..", ".....kSsk...", "..kk.Ssk....", "..kyyk......",
            "...yy.......", "..wkyy......", ".wk..yk.....", "kw..........",
        ],
        "dagger" => [
            "............", "............", "..........k.", ".........kSk", "........kSk.", ".......kSk..", "...kk.kSk...", "...kyysk....",
            "....yy......", "...wkyy.....", "..wk..y.....", "............",
        ],
        "mace" => [
            "........k.k.", ".......kgggk", "......kgSSgk", ".......gSgg.", "......kgggk.", ".....wk.k...", "....wk......", "...wk.......",
            "..wk........", ".wk.........", "wk..........", "............",
        ],
        "axe" => [
            ".......kk...", "......kSsk..", ".....kSssgk.", "......wsggk.", ".....wk.kk..", "....wk......", "...wk.......", "..wk........",
            ".wk.........", "wk..........", "............", "............",
        ],
        "spear" => [
            "...........k", "..........kS", ".........kSk", "........kwk.", ".......kwk..", "......kwk...", ".....kwk....", "....kwk.....",
            "...kwk......", "..kwk.......", ".kwk........", "kwk.........",
        ],
        "bow" => [
            "...kWWk.....", "..kW..pk....", ".kW....p....", ".kW.....p...", "kW......p...", "kW......p...", "kW......p...", "kW......p...",
            ".kW.....p...", ".kW....p....", "..kW..pk....", "...kWWk.....",
        ],
        "sling" => [
            "............", "..l.........", "..l.........", "...l........", "...l........", "....l.......", ".....l...l..", "......lgl...",
            ".....lggl...", "......ll....", "............", "............",
        ],
        "staff" => [
            ".........vv.", "........vVVv", "........vVVv", ".........vv.", "........w...", ".......w....", "......w.....", ".....w......",
            "....w.......", "...w........", "..w.........", ".w..........",
        ],
        "armor" => [
            "............", "..kk....kk..", ".kgsk..ksgk.", ".kgsskkssgk.", "..kgssssgk..", "..kgssssgk..", "..kgsSSsgk..", "..kgssssgk..",
            "..kgssssgk..", "..kggggggk..", "...kkkkkk...", "............",
        ],
        "robe" => [
            "............", "...kk..kk...", "..kvvkkvvk..", "..kvvvvvvk..", "...kvvvvk...", "...kvvvvk...", "..kvvvvvvk..", "..kvvyvvvk..",
            ".kvvvyvvvvk.", ".kvvvvvvvvk.", ".kkkkkkkkkk.", "............",
        ],
        "shield" => [
            "............", ".kkkkkkkkkk.", ".kWWWyyWWWk.", ".kWWWyyWWWk.", ".kyyyyyyyyk.", ".kWWWyyWWWk.", ".kWWWyyWWWk.", "..kWWyyWWk..",
            "..kWWyyWWk..", "...kWyyWk...", "....kkkk....", "............",
        ],
        "helm" => [
            "............", "............", "....kkkk....", "...kgSSgk...", "..kgSssggk..", "..kgsssggk..", "..kgkkkkgk..", "..kgk..kgk..",
            "..kgk..kgk..", "..kk....kk..", "............", "............",
        ],
        "boots" => [
            "............", "............", "..kkk.......", "..klk..kkk..", "..klk..klk..", "..klk..klk..", "..klk..klk..", "..klkk.klkk.",
            "..kllLk.kllLk", "..kkkkk.kkkk", "............", "............",
        ],
        "ring" => [
            "............", "............", ".....kk.....", "....keek....", ".....kk.....", "....yyyy....", "...yk..ky...", "...y....y...",
            "...y....y...", "...yk..ky...", "....yyyy....", "............",
        ],
        "amulet" => [
            "..y......y..", "...y....y...", "....y..y....", ".....yy.....", ".....yy.....", "....kyyk....", "...keeeek...", "...keEEek...",
            "...keeeek...", "....keek....", ".....kk.....", "............",
        ],
        "potion" => [
            "............", ".....kk.....", ".....ww.....", ".....kk.....", "....k..k....", "...k....k...", "..kCccccck..", "..kCccccck..",
            "..kccccccck.", "..kccccccck.", "...kkkkkk...", "............",
        ],
        "scroll" => [
            "............", "..kkkkkkkk..", ".kpPppppppk.", "..kpppppppk.", "..kpPPPpPpk.", "..kpppppppk.", "..kpPPpPPpk.", "..kpppppppk.",
            "..kpPpPPPpk.", "..kppppppPpk", "...kkkkkkkk.", "............",
        ],
        "wand" => [
            "..........e.", ".........eEe", "..........e.", ".........w..", "........w...", ".......w....", "......w.....", ".....w......",
            "....w.......", "...w........", "..k.........", "............",
        ],
        "key" => [
            "............", "...kkk......", "..kyyyk.....", "..ky.yk.....", "..kyyyk.....", "...kyk......", "....yk......", "....yyk.....",
            "....yk......", "....yyk.....", ".....k......", "............",
        ],
        "gold" => [
            "............", "............", "............", "......kk....", ".....kyyk...", "...kkyYyk...", "..kyykyyk...", ".kyYyykkkk..",
            ".kyyykyyyyk.", "..kkkkyYyyk.", "......kkkk..", "............",
        ],
        // dim glyphs for empty slots
        "hand" => [
            "............", "......k.....", "...k.kgk....", "..kgkkgk.k..", "..kgkgkgkgk.", "..kgkgkgkgk.", "..kgggggggk.", ".k.kgggggk..",
            ".kgkgggggk..", "..kggggggk..", "...kggggk...", "....kkkk....",
        ],
        "gem" => [
            "............", "............", "...kkkkkk...", "..kSsSsSsk..", ".kSsssssssk.", ".ksssssssk..", "..kssssssk..", "...kssssk...",
            "....kssk....", ".....kk.....", "............", "............",
        ],
        "crown" => [
            "............", "............", "..y..y..y...", "..yy.yy.yy..", "..yyyyyyyy..", "..yeyyeyyy..", "..yyyyyyyy..", "..YYYYYYYY..",
            "............", "............", "............", "............",
        ],
        _ => [
            "............", "....kkkk....", "...k....k...", "........k...", ".......k....", "......k.....", "......k.....", "............",
            "......k.....", "............", "............", "............",
        ],
    }
}

/// The colour of an unidentified look (potion liquid, ring gem): from the look's name, so each look keeps
/// its colour for the run.
pub fn look_colour(look: &str) -> (u8, u8) {
    let words: [(&str, u8); 18] = [
        ("murky", BROWN), ("amber", FIRE), ("violet", VIOLET), ("milky", BONE), ("smoky", GREY), ("crimson", RED),
        ("glowing", GREEN), ("oily", GREY), ("teal", BLUE), ("bubbling", GREEN), ("gritty", BROWN), ("fizzing", BLUE),
        ("opal", BONE), ("jade", GREEN), ("ruby", RED), ("garnet", RED), ("moonstone", BLUE), ("star", FIRE),
    ];
    let ramp = words.iter().find(|(w, _)| look.contains(w)).map(|(_, r)| *r).unwrap_or_else(|| (lac_sim::rng::fnv1a64(look.as_bytes()) % 8) as u8);
    (ix(ramp, 4), ix(ramp, 6))
}

pub fn icon(name: &str, liquid: (u8, u8)) -> Icon {
    let rows = art(name);
    let mut px = Vec::with_capacity(ICON * ICON);
    for row in rows {
        let mut chars: Vec<char> = row.chars().collect();
        chars.resize(ICON, '.');
        for ch in chars.into_iter().take(ICON) {
            px.push(match ch {
                'k' => Some(ix(GREY, 0)),
                's' => Some(ix(GREY, 5)),
                'S' => Some(ix(GREY, 7)),
                'g' => Some(ix(GREY, 3)),
                'w' => Some(ix(BROWN, 3)),
                'W' => Some(ix(BROWN, 2)),
                'l' => Some(ix(BROWN, 2)),
                'L' => Some(ix(BROWN, 4)),
                'y' => Some(ix(FIRE, 6)),
                'Y' => Some(ix(FIRE, 4)),
                'r' => Some(ix(RED, 4)),
                'b' => Some(ix(BONE, 6)),
                'p' => Some(ix(BONE, 6)),
                'P' => Some(ix(BONE, 3)),
                'v' => Some(ix(VIOLET, 3)),
                'V' => Some(ix(VIOLET, 6)),
                'c' | 'e' => Some(liquid.0),
                'C' | 'E' => Some(liquid.1),
                _ => None,
            });
        }
    }
    Icon { size: ICON, px }
}

fn rgba_image(w: usize, h: usize, px: &[Option<u8>], vision: Vision, bg: Option<u8>) -> Image {
    let cols = palette::colours(vision);
    let mut data = Vec::with_capacity(w * h * 4);
    for p in px {
        match p.or(bg) {
            Some(i) => {
                let c = cols[i as usize];
                data.extend_from_slice(&[(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 255]);
            }
            None => data.extend_from_slice(&[0, 0, 0, 0]),
        }
    }
    let mut img = Image::new(
        Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    img
}

pub fn icon_image(icon: &Icon, vision: Vision) -> Image {
    rgba_image(icon.size, icon.size, &icon.px, vision, None)
}

pub const PORTRAIT: usize = 24;

/// A portrait: shoulders, face and head-gear by class. OURS (board row C7).
pub fn portrait(class: Class, which: u32, vision: Vision) -> Image {
    let n = PORTRAIT as i32;
    let mut px: Vec<Option<u8>> = vec![None; PORTRAIT * PORTRAIT];
    let mut put = |x: i32, y: i32, c: u8| {
        if (0..n).contains(&x) && (0..n).contains(&y) {
            px[(y * n + x) as usize] = Some(c);
        }
    };
    let (bg, cloth, skin, hair) = match class {
        Class::Fighter => (ix(RED, 1), ix(GREY, 4), ix(BONE, 5), ix(BROWN, 3)),
        Class::Rogue => (ix(GREEN, 0), ix(GREEN, 2), ix(BONE, 4), ix(GREY, 1)),
        Class::Priest => (ix(BLUE, 1), ix(BONE, 6), ix(BROWN, 5), ix(GREY, 6)),
        Class::Mage => (ix(VIOLET, 0), ix(VIOLET, 3), ix(BONE, 5), ix(GREY, 5)),
    };
    let _ = which;
    for y in 0..n {
        for x in 0..n {
            // a vignette background
            let edge = x.min(y).min(n - 1 - x).min(n - 1 - y);
            put(x, y, if edge < 2 { bg - 1.min(bg % 8) } else { bg });
        }
    }
    // shoulders
    for y in 18..n {
        let half = 6 + (y - 18);
        for x in 12 - half..12 + half {
            put(x, y, cloth);
        }
    }
    // neck and face
    for y in 15..19 {
        for x in 10..14 {
            put(x, y, skin - 1);
        }
    }
    for y in 6..17 {
        let half = if y < 8 || y > 14 { 3 } else { 4 };
        for x in 12 - half..12 + half {
            put(x, y, skin);
        }
    }
    // eyes, nose, mouth
    put(10, 10, ix(GREY, 0));
    put(13, 10, ix(GREY, 0));
    put(9, 9, skin - 2);
    put(14, 9, skin - 2);
    put(11, 12, skin - 1);
    put(12, 12, skin - 1);
    for x in 10..14 {
        put(x, 14, skin - 2);
    }
    match class {
        Class::Fighter => {
            // a steel helm with a nose guard, and a beard
            for y in 3..9 {
                for x in 7..17 {
                    put(x, y, if y == 8 { ix(GREY, 3) } else { ix(GREY, 5) });
                }
            }
            for y in 8..12 {
                put(11, y, ix(GREY, 4));
                put(12, y, ix(GREY, 4));
            }
            for y in 13..18 {
                for x in 8..16 {
                    if (x + y) % 3 != 0 || y > 14 {
                        put(x, y, hair);
                    }
                }
            }
            put(10, 14, ix(RED, 3));
            put(13, 14, ix(RED, 3));
        }
        Class::Rogue => {
            // a deep hood and a mask over the mouth
            for y in 2..18 {
                for x in 5..19 {
                    let inside = y >= 6 && x >= 8 && x < 16 && y < 18;
                    if !inside {
                        put(x, y, ix(GREEN, 2));
                    }
                }
            }
            for y in 12..17 {
                for x in 8..16 {
                    put(x, y, ix(GREY, 1));
                }
            }
            put(10, 10, ix(GREEN, 6));
            put(13, 10, ix(GREEN, 6));
        }
        Class::Priest => {
            // a pale hood and a gold circlet
            for y in 2..18 {
                for x in 6..18 {
                    let inside = y >= 6 && x >= 8 && x < 16;
                    if !inside {
                        put(x, y, ix(BONE, 6));
                    }
                }
            }
            for x in 8..16 {
                put(x, 6, ix(FIRE, 6));
            }
            put(12, 5, ix(BLUE, 6));
            for y in 19..n {
                put(12, y, ix(FIRE, 6));
            }
        }
        Class::Mage => {
            // a tall hat and a long grey beard
            for y in 0..7 {
                let half = 1 + y;
                for x in 12 - half..12 + half {
                    put(x + (6 - y) / 3, y, ix(VIOLET, 3));
                }
            }
            for x in 5..19 {
                put(x, 7, ix(VIOLET, 2));
            }
            for y in 13..21 {
                let half = 4 - (y - 13) / 3;
                for x in 12 - half..12 + half {
                    put(x, y, hair);
                }
            }
            put(10, 10, ix(BLUE, 6));
            put(13, 10, ix(BLUE, 6));
        }
    }
    rgba_image(PORTRAIT, PORTRAIT, &px, vision, None)
}

/// The splash screen's title in an LCD look (REF R55: a stylised, pixelated LCD-display splash). Each letter
/// is a 5×7 dot matrix; lit dots are bright, unlit ones faintly visible, as on a real LCD.
pub fn lcd_text(text: &str, vision: Vision) -> Image {
    const FONT: &[(char, [u8; 7])] = &[
        ('A', [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11]),
        ('C', [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E]),
        ('D', [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E]),
        ('E', [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F]),
        ('H', [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11]),
        ('L', [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F]),
        ('N', [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11]),
        ('R', [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11]),
        ('S', [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E]),
        ('T', [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04]),
        ('V', [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04]),
        (' ', [0; 7]),
    ];
    let cols = text.chars().count();
    let (cw, chh) = (6usize, 9usize);
    let w = cols * cw + 3;
    let hh = chh + 2;
    let mut px = vec![Some(ix(GREEN, 0)); w * hh];
    for (k, ch) in text.chars().enumerate() {
        let glyph = FONT.iter().find(|(c, _)| *c == ch).map(|(_, g)| *g).unwrap_or([0; 7]);
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5 {
                let on = bits & (0x10 >> col) != 0;
                let x = 2 + k * cw + col;
                let y = 2 + row;
                px[y * w + x] = Some(if on { ix(GREEN, 7) } else { ix(GREEN, 1) });
            }
        }
    }
    rgba_image(w, hh, &px, vision, None)
}
