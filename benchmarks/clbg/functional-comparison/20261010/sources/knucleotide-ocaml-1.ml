(* 
 * The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by Troestler Christophe
 * modified by Mauricio Fernandez
 * fixed by glacialthinker 
 *
 *)

module S = struct
  type t = string

  let size = 0x40000

  let equal (s1:string) s2 = (s1 = s2)

  let hash s =
    let h = ref 0 in
    for i = 0 to String.length s - 1 do h := !h * 5 + Char.code s.[i] done;
    !h
end

module H = Hashtbl.Make(S)

(* [counts k dna] fills and return the hashtable [count] of
   k-nucleotide keys and count values for a particular reading-frame
   of length [k] of the string [dna].  Keys point to mutable values
   for speed (to avoid looking twice the same key to reinsert the
   value). *)
let count = H.create S.size
let counts k dna =
  H.clear count;
  let key = Bytes.create k in
    for i = 0 to String.length dna - k do
      String.unsafe_blit dna i key 0 k;
      try incr(H.find count (Bytes.unsafe_to_string key)) with Not_found ->
        H.add count (Bytes.to_string key) (ref 1)
    done;
    count

(* [write_frequencies k dna] writes the frequencies for a
   reading-frame of length [k] sorted by descending frequency and then
   ascending k-nucleotide key. *)
let compare_freq ((k1:string),(f1:float)) (k2, f2) =
  if f1 > f2 then -1 else if f1 < f2 then 1 else compare k1 k2

let write_frequencies k dna =
  let cnt = counts k dna in
  let tot = float(H.fold (fun _ n t -> !n + t) cnt 0) in
  let frq = H.fold (fun k n l -> (k, 100. *. float !n /. tot) :: l) cnt [] in
  let frq = List.sort compare_freq frq in
  List.iter (fun (k,f) -> Printf.printf "%s %.3f\n" k f) frq;
  print_string "\n"

let write_count seq dna =
  let cnt = counts (String.length seq) dna in
  Printf.printf "%d\t%s\n" (try !(H.find cnt seq) with Not_found -> 0) seq

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
  Buffer.contents buf

let () = Gc.set { (Gc.get()) with Gc.minor_heap_size = 1024 * 2048 }

let () =
  List.iter (fun i -> write_frequencies i dna_three) [1; 2];
  List.iter (fun k -> write_count k dna_three)
    ["GGT"; "GGTA"; "GGTATT"; "GGTATTTTAATT"; "GGTATTTTAATTTATAGT"]
