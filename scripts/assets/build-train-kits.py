"""Turn generated train elevations into the runtime train kit.

Reads art-workshop/train-kit.json and writes:
  public/art/trains/train-atlas.png     side and front colour, livery baked
  public/art/trains/train-mask.png      G lamp, B glass (R unused: baked livery)
  web/src/render/train-manifest.json    outlines, bogies and atlas rects in metres
  validation/captures/train-kit-debug.png  overlays for review

The side matting uses the separable background model (kit_image_ops) because a
20 m car leaves a visible studio floor and horizontal gradient. Bogies are found
as two dark centres and expanded to two axles each. Thin roof equipment (air
conditioners, pantograph) is cut from the outline and modelled at runtime.

usage: python3 scripts/assets/build-train-kits.py
"""
import json, pathlib
import numpy as np
from PIL import Image, ImageDraw
from kit_image_ops import (dilate, erode, luminance, foreground, wheels, refine_wheel,
                           column_outline, simplify, masks, bleed)

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
CONFIG = json.loads((ROOT / 'art-workshop/train-kit.json').read_text())
WORKSHOP = ROOT / 'art-workshop'
OUT_ART = ROOT / 'public/art/trains'
MANIFEST = ROOT / 'web/src/render/train-manifest.json'
DEBUG = ROOT / 'validation/captures/train-kit-debug.png'


def row_outline(mask, box, step):
    """Closed silhouette for a head-on view: left edge top to bottom, then the
    right edge bottom to top."""
    x0, y0, x1, y1 = box
    left, right = [], []
    for y in list(range(y0, y1, step)) + [y1]:
        xs = np.nonzero(mask[y, x0:x1 + 1])[0]
        if len(xs) == 0:
            continue
        left.append((x0 + xs[0], y))
        right.append((x0 + xs[-1] + 1, y))
    return left + right[::-1]


def to_metres(length, box, px, py):
    x0, y0, x1, y1 = box
    k = length / (x1 - x0)
    z = length / 2 - (px - x0) * k
    y = (y1 - py) * k
    return round(float(z), 4), round(float(y), 4)


def process_side(spec, length, rgb):
    fg, _ = foreground(rgb, background='separable')
    ys, xs = np.nonzero(fg)
    box = [int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())]
    Lpx = box[2] - box[0]
    ppm_img = Lpx / length
    centers = wheels(rgb, fg, box, spec['wheelRadius'], length, axles=2)
    if len(centers) != 2:
        raise SystemExit(f"{spec['id']}: found {len(centers)} bogie centres")
    ground = int(round(centers[0][1] + centers[0][2]))
    centers = [refine_wheel(rgb, cx, ground, r) for cx, _, r in centers]
    shared = float(np.mean([r for _, _, r in centers]))
    centers = [(cx, ground - shared, shared) for cx, _, _ in centers]
    half = spec.get('axleSpacing', 2.1) / 2
    wheel_list = [(cx + s * half * ppm_img, cy, r) for cx, cy, r in centers for s in (-1, 1)]

    fg = fg.copy()
    fg[ground + 1:] = False
    box[3] = ground
    widths = fg[:, box[0]:box[2] + 1].sum(axis=1)
    maxw = widths.max()
    roofline = spec.get('roofline', 0.5)
    top = box[1]
    while top < ground and widths[top] < roofline * maxw:
        top += 1

    yy, xx = np.mgrid[0:fg.shape[0], 0:fg.shape[1]]
    body = fg.copy()
    body[int(ground - spec.get('clearance', .16) * ppm_img):] = False
    for cx, cy, r in wheel_list:
        body &= (xx - cx) ** 2 + (yy - cy) ** 2 > (r * 1.08) ** 2
    body[:top] = False

    sbox = [box[0], top, box[2], ground]
    paint, glass, paint_ref, ref_lum = masks(rgb, fg, body, sbox, wheel_list)
    paint[:] = False  # livery is baked into the atlas
    m = np.zeros(rgb.shape, np.uint8)
    m[..., 2] = glass * 255
    rel = (ground - np.arange(rgb.shape[0])) / max(1, ground - top)
    for (a, b), cols in ((spec['lampFront'], slice(box[0], box[0] + int(Lpx * .05))),
                         (spec['lampRear'], slice(box[2] - int(Lpx * .04), box[2] + 1))):
        rows = (rel >= a) & (rel <= b)
        m[rows, cols, 1] = 255
    m[..., 1] &= (fg * 255).astype(np.uint8)

    out = bleed(rgb.copy(), fg)
    ring = simplify(column_outline(body, [box[0], top, box[2], ground], 2), Lpx * .004)
    outline = [to_metres(length, sbox, px, py) for px, py in ring]
    height = (ground - top + 1) / ppm_img
    glass_rows = np.nonzero(glass.any(axis=1))[0]
    belt = (ground - glass_rows.max()) / ppm_img if len(glass_rows) else height * .6
    result = dict(rgb=out, mask=m, box=sbox, full=[box[0], top, box[2], ground],
                  outline=outline, height=height, belt=belt,
                  wheels=[to_metres(length, sbox, cx, cy) + (round(float(rr / (box[2] - box[0]) * length), 4),) for cx, cy, rr in wheel_list],
                  paint_ref=[round(float(c) / 255, 4) for c in paint_ref], fg=fg, body=body)
    return result


def process_front(spec, width, rgb, height):
    fg, _ = foreground(rgb, background='separable')
    ys, xs = np.nonzero(fg)
    box = [int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())]
    Wpx, Hpx = box[2] - box[0] + 1, box[3] - box[1] + 1
    kx, ky = width / Wpx, height / Hpx
    lum = luminance(rgb)
    glass = fg & (lum < 70)
    glass = dilate(erode(glass, 2), 2) & fg
    m = np.zeros(rgb.shape, np.uint8)
    m[..., 2] = glass * 255
    ring = simplify(row_outline(fg, box, 3), Wpx * .01)
    outline = [[round(float((px - (box[0] + box[2]) / 2) * kx), 4), round(float((box[3] - py) * ky), 4)] for px, py in ring]
    return dict(rgb=bleed(rgb.copy(), fg), mask=m, box=box, outline=outline,
                width=Wpx * kx, height=Hpx * ky, fg=fg)


def main():
    ppm = CONFIG['pixelsPerMeter']
    AW, AH = CONFIG['atlas']
    atlas = Image.new('RGB', (AW, AH), (128, 128, 128))
    mask_full = Image.new('RGB', (AW, AH), (0, 0, 0))
    debug_tiles = []
    manifest = {'version': 1, 'atlas': [AW, AH], 'classes': {}}
    cursor_x, cursor_y, row_h, pad = 4, 4, 0, 4

    def place(w, h):
        nonlocal cursor_x, cursor_y, row_h
        if cursor_x + w + pad > AW:
            cursor_x, cursor_y, row_h = 4, cursor_y + row_h + pad, 0
        x, y = cursor_x, cursor_y
        cursor_x += w + pad
        row_h = max(row_h, h)
        if y + h > AH:
            raise SystemExit(f'atlas overflow at {w}x{h}')
        return x, y

    for cls in CONFIG['classes']:
        entry = {'length': cls['length'], 'width': cls['width'], 'tumblehome': cls['tumblehome'],
                 'gap': cls['gap'], 'consist': cls['consist'], 'cars': {}}
        heights = []
        processed = []
        for spec in cls['cars']:
            rgb = np.asarray(Image.open(WORKSHOP / spec['source']).convert('RGB')).astype(np.float32)
            r = process_side(spec, cls['length'], rgb)
            heights.append(r['height'])
            processed.append((spec, r, rgb))
        height = round(float(np.median(heights)), 3)
        entry['height'] = height
        for spec, r, rgb in processed:
            x0, y0, x1, y1 = r['box']
            tw, th = int(round(cls['length'] * ppm)), int(round(height * ppm))
            ax, ay = place(tw, th)
            crop = lambda arr: Image.fromarray(arr[y0:y1 + 1, x0:x1 + 1]).resize((tw, th), Image.LANCZOS)
            atlas.paste(crop(np.clip(r['rgb'], 0, 255).astype(np.uint8)), (ax, ay))
            mask_full.paste(crop(r['mask']), (ax, ay))
            side = {'uv': [ax / AW, 1 - (ay + th) / AH, (ax + tw) / AW, 1 - ay / AH],
                    'outline': r['outline'], 'belt': round(float(r['belt']), 4),
                    'roofY': round(height - (height - r['belt']) * .35, 4),
                    'wheels': [{'z': z, 'y': y, 'r': rr} for z, y, rr in r['wheels']],
                    'paint': r['paint_ref'], 'roof': spec.get('roof')}
            if spec.get('frontSource'):
                frgb = np.asarray(Image.open(WORKSHOP / spec['frontSource']).convert('RGB')).astype(np.float32)
                fr = process_front(spec, cls['width'], frgb, height)
                fw, fh = int(round(cls['width'] * ppm)), int(round(height * ppm))
                fax, fay = place(fw, fh)
                fr0 = fr['box']
                fcrop = lambda arr: Image.fromarray(arr[fr0[1]:fr0[3] + 1, fr0[0]:fr0[2] + 1]).resize((fw, fh), Image.LANCZOS)
                atlas.paste(fcrop(np.clip(fr['rgb'], 0, 255).astype(np.uint8)), (fax, fay))
                mask_full.paste(fcrop(fr['mask']), (fax, fay))
                side['front'] = {'uv': [fax / AW, 1 - (fay + fh) / AH, (fax + fw) / AW, 1 - fay / AH],
                                 'outline': fr['outline'], 'width': round(float(fr['width']), 3),
                                 'height': round(float(fr['height']), 3)}
            entry['cars'][spec['id']] = {'left': side}
            tile = Image.fromarray(np.clip(r['rgb'], 0, 255).astype(np.uint8)).convert('RGB')
            overlay = np.zeros((*r['fg'].shape, 4), np.uint8)
            overlay[r['body']] = (255, 80, 0, 60)
            tile = Image.alpha_composite(tile.convert('RGBA'), Image.fromarray(overlay, 'RGBA'))
            d = ImageDraw.Draw(tile)
            ring = column_outline(r['body'], r['box'], 2)
            d.line(ring + [ring[0]], fill=(255, 0, 255, 255), width=2)
            for w in side['wheels']:
                k = (x1 - x0) / cls['length']
                cxp = x0 + (cls['length'] / 2 - w['z']) / k
                cyp = y1 - w['y'] / k
                rr = w['r'] / k
                d.ellipse([cxp - rr, cyp - rr, cxp + rr, cyp + rr], outline=(0, 220, 0, 255), width=3)
            d.text((8, 8), f"{cls['id']} {spec['id']} H={height}", fill=(0, 0, 0, 255))
            debug_tiles.append(tile.convert('RGB').resize((960, 360)))
        manifest['classes'][cls['id']] = entry

    OUT_ART.mkdir(parents=True, exist_ok=True)
    atlas.save(OUT_ART / 'train-atlas.png', optimize=True)
    mask_full.resize((AW // 2, AH // 2), Image.BILINEAR).save(OUT_ART / 'train-mask.png', optimize=True)
    MANIFEST.write_text(json.dumps(manifest, indent=1) + '\n')
    sheet = Image.new('RGB', (960, 360 * len(debug_tiles)), 'white')
    for i, t in enumerate(debug_tiles):
        sheet.paste(t, (0, i * 360))
    DEBUG.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(DEBUG)
    for cid, c in manifest['classes'].items():
        for car, body in c['cars'].items():
            s = body['left']
            print(cid, car, 'outline', len(s['outline']), 'wheels', [(w['z'], w['r']) for w in s['wheels']], 'height', c['height'], 'front' if 'front' in s else '')


main()
