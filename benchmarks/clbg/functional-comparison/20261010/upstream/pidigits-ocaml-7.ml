(*
 * The Computer Language Benchmarks Game 
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by Gabriel Scherer with GMP advice from Guillaume Melquiond
 *)

let extract_digit =
  let tmp1 : Mpz.m Mpz.tt = Mpz.init () in
  let tmp2 : Mpz.m Mpz.tt = Mpz.init () in
  fun (num,den,acc) nth ->
   (* joggling between tmp1 and tmp2,
      so that GMP does not use temporary buffers *)
    Mpz.mul_si tmp1 num nth;
    Mpz.add tmp2 acc tmp1;
    Mpz.tdiv_q tmp1 tmp2 den;
    Mpz.get_int tmp1 (* (nth * num + acc) / den |> to_int *)


let eliminate_digit (num, den, acc) d =
  Mpz.mul_si num num 10;   (* num <- num * 10 *)
  Mpz.submul_ui acc den d; 
  Mpz.mul_si acc acc 10 (* acc <- 10 * (acc - den * d) *)

let next_term (num, den, acc) k =
  let k2 = k * 2 + 1 in
  Mpz.mul_si den den k2; (* den <- den * k2 *)
  Mpz.addmul_ui acc num 2;
  Mpz.mul_si acc acc k2; (* acc <- (acc + 2*num) * k2 *)
  Mpz.mul_si num num k (* num <- num*k *)

let rec terms z k =
  fun () -> 
  next_term z k;
  Seq.Cons(z, terms z (k + 1))

let digit z =
  let d = extract_digit z 3 in
  if extract_digit z 4 = d
  then (eliminate_digit z d; Some d)
  else None

let all_digits =
  terms (Mpz.of_int 1, Mpz.of_int 1, Mpz.of_int 0) 1
  |> Seq.filter (fun (num, _den, acc) -> Mpz.cmp num acc <= 0)
  |> Seq.filter_map digit

let columns = 10
let rec print_digits pos n digits =
  if pos mod columns = 0 && pos > 0 then Printf.printf "\t:%i\n" pos;
  if pos = n then ()
  else match digits () with
  | Seq.Nil ->
    if pos mod columns > 0 then begin
      print_char ' ';
      print_digits (pos + 1) n digits
    end
  | Seq.Cons (d, rest) ->
    print_int d;
    print_digits (pos + 1) n rest

let () =
  let n = try int_of_string (Array.get Sys.argv 1) with _ -> 27 in  
  print_digits 0 n all_digits
