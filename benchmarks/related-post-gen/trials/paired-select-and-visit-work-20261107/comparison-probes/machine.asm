
/private/tmp/mech-related-cranelift-comparison-pair-probes-20261107-v1/probes.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000000000 <_bool_eq_and_scalar>:
       0: d503237f     	pacibsp
       4: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
       8: 910003fd     	mov	x29, sp
       c: eb01001f     	cmp	x0, x1
      10: 9a9f17eb     	cset	x11, eq
      14: eb03005f     	cmp	x2, x3
      18: 9a9f17ec     	cset	x12, eq
      1c: 0a0c016c     	and	w12, w11, w12
      20: 53001d8c     	uxtb	w12, w12
      24: b500008c     	cbnz	x12, 0x34 <_bool_eq_and_scalar+0x34>
      28: d2800000     	mov	x0, #0x0                ; =0
      2c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
      30: d65f0fff     	retab
      34: d2800020     	mov	x0, #0x1                ; =1
      38: a8c17bfd     	ldp	x29, x30, [sp], #0x10
      3c: d65f0fff     	retab
		...

0000000000000080 <_bool_eq_and_pair>:
      80: d503237f     	pacibsp
      84: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
      88: 910003fd     	mov	x29, sp
      8c: eb01001f     	cmp	x0, x1
      90: fa430040     	ccmp	x2, x3, #0x0, eq
      94: 54000080     	b.eq	0xa4 <_bool_eq_and_pair+0x24>
      98: d2800000     	mov	x0, #0x0                ; =0
      9c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
      a0: d65f0fff     	retab
      a4: d2800020     	mov	x0, #0x1                ; =1
      a8: a8c17bfd     	ldp	x29, x30, [sp], #0x10
      ac: d65f0fff     	retab
		...

0000000000000100 <_bool_ne_or_scalar>:
     100: d503237f     	pacibsp
     104: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     108: 910003fd     	mov	x29, sp
     10c: eb01001f     	cmp	x0, x1
     110: 9a9f07eb     	cset	x11, ne
     114: eb03005f     	cmp	x2, x3
     118: 9a9f07ec     	cset	x12, ne
     11c: 2a0c016c     	orr	w12, w11, w12
     120: 53001d8c     	uxtb	w12, w12
     124: b500008c     	cbnz	x12, 0x134 <_bool_ne_or_scalar+0x34>
     128: d2800000     	mov	x0, #0x0                ; =0
     12c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     130: d65f0fff     	retab
     134: d2800020     	mov	x0, #0x1                ; =1
     138: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     13c: d65f0fff     	retab
		...

0000000000000180 <_bool_ne_or_pair>:
     180: d503237f     	pacibsp
     184: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     188: 910003fd     	mov	x29, sp
     18c: eb01001f     	cmp	x0, x1
     190: fa430040     	ccmp	x2, x3, #0x0, eq
     194: 54000081     	b.ne	0x1a4 <_bool_ne_or_pair+0x24>
     198: d2800000     	mov	x0, #0x0                ; =0
     19c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     1a0: d65f0fff     	retab
     1a4: d2800020     	mov	x0, #0x1                ; =1
     1a8: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     1ac: d65f0fff     	retab
