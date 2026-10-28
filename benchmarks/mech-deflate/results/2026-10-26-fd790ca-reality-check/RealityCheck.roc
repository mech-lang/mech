app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!, now!]
import Matchfinder
import BtMatchfinder
set16 = |xs, at, value| List.set(xs, at, value) ?? crash "table write"
set32 = |xs, at, value| List.set(xs, at, value) ?? crash "cache write"
collect = |input, end, table_size, cache_size| {
    var $mf = Matchfinder.init_nodes(table_size)
    var $cache = List.repeat(0.U32, cache_size)
    var $hash3 = 0.U64
    var $hash4 = 0.U64
    var $at = 0.U64
    var $ptr = 0.U64
    var $skip = 0.U64
    var $base = 0.U64
    var $slide_at = if input.len() < 32768 {input.len()} else {32768.U64}
    var $maximum = 258.U64
    var $nice = 258.U64
    target = if end < input.len() {end} else {input.len()}
    while $at < target {
        remaining = input.len().minus_wrap($at)
        if $at == $slide_at {
            $mf = Matchfinder.rebase_nodes($mf)
            $base = $at
            amount = if remaining < 32768 {remaining} else {32768.U64}
            $slide_at = $at.plus_wrap(amount)
        } else {}
        if remaining < 258 {
            $maximum = remaining
            $nice = if $nice < $maximum {$nice} else {$maximum}
        } else {}
        before = $ptr
        skipping = $skip != 0
        if $maximum >= 5 {
            if skipping {
                sk = BtMatchfinder.skip_byte($mf, $hash3, $hash4, input, $base, $at.minus_wrap($base), $nice, 300)
                $mf = sk.tables
                $hash3 = sk.next_hash3
                $hash4 = sk.next_hash4
            } else {
                adv = BtMatchfinder.get_matches($mf, $hash3, $hash4, input, $base, $at.minus_wrap($base), $maximum, $nice, 300, $cache, $ptr)
                $mf = adv.tables
                $hash3 = adv.next_hash3
                $hash4 = adv.next_hash4
                $cache = adv.cache
                $ptr = adv.cache_ptr
            }
        } else {}
        best = if $ptr > before {(List.get($cache, $ptr.minus_wrap(1)) ?? 0).bitwise_and(0xFFFF).to_u64()} else {0.U64}
        literal = (List.get(input, $at) ?? 0).to_u32()
        $cache = set32($cache, $ptr, BtMatchfinder.pack_cache_entry($ptr.minus_wrap(before).to_u32_wrap(), literal))
        if skipping {
            $skip = $skip.minus_wrap(1)
        } else {
            $skip = if best >= 3 and best >= $nice {best.minus_wrap(1)} else {0.U64}
        }
        $ptr = $ptr.plus_wrap(1)
        $at = $at.plus_wrap(1)
    }
    { tables: $mf, cache: $cache, next_hash3: $hash3, next_hash4: $hash4, cache_ptr: $ptr, skip: $skip, next_position: $at, base: $base, next_slide: $slide_at, maximum: $maximum, nice: $nice }
}

check_zeros16 = |xs| {
    var $i=0.U64
    while $i < xs.len() {
        if (List.get(xs,$i) ?? 1) != 0 { crash "table zero" } else {}
        $i=$i+1
    }
}
check_zeros32 = |xs| {
    var $i=0.U64
    while $i < xs.len() {
        if (List.get(xs,$i) ?? 1) != 0 { crash "cache zero" } else {}
        $i=$i+1
    }
}
check_after = |xs| {
    var $i=0.U64
    while $i < xs.len() {
        want = if $i == 127616 or $i == 174848 {32769.U16} else if $i == 174849 {32768.U16} else {0.U16}
        if (List.get(xs,$i) ?? 1) != want { crash "transition table" } else {}
        $i=$i+1
    }
}
check_before = |xs| {
    var $i=0.U64
    while $i < xs.len() {
        want = if $i == 127616 or $i == 174848 {32768.U16} else {0.U16}
        if (List.get(xs,$i) ?? 1) != want { crash "input table changed" } else {}
        $i=$i+1
    }
}
check_search_cache = |xs| {
    var $i=0.U64
    while $i < xs.len() {
        want = if $i == 3 {65539.U32} else if $i == 4 {65794.U32} else {0.U32}
        if (List.get(xs,$i) ?? 1) != want { crash "transition cache" } else {}
        $i=$i+1
    }
}
check_six = |state| {
    var $i=0.U64
    while $i < state.tables.len() {
        want = if $i == 65536 or $i == 131072 {32768.U16} else if $i == 127616 or $i == 174848 {32773.U16} else if $i == 174849 {32772.U16} else {0.U16}
        if (List.get(state.tables,$i) ?? 1) != want { crash "six table" } else {}
        $i=$i+1
    }
    var $j=0.U64
    headers : List(U32)
    headers = [6356992,6356992,65539,65794,6356994,6356992,6356992,6356992]
    while $j < state.cache.len() {
        want = List.get(headers,$j) ?? 0
        if (List.get(state.cache,$j) ?? 1) != want { crash "six cache" } else {}
        $j=$j+1
    }
    if state.next_hash3 != 21888 or state.next_hash4 != 62080 or state.cache_ptr != 8 or state.skip != 254 or state.next_position != 6 { crash "six scalar" } else {}
}
main! = |mode| {
    if mode > 3 { crash "mode 0 init, 1 search, 2 skip, 3 six" } else {}
    # Data/counts arrive through a runtime-dependent branch. No CTFE run.
    seed = if mode < 4 {97.U8} else {98.U8}
    table_size = if mode < 4 {262144.U64} else {262145.U64}
    cache_size = if mode < 4 {1500513.U64} else {1500514.U64}
    input=List.repeat(seed,320)
    var $rep=0.U64
    while $rep < 8 {
        if mode == 0 or mode == 3 {
            end = if mode == 3 {6.U64} else {0.U64}
            start=now!()
            result=collect(input,end,table_size,cache_size)
            elapsed=now!() - start
            if mode == 0 {
                check_zeros16(result.tables)
                check_zeros32(result.cache)
                if result.next_position != 0 or result.cache_ptr != 0 {crash "init scalars"} else {}
            } else {check_six(result)}
            echo!("sample\t${mode.to_str()}\t${$rep.to_str()}\t${elapsed.to_str()}")
        } else {
            before=set16(set16(Matchfinder.init_nodes(table_size),127616,32768),174848,32768)
            if mode == 1 {
                cache=List.repeat(0.U32,cache_size)
                start=now!()
                result=BtMatchfinder.get_matches(before,21888,62080,input,0,1,258,258,300,cache,3)
                elapsed=now!() - start
                check_after(result.tables)
                check_search_cache(result.cache)
                check_before(before)
                check_zeros32(cache)
                if result.next_hash3 != 21888 or result.next_hash4 != 62080 or result.cache_ptr != 5 {crash "search scalar"} else {}
                echo!("sample\t${mode.to_str()}\t${$rep.to_str()}\t${elapsed.to_str()}")
            } else {
                start=now!()
                result=BtMatchfinder.skip_byte(before,21888,62080,input,0,1,258,300)
                elapsed=now!() - start
                check_after(result.tables)
                check_before(before)
                if result.next_hash3 != 21888 or result.next_hash4 != 62080 {crash "skip scalar"} else {}
                echo!("sample\t${mode.to_str()}\t${$rep.to_str()}\t${elapsed.to_str()}")
            }
        }
        $rep=$rep+1
    }
}
