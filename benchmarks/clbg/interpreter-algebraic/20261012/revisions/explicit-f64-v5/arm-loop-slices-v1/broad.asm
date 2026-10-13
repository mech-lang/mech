0000000100127a8c <mech_gpu::sequence::runtime::resident::scalar_row>:
100127a8c:     	sub	sp, sp, #0xe0
100127a90:     	stp	d9, d8, [sp, #0x70]
100127a94:     	stp	x28, x27, [sp, #0x80]
100127a98:     	stp	x26, x25, [sp, #0x90]
100127a9c:     	stp	x24, x23, [sp, #0xa0]
100127aa0:     	stp	x22, x21, [sp, #0xb0]
100127aa4:     	stp	x20, x19, [sp, #0xc0]
100127aa8:     	stp	x29, x30, [sp, #0xd0]
100127aac:     	add	x29, sp, #0xd0
100127ab0:     	str	w7, [sp, #0x44]
100127ab4:     	mov	x20, x4
100127ab8:     	mov	x21, x3
100127abc:     	mov	x25, x0
100127ac0:     	ldr	x8, [x2]
100127ac4:     	mov	x9, #-0x8000000000000000 ; =-9223372036854775808
100127ac8:     	cmp	x8, x9
100127acc:     	b.ne	0x100127b74 <mech_gpu::sequence::runtime::resident::scalar_row+0xe8>
100127ad0:     	cbz	x5, 0x100128aa8 <mech_gpu::sequence::runtime::resident::scalar_row+0x101c>
100127ad4:     	mov	x22, x1
100127ad8:     	lsl	x19, x5, #3
100127adc:     	ldr	w8, [sp, #0x44]
100127ae0:     	cbz	w8, 0x100128ad4 <mech_gpu::sequence::runtime::resident::scalar_row+0x1048>
100127ae4:     	mov	x23, #0x0               ; =0
100127ae8:     	movi.2d	v8, #0000000000000000
100127aec:     	mov	x24, #-0x8000000000000000 ; =-9223372036854775808
100127af0:     	tbz	w6, #0x0, 0x100127b34 <mech_gpu::sequence::runtime::resident::scalar_row+0xa8>
100127af4:     	ldr	d9, [x20, x23, lsl #3]
100127af8:     	stp	x23, x21, [sp, #0x48]
100127afc:     	add	x0, sp, #0x58
100127b00:     	add	x2, sp, #0x48
100127b04:     	mov	x1, x22
100127b08:     	bl	0x100116b20 <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
100127b0c:     	ldr	x8, [sp, #0x58]
100127b10:     	cmp	x8, x24
100127b14:     	b.ne	0x100128b5c <mech_gpu::sequence::runtime::resident::scalar_row+0x10d0>
100127b18:     	add	x23, x23, #0x1
100127b1c:     	ldr	d0, [sp, #0x60]
100127b20:     	fdiv	d0, d9, d0
100127b24:     	fadd	d8, d0, d8
100127b28:     	subs	x19, x19, #0x8
100127b2c:     	b.ne	0x100127af4 <mech_gpu::sequence::runtime::resident::scalar_row+0x68>
100127b30:     	b	0x100128aac <mech_gpu::sequence::runtime::resident::scalar_row+0x1020>
100127b34:     	ldr	d9, [x20, x23, lsl #3]
100127b38:     	stp	x21, x23, [sp, #0x48]
100127b3c:     	add	x0, sp, #0x58
100127b40:     	add	x2, sp, #0x48
100127b44:     	mov	x1, x22
100127b48:     	bl	0x100116b20 <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
100127b4c:     	ldr	x8, [sp, #0x58]
100127b50:     	cmp	x8, x24
100127b54:     	b.ne	0x100128b5c <mech_gpu::sequence::runtime::resident::scalar_row+0x10d0>
100127b58:     	add	x23, x23, #0x1
100127b5c:     	ldr	d0, [sp, #0x60]
100127b60:     	fdiv	d0, d9, d0
100127b64:     	fadd	d8, d0, d8
100127b68:     	subs	x19, x19, #0x8
100127b6c:     	b.ne	0x100127b34 <mech_gpu::sequence::runtime::resident::scalar_row+0xa8>
100127b70:     	b	0x100128aac <mech_gpu::sequence::runtime::resident::scalar_row+0x1020>
100127b74:     	str	x25, [sp, #0x8]
100127b78:     	cbz	x5, 0x100128abc <mech_gpu::sequence::runtime::resident::scalar_row+0x1030>
100127b7c:     	mov	x11, #0x0               ; =0
100127b80:     	mov	x12, #0x0               ; =0
100127b84:     	ldp	x9, x10, [x2, #0x20]
100127b88:     	mov	w8, #0x18               ; =24
100127b8c:     	str	x9, [sp, #0x18]
100127b90:     	madd	x15, x10, x8, x9
100127b94:     	ldp	x16, x1, [x2, #0x8]
100127b98:     	ucvtf	d0, x21
100127b9c:     	ldp	x8, x9, [x2, #0x38]
100127ba0:     	str	x8, [sp, #0x10]
100127ba4:     	stp	x9, x10, [sp, #0x30]
100127ba8:     	add	x4, x8, x9, lsl #5
100127bac:     	ldp	x8, x6, [x2, #0x48]
100127bb0:     	add	x13, x16, x8, lsl #8
100127bb4:     	fmov	x22, d0
100127bb8:     	adrp	x26, 0x101884000 <_anon.d4d64e8e11edd2544ed4904eea679557.190+0xd67>
100127bbc:     	add	x26, x26, #0x36c
100127bc0:     	movi.2d	v0, #0000000000000000
100127bc4:     	mov	w28, #0x8               ; =8
100127bc8:     	stp	x13, x8, [sp, #0x20]
100127bcc:     	b	0x100127be4 <mech_gpu::sequence::runtime::resident::scalar_row+0x158>
100127bd0:     	add	x20, x20, x30, lsl #3
100127bd4:     	add	x12, x12, #0x1
100127bd8:     	add	x11, x11, #0x20
100127bdc:     	sub	x5, x5, x30
100127be0:     	cbz	x5, 0x100128ac0 <mech_gpu::sequence::runtime::resident::scalar_row+0x1034>
100127be4:     	cmp	x5, #0x20
100127be8:     	mov	w8, #0x20               ; =32
100127bec:     	csel	x30, x5, x8, lo
100127bf0:     	ldr	x8, [sp, #0x38]
100127bf4:     	cbz	x8, 0x100127dc8 <mech_gpu::sequence::runtime::resident::scalar_row+0x33c>
100127bf8:     	lsl	x8, x12, #5
100127bfc:     	lsl	x9, x30, #3
100127c00:     	sub	x10, x9, #0x8
100127c04:     	lsr	x13, x10, #3
100127c08:     	add	x13, x13, #0x1
100127c0c:     	and	x14, x13, #0x3ffffffffffffff8
100127c10:     	dup.2d	v1, x8
100127c14:     	lsl	x17, x14, #3
100127c18:     	mov	w8, #0x2                ; =2
100127c1c:     	dup.2d	v2, x8
100127c20:     	add.2d	v2, v1, v2
100127c24:     	mov	w8, #0x4                ; =4
100127c28:     	dup.2d	v3, x8
100127c2c:     	add.2d	v3, v1, v3
100127c30:     	mov	w8, #0x6                ; =6
100127c34:     	dup.2d	v4, x8
100127c38:     	add.2d	v4, v1, v4
100127c3c:     	ldr	x0, [sp, #0x18]
100127c40:     	mov	x3, x0
100127c44:     	b	0x100127c54 <mech_gpu::sequence::runtime::resident::scalar_row+0x1c8>
100127c48:     	mov	x0, x3
100127c4c:     	cmp	x3, x15
100127c50:     	b.eq	0x100127dc8 <mech_gpu::sequence::runtime::resident::scalar_row+0x33c>
100127c54:     	ldr	x8, [x3], #0x18
100127c58:     	cmp	x8, x1
100127c5c:     	b.hs	0x100128bb8 <mech_gpu::sequence::runtime::resident::scalar_row+0x112c>
100127c60:     	ldr	x2, [x0, #0x10]
100127c64:     	ldrb	w0, [x0, #0x8]
100127c68:     	add	x19, x16, x8, lsl #8
100127c6c:     	add	x25, x19, x9
100127c70:     	cmp	x2, x6
100127c74:     	b.ne	0x100127c90 <mech_gpu::sequence::runtime::resident::scalar_row+0x204>
100127c78:     	cmp	w0, #0x1
100127c7c:     	b.ne	0x100127cdc <mech_gpu::sequence::runtime::resident::scalar_row+0x250>
100127c80:     	cmp	x10, #0x38
100127c84:     	b.hs	0x100127cec <mech_gpu::sequence::runtime::resident::scalar_row+0x260>
100127c88:     	mov	x8, #0x0                ; =0
100127c8c:     	b	0x100127d38 <mech_gpu::sequence::runtime::resident::scalar_row+0x2ac>
100127c90:     	cmp	w0, #0x1
100127c94:     	csel	x7, x21, x22, eq
100127c98:     	cmp	x10, #0x38
100127c9c:     	b.lo	0x100127ccc <mech_gpu::sequence::runtime::resident::scalar_row+0x240>
100127ca0:     	add	x19, x19, x17
100127ca4:     	dup.2d	v5, x7
100127ca8:     	add	x0, x16, #0x20
100127cac:     	add	x8, x0, x8, lsl #8
100127cb0:     	and	x0, x13, #0x3ffffffffffffff8
100127cb4:     	stp	q5, q5, [x8, #-0x20]
100127cb8:     	stp	q5, q5, [x8], #0x40
100127cbc:     	subs	x0, x0, #0x8
100127cc0:     	b.ne	0x100127cb4 <mech_gpu::sequence::runtime::resident::scalar_row+0x228>
100127cc4:     	cmp	x13, x14
100127cc8:     	b.eq	0x100127c48 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
100127ccc:     	str	x7, [x19], #0x8
100127cd0:     	cmp	x19, x25
100127cd4:     	b.ne	0x100127ccc <mech_gpu::sequence::runtime::resident::scalar_row+0x240>
100127cd8:     	b	0x100127c48 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
100127cdc:     	cmp	x10, #0x38
100127ce0:     	b.hs	0x100127d50 <mech_gpu::sequence::runtime::resident::scalar_row+0x2c4>
100127ce4:     	mov	x8, #0x0                ; =0
100127ce8:     	b	0x100127dac <mech_gpu::sequence::runtime::resident::scalar_row+0x320>
100127cec:     	add	x19, x19, x17
100127cf0:     	add	x0, x16, #0x20
100127cf4:     	add	x8, x0, x8, lsl #8
100127cf8:     	adrp	x0, 0x10187e000 <_anon.ba0bf445a84b191d67f6477739584b8d.962+0x1841>
100127cfc:     	ldr	q5, [x0, #0xaa0]
100127d00:     	and	x0, x13, #0x3ffffffffffffff8
100127d04:     	add.2d	v6, v5, v1
100127d08:     	add.2d	v7, v5, v2
100127d0c:     	add.2d	v16, v5, v3
100127d10:     	add.2d	v17, v5, v4
100127d14:     	stp	q6, q7, [x8, #-0x20]
100127d18:     	stp	q16, q17, [x8], #0x40
100127d1c:     	dup.2d	v6, x28
100127d20:     	add.2d	v5, v5, v6
100127d24:     	subs	x0, x0, #0x8
100127d28:     	b.ne	0x100127d04 <mech_gpu::sequence::runtime::resident::scalar_row+0x278>
100127d2c:     	and	x8, x13, #0x3ffffffffffffff8
100127d30:     	cmp	x13, x14
100127d34:     	b.eq	0x100127c48 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
100127d38:     	add	x8, x11, x8
100127d3c:     	str	x8, [x19], #0x8
100127d40:     	add	x8, x8, #0x1
100127d44:     	cmp	x19, x25
100127d48:     	b.ne	0x100127d3c <mech_gpu::sequence::runtime::resident::scalar_row+0x2b0>
100127d4c:     	b	0x100127c48 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
100127d50:     	add	x19, x19, x17
100127d54:     	add	x0, x16, #0x20
100127d58:     	add	x8, x0, x8, lsl #8
100127d5c:     	adrp	x0, 0x10187e000 <_anon.ba0bf445a84b191d67f6477739584b8d.962+0x1841>
100127d60:     	ldr	q5, [x0, #0xaa0]
100127d64:     	and	x7, x13, #0x3ffffffffffffff8
100127d68:     	add.2d	v6, v5, v1
100127d6c:     	add.2d	v7, v5, v2
100127d70:     	add.2d	v16, v5, v3
100127d74:     	add.2d	v17, v5, v4
100127d78:     	ucvtf.2d	v6, v6
100127d7c:     	ucvtf.2d	v7, v7
100127d80:     	ucvtf.2d	v16, v16
100127d84:     	ucvtf.2d	v17, v17
100127d88:     	stp	q6, q7, [x8, #-0x20]
100127d8c:     	stp	q16, q17, [x8], #0x40
100127d90:     	dup.2d	v6, x28
100127d94:     	add.2d	v5, v5, v6
100127d98:     	subs	x7, x7, #0x8
100127d9c:     	b.ne	0x100127d68 <mech_gpu::sequence::runtime::resident::scalar_row+0x2dc>
100127da0:     	and	x8, x13, #0x3ffffffffffffff8
100127da4:     	cmp	x13, x14
100127da8:     	b.eq	0x100127c48 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
100127dac:     	add	x8, x11, x8
100127db0:     	ucvtf	d5, x8
100127db4:     	str	d5, [x19], #0x8
100127db8:     	add	x8, x8, #0x1
100127dbc:     	cmp	x19, x25
100127dc0:     	b.ne	0x100127db0 <mech_gpu::sequence::runtime::resident::scalar_row+0x324>
100127dc4:     	b	0x100127c48 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
100127dc8:     	ldr	x8, [sp, #0x30]
100127dcc:     	cbz	x8, 0x100128874 <mech_gpu::sequence::runtime::resident::scalar_row+0xde8>
100127dd0:     	and	x19, x30, #0x38
100127dd4:     	and	x8, x30, #0xfffffffffffffffe
100127dd8:     	neg	x25, x8
100127ddc:     	ldr	x17, [sp, #0x10]
100127de0:     	b	0x100127e00 <mech_gpu::sequence::runtime::resident::scalar_row+0x374>
100127de4:     	mov	x8, #0x0                ; =0
100127de8:     	ldr	x9, [x14, x8, lsl #3]
100127dec:     	ldr	x10, [x3, x8, lsl #3]
100127df0:     	mul	x9, x10, x9
100127df4:     	str	x9, [x13, x8, lsl #3]
100127df8:     	cmp	x17, x4
100127dfc:     	b.eq	0x100128874 <mech_gpu::sequence::runtime::resident::scalar_row+0xde8>
100127e00:     	ldr	x8, [x17, #0x18]
100127e04:     	cmp	x8, x1
100127e08:     	b.hi	0x100128b94 <mech_gpu::sequence::runtime::resident::scalar_row+0x1108>
100127e0c:     	cmp	x1, x8
100127e10:     	b.eq	0x100128bf0 <mech_gpu::sequence::runtime::resident::scalar_row+0x1164>
100127e14:     	mov	x13, x17
100127e18:     	ldr	x9, [x17, #0x8]
100127e1c:     	subs	x24, x8, x9
100127e20:     	b.ls	0x100128bdc <mech_gpu::sequence::runtime::resident::scalar_row+0x1150>
100127e24:     	ldr	x10, [x13, #0x10]
100127e28:     	subs	x7, x8, x10
100127e2c:     	b.ls	0x100128bc8 <mech_gpu::sequence::runtime::resident::scalar_row+0x113c>
100127e30:     	add	x17, x13, #0x20
100127e34:     	ldp	w2, w0, [x13]
100127e38:     	add	x13, x16, x8, lsl #8
100127e3c:     	add	x14, x16, x9, lsl #8
100127e40:     	add	x3, x16, x10, lsl #8
100127e44:     	adr	x27, 0x100127e54 <mech_gpu::sequence::runtime::resident::scalar_row+0x3c8>
100127e48:     	ldrh	w23, [x26, x2, lsl #1]
100127e4c:     	add	x27, x27, x23, lsl #2
100127e50:     	br	x27
100127e54:     	cmp	x5, #0x8
100127e58:     	b.lo	0x1001282e4 <mech_gpu::sequence::runtime::resident::scalar_row+0x858>
100127e5c:     	tst	x24, #0xffffffffffffff
100127e60:     	b.eq	0x1001282e4 <mech_gpu::sequence::runtime::resident::scalar_row+0x858>
100127e64:     	ldp	q1, q2, [x14]
100127e68:     	ldp	q3, q4, [x14, #0x20]
100127e6c:     	ucvtf.2d	v1, v1
100127e70:     	ucvtf.2d	v2, v2
100127e74:     	ucvtf.2d	v3, v3
100127e78:     	ucvtf.2d	v4, v4
100127e7c:     	stp	q1, q2, [x13]
100127e80:     	stp	q3, q4, [x13, #0x20]
100127e84:     	cmp	x19, #0x8
100127e88:     	b.eq	0x100127efc <mech_gpu::sequence::runtime::resident::scalar_row+0x470>
100127e8c:     	ldp	q1, q2, [x14, #0x40]
100127e90:     	ldp	q3, q4, [x14, #0x60]
100127e94:     	ucvtf.2d	v1, v1
100127e98:     	ucvtf.2d	v2, v2
100127e9c:     	ucvtf.2d	v3, v3
100127ea0:     	ucvtf.2d	v4, v4
100127ea4:     	stp	q1, q2, [x13, #0x40]
100127ea8:     	stp	q3, q4, [x13, #0x60]
100127eac:     	cmp	x19, #0x10
100127eb0:     	b.eq	0x100127efc <mech_gpu::sequence::runtime::resident::scalar_row+0x470>
100127eb4:     	ldp	q1, q2, [x14, #0x80]
100127eb8:     	ldp	q3, q4, [x14, #0xa0]
100127ebc:     	ucvtf.2d	v1, v1
100127ec0:     	ucvtf.2d	v2, v2
100127ec4:     	ucvtf.2d	v3, v3
100127ec8:     	ucvtf.2d	v4, v4
100127ecc:     	stp	q1, q2, [x13, #0x80]
100127ed0:     	stp	q3, q4, [x13, #0xa0]
100127ed4:     	cmp	x19, #0x18
100127ed8:     	b.eq	0x100127efc <mech_gpu::sequence::runtime::resident::scalar_row+0x470>
100127edc:     	ldp	q1, q2, [x14, #0xc0]
100127ee0:     	ldp	q3, q4, [x14, #0xe0]
100127ee4:     	ucvtf.2d	v1, v1
100127ee8:     	ucvtf.2d	v2, v2
100127eec:     	ucvtf.2d	v3, v3
100127ef0:     	ucvtf.2d	v4, v4
100127ef4:     	stp	q1, q2, [x13, #0xc0]
100127ef8:     	stp	q3, q4, [x13, #0xe0]
100127efc:     	and	x10, x30, #0x38
100127f00:     	cmp	x30, x19
100127f04:     	b.eq	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100127f08:     	b	0x1001282e8 <mech_gpu::sequence::runtime::resident::scalar_row+0x85c>
100127f0c:     	cmp	x5, #0x8
100127f10:     	b.hs	0x10012834c <mech_gpu::sequence::runtime::resident::scalar_row+0x8c0>
100127f14:     	mov	x0, #0x0                ; =0
100127f18:     	lsl	x13, x0, #3
100127f1c:     	add	x10, x13, x10, lsl #8
100127f20:     	add	x10, x16, x10
100127f24:     	add	x9, x13, x9, lsl #8
100127f28:     	add	x9, x16, x9
100127f2c:     	add	x8, x13, x8, lsl #8
100127f30:     	add	x8, x16, x8
100127f34:     	sub	x13, x30, x0
100127f38:     	ldr	x14, [x9], #0x8
100127f3c:     	ldr	x0, [x10], #0x8
100127f40:     	add	x14, x0, x14
100127f44:     	str	x14, [x8], #0x8
100127f48:     	subs	x13, x13, #0x1
100127f4c:     	b.ne	0x100127f38 <mech_gpu::sequence::runtime::resident::scalar_row+0x4ac>
100127f50:     	b	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100127f54:     	cmp	x5, #0x8
100127f58:     	b.hs	0x100128428 <mech_gpu::sequence::runtime::resident::scalar_row+0x99c>
100127f5c:     	mov	x0, #0x0                ; =0
100127f60:     	lsl	x13, x0, #3
100127f64:     	add	x10, x13, x10, lsl #8
100127f68:     	add	x10, x16, x10
100127f6c:     	add	x9, x13, x9, lsl #8
100127f70:     	add	x9, x16, x9
100127f74:     	add	x8, x13, x8, lsl #8
100127f78:     	add	x8, x16, x8
100127f7c:     	sub	x13, x30, x0
100127f80:     	ldr	d1, [x9], #0x8
100127f84:     	ldr	d2, [x10], #0x8
100127f88:     	fsub	d1, d1, d2
100127f8c:     	str	d1, [x8], #0x8
100127f90:     	subs	x13, x13, #0x1
100127f94:     	b.ne	0x100127f80 <mech_gpu::sequence::runtime::resident::scalar_row+0x4f4>
100127f98:     	b	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100127f9c:     	mov	x8, x30
100127fa0:     	ldr	x9, [x3], #0x8
100127fa4:     	cbz	x9, 0x100128bac <mech_gpu::sequence::runtime::resident::scalar_row+0x1120>
100127fa8:     	ldr	x10, [x14], #0x8
100127fac:     	udiv	x9, x10, x9
100127fb0:     	str	x9, [x13], #0x8
100127fb4:     	subs	x8, x8, #0x1
100127fb8:     	b.ne	0x100127fa0 <mech_gpu::sequence::runtime::resident::scalar_row+0x514>
100127fbc:     	b	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100127fc0:     	and	x10, x0, #0x3f
100127fc4:     	cmp	x5, #0x8
100127fc8:     	b.lo	0x1001282b0 <mech_gpu::sequence::runtime::resident::scalar_row+0x824>
100127fcc:     	tst	x24, #0xffffffffffffff
100127fd0:     	b.eq	0x1001282b0 <mech_gpu::sequence::runtime::resident::scalar_row+0x824>
100127fd4:     	dup.2d	v1, x10
100127fd8:     	ldp	q2, q3, [x14]
100127fdc:     	ldp	q4, q5, [x14, #0x20]
100127fe0:     	neg.2d	v1, v1
100127fe4:     	ushl.2d	v2, v2, v1
100127fe8:     	ushl.2d	v3, v3, v1
100127fec:     	ushl.2d	v4, v4, v1
100127ff0:     	ushl.2d	v5, v5, v1
100127ff4:     	stp	q2, q3, [x13]
100127ff8:     	stp	q4, q5, [x13, #0x20]
100127ffc:     	cmp	x19, #0x8
100128000:     	b.eq	0x100128074 <mech_gpu::sequence::runtime::resident::scalar_row+0x5e8>
100128004:     	ldp	q2, q3, [x14, #0x40]
100128008:     	ldp	q4, q5, [x14, #0x60]
10012800c:     	ushl.2d	v2, v2, v1
100128010:     	ushl.2d	v3, v3, v1
100128014:     	ushl.2d	v4, v4, v1
100128018:     	ushl.2d	v5, v5, v1
10012801c:     	stp	q2, q3, [x13, #0x40]
100128020:     	stp	q4, q5, [x13, #0x60]
100128024:     	cmp	x19, #0x10
100128028:     	b.eq	0x100128074 <mech_gpu::sequence::runtime::resident::scalar_row+0x5e8>
10012802c:     	ldp	q2, q3, [x14, #0x80]
100128030:     	ldp	q4, q5, [x14, #0xa0]
100128034:     	ushl.2d	v2, v2, v1
100128038:     	ushl.2d	v3, v3, v1
10012803c:     	ushl.2d	v4, v4, v1
100128040:     	ushl.2d	v5, v5, v1
100128044:     	stp	q2, q3, [x13, #0x80]
100128048:     	stp	q4, q5, [x13, #0xa0]
10012804c:     	cmp	x19, #0x18
100128050:     	b.eq	0x100128074 <mech_gpu::sequence::runtime::resident::scalar_row+0x5e8>
100128054:     	ldp	q2, q3, [x14, #0xc0]
100128058:     	ldp	q4, q5, [x14, #0xe0]
10012805c:     	ushl.2d	v2, v2, v1
100128060:     	ushl.2d	v3, v3, v1
100128064:     	ushl.2d	v4, v4, v1
100128068:     	ushl.2d	v1, v5, v1
10012806c:     	stp	q2, q3, [x13, #0xc0]
100128070:     	stp	q4, q1, [x13, #0xe0]
100128074:     	and	x13, x30, #0x38
100128078:     	cmp	x30, x19
10012807c:     	b.eq	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100128080:     	b	0x1001282b4 <mech_gpu::sequence::runtime::resident::scalar_row+0x828>
100128084:     	cmp	x5, #0x8
100128088:     	b.hs	0x100128504 <mech_gpu::sequence::runtime::resident::scalar_row+0xa78>
10012808c:     	mov	x0, #0x0                ; =0
100128090:     	lsl	x13, x0, #3
100128094:     	add	x10, x13, x10, lsl #8
100128098:     	add	x10, x16, x10
10012809c:     	add	x9, x13, x9, lsl #8
1001280a0:     	add	x9, x16, x9
1001280a4:     	add	x8, x13, x8, lsl #8
1001280a8:     	add	x8, x16, x8
1001280ac:     	sub	x13, x30, x0
1001280b0:     	ldr	d1, [x9], #0x8
1001280b4:     	ldr	d2, [x10], #0x8
1001280b8:     	fdiv	d1, d1, d2
1001280bc:     	str	d1, [x8], #0x8
1001280c0:     	subs	x13, x13, #0x1
1001280c4:     	b.ne	0x1001280b0 <mech_gpu::sequence::runtime::resident::scalar_row+0x624>
1001280c8:     	b	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1001280cc:     	cmp	x5, #0x1
1001280d0:     	b.eq	0x100127de4 <mech_gpu::sequence::runtime::resident::scalar_row+0x358>
1001280d4:     	mov	x7, #0x0                ; =0
1001280d8:     	add	x0, x16, #0x8
1001280dc:     	add	x8, x0, x8, lsl #8
1001280e0:     	add	x9, x0, x9, lsl #8
1001280e4:     	add	x10, x0, x10, lsl #8
1001280e8:     	ldur	x0, [x9, #-0x8]
1001280ec:     	ldur	x2, [x10, #-0x8]
1001280f0:     	mul	x0, x2, x0
1001280f4:     	stur	x0, [x8, #-0x8]
1001280f8:     	ldr	x0, [x9], #0x10
1001280fc:     	ldr	x2, [x10], #0x10
100128100:     	mul	x0, x2, x0
100128104:     	sub	x7, x7, #0x2
100128108:     	str	x0, [x8], #0x10
10012810c:     	cmp	x25, x7
100128110:     	b.ne	0x1001280e8 <mech_gpu::sequence::runtime::resident::scalar_row+0x65c>
100128114:     	tbz	w30, #0x0, 0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100128118:     	neg	x8, x7
10012811c:     	b	0x100127de8 <mech_gpu::sequence::runtime::resident::scalar_row+0x35c>
100128120:     	cmp	x5, #0x8
100128124:     	b.hs	0x1001285e0 <mech_gpu::sequence::runtime::resident::scalar_row+0xb54>
100128128:     	mov	x0, #0x0                ; =0
10012812c:     	lsl	x13, x0, #3
100128130:     	add	x10, x13, x10, lsl #8
100128134:     	add	x10, x16, x10
100128138:     	add	x9, x13, x9, lsl #8
10012813c:     	add	x9, x16, x9
100128140:     	add	x8, x13, x8, lsl #8
100128144:     	add	x8, x16, x8
100128148:     	sub	x13, x30, x0
10012814c:     	ldr	x14, [x9], #0x8
100128150:     	ldr	x0, [x10], #0x8
100128154:     	sub	x14, x14, x0
100128158:     	str	x14, [x8], #0x8
10012815c:     	subs	x13, x13, #0x1
100128160:     	b.ne	0x10012814c <mech_gpu::sequence::runtime::resident::scalar_row+0x6c0>
100128164:     	b	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100128168:     	cmp	x5, #0x8
10012816c:     	b.hs	0x1001286bc <mech_gpu::sequence::runtime::resident::scalar_row+0xc30>
100128170:     	mov	x0, #0x0                ; =0
100128174:     	lsl	x13, x0, #3
100128178:     	add	x10, x13, x10, lsl #8
10012817c:     	add	x10, x16, x10
100128180:     	add	x9, x13, x9, lsl #8
100128184:     	add	x9, x16, x9
100128188:     	add	x8, x13, x8, lsl #8
10012818c:     	add	x8, x16, x8
100128190:     	sub	x13, x30, x0
100128194:     	ldr	d1, [x9], #0x8
100128198:     	ldr	d2, [x10], #0x8
10012819c:     	fmul	d1, d1, d2
1001281a0:     	str	d1, [x8], #0x8
1001281a4:     	subs	x13, x13, #0x1
1001281a8:     	b.ne	0x100128194 <mech_gpu::sequence::runtime::resident::scalar_row+0x708>
1001281ac:     	b	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1001281b0:     	cmp	x5, #0x8
1001281b4:     	b.lo	0x100128318 <mech_gpu::sequence::runtime::resident::scalar_row+0x88c>
1001281b8:     	tst	x24, #0xffffffffffffff
1001281bc:     	b.eq	0x100128318 <mech_gpu::sequence::runtime::resident::scalar_row+0x88c>
1001281c0:     	ldp	q1, q2, [x14]
1001281c4:     	ldp	q3, q4, [x14, #0x20]
1001281c8:     	fsqrt.2d	v1, v1
1001281cc:     	fsqrt.2d	v2, v2
1001281d0:     	fsqrt.2d	v3, v3
1001281d4:     	fsqrt.2d	v4, v4
1001281d8:     	stp	q1, q2, [x13]
1001281dc:     	stp	q3, q4, [x13, #0x20]
1001281e0:     	cmp	x19, #0x8
1001281e4:     	b.eq	0x100128258 <mech_gpu::sequence::runtime::resident::scalar_row+0x7cc>
1001281e8:     	ldp	q1, q2, [x14, #0x40]
1001281ec:     	ldp	q3, q4, [x14, #0x60]
1001281f0:     	fsqrt.2d	v1, v1
1001281f4:     	fsqrt.2d	v2, v2
1001281f8:     	fsqrt.2d	v3, v3
1001281fc:     	fsqrt.2d	v4, v4
100128200:     	stp	q1, q2, [x13, #0x40]
100128204:     	stp	q3, q4, [x13, #0x60]
100128208:     	cmp	x19, #0x10
10012820c:     	b.eq	0x100128258 <mech_gpu::sequence::runtime::resident::scalar_row+0x7cc>
100128210:     	ldp	q1, q2, [x14, #0x80]
100128214:     	ldp	q3, q4, [x14, #0xa0]
100128218:     	fsqrt.2d	v1, v1
10012821c:     	fsqrt.2d	v2, v2
100128220:     	fsqrt.2d	v3, v3
100128224:     	fsqrt.2d	v4, v4
100128228:     	stp	q1, q2, [x13, #0x80]
10012822c:     	stp	q3, q4, [x13, #0xa0]
100128230:     	cmp	x19, #0x18
100128234:     	b.eq	0x100128258 <mech_gpu::sequence::runtime::resident::scalar_row+0x7cc>
100128238:     	ldp	q1, q2, [x14, #0xc0]
10012823c:     	ldp	q3, q4, [x14, #0xe0]
100128240:     	fsqrt.2d	v1, v1
100128244:     	fsqrt.2d	v2, v2
100128248:     	fsqrt.2d	v3, v3
10012824c:     	fsqrt.2d	v4, v4
100128250:     	stp	q1, q2, [x13, #0xc0]
100128254:     	stp	q3, q4, [x13, #0xe0]
100128258:     	and	x10, x30, #0x38
10012825c:     	cmp	x30, x19
100128260:     	b.eq	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100128264:     	b	0x10012831c <mech_gpu::sequence::runtime::resident::scalar_row+0x890>
100128268:     	cmp	x5, #0x8
10012826c:     	b.hs	0x100128798 <mech_gpu::sequence::runtime::resident::scalar_row+0xd0c>
100128270:     	mov	x0, #0x0                ; =0
100128274:     	lsl	x13, x0, #3
100128278:     	add	x10, x13, x10, lsl #8
10012827c:     	add	x10, x16, x10
100128280:     	add	x9, x13, x9, lsl #8
100128284:     	add	x9, x16, x9
100128288:     	add	x8, x13, x8, lsl #8
10012828c:     	add	x8, x16, x8
100128290:     	sub	x13, x30, x0
100128294:     	ldr	d1, [x9], #0x8
100128298:     	ldr	d2, [x10], #0x8
10012829c:     	fadd	d1, d1, d2
1001282a0:     	str	d1, [x8], #0x8
1001282a4:     	subs	x13, x13, #0x1
1001282a8:     	b.ne	0x100128294 <mech_gpu::sequence::runtime::resident::scalar_row+0x808>
1001282ac:     	b	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1001282b0:     	mov	x13, #0x0               ; =0
1001282b4:     	lsl	x14, x13, #3
1001282b8:     	add	x9, x14, x9, lsl #8
1001282bc:     	add	x9, x16, x9
1001282c0:     	add	x8, x14, x8, lsl #8
1001282c4:     	add	x8, x16, x8
1001282c8:     	sub	x13, x30, x13
1001282cc:     	ldr	x14, [x9], #0x8
1001282d0:     	lsr	x14, x14, x10
1001282d4:     	str	x14, [x8], #0x8
1001282d8:     	subs	x13, x13, #0x1
1001282dc:     	b.ne	0x1001282cc <mech_gpu::sequence::runtime::resident::scalar_row+0x840>
1001282e0:     	b	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1001282e4:     	mov	x10, #0x0               ; =0
1001282e8:     	lsl	x13, x10, #3
1001282ec:     	add	x9, x13, x9, lsl #8
1001282f0:     	add	x9, x16, x9
1001282f4:     	add	x8, x13, x8, lsl #8
1001282f8:     	add	x8, x16, x8
1001282fc:     	sub	x10, x30, x10
100128300:     	ldr	d1, [x9], #0x8
100128304:     	ucvtf	d1, d1
100128308:     	str	d1, [x8], #0x8
10012830c:     	subs	x10, x10, #0x1
100128310:     	b.ne	0x100128300 <mech_gpu::sequence::runtime::resident::scalar_row+0x874>
100128314:     	b	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100128318:     	mov	x10, #0x0               ; =0
10012831c:     	lsl	x13, x10, #3
100128320:     	add	x9, x13, x9, lsl #8
100128324:     	add	x9, x16, x9
100128328:     	add	x8, x13, x8, lsl #8
10012832c:     	add	x8, x16, x8
100128330:     	sub	x10, x30, x10
100128334:     	ldr	d1, [x9], #0x8
100128338:     	fsqrt	d1, d1
10012833c:     	str	d1, [x8], #0x8
100128340:     	subs	x10, x10, #0x1
100128344:     	b.ne	0x100128334 <mech_gpu::sequence::runtime::resident::scalar_row+0x8a8>
100128348:     	b	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
10012834c:     	mov	x0, #0x0                ; =0
100128350:     	and	x2, x24, #0xffffffffffffff
100128354:     	cbz	x2, 0x100127f18 <mech_gpu::sequence::runtime::resident::scalar_row+0x48c>
100128358:     	and	x2, x7, #0xffffffffffffff
10012835c:     	cbz	x2, 0x100127f18 <mech_gpu::sequence::runtime::resident::scalar_row+0x48c>
100128360:     	ldp	q1, q2, [x14]
100128364:     	ldp	q3, q4, [x14, #0x20]
100128368:     	ldp	q5, q6, [x3]
10012836c:     	ldp	q7, q16, [x3, #0x20]
100128370:     	add.2d	v1, v5, v1
100128374:     	add.2d	v2, v6, v2
100128378:     	add.2d	v3, v7, v3
10012837c:     	add.2d	v4, v16, v4
100128380:     	stp	q1, q2, [x13]
100128384:     	stp	q3, q4, [x13, #0x20]
100128388:     	cmp	x19, #0x8
10012838c:     	b.eq	0x100128418 <mech_gpu::sequence::runtime::resident::scalar_row+0x98c>
100128390:     	ldp	q1, q2, [x14, #0x40]
100128394:     	ldp	q3, q4, [x14, #0x60]
100128398:     	ldp	q5, q6, [x3, #0x40]
10012839c:     	ldp	q7, q16, [x3, #0x60]
1001283a0:     	add.2d	v1, v5, v1
1001283a4:     	add.2d	v2, v6, v2
1001283a8:     	add.2d	v3, v7, v3
1001283ac:     	add.2d	v4, v16, v4
1001283b0:     	stp	q1, q2, [x13, #0x40]
1001283b4:     	stp	q3, q4, [x13, #0x60]
1001283b8:     	cmp	x19, #0x10
1001283bc:     	b.eq	0x100128418 <mech_gpu::sequence::runtime::resident::scalar_row+0x98c>
1001283c0:     	ldp	q1, q2, [x14, #0x80]
1001283c4:     	ldp	q3, q4, [x14, #0xa0]
1001283c8:     	ldp	q5, q6, [x3, #0x80]
1001283cc:     	ldp	q7, q16, [x3, #0xa0]
1001283d0:     	add.2d	v1, v5, v1
1001283d4:     	add.2d	v2, v6, v2
1001283d8:     	add.2d	v3, v7, v3
1001283dc:     	add.2d	v4, v16, v4
1001283e0:     	stp	q1, q2, [x13, #0x80]
1001283e4:     	stp	q3, q4, [x13, #0xa0]
1001283e8:     	cmp	x19, #0x18
1001283ec:     	b.eq	0x100128418 <mech_gpu::sequence::runtime::resident::scalar_row+0x98c>
1001283f0:     	ldp	q1, q2, [x14, #0xc0]
1001283f4:     	ldp	q3, q4, [x14, #0xe0]
1001283f8:     	ldp	q5, q6, [x3, #0xc0]
1001283fc:     	ldp	q7, q16, [x3, #0xe0]
100128400:     	add.2d	v1, v5, v1
100128404:     	add.2d	v2, v6, v2
100128408:     	add.2d	v3, v7, v3
10012840c:     	add.2d	v4, v16, v4
100128410:     	stp	q1, q2, [x13, #0xc0]
100128414:     	stp	q3, q4, [x13, #0xe0]
100128418:     	and	x0, x30, #0x38
10012841c:     	cmp	x30, x19
100128420:     	b.eq	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100128424:     	b	0x100127f18 <mech_gpu::sequence::runtime::resident::scalar_row+0x48c>
100128428:     	mov	x0, #0x0                ; =0
10012842c:     	and	x2, x24, #0xffffffffffffff
100128430:     	cbz	x2, 0x100127f60 <mech_gpu::sequence::runtime::resident::scalar_row+0x4d4>
100128434:     	and	x2, x7, #0xffffffffffffff
100128438:     	cbz	x2, 0x100127f60 <mech_gpu::sequence::runtime::resident::scalar_row+0x4d4>
10012843c:     	ldp	q1, q2, [x14]
100128440:     	ldp	q3, q4, [x14, #0x20]
100128444:     	ldp	q5, q6, [x3]
100128448:     	ldp	q7, q16, [x3, #0x20]
10012844c:     	fsub.2d	v1, v1, v5
100128450:     	fsub.2d	v2, v2, v6
100128454:     	fsub.2d	v3, v3, v7
100128458:     	fsub.2d	v4, v4, v16
10012845c:     	stp	q1, q2, [x13]
100128460:     	stp	q3, q4, [x13, #0x20]
100128464:     	cmp	x19, #0x8
100128468:     	b.eq	0x1001284f4 <mech_gpu::sequence::runtime::resident::scalar_row+0xa68>
10012846c:     	ldp	q1, q2, [x14, #0x40]
100128470:     	ldp	q3, q4, [x14, #0x60]
100128474:     	ldp	q5, q6, [x3, #0x40]
100128478:     	ldp	q7, q16, [x3, #0x60]
10012847c:     	fsub.2d	v1, v1, v5
100128480:     	fsub.2d	v2, v2, v6
100128484:     	fsub.2d	v3, v3, v7
100128488:     	fsub.2d	v4, v4, v16
10012848c:     	stp	q1, q2, [x13, #0x40]
100128490:     	stp	q3, q4, [x13, #0x60]
100128494:     	cmp	x19, #0x10
100128498:     	b.eq	0x1001284f4 <mech_gpu::sequence::runtime::resident::scalar_row+0xa68>
10012849c:     	ldp	q1, q2, [x14, #0x80]
1001284a0:     	ldp	q3, q4, [x14, #0xa0]
1001284a4:     	ldp	q5, q6, [x3, #0x80]
1001284a8:     	ldp	q7, q16, [x3, #0xa0]
1001284ac:     	fsub.2d	v1, v1, v5
1001284b0:     	fsub.2d	v2, v2, v6
1001284b4:     	fsub.2d	v3, v3, v7
1001284b8:     	fsub.2d	v4, v4, v16
1001284bc:     	stp	q1, q2, [x13, #0x80]
1001284c0:     	stp	q3, q4, [x13, #0xa0]
1001284c4:     	cmp	x19, #0x18
1001284c8:     	b.eq	0x1001284f4 <mech_gpu::sequence::runtime::resident::scalar_row+0xa68>
1001284cc:     	ldp	q1, q2, [x14, #0xc0]
1001284d0:     	ldp	q3, q4, [x14, #0xe0]
1001284d4:     	ldp	q5, q6, [x3, #0xc0]
1001284d8:     	ldp	q7, q16, [x3, #0xe0]
1001284dc:     	fsub.2d	v1, v1, v5
1001284e0:     	fsub.2d	v2, v2, v6
1001284e4:     	fsub.2d	v3, v3, v7
1001284e8:     	fsub.2d	v4, v4, v16
1001284ec:     	stp	q1, q2, [x13, #0xc0]
1001284f0:     	stp	q3, q4, [x13, #0xe0]
1001284f4:     	and	x0, x30, #0x38
1001284f8:     	cmp	x30, x19
1001284fc:     	b.eq	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100128500:     	b	0x100127f60 <mech_gpu::sequence::runtime::resident::scalar_row+0x4d4>
100128504:     	mov	x0, #0x0                ; =0
100128508:     	and	x2, x24, #0xffffffffffffff
10012850c:     	cbz	x2, 0x100128090 <mech_gpu::sequence::runtime::resident::scalar_row+0x604>
100128510:     	and	x2, x7, #0xffffffffffffff
100128514:     	cbz	x2, 0x100128090 <mech_gpu::sequence::runtime::resident::scalar_row+0x604>
100128518:     	ldp	q1, q2, [x14]
10012851c:     	ldp	q3, q4, [x14, #0x20]
100128520:     	ldp	q5, q6, [x3]
100128524:     	ldp	q7, q16, [x3, #0x20]
100128528:     	fdiv.2d	v1, v1, v5
10012852c:     	fdiv.2d	v2, v2, v6
100128530:     	fdiv.2d	v3, v3, v7
100128534:     	fdiv.2d	v4, v4, v16
100128538:     	stp	q1, q2, [x13]
10012853c:     	stp	q3, q4, [x13, #0x20]
100128540:     	cmp	x19, #0x8
100128544:     	b.eq	0x1001285d0 <mech_gpu::sequence::runtime::resident::scalar_row+0xb44>
100128548:     	ldp	q1, q2, [x14, #0x40]
10012854c:     	ldp	q3, q4, [x14, #0x60]
100128550:     	ldp	q5, q6, [x3, #0x40]
100128554:     	ldp	q7, q16, [x3, #0x60]
100128558:     	fdiv.2d	v1, v1, v5
10012855c:     	fdiv.2d	v2, v2, v6
100128560:     	fdiv.2d	v3, v3, v7
100128564:     	fdiv.2d	v4, v4, v16
100128568:     	stp	q1, q2, [x13, #0x40]
10012856c:     	stp	q3, q4, [x13, #0x60]
100128570:     	cmp	x19, #0x10
100128574:     	b.eq	0x1001285d0 <mech_gpu::sequence::runtime::resident::scalar_row+0xb44>
100128578:     	ldp	q1, q2, [x14, #0x80]
10012857c:     	ldp	q3, q4, [x14, #0xa0]
100128580:     	ldp	q5, q6, [x3, #0x80]
100128584:     	ldp	q7, q16, [x3, #0xa0]
100128588:     	fdiv.2d	v1, v1, v5
10012858c:     	fdiv.2d	v2, v2, v6
100128590:     	fdiv.2d	v3, v3, v7
100128594:     	fdiv.2d	v4, v4, v16
100128598:     	stp	q1, q2, [x13, #0x80]
10012859c:     	stp	q3, q4, [x13, #0xa0]
1001285a0:     	cmp	x19, #0x18
1001285a4:     	b.eq	0x1001285d0 <mech_gpu::sequence::runtime::resident::scalar_row+0xb44>
1001285a8:     	ldp	q1, q2, [x14, #0xc0]
1001285ac:     	ldp	q3, q4, [x14, #0xe0]
1001285b0:     	ldp	q5, q6, [x3, #0xc0]
1001285b4:     	ldp	q7, q16, [x3, #0xe0]
1001285b8:     	fdiv.2d	v1, v1, v5
1001285bc:     	fdiv.2d	v2, v2, v6
1001285c0:     	fdiv.2d	v3, v3, v7
1001285c4:     	fdiv.2d	v4, v4, v16
1001285c8:     	stp	q1, q2, [x13, #0xc0]
1001285cc:     	stp	q3, q4, [x13, #0xe0]
1001285d0:     	and	x0, x30, #0x38
1001285d4:     	cmp	x30, x19
1001285d8:     	b.eq	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1001285dc:     	b	0x100128090 <mech_gpu::sequence::runtime::resident::scalar_row+0x604>
1001285e0:     	mov	x0, #0x0                ; =0
1001285e4:     	and	x2, x24, #0xffffffffffffff
1001285e8:     	cbz	x2, 0x10012812c <mech_gpu::sequence::runtime::resident::scalar_row+0x6a0>
1001285ec:     	and	x2, x7, #0xffffffffffffff
1001285f0:     	cbz	x2, 0x10012812c <mech_gpu::sequence::runtime::resident::scalar_row+0x6a0>
1001285f4:     	ldp	q1, q2, [x14]
1001285f8:     	ldp	q3, q4, [x14, #0x20]
1001285fc:     	ldp	q5, q6, [x3]
100128600:     	ldp	q7, q16, [x3, #0x20]
100128604:     	sub.2d	v1, v1, v5
100128608:     	sub.2d	v2, v2, v6
10012860c:     	sub.2d	v3, v3, v7
100128610:     	sub.2d	v4, v4, v16
100128614:     	stp	q1, q2, [x13]
100128618:     	stp	q3, q4, [x13, #0x20]
10012861c:     	cmp	x19, #0x8
100128620:     	b.eq	0x1001286ac <mech_gpu::sequence::runtime::resident::scalar_row+0xc20>
100128624:     	ldp	q1, q2, [x14, #0x40]
100128628:     	ldp	q3, q4, [x14, #0x60]
10012862c:     	ldp	q5, q6, [x3, #0x40]
100128630:     	ldp	q7, q16, [x3, #0x60]
100128634:     	sub.2d	v1, v1, v5
100128638:     	sub.2d	v2, v2, v6
10012863c:     	sub.2d	v3, v3, v7
100128640:     	sub.2d	v4, v4, v16
100128644:     	stp	q1, q2, [x13, #0x40]
100128648:     	stp	q3, q4, [x13, #0x60]
10012864c:     	cmp	x19, #0x10
100128650:     	b.eq	0x1001286ac <mech_gpu::sequence::runtime::resident::scalar_row+0xc20>
100128654:     	ldp	q1, q2, [x14, #0x80]
100128658:     	ldp	q3, q4, [x14, #0xa0]
10012865c:     	ldp	q5, q6, [x3, #0x80]
100128660:     	ldp	q7, q16, [x3, #0xa0]
100128664:     	sub.2d	v1, v1, v5
100128668:     	sub.2d	v2, v2, v6
10012866c:     	sub.2d	v3, v3, v7
100128670:     	sub.2d	v4, v4, v16
100128674:     	stp	q1, q2, [x13, #0x80]
100128678:     	stp	q3, q4, [x13, #0xa0]
10012867c:     	cmp	x19, #0x18
100128680:     	b.eq	0x1001286ac <mech_gpu::sequence::runtime::resident::scalar_row+0xc20>
100128684:     	ldp	q1, q2, [x14, #0xc0]
100128688:     	ldp	q3, q4, [x14, #0xe0]
10012868c:     	ldp	q5, q6, [x3, #0xc0]
100128690:     	ldp	q7, q16, [x3, #0xe0]
100128694:     	sub.2d	v1, v1, v5
100128698:     	sub.2d	v2, v2, v6
10012869c:     	sub.2d	v3, v3, v7
1001286a0:     	sub.2d	v4, v4, v16
1001286a4:     	stp	q1, q2, [x13, #0xc0]
1001286a8:     	stp	q3, q4, [x13, #0xe0]
1001286ac:     	and	x0, x30, #0x38
1001286b0:     	cmp	x30, x19
1001286b4:     	b.eq	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1001286b8:     	b	0x10012812c <mech_gpu::sequence::runtime::resident::scalar_row+0x6a0>
1001286bc:     	mov	x0, #0x0                ; =0
1001286c0:     	and	x2, x24, #0xffffffffffffff
1001286c4:     	cbz	x2, 0x100128174 <mech_gpu::sequence::runtime::resident::scalar_row+0x6e8>
1001286c8:     	and	x2, x7, #0xffffffffffffff
1001286cc:     	cbz	x2, 0x100128174 <mech_gpu::sequence::runtime::resident::scalar_row+0x6e8>
1001286d0:     	ldp	q1, q2, [x14]
1001286d4:     	ldp	q3, q4, [x14, #0x20]
1001286d8:     	ldp	q5, q6, [x3]
1001286dc:     	ldp	q7, q16, [x3, #0x20]
1001286e0:     	fmul.2d	v1, v1, v5
1001286e4:     	fmul.2d	v2, v2, v6
1001286e8:     	fmul.2d	v3, v3, v7
1001286ec:     	fmul.2d	v4, v4, v16
1001286f0:     	stp	q1, q2, [x13]
1001286f4:     	stp	q3, q4, [x13, #0x20]
1001286f8:     	cmp	x19, #0x8
1001286fc:     	b.eq	0x100128788 <mech_gpu::sequence::runtime::resident::scalar_row+0xcfc>
100128700:     	ldp	q1, q2, [x14, #0x40]
100128704:     	ldp	q3, q4, [x14, #0x60]
100128708:     	ldp	q5, q6, [x3, #0x40]
10012870c:     	ldp	q7, q16, [x3, #0x60]
100128710:     	fmul.2d	v1, v1, v5
100128714:     	fmul.2d	v2, v2, v6
100128718:     	fmul.2d	v3, v3, v7
10012871c:     	fmul.2d	v4, v4, v16
100128720:     	stp	q1, q2, [x13, #0x40]
100128724:     	stp	q3, q4, [x13, #0x60]
100128728:     	cmp	x19, #0x10
10012872c:     	b.eq	0x100128788 <mech_gpu::sequence::runtime::resident::scalar_row+0xcfc>
100128730:     	ldp	q1, q2, [x14, #0x80]
100128734:     	ldp	q3, q4, [x14, #0xa0]
100128738:     	ldp	q5, q6, [x3, #0x80]
10012873c:     	ldp	q7, q16, [x3, #0xa0]
100128740:     	fmul.2d	v1, v1, v5
100128744:     	fmul.2d	v2, v2, v6
100128748:     	fmul.2d	v3, v3, v7
10012874c:     	fmul.2d	v4, v4, v16
100128750:     	stp	q1, q2, [x13, #0x80]
100128754:     	stp	q3, q4, [x13, #0xa0]
100128758:     	cmp	x19, #0x18
10012875c:     	b.eq	0x100128788 <mech_gpu::sequence::runtime::resident::scalar_row+0xcfc>
100128760:     	ldp	q1, q2, [x14, #0xc0]
100128764:     	ldp	q3, q4, [x14, #0xe0]
100128768:     	ldp	q5, q6, [x3, #0xc0]
10012876c:     	ldp	q7, q16, [x3, #0xe0]
100128770:     	fmul.2d	v1, v1, v5
100128774:     	fmul.2d	v2, v2, v6
100128778:     	fmul.2d	v3, v3, v7
10012877c:     	fmul.2d	v4, v4, v16
100128780:     	stp	q1, q2, [x13, #0xc0]
100128784:     	stp	q3, q4, [x13, #0xe0]
100128788:     	and	x0, x30, #0x38
10012878c:     	cmp	x30, x19
100128790:     	b.eq	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100128794:     	b	0x100128174 <mech_gpu::sequence::runtime::resident::scalar_row+0x6e8>
100128798:     	mov	x0, #0x0                ; =0
10012879c:     	and	x2, x24, #0xffffffffffffff
1001287a0:     	cbz	x2, 0x100128274 <mech_gpu::sequence::runtime::resident::scalar_row+0x7e8>
1001287a4:     	and	x2, x7, #0xffffffffffffff
1001287a8:     	cbz	x2, 0x100128274 <mech_gpu::sequence::runtime::resident::scalar_row+0x7e8>
1001287ac:     	ldp	q1, q2, [x14]
1001287b0:     	ldp	q3, q4, [x14, #0x20]
1001287b4:     	ldp	q5, q6, [x3]
1001287b8:     	ldp	q7, q16, [x3, #0x20]
1001287bc:     	fadd.2d	v1, v1, v5
1001287c0:     	fadd.2d	v2, v2, v6
1001287c4:     	fadd.2d	v3, v3, v7
1001287c8:     	fadd.2d	v4, v4, v16
1001287cc:     	stp	q1, q2, [x13]
1001287d0:     	stp	q3, q4, [x13, #0x20]
1001287d4:     	cmp	x19, #0x8
1001287d8:     	b.eq	0x100128864 <mech_gpu::sequence::runtime::resident::scalar_row+0xdd8>
1001287dc:     	ldp	q1, q2, [x14, #0x40]
1001287e0:     	ldp	q3, q4, [x14, #0x60]
1001287e4:     	ldp	q5, q6, [x3, #0x40]
1001287e8:     	ldp	q7, q16, [x3, #0x60]
1001287ec:     	fadd.2d	v1, v1, v5
1001287f0:     	fadd.2d	v2, v2, v6
1001287f4:     	fadd.2d	v3, v3, v7
1001287f8:     	fadd.2d	v4, v4, v16
1001287fc:     	stp	q1, q2, [x13, #0x40]
100128800:     	stp	q3, q4, [x13, #0x60]
100128804:     	cmp	x19, #0x10
100128808:     	b.eq	0x100128864 <mech_gpu::sequence::runtime::resident::scalar_row+0xdd8>
10012880c:     	ldp	q1, q2, [x14, #0x80]
100128810:     	ldp	q3, q4, [x14, #0xa0]
100128814:     	ldp	q5, q6, [x3, #0x80]
100128818:     	ldp	q7, q16, [x3, #0xa0]
10012881c:     	fadd.2d	v1, v1, v5
100128820:     	fadd.2d	v2, v2, v6
100128824:     	fadd.2d	v3, v3, v7
100128828:     	fadd.2d	v4, v4, v16
10012882c:     	stp	q1, q2, [x13, #0x80]
100128830:     	stp	q3, q4, [x13, #0xa0]
100128834:     	cmp	x19, #0x18
100128838:     	b.eq	0x100128864 <mech_gpu::sequence::runtime::resident::scalar_row+0xdd8>
10012883c:     	ldp	q1, q2, [x14, #0xc0]
100128840:     	ldp	q3, q4, [x14, #0xe0]
100128844:     	ldp	q5, q6, [x3, #0xc0]
100128848:     	ldp	q7, q16, [x3, #0xe0]
10012884c:     	fadd.2d	v1, v1, v5
100128850:     	fadd.2d	v2, v2, v6
100128854:     	fadd.2d	v3, v3, v7
100128858:     	fadd.2d	v4, v4, v16
10012885c:     	stp	q1, q2, [x13, #0xc0]
100128860:     	stp	q3, q4, [x13, #0xe0]
100128864:     	and	x0, x30, #0x38
100128868:     	cmp	x30, x19
10012886c:     	b.eq	0x100127df8 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
100128870:     	b	0x100128274 <mech_gpu::sequence::runtime::resident::scalar_row+0x7e8>
100128874:     	ldr	x8, [sp, #0x28]
100128878:     	cmp	x8, x1
10012887c:     	b.hs	0x100128c04 <mech_gpu::sequence::runtime::resident::scalar_row+0x1178>
100128880:     	ldr	w8, [sp, #0x44]
100128884:     	ldr	x13, [sp, #0x20]
100128888:     	cmp	x5, #0x8
10012888c:     	tbz	w8, #0x0, 0x10012889c <mech_gpu::sequence::runtime::resident::scalar_row+0xe10>
100128890:     	b.hs	0x1001288a8 <mech_gpu::sequence::runtime::resident::scalar_row+0xe1c>
100128894:     	mov	x8, #0x0                ; =0
100128898:     	b	0x100128998 <mech_gpu::sequence::runtime::resident::scalar_row+0xf0c>
10012889c:     	b.hs	0x1001289c4 <mech_gpu::sequence::runtime::resident::scalar_row+0xf38>
1001288a0:     	mov	x8, #0x0                ; =0
1001288a4:     	b	0x100128a80 <mech_gpu::sequence::runtime::resident::scalar_row+0xff4>
1001288a8:     	and	x8, x30, #0x38
1001288ac:     	movi.2d	v1, #0000000000000000
1001288b0:     	mov.d	v1[0], v0[0]
1001288b4:     	ldp	q0, q2, [x20]
1001288b8:     	ldp	q3, q4, [x20, #0x20]
1001288bc:     	ldp	q5, q6, [x13]
1001288c0:     	ldp	q7, q16, [x13, #0x20]
1001288c4:     	fdiv.2d	v5, v0, v5
1001288c8:     	fdiv.2d	v0, v2, v6
1001288cc:     	fdiv.2d	v2, v3, v7
1001288d0:     	fdiv.2d	v3, v4, v16
1001288d4:     	fadd.2d	v1, v5, v1
1001288d8:     	cmp	x8, #0x8
1001288dc:     	b.eq	0x100128980 <mech_gpu::sequence::runtime::resident::scalar_row+0xef4>
1001288e0:     	ldp	q4, q5, [x20, #0x40]
1001288e4:     	ldp	q6, q7, [x20, #0x60]
1001288e8:     	ldp	q16, q17, [x13, #0x40]
1001288ec:     	ldp	q18, q19, [x13, #0x60]
1001288f0:     	fdiv.2d	v4, v4, v16
1001288f4:     	fdiv.2d	v5, v5, v17
1001288f8:     	fdiv.2d	v6, v6, v18
1001288fc:     	fdiv.2d	v7, v7, v19
100128900:     	fadd.2d	v1, v4, v1
100128904:     	fadd.2d	v0, v5, v0
100128908:     	fadd.2d	v2, v6, v2
10012890c:     	fadd.2d	v3, v7, v3
100128910:     	cmp	x8, #0x10
100128914:     	b.eq	0x100128980 <mech_gpu::sequence::runtime::resident::scalar_row+0xef4>
100128918:     	ldp	q4, q5, [x20, #0x80]
10012891c:     	ldp	q6, q7, [x20, #0xa0]
100128920:     	ldp	q16, q17, [x13, #0x80]
100128924:     	ldp	q18, q19, [x13, #0xa0]
100128928:     	fdiv.2d	v4, v4, v16
10012892c:     	fdiv.2d	v5, v5, v17
100128930:     	fdiv.2d	v6, v6, v18
100128934:     	fdiv.2d	v7, v7, v19
100128938:     	fadd.2d	v1, v4, v1
10012893c:     	fadd.2d	v0, v5, v0
100128940:     	fadd.2d	v2, v6, v2
100128944:     	fadd.2d	v3, v7, v3
100128948:     	cmp	x8, #0x18
10012894c:     	b.eq	0x100128980 <mech_gpu::sequence::runtime::resident::scalar_row+0xef4>
100128950:     	ldp	q4, q5, [x20, #0xc0]
100128954:     	ldp	q6, q7, [x20, #0xe0]
100128958:     	ldp	q16, q17, [x13, #0xc0]
10012895c:     	ldp	q18, q19, [x13, #0xe0]
100128960:     	fdiv.2d	v4, v4, v16
100128964:     	fdiv.2d	v5, v5, v17
100128968:     	fdiv.2d	v6, v6, v18
10012896c:     	fdiv.2d	v7, v7, v19
100128970:     	fadd.2d	v1, v4, v1
100128974:     	fadd.2d	v0, v5, v0
100128978:     	fadd.2d	v2, v6, v2
10012897c:     	fadd.2d	v3, v7, v3
100128980:     	fadd.2d	v0, v2, v0
100128984:     	fadd.2d	v0, v3, v0
100128988:     	fadd.2d	v0, v0, v1
10012898c:     	faddp.2d	d0, v0
100128990:     	cmp	x30, x8
100128994:     	b.eq	0x100127bd0 <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
100128998:     	lsl	x10, x8, #3
10012899c:     	add	x9, x13, x10
1001289a0:     	add	x10, x20, x10
1001289a4:     	sub	x8, x30, x8
1001289a8:     	ldr	d1, [x10], #0x8
1001289ac:     	ldr	d2, [x9], #0x8
1001289b0:     	fdiv	d1, d1, d2
1001289b4:     	fadd	d0, d1, d0
1001289b8:     	subs	x8, x8, #0x1
1001289bc:     	b.ne	0x1001289a8 <mech_gpu::sequence::runtime::resident::scalar_row+0xf1c>
1001289c0:     	b	0x100127bd0 <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
1001289c4:     	and	x8, x30, #0x38
1001289c8:     	movi.2d	v1, #0000000000000000
1001289cc:     	mov.d	v1[0], v0[0]
1001289d0:     	ldp	q4, q0, [x20]
1001289d4:     	ldp	q2, q3, [x20, #0x20]
1001289d8:     	ldp	q5, q6, [x13]
1001289dc:     	ldp	q7, q16, [x13, #0x20]
1001289e0:     	fmul.2d	v0, v6, v0
1001289e4:     	fmul.2d	v2, v7, v2
1001289e8:     	fmul.2d	v3, v16, v3
1001289ec:     	fmla.2d	v1, v5, v4
1001289f0:     	cmp	x8, #0x8
1001289f4:     	b.eq	0x100128a68 <mech_gpu::sequence::runtime::resident::scalar_row+0xfdc>
1001289f8:     	ldp	q4, q5, [x20, #0x40]
1001289fc:     	ldp	q6, q7, [x20, #0x60]
100128a00:     	ldp	q16, q17, [x13, #0x40]
100128a04:     	ldp	q18, q19, [x13, #0x60]
100128a08:     	fmla.2d	v1, v16, v4
100128a0c:     	fmla.2d	v0, v17, v5
100128a10:     	fmla.2d	v2, v18, v6
100128a14:     	fmla.2d	v3, v19, v7
100128a18:     	cmp	x8, #0x10
100128a1c:     	b.eq	0x100128a68 <mech_gpu::sequence::runtime::resident::scalar_row+0xfdc>
100128a20:     	ldp	q4, q5, [x20, #0x80]
100128a24:     	ldp	q6, q7, [x20, #0xa0]
100128a28:     	ldp	q16, q17, [x13, #0x80]
100128a2c:     	ldp	q18, q19, [x13, #0xa0]
100128a30:     	fmla.2d	v1, v16, v4
100128a34:     	fmla.2d	v0, v17, v5
100128a38:     	fmla.2d	v2, v18, v6
100128a3c:     	fmla.2d	v3, v19, v7
100128a40:     	cmp	x8, #0x18
100128a44:     	b.eq	0x100128a68 <mech_gpu::sequence::runtime::resident::scalar_row+0xfdc>
100128a48:     	ldp	q4, q5, [x20, #0xc0]
100128a4c:     	ldp	q6, q7, [x20, #0xe0]
100128a50:     	ldp	q16, q17, [x13, #0xc0]
100128a54:     	ldp	q18, q19, [x13, #0xe0]
100128a58:     	fmla.2d	v1, v16, v4
100128a5c:     	fmla.2d	v0, v17, v5
100128a60:     	fmla.2d	v2, v18, v6
100128a64:     	fmla.2d	v3, v19, v7
100128a68:     	fadd.2d	v0, v2, v0
100128a6c:     	fadd.2d	v0, v3, v0
100128a70:     	fadd.2d	v0, v0, v1
100128a74:     	faddp.2d	d0, v0
100128a78:     	cmp	x30, x8
100128a7c:     	b.eq	0x100127bd0 <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
100128a80:     	lsl	x10, x8, #3
100128a84:     	add	x9, x13, x10
100128a88:     	add	x10, x20, x10
100128a8c:     	sub	x8, x30, x8
100128a90:     	ldr	d1, [x10], #0x8
100128a94:     	ldr	d2, [x9], #0x8
100128a98:     	fmadd	d0, d2, d1, d0
100128a9c:     	subs	x8, x8, #0x1
100128aa0:     	b.ne	0x100128a90 <mech_gpu::sequence::runtime::resident::scalar_row+0x1004>
100128aa4:     	b	0x100127bd0 <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
100128aa8:     	movi.2d	v8, #0000000000000000
100128aac:     	str	d8, [x25, #0x8]
100128ab0:     	mov	x8, #-0x8000000000000000 ; =-9223372036854775808
100128ab4:     	str	x8, [x25]
100128ab8:     	b	0x100128b70 <mech_gpu::sequence::runtime::resident::scalar_row+0x10e4>
100128abc:     	movi.2d	v0, #0000000000000000
100128ac0:     	ldr	x9, [sp, #0x8]
100128ac4:     	str	d0, [x9, #0x8]
100128ac8:     	mov	x8, #-0x8000000000000000 ; =-9223372036854775808
100128acc:     	str	x8, [x9]
100128ad0:     	b	0x100128b70 <mech_gpu::sequence::runtime::resident::scalar_row+0x10e4>
100128ad4:     	mov	x23, #0x0               ; =0
100128ad8:     	movi.2d	v8, #0000000000000000
100128adc:     	mov	x24, #-0x8000000000000000 ; =-9223372036854775808
100128ae0:     	tbz	w6, #0x0, 0x100128b20 <mech_gpu::sequence::runtime::resident::scalar_row+0x1094>
100128ae4:     	ldr	d9, [x20, x23, lsl #3]
100128ae8:     	stp	x23, x21, [sp, #0x48]
100128aec:     	add	x0, sp, #0x58
100128af0:     	add	x2, sp, #0x48
100128af4:     	mov	x1, x22
100128af8:     	bl	0x100116b20 <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
100128afc:     	ldr	x8, [sp, #0x58]
100128b00:     	cmp	x8, x24
100128b04:     	b.ne	0x100128b5c <mech_gpu::sequence::runtime::resident::scalar_row+0x10d0>
100128b08:     	add	x23, x23, #0x1
100128b0c:     	ldr	d0, [sp, #0x60]
100128b10:     	fmadd	d8, d0, d9, d8
100128b14:     	subs	x19, x19, #0x8
100128b18:     	b.ne	0x100128ae4 <mech_gpu::sequence::runtime::resident::scalar_row+0x1058>
100128b1c:     	b	0x100128aac <mech_gpu::sequence::runtime::resident::scalar_row+0x1020>
100128b20:     	ldr	d9, [x20, x23, lsl #3]
100128b24:     	stp	x21, x23, [sp, #0x48]
100128b28:     	add	x0, sp, #0x58
100128b2c:     	add	x2, sp, #0x48
100128b30:     	mov	x1, x22
100128b34:     	bl	0x100116b20 <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
100128b38:     	ldr	x8, [sp, #0x58]
100128b3c:     	cmp	x8, x24
100128b40:     	b.ne	0x100128b5c <mech_gpu::sequence::runtime::resident::scalar_row+0x10d0>
100128b44:     	add	x23, x23, #0x1
100128b48:     	ldr	d0, [sp, #0x60]
100128b4c:     	fmadd	d8, d0, d9, d8
100128b50:     	subs	x19, x19, #0x8
100128b54:     	b.ne	0x100128b20 <mech_gpu::sequence::runtime::resident::scalar_row+0x1094>
100128b58:     	b	0x100128aac <mech_gpu::sequence::runtime::resident::scalar_row+0x1020>
100128b5c:     	ldr	d0, [sp, #0x60]
100128b60:     	ldr	x9, [sp, #0x68]
100128b64:     	str	x8, [x25]
100128b68:     	str	d0, [x25, #0x8]
100128b6c:     	str	x9, [x25, #0x10]
100128b70:     	ldp	x29, x30, [sp, #0xd0]
100128b74:     	ldp	x20, x19, [sp, #0xc0]
100128b78:     	ldp	x22, x21, [sp, #0xb0]
100128b7c:     	ldp	x24, x23, [sp, #0xa0]
100128b80:     	ldp	x26, x25, [sp, #0x90]
100128b84:     	ldp	x28, x27, [sp, #0x80]
100128b88:     	ldp	d9, d8, [sp, #0x70]
100128b8c:     	add	sp, sp, #0xe0
100128b90:     	ret
100128b94:     	adrp	x0, 0x101884000 <_anon.d4d64e8e11edd2544ed4904eea679557.190+0xd67>
100128b98:     	add	x0, x0, #0x551
100128b9c:     	adrp	x2, 0x101d68000 <_anon.d4d64e8e11edd2544ed4904eea679557.192+0x130>
100128ba0:     	add	x2, x2, #0xff8
100128ba4:     	mov	w1, #0x13               ; =19
100128ba8:     	bl	0x10172cbdc <core::panicking::panic_fmt>
100128bac:     	adrp	x0, 0x101d69000 <_anon.1c98b4fda85a16ce6949c5514dc998cf.196+0x318>
100128bb0:     	add	x0, x0, #0x58
100128bb4:     	bl	0x10172ccd4 <core::panicking::panic_const::panic_const_div_by_zero>
100128bb8:     	adrp	x2, 0x101d68000 <_anon.d4d64e8e11edd2544ed4904eea679557.192+0x130>
100128bbc:     	add	x2, x2, #0xfe0
100128bc0:     	mov	x0, x8
100128bc4:     	bl	0x10172ca6c <core::panicking::panic_bounds_check>
100128bc8:     	adrp	x2, 0x101d69000 <_anon.1c98b4fda85a16ce6949c5514dc998cf.196+0x318>
100128bcc:     	add	x2, x2, #0x40
100128bd0:     	mov	x0, x10
100128bd4:     	mov	x1, x8
100128bd8:     	bl	0x10172ca6c <core::panicking::panic_bounds_check>
100128bdc:     	adrp	x2, 0x101d69000 <_anon.1c98b4fda85a16ce6949c5514dc998cf.196+0x318>
100128be0:     	add	x2, x2, #0x28
100128be4:     	mov	x0, x9
100128be8:     	mov	x1, x8
100128bec:     	bl	0x10172ca6c <core::panicking::panic_bounds_check>
100128bf0:     	adrp	x2, 0x101d69000 <_anon.1c98b4fda85a16ce6949c5514dc998cf.196+0x318>
100128bf4:     	add	x2, x2, #0x10
100128bf8:     	mov	x0, #0x0                ; =0
100128bfc:     	mov	x1, #0x0                ; =0
100128c00:     	bl	0x10172ca6c <core::panicking::panic_bounds_check>
100128c04:     	adrp	x2, 0x101d68000 <_anon.d4d64e8e11edd2544ed4904eea679557.192+0x130>
100128c08:     	add	x2, x2, #0xfc8
100128c0c:     	ldr	x0, [sp, #0x28]
100128c10:     	bl	0x10172ca6c <core::panicking::panic_bounds_check>

0000000100128c14 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone>:
100128c14:     	sub	sp, sp, #0x90
100128c18:     	stp	x28, x27, [sp, #0x30]
100128c1c:     	stp	x26, x25, [sp, #0x40]
100128c20:     	stp	x24, x23, [sp, #0x50]
100128c24:     	stp	x22, x21, [sp, #0x60]
100128c28:     	stp	x20, x19, [sp, #0x70]
100128c2c:     	stp	x29, x30, [sp, #0x80]
100128c30:     	add	x29, sp, #0x80
100128c34:     	ldr	x23, [x1, #0x18]
100128c38:     	lsl	x20, x23, #4
100128c3c:     	lsr	x8, x23, #60
100128c40:     	cmp	x8, #0x0
100128c44:     	mov	x8, #0x7ffffffffffffff8 ; =9223372036854775800
100128c48:     	ccmp	x20, x8, #0x2, eq
100128c4c:     	b.ls	0x100128c60 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x4c>
100128c50:     	mov	x21, #0x0               ; =0
100128c54:     	mov	x0, x21
100128c58:     	mov	x1, x20
100128c5c:     	bl	0x10172c81c <alloc::raw_vec::handle_error>
100128c60:     	mov	x22, x0
100128c64:     	cbz	x20, 0x100128d34 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x120>
100128c68:     	mov	x19, x1
100128c6c:     	bl	0x1000016dc <__rustc::__rust_no_alloc_shim_is_unstable_v2>
100128c70:     	mov	w21, #0x8               ; =8
100128c74:     	mov	x0, x20
100128c78:     	mov	w1, #0x8                ; =8
100128c7c:     	bl	0x1000016cc <__rustc::__rust_alloc>
100128c80:     	cbz	x0, 0x100128c54 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x40>
100128c84:     	mov	x26, x23
100128c88:     	mov	x1, x19
100128c8c:     	stp	x22, x23, [sp, #0x8]
100128c90:     	stp	xzr, xzr, [sp, #0x20]
100128c94:     	str	x0, [sp, #0x18]
100128c98:     	cbz	x23, 0x100128d4c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x138>
100128c9c:     	ldr	x24, [x1]
100128ca0:     	ldr	x8, [x1, #0x10]
100128ca4:     	cmp	x8, x24
100128ca8:     	csel	x9, xzr, x24, lo
100128cac:     	sub	x25, x8, x9
100128cb0:     	sub	x8, x24, x25
100128cb4:     	subs	x22, x23, x8
100128cb8:     	b.hi	0x100128cc4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0xb0>
100128cbc:     	mov	x22, #0x0               ; =0
100128cc0:     	add	x24, x25, x23
100128cc4:     	ldr	x21, [x1, #0x8]
100128cc8:     	sub	x23, x24, x25
100128ccc:     	add	x27, x22, x23
100128cd0:     	cmp	x27, x26
100128cd4:     	b.ls	0x100128d6c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x158>
100128cd8:     	add	x0, sp, #0x10
100128cdc:     	add	x2, x22, x23
100128ce0:     	mov	x1, #0x0                ; =0
100128ce4:     	mov	w3, #0x8                ; =8
100128ce8:     	mov	w4, #0x10               ; =16
100128cec:     	bl	0x1016f7288 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
100128cf0:     	ldp	x19, x20, [sp, #0x20]
100128cf4:     	ldr	x28, [sp, #0x10]
100128cf8:     	sub	x8, x26, x20
100128cfc:     	cmp	x19, x8
100128d00:     	b.ls	0x100128d78 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x164>
100128d04:     	sub	x8, x26, x19
100128d08:     	sub	x9, x20, x8
100128d0c:     	cmp	x8, x9
100128d10:     	b.ls	0x10012901c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x408>
100128d14:     	sub	x10, x28, x26
100128d18:     	cmp	x10, x9
100128d1c:     	b.lo	0x10012901c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x408>
100128d20:     	ldr	x1, [sp, #0x18]
100128d24:     	add	x0, x1, x26, lsl #4
100128d28:     	lsl	x2, x9, #4
100128d2c:     	bl	0x10172d0d0 <_writev+0x10172d0d0>
100128d30:     	b	0x100128d78 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x164>
100128d34:     	mov	x26, #0x0               ; =0
100128d38:     	mov	w0, #0x8                ; =8
100128d3c:     	stp	x22, x26, [sp, #0x8]
100128d40:     	stp	xzr, xzr, [sp, #0x20]
100128d44:     	str	x0, [sp, #0x18]
100128d48:     	cbnz	x23, 0x100128c9c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x88>
100128d4c:     	mov	x25, #0x0               ; =0
100128d50:     	mov	x24, #0x0               ; =0
100128d54:     	mov	x22, #0x0               ; =0
100128d58:     	ldr	x21, [x1, #0x8]
100128d5c:     	sub	x23, x24, x25
100128d60:     	add	x27, x22, x23
100128d64:     	cmp	x27, x26
100128d68:     	b.hi	0x100128cd8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0xc4>
100128d6c:     	mov	x19, #0x0               ; =0
100128d70:     	mov	x20, #0x0               ; =0
100128d74:     	mov	x28, x26
100128d78:     	lsl	x17, x25, #4
100128d7c:     	add	x9, x21, x17
100128d80:     	lsl	x11, x24, #4
100128d84:     	add	x8, x19, x20
100128d88:     	cmp	x8, x28
100128d8c:     	csel	x10, xzr, x28, lo
100128d90:     	sub	x13, x8, x10
100128d94:     	sub	x14, x28, x13
100128d98:     	cmp	x14, x27
100128d9c:     	b.hs	0x100128e14 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x200>
100128da0:     	lsl	x8, x22, #4
100128da4:     	cmp	x28, x13
100128da8:     	b.ne	0x100128e24 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x210>
100128dac:     	mov	x12, #0x0               ; =0
100128db0:     	add	x10, x21, x11
100128db4:     	cmp	x9, x10
100128db8:     	ldr	x17, [sp, #0x8]
100128dbc:     	b.eq	0x100128e98 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x284>
100128dc0:     	sub	x10, x10, x9
100128dc4:     	lsr	x11, x10, #4
100128dc8:     	add	x10, x12, x11
100128dcc:     	ldr	x12, [sp, #0x18]
100128dd0:     	mov	w13, #0x1               ; =1
100128dd4:     	mov	x14, x11
100128dd8:     	ldr	q0, [x9], #0x10
100128ddc:     	fmov	x15, d0
100128de0:     	ldadd	x13, x15, [x15]
100128de4:     	tbnz	x15, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128de8:     	mov.d	x15, v0[1]
100128dec:     	ldadd	x13, x15, [x15]
100128df0:     	tbnz	x15, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128df4:     	str	q0, [x12], #0x10
100128df8:     	subs	x14, x14, #0x1
100128dfc:     	b.ne	0x100128dd8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x1c4>
100128e00:     	mov	x9, x21
100128e04:     	add	x8, x21, x8
100128e08:     	cmp	x21, x8
100128e0c:     	b.ne	0x100128eb0 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x29c>
100128e10:     	b	0x100128fe8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d4>
100128e14:     	cmp	x17, x11
100128e18:     	b.ne	0x100128ef4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x2e0>
100128e1c:     	mov	x23, #0x0               ; =0
100128e20:     	b	0x100128f2c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x318>
100128e24:     	mov	x15, #0x0               ; =0
100128e28:     	mov	x12, #0x0               ; =0
100128e2c:     	sub	x10, x11, x17
100128e30:     	lsr	x10, x10, #4
100128e34:     	ldr	x16, [sp, #0x18]
100128e38:     	lsl	x13, x13, #4
100128e3c:     	add	x16, x16, x13
100128e40:     	sub	x17, x17, x11
100128e44:     	mov	w0, #0x1                ; =1
100128e48:     	cmp	x17, x15
100128e4c:     	b.eq	0x100128f7c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x368>
100128e50:     	ldr	q0, [x9, x12, lsl #4]
100128e54:     	fmov	x1, d0
100128e58:     	ldadd	x0, x1, [x1]
100128e5c:     	tbnz	x1, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128e60:     	mov.d	x1, v0[1]
100128e64:     	ldadd	x0, x1, [x1]
100128e68:     	tbnz	x1, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128e6c:     	str	q0, [x16, x12, lsl #4]
100128e70:     	add	x12, x12, #0x1
100128e74:     	add	x13, x13, #0x10
100128e78:     	sub	x15, x15, #0x10
100128e7c:     	cmp	x14, x12
100128e80:     	b.ne	0x100128e48 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x234>
100128e84:     	sub	x9, x9, x15
100128e88:     	add	x10, x21, x11
100128e8c:     	cmp	x9, x10
100128e90:     	ldr	x17, [sp, #0x8]
100128e94:     	b.ne	0x100128dc0 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x1ac>
100128e98:     	mov	x11, #0x0               ; =0
100128e9c:     	mov	x9, x21
100128ea0:     	mov	x10, x12
100128ea4:     	add	x8, x21, x8
100128ea8:     	cmp	x21, x8
100128eac:     	b.eq	0x100128fe8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d4>
100128eb0:     	sub	x8, x8, x9
100128eb4:     	lsr	x8, x8, #4
100128eb8:     	add	x23, x10, x8
100128ebc:     	ldr	x10, [sp, #0x18]
100128ec0:     	add	x10, x10, x11, lsl #4
100128ec4:     	mov	w11, #0x1               ; =1
100128ec8:     	ldr	q0, [x9], #0x10
100128ecc:     	fmov	x12, d0
100128ed0:     	ldadd	x11, x12, [x12]
100128ed4:     	tbnz	x12, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128ed8:     	mov.d	x12, v0[1]
100128edc:     	ldadd	x11, x12, [x12]
100128ee0:     	tbnz	x12, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128ee4:     	str	q0, [x10], #0x10
100128ee8:     	subs	x8, x8, #0x1
100128eec:     	b.ne	0x100128ec8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x2b4>
100128ef0:     	b	0x100128fec <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d8>
100128ef4:     	ldr	x8, [sp, #0x18]
100128ef8:     	add	x8, x8, x13, lsl #4
100128efc:     	mov	w10, #0x1               ; =1
100128f00:     	mov	x11, x23
100128f04:     	ldr	q0, [x9], #0x10
100128f08:     	fmov	x12, d0
100128f0c:     	ldadd	x10, x12, [x12]
100128f10:     	tbnz	x12, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128f14:     	mov.d	x12, v0[1]
100128f18:     	ldadd	x10, x12, [x12]
100128f1c:     	tbnz	x12, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128f20:     	str	q0, [x8], #0x10
100128f24:     	subs	x11, x11, #0x1
100128f28:     	b.ne	0x100128f04 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x2f0>
100128f2c:     	cbz	x22, 0x100128f74 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x360>
100128f30:     	add	x8, x22, x23
100128f34:     	ldr	x9, [sp, #0x18]
100128f38:     	lsl	x10, x23, #4
100128f3c:     	add	x10, x10, x13, lsl #4
100128f40:     	add	x9, x9, x10
100128f44:     	mov	w10, #0x1               ; =1
100128f48:     	ldr	q0, [x21], #0x10
100128f4c:     	fmov	x11, d0
100128f50:     	ldadd	x10, x11, [x11]
100128f54:     	tbnz	x11, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128f58:     	mov.d	x11, v0[1]
100128f5c:     	ldadd	x10, x11, [x11]
100128f60:     	tbnz	x11, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128f64:     	str	q0, [x9], #0x10
100128f68:     	subs	x22, x22, #0x1
100128f6c:     	b.ne	0x100128f48 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x334>
100128f70:     	mov	x23, x8
100128f74:     	ldr	x17, [sp, #0x8]
100128f78:     	b	0x100128fec <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d8>
100128f7c:     	mov	x11, #0x0               ; =0
100128f80:     	add	x23, x22, x10
100128f84:     	ldr	x9, [sp, #0x18]
100128f88:     	add	x13, x9, x13
100128f8c:     	sub	x12, x14, x12
100128f90:     	mov	w14, #0x1               ; =1
100128f94:     	mov	x15, x8
100128f98:     	mov	x9, x21
100128f9c:     	ldr	x17, [sp, #0x8]
100128fa0:     	cbz	x15, 0x100128fec <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d8>
100128fa4:     	ldr	q0, [x9], #0x10
100128fa8:     	fmov	x16, d0
100128fac:     	ldadd	x14, x16, [x16]
100128fb0:     	tbnz	x16, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128fb4:     	mov.d	x16, v0[1]
100128fb8:     	ldadd	x14, x16, [x16]
100128fbc:     	tbnz	x16, #0x3f, 0x100129040 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
100128fc0:     	str	q0, [x13, x11, lsl #4]
100128fc4:     	add	x10, x10, #0x1
100128fc8:     	sub	x15, x15, #0x10
100128fcc:     	add	x11, x11, #0x1
100128fd0:     	cmp	x12, x11
100128fd4:     	b.ne	0x100128fa0 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x38c>
100128fd8:     	mov	x11, #0x0               ; =0
100128fdc:     	add	x8, x21, x8
100128fe0:     	cmp	x9, x8
100128fe4:     	b.ne	0x100128eb0 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x29c>
100128fe8:     	mov	x23, x10
100128fec:     	add	x8, x20, x23
100128ff0:     	str	x8, [sp, #0x28]
100128ff4:     	ldp	q0, q1, [sp, #0x10]
100128ff8:     	stp	q0, q1, [x17]
100128ffc:     	ldp	x29, x30, [sp, #0x80]
100129000:     	ldp	x20, x19, [sp, #0x70]
100129004:     	ldp	x22, x21, [sp, #0x60]
100129008:     	ldp	x24, x23, [sp, #0x50]
10012900c:     	ldp	x26, x25, [sp, #0x40]
100129010:     	ldp	x28, x27, [sp, #0x30]
100129014:     	add	sp, sp, #0x90
100129018:     	ret
10012901c:     	sub	x26, x28, x8
100129020:     	ldr	x9, [sp, #0x18]
100129024:     	add	x1, x9, x19, lsl #4
100129028:     	add	x0, x9, x26, lsl #4
10012902c:     	lsl	x2, x8, #4
100129030:     	bl	0x10172d0dc <_writev+0x10172d0dc>
100129034:     	str	x26, [sp, #0x20]
100129038:     	mov	x19, x26
10012903c:     	b	0x100128d78 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x164>
100129040:     	brk	#0x1
100129044:     	mov	x19, x0
100129048:     	add	x0, sp, #0x10
10012904c:     	bl	0x10010eba8 <core::ptr::drop_in_place::<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record>>>
100129050:     	mov	x0, x19
100129054:     	bl	0x10172ce00 <_writev+0x10172ce00>
100129058:     	bl	0x10172ca54 <core::panicking::panic_in_cleanup>

000000010012905c <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop>:
10012905c:     	stp	x24, x23, [sp, #-0x40]!
100129060:     	stp	x22, x21, [sp, #0x10]
100129064:     	stp	x20, x19, [sp, #0x20]
100129068:     	stp	x29, x30, [sp, #0x30]
10012906c:     	add	x29, sp, #0x30
100129070:     	mov	x20, x0
100129074:     	ldr	x8, [x0, #0x20]
100129078:     	cbz	x8, 0x1001290b0 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x54>
10012907c:     	cmp	x8, #0x2
100129080:     	b.eq	0x100129100 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xa4>
100129084:     	cmp	x8, #0x3
100129088:     	b.ne	0x100129114 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xb8>
10012908c:     	ldp	x19, x8, [x20, #0x8]
100129090:     	add	x23, x8, #0x1
100129094:     	mov	x0, x19
100129098:     	subs	x23, x23, #0x1
10012909c:     	b.eq	0x1001290d4 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x78>
1001290a0:     	add	x22, x0, #0x38
1001290a4:     	bl	0x1000233c4 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
1001290a8:     	mov	x0, x22
1001290ac:     	b	0x100129098 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x3c>
1001290b0:     	ldp	x19, x8, [x20, #0x8]
1001290b4:     	add	x23, x8, #0x1
1001290b8:     	mov	x0, x19
1001290bc:     	subs	x23, x23, #0x1
1001290c0:     	b.eq	0x1001290d4 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x78>
1001290c4:     	add	x22, x0, #0x38
1001290c8:     	bl	0x1000233c4 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
1001290cc:     	mov	x0, x22
1001290d0:     	b	0x1001290bc <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x60>
1001290d4:     	ldr	x8, [x20]
1001290d8:     	cbz	x8, 0x100129100 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xa4>
1001290dc:     	lsl	x9, x8, #6
1001290e0:     	sub	x1, x9, x8, lsl #3
1001290e4:     	mov	x0, x19
1001290e8:     	mov	w2, #0x8                ; =8
1001290ec:     	ldp	x29, x30, [sp, #0x30]
1001290f0:     	ldp	x20, x19, [sp, #0x20]
1001290f4:     	ldp	x22, x21, [sp, #0x10]
1001290f8:     	ldp	x24, x23, [sp], #0x40
1001290fc:     	b	0x1000016d0 <__rustc::__rust_dealloc>
100129100:     	ldp	x29, x30, [sp, #0x30]
100129104:     	ldp	x20, x19, [sp, #0x20]
100129108:     	ldp	x22, x21, [sp, #0x10]
10012910c:     	ldp	x24, x23, [sp], #0x40
100129110:     	ret
100129114:     	adrp	x0, 0x101884000 <_anon.d4d64e8e11edd2544ed4904eea679557.190+0xd67>
100129118:     	add	x0, x0, #0x6b7
10012911c:     	adrp	x2, 0x101d68000 <_anon.d4d64e8e11edd2544ed4904eea679557.192+0x130>
100129120:     	add	x2, x2, #0x828
100129124:     	mov	w1, #0x79               ; =121
100129128:     	bl	0x10172cbdc <core::panicking::panic_fmt>
10012912c:     	mov	x21, x0
100129130:     	subs	x23, x23, #0x1
100129134:     	b.eq	0x100129170 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x114>
100129138:     	add	x24, x22, #0x38
10012913c:     	mov	x0, x22
100129140:     	bl	0x1000233c4 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
100129144:     	mov	x22, x24
100129148:     	b	0x100129130 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xd4>
10012914c:     	bl	0x10172ca54 <core::panicking::panic_in_cleanup>
100129150:     	mov	x21, x0
100129154:     	subs	x23, x23, #0x1
100129158:     	b.eq	0x100129170 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x114>
10012915c:     	add	x24, x22, #0x38
100129160:     	mov	x0, x22
100129164:     	bl	0x1000233c4 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
100129168:     	mov	x22, x24
10012916c:     	b	0x100129154 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xf8>
100129170:     	ldr	x8, [x20]
100129174:     	cbz	x8, 0x10012918c <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x130>
100129178:     	lsl	x9, x8, #6
10012917c:     	sub	x1, x9, x8, lsl #3
100129180:     	mov	x0, x19
100129184:     	mov	w2, #0x8                ; =8
100129188:     	bl	0x1000016d0 <__rustc::__rust_dealloc>
10012918c:     	mov	x0, x21
100129190:     	bl	0x10172ce00 <_writev+0x10172ce00>
100129194:     	bl	0x10172ca54 <core::panicking::panic_in_cleanup>

