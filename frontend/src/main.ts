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
  is_solved(): boolean;
  stickers(): Uint8Array | number[];
  history(): string;
};

const FACE_ORDER = ["U", "D", "L", "R", "F", "B"] as const;
const MOVE_SET = ["R", "U", "F", "L", "D", "B", "R'", "U'", "F2"] as const;
const PALETTE = ["#f5f7fb", "#ffd84d", "#ff8a3d", "#e44837", "#25b35c", "#2f6bff"];

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("App root not found");
}

app.innerHTML = `
  <aside class="panel">
    <p class="eyebrow">Rust + WebAssembly + Three.js</p>
    <h1 class="title">RustCube</h1>
    <p class="lede">
      Real cube state now lives in Rust. The browser renders sticker colors from
      WASM instead of a placeholder spin demo.
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
    <div class="overlay" data-overlay>
      Loading the cube runtime…
    </div>
  </main>
`;

const viewport = document.querySelector<HTMLElement>(".viewport");
const historyValue = document.querySelector<HTMLElement>("[data-history]");
const overlay = document.querySelector<HTMLElement>("[data-overlay]");
const statusText = document.querySelector<HTMLElement>("[data-status-text]");
const statusDot = document.querySelector<HTMLElement>("[data-status-dot]");

if (!viewport || !historyValue || !overlay || !statusText || !statusDot) {
  throw new Error("App shell not rendered correctly");
}

const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
viewport.appendChild(renderer.domElement);

const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 100);
camera.position.set(5.6, 5.2, 6.8);
camera.lookAt(0, 0, 0);

scene.add(new THREE.AmbientLight(0xffffff, 1.25));

const keyLight = new THREE.DirectionalLight(0xfff1d8, 2.1);
keyLight.position.set(5, 8, 6);
scene.add(keyLight);

const fillLight = new THREE.DirectionalLight(0x7baeff, 1.2);
fillLight.position.set(-5, 4, -5);
scene.add(fillLight);

const cubeRoot = new THREE.Group();
cubeRoot.rotation.x = -0.55;
cubeRoot.rotation.y = 0.72;
scene.add(cubeRoot);

const cubieGeometry = new THREE.BoxGeometry(0.94, 0.94, 0.94);
const cubieMaterial = new THREE.MeshStandardMaterial({
  color: 0x121212,
  metalness: 0.08,
  roughness: 0.45,
});

const stickerGeometry = new THREE.PlaneGeometry(0.72, 0.72);
const stickerMaterials = PALETTE.map(
  (hex) =>
    new THREE.MeshStandardMaterial({
      color: new THREE.Color(hex),
      metalness: 0.05,
      roughness: 0.7,
    }),
);

const stickerMeshes: THREE.Mesh[] = [];

for (let x = -1; x <= 1; x += 1) {
  for (let y = -1; y <= 1; y += 1) {
    for (let z = -1; z <= 1; z += 1) {
      const cubie = new THREE.Mesh(cubieGeometry, cubieMaterial);
      cubie.position.set(x, y, z);
      cubeRoot.add(cubie);
    }
  }
}

const faceAnchors = FACE_ORDER.flatMap((face) => buildFaceAnchors(face));
faceAnchors.forEach((anchor) => {
  const sticker = new THREE.Mesh(stickerGeometry, stickerMaterials[0]);
  sticker.position.copy(anchor.position);
  sticker.quaternion.copy(anchor.rotation);
  stickerMeshes.push(sticker);
  cubeRoot.add(sticker);
});

const frame = new THREE.Mesh(
  new THREE.BoxGeometry(3.26, 3.26, 3.26),
  new THREE.MeshBasicMaterial({
    color: 0x25324f,
    wireframe: true,
    transparent: true,
    opacity: 0.14,
  }),
);
scene.add(frame);

function resize() {
  const { clientWidth, clientHeight } = viewport;
  camera.aspect = clientWidth / clientHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(clientWidth, clientHeight);
}

window.addEventListener("resize", resize);
resize();

let cube: CubeApi | null = null;

function setStatus(message: string, solved = false) {
  statusText.textContent = message;
  statusDot.classList.toggle("solved", solved);
}

function paintCube() {
  if (!cube) {
    return;
  }

  const stickers = Array.from(cube.stickers());
  stickers.forEach((code, index) => {
    stickerMeshes[index].material = stickerMaterials[code] ?? stickerMaterials[0];
  });

  const history = cube.history().trim();
  historyValue.textContent = history || "No moves yet.";

  if (cube.is_solved()) {
    overlay.textContent = "Solved. The Rust state and renderer are in sync.";
    setStatus("Solved", true);
  } else {
    overlay.textContent = `History: ${history || "fresh cube"}`;
    setStatus("Cube ready");
  }
}

function handleAction(action: string) {
  if (!cube) {
    return;
  }

  if (action === "scramble") {
    const scramble = cube.scramble(20);
    overlay.textContent = `Scramble: ${scramble}`;
  } else if (action === "undo") {
    cube.undo();
  } else if (action === "redo") {
    cube.redo();
  } else if (action === "reset") {
    cube.reset();
  }

  paintCube();
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
    if (!cube) {
      return;
    }

    const move = button.dataset.move;
    if (!move) {
      return;
    }

    cube.apply_move(move);
    paintCube();
  });
});

async function initCube() {
  try {
    const wasm = (await import(
      /* @vite-ignore */ "../../pkg/rustcube.js"
    )) as WasmBindings;
    await wasm.default();
    cube = new wasm.WasmCube();
    paintCube();
  } catch (error) {
    console.error(error);
    overlay.textContent =
      "WASM build not found yet. Run npm run build:wasm after installing wasm-pack.";
    setStatus("WASM build missing");
    historyValue.textContent = "Build the Rust package to continue.";
  }
}

initCube();

let tick = 0;

function animate() {
  tick += 0.01;
  frame.rotation.y = tick * 0.4;
  renderer.render(scene, camera);
  requestAnimationFrame(animate);
}

animate();

function buildFaceAnchors(face: (typeof FACE_ORDER)[number]) {
  const anchors: { position: THREE.Vector3; rotation: THREE.Quaternion }[] = [];

  for (let row = 0; row < 3; row += 1) {
    for (let col = 0; col < 3; col += 1) {
      const local = new THREE.Vector3((col - 1) * 1, (1 - row) * 1, 0);
      const position = new THREE.Vector3();
      const normal = new THREE.Vector3();
      const rotation = new THREE.Quaternion();

      if (face === "U") {
        position.set(local.x, 1.48, row - 1);
        normal.set(0, 1, 0);
        rotation.setFromEuler(new THREE.Euler(-Math.PI / 2, 0, 0));
      } else if (face === "D") {
        position.set(local.x, -1.48, 1 - row);
        normal.set(0, -1, 0);
        rotation.setFromEuler(new THREE.Euler(Math.PI / 2, 0, 0));
      } else if (face === "L") {
        position.set(-1.48, 1 - row, col - 1);
        normal.set(-1, 0, 0);
        rotation.setFromEuler(new THREE.Euler(0, -Math.PI / 2, 0));
      } else if (face === "R") {
        position.set(1.48, 1 - row, 1 - col);
        normal.set(1, 0, 0);
        rotation.setFromEuler(new THREE.Euler(0, Math.PI / 2, 0));
      } else if (face === "F") {
        position.set(col - 1, 1 - row, 1.48);
        normal.set(0, 0, 1);
        rotation.setFromEuler(new THREE.Euler(0, 0, 0));
      } else {
        position.set(1 - col, 1 - row, -1.48);
        normal.set(0, 0, -1);
        rotation.setFromEuler(new THREE.Euler(0, Math.PI, 0));
      }

      position.addScaledVector(normal, 0.01);
      anchors.push({ position, rotation });
    }
  }

  return anchors;
}
