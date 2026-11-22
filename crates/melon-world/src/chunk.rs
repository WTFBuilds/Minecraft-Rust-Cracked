//! Chunk and section storage.

use std::collections::HashMap;

pub const CHUNK_W: usize = 16;
pub const CHUNK_H: usize = 256;
pub const SECTION_SIZE: usize = 16;
/// Blocks per section: 16 × 16 × 16.
pub const SECTION_VOLUME: usize = SECTION_SIZE * SECTION_SIZE * SECTION_SIZE;
/// Sections stacked per chunk (256 blocks tall).
pub const SECTIONS_PER_CHUNK: usize = CHUNK_H / SECTION_SIZE;

/// Local block index inside a section: `(y, z, x)` order matches how the
/// renderer will slice sections later.
#[inline]
pub fn local_index(x: usize, y: usize, z: usize) -> usize {
    debug_assert!(x < SECTION_SIZE && y < SECTION_SIZE && z < SECTION_SIZE);
    (y << 8) | (z << 4) | x
}

/// A 16×16×16 block section. 8 KB of flat `u16`s — no palette yet.
#[derive(Debug, Clone)]
pub struct Section {
    blocks: Box<[u16; SECTION_VOLUME]>,
    non_air: usize,
}

impl Section {
    pub fn empty() -> Self {
        Self {
            blocks: Box::new([0; SECTION_VOLUME]),
            non_air: 0,
        }
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> u16 {
        self.blocks[local_index(x, y, z)]
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, id: u16) {
        let idx = local_index(x, y, z);
        let was_air = self.blocks[idx] == 0;
        let is_air = id == 0;
        if !was_air && is_air {
            self.non_air -= 1;
        } else if was_air && !is_air {
            self.non_air += 1;
        }
        self.blocks[idx] = id;
    }

    /// `true` when every block is air — lets the renderer skip empty sections.
    pub fn is_empty(&self) -> bool {
        self.non_air == 0
    }
}

impl Default for Section {
    fn default() -> Self {
        Self::empty()
    }
}

/// A 16×256×16 column of blocks.
#[derive(Debug, Clone)]
pub struct Chunk {
    sections: Vec<Section>,
}

impl Chunk {
    pub fn empty() -> Self {
        Self {
            sections: vec![Section::empty(); SECTIONS_PER_CHUNK],
        }
    }

    /// `y` is the world-space height within this chunk (0–255).
    pub fn get(&self, x: usize, y: usize, z: usize) -> u16 {
        let (section, local_y) = (y / SECTION_SIZE, y % SECTION_SIZE);
        self.sections[section].get(x, local_y, z)
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, id: u16) {
        let (section, local_y) = (y / SECTION_SIZE, y % SECTION_SIZE);
        self.sections[section].set(x, local_y, z, id);
    }

    pub fn section(&self, index: usize) -> &Section {
        &self.sections[index]
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::empty()
    }
}

/// Chunk position in the horizontal plane.
pub type ChunkPos = (i32, i32);

/// A world: chunk storage with world-space block access.
#[derive(Debug, Default)]
pub struct World {
    chunks: HashMap<ChunkPos, Chunk>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    /// Floor-dividing chunk lookup: negative coordinates resolve correctly.
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<u16> {
        if !(0..CHUNK_H as i32).contains(&y) {
            return None;
        }
        let chunk = self
            .chunks
            .get(&(x.div_euclid(CHUNK_W as i32), z.div_euclid(CHUNK_W as i32)))?;
        Some(chunk.get(
            x.rem_euclid(CHUNK_W as i32) as usize,
            y as usize,
            z.rem_euclid(CHUNK_W as i32) as usize,
        ))
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: u16) {
        if !(0..CHUNK_H as i32).contains(&y) {
            return;
        }
        let key = (x.div_euclid(CHUNK_W as i32), z.div_euclid(CHUNK_W as i32));
        let chunk = self.chunks.entry(key).or_default();
        chunk.set(
            x.rem_euclid(CHUNK_W as i32) as usize,
            y as usize,
            z.rem_euclid(CHUNK_W as i32) as usize,
            id,
        );
    }

    /// Install a pre-built chunk (generators and arena importers).
    pub fn insert(&mut self, cx: i32, cz: i32, chunk: Chunk) {
        self.chunks.insert((cx, cz), chunk);
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn chunks(&self) -> impl Iterator<Item = (&ChunkPos, &Chunk)> {
        self.chunks.iter()
    }

    /// Direct chunk access for meshing/serialization.
    pub fn chunk(&self, cx: i32, cz: i32) -> Option<&Chunk> {
        self.chunks.get(&(cx, cz))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block;

    #[test]
    fn local_index_matches_yzx_layout() {
        assert_eq!(local_index(0, 0, 0), 0);
        assert_eq!(local_index(1, 0, 0), 1);
        assert_eq!(local_index(0, 0, 1), 16);
        assert_eq!(local_index(0, 1, 0), 256);
        assert_eq!(local_index(15, 15, 15), SECTION_VOLUME - 1);
    }

    #[test]
    fn section_tracks_non_air() {
        let mut s = Section::empty();
        assert!(s.is_empty());
        s.set(3, 4, 5, block::STONE);
        assert!(!s.is_empty());
        s.set(3, 4, 5, block::AIR);
        assert!(s.is_empty());
    }

    #[test]
    fn chunk_get_set_across_heights() {
        let mut c = Chunk::empty();
        c.set(1, 0, 1, block::BEDROCK); // bottom section
        c.set(1, 255, 1, block::WOOL_BLUE); // top section
        assert_eq!(c.get(1, 0, 1), block::BEDROCK);
        assert_eq!(c.get(1, 255, 1), block::WOOL_BLUE);
        assert_eq!(c.get(1, 128, 1), block::AIR);
    }

    #[test]
    fn world_resolves_negative_chunk_coords() {
        let mut w = World::new();
        w.set_block(-1, 5, -1, block::GRASS); // chunk (-1, -1), local (15, 5, 15)
        assert_eq!(w.get_block(-1, 5, -1), Some(block::GRASS));
        assert_eq!(w.get_block(0, 5, 0), None);
        assert_eq!(w.chunk_count(), 1);
    }

    #[test]
    fn world_rejects_out_of_height() {
        let mut w = World::new();
        w.set_block(0, -1, 0, block::STONE);
        w.set_block(0, 256, 0, block::STONE);
        assert_eq!(w.get_block(0, -1, 0), None);
        assert_eq!(w.get_block(0, 256, 0), None);
        assert_eq!(w.chunk_count(), 0);
    }
}
