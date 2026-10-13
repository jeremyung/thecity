import { afterEach, describe, expect, it, vi } from "vitest";
import * as THREE from "three/webgpu";
import { BuildingChunks, buildingChunkKey } from "../web/src/render/building-chunks";
import { ArtMaterials } from "../web/src/render/materials";

function authoredBuilding(material: THREE.Material, position: THREE.Vector3, rotation = 0): { group: THREE.Group; geometry: THREE.BufferGeometry } {
  const group = new THREE.Group();
  group.position.copy(position);
  group.rotation.y = rotation;
  const geometry = new THREE.BoxGeometry(2, 3, 4);
  const mesh = new THREE.Mesh(geometry, material);
  mesh.position.set(3, 1, -2);
  mesh.rotation.y = Math.PI / 6;
  group.add(mesh);
  return { group, geometry };
}

function chunksWithMaterials(materials: Map<THREE.Material, THREE.Material>, retired: Array<() => void> = []): BuildingChunks {
  return new BuildingChunks((material) => materials.get(material) ?? material, (release) => retired.push(release));
}

function batchedMeshes(chunks: BuildingChunks): THREE.Mesh[] {
  const meshes: THREE.Mesh[] = [];
  chunks.root.traverse((object) => { if (object instanceof THREE.Mesh) meshes.push(object); });
  return meshes;
}

function triangleCount(mesh: THREE.Mesh): number {
  return (mesh.geometry.index?.count ?? mesh.geometry.getAttribute("position").count) / 3;
}

function largeIndexedBuilding(material: THREE.Material, x: number, vertexCount = 54_000) {
  const group = new THREE.Group(); group.position.x = x;
  const geometry = new THREE.BufferGeometry();
  const positions = new Float32Array(vertexCount * 3), indices = new Uint32Array(vertexCount);
  for (let i = 0; i < vertexCount; i++) { positions[i * 3] = i % 100; positions[i * 3 + 1] = Math.floor(i / 100) % 100; positions[i * 3 + 2] = 0; indices[i] = i; }
  geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  geometry.setIndex(new THREE.BufferAttribute(indices, 1));
  group.add(new THREE.Mesh(geometry, material));
  return { group, geometry };
}

function drain(chunks: BuildingChunks, pendingKeys?: ReadonlySet<string>, maxUpdates = 2000): number {
  let updates = 0;
  while (chunks.pendingCount > 0 && updates < maxUpdates) { chunks.update(pendingKeys); updates++; }
  if (chunks.pendingCount > 0) throw new Error(`Chunk builds did not drain after ${maxUpdates} updates`);
  return updates;
}

afterEach(() => vi.restoreAllMocks());

describe("BuildingChunks", () => {
  it("defers contour vertex reads until updates and safely cancels a leased source", () => {
    const material=new THREE.MeshBasicMaterial(),ink=new THREE.LineBasicNodeMaterial();
    const chunks=new BuildingChunks(m=>m,release=>release(),ink);
    const {group,geometry}=largeIndexedBuilding(material,0);
    const read=vi.spyOn(geometry.getAttribute('position'),'getX');
    const dispose=vi.fn((g:THREE.Group)=>{g.traverse(o=>{if(o instanceof THREE.Mesh)o.geometry.dispose();});});
    chunks.set(901,group,{createGroup:()=>{throw Error('unexpected regeneration');},disposeGroup:dispose});
    expect(read).not.toHaveBeenCalled();
    chunks.update();
    expect(read).toHaveBeenCalledTimes(2000);
    expect(chunks.root.children).toHaveLength(0);
    expect(dispose).not.toHaveBeenCalled();
    chunks.clear();expect(dispose).toHaveBeenCalledTimes(1);
    chunks.clear();expect(dispose).toHaveBeenCalledTimes(1);
    material.dispose();ink.dispose();
  });

  it("reports only exact descriptors represented by committed chunks across replacement, moves, and clear", () => {
    const material = new THREE.MeshBasicMaterial();
    const chunks = new BuildingChunks((surface) => surface, (dispose) => dispose());
    const makeSource = (x: number) => {
      const authored = authoredBuilding(material, new THREE.Vector3(x, 0, 0));
      const descriptor = {
        createGroup: () => authored.group,
        disposeGroup: (group: THREE.Group) => { group.removeFromParent(); authored.geometry.dispose(); },
      };
      return { ...authored, descriptor };
    };

    const original = makeSource(40);
    chunks.set(9, original.group, original.descriptor);
    expect(chunks.isSourceVisible(9, original.descriptor)).toBe(false);
    drain(chunks);
    expect(chunks.isSourceVisible(9, original.descriptor)).toBe(true);

    const replacement = makeSource(42);
    chunks.set(9, replacement.group, replacement.descriptor);
    expect(chunks.isSourceVisible(9, original.descriptor)).toBe(true);
    expect(chunks.isSourceVisible(9, replacement.descriptor)).toBe(false);
    drain(chunks);
    expect(chunks.isSourceVisible(9, original.descriptor)).toBe(false);
    expect(chunks.isSourceVisible(9, replacement.descriptor)).toBe(true);

    const moved = makeSource(260);
    chunks.set(9, moved.group, moved.descriptor);
    expect(chunks.isSourceVisible(9, replacement.descriptor)).toBe(true);
    expect(chunks.isSourceVisible(9, moved.descriptor)).toBe(false);
    drain(chunks);
    expect(chunks.isSourceVisible(9, replacement.descriptor)).toBe(false);
    expect(chunks.isSourceVisible(9, moved.descriptor)).toBe(true);

    chunks.clear();
    expect(chunks.isSourceVisible(9, moved.descriptor)).toBe(false);
    material.dispose();
  });

  it("keeps indexed inputs lossless, indexes non-indexed parts, and preserves face ownership and transformed triangle order", () => {
    const material = new THREE.MeshBasicMaterial({ color: "white" });
    const chunks = new BuildingChunks((surface) => surface, (dispose) => dispose());
    const indexed = new THREE.BufferGeometry();
    indexed.setAttribute("position", new THREE.Float32BufferAttribute([
      0, 0, 0,  2, 0, 0,  2, 2, 0,  0, 2, 0,
    ], 3));
    indexed.setIndex([0, 1, 2, 0, 2, 3]);
    const first = new THREE.Group();
    first.position.set(230, 0, 18);
    first.rotation.y = Math.PI / 3;
    const firstMesh = new THREE.Mesh(indexed, material);
    firstMesh.position.set(1, 0, -2);
    firstMesh.rotation.y = Math.PI / 7;
    first.add(firstMesh);

    const nonIndexed = new THREE.BufferGeometry();
    nonIndexed.setAttribute("position", new THREE.Float32BufferAttribute([
      0, 0, 0,  0, 2, 0,  -2, 0, 0,
    ], 3));
    const second = new THREE.Group();
    second.position.set(245, 0, 31);
    second.rotation.y = -Math.PI / 5;
    const secondMesh = new THREE.Mesh(nonIndexed, material);
    secondMesh.position.set(-1, 0, 2);
    second.add(secondMesh);
    first.updateMatrixWorld(true);
    second.updateMatrixWorld(true);

    const expectedTriangles: THREE.Vector3[][] = [];
    const collect = (geometry: THREE.BufferGeometry, matrix: THREE.Matrix4) => {
      const positions = geometry.getAttribute("position");
      const index = geometry.index;
      const indices = index ? Array.from({ length: index.count }, (_, i) => index.getX(i)) : Array.from({ length: positions.count }, (_, i) => i);
      for (let i = 0; i < indices.length; i += 3) {
        expectedTriangles.push(indices.slice(i, i + 3).map((vertex) => new THREE.Vector3().fromBufferAttribute(positions, vertex).applyMatrix4(matrix)));
      }
    };
    collect(indexed, firstMesh.matrixWorld);
    collect(nonIndexed, secondMesh.matrixWorld);
    chunks.set(11, first);
    chunks.set(12, second);
    drain(chunks);

    const [batch] = batchedMeshes(chunks);
    expect(triangleCount(batch)).toBe(3);
    expect(batch.geometry.index).toBeDefined();
    const spans = batch.userData.buildingSpans as Array<{ first: number; end: number; id: number }>;
    expect(spans).toEqual([{ first: 0, end: 2, id: 11 }, { first: 2, end: 3, id: 12 }]);
    for (let face = 0; face < expectedTriangles.length; face++) {
      const tri = expectedTriangles[face];
      for (let corner = 0; corner < 3; corner++) {
        const index = batch.geometry.index!.getX(face * 3 + corner);
        const actual = new THREE.Vector3().fromBufferAttribute(batch.geometry.getAttribute("position"), index);
        expect(actual.distanceTo(tri[corner])).toBeLessThan(1e-5);
      }
    }
    expect(chunks.buildingAt({ object: batch, faceIndex: 0 } as unknown as THREE.Intersection)).toBe(11);
    expect(chunks.buildingAt({ object: batch, faceIndex: 1 } as unknown as THREE.Intersection)).toBe(11);
    expect(chunks.buildingAt({ object: batch, faceIndex: 2 } as unknown as THREE.Intersection)).toBe(12);

    chunks.clear();
    indexed.dispose(); nonIndexed.dispose(); material.dispose();
  });

  it("retains box indexing and reduces merged position storage without dropping triangles", () => {
    const material = new THREE.MeshBasicMaterial();
    const chunks = new BuildingChunks((surface) => surface, (dispose) => dispose());
    const first = authoredBuilding(material, new THREE.Vector3(230, 0, 0));
    const second = authoredBuilding(material, new THREE.Vector3(250, 0, 0));
    chunks.set(1, first.group); chunks.set(2, second.group); drain(chunks);
    const [batch] = batchedMeshes(chunks);
    const position = batch.geometry.getAttribute("position");
    const indices = batch.geometry.index!;
    expect(triangleCount(batch)).toBe(24);
    expect(position.count).toBe(48); // two indexed boxes; non-indexed expansion would need 144 vertices
    expect(indices.count).toBe(72);
    expect(position.array.byteLength + indices.array.byteLength).toBeLessThan(144 * 3 * Float32Array.BYTES_PER_ELEMENT);
    chunks.clear();
    first.geometry.dispose(); second.geometry.dispose(); material.dispose();
  });

  it("batches colors into exact linear vertex data and preserves shader classes across style changes", () => {
    const art=new ArtMaterials(),retired:Array<()=>void>=[];
    const red=art.get('#b35f49'),blue=art.get('#4b7c86');
    const lantern=art.get('#e9aa58',.42,true,'shop'),homeLamp=art.get('#e9aa58',.42,true,'home');
    const chunks=new BuildingChunks(material=>art.buildingBatch(material),release=>retired.push(release));
    const first=authoredBuilding(red,new THREE.Vector3(224,0,12));
    const second=authoredBuilding(blue,new THREE.Vector3(244,0,24));
    chunks.set(1,first.group);chunks.set(2,second.group);drain(chunks);

    const [batch]=batchedMeshes(chunks),position=batch.geometry.getAttribute('position'),colors=batch.geometry.getAttribute('color');
    expect(batchedMeshes(chunks)).toHaveLength(1); // Color variants of one render class share a submission.
    expect(batch.material).toBe(art.buildingBatch(red).material);
    expect(colors.count).toBe(position.count);
    const spans=batch.userData.buildingSpans as Array<{first:number,end:number,id:number}>;
    const expectedById=new Map([[1,(red as THREE.MeshToonNodeMaterial).color],[2,(blue as THREE.MeshToonNodeMaterial).color]]);
    for(const span of spans){
      const expected=expectedById.get(span.id)!;
      for(let face=span.first;face<span.end;face++)for(let corner=0;corner<3;corner++){
        const vertex=batch.geometry.index!.getX(face*3+corner);
        expect(colors.getX(vertex)).toBeCloseTo(expected.r,6);
        expect(colors.getY(vertex)).toBeCloseTo(expected.g,6);
        expect(colors.getZ(vertex)).toBeCloseTo(expected.b,6);
      }
    }
    expect(art.buildingBatch(lantern).material).not.toBe(art.buildingBatch(homeLamp).material);
    expect(art.buildingBatch(red).material).not.toBe(art.buildingBatch(art.get('#b35f49',.4)).material);
    expect(art.variant(red,'miniature')).not.toBe(art.buildingBatch(red).material);
    expect((art.variant(red,'miniature') as THREE.MeshStandardNodeMaterial).color.toArray()).toEqual((red as THREE.MeshToonNodeMaterial).color.toArray());

    const geometry=batch.geometry,attribute=batch.geometry.getAttribute('color'),oldMaterial=batch.material as THREE.Material;
    art.setStyle('miniature');
    batch.material=art.variant(oldMaterial,'miniature');
    expect(batch.material).toBeInstanceOf(THREE.MeshStandardNodeMaterial);
    expect(batch.material).not.toBe(oldMaterial);
    expect(batch.geometry).toBe(geometry);
    expect(batch.geometry.getAttribute('color')).toBe(attribute);

    chunks.clear();for(const release of retired.splice(0))release();
    first.geometry.dispose();second.geometry.dispose();art.dispose();
  });

  it("keeps selective contours out of selection and releases their buffers with a retired chunk", () => {
    const material=new THREE.MeshBasicMaterial(),ink=new THREE.LineBasicNodeMaterial();
    const retired:Array<()=>void>=[],chunks=new BuildingChunks(m=>m,release=>retired.push(release),ink);
    const source=authoredBuilding(material,new THREE.Vector3(35,0,-10));
    chunks.set(77,source.group);drain(chunks);
    let lines:THREE.LineSegments|undefined;
    chunks.root.traverse(o=>{if(o instanceof THREE.LineSegments)lines=o;});
    expect(lines).toBeDefined();
    const positions=lines!.geometry.getAttribute('position');
    expect(positions.count).toBeGreaterThan(0);
    for(let i=0;i<positions.count;i+=2){const a=new THREE.Vector3().fromBufferAttribute(positions,i),b=new THREE.Vector3().fromBufferAttribute(positions,i+1);expect(a.distanceTo(b)).toBeGreaterThanOrEqual(3.5);}
    const hits:THREE.Intersection[]=[];lines!.raycast(new THREE.Raycaster(),hits);expect(hits).toHaveLength(0);
    const surface=batchedMeshes(chunks)[0],geometry=surface.geometry;
    chunks.setContoursVisible(false);expect(lines!.visible).toBe(false);
    expect(surface.geometry).toBe(geometry);expect(chunks.buildingAt({object:surface,faceIndex:0,distance:0,point:new THREE.Vector3()} as THREE.Intersection)).toBe(77);
    const disposed=vi.fn();lines!.geometry.addEventListener('dispose',disposed);
    chunks.clear();expect(disposed).not.toHaveBeenCalled();retired.forEach(release=>release());expect(disposed).toHaveBeenCalledTimes(1);
    source.geometry.dispose();material.dispose();ink.dispose();
  });

  it("preserves rotated world transforms and material grouping while retaining all triangles", () => {
    const sourceMaterialA = new THREE.MeshBasicMaterial({ color: "red" });
    const sourceMaterialB = new THREE.MeshBasicMaterial({ color: "blue" });
    const cachedA = new THREE.MeshBasicMaterial({ color: "red" });
    const cachedB = new THREE.MeshBasicMaterial({ color: "blue" });
    const retired: Array<() => void> = [];
    const chunks = chunksWithMaterials(new Map([[sourceMaterialA, cachedA], [sourceMaterialB, cachedB]]), retired);
    const first = authoredBuilding(sourceMaterialA, new THREE.Vector3(212, 0, -34), Math.PI / 3);
    const second = authoredBuilding(sourceMaterialA, new THREE.Vector3(238, 0, -12), -Math.PI / 5);
    const third = authoredBuilding(sourceMaterialB, new THREE.Vector3(245, 0, -8), Math.PI / 8);
    first.group.updateMatrixWorld(true);
    const sourceMesh = first.group.children[0] as THREE.Mesh;
    const sourcePoint = new THREE.Vector3().fromBufferAttribute(first.geometry.getAttribute("position"), 7).applyMatrix4(sourceMesh.matrixWorld);

    chunks.set(101, first.group);
    chunks.set(102, second.group);
    chunks.set(103, third.group);
    drain(chunks); // The spatial index rebuilds one dirty chunk at a time.

    const meshes = batchedMeshes(chunks);
    expect(meshes).toHaveLength(2); // Identical cached materials share one draw batch.
    expect(meshes.map((mesh) => mesh.material)).toEqual(expect.arrayContaining([cachedA, cachedB]));
    expect(meshes.reduce((sum, mesh) => sum + triangleCount(mesh), 0)).toBe(12 * 3);
    const mergedPositions = meshes.flatMap((mesh) => {
      const position = mesh.geometry.getAttribute("position");
      return Array.from({ length: position.count }, (_, index) => new THREE.Vector3().fromBufferAttribute(position, index));
    });
    expect(mergedPositions.some((position) => position.distanceTo(sourcePoint) < 1e-5)).toBe(true);

    const sharedBatch = meshes.find((mesh) => mesh.material === cachedA)!;
    expect(chunks.buildingAt({ object: sharedBatch, faceIndex: 0 } as unknown as THREE.Intersection)).toBe(101);
    expect(chunks.buildingAt({ object: sharedBatch, faceIndex: 12 } as unknown as THREE.Intersection)).toBe(102);
    const separateBatch = meshes.find((mesh) => mesh.material === cachedB)!;
    expect(chunks.buildingAt({ object: separateBatch, faceIndex: 0 } as unknown as THREE.Intersection)).toBe(103);
    expect(chunks.buildingAt({ object: separateBatch } as unknown as THREE.Intersection)).toBeUndefined();

    chunks.clear();
    for (const release of retired.splice(0)) release();
    sourceMaterialA.dispose(); sourceMaterialB.dispose(); cachedA.dispose(); cachedB.dispose();
    first.geometry.dispose(); second.geometry.dispose(); third.geometry.dispose();
  });

  it("coalesces repeated source updates to the latest mesh and releases replaced geometry after replacement", () => {
    const material = new THREE.MeshBasicMaterial();
    const retired: Array<() => void> = [];
    const chunks = chunksWithMaterials(new Map(), retired);
    const initial = authoredBuilding(material, new THREE.Vector3(220, 0, 0));
    chunks.set(7, initial.group);
    drain(chunks);
    const oldBatchGeometry = batchedMeshes(chunks)[0].geometry;
    const oldDispose = vi.fn();
    oldBatchGeometry.addEventListener("dispose", oldDispose);

    const intermediate = authoredBuilding(material, new THREE.Vector3(225, 0, 0));
    const latest = authoredBuilding(material, new THREE.Vector3(245, 0, 0));
    const releaseInitial = vi.fn();
    const releaseIntermediate = vi.fn();
    chunks.set(7, intermediate.group, releaseInitial);
    chunks.set(7, latest.group, releaseIntermediate);
    expect(releaseInitial).not.toHaveBeenCalled();
    expect(releaseIntermediate).not.toHaveBeenCalled();
    drain(chunks);

    const currentBatch = batchedMeshes(chunks)[0];
    expect(triangleCount(currentBatch)).toBe(12); // The two pending updates coalesce.
    expect(chunks.buildingAt({ object: currentBatch, faceIndex: 0 } as unknown as THREE.Intersection)).toBe(7);
    const latestCenter = new THREE.Vector3(245, 0, 0).add(new THREE.Vector3(3, 1, -2).applyAxisAngle(new THREE.Vector3(0, 1, 0), 0));
    currentBatch.geometry.computeBoundingBox();
    expect(currentBatch.geometry.boundingBox!.getCenter(new THREE.Vector3()).distanceTo(latestCenter)).toBeLessThan(1e-5);
    expect(releaseInitial).toHaveBeenCalledTimes(1);
    expect(releaseIntermediate).toHaveBeenCalledTimes(1);
    expect(oldDispose).not.toHaveBeenCalled();
    expect(retired).toHaveLength(1);
    retired.shift()!();
    expect(oldDispose).toHaveBeenCalledTimes(1);

    chunks.clear();
    for (const release of retired.splice(0)) release();
    material.dispose();
    initial.geometry.dispose(); intermediate.geometry.dispose(); latest.geometry.dispose();
  });

  it("keeps an immutable in-flight revision visible to completion while a newer source waits", () => {
    const material = new THREE.MeshBasicMaterial();
    const chunks = new BuildingChunks((surface) => surface, (dispose) => dispose());
    const initial = authoredBuilding(material, new THREE.Vector3(220, 0, 0));
    chunks.set(88, initial.group); drain(chunks);
    const previous = batchedMeshes(chunks)[0];
    const large = largeIndexedBuilding(material, 230);
    const latest = authoredBuilding(material, new THREE.Vector3(245, 0, 0));
    const releaseInitial = vi.fn(), releaseLarge = vi.fn();

    chunks.set(88, large.group, releaseInitial);
    chunks.update(); // Transform only the first 12k vertices / 36k indices of this oversized source.
    expect(batchedMeshes(chunks)[0]).toBe(previous); // No partial batch becomes visible.
    chunks.update(); // Continue the captured revision without exceeding per-update preparation quotas.
    expect(batchedMeshes(chunks)[0]).toBe(previous);

    chunks.set(88, latest.group, releaseLarge);
    for(let i=0;i<20&&batchedMeshes(chunks)[0]===previous;i++)chunks.update();
    // The captured large revision publishes atomically before the queued latest source.
    expect(batchedMeshes(chunks)[0].geometry.getAttribute("position").count).toBe(54_000);
    expect(releaseInitial).toHaveBeenCalledTimes(1);
    expect(releaseLarge).not.toHaveBeenCalled(); // Newer source remains live for its queued rebuild.
    expect(chunks.pendingCount).toBe(1);

    drain(chunks);
    const current = batchedMeshes(chunks)[0];
    expect(current.geometry.getAttribute("position").count).toBe(24);
    expect(chunks.buildingAt({ object: current, faceIndex: 0 } as unknown as THREE.Intersection)).toBe(88);
    expect(releaseLarge).toHaveBeenCalledTimes(1);

    chunks.clear(); initial.geometry.dispose(); large.geometry.dispose(); latest.geometry.dispose(); material.dispose();
  });

  it("cancels transient staged output on clear and releases queued sources exactly once", () => {
    const material = new THREE.MeshBasicMaterial(),retired:Array<()=>void>=[];
    const chunks = new BuildingChunks((surface) => surface, (dispose) => retired.push(dispose));
    const initial = authoredBuilding(material, new THREE.Vector3(220, 0, 0));
    chunks.set(89, initial.group);drain(chunks);
    const large = largeIndexedBuilding(material, 230),releaseOld=vi.fn();
    chunks.set(89,large.group,releaseOld);chunks.update();
    expect(chunks.pendingCount).toBe(1);
    expect(batchedMeshes(chunks)).toHaveLength(1);
    const visibleBatchGeometry=batchedMeshes(chunks)[0].geometry,visibleDispose=vi.fn();
    visibleBatchGeometry.addEventListener('dispose',visibleDispose);
    chunks.clear();
    expect(chunks.pendingCount).toBe(0);
    expect(batchedMeshes(chunks)).toHaveLength(0);
    expect(releaseOld).toHaveBeenCalledTimes(1);
    expect(retired).toHaveLength(1);
    // Retirement owns the visible old batch and runs only once when drained.
    expect(visibleDispose).not.toHaveBeenCalled();retired.shift()!();expect(visibleDispose).toHaveBeenCalledTimes(1);
    chunks.clear();expect(releaseOld).toHaveBeenCalledTimes(1);expect(retired).toHaveLength(0);
    initial.geometry.dispose();large.geometry.dispose();material.dispose();
  });

  it("re-resolves the current style at commit without recoloring staged geometry and honors current contour visibility", () => {
    const sourceMaterial=new THREE.MeshBasicMaterial(),anime=new THREE.MeshBasicMaterial(),miniature=new THREE.MeshBasicMaterial();
    const ink=new THREE.LineBasicNodeMaterial(),group=new THREE.Group();group.position.set(230,0,10);
    const sourceGeometries:THREE.BufferGeometry[]=[];
    for(let i=0;i<20;i++){const geometry=new THREE.BoxGeometry(2,3,4),mesh=new THREE.Mesh(geometry,sourceMaterial);mesh.position.x=i*3;group.add(mesh);sourceGeometries.push(geometry);}
    let miniatureStyle=false;const authoredColor=new THREE.Color('#b35f49');
    const chunks=new BuildingChunks(()=>({material:miniatureStyle?miniature:anime,color:authoredColor}),dispose=>dispose(),ink);
    chunks.set(90,group);chunks.update(); // Prepared mesh/contour slice; still staging.
    expect(batchedMeshes(chunks)).toHaveLength(0);
    miniatureStyle=true;chunks.setContoursVisible(false);chunks.update();
    const batches=batchedMeshes(chunks);
    expect(batches.length).toBeGreaterThan(0);
    for(const batch of batches){
      expect(batch.material).toBe(miniature);
      const colors=batch.geometry.getAttribute('color');
      for(let channel=0;channel<3;channel++)expect(colors.array[channel]).toBeCloseTo(authoredColor.toArray()[channel],6);
    }
    let lines:THREE.LineSegments|undefined;chunks.root.traverse(o=>{if(o instanceof THREE.LineSegments)lines=o;});
    expect(lines).toBeDefined();expect(lines!.visible).toBe(false);
    expect(batches.reduce((sum,batch)=>sum+batch.geometry.getAttribute('position').count,0)).toBe(20*24);
    chunks.clear();sourceGeometries.forEach(geometry=>geometry.dispose());sourceMaterial.dispose();anime.dispose();miniature.dispose();ink.dispose();
  });

  it("defers a partially streamed chunk, then batches all source triangles and picking spans together", () => {
    const material = new THREE.MeshBasicMaterial();
    const chunks = new BuildingChunks((surface) => surface, (dispose) => dispose());
    const first = authoredBuilding(material, new THREE.Vector3(40, 0, 30));
    const second = authoredBuilding(material, new THREE.Vector3(70, 0, 50));
    const pending = new Set([buildingChunkKey(40, 30)]);

    chunks.set(201, first.group);
    chunks.update(pending);
    expect(chunks.pendingCount).toBe(1);
    expect(batchedMeshes(chunks)).toHaveLength(0);

    chunks.set(202, second.group);
    chunks.update(pending);
    expect(chunks.pendingCount).toBe(1);
    expect(batchedMeshes(chunks)).toHaveLength(0);

    drain(chunks);
    const [batch] = batchedMeshes(chunks);
    expect(triangleCount(batch)).toBe(24);
    expect(chunks.pendingCount).toBe(0);
    expect(chunks.buildingAt({ object: batch, faceIndex: 0 } as unknown as THREE.Intersection)).toBe(201);
    expect(chunks.buildingAt({ object: batch, faceIndex: 12 } as unknown as THREE.Intersection)).toBe(202);

    chunks.clear();
    first.geometry.dispose(); second.geometry.dispose(); material.dispose();
  });

  it("commits a continuously pending chunk after twelve update calls", () => {
    const material = new THREE.MeshBasicMaterial();
    const chunks = new BuildingChunks((surface) => surface, (dispose) => dispose());
    const key = buildingChunkKey(40, 30);
    const pending = new Set([key]);
    const groups = Array.from({ length: 12 }, (_, index) => authoredBuilding(
      material,
      new THREE.Vector3(40 + index, 0, 30),
    ));

    for (let frame = 0; frame < 11; frame += 1) {
      chunks.set(300 + frame, groups[frame].group);
      chunks.update(pending);
      expect(chunks.pendingCount).toBe(1);
      expect(batchedMeshes(chunks)).toHaveLength(0);
    }
    chunks.set(311, groups[11].group);
    chunks.update(pending);

    expect(chunks.pendingCount).toBe(0);
    expect(batchedMeshes(chunks)).toHaveLength(1);
    expect(chunks.pendingCount).toBe(0);
    expect(triangleCount(batchedMeshes(chunks)[0])).toBe(12 * 12);
    for (let index = 0; index < groups.length; index += 1) {
      expect(chunks.buildingAt({ object: batchedMeshes(chunks)[0], faceIndex: index * 12 } as unknown as THREE.Intersection))
        .toBe(300 + index);
    }

    chunks.clear();
    for (const group of groups) group.geometry.dispose();
    material.dispose();
  });

  it("builds a ready chunk while a different incoming chunk remains deferred", () => {
    const material = new THREE.MeshBasicMaterial();
    const chunks = new BuildingChunks((surface) => surface, (dispose) => dispose());
    const busy = authoredBuilding(material, new THREE.Vector3(40, 0, 30));
    const ready = authoredBuilding(material, new THREE.Vector3(260, 0, 30));
    const busyKey = buildingChunkKey(40, 30);
    const readyKey = buildingChunkKey(260, 30);

    chunks.set(401, busy.group);
    chunks.set(402, ready.group);
    chunks.update(new Set([busyKey]));
    chunks.update(new Set([busyKey]));

    const meshes = batchedMeshes(chunks);
    expect(meshes).toHaveLength(1);
    expect(meshes[0].parent?.name).toBe(`Neighborhood ${readyKey}`);
    expect(chunks.pendingCount).toBe(1);

    for (let update = 0; update < 10; update += 1) chunks.update(new Set([busyKey]));
    expect(batchedMeshes(chunks).map((mesh) => mesh.parent?.name)).toContain(`Neighborhood ${busyKey}`);
    expect(chunks.pendingCount).toBe(0);

    chunks.clear();
    busy.geometry.dispose(); ready.geometry.dispose(); material.dispose();
  });

  it("releases replaced sources only when their coalesced chunk commits or clears", () => {
    const material = new THREE.MeshBasicMaterial();
    const deferredRetirements: Array<() => void> = [];
    const chunks = new BuildingChunks((surface) => surface, (dispose) => deferredRetirements.push(dispose));
    const initial = authoredBuilding(material, new THREE.Vector3(40, 0, 30));
    const replacement = authoredBuilding(material, new THREE.Vector3(60, 0, 30));
    const key = buildingChunkKey(40, 30);
    chunks.set(501, initial.group);
    drain(chunks);
    const oldBatch = batchedMeshes(chunks)[0];
    const oldBatchGroup = oldBatch.parent!;
    const releaseAtCommit = vi.fn();
    chunks.set(501, replacement.group, releaseAtCommit);
    chunks.update(new Set([key]));
    expect(releaseAtCommit).not.toHaveBeenCalled();
    expect(oldBatchGroup.parent).toBe(chunks.root);

    drain(chunks);
    expect(releaseAtCommit).toHaveBeenCalledTimes(1);
    expect(oldBatchGroup.parent).toBeNull();
    expect(deferredRetirements).toHaveLength(1);
    deferredRetirements.shift()!();

    const final = authoredBuilding(material, new THREE.Vector3(80, 0, 30));
    const releaseAtClear = vi.fn();
    chunks.set(501, final.group, releaseAtClear);
    chunks.update(new Set([key]));
    expect(releaseAtClear).not.toHaveBeenCalled();
    chunks.clear();
    expect(releaseAtClear).toHaveBeenCalledTimes(1);

    for (const dispose of deferredRetirements.splice(0)) dispose();
    initial.geometry.dispose(); replacement.geometry.dispose(); final.geometry.dispose(); material.dispose();
  });

  it("removes and clears batches once while leaving shared cached materials owned by the caller", () => {
    const sourceMaterial = new THREE.MeshBasicMaterial({ color: "green" });
    const cachedMaterial = new THREE.MeshBasicMaterial({ color: "green" });
    const cachedDispose = vi.fn();
    cachedMaterial.addEventListener("dispose", cachedDispose);
    const retired: Array<() => void> = [];
    const chunks = chunksWithMaterials(new Map([[sourceMaterial, cachedMaterial]]), retired);
    const authored = authoredBuilding(sourceMaterial, new THREE.Vector3(230, 0, 0));
    chunks.set(1, authored.group);
    drain(chunks);
    const batchGeometry = batchedMeshes(chunks)[0].geometry;
    const batchDispose = vi.fn();
    batchGeometry.addEventListener("dispose", batchDispose);
    const releaseSource = vi.fn();
    chunks.remove(1, releaseSource);
    expect(releaseSource).not.toHaveBeenCalled();
    chunks.clear();
    expect(releaseSource).toHaveBeenCalledTimes(1);
    expect(batchDispose).not.toHaveBeenCalled(); // Renderer-controlled retirement is deferred.
    expect(retired).toHaveLength(1);
    retired.shift()!();
    expect(batchDispose).toHaveBeenCalledTimes(1);
    chunks.clear();
    expect(releaseSource).toHaveBeenCalledTimes(1);
    expect(retired).toHaveLength(0);
    expect(cachedDispose).not.toHaveBeenCalled();

    sourceMaterial.dispose(); cachedMaterial.dispose(); authored.geometry.dispose();
  });
});
