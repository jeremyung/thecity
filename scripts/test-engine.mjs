import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
const env = { ...process.env };
// Type-export tests write to a scratch directory; `npm run types` owns web/src/generated.
env.TS_RS_EXPORT_DIR = join(tmpdir(), 'thecity-ts-rs');
if (existsSync('.tools/cargo/bin')) {
  env.CARGO_HOME = resolve('.tools/cargo');
  env.RUSTUP_HOME = resolve('.tools/rustup');
  env.PATH = `${resolve('.tools/cargo/bin')}:${env.PATH}`;
}
// Legacy-planner tests stay in the suite until master-plan acceptance passes.
const result = spawnSync(
  'cargo',
  [
    'test',
    '--manifest-path',
    'engine/Cargo.toml',
    '--locked',
    '--workspace',
    '--features',
    'legacy',
    ...process.argv.slice(2),
  ],
  { env, stdio: 'inherit' },
);
if (result.error) throw result.error;
process.exit(result.status ?? 1);
