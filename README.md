# RustCube

`RustCube` is a pure Rust desktop Rubik's Cube app built with Bevy.

- `src/lib.rs` keeps the canonical cube state and move logic.
- `src/main.rs` runs a native Bevy desktop app for rendering and input.

## Project Layout

- `src/lib.rs`: cube engine
- `src/main.rs`: Bevy desktop application

## Getting Started

### 1. Run tests

```bash
cargo test
```

### 2. Launch the desktop app

```bash
cargo run
```

## Controls

- `U D L R F B`: turn faces clockwise
- `Shift` + face key: counter-clockwise turn
- `Alt` + face key: half turn
- `Space`: scramble
- `Z`: undo
- `Y`: redo
- `Backspace`: reset
- Left mouse drag: orbit camera
- Mouse wheel: zoom
