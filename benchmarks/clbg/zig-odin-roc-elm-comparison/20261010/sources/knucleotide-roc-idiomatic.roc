app [main!] {
    pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU.tar.br",
}
import pf.Stdout
import pf.Stdin
import pf.Arg exposing [Arg]
get = |items, index|
    when List.get(items,index) is
        Ok(value) -> value
        Err(_) -> crash("index out of bounds")
fixed : F64, U64 -> Str
fixed = |value, places|
    factor = Num.pow_int(10u64, places)
    scaled : U64
    scaled = Num.round(Num.abs(value) * Num.to_f64(factor))
    whole = scaled // factor
    digits = Num.to_str(scaled % factor)
    padding = Str.repeat("0", places - Str.count_utf8_bytes(digits))
    sign = if value < 0 then "-" else ""
    "${sign}${Num.to_str(whole)}.${padding}${digits}"
code = |byte|
    when byte is
        67 | 99 -> 1u64
        71 | 103 -> 2u64
        84 | 116 -> 3u64
        _ -> 0u64
encode = |text| List.walk(Str.to_utf8(text),0u64,|key,byte| key*4+code(byte))
decode = |width, key, bytes|
    if width == 0 then Str.from_utf8_lossy(List.reverse(bytes)) else
        char = get([65u8,67,71,84],key%4)
        decode(width-1,key//4,List.append(bytes,char))
counts = |width, bytes|
    mask = Num.pow_int(4u64,width)-1
    (_,result) = List.walk_with_index(bytes,(0u64,Dict.empty({})),|state,byte,index|
        (key,table) = state
        next_key = Num.bitwise_and(key*4+Num.to_u64(byte),mask)
        updated = if index+1 >= width then
            Dict.update(table,next_key,|previous| Ok((previous ?? 0)+1))
        else table
        (next_key,updated),
    )
    result
frequencies = |width, bytes|
    entries = Dict.to_list(counts(width,bytes))
    sorted = List.sort_with(entries,|a,b|
        (ka,ca) = a
        (kb,cb) = b
        if ca == cb then Num.compare(ka,kb) else Num.compare(cb,ca),
    )
    denominator = Num.to_f64(List.len(bytes)-width+1)
    rows = List.map(sorted,|pair|
        (key,total) = pair
        "${decode(width,key,[])} ${fixed(100*Num.to_f64(total)/denominator,3)}\n",
    )
    Str.concat(Str.join_with(rows,""),"\n")
main! : List Arg => Result {} _
main! = |_args|
    raw = Stdin.read_to_end!({})?
    text = Str.from_utf8_lossy(raw)
    record = (Str.split_first(text,">THREE") |> Result.map_ok(|x| x.after)) ?? ""
    rest = (Str.split_first(record,"\n") |> Result.map_ok(|x| x.after)) ?? ""
    sequence = (Str.split_first(rest,">") |> Result.map_ok(|x| x.before)) ?? rest
    bytes = List.map(List.keep_if(Str.to_utf8(sequence),|b| b != 10 && b != 13),code)
    packed = List.map(bytes,Num.to_u8)
    first = frequencies(1,packed)
    second = frequencies(2,packed)
    queries = [(3u64,"GGT"),(4,"GGTA"),(6,"GGTATT"),(12,"GGTATTTTAATT"),(18,"GGTATTTTAATTTATAGT")]
    rows = List.map(queries,|pair|
        (width,query) = pair
        table = counts(width,packed)
        number = Dict.get(table,encode(query)) ?? 0
        "${Num.to_str(number)}\t${query}\n",
    )
    Stdout.write!(Str.concat(Str.concat(first,second),Str.join_with(rows,"")))
