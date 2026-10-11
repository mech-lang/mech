// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Port to F# by Jomo Fisher of the C#
// Small optimisations by Anthony Lloyd

#nowarn "9"

open System.Runtime.InteropServices

[<Struct; StructLayout(LayoutKind.Sequential)>]
type MPZ =
   val _mp_alloc:int
   val _mp_size:int
   val ptr:System.IntPtr

[<DllImport ("/opt/homebrew/opt/gmp/lib/libgmp.10.dylib",EntryPoint="__gmpz_init",ExactSpelling=true)>]
extern void mpzInit(MPZ& _value)

[<DllImport ("/opt/homebrew/opt/gmp/lib/libgmp.10.dylib",EntryPoint="__gmpz_mul_si",ExactSpelling=true)>]
extern void mpzMul(MPZ& _dest, MPZ&_src, int _value)

[<DllImport ("/opt/homebrew/opt/gmp/lib/libgmp.10.dylib",EntryPoint="__gmpz_add",ExactSpelling=true)>]
extern void mpzAdd(MPZ& _dest, MPZ& _src, MPZ& _src2)

[<DllImport ("/opt/homebrew/opt/gmp/lib/libgmp.10.dylib",EntryPoint="__gmpz_tdiv_q",ExactSpelling=true)>]
extern void mpzTdiv(MPZ& _dest, MPZ& _src, MPZ& _src2)

[<DllImport ("/opt/homebrew/opt/gmp/lib/libgmp.10.dylib",EntryPoint="__gmpz_set_si",ExactSpelling=true)>]
extern void mpzSet(MPZ& _src, int _value)

[<DllImport ("/opt/homebrew/opt/gmp/lib/libgmp.10.dylib",EntryPoint="__gmpz_get_si",ExactSpelling=true)>]
extern int mpzGet(MPZ& _src)

[<EntryPoint>]
let main args =

    let inline init() =
        let mutable result = MPZ()
        mpzInit(&result)
        result

    let mutable q = init()
    let mutable r = init()
    let mutable s = init()
    let mutable t = init()
    let mutable u = init()
    let mutable v = init()
    let mutable w = init()

    mpzSet(&q, 1)
    mpzSet(&r, 0)
    mpzSet(&s, 0)
    mpzSet(&t, 1)

    let inline composeR bq br bs bt =
        mpzMul(&u, &r, bs)
        mpzMul(&r, &r, bq)
        mpzMul(&v, &t, br)
        mpzAdd(&r, &r, &v)
        mpzMul(&t, &t, bt)
        mpzAdd(&t, &t, &u)
        mpzMul(&s, &s, bt)
        mpzMul(&u, &q, bs)
        mpzAdd(&s, &s, &u)
        mpzMul(&q, &q, bq)

    let inline composeL bq br bs bt =
        mpzMul(&r, &r, bt)
        mpzMul(&u, &q, br)
        mpzAdd(&r, &r, &u)
        mpzMul(&u, &t, bs)
        mpzMul(&t, &t, bt)
        mpzMul(&v, &s, br)
        mpzAdd(&t, &t, &v)
        mpzMul(&s, &s, bq)
        mpzAdd(&s, &s, &u)
        mpzMul(&q, &q, bq)

    let inline extract j =
        mpzMul(&u, &q, j)
        mpzAdd(&u, &u, &r)
        mpzMul(&v, &s, j)
        mpzAdd(&v, &v, &t)
        mpzTdiv(&w, &u, &v)
        mpzGet(&w)

    let out = System.Console.OpenStandardOutput()
    let n = int args.[0]
    let bytes = Array.zeroCreate 12
    bytes.[10] <- '\t'B; bytes.[11] <- ':'B
    let mutable i = 0
    let mutable k = 1
    while true do
        let y = extract 3
        if y = extract 4 then
            bytes.[i%10] <- byte(48+y)
            i<-i+1
            if i%10=0 then
                out.Write(bytes,0,12)
                System.Console.WriteLine i
            if i=n then
                if i%10<>0 then
                    for c = i%10 to 9 do
                        bytes.[c] <- ' 'B
                    out.Write(bytes,0,12)
                    System.Console.WriteLine n
                exit 0
            else composeR 10 (-10*y) 0 1
        else
            composeL k (4*k+2) 0 (2*k+1)
            k<-k+1
    exit 0
