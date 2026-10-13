import std/[strutils, re]
import wrappers/pcre as api
doAssert ($api.version()).startsWith("8.45 ")
doAssert sizeof(api.ExtraData) == 64
doAssert alignof(api.ExtraData) == 8
doAssert findAll("aaaaa", re("aa")).len == 2
let first = replace("taNDHaD", re("aND|Ha[DS]"), "<3>")
doAssert replace(first, re("<[^>]*>"), "|") == "t||"
doAssert findAll("AGGGTAAA", re("agggtaaa")).len == 0
echo "pcre-8.45-dynamic-layout-regex-contract-ok"
