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
const frontageNetwork = process.argv.includes('--frontage-network');
const profiling = process.argv.includes('--profiling');
const streetPlacement = process.argv.includes('--street-placement');
const compactExpressway = process.argv.includes('--compact-expressway');
const singleExpressway = process.argv.includes('--single-expressway');
const frontageFit = process.argv.includes('--frontage-fit');
const flagPassage = process.argv.includes('--flag-passage');
const blockProportion = process.argv.includes('--block-proportion');
const parentPassage = process.argv.includes('--parent-passage');
if ([frontageNetwork, profiling, streetPlacement, compactExpressway, singleExpressway, frontageFit, flagPassage, blockProportion, parentPassage].filter(Boolean).length > 1) throw new Error('Choose one isolated build mode.');
const features = frontageNetwork ? ['--features','frontage-network-candidate'] : streetPlacement ? ['--features','street-placement-candidate'] : compactExpressway ? ['--features','compact-expressway-candidate'] : profiling ? ['--features', 'profiling'] : singleExpressway ? ['--features', 'single-expressway-candidate'] : frontageFit ? ['--features', 'frontage-fit-candidate'] : flagPassage ? ['--features','flag-passage-candidate'] : blockProportion ? ['--features','block-proportion-candidate'] : parentPassage ? ['--features','parent-passage-candidate'] : [];
const defaultOutput = frontageNetwork ? '.tools/wasm-frontage-network' : streetPlacement ? '.tools/wasm-street-placement' : compactExpressway ? '.tools/wasm-compact-expressway' : profiling ? '.tools/wasm-profiling' : singleExpressway ? '.tools/wasm-single-expressway' : frontageFit ? '.tools/wasm-frontage-fit' : flagPassage ? '.tools/wasm-flag-passage' : blockProportion ? '.tools/wasm-block-proportion' : parentPassage ? '.tools/wasm-parent-passage' : 'public/wasm';
// Version isolated experiments without overwriting previous comparison binaries.
const outputIndex = process.argv.indexOf('--out-dir');
const output = outputIndex < 0 ? defaultOutput : process.argv[outputIndex + 1];
if (outputIndex >= 0) {
  if (!output || !(frontageNetwork || profiling || streetPlacement || compactExpressway || singleExpressway || frontageFit || flagPassage || blockProportion || parentPassage)) throw new Error('--out-dir requires an isolated build mode and a directory.');
  const withinTools = relative(resolve('.tools'), resolve(output));
  if (!withinTools || withinTools.startsWith('..') || isAbsolute(withinTools)) throw new Error('Isolated output must be beneath .tools.');
}
run('cargo', ['build', '--manifest-path', 'engine/Cargo.toml', '--target', 'wasm32-unknown-unknown', '--release', '--locked', ...features]);
run('wasm-bindgen', ['engine/target/wasm32-unknown-unknown/release/tokyo_city_engine.wasm', '--target', 'web', '--out-dir', output, '--out-name', 'city_engine']);
