// Host bindings only. Sensor simulation, geometry, trails, and drawing tables
// are evaluated from source/scene.mec by Mech's resident runtime.
import {WasmSceneProgram} from './runtime.mjs';

export class MechScene {
  constructor(source, instances, estimate, covariance, controls) {
    this.cameraPulses=[0,0,0,0];
    this.instances=instances;
    this.packetSize=Math.min(instances,4096);
    this.inputs={
      commit:0, 'lane-indices':this.laneIndices(0),
      ...this.prepareInputs(controls), estimate:Array.from(estimate),
      covariance:{rows:3,columns:3,values:Array.from(covariance)},
    };
    this.program=WasmSceneProgram.fromSource(source,this.inputs);
    try { this.program.turn(this.inputs); }
    catch(error) { this.dispose();throw error; }
  }
  laneIndices(offset) {
    return Float64Array.from({length:this.packetSize},(_,index)=>Math.min(offset+index+1,this.instances));
  }
  prepareInputs(controls) {
    return {commit:0,velocity:controls.velocity,omega:controls.omega,
      noise:controls.noise,'motion-noise':controls.motionNoise,
      'camera-range':controls.range,'camera-pulses':Array.from(this.cameraPulses)};
  }
  toggleCamera(index) {
    if(!Number.isInteger(index)||index<1||index>4) throw new Error('Camera index must be 1 through 4.');
    // Queue presses only. The Mech resident state applies their parity at the
    // next observation/refresh, after any in-flight numerical turn commits.
    this.cameraPulses[index-1]++;
  }
  refresh(controls) {
    this.program.turn(this.prepareInputs(controls));
    this.draw();
  }
  observation(controls, invalid=false) {
    // Bound the resident sensor workspace while preserving every lane's global
    // index and noise phase. All packets prepare one simulation instant; only
    // accepted() commits that instant after the complete numerical batch passes.
    const prepared=this.prepareInputs(controls);
    const measurements=new Float32Array(12*this.instances);
    for(let offset=0;offset<this.instances;offset+=this.packetSize) {
      this.program.turn({...prepared,'lane-indices':this.laneIndices(offset)});
      const packet=this.program.readNumbers('measurements');
      const count=Math.min(this.packetSize,this.instances-offset);
      measurements.set(packet.subarray(0,12*count),12*offset);
    }
    if(invalid) measurements[measurements.length-11]=NaN;
    return {inputs:{measurements,
      control:Float32Array.from(this.program.readNumbers('control')),
      cameras:Float32Array.from(this.program.readNumbers('cameras'))}};
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
    svg.setAttribute('role','group');
    svg.setAttribute('aria-label','Actual robot and solid cyan path, yellow EKF estimate and dotted gold path, four fixed cameras with clickable enable controls and sensing circles, and covariance');
    const enabled=Array.from(this.program.readNumbers('camera-enabled'));
    const focusedCamera=document.activeElement?.getAttribute?.('data-camera-index');
    for(let index=1;index<=4;index++) {
      const camera=svg.querySelector(`[data-mech-scene-id="camera-hit-${index}"]`);
      camera.setAttribute('data-camera-index',String(index));
      camera.setAttribute('role','button');
      camera.setAttribute('tabindex','0');
      camera.setAttribute('aria-pressed',String(Boolean(enabled[index-1])));
      camera.setAttribute('aria-label',`Camera ${index}: ${enabled[index-1]?'enabled, disable':'disabled, enable'} camera`);
      // Keep the Mech-defined hit target above rays/grid lines, which the
      // generic renderer emits after circles and would otherwise intercept it.
      svg.append(camera);
      svg.querySelector(`[data-mech-scene-id="camera-${index}"]`).setAttribute('data-camera-index',String(index));
      svg.querySelector(`[data-mech-scene-id="camera-label-${index}"]`).setAttribute('data-camera-index',String(index));
    }
    document.getElementById('robot-scene').replaceWith(svg);
    if(focusedCamera) svg.querySelector(`[role="button"][data-camera-index="${focusedCamera}"]`)?.focus({preventScroll:true});
    const visible=Array.from(this.program.readNumbers('camera-visible')).filter(Boolean).length;
    const enabledCount=enabled.filter(Boolean).length;
    document.getElementById('measurement-status').textContent=visible
      ? `${visible} of ${enabledCount} enabled cameras in range · range + bearing corrections`
      : `No enabled cameras in range (${enabledCount} enabled) · motion prediction only`;
  }
  dispose() {
    if(!this.program)return;
    try { this.program.stop(); }
    finally { this.program.free();this.program=null; }
  }
}
