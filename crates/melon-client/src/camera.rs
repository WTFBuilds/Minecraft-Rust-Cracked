//! Camera — eye position + yaw/pitch (movement is handled by the
//! `melon-world` collision module against the player simulation).

use glam::{Mat4, Vec3};

const LOOK_SENSITIVITY: f32 = 0.0032;
/// Just under 90° so the forward vector never degenerates.
const PITCH_LIMIT: f32 = 1.55;

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

    pub fn look(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * LOOK_SENSITIVITY;
        self.pitch = (self.pitch - dy * LOOK_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
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
    fn view_proj_projects_forward_point_in_front() {
        let c = Camera::new(Vec3::ZERO, 0.0, 0.0);
        let vp = c.view_proj(1.0);
        let p = vp * Vec3::new(10.0, 0.0, 0.0).extend(1.0);
        assert!(p.w > 0.0, "point ahead must be in front of the camera");
    }
}
