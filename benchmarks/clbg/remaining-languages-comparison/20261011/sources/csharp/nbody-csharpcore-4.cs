/**
  * The Computer Language Benchmarks Game
  * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
  *
  * Contributed by Derek Ziemba
  * 
  * This is nearly a direct port of Miles's "n-body C gcc #9 program"
  * The actual direct port was about 10% slower. 
  * I was able to speed it up primarily by just eliminating variables (C# doesn't handle lots of variables too well)
  */

using System;
using System.Runtime.CompilerServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;

using static System.Runtime.CompilerServices.MethodImplOptions;

using V256d = System.Runtime.Intrinsics.Vector256<double>;
using V128d = System.Runtime.Intrinsics.Vector128<float>;

public static unsafe class NBody_CPP2 {
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V256d _mm256_set1_pd(double x) => Vector256.Create(x);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V256d _mm256_setr_pd(double x, double y, double z, double w) => Vector256.Create(x, y, z, w);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V256d _mm256_load_pd(double* address) => Avx.LoadAlignedVector256(address);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static void _mm256_store_pd(double* address, V256d source) => Avx.StoreAligned(address, source);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V128d _mm_rsqrt_ps(V128d x) => Sse.ReciprocalSqrt(x);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V256d _mm256_cvtps_pd(V128d x) => Avx.ConvertToVector256Double(x);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V128d _mm256_cvtpd_ps(V256d x) => Avx.ConvertToVector128Single(x);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V256d _mm256_mul_pd(V256d x, V256d y) => Avx.Multiply(x, y);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V256d _mm256_sub_pd(V256d x, V256d y) => Avx.Subtract(x, y);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V256d _mm256_add_pd(V256d x, V256d y) => Avx.Add(x, y);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V256d _mm256_hadd_pd(V256d x, V256d y) => Avx.HorizontalAdd(x, y);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V256d _mm256_blend_pd(V256d x, V256d y, byte control) => Avx.Blend(x, y, control);
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | AggressiveInlining)] private static V256d _mm256_permute2f128_pd(V256d x, V256d y, byte control) => Avx.Permute2x128(x, y, control);


  // compute rsqrt of distance between each pair of bodies
  [SkipLocalsInit]
  [MethodImpl(AggressiveOptimization | AggressiveInlining)]
  private static void kernel(V256d* r, double* w, V256d* p) {
    for(int i = 1, k = 0; i < 5; i++) {
      for(int j = 0; j < i; j++, k++) {
        r[k] = _mm256_sub_pd(p[i], p[j]);
      }
    }

    V256d c0375 = _mm256_set1_pd(0.375); 
    V256d c1250 = _mm256_set1_pd(1.25); 
    V256d c1875 = _mm256_set1_pd(1.875); 
    for(int k = 0; k < 10; k += 4) {
      V256d x0 = r[k], x1 = r[k+1], x2 = r[k+2], x3 = r[k+3];
      V256d t0 = _mm256_hadd_pd(_mm256_mul_pd(x0, x0), _mm256_mul_pd(x1, x1));
      V256d t1 = _mm256_hadd_pd(_mm256_mul_pd(x2, x2), _mm256_mul_pd(x3, x3));
      V256d s = _mm256_add_pd(_mm256_permute2f128_pd(t0, t1, 0x21), _mm256_blend_pd(t0, t1, 0b1100));
      V256d x = _mm256_cvtps_pd(_mm_rsqrt_ps(_mm256_cvtpd_ps(s)));
      V256d y = _mm256_mul_pd(s, _mm256_mul_pd(x, x));
      _mm256_store_pd(w + k, _mm256_mul_pd(x, _mm256_sub_pd(_mm256_mul_pd(_mm256_mul_pd(y, c0375), y), _mm256_sub_pd(_mm256_mul_pd(y, c1250), c1875))));
    }
  }



  static void advance(int n, double dt, V256d* m, V256d* p, V256d* v) {
    V256d* mem = stackalloc V256d[16];
    // Align the memory (C# doesn't have a built in way AFAIK) to prevent fault when calling Avx.LoadAlignedVector256 or Avx.StoreAligned
    V256d* r = (V256d*)((((UInt64)mem)+31UL)&~31UL);
    double* w = (double*)(r + 12);

    r[10] = _mm256_set1_pd(1.0);
    r[11] = _mm256_set1_pd(1.0);

    V256d rt = _mm256_set1_pd(dt);

    for(int s = 0; s < n; s++) {
      kernel(r, w, p);

      for(int k = 0; k < 10; k += 4) {
        V256d x = _mm256_load_pd(w + k);
        _mm256_store_pd(w + k, _mm256_mul_pd(_mm256_mul_pd(x, x), _mm256_mul_pd(x, rt)));
      }

      for(int i = 1, k = 0; i < 5; i++)
        for(int j = 0; j < i; j++, k++) {
          V256d t = _mm256_mul_pd(r[k], _mm256_set1_pd(w[k]));
          v[i] = _mm256_sub_pd(v[i], _mm256_mul_pd(t, m[j]));
          v[j] = _mm256_add_pd(v[j], _mm256_mul_pd(t, m[i]));
        }

      for(int i = 0; i < 5; i++) {
        p[i] = _mm256_add_pd(p[i], _mm256_mul_pd(v[i], rt));
      }
    }
  }


  static double energy(V256d* m, V256d* p, V256d* v) {
    double e = 0.0;
    V256d* mem = stackalloc V256d[16];
    // Align the memory (C# doesn't have a built in way AFAIK) to prevent fault when calling Avx.LoadAlignedVector256 or Avx.StoreAligned
    V256d* r = (V256d*)((((UInt64)mem)+31UL)&~31UL); 
    double* w = (double*)(r + 12);


    r[5] = _mm256_set1_pd(0.0);
    r[6] = _mm256_set1_pd(0.0);

    for(int k = 0; k < 5; k++)
      r[k] = Avx.Multiply(v[k], v[k]);

    for(int k = 0; k < 5; k += 4) {
      V256d t0 = _mm256_hadd_pd(r[k], r[k + 1]);
      V256d t1 = _mm256_hadd_pd(r[k + 2], r[k + 3]);
      _mm256_store_pd(w + k, _mm256_add_pd(_mm256_permute2f128_pd(t0, t1, 0x21), _mm256_blend_pd(t0, t1, 0b1100)));
    }

    for(int k = 0; k < 5; k++)
      e += 0.5 * m[k].GetElement(1) * w[k];

    r[10] = _mm256_set1_pd(0.0);
    r[11] = _mm256_set1_pd(0.0);

    kernel(r, w, p);

    for(int i = 1, k = 0; i < 5; i++)
      for(int j = 0; j < i; j++, k++)
        e -= m[i].GetElement(1) * m[j].GetElement(1) * w[k];

    return e;
  }



  public static void Main(string[] args) {
    int iterations = args.Length > 0 ? Int32.Parse(args[0]) : 10000;
    if (iterations <= 0) { return; }

    const double PI = 3.141592653589793;
    const double SOLAR_MASS = (4 * PI * PI);
    const double DAYS_PER_YEAR = 365.24;

    V256d* mem = stackalloc V256d[16];
    // Align the memory (C# doesn't have a built in way AFAIK) to prevent fault when calling Avx.LoadAlignedVector256 or Avx.StoreAligned
    V256d* p = (V256d*)((((UInt64)mem)+31UL)&~31UL); 

    // positions
    p[0] = V256d.Zero;
    p[1] = _mm256_setr_pd(0.0, 4.84143144246472090e+00, -1.16032004402742839e+00, -1.03622044471123109e-01);
    p[2] = _mm256_setr_pd(0.0, 8.34336671824457987e+00, 4.12479856412430479e+00, -4.03523417114321381e-01);
    p[3] = _mm256_setr_pd(0.0, 1.28943695621391310e+01, -1.51111514016986312e+01, -2.23307578892655734e-01);
    p[4] = _mm256_setr_pd(0.0, 1.53796971148509165e+01, -2.59193146099879641e+01, 1.79258772950371181e-01);

    // velocities
    V256d* v = p+5; 
    //v[0] = _mm256_set1_pd(-1.0);
    v[1] = _mm256_setr_pd(0.0, 1.66007664274403694e-03 * DAYS_PER_YEAR, 7.69901118419740425e-03 * DAYS_PER_YEAR, -6.90460016972063023e-05 * DAYS_PER_YEAR);
    v[2] = _mm256_setr_pd(0.0, -2.76742510726862411e-03 * DAYS_PER_YEAR, 4.99852801234917238e-03 * DAYS_PER_YEAR, 2.30417297573763929e-05 * DAYS_PER_YEAR);
    v[3] = _mm256_setr_pd(0.0,  2.96460137564761618e-03 * DAYS_PER_YEAR, 2.37847173959480950e-03 * DAYS_PER_YEAR, -2.96589568540237556e-05 * DAYS_PER_YEAR);
    v[4] = _mm256_setr_pd(0.0, 2.68067772490389322e-03 * DAYS_PER_YEAR,  1.62824170038242295e-03 * DAYS_PER_YEAR,  -9.51592254519715870e-05 * DAYS_PER_YEAR);

    // masses
    V256d* m = p+10; 
    m[0] = _mm256_set1_pd(SOLAR_MASS);
    m[1] = _mm256_set1_pd(9.54791938424326609e-04 * SOLAR_MASS);
    m[2] = _mm256_set1_pd(2.85885980666130812e-04 * SOLAR_MASS);
    m[3] = _mm256_set1_pd(4.36624404335156298e-05 * SOLAR_MASS);
    m[4] = _mm256_set1_pd(5.15138902046611451e-05 * SOLAR_MASS);

    // Offset Momentmum
    v[0] = Avx.Divide(Avx.Add(Avx.Add(Avx.Add(Avx.Multiply(v[1], m[1]),
                                              Avx.Multiply(v[2], m[2])),
                                      Avx.Multiply(v[3], m[3])),
                              Avx.Multiply(v[4], m[4])),
                      Avx.Multiply(_mm256_set1_pd(-1.0), m[0]));


    Console.WriteLine(energy(m, p, v).ToString("F9"));
    advance(iterations, 0.01, m, p, v);
    Console.WriteLine(energy(m, p, v).ToString("F9"));
  }


} //END Class
