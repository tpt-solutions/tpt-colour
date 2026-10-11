// Node smoke test for the generated ES module: run with `node playground/smoke.mjs`.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import * as wasm from "./pkg/tpt_colour_wasm.js";

const here = dirname(fileURLToPath(import.meta.url));
wasm.initSync({ module: readFileSync(join(here, "pkg", "tpt_colour_wasm_bg.wasm")) });

const check = (label, ok) => {
  if (!ok) throw new Error(`smoke check failed: ${label}`);
  console.log(`ok  ${label}`);
};

check("parseColour", wasm.parseColour("#ff8000").join() === "255,128,0,255");
check("wcagContrast black/white", Math.abs(wasm.wcagContrast([0, 0, 0], [1, 1, 1]) - 21) < 0.01);
check("apcaContrast black on white", Math.abs(wasm.apcaContrast([0, 0, 0], [1, 1, 1]) - 106) < 0.5);
check("simulateDeficiency keeps greys", Math.abs(wasm.simulateDeficiency([0.5, 0.5, 0.5], "tritanopia")[0] - 0.5) < 1e-3);
check("harmony triadic length", wasm.harmony([1, 0, 0], "triadic").length === 6);
const m = new wasm.Manager();
const p3 = m.transformRgb("srgb", "display-p3", "perceptual", [0.8, 0.2, 0.1]);
check("Manager transformRgb", p3.length === 3);
