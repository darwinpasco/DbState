import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const demoRoot = path.resolve(__dirname, "..");
const flowPath = path.join(demoRoot, "dbstate-demo-flow.resolved.json");
const outputPath = path.join(demoRoot, "output", "dbstate-demo-voiceover.mp3");

function commandExists(command) {
  const result = spawnSync(command, ["-version"], { encoding: "utf8" });
  return !result.error && result.status === 0;
}

if (!fs.existsSync(flowPath)) {
  console.error("Missing dbstate-demo-flow.resolved.json. Run npm run demo:resolve first.");
  process.exit(1);
}

if (!commandExists("ffmpeg")) {
  console.error("ffmpeg was not found. Install ffmpeg locally, then rerun npm run demo:concat-audio.");
  process.exit(1);
}

const flow = JSON.parse(fs.readFileSync(flowPath, "utf8"));
if (!flow.length) {
  console.error("The resolved demo flow is empty.");
  process.exit(1);
}

const args = ["-y"];
const filterInputs = [];

flow.forEach((step, index) => {
  if (step.audio) {
    const filePath = path.resolve(demoRoot, step.audio);
    if (!fs.existsSync(filePath)) {
      throw new Error(`Audio file not found for section "${step.section}": ${filePath}`);
    }
    args.push("-i", filePath);
  } else {
    args.push(
      "-f",
      "lavfi",
      "-t",
      String(Math.max(0.1, Number(step.duration || 0))),
      "-i",
      "anullsrc=channel_layout=stereo:sample_rate=44100"
    );
  }
  filterInputs.push(`[${index}:a]`);
});

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
args.push(
  "-filter_complex",
  `${filterInputs.join("")}concat=n=${flow.length}:v=0:a=1[a]`,
  "-map",
  "[a]",
  "-c:a",
  "libmp3lame",
  "-q:a",
  "2",
  outputPath
);

const result = spawnSync("ffmpeg", args, { stdio: "inherit" });
if (result.error || result.status !== 0) {
  process.exit(result.status || 1);
}

console.log(`Wrote ${path.relative(demoRoot, outputPath)}.`);
