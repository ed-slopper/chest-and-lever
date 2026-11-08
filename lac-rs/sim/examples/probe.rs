//! Print what is around a tile of a run's first level: `cargo run -p lac-sim --example probe -- <seed> x y z`.
use lac_sim::{Content, IVec3, RunOptions, Sim};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let sim = Sim::new(&a[1], RunOptions::default(), Content::builtin());
    let p = IVec3::new(a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap());
    let l = &sim.state.level;
    for dy in (-2..=2).rev() {
        for dx in -1..=3 {
            let q = p + IVec3::new(dx, dy, 0);
            println!("{q:?}: {:?} {:?} monster {:?}", l.world.get(q), l.features.get(&q), l.monster_at(q).map(|i| &l.monsters[i].kind));
        }
    }
}
