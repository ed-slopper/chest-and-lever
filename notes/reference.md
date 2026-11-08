# The reference: what the public record shows of Levers and Chests

Every fact we build from, one per row. Code cites them by ID (`/// REF R21: ...`), the board by ID in Notes.
Rules: AGENTS.md sections 2 and 14. IDs are never reused or renumbered; gaps between numbers are on purpose.

**Trust** (AGENTS.md section 2): `1` = Notch's own post or clip · `2` = an article quoting or embedding it ·
`3` = a lead only (blogs, video titles, the subreddit). **Status**: `confirmed`, `unconfirmed` (trust 3, or one
outlet's reading), `superseded` (a later fact replaced it), `gap` (the record says nothing; ours to design).

Dates are UTC. Post dates without an article date were read from the post's ID. Gathered 2026-10-19 by
ed-slopper/wren; the long-form write-up with pictures is the research report in the project's Claude docs.

## Project and status

| ID | Fact | Date | Source | Trust | Status |
|---|---|---|---|---|---|
| R1 | Notch announces his studio Bitshift Entertainment (name, logo, an X account `@BitshiftEnt`); he works on the game mostly alone, with a couple of people on the business side | 2024-04-02 | [GWO](https://gameworldobserver.com/2024/04/03/bitshift-entertainment-studio-notch-new-game) | 2 | confirmed |
| R2 | "Basically on life support": the team is about to drop to one programmer; he can't find programmers in Los Angeles | 2025-09-27 | [post](https://x.com/notch/status/1972045972428742724) | 1 | confirmed |
| R3 | Latest update: far from done, mostly written with AI help ("vibe coded"), moving fast; feature creep around palette cycling | 2026-10-10 | [post](https://x.com/notch/status/2108907604953137305) | 1 | confirmed |
| R4 | Bitshift off life support; the game's ambition cut back to the first plans (the rest of the post is not in any copy found) | 2025-10-05 | [post](https://x.com/notch/status/1974905044396097918) | 1 | confirmed |
| R5 | Release plan: something extremely early for early adopters, then content added slowly from feedback, like early Minecraft; an internal date exists, not given | 2025-04-19 | [post](https://x.com/notch/status/1913677550657691939) | 1 | confirmed |
| R6 | Fans linked it to his abandoned *Legend of the Chambered*; he agreed it probably is, under the working title Levers and Chests | 2024-04-03 | [Techopse](https://www.techopse.com/notch-continues-to-tease-his-next-game-project/), [GWO](https://gameworldobserver.com/2024/04/03/bitshift-entertainment-studio-notch-new-game) | 2 | confirmed |
| R7 | The game would be sold or played at `lac.net` | 2024 | [Manifold market](https://manifold.markets/jim/when-will-notchs-new-game-levers-an) | 3 | unconfirmed |
| R8 | No politics in the game | 2024-09-03 | [post](https://x.com/notch/status/1831031638450696642) | 1 | confirmed |
| R9 | "A game made by humans for humans" | 2025-08-07 | [post](https://x.com/notch/status/1953582943705964882) | 1 | confirmed |
| R10 | Idea: ship it with a licence to make your own games in the same engine, as modding support | 2025-08-12 | [post](https://x.com/notch/status/1955344970916958598) | 1 | confirmed (idea only) |
| R11 | Chosen over a Minecraft spiritual successor after a poll; he called successors "sad nostalgia dumps" | 2025-01-07 | [post](https://x.com/notch/status/1876630234025677104), [GamesRadar](https://www.gamesradar.com/games/minecraft/notch-cancels-the-minecraft-2-he-announced-days-prior-because-spiritual-successors-are-always-kinda-sad-nostalgia-dumps/) | 1 | confirmed |

## Genre and design

| ID | Fact | Date | Source | Trust | Status |
|---|---|---|---|---|---|
| R12 | A roguelike "in the old sense" (cursed items, ridiculously broken mechanics) combined with a first-person dungeon crawler | 2024-04-03 | [GWO](https://gameworldobserver.com/2024/04/03/bitshift-entertainment-studio-notch-new-game) | 2 | confirmed |
| R13 | A turn-based old-school roguelike, presented as an old-school first-person **party-based** dungeon crawler | 2025-01 | [PCGamesN](https://www.pcgamesn.com/minecraft/2-sequel-plans-notch) | 2 | confirmed |
| R14 | Models: ADOM and NetHack for the roguelike; Legend of Grimrock (especially 2) and Eye of the Beholder for the tile-based crawler | 2025-01-01 | [poll post](https://x.com/notch/status/1874522284813107617) | 1 | confirmed |
| R15 | He hopes it will be at least half as fun as Legend of Grimrock 2 | 2025-01 | [PC Gamer](https://www.pcgamer.com/games/survival-crafting/sorry-everyone-minecraft-2-is-cancelled-and-it-might-be-our-fault/) | 2 | confirmed |
| R16 | Early pitch: "Borderlands with D20s", compared to Diablo | 2024-03 | [GWO](https://gameworldobserver.com/2024/04/03/bitshift-entertainment-studio-notch-new-game) | 2 | superseded by R13 |
| R17 | Not story-driven | 2025-02 | [USC Annenberg](https://www.uscannenbergmedia.com/2025/02/13/minecraft-creator-notch-and-his-next-move/) | 3 | unconfirmed |
| R18 | A hidden "true ending" | 2025-01 | [4Gamers](https://www.4gamers.com.tw/news/detail/69454/minecraft-2-sequel-plans-notch) | 3 | unconfirmed |

## Movement, input and time

| ID | Fact | Date | Source | Trust | Status |
|---|---|---|---|---|---|
| R20 | Movement is tile-based, first person | 2024-04-08 | [post](https://x.com/notch/status/1777402294897889740), [poll post](https://x.com/notch/status/1874522284813107617) | 1 | confirmed |
| R21 | Jump = move up one tile, with a one-frame grace before falling back down, so a move queued right after it lands you on the ledge | 2024-04-08 | [post](https://x.com/notch/status/1777402294897889740) | 1 | confirmed |
| R22 | A long jump crosses a one-tile gap (its animation was still placeholder) | 2024-04-08 | same post | 1 | confirmed |
| R23 | Player collision comes from the level data, so the world feels solid | 2024-04-08 | same post | 1 | confirmed |
| R24 | Player actions are decoupled from raw input, for custom key bindings | 2024-04-08 | same post | 1 | confirmed |
| R25 | Runs are seeded; the generator is a Mersenne Twister | 2024-04-08 | same post | 1 | confirmed |
| R26 | Free (non-tile) movement was also prototyped; he might use one in this game and the other in a sequel | 2024-07 | [Xboxygen](https://www.xboxygen.com/?p=326309), [post](https://x.com/notch/status/1777402294897889740) | 2 | confirmed (prototype only) |
| R27 | Time pauses while the player gives no input | 2024-09-03 | [clip post](https://x.com/notch/status/1831060458822127943), [gagadget](https://gagadget.com/en/500141-the-minecraft-creator-has-revealed-the-first-gameplay-footage-of-his-new-retro-game-levers-and-chests/) | 1 | confirmed |
| R28 | Playback speeds up the more commands are queued: spam actions without waiting for animations, or click slowly to watch them | 2024-09-03 | same | 1 | confirmed |
| R29 | Platforming on the test level felt like a 3D Prince of Persia | 2024-04-08 | [post](https://x.com/notch/status/1777402294897889740) | 1 | confirmed (feel) |

## Party, characters and interface

| ID | Fact | Date | Source | Trust | Status |
|---|---|---|---|---|---|
| R30 | The player leads a group of four characters; their portraits sit on the right of the screen | 2024-09-03 | [gagadget](https://gagadget.com/en/500141-the-minecraft-creator-has-revealed-the-first-gameplay-footage-of-his-new-retro-game-levers-and-chests/) | 2 | confirmed |
| R31 | The portraits spun in the Sept 2024 clip; he said to ignore it (a joke, not a feature) | 2024-09-03 | [clip post](https://x.com/notch/status/1831060458822127943) | 1 | confirmed (not a feature) |
| R32 | Named characters Mark and Reverie, with drawn portraits replacing the AI placeholders | 2025-02-23 | [80.lv](https://80.lv/articles/minecraft-creator-notch-offers-a-look-at-his-new-upcoming-game) | 2 | confirmed |
| R33 | A character sheet for "Jess", a Human Warrior: icons, categories, a list of attributes, flavour text; not final | 2025-01 | [HypeDrop](https://www.hypedrop.com/blog/everything-we-know-about-the-new-notch-game/) | 3 | unconfirmed |
| R34 | An inventory screen with item icons; portraits and icons were AI-made placeholders, every other asset a first draft | 2024-08-03 | [post](https://x.com/notch/status/1819802388959764541) | 1 | confirmed |
| R35 | A minimap, from his early design notes | 2024-07-25 | [post](https://x.com/notch/status/1816516091457470859) | 1 | confirmed (planned) |
| R36 | Characters ("warlocks") that slow down and speed up time | 2024-09 | [gagadget](https://gagadget.com/en/500141-the-minecraft-creator-has-revealed-the-first-gameplay-footage-of-his-new-retro-game-levers-and-chests/), [YouTube title](https://www.youtube.com/watch?v=4AfM1AO9cPA) | 3 | unconfirmed (likely a misreading of R28) |

## World and monsters

| ID | Fact | Date | Source | Trust | Status |
|---|---|---|---|---|---|
| R40 | Voxel graphics: blocky dungeons and voxel monsters | 2024-04 | [GWO](https://gameworldobserver.com/2024/04/03/bitshift-entertainment-studio-notch-new-game) | 2 | confirmed |
| R41 | Mobs like more detailed Minecraft zombies | 2025-02-23 | [80.lv](https://80.lv/articles/minecraft-creator-notch-offers-a-look-at-his-new-upcoming-game) | 2 | confirmed |
| R42 | A giant spider, and a ghost-like figure standing on tombstones | 2025-02-28 | [80.lv](https://80.lv/articles/notch-continues-working-on-his-new-game-sharing-even-more-wip-screenshots) | 2 | confirmed |
| R43 | A teammate made spiders spawn 10% of the time; Notch has arachnophobia | 2025-05-08 | [post](https://x.com/notch/status/1920412186125422666) | 1 | confirmed |
| R44 | Levels are procedurally generated | 2024-04-08 | [post](https://x.com/notch/status/1777402294897889740) | 1 | confirmed |
| R45 | Some structures are hand-built but "made with procedural generation in mind" | 2025-02-28 | [post](https://x.com/notch/status/1895609505742004735), [80.lv](https://80.lv/articles/notch-continues-working-on-his-new-game-sharing-even-more-wip-screenshots) | 1 | confirmed |
| R46 | Caves and tombs among the places shown | 2025-05 | [IndieKings](https://www.indiekings.com/2025/05/levers-and-chests-notchs-bold-return-to.html) | 3 | unconfirmed |
| R47 | The first test level was "random chaos" from the seeded generator, used to build collision and jumping | 2024-04-08 | [post](https://x.com/notch/status/1777402294897889740) | 1 | confirmed |

## Look and technology

| ID | Fact | Date | Source | Trust | Status |
|---|---|---|---|---|---|
| R50 | Screen-space ambient occlusion (SSAO) on the dungeon; screen-space reflections discussed | 2024-05-27 | [Techopse](https://www.techopse.com/notch-continues-to-tease-his-next-game-project/) | 2 | confirmed |
| R51 | A raytraced voxel renderer; its glitch pictures came from rendering back faces instead of front faces | 2025-02 | [80.lv](https://80.lv/articles/minecraft-creator-notch-offers-a-look-at-his-new-upcoming-game) | 2 | confirmed |
| R52 | Voxel fractals shown as engine work | 2025-02-28 | [80.lv](https://80.lv/articles/notch-continues-working-on-his-new-game-sharing-even-more-wip-screenshots) | 2 | confirmed |
| R53 | Sky colours from an artist-controlled lookup texture, chosen over computing Rayleigh scattering | 2025-01-07 | [post](https://x.com/notch/status/1876630234025677104) | 1 | confirmed |
| R54 | Palette cycling | 2026-10-10 | [post](https://x.com/notch/status/2108907604953137305) | 1 | confirmed |
| R55 | A splash screen in a stylised, pixelated LCD-display look | 2024-04-02 | [GWO](https://gameworldobserver.com/2024/04/03/bitshift-entertainment-studio-notch-new-game) | 2 | confirmed |
| R56 | 2024 placeholders: walls from Eye of the Beholder II, monsters from the Voxel Doom mod (Pinky Demon) | 2024-04 | [Techopse](https://www.techopse.com/notch-continues-to-tease-his-next-game-project/), [80.lv](https://80.lv/articles/minecraft-creator-launched-a-new-studio-bitshift-entertainment) | 2 | confirmed (placeholders, never to copy) |
| R57 | Written in JavaScript with WebGL (TWGL) | 2025-05 | [IndieKings](https://www.indiekings.com/2025/05/levers-and-chests-notchs-bold-return-to.html) | 3 | unconfirmed |
| R58 | A rendering engineer, Daryl Zuniga, on the team | 2025-05 | same | 3 | unconfirmed |
| R59 | Multiplayer considered and set aside | 2025-05 | same | 3 | unconfirmed |

## Shared by the project (images in a chat, origin not yet linked)

ed-slopper shared these on 2026-11-08; they look like Notch's own notes and screenshots, but until a post is
found and linked they count as leads (trust 3). The pictures themselves stay out of the repo.

| ID | Fact | Date | Source | Trust | Status |
|---|---|---|---|---|---|
| R60 | Design notes: a first-person roguelike "in the old school sense", ADOM meets Ultima Underworld; random, tile-based dungeons; a minimap the player can make notes on; an overworld between key locations (maybe seamless); lots of stats, exploitable on purpose (more than 100% protection from ice heals you), set items and unique items; danger that is always avoidable; permadeath; cursed items and mimics that teach good habits | 2026-11-08 | image shared by ed-slopper | 3 | unconfirmed |
| R61 | Interface in later clips: the party on the right as a 2×2 grid of cards, each with a name, four item slots, a portrait and two bars; a grid inventory below on a stone panel; a wide 3D view on the left with health bars over monsters | 2026-11-08 | images shared by ed-slopper | 3 | unconfirmed |
| R63 | The party is played with the mouse (as seen in the Sept 2024 clip, described by ed-slopper): clicking a member's portrait makes them attack; each card has two hand slots (a hand glyph when empty) and two lower slots for rings and gems; clicking an item uses it (a potion is drunk); right-clicking a member's item puts it in the main inventory; clicking an empty member slot opens a list of what fits, from the main inventory or swapped from another member, labelled by where it comes from; right-clicking in the main inventory drops the item on the ground. Spells throw out glowing particles | 2026-11-08 | [clip post](https://x.com/notch/status/1831060458822127943), described by ed-slopper | 2 | confirmed (our reading of the clip) |
| R62 | A monster line-up of voxel models: a slime, a beetle-like brute, a zombie, several humanoids, a cloaked figure, a mimic, a bat, a crow and a wolf among them | 2026-11-08 | image shared by ed-slopper | 3 | unconfirmed (we design our own, AGENTS.md §13) |

## Gaps: what the record does not show (ours to design)

| ID | Gap | Board row |
|---|---|---|
| R70 | How combat resolves (to-hit, damage, dice or not, who acts when) | G9 |
| R71 | Character classes, races, attributes and how they grow | G3 |
| R72 | What levers do (doors, bridges, traps, links between them) | G6 |
| R73 | What chests hold and how they bite back (loot tables, locks, traps, mimics, curses) | G7 |
| R74 | How cursed items work (sticking, identification, uncursing) | G5 |
| R75 | Death: permadeath is implied by "roguelike" but never stated | G11 |
| R76 | Food, light, rest or any other clock besides the action queue | L1 |
| R77 | Meta-progression between runs, and how a run is won | G11 |
| R78 | Spells or abilities beyond R36's unconfirmed time control | G10 |
