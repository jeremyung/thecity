import { describe, expect, it } from 'vitest';
import type { Point } from '../web/src/types';
import { RoadSurfacePartition } from '../web/src/render/road-surface-partition';

const area = (p: readonly Point[]) => Math.abs(p.reduce((sum, a, i) => {
  const b = p[(i + 1) % p.length]; return sum + a.x * b.z - b.x * a.z;
}, 0) / 2);
const total = (polys: readonly Point[][]) => polys.reduce((sum, p) => sum + area(p), 0);
const rect = (x: number, z: number, w: number, h: number): Point[] => [
  { x, z }, { x: x + w, z }, { x: x + w, z: z + h }, { x, z: z + h },
];

// The audit's exact convex intersection polygons for roads 1787 and 1807.
const captured = [
  { progress: .5721463, expected: 1.6199043521568228, polygon: [
    { x: 115.13457375736657, z: 14.554117152383345 }, { x: 115.21146732541071, z: 16.542038444565407 },
    { x: 115.81975664791896, z: 16.51850957720292 }, { x: 116.27864416178983, z: 14.873207207309202 },
  ] },
  { progress: .7151829, expected: 5.49017965957114, polygon: [
    { x: 115.13457375736657, z: 14.554117152383347 }, { x: 115.21146732541071, z: 16.542038444565407 },
    { x: 119.25363034578957, z: 16.385686008241393 }, { x: 119.4303061045262, z: 15.752230017904884 },
  ] },
  { progress: .8582195, expected: 6.163717340021208, polygon: [
    { x: 115.13457375736657, z: 14.554117152383345 }, { x: 115.21146732541071, z: 16.542038444565407 },
    { x: 121.40337155939852, z: 16.302533182756783 },
  ] },
  { progress: 1, expected: 6.163717340021208, polygon: [
    { x: 115.13457375736657, z: 14.554117152383347 }, { x: 115.21146732541071, z: 16.542038444565407 },
    { x: 121.40337155939851, z: 16.302533182756786 },
  ] },
];

// Independent Sutherland–Hodgman intersection used only as a coverage oracle.
function intersectionArea(subject: Point[], cutter: Point[]): number {
  let result = subject;
  const signed = cutter.reduce((s,p,i)=>{const q=cutter[(i+1)%cutter.length];return s+p.x*q.z-q.x*p.z;},0);
  const edges = signed > 0 ? cutter : [...cutter].reverse();
  for(let i=0;i<edges.length;i++){
    const a=edges[i],b=edges[(i+1)%edges.length], next:Point[]=[];
    const side=(p:Point)=>(b.x-a.x)*(p.z-a.z)-(b.z-a.z)*(p.x-a.x);
    for(let j=0;j<result.length;j++){
      const p=result[j],q=result[(j+1)%result.length],u=side(p),v=side(q);
      if(u>=0)next.push(p);
      if((u>=0)!==(v>=0)){const t=u/(u-v);next.push({x:p.x+t*(q.x-p.x),z:p.z+t*(q.z-p.z)});}
    }
    result=next;
  }
  return result.length>2?area(result):0;
}

describe('RoadSurfacePartition', () => {
  it('preserves captured 1787/1807 construction overlap as the arterial advances and opens', () => {
    for (let end = 0; end < captured.length; end++) {
      const partition = new RoadSurfacePartition();
      const emitted: Point[][] = [];
      for (let i = 0; i <= end; i++) emitted.push(...partition.claim(captured[i].polygon));
      expect(area(captured[end].polygon)).toBeCloseTo(captured[end].expected, 8);
      expect(total(emitted)).toBeCloseTo(captured[end].expected, 7);
      // Captured road 1787's first open surface remains owned as construction
      // progress grows; reopening at 1.0 does not alter the union.
      expect(total(emitted)).toBeGreaterThanOrEqual(area(captured[0].polygon) - 1e-8);
    }
  });

  it('partitions rectangles with exact union area and no positive-area overlap', () => {
    const partition = new RoadSurfacePartition();
    const emitted = [
      ...partition.claim(rect(0, 0, 10, 10)),
      ...partition.claim(rect(5, 5, 10, 10)),
      ...partition.claim(rect(8, 2, 8, 10)),
    ];
    expect(total(emitted)).toBeCloseTo(200, 7);
    for(let i=0;i<emitted.length;i++)for(let j=i+1;j<emitted.length;j++)
      expect(intersectionArea(emitted[i],emitted[j])).toBeLessThan(1e-7);

  });

  it('handles rotated, translated acute polygons, both windings, and stable insertion order', () => {
    const acute: Point[] = [{ x: 0, z: 0 }, { x: 17, z: 1 }, { x: 1, z: 3 }];
    const shifted = acute.map(p => ({ x: p.x + 1, z: p.z + .2 }));
    const reversed = [...shifted].reverse();
    const claimAll = () => {
      const partition = new RoadSurfacePartition();
      return [partition.claim(acute), partition.claim(reversed)];
    };
    const first = claimAll(), second = claimAll();
    expect(total(first[0])).toBeCloseTo(area(acute), 8);
    expect(total(first[1])).toBeGreaterThan(0);
    expect(first).toEqual(second);
    expect(total(first.flat())).toBeGreaterThan(area(acute));
    expect(total(first.flat())).toBeLessThan(2 * area(acute));
  });

  it('does not mutate inputs, ignores non-finite/degenerate claims, and keeps distant claims separate', () => {
    const input = rect(-2, -3, 4, 5);
    const before = structuredClone(input);
    const partition = new RoadSurfacePartition();
    expect(total(partition.claim(input))).toBe(20);
    expect(input).toEqual(before);
    expect(partition.claim([{ x: 0, z: 0 }, { x: Infinity, z: 0 }, { x: 1, z: 1 }])).toEqual([]);
    expect(partition.claim([{ x: 0, z: 0 }, { x: 1, z: 1 }, { x: 2, z: 2 }])).toEqual([]);
    expect(total(partition.claim(rect(1000, 1000, 2, 2)))).toBe(4);
  });

  it('forks ownership independently and matches replay for subsequent claims', () => {
    const base = new RoadSurfacePartition();
    base.claim(rect(0, 0, 10, 10));
    const fork = base.fork();

    const parentNext = base.claim(rect(5, 5, 10, 10));
    const forkNext = fork.claim(rect(2, 2, 10, 10));

    const forkReplay = new RoadSurfacePartition();
    forkReplay.claim(rect(0, 0, 10, 10));
    expect(forkNext).toEqual(forkReplay.claim(rect(2, 2, 10, 10)));

    // Claims added to the parent after forking do not enter the fork's index.
    const forkLater = fork.claim(rect(5, 5, 10, 10));
    const independentReplay = new RoadSurfacePartition();
    independentReplay.claim(rect(0, 0, 10, 10));
    independentReplay.claim(rect(2, 2, 10, 10));
    expect(forkLater).toEqual(independentReplay.claim(rect(5, 5, 10, 10)));

    // And fork claims do not alter the parent's next ownership result.
    const parentReplay = new RoadSurfacePartition();
    parentReplay.claim(rect(0, 0, 10, 10));
    expect(parentNext).toEqual(parentReplay.claim(rect(5, 5, 10, 10)));
  });
});
