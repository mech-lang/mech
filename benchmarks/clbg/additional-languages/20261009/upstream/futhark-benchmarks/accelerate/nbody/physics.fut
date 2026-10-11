import "../../lib/github.com/athas/vector/vspace"

module vec3 = mk_vspace_3d f32

type mass = f32
type position = vec3.vector
type acceleration = vec3.vector
type velocity = vec3.vector

type pointmass =
  { position: position
  , mass: mass
  }

type body =
  { position: position
  , mass: mass
  , velocity: velocity
  }

def pointmass ({position, mass, velocity = _}: body) : pointmass =
  {position, mass}

def accel (epsilon: f32) (x: pointmass) (y: pointmass) : velocity =
  let r = vec3.(y.position - x.position)
  let rsqr = vec3.dot r r + epsilon * epsilon
  let invr = 1.0f32 / f32.sqrt rsqr
  let invr3 = invr * invr * invr
  let s = y.mass * invr3
  in vec3.scale s r

def advance_body (time_step: f32) (body: body) (acc: acceleration) : body =
  let position = vec3.(body.position + scale time_step body.velocity)
  let velocity = vec3.(body.velocity + scale time_step acc)
  in {position, mass = body.mass, velocity}
