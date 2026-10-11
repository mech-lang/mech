
/tmp/mech-execution-modes-target/release/mech-execution-modes-optimized:	file format mach-o arm64

Disassembly of section __TEXT,__text:

000000010008764c <__RNvMNtNtCsfBfIVmeLE1N_8mech_gpu8sequence6countsNtB2_6Counts9increment>:
10008764c:     	ldr	x8, [x0]
100087650:     	sub	sp, sp, #0x70
100087654:     	stp	x26, x25, [sp, #0x20]
100087658:     	stp	x24, x23, [sp, #0x30]
10008765c:     	stp	x22, x21, [sp, #0x40]
100087660:     	stp	x20, x19, [sp, #0x50]
100087664:     	stp	x29, x30, [sp, #0x60]
100087668:     	add	x29, sp, #0x60
10008766c:     	mov	x19, x1
100087670:     	mov	x20, x0
100087674:     	adrp	x9, 0x10185a000 <_anon.b549620749464f397b163b30b86001d2.206+0xbe0>
100087678:     	add	x9, x9, #0x552
10008767c:     	adr	x10,  <L0>
100087680:     	ldrh	w11, [x9, x8, lsl #1]
100087684:     	add	x10, x10, x11, lsl #2
100087688:     	br	x10
<L0>:
10008768c:     	cbz	x2,  <L103>
100087690:     	add	x21, x20, #0x8
100087694:     	add	x22, x19, x2, lsl #3
100087698:     	adrp	x23, 0x101e56000 <__RNvNCNKNvNtNtCskSNJgEBggig_11mech_engine8resident6budget12CONTROL_WORK0023___RUST_STD_INTERNAL_VAL+0x8>
10008769c:     	add	x23, x23, #0x6e0
1000876a0:     	b	 <L6>
<L1>:
1000876a4:     	and	x13, x13, x11
1000876a8:     	ldr	d1, [x12, x13]
1000876ac:     	cmeq.8b	v2, v1, v0
1000876b0:     	fmov	x14, d2
1000876b4:     	ands	x14, x14, #0x8080808080808080
1000876b8:     	b.eq	 <L3>
<L2>:
1000876bc:     	rbit	x15, x14
1000876c0:     	clz	x15, x15
1000876c4:     	add	x15, x13, x15, lsr #3
1000876c8:     	and	x15, x15, x11
1000876cc:     	sub	x15, x12, x15, lsl #4
1000876d0:     	ldur	x16, [x15, #-0x10]
1000876d4:     	cmp	x9, x16
1000876d8:     	b.eq	 <L4>
1000876dc:     	sub	x15, x14, #0x1
1000876e0:     	ands	x14, x15, x14
1000876e4:     	b.ne	 <L2>
<L3>:
1000876e8:     	movi.2d	v2, #0xffffffffffffffff
1000876ec:     	cmeq.8b	v1, v1, v2
1000876f0:     	umaxv.8b	b1, v1
1000876f4:     	fmov	w14, s1
1000876f8:     	tbnz	w14, #0x0,  <L7>
1000876fc:     	add	x10, x10, #0x8
100087700:     	add	x13, x13, x10
100087704:     	b	 <L1>
<L4>:
100087708:     	mov	x9, #0x0                ; =0
10008770c:     	str	x15, [sp, #0x8]
100087710:     	add	x10, x20, #0x8
<L5>:
100087714:     	stp	x10, x8, [sp, #0x10]
100087718:     	str	x9, [sp]
10008771c:     	mov	x0, sp
100087720:     	bl	 <__RNvMs1d_NtCsd0jj78El3J_9hashbrown3mapINtB6_5EntryyyNtNtCseULylwdyJVZ_8foldhash4fast11RandomStateE9or_insertCsfBfIVmeLE1N_8mech_gpu>
100087724:     	ldr	x8, [x0]
100087728:     	add	x8, x8, #0x1
10008772c:     	str	x8, [x0]
100087730:     	cmp	x19, x22
100087734:     	b.eq	 <L103>
<L6>:
100087738:     	mov	x10, #0x0               ; =0
10008773c:     	ldr	x9, [x19], #0x8
100087740:     	ldr	x8, [x20, #0x28]
100087744:     	ldr	x11, [x23]
100087748:     	eor	x8, x8, x9
10008774c:     	mul	x12, x11, x8
100087750:     	umulh	x8, x11, x8
100087754:     	eor	x8, x8, x12
100087758:     	lsr	x13, x8, #57
10008775c:     	ldp	x12, x11, [x20, #0x8]
100087760:     	dup.8b	v0, w13
100087764:     	mov	x13, x8
100087768:     	b	 <L1>
<L7>:
10008776c:     	str	x21, [sp, #0x8]
100087770:     	mov	x10, x8
100087774:     	mov	x8, x9
100087778:     	mov	w9, #0x1                ; =1
10008777c:     	b	 <L5>
100087780:     	cbz	x2,  <L103>
100087784:     	add	x21, x20, #0x8
100087788:     	add	x22, x19, x2, lsl #3
10008778c:     	mov	x23, #0xa95             ; =2709
100087790:     	movk	x23, #0x2722, lsl #16
100087794:     	movk	x23, #0xc1b7, lsl #32
100087798:     	movk	x23, #0x517c, lsl #48
10008779c:     	b	 <L13>
<L8>:
1000877a0:     	and	x13, x13, x11
1000877a4:     	ldr	d1, [x12, x13]
1000877a8:     	cmeq.8b	v2, v1, v0
1000877ac:     	fmov	x14, d2
1000877b0:     	ands	x14, x14, #0x8080808080808080
1000877b4:     	b.eq	 <L10>
<L9>:
1000877b8:     	rbit	x15, x14
1000877bc:     	clz	x15, x15
1000877c0:     	add	x15, x13, x15, lsr #3
1000877c4:     	and	x15, x15, x11
1000877c8:     	sub	x15, x12, x15, lsl #4
1000877cc:     	ldur	x16, [x15, #-0x10]
1000877d0:     	cmp	x9, x16
1000877d4:     	b.eq	 <L11>
1000877d8:     	sub	x15, x14, #0x1
1000877dc:     	ands	x14, x15, x14
1000877e0:     	b.ne	 <L9>
<L10>:
1000877e4:     	movi.2d	v2, #0xffffffffffffffff
1000877e8:     	cmeq.8b	v1, v1, v2
1000877ec:     	umaxv.8b	b1, v1
1000877f0:     	fmov	w14, s1
1000877f4:     	tbnz	w14, #0x0,  <L14>
1000877f8:     	add	x10, x10, #0x8
1000877fc:     	add	x13, x13, x10
100087800:     	b	 <L8>
<L11>:
100087804:     	mov	x9, #0x0                ; =0
100087808:     	str	x15, [sp, #0x8]
10008780c:     	add	x10, x20, #0x8
<L12>:
100087810:     	stp	x10, x8, [sp, #0x10]
100087814:     	str	x9, [sp]
100087818:     	mov	x0, sp
10008781c:     	bl	 <__RNvMs1d_NtCsd0jj78El3J_9hashbrown3mapINtB6_5EntryymINtNtCsgDHHaSskbIF_4core4hash18BuildHasherDefaultNtCsinZPWOYcTxw_6fxhash8FxHasherEE9or_insertCsfBfIVmeLE1N_8mech_gpu>
100087820:     	ldr	w8, [x0]
100087824:     	add	w8, w8, #0x1
100087828:     	str	w8, [x0]
10008782c:     	cmp	x19, x22
100087830:     	b.eq	 <L103>
<L13>:
100087834:     	mov	x10, #0x0               ; =0
100087838:     	ldr	x9, [x19], #0x8
10008783c:     	mul	x8, x9, x23
100087840:     	lsr	x13, x8, #57
100087844:     	ldp	x12, x11, [x20, #0x8]
100087848:     	dup.8b	v0, w13
10008784c:     	mov	x13, x8
100087850:     	b	 <L8>
<L14>:
100087854:     	str	x21, [sp, #0x8]
100087858:     	mov	x10, x8
10008785c:     	mov	x8, x9
100087860:     	mov	w9, #0x1                ; =1
100087864:     	b	 <L12>
100087868:     	cbz	x2,  <L103>
10008786c:     	add	x21, x19, x2, lsl #3
100087870:     	mov	x22, #0xa95             ; =2709
100087874:     	movk	x22, #0x2722, lsl #16
100087878:     	movk	x22, #0xc1b7, lsl #32
10008787c:     	movk	x22, #0x517c, lsl #48
100087880:     	movi.2d	v3, #0xffffffffffffffff
100087884:     	b	 <L22>
<L15>:
100087888:     	and	x11, x11, x9
10008788c:     	ldr	d1, [x8, x11]
100087890:     	cmeq.8b	v2, v1, v0
100087894:     	fmov	x12, d2
100087898:     	ands	x12, x12, #0x8080808080808080
10008789c:     	b.eq	 <L17>
<L16>:
1000878a0:     	rbit	x13, x12
1000878a4:     	clz	x13, x13
1000878a8:     	add	x13, x11, x13, lsr #3
1000878ac:     	and	x13, x13, x9
1000878b0:     	sub	x13, x8, x13, lsl #4
1000878b4:     	ldur	x14, [x13, #-0x10]
1000878b8:     	cmp	x14, x23
1000878bc:     	b.eq	 <L21>
1000878c0:     	sub	x13, x12, #0x1
1000878c4:     	ands	x12, x13, x12
1000878c8:     	b.ne	 <L16>
<L17>:
1000878cc:     	cmeq.8b	v1, v1, v3
1000878d0:     	umaxv.8b	b1, v1
1000878d4:     	fmov	w12, s1
1000878d8:     	tbnz	w12, #0x0,  <L18>
1000878dc:     	add	x10, x10, #0x8
1000878e0:     	add	x11, x11, x10
1000878e4:     	b	 <L15>
<L18>:
1000878e8:     	ldr	x10, [x20, #0x18]
1000878ec:     	cbz	x10,  <L23>
1000878f0:     	and	x10, x9, x25
1000878f4:     	ldr	d0, [x8, x10]
1000878f8:     	cmlt.8b	v0, v0, #0
1000878fc:     	fmov	x12, d0
100087900:     	cbz	x12,  <L24>
<L19>:
100087904:     	rbit	x11, x12
100087908:     	clz	x11, x11
10008790c:     	add	x10, x10, x11, lsr #3
100087910:     	and	x11, x10, x9
100087914:     	ldrsb	w10, [x8, x11]
100087918:     	tbz	w10, #0x1f,  <L26>
<L20>:
10008791c:     	sub	x12, x11, #0x8
100087920:     	and	x9, x12, x9
100087924:     	strb	w24, [x8, x11]
100087928:     	add	x9, x8, x9
10008792c:     	strb	w24, [x9, #0x8]
100087930:     	sub	x13, x8, x11, lsl #4
100087934:     	and	x8, x10, #0x1
100087938:     	stur	x23, [x13, #-0x10]
10008793c:     	stur	wzr, [x13, #-0x8]
100087940:     	ldur	q0, [x20, #0x18]
100087944:     	movi.2d	v1, #0xffffffffffffffff
100087948:     	mov.d	v1[0], x8
10008794c:     	sub.2d	v0, v0, v1
100087950:     	stur	q0, [x20, #0x18]
<L21>:
100087954:     	ldur	w8, [x13, #-0x8]
100087958:     	add	w8, w8, #0x1
10008795c:     	stur	w8, [x13, #-0x8]
100087960:     	cmp	x19, x21
100087964:     	b.eq	 <L103>
<L22>:
100087968:     	mov	x10, #0x0               ; =0
10008796c:     	ldr	x23, [x19], #0x8
100087970:     	mul	x25, x23, x22
100087974:     	lsr	x24, x25, #57
100087978:     	ldp	x8, x9, [x20, #0x8]
10008797c:     	dup.8b	v0, w24
100087980:     	mov	x11, x25
100087984:     	b	 <L15>
<L23>:
100087988:     	add	x0, x20, #0x8
10008798c:     	add	x2, x20, #0x28
100087990:     	mov	w1, #0x1                ; =1
100087994:     	mov	w3, #0x1                ; =1
100087998:     	bl	 <__RINvMs6_NtCs3odSxwvi1jy_9hashbrown3rawINtB6_8RawTableTymEE14reserve_rehashNCINvNtB8_3map11make_hasherymINtNtCsgDHHaSskbIF_4core4hash18BuildHasherDefaultNtCsinZPWOYcTxw_6fxhash8FxHasherEE0ECsfBfIVmeLE1N_8mech_gpu>
10008799c:     	movi.2d	v3, #0xffffffffffffffff
1000879a0:     	ldp	x8, x9, [x20, #0x8]
1000879a4:     	and	x10, x9, x25
1000879a8:     	ldr	d0, [x8, x10]
1000879ac:     	cmlt.8b	v0, v0, #0
1000879b0:     	fmov	x12, d0
1000879b4:     	cbnz	x12,  <L19>
<L24>:
1000879b8:     	mov	w11, #0x8               ; =8
<L25>:
1000879bc:     	add	x10, x10, x11
1000879c0:     	and	x10, x10, x9
1000879c4:     	ldr	d0, [x8, x10]
1000879c8:     	cmlt.8b	v0, v0, #0
1000879cc:     	fmov	x12, d0
1000879d0:     	add	x11, x11, #0x8
1000879d4:     	cbnz	x12,  <L19>
1000879d8:     	b	 <L25>
<L26>:
1000879dc:     	ldr	d0, [x8]
1000879e0:     	cmlt.8b	v0, v0, #0
1000879e4:     	fmov	x10, d0
1000879e8:     	rbit	x10, x10
1000879ec:     	clz	x10, x10
1000879f0:     	lsr	x11, x10, #3
1000879f4:     	ldrb	w10, [x8, x11]
1000879f8:     	b	 <L20>
1000879fc:     	cbz	x2,  <L103>
100087a00:     	add	x21, x20, #0x8
100087a04:     	add	x22, x19, x2, lsl #3
100087a08:     	mov	x23, #0xa95             ; =2709
100087a0c:     	movk	x23, #0x2722, lsl #16
100087a10:     	movk	x23, #0xc1b7, lsl #32
100087a14:     	movk	x23, #0x517c, lsl #48
100087a18:     	b	 <L32>
<L27>:
100087a1c:     	and	x13, x13, x11
100087a20:     	ldr	d1, [x12, x13]
100087a24:     	cmeq.8b	v2, v1, v0
100087a28:     	fmov	x14, d2
100087a2c:     	ands	x14, x14, #0x8080808080808080
100087a30:     	b.eq	 <L29>
<L28>:
100087a34:     	rbit	x15, x14
100087a38:     	clz	x15, x15
100087a3c:     	add	x15, x13, x15, lsr #3
100087a40:     	and	x15, x15, x11
100087a44:     	sub	x15, x12, x15, lsl #3
100087a48:     	ldurh	w16, [x15, #-0x8]
100087a4c:     	cmp	w16, w8
100087a50:     	b.eq	 <L30>
100087a54:     	sub	x15, x14, #0x1
100087a58:     	ands	x14, x15, x14
100087a5c:     	b.ne	 <L28>
<L29>:
100087a60:     	movi.2d	v2, #0xffffffffffffffff
100087a64:     	cmeq.8b	v1, v1, v2
100087a68:     	umaxv.8b	b1, v1
100087a6c:     	fmov	w14, s1
100087a70:     	tbnz	w14, #0x0,  <L33>
100087a74:     	add	x10, x10, #0x8
100087a78:     	add	x13, x13, x10
100087a7c:     	b	 <L27>
<L30>:
100087a80:     	mov	x10, #0x0               ; =0
100087a84:     	stp	x15, x21, [sp, #0x8]
100087a88:     	str	x9, [sp, #0x18]
<L31>:
100087a8c:     	str	x10, [sp]
100087a90:     	mov	x0, sp
100087a94:     	bl	 <__RNvMs1d_NtCsd0jj78El3J_9hashbrown3mapINtB6_5EntrytmINtNtCsgDHHaSskbIF_4core4hash18BuildHasherDefaultNtCsinZPWOYcTxw_6fxhash8FxHasherEE9or_insertCsfBfIVmeLE1N_8mech_gpu>
100087a98:     	ldr	w8, [x0]
100087a9c:     	add	w8, w8, #0x1
100087aa0:     	str	w8, [x0]
100087aa4:     	cmp	x19, x22
100087aa8:     	b.eq	 <L103>
<L32>:
100087aac:     	mov	x10, #0x0               ; =0
100087ab0:     	ldrh	w8, [x19], #0x8
100087ab4:     	mul	x9, x8, x23
100087ab8:     	lsr	x13, x9, #57
100087abc:     	ldp	x12, x11, [x20, #0x8]
100087ac0:     	dup.8b	v0, w13
100087ac4:     	mov	x13, x9
100087ac8:     	b	 <L27>
<L33>:
100087acc:     	stp	x21, x9, [sp, #0x8]
100087ad0:     	mov	w10, #0x1               ; =1
100087ad4:     	strh	w8, [sp, #0x18]
100087ad8:     	b	 <L31>
100087adc:     	cbz	x2,  <L103>
100087ae0:     	add	x21, x20, #0x8
100087ae4:     	add	x22, x19, x2, lsl #3
100087ae8:     	mov	x23, #0xa95             ; =2709
100087aec:     	movk	x23, #0x2722, lsl #16
100087af0:     	movk	x23, #0xc1b7, lsl #32
100087af4:     	movk	x23, #0x517c, lsl #48
100087af8:     	b	 <L39>
<L34>:
100087afc:     	and	x13, x13, x11
100087b00:     	ldr	d1, [x12, x13]
100087b04:     	cmeq.8b	v2, v1, v0
100087b08:     	fmov	x14, d2
100087b0c:     	ands	x14, x14, #0x8080808080808080
100087b10:     	b.eq	 <L36>
<L35>:
100087b14:     	rbit	x15, x14
100087b18:     	clz	x15, x15
100087b1c:     	add	x15, x13, x15, lsr #3
100087b20:     	and	x15, x15, x11
100087b24:     	sub	x15, x12, x15, lsl #3
100087b28:     	ldur	w16, [x15, #-0x8]
100087b2c:     	cmp	w16, w8
100087b30:     	b.eq	 <L37>
100087b34:     	sub	x15, x14, #0x1
100087b38:     	ands	x14, x15, x14
100087b3c:     	b.ne	 <L35>
<L36>:
100087b40:     	movi.2d	v2, #0xffffffffffffffff
100087b44:     	cmeq.8b	v1, v1, v2
100087b48:     	umaxv.8b	b1, v1
100087b4c:     	fmov	w14, s1
100087b50:     	tbnz	w14, #0x0,  <L40>
100087b54:     	add	x9, x9, #0x8
100087b58:     	add	x13, x13, x9
100087b5c:     	b	 <L34>
<L37>:
100087b60:     	mov	x9, #0x0                ; =0
100087b64:     	stp	x15, x21, [sp, #0x8]
100087b68:     	str	x10, [sp, #0x18]
<L38>:
100087b6c:     	str	x9, [sp]
100087b70:     	mov	x0, sp
100087b74:     	bl	 <__RNvMs1d_NtCsd0jj78El3J_9hashbrown3mapINtB6_5EntrymmINtNtCsgDHHaSskbIF_4core4hash18BuildHasherDefaultNtCsinZPWOYcTxw_6fxhash8FxHasherEE9or_insertCsfBfIVmeLE1N_8mech_gpu>
100087b78:     	ldr	w8, [x0]
100087b7c:     	add	w8, w8, #0x1
100087b80:     	str	w8, [x0]
100087b84:     	cmp	x19, x22
100087b88:     	b.eq	 <L103>
<L39>:
100087b8c:     	mov	x9, #0x0                ; =0
100087b90:     	ldr	x8, [x19], #0x8
100087b94:     	mov	w10, w8
100087b98:     	mul	x10, x10, x23
100087b9c:     	lsr	x13, x10, #57
100087ba0:     	ldp	x12, x11, [x20, #0x8]
100087ba4:     	dup.8b	v0, w13
100087ba8:     	mov	x13, x10
100087bac:     	b	 <L34>
<L40>:
100087bb0:     	stp	x21, x10, [sp, #0x8]
100087bb4:     	mov	w9, #0x1                ; =1
100087bb8:     	str	w8, [sp, #0x18]
100087bbc:     	b	 <L38>
100087bc0:     	cbz	x2,  <L103>
100087bc4:     	add	x21, x19, x2, lsl #3
100087bc8:     	mov	x22, #0xa95             ; =2709
100087bcc:     	movk	x22, #0x2722, lsl #16
100087bd0:     	movk	x22, #0xc1b7, lsl #32
100087bd4:     	movk	x22, #0x517c, lsl #48
100087bd8:     	movi.2d	v3, #0xffffffffffffffff
100087bdc:     	b	 <L48>
<L41>:
100087be0:     	and	x11, x11, x9
100087be4:     	ldr	d1, [x8, x11]
100087be8:     	cmeq.8b	v2, v1, v0
100087bec:     	fmov	x12, d2
100087bf0:     	ands	x12, x12, #0x8080808080808080
100087bf4:     	b.eq	 <L43>
<L42>:
100087bf8:     	rbit	x13, x12
100087bfc:     	clz	x13, x13
100087c00:     	add	x13, x11, x13, lsr #3
100087c04:     	and	x13, x13, x9
100087c08:     	sub	x13, x8, x13, lsl #3
100087c0c:     	ldurh	w14, [x13, #-0x8]
100087c10:     	cmp	w14, w23
100087c14:     	b.eq	 <L47>
100087c18:     	sub	x13, x12, #0x1
100087c1c:     	ands	x12, x13, x12
100087c20:     	b.ne	 <L42>
<L43>:
100087c24:     	cmeq.8b	v1, v1, v3
100087c28:     	umaxv.8b	b1, v1
100087c2c:     	fmov	w12, s1
100087c30:     	tbnz	w12, #0x0,  <L44>
100087c34:     	add	x10, x10, #0x8
100087c38:     	add	x11, x11, x10
100087c3c:     	b	 <L41>
<L44>:
100087c40:     	ldr	x10, [x20, #0x18]
100087c44:     	cbz	x10,  <L49>
100087c48:     	and	x10, x9, x25
100087c4c:     	ldr	d0, [x8, x10]
100087c50:     	cmlt.8b	v0, v0, #0
100087c54:     	fmov	x12, d0
100087c58:     	cbz	x12,  <L50>
<L45>:
100087c5c:     	rbit	x11, x12
100087c60:     	clz	x11, x11
100087c64:     	add	x10, x10, x11, lsr #3
100087c68:     	and	x11, x10, x9
100087c6c:     	ldrsb	w10, [x8, x11]
100087c70:     	tbz	w10, #0x1f,  <L52>
<L46>:
100087c74:     	sub	x12, x11, #0x8
100087c78:     	and	x9, x12, x9
100087c7c:     	strb	w24, [x8, x11]
100087c80:     	add	x9, x8, x9
100087c84:     	strb	w24, [x9, #0x8]
100087c88:     	sub	x13, x8, x11, lsl #3
100087c8c:     	and	x8, x10, #0x1
100087c90:     	sturh	w23, [x13, #-0x8]
100087c94:     	stur	wzr, [x13, #-0x4]
100087c98:     	ldur	q0, [x20, #0x18]
100087c9c:     	movi.2d	v1, #0xffffffffffffffff
100087ca0:     	mov.d	v1[0], x8
100087ca4:     	sub.2d	v0, v0, v1
100087ca8:     	stur	q0, [x20, #0x18]
<L47>:
100087cac:     	ldur	w8, [x13, #-0x4]
100087cb0:     	add	w8, w8, #0x1
100087cb4:     	stur	w8, [x13, #-0x4]
100087cb8:     	cmp	x19, x21
100087cbc:     	b.eq	 <L103>
<L48>:
100087cc0:     	mov	x10, #0x0               ; =0
100087cc4:     	ldrh	w23, [x19], #0x8
100087cc8:     	mul	x25, x23, x22
100087ccc:     	lsr	x24, x25, #57
100087cd0:     	ldp	x8, x9, [x20, #0x8]
100087cd4:     	dup.8b	v0, w24
100087cd8:     	mov	x11, x25
100087cdc:     	b	 <L41>
<L49>:
100087ce0:     	add	x0, x20, #0x8
100087ce4:     	add	x2, x20, #0x28
100087ce8:     	mov	w1, #0x1                ; =1
100087cec:     	mov	w3, #0x1                ; =1
100087cf0:     	bl	 <__RINvMs6_NtCs3odSxwvi1jy_9hashbrown3rawINtB6_8RawTableTtmEE14reserve_rehashNCINvNtB8_3map11make_hashertmINtNtCsgDHHaSskbIF_4core4hash18BuildHasherDefaultNtCsinZPWOYcTxw_6fxhash8FxHasherEE0ECsfBfIVmeLE1N_8mech_gpu>
100087cf4:     	movi.2d	v3, #0xffffffffffffffff
100087cf8:     	ldp	x8, x9, [x20, #0x8]
100087cfc:     	and	x10, x9, x25
100087d00:     	ldr	d0, [x8, x10]
100087d04:     	cmlt.8b	v0, v0, #0
100087d08:     	fmov	x12, d0
100087d0c:     	cbnz	x12,  <L45>
<L50>:
100087d10:     	mov	w11, #0x8               ; =8
<L51>:
100087d14:     	add	x10, x10, x11
100087d18:     	and	x10, x10, x9
100087d1c:     	ldr	d0, [x8, x10]
100087d20:     	cmlt.8b	v0, v0, #0
100087d24:     	fmov	x12, d0
100087d28:     	add	x11, x11, #0x8
100087d2c:     	cbnz	x12,  <L45>
100087d30:     	b	 <L51>
<L52>:
100087d34:     	ldr	d0, [x8]
100087d38:     	cmlt.8b	v0, v0, #0
100087d3c:     	fmov	x10, d0
100087d40:     	rbit	x10, x10
100087d44:     	clz	x10, x10
100087d48:     	lsr	x11, x10, #3
100087d4c:     	ldrb	w10, [x8, x11]
100087d50:     	b	 <L46>
100087d54:     	cbz	x2,  <L103>
100087d58:     	add	x21, x20, #0x8
100087d5c:     	add	x22, x19, x2, lsl #3
100087d60:     	mov	x23, #0xa95             ; =2709
100087d64:     	movk	x23, #0x2722, lsl #16
100087d68:     	movk	x23, #0xc1b7, lsl #32
100087d6c:     	movk	x23, #0x517c, lsl #48
100087d70:     	b	 <L58>
<L53>:
100087d74:     	and	x13, x13, x11
100087d78:     	ldr	d1, [x12, x13]
100087d7c:     	cmeq.8b	v2, v1, v0
100087d80:     	fmov	x14, d2
100087d84:     	ands	x14, x14, #0x8080808080808080
100087d88:     	b.eq	 <L55>
<L54>:
100087d8c:     	rbit	x15, x14
100087d90:     	clz	x15, x15
100087d94:     	add	x15, x13, x15, lsr #3
100087d98:     	and	x15, x15, x11
100087d9c:     	sub	x15, x12, x15, lsl #3
100087da0:     	ldurb	w16, [x15, #-0x8]
100087da4:     	cmp	w16, w8
100087da8:     	b.eq	 <L56>
100087dac:     	sub	x15, x14, #0x1
100087db0:     	ands	x14, x15, x14
100087db4:     	b.ne	 <L54>
<L55>:
100087db8:     	movi.2d	v2, #0xffffffffffffffff
100087dbc:     	cmeq.8b	v1, v1, v2
100087dc0:     	umaxv.8b	b1, v1
100087dc4:     	fmov	w14, s1
100087dc8:     	tbnz	w14, #0x0,  <L59>
100087dcc:     	add	x10, x10, #0x8
100087dd0:     	add	x13, x13, x10
100087dd4:     	b	 <L53>
<L56>:
100087dd8:     	mov	x10, #0x0               ; =0
100087ddc:     	stp	x15, x21, [sp, #0x8]
100087de0:     	str	x9, [sp, #0x18]
<L57>:
100087de4:     	str	x10, [sp]
100087de8:     	mov	x0, sp
100087dec:     	bl	 <__RNvMs1d_NtCsd0jj78El3J_9hashbrown3mapINtB6_5EntryhmINtNtCsgDHHaSskbIF_4core4hash18BuildHasherDefaultNtCsinZPWOYcTxw_6fxhash8FxHasherEE9or_insertCsfBfIVmeLE1N_8mech_gpu>
100087df0:     	ldr	w8, [x0]
100087df4:     	add	w8, w8, #0x1
100087df8:     	str	w8, [x0]
100087dfc:     	cmp	x19, x22
100087e00:     	b.eq	 <L103>
<L58>:
100087e04:     	mov	x10, #0x0               ; =0
100087e08:     	ldrb	w8, [x19], #0x8
100087e0c:     	mul	x9, x8, x23
100087e10:     	lsr	x13, x9, #57
100087e14:     	ldp	x12, x11, [x20, #0x8]
100087e18:     	dup.8b	v0, w13
100087e1c:     	mov	x13, x9
100087e20:     	b	 <L53>
<L59>:
100087e24:     	stp	x21, x9, [sp, #0x8]
100087e28:     	mov	w10, #0x1               ; =1
100087e2c:     	strb	w8, [sp, #0x18]
100087e30:     	b	 <L57>
100087e34:     	cbz	x2,  <L103>
100087e38:     	add	x21, x20, #0x8
100087e3c:     	add	x22, x19, x2, lsl #3
100087e40:     	mov	x23, #0xa95             ; =2709
100087e44:     	movk	x23, #0x2722, lsl #16
100087e48:     	movk	x23, #0xc1b7, lsl #32
100087e4c:     	movk	x23, #0x517c, lsl #48
100087e50:     	b	 <L65>
<L60>:
100087e54:     	and	x13, x13, x11
100087e58:     	ldr	d1, [x12, x13]
100087e5c:     	cmeq.8b	v2, v1, v0
100087e60:     	fmov	x14, d2
100087e64:     	ands	x14, x14, #0x8080808080808080
100087e68:     	b.eq	 <L62>
<L61>:
100087e6c:     	rbit	x15, x14
100087e70:     	clz	x15, x15
100087e74:     	add	x15, x13, x15, lsr #3
100087e78:     	and	x15, x15, x11
100087e7c:     	sub	x15, x12, x15, lsl #4
100087e80:     	ldur	x16, [x15, #-0x10]
100087e84:     	cmp	x9, x16
100087e88:     	b.eq	 <L63>
100087e8c:     	sub	x15, x14, #0x1
100087e90:     	ands	x14, x15, x14
100087e94:     	b.ne	 <L61>
<L62>:
100087e98:     	movi.2d	v2, #0xffffffffffffffff
100087e9c:     	cmeq.8b	v1, v1, v2
100087ea0:     	umaxv.8b	b1, v1
100087ea4:     	fmov	w14, s1
100087ea8:     	tbnz	w14, #0x0,  <L66>
100087eac:     	add	x10, x10, #0x8
100087eb0:     	add	x13, x13, x10
100087eb4:     	b	 <L60>
<L63>:
100087eb8:     	mov	x9, #0x0                ; =0
100087ebc:     	str	x15, [sp, #0x8]
100087ec0:     	add	x10, x20, #0x8
<L64>:
100087ec4:     	stp	x10, x8, [sp, #0x10]
100087ec8:     	str	x9, [sp]
100087ecc:     	mov	x0, sp
100087ed0:     	bl	 <__RNvMs1d_NtCsd0jj78El3J_9hashbrown3mapINtB6_5EntryyyINtNtCsgDHHaSskbIF_4core4hash18BuildHasherDefaultNtCsinZPWOYcTxw_6fxhash8FxHasherEE9or_insertCsfBfIVmeLE1N_8mech_gpu>
100087ed4:     	ldr	x8, [x0]
100087ed8:     	add	x8, x8, #0x1
100087edc:     	str	x8, [x0]
100087ee0:     	cmp	x19, x22
100087ee4:     	b.eq	 <L103>
<L65>:
100087ee8:     	mov	x10, #0x0               ; =0
100087eec:     	ldr	x9, [x19], #0x8
100087ef0:     	mul	x8, x9, x23
100087ef4:     	lsr	x13, x8, #57
100087ef8:     	ldp	x12, x11, [x20, #0x8]
100087efc:     	dup.8b	v0, w13
100087f00:     	mov	x13, x8
100087f04:     	b	 <L60>
<L66>:
100087f08:     	str	x21, [sp, #0x8]
100087f0c:     	mov	x10, x8
100087f10:     	mov	x8, x9
100087f14:     	mov	w9, #0x1                ; =1
100087f18:     	b	 <L64>
100087f1c:     	cbz	x2,  <L103>
100087f20:     	add	x21, x19, x2, lsl #3
100087f24:     	mov	x22, #0xa95             ; =2709
100087f28:     	movk	x22, #0x2722, lsl #16
100087f2c:     	movk	x22, #0xc1b7, lsl #32
100087f30:     	movk	x22, #0x517c, lsl #48
100087f34:     	movi.2d	v3, #0xffffffffffffffff
100087f38:     	b	 <L74>
<L67>:
100087f3c:     	and	x11, x11, x9
100087f40:     	ldr	d1, [x8, x11]
100087f44:     	cmeq.8b	v2, v1, v0
100087f48:     	fmov	x12, d2
100087f4c:     	ands	x12, x12, #0x8080808080808080
100087f50:     	b.eq	 <L69>
<L68>:
100087f54:     	rbit	x13, x12
100087f58:     	clz	x13, x13
100087f5c:     	add	x13, x11, x13, lsr #3
100087f60:     	and	x13, x13, x9
100087f64:     	sub	x13, x8, x13, lsl #3
100087f68:     	ldurb	w14, [x13, #-0x8]
100087f6c:     	cmp	w14, w23
100087f70:     	b.eq	 <L73>
100087f74:     	sub	x13, x12, #0x1
100087f78:     	ands	x12, x13, x12
100087f7c:     	b.ne	 <L68>
<L69>:
100087f80:     	cmeq.8b	v1, v1, v3
100087f84:     	umaxv.8b	b1, v1
100087f88:     	fmov	w12, s1
100087f8c:     	tbnz	w12, #0x0,  <L70>
100087f90:     	add	x10, x10, #0x8
100087f94:     	add	x11, x11, x10
100087f98:     	b	 <L67>
<L70>:
100087f9c:     	ldr	x10, [x20, #0x18]
100087fa0:     	cbz	x10,  <L75>
100087fa4:     	and	x10, x9, x25
100087fa8:     	ldr	d0, [x8, x10]
100087fac:     	cmlt.8b	v0, v0, #0
100087fb0:     	fmov	x12, d0
100087fb4:     	cbz	x12,  <L76>
<L71>:
100087fb8:     	rbit	x11, x12
100087fbc:     	clz	x11, x11
100087fc0:     	add	x10, x10, x11, lsr #3
100087fc4:     	and	x11, x10, x9
100087fc8:     	ldrsb	w10, [x8, x11]
100087fcc:     	tbz	w10, #0x1f,  <L78>
<L72>:
100087fd0:     	sub	x12, x11, #0x8
100087fd4:     	and	x9, x12, x9
100087fd8:     	strb	w24, [x8, x11]
100087fdc:     	add	x9, x8, x9
100087fe0:     	strb	w24, [x9, #0x8]
100087fe4:     	sub	x13, x8, x11, lsl #3
100087fe8:     	and	x8, x10, #0x1
100087fec:     	sturb	w23, [x13, #-0x8]
100087ff0:     	stur	wzr, [x13, #-0x4]
100087ff4:     	ldur	q0, [x20, #0x18]
100087ff8:     	movi.2d	v1, #0xffffffffffffffff
100087ffc:     	mov.d	v1[0], x8
100088000:     	sub.2d	v0, v0, v1
100088004:     	stur	q0, [x20, #0x18]
<L73>:
100088008:     	ldur	w8, [x13, #-0x4]
10008800c:     	add	w8, w8, #0x1
100088010:     	stur	w8, [x13, #-0x4]
100088014:     	cmp	x19, x21
100088018:     	b.eq	 <L103>
<L74>:
10008801c:     	mov	x10, #0x0               ; =0
100088020:     	ldrb	w23, [x19], #0x8
100088024:     	mul	x25, x23, x22
100088028:     	lsr	x24, x25, #57
10008802c:     	ldp	x8, x9, [x20, #0x8]
100088030:     	dup.8b	v0, w24
100088034:     	mov	x11, x25
100088038:     	b	 <L67>
<L75>:
10008803c:     	add	x0, x20, #0x8
100088040:     	add	x2, x20, #0x28
100088044:     	mov	w1, #0x1                ; =1
100088048:     	mov	w3, #0x1                ; =1
10008804c:     	bl	 <__RINvMs6_NtCs3odSxwvi1jy_9hashbrown3rawINtB6_8RawTableThmEE14reserve_rehashNCINvNtB8_3map11make_hasherhmINtNtCsgDHHaSskbIF_4core4hash18BuildHasherDefaultNtCsinZPWOYcTxw_6fxhash8FxHasherEE0ECsfBfIVmeLE1N_8mech_gpu>
100088050:     	movi.2d	v3, #0xffffffffffffffff
100088054:     	ldp	x8, x9, [x20, #0x8]
100088058:     	and	x10, x9, x25
10008805c:     	ldr	d0, [x8, x10]
100088060:     	cmlt.8b	v0, v0, #0
100088064:     	fmov	x12, d0
100088068:     	cbnz	x12,  <L71>
<L76>:
10008806c:     	mov	w11, #0x8               ; =8
<L77>:
100088070:     	add	x10, x10, x11
100088074:     	and	x10, x10, x9
100088078:     	ldr	d0, [x8, x10]
10008807c:     	cmlt.8b	v0, v0, #0
100088080:     	fmov	x12, d0
100088084:     	add	x11, x11, #0x8
100088088:     	cbnz	x12,  <L71>
10008808c:     	b	 <L77>
<L78>:
100088090:     	ldr	d0, [x8]
100088094:     	cmlt.8b	v0, v0, #0
100088098:     	fmov	x10, d0
10008809c:     	rbit	x10, x10
1000880a0:     	clz	x10, x10
1000880a4:     	lsr	x11, x10, #3
1000880a8:     	ldrb	w10, [x8, x11]
1000880ac:     	b	 <L72>
1000880b0:     	cbz	x2,  <L103>
1000880b4:     	add	x21, x19, x2, lsl #3
1000880b8:     	mov	x22, #0xa95             ; =2709
1000880bc:     	movk	x22, #0x2722, lsl #16
1000880c0:     	movk	x22, #0xc1b7, lsl #32
1000880c4:     	movk	x22, #0x517c, lsl #48
1000880c8:     	movi.2d	v3, #0xffffffffffffffff
1000880cc:     	b	 <L86>
<L79>:
1000880d0:     	and	x11, x11, x9
1000880d4:     	ldr	d1, [x8, x11]
1000880d8:     	cmeq.8b	v2, v1, v0
1000880dc:     	fmov	x12, d2
1000880e0:     	ands	x12, x12, #0x8080808080808080
1000880e4:     	b.eq	 <L81>
<L80>:
1000880e8:     	rbit	x13, x12
1000880ec:     	clz	x13, x13
1000880f0:     	add	x13, x11, x13, lsr #3
1000880f4:     	and	x13, x13, x9
1000880f8:     	sub	x13, x8, x13, lsl #4
1000880fc:     	ldur	x14, [x13, #-0x10]
100088100:     	cmp	x14, x23
100088104:     	b.eq	 <L85>
100088108:     	sub	x13, x12, #0x1
10008810c:     	ands	x12, x13, x12
100088110:     	b.ne	 <L80>
<L81>:
100088114:     	cmeq.8b	v1, v1, v3
100088118:     	umaxv.8b	b1, v1
10008811c:     	fmov	w12, s1
100088120:     	tbnz	w12, #0x0,  <L82>
100088124:     	add	x10, x10, #0x8
100088128:     	add	x11, x11, x10
10008812c:     	b	 <L79>
<L82>:
100088130:     	ldr	x10, [x20, #0x18]
100088134:     	cbz	x10,  <L87>
100088138:     	and	x10, x9, x25
10008813c:     	ldr	d0, [x8, x10]
100088140:     	cmlt.8b	v0, v0, #0
100088144:     	fmov	x12, d0
100088148:     	cbz	x12,  <L88>
<L83>:
10008814c:     	rbit	x11, x12
100088150:     	clz	x11, x11
100088154:     	add	x10, x10, x11, lsr #3
100088158:     	and	x11, x10, x9
10008815c:     	ldrsb	w10, [x8, x11]
100088160:     	tbz	w10, #0x1f,  <L90>
<L84>:
100088164:     	sub	x12, x11, #0x8
100088168:     	and	x9, x12, x9
10008816c:     	strb	w24, [x8, x11]
100088170:     	add	x9, x8, x9
100088174:     	strb	w24, [x9, #0x8]
100088178:     	sub	x13, x8, x11, lsl #4
10008817c:     	and	x8, x10, #0x1
100088180:     	stp	x23, xzr, [x13, #-0x10]
100088184:     	ldur	q0, [x20, #0x18]
100088188:     	movi.2d	v1, #0xffffffffffffffff
10008818c:     	mov.d	v1[0], x8
100088190:     	sub.2d	v0, v0, v1
100088194:     	stur	q0, [x20, #0x18]
<L85>:
100088198:     	ldur	x8, [x13, #-0x8]
10008819c:     	add	x8, x8, #0x1
1000881a0:     	stur	x8, [x13, #-0x8]
1000881a4:     	cmp	x19, x21
1000881a8:     	b.eq	 <L103>
<L86>:
1000881ac:     	mov	x10, #0x0               ; =0
1000881b0:     	ldr	x23, [x19], #0x8
1000881b4:     	mul	x25, x23, x22
1000881b8:     	lsr	x24, x25, #57
1000881bc:     	ldp	x8, x9, [x20, #0x8]
1000881c0:     	dup.8b	v0, w24
1000881c4:     	mov	x11, x25
1000881c8:     	b	 <L79>
<L87>:
1000881cc:     	add	x0, x20, #0x8
1000881d0:     	add	x2, x20, #0x28
1000881d4:     	mov	w1, #0x1                ; =1
1000881d8:     	mov	w3, #0x1                ; =1
1000881dc:     	bl	 <__RINvMs6_NtCs3odSxwvi1jy_9hashbrown3rawINtB6_8RawTableTyyEE14reserve_rehashNCINvNtB8_3map11make_hasheryyINtNtCsgDHHaSskbIF_4core4hash18BuildHasherDefaultNtCsinZPWOYcTxw_6fxhash8FxHasherEE0ECsfBfIVmeLE1N_8mech_gpu>
1000881e0:     	movi.2d	v3, #0xffffffffffffffff
1000881e4:     	ldp	x8, x9, [x20, #0x8]
1000881e8:     	and	x10, x9, x25
1000881ec:     	ldr	d0, [x8, x10]
1000881f0:     	cmlt.8b	v0, v0, #0
1000881f4:     	fmov	x12, d0
1000881f8:     	cbnz	x12,  <L83>
<L88>:
1000881fc:     	mov	w11, #0x8               ; =8
<L89>:
100088200:     	add	x10, x10, x11
100088204:     	and	x10, x10, x9
100088208:     	ldr	d0, [x8, x10]
10008820c:     	cmlt.8b	v0, v0, #0
100088210:     	fmov	x12, d0
100088214:     	add	x11, x11, #0x8
100088218:     	cbnz	x12,  <L83>
10008821c:     	b	 <L89>
<L90>:
100088220:     	ldr	d0, [x8]
100088224:     	cmlt.8b	v0, v0, #0
100088228:     	fmov	x10, d0
10008822c:     	rbit	x10, x10
100088230:     	clz	x10, x10
100088234:     	lsr	x11, x10, #3
100088238:     	ldrb	w10, [x8, x11]
10008823c:     	b	 <L84>
100088240:     	cbz	x2,  <L103>
100088244:     	add	x21, x19, x2, lsl #3
100088248:     	mov	x22, #0xa95             ; =2709
10008824c:     	movk	x22, #0x2722, lsl #16
100088250:     	movk	x22, #0xc1b7, lsl #32
100088254:     	movk	x22, #0x517c, lsl #48
100088258:     	movi.2d	v3, #0xffffffffffffffff
10008825c:     	b	 <L98>
<L91>:
100088260:     	and	x11, x11, x9
100088264:     	ldr	d1, [x8, x11]
100088268:     	cmeq.8b	v2, v1, v0
10008826c:     	fmov	x12, d2
100088270:     	ands	x12, x12, #0x8080808080808080
100088274:     	b.eq	 <L93>
<L92>:
100088278:     	rbit	x13, x12
10008827c:     	clz	x13, x13
100088280:     	add	x13, x11, x13, lsr #3
100088284:     	and	x13, x13, x9
100088288:     	sub	x13, x8, x13, lsl #3
10008828c:     	ldur	w14, [x13, #-0x8]
100088290:     	cmp	w14, w23
100088294:     	b.eq	 <L97>
100088298:     	sub	x13, x12, #0x1
10008829c:     	ands	x12, x13, x12
1000882a0:     	b.ne	 <L92>
<L93>:
1000882a4:     	cmeq.8b	v1, v1, v3
1000882a8:     	umaxv.8b	b1, v1
1000882ac:     	fmov	w12, s1
1000882b0:     	tbnz	w12, #0x0,  <L94>
1000882b4:     	add	x10, x10, #0x8
1000882b8:     	add	x11, x11, x10
1000882bc:     	b	 <L91>
<L94>:
1000882c0:     	ldr	x10, [x20, #0x18]
1000882c4:     	cbz	x10,  <L99>
1000882c8:     	and	x10, x9, x25
1000882cc:     	ldr	d0, [x8, x10]
1000882d0:     	cmlt.8b	v0, v0, #0
1000882d4:     	fmov	x12, d0
1000882d8:     	cbz	x12,  <L100>
<L95>:
1000882dc:     	rbit	x11, x12
1000882e0:     	clz	x11, x11
1000882e4:     	add	x10, x10, x11, lsr #3
1000882e8:     	and	x11, x10, x9
1000882ec:     	ldrsb	w10, [x8, x11]
1000882f0:     	tbz	w10, #0x1f,  <L102>
<L96>:
1000882f4:     	sub	x12, x11, #0x8
1000882f8:     	and	x9, x12, x9
1000882fc:     	strb	w24, [x8, x11]
100088300:     	add	x9, x8, x9
100088304:     	strb	w24, [x9, #0x8]
100088308:     	sub	x13, x8, x11, lsl #3
10008830c:     	and	x8, x10, #0x1
100088310:     	stp	w23, wzr, [x13, #-0x8]
100088314:     	ldur	q0, [x20, #0x18]
100088318:     	movi.2d	v1, #0xffffffffffffffff
10008831c:     	mov.d	v1[0], x8
100088320:     	sub.2d	v0, v0, v1
100088324:     	stur	q0, [x20, #0x18]
<L97>:
100088328:     	ldur	w8, [x13, #-0x4]
10008832c:     	add	w8, w8, #0x1
100088330:     	stur	w8, [x13, #-0x4]
100088334:     	cmp	x19, x21
100088338:     	b.eq	 <L103>
<L98>:
10008833c:     	mov	x10, #0x0               ; =0
100088340:     	ldr	x23, [x19], #0x8
100088344:     	mov	w8, w23
100088348:     	mul	x25, x8, x22
10008834c:     	lsr	x24, x25, #57
100088350:     	ldp	x8, x9, [x20, #0x8]
100088354:     	dup.8b	v0, w24
100088358:     	mov	x11, x25
10008835c:     	b	 <L91>
<L99>:
100088360:     	add	x0, x20, #0x8
100088364:     	add	x2, x20, #0x28
100088368:     	mov	w1, #0x1                ; =1
10008836c:     	mov	w3, #0x1                ; =1
100088370:     	bl	 <__RINvMs6_NtCs3odSxwvi1jy_9hashbrown3rawINtB6_8RawTableTmmEE14reserve_rehashNCINvNtB8_3map11make_hashermmINtNtCsgDHHaSskbIF_4core4hash18BuildHasherDefaultNtCsinZPWOYcTxw_6fxhash8FxHasherEE0ECsfBfIVmeLE1N_8mech_gpu>
100088374:     	movi.2d	v3, #0xffffffffffffffff
100088378:     	ldp	x8, x9, [x20, #0x8]
10008837c:     	and	x10, x9, x25
100088380:     	ldr	d0, [x8, x10]
100088384:     	cmlt.8b	v0, v0, #0
100088388:     	fmov	x12, d0
10008838c:     	cbnz	x12,  <L95>
<L100>:
100088390:     	mov	w11, #0x8               ; =8
<L101>:
100088394:     	add	x10, x10, x11
100088398:     	and	x10, x10, x9
10008839c:     	ldr	d0, [x8, x10]
1000883a0:     	cmlt.8b	v0, v0, #0
1000883a4:     	fmov	x12, d0
1000883a8:     	add	x11, x11, #0x8
1000883ac:     	cbnz	x12,  <L95>
1000883b0:     	b	 <L101>
<L102>:
1000883b4:     	ldr	d0, [x8]
1000883b8:     	cmlt.8b	v0, v0, #0
1000883bc:     	fmov	x10, d0
1000883c0:     	rbit	x10, x10
1000883c4:     	clz	x10, x10
1000883c8:     	lsr	x11, x10, #3
1000883cc:     	ldrb	w10, [x8, x11]
1000883d0:     	b	 <L96>
<L103>:
1000883d4:     	ldp	x29, x30, [sp, #0x60]
1000883d8:     	ldp	x20, x19, [sp, #0x50]
1000883dc:     	ldp	x22, x21, [sp, #0x40]
1000883e0:     	ldp	x24, x23, [sp, #0x30]
1000883e4:     	ldp	x26, x25, [sp, #0x20]
1000883e8:     	add	sp, sp, #0x70
1000883ec:     	ret
