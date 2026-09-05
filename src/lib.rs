use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::BuildHasher;

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
            scramble_seed: RandomState::new().hash_one(0_u8),
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
            let face = faces[self.random_index(faces.len())];
            if last_face == Some(face) {
                continue;
            }

            let turn = turns[self.random_index(turns.len())];
            let mv = Move { face, turn };
            self.apply_move(mv);
            sequence.push(mv);
            last_face = Some(face);
        }

        sequence
    }

    fn next_rand(&mut self) -> u64 {
        // SplitMix64 mixes every output bit, unlike the alternating low bit of an LCG.
        self.scramble_seed = self.scramble_seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.scramble_seed;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn random_index(&mut self, len: usize) -> usize {
        let bound = len as u64;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next_rand();
            if value >= threshold {
                return (value % bound) as usize;
            }
        }
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

        for (target_index, target) in self.stickers.iter_mut().enumerate() {
            let target_ref = sticker_ref(target_index);
            if belongs_to_face_layer(target_ref.position, face) {
                let source_ref = StickerRef {
                    position: rotate_vec(target_ref.position, face, !clockwise),
                    normal: rotate_vec(target_ref.normal, face, !clockwise),
                };
                *target = current[sticker_index(source_ref)];
            }
        }
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

    pub fn notation(self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Move {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let face = match self.face {
            Face::Up => "U",
            Face::Down => "D",
            Face::Left => "L",
            Face::Right => "R",
            Face::Front => "F",
            Face::Back => "B",
        };

        let suffix = match self.turn {
            Turn::Clockwise => "",
            Turn::CounterClockwise => "'",
            Turn::Half => "2",
        };

        write!(formatter, "{face}{suffix}")
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

fn rotate_vec(vec: Vec3, face: Face, clockwise: bool) -> Vec3 {
    // Clockwise is a negative quarter-turn about the face's outward normal.
    match (face, clockwise) {
        (Face::Down, true) | (Face::Up, false) => Vec3 {
            x: vec.z,
            y: vec.y,
            z: -vec.x,
        },
        (Face::Up, true) | (Face::Down, false) => Vec3 {
            x: -vec.z,
            y: vec.y,
            z: vec.x,
        },
        (Face::Right, true) | (Face::Left, false) => Vec3 {
            x: vec.x,
            y: vec.z,
            z: -vec.y,
        },
        (Face::Left, true) | (Face::Right, false) => Vec3 {
            x: vec.x,
            y: -vec.z,
            z: vec.y,
        },
        (Face::Front, true) | (Face::Back, false) => Vec3 {
            x: vec.y,
            y: -vec.x,
            z: vec.z,
        },
        (Face::Back, true) | (Face::Front, false) => Vec3 {
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
