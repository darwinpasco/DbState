import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const demoRoot = path.resolve(__dirname, "..");
const flowPath = path.join(demoRoot, "dbstate-demo-flow.json");
const outputPath = path.join(demoRoot, "dbstate-demo-flow.resolved.json");

function commandExists(command) {
  const result = spawnSync(command, ["-version"], { encoding: "utf8" });
  return !result.error && result.status === 0;
}

function audioPath(step) {
  return path.resolve(demoRoot, step.audio);
}

function measureAudioSeconds(filePath) {
  const result = spawnSync(
    "ffprobe",
    ["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1", filePath],
    { encoding: "utf8" }
  );

  if (result.error || result.status !== 0) {
    const detail = result.stderr || result.error?.message || "unknown ffprobe error";
    throw new Error(`ffprobe failed for ${filePath}: ${detail}`);
  }

  const value = Number.parseFloat(result.stdout.trim());
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error(`ffprobe returned an invalid duration for ${filePath}.`);
  }
  return Math.round(value * 1000) / 1000;
}

const flow = JSON.parse(fs.readFileSync(flowPath, "utf8"));
const stepsWithAudio = flow.filter((step) => step.audio);

if (stepsWithAudio.length && !commandExists("ffprobe")) {
  console.error("ffprobe was not found. Install ffmpeg locally, then rerun npm run demo:resolve.");
  process.exit(1);
}

const resolved = flow.map((step) => {
  if (!step.audio) {
    return step;
  }

  const filePath = audioPath(step);
  if (!fs.existsSync(filePath)) {
    throw new Error(`Audio file not found for section "${step.section}": ${filePath}`);
  }

  return {
    ...step,
    duration: measureAudioSeconds(filePath)
  };
});

fs.writeFileSync(outputPath, `${JSON.stringify(resolved, null, 2)}\n`, "utf8");
console.log(`Wrote ${path.relative(demoRoot, outputPath)}. ffprobe requires ffmpeg installed locally.`);
