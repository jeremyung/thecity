/** Exterior visual envelope: gallery and stair occupy separate rear strips. */
export function circulationLayout(width:number,depth:number){
 const bodyWidth=width-1.3,bodyDepth=depth-4.4,centerZ=-.05;
 const rear=centerZ-bodyDepth/2,front=centerZ+bodyDepth/2;
 return {bodyWidth,bodyDepth,centerZ,rear,front,galleryZ:rear-.5,galleryDepth:.9,
  stairZ:rear-1.43,stairWidth:.82,stairLength:Math.min(4.8,bodyWidth-1.8),landingWidth:.85,
  external:bodyWidth>=6.2};
}
