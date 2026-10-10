#!/usr/bin/env node
import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const textureRoot = path.join(root, 'public/art');
const manifestPath = path.join(root, 'assets-manifest.json');
const sideBudgets = { tile: 1024, atlas: 2048 };
const totalBudgetBytes = 32 * 1024 * 1024;
const supportedExtensions = new Set(['.png', '.webp', '.jpg', '.jpeg']);

function args(argv) {
  const result = { check: false, out: null };
  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    if (arg === '--check') result.check = true;
    else if (arg === '--out') {
      const value = argv[++index];
      if (!value) throw new Error('--out requires a file path');
      result.out = value;
    } else if (arg === '--help' || arg === '-h') {
      result.help = true;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return result;
}

function normalizedAssetPath(value) {
  if (typeof value !== 'string') return null;
  const normalized = value.replaceAll('\\', '/').replace(/^\.\//, '');
  return normalized.startsWith('public/') ? normalized : null;
}

function collectAssetPaths(value, into = new Set()) {
  if (!value || typeof value !== 'object') return into;
  if (Array.isArray(value)) {
    for (const entry of value) collectAssetPaths(entry, into);
  } else {
    for (const [key, entry] of Object.entries(value)) {
      if (['file', 'path', 'url', 'src'].includes(key)) {
        const assetPath = normalizedAssetPath(entry);
        if (assetPath) into.add(assetPath);
      }
      collectAssetPaths(entry, into);
    }
  }
  return into;
}

function pngDimensions(buffer) {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  if (buffer.length < 24 || !buffer.subarray(0, 8).equals(signature) || buffer.toString('ascii', 12, 16) !== 'IHDR') {
    throw new Error('invalid PNG signature or missing IHDR header');
  }
  return { width: buffer.readUInt32BE(16), height: buffer.readUInt32BE(20) };
}

function jpegDimensions(buffer) {
  if (buffer.length < 4 || buffer[0] !== 0xff || buffer[1] !== 0xd8) throw new Error('invalid JPEG SOI marker');
  const startOfFrame = new Set([0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf]);
  let offset = 2;
  while (offset < buffer.length) {
    while (offset < buffer.length && buffer[offset] !== 0xff) offset++;
    while (offset < buffer.length && buffer[offset] === 0xff) offset++;
    if (offset >= buffer.length) break;
    const marker = buffer[offset++];
    if (marker === 0xd9 || marker === 0xda) break;
    if (marker >= 0xd0 && marker <= 0xd7 || marker === 0x01) continue;
    if (offset + 2 > buffer.length) break;
    const length = buffer.readUInt16BE(offset);
    if (length < 2 || offset + length > buffer.length) throw new Error('truncated JPEG segment');
    if (startOfFrame.has(marker)) {
      if (length < 7) throw new Error('short JPEG frame header');
      return { height: buffer.readUInt16BE(offset + 3), width: buffer.readUInt16BE(offset + 5) };
    }
    offset += length;
  }
  throw new Error('JPEG has no supported start-of-frame header');
}

function webpDimensions(buffer) {
  if (buffer.length < 30 || buffer.toString('ascii', 0, 4) !== 'RIFF' || buffer.toString('ascii', 8, 12) !== 'WEBP') {
    throw new Error('invalid WebP RIFF header');
  }
  let offset = 12;
  while (offset + 8 <= buffer.length) {
    const kind = buffer.toString('ascii', offset, offset + 4);
    const size = buffer.readUInt32LE(offset + 4);
    const payload = offset + 8;
    if (payload + size > buffer.length) throw new Error(`truncated WebP ${kind} chunk`);
    if (kind === 'VP8X' && size >= 10) {
      return {
        width: 1 + buffer[payload + 4] + (buffer[payload + 5] << 8) + (buffer[payload + 6] << 16),
        height: 1 + buffer[payload + 7] + (buffer[payload + 8] << 8) + (buffer[payload + 9] << 16),
      };
    }
    if (kind === 'VP8 ' && size >= 10 && buffer[payload + 3] === 0x9d && buffer[payload + 4] === 0x01 && buffer[payload + 5] === 0x2a) {
      return {
        width: buffer.readUInt16LE(payload + 6) & 0x3fff,
        height: buffer.readUInt16LE(payload + 8) & 0x3fff,
      };
    }
    if (kind === 'VP8L' && size >= 5 && buffer[payload] === 0x2f) {
      const b1 = buffer[payload + 1], b2 = buffer[payload + 2], b3 = buffer[payload + 3], b4 = buffer[payload + 4];
      return {
        width: 1 + b1 + ((b2 & 0x3f) << 8),
        height: 1 + ((b2 >> 6) & 0x03) + (b3 << 2) + ((b4 & 0x0f) << 10),
      };
    }
    offset = payload + size + (size & 1);
  }
  throw new Error('WebP has no supported VP8, VP8L, or VP8X image header');
}

function dimensionsFor(buffer, ext) {
  if (ext === '.png') return pngDimensions(buffer);
  if (ext === '.jpg' || ext === '.jpeg') return jpegDimensions(buffer);
  if (ext === '.webp') return webpDimensions(buffer);
  return null;
}

function mipChainRgba8Bytes(width, height) {
  let w = width, h = height, bytes = 0;
  while (true) {
    bytes += w * h * 4;
    if (w === 1 && h === 1) break;
    w = Math.max(1, Math.floor(w / 2));
    h = Math.max(1, Math.floor(h / 2));
  }
  return bytes;
}

async function walk(directory) {
  const found = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) found.push(...await walk(absolute));
    else if (entry.isFile() && supportedExtensions.has(path.extname(entry.name).toLowerCase())) found.push(absolute);
  }
  return found;
}

async function companionMetadata(file) {
  const ext = path.extname(file);
  const candidate = `${file.slice(0, -ext.length)}.json`;
  try { return JSON.parse(await readFile(candidate, 'utf8')); }
  catch (error) {
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
}

export async function auditTextureBudget({ directory = textureRoot, manifestFile = manifestPath } = {}) {
  const manifest = JSON.parse(await readFile(manifestFile, 'utf8'));
  const runtimePaths = collectAssetPaths(manifest.runtime_media);
  const candidateByPath = new Map((manifest.development_material_candidates ?? [])
    .map(item => [normalizedAssetPath(item.file), item]).filter(([file]) => file));
  const files = (await walk(directory)).sort();
  const assets = [];
  let totalKnownDecodedBytes = 0;

  for (const file of files) {
    const relative = path.relative(root, file).split(path.sep).join('/');
    const ext = path.extname(file).toLowerCase();
    const stat = await readFile(file);
    const metadata = await companionMetadata(file);
    const role = metadata?.role === 'atlas' ? 'atlas' : 'tile';
    const candidate = candidateByPath.get(relative) ?? null;
    const asset = {
      file: relative,
      format: ext.slice(1).toUpperCase(),
      fileBytes: stat.length,
      runtimeListed: runtimePaths.has(relative),
      developmentCandidate: candidate ? { id: candidate.id ?? null, usage: candidate.usage ?? null } : null,
      metadataRole: metadata?.role ?? null,
      budgetClass: role,
      budgetMaxSide: sideBudgets[role],
    };
    try {
      const { width, height } = dimensionsFor(stat, ext);
      if (!Number.isSafeInteger(width) || !Number.isSafeInteger(height) || width <= 0 || height <= 0) throw new Error('image header reported invalid dimensions');
      const decodedMipBytes = mipChainRgba8Bytes(width, height);
      totalKnownDecodedBytes += decodedMipBytes;
      asset.width = width;
      asset.height = height;
      asset.maxSide = Math.max(width, height);
      asset.estimatedDecodedRgba8MipBytes = decodedMipBytes;
      asset.status = asset.maxSide > asset.budgetMaxSide ? 'side-budget-exceeded' : 'ok';
    } catch (error) {
      asset.status = 'unsupported-or-invalid';
      asset.dimensionError = error instanceof Error ? error.message : String(error);
    }
    assets.push(asset);
  }

  const unknown = assets.filter(asset => asset.status === 'unsupported-or-invalid');
  const sideViolations = assets.filter(asset => asset.status === 'side-budget-exceeded');
  const totalExceeded = totalKnownDecodedBytes > totalBudgetBytes;
  return {
    schemaVersion: 1,
    audit: 'texture file dimensions and estimated RGBA8 mip-chain storage; this is a lower-bound source-file inventory, not measured GPU memory',
    scannedDirectory: path.relative(root, directory).split(path.sep).join('/'),
    decodedEstimate: 'RGBA8, complete mip chain through 1x1, each mip dimension floored and clamped to 1',
    limitations: [
      'Each source image is counted once; per-style clones, simultaneous copies, canvas-generated textures, render targets, driver alignment and device-specific formats are not measured or inferred.',
      'RGBA8 mip estimates are a consistent planning proxy and may differ from actual GPU allocation or mip-generation policy.',
    ],
    proposedBudget: {
      tileMaxSide: sideBudgets.tile,
      atlasMaxSide: sideBudgets.atlas,
      totalDecodedMipBytes: totalBudgetBytes,
      totalDecodedMipMiB: totalBudgetBytes / (1024 * 1024),
    },
    summary: {
      assetCount: assets.length,
      runtimeListedCount: assets.filter(asset => asset.runtimeListed).length,
      developmentCandidateCount: assets.filter(asset => asset.developmentCandidate).length,
      unsupportedOrInvalidCount: unknown.length,
      sideBudgetViolationCount: sideViolations.length,
      knownEstimatedDecodedMipBytes: totalKnownDecodedBytes,
      knownEstimatedDecodedMipMiB: Number((totalKnownDecodedBytes / (1024 * 1024)).toFixed(4)),
      totalBudgetExceeded: totalExceeded,
      complete: unknown.length === 0,
      passesProposedBudget: unknown.length === 0 && sideViolations.length === 0 && !totalExceeded,
    },
    assets,
  };
}

async function main() {
  const options = args(process.argv.slice(2));
  if (options.help) {
    process.stdout.write('Usage: node scripts/audit-texture-budget.mjs [--check] [--out <file>]\n');
    return;
  }
  const report = await auditTextureBudget();
  const json = `${JSON.stringify(report, null, 2)}\n`;
  if (options.out) {
    const output = path.resolve(root, options.out);
    await mkdir(path.dirname(output), { recursive: true });
    await writeFile(output, json);
    process.stdout.write(`${path.relative(root, output)}\n`);
  } else {
    process.stdout.write(json);
  }
  if (options.check && !report.summary.passesProposedBudget) process.exitCode = 1;
}

main().catch(error => {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
});
