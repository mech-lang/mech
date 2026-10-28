CALL SITE
     d7c:      	str	x7, [x23, #0xc88]
     d80:      	ldr	x13, [sp, #0x110]
     d84:      	str	x13, [x23, #0xd00]
     d88:      	ldr	x4, [sp, #0x108]
     d8c:      	str	x4, [x23, #0xd08]
     d90:      	mov	x1, #0x0                ; =0
     d94:      	str	x1, [x0, #0x48]
     d98:      	mov	x19, x0
     d9c:      	ldr	x0, [sp, #0x130]
     da0:      	blr	x20
     da4:      	mov	x3, #0x2                ; =2
     da8:      	cmp	x0, #0x0
     dac:      	b.ne	0xaeebc <_mech_canonical_program_entry_v1+0xaeebc>
     db0:      	mov	x1, #0x1                ; =1
     db4:      	str	x1, [x19, #0x48]
CALL SITE
     d98:      	mov	x19, x0
     d9c:      	ldr	x0, [sp, #0x130]
     da0:      	blr	x20
     da4:      	mov	x3, #0x2                ; =2
     da8:      	cmp	x0, #0x0
     dac:      	b.ne	0xaeebc <_mech_canonical_program_entry_v1+0xaeebc>
     db0:      	mov	x1, #0x1                ; =1
     db4:      	str	x1, [x19, #0x48]
     db8:      	ldr	x0, [sp, #0x130]
     dbc:      	blr	x20
     dc0:      	mov	x3, #0x2                ; =2
     dc4:      	cmp	x0, #0x0
     dc8:      	b.ne	0xaef04 <_mech_canonical_program_entry_v1+0xaef04>
     dcc:      	mov	x3, #0x3                ; =3
     dd0:      	cmp	x22, #0x2
CALL SITE
     dec:      	mov	x1, #0x1                ; =1
     df0:      	mov	x5, #0x0                ; =0
     df4:      	mov	x3, #0x2                ; =2
     df8:      	stur	x3, [sp]
     dfc:      	ldr	x0, [sp, #0x130]
     e00:      	ldr	x3, [sp, #0x120]
     e04:      	ldur	x4, [sp]
     e08:      	ldr	x6, [sp, #0x128]
     e0c:      	mov	x2, x5
     e10:      	blr	x21
     e14:      	cmp	x0, #0x0
     e18:      	b.ne	0xaef0c <_mech_canonical_program_entry_v1+0xaef0c>
     e1c:      	b	0xe24 <_mech_canonical_program_entry_v1+0xe24>
     e20:      	mov	x19, x0
     e24:      	mov	x0, #0x2                ; =2
