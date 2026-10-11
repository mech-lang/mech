// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// ported from C# version by Anthony Lloyd

#nowarn "9"

open Microsoft.FSharp.NativeInterop

[<EntryPoint>]
let main args =

    let run n fact taskSize taskId =

        let inline firstPermutation p pp count n idx =
            for i = 0 to n-1 do NativePtr.set p i (int16 i)
            let mutable idx = idx
            for i = n-1 downto 1 do
                let d = idx/NativePtr.get fact i
                NativePtr.set count i d
                if d<>0 then
                    for j = 0 to i do
                        NativePtr.get p j |> NativePtr.set pp j
                    for j = 0 to i do
                        NativePtr.get pp ((j+d) % (i+1)) |> NativePtr.set p j
                    idx <- idx % NativePtr.get fact i

        let inline nextPermutation p count =
            let mutable first = NativePtr.get p 1
            NativePtr.read p |> NativePtr.set p 1
            NativePtr.write p first
            let mutable i = 1
            let mutable c = NativePtr.get count i
            while c>=i do
                NativePtr.set count i 0
                let next = NativePtr.get p 1
                NativePtr.write p next
                for j = 1 to i do NativePtr.get p (j+1) |> NativePtr.set p j
                i <- i+1
                NativePtr.set p i first
                first <- next
                c <- NativePtr.get count i
            NativePtr.set count i (c+1)

        let inline copy p pp n =
            let startL = NativePtr.toNativeInt p |> NativePtr.ofNativeInt<int64>
            let stateL = NativePtr.toNativeInt pp |> NativePtr.ofNativeInt<int64>
            let lengthL = n / 4
            let mutable i = 0
            while i < lengthL do
                NativePtr.get startL i |> NativePtr.set stateL i
                i <- i + 1
            i <- lengthL * 4
            while i < n do
                NativePtr.get p i |> NativePtr.set pp i
                i <- i + 1

        let inline countFlips p pp n =
            let mutable flips = 1
            let mutable first = NativePtr.read p |> int
            if NativePtr.get p first <> 0s then
                copy p pp n
                while NativePtr.get pp first <> 0s do
                    flips <- flips + 1
                    if first > 2 then
                        let mutable lo = 1
                        let mutable hi = first-1
                        while lo<hi do
                            let t = NativePtr.get pp lo
                            NativePtr.get pp hi |> NativePtr.set pp lo
                            NativePtr.set pp hi t
                            lo <- lo+1
                            hi <- hi-1
                    let temp = NativePtr.get pp first
                    NativePtr.set pp first (int16 first)
                    first <- int temp
            flips

        let p = NativePtr.stackalloc<int16> n
        let pp = NativePtr.stackalloc<int16> n
        let count = NativePtr.stackalloc n
        firstPermutation p pp count n (taskId*taskSize)
        let mutable chksum =
            if NativePtr.read p = 0s then 0
            else countFlips p pp n
        let mutable maxflips = chksum
        for i = 1 to taskSize-1 do
            nextPermutation p count
            if NativePtr.read p <> 0s then
                let flips =  countFlips p pp n
                chksum <- chksum + (1-(i%2)*2) * flips
                if flips>maxflips then maxflips <- flips
        chksum, maxflips

    let n = if args.Length=0 then 7 else int args.[0]
    use fact = fixed &(Array.zeroCreate (n+1)).[0]
    NativePtr.write fact 1
    let mutable factn = 1
    for i = 1 to n do
        factn <- factn * i
        NativePtr.set fact i factn

    let chksum, maxFlips =
        let taskSize = factn / System.Environment.ProcessorCount
        Array.init System.Environment.ProcessorCount
            (fun i -> async { return run n fact taskSize i })
        |> Async.Parallel
        |> Async.RunSynchronously
        |> Array.reduce (fun (c1,f1) (c2,f2) -> c1+c2,max f1 f2)
           
    string chksum+"\nPfannkuchen("+string n+") = "+string maxFlips
    |> stdout.WriteLine

    0
