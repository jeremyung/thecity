import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolve, relative, isAbsolute } from 'node:path';
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
// Experiments stay opt-in so comparison builds cannot replace production WASM.
const modes = [
  ['frontage-network', 'frontage-network-candidate'],
  ['profiling', 'profiling'],
  ['street-placement', 'street-placement-candidate'],
  ['compact-expressway', 'compact-expressway-candidate'],
  ['single-expressway', 'single-expressway-candidate'],
  ['frontage-fit', 'frontage-fit-candidate'],
  ['flag-passage', 'flag-passage-candidate'],
  ['block-proportion', 'block-proportion-candidate'],
  ['parent-passage', 'parent-passage-candidate'],
  // Retired planners for the legacy WASM tests; never the application build.
  ['legacy', 'legacy'],
];
const selected = modes.filter(([flag]) => process.argv.includes(`--${flag}`));
if (selected.length > 1) throw new Error('Choose one isolated build mode.');
const mode = selected[0];
const features = mode ? ['--features', mode[1]] : [];
const defaultOutput = mode ? `.tools/wasm-${mode[0]}` : 'public/wasm';
// Version isolated experiments without overwriting previous comparison binaries.
const outputIndex = process.argv.indexOf('--out-dir');
const output = outputIndex < 0 ? defaultOutput : process.argv[outputIndex + 1];
if (outputIndex >= 0) {
  if (!output || !mode)
    throw new Error('--out-dir requires an isolated build mode and a directory.');
  const withinTools = relative(resolve('.tools'), resolve(output));
  if (!withinTools || withinTools.startsWith('..') || isAbsolute(withinTools))
    throw new Error('Isolated output must be beneath .tools.');
}
run('cargo', [
  'build',
  '--manifest-path',
  'engine/Cargo.toml',
  '-p',
  'city-wasm',
  '--target',
  'wasm32-unknown-unknown',
  '--release',
  '--locked',
  ...features,
]);
run('wasm-bindgen', [
  'engine/target/wasm32-unknown-unknown/release/city_wasm.wasm',
  '--target',
  'web',
  '--out-dir',
  output,
  '--out-name',
  'city_engine',
]);
