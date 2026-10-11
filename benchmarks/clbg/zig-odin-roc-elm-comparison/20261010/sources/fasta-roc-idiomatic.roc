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
alu = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
cumulative : List U8, List F64 -> List (U8,F64)
cumulative = |letters, probabilities|
    (_,pairs) = List.walk(List.map2(letters,probabilities,|a,b| (a,b)),(0f64,[]),|state,pair|
        (total,items) = state
        (letter,probability) = pair
        next = total+probability
        (next,List.append(items,(letter,next))),
    )
    pairs
choose = |value, pairs|
    when pairs is
        [(letter,threshold),.. as rest] -> if value < threshold then letter else choose(value,rest)
        [] -> 116u8
line = |remaining, offset, seed, repeated, distribution, bytes|
    if remaining == 0 then (bytes,offset,seed) else
        next_seed = if repeated then seed else (seed*3877+29573)%139968
        value : F64
        value = Num.to_f64(next_seed)/139968.0
        letter = if repeated then get(Str.to_utf8(alu),offset%Str.count_utf8_bytes(alu)) else choose(value,distribution)
        line(remaining-1,offset+1,next_seed,repeated,distribution,List.append(bytes,letter))
emit! = |remaining, offset, seed, repeated, distribution, buffer|
    if remaining == 0 then
        Stdout.write_bytes!(buffer)?
        Ok(seed)
    else
        take = Num.min(60,remaining)
        (bytes,next_offset,next_seed) = line(take,offset,seed,repeated,distribution,buffer)
        next = List.append(bytes,10u8)
        if List.len(next) >= 65536 then
            Stdout.write_bytes!(next)?
            emit!(remaining-take,next_offset,next_seed,repeated,distribution,List.with_capacity(65600))
        else emit!(remaining-take,next_offset,next_seed,repeated,distribution,next)
main! : List Arg => Result {} _
main! = |args|
    n = argument(args)
    iub = cumulative(Str.to_utf8("acgtBDHKMNRSVWY"),[0.27,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02])
    homo = cumulative(Str.to_utf8("acgt"),[0.3029549426680,0.1979883004921,0.1975473066391,0.3015094502008])
    Stdout.write!(">ONE Homo sapiens alu\n")?
    _ = emit!(2*n,0,42,Bool.true,[],List.with_capacity(65600))?
    Stdout.write!(">TWO IUB ambiguity codes\n")?
    seed = emit!(3*n,0,42,Bool.false,iub,List.with_capacity(65600))?
    Stdout.write!(">THREE Homo sapiens frequency\n")?
    _ = emit!(5*n,0,seed,Bool.false,homo,List.with_capacity(65600))?
    Ok({})
