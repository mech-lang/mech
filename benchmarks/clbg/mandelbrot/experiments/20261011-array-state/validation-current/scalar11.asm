
/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/mandelbrot/experiments/20261011-array-state/validation-current/scalar11.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000000000 <_mech_map_bits>:
       0: fd400000     	ldr	d0, [x0]
       4: fd400402     	ldr	d2, [x0, #0x8]
       8: fd400801     	ldr	d1, [x0, #0x10]
       c: 0f000405     	movi.2s	v5, #0x0
      10: 1e601003     	fmov	d3, #2.00000000
      14: 1e601863     	fdiv	d3, d3, d0
      18: 1e630840     	fmul	d0, d2, d3
      1c: 1e6f1002     	fmov	d2, #1.50000000
      20: 1e623800     	fsub	d0, d0, d2
      24: 1e630821     	fmul	d1, d1, d3
      28: 1e6e1002     	fmov	d2, #1.00000000
      2c: 1e623821     	fsub	d1, d1, d2
      30: 4ea51cb0     	mov.16b	v16, v5
      34: 4ea51cb1     	mov.16b	v17, v5
      38: d2e80920     	mov	x0, #0x4049000000000000 ; =4632233691727265792
      3c: 9e670002     	fmov	d2, x0
      40: 92800000     	mov	x0, #-0x1               ; =-1
      44: d2800001     	mov	x1, #0x0                ; =0
      48: 1e622220     	fcmp	d17, d2
      4c: 9a81a002     	csel	x2, x0, x1, ge
      50: 1e6508a2     	fmul	d2, d5, d5
      54: 1e700a03     	fmul	d3, d16, d16
      58: 1e632844     	fadd	d4, d2, d3
      5c: 1e621006     	fmov	d6, #4.00000000
      60: 1e662080     	fcmp	d4, d6
      64: 9a819000     	csel	x0, x0, x1, ls
      68: aa200041     	orn	x1, x2, x0
      6c: f100003f     	cmp	x1, #0x0
      70: 54000141     	b.ne	0x98 <_mech_map_bits+0x98>
      74: 1e633842     	fsub	d2, d2, d3
      78: 1e602842     	fadd	d2, d2, d0
      7c: 1e6528a3     	fadd	d3, d5, d5
      80: 1e700863     	fmul	d3, d3, d16
      84: 1e612870     	fadd	d16, d3, d1
      88: 1e6e1003     	fmov	d3, #1.00000000
      8c: 1e632a31     	fadd	d17, d17, d3
      90: 4ea21c45     	mov.16b	v5, v2
      94: 17ffffe9     	b	0x38 <_mech_map_bits+0x38>
      98: 92400000     	and	x0, x0, #0x1
      9c: d65f03c0     	ret

00000000000000a0 <_mech_grid_row>:
      a0: d503237f     	pacibsp
      a4: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
      a8: 910003fd     	mov	x29, sp
      ac: a9bf5bf5     	stp	x21, x22, [sp, #-0x10]!
      b0: a9bf53f3     	stp	x19, x20, [sp, #-0x10]!
      b4: d10083ff     	sub	sp, sp, #0x20
      b8: aa0203f5     	mov	x21, x2
      bc: 9e630013     	ucvtf	d19, x0
      c0: 910003f3     	mov	x19, sp
      c4: fc0003f3     	stur	d19, [sp]
      c8: 9e630034     	ucvtf	d20, x1
      cc: fc0103f4     	stur	d20, [sp, #0x10]
      d0: d2800014     	mov	x20, #0x0               ; =0
      d4: f100001f     	cmp	x0, #0x0
      d8: aa0003f6     	mov	x22, x0
      dc: 54000340     	b.eq	0x144 <_mech_grid_row+0xa4>
      e0: 9e630299     	ucvtf	d25, x20
      e4: fc0083f9     	stur	d25, [sp, #0x8]
      e8: aa1303e0     	mov	x0, x19
      ec: 94000000     	bl	0xec <_mech_grid_row+0x4c>
      f0: aa1603e4     	mov	x4, x22
      f4: eb04029f     	cmp	x20, x4
      f8: 54000063     	b.lo	0x104 <_mech_grid_row+0x64>
      fc: aa1503e2     	mov	x2, x21
     100: 1400000b     	b	0x12c <_mech_grid_row+0x8c>
     104: d343fe81     	lsr	x1, x20, #3
     108: aa1503e2     	mov	x2, x21
     10c: 38616843     	ldrb	w3, [x2, x1]
     110: 92400000     	and	x0, x0, #0x1
     114: d28000e5     	mov	x5, #0x7                ; =7
     118: 92400a86     	and	x6, x20, #0x7
     11c: cb0600a5     	sub	x5, x5, x6
     120: 9ac52000     	lsl	x0, x0, x5
     124: 2a000060     	orr	w0, w3, w0
     128: 38216840     	strb	w0, [x2, x1]
     12c: 91000694     	add	x20, x20, #0x1
     130: eb04029f     	cmp	x20, x4
     134: 54000082     	b.hs	0x144 <_mech_grid_row+0xa4>
     138: aa0203f5     	mov	x21, x2
     13c: aa0403f6     	mov	x22, x4
     140: 17ffffe8     	b	0xe0 <_mech_grid_row+0x40>
     144: 910083ff     	add	sp, sp, #0x20
     148: a8c153f3     	ldp	x19, x20, [sp], #0x10
     14c: a8c15bf5     	ldp	x21, x22, [sp], #0x10
     150: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     154: d65f0fff     	retab
