/**
  * The Computer Language Benchmarks Game
  * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
  *
  * Contributed by Derek Ziemba
  * 
  * This is a port based on Miles's "n-body C gcc #9 program"
  */

using System;
using System.Runtime.CompilerServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;

using static System.Runtime.CompilerServices.MethodImplOptions;

using V256d = System.Runtime.Intrinsics.Vector256<double>;

public static unsafe class Net60_NBody_AVX_9_3b {

  [MethodImpl(AggressiveOptimization | AggressiveInlining)]
  private static V256d Square(V256d x)
    => Avx.Multiply(x, x);

  [MethodImpl(AggressiveOptimization | AggressiveInlining)]
  private static V256d Permute2x128AndBlend(V256d t0, V256d t1)
    => Avx.Add(Avx.Permute2x128(t0, t1, 0b10_0001), Avx.Blend(t0, t1, 0b1100));

  [MethodImpl(AggressiveOptimization | AggressiveInlining)][SkipLocalsInit]
  private static void InitDiffs(V256d* positions, V256d* rsqrts) {
    V256d* r = rsqrts, p = positions;
    for (int i = 1, k = 0; i < 5; ++i) {
      V256d pi = p[i];
      for (int j = 0; j < i; ++j, ++k) {
        V256d pj = p[j];
        r[k] = Avx.Subtract(pi, pj);
      }
    }
  }

  [MethodImpl(AggressiveOptimization | AggressiveInlining)][SkipLocalsInit]
  private static V256d FastReciprocalSqRoot(V256d c0375, V256d c1250, V256d c1875, V256d t0, V256d t1) {
    V256d s = Permute2x128AndBlend(t0, t1);
    V256d x = Avx.ConvertToVector256Double(Sse.ReciprocalSqrt(Avx.ConvertToVector128Single(s)));
    V256d y = Avx.Multiply(s, Avx.Multiply(x, x));
    V256d y0 = Avx.Multiply(Avx.Multiply(y, c0375), y);
    V256d y1 = Avx.Subtract(Avx.Multiply(y, c1250), c1875);
    return Avx.Multiply(x, Avx.Subtract(y0, y1));
  }


  [MethodImpl(AggressiveOptimization)][SkipLocalsInit]
  static void Advance(int iterations, double dt, V256d* masses, V256d* positions, V256d* velocities) {
    unchecked {
      V256d* v = velocities, p = positions, m = masses;
      V256d step = Vector256.Create(dt);
      V256d c0375 = Vector256.Create(0.375);
      V256d c1250 = Vector256.Create(1.25);
      V256d c1875 = Vector256.Create(1.875);
      V256d* r = stackalloc V256d[14];
      // Align the memory (C# doesn't have a built in way AFAIK) to prevent fault when calling Avx.LoadAlignedVector256 or Avx.StoreAligned
      r = (V256d*)((((UInt64)r)+31UL)&~31UL);
      double* w = (double*)(r+10);
    ADVANCE:
      InitDiffs(p, r);
      CalcStepDistances(step, c0375, c1250, c1875, r, r+10);
      CalcNewVelocities(v, m, r, w);
      CalcNewPositions(step, p, v);
      --iterations;
      if (iterations > 0) { goto ADVANCE; }


      [MethodImpl(AggressiveOptimization | AggressiveInlining)][SkipLocalsInit]
      static void CalcStepDistances(V256d step, V256d c0375, V256d c1250, V256d c1875, V256d* r, V256d* w) {
        w[0] = TimeAdjust(step, FastReciprocalSqRoot(c0375, c1250, c1875, Avx.HorizontalAdd(Square(r[0]), Square(r[1])), Avx.HorizontalAdd(Square(r[2]), Square(r[3]))));
        w[1] = TimeAdjust(step, FastReciprocalSqRoot(c0375, c1250, c1875, Avx.HorizontalAdd(Square(r[4]), Square(r[5])), Avx.HorizontalAdd(Square(r[6]), Square(r[7]))));
        w[2] = TimeAdjust(step, FastReciprocalSqRoot(c0375, c1250, c1875, Avx.HorizontalAdd(Square(r[8]), Square(r[9])), V256d.Zero));

        [MethodImpl(AggressiveOptimization | AggressiveInlining)][SkipLocalsInit]
        static V256d TimeAdjust(V256d rt, V256d x) => Avx.Multiply(Avx.Multiply(x, x), Avx.Multiply(x, rt));
      }

      [MethodImpl(AggressiveOptimization | AggressiveInlining)][SkipLocalsInit]
      static void CalcNewVelocities(V256d* v, V256d* m, V256d* r, double* w) {
        for (int i = 1; i < 5; ++i) {
          V256d iV = v[i];
          V256d iM = m[i];
          for (int j = 0; j < i; ++j) {
            V256d kW = Avx.BroadcastScalarToVector256(w);
            ++w;
            V256d kR = r[0];
            ++r;
            V256d jM = m[j];
            V256d jV = v[j];
            V256d t = Avx.Multiply(kR, kW);
            V256d jM_t = Avx.Multiply(jM, t);
            V256d iM_t = Avx.Multiply(iM, t);
            iV = Avx.Subtract(iV, jM_t);
            v[j] = Avx.Add(jV, iM_t);
          }
          v[i] = iV;
        }
      }

      [MethodImpl(AggressiveOptimization | AggressiveInlining)][SkipLocalsInit]
      static void CalcNewPositions(V256d step, V256d* p, V256d* v) {
        for (int i = 0; i < 5; ++i) {
          V256d iP = p[i];
          V256d iV = v[i];
          p[i] = Avx.Add(iP, Avx.Multiply(iV, step));
        }
      }
    }
  }

  [SkipLocalsInit]
  static double Energy(double* m, V256d* p, V256d* v) {
    unchecked {
      double e = SumComponents256(
        Avx.Multiply(
          Avx.Multiply(
            Permute2x128AndBlend(
              Avx.HorizontalAdd(Square(v[0]), Square(v[1])),
              Avx.HorizontalAdd(Square(v[2]), Square(v[3]))),
            Avx.LoadAlignedVector256(m)),
          Vector256.Create(0.5)))
        + Permute2x128AndBlend(Avx.HorizontalAdd(Square(v[4]), V256d.Zero), V256d.Zero).GetElement(0) * m[4] * 0.5;


      V256d* r = stackalloc V256d[14];
      // Align the memory (C# doesn't have a built in way AFAIK) to prevent fault when calling Avx.LoadAlignedVector256 or Avx.StoreAligned
      r = (V256d*)((((UInt64)r)+31UL)&~31UL);
      InitDiffs(p, r);

      V256d c0375 = Vector256.Create(0.375), c1250 = Vector256.Create(1.25), c1875 = Vector256.Create(1.875);
      r[10] = FastReciprocalSqRoot(c0375, c1250, c1875, Avx.HorizontalAdd(Square(r[0]), Square(r[1])), Avx.HorizontalAdd(Square(r[2]), Square(r[3])));
      r[11] = FastReciprocalSqRoot(c0375, c1250, c1875, Avx.HorizontalAdd(Square(r[4]), Square(r[5])), Avx.HorizontalAdd(Square(r[6]), Square(r[7])));
      r[12] = FastReciprocalSqRoot(c0375, c1250, c1875, Avx.HorizontalAdd(Square(r[8]), Square(r[9])), V256d.Zero);

      double* w = (double*)(r+10);
      for (int i = 1; i < 5; ++i) {
        double iMass = m[i];
        for (int j = 0; j < i; ++j, ++w) {
          e = e - (iMass * m[j] * w[0]);
        }
      }
      return e;

      [MethodImpl(AggressiveOptimization | AggressiveInlining)]
      static double SumComponents128(Vector128<double> x) => x.GetElement(1) + x.GetElement(0);
      [MethodImpl(AggressiveOptimization | AggressiveInlining)]
      static double SumComponents256(V256d x) => SumComponents128(Avx.Add(x.GetLower(), x.GetUpper()));
    }
  }

  [SkipLocalsInit]
  public static void Main(string[] args) {
    int iterations = args.Length > 0 ? Int32.Parse(args[0]) : 10000;
    if (iterations <= 0) { return; }

    V256d* mem = stackalloc V256d[18];
    // Align the memory (C# doesn't have a built in way AFAIK) to prevent fault when calling Avx.LoadAlignedVector256 or Avx.StoreAligned
    mem = (V256d*)((((UInt64)mem)+31UL)&~31UL);

    InitSystem(mem, out V256d* m, out V256d* p, out V256d* v);

    Console.WriteLine(Energy((double*)mem, p, v).ToString("F9"));

    Advance(iterations, 0.01, m, p, v);

    Console.WriteLine(Energy((double*)mem, p, v).ToString("F9"));


    [SkipLocalsInit]
    static void InitSystem(V256d* mem, out V256d* m, out V256d* p, out V256d* v) {
      const double PI = 3.141592653589793;
      const double SOLAR_MASS = (4 * PI * PI);
      const double DAYS_PER_YEAR = 365.24;

      double* masses = (double*)mem;

      masses[0] = SOLAR_MASS;
      masses[1] = 9.54791938424326609e-04 * SOLAR_MASS;
      masses[2] = 2.85885980666130812e-04 * SOLAR_MASS;
      masses[3] = 4.36624404335156298e-05 * SOLAR_MASS;
      masses[4] = 5.15138902046611451e-05 * SOLAR_MASS;

      m = mem + 2;
      for (int i = 0; i < 5; ++i) { m[i] = Vector256.Create(masses[i]); }

      // positions
      p = mem + 7;
      p[0] = V256d.Zero;
      p[1] = Vector256.Create(0.0, 4.84143144246472090e+00, -1.16032004402742839e+00, -1.03622044471123109e-01);
      p[2] = Vector256.Create(0.0, 8.34336671824457987e+00, 4.12479856412430479e+00, -4.03523417114321381e-01);
      p[3] = Vector256.Create(0.0, 1.28943695621391310e+01, -1.51111514016986312e+01, -2.23307578892655734e-01);
      p[4] = Vector256.Create(0.0, 1.53796971148509165e+01, -2.59193146099879641e+01, 1.79258772950371181e-01);

      // velocities
      v = mem + 12;
      //v[0] = Vector256.Create(-1.0);
      v[1] = Vector256.Create(0.0, 1.66007664274403694e-03 * DAYS_PER_YEAR, 7.69901118419740425e-03 * DAYS_PER_YEAR, -6.90460016972063023e-05 * DAYS_PER_YEAR);
      v[2] = Vector256.Create(0.0, -2.76742510726862411e-03 * DAYS_PER_YEAR, 4.99852801234917238e-03 * DAYS_PER_YEAR, 2.30417297573763929e-05 * DAYS_PER_YEAR);
      v[3] = Vector256.Create(0.0, 2.96460137564761618e-03 * DAYS_PER_YEAR, 2.37847173959480950e-03 * DAYS_PER_YEAR, -2.96589568540237556e-05 * DAYS_PER_YEAR);
      v[4] = Vector256.Create(0.0, 2.68067772490389322e-03 * DAYS_PER_YEAR, 1.62824170038242295e-03 * DAYS_PER_YEAR, -9.51592254519715870e-05 * DAYS_PER_YEAR);

      // Offset Momentmum
      v[0] = Avx.Divide(Avx.Add(Avx.Add(Avx.Add(Avx.Multiply(v[1], m[1]),
                                                Avx.Multiply(v[2], m[2])),
                                        Avx.Multiply(v[3], m[3])),
                                Avx.Multiply(v[4], m[4])),
                        Avx.Multiply(Vector256.Create(-1.0), m[0]));
    }
  }
} //END Class
