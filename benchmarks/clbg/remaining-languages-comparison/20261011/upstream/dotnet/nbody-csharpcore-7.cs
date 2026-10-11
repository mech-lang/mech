/**
  * The Computer Language Benchmarks Game
  * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
  *
  * Contributed by Derek Ziemba
  *
  * Optimized using:
  * - Unmanaged stack allocated structs (zero memory allocation)
  * - AVX Vector Intrinsics targeting IvyBridge ix-3xxx series cpus
  * - Using GOTO which currently causes dotnet to generate better machine code
  */

using System;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;

using static System.Runtime.CompilerServices.MethodImplOptions;

using V256d = System.Runtime.Intrinsics.Vector256<double>;

public static unsafe class NBody {

  [SkipLocalsInit][StructLayout(LayoutKind.Explicit, Pack = 32)]
  struct Body {
    [FieldOffset(0)] public V256d Position;
    [FieldOffset(32)] public V256d Velocity;
    [FieldOffset(64)] public V256d Mass;
  }

  [SkipLocalsInit]
  public static void Main(string[] args) {
    unchecked {
      Body* system = stackalloc Body[5];

      InitSystem(system);

      Console.WriteLine(Energy(system).ToString("F9"));

      RunSimulation(system, Vector256.Create(0.01d).WithElement(3, 0d), args.Length > 0 ? Int32.Parse(args[0]) : 10000);

      Console.WriteLine(Energy(system).ToString("F9"));
    }
  }


  // prevent inlining into main to decrease JIT time to generate main
  [SkipLocalsInit][MethodImpl(NoInlining)] 
  private static void InitSystem(Body* system) {
    unchecked {
      const double SOLAR_MASS = 4 * Math.PI * Math.PI;
      const double DAYS_PER_YEAR = 365.24;

      /**** SUN ****/
      system[0].Position = V256d.Zero;
      system[0].Velocity = Vector256.Create(-1d).WithElement(3, 0d);
      system[0].Mass = Vector256.Create(SOLAR_MASS).WithElement(3, 0d);

      /**** JUPITER ****/
      system[1].Position = Vector256.Create(4.84143144246472090e+00, -1.16032004402742839e+00, -1.03622044471123109e-01, 0d);
      system[1].Velocity = Vector256.Create(1.66007664274403694e-03 * DAYS_PER_YEAR, 7.69901118419740425e-03 * DAYS_PER_YEAR, -6.90460016972063023e-05 * DAYS_PER_YEAR, 0d);
      system[1].Mass = Vector256.Create(9.54791938424326609e-04 * SOLAR_MASS).WithElement(3, 0d);

      /**** SATURN ****/
      system[2].Position = Vector256.Create(8.34336671824457987e+00, 4.12479856412430479e+00, -4.03523417114321381e-01, 0d);
      system[2].Velocity = Vector256.Create(-2.76742510726862411e-03 * DAYS_PER_YEAR, 4.99852801234917238e-03 * DAYS_PER_YEAR, 2.30417297573763929e-05 * DAYS_PER_YEAR, 0d);
      system[2].Mass = Vector256.Create(2.85885980666130812e-04 * SOLAR_MASS).WithElement(3, 0d);

      /**** URANUS ****/
      system[3].Position = Vector256.Create(1.28943695621391310e+01, -1.51111514016986312e+01, -2.23307578892655734e-01, 0d);
      system[3].Velocity = Vector256.Create(2.96460137564761618e-03 * DAYS_PER_YEAR, 2.37847173959480950e-03 * DAYS_PER_YEAR, -2.96589568540237556e-05 * DAYS_PER_YEAR, 0d);
      system[3].Mass = Vector256.Create(4.36624404335156298e-05 * SOLAR_MASS).WithElement(3, 0d);

      /**** NEPTUNE ****/
      system[4].Position = Vector256.Create(1.53796971148509165e+01, -2.59193146099879641e+01, 1.79258772950371181e-01, 0d);
      system[4].Velocity = Vector256.Create(2.68067772490389322e-03 * DAYS_PER_YEAR, 1.62824170038242295e-03 * DAYS_PER_YEAR, -9.51592254519715870e-05 * DAYS_PER_YEAR, 0d);
      system[4].Mass = Vector256.Create(5.15138902046611451e-05 * SOLAR_MASS).WithElement(3, 0d);

      // Offset Momentmum
      system->Velocity =
        Avx.Divide(Avx.Add(Avx.Add(Avx.Add(Avx.Multiply((system+1)->Velocity, (system+1)->Mass),
                                            Avx.Multiply((system+2)->Velocity, (system+2)->Mass)),
                                    Avx.Multiply((system+3)->Velocity, (system+3)->Mass)),
                            Avx.Multiply((system+4)->Velocity, (system+4)->Mass)),
                    Avx.Multiply(system->Velocity, system->Mass)).WithElement(3, 0d);
    }
  }


  // prevent inlining into main to decrease JIT time to generate main
  // don't aggressively optimize because only entered twice for relativly short time
  [SkipLocalsInit][MethodImpl(NoInlining)]
  private static double Energy(Body* bi) {
    unchecked {
      double e = 0.0;
      for (Body* last = bi + 4; bi <= last; ++bi) {
        V256d iPos = bi->Position;
        V256d iVel = bi->Velocity;
        V256d square = Avx.Multiply(iVel, iVel);
        double sum = square.GetElement(1) + square.GetElement(0) + square.GetElement(2);
        double iMass = bi->Mass.ToScalar();
        e = e + 0.5 * iMass * sum;

        for (Body* bj = bi + 1; bj <= last; ++bj) {
          V256d dx = Avx.Subtract(iPos, bj->Position);
          V256d sq = Avx.Multiply(dx, dx);
          e = e
            - iMass
            * bj->Mass.ToScalar()
            / Math.Sqrt(sq.GetElement(1) + sq.GetElement(0) + sq.GetElement(2));
        }//END of j loop
      }//END of i loop
      return e;
    }
  }

  // prevent inlining into main to decrease JIT time to generate main
  // spend time aggressively optimizing since method is only entered once
  [SkipLocalsInit][MethodImpl(AggressiveOptimization | NoInlining)]
  private static void RunSimulation(Body* system, V256d stepV, int iterations) {
    unchecked {    
      // Using GOTO because actual loops cause pushing and popping vars on stack
ADVANCE:
      Body* bi = system;
OUTER_LOOP:
      Body* bj = bi + 1;    
      // move from stack into ymm registers
      V256d iPos = bi->Position;                  
      V256d iVel = bi->Velocity;                 
      V256d iMass = bi->Mass;                  
INNER_LOOP:      
      V256d jPos = bj->Position;                  
      V256d jVel = bj->Velocity;
      V256d jMass = bj->Mass;
      V256d dx = Avx.Subtract(jPos, iPos);
      V256d square = Avx.Multiply(dx, dx);
      // The order the elements are added matters. (1, 0, 3, 2) gives best codegen
      double sum = (square.GetElement(1) + square.GetElement(0)) + square.GetElement(2);                
      V256d mag = Avx.Multiply(dx, Vector256.Create(stepV.GetElement(1) / (Math.Sqrt(sum) * sum)));
      bj->Velocity = Avx.Subtract(jVel, Avx.Multiply(mag, iMass));
      iVel = Avx.Add(iVel, Avx.Multiply(mag, jMass));                          
      if(++bj <= system + 4) { goto INNER_LOOP; }

      bi->Position = Avx.Add(iPos, Avx.Multiply(iVel, stepV));
      bi->Velocity = iVel;
      if(++bi < system + 4) { goto OUTER_LOOP; }

      bi->Position = Avx.Add(bi->Position, Avx.Multiply(bi->Velocity, stepV));

      if(--iterations > 0) { goto ADVANCE; }
    }
  }

} //END Class
