
benchmarks/clbg/mandelbrot/experiments/20261011-array-state/validation/five-rows.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000000000 <_mech_map_bits>:
       0: fd400001     	ldr	d1, [x0]
       4: fd400412     	ldr	d18, [x0, #0x8]
       8: fd400802     	ldr	d2, [x0, #0x10]
       c: fd400c11     	ldr	d17, [x0, #0x18]
      10: fd401003     	ldr	d3, [x0, #0x20]
      14: fd401407     	ldr	d7, [x0, #0x28]
      18: fd401804     	ldr	d4, [x0, #0x30]
      1c: fd401c06     	ldr	d6, [x0, #0x38]
      20: fd402005     	ldr	d5, [x0, #0x40]
      24: fd402414     	ldr	d20, [x0, #0x48]
      28: 4f00e410     	movi.16b	v16, #0x0
      2c: 0f000415     	movi.2s	v21, #0x0
      30: 6f00f600     	fmov.2d	v0, #4.00000000
      34: 1e604253     	fmov	d19, d18
      38: 6e180453     	mov.d	v19[1], v2[0]
      3c: 1e601002     	fmov	d2, #2.00000000
      40: 1e611842     	fdiv	d2, d2, d1
      44: 4e080452     	dup.2d	v18, v2[0]
      48: 6e72de61     	fmul.2d	v1, v19, v18
      4c: 6f03f713     	fmov.2d	v19, #1.50000000
      50: 4ef3d421     	fsub.2d	v1, v1, v19
      54: 1e620a82     	fmul	d2, d20, d2
      58: 1e6e1014     	fmov	d20, #1.00000000
      5c: 1e743842     	fsub	d2, d2, d20
      60: 4e080442     	dup.2d	v2, v2[0]
      64: 1e604231     	fmov	d17, d17
      68: 6e180471     	mov.d	v17[1], v3[0]
      6c: 6e72de23     	fmul.2d	v3, v17, v18
      70: 4ef3d463     	fsub.2d	v3, v3, v19
      74: 1e6040e7     	fmov	d7, d7
      78: 6e180487     	mov.d	v7[1], v4[0]
      7c: 6e72dce4     	fmul.2d	v4, v7, v18
      80: 4ef3d484     	fsub.2d	v4, v4, v19
      84: 1e6040c6     	fmov	d6, d6
      88: 6e1804a6     	mov.d	v6[1], v5[0]
      8c: 6e72dcc5     	fmul.2d	v5, v6, v18
      90: 4ef3d4a5     	fsub.2d	v5, v5, v19
      94: 4eb01e06     	mov.16b	v6, v16
      98: 4eb01e07     	mov.16b	v7, v16
      9c: 4eb01e11     	mov.16b	v17, v16
      a0: 4eb01e12     	mov.16b	v18, v16
      a4: 4eb01e13     	mov.16b	v19, v16
      a8: 4eb01e17     	mov.16b	v23, v16
      ac: 4eb01e19     	mov.16b	v25, v16
      b0: d2e80920     	mov	x0, #0x4049000000000000 ; =4632233691727265792
      b4: 9e670014     	fmov	d20, x0
      b8: 92800000     	mov	x0, #-0x1               ; =-1
      bc: d2800001     	mov	x1, #0x0                ; =0
      c0: 1e7422a0     	fcmp	d21, d20
      c4: 9a81a000     	csel	x0, x0, x1, ge
      c8: 6e70de18     	fmul.2d	v24, v16, v16
      cc: 6e77defe     	fmul.2d	v30, v23, v23
      d0: 4e7ed714     	fadd.2d	v20, v24, v30
      d4: 6e74e414     	fcmge.2d	v20, v0, v20
      d8: 4e083e81     	mov.d	x1, v20[0]
      dc: 4e183e82     	mov.d	x2, v20[1]
      e0: aa020024     	orr	x4, x1, x2
      e4: 6e71de3c     	fmul.2d	v28, v17, v17
      e8: 6e79df3d     	fmul.2d	v29, v25, v25
      ec: 4e7dd794     	fadd.2d	v20, v28, v29
      f0: 6e74e414     	fcmge.2d	v20, v0, v20
      f4: 4e083e83     	mov.d	x3, v20[0]
      f8: aa030085     	orr	x5, x4, x3
      fc: 4e183e84     	mov.d	x4, v20[1]
     100: aa0400a6     	orr	x6, x5, x4
     104: 6e67dcfa     	fmul.2d	v26, v7, v7
     108: 6e73de7b     	fmul.2d	v27, v19, v19
     10c: 4e7bd754     	fadd.2d	v20, v26, v27
     110: 6e74e414     	fcmge.2d	v20, v0, v20
     114: 4e083e85     	mov.d	x5, v20[0]
     118: aa0500c7     	orr	x7, x6, x5
     11c: 4e183e86     	mov.d	x6, v20[1]
     120: aa0600e8     	orr	x8, x7, x6
     124: 6e66dcd4     	fmul.2d	v20, v6, v6
     128: 6e72de56     	fmul.2d	v22, v18, v18
     12c: 4e76d69f     	fadd.2d	v31, v20, v22
     130: 6e7fe41f     	fcmge.2d	v31, v0, v31
     134: 4e083fe7     	mov.d	x7, v31[0]
     138: aa070109     	orr	x9, x8, x7
     13c: 4e183fe8     	mov.d	x8, v31[1]
     140: aa080129     	orr	x9, x9, x8
     144: aa290000     	orn	x0, x0, x9
     148: f100001f     	cmp	x0, #0x0
     14c: 54001101     	b.ne	0x36c <_mech_map_bits+0x36c>
     150: 4efed718     	fsub.2d	v24, v24, v30
     154: 4e61d718     	fadd.2d	v24, v24, v1
     158: 6e78df1e     	fmul.2d	v30, v24, v24
     15c: 4e70d610     	fadd.2d	v16, v16, v16
     160: 6e77de10     	fmul.2d	v16, v16, v23
     164: 4e62d617     	fadd.2d	v23, v16, v2
     168: 6e77def0     	fmul.2d	v16, v23, v23
     16c: 4ef0d7d0     	fsub.2d	v16, v30, v16
     170: 4e61d610     	fadd.2d	v16, v16, v1
     174: 6e70de1e     	fmul.2d	v30, v16, v16
     178: 4e78d718     	fadd.2d	v24, v24, v24
     17c: 6e77df17     	fmul.2d	v23, v24, v23
     180: 4e62d6f7     	fadd.2d	v23, v23, v2
     184: 6e77def8     	fmul.2d	v24, v23, v23
     188: 4ef8d7d8     	fsub.2d	v24, v30, v24
     18c: 4e61d718     	fadd.2d	v24, v24, v1
     190: 6e78df1e     	fmul.2d	v30, v24, v24
     194: 4e70d610     	fadd.2d	v16, v16, v16
     198: 6e77de10     	fmul.2d	v16, v16, v23
     19c: 4e62d610     	fadd.2d	v16, v16, v2
     1a0: 6e70de17     	fmul.2d	v23, v16, v16
     1a4: 4ef7d7d7     	fsub.2d	v23, v30, v23
     1a8: 4e61d6f7     	fadd.2d	v23, v23, v1
     1ac: 6e77defe     	fmul.2d	v30, v23, v23
     1b0: 4e78d718     	fadd.2d	v24, v24, v24
     1b4: 6e70df10     	fmul.2d	v16, v24, v16
     1b8: 4e62d618     	fadd.2d	v24, v16, v2
     1bc: 6e78df10     	fmul.2d	v16, v24, v24
     1c0: 4ef0d7d0     	fsub.2d	v16, v30, v16
     1c4: 4e61d610     	fadd.2d	v16, v16, v1
     1c8: 4efdd79c     	fsub.2d	v28, v28, v29
     1cc: 4e63d79c     	fadd.2d	v28, v28, v3
     1d0: 6e7cdf9d     	fmul.2d	v29, v28, v28
     1d4: 4e71d631     	fadd.2d	v17, v17, v17
     1d8: 6e79de31     	fmul.2d	v17, v17, v25
     1dc: 4e62d639     	fadd.2d	v25, v17, v2
     1e0: 6e79df31     	fmul.2d	v17, v25, v25
     1e4: 4ef1d7b1     	fsub.2d	v17, v29, v17
     1e8: 4e63d631     	fadd.2d	v17, v17, v3
     1ec: 6e71de3d     	fmul.2d	v29, v17, v17
     1f0: 4e7cd79c     	fadd.2d	v28, v28, v28
     1f4: 6e79df99     	fmul.2d	v25, v28, v25
     1f8: 4e62d739     	fadd.2d	v25, v25, v2
     1fc: 6e79df3c     	fmul.2d	v28, v25, v25
     200: 4efcd7bc     	fsub.2d	v28, v29, v28
     204: 4e63d79c     	fadd.2d	v28, v28, v3
     208: 6e7cdf9d     	fmul.2d	v29, v28, v28
     20c: 4e71d631     	fadd.2d	v17, v17, v17
     210: 6e79de31     	fmul.2d	v17, v17, v25
     214: 4e62d631     	fadd.2d	v17, v17, v2
     218: 6e71de39     	fmul.2d	v25, v17, v17
     21c: 4ef9d7b9     	fsub.2d	v25, v29, v25
     220: 4e63d739     	fadd.2d	v25, v25, v3
     224: 6e79df3d     	fmul.2d	v29, v25, v25
     228: 4e7cd79c     	fadd.2d	v28, v28, v28
     22c: 6e71df91     	fmul.2d	v17, v28, v17
     230: 4e62d63c     	fadd.2d	v28, v17, v2
     234: 6e7cdf91     	fmul.2d	v17, v28, v28
     238: 4ef1d7b1     	fsub.2d	v17, v29, v17
     23c: 4e63d631     	fadd.2d	v17, v17, v3
     240: 4efbd75a     	fsub.2d	v26, v26, v27
     244: 4e64d75a     	fadd.2d	v26, v26, v4
     248: 6e7adf5b     	fmul.2d	v27, v26, v26
     24c: 4e67d4e7     	fadd.2d	v7, v7, v7
     250: 6e73dce7     	fmul.2d	v7, v7, v19
     254: 4e62d4f3     	fadd.2d	v19, v7, v2
     258: 6e73de67     	fmul.2d	v7, v19, v19
     25c: 4ee7d767     	fsub.2d	v7, v27, v7
     260: 4e64d4e7     	fadd.2d	v7, v7, v4
     264: 6e67dcfb     	fmul.2d	v27, v7, v7
     268: 4e7ad75a     	fadd.2d	v26, v26, v26
     26c: 6e73df53     	fmul.2d	v19, v26, v19
     270: 4e62d673     	fadd.2d	v19, v19, v2
     274: 6e73de7a     	fmul.2d	v26, v19, v19
     278: 4efad77a     	fsub.2d	v26, v27, v26
     27c: 4e64d75a     	fadd.2d	v26, v26, v4
     280: 6e7adf5b     	fmul.2d	v27, v26, v26
     284: 4e67d4e7     	fadd.2d	v7, v7, v7
     288: 6e73dce7     	fmul.2d	v7, v7, v19
     28c: 4e62d4e7     	fadd.2d	v7, v7, v2
     290: 6e67dcf3     	fmul.2d	v19, v7, v7
     294: 4ef3d773     	fsub.2d	v19, v27, v19
     298: 4e64d673     	fadd.2d	v19, v19, v4
     29c: 6e73de7b     	fmul.2d	v27, v19, v19
     2a0: 4e7ad75a     	fadd.2d	v26, v26, v26
     2a4: 6e67df47     	fmul.2d	v7, v26, v7
     2a8: 4e62d4fa     	fadd.2d	v26, v7, v2
     2ac: 6e7adf47     	fmul.2d	v7, v26, v26
     2b0: 4ee7d767     	fsub.2d	v7, v27, v7
     2b4: 4e64d4e7     	fadd.2d	v7, v7, v4
     2b8: 4ef6d694     	fsub.2d	v20, v20, v22
     2bc: 4e65d694     	fadd.2d	v20, v20, v5
     2c0: 6e74de96     	fmul.2d	v22, v20, v20
     2c4: 4e66d4c6     	fadd.2d	v6, v6, v6
     2c8: 6e72dcc6     	fmul.2d	v6, v6, v18
     2cc: 4e62d4d2     	fadd.2d	v18, v6, v2
     2d0: 6e72de46     	fmul.2d	v6, v18, v18
     2d4: 4ee6d6c6     	fsub.2d	v6, v22, v6
     2d8: 4e65d4c6     	fadd.2d	v6, v6, v5
     2dc: 6e66dcd6     	fmul.2d	v22, v6, v6
     2e0: 4e74d694     	fadd.2d	v20, v20, v20
     2e4: 6e72de92     	fmul.2d	v18, v20, v18
     2e8: 4e62d652     	fadd.2d	v18, v18, v2
     2ec: 6e72de54     	fmul.2d	v20, v18, v18
     2f0: 4ef4d6d4     	fsub.2d	v20, v22, v20
     2f4: 4e65d694     	fadd.2d	v20, v20, v5
     2f8: 6e74de96     	fmul.2d	v22, v20, v20
     2fc: 4e66d4c6     	fadd.2d	v6, v6, v6
     300: 6e72dcc6     	fmul.2d	v6, v6, v18
     304: 4e62d4c6     	fadd.2d	v6, v6, v2
     308: 6e66dcd2     	fmul.2d	v18, v6, v6
     30c: 4ef2d6d2     	fsub.2d	v18, v22, v18
     310: 4e65d652     	fadd.2d	v18, v18, v5
     314: 6e72de56     	fmul.2d	v22, v18, v18
     318: 4e74d694     	fadd.2d	v20, v20, v20
     31c: 6e66de86     	fmul.2d	v6, v20, v6
     320: 4e62d4d4     	fadd.2d	v20, v6, v2
     324: 6e74de86     	fmul.2d	v6, v20, v20
     328: 4ee6d6c6     	fsub.2d	v6, v22, v6
     32c: 4e65d4c6     	fadd.2d	v6, v6, v5
     330: 4e77d6f6     	fadd.2d	v22, v23, v23
     334: 6e78ded6     	fmul.2d	v22, v22, v24
     338: 4e62d6d7     	fadd.2d	v23, v22, v2
     33c: 4e79d736     	fadd.2d	v22, v25, v25
     340: 6e7cded6     	fmul.2d	v22, v22, v28
     344: 4e62d6d9     	fadd.2d	v25, v22, v2
     348: 4e73d673     	fadd.2d	v19, v19, v19
     34c: 6e7ade73     	fmul.2d	v19, v19, v26
     350: 4e62d673     	fadd.2d	v19, v19, v2
     354: 4e72d652     	fadd.2d	v18, v18, v18
     358: 6e74de52     	fmul.2d	v18, v18, v20
     35c: 4e62d652     	fadd.2d	v18, v18, v2
     360: 1e629014     	fmov	d20, #5.00000000
     364: 1e742ab5     	fadd	d21, d21, d20
     368: 17ffff52     	b	0xb0 <_mech_map_bits+0xb0>
     36c: 92790020     	and	x0, x1, #0x80
     370: 927a0041     	and	x1, x2, #0x40
     374: aa010000     	orr	x0, x0, x1
     378: 927b0061     	and	x1, x3, #0x20
     37c: aa010000     	orr	x0, x0, x1
     380: 927c0081     	and	x1, x4, #0x10
     384: aa010000     	orr	x0, x0, x1
     388: 927d00a1     	and	x1, x5, #0x8
     38c: aa010000     	orr	x0, x0, x1
     390: 927e00c1     	and	x1, x6, #0x4
     394: aa010000     	orr	x0, x0, x1
     398: 927f00e1     	and	x1, x7, #0x2
     39c: aa010000     	orr	x0, x0, x1
     3a0: 92400101     	and	x1, x8, #0x1
     3a4: aa010000     	orr	x0, x0, x1
     3a8: d65f03c0     	ret

00000000000003ac <_mech_grid_row>:
     3ac: d503237f     	pacibsp
     3b0: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     3b4: 910003fd     	mov	x29, sp
     3b8: a9bf5bf5     	stp	x21, x22, [sp, #-0x10]!
     3bc: a9bf53f3     	stp	x19, x20, [sp, #-0x10]!
     3c0: d10143ff     	sub	sp, sp, #0x50
     3c4: aa0203f5     	mov	x21, x2
     3c8: 9e630000     	ucvtf	d0, x0
     3cc: 910003f3     	mov	x19, sp
     3d0: fc0003e0     	stur	d0, [sp]
     3d4: 9e630020     	ucvtf	d0, x1
     3d8: fc0483e0     	stur	d0, [sp, #0x48]
     3dc: d2800014     	mov	x20, #0x0               ; =0
     3e0: f100001f     	cmp	x0, #0x0
     3e4: aa0003f6     	mov	x22, x0
     3e8: 540005a0     	b.eq	0x49c <_mech_grid_row+0xf0>
     3ec: 9e630280     	ucvtf	d0, x20
     3f0: fc0083e0     	stur	d0, [sp, #0x8]
     3f4: 91000680     	add	x0, x20, #0x1
     3f8: 9e630000     	ucvtf	d0, x0
     3fc: fc0103e0     	stur	d0, [sp, #0x10]
     400: 91000a80     	add	x0, x20, #0x2
     404: 9e630000     	ucvtf	d0, x0
     408: fc0183e0     	stur	d0, [sp, #0x18]
     40c: 91000e80     	add	x0, x20, #0x3
     410: 9e630000     	ucvtf	d0, x0
     414: fc0203e0     	stur	d0, [sp, #0x20]
     418: 91001280     	add	x0, x20, #0x4
     41c: 9e630000     	ucvtf	d0, x0
     420: fc0283e0     	stur	d0, [sp, #0x28]
     424: 91001680     	add	x0, x20, #0x5
     428: 9e630000     	ucvtf	d0, x0
     42c: fc0303e0     	stur	d0, [sp, #0x30]
     430: 91001a80     	add	x0, x20, #0x6
     434: 9e630000     	ucvtf	d0, x0
     438: fc0383e0     	stur	d0, [sp, #0x38]
     43c: 91001e80     	add	x0, x20, #0x7
     440: 9e630000     	ucvtf	d0, x0
     444: fc0403e0     	stur	d0, [sp, #0x40]
     448: aa1303e0     	mov	x0, x19
     44c: 94000000     	bl	0x44c <_mech_grid_row+0xa0>
     450: aa1603e8     	mov	x8, x22
     454: cb140101     	sub	x1, x8, x20
     458: d2800102     	mov	x2, #0x8                ; =8
     45c: cb010042     	sub	x2, x2, x1
     460: d2800003     	mov	x3, #0x0                ; =0
     464: f100203f     	cmp	x1, #0x8
     468: 9a833041     	csel	x1, x2, x3, lo
     46c: d2801fe2     	mov	x2, #0xff               ; =255
     470: 9ac12041     	lsl	x1, x2, x1
     474: 8a010000     	and	x0, x0, x1
     478: d343fe81     	lsr	x1, x20, #3
     47c: aa1503e2     	mov	x2, x21
     480: 38216840     	strb	w0, [x2, x1]
     484: 91002294     	add	x20, x20, #0x8
     488: eb08029f     	cmp	x20, x8
     48c: 54000082     	b.hs	0x49c <_mech_grid_row+0xf0>
     490: aa0203f5     	mov	x21, x2
     494: aa0803f6     	mov	x22, x8
     498: 17ffffd5     	b	0x3ec <_mech_grid_row+0x40>
     49c: 910143ff     	add	sp, sp, #0x50
     4a0: a8c153f3     	ldp	x19, x20, [sp], #0x10
     4a4: a8c15bf5     	ldp	x21, x22, [sp], #0x10
     4a8: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     4ac: d65f0fff     	retab
