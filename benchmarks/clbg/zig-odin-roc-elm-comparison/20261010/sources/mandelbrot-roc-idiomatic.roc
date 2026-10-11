app [main!] {
    pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU.tar.br",
}
import pf.Stdout
import pf.Arg exposing [Arg]
argument : List Arg -> U64
argument = |args|
    arg = (List.get(args, 1) |> Result.map_ok(Arg.display)) ?? "0"
    Str.to_u64(arg) ?? 0
inside : U64, F64, F64, F64, F64 -> U8
inside = |remaining, cr, ci, zr, zi|
    if zr*zr+zi*zi > 4 then 0 else
        if remaining == 0 then 1 else inside(remaining-1,cr,ci,zr*zr-zi*zi+cr,2*zr*zi+ci)
byte = |n, row, x, bit, packed|
    if bit == 8 then packed else
        value = if x+bit < n then inside(50,2*Num.to_f64(x+bit)/Num.to_f64(n)-1.5,2*Num.to_f64(row)/Num.to_f64(n)-1,0,0) else 0
        byte(n,row,x,bit+1,Num.shift_left_by(packed,1)+value)
row_bytes = |n, row, x, bytes|
    if x >= n then bytes else row_bytes(n,row,x+8,List.append(bytes,byte(n,row,x,0,0u8)))
rows! = |n, row|
    if row >= n then Ok({}) else
        Stdout.write_bytes!(row_bytes(n,row,0,List.with_capacity((n+7)//8)))?
        rows!(n,row+1)
main! : List Arg => Result {} _
main! = |args|
    n = argument(args)
    Stdout.write!("P4\n${Num.to_str(n)} ${Num.to_str(n)}\n")?
    rows!(n,0)
