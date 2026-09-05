use std::{collections::VecDeque, fmt::Write};

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    window::{CursorLeft, PrimaryWindow, WindowFocused},
};
use rustcube::{Color as CubeColor, Cube, Face, Move, Turn};

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 820.0;
const CUBIE_SPACING: f32 = 1.05;
const CUBIE_SIZE: f32 = 0.95;
const STICKER_SIZE: f32 = 0.78;
const STICKER_THICKNESS: f32 = 0.06;
const STICKER_LIFT: f32 = (CUBIE_SIZE * 0.5) + (STICKER_THICKNESS * 0.5) + 0.02;
const FACE_SURFACE: f32 = CUBIE_SPACING + STICKER_LIFT + STICKER_THICKNESS * 0.5;
const DRAG_THRESHOLD: f32 = 24.0;

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.045, 0.052, 0.08)))
        .init_resource::<CubeState>()
        .init_resource::<SceneView>()
        .init_resource::<DragInteraction>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "RustCube".into(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                handle_keyboard_input,
                zoom_camera_system,
                cancel_drag_on_window_change,
                handle_pointer_input,
                animate_turns,
                sync_sticker_materials,
                sync_status,
            )
                .chain(),
        );
    app.run();
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Turn(Move),
    Undo,
    Redo,
    Scramble,
}

#[derive(Clone, Copy)]
struct ActiveTurn {
    mv: Move,
    elapsed: f32,
}

impl ActiveTurn {
    fn duration(self) -> f32 {
        if self.mv.turn == Turn::Half {
            0.28
        } else {
            0.19
        }
    }
}

#[derive(Resource)]
struct CubeState {
    // The engine advances when a queued action starts. Materials retain the previous
    // state until its rotation finishes; no later action executes before then.
    cube: Cube,
    scramble_source: Cube,
    pending: VecDeque<Action>,
    active: Option<ActiveTurn>,
    redo_count: usize,
    visuals_dirty: bool,
    restore_transforms: bool,
    status_dirty: bool,
}

impl Default for CubeState {
    fn default() -> Self {
        Self {
            cube: Cube::default(),
            scramble_source: Cube::default(),
            pending: VecDeque::new(),
            active: None,
            redo_count: 0,
            visuals_dirty: true,
            restore_transforms: false,
            status_dirty: true,
        }
    }
}

impl CubeState {
    fn enqueue(&mut self, action: Action) {
        self.pending.push_back(action);
        self.status_dirty = true;
    }

    fn reset(&mut self) {
        self.cube.reset();
        self.pending.clear();
        self.active = None;
        self.redo_count = 0;
        self.visuals_dirty = true;
        self.restore_transforms = true;
        self.status_dirty = true;
    }

    fn start_next(&mut self) {
        while let Some(action) = self.pending.pop_front() {
            self.status_dirty = true;
            let mv = match action {
                Action::Turn(mv) => {
                    self.cube.apply_move(mv);
                    self.redo_count = 0;
                    Some(mv)
                }
                Action::Undo => {
                    let mv = self.cube.undo();
                    if mv.is_some() {
                        self.redo_count += 1;
                    }
                    mv
                }
                Action::Redo => {
                    let mv = self.cube.redo();
                    if mv.is_some() {
                        self.redo_count -= 1;
                    }
                    mv
                }
                Action::Scramble => {
                    // Keep the generator's seed between scrambles without changing
                    // the real cube or overwriting its history with a cloned state.
                    self.scramble_source.reset();
                    for mv in self.scramble_source.scramble(24).into_iter().rev() {
                        self.pending.push_front(Action::Turn(mv));
                    }
                    continue;
                }
            };
            if let Some(mv) = mv {
                self.active = Some(ActiveTurn { mv, elapsed: 0.0 });
                break;
            }
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
    Orbit {
        button: MouseButton,
        last_cursor: Vec2,
    },
    Face {
        face: Face,
        start_cursor: Vec2,
        clockwise: Vec2,
    },
}

#[derive(Component)]
struct OrbitCameraMarker;

#[derive(Component)]
struct CubeRoot;

#[derive(Component)]
struct RestTransform(Transform);

#[derive(Component)]
struct Sticker {
    index: usize,
}

#[derive(Component)]
struct StatusText;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_root = commands
        .spawn((
            Transform::from_rotation(Quat::from_rotation_x(-0.55) * Quat::from_rotation_y(0.72)),
            Visibility::default(),
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

    let body_mesh = meshes.add(Cuboid::new(CUBIE_SIZE, CUBIE_SIZE, CUBIE_SIZE));
    let body_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.08, 0.09),
        perceptual_roughness: 0.72,
        metallic: 0.02,
        ..default()
    });
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                let rest = Transform::from_translation(grid_to_world(x, y, z));
                commands.entity(cube_root).with_children(|parent| {
                    parent.spawn((
                        Mesh3d(body_mesh.clone()),
                        MeshMaterial3d(body_material.clone()),
                        rest,
                        RestTransform(rest),
                    ));
                });
            }
        }
    }

    let sticker_meshes = [
        meshes.add(Cuboid::new(STICKER_THICKNESS, STICKER_SIZE, STICKER_SIZE)),
        meshes.add(Cuboid::new(STICKER_SIZE, STICKER_THICKNESS, STICKER_SIZE)),
        meshes.add(Cuboid::new(STICKER_SIZE, STICKER_SIZE, STICKER_THICKNESS)),
    ];
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
        let normal = face_normal_local(sticker_face(index));
        let axis = if normal.x != 0.0 {
            0
        } else if normal.y != 0.0 {
            1
        } else {
            2
        };
        let rest = Transform::from_translation(sticker_translation(index));
        commands.entity(cube_root).with_children(|parent| {
            parent.spawn((
                Mesh3d(sticker_meshes[axis].clone()),
                MeshMaterial3d(palette[0].clone()),
                rest,
                RestTransform(rest),
                Sticker { index },
            ));
        });
    }

    commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            top: Val::Px(22.0),
            left: Val::Px(24.0),
            right: Val::Px(24.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        },))
        .with_children(|parent| {
            parent.spawn((
                Text::new("RustCube"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb_u8(245, 247, 251)),
            ));
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb_u8(190, 204, 228)),
                StatusText,
            ));
        });
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(22.0),
            left: Val::Px(24.0),
            right: Val::Px(24.0),
            ..default()
        },
        Text::new(
            "U D L R F B  Turn face   /   Shift  Reverse   /   Alt  Half turn\n\
             Space  Scramble   /   Z / Y  Undo / Redo   /   Backspace  Reset now\n\
             Drag sticker around face center (center: sideways)   /   Right-drag or drag background  Orbit   /   Scroll  Zoom",
        ),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb_u8(190, 204, 228)),
    ));
}

fn handle_keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<CubeState>,
    mut drag: ResMut<DragInteraction>,
) {
    if !windows.get_single().is_ok_and(|window| window.focused) {
        return;
    }
    // Reset wins over every other input in this frame and cancels the entire queue.
    if keys.just_pressed(KeyCode::Backspace) {
        state.reset();
        drag.current = None;
        return;
    }
    let turn = if keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight) {
        Turn::Half
    } else if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        Turn::CounterClockwise
    } else {
        Turn::Clockwise
    };
    // Simultaneous keys have a stable order rather than dropping all but one.
    for (key, face) in [
        (KeyCode::KeyU, Face::Up),
        (KeyCode::KeyD, Face::Down),
        (KeyCode::KeyL, Face::Left),
        (KeyCode::KeyR, Face::Right),
        (KeyCode::KeyF, Face::Front),
        (KeyCode::KeyB, Face::Back),
    ] {
        if keys.just_pressed(key) {
            state.enqueue(Action::Turn(Move { face, turn }));
        }
    }
    for (key, action) in [
        (KeyCode::Space, Action::Scramble),
        (KeyCode::KeyZ, Action::Undo),
        (KeyCode::KeyY, Action::Redo),
    ] {
        if keys.just_pressed(key) {
            state.enqueue(action);
        }
    }
}

fn cancel_drag_on_window_change(
    mut focus_events: EventReader<WindowFocused>,
    mut leave_events: EventReader<CursorLeft>,
    mut drag: ResMut<DragInteraction>,
) {
    let lost_focus = focus_events.read().any(|event| !event.focused);
    let left_window = leave_events.read().next().is_some();
    focus_events.clear();
    leave_events.clear();
    if lost_focus || left_window {
        drag.current = None;
    }
}

fn handle_pointer_input(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &Transform), With<OrbitCameraMarker>>,
    mut root_query: Query<&mut Transform, (With<CubeRoot>, Without<OrbitCameraMarker>)>,
    mut drag: ResMut<DragInteraction>,
    mut state: ResMut<CubeState>,
) {
    let Ok(window) = windows.get_single() else {
        drag.current = None;
        return;
    };
    if !window.focused || keys.just_pressed(KeyCode::Backspace) {
        drag.current = None;
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        drag.current = None;
        return;
    };
    let Ok(mut root) = root_query.get_single_mut() else {
        return;
    };

    if buttons.just_pressed(MouseButton::Right) {
        drag.current = Some(DragMode::Orbit {
            button: MouseButton::Right,
            last_cursor: cursor,
        });
    } else if buttons.just_pressed(MouseButton::Left) && !buttons.pressed(MouseButton::Right) {
        drag.current = None;
        if let Ok((camera, camera_transform)) = camera_query.get_single() {
            // These entities have no parents. Use their current local transforms,
            // not last frame's GlobalTransforms, after zoom/orbit updates.
            let camera_world = GlobalTransform::from(*camera_transform);
            if let Ok(ray) = camera.viewport_to_world(&camera_world, cursor) {
                let inverse = root.compute_affine().inverse();
                let origin = inverse.transform_point3(ray.origin);
                let direction = inverse.transform_vector3(*ray.direction);
                if let Some((face, hit)) = pick_face(origin, direction) {
                    let normal = root.rotation * face_normal_local(face);
                    let radial = root.rotation * (hit - face_normal_local(face) * FACE_SURFACE);
                    let tangent = clockwise_tangent(
                        normal,
                        radial,
                        *camera_transform.up(),
                        *camera_transform.right(),
                    );
                    let hit_world = root.transform_point(hit);
                    if let (Ok(start), Ok(end)) = (
                        camera.world_to_viewport(&camera_world, hit_world),
                        camera.world_to_viewport(&camera_world, hit_world + tangent * 0.2),
                    ) {
                        let clockwise = (end - start).normalize_or_zero();
                        if clockwise != Vec2::ZERO {
                            drag.current = Some(DragMode::Face {
                                face,
                                start_cursor: cursor,
                                clockwise,
                            });
                        }
                    }
                } else {
                    drag.current = Some(DragMode::Orbit {
                        button: MouseButton::Left,
                        last_cursor: cursor,
                    });
                }
            }
        }
    }

    match drag.current {
        Some(DragMode::Orbit {
            button,
            last_cursor,
        }) => {
            if !buttons.pressed(button) {
                drag.current = None;
                return;
            }
            let delta = cursor - last_cursor;
            root.rotate_y(delta.x * 0.008);
            root.rotate_local_x(delta.y * 0.008);
            root.rotation = root.rotation.normalize();
            drag.current = Some(DragMode::Orbit {
                button,
                last_cursor: cursor,
            });
        }
        Some(DragMode::Face {
            face,
            start_cursor,
            clockwise,
        }) => {
            if buttons.just_released(MouseButton::Left) {
                drag.current = None;
                let distance = (cursor - start_cursor).dot(clockwise);
                if distance.abs() >= DRAG_THRESHOLD {
                    state.enqueue(Action::Turn(Move {
                        face,
                        turn: if distance > 0.0 {
                            Turn::Clockwise
                        } else {
                            Turn::CounterClockwise
                        },
                    }));
                }
            } else if !buttons.pressed(MouseButton::Left) {
                drag.current = None;
            }
        }
        None => {}
    }
}

fn zoom_camera_system(
    mut mouse_wheel: EventReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scene_view: ResMut<SceneView>,
    mut camera_query: Query<&mut Transform, With<OrbitCameraMarker>>,
    mut drag: ResMut<DragInteraction>,
) {
    let scroll = mouse_wheel.read().fold(0.0, |sum, event| {
        sum + event.y
            * match event.unit {
                MouseScrollUnit::Line => 0.4,
                MouseScrollUnit::Pixel => 0.008,
            }
    });
    if scroll == 0.0 || !windows.get_single().is_ok_and(|window| window.focused) {
        return;
    }
    scene_view.radius = (scene_view.radius - scroll).clamp(5.5, 18.0);
    if let Ok(mut transform) = camera_query.get_single_mut() {
        *transform = camera_transform(*scene_view);
    }
    // A face gesture's projected direction belongs to the old camera view.
    if matches!(drag.current, Some(DragMode::Face { .. })) {
        drag.current = None;
    }
}

fn animate_turns(
    time: Res<Time>,
    mut state: ResMut<CubeState>,
    mut pieces: Query<(&RestTransform, &mut Transform)>,
) {
    if state.restore_transforms {
        for (rest, mut transform) in &mut pieces {
            *transform = rest.0;
        }
        state.restore_transforms = false;
    }
    if state.active.is_none() {
        state.start_next();
        return;
    }
    let active = state.active.as_mut().unwrap();
    active.elapsed += time.delta_secs();
    let progress = (active.elapsed / active.duration()).min(1.0);
    let eased = progress * progress * (3.0 - 2.0 * progress);
    let rotation = turn_rotation(active.mv, eased);
    let normal = face_normal_local(active.mv.face);
    for (rest, mut transform) in &mut pieces {
        if belongs_to_layer(rest.0.translation, normal) {
            transform.translation = rotation * rest.0.translation;
            transform.rotation = rotation * rest.0.rotation;
        }
    }
    if progress == 1.0 {
        for (rest, mut transform) in &mut pieces {
            *transform = rest.0;
        }
        state.active = None;
        state.visuals_dirty = true;
        state.status_dirty = true;
        // Sync the completed move's colors before starting the next action.
    }
}

fn belongs_to_layer(position: Vec3, normal: Vec3) -> bool {
    position.dot(normal) > CUBIE_SPACING * 0.5
}

fn turn_rotation(mv: Move, progress: f32) -> Quat {
    let angle = match mv.turn {
        Turn::Clockwise => -std::f32::consts::FRAC_PI_2,
        Turn::CounterClockwise => std::f32::consts::FRAC_PI_2,
        Turn::Half => -std::f32::consts::PI,
    };
    Quat::from_axis_angle(face_normal_local(mv.face), angle * progress)
}

fn sync_sticker_materials(
    mut state: ResMut<CubeState>,
    palette: Res<MaterialPalette>,
    mut stickers: Query<(&Sticker, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    if !state.visuals_dirty {
        return;
    }
    for (sticker, mut material) in &mut stickers {
        material.0 = palette.stickers[color_index(state.cube.stickers()[sticker.index])].clone();
    }
    state.visuals_dirty = false;
}

fn sync_status(
    mut state: ResMut<CubeState>,
    mut texts: Query<&mut Text, With<StatusText>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !state.status_dirty {
        return;
    }
    let Ok(mut text) = texts.get_single_mut() else {
        return;
    };
    let label = if state.active.is_some() {
        "Turning"
    } else if state.cube.is_solved() {
        "Solved"
    } else {
        "In progress"
    };
    text.0.clear();
    let _ = write!(text.0, "{label}");
    if let Some(active) = state.active {
        text.0.push(' ');
        let _ = write!(text.0, "{}", active.mv);
    }
    let _ = write!(
        text.0,
        "   |   {} moves   |   {} redo",
        state.cube.history().len(),
        state.redo_count
    );
    if !state.pending.is_empty() {
        let _ = write!(text.0, "   |   {} queued", state.pending.len());
    }
    text.0.push_str("\nRecent: ");
    let history = state.cube.history();
    if history.is_empty() {
        text.0.push_str("No moves yet");
    } else {
        for mv in &history[history.len().saturating_sub(10)..] {
            let _ = write!(text.0, "{mv} ");
        }
    }
    if let Ok(mut window) = windows.get_single_mut() {
        window.title.clear();
        let _ = write!(window.title, "RustCube | {label}");
    }
    state.status_dirty = false;
}

fn camera_transform(scene_view: SceneView) -> Transform {
    Transform::from_xyz(0.0, 0.0, scene_view.radius).looking_at(Vec3::ZERO, Vec3::Y)
}

fn grid_to_world(x: i32, y: i32, z: i32) -> Vec3 {
    Vec3::new(x as f32, y as f32, z as f32) * CUBIE_SPACING
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
    let row = (index % 9 / 3) as i32;
    let col = (index % 3) as i32;
    let center = match sticker_face(index) {
        Face::Up => grid_to_world(col - 1, 1, row - 1),
        Face::Down => grid_to_world(col - 1, -1, 1 - row),
        Face::Left => grid_to_world(-1, 1 - row, col - 1),
        Face::Right => grid_to_world(1, 1 - row, 1 - col),
        Face::Front => grid_to_world(col - 1, 1 - row, 1),
        Face::Back => grid_to_world(1 - col, 1 - row, -1),
    };
    center + face_normal_local(sticker_face(index)) * STICKER_LIFT
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

// Intersect the opaque outer envelope first, then check its sticker footprint.
// A ray through a black seam never searches the far side of the cube.
fn pick_face(origin: Vec3, direction: Vec3) -> Option<(Face, Vec3)> {
    let mut near = 0.0_f32;
    let mut far = f32::INFINITY;
    let mut face = None;
    for (axis, negative, positive) in [
        (0, Face::Left, Face::Right),
        (1, Face::Down, Face::Up),
        (2, Face::Back, Face::Front),
    ] {
        if direction[axis].abs() < 1e-6 {
            if origin[axis].abs() > FACE_SURFACE {
                return None;
            }
            continue;
        }
        let a = (-FACE_SURFACE - origin[axis]) / direction[axis];
        let b = (FACE_SURFACE - origin[axis]) / direction[axis];
        let (entry, exit, entry_face) = if a < b {
            (a, b, negative)
        } else {
            (b, a, positive)
        };
        if entry > near {
            near = entry;
            face = Some(entry_face);
        }
        far = far.min(exit);
        if near > far {
            return None;
        }
    }
    let face = face?;
    let hit = origin + direction * near;
    let normal = face_normal_local(face);
    for axis in 0..3 {
        if normal[axis] == 0.0 {
            let cell = (hit[axis] / CUBIE_SPACING).round().clamp(-1.0, 1.0);
            if (hit[axis] - cell * CUBIE_SPACING).abs() > STICKER_SIZE * 0.5 {
                return None;
            }
        }
    }
    Some((face, hit))
}

fn clockwise_tangent(normal: Vec3, radial: Vec3, view_up: Vec3, view_right: Vec3) -> Vec3 {
    let radial = if radial.length_squared() < 0.65 * 0.65 {
        // The center has no useful rotation radius. Treat it as a handle at
        // the projected top of the face, so a sideways drag has a stable sign.
        let up = view_up - normal * view_up.dot(normal);
        if up.length_squared() > 0.01 {
            up
        } else {
            view_right - normal * view_right.dot(normal)
        }
    } else {
        radial
    };
    -normal.cross(radial).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animated_stickers_land_on_engine_permutation_for_every_turn() {
        let mut initial = Cube::new();
        initial.apply_moves(&[
            Move {
                face: Face::Front,
                turn: Turn::Clockwise,
            },
            Move {
                face: Face::Right,
                turn: Turn::CounterClockwise,
            },
            Move {
                face: Face::Up,
                turn: Turn::Half,
            },
        ]);
        for face in [
            Face::Up,
            Face::Down,
            Face::Left,
            Face::Right,
            Face::Front,
            Face::Back,
        ] {
            for turn in [Turn::Clockwise, Turn::CounterClockwise, Turn::Half] {
                let mv = Move { face, turn };
                let mut expected = initial.clone();
                expected.apply_move(mv);
                for source in 0..54 {
                    let position = sticker_translation(source);
                    let destination = if belongs_to_layer(position, face_normal_local(face)) {
                        turn_rotation(mv, 1.0) * position
                    } else {
                        position
                    };
                    let target = (0..54)
                        .find(|&index| sticker_translation(index).distance(destination) < 1e-4)
                        .expect("rotated sticker must occupy a canonical slot");
                    assert_eq!(
                        initial.stickers()[source],
                        expected.stickers()[target],
                        "{mv:?}, source {source}"
                    );
                }
            }
        }
    }

    #[test]
    fn queued_history_resolves_after_prior_actions_and_reset_cancels_everything() {
        let mut state = CubeState::default();
        let mv = Move {
            face: Face::Right,
            turn: Turn::Clockwise,
        };
        state.enqueue(Action::Turn(mv));
        state.enqueue(Action::Undo);
        state.enqueue(Action::Redo);
        state.start_next();
        assert_eq!(state.active.unwrap().mv, mv);
        state.active = None;
        state.start_next();
        assert_eq!(state.active.unwrap().mv, mv.inverse());
        assert!(state.cube.is_solved());
        state.active = None;
        state.start_next();
        assert_eq!(state.active.unwrap().mv, mv);
        assert_eq!(state.cube.history(), &[mv]);
        state.enqueue(Action::Scramble);
        state.reset();
        state.start_next();
        assert!(state.active.is_none());
        assert!(state.cube.is_solved());
        assert!(state.cube.history().is_empty());
        assert!(state.cube.redo().is_none());
    }

    #[test]
    fn seam_ray_does_not_pick_a_rear_sticker() {
        // The front hit lies between columns, while the same ray would reach
        // the middle back sticker if picking searched through the front seam.
        let direction = Vec3::new(-0.2, 0.0, -1.0).normalize();
        let origin = Vec3::new(CUBIE_SPACING * 0.5, 0.0, FACE_SURFACE) - direction * 3.0;
        assert!(pick_face(origin, direction).is_none());
        assert_eq!(
            pick_face(Vec3::new(0.0, 0.0, 5.0), -Vec3::Z).unwrap().0,
            Face::Front
        );
    }

    #[test]
    fn center_drag_has_a_stable_clockwise_direction_on_visible_faces() {
        for normal in [Vec3::Z, Vec3::new(0.0, 0.8, 0.6), Vec3::new(0.8, 0.0, 0.6)] {
            let tangent = clockwise_tangent(normal, Vec3::ZERO, Vec3::Y, Vec3::X);
            assert!(tangent.x > 0.0);
            assert!(tangent.dot(normal).abs() < 1e-6);
        }
    }
}
