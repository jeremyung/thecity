import { build } from 'esbuild';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(import.meta.dirname, '..');
if (!process.argv[2]) process.argv[2] = resolve(root, 'validation/building-chunks-before.json');
const temp = await mkdtemp(join(tmpdir(), 'thecity-chunks-bench-'));
try {
  const output = join(temp, 'benchmark.mjs');
  await build({
    entryPoints: [resolve(root, 'scripts/benchmark/benchmark-building-chunks.ts')],
    outfile: output,
    bundle: true,
    platform: 'node',
    format: 'esm',
    target: 'node20',
  });
  await import(pathToFileURL(output).href);
} finally {
  await rm(temp, { recursive: true, force: true });
}
