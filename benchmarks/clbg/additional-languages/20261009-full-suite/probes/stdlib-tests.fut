import "../stdlib/bigint"
import "../stdlib/hashmap"
import "../stdlib/regex"
import "../stdlib/common"
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
entry hashmap_increment (keys:[]u64) (deltas:[]i64) =
  let table = loop table = empty 2 for i < length keys do increment table keys[i] deltas[i]
  in map (get table) keys
entry hashmap_reserved (keys:[]u64) (deltas:[]i64) (batch:i64) =
  let table = loop (table,next) = (empty 2,0i64) while next < length keys do
    let stop = i64.min (length keys) (next+batch)
    let (slots,size) = reserve table (stop-next)
    let (slots,size) = loop (slots,size) = (slots,size) for i in next..<stop do
      let (slots,added) = increment_slots slots keys[i] deltas[i]
      in (slots,size+added)
    in ((slots,size),stop)
  in map (get table.0) keys
entry wrap_test (width:i64) (text:[]u8) = wrap width text
