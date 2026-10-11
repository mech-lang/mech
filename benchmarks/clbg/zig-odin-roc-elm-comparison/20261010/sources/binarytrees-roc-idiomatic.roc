app [main!] {
    pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU.tar.br",
}
import pf.Stdout
import pf.Arg exposing [Arg]
argument : List Arg -> U64
argument = |args|
    arg = (List.get(args, 1) |> Result.map_ok(Arg.display)) ?? "0"
    Str.to_u64(arg) ?? 0
Tree : [Node U64 Tree Tree, Empty]

build = |depth, identity|
    if depth == 0 then Node(identity,Empty,Empty) else
        Node(identity,build(depth-1,identity*2),build(depth-1,identity*2+1))
check = |tree|
    when tree is
        Empty -> 0u64
        Node(_,left,right) -> 1+check(left)+check(right)
temporary = |remaining, depth, total|
    if remaining == 0 then total else
        temporary(remaining-1,depth,total+check(build(depth,remaining)))
depths = |depth, maximum, output|
    if depth > maximum then output else
        iterations = Num.pow_int(2u64,maximum-depth+4)
        total = temporary(iterations,depth,0)
        line = "${Num.to_str(iterations)}\t trees of depth ${Num.to_str(depth)}\t check: ${Num.to_str(total)}\n"
        depths(depth+2,maximum,Str.concat(output,line))
main! : List Arg => Result {} _
main! = |args|
    maximum = Num.max(6,argument(args))
    stretch = check(build(maximum+1,1))
    long_lived = build(maximum,1)
    rows = depths(4,maximum,"")
    retained = check(long_lived)
    Stdout.write!("stretch tree of depth ${Num.to_str(maximum+1)}\t check: ${Num.to_str(stretch)}\n${rows}long lived tree of depth ${Num.to_str(maximum)}\t check: ${Num.to_str(retained)}\n")
