app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import Deflate
import "16384-distribution-change.qualification-input" as changing : List(U8)
import "33027-periodic-window.qualification-input" as window : List(U8)
main! = |mode| {
    input = if mode == 0 { changing } else { window }
    output = List.with_capacity(Deflate.compress_bound(input.len()))
    compressed = Deflate.compress_into(input, 12, output)
    var $at = 0.U64
    while $at < compressed.len() {
        echo!("byte ${(List.get(compressed, $at) ?? 0).to_str()}")
        $at = $at + 1
    }
}
