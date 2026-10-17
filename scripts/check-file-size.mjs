// Keeps source files under 1,500 lines. Files already over the limit are listed
// in scripts/file-size-allowlist.json with their current size; they may shrink
// but not grow. Anything else over the limit fails.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const LIMIT = 1500;
const allow = JSON.parse(readFileSync('scripts/file-size-allowlist.json', 'utf8'));
const files = execFileSync('git', ['ls-files', 'web/src', 'engine/src', 'engine/crates'], {
  encoding: 'utf8',
})
  .split('\n')
  .filter((file) => /\.(ts|tsx|rs)$/.test(file) && !file.startsWith('web/src/render/generated/'));
let failed = false;
for (const file of files) {
  const lines = readFileSync(file, 'utf8').split('\n').length;
  if (lines <= LIMIT) continue;
  const allowed = allow[file];
  if (allowed === undefined) {
    console.error(`${file}: ${lines} lines (limit ${LIMIT}); split it before adding more.`);
    failed = true;
  } else if (lines > allowed) {
    console.error(
      `${file}: grew from ${allowed} to ${lines} lines; oversized files may only shrink.`,
    );
    failed = true;
  } else if (process.argv.includes('--verbose')) {
    console.warn(`${file}: ${lines} lines (allowed while it shrinks)`);
  }
}
process.exit(failed ? 1 : 0);
