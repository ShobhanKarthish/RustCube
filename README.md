# RustCube

`RustCube` is a browser-based 3D Rubik's Cube project with:

- Rust for canonical cube state and move logic
- WebAssembly for browser interop
- Three.js for rendering and animation

## Project Layout

- `src/`: Rust cube engine
- `frontend/`: Vite + Three.js app shell
- `scripts/build-wasm.mjs`: builds the Rust crate into a `pkg/` directory with `wasm-pack`

## Getting Started

### 1. Verify the Rust core

```bash
cargo test
```

### 2. Install frontend dependencies

```bash
cd frontend
npm install
```

### 3. Install `wasm-pack`

```bash
cargo install wasm-pack
```

### 4. Start the frontend

```bash
cd frontend
npm run dev
```

## Planned Feature Flow

1. Finish the WASM binding layer for `Cube`
2. Sync move application from Rust into the renderer
3. Add pointer-based slice picking and animated turns
4. Add scramble UI, undo/redo controls, and solved-state feedback
