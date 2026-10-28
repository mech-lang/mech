app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import Deflate
main! = |mode| {
    if mode == 0 {
        input = [102.U8, 102.U8, 102.U8, 100.U8, 101.U8, 99.U8, 99.U8, 99.U8, 103.U8, 98.U8, 97.U8, 104.U8, 100.U8, 101.U8, 99.U8, 103.U8, 100.U8, 99.U8, 98.U8, 101.U8, 100.U8, 103.U8, 104.U8, 98.U8, 101.U8, 102.U8, 98.U8, 102.U8, 103.U8, 100.U8, 103.U8, 100.U8, 104.U8, 98.U8, 102.U8, 103.U8, 99.U8, 97.U8, 100.U8, 98.U8, 100.U8, 103.U8, 99.U8, 98.U8, 101.U8, 100.U8, 104.U8, 101.U8, 103.U8, 100.U8, 99.U8, 104.U8, 102.U8, 101.U8, 101.U8, 99.U8, 103.U8, 104.U8, 97.U8, 101.U8, 99.U8, 99.U8, 99.U8, 101.U8, 103.U8, 99.U8, 103.U8, 104.U8, 98.U8, 98.U8, 100.U8, 103.U8, 104.U8, 100.U8, 97.U8, 97.U8, 103.U8, 100.U8, 97.U8, 99.U8, 102.U8, 104.U8, 97.U8, 104.U8, 98.U8, 100.U8, 101.U8, 102.U8, 98.U8, 102.U8, 102.U8, 98.U8, 104.U8, 103.U8, 100.U8, 100.U8, 101.U8, 97.U8, 100.U8, 98.U8, 99.U8, 97.U8, 101.U8, 100.U8, 102.U8, 102.U8, 102.U8, 101.U8, 100.U8, 100.U8, 104.U8, 97.U8, 104.U8, 103.U8, 104.U8, 99.U8, 98.U8, 97.U8, 99.U8, 100.U8, 98.U8, 102.U8, 98.U8, 97.U8, 98.U8, 98.U8, 101.U8, 97.U8]
        out = List.with_capacity(Deflate.compress_bound(input.len()))
        compressed = Deflate.compress_into(input, 12, out)
        var $at = 0.U64
        while $at < compressed.len() {
            echo!("byte ${(List.get(compressed, $at) ?? 0).to_str()}")
            $at = $at + 1
        }
    } else {}
    if mode == 1 {
        input = [144.U8, 147.U8, 150.U8, 153.U8, 156.U8, 159.U8, 162.U8, 165.U8, 168.U8, 171.U8, 174.U8, 177.U8, 180.U8, 183.U8, 186.U8, 189.U8, 192.U8, 195.U8, 198.U8, 201.U8, 204.U8, 207.U8, 210.U8, 213.U8, 216.U8, 219.U8, 222.U8, 225.U8, 228.U8, 231.U8, 234.U8, 237.U8]
        out = List.with_capacity(Deflate.compress_bound(input.len()))
        compressed = Deflate.compress_into(input, 12, out)
        var $at = 0.U64
        while $at < compressed.len() {
            echo!("byte ${(List.get(compressed, $at) ?? 0).to_str()}")
            $at = $at + 1
        }
    } else {}
}
