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

guard   CommandLine.argc > 1,
        let width = Int(CommandLine.arguments[1]), width > 0
else    {
            print("usage: \(CommandLine.arguments[0]) <size>")
            exit(EXIT_FAILURE)
        }
let height      = width
let iterations  = 50
let depth       = 4.0
let startPoint  = (x:-1.5, i:-1.0)
let endPoint    = (x:0.5,  i:1.0)

let linecount   = height
let linesize    = (width + 7) / 8
let pixelheight = abs(endPoint.x-startPoint.x) / Double(height)
let pixelwidth  = abs(endPoint.i-startPoint.i) / Double(width)
let pixelwidth8 = pixelwidth * 8.0

let outputsize  = linecount * linesize
let pixelBuffer = UnsafeMutablePointer<UInt8>.allocate(capacity: outputsize)
var Ci0         = startPoint.i
let Cr0         = (SIMD8<Double>(0,1,2,3,4,5,6,7) * pixelwidth) + startPoint.x

let Cr0Array = ContiguousArray<SIMD8<Double>>.init(unsafeUninitializedCapacity: linesize)
    {
        buffer, initializedCount in

        let safeBuffer = buffer
        DispatchQueue.concurrentPerform(iterations: linesize)
        {
            x in
            safeBuffer[x] = Cr0 + ( Double(x) * pixelwidth8 )
        }

        initializedCount = linesize
    }

DispatchQueue.concurrentPerform(iterations: linecount)
{
    y in

    let lineaddress = pixelBuffer + (y * linesize)
    let Ci = Ci0 + (Double(y) * pixelheight)
    var pixel:UInt8 = 0

    for x in 0..<linesize
    {
        mandelbrot(Cr: Cr0Array[x],Ci: Ci,pixel:&pixel)
        lineaddress[x] = pixel
    }
}

let stdout  = FileHandle.standardOutput
    stdout.write("P4\n\(width) \(height)\n".data(using:.utf8)!)
    stdout.write(Data(bytesNoCopy: pixelBuffer,
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

// one pixel calculation stepping 8 points at a time

@inline(__always)
func mandelbrot(Cr:SIMD8<Double>,Ci:Double, pixel:inout UInt8)
{
    var Zr:SIMD8<Double> = .zero
    var Zi:SIMD8<Double> = .zero
    var Tr:SIMD8<Double> = .zero
    var Ti:SIMD8<Double> = .zero

    let thresholds  = SIMD8<Double>(repeating:depth)
    let isFalse     = SIMDMask<SIMD8<Double.SIMDMaskScalar>>(repeating: false)
    let ramp        = SIMD8<Int64>([128,64,32,16,8,4,2,1])

    if pixel == 0
    {
        for _ in 0..<iterations/5
        {
            for _ in 0..<5
            {
                onepixel(Cr:Cr,Ci:Ci,Zr:&Zr,Zi:&Zi,Tr:&Tr,Ti:&Ti)
            }

            let result = Tr + Ti .< thresholds

            if result == isFalse
            {
                return
            }
        }
    }
    else
    {
        for _ in 0..<iterations
        {
            onepixel(Cr:Cr,Ci:Ci,Zr:&Zr,Zi:&Zi,Tr:&Tr,Ti:&Ti)
        }
    }
    let cmpresult = Tr + Ti .< thresholds

    let reduced: SIMD8<Int64> = unsafeBitCast(cmpresult,to: SIMD8<Int64>.self)
    let summask: SIMD8<Int64> = ramp & reduced

    pixel = UInt8(summask.wrappedSum())
}
