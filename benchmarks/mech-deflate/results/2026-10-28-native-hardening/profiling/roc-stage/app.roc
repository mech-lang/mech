app [main!] {
 pf: platform "../../roc-deflate-repro-20261024/roc-deflate/benchmark/echo-clock/main.roc",
 deflate: "../../roc-deflate-repro-20261024/roc-deflate/package/main.roc",
}
import Bench
import "input" as input : List(U8)
import "prefix" as prefix : List(U8)
import "expected.deflate" as expected : List(U8)
main! = |level| Bench.run!(input, prefix, expected, 20000, level)
