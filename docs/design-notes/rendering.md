# Rendering

Rows E4, E7, E8, E9, C8, C9. Code: `lac-rs/src/voxel.rs`, `palette.rs`, `textures.rs`, `models.rs`,
`lac-rs/assets/shaders/palette.wesl`.

## From the record

- R40: voxel graphics. R50: SSAO. R53: sky colours from an artist-made lookup texture. R54: palette cycling.
  R55: an LCD-style splash screen.

## Ours (F8, decided 2026-11-08: 480×270, 64 colours, 16 texels per tile)

- The 3D view renders to a 480×360 image (960×720 as an option), shown in the HUD with nearest filtering. F8 set
  480×270; ed-slopper asked for a roomier view, so it is 4:3 and fills the window's height.
- The camera stands 0.38 of a tile behind the centre of the party's tile, with a 74° vertical field of view, so
  the walls beside the party are in view.
- There is one palette of 64 colours, in 8 ramps of 8: grey, brown, red, green, blue, fire, violet and bone.
  Tile textures are palette indices in an R8 atlas. The material extends Bevy's standard material, so lighting,
  SSAO and fog still apply. It looks colours up on the GPU and cycles three ranges: water, lava and crystal.
- One mesh per chunk (`lac-mesh`, kept optimised even in debug builds), with faces only between solid and open
  cells. Each face is carved into voxel relief at 16 per tile edge: every texel of a texture has a depth (0 to 2
  sixteenths), so mortar, seams, cracks and worn bricks sit back from the face, with small side walls lit by
  their own direction. Outside corners stay flush; ceilings are flat (they cost the most and are seen least).
  Each face direction gets a fixed shade, a little darker the deeper the texel. Liquids are lowered surfaces.
- Monsters and things are voxel models built in code from boxes, coloured from the same palette. Wooden doors
  swing 90° on a hinge at one edge and portcullises slide up, animated over a third of a second.
- Light: the party carries a warm point light, wall torches flicker with their own lights, and there is a dim
  ambient light. Fog colour comes from a strip of palette colours, one per depth: our reading of R53 for an
  underground game.
- Colour-blind palettes shift the clashing ramps (Options > Colours).
- The splash spells the title in 5×7 dot-matrix letters, with lit dots bright and unlit ones faint.
