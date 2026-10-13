00000001000fdd6c <mech_gpu::sequence::runtime::resident::scalar_row>:
1000fdd6c:     	sub	sp, sp, #0xe0
1000fdd70:     	stp	d9, d8, [sp, #0x70]
1000fdd74:     	stp	x28, x27, [sp, #0x80]
1000fdd78:     	stp	x26, x25, [sp, #0x90]
1000fdd7c:     	stp	x24, x23, [sp, #0xa0]
1000fdd80:     	stp	x22, x21, [sp, #0xb0]
1000fdd84:     	stp	x20, x19, [sp, #0xc0]
1000fdd88:     	stp	x29, x30, [sp, #0xd0]
1000fdd8c:     	add	x29, sp, #0xd0
1000fdd90:     	str	w7, [sp, #0x44]
1000fdd94:     	mov	x20, x4
1000fdd98:     	mov	x21, x3
1000fdd9c:     	mov	x25, x0
1000fdda0:     	ldr	x8, [x2]
1000fdda4:     	mov	x9, #-0x8000000000000000 ; =-9223372036854775808
1000fdda8:     	cmp	x8, x9
1000fddac:     	b.ne	0x1000fde54 <mech_gpu::sequence::runtime::resident::scalar_row+0xe8>
1000fddb0:     	cbz	x5, 0x1000fed88 <mech_gpu::sequence::runtime::resident::scalar_row+0x101c>
1000fddb4:     	mov	x22, x1
1000fddb8:     	lsl	x19, x5, #3
1000fddbc:     	ldr	w8, [sp, #0x44]
1000fddc0:     	cbz	w8, 0x1000fedb4 <mech_gpu::sequence::runtime::resident::scalar_row+0x1048>
1000fddc4:     	mov	x23, #0x0               ; =0
1000fddc8:     	movi.2d	v8, #0000000000000000
1000fddcc:     	mov	x24, #-0x8000000000000000 ; =-9223372036854775808
1000fddd0:     	tbz	w6, #0x0, 0x1000fde14 <mech_gpu::sequence::runtime::resident::scalar_row+0xa8>
1000fddd4:     	ldr	d9, [x20, x23, lsl #3]
1000fddd8:     	stp	x23, x21, [sp, #0x48]
1000fdddc:     	add	x0, sp, #0x58
1000fdde0:     	add	x2, sp, #0x48
1000fdde4:     	mov	x1, x22
1000fdde8:     	bl	0x1000ecf04 <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
1000fddec:     	ldr	x8, [sp, #0x58]
1000fddf0:     	cmp	x8, x24
1000fddf4:     	b.ne	0x1000fee3c <mech_gpu::sequence::runtime::resident::scalar_row+0x10d0>
1000fddf8:     	add	x23, x23, #0x1
1000fddfc:     	ldr	d0, [sp, #0x60]
1000fde00:     	fdiv	d0, d9, d0
1000fde04:     	fadd	d8, d0, d8
1000fde08:     	subs	x19, x19, #0x8
1000fde0c:     	b.ne	0x1000fddd4 <mech_gpu::sequence::runtime::resident::scalar_row+0x68>
1000fde10:     	b	0x1000fed8c <mech_gpu::sequence::runtime::resident::scalar_row+0x1020>
1000fde14:     	ldr	d9, [x20, x23, lsl #3]
1000fde18:     	stp	x21, x23, [sp, #0x48]
1000fde1c:     	add	x0, sp, #0x58
1000fde20:     	add	x2, sp, #0x48
1000fde24:     	mov	x1, x22
1000fde28:     	bl	0x1000ecf04 <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
1000fde2c:     	ldr	x8, [sp, #0x58]
1000fde30:     	cmp	x8, x24
1000fde34:     	b.ne	0x1000fee3c <mech_gpu::sequence::runtime::resident::scalar_row+0x10d0>
1000fde38:     	add	x23, x23, #0x1
1000fde3c:     	ldr	d0, [sp, #0x60]
1000fde40:     	fdiv	d0, d9, d0
1000fde44:     	fadd	d8, d0, d8
1000fde48:     	subs	x19, x19, #0x8
1000fde4c:     	b.ne	0x1000fde14 <mech_gpu::sequence::runtime::resident::scalar_row+0xa8>
1000fde50:     	b	0x1000fed8c <mech_gpu::sequence::runtime::resident::scalar_row+0x1020>
1000fde54:     	str	x25, [sp, #0x8]
1000fde58:     	cbz	x5, 0x1000fed9c <mech_gpu::sequence::runtime::resident::scalar_row+0x1030>
1000fde5c:     	mov	x11, #0x0               ; =0
1000fde60:     	mov	x12, #0x0               ; =0
1000fde64:     	ldp	x9, x10, [x2, #0x20]
1000fde68:     	mov	w8, #0x18               ; =24
1000fde6c:     	str	x9, [sp, #0x18]
1000fde70:     	madd	x15, x10, x8, x9
1000fde74:     	ldp	x16, x1, [x2, #0x8]
1000fde78:     	ucvtf	d0, x21
1000fde7c:     	ldp	x8, x9, [x2, #0x38]
1000fde80:     	str	x8, [sp, #0x10]
1000fde84:     	stp	x9, x10, [sp, #0x30]
1000fde88:     	add	x4, x8, x9, lsl #5
1000fde8c:     	ldp	x8, x6, [x2, #0x48]
1000fde90:     	add	x13, x16, x8, lsl #8
1000fde94:     	fmov	x22, d0
1000fde98:     	adrp	x26, 0x101882000 <_anon.1ee260a9b71e4d71ca50cf18b810cf6c.441+0x4>
1000fde9c:     	add	x26, x26, #0x9d4
1000fdea0:     	movi.2d	v0, #0000000000000000
1000fdea4:     	mov	w28, #0x8               ; =8
1000fdea8:     	stp	x13, x8, [sp, #0x20]
1000fdeac:     	b	0x1000fdec4 <mech_gpu::sequence::runtime::resident::scalar_row+0x158>
1000fdeb0:     	add	x20, x20, x30, lsl #3
1000fdeb4:     	add	x12, x12, #0x1
1000fdeb8:     	add	x11, x11, #0x20
1000fdebc:     	sub	x5, x5, x30
1000fdec0:     	cbz	x5, 0x1000feda0 <mech_gpu::sequence::runtime::resident::scalar_row+0x1034>
1000fdec4:     	cmp	x5, #0x20
1000fdec8:     	mov	w8, #0x20               ; =32
1000fdecc:     	csel	x30, x5, x8, lo
1000fded0:     	ldr	x8, [sp, #0x38]
1000fded4:     	cbz	x8, 0x1000fe0a8 <mech_gpu::sequence::runtime::resident::scalar_row+0x33c>
1000fded8:     	lsl	x8, x12, #5
1000fdedc:     	lsl	x9, x30, #3
1000fdee0:     	sub	x10, x9, #0x8
1000fdee4:     	lsr	x13, x10, #3
1000fdee8:     	add	x13, x13, #0x1
1000fdeec:     	and	x14, x13, #0x3ffffffffffffff8
1000fdef0:     	dup.2d	v1, x8
1000fdef4:     	lsl	x17, x14, #3
1000fdef8:     	mov	w8, #0x2                ; =2
1000fdefc:     	dup.2d	v2, x8
1000fdf00:     	add.2d	v2, v1, v2
1000fdf04:     	mov	w8, #0x4                ; =4
1000fdf08:     	dup.2d	v3, x8
1000fdf0c:     	add.2d	v3, v1, v3
1000fdf10:     	mov	w8, #0x6                ; =6
1000fdf14:     	dup.2d	v4, x8
1000fdf18:     	add.2d	v4, v1, v4
1000fdf1c:     	ldr	x0, [sp, #0x18]
1000fdf20:     	mov	x3, x0
1000fdf24:     	b	0x1000fdf34 <mech_gpu::sequence::runtime::resident::scalar_row+0x1c8>
1000fdf28:     	mov	x0, x3
1000fdf2c:     	cmp	x3, x15
1000fdf30:     	b.eq	0x1000fe0a8 <mech_gpu::sequence::runtime::resident::scalar_row+0x33c>
1000fdf34:     	ldr	x8, [x3], #0x18
1000fdf38:     	cmp	x8, x1
1000fdf3c:     	b.hs	0x1000fee98 <mech_gpu::sequence::runtime::resident::scalar_row+0x112c>
1000fdf40:     	ldr	x2, [x0, #0x10]
1000fdf44:     	ldrb	w0, [x0, #0x8]
1000fdf48:     	add	x19, x16, x8, lsl #8
1000fdf4c:     	add	x25, x19, x9
1000fdf50:     	cmp	x2, x6
1000fdf54:     	b.ne	0x1000fdf70 <mech_gpu::sequence::runtime::resident::scalar_row+0x204>
1000fdf58:     	cmp	w0, #0x1
1000fdf5c:     	b.ne	0x1000fdfbc <mech_gpu::sequence::runtime::resident::scalar_row+0x250>
1000fdf60:     	cmp	x10, #0x38
1000fdf64:     	b.hs	0x1000fdfcc <mech_gpu::sequence::runtime::resident::scalar_row+0x260>
1000fdf68:     	mov	x8, #0x0                ; =0
1000fdf6c:     	b	0x1000fe018 <mech_gpu::sequence::runtime::resident::scalar_row+0x2ac>
1000fdf70:     	cmp	w0, #0x1
1000fdf74:     	csel	x7, x21, x22, eq
1000fdf78:     	cmp	x10, #0x38
1000fdf7c:     	b.lo	0x1000fdfac <mech_gpu::sequence::runtime::resident::scalar_row+0x240>
1000fdf80:     	add	x19, x19, x17
1000fdf84:     	dup.2d	v5, x7
1000fdf88:     	add	x0, x16, #0x20
1000fdf8c:     	add	x8, x0, x8, lsl #8
1000fdf90:     	and	x0, x13, #0x3ffffffffffffff8
1000fdf94:     	stp	q5, q5, [x8, #-0x20]
1000fdf98:     	stp	q5, q5, [x8], #0x40
1000fdf9c:     	subs	x0, x0, #0x8
1000fdfa0:     	b.ne	0x1000fdf94 <mech_gpu::sequence::runtime::resident::scalar_row+0x228>
1000fdfa4:     	cmp	x13, x14
1000fdfa8:     	b.eq	0x1000fdf28 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000fdfac:     	str	x7, [x19], #0x8
1000fdfb0:     	cmp	x19, x25
1000fdfb4:     	b.ne	0x1000fdfac <mech_gpu::sequence::runtime::resident::scalar_row+0x240>
1000fdfb8:     	b	0x1000fdf28 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000fdfbc:     	cmp	x10, #0x38
1000fdfc0:     	b.hs	0x1000fe030 <mech_gpu::sequence::runtime::resident::scalar_row+0x2c4>
1000fdfc4:     	mov	x8, #0x0                ; =0
1000fdfc8:     	b	0x1000fe08c <mech_gpu::sequence::runtime::resident::scalar_row+0x320>
1000fdfcc:     	add	x19, x19, x17
1000fdfd0:     	add	x0, x16, #0x20
1000fdfd4:     	add	x8, x0, x8, lsl #8
1000fdfd8:     	adrp	x0, 0x10187e000 <_anon.15555e135ea0107e25419003c1daf755.962+0x1861>
1000fdfdc:     	ldr	q5, [x0, #0xa80]
1000fdfe0:     	and	x0, x13, #0x3ffffffffffffff8
1000fdfe4:     	add.2d	v6, v5, v1
1000fdfe8:     	add.2d	v7, v5, v2
1000fdfec:     	add.2d	v16, v5, v3
1000fdff0:     	add.2d	v17, v5, v4
1000fdff4:     	stp	q6, q7, [x8, #-0x20]
1000fdff8:     	stp	q16, q17, [x8], #0x40
1000fdffc:     	dup.2d	v6, x28
1000fe000:     	add.2d	v5, v5, v6
1000fe004:     	subs	x0, x0, #0x8
1000fe008:     	b.ne	0x1000fdfe4 <mech_gpu::sequence::runtime::resident::scalar_row+0x278>
1000fe00c:     	and	x8, x13, #0x3ffffffffffffff8
1000fe010:     	cmp	x13, x14
1000fe014:     	b.eq	0x1000fdf28 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000fe018:     	add	x8, x11, x8
1000fe01c:     	str	x8, [x19], #0x8
1000fe020:     	add	x8, x8, #0x1
1000fe024:     	cmp	x19, x25
1000fe028:     	b.ne	0x1000fe01c <mech_gpu::sequence::runtime::resident::scalar_row+0x2b0>
1000fe02c:     	b	0x1000fdf28 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000fe030:     	add	x19, x19, x17
1000fe034:     	add	x0, x16, #0x20
1000fe038:     	add	x8, x0, x8, lsl #8
1000fe03c:     	adrp	x0, 0x10187e000 <_anon.15555e135ea0107e25419003c1daf755.962+0x1861>
1000fe040:     	ldr	q5, [x0, #0xa80]
1000fe044:     	and	x7, x13, #0x3ffffffffffffff8
1000fe048:     	add.2d	v6, v5, v1
1000fe04c:     	add.2d	v7, v5, v2
1000fe050:     	add.2d	v16, v5, v3
1000fe054:     	add.2d	v17, v5, v4
1000fe058:     	ucvtf.2d	v6, v6
1000fe05c:     	ucvtf.2d	v7, v7
1000fe060:     	ucvtf.2d	v16, v16
1000fe064:     	ucvtf.2d	v17, v17
1000fe068:     	stp	q6, q7, [x8, #-0x20]
1000fe06c:     	stp	q16, q17, [x8], #0x40
1000fe070:     	dup.2d	v6, x28
1000fe074:     	add.2d	v5, v5, v6
1000fe078:     	subs	x7, x7, #0x8
1000fe07c:     	b.ne	0x1000fe048 <mech_gpu::sequence::runtime::resident::scalar_row+0x2dc>
1000fe080:     	and	x8, x13, #0x3ffffffffffffff8
1000fe084:     	cmp	x13, x14
1000fe088:     	b.eq	0x1000fdf28 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000fe08c:     	add	x8, x11, x8
1000fe090:     	ucvtf	d5, x8
1000fe094:     	str	d5, [x19], #0x8
1000fe098:     	add	x8, x8, #0x1
1000fe09c:     	cmp	x19, x25
1000fe0a0:     	b.ne	0x1000fe090 <mech_gpu::sequence::runtime::resident::scalar_row+0x324>
1000fe0a4:     	b	0x1000fdf28 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000fe0a8:     	ldr	x8, [sp, #0x30]
1000fe0ac:     	cbz	x8, 0x1000feb54 <mech_gpu::sequence::runtime::resident::scalar_row+0xde8>
1000fe0b0:     	and	x19, x30, #0x38
1000fe0b4:     	and	x8, x30, #0xfffffffffffffffe
1000fe0b8:     	neg	x25, x8
1000fe0bc:     	ldr	x17, [sp, #0x10]
1000fe0c0:     	b	0x1000fe0e0 <mech_gpu::sequence::runtime::resident::scalar_row+0x374>
1000fe0c4:     	mov	x8, #0x0                ; =0
1000fe0c8:     	ldr	x9, [x14, x8, lsl #3]
1000fe0cc:     	ldr	x10, [x3, x8, lsl #3]
1000fe0d0:     	mul	x9, x10, x9
1000fe0d4:     	str	x9, [x13, x8, lsl #3]
1000fe0d8:     	cmp	x17, x4
1000fe0dc:     	b.eq	0x1000feb54 <mech_gpu::sequence::runtime::resident::scalar_row+0xde8>
1000fe0e0:     	ldr	x8, [x17, #0x18]
1000fe0e4:     	cmp	x8, x1
1000fe0e8:     	b.hi	0x1000fee74 <mech_gpu::sequence::runtime::resident::scalar_row+0x1108>
1000fe0ec:     	cmp	x1, x8
1000fe0f0:     	b.eq	0x1000feed0 <mech_gpu::sequence::runtime::resident::scalar_row+0x1164>
1000fe0f4:     	mov	x13, x17
1000fe0f8:     	ldr	x9, [x17, #0x8]
1000fe0fc:     	subs	x24, x8, x9
1000fe100:     	b.ls	0x1000feebc <mech_gpu::sequence::runtime::resident::scalar_row+0x1150>
1000fe104:     	ldr	x10, [x13, #0x10]
1000fe108:     	subs	x7, x8, x10
1000fe10c:     	b.ls	0x1000feea8 <mech_gpu::sequence::runtime::resident::scalar_row+0x113c>
1000fe110:     	add	x17, x13, #0x20
1000fe114:     	ldp	w2, w0, [x13]
1000fe118:     	add	x13, x16, x8, lsl #8
1000fe11c:     	add	x14, x16, x9, lsl #8
1000fe120:     	add	x3, x16, x10, lsl #8
1000fe124:     	adr	x27, 0x1000fe134 <mech_gpu::sequence::runtime::resident::scalar_row+0x3c8>
1000fe128:     	ldrh	w23, [x26, x2, lsl #1]
1000fe12c:     	add	x27, x27, x23, lsl #2
1000fe130:     	br	x27
1000fe134:     	cmp	x5, #0x8
1000fe138:     	b.lo	0x1000fe5c4 <mech_gpu::sequence::runtime::resident::scalar_row+0x858>
1000fe13c:     	tst	x24, #0xffffffffffffff
1000fe140:     	b.eq	0x1000fe5c4 <mech_gpu::sequence::runtime::resident::scalar_row+0x858>
1000fe144:     	ldp	q1, q2, [x14]
1000fe148:     	ldp	q3, q4, [x14, #0x20]
1000fe14c:     	ucvtf.2d	v1, v1
1000fe150:     	ucvtf.2d	v2, v2
1000fe154:     	ucvtf.2d	v3, v3
1000fe158:     	ucvtf.2d	v4, v4
1000fe15c:     	stp	q1, q2, [x13]
1000fe160:     	stp	q3, q4, [x13, #0x20]
1000fe164:     	cmp	x19, #0x8
1000fe168:     	b.eq	0x1000fe1dc <mech_gpu::sequence::runtime::resident::scalar_row+0x470>
1000fe16c:     	ldp	q1, q2, [x14, #0x40]
1000fe170:     	ldp	q3, q4, [x14, #0x60]
1000fe174:     	ucvtf.2d	v1, v1
1000fe178:     	ucvtf.2d	v2, v2
1000fe17c:     	ucvtf.2d	v3, v3
1000fe180:     	ucvtf.2d	v4, v4
1000fe184:     	stp	q1, q2, [x13, #0x40]
1000fe188:     	stp	q3, q4, [x13, #0x60]
1000fe18c:     	cmp	x19, #0x10
1000fe190:     	b.eq	0x1000fe1dc <mech_gpu::sequence::runtime::resident::scalar_row+0x470>
1000fe194:     	ldp	q1, q2, [x14, #0x80]
1000fe198:     	ldp	q3, q4, [x14, #0xa0]
1000fe19c:     	ucvtf.2d	v1, v1
1000fe1a0:     	ucvtf.2d	v2, v2
1000fe1a4:     	ucvtf.2d	v3, v3
1000fe1a8:     	ucvtf.2d	v4, v4
1000fe1ac:     	stp	q1, q2, [x13, #0x80]
1000fe1b0:     	stp	q3, q4, [x13, #0xa0]
1000fe1b4:     	cmp	x19, #0x18
1000fe1b8:     	b.eq	0x1000fe1dc <mech_gpu::sequence::runtime::resident::scalar_row+0x470>
1000fe1bc:     	ldp	q1, q2, [x14, #0xc0]
1000fe1c0:     	ldp	q3, q4, [x14, #0xe0]
1000fe1c4:     	ucvtf.2d	v1, v1
1000fe1c8:     	ucvtf.2d	v2, v2
1000fe1cc:     	ucvtf.2d	v3, v3
1000fe1d0:     	ucvtf.2d	v4, v4
1000fe1d4:     	stp	q1, q2, [x13, #0xc0]
1000fe1d8:     	stp	q3, q4, [x13, #0xe0]
1000fe1dc:     	and	x10, x30, #0x38
1000fe1e0:     	cmp	x30, x19
1000fe1e4:     	b.eq	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe1e8:     	b	0x1000fe5c8 <mech_gpu::sequence::runtime::resident::scalar_row+0x85c>
1000fe1ec:     	cmp	x5, #0x8
1000fe1f0:     	b.hs	0x1000fe62c <mech_gpu::sequence::runtime::resident::scalar_row+0x8c0>
1000fe1f4:     	mov	x0, #0x0                ; =0
1000fe1f8:     	lsl	x13, x0, #3
1000fe1fc:     	add	x10, x13, x10, lsl #8
1000fe200:     	add	x10, x16, x10
1000fe204:     	add	x9, x13, x9, lsl #8
1000fe208:     	add	x9, x16, x9
1000fe20c:     	add	x8, x13, x8, lsl #8
1000fe210:     	add	x8, x16, x8
1000fe214:     	sub	x13, x30, x0
1000fe218:     	ldr	x14, [x9], #0x8
1000fe21c:     	ldr	x0, [x10], #0x8
1000fe220:     	add	x14, x0, x14
1000fe224:     	str	x14, [x8], #0x8
1000fe228:     	subs	x13, x13, #0x1
1000fe22c:     	b.ne	0x1000fe218 <mech_gpu::sequence::runtime::resident::scalar_row+0x4ac>
1000fe230:     	b	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe234:     	cmp	x5, #0x8
1000fe238:     	b.hs	0x1000fe708 <mech_gpu::sequence::runtime::resident::scalar_row+0x99c>
1000fe23c:     	mov	x0, #0x0                ; =0
1000fe240:     	lsl	x13, x0, #3
1000fe244:     	add	x10, x13, x10, lsl #8
1000fe248:     	add	x10, x16, x10
1000fe24c:     	add	x9, x13, x9, lsl #8
1000fe250:     	add	x9, x16, x9
1000fe254:     	add	x8, x13, x8, lsl #8
1000fe258:     	add	x8, x16, x8
1000fe25c:     	sub	x13, x30, x0
1000fe260:     	ldr	d1, [x9], #0x8
1000fe264:     	ldr	d2, [x10], #0x8
1000fe268:     	fsub	d1, d1, d2
1000fe26c:     	str	d1, [x8], #0x8
1000fe270:     	subs	x13, x13, #0x1
1000fe274:     	b.ne	0x1000fe260 <mech_gpu::sequence::runtime::resident::scalar_row+0x4f4>
1000fe278:     	b	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe27c:     	mov	x8, x30
1000fe280:     	ldr	x9, [x3], #0x8
1000fe284:     	cbz	x9, 0x1000fee8c <mech_gpu::sequence::runtime::resident::scalar_row+0x1120>
1000fe288:     	ldr	x10, [x14], #0x8
1000fe28c:     	udiv	x9, x10, x9
1000fe290:     	str	x9, [x13], #0x8
1000fe294:     	subs	x8, x8, #0x1
1000fe298:     	b.ne	0x1000fe280 <mech_gpu::sequence::runtime::resident::scalar_row+0x514>
1000fe29c:     	b	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe2a0:     	and	x10, x0, #0x3f
1000fe2a4:     	cmp	x5, #0x8
1000fe2a8:     	b.lo	0x1000fe590 <mech_gpu::sequence::runtime::resident::scalar_row+0x824>
1000fe2ac:     	tst	x24, #0xffffffffffffff
1000fe2b0:     	b.eq	0x1000fe590 <mech_gpu::sequence::runtime::resident::scalar_row+0x824>
1000fe2b4:     	dup.2d	v1, x10
1000fe2b8:     	ldp	q2, q3, [x14]
1000fe2bc:     	ldp	q4, q5, [x14, #0x20]
1000fe2c0:     	neg.2d	v1, v1
1000fe2c4:     	ushl.2d	v2, v2, v1
1000fe2c8:     	ushl.2d	v3, v3, v1
1000fe2cc:     	ushl.2d	v4, v4, v1
1000fe2d0:     	ushl.2d	v5, v5, v1
1000fe2d4:     	stp	q2, q3, [x13]
1000fe2d8:     	stp	q4, q5, [x13, #0x20]
1000fe2dc:     	cmp	x19, #0x8
1000fe2e0:     	b.eq	0x1000fe354 <mech_gpu::sequence::runtime::resident::scalar_row+0x5e8>
1000fe2e4:     	ldp	q2, q3, [x14, #0x40]
1000fe2e8:     	ldp	q4, q5, [x14, #0x60]
1000fe2ec:     	ushl.2d	v2, v2, v1
1000fe2f0:     	ushl.2d	v3, v3, v1
1000fe2f4:     	ushl.2d	v4, v4, v1
1000fe2f8:     	ushl.2d	v5, v5, v1
1000fe2fc:     	stp	q2, q3, [x13, #0x40]
1000fe300:     	stp	q4, q5, [x13, #0x60]
1000fe304:     	cmp	x19, #0x10
1000fe308:     	b.eq	0x1000fe354 <mech_gpu::sequence::runtime::resident::scalar_row+0x5e8>
1000fe30c:     	ldp	q2, q3, [x14, #0x80]
1000fe310:     	ldp	q4, q5, [x14, #0xa0]
1000fe314:     	ushl.2d	v2, v2, v1
1000fe318:     	ushl.2d	v3, v3, v1
1000fe31c:     	ushl.2d	v4, v4, v1
1000fe320:     	ushl.2d	v5, v5, v1
1000fe324:     	stp	q2, q3, [x13, #0x80]
1000fe328:     	stp	q4, q5, [x13, #0xa0]
1000fe32c:     	cmp	x19, #0x18
1000fe330:     	b.eq	0x1000fe354 <mech_gpu::sequence::runtime::resident::scalar_row+0x5e8>
1000fe334:     	ldp	q2, q3, [x14, #0xc0]
1000fe338:     	ldp	q4, q5, [x14, #0xe0]
1000fe33c:     	ushl.2d	v2, v2, v1
1000fe340:     	ushl.2d	v3, v3, v1
1000fe344:     	ushl.2d	v4, v4, v1
1000fe348:     	ushl.2d	v1, v5, v1
1000fe34c:     	stp	q2, q3, [x13, #0xc0]
1000fe350:     	stp	q4, q1, [x13, #0xe0]
1000fe354:     	and	x13, x30, #0x38
1000fe358:     	cmp	x30, x19
1000fe35c:     	b.eq	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe360:     	b	0x1000fe594 <mech_gpu::sequence::runtime::resident::scalar_row+0x828>
1000fe364:     	cmp	x5, #0x8
1000fe368:     	b.hs	0x1000fe7e4 <mech_gpu::sequence::runtime::resident::scalar_row+0xa78>
1000fe36c:     	mov	x0, #0x0                ; =0
1000fe370:     	lsl	x13, x0, #3
1000fe374:     	add	x10, x13, x10, lsl #8
1000fe378:     	add	x10, x16, x10
1000fe37c:     	add	x9, x13, x9, lsl #8
1000fe380:     	add	x9, x16, x9
1000fe384:     	add	x8, x13, x8, lsl #8
1000fe388:     	add	x8, x16, x8
1000fe38c:     	sub	x13, x30, x0
1000fe390:     	ldr	d1, [x9], #0x8
1000fe394:     	ldr	d2, [x10], #0x8
1000fe398:     	fdiv	d1, d1, d2
1000fe39c:     	str	d1, [x8], #0x8
1000fe3a0:     	subs	x13, x13, #0x1
1000fe3a4:     	b.ne	0x1000fe390 <mech_gpu::sequence::runtime::resident::scalar_row+0x624>
1000fe3a8:     	b	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe3ac:     	cmp	x5, #0x1
1000fe3b0:     	b.eq	0x1000fe0c4 <mech_gpu::sequence::runtime::resident::scalar_row+0x358>
1000fe3b4:     	mov	x7, #0x0                ; =0
1000fe3b8:     	add	x0, x16, #0x8
1000fe3bc:     	add	x8, x0, x8, lsl #8
1000fe3c0:     	add	x9, x0, x9, lsl #8
1000fe3c4:     	add	x10, x0, x10, lsl #8
1000fe3c8:     	ldur	x0, [x9, #-0x8]
1000fe3cc:     	ldur	x2, [x10, #-0x8]
1000fe3d0:     	mul	x0, x2, x0
1000fe3d4:     	stur	x0, [x8, #-0x8]
1000fe3d8:     	ldr	x0, [x9], #0x10
1000fe3dc:     	ldr	x2, [x10], #0x10
1000fe3e0:     	mul	x0, x2, x0
1000fe3e4:     	sub	x7, x7, #0x2
1000fe3e8:     	str	x0, [x8], #0x10
1000fe3ec:     	cmp	x25, x7
1000fe3f0:     	b.ne	0x1000fe3c8 <mech_gpu::sequence::runtime::resident::scalar_row+0x65c>
1000fe3f4:     	tbz	w30, #0x0, 0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe3f8:     	neg	x8, x7
1000fe3fc:     	b	0x1000fe0c8 <mech_gpu::sequence::runtime::resident::scalar_row+0x35c>
1000fe400:     	cmp	x5, #0x8
1000fe404:     	b.hs	0x1000fe8c0 <mech_gpu::sequence::runtime::resident::scalar_row+0xb54>
1000fe408:     	mov	x0, #0x0                ; =0
1000fe40c:     	lsl	x13, x0, #3
1000fe410:     	add	x10, x13, x10, lsl #8
1000fe414:     	add	x10, x16, x10
1000fe418:     	add	x9, x13, x9, lsl #8
1000fe41c:     	add	x9, x16, x9
1000fe420:     	add	x8, x13, x8, lsl #8
1000fe424:     	add	x8, x16, x8
1000fe428:     	sub	x13, x30, x0
1000fe42c:     	ldr	x14, [x9], #0x8
1000fe430:     	ldr	x0, [x10], #0x8
1000fe434:     	sub	x14, x14, x0
1000fe438:     	str	x14, [x8], #0x8
1000fe43c:     	subs	x13, x13, #0x1
1000fe440:     	b.ne	0x1000fe42c <mech_gpu::sequence::runtime::resident::scalar_row+0x6c0>
1000fe444:     	b	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe448:     	cmp	x5, #0x8
1000fe44c:     	b.hs	0x1000fe99c <mech_gpu::sequence::runtime::resident::scalar_row+0xc30>
1000fe450:     	mov	x0, #0x0                ; =0
1000fe454:     	lsl	x13, x0, #3
1000fe458:     	add	x10, x13, x10, lsl #8
1000fe45c:     	add	x10, x16, x10
1000fe460:     	add	x9, x13, x9, lsl #8
1000fe464:     	add	x9, x16, x9
1000fe468:     	add	x8, x13, x8, lsl #8
1000fe46c:     	add	x8, x16, x8
1000fe470:     	sub	x13, x30, x0
1000fe474:     	ldr	d1, [x9], #0x8
1000fe478:     	ldr	d2, [x10], #0x8
1000fe47c:     	fmul	d1, d1, d2
1000fe480:     	str	d1, [x8], #0x8
1000fe484:     	subs	x13, x13, #0x1
1000fe488:     	b.ne	0x1000fe474 <mech_gpu::sequence::runtime::resident::scalar_row+0x708>
1000fe48c:     	b	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe490:     	cmp	x5, #0x8
1000fe494:     	b.lo	0x1000fe5f8 <mech_gpu::sequence::runtime::resident::scalar_row+0x88c>
1000fe498:     	tst	x24, #0xffffffffffffff
1000fe49c:     	b.eq	0x1000fe5f8 <mech_gpu::sequence::runtime::resident::scalar_row+0x88c>
1000fe4a0:     	ldp	q1, q2, [x14]
1000fe4a4:     	ldp	q3, q4, [x14, #0x20]
1000fe4a8:     	fsqrt.2d	v1, v1
1000fe4ac:     	fsqrt.2d	v2, v2
1000fe4b0:     	fsqrt.2d	v3, v3
1000fe4b4:     	fsqrt.2d	v4, v4
1000fe4b8:     	stp	q1, q2, [x13]
1000fe4bc:     	stp	q3, q4, [x13, #0x20]
1000fe4c0:     	cmp	x19, #0x8
1000fe4c4:     	b.eq	0x1000fe538 <mech_gpu::sequence::runtime::resident::scalar_row+0x7cc>
1000fe4c8:     	ldp	q1, q2, [x14, #0x40]
1000fe4cc:     	ldp	q3, q4, [x14, #0x60]
1000fe4d0:     	fsqrt.2d	v1, v1
1000fe4d4:     	fsqrt.2d	v2, v2
1000fe4d8:     	fsqrt.2d	v3, v3
1000fe4dc:     	fsqrt.2d	v4, v4
1000fe4e0:     	stp	q1, q2, [x13, #0x40]
1000fe4e4:     	stp	q3, q4, [x13, #0x60]
1000fe4e8:     	cmp	x19, #0x10
1000fe4ec:     	b.eq	0x1000fe538 <mech_gpu::sequence::runtime::resident::scalar_row+0x7cc>
1000fe4f0:     	ldp	q1, q2, [x14, #0x80]
1000fe4f4:     	ldp	q3, q4, [x14, #0xa0]
1000fe4f8:     	fsqrt.2d	v1, v1
1000fe4fc:     	fsqrt.2d	v2, v2
1000fe500:     	fsqrt.2d	v3, v3
1000fe504:     	fsqrt.2d	v4, v4
1000fe508:     	stp	q1, q2, [x13, #0x80]
1000fe50c:     	stp	q3, q4, [x13, #0xa0]
1000fe510:     	cmp	x19, #0x18
1000fe514:     	b.eq	0x1000fe538 <mech_gpu::sequence::runtime::resident::scalar_row+0x7cc>
1000fe518:     	ldp	q1, q2, [x14, #0xc0]
1000fe51c:     	ldp	q3, q4, [x14, #0xe0]
1000fe520:     	fsqrt.2d	v1, v1
1000fe524:     	fsqrt.2d	v2, v2
1000fe528:     	fsqrt.2d	v3, v3
1000fe52c:     	fsqrt.2d	v4, v4
1000fe530:     	stp	q1, q2, [x13, #0xc0]
1000fe534:     	stp	q3, q4, [x13, #0xe0]
1000fe538:     	and	x10, x30, #0x38
1000fe53c:     	cmp	x30, x19
1000fe540:     	b.eq	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe544:     	b	0x1000fe5fc <mech_gpu::sequence::runtime::resident::scalar_row+0x890>
1000fe548:     	cmp	x5, #0x8
1000fe54c:     	b.hs	0x1000fea78 <mech_gpu::sequence::runtime::resident::scalar_row+0xd0c>
1000fe550:     	mov	x0, #0x0                ; =0
1000fe554:     	lsl	x13, x0, #3
1000fe558:     	add	x10, x13, x10, lsl #8
1000fe55c:     	add	x10, x16, x10
1000fe560:     	add	x9, x13, x9, lsl #8
1000fe564:     	add	x9, x16, x9
1000fe568:     	add	x8, x13, x8, lsl #8
1000fe56c:     	add	x8, x16, x8
1000fe570:     	sub	x13, x30, x0
1000fe574:     	ldr	d1, [x9], #0x8
1000fe578:     	ldr	d2, [x10], #0x8
1000fe57c:     	fadd	d1, d1, d2
1000fe580:     	str	d1, [x8], #0x8
1000fe584:     	subs	x13, x13, #0x1
1000fe588:     	b.ne	0x1000fe574 <mech_gpu::sequence::runtime::resident::scalar_row+0x808>
1000fe58c:     	b	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe590:     	mov	x13, #0x0               ; =0
1000fe594:     	lsl	x14, x13, #3
1000fe598:     	add	x9, x14, x9, lsl #8
1000fe59c:     	add	x9, x16, x9
1000fe5a0:     	add	x8, x14, x8, lsl #8
1000fe5a4:     	add	x8, x16, x8
1000fe5a8:     	sub	x13, x30, x13
1000fe5ac:     	ldr	x14, [x9], #0x8
1000fe5b0:     	lsr	x14, x14, x10
1000fe5b4:     	str	x14, [x8], #0x8
1000fe5b8:     	subs	x13, x13, #0x1
1000fe5bc:     	b.ne	0x1000fe5ac <mech_gpu::sequence::runtime::resident::scalar_row+0x840>
1000fe5c0:     	b	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe5c4:     	mov	x10, #0x0               ; =0
1000fe5c8:     	lsl	x13, x10, #3
1000fe5cc:     	add	x9, x13, x9, lsl #8
1000fe5d0:     	add	x9, x16, x9
1000fe5d4:     	add	x8, x13, x8, lsl #8
1000fe5d8:     	add	x8, x16, x8
1000fe5dc:     	sub	x10, x30, x10
1000fe5e0:     	ldr	d1, [x9], #0x8
1000fe5e4:     	ucvtf	d1, d1
1000fe5e8:     	str	d1, [x8], #0x8
1000fe5ec:     	subs	x10, x10, #0x1
1000fe5f0:     	b.ne	0x1000fe5e0 <mech_gpu::sequence::runtime::resident::scalar_row+0x874>
1000fe5f4:     	b	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe5f8:     	mov	x10, #0x0               ; =0
1000fe5fc:     	lsl	x13, x10, #3
1000fe600:     	add	x9, x13, x9, lsl #8
1000fe604:     	add	x9, x16, x9
1000fe608:     	add	x8, x13, x8, lsl #8
1000fe60c:     	add	x8, x16, x8
1000fe610:     	sub	x10, x30, x10
1000fe614:     	ldr	d1, [x9], #0x8
1000fe618:     	fsqrt	d1, d1
1000fe61c:     	str	d1, [x8], #0x8
1000fe620:     	subs	x10, x10, #0x1
1000fe624:     	b.ne	0x1000fe614 <mech_gpu::sequence::runtime::resident::scalar_row+0x8a8>
1000fe628:     	b	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe62c:     	mov	x0, #0x0                ; =0
1000fe630:     	and	x2, x24, #0xffffffffffffff
1000fe634:     	cbz	x2, 0x1000fe1f8 <mech_gpu::sequence::runtime::resident::scalar_row+0x48c>
1000fe638:     	and	x2, x7, #0xffffffffffffff
1000fe63c:     	cbz	x2, 0x1000fe1f8 <mech_gpu::sequence::runtime::resident::scalar_row+0x48c>
1000fe640:     	ldp	q1, q2, [x14]
1000fe644:     	ldp	q3, q4, [x14, #0x20]
1000fe648:     	ldp	q5, q6, [x3]
1000fe64c:     	ldp	q7, q16, [x3, #0x20]
1000fe650:     	add.2d	v1, v5, v1
1000fe654:     	add.2d	v2, v6, v2
1000fe658:     	add.2d	v3, v7, v3
1000fe65c:     	add.2d	v4, v16, v4
1000fe660:     	stp	q1, q2, [x13]
1000fe664:     	stp	q3, q4, [x13, #0x20]
1000fe668:     	cmp	x19, #0x8
1000fe66c:     	b.eq	0x1000fe6f8 <mech_gpu::sequence::runtime::resident::scalar_row+0x98c>
1000fe670:     	ldp	q1, q2, [x14, #0x40]
1000fe674:     	ldp	q3, q4, [x14, #0x60]
1000fe678:     	ldp	q5, q6, [x3, #0x40]
1000fe67c:     	ldp	q7, q16, [x3, #0x60]
1000fe680:     	add.2d	v1, v5, v1
1000fe684:     	add.2d	v2, v6, v2
1000fe688:     	add.2d	v3, v7, v3
1000fe68c:     	add.2d	v4, v16, v4
1000fe690:     	stp	q1, q2, [x13, #0x40]
1000fe694:     	stp	q3, q4, [x13, #0x60]
1000fe698:     	cmp	x19, #0x10
1000fe69c:     	b.eq	0x1000fe6f8 <mech_gpu::sequence::runtime::resident::scalar_row+0x98c>
1000fe6a0:     	ldp	q1, q2, [x14, #0x80]
1000fe6a4:     	ldp	q3, q4, [x14, #0xa0]
1000fe6a8:     	ldp	q5, q6, [x3, #0x80]
1000fe6ac:     	ldp	q7, q16, [x3, #0xa0]
1000fe6b0:     	add.2d	v1, v5, v1
1000fe6b4:     	add.2d	v2, v6, v2
1000fe6b8:     	add.2d	v3, v7, v3
1000fe6bc:     	add.2d	v4, v16, v4
1000fe6c0:     	stp	q1, q2, [x13, #0x80]
1000fe6c4:     	stp	q3, q4, [x13, #0xa0]
1000fe6c8:     	cmp	x19, #0x18
1000fe6cc:     	b.eq	0x1000fe6f8 <mech_gpu::sequence::runtime::resident::scalar_row+0x98c>
1000fe6d0:     	ldp	q1, q2, [x14, #0xc0]
1000fe6d4:     	ldp	q3, q4, [x14, #0xe0]
1000fe6d8:     	ldp	q5, q6, [x3, #0xc0]
1000fe6dc:     	ldp	q7, q16, [x3, #0xe0]
1000fe6e0:     	add.2d	v1, v5, v1
1000fe6e4:     	add.2d	v2, v6, v2
1000fe6e8:     	add.2d	v3, v7, v3
1000fe6ec:     	add.2d	v4, v16, v4
1000fe6f0:     	stp	q1, q2, [x13, #0xc0]
1000fe6f4:     	stp	q3, q4, [x13, #0xe0]
1000fe6f8:     	and	x0, x30, #0x38
1000fe6fc:     	cmp	x30, x19
1000fe700:     	b.eq	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe704:     	b	0x1000fe1f8 <mech_gpu::sequence::runtime::resident::scalar_row+0x48c>
1000fe708:     	mov	x0, #0x0                ; =0
1000fe70c:     	and	x2, x24, #0xffffffffffffff
1000fe710:     	cbz	x2, 0x1000fe240 <mech_gpu::sequence::runtime::resident::scalar_row+0x4d4>
1000fe714:     	and	x2, x7, #0xffffffffffffff
1000fe718:     	cbz	x2, 0x1000fe240 <mech_gpu::sequence::runtime::resident::scalar_row+0x4d4>
1000fe71c:     	ldp	q1, q2, [x14]
1000fe720:     	ldp	q3, q4, [x14, #0x20]
1000fe724:     	ldp	q5, q6, [x3]
1000fe728:     	ldp	q7, q16, [x3, #0x20]
1000fe72c:     	fsub.2d	v1, v1, v5
1000fe730:     	fsub.2d	v2, v2, v6
1000fe734:     	fsub.2d	v3, v3, v7
1000fe738:     	fsub.2d	v4, v4, v16
1000fe73c:     	stp	q1, q2, [x13]
1000fe740:     	stp	q3, q4, [x13, #0x20]
1000fe744:     	cmp	x19, #0x8
1000fe748:     	b.eq	0x1000fe7d4 <mech_gpu::sequence::runtime::resident::scalar_row+0xa68>
1000fe74c:     	ldp	q1, q2, [x14, #0x40]
1000fe750:     	ldp	q3, q4, [x14, #0x60]
1000fe754:     	ldp	q5, q6, [x3, #0x40]
1000fe758:     	ldp	q7, q16, [x3, #0x60]
1000fe75c:     	fsub.2d	v1, v1, v5
1000fe760:     	fsub.2d	v2, v2, v6
1000fe764:     	fsub.2d	v3, v3, v7
1000fe768:     	fsub.2d	v4, v4, v16
1000fe76c:     	stp	q1, q2, [x13, #0x40]
1000fe770:     	stp	q3, q4, [x13, #0x60]
1000fe774:     	cmp	x19, #0x10
1000fe778:     	b.eq	0x1000fe7d4 <mech_gpu::sequence::runtime::resident::scalar_row+0xa68>
1000fe77c:     	ldp	q1, q2, [x14, #0x80]
1000fe780:     	ldp	q3, q4, [x14, #0xa0]
1000fe784:     	ldp	q5, q6, [x3, #0x80]
1000fe788:     	ldp	q7, q16, [x3, #0xa0]
1000fe78c:     	fsub.2d	v1, v1, v5
1000fe790:     	fsub.2d	v2, v2, v6
1000fe794:     	fsub.2d	v3, v3, v7
1000fe798:     	fsub.2d	v4, v4, v16
1000fe79c:     	stp	q1, q2, [x13, #0x80]
1000fe7a0:     	stp	q3, q4, [x13, #0xa0]
1000fe7a4:     	cmp	x19, #0x18
1000fe7a8:     	b.eq	0x1000fe7d4 <mech_gpu::sequence::runtime::resident::scalar_row+0xa68>
1000fe7ac:     	ldp	q1, q2, [x14, #0xc0]
1000fe7b0:     	ldp	q3, q4, [x14, #0xe0]
1000fe7b4:     	ldp	q5, q6, [x3, #0xc0]
1000fe7b8:     	ldp	q7, q16, [x3, #0xe0]
1000fe7bc:     	fsub.2d	v1, v1, v5
1000fe7c0:     	fsub.2d	v2, v2, v6
1000fe7c4:     	fsub.2d	v3, v3, v7
1000fe7c8:     	fsub.2d	v4, v4, v16
1000fe7cc:     	stp	q1, q2, [x13, #0xc0]
1000fe7d0:     	stp	q3, q4, [x13, #0xe0]
1000fe7d4:     	and	x0, x30, #0x38
1000fe7d8:     	cmp	x30, x19
1000fe7dc:     	b.eq	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe7e0:     	b	0x1000fe240 <mech_gpu::sequence::runtime::resident::scalar_row+0x4d4>
1000fe7e4:     	mov	x0, #0x0                ; =0
1000fe7e8:     	and	x2, x24, #0xffffffffffffff
1000fe7ec:     	cbz	x2, 0x1000fe370 <mech_gpu::sequence::runtime::resident::scalar_row+0x604>
1000fe7f0:     	and	x2, x7, #0xffffffffffffff
1000fe7f4:     	cbz	x2, 0x1000fe370 <mech_gpu::sequence::runtime::resident::scalar_row+0x604>
1000fe7f8:     	ldp	q1, q2, [x14]
1000fe7fc:     	ldp	q3, q4, [x14, #0x20]
1000fe800:     	ldp	q5, q6, [x3]
1000fe804:     	ldp	q7, q16, [x3, #0x20]
1000fe808:     	fdiv.2d	v1, v1, v5
1000fe80c:     	fdiv.2d	v2, v2, v6
1000fe810:     	fdiv.2d	v3, v3, v7
1000fe814:     	fdiv.2d	v4, v4, v16
1000fe818:     	stp	q1, q2, [x13]
1000fe81c:     	stp	q3, q4, [x13, #0x20]
1000fe820:     	cmp	x19, #0x8
1000fe824:     	b.eq	0x1000fe8b0 <mech_gpu::sequence::runtime::resident::scalar_row+0xb44>
1000fe828:     	ldp	q1, q2, [x14, #0x40]
1000fe82c:     	ldp	q3, q4, [x14, #0x60]
1000fe830:     	ldp	q5, q6, [x3, #0x40]
1000fe834:     	ldp	q7, q16, [x3, #0x60]
1000fe838:     	fdiv.2d	v1, v1, v5
1000fe83c:     	fdiv.2d	v2, v2, v6
1000fe840:     	fdiv.2d	v3, v3, v7
1000fe844:     	fdiv.2d	v4, v4, v16
1000fe848:     	stp	q1, q2, [x13, #0x40]
1000fe84c:     	stp	q3, q4, [x13, #0x60]
1000fe850:     	cmp	x19, #0x10
1000fe854:     	b.eq	0x1000fe8b0 <mech_gpu::sequence::runtime::resident::scalar_row+0xb44>
1000fe858:     	ldp	q1, q2, [x14, #0x80]
1000fe85c:     	ldp	q3, q4, [x14, #0xa0]
1000fe860:     	ldp	q5, q6, [x3, #0x80]
1000fe864:     	ldp	q7, q16, [x3, #0xa0]
1000fe868:     	fdiv.2d	v1, v1, v5
1000fe86c:     	fdiv.2d	v2, v2, v6
1000fe870:     	fdiv.2d	v3, v3, v7
1000fe874:     	fdiv.2d	v4, v4, v16
1000fe878:     	stp	q1, q2, [x13, #0x80]
1000fe87c:     	stp	q3, q4, [x13, #0xa0]
1000fe880:     	cmp	x19, #0x18
1000fe884:     	b.eq	0x1000fe8b0 <mech_gpu::sequence::runtime::resident::scalar_row+0xb44>
1000fe888:     	ldp	q1, q2, [x14, #0xc0]
1000fe88c:     	ldp	q3, q4, [x14, #0xe0]
1000fe890:     	ldp	q5, q6, [x3, #0xc0]
1000fe894:     	ldp	q7, q16, [x3, #0xe0]
1000fe898:     	fdiv.2d	v1, v1, v5
1000fe89c:     	fdiv.2d	v2, v2, v6
1000fe8a0:     	fdiv.2d	v3, v3, v7
1000fe8a4:     	fdiv.2d	v4, v4, v16
1000fe8a8:     	stp	q1, q2, [x13, #0xc0]
1000fe8ac:     	stp	q3, q4, [x13, #0xe0]
1000fe8b0:     	and	x0, x30, #0x38
1000fe8b4:     	cmp	x30, x19
1000fe8b8:     	b.eq	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe8bc:     	b	0x1000fe370 <mech_gpu::sequence::runtime::resident::scalar_row+0x604>
1000fe8c0:     	mov	x0, #0x0                ; =0
1000fe8c4:     	and	x2, x24, #0xffffffffffffff
1000fe8c8:     	cbz	x2, 0x1000fe40c <mech_gpu::sequence::runtime::resident::scalar_row+0x6a0>
1000fe8cc:     	and	x2, x7, #0xffffffffffffff
1000fe8d0:     	cbz	x2, 0x1000fe40c <mech_gpu::sequence::runtime::resident::scalar_row+0x6a0>
1000fe8d4:     	ldp	q1, q2, [x14]
1000fe8d8:     	ldp	q3, q4, [x14, #0x20]
1000fe8dc:     	ldp	q5, q6, [x3]
1000fe8e0:     	ldp	q7, q16, [x3, #0x20]
1000fe8e4:     	sub.2d	v1, v1, v5
1000fe8e8:     	sub.2d	v2, v2, v6
1000fe8ec:     	sub.2d	v3, v3, v7
1000fe8f0:     	sub.2d	v4, v4, v16
1000fe8f4:     	stp	q1, q2, [x13]
1000fe8f8:     	stp	q3, q4, [x13, #0x20]
1000fe8fc:     	cmp	x19, #0x8
1000fe900:     	b.eq	0x1000fe98c <mech_gpu::sequence::runtime::resident::scalar_row+0xc20>
1000fe904:     	ldp	q1, q2, [x14, #0x40]
1000fe908:     	ldp	q3, q4, [x14, #0x60]
1000fe90c:     	ldp	q5, q6, [x3, #0x40]
1000fe910:     	ldp	q7, q16, [x3, #0x60]
1000fe914:     	sub.2d	v1, v1, v5
1000fe918:     	sub.2d	v2, v2, v6
1000fe91c:     	sub.2d	v3, v3, v7
1000fe920:     	sub.2d	v4, v4, v16
1000fe924:     	stp	q1, q2, [x13, #0x40]
1000fe928:     	stp	q3, q4, [x13, #0x60]
1000fe92c:     	cmp	x19, #0x10
1000fe930:     	b.eq	0x1000fe98c <mech_gpu::sequence::runtime::resident::scalar_row+0xc20>
1000fe934:     	ldp	q1, q2, [x14, #0x80]
1000fe938:     	ldp	q3, q4, [x14, #0xa0]
1000fe93c:     	ldp	q5, q6, [x3, #0x80]
1000fe940:     	ldp	q7, q16, [x3, #0xa0]
1000fe944:     	sub.2d	v1, v1, v5
1000fe948:     	sub.2d	v2, v2, v6
1000fe94c:     	sub.2d	v3, v3, v7
1000fe950:     	sub.2d	v4, v4, v16
1000fe954:     	stp	q1, q2, [x13, #0x80]
1000fe958:     	stp	q3, q4, [x13, #0xa0]
1000fe95c:     	cmp	x19, #0x18
1000fe960:     	b.eq	0x1000fe98c <mech_gpu::sequence::runtime::resident::scalar_row+0xc20>
1000fe964:     	ldp	q1, q2, [x14, #0xc0]
1000fe968:     	ldp	q3, q4, [x14, #0xe0]
1000fe96c:     	ldp	q5, q6, [x3, #0xc0]
1000fe970:     	ldp	q7, q16, [x3, #0xe0]
1000fe974:     	sub.2d	v1, v1, v5
1000fe978:     	sub.2d	v2, v2, v6
1000fe97c:     	sub.2d	v3, v3, v7
1000fe980:     	sub.2d	v4, v4, v16
1000fe984:     	stp	q1, q2, [x13, #0xc0]
1000fe988:     	stp	q3, q4, [x13, #0xe0]
1000fe98c:     	and	x0, x30, #0x38
1000fe990:     	cmp	x30, x19
1000fe994:     	b.eq	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fe998:     	b	0x1000fe40c <mech_gpu::sequence::runtime::resident::scalar_row+0x6a0>
1000fe99c:     	mov	x0, #0x0                ; =0
1000fe9a0:     	and	x2, x24, #0xffffffffffffff
1000fe9a4:     	cbz	x2, 0x1000fe454 <mech_gpu::sequence::runtime::resident::scalar_row+0x6e8>
1000fe9a8:     	and	x2, x7, #0xffffffffffffff
1000fe9ac:     	cbz	x2, 0x1000fe454 <mech_gpu::sequence::runtime::resident::scalar_row+0x6e8>
1000fe9b0:     	ldp	q1, q2, [x14]
1000fe9b4:     	ldp	q3, q4, [x14, #0x20]
1000fe9b8:     	ldp	q5, q6, [x3]
1000fe9bc:     	ldp	q7, q16, [x3, #0x20]
1000fe9c0:     	fmul.2d	v1, v1, v5
1000fe9c4:     	fmul.2d	v2, v2, v6
1000fe9c8:     	fmul.2d	v3, v3, v7
1000fe9cc:     	fmul.2d	v4, v4, v16
1000fe9d0:     	stp	q1, q2, [x13]
1000fe9d4:     	stp	q3, q4, [x13, #0x20]
1000fe9d8:     	cmp	x19, #0x8
1000fe9dc:     	b.eq	0x1000fea68 <mech_gpu::sequence::runtime::resident::scalar_row+0xcfc>
1000fe9e0:     	ldp	q1, q2, [x14, #0x40]
1000fe9e4:     	ldp	q3, q4, [x14, #0x60]
1000fe9e8:     	ldp	q5, q6, [x3, #0x40]
1000fe9ec:     	ldp	q7, q16, [x3, #0x60]
1000fe9f0:     	fmul.2d	v1, v1, v5
1000fe9f4:     	fmul.2d	v2, v2, v6
1000fe9f8:     	fmul.2d	v3, v3, v7
1000fe9fc:     	fmul.2d	v4, v4, v16
1000fea00:     	stp	q1, q2, [x13, #0x40]
1000fea04:     	stp	q3, q4, [x13, #0x60]
1000fea08:     	cmp	x19, #0x10
1000fea0c:     	b.eq	0x1000fea68 <mech_gpu::sequence::runtime::resident::scalar_row+0xcfc>
1000fea10:     	ldp	q1, q2, [x14, #0x80]
1000fea14:     	ldp	q3, q4, [x14, #0xa0]
1000fea18:     	ldp	q5, q6, [x3, #0x80]
1000fea1c:     	ldp	q7, q16, [x3, #0xa0]
1000fea20:     	fmul.2d	v1, v1, v5
1000fea24:     	fmul.2d	v2, v2, v6
1000fea28:     	fmul.2d	v3, v3, v7
1000fea2c:     	fmul.2d	v4, v4, v16
1000fea30:     	stp	q1, q2, [x13, #0x80]
1000fea34:     	stp	q3, q4, [x13, #0xa0]
1000fea38:     	cmp	x19, #0x18
1000fea3c:     	b.eq	0x1000fea68 <mech_gpu::sequence::runtime::resident::scalar_row+0xcfc>
1000fea40:     	ldp	q1, q2, [x14, #0xc0]
1000fea44:     	ldp	q3, q4, [x14, #0xe0]
1000fea48:     	ldp	q5, q6, [x3, #0xc0]
1000fea4c:     	ldp	q7, q16, [x3, #0xe0]
1000fea50:     	fmul.2d	v1, v1, v5
1000fea54:     	fmul.2d	v2, v2, v6
1000fea58:     	fmul.2d	v3, v3, v7
1000fea5c:     	fmul.2d	v4, v4, v16
1000fea60:     	stp	q1, q2, [x13, #0xc0]
1000fea64:     	stp	q3, q4, [x13, #0xe0]
1000fea68:     	and	x0, x30, #0x38
1000fea6c:     	cmp	x30, x19
1000fea70:     	b.eq	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000fea74:     	b	0x1000fe454 <mech_gpu::sequence::runtime::resident::scalar_row+0x6e8>
1000fea78:     	mov	x0, #0x0                ; =0
1000fea7c:     	and	x2, x24, #0xffffffffffffff
1000fea80:     	cbz	x2, 0x1000fe554 <mech_gpu::sequence::runtime::resident::scalar_row+0x7e8>
1000fea84:     	and	x2, x7, #0xffffffffffffff
1000fea88:     	cbz	x2, 0x1000fe554 <mech_gpu::sequence::runtime::resident::scalar_row+0x7e8>
1000fea8c:     	ldp	q1, q2, [x14]
1000fea90:     	ldp	q3, q4, [x14, #0x20]
1000fea94:     	ldp	q5, q6, [x3]
1000fea98:     	ldp	q7, q16, [x3, #0x20]
1000fea9c:     	fadd.2d	v1, v1, v5
1000feaa0:     	fadd.2d	v2, v2, v6
1000feaa4:     	fadd.2d	v3, v3, v7
1000feaa8:     	fadd.2d	v4, v4, v16
1000feaac:     	stp	q1, q2, [x13]
1000feab0:     	stp	q3, q4, [x13, #0x20]
1000feab4:     	cmp	x19, #0x8
1000feab8:     	b.eq	0x1000feb44 <mech_gpu::sequence::runtime::resident::scalar_row+0xdd8>
1000feabc:     	ldp	q1, q2, [x14, #0x40]
1000feac0:     	ldp	q3, q4, [x14, #0x60]
1000feac4:     	ldp	q5, q6, [x3, #0x40]
1000feac8:     	ldp	q7, q16, [x3, #0x60]
1000feacc:     	fadd.2d	v1, v1, v5
1000fead0:     	fadd.2d	v2, v2, v6
1000fead4:     	fadd.2d	v3, v3, v7
1000fead8:     	fadd.2d	v4, v4, v16
1000feadc:     	stp	q1, q2, [x13, #0x40]
1000feae0:     	stp	q3, q4, [x13, #0x60]
1000feae4:     	cmp	x19, #0x10
1000feae8:     	b.eq	0x1000feb44 <mech_gpu::sequence::runtime::resident::scalar_row+0xdd8>
1000feaec:     	ldp	q1, q2, [x14, #0x80]
1000feaf0:     	ldp	q3, q4, [x14, #0xa0]
1000feaf4:     	ldp	q5, q6, [x3, #0x80]
1000feaf8:     	ldp	q7, q16, [x3, #0xa0]
1000feafc:     	fadd.2d	v1, v1, v5
1000feb00:     	fadd.2d	v2, v2, v6
1000feb04:     	fadd.2d	v3, v3, v7
1000feb08:     	fadd.2d	v4, v4, v16
1000feb0c:     	stp	q1, q2, [x13, #0x80]
1000feb10:     	stp	q3, q4, [x13, #0xa0]
1000feb14:     	cmp	x19, #0x18
1000feb18:     	b.eq	0x1000feb44 <mech_gpu::sequence::runtime::resident::scalar_row+0xdd8>
1000feb1c:     	ldp	q1, q2, [x14, #0xc0]
1000feb20:     	ldp	q3, q4, [x14, #0xe0]
1000feb24:     	ldp	q5, q6, [x3, #0xc0]
1000feb28:     	ldp	q7, q16, [x3, #0xe0]
1000feb2c:     	fadd.2d	v1, v1, v5
1000feb30:     	fadd.2d	v2, v2, v6
1000feb34:     	fadd.2d	v3, v3, v7
1000feb38:     	fadd.2d	v4, v4, v16
1000feb3c:     	stp	q1, q2, [x13, #0xc0]
1000feb40:     	stp	q3, q4, [x13, #0xe0]
1000feb44:     	and	x0, x30, #0x38
1000feb48:     	cmp	x30, x19
1000feb4c:     	b.eq	0x1000fe0d8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000feb50:     	b	0x1000fe554 <mech_gpu::sequence::runtime::resident::scalar_row+0x7e8>
1000feb54:     	ldr	x8, [sp, #0x28]
1000feb58:     	cmp	x8, x1
1000feb5c:     	b.hs	0x1000feee4 <mech_gpu::sequence::runtime::resident::scalar_row+0x1178>
1000feb60:     	ldr	w8, [sp, #0x44]
1000feb64:     	ldr	x13, [sp, #0x20]
1000feb68:     	cmp	x5, #0x8
1000feb6c:     	tbz	w8, #0x0, 0x1000feb7c <mech_gpu::sequence::runtime::resident::scalar_row+0xe10>
1000feb70:     	b.hs	0x1000feb88 <mech_gpu::sequence::runtime::resident::scalar_row+0xe1c>
1000feb74:     	mov	x8, #0x0                ; =0
1000feb78:     	b	0x1000fec78 <mech_gpu::sequence::runtime::resident::scalar_row+0xf0c>
1000feb7c:     	b.hs	0x1000feca4 <mech_gpu::sequence::runtime::resident::scalar_row+0xf38>
1000feb80:     	mov	x8, #0x0                ; =0
1000feb84:     	b	0x1000fed60 <mech_gpu::sequence::runtime::resident::scalar_row+0xff4>
1000feb88:     	and	x8, x30, #0x38
1000feb8c:     	movi.2d	v1, #0000000000000000
1000feb90:     	mov.d	v1[0], v0[0]
1000feb94:     	ldp	q0, q2, [x20]
1000feb98:     	ldp	q3, q4, [x20, #0x20]
1000feb9c:     	ldp	q5, q6, [x13]
1000feba0:     	ldp	q7, q16, [x13, #0x20]
1000feba4:     	fdiv.2d	v5, v0, v5
1000feba8:     	fdiv.2d	v0, v2, v6
1000febac:     	fdiv.2d	v2, v3, v7
1000febb0:     	fdiv.2d	v3, v4, v16
1000febb4:     	fadd.2d	v1, v5, v1
1000febb8:     	cmp	x8, #0x8
1000febbc:     	b.eq	0x1000fec60 <mech_gpu::sequence::runtime::resident::scalar_row+0xef4>
1000febc0:     	ldp	q4, q5, [x20, #0x40]
1000febc4:     	ldp	q6, q7, [x20, #0x60]
1000febc8:     	ldp	q16, q17, [x13, #0x40]
1000febcc:     	ldp	q18, q19, [x13, #0x60]
1000febd0:     	fdiv.2d	v4, v4, v16
1000febd4:     	fdiv.2d	v5, v5, v17
1000febd8:     	fdiv.2d	v6, v6, v18
1000febdc:     	fdiv.2d	v7, v7, v19
1000febe0:     	fadd.2d	v1, v4, v1
1000febe4:     	fadd.2d	v0, v5, v0
1000febe8:     	fadd.2d	v2, v6, v2
1000febec:     	fadd.2d	v3, v7, v3
1000febf0:     	cmp	x8, #0x10
1000febf4:     	b.eq	0x1000fec60 <mech_gpu::sequence::runtime::resident::scalar_row+0xef4>
1000febf8:     	ldp	q4, q5, [x20, #0x80]
1000febfc:     	ldp	q6, q7, [x20, #0xa0]
1000fec00:     	ldp	q16, q17, [x13, #0x80]
1000fec04:     	ldp	q18, q19, [x13, #0xa0]
1000fec08:     	fdiv.2d	v4, v4, v16
1000fec0c:     	fdiv.2d	v5, v5, v17
1000fec10:     	fdiv.2d	v6, v6, v18
1000fec14:     	fdiv.2d	v7, v7, v19
1000fec18:     	fadd.2d	v1, v4, v1
1000fec1c:     	fadd.2d	v0, v5, v0
1000fec20:     	fadd.2d	v2, v6, v2
1000fec24:     	fadd.2d	v3, v7, v3
1000fec28:     	cmp	x8, #0x18
1000fec2c:     	b.eq	0x1000fec60 <mech_gpu::sequence::runtime::resident::scalar_row+0xef4>
1000fec30:     	ldp	q4, q5, [x20, #0xc0]
1000fec34:     	ldp	q6, q7, [x20, #0xe0]
1000fec38:     	ldp	q16, q17, [x13, #0xc0]
1000fec3c:     	ldp	q18, q19, [x13, #0xe0]
1000fec40:     	fdiv.2d	v4, v4, v16
1000fec44:     	fdiv.2d	v5, v5, v17
1000fec48:     	fdiv.2d	v6, v6, v18
1000fec4c:     	fdiv.2d	v7, v7, v19
1000fec50:     	fadd.2d	v1, v4, v1
1000fec54:     	fadd.2d	v0, v5, v0
1000fec58:     	fadd.2d	v2, v6, v2
1000fec5c:     	fadd.2d	v3, v7, v3
1000fec60:     	fadd.2d	v0, v2, v0
1000fec64:     	fadd.2d	v0, v3, v0
1000fec68:     	fadd.2d	v0, v0, v1
1000fec6c:     	faddp.2d	d0, v0
1000fec70:     	cmp	x30, x8
1000fec74:     	b.eq	0x1000fdeb0 <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
1000fec78:     	lsl	x10, x8, #3
1000fec7c:     	add	x9, x13, x10
1000fec80:     	add	x10, x20, x10
1000fec84:     	sub	x8, x30, x8
1000fec88:     	ldr	d1, [x10], #0x8
1000fec8c:     	ldr	d2, [x9], #0x8
1000fec90:     	fdiv	d1, d1, d2
1000fec94:     	fadd	d0, d1, d0
1000fec98:     	subs	x8, x8, #0x1
1000fec9c:     	b.ne	0x1000fec88 <mech_gpu::sequence::runtime::resident::scalar_row+0xf1c>
1000feca0:     	b	0x1000fdeb0 <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
1000feca4:     	and	x8, x30, #0x38
1000feca8:     	movi.2d	v1, #0000000000000000
1000fecac:     	mov.d	v1[0], v0[0]
1000fecb0:     	ldp	q4, q0, [x20]
1000fecb4:     	ldp	q2, q3, [x20, #0x20]
1000fecb8:     	ldp	q5, q6, [x13]
1000fecbc:     	ldp	q7, q16, [x13, #0x20]
1000fecc0:     	fmul.2d	v0, v0, v6
1000fecc4:     	fmul.2d	v2, v2, v7
1000fecc8:     	fmul.2d	v3, v3, v16
1000feccc:     	fmla.2d	v1, v4, v5
1000fecd0:     	cmp	x8, #0x8
1000fecd4:     	b.eq	0x1000fed48 <mech_gpu::sequence::runtime::resident::scalar_row+0xfdc>
1000fecd8:     	ldp	q4, q5, [x20, #0x40]
1000fecdc:     	ldp	q6, q7, [x20, #0x60]
1000fece0:     	ldp	q16, q17, [x13, #0x40]
1000fece4:     	ldp	q18, q19, [x13, #0x60]
1000fece8:     	fmla.2d	v1, v4, v16
1000fecec:     	fmla.2d	v0, v5, v17
1000fecf0:     	fmla.2d	v2, v6, v18
1000fecf4:     	fmla.2d	v3, v7, v19
1000fecf8:     	cmp	x8, #0x10
1000fecfc:     	b.eq	0x1000fed48 <mech_gpu::sequence::runtime::resident::scalar_row+0xfdc>
1000fed00:     	ldp	q4, q5, [x20, #0x80]
1000fed04:     	ldp	q6, q7, [x20, #0xa0]
1000fed08:     	ldp	q16, q17, [x13, #0x80]
1000fed0c:     	ldp	q18, q19, [x13, #0xa0]
1000fed10:     	fmla.2d	v1, v4, v16
1000fed14:     	fmla.2d	v0, v5, v17
1000fed18:     	fmla.2d	v2, v6, v18
1000fed1c:     	fmla.2d	v3, v7, v19
1000fed20:     	cmp	x8, #0x18
1000fed24:     	b.eq	0x1000fed48 <mech_gpu::sequence::runtime::resident::scalar_row+0xfdc>
1000fed28:     	ldp	q4, q5, [x20, #0xc0]
1000fed2c:     	ldp	q6, q7, [x20, #0xe0]
1000fed30:     	ldp	q16, q17, [x13, #0xc0]
1000fed34:     	ldp	q18, q19, [x13, #0xe0]
1000fed38:     	fmla.2d	v1, v4, v16
1000fed3c:     	fmla.2d	v0, v5, v17
1000fed40:     	fmla.2d	v2, v6, v18
1000fed44:     	fmla.2d	v3, v7, v19
1000fed48:     	fadd.2d	v0, v2, v0
1000fed4c:     	fadd.2d	v0, v3, v0
1000fed50:     	fadd.2d	v0, v0, v1
1000fed54:     	faddp.2d	d0, v0
1000fed58:     	cmp	x30, x8
1000fed5c:     	b.eq	0x1000fdeb0 <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
1000fed60:     	lsl	x10, x8, #3
1000fed64:     	add	x9, x13, x10
1000fed68:     	add	x10, x20, x10
1000fed6c:     	sub	x8, x30, x8
1000fed70:     	ldr	d1, [x10], #0x8
1000fed74:     	ldr	d2, [x9], #0x8
1000fed78:     	fmadd	d0, d1, d2, d0
1000fed7c:     	subs	x8, x8, #0x1
1000fed80:     	b.ne	0x1000fed70 <mech_gpu::sequence::runtime::resident::scalar_row+0x1004>
1000fed84:     	b	0x1000fdeb0 <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
1000fed88:     	movi.2d	v8, #0000000000000000
1000fed8c:     	str	d8, [x25, #0x8]
1000fed90:     	mov	x8, #-0x8000000000000000 ; =-9223372036854775808
1000fed94:     	str	x8, [x25]
1000fed98:     	b	0x1000fee50 <mech_gpu::sequence::runtime::resident::scalar_row+0x10e4>
1000fed9c:     	movi.2d	v0, #0000000000000000
1000feda0:     	ldr	x9, [sp, #0x8]
1000feda4:     	str	d0, [x9, #0x8]
1000feda8:     	mov	x8, #-0x8000000000000000 ; =-9223372036854775808
1000fedac:     	str	x8, [x9]
1000fedb0:     	b	0x1000fee50 <mech_gpu::sequence::runtime::resident::scalar_row+0x10e4>
1000fedb4:     	mov	x23, #0x0               ; =0
1000fedb8:     	movi.2d	v8, #0000000000000000
1000fedbc:     	mov	x24, #-0x8000000000000000 ; =-9223372036854775808
1000fedc0:     	tbz	w6, #0x0, 0x1000fee00 <mech_gpu::sequence::runtime::resident::scalar_row+0x1094>
1000fedc4:     	ldr	d9, [x20, x23, lsl #3]
1000fedc8:     	stp	x23, x21, [sp, #0x48]
1000fedcc:     	add	x0, sp, #0x58
1000fedd0:     	add	x2, sp, #0x48
1000fedd4:     	mov	x1, x22
1000fedd8:     	bl	0x1000ecf04 <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
1000feddc:     	ldr	x8, [sp, #0x58]
1000fede0:     	cmp	x8, x24
1000fede4:     	b.ne	0x1000fee3c <mech_gpu::sequence::runtime::resident::scalar_row+0x10d0>
1000fede8:     	add	x23, x23, #0x1
1000fedec:     	ldr	d0, [sp, #0x60]
1000fedf0:     	fmadd	d8, d9, d0, d8
1000fedf4:     	subs	x19, x19, #0x8
1000fedf8:     	b.ne	0x1000fedc4 <mech_gpu::sequence::runtime::resident::scalar_row+0x1058>
1000fedfc:     	b	0x1000fed8c <mech_gpu::sequence::runtime::resident::scalar_row+0x1020>
1000fee00:     	ldr	d9, [x20, x23, lsl #3]
1000fee04:     	stp	x21, x23, [sp, #0x48]
1000fee08:     	add	x0, sp, #0x58
1000fee0c:     	add	x2, sp, #0x48
1000fee10:     	mov	x1, x22
1000fee14:     	bl	0x1000ecf04 <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
1000fee18:     	ldr	x8, [sp, #0x58]
1000fee1c:     	cmp	x8, x24
1000fee20:     	b.ne	0x1000fee3c <mech_gpu::sequence::runtime::resident::scalar_row+0x10d0>
1000fee24:     	add	x23, x23, #0x1
1000fee28:     	ldr	d0, [sp, #0x60]
1000fee2c:     	fmadd	d8, d9, d0, d8
1000fee30:     	subs	x19, x19, #0x8
1000fee34:     	b.ne	0x1000fee00 <mech_gpu::sequence::runtime::resident::scalar_row+0x1094>
1000fee38:     	b	0x1000fed8c <mech_gpu::sequence::runtime::resident::scalar_row+0x1020>
1000fee3c:     	ldr	d0, [sp, #0x60]
1000fee40:     	ldr	x9, [sp, #0x68]
1000fee44:     	str	x8, [x25]
1000fee48:     	str	d0, [x25, #0x8]
1000fee4c:     	str	x9, [x25, #0x10]
1000fee50:     	ldp	x29, x30, [sp, #0xd0]
1000fee54:     	ldp	x20, x19, [sp, #0xc0]
1000fee58:     	ldp	x22, x21, [sp, #0xb0]
1000fee5c:     	ldp	x24, x23, [sp, #0xa0]
1000fee60:     	ldp	x26, x25, [sp, #0x90]
1000fee64:     	ldp	x28, x27, [sp, #0x80]
1000fee68:     	ldp	d9, d8, [sp, #0x70]
1000fee6c:     	add	sp, sp, #0xe0
1000fee70:     	ret
1000fee74:     	adrp	x0, 0x101882000 <_anon.1ee260a9b71e4d71ca50cf18b810cf6c.441+0x4>
1000fee78:     	add	x0, x0, #0xbb9
1000fee7c:     	adrp	x2, 0x101d68000 <_anon.bc61513da336d9e96267327fcd73f2b9.198+0x230>
1000fee80:     	add	x2, x2, #0xe0
1000fee84:     	mov	w1, #0x13               ; =19
1000fee88:     	bl	0x10172c6b8 <core::panicking::panic_fmt>
1000fee8c:     	adrp	x0, 0x101d68000 <_anon.bc61513da336d9e96267327fcd73f2b9.198+0x230>
1000fee90:     	add	x0, x0, #0x140
1000fee94:     	bl	0x10172c7b0 <core::panicking::panic_const::panic_const_div_by_zero>
1000fee98:     	adrp	x2, 0x101d68000 <_anon.bc61513da336d9e96267327fcd73f2b9.198+0x230>
1000fee9c:     	add	x2, x2, #0xc8
1000feea0:     	mov	x0, x8
1000feea4:     	bl	0x10172c548 <core::panicking::panic_bounds_check>
1000feea8:     	adrp	x2, 0x101d68000 <_anon.bc61513da336d9e96267327fcd73f2b9.198+0x230>
1000feeac:     	add	x2, x2, #0x128
1000feeb0:     	mov	x0, x10
1000feeb4:     	mov	x1, x8
1000feeb8:     	bl	0x10172c548 <core::panicking::panic_bounds_check>
1000feebc:     	adrp	x2, 0x101d68000 <_anon.bc61513da336d9e96267327fcd73f2b9.198+0x230>
1000feec0:     	add	x2, x2, #0x110
1000feec4:     	mov	x0, x9
1000feec8:     	mov	x1, x8
1000feecc:     	bl	0x10172c548 <core::panicking::panic_bounds_check>
1000feed0:     	adrp	x2, 0x101d68000 <_anon.bc61513da336d9e96267327fcd73f2b9.198+0x230>
1000feed4:     	add	x2, x2, #0xf8
1000feed8:     	mov	x0, #0x0                ; =0
1000feedc:     	mov	x1, #0x0                ; =0
1000feee0:     	bl	0x10172c548 <core::panicking::panic_bounds_check>
1000feee4:     	adrp	x2, 0x101d68000 <_anon.bc61513da336d9e96267327fcd73f2b9.198+0x230>
1000feee8:     	add	x2, x2, #0xb0
1000feeec:     	ldr	x0, [sp, #0x28]
1000feef0:     	bl	0x10172c548 <core::panicking::panic_bounds_check>

00000001000feef4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone>:
1000feef4:     	sub	sp, sp, #0x90
1000feef8:     	stp	x28, x27, [sp, #0x30]
1000feefc:     	stp	x26, x25, [sp, #0x40]
1000fef00:     	stp	x24, x23, [sp, #0x50]
1000fef04:     	stp	x22, x21, [sp, #0x60]
1000fef08:     	stp	x20, x19, [sp, #0x70]
1000fef0c:     	stp	x29, x30, [sp, #0x80]
1000fef10:     	add	x29, sp, #0x80
1000fef14:     	ldr	x23, [x1, #0x18]
1000fef18:     	lsl	x20, x23, #4
1000fef1c:     	lsr	x8, x23, #60
1000fef20:     	cmp	x8, #0x0
1000fef24:     	mov	x8, #0x7ffffffffffffff8 ; =9223372036854775800
1000fef28:     	ccmp	x20, x8, #0x2, eq
1000fef2c:     	b.ls	0x1000fef40 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x4c>
1000fef30:     	mov	x21, #0x0               ; =0
1000fef34:     	mov	x0, x21
1000fef38:     	mov	x1, x20
1000fef3c:     	bl	0x10172c2f8 <alloc::raw_vec::handle_error>
1000fef40:     	mov	x22, x0
1000fef44:     	cbz	x20, 0x1000ff014 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x120>
1000fef48:     	mov	x19, x1
1000fef4c:     	bl	0x1000011f8 <__rustc::__rust_no_alloc_shim_is_unstable_v2>
1000fef50:     	mov	w21, #0x8               ; =8
1000fef54:     	mov	x0, x20
1000fef58:     	mov	w1, #0x8                ; =8
1000fef5c:     	bl	0x1000011e8 <__rustc::__rust_alloc>
1000fef60:     	cbz	x0, 0x1000fef34 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x40>
1000fef64:     	mov	x26, x23
1000fef68:     	mov	x1, x19
1000fef6c:     	stp	x22, x23, [sp, #0x8]
1000fef70:     	stp	xzr, xzr, [sp, #0x20]
1000fef74:     	str	x0, [sp, #0x18]
1000fef78:     	cbz	x23, 0x1000ff02c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x138>
1000fef7c:     	ldr	x24, [x1]
1000fef80:     	ldr	x8, [x1, #0x10]
1000fef84:     	cmp	x8, x24
1000fef88:     	csel	x9, xzr, x24, lo
1000fef8c:     	sub	x25, x8, x9
1000fef90:     	sub	x8, x24, x25
1000fef94:     	subs	x22, x23, x8
1000fef98:     	b.hi	0x1000fefa4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0xb0>
1000fef9c:     	mov	x22, #0x0               ; =0
1000fefa0:     	add	x24, x25, x23
1000fefa4:     	ldr	x21, [x1, #0x8]
1000fefa8:     	sub	x23, x24, x25
1000fefac:     	add	x27, x22, x23
1000fefb0:     	cmp	x27, x26
1000fefb4:     	b.ls	0x1000ff04c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x158>
1000fefb8:     	add	x0, sp, #0x10
1000fefbc:     	add	x2, x22, x23
1000fefc0:     	mov	x1, #0x0                ; =0
1000fefc4:     	mov	w3, #0x8                ; =8
1000fefc8:     	mov	w4, #0x10               ; =16
1000fefcc:     	bl	0x1016f6e68 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
1000fefd0:     	ldp	x19, x20, [sp, #0x20]
1000fefd4:     	ldr	x28, [sp, #0x10]
1000fefd8:     	sub	x8, x26, x20
1000fefdc:     	cmp	x19, x8
1000fefe0:     	b.ls	0x1000ff058 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x164>
1000fefe4:     	sub	x8, x26, x19
1000fefe8:     	sub	x9, x20, x8
1000fefec:     	cmp	x8, x9
1000feff0:     	b.ls	0x1000ff2fc <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x408>
1000feff4:     	sub	x10, x28, x26
1000feff8:     	cmp	x10, x9
1000feffc:     	b.lo	0x1000ff2fc <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x408>
1000ff000:     	ldr	x1, [sp, #0x18]
1000ff004:     	add	x0, x1, x26, lsl #4
1000ff008:     	lsl	x2, x9, #4
1000ff00c:     	bl	0x10172cbac <_writev+0x10172cbac>
1000ff010:     	b	0x1000ff058 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x164>
1000ff014:     	mov	x26, #0x0               ; =0
1000ff018:     	mov	w0, #0x8                ; =8
1000ff01c:     	stp	x22, x26, [sp, #0x8]
1000ff020:     	stp	xzr, xzr, [sp, #0x20]
1000ff024:     	str	x0, [sp, #0x18]
1000ff028:     	cbnz	x23, 0x1000fef7c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x88>
1000ff02c:     	mov	x25, #0x0               ; =0
1000ff030:     	mov	x24, #0x0               ; =0
1000ff034:     	mov	x22, #0x0               ; =0
1000ff038:     	ldr	x21, [x1, #0x8]
1000ff03c:     	sub	x23, x24, x25
1000ff040:     	add	x27, x22, x23
1000ff044:     	cmp	x27, x26
1000ff048:     	b.hi	0x1000fefb8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0xc4>
1000ff04c:     	mov	x19, #0x0               ; =0
1000ff050:     	mov	x20, #0x0               ; =0
1000ff054:     	mov	x28, x26
1000ff058:     	lsl	x17, x25, #4
1000ff05c:     	add	x9, x21, x17
1000ff060:     	lsl	x11, x24, #4
1000ff064:     	add	x8, x19, x20
1000ff068:     	cmp	x8, x28
1000ff06c:     	csel	x10, xzr, x28, lo
1000ff070:     	sub	x13, x8, x10
1000ff074:     	sub	x14, x28, x13
1000ff078:     	cmp	x14, x27
1000ff07c:     	b.hs	0x1000ff0f4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x200>
1000ff080:     	lsl	x8, x22, #4
1000ff084:     	cmp	x28, x13
1000ff088:     	b.ne	0x1000ff104 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x210>
1000ff08c:     	mov	x12, #0x0               ; =0
1000ff090:     	add	x10, x21, x11
1000ff094:     	cmp	x9, x10
1000ff098:     	ldr	x17, [sp, #0x8]
1000ff09c:     	b.eq	0x1000ff178 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x284>
1000ff0a0:     	sub	x10, x10, x9
1000ff0a4:     	lsr	x11, x10, #4
1000ff0a8:     	add	x10, x12, x11
1000ff0ac:     	ldr	x12, [sp, #0x18]
1000ff0b0:     	mov	w13, #0x1               ; =1
1000ff0b4:     	mov	x14, x11
1000ff0b8:     	ldr	q0, [x9], #0x10
1000ff0bc:     	fmov	x15, d0
1000ff0c0:     	ldadd	x13, x15, [x15]
1000ff0c4:     	tbnz	x15, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff0c8:     	mov.d	x15, v0[1]
1000ff0cc:     	ldadd	x13, x15, [x15]
1000ff0d0:     	tbnz	x15, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff0d4:     	str	q0, [x12], #0x10
1000ff0d8:     	subs	x14, x14, #0x1
1000ff0dc:     	b.ne	0x1000ff0b8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x1c4>
1000ff0e0:     	mov	x9, x21
1000ff0e4:     	add	x8, x21, x8
1000ff0e8:     	cmp	x21, x8
1000ff0ec:     	b.ne	0x1000ff190 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x29c>
1000ff0f0:     	b	0x1000ff2c8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d4>
1000ff0f4:     	cmp	x17, x11
1000ff0f8:     	b.ne	0x1000ff1d4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x2e0>
1000ff0fc:     	mov	x23, #0x0               ; =0
1000ff100:     	b	0x1000ff20c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x318>
1000ff104:     	mov	x15, #0x0               ; =0
1000ff108:     	mov	x12, #0x0               ; =0
1000ff10c:     	sub	x10, x11, x17
1000ff110:     	lsr	x10, x10, #4
1000ff114:     	ldr	x16, [sp, #0x18]
1000ff118:     	lsl	x13, x13, #4
1000ff11c:     	add	x16, x16, x13
1000ff120:     	sub	x17, x17, x11
1000ff124:     	mov	w0, #0x1                ; =1
1000ff128:     	cmp	x17, x15
1000ff12c:     	b.eq	0x1000ff25c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x368>
1000ff130:     	ldr	q0, [x9, x12, lsl #4]
1000ff134:     	fmov	x1, d0
1000ff138:     	ldadd	x0, x1, [x1]
1000ff13c:     	tbnz	x1, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff140:     	mov.d	x1, v0[1]
1000ff144:     	ldadd	x0, x1, [x1]
1000ff148:     	tbnz	x1, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff14c:     	str	q0, [x16, x12, lsl #4]
1000ff150:     	add	x12, x12, #0x1
1000ff154:     	add	x13, x13, #0x10
1000ff158:     	sub	x15, x15, #0x10
1000ff15c:     	cmp	x14, x12
1000ff160:     	b.ne	0x1000ff128 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x234>
1000ff164:     	sub	x9, x9, x15
1000ff168:     	add	x10, x21, x11
1000ff16c:     	cmp	x9, x10
1000ff170:     	ldr	x17, [sp, #0x8]
1000ff174:     	b.ne	0x1000ff0a0 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x1ac>
1000ff178:     	mov	x11, #0x0               ; =0
1000ff17c:     	mov	x9, x21
1000ff180:     	mov	x10, x12
1000ff184:     	add	x8, x21, x8
1000ff188:     	cmp	x21, x8
1000ff18c:     	b.eq	0x1000ff2c8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d4>
1000ff190:     	sub	x8, x8, x9
1000ff194:     	lsr	x8, x8, #4
1000ff198:     	add	x23, x10, x8
1000ff19c:     	ldr	x10, [sp, #0x18]
1000ff1a0:     	add	x10, x10, x11, lsl #4
1000ff1a4:     	mov	w11, #0x1               ; =1
1000ff1a8:     	ldr	q0, [x9], #0x10
1000ff1ac:     	fmov	x12, d0
1000ff1b0:     	ldadd	x11, x12, [x12]
1000ff1b4:     	tbnz	x12, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff1b8:     	mov.d	x12, v0[1]
1000ff1bc:     	ldadd	x11, x12, [x12]
1000ff1c0:     	tbnz	x12, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff1c4:     	str	q0, [x10], #0x10
1000ff1c8:     	subs	x8, x8, #0x1
1000ff1cc:     	b.ne	0x1000ff1a8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x2b4>
1000ff1d0:     	b	0x1000ff2cc <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d8>
1000ff1d4:     	ldr	x8, [sp, #0x18]
1000ff1d8:     	add	x8, x8, x13, lsl #4
1000ff1dc:     	mov	w10, #0x1               ; =1
1000ff1e0:     	mov	x11, x23
1000ff1e4:     	ldr	q0, [x9], #0x10
1000ff1e8:     	fmov	x12, d0
1000ff1ec:     	ldadd	x10, x12, [x12]
1000ff1f0:     	tbnz	x12, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff1f4:     	mov.d	x12, v0[1]
1000ff1f8:     	ldadd	x10, x12, [x12]
1000ff1fc:     	tbnz	x12, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff200:     	str	q0, [x8], #0x10
1000ff204:     	subs	x11, x11, #0x1
1000ff208:     	b.ne	0x1000ff1e4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x2f0>
1000ff20c:     	cbz	x22, 0x1000ff254 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x360>
1000ff210:     	add	x8, x22, x23
1000ff214:     	ldr	x9, [sp, #0x18]
1000ff218:     	lsl	x10, x23, #4
1000ff21c:     	add	x10, x10, x13, lsl #4
1000ff220:     	add	x9, x9, x10
1000ff224:     	mov	w10, #0x1               ; =1
1000ff228:     	ldr	q0, [x21], #0x10
1000ff22c:     	fmov	x11, d0
1000ff230:     	ldadd	x10, x11, [x11]
1000ff234:     	tbnz	x11, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff238:     	mov.d	x11, v0[1]
1000ff23c:     	ldadd	x10, x11, [x11]
1000ff240:     	tbnz	x11, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff244:     	str	q0, [x9], #0x10
1000ff248:     	subs	x22, x22, #0x1
1000ff24c:     	b.ne	0x1000ff228 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x334>
1000ff250:     	mov	x23, x8
1000ff254:     	ldr	x17, [sp, #0x8]
1000ff258:     	b	0x1000ff2cc <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d8>
1000ff25c:     	mov	x11, #0x0               ; =0
1000ff260:     	add	x23, x22, x10
1000ff264:     	ldr	x9, [sp, #0x18]
1000ff268:     	add	x13, x9, x13
1000ff26c:     	sub	x12, x14, x12
1000ff270:     	mov	w14, #0x1               ; =1
1000ff274:     	mov	x15, x8
1000ff278:     	mov	x9, x21
1000ff27c:     	ldr	x17, [sp, #0x8]
1000ff280:     	cbz	x15, 0x1000ff2cc <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d8>
1000ff284:     	ldr	q0, [x9], #0x10
1000ff288:     	fmov	x16, d0
1000ff28c:     	ldadd	x14, x16, [x16]
1000ff290:     	tbnz	x16, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff294:     	mov.d	x16, v0[1]
1000ff298:     	ldadd	x14, x16, [x16]
1000ff29c:     	tbnz	x16, #0x3f, 0x1000ff320 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000ff2a0:     	str	q0, [x13, x11, lsl #4]
1000ff2a4:     	add	x10, x10, #0x1
1000ff2a8:     	sub	x15, x15, #0x10
1000ff2ac:     	add	x11, x11, #0x1
1000ff2b0:     	cmp	x12, x11
1000ff2b4:     	b.ne	0x1000ff280 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x38c>
1000ff2b8:     	mov	x11, #0x0               ; =0
1000ff2bc:     	add	x8, x21, x8
1000ff2c0:     	cmp	x9, x8
1000ff2c4:     	b.ne	0x1000ff190 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x29c>
1000ff2c8:     	mov	x23, x10
1000ff2cc:     	add	x8, x20, x23
1000ff2d0:     	str	x8, [sp, #0x28]
1000ff2d4:     	ldp	q0, q1, [sp, #0x10]
1000ff2d8:     	stp	q0, q1, [x17]
1000ff2dc:     	ldp	x29, x30, [sp, #0x80]
1000ff2e0:     	ldp	x20, x19, [sp, #0x70]
1000ff2e4:     	ldp	x22, x21, [sp, #0x60]
1000ff2e8:     	ldp	x24, x23, [sp, #0x50]
1000ff2ec:     	ldp	x26, x25, [sp, #0x40]
1000ff2f0:     	ldp	x28, x27, [sp, #0x30]
1000ff2f4:     	add	sp, sp, #0x90
1000ff2f8:     	ret
1000ff2fc:     	sub	x26, x28, x8
1000ff300:     	ldr	x9, [sp, #0x18]
1000ff304:     	add	x1, x9, x19, lsl #4
1000ff308:     	add	x0, x9, x26, lsl #4
1000ff30c:     	lsl	x2, x8, #4
1000ff310:     	bl	0x10172cbb8 <_writev+0x10172cbb8>
1000ff314:     	str	x26, [sp, #0x20]
1000ff318:     	mov	x19, x26
1000ff31c:     	b	0x1000ff058 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x164>
1000ff320:     	brk	#0x1
1000ff324:     	mov	x19, x0
1000ff328:     	add	x0, sp, #0x10
1000ff32c:     	bl	0x1000e4d30 <core::ptr::drop_in_place::<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record>>>
1000ff330:     	mov	x0, x19
1000ff334:     	bl	0x10172c8dc <_writev+0x10172c8dc>
1000ff338:     	bl	0x10172c530 <core::panicking::panic_in_cleanup>

00000001000ff33c <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop>:
1000ff33c:     	stp	x24, x23, [sp, #-0x40]!
1000ff340:     	stp	x22, x21, [sp, #0x10]
1000ff344:     	stp	x20, x19, [sp, #0x20]
1000ff348:     	stp	x29, x30, [sp, #0x30]
1000ff34c:     	add	x29, sp, #0x30
1000ff350:     	mov	x20, x0
1000ff354:     	ldr	x8, [x0, #0x20]
1000ff358:     	cbz	x8, 0x1000ff390 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x54>
1000ff35c:     	cmp	x8, #0x2
1000ff360:     	b.eq	0x1000ff3e0 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xa4>
1000ff364:     	cmp	x8, #0x3
1000ff368:     	b.ne	0x1000ff3f4 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xb8>
1000ff36c:     	ldp	x19, x8, [x20, #0x8]
1000ff370:     	add	x23, x8, #0x1
1000ff374:     	mov	x0, x19
1000ff378:     	subs	x23, x23, #0x1
1000ff37c:     	b.eq	0x1000ff3b4 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x78>
1000ff380:     	add	x22, x0, #0x38
1000ff384:     	bl	0x100023300 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
1000ff388:     	mov	x0, x22
1000ff38c:     	b	0x1000ff378 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x3c>
1000ff390:     	ldp	x19, x8, [x20, #0x8]
1000ff394:     	add	x23, x8, #0x1
1000ff398:     	mov	x0, x19
1000ff39c:     	subs	x23, x23, #0x1
1000ff3a0:     	b.eq	0x1000ff3b4 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x78>
1000ff3a4:     	add	x22, x0, #0x38
1000ff3a8:     	bl	0x100023300 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
1000ff3ac:     	mov	x0, x22
1000ff3b0:     	b	0x1000ff39c <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x60>
1000ff3b4:     	ldr	x8, [x20]
1000ff3b8:     	cbz	x8, 0x1000ff3e0 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xa4>
1000ff3bc:     	lsl	x9, x8, #6
1000ff3c0:     	sub	x1, x9, x8, lsl #3
1000ff3c4:     	mov	x0, x19
1000ff3c8:     	mov	w2, #0x8                ; =8
1000ff3cc:     	ldp	x29, x30, [sp, #0x30]
1000ff3d0:     	ldp	x20, x19, [sp, #0x20]
1000ff3d4:     	ldp	x22, x21, [sp, #0x10]
1000ff3d8:     	ldp	x24, x23, [sp], #0x40
1000ff3dc:     	b	0x1000011ec <__rustc::__rust_dealloc>
1000ff3e0:     	ldp	x29, x30, [sp, #0x30]
1000ff3e4:     	ldp	x20, x19, [sp, #0x20]
1000ff3e8:     	ldp	x22, x21, [sp, #0x10]
1000ff3ec:     	ldp	x24, x23, [sp], #0x40
1000ff3f0:     	ret
1000ff3f4:     	adrp	x0, 0x101882000 <_anon.1ee260a9b71e4d71ca50cf18b810cf6c.441+0x4>
1000ff3f8:     	add	x0, x0, #0xd1f
1000ff3fc:     	adrp	x2, 0x101d67000 <_anon.2fd9b896ffe5d61f3cc11fa0b88d2dda.80+0x58>
1000ff400:     	add	x2, x2, #0x910
1000ff404:     	mov	w1, #0x79               ; =121
1000ff408:     	bl	0x10172c6b8 <core::panicking::panic_fmt>
1000ff40c:     	mov	x21, x0
1000ff410:     	subs	x23, x23, #0x1
1000ff414:     	b.eq	0x1000ff450 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x114>
1000ff418:     	add	x24, x22, #0x38
1000ff41c:     	mov	x0, x22
1000ff420:     	bl	0x100023300 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
1000ff424:     	mov	x22, x24
1000ff428:     	b	0x1000ff410 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xd4>
1000ff42c:     	bl	0x10172c530 <core::panicking::panic_in_cleanup>
1000ff430:     	mov	x21, x0
1000ff434:     	subs	x23, x23, #0x1
1000ff438:     	b.eq	0x1000ff450 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x114>
1000ff43c:     	add	x24, x22, #0x38
1000ff440:     	mov	x0, x22
1000ff444:     	bl	0x100023300 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
1000ff448:     	mov	x22, x24
1000ff44c:     	b	0x1000ff434 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xf8>
1000ff450:     	ldr	x8, [x20]
1000ff454:     	cbz	x8, 0x1000ff46c <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x130>
1000ff458:     	lsl	x9, x8, #6
1000ff45c:     	sub	x1, x9, x8, lsl #3
1000ff460:     	mov	x0, x19
1000ff464:     	mov	w2, #0x8                ; =8
1000ff468:     	bl	0x1000011ec <__rustc::__rust_dealloc>
1000ff46c:     	mov	x0, x21
1000ff470:     	bl	0x10172c8dc <_writev+0x10172c8dc>
1000ff474:     	bl	0x10172c530 <core::panicking::panic_in_cleanup>

