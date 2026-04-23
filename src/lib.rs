#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Up,
    Down,
    Left,
    Right,
    Front,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    Clockwise,
    CounterClockwise,
    Half,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Yellow,
    Orange,
    Red,
    Green,
    Blue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub face: Face,
    pub turn: Turn,
}

#[derive(Debug, Clone)]
pub struct Cube {
    stickers: [Color; 54],
    history: Vec<Move>,
    redo_stack: Vec<Move>,
    scramble_seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Vec3 {
    x: i8,
    y: i8,
    z: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StickerRef {
    position: Vec3,
    normal: Vec3,
}

impl Cube {
    pub fn new() -> Self {
        Self {
            stickers: solved_stickers(),
            history: Vec::new(),
            redo_stack: Vec::new(),
            scramble_seed: 0xC0B3_1234_ABCD_EF01,
        }
    }

    pub fn reset(&mut self) {
        self.stickers = solved_stickers();
        self.history.clear();
        self.redo_stack.clear();
    }

    pub fn stickers(&self) -> &[Color; 54] {
        &self.stickers
    }

    pub fn history(&self) -> &[Move] {
        &self.history
    }

    pub fn is_solved(&self) -> bool {
        self.stickers == solved_stickers()
    }

    pub fn apply_move(&mut self, mv: Move) {
        self.rotate(mv);
        self.history.push(mv);
        self.redo_stack.clear();
    }

    pub fn apply_moves(&mut self, moves: &[Move]) {
        for mv in moves {
            self.apply_move(*mv);
        }
    }

    pub fn undo(&mut self) -> Option<Move> {
        let mv = self.history.pop()?;
        let inverse = mv.inverse();
        self.rotate(inverse);
        self.redo_stack.push(mv);
        Some(inverse)
    }

    pub fn redo(&mut self) -> Option<Move> {
        let mv = self.redo_stack.pop()?;
        self.rotate(mv);
        self.history.push(mv);
        Some(mv)
    }

    pub fn scramble(&mut self, len: usize) -> Vec<Move> {
        let mut sequence = Vec::with_capacity(len);
        let faces = [
            Face::Up,
            Face::Down,
            Face::Left,
            Face::Right,
            Face::Front,
            Face::Back,
        ];
        let turns = [Turn::Clockwise, Turn::CounterClockwise, Turn::Half];
        let mut last_face = None;

        while sequence.len() < len {
            let face = faces[self.next_rand() as usize % faces.len()];
            if last_face == Some(face) {
                continue;
            }

            let turn = turns[self.next_rand() as usize % turns.len()];
            let mv = Move { face, turn };
            self.apply_move(mv);
            sequence.push(mv);
            last_face = Some(face);
        }

        sequence
    }

    fn next_rand(&mut self) -> u64 {
        self.scramble_seed = self
            .scramble_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        self.scramble_seed
    }

    fn rotate(&mut self, mv: Move) {
        match mv.turn {
            Turn::Clockwise => self.rotate_quarter(mv.face, true),
            Turn::CounterClockwise => self.rotate_quarter(mv.face, false),
            Turn::Half => {
                self.rotate_quarter(mv.face, true);
                self.rotate_quarter(mv.face, true);
            }
        }
    }

    fn rotate_quarter(&mut self, face: Face, clockwise: bool) {
        let current = self.stickers;
        let mut next = current;

        for target_index in 0..54 {
            let target_ref = sticker_ref(target_index);
            let source_ref = inverse_rotate_sticker(target_ref, face, clockwise);
            let source_index = sticker_index(source_ref);
            next[target_index] = current[source_index];
        }

        self.stickers = next;
    }
}

impl Default for Cube {
    fn default() -> Self {
        Self::new()
    }
}

impl Move {
    pub fn inverse(self) -> Self {
        let turn = match self.turn {
            Turn::Clockwise => Turn::CounterClockwise,
            Turn::CounterClockwise => Turn::Clockwise,
            Turn::Half => Turn::Half,
        };

        Self {
            face: self.face,
            turn,
        }
    }
}

fn solved_stickers() -> [Color; 54] {
    let mut stickers = [Color::White; 54];

    for (face_index, color) in [
        Color::White,
        Color::Yellow,
        Color::Orange,
        Color::Red,
        Color::Green,
        Color::Blue,
    ]
    .into_iter()
    .enumerate()
    {
        let start = face_index * 9;
        let end = start + 9;
        stickers[start..end].fill(color);
    }

    stickers
}

fn inverse_rotate_sticker(sticker: StickerRef, face: Face, clockwise: bool) -> StickerRef {
    if !belongs_to_face_layer(sticker.position, face) {
        return sticker;
    }

    let turns = if clockwise { 3 } else { 1 };
    let mut rotated = sticker;
    for _ in 0..turns {
        rotated.position = rotate_vec(rotated.position, face);
        rotated.normal = rotate_vec(rotated.normal, face);
    }
    rotated
}

fn belongs_to_face_layer(position: Vec3, face: Face) -> bool {
    match face {
        Face::Up => position.y == 1,
        Face::Down => position.y == -1,
        Face::Left => position.x == -1,
        Face::Right => position.x == 1,
        Face::Front => position.z == 1,
        Face::Back => position.z == -1,
    }
}

fn rotate_vec(vec: Vec3, face: Face) -> Vec3 {
    match face {
        Face::Up => Vec3 {
            x: vec.z,
            y: vec.y,
            z: -vec.x,
        },
        Face::Down => Vec3 {
            x: -vec.z,
            y: vec.y,
            z: vec.x,
        },
        Face::Right => Vec3 {
            x: vec.x,
            y: vec.z,
            z: -vec.y,
        },
        Face::Left => Vec3 {
            x: vec.x,
            y: -vec.z,
            z: vec.y,
        },
        Face::Front => Vec3 {
            x: vec.y,
            y: -vec.x,
            z: vec.z,
        },
        Face::Back => Vec3 {
            x: -vec.y,
            y: vec.x,
            z: vec.z,
        },
    }
}

fn sticker_ref(index: usize) -> StickerRef {
    let face = index / 9;
    let offset = index % 9;
    let row = (offset / 3) as i8;
    let col = (offset % 3) as i8;

    match face {
        0 => StickerRef {
            position: Vec3 {
                x: col - 1,
                y: 1,
                z: row - 1,
            },
            normal: Vec3 { x: 0, y: 1, z: 0 },
        },
        1 => StickerRef {
            position: Vec3 {
                x: col - 1,
                y: -1,
                z: 1 - row,
            },
            normal: Vec3 { x: 0, y: -1, z: 0 },
        },
        2 => StickerRef {
            position: Vec3 {
                x: -1,
                y: 1 - row,
                z: col - 1,
            },
            normal: Vec3 { x: -1, y: 0, z: 0 },
        },
        3 => StickerRef {
            position: Vec3 {
                x: 1,
                y: 1 - row,
                z: 1 - col,
            },
            normal: Vec3 { x: 1, y: 0, z: 0 },
        },
        4 => StickerRef {
            position: Vec3 {
                x: col - 1,
                y: 1 - row,
                z: 1,
            },
            normal: Vec3 { x: 0, y: 0, z: 1 },
        },
        5 => StickerRef {
            position: Vec3 {
                x: 1 - col,
                y: 1 - row,
                z: -1,
            },
            normal: Vec3 { x: 0, y: 0, z: -1 },
        },
        _ => unreachable!("sticker index out of bounds"),
    }
}

fn sticker_index(sticker: StickerRef) -> usize {
    let face_index = match sticker.normal {
        Vec3 { x: 0, y: 1, z: 0 } => 0,
        Vec3 { x: 0, y: -1, z: 0 } => 1,
        Vec3 { x: -1, y: 0, z: 0 } => 2,
        Vec3 { x: 1, y: 0, z: 0 } => 3,
        Vec3 { x: 0, y: 0, z: 1 } => 4,
        Vec3 { x: 0, y: 0, z: -1 } => 5,
        _ => unreachable!("invalid sticker normal"),
    };

    let (row, col) = match sticker.normal {
        Vec3 { x: 0, y: 1, z: 0 } => (sticker.position.z + 1, sticker.position.x + 1),
        Vec3 { x: 0, y: -1, z: 0 } => (1 - sticker.position.z, sticker.position.x + 1),
        Vec3 { x: -1, y: 0, z: 0 } => (1 - sticker.position.y, sticker.position.z + 1),
        Vec3 { x: 1, y: 0, z: 0 } => (1 - sticker.position.y, 1 - sticker.position.z),
        Vec3 { x: 0, y: 0, z: 1 } => (1 - sticker.position.y, sticker.position.x + 1),
        Vec3 { x: 0, y: 0, z: -1 } => (1 - sticker.position.y, 1 - sticker.position.x),
        _ => unreachable!("invalid sticker normal"),
    };

    (face_index * 9) + (row as usize * 3) + col as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_cube_starts_solved() {
        let cube = Cube::new();
        assert!(cube.is_solved());
    }

    #[test]
    fn inverse_moves_restore_state() {
        let mut cube = Cube::new();
        let original = *cube.stickers();

        let mv = Move {
            face: Face::Right,
            turn: Turn::Clockwise,
        };

        cube.apply_move(mv);
        cube.apply_move(mv.inverse());

        assert_eq!(*cube.stickers(), original);
        assert!(cube.is_solved());
    }

    #[test]
    fn half_turn_applied_twice_restores_state() {
        let mut cube = Cube::new();
        let original = *cube.stickers();

        let mv = Move {
            face: Face::Front,
            turn: Turn::Half,
        };

        cube.apply_move(mv);
        cube.apply_move(mv);

        assert_eq!(*cube.stickers(), original);
    }

    #[test]
    fn undo_and_redo_round_trip() {
        let mut cube = Cube::new();
        let original = *cube.stickers();

        cube.apply_move(Move {
            face: Face::Up,
            turn: Turn::Clockwise,
        });
        assert!(!cube.is_solved());

        cube.undo();
        assert_eq!(*cube.stickers(), original);

        cube.redo();
        assert!(!cube.is_solved());
    }
}
