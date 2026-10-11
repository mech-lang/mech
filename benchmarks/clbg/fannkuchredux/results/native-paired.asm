
benchmarks/clbg/fannkuchredux/.build/mech-fannkuch-paired.project/mech-control.o:	file format mach-o arm64

Disassembly of section __TEXT,__text:

0000000000001330 <_mech_control_range_0>:
    1330: aa0003e5     	mov	x5, x0
    1334: f9400040     	ldr	x0, [x2]
    1338: 9c0026c0     	ldr	q0, 0x1810 <_mech_control_range_0+0x4e0>
    133c: 4f00e401     	movi.16b	v1, #0x0
    1340: d1000402     	sub	x2, x0, #0x1
    1344: aa0503e4     	mov	x4, x5
    1348: 4ea01c02     	mov.16b	v2, v0
    134c: f100005f     	cmp	x2, #0x0
    1350: 54001da1     	b.ne	0x1704 <_mech_control_range_0+0x3d4>
    1354: d280000d     	mov	x13, #0x0               ; =0
    1358: aa0d03e2     	mov	x2, x13
    135c: aa0503e8     	mov	x8, x5
    1360: eb01011f     	cmp	x8, x1
    1364: 540000a3     	b.lo	0x1378 <_mech_control_range_0+0x48>
    1368: f900006d     	str	x13, [x3]
    136c: f9000462     	str	x2, [x3, #0x8]
    1370: d2800000     	mov	x0, #0x0                ; =0
    1374: d65f03c0     	ret
    1378: 91000504     	add	x4, x8, #0x1
    137c: eb01009f     	cmp	x4, x1
    1380: 54000963     	b.lo	0x14ac <_mech_control_range_0+0x17c>
    1384: eb01009f     	cmp	x4, x1
    1388: 54000063     	b.lo	0x1394 <_mech_control_range_0+0x64>
    138c: 4ea21c56     	mov.16b	v22, v2
    1390: 14000007     	b	0x13ac <_mech_control_range_0+0x7c>
    1394: d280002b     	mov	x11, #0x1               ; =1
    1398: 5280000e     	mov	w14, #0x0               ; =0
    139c: 4ea21c56     	mov.16b	v22, v2
    13a0: 53001dc0     	uxtb	w0, w14
    13a4: 7100001f     	cmp	w0, #0x0
    13a8: 540002e0     	b.eq	0x1404 <_mech_control_range_0+0xd4>
    13ac: d2800000     	mov	x0, #0x0                ; =0
    13b0: 0e013c45     	umov.b	w5, v2[0]
    13b4: 53001ca5     	uxtb	w5, w5
    13b8: 710000bf     	cmp	w5, #0x0
    13bc: 54000141     	b.ne	0x13e4 <_mech_control_range_0+0xb4>
    13c0: 92400105     	and	x5, x8, #0x1
    13c4: 8b0001a6     	add	x6, x13, x0
    13c8: cb0001a7     	sub	x7, x13, x0
    13cc: f10000bf     	cmp	x5, #0x0
    13d0: 9a8700cd     	csel	x13, x6, x7, eq
    13d4: eb00005f     	cmp	x2, x0
    13d8: 9a808042     	csel	x2, x2, x0, hi
    13dc: aa0403e8     	mov	x8, x4
    13e0: 1400005e     	b	0x1558 <_mech_control_range_0+0x228>
    13e4: 90000005     	adrp	x5, 0x1000 <_mech_control_reduce+0x1000>
    13e8: f94000a5     	ldr	x5, [x5]
    13ec: 0e013c46     	umov.b	w6, v2[0]
    13f0: 910004c6     	add	x6, x6, #0x1
    13f4: 3ce678a5     	ldr	q5, [x5, x6, lsl #4]
    13f8: 4e050042     	tbl.16b	v2, { v2 }, v5
    13fc: 91000400     	add	x0, x0, #0x1
    1400: 17ffffec     	b	0x13b0 <_mech_control_range_0+0x80>
    1404: 91000566     	add	x6, x11, #0x1
    1408: f10040df     	cmp	x6, #0x10
    140c: 9a9f87e7     	cset	x7, ls
    1410: f10040df     	cmp	x6, #0x10
    1414: 54000069     	b.ls	0x1420 <_mech_control_range_0+0xf0>
    1418: d2800080     	mov	x0, #0x4                ; =4
    141c: d65f03c0     	ret
    1420: 9000000a     	adrp	x10, 0x1000 <_mech_control_reduce+0x1000>
    1424: f940014a     	ldr	x10, [x10]
    1428: d2800080     	mov	x0, #0x4                ; =4
    142c: 3ce67950     	ldr	q16, [x10, x6, lsl #4]
    1430: 92800009     	mov	x9, #-0x1               ; =-1
    1434: eb09017f     	cmp	x11, x9
    1438: 9a9f07ec     	cset	x12, ne
    143c: 0a07018c     	and	w12, w12, w7
    1440: 53001d8e     	uxtb	w14, w12
    1444: b500004e     	cbnz	x14, 0x144c <_mech_control_range_0+0x11c>
    1448: d65f03c0     	ret
    144c: 53001d8e     	uxtb	w14, w12
    1450: b500004e     	cbnz	x14, 0x1458 <_mech_control_range_0+0x128>
    1454: d65f03c0     	ret
    1458: 4e010d71     	dup.16b	v17, w11
    145c: 4e110032     	tbl.16b	v18, { v1 }, v17
    1460: 0e013e4f     	umov.b	w15, v18[0]
    1464: 910005ef     	add	x15, x15, #0x1
    1468: d2800005     	mov	x5, #0x0                ; =0
    146c: eb0b01ff     	cmp	x15, x11
    1470: 9a8f80a5     	csel	x5, x5, x15, hi
    1474: f103fcbf     	cmp	x5, #0xff
    1478: 54000049     	b.ls	0x1480 <_mech_control_range_0+0x150>
    147c: d65f03c0     	ret
    1480: 4e1002d6     	tbl.16b	v22, { v22 }, v16
    1484: 6e208e31     	cmeq.16b	v17, v17, v0
    1488: 4e010cb3     	dup.16b	v19, w5
    148c: 6e611e71     	bsl.16b	v17, v19, v1
    1490: eb0b01ff     	cmp	x15, x11
    1494: 9a8b80c0     	csel	x0, x6, x11, hi
    1498: eb0b01ff     	cmp	x15, x11
    149c: 9a9f87ee     	cset	x14, ls
    14a0: aa0003eb     	mov	x11, x0
    14a4: 4eb11e21     	mov.16b	v1, v17
    14a8: 17ffffbe     	b	0x13a0 <_mech_control_range_0+0x70>
    14ac: d2800025     	mov	x5, #0x1                ; =1
    14b0: 52800004     	mov	w4, #0x0                ; =0
    14b4: aa0403e0     	mov	x0, x4
    14b8: 4ea21c50     	mov.16b	v16, v2
    14bc: 53001c06     	uxtb	w6, w0
    14c0: 710000df     	cmp	w6, #0x0
    14c4: 54000cc0     	b.eq	0x165c <_mech_control_range_0+0x32c>
    14c8: 91000900     	add	x0, x8, #0x2
    14cc: eb01001f     	cmp	x0, x1
    14d0: 54000063     	b.lo	0x14dc <_mech_control_range_0+0x1ac>
    14d4: 4eb01e16     	mov.16b	v22, v16
    14d8: 14000006     	b	0x14f0 <_mech_control_range_0+0x1c0>
    14dc: d2800027     	mov	x7, #0x1                ; =1
    14e0: 4eb01e16     	mov.16b	v22, v16
    14e4: 53001c80     	uxtb	w0, w4
    14e8: 7100001f     	cmp	w0, #0x0
    14ec: 54000640     	b.eq	0x15b4 <_mech_control_range_0+0x284>
    14f0: d2800004     	mov	x4, #0x0                ; =0
    14f4: aa0403e5     	mov	x5, x4
    14f8: 0e013c46     	umov.b	w6, v2[0]
    14fc: 53001cc0     	uxtb	w0, w6
    1500: 7100001f     	cmp	w0, #0x0
    1504: 9a9f07e0     	cset	x0, ne
    1508: 0e013e07     	umov.b	w7, v16[0]
    150c: 53001ce9     	uxtb	w9, w7
    1510: 7100013f     	cmp	w9, #0x0
    1514: 9a9f07e9     	cset	x9, ne
    1518: 2a090000     	orr	w0, w0, w9
    151c: 53001c00     	uxtb	w0, w0
    1520: b5000200     	cbnz	x0, 0x1560 <_mech_control_range_0+0x230>
    1524: 91000900     	add	x0, x8, #0x2
    1528: 92400106     	and	x6, x8, #0x1
    152c: 8b0401a7     	add	x7, x13, x4
    1530: cb0500e7     	sub	x7, x7, x5
    1534: cb0401a8     	sub	x8, x13, x4
    1538: 8b050108     	add	x8, x8, x5
    153c: f10000df     	cmp	x6, #0x0
    1540: 9a8800ed     	csel	x13, x7, x8, eq
    1544: eb05009f     	cmp	x4, x5
    1548: 9a858084     	csel	x4, x4, x5, hi
    154c: eb04005f     	cmp	x2, x4
    1550: 9a848042     	csel	x2, x2, x4, hi
    1554: aa0003e8     	mov	x8, x0
    1558: 4eb61ec2     	mov.16b	v2, v22
    155c: 17ffff81     	b	0x1360 <_mech_control_range_0+0x30>
    1560: 90000009     	adrp	x9, 0x1000 <_mech_control_reduce+0x1000>
    1564: f9400129     	ldr	x9, [x9]
    1568: 0e013c4a     	umov.b	w10, v2[0]
    156c: d2800020     	mov	x0, #0x1                ; =1
    1570: 9100054a     	add	x10, x10, #0x1
    1574: 3cea7923     	ldr	q3, [x9, x10, lsl #4]
    1578: 90000009     	adrp	x9, 0x1000 <_mech_control_reduce+0x1000>
    157c: f9400129     	ldr	x9, [x9]
    1580: 0e013e0a     	umov.b	w10, v16[0]
    1584: 9100054a     	add	x10, x10, #0x1
    1588: 3cea7924     	ldr	q4, [x9, x10, lsl #4]
    158c: 4e030042     	tbl.16b	v2, { v2 }, v3
    1590: 4e040210     	tbl.16b	v16, { v16 }, v4
    1594: d2800009     	mov	x9, #0x0                ; =0
    1598: 72001cdf     	tst	w6, #0xff
    159c: 9a891006     	csel	x6, x0, x9, ne
    15a0: 8b060084     	add	x4, x4, x6
    15a4: 72001cff     	tst	w7, #0xff
    15a8: 9a891000     	csel	x0, x0, x9, ne
    15ac: 8b0000a5     	add	x5, x5, x0
    15b0: 17ffffd2     	b	0x14f8 <_mech_control_range_0+0x1c8>
    15b4: 910004e4     	add	x4, x7, #0x1
    15b8: f100409f     	cmp	x4, #0x10
    15bc: 9a9f87e5     	cset	x5, ls
    15c0: f100409f     	cmp	x4, #0x10
    15c4: 54000069     	b.ls	0x15d0 <_mech_control_range_0+0x2a0>
    15c8: d2800080     	mov	x0, #0x4                ; =4
    15cc: d65f03c0     	ret
    15d0: 90000006     	adrp	x6, 0x1000 <_mech_control_reduce+0x1000>
    15d4: f94000c6     	ldr	x6, [x6]
    15d8: d2800080     	mov	x0, #0x4                ; =4
    15dc: 3ce478c3     	ldr	q3, [x6, x4, lsl #4]
    15e0: 92800006     	mov	x6, #-0x1               ; =-1
    15e4: eb0600ff     	cmp	x7, x6
    15e8: 9a9f07e6     	cset	x6, ne
    15ec: 0a0500c5     	and	w5, w6, w5
    15f0: 53001ca6     	uxtb	w6, w5
    15f4: b5000046     	cbnz	x6, 0x15fc <_mech_control_range_0+0x2cc>
    15f8: d65f03c0     	ret
    15fc: 53001ca5     	uxtb	w5, w5
    1600: b5000045     	cbnz	x5, 0x1608 <_mech_control_range_0+0x2d8>
    1604: d65f03c0     	ret
    1608: 4e010ce4     	dup.16b	v4, w7
    160c: 4e040025     	tbl.16b	v5, { v1 }, v4
    1610: 0e013ca5     	umov.b	w5, v5[0]
    1614: 910004a5     	add	x5, x5, #0x1
    1618: d2800006     	mov	x6, #0x0                ; =0
    161c: eb0700bf     	cmp	x5, x7
    1620: 9a8580c6     	csel	x6, x6, x5, hi
    1624: f103fcdf     	cmp	x6, #0xff
    1628: 54000049     	b.ls	0x1630 <_mech_control_range_0+0x300>
    162c: d65f03c0     	ret
    1630: 4e0302d6     	tbl.16b	v22, { v22 }, v3
    1634: 6e208c83     	cmeq.16b	v3, v4, v0
    1638: 4e010cc4     	dup.16b	v4, w6
    163c: 6e611c83     	bsl.16b	v3, v4, v1
    1640: eb0700bf     	cmp	x5, x7
    1644: 9a878080     	csel	x0, x4, x7, hi
    1648: eb0700bf     	cmp	x5, x7
    164c: 9a9f87e4     	cset	x4, ls
    1650: aa0003e7     	mov	x7, x0
    1654: 4ea31c61     	mov.16b	v1, v3
    1658: 17ffffa3     	b	0x14e4 <_mech_control_range_0+0x1b4>
    165c: 910004a6     	add	x6, x5, #0x1
    1660: f10040df     	cmp	x6, #0x10
    1664: 9a9f87e7     	cset	x7, ls
    1668: f10040df     	cmp	x6, #0x10
    166c: 54000069     	b.ls	0x1678 <_mech_control_range_0+0x348>
    1670: d2800080     	mov	x0, #0x4                ; =4
    1674: d65f03c0     	ret
    1678: 90000009     	adrp	x9, 0x1000 <_mech_control_reduce+0x1000>
    167c: f9400129     	ldr	x9, [x9]
    1680: d2800080     	mov	x0, #0x4                ; =4
    1684: 3ce67923     	ldr	q3, [x9, x6, lsl #4]
    1688: 92800009     	mov	x9, #-0x1               ; =-1
    168c: eb0900bf     	cmp	x5, x9
    1690: 9a9f07e9     	cset	x9, ne
    1694: 0a070127     	and	w7, w9, w7
    1698: 53001ce9     	uxtb	w9, w7
    169c: b5000049     	cbnz	x9, 0x16a4 <_mech_control_range_0+0x374>
    16a0: d65f03c0     	ret
    16a4: 53001ce7     	uxtb	w7, w7
    16a8: b5000047     	cbnz	x7, 0x16b0 <_mech_control_range_0+0x380>
    16ac: d65f03c0     	ret
    16b0: 4e010ca6     	dup.16b	v6, w5
    16b4: 4e060027     	tbl.16b	v7, { v1 }, v6
    16b8: 0e013ce7     	umov.b	w7, v7[0]
    16bc: 910004e7     	add	x7, x7, #0x1
    16c0: d2800009     	mov	x9, #0x0                ; =0
    16c4: eb0500ff     	cmp	x7, x5
    16c8: 9a878129     	csel	x9, x9, x7, hi
    16cc: f103fd3f     	cmp	x9, #0xff
    16d0: 54000049     	b.ls	0x16d8 <_mech_control_range_0+0x3a8>
    16d4: d65f03c0     	ret
    16d8: 4e030210     	tbl.16b	v16, { v16 }, v3
    16dc: 6e208cd1     	cmeq.16b	v17, v6, v0
    16e0: 4e010d32     	dup.16b	v18, w9
    16e4: 6e611e51     	bsl.16b	v17, v18, v1
    16e8: eb0500ff     	cmp	x7, x5
    16ec: 9a8580cb     	csel	x11, x6, x5, hi
    16f0: eb0500ff     	cmp	x7, x5
    16f4: 9a9f87e0     	cset	x0, ls
    16f8: aa0b03e5     	mov	x5, x11
    16fc: 4eb11e21     	mov.16b	v1, v17
    1700: 17ffff6f     	b	0x14bc <_mech_control_range_0+0x18c>
    1704: aa0503e8     	mov	x8, x5
    1708: f100505f     	cmp	x2, #0x14
    170c: 54000069     	b.ls	0x1718 <_mech_control_range_0+0x3e8>
    1710: d2800060     	mov	x0, #0x3                ; =3
    1714: d65f03c0     	ret
    1718: d2800020     	mov	x0, #0x1                ; =1
    171c: aa0003e5     	mov	x5, x0
    1720: eb02001f     	cmp	x0, x2
    1724: 54000669     	b.ls	0x17f0 <_mech_control_range_0+0x4c0>
    1728: f10000bf     	cmp	x5, #0x0
    172c: 54000061     	b.ne	0x1738 <_mech_control_range_0+0x408>
    1730: d2800040     	mov	x0, #0x2                ; =2
    1734: d65f03c0     	ret
    1738: b4000645     	cbz	x5, 0x1800 <_mech_control_range_0+0x4d0>
    173c: 9ac50886     	udiv	x6, x4, x5
    1740: 91000447     	add	x7, x2, #0x1
    1744: f10040ff     	cmp	x7, #0x10
    1748: 9a9f87e9     	cset	x9, ls
    174c: f10040ff     	cmp	x7, #0x10
    1750: 54000069     	b.ls	0x175c <_mech_control_range_0+0x42c>
    1754: d2800080     	mov	x0, #0x4                ; =4
    1758: d65f03c0     	ret
    175c: d2800020     	mov	x0, #0x1                ; =1
    1760: eb1f00ff     	cmp	x7, xzr
    1764: 9a8010e0     	csel	x0, x7, x0, ne
    1768: b40004e0     	cbz	x0, 0x1804 <_mech_control_range_0+0x4d4>
    176c: 9ac008ca     	udiv	x10, x6, x0
    1770: 9b00994a     	msub	x10, x10, x0, x6
    1774: 9000000b     	adrp	x11, 0x1000 <_mech_control_reduce+0x1000>
    1778: f940016b     	ldr	x11, [x11]
    177c: d2800080     	mov	x0, #0x4                ; =4
    1780: 8b071147     	add	x7, x10, x7, lsl #4
    1784: 3ce7797d     	ldr	q29, [x11, x7, lsl #4]
    1788: 92800007     	mov	x7, #-0x1               ; =-1
    178c: eb07005f     	cmp	x2, x7
    1790: 9a9f07e7     	cset	x7, ne
    1794: 0a0900e7     	and	w7, w7, w9
    1798: 53001ce7     	uxtb	w7, w7
    179c: b5000047     	cbnz	x7, 0x17a4 <_mech_control_range_0+0x474>
    17a0: d65f03c0     	ret
    17a4: f103fcdf     	cmp	x6, #0xff
    17a8: 54000049     	b.ls	0x17b0 <_mech_control_range_0+0x480>
    17ac: d65f03c0     	ret
    17b0: f10000bf     	cmp	x5, #0x0
    17b4: 54000061     	b.ne	0x17c0 <_mech_control_range_0+0x490>
    17b8: d2800040     	mov	x0, #0x2                ; =2
    17bc: d65f03c0     	ret
    17c0: b4000245     	cbz	x5, 0x1808 <_mech_control_range_0+0x4d8>
    17c4: 9ac50880     	udiv	x0, x4, x5
    17c8: 9b059004     	msub	x4, x0, x5, x4
    17cc: 4e1d0042     	tbl.16b	v2, { v2 }, v29
    17d0: 4e010c43     	dup.16b	v3, w2
    17d4: 6e208c63     	cmeq.16b	v3, v3, v0
    17d8: 4e010cc4     	dup.16b	v4, w6
    17dc: 6e611c83     	bsl.16b	v3, v4, v1
    17e0: d1000442     	sub	x2, x2, #0x1
    17e4: aa0803e5     	mov	x5, x8
    17e8: 4ea31c61     	mov.16b	v1, v3
    17ec: 17fffed8     	b	0x134c <_mech_control_range_0+0x1c>
    17f0: 91000406     	add	x6, x0, #0x1
    17f4: 9b007ca5     	mul	x5, x5, x0
    17f8: aa0603e0     	mov	x0, x6
    17fc: 17ffffc9     	b	0x1720 <_mech_control_range_0+0x3f0>
    1800: 0000c11f     	udf	#0xc11f
    1804: 0000c11f     	udf	#0xc11f
    1808: 0000c11f     	udf	#0xc11f
    180c: 00000000     	udf	#0x0
    1810: 03020100     	<unknown>
    1814: 07060504     	<unknown>
    1818: 0b0a0908     	add	w8, w8, w10, lsl #2
    181c: 0f0e0d0c     	<unknown>

0000000000001820 <_mech_control_entry>:
    1820: d503237f     	pacibsp
    1824: a9bf7bfd     	stp	x29, x30, [sp, #-0x10]!
    1828: 910003fd     	mov	x29, sp
    182c: a9bf53f3     	stp	x19, x20, [sp, #-0x10]!
    1830: d100c3ff     	sub	sp, sp, #0x30
    1834: aa0103f4     	mov	x20, x1
    1838: f9400013     	ldr	x19, [x0]
    183c: f100027f     	cmp	x19, #0x0
    1840: 9a9f07e0     	cset	x0, ne
    1844: f100427f     	cmp	x19, #0x10
    1848: 9a9f87e1     	cset	x1, ls
    184c: 0a010000     	and	w0, w0, w1
    1850: 53001c00     	uxtb	w0, w0
    1854: b50000c0     	cbnz	x0, 0x186c <_mech_control_entry+0x4c>
    1858: d2800020     	mov	x0, #0x1                ; =1
    185c: 9100c3ff     	add	sp, sp, #0x30
    1860: a8c153f3     	ldp	x19, x20, [sp], #0x10
    1864: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1868: d65f0fff     	retab
    186c: f100527f     	cmp	x19, #0x14
    1870: 540000c9     	b.ls	0x1888 <_mech_control_entry+0x68>
    1874: d2800060     	mov	x0, #0x3                ; =3
    1878: 9100c3ff     	add	sp, sp, #0x30
    187c: a8c153f3     	ldp	x19, x20, [sp], #0x10
    1880: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1884: d65f0fff     	retab
    1888: d2800024     	mov	x4, #0x1                ; =1
    188c: aa0403e1     	mov	x1, x4
    1890: aa0403e2     	mov	x2, x4
    1894: eb13005f     	cmp	x2, x19
    1898: 54000489     	b.ls	0x1928 <_mech_control_entry+0x108>
    189c: 910043e3     	add	x3, sp, #0x10
    18a0: f80103f3     	stur	x19, [sp, #0x10]
    18a4: d2800000     	mov	x0, #0x0                ; =0
    18a8: d2800047     	mov	x7, #0x2                ; =2
    18ac: d347fc22     	lsr	x2, x1, #7
    18b0: 927ff842     	and	x2, x2, #0xfffffffffffffffe
    18b4: eb0200ff     	cmp	x7, x2
    18b8: 9a8280e2     	csel	x2, x7, x2, hi
    18bc: 90000005     	adrp	x5, 0x1000 <_mech_control_reduce+0x1000>
    18c0: f94000a5     	ldr	x5, [x5]
    18c4: 90000006     	adrp	x6, 0x1000 <_mech_control_reduce+0x1000>
    18c8: f94000c6     	ldr	x6, [x6]
    18cc: 910083e8     	add	x8, sp, #0x20
    18d0: f80003e8     	stur	x8, [sp]
    18d4: 90000008     	adrp	x8, 0x1000 <_mech_control_reduce+0x1000>
    18d8: f9400108     	ldr	x8, [x8]
    18dc: d63f0100     	blr	x8
    18e0: f100001f     	cmp	x0, #0x0
    18e4: 540000c0     	b.eq	0x18fc <_mech_control_entry+0xdc>
    18e8: d28000a0     	mov	x0, #0x5                ; =5
    18ec: 9100c3ff     	add	sp, sp, #0x30
    18f0: a8c153f3     	ldp	x19, x20, [sp], #0x10
    18f4: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    18f8: d65f0fff     	retab
    18fc: f84203e1     	ldur	x1, [sp, #0x20]
    1900: f84283e0     	ldur	x0, [sp, #0x28]
    1904: aa1403e6     	mov	x6, x20
    1908: f90000c1     	str	x1, [x6]
    190c: f90004d3     	str	x19, [x6, #0x8]
    1910: f90008c0     	str	x0, [x6, #0x10]
    1914: d2800000     	mov	x0, #0x0                ; =0
    1918: 9100c3ff     	add	sp, sp, #0x30
    191c: a8c153f3     	ldp	x19, x20, [sp], #0x10
    1920: a8c17bfd     	ldp	x29, x30, [sp], #0x10
    1924: d65f0fff     	retab
    1928: aa1403e6     	mov	x6, x20
    192c: 91000440     	add	x0, x2, #0x1
    1930: 9b027c21     	mul	x1, x1, x2
    1934: aa0003e2     	mov	x2, x0
    1938: 17ffffd7     	b	0x1894 <_mech_control_entry+0x74>
