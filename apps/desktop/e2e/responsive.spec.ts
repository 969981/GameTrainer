import { expect, test } from "@playwright/test";

for (const width of [1100, 1366, 1500]) {
  test('M0 browser layout at ' + width + 'px and cross-platform navigation', async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "游戏库" })).toBeVisible();
    await expect(page.getByRole("button", { name: "打开 Pokémon Emerald" })).toBeVisible();
    await expect(page.locator(".demo-badge")).toBeVisible();

    const overflow = await page.evaluate(() => ({
      screen: window.innerWidth,
      html: document.documentElement.scrollWidth,
      body: document.body.scrollWidth,
    }));
    expect(overflow.html).toBeLessThanOrEqual(overflow.screen + 1);
    expect(overflow.body).toBeLessThanOrEqual(overflow.screen + 1);
    await page.screenshot({ path: 'test-results/screenshots/library-' + width + '.png', fullPage: true });

    await page.getByRole("button", { name: "Gen 3", exact: true }).last().click();
    await expect(page.getByRole("button", { name: "打开 Pokémon Ruby" })).toBeVisible();
    await expect(page.getByRole("button", { name: "打开 Pokémon Sapphire" })).toBeVisible();
    await page.getByRole("button", { name: "打开 Pokémon Ruby" }).click();
    await expect(page.getByRole("button", { name: "我方必会心（实验研究）" })).toBeVisible();
    await page.screenshot({ path: 'test-results/screenshots/ruby-' + width + '.png', fullPage: true });
    await page.getByRole("button", { name: "游戏库" }).click();
    await page.getByRole("button", { name: "打开 Pokémon Sapphire" }).click();
    await expect(page.getByRole("button", { name: "WISHMKR 存档 RNG" })).toBeVisible();
    await page.screenshot({ path: 'test-results/screenshots/sapphire-' + width + '.png', fullPage: true });
    await page.getByRole("button", { name: "游戏库" }).click();
    await page.getByRole("button", { name: "Gen 6", exact: true }).last().click();
    await expect(page.getByRole("button", { name: "打开 Pokémon X / Y" })).toBeVisible();
    await expect(page.getByRole("button", { name: "打开 Pokémon Emerald" })).toHaveCount(0);
    await page.getByRole("button", { name: "打开 Pokémon X / Y" }).click();
    await expect(page.getByRole("button", { name: "队伍与盒子", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "RNG / Seed" })).toHaveCount(0);
    await page.screenshot({ path: 'test-results/screenshots/gen6-' + width + '.png', fullPage: true });

    await page.getByRole("button", { name: "游戏库" }).click();
    await page.getByRole("button", { name: "PC Native", exact: true }).last().click();
    await page.getByRole("button", { name: "打开 Windows Native · Template" }).click();
    await expect(page.getByRole("button", { name: "进程观察", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "RNG / Seed" })).toHaveCount(0);
    await page.screenshot({ path: 'test-results/screenshots/pc-' + width + '.png', fullPage: true });

    await page.getByRole("button", { name: "补丁实验室" }).click();
    await expect(page.getByRole("heading", { name: "安全补丁实验室" })).toBeVisible();
    await expect(page.getByText("浏览器仅能预览 M2 设计")).toBeVisible();
    await expect(page.getByRole("button", { name: "生成后端预览" })).toBeDisabled();
    const m2Overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(m2Overflow).toBeLessThanOrEqual(1);
    await page.screenshot({ path: 'test-results/screenshots/m2-' + width + '.png', fullPage: true });
  });
}
