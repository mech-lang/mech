<img width="40%" height="40%" src="https://mechlang.net/img/logo.png">

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your problem. 

Read about progress on our [blog](https://mechlang.net/blog/), follow us on Twitter [@MechLang](https://twitter.com/MechLang), or join the mailing list: [talk@mechlang.net](https://mechlang.net/page/community/).

# Core

There are three components to Mech:

1. Core (this repository) - A small dataflow engine that accepts transactions of changes, and applies them to a compute network.  
2. [Server](https://gitlab.com/mech-lang/server) - Hosts Mech cores for connected clients. 
3. [Notebook](https://gitlab.com/mech-lang/notebook) - A graphical interface that connects to a Mech server.
4. [Syntax](https://gitlab.com/mech-lang/syntax) - A compiler for a textual Mech syntax.

Mech core does not rely on the Rust standard library, so it can be compiled and used on bare-bones operating systems (check out [HiveMind OS](https://gitlab.com/cmontella/hivemind) for an example of this).

## Contents

- table - defines a `Table`, the core data structure of Mech. Also defines a `Value`, which unifies the various data types (Number, String, Bool, Table).
- database - defines a `Database` of tables. Databases accept `Transactions`, which is are sets of `Changes` to the database.
- indexes - defines the various indexes used to quickly look up information in the database
- runtime - defines a `Runtime`, which orchestrates the compute graph; and `Blocks`, which comprise the compute graph.
- operations - defines the primitive operations that can be performed by nodes in the compute network.

## Example Mech Code

```mech
# Bouncing Balls

Define the environment
  #html/event/click = [x: 0 y: 0]
  #ball = [x: 15 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15]
  #gravity = 2
  #boundary = 5000

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity

## Boundary Condition

Keep the balls within the y boundary
  ~ #ball.x
  iy = #ball.y > #boundary
  #ball.y[iy] := #boundary
  #ball.vy[iy] := 0 - 1 * #ball.vy * 80 / 100

Keep the balls within the x boundary
  ~ #ball.y
  ix = #ball.x > #boundary
  ixx = #ball.x < 0
  #ball.x[ix] := #boundary
  #ball.x[ixx] := 0
  #ball.vx[ix] := 0 - 1 * #ball.vx * 80 / 100
  #ball.vx[ixx] := 0 - 1 * #ball.vx * 80 / 100

## Create More Balls

Create ball on click
  ~ #html/event/click.x
  #ball += [x: 2 y: 3 vx: 40 vy: 0]
```

## License

Apache 2.0