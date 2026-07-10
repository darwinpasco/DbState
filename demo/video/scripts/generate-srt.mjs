import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const demoRoot = path.resolve(__dirname, "..");
const resolvedFlowPath = path.join(demoRoot, "dbstate-demo-flow.resolved.json");
const baseFlowPath = path.join(demoRoot, "dbstate-demo-flow.json");
const inputPath = fs.existsSync(resolvedFlowPath) ? resolvedFlowPath : baseFlowPath;
const outputPath = path.join(demoRoot, "subtitles", "dbstate-demo.srt");

function readFlow(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function timestamp(ms) {
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const millis = Math.floor(ms % 1000);
  return [
    String(hours).padStart(2, "0"),
    String(minutes).padStart(2, "0"),
    String(seconds).padStart(2, "0")
  ].join(":") + "," + String(millis).padStart(3, "0");
}

function splitSubtitle(text, width = 54) {
  const words = String(text || "").split(/\s+/).filter(Boolean);
  const lines = [];
  let current = "";

  for (const word of words) {
    const candidate = current ? `${current} ${word}` : word;
    if (candidate.length > width && current) {
      lines.push(current);
      current = word;
    } else {
      current = candidate;
    }
  }

  if (current) {
    lines.push(current);
  }

  return lines.join("\n");
}

const flow = readFlow(inputPath);
let cursorMs = 0;
const blocks = flow.map((step, index) => {
  const durationMs = Math.round(Number(step.duration || 0) * 1000);
  const startMs = cursorMs;
  const endMs = cursorMs + durationMs;
  cursorMs = endMs;
  return [
    String(index + 1),
    `${timestamp(startMs)} --> ${timestamp(endMs)}`,
    splitSubtitle(step.subtitle),
    ""
  ].join("\n");
});

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, blocks.join("\n"), "utf8");
console.log(`Wrote ${path.relative(demoRoot, outputPath)} from ${path.basename(inputPath)}.`);
