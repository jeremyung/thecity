import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
const local = resolve('.tools/cargo/bin');
const env = { ...process.env };
if (existsSync(local)) {
  env.CARGO_HOME = resolve('.tools/cargo');
  env.RUSTUP_HOME = resolve('.tools/rustup');
  env.PATH = `${local}:${env.PATH}`;
}
function run(command, args) {
  const result = spawnSync(command, args, { env, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
const profiling = process.argv.includes('--profiling');
run('cargo', ['build', '--manifest-path', 'engine/Cargo.toml', '--target', 'wasm32-unknown-unknown', '--release', '--locked', ...(profiling ? ['--features', 'profiling'] : [])]);
run('wasm-bindgen', ['engine/target/wasm32-unknown-unknown/release/tokyo_city_engine.wasm', '--target', 'web', '--out-dir', profiling ? '.tools/wasm-profiling' : 'public/wasm', '--out-name', 'city_engine']);
