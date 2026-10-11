-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
-- contributed by Mike Pall (with ideas from Rici Lake)
-- modified for 5.3 by Robin
-- optimized using string.byte by farteryhr

local sub, byte, insert, format =
  string.sub, string.byte, table.insert, string.format
iubc = {
  A="T", C="G", B="V", D="H", K="M", R="Y",
  a="T", c="G", b="V", d="H", k="M", r="Y",
  T="A", G="C", V="B", H="D", M="K", Y="R", U="A",
  t="A", g="C", v="B", h="D", m="K", y="R", u="A",
  N="N", S="S", W="W", n="N", s="S", w="W",
}
niubc = {} -- numeric lookup table
for k, v in pairs(iubc) do
  niubc[byte(k,1)] = byte(v,1)
end

local sbcode = {} -- string builder of code
-- to append a piece of code
local function add(s) insert(sbcode, s) end
add(
-- generate code for the processing function
-- @param t: the string builder of dna sequence
-- @param n: the number of elements + 1
[=[
return function(t, n)
  if n == 1 then return end
  local iubc, niubc, sub, byte, char, gc =
    iubc, niubc, string.sub, string.byte, string.char, collectgarbage
  local s = table.concat(t, "", 1, n-1)
-- collect the input string builder
  for i = 1, n-1 do t[i]=nil end
  gc()
-- string builder of output lines
  local sb, sbn = {}, 1
  for i=#s-59,1,-60 do
    local ]=])
-- extract 60 bytes a time backward from the end
for i = 1, 60 do
  add(format("c%d%s", i, i == 60 and " = " or ", "))
end
add("byte(s, i, i + 59)\n    ")
-- lookup and reverse
for i = 1, 60 do
  add(format("c%d%s", i, i == 60 and " = " or ", "))
end
for i = 1, 60 do
  add(format("niubc[c%d]%s", 61 - i, i == 60 and "" or ", "))
end
add("\n    ")
-- assemble a 60-char string
add("sb[sbn] = char(")
for i = 1, 60 do
  add(format("c%d%s", i, i == 60 and "" or ", "))
end
-- line ending and the rest
add([=[)
    sb[sbn+1] = "\n"
    sbn = sbn + 2
  end
  local r = #s % 60
  if r ~= 0 then
    for i=r,1,-1 do sb[sbn] = iubc[sub(s, i, i)]; sbn = sbn + 1 end
    sb[sbn] = "\n"; sbn = sbn + 1
  end
  s = nil
  gc()
  io.write(table.concat(sb))
  sb = nil
  gc()
end
]=])

local wcode = table.concat(sbcode) -- the string
local writerev = assert(load(wcode)()) -- the function

collectgarbage("stop") -- we do it manually

-- main routine
local t, n = {}, 1
for line in io.lines() do
  local c = sub(line, 1, 1)
  if c == ">" then writerev(t, n); io.write(line, "\n"); n = 1
  elseif c ~= ";" then t[n] = line; n = n + 1 end
end
writerev(t, n)
