/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Isaac Gouy, transliterated from Oleg Mazurov's Java program
*/

using System;
using System.Threading;


class FannkuchRedux
{
   private static int NCHUNKS = 150;
   private static int CHUNKSZ;
   private static int NTASKS;
   private static int n;
   private static int[] Fact;
   private static int[] maxFlips;
   private static int[] chkSums;
   private static int taskId;

   int[] p, pp, count;


   void FirstPermutation( int idx )
   {
       for ( int i=0; i<p.Length; ++i ) {
          p[i] = i;
       }

       for ( int i=count.Length-1; i>0; --i ) {
           int d = idx / Fact[i];
           count[i] = d;
           idx = idx % Fact[i];

           Array.Copy( p, 0, pp, 0, i+1 );
           for ( int j=0; j<=i; ++j ) {
               p[j] = j+d <= i ? pp[j+d] : pp[j+d-i-1];
           }
       }
   }


   bool NextPermutation()
   {
      int first = p[1];
      p[1] = p[0];
      p[0] = first;

      int i=1;
      while ( ++count[i] > i ) {
         count[i++] = 0;
         int next = p[0] = p[1];
         for ( int j=1; j<i; ++j ) {
            p[j] = p[j+1];
         }
         p[i] = first;
         first = next;
      }
      return true;
   }


   int CountFlips()
   {
      int flips = 1;
      int first = p[0];
      if ( p[first] != 0 ) {
         Array.Copy( p, 0, pp, 0, pp.Length );
         do {
            ++flips;
            for ( int lo = 1, hi = first - 1; lo < hi; ++lo, --hi ) {
               int t = pp[lo];
               pp[lo] = pp[hi];
               pp[hi] = t;
            }
            int tp = pp[first];
            pp[first] = first;
            first = tp;
         } while ( pp[first] != 0 );
      }
      return flips;
   }


   void RunTask( int task )
   {
      int idxMin = task*CHUNKSZ;
      int idxMax = Math.Min( Fact[n], idxMin+CHUNKSZ );

      FirstPermutation( idxMin );

      int maxflips = 1;
      int chksum = 0;
      for ( int i=idxMin;; ) {

         if ( p[0] != 0 ) {
            int flips = CountFlips();
            maxflips = Math.Max( maxflips, flips );
            chksum += i%2 ==0 ? flips : -flips;
         }

         if ( ++i == idxMax ) {
	    break;
	 }

         NextPermutation();
      }
      maxFlips[task] = maxflips;
      chkSums[task]  = chksum;
   }


   public void Run()
   {
      p     = new int[n];
      pp    = new int[n];
      count = new int[n];

      int task;
      while ( (task = taskId++) < NTASKS ) { // NOT SAFE - need PFX
	 RunTask( task );       
      } 
   }


   static void Main(string[] args)
   {
      n = 7;
      if (args.Length > 0) n = Int32.Parse(args[0]);

      Fact = new int[n+1];
      Fact[0] = 1;
      for ( int i=1; i<Fact.Length; ++i ) {
         Fact[i] = Fact[i-1] * i;
      }

      CHUNKSZ = (Fact[n] + NCHUNKS - 1) / NCHUNKS;
      NTASKS = (Fact[n] + CHUNKSZ - 1) / CHUNKSZ;
      maxFlips = new int[NTASKS];
      chkSums  = new int[NTASKS];
      taskId = 0;

      int nthreads = Environment.ProcessorCount;
      Thread[] threads = new Thread[nthreads];
      for ( int i=0; i<nthreads; ++i ) {
         threads[i] = new Thread( new ThreadStart(new FannkuchRedux().Run) );
         threads[i].Start();
      }
      foreach ( Thread t in threads ) {
         t.Join();
      }

      int res = 0;
      foreach ( int v in maxFlips ) {
         res = Math.Max( res, v );
      }
      int chk = 0;
      foreach ( int v in chkSums ) {
         chk += v;
      }

      Console.WriteLine("{0}\nPfannkuchen({1}) = {2}", chk, n, res);
   }
}
