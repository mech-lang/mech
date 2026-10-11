# CLBG: full double-precision escape iteration and PBM byte output.
n ← •ParseFloat ⊑•args
cr ← ¯1.5 + 2×(↕n)÷n
Row ← {
  zr ← n⥊0
  zi ← n⥊0
  inside ← n⥊1
  Step ← {
    rr ← (zr×zr)-(zi×zi)
    zi ↩ 𝕩 + 2×zr×zi
    zr ↩ cr+rr
    inside ∧↩ 4≥(zr×zr)+(zi×zi)
    𝕩
  }
  Step⍟50 𝕩
  padded ← (8×⌈n÷8)↑inside
  bytes ← {+´𝕩×2⋆⌽↕8}˘ ((⌈n÷8)‿8)⥊padded
  •term.OutRaw bytes
}
•Out "P4"
•Out (•Repr n)∾" "∾•Repr n
Row¨ ¯1+2×(↕n)÷n
