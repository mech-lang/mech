// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// ported from C# version by Anthony Lloyd
// Use DictionarySlim by Anthony Lloyd

open System
open Microsoft.Collections.Extensions

[<Literal>]
let BLOCK_SIZE = 8388608 // 1024 * 1024 * 8

[<EntryPoint>]
let main _ =
  let threeStart,threeBlocks,threeEnd =
    let input = Console.OpenStandardInput()
    let mutable threeEnd = 0
    let read buffer =
        let rec read offset count =
            let bytesRead = input.Read(buffer, offset, count)
            if bytesRead=count then offset+count
            elif bytesRead=0 then offset
            else read (offset+bytesRead) (count-bytesRead)
        threeEnd <- read 0 BLOCK_SIZE

    let rec findHeader matchIndex buffer =
        let toFind = ">THREE"B
        let find i matchIndex =
            let rec find i matchIndex =
                if matchIndex=0 then
                    let i = Array.IndexOf(buffer, toFind.[0], i)
                    if -1=i then -1,0
                    else find (i+1) 1
                else
                    let fl = toFind.Length
                    let rec tryMatch i matchIndex =
                        if i>=BLOCK_SIZE || matchIndex>=fl then i,matchIndex
                        else
                            if buffer.[i]=toFind.[matchIndex] then
                                tryMatch (i+1) (matchIndex+1)
                            else
                                find i 0
                    let i,matchIndex = tryMatch i matchIndex
                    if matchIndex=fl then i,matchIndex else -1,matchIndex
            find i matchIndex
        read buffer
        let i,matchIndex = find 0 matchIndex
        if -1<>i then i,buffer
        else findHeader matchIndex buffer

    let rec findSequence i buffer =
        let i = Array.IndexOf(buffer, '\n'B, i)
        if i <> -1 then buffer,i+1
        else
            read buffer
            findSequence 0 buffer

    let buffer,threeStart = Array.zeroCreate BLOCK_SIZE
                            |> findHeader 0 ||> findSequence

    let threeBlocks =
        if threeEnd<>BLOCK_SIZE then // Needs to be at least 2 blocks
            for i = threeEnd to BLOCK_SIZE-1 do
                buffer.[i] <- 255uy
            threeEnd <- 0
            [[||];buffer]
        else
            let rec findEnd i buffer threeBlocks =
                let i = Array.IndexOf(buffer, '>'B, i)
                if i <> -1 then
                    threeEnd <- i
                    buffer::threeBlocks
                else
                    let threeBlocks = buffer::threeBlocks
                    let buffer = Array.zeroCreate BLOCK_SIZE
                    read buffer
                    if threeEnd<>BLOCK_SIZE then buffer::threeBlocks
                    else findEnd 0 buffer threeBlocks
            let threeBlocks = findEnd threeStart buffer []
            if threeStart+18>BLOCK_SIZE then // Key needs to be in first block
                let block0 = threeBlocks.[0]
                let block1 = threeBlocks.[1]
                Buffer.BlockCopy(block0, threeStart, block0, threeStart-18,
                    BLOCK_SIZE-threeStart)
                Buffer.BlockCopy(block1, 0, block0, BLOCK_SIZE-18, 18)
                for i = 0 to 17 do block1.[i] <- 255uy
            threeBlocks

    threeStart, List.rev threeBlocks |> List.toArray, threeEnd

  let toChar = [|'A'; 'C'; 'G'; 'T'|]
  let toNum = Array.zeroCreate 256
  toNum.[int 'c'B] <- 1uy; toNum.[int 'C'B] <- 1uy
  toNum.[int 'g'B] <- 2uy; toNum.[int 'G'B] <- 2uy
  toNum.[int 't'B] <- 3uy; toNum.[int 'T'B] <- 3uy
  toNum.[int '\n'B] <- 255uy; toNum.[int '>'B] <- 255uy; toNum.[255] <- 255uy

  Array.Parallel.iter (fun bs ->
    for i = 0 to Array.length bs-1 do
        bs.[i] <- toNum.[int bs.[i]]
  ) threeBlocks

  let count l mask (summary:_->string) = async {
      let mutable rollingKey = 0
      let firstBlock = threeBlocks.[0]
      let rec startKey l start =
          if l>0 then
             rollingKey <- rollingKey <<< 2 ||| int firstBlock.[start]
             startKey (l-1) (start+1)
      startKey l threeStart
      let dict = DictionarySlim<int,int> 256
      let inline check a lo hi =
        for i = lo to hi do
          let nb = Array.get a i
          if nb<>255uy then
              rollingKey <- rollingKey &&& mask <<< 2 ||| int nb
              let x = &dict.GetOrAddValueRef(rollingKey)
              x <- x + 1

      check firstBlock (threeStart+l) (BLOCK_SIZE-1)
      
      for i = 1 to threeBlocks.Length-2 do
          check threeBlocks.[i] 0 (BLOCK_SIZE-1)
          
      let lastBlock = threeBlocks.[threeBlocks.Length-1]
      check lastBlock 0 (threeEnd-1)
      return summary dict
    }

  let writeFrequencies fragmentLength (freq:DictionarySlim<int,int>) =
    let percent = 100.0 / (freq |> Seq.sumBy (fun i -> i.Value) |> float)
    freq |> Seq.sortByDescending (fun kv -> kv.Value)
    |> Seq.collect (fun kv ->
        let keyChars = Array.zeroCreate fragmentLength
        let mutable key = kv.Key
        for i in keyChars.Length-1..-1..0 do
            keyChars.[i] <- toChar.[int key &&& 0x3]
            key <- key >>> 2
        [String(keyChars);" ";(float kv.Value * percent).ToString("F3");"\n"]
      )
    |> String.Concat

  let writeCount (fragment:string) (dict:DictionarySlim<int,int>) =
    let mutable key = 0
    for i = 0 to fragment.Length-1 do
        key <- key <<< 2 ||| int toNum.[int fragment.[i]]
    let _,v = dict.TryGetValue key
    String.Concat(string v, "\t", fragment)

  let countEnding l mask b =
    let mutable rollingKey = 0L
    let firstBlock = threeBlocks.[0]
    let rec startKey l start =
          if l>0 then
             rollingKey <- rollingKey <<< 2 ||| int64 firstBlock.[start]
             startKey (l-1) (start+1)
    startKey l threeStart
    let dict = DictionarySlim<int64,int> 256
    let inline check a lo hi =
        for i = lo to hi do
          let nb = Array.get a i
          if nb=b then
            rollingKey <- rollingKey &&& mask <<< 2 ||| int64 nb
            let x = &dict.GetOrAddValueRef(rollingKey)
            x <- x + 1
          elif nb<>255uy then
            rollingKey <- rollingKey &&& mask <<< 2 ||| int64 nb

    check firstBlock (threeStart+l) (BLOCK_SIZE-1)

    for i = 1 to threeBlocks.Length-2 do
        check threeBlocks.[i] 0 (BLOCK_SIZE-1)

    let lastBlock = threeBlocks.[threeBlocks.Length-1]
    check lastBlock 0 (threeEnd-1)

    dict

  let count64 l mask (summary:_->string) = async {
      let! dicts =
        Seq.init 4 (fun i -> async { return byte i |> countEnding l mask })
        |> Async.Parallel
      let d = DictionarySlim<int64,int>(dicts |> Array.sumBy (fun i -> i.Count))
      dicts |> Array.iter (fun di ->
        di |> Seq.iter (fun kv ->
          let x = &d.GetOrAddValueRef kv.Key
          x <- kv.Value
        )
      )
      return summary d
    }

  let writeCount64 (fragment:string) (dict:DictionarySlim<_,_>) =
    let mutable key = 0L
    for i = 0 to fragment.Length-1 do
        key <- key <<< 2 ||| int64 toNum.[int fragment.[i]]
    let _,v = dict.TryGetValue key
    String.Concat(string v, "\t", fragment)

  Async.Parallel [
    count64 18 0x7FFFFFFFFL (writeCount64 "GGTATTTTAATTTATAGT")
    count 12 0x7FFFFF (writeCount "GGTATTTTAATT")
    count 6 0x3FF (writeCount "GGTATT")
    count 4 0x3F (writeCount "GGTA")
    count 3 0xF (writeCount "GGT")
    count 2 0x3 (writeFrequencies 2)
    count 1 0 (writeFrequencies 1)
  ]
  |> Async.RunSynchronously
  |> Array.rev
  |> Array.iter stdout.WriteLine

  exit 0
