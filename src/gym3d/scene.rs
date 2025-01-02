use bevy::{
    prelude::*,
    render::{render_resource::{AsBindGroup, ShaderRef}, view::ViewUniform},
    reflect::TypePath,
};
use crate::gym3d::camera::OrbitCamera;
use crate::Config;

/// Material for rendering an infinite grid with customizable scale and line width.
/// Used for creating a visual reference plane in 3D space.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct InfiniteGridMaterial {
    #[uniform(0)]
    grid_scale: f32,
    #[uniform(1)]
    line_width: f32,
    #[uniform(2)]
    view: ViewUniform,
}

impl Material for InfiniteGridMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/infinite_grid.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/infinite_grid.wgsl".into()
    }
}

impl Default for InfiniteGridMaterial {
    fn default() -> Self {
        Self {
            grid_scale: 0.5,
            line_width: 0.1,
            view: unsafe { std::mem::zeroed() },
        }
    }
}

#[derive(Component)]
pub struct InfinitePlane;

/// Updates the position of the infinite plane to follow the camera's X and Z coordinates.
/// This creates the illusion of an infinite grid extending to the horizon.
/// 
/// # Arguments
/// * `plane_query` - Query for the infinite plane's transform
/// * `camera_query` - Query for the camera's transform
pub fn update_infinite_plane(
    mut plane_query: Query<&mut Transform, With<InfinitePlane>>,
    camera_query: Query<&Transform, (With<Camera>, Without<InfinitePlane>)>,
) {
    let Ok(camera_transform) = camera_query.get_single() else { return };
    let Ok(mut plane_transform) = plane_query.get_single_mut() else { return };

    // Get camera position
    let camera_pos = camera_transform.translation;
    
    // Update plane position to follow camera (only X and Z, keep Y at 0)
    plane_transform.translation.x = camera_pos.x;
    plane_transform.translation.z = camera_pos.z;
}

/// Creates a basic 3D scene with an infinite grid floor and camera setup.
/// 
/// # Arguments
/// * `commands` - Commands for entity creation
/// * `meshes` - Asset storage for meshes
/// * `materials` - Asset storage for InfiniteGridMaterial
fn create_scene(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<InfiniteGridMaterial>>,
) {
    // Debug prints
    println!("Creating scene...");

    // Set background color to black
    commands.insert_resource(ClearColor(Color::BLACK));

    let floor_material = materials.add(InfiniteGridMaterial {
        grid_scale: 2.0,
        line_width: 0.05,
        view: unsafe { std::mem::zeroed() },
    });
    println!("Material created: {:?}", floor_material);

    let mesh = meshes.add(Plane3d::default().mesh().size(1000.0, 1000.0));
    println!("Mesh created: {:?}", mesh);

    // Spawn closer to camera
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(0.0, -1.0, 0.0),
        InfinitePlane,
        Name::new("Infinite Floor"),
    ));

    // Move camera closer and look down
    commands.spawn((
        Camera3d::default(),

        Transform::from_xyz(0.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
        OrbitCamera::default(),
    ));
}

/// Initializes the 3D scene and sets up the camera position.
/// This is typically called during the initial setup phase.
/// 
/// # Arguments
/// * `commands` - Commands for entity creation
/// * `meshes` - Asset storage for meshes
/// * `materials` - Asset storage for InfiniteGridMaterial
/// * `camera` - Query for accessing the camera transform and orbit settings
pub fn initialize_scene_with_camera(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<InfiniteGridMaterial>>,
    mut camera: Query<(&mut Transform, &OrbitCamera)>,
) {
    // Camera setup
    if let Ok((mut transform, _)) = camera.get_single_mut() {
        *transform = Transform::from_xyz(0.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y);
    }

    create_scene(&mut commands, &mut meshes, &mut materials);
}

/// Creates a basic scene with a simple colored floor using StandardMaterial.
/// This is an alternative to the infinite grid setup.
/// 
/// # Arguments
/// * `commands` - Commands for entity creation
/// * `meshes` - Asset storage for meshes
/// * `materials` - Asset storage for StandardMaterial
pub fn create_scene_with_basic_floor(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Basic floor setup with StandardMaterial
    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.3, 0.8),
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1000.0, 1000.0))),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("Floor"),
    ));
}

/// Updates the 3D scene based on configuration changes.
/// Either clears the entire scene or reinitializes it with basic components.
/// 
/// # Arguments
/// * `new_config` - New configuration settings
/// * `commands` - Commands for entity manipulation
/// * `camera_query` - Query for finding camera entities
/// * `light_query` - Query for finding light entities
/// * `mesh_query` - Query for finding mesh entities
/// * `_asset_server` - Asset server (currently unused)
/// * `meshes` - Asset storage for meshes
/// * `materials` - Asset storage for materials
pub fn handle_3d_scene_update(
    new_config: &Config,
    commands: &mut Commands,
    camera_query: &Query<Entity, With<Camera3d>>,
    light_query: &Query<Entity, With<PointLight>>,
    mesh_query: &Query<Entity, With<Mesh3d>>,
    _asset_server: &Res<AssetServer>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    if !new_config.layout.show_3d_scene {
        // Clear 3D scene
        for camera_entity in camera_query.iter() {
            commands.entity(camera_entity).despawn_recursive();
        }
        for light_entity in light_query.iter() {
            commands.entity(light_entity).despawn_recursive();
        }
        for entity in mesh_query.iter() {
            commands.entity(entity).despawn_recursive();
        }
    } else {
        // Reinitialize the 3D scene
        create_scene_with_basic_floor(
            commands,
            meshes,
            materials,
        );
    }
}
