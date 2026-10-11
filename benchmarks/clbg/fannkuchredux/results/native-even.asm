
benchmarks/clbg/fannkuchredux/.build/mech-fannkuch-even.project/mech-control.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000001330 <_mech_control_range_0>:
    1330: d503237f     	pacibsp
    1334: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
    1338: 910003fd     	mov	x29, sp
    133c: d10143ff     	sub	sp, sp, #0x50
    1340: aa0003e5     	mov	x5, x0
    1344: f9400040     	ldr	x0, [x2]
    1348: 9c002c40     	ldr	q0, 0x18d0 <_mech_control_range_0+0x5a0>
    134c: 4f00e401     	movi.16b	v1, #0x0
    1350: d1000402     	sub	x2, x0, #0x1
    1354: aa0503e4     	mov	x4, x5
    1358: 4ea01c05     	mov.16b	v5, v0
    135c: f100005f     	cmp	x2, #0x0
    1360: 54002181     	b.ne	0x1790 <_mech_control_range_0+0x460>
    1364: d2800004     	mov	x4, #0x0                ; =0
    1368: aa0403e2     	mov	x2, x4
    136c: aa0503eb     	mov	x11, x5
    1370: eb01017f     	cmp	x11, x1
    1374: 540000e3     	b.lo	0x1390 <_mech_control_range_0+0x60>
    1378: f9000064     	str	x4, [x3]
    137c: f9000462     	str	x2, [x3, #0x8]
    1380: d2800000     	mov	x0, #0x0                ; =0
    1384: 910143ff     	add	sp, sp, #0x50
    1388: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    138c: d65f0fff     	retab
    1390: 91000565     	add	x5, x11, #0x1
    1394: eb0100bf     	cmp	x5, x1
    1398: 54000aa3     	b.lo	0x14ec <_mech_control_range_0+0x1bc>
    139c: eb0100bf     	cmp	x5, x1
    13a0: 54000063     	b.lo	0x13ac <_mech_control_range_0+0x7c>
    13a4: 4ea51cba     	mov.16b	v26, v5
    13a8: 14000007     	b	0x13c4 <_mech_control_range_0+0x94>
    13ac: d280002f     	mov	x15, #0x1               ; =1
    13b0: 52800000     	mov	w0, #0x0                ; =0
    13b4: 4ea51cba     	mov.16b	v26, v5
    13b8: 53001c06     	uxtb	w6, w0
    13bc: 710000df     	cmp	w6, #0x0
    13c0: 540002e0     	b.eq	0x141c <_mech_control_range_0+0xec>
    13c4: d2800000     	mov	x0, #0x0                ; =0
    13c8: 0e013ca6     	umov.b	w6, v5[0]
    13cc: 53001cc6     	uxtb	w6, w6
    13d0: 710000df     	cmp	w6, #0x0
    13d4: 54000141     	b.ne	0x13fc <_mech_control_range_0+0xcc>
    13d8: 92400166     	and	x6, x11, #0x1
    13dc: 8b000087     	add	x7, x4, x0
    13e0: cb000088     	sub	x8, x4, x0
    13e4: f10000df     	cmp	x6, #0x0
    13e8: 9a8800e4     	csel	x4, x7, x8, eq
    13ec: eb00005f     	cmp	x2, x0
    13f0: 9a808042     	csel	x2, x2, x0, hi
    13f4: aa0503eb     	mov	x11, x5
    13f8: 14000060     	b	0x1578 <_mech_control_range_0+0x248>
    13fc: 90000009     	adrp	x9, 0x1000 <_mech_control_reduce+0x1000>
    1400: f9400129     	ldr	x9, [x9]
    1404: 0e013caa     	umov.b	w10, v5[0]
    1408: 9100054a     	add	x10, x10, #0x1
    140c: 3cea7930     	ldr	q16, [x9, x10, lsl #4]
    1410: 4e1000a5     	tbl.16b	v5, { v5 }, v16
    1414: 91000400     	add	x0, x0, #0x1
    1418: 17ffffec     	b	0x13c8 <_mech_control_range_0+0x98>
    141c: 910005ea     	add	x10, x15, #0x1
    1420: f100415f     	cmp	x10, #0x10
    1424: 9a9f87ec     	cset	x12, ls
    1428: f100415f     	cmp	x10, #0x10
    142c: 540000a9     	b.ls	0x1440 <_mech_control_range_0+0x110>
    1430: d2800080     	mov	x0, #0x4                ; =4
    1434: 910143ff     	add	sp, sp, #0x50
    1438: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    143c: d65f0fff     	retab
    1440: 9000000e     	adrp	x14, 0x1000 <_mech_control_reduce+0x1000>
    1444: f94001ce     	ldr	x14, [x14]
    1448: d2800080     	mov	x0, #0x4                ; =4
    144c: 3cea79d0     	ldr	q16, [x14, x10, lsl #4]
    1450: 9280000d     	mov	x13, #-0x1              ; =-1
    1454: eb0d01ff     	cmp	x15, x13
    1458: 9a9f07e6     	cset	x6, ne
    145c: 0a0c00c6     	and	w6, w6, w12
    1460: 53001cc7     	uxtb	w7, w6
    1464: b5000087     	cbnz	x7, 0x1474 <_mech_control_range_0+0x144>
    1468: 910143ff     	add	sp, sp, #0x50
    146c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1470: d65f0fff     	retab
    1474: 910103e7     	add	x7, sp, #0x40
    1478: 3c8403e1     	stur	q1, [sp, #0x40]
    147c: 386f68e7     	ldrb	w7, [x7, x15]
    1480: 53001cc6     	uxtb	w6, w6
    1484: b5000086     	cbnz	x6, 0x1494 <_mech_control_range_0+0x164>
    1488: 910143ff     	add	sp, sp, #0x50
    148c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1490: d65f0fff     	retab
    1494: 53001ce6     	uxtb	w6, w7
    1498: 910004c6     	add	x6, x6, #0x1
    149c: d2800007     	mov	x7, #0x0                ; =0
    14a0: eb0f00df     	cmp	x6, x15
    14a4: 9a8680e7     	csel	x7, x7, x6, hi
    14a8: f103fcff     	cmp	x7, #0xff
    14ac: 54000089     	b.ls	0x14bc <_mech_control_range_0+0x18c>
    14b0: 910143ff     	add	sp, sp, #0x50
    14b4: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    14b8: d65f0fff     	retab
    14bc: 4e10035a     	tbl.16b	v26, { v26 }, v16
    14c0: 4e010df8     	dup.16b	v24, w15
    14c4: 6e208f16     	cmeq.16b	v22, v24, v0
    14c8: 4e010cf8     	dup.16b	v24, w7
    14cc: 6e611f16     	bsl.16b	v22, v24, v1
    14d0: eb0f00df     	cmp	x6, x15
    14d4: 9a8f8147     	csel	x7, x10, x15, hi
    14d8: eb0f00df     	cmp	x6, x15
    14dc: 9a9f87e0     	cset	x0, ls
    14e0: aa0703ef     	mov	x15, x7
    14e4: 4eb61ec1     	mov.16b	v1, v22
    14e8: 17ffffb4     	b	0x13b8 <_mech_control_range_0+0x88>
    14ec: d2800026     	mov	x6, #0x1                ; =1
    14f0: 52800005     	mov	w5, #0x0                ; =0
    14f4: aa0503e0     	mov	x0, x5
    14f8: 4ea51cb0     	mov.16b	v16, v5
    14fc: 53001c07     	uxtb	w7, w0
    1500: 710000ff     	cmp	w7, #0x0
    1504: 54000de0     	b.eq	0x16c0 <_mech_control_range_0+0x390>
    1508: 91000960     	add	x0, x11, #0x2
    150c: eb01001f     	cmp	x0, x1
    1510: 54000063     	b.lo	0x151c <_mech_control_range_0+0x1ec>
    1514: 4eb01e1a     	mov.16b	v26, v16
    1518: 14000006     	b	0x1530 <_mech_control_range_0+0x200>
    151c: d2800029     	mov	x9, #0x1                ; =1
    1520: 4eb01e1a     	mov.16b	v26, v16
    1524: 53001ca0     	uxtb	w0, w5
    1528: 7100001f     	cmp	w0, #0x0
    152c: 54000620     	b.eq	0x15f0 <_mech_control_range_0+0x2c0>
    1530: 0e013ca8     	umov.b	w8, v5[0]
    1534: 0e013e07     	umov.b	w7, v16[0]
    1538: d2800005     	mov	x5, #0x0                ; =0
    153c: aa0503e6     	mov	x6, x5
    1540: f100011f     	cmp	x8, #0x0
    1544: 9a9f07e0     	cset	x0, ne
    1548: f10000ff     	cmp	x7, #0x0
    154c: 9a9f07e9     	cset	x9, ne
    1550: 2a090000     	orr	w0, w0, w9
    1554: 53001c00     	uxtb	w0, w0
    1558: b5000140     	cbnz	x0, 0x1580 <_mech_control_range_0+0x250>
    155c: 9100096b     	add	x11, x11, #0x2
    1560: 8b050080     	add	x0, x4, x5
    1564: cb060004     	sub	x4, x0, x6
    1568: eb0600bf     	cmp	x5, x6
    156c: 9a8680a0     	csel	x0, x5, x6, hi
    1570: eb00005f     	cmp	x2, x0
    1574: 9a808042     	csel	x2, x2, x0, hi
    1578: 4eba1f45     	mov.16b	v5, v26
    157c: 17ffff7d     	b	0x1370 <_mech_control_range_0+0x40>
    1580: 90000009     	adrp	x9, 0x1000 <_mech_control_reduce+0x1000>
    1584: f9400129     	ldr	x9, [x9]
    1588: d2800020     	mov	x0, #0x1                ; =1
    158c: 9100050a     	add	x10, x8, #0x1
    1590: 3cea7922     	ldr	q2, [x9, x10, lsl #4]
    1594: 90000009     	adrp	x9, 0x1000 <_mech_control_reduce+0x1000>
    1598: f9400129     	ldr	x9, [x9]
    159c: 910004ea     	add	x10, x7, #0x1
    15a0: 3cea7923     	ldr	q3, [x9, x10, lsl #4]
    15a4: 910083e9     	add	x9, sp, #0x20
    15a8: 3c8203e5     	stur	q5, [sp, #0x20]
    15ac: 38686929     	ldrb	w9, [x9, x8]
    15b0: 9100c3ea     	add	x10, sp, #0x30
    15b4: 3c8303f0     	stur	q16, [sp, #0x30]
    15b8: 4e0200a5     	tbl.16b	v5, { v5 }, v2
    15bc: 4e030210     	tbl.16b	v16, { v16 }, v3
    15c0: 53001d29     	uxtb	w9, w9
    15c4: 3867694a     	ldrb	w10, [x10, x7]
    15c8: d280000c     	mov	x12, #0x0               ; =0
    15cc: eb1f011f     	cmp	x8, xzr
    15d0: 9a8c1008     	csel	x8, x0, x12, ne
    15d4: 8b0800a5     	add	x5, x5, x8
    15d8: eb1f00ff     	cmp	x7, xzr
    15dc: 9a8c1000     	csel	x0, x0, x12, ne
    15e0: 8b0000c6     	add	x6, x6, x0
    15e4: aa0a03e7     	mov	x7, x10
    15e8: aa0903e8     	mov	x8, x9
    15ec: 17ffffd5     	b	0x1540 <_mech_control_range_0+0x210>
    15f0: 91000525     	add	x5, x9, #0x1
    15f4: f10040bf     	cmp	x5, #0x10
    15f8: 9a9f87e6     	cset	x6, ls
    15fc: f10040bf     	cmp	x5, #0x10
    1600: 540000a9     	b.ls	0x1614 <_mech_control_range_0+0x2e4>
    1604: d2800080     	mov	x0, #0x4                ; =4
    1608: 910143ff     	add	sp, sp, #0x50
    160c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1610: d65f0fff     	retab
    1614: 90000007     	adrp	x7, 0x1000 <_mech_control_reduce+0x1000>
    1618: f94000e7     	ldr	x7, [x7]
    161c: d2800080     	mov	x0, #0x4                ; =4
    1620: 3ce578e2     	ldr	q2, [x7, x5, lsl #4]
    1624: 92800007     	mov	x7, #-0x1               ; =-1
    1628: eb07013f     	cmp	x9, x7
    162c: 9a9f07e7     	cset	x7, ne
    1630: 0a0600e6     	and	w6, w7, w6
    1634: 53001cc7     	uxtb	w7, w6
    1638: b5000087     	cbnz	x7, 0x1648 <_mech_control_range_0+0x318>
    163c: 910143ff     	add	sp, sp, #0x50
    1640: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1644: d65f0fff     	retab
    1648: 910043e7     	add	x7, sp, #0x10
    164c: 3c8103e1     	stur	q1, [sp, #0x10]
    1650: 386968e7     	ldrb	w7, [x7, x9]
    1654: 53001cc6     	uxtb	w6, w6
    1658: b5000086     	cbnz	x6, 0x1668 <_mech_control_range_0+0x338>
    165c: 910143ff     	add	sp, sp, #0x50
    1660: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1664: d65f0fff     	retab
    1668: 53001ce6     	uxtb	w6, w7
    166c: 910004c6     	add	x6, x6, #0x1
    1670: d2800007     	mov	x7, #0x0                ; =0
    1674: eb0900df     	cmp	x6, x9
    1678: 9a8680e7     	csel	x7, x7, x6, hi
    167c: f103fcff     	cmp	x7, #0xff
    1680: 54000089     	b.ls	0x1690 <_mech_control_range_0+0x360>
    1684: 910143ff     	add	sp, sp, #0x50
    1688: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    168c: d65f0fff     	retab
    1690: 4e02035a     	tbl.16b	v26, { v26 }, v2
    1694: 4e010d22     	dup.16b	v2, w9
    1698: 6e208c42     	cmeq.16b	v2, v2, v0
    169c: 4e010ce3     	dup.16b	v3, w7
    16a0: 6e611c62     	bsl.16b	v2, v3, v1
    16a4: eb0900df     	cmp	x6, x9
    16a8: 9a8980a7     	csel	x7, x5, x9, hi
    16ac: eb0900df     	cmp	x6, x9
    16b0: 9a9f87e5     	cset	x5, ls
    16b4: aa0703e9     	mov	x9, x7
    16b8: 4ea21c41     	mov.16b	v1, v2
    16bc: 17ffff9a     	b	0x1524 <_mech_control_range_0+0x1f4>
    16c0: 910004c7     	add	x7, x6, #0x1
    16c4: f10040ff     	cmp	x7, #0x10
    16c8: 9a9f87e8     	cset	x8, ls
    16cc: f10040ff     	cmp	x7, #0x10
    16d0: 540000a9     	b.ls	0x16e4 <_mech_control_range_0+0x3b4>
    16d4: d2800080     	mov	x0, #0x4                ; =4
    16d8: 910143ff     	add	sp, sp, #0x50
    16dc: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    16e0: d65f0fff     	retab
    16e4: 90000009     	adrp	x9, 0x1000 <_mech_control_reduce+0x1000>
    16e8: f9400129     	ldr	x9, [x9]
    16ec: d2800080     	mov	x0, #0x4                ; =4
    16f0: 3ce77931     	ldr	q17, [x9, x7, lsl #4]
    16f4: 92800009     	mov	x9, #-0x1               ; =-1
    16f8: eb0900df     	cmp	x6, x9
    16fc: 9a9f07ea     	cset	x10, ne
    1700: 0a080149     	and	w9, w10, w8
    1704: 53001d2a     	uxtb	w10, w9
    1708: b500008a     	cbnz	x10, 0x1718 <_mech_control_range_0+0x3e8>
    170c: 910143ff     	add	sp, sp, #0x50
    1710: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1714: d65f0fff     	retab
    1718: 910003ec     	mov	x12, sp
    171c: 3c8003e1     	stur	q1, [sp]
    1720: 3866698c     	ldrb	w12, [x12, x6]
    1724: 53001d2d     	uxtb	w13, w9
    1728: b500008d     	cbnz	x13, 0x1738 <_mech_control_range_0+0x408>
    172c: 910143ff     	add	sp, sp, #0x50
    1730: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1734: d65f0fff     	retab
    1738: 53001d8e     	uxtb	w14, w12
    173c: 910005ce     	add	x14, x14, #0x1
    1740: d280000f     	mov	x15, #0x0               ; =0
    1744: eb0601df     	cmp	x14, x6
    1748: 9a8e81ef     	csel	x15, x15, x14, hi
    174c: f103fdff     	cmp	x15, #0xff
    1750: 54000089     	b.ls	0x1760 <_mech_control_range_0+0x430>
    1754: 910143ff     	add	sp, sp, #0x50
    1758: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    175c: d65f0fff     	retab
    1760: 4e110210     	tbl.16b	v16, { v16 }, v17
    1764: 4e010cd2     	dup.16b	v18, w6
    1768: 6e208e51     	cmeq.16b	v17, v18, v0
    176c: 4e010df2     	dup.16b	v18, w15
    1770: 6e611e51     	bsl.16b	v17, v18, v1
    1774: eb0601df     	cmp	x14, x6
    1778: 9a8680e7     	csel	x7, x7, x6, hi
    177c: eb0601df     	cmp	x14, x6
    1780: 9a9f87e0     	cset	x0, ls
    1784: aa0703e6     	mov	x6, x7
    1788: 4eb11e21     	mov.16b	v1, v17
    178c: 17ffff5c     	b	0x14fc <_mech_control_range_0+0x1cc>
    1790: aa0503eb     	mov	x11, x5
    1794: f100505f     	cmp	x2, #0x14
    1798: 540000a9     	b.ls	0x17ac <_mech_control_range_0+0x47c>
    179c: d2800060     	mov	x0, #0x3                ; =3
    17a0: 910143ff     	add	sp, sp, #0x50
    17a4: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    17a8: d65f0fff     	retab
    17ac: d2800020     	mov	x0, #0x1                ; =1
    17b0: aa0003e5     	mov	x5, x0
    17b4: eb02001f     	cmp	x0, x2
    17b8: 540007a9     	b.ls	0x18ac <_mech_control_range_0+0x57c>
    17bc: f10000bf     	cmp	x5, #0x0
    17c0: 540000a1     	b.ne	0x17d4 <_mech_control_range_0+0x4a4>
    17c4: d2800040     	mov	x0, #0x2                ; =2
    17c8: 910143ff     	add	sp, sp, #0x50
    17cc: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    17d0: d65f0fff     	retab
    17d4: b4000745     	cbz	x5, 0x18bc <_mech_control_range_0+0x58c>
    17d8: 9ac50886     	udiv	x6, x4, x5
    17dc: 91000447     	add	x7, x2, #0x1
    17e0: f10040ff     	cmp	x7, #0x10
    17e4: 9a9f87e8     	cset	x8, ls
    17e8: f10040ff     	cmp	x7, #0x10
    17ec: 540000a9     	b.ls	0x1800 <_mech_control_range_0+0x4d0>
    17f0: d2800080     	mov	x0, #0x4                ; =4
    17f4: 910143ff     	add	sp, sp, #0x50
    17f8: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    17fc: d65f0fff     	retab
    1800: d2800020     	mov	x0, #0x1                ; =1
    1804: eb1f00ff     	cmp	x7, xzr
    1808: 9a8010e0     	csel	x0, x7, x0, ne
    180c: b40005a0     	cbz	x0, 0x18c0 <_mech_control_range_0+0x590>
    1810: 9ac008c9     	udiv	x9, x6, x0
    1814: 9b009929     	msub	x9, x9, x0, x6
    1818: 9000000a     	adrp	x10, 0x1000 <_mech_control_reduce+0x1000>
    181c: f940014a     	ldr	x10, [x10]
    1820: d2800080     	mov	x0, #0x4                ; =4
    1824: 8b071127     	add	x7, x9, x7, lsl #4
    1828: 3ce77942     	ldr	q2, [x10, x7, lsl #4]
    182c: 92800007     	mov	x7, #-0x1               ; =-1
    1830: eb07005f     	cmp	x2, x7
    1834: 9a9f07e7     	cset	x7, ne
    1838: 0a0800e7     	and	w7, w7, w8
    183c: 53001ce7     	uxtb	w7, w7
    1840: b5000087     	cbnz	x7, 0x1850 <_mech_control_range_0+0x520>
    1844: 910143ff     	add	sp, sp, #0x50
    1848: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    184c: d65f0fff     	retab
    1850: f103fcdf     	cmp	x6, #0xff
    1854: 54000089     	b.ls	0x1864 <_mech_control_range_0+0x534>
    1858: 910143ff     	add	sp, sp, #0x50
    185c: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1860: d65f0fff     	retab
    1864: f10000bf     	cmp	x5, #0x0
    1868: 540000a1     	b.ne	0x187c <_mech_control_range_0+0x54c>
    186c: d2800040     	mov	x0, #0x2                ; =2
    1870: 910143ff     	add	sp, sp, #0x50
    1874: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1878: d65f0fff     	retab
    187c: b4000245     	cbz	x5, 0x18c4 <_mech_control_range_0+0x594>
    1880: 9ac50880     	udiv	x0, x4, x5
    1884: 9b059004     	msub	x4, x0, x5, x4
    1888: 4e0200a5     	tbl.16b	v5, { v5 }, v2
    188c: 4e010c42     	dup.16b	v2, w2
    1890: 6e208c42     	cmeq.16b	v2, v2, v0
    1894: 4e010cc3     	dup.16b	v3, w6
    1898: 6e611c62     	bsl.16b	v2, v3, v1
    189c: d1000442     	sub	x2, x2, #0x1
    18a0: aa0b03e5     	mov	x5, x11
    18a4: 4ea21c41     	mov.16b	v1, v2
    18a8: 17fffead     	b	0x135c <_mech_control_range_0+0x2c>
    18ac: 91000406     	add	x6, x0, #0x1
    18b0: 9b007ca5     	mul	x5, x5, x0
    18b4: aa0603e0     	mov	x0, x6
    18b8: 17ffffbf     	b	0x17b4 <_mech_control_range_0+0x484>
    18bc: 0000c11f     	udf	#0xc11f
    18c0: 0000c11f     	udf	#0xc11f
    18c4: 0000c11f     	udf	#0xc11f
		...
    18d0: 03020100     	<unknown>
    18d4: 07060504     	<unknown>
    18d8: 0b0a0908     	add	w8, w8, w10, lsl #2
    18dc: 0f0e0d0c     	<unknown>

00000000000018e0 <_mech_control_entry>:
    18e0: d503237f     	pacibsp
    18e4: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
    18e8: 910003fd     	mov	x29, sp
    18ec: a9bf53f3     	stp	x19, x20, [sp, #-0x10]!
    18f0: d100c3ff     	sub	sp, sp, #0x30
    18f4: aa0103f4     	mov	x20, x1
    18f8: f9400013     	ldr	x19, [x0]
    18fc: f100027f     	cmp	x19, #0x0
    1900: 9a9f07e0     	cset	x0, ne
    1904: f100427f     	cmp	x19, #0x10
    1908: 9a9f87e1     	cset	x1, ls
    190c: 0a010000     	and	w0, w0, w1
    1910: 53001c00     	uxtb	w0, w0
    1914: b50000c0     	cbnz	x0, 0x192c <_mech_control_entry+0x4c>
    1918: d2800020     	mov	x0, #0x1                ; =1
    191c: 9100c3ff     	add	sp, sp, #0x30
    1920: a8c153f3     	ldp	x19, x20, [sp], #0x10
    1924: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1928: d65f0fff     	retab
    192c: f100527f     	cmp	x19, #0x14
    1930: 540000c9     	b.ls	0x1948 <_mech_control_entry+0x68>
    1934: d2800060     	mov	x0, #0x3                ; =3
    1938: 9100c3ff     	add	sp, sp, #0x30
    193c: a8c153f3     	ldp	x19, x20, [sp], #0x10
    1940: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1944: d65f0fff     	retab
    1948: d2800024     	mov	x4, #0x1                ; =1
    194c: aa0403e1     	mov	x1, x4
    1950: aa0403e2     	mov	x2, x4
    1954: eb13005f     	cmp	x2, x19
    1958: 54000489     	b.ls	0x19e8 <_mech_control_entry+0x108>
    195c: 910043e3     	add	x3, sp, #0x10
    1960: f80103f3     	stur	x19, [sp, #0x10]
    1964: d2800000     	mov	x0, #0x0                ; =0
    1968: d2800047     	mov	x7, #0x2                ; =2
    196c: d347fc22     	lsr	x2, x1, #7
    1970: 927ff842     	and	x2, x2, #0xfffffffffffffffe
    1974: eb0200ff     	cmp	x7, x2
    1978: 9a8280e2     	csel	x2, x7, x2, hi
    197c: 90000005     	adrp	x5, 0x1000 <_mech_control_reduce+0x1000>
    1980: f94000a5     	ldr	x5, [x5]
    1984: 90000006     	adrp	x6, 0x1000 <_mech_control_reduce+0x1000>
    1988: f94000c6     	ldr	x6, [x6]
    198c: 910083e8     	add	x8, sp, #0x20
    1990: f80003e8     	stur	x8, [sp]
    1994: 90000008     	adrp	x8, 0x1000 <_mech_control_reduce+0x1000>
    1998: f9400108     	ldr	x8, [x8]
    199c: d63f0100     	blr	x8
    19a0: f100001f     	cmp	x0, #0x0
    19a4: 540000c0     	b.eq	0x19bc <_mech_control_entry+0xdc>
    19a8: d28000a0     	mov	x0, #0x5                ; =5
    19ac: 9100c3ff     	add	sp, sp, #0x30
    19b0: a8c153f3     	ldp	x19, x20, [sp], #0x10
    19b4: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    19b8: d65f0fff     	retab
    19bc: f84203e1     	ldur	x1, [sp, #0x20]
    19c0: f84283e0     	ldur	x0, [sp, #0x28]
    19c4: aa1403e6     	mov	x6, x20
    19c8: f90000c1     	str	x1, [x6]
    19cc: f90004d3     	str	x19, [x6, #0x8]
    19d0: f90008c0     	str	x0, [x6, #0x10]
    19d4: d2800000     	mov	x0, #0x0                ; =0
    19d8: 9100c3ff     	add	sp, sp, #0x30
    19dc: a8c153f3     	ldp	x19, x20, [sp], #0x10
    19e0: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    19e4: d65f0fff     	retab
    19e8: aa1403e6     	mov	x6, x20
    19ec: 91000440     	add	x0, x2, #0x1
    19f0: 9b027c21     	mul	x1, x1, x2
    19f4: aa0003e2     	mov	x2, x0
    19f8: 17ffffd7     	b	0x1954 <_mech_control_entry+0x74>
