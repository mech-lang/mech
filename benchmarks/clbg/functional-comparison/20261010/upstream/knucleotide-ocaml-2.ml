(* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by Troestler Christophe
 * modified by Mauricio Fernandez
 * fixed by glacialthinker 
 *
 *)

module C(S : sig
           val k : int
           val dna : bytes
         end) =
struct
  let dna, k = S.dna, S.k

  module K = struct
    type t = int
    let equal k1 k2 =
      let rec cmp n ka kb =
        if n = 0 then true
        else if Bytes.unsafe_get dna ka = Bytes.unsafe_get dna kb then cmp (n - 1) (ka + 1) (kb + 1)
        else false
      in cmp k k1 k2

    let hash n =
      let h = ref 0 in
        for i = n to n + k - 1 do h := !h * 5 + Char.code (Bytes.unsafe_get dna i) done;
        !h
  end

  let c = 0x40000
  include Hashtbl.Make(K)
  let h = create c

  let count () = 
    for i = 0 to Bytes.length dna - k - 1 do
      try incr (find h i) with Not_found -> add h i (ref 1)
    done

  let compare_freq ((k1:bytes),(f1:float)) (k2, f2) =
    if f1 > f2 then -1 else if f1 < f2 then 1 else Bytes.compare k1 k2

  let write_frequencies () =
    count ();
    let tot = float(fold (fun _ n t -> !n + t) h 0) in
    let frq =
      fold (fun off n l -> 
              (Bytes.sub dna off k, 100. *. float !n /. tot) :: l) h [] in
    let frq = List.sort compare_freq frq in
      String.concat "" 
        (List.map (fun (k,f) -> (Printf.sprintf "%s %.3f\n" (Bytes.unsafe_to_string k) f)) frq)

  let write_count seq =
    assert (String.length seq = k);
    count ();
    String.blit seq 0 dna 0 k;
    Printf.sprintf "%d\t%s" (try !(find h 0) with Not_found -> 0) seq
end

(* Extract DNA sequence "THREE" from stdin *)
let dna_three =
  let is_not_three s = try String.sub s 0 6 <> ">THREE" with _ -> true in
  while is_not_three(input_line stdin) do () done;
  let buf = Buffer.create 1000 in
  (* Skip possible comment *)
  (try while true do
     let line = input_line stdin in
     if line.[0] <> ';' then
       (Buffer.add_string buf (String.uppercase_ascii line); raise Exit)
   done with _ -> ());
  (* Read the DNA sequence *)
  (try while true do
       let line = input_line stdin in
       if line.[0] = '>' then raise End_of_file;
       Buffer.add_string buf (String.uppercase_ascii line)
   done with End_of_file -> ());
  Buffer.to_bytes buf

let invoke (f : 'a -> 'b) x : unit -> 'b =
  let input, output = Unix.pipe() in
  match Unix.fork() with
  | -1 -> Unix.close input; Unix.close output; (let v = f x in fun () -> v)
  | 0 ->
      Unix.close input;
      let output = Unix.out_channel_of_descr output in
        Marshal.to_channel output (try `Res(f x) with e -> `Exn e) [];
        close_out output;
        exit 0
  | pid ->
      Unix.close output;
      let input = Unix.in_channel_of_descr input in fun () ->
        let v = Marshal.from_channel input in
        ignore (Unix.waitpid [] pid);
        close_in input;
        match v with `Res x -> x | `Exn e -> raise e

let parallelize f l =
  List.iter (fun g -> print_endline (g ())) (List.map (invoke f) l)

let () =
  parallelize
    (fun i -> 
       let module M = C(struct let k = i let dna = dna_three end) in
         M.write_frequencies ()) [1; 2];
  parallelize
    (fun k -> 
       let module M = C(struct let k = String.length k let dna = dna_three end) in
         M.write_count k)
    ["GGT"; "GGTA"; "GGTATT"; "GGTATTTTAATT"; "GGTATTTTAATTTATAGT"]
