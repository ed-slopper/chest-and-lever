# AGENTS.md: how agents work in this repo

## 1. The goal

A first-person, tile-based, party dungeon crawler crossed with an old-school roguelike, written in **Rust**
(`lac-rs/`, Bevy as the window, GPU, input and audio host), recreating **Levers and Chests**: the unreleased game
Markus "Notch" Persson and Bitshift Entertainment have shown since April 2024. We rebuild everything the public
record shows (movement, the queued-action clock, the party, seeded levels, the look) and design the rest
ourselves, keeping the two apart.

The board is [`WORK.md`](WORK.md). The rules below let any number of AI agents, working for any number of
people, pick work from it without clashing.

## 2. The reference

There is no binary to read: the game was never released. The reference is **the public record**, collected in
[`notes/reference.md`](notes/reference.md), one fact per row with an ID (`R1`, `R2`, ...) and a source link.

| | |
|---|---|
| Game | Levers and Chests (working title), Bitshift Entertainment, Los Angeles; unreleased |
| Shown | April 2024 to October 2026, on Notch's X account (`@notch`) and in press that quoted it |
| Facts | `notes/reference.md` (IDs `R…`); pictures and clips listed in `notes/media.md` |
| Status | Still in development as of 2026-10-10 (R3). It may ship; we are a separate, fan-made game (section 13) |

How much to trust a source, highest first:

1. Notch's own posts, and clips he posted.
2. Articles that quote or embed those posts word for word (80.lv, Game World Observer, PC Gamer, PCGamesN,
   GamesRadar, gagadget, Techopse).
3. Everything else (IndieKings, HypeDrop, TikTok, YouTube re-cuts, the subreddit): **leads only**. Their claims
   go into `notes/reference.md` marked `unconfirmed` and never decide behaviour on their own.

When two sources disagree the higher one wins. When the record says nothing, the behaviour is ours (section 13).

## 3. When to self-assign

- **No task given** ("start", "go", "work on the board", a link to `WORK.md`): pick a row with the rules in
  section 6 and start without asking.
- **A task named** ("do E2", "fix the jump"): do that one. Claim its row first, or add a row for it if there is
  none.
- **Only a question**: answer it and claim nothing.

What the human says always beats the board's order.

## 4. Who you work for, and your agent name

1. Find the person's GitHub name: ask once if you can't tell, otherwise take it from
   `git config user.email` / `gh auth status` / the remote. Write it in the form GitHub shows it (`ed-slopper`).
2. Give yourself an agent name: a short plain word (`puffin`, `otter`, `birch`). Save it once per clone:
   `git config --local agent.name <word>`. If the clone already has one, use it. One person can run several
   agents; each agent needs its own clone (or worktree) and its own name.
3. Every claim, branch and commit carries `<person>/<agent>`, for example `ed-slopper/wren`. In a commit, put it
   on its own line above the `Co-Authored-By` lines.

## 5. What is taken

Before you pick, `git fetch --all --prune` and look at:

- every row on the board with anything in **Who** (claimed), on `main` **and** on every other branch;
- every file someone else changed in the last 7 days, on any branch or open pull request:
  `git log --all --since=7.days --name-only --format='%an %s'` and `gh pr list` / `gh pr diff <n> --name-only`.
  You may read those files; do not change them unless your row needs it and their owner is you, or the human
  says so.

## 6. How to pick

Go through the tiers in order: P0, P1, P2, P3, P4, P5. Take the **first** open row (empty Who) where:

- **Area** is your person, `unowned` or `any` (never `humans`: those are decisions for people, not agents);
- every row in **Needs** is in the Done table;
- none of the files it will touch are taken (section 5).

If the row is big, split it first (into `E3a`, `E3b`, ... rows with the same Area and Needs) and claim only the
part you will finish this session.

## 7. How to claim

1. On `main` (or the board branch, if a human moved the board), write `<person>/<agent> (<YYYY-MM-DD>)` in the
   row's **Who**.
2. Commit **only `WORK.md`**, message `claim <ID>: <task in a few words>`, and push straight away, before you start
   the work. Push to `main` if you can. If you can't, push a branch `claim-<ID>-<words>-<person>-<agent>` and open
   a pull request.
3. If the push is rejected, `git pull --rebase`, re-read the board, and if the row is still free try once more.
   If someone else got it, pick again.

## 8. Branches

Names: `<what>-<ID>-<two to five plain words>-<person>-<agent>`, where `<what>` is:

| what | for |
|---|---|
| `claim` | a claim that could not be pushed to `main` |
| `join` | adding yourself to a shared row |
| `done` | moving a row to Done |
| `board` | any other change to the board or these rules |
| `work` | the code itself |

Examples: `work-E1-action-queue-ed-slopper-wren`, `board-split-g6-levers-ed-slopper-wren`.
Never a bare name such as `claim-12`, `patch-1` or `fix`.

When several agents of one person are working at the same time, each uses its own `work-` branch and merges it
back into that person's branch (`main` while there is only one person) when its row is done. One agent on its
own may commit straight to the person's branch.

## 9. Finishing and stopping

- **Done**: move the row from its tier to the **Done** table with the date and the commit hash, in the same
  commit as the last code change or right after it. Every gap you found becomes a new open row (next free ID in
  that tier, Notes: `Gap of <ID>`).
- **Stopping early**: clear your name from Who, and write in Notes what is finished, what is left and where
  (files, functions). Commit and push that.

## 10. Sharing a row

Two people, or two agents, may be on one row **only when a human asks**. Add the second name with `+`:
`ed-slopper/wren (2026-10-19) + ed-slopper/otter (2026-10-20)`, and write in Notes who takes which part
(systems, files). Push it like a claim, on a `join-` branch if not on `main`.

## 11. Changing the board

Anyone may change priorities, rows, areas or these rules, and their agent does it when asked. On its own an
agent may only **add rows, split rows and fix facts** (a wrong source, a file that moved). Every change, by
anyone, gets one line in **Changes to the board** at the bottom of `WORK.md`:
`2026-10-19 ed-slopper/wren: split G6 into G6a-G6c`.

`python tools/check_board.py` must pass before you push a board change (CI runs it too).

## 12. Areas

Who owns what. One person so far; **owners, please correct this table.** A row's Area is a person, `unowned`
(nobody's yet, anyone may take it), `any` (shared files, anyone) or `humans` (decisions agents must not take).

| Person | Branch | Area | Files |
|---|---|---|---|
| ed-slopper | `main` | Everything so far: the board, the notes, the Rust game | `lac-rs/**`, `notes/**`, `docs/**`, `tools/**`, `assets/**` |

New people: add your own line (in a `board-` commit) before you claim a row, and take rows that are `unowned` or
`any` until an owner hands an area over.

## 13. Rules of the project

- **Nothing of theirs in the repo, ever.** No screenshots, frames, portraits, models, textures, sounds or text
  taken from Levers and Chests, Notch's posts or any video of them, not even traced or "cleaned up". The same
  goes for what L&C itself borrowed as placeholders (Eye of the Beholder II textures, the Voxel Doom models) and
  for the assets of Notch's *Prelude of the Chambered* (copyright Mojang). Pictures stay on your PC as
  reference; the repo gets **links** (`notes/media.md`) and facts in our own words. Every file under `assets/`
  is ours or openly licensed and listed in `assets/CREDITS.md` (`tools/check_assets.py`, CI). The `.gitignore`
  is an allow-list: only our own folders are let in.
- **Not their name or their characters.** The product's name is a `humans` decision (board row F5). Until then
  "Levers and Chests" appears only in notes and docs, as the thing we recreate, never as our title in the game,
  a store page or a release. Notch's character names (Mark, Reverie, Jess) are not used for ours.
- **Build what the record shows, as it shows it.** Where `notes/reference.md` describes a behaviour (one tile per
  step, a jump of one tile with one tick of grace, time frozen while no action is queued), build exactly that.
  A row is done when it matches the reference, not when it feels right.
- **Cite the reference next to every behaviour taken from it**, in the doc comment, by fact ID:
  ``/// REF R21: jump = up one tile, one tick of grace before the fall``. Several facts: list them all.
- **Mark everything that is ours** with the tag `OURS:` and what it decides:
  `// OURS: damage roll 1d6 + strength bonus; the record says nothing about combat (R70)`.
  Bevy itself (window, GPU, audio output) is our host, not a design choice. `grep -rn "OURS:" lac-rs` must list
  every design decision we made; a later row may replace one when new facts appear.
- **The simulation is deterministic.** `lac-rs/sim` has no Bevy, no wall clock, no thread timing, no unseeded
  randomness, and no iteration over `HashMap`/`HashSet` (use `BTreeMap`, `IndexMap` or sorted `Vec`s). Every
  random draw comes from the run's seed through its own named stream (row F9). Same seed + same actions = the
  same run, bit for bit; the replay test (row F4) proves it.
- **Do not chase a gap outside your task.** Write it in the design notes of its system and add it as a new open
  row.

## 14. Research

- New facts go into `notes/reference.md` as new rows with the next free `R` ID, a date, the source link and the
  trust level of section 2. Never renumber a fact others cite; fix it in place and say so in Changes.
- Write facts in our own words. Quote at most a few words of a source, and only when the exact wording matters.
- X blocks most tools from reading `x.com` directly. Search-engine copies of a post are fine to read; give the
  post's own URL as the source. Reddit (`r/leversandchests`) is a lead source (section 2); a human may need to
  read it in a browser.
- Images and clips: add a row to `notes/media.md` (date, what it shows, link). The file itself stays out of the
  repo (section 13).
- Design notes, one file per system, live in `docs/design-notes/` (index: its `README.md`). Write "from the
  record" (with `R` IDs) and "ours" (our choices and why) apart.

## 15. Building and checking

- `cd lac-rs && cargo run --release`; the simulation's tests: `cd lac-rs && cargo test -p lac-sim`. Rust stable,
  Bevy 0.20 (row F1 pins it). On Windows use the MSVC toolchain. On Linux Bevy needs
  `pkg-config libasound2-dev libudev-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev libxkbcommon-dev`.
- Run the build and the tests that cover your system before marking a row done. Once the replay test exists
  (row F4), a sim change is **checked** only when the replay test still passes or its expected hash is updated
  on purpose, with the reason in the commit.
- CI (`.github/workflows/`): `Guard` runs on every push (board format, asset credits, our Python tools);
  `lac-rs` (Bevy, slow) runs only when `lac-rs/` changes, or by hand. Don't build the Bevy app, locally or by
  pushing to it, unless a change needs testing or the human asks.

## 16. End of every session

Finish by writing, for each person with an agent on this repo, a ready-to-paste prompt for their next task.
Each prompt must work with no other context, and contain:

- repo URL and the branch to work on;
- the row ID and the task in one sentence;
- the reference facts it builds on (`R` IDs in `notes/reference.md`);
- our files it touches;
- how to tell it is done (what to build or run, what must pass or match);
- what not to touch (taken files, other people's areas, `humans` rows).
