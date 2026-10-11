app [main!] {
    pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU.tar.br",
}
import pf.Stdout
import pf.Arg exposing [Arg]
argument : List Arg -> U64
argument = |args|
    arg = (List.get(args, 1) |> Result.map_ok(Arg.display)) ?? "0"
    Str.to_u64(arg) ?? 0
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
coefficient : U64, U64 -> F64
coefficient = |i,j| 1 / Num.to_f64(((i+j)*(i+j+1)//2)+i+1)
multiply = |transposed, n, values|
    List.map(List.range({start:At(0u64),end:Before(n)}), |i|
        List.walk_with_index(values,0f64,|total,value,j|
            a = if transposed then coefficient(j,i) else coefficient(i,j)
            total+a*value,
        ),
    )
ata = |n, values| multiply(Bool.true,n,multiply(Bool.false,n,values))
power = |remaining, n, u, v|
    if remaining == 0 then (u,v) else
        v1 = ata(n,u)
        u1 = ata(n,v1)
        power(remaining-1,n,u1,v1)
main! : List Arg => Result {} _
main! = |args|
    n = argument(args)
    (u,v) = power(10,n,List.repeat(1f64,n),List.repeat(0f64,n))
    uv = List.sum(List.map2(u,v,Num.mul))
    vv = List.sum(List.map(v,|x| x*x))
    Stdout.write!("${fixed(Num.sqrt(uv/vv),9)}\n")
