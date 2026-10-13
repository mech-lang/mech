import clbg_io as io
import gleam/list

type Body {
  Body(x: Float, y: Float, z: Float, vx: Float, vy: Float, vz: Float, mass: Float)
}

fn initial() -> List(Body) {
  let pi = 3.141592653589793
  let solar = 4.0 *. pi *. pi
  let days = 365.24
  [
    Body(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, solar),
    Body(4.84143144246472090, -1.16032004402742839, -1.03622044471123109e-1, 1.66007664274403694e-3 *. days, 7.69901118419740425e-3 *. days, -6.90460016972063023e-5 *. days, 9.54791938424326609e-4 *. solar),
    Body(8.34336671824457987, 4.12479856412430479, -4.03523417114321381e-1, -2.76742510726862411e-3 *. days, 4.99852801234917238e-3 *. days, 2.30417297573763929e-5 *. days, 2.85885980666130812e-4 *. solar),
    Body(1.28943695621391310e1, -1.51111514016986312e1, -2.23307578892655734e-1, 2.96460137564761618e-3 *. days, 2.37847173959480950e-3 *. days, -2.96589568540237556e-5 *. days, 4.36624404335156298e-5 *. solar),
    Body(1.53796971148509165e1, -2.59193146099879641e1, 1.79258772950371181e-1, 2.68067772490389322e-3 *. days, 1.62824170038242295e-3 *. days, -9.51592254519715870e-5 *. days, 5.15138902046611451e-5 *. solar),
  ]
}

fn offset(bodies: List(Body)) -> List(Body) {
  let #(px, py, pz) = list.fold(bodies, #(0.0, 0.0, 0.0), fn(total, body) {
    #(total.0 +. body.vx *. body.mass, total.1 +. body.vy *. body.mass, total.2 +. body.vz *. body.mass)
  })
  let assert [sun, ..rest] = bodies
  [Body(..sun, vx: { 0.0 -. px } /. sun.mass, vy: { 0.0 -. py } /. sun.mass, vz: { 0.0 -. pz } /. sun.mass), ..rest]
}

fn impulse(a: Body, b: Body) -> #(Body, Body) {
  let dx = a.x -. b.x
  let dy = a.y -. b.y
  let dz = a.z -. b.z
  let squared = dx *. dx +. dy *. dy +. dz *. dz
  let magnitude = 0.01 /. { squared *. io.sqrt(squared) }
  #(
    Body(..a, vx: a.vx -. dx *. b.mass *. magnitude, vy: a.vy -. dy *. b.mass *. magnitude, vz: a.vz -. dz *. b.mass *. magnitude),
    Body(..b, vx: b.vx +. dx *. a.mass *. magnitude, vy: b.vy +. dy *. a.mass *. magnitude, vz: b.vz +. dz *. a.mass *. magnitude),
  )
}

fn partners(body: Body, rest: List(Body)) -> #(Body, List(Body)) {
  case rest {
    [] -> #(body, [])
    [other, ..remaining] -> {
      let #(body, other) = impulse(body, other)
      let #(body, remaining) = partners(body, remaining)
      #(body, [other, ..remaining])
    }
  }
}

fn velocities(bodies: List(Body)) -> List(Body) {
  case bodies {
    [] -> []
    [body, ..rest] -> {
      let #(body, rest) = partners(body, rest)
      [body, ..velocities(rest)]
    }
  }
}

fn move(body: Body) -> Body {
  Body(..body, x: body.x +. 0.01 *. body.vx, y: body.y +. 0.01 *. body.vy, z: body.z +. 0.01 *. body.vz)
}

fn advance(n: Int, bodies: List(Body)) -> List(Body) {
  case n == 0 {
    True -> bodies
    False -> advance(n - 1, list.map(velocities(bodies), move))
  }
}

fn potential(body: Body, rest: List(Body), total: Float) -> Float {
  case rest {
    [] -> total
    [other, ..remaining] -> {
      let dx = body.x -. other.x
      let dy = body.y -. other.y
      let dz = body.z -. other.z
      potential(body, remaining, total -. body.mass *. other.mass /. io.sqrt(dx *. dx +. dy *. dy +. dz *. dz))
    }
  }
}

fn energy(bodies: List(Body), total: Float) -> Float {
  case bodies {
    [] -> total
    [body, ..rest] -> {
      let total = total +. 0.5 *. body.mass *. { body.vx *. body.vx +. body.vy *. body.vy +. body.vz *. body.vz }
      energy(rest, potential(body, rest, total))
    }
  }
}

pub fn main() -> Nil {
  let bodies = offset(initial())
  io.text(io.decimal(energy(bodies, 0.0), 9) <> "\n")
  io.text(io.decimal(energy(advance(io.argument(), bodies), 0.0), 9) <> "\n")
}
