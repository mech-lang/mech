SOLAR_MASS = 4.0 * Math::PI * Math::PI
DAYS_PER_YEAR = 365.24
def initial_bodies
  [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, SOLAR_MASS],
    [4.84143144246472090e+00, -1.16032004402742839e+00, -1.03622044471123109e-01, 1.66007664274403694e-03 * DAYS_PER_YEAR, 7.69901118419740425e-03 * DAYS_PER_YEAR, -6.90460016972063023e-05 * DAYS_PER_YEAR, 9.54791938424326609e-04 * SOLAR_MASS],
    [8.34336671824457987e+00, 4.12479856412430479e+00, -4.03523417114321381e-01, -2.76742510726862411e-03 * DAYS_PER_YEAR, 4.99852801234917238e-03 * DAYS_PER_YEAR, 2.30417297573763929e-05 * DAYS_PER_YEAR, 2.85885980666130812e-04 * SOLAR_MASS],
    [1.28943695621391310e+01, -1.51111514016986312e+01, -2.23307578892655734e-01, 2.96460137564761618e-03 * DAYS_PER_YEAR, 2.37847173959480950e-03 * DAYS_PER_YEAR, -2.96589568540237556e-05 * DAYS_PER_YEAR, 4.36624404335156298e-05 * SOLAR_MASS],
    [1.53796971148509165e+01, -2.59193146099879641e+01, 1.79258772950371181e-01, 2.68067772490389322e-03 * DAYS_PER_YEAR, 1.62824170038242295e-03 * DAYS_PER_YEAR, -9.51592254519715870e-05 * DAYS_PER_YEAR, 5.15138902046611451e-05 * SOLAR_MASS],
  ]
end

def energy(b)
  total = 0.0
  b.size.times do |i|
    total += 0.5 * b[i][6] * (b[i][3] ** 2 + b[i][4] ** 2 + b[i][5] ** 2)
    ((i + 1)...b.size).each do |j|
      dx = b[i][0] - b[j][0]
      dy = b[i][1] - b[j][1]
      dz = b[i][2] - b[j][2]
      total -= b[i][6] * b[j][6] / Math.sqrt(dx * dx + dy * dy + dz * dz)
    end
  end
  total
end
n = (ARGV[0]? || "1000").to_i
b = initial_bodies
(3...6).each do |coordinate|
  momentum = 0.0
  b.each { |body| momentum += body[coordinate] * body[6] }
  b[0][coordinate] = -momentum / SOLAR_MASS
end
puts "%.9f" % energy(b)
n.times do
  b.size.times do |i|
    ((i + 1)...b.size).each do |j|
      dx = b[i][0] - b[j][0]
      dy = b[i][1] - b[j][1]
      dz = b[i][2] - b[j][2]
      squared = dx * dx + dy * dy + dz * dz
      magnitude = 0.01 / (squared * Math.sqrt(squared))
      displacement = StaticArray[dx, dy, dz]
      3.times do |coordinate|
        b[i][coordinate + 3] -= displacement[coordinate] * b[j][6] * magnitude
        b[j][coordinate + 3] += displacement[coordinate] * b[i][6] * magnitude
      end
    end
  end
  b.each do |body|
    3.times { |coordinate| body[coordinate] += 0.01 * body[coordinate + 3] }
  end
end
puts "%.9f" % energy(b)
