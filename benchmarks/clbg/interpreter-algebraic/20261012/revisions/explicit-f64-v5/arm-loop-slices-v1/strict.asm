00000001000f72e8 <mech_gpu::sequence::runtime::resident::scalar_row>:
1000f72e8:     	sub	sp, sp, #0xe0
1000f72ec:     	stp	d9, d8, [sp, #0x70]
1000f72f0:     	stp	x28, x27, [sp, #0x80]
1000f72f4:     	stp	x26, x25, [sp, #0x90]
1000f72f8:     	stp	x24, x23, [sp, #0xa0]
1000f72fc:     	stp	x22, x21, [sp, #0xb0]
1000f7300:     	stp	x20, x19, [sp, #0xc0]
1000f7304:     	stp	x29, x30, [sp, #0xd0]
1000f7308:     	add	x29, sp, #0xd0
1000f730c:     	str	w7, [sp, #0x44]
1000f7310:     	mov	x20, x4
1000f7314:     	mov	x21, x3
1000f7318:     	mov	x25, x0
1000f731c:     	ldr	x8, [x2]
1000f7320:     	mov	x9, #-0x8000000000000000 ; =-9223372036854775808
1000f7324:     	cmp	x8, x9
1000f7328:     	b.ne	0x1000f73d0 <mech_gpu::sequence::runtime::resident::scalar_row+0xe8>
1000f732c:     	cbz	x5, 0x1000f8424 <mech_gpu::sequence::runtime::resident::scalar_row+0x113c>
1000f7330:     	mov	x22, x1
1000f7334:     	lsl	x19, x5, #3
1000f7338:     	ldr	w8, [sp, #0x44]
1000f733c:     	cbz	w8, 0x1000f8450 <mech_gpu::sequence::runtime::resident::scalar_row+0x1168>
1000f7340:     	mov	x23, #0x0               ; =0
1000f7344:     	movi.2d	v8, #0000000000000000
1000f7348:     	mov	x24, #-0x8000000000000000 ; =-9223372036854775808
1000f734c:     	tbz	w6, #0x0, 0x1000f7390 <mech_gpu::sequence::runtime::resident::scalar_row+0xa8>
1000f7350:     	ldr	d9, [x20, x23, lsl #3]
1000f7354:     	stp	x23, x21, [sp, #0x48]
1000f7358:     	add	x0, sp, #0x58
1000f735c:     	add	x2, sp, #0x48
1000f7360:     	mov	x1, x22
1000f7364:     	bl	0x1000e637c <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
1000f7368:     	ldr	x8, [sp, #0x58]
1000f736c:     	cmp	x8, x24
1000f7370:     	b.ne	0x1000f84e0 <mech_gpu::sequence::runtime::resident::scalar_row+0x11f8>
1000f7374:     	add	x23, x23, #0x1
1000f7378:     	ldr	d0, [sp, #0x60]
1000f737c:     	fdiv	d0, d9, d0
1000f7380:     	fadd	d8, d8, d0
1000f7384:     	subs	x19, x19, #0x8
1000f7388:     	b.ne	0x1000f7350 <mech_gpu::sequence::runtime::resident::scalar_row+0x68>
1000f738c:     	b	0x1000f8428 <mech_gpu::sequence::runtime::resident::scalar_row+0x1140>
1000f7390:     	ldr	d9, [x20, x23, lsl #3]
1000f7394:     	stp	x21, x23, [sp, #0x48]
1000f7398:     	add	x0, sp, #0x58
1000f739c:     	add	x2, sp, #0x48
1000f73a0:     	mov	x1, x22
1000f73a4:     	bl	0x1000e637c <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
1000f73a8:     	ldr	x8, [sp, #0x58]
1000f73ac:     	cmp	x8, x24
1000f73b0:     	b.ne	0x1000f84e0 <mech_gpu::sequence::runtime::resident::scalar_row+0x11f8>
1000f73b4:     	add	x23, x23, #0x1
1000f73b8:     	ldr	d0, [sp, #0x60]
1000f73bc:     	fdiv	d0, d9, d0
1000f73c0:     	fadd	d8, d8, d0
1000f73c4:     	subs	x19, x19, #0x8
1000f73c8:     	b.ne	0x1000f7390 <mech_gpu::sequence::runtime::resident::scalar_row+0xa8>
1000f73cc:     	b	0x1000f8428 <mech_gpu::sequence::runtime::resident::scalar_row+0x1140>
1000f73d0:     	str	x25, [sp, #0x8]
1000f73d4:     	cbz	x5, 0x1000f8438 <mech_gpu::sequence::runtime::resident::scalar_row+0x1150>
1000f73d8:     	mov	x11, #0x0               ; =0
1000f73dc:     	mov	x12, #0x0               ; =0
1000f73e0:     	ldp	x9, x10, [x2, #0x20]
1000f73e4:     	mov	w8, #0x18               ; =24
1000f73e8:     	str	x9, [sp, #0x18]
1000f73ec:     	madd	x15, x10, x8, x9
1000f73f0:     	ldp	x16, x1, [x2, #0x8]
1000f73f4:     	ucvtf	d0, x21
1000f73f8:     	ldp	x8, x9, [x2, #0x38]
1000f73fc:     	str	x8, [sp, #0x10]
1000f7400:     	stp	x9, x10, [sp, #0x30]
1000f7404:     	add	x4, x8, x9, lsl #5
1000f7408:     	ldp	x8, x6, [x2, #0x48]
1000f740c:     	add	x13, x16, x8, lsl #8
1000f7410:     	fmov	x22, d0
1000f7414:     	adrp	x26, 0x101882000 <_anon.f7c957ecca79fd017cf77e2dfd4b2b6d.428+0x4>
1000f7418:     	add	x26, x26, #0x64c
1000f741c:     	movi.2d	v0, #0000000000000000
1000f7420:     	mov	w28, #0x8               ; =8
1000f7424:     	stp	x13, x8, [sp, #0x20]
1000f7428:     	b	0x1000f7440 <mech_gpu::sequence::runtime::resident::scalar_row+0x158>
1000f742c:     	add	x20, x20, x30, lsl #3
1000f7430:     	add	x12, x12, #0x1
1000f7434:     	add	x11, x11, #0x20
1000f7438:     	sub	x5, x5, x30
1000f743c:     	cbz	x5, 0x1000f843c <mech_gpu::sequence::runtime::resident::scalar_row+0x1154>
1000f7440:     	cmp	x5, #0x20
1000f7444:     	mov	w8, #0x20               ; =32
1000f7448:     	csel	x30, x5, x8, lo
1000f744c:     	ldr	x8, [sp, #0x38]
1000f7450:     	cbz	x8, 0x1000f7624 <mech_gpu::sequence::runtime::resident::scalar_row+0x33c>
1000f7454:     	lsl	x8, x12, #5
1000f7458:     	lsl	x9, x30, #3
1000f745c:     	sub	x10, x9, #0x8
1000f7460:     	lsr	x13, x10, #3
1000f7464:     	add	x13, x13, #0x1
1000f7468:     	and	x14, x13, #0x3ffffffffffffff8
1000f746c:     	dup.2d	v1, x8
1000f7470:     	lsl	x17, x14, #3
1000f7474:     	mov	w8, #0x2                ; =2
1000f7478:     	dup.2d	v2, x8
1000f747c:     	add.2d	v2, v1, v2
1000f7480:     	mov	w8, #0x4                ; =4
1000f7484:     	dup.2d	v3, x8
1000f7488:     	add.2d	v3, v1, v3
1000f748c:     	mov	w8, #0x6                ; =6
1000f7490:     	dup.2d	v4, x8
1000f7494:     	add.2d	v4, v1, v4
1000f7498:     	ldr	x0, [sp, #0x18]
1000f749c:     	mov	x3, x0
1000f74a0:     	b	0x1000f74b0 <mech_gpu::sequence::runtime::resident::scalar_row+0x1c8>
1000f74a4:     	mov	x0, x3
1000f74a8:     	cmp	x3, x15
1000f74ac:     	b.eq	0x1000f7624 <mech_gpu::sequence::runtime::resident::scalar_row+0x33c>
1000f74b0:     	ldr	x8, [x3], #0x18
1000f74b4:     	cmp	x8, x1
1000f74b8:     	b.hs	0x1000f853c <mech_gpu::sequence::runtime::resident::scalar_row+0x1254>
1000f74bc:     	ldr	x2, [x0, #0x10]
1000f74c0:     	ldrb	w0, [x0, #0x8]
1000f74c4:     	add	x19, x16, x8, lsl #8
1000f74c8:     	add	x25, x19, x9
1000f74cc:     	cmp	x2, x6
1000f74d0:     	b.ne	0x1000f74ec <mech_gpu::sequence::runtime::resident::scalar_row+0x204>
1000f74d4:     	cmp	w0, #0x1
1000f74d8:     	b.ne	0x1000f7538 <mech_gpu::sequence::runtime::resident::scalar_row+0x250>
1000f74dc:     	cmp	x10, #0x38
1000f74e0:     	b.hs	0x1000f7548 <mech_gpu::sequence::runtime::resident::scalar_row+0x260>
1000f74e4:     	mov	x8, #0x0                ; =0
1000f74e8:     	b	0x1000f7594 <mech_gpu::sequence::runtime::resident::scalar_row+0x2ac>
1000f74ec:     	cmp	w0, #0x1
1000f74f0:     	csel	x7, x21, x22, eq
1000f74f4:     	cmp	x10, #0x38
1000f74f8:     	b.lo	0x1000f7528 <mech_gpu::sequence::runtime::resident::scalar_row+0x240>
1000f74fc:     	add	x19, x19, x17
1000f7500:     	dup.2d	v5, x7
1000f7504:     	add	x0, x16, #0x20
1000f7508:     	add	x8, x0, x8, lsl #8
1000f750c:     	and	x0, x13, #0x3ffffffffffffff8
1000f7510:     	stp	q5, q5, [x8, #-0x20]
1000f7514:     	stp	q5, q5, [x8], #0x40
1000f7518:     	subs	x0, x0, #0x8
1000f751c:     	b.ne	0x1000f7510 <mech_gpu::sequence::runtime::resident::scalar_row+0x228>
1000f7520:     	cmp	x13, x14
1000f7524:     	b.eq	0x1000f74a4 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000f7528:     	str	x7, [x19], #0x8
1000f752c:     	cmp	x19, x25
1000f7530:     	b.ne	0x1000f7528 <mech_gpu::sequence::runtime::resident::scalar_row+0x240>
1000f7534:     	b	0x1000f74a4 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000f7538:     	cmp	x10, #0x38
1000f753c:     	b.hs	0x1000f75ac <mech_gpu::sequence::runtime::resident::scalar_row+0x2c4>
1000f7540:     	mov	x8, #0x0                ; =0
1000f7544:     	b	0x1000f7608 <mech_gpu::sequence::runtime::resident::scalar_row+0x320>
1000f7548:     	add	x19, x19, x17
1000f754c:     	add	x0, x16, #0x20
1000f7550:     	add	x8, x0, x8, lsl #8
1000f7554:     	adrp	x0, 0x10187e000 <_anon.f125f6eaa94008c055d34212637ea7ab.962+0x1800>
1000f7558:     	ldr	q5, [x0, #0xae0]
1000f755c:     	and	x0, x13, #0x3ffffffffffffff8
1000f7560:     	add.2d	v6, v5, v1
1000f7564:     	add.2d	v7, v5, v2
1000f7568:     	add.2d	v16, v5, v3
1000f756c:     	add.2d	v17, v5, v4
1000f7570:     	stp	q6, q7, [x8, #-0x20]
1000f7574:     	stp	q16, q17, [x8], #0x40
1000f7578:     	dup.2d	v6, x28
1000f757c:     	add.2d	v5, v5, v6
1000f7580:     	subs	x0, x0, #0x8
1000f7584:     	b.ne	0x1000f7560 <mech_gpu::sequence::runtime::resident::scalar_row+0x278>
1000f7588:     	and	x8, x13, #0x3ffffffffffffff8
1000f758c:     	cmp	x13, x14
1000f7590:     	b.eq	0x1000f74a4 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000f7594:     	add	x8, x11, x8
1000f7598:     	str	x8, [x19], #0x8
1000f759c:     	add	x8, x8, #0x1
1000f75a0:     	cmp	x19, x25
1000f75a4:     	b.ne	0x1000f7598 <mech_gpu::sequence::runtime::resident::scalar_row+0x2b0>
1000f75a8:     	b	0x1000f74a4 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000f75ac:     	add	x19, x19, x17
1000f75b0:     	add	x0, x16, #0x20
1000f75b4:     	add	x8, x0, x8, lsl #8
1000f75b8:     	adrp	x0, 0x10187e000 <_anon.f125f6eaa94008c055d34212637ea7ab.962+0x1800>
1000f75bc:     	ldr	q5, [x0, #0xae0]
1000f75c0:     	and	x7, x13, #0x3ffffffffffffff8
1000f75c4:     	add.2d	v6, v5, v1
1000f75c8:     	add.2d	v7, v5, v2
1000f75cc:     	add.2d	v16, v5, v3
1000f75d0:     	add.2d	v17, v5, v4
1000f75d4:     	ucvtf.2d	v6, v6
1000f75d8:     	ucvtf.2d	v7, v7
1000f75dc:     	ucvtf.2d	v16, v16
1000f75e0:     	ucvtf.2d	v17, v17
1000f75e4:     	stp	q6, q7, [x8, #-0x20]
1000f75e8:     	stp	q16, q17, [x8], #0x40
1000f75ec:     	dup.2d	v6, x28
1000f75f0:     	add.2d	v5, v5, v6
1000f75f4:     	subs	x7, x7, #0x8
1000f75f8:     	b.ne	0x1000f75c4 <mech_gpu::sequence::runtime::resident::scalar_row+0x2dc>
1000f75fc:     	and	x8, x13, #0x3ffffffffffffff8
1000f7600:     	cmp	x13, x14
1000f7604:     	b.eq	0x1000f74a4 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000f7608:     	add	x8, x11, x8
1000f760c:     	ucvtf	d5, x8
1000f7610:     	str	d5, [x19], #0x8
1000f7614:     	add	x8, x8, #0x1
1000f7618:     	cmp	x19, x25
1000f761c:     	b.ne	0x1000f760c <mech_gpu::sequence::runtime::resident::scalar_row+0x324>
1000f7620:     	b	0x1000f74a4 <mech_gpu::sequence::runtime::resident::scalar_row+0x1bc>
1000f7624:     	ldr	x8, [sp, #0x30]
1000f7628:     	cbz	x8, 0x1000f80d0 <mech_gpu::sequence::runtime::resident::scalar_row+0xde8>
1000f762c:     	and	x19, x30, #0x38
1000f7630:     	and	x8, x30, #0xfffffffffffffffe
1000f7634:     	neg	x25, x8
1000f7638:     	ldr	x17, [sp, #0x10]
1000f763c:     	b	0x1000f765c <mech_gpu::sequence::runtime::resident::scalar_row+0x374>
1000f7640:     	mov	x8, #0x0                ; =0
1000f7644:     	ldr	x9, [x14, x8, lsl #3]
1000f7648:     	ldr	x10, [x3, x8, lsl #3]
1000f764c:     	mul	x9, x10, x9
1000f7650:     	str	x9, [x13, x8, lsl #3]
1000f7654:     	cmp	x17, x4
1000f7658:     	b.eq	0x1000f80d0 <mech_gpu::sequence::runtime::resident::scalar_row+0xde8>
1000f765c:     	ldr	x8, [x17, #0x18]
1000f7660:     	cmp	x8, x1
1000f7664:     	b.hi	0x1000f8518 <mech_gpu::sequence::runtime::resident::scalar_row+0x1230>
1000f7668:     	cmp	x1, x8
1000f766c:     	b.eq	0x1000f8574 <mech_gpu::sequence::runtime::resident::scalar_row+0x128c>
1000f7670:     	mov	x13, x17
1000f7674:     	ldr	x9, [x17, #0x8]
1000f7678:     	subs	x24, x8, x9
1000f767c:     	b.ls	0x1000f8560 <mech_gpu::sequence::runtime::resident::scalar_row+0x1278>
1000f7680:     	ldr	x10, [x13, #0x10]
1000f7684:     	subs	x7, x8, x10
1000f7688:     	b.ls	0x1000f854c <mech_gpu::sequence::runtime::resident::scalar_row+0x1264>
1000f768c:     	add	x17, x13, #0x20
1000f7690:     	ldp	w2, w0, [x13]
1000f7694:     	add	x13, x16, x8, lsl #8
1000f7698:     	add	x14, x16, x9, lsl #8
1000f769c:     	add	x3, x16, x10, lsl #8
1000f76a0:     	adr	x27, 0x1000f76b0 <mech_gpu::sequence::runtime::resident::scalar_row+0x3c8>
1000f76a4:     	ldrh	w23, [x26, x2, lsl #1]
1000f76a8:     	add	x27, x27, x23, lsl #2
1000f76ac:     	br	x27
1000f76b0:     	cmp	x5, #0x8
1000f76b4:     	b.lo	0x1000f7b40 <mech_gpu::sequence::runtime::resident::scalar_row+0x858>
1000f76b8:     	tst	x24, #0xffffffffffffff
1000f76bc:     	b.eq	0x1000f7b40 <mech_gpu::sequence::runtime::resident::scalar_row+0x858>
1000f76c0:     	ldp	q1, q2, [x14]
1000f76c4:     	ldp	q3, q4, [x14, #0x20]
1000f76c8:     	ucvtf.2d	v1, v1
1000f76cc:     	ucvtf.2d	v2, v2
1000f76d0:     	ucvtf.2d	v3, v3
1000f76d4:     	ucvtf.2d	v4, v4
1000f76d8:     	stp	q1, q2, [x13]
1000f76dc:     	stp	q3, q4, [x13, #0x20]
1000f76e0:     	cmp	x19, #0x8
1000f76e4:     	b.eq	0x1000f7758 <mech_gpu::sequence::runtime::resident::scalar_row+0x470>
1000f76e8:     	ldp	q1, q2, [x14, #0x40]
1000f76ec:     	ldp	q3, q4, [x14, #0x60]
1000f76f0:     	ucvtf.2d	v1, v1
1000f76f4:     	ucvtf.2d	v2, v2
1000f76f8:     	ucvtf.2d	v3, v3
1000f76fc:     	ucvtf.2d	v4, v4
1000f7700:     	stp	q1, q2, [x13, #0x40]
1000f7704:     	stp	q3, q4, [x13, #0x60]
1000f7708:     	cmp	x19, #0x10
1000f770c:     	b.eq	0x1000f7758 <mech_gpu::sequence::runtime::resident::scalar_row+0x470>
1000f7710:     	ldp	q1, q2, [x14, #0x80]
1000f7714:     	ldp	q3, q4, [x14, #0xa0]
1000f7718:     	ucvtf.2d	v1, v1
1000f771c:     	ucvtf.2d	v2, v2
1000f7720:     	ucvtf.2d	v3, v3
1000f7724:     	ucvtf.2d	v4, v4
1000f7728:     	stp	q1, q2, [x13, #0x80]
1000f772c:     	stp	q3, q4, [x13, #0xa0]
1000f7730:     	cmp	x19, #0x18
1000f7734:     	b.eq	0x1000f7758 <mech_gpu::sequence::runtime::resident::scalar_row+0x470>
1000f7738:     	ldp	q1, q2, [x14, #0xc0]
1000f773c:     	ldp	q3, q4, [x14, #0xe0]
1000f7740:     	ucvtf.2d	v1, v1
1000f7744:     	ucvtf.2d	v2, v2
1000f7748:     	ucvtf.2d	v3, v3
1000f774c:     	ucvtf.2d	v4, v4
1000f7750:     	stp	q1, q2, [x13, #0xc0]
1000f7754:     	stp	q3, q4, [x13, #0xe0]
1000f7758:     	and	x10, x30, #0x38
1000f775c:     	cmp	x30, x19
1000f7760:     	b.eq	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7764:     	b	0x1000f7b44 <mech_gpu::sequence::runtime::resident::scalar_row+0x85c>
1000f7768:     	cmp	x5, #0x8
1000f776c:     	b.hs	0x1000f7ba8 <mech_gpu::sequence::runtime::resident::scalar_row+0x8c0>
1000f7770:     	mov	x0, #0x0                ; =0
1000f7774:     	lsl	x13, x0, #3
1000f7778:     	add	x10, x13, x10, lsl #8
1000f777c:     	add	x10, x16, x10
1000f7780:     	add	x9, x13, x9, lsl #8
1000f7784:     	add	x9, x16, x9
1000f7788:     	add	x8, x13, x8, lsl #8
1000f778c:     	add	x8, x16, x8
1000f7790:     	sub	x13, x30, x0
1000f7794:     	ldr	x14, [x9], #0x8
1000f7798:     	ldr	x0, [x10], #0x8
1000f779c:     	add	x14, x0, x14
1000f77a0:     	str	x14, [x8], #0x8
1000f77a4:     	subs	x13, x13, #0x1
1000f77a8:     	b.ne	0x1000f7794 <mech_gpu::sequence::runtime::resident::scalar_row+0x4ac>
1000f77ac:     	b	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f77b0:     	cmp	x5, #0x8
1000f77b4:     	b.hs	0x1000f7c84 <mech_gpu::sequence::runtime::resident::scalar_row+0x99c>
1000f77b8:     	mov	x0, #0x0                ; =0
1000f77bc:     	lsl	x13, x0, #3
1000f77c0:     	add	x10, x13, x10, lsl #8
1000f77c4:     	add	x10, x16, x10
1000f77c8:     	add	x9, x13, x9, lsl #8
1000f77cc:     	add	x9, x16, x9
1000f77d0:     	add	x8, x13, x8, lsl #8
1000f77d4:     	add	x8, x16, x8
1000f77d8:     	sub	x13, x30, x0
1000f77dc:     	ldr	d1, [x9], #0x8
1000f77e0:     	ldr	d2, [x10], #0x8
1000f77e4:     	fsub	d1, d1, d2
1000f77e8:     	str	d1, [x8], #0x8
1000f77ec:     	subs	x13, x13, #0x1
1000f77f0:     	b.ne	0x1000f77dc <mech_gpu::sequence::runtime::resident::scalar_row+0x4f4>
1000f77f4:     	b	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f77f8:     	mov	x8, x30
1000f77fc:     	ldr	x9, [x3], #0x8
1000f7800:     	cbz	x9, 0x1000f8530 <mech_gpu::sequence::runtime::resident::scalar_row+0x1248>
1000f7804:     	ldr	x10, [x14], #0x8
1000f7808:     	udiv	x9, x10, x9
1000f780c:     	str	x9, [x13], #0x8
1000f7810:     	subs	x8, x8, #0x1
1000f7814:     	b.ne	0x1000f77fc <mech_gpu::sequence::runtime::resident::scalar_row+0x514>
1000f7818:     	b	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f781c:     	and	x10, x0, #0x3f
1000f7820:     	cmp	x5, #0x8
1000f7824:     	b.lo	0x1000f7b0c <mech_gpu::sequence::runtime::resident::scalar_row+0x824>
1000f7828:     	tst	x24, #0xffffffffffffff
1000f782c:     	b.eq	0x1000f7b0c <mech_gpu::sequence::runtime::resident::scalar_row+0x824>
1000f7830:     	dup.2d	v1, x10
1000f7834:     	ldp	q2, q3, [x14]
1000f7838:     	ldp	q4, q5, [x14, #0x20]
1000f783c:     	neg.2d	v1, v1
1000f7840:     	ushl.2d	v2, v2, v1
1000f7844:     	ushl.2d	v3, v3, v1
1000f7848:     	ushl.2d	v4, v4, v1
1000f784c:     	ushl.2d	v5, v5, v1
1000f7850:     	stp	q2, q3, [x13]
1000f7854:     	stp	q4, q5, [x13, #0x20]
1000f7858:     	cmp	x19, #0x8
1000f785c:     	b.eq	0x1000f78d0 <mech_gpu::sequence::runtime::resident::scalar_row+0x5e8>
1000f7860:     	ldp	q2, q3, [x14, #0x40]
1000f7864:     	ldp	q4, q5, [x14, #0x60]
1000f7868:     	ushl.2d	v2, v2, v1
1000f786c:     	ushl.2d	v3, v3, v1
1000f7870:     	ushl.2d	v4, v4, v1
1000f7874:     	ushl.2d	v5, v5, v1
1000f7878:     	stp	q2, q3, [x13, #0x40]
1000f787c:     	stp	q4, q5, [x13, #0x60]
1000f7880:     	cmp	x19, #0x10
1000f7884:     	b.eq	0x1000f78d0 <mech_gpu::sequence::runtime::resident::scalar_row+0x5e8>
1000f7888:     	ldp	q2, q3, [x14, #0x80]
1000f788c:     	ldp	q4, q5, [x14, #0xa0]
1000f7890:     	ushl.2d	v2, v2, v1
1000f7894:     	ushl.2d	v3, v3, v1
1000f7898:     	ushl.2d	v4, v4, v1
1000f789c:     	ushl.2d	v5, v5, v1
1000f78a0:     	stp	q2, q3, [x13, #0x80]
1000f78a4:     	stp	q4, q5, [x13, #0xa0]
1000f78a8:     	cmp	x19, #0x18
1000f78ac:     	b.eq	0x1000f78d0 <mech_gpu::sequence::runtime::resident::scalar_row+0x5e8>
1000f78b0:     	ldp	q2, q3, [x14, #0xc0]
1000f78b4:     	ldp	q4, q5, [x14, #0xe0]
1000f78b8:     	ushl.2d	v2, v2, v1
1000f78bc:     	ushl.2d	v3, v3, v1
1000f78c0:     	ushl.2d	v4, v4, v1
1000f78c4:     	ushl.2d	v1, v5, v1
1000f78c8:     	stp	q2, q3, [x13, #0xc0]
1000f78cc:     	stp	q4, q1, [x13, #0xe0]
1000f78d0:     	and	x13, x30, #0x38
1000f78d4:     	cmp	x30, x19
1000f78d8:     	b.eq	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f78dc:     	b	0x1000f7b10 <mech_gpu::sequence::runtime::resident::scalar_row+0x828>
1000f78e0:     	cmp	x5, #0x8
1000f78e4:     	b.hs	0x1000f7d60 <mech_gpu::sequence::runtime::resident::scalar_row+0xa78>
1000f78e8:     	mov	x0, #0x0                ; =0
1000f78ec:     	lsl	x13, x0, #3
1000f78f0:     	add	x10, x13, x10, lsl #8
1000f78f4:     	add	x10, x16, x10
1000f78f8:     	add	x9, x13, x9, lsl #8
1000f78fc:     	add	x9, x16, x9
1000f7900:     	add	x8, x13, x8, lsl #8
1000f7904:     	add	x8, x16, x8
1000f7908:     	sub	x13, x30, x0
1000f790c:     	ldr	d1, [x9], #0x8
1000f7910:     	ldr	d2, [x10], #0x8
1000f7914:     	fdiv	d1, d1, d2
1000f7918:     	str	d1, [x8], #0x8
1000f791c:     	subs	x13, x13, #0x1
1000f7920:     	b.ne	0x1000f790c <mech_gpu::sequence::runtime::resident::scalar_row+0x624>
1000f7924:     	b	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7928:     	cmp	x5, #0x1
1000f792c:     	b.eq	0x1000f7640 <mech_gpu::sequence::runtime::resident::scalar_row+0x358>
1000f7930:     	mov	x7, #0x0                ; =0
1000f7934:     	add	x0, x16, #0x8
1000f7938:     	add	x8, x0, x8, lsl #8
1000f793c:     	add	x9, x0, x9, lsl #8
1000f7940:     	add	x10, x0, x10, lsl #8
1000f7944:     	ldur	x0, [x9, #-0x8]
1000f7948:     	ldur	x2, [x10, #-0x8]
1000f794c:     	mul	x0, x2, x0
1000f7950:     	stur	x0, [x8, #-0x8]
1000f7954:     	ldr	x0, [x9], #0x10
1000f7958:     	ldr	x2, [x10], #0x10
1000f795c:     	mul	x0, x2, x0
1000f7960:     	sub	x7, x7, #0x2
1000f7964:     	str	x0, [x8], #0x10
1000f7968:     	cmp	x25, x7
1000f796c:     	b.ne	0x1000f7944 <mech_gpu::sequence::runtime::resident::scalar_row+0x65c>
1000f7970:     	tbz	w30, #0x0, 0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7974:     	neg	x8, x7
1000f7978:     	b	0x1000f7644 <mech_gpu::sequence::runtime::resident::scalar_row+0x35c>
1000f797c:     	cmp	x5, #0x8
1000f7980:     	b.hs	0x1000f7e3c <mech_gpu::sequence::runtime::resident::scalar_row+0xb54>
1000f7984:     	mov	x0, #0x0                ; =0
1000f7988:     	lsl	x13, x0, #3
1000f798c:     	add	x10, x13, x10, lsl #8
1000f7990:     	add	x10, x16, x10
1000f7994:     	add	x9, x13, x9, lsl #8
1000f7998:     	add	x9, x16, x9
1000f799c:     	add	x8, x13, x8, lsl #8
1000f79a0:     	add	x8, x16, x8
1000f79a4:     	sub	x13, x30, x0
1000f79a8:     	ldr	x14, [x9], #0x8
1000f79ac:     	ldr	x0, [x10], #0x8
1000f79b0:     	sub	x14, x14, x0
1000f79b4:     	str	x14, [x8], #0x8
1000f79b8:     	subs	x13, x13, #0x1
1000f79bc:     	b.ne	0x1000f79a8 <mech_gpu::sequence::runtime::resident::scalar_row+0x6c0>
1000f79c0:     	b	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f79c4:     	cmp	x5, #0x8
1000f79c8:     	b.hs	0x1000f7f18 <mech_gpu::sequence::runtime::resident::scalar_row+0xc30>
1000f79cc:     	mov	x0, #0x0                ; =0
1000f79d0:     	lsl	x13, x0, #3
1000f79d4:     	add	x10, x13, x10, lsl #8
1000f79d8:     	add	x10, x16, x10
1000f79dc:     	add	x9, x13, x9, lsl #8
1000f79e0:     	add	x9, x16, x9
1000f79e4:     	add	x8, x13, x8, lsl #8
1000f79e8:     	add	x8, x16, x8
1000f79ec:     	sub	x13, x30, x0
1000f79f0:     	ldr	d1, [x9], #0x8
1000f79f4:     	ldr	d2, [x10], #0x8
1000f79f8:     	fmul	d1, d1, d2
1000f79fc:     	str	d1, [x8], #0x8
1000f7a00:     	subs	x13, x13, #0x1
1000f7a04:     	b.ne	0x1000f79f0 <mech_gpu::sequence::runtime::resident::scalar_row+0x708>
1000f7a08:     	b	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7a0c:     	cmp	x5, #0x8
1000f7a10:     	b.lo	0x1000f7b74 <mech_gpu::sequence::runtime::resident::scalar_row+0x88c>
1000f7a14:     	tst	x24, #0xffffffffffffff
1000f7a18:     	b.eq	0x1000f7b74 <mech_gpu::sequence::runtime::resident::scalar_row+0x88c>
1000f7a1c:     	ldp	q1, q2, [x14]
1000f7a20:     	ldp	q3, q4, [x14, #0x20]
1000f7a24:     	fsqrt.2d	v1, v1
1000f7a28:     	fsqrt.2d	v2, v2
1000f7a2c:     	fsqrt.2d	v3, v3
1000f7a30:     	fsqrt.2d	v4, v4
1000f7a34:     	stp	q1, q2, [x13]
1000f7a38:     	stp	q3, q4, [x13, #0x20]
1000f7a3c:     	cmp	x19, #0x8
1000f7a40:     	b.eq	0x1000f7ab4 <mech_gpu::sequence::runtime::resident::scalar_row+0x7cc>
1000f7a44:     	ldp	q1, q2, [x14, #0x40]
1000f7a48:     	ldp	q3, q4, [x14, #0x60]
1000f7a4c:     	fsqrt.2d	v1, v1
1000f7a50:     	fsqrt.2d	v2, v2
1000f7a54:     	fsqrt.2d	v3, v3
1000f7a58:     	fsqrt.2d	v4, v4
1000f7a5c:     	stp	q1, q2, [x13, #0x40]
1000f7a60:     	stp	q3, q4, [x13, #0x60]
1000f7a64:     	cmp	x19, #0x10
1000f7a68:     	b.eq	0x1000f7ab4 <mech_gpu::sequence::runtime::resident::scalar_row+0x7cc>
1000f7a6c:     	ldp	q1, q2, [x14, #0x80]
1000f7a70:     	ldp	q3, q4, [x14, #0xa0]
1000f7a74:     	fsqrt.2d	v1, v1
1000f7a78:     	fsqrt.2d	v2, v2
1000f7a7c:     	fsqrt.2d	v3, v3
1000f7a80:     	fsqrt.2d	v4, v4
1000f7a84:     	stp	q1, q2, [x13, #0x80]
1000f7a88:     	stp	q3, q4, [x13, #0xa0]
1000f7a8c:     	cmp	x19, #0x18
1000f7a90:     	b.eq	0x1000f7ab4 <mech_gpu::sequence::runtime::resident::scalar_row+0x7cc>
1000f7a94:     	ldp	q1, q2, [x14, #0xc0]
1000f7a98:     	ldp	q3, q4, [x14, #0xe0]
1000f7a9c:     	fsqrt.2d	v1, v1
1000f7aa0:     	fsqrt.2d	v2, v2
1000f7aa4:     	fsqrt.2d	v3, v3
1000f7aa8:     	fsqrt.2d	v4, v4
1000f7aac:     	stp	q1, q2, [x13, #0xc0]
1000f7ab0:     	stp	q3, q4, [x13, #0xe0]
1000f7ab4:     	and	x10, x30, #0x38
1000f7ab8:     	cmp	x30, x19
1000f7abc:     	b.eq	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7ac0:     	b	0x1000f7b78 <mech_gpu::sequence::runtime::resident::scalar_row+0x890>
1000f7ac4:     	cmp	x5, #0x8
1000f7ac8:     	b.hs	0x1000f7ff4 <mech_gpu::sequence::runtime::resident::scalar_row+0xd0c>
1000f7acc:     	mov	x0, #0x0                ; =0
1000f7ad0:     	lsl	x13, x0, #3
1000f7ad4:     	add	x10, x13, x10, lsl #8
1000f7ad8:     	add	x10, x16, x10
1000f7adc:     	add	x9, x13, x9, lsl #8
1000f7ae0:     	add	x9, x16, x9
1000f7ae4:     	add	x8, x13, x8, lsl #8
1000f7ae8:     	add	x8, x16, x8
1000f7aec:     	sub	x13, x30, x0
1000f7af0:     	ldr	d1, [x9], #0x8
1000f7af4:     	ldr	d2, [x10], #0x8
1000f7af8:     	fadd	d1, d1, d2
1000f7afc:     	str	d1, [x8], #0x8
1000f7b00:     	subs	x13, x13, #0x1
1000f7b04:     	b.ne	0x1000f7af0 <mech_gpu::sequence::runtime::resident::scalar_row+0x808>
1000f7b08:     	b	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7b0c:     	mov	x13, #0x0               ; =0
1000f7b10:     	lsl	x14, x13, #3
1000f7b14:     	add	x9, x14, x9, lsl #8
1000f7b18:     	add	x9, x16, x9
1000f7b1c:     	add	x8, x14, x8, lsl #8
1000f7b20:     	add	x8, x16, x8
1000f7b24:     	sub	x13, x30, x13
1000f7b28:     	ldr	x14, [x9], #0x8
1000f7b2c:     	lsr	x14, x14, x10
1000f7b30:     	str	x14, [x8], #0x8
1000f7b34:     	subs	x13, x13, #0x1
1000f7b38:     	b.ne	0x1000f7b28 <mech_gpu::sequence::runtime::resident::scalar_row+0x840>
1000f7b3c:     	b	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7b40:     	mov	x10, #0x0               ; =0
1000f7b44:     	lsl	x13, x10, #3
1000f7b48:     	add	x9, x13, x9, lsl #8
1000f7b4c:     	add	x9, x16, x9
1000f7b50:     	add	x8, x13, x8, lsl #8
1000f7b54:     	add	x8, x16, x8
1000f7b58:     	sub	x10, x30, x10
1000f7b5c:     	ldr	d1, [x9], #0x8
1000f7b60:     	ucvtf	d1, d1
1000f7b64:     	str	d1, [x8], #0x8
1000f7b68:     	subs	x10, x10, #0x1
1000f7b6c:     	b.ne	0x1000f7b5c <mech_gpu::sequence::runtime::resident::scalar_row+0x874>
1000f7b70:     	b	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7b74:     	mov	x10, #0x0               ; =0
1000f7b78:     	lsl	x13, x10, #3
1000f7b7c:     	add	x9, x13, x9, lsl #8
1000f7b80:     	add	x9, x16, x9
1000f7b84:     	add	x8, x13, x8, lsl #8
1000f7b88:     	add	x8, x16, x8
1000f7b8c:     	sub	x10, x30, x10
1000f7b90:     	ldr	d1, [x9], #0x8
1000f7b94:     	fsqrt	d1, d1
1000f7b98:     	str	d1, [x8], #0x8
1000f7b9c:     	subs	x10, x10, #0x1
1000f7ba0:     	b.ne	0x1000f7b90 <mech_gpu::sequence::runtime::resident::scalar_row+0x8a8>
1000f7ba4:     	b	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7ba8:     	mov	x0, #0x0                ; =0
1000f7bac:     	and	x2, x24, #0xffffffffffffff
1000f7bb0:     	cbz	x2, 0x1000f7774 <mech_gpu::sequence::runtime::resident::scalar_row+0x48c>
1000f7bb4:     	and	x2, x7, #0xffffffffffffff
1000f7bb8:     	cbz	x2, 0x1000f7774 <mech_gpu::sequence::runtime::resident::scalar_row+0x48c>
1000f7bbc:     	ldp	q1, q2, [x14]
1000f7bc0:     	ldp	q3, q4, [x14, #0x20]
1000f7bc4:     	ldp	q5, q6, [x3]
1000f7bc8:     	ldp	q7, q16, [x3, #0x20]
1000f7bcc:     	add.2d	v1, v5, v1
1000f7bd0:     	add.2d	v2, v6, v2
1000f7bd4:     	add.2d	v3, v7, v3
1000f7bd8:     	add.2d	v4, v16, v4
1000f7bdc:     	stp	q1, q2, [x13]
1000f7be0:     	stp	q3, q4, [x13, #0x20]
1000f7be4:     	cmp	x19, #0x8
1000f7be8:     	b.eq	0x1000f7c74 <mech_gpu::sequence::runtime::resident::scalar_row+0x98c>
1000f7bec:     	ldp	q1, q2, [x14, #0x40]
1000f7bf0:     	ldp	q3, q4, [x14, #0x60]
1000f7bf4:     	ldp	q5, q6, [x3, #0x40]
1000f7bf8:     	ldp	q7, q16, [x3, #0x60]
1000f7bfc:     	add.2d	v1, v5, v1
1000f7c00:     	add.2d	v2, v6, v2
1000f7c04:     	add.2d	v3, v7, v3
1000f7c08:     	add.2d	v4, v16, v4
1000f7c0c:     	stp	q1, q2, [x13, #0x40]
1000f7c10:     	stp	q3, q4, [x13, #0x60]
1000f7c14:     	cmp	x19, #0x10
1000f7c18:     	b.eq	0x1000f7c74 <mech_gpu::sequence::runtime::resident::scalar_row+0x98c>
1000f7c1c:     	ldp	q1, q2, [x14, #0x80]
1000f7c20:     	ldp	q3, q4, [x14, #0xa0]
1000f7c24:     	ldp	q5, q6, [x3, #0x80]
1000f7c28:     	ldp	q7, q16, [x3, #0xa0]
1000f7c2c:     	add.2d	v1, v5, v1
1000f7c30:     	add.2d	v2, v6, v2
1000f7c34:     	add.2d	v3, v7, v3
1000f7c38:     	add.2d	v4, v16, v4
1000f7c3c:     	stp	q1, q2, [x13, #0x80]
1000f7c40:     	stp	q3, q4, [x13, #0xa0]
1000f7c44:     	cmp	x19, #0x18
1000f7c48:     	b.eq	0x1000f7c74 <mech_gpu::sequence::runtime::resident::scalar_row+0x98c>
1000f7c4c:     	ldp	q1, q2, [x14, #0xc0]
1000f7c50:     	ldp	q3, q4, [x14, #0xe0]
1000f7c54:     	ldp	q5, q6, [x3, #0xc0]
1000f7c58:     	ldp	q7, q16, [x3, #0xe0]
1000f7c5c:     	add.2d	v1, v5, v1
1000f7c60:     	add.2d	v2, v6, v2
1000f7c64:     	add.2d	v3, v7, v3
1000f7c68:     	add.2d	v4, v16, v4
1000f7c6c:     	stp	q1, q2, [x13, #0xc0]
1000f7c70:     	stp	q3, q4, [x13, #0xe0]
1000f7c74:     	and	x0, x30, #0x38
1000f7c78:     	cmp	x30, x19
1000f7c7c:     	b.eq	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7c80:     	b	0x1000f7774 <mech_gpu::sequence::runtime::resident::scalar_row+0x48c>
1000f7c84:     	mov	x0, #0x0                ; =0
1000f7c88:     	and	x2, x24, #0xffffffffffffff
1000f7c8c:     	cbz	x2, 0x1000f77bc <mech_gpu::sequence::runtime::resident::scalar_row+0x4d4>
1000f7c90:     	and	x2, x7, #0xffffffffffffff
1000f7c94:     	cbz	x2, 0x1000f77bc <mech_gpu::sequence::runtime::resident::scalar_row+0x4d4>
1000f7c98:     	ldp	q1, q2, [x14]
1000f7c9c:     	ldp	q3, q4, [x14, #0x20]
1000f7ca0:     	ldp	q5, q6, [x3]
1000f7ca4:     	ldp	q7, q16, [x3, #0x20]
1000f7ca8:     	fsub.2d	v1, v1, v5
1000f7cac:     	fsub.2d	v2, v2, v6
1000f7cb0:     	fsub.2d	v3, v3, v7
1000f7cb4:     	fsub.2d	v4, v4, v16
1000f7cb8:     	stp	q1, q2, [x13]
1000f7cbc:     	stp	q3, q4, [x13, #0x20]
1000f7cc0:     	cmp	x19, #0x8
1000f7cc4:     	b.eq	0x1000f7d50 <mech_gpu::sequence::runtime::resident::scalar_row+0xa68>
1000f7cc8:     	ldp	q1, q2, [x14, #0x40]
1000f7ccc:     	ldp	q3, q4, [x14, #0x60]
1000f7cd0:     	ldp	q5, q6, [x3, #0x40]
1000f7cd4:     	ldp	q7, q16, [x3, #0x60]
1000f7cd8:     	fsub.2d	v1, v1, v5
1000f7cdc:     	fsub.2d	v2, v2, v6
1000f7ce0:     	fsub.2d	v3, v3, v7
1000f7ce4:     	fsub.2d	v4, v4, v16
1000f7ce8:     	stp	q1, q2, [x13, #0x40]
1000f7cec:     	stp	q3, q4, [x13, #0x60]
1000f7cf0:     	cmp	x19, #0x10
1000f7cf4:     	b.eq	0x1000f7d50 <mech_gpu::sequence::runtime::resident::scalar_row+0xa68>
1000f7cf8:     	ldp	q1, q2, [x14, #0x80]
1000f7cfc:     	ldp	q3, q4, [x14, #0xa0]
1000f7d00:     	ldp	q5, q6, [x3, #0x80]
1000f7d04:     	ldp	q7, q16, [x3, #0xa0]
1000f7d08:     	fsub.2d	v1, v1, v5
1000f7d0c:     	fsub.2d	v2, v2, v6
1000f7d10:     	fsub.2d	v3, v3, v7
1000f7d14:     	fsub.2d	v4, v4, v16
1000f7d18:     	stp	q1, q2, [x13, #0x80]
1000f7d1c:     	stp	q3, q4, [x13, #0xa0]
1000f7d20:     	cmp	x19, #0x18
1000f7d24:     	b.eq	0x1000f7d50 <mech_gpu::sequence::runtime::resident::scalar_row+0xa68>
1000f7d28:     	ldp	q1, q2, [x14, #0xc0]
1000f7d2c:     	ldp	q3, q4, [x14, #0xe0]
1000f7d30:     	ldp	q5, q6, [x3, #0xc0]
1000f7d34:     	ldp	q7, q16, [x3, #0xe0]
1000f7d38:     	fsub.2d	v1, v1, v5
1000f7d3c:     	fsub.2d	v2, v2, v6
1000f7d40:     	fsub.2d	v3, v3, v7
1000f7d44:     	fsub.2d	v4, v4, v16
1000f7d48:     	stp	q1, q2, [x13, #0xc0]
1000f7d4c:     	stp	q3, q4, [x13, #0xe0]
1000f7d50:     	and	x0, x30, #0x38
1000f7d54:     	cmp	x30, x19
1000f7d58:     	b.eq	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7d5c:     	b	0x1000f77bc <mech_gpu::sequence::runtime::resident::scalar_row+0x4d4>
1000f7d60:     	mov	x0, #0x0                ; =0
1000f7d64:     	and	x2, x24, #0xffffffffffffff
1000f7d68:     	cbz	x2, 0x1000f78ec <mech_gpu::sequence::runtime::resident::scalar_row+0x604>
1000f7d6c:     	and	x2, x7, #0xffffffffffffff
1000f7d70:     	cbz	x2, 0x1000f78ec <mech_gpu::sequence::runtime::resident::scalar_row+0x604>
1000f7d74:     	ldp	q1, q2, [x14]
1000f7d78:     	ldp	q3, q4, [x14, #0x20]
1000f7d7c:     	ldp	q5, q6, [x3]
1000f7d80:     	ldp	q7, q16, [x3, #0x20]
1000f7d84:     	fdiv.2d	v1, v1, v5
1000f7d88:     	fdiv.2d	v2, v2, v6
1000f7d8c:     	fdiv.2d	v3, v3, v7
1000f7d90:     	fdiv.2d	v4, v4, v16
1000f7d94:     	stp	q1, q2, [x13]
1000f7d98:     	stp	q3, q4, [x13, #0x20]
1000f7d9c:     	cmp	x19, #0x8
1000f7da0:     	b.eq	0x1000f7e2c <mech_gpu::sequence::runtime::resident::scalar_row+0xb44>
1000f7da4:     	ldp	q1, q2, [x14, #0x40]
1000f7da8:     	ldp	q3, q4, [x14, #0x60]
1000f7dac:     	ldp	q5, q6, [x3, #0x40]
1000f7db0:     	ldp	q7, q16, [x3, #0x60]
1000f7db4:     	fdiv.2d	v1, v1, v5
1000f7db8:     	fdiv.2d	v2, v2, v6
1000f7dbc:     	fdiv.2d	v3, v3, v7
1000f7dc0:     	fdiv.2d	v4, v4, v16
1000f7dc4:     	stp	q1, q2, [x13, #0x40]
1000f7dc8:     	stp	q3, q4, [x13, #0x60]
1000f7dcc:     	cmp	x19, #0x10
1000f7dd0:     	b.eq	0x1000f7e2c <mech_gpu::sequence::runtime::resident::scalar_row+0xb44>
1000f7dd4:     	ldp	q1, q2, [x14, #0x80]
1000f7dd8:     	ldp	q3, q4, [x14, #0xa0]
1000f7ddc:     	ldp	q5, q6, [x3, #0x80]
1000f7de0:     	ldp	q7, q16, [x3, #0xa0]
1000f7de4:     	fdiv.2d	v1, v1, v5
1000f7de8:     	fdiv.2d	v2, v2, v6
1000f7dec:     	fdiv.2d	v3, v3, v7
1000f7df0:     	fdiv.2d	v4, v4, v16
1000f7df4:     	stp	q1, q2, [x13, #0x80]
1000f7df8:     	stp	q3, q4, [x13, #0xa0]
1000f7dfc:     	cmp	x19, #0x18
1000f7e00:     	b.eq	0x1000f7e2c <mech_gpu::sequence::runtime::resident::scalar_row+0xb44>
1000f7e04:     	ldp	q1, q2, [x14, #0xc0]
1000f7e08:     	ldp	q3, q4, [x14, #0xe0]
1000f7e0c:     	ldp	q5, q6, [x3, #0xc0]
1000f7e10:     	ldp	q7, q16, [x3, #0xe0]
1000f7e14:     	fdiv.2d	v1, v1, v5
1000f7e18:     	fdiv.2d	v2, v2, v6
1000f7e1c:     	fdiv.2d	v3, v3, v7
1000f7e20:     	fdiv.2d	v4, v4, v16
1000f7e24:     	stp	q1, q2, [x13, #0xc0]
1000f7e28:     	stp	q3, q4, [x13, #0xe0]
1000f7e2c:     	and	x0, x30, #0x38
1000f7e30:     	cmp	x30, x19
1000f7e34:     	b.eq	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7e38:     	b	0x1000f78ec <mech_gpu::sequence::runtime::resident::scalar_row+0x604>
1000f7e3c:     	mov	x0, #0x0                ; =0
1000f7e40:     	and	x2, x24, #0xffffffffffffff
1000f7e44:     	cbz	x2, 0x1000f7988 <mech_gpu::sequence::runtime::resident::scalar_row+0x6a0>
1000f7e48:     	and	x2, x7, #0xffffffffffffff
1000f7e4c:     	cbz	x2, 0x1000f7988 <mech_gpu::sequence::runtime::resident::scalar_row+0x6a0>
1000f7e50:     	ldp	q1, q2, [x14]
1000f7e54:     	ldp	q3, q4, [x14, #0x20]
1000f7e58:     	ldp	q5, q6, [x3]
1000f7e5c:     	ldp	q7, q16, [x3, #0x20]
1000f7e60:     	sub.2d	v1, v1, v5
1000f7e64:     	sub.2d	v2, v2, v6
1000f7e68:     	sub.2d	v3, v3, v7
1000f7e6c:     	sub.2d	v4, v4, v16
1000f7e70:     	stp	q1, q2, [x13]
1000f7e74:     	stp	q3, q4, [x13, #0x20]
1000f7e78:     	cmp	x19, #0x8
1000f7e7c:     	b.eq	0x1000f7f08 <mech_gpu::sequence::runtime::resident::scalar_row+0xc20>
1000f7e80:     	ldp	q1, q2, [x14, #0x40]
1000f7e84:     	ldp	q3, q4, [x14, #0x60]
1000f7e88:     	ldp	q5, q6, [x3, #0x40]
1000f7e8c:     	ldp	q7, q16, [x3, #0x60]
1000f7e90:     	sub.2d	v1, v1, v5
1000f7e94:     	sub.2d	v2, v2, v6
1000f7e98:     	sub.2d	v3, v3, v7
1000f7e9c:     	sub.2d	v4, v4, v16
1000f7ea0:     	stp	q1, q2, [x13, #0x40]
1000f7ea4:     	stp	q3, q4, [x13, #0x60]
1000f7ea8:     	cmp	x19, #0x10
1000f7eac:     	b.eq	0x1000f7f08 <mech_gpu::sequence::runtime::resident::scalar_row+0xc20>
1000f7eb0:     	ldp	q1, q2, [x14, #0x80]
1000f7eb4:     	ldp	q3, q4, [x14, #0xa0]
1000f7eb8:     	ldp	q5, q6, [x3, #0x80]
1000f7ebc:     	ldp	q7, q16, [x3, #0xa0]
1000f7ec0:     	sub.2d	v1, v1, v5
1000f7ec4:     	sub.2d	v2, v2, v6
1000f7ec8:     	sub.2d	v3, v3, v7
1000f7ecc:     	sub.2d	v4, v4, v16
1000f7ed0:     	stp	q1, q2, [x13, #0x80]
1000f7ed4:     	stp	q3, q4, [x13, #0xa0]
1000f7ed8:     	cmp	x19, #0x18
1000f7edc:     	b.eq	0x1000f7f08 <mech_gpu::sequence::runtime::resident::scalar_row+0xc20>
1000f7ee0:     	ldp	q1, q2, [x14, #0xc0]
1000f7ee4:     	ldp	q3, q4, [x14, #0xe0]
1000f7ee8:     	ldp	q5, q6, [x3, #0xc0]
1000f7eec:     	ldp	q7, q16, [x3, #0xe0]
1000f7ef0:     	sub.2d	v1, v1, v5
1000f7ef4:     	sub.2d	v2, v2, v6
1000f7ef8:     	sub.2d	v3, v3, v7
1000f7efc:     	sub.2d	v4, v4, v16
1000f7f00:     	stp	q1, q2, [x13, #0xc0]
1000f7f04:     	stp	q3, q4, [x13, #0xe0]
1000f7f08:     	and	x0, x30, #0x38
1000f7f0c:     	cmp	x30, x19
1000f7f10:     	b.eq	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7f14:     	b	0x1000f7988 <mech_gpu::sequence::runtime::resident::scalar_row+0x6a0>
1000f7f18:     	mov	x0, #0x0                ; =0
1000f7f1c:     	and	x2, x24, #0xffffffffffffff
1000f7f20:     	cbz	x2, 0x1000f79d0 <mech_gpu::sequence::runtime::resident::scalar_row+0x6e8>
1000f7f24:     	and	x2, x7, #0xffffffffffffff
1000f7f28:     	cbz	x2, 0x1000f79d0 <mech_gpu::sequence::runtime::resident::scalar_row+0x6e8>
1000f7f2c:     	ldp	q1, q2, [x14]
1000f7f30:     	ldp	q3, q4, [x14, #0x20]
1000f7f34:     	ldp	q5, q6, [x3]
1000f7f38:     	ldp	q7, q16, [x3, #0x20]
1000f7f3c:     	fmul.2d	v1, v1, v5
1000f7f40:     	fmul.2d	v2, v2, v6
1000f7f44:     	fmul.2d	v3, v3, v7
1000f7f48:     	fmul.2d	v4, v4, v16
1000f7f4c:     	stp	q1, q2, [x13]
1000f7f50:     	stp	q3, q4, [x13, #0x20]
1000f7f54:     	cmp	x19, #0x8
1000f7f58:     	b.eq	0x1000f7fe4 <mech_gpu::sequence::runtime::resident::scalar_row+0xcfc>
1000f7f5c:     	ldp	q1, q2, [x14, #0x40]
1000f7f60:     	ldp	q3, q4, [x14, #0x60]
1000f7f64:     	ldp	q5, q6, [x3, #0x40]
1000f7f68:     	ldp	q7, q16, [x3, #0x60]
1000f7f6c:     	fmul.2d	v1, v1, v5
1000f7f70:     	fmul.2d	v2, v2, v6
1000f7f74:     	fmul.2d	v3, v3, v7
1000f7f78:     	fmul.2d	v4, v4, v16
1000f7f7c:     	stp	q1, q2, [x13, #0x40]
1000f7f80:     	stp	q3, q4, [x13, #0x60]
1000f7f84:     	cmp	x19, #0x10
1000f7f88:     	b.eq	0x1000f7fe4 <mech_gpu::sequence::runtime::resident::scalar_row+0xcfc>
1000f7f8c:     	ldp	q1, q2, [x14, #0x80]
1000f7f90:     	ldp	q3, q4, [x14, #0xa0]
1000f7f94:     	ldp	q5, q6, [x3, #0x80]
1000f7f98:     	ldp	q7, q16, [x3, #0xa0]
1000f7f9c:     	fmul.2d	v1, v1, v5
1000f7fa0:     	fmul.2d	v2, v2, v6
1000f7fa4:     	fmul.2d	v3, v3, v7
1000f7fa8:     	fmul.2d	v4, v4, v16
1000f7fac:     	stp	q1, q2, [x13, #0x80]
1000f7fb0:     	stp	q3, q4, [x13, #0xa0]
1000f7fb4:     	cmp	x19, #0x18
1000f7fb8:     	b.eq	0x1000f7fe4 <mech_gpu::sequence::runtime::resident::scalar_row+0xcfc>
1000f7fbc:     	ldp	q1, q2, [x14, #0xc0]
1000f7fc0:     	ldp	q3, q4, [x14, #0xe0]
1000f7fc4:     	ldp	q5, q6, [x3, #0xc0]
1000f7fc8:     	ldp	q7, q16, [x3, #0xe0]
1000f7fcc:     	fmul.2d	v1, v1, v5
1000f7fd0:     	fmul.2d	v2, v2, v6
1000f7fd4:     	fmul.2d	v3, v3, v7
1000f7fd8:     	fmul.2d	v4, v4, v16
1000f7fdc:     	stp	q1, q2, [x13, #0xc0]
1000f7fe0:     	stp	q3, q4, [x13, #0xe0]
1000f7fe4:     	and	x0, x30, #0x38
1000f7fe8:     	cmp	x30, x19
1000f7fec:     	b.eq	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f7ff0:     	b	0x1000f79d0 <mech_gpu::sequence::runtime::resident::scalar_row+0x6e8>
1000f7ff4:     	mov	x0, #0x0                ; =0
1000f7ff8:     	and	x2, x24, #0xffffffffffffff
1000f7ffc:     	cbz	x2, 0x1000f7ad0 <mech_gpu::sequence::runtime::resident::scalar_row+0x7e8>
1000f8000:     	and	x2, x7, #0xffffffffffffff
1000f8004:     	cbz	x2, 0x1000f7ad0 <mech_gpu::sequence::runtime::resident::scalar_row+0x7e8>
1000f8008:     	ldp	q1, q2, [x14]
1000f800c:     	ldp	q3, q4, [x14, #0x20]
1000f8010:     	ldp	q5, q6, [x3]
1000f8014:     	ldp	q7, q16, [x3, #0x20]
1000f8018:     	fadd.2d	v1, v1, v5
1000f801c:     	fadd.2d	v2, v2, v6
1000f8020:     	fadd.2d	v3, v3, v7
1000f8024:     	fadd.2d	v4, v4, v16
1000f8028:     	stp	q1, q2, [x13]
1000f802c:     	stp	q3, q4, [x13, #0x20]
1000f8030:     	cmp	x19, #0x8
1000f8034:     	b.eq	0x1000f80c0 <mech_gpu::sequence::runtime::resident::scalar_row+0xdd8>
1000f8038:     	ldp	q1, q2, [x14, #0x40]
1000f803c:     	ldp	q3, q4, [x14, #0x60]
1000f8040:     	ldp	q5, q6, [x3, #0x40]
1000f8044:     	ldp	q7, q16, [x3, #0x60]
1000f8048:     	fadd.2d	v1, v1, v5
1000f804c:     	fadd.2d	v2, v2, v6
1000f8050:     	fadd.2d	v3, v3, v7
1000f8054:     	fadd.2d	v4, v4, v16
1000f8058:     	stp	q1, q2, [x13, #0x40]
1000f805c:     	stp	q3, q4, [x13, #0x60]
1000f8060:     	cmp	x19, #0x10
1000f8064:     	b.eq	0x1000f80c0 <mech_gpu::sequence::runtime::resident::scalar_row+0xdd8>
1000f8068:     	ldp	q1, q2, [x14, #0x80]
1000f806c:     	ldp	q3, q4, [x14, #0xa0]
1000f8070:     	ldp	q5, q6, [x3, #0x80]
1000f8074:     	ldp	q7, q16, [x3, #0xa0]
1000f8078:     	fadd.2d	v1, v1, v5
1000f807c:     	fadd.2d	v2, v2, v6
1000f8080:     	fadd.2d	v3, v3, v7
1000f8084:     	fadd.2d	v4, v4, v16
1000f8088:     	stp	q1, q2, [x13, #0x80]
1000f808c:     	stp	q3, q4, [x13, #0xa0]
1000f8090:     	cmp	x19, #0x18
1000f8094:     	b.eq	0x1000f80c0 <mech_gpu::sequence::runtime::resident::scalar_row+0xdd8>
1000f8098:     	ldp	q1, q2, [x14, #0xc0]
1000f809c:     	ldp	q3, q4, [x14, #0xe0]
1000f80a0:     	ldp	q5, q6, [x3, #0xc0]
1000f80a4:     	ldp	q7, q16, [x3, #0xe0]
1000f80a8:     	fadd.2d	v1, v1, v5
1000f80ac:     	fadd.2d	v2, v2, v6
1000f80b0:     	fadd.2d	v3, v3, v7
1000f80b4:     	fadd.2d	v4, v4, v16
1000f80b8:     	stp	q1, q2, [x13, #0xc0]
1000f80bc:     	stp	q3, q4, [x13, #0xe0]
1000f80c0:     	and	x0, x30, #0x38
1000f80c4:     	cmp	x30, x19
1000f80c8:     	b.eq	0x1000f7654 <mech_gpu::sequence::runtime::resident::scalar_row+0x36c>
1000f80cc:     	b	0x1000f7ad0 <mech_gpu::sequence::runtime::resident::scalar_row+0x7e8>
1000f80d0:     	ldr	x8, [sp, #0x28]
1000f80d4:     	cmp	x8, x1
1000f80d8:     	b.hs	0x1000f8588 <mech_gpu::sequence::runtime::resident::scalar_row+0x12a0>
1000f80dc:     	ldr	w8, [sp, #0x44]
1000f80e0:     	ldr	x13, [sp, #0x20]
1000f80e4:     	cmp	x5, #0x8
1000f80e8:     	tbz	w8, #0x0, 0x1000f80f8 <mech_gpu::sequence::runtime::resident::scalar_row+0xe10>
1000f80ec:     	b.hs	0x1000f8104 <mech_gpu::sequence::runtime::resident::scalar_row+0xe1c>
1000f80f0:     	mov	x8, #0x0                ; =0
1000f80f4:     	b	0x1000f8268 <mech_gpu::sequence::runtime::resident::scalar_row+0xf80>
1000f80f8:     	b.hs	0x1000f8294 <mech_gpu::sequence::runtime::resident::scalar_row+0xfac>
1000f80fc:     	mov	x8, #0x0                ; =0
1000f8100:     	b	0x1000f83f8 <mech_gpu::sequence::runtime::resident::scalar_row+0x1110>
1000f8104:     	and	x8, x30, #0x38
1000f8108:     	ldp	q1, q2, [x20]
1000f810c:     	ldp	q3, q4, [x20, #0x20]
1000f8110:     	ldp	q5, q6, [x13]
1000f8114:     	ldp	q7, q16, [x13, #0x20]
1000f8118:     	fdiv.2d	v1, v1, v5
1000f811c:     	mov	d5, v1[1]
1000f8120:     	fdiv.2d	v2, v2, v6
1000f8124:     	mov	d6, v2[1]
1000f8128:     	fdiv.2d	v3, v3, v7
1000f812c:     	mov	d7, v3[1]
1000f8130:     	fdiv.2d	v4, v4, v16
1000f8134:     	mov	d16, v4[1]
1000f8138:     	fadd	d0, d0, d1
1000f813c:     	fadd	d0, d0, d5
1000f8140:     	fadd	d0, d0, d2
1000f8144:     	fadd	d0, d0, d6
1000f8148:     	fadd	d0, d0, d3
1000f814c:     	fadd	d0, d0, d7
1000f8150:     	fadd	d0, d0, d4
1000f8154:     	fadd	d0, d0, d16
1000f8158:     	cmp	x8, #0x8
1000f815c:     	b.eq	0x1000f8260 <mech_gpu::sequence::runtime::resident::scalar_row+0xf78>
1000f8160:     	ldp	q1, q2, [x20, #0x40]
1000f8164:     	ldp	q3, q4, [x20, #0x60]
1000f8168:     	ldp	q5, q6, [x13, #0x40]
1000f816c:     	ldp	q7, q16, [x13, #0x60]
1000f8170:     	fdiv.2d	v1, v1, v5
1000f8174:     	mov	d5, v1[1]
1000f8178:     	fdiv.2d	v2, v2, v6
1000f817c:     	mov	d6, v2[1]
1000f8180:     	fdiv.2d	v3, v3, v7
1000f8184:     	mov	d7, v3[1]
1000f8188:     	fdiv.2d	v4, v4, v16
1000f818c:     	mov	d16, v4[1]
1000f8190:     	fadd	d0, d0, d1
1000f8194:     	fadd	d0, d0, d5
1000f8198:     	fadd	d0, d0, d2
1000f819c:     	fadd	d0, d0, d6
1000f81a0:     	fadd	d0, d0, d3
1000f81a4:     	fadd	d0, d0, d7
1000f81a8:     	fadd	d0, d0, d4
1000f81ac:     	fadd	d0, d0, d16
1000f81b0:     	cmp	x8, #0x10
1000f81b4:     	b.eq	0x1000f8260 <mech_gpu::sequence::runtime::resident::scalar_row+0xf78>
1000f81b8:     	ldp	q1, q2, [x20, #0x80]
1000f81bc:     	ldp	q3, q4, [x20, #0xa0]
1000f81c0:     	ldp	q5, q6, [x13, #0x80]
1000f81c4:     	ldp	q7, q16, [x13, #0xa0]
1000f81c8:     	fdiv.2d	v1, v1, v5
1000f81cc:     	mov	d5, v1[1]
1000f81d0:     	fdiv.2d	v2, v2, v6
1000f81d4:     	mov	d6, v2[1]
1000f81d8:     	fdiv.2d	v3, v3, v7
1000f81dc:     	mov	d7, v3[1]
1000f81e0:     	fdiv.2d	v4, v4, v16
1000f81e4:     	mov	d16, v4[1]
1000f81e8:     	fadd	d0, d0, d1
1000f81ec:     	fadd	d0, d0, d5
1000f81f0:     	fadd	d0, d0, d2
1000f81f4:     	fadd	d0, d0, d6
1000f81f8:     	fadd	d0, d0, d3
1000f81fc:     	fadd	d0, d0, d7
1000f8200:     	fadd	d0, d0, d4
1000f8204:     	fadd	d0, d0, d16
1000f8208:     	cmp	x8, #0x18
1000f820c:     	b.eq	0x1000f8260 <mech_gpu::sequence::runtime::resident::scalar_row+0xf78>
1000f8210:     	ldp	q1, q2, [x20, #0xc0]
1000f8214:     	ldp	q3, q4, [x20, #0xe0]
1000f8218:     	ldp	q5, q6, [x13, #0xc0]
1000f821c:     	ldp	q7, q16, [x13, #0xe0]
1000f8220:     	fdiv.2d	v1, v1, v5
1000f8224:     	mov	d5, v1[1]
1000f8228:     	fdiv.2d	v2, v2, v6
1000f822c:     	mov	d6, v2[1]
1000f8230:     	fdiv.2d	v3, v3, v7
1000f8234:     	mov	d7, v3[1]
1000f8238:     	fdiv.2d	v4, v4, v16
1000f823c:     	mov	d16, v4[1]
1000f8240:     	fadd	d0, d0, d1
1000f8244:     	fadd	d0, d0, d5
1000f8248:     	fadd	d0, d0, d2
1000f824c:     	fadd	d0, d0, d6
1000f8250:     	fadd	d0, d0, d3
1000f8254:     	fadd	d0, d0, d7
1000f8258:     	fadd	d0, d0, d4
1000f825c:     	fadd	d0, d0, d16
1000f8260:     	cmp	x30, x8
1000f8264:     	b.eq	0x1000f742c <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
1000f8268:     	lsl	x10, x8, #3
1000f826c:     	add	x9, x13, x10
1000f8270:     	add	x10, x20, x10
1000f8274:     	sub	x8, x30, x8
1000f8278:     	ldr	d1, [x10], #0x8
1000f827c:     	ldr	d2, [x9], #0x8
1000f8280:     	fdiv	d1, d1, d2
1000f8284:     	fadd	d0, d0, d1
1000f8288:     	subs	x8, x8, #0x1
1000f828c:     	b.ne	0x1000f8278 <mech_gpu::sequence::runtime::resident::scalar_row+0xf90>
1000f8290:     	b	0x1000f742c <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
1000f8294:     	and	x8, x30, #0x38
1000f8298:     	ldp	q1, q2, [x20]
1000f829c:     	ldp	q3, q4, [x20, #0x20]
1000f82a0:     	ldp	q5, q6, [x13]
1000f82a4:     	ldp	q7, q16, [x13, #0x20]
1000f82a8:     	fmul.2d	v1, v1, v5
1000f82ac:     	mov	d5, v1[1]
1000f82b0:     	fmul.2d	v2, v2, v6
1000f82b4:     	mov	d6, v2[1]
1000f82b8:     	fmul.2d	v3, v3, v7
1000f82bc:     	mov	d7, v3[1]
1000f82c0:     	fmul.2d	v4, v4, v16
1000f82c4:     	mov	d16, v4[1]
1000f82c8:     	fadd	d0, d0, d1
1000f82cc:     	fadd	d0, d0, d5
1000f82d0:     	fadd	d0, d0, d2
1000f82d4:     	fadd	d0, d0, d6
1000f82d8:     	fadd	d0, d0, d3
1000f82dc:     	fadd	d0, d0, d7
1000f82e0:     	fadd	d0, d0, d4
1000f82e4:     	fadd	d0, d0, d16
1000f82e8:     	cmp	x8, #0x8
1000f82ec:     	b.eq	0x1000f83f0 <mech_gpu::sequence::runtime::resident::scalar_row+0x1108>
1000f82f0:     	ldp	q1, q2, [x20, #0x40]
1000f82f4:     	ldp	q3, q4, [x20, #0x60]
1000f82f8:     	ldp	q5, q6, [x13, #0x40]
1000f82fc:     	ldp	q7, q16, [x13, #0x60]
1000f8300:     	fmul.2d	v1, v1, v5
1000f8304:     	mov	d5, v1[1]
1000f8308:     	fmul.2d	v2, v2, v6
1000f830c:     	mov	d6, v2[1]
1000f8310:     	fmul.2d	v3, v3, v7
1000f8314:     	mov	d7, v3[1]
1000f8318:     	fmul.2d	v4, v4, v16
1000f831c:     	mov	d16, v4[1]
1000f8320:     	fadd	d0, d0, d1
1000f8324:     	fadd	d0, d0, d5
1000f8328:     	fadd	d0, d0, d2
1000f832c:     	fadd	d0, d0, d6
1000f8330:     	fadd	d0, d0, d3
1000f8334:     	fadd	d0, d0, d7
1000f8338:     	fadd	d0, d0, d4
1000f833c:     	fadd	d0, d0, d16
1000f8340:     	cmp	x8, #0x10
1000f8344:     	b.eq	0x1000f83f0 <mech_gpu::sequence::runtime::resident::scalar_row+0x1108>
1000f8348:     	ldp	q1, q2, [x20, #0x80]
1000f834c:     	ldp	q3, q4, [x20, #0xa0]
1000f8350:     	ldp	q5, q6, [x13, #0x80]
1000f8354:     	ldp	q7, q16, [x13, #0xa0]
1000f8358:     	fmul.2d	v1, v1, v5
1000f835c:     	mov	d5, v1[1]
1000f8360:     	fmul.2d	v2, v2, v6
1000f8364:     	mov	d6, v2[1]
1000f8368:     	fmul.2d	v3, v3, v7
1000f836c:     	mov	d7, v3[1]
1000f8370:     	fmul.2d	v4, v4, v16
1000f8374:     	mov	d16, v4[1]
1000f8378:     	fadd	d0, d0, d1
1000f837c:     	fadd	d0, d0, d5
1000f8380:     	fadd	d0, d0, d2
1000f8384:     	fadd	d0, d0, d6
1000f8388:     	fadd	d0, d0, d3
1000f838c:     	fadd	d0, d0, d7
1000f8390:     	fadd	d0, d0, d4
1000f8394:     	fadd	d0, d0, d16
1000f8398:     	cmp	x8, #0x18
1000f839c:     	b.eq	0x1000f83f0 <mech_gpu::sequence::runtime::resident::scalar_row+0x1108>
1000f83a0:     	ldp	q1, q2, [x20, #0xc0]
1000f83a4:     	ldp	q3, q4, [x20, #0xe0]
1000f83a8:     	ldp	q5, q6, [x13, #0xc0]
1000f83ac:     	ldp	q7, q16, [x13, #0xe0]
1000f83b0:     	fmul.2d	v1, v1, v5
1000f83b4:     	mov	d5, v1[1]
1000f83b8:     	fmul.2d	v2, v2, v6
1000f83bc:     	mov	d6, v2[1]
1000f83c0:     	fmul.2d	v3, v3, v7
1000f83c4:     	mov	d7, v3[1]
1000f83c8:     	fmul.2d	v4, v4, v16
1000f83cc:     	mov	d16, v4[1]
1000f83d0:     	fadd	d0, d0, d1
1000f83d4:     	fadd	d0, d0, d5
1000f83d8:     	fadd	d0, d0, d2
1000f83dc:     	fadd	d0, d0, d6
1000f83e0:     	fadd	d0, d0, d3
1000f83e4:     	fadd	d0, d0, d7
1000f83e8:     	fadd	d0, d0, d4
1000f83ec:     	fadd	d0, d0, d16
1000f83f0:     	cmp	x30, x8
1000f83f4:     	b.eq	0x1000f742c <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
1000f83f8:     	lsl	x10, x8, #3
1000f83fc:     	add	x9, x13, x10
1000f8400:     	add	x10, x20, x10
1000f8404:     	sub	x8, x30, x8
1000f8408:     	ldr	d1, [x10], #0x8
1000f840c:     	ldr	d2, [x9], #0x8
1000f8410:     	fmul	d1, d1, d2
1000f8414:     	fadd	d0, d0, d1
1000f8418:     	subs	x8, x8, #0x1
1000f841c:     	b.ne	0x1000f8408 <mech_gpu::sequence::runtime::resident::scalar_row+0x1120>
1000f8420:     	b	0x1000f742c <mech_gpu::sequence::runtime::resident::scalar_row+0x144>
1000f8424:     	movi.2d	v8, #0000000000000000
1000f8428:     	str	d8, [x25, #0x8]
1000f842c:     	mov	x8, #-0x8000000000000000 ; =-9223372036854775808
1000f8430:     	str	x8, [x25]
1000f8434:     	b	0x1000f84f4 <mech_gpu::sequence::runtime::resident::scalar_row+0x120c>
1000f8438:     	movi.2d	v0, #0000000000000000
1000f843c:     	ldr	x9, [sp, #0x8]
1000f8440:     	str	d0, [x9, #0x8]
1000f8444:     	mov	x8, #-0x8000000000000000 ; =-9223372036854775808
1000f8448:     	str	x8, [x9]
1000f844c:     	b	0x1000f84f4 <mech_gpu::sequence::runtime::resident::scalar_row+0x120c>
1000f8450:     	mov	x23, #0x0               ; =0
1000f8454:     	movi.2d	v8, #0000000000000000
1000f8458:     	mov	x24, #-0x8000000000000000 ; =-9223372036854775808
1000f845c:     	tbz	w6, #0x0, 0x1000f84a0 <mech_gpu::sequence::runtime::resident::scalar_row+0x11b8>
1000f8460:     	ldr	d9, [x20, x23, lsl #3]
1000f8464:     	stp	x23, x21, [sp, #0x48]
1000f8468:     	add	x0, sp, #0x58
1000f846c:     	add	x2, sp, #0x48
1000f8470:     	mov	x1, x22
1000f8474:     	bl	0x1000e637c <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
1000f8478:     	ldr	x8, [sp, #0x58]
1000f847c:     	cmp	x8, x24
1000f8480:     	b.ne	0x1000f84e0 <mech_gpu::sequence::runtime::resident::scalar_row+0x11f8>
1000f8484:     	add	x23, x23, #0x1
1000f8488:     	ldr	d0, [sp, #0x60]
1000f848c:     	fmul	d0, d9, d0
1000f8490:     	fadd	d8, d8, d0
1000f8494:     	subs	x19, x19, #0x8
1000f8498:     	b.ne	0x1000f8460 <mech_gpu::sequence::runtime::resident::scalar_row+0x1178>
1000f849c:     	b	0x1000f8428 <mech_gpu::sequence::runtime::resident::scalar_row+0x1140>
1000f84a0:     	ldr	d9, [x20, x23, lsl #3]
1000f84a4:     	stp	x21, x23, [sp, #0x48]
1000f84a8:     	add	x0, sp, #0x58
1000f84ac:     	add	x2, sp, #0x48
1000f84b0:     	mov	x1, x22
1000f84b4:     	bl	0x1000e637c <<mech_gpu::sequence::runtime::resident::ScalarTape>::evaluate>
1000f84b8:     	ldr	x8, [sp, #0x58]
1000f84bc:     	cmp	x8, x24
1000f84c0:     	b.ne	0x1000f84e0 <mech_gpu::sequence::runtime::resident::scalar_row+0x11f8>
1000f84c4:     	add	x23, x23, #0x1
1000f84c8:     	ldr	d0, [sp, #0x60]
1000f84cc:     	fmul	d0, d9, d0
1000f84d0:     	fadd	d8, d8, d0
1000f84d4:     	subs	x19, x19, #0x8
1000f84d8:     	b.ne	0x1000f84a0 <mech_gpu::sequence::runtime::resident::scalar_row+0x11b8>
1000f84dc:     	b	0x1000f8428 <mech_gpu::sequence::runtime::resident::scalar_row+0x1140>
1000f84e0:     	ldr	d0, [sp, #0x60]
1000f84e4:     	ldr	x9, [sp, #0x68]
1000f84e8:     	str	x8, [x25]
1000f84ec:     	str	d0, [x25, #0x8]
1000f84f0:     	str	x9, [x25, #0x10]
1000f84f4:     	ldp	x29, x30, [sp, #0xd0]
1000f84f8:     	ldp	x20, x19, [sp, #0xc0]
1000f84fc:     	ldp	x22, x21, [sp, #0xb0]
1000f8500:     	ldp	x24, x23, [sp, #0xa0]
1000f8504:     	ldp	x26, x25, [sp, #0x90]
1000f8508:     	ldp	x28, x27, [sp, #0x80]
1000f850c:     	ldp	d9, d8, [sp, #0x70]
1000f8510:     	add	sp, sp, #0xe0
1000f8514:     	ret
1000f8518:     	adrp	x0, 0x101882000 <_anon.f7c957ecca79fd017cf77e2dfd4b2b6d.428+0x4>
1000f851c:     	add	x0, x0, #0x831
1000f8520:     	adrp	x2, 0x101d67000 <_anon.6813bbecbd0a1b2869039a79ca820c23.58+0xe8>
1000f8524:     	add	x2, x2, #0xa08
1000f8528:     	mov	w1, #0x13               ; =19
1000f852c:     	bl	0x10172cae4 <core::panicking::panic_fmt>
1000f8530:     	adrp	x0, 0x101d67000 <_anon.6813bbecbd0a1b2869039a79ca820c23.58+0xe8>
1000f8534:     	add	x0, x0, #0xa68
1000f8538:     	bl	0x10172cbdc <core::panicking::panic_const::panic_const_div_by_zero>
1000f853c:     	adrp	x2, 0x101d67000 <_anon.6813bbecbd0a1b2869039a79ca820c23.58+0xe8>
1000f8540:     	add	x2, x2, #0x9f0
1000f8544:     	mov	x0, x8
1000f8548:     	bl	0x10172c974 <core::panicking::panic_bounds_check>
1000f854c:     	adrp	x2, 0x101d67000 <_anon.6813bbecbd0a1b2869039a79ca820c23.58+0xe8>
1000f8550:     	add	x2, x2, #0xa50
1000f8554:     	mov	x0, x10
1000f8558:     	mov	x1, x8
1000f855c:     	bl	0x10172c974 <core::panicking::panic_bounds_check>
1000f8560:     	adrp	x2, 0x101d67000 <_anon.6813bbecbd0a1b2869039a79ca820c23.58+0xe8>
1000f8564:     	add	x2, x2, #0xa38
1000f8568:     	mov	x0, x9
1000f856c:     	mov	x1, x8
1000f8570:     	bl	0x10172c974 <core::panicking::panic_bounds_check>
1000f8574:     	adrp	x2, 0x101d67000 <_anon.6813bbecbd0a1b2869039a79ca820c23.58+0xe8>
1000f8578:     	add	x2, x2, #0xa20
1000f857c:     	mov	x0, #0x0                ; =0
1000f8580:     	mov	x1, #0x0                ; =0
1000f8584:     	bl	0x10172c974 <core::panicking::panic_bounds_check>
1000f8588:     	adrp	x2, 0x101d67000 <_anon.6813bbecbd0a1b2869039a79ca820c23.58+0xe8>
1000f858c:     	add	x2, x2, #0x9d8
1000f8590:     	ldr	x0, [sp, #0x28]
1000f8594:     	bl	0x10172c974 <core::panicking::panic_bounds_check>

00000001000f8598 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone>:
1000f8598:     	sub	sp, sp, #0x90
1000f859c:     	stp	x28, x27, [sp, #0x30]
1000f85a0:     	stp	x26, x25, [sp, #0x40]
1000f85a4:     	stp	x24, x23, [sp, #0x50]
1000f85a8:     	stp	x22, x21, [sp, #0x60]
1000f85ac:     	stp	x20, x19, [sp, #0x70]
1000f85b0:     	stp	x29, x30, [sp, #0x80]
1000f85b4:     	add	x29, sp, #0x80
1000f85b8:     	ldr	x23, [x1, #0x18]
1000f85bc:     	lsl	x20, x23, #4
1000f85c0:     	lsr	x8, x23, #60
1000f85c4:     	cmp	x8, #0x0
1000f85c8:     	mov	x8, #0x7ffffffffffffff8 ; =9223372036854775800
1000f85cc:     	ccmp	x20, x8, #0x2, eq
1000f85d0:     	b.ls	0x1000f85e4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x4c>
1000f85d4:     	mov	x21, #0x0               ; =0
1000f85d8:     	mov	x0, x21
1000f85dc:     	mov	x1, x20
1000f85e0:     	bl	0x10172c724 <alloc::raw_vec::handle_error>
1000f85e4:     	mov	x22, x0
1000f85e8:     	cbz	x20, 0x1000f86b8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x120>
1000f85ec:     	mov	x19, x1
1000f85f0:     	bl	0x1000014f8 <__rustc::__rust_no_alloc_shim_is_unstable_v2>
1000f85f4:     	mov	w21, #0x8               ; =8
1000f85f8:     	mov	x0, x20
1000f85fc:     	mov	w1, #0x8                ; =8
1000f8600:     	bl	0x1000014e8 <__rustc::__rust_alloc>
1000f8604:     	cbz	x0, 0x1000f85d8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x40>
1000f8608:     	mov	x26, x23
1000f860c:     	mov	x1, x19
1000f8610:     	stp	x22, x23, [sp, #0x8]
1000f8614:     	stp	xzr, xzr, [sp, #0x20]
1000f8618:     	str	x0, [sp, #0x18]
1000f861c:     	cbz	x23, 0x1000f86d0 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x138>
1000f8620:     	ldr	x24, [x1]
1000f8624:     	ldr	x8, [x1, #0x10]
1000f8628:     	cmp	x8, x24
1000f862c:     	csel	x9, xzr, x24, lo
1000f8630:     	sub	x25, x8, x9
1000f8634:     	sub	x8, x24, x25
1000f8638:     	subs	x22, x23, x8
1000f863c:     	b.hi	0x1000f8648 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0xb0>
1000f8640:     	mov	x22, #0x0               ; =0
1000f8644:     	add	x24, x25, x23
1000f8648:     	ldr	x21, [x1, #0x8]
1000f864c:     	sub	x23, x24, x25
1000f8650:     	add	x27, x22, x23
1000f8654:     	cmp	x27, x26
1000f8658:     	b.ls	0x1000f86f0 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x158>
1000f865c:     	add	x0, sp, #0x10
1000f8660:     	add	x2, x22, x23
1000f8664:     	mov	x1, #0x0                ; =0
1000f8668:     	mov	w3, #0x8                ; =8
1000f866c:     	mov	w4, #0x10               ; =16
1000f8670:     	bl	0x1016f75ac <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
1000f8674:     	ldp	x19, x20, [sp, #0x20]
1000f8678:     	ldr	x28, [sp, #0x10]
1000f867c:     	sub	x8, x26, x20
1000f8680:     	cmp	x19, x8
1000f8684:     	b.ls	0x1000f86fc <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x164>
1000f8688:     	sub	x8, x26, x19
1000f868c:     	sub	x9, x20, x8
1000f8690:     	cmp	x8, x9
1000f8694:     	b.ls	0x1000f89a0 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x408>
1000f8698:     	sub	x10, x28, x26
1000f869c:     	cmp	x10, x9
1000f86a0:     	b.lo	0x1000f89a0 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x408>
1000f86a4:     	ldr	x1, [sp, #0x18]
1000f86a8:     	add	x0, x1, x26, lsl #4
1000f86ac:     	lsl	x2, x9, #4
1000f86b0:     	bl	0x10172cfd8 <_writev+0x10172cfd8>
1000f86b4:     	b	0x1000f86fc <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x164>
1000f86b8:     	mov	x26, #0x0               ; =0
1000f86bc:     	mov	w0, #0x8                ; =8
1000f86c0:     	stp	x22, x26, [sp, #0x8]
1000f86c4:     	stp	xzr, xzr, [sp, #0x20]
1000f86c8:     	str	x0, [sp, #0x18]
1000f86cc:     	cbnz	x23, 0x1000f8620 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x88>
1000f86d0:     	mov	x25, #0x0               ; =0
1000f86d4:     	mov	x24, #0x0               ; =0
1000f86d8:     	mov	x22, #0x0               ; =0
1000f86dc:     	ldr	x21, [x1, #0x8]
1000f86e0:     	sub	x23, x24, x25
1000f86e4:     	add	x27, x22, x23
1000f86e8:     	cmp	x27, x26
1000f86ec:     	b.hi	0x1000f865c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0xc4>
1000f86f0:     	mov	x19, #0x0               ; =0
1000f86f4:     	mov	x20, #0x0               ; =0
1000f86f8:     	mov	x28, x26
1000f86fc:     	lsl	x17, x25, #4
1000f8700:     	add	x9, x21, x17
1000f8704:     	lsl	x11, x24, #4
1000f8708:     	add	x8, x19, x20
1000f870c:     	cmp	x8, x28
1000f8710:     	csel	x10, xzr, x28, lo
1000f8714:     	sub	x13, x8, x10
1000f8718:     	sub	x14, x28, x13
1000f871c:     	cmp	x14, x27
1000f8720:     	b.hs	0x1000f8798 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x200>
1000f8724:     	lsl	x8, x22, #4
1000f8728:     	cmp	x28, x13
1000f872c:     	b.ne	0x1000f87a8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x210>
1000f8730:     	mov	x12, #0x0               ; =0
1000f8734:     	add	x10, x21, x11
1000f8738:     	cmp	x9, x10
1000f873c:     	ldr	x17, [sp, #0x8]
1000f8740:     	b.eq	0x1000f881c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x284>
1000f8744:     	sub	x10, x10, x9
1000f8748:     	lsr	x11, x10, #4
1000f874c:     	add	x10, x12, x11
1000f8750:     	ldr	x12, [sp, #0x18]
1000f8754:     	mov	w13, #0x1               ; =1
1000f8758:     	mov	x14, x11
1000f875c:     	ldr	q0, [x9], #0x10
1000f8760:     	fmov	x15, d0
1000f8764:     	ldadd	x13, x15, [x15]
1000f8768:     	tbnz	x15, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f876c:     	mov.d	x15, v0[1]
1000f8770:     	ldadd	x13, x15, [x15]
1000f8774:     	tbnz	x15, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f8778:     	str	q0, [x12], #0x10
1000f877c:     	subs	x14, x14, #0x1
1000f8780:     	b.ne	0x1000f875c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x1c4>
1000f8784:     	mov	x9, x21
1000f8788:     	add	x8, x21, x8
1000f878c:     	cmp	x21, x8
1000f8790:     	b.ne	0x1000f8834 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x29c>
1000f8794:     	b	0x1000f896c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d4>
1000f8798:     	cmp	x17, x11
1000f879c:     	b.ne	0x1000f8878 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x2e0>
1000f87a0:     	mov	x23, #0x0               ; =0
1000f87a4:     	b	0x1000f88b0 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x318>
1000f87a8:     	mov	x15, #0x0               ; =0
1000f87ac:     	mov	x12, #0x0               ; =0
1000f87b0:     	sub	x10, x11, x17
1000f87b4:     	lsr	x10, x10, #4
1000f87b8:     	ldr	x16, [sp, #0x18]
1000f87bc:     	lsl	x13, x13, #4
1000f87c0:     	add	x16, x16, x13
1000f87c4:     	sub	x17, x17, x11
1000f87c8:     	mov	w0, #0x1                ; =1
1000f87cc:     	cmp	x17, x15
1000f87d0:     	b.eq	0x1000f8900 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x368>
1000f87d4:     	ldr	q0, [x9, x12, lsl #4]
1000f87d8:     	fmov	x1, d0
1000f87dc:     	ldadd	x0, x1, [x1]
1000f87e0:     	tbnz	x1, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f87e4:     	mov.d	x1, v0[1]
1000f87e8:     	ldadd	x0, x1, [x1]
1000f87ec:     	tbnz	x1, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f87f0:     	str	q0, [x16, x12, lsl #4]
1000f87f4:     	add	x12, x12, #0x1
1000f87f8:     	add	x13, x13, #0x10
1000f87fc:     	sub	x15, x15, #0x10
1000f8800:     	cmp	x14, x12
1000f8804:     	b.ne	0x1000f87cc <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x234>
1000f8808:     	sub	x9, x9, x15
1000f880c:     	add	x10, x21, x11
1000f8810:     	cmp	x9, x10
1000f8814:     	ldr	x17, [sp, #0x8]
1000f8818:     	b.ne	0x1000f8744 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x1ac>
1000f881c:     	mov	x11, #0x0               ; =0
1000f8820:     	mov	x9, x21
1000f8824:     	mov	x10, x12
1000f8828:     	add	x8, x21, x8
1000f882c:     	cmp	x21, x8
1000f8830:     	b.eq	0x1000f896c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d4>
1000f8834:     	sub	x8, x8, x9
1000f8838:     	lsr	x8, x8, #4
1000f883c:     	add	x23, x10, x8
1000f8840:     	ldr	x10, [sp, #0x18]
1000f8844:     	add	x10, x10, x11, lsl #4
1000f8848:     	mov	w11, #0x1               ; =1
1000f884c:     	ldr	q0, [x9], #0x10
1000f8850:     	fmov	x12, d0
1000f8854:     	ldadd	x11, x12, [x12]
1000f8858:     	tbnz	x12, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f885c:     	mov.d	x12, v0[1]
1000f8860:     	ldadd	x11, x12, [x12]
1000f8864:     	tbnz	x12, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f8868:     	str	q0, [x10], #0x10
1000f886c:     	subs	x8, x8, #0x1
1000f8870:     	b.ne	0x1000f884c <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x2b4>
1000f8874:     	b	0x1000f8970 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d8>
1000f8878:     	ldr	x8, [sp, #0x18]
1000f887c:     	add	x8, x8, x13, lsl #4
1000f8880:     	mov	w10, #0x1               ; =1
1000f8884:     	mov	x11, x23
1000f8888:     	ldr	q0, [x9], #0x10
1000f888c:     	fmov	x12, d0
1000f8890:     	ldadd	x10, x12, [x12]
1000f8894:     	tbnz	x12, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f8898:     	mov.d	x12, v0[1]
1000f889c:     	ldadd	x10, x12, [x12]
1000f88a0:     	tbnz	x12, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f88a4:     	str	q0, [x8], #0x10
1000f88a8:     	subs	x11, x11, #0x1
1000f88ac:     	b.ne	0x1000f8888 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x2f0>
1000f88b0:     	cbz	x22, 0x1000f88f8 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x360>
1000f88b4:     	add	x8, x22, x23
1000f88b8:     	ldr	x9, [sp, #0x18]
1000f88bc:     	lsl	x10, x23, #4
1000f88c0:     	add	x10, x10, x13, lsl #4
1000f88c4:     	add	x9, x9, x10
1000f88c8:     	mov	w10, #0x1               ; =1
1000f88cc:     	ldr	q0, [x21], #0x10
1000f88d0:     	fmov	x11, d0
1000f88d4:     	ldadd	x10, x11, [x11]
1000f88d8:     	tbnz	x11, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f88dc:     	mov.d	x11, v0[1]
1000f88e0:     	ldadd	x10, x11, [x11]
1000f88e4:     	tbnz	x11, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f88e8:     	str	q0, [x9], #0x10
1000f88ec:     	subs	x22, x22, #0x1
1000f88f0:     	b.ne	0x1000f88cc <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x334>
1000f88f4:     	mov	x23, x8
1000f88f8:     	ldr	x17, [sp, #0x8]
1000f88fc:     	b	0x1000f8970 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d8>
1000f8900:     	mov	x11, #0x0               ; =0
1000f8904:     	add	x23, x22, x10
1000f8908:     	ldr	x9, [sp, #0x18]
1000f890c:     	add	x13, x9, x13
1000f8910:     	sub	x12, x14, x12
1000f8914:     	mov	w14, #0x1               ; =1
1000f8918:     	mov	x15, x8
1000f891c:     	mov	x9, x21
1000f8920:     	ldr	x17, [sp, #0x8]
1000f8924:     	cbz	x15, 0x1000f8970 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x3d8>
1000f8928:     	ldr	q0, [x9], #0x10
1000f892c:     	fmov	x16, d0
1000f8930:     	ldadd	x14, x16, [x16]
1000f8934:     	tbnz	x16, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f8938:     	mov.d	x16, v0[1]
1000f893c:     	ldadd	x14, x16, [x16]
1000f8940:     	tbnz	x16, #0x3f, 0x1000f89c4 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x42c>
1000f8944:     	str	q0, [x13, x11, lsl #4]
1000f8948:     	add	x10, x10, #0x1
1000f894c:     	sub	x15, x15, #0x10
1000f8950:     	add	x11, x11, #0x1
1000f8954:     	cmp	x12, x11
1000f8958:     	b.ne	0x1000f8924 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x38c>
1000f895c:     	mov	x11, #0x0               ; =0
1000f8960:     	add	x8, x21, x8
1000f8964:     	cmp	x9, x8
1000f8968:     	b.ne	0x1000f8834 <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x29c>
1000f896c:     	mov	x23, x10
1000f8970:     	add	x8, x20, x23
1000f8974:     	str	x8, [sp, #0x28]
1000f8978:     	ldp	q0, q1, [sp, #0x10]
1000f897c:     	stp	q0, q1, [x17]
1000f8980:     	ldp	x29, x30, [sp, #0x80]
1000f8984:     	ldp	x20, x19, [sp, #0x70]
1000f8988:     	ldp	x22, x21, [sp, #0x60]
1000f898c:     	ldp	x24, x23, [sp, #0x50]
1000f8990:     	ldp	x26, x25, [sp, #0x40]
1000f8994:     	ldp	x28, x27, [sp, #0x30]
1000f8998:     	add	sp, sp, #0x90
1000f899c:     	ret
1000f89a0:     	sub	x26, x28, x8
1000f89a4:     	ldr	x9, [sp, #0x18]
1000f89a8:     	add	x1, x9, x19, lsl #4
1000f89ac:     	add	x0, x9, x26, lsl #4
1000f89b0:     	lsl	x2, x8, #4
1000f89b4:     	bl	0x10172cfe4 <_writev+0x10172cfe4>
1000f89b8:     	str	x26, [sp, #0x20]
1000f89bc:     	mov	x19, x26
1000f89c0:     	b	0x1000f86fc <<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record> as core::clone::Clone>::clone+0x164>
1000f89c4:     	brk	#0x1
1000f89c8:     	mov	x19, x0
1000f89cc:     	add	x0, sp, #0x10
1000f89d0:     	bl	0x1000de404 <core::ptr::drop_in_place::<alloc::collections::vec_deque::VecDeque<mech_gpu::sequence::runtime::Record>>>
1000f89d4:     	mov	x0, x19
1000f89d8:     	bl	0x10172cd08 <_writev+0x10172cd08>
1000f89dc:     	bl	0x10172c95c <core::panicking::panic_in_cleanup>

00000001000f89e0 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop>:
1000f89e0:     	stp	x24, x23, [sp, #-0x40]!
1000f89e4:     	stp	x22, x21, [sp, #0x10]
1000f89e8:     	stp	x20, x19, [sp, #0x20]
1000f89ec:     	stp	x29, x30, [sp, #0x30]
1000f89f0:     	add	x29, sp, #0x30
1000f89f4:     	mov	x20, x0
1000f89f8:     	ldr	x8, [x0, #0x20]
1000f89fc:     	cbz	x8, 0x1000f8a34 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x54>
1000f8a00:     	cmp	x8, #0x2
1000f8a04:     	b.eq	0x1000f8a84 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xa4>
1000f8a08:     	cmp	x8, #0x3
1000f8a0c:     	b.ne	0x1000f8a98 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xb8>
1000f8a10:     	ldp	x19, x8, [x20, #0x8]
1000f8a14:     	add	x23, x8, #0x1
1000f8a18:     	mov	x0, x19
1000f8a1c:     	subs	x23, x23, #0x1
1000f8a20:     	b.eq	0x1000f8a58 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x78>
1000f8a24:     	add	x22, x0, #0x38
1000f8a28:     	bl	0x100023600 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
1000f8a2c:     	mov	x0, x22
1000f8a30:     	b	0x1000f8a1c <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x3c>
1000f8a34:     	ldp	x19, x8, [x20, #0x8]
1000f8a38:     	add	x23, x8, #0x1
1000f8a3c:     	mov	x0, x19
1000f8a40:     	subs	x23, x23, #0x1
1000f8a44:     	b.eq	0x1000f8a58 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x78>
1000f8a48:     	add	x22, x0, #0x38
1000f8a4c:     	bl	0x100023600 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
1000f8a50:     	mov	x0, x22
1000f8a54:     	b	0x1000f8a40 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x60>
1000f8a58:     	ldr	x8, [x20]
1000f8a5c:     	cbz	x8, 0x1000f8a84 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xa4>
1000f8a60:     	lsl	x9, x8, #6
1000f8a64:     	sub	x1, x9, x8, lsl #3
1000f8a68:     	mov	x0, x19
1000f8a6c:     	mov	w2, #0x8                ; =8
1000f8a70:     	ldp	x29, x30, [sp, #0x30]
1000f8a74:     	ldp	x20, x19, [sp, #0x20]
1000f8a78:     	ldp	x22, x21, [sp, #0x10]
1000f8a7c:     	ldp	x24, x23, [sp], #0x40
1000f8a80:     	b	0x1000014ec <__rustc::__rust_dealloc>
1000f8a84:     	ldp	x29, x30, [sp, #0x30]
1000f8a88:     	ldp	x20, x19, [sp, #0x20]
1000f8a8c:     	ldp	x22, x21, [sp, #0x10]
1000f8a90:     	ldp	x24, x23, [sp], #0x40
1000f8a94:     	ret
1000f8a98:     	adrp	x0, 0x101882000 <_anon.f7c957ecca79fd017cf77e2dfd4b2b6d.428+0x4>
1000f8a9c:     	add	x0, x0, #0x997
1000f8aa0:     	adrp	x2, 0x101d67000 <_anon.6813bbecbd0a1b2869039a79ca820c23.58+0xe8>
1000f8aa4:     	add	x2, x2, #0x238
1000f8aa8:     	mov	w1, #0x79               ; =121
1000f8aac:     	bl	0x10172cae4 <core::panicking::panic_fmt>
1000f8ab0:     	mov	x21, x0
1000f8ab4:     	subs	x23, x23, #0x1
1000f8ab8:     	b.eq	0x1000f8af4 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x114>
1000f8abc:     	add	x24, x22, #0x38
1000f8ac0:     	mov	x0, x22
1000f8ac4:     	bl	0x100023600 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
1000f8ac8:     	mov	x22, x24
1000f8acc:     	b	0x1000f8ab4 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xd4>
1000f8ad0:     	bl	0x10172c95c <core::panicking::panic_in_cleanup>
1000f8ad4:     	mov	x21, x0
1000f8ad8:     	subs	x23, x23, #0x1
1000f8adc:     	b.eq	0x1000f8af4 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x114>
1000f8ae0:     	add	x24, x22, #0x38
1000f8ae4:     	mov	x0, x22
1000f8ae8:     	bl	0x100023600 <core::ptr::drop_in_place::<std::backtrace::BacktraceFrame>>
1000f8aec:     	mov	x22, x24
1000f8af0:     	b	0x1000f8ad8 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0xf8>
1000f8af4:     	ldr	x8, [x20]
1000f8af8:     	cbz	x8, 0x1000f8b10 <<std::sync::lazy_lock::LazyLock<std::backtrace::Capture, std::backtrace::helper::lazy_resolve::{closure#0}> as core::ops::drop::Drop>::drop+0x130>
1000f8afc:     	lsl	x9, x8, #6
1000f8b00:     	sub	x1, x9, x8, lsl #3
1000f8b04:     	mov	x0, x19
1000f8b08:     	mov	w2, #0x8                ; =8
1000f8b0c:     	bl	0x1000014ec <__rustc::__rust_dealloc>
1000f8b10:     	mov	x0, x21
1000f8b14:     	bl	0x10172cd08 <_writev+0x10172cd08>
1000f8b18:     	bl	0x10172c95c <core::panicking::panic_in_cleanup>

