
/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/mandelbrot/experiments/20261011-array-state/validation-current/five16-loop-fma.o:	file format mach-o arm64

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
      34: 1e60425f     	fmov	d31, d18
      38: 6e18045f     	mov.d	v31[1], v2[0]
      3c: 1e601002     	fmov	d2, #2.00000000
      40: 1e611842     	fdiv	d2, d2, d1
      44: 4e080452     	dup.2d	v18, v2[0]
      48: 6e72dfe1     	fmul.2d	v1, v31, v18
      4c: 6f03f713     	fmov.2d	v19, #1.50000000
      50: 4ef3d421     	fsub.2d	v1, v1, v19
      54: 1e620a82     	fmul	d2, d20, d2
      58: 1e6e1014     	fmov	d20, #1.00000000
      5c: 1e743842     	fsub	d2, d2, d20
      60: 4e080442     	dup.2d	v2, v2[0]
      64: 1e60423f     	fmov	d31, d17
      68: 6e18047f     	mov.d	v31[1], v3[0]
      6c: 6e72dfe3     	fmul.2d	v3, v31, v18
      70: 4ef3d463     	fsub.2d	v3, v3, v19
      74: 1e6040ff     	fmov	d31, d7
      78: 6e18049f     	mov.d	v31[1], v4[0]
      7c: 6e72dfe4     	fmul.2d	v4, v31, v18
      80: 4ef3d484     	fsub.2d	v4, v4, v19
      84: 1e6040df     	fmov	d31, d6
      88: 6e1804bf     	mov.d	v31[1], v5[0]
      8c: 6e72dfe5     	fmul.2d	v5, v31, v18
      90: 4ef3d4a5     	fsub.2d	v5, v5, v19
      94: 4eb01e06     	mov.16b	v6, v16
      98: 4eb01e07     	mov.16b	v7, v16
      9c: 4eb01e11     	mov.16b	v17, v16
      a0: 4eb01e12     	mov.16b	v18, v16
      a4: 4eb01e13     	mov.16b	v19, v16
      a8: 4eb01e14     	mov.16b	v20, v16
      ac: 4eb01e16     	mov.16b	v22, v16
      b0: d2e80920     	mov	x0, #0x4049000000000000 ; =4632233691727265792
      b4: 9e670017     	fmov	d23, x0
      b8: 92800000     	mov	x0, #-0x1               ; =-1
      bc: d2800001     	mov	x1, #0x0                ; =0
      c0: 1e7722a0     	fcmp	d21, d23
      c4: 9a81a000     	csel	x0, x0, x1, ge
      c8: 4ed65a1d     	uzp2.2d	v29, v16, v22
      cc: 1e700a17     	fmul	d23, d16, d16
      d0: 1e6042fe     	fmov	d30, d23
      d4: 1e760ad7     	fmul	d23, d22, d22
      d8: 6e1806fe     	mov.d	v30[1], v23[0]
      dc: 4ebe1fd7     	mov.16b	v23, v30
      e0: 4e7dcfb7     	fmla.2d	v23, v29, v29
      e4: 6e77e417     	fcmge.2d	v23, v0, v23
      e8: 4e083ee1     	mov.d	x1, v23[0]
      ec: 4e183ee2     	mov.d	x2, v23[1]
      f0: aa020024     	orr	x4, x1, x2
      f4: 4ed45a3b     	uzp2.2d	v27, v17, v20
      f8: 1e710a37     	fmul	d23, d17, d17
      fc: 1e6042fc     	fmov	d28, d23
     100: 1e740a97     	fmul	d23, d20, d20
     104: 6e1806fc     	mov.d	v28[1], v23[0]
     108: 4ebc1f97     	mov.16b	v23, v28
     10c: 4e7bcf77     	fmla.2d	v23, v27, v27
     110: 6e77e417     	fcmge.2d	v23, v0, v23
     114: 4e083ee3     	mov.d	x3, v23[0]
     118: aa030085     	orr	x5, x4, x3
     11c: 4e183ee4     	mov.d	x4, v23[1]
     120: aa0400a6     	orr	x6, x5, x4
     124: 4ed358f9     	uzp2.2d	v25, v7, v19
     128: 1e6708f7     	fmul	d23, d7, d7
     12c: 1e6042fa     	fmov	d26, d23
     130: 1e730a77     	fmul	d23, d19, d19
     134: 6e1806fa     	mov.d	v26[1], v23[0]
     138: 4eba1f57     	mov.16b	v23, v26
     13c: 4e79cf37     	fmla.2d	v23, v25, v25
     140: 6e77e417     	fcmge.2d	v23, v0, v23
     144: 4e083ee5     	mov.d	x5, v23[0]
     148: aa0500c7     	orr	x7, x6, x5
     14c: 4e183ee6     	mov.d	x6, v23[1]
     150: aa0600e8     	orr	x8, x7, x6
     154: 4ed258d7     	uzp2.2d	v23, v6, v18
     158: 1e6608d8     	fmul	d24, d6, d6
     15c: 1e604318     	fmov	d24, d24
     160: 1e720a5f     	fmul	d31, d18, d18
     164: 6e1807f8     	mov.d	v24[1], v31[0]
     168: 4eb81f1f     	mov.16b	v31, v24
     16c: 4e77ceff     	fmla.2d	v31, v23, v23
     170: 6e7fe41f     	fcmge.2d	v31, v0, v31
     174: 4e083fe7     	mov.d	x7, v31[0]
     178: aa070109     	orr	x9, x8, x7
     17c: 4e183fe8     	mov.d	x8, v31[1]
     180: aa080129     	orr	x9, x9, x8
     184: aa290000     	orn	x0, x0, x9
     188: f100001f     	cmp	x0, #0x0
     18c: 54001081     	b.ne	0x39c <_mech_map_bits+0x39c>
     190: 4efdcfbe     	fmls.2d	v30, v29, v29
     194: 4e61d7de     	fadd.2d	v30, v30, v1
     198: 4e7ed7df     	fadd.2d	v31, v30, v30
     19c: 4ed61a10     	uzp1.2d	v16, v16, v22
     1a0: 4e70d610     	fadd.2d	v16, v16, v16
     1a4: 4ea21c56     	mov.16b	v22, v2
     1a8: 4e7dce16     	fmla.2d	v22, v16, v29
     1ac: 4ea21c50     	mov.16b	v16, v2
     1b0: 4e76cff0     	fmla.2d	v16, v31, v22
     1b4: 6e7edfdd     	fmul.2d	v29, v30, v30
     1b8: 4ef6cedd     	fmls.2d	v29, v22, v22
     1bc: 4e61d7b6     	fadd.2d	v22, v29, v1
     1c0: 6e76dedd     	fmul.2d	v29, v22, v22
     1c4: 4ef0ce1d     	fmls.2d	v29, v16, v16
     1c8: 4e61d7bd     	fadd.2d	v29, v29, v1
     1cc: 4e7dd7be     	fadd.2d	v30, v29, v29
     1d0: 4e76d6df     	fadd.2d	v31, v22, v22
     1d4: 4ea21c56     	mov.16b	v22, v2
     1d8: 4e70cff6     	fmla.2d	v22, v31, v16
     1dc: 4ea21c50     	mov.16b	v16, v2
     1e0: 4e76cfd0     	fmla.2d	v16, v30, v22
     1e4: 6e7ddfbd     	fmul.2d	v29, v29, v29
     1e8: 4ef6cedd     	fmls.2d	v29, v22, v22
     1ec: 4e61d7b6     	fadd.2d	v22, v29, v1
     1f0: 6e76dedd     	fmul.2d	v29, v22, v22
     1f4: 4ef0ce1d     	fmls.2d	v29, v16, v16
     1f8: 4e61d7bd     	fadd.2d	v29, v29, v1
     1fc: 4e76d6d6     	fadd.2d	v22, v22, v22
     200: 4ea21c5e     	mov.16b	v30, v2
     204: 4e70cede     	fmla.2d	v30, v22, v16
     208: 4ede1bb0     	uzp1.2d	v16, v29, v30
     20c: 4ede5bb6     	uzp2.2d	v22, v29, v30
     210: 4efbcf7c     	fmls.2d	v28, v27, v27
     214: 4e63d79c     	fadd.2d	v28, v28, v3
     218: 4e7cd79d     	fadd.2d	v29, v28, v28
     21c: 4ed41a31     	uzp1.2d	v17, v17, v20
     220: 4e71d631     	fadd.2d	v17, v17, v17
     224: 4ea21c54     	mov.16b	v20, v2
     228: 4e7bce34     	fmla.2d	v20, v17, v27
     22c: 4ea21c51     	mov.16b	v17, v2
     230: 4e74cfb1     	fmla.2d	v17, v29, v20
     234: 6e7cdf9b     	fmul.2d	v27, v28, v28
     238: 4ef4ce9b     	fmls.2d	v27, v20, v20
     23c: 4e63d774     	fadd.2d	v20, v27, v3
     240: 6e74de9b     	fmul.2d	v27, v20, v20
     244: 4ef1ce3b     	fmls.2d	v27, v17, v17
     248: 4e63d77b     	fadd.2d	v27, v27, v3
     24c: 4e7bd77c     	fadd.2d	v28, v27, v27
     250: 4e74d69d     	fadd.2d	v29, v20, v20
     254: 4ea21c54     	mov.16b	v20, v2
     258: 4e71cfb4     	fmla.2d	v20, v29, v17
     25c: 4ea21c51     	mov.16b	v17, v2
     260: 4e74cf91     	fmla.2d	v17, v28, v20
     264: 6e7bdf7b     	fmul.2d	v27, v27, v27
     268: 4ef4ce9b     	fmls.2d	v27, v20, v20
     26c: 4e63d774     	fadd.2d	v20, v27, v3
     270: 6e74de9b     	fmul.2d	v27, v20, v20
     274: 4ef1ce3b     	fmls.2d	v27, v17, v17
     278: 4e63d77b     	fadd.2d	v27, v27, v3
     27c: 4e74d694     	fadd.2d	v20, v20, v20
     280: 4ea21c5c     	mov.16b	v28, v2
     284: 4e71ce9c     	fmla.2d	v28, v20, v17
     288: 4edc1b71     	uzp1.2d	v17, v27, v28
     28c: 4edc5b74     	uzp2.2d	v20, v27, v28
     290: 4ef9cf3a     	fmls.2d	v26, v25, v25
     294: 4e64d75a     	fadd.2d	v26, v26, v4
     298: 4e7ad75b     	fadd.2d	v27, v26, v26
     29c: 4ed318e7     	uzp1.2d	v7, v7, v19
     2a0: 4e67d4e7     	fadd.2d	v7, v7, v7
     2a4: 4ea21c53     	mov.16b	v19, v2
     2a8: 4e79ccf3     	fmla.2d	v19, v7, v25
     2ac: 4ea21c47     	mov.16b	v7, v2
     2b0: 4e73cf67     	fmla.2d	v7, v27, v19
     2b4: 6e7adf59     	fmul.2d	v25, v26, v26
     2b8: 4ef3ce79     	fmls.2d	v25, v19, v19
     2bc: 4e64d733     	fadd.2d	v19, v25, v4
     2c0: 6e73de79     	fmul.2d	v25, v19, v19
     2c4: 4ee7ccf9     	fmls.2d	v25, v7, v7
     2c8: 4e64d739     	fadd.2d	v25, v25, v4
     2cc: 4e79d73a     	fadd.2d	v26, v25, v25
     2d0: 4e73d67b     	fadd.2d	v27, v19, v19
     2d4: 4ea21c53     	mov.16b	v19, v2
     2d8: 4e67cf73     	fmla.2d	v19, v27, v7
     2dc: 4ea21c47     	mov.16b	v7, v2
     2e0: 4e73cf47     	fmla.2d	v7, v26, v19
     2e4: 6e79df39     	fmul.2d	v25, v25, v25
     2e8: 4ef3ce79     	fmls.2d	v25, v19, v19
     2ec: 4e64d733     	fadd.2d	v19, v25, v4
     2f0: 6e73de79     	fmul.2d	v25, v19, v19
     2f4: 4ee7ccf9     	fmls.2d	v25, v7, v7
     2f8: 4e64d739     	fadd.2d	v25, v25, v4
     2fc: 4e73d673     	fadd.2d	v19, v19, v19
     300: 4ea21c5a     	mov.16b	v26, v2
     304: 4e67ce7a     	fmla.2d	v26, v19, v7
     308: 4eda1b27     	uzp1.2d	v7, v25, v26
     30c: 4eda5b33     	uzp2.2d	v19, v25, v26
     310: 4ef7cef8     	fmls.2d	v24, v23, v23
     314: 4e65d718     	fadd.2d	v24, v24, v5
     318: 4e78d719     	fadd.2d	v25, v24, v24
     31c: 4ed218c6     	uzp1.2d	v6, v6, v18
     320: 4e66d4c6     	fadd.2d	v6, v6, v6
     324: 4ea21c52     	mov.16b	v18, v2
     328: 4e77ccd2     	fmla.2d	v18, v6, v23
     32c: 4ea21c46     	mov.16b	v6, v2
     330: 4e72cf26     	fmla.2d	v6, v25, v18
     334: 6e78df17     	fmul.2d	v23, v24, v24
     338: 4ef2ce57     	fmls.2d	v23, v18, v18
     33c: 4e65d6f2     	fadd.2d	v18, v23, v5
     340: 6e72de57     	fmul.2d	v23, v18, v18
     344: 4ee6ccd7     	fmls.2d	v23, v6, v6
     348: 4e65d6f7     	fadd.2d	v23, v23, v5
     34c: 4e77d6f8     	fadd.2d	v24, v23, v23
     350: 4e72d659     	fadd.2d	v25, v18, v18
     354: 4ea21c52     	mov.16b	v18, v2
     358: 4e66cf32     	fmla.2d	v18, v25, v6
     35c: 4ea21c46     	mov.16b	v6, v2
     360: 4e72cf06     	fmla.2d	v6, v24, v18
     364: 6e77def7     	fmul.2d	v23, v23, v23
     368: 4ef2ce57     	fmls.2d	v23, v18, v18
     36c: 4e65d6f2     	fadd.2d	v18, v23, v5
     370: 6e72de57     	fmul.2d	v23, v18, v18
     374: 4ee6ccd7     	fmls.2d	v23, v6, v6
     378: 4e65d6f7     	fadd.2d	v23, v23, v5
     37c: 4e72d652     	fadd.2d	v18, v18, v18
     380: 4ea21c58     	mov.16b	v24, v2
     384: 4e66ce58     	fmla.2d	v24, v18, v6
     388: 4ed81ae6     	uzp1.2d	v6, v23, v24
     38c: 4ed85af2     	uzp2.2d	v18, v23, v24
     390: 1e629017     	fmov	d23, #5.00000000
     394: 1e772ab5     	fadd	d21, d21, d23
     398: 17ffff46     	b	0xb0 <_mech_map_bits+0xb0>
     39c: 92790020     	and	x0, x1, #0x80
     3a0: 927a0041     	and	x1, x2, #0x40
     3a4: aa010000     	orr	x0, x0, x1
     3a8: 927b0061     	and	x1, x3, #0x20
     3ac: aa010000     	orr	x0, x0, x1
     3b0: 927c0081     	and	x1, x4, #0x10
     3b4: aa010000     	orr	x0, x0, x1
     3b8: 927d00a1     	and	x1, x5, #0x8
     3bc: aa010000     	orr	x0, x0, x1
     3c0: 927e00c1     	and	x1, x6, #0x4
     3c4: aa010000     	orr	x0, x0, x1
     3c8: 927f00e1     	and	x1, x7, #0x2
     3cc: aa010000     	orr	x0, x0, x1
     3d0: 92400101     	and	x1, x8, #0x1
     3d4: aa010000     	orr	x0, x0, x1
     3d8: d65f03c0     	ret

00000000000003dc <_mech_grid_row>:
     3dc: d503237f     	pacibsp
     3e0: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     3e4: 910003fd     	mov	x29, sp
     3e8: a9bf5bf5     	stp	x21, x22, [sp, #-0x10]!
     3ec: a9bf53f3     	stp	x19, x20, [sp, #-0x10]!
     3f0: d10143ff     	sub	sp, sp, #0x50
     3f4: aa0203f5     	mov	x21, x2
     3f8: 9e630000     	ucvtf	d0, x0
     3fc: 910003f3     	mov	x19, sp
     400: fc0003e0     	stur	d0, [sp]
     404: 9e630020     	ucvtf	d0, x1
     408: fc0483e0     	stur	d0, [sp, #0x48]
     40c: d2800014     	mov	x20, #0x0               ; =0
     410: f100001f     	cmp	x0, #0x0
     414: aa0003f6     	mov	x22, x0
     418: 540005a0     	b.eq	0x4cc <_mech_grid_row+0xf0>
     41c: 9e630280     	ucvtf	d0, x20
     420: fc0083e0     	stur	d0, [sp, #0x8]
     424: 91000680     	add	x0, x20, #0x1
     428: 9e630000     	ucvtf	d0, x0
     42c: fc0103e0     	stur	d0, [sp, #0x10]
     430: 91000a80     	add	x0, x20, #0x2
     434: 9e630000     	ucvtf	d0, x0
     438: fc0183e0     	stur	d0, [sp, #0x18]
     43c: 91000e80     	add	x0, x20, #0x3
     440: 9e630000     	ucvtf	d0, x0
     444: fc0203e0     	stur	d0, [sp, #0x20]
     448: 91001280     	add	x0, x20, #0x4
     44c: 9e630000     	ucvtf	d0, x0
     450: fc0283e0     	stur	d0, [sp, #0x28]
     454: 91001680     	add	x0, x20, #0x5
     458: 9e630000     	ucvtf	d0, x0
     45c: fc0303e0     	stur	d0, [sp, #0x30]
     460: 91001a80     	add	x0, x20, #0x6
     464: 9e630000     	ucvtf	d0, x0
     468: fc0383e0     	stur	d0, [sp, #0x38]
     46c: 91001e80     	add	x0, x20, #0x7
     470: 9e630000     	ucvtf	d0, x0
     474: fc0403e0     	stur	d0, [sp, #0x40]
     478: aa1303e0     	mov	x0, x19
     47c: 94000000     	bl	0x47c <_mech_grid_row+0xa0>
     480: aa1603e8     	mov	x8, x22
     484: cb140101     	sub	x1, x8, x20
     488: d2800102     	mov	x2, #0x8                ; =8
     48c: cb010042     	sub	x2, x2, x1
     490: d2800003     	mov	x3, #0x0                ; =0
     494: f100203f     	cmp	x1, #0x8
     498: 9a833041     	csel	x1, x2, x3, lo
     49c: d2801fe2     	mov	x2, #0xff               ; =255
     4a0: 9ac12041     	lsl	x1, x2, x1
     4a4: 8a010000     	and	x0, x0, x1
     4a8: d343fe81     	lsr	x1, x20, #3
     4ac: aa1503e2     	mov	x2, x21
     4b0: 38216840     	strb	w0, [x2, x1]
     4b4: 91002294     	add	x20, x20, #0x8
     4b8: eb08029f     	cmp	x20, x8
     4bc: 54000082     	b.hs	0x4cc <_mech_grid_row+0xf0>
     4c0: aa0203f5     	mov	x21, x2
     4c4: aa0803f6     	mov	x22, x8
     4c8: 17ffffd5     	b	0x41c <_mech_grid_row+0x40>
     4cc: 910143ff     	add	sp, sp, #0x50
     4d0: a8c153f3     	ldp	x19, x20, [sp], #0x10
     4d4: a8c15bf5     	ldp	x21, x22, [sp], #0x10
     4d8: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     4dc: d65f0fff     	retab
