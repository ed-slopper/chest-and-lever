//! Print a generated level from above: `cargo run -p lac-sim --example map -- <seed> <depth>`.
//! Digits are floor heights, `#` rock; S start, > stairs, C chest, D door, P portcullis, L lever, ? secret,
//! F fountain, ^ spikes or plate, ~ water, % lava, m monster, * items, X the great chest.
use lac_sim::world::Feature;
use lac_sim::{levelgen, Cell, Content, IVec3, RunOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed = lac_sim::rng::seed_from_text(args.get(1).map(String::as_str).unwrap_or("MAP"));
    let depth: u8 = args.get(2).and_then(|d| d.parse().ok()).unwrap_or(1);
    let c = Content::builtin();
    let l = levelgen::generate(&c, seed, depth, RunOptions::default());
    println!("depth {depth} {:?}: {} monsters, {} features", l.theme, l.monsters.len(), l.features.len());
    for z in 0..l.world.size.z {
        let mut row = String::new();
        for x in 0..l.world.size.x {
            let ground = l.ground_below(x, z, l.world.size.y - 2);
            let mut ch = match ground {
                Some(y) => {
                    let p = IVec3::new(x, y, z);
                    match l.world.get(p) {
                        Cell::Water => '~',
                        Cell::Lava => '%',
                        _ => char::from_digit(y as u32 % 10, 10).unwrap(),
                    }
                }
                None => '#',
            };
            for y in 0..l.world.size.y {
                let p = IVec3::new(x, y, z);
                if let Some(f) = l.features.get(&p) {
                    ch = match f {
                        Feature::Door { portcullis: true, .. } => 'P',
                        Feature::Door { .. } => 'D',
                        Feature::Lever { .. } => 'L',
                        Feature::Plate { .. } | Feature::Spikes => '^',
                        Feature::Chest { great: true, .. } => 'X',
                        Feature::Chest { .. } => 'C',
                        Feature::Stairs { .. } => '>',
                        Feature::Fountain { .. } => 'F',
                        Feature::Tomb => 'T',
                        Feature::SecretWall => '?',
                        Feature::Torch { .. } => ch,
                    };
                }
                if l.items.contains_key(&p) {
                    ch = '*';
                }
                if l.monster_at(p).is_some() {
                    ch = 'm';
                }
            }
            if x == l.start.x && z == l.start.z {
                ch = 'S';
            }
            row.push(ch);
        }
        println!("{row}");
    }
}
