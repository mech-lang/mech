/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Contributed by Peperud
   Modified by Anthony Lloyd
*/

using System;
using System.IO;
using System.Collections.Generic;
using System.Collections.Concurrent;
using System.Threading;

class page { public byte[] data; public int length; }
class sequence { public List<page> pages; public int startHeader, endExclusive; }

public static class revcomp
{
    const int READER_BUFFER_SIZE = 1024*1024*16;
    const int POOL_ADD = 8;
    static BlockingCollection<page> readQue = new BlockingCollection<page>();
    static BlockingCollection<sequence> groupQue = new BlockingCollection<sequence>();
    static BlockingCollection<sequence> writeQue = new BlockingCollection<sequence>();
    static ConcurrentBag<byte[]> bytePool = new ConcurrentBag<byte[]>(); 

    static byte[] borrowBuffer()
    {
        byte[] ret;
        return bytePool.TryTake(out ret) ? ret : new byte[READER_BUFFER_SIZE];
    }

    static void returnBuffer(byte[] bytes)
    {
        bytePool.Add(bytes);
    }

    static void Pooler()
    {
        for(int i=0; i<POOL_ADD; i++) bytePool.Add(new byte[READER_BUFFER_SIZE]);
    }

    static void Reader()
    {
        using (var stream = Console.OpenStandardInput())
        {
            for (;;)
            {
                var buffer = borrowBuffer();
                var bytesRead = stream.Read(buffer, 0, READER_BUFFER_SIZE);
                if (bytesRead == 0) break;
                readQue.Add(new page { data = buffer, length = bytesRead });
            }
            readQue.CompleteAdding();
        }
    }

    static bool tryTake<T>(BlockingCollection<T> q, out T t) where T : class
    {
        t = null;
        while(!q.IsCompleted && !q.TryTake(out t, 100));
        return t!=null;
    }

    static void Grouper()
    {
        const byte GT = (byte)'>';
        var startHeader = 0;
        var i = 1;
        var data = new List<page>();
        page page;
        while (tryTake(readQue, out page))
        {
            data.Add(page);
            var bytes = page.data;
            var l = page.length;
            for (; i<l; i++)
            {
                if (bytes[i] == GT)
                {
                    groupQue.Add(new sequence { pages = data, startHeader = startHeader, endExclusive = i });
                    startHeader = i;
                    data = new List<page> { page };
                }
            }
            i = 0;
        }
        groupQue.Add(new sequence { pages = data, startHeader = startHeader, endExclusive = data[data.Count-1].length });
        groupQue.CompleteAdding();
    }

    static void Reverser()
    {
        const byte LF = 10;

        // Set up complements map
        var map = new byte[256];
        for (byte i=0; i<255; i++) map[i]=i;
        map[(byte)'A'] = (byte)'T';
        map[(byte)'B'] = (byte)'V';
        map[(byte)'C'] = (byte)'G';
        map[(byte)'D'] = (byte)'H';
        map[(byte)'G'] = (byte)'C';
        map[(byte)'H'] = (byte)'D';
        map[(byte)'K'] = (byte)'M';
        map[(byte)'M'] = (byte)'K';
        map[(byte)'R'] = (byte)'Y';
        map[(byte)'T'] = (byte)'A';
        map[(byte)'V'] = (byte)'B';
        map[(byte)'Y'] = (byte)'R';
        map[(byte)'a'] = (byte)'T';
        map[(byte)'b'] = (byte)'V';
        map[(byte)'c'] = (byte)'G';
        map[(byte)'d'] = (byte)'H';
        map[(byte)'g'] = (byte)'C';
        map[(byte)'h'] = (byte)'D';
        map[(byte)'k'] = (byte)'M';
        map[(byte)'m'] = (byte)'K';
        map[(byte)'r'] = (byte)'Y';
        map[(byte)'t'] = (byte)'A';
        map[(byte)'v'] = (byte)'B';
        map[(byte)'y'] = (byte)'R';

        sequence sequence;
        while (tryTake(groupQue, out sequence))
        {
            var startPageId = 0;
            var startPage = sequence.pages[0];
            var startIndex = sequence.startHeader;

            // Skip header line
            do
            {
                if (++startIndex == startPage.length)
                {
                    startPage = sequence.pages[++startPageId];
                    startIndex = 0;
                }
            } while (startPage.data[startIndex] != LF);

            var endPageId = sequence.pages.Count - 1;
            var endIndex = sequence.endExclusive - 1;
            if(endIndex==-1) endIndex = sequence.pages[--endPageId].length-1;
            var endPage = sequence.pages[endPageId];
            
            // Swap in place across pages
            do
            {
                var startByte = startPage.data[startIndex];
                if(startByte==LF)
                {
                    if (++startIndex == startPage.length)
                    {
                        startPage = sequence.pages[++startPageId];
                        startIndex = 0;
                    }
                    if (startIndex == endIndex && startPageId == endPageId) break;
                    startByte = startPage.data[startIndex];
                }
                if(endIndex<0 || endIndex>=endPage.data.Length) Console.Error.WriteLine(endIndex);
                var endByte = endPage.data[endIndex];
                if(endByte==LF)
                {
                    if (--endIndex == -1)
                    {
                        endPage = sequence.pages[--endPageId];
                        endIndex = endPage.length - 1;
                    }
                    if (startIndex == endIndex && startPageId == endPageId) break;
                    endByte = endPage.data[endIndex];
                }

                startPage.data[startIndex] = map[endByte];
                endPage.data[endIndex] = map[startByte];

                if (++startIndex == startPage.length)
                {
                    startPage = sequence.pages[++startPageId];
                    startIndex = 0;
                }
                if (--endIndex == -1)
                {
                    endPage = sequence.pages[--endPageId];
                    endIndex = endPage.length - 1;
                }
            } while (startPageId < endPageId || (startPageId == endPageId && startIndex < endIndex));
            if (startIndex == endIndex) startPage.data[startIndex] = map[startPage.data[startIndex]];
            writeQue.Add(sequence);
        }
        writeQue.CompleteAdding();
    }

    static void Writer()
    {
        using (var stream = Console.OpenStandardOutput())
        {
            sequence sequence;
            while (tryTake(writeQue, out sequence))
            {
                var startIndex = sequence.startHeader;
                var pages = sequence.pages;

                for (int i = 0; i < pages.Count - 1; i++)
                {
                    var page = pages[i];
                    stream.Write(page.data, startIndex, page.length - startIndex);
                    returnBuffer(page.data);
                    startIndex = 0;
                }
                stream.Write(pages[pages.Count - 1].data, startIndex, sequence.endExclusive - startIndex);
            }
        }
    }

    public static void Main(string[] args)
    {
        new Thread(Reader).Start();
        new Thread(Pooler).Start();
        new Thread(Grouper).Start();
        new Thread(Reverser).Start();
        Writer();
    }
}
