
benchmarks/clbg/fannkuchredux/.build/mech-fannkuch-cached.project/mech-control.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000001330 <_mech_control_range_0>:
    1330: aa0003e5     	mov	x5, x0
    1334: f9400040     	ldr	x0, [x2]
    1338: 9c0027c0     	ldr	q0, 0x1830 <_mech_control_range_0+0x500>
    133c: 4f00e401     	movi.16b	v1, #0x0
    1340: d1000402     	sub	x2, x0, #0x1
    1344: aa0503e4     	mov	x4, x5
    1348: 4ea01c05     	mov.16b	v5, v0
    134c: f100005f     	cmp	x2, #0x0
    1350: 54001e61     	b.ne	0x171c <_mech_control_range_0+0x3ec>
    1354: d2800004     	mov	x4, #0x0                ; =0
    1358: aa0403e2     	mov	x2, x4
    135c: aa0503e9     	mov	x9, x5
    1360: eb01013f     	cmp	x9, x1
    1364: 540000a3     	b.lo	0x1378 <_mech_control_range_0+0x48>
    1368: f9000064     	str	x4, [x3]
    136c: f9000462     	str	x2, [x3, #0x8]
    1370: d2800000     	mov	x0, #0x0                ; =0
    1374: d65f03c0     	ret
    1378: 91000525     	add	x5, x9, #0x1
    137c: eb0100bf     	cmp	x5, x1
    1380: 54000963     	b.lo	0x14ac <_mech_control_range_0+0x17c>
    1384: eb0100bf     	cmp	x5, x1
    1388: 54000063     	b.lo	0x1394 <_mech_control_range_0+0x64>
    138c: 4ea51cba     	mov.16b	v26, v5
    1390: 14000007     	b	0x13ac <_mech_control_range_0+0x7c>
    1394: d280002f     	mov	x15, #0x1               ; =1
    1398: 52800000     	mov	w0, #0x0                ; =0
    139c: 4ea51cba     	mov.16b	v26, v5
    13a0: 53001c06     	uxtb	w6, w0
    13a4: 710000df     	cmp	w6, #0x0
    13a8: 540002e0     	b.eq	0x1404 <_mech_control_range_0+0xd4>
    13ac: d2800000     	mov	x0, #0x0                ; =0
    13b0: 0e013ca6     	umov.b	w6, v5[0]
    13b4: 53001cc6     	uxtb	w6, w6
    13b8: 710000df     	cmp	w6, #0x0
    13bc: 54000141     	b.ne	0x13e4 <_mech_control_range_0+0xb4>
    13c0: 92400128     	and	x8, x9, #0x1
    13c4: 8b000089     	add	x9, x4, x0
    13c8: cb00008a     	sub	x10, x4, x0
    13cc: f100011f     	cmp	x8, #0x0
    13d0: 9a8a0124     	csel	x4, x9, x10, eq
    13d4: eb00005f     	cmp	x2, x0
    13d8: 9a808042     	csel	x2, x2, x0, hi
    13dc: aa0503e9     	mov	x9, x5
    13e0: 1400005c     	b	0x1550 <_mech_control_range_0+0x220>
    13e4: 9000000b     	adrp	x11, 0x1000 <_mech_control_reduce+0x1000>
    13e8: f940016b     	ldr	x11, [x11]
    13ec: 0e013cac     	umov.b	w12, v5[0]
    13f0: 9100058c     	add	x12, x12, #0x1
    13f4: 3cec7970     	ldr	q16, [x11, x12, lsl #4]
    13f8: 4e1000a5     	tbl.16b	v5, { v5 }, v16
    13fc: 91000400     	add	x0, x0, #0x1
    1400: 17ffffec     	b	0x13b0 <_mech_control_range_0+0x80>
    1404: 910005ec     	add	x12, x15, #0x1
    1408: f100419f     	cmp	x12, #0x10
    140c: 9a9f87ed     	cset	x13, ls
    1410: f100419f     	cmp	x12, #0x10
    1414: 54000069     	b.ls	0x1420 <_mech_control_range_0+0xf0>
    1418: d2800080     	mov	x0, #0x4                ; =4
    141c: d65f03c0     	ret
    1420: 90000006     	adrp	x6, 0x1000 <_mech_control_reduce+0x1000>
    1424: f94000c6     	ldr	x6, [x6]
    1428: d2800080     	mov	x0, #0x4                ; =4
    142c: 3cec78d0     	ldr	q16, [x6, x12, lsl #4]
    1430: 92800006     	mov	x6, #-0x1               ; =-1
    1434: eb0601ff     	cmp	x15, x6
    1438: 9a9f07e6     	cset	x6, ne
    143c: 0a0d00c6     	and	w6, w6, w13
    1440: 53001cc7     	uxtb	w7, w6
    1444: b5000047     	cbnz	x7, 0x144c <_mech_control_range_0+0x11c>
    1448: d65f03c0     	ret
    144c: 53001cc6     	uxtb	w6, w6
    1450: b5000046     	cbnz	x6, 0x1458 <_mech_control_range_0+0x128>
    1454: d65f03c0     	ret
    1458: 4e010df5     	dup.16b	v21, w15
    145c: 4e150036     	tbl.16b	v22, { v1 }, v21
    1460: 0e013ec6     	umov.b	w6, v22[0]
    1464: 910004c6     	add	x6, x6, #0x1
    1468: d2800007     	mov	x7, #0x0                ; =0
    146c: eb0f00df     	cmp	x6, x15
    1470: 9a8680e7     	csel	x7, x7, x6, hi
    1474: f103fcff     	cmp	x7, #0xff
    1478: 54000049     	b.ls	0x1480 <_mech_control_range_0+0x150>
    147c: d65f03c0     	ret
    1480: 4e10035a     	tbl.16b	v26, { v26 }, v16
    1484: 6e208eb7     	cmeq.16b	v23, v21, v0
    1488: 4e010cf9     	dup.16b	v25, w7
    148c: 6e611f37     	bsl.16b	v23, v25, v1
    1490: eb0f00df     	cmp	x6, x15
    1494: 9a8f8187     	csel	x7, x12, x15, hi
    1498: eb0f00df     	cmp	x6, x15
    149c: 9a9f87e0     	cset	x0, ls
    14a0: aa0703ef     	mov	x15, x7
    14a4: 4eb71ee1     	mov.16b	v1, v23
    14a8: 17ffffbe     	b	0x13a0 <_mech_control_range_0+0x70>
    14ac: d2800026     	mov	x6, #0x1                ; =1
    14b0: 52800005     	mov	w5, #0x0                ; =0
    14b4: aa0503e0     	mov	x0, x5
    14b8: 4ea51cb0     	mov.16b	v16, v5
    14bc: 53001c07     	uxtb	w7, w0
    14c0: 710000ff     	cmp	w7, #0x0
    14c4: 54000d80     	b.eq	0x1674 <_mech_control_range_0+0x344>
    14c8: 91000920     	add	x0, x9, #0x2
    14cc: eb01001f     	cmp	x0, x1
    14d0: 54000063     	b.lo	0x14dc <_mech_control_range_0+0x1ac>
    14d4: 4eb01e1a     	mov.16b	v26, v16
    14d8: 14000006     	b	0x14f0 <_mech_control_range_0+0x1c0>
    14dc: d2800027     	mov	x7, #0x1                ; =1
    14e0: 4eb01e1a     	mov.16b	v26, v16
    14e4: 53001ca0     	uxtb	w0, w5
    14e8: 7100001f     	cmp	w0, #0x0
    14ec: 54000700     	b.eq	0x15cc <_mech_control_range_0+0x29c>
    14f0: 0e013ca8     	umov.b	w8, v5[0]
    14f4: 0e013e07     	umov.b	w7, v16[0]
    14f8: d2800005     	mov	x5, #0x0                ; =0
    14fc: aa0503e6     	mov	x6, x5
    1500: f100011f     	cmp	x8, #0x0
    1504: 9a9f07e0     	cset	x0, ne
    1508: f10000ff     	cmp	x7, #0x0
    150c: 9a9f07ea     	cset	x10, ne
    1510: 2a0a0000     	orr	w0, w0, w10
    1514: 53001c00     	uxtb	w0, w0
    1518: b5000200     	cbnz	x0, 0x1558 <_mech_control_range_0+0x228>
    151c: 91000920     	add	x0, x9, #0x2
    1520: 92400127     	and	x7, x9, #0x1
    1524: 8b050088     	add	x8, x4, x5
    1528: cb060108     	sub	x8, x8, x6
    152c: cb050084     	sub	x4, x4, x5
    1530: 8b060084     	add	x4, x4, x6
    1534: f10000ff     	cmp	x7, #0x0
    1538: 9a840104     	csel	x4, x8, x4, eq
    153c: eb0600bf     	cmp	x5, x6
    1540: 9a8680a5     	csel	x5, x5, x6, hi
    1544: eb05005f     	cmp	x2, x5
    1548: 9a858042     	csel	x2, x2, x5, hi
    154c: aa0003e9     	mov	x9, x0
    1550: 4eba1f45     	mov.16b	v5, v26
    1554: 17ffff83     	b	0x1360 <_mech_control_range_0+0x30>
    1558: 9000000a     	adrp	x10, 0x1000 <_mech_control_reduce+0x1000>
    155c: f940014a     	ldr	x10, [x10]
    1560: d2800020     	mov	x0, #0x1                ; =1
    1564: 9100050b     	add	x11, x8, #0x1
    1568: 3ceb7942     	ldr	q2, [x10, x11, lsl #4]
    156c: 9000000a     	adrp	x10, 0x1000 <_mech_control_reduce+0x1000>
    1570: f940014a     	ldr	x10, [x10]
    1574: 910004eb     	add	x11, x7, #0x1
    1578: 3ceb7943     	ldr	q3, [x10, x11, lsl #4]
    157c: 4e0200a2     	tbl.16b	v2, { v5 }, v2
    1580: 4e030203     	tbl.16b	v3, { v16 }, v3
    1584: 4e010d04     	dup.16b	v4, w8
    1588: 4e0400a4     	tbl.16b	v4, { v5 }, v4
    158c: 0e013c8a     	umov.b	w10, v4[0]
    1590: 4e010ce4     	dup.16b	v4, w7
    1594: 4e040204     	tbl.16b	v4, { v16 }, v4
    1598: 0e013c8b     	umov.b	w11, v4[0]
    159c: d280000c     	mov	x12, #0x0               ; =0
    15a0: eb1f011f     	cmp	x8, xzr
    15a4: 9a8c1008     	csel	x8, x0, x12, ne
    15a8: 8b0800a5     	add	x5, x5, x8
    15ac: eb1f00ff     	cmp	x7, xzr
    15b0: 9a8c1000     	csel	x0, x0, x12, ne
    15b4: 8b0000c6     	add	x6, x6, x0
    15b8: aa0b03e7     	mov	x7, x11
    15bc: aa0a03e8     	mov	x8, x10
    15c0: 4ea21c45     	mov.16b	v5, v2
    15c4: 4ea31c70     	mov.16b	v16, v3
    15c8: 17ffffce     	b	0x1500 <_mech_control_range_0+0x1d0>
    15cc: 910004e5     	add	x5, x7, #0x1
    15d0: f10040bf     	cmp	x5, #0x10
    15d4: 9a9f87e6     	cset	x6, ls
    15d8: f10040bf     	cmp	x5, #0x10
    15dc: 54000069     	b.ls	0x15e8 <_mech_control_range_0+0x2b8>
    15e0: d2800080     	mov	x0, #0x4                ; =4
    15e4: d65f03c0     	ret
    15e8: 90000008     	adrp	x8, 0x1000 <_mech_control_reduce+0x1000>
    15ec: f9400108     	ldr	x8, [x8]
    15f0: d2800080     	mov	x0, #0x4                ; =4
    15f4: 3ce57902     	ldr	q2, [x8, x5, lsl #4]
    15f8: 92800008     	mov	x8, #-0x1               ; =-1
    15fc: eb0800ff     	cmp	x7, x8
    1600: 9a9f07e8     	cset	x8, ne
    1604: 0a060106     	and	w6, w8, w6
    1608: 53001cc8     	uxtb	w8, w6
    160c: b5000048     	cbnz	x8, 0x1614 <_mech_control_range_0+0x2e4>
    1610: d65f03c0     	ret
    1614: 53001cc6     	uxtb	w6, w6
    1618: b5000046     	cbnz	x6, 0x1620 <_mech_control_range_0+0x2f0>
    161c: d65f03c0     	ret
    1620: 4e010ce3     	dup.16b	v3, w7
    1624: 4e030024     	tbl.16b	v4, { v1 }, v3
    1628: 0e013c86     	umov.b	w6, v4[0]
    162c: 910004c6     	add	x6, x6, #0x1
    1630: d2800008     	mov	x8, #0x0                ; =0
    1634: eb0700df     	cmp	x6, x7
    1638: 9a868108     	csel	x8, x8, x6, hi
    163c: f103fd1f     	cmp	x8, #0xff
    1640: 54000049     	b.ls	0x1648 <_mech_control_range_0+0x318>
    1644: d65f03c0     	ret
    1648: 4e02035a     	tbl.16b	v26, { v26 }, v2
    164c: 6e208c62     	cmeq.16b	v2, v3, v0
    1650: 4e010d03     	dup.16b	v3, w8
    1654: 6e611c62     	bsl.16b	v2, v3, v1
    1658: eb0700df     	cmp	x6, x7
    165c: 9a8780a8     	csel	x8, x5, x7, hi
    1660: eb0700df     	cmp	x6, x7
    1664: 9a9f87e5     	cset	x5, ls
    1668: aa0803e7     	mov	x7, x8
    166c: 4ea21c41     	mov.16b	v1, v2
    1670: 17ffff9d     	b	0x14e4 <_mech_control_range_0+0x1b4>
    1674: 910004c7     	add	x7, x6, #0x1
    1678: f10040ff     	cmp	x7, #0x10
    167c: 9a9f87e8     	cset	x8, ls
    1680: f10040ff     	cmp	x7, #0x10
    1684: 54000069     	b.ls	0x1690 <_mech_control_range_0+0x360>
    1688: d2800080     	mov	x0, #0x4                ; =4
    168c: d65f03c0     	ret
    1690: 9000000a     	adrp	x10, 0x1000 <_mech_control_reduce+0x1000>
    1694: f940014a     	ldr	x10, [x10]
    1698: d2800080     	mov	x0, #0x4                ; =4
    169c: 3ce77947     	ldr	q7, [x10, x7, lsl #4]
    16a0: 9280000a     	mov	x10, #-0x1              ; =-1
    16a4: eb0a00df     	cmp	x6, x10
    16a8: 9a9f07ea     	cset	x10, ne
    16ac: 0a080148     	and	w8, w10, w8
    16b0: 53001d0a     	uxtb	w10, w8
    16b4: b500004a     	cbnz	x10, 0x16bc <_mech_control_range_0+0x38c>
    16b8: d65f03c0     	ret
    16bc: 53001d0a     	uxtb	w10, w8
    16c0: b500004a     	cbnz	x10, 0x16c8 <_mech_control_range_0+0x398>
    16c4: d65f03c0     	ret
    16c8: 4e010cd1     	dup.16b	v17, w6
    16cc: 4e110032     	tbl.16b	v18, { v1 }, v17
    16d0: 0e013e4c     	umov.b	w12, v18[0]
    16d4: 9100058c     	add	x12, x12, #0x1
    16d8: d280000d     	mov	x13, #0x0               ; =0
    16dc: eb06019f     	cmp	x12, x6
    16e0: 9a8c81ad     	csel	x13, x13, x12, hi
    16e4: f103fdbf     	cmp	x13, #0xff
    16e8: 54000049     	b.ls	0x16f0 <_mech_control_range_0+0x3c0>
    16ec: d65f03c0     	ret
    16f0: 4e070210     	tbl.16b	v16, { v16 }, v7
    16f4: 6e208e31     	cmeq.16b	v17, v17, v0
    16f8: 4e010db2     	dup.16b	v18, w13
    16fc: 6e611e51     	bsl.16b	v17, v18, v1
    1700: eb06019f     	cmp	x12, x6
    1704: 9a8680e7     	csel	x7, x7, x6, hi
    1708: eb06019f     	cmp	x12, x6
    170c: 9a9f87e0     	cset	x0, ls
    1710: aa0703e6     	mov	x6, x7
    1714: 4eb11e21     	mov.16b	v1, v17
    1718: 17ffff69     	b	0x14bc <_mech_control_range_0+0x18c>
    171c: aa0503e9     	mov	x9, x5
    1720: f100505f     	cmp	x2, #0x14
    1724: 54000069     	b.ls	0x1730 <_mech_control_range_0+0x400>
    1728: d2800060     	mov	x0, #0x3                ; =3
    172c: d65f03c0     	ret
    1730: d2800020     	mov	x0, #0x1                ; =1
    1734: aa0003e5     	mov	x5, x0
    1738: eb02001f     	cmp	x0, x2
    173c: 54000669     	b.ls	0x1808 <_mech_control_range_0+0x4d8>
    1740: f10000bf     	cmp	x5, #0x0
    1744: 54000061     	b.ne	0x1750 <_mech_control_range_0+0x420>
    1748: d2800040     	mov	x0, #0x2                ; =2
    174c: d65f03c0     	ret
    1750: b4000645     	cbz	x5, 0x1818 <_mech_control_range_0+0x4e8>
    1754: 9ac50886     	udiv	x6, x4, x5
    1758: 91000447     	add	x7, x2, #0x1
    175c: f10040ff     	cmp	x7, #0x10
    1760: 9a9f87e8     	cset	x8, ls
    1764: f10040ff     	cmp	x7, #0x10
    1768: 54000069     	b.ls	0x1774 <_mech_control_range_0+0x444>
    176c: d2800080     	mov	x0, #0x4                ; =4
    1770: d65f03c0     	ret
    1774: d2800020     	mov	x0, #0x1                ; =1
    1778: eb1f00ff     	cmp	x7, xzr
    177c: 9a8010e0     	csel	x0, x7, x0, ne
    1780: b40004e0     	cbz	x0, 0x181c <_mech_control_range_0+0x4ec>
    1784: 9ac008ca     	udiv	x10, x6, x0
    1788: 9b00994a     	msub	x10, x10, x0, x6
    178c: 9000000b     	adrp	x11, 0x1000 <_mech_control_reduce+0x1000>
    1790: f940016b     	ldr	x11, [x11]
    1794: d2800080     	mov	x0, #0x4                ; =4
    1798: 8b071147     	add	x7, x10, x7, lsl #4
    179c: 3ce77962     	ldr	q2, [x11, x7, lsl #4]
    17a0: 92800007     	mov	x7, #-0x1               ; =-1
    17a4: eb07005f     	cmp	x2, x7
    17a8: 9a9f07e7     	cset	x7, ne
    17ac: 0a0800e7     	and	w7, w7, w8
    17b0: 53001ce7     	uxtb	w7, w7
    17b4: b5000047     	cbnz	x7, 0x17bc <_mech_control_range_0+0x48c>
    17b8: d65f03c0     	ret
    17bc: f103fcdf     	cmp	x6, #0xff
    17c0: 54000049     	b.ls	0x17c8 <_mech_control_range_0+0x498>
    17c4: d65f03c0     	ret
    17c8: f10000bf     	cmp	x5, #0x0
    17cc: 54000061     	b.ne	0x17d8 <_mech_control_range_0+0x4a8>
    17d0: d2800040     	mov	x0, #0x2                ; =2
    17d4: d65f03c0     	ret
    17d8: b4000245     	cbz	x5, 0x1820 <_mech_control_range_0+0x4f0>
    17dc: 9ac50880     	udiv	x0, x4, x5
    17e0: 9b059004     	msub	x4, x0, x5, x4
    17e4: 4e0200a5     	tbl.16b	v5, { v5 }, v2
    17e8: 4e010c42     	dup.16b	v2, w2
    17ec: 6e208c42     	cmeq.16b	v2, v2, v0
    17f0: 4e010cc3     	dup.16b	v3, w6
    17f4: 6e611c62     	bsl.16b	v2, v3, v1
    17f8: d1000442     	sub	x2, x2, #0x1
    17fc: aa0903e5     	mov	x5, x9
    1800: 4ea21c41     	mov.16b	v1, v2
    1804: 17fffed2     	b	0x134c <_mech_control_range_0+0x1c>
    1808: 91000406     	add	x6, x0, #0x1
    180c: 9b007ca5     	mul	x5, x5, x0
    1810: aa0603e0     	mov	x0, x6
    1814: 17ffffc9     	b	0x1738 <_mech_control_range_0+0x408>
    1818: 0000c11f     	udf	#0xc11f
    181c: 0000c11f     	udf	#0xc11f
    1820: 0000c11f     	udf	#0xc11f
		...
    1830: 03020100     	<unknown>
    1834: 07060504     	<unknown>
    1838: 0b0a0908     	add	w8, w8, w10, lsl #2
    183c: 0f0e0d0c     	<unknown>

0000000000001840 <_mech_control_entry>:
    1840: d503237f     	pacibsp
    1844: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
    1848: 910003fd     	mov	x29, sp
    184c: a9bf53f3     	stp	x19, x20, [sp, #-0x10]!
    1850: d100c3ff     	sub	sp, sp, #0x30
    1854: aa0103f4     	mov	x20, x1
    1858: f9400013     	ldr	x19, [x0]
    185c: f100027f     	cmp	x19, #0x0
    1860: 9a9f07e0     	cset	x0, ne
    1864: f100427f     	cmp	x19, #0x10
    1868: 9a9f87e1     	cset	x1, ls
    186c: 0a010000     	and	w0, w0, w1
    1870: 53001c00     	uxtb	w0, w0
    1874: b50000c0     	cbnz	x0, 0x188c <_mech_control_entry+0x4c>
    1878: d2800020     	mov	x0, #0x1                ; =1
    187c: 9100c3ff     	add	sp, sp, #0x30
    1880: a8c153f3     	ldp	x19, x20, [sp], #0x10
    1884: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1888: d65f0fff     	retab
    188c: f100527f     	cmp	x19, #0x14
    1890: 540000c9     	b.ls	0x18a8 <_mech_control_entry+0x68>
    1894: d2800060     	mov	x0, #0x3                ; =3
    1898: 9100c3ff     	add	sp, sp, #0x30
    189c: a8c153f3     	ldp	x19, x20, [sp], #0x10
    18a0: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    18a4: d65f0fff     	retab
    18a8: d2800024     	mov	x4, #0x1                ; =1
    18ac: aa0403e1     	mov	x1, x4
    18b0: aa0403e2     	mov	x2, x4
    18b4: eb13005f     	cmp	x2, x19
    18b8: 54000489     	b.ls	0x1948 <_mech_control_entry+0x108>
    18bc: 910043e3     	add	x3, sp, #0x10
    18c0: f80103f3     	stur	x19, [sp, #0x10]
    18c4: d2800000     	mov	x0, #0x0                ; =0
    18c8: d2800047     	mov	x7, #0x2                ; =2
    18cc: d347fc22     	lsr	x2, x1, #7
    18d0: 927ff842     	and	x2, x2, #0xfffffffffffffffe
    18d4: eb0200ff     	cmp	x7, x2
    18d8: 9a8280e2     	csel	x2, x7, x2, hi
    18dc: 90000005     	adrp	x5, 0x1000 <_mech_control_reduce+0x1000>
    18e0: f94000a5     	ldr	x5, [x5]
    18e4: 90000006     	adrp	x6, 0x1000 <_mech_control_reduce+0x1000>
    18e8: f94000c6     	ldr	x6, [x6]
    18ec: 910083e8     	add	x8, sp, #0x20
    18f0: f80003e8     	stur	x8, [sp]
    18f4: 90000008     	adrp	x8, 0x1000 <_mech_control_reduce+0x1000>
    18f8: f9400108     	ldr	x8, [x8]
    18fc: d63f0100     	blr	x8
    1900: f100001f     	cmp	x0, #0x0
    1904: 540000c0     	b.eq	0x191c <_mech_control_entry+0xdc>
    1908: d28000a0     	mov	x0, #0x5                ; =5
    190c: 9100c3ff     	add	sp, sp, #0x30
    1910: a8c153f3     	ldp	x19, x20, [sp], #0x10
    1914: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1918: d65f0fff     	retab
    191c: f84203e1     	ldur	x1, [sp, #0x20]
    1920: f84283e0     	ldur	x0, [sp, #0x28]
    1924: aa1403e6     	mov	x6, x20
    1928: f90000c1     	str	x1, [x6]
    192c: f90004d3     	str	x19, [x6, #0x8]
    1930: f90008c0     	str	x0, [x6, #0x10]
    1934: d2800000     	mov	x0, #0x0                ; =0
    1938: 9100c3ff     	add	sp, sp, #0x30
    193c: a8c153f3     	ldp	x19, x20, [sp], #0x10
    1940: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1944: d65f0fff     	retab
    1948: aa1403e6     	mov	x6, x20
    194c: 91000440     	add	x0, x2, #0x1
    1950: 9b027c21     	mul	x1, x1, x2
    1954: aa0003e2     	mov	x2, x0
    1958: 17ffffd7     	b	0x18b4 <_mech_control_entry+0x74>
