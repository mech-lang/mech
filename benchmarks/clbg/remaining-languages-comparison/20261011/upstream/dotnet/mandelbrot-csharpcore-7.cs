// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// direct transliteration of Greg Buchholz's C program
// contributed by Isaac Gouy, fix by David Turnbull
// dispatching to multiple cores, use SIMD operationn, early break
// depending on previous depth by Patrick Stein,
// direct transliteration of Swift #7 program by Arseniy Zlobintsev

using System.Runtime.CompilerServices;
using System.Runtime.Intrinsics;

const int iterations = 50;
const double depth = 4.0;

var width = int.Parse(args[0]);
var height = width;
var startPoint = (x: -1.5, i: -1.0);
var endPoint = (x: 0.5, i: 1.0);

var lineCount = height;
var lineSize = (width + 7) / 8;
var pixelHeight = Math.Abs(endPoint.x - startPoint.x) / height;
var pixelWidth = Math.Abs(endPoint.i - startPoint.i) / width;
var pixelWidth8 = pixelWidth * 8.0;

var outputSize = lineCount * lineSize;
var pixelBuffer = new byte[outputSize];
var Ci0 = startPoint.i;
var Cr0 = (Vector512.Create((double)0, 1, 2, 3, 4, 5, 6, 7)
    * pixelWidth)
    + Vector512.Create(startPoint.x);

var Cr0Array = new Vector512<double>[lineSize];

Parallel.For(0, lineSize, x => {
    Cr0Array[x] = Cr0 + Vector512.Create(x * pixelWidth8);
});

Parallel.For(0, lineCount, y => {
    var slice = pixelBuffer.AsSpan(y * lineSize, lineSize);
    var Ci = Ci0 + (y * pixelHeight);
    var pixel = (byte)0;

    for (var x = 0; x < slice.Length; x++) {
        Mandelbrot(Cr0Array[x], Ci, ref pixel);
        slice[x] = pixel;
    }
});

Console.WriteLine($"P4\n{width} {height}");
Console.OpenStandardOutput().Write(pixelBuffer);

[MethodImpl(MethodImplOptions.AggressiveInlining)]
static void Mandelbrot(Vector512<double> Cr, double Ci, ref byte pixel) {
    var Zr = Vector512<double>.Zero;
    var Zi = Vector512<double>.Zero;
    var Tr = Vector512<double>.Zero;
    var Ti = Vector512<double>.Zero;
    var CiV = Vector512.Create(Ci);

    var thresholds = Vector512.Create(depth);
    var ramp = Vector512.Create(128, 64, 32, 16, 8, 4, 2, 1);

    if (pixel is 0) {
        for (var i = 0; i < iterations / 5; i++) {
            for (var j = 0; j < 5; j++) {
                Zi = 2.0 * Zr * Zi + CiV;
                Zr = Tr - Ti + Cr;
                Tr = Zr * Zr;
                Ti = Zi * Zi;
            }

            var result = Vector512.LessThanAny(Tr + Ti, thresholds);
            if (result is false) return;
        }
    }
    else {
        for (var i = 0; i < iterations; i++) {
            Zi = 2.0 * Zr * Zi + CiV;
            Zr = Tr - Ti + Cr;
            Tr = Zr * Zr;
            Ti = Zi * Zi;
        }
    }

    var cmpresult = Vector512.LessThan(Tr + Ti, thresholds);
    var summask = ramp & cmpresult.AsInt64();

    pixel = (byte)Vector512.Sum(summask);
}
