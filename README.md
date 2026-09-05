# RustCube

A desktop Rubik's Cube in pure Rust. The engine is a 54-sticker permutation model; [Bevy](https://bevyengine.org) draws it, animates turns, and reads the keyboard and mouse.

Native only: `cargo run`. No web or WASM target.

- Standard outside-view notation (`U D L R F B`, `'` for reverse, `2` for a half turn)
- Queued, animated face turns — fast keyboard input plays in order
- Mouse gestures on stickers, plus orbit and zoom
- Scramble, undo/redo, and an immediate reset that cancels the queue
- On-screen status: solved / turning / in progress, move count, last ten moves

## Why Rust and Bevy

A cube is a permutation problem, not a pile of meshes. `Face`, `Turn`, and `Color` are enums; the state is `[Color; 54]`. Undo is the inverse move. Redo is a stack. A new turn after undo drops the redo branch. Tests apply each of the 18 face turns sticker-by-sticker so matching colors cannot hide a broken cycle.

Bevy is the desktop shell: 3D scene, input, and animation. The engine advances when a queued action starts; sticker materials stay on the previous permutation until that rotation finishes. The two layers can be tested apart — `src/lib.rs` for the cube, `src/main.rs` for the app.

## Project Layout

- `src/lib.rs`: cube engine (state, notation, scramble, undo/redo)
- `src/main.rs`: Bevy desktop application (scene, input, animation, HUD)

## Getting Started

Install a current stable Rust toolchain with [rustup](https://rustup.rs).

### 1. Run tests

```bash
cargo test
```

### 2. Launch the desktop app

```bash
cargo run
```

The window is 1280×820. On macOS, Option is treated as Alt so `Option` + a face key is a half turn, not a compose sequence.

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
