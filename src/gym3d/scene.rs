use crate::executors::streaming::StreamManager;
use crate::gym3d::camera::OrbitCamera;
use crate::Config;
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::mesh::Indices,
    render::render_asset::RenderAssetUsages,
    render::render_resource::PrimitiveTopology,
    render::{
        render_resource::{AsBindGroup, ShaderRef},
        view::ViewUniform,
    },
};

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
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    let Ok(mut plane_transform) = plane_query.get_single_mut() else {
        return;
    };

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
/// * `standard_materials` - Asset storage for StandardMaterial
fn create_scene(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<InfiniteGridMaterial>>,
    standard_materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Debug prints
    debug!("Creating scene...");

    // Set background color to black
    commands.insert_resource(ClearColor(Color::BLACK));

    let floor_material = materials.add(InfiniteGridMaterial {
        grid_scale: 2.0,
        line_width: 0.05,
        view: unsafe { std::mem::zeroed() },
    });
    debug!("Material created: {:?}", floor_material);

    let mesh = meshes.add(Plane3d::default().mesh().size(1000.0, 1000.0));
    debug!("Mesh created: {:?}", mesh);

    // Spawn closer to camera
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(0.0, -1.0, 0.0),
        InfinitePlane,
        Name::new("Infinite Floor"),
    ));

    const CUBE_LENGTH: f32 = 1.0;

    // Add glowing white cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(CUBE_LENGTH, CUBE_LENGTH, CUBE_LENGTH))),
        MeshMaterial3d(standard_materials.add(StandardMaterial {
            base_color: Color::WHITE,
            emissive: Color::WHITE.into(),
            ..default()
        })),
        Transform::from_xyz(0.0, CUBE_LENGTH / 2.0, 0.0),
        Name::new("Glowing Cube"),
        PositionedCube,
    ));

    // Add initial trail (empty)
    commands.spawn((
        Mesh3d(meshes.add(create_line_mesh(&[]))),
        MeshMaterial3d(standard_materials.add(StandardMaterial {
            base_color: Color::WHITE,
            emissive: Color::WHITE.into(),
            unlit: true,
            ..default()
        })),
        Transform::default(),
        CubeTrail,
        Name::new("Cube Trail"),
    ));

    // Move camera closer and look down
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
        OrbitCamera::default(),
    ));

    // Add spotlight at 3 o'clock position
    commands.spawn((
        SpotLight {
            intensity: 10000000.0,
            color: Color::WHITE,
            range: 50.0,
            radius: 1.0,
            outer_angle: 0.8,
            inner_angle: 0.6,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(5.0, 5.0, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));
}

/// Initializes the 3D scene and sets up the camera position.
/// This is typically called during the initial setup phase.
///
/// # Arguments
/// * `commands` - Commands for entity creation
/// * `meshes` - Asset storage for meshes
/// * `materials` - Asset storage for InfiniteGridMaterial
/// * `standard_materials` - Asset storage for StandardMaterial
/// * `camera` - Query for accessing the camera transform and orbit settings
pub fn initialize_scene_with_camera(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<InfiniteGridMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut camera: Query<(&mut Transform, &OrbitCamera)>,
) {
    // Camera setup
    if let Ok((mut transform, _)) = camera.get_single_mut() {
        *transform = Transform::from_xyz(0.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y);
    }

    create_scene(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut standard_materials,
    );
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
        create_scene_with_basic_floor(commands, meshes, materials);
    }
}

#[derive(Component)]
pub struct PositionedCube;

#[derive(Component)]
pub struct CubeTrail;

/// Creates a line mesh from points
fn create_line_mesh(points: &[Vec3]) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::LineStrip,
        RenderAssetUsages::RENDER_WORLD,
    );

    // Handle empty case
    if points.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0, 0.0, 0.0]; 2]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0, 0.0, 0.0, 1.0]; 2]);
        mesh.insert_indices(Indices::U32(vec![0, 1]));
        return mesh;
    }

    // Convert points to arrays for mesh
    let positions: Vec<[f32; 3]> = points.iter().map(|p| p.to_array()).collect();

    // Find min and max heights for normalization
    let min_height = points
        .iter()
        .map(|p| p.y)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);
    let max_height = points
        .iter()
        .map(|p| p.y)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(1.0);
    let height_range = max_height - min_height;

    // Create colors based on height
    let colors: Vec<[f32; 4]> = points
        .iter()
        .map(|p| {
            let t = if height_range == 0.0 {
                0.0
            } else {
                (p.y - min_height) / height_range
            };

            // Interpolate between colors: blue (low) -> green (middle) -> red (high)
            if t < 0.5 {
                let t2 = t * 2.0;
                [0.0, t2, 1.0 - t2, 1.0] // blue to green
            } else {
                let t2 = (t - 0.5) * 2.0;
                [t2, 1.0 - t2, 0.0, 1.0] // green to red
            }
        })
        .collect();

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

    let indices: Vec<u32> = (0..points.len() as u32).collect();
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

/// Updates the position of the cube and its trail.
///
/// # Arguments
/// * `stream_manager` - Stream manager for accessing flight_position stream
/// * `cube_query` - Query for the cube's transform
/// * `trail_query` - Query for the trail's mesh and material
/// * `meshes` - Asset storage for meshes
pub fn update_cube_position(
    stream_manager: Res<StreamManager>,
    mut cube_query: Query<&mut Transform, With<PositionedCube>>,
    mut trail_query: Query<(&mut Mesh3d, &MeshMaterial3d<StandardMaterial>), With<CubeTrail>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if let Ok(streams) = stream_manager.streams.lock() {
        if let Some(points) = streams.get("flight_position") {
            if let Ok(mut transform) = cube_query.get_single_mut() {
                if let Some(last_point) = points.last() {
                    // Expect [lat, lon, alt, yaw, pitch, roll]
                    if let Some([lat, lon, alt, yaw, pitch, roll]) = last_point.as_flight_data() {
                        // Update position
                        let new_x = lat as f32;
                        let new_y = alt as f32;
                        let new_z = lon as f32;
                        transform.translation = Vec3::new(new_x, new_y, new_z);

                        // Update rotation (convert angles from degrees to radians)
                        let yaw_rad = (yaw as f32).to_radians();
                        let pitch_rad = (pitch as f32).to_radians();
                        let roll_rad = (roll as f32).to_radians();

                        // Create rotation quaternion using yaw (y-axis), pitch (x-axis), and roll (z-axis)
                        transform.rotation =
                            Quat::from_euler(EulerRot::YXZ, yaw_rad, pitch_rad, roll_rad);
                    }
                }

                // Update trail with all points
                if let Ok((mut trail_mesh, _)) = trail_query.get_single_mut() {
                    let trail_points: Vec<Vec3> = points
                        .iter()
                        .filter_map(|point| point.as_flight_data())
                        .map(|[lat, lon, alt, ..]| Vec3::new(lat as f32, alt as f32, lon as f32))
                        .collect();

                    if !trail_points.is_empty() {
                        trail_mesh.0 = meshes.add(create_line_mesh(&trail_points));
                    }
                }
            }
        }
    }
}
