import { Node, TempNode, TextureNode } from 'three/webgpu';
/** Signed normalized radius: negative foreground, positive background. */
export default class SignedCoCDepthOfFieldNode extends TempNode {
  textureNode: TextureNode;
  signedCoCNode: Node;
  bokehScaleNode: Node;
  constructor(textureNode: TextureNode, signedCoCNode: Node, bokehScaleNode: Node);
  setSize(width: number, height: number): void;
  dispose(): void;
}
/** Radius budget is per gather; sequential passes have wider combined support. */
export function signedCoCDof(node: Node, signedCoC: Node | number, maxGatherRadiusPx?: Node | number): SignedCoCDepthOfFieldNode;
