
benchmarks/clbg/mandelbrot/.build/mech-unrolled-simd-loop-fma.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000000000 <_mech_map_batch>:
       0: d503237f     	pacibsp
       4: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
       8: 910003fd     	mov	x29, sp
       c: 6dbf3fee     	stp	d14, d15, [sp, #-0x10]!
      10: 6dbf37ec     	stp	d12, d13, [sp, #-0x10]!
      14: 6dbf2fea     	stp	d10, d11, [sp, #-0x10]!
      18: 6dbf27e8     	stp	d8, d9, [sp, #-0x10]!
      1c: d10083ff     	sub	sp, sp, #0x20
      20: fd400004     	ldr	d4, [x0]
      24: fd400414     	ldr	d20, [x0, #0x8]
      28: fd400805     	ldr	d5, [x0, #0x10]
      2c: fd400c13     	ldr	d19, [x0, #0x18]
      30: fd401006     	ldr	d6, [x0, #0x20]
      34: fd401412     	ldr	d18, [x0, #0x28]
      38: fd401807     	ldr	d7, [x0, #0x30]
      3c: fd401c11     	ldr	d17, [x0, #0x38]
      40: fd402010     	ldr	d16, [x0, #0x40]
      44: fd402417     	ldr	d23, [x0, #0x48]
      48: 4f00e401     	movi.16b	v1, #0x0
      4c: 0f000400     	movi.2s	v0, #0x0
      50: 3c8003e0     	stur	q0, [sp]
      54: 1e6e1016     	fmov	d22, #1.00000000
      58: 6f00f602     	fmov.2d	v2, #4.00000000
      5c: 3c8103e2     	stur	q2, [sp, #0x10]
      60: 6f03f600     	fmov.2d	v0, #1.00000000
      64: 4ea01c18     	mov.16b	v24, v0
      68: 6f00f403     	fmov.2d	v3, #2.00000000
      6c: 1e604295     	fmov	d21, d20
      70: 6e1804b5     	mov.d	v21[1], v5[0]
      74: 1e601000     	fmov	d0, #2.00000000
      78: 1e641805     	fdiv	d5, d0, d4
      7c: 4e0804b4     	dup.2d	v20, v5[0]
      80: 6e74dea4     	fmul.2d	v4, v21, v20
      84: 6f03f715     	fmov.2d	v21, #1.50000000
      88: 4ef5d484     	fsub.2d	v4, v4, v21
      8c: 1e650ae0     	fmul	d0, d23, d5
      90: 1e763800     	fsub	d0, d0, d22
      94: 4e080405     	dup.2d	v5, v0[0]
      98: 1e604273     	fmov	d19, d19
      9c: 6e1804d3     	mov.d	v19[1], v6[0]
      a0: 6e74de60     	fmul.2d	v0, v19, v20
      a4: 4ef5d406     	fsub.2d	v6, v0, v21
      a8: 1e604252     	fmov	d18, d18
      ac: 6e1804f2     	mov.d	v18[1], v7[0]
      b0: 6e74de40     	fmul.2d	v0, v18, v20
      b4: 4ef5d407     	fsub.2d	v7, v0, v21
      b8: 1e604231     	fmov	d17, d17
      bc: 6e180611     	mov.d	v17[1], v16[0]
      c0: 6e74de20     	fmul.2d	v0, v17, v20
      c4: 4ef5d410     	fsub.2d	v16, v0, v21
      c8: 4ea11c31     	mov.16b	v17, v1
      cc: 4ea11c32     	mov.16b	v18, v1
      d0: 4ea11c33     	mov.16b	v19, v1
      d4: 4ea11c34     	mov.16b	v20, v1
      d8: 4ea11c35     	mov.16b	v21, v1
      dc: 4ea11c37     	mov.16b	v23, v1
      e0: 4ea11c3a     	mov.16b	v26, v1
      e4: 4ea11c3b     	mov.16b	v27, v1
      e8: d2e80920     	mov	x0, #0x4049000000000000 ; =4632233691727265792
      ec: 9e670016     	fmov	d22, x0
      f0: 1e6e101f     	fmov	d31, #1.00000000
      f4: 0f000419     	movi.2s	v25, #0x0
      f8: 3cc003e0     	ldur	q0, [sp]
      fc: 1e762000     	fcmp	d0, d22
     100: 1e79afe8     	fcsel	d8, d31, d25, ge
     104: 6e74de9e     	fmul.2d	v30, v20, v20
     108: 4ebe1fc0     	mov.16b	v0, v30
     10c: 4e7bcf60     	fmla.2d	v0, v27, v27
     110: 3cc103e2     	ldur	q2, [sp, #0x10]
     114: 6e60e449     	fcmge.2d	v9, v2, v0
     118: 4eb81f02     	mov.16b	v2, v24
     11c: 6e611c49     	bsl.16b	v9, v2, v1
     120: 5e18052a     	mov	d10, v9[1]
     124: 1e6a4936     	fmax	d22, d9, d10
     128: 6e73de7d     	fmul.2d	v29, v19, v19
     12c: 4ebd1fa0     	mov.16b	v0, v29
     130: 4e7acf40     	fmla.2d	v0, v26, v26
     134: 3cc103f8     	ldur	q24, [sp, #0x10]
     138: 6e60e70b     	fcmge.2d	v11, v24, v0
     13c: 6e611c4b     	bsl.16b	v11, v2, v1
     140: 1e6b4ad6     	fmax	d22, d22, d11
     144: 5e18056c     	mov	d12, v11[1]
     148: 1e6c4ad6     	fmax	d22, d22, d12
     14c: 6e72de5c     	fmul.2d	v28, v18, v18
     150: 4ebc1f80     	mov.16b	v0, v28
     154: 4e77cee0     	fmla.2d	v0, v23, v23
     158: 6e60e70d     	fcmge.2d	v13, v24, v0
     15c: 6e611c4d     	bsl.16b	v13, v2, v1
     160: 1e6d4ad6     	fmax	d22, d22, d13
     164: 5e1805ae     	mov	d14, v13[1]
     168: 1e6e4ad6     	fmax	d22, d22, d14
     16c: 6e71de38     	fmul.2d	v24, v17, v17
     170: 4eb81f00     	mov.16b	v0, v24
     174: 4e75cea0     	fmla.2d	v0, v21, v21
     178: 3cc103ef     	ldur	q15, [sp, #0x10]
     17c: 6e60e5ef     	fcmge.2d	v15, v15, v0
     180: 6e611c4f     	bsl.16b	v15, v2, v1
     184: 1e6f4ac0     	fmax	d0, d22, d15
     188: 5e1805f6     	mov	d22, v15[1]
     18c: 1e764800     	fmax	d0, d0, d22
     190: 1e792000     	fcmp	d0, d25
     194: 1e790fe0     	fcsel	d0, d31, d25, eq
     198: 1e604900     	fmax	d0, d8, d0
     19c: 1e792000     	fcmp	d0, d25
     1a0: 54000f61     	b.ne	0x38c <_mech_map_batch+0x38c>
     1a4: 4efbcf7e     	fmls.2d	v30, v27, v27
     1a8: 4e64d7d6     	fadd.2d	v22, v30, v4
     1ac: 6e76dc79     	fmul.2d	v25, v3, v22
     1b0: 6e74dc60     	fmul.2d	v0, v3, v20
     1b4: 4ea51cbe     	mov.16b	v30, v5
     1b8: 4e7bcc1e     	fmla.2d	v30, v0, v27
     1bc: 4ea51cb4     	mov.16b	v20, v5
     1c0: 4e7ecf34     	fmla.2d	v20, v25, v30
     1c4: 6e76dec0     	fmul.2d	v0, v22, v22
     1c8: 4efecfc0     	fmls.2d	v0, v30, v30
     1cc: 4e64d416     	fadd.2d	v22, v0, v4
     1d0: 6e76dec0     	fmul.2d	v0, v22, v22
     1d4: 4ef4ce80     	fmls.2d	v0, v20, v20
     1d8: 4e64d41b     	fadd.2d	v27, v0, v4
     1dc: 6e7bdc7e     	fmul.2d	v30, v3, v27
     1e0: 6e76dc60     	fmul.2d	v0, v3, v22
     1e4: 4ea51cb6     	mov.16b	v22, v5
     1e8: 4e74cc16     	fmla.2d	v22, v0, v20
     1ec: 4ea51cb9     	mov.16b	v25, v5
     1f0: 4e76cfd9     	fmla.2d	v25, v30, v22
     1f4: 6e7bdf60     	fmul.2d	v0, v27, v27
     1f8: 4ef6cec0     	fmls.2d	v0, v22, v22
     1fc: 4e64d41b     	fadd.2d	v27, v0, v4
     200: 6e7bdf60     	fmul.2d	v0, v27, v27
     204: 4ef9cf20     	fmls.2d	v0, v25, v25
     208: 4e64d414     	fadd.2d	v20, v0, v4
     20c: 4efacf5d     	fmls.2d	v29, v26, v26
     210: 4e66d7b6     	fadd.2d	v22, v29, v6
     214: 6e76dc7d     	fmul.2d	v29, v3, v22
     218: 6e73dc60     	fmul.2d	v0, v3, v19
     21c: 4ea51cbe     	mov.16b	v30, v5
     220: 4e7acc1e     	fmla.2d	v30, v0, v26
     224: 4ea51cb3     	mov.16b	v19, v5
     228: 4e7ecfb3     	fmla.2d	v19, v29, v30
     22c: 6e76dec0     	fmul.2d	v0, v22, v22
     230: 4efecfc0     	fmls.2d	v0, v30, v30
     234: 4e66d416     	fadd.2d	v22, v0, v6
     238: 6e76dec0     	fmul.2d	v0, v22, v22
     23c: 4ef3ce60     	fmls.2d	v0, v19, v19
     240: 4e66d41a     	fadd.2d	v26, v0, v6
     244: 6e7adc7e     	fmul.2d	v30, v3, v26
     248: 6e76dc60     	fmul.2d	v0, v3, v22
     24c: 4ea51cb6     	mov.16b	v22, v5
     250: 4e73cc16     	fmla.2d	v22, v0, v19
     254: 4ea51cbd     	mov.16b	v29, v5
     258: 4e76cfdd     	fmla.2d	v29, v30, v22
     25c: 6e7adf40     	fmul.2d	v0, v26, v26
     260: 4ef6cec0     	fmls.2d	v0, v22, v22
     264: 4e66d41a     	fadd.2d	v26, v0, v6
     268: 6e7adf40     	fmul.2d	v0, v26, v26
     26c: 4efdcfa0     	fmls.2d	v0, v29, v29
     270: 4e66d413     	fadd.2d	v19, v0, v6
     274: 4ef7cefc     	fmls.2d	v28, v23, v23
     278: 4e67d796     	fadd.2d	v22, v28, v7
     27c: 6e76dc7c     	fmul.2d	v28, v3, v22
     280: 6e72dc60     	fmul.2d	v0, v3, v18
     284: 4ea51cbe     	mov.16b	v30, v5
     288: 4e77cc1e     	fmla.2d	v30, v0, v23
     28c: 4ea51cb2     	mov.16b	v18, v5
     290: 4e7ecf92     	fmla.2d	v18, v28, v30
     294: 6e76dec0     	fmul.2d	v0, v22, v22
     298: 4efecfc0     	fmls.2d	v0, v30, v30
     29c: 4e67d416     	fadd.2d	v22, v0, v7
     2a0: 6e76dec0     	fmul.2d	v0, v22, v22
     2a4: 4ef2ce40     	fmls.2d	v0, v18, v18
     2a8: 4e67d417     	fadd.2d	v23, v0, v7
     2ac: 6e77dc7e     	fmul.2d	v30, v3, v23
     2b0: 6e76dc60     	fmul.2d	v0, v3, v22
     2b4: 4ea51cb6     	mov.16b	v22, v5
     2b8: 4e72cc16     	fmla.2d	v22, v0, v18
     2bc: 4ea51cbc     	mov.16b	v28, v5
     2c0: 4e76cfdc     	fmla.2d	v28, v30, v22
     2c4: 6e77dee0     	fmul.2d	v0, v23, v23
     2c8: 4ef6cec0     	fmls.2d	v0, v22, v22
     2cc: 4e67d417     	fadd.2d	v23, v0, v7
     2d0: 6e77dee0     	fmul.2d	v0, v23, v23
     2d4: 4efccf80     	fmls.2d	v0, v28, v28
     2d8: 4e67d412     	fadd.2d	v18, v0, v7
     2dc: 4ef5ceb8     	fmls.2d	v24, v21, v21
     2e0: 4e70d716     	fadd.2d	v22, v24, v16
     2e4: 6e76dc78     	fmul.2d	v24, v3, v22
     2e8: 6e71dc60     	fmul.2d	v0, v3, v17
     2ec: 4ea51cbe     	mov.16b	v30, v5
     2f0: 4e75cc1e     	fmla.2d	v30, v0, v21
     2f4: 4ea51cb1     	mov.16b	v17, v5
     2f8: 4e7ecf11     	fmla.2d	v17, v24, v30
     2fc: 6e76dec0     	fmul.2d	v0, v22, v22
     300: 4efecfc0     	fmls.2d	v0, v30, v30
     304: 4e70d415     	fadd.2d	v21, v0, v16
     308: 6e75dea0     	fmul.2d	v0, v21, v21
     30c: 4ef1ce20     	fmls.2d	v0, v17, v17
     310: 4e70d416     	fadd.2d	v22, v0, v16
     314: 6e76dc7e     	fmul.2d	v30, v3, v22
     318: 6e75dc60     	fmul.2d	v0, v3, v21
     31c: 4ea51cb5     	mov.16b	v21, v5
     320: 4e71cc15     	fmla.2d	v21, v0, v17
     324: 4ea51cb8     	mov.16b	v24, v5
     328: 4e75cfd8     	fmla.2d	v24, v30, v21
     32c: 6e76dec0     	fmul.2d	v0, v22, v22
     330: 4ef5cea0     	fmls.2d	v0, v21, v21
     334: 4e70d415     	fadd.2d	v21, v0, v16
     338: 6e75dea0     	fmul.2d	v0, v21, v21
     33c: 4ef8cf00     	fmls.2d	v0, v24, v24
     340: 4e70d411     	fadd.2d	v17, v0, v16
     344: 6e7bdc60     	fmul.2d	v0, v3, v27
     348: 4ea51cbb     	mov.16b	v27, v5
     34c: 4e79cc1b     	fmla.2d	v27, v0, v25
     350: 6e7adc60     	fmul.2d	v0, v3, v26
     354: 4ea51cba     	mov.16b	v26, v5
     358: 4e7dcc1a     	fmla.2d	v26, v0, v29
     35c: 6e77dc60     	fmul.2d	v0, v3, v23
     360: 4ea51cb7     	mov.16b	v23, v5
     364: 4e7ccc17     	fmla.2d	v23, v0, v28
     368: 6e75dc60     	fmul.2d	v0, v3, v21
     36c: 4ea51cb5     	mov.16b	v21, v5
     370: 4e78cc15     	fmla.2d	v21, v0, v24
     374: 1e629000     	fmov	d0, #5.00000000
     378: 3cc003f6     	ldur	q22, [sp]
     37c: 1e602ad6     	fadd	d22, d22, d0
     380: 4ea21c58     	mov.16b	v24, v2
     384: 3c8003f6     	stur	q22, [sp]
     388: 17ffff58     	b	0xe8 <_mech_map_batch+0xe8>
     38c: fd000029     	str	d9, [x1]
     390: fd00042a     	str	d10, [x1, #0x8]
     394: fd00082b     	str	d11, [x1, #0x10]
     398: fd000c2c     	str	d12, [x1, #0x18]
     39c: fd00102d     	str	d13, [x1, #0x20]
     3a0: fd00142e     	str	d14, [x1, #0x28]
     3a4: fd00182f     	str	d15, [x1, #0x30]
     3a8: fd001c36     	str	d22, [x1, #0x38]
     3ac: 910083ff     	add	sp, sp, #0x20
     3b0: 6cc127e8     	ldp	d8, d9, [sp], #0x10
     3b4: 6cc12fea     	ldp	d10, d11, [sp], #0x10
     3b8: 6cc137ec     	ldp	d12, d13, [sp], #0x10
     3bc: 6cc13fee     	ldp	d14, d15, [sp], #0x10
     3c0: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     3c4: d65f0fff     	retab
