import {Core, hash_string} from "mech-wasm";

let mech_core = Core.new();

let balls = hash_string("ball");
let time = 1;

// ## Controls

let controls = document.createElement("div");
controls.setAttribute("class", "controls");

let compile = document.createElement("button");
compile.setAttribute("id", "compile");
compile.innerHTML =  "Compile";
controls.appendChild(compile);

let view_core = document.createElement("button");
view_core.setAttribute("id", "view core");
view_core.innerHTML =  "View Core";
controls.appendChild(view_core);

let view_runtime = document.createElement("button");
view_runtime.setAttribute("id", "view runtime");
view_runtime.innerHTML =  "View Runtime";
controls.appendChild(view_runtime);

let clear_core = document.createElement("button");
clear_core.setAttribute("id", "clear core");
clear_core.innerHTML =  "Clear Core";
controls.appendChild(clear_core);

let start_timer = document.createElement("button");
start_timer.setAttribute("id", "start timer");
start_timer.innerHTML =  "Start Timer";
controls.appendChild(start_timer);

let txn = document.createElement("button");
txn.setAttribute("id", "txn");
txn.innerHTML =  "Add Txn";

// ## Time Travel

let time_travel = document.createElement("div");
time_travel.setAttribute("class", "controls");

let time_slider = document.createElement("input");
time_slider.setAttribute("id", "time slider");
time_slider.setAttribute("min", "1");
time_slider.setAttribute("max", "100");
time_slider.setAttribute("value", "100");
time_slider.setAttribute("type", "range");
time_travel.appendChild(time_slider);

let last_slider_value = 100;
time_slider.oninput = function() {
  mech_core.pause();
  let current_value = this.value;
  // Time travel forward
  if (current_value > last_slider_value) {
    mech_core.step_forward_one();
  // Time travel backward
  } else if (current_value < last_slider_value) {
    mech_core.step_back_one();
  }
  last_slider_value = current_value;
}

let step_back = document.createElement("button");
step_back.setAttribute("id", "step back");
step_back.innerHTML =  "<";
time_travel.appendChild(step_back);

let toggle_core = document.createElement("button");
toggle_core.setAttribute("id", "toggle core");
toggle_core.innerHTML =  "Pause";
time_travel.appendChild(toggle_core);

let step_forward = document.createElement("button");
step_forward.setAttribute("id", "step forward");
step_forward.innerHTML =  ">";
time_travel.appendChild(step_forward);

// ## Editor

let editor = document.createElement("div");
editor.setAttribute("class", "editor");

let code = document.createElement("textarea");
code.setAttribute("class", "code");
code.setAttribute("id", "code");
code.innerHTML =  `# Bouncing Balls

Define the environment
  #html/event/click = [|x y|]
  range = 1:5
  x = range * 30
  v = x * 0
  #ball = [|x y vx vy| x x v v]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 1
  #boundary-y = 820
  #boundary-x = 500

## Update condition

Update the block positions on each tick of the timer
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity

## Boundary Condition

Keep the balls within the y boundary
  ~ #system/timer.tick
  iy = #ball.y > #boundary-y
  #ball.y{iy} := #boundary-y
  #ball.vy{iy} := -#ball.vy * 0.80

Keep the balls within the x boundary
  ~ #system/timer.tick
  ix = #ball.x > #boundary-x
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary-x
  #ball.x{ixx} := 0
  #ball.vx{ix | ixx} := -#ball.vx * 0.80

## Create More Balls

Create ball at click point
  ~ #html/event/click.x
  x = #html/event/click.x
  y = #html/event/click.y
  #ball += [x: x, y: y, vx: 30, vy: 0]`;

let canvas = document.createElement("canvas");
canvas.setAttribute("class", "canvas");
canvas.setAttribute("id", "drawing area");
canvas.setAttribute("width", "500");
canvas.setAttribute("height", "820");
canvas.style.backgroundColor = 'rgb(226, 79, 94)';

editor.appendChild(code);
editor.appendChild(canvas);

// ## Container

let container = document.createElement("div");
container.setAttribute("class","container");

container.appendChild(controls);
container.appendChild(editor);
container.appendChild(time_travel);

document.body.appendChild(container);

// ## Event handlers

function system_timer() {
  mech_core.queue_change("system/timer",1,2,time);
  mech_core.process_transaction();
  time = time + 1;
  render();
}

function render() {
  //render
  let canvas = document.getElementById("drawing area");
  let context = canvas.getContext("2d");
  context.clearRect(0, 0, canvas.width, canvas.height);

  let radius = 10;
  let x = mech_core.get_column(balls,BigInt(1));
  let y = mech_core.get_column(balls,BigInt(2));

  let i;
  for (i = 0; i < x.length; i++) {
    let centerY = Number(y[i]);
    let centerX = Number(x[i]);
    context.beginPath();
    context.arc(centerX, centerY, radius, 0, 2 * Math.PI, false);
    context.fillStyle = 'black';
    context.fill();
    context.lineWidth = 1;
    context.strokeStyle = '#000000';
    context.stroke();
  }
}

document.getElementById("compile").addEventListener("click", function(click) {
  console.log(click);
  let code = document.getElementById("code");
  mech_core.compile_code(code.value);
  render();
});

document.getElementById("drawing area").addEventListener("click", function(click) {
  console.log(click.layerX, click.layerY);
  mech_core.queue_change("html/event/click",1,1,click.layerX);
  mech_core.queue_change("html/event/click",1,2,click.layerY);
  mech_core.process_transaction();
  render();
});

document.getElementById("view core").addEventListener("click", function() {
  mech_core.display_core();
});

document.getElementById("view runtime").addEventListener("click", function() {
  mech_core.display_runtime();
});

document.getElementById("clear core").addEventListener("click", function() {
  mech_core.clear();
  render();
});

document.getElementById("step back").addEventListener("click", function() {
  mech_core.step_back_one();
  render();
});

document.getElementById("step forward").addEventListener("click", function() {
  mech_core.step_forward_one();
  render();
});

document.getElementById("toggle core").addEventListener("click", function() {
  let toggle_core = document.getElementById("toggle core");
  let state = toggle_core.innerHTML;
  if (state == "Resume") {
    mech_core.resume();
    toggle_core.innerHTML = "Pause";
  } else {
    mech_core.pause();
    toggle_core.innerHTML = "Resume";
  }
});

document.getElementById("start timer").addEventListener("click", function() {
  setInterval(system_timer, 16);
});

/*document.getElementById("txn").addEventListener("click", function() {
  mech_core.process_transaction("test",1,1,3);
});*/