//* The Computer Language Benchmarks Game
//   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

//   Contributed by Peperud
//   Modified to reduce memory use by Anthony Lloyd
//   Rewritten with simd. Start reverse after loading the first block
//   Contributed by Thom Kiesewetter
//*
using System.Buffers;
using System.Runtime.CompilerServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;

public static class Program
{
    public delegate int SpanFunc<T>(Span<T> span);

    private const byte GT = (byte)'>', LF = 10;
    private const int LineLength = 61;
    private const int BUFFER_SIZE = 1024 * 1024 * 1;
    private const int writeStart = 64;
    private static readonly byte[][] blocks = new byte[10000][];
    private static int _index;
    public static Thread TaskStep2 = null!;
    public static Thread TaskStep3 = null!;
    private static readonly List<Sequence> Sequences = new();
    private static int _pageIndex;
    private static int _sequenceIndex;

    private static readonly byte[] _complements = 
        //ABCDEFGHIJKLMNOPQRSTUVWXYZ
        " TVGH  CD  M KN   YSAABW R      ".ToASCII();

    private static int _sequenceStep2Done = -1;

    private static byte[] _writeBlock = null!;

    // Vectors is used to remove line feeds from the input and swap the bytes in reverse order
    //{
    //    0, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
    //    1, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 0,
    //    2, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 1, 0,
    //    3, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 2, 1, 0,
    //    4, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 3, 2, 1, 0,
    //    5, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 4, 3, 2, 1, 0,
    //    6, 15, 14, 13, 12, 11, 10, 9, 8, 7, 5, 4, 3, 2, 1, 0,
    //    7, 15, 14, 13, 12, 11, 10, 9, 8, 6, 5, 4, 3, 2, 1, 0,
    //    8, 15, 14, 13, 12, 11, 10, 9, 7, 6, 5, 4, 3, 2, 1, 0,
    //    9, 15, 14, 13, 12, 11, 10, 8, 7, 6, 5, 4, 3, 2, 1, 0,
    //    10, 15, 14, 13, 12, 11, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
    //    11, 15, 14, 13, 12, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
    //    12, 15, 14, 13, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
    //    13, 15, 14, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
    //    14, 15, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
    //    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0
    //};
    private static readonly byte[] _vectors = new byte[(61 + 16) * 16];
    private static readonly ArrayPool<byte> bytePool = ArrayPool<byte>.Shared;


    [MethodImpl(MethodImplOptions.AggressiveOptimization)]
    public static void Main(string[] args)
    {
        TaskStep2 = new Thread(Step2)
            { IsBackground = true, Priority = ThreadPriority.Highest };
        TaskStep2.Start();
        TaskStep3 = new Thread(Step3)
            { IsBackground = true, Priority = ThreadPriority.Highest };
        TaskStep3.Start();
        int bufferPos;
        SpanFunc<byte> Read = Console.OpenStandardInput().Read;
        do
        {
            var buffer = bytePool.Rent(BUFFER_SIZE);
            //var buffer = new byte[BUFFER_SIZE];
            bufferPos = ReadBlock(Read, buffer);
            blocks[_index] = buffer;
            _index++;
        } while (bufferPos == BUFFER_SIZE);

        TaskStep3.Join();
    }

    private static void Step2()
    {
        SetupLFSkipsAndReverseTable();

        _pageIndex = 0;
        var blockOffset = 0;
        while (true)
        {
            while (_pageIndex == _index) Thread.Sleep(0);
            var seq = new Sequence
            {
                pages = new List<byte[]>(), Header = GetHeader(ref blockOffset)
            };
            Sequences.Add(seq);
            ReadSequence(ref blockOffset);
            _sequenceStep2Done++;
            if (blocks[_pageIndex][blockOffset] == 0)
                return;
        }
    }

    private static void SetupLFSkipsAndReverseTable()
    {
        for (var i = 0; i < 16 + 61; i++) // linefeed skips
            _vectors[i] = (byte)(i < 61 ? 16 : 15);
        for (var i = 16 * 16; i < 61 * 16; i++)
            _vectors[i] = (byte)(15 - i % 16);
        var offset = _vectors.AsSpan()[(61 * 16)..];
        for (var i = 0; i < 16; i++)
        for (var j = 0; j < 16; j++)
            offset[i * 16 + j] =
                (byte)(j == 0 ? i : j < 16 - i ? 16 - j : 15 - j);
    }

    private static void ReadSequence(ref int offset)
    {
        var lfPosition = 60;
        while (true)
        {
            var block = blocks[_pageIndex];
            //var reverseBlock = new byte[BUFFER_SIZE];
            var reverseBlock = bytePool.Rent(BUFFER_SIZE);
            Sequences[^1].pages.Add(reverseBlock);
            ParsePage(block, reverseBlock, ref lfPosition, ref offset);
            if (offset != BUFFER_SIZE) return;
            offset = 0;
            bytePool.Return(block);
            _pageIndex++;
            while (_pageIndex == _index) Thread.Sleep(0);
        }
    }

    /* The page is read in block of 16 bytes.
     * The first tree SimD statement will reverse the input and remove the linefeed. The offset should guarantee that we start and the beginning of a line
     * We keep track of the linefeed position. If there is a line feed in 16 byte block. The linefeed is moved at the first position of the 16 byte block
     * and is overwritten in the next 16 byte write (lfSkip=15)
     * To find the complement: The same shuffle trick as other programs is used.
     * - Take only the 5 lower bits of the ascii.
     * - 1e shuffle with 4 lower bits from the ascii to select the complements of the letter o-z
     * - 2e shuffle with 4 lower bits from the ascii to select the complements of the letter a-n
     * - compare to find all byte positions from the 1e shuffle 0-z
     * - blend the three vectors to select all from 2e shuffle + overwrite all selected from 1e shuffle.
     *
     * At the end of the block:
     * - calc the next lfPosition in for the next block
     * - copy the remainder bytes
     * - The write block is oversized because the line feeds are removed. So save the first position where the content start at the beginning of the block.
     * We need this later when we write to output
     */
    private static unsafe void ParsePage(byte[] block, byte[] reverseBlock,
        ref int lfPosition, ref int offset)
    {
        fixed (byte* readBlockPtr = &block[0], writeBlockPrt =
                   &reverseBlock[0], vectorsPtr = &_vectors[0])
        {
            var writePtr = writeBlockPrt + reverseBlock.Length - 16;
            var readPtr = readBlockPtr + offset;
            var endPtr = readBlockPtr + block.Length;
            while (readPtr + Math.Max(16, lfPosition) < endPtr &&
                   *(readPtr + lfPosition) == LF)
            {
                var swapSelector =
                    Sse2.LoadVector128(vectorsPtr + 16 * lfPosition);
                var org = Sse2.LoadVector128(readPtr);
                var swap = Ssse3.Shuffle(org, swapSelector);
                var complementSelector =
                    Sse2.And(swap, Vector128.Create((byte)0x1F));
                var blendMask = Sse2.CompareGreaterThan(
                    complementSelector.AsSByte(),
                    Vector128.Create((byte)0xF).AsSByte());
                var lookupHigh = Vector128.Create(0, 0, (byte)'Y', (byte)'S',
                    (byte)'A', (byte)'A', (byte)'B', (byte)'W', 0, (byte)'R', 0,
                    0, 0, 0, 0, 0);
                var r2 = Ssse3.Shuffle(lookupHigh, complementSelector);
                var lookupLow = Vector128.Create(0, (byte)'T', (byte)'V',
                    (byte)'G', (byte)'H', 0, 0, (byte)'C', (byte)'D', 0, 0,
                    (byte)'M', 0, (byte)'K', (byte)'N', 0);
                var r1 = Ssse3.Shuffle(lookupLow, complementSelector);
                var result = Sse41.BlendVariable(r1, r2, blendMask.AsByte());
                Sse2.Store(writePtr, result.AsByte());
                int lfSkip = *(vectorsPtr + lfPosition);
                readPtr += 16;
                writePtr -= lfSkip;
                lfPosition -= 16;
                if (lfPosition < 16) lfPosition += LineLength;
            }

            if (readPtr + Math.Max(16, lfPosition) >= endPtr) //end of page
            {
                lfPosition -= (int)(endPtr - readPtr);
                if (lfPosition < 16) lfPosition += LineLength;
            }
            else //end of sequence
            {
                lfPosition = 60;
            }

            writePtr += 15;
            byte value;
            while (readPtr < endPtr && (value = *readPtr) != 0 && value != GT)
            {
                if (value != LF)
                {
                    value = _complements[(byte)(value & 0x1f)];
                    *writePtr = value;
                    writePtr--;
                }

                readPtr++;
            }

            // Save position of content start
            var startContent = (int)(writePtr + 1 - writeBlockPrt);
            *(int*)writeBlockPrt = startContent;

            // calculate offset for normal blocks this is 0. But not at the end of a sequence
            offset = (int)(readPtr - readBlockPtr);
        }
    }

    private static byte[] GetHeader(ref int offset)
    {
        var block = blocks[_pageIndex];
        var start = offset;
        var end = Array.IndexOf(block, LF, start);
        offset = end + 1;
        return block[start..(end + 1)];
    }

    private static void Step3()
    {
        CreateWriteTemplate();
        var output = Console.OpenStandardOutput();
        Action<byte[], int, int> Write = output.Write;

        _sequenceIndex = 0;
        while (true)
        {
            while (_sequenceStep2Done < _sequenceIndex)
            {
                Thread.Sleep(0);
                if (!TaskStep2.IsAlive)
                {
                    output.Dispose();
                    return;
                }
            }

            WriteSequence(Sequences[_sequenceIndex], Write);
            _sequenceIndex++;
        }
    }

    private static unsafe void WriteSequence(Sequence sequence,
        Action<byte[], int, int> Write)
    {
        Write(sequence.Header, 0, sequence.Header.Length);
        var firstLinePosition = 0;
        var pages = sequence.pages;
        for (var i = pages.Count - 1; i >= 0; i--)
        {
            var page = pages[i];
            fixed (byte* readBlockPtr = &page[0], writeBlockPtr =
                       &_writeBlock[0])
            {
                var writePtr = writeBlockPtr + writeStart;
                var start = *(int*)readBlockPtr;
                var readPtr = readBlockPtr + start;
                var endPtr = readBlockPtr + BUFFER_SIZE - 60;
                if (firstLinePosition >
                    0) // write the remainder of the first line and repair the LF
                {
                    Avx.Store(writePtr + firstLinePosition + 28,
                        Avx.LoadDquVector256(readPtr + 28));
                    Avx.Store(writePtr + firstLinePosition,
                        Avx.LoadDquVector256(readPtr));
                    *(writePtr + 60) = LF;
                    writePtr += 61;
                    readPtr += 60 - firstLinePosition;
                }

                while (readPtr < endPtr)
                {
                    Avx.Store(writePtr + 28,
                        Avx.LoadDquVector256(readPtr + 28));
                    Avx.Store(writePtr, Avx.LoadDquVector256(readPtr));
                    readPtr += 60;
                    writePtr += 61;
                }

                Write(_writeBlock, writeStart,
                    (int)(writePtr - (writeBlockPtr + writeStart)));
                firstLinePosition = (int)(endPtr + 60 - readPtr);
                writePtr = writeBlockPtr + firstLinePosition;
                Avx.Store(writePtr, Avx.LoadDquVector256(endPtr + 60 - 64));
                Avx.Store(writePtr + 32,
                    Avx.LoadDquVector256(endPtr + 60 - 32));
                bytePool.Return(page);
            }
        }

        if (firstLinePosition > 0)
            Write(_writeBlock, 64, firstLinePosition);
        Write(new[] { LF }, 0, 1);
    }

    private static void CreateWriteTemplate()
    {
        _writeBlock = new byte[BUFFER_SIZE + 64 + 64];
        var j = writeStart + 60;
        while (j < _writeBlock.Length)
        {
            _writeBlock[j] = LF;
            j += 61;
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static int ReadBlock(SpanFunc<byte> Read, byte[] buffer)
    {
        var bufferPos = 0;
        var buf = buffer.AsSpan();
        while (true)
        {
            var bytesRead1 = Read(buf);
            bufferPos += bytesRead1;
            if (bufferPos == BUFFER_SIZE || bytesRead1 == 0) return bufferPos;
            Array.Fill(buffer, (byte)0, bytesRead1, buffer.Length - bytesRead1);
        }
    }

    public static byte[] ToASCII(this string s)
    {
        var r = new byte[s.Length];
        for (var i = 0; i < s.Length; i++)
            r[i] = (byte)s[i];

        return r;
    }
}

public class Sequence
{
    public byte[] Header = null!;
    public List<byte[]> pages = new();
}
