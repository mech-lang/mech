/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   contributed by Isaac Gouy
   parallel by The Anh Tran
   Updated by Alan McGovern
*/

using System;
using System.Threading;

class SpectralNorm
{
    public class BarrierHandle : System.Threading.WaitHandle
    {
        int current;
        int threads;
        ManualResetEvent handle = new ManualResetEvent (false);

        public BarrierHandle (int threads)
        {
	    this.current = threads;
            this.threads = threads;
        }

        public override bool WaitOne()
        {
            ManualResetEvent h = handle;
            if (Interlocked.Decrement (ref current) > 0) {
                h.WaitOne ();
            }
            else {
                handle = new ManualResetEvent (false);
                Interlocked.Exchange (ref current, threads);
                h.Set ();
                h.Close ();
            }

            return true;
        }
    }
	
    public static void Main(String[] args)
    {
        int n = 2500;
        if (args.Length > 0)
            n = Int32.Parse(args[0]);

        Console.WriteLine("{0:f9}", RunGame(n));
    }
	
    private static double RunGame(int n)
    {
        // create unit vector
        double[] u = new double[n];
        double[] tmp = new double[n];
        double[] v = new double[n];

        for (int i = 0; i < n; i++)
            u[i] = 1.0;

        int nthread = Environment.ProcessorCount;

		BarrierHandle barrier = new BarrierHandle (nthread);
        // objects contain result of each thread
        Approximate[] apx = new Approximate[nthread];

        // thread handle for waiting/joining
        Thread[] threads = new Thread[nthread];

        // create thread and hand out tasks
        int chunk = n / nthread;
        for (int i = 0; i < nthread; i++)
        {
            int r1 = i * chunk;
            int r2 = (i < (nthread - 1)) ? r1 + chunk : n;

            apx[i] = new Approximate(u, v, tmp, r1, r2);
			apx[i].Barrier = barrier;
            threads[i] = new Thread(new ThreadStart(apx[i].Evaluate));
            threads[i].Start();
        }

        // collect results
        double vBv = 0, vv = 0;
        for (int i = 0; i < nthread; i++)
        {
            threads[i].Join();

            vBv += apx[i].m_vBv;
            vv += apx[i].m_vv;
        }

        return Math.Sqrt(vBv / vv);
    }

    private class Approximate
    {
        internal BarrierHandle? Barrier;
        private double[] m_u;
        private double[] m_v;
        private double[] m_tmp;
        private int m_range_begin, m_range_end;

        public double m_vBv = 0, m_vv = 0;

        public Approximate(double[] u, double[] v, double[] tmp, int rbegin, int rend)
        {
            m_u = u;
            m_v = v;
            m_tmp = tmp;

            m_range_begin = rbegin;
            m_range_end = rend;
        }

        public void Evaluate()
        {
            for (int i = 0; i < 10; i++)
            {
                MultiplyAtAv(m_u, m_tmp, m_v);
                MultiplyAtAv(m_v, m_tmp, m_u);
            }

            for (int i = m_range_begin; i < m_range_end; i++)
            {
                m_vBv += m_u[i] * m_v[i];
                m_vv += m_v[i] * m_v[i];
            }
        }

        /* return element i,j of infinite matrix A */
        private static double eval_A(int i, int j)
        {
            int div = (((i + j) * (i + j + 1) >> 1) + i + 1);
            return 1.0 / div;
        }

        /* multiply vector v by matrix A */
        private void MultiplyAv(double[] v, double[] Av)
        {
            for (int i = m_range_begin; i < m_range_end; i++)
            {
                double sum = 0.0;
                for (int j = 0; j < v.Length; j++)
                    sum += eval_A(i, j) * v[j];
                Av[i] = sum;
            }
        }

        /* multiply vector v by matrix A transposed */
        private void MultiplyAtv(double[] v, double[] Atv)
        {
            for (int i = m_range_begin; i < m_range_end; i++)
            {
                double sum = 0.0;
                for (int j = 0; j < v.Length; j++)
                    sum += eval_A(j, i) * v[j];
                Atv[i] = sum;
            }
        }

        /* multiply vector v by matrix A and then by matrix A transposed */
        private void MultiplyAtAv(double[] v, double[] tmp, double[] AtAv)
        {
            MultiplyAv(v, tmp);
            Barrier.WaitOne ();

            MultiplyAtv(tmp, AtAv);
            Barrier.WaitOne ();
        }
    }
}
