/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   Naive transliteration from Michael Ferguson's Chapel program
   contributed by Isaac Gouy   
*/

const PI = Math.PI;
const SOLAR_MASS = 4 * PI * PI;
const DAYS_PER_YEAR = 365.24;

class Body {
    constructor(x, y, z, vx, vy, vz, mass) {
        this.x = x;
        this.y = y;
        this.z = z;
        this.vx = vx;
        this.vy = vy;
        this.vz = vz;
        this.mass = mass;
    } 
}

function offsetMomentum(bodies) { 
    let px = py = pz = 0;
    for (const b of bodies) {
        px += b.vx * b.mass;
        py += b.vy * b.mass;
        pz += b.vz * b.mass;
    }
    const b = bodies[0];
    b.vx = -px / SOLAR_MASS;
    b.vy = -py / SOLAR_MASS;
    b.vz = -pz / SOLAR_MASS;
}

function energy(bodies) {
    let e = 0;
    const numBodies = bodies.length;
    for (let i = 0; i < numBodies; i++) {
        const b = bodies[i];
        const sq = b.vx * b.vx + b.vy * b.vy + b.vz * b.vz;    
        e += 0.5 * bodies[i].mass * sq;
        for (let j = i + 1; j < numBodies; j++) {
            const dx = b.x - bodies[j].x; 
            const dy = b.y - bodies[j].y; 
            const dz = b.z - bodies[j].z;   
            const sq = dx * dx + dy * dy + dz * dz;          
            e -= (b.mass * bodies[j].mass) / Math.sqrt(sq)        
        }
    }
    return e;
}

function advance(bodies, dt) {
    const numBodies = bodies.length;
    for (let i = 0; i < numBodies; i++) {
        for (let j = i + 1; j < numBodies; j++) {
            const dx = bodies[i].x - bodies[j].x; 
            const dy = bodies[i].y - bodies[j].y; 
            const dz = bodies[i].z - bodies[j].z;          
            const sq = dx * dx + dy * dy + dz * dz;
            const mag = dt / (sq * Math.sqrt(sq));            

            const mj = bodies[j].mass * mag;
            bodies[i].vx -= dx * mj;  
            bodies[i].vy -= dy * mj;   
            bodies[i].vz -= dz * mj;

            const mi = bodies[i].mass * mag;
            bodies[j].vx += dx * mi;  
            bodies[j].vy += dy * mi;   
            bodies[j].vz += dz * mi;
        }
    }
    for (const b of bodies) {    
        b.x += b.vx * dt;  
        b.y += b.vy * dt;   
        b.z += b.vz * dt;
    }
}

function main(n) { 
    const bodies = [
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
        
        // saturn            
        new Body(
            8.34336671824457987e+00,
            4.12479856412430479e+00,
            -4.03523417114321381e-01,      
            -2.76742510726862411e-03 * DAYS_PER_YEAR,
            4.99852801234917238e-03 * DAYS_PER_YEAR,
            2.30417297573763929e-05 * DAYS_PER_YEAR,
            2.85885980666130812e-04 * SOLAR_MASS        
        ),        
        
        // uranus            
        new Body(
            1.28943695621391310e+01,
            -1.51111514016986312e+01,
            -2.23307578892655734e-01,
            2.96460137564761618e-03 * DAYS_PER_YEAR,
            2.37847173959480950e-03 * DAYS_PER_YEAR,
            -2.96589568540237556e-05 * DAYS_PER_YEAR,
            4.36624404335156298e-05 * SOLAR_MASS                
        ),   
        
        // neptune            
        new Body(        
            1.53796971148509165e+01,
            -2.59193146099879641e+01,
            1.79258772950371181e-01,    
            2.68067772490389322e-03 * DAYS_PER_YEAR,
            1.62824170038242295e-03 * DAYS_PER_YEAR,
            -9.51592254519715870e-05 * DAYS_PER_YEAR,
            5.15138902046611451e-05 * SOLAR_MASS         
        ),          
    ];       
    
    offsetMomentum(bodies); 
    console.log(energy(bodies).toFixed(9));
    for (let i = 0; i < n; i++) {
        advance(bodies,0.01);
    }
    console.log(energy(bodies).toFixed(9));
}

main(+process.argv[2]);
