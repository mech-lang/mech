/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   AVX2 version written from scratch by Chris Lomont
*/

using System;
using System.IO;
using System.Numerics;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;

public class MandelBrot
{
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    static unsafe byte GetByte(double* pCrb, double Ciby)
    {
        var res = 0;
        for (var i=0; i<8; i+=2)
        {
            var vCrbx = Unsafe.Read<Vector<double>>(pCrb+i);
            var vCiby = new Vector<double>(Ciby);
            var Zr = vCrbx;
            var Zi = vCiby;
            int b = 0, j = 49;
            do
            {
                for (int counter = 0; counter < 7; counter++)
                {
                    var nZr = Zr * Zr - Zi * Zi + vCrbx;
                    var ZrZi = Zr * Zi;
                    Zi = ZrZi + ZrZi + vCiby;
                    Zr = nZr;
                    j--;
                }

                var t = Zr * Zr + Zi * Zi;
                if (t[0]>4.0) { b|=2; if (b==3) break; }
                if (t[1]>4.0) { b|=1; if (b==3) break; }
            } while (j>0);
            res = (res << 2) + b;
        }
        return (byte)(res^-1);
    }

    public static unsafe void MainOld(string[] args)
    {
        var size = args.Length==0 ? 200 : int.Parse(args[0]);
        Console.Out.WriteAsync(String.Concat("P4\n",size," ",size,"\n"));
        var Crb = new double[size+2];
        var lineLength = size >> 3;
        var data = new byte[size * lineLength];
        fixed (double* pCrb = &Crb[0])
        fixed (byte* pdata = &data[0])
        {
            var value = new Vector<double>(
                  new double[] {0,1,0,0,0,0,0,0}
            );
            var invN = new Vector<double>(2.0/size);
            var onePtFive = new Vector<double>(1.5);
            var step = new Vector<double>(2);
            for (var i=0; i<size; i+=2)
            {
                Unsafe.Write(pCrb+i, value*invN-onePtFive);
                value += step;
            }
            var _Crb = pCrb;
            var _pdata = pdata;
            Parallel.For(0, size, y =>
            {
                var Ciby = _Crb[y]+0.5;
                for (var x=0; x<lineLength; x++)
                {
                    _pdata[y*lineLength+x] = GetByte(_Crb+x*8, Ciby);
                }
            });
            Console.OpenStandardOutput().Write(data, 0, data.Length);
        }
    }

    // x86 version, AVX2
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    static byte Process8(double x, double y, double dx)
    {
        // initial x coords
        var x01 = Vector256.Create(x+0*dx,x+1*dx,x+2*dx,x+3*dx);
        var x02 = Vector256.Create(x+4*dx,x+5*dx,x+6*dx,x+7*dx);
        
        // initial y coords
        var y0  = Vector256.Create(y); 
        
        Vector256<double> x1 = x01,y1 = y0; // current iteration 1
        Vector256<double> x2 = x02,y2 = y0; // current iteration 2

        Vector256<double> four = Vector256.Create(4.0); // 4 in each slot        

        var pass = 0;

        // temp space, C# requires init.
        Vector256<double> 
            x12=Vector256<double>.Zero,
            y12=Vector256<double>.Zero,
            x22=Vector256<double>.Zero,
            y22=Vector256<double>.Zero;

        // bit masks for results
        uint res1=1,res2=1;

        while (pass < 49 && (res1 != 0 || res2 != 0))
        {

            // do several between checks a time like other code
            for (var p = 0 ; p < 7; ++p)
            {
                // unroll loop 2x to decrease register stalls

                // squares x*x and y*y
                x12 = Avx2.Multiply(x1,x1);
                y12 = Avx2.Multiply(y1,y1);
                x22 = Avx2.Multiply(x2,x2);
                y22 = Avx2.Multiply(y2,y2);

                // mixed products x*y
                var xy1 = Avx2.Multiply(x1, y1);
                var xy2 = Avx2.Multiply(x2, y2);

                // diff of squares x*x - y*y
                var ds1 = Avx2.Subtract(x12, y12);
                var ds2 = Avx2.Subtract(x22, y22);

                // 2*x*y
                xy1 = Avx2.Add(xy1, xy1);
                xy2 = Avx2.Add(xy2, xy2);

                // next iters
                y1 = Avx2.Add(xy1, y0);
                y2 = Avx2.Add(xy2, y0);
                x1 = Avx2.Add(ds1, x01);
                x2 = Avx2.Add(ds2, x02);
            }
            pass+=7;

            // numbers overflow, which gives an Infinity or NaN, which, 
            // when compared N < 4, results in false, which is what we want

            // sum of squares x*x + y*y, compare to 4 (escape mandelbrot)
            var ss1  = Avx2.Add(x12, y12);
            var ss2  = Avx2.Add(x22, y22);

            // compare - puts all 0 in reg if false, else all 1 (=NaN bitwise)
            // when each register is 0, then all points escaped, so exit
            var cmp1 = Avx.Compare(ss1,four,
                    FloatComparisonMode.OrderedLessThanOrEqualNonSignaling);
            var cmp2 = Avx.Compare(ss2,four,
                    FloatComparisonMode.OrderedLessThanOrEqualNonSignaling);

            // take top bit from each byte
            res1 = (uint)Avx2.MoveMask(Vector256.AsByte(cmp1));
            res2 = (uint)Avx2.MoveMask(Vector256.AsByte(cmp2));
        }        

        // can make a mask of bits in any order, which is the +7, +6, .., +1, +0
        res1 &= 
            (1<<( 0+7)) |
            (1<<( 8+6)) |
            (1<<(16+5)) |
            (1<<(24+4));
        res2 &= 
            (1<<( 0+3)) |
            (1<<( 8+2)) |
            (1<<(16+1)) |
            (1<<(24+0));

        var res = res1|res2;
        res |= res>>16;
        res |= res>>8;
        return (byte)(res);
    }

    static void Test(byte [] data)
    {
        var filename = "mandelbrot-output.txt";
        if (!File.Exists(filename))
        {
            System.Console.WriteLine($"Cannot open file {filename}");
            return;
        }
        var len = data.Length;
        var truth = File.ReadAllBytes(filename);
        Array.Copy(truth,truth.Length-len,truth,0,len);
        for (var i = 0; i < len; ++i)
        {
            if (data[i] != truth[i])
            {
                var bits = data[i]^truth[i];
                System.Console.Write($"ERROR: Mismatch {i}: {data[i]:X2} != {truth[i]:X2}, ^={bits:X2},");
                var x = ((i*8)%200);
                var y = (i*8)/200;
                while (bits != 0)
                {
                    if ((bits&1) != 0)
                        System.Console.Write($"({x},{y}), ");
                    ++x;
                    bits>>=1;                    
                }
                System.Console.WriteLine();

            }
        }

    }

    static byte Process8a(double x0_, double y0, double delta)
    {
        var ans = 0;
        for (var bit = 0; bit < 8; ++bit)
        {

            var x0 = x0_+delta*bit;
            double x1 = x0, y1 = y0;
            for (var pass = 0; pass < 49; ++pass)
            {
                var xt = x1*x1-y1*y1+x0;
                y1 = 2*x1*y1+y0;
                x1 = xt;
            }
            ans <<= 1;
            if (x1*x1 + y1*y1 <= 4.0)
                ans |= 1;
            x0 += delta;
        }
        return (byte)ans;
    }

    public static void MainNew(string[] args)
    {

        var size = args.Length<2 ? 200 : int.Parse(args[1]);
        Console.Out.WriteAsync(String.Concat("P4\n",size," ",size,"\n"));
        var lineLength = size >> 3;
        var data = new byte[size * lineLength];

        // step size
        var delta = 2.0/size; // (0.5 - (-1.5))/size;

        Parallel.For(0, size, y =>
        {
            var yd = y*delta-1;
            for (var x=0; x<lineLength; x++)
            {
                var xd = (x*8)*delta-1.5;
                data[y*lineLength+x] = Process8(xd,yd,delta);
            }
        }
        );
        //if (size == 200)
        //    Test(data);
        Console.OpenStandardOutput().Write(data, 0, data.Length);
    }


    public static void Main(string[] args)
    {
        if (System.Runtime.Intrinsics.X86.Avx2.IsSupported)
            MainNew(args);
        else
            MainOld(args);
    }
}
