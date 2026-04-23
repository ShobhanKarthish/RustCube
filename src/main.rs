use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    window::PrimaryWindow,
};
use rustcube::{Color as CubeColor, Cube, Face, Move, Turn};

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 820.0;
const CUBIE_SPACING: f32 = 1.05;
const CUBIE_SIZE: f32 = 0.95;
const STICKER_SIZE: f32 = 0.78;
const STICKER_THICKNESS: f32 = 0.06;
const STICKER_LIFT: f32 = (CUBIE_SIZE * 0.5) + (STICKER_THICKNESS * 0.5) + 0.02;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.045, 0.052, 0.08)))
        .insert_resource(CubeState::default())
        .insert_resource(OrbitCamera::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "RustCube".into(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (print_controls, setup_scene))
        .add_systems(
            Update,
            (
                handle_keyboard_input,
                orbit_camera_system,
                sync_sticker_materials,
                sync_window_title,
            ),
        )
        .run();
}

#[derive(Resource)]
struct CubeState {
    cube: Cube,
    visuals_dirty: bool,
}

impl Default for CubeState {
    fn default() -> Self {
        Self {
            cube: Cube::default(),
            visuals_dirty: true,
        }
    }
}

#[derive(Resource)]
struct MaterialPalette {
    stickers: [Handle<StandardMaterial>; 6],
}

#[derive(Resource, Clone, Copy)]
struct OrbitCamera {
    yaw: f32,
    pitch: f32,
    radius: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.75,
            pitch: -0.45,
            radius: 11.5,
        }
    }
}

#[derive(Component)]
struct OrbitCameraMarker;

#[derive(Component)]
struct Sticker {
    index: usize,
}

fn print_controls() {
    info!("RustCube desktop controls:");
    info!("  U/D/L/R/F/B -> quarter turn");
    info!("  Hold Shift -> counter-clockwise turn");
    info!("  Hold Alt   -> half turn");
    info!("  Space      -> scramble");
    info!("  Z / Y      -> undo / redo");
    info!("  Backspace  -> reset");
    info!("  Left drag  -> orbit camera");
    info!("  Mouse wheel -> zoom");
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 220.0,
    });

    commands.spawn((
        Camera3d::default(),
        camera_transform(OrbitCamera::default()),
        OrbitCameraMarker,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 15_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 18.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        PointLight {
            intensity: 4_000.0,
            range: 30.0,
            ..default()
        },
        Transform::from_xyz(-8.0, 7.0, -10.0),
    ));

    let body_mesh = meshes.add(Mesh::from(Cuboid::new(CUBIE_SIZE, CUBIE_SIZE, CUBIE_SIZE)));
    let body_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.08, 0.09),
        perceptual_roughness: 0.72,
        metallic: 0.02,
        ..default()
    });

    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                commands.spawn((
                    Mesh3d(body_mesh.clone()),
                    MeshMaterial3d(body_material.clone()),
                    Transform::from_translation(grid_to_world(x, y, z)),
                ));
            }
        }
    }

    let sticker_mesh_x = meshes.add(Mesh::from(Cuboid::new(
        STICKER_THICKNESS,
        STICKER_SIZE,
        STICKER_SIZE,
    )));
    let sticker_mesh_y = meshes.add(Mesh::from(Cuboid::new(
        STICKER_SIZE,
        STICKER_THICKNESS,
        STICKER_SIZE,
    )));
    let sticker_mesh_z = meshes.add(Mesh::from(Cuboid::new(
        STICKER_SIZE,
        STICKER_SIZE,
        STICKER_THICKNESS,
    )));

    let palette = [
        materials.add(Color::srgb_u8(245, 247, 251)),
        materials.add(Color::srgb_u8(255, 216, 77)),
        materials.add(Color::srgb_u8(255, 138, 61)),
        materials.add(Color::srgb_u8(228, 72, 55)),
        materials.add(Color::srgb_u8(37, 179, 92)),
        materials.add(Color::srgb_u8(47, 107, 255)),
    ];
    commands.insert_resource(MaterialPalette {
        stickers: palette.clone(),
    });

    for index in 0..54 {
        let face = index / 9;
        let mesh = match face_axis(face) {
            Axis::X => sticker_mesh_x.clone(),
            Axis::Y => sticker_mesh_y.clone(),
            Axis::Z => sticker_mesh_z.clone(),
        };

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(palette[0].clone()),
            Transform::from_translation(sticker_translation(index)),
            Sticker { index },
        ));
    }
}

fn handle_keyboard_input(keys: Res<ButtonInput<KeyCode>>, mut cube_state: ResMut<CubeState>) {
    let turn = if keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight) {
        Turn::Half
    } else if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        Turn::CounterClockwise
    } else {
        Turn::Clockwise
    };

    let move_face = [
        (KeyCode::KeyU, Face::Up),
        (KeyCode::KeyD, Face::Down),
        (KeyCode::KeyL, Face::Left),
        (KeyCode::KeyR, Face::Right),
        (KeyCode::KeyF, Face::Front),
        (KeyCode::KeyB, Face::Back),
    ]
    .into_iter()
    .find_map(|(key, face)| keys.just_pressed(key).then_some(face));

    if let Some(face) = move_face {
        let mv = Move { face, turn };
        cube_state.cube.apply_move(mv);
        cube_state.visuals_dirty = true;
        info!("Applied move {}", mv.notation());
        return;
    }

    if keys.just_pressed(KeyCode::Space) {
        let scramble = cube_state.cube.scramble(24);
        cube_state.visuals_dirty = true;
        let notation = scramble
            .into_iter()
            .map(Move::notation)
            .collect::<Vec<_>>()
            .join(" ");
        info!("Scramble: {notation}");
        return;
    }

    if keys.just_pressed(KeyCode::KeyZ) {
        if let Some(mv) = cube_state.cube.undo() {
            cube_state.visuals_dirty = true;
            info!("Undo {}", mv.notation());
        }
        return;
    }

    if keys.just_pressed(KeyCode::KeyY) {
        if let Some(mv) = cube_state.cube.redo() {
            cube_state.visuals_dirty = true;
            info!("Redo {}", mv.notation());
        }
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) {
        cube_state.cube.reset();
        cube_state.visuals_dirty = true;
        info!("Cube reset");
    }
}

fn orbit_camera_system(
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut orbit: ResMut<OrbitCamera>,
    mut camera_query: Query<&mut Transform, With<OrbitCameraMarker>>,
) {
    if buttons.pressed(MouseButton::Left) {
        let delta = mouse_motion
            .read()
            .fold(Vec2::ZERO, |acc, event| acc + event.delta);
        if delta != Vec2::ZERO {
            orbit.yaw -= delta.x * 0.005;
            orbit.pitch = (orbit.pitch - delta.y * 0.004).clamp(-1.25, 1.25);
        }
    } else {
        mouse_motion.clear();
    }

    let scroll = mouse_wheel.read().fold(0.0, |acc, event| acc + event.y);
    if scroll != 0.0 {
        orbit.radius = (orbit.radius - scroll * 0.4).clamp(5.5, 18.0);
    }

    let Ok(mut transform) = camera_query.get_single_mut() else {
        return;
    };

    *transform = camera_transform(*orbit);
}

fn sync_sticker_materials(
    mut cube_state: ResMut<CubeState>,
    palette: Res<MaterialPalette>,
    mut stickers: Query<(&Sticker, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    if !cube_state.visuals_dirty {
        return;
    }

    for (sticker, mut material) in &mut stickers {
        material.0 =
            palette.stickers[color_index(cube_state.cube.stickers()[sticker.index])].clone();
    }

    cube_state.visuals_dirty = false;
}

fn sync_window_title(
    cube_state: Res<CubeState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !cube_state.is_changed() {
        return;
    }

    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    let solved_state = if cube_state.cube.is_solved() {
        "Solved"
    } else {
        "In progress"
    };
    let history = cube_state
        .cube
        .history()
        .iter()
        .rev()
        .take(10)
        .copied()
        .map(Move::notation)
        .collect::<Vec<_>>();
    let preview = if history.is_empty() {
        "ready".to_string()
    } else {
        history.into_iter().rev().collect::<Vec<_>>().join(" ")
    };

    window.title = format!("RustCube | {solved_state} | {preview}");
}

fn camera_transform(orbit: OrbitCamera) -> Transform {
    let yaw_rot = Quat::from_rotation_y(orbit.yaw);
    let pitch_rot = Quat::from_rotation_x(orbit.pitch);
    let offset = yaw_rot * pitch_rot * Vec3::new(0.0, 0.0, orbit.radius);
    Transform::from_translation(offset).looking_at(Vec3::ZERO, Vec3::Y)
}

fn grid_to_world(x: i32, y: i32, z: i32) -> Vec3 {
    Vec3::new(
        x as f32 * CUBIE_SPACING,
        y as f32 * CUBIE_SPACING,
        z as f32 * CUBIE_SPACING,
    )
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

fn face_axis(face: usize) -> Axis {
    match face {
        0 | 1 => Axis::Y,
        2 | 3 => Axis::X,
        4 | 5 => Axis::Z,
        _ => unreachable!("face index out of bounds"),
    }
}

fn sticker_translation(index: usize) -> Vec3 {
    let face = index / 9;
    let offset = index % 9;
    let row = (offset / 3) as i32;
    let col = (offset % 3) as i32;

    match face {
        0 => Vec3::new(
            (col - 1) as f32 * CUBIE_SPACING,
            CUBIE_SPACING + STICKER_LIFT,
            (row - 1) as f32 * CUBIE_SPACING,
        ),
        1 => Vec3::new(
            (col - 1) as f32 * CUBIE_SPACING,
            -CUBIE_SPACING - STICKER_LIFT,
            (1 - row) as f32 * CUBIE_SPACING,
        ),
        2 => Vec3::new(
            -CUBIE_SPACING - STICKER_LIFT,
            (1 - row) as f32 * CUBIE_SPACING,
            (col - 1) as f32 * CUBIE_SPACING,
        ),
        3 => Vec3::new(
            CUBIE_SPACING + STICKER_LIFT,
            (1 - row) as f32 * CUBIE_SPACING,
            (1 - col) as f32 * CUBIE_SPACING,
        ),
        4 => Vec3::new(
            (col - 1) as f32 * CUBIE_SPACING,
            (1 - row) as f32 * CUBIE_SPACING,
            CUBIE_SPACING + STICKER_LIFT,
        ),
        5 => Vec3::new(
            (1 - col) as f32 * CUBIE_SPACING,
            (1 - row) as f32 * CUBIE_SPACING,
            -CUBIE_SPACING - STICKER_LIFT,
        ),
        _ => unreachable!("sticker index out of bounds"),
    }
}

fn color_index(color: CubeColor) -> usize {
    match color {
        CubeColor::White => 0,
        CubeColor::Yellow => 1,
        CubeColor::Orange => 2,
        CubeColor::Red => 3,
        CubeColor::Green => 4,
        CubeColor::Blue => 5,
    }
}
