import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('source/scene.mec',import.meta.url),'utf8');
const bridge=readFileSync(new URL('drawing.mjs',import.meta.url),'utf8');
assert(source.includes('@scene/replace <- field-presentation'),'Mech publishes its scene tables');
for(const name of ['scene-circles','scene-lines','scene-line-strips','scene-text'])
  assert(source.includes(name),'scene table '+name);
assert(source.includes('0xad9159'),'muted gold trail');
assert(source.includes('0x687780'),'visible dark-gray heading');
assert(source.includes('stroke-dasharray'),'dotted trail lives in Mech source');
assert(source.includes('measurement-visible')&&source.includes('@input/camera-range'),'Mech controls sensor availability');
assert(bridge.includes('WasmSceneProgram.fromSource'),'compile and activate Mech scene');
assert(bridge.includes('MechDocumentController?.renderSceneSvg'),'use shared scene renderer');
assert(!/Math\.(sin|cos|atan2|sqrt|hypot)|createElementNS|replaceChildren/.test(bridge),'no duplicate JavaScript sensor or drawing implementation');
console.log('PASS: Mech scene tables, camera availability, styling, and generic browser bridge.');
