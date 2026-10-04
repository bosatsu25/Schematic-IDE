import { test, expect } from '@playwright/test';
import * as path from 'path';
import * as fs from 'fs';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

test.describe('End-to-End Litematic Browser Workflow', () => {
  test('Full Open -> 3D View -> Select -> Replace -> Preview -> Commit -> Undo -> Redo -> Cleanup -> Export -> Reload workflow', async ({
    page,
  }) => {
    // 1. Navigate to editor
    await page.goto('/');
    await expect(page.locator('.app-title')).toHaveText('Schematic IDE');
    await expect(page.getByTestId('empty-state')).toBeVisible();

    // 2. Open .litematic from local disk
    const fixturePath = path.resolve(__dirname, '../public/sample.litematic');
    expect(fs.existsSync(fixturePath)).toBe(true);

    const fileInput = page.getByTestId('open-file-input');
    await fileInput.setInputFiles(fixturePath);

    // 3. Lossless parse in Rust Core + 3D View
    await expect(page.getByTestId('doc-name')).toHaveText('WasmTestSchematic');
    await expect(page.getByTestId('region-block-count')).toHaveText('12');
    await expect(page.getByTestId('empty-state')).not.toBeVisible();
    await expect(page.getByTestId('viewport-canvas')).toBeVisible();

    // 3.5. Wave 8A Inspection & Diagnostics verification
    await page.getByTestId('tab-inspector').click();
    await expect(page.getByTestId('inspector-panel')).toBeVisible();
    await page.getByTestId('inspect-input-x').fill('0');
    await page.getByTestId('inspect-input-y').fill('1');
    await page.getByTestId('inspect-input-z').fill('1');
    await page.getByTestId('inspect-btn').click();
    await expect(page.getByTestId('inspect-block-id')).toHaveText('minecraft:stone');

    await page.getByTestId('tab-problems').click();
    await expect(page.getByTestId('problems-panel')).toBeVisible();

    // Switch back to regions tab
    await page.getByTestId('tab-regions').click();

    // 4. Region / Selection
    await page.getByTestId('select-all-btn').click();
    await expect(page.getByTestId('sel-min-x')).toHaveValue('0');
    await expect(page.getByTestId('sel-max-x')).toHaveValue('15');

    // 5. Replace: minecraft:stone -> minecraft:diorite
    await page.getByTestId('replace-from-input').fill('minecraft:stone');
    await page.getByTestId('replace-to-input').fill('minecraft:diorite');
    await page.getByTestId('preview-replace-btn').click();

    // 6. Before / After Preview
    await expect(page.getByTestId('preview-panel')).toBeVisible();
    await expect(page.getByTestId('preview-changed-count')).toHaveText('11');
    await expect(page.getByTestId('preview-indicator')).toBeVisible();

    // 7. Commit
    await page.getByTestId('commit-btn').click();
    await expect(page.getByTestId('preview-panel')).not.toBeVisible();
    await expect(page.getByTestId('dirty-indicator')).toBeVisible();
    await expect(page.getByTestId('undo-btn')).toBeEnabled();

    // 8. Undo
    await page.getByTestId('undo-btn').click();
    await expect(page.getByTestId('dirty-indicator')).not.toBeVisible();
    await expect(page.getByTestId('redo-btn')).toBeEnabled();

    // 9. Redo
    await page.getByTestId('redo-btn').click();
    await expect(page.getByTestId('dirty-indicator')).toBeVisible();

    // 9.5. Structural Editing (Fill, Copy, Paste, Move)
    await page.getByTestId('sel-min-x').fill('2');
    await page.getByTestId('sel-min-y').fill('2');
    await page.getByTestId('sel-min-z').fill('2');
    await page.getByTestId('sel-max-x').fill('3');
    await page.getByTestId('sel-max-y').fill('2');
    await page.getByTestId('sel-max-z').fill('3');
    await page.getByTestId('fill-block-input').fill('minecraft:oak_planks');
    await page.getByTestId('preview-fill-btn').click();
    await expect(page.getByTestId('preview-panel')).toBeVisible();
    await expect(page.getByTestId('preview-changed-count')).toHaveText('4');
    await page.getByTestId('commit-btn').click();
    await expect(page.getByTestId('preview-panel')).not.toBeVisible();

    // Copy selection
    await page.getByTestId('copy-selection-btn').click();
    await expect(page.getByTestId('copy-selection-btn')).toContainText('4 blocks copied');

    // Paste at (6, 2, 6)
    await page.getByTestId('paste-target-x').fill('6');
    await page.getByTestId('paste-target-y').fill('2');
    await page.getByTestId('paste-target-z').fill('6');
    await page.getByTestId('preview-paste-btn').click();
    await expect(page.getByTestId('preview-panel')).toBeVisible();
    await expect(page.getByTestId('preview-changed-count')).toHaveText('4');
    await page.getByTestId('commit-btn').click();

    // Move selection: select [6, 2, 6]..[7, 2, 7] and move by [0, 1, 0]
    await page.getByTestId('sel-min-x').fill('6');
    await page.getByTestId('sel-min-y').fill('2');
    await page.getByTestId('sel-min-z').fill('6');
    await page.getByTestId('sel-max-x').fill('7');
    await page.getByTestId('sel-max-y').fill('2');
    await page.getByTestId('sel-max-z').fill('7');
    await page.getByTestId('move-dx').fill('0');
    await page.getByTestId('move-dy').fill('1');
    await page.getByTestId('move-dz').fill('0');
    await page.getByTestId('preview-move-btn').click();
    await expect(page.getByTestId('preview-panel')).toBeVisible();
    await expect(page.getByTestId('preview-changed-count')).toHaveText('8');
    await page.getByTestId('commit-btn').click();

    // 10. VoxelWeave Cleanup Tool
    await page.getByTestId('cleanup-max-size-input').fill('2');
    await page.getByTestId('preview-cleanup-btn').click();
    await expect(page.getByTestId('preview-panel')).toBeVisible();
    await page.getByTestId('commit-btn').click();
    await expect(page.getByTestId('preview-panel')).not.toBeVisible();

    // 10.5. Canonical Diff Verification (Wave 8C)
    await page.getByTestId('tab-diff').click();
    await expect(page.getByTestId('diff-panel')).toBeVisible();
    await page.getByTestId('diff-source-btn').click();
    await expect(page.getByTestId('diff-stats')).toBeVisible();
    await expect(page.getByTestId('toggle-diff-overlay')).toBeVisible();
    await page.getByTestId('toggle-diff-overlay').click();
    await expect(page.getByTestId('toggle-diff-overlay')).toContainText('OFF');
    await page.getByTestId('toggle-diff-overlay').click();
    await expect(page.getByTestId('toggle-diff-overlay')).toContainText('ON');
    // Return to regions tab
    await page.getByTestId('tab-regions').click();

    // 11. Export as .litematic
    const downloadPromise = page.waitForEvent('download');
    await page.getByTestId('export-btn').click();
    const download = await downloadPromise;

    const exportPath = path.resolve(__dirname, '../test-results/exported.litematic');
    fs.mkdirSync(path.dirname(exportPath), { recursive: true });
    await download.saveAs(exportPath);

    expect(fs.existsSync(exportPath)).toBe(true);
    const exportedStats = fs.statSync(exportPath);
    expect(exportedStats.size).toBeGreaterThan(100);

    // 12. Reload exported file into browser and verify data retention
    await fileInput.setInputFiles(exportPath);

    await expect(page.getByTestId('doc-name')).toHaveText('WasmTestSchematic');
    await expect(page.getByTestId('region-selector')).toHaveValue('MainRegion');
    // After replace, fill, paste, move, and cleanup, non-air count should be exactly 18 blocks (12 original + 4 fill + 4 paste - 2 cleanup)
    const blockCountText = await page.getByTestId('region-block-count').textContent();
    const count = parseInt(blockCountText || '0', 10);
    expect(count).toBe(18);
  });
});
