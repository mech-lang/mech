defmodule Clbg.Main do
  alias Clbg.Support, as: S
  @solar 4.0 * 3.141592653589793 * 3.141592653589793
  @days 365.24
  def planet(x, y, z, vx, vy, vz, mass), do: {x, y, z, vx * @days, vy * @days, vz * @days, mass * @solar}
  def energy(bodies) do
    Enum.reduce(S.indices(5), 0.0, fn i, total ->
      a = elem(bodies, i)
      total = total + 0.5 * elem(a, 6) * (elem(a, 3) * elem(a, 3) + elem(a, 4) * elem(a, 4) + elem(a, 5) * elem(a, 5))
      Enum.reduce(S.indices(4 - i), total, fn offset, sum ->
        b = elem(bodies, i + offset + 1)
        dx = elem(a, 0) - elem(b, 0)
        dy = elem(a, 1) - elem(b, 1)
        dz = elem(a, 2) - elem(b, 2)
        sum - elem(a, 6) * elem(b, 6) / :math.sqrt(dx * dx + dy * dy + dz * dz)
      end)
    end)
  end
  def advance(bodies) do
    bodies = Enum.reduce(S.indices(4), bodies, fn i, bodies ->
      Enum.reduce(S.indices(4 - i), bodies, fn offset, bodies ->
        j = i + offset + 1
        a = elem(bodies, i)
        b = elem(bodies, j)
        displacement = {elem(a, 0) - elem(b, 0), elem(a, 1) - elem(b, 1), elem(a, 2) - elem(b, 2)}
        squared = elem(displacement, 0) * elem(displacement, 0) + elem(displacement, 1) * elem(displacement, 1) + elem(displacement, 2) * elem(displacement, 2)
        magnitude = 0.01 / (squared * :math.sqrt(squared))
        {a, b} = Enum.reduce(S.indices(3), {a, b}, fn axis, {a, b} ->
          d = elem(displacement, axis)
          {put_elem(a, axis + 3, elem(a, axis + 3) - d * elem(b, 6) * magnitude), put_elem(b, axis + 3, elem(b, axis + 3) + d * elem(a, 6) * magnitude)}
        end)
        bodies |> put_elem(i, a) |> put_elem(j, b)
      end)
    end)
    Enum.reduce(S.indices(5), bodies, fn i, bodies ->
      body = elem(bodies, i)
      body = Enum.reduce(S.indices(3), body, fn axis, body -> put_elem(body, axis, elem(body, axis) + 0.01 * elem(body, axis + 3)) end)
      put_elem(bodies, i, body)
    end)
  end
  defp steps(0, bodies), do: bodies
  defp steps(n, bodies), do: steps(n - 1, advance(bodies))
  def main(arguments) do
    bodies = {
      {0.0, 0.0, 0.0, 0.0, 0.0, 0.0, @solar},
      planet(4.84143144246472090, -1.16032004402742839, -1.03622044471123109e-1, 1.66007664274403694e-3, 7.69901118419740425e-3, -6.90460016972063023e-5, 9.54791938424326609e-4),
      planet(8.34336671824457987, 4.12479856412430479, -4.03523417114321381e-1, -2.76742510726862411e-3, 4.99852801234917238e-3, 2.30417297573763929e-5, 2.85885980666130812e-4),
      planet(1.28943695621391310e1, -1.51111514016986312e1, -2.23307578892655734e-1, 2.96460137564761618e-3, 2.37847173959480950e-3, -2.96589568540237556e-5, 4.36624404335156298e-5),
      planet(1.53796971148509165e1, -2.59193146099879641e1, 1.79258772950371181e-1, 2.68067772490389322e-3, 1.62824170038242295e-3, -9.51592254519715870e-5, 5.15138902046611451e-5)}
    momentum = Enum.reduce(S.indices(5), {0.0, 0.0, 0.0}, fn i, momentum ->
      body = elem(bodies, i)
      Enum.reduce(S.indices(3), momentum, fn axis, momentum -> put_elem(momentum, axis, elem(momentum, axis) + elem(body, axis + 3) * elem(body, 6)) end)
    end)
    sun = Enum.reduce(S.indices(3), elem(bodies, 0), fn axis, sun -> put_elem(sun, axis + 3, -elem(momentum, axis) / @solar) end)
    bodies = put_elem(bodies, 0, sun)
    before = S.fixed(energy(bodies), 9)
    after_energy = bodies |> then(fn bodies -> steps(S.size(arguments, 1000), bodies) end) |> energy() |> S.fixed(9)
    S.write([before, "\n", after_energy, "\n"])
  end
end
