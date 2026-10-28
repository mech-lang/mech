
/private/tmp/mech-packed-length-round05-20261028/current:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000102bfb644 <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence>:
102bfb644: d10143ff    	sub	sp, sp, #0x50
102bfb648: a90257f6    	stp	x22, x21, [sp, #0x20]
102bfb64c: a9034ff4    	stp	x20, x19, [sp, #0x30]
102bfb650: a9047bfd    	stp	x29, x30, [sp, #0x40]
102bfb654: 910103fd    	add	x29, sp, #0x40
102bfb658: f9400048    	ldr	x8, [x2]
102bfb65c: 52800035    	mov	w21, #0x1               ; =1
102bfb660: aa0003f3    	mov	x19, x0
102bfb664: f0002c89    	adrp	x9, 0x10318e000 <_anon.cecf560dfe9f8316b7bba111f433730c.2+0x124>
102bfb668: 91069929    	add	x9, x9, #0x1a6
102bfb66c: 1000008a    	adr	x10, 0x102bfb67c <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0x38>
102bfb670: 3868692b    	ldrb	w11, [x9, x8]
102bfb674: 8b0b094a    	add	x10, x10, x11, lsl #2
102bfb678: d61f0140    	br	x10
102bfb67c: 52800115    	mov	w21, #0x8               ; =8
102bfb680: 14000014    	b	0x102bfb6d0 <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0x8c>
102bfb684: 52800215    	mov	w21, #0x10              ; =16
102bfb688: 14000012    	b	0x102bfb6d0 <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0x8c>
102bfb68c: 52800095    	mov	w21, #0x4               ; =4
102bfb690: 14000010    	b	0x102bfb6d0 <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0x8c>
102bfb694: f100491f    	cmp	x8, #0x12
102bfb698: 540004cc    	b.gt	0x102bfb730 <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0xec>
102bfb69c: d1004508    	sub	x8, x8, #0x11
102bfb6a0: f100091f    	cmp	x8, #0x2
102bfb6a4: 54000662    	b.hs	0x102bfb770 <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0x12c>
102bfb6a8: f9400848    	ldr	x8, [x2, #0x10]
102bfb6ac: b4000388    	cbz	x8, 0x102bfb71c <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0xd8>
102bfb6b0: d37df109    	lsl	x9, x8, #3
102bfb6b4: f9400268    	ldr	x8, [x19]
102bfb6b8: b1002108    	adds	x8, x8, #0x8
102bfb6bc: da9f3108    	csinv	x8, x8, xzr, lo
102bfb6c0: f1002129    	subs	x9, x9, #0x8
102bfb6c4: 54ffffa1    	b.ne	0x102bfb6b8 <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0x74>
102bfb6c8: 14000014    	b	0x102bfb718 <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0xd4>
102bfb6cc: 52800055    	mov	w21, #0x2               ; =2
102bfb6d0: 910023f4    	add	x20, sp, #0x8
102bfb6d4: 910023e8    	add	x8, sp, #0x8
102bfb6d8: aa0203e0    	mov	x0, x2
102bfb6dc: 94023842    	bl	0x102c897e4 <__RNvMs_NtNtCskIfnrfrMWVM_9mech_core8snapshot8sequenceNtB4_15SequenceStorage4view>
102bfb6e0: f94007e8    	ldr	x8, [sp, #0x8]
102bfb6e4: 52800209    	mov	w9, #0x10               ; =16
102bfb6e8: 5280010a    	mov	w10, #0x8               ; =8
102bfb6ec: f1004d1f    	cmp	x8, #0x13
102bfb6f0: 9a890148    	csel	x8, x10, x9, eq
102bfb6f4: f8686a88    	ldr	x8, [x20, x8]
102bfb6f8: f9400269    	ldr	x9, [x19]
102bfb6fc: 9b157d0a    	mul	x10, x8, x21
102bfb700: 9bd57d08    	umulh	x8, x8, x21
102bfb704: 9280000b    	mov	x11, #-0x1              ; =-1
102bfb708: f100011f    	cmp	x8, #0x0
102bfb70c: 9a8b0148    	csel	x8, x10, x11, eq
102bfb710: ab080128    	adds	x8, x9, x8
102bfb714: da9f3108    	csinv	x8, x8, xzr, lo
102bfb718: f9000268    	str	x8, [x19]
102bfb71c: a9447bfd    	ldp	x29, x30, [sp, #0x40]
102bfb720: a9434ff4    	ldp	x20, x19, [sp, #0x30]
102bfb724: a94257f6    	ldp	x22, x21, [sp, #0x20]
102bfb728: 910143ff    	add	sp, sp, #0x50
102bfb72c: d65f03c0    	ret
102bfb730: f1004d1f    	cmp	x8, #0x13
102bfb734: 54ffff40    	b.eq	0x102bfb71c <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0xd8>
102bfb738: f9400848    	ldr	x8, [x2, #0x10]
102bfb73c: b4ffff08    	cbz	x8, 0x102bfb71c <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0xd8>
102bfb740: aa0103f4    	mov	x20, x1
102bfb744: f9400441    	ldr	x1, [x2, #0x8]
102bfb748: 8b080508    	add	x8, x8, x8, lsl #1
102bfb74c: d37ced15    	lsl	x21, x8, #4
102bfb750: 9100c036    	add	x22, x1, #0x30
102bfb754: aa1403e0    	mov	x0, x20
102bfb758: aa1303e2    	mov	x2, x19
102bfb75c: 97ff9205    	bl	0x102bdff70 <__RINvNtNtCskIfnrfrMWVM_9mech_core8snapshot8encoding11encode_dataNtB2_18LengthSnapshotSinkEB6_>
102bfb760: aa1603e1    	mov	x1, x22
102bfb764: f100c2b5    	subs	x21, x21, #0x30
102bfb768: 54ffff41    	b.ne	0x102bfb750 <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0x10c>
102bfb76c: 17ffffec    	b	0x102bfb71c <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0xd8>
102bfb770: f9400848    	ldr	x8, [x2, #0x10]
102bfb774: b4fffd48    	cbz	x8, 0x102bfb71c <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0xd8>
102bfb778: f940044a    	ldr	x10, [x2, #0x8]
102bfb77c: d37ced09    	lsl	x9, x8, #4
102bfb780: f9400268    	ldr	x8, [x19]
102bfb784: 9100214a    	add	x10, x10, #0x8
102bfb788: f841054b    	ldr	x11, [x10], #0x10
102bfb78c: b1002108    	adds	x8, x8, #0x8
102bfb790: da9f3108    	csinv	x8, x8, xzr, lo
102bfb794: ab0b0108    	adds	x8, x8, x11
102bfb798: da9f3108    	csinv	x8, x8, xzr, lo
102bfb79c: f1004129    	subs	x9, x9, #0x10
102bfb7a0: 54ffff41    	b.ne	0x102bfb788 <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0x144>
102bfb7a4: 17ffffdd    	b	0x102bfb718 <__RNvXs2_NtNtCskIfnrfrMWVM_9mech_core8snapshot8encodingNtB5_18LengthSnapshotSinkNtB5_16SnapshotByteSink14write_sequence+0xd4>
