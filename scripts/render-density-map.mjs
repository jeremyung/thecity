import { readFile, writeFile } from 'node:fs/promises';
import { resolve, dirname, relative } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const MAP_SIZE = 800;
const MAP_LEFT = 92;
const MAP_TOP = 78;
const WIDTH = 1000;
const HEIGHT = 1080;
const CONSTRUCTION_STAGES = new Set(['foundation', 'survey', 'clearing', 'structure', 'surface', 'finishing']);
const HOUSING_KINDS = new Set(['home', 'apartment']);
const COMMERCIAL_KINDS = new Set(['shop', 'cafe', 'office']);
const STATUS_NAMES = ['resident_occupied', 'empty_housing', 'active_commercial', 'vacant_commercial', 'commercial_unknown', 'renovation', 'construction', 'unknown'];

function buildingStatus(building) {
  if (building.stage === 'renovation') return 'renovation';
  if (CONSTRUCTION_STAGES.has(building.stage)) return 'construction';
  if (building.stage !== 'open') return 'unknown';
  if (COMMERCIAL_KINDS.has(building.kind)) {
    if (building.business_active === true) return 'active_commercial';
    if (building.business_active === false) return 'vacant_commercial';
    return 'commercial_unknown';
  }
  if (!HOUSING_KINDS.has(building.kind) || !Number.isFinite(building.occupancy) || building.occupancy < 0) return 'unknown';
  return building.occupancy === 0 ? 'empty_housing' : 'resident_occupied';
}

function escapeXml(value) {
  return String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;').replaceAll("'", '&apos;');
}

function mapPoint(point, bounds) {
  return {
    x: MAP_LEFT + (point.x - bounds.min_x) / (bounds.max_x - bounds.min_x) * MAP_SIZE,
    y: MAP_TOP + (bounds.max_z - point.z) / (bounds.max_z - bounds.min_z) * MAP_SIZE,
  };
}

function riverPolygon(points, halfWidth, bounds) {
  if (!Array.isArray(points) || points.length < 2 || !Number.isFinite(halfWidth) || halfWidth <= 0) return '';
  const offsets = points.map((point, index) => {
    const before = points[Math.max(0, index - 1)];
    const after = points[Math.min(points.length - 1, index + 1)];
    const dx = after.x - before.x, dz = after.z - before.z;
    const length = Math.hypot(dx, dz) || 1;
    const normal = { x: dz / length, z: -dx / length };
    return {
      a: mapPoint({ x: point.x + normal.x * halfWidth, z: point.z + normal.z * halfWidth }, bounds),
      b: mapPoint({ x: point.x - normal.x * halfWidth, z: point.z - normal.z * halfWidth }, bounds),
    };
  });
  const pathPoints = [...offsets.map((item) => item.a), ...offsets.slice().reverse().map((item) => item.b)];
  return pathPoints.map((point, index) => `${index === 0 ? 'M' : 'L'}${point.x.toFixed(2)},${point.y.toFixed(2)}`).join(' ') + ' Z';
}

function floorColor(floors) {
  if (floors <= 2) return '#d9cfb0';
  if (floors <= 5) return '#aeb99e';
  if (floors <= 9) return '#789796';
  return '#a9795b';
}

function roadWidth(road) {
  if (road.kind === 'rail') return 2.5;
  if (road.kind === 'arterial') return 4.2;
  if (road.kind === 'bridge') return 3.5;
  return 2.1;
}

export function renderDensityMap(snapshot, { source = 'validation/mature-world.json' } = {}) {
  const bounds = snapshot?.bounds;
  if (!bounds || ![bounds.min_x, bounds.max_x, bounds.min_z, bounds.max_z].every(Number.isFinite)
      || bounds.max_x <= bounds.min_x || bounds.max_z <= bounds.min_z) {
    throw new TypeError('World snapshot requires finite, increasing bounds');
  }
  const scaleX = MAP_SIZE / (bounds.max_x - bounds.min_x);
  const scaleZ = MAP_SIZE / (bounds.max_z - bounds.min_z);
  const nodes = new Map((snapshot.nodes ?? []).map((node) => [node.id, node]));
  const buildingList = Array.isArray(snapshot.buildings) ? snapshot.buildings : [];
  const roads = Array.isArray(snapshot.roads) ? snapshot.roads : [];
  const validBuildings = buildingList.filter((b) => [b.x, b.z, b.width, b.depth, b.floors].every(Number.isFinite) && b.width > 0 && b.depth > 0 && b.floors > 0);
  const roadsWithEndpoints = roads.flatMap((road) => {
    const from = nodes.get(road.from), to = nodes.get(road.to);
    return from && to ? [{ road, from: mapPoint(from, bounds), to: mapPoint(to, bounds) }] : [];
  });
  const stations = (snapshot.nodes ?? []).filter((node) => node.kind === 'station');
  const statusCounts = Object.fromEntries(STATUS_NAMES.map((status) => [status, 0]));
  for (const building of validBuildings) statusCounts[buildingStatus(building)]++;
  const floorBuckets = [
    { name: '1–2 floors', color: floorColor(2) },
    { name: '3–5 floors', color: floorColor(4) },
    { name: '6–9 floors', color: floorColor(7) },
    { name: '10+ floors', color: floorColor(10) },
  ];
  const river = riverPolygon(snapshot.terrain?.river, snapshot.terrain?.river_half_width, bounds);
  const buildingShapes = validBuildings.map((building) => {
    const topLeft = mapPoint({ x: building.x - building.width / 2, z: building.z + building.depth / 2 }, bounds);
    const width = building.width * scaleX, height = building.depth * scaleZ;
    const status = buildingStatus(building);
    return `<rect class="building ${status}" x="${topLeft.x.toFixed(2)}" y="${topLeft.y.toFixed(2)}" width="${width.toFixed(2)}" height="${height.toFixed(2)}" fill="${floorColor(building.floors)}" data-id="${escapeXml(building.id)}" data-kind="${escapeXml(building.kind)}" data-floors="${escapeXml(building.floors)}" data-stage="${escapeXml(building.stage)}" data-status="${status}" data-business-active="${building.business_active === true ? 'true' : building.business_active === false ? 'false' : 'unknown'}"><title>${escapeXml(building.name)} · ${building.floors} floors · ${status} (${building.stage}) · nominal ${building.width} × ${building.depth} m envelope</title></rect>`;
  }).join('\n');
  const roadShapes = roadsWithEndpoints.map(({ road, from, to }) => {
    const isRail = road.kind === 'rail';
    const construction = road.stage !== 'open';
    const klass = `road${isRail ? ' rail' : ''}${construction ? ' construction' : ''}`;
    return `<path class="${klass}" d="M${from.x.toFixed(2)},${from.y.toFixed(2)} L${to.x.toFixed(2)},${to.y.toFixed(2)}" stroke-width="${roadWidth(road)}" data-id="${escapeXml(road.id)}" data-kind="${escapeXml(road.kind)}" data-stage="${escapeXml(road.stage)}"><title>${escapeXml(road.kind)} · ${road.stage} · ${Math.round(road.length)} m</title></path>`;
  }).join('\n');
  const stationShapes = stations.map((station) => {
    const point = mapPoint(station, bounds);
    return `<g class="station" transform="translate(${point.x.toFixed(2)} ${point.y.toFixed(2)})" data-id="${escapeXml(station.id)}"><circle r="8"/><circle class="station-core" r="3"/><text x="11" y="-10">Station ${escapeXml(station.id)}</text></g>`;
  }).join('\n');
  const minX = mapPoint({ x: bounds.min_x, z: bounds.max_z }, bounds).x;
  const maxX = mapPoint({ x: bounds.max_x, z: bounds.max_z }, bounds).x;
  const minY = MAP_TOP;
  const maxY = MAP_TOP + MAP_SIZE;
  const scaleLength = 100 * scaleX;
  const dateLine = `Seed ${escapeXml(snapshot.seed)} · tick ${escapeXml(snapshot.tick)} · ${escapeXml(snapshot.stage)} · ${escapeXml(source)}`;

  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${WIDTH}" height="${HEIGHT}" viewBox="0 0 ${WIDTH} ${HEIGHT}" role="img" aria-labelledby="title desc">
  <title id="title">Mature city nominal parcel-envelope map</title>
  <desc id="desc">Overhead map showing every valid nominal parcel envelope, actual road links, the river corridor, and station nodes. Envelope colors indicate floors; outlines distinguish resident use, commercial activity, and building stage. Legacy commercial saves without business activity data are marked unknown.</desc>
  <defs>
    <clipPath id="map-clip"><rect x="${minX}" y="${minY}" width="${maxX - minX}" height="${maxY - minY}"/></clipPath>
    <pattern id="rail-ties" width="8" height="8" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><path d="M0 0V8" stroke="#596568" stroke-width="1.2"/></pattern>
  </defs>
  <style>
    text{font-family:ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif;fill:#35464a}
    .title{font-size:22px;font-weight:650;letter-spacing:.02em}.meta{font-size:12px;fill:#647375}
    .land{fill:#e8e8d8}.frame{fill:none;stroke:#86928a;stroke-width:1}
    .river{fill:#a9c9c7;stroke:#7faaa9;stroke-width:1.5;opacity:.86}
    .road{fill:none;stroke:#727c78;stroke-linecap:round;opacity:.78}
    .road.rail{stroke:#48595b;stroke-dasharray:2.2 4;opacity:.95}
    .road.construction{stroke:#bd8258;stroke-dasharray:5 4;opacity:.95}
    .building{stroke-width:.95;vector-effect:non-scaling-stroke}
    .building.resident_occupied{stroke:#35484c}
    .building.empty_housing{stroke:#687976;stroke-dasharray:1.5 1.5}
    .building.active_commercial{stroke:#35725e;stroke-width:1.35}
    .building.vacant_commercial{stroke:#a87345;stroke-width:1.35;stroke-dasharray:3 2}
    .building.commercial_unknown{stroke:#7d6b78;stroke-width:1.35;stroke-dasharray:1 2}
    .building.renovation{stroke:#796488;stroke-width:1.5;stroke-dasharray:4 2}
    .building.construction{stroke:#b76840;stroke-width:1.5;stroke-dasharray:3 2;fill-opacity:.62}
    .building.unknown{stroke:#a34242;stroke-width:1.5;stroke-dasharray:1 2;fill-opacity:.6}
    .station circle{fill:#f4e7bf;stroke:#2d615f;stroke-width:2.2}.station .station-core{fill:#2d615f;stroke:none}
    .station text{font-size:11px;font-weight:650;paint-order:stroke;stroke:#e8e8d8;stroke-width:4px;stroke-linejoin:round}
    .legend-title{font-size:12px;font-weight:650}.legend{font-size:11px}.small{font-size:10px;fill:#6a7777}
    .scale{stroke:#35464a;stroke-width:2}.axis{stroke:#87918a;stroke-width:1}
  </style>
  <rect width="100%" height="100%" fill="#f3f0e6"/>
  <text class="title" x="${MAP_LEFT}" y="32">Mature city · nominal parcel envelopes</text>
  <text class="meta" x="${MAP_LEFT}" y="52">${dateLine}</text>
  <g clip-path="url(#map-clip)">
    <rect class="land" x="${minX}" y="${minY}" width="${maxX - minX}" height="${maxY - minY}"/>
    ${river ? `<path class="river" d="${river}"/>` : ''}
    ${roadShapes}
    ${buildingShapes}
    ${stationShapes}
  </g>
  <rect class="frame" x="${minX}" y="${minY}" width="${maxX - minX}" height="${maxY - minY}"/>
  <text class="small" x="${MAP_LEFT}" y="${maxY + 17}">N ↑</text>
  <line class="axis" x1="${MAP_LEFT + 27}" y1="${maxY + 13}" x2="${MAP_LEFT + 27}" y2="${maxY - 1}"/>
  <text class="legend-title" x="${MAP_LEFT}" y="${maxY + 44}">Nominal envelope · floor count</text>
  ${floorBuckets.map((bucket, i) => `<g transform="translate(${MAP_LEFT + i * 118} ${maxY + 58})"><rect width="13" height="13" fill="${bucket.color}" stroke="#35484c" stroke-width=".7"/><text class="legend" x="19" y="11">${bucket.name}</text></g>`).join('')}
  <text class="legend-title" x="${MAP_LEFT}" y="${maxY + 79}">Outline · residents, commercial activity, and building stage</text>
  <g transform="translate(${MAP_LEFT} ${maxY + 88})"><rect width="13" height="13" fill="#d9cfb0" stroke="#35484c" stroke-width="1"/><text class="legend" x="18" y="11">Resident occupied</text></g>
  <g transform="translate(${MAP_LEFT + 150} ${maxY + 88})"><rect width="13" height="13" fill="#d9cfb0" stroke="#687976" stroke-width="1" stroke-dasharray="1.5 1.5"/><text class="legend" x="18" y="11">Empty housing</text></g>
  <g transform="translate(${MAP_LEFT + 290} ${maxY + 88})"><rect width="13" height="13" fill="#d9cfb0" stroke="#35725e" stroke-width="1.35"/><text class="legend" x="18" y="11">Active commercial</text></g>
  <g transform="translate(${MAP_LEFT + 460} ${maxY + 88})"><rect width="13" height="13" fill="#d9cfb0" stroke="#a87345" stroke-width="1.35" stroke-dasharray="3 2"/><text class="legend" x="18" y="11">Vacant commercial</text></g>
  <g transform="translate(${MAP_LEFT + 640} ${maxY + 88})"><rect width="13" height="13" fill="#d9cfb0" stroke="#7d6b78" stroke-width="1.35" stroke-dasharray="1 2"/><text class="legend" x="18" y="11">Commercial unknown</text></g>
  <g transform="translate(${MAP_LEFT} ${maxY + 109})"><rect width="13" height="13" fill="#d9cfb0" stroke="#796488" stroke-width="1.4" stroke-dasharray="4 2"/><text class="legend" x="18" y="11">Renovation</text></g>
  <g transform="translate(${MAP_LEFT + 135} ${maxY + 109})"><rect width="13" height="13" fill="#d9cfb0" stroke="#b76840" stroke-width="1.4" stroke-dasharray="3 2"/><text class="legend" x="18" y="11">Construction</text></g>
  <g transform="translate(${MAP_LEFT + 270} ${maxY + 109})"><rect width="13" height="13" fill="#d9cfb0" stroke="#a34242" stroke-width="1.4" stroke-dasharray="1 2"/><text class="legend" x="18" y="11">Unknown stage</text></g>
  <line class="scale" x1="${MAP_LEFT + 620}" y1="${maxY + 6}" x2="${MAP_LEFT + 620 + scaleLength}" y2="${maxY + 6}"/><line class="scale" x1="${MAP_LEFT + 620}" y1="${maxY + 2}" x2="${MAP_LEFT + 620}" y2="${maxY + 10}"/><line class="scale" x1="${MAP_LEFT + 620 + scaleLength}" y1="${maxY + 2}" x2="${MAP_LEFT + 620 + scaleLength}" y2="${maxY + 10}"/><text class="legend" x="${MAP_LEFT + 620 + scaleLength + 8}" y="${maxY + 10}">100 m</text>
  <text class="small" x="${MAP_LEFT}" y="${maxY + 141}">${validBuildings.length} envelopes (${statusCounts.resident_occupied} resident occupied · ${statusCounts.empty_housing} empty housing · ${statusCounts.active_commercial} active commercial · ${statusCounts.vacant_commercial} vacant commercial · ${statusCounts.commercial_unknown} commercial unknown · ${statusCounts.renovation} renovation · ${statusCounts.construction} construction · ${statusCounts.unknown} unknown stage) · ${roadsWithEndpoints.length}/${roads.length} roads · ${stations.length} station node${stations.length === 1 ? '' : 's'}</text>
  <text class="small" x="${MAP_LEFT}" y="${maxY + 157}">business_active classifies commercial use; missing data in legacy saves is unknown. Envelopes use parcel width × depth; renderer setbacks reduce actual mass.</text>
</svg>
`;
}

async function runCli(argv) {
  let inputPath = resolve(root, 'validation/mature-world.json');
  let outputPath = resolve(root, 'validation/density-map.svg');
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--output') {
      if (!argv[i + 1]) throw new Error('--output requires a path');
      outputPath = resolve(argv[++i]);
    } else if (argv[i] === '--help' || argv[i] === '-h') {
      process.stdout.write('Usage: node scripts/render-density-map.mjs [save.json] [--output map.svg]\n');
      return;
    } else if (argv[i].startsWith('-')) {
      throw new Error(`Unknown option: ${argv[i]}`);
    } else {
      inputPath = resolve(argv[i]);
    }
  }

  const saveData = await readFile(inputPath, 'utf8');
  const wasmBytes = await readFile(resolve(root, 'public/wasm/city_engine_bg.wasm'));
  const wasmModule = await import(pathToFileURL(resolve(root, 'public/wasm/city_engine.js')).href);
  await wasmModule.default({ module_or_path: wasmBytes });
  const engine = wasmModule.CityEngine.restore(saveData);
  try {
    const snapshot = JSON.parse(engine.snapshot());
    const rendered = renderDensityMap(snapshot, { source: relative(root, inputPath) });
    await writeFile(outputPath, rendered);
    const buildingCount = (rendered.match(/class="building (?:resident_occupied|empty_housing|active_commercial|vacant_commercial|commercial_unknown|renovation|construction|unknown)"/g) ?? []).length;
    const roadCount = (rendered.match(/class="road(?: rail)?(?: construction)?"/g) ?? []).length;
    const expectedRoadCount = (snapshot.roads ?? []).filter((road) => (snapshot.nodes ?? []).some((node) => node.id === road.from) && (snapshot.nodes ?? []).some((node) => node.id === road.to)).length;
    if (buildingCount !== (snapshot.buildings ?? []).filter((b) => [b.x, b.z, b.width, b.depth, b.floors].every(Number.isFinite) && b.width > 0 && b.depth > 0 && b.floors > 0).length
        || roadCount !== expectedRoadCount) {
      throw new Error(`SVG count check failed: buildings ${buildingCount}, roads ${roadCount}`);
    }
    process.stdout.write(JSON.stringify({
      output: relative(root, outputPath), seed: snapshot.seed, tick: snapshot.tick,
      buildings: buildingCount, roads: roadCount, stations: (snapshot.nodes ?? []).filter((node) => node.kind === 'station').length,
    }) + '\n');
  } finally {
    engine.free?.();
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  runCli(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
