# Four separately defined functions, with coefficients evaluated in every multiply.
n ← •ParseFloat⊑•args
A ← {1÷1+𝕨+0.5×(𝕨+𝕩)×1+𝕨+𝕩}
Av ← {v←𝕩 ⋄ {+´v×𝕩 A ↕≠v}¨↕≠v}
Atv ← {v←𝕩 ⋄ {+´v×(↕≠v) A 𝕩}¨↕≠v}
AtAv ← Atv∘Av
v ← AtAv⍟19 n⥊1
u ← AtAv v
r ← √(+´u×v)÷+´v×v
•Out {¯9(↓∾'.'∾↑)•Repr⌊0.5+1e9×𝕩} r
