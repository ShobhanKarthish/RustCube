import "./style.css";
import * as THREE from "three";

type WasmBindings = {
  default: (moduleOrPath?: unknown) => Promise<unknown>;
  WasmCube: new () => CubeApi;
};

type CubeApi = {
  reset(): void;
  apply_move(notation: string): void;
  scramble(length: number): string;
  undo(): boolean;
  redo(): boolean;
  undo_move(): string;
  redo_move(): string;
  is_solved(): boolean;
  stickers(): Uint8Array | number[];
  history(): string;
};

type MoveNotation = "R" | "U" | "F" | "L" | "D" | "B" | "R'" | "U'" | "F2";
type FaceKey = "U" | "D" | "L" | "R" | "F" | "B";

type FaceConfig = {
  axis: "x" | "y" | "z";
  layer: -1 | 1;
  axisVector: any;
  visualSign: 1 | -1;
};

type MoveIntent = {
  notation: string;
  applyBeforeAnimation: boolean;
  overlayLabel?: string;
};

type AnimationState = {
  notation: string;
  group: any;
  members: CubieNode[];
  axis: any;
  startTime: number;
  durationMs: number;
  startAngle: number;
  targetAngle: number;
};

type CubieNode = {
  mesh: any;
  coord: any;
};

const MOVE_SET: readonly MoveNotation[] = ["R", "U", "F", "L", "D", "B", "R'", "U'", "F2"];
const PALETTE = ["#f5f7fb", "#ffd84d", "#ff8a3d", "#e44837", "#25b35c", "#2f6bff"];
const FACE_ORDER: readonly FaceKey[] = ["U", "D", "L", "R", "F", "B"];
const FACE_INDEX: Record<FaceKey, number> = { U: 0, D: 1, L: 2, R: 3, F: 4, B: 5 };
const FACE_CONFIG: Record<FaceKey, FaceConfig> = {
  U: { axis: "y", layer: 1, axisVector: new THREE.Vector3(0, 1, 0), visualSign: -1 },
  D: { axis: "y", layer: -1, axisVector: new THREE.Vector3(0, 1, 0), visualSign: 1 },
  L: { axis: "x", layer: -1, axisVector: new THREE.Vector3(1, 0, 0), visualSign: 1 },
  R: { axis: "x", layer: 1, axisVector: new THREE.Vector3(1, 0, 0), visualSign: -1 },
  F: { axis: "z", layer: 1, axisVector: new THREE.Vector3(0, 0, 1), visualSign: -1 },
  B: { axis: "z", layer: -1, axisVector: new THREE.Vector3(0, 0, 1), visualSign: 1 },
};

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("App root not found");
}

app.innerHTML = `
  <aside class="panel">
    <p class="eyebrow">Rust + WebAssembly + Three.js</p>
    <h1 class="title">RustCube</h1>
    <p class="lede">
      The cube now turns as grouped slices instead of jumping state instantly.
      Rust still owns the canonical move history.
    </p>
    <div class="status-row">
      <span class="status-dot" data-status-dot></span>
      <span class="status-text" data-status-text>Loading Rust engine…</span>
    </div>
    <div class="actions">
      <button class="button" data-action="scramble">Scramble</button>
      <button class="button secondary" data-action="undo">Undo</button>
      <button class="button secondary" data-action="redo">Redo</button>
      <button class="button secondary" data-action="reset">Reset</button>
    </div>
    <div class="move-list">
      ${MOVE_SET.map((move) => `<button class="move-chip" data-move="${move}">${move}</button>`).join("")}
    </div>
    <section class="history">
      <p class="history-label">Move History</p>
      <p class="history-value" data-history>Waiting for engine…</p>
    </section>
  </aside>
  <main class="viewport">
    <div class="overlay" data-overlay>Loading the cube runtime…</div>
  </main>
`;

const viewport = requireElement<HTMLElement>(".viewport");
const historyValue = requireElement<HTMLElement>("[data-history]");
const overlay = requireElement<HTMLElement>("[data-overlay]");
const statusText = requireElement<HTMLElement>("[data-status-text]");
const statusDot = requireElement<HTMLElement>("[data-status-dot]");

const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
viewport.appendChild(renderer.domElement);

const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 100);
camera.position.set(5.6, 5.1, 6.8);
camera.lookAt(0, 0, 0);

scene.add(new THREE.AmbientLight(0xffffff, 1.28));

const keyLight = new THREE.DirectionalLight(0xfff1d8, 2.1);
keyLight.position.set(5, 8, 6);
scene.add(keyLight);

const fillLight = new THREE.DirectionalLight(0x7baeff, 1.15);
fillLight.position.set(-5, 4, -5);
scene.add(fillLight);

const cubeRoot = new THREE.Group();
cubeRoot.rotation.x = -0.55;
cubeRoot.rotation.y = 0.72;
scene.add(cubeRoot);

const frame = new THREE.Mesh(
  new THREE.BoxGeometry(3.32, 3.32, 3.32),
  new THREE.MeshBasicMaterial({
    color: 0x25324f,
    wireframe: true,
    transparent: true,
    opacity: 0.12,
  }),
);
scene.add(frame);

const cubieBodyGeometry = new THREE.BoxGeometry(0.96, 0.96, 0.96);
const cubieBodyMaterial = new THREE.MeshStandardMaterial({
  color: 0x191919,
  metalness: 0.08,
  roughness: 0.48,
});
const stickerGeometry = new THREE.PlaneGeometry(0.74, 0.74);
const stickerMaterials = PALETTE.map(
  (hex) =>
    new THREE.MeshStandardMaterial({
      color: new THREE.Color(hex),
      metalness: 0.04,
      roughness: 0.7,
    }),
);

const cubies: CubieNode[] = [];

let cube: CubeApi | null = null;
let moveQueue: MoveIntent[] = [];
let activeAnimation: AnimationState | null = null;
let overlayOverride: string | null = null;

buildCubies();
layoutSolvedCubies();

function setStatus(message: string, solved = false) {
  statusText.textContent = message;
  statusDot.classList.toggle("solved", solved);
}

function resize() {
  const { clientWidth, clientHeight } = viewport;
  camera.aspect = clientWidth / clientHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(clientWidth, clientHeight);
}

window.addEventListener("resize", resize);
resize();

function buildCubies() {
  cubies.length = 0;
  cubeRoot.clear();

  for (let x = -1; x <= 1; x += 1) {
    for (let y = -1; y <= 1; y += 1) {
      for (let z = -1; z <= 1; z += 1) {
        const cubieGroup = new THREE.Group();
        cubieGroup.add(new THREE.Mesh(cubieBodyGeometry, cubieBodyMaterial));

        addFaceSticker(cubieGroup, "U", x, y, z);
        addFaceSticker(cubieGroup, "D", x, y, z);
        addFaceSticker(cubieGroup, "L", x, y, z);
        addFaceSticker(cubieGroup, "R", x, y, z);
        addFaceSticker(cubieGroup, "F", x, y, z);
        addFaceSticker(cubieGroup, "B", x, y, z);

        const node: CubieNode = {
          mesh: cubieGroup,
          coord: new THREE.Vector3(x, y, z),
        };

        cubies.push(node);
        cubeRoot.add(cubieGroup);
      }
    }
  }
}

function addFaceSticker(group: any, face: FaceKey, x: number, y: number, z: number) {
  if (
    (face === "U" && y !== 1) ||
    (face === "D" && y !== -1) ||
    (face === "L" && x !== -1) ||
    (face === "R" && x !== 1) ||
    (face === "F" && z !== 1) ||
    (face === "B" && z !== -1)
  ) {
    return;
  }

  const sticker = new THREE.Mesh(stickerGeometry, stickerMaterials[0]);
  sticker.userData.face = face;

  if (face === "U") {
    sticker.position.set(0, 0.49, 0);
    sticker.rotation.x = -Math.PI / 2;
  } else if (face === "D") {
    sticker.position.set(0, -0.49, 0);
    sticker.rotation.x = Math.PI / 2;
  } else if (face === "L") {
    sticker.position.set(-0.49, 0, 0);
    sticker.rotation.y = -Math.PI / 2;
  } else if (face === "R") {
    sticker.position.set(0.49, 0, 0);
    sticker.rotation.y = Math.PI / 2;
  } else if (face === "F") {
    sticker.position.set(0, 0, 0.49);
  } else {
    sticker.position.set(0, 0, -0.49);
    sticker.rotation.y = Math.PI;
  }

  group.add(sticker);
}

function layoutSolvedCubies() {
  for (const cubie of cubies) {
    cubie.mesh.position.copy(cubie.coord);
    cubie.mesh.quaternion.identity();
    cubie.mesh.scale.setScalar(1);
    cubeRoot.add(cubie.mesh);
  }
}

function resetVisualCube() {
  activeAnimation = null;
  moveQueue = [];
  overlayOverride = null;

  for (const cubie of cubies) {
    cubeRoot.attach(cubie.mesh);
  }

  layoutSolvedCubies();
  updateStickerColors();
}

function updateStickerColors() {
  if (!cube) {
    return;
  }

  const stickers = Array.from(cube.stickers());

  for (const cubie of cubies) {
    for (const child of cubie.mesh.children) {
      if (!(child instanceof THREE.Mesh)) {
        continue;
      }

      const face = child.userData.face as FaceKey | undefined;
      if (!face) {
        continue;
      }

      const index = getStickerIndex(face, cubie.coord);
      const colorCode = stickers[index] ?? 0;
      child.material = stickerMaterials[colorCode] ?? stickerMaterials[0];
    }
  }
}

function refreshUi() {
  if (!cube) {
    return;
  }

  const history = cube.history().trim();
  historyValue.textContent = history || "No moves yet.";

  if (overlayOverride) {
    overlay.textContent = overlayOverride;
  } else if (cube.is_solved()) {
    overlay.textContent = "Solved. Rust state and cubie transforms match.";
  } else {
    overlay.textContent = history ? `History: ${history}` : "Fresh cube. Ready for a turn.";
  }

  if (cube.is_solved()) {
    setStatus("Solved", true);
  } else if (activeAnimation || moveQueue.length > 0) {
    setStatus("Turning…");
  } else {
    setStatus("Cube ready");
  }
}

function queueMove(notation: string, applyBeforeAnimation = true, overlayLabel?: string) {
  moveQueue.push({ notation, applyBeforeAnimation, overlayLabel });
  if (overlayLabel) {
    overlayOverride = overlayLabel;
  }
  startNextMove();
}

function queueSequence(sequence: string, applyBeforeAnimation = true, overlayLabel?: string) {
  const moves = sequence.split(/\s+/).filter(Boolean);
  if (moves.length === 0) {
    return;
  }

  overlayOverride = overlayLabel ?? null;
  for (const move of moves) {
    moveQueue.push({ notation: move, applyBeforeAnimation, overlayLabel });
  }
  startNextMove();
}

function startNextMove() {
  if (!cube || activeAnimation || moveQueue.length === 0) {
    refreshUi();
    return;
  }

  const next = moveQueue.shift()!;
  const parsed = parseNotation(next.notation);
  const config = FACE_CONFIG[parsed.face];

  if (next.applyBeforeAnimation) {
    cube.apply_move(next.notation);
  }

  const members = cubies.filter((cubie) => cubie.coord[config.axis] === config.layer);
  const group = new THREE.Group();
  cubeRoot.add(group);

  for (const cubie of members) {
    group.attach(cubie.mesh);
  }

  const quarterTurns = parsed.turn === "2" ? 2 : 1;
  const direction = parsed.turn === "'" ? -1 : 1;
  const targetAngle = config.visualSign * direction * quarterTurns * (Math.PI / 2);

  activeAnimation = {
    notation: next.notation,
    group,
    members,
    axis: config.axisVector.clone(),
    startTime: performance.now(),
    durationMs: quarterTurns === 2 ? 460 : 320,
    startAngle: 0,
    targetAngle,
  };

  overlayOverride = next.overlayLabel ?? null;
  refreshUi();
}

function handleAction(action: string) {
  if (!cube || activeAnimation) {
    return;
  }

  if (action === "scramble") {
    const scramble = cube.scramble(20);
    resetVisualCube();
    queueSequence(scramble, false, `Scramble: ${scramble}`);
    return;
  }

  if (action === "undo") {
    const move = cube.undo_move();
    if (move) {
      queueMove(move, false);
    }
    return;
  }

  if (action === "redo") {
    const move = cube.redo_move();
    if (move) {
      queueMove(move, false);
    }
    return;
  }

  if (action === "reset") {
    cube.reset();
    resetVisualCube();
    refreshUi();
  }
}

document.querySelectorAll<HTMLElement>("[data-action]").forEach((button) => {
  button.addEventListener("click", () => {
    const action = button.dataset.action;
    if (action) {
      handleAction(action);
    }
  });
});

document.querySelectorAll<HTMLElement>("[data-move]").forEach((button) => {
  button.addEventListener("click", () => {
    const move = button.dataset.move;
    if (!cube || !move || activeAnimation) {
      return;
    }

    queueMove(move, true);
  });
});

async function initCube() {
  try {
    const wasm = (await import(
      /* @vite-ignore */ "../../pkg/rustcube.js"
    )) as unknown as WasmBindings;
    await wasm.default();
    cube = new wasm.WasmCube();
    updateStickerColors();
    refreshUi();
  } catch (error) {
    console.error(error);
    overlay.textContent =
      "WASM build not found yet. Run npm run build:wasm after installing wasm-pack.";
    setStatus("WASM build missing");
    historyValue.textContent = "Build the Rust package to continue.";
  }
}

void initCube();

function animate(now: number) {
  frame.rotation.y += 0.0035;

  if (activeAnimation) {
    const progress = Math.min(1, (now - activeAnimation.startTime) / activeAnimation.durationMs);
    const eased = easeOutQuart(progress);
    const angle = THREE.MathUtils.lerp(
      activeAnimation.startAngle,
      activeAnimation.targetAngle,
      eased,
    );

    activeAnimation.group.setRotationFromAxisAngle(activeAnimation.axis, angle);

    if (progress >= 1) {
      finalizeAnimation();
    }
  }

  renderer.render(scene, camera);
  requestAnimationFrame(animate);
}

requestAnimationFrame(animate);

function finalizeAnimation() {
  if (!activeAnimation) {
    return;
  }

  const finished = activeAnimation;

  finished.group.setRotationFromAxisAngle(finished.axis, finished.targetAngle);

  for (const cubie of finished.members) {
    cubeRoot.attach(cubie.mesh);
    cubie.coord.copy(roundVector(cubie.mesh.position));
    cubie.mesh.position.copy(cubie.coord);
    cubie.mesh.quaternion.set(
      roundUnit(cubie.mesh.quaternion.x),
      roundUnit(cubie.mesh.quaternion.y),
      roundUnit(cubie.mesh.quaternion.z),
      roundUnit(cubie.mesh.quaternion.w),
    );
  }

  cubeRoot.remove(finished.group);
  activeAnimation = null;

  updateStickerColors();
  if (moveQueue.length === 0 && overlayOverride?.startsWith("Scramble:")) {
    overlayOverride = null;
  }
  refreshUi();
  startNextMove();
}

function parseNotation(notation: string) {
  const face = notation[0] as FaceKey;
  const turn = notation[1] === "'" || notation[1] === "2" ? notation[1] : "";
  return { face, turn: turn as "" | "'" | "2" };
}

function easeOutQuart(value: number) {
  return 1 - (1 - value) ** 4;
}

function roundVector(vector: any) {
  return new THREE.Vector3(Math.round(vector.x), Math.round(vector.y), Math.round(vector.z));
}

function roundUnit(value: number) {
  const rounded = Math.round(value * 1_000_000) / 1_000_000;
  return Math.abs(rounded) < 1e-6 ? 0 : rounded;
}

function getStickerIndex(face: FaceKey, coord: any) {
  const faceIndex = FACE_INDEX[face];
  const rowCol = faceRowCol(face, coord);
  return faceIndex * 9 + rowCol.row * 3 + rowCol.col;
}

function faceRowCol(face: FaceKey, coord: any) {
  if (face === "U") {
    return { row: coord.z + 1, col: coord.x + 1 };
  }
  if (face === "D") {
    return { row: 1 - coord.z, col: coord.x + 1 };
  }
  if (face === "L") {
    return { row: 1 - coord.y, col: coord.z + 1 };
  }
  if (face === "R") {
    return { row: 1 - coord.y, col: 1 - coord.z };
  }
  if (face === "F") {
    return { row: 1 - coord.y, col: coord.x + 1 };
  }
  return { row: 1 - coord.y, col: 1 - coord.x };
}

function requireElement<T extends Element>(selector: string) {
  const element = document.querySelector<T>(selector);
  if (!element) {
    throw new Error(`Missing required element: ${selector}`);
  }
  return element;
}
