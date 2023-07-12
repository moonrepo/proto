const fs = require("fs");
const path = require("path");

for (let file of [
  "plugins/target/debug",
  "plugins/target/wasm32-wasi/debug/.fingerprint",
  "plugins/target/wasm32-wasi/debug/build",
  "plugins/target/wasm32-wasi/debug/deps",
  "plugins/target/wasm32-wasi/debug/incremental",
]) {
  try {
    fs.rmSync(path.join(process.env.GITHUB_WORKSPACE || process.cwd(), file), {
      recursive: true,
      force: true,
    });
  } catch (e) {
    console.error(e);
  }
}
