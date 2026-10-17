import GTAONode from 'three/addons/tsl/display/GTAONode.js';
import type { Camera, Node, TextureNode } from 'three/webgpu';

/** Three r183 releases AO targets but omits its privately generated noise map. */
class OwnedGTAONode extends GTAONode {
  // Version-specific ownership adapter; this texture is created by GTAONode,
  // unlike its borrowed depth/normal inputs. Recheck on Three upgrades.
  declare private _noiseNode: TextureNode;
  private released = false;
  override dispose() {
    if (this.released) return;
    this.released = true;
    super.dispose();
    this._noiseNode?.value?.dispose?.();
  }
}

export const ownedAO = (depth: Node, normal: Node, camera: Camera) =>
  new OwnedGTAONode(depth, normal, camera);
