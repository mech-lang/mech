-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
-- contributed by Mike Pall
-- requires LGMP which requires lua >= 5.1, < 5.4

local g, aux = {}, {}
require"c-gmp"(g, aux)
local add, mul, div = g.mpz_add, g.mpz_mul_si, g.mpz_tdiv_q
local init, get = g.mpz_init_set_d, g.mpz_get_d

local q, r, s, t, u, v, w

-- Compose matrix with numbers on the right.
local function compose_r(bq, br, bs, bt)
  mul(r, bs, u)
  mul(r, bq, r)
  mul(t, br, v)
  add(r, v, r)
  mul(t, bt, t)
  add(t, u, t)
  mul(s, bt, s)
  mul(q, bs, u)
  add(s, u, s)
  mul(q, bq, q)
end

-- Compose matrix with numbers on the left.
local function compose_l(bq, br, bs, bt)
  mul(r, bt, r)
  mul(q, br, u)
  add(r, u, r)
  mul(t, bs, u)
  mul(t, bt, t)
  mul(s, br, v)
  add(t, v, t)
  mul(s, bq, s)
  add(s, u, s)
  mul(q, bq, q)
end

-- Extract one digit.
local function extract(j)
  mul(q, j, u)
  add(u, r, u)
  mul(s, j, v)
  add(v, t, v)
  return get(div(u, v, w))
end

-- Generate successive digits of PI.
local function pidigits(N)
  local write = io.write
  local k = 1
  q, r, s, t = init(1), init(0), init(0), init(1)
  u, v, w = init(0), init(0), init(0)
  local i = 0
  while i < N do
    local y = extract(3)
    if y == extract(4) then
      write(y)
      i = i + 1; if i % 10 == 0 then write("\t:", i, "\n") end
      compose_r(10, -10*y, 0, 1)
    else
      compose_l(k, 4*k+2, 0, 2*k+1)
      k = k + 1
    end
  end
  if i % 10 ~= 0 then write(string.rep(" ", 10 - N % 10), "\t:", N, "\n") end
end

local N = tonumber(arg and arg[1]) or 27
pidigits(N)
