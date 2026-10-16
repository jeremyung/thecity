import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
const env = { ...process.env };
if (existsSync('.tools/cargo/bin')) {
  env.CARGO_HOME = resolve('.tools/cargo');
  env.RUSTUP_HOME = resolve('.tools/rustup');
  env.PATH = `${resolve('.tools/cargo/bin')}:${env.PATH}`;
}
const result = spawnSync(
  'cargo',
  ['test', '--manifest-path', 'engine/Cargo.toml', '--locked', ...process.argv.slice(2)],
  { env, stdio: 'inherit' },
);
if (result.error) throw result.error;
process.exit(result.status ?? 1);
