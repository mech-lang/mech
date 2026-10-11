# Pure BQN n-body; standard five-body inputs, one matrix row per line.
solar_mass ← 4×3.14159265358979323×3.14159265358979323
p ← ⟨
  0, 4.84143144246472090e00, 8.34336671824457987e00, 1.28943695621391310e01, 1.53796971148509165e01,
  0, -1.16032004402742839e00, 4.12479856412430479e00, -1.51111514016986312e01, -2.59193146099879641e01,
  0, -1.03622044471123109e¯01, -4.03523417114321381e¯01, -2.23307578892655734e¯01, 1.79258772950371181e¯01⟩
v ← 0.01×365.24×⟨
  0, 1.66007664274403694e¯03, -2.76742510726862411e¯03, 2.96460137564761618e¯03, 2.68067772490389322e¯03,
  0, 7.69901118419740425e¯03, 4.99852801234917238e¯03, 2.37847173959480950e¯03, 1.62824170038242295e¯03,
  0, -6.90460016972063023e¯05, 2.30417297573763929e¯05, -2.96589568540237556e¯05, -9.51592254519715870e¯05⟩
masses ← solar_mass×⟨1, 9.54791938424326609e¯04, 2.85885980666130812e¯04, 4.36624404335156298e¯05, 5.15138902046611451e¯05⟩

pair_count ← ≠masses
pair_flat ← /⥊<⌜˜↕pair_count
pair_i ← ⌊pair_flat ÷ pair_count
pair_j ← pair_count | pair_flat
pair_size ← ≠pair_i
position_i ← ⥊(pair_count×↕3) +⌜ pair_i
position_j ← ⥊(pair_count×↕3) +⌜ pair_j
coord_pair ← pair_size | ↕3×pair_size
pair_mass3 ← ((∾3⥊<(-pair_j⊏masses)) ∾ ∾3⥊<pair_i⊏masses) × 0.01×0.01
force_groups ← >{ /𝕩 = pair_i ∾ pair_j }¨ ↕pair_count
force_groups +↩ (2×pair_size) × force_groups≥pair_size
force_slots ← ⥊1‿2‿0⍉(pair_size×↕3) +⌜ force_groups
momentum ← -(+˝˘3‿pair_count⥊v×∾3⥊<masses)÷solar_mass
v momentum⌾((pair_count×↕3)⊸⊏)↩

Energy ← {𝕊:
  (0.5×+´masses×(+˝3‿pair_count⥊v×v)÷0.01×0.01) - +´((pair_i⊏masses)×pair_j⊏masses)÷√+˝3‿pair_size⥊×˜(position_i⊏p)-position_j⊏p}

Advance ← {𝕊:
  d ← (position_i ⊏ p) - position_j ⊏ p
  distance2 ← +˝3‿pair_size⥊d×d
  mag ← distance2 × √distance2
  a ← d ÷ coord_pair ⊏ mag
  force ← (a ∾ a) × pair_mass3
  v +↩ +˝4‿(3×pair_count)⥊force_slots⊏force
  p +↩ v
}

Fmt9 ← {•Out'-'⌾⊑⍟('¯'=⊑) ∾⟜('0'⥊˜0⌈≠-˜10+⊐⟜'.') ∾⟜'.'⍟(⊑'.'¬∘∊⊢) •Repr (××(⌊0.5⊸+)⌾(1e9⊸×)∘|) 𝕩}
Fmt9 Energy@
Advance⍟(•ParseFloat ⋈⁼•args) @
Fmt9 Energy@
