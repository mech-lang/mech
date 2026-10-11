/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Isaac Gouy, transliterated from Oleg Mazurov's Java program
   concurrency fix and minor improvements by Peperud
   parallel and small optimisations by Anthony Lloyd
*/

using System;
using System.Threading;

public static class FannkuchRedux
{
    static int taskCount;
    static int[] fact, chkSums, maxFlips;
    static void firstPermutation(int[] p, int[] pp, int[] count, int idx)
    {
        for (int i=0; i<p.Length; ++i) p[i] = i;
        for (int i=count.Length-1; i>0; --i)
        {
            int d = idx/fact[i];
            count[i] = d;
            if(d>0)
            {
                idx = idx%fact[i];
                for (int j=i ;j>=0; --j) pp[j] = p[j];
                for (int j = 0; j <= i; ++j) p[j] = pp[(j+d)%(i+1)];
            }
        }
    }

    static int nextPermutation(int[] p, int[] count)
    {
        int first = p[1];
        p[1] = p[0];
        p[0] = first;
        int i = 1;
        while (++count[i] > i)
        {
            count[i++] = 0;
            int next = p[1];
            p[0] = next;
            for(int j=1;j<i;) p[j] = p[++j];
            p[i] = first;
            first = next;
        }
        return first;
    }

    static int countFlips(int first, int[] p, int[] pp)
    {
        if (first==0) return 0;
        if (p[first]==0) return 1;
        for(int i=0; i<pp.Length; i++) pp[i] = p[i];
        int flips = 2;
        while(true)
        {
            for (int lo=1,hi=first-1; lo<hi; lo++,hi--)
            {
                int t = pp[lo];
                pp[lo] = pp[hi];
                pp[hi] = t;
            }
            int tp = pp[first];
            if (pp[tp]==0) return flips;
            pp[first] = first;
            first = tp;
            flips++;
        }
    }

    static void run(int n, int taskSize)
    {
        int[] p = new int[n], pp = new int[n], count = new int[n];
        int taskId = 0, chksum = 0, maxflips = 0;
        while((taskId = Interlocked.Decrement(ref taskCount)) >= 0)
        {
            firstPermutation(p, pp, count, taskId*taskSize);
            var flips = countFlips(p[0], p, pp);
            chksum += flips;
            if(flips>maxflips) maxflips = flips;
            for(int i=1; i<taskSize; i++)
            {
                flips = countFlips(nextPermutation(p, count), p, pp);
                chksum += (1-(i%2)*2) * flips;
                if(flips>maxflips) maxflips = flips;
            }
        }
        chkSums[-taskId-1] = chksum;
        maxFlips[-taskId-1] = maxflips;
    }

    public static void Main(string[] args)
    {
        int n = args.Length > 0 ? int.Parse(args[0]) : 7;
        fact = new int[n+1];
        fact[0] = 1;
        var factn = 1;
        for (int i=1; i<fact.Length; i++) { fact[i] = factn *= i; }

        taskCount = n>9 ? factn/(7*6*5*4*3*2) : Environment.ProcessorCount;
        int taskSize = factn/taskCount;
        int nThreads = Environment.ProcessorCount;
        chkSums = new int[nThreads];
        maxFlips = new int[nThreads];
        var threads = new Thread[nThreads];
        for(int i=1; i<nThreads; i++)
        {
            (threads[i] = new Thread(() => run(n, taskSize))).Start();
        }
        run(n, taskSize);
        int chksum=chkSums[0], maxflips=maxFlips[0];
        for(int i=1; i<threads.Length; i++)
        {
            threads[i].Join();
            chksum += chkSums[i];
            if(maxFlips[i]>maxflips) maxflips = maxFlips[i];
        }
        Console.Out.WriteLineAsync(chksum+"\nPfannkuchen("+n+") = "+maxflips);
    }
}
