app [main!] {
 pf: platform "../../../main.roc",
 deflate: "../../../../../package/main.roc",
}
import Bench
import "input" as input : List(U8)
import "prefix" as prefix : List(U8)
import "reference.deflate" as expected : List(U8)
main! = |level| Bench.run!(input, prefix, expected, 5, level)
