import pf.Host exposing [echo!, now!]
import deflate.Deflate

Bench := [].{
    run! : List(U8), List(U8), List(U8), U64, U64 => {}
    run! = |input, prefix, expected, reps, level_number| {
        level = Deflate.Level.from_u64(level_number) ?? crash "bad level"
        var $buf = List.with_capacity(Deflate.compress_bound(input.len()) + prefix.len())
        $buf = List.append_sublist($buf, prefix, { start: 0, len: prefix.len() })
        warm_started = now!()
        $buf = Deflate.compress_into(input, level, $buf)
        if $buf != expected { crash "reference output differs" }
        raw = List.sublist($buf, { start: prefix.len(), len: $buf.len() - prefix.len() })
        decoded = Deflate.decompress(raw) ?? crash "decompression failed"
        if decoded != input { crash "round-trip differs" }
        while now!() - warm_started < 200000000 {
            empty = List.append_sublist(List.clear($buf), prefix, { start: 0, len: prefix.len() })
            $buf = Deflate.compress_into(input, level, empty)
            if $buf != expected { crash "warmup output differs" }
        }
        var $rep = 0.U64
        while $rep < reps {
            empty = List.append_sublist(List.clear($buf), prefix, { start: 0, len: prefix.len() })
            start = now!()
            $buf = Deflate.compress_into(input, level, empty)
            elapsed = now!() - start
            if $buf != expected { crash "timed output differs" }
            echo!("rep\troc-deflate\t${$rep.to_str()}\t${elapsed.to_str()}\t${($buf.len() - prefix.len()).to_str()}")
            $rep = $rep + 1
        }
    }
}
