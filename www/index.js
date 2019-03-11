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
time_travel.setAttribute("class", "time-travel");

let time_slider = document.createElement("input");
time_slider.setAttribute("id", "time slider");
time_slider.setAttribute("class", "slider");
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
step_back.onclick = function() {
  mech_core.step_back_one();
  time_slider.value = time_slider.value - 1;
  render();
}
time_travel.appendChild(step_back);

let toggle_core = document.createElement("button");
toggle_core.setAttribute("id", "toggle core");
toggle_core.innerHTML =  "Pause";
toggle_core.onclick = function() {
  let toggle_core = document.getElementById("toggle core");
  let state = toggle_core.innerHTML;
  if (state == "Resume") {
    mech_core.resume();
    toggle_core.innerHTML = "Pause";
    time_slider.value = time_slider.max;
  } else {
    mech_core.pause();
    toggle_core.innerHTML = "Resume";
  }
};
time_travel.appendChild(toggle_core);

let step_forward = document.createElement("button");
step_forward.setAttribute("id", "step forward");
step_forward.innerHTML =  ">";
step_forward.onclick = function() {
  mech_core.step_forward_one();
  time_slider.value = time_slider.value*1 + 1;
  render();
}
time_travel.appendChild(step_forward);

// ## Editor

let editor = document.createElement("div");
editor.setAttribute("class", "editor");

let code = document.createElement("textarea");
code.setAttribute("class", "code");
code.setAttribute("id", "code");
code.setAttribute("spellcheck", "false");
code.innerHTML =  `# Editor

This program defines the online editor for Mech.

block
  container = [|type text| 
                "h1"   "A Mech Webpage"
                "div"  "Hello World"]
  #app/main = [|direction contains| 
               "column"  [container]]`;

let drawing_area = document.createElement("div")
drawing_area.setAttribute("id", "drawing");
drawing_area.setAttribute("class", "drawing-area");

editor.appendChild(drawing_area)

// ## Editor Container

let editor_container = document.createElement("div");
editor_container.setAttribute("id","editor container");
editor_container.setAttribute("class","editor-container");

editor_container.appendChild(controls);
editor_container.appendChild(editor);
editor_container.appendChild(time_travel);

// ## Navigation

let nav = document.createElement("div");
nav.setAttribute("id","navigation");
nav.setAttribute("class","navigation");


// ## Bring it all together

let app = document.createElement("div");
app.setAttribute("id","app");
app.setAttribute("class","app");
app.appendChild(nav);
app.appendChild(code);
app.appendChild(editor_container);

document.body.appendChild(app);

// ## Event handlers
function system_timer() {
  var d = new Date();
  mech_core.queue_change("system/timer",1,2,time);
  mech_core.queue_change("system/timer",1,3,d.getHours() - 12);
  mech_core.queue_change("system/timer",1,4,d.getMinutes());
  mech_core.queue_change("system/timer",1,5,d.getSeconds());
  mech_core.process_transaction();
  time = time + 1;
  render();
}

function render() {
  mech_core.render_balls();
}

document.getElementById("compile").addEventListener("click", function(click) {
  console.log(click);
  let code = document.getElementById("code");
  mech_core.compile_code(code.value);
  mech_core.add_application();
  /*
  mech_core.add_canvas();
  document.getElementById("drawing canvas").addEventListener("click", function(click) {
    console.log(click.layerX, click.layerY);
    mech_core.queue_change("html/event/click",1,1,click.layerX);
    mech_core.queue_change("html/event/click",1,2,click.layerY);
    mech_core.process_transaction();
    render();
  });*/
  //render();
});

document.getElementById("view core").addEventListener("click", function() {
  mech_core.display_core();
  mech_core.list_global_tables();
});

document.getElementById("view runtime").addEventListener("click", function() {
  mech_core.display_runtime();
});

document.getElementById("clear core").addEventListener("click", function() {
  mech_core.clear();
  render();
});

document.getElementById("start timer").addEventListener("click", function() {
  let timer_hash = hash_string("system/timer");
  let column = mech_core.get_column(timer_hash,BigInt(1));
  setInterval(system_timer, column[0]);
});

/*document.getElementById("txn").addEventListener("click", function() {
  mech_core.process_transaction("test",1,1,3);
});*/