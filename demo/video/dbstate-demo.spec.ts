import { expect, Locator, Page, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

type DemoStep = {
  section: string;
  action?: "click";
  target?: string;
  fallbackTarget?: string;
  duration: number;
  subtitle: string;
  audio?: string;
};

type ResolvedLocator = {
  locator: Locator;
  testId: string;
};

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const flowPath = path.join(__dirname, "dbstate-demo-flow.json");
const flow = JSON.parse(fs.readFileSync(flowPath, "utf8")) as DemoStep[];

async function locatorIsVisible(locator: Locator, timeout = 5_000): Promise<boolean> {
  try {
    await locator.waitFor({ state: "visible", timeout });
    return true;
  } catch {
    return false;
  }
}

async function getDemoLocator(page: Page, step: DemoStep): Promise<ResolvedLocator | null> {
  if (!step.target) {
    return null;
  }

  await prepareStep(page, step);

  const primary = page.getByTestId(step.target);
  if (await locatorIsVisible(primary)) {
    return { locator: primary, testId: step.target };
  }

  if (step.fallbackTarget) {
    const fallback = page.getByTestId(step.fallbackTarget);
    if (await locatorIsVisible(fallback)) {
      return { locator: fallback, testId: step.fallbackTarget };
    }
  }

  const fallbackMessage = step.fallbackTarget ? ` or fallback "${step.fallbackTarget}"` : "";
  throw new Error(`Demo step "${step.section}" could not find visible test id "${step.target}"${fallbackMessage}.`);
}

async function safeScrollIntoView(locator: Locator): Promise<void> {
  await locator.scrollIntoViewIfNeeded();
}

async function selectWorkflowMode(page: Page, value: string): Promise<void> {
  await page.getByTestId("tab-source-target").click();
  await page.getByTestId("workflow-mode").selectOption(value);
}

async function prepareStep(page: Page, step: DemoStep): Promise<void> {
  if (step.target === "source-target-run") {
    await page.getByTestId("tab-source-target").click();
    await page.getByTestId("connection-mode").selectOption("profile");
    return;
  }

  if (step.target === "preview-repository-sync" || step.target === "write-repository-changes") {
    await selectWorkflowMode(page, "databaseToRepository");
    await page.getByTestId("tab-compare-options").click();
    return;
  }

  if (
    step.target === "release-plan-dry-run" ||
    step.target === "release-context" ||
    step.target === "risk-summary" ||
    step.target === "object-summary" ||
    step.target === "release-candidates" ||
    step.target === "generated-artifacts" ||
    step.target === "generate-release-artifact"
  ) {
    await selectWorkflowMode(page, "compare");
    await page.getByTestId("tab-release-plan").click();
  }
}

async function clickStep(step: DemoStep, resolved: ResolvedLocator | null): Promise<void> {
  if (!resolved) {
    return;
  }

  await safeScrollIntoView(resolved.locator);
  if (step.action !== "click") {
    return;
  }

  if (await resolved.locator.isDisabled()) {
    await resolved.locator.focus();
    return;
  }

  await resolved.locator.click();
}

async function pauseStep(page: Page, step: DemoStep): Promise<void> {
  await page.waitForTimeout(Math.max(0, step.duration) * 1000);
}

test.describe("DbState product demo", () => {
  test("records the deterministic product walkthrough", async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("tab-workspace")).toBeVisible();

    for (const step of flow) {
      await test.step(step.section, async () => {
        const resolved = await getDemoLocator(page, step);
        if (resolved) {
          await clickStep(step, resolved);
        }
        await pauseStep(page, step);
      });
    }
  });
});
