app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import Deflate
main! = |mode| {
    if mode == 0 {
        input = []
        var $out = List.with_capacity(Deflate.compress_bound(input.len()) + 0.U64)
        compressed = Deflate.compress_into(input, 12, $out)
        var $i_byte = 0.U64
        while $i_byte < compressed.len() {
            echo!("byte ${(List.get(compressed, $i_byte) ?? 0).to_str()}")
            $i_byte = $i_byte + 1
        }
    } else {}
    if mode == 1 {
        input = [65.U8]
        var $out = List.with_capacity(Deflate.compress_bound(input.len()) + 0.U64)
        compressed = Deflate.compress_into(input, 12, $out)
        var $i_byte = 0.U64
        while $i_byte < compressed.len() {
            echo!("byte ${(List.get(compressed, $i_byte) ?? 0).to_str()}")
            $i_byte = $i_byte + 1
        }
    } else {}
    if mode == 2 {
        input = [97.U8, 98.U8, 99.U8, 100.U8, 101.U8, 102.U8, 103.U8]
        var $out = List.with_capacity(Deflate.compress_bound(input.len()) + 2.U64)
        $out = List.append($out, 7.U8)
        $out = List.append($out, 8.U8)
        compressed = Deflate.compress_into(input, 12, $out)
        var $i_byte = 0.U64
        while $i_byte < compressed.len() {
            echo!("byte ${(List.get(compressed, $i_byte) ?? 0).to_str()}")
            $i_byte = $i_byte + 1
        }
    } else {}
    if mode == 3 {
        input = [97.U8, 98.U8, 99.U8, 100.U8, 101.U8, 102.U8, 103.U8, 104.U8]
        var $out = List.with_capacity(Deflate.compress_bound(input.len()) + 0.U64)
        compressed = Deflate.compress_into(input, 12, $out)
        var $i_byte = 0.U64
        while $i_byte < compressed.len() {
            echo!("byte ${(List.get(compressed, $i_byte) ?? 0).to_str()}")
            $i_byte = $i_byte + 1
        }
    } else {}
    if mode == 4 {
        input = [97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8]
        var $out = List.with_capacity(Deflate.compress_bound(input.len()) + 0.U64)
        compressed = Deflate.compress_into(input, 12, $out)
        var $i_byte = 0.U64
        while $i_byte < compressed.len() {
            echo!("byte ${(List.get(compressed, $i_byte) ?? 0).to_str()}")
            $i_byte = $i_byte + 1
        }
    } else {}
    if mode == 5 {
        input = [97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8, 97.U8]
        var $out = List.with_capacity(Deflate.compress_bound(input.len()) + 0.U64)
        compressed = Deflate.compress_into(input, 12, $out)
        var $i_byte = 0.U64
        while $i_byte < compressed.len() {
            echo!("byte ${(List.get(compressed, $i_byte) ?? 0).to_str()}")
            $i_byte = $i_byte + 1
        }
    } else {}
    if mode == 6 {
        input = [97.U8, 98.U8, 99.U8, 100.U8, 101.U8, 102.U8, 103.U8, 104.U8]
        var $out = List.with_capacity(Deflate.compress_bound(input.len()) + 2.U64)
        $out = List.append($out, 7.U8)
        $out = List.append($out, 8.U8)
        compressed = Deflate.compress_into(input, 12, $out)
        var $i_byte = 0.U64
        while $i_byte < compressed.len() {
            echo!("byte ${(List.get(compressed, $i_byte) ?? 0).to_str()}")
            $i_byte = $i_byte + 1
        }
    } else {}
}
