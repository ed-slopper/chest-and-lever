//! Spots to look at torches and tall doors from, for screenshots: `cargo run -p lac-sim --example views -- <seed> [depth]`.
//! Prints `--at x,y,z,Facing` arguments for the app, one per feature.
use lac_sim::{levelgen, Content, Dir, IVec3, RunOptions, Sim};
use lac_sim::world::Feature;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let sim = Sim::new(&a[1], RunOptions::default(), Content::builtin());
    let depth: u8 = a.get(2).and_then(|d| d.parse().ok()).unwrap_or(1);
    let l = if depth == 1 { sim.state.level.clone() } else { levelgen::generate(&sim.content, sim.state.seed, depth, sim.state.options) };
    let stand = |q: IVec3| !l.world.solid(q) && l.world.solid(q.down()) && !l.features.contains_key(&q);
    for (p, f) in &l.features {
        match f {
            Feature::Torch { face } => {
                // the torch hangs on wall cell p, facing the open side: stand two tiles out, looking back at it
                // torches hang a tile above the floor the party stands on
                let q = *p + face.delta() * 2;
                let q = if stand(q) { q } else { q.down() };
                // one tile out if two is blocked
                let near = *p + face.delta();
                let near = if stand(near) { near } else { near.down() };
                let at = if stand(q) && !l.world.solid(near) { Some(q) } else if stand(near) { Some(near) } else { None };
                match at {
                    Some(q) => println!("torch {p:?}: --at {},{},{},{:?}", q.x, q.y, q.z, face.opposite()),
                    None => println!("torch {p:?}: no spot ({:?} below {:?})", l.world.get(near), l.world.get(near.down())),
                }
            }
            Feature::Door { .. } if !l.world.solid(p.up()) => {
                for d in Dir::ALL {
                    let q = *p - d.delta() * 2;
                    if stand(q) && !l.world.solid(*p - d.delta()) && !l.world.solid(*p + d.delta()) {
                        println!("tall door {p:?}: --at {},{},{},{:?}", q.x, q.y, q.z, d);
                        break;
                    }
                }
            }
            _ => {}
        }
    }
}
