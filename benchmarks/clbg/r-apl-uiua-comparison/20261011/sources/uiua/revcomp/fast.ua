Alphabet   ← "ACBDGHKMNSRUTWVYacbdghkmnsrutwvy"
Complement ← "TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"
Wrap       ← ⨬(◌|&pf /◇⊂ ⊕(□⊂⊙"\n")⊸(⌊÷60⇡⧻)) ⊸(>0⧻)
Clean      ← ⍜regex≡(⋅"") "(?m)^;[^\n]*\n|\r"
Records    ← regex "(?m)(>[^\n]*\n)([^>]*)" Clean &fras "/dev/stdin"
Sequence   ← ▽⊸(¬∊"\r\n") ◇∘⊡2
Record ← (
  ⟜(&pf◇∘⊡1)
  Wrap ⊏⊙Complement ⊗Alphabet ⇌Sequence
)
≡Record Records
