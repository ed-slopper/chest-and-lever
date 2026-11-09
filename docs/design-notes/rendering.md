# Rendering

Rows E4, E7, E8, E9, C8, C9. Code: `lac-rs/src/voxel.rs`, `palette.rs`, `textures.rs`, `models.rs`,
`lac-rs/assets/shaders/palette.wesl`.

## From the record

- R40: voxel graphics. R50: SSAO. R53: sky colours from an artist-made lookup texture. R54: palette cycling.
  R55: an LCD-style splash screen.

## Ours (F8, decided 2026-11-08: 480×270, 64 colours, 16 texels per tile)

- The 3D view renders to a 480×360 image (960×720 as an option), shown in the HUD with nearest filtering. F8 set
  480×270; ed-slopper asked for a roomier view, so it is 4:3 and fills the window's height.
- Tiles are drawn 1.5 wide and 1.25 tall (the sim's grid is unchanged); monsters are scaled up by 1.35 to match, so corridors and rooms
  feel roomy. The camera stands 0.42 of a tile behind the centre of the party's tile, with an 80° vertical
  field of view, so the walls beside the party are in view.
- There is one palette of 64 colours, in 8 ramps of 8: grey, brown, red, green, blue, fire, violet and bone.
  Tile textures are palette indices in an R8 atlas. The material extends Bevy's standard material, so lighting,
  SSAO and fog still apply. It looks colours up on the GPU and cycles three ranges: water, lava and crystal.
- One mesh per chunk (`lac-mesh`, kept optimised even in debug builds), with faces only between solid and open
  cells. Walls and floors are height fields over their texels (ed-slopper's reference: each brick's face stays
  flat, mortar and cracks sit back, and every brick's edge is rounded off rather than cut square). A point sits
  at each texel's centre at that texel's own depth (0 to 2, 0.035 tile a step; floors half as deep), so brick
  tops are flat and mortar reaches full depth; between them the surface slopes over one texel, softened into a
  bevel by a light 1-2-1 rounding. A face's rim takes the mean of its edge texels and the neighbouring face's,
  so a wall that goes on meets its neighbour without a crack; at corners the rim stays flush. Normals come from
  the surface, so the light rolls over the bevels, and hollows are a little darker. Ceilings stay flat. Liquids are lowered surfaces.
- The party's eye is at three quarters of a tile's height, so the party feels tall beside doors and monsters.
- Doors are two tiles high where the doorway is open above (the level generator keeps it so), one under a lintel.
  A two-high doorway gets a stone arch: the upper cell is filled but for a round vault, its stones laid as wedges
  round the curve; the door has a rounded top to fit it.
- Monsters and things are voxel models built in code from boxes and balls, coloured from the same palette. They
  are shaded soft: each corner's normal leans toward the rounded shape around it, and hollows darken, so the
  blocks read as smooth forms. Eyes and flames are brighter than white: they shine in the dark, and a soft bloom
  spreads their glow. The cave spider is a giant of our own: heavy bristled abdomen with a pale chevron, a
  cluster of red eyes, and long legs whose knees rise above its back. Wooden doors
  swing 90° on a hinge at one edge and portcullises slide up, animated over a third of a second.
- Light: the party carries a warm point light, wall torches flicker with their own lights, and there is a dim
  ambient light tinted by the level's theme (grey-green crypt and caves, blue tombs, violet deep). Stone is a
  damp grey-green, low in contrast (ed-slopper's reference: dark, mossy stone, fog fading to green-black).
  The ambient light carries most of the scene (1100), so walls beyond the party's light stay readable; the
  party's own light is weak and near-white, and fog runs from 4 to 16 tiles. Matched by measuring: the
  reference's middle brightness is about 41 of 255, with nine pixels in ten between 30 and 50. A wall
  torch's light (140k) sits close to the wall, so it throws a warm halo round the flame, about twice the
  room's brightness up close. Fog colour comes from a strip of palette colours, one per depth: our reading
  of R53 for an underground game.
- Particles (R63): spells, bolts and their impacts throw clouds of tiny glowing voxel cubes, with a light that
  fades with them; a hit monster sheds a puff of dark debris. Time stands still between actions (R27, R64), and
  so do particles: they hang in mid-air until the next action plays. Arrows fly as thin grey shafts and
  splinter where they land.
- Colour-blind palettes shift the clashing ramps (Options > Colours).
- The splash spells the title in 5×7 dot-matrix letters, with lit dots bright and unlit ones faint.
