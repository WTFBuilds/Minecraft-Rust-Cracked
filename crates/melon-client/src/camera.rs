//! Fly camera — position + yaw/pitch with world-space movement.

use glam::{Mat4, Vec3};

const SPEED: f32 = 10.0; // blocks per second
const LOOK_SENSITIVITY: f32 = 0.0032;
/// Just under 90° so the forward vector never degenerates.
const PITCH_LIMIT: f32 = 1.55;

/// Spawn high above the superflat, looking down across the world.
pub const SPAWN: (f32, f32, f32) = (8.0, 14.0, 28.0);
pub const SPAWN_YAW: f32 = -std::f32::consts::FRAC_PI_2;
pub const SPAWN_PITCH: f32 = -0.45;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    /// Rotation around the Y axis; +yaw turns right.
    pub yaw: f32,
    /// Look up/down; clamped so forward never degenerates.
    pub pitch: f32,
}

/// Per-frame movement intent (from the keyboard, in `[−1, 1]` ranges).
#[derive(Debug, Clone, Copy, Default)]
pub struct MoveInput {
    /// W/S — forward/back.
    pub forward: f32,
    /// D/A — right/left.
    pub strafe: f32,
    /// Space/Shift — up/down.
    pub vertical: f32,
}

impl Camera {
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            yaw,
            pitch,
        }
    }

    /// Unit forward vector including pitch.
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.pitch.cos() * self.yaw.cos(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.sin(),
        )
    }

    /// Horizontal forward (movement plane).
    fn forward_h(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin())
    }

    pub fn look(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * LOOK_SENSITIVITY;
        self.pitch = (self.pitch - dy * LOOK_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    pub fn apply_move(&mut self, dt: f32, input: MoveInput) {
        let f = self.forward_h();
        // Screen-right = forward × up (matches look_at's right vector).
        let right = Vec3::new(-f.z, 0.0, f.x);
        let mut delta = f * input.forward + right * input.strafe;
        if delta.length_squared() > 1.0 {
            delta = delta.normalize();
        }
        delta.y = input.vertical;
        self.position += delta * SPEED * dt;
    }

    /// View-projection for the given viewport aspect.
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let view = Mat4::look_at_rh(self.position, self.position + self.forward(), Vec3::Y);
        let proj = Mat4::perspective_rh(70.0_f32.to_radians(), aspect, 0.1, 500.0);
        proj * view
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_zero_faces_plus_x() {
        let c = Camera::new(Vec3::ZERO, 0.0, 0.0);
        let f = c.forward();
        assert!((f.x - 1.0).abs() < 1e-6 && f.y.abs() < 1e-6 && f.z.abs() < 1e-6);
    }

    #[test]
    fn pitch_positive_looks_up() {
        let c = Camera::new(Vec3::ZERO, 0.0, 0.5);
        assert!(c.forward().y > 0.0);
    }

    #[test]
    fn look_clamps_pitch() {
        let mut c = Camera::new(Vec3::ZERO, 0.0, 0.0);
        c.look(0.0, -1_000_000.0); // drag up forever
        assert!((c.pitch - PITCH_LIMIT).abs() < 1e-4, "pitch must clamp");
        c.look(0.0, 1_000_000.0);
        assert!((c.pitch + PITCH_LIMIT).abs() < 1e-4);
    }

    #[test]
    fn forward_move_moves_along_heading() {
        let mut c = Camera::new(Vec3::ZERO, 0.0, 0.0);
        c.apply_move(
            1.0,
            MoveInput {
                forward: 1.0,
                ..Default::default()
            },
        );
        assert!((c.position.x - SPEED).abs() < 1e-4);
        assert_eq!(c.position.y, 0.0);
        assert_eq!(c.position.z, 0.0);
    }

    #[test]
    fn diagonal_move_is_normalized() {
        let mut c = Camera::new(Vec3::ZERO, 0.0, 0.0);
        c.apply_move(
            1.0,
            MoveInput {
                forward: 1.0,
                strafe: 1.0,
                ..Default::default()
            },
        );
        let dist = c.position.length();
        assert!(
            (dist - SPEED).abs() < 1e-3,
            "W+D must not move faster than W"
        );
    }

    #[test]
    fn view_proj_projects_forward_point_in_front() {
        let c = Camera::new(Vec3::ZERO, 0.0, 0.0);
        let vp = c.view_proj(1.0);
        let p = vp * Vec3::new(10.0, 0.0, 0.0).extend(1.0);
        assert!(p.w > 0.0, "point ahead must be in front of the camera");
    }
}
