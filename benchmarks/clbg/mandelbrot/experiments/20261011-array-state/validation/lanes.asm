
benchmarks/clbg/mandelbrot/experiments/20261011-array-state/validation/lanes.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000000000 <_mech_map_bits>:
       0: d503237f     	pacibsp
       4: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
       8: 910003fd     	mov	x29, sp
       c: 6dbf3fee     	stp	d14, d15, [sp, #-0x10]!
      10: 6dbf37ec     	stp	d12, d13, [sp, #-0x10]!
      14: 6dbf2fea     	stp	d10, d11, [sp, #-0x10]!
      18: 6dbf27e8     	stp	d8, d9, [sp, #-0x10]!
      1c: fd400001     	ldr	d1, [x0]
      20: fd400403     	ldr	d3, [x0, #0x8]
      24: fd400802     	ldr	d2, [x0, #0x10]
      28: fd400c13     	ldr	d19, [x0, #0x18]
      2c: fd401004     	ldr	d4, [x0, #0x20]
      30: fd401412     	ldr	d18, [x0, #0x28]
      34: fd401805     	ldr	d5, [x0, #0x30]
      38: fd401c07     	ldr	d7, [x0, #0x38]
      3c: fd402006     	ldr	d6, [x0, #0x40]
      40: fd402416     	ldr	d22, [x0, #0x48]
      44: 4f00e411     	movi.16b	v17, #0x0
      48: 0f000410     	movi.2s	v16, #0x0
      4c: 6f00f600     	fmov.2d	v0, #4.00000000
      50: 1e604063     	fmov	d3, d3
      54: 6e180443     	mov.d	v3[1], v2[0]
      58: 1e601002     	fmov	d2, #2.00000000
      5c: 1e611842     	fdiv	d2, d2, d1
      60: 4e080454     	dup.2d	v20, v2[0]
      64: 6e74dc61     	fmul.2d	v1, v3, v20
      68: 6f03f715     	fmov.2d	v21, #1.50000000
      6c: 4ef5d421     	fsub.2d	v1, v1, v21
      70: 1e620ac2     	fmul	d2, d22, d2
      74: 1e6e1003     	fmov	d3, #1.00000000
      78: 1e633842     	fsub	d2, d2, d3
      7c: 4e080442     	dup.2d	v2, v2[0]
      80: 6f03f603     	fmov.2d	v3, #1.00000000
      84: 1e604273     	fmov	d19, d19
      88: 6e180493     	mov.d	v19[1], v4[0]
      8c: 6e74de64     	fmul.2d	v4, v19, v20
      90: 4ef5d484     	fsub.2d	v4, v4, v21
      94: 1e604252     	fmov	d18, d18
      98: 6e1804b2     	mov.d	v18[1], v5[0]
      9c: 6e74de45     	fmul.2d	v5, v18, v20
      a0: 4ef5d4a5     	fsub.2d	v5, v5, v21
      a4: 1e6040e7     	fmov	d7, d7
      a8: 6e1804c7     	mov.d	v7[1], v6[0]
      ac: 6e74dce6     	fmul.2d	v6, v7, v20
      b0: 4ef5d4c6     	fsub.2d	v6, v6, v21
      b4: 4eb01e07     	mov.16b	v7, v16
      b8: 4eb01e12     	mov.16b	v18, v16
      bc: 4eb01e13     	mov.16b	v19, v16
      c0: 4eb01e14     	mov.16b	v20, v16
      c4: 4eb01e15     	mov.16b	v21, v16
      c8: 4eb01e16     	mov.16b	v22, v16
      cc: 4eb01e17     	mov.16b	v23, v16
      d0: 4eb11e38     	mov.16b	v24, v17
      d4: 4eb11e39     	mov.16b	v25, v17
      d8: 4eb11e3a     	mov.16b	v26, v17
      dc: 4eb11e3b     	mov.16b	v27, v17
      e0: 4eb11e3c     	mov.16b	v28, v17
      e4: 4eb11e3d     	mov.16b	v29, v17
      e8: 4eb11e3e     	mov.16b	v30, v17
      ec: d2e80920     	mov	x0, #0x4049000000000000 ; =4632233691727265792
      f0: 9e67001f     	fmov	d31, x0
      f4: 92800000     	mov	x0, #-0x1               ; =-1
      f8: d2800001     	mov	x1, #0x0                ; =0
      fc: 1e7f2200     	fcmp	d16, d31
     100: 9a81a000     	csel	x0, x0, x1, ge
     104: 6e71de3f     	fmul.2d	v31, v17, v17
     108: 6e7edfc8     	fmul.2d	v8, v30, v30
     10c: 4ec81bed     	uzp1.2d	v13, v31, v8
     110: 4ec85bee     	uzp2.2d	v14, v31, v8
     114: 4e6ed5bf     	fadd.2d	v31, v13, v14
     118: 6e7fe41f     	fcmge.2d	v31, v0, v31
     11c: 4e083fe1     	mov.d	x1, v31[0]
     120: 4e183fe2     	mov.d	x2, v31[1]
     124: aa020024     	orr	x4, x1, x2
     128: 6e7adf5f     	fmul.2d	v31, v26, v26
     12c: 6e7ddfa8     	fmul.2d	v8, v29, v29
     130: 4ec81beb     	uzp1.2d	v11, v31, v8
     134: 4ec85bec     	uzp2.2d	v12, v31, v8
     138: 4e6cd57f     	fadd.2d	v31, v11, v12
     13c: 6e7fe41f     	fcmge.2d	v31, v0, v31
     140: 4e083fe3     	mov.d	x3, v31[0]
     144: aa030085     	orr	x5, x4, x3
     148: 4e183fe4     	mov.d	x4, v31[1]
     14c: aa0400a6     	orr	x6, x5, x4
     150: 6e79df3f     	fmul.2d	v31, v25, v25
     154: 6e7cdf88     	fmul.2d	v8, v28, v28
     158: 4ec81be9     	uzp1.2d	v9, v31, v8
     15c: 4ec85bea     	uzp2.2d	v10, v31, v8
     160: 4e6ad53f     	fadd.2d	v31, v9, v10
     164: 6e7fe41f     	fcmge.2d	v31, v0, v31
     168: 4e083fe5     	mov.d	x5, v31[0]
     16c: aa0500c7     	orr	x7, x6, x5
     170: 4e183fe6     	mov.d	x6, v31[1]
     174: aa0600e8     	orr	x8, x7, x6
     178: 6e78df08     	fmul.2d	v8, v24, v24
     17c: 6e7bdf6f     	fmul.2d	v15, v27, v27
     180: 4ecf191f     	uzp1.2d	v31, v8, v15
     184: 4ecf5908     	uzp2.2d	v8, v8, v15
     188: 4e68d7ef     	fadd.2d	v15, v31, v8
     18c: 6e6fe40f     	fcmge.2d	v15, v0, v15
     190: 4e083de7     	mov.d	x7, v15[0]
     194: aa070109     	orr	x9, x8, x7
     198: 4e183de8     	mov.d	x8, v15[1]
     19c: aa080129     	orr	x9, x9, x8
     1a0: aa290000     	orn	x0, x0, x9
     1a4: f100001f     	cmp	x0, #0x0
     1a8: 540006c1     	b.ne	0x280 <_mech_map_bits+0x280>
     1ac: 4eeed5ad     	fsub.2d	v13, v13, v14
     1b0: 4e61d5ad     	fadd.2d	v13, v13, v1
     1b4: 4ede1a2e     	uzp1.2d	v14, v17, v30
     1b8: 4e6ed5ce     	fadd.2d	v14, v14, v14
     1bc: 4ede5a31     	uzp2.2d	v17, v17, v30
     1c0: 6e71ddd1     	fmul.2d	v17, v14, v17
     1c4: 4e62d63e     	fadd.2d	v30, v17, v2
     1c8: 4ede19b1     	uzp1.2d	v17, v13, v30
     1cc: 1e604210     	fmov	d16, d16
     1d0: 6e1806f0     	mov.d	v16[1], v23[0]
     1d4: 4e63d610     	fadd.2d	v16, v16, v3
     1d8: 4ede59be     	uzp2.2d	v30, v13, v30
     1dc: 5e180617     	mov	d23, v16[1]
     1e0: 4eecd56b     	fsub.2d	v11, v11, v12
     1e4: 4e64d56b     	fadd.2d	v11, v11, v4
     1e8: 4edd1b4c     	uzp1.2d	v12, v26, v29
     1ec: 4e6cd58c     	fadd.2d	v12, v12, v12
     1f0: 4edd5b5a     	uzp2.2d	v26, v26, v29
     1f4: 6e7add9a     	fmul.2d	v26, v12, v26
     1f8: 4e62d75d     	fadd.2d	v29, v26, v2
     1fc: 4edd197a     	uzp1.2d	v26, v11, v29
     200: 1e6042d6     	fmov	d22, d22
     204: 6e1806b6     	mov.d	v22[1], v21[0]
     208: 4e63d6d6     	fadd.2d	v22, v22, v3
     20c: 4edd597d     	uzp2.2d	v29, v11, v29
     210: 5e1806d5     	mov	d21, v22[1]
     214: 4eead529     	fsub.2d	v9, v9, v10
     218: 4e65d529     	fadd.2d	v9, v9, v5
     21c: 4edc1b2a     	uzp1.2d	v10, v25, v28
     220: 4e6ad54a     	fadd.2d	v10, v10, v10
     224: 4edc5b39     	uzp2.2d	v25, v25, v28
     228: 6e79dd59     	fmul.2d	v25, v10, v25
     22c: 4e62d73c     	fadd.2d	v28, v25, v2
     230: 4edc1939     	uzp1.2d	v25, v9, v28
     234: 1e604294     	fmov	d20, d20
     238: 6e180674     	mov.d	v20[1], v19[0]
     23c: 4e63d694     	fadd.2d	v20, v20, v3
     240: 4edc593c     	uzp2.2d	v28, v9, v28
     244: 5e180693     	mov	d19, v20[1]
     248: 4ee8d7ff     	fsub.2d	v31, v31, v8
     24c: 4e66d7ff     	fadd.2d	v31, v31, v6
     250: 4edb1b08     	uzp1.2d	v8, v24, v27
     254: 4e68d508     	fadd.2d	v8, v8, v8
     258: 4edb5b18     	uzp2.2d	v24, v24, v27
     25c: 6e78dd18     	fmul.2d	v24, v8, v24
     260: 4e62d71b     	fadd.2d	v27, v24, v2
     264: 4edb1bf8     	uzp1.2d	v24, v31, v27
     268: 1e604252     	fmov	d18, d18
     26c: 6e1804f2     	mov.d	v18[1], v7[0]
     270: 4e63d652     	fadd.2d	v18, v18, v3
     274: 4edb5bfb     	uzp2.2d	v27, v31, v27
     278: 5e180647     	mov	d7, v18[1]
     27c: 17ffff9c     	b	0xec <_mech_map_bits+0xec>
     280: 92790020     	and	x0, x1, #0x80
     284: 927a0041     	and	x1, x2, #0x40
     288: aa010000     	orr	x0, x0, x1
     28c: 927b0061     	and	x1, x3, #0x20
     290: aa010000     	orr	x0, x0, x1
     294: 927c0081     	and	x1, x4, #0x10
     298: aa010000     	orr	x0, x0, x1
     29c: 927d00a1     	and	x1, x5, #0x8
     2a0: aa010000     	orr	x0, x0, x1
     2a4: 927e00c1     	and	x1, x6, #0x4
     2a8: aa010000     	orr	x0, x0, x1
     2ac: 927f00e1     	and	x1, x7, #0x2
     2b0: aa010000     	orr	x0, x0, x1
     2b4: 92400101     	and	x1, x8, #0x1
     2b8: aa010000     	orr	x0, x0, x1
     2bc: 6cc127e8     	ldp	d8, d9, [sp], #0x10
     2c0: 6cc12fea     	ldp	d10, d11, [sp], #0x10
     2c4: 6cc137ec     	ldp	d12, d13, [sp], #0x10
     2c8: 6cc13fee     	ldp	d14, d15, [sp], #0x10
     2cc: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     2d0: d65f0fff     	retab

00000000000002d4 <_mech_grid_row>:
     2d4: d503237f     	pacibsp
     2d8: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     2dc: 910003fd     	mov	x29, sp
     2e0: a9bf5bf5     	stp	x21, x22, [sp, #-0x10]!
     2e4: a9bf53f3     	stp	x19, x20, [sp, #-0x10]!
     2e8: d10143ff     	sub	sp, sp, #0x50
     2ec: aa0203f5     	mov	x21, x2
     2f0: 9e630000     	ucvtf	d0, x0
     2f4: 910003f3     	mov	x19, sp
     2f8: fc0003e0     	stur	d0, [sp]
     2fc: 9e630020     	ucvtf	d0, x1
     300: fc0483e0     	stur	d0, [sp, #0x48]
     304: d2800014     	mov	x20, #0x0               ; =0
     308: f100001f     	cmp	x0, #0x0
     30c: aa0003f6     	mov	x22, x0
     310: 540005a0     	b.eq	0x3c4 <_mech_grid_row+0xf0>
     314: 9e630280     	ucvtf	d0, x20
     318: fc0083e0     	stur	d0, [sp, #0x8]
     31c: 91000680     	add	x0, x20, #0x1
     320: 9e630000     	ucvtf	d0, x0
     324: fc0103e0     	stur	d0, [sp, #0x10]
     328: 91000a80     	add	x0, x20, #0x2
     32c: 9e630000     	ucvtf	d0, x0
     330: fc0183e0     	stur	d0, [sp, #0x18]
     334: 91000e80     	add	x0, x20, #0x3
     338: 9e630000     	ucvtf	d0, x0
     33c: fc0203e0     	stur	d0, [sp, #0x20]
     340: 91001280     	add	x0, x20, #0x4
     344: 9e630000     	ucvtf	d0, x0
     348: fc0283e0     	stur	d0, [sp, #0x28]
     34c: 91001680     	add	x0, x20, #0x5
     350: 9e630000     	ucvtf	d0, x0
     354: fc0303e0     	stur	d0, [sp, #0x30]
     358: 91001a80     	add	x0, x20, #0x6
     35c: 9e630000     	ucvtf	d0, x0
     360: fc0383e0     	stur	d0, [sp, #0x38]
     364: 91001e80     	add	x0, x20, #0x7
     368: 9e630000     	ucvtf	d0, x0
     36c: fc0403e0     	stur	d0, [sp, #0x40]
     370: aa1303e0     	mov	x0, x19
     374: 94000000     	bl	0x374 <_mech_grid_row+0xa0>
     378: aa1603e8     	mov	x8, x22
     37c: cb140101     	sub	x1, x8, x20
     380: d2800102     	mov	x2, #0x8                ; =8
     384: cb010042     	sub	x2, x2, x1
     388: d2800003     	mov	x3, #0x0                ; =0
     38c: f100203f     	cmp	x1, #0x8
     390: 9a833041     	csel	x1, x2, x3, lo
     394: d2801fe2     	mov	x2, #0xff               ; =255
     398: 9ac12041     	lsl	x1, x2, x1
     39c: 8a010000     	and	x0, x0, x1
     3a0: d343fe81     	lsr	x1, x20, #3
     3a4: aa1503e2     	mov	x2, x21
     3a8: 38216840     	strb	w0, [x2, x1]
     3ac: 91002294     	add	x20, x20, #0x8
     3b0: eb08029f     	cmp	x20, x8
     3b4: 54000082     	b.hs	0x3c4 <_mech_grid_row+0xf0>
     3b8: aa0203f5     	mov	x21, x2
     3bc: aa0803f6     	mov	x22, x8
     3c0: 17ffffd5     	b	0x314 <_mech_grid_row+0x40>
     3c4: 910143ff     	add	sp, sp, #0x50
     3c8: a8c153f3     	ldp	x19, x20, [sp], #0x10
     3cc: a8c15bf5     	ldp	x21, x22, [sp], #0x10
     3d0: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     3d4: d65f0fff     	retab
