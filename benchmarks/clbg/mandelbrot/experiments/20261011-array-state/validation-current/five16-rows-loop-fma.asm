
/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/mandelbrot/experiments/20261011-array-state/validation-current/five16-rows-loop-fma.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000000000 <_mech_map_bits>:
       0: fd400005     	ldr	d5, [x0]
       4: fd400417     	ldr	d23, [x0, #0x8]
       8: fd400806     	ldr	d6, [x0, #0x10]
       c: fd400c16     	ldr	d22, [x0, #0x18]
      10: fd401007     	ldr	d7, [x0, #0x20]
      14: fd401414     	ldr	d20, [x0, #0x28]
      18: fd401811     	ldr	d17, [x0, #0x30]
      1c: fd401c13     	ldr	d19, [x0, #0x38]
      20: fd402012     	ldr	d18, [x0, #0x40]
      24: fd402419     	ldr	d25, [x0, #0x48]
      28: 4f00e410     	movi.16b	v16, #0x0
      2c: 0f000415     	movi.2s	v21, #0x0
      30: 6f00f604     	fmov.2d	v4, #4.00000000
      34: 1e6042e3     	fmov	d3, d23
      38: 6e1804c3     	mov.d	v3[1], v6[0]
      3c: 1e601006     	fmov	d6, #2.00000000
      40: 1e6518c6     	fdiv	d6, d6, d5
      44: 4e0804d7     	dup.2d	v23, v6[0]
      48: 6e77dc65     	fmul.2d	v5, v3, v23
      4c: 6f03f718     	fmov.2d	v24, #1.50000000
      50: 4ef8d4a5     	fsub.2d	v5, v5, v24
      54: 1e660b26     	fmul	d6, d25, d6
      58: 1e6e1019     	fmov	d25, #1.00000000
      5c: 1e7938c6     	fsub	d6, d6, d25
      60: 4e0804c6     	dup.2d	v6, v6[0]
      64: 1e6042c3     	fmov	d3, d22
      68: 6e1804e3     	mov.d	v3[1], v7[0]
      6c: 6e77dc67     	fmul.2d	v7, v3, v23
      70: 4ef8d4e7     	fsub.2d	v7, v7, v24
      74: 1e604283     	fmov	d3, d20
      78: 6e180623     	mov.d	v3[1], v17[0]
      7c: 6e77dc71     	fmul.2d	v17, v3, v23
      80: 4ef8d631     	fsub.2d	v17, v17, v24
      84: 1e604263     	fmov	d3, d19
      88: 6e180643     	mov.d	v3[1], v18[0]
      8c: 6e77dc72     	fmul.2d	v18, v3, v23
      90: 4ef8d652     	fsub.2d	v18, v18, v24
      94: 4eb01e00     	mov.16b	v0, v16
      98: 4eb01e01     	mov.16b	v1, v16
      9c: 4eb01e02     	mov.16b	v2, v16
      a0: 4eb01e03     	mov.16b	v3, v16
      a4: 4eb01e13     	mov.16b	v19, v16
      a8: 4eb01e17     	mov.16b	v23, v16
      ac: 4eb01e18     	mov.16b	v24, v16
      b0: d2e80924     	mov	x4, #0x4049000000000000 ; =4632233691727265792
      b4: 9e670094     	fmov	d20, x4
      b8: 92800005     	mov	x5, #-0x1               ; =-1
      bc: d2800006     	mov	x6, #0x0                ; =0
      c0: 1e7422a0     	fcmp	d21, d20
      c4: 9a86a0a6     	csel	x6, x5, x6, ge
      c8: 6e70de16     	fmul.2d	v22, v16, v16
      cc: 4eb61ed4     	mov.16b	v20, v22
      d0: 4e78cf14     	fmla.2d	v20, v24, v24
      d4: 6e74e494     	fcmge.2d	v20, v4, v20
      d8: 4e083e87     	mov.d	x7, v20[0]
      dc: 4e183e88     	mov.d	x8, v20[1]
      e0: aa0800ea     	orr	x10, x7, x8
      e4: 6e62dc5a     	fmul.2d	v26, v2, v2
      e8: 4eba1f54     	mov.16b	v20, v26
      ec: 4e77cef4     	fmla.2d	v20, v23, v23
      f0: 6e74e494     	fcmge.2d	v20, v4, v20
      f4: 4e083e89     	mov.d	x9, v20[0]
      f8: aa09014b     	orr	x11, x10, x9
      fc: 4e183e8a     	mov.d	x10, v20[1]
     100: aa0a016c     	orr	x12, x11, x10
     104: 6e61dc39     	fmul.2d	v25, v1, v1
     108: 4eb91f34     	mov.16b	v20, v25
     10c: 4e73ce74     	fmla.2d	v20, v19, v19
     110: 6e74e494     	fcmge.2d	v20, v4, v20
     114: 4e083e8b     	mov.d	x11, v20[0]
     118: aa0b018d     	orr	x13, x12, x11
     11c: 4e183e8c     	mov.d	x12, v20[1]
     120: aa0c01ae     	orr	x14, x13, x12
     124: 6e60dc14     	fmul.2d	v20, v0, v0
     128: 4eb41e9b     	mov.16b	v27, v20
     12c: 4e63cc7b     	fmla.2d	v27, v3, v3
     130: 6e7be49b     	fcmge.2d	v27, v4, v27
     134: 4e083f6d     	mov.d	x13, v27[0]
     138: aa0d01cf     	orr	x15, x14, x13
     13c: 4e183f6e     	mov.d	x14, v27[1]
     140: aa0e01ef     	orr	x15, x15, x14
     144: aa2f00c6     	orn	x6, x6, x15
     148: f10000df     	cmp	x6, #0x0
     14c: 54000f01     	b.ne	0x32c <_mech_map_bits+0x32c>
     150: 4ef8cf16     	fmls.2d	v22, v24, v24
     154: 4e65d6d6     	fadd.2d	v22, v22, v5
     158: 4e76d6db     	fadd.2d	v27, v22, v22
     15c: 4e70d610     	fadd.2d	v16, v16, v16
     160: 4ea61cdc     	mov.16b	v28, v6
     164: 4e78ce1c     	fmla.2d	v28, v16, v24
     168: 4ea61cd0     	mov.16b	v16, v6
     16c: 4e7ccf70     	fmla.2d	v16, v27, v28
     170: 6e76ded6     	fmul.2d	v22, v22, v22
     174: 4efccf96     	fmls.2d	v22, v28, v28
     178: 4e65d6d6     	fadd.2d	v22, v22, v5
     17c: 6e76ded8     	fmul.2d	v24, v22, v22
     180: 4ef0ce18     	fmls.2d	v24, v16, v16
     184: 4e65d718     	fadd.2d	v24, v24, v5
     188: 4e78d71b     	fadd.2d	v27, v24, v24
     18c: 4e76d6d6     	fadd.2d	v22, v22, v22
     190: 4ea61cdc     	mov.16b	v28, v6
     194: 4e70cedc     	fmla.2d	v28, v22, v16
     198: 4ea61cd6     	mov.16b	v22, v6
     19c: 4e7ccf76     	fmla.2d	v22, v27, v28
     1a0: 6e78df10     	fmul.2d	v16, v24, v24
     1a4: 4efccf90     	fmls.2d	v16, v28, v28
     1a8: 4e65d618     	fadd.2d	v24, v16, v5
     1ac: 6e78df10     	fmul.2d	v16, v24, v24
     1b0: 4ef6ced0     	fmls.2d	v16, v22, v22
     1b4: 4e65d610     	fadd.2d	v16, v16, v5
     1b8: 4ef7cefa     	fmls.2d	v26, v23, v23
     1bc: 4e67d75a     	fadd.2d	v26, v26, v7
     1c0: 4e7ad75b     	fadd.2d	v27, v26, v26
     1c4: 4e62d45d     	fadd.2d	v29, v2, v2
     1c8: 4ea61cdc     	mov.16b	v28, v6
     1cc: 4e77cfbc     	fmla.2d	v28, v29, v23
     1d0: 4ea61cd7     	mov.16b	v23, v6
     1d4: 4e7ccf77     	fmla.2d	v23, v27, v28
     1d8: 6e7adf5a     	fmul.2d	v26, v26, v26
     1dc: 4efccf9a     	fmls.2d	v26, v28, v28
     1e0: 4e67d75a     	fadd.2d	v26, v26, v7
     1e4: 6e7adf5b     	fmul.2d	v27, v26, v26
     1e8: 4ef7cefb     	fmls.2d	v27, v23, v23
     1ec: 4e67d77b     	fadd.2d	v27, v27, v7
     1f0: 4e7bd77c     	fadd.2d	v28, v27, v27
     1f4: 4e7ad75a     	fadd.2d	v26, v26, v26
     1f8: 4ea61cdd     	mov.16b	v29, v6
     1fc: 4e77cf5d     	fmla.2d	v29, v26, v23
     200: 4ea61cda     	mov.16b	v26, v6
     204: 4e7dcf9a     	fmla.2d	v26, v28, v29
     208: 6e7bdf77     	fmul.2d	v23, v27, v27
     20c: 4efdcfb7     	fmls.2d	v23, v29, v29
     210: 4e67d6f7     	fadd.2d	v23, v23, v7
     214: 6e77defb     	fmul.2d	v27, v23, v23
     218: 4efacf5b     	fmls.2d	v27, v26, v26
     21c: 4e67d762     	fadd.2d	v2, v27, v7
     220: 4ef3ce79     	fmls.2d	v25, v19, v19
     224: 4e71d739     	fadd.2d	v25, v25, v17
     228: 4e79d73b     	fadd.2d	v27, v25, v25
     22c: 4e61d43d     	fadd.2d	v29, v1, v1
     230: 4ea61cdc     	mov.16b	v28, v6
     234: 4e73cfbc     	fmla.2d	v28, v29, v19
     238: 4ea61cd3     	mov.16b	v19, v6
     23c: 4e7ccf73     	fmla.2d	v19, v27, v28
     240: 6e79df39     	fmul.2d	v25, v25, v25
     244: 4efccf99     	fmls.2d	v25, v28, v28
     248: 4e71d739     	fadd.2d	v25, v25, v17
     24c: 6e79df3b     	fmul.2d	v27, v25, v25
     250: 4ef3ce7b     	fmls.2d	v27, v19, v19
     254: 4e71d77b     	fadd.2d	v27, v27, v17
     258: 4e7bd77c     	fadd.2d	v28, v27, v27
     25c: 4e79d739     	fadd.2d	v25, v25, v25
     260: 4ea61cdd     	mov.16b	v29, v6
     264: 4e73cf3d     	fmla.2d	v29, v25, v19
     268: 4ea61cd9     	mov.16b	v25, v6
     26c: 4e7dcf99     	fmla.2d	v25, v28, v29
     270: 6e7bdf73     	fmul.2d	v19, v27, v27
     274: 4efdcfb3     	fmls.2d	v19, v29, v29
     278: 4e71d673     	fadd.2d	v19, v19, v17
     27c: 6e73de7b     	fmul.2d	v27, v19, v19
     280: 4ef9cf3b     	fmls.2d	v27, v25, v25
     284: 4e71d761     	fadd.2d	v1, v27, v17
     288: 4ee3cc74     	fmls.2d	v20, v3, v3
     28c: 4e72d69b     	fadd.2d	v27, v20, v18
     290: 4e7bd77c     	fadd.2d	v28, v27, v27
     294: 4e60d414     	fadd.2d	v20, v0, v0
     298: 4ea61cdd     	mov.16b	v29, v6
     29c: 4e63ce9d     	fmla.2d	v29, v20, v3
     2a0: 4ea61cd4     	mov.16b	v20, v6
     2a4: 4e7dcf94     	fmla.2d	v20, v28, v29
     2a8: 6e7bdf7b     	fmul.2d	v27, v27, v27
     2ac: 4efdcfbb     	fmls.2d	v27, v29, v29
     2b0: 4e72d77b     	fadd.2d	v27, v27, v18
     2b4: 6e7bdf7c     	fmul.2d	v28, v27, v27
     2b8: 4ef4ce9c     	fmls.2d	v28, v20, v20
     2bc: 4e72d79c     	fadd.2d	v28, v28, v18
     2c0: 4e7cd79d     	fadd.2d	v29, v28, v28
     2c4: 4e7bd77e     	fadd.2d	v30, v27, v27
     2c8: 4ea61cdb     	mov.16b	v27, v6
     2cc: 4e74cfdb     	fmla.2d	v27, v30, v20
     2d0: 4ea61cd4     	mov.16b	v20, v6
     2d4: 4e7bcfb4     	fmla.2d	v20, v29, v27
     2d8: 6e7cdf9c     	fmul.2d	v28, v28, v28
     2dc: 4efbcf7c     	fmls.2d	v28, v27, v27
     2e0: 4e72d79b     	fadd.2d	v27, v28, v18
     2e4: 6e7bdf7c     	fmul.2d	v28, v27, v27
     2e8: 4ef4ce9c     	fmls.2d	v28, v20, v20
     2ec: 4e72d780     	fadd.2d	v0, v28, v18
     2f0: 4e78d71c     	fadd.2d	v28, v24, v24
     2f4: 4ea61cd8     	mov.16b	v24, v6
     2f8: 4e76cf98     	fmla.2d	v24, v28, v22
     2fc: 4e77d6f6     	fadd.2d	v22, v23, v23
     300: 4ea61cd7     	mov.16b	v23, v6
     304: 4e7aced7     	fmla.2d	v23, v22, v26
     308: 4e73d676     	fadd.2d	v22, v19, v19
     30c: 4ea61cd3     	mov.16b	v19, v6
     310: 4e79ced3     	fmla.2d	v19, v22, v25
     314: 4e7bd776     	fadd.2d	v22, v27, v27
     318: 4ea61cc3     	mov.16b	v3, v6
     31c: 4e74cec3     	fmla.2d	v3, v22, v20
     320: 1e629014     	fmov	d20, #5.00000000
     324: 1e742ab5     	fadd	d21, d21, d20
     328: 17ffff62     	b	0xb0 <_mech_map_bits+0xb0>
     32c: 927900ef     	and	x15, x7, #0x80
     330: 927a0100     	and	x0, x8, #0x40
     334: aa0001ef     	orr	x15, x15, x0
     338: 927b0129     	and	x9, x9, #0x20
     33c: aa0901e9     	orr	x9, x15, x9
     340: 927c014a     	and	x10, x10, #0x10
     344: aa0a0129     	orr	x9, x9, x10
     348: 927d016a     	and	x10, x11, #0x8
     34c: aa0a0129     	orr	x9, x9, x10
     350: 927e018a     	and	x10, x12, #0x4
     354: aa0a0129     	orr	x9, x9, x10
     358: 927f01aa     	and	x10, x13, #0x2
     35c: aa0a0129     	orr	x9, x9, x10
     360: 924001ca     	and	x10, x14, #0x1
     364: aa0a0120     	orr	x0, x9, x10
     368: d65f03c0     	ret

000000000000036c <_mech_grid_row>:
     36c: d503237f     	pacibsp
     370: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     374: 910003fd     	mov	x29, sp
     378: a9bf5bf5     	stp	x21, x22, [sp, #-0x10]!
     37c: a9bf53f3     	stp	x19, x20, [sp, #-0x10]!
     380: d10143ff     	sub	sp, sp, #0x50
     384: aa0203f5     	mov	x21, x2
     388: 9e630000     	ucvtf	d0, x0
     38c: 910003f3     	mov	x19, sp
     390: fc0003e0     	stur	d0, [sp]
     394: 9e630020     	ucvtf	d0, x1
     398: fc0483e0     	stur	d0, [sp, #0x48]
     39c: d2800014     	mov	x20, #0x0               ; =0
     3a0: f100001f     	cmp	x0, #0x0
     3a4: aa0003f6     	mov	x22, x0
     3a8: 540005a0     	b.eq	0x45c <_mech_grid_row+0xf0>
     3ac: 9e630280     	ucvtf	d0, x20
     3b0: fc0083e0     	stur	d0, [sp, #0x8]
     3b4: 91000680     	add	x0, x20, #0x1
     3b8: 9e630000     	ucvtf	d0, x0
     3bc: fc0103e0     	stur	d0, [sp, #0x10]
     3c0: 91000a80     	add	x0, x20, #0x2
     3c4: 9e630000     	ucvtf	d0, x0
     3c8: fc0183e0     	stur	d0, [sp, #0x18]
     3cc: 91000e80     	add	x0, x20, #0x3
     3d0: 9e630000     	ucvtf	d0, x0
     3d4: fc0203e0     	stur	d0, [sp, #0x20]
     3d8: 91001280     	add	x0, x20, #0x4
     3dc: 9e630000     	ucvtf	d0, x0
     3e0: fc0283e0     	stur	d0, [sp, #0x28]
     3e4: 91001680     	add	x0, x20, #0x5
     3e8: 9e630000     	ucvtf	d0, x0
     3ec: fc0303e0     	stur	d0, [sp, #0x30]
     3f0: 91001a80     	add	x0, x20, #0x6
     3f4: 9e630000     	ucvtf	d0, x0
     3f8: fc0383e0     	stur	d0, [sp, #0x38]
     3fc: 91001e80     	add	x0, x20, #0x7
     400: 9e630000     	ucvtf	d0, x0
     404: fc0403e0     	stur	d0, [sp, #0x40]
     408: aa1303e0     	mov	x0, x19
     40c: 94000000     	bl	0x40c <_mech_grid_row+0xa0>
     410: aa1603e8     	mov	x8, x22
     414: cb140101     	sub	x1, x8, x20
     418: d2800102     	mov	x2, #0x8                ; =8
     41c: cb010042     	sub	x2, x2, x1
     420: d2800003     	mov	x3, #0x0                ; =0
     424: f100203f     	cmp	x1, #0x8
     428: 9a833041     	csel	x1, x2, x3, lo
     42c: d2801fe2     	mov	x2, #0xff               ; =255
     430: 9ac12041     	lsl	x1, x2, x1
     434: 8a010000     	and	x0, x0, x1
     438: d343fe81     	lsr	x1, x20, #3
     43c: aa1503e2     	mov	x2, x21
     440: 38216840     	strb	w0, [x2, x1]
     444: 91002294     	add	x20, x20, #0x8
     448: eb08029f     	cmp	x20, x8
     44c: 54000082     	b.hs	0x45c <_mech_grid_row+0xf0>
     450: aa0203f5     	mov	x21, x2
     454: aa0803f6     	mov	x22, x8
     458: 17ffffd5     	b	0x3ac <_mech_grid_row+0x40>
     45c: 910143ff     	add	sp, sp, #0x50
     460: a8c153f3     	ldp	x19, x20, [sp], #0x10
     464: a8c15bf5     	ldp	x21, x22, [sp], #0x10
     468: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     46c: d65f0fff     	retab
