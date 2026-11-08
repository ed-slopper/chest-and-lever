//! The replay test (board row F4): a run is a seed plus a list of actions. Each recording in `runs/` is played
//! through the sim and the hash of the whole state compared with the one it was recorded with. Same seed + same
//! actions = the same run, bit for bit (AGENTS.md section 13).
//!
//! A sim change that changes behaviour on purpose updates the hashes: run with `LAC_BLESS=1` and say why in the
//! commit.

use lac_sim::{Content, Recording};
use std::path::PathBuf;

#[test]
fn recordings_replay_to_their_hash() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("runs");
    let bless = std::env::var("LAC_BLESS").is_ok_and(|v| v == "1");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "ron")).collect();
    files.sort();
    assert!(!files.is_empty(), "no recordings in {dir:?}");
    let content = Content::builtin();
    let mut failures = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        let mut rec: Recording = ron::from_str(&text).unwrap_or_else(|e| panic!("{f:?}: {e}"));
        let a = rec.play(content.clone());
        let b = rec.play(content.clone());
        assert_eq!(a.state_hash(), b.state_hash(), "{f:?} is not deterministic");
        let got = format!("{:016x}", a.state_hash());
        if bless {
            rec.expect = Some(got);
            let pretty = ron::ser::PrettyConfig::new().compact_arrays(true).depth_limit(2);
            std::fs::write(&f, ron::ser::to_string_pretty(&rec, pretty).unwrap()).unwrap();
        } else if rec.expect.as_deref() != Some(got.as_str()) {
            failures.push(format!("{}: expected {:?}, got {got} (depth {}, tick {})", f.display(), rec.expect, a.state.level.depth, a.state.tick));
        }
    }
    assert!(failures.is_empty(), "replays changed:\n{}", failures.join("\n"));
}
