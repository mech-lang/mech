# Sequential unbounded LFT spigot: extract both 3 and 4 before emitting.
std ← •Import "../stdlib/standard.bqn"
n ← •ParseFloat⊑•args
q ← std.ZNew 1
r ← std.ZNew 0
t ← std.ZNew 1
a ← std.ZNew 0
b ← std.ZNew 0
k ← 0
digits ← ""
Extract ← {std.ZMul ⟨a,q,𝕩⟩ ⋄ std.ZAdd ⟨a,a,r⟩ ⋄ std.ZDiv ⟨b,a,t⟩ ⋄ std.ZUI b}
Step ← {
 k +↩ 1
 std.ZMul ⟨a,q,2⟩
 std.ZAdd ⟨r,r,a⟩
 std.ZMul ⟨r,r,1+2×k⟩
 std.ZMul ⟨t,t,1+2×k⟩
 std.ZMul ⟨q,q,k⟩
 {𝕊 unused: 0<(std.ZCmp ⟨q,r⟩)? @;
  d ← Extract 3
  {𝕊 unused: d≠Extract 4? @;
   digits ∾↩ '0'+d
   std.ZMul ⟨q,q,10⟩
   std.ZMul ⟨a,t,d⟩
   std.ZSub ⟨r,r,a⟩
   std.ZMul ⟨r,r,10⟩
   @
  } @
 } @
 𝕩
}
Step •_while_ {𝕊 unused: n>≠digits} @
Report ← {𝕊 i:
 length ← n⌊10+i
 line ← (length-i)↑i↓digits
 •Out line∾((10-≠line)⥊' ')∾(@+9)∾":"∾•Repr length
}
Report¨ 10×↕⌈n÷10
