use bevy::{input::mouse::MouseWheel, prelude::*, window::PrimaryWindow};
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
        .insert_resource(SceneView::default())
        .insert_resource(DragInteraction::default())
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
                begin_drag_system,
                drag_cube_or_face_system,
                finish_drag_system,
                zoom_camera_system,
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
struct SceneView {
    radius: f32,
}

impl Default for SceneView {
    fn default() -> Self {
        Self { radius: 11.5 }
    }
}

#[derive(Resource, Default)]
struct DragInteraction {
    current: Option<DragMode>,
}

#[derive(Clone, Copy)]
enum DragMode {
    RotateCube {
        last_cursor: Vec2,
    },
    TurnFace {
        face: Face,
        start_cursor: Vec2,
        face_center: Vec2,
        face_normal_world: Vec3,
    },
}

#[derive(Component)]
struct OrbitCameraMarker;

#[derive(Component)]
struct CubeRoot;

#[derive(Component, Clone, Copy)]
struct Sticker {
    index: usize,
    face: Face,
    axis: Axis,
}

fn print_controls() {
    info!("RustCube desktop controls:");
    info!("  U/D/L/R/F/B -> quarter turn");
    info!("  Hold Shift -> counter-clockwise turn");
    info!("  Hold Alt   -> half turn");
    info!("  Space      -> scramble");
    info!("  Z / Y      -> undo / redo");
    info!("  Backspace  -> reset");
    info!("  Drag empty space -> rotate cube");
    info!("  Drag a sticker    -> turn that face");
    info!("  Mouse wheel -> zoom");
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_root = commands
        .spawn((
            Transform::from_rotation(Quat::from_rotation_x(-0.55) * Quat::from_rotation_y(0.72)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            CubeRoot,
        ))
        .id();

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 220.0,
    });

    commands.spawn((
        Camera3d::default(),
        camera_transform(SceneView::default()),
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
                commands.entity(cube_root).with_children(|parent| {
                    parent.spawn((
                        Mesh3d(body_mesh.clone()),
                        MeshMaterial3d(body_material.clone()),
                        Transform::from_translation(grid_to_world(x, y, z)),
                    ));
                });
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
        let face = sticker_face(index);
        let axis = face_axis(face);
        let mesh = match axis {
            Axis::X => sticker_mesh_x.clone(),
            Axis::Y => sticker_mesh_y.clone(),
            Axis::Z => sticker_mesh_z.clone(),
        };

        commands.entity(cube_root).with_children(|parent| {
            parent.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(palette[0].clone()),
                Transform::from_translation(sticker_translation(index)),
                Sticker { index, face, axis },
            ));
        });
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

fn begin_drag_system(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<OrbitCameraMarker>>,
    cube_root_query: Query<&GlobalTransform, With<CubeRoot>>,
    stickers: Query<(&Sticker, &GlobalTransform)>,
    mut drag: ResMut<DragInteraction>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(cursor) = cursor_position(&windows) else {
        return;
    };

    let Some((camera, camera_transform)) = camera_query.iter().next() else {
        return;
    };
    let Some((ray_origin, ray_direction)) = cursor_ray(camera, camera_transform, cursor) else {
        return;
    };

    if let Some((sticker, _, _)) = pick_sticker(ray_origin, ray_direction, &stickers) {
        let Ok(cube_root) = cube_root_query.get_single() else {
            return;
        };
        let face_center_world = cube_root.transform_point(face_center_local(sticker.face));
        if let Ok(face_center) = camera.world_to_viewport(camera_transform, face_center_world) {
            drag.current = Some(DragMode::TurnFace {
                face: sticker.face,
                start_cursor: cursor,
                face_center,
                face_normal_world: cube_root
                    .compute_transform()
                    .rotation
                    .mul_vec3(face_normal_local(sticker.face)),
            });
            return;
        }
    }

    drag.current = Some(DragMode::RotateCube {
        last_cursor: cursor,
    });
}

fn drag_cube_or_face_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut drag: ResMut<DragInteraction>,
    mut cube_root_query: Query<&mut Transform, With<CubeRoot>>,
) {
    if !buttons.pressed(MouseButton::Left) {
        return;
    }

    let Some(cursor) = cursor_position(&windows) else {
        return;
    };

    let Some(current) = drag.current else {
        return;
    };

    if let DragMode::RotateCube { last_cursor } = current {
        let delta = cursor - last_cursor;
        if delta != Vec2::ZERO {
            let Ok(mut cube_root) = cube_root_query.get_single_mut() else {
                return;
            };
            cube_root.rotate_y(delta.x * 0.008);
            cube_root.rotate_local_x(delta.y * 0.008);
            drag.current = Some(DragMode::RotateCube {
                last_cursor: cursor,
            });
        }
    }
}

fn finish_drag_system(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<OrbitCameraMarker>>,
    mut drag: ResMut<DragInteraction>,
    mut cube_state: ResMut<CubeState>,
) {
    if !buttons.just_released(MouseButton::Left) {
        return;
    }

    let Some(current) = drag.current.take() else {
        return;
    };

    let DragMode::TurnFace {
        face,
        start_cursor,
        face_center,
        face_normal_world,
    } = current
    else {
        return;
    };

    let Some(end_cursor) = cursor_position(&windows) else {
        return;
    };

    let drag_delta = end_cursor - start_cursor;
    if drag_delta.length() < 24.0 {
        return;
    }

    let Some((_, camera_transform)) = camera_query.iter().next() else {
        return;
    };

    let camera_position = camera_transform.translation();
    let face_to_camera = camera_position.normalize_or_zero();
    let facing_camera = face_normal_world.dot(face_to_camera) > 0.0;

    let from = start_cursor - face_center;
    let to = end_cursor - face_center;
    let angle = signed_screen_angle(from, to);
    let primary = if angle.abs() > 0.35 {
        angle
    } else {
        drag_delta.x - drag_delta.y
    };
    let clockwise = if facing_camera {
        primary > 0.0
    } else {
        primary < 0.0
    };

    let mv = Move {
        face,
        turn: if clockwise {
            Turn::Clockwise
        } else {
            Turn::CounterClockwise
        },
    };
    cube_state.cube.apply_move(mv);
    cube_state.visuals_dirty = true;
    info!("Dragged face {}", mv.notation());
}

fn zoom_camera_system(
    mut mouse_wheel: EventReader<MouseWheel>,
    mut scene_view: ResMut<SceneView>,
    mut camera_query: Query<&mut Transform, With<OrbitCameraMarker>>,
) {
    let scroll = mouse_wheel.read().fold(0.0, |acc, event| acc + event.y);
    if scroll != 0.0 {
        scene_view.radius = (scene_view.radius - scroll * 0.4).clamp(5.5, 18.0);
    }

    let Ok(mut transform) = camera_query.get_single_mut() else {
        return;
    };

    *transform = camera_transform(*scene_view);
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

fn camera_transform(scene_view: SceneView) -> Transform {
    Transform::from_xyz(0.0, 0.0, scene_view.radius).looking_at(Vec3::ZERO, Vec3::Y)
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

fn face_axis(face: Face) -> Axis {
    match face {
        Face::Up | Face::Down => Axis::Y,
        Face::Left | Face::Right => Axis::X,
        Face::Front | Face::Back => Axis::Z,
    }
}

fn sticker_face(index: usize) -> Face {
    match index / 9 {
        0 => Face::Up,
        1 => Face::Down,
        2 => Face::Left,
        3 => Face::Right,
        4 => Face::Front,
        5 => Face::Back,
        _ => unreachable!("sticker index out of bounds"),
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

fn cursor_position(windows: &Query<&Window, With<PrimaryWindow>>) -> Option<Vec2> {
    windows.get_single().ok()?.cursor_position()
}

fn cursor_ray(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    cursor: Vec2,
) -> Option<(Vec3, Vec3)> {
    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    Some((ray.origin, *ray.direction))
}

fn pick_sticker(
    ray_origin: Vec3,
    ray_direction: Vec3,
    stickers: &Query<(&Sticker, &GlobalTransform)>,
) -> Option<(Sticker, Vec3, f32)> {
    let mut closest: Option<(Sticker, Vec3, f32)> = None;

    for (sticker, transform) in stickers.iter() {
        let half_extents = sticker_half_extents(sticker.axis);
        if let Some(distance) = ray_hits_box(ray_origin, ray_direction, transform, half_extents) {
            let hit_point = ray_origin + ray_direction * distance;
            if closest
                .as_ref()
                .map(|(_, _, current)| distance < *current)
                .unwrap_or(true)
            {
                closest = Some((*sticker, hit_point, distance));
            }
        }
    }

    closest
}

fn ray_hits_box(
    ray_origin: Vec3,
    ray_direction: Vec3,
    transform: &GlobalTransform,
    half_extents: Vec3,
) -> Option<f32> {
    let inverse = transform.affine().inverse();
    let origin_local = inverse.transform_point3(ray_origin);
    let direction_local = inverse.transform_vector3(ray_direction);

    let mut t_min: f32 = 0.0;
    let mut t_max = f32::INFINITY;

    for axis in 0..3 {
        let origin = origin_local[axis];
        let direction = direction_local[axis];
        let min = -half_extents[axis];
        let max = half_extents[axis];

        if direction.abs() < f32::EPSILON {
            if origin < min || origin > max {
                return None;
            }
            continue;
        }

        let inv_direction = 1.0 / direction;
        let mut near = (min - origin) * inv_direction;
        let mut far = (max - origin) * inv_direction;
        if near > far {
            std::mem::swap(&mut near, &mut far);
        }

        t_min = t_min.max(near);
        t_max = t_max.min(far);
        if t_min > t_max {
            return None;
        }
    }

    (t_max >= 0.0).then_some(t_min.max(0.0))
}

fn sticker_half_extents(axis: Axis) -> Vec3 {
    match axis {
        Axis::X => Vec3::new(
            STICKER_THICKNESS * 0.5,
            STICKER_SIZE * 0.5,
            STICKER_SIZE * 0.5,
        ),
        Axis::Y => Vec3::new(
            STICKER_SIZE * 0.5,
            STICKER_THICKNESS * 0.5,
            STICKER_SIZE * 0.5,
        ),
        Axis::Z => Vec3::new(
            STICKER_SIZE * 0.5,
            STICKER_SIZE * 0.5,
            STICKER_THICKNESS * 0.5,
        ),
    }
}

fn face_center_local(face: Face) -> Vec3 {
    match face {
        Face::Up => Vec3::new(0.0, CUBIE_SPACING + STICKER_LIFT, 0.0),
        Face::Down => Vec3::new(0.0, -CUBIE_SPACING - STICKER_LIFT, 0.0),
        Face::Left => Vec3::new(-CUBIE_SPACING - STICKER_LIFT, 0.0, 0.0),
        Face::Right => Vec3::new(CUBIE_SPACING + STICKER_LIFT, 0.0, 0.0),
        Face::Front => Vec3::new(0.0, 0.0, CUBIE_SPACING + STICKER_LIFT),
        Face::Back => Vec3::new(0.0, 0.0, -CUBIE_SPACING - STICKER_LIFT),
    }
}

fn face_normal_local(face: Face) -> Vec3 {
    match face {
        Face::Up => Vec3::Y,
        Face::Down => -Vec3::Y,
        Face::Left => -Vec3::X,
        Face::Right => Vec3::X,
        Face::Front => Vec3::Z,
        Face::Back => -Vec3::Z,
    }
}

fn signed_screen_angle(from: Vec2, to: Vec2) -> f32 {
    if from.length_squared() < 1.0 || to.length_squared() < 1.0 {
        return 0.0;
    }

    let from = from.normalize();
    let to = to.normalize();
    let cross = from.x * to.y - from.y * to.x;
    let dot = from.dot(to).clamp(-1.0, 1.0);
    cross.atan2(dot)
}
