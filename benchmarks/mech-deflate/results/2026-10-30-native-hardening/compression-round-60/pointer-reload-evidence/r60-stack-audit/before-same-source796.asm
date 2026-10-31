   1eccc:      	mov	w16, #0xa6d0            ; =42704
   1ecd0:      	movk	w16, #0x1, lsl #16
   1ecd4:      	ldr	x13, [sp, x16, sxtx]
   1ecd8:      	ldr	x0, [x13, #0x1ac0]
   1ecdc:      	ldr	x1, [x13, #0x278]
   1ece0:      	adds	x15, x0, x1
   1ece4:      	cset	x0, hs
   1ece8:      	mov	x1, #0x1                ; =1
   1ecec:      	mov	w16, #0x3b20            ; =15136
   1ecf0:      	movk	w16, #0x1, lsl #16
   1ecf4:      	str	x1, [sp, x16, sxtx]
   1ecf8:      	uxtb	w0, w0
   1ecfc:      	cmp	w0, #0x0
   1ed00:      	b.ne	0xede30 <_mech_canonical_program_entry_v1+0xede30>
   1ed04:      	mov	w16, #0xa6d0            ; =42704
   1ed08:      	movk	w16, #0x1, lsl #16
   1ed0c:      	ldr	x13, [sp, x16, sxtx]
   1ed10:      	str	x15, [x13, #0x1b00]
