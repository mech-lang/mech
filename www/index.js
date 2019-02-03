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

let get_ball = document.createElement("button");
get_ball.setAttribute("id", "get balls");
get_ball.innerHTML =  "Get Balls";
controls.appendChild(get_ball);

let increment_time = document.createElement("button");
increment_time.setAttribute("id", "increment time");
increment_time.innerHTML =  "Increment Time";
controls.appendChild(increment_time);

let txn = document.createElement("button");
txn.setAttribute("id", "txn");
txn.innerHTML =  "Add Txn";

// ## Editor

let editor = document.createElement("div");
editor.setAttribute("class", "editor");

let code = document.createElement("textarea");
code.setAttribute("class", "code");
code.setAttribute("id", "code");
code.innerHTML =  `# Bouncing Balls

Define the environment
  #html/event/click = [x: 0 y: 0]
  x = 1:10
  v = x * 0
  #ball = [|x y vx vy| x x v v]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = 420

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity`;

let canvas = document.createElement("canvas");
canvas.setAttribute("class", "canvas");
canvas.setAttribute("id", "drawing area");
canvas.setAttribute("width", "500");
canvas.setAttribute("height", "420");
canvas.style.backgroundColor = 'rgb(226, 79, 94)';

editor.appendChild(code);
editor.appendChild(canvas);

// ## Container

let container = document.createElement("div");
container.setAttribute("class","container");

container.appendChild(controls);
container.appendChild(editor);

document.body.appendChild(container);

// ## Event handlers

document.getElementById("compile").addEventListener("click", function(click) {
  console.log(click);
  let code = document.getElementById("code");
  mech_core.compile_code(code.value);
});

document.getElementById("drawing area").addEventListener("click", function(click) {
  console.log(click.layerX, click.layerY);
});

document.getElementById("view core").addEventListener("click", function() {
  mech_core.display_core();
});

document.getElementById("view runtime").addEventListener("click", function() {
  mech_core.display_runtime();
});

document.getElementById("clear core").addEventListener("click", function() {
  mech_core.clear();
});

document.getElementById("get balls").addEventListener("click", function() {
  let column = mech_core.get_column(balls,BigInt(1));
  console.log(column);
});

document.getElementById("increment time").addEventListener("click", function() {
  mech_core.process_transaction("system/timer",1,2,time);
  time = time + 1;

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
});

/*document.getElementById("txn").addEventListener("click", function() {
  mech_core.process_transaction("test",1,1,3);
});*/