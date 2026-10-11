// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Port to F# by Jomo Fisher of the C#
// Port to F# by Anthony Lloyd of the C program

open System.Runtime.InteropServices

[<Struct;StructLayout(LayoutKind.Sequential)>]
type MPZ =
    val alloc: int
    val size: int
    val ptr: System.IntPtr

[<DllImport("gmp",EntryPoint="__gmpz_init",ExactSpelling=true)>]
extern void private mpzInit(MPZ& _value)

[<DllImport("gmp",EntryPoint="__gmpz_init_set_ui",ExactSpelling=true)>]
extern void private mpzInitSetUi(MPZ& _dest, int _value)

[<DllImport("gmp",EntryPoint="__gmpz_mul_ui",ExactSpelling=true)>]
extern void private mpzMulUi(MPZ& _dest, MPZ& _src, int _value)

[<DllImport("gmp",EntryPoint="__gmpz_add",ExactSpelling=true)>]
extern void private mpzAdd(MPZ& _dest, MPZ& _src, MPZ& _src2)

[<DllImport("gmp",EntryPoint="__gmpz_tdiv_q",ExactSpelling=true)>]
extern void private mpzTdivQ(MPZ& _dest, MPZ& _src, MPZ& _src2)

[<DllImport("gmp",EntryPoint="__gmpz_get_ui",ExactSpelling=true)>]
extern int private mpzGetUi(MPZ& _src)

[<DllImport("gmp",EntryPoint="__gmpz_submul_ui",ExactSpelling=true)>]
extern void private mpzSubmulUi(MPZ& _dest, MPZ& _src, int _value)

[<DllImport("gmp",EntryPoint="__gmpz_addmul_ui",ExactSpelling=true)>]
extern void private mpzAddmulUi(MPZ& _dest, MPZ& _src, int _value)

[<DllImport("gmp",EntryPoint="__gmpz_cmp",ExactSpelling=true)>]
extern int private mpzCmp(MPZ& _op1, MPZ& _op2)

[<EntryPoint>]
let main (args:string[]) =

    let mutable tmp1, tmp2, acc, den, num = MPZ(), MPZ(), MPZ(), MPZ(), MPZ()
    
    mpzInit(&tmp1)
    mpzInit(&tmp2)
    mpzInitSetUi(&acc, 0)
    mpzInitSetUi(&den, 1)
    mpzInitSetUi(&num, 1)
    
    let extractDigit nth =
        mpzMulUi(&tmp1, &num, nth)
        mpzAdd(&tmp2, &tmp1, &acc)
        mpzTdivQ(&tmp1, &tmp2, &den)
        mpzGetUi(&tmp1)

    let eliminateDigit d =
        mpzSubmulUi(&acc, &den, d)
        mpzMulUi(&acc, &acc, 10)
        mpzMulUi(&num, &num, 10)

    let nextTerm k =
        let k2 = k * 2 + 1
        mpzAddmulUi(&acc, &num, 2)
        mpzMulUi(&acc, &acc, k2)
        mpzMulUi(&den, &den, k2)
        mpzMulUi(&num, &num, k)

    let out = System.Console.OpenStandardOutput()
    let bytes = "0123456789\t:00\n\n\n\n\n\n\n\n"B
    let mutable i, j, k, n = 0, 12, 0, int args.[0]
 
    let rec incTotal p =
        let v = bytes.[p]
        if v<>'9'B then bytes.[p] <- v + 1uy
        elif p=12 then
            bytes.[12] <- '1'B
            j <- j + 1
            for k = 13 to j+1 do
                bytes.[k] <- '0'B
        else
            bytes.[p] <- '0'B
            incTotal (p-1)

    while n > 0 do
        k <- k + 1
        nextTerm k
        if mpzCmp(&num, &acc) <= 0 then
            let d = extractDigit 3
            if d = extractDigit 4 then
                bytes.[i] <- byte d + '0'B
                i <- if i=9 then
                        incTotal j
                        out.Write(bytes, 0, j+3)
                        0
                     else i + 1
                eliminateDigit d
                n <- n - 1

    0
