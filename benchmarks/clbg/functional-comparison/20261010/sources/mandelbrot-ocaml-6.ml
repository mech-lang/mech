(*
 * The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * Contributed by Paolo Ribeca
 *
 * (Very loosely based on previous version Ocaml #3,
 *  which had been contributed by
 *   Christophe TROESTLER
 *  and enhanced by
 *   Christian Szegedy and Yaron Minsky)
 *)

let niter = 50
let limit = 4.

let () =
  let w = int_of_string (Array.get Sys.argv 1) in
  let h = w in
  let fw = float w /. 2. and fh = float h /. 2. in
  Printf.printf "P4\n%i %i\n" w h;
  let red_h = h - 1 and red_w = w - 1 and byte = ref 0 in
  for y = 0 to red_h do
    let ci = float y /. fh -. 1. in
    for x = 0 to red_w do
      let cr = float x /. fw -. 1.5
      and zr = ref 0. and zi = ref 0. and trmti = ref 0. and n = ref 0 in
      begin try
	while true do
	  zi := 2. *. !zr *. !zi +. ci;
	  zr := !trmti +. cr;
	  let tr = !zr *. !zr and ti = !zi *. !zi in
	  if tr +. ti > limit then begin
	    byte := !byte lsl 1;
	    raise Exit
	  end else if incr n; !n = niter then begin
	    byte := (!byte lsl 1) lor 0x01;
	    raise Exit
	  end else
	    trmti := tr -. ti
	done
      with Exit -> ()
      end;
      if x mod 8 = 7 then output_byte stdout !byte
    done;
    let rem = w mod 8 in
    if rem != 0 then (* the row doesnt divide evenly by 8 *)
      output_byte stdout (!byte lsl (8 - rem)) (* output last few bits *)
  done
