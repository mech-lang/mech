
/private/tmp/mech-matrix-read-fix-20261028/timing/current:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000102c95b40 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_>:
102c95b40: f9400028    	ldr	x8, [x1]
102c95b44: a9bd57f6    	stp	x22, x21, [sp, #-0x30]!
102c95b48: a9014ff4    	stp	x20, x19, [sp, #0x10]
102c95b4c: a9027bfd    	stp	x29, x30, [sp, #0x20]
102c95b50: 910083fd    	add	x29, sp, #0x20
102c95b54: aa0203f3    	mov	x19, x2
102c95b58: f00027e9    	adrp	x9, 0x103194000 <_anon.a18f98b2422b0223634141541a2296c0.280+0x118>
102c95b5c: 913be929    	add	x9, x9, #0xefa
102c95b60: 1000008a    	adr	x10, 0x102c95b70 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x30>
102c95b64: 3868692b    	ldrb	w11, [x9, x8]
102c95b68: 8b0b094a    	add	x10, x10, x11, lsl #2
102c95b6c: d61f0140    	br	x10
102c95b70: f9400828    	ldr	x8, [x1, #0x10]
102c95b74: b40015c8    	cbz	x8, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95b78: d37df109    	lsl	x9, x8, #3
102c95b7c: f9400268    	ldr	x8, [x19]
102c95b80: b1002108    	adds	x8, x8, #0x8
102c95b84: da9f3108    	csinv	x8, x8, xzr, lo
102c95b88: f1002129    	subs	x9, x9, #0x8
102c95b8c: 54ffffa1    	b.ne	0x102c95b80 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x40>
102c95b90: 140000a6    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95b94: f9400829    	ldr	x9, [x1, #0x10]
102c95b98: b40014a9    	cbz	x9, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95b9c: f9400268    	ldr	x8, [x19]
102c95ba0: b1000508    	adds	x8, x8, #0x1
102c95ba4: da9f3108    	csinv	x8, x8, xzr, lo
102c95ba8: f1000529    	subs	x9, x9, #0x1
102c95bac: 54ffffa1    	b.ne	0x102c95ba0 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x60>
102c95bb0: 1400009e    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95bb4: f9400828    	ldr	x8, [x1, #0x10]
102c95bb8: b40013a8    	cbz	x8, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95bbc: d37ced09    	lsl	x9, x8, #4
102c95bc0: f9400268    	ldr	x8, [x19]
102c95bc4: b1004108    	adds	x8, x8, #0x10
102c95bc8: da9f3108    	csinv	x8, x8, xzr, lo
102c95bcc: f1004129    	subs	x9, x9, #0x10
102c95bd0: 54ffffa1    	b.ne	0x102c95bc4 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x84>
102c95bd4: 14000095    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95bd8: f9400828    	ldr	x8, [x1, #0x10]
102c95bdc: b4001288    	cbz	x8, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95be0: d37ef509    	lsl	x9, x8, #2
102c95be4: f9400268    	ldr	x8, [x19]
102c95be8: b1001108    	adds	x8, x8, #0x4
102c95bec: da9f3108    	csinv	x8, x8, xzr, lo
102c95bf0: f1001129    	subs	x9, x9, #0x4
102c95bf4: 54ffffa1    	b.ne	0x102c95be8 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0xa8>
102c95bf8: 1400008c    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95bfc: f9400c28    	ldr	x8, [x1, #0x18]
102c95c00: b4001168    	cbz	x8, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95c04: d37df109    	lsl	x9, x8, #3
102c95c08: f9400268    	ldr	x8, [x19]
102c95c0c: b1002108    	adds	x8, x8, #0x8
102c95c10: da9f3108    	csinv	x8, x8, xzr, lo
102c95c14: f1002129    	subs	x9, x9, #0x8
102c95c18: 54ffffa1    	b.ne	0x102c95c0c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0xcc>
102c95c1c: 14000083    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95c20: f9400828    	ldr	x8, [x1, #0x10]
102c95c24: b4001048    	cbz	x8, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95c28: d37df109    	lsl	x9, x8, #3
102c95c2c: f9400268    	ldr	x8, [x19]
102c95c30: b1002108    	adds	x8, x8, #0x8
102c95c34: da9f3108    	csinv	x8, x8, xzr, lo
102c95c38: f1002129    	subs	x9, x9, #0x8
102c95c3c: 54ffffa1    	b.ne	0x102c95c30 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0xf0>
102c95c40: 1400007a    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95c44: f9400828    	ldr	x8, [x1, #0x10]
102c95c48: b4000f28    	cbz	x8, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95c4c: d37df109    	lsl	x9, x8, #3
102c95c50: f9400268    	ldr	x8, [x19]
102c95c54: b1002108    	adds	x8, x8, #0x8
102c95c58: da9f3108    	csinv	x8, x8, xzr, lo
102c95c5c: f1002129    	subs	x9, x9, #0x8
102c95c60: 54ffffa1    	b.ne	0x102c95c54 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x114>
102c95c64: 14000071    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95c68: f9400c28    	ldr	x8, [x1, #0x18]
102c95c6c: b4000e08    	cbz	x8, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95c70: d37ff909    	lsl	x9, x8, #1
102c95c74: f9400268    	ldr	x8, [x19]
102c95c78: b1000908    	adds	x8, x8, #0x2
102c95c7c: da9f3108    	csinv	x8, x8, xzr, lo
102c95c80: f1000929    	subs	x9, x9, #0x2
102c95c84: 54ffffa1    	b.ne	0x102c95c78 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x138>
102c95c88: 14000068    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95c8c: f9400828    	ldr	x8, [x1, #0x10]
102c95c90: b4000ce8    	cbz	x8, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95c94: d37ced09    	lsl	x9, x8, #4
102c95c98: f9400268    	ldr	x8, [x19]
102c95c9c: b1004108    	adds	x8, x8, #0x10
102c95ca0: da9f3108    	csinv	x8, x8, xzr, lo
102c95ca4: f1004129    	subs	x9, x9, #0x10
102c95ca8: 54ffffa1    	b.ne	0x102c95c9c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x15c>
102c95cac: 1400005f    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95cb0: f9400c28    	ldr	x8, [x1, #0x18]
102c95cb4: b4000bc8    	cbz	x8, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95cb8: d37ef509    	lsl	x9, x8, #2
102c95cbc: f9400268    	ldr	x8, [x19]
102c95cc0: b1001108    	adds	x8, x8, #0x4
102c95cc4: da9f3108    	csinv	x8, x8, xzr, lo
102c95cc8: f1001129    	subs	x9, x9, #0x4
102c95ccc: 54ffffa1    	b.ne	0x102c95cc0 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x180>
102c95cd0: 14000056    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95cd4: f9400828    	ldr	x8, [x1, #0x10]
102c95cd8: b4000aa8    	cbz	x8, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95cdc: d37ff909    	lsl	x9, x8, #1
102c95ce0: f9400268    	ldr	x8, [x19]
102c95ce4: b1000908    	adds	x8, x8, #0x2
102c95ce8: da9f3108    	csinv	x8, x8, xzr, lo
102c95cec: f1000929    	subs	x9, x9, #0x2
102c95cf0: 54ffffa1    	b.ne	0x102c95ce4 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x1a4>
102c95cf4: 1400004d    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95cf8: f9400c28    	ldr	x8, [x1, #0x18]
102c95cfc: f9400269    	ldr	x9, [x19]
102c95d00: ab080128    	adds	x8, x9, x8
102c95d04: da9f3108    	csinv	x8, x8, xzr, lo
102c95d08: 14000048    	b	0x102c95e28 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2e8>
102c95d0c: f9400828    	ldr	x8, [x1, #0x10]
102c95d10: b40008e8    	cbz	x8, 0x102c95e2c <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x2ec>
102c95d14: aa0003f4    	mov	x20, x0
102c95d18: f9400421    	ldr	x1, [x1, #0x8]
102c95d1c: 8b080508    	add	x8, x8, x8, lsl #1
102c95d20: d37ced15    	lsl	x21, x8, #4
102c95d24: 9100c036    	add	x22, x1, #0x30
102c95d28: aa1403e0    	mov	x0, x20
102c95d2c: aa1303e2    	mov	x2, x19
102c95d30: 97fff291    	bl	0x102c92774 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding11encode_dataNtB2_18LengthSnapshotSinkEB6_>
102c95d34: aa1603e1    	mov	x1, x22
102c95d38: f100c2b5    	subs	x21, x21, #0x30
102c95d3c: 54ffff41    	b.ne	0x102c95d24 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding15encode_sequenceNtB2_18LengthSnapshotSinkEB6_+0x1e4>
