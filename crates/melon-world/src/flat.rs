//! Procedural flat-world generator — the Phase 2/3 test world and the base
//! layer of arena templates.

use crate::chunk::{World, CHUNK_W};
use crate::{block::*, Chunk};

/// Layer stack from bedrock upward: `(block_id, thickness)`.
#[derive(Debug, Clone)]
pub struct FlatGenerator {
    pub layers: Vec<(u16, u8)>,
}

impl FlatGenerator {
    /// Classic superflat: bedrock → stone → dirt → grass.
    pub fn superflat() -> Self {
        Self {
            layers: vec![(BEDROCK, 1), (STONE, 2), (DIRT, 2), (GRASS, 1)],
        }
    }

    /// Generate a `width × depth` chunk flat world. Chunk coordinates run
    /// `0..width`, matching how the server will assign world regions in
    /// Phase 3.
    pub fn generate(&self, width: u32, depth: u32) -> World {
        let mut world = World::new();
        for cx in 0..width as i32 {
            for cz in 0..depth as i32 {
                let mut chunk = Chunk::empty();
                for lx in 0..CHUNK_W {
                    for lz in 0..CHUNK_W {
                        let mut y = 0u32;
                        for &(id, thickness) in &self.layers {
                            for _ in 0..thickness {
                                chunk.set(lx, y as usize, lz, id);
                                y += 1;
                            }
                        }
                    }
                }
                world.insert(cx, cz, chunk);
            }
        }
        world
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn superflat_layer_stack() {
        let world = FlatGenerator::superflat().generate(2, 2);
        assert_eq!(world.chunk_count(), 4);
        // Layers: bedrock y0 · stone y1–2 · dirt y3–4 · grass y5 · air y6+.
        assert_eq!(world.get_block(0, 0, 0), Some(BEDROCK));
        assert_eq!(world.get_block(10, 1, 10), Some(STONE));
        assert_eq!(world.get_block(10, 2, 10), Some(STONE));
        assert_eq!(world.get_block(10, 3, 10), Some(DIRT));
        assert_eq!(world.get_block(10, 4, 10), Some(DIRT));
        assert_eq!(world.get_block(10, 5, 10), Some(GRASS));
        assert_eq!(world.get_block(10, 6, 10), Some(AIR));
        assert_eq!(world.get_block(20, 0, 20), Some(BEDROCK)); // second chunk
    }

    #[test]
    fn chunk_edges_line_up() {
        let world = FlatGenerator::superflat().generate(2, 1);
        // Block at local x=15 of chunk 0 and local x=0 of chunk 1 must both
        // be grass — proves layers were applied per-column, not per-chunk.
        assert_eq!(world.get_block(15, 5, 5), Some(GRASS));
        assert_eq!(world.get_block(16, 5, 5), Some(GRASS));
        assert_eq!(world.get_block(15, 6, 5), Some(AIR));
        assert_eq!(world.get_block(16, 6, 5), Some(AIR));
    }
}
