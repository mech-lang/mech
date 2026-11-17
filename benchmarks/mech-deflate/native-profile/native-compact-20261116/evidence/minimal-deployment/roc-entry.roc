app [run] { pf: platform "platform/main.roc", deflate: "../roc-deflate/roc-deflate-671077457ff7ef3a3d44caf715a3fc595addf379/package/main.roc" }
import deflate.Deflate
run : List(U8), List(U8) -> List(U8)
run = |input, output| Deflate.compress_into(input, 12, output)
