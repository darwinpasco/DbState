# DbState Product Demo Runner

This folder contains an isolated Playwright runner for recording a DbState product demo. It does not change DbState runtime behavior, database logic, release logic, API contracts, UI copy, or safety posture.

The demo flow is driven by `dbstate-demo-flow.json`. The same file provides the timing and caption text used to generate the SRT subtitle file.

## Prerequisites

- Node.js
- Playwright Chromium browser dependencies
- ffmpeg and ffprobe for audio duration resolution, audio concatenation, and final video merging
- DbState running locally, for example on `http://localhost:8080`

DbState does not directly apply database changes. The demo runner only drives the browser UI.

## Setup

From the repository root:

```powershell
cd demo/video
npm install
npm run demo:install
```

`npm run demo:install` runs `playwright install chromium`.

## Run The UI Recording

Start DbState in another terminal, using port `8080` or your preferred port:

```powershell
cargo run -- serve --port 8080
```

Then record the demo:

```powershell
$env:DBSTATE_DEMO_URL = "http://localhost:8080"
npm run demo:record
```

Playwright video output is written under:

```text
demo/video/output/playwright-results/
```

Find the generated `video.webm` in the test result folder. Use that path as `DBSTATE_DEMO_VIDEO_PATH` when merging.

## Generate Subtitles

Generate the SRT file from the current timing file:

```powershell
npm run demo:srt
```

Output:

```text
demo/video/subtitles/dbstate-demo.srt
```

The SRT can be imported into a video editor directly. The script uses `dbstate-demo-flow.resolved.json` when it exists; otherwise it uses `dbstate-demo-flow.json`.

## Use AI Voice-Over Audio

Place generated section audio files under `demo/video/audio/`, then add an `audio` field to the matching step in `dbstate-demo-flow.json`, for example:

```json
{
  "section": "Workspace",
  "audio": "audio/workspace.mp3",
  "duration": 8,
  "subtitle": "The Workspace section tells DbState where the local project repository is."
}
```

Resolve real audio durations with ffprobe:

```powershell
npm run demo:resolve
```

Output:

```text
demo/video/dbstate-demo-flow.resolved.json
```

Regenerate the SRT after resolving durations:

```powershell
npm run demo:srt
```

## Concatenate Voice-Over Audio

After resolving durations, create one voice-over MP3:

```powershell
npm run demo:concat-audio
```

Output:

```text
demo/video/output/dbstate-demo-voiceover.mp3
```

For steps without an `audio` field, the script inserts silence for that step duration.

## Merge Video, Audio, And Captions

Pass the Playwright `video.webm` path through `DBSTATE_DEMO_VIDEO_PATH`:

```powershell
$env:DBSTATE_DEMO_VIDEO_PATH = "demo/video/output/playwright-results/<test-folder>/video.webm"
npm run demo:merge
```

Outputs:

```text
demo/video/output/dbstate-demo-with-optional-captions.mp4
demo/video/output/dbstate-demo-burned-in-captions.mp4
```

The optional captions MP4 contains a subtitle track. The burned-in captions MP4 renders subtitles directly into the video.

## Commands

```text
npm run demo:record        Record the UI with Playwright.
npm run demo:srt           Generate subtitles/dbstate-demo.srt.
npm run demo:resolve       Resolve audio durations into dbstate-demo-flow.resolved.json.
npm run demo:concat-audio  Build output/dbstate-demo-voiceover.mp3.
npm run demo:merge         Merge Playwright video, voice-over MP3, and SRT captions.
```
