import init, {
  WasmAnalogClock,
} from "./pkg/mech_wasm.js";

let clock;
let running = true;

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
    if (!running) {
      return;
    }

    try {
      clock.pumpAndRender();
    } catch (error) {
      running = false;
      console.error(error);
      return;
    }

    requestAnimationFrame(frame);
  }

  requestAnimationFrame(frame);
}

window.addEventListener('beforeunload', () => {
  running = false;
  if (clock) {
    clock.stop();
  }
});

main().catch((error) => {
  running = false;
  console.error(error);
});
