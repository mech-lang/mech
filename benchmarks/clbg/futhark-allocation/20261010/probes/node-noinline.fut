#[noinline]
def allocate (a:i64) (b:i64):[2]i64 = #[opaque] [a,b]
entry main (n:i64):[][2]i64 =
  loop nodes = replicate n [0i64,0i64] for i < n do
    let node = allocate (2*i+1) (2*i+2)
    in nodes with [i] = node
