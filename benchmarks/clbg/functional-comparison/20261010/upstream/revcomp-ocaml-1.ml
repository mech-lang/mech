(* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * Contributed by Paolo Ribeca
 * Modified by Roman Kashitsyn: use Bytes instead of String
 *)

let chars_per_line = 60
and lines_per_worker =
  match Sys.word_size with
  | 32 -> 200000
  | 64 -> 500000
  | _ -> assert false

let rc_table =
  let complements = [ ('a', 't'); ('c', 'g'); ('m', 'k'); ('r', 'y'); ('w', 'w');
                      ('s', 's'); ('v', 'b'); ('h', 'd'); ('n', 'n') ] in
  let buf = Bytes.make 256 '\000' in
  let set (i, c) = Bytes.set buf (Char.code i) (Char.uppercase_ascii c) in
  let set_case (i, c) = set (i, c); set (Char.uppercase_ascii i, c) in
  let set_bidirect (i, c) = set_case (i, c); set_case (c, i) in
  set_case ('u', 'A');
  List.iter set_bidirect complements;
  Bytes.unsafe_to_string buf

let _ =
  let aug_chars_per_line = chars_per_line + 1
  and in_ack, out_ack = Unix.pipe () and in_end, out_end = Unix.pipe ()
  and put out_pipe () =
    if Unix.write out_pipe (Bytes.of_string " ") 0 1 <> 1 then
      failwith "Pipe problem"
  and get in_pipe () =
    let res = (Bytes.of_string " ") in
    if Unix.read in_pipe res 0 1 <> 1 then
      failwith "Pipe problem" in
  let put_ack = put out_ack and get_ack = get in_ack
  and put_end_ack = put out_end and get_end_ack = get in_end in
  let rec spawn tag beg first =
    let output_tag () =
      print_string tag;
      print_char '\n';
      flush stdout
    and buf = Bytes.create (lines_per_worker * chars_per_line + 2)
    and len = ref (String.length beg) in
    String.blit beg 0 buf 0 !len;
    let process_buffer () =
      let red_len = !len - 1 in
      let mid_point = red_len / 2 in
      for i = 0 to mid_point do
        let ri = red_len - i and tmp = Bytes.get buf i in
        Bytes.set buf i  rc_table.[Char.code (Bytes.get buf ri)];
        Bytes.set buf ri rc_table.[Char.code tmp]
      done
    and write_by_cols rem eol =
      let len = !len and dne = ref 0 in
      if rem > 0 then begin
        let to_do = min rem (len - !dne) in
        output stdout buf !dne to_do;
        output_char stdout '\n';
        dne := !dne + to_do
      end;
      while len - !dne >= chars_per_line do
        output stdout buf !dne chars_per_line;
        output_char stdout '\n';
        dne := !dne + chars_per_line
      done;
      let rem = len - !dne in
      if rem > 0 then begin
        output stdout buf !dne rem;
        if eol then
          output_char stdout '\n'
      end;
      flush stdout;
      if eol then
        0
      else
        rem in
    try
      for i = 2 to lines_per_worker do
        really_input stdin buf !len aug_chars_per_line;
        let new_len = ref (!len + chars_per_line) in
        if Bytes.get buf !len = '>' || Bytes.get buf !new_len <> '\n' then begin
          while Bytes.get buf !len <> '>' do
            incr len
          done;
          let ptr = ref !len in
          (* Needed to patch the hideous bug in the output of the C program *)
          if Bytes.get buf (!len - 1) <> '\n' then begin
            Bytes.blit buf !len buf (!len + 1) aug_chars_per_line;
            Bytes.set buf !len '\n';
            incr new_len;
            incr ptr
          end else
            decr len;
          while !ptr < !new_len && Bytes.get buf !ptr <> '\n' do
            incr ptr
          done;
          match Unix.fork () with
          | 0 ->
            let aug_len = !len + 1 in
            if !ptr = !new_len then
              spawn
                (Bytes.unsafe_to_string (Bytes.sub buf aug_len (!new_len - aug_len))
                 ^ input_line stdin)
                "" true
            else
              let aug_ptr = !ptr + 1 in
              spawn
                (Bytes.unsafe_to_string (Bytes.sub buf aug_len (!ptr - aug_len)))
                ((Bytes.unsafe_to_string (Bytes.sub buf aug_ptr (!new_len - !ptr)) ^ input_line stdin))
                true
          | _ ->
            get_ack ();
            output_tag ();
            process_buffer ();
            let rem = write_by_cols 0 first in
            if first then
              put_ack ();
            exit rem
        end;
        len := !new_len
      done;
      match Unix.fork () with
      | 0 -> spawn tag "" false
      | pid ->
        process_buffer ();
        match Unix.waitpid [] pid with
        | _, Unix.WEXITED rem ->
          let rem = write_by_cols (chars_per_line - rem) first in
          if first then
            put_ack ();
          exit rem
        | _ -> assert false
    with End_of_file ->
      while Bytes.get buf !len <> '\n' do
        incr len
      done;
      get_ack ();
      put_end_ack ();
      output_tag ();
      process_buffer ();
      let rem = write_by_cols 0 first in
      if first then
        put_ack ();
      exit rem in
  match Unix.fork () with
  | 0 ->
    put_ack ();
    spawn (read_line ()) "" true
  | _ ->
    get_end_ack ();
    get_ack ();
    exit 0
