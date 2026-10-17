// Synthetic reviewed authority is confined to the disposable example driver.
// These are control tests; no human content review is claimed.
import { expect } from "@playwright/test";
import { mkdirSync } from "node:fs";

export async function checkReview(page) {
  const nav = (name) => page.getByRole("button", { name, exact: true });
  await expect(
    page.getByRole("heading", { name: "Your learning path" }),
  ).toBeVisible();
  await expect(
    page
      .getByText("Synthetic review test - no human review", { exact: true })
      .first(),
  ).toBeVisible();
  await nav("Review").click();
  await expect(
    page.getByRole("heading", { name: "0 items due" }),
  ).toBeVisible();
  await expect(
    page.getByText("2 eligible items", { exact: false }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Review due items →", exact: true }),
  ).toBeDisabled();
  await nav("Practice available items").click();
  await page.getByRole("radio", { name: /Good evening/ }).check();
  await nav("Check answer").click();
  await expect(
    page.getByText("Try again — unlimited retries", { exact: true }),
  ).toBeVisible();
  await nav("Show hint").click();
  await nav("← Save & exit").click();
  await page.reload();
  await nav("Resume saved review →").click();
  await nav("Resume saved review →").click();
  await expect(page.getByRole("radio", { name: /Good evening/ })).toBeChecked();
  await expect(
    page.getByText("Try again — unlimited retries", { exact: true }),
  ).toBeVisible();
  await page.getByRole("radio", { name: /A coffee, please/ }).check();
  await nav("Check answer").click();
  await expect(
    page.getByRole("checkbox", { name: /This felt easy/ }),
  ).toHaveCount(0);
  await nav("Continue →").click();
  await page
    .getByRole("textbox", { name: "Your answer", exact: true })
    .fill("Un ca");
  await nav("← Save & exit").click();
  await nav("Resume saved review →").click();
  await expect(
    page.getByRole("textbox", { name: "Your answer", exact: true }),
  ).toHaveValue("Un ca");
  await page
    .getByRole("textbox", { name: "Your answer", exact: true })
    .fill("Un caffè, per favore");
  await nav("Check answer").click();
  await page.getByRole("checkbox", { name: /This felt easy/ }).check();
  await nav("Finish review →").click();
  await expect(
    page.getByRole("heading", { name: "Review complete", exact: true }),
  ).toBeVisible();
  await expect(page.getByText("5 XP earned.", { exact: true })).toBeVisible();
  await nav("Back to review →").click();
  await nav("Practice available items").click();
  await nav("Review solution").click();
  await nav("Continue →").click();
  await nav("Review solution").click();
  await nav("Finish review →").click();
  await expect(page.getByText("0 XP earned.", { exact: true })).toBeVisible();
  await nav("Back to review →").click();
  await nav("Practice available items").click();
  await nav("End this review without completing").click();
  await nav("Keep reviewing").click();
  await nav("End this review without completing").click();
  await nav("End review").click();
  await expect(
    page.getByRole("heading", { name: "Review ended", exact: true }),
  ).toBeVisible();
  await expect(page.getByText("0 XP earned.", { exact: true })).toBeVisible();
  await nav("Progress").click();
  await expect(
    page.getByText("First review · complete a review session", { exact: true }),
  ).toBeVisible();
  await nav("Settings").click();
  await nav("Export backup").click();
  await expect(
    page.getByText("Backup saved to", { exact: false }),
  ).toBeVisible();
  await nav("Choose backup to restore").click();
  await expect(
    page.getByText("Saved review sessions", { exact: true }),
  ).toBeVisible();
  await nav("Cancel restore").click();
  await nav("Review").click();
  await page.setViewportSize({ width: 720, height: 900 });
  mkdirSync("test-results", { recursive: true });
  await page.screenshot({
    path: "test-results/synthetic-review.png",
    fullPage: true,
  });
  console.log(
    "PASS synthetic review: preview, correction/hint, reload/resume/draft, explicit Easy, 5-XP cap, reveal, abort, achievement, validated export and 720px layout",
  );
}
