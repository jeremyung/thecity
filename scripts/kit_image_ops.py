"""Shared pixel operations for the vehicle and train kit builders.

Extracted verbatim from scripts/build-vehicle-kits.py so a second builder can
reuse them without forking. See scripts/train-kit-plan notes in specs.
"""
import numpy as np


def shift(mask, dy, dx):
    out = np.zeros_like(mask)
    h, w = mask.shape
    out[max(dy, 0):h + min(dy, 0), max(dx, 0):w + min(dx, 0)] = mask[max(-dy, 0):h + min(-dy, 0), max(-dx, 0):w + min(-dx, 0)]
    return out


def dilate(mask, r=1):
    out = mask.copy()
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            if dx * dx + dy * dy <= r * r: out |= shift(mask, dy, dx)
    return out


def erode(mask, r=1):
    return ~dilate(~mask, r)


def flood(seed, allowed):
    """Grow seed through allowed pixels (8-connected) until stable."""
    region = seed & allowed
    while True:
        grown = dilate(region, 1) & allowed
        if grown.sum() == region.sum(): return region
        # Large steps first: repeated single-pixel dilation is slow on 1K images.
        for _ in range(8): grown = dilate(grown, 1) & allowed
        region = grown


def luminance(rgb):
    return rgb[..., 0] * .2126 + rgb[..., 1] * .7152 + rgb[..., 2] * .0722


def separable_background(rgb):
    """Backdrop with a horizon and a horizontal gradient: a per-row level from
    the left/right edges plus a per-column offset from the car-free top band.
    A 20 m train leaves far more backdrop visible than a car, so the per-row
    test alone misclassifies it (see specs/train-kit-plan.md)."""
    h, w, _ = rgb.shape
    edges = np.concatenate([rgb[:, :12], rgb[:, -12:]], axis=1)
    row_bg = np.median(edges, axis=1)
    band = min(150, max(20, h // 5))
    col = np.median(rgb[:band] - row_bg[:band, None, :], axis=0)
    return row_bg[:, None, :] + col[None, :, :]


def foreground(rgb, background='row'):
    h, w, _ = rgb.shape
    border = np.concatenate([rgb[0], rgb[-1], rgb[:, 0], rgb[:, -1]])
    bg = np.median(border, axis=0)
    if background == 'separable':
        near = np.abs(rgb - separable_background(rgb)).max(axis=2) < 14
    else:
        # Background may carry a soft vertical gradient: compare per row.
        row_bg = np.median(np.concatenate([rgb[:, :12], rgb[:, -12:]], axis=1), axis=1)[:, None, :]
        near = np.abs(rgb - row_bg).max(axis=2) < 9
    seed = np.zeros((h, w), bool); seed[0] = seed[-1] = seed[:, 0] = seed[:, -1] = True
    background = flood(seed, near)
    fg = ~background
    # Opening removes ground lines, antennae and stray JPEG speckle.
    fg = dilate(erode(fg, 3), 3)
    # Keep the component holding the image centre of mass.
    ys, xs = np.nonzero(fg)
    cy, cx = int(np.median(ys)), int(np.median(xs))
    seed = np.zeros_like(fg); seed[cy, cx] = True
    if not fg[cy, cx]:
        d = (ys - cy) ** 2 + (xs - cx) ** 2; i = int(np.argmin(d)); seed[ys[i], xs[i]] = True
    fg = flood(seed, fg)
    return fg, bg


def wheels(rgb, fg, box, radius_m, length_m, axles=2, near=None):
    """Two dark discs of known radius that reach the lowest dark row.

    Floor shadow, black bumpers and dark bus doors defeat simpler tests:
    score each column window by dark pixels in a tyre-sized band, and
    require darkness right at the ground under the window centre.
    """
    x0, y0, x1, y1 = box
    L = x1 - x0
    r = radius_m / length_m * L
    dark = (luminance(rgb) < 80) & fg & ((rgb.max(axis=2) - rgb.min(axis=2)) < 50)
    ground = int(np.nonzero(dark[:, x0:x1 + 1].any(axis=1))[0].max())
    band = dark[int(ground - 2 * r):ground + 1, x0:x1 + 1].astype(float)
    profile = band.sum(axis=0)
    w = max(3, int(r * 2))
    score = np.convolve(profile, np.ones(w), mode='same') / (w * 2 * r)
    feet = dark[int(ground - r * .2):ground + 1, x0:x1 + 1].any(axis=0)
    feet = np.convolve(feet.astype(float), np.ones(max(3, int(r * .5))), mode='same') > 0
    score = score * feet
    if near:
        # Another view of the same body located the axles (as fractions of length).
        allowed = np.zeros_like(score)
        for f in near: allowed[max(0, int(f * L - L * .05)):int(f * L + L * .05)] = 1
        score = score * allowed
    found = []
    for _ in range(axles):
        i = int(np.argmax(score))
        if score[i] < .2: break
        found.append((x0 + i, ground - r, r))
        score[max(0, int(i - 4 * r)):int(i + 4 * r)] = 0
    return sorted(found)


def column_outline(mask, box, step):
    x0, y0, x1, y1 = box
    top, bottom = [], []
    for x in list(range(x0, x1, step)) + [x1]:
        ys = np.nonzero(mask[:, x])[0]
        if len(ys) == 0: continue
        top.append((x, ys[0])); bottom.append((x, ys[-1] + 1))
    # Close the ends exactly on the bounding box so faces stay vertical.
    pts = top + bottom[::-1]
    return pts


def simplify(points, tol):
    """Douglas-Peucker on a closed ring."""
    def rdp(seq):
        if len(seq) < 3: return seq
        a, b = np.array(seq[0], float), np.array(seq[-1], float)
        ab = b - a; n = np.hypot(*ab) or 1
        d = [abs(ab[0] * (p[1] - a[1]) - ab[1] * (p[0] - a[0])) / n for p in seq[1:-1]]
        i = int(np.argmax(d))
        if d[i] > tol: return rdp(seq[:i + 2])[:-1] + rdp(seq[i + 1:])
        return [seq[0], seq[-1]]
    half = len(points) // 2
    return rdp(points[:half + 1])[:-1] + rdp(points[half:] + [points[0]])[:-1]


def masks(rgb, fg, body_mask, box, wheel_list):
    x0, y0, x1, y1 = box
    H = y1 - y0
    lum = luminance(rgb)
    sat = rgb.max(axis=2) - rgb.min(axis=2)
    rows = np.arange(rgb.shape[0])[:, None] * np.ones((1, rgb.shape[1]))
    rel = (y1 - rows) / H  # 0 at the ground, 1 at the roof
    tyre = np.zeros_like(fg)
    yy, xx = np.mgrid[0:rgb.shape[0], 0:rgb.shape[1]]
    for cx, cy, r in wheel_list: tyre |= (xx - cx) ** 2 + (yy - cy) ** 2 <= (r * 1.02) ** 2
    body = body_mask & ~tyre
    # Reference paint: the most common colour of the lower-middle body band.
    band = body & (rel > .25) & (rel < .5) & (lum > 25)
    sample = rgb[band]
    q = (sample // 24).astype(int)
    keys, counts = np.unique(q[:, 0] * 10000 + q[:, 1] * 100 + q[:, 2], return_counts=True)
    top = keys[np.argmax(counts)]
    pick = (q[:, 0] * 10000 + q[:, 1] * 100 + q[:, 2]) == top
    paint_ref = sample[pick].mean(axis=0)
    ref_lum = luminance(paint_ref[None])[0]
    chroma = rgb / np.maximum(lum[..., None], 1)
    ref_chroma = paint_ref / max(ref_lum, 1)
    ratio = lum / max(ref_lum, 1)
    ref_sat = paint_ref.max() - paint_ref.min()
    if ref_sat > 28:
        # Saturated metallic paint swings in brightness; match hue instead.
        def hue(c):
            r, g, b = c[..., 0], c[..., 1], c[..., 2]
            return np.arctan2(np.sqrt(3) * (g - b), 2 * r - g - b)
        dh = np.abs(np.angle(np.exp(1j * (hue(rgb) - hue(paint_ref[None])[0]))))
        paint = body & (dh < .35) & (sat > ref_sat * .35) & (ratio > .3) & (ratio < 2.6)
    else:
        paint = body & (np.abs(chroma - ref_chroma).max(axis=2) < .16) & (ratio > .55) & (ratio < 1.3)
    paint = dilate(erode(paint, 1), 1)
    # Shaded panel edges (arch lips, roof rails) fall outside the colour test.
    paint = (paint | (dilate(paint, 3) & body & (lum > 18))) & ~(dilate(body & (lum < 18), 1))
    # Dark trim around arches and sills is not glass: keep glass above mid
    # height and clear of the wheels.
    arches = np.zeros_like(fg)
    for cx, cy, r in wheel_list: arches |= (xx - cx) ** 2 + (yy - cy) ** 2 <= (r * 1.45) ** 2
    glass = body & ~paint & ~arches & (rel > .5) & (lum < max(60, ref_lum * .62)) & (sat < 60)
    glass = dilate(erode(glass, 2), 2) & body
    return paint, glass, paint_ref, ref_lum


def box_blur(a, r):
    """Three separable box passes approximate a Gaussian of sigma ~ r."""
    for _ in range(3):
        for axis in (0, 1):
            c = np.cumsum(np.pad(a, [(r + 1, r) if i == axis else (0, 0) for i in range(2)], mode='edge'), axis=axis)
            hi = np.take(c, range(2 * r + 1, c.shape[axis]), axis=axis)
            lo = np.take(c, range(0, c.shape[axis] - 2 * r - 1), axis=axis)
            a = (hi - lo) / (2 * r + 1)
    return a


def refine_wheel(rgb, cx, ground, r):
    """Best dark tyre ring around a brighter rim, sitting on the ground line."""
    lum = luminance(rgb)
    best = (-1e9, cx, r)
    angles = np.linspace(np.pi * .05, np.pi * .95, 24)  # upper half: the lower rim meets shadow
    for rr in np.linspace(r * .85, r * 1.25, 9):
        cy = ground - rr
        for x in np.arange(cx - r, cx + r + 1, max(1, r / 12)):
            ring = [lum[int(cy - np.sin(a) * rr * .93), int(x + np.cos(a) * rr * .93)] for a in angles]
            inner = [lum[int(cy - np.sin(a) * rr * .45), int(x + np.cos(a) * rr * .45)] for a in angles]
            outside = [lum[int(cy - np.sin(a) * rr * 1.12), int(np.clip(x + np.cos(a) * rr * 1.12, 0, lum.shape[1] - 1))] for a in angles]
            score = np.mean(inner) - np.mean(ring) * 1.5 + np.mean(outside) * .25
            if score > best[0]: best = (score, x, rr)
    return best[1], ground - best[2], best[2]


def bleed(rgb, fg, steps=12):
    """Extend edge colours outward so filtering never mixes in the studio grey."""
    out, known = rgb.copy(), fg.copy()
    for _ in range(steps):
        acc = np.zeros_like(out); n = np.zeros(known.shape)
        for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1), (-2, 0), (2, 0), (0, -2), (0, 2)):
            k = shift(known, dy, dx)
            acc += np.where(k[..., None], np.roll(np.roll(out, dy, 0), dx, 1), 0); n += k
        fill = ~known & (n > 0)
        out[fill] = acc[fill] / n[fill][:, None]
        known |= fill
    return out


def masked_blur(values, mask, radius):
    m = mask.astype(np.float64)
    return box_blur(values * m, radius) / np.maximum(box_blur(m, radius), 1e-3)


def roof_row(r):
    """Row whose colour best stands in for the roof: the longest run of
    uniform, non-glass, non-dark body pixels. Doors and windows are excluded,
    so a bus roof does not inherit dark patches above its doorways."""
    x0, y0, x1, y1 = r['box']
    lum = luminance(r['rgb'])
    best, best_score = (y0 + y1) // 2, -1.0
    for y in range(y0 + int((y1 - y0) * .25), y1 - int((y1 - y0) * .03)):
        cols = r['body'][y, x0:x1 + 1] & ~r['glass'][y, x0:x1 + 1]
        if cols.sum() < (x1 - x0) * .5:
            continue
        values = r['rgb'][y, x0:x1 + 1][cols]
        light = (lum[y, x0:x1 + 1][cols] > 60).mean()
        spread = values.std(axis=0).mean() / 64
        # Prefer high rows: a roof matches the band just below the roofline.
        height = (y1 - y) / (y1 - y0)
        score = cols.mean() * light * max(0.0, 1 - spread) * (.4 + height)
        if score > best_score:
            best, best_score = y, score
    return best
