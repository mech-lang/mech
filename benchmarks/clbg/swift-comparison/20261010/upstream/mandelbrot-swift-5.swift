/* The Computer Language Benchmarks Game
 https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

 direct transliteration of Greg Buchholz's C program
 contributed by Isaac Gouy, fix by David Turnbull
 dispatching to multiple cores by Patrick Stein
 */

#if os(Linux)
import Glibc
import Dispatch
#else
import Foundation
#endif

let width: Int = Int(CommandLine.arguments[1])!
let height = width

let iter = 50, limit = 2.0

print("P4\n\(width) \(height)")

let linesize = (width + (width % 8 != 0 ? 8 : 0 )) / 8
let dispatchGroup = DispatchGroup()


var outputbits:[UInt8] = [UInt8](repeating:0, count: linesize*height)
var counter = 0
var height_d = Double(height)

for y in 0..<height
{
    var address = counter
    counter += linesize

    dispatchGroup.enter()
    DispatchQueue.global().async
    {
        var Zr, Zi, Cr, Ci, Tr, Ti: Double
        var byte_acc:UInt8  = 0
        var bit_num = 0
        Ci = 2.0*Double(y)/height_d - 1.0
        let twodivwidth = 2.0 / Double(width)

        for x in 0..<width
        {
            Zr = 0.0; Zi = 0.0; Tr = 0.0; Ti = 0.0
            Cr = twodivwidth*Double(x) - 1.5;

            var i = 0
            while i < iter && (Tr+Ti <= limit*limit)
            {
                i += 1
                Zi = 2.0*Zr*Zi + Ci
                Zr = Tr - Ti + Cr
                Tr = Zr * Zr
                Ti = Zi * Zi
            }

            byte_acc <<= 1
            if Tr+Ti <= limit*limit { byte_acc |= 0x01 }

            bit_num += 1

            if bit_num == 8 {
                outputbits[address] = byte_acc
                address += 1
                byte_acc = 0
                bit_num = 0
            }
        }
        if bit_num != 0
        {
             byte_acc <<= (8-width%8)
             outputbits[address] = byte_acc
        }

        dispatchGroup.leave()
    }
}

dispatchGroup.wait()

counter = 0
for _ in 0..<height
{
    for x in 0..<linesize
    {
        putc(Int32(outputbits[counter+x]),stdout)
    }
    counter += linesize
}
