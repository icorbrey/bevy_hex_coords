use bevy::ecs::message::MessageReader;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_hex_coords::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(HexCoordsPlugin {
            auto_attach_transforms: true,
        })
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, pan_camera)
        .add_systems(Update, zoom_camera)
        .add_systems(Update, update_fps_text)
        .run();
}

#[derive(Component)]
struct FpsText;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    let unit_size = HexUnitSize(200.0);
    let hex_mesh = meshes.add(RegularPolygon::new(110.0, 6));
    let vert_mesh = meshes.add(Circle::new(16.0));
    let edge_mesh = meshes.add(Rectangle::new(90.0, 6.0));
    let white = materials.add(Color::WHITE);

    let coords: Vec<HexCoord> = (-3..=3)
        .flat_map(|q| (-3..=3).map(move |r| HexCoord::new(q, r)))
        .collect();

    let verts: std::collections::HashSet<HexVert> = coords.iter()
        .flat_map(|c| c.vertices())
        .collect();

    let edges: std::collections::HashSet<HexEdge> = coords.iter()
        .flat_map(|c| c.edges())
        .collect();

    // Hexes
    for (i, coord) in coords.iter().enumerate() {
        let color = Color::hsl(360.0 * i as f32 / coords.len() as f32, 0.95, 0.7);
        commands.spawn((
            *coord,
            unit_size,
            Mesh2d(hex_mesh.clone()),
            MeshMaterial2d(materials.add(color)),
        ));
    }

    // Vertices
    for vert in verts {
        commands.spawn((
            vert,
            unit_size,
            Mesh2d(vert_mesh.clone()),
            MeshMaterial2d(white.clone()),
        ));
    }

    // Edges
    let edge_count = edges.len();
    for (i, edge) in edges.into_iter().enumerate() {
        let color = Color::hsl(360.0 * (1.0 - i as f32 / edge_count as f32), 0.95, 0.7);
        commands.spawn((
            edge,
            unit_size,
            Mesh2d(edge_mesh.clone()),
            MeshMaterial2d(materials.add(color)),
        ));
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.75)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("FPS: --"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.0, 1.0, 0.0)),
                FpsText,
            ));
        });
}

fn pan_camera(
    mut cursor_reader: MessageReader<CursorMoved>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut camera_q: Query<(&mut Transform, &Camera, &GlobalTransform), With<Camera2d>>,
    mut last_cursor_pos: Local<Option<Vec2>>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        cursor_reader.clear();
        *last_cursor_pos = None;
        return;
    }

    let Some((mut transform, camera, camera_transform)) = camera_q.iter_mut().next() else {
        return;
    };

    for ev in cursor_reader.read() {
        let current_pos = ev.position;
        let Some(prev_pos) = *last_cursor_pos else {
            *last_cursor_pos = Some(current_pos);
            continue;
        };

        let Ok(prev_world) = camera.viewport_to_world_2d(camera_transform, prev_pos) else {
            *last_cursor_pos = Some(current_pos);
            continue;
        };
        let Ok(current_world) = camera.viewport_to_world_2d(camera_transform, current_pos) else {
            *last_cursor_pos = Some(current_pos);
            continue;
        };

        let world_delta = prev_world - current_world;
        transform.translation += world_delta.extend(0.0);
        *last_cursor_pos = Some(current_pos);
    }
}

fn zoom_camera(
    mut wheel_reader: MessageReader<MouseWheel>,
    mut camera_q: Query<&mut Projection, With<Camera2d>>,
) {
    let mut scroll = 0.0;
    for ev in wheel_reader.read() {
        scroll += ev.y;
    }

    if scroll == 0.0 {
        return;
    }

    let Some(mut projection) = camera_q.iter_mut().next() else { return };
    let zoom_factor = 1.0 - scroll * 0.1;
    if let Projection::Orthographic(ortho) = projection.as_mut() {
        ortho.scale = (ortho.scale * zoom_factor).clamp(0.1, 10.0);
    }
}

fn update_fps_text(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<FpsText>>) {
    let Ok(mut text) = query.single_mut() else { return };
    let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
    else {
        return;
    };

    text.0 = format!("FPS: {fps:.0}");
}
