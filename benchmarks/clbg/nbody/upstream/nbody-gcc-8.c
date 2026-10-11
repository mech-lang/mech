/* The Computer Language Benchmarks game

   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  Naive transliteration from Michael Ferguson's Chapel program
  contributed by Isaac Gouy  
*/

#include <stdio.h>
#include <stdlib.h>
#include <math.h>

#define pi 3.141592653589793
#define solar_mass (4 * pi * pi)
#define days_per_year 365.24

struct planet {
  double x, y, z;
  double vx, vy, vz;
  double mass;
};

void offset_momentum(int nbodies, struct planet * bodies)
{
    double px = 0.0, py = 0.0, pz = 0.0;
    for (int i = 0; i < nbodies; i++) {
        struct planet * b = &(bodies[i]);    
        px += b->vx * b->mass;
        py += b->vy * b->mass;
        pz += b->vz * b->mass;
    }
    struct planet * b = &(bodies[0]);    
    b->vx = - px / solar_mass;
    b->vy = - py / solar_mass;
    b->vz = - pz / solar_mass;
}

double energy(int nbodies, struct planet * bodies)
{
    double e = 0.0; 
    for (int i = 0; i < nbodies; i++) {   
        struct planet * b = &(bodies[i]);
        double sq = b->vx * b->vx + b->vy * b->vy + b->vz * b->vz; 
        e += 0.5 * b->mass * sq;    
        for (int j=i+1; j < nbodies; ++j) {
            struct planet * b2 = &(bodies[j]);        
            double dx = b->x - b2->x; 
            double dy = b->y - b2->y;
            double dz = b->z - b2->z;            
            double dsq = dx * dx + dy * dy + dz * dz;
            e -= (b->mass * b2->mass) / sqrt(dsq);
        }            
    }
    return e;
}    

void advance(int nbodies, struct planet * bodies, double dt)
{
    for (int i = 0; i < nbodies; i++) { 
        struct planet * b = &(bodies[i]);    
        for (int j=i+1; j < nbodies; ++j) { 
            struct planet * b2 = &(bodies[j]);        
            double dx = b->x - b2->x; 
            double dy = b->y - b2->y;
            double dz = b->z - b2->z;  
            double dsq = dx * dx + dy * dy + dz * dz; 
            double mag = dt / (dsq * sqrt(dsq));            
    
            double mj = b2->mass * mag;
            b->vx -= dx * mj;
            b->vy -= dy * mj;
            b->vz -= dz * mj;            
            
            double mi = b->mass * mag;            
            b2->vx += dx * mi;
            b2->vy += dy * mi;
            b2->vz += dz * mi;            
        }
    }            
    for (int i = 0; i < nbodies; i++) { 
        struct planet * b = &(bodies[i]);    
        b->x += b->vx * dt;
        b->y += b->vy * dt;
        b->z += b->vz * dt;
    }    
}

void nbody(int n)
{
    #define NBODIES 5
    struct planet bodies[NBODIES] = {
    {                               /* sun */
        0, 0, 0, 0, 0, 0, solar_mass
    },
    {                               /* jupiter */
        4.84143144246472090e+00,
        -1.16032004402742839e+00,
        -1.03622044471123109e-01,
        1.66007664274403694e-03 * days_per_year,
        7.69901118419740425e-03 * days_per_year,
        -6.90460016972063023e-05 * days_per_year,
        9.54791938424326609e-04 * solar_mass
    },
    {                               /* saturn */
        8.34336671824457987e+00,
        4.12479856412430479e+00,
        -4.03523417114321381e-01,
        -2.76742510726862411e-03 * days_per_year,
        4.99852801234917238e-03 * days_per_year,
        2.30417297573763929e-05 * days_per_year,
        2.85885980666130812e-04 * solar_mass
    },
    {                               /* uranus */
        1.28943695621391310e+01,
        -1.51111514016986312e+01,
        -2.23307578892655734e-01,
        2.96460137564761618e-03 * days_per_year,
        2.37847173959480950e-03 * days_per_year,
        -2.96589568540237556e-05 * days_per_year,
        4.36624404335156298e-05 * solar_mass
    },
    {                               /* neptune */
        1.53796971148509165e+01,
        -2.59193146099879641e+01,
        1.79258772950371181e-01,
        2.68067772490389322e-03 * days_per_year,
        1.62824170038242295e-03 * days_per_year,
        -9.51592254519715870e-05 * days_per_year,
        5.15138902046611451e-05 * solar_mass
    }
    };

    offset_momentum(NBODIES,bodies);
    printf ("%.9f\n", energy(NBODIES,bodies));  
    for (int i=0; i<n; ++i)
        advance(NBODIES,bodies, 0.01);    
    printf ("%.9f\n", energy(NBODIES,bodies));    
}

int main(int argc, char *argv[])
{
    int n = argc > 1 ? atoi(argv[1]) : 1000;
    nbody(n);
    return 0;
}
