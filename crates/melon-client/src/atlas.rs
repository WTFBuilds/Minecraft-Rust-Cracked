//! Procedural texture atlas — MELON's own art, generated deterministically at
//! runtime. No external image files, no Mojang assets.
//!
//! Layout: 16×16 grid of 16×16 pixel tiles in one 256×256 RGBA8 texture.
//! Every tile is a base color with per-pixel hash noise — simple, readable,
//! and deterministic (same bytes on every run and every machine).

/// Pixels per tile edge.
pub const TILE_PX: usize = 16;
/// Tiles per atlas edge.
pub const ATLAS_TILES: usize = 16;
/// Atlas edge in pixels.
pub const ATLAS_PX: usize = TILE_PX * ATLAS_TILES;

/// Half-texel inset so neighboring tiles never bleed at quad edges.
const UV_INSET: f32 = 0.5 / ATLAS_PX as f32;

/// Tile ids — row 0 of the atlas.
pub mod tile {
    pub const GRASS_TOP: u8 = 0;
    pub const GRASS_SIDE: u8 = 1;
    pub const DIRT: u8 = 2;
    pub const STONE: u8 = 3;
    pub const BEDROCK: u8 = 4;
    pub const WOOL_RED: u8 = 5;
    pub const WOOL_BLUE: u8 = 6;
    pub const WOOL_YELLOW: u8 = 7;
    pub const WOOL_GREEN: u8 = 8;
    pub const IRON: u8 = 9;
    pub const GOLD: u8 = 10;
    pub const DIAMOND: u8 = 11;
    pub const BED_HEAD: u8 = 12;
    pub const BED_FOOT: u8 = 13;
}

/// Deterministic 8-bit hash noise for pixel (x, y) with a per-tile seed.
fn noise(x: u32, y: u32, seed: u32) -> u32 {
    let mut h =
        x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA6B) ^ seed.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 13;
    h = h.wrapping_mul(0x27D4_EB2F);
    h ^= h >> 15;
    h & 0xFF
}

/// Fill one 16×16 tile with `base` + noise, optionally drawing rows from the
/// top with an override color (grass fringe, bed pillow, …).
fn fill_tile(
    px: &mut [u8],
    tile: u8,
    base: [u8; 3],
    noise_amp: u8,
    seed: u32,
    top_rows: Option<(usize, [u8; 3])>,
) {
    let tx = (tile % ATLAS_TILES as u8) as usize;
    let ty = (tile / ATLAS_TILES as u8) as usize;
    for py in 0..TILE_PX {
        for pxx in 0..TILE_PX {
            let n = (noise(pxx as u32, py as u32, seed) as i32 - 128) * noise_amp as i32 / 255;
            let color = top_rows
                .filter(|(rows, _)| py < *rows)
                .map_or(base, |(_, c)| c);
            let r = (color[0] as i32 + n).clamp(0, 255) as u8;
            let g = (color[1] as i32 + n).clamp(0, 255) as u8;
            let b = (color[2] as i32 + n).clamp(0, 255) as u8;
            let x = tx * TILE_PX + pxx;
            let y = ty * TILE_PX + py;
            let idx = (y * ATLAS_PX + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[r, g, b, 0xFF]);
        }
    }
}

/// Generate the full atlas RGBA8 buffer (`ATLAS_PX × ATLAS_PX × 4` bytes).
pub fn generate() -> Vec<u8> {
    let mut px = vec![0u8; ATLAS_PX * ATLAS_PX * 4];

    use tile::*;
    fill_tile(&mut px, GRASS_TOP, [106, 170, 64], 46, 0x51E3, None);
    // Grass side: dirt with a 4-row grass fringe on top.
    fill_tile(
        &mut px,
        GRASS_SIDE,
        [134, 96, 67],
        30,
        0x9A2D,
        Some((4, [106, 170, 64])),
    );
    fill_tile(&mut px, DIRT, [134, 96, 67], 34, 0xD1CE, None);
    fill_tile(&mut px, STONE, [127, 127, 127], 26, 0x7B04, None);
    fill_tile(&mut px, BEDROCK, [56, 56, 56], 52, 0x0BED, None);
    fill_tile(&mut px, WOOL_RED, [176, 46, 38], 18, 0x4E11, None);
    fill_tile(&mut px, WOOL_BLUE, [53, 57, 157], 18, 0x4E12, None);
    fill_tile(&mut px, WOOL_YELLOW, [248, 197, 39], 14, 0x4E13, None);
    fill_tile(&mut px, WOOL_GREEN, [93, 124, 22], 18, 0x4E14, None);
    fill_tile(&mut px, IRON, [216, 216, 216], 12, 0x4E15, None);
    fill_tile(&mut px, GOLD, [250, 214, 53], 12, 0x4E16, None);
    fill_tile(&mut px, DIAMOND, [98, 237, 228], 12, 0x4E17, None);
    // Bed head: red blanket with a white pillow strip.
    fill_tile(
        &mut px,
        BED_HEAD,
        [176, 46, 38],
        14,
        0x4E18,
        Some((5, [235, 235, 235])),
    );
    // Bed foot: blanket with a pale footboard edge.
    fill_tile(
        &mut px,
        BED_FOOT,
        [176, 46, 38],
        14,
        0x4E19,
        Some((3, [214, 214, 214])),
    );

    // Unused tile slots get the classic magenta/black "missing texture"
    // checker — visible in-world instead of silently invisible.
    for py in 0..ATLAS_PX {
        for pxx in 0..ATLAS_PX {
            let in_known_tile = py < TILE_PX && pxx < TILE_PX * 14;
            if in_known_tile {
                continue;
            }
            let checker = ((pxx >> 3) + (py >> 3)) % 2 == 0;
            let color = if checker {
                [236, 68, 236]
            } else {
                [28, 28, 28]
            };
            let idx = (py * ATLAS_PX + pxx) * 4;
            px[idx..idx + 4].copy_from_slice(&[color[0], color[1], color[2], 0xFF]);
        }
    }

    px
}

/// Atlas-normalized UV rect for a tile: `[u0, v0, u1, v1]`, inset by half a
/// texel on every edge.
pub fn tile_uv(tile: u8) -> [f32; 4] {
    let tx = (tile % ATLAS_TILES as u8) as usize;
    let ty = (tile / ATLAS_TILES as u8) as usize;
    let step = 1.0 / ATLAS_TILES as f32;
    let (u0, v0) = (tx as f32 * step, ty as f32 * step);
    let (u1, v1) = (u0 + step, v0 + step);
    [u0 + UV_INSET, v0 + UV_INSET, u1 - UV_INSET, v1 - UV_INSET]
}

/// Per-block tile mapping, ordered to match the mesher's face order
/// (`[PosX, NegX, PosY, NegY, PosZ, NegZ]`). Unknown ids fall back to stone —
/// never crash, never invisible.
pub fn tiles_for_block(id: u16) -> [u8; 6] {
    use melon_world::block;
    use tile::*;
    let all = |t: u8| [t; 6];
    match id {
        block::GRASS => [
            GRASS_SIDE, GRASS_SIDE, GRASS_TOP, DIRT, GRASS_SIDE, GRASS_SIDE,
        ],
        block::DIRT => all(DIRT),
        block::STONE => all(STONE),
        block::BEDROCK => all(BEDROCK),
        block::WOOL_RED => all(WOOL_RED),
        block::WOOL_BLUE => all(WOOL_BLUE),
        block::WOOL_YELLOW => all(WOOL_YELLOW),
        block::WOOL_GREEN => all(WOOL_GREEN),
        block::IRON_BLOCK => all(IRON),
        block::GOLD_BLOCK => all(GOLD),
        block::DIAMOND_BLOCK => all(DIAMOND),
        block::BED_HEAD => all(BED_HEAD),
        block::BED_FOOT => all(BED_FOOT),
        _ => all(STONE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melon_world::block;

    #[test]
    fn atlas_is_fully_opaque() {
        let px = generate();
        assert_eq!(px.len(), ATLAS_PX * ATLAS_PX * 4);
        for a in px.iter().skip(3).step_by(4) {
            assert_eq!(*a, 0xFF, "every atlas pixel must be opaque");
        }
    }

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(
            generate(),
            generate(),
            "atlas bytes must be identical per run"
        );
    }

    #[test]
    fn grass_top_is_green_dominant() {
        let px = generate();
        // Center pixel of the grass top tile.
        let idx = (8 * ATLAS_PX + 8) * 4;
        let [r, g, b] = [px[idx], px[idx + 1], px[idx + 2]];
        assert!(g > r && g > b, "grass top must read green: {r},{g},{b}");
    }

    #[test]
    fn grass_side_has_green_fringe_over_dirt() {
        let px = generate();
        let at = |x: usize, y: usize, c: usize| px[(y * ATLAS_PX + x) * 4 + c] as i32;
        // Tile 1 sits at atlas x=16..32: fringe row 1 green, row 12 brown dirt.
        let (g_top, r_top) = (at(24, 1, 1), at(24, 1, 0));
        let (g_low, r_low) = (at(24, 12, 1), at(24, 12, 0));
        assert!(g_top > r_top, "fringe must be green");
        assert!(r_low > g_low, "body must be dirt-brown");
    }

    #[test]
    fn tile_uv_rects_are_inset_and_tile_sized() {
        let [u0, v0, u1, v1] = tile_uv(tile::STONE);
        let step = 1.0 / ATLAS_TILES as f32;
        assert!(
            (u0 - 3.0 * step).abs() < step && u0 > 3.0 * step,
            "u0 inside tile 3"
        );
        assert!(
            (v1 - 1.0 * step).abs() < step && v1 < 1.0 * step,
            "v1 inside tile row 0"
        );
        assert!((u1 - u0) < step && (v1 - v0) < step, "rect must be inset");
    }

    #[test]
    fn unknown_block_falls_back_to_stone() {
        assert_eq!(tiles_for_block(0xFFFF), [tile::STONE; 6]);
        assert_eq!(tiles_for_block(block::STONE), [tile::STONE; 6]);
        // Grass: sides are grass_side, top grass_top, bottom dirt.
        let grass = tiles_for_block(block::GRASS);
        assert_eq!(grass[2], tile::GRASS_TOP); // PosY
        assert_eq!(grass[3], tile::DIRT); // NegY
        assert_eq!(grass[0], tile::GRASS_SIDE); // PosX
    }
}
