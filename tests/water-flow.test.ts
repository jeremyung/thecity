import {expect,it} from 'vitest';
import {Vector3} from 'three/webgpu';
import {waterFlowPhase,WATER_FLOW_CYCLE} from '../web/src/render/water-flow';

it('hides resetting samples and keeps advected detail continuous across both boundaries',()=>{
  const phase=new Vector3();
  const detail=(t:number,flow:number,scale:number,shift:number)=>{
    waterFlowPhase(t,phase);
    // An independent periodic texture with unequal frequencies exposes phase jumps.
    const sample=(u:number)=>Math.sin(u*2*Math.PI)+.3*Math.cos(u*6*Math.PI);
    const a=sample(.231-flow*phase.x*WATER_FLOW_CYCLE/scale);
    const b=sample(.231+shift-flow*phase.y*WATER_FLOW_CYCLE/scale);
    return a*(1-phase.z)+b*phase.z;
  };
  for(const boundary of [0,3,6,9,60,63]){
    waterFlowPhase(boundary,phase);
    if(phase.x===0)expect(1-phase.z).toBe(0);
    if(phase.y===0)expect(phase.z).toBe(0);
    for(const flow of [.15,.6])for(const [scale,shift] of [[38,.37],[13,.61]]){
      expect(Math.abs(detail(boundary-1e-6,flow,scale,shift)-detail(boundary+1e-6,flow,scale,shift))).toBeLessThan(1e-5);
    }
  }
});
