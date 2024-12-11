use bevy::{
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    math::Affine2,
    prelude::*,
};
use crate::camera::OrbitCamera;

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Set background color to black
    commands.insert_resource(ClearColor(Color::BLACK));

    let floor_material = materials.add(StandardMaterial {
        perceptual_roughness: 0.1,
        metallic: 0.8,
        base_color: Color::srgb(0.2, 0.3, 0.8),  // Blue base color
        base_color_texture: Some(asset_server.load_with_settings(
            "textures/checkered.png",
            |s: &mut _| {
                *s = ImageLoaderSettings {
                    sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::Repeat,
                        address_mode_v: ImageAddressMode::Repeat,
                        ..default()
                    }),
                    ..default()
                }
            },
        )),
        uv_transform: Affine2::from_scale(Vec2::new(2., 3.)),
        ..default()        
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(90.0, 0.2, 90.0))),
        MeshMaterial3d(floor_material),
    ));

    /*
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 2.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(-0.0, 0.0, -0.0),
    ));
    */

    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            range: 100.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-40.0, 40.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
        OrbitCamera::default(),
    ));
}