# RustCube

`RustCube` is a pure Rust desktop Rubik's Cube app built with Bevy.

- Canonical 54-sticker cube state with standard outside-view move notation.
- Animated face turns, mouse gestures, scramble, undo/redo, and immediate reset.
- On-screen controls, move history, and solved/turning status.

## Project Layout

- `src/lib.rs`: cube engine
- `src/main.rs`: Bevy desktop application

## Getting Started

### 1. Run tests

Install a current stable Rust toolchain with [rustup](https://rustup.rs) first.

```bash
cargo test
```

### 2. Launch the desktop app

```bash
cargo run
```

## Controls

- `U D L R F B`: turn faces clockwise, looking directly at the named face
- `Shift` + face key: counter-clockwise turn
- `Alt` + face key: half turn
- `Space`: scramble
- `Z`: undo
- `Y`: redo
- `Backspace`: reset
- Left-drag a sticker around its face center: turn that face on release
- Left-drag a center sticker sideways: turn that face
- Left-drag the background or right-drag anywhere: orbit the cube
- Mouse wheel or trackpad scroll: zoom

Turns animate in order, including fast keyboard input. Scramble adds 24 moves;
undo/redo pressed during an animation run after the preceding queued moves.
`Backspace` immediately cancels the animation and queue and clears both histories,
without changing your viewing angle. Leaving the window or losing focus cancels
an unfinished drag.

Face names stay attached to the cube when you orbit it; the green center is
front, white is up, red is right, orange is left, yellow is down, and blue is back.
Scrambles vary between sessions and avoid consecutive turns of the same face.

## Development Checks

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Regression coverage includes all 18 face-turn permutations, matching animation
endpoints, undo/redo branching, reset cancellation, and sticker picking through seams.
