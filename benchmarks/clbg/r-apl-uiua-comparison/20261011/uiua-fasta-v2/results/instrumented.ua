# Experimental!
N     ← ⋕◇∘⊡1 &args
Alu   ← "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
Iub   ← \+[0.27 0.12 0.12 0.27 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02]
Human ← \+[0.3029549426680 0.1979883004921 0.1975473066391 0.3015094502008]
~Wrapped {Chars Indices}
Indexed   ← Wrapped ⊃(∘|⇡+⊃⧻(⌈÷60⧻))
CharIndex ← -⊃(⌊÷61Wrapped.Indices)Wrapped.Indices
LineEnd   ← ↥⊃(=0◿61+1Wrapped.Indices)(=⊃Wrapped.Indices(-1⧻Wrapped.Indices))
Packed    ← ⊏⊃(+⊃CharIndex(×⊃LineEnd(-⊃CharIndex(⧻Wrapped.Chars))))(⊂⊃Wrapped.Chars(⋅"\n")) Indexed
Wrap ← &p ⊂ "PACKED=" json Packed
~G {Seed Left Letters Cdf}
~Chunk {Draws State}
Size         ← ↧65520 G.Left
Compose      ← [⊃(◿139968×∩⊢)(◿139968+⊃⊣(×⊃⊢(⋅⊣)))]
Coefficients ← \Compose ↯65520_2 3877_29573
Slice        ← ↙⊃Size(⋅Coefficients)
Draw ← (
  ◿139968+⊃(×⊃G.Seed(≡⊢Slice))(≡⊣Slice)
  &p ⊂ "DRAW=" json .
)
Chars ← (
  &p ⊂ "CDF_BYTES=" json ♭ bytes⌞ "f64" G.Cdf Chunk.State .
  &p ⊂ "RANDOM_BYTES=" json ♭ bytes⌞ "f64" ÷139968 Chunk.Draws .
  ⊏⊃(/+⊞≥⊃(G.Cdf Chunk.State)(÷139968 Chunk.Draws))(G.Letters Chunk.State)
)
Advance ← (
  ⟜(Wrap Chars)
  G ⊃(⊣Chunk.Draws|-⊃(⧻Chunk.Draws)(G.Left Chunk.State)|G.Letters Chunk.State|G.Cdf Chunk.State)
  &p ⊂ "STATE=" json .
) Chunk ⊃Draw∘
Generate ← (
  &p ⊂ "START=" json .
  G.Seed ⍢(Advance|>0G.Left)
)
AluDraw ← ⊏⊃(◿(⧻Alu)+⊃G.Seed(⇡Size))(⋅Alu)
AluAdvance ← (
  &p ⊂ "ALU_START=" json .
  ⟜(Wrap AluDraw)
  G ⊃(+⊃G.Seed Size|-⊃Size G.Left|G.Letters|G.Cdf)
  &p ⊂ "ALU_END=" json .
)
&p ">ONE Homo sapiens alu"
◌ ⍢(AluAdvance|>0G.Left) G 0 (×2N) Alu []
&p ">TWO IUB ambiguity codes"
Seed ← Generate G 42 (×3N) "acgtBDHKMNRSVWY" Iub
&p ">THREE Homo sapiens frequency"
◌ Generate G Seed (×5N) "acgt" Human
