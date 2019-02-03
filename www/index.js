import {Core} from "mech-wasm";

let mech_core = Core.new();

let code = document.createElement("textarea");
code.setAttribute("class", "code");
code.setAttribute("id", "code");
code.innerHTML =  "#test = [1 2 3]";

let compile = document.createElement("button");
compile.setAttribute("id", "compile");
compile.innerHTML =  "Compile";

let txn = document.createElement("button");
txn.setAttribute("id", "txn");
txn.innerHTML =  "Add Txn";

let container = document.createElement("div");
container.setAttribute("class","container");

container.appendChild(code);
container.appendChild(compile);
container.appendChild(txn);

document.body.appendChild(container);

document.getElementById("compile").addEventListener("click", function() {
  let code = document.getElementById("code");
  mech_core.compile_code(code.value);
});

document.getElementById("txn").addEventListener("click", function() {
  mech_core.process_transaction("test",1,1,3);
});