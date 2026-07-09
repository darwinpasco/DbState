import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const demoRoot = path.resolve(__dirname, "..");
const videoPath = process.env.DBSTATE_DEMO_VIDEO_PATH;
const audioPath = path.join(demoRoot, "output", "dbstate-demo-voiceover.mp3");
const srtPath = path.join(demoRoot, "subtitles", "dbstate-demo.srt");
const optionalCaptionsPath = path.join(demoRoot, "output", "dbstate-demo-with-optional-captions.mp4");
const burnedCaptionsPath = path.join(demoRoot, "output", "dbstate-demo-burned-in-captions.mp4");

function commandExists(command) {
  const result = spawnSync(command, ["-version"], { encoding: "utf8" });
  return !result.error && result.status === 0;
}

function runFfmpeg(args) {
  const result = spawnSync("ffmpeg", args, { stdio: "inherit" });
  if (result.error || result.status !== 0) {
    process.exit(result.status || 1);
  }
}

function subtitlesFilterPath(filePath) {
  const normalized = filePath.replace(/\\/g, "/").replace(/:/g, "\\:");
  return `subtitles='${normalized.replace(/'/g, "\\'")}'`;
}

if (!videoPath) {
  console.error("Set DBSTATE_DEMO_VIDEO_PATH to the Playwright video.webm path, then rerun npm run demo:merge.");
  console.error("Playwright videos are written under demo/video/output/playwright-results/.");
  process.exit(1);
}

for (const requiredPath of [videoPath, audioPath, srtPath]) {
  if (!fs.existsSync(requiredPath)) {
    console.error(`Required file not found: ${requiredPath}`);
    process.exit(1);
  }
}

if (!commandExists("ffmpeg")) {
  console.error("ffmpeg was not found. Install ffmpeg locally, then rerun npm run demo:merge.");
  process.exit(1);
}

fs.mkdirSync(path.dirname(optionalCaptionsPath), { recursive: true });

runFfmpeg([
  "-y",
  "-i",
  videoPath,
  "-i",
  audioPath,
  "-i",
  srtPath,
  "-map",
  "0:v:0",
  "-map",
  "1:a:0",
  "-map",
  "2:0",
  "-c:v",
  "copy",
  "-c:a",
  "aac",
  "-c:s",
  "mov_text",
  "-shortest",
  optionalCaptionsPath
]);

runFfmpeg([
  "-y",
  "-i",
  videoPath,
  "-i",
  audioPath,
  "-vf",
  subtitlesFilterPath(srtPath),
  "-map",
  "0:v:0",
  "-map",
  "1:a:0",
  "-c:v",
  "libx264",
  "-c:a",
  "aac",
  "-shortest",
  burnedCaptionsPath
]);

console.log(`Wrote ${path.relative(demoRoot, optionalCaptionsPath)}.`);
console.log(`Wrote ${path.relative(demoRoot, burnedCaptionsPath)}.`);
