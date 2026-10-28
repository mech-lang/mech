
/private/tmp/mech-deflate-profiling-20261028/roc-profile:	file format mach-o arm64

Disassembly of section __TEXT,__text:

000000010001e298 <_roc__p5794f552e2e0fcba87a322629fdca7c0.fast>:
10001e298: 6db82beb    	stp	d11, d10, [sp, #-0x80]!
10001e29c: 6d0123e9    	stp	d9, d8, [sp, #0x10]
10001e2a0: a9026ffc    	stp	x28, x27, [sp, #0x20]
10001e2a4: a90367fa    	stp	x26, x25, [sp, #0x30]
10001e2a8: a9045ff8    	stp	x24, x23, [sp, #0x40]
10001e2ac: a90557f6    	stp	x22, x21, [sp, #0x50]
10001e2b0: a9064ff4    	stp	x20, x19, [sp, #0x60]
10001e2b4: a9077bfd    	stp	x29, x30, [sp, #0x70]
10001e2b8: d14007ff    	sub	sp, sp, #0x1, lsl #12   ; =0x1000
10001e2bc: f90003ff    	str	xzr, [sp]
10001e2c0: d13783ff    	sub	sp, sp, #0xde0
10001e2c4: f90003ff    	str	xzr, [sp]
10001e2c8: f9011fe0    	str	x0, [sp, #0x238]
10001e2cc: a940302b    	ldp	x11, x12, [x1]
10001e2d0: a940647a    	ldp	x26, x25, [x3]
10001e2d4: f940087b    	ldr	x27, [x3, #0x10]
10001e2d8: 12001c48    	and	w8, w2, #0xff
10001e2dc: f9019feb    	str	x11, [sp, #0x338]
10001e2e0: f901a3ec    	str	x12, [sp, #0x340]
10001e2e4: 3400e7e8    	cbz	w8, 0x10001ffe0 <_roc__p5794f552e2e0fcba87a322629fdca7c0.fast+0x1d48>
10001e2e8: d37e1448    	ubfiz	x8, x2, #2, #6
10001e2ec: 528006e9    	mov	w9, #0x37               ; =55
10001e2f0: cb080128    	sub	x8, x9, x8
10001e2f4: eb0c011f    	cmp	x8, x12
10001e2f8: 5400e742    	b.hs	0x10001ffe0 <_roc__p5794f552e2e0fcba87a322629fdca7c0.fast+0x1d48>
10001e2fc: 914007f5    	add	x21, sp, #0x1, lsl #12  ; =0x1000
10001e300: 9122e2b5    	add	x21, x21, #0x8b8
10001e304: f9400828    	ldr	x8, [x1, #0x10]
