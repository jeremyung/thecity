import { BuildingAccessPaths } from './building-access';
import type { LayerFactory } from './layer';
import { RearExtensions } from './rear-extensions';

/** Registered render layers, in update order. */
export const RENDER_LAYERS: readonly LayerFactory[] = [
  (host) => {
    const paths = new BuildingAccessPaths(host.art, (release) => host.retire(release));
    return {
      name: 'building-access',
      root: paths.root,
      setWorld: (world) => paths.setWorld(world),
      reset: () => paths.reset(),
    };
  },
  (host) => {
    const extensions = new RearExtensions(
      host.art,
      (release) => host.retire(release),
      (building) => host.isBuildingVisible(building),
    );
    return {
      name: 'rear-extensions',
      root: extensions.root,
      setWorld: (world) => extensions.setWorld(world),
      frame: (world) => {
        if (world) extensions.flushCompleted(world);
      },
      reset: () => extensions.reset(),
    };
  },
];
