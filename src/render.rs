use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::mesh::PrimitiveTopology,
};
use bevy_inspector_egui::WorldInspectorPlugin;
use simula_camera::orbitcam::{OrbitCamera, OrbitCameraPlugin};
use simula_viz::{
    axes::{Axes, AxesBundle, AxesPlugin},
    grid::{Grid, GridBundle, GridPlugin},
    lines::{LinesMaterial, LinesPlugin},
    pointcloud::PointcloudPlugin,
};

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut lines_materials: ResMut<Assets<LinesMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // digital twin assets

    let obj_gltf = asset_server.load("models/logos/Logo_SugarFunge.gltf#Scene0");
    commands
        .spawn_bundle(TransformBundle {
            local: Transform::from_xyz(-8.0, 4.0, 0.0),
            global: GlobalTransform::identity(),
        })
        .with_children(|parent| {
            parent.spawn_scene(obj_gltf);
        });

    let obj_gltf = asset_server.load("models/logos/Logo_IPFS.gltf#Scene0");
    commands
        .spawn_bundle(TransformBundle {
            local: Transform::from_xyz(-6.0, 4.0, 0.0),
            global: GlobalTransform::identity(),
        })
        .with_children(|parent| {
            parent.spawn_scene(obj_gltf);
        });

    let obj_gltf = asset_server.load("models/logos/Logo_FULA.gltf#Scene0");
    commands
        .spawn_bundle(TransformBundle {
            local: Transform::from_xyz(-4.0, 4.0, 0.0),
            global: GlobalTransform::identity(),
        })
        .with_children(|parent| {
            parent.spawn_scene(obj_gltf);
        });

    // grid
    commands
        .spawn_bundle(GridBundle {
            grid: Grid {
                size: 10,
                divisions: 10,
                start_color: Color::BLUE,
                end_color: Color::RED,
                ..Default::default()
            },
            mesh: meshes.add(Mesh::new(PrimitiveTopology::LineList)),
            material: lines_materials.add(LinesMaterial {}),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..Default::default()
        })
        .insert(Name::new("Fula: Grid"));

    // axes
    commands
        .spawn_bundle(AxesBundle {
            axes: Axes {
                size: 1.,
                inner_offset: 5.,
            },
            mesh: meshes.add(Mesh::new(PrimitiveTopology::LineList)),
            material: lines_materials.add(LinesMaterial {}),
            transform: Transform::from_xyz(0.0, 0.01, 0.0),
            ..Default::default()
        })
        .insert(Name::new("Axes: World"));

    // lighting
    let theta = std::f32::consts::FRAC_PI_4;
    let light_transform = Mat4::from_euler(EulerRot::ZYX, 0.0, std::f32::consts::FRAC_PI_2, -theta);
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::rgb(1.0, 1.0, 1.0),
            illuminance: 50000.,
            ..Default::default()
        },
        transform: Transform::from_matrix(light_transform),
        ..Default::default()
    });

    // camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            ..Default::default()
        })
        .insert(OrbitCamera {
            center: Vec3::new(0.0, 1.0, 0.0),
            distance: 10.0,
            ..Default::default()
        });

    commands.spawn_bundle(UiCameraBundle::default());

    commands.spawn_bundle(TextBundle {
        text: Text {
            sections: vec![TextSection {
                value: "\nFPS: ".to_string(),
                style: TextStyle {
                    font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                    font_size: 12.0,
                    color: Color::rgb(0.0, 1.0, 0.0),
                },
            }],
            ..Default::default()
        },
        style: Style {
            position_type: PositionType::Absolute,
            position: Rect {
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    });
}

fn debug_info(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text>) {
    if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(average) = fps.average() {
            for mut text in query.iter_mut() {
                text.sections[0].value = format!("{:.2}", average);
            }
        }
    };
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(WorldInspectorPlugin::new())
            .add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_plugin(OrbitCameraPlugin)
            .add_plugin(LinesPlugin)
            .add_plugin(AxesPlugin)
            .add_plugin(GridPlugin)
            .add_plugin(PointcloudPlugin)
            .add_startup_system(setup)
            .add_system(debug_info);
    }
}
