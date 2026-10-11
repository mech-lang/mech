	.file	""
	.data
	.globl	_camlBinarytrees$$2docaml$$2d5$data_begin
_camlBinarytrees$$2docaml$$2d5$data_begin:
	.text
	.globl	_camlBinarytrees$$2docaml$$2d5$code_begin
_camlBinarytrees$$2docaml$$2d5$code_begin:
	nop
	.align	3
	.data
	.align	3
	.data
	.align	3
	.quad	3063
	.globl	_camlBinarytrees$$2docaml$$2d5__set_of_closures_379
_camlBinarytrees$$2docaml$$2d5__set_of_closures_379:
	.globl	_camlBinarytrees$$2docaml$$2d5$loop_depths_113_closure
_camlBinarytrees$$2docaml$$2d5$loop_depths_113_closure:
	.quad	_camlBinarytrees$$2docaml$$2d5$loop_depths_113
	.quad	72057594037927941
	.data
	.align	3
	.quad	3063
	.globl	_camlBinarytrees$$2docaml$$2d5__set_of_closures_218
_camlBinarytrees$$2docaml$$2d5__set_of_closures_218:
	.globl	_camlBinarytrees$$2docaml$$2d5$check_43_closure
_camlBinarytrees$$2docaml$$2d5$check_43_closure:
	.quad	_camlBinarytrees$$2docaml$$2d5$check_43
	.quad	72057594037927941
	.data
	.align	3
	.quad	3063
	.globl	_camlBinarytrees$$2docaml$$2d5__set_of_closures_217
_camlBinarytrees$$2docaml$$2d5__set_of_closures_217:
	.globl	_camlBinarytrees$$2docaml$$2d5$make_23_closure
_camlBinarytrees$$2docaml$$2d5$make_23_closure:
	.quad	_camlBinarytrees$$2docaml$$2d5$make_23
	.quad	72057594037927941
	.data
	.align	3
	.quad	7936
	.globl	_camlBinarytrees$$2docaml$$2d5
_camlBinarytrees$$2docaml$$2d5:
	.quad	_camlBinarytrees$$2docaml$$2d5$make_23_closure
	.quad	_camlBinarytrees$$2docaml$$2d5$check_43_closure
	.quad	9
	.quad	1
	.quad	1
	.quad	1
	.quad	_camlBinarytrees$$2docaml$$2d5$loop_depths_113_closure
	.data
	.align	3
	.quad	1792
	.globl	_camlBinarytrees$$2docaml$$2d5__full_apply_381
_camlBinarytrees$$2docaml$$2d5__full_apply_381:
	.quad	1
	.data
	.align	3
	.quad	1792
	.globl	_camlBinarytrees$$2docaml$$2d5__apply_arg_241
_camlBinarytrees$$2docaml$$2d5__apply_arg_241:
	.quad	1
	.data
	.align	3
	.quad	1792
	.globl	_camlBinarytrees$$2docaml$$2d5__long_lived_tree_248
_camlBinarytrees$$2docaml$$2d5__long_lived_tree_248:
	.quad	1
	.data
	.align	3
	.quad	1792
	.globl	_camlBinarytrees$$2docaml$$2d5__full_apply_388
_camlBinarytrees$$2docaml$$2d5__full_apply_388:
	.quad	1
	.data
	.align	3
	.quad	1792
	.globl	_camlBinarytrees$$2docaml$$2d5__c_251
_camlBinarytrees$$2docaml$$2d5__c_251:
	.quad	1
	.data
	.align	3
	.quad	1792
	.globl	_camlBinarytrees$$2docaml$$2d5__apply_arg_252
_camlBinarytrees$$2docaml$$2d5__apply_arg_252:
	.quad	1
	.data
	.align	3
	.quad	1792
	.globl	_camlBinarytrees$$2docaml$$2d5__Paddint_253
_camlBinarytrees$$2docaml$$2d5__Paddint_253:
	.quad	1
	.data
	.align	3
	.quad	1792
	.globl	_camlBinarytrees$$2docaml$$2d5__max_depth_254
_camlBinarytrees$$2docaml$$2d5__max_depth_254:
	.quad	1
	.data
	.align	3
	.quad	1792
	.globl	_camlBinarytrees$$2docaml$$2d5__n_257
_camlBinarytrees$$2docaml$$2d5__n_257:
	.quad	1
	.data
	.align	3
	.globl	_camlBinarytrees$$2docaml$$2d5$gc_roots
_camlBinarytrees$$2docaml$$2d5$gc_roots:
	.quad	_camlBinarytrees$$2docaml$$2d5__n_257
	.quad	_camlBinarytrees$$2docaml$$2d5__max_depth_254
	.quad	_camlBinarytrees$$2docaml$$2d5__Paddint_253
	.quad	_camlBinarytrees$$2docaml$$2d5__apply_arg_252
	.quad	_camlBinarytrees$$2docaml$$2d5__c_251
	.quad	_camlBinarytrees$$2docaml$$2d5__full_apply_388
	.quad	_camlBinarytrees$$2docaml$$2d5__long_lived_tree_248
	.quad	_camlBinarytrees$$2docaml$$2d5__apply_arg_241
	.quad	_camlBinarytrees$$2docaml$$2d5__full_apply_381
	.quad	_camlBinarytrees$$2docaml$$2d5
	.quad	0
	.text
	.align	3
	.globl	_camlBinarytrees$$2docaml$$2d5$make_23
L102:
	mov	x16, #36
	stp	x16, x30, [sp, #-16]!
	bl	_caml_call_realloc_stack
	ldp	x16, x30, [sp], #16
_camlBinarytrees$$2docaml$$2d5$make_23:
	.cfi_startproc
	ldr	x16, [x28, #40]
	add	x16, x16, #344
	cmp	sp, x16
	bcc	L102
L103:
L104:
	str	x30, [sp, #-8]
	.cfi_offset 30, -8
	sub	sp, sp, #32
	.cfi_adjust_cfa_offset	32
	.ifne (. - L104) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L105:
L101:
	.ifne (. - L105) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L106:
	cmp	x0, #1
	b.ne	L100
	.ifne (. - L106) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L107:
	adrp	x0, _camlBinarytrees$$2docaml$$2d5__const_block_40@GOTPAGE
	ldr	x0, [x0, _camlBinarytrees$$2docaml$$2d5__const_block_40@GOTPAGEOFF]
	.ifne (. - L107) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L108:
	.ifne (. - L108) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L109:
	add	sp, sp, #32
	.cfi_adjust_cfa_offset	-32
	ldr	x30, [sp, #-8]
	ret
	.cfi_adjust_cfa_offset	32
	.ifne (. - L109) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L110:
L100:
	.ifne (. - L110) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L111:
	sub	x0, x0, #2
	.ifne (. - L111) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L112:
	str	x0, [sp, #0]
	.ifne (. - L112) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L113:
	bl	_camlBinarytrees$$2docaml$$2d5$make_23
L114:
	.ifne (. - L113) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L115:
	str	x0, [sp, #8]
	.ifne (. - L115) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L116:
	ldr	x0, [sp, #0]
	.ifne (. - L116) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L117:
	bl	_camlBinarytrees$$2docaml$$2d5$make_23
L118:
	.ifne (. - L117) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L119:
	ldr	x16, [x28, #0]
	sub	x27, x27, #24
	cmp	x27, x16
	b.lo	L122
L121:	add	x4, x27, #8
	.ifne (. - L119) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L123:
	orr	x5, xzr, #2048
	.ifne (. - L123) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L124:
	str	x5, [x4, #-8]
	.ifne (. - L124) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L125:
	str	x0, [x4, #0]
	.ifne (. - L125) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L126:
	ldr	x8, [sp, #8]
	.ifne (. - L126) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L127:
	str	x8, [x4, #8]
	.ifne (. - L127) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L128:
	mov	x0, x4
	.ifne (. - L128) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L129:
	.ifne (. - L129) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L130:
	add	sp, sp, #32
	.cfi_adjust_cfa_offset	-32
	ldr	x30, [sp, #-8]
	ret
	.cfi_adjust_cfa_offset	32
	.ifne (. - L130) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
	.ifne (. - L103) - 116
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L122:	bl	_caml_call_gc
L120:	b	L121
	.cfi_endproc
	.text
	.align	3
	.globl	_camlBinarytrees$$2docaml$$2d5$check_43
L133:
	mov	x16, #36
	stp	x16, x30, [sp, #-16]!
	bl	_caml_call_realloc_stack
	ldp	x16, x30, [sp], #16
_camlBinarytrees$$2docaml$$2d5$check_43:
	.cfi_startproc
	ldr	x16, [x28, #40]
	add	x16, x16, #344
	cmp	sp, x16
	bcc	L133
L134:
L135:
	str	x30, [sp, #-8]
	.cfi_offset 30, -8
	sub	sp, sp, #32
	.cfi_adjust_cfa_offset	32
	.ifne (. - L135) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L136:
L132:
	.ifne (. - L136) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L137:
	tbz	x0, #0, L131
	.ifne (. - L137) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L138:
	orr	x0, xzr, #1
	.ifne (. - L138) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L139:
	.ifne (. - L139) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L140:
	add	sp, sp, #32
	.cfi_adjust_cfa_offset	-32
	ldr	x30, [sp, #-8]
	ret
	.cfi_adjust_cfa_offset	32
	.ifne (. - L140) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L141:
L131:
	.ifne (. - L141) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L142:
	str	x0, [sp, #0]
	.ifne (. - L142) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L143:
	ldr	x0, [x0, #8]
	.ifne (. - L143) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L144:
	bl	_camlBinarytrees$$2docaml$$2d5$check_43
L145:
	.ifne (. - L144) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L146:
	str	x0, [sp, #8]
	.ifne (. - L146) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L147:
	ldr	x8, [sp, #0]
	.ifne (. - L147) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L148:
	ldr	x0, [x8, #0]
	.ifne (. - L148) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L149:
	bl	_camlBinarytrees$$2docaml$$2d5$check_43
L150:
	.ifne (. - L149) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L151:
	ldr	x9, [sp, #8]
	.ifne (. - L151) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L152:
	add	x5, x0, x9
	.ifne (. - L152) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L153:
	add	x0, x5, #1
	.ifne (. - L153) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L154:
	.ifne (. - L154) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L155:
	add	sp, sp, #32
	.cfi_adjust_cfa_offset	-32
	ldr	x30, [sp, #-8]
	ret
	.cfi_adjust_cfa_offset	32
	.ifne (. - L155) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
	.ifne (. - L134) - 80
	.error "Emit.instr_size: instruction length mismatch"
	.endif
	.cfi_endproc
	.text
	.align	3
	.globl	_camlBinarytrees$$2docaml$$2d5$loop_depths_113
L161:
	mov	x16, #40
	stp	x16, x30, [sp, #-16]!
	bl	_caml_call_realloc_stack
	ldp	x16, x30, [sp], #16
_camlBinarytrees$$2docaml$$2d5$loop_depths_113:
	.cfi_startproc
	ldr	x16, [x28, #40]
	add	x16, x16, #376
	cmp	sp, x16
	bcc	L161
L162:
L163:
	str	x30, [sp, #-8]
	.cfi_offset 30, -8
	sub	sp, sp, #64
	.cfi_adjust_cfa_offset	64
	.ifne (. - L163) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L164:
L160:
	.ifne (. - L164) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L165:
	adrp	x1, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGE
	ldr	x1, [x1, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGEOFF]
	.ifne (. - L165) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L166:
	ldr	x2, [x1, #0]
	.ifne (. - L166) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L167:
	sub	x3, x2, x0
	.ifne (. - L167) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L168:
	add	x4, x3, #1
	.ifne (. - L168) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L169:
	asr	x5, x4, #1
	.ifne (. - L169) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L170:
	lsr	x6, x5, #63
	.ifne (. - L170) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L171:
	add	x7, x5, x6
	.ifne (. - L171) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L172:
	orr	x8, x7, #1
	.ifne (. - L172) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L173:
	orr	x9, xzr, #1
	.ifne (. - L173) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L174:
	cmp	x9, x8
	b.gt	L156
	.ifne (. - L174) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L175:
	str	x9, [sp, #16]
	.ifne (. - L175) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L176:
	str	x8, [sp, #24]
	.ifne (. - L176) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L177:
	str	x0, [sp, #0]
	.ifne (. - L177) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L178:
L157:
	.ifne (. - L178) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L179:
	lsl	x13, x9, #1
	.ifne (. - L179) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L180:
	ldr	x6, [sp, #0]
	.ifne (. - L180) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L181:
	add	x14, x6, x13
	.ifne (. - L181) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L182:
	sub	x15, x14, #2
	.ifne (. - L182) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L183:
	str	x15, [sp, #32]
	.ifne (. - L183) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L184:
	adrp	x19, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGE
	ldr	x19, [x19, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGEOFF]
	.ifne (. - L184) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L185:
	ldr	x20, [x19, #0]
	.ifne (. - L185) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L186:
	sub	x21, x20, x15
	.ifne (. - L186) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L187:
	add	x22, x21, #9
	.ifne (. - L187) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L188:
	asr	x23, x22, #1
	.ifne (. - L188) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L189:
	orr	x24, xzr, #2
	.ifne (. - L189) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L190:
	lsl	x25, x24, x23
	.ifne (. - L190) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L191:
	add	x0, x25, #1
	.ifne (. - L191) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L192:
	str	x0, [sp, #48]
	.ifne (. - L192) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L193:
	orr	x1, xzr, #1
	.ifne (. - L193) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L194:
	str	x1, [sp, #8]
	.ifne (. - L194) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L195:
	orr	x3, xzr, #3
	.ifne (. - L195) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L196:
	str	x3, [sp, #40]
	.ifne (. - L196) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L197:
	cmp	x3, x0
	b.gt	L158
	.ifne (. - L197) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L198:
L159:
	.ifne (. - L198) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L199:
	ldr	x0, [sp, #32]
	.ifne (. - L199) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L200:
	bl	_camlBinarytrees$$2docaml$$2d5$make_23
L201:
	.ifne (. - L200) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L202:
	bl	_camlBinarytrees$$2docaml$$2d5$check_43
L203:
	.ifne (. - L202) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L204:
	ldr	x1, [sp, #8]
	.ifne (. - L204) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L205:
	add	x9, x1, x0
	.ifne (. - L205) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L206:
	sub	x1, x9, #1
	.ifne (. - L206) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L207:
	str	x1, [sp, #8]
	.ifne (. - L207) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L208:
	ldr	x12, [sp, #40]
	.ifne (. - L208) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L209:
	mov	x11, x12
	.ifne (. - L209) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L210:
	add	x12, x12, #2
	.ifne (. - L210) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L211:
	str	x12, [sp, #40]
	.ifne (. - L211) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L212:
	ldr	x10, [sp, #48]
	.ifne (. - L212) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L213:
	cmp	x11, x10
	b.eq	L158
	.ifne (. - L213) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L214:
	ldr	x16, [x28, #0]
	cmp	x27, x16
	b.hi	L159
	b	L216
	.ifne (. - L214) - 16
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L217:
L158:
	.ifne (. - L217) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L218:
	str	x1, [sp, #8]
	.ifne (. - L218) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L219:
	orr	x6, xzr, #7
	.ifne (. - L219) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L220:
	adrp	x5, _camlCamlinternalFormat$convert_int_4861_closure@GOTPAGE
	ldr	x5, [x5, _camlCamlinternalFormat$convert_int_4861_closure@GOTPAGEOFF]
	.ifne (. - L220) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L221:
	orr	x4, xzr, #1
	.ifne (. - L221) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L222:
	orr	x3, xzr, #1
	.ifne (. - L222) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L223:
	adrp	x2, _camlBinarytrees$$2docaml$$2d5__const_block_160@GOTPAGE
	ldr	x2, [x2, _camlBinarytrees$$2docaml$$2d5__const_block_160@GOTPAGEOFF]
	.ifne (. - L223) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L224:
	orr	x1, xzr, #1
	.ifne (. - L224) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L225:
	adrp	x0, _camlStdlib__Printf$anon_fn$$5bprintf$$2eml$$3a20$$2c14$$2d$$2d48$$5d_409_closure@GOTPAGE
	ldr	x0, [x0, _camlStdlib__Printf$anon_fn$$5bprintf$$2eml$$3a20$$2c14$$2d$$2d48$$5d_409_closure@GOTPAGEOFF]
	.ifne (. - L225) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L226:
	bl	_camlCamlinternalFormat$make_int_padding_precision_5102
L227:
	.ifne (. - L226) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L228:
	mov	x3, x0
	.ifne (. - L228) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L229:
	ldr	x0, [sp, #48]
	.ifne (. - L229) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L230:
	ldr	x1, [sp, #32]
	.ifne (. - L230) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L231:
	ldr	x2, [sp, #8]
	.ifne (. - L231) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L232:
	bl	_caml_apply3
L233:
	.ifne (. - L232) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L234:
	ldr	x9, [sp, #16]
	.ifne (. - L234) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L235:
	mov	x2, x9
	.ifne (. - L235) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L236:
	add	x9, x9, #2
	.ifne (. - L236) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L237:
	str	x9, [sp, #16]
	.ifne (. - L237) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L238:
	ldr	x15, [sp, #24]
	.ifne (. - L238) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L239:
	cmp	x2, x15
	b.eq	L156
	.ifne (. - L239) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L240:
	ldr	x16, [x28, #0]
	cmp	x27, x16
	b.hi	L157
	b	L242
	.ifne (. - L240) - 16
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L243:
L156:
	.ifne (. - L243) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L244:
	orr	x0, xzr, #1
	.ifne (. - L244) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L245:
	.ifne (. - L245) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L246:
	add	sp, sp, #64
	.cfi_adjust_cfa_offset	-64
	ldr	x30, [sp, #-8]
	ret
	.cfi_adjust_cfa_offset	64
	.ifne (. - L246) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
	.ifne (. - L162) - 352
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L242:	bl	_caml_call_gc
L241:	b	L157
L216:	bl	_caml_call_gc
L215:	b	L159
	.cfi_endproc
	.data
	.align	3
	.quad	2818
	.globl	_camlBinarytrees$$2docaml$$2d5__Pmakeblock_771
_camlBinarytrees$$2docaml$$2d5__Pmakeblock_771:
	.quad	1
	.quad	_camlBinarytrees$$2docaml$$2d5__const_immstring_198
	.data
	.align	3
	.quad	2818
	.globl	_camlBinarytrees$$2docaml$$2d5__Pmakeblock_770
_camlBinarytrees$$2docaml$$2d5__Pmakeblock_770:
	.quad	1
	.quad	_camlBinarytrees$$2docaml$$2d5__const_immstring_95
	.data
	.align	3
	.data
	.align	3
	.data
	.align	3
	.data
	.align	3
	.quad	4868
	.globl	_camlBinarytrees$$2docaml$$2d5__const_block_203
_camlBinarytrees$$2docaml$$2d5__const_block_203:
	.quad	7
	.quad	1
	.quad	1
	.quad	_camlBinarytrees$$2docaml$$2d5__const_block_202
	.data
	.align	3
	.quad	2827
	.globl	_camlBinarytrees$$2docaml$$2d5__const_block_202
_camlBinarytrees$$2docaml$$2d5__const_block_202:
	.quad	_camlBinarytrees$$2docaml$$2d5__const_immstring_199
	.quad	_camlBinarytrees$$2docaml$$2d5__const_block_201
	.data
	.align	3
	.quad	4868
	.globl	_camlBinarytrees$$2docaml$$2d5__const_block_201
_camlBinarytrees$$2docaml$$2d5__const_block_201:
	.quad	7
	.quad	1
	.quad	1
	.quad	_camlBinarytrees$$2docaml$$2d5__const_block_200
	.data
	.align	3
	.quad	2828
	.globl	_camlBinarytrees$$2docaml$$2d5__const_block_200
_camlBinarytrees$$2docaml$$2d5__const_block_200:
	.quad	21
	.quad	1
	.data
	.align	3
	.quad	3068
	.globl	_camlBinarytrees$$2docaml$$2d5__const_immstring_199
_camlBinarytrees$$2docaml$$2d5__const_immstring_199:
	.ascii  "\11 check: "
	.space	6
	.byte	6
	.data
	.align	3
	.quad	5116
	.globl	_camlBinarytrees$$2docaml$$2d5__const_immstring_198
_camlBinarytrees$$2docaml$$2d5__const_immstring_198:
	.ascii  "long lived tree of depth "
	.space	6
	.byte	6
	.data
	.align	3
	.quad	2827
	.globl	_camlBinarytrees$$2docaml$$2d5__const_block_160
_camlBinarytrees$$2docaml$$2d5__const_block_160:
	.quad	_camlBinarytrees$$2docaml$$2d5__const_immstring_154
	.quad	_camlBinarytrees$$2docaml$$2d5__const_block_203
	.data
	.align	3
	.quad	4092
	.globl	_camlBinarytrees$$2docaml$$2d5__const_immstring_154
_camlBinarytrees$$2docaml$$2d5__const_immstring_154:
	.ascii  "\11 trees of depth "
	.space	6
	.byte	6
	.data
	.align	3
	.quad	4092
	.globl	_camlBinarytrees$$2docaml$$2d5__const_immstring_95
_camlBinarytrees$$2docaml$$2d5__const_immstring_95:
	.ascii  "stretch tree of depth "
	.space	1
	.byte	1
	.data
	.align	3
	.quad	2816
	.globl	_camlBinarytrees$$2docaml$$2d5__const_block_40
_camlBinarytrees$$2docaml$$2d5__const_block_40:
	.quad	1
	.quad	1
	.text
	.align	3
	.globl	_camlBinarytrees$$2docaml$$2d5$entry
L256:
	mov	x16, #42
	stp	x16, x30, [sp, #-16]!
	bl	_caml_call_realloc_stack
	ldp	x16, x30, [sp], #16
_camlBinarytrees$$2docaml$$2d5$entry:
	.cfi_startproc
	ldr	x16, [x28, #40]
	add	x16, x16, #392
	cmp	sp, x16
	bcc	L256
L257:
L258:
	str	x30, [sp, #-8]
	.cfi_offset 30, -8
	sub	sp, sp, #64
	.cfi_adjust_cfa_offset	64
	.ifne (. - L258) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L259:
L255:
	.ifne (. - L259) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L260:
	adr	x16, L254
	stp	x26, x16, [sp, -16]!
	.cfi_adjust_cfa_offset	16
	mov	x26, sp
	.ifne (. - L260) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L261:
	orr	x0, xzr, #1
	.ifne (. - L261) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L262:
	adrp	x8, _caml_sys_argv@GOTPAGE
	ldr	x8, [x8, _caml_sys_argv@GOTPAGEOFF]
	bl	_caml_c_call
L263:
	.ifne (. - L262) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L264:
	ldr	x2, [x0, #-8]
	.ifne (. - L264) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L265:
	cmp	x2, #2047
	b.ls	L266
	.ifne (. - L265) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L268:
	ldr	x0, [x0, #8]
	.ifne (. - L268) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L269:
	adrp	x8, _caml_int_of_string@GOTPAGE
	ldr	x8, [x8, _caml_int_of_string@GOTPAGEOFF]
	bl	_caml_c_call
L270:
	.ifne (. - L269) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L271:
	mov	x1, x0
	.ifne (. - L271) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L272:
	ldr	x26, [sp], 16
	.cfi_adjust_cfa_offset	-16
	.ifne (. - L272) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L273:
	b	L253
	.ifne (. - L273) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L274:
L254:
	.ifne (. - L274) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L275:
	.ifne (. - L275) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L276:
	movz	x1, #21, lsl #0
	.ifne (. - L276) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L277:
L253:
	.ifne (. - L277) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L278:
	adrp	x0, _camlBinarytrees$$2docaml$$2d5__n_257@GOTPAGE
	ldr	x0, [x0, _camlBinarytrees$$2docaml$$2d5__n_257@GOTPAGEOFF]
	.ifne (. - L278) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L279:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L279) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L280:
	adrp	x8, _camlBinarytrees$$2docaml$$2d5__n_257@GOTPAGE
	ldr	x8, [x8, _camlBinarytrees$$2docaml$$2d5__n_257@GOTPAGEOFF]
	.ifne (. - L280) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L281:
	ldr	x1, [x8, #0]
	.ifne (. - L281) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L282:
	str	x1, [sp, #0]
	.ifne (. - L282) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L283:
	movz	x0, #13, lsl #0
	.ifne (. - L283) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L284:
	adrp	x8, _caml_greaterequal@GOTPAGE
	ldr	x8, [x8, _caml_greaterequal@GOTPAGEOFF]
	bl	_caml_c_call
L285:
	.ifne (. - L284) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L286:
	cmp	x0, #1
	b.eq	L252
	.ifne (. - L286) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L287:
	movz	x1, #13, lsl #0
	.ifne (. - L287) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L288:
	b	L251
	.ifne (. - L288) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L289:
L252:
	.ifne (. - L289) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L290:
	ldr	x1, [sp, #0]
	.ifne (. - L290) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L291:
L251:
	.ifne (. - L291) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L292:
	adrp	x0, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGE
	ldr	x0, [x0, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGEOFF]
	.ifne (. - L292) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L293:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L293) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L294:
	adrp	x15, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGE
	ldr	x15, [x15, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGEOFF]
	.ifne (. - L294) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L295:
	ldr	x19, [x15, #0]
	.ifne (. - L295) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L296:
	add	x1, x19, #2
	.ifne (. - L296) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L297:
	adrp	x0, _camlBinarytrees$$2docaml$$2d5__Paddint_253@GOTPAGE
	ldr	x0, [x0, _camlBinarytrees$$2docaml$$2d5__Paddint_253@GOTPAGEOFF]
	.ifne (. - L297) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L298:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L298) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L299:
	adrp	x22, _camlBinarytrees$$2docaml$$2d5__Paddint_253@GOTPAGE
	ldr	x22, [x22, _camlBinarytrees$$2docaml$$2d5__Paddint_253@GOTPAGEOFF]
	.ifne (. - L299) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L300:
	ldr	x0, [x22, #0]
	.ifne (. - L300) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L301:
	bl	_camlBinarytrees$$2docaml$$2d5$make_23
L302:
	.ifne (. - L301) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L303:
	mov	x1, x0
	.ifne (. - L303) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L304:
	adrp	x0, _camlBinarytrees$$2docaml$$2d5__apply_arg_252@GOTPAGE
	ldr	x0, [x0, _camlBinarytrees$$2docaml$$2d5__apply_arg_252@GOTPAGEOFF]
	.ifne (. - L304) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L305:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L305) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L306:
	adrp	x0, _camlBinarytrees$$2docaml$$2d5__apply_arg_252@GOTPAGE
	ldr	x0, [x0, _camlBinarytrees$$2docaml$$2d5__apply_arg_252@GOTPAGEOFF]
	.ifne (. - L306) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L307:
	ldr	x0, [x0, #0]
	.ifne (. - L307) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L308:
	bl	_camlBinarytrees$$2docaml$$2d5$check_43
L309:
	.ifne (. - L308) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L310:
	mov	x1, x0
	.ifne (. - L310) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L311:
	adrp	x0, _camlBinarytrees$$2docaml$$2d5__c_251@GOTPAGE
	ldr	x0, [x0, _camlBinarytrees$$2docaml$$2d5__c_251@GOTPAGEOFF]
	.ifne (. - L311) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L312:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L312) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L313:
	adrp	x2, _camlBinarytrees$$2docaml$$2d5__const_block_203@GOTPAGE
	ldr	x2, [x2, _camlBinarytrees$$2docaml$$2d5__const_block_203@GOTPAGEOFF]
	.ifne (. - L313) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L314:
	adrp	x1, _camlBinarytrees$$2docaml$$2d5__Pmakeblock_770@GOTPAGE
	ldr	x1, [x1, _camlBinarytrees$$2docaml$$2d5__Pmakeblock_770@GOTPAGEOFF]
	.ifne (. - L314) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L315:
	adrp	x0, _camlStdlib__Printf$anon_fn$$5bprintf$$2eml$$3a20$$2c14$$2d$$2d48$$5d_409_closure@GOTPAGE
	ldr	x0, [x0, _camlStdlib__Printf$anon_fn$$5bprintf$$2eml$$3a20$$2c14$$2d$$2d48$$5d_409_closure@GOTPAGEOFF]
	.ifne (. - L315) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L316:
	bl	_camlCamlinternalFormat$make_printf_5097
L317:
	.ifne (. - L316) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L318:
	mov	x1, x0
	.ifne (. - L318) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L319:
	adrp	x0, _camlBinarytrees$$2docaml$$2d5__full_apply_388@GOTPAGE
	ldr	x0, [x0, _camlBinarytrees$$2docaml$$2d5__full_apply_388@GOTPAGEOFF]
	.ifne (. - L319) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L320:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L320) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L321:
	adrp	x9, _camlBinarytrees$$2docaml$$2d5__full_apply_388@GOTPAGE
	ldr	x9, [x9, _camlBinarytrees$$2docaml$$2d5__full_apply_388@GOTPAGEOFF]
	.ifne (. - L321) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L322:
	ldr	x2, [x9, #0]
	.ifne (. - L322) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L323:
	adrp	x11, _camlBinarytrees$$2docaml$$2d5__c_251@GOTPAGE
	ldr	x11, [x11, _camlBinarytrees$$2docaml$$2d5__c_251@GOTPAGEOFF]
	.ifne (. - L323) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L324:
	ldr	x1, [x11, #0]
	.ifne (. - L324) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L325:
	adrp	x13, _camlBinarytrees$$2docaml$$2d5__Paddint_253@GOTPAGE
	ldr	x13, [x13, _camlBinarytrees$$2docaml$$2d5__Paddint_253@GOTPAGEOFF]
	.ifne (. - L325) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L326:
	ldr	x0, [x13, #0]
	.ifne (. - L326) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L327:
	bl	_caml_apply2
L328:
	.ifne (. - L327) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L329:
	adrp	x15, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGE
	ldr	x15, [x15, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGEOFF]
	.ifne (. - L329) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L330:
	ldr	x0, [x15, #0]
	.ifne (. - L330) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L331:
	bl	_camlBinarytrees$$2docaml$$2d5$make_23
L332:
	.ifne (. - L331) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L333:
	mov	x1, x0
	.ifne (. - L333) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L334:
	adrp	x0, _camlBinarytrees$$2docaml$$2d5__long_lived_tree_248@GOTPAGE
	ldr	x0, [x0, _camlBinarytrees$$2docaml$$2d5__long_lived_tree_248@GOTPAGEOFF]
	.ifne (. - L334) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L335:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L335) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L336:
	adrp	x22, _camlStdlib@GOTPAGE
	ldr	x22, [x22, _camlStdlib@GOTPAGEOFF]
	.ifne (. - L336) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L337:
	ldr	x0, [x22, #304]
	.ifne (. - L337) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L338:
	adrp	x8, _caml_ml_flush@GOTPAGE
	ldr	x8, [x8, _caml_ml_flush@GOTPAGEOFF]
	bl	_caml_c_call
L339:
	.ifne (. - L338) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L340:
	adrp	x24, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGE
	ldr	x24, [x24, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGEOFF]
	.ifne (. - L340) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L341:
	ldr	x25, [x24, #0]
	.ifne (. - L341) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L342:
	sub	x0, x25, #8
	.ifne (. - L342) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L343:
	asr	x1, x0, #1
	.ifne (. - L343) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L344:
	lsr	x2, x1, #63
	.ifne (. - L344) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L345:
	add	x3, x1, x2
	.ifne (. - L345) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L346:
	orr	x4, x3, #1
	.ifne (. - L346) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L347:
	orr	x5, xzr, #1
	.ifne (. - L347) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L348:
	cmp	x5, x4
	b.gt	L247
	.ifne (. - L348) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L349:
	str	x5, [sp, #8]
	.ifne (. - L349) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L350:
	str	x4, [sp, #16]
	.ifne (. - L350) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L351:
L248:
	.ifne (. - L351) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L352:
	lsl	x9, x5, #1
	.ifne (. - L352) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L353:
	add	x10, x9, #7
	.ifne (. - L353) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L354:
	str	x10, [sp, #24]
	.ifne (. - L354) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L355:
	adrp	x11, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGE
	ldr	x11, [x11, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGEOFF]
	.ifne (. - L355) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L356:
	ldr	x12, [x11, #0]
	.ifne (. - L356) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L357:
	sub	x13, x12, x10
	.ifne (. - L357) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L358:
	add	x14, x13, #9
	.ifne (. - L358) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L359:
	asr	x15, x14, #1
	.ifne (. - L359) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L360:
	orr	x19, xzr, #2
	.ifne (. - L360) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L361:
	lsl	x20, x19, x15
	.ifne (. - L361) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L362:
	add	x1, x20, #1
	.ifne (. - L362) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L363:
	str	x1, [sp, #40]
	.ifne (. - L363) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L364:
	orr	x22, xzr, #1
	.ifne (. - L364) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L365:
	str	x22, [sp, #0]
	.ifne (. - L365) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L366:
	orr	x24, xzr, #3
	.ifne (. - L366) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L367:
	str	x24, [sp, #32]
	.ifne (. - L367) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L368:
	cmp	x24, x1
	b.gt	L249
	.ifne (. - L368) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L369:
L250:
	.ifne (. - L369) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L370:
	ldr	x0, [sp, #24]
	.ifne (. - L370) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L371:
	bl	_camlBinarytrees$$2docaml$$2d5$make_23
L372:
	.ifne (. - L371) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L373:
	bl	_camlBinarytrees$$2docaml$$2d5$check_43
L374:
	.ifne (. - L373) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L375:
	ldr	x22, [sp, #0]
	.ifne (. - L375) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L376:
	add	x4, x22, x0
	.ifne (. - L376) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L377:
	sub	x22, x4, #1
	.ifne (. - L377) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L378:
	str	x22, [sp, #0]
	.ifne (. - L378) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L379:
	ldr	x7, [sp, #32]
	.ifne (. - L379) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L380:
	mov	x6, x7
	.ifne (. - L380) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L381:
	add	x7, x7, #2
	.ifne (. - L381) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L382:
	str	x7, [sp, #32]
	.ifne (. - L382) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L383:
	ldr	x1, [sp, #40]
	.ifne (. - L383) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L384:
	cmp	x6, x1
	b.eq	L249
	.ifne (. - L384) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L385:
	ldr	x16, [x28, #0]
	cmp	x27, x16
	b.hi	L250
	b	L387
	.ifne (. - L385) - 16
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L388:
L249:
	.ifne (. - L388) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L389:
	str	x22, [sp, #0]
	.ifne (. - L389) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L390:
	orr	x0, xzr, #7
	.ifne (. - L390) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L391:
	bl	_camlCamlinternalFormat$convert_int_4861
L392:
	.ifne (. - L391) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L393:
	orr	x8, xzr, #48
	bl	_caml_allocN
L394:	add	x13, x27, #8
	.ifne (. - L393) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L395:
	add	x13, x13, #24
	.ifne (. - L395) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L396:
	movz	x14, #2052, lsl #0
	.ifne (. - L396) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L397:
	str	x14, [x13, #-8]
	.ifne (. - L397) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L398:
	orr	x15, xzr, #1
	.ifne (. - L398) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L399:
	str	x15, [x13, #0]
	.ifne (. - L399) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L400:
	str	x0, [x13, #8]
	.ifne (. - L400) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L401:
	sub	x1, x13, #24
	.ifne (. - L401) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L402:
	movz	x20, #2050, lsl #0
	.ifne (. - L402) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L403:
	str	x20, [x1, #-8]
	.ifne (. - L403) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L404:
	str	x13, [x1, #0]
	.ifne (. - L404) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L405:
	adrp	x21, _camlBinarytrees$$2docaml$$2d5__const_immstring_154@GOTPAGE
	ldr	x21, [x21, _camlBinarytrees$$2docaml$$2d5__const_immstring_154@GOTPAGEOFF]
	.ifne (. - L405) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L406:
	str	x21, [x1, #8]
	.ifne (. - L406) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L407:
	adrp	x2, _camlBinarytrees$$2docaml$$2d5__const_block_203@GOTPAGE
	ldr	x2, [x2, _camlBinarytrees$$2docaml$$2d5__const_block_203@GOTPAGEOFF]
	.ifne (. - L407) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L408:
	adrp	x0, _camlStdlib__Printf$anon_fn$$5bprintf$$2eml$$3a20$$2c14$$2d$$2d48$$5d_409_closure@GOTPAGE
	ldr	x0, [x0, _camlStdlib__Printf$anon_fn$$5bprintf$$2eml$$3a20$$2c14$$2d$$2d48$$5d_409_closure@GOTPAGEOFF]
	.ifne (. - L408) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L409:
	bl	_camlCamlinternalFormat$make_printf_5097
L410:
	.ifne (. - L409) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L411:
	mov	x2, x0
	.ifne (. - L411) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L412:
	ldr	x0, [sp, #24]
	.ifne (. - L412) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L413:
	ldr	x1, [sp, #0]
	.ifne (. - L413) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L414:
	bl	_caml_apply2
L415:
	.ifne (. - L414) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L416:
	ldr	x5, [sp, #8]
	.ifne (. - L416) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L417:
	mov	x0, x5
	.ifne (. - L417) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L418:
	add	x5, x5, #2
	.ifne (. - L418) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L419:
	str	x5, [sp, #8]
	.ifne (. - L419) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L420:
	ldr	x19, [sp, #16]
	.ifne (. - L420) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L421:
	cmp	x0, x19
	b.eq	L247
	.ifne (. - L421) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L422:
	ldr	x16, [x28, #0]
	cmp	x27, x16
	b.hi	L248
	b	L424
	.ifne (. - L422) - 16
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L425:
L247:
	.ifne (. - L425) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L426:
	adrp	x3, _camlBinarytrees$$2docaml$$2d5__long_lived_tree_248@GOTPAGE
	ldr	x3, [x3, _camlBinarytrees$$2docaml$$2d5__long_lived_tree_248@GOTPAGEOFF]
	.ifne (. - L426) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L427:
	ldr	x0, [x3, #0]
	.ifne (. - L427) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L428:
	bl	_camlBinarytrees$$2docaml$$2d5$check_43
L429:
	.ifne (. - L428) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L430:
	mov	x1, x0
	.ifne (. - L430) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L431:
	adrp	x0, _camlBinarytrees$$2docaml$$2d5__apply_arg_241@GOTPAGE
	ldr	x0, [x0, _camlBinarytrees$$2docaml$$2d5__apply_arg_241@GOTPAGEOFF]
	.ifne (. - L431) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L432:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L432) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L433:
	adrp	x2, _camlBinarytrees$$2docaml$$2d5__const_block_203@GOTPAGE
	ldr	x2, [x2, _camlBinarytrees$$2docaml$$2d5__const_block_203@GOTPAGEOFF]
	.ifne (. - L433) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L434:
	adrp	x1, _camlBinarytrees$$2docaml$$2d5__Pmakeblock_771@GOTPAGE
	ldr	x1, [x1, _camlBinarytrees$$2docaml$$2d5__Pmakeblock_771@GOTPAGEOFF]
	.ifne (. - L434) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L435:
	adrp	x0, _camlStdlib__Printf$anon_fn$$5bprintf$$2eml$$3a20$$2c14$$2d$$2d48$$5d_409_closure@GOTPAGE
	ldr	x0, [x0, _camlStdlib__Printf$anon_fn$$5bprintf$$2eml$$3a20$$2c14$$2d$$2d48$$5d_409_closure@GOTPAGEOFF]
	.ifne (. - L435) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L436:
	bl	_camlCamlinternalFormat$make_printf_5097
L437:
	.ifne (. - L436) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L438:
	mov	x1, x0
	.ifne (. - L438) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L439:
	adrp	x0, _camlBinarytrees$$2docaml$$2d5__full_apply_381@GOTPAGE
	ldr	x0, [x0, _camlBinarytrees$$2docaml$$2d5__full_apply_381@GOTPAGEOFF]
	.ifne (. - L439) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L440:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L440) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L441:
	adrp	x12, _camlBinarytrees$$2docaml$$2d5__full_apply_381@GOTPAGE
	ldr	x12, [x12, _camlBinarytrees$$2docaml$$2d5__full_apply_381@GOTPAGEOFF]
	.ifne (. - L441) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L442:
	ldr	x2, [x12, #0]
	.ifne (. - L442) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L443:
	adrp	x14, _camlBinarytrees$$2docaml$$2d5__apply_arg_241@GOTPAGE
	ldr	x14, [x14, _camlBinarytrees$$2docaml$$2d5__apply_arg_241@GOTPAGEOFF]
	.ifne (. - L443) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L444:
	ldr	x1, [x14, #0]
	.ifne (. - L444) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L445:
	adrp	x19, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGE
	ldr	x19, [x19, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGEOFF]
	.ifne (. - L445) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L446:
	ldr	x0, [x19, #0]
	.ifne (. - L446) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L447:
	bl	_caml_apply2
L448:
	.ifne (. - L447) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L449:
	adrp	x21, _camlBinarytrees$$2docaml$$2d5__long_lived_tree_248@GOTPAGE
	ldr	x21, [x21, _camlBinarytrees$$2docaml$$2d5__long_lived_tree_248@GOTPAGEOFF]
	.ifne (. - L449) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L450:
	ldr	x1, [x21, #0]
	.ifne (. - L450) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L451:
	adrp	x23, _camlBinarytrees$$2docaml$$2d5@GOTPAGE
	ldr	x23, [x23, _camlBinarytrees$$2docaml$$2d5@GOTPAGEOFF]
	.ifne (. - L451) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L452:
	add	x0, x23, #40
	.ifne (. - L452) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L453:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L453) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L454:
	adrp	x25, _camlBinarytrees$$2docaml$$2d5__Paddint_253@GOTPAGE
	ldr	x25, [x25, _camlBinarytrees$$2docaml$$2d5__Paddint_253@GOTPAGEOFF]
	.ifne (. - L454) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L455:
	ldr	x1, [x25, #0]
	.ifne (. - L455) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L456:
	adrp	x2, _camlBinarytrees$$2docaml$$2d5@GOTPAGE
	ldr	x2, [x2, _camlBinarytrees$$2docaml$$2d5@GOTPAGEOFF]
	.ifne (. - L456) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L457:
	add	x0, x2, #32
	.ifne (. - L457) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L458:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L458) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L459:
	adrp	x3, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGE
	ldr	x3, [x3, _camlBinarytrees$$2docaml$$2d5__max_depth_254@GOTPAGEOFF]
	.ifne (. - L459) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L460:
	ldr	x1, [x3, #0]
	.ifne (. - L460) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L461:
	adrp	x5, _camlBinarytrees$$2docaml$$2d5@GOTPAGE
	ldr	x5, [x5, _camlBinarytrees$$2docaml$$2d5@GOTPAGEOFF]
	.ifne (. - L461) - 8
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L462:
	add	x0, x5, #24
	.ifne (. - L462) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L463:
	mov	x19, sp
	.cfi_remember_state
	.cfi_def_cfa_register 19
	ldr	x16, [x28, 64]
	mov	sp, x16
	bl	_caml_initialize
	mov	sp, x19
	.cfi_restore_state
	.ifne (. - L463) - 20
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L464:
	orr	x0, xzr, #1
	.ifne (. - L464) - 4
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L465:
	.ifne (. - L465) - 0
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L466:
	add	sp, sp, #64
	.cfi_adjust_cfa_offset	-64
	ldr	x30, [sp, #-8]
	ret
	.cfi_adjust_cfa_offset	64
	.ifne (. - L466) - 12
	.error "Emit.instr_size: instruction length mismatch"
	.endif
	.ifne (. - L257) - 1152
	.error "Emit.instr_size: instruction length mismatch"
	.endif
L424:	bl	_caml_call_gc
L423:	b	L248
L387:	bl	_caml_call_gc
L386:	b	L250
L266:	bl	_caml_ml_array_bound_error
L267:
	.cfi_endproc
	.data
	.align	3
	.text
	.globl	_camlBinarytrees$$2docaml$$2d5$code_end
_camlBinarytrees$$2docaml$$2d5$code_end:
	.data
	.quad	0
	.globl	_camlBinarytrees$$2docaml$$2d5$data_end
_camlBinarytrees$$2docaml$$2d5$data_end:
	.quad	0
	.align	3
	.globl	_camlBinarytrees$$2docaml$$2d5$frametable
_camlBinarytrees$$2docaml$$2d5$frametable:
	.quad	32
	.quad	L448
	.short	65
	.short	0
	.align	2
	.long	L467 - . + 0x0
	.align	3
	.quad	L437
	.short	65
	.short	0
	.align	2
	.long	L468 - . + 0x0
	.align	3
	.quad	L429
	.short	65
	.short	0
	.align	2
	.long	L469 - . + 0x0
	.align	3
	.quad	L423
	.short	66
	.short	0
	.byte	0
	.align	3
	.quad	L415
	.short	65
	.short	0
	.align	2
	.long	L470 - . + 0x0
	.align	3
	.quad	L410
	.short	65
	.short	0
	.align	2
	.long	L471 - . + 0x0
	.align	3
	.quad	L394
	.short	66
	.short	1
	.short	1
	.byte	2
	.byte	1
	.byte	1
	.align	3
	.quad	L392
	.short	65
	.short	0
	.align	2
	.long	L472 - . + 0x0
	.align	3
	.quad	L386
	.short	66
	.short	0
	.byte	0
	.align	3
	.quad	L374
	.short	65
	.short	0
	.align	2
	.long	L473 - . + 0x0
	.align	3
	.quad	L372
	.short	65
	.short	0
	.align	2
	.long	L474 - . + 0x0
	.align	3
	.quad	L339
	.short	65
	.short	0
	.align	2
	.long	L475 - . + 0x0
	.align	3
	.quad	L332
	.short	65
	.short	0
	.align	2
	.long	L476 - . + 0x0
	.align	3
	.quad	L328
	.short	65
	.short	0
	.align	2
	.long	L477 - . + 0x0
	.align	3
	.quad	L317
	.short	65
	.short	0
	.align	2
	.long	L478 - . + 0x0
	.align	3
	.quad	L309
	.short	65
	.short	0
	.align	2
	.long	L479 - . + 0x0
	.align	3
	.quad	L302
	.short	65
	.short	0
	.align	2
	.long	L480 - . + 0x0
	.align	3
	.quad	L285
	.short	65
	.short	1
	.short	0
	.align	2
	.long	L481 - . + 0x0
	.align	3
	.quad	L270
	.short	81
	.short	0
	.align	2
	.long	L482 - . + 0x0
	.align	3
	.quad	L267
	.short	81
	.short	0
	.align	2
	.long	L483 - . + 0x0
	.align	3
	.quad	L263
	.short	81
	.short	0
	.align	2
	.long	L484 - . + 0x0
	.align	3
	.quad	L241
	.short	66
	.short	1
	.short	0
	.byte	0
	.align	3
	.quad	L233
	.short	65
	.short	1
	.short	0
	.align	2
	.long	L485 - . + 0x0
	.align	3
	.quad	L227
	.short	65
	.short	1
	.short	0
	.align	2
	.long	L486 - . + 0x0
	.align	3
	.quad	L215
	.short	66
	.short	1
	.short	0
	.byte	0
	.align	3
	.quad	L203
	.short	65
	.short	1
	.short	0
	.align	2
	.long	L487 - . + 0x0
	.align	3
	.quad	L201
	.short	65
	.short	1
	.short	0
	.align	2
	.long	L488 - . + 0x0
	.align	3
	.quad	L150
	.short	33
	.short	1
	.short	8
	.align	2
	.long	L489 - . + 0x0
	.align	3
	.quad	L145
	.short	33
	.short	1
	.short	0
	.align	2
	.long	L490 - . + 0x0
	.align	3
	.quad	L120
	.short	34
	.short	2
	.short	1
	.short	8
	.byte	1
	.byte	1
	.align	3
	.quad	L118
	.short	33
	.short	1
	.short	8
	.align	2
	.long	L491 - . + 0x0
	.align	3
	.quad	L114
	.short	33
	.short	0
	.align	2
	.long	L492 - . + 0x0
	.align	3
	.align	2
L469:
	.long	L494 - . + 0x0
	.long	0x1583928
	.align	2
L481:
	.long	L496 - . + 0x1
	.long	0x25844b8
	.long	L497 - . + 0x0
	.long	0xa04530
	.align	2
L470:
	.long	L498 - . + 0x1
	.long	0x1201a28
	.long	L494 - . + 0x0
	.long	0x14808b8
	.align	2
L482:
	.long	L497 - . + 0x0
	.long	0x987600
	.align	2
L480:
	.long	L494 - . + 0x0
	.long	0xc84120
	.align	2
L490:
	.long	L499 - . + 0x0
	.long	0x80010000
	.align	2
L475:
	.long	L500 - . + 0x1
	.long	0xaa801b0
	.long	L494 - . + 0x0
	.long	0x1400870
	.align	2
L468:
	.long	L502 - . + 0x1
	.long	0x30d811b0
	.long	L504 - . + 0x1
	.long	0xa009f8
	.long	L505 - . + 0x1
	.long	0xd85558
	.long	L506 - . + 0x1
	.long	0xf84518
	.long	L494 - . + 0xf0000000
	.long	0x1510928
	.align	2
L491:
	.long	L507 - . + 0x0
	.long	0x707518
	.align	2
L485:
	.long	L498 - . + 0x0
	.long	0x1201a28
	.align	2
L484:
	.long	L497 - . + 0x0
	.long	0x98d5e8
	.align	2
L467:
	.long	L494 - . + 0xf0000000
	.long	0x1510928
	.align	2
L489:
	.long	L508 - . + 0x0
	.long	0x80e1f8
	.align	2
L477:
	.long	L494 - . + 0x0
	.long	0xd00a40
	.align	2
L473:
	.long	L498 - . + 0x1
	.long	0x1189998
	.long	L494 - . + 0x0
	.long	0x14808b8
	.align	2
L488:
	.long	L498 - . + 0x0
	.long	0x118ad98
	.align	2
L478:
	.long	L502 - . + 0x1
	.long	0x30d811b0
	.long	L504 - . + 0x1
	.long	0xa009f8
	.long	L505 - . + 0x1
	.long	0xd85558
	.long	L506 - . + 0x1
	.long	0xf84518
	.long	L494 - . + 0x0
	.long	0xd00a40
	.align	2
L471:
	.long	L502 - . + 0x1
	.long	0x30d811b0
	.long	L509 - . + 0x1
	.long	0x34f819a0
	.long	L498 - . + 0x1
	.long	0x1201a28
	.long	L494 - . + 0x0
	.long	0x14808b8
	.align	2
L479:
	.long	L494 - . + 0x0
	.long	0xc82920
	.align	2
L472:
	.long	L509 - . + 0x1
	.long	0x34f040e8
	.long	L498 - . + 0x1
	.long	0x1201a28
	.long	L494 - . + 0x0
	.long	0x14808b8
	.align	2
L492:
	.long	L507 - . + 0x0
	.long	0x709558
	.align	2
L476:
	.long	L510 - . + 0x0
	.long	0xe05920
	.align	2
L474:
	.long	L498 - . + 0x1
	.long	0x118ad98
	.long	L494 - . + 0x0
	.long	0x14808b8
	.align	2
L486:
	.long	L502 - . + 0x1
	.long	0x2fe01220
	.long	L504 - . + 0x1
	.long	0xa009f8
	.long	L505 - . + 0x1
	.long	0xd85558
	.long	L506 - . + 0x1
	.long	0xf84518
	.long	L498 - . + 0x0
	.long	0x1201a28
	.align	2
L487:
	.long	L498 - . + 0x0
	.long	0x1189998
	.align	2
L483:
	.long	L497 - . + 0x0
	.long	0x98aa00
L495:
	.asciz	"stdlib.ml"
L503:
	.asciz	"printf.ml"
L501:
	.asciz	"camlinternalFormat.ml"
L493:
	.asciz	"/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/functional-co"
	.asciz	"mparison/20261010/sources/binarytrees-ocaml-5.ml"
	.align	2
L510:
	.long	L493 - . + 0x0
	.asciz	"Binarytrees-ocaml-5.long_lived_tree"
	.align	2
L494:
	.long	L493 - . + 0x0
	.asciz	"Binarytrees-ocaml-5"
	.align	2
L497:
	.long	L493 - . + 0x0
	.asciz	"Binarytrees-ocaml-5.max_depth"
	.align	2
L509:
	.long	L501 - . + 0x0
	.asciz	"CamlinternalFormat.make_int_padding_precision.(fun)"
	.align	2
L507:
	.long	L493 - . + 0x0
	.asciz	"Binarytrees-ocaml-5.make"
	.align	2
L506:
	.long	L503 - . + 0x0
	.asciz	"Stdlib__Printf.printf"
	.align	2
L505:
	.long	L503 - . + 0x0
	.asciz	"Stdlib__Printf.fprintf"
	.align	2
L500:
	.long	L495 - . + 0x0
	.asciz	"Stdlib"
	.align	2
L508:
	.long	L493 - . + 0x0
	.asciz	"Binarytrees-ocaml-5.check"
	.align	2
L498:
	.long	L493 - . + 0x0
	.asciz	"Binarytrees-ocaml-5.loop_depths"
	.align	2
L504:
	.long	L503 - . + 0x0
	.asciz	"Stdlib__Printf.kfprintf"
	.align	2
L496:
	.long	L495 - . + 0x0
	.asciz	"Stdlib.max"
	.align	2
L502:
	.long	L501 - . + 0x0
	.asciz	"CamlinternalFormat.make_printf"
	.align	2
L499:
	.long	L493 - . + 0x0
	.short	66
	.short	73
	.long	0x49
	.asciz	"Binarytrees-ocaml-5.check"
	.align	3
