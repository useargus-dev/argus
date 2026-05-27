import { Resvg } from "@resvg/resvg-js";
import { readFileSync, writeFileSync } from "fs";
import { dirname, resolve } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));

const size = 1024;
const padding = 64;
const iconSize = size - padding * 2;

const tokenPath = resolve(__dirname, "../public/token.svg");
const tokenSvg = readFileSync(tokenPath, "utf8");
const pathMatch = tokenSvg.match(/<path[^>]*d="([^"]+)"[^>]*\/>/);
if (!pathMatch) {
  throw new Error("Could not find path in public/token.svg");
}

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">
  <g transform="translate(${padding}, ${padding})">
    <svg width="${iconSize}" height="${iconSize}" viewBox="0 -960 960 960" fill="#000000">
      <path d="${pathMatch[1]}"/>
    </svg>
  </g>
</svg>`;

const resvg = new Resvg(svg, {
  fitTo: { mode: "width", value: size },
});

const pngData = resvg.render();
const pngBuffer = pngData.asPng();

const outPath = resolve(__dirname, "../src-tauri/icons/icon.png");
writeFileSync(outPath, pngBuffer);
console.log(`Generated ${outPath} (${pngBuffer.length} bytes)`);
