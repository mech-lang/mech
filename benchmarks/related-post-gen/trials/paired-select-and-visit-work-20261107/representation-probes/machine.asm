
/private/tmp/mech-related-cranelift-representation-probes-20261107-v1/probes.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000000000 <_independent_selects>:
       0: d503237f     	pacibsp
       4: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
       8: 910003fd     	mov	x29, sp
       c: eb01001f     	cmp	x0, x1
      10: 9a818005     	csel	x5, x0, x1, hi
      14: eb01001f     	cmp	x0, x1
      18: 9a813001     	csel	x1, x0, x1, lo
      1c: aa0503e0     	mov	x0, x5
      20: a8c17bfd     	ldp	x29, x30, [sp], #0x10
      24: d65f0fff     	retab
		...

0000000000000080 <_paired_selects>:
      80: d503237f     	pacibsp
      84: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
      88: 910003fd     	mov	x29, sp
      8c: eb01001f     	cmp	x0, x1
      90: 9a818004     	csel	x4, x0, x1, hi
      94: 9a808021     	csel	x1, x1, x0, hi
      98: aa0403e0     	mov	x0, x4
      9c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
      a0: d65f0fff     	retab
		...

0000000000000100 <_scalar_clear>:
     100: d503237f     	pacibsp
     104: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     108: 910003fd     	mov	x29, sp
     10c: d2800002     	mov	x2, #0x0                ; =0
     110: f9000002     	str	x2, [x0]
     114: f9000402     	str	x2, [x0, #0x8]
     118: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     11c: d65f0fff     	retab
		...

0000000000000180 <_vector_clear>:
     180: d503237f     	pacibsp
     184: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     188: 910003fd     	mov	x29, sp
     18c: 4f00e402     	movi.16b	v2, #0x0
     190: 3d800002     	str	q2, [x0]
     194: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     198: d65f0fff     	retab
		...

0000000000000200 <_bool_cone_i8>:
     200: d503237f     	pacibsp
     204: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     208: 910003fd     	mov	x29, sp
     20c: eb01001f     	cmp	x0, x1
     210: 9a9f27eb     	cset	x11, lo
     214: eb03005f     	cmp	x2, x3
     218: 9a9f87ec     	cset	x12, ls
     21c: 0a0c016c     	and	w12, w11, w12
     220: 53001d8c     	uxtb	w12, w12
     224: b500008c     	cbnz	x12, 0x234 <_bool_cone_i8+0x34>
     228: d2800000     	mov	x0, #0x0                ; =0
     22c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     230: d65f0fff     	retab
     234: d2800020     	mov	x0, #0x1                ; =1
     238: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     23c: d65f0fff     	retab
		...

0000000000000280 <_bool_cone_i64>:
     280: d503237f     	pacibsp
     284: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     288: 910003fd     	mov	x29, sp
     28c: eb01001f     	cmp	x0, x1
     290: 9a9f27eb     	cset	x11, lo
     294: eb03005f     	cmp	x2, x3
     298: 9a9f87ec     	cset	x12, ls
     29c: 0a0c016c     	and	w12, w11, w12
     2a0: 53001d8c     	uxtb	w12, w12
     2a4: b500008c     	cbnz	x12, 0x2b4 <_bool_cone_i64+0x34>
     2a8: d2800000     	mov	x0, #0x0                ; =0
     2ac: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     2b0: d65f0fff     	retab
     2b4: d2800020     	mov	x0, #0x1                ; =1
     2b8: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     2bc: d65f0fff     	retab
		...

0000000000000300 <_checked_add>:
     300: d503237f     	pacibsp
     304: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
     308: 910003fd     	mov	x29, sp
     30c: ab010000     	adds	x0, x0, x1
     310: 9a9f37e7     	cset	x7, hs
     314: 53001ce6     	uxtb	w6, w7
     318: b5000066     	cbnz	x6, 0x324 <_checked_add+0x24>
     31c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     320: d65f0fff     	retab
     324: d2800000     	mov	x0, #0x0                ; =0
     328: a8c17bfd     	ldp	x29, x30, [sp], #0x10
     32c: d65f0fff     	retab
