import { test, expect, Page } from '@playwright/test';
import * as path from 'path';
import * as fs from 'fs';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURE = path.resolve(__dirname, 'fixtures/multi_region.litematic');

type Vec3 = [number, number, number];

/** Client coordinates of a world point, via the dev-only viewport hook. */
async function project(page: Page, world: Vec3): Promise<{ x: number; y: number }> {
  const p = await page.evaluate((w) => {
    const vp = (window as unknown as {
      __schematicViewport?: { projectToClient(w: Vec3): { x: number; y: number } | null };
    }).__schematicViewport;
    return vp?.projectToClient(w) ?? null;
  }, world);
  expect(p).not.toBeNull();
  return p!;
}

/** Clicks the top face of the block at world `block`. */
async function clickBlockTop(page: Page, block: Vec3, modifiers: ('Shift' | 'Control')[] = []) {
  const p = await project(page, [block[0] + 0.5, block[1] + 1, block[2] + 0.5]);
  await page.mouse.move(p.x, p.y);
  for (const m of modifiers) await page.keyboard.down(m);
  await page.mouse.down();
  await page.mouse.up();
  for (const m of modifiers) await page.keyboard.up(m);
}

async function selectionInputs(page: Page) {
  const ids = ['sel-min-x', 'sel-min-y', 'sel-min-z', 'sel-max-x', 'sel-max-y', 'sel-max-z'];
  return Promise.all(ids.map((id) => page.getByTestId(id).inputValue()));
}

/** Drops a file onto `selector` the way a browser drag & drop would. */
async function dropFile(page: Page, selector: string, name: string, bytes: Buffer) {
  const dataTransfer = await page.evaluateHandle(
    ({ name, b64 }) => {
      const bin = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
      const dt = new DataTransfer();
      dt.items.add(new File([bin], name));
      return dt;
    },
    { name, b64: bytes.toString('base64') },
  );
  await page.dispatchEvent(selector, 'dragenter', { dataTransfer });
  await page.dispatchEvent(selector, 'dragover', { dataTransfer });
  await page.dispatchEvent(selector, 'drop', { dataTransfer });
}

test.describe('IDE features on a multi-region document', () => {
  test('viewport picking, multi-region display, entities, repair, cross-format diff, drag & drop', async ({
    page,
  }) => {
    await page.goto('/');
    await page.getByTestId('open-file-input').setInputFiles(FIXTURE);
    await expect(page.getByTestId('doc-name')).toHaveText('MultiRegionFixture');

    // #28: both regions are drawn; visibility can be toggled per region.
    await expect(page.getByTestId('region-visibility-list')).toBeVisible();
    await expect(page.getByTestId('region-visibility-North')).toBeChecked();
    const visible = (id: string) =>
      page.evaluate(
        (r) =>
          (window as unknown as { __schematicViewport: { isRegionVisible(r: string): boolean } })
            .__schematicViewport.isRegionVisible(r),
        id,
      );
    expect(await visible('North')).toBe(true);
    expect(await visible('South')).toBe(true);
    await page.getByTestId('region-visibility-South').uncheck();
    expect(await visible('South')).toBe(false);
    await page.getByTestId('region-visibility-South').check();
    expect(await visible('South')).toBe(true);

    // #24: hovering highlights, clicking selects and inspects the block.
    const top = await project(page, [3.5, 1, 3.5]);
    await page.mouse.move(top.x, top.y);
    await expect(page.getByTestId('viewport-hover')).toHaveText('North · (3, 0, 3)');
    await clickBlockTop(page, [3, 0, 3]);
    expect(await selectionInputs(page)).toEqual(['3', '0', '3', '3', '0', '3']);
    // Shift+click extends the box from the anchor.
    await clickBlockTop(page, [0, 0, 3], ['Shift']);
    expect(await selectionInputs(page)).toEqual(['0', '0', '3', '3', '0', '3']);
    // The chest in front occludes the floor behind it: picking hits the chest.
    await clickBlockTop(page, [1, 1, 1]);
    expect(await selectionInputs(page)).toEqual(['1', '1', '1', '1', '1', '1']);
    // Clicking a block in the other region switches the active region.
    await clickBlockTop(page, [9, 0, 1]);
    await expect(page.getByTestId('region-selector')).toHaveValue('South');
    await page.getByTestId('tab-inspector').click();
    await expect(page.getByTestId('inspector-panel')).toContainText('minecraft:granite');

    // #31: entities and block entities are listed; entities can be moved/removed.
    await page.getByTestId('tab-entities').click();
    await expect(page.getByTestId('block-entity-list')).toContainText('minecraft:chest');
    await expect(page.getByTestId('block-entity-list')).toContainText('minecraft:furnace');
    await expect(page.getByTestId('entity-item-0')).toContainText('minecraft:pig');
    await page.getByTestId('entity-item-0').click();
    await expect(page.getByTestId('entity-detail')).toBeVisible();
    await page.getByTestId('entity-move-x+').click();
    await expect(page.getByTestId('preview-panel')).toBeVisible();
    await page.getByTestId('commit-btn').click();
    await expect(page.getByTestId('entity-item-0')).toContainText('(3.5, 1.0, 2.5)');
    // The moved entity stays selected.
    await expect(page.getByTestId('entity-detail')).toBeVisible();
    await page.getByTestId('entity-delete-btn').click();
    await page.getByTestId('commit-btn').click();
    await expect(page.getByTestId('entity-list')).not.toContainText('minecraft:pig');
    await page.getByTestId('undo-btn').click();
    await expect(page.getByTestId('entity-list')).toContainText('minecraft:pig');

    // #25: the orphaned furnace block entity is repaired automatically.
    await page.getByTestId('tab-problems').click();
    await expect(page.getByTestId('diagnostics-list')).toContainText('WARN_ORPHAN_BLOCK_ENTITY');
    await page.getByTestId('fix-all-btn').click();
    await expect(page.getByTestId('preview-panel')).toBeVisible();
    await page.getByTestId('commit-btn').click();
    await expect(page.getByTestId('problems-panel')).not.toContainText('WARN_ORPHAN_BLOCK_ENTITY');

    // #20/#22 + #30: exporting two regions to .schem warns about the merge;
    // the exported .schem can then be used as a diff target.
    await page.getByTestId('export-format-select').selectOption('sponge');
    await page.getByTestId('export-btn').click();
    await expect(page.getByTestId('export-loss-REGIONS_MERGED')).toContainText('North, South');
    const download = page.waitForEvent('download');
    await page.getByTestId('export-loss-confirm').click();
    const schemPath = path.resolve(__dirname, '../test-results/multi_region.schem');
    fs.mkdirSync(path.dirname(schemPath), { recursive: true });
    await (await download).saveAs(schemPath);

    await page.getByTestId('tab-diff').click();
    await expect(page.getByTestId('diff-file-input')).toHaveAttribute(
      'accept',
      '.litematic,.schem,.nbt',
    );
    await page.getByTestId('diff-file-input').setInputFiles(schemPath);
    await expect(page.getByTestId('diff-content')).toBeVisible();

    // #29: dropping an unsupported file reports it; dropping a schematic opens
    // it after confirming that unsaved changes will be discarded.
    await dropFile(page, '[data-testid="app-container"]', 'notes.txt', Buffer.from('hello'));
    await expect(page.getByTestId('error-banner')).toContainText('Unsupported file');
    page.once('dialog', (d) => d.accept());
    await dropFile(page, '[data-testid="app-container"]', 'multi_region.schem', fs.readFileSync(schemPath));
    await page.getByTestId('tab-regions').click();
    await expect(page.getByTestId('region-selector')).toHaveValue('Main');
    // Merged regions: 16 stone + chest + 9 granite.
    await expect(page.getByTestId('region-block-count')).toHaveText('26');
    await expect(page.getByTestId('drop-overlay')).toHaveCount(0);
  });
});
