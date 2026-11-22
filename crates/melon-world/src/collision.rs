//! Player physics: AABB-vs-voxel collision with per-axis resolution.
//!
//! The player is a 0.6×1.8×0.6 box whose `feet` position is the bottom-center
//! voxel coordinate. Movement resolves one axis at a time (Y, then X, then
//! Z) so sliding along walls works naturally. Shared by the client (local
//! simulation) and later by the server (movement clamping in Phase 6).

use crate::block;
use crate::chunk::World;

/// Player collision box width/depth (blocks).
pub const PLAYER_WIDTH: f32 = 0.6;
/// Player collision box height (blocks).
pub const PLAYER_HEIGHT: f32 = 1.8;
/// Eye height above the feet position (blocks).
pub const PLAYER_EYE_HEIGHT: f32 = 1.62;
/// Downward acceleration (blocks/s²) — Minecraft-like.
pub const GRAVITY: f32 = 32.0;
/// Upward velocity applied by a jump (blocks/s) — clears a 1.25-block step.
pub const JUMP_VELOCITY: f32 = 9.0;
/// Ground walking speed (blocks/s).
pub const WALK_SPEED: f32 = 4.317;
/// Terminal fall speed (blocks/s).
pub const TERMINAL_VELOCITY: f32 = 50.0;

/// Half the player width, precomputed.
const HALF_WIDTH: f32 = PLAYER_WIDTH / 2.0;

/// Whether a voxel blocks player movement (everything except air for now).
pub fn is_solid(world: &World, x: i32, y: i32, z: i32) -> bool {
    world.get_block(x, y, z).unwrap_or(block::AIR) != block::AIR
}

/// The voxel range a player AABB overlaps: `(min, max)` inclusive per axis.
fn aabb_voxel_range(feet: [f32; 3]) -> ([i32; 3], [i32; 3]) {
    let min = [feet[0] - HALF_WIDTH, feet[1], feet[2] - HALF_WIDTH];
    let max = [
        feet[0] + HALF_WIDTH,
        feet[1] + PLAYER_HEIGHT,
        feet[2] + HALF_WIDTH,
    ];
    let min_v = [
        min[0].floor() as i32,
        min[1].floor() as i32,
        min[2].floor() as i32,
    ];
    // `- 1e-4` keeps an AABB exactly flush with a voxel face from touching
    // the next voxel over.
    let max_v = [
        (max[0] - 1e-4).floor() as i32,
        (max[1] - 1e-4).floor() as i32,
        (max[2] - 1e-4).floor() as i32,
    ];
    (min_v, max_v)
}

/// Does the player box at `feet` intersect any solid voxel?
fn collides(world: &World, feet: [f32; 3]) -> bool {
    let (min, max) = aabb_voxel_range(feet);
    for y in min[1]..=max[1] {
        for z in min[2]..=max[2] {
            for x in min[0]..=max[0] {
                if is_solid(world, x, y, z) {
                    return true;
                }
            }
        }
    }
    false
}

/// The result of one physics step.
#[derive(Debug, Clone, Copy)]
pub struct StepResult {
    /// New feet position (bottom-center of the player box).
    pub feet: [f32; 3],
    /// Updated velocity (gravity applied, axes zeroed on collision).
    pub velocity: [f32; 3],
    /// `true` when the player is standing on solid ground.
    pub on_ground: bool,
}

/// Integrate one physics step for a walking player:
/// gravity → Y move + resolve → X move + resolve → Z move + resolve.
///
/// `wish` is the desired horizontal velocity (already includes speed and
/// direction); `jumping` triggers a jump when standing on ground.
pub fn step(
    world: &World,
    feet: [f32; 3],
    velocity: [f32; 3],
    wish: [f32; 2],
    jumping: bool,
    dt: f32,
) -> StepResult {
    let mut feet = feet;
    let mut vel = velocity;

    // Horizontal: direct velocity (Minecraft-flavoured responsiveness; real
    // inertia can come later without touching collision).
    vel[0] = wish[0];
    vel[2] = wish[1];

    // Vertical: gravity + jump impulse.
    if jumping && velocity[1] >= 0.0 && collides(world, [feet[0], feet[1] - 0.05, feet[2]]) {
        vel[1] = JUMP_VELOCITY;
    } else {
        vel[1] = (vel[1] - GRAVITY * dt).max(-TERMINAL_VELOCITY);
    }

    let mut on_ground = false;

    // --- Y axis ---
    let dy = vel[1] * dt;
    feet[1] += dy;
    if collides(world, feet) {
        if dy < 0.0 {
            // Landed: snap feet to the top of whatever is below.
            feet[1] = feet[1].floor() + 1.0;
            on_ground = true;
        } else {
            // Head bump: clamp to the underside of the ceiling voxel.
            feet[1] = feet[1].ceil() - PLAYER_HEIGHT - 1e-3;
        }
        vel[1] = 0.0;
    }

    // --- X axis ---
    let dx = vel[0] * dt;
    feet[0] += dx;
    if collides(world, feet) {
        feet[0] -= dx;
        vel[0] = 0.0;
    }

    // --- Z axis ---
    let dz = vel[2] * dt;
    feet[2] += dz;
    if collides(world, feet) {
        feet[2] -= dz;
        vel[2] = 0.0;
    }

    // Ground check: a hair below the feet must be solid when resting.
    if !on_ground && vel[1] <= 0.0 {
        on_ground = collides(world, [feet[0], feet[1] - 0.05, feet[2]]);
    }

    StepResult {
        feet,
        velocity: vel,
        on_ground,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FlatGenerator;

    const START: [f32; 3] = [8.5, 8.0, 8.5]; // 2 blocks above the grass top (y=6)

    #[test]
    fn falling_lands_on_grass() {
        let world = FlatGenerator::superflat().generate(4, 4);
        let mut feet = START;
        let mut vel = [0.0, 0.0, 0.0];
        let mut on_ground = false;
        // Simulate up to 2 seconds.
        for _ in 0..200 {
            let r = step(&world, feet, vel, [0.0, 0.0], false, 0.016);
            feet = r.feet;
            vel = r.velocity;
            on_ground = r.on_ground;
            if on_ground {
                break;
            }
        }
        assert!(on_ground, "must land");
        assert!(
            (feet[1] - 6.0).abs() < 1e-3,
            "feet rest on the grass top (y=6), got {}",
            feet[1]
        );
    }

    #[test]
    fn jump_clears_one_block_and_lands_again() {
        let world = FlatGenerator::superflat().generate(4, 4);
        let mut feet = [8.5, 6.0, 8.5]; // standing on grass
        let mut vel = [0.0, 0.0, 0.0];
        // Jump.
        let r = step(&world, feet, vel, [0.0, 0.0], true, 0.016);
        feet = r.feet;
        vel = r.velocity;
        assert!(vel[1] > 0.0, "jump applies upward velocity");
        // Fly through the arc until we land again.
        let mut max_y = feet[1];
        let mut landed = false;
        for _ in 0..300 {
            let r = step(&world, feet, vel, [0.0, 0.0], false, 0.016);
            feet = r.feet;
            vel = r.velocity;
            max_y = max_y.max(feet[1]);
            if r.on_ground && feet[1] <= 6.0 + 1e-3 {
                landed = true;
                break;
            }
        }
        assert!(landed, "must land back on the grass");
        assert!(
            max_y > 7.0 && max_y < 7.5,
            "jump apex ~1.25 blocks, got {max_y}"
        );
    }

    #[test]
    fn walls_stop_horizontal_movement() {
        let mut world = FlatGenerator::superflat().generate(4, 4);
        // Wall two blocks tall directly +x of the spawn column.
        world.set_block(12, 6, 8, block::STONE);
        world.set_block(12, 7, 8, block::STONE);

        let mut feet = [8.5, 6.0, 8.5]; // standing on the grass
        let mut vel = [0.0, 0.0, 0.0];
        // Walk +x for a second.
        for _ in 0..100 {
            let r = step(&world, feet, vel, [WALK_SPEED, 0.0], false, 0.016);
            feet = r.feet;
            vel = r.velocity;
        }
        // Wall voxel x=12 spans 12.0..13.0; the player's +x face is at
        // feet.x + 0.3 → must be clamped just shy of 11.7.
        assert!(
            feet[0] < 11.75,
            "player must stop before the wall, got {}",
            feet[0]
        );
        assert!(feet[0] > 9.0, "but must have moved some distance");
    }

    #[test]
    fn walking_on_flat_ground_stays_level() {
        let world = FlatGenerator::superflat().generate(4, 4);
        let mut feet = [8.5, 6.0, 8.5];
        let mut vel = [0.0, 0.0, 0.0];
        for _ in 0..100 {
            let r = step(&world, feet, vel, [WALK_SPEED, 0.0], false, 0.016);
            feet = r.feet;
            vel = r.velocity;
        }
        assert!(
            (feet[1] - 6.0).abs() < 1e-2,
            "walking on flat ground must not sink or float: {}",
            feet[1]
        );
    }

    #[test]
    fn cannot_walk_off_into_the_void() {
        // A 1×1-chunk world: walking past its edge must still collide with
        // the boundary columns (get_block returns None → not solid, so the
        // player *can* fall off the edge — that is correct Minecraft-like
        // behaviour). This test documents the intent instead.
        let world = FlatGenerator::superflat().generate(1, 1);
        let mut feet = [15.5, 6.0, 8.5];
        let mut vel = [0.0, 0.0, 0.0];
        for _ in 0..100 {
            let r = step(&world, feet, vel, [WALK_SPEED, 0.0], false, 0.016);
            feet = r.feet;
            vel = r.velocity;
        }
        assert!(feet[0] > 16.0, "walked past the loaded edge");
        assert!(
            feet[1] < 6.0,
            "then fell off (no collision beyond loaded chunks)"
        );
    }
}
