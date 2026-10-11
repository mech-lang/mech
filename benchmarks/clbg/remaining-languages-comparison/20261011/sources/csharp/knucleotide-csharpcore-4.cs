/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * byte processing, C# 3.0 idioms, frame level paralellism by Robert F. Tobler
 */

using System;
using System.IO;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading;

public struct ByteString : IEquatable<ByteString>
{
    public byte[] Array;
    public int Start;
    public int Length;

    public ByteString(byte[] array, int start, int length)
    {
        Array = array; Start = start; Length = length;
    }
    
    public ByteString(string text)
    {
        Start = 0; Length = text.Length;
        Array = Encoding.ASCII.GetBytes(text);
    }
    
    public override int GetHashCode()
    {
        int hc = 0;
        for (int i = 0; i < Length; i++)
            hc = hc * 31 + Array[Start + i];
        return hc;
    }

    public bool Equals(ByteString other)
    {
        if (Length != other.Length) return false;
        for (int i = 0; i < Length; i++)
            if (Array[Start+i] != other.Array[other.Start+i]) return false;
        return true;
    }
    
    public override string ToString()
    {
        return Encoding.ASCII.GetString(Array, Start, Length);
    }
}

public static class Extensions
{
    public static byte[] GetBytes(this List<string> lines)
    {
        int count = lines.Aggregate(0, (cnt, str) => cnt + str.Length);
        var array = new byte[count];
        lines.Aggregate(0, (pos, str) => {
                Encoding.ASCII.GetBytes(str, 0, str.Length, array, pos);
                return pos + str. Length;
            });
        return array;
    }
}

public class Program
{
    public static int TaskCount;
    public static int Current = -1;
    public static KNucleotide[] kna;

    public static void Main(string[] args) {
        string line;
        StreamReader source = new StreamReader(Console.OpenStandardInput());
        var input = new List<string>();
    
        while ( (line = source.ReadLine() ) != null )
            if (line[0] == '>' && line.Substring(1, 5) == "THREE")
                break;
    
        while ( (line = source.ReadLine()) != null ) {
            char c = line[0];
            if (c == '>') break;
            if (c != ';') input.Add(line.ToUpper());
        }
    
        var lengths = new [] { 1, 2, 3, 4, 6, 12, 18 };
        
        TaskCount = lengths.Aggregate(0, (cnt, len) => cnt + len);
        kna = new KNucleotide[TaskCount];

        var bytes = input.GetBytes();        
        lengths.Aggregate(0, (cnt, len) => 
            {
                for (int i = 0; i < len; i++)
                    kna[cnt + i] = new KNucleotide(bytes, len, i); 
                return cnt + len;
            });

        var threads = new Thread[Environment.ProcessorCount];
        for (int i = 0; i < threads.Length; i++)
            (threads[i] = new Thread(CountFrequencies)).Start();

        foreach (var t in threads)
            t.Join();

        var seqs = new[] { null, null,
                "GGT", "GGTA", "GGTATT", "GGTATTTTAATT",
                "GGTATTTTAATTTATAGT"};

        int index = 0;
        lengths.Aggregate(0, (cnt, len) =>
            {
                if (len < 3)
                {
                    for (int i = 1; i < len; i++)
                        kna[cnt].AddFrequencies(kna[cnt+i]);
                    kna[cnt].WriteFrequencies();                    
                }
                else
                {
                    var fragment = seqs[index];
                    int freq = 0;
                    for (int i = 0; i < len; i++)
                        freq += kna[cnt + i].GetCount(fragment);
                    Console.WriteLine("{0}\t{1}", freq, fragment);
                }
                index++;
                return cnt + len;
            });
    }
    
    static void CountFrequencies()
    {
        int index;
        while ((index = Interlocked.Increment(ref Current)) < TaskCount)
            kna[index].KFrequency();
    }
    
}

public class KNucleotide {

    private class Count {
       public int V;
       public Count(int v) { V = v; }
    }

    private Dictionary<ByteString, Count> frequencies
        = new Dictionary<ByteString, Count>();
    private byte[] sequence;
    int length;
    int frame;

    public KNucleotide(byte[] s, int l, int f)
    {   
        sequence = s; length = l; frame = f;
    }

    public void AddFrequencies(KNucleotide other)
    {
        foreach(var kvp in other.frequencies)            
        {
            Count count;
            if (frequencies.TryGetValue(kvp.Key, out count))
                count.V += kvp.Value.V;
            else
                frequencies[kvp.Key] = kvp.Value;
        }
    }

    public void WriteFrequencies() {
        var items = new List<KeyValuePair<ByteString, Count>>(frequencies);
        items.Sort(SortByFrequencyAndCode);    
        double percent = 100.0 / (sequence.Length - length + 1);
        foreach (var item in items)
            Console.WriteLine("{0} {1:f3}",
                        item.Key.ToString(), item.Value.V * percent);
        Console.WriteLine();
    }

    public int GetCount(string fragment) {
        Count count;
        if (!frequencies.TryGetValue(new ByteString(fragment), out count))
            count = new Count(0);
        return count.V;
    }

    public void KFrequency() {
        int n = sequence.Length - length + 1;
        for (int i = frame; i < n; i += length) {
            var key = new ByteString(sequence, i, length);
            Count count;
            if (frequencies.TryGetValue(key, out count))
                count.V++;
            else
                frequencies[key] = new Count(1);
        }
    }

    int SortByFrequencyAndCode(
            KeyValuePair<ByteString, Count> i0,
            KeyValuePair<ByteString, Count> i1) {
        int order = i1.Value.V.CompareTo(i0.Value.V);
        if (order != 0) return order;
        return i0.Key.ToString().CompareTo(i1.Key.ToString());
    }
}
