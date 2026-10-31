   20260:      	mov	w16, #0xa8d8            ; =43224
   20264:      	movk	w16, #0x1, lsl #16
   20268:      	ldr	x0, [sp, x16, sxtx]
   2026c:      	ldr	x0, [x0, #0x1ac0]
   20270:      	mov	w16, #0xa8d8            ; =43224
   20274:      	movk	w16, #0x1, lsl #16
   20278:      	ldr	x1, [sp, x16, sxtx]
   2027c:      	ldr	x1, [x1, #0x278]
   20280:      	adds	x0, x0, x1
   20284:      	cset	x1, hs
   20288:      	mov	x2, #0x1                ; =1
   2028c:      	mov	w16, #0x3af0            ; =15088
   20290:      	movk	w16, #0x1, lsl #16
   20294:      	str	x2, [sp, x16, sxtx]
   20298:      	uxtb	w1, w1
   2029c:      	cmp	w1, #0x0
   202a0:      	b.ne	0xf2b38 <_mech_canonical_program_entry_v1+0xf2b38>
   202a4:      	mov	w16, #0xa8d8            ; =43224
   202a8:      	movk	w16, #0x1, lsl #16
   202ac:      	ldr	x1, [sp, x16, sxtx]
   202b0:      	str	x0, [x1, #0x1b00]
