-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

-- Translated from Mr Ledrug's C program by Jeremy Zerfas.
-- Transliterated from GMP to bn by Isaac Gouy

local Lbn=require"bn"
local add, sub, mul, div = Lbn.add, Lbn.sub, Lbn.mul, Lbn.div
local set, get = Lbn.number, Lbn.tonumber

local tmp1, tmp2, acc, den, num

local function extractDigit(nth)
    tmp1 = mul(num, nth)
    tmp2 = add(tmp1, acc)
    tmp1 = div(tmp2, den)
    return tmp1
end    

local function eliminateDigit(d)
    acc = sub(acc, mul(den, d))
    acc = mul(acc, 10)
    num = mul(num, 10)
end       

local function nextTerm(k)
    k2 = k * 2 + 1
    acc = add(acc, mul(num, 2))
    acc = mul(acc, k2)
    den = mul(den, k2)
    num = mul(num, k)
end    

local function main(n)
    local write = io.write
    tmp1 = set(0)
    tmp2 = set(0)
    acc = set(0)
    den = set(1)
    num = set(1)
    i = 0
    k = 0
    while i < n do
        k = k + 1
        nextTerm(k)
        if num > acc then goto continue end
        
        d = extractDigit(3)
        if d ~= extractDigit(4) then goto continue end
        
        write(get(d))
        i = i + 1; if i % 10 == 0 then write("\t:", i, "\n") end
        eliminateDigit(d)              
        ::continue::
    end         
    if i % 10 ~= 0 then write(string.rep(" ", 10 - n % 10), "\t:", n, "\n") end    
end 

main(tonumber(arg and arg[1]) or 27)
