// The Computer Language Benchmarks Game
// https://benchmarksgame-team.pages.debian.net/benchmarksgame/

// submitted by Josh Goldfoot
// Modified to reduce memory and do more in parallel by Anthony Lloyd
// Added dictionary incrementor by Anthony Lloyd
// Use DictionarySlim by Anthony Lloyd

using System;
using System.IO;
using System.Text;
using System.Linq;
using System.Collections.Generic;
using System.Threading.Tasks;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

public static class KNucleotide
{
    const int BLOCK_SIZE = 1024 * 1024 * 8;
    public static List<byte[]> threeBlocks = new List<byte[]>();
    public static int threeStart, threeEnd;
    static readonly byte[] tonum = new byte[256];
    static readonly char[] tochar = new char[] { 'A', 'C', 'G', 'T' };

    static int Read(Stream stream, byte[] buffer, int offset, int count)
    {
        var bytesRead = stream.Read(buffer, offset, count);
        return bytesRead == count ? offset + count
             : bytesRead == 0 ? offset
             : Read(stream, buffer, offset + bytesRead, count - bytesRead);
    }

    static int Find(byte[] buffer, byte[] toFind, int i, ref int matchIndex)
    {
        if (matchIndex == 0)
        {
            i = Array.IndexOf(buffer, toFind[0], i);
            if (i == -1) return -1;
            matchIndex = 1;
            return Find(buffer, toFind, i + 1, ref matchIndex);
        }
        else
        {
            int bl = buffer.Length, fl = toFind.Length;
            while (i < bl && matchIndex < fl)
            {
                if (buffer[i++] != toFind[matchIndex++])
                {
                    matchIndex = 0;
                    return Find(buffer, toFind, i, ref matchIndex);
                }
            }
            return matchIndex == fl ? i : -1;
        }
    }

    public static void LoadThreeData() {
        using var input = new StreamReader(Console.OpenStandardInput(), Encoding.ASCII);
        bool selected = false;
        var buffer = new byte[BLOCK_SIZE];
        int offset = 0;
        string line;
        while ((line = input.ReadLine()) != null) {
            if (line.StartsWith(">")) {
                if (selected) break;
                selected = line.StartsWith(">THREE");
            } else if (selected) {
                foreach (byte symbol in Encoding.ASCII.GetBytes(line.Trim())) {
                    buffer[offset++] = symbol;
                    if (offset == BLOCK_SIZE) {
                        threeBlocks.Add(buffer);
                        buffer = new byte[BLOCK_SIZE];
                        offset = 0;
                    }
                }
            }
        }
        threeStart = 0;
        threeEnd = offset;
        threeBlocks.Add(buffer);
        if (threeBlocks.Count == 1) {
            for (int i = offset; i < buffer.Length; i++) buffer[i] = 255;
            threeBlocks.Add(Array.Empty<byte>());
            threeEnd = 0;
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static void Check(Dictionary<long, int> dict, ref long rollingKey, byte nb, long mask)
    {
        if (nb == 255) return;
        rollingKey = ((rollingKey & mask) << 2) | nb;
        CollectionsMarshal.GetValueRefOrAddDefault(dict, rollingKey, out _)++;
    }

    static Task<string> Count(int l, long mask, Func<Dictionary<long, int>, string> summary)
    {
        return Task.Run(() =>
        {
            long rollingKey = 0;
            var firstBlock = threeBlocks[0];
            var start = threeStart;
            while (--l > 0) rollingKey = (rollingKey << 2) | firstBlock[start++];
            var dict = new Dictionary<long, int>(1024);
            
            for (int i = start; i < firstBlock.Length; i++)
                Check(dict, ref rollingKey, firstBlock[i], mask);

            int lastBlockId = threeBlocks.Count - 1;
            for (int bl = 1; bl < lastBlockId; bl++)
            {
                var bytes = threeBlocks[bl];
                for (int i = 0; i < bytes.Length; i++)
                    Check(dict, ref rollingKey, bytes[i], mask);
            }

            var lastBlock = threeBlocks[lastBlockId];
            for (int i = 0; i < threeEnd; i++)
                Check(dict, ref rollingKey, lastBlock[i], mask);
            
            return summary(dict);
        });
    }

    static string WriteFrequencies(Dictionary<long, int> freq, int fragmentLength)
    {
        var sb = new StringBuilder();
        double percent = 100.0 / freq.Select(x => x.Value).Sum();
        foreach (var kv in freq.OrderByDescending(i => i.Value).ThenBy(i => i.Key))
        {
            var keyChars = new char[fragmentLength];
            var key = kv.Key;
            for (int i = keyChars.Length - 1; i >= 0; --i)
            {
                keyChars[i] = tochar[key & 0x3];
                key >>= 2;
            }
            sb.Append(keyChars);
            sb.Append(" ");
            sb.AppendLine((kv.Value * percent).ToString("F3"));
        }
        return sb.ToString();
    }

    static string WriteCount(Dictionary<long, int> dictionary, string fragment)
    {
        long key = 0;
        for (int i = 0; i < fragment.Length; ++i)
            key = (key << 2) | tonum[fragment[i]];
        dictionary.TryGetValue(key, out var v);
        return string.Concat(v.ToString(), "\t", fragment);
    }

    public static void Main(string[] args)
    {
        tonum['c'] = 1; tonum['C'] = 1;
        tonum['g'] = 2; tonum['G'] = 2;
        tonum['t'] = 3; tonum['T'] = 3;
        tonum['\r'] = 255; tonum['\n'] = 255; tonum['>'] = 255; tonum[255] = 255;

        LoadThreeData();

        Parallel.ForEach(threeBlocks, bytes =>
        {
            for (int i = 0; i < bytes.Length; i++)
                bytes[i] = tonum[bytes[i]];
        });

        var task18 = Count(18, 17179869183, d => WriteCount(d, "GGTATTTTAATTTATAGT"));
        var task12 = Count(12, 4194303, d => WriteCount(d, "GGTATTTTAATT"));
        var task6 = Count(6, 0b1111111111, d => WriteCount(d, "GGTATT"));
        var task1 = Count(1, 0, d => WriteFrequencies(d, 1));
        var task2 = Count(2, 0b11, d => WriteFrequencies(d, 2));
        var task3 = Count(3, 0b1111, d => WriteCount(d, "GGT"));
        var task4 = Count(4, 0b111111, d => WriteCount(d, "GGTA"));

        Console.Out.WriteLineAsync(task1.Result);
        Console.Out.WriteLineAsync(task2.Result);
        Console.Out.WriteLineAsync(task3.Result);
        Console.Out.WriteLineAsync(task4.Result);
        Console.Out.WriteLineAsync(task6.Result);
        Console.Out.WriteLineAsync(task12.Result);
        Console.WriteLine(task18.Result);
    }
}
