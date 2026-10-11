import "../stdlib/bigint"
import "../stdlib/hashmap"
import "../stdlib/regex"
entry arithmetic (negative_a:bool) (a:[]u32) (negative_b:bool) (b:[]u32) (factor:u32) =
  let a = normalise negative_a a
  let b = normalise negative_b b
  in (add a b,subtract a b,multiply_small a factor,quotient a b,compare a b)
entry regex_test (pattern:[]u8) (text:[]u8) (replacement:[]u8) =
  let rx = compile pattern
  in (count rx text,replace rx text replacement)
entry hashmap_test (keys:[]u64) =
  let table = loop table = empty 2 for key in keys do
    let value = get table key
    in put table key (value+1)
  in map (get table) keys
