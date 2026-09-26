// Host bindings only. Sensor simulation, geometry, trails, and drawing tables
// are evaluated from source/scene.mec by Mech's resident runtime.
import {WasmSceneProgram} from './runtime.mjs';

export class MechScene {
  constructor(source, instances, estimate, covariance, controls) {
    this.inputs={
      commit:0, 'lane-indices':Float64Array.from({length:instances},(_,i)=>i+1),
      velocity:controls.velocity, omega:controls.omega,
      noise:controls.noise, 'camera-index':controls.landmark,
      'camera-range':controls.range, estimate:Array.from(estimate),
      covariance:{rows:3,columns:3,values:Array.from(covariance)},
    };
    this.program=WasmSceneProgram.fromSource(source,this.inputs);
    try { this.program.turn(this.inputs); }
    catch(error) { this.dispose();throw error; }
  }
  observation(controls, invalid=false) {
    this.program.turn({commit:0,velocity:controls.velocity,omega:controls.omega,
      noise:controls.noise,'camera-index':controls.landmark,'camera-range':controls.range});
    const bearing=Float32Array.from(this.program.readNumbers('readings'));
    const visible=this.program.readNumbers('measurement-visible')[0];
    if(invalid) bearing[bearing.length-1]=NaN;
    return {inputs:{bearing,u:[controls.velocity,controls.omega,visible],
      m:Float32Array.from(this.program.readNumbers('camera-position'))}};
  }
  accepted(estimate,covariance) {
    this.program.turn({commit:1,estimate:Array.from(estimate),
      covariance:{rows:3,columns:3,values:Array.from(covariance)}});
    this.draw();
  }
  draw() {
    const render=globalThis.MechDocumentController?.renderSceneSvg;
    if(!render) throw new Error('The shared Mech scene renderer is unavailable.');
    const svg=render(this.program.scene());
    svg.id='robot-scene';
    svg.setAttribute('aria-label','Robot camera, selectable landmarks, accepted EKF estimate, trail, and covariance');
    document.getElementById('robot-scene').replaceWith(svg);
    const visible=this.program.readNumbers('landmark-visible')[0];
    document.getElementById('measurement-status').textContent=visible
      ? 'Landmark in camera range · bearing correction'
      : 'Landmark outside camera range · motion prediction only';
  }
  dispose() {
    if(!this.program)return;
    try { this.program.stop(); }
    finally { this.program.free();this.program=null; }
  }
}
