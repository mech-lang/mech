# Generic, thin bindings. Algorithms remain in the importing BQN source.
library ← "libarray-standard.dylib"
Bind ← {library •FFI 𝕩}
znew ⇐ Bind "*"‿"z_new"‿">u64"
zadd ⇐ Bind ""‿"z_add"‿"*"‿"*"‿"*"
zsub ⇐ Bind ""‿"z_sub"‿"*"‿"*"‿"*"
zmul ⇐ Bind ""‿"z_mul_ui"‿"*"‿"*"‿"u64"
zdiv ⇐ Bind ""‿"z_div"‿"*"‿"*"‿"*"
zui ⇐ Bind "u64"‿"z_ui"‿">*"
zcmp ⇐ Bind "i32"‿"z_cmp"‿"*"‿"*"
newraw ← Bind "*"‿"regex_new"‿"*u8:c8"‿"usize"
RxNew ⇐ {NewRaw ⟨𝕩,≠𝕩⟩}
countraw ← Bind "u64"‿"regex_count"‿"*"‿"*u8:c8"‿"usize"
RxCount ⇐ {CountRaw ⟨𝕨,𝕩,≠𝕩⟩}
replaceraw ← Bind "*"‿"regex_replace"‿"*"‿"*u8:c8"‿"usize"‿"*u8:c8"‿"usize"
bufferdata ← Bind "*u8"‿"buffer_data"‿">*"
buffersize ← Bind "usize"‿"buffer_size"‿">*"
bufferclose ← Bind ""‿"buffer_close"‿">*"
RxReplace ⇐ {𝕊 h‿s‿r:
 b ← ReplaceRaw ⟨h,s,≠s,r,≠r⟩
 bytes ← (BufferData b).ReadList BufferSize b
 BufferClose b
 @+bytes
}

# Parallel map for serializable immutable values, using at most eight processes.
forkRaw ← Bind "*"‿"process_fork"
pidRaw ← Bind "i32"‿"process_pid"‿">*"
fdRaw ← Bind "i32"‿"process_fd"‿">*"
waitRaw ← Bind ""‿"process_wait"‿">*"
_Parallel ⇐ {
 0=≠𝕩? ⟨⟩;
 F ← 𝔽
 values ← 𝕩
 workers ← 8⌊≠values
 cuts ← ⌊(↕workers+1)×(≠values)÷workers
 ForkOne ← {𝕊 a‿b:
  part ← (b-a)↑a↓values
  child ← ForkRaw ⟨⟩
  {𝕊 unused: 0=PidRaw child?
   •Out •Repr F¨part
   •Exit 0;
   child
  } @
 }
 children ← ForkOne˘ 2↕cuts
 ReadOne ← {
  value ← •BQN ⊑•FLines "/dev/fd/"∾•Repr FdRaw 𝕩
  WaitRaw 𝕩
  value
 }
 ∾ ReadOne¨children
}
