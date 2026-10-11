/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Mark C. Lewis
   double[] instead of Object[] by Han Kai
*/

public final class nbody {
   public static void main(String[] args) {
      int n = Integer.parseInt(args[0]);

      NBodySystem bodies = new NBodySystem();
      System.out.printf("%.9f\n", bodies.energy());
      for (int i = 0; i < n; ++i)
         bodies.advance(0.01);
      System.out.printf("%.9f\n", bodies.energy());
   }

   final static class NBodySystem {
      private static final double PI = 3.141592653589793;
      private static final double SOLAR_MASS = 4 * PI * PI;
      private static final double DAYS_PER_YEAR = 365.24;
      private static final int BODY_SIZE = 8;
      private static final int BODY_COUNT = 5;

      private static final int x = 0;
      private static final int y = 1;
      private static final int z = 2;
      private static final int vx = 3;
      private static final int vy = 4;
      private static final int vz = 5;
      private static final int mass = 6;

      private final double[] _bodies = {
            //sun begin
            0, 0, 0, 0, 0, 0, SOLAR_MASS, 0,
            //sun end

            //jupiter begin
            4.84143144246472090e+00,//
            -1.16032004402742839e+00,//
            -1.03622044471123109e-01,//
            1.66007664274403694e-03 * DAYS_PER_YEAR,//
            7.69901118419740425e-03 * DAYS_PER_YEAR,//
            -6.90460016972063023e-05 * DAYS_PER_YEAR,//
            9.54791938424326609e-04 * SOLAR_MASS,//
            0,
            //jupiter end

            //saturn begin
            8.34336671824457987e+00,//
            4.12479856412430479e+00,//
            -4.03523417114321381e-01,//
            -2.76742510726862411e-03 * DAYS_PER_YEAR,//
            4.99852801234917238e-03 * DAYS_PER_YEAR,//
            2.30417297573763929e-05 * DAYS_PER_YEAR,//
            2.85885980666130812e-04 * SOLAR_MASS,//
            0,
            //saturn end

            //uranus begin
            1.28943695621391310e+01,//
            -1.51111514016986312e+01,//
            -2.23307578892655734e-01,//
            2.96460137564761618e-03 * DAYS_PER_YEAR,//
            2.37847173959480950e-03 * DAYS_PER_YEAR,//
            -2.96589568540237556e-05 * DAYS_PER_YEAR,//
            4.36624404335156298e-05 * SOLAR_MASS,//
            0,
            //uranus end

            //neptune begin
            1.53796971148509165e+01,//
            -2.59193146099879641e+01,//
            1.79258772950371181e-01,//
            2.68067772490389322e-03 * DAYS_PER_YEAR,//
            1.62824170038242295e-03 * DAYS_PER_YEAR,//
            -9.51592254519715870e-05 * DAYS_PER_YEAR,//
            5.15138902046611451e-05 * SOLAR_MASS, //
            0
            //neptune end
      };

      public NBodySystem() {
         double px = 0.0;
         double py = 0.0;
         double pz = 0.0;

         for (int i = 0; i < BODY_COUNT; ++i) {
            final int ioffset = BODY_SIZE * i;
            double imass = _bodies[ioffset + mass];

            px += _bodies[ioffset + vx] * imass;
            py += _bodies[ioffset + vy] * imass;
            pz += _bodies[ioffset + vz] * imass;
         }

         _bodies[vx] = -px / SOLAR_MASS;
         _bodies[vy] = -py / SOLAR_MASS;
         _bodies[vz] = -pz / SOLAR_MASS;
      }

      public void advance(double dt) {
         final double[] bodies = _bodies;

         for (int i = 0; i < BODY_COUNT; ++i) {
            final int offset = BODY_SIZE * i;

            for (int j = i + 1; j < BODY_COUNT; ++j) {
               final int ioffset = offset;
               final int joffset = BODY_SIZE * j;

               final double dx = bodies[ioffset + x] - bodies[joffset + x];
               final double dy = bodies[ioffset + y] - bodies[joffset + y];
               final double dz = bodies[ioffset + z] - bodies[joffset + z];

               final double dSquared = dx * dx + dy * dy + dz * dz;
               final double distance = Math.sqrt(dSquared);
               final double mag = dt / (dSquared * distance);

               final double jmass = bodies[joffset + mass];

               bodies[ioffset + vx] -= dx * jmass * mag;
               bodies[ioffset + vy] -= dy * jmass * mag;
               bodies[ioffset + vz] -= dz * jmass * mag;

               final double imass = bodies[ioffset + mass];
               bodies[joffset + vx] += dx * imass * mag;
               bodies[joffset + vy] += dy * imass * mag;
               bodies[joffset + vz] += dz * imass * mag;
            }
         }

         for (int i = 0; i < BODY_COUNT; ++i) {
            final int ioffset = BODY_SIZE * i;

            bodies[ioffset + x] += dt * bodies[ioffset + vx];
            bodies[ioffset + y] += dt * bodies[ioffset + vy];
            bodies[ioffset + z] += dt * bodies[ioffset + vz];
         }
      }

      public double energy() {
         final double[] bodies = _bodies;

         double dx, dy, dz, distance;
         double e = 0.0;

         for (int i = 0; i < BODY_COUNT; ++i) {
            final int offset = BODY_SIZE * i;

            final double ivx = bodies[offset + vx];
            final double ivy = bodies[offset + vy];
            final double ivz = bodies[offset + vz];
            final double imass = bodies[offset + mass];

            e += 0.5 * imass * (ivx * ivx + ivy * ivy + ivz * ivz);

            for (int j = i + 1; j < BODY_COUNT; ++j) {
               final int ioffset = offset;
               final int joffset = BODY_SIZE * j;

               final double ix = bodies[ioffset + x];
               final double iy = bodies[ioffset + y];
               final double iz = bodies[ioffset + z];

               dx = ix - bodies[joffset + x];
               dy = iy - bodies[joffset + y];
               dz = iz - bodies[joffset + z];

               distance = Math.sqrt(dx * dx + dy * dy + dz * dz);
               e -= (imass * bodies[joffset + mass]) / distance;
            }
         }

         return e;
      }

   }

}
