app [main!] {
    pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU.tar.br",
}
import pf.Stdout
import pf.Arg exposing [Arg]
argument : List Arg -> U64
argument = |args|
    arg = (List.get(args, 1) |> Result.map_ok(Arg.display)) ?? "0"
    Str.to_u64(arg) ?? 0
get = |items, index|
    when List.get(items,index) is
        Ok(value) -> value
        Err(_) -> crash("index out of bounds")
reverse_prefix = |items, first, last|
    if first >= last then items else reverse_prefix(List.swap(items,first,last),first+1,last-1)
flips = |items, total|
    first = get(items,0)
    if first == 0 then total else flips(reverse_prefix(items,0,first),total+1)
rotate = |items, first, i, rank|
    if i == rank then List.set(items,rank,first) else
        value = get(items,i+1)
        rotate(List.set(items,i,value),first,i+1,rank)
prepare = |rank, counts|
    if rank <= 1 then counts else prepare(rank-1,List.set(counts,rank-1,rank))
next = |n, rank, items, counts|
    if rank == n then Finished else
        first = get(items,0)
        rotated = rotate(items,first,0,rank)
        remaining = get(counts,rank)-1
        updated = List.set(counts,rank,remaining)
        if remaining > 0 then Next(rotated,updated,rank) else next(n,rank+1,rotated,updated)
enumerate = |n, items, counts, rank, index, checksum, maximum|
    prepared = prepare(rank,counts)
    current : I64
    current = flips(items,0)
    total = checksum+(if Num.is_even(index) then current else -current)
    largest = Num.max(maximum,current)
    when next(n,1,items,prepared) is
        Finished -> "${Num.to_str(total)}\nPfannkuchen(${Num.to_str(n)}) = ${Num.to_str(largest)}\n"
        Next(perm,cs,r) -> enumerate(n,perm,cs,r,index+1,total,largest)
main! : List Arg => Result {} _
main! = |args|
    n = argument(args)
    output = enumerate(n,List.range({start:At(0u64),end:Before(n)}),List.repeat(0u64,n),n,0u64,0i64,0i64)
    Stdout.write!(output)
