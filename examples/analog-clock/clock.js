import init, { WasmAnalogClock } from '../../pkg/mech_wasm.js';

let clock;

async function main() {
  await init();
  clock = new WasmAnalogClock(
    '#clock-hour-hand',
    '#clock-minute-hand',
    '#clock-second-hand',
    100,
    100,
    100,
  );
  clock.start();

  function frame() {
    clock.pumpAndRender();
    requestAnimationFrame(frame);
  }

  requestAnimationFrame(frame);
}

window.addEventListener('beforeunload', () => {
  if (clock) {
    clock.stop();
  }
});

main().catch((error) => {
  console.error(error);
});
