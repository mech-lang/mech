import init, { WasmMech } from '/pkg/mech_wasm.js';

const canvas = document.querySelector('#arm-canvas');
const ctx = canvas.getContext('2d');
const controls = [
  '#x-slider',
  '#y-slider',
  '#z-slider',
  '#grip-toggle',
  '#step-button',
].map((selector) => document.querySelector(selector));

let mech = null;
let mechSource = '';

function pointFromControls() {
  return {
    x: Number(document.querySelector('#x-slider').value),
    y: Number(document.querySelector('#y-slider').value),
    z: Number(document.querySelector('#z-slider').value),
    closed: document.querySelector('#grip-toggle').checked,
  };
}

function drawArm() {
  const { x, y, z, closed } = pointFromControls();
  const base = { x: 60, y: 190 };
  const elbow = { x: base.x + x, y: base.y - y };
  const wrist = { x: elbow.x + z, y: elbow.y - z / 2 };

  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.lineWidth = 8;
  ctx.lineCap = 'round';
  ctx.strokeStyle = '#2563eb';
  ctx.beginPath();
  ctx.moveTo(base.x, base.y);
  ctx.lineTo(elbow.x, elbow.y);
  ctx.lineTo(wrist.x, wrist.y);
  ctx.stroke();

  ctx.fillStyle = closed ? '#dc2626' : '#16a34a';
  ctx.beginPath();
  ctx.arc(wrist.x, wrist.y, closed ? 8 : 12, 0, Math.PI * 2);
  ctx.fill();
}

async function loadMech() {
  await init();
  mechSource = await fetch('demo.mec').then((response) => response.text());
  mech = WasmMech.fromHostConfig();
}

async function stepMech() {
  if (mech === null) {
    await loadMech();
  }
  const gripToggle = document.querySelector('#grip-toggle');
  gripToggle.toggleAttribute('checked', gripToggle.checked);
  mech.run_program(mechSource);
  drawArm();
}

controls.forEach((control) => control.addEventListener('input', stepMech));
controls.at(-1).addEventListener('click', stepMech);
stepMech().catch((error) => console.error(error));
