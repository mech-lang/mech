import Matchfinder
import BtMatchfinder

set16 = |xs, at, value| match List.set(xs, at, value) {
    Ok(next) => next
    Err(_) => crash "oracle table write"
}
set32 = |xs, at, value| match List.set(xs, at, value) {
    Ok(next) => next
    Err(_) => crash "oracle cache write"
}

collect6 = |_| {
    input = List.repeat(97.U8, 320)
    var $mf = Matchfinder.init_nodes(262144)
    var $cache = List.repeat(0.U32, 1500513)
    var $hash3 = 0.U64
    var $hash4 = 0.U64
    var $at = 0.U64
    var $ptr = 0.U64
    var $skip = 0.U64
    while $at < 6 {
        before = $ptr
        if $skip != 0 {
            sk = BtMatchfinder.skip_byte($mf, $hash3, $hash4, input, 0, $at, 258, 300)
            $mf = sk.tables
            $hash3 = sk.next_hash3
            $hash4 = sk.next_hash4
            $skip = $skip - 1
        } else {
            adv = BtMatchfinder.get_matches($mf, $hash3, $hash4, input, 0, $at, 258, 258, 300, $cache, $ptr)
            $mf = adv.tables
            $hash3 = adv.next_hash3
            $hash4 = adv.next_hash4
            $cache = adv.cache
            $ptr = adv.cache_ptr
            if $ptr > before {
                best = (List.get($cache, $ptr - 1) ?? 0).bitwise_and(0xFFFF).to_u64()
                if best >= 258 { $skip = best - 1 } else {}
            } else {}
        }
        $cache = set32($cache, $ptr, BtMatchfinder.pack_cache_entry(($ptr - before).to_u32_wrap(), 97))
        $ptr = $ptr + 1
        $at = $at + 1
    }
    { tables: $mf, cache: $cache, hash3: $hash3, hash4: $hash4, ptr: $ptr, skip: $skip }
}

expected_table = |_| {
    var $xs = List.repeat(0.U16, 262144)
    $xs = set16($xs, 65536, 32768)
    $xs = set16($xs, 131072, 32768)
    $xs = set16($xs, 127616, 32773)
    $xs = set16($xs, 174848, 32773)
    $xs = set16($xs, 174849, 32772)
    $xs
}
expected_cache = |_| {
    headers : List(U32)
    headers = [6356992, 6356992, 65539, 65794, 6356994, 6356992, 6356992, 6356992]
    var $xs = List.repeat(0.U32, 1500513)
    var $i = 0.U64
    while $i < 8 {
        $xs = set32($xs, $i, List.get(headers, $i) ?? 0)
        $i = $i + 1
    }
    $xs
}
expect {
    actual = collect6(0)
    actual.tables == expected_table(0) and actual.cache == expected_cache(0)
        and actual.hash3 == 21888 and actual.hash4 == 62080
        and actual.ptr == 8 and actual.skip == 254
}
