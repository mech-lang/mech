inp ← {⟨⟩:•FBytes "/dev/stdin"; •file.MapBytes ⋈⁼𝕩}•args
cn ← 60000 # chunk size

lf ← @+10

inp ↩ {a‿b: (-1+≠b)↓a} (1+⊐⟜'>')⊸↓⍟3‿4 inp
! "THREE" ≡ 5↑inp # just making sure the above approach is correct
inp↓˜↩ inp⊐lf
inp/˜↩ inp≠lf
inp 8‿8•bit._cast↩

z ← "GTAC" # A & T can't be 0 - sequence searching assumes last char isn't zero
zi ← z-@


vs ← 3‿4‿6‿12‿18 ↑¨ <"GGTATTTTAATTTATAGT"
vsi ← {p‿c: ⟨n←≠p, s←z⊐n↓c ⋄ ≠s⟩}¨ <˘ 2↕2↓vs
lut ← ↕∘≠⌾((8|zi)⊸⊏) 8⥊9

stat16g ← 16⥊0
rsg ← 5⥊0
cnExt ← 17+cn # allows for peeking into the next chunk for the remaining 17 chars of the 18-long match

(⥊⟜0¨ 1‿1‿3‿4) { 𝕨 𝕊 n1_it‿n1_ig_it1‿n3_ia‿n4_it_it1:
  tn←≠tail ← inp↓˜cn×𝕨
  chunk ← (8|(tn⌊cnExt) ↑ tail) ⊏ lut
  stat16g+↩ 16↑/⁼ (cn⌊tn-1) ↑ (1↓chunk) + ¯1↓4×chunk
  chunkx ← (tn⌊cn) ↑ chunk
  igt ← chunkx ≤ 1
  iga ← chunkx ⊏ 1‿0‿1‿0
  ig ← igt∧iga
  it ← igt>iga
  ia ← igt<iga
  
  it1 ← n1_it « it
  ig_it1 ← ig∧it1
  m0 ← ig ∧ n1_ig_it1 « ig_it1
  m1 ← m0 ∧ n3_ia « ia
  it_it1 ← it∧it1
  i2 ← /m1 ∧ n4_it_it1 « it_it1
  
  r0 ← ⟨+´m0, +´m1, ≠i2⟩
  rsg+↩ r0 ∾ { 𝕊 p‿c‿nn:
    inpp ← p↓chunk
    i2 /˜↩ {c≡nn↑𝕩↓inpp}¨ i2
    ≠i2
  }¨ vsi
  ⟨1↑it, 1↑ig_it1, 3↑ia, 4↑it_it1⟩
}´ ↕⌊(¯1+cn+≠inp) ÷ cn


J ← {•Out¨ ""∾⟜<˜ J0¨ ∨ 𝕩⋈¨𝕨}

J0 ← {a‿b: a↩⌊0.5+1e5×a ⋄ ∾⟨b," ",•Repr ⌊a÷1e3,".","000"«˜•Repr 1e3|a⟩}
n ← ≠inp
z J n÷˜ 1⊸+⌾((lut⊑˜8|¯1⊑inp)⊸⊑) +˝˘4‿4⥊stat16g
(⥊∾⌜˜z) J stat16g ÷ n-1

# precise results instead of rounded ones, for verification
# J0 ← {a‿b: b∾" "∾•Repr a}
# z J 1⊸+⌾((lut⊑˜8|¯1⊑inp)⊸⊑) +˝˘4‿4⥊stat16g
# (⥊∾⌜˜z) J stat16g

vs {•Out ∾⟨•Repr 𝕩, @+9, 𝕨⟩}¨ rsg

