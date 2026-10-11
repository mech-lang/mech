~Wrapped {Chars Indices}
Indexed   ← Wrapped ⊃(∘|⇡+⊃⧻(⌈÷60⧻))
CharIndex ← -⊃(⌊÷61Wrapped.Indices)Wrapped.Indices
LineEnd   ← ↥⊃(=0◿61+1Wrapped.Indices)(=⊃Wrapped.Indices(-1⧻Wrapped.Indices))
Packed    ← ⊏⊃(+⊃CharIndex(×⊃LineEnd(-⊃CharIndex(⧻Wrapped.Chars))))(⊂⊃Wrapped.Chars(⋅"\n")) Indexed
Wrap      ← &w⊃Packed(⋅1)
Wrap &fras "/dev/stdin"
