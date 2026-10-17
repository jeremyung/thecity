import type { World } from '../types';
import { sampledRail, type RailPath } from './rail-path';

export interface ConsistCenter {
  x: number;
  z: number;
  yaw: number;
  route?: { path: RailPath; sign: number };
  distance: number;
}

/** Physical car order follows the route, independent of the travel direction.
 * Pass a world whose infrastructure has committed before using these paths. */
export class TrainPlacement {
  private routes = new Map<number, { yaw: number; path?: RailPath; sign: number }>();

  setWorld(world: World): void {
    this.routes.clear();
    const nodes = new Map(world.nodes.map((node) => [node.id, node]));
    const paths = new Map((world.rail_alignments ?? []).map((a) => [a.road_id, sampledRail(a)]));
    for (const train of world.trains) {
      const a = nodes.get(train.route[0]),
        b = nodes.get(train.route[1]);
      if (!a || !b || Math.hypot(b.x - a.x, b.z - a.z) <= 0.01) continue;
      const road = (world.roads ?? []).find(
        (r) =>
          r.kind === 'rail' &&
          ((r.from === train.route[0] && r.to === train.route[1]) ||
            (r.to === train.route[0] && r.from === train.route[1])),
      );
      this.routes.set(train.id, {
        yaw: Math.atan2(b.x - a.x, b.z - a.z),
        path: road && paths.get(road.id),
        sign: road?.from === train.route[0] ? 1 : -1,
      });
    }
  }

  center(id: number, x: number, z: number, fallbackYaw: number): ConsistCenter {
    const route = this.routes.get(id);
    return {
      x,
      z,
      yaw: route?.yaw ?? fallbackYaw,
      route: route?.path ? { path: route.path, sign: route.sign } : undefined,
      distance: route?.path?.distanceAt({ x, z }) ?? 0,
    };
  }

  at(center: ConsistCenter, offset: number): { x: number; z: number; yaw: number } {
    const { route } = center;
    if (route) {
      const p = route.path.at(center.distance + offset * route.sign);
      return { x: p.x, z: p.z, yaw: Math.atan2(p.ux * route.sign, p.uz * route.sign) };
    }
    return {
      x: center.x + Math.sin(center.yaw) * offset,
      z: center.z + Math.cos(center.yaw) * offset,
      yaw: center.yaw,
    };
  }

  reset(): void {
    this.routes.clear();
  }
}
