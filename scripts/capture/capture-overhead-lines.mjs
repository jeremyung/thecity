// Capture utility poles and street wiring beside the curved rail corridor in the real app (software WebGL).
//
// Usage: node scripts/capture/capture-overhead-lines.mjs <save.json> <out-dir> [preview hour] [anime|miniature] [strip]
// Requires `npx vite --host 127.0.0.1 --port 5173` running. As in
// capture-parks.mjs, geometry queues are drained with the post pipeline
// detached; these images are visual evidence, not frame-rate evidence.
// `strip` also records a short dolly along a wired street (frames 0.4 m apart)
// for the wire shimmer review.
// CHROMIUM_PATH selects a preinstalled browser when Playwright's pinned build is absent.
import { chromium } from '@playwright/test';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const [savePath, outDir, hourArg, styleArg, stripArg] = process.argv.slice(2);
if (!savePath || !outDir) {
  console.error(
    'Usage: node scripts/capture/capture-overhead-lines.mjs <save.json> <out-dir> [preview hour] [anime|miniature] [strip]',
  );
  process.exit(1);
}
const hour = Number(hourArg ?? 15);
const style = styleArg === 'miniature' ? 'miniature' : 'anime';
const saved = await readFile(savePath, 'utf8');
const expected = JSON.parse(saved);
await mkdir(outDir, { recursive: true });

const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.CHROMIUM_PATH || undefined,
  args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
try {
  const page = await browser.newPage({
    viewport: { width: 1400, height: 900 },
    deviceScaleFactor: 1,
  });
  page.on('pageerror', (error) => console.error(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') console.error(message.text());
  });
  await page.goto('http://127.0.0.1:5173/?webgl=1', { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(
    () => document.querySelector('canvas')?.dataset.graphicsPhase === 'first-frame-submitted',
    null,
    { timeout: 120000 },
  );
  await page.evaluate(
    ([s, save]) => {
      const c = window.__city;
      c.renderer.renderer.setAnimationLoop(null);
      c.client.controls({ paused: true });
      c.client.init(s, save);
    },
    [expected.seed, saved],
  );
  await page.waitForFunction(
    (count) => window.__city?.world?.buildings.length === count,
    expected.buildings.length,
    { timeout: 60000 },
  );
  if (style !== 'anime') await page.evaluate((s) => window.__city.renderer.setStyle(s), style);

  const scene = await page.evaluate((previewHour) => {
    const r = window.__city.renderer;
    r.renderer.setAnimationLoop(null);
    r.director.setMode('explore');
    r.setPreview(previewHour);
    r.setFocus(false);
    r.setQuality('balanced');
    const pipeline = r.pipeline;
    r.pipeline = null;
    let steps = 0;
    while ((r.objectQueue.size || r.chunks.pendingCount) && steps < 40000) {
      r.animate();
      steps++;
    }
    r.pipeline = pipeline;
    // Daylight eases toward the preview hour with frame time; step it there.
    for (let i = 0; i < 40; i++) {
      r.lastFrame = performance.now() - 100;
      r.animate();
    }
    const w = r.worldState;
    const station = w.nodes.find((n) => n.kind === 'station');
    const layer = r.parks.getObjectByName('overhead-lines');
    const diagnostics = {};
    layer?.traverse((o) => {
      if (o.isInstancedMesh) diagnostics[o.name] = o.count;
      else if (o.isMesh)
        diagnostics.wireTriangles = (diagnostics.wireTriangles ?? 0) + o.geometry.index.count / 3;
    });
    // Read placements back from the instanced meshes so views frame real poles and masts.
    const read = (name) => {
      const mesh = layer?.getObjectByName(name);
      if (!mesh) return [];
      const m = mesh.matrix.clone(),
        out = [];
      for (let i = 0; i < mesh.count; i++) {
        mesh.getMatrixAt(i, m);
        const e = m.elements,
          s = Math.hypot(e[0], e[2]) || 1;
        out.push({ x: e[12], z: e[14], ix: e[0] / s, iz: e[2] / s, height: e[5] });
      }
      return out;
    };
    const far = (p) => !station || Math.hypot(p.x - station.x, p.z - station.z) > 260;
    const poles = read('utility-pole-shafts')
      .filter(far)
      .sort((a, b) => Math.hypot(a.x, a.z) - Math.hypot(b.x, b.z));
    // The street pole nearest the curved rail's midpoint frames where wiring meets the viaduct.
    const samples = (w.rail_alignments ?? []).flatMap((a) => a.samples);
    const bend = samples.length ? samples[Math.floor(samples.length / 2)] : null;
    const railPole = bend
      ? read('utility-pole-shafts').sort(
          (a, b) =>
            Math.hypot(a.x - bend[1], a.z - bend[2]) - Math.hypot(b.x - bend[1], b.z - bend[2]),
        )[0]
      : null;
    r.director.update = () => {};
    return {
      diagnostics,
      tick: w.tick,
      steps,
      station,
      pole: poles[Math.floor(poles.length / 3)] ?? null,
      rail: bend && { x: bend[1], z: bend[2], ux: bend[3], uz: bend[4] },
      railPole: railPole ?? null,
      platform: (w.rail_platforms ?? [])[0] ?? null,
    };
  }, hour);

  const views = [];
  if (scene.pole) {
    // The crossarm points across the street; the street runs perpendicular to it.
    // Eye height in the middle of the street (2.9 m in from the pole), set back from it.
    const p = scene.pole,
      ux = -p.iz,
      uz = p.ix,
      cx = p.x + p.ix * 2.9,
      cz = p.z + p.iz * 2.9;
    views.push({
      name: 'local-street',
      target: [cx + ux * 40, 6, cz + uz * 40],
      position: [cx - ux * 14, 1.7, cz - uz * 14],
    });
    views.push({
      name: 'local-street-reverse',
      target: [cx - ux * 40, 6, cz - uz * 40],
      position: [cx + ux * 14, 1.7, cz + uz * 14],
    });
    views.push({
      name: 'local-street-above',
      target: [p.x, 5, p.z],
      position: [p.x + p.ix * 26 - ux * 22, 24, p.z + p.iz * 26 - uz * 22],
    });
  }
  if (scene.rail) {
    // Above the curved track, looking along it: the corridor's own catenary and the street wiring beside it.
    const t = scene.rail;
    views.push({
      name: 'rail-curve',
      target: [t.x + t.ux * 30, 9, t.z + t.uz * 30],
      position: [t.x - t.ux * 24, 18, t.z - t.uz * 24],
    });
  }
  if (scene.railPole) {
    const p = scene.railPole;
    views.push({
      name: 'rail-street',
      target: [p.x, 6, p.z],
      position: [p.x + p.ix * 14 + 10, 14, p.z + p.iz * 14 + 10],
    });
  }
  if (scene.platform) {
    // Above the track behind the terminal, looking down the platform (+Z = (ux, uz) in the engine frame).
    const p = scene.platform;
    views.push({
      name: 'station-approach',
      target: [p.x + p.ux * 40, 9, p.z + p.uz * 40],
      position: [p.x - p.ux * 28, 20, p.z - p.uz * 28],
    });
  }
  views.push({ name: 'overview', target: [0, 0, 0], position: [380, 420, -470] });

  const shoot = async (view) => {
    await page.evaluate((v) => {
      const r = window.__city.renderer;
      r.director.target.set(...v.target);
      r.camera.position.set(...v.position);
      r.camera.lookAt(r.director.target);
      r.animate();
    }, view);
    const file = join(outDir, `${view.name}.png`);
    await page.screenshot({ path: file, timeout: 300000 });
    return { ...view, file };
  };
  const captured = [];
  for (const view of views) captured.push(await shoot(view));
  if (stripArg === 'strip' && scene.pole) {
    const p = scene.pole,
      ux = -p.iz,
      uz = p.ix,
      cx = p.x + p.ix * 1.5,
      cz = p.z + p.iz * 1.5;
    for (let i = 0; i < 4; i++) {
      const d = i * 0.4;
      captured.push(
        await shoot({
          name: `strip-${i}`,
          target: [cx + ux * (90 + d), 8, cz + uz * (90 + d)],
          position: [cx - ux * (10 - d), 4, cz - uz * (10 - d)],
        }),
      );
    }
  }
  await writeFile(
    join(outDir, 'capture.json'),
    `${JSON.stringify({ source: savePath, backend: 'software WebGL (swiftshader)', performanceEvidence: false, previewHour: hour, style, depthOfField: false, scene, views: captured }, null, 2)}\n`,
  );
  console.log(
    JSON.stringify({
      diagnostics: scene.diagnostics,
      tick: scene.tick,
      steps: scene.steps,
      views: captured.map((v) => v.name),
    }),
  );
} finally {
  await browser.close();
}
