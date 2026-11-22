//! Voxel raycasting (Amanatides & Woo DDA) — used by the client to pick
//! blocks and later by the server for hit validation.

use crate::block;
use crate::chunk::World;

/// A solid voxel hit by a ray.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RayHit {
    /// Voxel the ray entered (the block you'd break).
    pub block: [i32; 3],
    /// Normal of the face that was crossed (the direction you'd place in).
    pub face: [i32; 3],
}

impl RayHit {
    /// The empty voxel in front of the hit face (the block you'd place).
    pub fn neighbor(&self) -> [i32; 3] {
        [
            self.block[0] + self.face[0],
            self.block[1] + self.face[1],
            self.block[2] + self.face[2],
        ]
    }
}

/// March `origin` along `dir` (normalized internally) up to `max_dist`,
/// returning the first solid voxel and the face it was entered through.
///
/// Returns `None` if nothing solid is hit within `max_dist`, or if the ray
/// starts inside a solid voxel (nothing sensible to pick there).
pub fn raycast(world: &World, origin: [f32; 3], dir: [f32; 3], max_dist: f32) -> Option<RayHit> {
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if len <= f32::EPSILON {
        return None;
    }
    let dir = [dir[0] / len, dir[1] / len, dir[2] / len];

    // Starting voxel.
    let mut voxel = [
        origin[0].floor() as i32,
        origin[1].floor() as i32,
        origin[2].floor() as i32,
    ];
    if world.get_block(voxel[0], voxel[1], voxel[2]).unwrap_or(0) != block::AIR {
        return None; // started inside a solid block
    }

    // Per-axis step direction and t-parameters.
    let step = [
        if dir[0] > 0.0 { 1i32 } else { -1i32 },
        if dir[1] > 0.0 { 1i32 } else { -1i32 },
        if dir[2] > 0.0 { 1i32 } else { -1i32 },
    ];
    let axis_t_max = |o: f32, d: f32, v: i32| -> f32 {
        if d > 0.0 {
            (v as f32 + 1.0 - o) / d
        } else if d < 0.0 {
            (o - v as f32) / -d
        } else {
            f32::INFINITY
        }
    };
    let mut t_max = [
        axis_t_max(origin[0], dir[0], voxel[0]),
        axis_t_max(origin[1], dir[1], voxel[1]),
        axis_t_max(origin[2], dir[2], voxel[2]),
    ];
    let t_delta = [
        if dir[0] != 0.0 {
            1.0 / dir[0].abs()
        } else {
            f32::INFINITY
        },
        if dir[1] != 0.0 {
            1.0 / dir[1].abs()
        } else {
            f32::INFINITY
        },
        if dir[2] != 0.0 {
            1.0 / dir[2].abs()
        } else {
            f32::INFINITY
        },
    ];

    // The face normal is minus the direction of the last axis step.
    loop {
        // Next boundary crossing = the axis with the smallest t_max.
        let axis = if t_max[0] <= t_max[1] && t_max[0] <= t_max[2] {
            0
        } else if t_max[1] <= t_max[2] {
            1
        } else {
            2
        };
        if t_max[axis] > max_dist {
            return None; // ran out of distance
        }
        voxel[axis] += step[axis];
        t_max[axis] += t_delta[axis];

        if world.get_block(voxel[0], voxel[1], voxel[2]).unwrap_or(0) != block::AIR {
            let mut face = [0i32; 3];
            face[axis] = -step[axis];
            return Some(RayHit { block: voxel, face });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FlatGenerator;

    fn flat_world() -> World {
        FlatGenerator::superflat().generate(4, 4)
    }

    #[test]
    fn ray_down_hits_grass_top_face() {
        let world = flat_world();
        // Straight down from 3 blocks above the surface (grass top at y=5,
        // its top face at y=6).
        let hit =
            raycast(&world, [8.5, 9.0, 8.5], [0.0, -1.0, 0.0], 20.0).expect("must hit the ground");
        assert_eq!(hit.block, [8, 5, 8], "hits the grass block");
        assert_eq!(hit.face, [0, 1, 0], "entered through its top face");
        assert_eq!(hit.neighbor(), [8, 6, 8], "placement voxel above it");
    }

    #[test]
    fn ray_sideways_hits_side_face() {
        let mut world = World::new();
        world.set_block(3, 5, 3, block::STONE);
        let hit = raycast(&world, [0.5, 5.5, 3.5], [1.0, 0.0, 0.0], 20.0)
            .expect("must hit the stone block");
        assert_eq!(hit.block, [3, 5, 3]);
        assert_eq!(hit.face, [-1, 0, 0], "entered through its -x face");
        assert_eq!(hit.neighbor(), [2, 5, 3]);
    }

    #[test]
    fn ray_stops_at_max_dist() {
        let world = flat_world();
        assert!(
            raycast(&world, [8.5, 9.0, 8.5], [0.0, -1.0, 0.0], 2.0).is_none(),
            "ground is 3+ blocks away — 2.0 range must miss"
        );
        assert!(raycast(&world, [8.5, 9.0, 8.5], [0.0, -1.0, 0.0], 4.0).is_some());
    }

    #[test]
    fn ray_into_sky_misses() {
        let world = flat_world();
        assert!(raycast(&world, [8.5, 9.0, 8.5], [0.0, 1.0, 0.0], 50.0).is_none());
    }

    #[test]
    fn ray_starting_inside_solid_returns_none() {
        let world = flat_world();
        assert!(
            raycast(&world, [8.5, 5.5, 8.5], [0.0, -1.0, 0.0], 10.0).is_none(),
            "starting inside the grass block gives nothing to pick"
        );
    }

    #[test]
    fn zero_direction_is_none() {
        let world = flat_world();
        assert!(raycast(&world, [8.5, 9.0, 8.5], [0.0, 0.0, 0.0], 10.0).is_none());
    }

    #[test]
    fn diagonal_ray_hits_top_face() {
        let world = flat_world();
        // 45° down toward +x: enters a block from above or the side depending
        // on geometry — assert only that we hit the grass layer at y=5.
        let hit =
            raycast(&world, [8.5, 9.0, 8.5], [0.7, -0.7, 0.0], 20.0).expect("must hit something");
        assert_eq!(hit.block[1], 5);
    }
}
