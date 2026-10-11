-- N-body simulation based on the one from Accelerate:
-- https://github.com/AccelerateHS/accelerate-examples/tree/master/examples/n-body
--
-- ==
-- tags { futhark-opencl futhark-c }
--
-- nobench input @ data/nbody-acc-t0.in
-- output @ data/nbody-acc-t0.out
--
-- nobench input @ data/nbody-acc-t10.in
-- output @ data/nbody-acc-t10.out
--
-- nobench input @ data/100-bodies.in
-- input @ data/1000-bodies.in
-- input @ data/10000-bodies.in
-- input @ data/100000-bodies.in

-- "data/N-bodies.in" all have the other attributes n_steps=1, timestep=1.0, and
-- epsilon=50.0.

import "physics"

def calc_accels [n] (epsilon: f32) (bodies: [n]pointmass) : [n]acceleration =
  let move (body: pointmass) =
    let accels = map (accel epsilon body) bodies
    in reduce_comm (vec3.+) {x = 0f32, y = 0f32, z = 0f32} accels
  in map move bodies

def advance_bodies [n] (epsilon: f32) (time_step: f32) (bodies: [n]body) : [n]body =
  let accels = calc_accels epsilon (map pointmass bodies)
  in map2 (advance_body time_step) bodies accels

def advance_bodies_steps [n]
                         (n_steps: i32)
                         (epsilon: f32)
                         (time_step: f32)
                         (bodies: [n]body) : [n]body =
  loop bodies for _i < n_steps do
    advance_bodies epsilon time_step bodies

def wrap_body (posx: f32, posy: f32, posz: f32)
              (mass: f32)
              (velx: f32, vely: f32, velz: f32) : body =
  { position = {x = posx, y = posy, z = posz}
  , mass
  , velocity = {x = velx, y = vely, z = velz}
  }

def unwrap_body (b: body) : ((f32, f32, f32), f32, (f32, f32, f32)) =
  ( (b.position.x, b.position.y, b.position.z)
  , b.mass
  , (b.velocity.x, b.velocity.y, b.velocity.z)
  )

entry main [n]
           (n_steps: i32)
           (epsilon: f32)
           (time_step: f32)
           (xps: [n]f32)
           (yps: [n]f32)
           (zps: [n]f32)
           (ms: [n]f32)
           (xvs: [n]f32)
           (yvs: [n]f32)
           (zvs: [n]f32) : ([n]f32, [n]f32, [n]f32, [n]f32, [n]f32, [n]f32, [n]f32) =
  let bodies = map3 wrap_body (zip3 xps yps zps) ms (zip3 xvs yvs zvs)
  let bodies' = advance_bodies_steps n_steps epsilon time_step bodies
  let (final_pos, ms', final_vel) = map unwrap_body (bodies') |> unzip3
  let (xps', yps', zps') = unzip3 final_pos
  let (xvs', yvs', zvs') = unzip3 final_vel
  in (xps', yps', zps', ms', xvs', yvs', zvs')
