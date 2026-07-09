import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  testMatch: /dbstate-demo\.spec\.ts/,
  timeout: 10 * 60 * 1000,
  expect: {
    timeout: 10_000
  },
  outputDir: "output/playwright-results",
  reporter: [["list"], ["html", { outputFolder: "playwright-report", open: "never" }]],
  use: {
    baseURL: process.env.DBSTATE_DEMO_URL || "http://localhost:8080",
    viewport: { width: 1920, height: 1080 },
    video: "on",
    screenshot: "only-on-failure",
    trace: "retain-on-failure"
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        browserName: "chromium",
        viewport: { width: 1920, height: 1080 }
      }
    }
  ]
});
