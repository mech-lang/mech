/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 
   contributed by Jesper Meyer
*/

using System;
using System.Runtime.CompilerServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;
using System.Threading.Tasks;

namespace SpectralNorm
{
    unsafe class Program
    {
        public static void Main(string[] args)
        {
            int n = 100;
            if (args.Length > 0) n = int.Parse(args[0]);

            fixed (double* u = new double[n])
            fixed (double* v = new double[n])
            {
                new Span<double>(u, n).Fill(1);
                for (var i = 0; i < 10; i++)
                {
                    mult_AtAv(u, v, n);
                    mult_AtAv(v, u, n);
                }

                var result = Math.Sqrt(dot(u, v, n) / dot(v, v, n));
                Console.WriteLine("{0:f9}", result);
            }
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        private static double A(int i, int j)
        {
            return (i + j) * (i + j + 1) / 2 + i + 1;
        }

        private static double dot(double* v, double* u, int n)
        {
            double sum = 0;
            for (var i = 0; i < n; i++)
                sum += v[i] * u[i];
            return sum;
        }

        [MethodImpl(MethodImplOptions.AggressiveOptimization)]
        private static void mult_Av(double* v, double* outv, int n)
        {
            Parallel.For(0, n, i =>
            {
                var sum = Vector128<double>.Zero;
                for (var j = 0; j + 1 < n; j += 2)
                {
                    var b = Vector128.Load(v + j);
                    var a = Vector128.Create(A(i, j), A(i, j + 1));
                    sum = Vector128.Add(sum, Vector128.Divide(b, a));
                }

                var add = Vector128.Create(sum.GetElement(0) + sum.GetElement(1));
                var value = Unsafe.As<Vector128<double>, double>(ref add);
                if ((n & 1) != 0) value += v[n - 1] / A(i, n - 1);
                Unsafe.WriteUnaligned(outv + i, value);
            });
        }

        [MethodImpl(MethodImplOptions.AggressiveOptimization)]
        private static void mult_Atv(double* v, double* outv, int n)
        {
            Parallel.For(0, n, i =>
            {
                var sum = Vector128<double>.Zero;
                for (var j = 0; j + 1 < n; j += 2)
                {
                    var b = Vector128.Load(v + j);
                    var a = Vector128.Create(A(j, i), A(j + 1, i));
                    sum = Vector128.Add(sum, Vector128.Divide(b, a));
                }

                var add = Vector128.Create(sum.GetElement(0) + sum.GetElement(1));
                var value = Unsafe.As<Vector128<double>, double>(ref add);
                if ((n & 1) != 0) value += v[n - 1] / A(n - 1, i);
                Unsafe.WriteUnaligned(outv + i, value);
            });
        }

        private static void mult_AtAv(double* v, double* outv, int n)
        {
            fixed (double* tmp = new double[n])
            {
                mult_Av(v, tmp, n);
                mult_Atv(tmp, outv, n);
            }
        }
    }
}
