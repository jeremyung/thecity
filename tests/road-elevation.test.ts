import { describe, expect, it } from 'vitest';
import { bridgeDeckHeight, bridgeLift, bridgeVehiclePose } from '../web/src/render/road-elevation';
describe('bridge approach elevations',()=>{
  it('places vehicle axles on the deck through ascending and descending approaches',()=>{
    const spans=[{a:{x:0,z:0},b:{x:0,z:100}}];
    for(const z of [5,11,50,89,95])for(const yaw of [0,Math.PI]){
      const pose=bridgeVehiclePose({x:0,z},yaw,5.1,spans),rise=Math.tan(pose.pitch)*5.1/2;
      expect(pose.lift+rise).toBeCloseTo(bridgeLift({x:0,z:z+Math.cos(yaw)*2.55},spans),6);
      expect(pose.lift-rise).toBeCloseTo(bridgeLift({x:0,z:z-Math.cos(yaw)*2.55},spans),6);
    }
  });
  it('joins streets continuously, supports both road directions and leaves adjacent banks alone',()=>{
    const span={a:{x:10,z:20},b:{x:70,z:100}},length=100;
    expect(bridgeDeckHeight(0,length)).toBe(.3);expect(bridgeDeckHeight(length,length)).toBe(.3);
    expect(bridgeDeckHeight(50,length)).toBe(2);
    for(let d=0;d<=100;d+=.5){const p={x:10+.6*d,z:20+.8*d};expect(bridgeLift(p,[span])).toBeCloseTo(bridgeLift(p,[{a:span.b,b:span.a}]),6);expect(bridgeLift(p,[span])).toBeGreaterThanOrEqual(0);expect(bridgeLift(p,[span])).toBeLessThanOrEqual(1.7);}
    expect(bridgeLift({x:40+8,z:60-6},[span])).toBe(0);
    expect(bridgeLift({x:10-.6,z:20-.8},[span])).toBe(0);
    expect(Math.abs(bridgeDeckHeight(12.001,100)-bridgeDeckHeight(11.999,100))).toBeLessThan(.001);
  });
});
