use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy_egui::EguiContexts;

#[derive(Component)]
pub struct OrbitCamera {
    pub focus: Vec3,
    pub radius: f32,
    pub min_radius: f32,
    pub max_radius: f32,
}

impl OrbitCamera {
    pub fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            radius: 50.0,
            min_radius: 2.0,
            max_radius: 100.0,
        }
    }

    #[allow(dead_code)]
    pub fn new_isometric() -> Self {
        Self {
            focus: Vec3::ZERO,
            radius: 50.0,
            min_radius: 2.0,
            max_radius: 100.0,
        }
    }

    #[allow(dead_code)]
    pub fn reset_to_home(&mut self, transform: &mut Transform) {
        self.focus = Vec3::ZERO;
        self.radius = 10.0;

        // Set to isometric-style angle
        let distance = self.radius;
        let angle = std::f32::consts::PI / 4.0; // 45 degrees
        let height = distance * 0.5; // Slightly above the scene

        transform.translation = Vec3::new(distance * angle.cos(), height, distance * angle.sin());
        transform.look_at(self.focus, Vec3::Y);
    }
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self::default()
    }
}

pub fn orbit_camera(
    windows: Query<&Window>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut OrbitCamera)>,
    _grabbed: Local<bool>,
    mut contexts: EguiContexts,
) {
    let _window = windows.single();

    // Skip camera controls if the mouse is over egui UI
    if contexts.ctx_mut().is_pointer_over_area() {
        return;
    }

    for (mut transform, mut orbit) in query.iter_mut() {
        // Handle zooming with mouse wheel (reduced sensitivity)
        for ev in ev_scroll.read() {
            let zoom_sensitivity = 0.2;
            orbit.radius =
                (orbit.radius - ev.y * zoom_sensitivity).clamp(orbit.min_radius, orbit.max_radius);

            // Update camera position while maintaining current angles
            let forward = -(transform.translation - orbit.focus).normalize();
            transform.translation = orbit.focus - forward * orbit.radius;
        }

        // Handle rotation (Command/Super + left click)
        let is_rotating = mouse.pressed(MouseButton::Left)
            && (keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight))
            && !keys.pressed(KeyCode::ShiftLeft)
            && !keys.pressed(KeyCode::ShiftRight);

        if is_rotating {
            let mut delta = Vec2::ZERO;
            for ev in ev_motion.read() {
                delta += ev.delta;
            }

            let sensitivity = 0.5;

            // Rotate around global Y axis
            let rot = Quat::from_rotation_y(-delta.x * sensitivity * 0.01);
            transform.translation = rot * (transform.translation - orbit.focus) + orbit.focus;

            // Rotate around local X axis
            let right = transform.rotation * Vec3::X;
            let rot = Quat::from_axis_angle(right, -delta.y * sensitivity * 0.01);
            transform.translation = rot * (transform.translation - orbit.focus) + orbit.focus;
        }

        // Handle panning (Shift + left click)
        let is_panning = mouse.pressed(MouseButton::Left)
            && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight));

        if is_panning {
            let mut delta = Vec2::ZERO;
            for ev in ev_motion.read() {
                delta += ev.delta;
            }

            let sensitivity = 0.005 * orbit.radius; // Scale pan speed with zoom level

            // Get camera right and up vectors
            let right = transform.rotation * Vec3::X;
            let up = transform.rotation * Vec3::Y;

            // Move both camera and focus point
            let translation = right * (-delta.x * sensitivity) + up * (delta.y * sensitivity);
            transform.translation += translation;
            orbit.focus += translation;
        }

        // Always look at focus point
        transform.look_at(orbit.focus, Vec3::Y);
    }
}

pub fn setup_isometric_camera(
    mut query: Query<(&mut Transform, &OrbitCamera), Added<OrbitCamera>>,
) {
    for (mut transform, orbit) in query.iter_mut() {
        let distance = orbit.radius;
        let angle = std::f32::consts::PI / 4.0; // 45 degrees
        let height = distance * 0.5; // Camera height = 50% of distance

        transform.translation = Vec3::new(distance * angle.cos(), height, distance * angle.sin());
        transform.look_at(orbit.focus, Vec3::Y);
    }
}
