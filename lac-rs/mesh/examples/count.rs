//! How heavy is a level's mesh? `cargo run --release -p lac-mesh --example count -- <seed> <depth>`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed = lac_sim::rng::seed_from_text(a.get(1).map(String::as_str).unwrap_or("HELLO"));
    let depth: u8 = a.get(2).and_then(|d| d.parse().ok()).unwrap_or(1);
    let c = lac_sim::Content::builtin();
    let l = lac_sim::levelgen::generate(&c, seed, depth, lac_sim::RunOptions::default());
    let tex = lac_mesh::textures::all_textures();
    let t0 = std::time::Instant::now();
    let (mut quads, mut verts) = (0, 0);
    for k in l.world.chunk_keys() {
        let m = lac_mesh::chunk_mesh(&l.world, k, &tex);
        quads += m.idx.len() / 6;
        verts += m.pos.len();
    }
    println!("depth {depth}: {quads} quads, {verts} vertices, {:?}", t0.elapsed());
}
