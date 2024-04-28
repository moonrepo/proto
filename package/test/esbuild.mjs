import { build } from "esbuild";

await build({
  entryPoints: ["src/index.ts"],
  outdir: "dist",
  bundle: true,
  // Definitely minify for production builds, as each kb of js for some reason makes the wasm 18kb larger.
  minify: true,
  format: "cjs", // extism-js only supports cjs currently
  target: ["es2020"], // don't go over es2020 because quickjs doesn't support it
});
