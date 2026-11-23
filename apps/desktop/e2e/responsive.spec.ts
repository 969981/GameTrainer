import { expect, test } from "@playwright/test";

for (const width of [1100, 1366, 1500]) {
  test('M0 browser layout at ' + width + 'px and cross-platform navigation', async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "游戏库" })).toBeVisible();
    await expect(page.getByRole("button", { name: "打开 Pokémon Emerald" })).toBeVisible();
    await expect(page.getByText("M0 MOCK")).toBeVisible();

    const overflow = await page.evaluate(() => ({
      screen: window.innerWidth,
      html: document.documentElement.scrollWidth,
      body: document.body.scrollWidth,
    }));
    expect(overflow.html).toBeLessThanOrEqual(overflow.screen + 1);
    expect(overflow.body).toBeLessThanOrEqual(overflow.screen + 1);
    await page.screenshot({ path: 'test-results/screenshots/library-' + width + '.png', fullPage: true });

    await page.getByRole("button", { name: "Gen 6", exact: true }).last().click();
    await expect(page.getByRole("button", { name: "打开 Pokémon X / Y" })).toBeVisible();
    await expect(page.getByRole("button", { name: "打开 Pokémon Emerald" })).toHaveCount(0);
    await page.getByRole("button", { name: "打开 Pokémon X / Y" }).click();
    await expect(page.getByRole("button", { name: "队伍与盒子" })).toBeVisible();
    await expect(page.getByRole("button", { name: "RNG / Seed" })).toHaveCount(0);
    await page.screenshot({ path: 'test-results/screenshots/gen6-' + width + '.png', fullPage: true });

    await page.getByRole("button", { name: "游戏库" }).click();
    await page.getByRole("button", { name: "PC Native", exact: true }).last().click();
    await page.getByRole("button", { name: "打开 Windows Native · Template" }).click();
    await expect(page.getByRole("button", { name: "进程观察" })).toBeVisible();
    await expect(page.getByRole("button", { name: "RNG / Seed" })).toHaveCount(0);
    await page.screenshot({ path: 'test-results/screenshots/pc-' + width + '.png', fullPage: true });
  });
}
