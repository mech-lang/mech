///* The Computer Language Benchmarks Game
//   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

// Contributed by Thom Kiesewetter based on C Gcc #6
// which in turn was based C++ g++ #6" by Ilya Kurdyukov,
// which in turn was based on the C Gcc #6 by Andrei Simion (with patch from Vincent Yu)
// which in turn was based on the C++ program by Dave Compton,
// which in turn was based on the C program by Jeremy Zerfasm
// which in turn was based on the Ada program by Jonathan Parker and 
// Georg Bauhaus which in turn was based on code by Dave Fladebo, 
// Eckehard Berns, Heiner Marxen, Hongwei Xi, and The Anh Tran and 
// also the Java program by Oleg Mazurov.
//*/

using System.Numerics;
using System.Runtime.CompilerServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;

public class Program
{
    private const int MAX_N = 16;
    private static readonly int[] _factorials = new int[MAX_N + 1];
    private static int _n;
    private static int _checksum;
    private static byte _maxFlips;
    private static int _blockCount;
    private static int _blockSize;

    [MethodImpl(MethodImplOptions.AggressiveOptimization)]
    private static void Main(string[] args)
    {
        _n = args.Length > 0 ? int.Parse(args[0]) : 12;

        // Start Setup
        var factorials = _factorials;
        factorials[0] = 1;
        var factN = 1;
        for (var x = 0; x < MAX_N;)
        {
            factN *= ++x;
            factorials[x] = factN;
        }

        // End Setup
        // Thread Setup
        var nThreads = 4;
        var maxBlocks = 96 / 4;
        _blockCount = maxBlocks * nThreads;
        _blockSize = factorials[_n] / _blockCount;
        var threads = new Thread[nThreads];
        for (var i = 1; i < nThreads; i++)
            (threads[i] = new Thread(() => pfannkuchThread()) { IsBackground = true, Priority = ThreadPriority.Highest }).Start();
        Console.Out.Write("");
        pfannkuchThread();
        for (var i = 1; i < threads.Length; i++)
            threads[i].Join();
        Console.Out.WriteLineAsync(_checksum+ "\nPfannkuchen(" + _n + ") = " + _maxFlips);
    }

    [MethodImpl(MethodImplOptions.AggressiveOptimization)]
    private static void pfannkuchThread()
    {
        var masks_shift = new Vector128<byte>[16];
        var c0 = Vector128<byte>.Zero;
        var c1 = Vector128.Create((byte)1);
        var ramp = Vector128.Create((byte)0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
        var ramp1 = Sse2.ShiftRightLogical128BitLane(ramp, 1);
        var vX = Sse2.Subtract(c0, ramp);
        var old = ramp;
        for (var x = 0; x < MAX_N; x++)
        {
            var v2 = Sse41.BlendVariable(vX, ramp, vX);
            var v1 = Sse41.BlendVariable(ramp1, v2, Sse2.Subtract(vX, c1));
            old = Ssse3.Shuffle(old, v1);
            masks_shift[x] = old;
            vX = Sse2.Add(vX, c1);
        }

        var checksum = 0;
        var maxFlips = 0;
        int blockId;
        var n = _n;
        var factorials = _factorials;
        var blockSize = _blockSize;
        while ((blockId = Interlocked.Decrement(ref _blockCount)) >= 0)
        {
            // First permutation in block
            var next = ramp;
            var i = n;
            var j = blockSize * blockId;
            var countVector = c0;
            var blockLeft = blockSize;
            var mask = Sse2.Subtract(ramp, Vector128.Create((byte)i));
            while (i-- > 0)
            {
                var d = j / factorials[i];
                j -= d * factorials[i];
                var v2 = Vector128.Create((byte)d);
                countVector = Ssse3.AlignRight(countVector, v2, 15);
                var v1 = Sse2.Add(ramp, v2);
                var v0 = Sse2.Add(mask, v2); // ramp - i + d
                v0 = Sse41.BlendVariable(v0, v1, v0);
                v2 = Ssse3.Shuffle(next, v0);
                next = Sse41.BlendVariable(next, v2, mask);
                mask = Sse2.Add(mask, c1);
            }


            do
            {
                var current = next;
                var v0 = Sse2.Subtract(countVector, ramp);
                var bits = BitOperations.TrailingZeroCount(Sse2.MoveMask(v0));
                v0 = Vector128.Create((byte)bits);
                var v1 = Sse2.AndNot(Sse2.CompareGreaterThan(v0.AsSByte(), ramp.AsSByte()).AsByte(), countVector);
                countVector = Sse2.Subtract(v1, Sse2.CompareEqual(v0, ramp));
                next = Ssse3.Shuffle(next, masks_shift[bits]);
                var first = Sse2.ConvertToInt32(current.AsInt32());
                {
                    var flips = 0;
                    var v3 = Ssse3.Shuffle(current, c0);
                    while ((first & 0xff) != 0)
                    {
                        v0 = Sse2.Subtract(v3, ramp);
                        v3 = Ssse3.Shuffle(current, v3);
                        v0 = Sse41.BlendVariable(v0, ramp, v0);
                        current = Ssse3.Shuffle(current, v0);
                        flips++;
                        first = Sse2.ConvertToInt32(v3.AsInt32());
                    }

                    checksum += flips;
                    if (flips > maxFlips) maxFlips = flips;
                }

                --blockLeft;
                if (blockLeft == 0) break;
                current = next;
                v0 = Sse2.Subtract(countVector, ramp);
                bits = (byte)BitOperations.TrailingZeroCount(Sse2.MoveMask(v0));
                v0 = Vector128.Create((byte)bits);
                v1 = Sse2.AndNot(Sse2.CompareGreaterThan(v0.AsSByte(), ramp.AsSByte()).AsByte(), countVector);
                countVector = Sse2.Subtract(v1, Sse2.CompareEqual(v0, ramp));
                next = Ssse3.Shuffle(next, masks_shift[bits]);
                first = Sse2.ConvertToInt32(current.AsInt32());
                {
                    var flips = 0;
                    var v3 = Ssse3.Shuffle(current, c0);
                    while ((first & 0xff) != 0)
                    {
                        v0 = Sse2.Subtract(v3, ramp);
                        v3 = Ssse3.Shuffle(current, v3);
                        v0 = Sse41.BlendVariable(v0, ramp, v0);
                        current = Ssse3.Shuffle(current, v0);
                        flips++;
                        first = Sse2.ConvertToInt32(v3.AsInt32());
                    }

                    checksum -= flips;
                    if (flips > maxFlips) maxFlips = flips;
                }

                --blockLeft;
            } while (blockLeft != 0);
        }

        Interlocked.Add(ref _checksum, checksum);
        if (maxFlips > _maxFlips) _maxFlips = (byte)maxFlips;
        if (maxFlips > _maxFlips) _maxFlips = (byte)maxFlips;
    }
}
