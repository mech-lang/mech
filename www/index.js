import * as mech from "wasmtest";

let code = document.createElement("textarea");
code.setAttribute("class", "code");
code.setAttribute("id", "code");
code.innerHTML =  "# Program";

let compile = document.createElement("button");
compile.setAttribute("id", "compile");
compile.innerHTML =  "Compile";

let container = document.createElement("div");
container.setAttribute("class","container");

container.appendChild(code);
container.appendChild(compile);

document.body.appendChild(container);



document.getElementById("compile").addEventListener("click", function() {
  let code = document.getElementById("code");
  mech.compile(code.value);
});