
/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/mandelbrot/experiments/20261011-array-state/validation-current/five16.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000000000 <_mech_map_bits>:
       0: fd400006     	ldr	d6, [x0]
       4: fd400418     	ldr	d24, [x0, #0x8]
       8: fd400807     	ldr	d7, [x0, #0x10]
       c: fd400c17     	ldr	d23, [x0, #0x18]
      10: fd401010     	ldr	d16, [x0, #0x20]
      14: fd401416     	ldr	d22, [x0, #0x28]
      18: fd401811     	ldr	d17, [x0, #0x30]
      1c: fd401c14     	ldr	d20, [x0, #0x38]
      20: fd402012     	ldr	d18, [x0, #0x40]
      24: fd40241a     	ldr	d26, [x0, #0x48]
      28: 4f00e413     	movi.16b	v19, #0x0
      2c: 0f000415     	movi.2s	v21, #0x0
      30: 6f00f605     	fmov.2d	v5, #4.00000000
      34: 1e604304     	fmov	d4, d24
      38: 6e1804e4     	mov.d	v4[1], v7[0]
      3c: 1e601007     	fmov	d7, #2.00000000
      40: 1e6618e7     	fdiv	d7, d7, d6
      44: 4e0804f8     	dup.2d	v24, v7[0]
      48: 6e78dc86     	fmul.2d	v6, v4, v24
      4c: 6f03f719     	fmov.2d	v25, #1.50000000
      50: 4ef9d4c6     	fsub.2d	v6, v6, v25
      54: 1e670b47     	fmul	d7, d26, d7
      58: 1e6e101a     	fmov	d26, #1.00000000
      5c: 1e7a38e7     	fsub	d7, d7, d26
      60: 4e0804e7     	dup.2d	v7, v7[0]
      64: 1e6042e4     	fmov	d4, d23
      68: 6e180604     	mov.d	v4[1], v16[0]
      6c: 6e78dc90     	fmul.2d	v16, v4, v24
      70: 4ef9d610     	fsub.2d	v16, v16, v25
      74: 1e6042c4     	fmov	d4, d22
      78: 6e180624     	mov.d	v4[1], v17[0]
      7c: 6e78dc91     	fmul.2d	v17, v4, v24
      80: 4ef9d631     	fsub.2d	v17, v17, v25
      84: 1e604284     	fmov	d4, d20
      88: 6e180644     	mov.d	v4[1], v18[0]
      8c: 6e78dc92     	fmul.2d	v18, v4, v24
      90: 4ef9d652     	fsub.2d	v18, v18, v25
      94: 4eb31e60     	mov.16b	v0, v19
      98: 4eb31e61     	mov.16b	v1, v19
      9c: 4eb31e62     	mov.16b	v2, v19
      a0: 4eb31e63     	mov.16b	v3, v19
      a4: 4eb31e64     	mov.16b	v4, v19
      a8: 4eb31e74     	mov.16b	v20, v19
      ac: 4eb31e76     	mov.16b	v22, v19
      b0: d2e80925     	mov	x5, #0x4049000000000000 ; =4632233691727265792
      b4: 9e6700b7     	fmov	d23, x5
      b8: 92800006     	mov	x6, #-0x1               ; =-1
      bc: d2800007     	mov	x7, #0x0                ; =0
      c0: 1e7722a0     	fcmp	d21, d23
      c4: 9a87a0c7     	csel	x7, x6, x7, ge
      c8: 6e73de77     	fmul.2d	v23, v19, v19
      cc: 6e76ded8     	fmul.2d	v24, v22, v22
      d0: 4ed81afd     	uzp1.2d	v29, v23, v24
      d4: 4ed85afe     	uzp2.2d	v30, v23, v24
      d8: 4e7ed7b7     	fadd.2d	v23, v29, v30
      dc: 6e77e4b7     	fcmge.2d	v23, v5, v23
      e0: 4e083ee8     	mov.d	x8, v23[0]
      e4: 4e183ee9     	mov.d	x9, v23[1]
      e8: aa09010b     	orr	x11, x8, x9
      ec: 6e62dc57     	fmul.2d	v23, v2, v2
      f0: 6e74de98     	fmul.2d	v24, v20, v20
      f4: 4ed81afb     	uzp1.2d	v27, v23, v24
      f8: 4ed85afc     	uzp2.2d	v28, v23, v24
      fc: 4e7cd777     	fadd.2d	v23, v27, v28
     100: 6e77e4b7     	fcmge.2d	v23, v5, v23
     104: 4e083eea     	mov.d	x10, v23[0]
     108: aa0a016c     	orr	x12, x11, x10
     10c: 4e183eeb     	mov.d	x11, v23[1]
     110: aa0b018d     	orr	x13, x12, x11
     114: 6e61dc37     	fmul.2d	v23, v1, v1
     118: 6e64dc98     	fmul.2d	v24, v4, v4
     11c: 4ed81af9     	uzp1.2d	v25, v23, v24
     120: 4ed85afa     	uzp2.2d	v26, v23, v24
     124: 4e7ad737     	fadd.2d	v23, v25, v26
     128: 6e77e4b7     	fcmge.2d	v23, v5, v23
     12c: 4e083eec     	mov.d	x12, v23[0]
     130: aa0c01ae     	orr	x14, x13, x12
     134: 4e183eed     	mov.d	x13, v23[1]
     138: aa0d01cf     	orr	x15, x14, x13
     13c: 6e60dc18     	fmul.2d	v24, v0, v0
     140: 6e63dc7f     	fmul.2d	v31, v3, v3
     144: 4edf1b17     	uzp1.2d	v23, v24, v31
     148: 4edf5b18     	uzp2.2d	v24, v24, v31
     14c: 4e78d6ff     	fadd.2d	v31, v23, v24
     150: 6e7fe4bf     	fcmge.2d	v31, v5, v31
     154: 4e083fee     	mov.d	x14, v31[0]
     158: aa0e01e0     	orr	x0, x15, x14
     15c: 4e183fef     	mov.d	x15, v31[1]
     160: aa0f0000     	orr	x0, x0, x15
     164: aa2000e7     	orn	x7, x7, x0
     168: f10000ff     	cmp	x7, #0x0
     16c: 54001301     	b.ne	0x3cc <_mech_map_bits+0x3cc>
     170: 4efed7bd     	fsub.2d	v29, v29, v30
     174: 4e66d7bd     	fadd.2d	v29, v29, v6
     178: 6e7ddfbe     	fmul.2d	v30, v29, v29
     17c: 4ed61a7f     	uzp1.2d	v31, v19, v22
     180: 4e7fd7ff     	fadd.2d	v31, v31, v31
     184: 4ed65a73     	uzp2.2d	v19, v19, v22
     188: 6e73dff3     	fmul.2d	v19, v31, v19
     18c: 4e67d676     	fadd.2d	v22, v19, v7
     190: 6e76ded3     	fmul.2d	v19, v22, v22
     194: 4ef3d7d3     	fsub.2d	v19, v30, v19
     198: 4e66d673     	fadd.2d	v19, v19, v6
     19c: 6e73de7e     	fmul.2d	v30, v19, v19
     1a0: 4e7dd7bd     	fadd.2d	v29, v29, v29
     1a4: 6e76dfb6     	fmul.2d	v22, v29, v22
     1a8: 4e67d6dd     	fadd.2d	v29, v22, v7
     1ac: 6e7ddfb6     	fmul.2d	v22, v29, v29
     1b0: 4ef6d7d6     	fsub.2d	v22, v30, v22
     1b4: 4e66d6d6     	fadd.2d	v22, v22, v6
     1b8: 6e76dede     	fmul.2d	v30, v22, v22
     1bc: 4e73d673     	fadd.2d	v19, v19, v19
     1c0: 6e7dde73     	fmul.2d	v19, v19, v29
     1c4: 4e67d67d     	fadd.2d	v29, v19, v7
     1c8: 6e7ddfb3     	fmul.2d	v19, v29, v29
     1cc: 4ef3d7d3     	fsub.2d	v19, v30, v19
     1d0: 4e66d673     	fadd.2d	v19, v19, v6
     1d4: 6e73de7e     	fmul.2d	v30, v19, v19
     1d8: 4e76d6d6     	fadd.2d	v22, v22, v22
     1dc: 6e7dded6     	fmul.2d	v22, v22, v29
     1e0: 4e67d6d6     	fadd.2d	v22, v22, v7
     1e4: 6e76dedd     	fmul.2d	v29, v22, v22
     1e8: 4efdd7dd     	fsub.2d	v29, v30, v29
     1ec: 4e66d7bd     	fadd.2d	v29, v29, v6
     1f0: 4e73d673     	fadd.2d	v19, v19, v19
     1f4: 6e76de73     	fmul.2d	v19, v19, v22
     1f8: 4e67d676     	fadd.2d	v22, v19, v7
     1fc: 4ed61bb3     	uzp1.2d	v19, v29, v22
     200: 4ed65bb6     	uzp2.2d	v22, v29, v22
     204: 4efcd77b     	fsub.2d	v27, v27, v28
     208: 4e70d77b     	fadd.2d	v27, v27, v16
     20c: 6e7bdf7c     	fmul.2d	v28, v27, v27
     210: 4ed4185d     	uzp1.2d	v29, v2, v20
     214: 4e7dd7bd     	fadd.2d	v29, v29, v29
     218: 4ed45854     	uzp2.2d	v20, v2, v20
     21c: 6e74dfb4     	fmul.2d	v20, v29, v20
     220: 4e67d69d     	fadd.2d	v29, v20, v7
     224: 6e7ddfb4     	fmul.2d	v20, v29, v29
     228: 4ef4d794     	fsub.2d	v20, v28, v20
     22c: 4e70d694     	fadd.2d	v20, v20, v16
     230: 6e74de9e     	fmul.2d	v30, v20, v20
     234: 4e7bd77b     	fadd.2d	v27, v27, v27
     238: 6e7ddf7b     	fmul.2d	v27, v27, v29
     23c: 4e67d77c     	fadd.2d	v28, v27, v7
     240: 6e7cdf9b     	fmul.2d	v27, v28, v28
     244: 4efbd7db     	fsub.2d	v27, v30, v27
     248: 4e70d77b     	fadd.2d	v27, v27, v16
     24c: 6e7bdf7d     	fmul.2d	v29, v27, v27
     250: 4e74d694     	fadd.2d	v20, v20, v20
     254: 6e7cde94     	fmul.2d	v20, v20, v28
     258: 4e67d69c     	fadd.2d	v28, v20, v7
     25c: 6e7cdf94     	fmul.2d	v20, v28, v28
     260: 4ef4d7b4     	fsub.2d	v20, v29, v20
     264: 4e70d694     	fadd.2d	v20, v20, v16
     268: 6e74de9d     	fmul.2d	v29, v20, v20
     26c: 4e7bd77b     	fadd.2d	v27, v27, v27
     270: 6e7cdf7b     	fmul.2d	v27, v27, v28
     274: 4e67d77b     	fadd.2d	v27, v27, v7
     278: 6e7bdf7c     	fmul.2d	v28, v27, v27
     27c: 4efcd7bc     	fsub.2d	v28, v29, v28
     280: 4e70d79c     	fadd.2d	v28, v28, v16
     284: 4e74d694     	fadd.2d	v20, v20, v20
     288: 6e7bde94     	fmul.2d	v20, v20, v27
     28c: 4e67d694     	fadd.2d	v20, v20, v7
     290: 4ed41b82     	uzp1.2d	v2, v28, v20
     294: 4ed45b94     	uzp2.2d	v20, v28, v20
     298: 4efad739     	fsub.2d	v25, v25, v26
     29c: 4e71d739     	fadd.2d	v25, v25, v17
     2a0: 6e79df3a     	fmul.2d	v26, v25, v25
     2a4: 4ec4183b     	uzp1.2d	v27, v1, v4
     2a8: 4e7bd77b     	fadd.2d	v27, v27, v27
     2ac: 4ec4583c     	uzp2.2d	v28, v1, v4
     2b0: 6e7cdf7b     	fmul.2d	v27, v27, v28
     2b4: 4e67d77b     	fadd.2d	v27, v27, v7
     2b8: 6e7bdf7c     	fmul.2d	v28, v27, v27
     2bc: 4efcd75a     	fsub.2d	v26, v26, v28
     2c0: 4e71d75a     	fadd.2d	v26, v26, v17
     2c4: 6e7adf5c     	fmul.2d	v28, v26, v26
     2c8: 4e79d739     	fadd.2d	v25, v25, v25
     2cc: 6e7bdf39     	fmul.2d	v25, v25, v27
     2d0: 4e67d73b     	fadd.2d	v27, v25, v7
     2d4: 6e7bdf79     	fmul.2d	v25, v27, v27
     2d8: 4ef9d799     	fsub.2d	v25, v28, v25
     2dc: 4e71d739     	fadd.2d	v25, v25, v17
     2e0: 6e79df3c     	fmul.2d	v28, v25, v25
     2e4: 4e7ad75a     	fadd.2d	v26, v26, v26
     2e8: 6e7bdf5a     	fmul.2d	v26, v26, v27
     2ec: 4e67d75b     	fadd.2d	v27, v26, v7
     2f0: 6e7bdf7a     	fmul.2d	v26, v27, v27
     2f4: 4efad79a     	fsub.2d	v26, v28, v26
     2f8: 4e71d75a     	fadd.2d	v26, v26, v17
     2fc: 6e7adf5c     	fmul.2d	v28, v26, v26
     300: 4e79d739     	fadd.2d	v25, v25, v25
     304: 6e7bdf39     	fmul.2d	v25, v25, v27
     308: 4e67d739     	fadd.2d	v25, v25, v7
     30c: 6e79df3b     	fmul.2d	v27, v25, v25
     310: 4efbd79b     	fsub.2d	v27, v28, v27
     314: 4e71d77b     	fadd.2d	v27, v27, v17
     318: 4e7ad75a     	fadd.2d	v26, v26, v26
     31c: 6e79df59     	fmul.2d	v25, v26, v25
     320: 4e67d739     	fadd.2d	v25, v25, v7
     324: 4ed91b61     	uzp1.2d	v1, v27, v25
     328: 4ed95b64     	uzp2.2d	v4, v27, v25
     32c: 4ef8d6f7     	fsub.2d	v23, v23, v24
     330: 4e72d6f7     	fadd.2d	v23, v23, v18
     334: 6e77def8     	fmul.2d	v24, v23, v23
     338: 4ec31819     	uzp1.2d	v25, v0, v3
     33c: 4e79d739     	fadd.2d	v25, v25, v25
     340: 4ec3581a     	uzp2.2d	v26, v0, v3
     344: 6e7adf39     	fmul.2d	v25, v25, v26
     348: 4e67d739     	fadd.2d	v25, v25, v7
     34c: 6e79df3a     	fmul.2d	v26, v25, v25
     350: 4efad718     	fsub.2d	v24, v24, v26
     354: 4e72d718     	fadd.2d	v24, v24, v18
     358: 6e78df1a     	fmul.2d	v26, v24, v24
     35c: 4e77d6f7     	fadd.2d	v23, v23, v23
     360: 6e79def7     	fmul.2d	v23, v23, v25
     364: 4e67d6f9     	fadd.2d	v25, v23, v7
     368: 6e79df37     	fmul.2d	v23, v25, v25
     36c: 4ef7d757     	fsub.2d	v23, v26, v23
     370: 4e72d6f7     	fadd.2d	v23, v23, v18
     374: 6e77defa     	fmul.2d	v26, v23, v23
     378: 4e78d718     	fadd.2d	v24, v24, v24
     37c: 6e79df18     	fmul.2d	v24, v24, v25
     380: 4e67d719     	fadd.2d	v25, v24, v7
     384: 6e79df38     	fmul.2d	v24, v25, v25
     388: 4ef8d758     	fsub.2d	v24, v26, v24
     38c: 4e72d718     	fadd.2d	v24, v24, v18
     390: 6e78df1a     	fmul.2d	v26, v24, v24
     394: 4e77d6f7     	fadd.2d	v23, v23, v23
     398: 6e79def7     	fmul.2d	v23, v23, v25
     39c: 4e67d6f7     	fadd.2d	v23, v23, v7
     3a0: 6e77def9     	fmul.2d	v25, v23, v23
     3a4: 4ef9d759     	fsub.2d	v25, v26, v25
     3a8: 4e72d739     	fadd.2d	v25, v25, v18
     3ac: 4e78d718     	fadd.2d	v24, v24, v24
     3b0: 6e77df17     	fmul.2d	v23, v24, v23
     3b4: 4e67d6f7     	fadd.2d	v23, v23, v7
     3b8: 4ed71b20     	uzp1.2d	v0, v25, v23
     3bc: 4ed75b23     	uzp2.2d	v3, v25, v23
     3c0: 1e629017     	fmov	d23, #5.00000000
     3c4: 1e772ab5     	fadd	d21, d21, d23
     3c8: 17ffff3a     	b	0xb0 <_mech_map_bits+0xb0>
     3cc: 92790100     	and	x0, x8, #0x80
     3d0: 927a0121     	and	x1, x9, #0x40
     3d4: aa010000     	orr	x0, x0, x1
     3d8: 927b014a     	and	x10, x10, #0x20
     3dc: aa0a000a     	orr	x10, x0, x10
     3e0: 927c016b     	and	x11, x11, #0x10
     3e4: aa0b014a     	orr	x10, x10, x11
     3e8: 927d018b     	and	x11, x12, #0x8
     3ec: aa0b014a     	orr	x10, x10, x11
     3f0: 927e01ab     	and	x11, x13, #0x4
     3f4: aa0b014a     	orr	x10, x10, x11
     3f8: 927f01cb     	and	x11, x14, #0x2
     3fc: aa0b014a     	orr	x10, x10, x11
     400: 924001eb     	and	x11, x15, #0x1
     404: aa0b0140     	orr	x0, x10, x11
     408: d65f03c0     	ret

000000000000040c <_mech_grid_row>:
     40c: d503237f     	pacibsp
     410: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     414: 910003fd     	mov	x29, sp
     418: a9bf5bf5     	stp	x21, x22, [sp, #-0x10]!
     41c: a9bf53f3     	stp	x19, x20, [sp, #-0x10]!
     420: d10143ff     	sub	sp, sp, #0x50
     424: aa0203f5     	mov	x21, x2
     428: 9e630000     	ucvtf	d0, x0
     42c: 910003f3     	mov	x19, sp
     430: fc0003e0     	stur	d0, [sp]
     434: 9e630020     	ucvtf	d0, x1
     438: fc0483e0     	stur	d0, [sp, #0x48]
     43c: d2800014     	mov	x20, #0x0               ; =0
     440: f100001f     	cmp	x0, #0x0
     444: aa0003f6     	mov	x22, x0
     448: 540005a0     	b.eq	0x4fc <_mech_grid_row+0xf0>
     44c: 9e630280     	ucvtf	d0, x20
     450: fc0083e0     	stur	d0, [sp, #0x8]
     454: 91000680     	add	x0, x20, #0x1
     458: 9e630000     	ucvtf	d0, x0
     45c: fc0103e0     	stur	d0, [sp, #0x10]
     460: 91000a80     	add	x0, x20, #0x2
     464: 9e630000     	ucvtf	d0, x0
     468: fc0183e0     	stur	d0, [sp, #0x18]
     46c: 91000e80     	add	x0, x20, #0x3
     470: 9e630000     	ucvtf	d0, x0
     474: fc0203e0     	stur	d0, [sp, #0x20]
     478: 91001280     	add	x0, x20, #0x4
     47c: 9e630000     	ucvtf	d0, x0
     480: fc0283e0     	stur	d0, [sp, #0x28]
     484: 91001680     	add	x0, x20, #0x5
     488: 9e630000     	ucvtf	d0, x0
     48c: fc0303e0     	stur	d0, [sp, #0x30]
     490: 91001a80     	add	x0, x20, #0x6
     494: 9e630000     	ucvtf	d0, x0
     498: fc0383e0     	stur	d0, [sp, #0x38]
     49c: 91001e80     	add	x0, x20, #0x7
     4a0: 9e630000     	ucvtf	d0, x0
     4a4: fc0403e0     	stur	d0, [sp, #0x40]
     4a8: aa1303e0     	mov	x0, x19
     4ac: 94000000     	bl	0x4ac <_mech_grid_row+0xa0>
     4b0: aa1603e8     	mov	x8, x22
     4b4: cb140101     	sub	x1, x8, x20
     4b8: d2800102     	mov	x2, #0x8                ; =8
     4bc: cb010042     	sub	x2, x2, x1
     4c0: d2800003     	mov	x3, #0x0                ; =0
     4c4: f100203f     	cmp	x1, #0x8
     4c8: 9a833041     	csel	x1, x2, x3, lo
     4cc: d2801fe2     	mov	x2, #0xff               ; =255
     4d0: 9ac12041     	lsl	x1, x2, x1
     4d4: 8a010000     	and	x0, x0, x1
     4d8: d343fe81     	lsr	x1, x20, #3
     4dc: aa1503e2     	mov	x2, x21
     4e0: 38216840     	strb	w0, [x2, x1]
     4e4: 91002294     	add	x20, x20, #0x8
     4e8: eb08029f     	cmp	x20, x8
     4ec: 54000082     	b.hs	0x4fc <_mech_grid_row+0xf0>
     4f0: aa0203f5     	mov	x21, x2
     4f4: aa0803f6     	mov	x22, x8
     4f8: 17ffffd5     	b	0x44c <_mech_grid_row+0x40>
     4fc: 910143ff     	add	sp, sp, #0x50
     500: a8c153f3     	ldp	x19, x20, [sp], #0x10
     504: a8c15bf5     	ldp	x21, x22, [sp], #0x10
     508: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     50c: d65f0fff     	retab
