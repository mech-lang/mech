
benchmarks/clbg/mandelbrot/experiments/20261011-array-state/validation/lanes-rows.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000000000 <_mech_map_bits>:
       0: d503237f     	pacibsp
       4: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
       8: 910003fd     	mov	x29, sp
       c: 6dbf2fea     	stp	d10, d11, [sp, #-0x10]!
      10: 6dbf27e8     	stp	d8, d9, [sp, #-0x10]!
      14: fd400001     	ldr	d1, [x0]
      18: fd400413     	ldr	d19, [x0, #0x8]
      1c: fd400812     	ldr	d18, [x0, #0x10]
      20: fd400c11     	ldr	d17, [x0, #0x18]
      24: fd401002     	ldr	d2, [x0, #0x20]
      28: fd401407     	ldr	d7, [x0, #0x28]
      2c: fd401803     	ldr	d3, [x0, #0x30]
      30: fd401c06     	ldr	d6, [x0, #0x38]
      34: fd402004     	ldr	d4, [x0, #0x40]
      38: fd402405     	ldr	d5, [x0, #0x48]
      3c: 4f00e410     	movi.16b	v16, #0x0
      40: 6f00f600     	fmov.2d	v0, #4.00000000
      44: 1e604274     	fmov	d20, d19
      48: 6e180654     	mov.d	v20[1], v18[0]
      4c: 1e601012     	fmov	d18, #2.00000000
      50: 1e611a52     	fdiv	d18, d18, d1
      54: 4e080653     	dup.2d	v19, v18[0]
      58: 6e73de81     	fmul.2d	v1, v20, v19
      5c: 6f03f714     	fmov.2d	v20, #1.50000000
      60: 4ef4d421     	fsub.2d	v1, v1, v20
      64: 1e604231     	fmov	d17, d17
      68: 6e180451     	mov.d	v17[1], v2[0]
      6c: 6e73de22     	fmul.2d	v2, v17, v19
      70: 4ef4d442     	fsub.2d	v2, v2, v20
      74: 1e6040e7     	fmov	d7, d7
      78: 6e180467     	mov.d	v7[1], v3[0]
      7c: 6e73dce3     	fmul.2d	v3, v7, v19
      80: 4ef4d463     	fsub.2d	v3, v3, v20
      84: 1e6040c6     	fmov	d6, d6
      88: 6e180486     	mov.d	v6[1], v4[0]
      8c: 6e73dcc4     	fmul.2d	v4, v6, v19
      90: 4ef4d484     	fsub.2d	v4, v4, v20
      94: 1e7208a5     	fmul	d5, d5, d18
      98: 1e6e1006     	fmov	d6, #1.00000000
      9c: 1e6638a5     	fsub	d5, d5, d6
      a0: 4e0804a5     	dup.2d	v5, v5[0]
      a4: 6f03f606     	fmov.2d	v6, #1.00000000
      a8: 4eb01e07     	mov.16b	v7, v16
      ac: 4eb01e11     	mov.16b	v17, v16
      b0: 4eb01e12     	mov.16b	v18, v16
      b4: 4eb01e13     	mov.16b	v19, v16
      b8: 4eb01e14     	mov.16b	v20, v16
      bc: 4eb01e15     	mov.16b	v21, v16
      c0: 4eb01e16     	mov.16b	v22, v16
      c4: 4eb01e17     	mov.16b	v23, v16
      c8: 4eb01e18     	mov.16b	v24, v16
      cc: 4eb01e19     	mov.16b	v25, v16
      d0: 4eb01e1a     	mov.16b	v26, v16
      d4: d2e80920     	mov	x0, #0x4049000000000000 ; =4632233691727265792
      d8: 9e67001b     	fmov	d27, x0
      dc: 92800001     	mov	x1, #-0x1               ; =-1
      e0: d2800002     	mov	x2, #0x0                ; =0
      e4: 1e7b2260     	fcmp	d19, d27
      e8: 9a82a022     	csel	x2, x1, x2, ge
      ec: 6e70de1b     	fmul.2d	v27, v16, v16
      f0: 6e77defc     	fmul.2d	v28, v23, v23
      f4: 4e7cd77d     	fadd.2d	v29, v27, v28
      f8: 6e7de41d     	fcmge.2d	v29, v0, v29
      fc: 4e083fa3     	mov.d	x3, v29[0]
     100: 4e183fa4     	mov.d	x4, v29[1]
     104: aa040066     	orr	x6, x3, x4
     108: 6e7adf5d     	fmul.2d	v29, v26, v26
     10c: 6e76dede     	fmul.2d	v30, v22, v22
     110: 4e7ed7bf     	fadd.2d	v31, v29, v30
     114: 6e7fe41f     	fcmge.2d	v31, v0, v31
     118: 4e083fe5     	mov.d	x5, v31[0]
     11c: aa0500c7     	orr	x7, x6, x5
     120: 4e183fe6     	mov.d	x6, v31[1]
     124: aa0600e8     	orr	x8, x7, x6
     128: 6e79df3f     	fmul.2d	v31, v25, v25
     12c: 6e75dea8     	fmul.2d	v8, v21, v21
     130: 4e68d7e9     	fadd.2d	v9, v31, v8
     134: 6e69e409     	fcmge.2d	v9, v0, v9
     138: 4e083d27     	mov.d	x7, v9[0]
     13c: aa070109     	orr	x9, x8, x7
     140: 4e183d28     	mov.d	x8, v9[1]
     144: aa08012a     	orr	x10, x9, x8
     148: 6e78df09     	fmul.2d	v9, v24, v24
     14c: 6e74de8a     	fmul.2d	v10, v20, v20
     150: 4e6ad52b     	fadd.2d	v11, v9, v10
     154: 6e6be40b     	fcmge.2d	v11, v0, v11
     158: 4e083d69     	mov.d	x9, v11[0]
     15c: aa09014b     	orr	x11, x10, x9
     160: 4e183d6a     	mov.d	x10, v11[1]
     164: aa0a016b     	orr	x11, x11, x10
     168: aa2b0042     	orn	x2, x2, x11
     16c: f100005f     	cmp	x2, #0x0
     170: 540003c1     	b.ne	0x1e8 <_mech_map_bits+0x1e8>
     174: 4efcd77b     	fsub.2d	v27, v27, v28
     178: 4e61d77b     	fadd.2d	v27, v27, v1
     17c: 4efed7bc     	fsub.2d	v28, v29, v30
     180: 4e62d79c     	fadd.2d	v28, v28, v2
     184: 4ee8d7fd     	fsub.2d	v29, v31, v8
     188: 4e63d7bd     	fadd.2d	v29, v29, v3
     18c: 4eead53e     	fsub.2d	v30, v9, v10
     190: 4e64d7de     	fadd.2d	v30, v30, v4
     194: 4e70d610     	fadd.2d	v16, v16, v16
     198: 6e77de10     	fmul.2d	v16, v16, v23
     19c: 4e65d617     	fadd.2d	v23, v16, v5
     1a0: 4e7ad750     	fadd.2d	v16, v26, v26
     1a4: 6e76de10     	fmul.2d	v16, v16, v22
     1a8: 4e65d616     	fadd.2d	v22, v16, v5
     1ac: 4e79d730     	fadd.2d	v16, v25, v25
     1b0: 6e75de10     	fmul.2d	v16, v16, v21
     1b4: 4e65d615     	fadd.2d	v21, v16, v5
     1b8: 4e78d710     	fadd.2d	v16, v24, v24
     1bc: 6e74de10     	fmul.2d	v16, v16, v20
     1c0: 4e65d614     	fadd.2d	v20, v16, v5
     1c4: 4e66d673     	fadd.2d	v19, v19, v6
     1c8: 4e66d652     	fadd.2d	v18, v18, v6
     1cc: 4e66d631     	fadd.2d	v17, v17, v6
     1d0: 4e66d4e7     	fadd.2d	v7, v7, v6
     1d4: 4ebb1f70     	mov.16b	v16, v27
     1d8: 4ebe1fd8     	mov.16b	v24, v30
     1dc: 4ebd1fb9     	mov.16b	v25, v29
     1e0: 4ebc1f9a     	mov.16b	v26, v28
     1e4: 17ffffbc     	b	0xd4 <_mech_map_bits+0xd4>
     1e8: 9279006b     	and	x11, x3, #0x80
     1ec: 927a008c     	and	x12, x4, #0x40
     1f0: aa0c016b     	orr	x11, x11, x12
     1f4: 927b00a5     	and	x5, x5, #0x20
     1f8: aa050165     	orr	x5, x11, x5
     1fc: 927c00c6     	and	x6, x6, #0x10
     200: aa0600a5     	orr	x5, x5, x6
     204: 927d00e6     	and	x6, x7, #0x8
     208: aa0600a5     	orr	x5, x5, x6
     20c: 927e0106     	and	x6, x8, #0x4
     210: aa0600a5     	orr	x5, x5, x6
     214: 927f0126     	and	x6, x9, #0x2
     218: aa0600a5     	orr	x5, x5, x6
     21c: 92400146     	and	x6, x10, #0x1
     220: aa0600a0     	orr	x0, x5, x6
     224: 6cc127e8     	ldp	d8, d9, [sp], #0x10
     228: 6cc12fea     	ldp	d10, d11, [sp], #0x10
     22c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     230: d65f0fff     	retab

0000000000000234 <_mech_grid_row>:
     234: d503237f     	pacibsp
     238: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     23c: 910003fd     	mov	x29, sp
     240: a9bf5bf5     	stp	x21, x22, [sp, #-0x10]!
     244: a9bf53f3     	stp	x19, x20, [sp, #-0x10]!
     248: d10143ff     	sub	sp, sp, #0x50
     24c: aa0203f5     	mov	x21, x2
     250: 9e630000     	ucvtf	d0, x0
     254: 910003f3     	mov	x19, sp
     258: fc0003e0     	stur	d0, [sp]
     25c: 9e630020     	ucvtf	d0, x1
     260: fc0483e0     	stur	d0, [sp, #0x48]
     264: d2800014     	mov	x20, #0x0               ; =0
     268: f100001f     	cmp	x0, #0x0
     26c: aa0003f6     	mov	x22, x0
     270: 540005a0     	b.eq	0x324 <_mech_grid_row+0xf0>
     274: 9e630280     	ucvtf	d0, x20
     278: fc0083e0     	stur	d0, [sp, #0x8]
     27c: 91000680     	add	x0, x20, #0x1
     280: 9e630000     	ucvtf	d0, x0
     284: fc0103e0     	stur	d0, [sp, #0x10]
     288: 91000a80     	add	x0, x20, #0x2
     28c: 9e630000     	ucvtf	d0, x0
     290: fc0183e0     	stur	d0, [sp, #0x18]
     294: 91000e80     	add	x0, x20, #0x3
     298: 9e630000     	ucvtf	d0, x0
     29c: fc0203e0     	stur	d0, [sp, #0x20]
     2a0: 91001280     	add	x0, x20, #0x4
     2a4: 9e630000     	ucvtf	d0, x0
     2a8: fc0283e0     	stur	d0, [sp, #0x28]
     2ac: 91001680     	add	x0, x20, #0x5
     2b0: 9e630000     	ucvtf	d0, x0
     2b4: fc0303e0     	stur	d0, [sp, #0x30]
     2b8: 91001a80     	add	x0, x20, #0x6
     2bc: 9e630000     	ucvtf	d0, x0
     2c0: fc0383e0     	stur	d0, [sp, #0x38]
     2c4: 91001e80     	add	x0, x20, #0x7
     2c8: 9e630000     	ucvtf	d0, x0
     2cc: fc0403e0     	stur	d0, [sp, #0x40]
     2d0: aa1303e0     	mov	x0, x19
     2d4: 94000000     	bl	0x2d4 <_mech_grid_row+0xa0>
     2d8: aa1603e8     	mov	x8, x22
     2dc: cb140101     	sub	x1, x8, x20
     2e0: d2800102     	mov	x2, #0x8                ; =8
     2e4: cb010042     	sub	x2, x2, x1
     2e8: d2800003     	mov	x3, #0x0                ; =0
     2ec: f100203f     	cmp	x1, #0x8
     2f0: 9a833041     	csel	x1, x2, x3, lo
     2f4: d2801fe2     	mov	x2, #0xff               ; =255
     2f8: 9ac12041     	lsl	x1, x2, x1
     2fc: 8a010000     	and	x0, x0, x1
     300: d343fe81     	lsr	x1, x20, #3
     304: aa1503e2     	mov	x2, x21
     308: 38216840     	strb	w0, [x2, x1]
     30c: 91002294     	add	x20, x20, #0x8
     310: eb08029f     	cmp	x20, x8
     314: 54000082     	b.hs	0x324 <_mech_grid_row+0xf0>
     318: aa0203f5     	mov	x21, x2
     31c: aa0803f6     	mov	x22, x8
     320: 17ffffd5     	b	0x274 <_mech_grid_row+0x40>
     324: 910143ff     	add	sp, sp, #0x50
     328: a8c153f3     	ldp	x19, x20, [sp], #0x10
     32c: a8c15bf5     	ldp	x21, x22, [sp], #0x10
     330: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     334: d65f0fff     	retab
