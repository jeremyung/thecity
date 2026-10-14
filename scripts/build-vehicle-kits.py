"""Turn generated side elevations into the runtime vehicle kit.

Reads art-workshop/vehicle-kit.json and writes:
  public/art/vehicles/vehicle-atlas.png   side colour, paint normalised to grey
  public/art/vehicles/vehicle-mask.png    R paint, G lamp, B glass (half resolution)
  web/src/render/vehicle-manifest.json    outlines, wheels and atlas rects in metres
  validation/captures/vehicle-kit-debug.png  overlays for review

Every source faces left, so image x runs from the front bumper to the rear.
Runtime geometry is built from the manifest in web/src/render/vehicle-kit.ts.

usage: python3 scripts/build-vehicle-kits.py
"""
import json, pathlib
import numpy as np
from PIL import Image, ImageDraw
from kit_image_ops import (shift, dilate, erode, flood, luminance, foreground, wheels,
    column_outline, simplify, masks, box_blur, refine_wheel, bleed, masked_blur, roof_row)

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONFIG = json.loads((ROOT / 'art-workshop/vehicle-kit.json').read_text())
WORKSHOP = ROOT / 'art-workshop'
OUT_ART = ROOT / 'public/art/vehicles'
MANIFEST = ROOT / 'web/src/render/vehicle-manifest.json'
DEBUG = ROOT / 'validation/captures/vehicle-kit-debug.png'


def process(spec, source_name, flip, near=None):
    rgb = np.asarray(Image.open(WORKSHOP / source_name).convert('RGB')).astype(np.float32)
    if flip: rgb = rgb[:, ::-1].copy()  # right-side sources face right; normalise to facing left
    fg, _ = foreground(rgb)
    ys, xs = np.nonzero(fg)
    box = [int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())]
    wheel_list = wheels(rgb, fg, box, spec['wheelRadius'], spec['length'], near=near)
    if len(wheel_list) != 2: raise SystemExit(f"{spec['id']} {source_name}: found {len(wheel_list)} wheels")
    # Everything under the tyre contact line is floor shadow.
    ground = int(round(wheel_list[0][1] + wheel_list[0][2]))
    wheel_list = [refine_wheel(rgb, cx, ground, r) for cx, _, r in wheel_list]
    shared = float(np.mean([r for _, _, r in wheel_list]))
    wheel_list = [(cx, ground - shared, shared) for cx, _, _ in wheel_list]
    fg[ground + 1:] = False
    box[3] = ground
    body_mask = fg.copy()
    # Below the sill the soft under-car shadow reads as body: cut at the
    # ground clearance, and open each wheel well below its axle.
    px_per_m = (box[2] - box[0]) / spec['length']
    body_mask[int(ground - spec.get('clearance', .16) * px_per_m):] = False
    for cx, cy, r in wheel_list:
        body_mask[int(cy):, int(cx - r * 1.1):int(cx + r * 1.1) + 1] = False
    roof = None
    if spec.get('roofCut'):
        width = fg[:, box[0]:box[2] + 1].sum(axis=1)
        L = box[2] - box[0]
        y = box[1]
        while y < box[3] and width[y] < L * .15: y += 1
        if y > box[1] + 2:
            cols = np.nonzero(fg[box[1]:y].any(axis=0))[0]
            roof = (int(cols.min()), int(cols.max()), box[1], y)
            body_mask[:y] = False
            box[1] = y
    yy, xx = np.mgrid[0:fg.shape[0], 0:fg.shape[1]]
    for cx, cy, r in wheel_list: body_mask &= (xx - cx) ** 2 + (yy - cy) ** 2 > (r * 1.08) ** 2
    paint, glass, paint_ref, ref_lum = masks(rgb, fg, body_mask, box, wheel_list)
    if spec.get('paintRegion'):
        a, b = spec['paintRegion']
        cols = np.arange(fg.shape[1])
        L = box[2] - box[0]
        paint &= ((cols >= box[0] + a * L) & (cols <= box[0] + b * L))[None, :]
    if spec.get('tint') is False: paint[:] = False
    lum = luminance(rgb)
    flat = lum / np.maximum(masked_blur(lum, paint, 40), 1)
    out = rgb.copy()
    grey = np.clip(flat * .8, 0, 1) * 255
    out[paint] = grey[paint][:, None]
    out = bleed(out, fg)
    return dict(rgb=out, fg=fg, body=body_mask, paint=paint, glass=glass, box=box, wheels=wheel_list,
                roof=roof, paint_ref=paint_ref)


def to_metres(spec, box, px, py):
    x0, y0, x1, y1 = box
    # Uniform scale from length keeps tyres round; height follows the image.
    L = spec['length']
    k = L / (x1 - x0)
    z = L / 2 - (px - x0) * k  # +z is forward
    y = (y1 - py) * k
    return round(float(z), 4), round(float(y), 4)


def main():
    ppm = CONFIG['pixelsPerMeter']
    AW, AH = CONFIG['atlas']
    atlas = Image.new('RGB', (AW, AH), (128, 128, 128))
    mask_full = Image.new('RGB', (AW, AH), (0, 0, 0))
    debug_tiles = []
    manifest = {'version': 1, 'atlas': [AW, AH], 'bodies': {}}
    cursor_x, cursor_y, row_h, pad = 4, 4, 0, 4

    def place(w, h):
        nonlocal cursor_x, cursor_y, row_h
        if cursor_x + w + pad > AW: cursor_x, cursor_y, row_h = 4, cursor_y + row_h + pad, 0
        x, y = cursor_x, cursor_y
        cursor_x += w + pad; row_h = max(row_h, h)
        if y + h > AH: raise SystemExit(f'atlas overflow at {w}x{h}')
        return x, y

    for spec in CONFIG['bodies']:
        sides = [('left', spec['source'], False)]
        # A door-free right side locates the axles more reliably; process it first.
        if spec.get('sourceRight'): sides.insert(0, ('right', spec['sourceRight'], True))
        near = None
        entry = {k: spec[k] for k in ('length', 'width', 'height', 'tumblehome', 'lampFront', 'lampRear')}
        entry['round'] = spec.get('round', [.12, .08])
        for side, source, flip in sides:
            r = process(spec, source, flip, near)
            if near is None: near = [(cx - r['box'][0]) / (r['box'][2] - r['box'][0]) for cx, _, _ in r['wheels']]
            x0, y0, x1, y1 = r['box']
            # Ground is the tyre bottom even when the roof was cut.
            ground = max(y1, max((cy + rr for _, cy, rr in r['wheels']), default=y1))
            box = [x0, y0, x1, int(round(ground))]
            if r['roof']: full_top = r['roof'][2]
            else: full_top = y0
            full_box = [x0, full_top, x1, box[3]]
            measured = (full_box[3] - full_box[1]) / (x1 - x0) * spec['length']
            if side == 'left' or 'measuredHeight' not in entry: entry['height'] = round(float(measured), 3)
            tw, th = int(round(spec['length'] * ppm)), int(round(measured * ppm))
            ax, ay = place(tw, th)
            crop = lambda arr: Image.fromarray(arr[full_box[1]:full_box[3] + 1, x0:x1 + 1]).resize((tw, th), Image.LANCZOS)
            atlas.paste(crop(np.clip(r['rgb'], 0, 255).astype(np.uint8)), (ax, ay))
            m = np.zeros(r['rgb'].shape, np.uint8)
            m[..., 0] = r['paint'] * 255; m[..., 2] = r['glass'] * 255
            # Lamps: front and rear end bands, configured per body.
            rel = (full_box[3] - np.arange(m.shape[0])) / (full_box[3] - full_box[1])
            for (a, b), cols in ((spec['lampFront'], slice(x0, x0 + int((x1 - x0) * .05))), (spec['lampRear'], slice(x1 - int((x1 - x0) * .04), x1 + 1))):
                rows = (rel >= a) & (rel <= b)
                m[rows, cols, 1] = 255
            m[..., 1] &= (r['fg'] * 255).astype(np.uint8)
            mask_full.paste(crop(m), (ax, ay))
            # Outline in metres, measured against the full (uncut) height so UVs match the atlas.
            full_spec = dict(spec)
            ring = simplify(column_outline(r['body'], r['box'], 2), (x1 - x0) * .004)
            outline = [to_metres(full_spec, full_box, px, py) for px, py in ring]
            glass_rows = np.nonzero(r['glass'].any(axis=1))[0]
            belt = to_metres(full_spec, full_box, x0, glass_rows.max())[1] if len(glass_rows) else spec['height'] * .55
            side_entry = {
                'uv': [ax / AW, 1 - (ay + th) / AH, (ax + tw) / AW, 1 - ay / AH],
                'outline': outline,
                'belt': belt,
                'wheels': [dict(zip(('z', 'y'), to_metres(full_spec, full_box, cx, cy)), r=round(float(rr / (x1 - x0) * spec['length']), 4)) for cx, cy, rr in r['wheels']],
                'paint': [round(float(c) / 255, 4) for c in r['paint_ref']],
                'roofY': to_metres(full_spec, full_box, x0, roof_row(r))[1],
            }
            if r['roof']:
                a, b, top, bottom = r['roof']
                z0, y_top = to_metres(full_spec, full_box, a, top)
                z1, y_base = to_metres(full_spec, full_box, b, bottom)
                side_entry['roofSign'] = {'z0': min(z0, z1), 'z1': max(z0, z1), 'y0': y_base, 'y1': y_top}
            entry[side] = side_entry
            # Debug tile: source with outline, wheels, masks.
            tile = Image.fromarray(np.clip(r['rgb'], 0, 255).astype(np.uint8)).convert('RGB')
            overlay = np.zeros((*r['fg'].shape, 4), np.uint8)
            overlay[r['paint']] = (255, 80, 0, 70); overlay[r['glass']] = (0, 120, 255, 110)
            tile = Image.alpha_composite(tile.convert('RGBA'), Image.fromarray(overlay, 'RGBA'))
            d = ImageDraw.Draw(tile)
            d.line(ring + [ring[0]], fill=(255, 0, 255, 255), width=2)
            for cx, cy, rr in r['wheels']: d.ellipse([cx - rr, cy - rr, cx + rr, cy + rr], outline=(0, 220, 0, 255), width=3)
            d.text((8, 8), f"{spec['id']} {side} paint={tuple(int(c) for c in r['paint_ref'])}", fill=(0, 0, 0, 255))
            debug_tiles.append(tile.convert('RGB').resize((640, 360)))
        if 'right' not in entry: entry['right'] = None
        manifest['bodies'][spec['id']] = entry

    OUT_ART.mkdir(parents=True, exist_ok=True)
    atlas.save(OUT_ART / 'vehicle-atlas.png', optimize=True)
    mask_full.resize((AW // 2, AH // 2), Image.BILINEAR).save(OUT_ART / 'vehicle-mask.png', optimize=True)
    MANIFEST.write_text(json.dumps(manifest, indent=1) + '\n')
    sheet = Image.new('RGB', (1280, 360 * ((len(debug_tiles) + 1) // 2)), 'white')
    for i, t in enumerate(debug_tiles): sheet.paste(t, ((i % 2) * 640, (i // 2) * 360))
    DEBUG.parent.mkdir(parents=True, exist_ok=True); sheet.save(DEBUG)
    print('bodies', list(manifest['bodies']), 'atlas', atlas.size)
    for k, v in manifest['bodies'].items():
        print(k, 'outline pts', len(v['left']['outline']), 'wheels', v['left']['wheels'], 'belt', v['left']['belt'])


main()
