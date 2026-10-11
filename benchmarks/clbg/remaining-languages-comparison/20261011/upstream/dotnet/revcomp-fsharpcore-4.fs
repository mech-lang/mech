// The Computer Language Benchmarks Game
// https://benchmarksgame-team.pages.debian.net/benchmarksgame/
//
// contributed by Jimmy Tang
// multithreaded by Anthony Lloyd
open System
open System.Threading

[<Literal>]
let PageSize = 1048576 // 1024 * 1024
let pages = Array.zeroCreate 2048
let mutable readCount, canWriteCount, lastPageSize = 0, 0, -1

Thread(fun () ->
    let stream = Console.OpenStandardInput()
    let rec loop() =
        let buffer = Array.zeroCreate PageSize
        let rec read offset count =
            let bytesRead = stream.Read(buffer, offset, count)
            if bytesRead=count then offset+count
            elif bytesRead=0 then offset
            else read (offset+bytesRead) (count-bytesRead)
        let bytesRead = read 0 PageSize
        pages.[readCount] <- buffer
        readCount <- readCount + 1
        if bytesRead<>PageSize then lastPageSize <- bytesRead else loop()
    loop()
).Start()

Thread(fun () ->
    let map = Array.init 256 byte
    Array.iter2 (fun i v -> map.[int i] <- v)
        "ABCDGHKMRTVYabcdghkmrtvy"B
        "TVGHCDMKYABRTVGHCDMKYABR"B

    let reverse startPage startIndex endPage endIndex (previous:Thread option) =
        let mutable loPageID, lo, loPage = startPage, startIndex, pages.[startPage]
        let mutable hiPageID, hi, hiPage = endPage, endIndex, pages.[endPage]
        let inline checkhilo() =
            if PageSize=lo then
                loPageID <- loPageID+1
                if previous.IsNone || not previous.Value.IsAlive then
                    canWriteCount <- loPageID
                loPage <- pages.[loPageID]
                lo <- 0
            if -1=hi then
                hiPageID <- hiPageID-1
                hiPage <- pages.[hiPageID]
                hi <- PageSize-1
            loPageID<hiPageID || (loPageID=hiPageID && lo<=hi)
        while checkhilo() do
            let iValue = loPage.[lo]
            let jValue = hiPage.[hi]
            if iValue='\n'B || jValue='\n'B then
                if iValue='\n'B then lo <- lo+1
                if jValue='\n'B then hi <- hi-1
            else
                loPage.[lo] <- map.[int jValue]
                hiPage.[hi] <- map.[int iValue]
                lo <- lo+1
                hi <- hi-1
        if previous.IsSome then previous.Value.Join()
        canWriteCount <- endPage

    let rec reverseAll page i previousThread =
        let rec skipHeader page i =
            while page = readCount do Thread.SpinWait 0
            let i = Array.IndexOf(pages.[page],'\n'B, i, PageSize-i)
            if -1<>i then page,i+1 else skipHeader (page+1) 0
        let loPageID, lo = skipHeader page i
        let rec findNextAndReverse pageID i previousThread =
            while pageID = readCount do Thread.SpinWait 0
            let onLastPage = pageID + 1 = readCount && lastPageSize <> -1
            let thisPageSize = if onLastPage then lastPageSize else PageSize
            let i = Array.IndexOf(pages.[pageID],'>'B, i, thisPageSize-i)
            if -1<>i then
                let newThread = Thread(fun () ->
                    reverse loPageID lo pageID (i-1) previousThread)
                newThread.Start()
                reverseAll pageID i (Some newThread)
            elif onLastPage then
                reverse loPageID lo pageID (lastPageSize-1) previousThread
                canWriteCount <- readCount
            else findNextAndReverse (pageID+1) 0 previousThread
        findNextAndReverse loPageID lo previousThread
    reverseAll 0 0 None
).Start()

let stream = Console.OpenStandardOutput()
let rec loop writtenCount =
    while writtenCount = canWriteCount do Thread.SpinWait 0
    if writtenCount+1 = readCount && lastPageSize <> -1 then
        stream.Write(pages.[writtenCount], 0, lastPageSize)
    else
        stream.Write(pages.[writtenCount], 0, PageSize)
        loop (writtenCount+1)
loop 0
