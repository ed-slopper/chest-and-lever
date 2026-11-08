//! How much of the first level is on the automap at the start? `cargo run -p lac-sim --example explored -- <seed>`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let sim = lac_sim::Sim::new(a.get(1).map(String::as_str).unwrap_or("AMBER-DOOR-42"), lac_sim::RunOptions::default(), lac_sim::Content::builtin());
    let s = &sim.state;
    println!("pos {:?} facing {:?}, explored {}", s.pos, s.facing, s.level.explored.len());
    for dz in -8..=8 {
        let mut row = String::new();
        for dx in -8..=8 {
            let (x, z) = (s.pos.x + dx, s.pos.z + dz);
            row.push(if (dx, dz) == (0, 0) { '@' } else if s.level.explored.contains(&(x, z)) { '.' } else { ' ' });
        }
        println!("{row}");
    }
}
