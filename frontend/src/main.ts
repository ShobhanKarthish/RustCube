import "./style.css";
import * as THREE from "three";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("App root not found");
}

app.innerHTML = `
  <aside class="panel">
    <p class="eyebrow">Rust + WebAssembly + Three.js</p>
    <h1 class="title">RustCube</h1>
    <p class="lede">
      Starter shell for a browser Rubik's Cube with Rust as the canonical move
      engine and Three.js driving the scene.
    </p>
    <div class="actions">
      <button class="button" data-action="scramble">Scramble</button>
      <button class="button secondary" data-action="undo">Undo</button>
      <button class="button secondary" data-action="reset">Reset</button>
    </div>
    <div class="move-list">
      <div class="move-chip">R</div>
      <div class="move-chip">U</div>
      <div class="move-chip">F'</div>
      <div class="move-chip">L2</div>
      <div class="move-chip">D</div>
      <div class="move-chip">B'</div>
    </div>
  </aside>
  <main class="viewport">
    <div class="overlay">
      Scene shell ready. Next step: wire moves from the Rust WASM cube engine.
    </div>
  </main>
`;

const viewport = document.querySelector<HTMLElement>(".viewport");

if (!viewport) {
  throw new Error("Viewport not found");
}

const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
viewport.appendChild(renderer.domElement);

const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 100);
camera.position.set(5, 5, 6);
camera.lookAt(0, 0, 0);

scene.add(new THREE.AmbientLight(0xffffff, 1.4));

const keyLight = new THREE.DirectionalLight(0xfff0d6, 2.2);
keyLight.position.set(5, 8, 6);
scene.add(keyLight);

const rimLight = new THREE.DirectionalLight(0x7bb0ff, 1.6);
rimLight.position.set(-5, -4, -6);
scene.add(rimLight);

const cubeRoot = new THREE.Group();
scene.add(cubeRoot);

const cubieGeometry = new THREE.BoxGeometry(0.94, 0.94, 0.94);
const bodyMaterial = new THREE.MeshStandardMaterial({
  color: 0x111111,
  metalness: 0.15,
  roughness: 0.5,
});

for (let x = -1; x <= 1; x += 1) {
  for (let y = -1; y <= 1; y += 1) {
    for (let z = -1; z <= 1; z += 1) {
      const cubie = new THREE.Mesh(cubieGeometry, bodyMaterial);
      cubie.position.set(x, y, z);
      cubeRoot.add(cubie);
    }
  }
}

const frame = new THREE.Mesh(
  new THREE.BoxGeometry(3.25, 3.25, 3.25),
  new THREE.MeshBasicMaterial({
    color: 0x24304d,
    wireframe: true,
    transparent: true,
    opacity: 0.18,
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

let tick = 0;

function animate() {
  tick += 0.01;
  cubeRoot.rotation.y = tick;
  cubeRoot.rotation.x = Math.sin(tick * 0.5) * 0.2 - 0.4;
  renderer.render(scene, camera);
  requestAnimationFrame(animate);
}

animate();
