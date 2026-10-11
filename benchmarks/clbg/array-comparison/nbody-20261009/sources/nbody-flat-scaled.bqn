# contributed by dzaima Sep 2026
pi ← 3.14159265358979323
solar_mass ← 4 × pi × pi
days_per_year ← 365.24

bodies ← ⟨
  "sun" ⋈ ⟨⟨0.0, 0.0, 0.0⟩, ⟨0.0, 0.0, 0.0⟩, solar_mass⟩,

  "jupiter" ⋈ ⟨⟨4.84143144246472090e00,
               -1.16032004402742839e00,
               -1.03622044471123109e¯01⟩,
              ⟨1.66007664274403694e¯03 × days_per_year,
               7.69901118419740425e¯03 × days_per_year,
               -6.90460016972063023e¯05 × days_per_year⟩,
               9.54791938424326609e¯04 × solar_mass⟩,

  "saturn" ⋈ ⟨⟨8.34336671824457987e00,
              4.12479856412430479e00,
              -4.03523417114321381e¯01⟩,
             ⟨-2.76742510726862411e¯03 × days_per_year,
              4.99852801234917238e¯03 × days_per_year,
              2.30417297573763929e¯05 × days_per_year⟩,
             2.85885980666130812e¯04 × solar_mass⟩,

  "uranus" ⋈ ⟨⟨1.28943695621391310e01,
              -1.51111514016986312e01,
              -2.23307578892655734e¯01⟩,
             ⟨2.96460137564761618e¯03 × days_per_year,
              2.37847173959480950e¯03 × days_per_year,
              -2.96589568540237556e¯05 × days_per_year⟩,
             4.36624404335156298e¯05 × solar_mass⟩,

  "neptune" ⋈ ⟨⟨1.53796971148509165e01,
               -2.59193146099879641e01,
               1.79258772950371181e¯01⟩,
              ⟨2.68067772490389322e¯03 × days_per_year,
               1.62824170038242295e¯03 × days_per_year,
               -9.51592254519715870e¯05 × days_per_year⟩,
              5.15138902046611451e¯05 × solar_mass⟩
⟩

system ← 1 ⊑¨ bodies
pxyz ← ⍉ > ⊑¨ system
vxyz ← ⍉ > 1 ⊑¨ system
masses ← 2 ⊑¨ system

OffsetMomentum ← {𝕊:
  p2 ← - (+˝˘ vxyz ×⎉1 masses) ÷ solar_mass
  vxyz p2⌾(⊏˘)↩
}

Energy ← {𝕊:
  ep ← 0.5 × +´masses × +˝×˜vxyz
  en ← +´ +´¨ (↕∘≠ ↑¨ <˘) (×⌜˜ masses) ÷ √ +˝⎉1 ×˜ > -⌜˜ <˘ ⍉pxyz
  ep - en
}

pair_count ← ≠masses
pair_flat ← /⥊<⌜˜↕pair_count
pair_i ← ⌊pair_flat ÷ pair_count
pair_j ← pair_count | pair_flat
pair_size ← ≠pair_i
position_i ← ⥊(pair_count×↕3) +⌜ pair_i
position_j ← ⥊(pair_count×↕3) +⌜ pair_j
d0 ← ↕pair_size
d1 ← pair_size + d0
d2 ← pair_size + d1
coord_pair ← pair_size | ↕3×pair_size
pair_mass3 ← ((∾3⥊<(-pair_j⊏masses)) ∾ ∾3⥊<pair_i⊏masses) × 0.01×0.01
force_groups ← >{ /𝕩 = pair_i ∾ pair_j }¨ ↕pair_count
force_groups +↩ (2×pair_size) × force_groups≥pair_size
force_by_axis ← (pair_size×↕3) +⌜ force_groups
f0 ← ⥊ 0 ⊏⎉1 force_by_axis
f1 ← ⥊ 1 ⊏⎉1 force_by_axis
f2 ← ⥊ 2 ⊏⎉1 force_by_axis
f3 ← ⥊ 3 ⊏⎉1 force_by_axis
p ← ⥊pxyz
v ← 0.01×⥊vxyz
Advance ← {𝕊:
  d ← (position_i ⊏ p) - position_j ⊏ p
  dd ← d×d
  distance2 ← (d0 ⊏ dd) + (d1 ⊏ dd) + d2 ⊏ dd
  mag ← distance2 × √distance2
  a ← d ÷ coord_pair ⊏ mag
  force ← (a ∾ a) × pair_mass3
  v +↩ (f0 ⊏ force) + (f1 ⊏ force) + (f2 ⊏ force) + f3 ⊏ force
  p +↩ v
}

· ← {
  n ← •ParseFloat ⋈⁼•args
  OffsetMomentum@
  Fmt9 ← {•Out'-'⌾⊑⍟('¯'=⊑) ∾⟜('0'⥊˜0⌈≠-˜10+⊐⟜'.') ∾⟜'.'⍟(⊑'.'¬∘∊⊢) •Repr (××(⌊0.5⊸+)⌾(1e9⊸×)∘|) 𝕩}
  Fmt9 Energy@
  p ↩ ⥊pxyz
  v ↩ 0.01×⥊vxyz
  Advance⍟n @
  pxyz ↩ 3‿pair_count⥊p
  vxyz ↩ (3‿pair_count⥊v)÷0.01
  Fmt9 Energy@
}
