# The clock

Rows E1, E2. Code: `lac-rs/sim/src/sim.rs` (the queue), `lac-rs/src/game.rs` (`playback`, `speed`).

## From the record

- R27: time pauses while the player gives no input.
- R28: playback speeds up the more commands are queued: spam actions without waiting, or act slowly to watch.

## Ours

- The app takes the next action only when the previous animation has finished. Each animation lasts its base
  time divided by `1 + 0.8 × (actions still waiting)`, and never less than 35 ms.
- Base times: step 0.22 s, turn 0.17 s, jump 0.24 s, long jump 0.34 s, falls 0.08 s per tile, attacks and
  spells about 0.22 s, a wait 0.08 s.
- The queue holds up to 32 actions.
- R36 (warlocks who slow time) is unconfirmed and probably a misreading of R28. The mage's Quicken spell and
  potions of haste are our own take: the party acts twice for each monster turn.

## Ticks (R64)

- A tick is a sixtieth of a second of game time; it passes only while an action plays. Each action costs its own
  ticks (`action_ticks`): steps, waits, using and items 30; jumps 15; long jumps 40; attacks and spells 36;
  turning, dropping, picking up and swapping 0 (free, but still animated). Haste halves what the party's actions
  cost.
- Monsters gain their `speed` in energy every tick and act for every 360 (speed 12 = once per 30 ticks).
- Poison, healing, mana and timers beat once per round of 30 ticks.
- The animation length (R64's ActionDuration) is separate, and the queue speeds it up (R28, R64's
  ActionWarpFactor).
