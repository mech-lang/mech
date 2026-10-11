Base ← 10
New  ← ⍥(⊥Base)⊸(=₀⧻△)
Call ← New
Str  ← +@0⇌

Fix ↚ (
  Overflow ← ⍢(
    ⨬(+°↻₁|+⊓⌞⊂˜⊂0)⊸(±⊣) ⊃(⌊÷|◿)Base
  | ¬/××⊃(≥₀|<Base))
  ▽¬⍜⇌\×⊸=₀ Overflow
)

Eq ← ≍ ∩New
Ne ← ¬Eq
Le ← ⨬(⍥⋅(<∩⌞⊏⊣⊚◡≠)⊃=<◡∩⧻|1)◡Eq ∩New
Ge ← ˜Le
Gt ← ¬Le
Lt ← ˜Gt

Add ← ⌅(Fix ⬚0+ ∩New|Fix ⬚0- ◡(˙⍤Ge) ∩New)
Sub ← ⌝Add

ModDiv ← (
  ∩New
  ⊸Eq0
  ⨬(◡Lt
    ⨬(⤚(⊃↘↙¯-₁⧻)
      ⊙Fix⊙◌∧(⊃⋅⊙∘(⊂⊙⋅⋅∘) °⍥°⍣⊸˜Sub∘ ⊂) ⇌ ⊙⊙⊙[]
    | ⋅⊙[])
  | ˙⍤0)
)

Mul ← ⌅(Fix /+⬚0≡⌝↘°⊏⊞× ∩New|⋅∘ModDiv)
Div ← ⌝Mul

Pow ← ⨬(New 1|⋅New|/Mul↯) ⊸⨂0_1 °[]⊸△

Mod ← ⊙◌ModDiv

Gcd ← ◌⍢⤚Mod(Ne0)

┌─╴test
  Factorial ← /Mul⇡₁

  ⍤⤙≍ "3628800" Str Factorial 10
  Str Factorial 52
  "80658175170943878571660636856403766975289505440883277824000000000000"
  ⍤⤙≍

  Fibonacci ← ◌⍥⤚Add⊙(1 0)

  Str Fibonacci 256
  "141693817714056513234709965875411919657707794958199867"
  ⍤⤙≍

  ⍤⤙≍"10517015330328389030" ⍜∩°Str Mod "14712229619566877972" "39941474569462144974"

  ⍤⤙≍ "11" ⍜∩°Str Gcd "120051078586869202932203" "108614120051078586869202932203"
└─╴
