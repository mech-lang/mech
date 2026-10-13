# vlang-big-integer
Big interger impelemented in pure vlang

[![CI](https://github.com/hanabi1224/vlang-big-integer/actions/workflows/main.yml/badge.svg)](https://github.com/hanabi1224/vlang-big-integer/actions/workflows/main.yml)
[![Build status](https://img.shields.io/appveyor/ci/hanabi1224/vlang-big-integer/main.svg)](https://ci.appveyor.com/project/hanabi1224/vlang-big-integer)
[![MIT License](https://img.shields.io/github/license/hanabi1224/vlang-big-integer.svg)](https://github.com/hanabi1224/vlang-big-integer/blob/master/LICENSE)

## Install
```bash
v install hanabi1224.biginteger
```

## Import
```v
import hanabi1224.biginteger
```

## Usage
```v
v1 := biginteger.from_int(-3)
// Note that '_' and ',' are allowed, only base-10 is currently supported.
v2 := biginteger.from_str("-23333333333333333333333333333333333333") 
v3 := biginteger.from_u64(9876554321)
// constants
zero := biginteger.zero
// + - * / %, TODO: pow etc.
println('$v2 + $ v3 = ${v2 + v3}')
println('$v2 - $ v3 = ${v2 - v3}')
println('$v2 * $ v3 = ${v2 * v3}')
println('$v2 / $ v3 = ${v2 / v3}')
println('$v2 % $ v3 = ${v2 % v3}')

// bit shifting TODO: bit and / or ops
println('v2 << 32 = ${v2.lshift(32)}')
println('v2 >> 2 = ${v2.rshift(2)}')
```

## Examples
### pidigits
```bash
v -prod -cc clang -gc boehm_full run ./examples/pidigits.v
```
