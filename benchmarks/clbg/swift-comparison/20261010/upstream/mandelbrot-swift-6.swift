/* The Computer Language Benchmarks Game
 https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

 direct transliteration of Greg Buchholz's C program
 contributed by Isaac Gouy, fix by David Turnbull
 dispatching to multiple cores, use SIMD operationn, early break
 depending on previous depth by Patrick Stein
 */

import Foundation
#if os(Linux)
import Dispatch
#endif

let width       = Int(CommandLine.arguments[1])!
let height      = width
let iterations  = 50
let depth       = 4.0
let startPoint  = (x:-1.5, i:-1.0)
let endPoint    = (x:0.5,  i:1.0)

let serialQueue     = DispatchQueue(label: "outputlinesaccess")
let serialGroup     = DispatchGroup()
let stdout          = FileHandle.standardOutput

serialGroup.enter()
serialQueue.async {
    stdout.write("P4\n\(width) \(height)\n".data(using:.utf8)!)
    serialGroup.leave()
}

let linecount   = height
let linesize    = (width + 7) / 8

let pixelheight = abs(endPoint.x-startPoint.x) / Double(height)
let pixelwidth  = abs(endPoint.i-startPoint.i) / Double(width)
let pixelwidth8 = pixelwidth * 8.0

var Ci      = startPoint.i
let Cr0     = (SIMD8<Double>(0,1,2,3,4,5,6,7) * pixelwidth) + startPoint.x


let dispatchGroup   = DispatchGroup()
let outputsize      = linecount * linesize
let originalAddress = UnsafeMutablePointer<UInt8>.allocate(capacity: outputsize)

var counter = 0

for _ in 0..<linecount
{
    dispatchGroup.enter()
    DispatchQueue.global().async
    {
        [counter,Ci] in

        var address = counter
        var lasttimeshallow = true
        var xd = 0.0

        for _ in 0..<linesize
        {
            let Cr = Cr0 + ( xd * pixelwidth8 )
            let byte = mandelbrot(Cr: Cr,Ci: Ci,shallow:lasttimeshallow)

            lasttimeshallow = (0 == byte)

            originalAddress[address] = byte
            address += 1
            xd += 1.0
        }
        dispatchGroup.leave()
    }
    counter += linesize
    Ci      += pixelheight
}

dispatchGroup.wait()
serialGroup.wait()
stdout.write(Data(bytesNoCopy: originalAddress,
                        count: outputsize,
                  deallocator: .none))


// SIMD mandelbrot calculation for 8 points in parallel

@inline(__always) func onepixel(Cr:SIMD8<Double>,Ci:Double,
                                Zr:inout SIMD8<Double>,Zi:inout SIMD8<Double>,
                                Tr:inout SIMD8<Double>,Ti:inout SIMD8<Double>)
{
    Zi = 2.0 * Zr * Zi + Ci
    Zr = Tr - Ti + Cr
    Tr = Zr * Zr
    Ti = Zi * Zi
}

// one line calculation stepping 8 points at a time

@inline(__always)
func mandelbrot(Cr:SIMD8<Double>,Ci:Double,shallow:Bool) -> UInt8
{
    var Zr:SIMD8<Double> = .zero
    var Zi:SIMD8<Double> = .zero
    var Tr:SIMD8<Double> = .zero
    var Ti:SIMD8<Double> = .zero

    let thresholds  = SIMD8<Double>(repeating:depth)
    let isTrue      = SIMDMask<SIMD8<Double.SIMDMaskScalar>>(repeating: true)
    let ramp        = SIMD8<Int64>([128,64,32,16,8,4,2,1])

    var cmpresult:SIMDMask<SIMD8<Double.SIMDMaskScalar>>

    if shallow
    {
        for _ in 0..<iterations/4
        {
            for _ in 0..<4
            {
                onepixel(Cr:Cr,Ci:Ci,Zr:&Zr,Zi:&Zi,Tr:&Tr,Ti:&Ti)
            }
            cmpresult = Tr + Ti .>= thresholds

            if cmpresult == isTrue
            {
                return 0
            }
        }
        onepixel(Cr:Cr,Ci:Ci,Zr:&Zr,Zi:&Zi,Tr:&Tr,Ti:&Ti)
        onepixel(Cr:Cr,Ci:Ci,Zr:&Zr,Zi:&Zi,Tr:&Tr,Ti:&Ti)
    }
    else
    {
        for _ in 0..<iterations
        {
            onepixel(Cr:Cr,Ci:Ci,Zr:&Zr,Zi:&Zi,Tr:&Tr,Ti:&Ti)
        }
    }
    cmpresult = Tr + Ti .< thresholds

    let reduced: SIMD8<Int64> = unsafeBitCast(cmpresult,to: SIMD8<Int64>.self)
    let summask: SIMD8<Int64> = ramp & reduced

    let byte = summask.wrappedSum()

    return UInt8(byte)
}
