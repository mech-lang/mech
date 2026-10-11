/* The Computer Language Benchmarks game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  Naive transliteration from Michael Ferguson's Chapel program
  contributed by Isaac Gouy  
*/

final class Body {
   public double x, y, z, vx, vy, vz, mass;
   
   Body(double x, double y, double z, 
         double vx, double vy, double vz, double mass){
      this.x = x;   
      this.y = y;  
      this.z = z;  
      this.vx = vx;   
      this.vy = vy;  
      this.vz = vz;
      this.mass = mass;           
   } 
}   

public final class nbody {
   static double PI = 3.141592653589793;   
   static double SOLAR_MASS = 4 * PI * PI;
   static double DAYS_PER_YEAR = 365.24;
            
   static void offsetMomentum(Body[] bodies) {
      double px = 0.0;
      double py = 0.0;   
      double pz = 0.0;    
      Body b;       
      for(int i=0; i < bodies.length; ++i) {
         b = bodies[i];
         px += b.vx * b.mass;
         py += b.vy * b.mass;      
         pz += b.vz * b.mass;            
      }  
      b = bodies[0];      
      b.vx = -px / SOLAR_MASS;
      b.vy = -py / SOLAR_MASS;
      b.vz = -pz / SOLAR_MASS; 
   }
         
   static double energy(Body[] bodies){           
      double e = 0.0;  
      int numBodies = bodies.length;	      
      for (int i=0; i < numBodies; ++i) {
         Body b = bodies[i];
         double sq = b.vx * b.vx + b.vy * b.vy + b.vz * b.vz;          
         e += 0.5 * bodies[i].mass * sq;          
         for (int j=i+1; j < numBodies; ++j) {
            double dx = b.x - bodies[j].x; 
            double dy = b.y - bodies[j].y; 
            double dz = b.z - bodies[j].z;   
            double dsq = dx * dx + dy * dy + dz * dz;                    
            e -= (b.mass * bodies[j].mass) / Math.sqrt(dsq);
         }
      }
      return e;
   }     
   
   static void advance(Body[] bodies, double dt) {
      int numBodies = bodies.length;	   
      for(int i=0; i < numBodies; ++i) {
         for(int j=i+1; j < numBodies; ++j) {   
            double dx = bodies[i].x - bodies[j].x;
            double dy = bodies[i].y - bodies[j].y;
            double dz = bodies[i].z - bodies[j].z;
            double dsq = dx*dx + dy*dy + dz*dz;
            double mag = dt / (dsq * Math.sqrt(dsq));            

            double mj = bodies[j].mass * mag;
            bodies[i].vx -= dx * mj;
            bodies[i].vy -= dy * mj;
            bodies[i].vz -= dz * mj;
            
            double mi = bodies[i].mass * mag;            
            bodies[j].vx += dx * mi;
            bodies[j].vy += dy * mi;
            bodies[j].vz += dz * mi;
         }
      }
      
      for(int i=0; i < numBodies; ++i) {
         bodies[i].x += bodies[i].vx * dt;
         bodies[i].y += bodies[i].vy * dt;
         bodies[i].z += bodies[i].vz * dt;
      }      
   }          
         
   static void nbody(int n) {    
      var bodies = new Body[]{      
         // sun      
         new Body(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, SOLAR_MASS),        
         // jupiter
         new Body(
            4.84143144246472090e+00,
            -1.16032004402742839e+00,
            -1.03622044471123109e-01,
            1.66007664274403694e-03 * DAYS_PER_YEAR,
            7.69901118419740425e-03 * DAYS_PER_YEAR,
            -6.90460016972063023e-05 * DAYS_PER_YEAR,
            9.54791938424326609e-04 * SOLAR_MASS  
         ),         
         new Body(         
            // saturn
            8.34336671824457987e+00,
            4.12479856412430479e+00,
            -4.03523417114321381e-01,
            -2.76742510726862411e-03 * DAYS_PER_YEAR,
            4.99852801234917238e-03 * DAYS_PER_YEAR,
            2.30417297573763929e-05 * DAYS_PER_YEAR,
            2.85885980666130812e-04 * SOLAR_MASS                 
         ),                
         new Body(         
            // uranus         
            1.28943695621391310e+01,
            -1.51111514016986312e+01,
            -2.23307578892655734e-01,
            2.96460137564761618e-03 * DAYS_PER_YEAR,
            2.37847173959480950e-03 * DAYS_PER_YEAR,
            -2.96589568540237556e-05 * DAYS_PER_YEAR,
            4.36624404335156298e-05 * SOLAR_MASS       
         ), 
         new Body(         
            // neptune              
            1.53796971148509165e+01,
            -2.59193146099879641e+01,
            1.79258772950371181e-01,
            2.68067772490389322e-03 * DAYS_PER_YEAR,
            1.62824170038242295e-03 * DAYS_PER_YEAR,
            -9.51592254519715870e-05 * DAYS_PER_YEAR,
            5.15138902046611451e-05 * SOLAR_MASS  
         )             
      };   
      
      offsetMomentum(bodies);        
      System.out.printf("%.9f\n", energy(bodies));      
      for (int i=0; i<n; ++i) {
         advance(bodies, 0.01);
      }
      System.out.printf("%.9f\n", energy(bodies)); 
   }

   public static void main(String[] args) {     
      int n = Integer.parseInt(args[0]); 
      nbody(n);     
   }
}
