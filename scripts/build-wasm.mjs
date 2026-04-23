import { spawnSync } from "node:child_process";

const versionCheck = spawnSync("wasm-pack", ["--version"], {
  stdio: "inherit",
});

if (versionCheck.status !== 0) {
  console.error(
    [
      "wasm-pack is required to build the browser package.",
      "Install it with: cargo install wasm-pack",
    ].join("\n"),
  );
  process.exit(versionCheck.status ?? 1);
}

const build = spawnSync(
  "wasm-pack",
  ["build", "--target", "web", "--out-dir", "pkg", "--release"],
  {
    stdio: "inherit",
  },
);

process.exit(build.status ?? 1);
