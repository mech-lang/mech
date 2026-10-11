def factorial (n:i64):i64 = if n == 0 then 1 else n * factorial (n-1)
entry main (n:i64) = factorial n
