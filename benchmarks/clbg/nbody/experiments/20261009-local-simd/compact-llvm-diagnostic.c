#include <stddef.h>
#include <math.h>
void mech_turns(double *restrict state, size_t turns) {
  double s0 = state[0];
  double s1 = state[1];
  double s2 = state[2];
  double s3 = state[3];
  double s4 = state[4];
  double s5 = state[5];
  double s6 = state[6];
  double s7 = state[7];
  double s8 = state[8];
  double s9 = state[9];
  double s10 = state[10];
  double s11 = state[11];
  double s12 = state[12];
  double s13 = state[13];
  double s14 = state[14];
  double s15 = state[15];
  double s16 = state[16];
  double s17 = state[17];
  double s18 = state[18];
  double s19 = state[19];
  double s20 = state[20];
  double s21 = state[21];
  double s22 = state[22];
  double s23 = state[23];
  double s24 = state[24];
  double s25 = state[25];
  double s26 = state[26];
  double s27 = state[27];
  double s28 = state[28];
  double s29 = state[29];
  for (size_t turn = 0; turn < turns; ++turn) {
    const double r0 = (s0 - s1);
    const double r1 = (s5 - s6);
    const double r2 = (s10 - s11);
    const double r3 = (s0 - s2);
    const double r4 = (s5 - s7);
    const double r5 = (s10 - s12);
    const double r6 = (s0 - s3);
    const double r7 = (s5 - s8);
    const double r8 = (s10 - s13);
    const double r9 = (s0 - s4);
    const double r10 = (s5 - s9);
    const double r11 = (s10 - s14);
    const double r12 = (s1 - s2);
    const double r13 = (s6 - s7);
    const double r14 = (s11 - s12);
    const double r15 = (s1 - s3);
    const double r16 = (s6 - s8);
    const double r17 = (s11 - s13);
    const double r18 = (s1 - s4);
    const double r19 = (s6 - s9);
    const double r20 = (s11 - s14);
    const double r21 = (s2 - s3);
    const double r22 = (s7 - s8);
    const double r23 = (s12 - s13);
    const double r24 = (s2 - s4);
    const double r25 = (s7 - s9);
    const double r26 = (s12 - s14);
    const double r27 = (s3 - s4);
    const double r28 = (s8 - s9);
    const double r29 = (s13 - s14);
    const double r60 = fma(r0, r0, 0.00000000000000000e0);
    const double r61 = fma(r3, r3, 0.00000000000000000e0);
    const double r62 = fma(r6, r6, 0.00000000000000000e0);
    const double r63 = fma(r9, r9, 0.00000000000000000e0);
    const double r64 = fma(r12, r12, 0.00000000000000000e0);
    const double r65 = fma(r15, r15, 0.00000000000000000e0);
    const double r66 = fma(r18, r18, 0.00000000000000000e0);
    const double r67 = fma(r21, r21, 0.00000000000000000e0);
    const double r68 = fma(r24, r24, 0.00000000000000000e0);
    const double r69 = fma(r27, r27, 0.00000000000000000e0);
    const double r70 = fma(r1, r1, r60);
    const double r71 = fma(r4, r4, r61);
    const double r72 = fma(r7, r7, r62);
    const double r73 = fma(r10, r10, r63);
    const double r74 = fma(r13, r13, r64);
    const double r75 = fma(r16, r16, r65);
    const double r76 = fma(r19, r19, r66);
    const double r77 = fma(r22, r22, r67);
    const double r78 = fma(r25, r25, r68);
    const double r79 = fma(r28, r28, r69);
    const double r80 = fma(r2, r2, r70);
    const double r81 = fma(r5, r5, r71);
    const double r82 = fma(r8, r8, r72);
    const double r83 = fma(r11, r11, r73);
    const double r84 = fma(r14, r14, r74);
    const double r85 = fma(r17, r17, r75);
    const double r86 = fma(r20, r20, r76);
    const double r87 = fma(r23, r23, r77);
    const double r88 = fma(r26, r26, r78);
    const double r89 = fma(r29, r29, r79);
    const double r161 = sqrt(r80);
    const double r162 = sqrt(r81);
    const double r163 = sqrt(r82);
    const double r164 = sqrt(r83);
    const double r165 = sqrt(r84);
    const double r166 = sqrt(r85);
    const double r167 = sqrt(r86);
    const double r168 = sqrt(r87);
    const double r169 = sqrt(r88);
    const double r170 = sqrt(r89);
    const double r171 = (r80 * r161);
    const double r172 = (r81 * r162);
    const double r173 = (r82 * r163);
    const double r174 = (r83 * r164);
    const double r175 = (r84 * r165);
    const double r176 = (r85 * r166);
    const double r177 = (r86 * r167);
    const double r178 = (r87 * r168);
    const double r179 = (r88 * r169);
    const double r180 = (r89 * r170);
    const double r181 = (1.00000000000000002e-2 / r171);
    const double r182 = (1.00000000000000002e-2 / r172);
    const double r183 = (1.00000000000000002e-2 / r173);
    const double r184 = (1.00000000000000002e-2 / r174);
    const double r185 = (1.00000000000000002e-2 / r175);
    const double r186 = (1.00000000000000002e-2 / r176);
    const double r187 = (1.00000000000000002e-2 / r177);
    const double r188 = (1.00000000000000002e-2 / r178);
    const double r189 = (1.00000000000000002e-2 / r179);
    const double r190 = (1.00000000000000002e-2 / r180);
    const double r191 = (r0 * 3.76936748703894930e-2);
    const double r192 = (r1 * 3.76936748703894930e-2);
    const double r193 = (r2 * 3.76936748703894930e-2);
    const double r194 = (r3 * 1.12863261319687668e-2);
    const double r195 = (r4 * 1.12863261319687668e-2);
    const double r196 = (r5 * 1.12863261319687668e-2);
    const double r197 = (r6 * 1.72372405705971117e-3);
    const double r198 = (r7 * 1.72372405705971117e-3);
    const double r199 = (r8 * 1.72372405705971117e-3);
    const double r200 = (r9 * 2.03368686992463042e-3);
    const double r201 = (r10 * 2.03368686992463042e-3);
    const double r202 = (r11 * 2.03368686992463042e-3);
    const double r203 = (r12 * 1.12863261319687668e-2);
    const double r204 = (r13 * 1.12863261319687668e-2);
    const double r205 = (r14 * 1.12863261319687668e-2);
    const double r206 = (r15 * 1.72372405705971117e-3);
    const double r207 = (r16 * 1.72372405705971117e-3);
    const double r208 = (r17 * 1.72372405705971117e-3);
    const double r209 = (r18 * 2.03368686992463042e-3);
    const double r210 = (r19 * 2.03368686992463042e-3);
    const double r211 = (r20 * 2.03368686992463042e-3);
    const double r212 = (r21 * 1.72372405705971117e-3);
    const double r213 = (r22 * 1.72372405705971117e-3);
    const double r214 = (r23 * 1.72372405705971117e-3);
    const double r215 = (r24 * 2.03368686992463042e-3);
    const double r216 = (r25 * 2.03368686992463042e-3);
    const double r217 = (r26 * 2.03368686992463042e-3);
    const double r218 = (r27 * 2.03368686992463042e-3);
    const double r219 = (r28 * 2.03368686992463042e-3);
    const double r220 = (r29 * 2.03368686992463042e-3);
    const double r251 = fma(-(r191), r181, s15);
    const double r252 = fma(-(r192), r181, s20);
    const double r253 = fma(-(r193), r181, s25);
    const double r254 = fma(-(r194), r182, r251);
    const double r255 = fma(-(r195), r182, r252);
    const double r256 = fma(-(r196), r182, r253);
    const double r257 = fma(-(r197), r183, r254);
    const double r258 = fma(-(r198), r183, r255);
    const double r259 = fma(-(r199), r183, r256);
    const double r260 = fma(-(r200), r184, r257);
    const double r261 = fma(-(r201), r184, r258);
    const double r262 = fma(-(r202), r184, r259);
    const double r263 = fma(-(r203), r185, s16);
    const double r264 = fma(-(r204), r185, s21);
    const double r265 = fma(-(r205), r185, s26);
    const double r266 = fma(-(r206), r186, r263);
    const double r267 = fma(-(r207), r186, r264);
    const double r268 = fma(-(r208), r186, r265);
    const double r269 = fma(-(r209), r187, r266);
    const double r270 = fma(-(r210), r187, r267);
    const double r271 = fma(-(r211), r187, r268);
    const double r272 = fma(-(r212), r188, s17);
    const double r273 = fma(-(r213), r188, s22);
    const double r274 = fma(-(r214), r188, s27);
    const double r275 = fma(-(r215), r189, r272);
    const double r276 = fma(-(r216), r189, r273);
    const double r277 = fma(-(r217), r189, r274);
    const double r278 = fma(-(r218), r190, s18);
    const double r279 = fma(-(r219), r190, s23);
    const double r280 = fma(-(r220), r190, s28);
    const double r281 = (r0 * 3.94784176043574320e1);
    const double r282 = (r1 * 3.94784176043574320e1);
    const double r283 = (r2 * 3.94784176043574320e1);
    const double r284 = (r3 * 3.94784176043574320e1);
    const double r285 = (r4 * 3.94784176043574320e1);
    const double r286 = (r5 * 3.94784176043574320e1);
    const double r287 = (r6 * 3.94784176043574320e1);
    const double r288 = (r7 * 3.94784176043574320e1);
    const double r289 = (r8 * 3.94784176043574320e1);
    const double r290 = (r9 * 3.94784176043574320e1);
    const double r291 = (r10 * 3.94784176043574320e1);
    const double r292 = (r11 * 3.94784176043574320e1);
    const double r293 = (r12 * 3.76936748703894930e-2);
    const double r294 = (r13 * 3.76936748703894930e-2);
    const double r295 = (r14 * 3.76936748703894930e-2);
    const double r296 = (r15 * 3.76936748703894930e-2);
    const double r297 = (r16 * 3.76936748703894930e-2);
    const double r298 = (r17 * 3.76936748703894930e-2);
    const double r299 = (r18 * 3.76936748703894930e-2);
    const double r300 = (r19 * 3.76936748703894930e-2);
    const double r301 = (r20 * 3.76936748703894930e-2);
    const double r302 = (r21 * 1.12863261319687668e-2);
    const double r303 = (r22 * 1.12863261319687668e-2);
    const double r304 = (r23 * 1.12863261319687668e-2);
    const double r305 = (r24 * 1.12863261319687668e-2);
    const double r306 = (r25 * 1.12863261319687668e-2);
    const double r307 = (r26 * 1.12863261319687668e-2);
    const double r308 = (r27 * 1.72372405705971117e-3);
    const double r309 = (r28 * 1.72372405705971117e-3);
    const double r310 = (r29 * 1.72372405705971117e-3);
    const double r341 = fma(r281, r181, r269);
    const double r342 = fma(r282, r181, r270);
    const double r343 = fma(r283, r181, r271);
    const double r344 = fma(r284, r182, r275);
    const double r345 = fma(r285, r182, r276);
    const double r346 = fma(r286, r182, r277);
    const double r347 = fma(r287, r183, r278);
    const double r348 = fma(r288, r183, r279);
    const double r349 = fma(r289, r183, r280);
    const double r350 = fma(r290, r184, s19);
    const double r351 = fma(r291, r184, s24);
    const double r352 = fma(r292, r184, s29);
    const double r353 = fma(r293, r185, r344);
    const double r354 = fma(r294, r185, r345);
    const double r355 = fma(r295, r185, r346);
    const double r356 = fma(r296, r186, r347);
    const double r357 = fma(r297, r186, r348);
    const double r358 = fma(r298, r186, r349);
    const double r359 = fma(r299, r187, r350);
    const double r360 = fma(r300, r187, r351);
    const double r361 = fma(r301, r187, r352);
    const double r362 = fma(r302, r188, r356);
    const double r363 = fma(r303, r188, r357);
    const double r364 = fma(r304, r188, r358);
    const double r365 = fma(r305, r189, r359);
    const double r366 = fma(r306, r189, r360);
    const double r367 = fma(r307, r189, r361);
    const double r368 = fma(r308, r190, r365);
    const double r369 = fma(r309, r190, r366);
    const double r370 = fma(r310, r190, r367);
    const double r386 = fma(r260, 1.00000000000000002e-2, s0);
    const double r387 = fma(r261, 1.00000000000000002e-2, s5);
    const double r388 = fma(r262, 1.00000000000000002e-2, s10);
    const double r389 = fma(r341, 1.00000000000000002e-2, s1);
    const double r390 = fma(r342, 1.00000000000000002e-2, s6);
    const double r391 = fma(r343, 1.00000000000000002e-2, s11);
    const double r392 = fma(r353, 1.00000000000000002e-2, s2);
    const double r393 = fma(r354, 1.00000000000000002e-2, s7);
    const double r394 = fma(r355, 1.00000000000000002e-2, s12);
    const double r395 = fma(r362, 1.00000000000000002e-2, s3);
    const double r396 = fma(r363, 1.00000000000000002e-2, s8);
    const double r397 = fma(r364, 1.00000000000000002e-2, s13);
    const double r398 = fma(r368, 1.00000000000000002e-2, s4);
    const double r399 = fma(r369, 1.00000000000000002e-2, s9);
    const double r400 = fma(r370, 1.00000000000000002e-2, s14);
    const double next0 = r386;
    const double next1 = r389;
    const double next2 = r392;
    const double next3 = r395;
    const double next4 = r398;
    const double next5 = r387;
    const double next6 = r390;
    const double next7 = r393;
    const double next8 = r396;
    const double next9 = r399;
    const double next10 = r388;
    const double next11 = r391;
    const double next12 = r394;
    const double next13 = r397;
    const double next14 = r400;
    const double next15 = r260;
    const double next16 = r341;
    const double next17 = r353;
    const double next18 = r362;
    const double next19 = r368;
    const double next20 = r261;
    const double next21 = r342;
    const double next22 = r354;
    const double next23 = r363;
    const double next24 = r369;
    const double next25 = r262;
    const double next26 = r343;
    const double next27 = r355;
    const double next28 = r364;
    const double next29 = r370;
    s0 = next0;
    s1 = next1;
    s2 = next2;
    s3 = next3;
    s4 = next4;
    s5 = next5;
    s6 = next6;
    s7 = next7;
    s8 = next8;
    s9 = next9;
    s10 = next10;
    s11 = next11;
    s12 = next12;
    s13 = next13;
    s14 = next14;
    s15 = next15;
    s16 = next16;
    s17 = next17;
    s18 = next18;
    s19 = next19;
    s20 = next20;
    s21 = next21;
    s22 = next22;
    s23 = next23;
    s24 = next24;
    s25 = next25;
    s26 = next26;
    s27 = next27;
    s28 = next28;
    s29 = next29;
  }
  state[0] = s0;
  state[1] = s1;
  state[2] = s2;
  state[3] = s3;
  state[4] = s4;
  state[5] = s5;
  state[6] = s6;
  state[7] = s7;
  state[8] = s8;
  state[9] = s9;
  state[10] = s10;
  state[11] = s11;
  state[12] = s12;
  state[13] = s13;
  state[14] = s14;
  state[15] = s15;
  state[16] = s16;
  state[17] = s17;
  state[18] = s18;
  state[19] = s19;
  state[20] = s20;
  state[21] = s21;
  state[22] = s22;
  state[23] = s23;
  state[24] = s24;
  state[25] = s25;
  state[26] = s26;
  state[27] = s27;
  state[28] = s28;
  state[29] = s29;
}
void mech_observe(const double *state, double *output) {
  const double s0 = state[0];
  const double s1 = state[1];
  const double s2 = state[2];
  const double s3 = state[3];
  const double s4 = state[4];
  const double s5 = state[5];
  const double s6 = state[6];
  const double s7 = state[7];
  const double s8 = state[8];
  const double s9 = state[9];
  const double s10 = state[10];
  const double s11 = state[11];
  const double s12 = state[12];
  const double s13 = state[13];
  const double s14 = state[14];
  const double s15 = state[15];
  const double s16 = state[16];
  const double s17 = state[17];
  const double s18 = state[18];
  const double s19 = state[19];
  const double s20 = state[20];
  const double s21 = state[21];
  const double s22 = state[22];
  const double s23 = state[23];
  const double s24 = state[24];
  const double s25 = state[25];
  const double s26 = state[26];
  const double s27 = state[27];
  const double s28 = state[28];
  const double s29 = state[29];
    const double r0 = (s0 - s1);
    const double r1 = (s5 - s6);
    const double r2 = (s10 - s11);
    const double r3 = (s0 - s2);
    const double r4 = (s5 - s7);
    const double r5 = (s10 - s12);
    const double r6 = (s0 - s3);
    const double r7 = (s5 - s8);
    const double r8 = (s10 - s13);
    const double r9 = (s0 - s4);
    const double r10 = (s5 - s9);
    const double r11 = (s10 - s14);
    const double r12 = (s1 - s2);
    const double r13 = (s6 - s7);
    const double r14 = (s11 - s12);
    const double r15 = (s1 - s3);
    const double r16 = (s6 - s8);
    const double r17 = (s11 - s13);
    const double r18 = (s1 - s4);
    const double r19 = (s6 - s9);
    const double r20 = (s11 - s14);
    const double r21 = (s2 - s3);
    const double r22 = (s7 - s8);
    const double r23 = (s12 - s13);
    const double r24 = (s2 - s4);
    const double r25 = (s7 - s9);
    const double r26 = (s12 - s14);
    const double r27 = (s3 - s4);
    const double r28 = (s8 - s9);
    const double r29 = (s13 - s14);
    const double r60 = fma(r0, r0, 0.00000000000000000e0);
    const double r61 = fma(r3, r3, 0.00000000000000000e0);
    const double r62 = fma(r6, r6, 0.00000000000000000e0);
    const double r63 = fma(r9, r9, 0.00000000000000000e0);
    const double r64 = fma(r12, r12, 0.00000000000000000e0);
    const double r65 = fma(r15, r15, 0.00000000000000000e0);
    const double r66 = fma(r18, r18, 0.00000000000000000e0);
    const double r67 = fma(r21, r21, 0.00000000000000000e0);
    const double r68 = fma(r24, r24, 0.00000000000000000e0);
    const double r69 = fma(r27, r27, 0.00000000000000000e0);
    const double r70 = fma(r1, r1, r60);
    const double r71 = fma(r4, r4, r61);
    const double r72 = fma(r7, r7, r62);
    const double r73 = fma(r10, r10, r63);
    const double r74 = fma(r13, r13, r64);
    const double r75 = fma(r16, r16, r65);
    const double r76 = fma(r19, r19, r66);
    const double r77 = fma(r22, r22, r67);
    const double r78 = fma(r25, r25, r68);
    const double r79 = fma(r28, r28, r69);
    const double r80 = fma(r2, r2, r70);
    const double r81 = fma(r5, r5, r71);
    const double r82 = fma(r8, r8, r72);
    const double r83 = fma(r11, r11, r73);
    const double r84 = fma(r14, r14, r74);
    const double r85 = fma(r17, r17, r75);
    const double r86 = fma(r20, r20, r76);
    const double r87 = fma(r23, r23, r77);
    const double r88 = fma(r26, r26, r78);
    const double r89 = fma(r29, r29, r79);
    const double r105 = fma(s15, s15, 0.00000000000000000e0);
    const double r106 = fma(s16, s16, 0.00000000000000000e0);
    const double r107 = fma(s17, s17, 0.00000000000000000e0);
    const double r108 = fma(s18, s18, 0.00000000000000000e0);
    const double r109 = fma(s19, s19, 0.00000000000000000e0);
    const double r110 = fma(s20, s20, r105);
    const double r111 = fma(s21, s21, r106);
    const double r112 = fma(s22, s22, r107);
    const double r113 = fma(s23, s23, r108);
    const double r114 = fma(s24, s24, r109);
    const double r115 = fma(s25, s25, r110);
    const double r116 = fma(s26, s26, r111);
    const double r117 = fma(s27, s27, r112);
    const double r118 = fma(s28, s28, r113);
    const double r119 = fma(s29, s29, r114);
    const double r125 = fma(1.97392088021787160e1, r115, 0.00000000000000000e0);
    const double r126 = fma(1.88468374351947465e-2, r116, r125);
    const double r127 = fma(5.64316306598438341e-3, r117, r126);
    const double r128 = fma(8.61862028529855584e-4, r118, r127);
    const double r129 = fma(1.01684343496231521e-3, r119, r128);
    const double r130 = sqrt(r80);
    const double r131 = sqrt(r81);
    const double r132 = sqrt(r82);
    const double r133 = sqrt(r83);
    const double r134 = sqrt(r84);
    const double r135 = sqrt(r85);
    const double r136 = sqrt(r86);
    const double r137 = sqrt(r87);
    const double r138 = sqrt(r88);
    const double r139 = sqrt(r89);
    const double r140 = (1.48808663757610993e0 / r130);
    const double r141 = (4.45566296256835082e-1 / r131);
    const double r142 = (6.80498981592805169e-2 / r132);
    const double r143 = (8.02867395273830914e-2 / r133);
    const double r144 = (4.25423107699611351e-4 / r134);
    const double r145 = (6.49734941730774617e-5 / r135);
    const double r146 = (7.66571316631191091e-5 / r136);
    const double r147 = (1.94545118694962389e-5 / r137);
    const double r148 = (2.29528532642721211e-5 / r138);
    const double r149 = (3.50551498221554921e-6 / r139);
    const double r150 = (0.00000000000000000e0 + r140);
    const double r151 = (r150 + r141);
    const double r152 = (r151 + r142);
    const double r153 = (r152 + r143);
    const double r154 = (r153 + r144);
    const double r155 = (r154 + r145);
    const double r156 = (r155 + r146);
    const double r157 = (r156 + r147);
    const double r158 = (r157 + r148);
    const double r159 = (r158 + r149);
    const double r160 = (r129 - r159);
  output[0] = r160;
}
#define STATE_COUNT 30
#define OUTPUT_COUNT 1
static const double initial[30] = {
0.00000000000000000e0,
4.84143144246472090e0,
8.34336671824457987e0,
1.28943695621391310e1,
1.53796971148509165e1,
0.00000000000000000e0,
-1.16032004402742839e0,
4.12479856412430479e0,
-1.51111514016986312e1,
-2.59193146099879641e1,
0.00000000000000000e0,
-1.03622044471123109e-1,
-4.03523417114321381e-1,
-2.23307578892655734e-1,
1.79258772950371181e-1,
-3.87663407198742666e-4,
6.06326392995832020e-1,
-1.01077434617879236e0,
1.08279100644153536e0,
9.79090732243897977e-1,
-3.27535903717657067e-3,
2.81198684491626016e0,
1.82566237123041186e0,
8.68713018169608220e-1,
5.94698998647676169e-1,
2.39357340800029987e-5,
-2.52183616598876288e-2,
8.41576137658415352e-3,
-1.08326374013636359e-2,
-3.47559555040781037e-2,
};
/* Generic native runtime: execute turns and display the selected Mech output.
 * State initializers and dimensions are generated from the compiled artifact. */
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <time.h>
#include <string.h>

double mech_fused_pow(double x, double y) { return pow(x, y); }

static void print_output(const double *values) {
  if (OUTPUT_COUNT == 1) { printf("%.9f\n", values[0]); return; }
  printf("[");
  for (size_t i = 0; i < OUTPUT_COUNT; ++i) printf("%s%.17g", i ? ", " : "", values[i]);
  printf("]\n");
}

int main(int argc, char **argv) {
  if (argc != 2) return 2;
  errno = 0; char *end;
  unsigned long long turns = strtoull(argv[1], &end, 10);
  if (errno || *end || argv[1][0] == '-' || end == argv[1] || turns > (size_t)-1) return 2;
  double state[STATE_COUNT ? STATE_COUNT : 1], output[OUTPUT_COUNT];
  memcpy(state, initial, STATE_COUNT * sizeof(double));
  mech_observe(state, output);
  print_output(output);
  struct timespec start, finish;
  clock_gettime(CLOCK_MONOTONIC, &start);
  mech_turns(state, (size_t)turns);
  clock_gettime(CLOCK_MONOTONIC, &finish);
  mech_observe(state, output);
  print_output(output);
  if (getenv("MECH_DUMP_STATE")) {
    fprintf(stderr, "STATE [");
    for (size_t i = 0; i < STATE_COUNT; ++i) fprintf(stderr, "%s%.17g", i ? ", " : "", state[i]);
    fprintf(stderr, "]\n");
  }
  if (getenv("MECH_TIMINGS")) {
    double elapsed = finish.tv_sec-start.tv_sec + (finish.tv_nsec-start.tv_nsec)*1e-9;
    fprintf(stderr, "TIMING {\"prepare_seconds\":0,\"kernel_seconds\":%.9g}\n", elapsed);
  }
  return 0;
}
