# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Naive transliteration from Michael Ferguson's Chapel program
# create a lot of temporary Struct
# contributed by Isaac Gouy

SOLAR_MASS = 4 * Math::PI * Math::PI
DAYS_PER_YEAR = 365.24

Triple = Struct.new :x, :y, :z do 
  def inc v
    self.x += v.x 
    self.y += v.y
    self.z += v.z  
    return self
  end    
  
  def dec v 
    self.x -= v.x 
    self.y -= v.y
    self.z -= v.z   
    return self    
  end   
  
  def s_mul s 
    # create a lot of temporary Struct  
    Triple[x * s, y * s, z * s]    
  end
  
  def s_div s   
    # used once
    Triple[x / s, y / s, z / s]        
  end       
  
  def - v 
    # create a lot of temporary Struct  
    Triple[x - v.x, y - v.y, z - v.z]    
  end 
  
  def sum_of_squares
    return x**2 + y**2 + z**2    
  end  
end

Body = Struct.new :pos, :vel, :mass

def init_sun bodies
 p = (bodies.map {|b| b.vel.s_mul b.mass }) \
     .inject(Triple[0.0, 0.0, 0.0], :inc)
     
  bodies[0].vel.inc ((p.s_div SOLAR_MASS).s_mul -1.0)
end

def advance(bodies, dt)
  num_bodies = bodies.size 
  for i in 0 ... num_bodies 
    for j in i+1 ... num_bodies    
      dpos = bodies[i].pos - bodies[j].pos
      dpos_norm_sq = dpos.sum_of_squares   
      mag = dt / (dpos_norm_sq * Math.sqrt(dpos_norm_sq)) 
      
      bodies[i].vel.dec dpos.s_mul bodies[j].mass * mag  
      bodies[j].vel.inc dpos.s_mul bodies[i].mass * mag       
    end 
  end
  for i in 0 ... num_bodies 
    bodies[i].pos.inc bodies[i].vel.s_mul dt 
  end
end

def energy bodies
  e = 0.0  
  num_bodies = bodies.size  
  for i in 0 ... num_bodies   
    e += 0.5 * bodies[i].mass * bodies[i].vel.sum_of_squares
    for j in i+1 ... num_bodies    
      e -= (bodies[i].mass * bodies[j].mass) \
          / Math.sqrt((bodies[i].pos - bodies[j].pos).sum_of_squares)     
    end           
  end
  return e
end

def nbody n
  bodies = [
    # sun
    Body[
      pos: Triple[0.0, 0.0, 0.0],
      vel: Triple[0.0, 0.0, 0.0],
      mass: SOLAR_MASS
      ],
    
    # jupiter
    Body[
      pos: Triple[ 
        4.84143144246472090e+00,
        -1.16032004402742839e+00,
        -1.03622044471123109e-01
        ],
      vel: Triple[       
        1.66007664274403694e-03 * DAYS_PER_YEAR,
        7.69901118419740425e-03 * DAYS_PER_YEAR,
        -6.90460016972063023e-05 * DAYS_PER_YEAR
        ],
      mass: 9.54791938424326609e-04 * SOLAR_MASS
      ],
    
    # saturn
    Body[
      pos: Triple[ 
        8.34336671824457987e+00,
        4.12479856412430479e+00,
        -4.03523417114321381e-01
        ],
      vel: Triple[       
        -2.76742510726862411e-03 * DAYS_PER_YEAR,
        4.99852801234917238e-03 * DAYS_PER_YEAR,
        2.30417297573763929e-05 * DAYS_PER_YEAR
        ],
      mass: 2.85885980666130812e-04 * SOLAR_MASS 
      ],    
    
    # uranus
    Body[
      pos: Triple[ 
        1.28943695621391310e+01,
        -1.51111514016986312e+01,
        -2.23307578892655734e-01
        ],
      vel: Triple[       
        2.96460137564761618e-03 * DAYS_PER_YEAR,
        2.37847173959480950e-03 * DAYS_PER_YEAR,
        -2.96589568540237556e-05 * DAYS_PER_YEAR
        ],
        mass: 4.36624404335156298e-05 * SOLAR_MASS 
      ],    
    
    # neptune
    Body[
      pos: Triple[ 
        1.53796971148509165e+01,
        -2.59193146099879641e+01,
        1.79258772950371181e-01
        ],
      vel: Triple[       
        2.68067772490389322e-03 * DAYS_PER_YEAR,
        1.62824170038242295e-03 * DAYS_PER_YEAR,
        -9.51592254519715870e-05 * DAYS_PER_YEAR,
        ],
      mass: 5.15138902046611451e-05 * SOLAR_MASS 
    ]   
  ]    

  init_sun bodies
  puts "%.9f" % (energy bodies)  
  n.times do
    advance(bodies, 0.01)
  end  
  puts "%.9f" % (energy bodies)  
end 

n = Integer(ARGV[0])  
nbody n
