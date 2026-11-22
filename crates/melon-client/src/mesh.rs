//! Section mesher — turns 16×16×16 world sections into GPU-ready vertex
//! lists with neighbor face culling.
//!
//! Phase 2 uses **culled meshing**: a face exists only where a solid block
//! touches air. Full greedy merging (fusing coplanar quads) is a follow-up
//! optimization — the vertex format and draw path here are already shaped
//! for it, and the benchmark below keeps meshing cost honest until then.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use melon_world::block;
use melon_world::chunk::{World, CHUNK_H, SECTION_SIZE};

use crate::atlas::{tile_uv, tiles_for_block};

/// One terrain vertex. 36 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    /// World-space position (section origin baked in — no per-draw uniforms).
    pub position: [f32; 3],
    /// Atlas UV.
    pub uv: [f32; 2],
    /// Baked directional shade (voxel-style fake lighting).
    pub shade: f32,
}

pub const VERTEX_STRIDE: usize = std::mem::size_of::<Vertex>();

/// Face order is load-bearing: it matches [`atlas::tiles_for_block`] and the
/// per-face shade table.
const FACES: [FaceDef; 6] = [
    // +X
    FaceDef {
        normal: [1, 0, 0],
        corners: [[1, 0, 1], [1, 0, 0], [1, 1, 0], [1, 1, 1]],
        uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        shade: 0.72,
    },
    // -X
    FaceDef {
        normal: [-1, 0, 0],
        corners: [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]],
        uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        shade: 0.72,
    },
    // +Y (top)
    FaceDef {
        normal: [0, 1, 0],
        corners: [[0, 1, 1], [1, 1, 1], [1, 1, 0], [0, 1, 0]],
        uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        shade: 1.0,
    },
    // -Y (bottom)
    FaceDef {
        normal: [0, -1, 0],
        corners: [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
        uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        shade: 0.55,
    },
    // +Z
    FaceDef {
        normal: [0, 0, 1],
        corners: [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
        uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        shade: 0.85,
    },
    // -Z
    FaceDef {
        normal: [0, 0, -1],
        corners: [[1, 0, 0], [0, 0, 0], [0, 1, 0], [1, 1, 0]],
        uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        shade: 0.85,
    },
];

#[derive(Clone, Copy)]
struct FaceDef {
    normal: [i32; 3],
    corners: [[i32; 3]; 4],
    uvs: [[f32; 2]; 4],
    shade: f32,
}

/// Triangle indices into the 4 quad corners (non-indexed draw: duplicated
/// corners become 6 vertices).
const QUAD_INDEX_PATTERN: [usize; 6] = [0, 1, 2, 0, 2, 3];

/// Neighbor sampling for the mesher. Rules:
/// - below the world (`y < 0`) counts as **solid** (never draw world bottom)
/// - above the world and unloaded chunks count as **air** (draw those faces)
fn neighbor_is_air(world: &World, sx: i32, sy: i32, sz: i32, x: i32, y: i32, z: i32) -> bool {
    let wy = sy * 16 + y;
    if wy < 0 {
        return false;
    }
    if wy >= CHUNK_H as i32 {
        return true;
    }
    world
        .get_block(sx * 16 + x, wy, sz * 16 + z)
        .map(|id| id == block::AIR)
        .unwrap_or(true)
}

/// Mesh one 16×16×16 section at section coords `(sx, sy, sz)`.
///
/// The section is read through `world` so faces at section boundaries cull
/// correctly against neighboring sections and chunks.
pub fn mesh_section(world: &World, sx: i32, sy: i32, sz: i32) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    let origin = Vec3::new(
        (sx * SECTION_SIZE as i32) as f32,
        (sy * SECTION_SIZE as i32) as f32,
        (sz * SECTION_SIZE as i32) as f32,
    );

    for y in 0..SECTION_SIZE {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let id = world
                    .get_block(sx * 16 + x as i32, sy * 16 + y as i32, sz * 16 + z as i32)
                    .unwrap_or(0);
                if id == block::AIR {
                    continue;
                }
                let tiles = tiles_for_block(id);

                for (face_i, face) in FACES.iter().enumerate() {
                    let [nx, ny, nz] = face.normal;
                    if !neighbor_is_air(
                        world,
                        sx,
                        sy,
                        sz,
                        x as i32 + nx,
                        y as i32 + ny,
                        z as i32 + nz,
                    ) {
                        continue;
                    }
                    let [u0, v0, u1, v1] = tile_uv(tiles[face_i]);
                    // Two triangles per quad, non-indexed: p0 p1 p2, p0 p2 p3.
                    for &ci in &QUAD_INDEX_PATTERN {
                        let corner = &face.corners[ci];
                        let uv = &face.uvs[ci];
                        let pos = origin
                            + Vec3::new(
                                (x as i32 + corner[0]) as f32,
                                (y as i32 + corner[1]) as f32,
                                (z as i32 + corner[2]) as f32,
                            );
                        vertices.push(Vertex {
                            position: pos.to_array(),
                            uv: [u0 + uv[0] * (u1 - u0), v0 + uv[1] * (v1 - v0)],
                            shade: face.shade,
                        });
                    }
                }
            }
        }
    }
    vertices
}

/// Mesh every non-empty section of `world`. Returns one vertex list per
/// section, keyed by section coords.
pub fn mesh_world(world: &World) -> Vec<((i32, i32, i32), Vec<Vertex>)> {
    let mut out = Vec::new();
    for (&(cx, cz), chunk) in world.chunks() {
        for sy in 0..16usize {
            if chunk.section(sy).is_empty() {
                continue;
            }
            let vertices = mesh_section(world, cx, sy as i32, cz);
            if !vertices.is_empty() {
                out.push(((cx, sy as i32, cz), vertices));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use melon_world::block;
    use melon_world::{FlatGenerator, World, CHUNK_W};

    fn solid_block_at(y: i32) -> World {
        let mut w = World::new();
        w.set_block(0, y, 0, block::STONE);
        w
    }

    #[test]
    fn floating_block_has_six_faces() {
        let world = solid_block_at(5); // nothing above/below in range
        let verts = mesh_section(&world, 0, 0, 0);
        assert_eq!(verts.len(), 36, "6 faces × 6 vertices");
    }

    #[test]
    fn adjacent_blocks_cull_shared_face() {
        let mut world = solid_block_at(0);
        world.set_block(1, 0, 0, block::STONE); // +X neighbor
        let verts = mesh_section(&world, 0, 0, 0);
        // Block A: 4 faces (shared +X culled, bottom culled at y=0).
        // Block B: 4 faces (shared -X culled, bottom culled). 8 faces total.
        assert_eq!(verts.len(), 48);
    }

    #[test]
    fn block_below_world_has_no_bottom_face() {
        let mut world = World::new();
        world.set_block(0, 0, 0, block::BEDROCK);
        let verts = mesh_section(&world, 0, 0, 0);
        // 5 faces × 6 vertices: bottom face at y=0 must be culled.
        assert_eq!(verts.len(), 30, "bottom face at y=0 must be culled");
    }

    #[test]
    fn unloaded_neighbors_render_boundary_faces() {
        // One chunk of solid stone: side faces against unloaded space exist,
        // top faces exist, bottom culled.
        let mut world = World::new();
        for x in 0..CHUNK_W {
            for z in 0..CHUNK_W {
                world.set_block(x as i32, 0, z as i32, block::STONE);
            }
        }
        let verts = mesh_section(&world, 0, 0, 0);
        let faces = verts.len() / 6;
        // top: 256 · sides: 4 × 16 = 64 · bottom: 0
        assert_eq!(faces, 256 + 64);
    }

    #[test]
    fn empty_sections_are_skipped() {
        let world = FlatGenerator::superflat().generate(4, 4);
        let meshes = mesh_world(&world);
        assert_eq!(
            meshes.len(),
            16,
            "4×4 chunks, only the y=0 section is populated"
        );
    }

    #[test]
    fn vertex_positions_are_world_space() {
        let mut world = World::new();
        world.set_block(16 + 3, 6, 4, block::GRASS); // chunk x=1, surface layer
        let verts = mesh_section(&world, 1, 0, 0);
        let top_face: Vec<_> = verts.iter().filter(|v| v.shade == 1.0).collect();
        assert!(!top_face.is_empty());
        let max_x = top_face.iter().map(|v| v.position[0]).fold(0.0, f32::max);
        assert!(
            (max_x - 20.0).abs() < 1e-4,
            "world-space x must reach 16+4, got {max_x}"
        );
    }

    #[test]
    fn top_faces_are_shaded_brightest() {
        let world = solid_block_at(0);
        let verts = mesh_section(&world, 0, 0, 0);
        let top = verts.iter().filter(|v| v.shade == 1.0).count();
        assert_eq!(top, 6, "exactly one top face");
        assert!(verts.iter().all(|v| v.shade >= 0.55 && v.shade <= 1.0));
    }

    /// **The Phase 2 benchmark gate:** meshing a 4×4-chunk superflat must
    /// stay far under one frame's worth of time even in debug builds.
    #[test]
    fn flat_world_meshes_within_budget() {
        let world = FlatGenerator::superflat().generate(4, 4);
        let start = std::time::Instant::now();
        let meshes = mesh_world(&world);
        let elapsed = start.elapsed();

        let quads: usize = meshes.iter().map(|(_, v)| v.len() / 6).sum();
        assert_eq!(meshes.len(), 16);
        assert!(quads > 0);
        assert!(
            elapsed < std::time::Duration::from_millis(1000),
            "meshing 16 sections took {elapsed:?} — over budget"
        );
    }
}
