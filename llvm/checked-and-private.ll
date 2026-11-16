; ModuleID = 'checked-and-private.ll'
source_filename = "llvm-link"
target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-n32:64-S128-Fn32"
target triple = "arm64-apple-macosx26.0.0"

@mech_compact_table_11 = external dso_local constant [1094 x i8]
@mech_compact_table_14 = external dso_local constant [19 x i8]
@mech_compact_table_15 = external dso_local constant [19 x i8]
@mech_compact_table_18 = external dso_local constant [256 x i8]
@mech_compact_table_19 = external dso_local constant [256 x i8]
@mech_compact_table_2 = external dso_local constant [2188 x i8]
@mech_compact_table_20 = external dso_local constant [1164 x i8]
@mech_compact_table_21 = external dso_local constant [320 x i8]
@mech_compact_table_22 = external dso_local constant [320 x i8]
@mech_compact_table_24 = external dso_local constant [257 x i8]
@mech_compact_table_25 = external dso_local constant [257 x i8]
@mech_compact_table_26 = external dso_local constant [257 x i8]
@mech_compact_table_4 = external dso_local constant [3 x i8]
@mech_compact_table_6 = external dso_local constant [80 x i8]
@mech_compact_table_7 = external dso_local constant [259 x i8]
@mech_compact_table_8 = external dso_local constant [2064 x i8]
@mech_compact_table_9 = external dso_local constant [1280 x i8]
@mech_compact_cost_codes_0 = external dso_local constant [65536 x i8]

; Function Attrs: noinline uwtable
define { i64, i64 } @mech_compact_closed_checked(i64 %v0, i64 %v1, i64 %v2) local_unnamed_addr #0 align 128 {
block0:
  %ss0 = alloca [48 x i8], align 16
  %ss1 = alloca [48 x i8], align 8
  %ss2 = alloca [440 x i8], align 16
  %ss3 = alloca [616 x i8], align 8
  %ss4 = alloca [32 x i8], align 16
  %t0 = inttoptr i64 %v1 to ptr
  %v1396 = load i64, ptr %t0, align 1
  %t2 = getelementptr i8, ptr %t0, i64 8
  %v1397 = load i64, ptr %t2, align 1
  %t6 = getelementptr i8, ptr %t0, i64 24
  %v1399 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 32
  %v1400 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 40
  %v1401 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 48
  %v1402 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 56
  %v1403 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 64
  %v1404 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 72
  %v1405 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 80
  %v1406 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 88
  %v1407 = load i64, ptr %t22, align 1
  %t23.not = icmp eq i64 %v1401, 0
  br i1 %t23.not, label %block2, label %edge0

block2:                                           ; preds = %block0, %edge0
  %t26.not = icmp eq i64 %v1407, 0
  br i1 %t26.not, label %edge12, label %edge3

common.ret.loopexit22434.split.loop.exit33132:    ; preds = %edge12877
  %.mux.le = select i1 %28, { i64, i64 } { i64 1, i64 6726 }, { i64, i64 } { i64 1, i64 6727 }
  br label %common.ret

common.ret:                                       ; preds = %edge14176, %edge14166, %edge14158, %edge13653, %edge13645, %edge13634, %edge13630, %edge13540, %edge5300, %edge5147, %edge4971, %edge4885, %edge952, %edge940, %edge16243, %edge1063, %edge1179, %edge4539, %edge4931, %edge4917, %edge5027, %edge5146, %edge5477, %edge5422, %edge5941, %edge11362, %edge11368, %edge11652, %edge12057, %edge12238, %edge12667, %edge14011, %block7018, %edge14435, %edge14893, %edge15733, %edge4497, %edge4495, %edge4484, %edge4479, %edge4468, %edge4467, %edge4464, %edge3986, %edge1986, %edge1937, %edge1909, %edge1781, %edge1545, %edge1281, %edge4538, %edge4678, %edge10388, %edge11482, %edge11605, %edge12492, %edge12618, %edge12591, %edge12862, %edge12810, %edge12800, %edge4449, %edge4397, %edge4387, %block4170, %edge8645, %edge8617, %edge8562, %edge9275, %edge9219, %edge9224, %edge9971, %edge10800, %edge10737, %edge10697, %block5388, %edge10550, %edge8105, %block4623, %edge9572, %edge10635, %edge432, %common.ret.loopexit22434.split.loop.exit33132, %edge6644, %block37924, %block21488, %edge3341, %block16790, %edge2400, %block13251, %edge1758, %edge477, %edge462, %edge16315, %block100765, %block100709, %block100317, %block100179, %edge16038, %block100124, %edge15995, %block100069, %edge15955, %block100014, %edge15915, %block99915, %edge15862, %block99860, %edge15819, %block99672, %edge15713, %block99162, %block99082, %edge15571, %edge15559, %block98703, %block98636, %block96960, %edge14791, %block94357, %block94290, %block93504, %edge14428, %block92607, %block92183, %block92031, %edge14022, %block91830, %block91789, %block91747, %block91711, %block91569, %block91127, %edge13555, %block88851, %block87387, %block87363, %edge12949, %block85009, %edge12631, %edge12505, %block78101, %block77240, %block76577, %edge12074, %block75862, %block75842, %edge11880, %edge11874, %edge11673, %block73430, %block71785, %edge11102, %block68104, %block67542, %block67456, %block67436, %edge10629, %block64071, %edge10445, %block64016, %edge10405, %edge10373, %edge10302, %block62320, %block61591, %block61535, %block61001, %block60005, %edge9876, %block59922, %edge9741, %edge9716, %edge9710, %block59111, %edge9669, %block58820, %block58737, %edge9508, %block57691, %edge9418, %block57357, %block57294, %edge9242, %edge9190, %block56045, %block55283, %block55025, %block54988, %edge8928, %block54723, %block54675, %block53111, %block52879, %edge8516, %block51416, %block51401, %block51386, %block51371, %block50232, %block50189, %block49703, %edge8090, %edge8044, %block47804, %block47793, %block45966, %block45728, %edge7724, %block44529, %edge7681, %block44474, %edge7641, %block44054, %block43817, %block43154, %edge7410, %edge7396, %edge7386, %block42484, %block41982, %block41932, %block41439, %edge7103, %edge6999, %edge6993, %block39892, %edge6946, %block39394, %block39346, %block37732, %block35536, %edge6185, %block35287, %block34281, %block33929, %block33909, %edge5398, %edge5393, %block33335, %edge5345, %block32642, %block32624, %block31770, %block31754, %edge4949, %edge4857, %block29474, %block29458, %block28623, %block28605, %block27580, %block25847, %block25617, %block25299, %block25141, %edge3827, %block24005, %block23970, %block23684, %block23222, %block23187, %edge3519, %edge3452, %edge3398, %block21368, %edge2953, %block19745, %block19710, %block19424, %block18956, %block18921, %block18487, %edge2578, %edge2511, %edge2457, %block16670, %block15747, %block15712, %block15567, %block15376, %block13635, %block13600, %block13135, %block12170, %block12160, %block11840, %edge1349, %edge1165, %edge1122, %block11022, %edge1040, %edge1034, %block10520, %edge977, %block9715, %edge839, %block9660, %edge802, %block9605, %edge766, %block9550, %edge730, %block9495, %edge694, %block9440, %edge658, %block9385, %edge622, %edge609, %block9231, %edge570, %block8995, %edge514, %block8962, %edge415, %block8748, %block8707, %block8665, %block8591, %block8446, %block8422, %edge75, %block8389
  %common.ret.op = phi { i64, i64 } [ %t9414, %block99672 ], [ %t82, %block8389 ], [ %t89, %edge75 ], [ %t149, %block8422 ], [ %t215, %block8446 ], [ %t7345, %block75842 ], [ %t291, %block8591 ], [ %t352, %block8665 ], [ %t413, %block8707 ], [ %t477, %block8748 ], [ %t7528, %edge12505 ], [ %t491, %edge415 ], [ %t9717, %edge16315 ], [ { i64 9, i64 998 }, %edge2400 ], [ %t9222, %edge15571 ], [ { i64 1, i64 105 }, %edge462 ], [ %t585, %block8962 ], [ %t592, %edge514 ], [ zeroinitializer, %block100765 ], [ %t612, %block8995 ], [ %t625, %edge570 ], [ %t7686, %block85009 ], [ %t645, %block9231 ], [ %t656, %edge609 ], [ %t664, %edge622 ], [ %t9698, %block100709 ], [ %t684, %block9385 ], [ %t692, %edge658 ], [ %t9634, %block100317 ], [ %t712, %block9440 ], [ %t720, %edge694 ], [ %t9602, %block100179 ], [ %t740, %block9495 ], [ %t748, %edge730 ], [ %t9447, %block99860 ], [ %t768, %block9550 ], [ %t776, %edge766 ], [ %t9582, %edge16038 ], [ %t795, %block9605 ], [ %t803, %edge802 ], [ { i64 1, i64 6342 }, %edge12492 ], [ %t823, %block9660 ], [ %t830, %edge839 ], [ %t9568, %block100124 ], [ %t850, %block9715 ], [ { i64 1, i64 117 }, %edge477 ], [ %t5936, %edge9242 ], [ %t5987, %block57294 ], [ %t9216, %edge15559 ], [ %t885, %edge977 ], [ %t9315, %block99082 ], [ %t905, %block10520 ], [ %t957, %edge1034 ], [ %t961, %edge1040 ], [ %t6011, %block57357 ], [ { i64 1, i64 2535 }, %edge4917 ], [ %t1021, %block11022 ], [ %t1031, %edge1122 ], [ %t1042, %edge1165 ], [ %t9427, %edge15819 ], [ { i64 1, i64 7636 }, %edge14893 ], [ { i64 1, i64 4754 }, %edge9219 ], [ %t1317, %edge1349 ], [ %t1344, %block11840 ], [ %t9206, %block98703 ], [ %t1359, %block12160 ], [ %t1394, %block12170 ], [ %t9193, %block98636 ], [ %t9548, %edge15995 ], [ { i64 1, i64 7187 }, %edge14011 ], [ %t9052, %block96960 ], [ %t6247, %edge9710 ], [ %t1530, %block13135 ], [ %t6254, %edge9716 ], [ { i64 9, i64 591 }, %block13251 ], [ { i64 1, i64 6505 }, %edge12591 ], [ %t8953, %edge14791 ], [ %t8946, %block94357 ], [ %t8935, %block94290 ], [ %t1612, %block13600 ], [ %t1651, %block13635 ], [ %t6264, %edge9741 ], [ %t7897, %block87363 ], [ %t6317, %block59922 ], [ %t1744, %block15376 ], [ { i64 1, i64 5464 }, %edge10635 ], [ %t1792, %block15567 ], [ %t6902, %block67542 ], [ %t8817, %block93504 ], [ %t1849, %block15712 ], [ %t1888, %block15747 ], [ %t6074, %edge9418 ], [ %t7361, %block75862 ], [ %t1965, %block16670 ], [ { i64 9, i64 3238 }, %block37924 ], [ { i64 9, i64 998 }, %block16790 ], [ %t1987, %edge2457 ], [ %t2017, %edge2511 ], [ %t2037, %edge2578 ], [ %t2082, %block18487 ], [ %t8726, %edge14428 ], [ %t8695, %block92607 ], [ %t6755, %edge10629 ], [ %t7911, %block87387 ], [ %t2155, %block18921 ], [ %t2194, %block18956 ], [ %t2256, %block19424 ], [ %t6080, %block57691 ], [ %t7964, %block88851 ], [ %t6100, %edge9508 ], [ %t2323, %block19710 ], [ %t2362, %block19745 ], [ %t2368, %edge2953 ], [ %t9539, %block100069 ], [ { i64 1, i64 5988 }, %edge11605 ], [ { i64 1, i64 2879 }, %edge5477 ], [ { i64 1, i64 5081 }, %edge9971 ], [ %t2488, %block21368 ], [ { i64 1, i64 2215 }, %edge4387 ], [ { i64 9, i64 1546 }, %block21488 ], [ %t2510, %edge3398 ], [ %t2540, %edge3452 ], [ %t2560, %edge3519 ], [ %t7099, %block71785 ], [ %t7272, %edge11880 ], [ %t8555, %block92183 ], [ %t7156, %edge11673 ], [ %t2631, %block23187 ], [ %t2670, %block23222 ], [ %t2733, %block23684 ], [ { i64 1, i64 2432 }, %edge4678 ], [ %t8463, %block92031 ], [ %t8454, %edge14022 ], [ %t2800, %block23970 ], [ %t2839, %block24005 ], [ %t2845, %edge3827 ], [ %t9327, %block99162 ], [ { i64 1, i64 378 }, %edge1063 ], [ { i64 1, i64 45 }, %edge432 ], [ %t8440, %block91830 ], [ %t8376, %block91789 ], [ %t2951, %block25141 ], [ %t2999, %block25299 ], [ %t3055, %block25617 ], [ %t3105, %block25847 ], [ %t6393, %edge9876 ], [ %t6710, %block64071 ], [ %t8315, %block91747 ], [ %t8254, %block91711 ], [ %t3184, %block27580 ], [ { i64 1, i64 4348 }, %edge8562 ], [ { i64 1, i64 4695 }, %block4623 ], [ %t6398, %block60005 ], [ { i64 1, i64 441 }, %edge1179 ], [ %t6457, %block61001 ], [ %t8020, %edge13555 ], [ %t6510, %block61535 ], [ %t6528, %block61591 ], [ %t6595, %block62320 ], [ %t3294, %block28605 ], [ %t3334, %block28623 ], [ { i64 1, i64 8184 }, %edge15733 ], [ { i64 1, i64 2355 }, %edge4538 ], [ %t3395, %block29458 ], [ %t3437, %block29474 ], [ %t3451, %edge4857 ], [ { i64 1, i64 5871 }, %edge11482 ], [ %t6146, %block58737 ], [ %t6622, %edge10302 ], [ %t3485, %edge4949 ], [ %.mux.le, %common.ret.loopexit22434.split.loop.exit33132 ], [ { i64 1, i64 2643 }, %edge5027 ], [ %t3550, %block31754 ], [ %t3592, %block31770 ], [ { i64 1, i64 4101 }, %edge8105 ], [ %t3643, %block32624 ], [ %t3683, %block32642 ], [ { i64 1, i64 5311 }, %edge10388 ], [ %t6168, %block58820 ], [ %t8191, %block91569 ], [ %t3715, %edge5345 ], [ %t9519, %edge15955 ], [ %t3735, %block33335 ], [ %t3784, %edge5393 ], [ %t3788, %edge5398 ], [ { i64 1, i64 2385 }, %edge4539 ], [ %t6644, %edge10373 ], [ %t3851, %block33909 ], [ %t3867, %block33929 ], [ %t3910, %block34281 ], [ %t6690, %edge10445 ], [ %t6940, %block68104 ], [ %t8115, %block91127 ], [ { i64 1, i64 2726 }, %edge5146 ], [ %t4085, %block35287 ], [ %t9510, %block100014 ], [ %t7268, %edge11874 ], [ %t4223, %edge6185 ], [ %t4238, %block35536 ], [ %t4344, %block37732 ], [ %t6661, %edge10405 ], [ %t4426, %block39346 ], [ %t4447, %block39394 ], [ %t6186, %edge9669 ], [ %t4464, %edge6946 ], [ %t9359, %edge15713 ], [ %t4484, %block39892 ], [ %t4546, %edge6993 ], [ %t4553, %edge6999 ], [ %t4604, %edge7103 ], [ %t4651, %block41439 ], [ %t4705, %block41932 ], [ %t4724, %block41982 ], [ %t4763, %block42484 ], [ %t4769, %edge7386 ], [ %t4775, %edge7396 ], [ %t4781, %edge7410 ], [ %t4796, %block43154 ], [ %t4809, %block43817 ], [ %t4819, %block44054 ], [ { i64 1, i64 2282 }, %edge4484 ], [ %t4841, %edge7641 ], [ %t9490, %edge15915 ], [ %t4861, %block44474 ], [ %t7141, %block73430 ], [ %t4870, %edge7681 ], [ %t7588, %edge12631 ], [ %t4890, %block44529 ], [ %t4898, %edge7724 ], [ %t4939, %block45728 ], [ { i64 9, i64 3238 }, %edge6644 ], [ %t4961, %block45966 ], [ %t7448, %block78101 ], [ %t9461, %edge15862 ], [ %t5031, %block47793 ], [ %t5071, %block47804 ], [ %t5080, %edge8044 ], [ %t6206, %block59111 ], [ %t5125, %edge8090 ], [ { i64 9, i64 1546 }, %edge3341 ], [ %t7418, %block77240 ], [ %t7396, %block76577 ], [ { i64 1, i64 6696 }, %edge12810 ], [ { i64 1, i64 7399 }, %edge14435 ], [ %t5215, %block49703 ], [ { i64 1, i64 4890 }, %edge9572 ], [ %t5263, %block50189 ], [ %t5280, %block50232 ], [ %t5334, %block51371 ], [ %t5373, %block51386 ], [ %t5412, %block51401 ], [ %t5455, %block51416 ], [ %t5491, %edge8516 ], [ { i64 1, i64 6561 }, %edge12667 ], [ { i64 9, i64 591 }, %edge1758 ], [ %t6828, %block67436 ], [ %t5572, %block52879 ], [ %t5615, %block53111 ], [ %t6868, %block67456 ], [ %t5677, %block54675 ], [ %t5695, %block54723 ], [ %t7381, %edge12074 ], [ %t6681, %block64016 ], [ %t5708, %edge8928 ], [ %t9481, %block99915 ], [ %t5728, %block54988 ], [ %t5769, %block55025 ], [ %t5816, %block55283 ], [ %t7797, %edge12949 ], [ %t5873, %block56045 ], [ { i64 1, i64 5575 }, %edge10800 ], [ %t5894, %edge9190 ], [ %t6968, %edge11102 ], [ { i64 1, i64 4223 }, %block4170 ], [ { i64 1, i64 5558 }, %edge10737 ], [ { i64 1, i64 5533 }, %edge10697 ], [ { i64 1, i64 5476 }, %block5388 ], [ { i64 1, i64 5408 }, %edge10550 ], [ { i64 1, i64 4754 }, %edge9224 ], [ { i64 1, i64 4778 }, %edge9275 ], [ { i64 1, i64 4423 }, %edge8645 ], [ { i64 1, i64 4392 }, %edge8617 ], [ { i64 1, i64 2260 }, %edge4449 ], [ { i64 1, i64 2238 }, %edge4397 ], [ { i64 1, i64 6673 }, %edge12800 ], [ { i64 1, i64 6718 }, %edge12862 ], [ { i64 1, i64 6520 }, %edge12618 ], [ { i64 1, i64 2288 }, %edge4495 ], [ { i64 1, i64 2289 }, %edge4497 ], [ %t1249, %edge1281 ], [ { i64 1, i64 586 }, %edge1545 ], [ { i64 1, i64 626 }, %edge1781 ], [ { i64 1, i64 709 }, %edge1909 ], [ { i64 1, i64 756 }, %edge1937 ], [ { i64 1, i64 824 }, %edge1986 ], [ { i64 103425, i64 1948 }, %edge3986 ], [ { i64 1, i64 2268 }, %edge4464 ], [ { i64 1, i64 2269 }, %edge4467 ], [ { i64 1, i64 2271 }, %edge4468 ], [ { i64 1, i64 2280 }, %edge4479 ], [ { i64 1, i64 7148 }, %block7018 ], [ { i64 1, i64 6265 }, %edge12238 ], [ { i64 1, i64 5763 }, %edge11362 ], [ { i64 1, i64 6144 }, %edge12057 ], [ { i64 1, i64 5764 }, %edge11368 ], [ { i64 1, i64 6008 }, %edge11652 ], [ { i64 103427, i64 3000 }, %edge5941 ], [ { i64 1, i64 2837 }, %edge5422 ], [ { i64 1, i64 2559 }, %edge4931 ], [ { i64 1, i64 7084 }, %edge13634 ], [ { i64 1, i64 2518 }, %edge4885 ], [ { i64 1, i64 2756 }, %edge5147 ], [ { i64 1, i64 329 }, %edge952 ], [ { i64 1, i64 7245 }, %edge14166 ], [ { i64 1, i64 2782 }, %edge5300 ], [ { i64 1, i64 310 }, %edge16243 ], [ { i64 1, i64 319 }, %edge940 ], [ { i64 1, i64 7087 }, %edge13645 ], [ { i64 1, i64 7251 }, %edge14176 ], [ { i64 1, i64 7077 }, %edge13630 ], [ { i64 1, i64 6991 }, %edge13540 ], [ { i64 1, i64 7239 }, %edge14158 ], [ { i64 1, i64 7088 }, %edge13653 ], [ { i64 1, i64 2585 }, %edge4971 ]
  ret { i64, i64 } %common.ret.op

block8392:                                        ; preds = %edge49
  %v1863 = load i64, ptr %ss1, align 8
  %t71 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %v1864 = load i64, ptr %t71, align 8
  %t72 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %v1865 = load i64, ptr %t72, align 8
  %t73 = icmp eq i64 %v1407, %v1865
  %t75.not275 = icmp eq i64 %v1865, 0
  %t75.not = or i1 %t73, %t75.not275
  br i1 %t75.not, label %block8398, label %edge59

block8398:                                        ; preds = %block8392, %edge59
  %t79.not = or i1 %t26.not, %t73
  br i1 %t79.not, label %edge68, label %edge64

block8389:                                        ; preds = %edge64, %edge49, %edge38
  %v1831 = phi i64 [ %v1862, %edge49 ], [ 103427, %edge38 ], [ %v1874, %edge64 ]
  %t81 = insertvalue { i64, i64 } poison, i64 %v1831, 0
  %t82 = insertvalue { i64, i64 } %t81, i64 17, 1
  br label %common.ret

block8429:                                        ; preds = %edge85, %edge89, %edge98
  %v1925 = phi i64 [ %v1930, %edge89 ], [ %v1947, %edge98 ], [ 64, %edge85 ]
  %t116 = call i64 @llvm.umax.i64(i64 %v1925, i64 %v1919)
  br label %block8426

block8426:                                        ; preds = %edge82, %block8429
  %v1924 = phi i64 [ %t116, %block8429 ], [ %v1843344356, %edge82 ]
  br i1 %t75.not275347354, label %block8437, label %edge102

block8437:                                        ; preds = %edge102, %block8426
  %v1959 = ptrtoint ptr %ss1 to i64
  %v219528 = ptrtoint ptr %ss0 to i64
  %t122 = zext i64 %v1841343357 to i128
  %t123 = zext i64 %v1403 to i128
  %t124 = shl nuw i128 %t123, 64
  %v219529 = or disjoint i128 %t124, %t122
  store i128 %v219529, ptr %ss0, align 16
  %t126 = zext i64 %v1843344356 to i128
  %v219531 = or disjoint i128 %t126, 18446744073709551616
  %t130 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  store i128 %v219531, ptr %t130, align 16
  %t131 = zext i64 %v1406 to i128
  %t132 = zext i64 %v1846345355 to i128
  %t133 = shl nuw i128 %t132, 64
  %v219533 = or disjoint i128 %t133, %t131
  %t135 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  store i128 %v219533, ptr %t135, align 16
  %v1964 = call i64 @mech_compact_writable(i64 %v0, i64 %v219528, i64 1, i64 %v1924, i64 1, i64 1, i64 %v1959)
  %t136.not = icmp eq i64 %v1964, 0
  br i1 %t136.not, label %edge107, label %block8422

block8436:                                        ; preds = %edge102, %edge107
  %v1949 = phi i64 [ %v1965, %edge107 ], [ %v1841343357, %edge102 ]
  %v1951 = phi i64 [ %v1966, %edge107 ], [ %v1843344356, %edge102 ]
  %v1954 = phi i64 [ %v1967, %edge107 ], [ %v1846345355, %edge102 ]
  %v1969 = add i64 %v1949, %v1403
  %t139 = inttoptr i64 %v1969 to ptr
  store i8 1, ptr %t139, align 1
  %t140 = icmp eq i64 %v1846345355, %v1954
  %t142.not295 = icmp eq i64 %v1954, 0
  %t142.not = or i1 %t140, %t142.not295
  br i1 %t142.not, label %block8441, label %edge109

block8441:                                        ; preds = %block8436, %edge109
  %t146.not = or i1 %t75.not275347354, %t140
  br i1 %t146.not, label %edge118, label %edge114

block8422:                                        ; preds = %edge114, %block8437, %edge89, %edge81
  %v1913 = phi i64 [ %v1964, %block8437 ], [ 1, %edge81 ], [ 1, %edge89 ], [ %v1980, %edge114 ]
  %t148 = insertvalue { i64, i64 } poison, i64 %v1913, 0
  %t149 = insertvalue { i64, i64 } %t148, i64 23, 1
  br label %common.ret

block8454:                                        ; preds = %edge124, %edge128, %edge137
  %v1995 = phi i64 [ %v2000, %edge128 ], [ %v2017, %edge137 ], [ 64, %edge124 ]
  %t176 = call i64 @llvm.umax.i64(i64 %v1995, i64 %v1989)
  br label %block8451

block8451:                                        ; preds = %edge121, %block8454
  %v1994 = phi i64 [ %t176, %block8454 ], [ %v1951, %edge121 ]
  br i1 %t142.not295, label %block8462, label %edge141

block8462:                                        ; preds = %edge141, %block8451
  %v2029 = ptrtoint ptr %ss1 to i64
  %v219534 = ptrtoint ptr %ss0 to i64
  %t182 = zext i64 %v1949 to i128
  %t183 = zext i64 %v1919 to i128
  %t184 = shl nuw i128 %t183, 64
  %v219535 = or disjoint i128 %t184, %t182
  store i128 %v219535, ptr %ss0, align 16
  %t186 = zext i64 %v1951 to i128
  %v219537 = or disjoint i128 %t186, 18446744073709551616
  %t190 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  store i128 %v219537, ptr %t190, align 16
  %t192 = zext i64 %v1954 to i128
  %t193 = shl nuw i128 %t192, 64
  %v219539 = or disjoint i128 %t193, %t183
  %t195 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  store i128 %v219539, ptr %t195, align 16
  %v2034 = call i64 @mech_compact_writable(i64 %v0, i64 %v219534, i64 1, i64 %v1994, i64 1, i64 1, i64 %v2029)
  %t196.not = icmp eq i64 %v2034, 0
  br i1 %t196.not, label %edge146, label %block8446

block8482:                                        ; preds = %edge148, %edge160
  %t209.not = or i1 %t142.not295, %t203
  br i1 %t209.not, label %edge16309, label %edge163

block8446:                                        ; preds = %edge166, %block8462, %edge128, %edge118
  %v1982 = phi i64 [ %v2034, %block8462 ], [ 1, %edge118 ], [ 1, %edge128 ], [ %v2086, %edge166 ]
  %t214 = insertvalue { i64, i64 } poison, i64 %v1982, 0
  %t215 = insertvalue { i64, i64 } %t214, i64 24, 1
  br label %common.ret

block8548:                                        ; preds = %block8548.preheader, %edge432
  %v219175 = phi i64 [ %v2795, %edge432 ], [ %v1841343357, %block8548.preheader ]
  %v219176 = phi i64 [ %v2765, %edge432 ], [ %v1403, %block8548.preheader ]
  %v219177 = phi i64 [ %v2797, %edge432 ], [ %v1843344356, %block8548.preheader ]
  %v219179 = phi i64 [ %v2765, %edge432 ], [ %v1406, %block8548.preheader ]
  %v219180 = phi i64 [ %v2800, %edge432 ], [ %v1846345355, %block8548.preheader ]
  %v219181 = phi i64 [ %v2833, %edge432 ], [ 0, %block8548.preheader ]
  %v2210 = sub nsw i64 %v1397, %v219181
  %t220 = icmp ult i64 %v2210, 65535
  %spec.select = zext i1 %t220 to i8
  %spec.select301 = call i64 @llvm.umin.i64(i64 %v2210, i64 65535)
  %t229 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v219176, i64 1)
  %v2342 = extractvalue { i64, i1 } %t229, 0
  %t230 = extractvalue { i64, i1 } %t229, 1
  br i1 %t230, label %block8591, label %edge218

block8598:                                        ; preds = %edge221, %edge225, %edge234
  %v2348 = phi i64 [ %v2353, %edge225 ], [ %v2370, %edge234 ], [ 64, %edge221 ]
  %t255 = call i64 @llvm.umax.i64(i64 %v2348, i64 %v2342)
  br label %block8595

block8595:                                        ; preds = %edge218, %block8598
  %v2347 = phi i64 [ %t255, %block8598 ], [ %v219177, %edge218 ]
  %t256.not = icmp eq i64 %v219180, 0
  br i1 %t256.not, label %block8606, label %edge238

block8606:                                        ; preds = %edge238, %block8595
  %t261 = zext i64 %v219175 to i128
  %t262 = zext i64 %v219176 to i128
  %t263 = shl nuw i128 %t262, 64
  %v219541 = or disjoint i128 %t263, %t261
  store i128 %v219541, ptr %ss0, align 16
  %t265 = zext i64 %v219177 to i128
  %v219543 = or disjoint i128 %t265, 18446744073709551616
  store i128 %v219543, ptr %t269, align 16
  %t270 = zext i64 %v219179 to i128
  %t271 = zext i64 %v219180 to i128
  %t272 = shl nuw i128 %t271, 64
  %v219545 = or disjoint i128 %t272, %t270
  store i128 %v219545, ptr %t274, align 16
  %v2387 = call i64 @mech_compact_writable(i64 %v0, i64 %v219540, i64 1, i64 %v2347, i64 1, i64 1, i64 %v2382)
  %t275.not = icmp eq i64 %v2387, 0
  br i1 %t275.not, label %edge243, label %block8591

block8605:                                        ; preds = %edge238, %edge243
  %v2372 = phi i64 [ %v2388, %edge243 ], [ %v219175, %edge238 ]
  %v2374 = phi i64 [ %v2389, %edge243 ], [ %v219177, %edge238 ]
  %v2377 = phi i64 [ %v2390, %edge243 ], [ %v219180, %edge238 ]
  %v2392 = add i64 %v2372, %v219176
  %t278 = inttoptr i64 %v2392 to ptr
  store i8 %spec.select, ptr %t278, align 1
  %t279 = icmp eq i64 %v219180, %v2377
  %t281.not279 = icmp eq i64 %v2377, 0
  %t281.not = or i1 %t279, %t281.not279
  br i1 %t281.not, label %block8610, label %edge245

block8610:                                        ; preds = %block8605, %edge245
  %t285.not = or i1 %t256.not, %t279
  br i1 %t285.not, label %edge262, label %edge248

block8591:                                        ; preds = %edge251, %block8606, %edge225, %block8548
  %v2336 = phi i64 [ %v2387, %block8606 ], [ %v2403, %edge251 ], [ 1, %block8548 ], [ 1, %edge225 ]
  %t290 = insertvalue { i64, i64 } poison, i64 %v2336, 0
  %t291 = insertvalue { i64, i64 } %t290, i64 69, 1
  br label %common.ret

block8673:                                        ; preds = %edge268, %edge272, %edge281
  %v2549 = phi i64 [ %v2554, %edge272 ], [ %v2571, %edge281 ], [ 64, %edge268 ]
  %t318 = call i64 @llvm.umax.i64(i64 %v2549, i64 %v2543)
  br label %block8670

block8670:                                        ; preds = %edge265, %block8673
  %v2548 = phi i64 [ %t318, %block8673 ], [ %v2374, %edge265 ]
  br i1 %t281.not279, label %block8681, label %edge285

block8681:                                        ; preds = %edge285, %block8670
  %t324 = zext i64 %v2372 to i128
  %t325 = zext i64 %v2342 to i128
  %t326 = shl nuw i128 %t325, 64
  %v219547 = or disjoint i128 %t326, %t324
  store i128 %v219547, ptr %ss0, align 16
  %t328 = zext i64 %v2374 to i128
  %v219549 = or disjoint i128 %t328, 18446744073709551616
  store i128 %v219549, ptr %t269, align 16
  %t334 = zext i64 %v2377 to i128
  %t335 = shl nuw i128 %t334, 64
  %v219551 = or disjoint i128 %t335, %t325
  store i128 %v219551, ptr %t274, align 16
  %v2588 = call i64 @mech_compact_writable(i64 %v0, i64 %v219540, i64 1, i64 %v2548, i64 1, i64 1, i64 %v2382)
  %t338.not = icmp eq i64 %v2588, 0
  br i1 %t338.not, label %edge290, label %block8665

block8701:                                        ; preds = %edge292, %edge302
  %t349.not = or i1 %t281.not279, %t343
  br i1 %t349.not, label %edge311, label %edge307

block8665:                                        ; preds = %edge307, %block8681, %edge272, %edge262
  %v2530 = phi i64 [ %v2588, %block8681 ], [ 1, %edge262 ], [ 1, %edge272 ], [ %v2640, %edge307 ]
  %t351 = insertvalue { i64, i64 } poison, i64 %v2530, 0
  %t352 = insertvalue { i64, i64 } %t351, i64 80, 1
  br label %common.ret

block8715:                                        ; preds = %edge318, %edge322, %edge331
  %v2662 = phi i64 [ %v2667, %edge322 ], [ %v2684, %edge331 ], [ 64, %edge318 ]
  %t379 = call i64 @llvm.umax.i64(i64 %v2662, i64 %v2656)
  br label %block8712

block8712:                                        ; preds = %edge315, %block8715
  %v2661 = phi i64 [ %t379, %block8715 ], [ %v2575, %edge315 ]
  br i1 %t345.not283, label %block8723, label %edge335

block8723:                                        ; preds = %edge335, %block8712
  %t385 = zext i64 %v2573 to i128
  %t386 = zext i64 %v2543 to i128
  %t387 = shl nuw i128 %t386, 64
  %v219553 = or disjoint i128 %t387, %t385
  store i128 %v219553, ptr %ss0, align 16
  %t389 = zext i64 %v2575 to i128
  %v219555 = or disjoint i128 %t389, 18446744073709551616
  store i128 %v219555, ptr %t269, align 16
  %t395 = zext i64 %v2578 to i128
  %t396 = shl nuw i128 %t395, 64
  %v219557 = or disjoint i128 %t396, %t386
  store i128 %v219557, ptr %t274, align 16
  %v2701 = call i64 @mech_compact_writable(i64 %v0, i64 %v219540, i64 1, i64 %v2661, i64 1, i64 1, i64 %v2382)
  %t399.not = icmp eq i64 %v2701, 0
  br i1 %t399.not, label %edge340, label %block8707

block8743:                                        ; preds = %edge342, %edge352
  %t410.not = or i1 %t345.not283, %t404
  br i1 %t410.not, label %edge361, label %edge357

block8707:                                        ; preds = %edge357, %block8723, %edge322, %edge311
  %v2649 = phi i64 [ %v2701, %block8723 ], [ 1, %edge311 ], [ 1, %edge322 ], [ %v2753, %edge357 ]
  %t412 = insertvalue { i64, i64 } poison, i64 %v2649, 0
  %t413 = insertvalue { i64, i64 } %t412, i64 82, 1
  br label %common.ret

block8756:                                        ; preds = %edge368, %edge372, %edge381
  %v2771 = phi i64 [ %v2776, %edge372 ], [ %v2793, %edge381 ], [ 64, %edge368 ]
  %t444 = call i64 @llvm.umax.i64(i64 %v2771, i64 %v2765)
  br label %block8753

block8753:                                        ; preds = %edge365, %block8756
  %v2770 = phi i64 [ %t444, %block8756 ], [ %v2688, %edge365 ]
  br i1 %t406.not287, label %block8764, label %edge385

block8764:                                        ; preds = %edge385, %block8753
  %t450 = zext i64 %v2686 to i128
  %t451 = zext i64 %v2656 to i128
  %t452 = shl nuw i128 %t451, 64
  %v219559 = or disjoint i128 %t452, %t450
  store i128 %v219559, ptr %ss0, align 16
  %t454 = zext i64 %v2688 to i128
  %v219561 = or disjoint i128 %t454, 18446744073709551616
  store i128 %v219561, ptr %t269, align 16
  %t460 = zext i64 %v2691 to i128
  %t461 = shl nuw i128 %t460, 64
  %v219563 = or disjoint i128 %t461, %t451
  store i128 %v219563, ptr %t274, align 16
  %v2810 = call i64 @mech_compact_writable(i64 %v0, i64 %v219540, i64 1, i64 %v2770, i64 1, i64 1, i64 %v2382)
  %t464.not = icmp eq i64 %v2810, 0
  br i1 %t464.not, label %edge390, label %block8748

block8763:                                        ; preds = %edge385, %edge390
  %v2795 = phi i64 [ %v2811, %edge390 ], [ %v2686, %edge385 ]
  %v2797 = phi i64 [ %v2812, %edge390 ], [ %v2688, %edge385 ]
  %v2800 = phi i64 [ %v2813, %edge390 ], [ %v2691, %edge385 ]
  %t467.not = icmp eq i64 %t417, 0
  br i1 %t467.not, label %block8768, label %edge392

block8768:                                        ; preds = %block8763, %edge392
  %t468 = icmp eq i64 %v2691, %v2800
  %t470.not291 = icmp eq i64 %v2800, 0
  %t470.not = or i1 %t468, %t470.not291
  br i1 %t470.not, label %block8770, label %edge395

block8770:                                        ; preds = %block8768, %edge395
  %t474.not = or i1 %t406.not287, %t468
  br i1 %t474.not, label %edge404, label %edge400

block8748:                                        ; preds = %edge400, %block8764, %edge372, %edge363, %edge361
  %v2755 = phi i64 [ %v2810, %block8764 ], [ 103427, %edge361 ], [ 1, %edge363 ], [ 1, %edge372 ], [ %v2828, %edge400 ]
  %t476 = insertvalue { i64, i64 } poison, i64 %v2755, 0
  %t477 = insertvalue { i64, i64 } %t476, i64 83, 1
  br label %common.ret

block8965:                                        ; preds = %edge485
  %v3200 = load i64, ptr %ss1, align 8
  %t571 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %v3201 = load i64, ptr %t571, align 8
  %t572 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %v3202 = load i64, ptr %t572, align 8
  %t573 = icmp eq i64 %v1407, %v3202
  %t575.not44 = icmp eq i64 %v3202, 0
  %t575.not = or i1 %t573, %t575.not44
  br i1 %t575.not, label %block8971, label %edge495

block8971:                                        ; preds = %block8965, %edge495
  %t579.not = or i1 %t26.not, %t573
  br i1 %t579.not, label %edge534, label %edge498

block8962:                                        ; preds = %edge501, %edge485, %edge482
  %v3168 = phi i64 [ %v3199, %edge485 ], [ 103427, %edge482 ], [ %v3211, %edge501 ]
  %t584 = insertvalue { i64, i64 } poison, i64 %v3168, 0
  %t585 = insertvalue { i64, i64 } %t584, i64 118, 1
  br label %common.ret

block8995:                                        ; preds = %edge534
  %t611 = insertvalue { i64, i64 } poison, i64 %v3276, 0
  %t612 = insertvalue { i64, i64 } %t611, i64 124, 1
  br label %common.ret

block9231:                                        ; preds = %edge517
  %t644 = insertvalue { i64, i64 } poison, i64 %v3760, 0
  %t645 = insertvalue { i64, i64 } %t644, i64 156, 1
  br label %common.ret

block9385:                                        ; preds = %edge642
  %t683 = insertvalue { i64, i64 } poison, i64 %v4148, 0
  %t684 = insertvalue { i64, i64 } %t683, i64 180, 1
  br label %common.ret

block9440:                                        ; preds = %edge678
  %t711 = insertvalue { i64, i64 } poison, i64 %v4367, 0
  %t712 = insertvalue { i64, i64 } %t711, i64 193, 1
  br label %common.ret

block9495:                                        ; preds = %edge714
  %t739 = insertvalue { i64, i64 } poison, i64 %v4606, 0
  %t740 = insertvalue { i64, i64 } %t739, i64 206, 1
  br label %common.ret

block9550:                                        ; preds = %edge750
  %t767 = insertvalue { i64, i64 } poison, i64 %v4849, 0
  %t768 = insertvalue { i64, i64 } %t767, i64 219, 1
  br label %common.ret

block9605:                                        ; preds = %edge786
  %t794 = insertvalue { i64, i64 } poison, i64 %v5116, 0
  %t795 = insertvalue { i64, i64 } %t794, i64 232, 1
  br label %common.ret

block9660:                                        ; preds = %edge822
  %t822 = insertvalue { i64, i64 } poison, i64 %v5383, 0
  %t823 = insertvalue { i64, i64 } %t822, i64 245, 1
  br label %common.ret

block9715:                                        ; preds = %edge859
  %t849 = insertvalue { i64, i64 } poison, i64 %v5662, 0
  %t850 = insertvalue { i64, i64 } %t849, i64 258, 1
  br label %common.ret

block10120:                                       ; preds = %edge16243, %edge842
  %v219193 = phi i64 [ %v3178364375, %edge842 ], [ %v206768, %edge16243 ]
  %v219194 = phi i64 [ %v1403, %edge842 ], [ %v206782, %edge16243 ]
  %v219195 = phi i64 [ %v3180365374, %edge842 ], [ %v206796, %edge16243 ]
  %v219197 = phi i64 [ %v1406, %edge842 ], [ %v206782, %edge16243 ]
  %v219198 = phi i64 [ %v3183366373, %edge842 ], [ %v206838, %edge16243 ]
  %v219199 = phi i64 [ 0, %edge842 ], [ %v206858, %edge16243 ]
  %v219200 = phi i64 [ 0, %edge842 ], [ %v206874, %edge16243 ]
  %v219201 = phi i64 [ %v3279, %edge842 ], [ %v54915, %edge16243 ]
  %v219203 = phi i64 [ %v3280, %edge842 ], [ %v54917, %edge16243 ]
  %v219204 = phi i64 [ %v3281, %edge842 ], [ %v54920, %edge16243 ]
  %v219205 = phi i64 [ %v3763, %edge842 ], [ %v205562, %edge16243 ]
  %v219206 = phi i64 [ %v3764, %edge842 ], [ %v205564, %edge16243 ]
  %v219207 = phi i64 [ %v3765, %edge842 ], [ %v205567, %edge16243 ]
  %v219208 = phi i64 [ %v4051, %edge842 ], [ %v7457922180, %edge16243 ]
  %v219209 = phi i64 [ 0, %edge842 ], [ %v7458022184, %edge16243 ]
  %v219210 = phi i64 [ %v4052, %edge842 ], [ %v7458122187, %edge16243 ]
  %v219211 = phi i64 [ 1, %edge842 ], [ %v7458222191, %edge16243 ]
  %v219212 = phi i64 [ 0, %edge842 ], [ %v7458322195, %edge16243 ]
  %v219213 = phi i64 [ %v4053, %edge842 ], [ %v7458422199, %edge16243 ]
  %v219214 = phi i64 [ 0, %edge842 ], [ %v54927, %edge16243 ]
  %v219215 = phi i64 [ 0, %edge842 ], [ %v54928, %edge16243 ]
  %v219216 = phi i64 [ 0, %edge842 ], [ %v54929, %edge16243 ]
  %v219217 = phi i64 [ 0, %edge842 ], [ %v205929, %edge16243 ]
  %v219218 = phi i64 [ 0, %edge842 ], [ %v54931, %edge16243 ]
  %v219219 = phi i64 [ %spec.select10, %edge842 ], [ %v54932, %edge16243 ]
  %v219220 = phi i64 [ 0, %edge842 ], [ %v206116, %edge16243 ]
  %v219221 = phi i64 [ %v4151, %edge842 ], [ %v219067, %edge16243 ]
  %v219223 = phi i64 [ %v4152, %edge842 ], [ %v219069, %edge16243 ]
  %v219226 = phi i64 [ %v4153, %edge842 ], [ %v219119, %edge16243 ]
  %v219227 = phi i64 [ %v4370, %edge842 ], [ %v147323, %edge16243 ]
  %v219229 = phi i64 [ %v4371, %edge842 ], [ %v147329, %edge16243 ]
  %v219232 = phi i64 [ %v4372, %edge842 ], [ %v140768, %edge16243 ]
  %v219233 = phi i64 [ %v4609, %edge842 ], [ %v206119, %edge16243 ]
  %v219234 = phi i64 [ %v4610, %edge842 ], [ %v206121, %edge16243 ]
  %v219236 = phi i64 [ %v4611, %edge842 ], [ %v206124, %edge16243 ]
  %v219237 = phi i64 [ %v4852, %edge842 ], [ %v206131, %edge16243 ]
  %v219238 = phi i64 [ %v4853, %edge842 ], [ %v206133, %edge16243 ]
  %v219240 = phi i64 [ %v4854, %edge842 ], [ %v206136, %edge16243 ]
  %v219241 = phi i64 [ %v5119, %edge842 ], [ %v54945, %edge16243 ]
  %v219242 = phi i64 [ %v5121, %edge842 ], [ %v54950, %edge16243 ]
  %v219243 = phi i64 [ 0, %edge842 ], [ %v54957, %edge16243 ]
  %v219244 = phi i64 [ %v5665, %edge842 ], [ %v206706, %edge16243 ]
  %v219245 = phi i64 [ %v5666, %edge842 ], [ %v206708, %edge16243 ]
  %v219247 = phi i64 [ %v5667, %edge842 ], [ %v206711, %edge16243 ]
  %v219248 = phi i64 [ %v5386, %edge842 ], [ %v206718, %edge16243 ]
  %v219249 = phi i64 [ %v5387, %edge842 ], [ %v206720, %edge16243 ]
  %v219251 = phi i64 [ %v5388, %edge842 ], [ %v206723, %edge16243 ]
  %v219252 = phi i64 [ 0, %edge842 ], [ %v206733, %edge16243 ]
  %v219253 = phi i1 [ false, %edge842 ], [ %t3962, %edge16243 ]
  %v219256 = phi i64 [ 0, %edge842 ], [ %v210497, %edge16243 ]
  %v219257 = phi i64 [ 0, %edge842 ], [ %v210594, %edge16243 ]
  %v7234 = sub i64 %v1397, %v219217
  %t858 = icmp ult i64 %v7234, 305000
  br i1 %t858, label %edge936, label %edge940

block10520:                                       ; preds = %edge997
  %t904 = insertvalue { i64, i64 } poison, i64 %v8090, 0
  %t905 = insertvalue { i64, i64 } %t904, i64 343, 1
  br label %common.ret

block364:                                         ; preds = %block364.preheader, %edge1128
  %v8476 = phi i64 [ %v9001, %edge1128 ], [ %v8095, %block364.preheader ]
  %v8510 = phi i64 [ %v8998, %edge1128 ], [ %v8094, %block364.preheader ]
  %v8513 = phi i64 [ %v9256, %edge1128 ], [ 0, %block364.preheader ]
  %v8514 = phi i64 [ %v8996, %edge1128 ], [ %v8093, %block364.preheader ]
  %t912.not = icmp eq i64 %v8476, 0
  br i1 %t912.not, label %edge1057, label %edge1023

block10708:                                       ; preds = %edge1031, %edge1030
  %t951.pn = phi { i64, i64 } [ %t951, %edge1030 ], [ %t952, %edge1031 ]
  %v8482 = extractvalue { i64, i64 } %t951.pn, 0
  %t953.not = icmp eq i64 %v8482, 0
  br i1 %t953.not, label %edge1036, label %edge1034

trap976:                                          ; preds = %edge1078
  call void @llvm.trap()
  unreachable

ok976:                                            ; preds = %edge1078
  %v8949 = udiv i64 %v8747, %v1399
  %v8950 = mul i64 %v8949, %v1399
  %v8951 = sub nuw i64 %v8747, %v8950
  %v8952 = mul i64 %v8951, %v1400
  %v8953 = add i64 %v8952, %v8949
  br label %block11014

block11014:                                       ; preds = %edge1073, %ok976
  %v8948 = phi i64 [ %v8953, %ok976 ], [ %v8747, %edge1073 ]
  %v8955 = add i64 %v8948, %v1396
  %t977 = inttoptr i64 %v8955 to ptr
  %t978 = load i8, ptr %t977, align 1
  %v8956 = zext i8 %t978 to i64
  br label %edge1088

block11027:                                       ; preds = %edge1092, %edge1091
  %t994 = zext i64 %v8514 to i128
  %v219589 = or disjoint i128 %t994, 4722366482869645213696
  store i128 %v219589, ptr %ss0, align 16
  %t998 = zext i64 %v8510 to i128
  %v219591 = or disjoint i128 %t998, 18446744073709551616
  store i128 %v219591, ptr %t1002, align 16
  %t1004 = zext i64 %v8476 to i128
  %t1005 = shl nuw i128 %t1004, 64
  %v219593 = or disjoint i128 %t1005, 256
  store i128 %v219593, ptr %t1007, align 16
  %v9017 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v8510, i64 1, i64 1, i64 %v3194)
  %t1008.not = icmp eq i64 %v9017, 0
  br i1 %t1008.not, label %edge1097, label %block11022

block11026:                                       ; preds = %edge1092, %edge1097
  %t1011.pre-phi = phi ptr [ %t985, %edge1092 ], [ %.pre21806, %edge1097 ]
  %v9002 = phi i64 [ %v8514, %edge1092 ], [ %v9018, %edge1097 ]
  %v9004 = phi i64 [ %v8510, %edge1092 ], [ %v9019, %edge1097 ]
  %v9007 = phi i64 [ %v8476, %edge1092 ], [ %v9020, %edge1097 ]
  store i8 1, ptr %t1011.pre-phi, align 1
  br label %block11025

block11025:                                       ; preds = %edge1088, %block11026
  %v8996 = phi i64 [ %v9002, %block11026 ], [ %v8514, %edge1088 ]
  %v8998 = phi i64 [ %v9004, %block11026 ], [ %v8510, %edge1088 ]
  %v9001 = phi i64 [ %v9007, %block11026 ], [ %v8476, %edge1088 ]
  %t1012 = icmp eq i64 %v8476, %v9001
  %t1014.not47 = icmp eq i64 %v9001, 0
  %t1014.not = or i1 %t1012, %t1014.not47
  br i1 %t1014.not, label %block11031, label %edge1100

block11031:                                       ; preds = %block11025, %edge1100
  %t1018.not = or i1 %t912.not, %t1012
  br i1 %t1018.not, label %edge1128, label %edge1105

block11022:                                       ; preds = %edge1105, %block11027
  %v8984 = phi i64 [ %v9017, %block11027 ], [ %v9031, %edge1105 ]
  %t1020 = insertvalue { i64, i64 } poison, i64 %v8984, 0
  %t1021 = insertvalue { i64, i64 } %t1020, i64 393, 1
  br label %common.ret

block475:                                         ; preds = %block475.backedge, %block475.outer
  %v10789 = phi i64 [ %v10789.ph, %block475.outer ], [ %v36630646, %block475.backedge ]
  %v10790 = phi i64 [ %v10790.ph, %block475.outer ], [ %v4033512111224, %block475.backedge ]
  %v10792 = phi i64 [ %v10792.ph, %block475.outer ], [ %v40037, %block475.backedge ]
  %v10794 = phi i64 [ %v10794.ph, %block475.outer ], [ %v40049, %block475.backedge ]
  %v10796 = phi i64 [ %v10796.ph, %block475.outer ], [ %v36627645, %block475.backedge ]
  %v10798 = phi i64 [ %v10798.ph, %block475.outer ], [ %v4033212091225, %block475.backedge ]
  %v10802 = phi i64 [ %v10802.ph, %block475.outer ], [ %v40034, %block475.backedge ]
  %v10806 = phi i64 [ %v10806.ph, %block475.outer ], [ %v40046, %block475.backedge ]
  %v10809 = phi i64 [ %v10809.ph, %block475.outer ], [ %v36649651, %block475.backedge ]
  %v10810 = phi i64 [ %v10810.ph, %block475.outer ], [ %v36652652, %block475.backedge ]
  %v10811 = phi i64 [ %v10811.ph, %block475.outer ], [ %v41285, %block475.backedge ]
  %v10812 = phi i64 [ %v10812.ph, %block475.outer ], [ %spec.select1, %block475.backedge ]
  %v10813 = phi i64 [ %v10813.ph, %block475.outer ], [ %v41335, %block475.backedge ]
  %v10814 = phi i64 [ %v10814.ph, %block475.outer ], [ %v40316, %block475.backedge ]
  %v10815 = phi i64 [ %v10815.ph, %block475.outer ], [ %v218798, %block475.backedge ]
  %v10816 = phi i64 [ %v10816.ph, %block475.outer ], [ %v40056, %block475.backedge ]
  %v10817 = phi i64 [ %v10817.ph, %block475.outer ], [ %v40059, %block475.backedge ]
  %v10818 = phi i64 [ %v10818.ph, %block475.outer ], [ 0, %block475.backedge ]
  %v10819 = phi i64 [ %v10819.ph, %block475.outer ], [ %v218797, %block475.backedge ]
  %v10820 = phi i64 [ %v10820.ph, %block475.outer ], [ %v218796, %block475.backedge ]
  %v10821 = phi i64 [ %v10821.ph, %block475.outer ], [ %v218795, %block475.backedge ]
  %v10822 = phi i64 [ %v10822.ph, %block475.outer ], [ %v218803, %block475.backedge ]
  %v10823 = phi i64 [ %v10823.ph, %block475.outer ], [ %v218804, %block475.backedge ]
  %v10824 = phi i64 [ %v10824.ph, %block475.outer ], [ %v36625644, %block475.backedge ]
  %v10825 = phi i64 [ %v10825.ph, %block475.outer ], [ %v4033012061226, %block475.backedge ]
  %v10829 = phi i64 [ %v10829.ph, %block475.outer ], [ %v40032, %block475.backedge ]
  %v10831 = phi i64 [ %v10831.ph, %block475.outer ], [ %v40044, %block475.backedge ]
  %v12013 = phi i64 [ %v12013.ph, %block475.outer ], [ %v218771, %block475.backedge ]
  %v12417 = phi i64 [ %v12417.ph, %block475.outer ], [ %v218773, %block475.backedge ]
  %t1075.not = icmp eq i64 %v10789, 0
  %or.cond304 = select i1 %v1062051.not1294, i1 true, i1 %t1075.not
  br i1 %or.cond304, label %edge1406, label %edge1242

block11893:                                       ; preds = %edge1314
  br i1 %t23.not, label %edge1326, label %edge1320

block11840:                                       ; preds = %edge1356, %edge1344, %edge1338, %edge1332, %edge1326, %edge1320, %edge1314, %edge1294
  %v10612 = phi i64 [ %v10722, %edge1344 ], [ %v10718, %edge1338 ], [ %v10714, %edge1332 ], [ %v10710, %edge1326 ], [ %v10706, %edge1320 ], [ %v10702, %edge1314 ], [ %v10698, %edge1294 ], [ %v10759, %edge1356 ]
  %t1343 = insertvalue { i64, i64 } poison, i64 %v10612, 0
  %t1344 = insertvalue { i64, i64 } %t1343, i64 475, 1
  br label %common.ret

block12160:                                       ; preds = %edge1423
  %t1358 = insertvalue { i64, i64 } poison, i64 %v11402, 0
  %t1359 = insertvalue { i64, i64 } %t1358, i64 511, 1
  br label %common.ret

block12171:                                       ; preds = %edge1442, %edge1444
  br i1 %t1075.not, label %edge1496, label %edge1453

block12170:                                       ; preds = %edge1453, %edge1441, %edge1438
  %v11418 = phi i64 [ 103427, %edge1441 ], [ %v11477, %edge1453 ], [ %v11458, %edge1438 ]
  %t1393 = insertvalue { i64, i64 } poison, i64 %v11418, 0
  %t1394 = insertvalue { i64, i64 } %t1393, i64 514, 1
  br label %common.ret

block535:                                         ; preds = %edge1406, %edge1423, %edge1496
  %v218769 = phi i64 [ %v11830, %edge1423 ], [ %v11830, %edge1496 ], [ 0, %edge1406 ]
  %v218770 = phi i64 [ %v11998, %edge1423 ], [ %v11998, %edge1496 ], [ %v10811, %edge1406 ]
  %v218771 = phi i64 [ %v12011, %edge1423 ], [ %v12011, %edge1496 ], [ %v10812, %edge1406 ]
  %v218772 = phi i64 [ %v12211, %edge1423 ], [ %v12211, %edge1496 ], [ %v11039, %edge1406 ]
  %v218773 = phi i64 [ %v12415, %edge1423 ], [ %v12415, %edge1496 ], [ %v10813, %edge1406 ]
  %v218774 = phi i64 [ %v12659, %edge1423 ], [ %v12659, %edge1496 ], [ %v10818, %edge1406 ]
  %v218781 = phi i64 [ %v11463, %edge1423 ], [ 0, %edge1496 ], [ %v10789, %edge1406 ]
  %v218782 = phi i64 [ %v14681, %edge1423 ], [ %v14681, %edge1496 ], [ %v10809, %edge1406 ]
  %v218783 = phi i64 [ %v11461, %edge1423 ], [ %v11461, %edge1496 ], [ %v10824, %edge1406 ]
  %v218784 = phi i64 [ %v11462, %edge1423 ], [ %v11462, %edge1496 ], [ %v10796, %edge1406 ]
  %v218785 = phi i64 [ %v18190, %edge1423 ], [ %v18190, %edge1496 ], [ %v10814, %edge1406 ]
  %v218788 = phi i64 [ %v18669, %edge1423 ], [ %v18669, %edge1496 ], [ %v10810, %edge1406 ]
  %v218789 = phi i64 [ %v37162, %edge1423 ], [ %v37162, %edge1496 ], [ %v10817, %edge1406 ]
  %v218794 = phi i64 [ %v37925, %edge1423 ], [ %v37925, %edge1496 ], [ %v10816, %edge1406 ]
  %v218795 = phi i64 [ %v38009, %edge1423 ], [ %v38009, %edge1496 ], [ %v10821, %edge1406 ]
  %v218796 = phi i64 [ %v41626, %edge1423 ], [ %v41626, %edge1496 ], [ %v10820, %edge1406 ]
  %v218797 = phi i64 [ %v41853, %edge1423 ], [ %v41853, %edge1496 ], [ %v10819, %edge1406 ]
  %v218798 = phi i64 [ %v42190, %edge1423 ], [ %v42190, %edge1496 ], [ %v10815, %edge1406 ]
  %v218803 = phi i64 [ %v48120, %edge1423 ], [ %v48120, %edge1496 ], [ %v10822, %edge1406 ]
  %v218804 = phi i64 [ %v48176, %edge1423 ], [ %v48176, %edge1496 ], [ %v10823, %edge1406 ]
  %t1446 = icmp eq i64 %v218769, 1
  %spec.select1 = select i1 %t1446, i64 %v218770, i64 %v218771
  %spec.select19 = call i64 @llvm.umin.i64(i64 %v218772, i64 32768)
  %v12400 = add i64 %spec.select19, %v218770
  %v41335 = select i1 %t1446, i64 %v12400, i64 %v218773
  %spec.select18 = call i64 @llvm.umin.i64(i64 %v218772, i64 258)
  %v12690 = lshr i64 %v218774, 32
  %t1457 = icmp ne i64 %v12690, 0
  %t1458 = icmp ugt i64 %v218772, 4
  br i1 %t1458, label %edge1545, label %edge3984

block13203:                                       ; preds = %edge1680, %edge1677
  %t1503.not = icmp eq i64 %v219198, 0
  br i1 %t1503.not, label %block13207, label %edge1686

block13207:                                       ; preds = %edge1686, %block13203
  %t1505.not = icmp eq i64 %v219213, 0
  br i1 %t1505.not, label %block13211, label %edge1692

block13211:                                       ; preds = %edge1692, %block13207
  %t1507.not = icmp eq i64 %v219226, 0
  br i1 %t1507.not, label %block13215, label %edge1698

block13215:                                       ; preds = %edge1698, %block13211
  %t1509.not = icmp eq i64 %v219232, 0
  br i1 %t1509.not, label %block13219, label %edge1704

block13219:                                       ; preds = %edge1704, %block13215
  %t1511.not = icmp eq i64 %v219242, 0
  br i1 %t1511.not, label %block13227, label %edge1710

block13227:                                       ; preds = %block13219, %edge1710
  %t1515.not = icmp eq i64 %v10790, 0
  br i1 %t1515.not, label %block13231, label %edge1722

block13231:                                       ; preds = %edge1722, %block13227
  br i1 %t23.not, label %block13235, label %edge1728

block13235:                                       ; preds = %edge1728, %block13231
  %t1519.not = icmp eq i64 %v10791.ph.ph, 0
  br i1 %t1519.not, label %block13239, label %edge1734

block13239:                                       ; preds = %edge1734, %block13235
  %t1521.not = icmp eq i64 %v10792, 0
  br i1 %t1521.not, label %block13243, label %edge1740

block13243:                                       ; preds = %edge1740, %block13239
  br i1 %t3136.not, label %block13247, label %edge1746

block13247:                                       ; preds = %edge1746, %block13243
  %t1525.not = icmp eq i64 %v10794, 0
  br i1 %t1525.not, label %block13251, label %edge1752

block13251:                                       ; preds = %edge1752, %block13247
  %t1527.not = icmp eq i64 %v218781, 0
  br i1 %t1527.not, label %common.ret, label %edge1758

block13135:                                       ; preds = %edge1758, %edge1752, %edge1746, %edge1740, %edge1734, %edge1728, %edge1722, %edge1710, %edge1704, %edge1698, %edge1692, %edge1686, %edge1680
  %v13673 = phi i64 [ %v13791, %edge1752 ], [ %v13743, %edge1680 ], [ %v13747, %edge1686 ], [ %v13751, %edge1692 ], [ %v13755, %edge1698 ], [ %v13759, %edge1704 ], [ %v13763, %edge1710 ], [ %v13795, %edge1758 ], [ %v13771, %edge1722 ], [ %v13775, %edge1728 ], [ %v13779, %edge1734 ], [ %v13783, %edge1740 ], [ %v13787, %edge1746 ]
  %t1529 = insertvalue { i64, i64 } poison, i64 %v13673, 0
  %t1530 = insertvalue { i64, i64 } %t1529, i64 589, 1
  br label %common.ret

block13625:                                       ; preds = %edge1835.thread, %edge1835
  %0 = phi i16 [ %12, %edge1835 ], [ %v14829, %edge1835.thread ]
  %t1585 = zext i64 %v218783 to i128
  %v219711 = or disjoint i128 %t1585, 4835703278458516698824704
  store i128 %v219711, ptr %ss0, align 16
  %t1589 = zext i64 %v218784 to i128
  %v219713 = or disjoint i128 %t1589, 18446744073709551616
  store i128 %v219713, ptr %t1002, align 16
  %t1595 = zext i64 %v218781 to i128
  %t1596 = shl nuw i128 %t1595, 64
  %v219715 = or disjoint i128 %t1596, 262144
  store i128 %v219715, ptr %t1007, align 16
  %v14896 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v218784, i64 2, i64 2, i64 %v3194)
  %t1599.not = icmp eq i64 %v14896, 0
  br i1 %t1599.not, label %block13624, label %block13600

block13624:                                       ; preds = %block13625
  %v14899 = load i64, ptr %t607, align 8
  %v14898 = load i64, ptr %t606, align 8
  %v14897 = load i64, ptr %ss1, align 8
  %v14900 = add i64 %v14897, %v14776
  %t1602 = inttoptr i64 %v14900 to ptr
  store i16 %0, ptr %t1602, align 1
  br label %block13623

block13623:                                       ; preds = %edge1835.thread, %block13624
  %v14875 = phi i64 [ %v14897, %block13624 ], [ %v218783, %edge1835.thread ]
  %v14877 = phi i64 [ %v14898, %block13624 ], [ %v218784, %edge1835.thread ]
  %v14880 = phi i64 [ %v14899, %block13624 ], [ %v218781, %edge1835.thread ]
  %t1603 = icmp eq i64 %v218781, %v14880
  %t1605.not52 = icmp eq i64 %v14880, 0
  %t1605.not = or i1 %t1603, %t1605.not52
  br i1 %t1605.not, label %block13629, label %edge1847

block13629:                                       ; preds = %block13623, %edge1847
  %t1609.not = or i1 %t1559.not, %t1603
  br i1 %t1609.not, label %edge1858, label %edge1852

block13600:                                       ; preds = %edge1852, %block13625
  %v14813 = phi i64 [ %v14896, %block13625 ], [ %v14910, %edge1852 ]
  %t1611 = insertvalue { i64, i64 } poison, i64 %v14813, 0
  %t1612 = insertvalue { i64, i64 } %t1611, i64 660, 1
  br label %common.ret

block13640:                                       ; preds = %edge1858.thread, %edge1862, %edge1861
  %v1487521824218392205222067 = phi i64 [ %v14875, %edge1861 ], [ %v14875, %edge1862 ], [ %v218783, %edge1858.thread ]
  %v1487721825218382205322066 = phi i64 [ %v14877, %edge1861 ], [ %v14877, %edge1862 ], [ %v218784, %edge1858.thread ]
  %v1488021826218372205422064 = phi i64 [ %v14880, %edge1861 ], [ %v14880, %edge1862 ], [ 0, %edge1858.thread ]
  %t1605.not5221829218362205522063 = phi i1 [ true, %edge1861 ], [ false, %edge1862 ], [ true, %edge1858.thread ]
  %t1624 = zext i64 %v1487521824218392205222067 to i128
  %v219717 = or disjoint i128 %t1624, 4835703278458516698824704
  store i128 %v219717, ptr %ss0, align 16
  %t1628 = zext i64 %v1487721825218382205322066 to i128
  %v219719 = or disjoint i128 %t1628, 18446744073709551616
  store i128 %v219719, ptr %t1002, align 16
  %t1634 = zext i64 %v1488021826218372205422064 to i128
  %t1635 = shl nuw i128 %t1634, 64
  %v219721 = or disjoint i128 %t1635, 262144
  store i128 %v219721, ptr %t1007, align 16
  %v14955 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v1487721825218382205322066, i64 2, i64 2, i64 %v3194)
  %t1638.not = icmp eq i64 %v14955, 0
  br i1 %t1638.not, label %block13638, label %block13635

block13638:                                       ; preds = %block13640
  %v14956 = load i64, ptr %ss1, align 8
  %v14957 = load i64, ptr %t606, align 8
  %v14958 = load i64, ptr %t607, align 8
  %.pre21803 = add i64 %v14956, %v14804
  %.pre21804 = inttoptr i64 %.pre21803 to ptr
  store i16 %t1550, ptr %.pre21804, align 1
  %t1642 = icmp eq i64 %v1488021826218372205422064, %v14958
  %t1644.not54 = icmp eq i64 %v14958, 0
  %t1644.not = or i1 %t1642, %t1644.not54
  br i1 %t1644.not, label %block13644, label %edge1870

block13644:                                       ; preds = %block13638, %edge1870
  %t1648.not = or i1 %t1605.not5221829218362205522063, %t1642
  br i1 %t1648.not, label %block662, label %edge1875

block13635:                                       ; preds = %edge1875, %block13640
  %v14922 = phi i64 [ %v14955, %block13640 ], [ %v14969, %edge1875 ]
  %t1650 = insertvalue { i64, i64 } poison, i64 %v14922, 0
  %t1651 = insertvalue { i64, i64 } %t1650, i64 662, 1
  br label %common.ret

block662.sink.split:                              ; preds = %edge1862, %edge1826
  %t1615.sink = phi ptr [ %t1555, %edge1826 ], [ %t1615, %edge1862 ]
  %v18809.ph = phi i64 [ %v218783, %edge1826 ], [ %v14875, %edge1862 ]
  %v18832.ph = phi i64 [ %v218781, %edge1826 ], [ %v14880, %edge1862 ]
  %v18851.ph = phi i64 [ %v218784, %edge1826 ], [ %v14877, %edge1862 ]
  store i16 %t1550, ptr %t1615.sink, align 1
  br label %block662

block662:                                         ; preds = %block662.sink.split, %edge1858.thread, %edge1858, %block13644, %edge1875
  %v18809 = phi i64 [ %v218783, %edge1858.thread ], [ %v14956, %edge1875 ], [ %v14956, %block13644 ], [ %v14875, %edge1858 ], [ %v18809.ph, %block662.sink.split ]
  %v18832 = phi i64 [ 0, %edge1858.thread ], [ %v14958, %edge1875 ], [ %v14958, %block13644 ], [ %v14880, %edge1858 ], [ %v18832.ph, %block662.sink.split ]
  %v18851 = phi i64 [ %v218784, %edge1858.thread ], [ %v14957, %edge1875 ], [ %v14957, %block13644 ], [ %v14877, %edge1858 ], [ %v18851.ph, %block662.sink.split ]
  br i1 %t1457, label %edge2077, label %edge1885

block15381:                                       ; preds = %edge2029, %edge2028
  %t1717 = zext i64 %v10825 to i128
  %v219723 = or disjoint i128 %t1719, %t1717
  store i128 %v219723, ptr %ss0, align 16
  %t1721 = zext i64 %v10798 to i128
  %v219725 = or disjoint i128 %t1721, 18446744073709551616
  store i128 %v219725, ptr %t1002, align 16
  %t1727 = zext i64 %v10790 to i128
  %t1728 = shl nuw i128 %t1727, 64
  %v219727 = or disjoint i128 %t1728, %t1163
  store i128 %v219727, ptr %t1007, align 16
  %v18377 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v10798, i64 4, i64 4, i64 %v3194)
  %t1731.not = icmp eq i64 %v18377, 0
  br i1 %t1731.not, label %edge2034, label %block15376

block15380:                                       ; preds = %edge2029, %edge2034
  %t1734.pre-phi = phi ptr [ %t1708, %edge2029 ], [ %.pre21802, %edge2034 ]
  %v18362 = phi i64 [ %v10825, %edge2029 ], [ %v18378, %edge2034 ]
  %v18364 = phi i64 [ %v10798, %edge2029 ], [ %v18379, %edge2034 ]
  %v18367 = phi i64 [ %v10790, %edge2029 ], [ %v18380, %edge2034 ]
  %v18382 = trunc nuw i64 %v18174 to i32
  store i32 %v18382, ptr %t1734.pre-phi, align 1
  br label %block15379

block15379:                                       ; preds = %edge2025, %block15380
  %v18356 = phi i64 [ %v18362, %block15380 ], [ %v10825, %edge2025 ]
  %v18358 = phi i64 [ %v18364, %block15380 ], [ %v10798, %edge2025 ]
  %v18361 = phi i64 [ %v18367, %block15380 ], [ %v10790, %edge2025 ]
  %t1735 = icmp eq i64 %v10790, %v18361
  %t1737.not56 = icmp eq i64 %v18361, 0
  %t1737.not = or i1 %t1735, %t1737.not56
  br i1 %t1737.not, label %block15385, label %edge2037

block15385:                                       ; preds = %block15379, %edge2037
  %t1741.not57 = icmp eq i64 %v10790, 0
  %t1741.not = or i1 %t1741.not57, %t1735
  br i1 %t1741.not, label %edge2015, label %edge2042

block15376:                                       ; preds = %edge2042, %block15381, %edge2017
  %v18220 = phi i64 [ %v18377, %block15381 ], [ 103425, %edge2017 ], [ %v18391, %edge2042 ]
  %t1743 = insertvalue { i64, i64 } poison, i64 %v18220, 0
  %t1744 = insertvalue { i64, i64 } %t1743, i64 862, 1
  br label %common.ret

block15572:                                       ; preds = %edge2085, %edge2084
  %t1765 = zext i64 %v18809 to i128
  %v219729 = or disjoint i128 %t1765, 4835703278458516698824704
  store i128 %v219729, ptr %ss0, align 16
  %t1769 = zext i64 %v18851 to i128
  %v219731 = or disjoint i128 %t1769, 18446744073709551616
  store i128 %v219731, ptr %t1002, align 16
  %t1775 = zext i64 %v18832 to i128
  %t1776 = shl nuw i128 %t1775, 64
  %v219733 = or disjoint i128 %t1776, 262144
  store i128 %v219733, ptr %t1007, align 16
  %v18896 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v18851, i64 2, i64 2, i64 %v3194)
  %t1779.not = icmp eq i64 %v18896, 0
  br i1 %t1779.not, label %edge2090, label %block15567

block15571:                                       ; preds = %edge2085, %edge2090
  %t1782.pre-phi = phi ptr [ %t1750, %edge2085 ], [ %.pre21800, %edge2090 ]
  %v18881 = phi i64 [ %v18809, %edge2085 ], [ %v18897, %edge2090 ]
  %v18883 = phi i64 [ %v18851, %edge2085 ], [ %v18898, %edge2090 ]
  %v18886 = phi i64 [ %v18832, %edge2085 ], [ %v18899, %edge2090 ]
  store i16 %14, ptr %t1782.pre-phi, align 1
  br label %block15570

block15570:                                       ; preds = %edge2077, %block15571
  %v18875 = phi i64 [ %v18881, %block15571 ], [ %v18809, %edge2077 ]
  %v18877 = phi i64 [ %v18883, %block15571 ], [ %v18851, %edge2077 ]
  %v18880 = phi i64 [ %v18886, %block15571 ], [ %v18832, %edge2077 ]
  %t1783 = icmp eq i64 %v18832, %v18880
  %t1785.not58 = icmp eq i64 %v18880, 0
  %t1785.not = or i1 %t1783, %t1785.not58
  br i1 %t1785.not, label %block15576, label %edge2093

block15576:                                       ; preds = %block15570, %edge2093
  %t1789.not59 = icmp eq i64 %v18832, 0
  %t1789.not = or i1 %t1789.not59, %t1783
  br i1 %t1789.not, label %edge2102, label %edge2098

block15567:                                       ; preds = %edge2098, %block15572
  %v18829 = phi i64 [ %v18896, %block15572 ], [ %v18910, %edge2098 ]
  %t1791 = insertvalue { i64, i64 } poison, i64 %v18829, 0
  %t1792 = insertvalue { i64, i64 } %t1791, i64 890, 1
  br label %common.ret

block15737:                                       ; preds = %edge2127.thread, %edge2127
  %v19248513515 = phi i64 [ %v19248, %edge2127 ], [ %v19206, %edge2127.thread ]
  %t1822 = zext i64 %v18875 to i128
  %v219735 = or disjoint i128 %t1822, 4835703278458516698824704
  store i128 %v219735, ptr %ss0, align 16
  %t1826 = zext i64 %v18877 to i128
  %v219737 = or disjoint i128 %t1826, 18446744073709551616
  store i128 %v219737, ptr %t1002, align 16
  %t1832 = zext i64 %v18880 to i128
  %t1833 = shl nuw i128 %t1832, 64
  %v219739 = or disjoint i128 %t1833, 262144
  store i128 %v219739, ptr %t1007, align 16
  %v19275 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v18877, i64 2, i64 2, i64 %v3194)
  %t1836.not = icmp eq i64 %v19275, 0
  br i1 %t1836.not, label %block15736, label %block15712

block15736:                                       ; preds = %block15737
  %v19278 = load i64, ptr %t607, align 8
  %v19277 = load i64, ptr %t606, align 8
  %v19276 = load i64, ptr %ss1, align 8
  %v19279 = add i64 %v19276, %v19248513515
  %t1839 = inttoptr i64 %v19279 to ptr
  store i16 0, ptr %t1839, align 1
  br label %block15735

block15735:                                       ; preds = %edge2127.thread, %block15736
  %v19254 = phi i64 [ %v19276, %block15736 ], [ %v18875, %edge2127.thread ]
  %v19256 = phi i64 [ %v19277, %block15736 ], [ %v18877, %edge2127.thread ]
  %v19259 = phi i64 [ %v19278, %block15736 ], [ %v18880, %edge2127.thread ]
  %t1840 = icmp eq i64 %v18880, %v19259
  %t1842.not90 = icmp eq i64 %v19259, 0
  %t1842.not = or i1 %t1840, %t1842.not90
  br i1 %t1842.not, label %block15741, label %edge2139

block15741:                                       ; preds = %block15735, %edge2139
  %t1846.not = or i1 %t1785.not58, %t1840
  br i1 %t1846.not, label %edge2150, label %edge2144

block15712:                                       ; preds = %edge2144, %block15737
  %v19196 = phi i64 [ %v19275, %block15737 ], [ %v19289, %edge2144 ]
  %t1848 = insertvalue { i64, i64 } poison, i64 %v19196, 0
  %t1849 = insertvalue { i64, i64 } %t1848, i64 901, 1
  br label %common.ret

block15752:                                       ; preds = %edge2150.thread, %edge2154, %edge2153
  %v1925421864218792207722095 = phi i64 [ %v19254, %edge2153 ], [ %v19254, %edge2154 ], [ %v18875, %edge2150.thread ]
  %v1925621865218782207822094 = phi i64 [ %v19256, %edge2153 ], [ %v19256, %edge2154 ], [ %v18877, %edge2150.thread ]
  %v1925921866218772207922092 = phi i64 [ %v19259, %edge2153 ], [ %v19259, %edge2154 ], [ 0, %edge2150.thread ]
  %t1842.not9021869218762208022091 = phi i1 [ true, %edge2153 ], [ false, %edge2154 ], [ true, %edge2150.thread ]
  %v193492208122089 = phi i64 [ %v19349, %edge2153 ], [ %v19349, %edge2154 ], [ %v1934922072, %edge2150.thread ]
  %t1861 = zext i64 %v1925421864218792207722095 to i128
  %v219741 = or disjoint i128 %t1861, 4835703278458516698824704
  store i128 %v219741, ptr %ss0, align 16
  %t1865 = zext i64 %v1925621865218782207822094 to i128
  %v219743 = or disjoint i128 %t1865, 18446744073709551616
  store i128 %v219743, ptr %t1002, align 16
  %t1871 = zext i64 %v1925921866218772207922092 to i128
  %t1872 = shl nuw i128 %t1871, 64
  %v219745 = or disjoint i128 %t1872, 262144
  store i128 %v219745, ptr %t1007, align 16
  %v19376 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v1925621865218782207822094, i64 2, i64 2, i64 %v3194)
  %t1875.not = icmp eq i64 %v19376, 0
  br i1 %t1875.not, label %block15750, label %block15747

block15750:                                       ; preds = %block15752
  %v19377 = load i64, ptr %ss1, align 8
  %v19378 = load i64, ptr %t606, align 8
  %v19379 = load i64, ptr %t607, align 8
  %.pre21797 = add i64 %v19377, %v193492208122089
  %.pre21798 = inttoptr i64 %.pre21797 to ptr
  store i16 0, ptr %.pre21798, align 1
  %t1879 = icmp eq i64 %v1925921866218772207922092, %v19379
  %t1881.not92 = icmp eq i64 %v19379, 0
  %t1881.not = or i1 %t1879, %t1881.not92
  br i1 %t1881.not, label %block15756, label %edge2162

block15756:                                       ; preds = %block15750, %edge2162
  %t1885.not = or i1 %t1842.not9021869218762208022091, %t1879
  br i1 %t1885.not, label %edge3980, label %edge2167

block15747:                                       ; preds = %edge2167, %block15752
  %v19343 = phi i64 [ %v19376, %block15752 ], [ %v19390, %edge2167 ]
  %t1887 = insertvalue { i64, i64 } poison, i64 %v19343, 0
  %t1888 = insertvalue { i64, i64 } %t1887, i64 903, 1
  br label %common.ret

block913:                                         ; preds = %block913.preheader, %edge2826
  %v19844 = phi i64 [ %v26933, %edge2826 ], [ %v18875, %block913.preheader ]
  %v19846 = phi i64 [ %v26935, %edge2826 ], [ %v18877, %block913.preheader ]
  %v19849 = phi i64 [ %v26938, %edge2826 ], [ %v18880, %block913.preheader ]
  %v19856 = phi i64 [ %v26103554, %edge2826 ], [ %v18646, %block913.preheader ]
  %v19858 = phi i64 [ %v26105555, %edge2826 ], [ %v18648, %block913.preheader ]
  %v19861 = phi i64 [ %v26108556, %edge2826 ], [ %v18651, %block913.preheader ]
  %v19868 = phi i64 [ %spec.select20557, %edge2826 ], [ %v18658, %block913.preheader ]
  %v19901 = phi i64 [ %v26902, %edge2826 ], [ %v19006, %block913.preheader ]
  %v19904 = phi i64 [ %v26905, %edge2826 ], [ %v19011, %block913.preheader ]
  %v19907 = phi i64 [ %v26981, %edge2826 ], [ %v18771, %block913.preheader ]
  %v19910 = phi i64 [ %v27177, %edge2826 ], [ 300, %block913.preheader ]
  %v19913 = phi i64 [ %v26908, %edge2826 ], [ 0, %block913.preheader ]
  %v19916 = phi i64 [ %v26911, %edge2826 ], [ 0, %block913.preheader ]
  %v19919 = phi i64 [ %spec.select21, %edge2826 ], [ 0, %block913.preheader ]
  %v19922 = phi i64 [ %v26146553, %edge2826 ], [ 3, %block913.preheader ]
  %t1894 = icmp ult i64 %v19919, %spec.select18
  br i1 %t1894, label %edge2227, label %block19858

trap1911:                                         ; preds = %edge2256
  call void @llvm.trap()
  unreachable

ok1911:                                           ; preds = %edge2256
  %v20533 = udiv i64 %v20335, %v1399
  %v20534 = mul i64 %v20533, %v1399
  %v20535 = sub i64 %v20335, %v20534
  %v20536 = mul i64 %v20535, %v1400
  %v20537 = add i64 %v20536, %v20533
  br label %block16384

block16384:                                       ; preds = %edge2251, %ok1911
  %v20532 = phi i64 [ %v20537, %ok1911 ], [ %v20335, %edge2251 ]
  %v20539 = add i64 %v20532, %v1396
  %t1912 = inttoptr i64 %v20539 to ptr
  %t1913 = load i8, ptr %t1912, align 1
  %v20540 = zext i8 %t1913 to i64
  br label %edge2249

trap1921:                                         ; preds = %edge2275
  call void @llvm.trap()
  unreachable

ok1921:                                           ; preds = %edge2275
  %v20759 = udiv i64 %v20562, %v1399
  %v20760 = mul i64 %v20759, %v1399
  %v20761 = sub i64 %v20562, %v20760
  %v20762 = mul i64 %v20761, %v1400
  %v20763 = add i64 %v20762, %v20759
  br label %block16527

block16527:                                       ; preds = %edge2270, %ok1921
  %v20758 = phi i64 [ %v20763, %ok1921 ], [ %v20562, %edge2270 ]
  %v20765 = add i64 %v20758, %v1396
  %t1922 = inttoptr i64 %v20765 to ptr
  %t1923 = load i8, ptr %t1922, align 1
  %v20766 = zext i8 %t1923 to i64
  br label %edge2268

block16738:                                       ; preds = %edge2316, %edge2313
  %t1936.not = icmp eq i64 %v219198, 0
  br i1 %t1936.not, label %block16742, label %edge2322

block16742:                                       ; preds = %edge2322, %block16738
  %t1938.not = icmp eq i64 %v219213, 0
  br i1 %t1938.not, label %block16746, label %edge2328

block16746:                                       ; preds = %edge2328, %block16742
  %t1940.not = icmp eq i64 %v219226, 0
  br i1 %t1940.not, label %block16750, label %edge2334

block16750:                                       ; preds = %edge2334, %block16746
  %t1942.not = icmp eq i64 %v219232, 0
  br i1 %t1942.not, label %block16754, label %edge2340

block16754:                                       ; preds = %edge2340, %block16750
  %t1944.not = icmp eq i64 %v219242, 0
  br i1 %t1944.not, label %block16762, label %edge2346

block16762:                                       ; preds = %block16754, %edge2346
  br i1 %t23.not, label %block16766, label %edge2358

block16766:                                       ; preds = %edge2358, %block16762
  %t1950.not = icmp eq i64 %v10791.ph.ph, 0
  br i1 %t1950.not, label %block16770, label %edge2364

block16770:                                       ; preds = %edge2364, %block16766
  %t1952.not = icmp eq i64 %v10792, 0
  br i1 %t1952.not, label %block16774, label %edge2370

block16774:                                       ; preds = %edge2370, %block16770
  br i1 %t3136.not, label %block16778, label %edge2376

block16778:                                       ; preds = %edge2376, %block16774
  %t1956.not = icmp eq i64 %v10794, 0
  br i1 %t1956.not, label %block16782, label %edge2382

block16782:                                       ; preds = %edge2382, %block16778
  %t1958.not = icmp eq i64 %v19849, 0
  br i1 %t1958.not, label %block16786, label %edge2388

block16786:                                       ; preds = %edge2388, %block16782
  %t1960.not = icmp eq i64 %v19861, 0
  br i1 %t1960.not, label %block16790, label %edge2394

block16790:                                       ; preds = %edge2394, %block16786
  br i1 %t23.not, label %common.ret, label %edge2400

block16670:                                       ; preds = %edge2400, %edge2394, %edge2388, %edge2382, %edge2376, %edge2370, %edge2364, %edge2358, %edge2346, %edge2340, %edge2334, %edge2328, %edge2322, %edge2316
  %v20996 = phi i64 [ %v21118, %edge2394 ], [ %v21066, %edge2316 ], [ %v21070, %edge2322 ], [ %v21074, %edge2328 ], [ %v21078, %edge2334 ], [ %v21082, %edge2340 ], [ %v21086, %edge2346 ], [ %v21122, %edge2400 ], [ %v21094, %edge2358 ], [ %v21098, %edge2364 ], [ %v21102, %edge2370 ], [ %v21106, %edge2376 ], [ %v21110, %edge2382 ], [ %v21114, %edge2388 ]
  %t1964 = insertvalue { i64, i64 } poison, i64 %v20996, 0
  %t1965 = insertvalue { i64, i64 } %t1964, i64 996, 1
  br label %common.ret

block1017:                                        ; preds = %block1017.preheader, %edge2517
  %v22059 = phi i64 [ %v22922, %edge2517 ], [ %v21750, %block1017.preheader ]
  %t1978.not = icmp ult i64 %v22059, %spec.select18
  br i1 %t1978.not, label %edge2465, label %edge2451

trap1995:                                         ; preds = %edge2478
  call void @llvm.trap()
  unreachable

ok1995:                                           ; preds = %edge2478
  %v22476 = udiv i64 %v22270, %v1399
  %v22477 = mul i64 %v22476, %v1399
  %v22478 = sub nuw i64 %v22270, %v22477
  %v22479 = mul i64 %v22478, %v1400
  %v22480 = add i64 %v22479, %v22476
  br label %block17271

block17271:                                       ; preds = %edge2473, %ok1995
  %v22475 = phi i64 [ %v22480, %ok1995 ], [ %v22270, %edge2473 ]
  %v22482 = add i64 %v22475, %v1396
  %t1996 = inttoptr i64 %v22482 to ptr
  %t1997 = load i8, ptr %t1996, align 1
  %v22483 = zext i8 %t1997 to i64
  br label %edge2471

trap2005:                                         ; preds = %edge2497
  call void @llvm.trap()
  unreachable

ok2005:                                           ; preds = %edge2497
  %v22709 = udiv i64 %v22507, %v1399
  %v22710 = mul i64 %v22709, %v1399
  %v22711 = sub nuw i64 %v22507, %v22710
  %v22712 = mul i64 %v22711, %v1400
  %v22713 = add i64 %v22712, %v22709
  br label %block17418

block17418:                                       ; preds = %edge2492, %ok2005
  %v22708 = phi i64 [ %v22713, %ok2005 ], [ %v22507, %edge2492 ]
  %v22715 = add i64 %v22708, %v1396
  %t2006 = inttoptr i64 %v22715 to ptr
  %t2007 = load i8, ptr %t2006, align 1
  %v22716 = zext i8 %t2007 to i64
  br label %edge2490

block18492:                                       ; preds = %edge2620, %edge2619
  %t2055 = zext i64 %v19856 to i128
  %v219747 = or disjoint i128 %t1719, %t2055
  store i128 %v219747, ptr %ss0, align 16
  %t2059 = zext i64 %v19858 to i128
  %v219749 = or disjoint i128 %t2059, 18446744073709551616
  store i128 %v219749, ptr %t1002, align 16
  %t2065 = zext i64 %v19861 to i128
  %t2066 = shl nuw i128 %t2065, 64
  %v219751 = or disjoint i128 %t2066, %t1163
  store i128 %v219751, ptr %t1007, align 16
  %v25164 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v19858, i64 4, i64 4, i64 %v3194)
  %t2069.not = icmp eq i64 %v25164, 0
  br i1 %t2069.not, label %edge2625, label %block18487

block18491:                                       ; preds = %edge2620, %edge2625
  %t2072.pre-phi = phi ptr [ %t2046, %edge2620 ], [ %.pre21796, %edge2625 ]
  %v25149 = phi i64 [ %v19856, %edge2620 ], [ %v25165, %edge2625 ]
  %v25151 = phi i64 [ %v19858, %edge2620 ], [ %v25166, %edge2625 ]
  %v25154 = phi i64 [ %v19861, %edge2620 ], [ %v25167, %edge2625 ]
  %v25169 = trunc nuw i64 %v25032 to i32
  store i32 %v25169, ptr %t2072.pre-phi, align 1
  br label %block18490

block18490:                                       ; preds = %edge2616, %block18491
  %v25143 = phi i64 [ %v25149, %block18491 ], [ %v19856, %edge2616 ]
  %v25145 = phi i64 [ %v25151, %block18491 ], [ %v19858, %edge2616 ]
  %v25148 = phi i64 [ %v25154, %block18491 ], [ %v19861, %edge2616 ]
  %t2073 = icmp eq i64 %v19861, %v25148
  %t2075.not76 = icmp eq i64 %v25148, 0
  %t2075.not = or i1 %t2073, %t2075.not76
  br i1 %t2075.not, label %block18496, label %edge2628

block18496:                                       ; preds = %block18490, %edge2628
  %t2079.not77 = icmp eq i64 %v19861, 0
  %t2079.not = or i1 %t2079.not77, %t2073
  br i1 %t2079.not, label %edge2605, label %edge2633

block18487:                                       ; preds = %edge2633, %block18492, %edge2607
  %v25059 = phi i64 [ %v25164, %block18492 ], [ 103425, %edge2607 ], [ %v25178, %edge2633 ]
  %t2081 = insertvalue { i64, i64 } poison, i64 %v25059, 0
  %t2082 = insertvalue { i64, i64 } %t2081, i64 1250, 1
  br label %common.ret

block18946:                                       ; preds = %edge2692.thread, %edge2692
  %v25975563565 = phi i64 [ %v25975, %edge2692 ], [ %v25933, %edge2692.thread ]
  %t2128 = zext i64 %v19844 to i128
  %v219753 = or disjoint i128 %t2128, 4835703278458516698824704
  store i128 %v219753, ptr %ss0, align 16
  %t2132 = zext i64 %v19846 to i128
  %v219755 = or disjoint i128 %t2132, 18446744073709551616
  store i128 %v219755, ptr %t1002, align 16
  %t2138 = zext i64 %v19849 to i128
  %t2139 = shl nuw i128 %t2138, 64
  %v219757 = or disjoint i128 %t2139, 262144
  store i128 %v219757, ptr %t1007, align 16
  %v26002 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v19846, i64 2, i64 2, i64 %v3194)
  %t2142.not = icmp eq i64 %v26002, 0
  br i1 %t2142.not, label %block18944, label %block18921

block18944.thread:                                ; preds = %edge2692, %edge2692.thread
  %t2148.not8621903 = icmp eq i64 %v19849, 0
  br label %block18950

block18944:                                       ; preds = %block18946
  %v26005 = load i64, ptr %t607, align 8
  %v26004 = load i64, ptr %t606, align 8
  %v26003 = load i64, ptr %ss1, align 8
  %v26006 = add i64 %v26003, %v25975563565
  %t2145 = inttoptr i64 %v26006 to ptr
  store i16 %t2093, ptr %t2145, align 1
  %t2146 = icmp eq i64 %v19849, %v26005
  %t2148.not86 = icmp eq i64 %v26005, 0
  %t2148.not = or i1 %t2146, %t2148.not86
  br i1 %t2148.not, label %block18950, label %edge2704

block18950:                                       ; preds = %block18944.thread, %block18944, %edge2704
  %t2148.not8621911 = phi i1 [ %t2148.not8621903, %block18944.thread ], [ %t2148.not86, %block18944 ], [ %t2148.not86, %edge2704 ]
  %t214621910 = phi i1 [ true, %block18944.thread ], [ %t2146, %block18944 ], [ %t2146, %edge2704 ]
  %v2598621908 = phi i64 [ %v19849, %block18944.thread ], [ %v26005, %block18944 ], [ %v26005, %edge2704 ]
  %v2598321907 = phi i64 [ %v19846, %block18944.thread ], [ %v26004, %block18944 ], [ %v26004, %edge2704 ]
  %v2598121906 = phi i64 [ %v19844, %block18944.thread ], [ %v26003, %block18944 ], [ %v26003, %edge2704 ]
  %t2152.not = or i1 %t2102.not, %t214621910
  br i1 %t2152.not, label %edge2715, label %edge2709

block18921:                                       ; preds = %edge2709, %block18946
  %v25911 = phi i64 [ %v26002, %block18946 ], [ %v26016, %edge2709 ]
  %t2154 = insertvalue { i64, i64 } poison, i64 %v25911, 0
  %t2155 = insertvalue { i64, i64 } %t2154, i64 1286, 1
  br label %common.ret

block18961:                                       ; preds = %edge2719, %edge2718
  %t2167 = zext i64 %v2598121906 to i128
  %v219759 = or disjoint i128 %t2167, 4835703278458516698824704
  store i128 %v219759, ptr %ss0, align 16
  %t2171 = zext i64 %v2598321907 to i128
  %v219761 = or disjoint i128 %t2171, 18446744073709551616
  store i128 %v219761, ptr %t1002, align 16
  %t2177 = zext i64 %v2598621908 to i128
  %t2178 = shl nuw i128 %t2177, 64
  %v219763 = or disjoint i128 %t2178, 262144
  store i128 %v219763, ptr %t1007, align 16
  %v26080 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v2598321907, i64 2, i64 2, i64 %v3194)
  %t2181.not = icmp eq i64 %v26080, 0
  br i1 %t2181.not, label %edge2724, label %block18956

block18959:                                       ; preds = %edge2724, %edge2719
  %t2184.pre-phi = phi ptr [ %t2158, %edge2719 ], [ %.pre21794, %edge2724 ]
  %v26065 = phi i64 [ %v2598121906, %edge2719 ], [ %v26081, %edge2724 ]
  %v26067 = phi i64 [ %v2598321907, %edge2719 ], [ %v26082, %edge2724 ]
  %v26070 = phi i64 [ %v2598621908, %edge2719 ], [ %v26083, %edge2724 ]
  store i16 %t2099, ptr %t2184.pre-phi, align 1
  %t2185 = icmp eq i64 %v2598621908, %v26070
  %t2187.not88 = icmp eq i64 %v26070, 0
  %t2187.not = or i1 %t2185, %t2187.not88
  br i1 %t2187.not, label %block18965, label %edge2727

block18965:                                       ; preds = %block18959, %edge2727
  %t2191.not = or i1 %t2148.not8621911, %t2185
  br i1 %t2191.not, label %block19858, label %edge2732

block18956:                                       ; preds = %edge2732, %block18961
  %v26047 = phi i64 [ %v26080, %block18961 ], [ %v26094, %edge2732 ]
  %t2193 = insertvalue { i64, i64 } poison, i64 %v26047, 0
  %t2194 = insertvalue { i64, i64 } %t2193, i64 1288, 1
  br label %common.ret

trap2202:                                         ; preds = %edge2754
  call void @llvm.trap()
  unreachable

ok2202:                                           ; preds = %edge2754
  %v26389 = udiv i64 %v26180, %v1399
  %v26390 = mul i64 %v26389, %v1399
  %v26391 = sub i64 %v26180, %v26390
  %v26392 = mul i64 %v26391, %v1400
  %v26393 = add i64 %v26392, %v26389
  br label %block19127

block19127:                                       ; preds = %edge2749, %ok2202
  %v26388 = phi i64 [ %v26393, %ok2202 ], [ %v26180, %edge2749 ]
  %v26395 = add i64 %v26388, %v1396
  %t2203 = inttoptr i64 %v26395 to ptr
  %t2204 = load i8, ptr %t2203, align 1
  %v26396 = zext i8 %t2204 to i64
  br label %edge2747

trap2212:                                         ; preds = %edge2773
  call void @llvm.trap()
  unreachable

ok2212:                                           ; preds = %edge2773
  %v26653 = udiv i64 %v26456, %v1399
  %v26654 = mul i64 %v26653, %v1399
  %v26655 = sub i64 %v26456, %v26654
  %v26656 = mul i64 %v26655, %v1400
  %v26657 = add i64 %v26656, %v26653
  br label %edge2766

block19429:                                       ; preds = %edge2807, %edge2806
  %t2229 = zext i64 %v19844 to i128
  %v219765 = or disjoint i128 %t2229, 4835703278458516698824704
  store i128 %v219765, ptr %ss0, align 16
  %t2233 = zext i64 %v19846 to i128
  %v219767 = or disjoint i128 %t2233, 18446744073709551616
  store i128 %v219767, ptr %t1002, align 16
  %t2239 = zext i64 %v19849 to i128
  %t2240 = shl nuw i128 %t2239, 64
  %v219769 = or disjoint i128 %t2240, 262144
  store i128 %v219769, ptr %t1007, align 16
  %v26954 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v19846, i64 2, i64 2, i64 %v3194)
  %t2243.not = icmp eq i64 %v26954, 0
  br i1 %t2243.not, label %edge2812, label %block19424

block19428:                                       ; preds = %edge2807, %edge2812
  %t2246.pre-phi = phi ptr [ %t2220, %edge2807 ], [ %.pre21792, %edge2812 ]
  %v26939 = phi i64 [ %v19844, %edge2807 ], [ %v26955, %edge2812 ]
  %v26941 = phi i64 [ %v19846, %edge2807 ], [ %v26956, %edge2812 ]
  %v26944 = phi i64 [ %v19849, %edge2807 ], [ %v26957, %edge2812 ]
  %v26959 = trunc nuw i64 %v19907 to i16
  store i16 %v26959, ptr %t2246.pre-phi, align 1
  br label %block19427

block19427:                                       ; preds = %edge2803, %block19428
  %v26933 = phi i64 [ %v26939, %block19428 ], [ %v19844, %edge2803 ]
  %v26935 = phi i64 [ %v26941, %block19428 ], [ %v19846, %edge2803 ]
  %v26938 = phi i64 [ %v26944, %block19428 ], [ %v19849, %edge2803 ]
  %t2247 = icmp eq i64 %v19849, %v26938
  %t2249.not79 = icmp eq i64 %v26938, 0
  %t2249.not = or i1 %t2247, %t2249.not79
  br i1 %t2249.not, label %block19433, label %edge2815

block19433:                                       ; preds = %block19427, %edge2815
  %t2253.not80 = icmp eq i64 %v19849, 0
  %t2253.not = or i1 %t2253.not80, %t2247
  br i1 %t2253.not, label %edge2826, label %edge2820

block19424:                                       ; preds = %edge2820, %block19429
  %v26921 = phi i64 [ %v26954, %block19429 ], [ %v26968, %edge2820 ]
  %t2255 = insertvalue { i64, i64 } poison, i64 %v26921, 0
  %t2256 = insertvalue { i64, i64 } %t2255, i64 1344, 1
  br label %common.ret

block19735:                                       ; preds = %edge2862.thread, %edge2862
  %v27441581583 = phi i64 [ %v27441, %edge2862 ], [ %v27399, %edge2862.thread ]
  %t2296 = zext i64 %v26933 to i128
  %v219771 = or disjoint i128 %t2296, 4835703278458516698824704
  store i128 %v219771, ptr %ss0, align 16
  %t2300 = zext i64 %v26935 to i128
  %v219773 = or disjoint i128 %t2300, 18446744073709551616
  store i128 %v219773, ptr %t1002, align 16
  %t2306 = zext i64 %v26938 to i128
  %t2307 = shl nuw i128 %t2306, 64
  %v219775 = or disjoint i128 %t2307, 262144
  store i128 %v219775, ptr %t1007, align 16
  %v27468 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v26935, i64 2, i64 2, i64 %v3194)
  %t2310.not = icmp eq i64 %v27468, 0
  br i1 %t2310.not, label %block19734, label %block19710

block19734:                                       ; preds = %block19735
  %v27471 = load i64, ptr %t607, align 8
  %v27470 = load i64, ptr %t606, align 8
  %v27469 = load i64, ptr %ss1, align 8
  %v27472 = add i64 %v27469, %v27441581583
  %t2313 = inttoptr i64 %v27472 to ptr
  store i16 0, ptr %t2313, align 1
  br label %block19733

block19733:                                       ; preds = %edge2862.thread, %block19734
  %v27447 = phi i64 [ %v27469, %block19734 ], [ %v26933, %edge2862.thread ]
  %v27449 = phi i64 [ %v27470, %block19734 ], [ %v26935, %edge2862.thread ]
  %v27452 = phi i64 [ %v27471, %block19734 ], [ %v26938, %edge2862.thread ]
  %t2314 = icmp eq i64 %v26938, %v27452
  %t2316.not82 = icmp eq i64 %v27452, 0
  %t2316.not = or i1 %t2314, %t2316.not82
  br i1 %t2316.not, label %block19739, label %edge2874

block19739:                                       ; preds = %block19733, %edge2874
  %t2320.not = or i1 %t2249.not79, %t2314
  br i1 %t2320.not, label %edge2885, label %edge2879

block19710:                                       ; preds = %edge2879, %block19735
  %v27389 = phi i64 [ %v27468, %block19735 ], [ %v27482, %edge2879 ]
  %t2322 = insertvalue { i64, i64 } poison, i64 %v27389, 0
  %t2323 = insertvalue { i64, i64 } %t2322, i64 1366, 1
  br label %common.ret

block19750:                                       ; preds = %edge2885.thread, %edge2889, %edge2888
  %v2744721936219512210522123 = phi i64 [ %v27447, %edge2888 ], [ %v27447, %edge2889 ], [ %v26933, %edge2885.thread ]
  %v2744921937219502210622122 = phi i64 [ %v27449, %edge2888 ], [ %v27449, %edge2889 ], [ %v26935, %edge2885.thread ]
  %v2745221938219492210722120 = phi i64 [ %v27452, %edge2888 ], [ %v27452, %edge2889 ], [ 0, %edge2885.thread ]
  %t2316.not8221941219482210822119 = phi i1 [ true, %edge2888 ], [ false, %edge2889 ], [ true, %edge2885.thread ]
  %v275082210922117 = phi i64 [ %v27508, %edge2888 ], [ %v27508, %edge2889 ], [ %v2750822100, %edge2885.thread ]
  %t2335 = zext i64 %v2744721936219512210522123 to i128
  %v219777 = or disjoint i128 %t2335, 4835703278458516698824704
  store i128 %v219777, ptr %ss0, align 16
  %t2339 = zext i64 %v2744921937219502210622122 to i128
  %v219779 = or disjoint i128 %t2339, 18446744073709551616
  store i128 %v219779, ptr %t1002, align 16
  %t2345 = zext i64 %v2745221938219492210722120 to i128
  %t2346 = shl nuw i128 %t2345, 64
  %v219781 = or disjoint i128 %t2346, 262144
  store i128 %v219781, ptr %t1007, align 16
  %v27535 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v2744921937219502210622122, i64 2, i64 2, i64 %v3194)
  %t2349.not = icmp eq i64 %v27535, 0
  br i1 %t2349.not, label %block19748, label %block19745

block19748:                                       ; preds = %block19750
  %v27536 = load i64, ptr %ss1, align 8
  %v27537 = load i64, ptr %t606, align 8
  %v27538 = load i64, ptr %t607, align 8
  %.pre21789 = add i64 %v27536, %v275082210922117
  %.pre21790 = inttoptr i64 %.pre21789 to ptr
  store i16 0, ptr %.pre21790, align 1
  %t2353 = icmp eq i64 %v2745221938219492210722120, %v27538
  %t2355.not84 = icmp eq i64 %v27538, 0
  %t2355.not = or i1 %t2353, %t2355.not84
  br i1 %t2355.not, label %block19754, label %edge2897

block19754:                                       ; preds = %block19748, %edge2897
  %t2359.not = or i1 %t2316.not8221941219482210822119, %t2353
  br i1 %t2359.not, label %block19858, label %edge2902

block19745:                                       ; preds = %edge2902, %block19750
  %v27502 = phi i64 [ %v27535, %block19750 ], [ %v27549, %edge2902 ]
  %t2361 = insertvalue { i64, i64 } poison, i64 %v27502, 0
  %t2362 = insertvalue { i64, i64 } %t2361, i64 1368, 1
  br label %common.ret

block19858:                                       ; preds = %block913, %block19754.thread22297, %edge2885.thread, %edge2885, %edge2715, %edge2853, %edge2902, %block19754, %edge2732, %block18965, %edge2683
  %v219272 = phi i64 [ %v26065, %edge2732 ], [ %v27536, %block19754 ], [ %v26065, %block18965 ], [ %v19844, %edge2683 ], [ %v27536, %edge2902 ], [ %v26933, %edge2853 ], [ %v2598121906, %edge2715 ], [ %v27447, %block19754.thread22297 ], [ %v27447, %edge2885 ], [ %v26933, %edge2885.thread ], [ %v19844, %block913 ]
  %v219273 = phi i64 [ %v26067, %edge2732 ], [ %v27537, %block19754 ], [ %v26067, %block18965 ], [ %v19846, %edge2683 ], [ %v27537, %edge2902 ], [ %v26935, %edge2853 ], [ %v2598321907, %edge2715 ], [ %v27449, %block19754.thread22297 ], [ %v27449, %edge2885 ], [ %v26935, %edge2885.thread ], [ %v19846, %block913 ]
  %v219274 = phi i64 [ %v26070, %edge2732 ], [ %v27538, %block19754 ], [ %v26070, %block18965 ], [ %v19849, %edge2683 ], [ %v27538, %edge2902 ], [ %v26938, %edge2853 ], [ %v2598621908, %edge2715 ], [ %v27452, %block19754.thread22297 ], [ %v27452, %edge2885 ], [ 0, %edge2885.thread ], [ %v19849, %block913 ]
  %v219275 = phi i64 [ %v25143, %edge2732 ], [ %v26103554, %block19754 ], [ %v25143, %block18965 ], [ %v25143, %edge2683 ], [ %v26103554, %edge2902 ], [ %v26103554, %edge2853 ], [ %v25143, %edge2715 ], [ %v26103554, %block19754.thread22297 ], [ %v26103554, %edge2885 ], [ %v26103554, %edge2885.thread ], [ %v19856, %block913 ]
  %v219276 = phi i64 [ %v25145, %edge2732 ], [ %v26105555, %block19754 ], [ %v25145, %block18965 ], [ %v25145, %edge2683 ], [ %v26105555, %edge2902 ], [ %v26105555, %edge2853 ], [ %v25145, %edge2715 ], [ %v26105555, %block19754.thread22297 ], [ %v26105555, %edge2885 ], [ %v26105555, %edge2885.thread ], [ %v19858, %block913 ]
  %v219277 = phi i64 [ %v25148, %edge2732 ], [ %v26108556, %block19754 ], [ %v25148, %block18965 ], [ %v25148, %edge2683 ], [ %v26108556, %edge2902 ], [ %v26108556, %edge2853 ], [ %v25148, %edge2715 ], [ %v26108556, %block19754.thread22297 ], [ %v26108556, %edge2885 ], [ %v26108556, %edge2885.thread ], [ %v19861, %block913 ]
  %v219278 = phi i64 [ %spec.select20, %edge2732 ], [ %spec.select20557, %block19754 ], [ %spec.select20, %block18965 ], [ %spec.select20, %edge2683 ], [ %spec.select20557, %edge2902 ], [ %spec.select20557, %edge2853 ], [ %spec.select20, %edge2715 ], [ %spec.select20557, %block19754.thread22297 ], [ %spec.select20557, %edge2885 ], [ %spec.select20557, %edge2885.thread ], [ %v19868, %block913 ]
  br i1 %t23.not, label %edge3980, label %edge2952

block1421:                                        ; preds = %edge3706, %edge3105
  %v28543 = phi i64 [ %v18875, %edge3105 ], [ %v35325, %edge3706 ]
  %v28545 = phi i64 [ %v18877, %edge3105 ], [ %v35327, %edge3706 ]
  %v28548 = phi i64 [ %v18880, %edge3105 ], [ %v35330, %edge3706 ]
  %v28576 = phi i64 [ %v19006, %edge3105 ], [ %v35294, %edge3706 ]
  %v28579 = phi i64 [ %v19011, %edge3105 ], [ %v35297, %edge3706 ]
  %v28582 = phi i64 [ %v18771, %edge3105 ], [ %v35373, %edge3706 ]
  %v28585 = phi i64 [ 300, %edge3105 ], [ %v35569, %edge3706 ]
  %v28588 = phi i64 [ 0, %edge3105 ], [ %v35300, %edge3706 ]
  %v28591 = phi i64 [ 0, %edge3105 ], [ %v35303, %edge3706 ]
  %v28594 = phi i64 [ 0, %edge3105 ], [ %spec.select22, %edge3706 ]
  %t2406 = icmp ult i64 %v28594, %spec.select18
  br i1 %t2406, label %edge3132, label %block24092

trap2434:                                         ; preds = %edge3191
  call void @llvm.trap()
  unreachable

ok2434:                                           ; preds = %edge3191
  %v29811 = udiv i64 %v29614, %v1399
  %v29812 = mul i64 %v29811, %v1399
  %v29813 = sub i64 %v29614, %v29812
  %v29814 = mul i64 %v29813, %v1400
  %v29815 = add i64 %v29814, %v29811
  br label %block20948

block20948:                                       ; preds = %edge3186, %ok2434
  %v29810 = phi i64 [ %v29815, %ok2434 ], [ %v29614, %edge3186 ]
  %v29817 = add i64 %v29810, %v1396
  %t2435 = inttoptr i64 %v29817 to ptr
  %t2436 = load i8, ptr %t2435, align 1
  %v29818 = zext i8 %t2436 to i64
  br label %edge3184

trap2444:                                         ; preds = %edge3210
  call void @llvm.trap()
  unreachable

ok2444:                                           ; preds = %edge3210
  %v30037 = udiv i64 %v29840, %v1399
  %v30038 = mul i64 %v30037, %v1399
  %v30039 = sub i64 %v29840, %v30038
  %v30040 = mul i64 %v30039, %v1400
  %v30041 = add i64 %v30040, %v30037
  br label %block21091

block21091:                                       ; preds = %edge3205, %ok2444
  %v30036 = phi i64 [ %v30041, %ok2444 ], [ %v29840, %edge3205 ]
  %v30043 = add i64 %v30036, %v1396
  %t2445 = inttoptr i64 %v30043 to ptr
  %t2446 = load i8, ptr %t2445, align 1
  %v30044 = zext i8 %t2446 to i64
  br label %edge3203

block21436:                                       ; preds = %edge3257, %edge3254
  %t2459.not = icmp eq i64 %v219198, 0
  br i1 %t2459.not, label %block21440, label %edge3263

block21440:                                       ; preds = %edge3263, %block21436
  %t2461.not = icmp eq i64 %v219213, 0
  br i1 %t2461.not, label %block21444, label %edge3269

block21444:                                       ; preds = %edge3269, %block21440
  %t2463.not = icmp eq i64 %v219226, 0
  br i1 %t2463.not, label %block21448, label %edge3275

block21448:                                       ; preds = %edge3275, %block21444
  %t2465.not = icmp eq i64 %v219232, 0
  br i1 %t2465.not, label %block21452, label %edge3281

block21452:                                       ; preds = %edge3281, %block21448
  %t2467.not = icmp eq i64 %v219242, 0
  br i1 %t2467.not, label %block21460, label %edge3287

block21460:                                       ; preds = %block21452, %edge3287
  br i1 %t23.not, label %block21464, label %edge3299

block21464:                                       ; preds = %edge3299, %block21460
  %t2473.not = icmp eq i64 %v10791.ph.ph, 0
  br i1 %t2473.not, label %block21468, label %edge3305

block21468:                                       ; preds = %edge3305, %block21464
  %t2475.not = icmp eq i64 %v10792, 0
  br i1 %t2475.not, label %block21472, label %edge3311

block21472:                                       ; preds = %edge3311, %block21468
  br i1 %t3136.not, label %block21476, label %edge3317

block21476:                                       ; preds = %edge3317, %block21472
  %t2479.not = icmp eq i64 %v10794, 0
  br i1 %t2479.not, label %block21480, label %edge3323

block21480:                                       ; preds = %edge3323, %block21476
  %t2481.not = icmp eq i64 %v18651, 0
  br i1 %t2481.not, label %block21484, label %edge3329

block21484:                                       ; preds = %edge3329, %block21480
  %t2483.not = icmp eq i64 %v28548, 0
  br i1 %t2483.not, label %block21488, label %edge3335

block21488:                                       ; preds = %edge3335, %block21484
  br i1 %t23.not, label %common.ret, label %edge3341

block21368:                                       ; preds = %edge3341, %edge3335, %edge3329, %edge3323, %edge3317, %edge3311, %edge3305, %edge3299, %edge3287, %edge3281, %edge3275, %edge3269, %edge3263, %edge3257
  %v30462 = phi i64 [ %v30584, %edge3335 ], [ %v30532, %edge3257 ], [ %v30536, %edge3263 ], [ %v30540, %edge3269 ], [ %v30544, %edge3275 ], [ %v30548, %edge3281 ], [ %v30552, %edge3287 ], [ %v30588, %edge3341 ], [ %v30560, %edge3299 ], [ %v30564, %edge3305 ], [ %v30568, %edge3311 ], [ %v30572, %edge3317 ], [ %v30576, %edge3323 ], [ %v30580, %edge3329 ]
  %t2487 = insertvalue { i64, i64 } poison, i64 %v30462, 0
  %t2488 = insertvalue { i64, i64 } %t2487, i64 1544, 1
  br label %common.ret

block1561:                                        ; preds = %edge3368, %edge3458
  %v30904 = phi i64 [ %v31767, %edge3458 ], [ %v30649, %edge3368 ]
  %t2501.not = icmp ult i64 %v30904, %spec.select18
  br i1 %t2501.not, label %edge3406, label %edge3392

trap2518:                                         ; preds = %edge3419
  call void @llvm.trap()
  unreachable

ok2518:                                           ; preds = %edge3419
  %v31321 = udiv i64 %v31115, %v1399
  %v31322 = mul i64 %v31321, %v1399
  %v31323 = sub nuw i64 %v31115, %v31322
  %v31324 = mul i64 %v31323, %v1400
  %v31325 = add i64 %v31324, %v31321
  br label %block21969

block21969:                                       ; preds = %edge3414, %ok2518
  %v31320 = phi i64 [ %v31325, %ok2518 ], [ %v31115, %edge3414 ]
  %v31327 = add i64 %v31320, %v1396
  %t2519 = inttoptr i64 %v31327 to ptr
  %t2520 = load i8, ptr %t2519, align 1
  %v31328 = zext i8 %t2520 to i64
  br label %edge3412

trap2528:                                         ; preds = %edge3438
  call void @llvm.trap()
  unreachable

ok2528:                                           ; preds = %edge3438
  %v31554 = udiv i64 %v31352, %v1399
  %v31555 = mul i64 %v31554, %v1399
  %v31556 = sub nuw i64 %v31352, %v31555
  %v31557 = mul i64 %v31556, %v1400
  %v31558 = add i64 %v31557, %v31554
  br label %block22116

block22116:                                       ; preds = %edge3433, %ok2528
  %v31553 = phi i64 [ %v31558, %ok2528 ], [ %v31352, %edge3433 ]
  %v31560 = add i64 %v31553, %v1396
  %t2529 = inttoptr i64 %v31560 to ptr
  %t2530 = load i8, ptr %t2529, align 1
  %v31561 = zext i8 %t2530 to i64
  br label %edge3431

block23212:                                       ; preds = %edge3571.thread, %edge3571
  %v34328610612 = phi i64 [ %v34328, %edge3571 ], [ %v34286, %edge3571.thread ]
  %t2604 = zext i64 %v28543 to i128
  %v219783 = or disjoint i128 %t2604, 4835703278458516698824704
  store i128 %v219783, ptr %ss0, align 16
  %t2608 = zext i64 %v28545 to i128
  %v219785 = or disjoint i128 %t2608, 18446744073709551616
  store i128 %v219785, ptr %t1002, align 16
  %t2614 = zext i64 %v28548 to i128
  %t2615 = shl nuw i128 %t2614, 64
  %v219787 = or disjoint i128 %t2615, 262144
  store i128 %v219787, ptr %t1007, align 16
  %v34355 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v28545, i64 2, i64 2, i64 %v3194)
  %t2618.not = icmp eq i64 %v34355, 0
  br i1 %t2618.not, label %block23210, label %block23187

block23210.thread:                                ; preds = %edge3571, %edge3571.thread
  %t2624.not7021975 = icmp eq i64 %v28548, 0
  br label %block23216

block23210:                                       ; preds = %block23212
  %v34358 = load i64, ptr %t607, align 8
  %v34357 = load i64, ptr %t606, align 8
  %v34356 = load i64, ptr %ss1, align 8
  %v34359 = add i64 %v34356, %v34328610612
  %t2621 = inttoptr i64 %v34359 to ptr
  store i16 %t2569, ptr %t2621, align 1
  %t2622 = icmp eq i64 %v28548, %v34358
  %t2624.not70 = icmp eq i64 %v34358, 0
  %t2624.not = or i1 %t2622, %t2624.not70
  br i1 %t2624.not, label %block23216, label %edge3583

block23216:                                       ; preds = %block23210.thread, %block23210, %edge3583
  %t2624.not7021983 = phi i1 [ %t2624.not7021975, %block23210.thread ], [ %t2624.not70, %block23210 ], [ %t2624.not70, %edge3583 ]
  %t262221982 = phi i1 [ true, %block23210.thread ], [ %t2622, %block23210 ], [ %t2622, %edge3583 ]
  %v3433921980 = phi i64 [ %v28548, %block23210.thread ], [ %v34358, %block23210 ], [ %v34358, %edge3583 ]
  %v3433621979 = phi i64 [ %v28545, %block23210.thread ], [ %v34357, %block23210 ], [ %v34357, %edge3583 ]
  %v3433421978 = phi i64 [ %v28543, %block23210.thread ], [ %v34356, %block23210 ], [ %v34356, %edge3583 ]
  %t2628.not = or i1 %t2578.not, %t262221982
  br i1 %t2628.not, label %edge3594, label %edge3588

block23187:                                       ; preds = %edge3588, %block23212
  %v34276 = phi i64 [ %v34355, %block23212 ], [ %v34369, %edge3588 ]
  %t2630 = insertvalue { i64, i64 } poison, i64 %v34276, 0
  %t2631 = insertvalue { i64, i64 } %t2630, i64 1799, 1
  br label %common.ret

block23227:                                       ; preds = %edge3598, %edge3597
  %t2643 = zext i64 %v3433421978 to i128
  %v219789 = or disjoint i128 %t2643, 4835703278458516698824704
  store i128 %v219789, ptr %ss0, align 16
  %t2647 = zext i64 %v3433621979 to i128
  %v219791 = or disjoint i128 %t2647, 18446744073709551616
  store i128 %v219791, ptr %t1002, align 16
  %t2653 = zext i64 %v3433921980 to i128
  %t2654 = shl nuw i128 %t2653, 64
  %v219793 = or disjoint i128 %t2654, 262144
  store i128 %v219793, ptr %t1007, align 16
  %v34433 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v3433621979, i64 2, i64 2, i64 %v3194)
  %t2657.not = icmp eq i64 %v34433, 0
  br i1 %t2657.not, label %edge3603, label %block23222

block23225:                                       ; preds = %edge3603, %edge3598
  %t2660.pre-phi = phi ptr [ %t2634, %edge3598 ], [ %.pre21788, %edge3603 ]
  %v34418 = phi i64 [ %v3433421978, %edge3598 ], [ %v34434, %edge3603 ]
  %v34420 = phi i64 [ %v3433621979, %edge3598 ], [ %v34435, %edge3603 ]
  %v34423 = phi i64 [ %v3433921980, %edge3598 ], [ %v34436, %edge3603 ]
  store i16 %t2575, ptr %t2660.pre-phi, align 1
  %t2661 = icmp eq i64 %v3433921980, %v34423
  %t2663.not72 = icmp eq i64 %v34423, 0
  %t2663.not = or i1 %t2661, %t2663.not72
  br i1 %t2663.not, label %block23231, label %edge3606

block23231:                                       ; preds = %block23225, %edge3606
  %t2667.not = or i1 %t2624.not7021983, %t2661
  br i1 %t2667.not, label %block1793, label %edge3611

block23222:                                       ; preds = %edge3611, %block23227
  %v34400 = phi i64 [ %v34433, %block23227 ], [ %v34447, %edge3611 ]
  %t2669 = insertvalue { i64, i64 } poison, i64 %v34400, 0
  %t2670 = insertvalue { i64, i64 } %t2669, i64 1801, 1
  br label %common.ret

block1793:                                        ; preds = %edge3594, %block23231, %edge3611, %edge3562
  %v34483 = phi i64 [ %v28543, %edge3562 ], [ %v34418, %edge3611 ], [ %v34418, %block23231 ], [ %v3433421978, %edge3594 ]
  %v34485 = phi i64 [ %v28545, %edge3562 ], [ %v34420, %edge3611 ], [ %v34420, %block23231 ], [ %v3433621979, %edge3594 ]
  %v34488 = phi i64 [ %v28548, %edge3562 ], [ %v34423, %edge3611 ], [ %v34423, %block23231 ], [ %v3433921980, %edge3594 ]
  %v34463 = shl nuw nsw i64 %v3356322126, 16
  %v34480 = or i64 %v34463, %v29006
  br label %block24092

block1799:                                        ; preds = %edge3539.block1799_crit_edge, %edge3203
  %v34560.pre-phi = phi i64 [ %.pre21808, %edge3539.block1799_crit_edge ], [ %v29614, %edge3203 ]
  %v218808 = phi i64 [ %v33563, %edge3539.block1799_crit_edge ], [ %v28594, %edge3203 ]
  %t2672 = icmp ult i64 %v34560.pre-phi, %v1397
  br i1 %t2672, label %edge3629, label %edge3627

trap2679:                                         ; preds = %edge3634
  call void @llvm.trap()
  unreachable

ok2679:                                           ; preds = %edge3634
  %v34769 = udiv i64 %v34560.pre-phi, %v1399
  %v34770 = mul i64 %v34769, %v1399
  %v34771 = sub i64 %v34560.pre-phi, %v34770
  %v34772 = mul i64 %v34771, %v1400
  %v34773 = add i64 %v34772, %v34769
  br label %block23387

block23387:                                       ; preds = %edge3629, %ok2679
  %v34768 = phi i64 [ %v34773, %ok2679 ], [ %v34560.pre-phi, %edge3629 ]
  %v34775 = add i64 %v34768, %v1396
  %t2680 = inttoptr i64 %v34775 to ptr
  %t2681 = load i8, ptr %t2680, align 1
  %v34776 = zext i8 %t2681 to i64
  br label %edge3627

trap2689:                                         ; preds = %edge3653
  call void @llvm.trap()
  unreachable

ok2689:                                           ; preds = %edge3653
  %v35045 = udiv i64 %v34848, %v1399
  %v35046 = mul i64 %v35045, %v1399
  %v35047 = sub i64 %v34848, %v35046
  %v35048 = mul i64 %v35047, %v1400
  %v35049 = add i64 %v35048, %v35045
  br label %edge3646

block23689:                                       ; preds = %edge3687, %edge3686
  %t2706 = zext i64 %v28543 to i128
  %v219795 = or disjoint i128 %t2706, 4835703278458516698824704
  store i128 %v219795, ptr %ss0, align 16
  %t2710 = zext i64 %v28545 to i128
  %v219797 = or disjoint i128 %t2710, 18446744073709551616
  store i128 %v219797, ptr %t1002, align 16
  %t2716 = zext i64 %v28548 to i128
  %t2717 = shl nuw i128 %t2716, 64
  %v219799 = or disjoint i128 %t2717, 262144
  store i128 %v219799, ptr %t1007, align 16
  %v35346 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v28545, i64 2, i64 2, i64 %v3194)
  %t2720.not = icmp eq i64 %v35346, 0
  br i1 %t2720.not, label %edge3692, label %block23684

block23688:                                       ; preds = %edge3687, %edge3692
  %t2723.pre-phi = phi ptr [ %t2697, %edge3687 ], [ %.pre21786, %edge3692 ]
  %v35331 = phi i64 [ %v28543, %edge3687 ], [ %v35347, %edge3692 ]
  %v35333 = phi i64 [ %v28545, %edge3687 ], [ %v35348, %edge3692 ]
  %v35336 = phi i64 [ %v28548, %edge3687 ], [ %v35349, %edge3692 ]
  %v35351 = trunc nuw i64 %v28582 to i16
  store i16 %v35351, ptr %t2723.pre-phi, align 1
  br label %block23687

block23687:                                       ; preds = %edge3683, %block23688
  %v35325 = phi i64 [ %v35331, %block23688 ], [ %v28543, %edge3683 ]
  %v35327 = phi i64 [ %v35333, %block23688 ], [ %v28545, %edge3683 ]
  %v35330 = phi i64 [ %v35336, %block23688 ], [ %v28548, %edge3683 ]
  %t2724 = icmp eq i64 %v28548, %v35330
  %t2726.not63 = icmp eq i64 %v35330, 0
  %t2726.not = or i1 %t2724, %t2726.not63
  br i1 %t2726.not, label %block23693, label %edge3695

block23693:                                       ; preds = %block23687, %edge3695
  %t2730.not64 = icmp eq i64 %v28548, 0
  %t2730.not = or i1 %t2730.not64, %t2724
  br i1 %t2730.not, label %edge3706, label %edge3700

block23684:                                       ; preds = %edge3700, %block23689
  %v35313 = phi i64 [ %v35346, %block23689 ], [ %v35360, %edge3700 ]
  %t2732 = insertvalue { i64, i64 } poison, i64 %v35313, 0
  %t2733 = insertvalue { i64, i64 } %t2732, i64 1859, 1
  br label %common.ret

block23995:                                       ; preds = %edge3742.thread, %edge3742
  %v35833628630 = phi i64 [ %v35833, %edge3742 ], [ %v35791, %edge3742.thread ]
  %t2773 = zext i64 %v35325 to i128
  %v219801 = or disjoint i128 %t2773, 4835703278458516698824704
  store i128 %v219801, ptr %ss0, align 16
  %t2777 = zext i64 %v35327 to i128
  %v219803 = or disjoint i128 %t2777, 18446744073709551616
  store i128 %v219803, ptr %t1002, align 16
  %t2783 = zext i64 %v35330 to i128
  %t2784 = shl nuw i128 %t2783, 64
  %v219805 = or disjoint i128 %t2784, 262144
  store i128 %v219805, ptr %t1007, align 16
  %v35860 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v35327, i64 2, i64 2, i64 %v3194)
  %t2787.not = icmp eq i64 %v35860, 0
  br i1 %t2787.not, label %block23994, label %block23970

block23994:                                       ; preds = %block23995
  %v35863 = load i64, ptr %t607, align 8
  %v35862 = load i64, ptr %t606, align 8
  %v35861 = load i64, ptr %ss1, align 8
  %v35864 = add i64 %v35861, %v35833628630
  %t2790 = inttoptr i64 %v35864 to ptr
  store i16 0, ptr %t2790, align 1
  br label %block23993

block23993:                                       ; preds = %edge3742.thread, %block23994
  %v35839 = phi i64 [ %v35861, %block23994 ], [ %v35325, %edge3742.thread ]
  %v35841 = phi i64 [ %v35862, %block23994 ], [ %v35327, %edge3742.thread ]
  %v35844 = phi i64 [ %v35863, %block23994 ], [ %v35330, %edge3742.thread ]
  %t2791 = icmp eq i64 %v35330, %v35844
  %t2793.not66 = icmp eq i64 %v35844, 0
  %t2793.not = or i1 %t2791, %t2793.not66
  br i1 %t2793.not, label %block23999, label %edge3754

block23999:                                       ; preds = %block23993, %edge3754
  %t2797.not = or i1 %t2726.not63, %t2791
  br i1 %t2797.not, label %edge3765, label %edge3759

block23970:                                       ; preds = %edge3759, %block23995
  %v35781 = phi i64 [ %v35860, %block23995 ], [ %v35874, %edge3759 ]
  %t2799 = insertvalue { i64, i64 } poison, i64 %v35781, 0
  %t2800 = insertvalue { i64, i64 } %t2799, i64 1881, 1
  br label %common.ret

block24010:                                       ; preds = %edge3765.thread, %edge3769, %edge3768
  %v3583922008220232213622154 = phi i64 [ %v35839, %edge3768 ], [ %v35839, %edge3769 ], [ %v35325, %edge3765.thread ]
  %v3584122009220222213722153 = phi i64 [ %v35841, %edge3768 ], [ %v35841, %edge3769 ], [ %v35327, %edge3765.thread ]
  %v3584422010220212213822151 = phi i64 [ %v35844, %edge3768 ], [ %v35844, %edge3769 ], [ 0, %edge3765.thread ]
  %t2793.not6622013220202213922150 = phi i1 [ true, %edge3768 ], [ false, %edge3769 ], [ true, %edge3765.thread ]
  %v359002214022148 = phi i64 [ %v35900, %edge3768 ], [ %v35900, %edge3769 ], [ %v3590022131, %edge3765.thread ]
  %t2812 = zext i64 %v3583922008220232213622154 to i128
  %v219807 = or disjoint i128 %t2812, 4835703278458516698824704
  store i128 %v219807, ptr %ss0, align 16
  %t2816 = zext i64 %v3584122009220222213722153 to i128
  %v219809 = or disjoint i128 %t2816, 18446744073709551616
  store i128 %v219809, ptr %t1002, align 16
  %t2822 = zext i64 %v3584422010220212213822151 to i128
  %t2823 = shl nuw i128 %t2822, 64
  %v219811 = or disjoint i128 %t2823, 262144
  store i128 %v219811, ptr %t1007, align 16
  %v35927 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v3584122009220222213722153, i64 2, i64 2, i64 %v3194)
  %t2826.not = icmp eq i64 %v35927, 0
  br i1 %t2826.not, label %block24008, label %block24005

block24008:                                       ; preds = %block24010
  %v35928 = load i64, ptr %ss1, align 8
  %v35929 = load i64, ptr %t606, align 8
  %v35930 = load i64, ptr %t607, align 8
  %.pre21783 = add i64 %v35928, %v359002214022148
  %.pre21784 = inttoptr i64 %.pre21783 to ptr
  store i16 0, ptr %.pre21784, align 1
  %t2830 = icmp eq i64 %v3584422010220212213822151, %v35930
  %t2832.not68 = icmp eq i64 %v35930, 0
  %t2832.not = or i1 %t2830, %t2832.not68
  br i1 %t2832.not, label %block24014, label %edge3777

block24014:                                       ; preds = %block24008, %edge3777
  %t2836.not = or i1 %t2793.not6622013220202213922150, %t2830
  br i1 %t2836.not, label %block24092, label %edge3782

block24005:                                       ; preds = %edge3782, %block24010
  %v35894 = phi i64 [ %v35927, %block24010 ], [ %v35941, %edge3782 ]
  %t2838 = insertvalue { i64, i64 } poison, i64 %v35894, 0
  %t2839 = insertvalue { i64, i64 } %t2838, i64 1883, 1
  br label %common.ret

block24092:                                       ; preds = %block1421, %block24014.thread22316, %edge3765.thread, %edge3765, %edge3733, %edge3782, %block24014, %block1793
  %v219280 = phi i64 [ %v35928, %block24014 ], [ %v34483, %block1793 ], [ %v35928, %edge3782 ], [ %v35325, %edge3733 ], [ %v35839, %block24014.thread22316 ], [ %v35839, %edge3765 ], [ %v35325, %edge3765.thread ], [ %v28543, %block1421 ]
  %v219281 = phi i64 [ %v35929, %block24014 ], [ %v34485, %block1793 ], [ %v35929, %edge3782 ], [ %v35327, %edge3733 ], [ %v35841, %block24014.thread22316 ], [ %v35841, %edge3765 ], [ %v35327, %edge3765.thread ], [ %v28545, %block1421 ]
  %v219282 = phi i64 [ %v35930, %block24014 ], [ %v34488, %block1793 ], [ %v35930, %edge3782 ], [ %v35330, %edge3733 ], [ %v35844, %block24014.thread22316 ], [ %v35844, %edge3765 ], [ 0, %edge3765.thread ], [ %v28548, %block1421 ]
  %v219284 = phi i64 [ 0, %block24014 ], [ %v34480, %block1793 ], [ 0, %edge3782 ], [ 0, %edge3733 ], [ 0, %block24014.thread22316 ], [ 0, %edge3765 ], [ 0, %edge3765.thread ], [ 0, %block1421 ]
  br i1 %t23.not, label %edge3980, label %edge3826

block25146:                                       ; preds = %edge4057, %edge4056
  %t2924 = zext i64 %v10831 to i128
  %v219813 = or disjoint i128 %t2924, 184467440737095516160
  store i128 %v219813, ptr %ss0, align 16
  %t2928 = zext i64 %v10806 to i128
  %v219815 = or disjoint i128 %t2928, 18446744073709551616
  store i128 %v219815, ptr %t1002, align 16
  %t2934 = zext i64 %v10794 to i128
  %t2935 = shl nuw i128 %t2934, 64
  %v219817 = or disjoint i128 %t2935, 10
  store i128 %v219817, ptr %t1007, align 16
  %v38824 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v10806, i64 4, i64 4, i64 %v3194)
  %t2938.not = icmp eq i64 %v38824, 0
  br i1 %t2938.not, label %edge4062, label %block25141

block25144:                                       ; preds = %edge4062, %edge4057
  %t2941.pre-phi = phi ptr [ %.pre21780, %edge4062 ], [ %t2907, %edge4057 ]
  %v38809 = phi i64 [ %v38825, %edge4062 ], [ %v10831, %edge4057 ]
  %v38811 = phi i64 [ %v38826, %edge4062 ], [ %v10806, %edge4057 ]
  %v38814 = phi i64 [ %v38827, %edge4062 ], [ %v10794, %edge4057 ]
  store i32 %17, ptr %t2941.pre-phi, align 1
  %t2942 = icmp eq i64 %v10794, %v38814
  %t2944.not96 = icmp eq i64 %v38814, 0
  %t2944.not = or i1 %t2942, %t2944.not96
  br i1 %t2944.not, label %block25150, label %edge4065

block25150:                                       ; preds = %block25144, %edge4065
  %t2948.not = or i1 %t2919.not, %t2942
  br i1 %t2948.not, label %edge4074, label %edge4070

block25141:                                       ; preds = %edge4070, %block25146
  %v38791 = phi i64 [ %v38824, %block25146 ], [ %v38838, %edge4070 ]
  %t2950 = insertvalue { i64, i64 } poison, i64 %v38791, 0
  %t2951 = insertvalue { i64, i64 } %t2950, i64 2000, 1
  br label %common.ret

block25304:                                       ; preds = %edge4102, %edge4101
  %t2969 = zext i64 %v10829 to i128
  %v219819 = or disjoint i128 %t2969, 4777706715090773868544
  store i128 %v219819, ptr %ss0, align 16
  %t2973 = zext i64 %v10802 to i128
  %v219821 = or disjoint i128 %t2973, 18446744073709551616
  store i128 %v219821, ptr %t1002, align 16
  %t2979 = zext i64 %v10792 to i128
  %t2980 = shl nuw i128 %t2979, 64
  %v219823 = or disjoint i128 %t2980, 259
  store i128 %v219823, ptr %t1007, align 16
  %v39114 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v10802, i64 4, i64 4, i64 %v3194)
  %t2983.not = icmp eq i64 %v39114, 0
  br i1 %t2983.not, label %edge4107, label %block25299

block25302:                                       ; preds = %edge4107, %edge4102
  %t2986.pre-phi = phi ptr [ %.pre21782, %edge4107 ], [ %t2955, %edge4102 ]
  %v39099 = phi i64 [ %v39115, %edge4107 ], [ %v10829, %edge4102 ]
  %v39101 = phi i64 [ %v39116, %edge4107 ], [ %v10802, %edge4102 ]
  %v39104 = phi i64 [ %v39117, %edge4107 ], [ %v10792, %edge4102 ]
  store i32 %18, ptr %t2986.pre-phi, align 1
  %t2987 = icmp eq i64 %v10792, %v39104
  %t2989.not98 = icmp eq i64 %v39104, 0
  %t2989.not = or i1 %t2987, %t2989.not98
  br i1 %t2989.not, label %block25308, label %edge4110

block25308:                                       ; preds = %block25302, %edge4110
  %t2993.not = or i1 %t2964.not, %t2987
  br i1 %t2993.not, label %edge4214, label %edge4113

block25299:                                       ; preds = %edge4074, %edge4116, %block25304
  %v39081 = phi i64 [ %v39114, %block25304 ], [ 103425, %edge4074 ], [ %v39128, %edge4116 ]
  %t2998 = insertvalue { i64, i64 } poison, i64 %v39081, 0
  %t2999 = insertvalue { i64, i64 } %t2998, i64 2020, 1
  br label %common.ret

trap3007:                                         ; preds = %edge4139
  call void @llvm.trap()
  unreachable

ok3007:                                           ; preds = %edge4139
  %v39380 = udiv i64 %v218770, %v1399
  %v39381 = mul i64 %v39380, %v1399
  %v39382 = sub i64 %v218770, %v39381
  %v39383 = mul i64 %v39382, %v1400
  %v39384 = add i64 %v39383, %v39380
  br label %block25465

block25465:                                       ; preds = %edge4134, %ok3007
  %v39379 = phi i64 [ %v39384, %ok3007 ], [ %v218770, %edge4134 ]
  %v39386 = add i64 %v39379, %v1396
  %t3008 = inttoptr i64 %v39386 to ptr
  %t3009 = load i8, ptr %t3008, align 1
  %v39387 = zext i8 %t3009 to i64
  br label %edge4132

block25622:                                       ; preds = %edge4170, %edge4132
  %t3028 = zext i64 %v10831 to i128
  %v219825 = or disjoint i128 %t3028, 184467440737095516160
  store i128 %v219825, ptr %ss0, align 16
  %t3032 = zext i64 %v10806 to i128
  %v219827 = or disjoint i128 %t3032, 18446744073709551616
  store i128 %v219827, ptr %t1002, align 16
  %t3038 = zext i64 %v10794 to i128
  %t3039 = shl nuw i128 %t3038, 64
  %v219829 = or disjoint i128 %t3039, 10
  store i128 %v219829, ptr %t1007, align 16
  %v39908 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v10806, i64 4, i64 4, i64 %v3194)
  %t3042.not = icmp eq i64 %v39908, 0
  br i1 %t3042.not, label %edge4175, label %block25617

block25621:                                       ; preds = %edge4170, %edge4175
  %t3045.pre-phi = phi ptr [ %t3015, %edge4170 ], [ %.pre21778, %edge4175 ]
  %v39893 = phi i64 [ %v10831, %edge4170 ], [ %v39909, %edge4175 ]
  %v39895 = phi i64 [ %v10806, %edge4170 ], [ %v39910, %edge4175 ]
  %v39898 = phi i64 [ %v10794, %edge4170 ], [ %v39911, %edge4175 ]
  store i32 %v39855, ptr %t3045.pre-phi, align 1
  %t3046 = icmp eq i64 %v10794, %v39898
  %t3048.not94 = icmp eq i64 %v39898, 0
  %t3048.not = or i1 %t3046, %t3048.not94
  br i1 %t3048.not, label %block25626, label %edge4178

block25626:                                       ; preds = %block25621, %edge4178
  %t3052.not = or i1 %t3023.not, %t3046
  br i1 %t3052.not, label %edge4187, label %edge4183

block25617:                                       ; preds = %edge4183, %block25622
  %v39875 = phi i64 [ %v39908, %block25622 ], [ %v39922, %edge4183 ]
  %t3054 = insertvalue { i64, i64 } poison, i64 %v39875, 0
  %t3055 = insertvalue { i64, i64 } %t3054, i64 2059, 1
  br label %common.ret

trap3063:                                         ; preds = %edge4227
  call void @llvm.trap()
  unreachable

ok3063:                                           ; preds = %edge4227
  %v40256 = udiv i64 %v218770, %v1399
  %v40257 = mul i64 %v40256, %v1399
  %v40258 = sub i64 %v218770, %v40257
  %v40259 = mul i64 %v40258, %v1400
  %v40260 = add i64 %v40259, %v40256
  br label %block25834

block25834:                                       ; preds = %edge4222, %ok3063
  %v40255 = phi i64 [ %v40260, %ok3063 ], [ %v218770, %edge4222 ]
  %v40262 = add i64 %v40255, %v1396
  %t3064 = inttoptr i64 %v40262 to ptr
  %t3065 = load i8, ptr %t3064, align 1
  %v40263 = zext i8 %t3065 to i64
  %1 = shl nuw nsw i64 %v40263, 16
  br label %edge4220

block25852:                                       ; preds = %edge4245, %edge4244
  %t3078 = zext i64 %v36637647 to i128
  %v219831 = or disjoint i128 %t1719, %t3078
  store i128 %v219831, ptr %ss0, align 16
  %t3082 = zext i64 %v36639648 to i128
  %v219833 = or disjoint i128 %t3082, 18446744073709551616
  store i128 %v219833, ptr %t1002, align 16
  %t3088 = zext i64 %v36642649 to i128
  %t3089 = shl nuw i128 %t3088, 64
  %v219835 = or disjoint i128 %t3089, %t1163
  store i128 %v219835, ptr %t1007, align 16
  %v40351 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v36639648, i64 4, i64 4, i64 %v3194)
  %t3092.not = icmp eq i64 %v40351, 0
  br i1 %t3092.not, label %edge4250, label %block25847

block25850:                                       ; preds = %edge4250, %edge4245
  %t3095.pre-phi = phi ptr [ %.pre21776, %edge4250 ], [ %t3069, %edge4245 ]
  %v40336 = phi i64 [ %v40352, %edge4250 ], [ %v36637647, %edge4245 ]
  %v40338 = phi i64 [ %v40353, %edge4250 ], [ %v36639648, %edge4245 ]
  %v40341 = phi i64 [ %v40354, %edge4250 ], [ %v36642649, %edge4245 ]
  %v40356 = trunc nuw nsw i64 %v40300 to i32
  store i32 %v40356, ptr %t3095.pre-phi, align 1
  %t3096 = icmp eq i64 %v36642649, %v40341
  %t3098.not100 = icmp eq i64 %v40341, 0
  %t3098.not = or i1 %t3096, %t3098.not100
  br i1 %t3098.not, label %block25856, label %edge4253

block25856:                                       ; preds = %block25850, %edge4253
  %t3102.not = or i1 %t3098.not1001203, %t3096
  br i1 %t3102.not, label %edge4262, label %edge4258

block25847:                                       ; preds = %edge4258, %block25852, %edge4220
  %v40318 = phi i64 [ %v40351, %block25852 ], [ 103425, %edge4220 ], [ %v40365, %edge4258 ]
  %t3104 = insertvalue { i64, i64 } poison, i64 %v40318, 0
  %t3105 = insertvalue { i64, i64 } %t3104, i64 2094, 1
  br label %common.ret

block26830:                                       ; preds = %edge4360, %edge4361
  br i1 %t3136.not, label %block2176.preheader, label %edge4364

block2176.preheader:                              ; preds = %edge4364, %block26830
  br label %block2176

block2176:                                        ; preds = %block2176.preheader, %edge4449
  %v42746 = phi i64 [ %v44062, %edge4449 ], [ 0, %block2176.preheader ]
  %v42749 = phi i64 [ %v44070, %edge4449 ], [ 0, %block2176.preheader ]
  %exitcond21628.not = icmp eq i64 %v42746, 10
  br i1 %exitcond21628.not, label %edge4432, label %edge4387

block27584:                                       ; preds = %edge4435, %edge4432
  br i1 %t3136.not, label %edge4464, label %edge4439

block27580:                                       ; preds = %edge4442, %edge4435
  %v43884 = phi i64 [ %v43891, %edge4435 ], [ %v43895, %edge4442 ]
  %t3183 = insertvalue { i64, i64 } poison, i64 %v43884, 0
  %t3184 = insertvalue { i64, i64 } %t3183, i64 2255, 1
  br label %common.ret

block2305:                                        ; preds = %block2305.backedge, %block2305.preheader
  %v45178 = phi i64 [ %v40044, %block2305.preheader ], [ %v45898, %block2305.backedge ]
  %v45180 = phi i64 [ %v40046, %block2305.preheader ], [ %v45900, %block2305.backedge ]
  %v45183 = phi i64 [ %v40049, %block2305.preheader ], [ %v45903, %block2305.backedge ]
  %v45193 = phi i64 [ %v10830.ph.ph, %block2305.preheader ], [ %v45835, %block2305.backedge ]
  %v45195 = phi i64 [ %v10804.ph.ph, %block2305.preheader ], [ %v45837, %block2305.backedge ]
  %v45198 = phi i64 [ %v10793.ph.ph, %block2305.preheader ], [ %v45840, %block2305.backedge ]
  %v45205 = phi i64 [ 0, %block2305.preheader ], [ %v45569, %block2305.backedge ]
  %exitcond21629.not = icmp eq i64 %v45205, 10
  br i1 %exitcond21629.not, label %edge4539, label %edge4538

block28610:                                       ; preds = %edge4567, %edge4566
  %t3267 = zext i64 %v45193 to i128
  %v219837 = or disjoint i128 %t3267, 184467440737095516160
  store i128 %v219837, ptr %ss0, align 16
  %t3271 = zext i64 %v45195 to i128
  %v219839 = or disjoint i128 %t3271, 18446744073709551616
  store i128 %v219839, ptr %t1002, align 16
  %t3277 = zext i64 %v45198 to i128
  %t3278 = shl nuw i128 %t3277, 64
  %v219841 = or disjoint i128 %t3278, 10
  store i128 %v219841, ptr %t1007, align 16
  %v45856 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v45195, i64 4, i64 4, i64 %v3194)
  %t3281.not = icmp eq i64 %v45856, 0
  br i1 %t3281.not, label %edge4572, label %block28605

block28609:                                       ; preds = %edge4567, %edge4572
  %t3284.pre-phi = phi ptr [ %t3250, %edge4567 ], [ %.pre21706, %edge4572 ]
  %v45841 = phi i64 [ %v45193, %edge4567 ], [ %v45857, %edge4572 ]
  %v45843 = phi i64 [ %v45195, %edge4567 ], [ %v45858, %edge4572 ]
  %v45846 = phi i64 [ %v45198, %edge4567 ], [ %v45859, %edge4572 ]
  %v45861 = trunc nuw i64 %v45812 to i32
  store i32 %v45861, ptr %t3284.pre-phi, align 1
  br label %block28608

block28608:                                       ; preds = %edge4561, %block28609
  %v45835 = phi i64 [ %v45841, %block28609 ], [ %v45193, %edge4561 ]
  %v45837 = phi i64 [ %v45843, %block28609 ], [ %v45195, %edge4561 ]
  %v45840 = phi i64 [ %v45846, %block28609 ], [ %v45198, %edge4561 ]
  %t3285 = icmp eq i64 %v45198, %v45840
  %t3287.not119 = icmp eq i64 %v45840, 0
  %t3287.not = or i1 %t3285, %t3287.not119
  br i1 %t3287.not, label %block28614, label %edge4575

block28614:                                       ; preds = %block28608, %edge4575
  %t3291.not120 = icmp eq i64 %v45198, 0
  %t3291.not = or i1 %t3291.not120, %t3285
  br i1 %t3291.not, label %edge4586, label %edge4580

block28605:                                       ; preds = %edge4580, %block28610
  %v45823 = phi i64 [ %v45856, %block28610 ], [ %v45870, %edge4580 ]
  %t3293 = insertvalue { i64, i64 } poison, i64 %v45823, 0
  %t3294 = insertvalue { i64, i64 } %t3293, i64 2357, 1
  br label %common.ret

block28628:                                       ; preds = %edge4592, %edge4591
  %t3304 = zext i64 %v45178 to i128
  %v219843 = or disjoint i128 %t3304, 184467440737095516160
  store i128 %v219843, ptr %ss0, align 16
  %t3308 = zext i64 %v45180 to i128
  %v219845 = or disjoint i128 %t3308, 18446744073709551616
  store i128 %v219845, ptr %t1002, align 16
  %t3314 = zext i64 %v45183 to i128
  %t3315 = shl nuw i128 %t3314, 64
  %v219847 = or disjoint i128 %t3315, 10
  store i128 %v219847, ptr %t1007, align 16
  %v45919 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v45180, i64 4, i64 4, i64 %v3194)
  %t3318.not = icmp eq i64 %v45919, 0
  br i1 %t3318.not, label %edge4597, label %block28623

block28627:                                       ; preds = %edge4592, %edge4597
  %t3321.pre-phi = phi ptr [ %t3252, %edge4592 ], [ %.pre21704, %edge4597 ]
  %v45904 = phi i64 [ %v45178, %edge4592 ], [ %v45920, %edge4597 ]
  %v45906 = phi i64 [ %v45180, %edge4592 ], [ %v45921, %edge4597 ]
  %v45909 = phi i64 [ %v45183, %edge4592 ], [ %v45922, %edge4597 ]
  store i32 0, ptr %t3321.pre-phi, align 1
  br label %block28626

block28626:                                       ; preds = %edge4586, %block28627
  %v45898 = phi i64 [ %v45904, %block28627 ], [ %v45178, %edge4586 ]
  %v45900 = phi i64 [ %v45906, %block28627 ], [ %v45180, %edge4586 ]
  %v45903 = phi i64 [ %v45909, %block28627 ], [ %v45183, %edge4586 ]
  %t3322 = icmp eq i64 %v45183, %v45903
  %t3324.not121 = icmp eq i64 %v45903, 0
  %t3324.not = or i1 %t3322, %t3324.not121
  br i1 %t3324.not, label %block28632, label %edge4600

block28632:                                       ; preds = %block28626, %edge4600
  %t3328.not122 = icmp eq i64 %v45183, 0
  %t3328.not = or i1 %t3328.not122, %t3322
  br i1 %t3328.not, label %block2305.backedge, label %edge4603

block28623:                                       ; preds = %edge4606, %block28628
  %v45886 = phi i64 [ %v45919, %block28628 ], [ %v45933, %edge4606 ]
  %t3333 = insertvalue { i64, i64 } poison, i64 %v45886, 0
  %t3334 = insertvalue { i64, i64 } %t3333, i64 2360, 1
  br label %common.ret

block2378:                                        ; preds = %edge4539, %edge4773
  %v46660 = phi i64 [ %v47723.ph, %edge4773 ], [ %v10828.ph.ph, %edge4539 ]
  %v46662 = phi i64 [ %v47725.ph, %edge4773 ], [ %v10800.ph.ph, %edge4539 ]
  %v46665 = phi i64 [ %v47728.ph, %edge4773 ], [ %v10791.ph.ph, %edge4539 ]
  %v46672 = phi i64 [ %v47735.ph, %edge4773 ], [ %v40032, %edge4539 ]
  %v46674 = phi i64 [ %v47737.ph, %edge4773 ], [ %v40034, %edge4539 ]
  %v46677 = phi i64 [ %v47740.ph, %edge4773 ], [ %v40037, %edge4539 ]
  %v46684 = phi i64 [ %v47952, %edge4773 ], [ 0, %edge4539 ]
  %exitcond21630 = icmp eq i64 %v46684, 259
  br i1 %exitcond21630, label %block475.outer.outer, label %edge4658

block29463:                                       ; preds = %edge4692, %edge4691
  %t3368 = zext i64 %v46660 to i128
  %v219849 = or disjoint i128 %t3368, 4777706715090773868544
  store i128 %v219849, ptr %ss0, align 16
  %t3372 = zext i64 %v46662 to i128
  %v219851 = or disjoint i128 %t3372, 18446744073709551616
  store i128 %v219851, ptr %t1002, align 16
  %t3378 = zext i64 %v46665 to i128
  %t3379 = shl nuw i128 %t3378, 64
  %v219853 = or disjoint i128 %t3379, 259
  store i128 %v219853, ptr %t1007, align 16
  %v47550 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v46662, i64 4, i64 4, i64 %v3194)
  %t3382.not = icmp eq i64 %v47550, 0
  br i1 %t3382.not, label %edge4697, label %block29458

block29462:                                       ; preds = %edge4692, %edge4697
  %t3385.pre-phi = phi ptr [ %t3351, %edge4692 ], [ %.pre21708, %edge4697 ]
  %v47535 = phi i64 [ %v46660, %edge4692 ], [ %v47551, %edge4697 ]
  %v47537 = phi i64 [ %v46662, %edge4692 ], [ %v47552, %edge4697 ]
  %v47540 = phi i64 [ %v46665, %edge4692 ], [ %v47553, %edge4697 ]
  %v47555 = trunc nuw i64 %v47506 to i32
  store i32 %v47555, ptr %t3385.pre-phi, align 1
  %t3386 = icmp eq i64 %v46665, %v47540
  %t3388.not115 = icmp eq i64 %v47540, 0
  %t3388.not = or i1 %t3386, %t3388.not115
  br i1 %t3388.not, label %block29467, label %edge4700

block29467:                                       ; preds = %block29462, %edge4700
  %t3392.not = or i1 %t3363.not, %t3386
  br i1 %t3392.not, label %edge4709, label %edge4705

block29458:                                       ; preds = %edge4705, %block29463
  %v47517 = phi i64 [ %v47550, %block29463 ], [ %v47564, %edge4705 ]
  %t3394 = insertvalue { i64, i64 } poison, i64 %v47517, 0
  %t3395 = insertvalue { i64, i64 } %t3394, i64 2434, 1
  br label %common.ret

block29479:                                       ; preds = %edge4715, %edge4714
  %t3407 = zext i64 %v46672 to i128
  %v219855 = or disjoint i128 %t3407, 4777706715090773868544
  store i128 %v219855, ptr %ss0, align 16
  %t3411 = zext i64 %v46674 to i128
  %v219857 = or disjoint i128 %t3411, 18446744073709551616
  store i128 %v219857, ptr %t1002, align 16
  %t3417 = zext i64 %v46677 to i128
  %t3418 = shl nuw i128 %t3417, 64
  %v219859 = or disjoint i128 %t3418, 259
  store i128 %v219859, ptr %t1007, align 16
  %v47606 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v46674, i64 4, i64 4, i64 %v3194)
  %t3421.not = icmp eq i64 %v47606, 0
  br i1 %t3421.not, label %edge4720, label %block29474

block29478:                                       ; preds = %edge4715, %edge4720
  %t3424.pre-phi = phi ptr [ %t3344, %edge4715 ], [ %.pre21710, %edge4720 ]
  %v47591 = phi i64 [ %v46672, %edge4715 ], [ %v47607, %edge4720 ]
  %v47593 = phi i64 [ %v46674, %edge4715 ], [ %v47608, %edge4720 ]
  %v47596 = phi i64 [ %v46677, %edge4715 ], [ %v47609, %edge4720 ]
  store i32 0, ptr %t3424.pre-phi, align 1
  br label %block29477

block29477:                                       ; preds = %edge4709, %block29478
  %v47585 = phi i64 [ %v47591, %block29478 ], [ %v46672, %edge4709 ]
  %v47587 = phi i64 [ %v47593, %block29478 ], [ %v46674, %edge4709 ]
  %v47590 = phi i64 [ %v47596, %block29478 ], [ %v46677, %edge4709 ]
  %t3425 = icmp eq i64 %v46677, %v47590
  %t3427.not117 = icmp eq i64 %v47590, 0
  %t3427.not = or i1 %t3425, %t3427.not117
  br i1 %t3427.not, label %block29483, label %edge4723

block29483:                                       ; preds = %block29477, %edge4723
  %t3431.not118 = icmp eq i64 %v46677, 0
  %t3431.not = or i1 %t3431.not118, %t3425
  br i1 %t3431.not, label %edge4773, label %edge4726

block29474:                                       ; preds = %edge4729, %block29479
  %v47573 = phi i64 [ %v47606, %block29479 ], [ %v47620, %edge4729 ]
  %t3436 = insertvalue { i64, i64 } poison, i64 %v47573, 0
  %t3437 = insertvalue { i64, i64 } %t3436, i64 2436, 1
  br label %common.ret

block2462:                                        ; preds = %edge4502
  br i1 %v41763103.not.not33136, label %block475.backedge, label %edge4852

block475.backedge:                                ; preds = %block2462, %edge4337.block2462_crit_edge
  br label %block475

block2509:                                        ; preds = %block30781, %edge4902
  %v50450 = phi i64 [ %v40316, %edge4902 ], [ %v51049, %block30781 ]
  %v50453 = phi i64 [ %v50403, %edge4902 ], [ %v51241, %block30781 ]
  %t3463.not = icmp eq i64 %v50453, 0
  br i1 %t3463.not, label %edge4915, label %edge4917

block30781:                                       ; preds = %edge4931, %edge4918
  %v219287 = phi i64 [ 0, %edge4918 ], [ %v51042, %edge4931 ]
  %v51049 = sub i64 %v50640, %v219287
  %v51241 = add i64 %v50453, -1
  br label %block2509

block2587:                                        ; preds = %block2587.outer, %edge5008
  %v51759 = phi i64 [ %v52112, %edge5008 ], [ %v51759.ph, %block2587.outer ]
  %exitcond21631.not = icmp eq i64 %v51759, 259
  br i1 %exitcond21631.not, label %block2672, label %edge5008

block31759:                                       ; preds = %edge5041, %edge5040
  %t3523 = zext i64 %v51735.ph to i128
  %v219861 = or disjoint i128 %t3523, 4777706715090773868544
  store i128 %v219861, ptr %ss0, align 16
  %t3527 = zext i64 %v51737.ph to i128
  %v219863 = or disjoint i128 %t3527, 18446744073709551616
  store i128 %v219863, ptr %t1002, align 16
  %t3533 = zext i64 %v51740.ph to i128
  %t3534 = shl nuw i128 %t3533, 64
  %v219865 = or disjoint i128 %t3534, 259
  store i128 %v219865, ptr %t1007, align 16
  %v52554 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v51737.ph, i64 4, i64 4, i64 %v3194)
  %t3537.not = icmp eq i64 %v52554, 0
  br i1 %t3537.not, label %edge5046, label %block31754

block31758:                                       ; preds = %edge5041, %edge5046
  %t3540.pre-phi = phi ptr [ %t3506, %edge5041 ], [ %.pre21712, %edge5046 ]
  %v52539 = phi i64 [ %v51735.ph, %edge5041 ], [ %v52555, %edge5046 ]
  %v52541 = phi i64 [ %v51737.ph, %edge5041 ], [ %v52556, %edge5046 ]
  %v52544 = phi i64 [ %v51740.ph, %edge5041 ], [ %v52557, %edge5046 ]
  %v52559 = trunc nuw i64 %v52510 to i32
  store i32 %v52559, ptr %t3540.pre-phi, align 1
  %t3541 = icmp eq i64 %v51740.ph, %v52544
  %t3543.not125 = icmp eq i64 %v52544, 0
  %t3543.not = or i1 %t3541, %t3543.not125
  br i1 %t3543.not, label %block31763, label %edge5049

block31763:                                       ; preds = %block31758, %edge5049
  %t3547.not = or i1 %t3518.not, %t3541
  br i1 %t3547.not, label %edge5058, label %edge5054

block31754:                                       ; preds = %edge5054, %block31759
  %v52521 = phi i64 [ %v52554, %block31759 ], [ %v52568, %edge5054 ]
  %t3549 = insertvalue { i64, i64 } poison, i64 %v52521, 0
  %t3550 = insertvalue { i64, i64 } %t3549, i64 2645, 1
  br label %common.ret

block31775:                                       ; preds = %edge5064, %edge5063
  %t3562 = zext i64 %v51747.ph to i128
  %v219867 = or disjoint i128 %t3562, 4777706715090773868544
  store i128 %v219867, ptr %ss0, align 16
  %t3566 = zext i64 %v51749.ph to i128
  %v219869 = or disjoint i128 %t3566, 18446744073709551616
  store i128 %v219869, ptr %t1002, align 16
  %t3572 = zext i64 %v51752.ph to i128
  %t3573 = shl nuw i128 %t3572, 64
  %v219871 = or disjoint i128 %t3573, 259
  store i128 %v219871, ptr %t1007, align 16
  %v52610 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v51749.ph, i64 4, i64 4, i64 %v3194)
  %t3576.not = icmp eq i64 %v52610, 0
  br i1 %t3576.not, label %edge5069, label %block31770

block31774:                                       ; preds = %edge5064, %edge5069
  %t3579.pre-phi = phi ptr [ %t3499, %edge5064 ], [ %.pre21714, %edge5069 ]
  %v52595 = phi i64 [ %v51747.ph, %edge5064 ], [ %v52611, %edge5069 ]
  %v52597 = phi i64 [ %v51749.ph, %edge5064 ], [ %v52612, %edge5069 ]
  %v52600 = phi i64 [ %v51752.ph, %edge5064 ], [ %v52613, %edge5069 ]
  store i32 0, ptr %t3579.pre-phi, align 1
  br label %block31773

block31773:                                       ; preds = %edge5058, %block31774
  %v52589 = phi i64 [ %v52595, %block31774 ], [ %v51747.ph, %edge5058 ]
  %v52591 = phi i64 [ %v52597, %block31774 ], [ %v51749.ph, %edge5058 ]
  %v52594 = phi i64 [ %v52600, %block31774 ], [ %v51752.ph, %edge5058 ]
  %t3580 = icmp eq i64 %v51752.ph, %v52594
  %t3582.not127 = icmp eq i64 %v52594, 0
  %t3582.not = or i1 %t3580, %t3582.not127
  br i1 %t3582.not, label %block31779, label %edge5072

block31779:                                       ; preds = %block31773, %edge5072
  %t3586.not128 = icmp eq i64 %v51752.ph, 0
  %t3586.not = or i1 %t3586.not128, %t3580
  br i1 %t3586.not, label %block2587.outer.backedge, label %edge5075

block2587.outer.backedge:                         ; preds = %block31779, %edge5078, %edge5075
  br label %block2587.outer

block31770:                                       ; preds = %edge5078, %block31775
  %v52577 = phi i64 [ %v52610, %block31775 ], [ %v52624, %edge5078 ]
  %t3591 = insertvalue { i64, i64 } poison, i64 %v52577, 0
  %t3592 = insertvalue { i64, i64 } %t3591, i64 2647, 1
  br label %common.ret

block2672:                                        ; preds = %block2587, %block2672.backedge
  %v53306 = phi i64 [ %v54011, %block2672.backedge ], [ %v40044, %block2587 ]
  %v53308 = phi i64 [ %v54013, %block2672.backedge ], [ %v40046, %block2587 ]
  %v53311 = phi i64 [ %v54016, %block2672.backedge ], [ %v40049, %block2587 ]
  %v53321 = phi i64 [ %v53948, %block2672.backedge ], [ %v10830.ph.ph, %block2587 ]
  %v53323 = phi i64 [ %v53950, %block2672.backedge ], [ %v10804.ph.ph, %block2587 ]
  %v53326 = phi i64 [ %v53953, %block2672.backedge ], [ %v10793.ph.ph, %block2587 ]
  %v53333 = phi i64 [ %v53688, %block2672.backedge ], [ 0, %block2587 ]
  %exitcond21632.not = icmp eq i64 %v53333, 10
  br i1 %exitcond21632.not, label %edge5147, label %edge5146

block32629:                                       ; preds = %edge5175, %edge5174
  %t3616 = zext i64 %v53321 to i128
  %v219873 = or disjoint i128 %t3616, 184467440737095516160
  store i128 %v219873, ptr %ss0, align 16
  %t3620 = zext i64 %v53323 to i128
  %v219875 = or disjoint i128 %t3620, 18446744073709551616
  store i128 %v219875, ptr %t1002, align 16
  %t3626 = zext i64 %v53326 to i128
  %t3627 = shl nuw i128 %t3626, 64
  %v219877 = or disjoint i128 %t3627, 10
  store i128 %v219877, ptr %t1007, align 16
  %v53969 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v53323, i64 4, i64 4, i64 %v3194)
  %t3630.not = icmp eq i64 %v53969, 0
  br i1 %t3630.not, label %edge5180, label %block32624

block32628:                                       ; preds = %edge5175, %edge5180
  %t3633.pre-phi = phi ptr [ %t3599, %edge5175 ], [ %.pre21718, %edge5180 ]
  %v53954 = phi i64 [ %v53321, %edge5175 ], [ %v53970, %edge5180 ]
  %v53956 = phi i64 [ %v53323, %edge5175 ], [ %v53971, %edge5180 ]
  %v53959 = phi i64 [ %v53326, %edge5175 ], [ %v53972, %edge5180 ]
  %v53974 = trunc nuw i64 %v53925 to i32
  store i32 %v53974, ptr %t3633.pre-phi, align 1
  br label %block32627

block32627:                                       ; preds = %edge5169, %block32628
  %v53948 = phi i64 [ %v53954, %block32628 ], [ %v53321, %edge5169 ]
  %v53950 = phi i64 [ %v53956, %block32628 ], [ %v53323, %edge5169 ]
  %v53953 = phi i64 [ %v53959, %block32628 ], [ %v53326, %edge5169 ]
  %t3634 = icmp eq i64 %v53326, %v53953
  %t3636.not129 = icmp eq i64 %v53953, 0
  %t3636.not = or i1 %t3634, %t3636.not129
  br i1 %t3636.not, label %block32633, label %edge5183

block32633:                                       ; preds = %block32627, %edge5183
  %t3640.not130 = icmp eq i64 %v53326, 0
  %t3640.not = or i1 %t3640.not130, %t3634
  br i1 %t3640.not, label %edge5194, label %edge5188

block32624:                                       ; preds = %edge5188, %block32629
  %v53936 = phi i64 [ %v53969, %block32629 ], [ %v53983, %edge5188 ]
  %t3642 = insertvalue { i64, i64 } poison, i64 %v53936, 0
  %t3643 = insertvalue { i64, i64 } %t3642, i64 2728, 1
  br label %common.ret

block32647:                                       ; preds = %edge5200, %edge5199
  %t3653 = zext i64 %v53306 to i128
  %v219879 = or disjoint i128 %t3653, 184467440737095516160
  store i128 %v219879, ptr %ss0, align 16
  %t3657 = zext i64 %v53308 to i128
  %v219881 = or disjoint i128 %t3657, 18446744073709551616
  store i128 %v219881, ptr %t1002, align 16
  %t3663 = zext i64 %v53311 to i128
  %t3664 = shl nuw i128 %t3663, 64
  %v219883 = or disjoint i128 %t3664, 10
  store i128 %v219883, ptr %t1007, align 16
  %v54032 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v53308, i64 4, i64 4, i64 %v3194)
  %t3667.not = icmp eq i64 %v54032, 0
  br i1 %t3667.not, label %edge5205, label %block32642

block32646:                                       ; preds = %edge5200, %edge5205
  %t3670.pre-phi = phi ptr [ %t3601, %edge5200 ], [ %.pre21716, %edge5205 ]
  %v54017 = phi i64 [ %v53306, %edge5200 ], [ %v54033, %edge5205 ]
  %v54019 = phi i64 [ %v53308, %edge5200 ], [ %v54034, %edge5205 ]
  %v54022 = phi i64 [ %v53311, %edge5200 ], [ %v54035, %edge5205 ]
  store i32 0, ptr %t3670.pre-phi, align 1
  br label %block32645

block32645:                                       ; preds = %edge5194, %block32646
  %v54011 = phi i64 [ %v54017, %block32646 ], [ %v53306, %edge5194 ]
  %v54013 = phi i64 [ %v54019, %block32646 ], [ %v53308, %edge5194 ]
  %v54016 = phi i64 [ %v54022, %block32646 ], [ %v53311, %edge5194 ]
  %t3671 = icmp eq i64 %v53311, %v54016
  %t3673.not131 = icmp eq i64 %v54016, 0
  %t3673.not = or i1 %t3671, %t3673.not131
  br i1 %t3673.not, label %block32651, label %edge5208

block32651:                                       ; preds = %block32645, %edge5208
  %t3677.not132 = icmp eq i64 %v53311, 0
  %t3677.not = or i1 %t3677.not132, %t3671
  br i1 %t3677.not, label %block2672.backedge, label %edge5211

block32642:                                       ; preds = %edge5214, %block32647
  %v53999 = phi i64 [ %v54032, %block32647 ], [ %v54046, %edge5214 ]
  %t3682 = insertvalue { i64, i64 } poison, i64 %v53999, 0
  %t3683 = insertvalue { i64, i64 } %t3682, i64 2731, 1
  br label %common.ret

block33335:                                       ; preds = %edge5365
  %t3734 = insertvalue { i64, i64 } poison, i64 %v55963, 0
  %t3735 = insertvalue { i64, i64 } %t3734, i64 2811, 1
  br label %common.ret

block2804:                                        ; preds = %edge5367, %edge5368
  br i1 %t23.not, label %block2806.preheader, label %edge5374

block2806.preheader:                              ; preds = %block2804, %edge5374
  br label %block2806

block2806:                                        ; preds = %block2806.preheader, %edge5530
  %v56479 = phi i64 [ %v57489, %edge5530 ], [ %v55968, %block2806.preheader ]
  %v56513 = phi i64 [ %v57486, %edge5530 ], [ %v55967, %block2806.preheader ]
  %v56516 = phi i64 [ %v57724, %edge5530 ], [ 0, %block2806.preheader ]
  %v56521 = phi i64 [ %v57484, %edge5530 ], [ %v55966, %block2806.preheader ]
  %t3739.not = icmp eq i64 %v56479, 0
  br i1 %t3739.not, label %edge5416, label %edge5382

block33393:                                       ; preds = %edge5389, %edge5390
  %t3778.pn = phi { i64, i64 } [ %t3778, %edge5390 ], [ %t3779, %edge5389 ]
  %v56485 = extractvalue { i64, i64 } %t3778.pn, 0
  %t3780.not = icmp eq i64 %v56485, 0
  br i1 %t3780.not, label %edge5394, label %edge5393

trap3803:                                         ; preds = %edge5445
  call void @llvm.trap()
  unreachable

ok3803:                                           ; preds = %edge5445
  %v56998 = udiv i64 %v56762, %v1399
  %v56999 = mul i64 %v56998, %v1399
  %v57000 = sub nuw i64 %v56762, %v56999
  %v57001 = mul i64 %v57000, %v1400
  %v57002 = add i64 %v57001, %v56998
  br label %block33729

block33729:                                       ; preds = %edge5441, %ok3803
  %v56997 = phi i64 [ %v57002, %ok3803 ], [ %v56762, %edge5441 ]
  %v57004 = add i64 %v56997, %v1396
  %t3804 = inttoptr i64 %v57004 to ptr
  %t3805 = load i8, ptr %t3804, align 1
  %v57005 = zext i8 %t3805 to i64
  %2 = shl nuw nsw i64 %v57005, 2
  br label %edge5477

block33914:                                       ; preds = %edge5486, %edge5485
  %t3826 = zext i64 %v56521 to i128
  %v219897 = or disjoint i128 %t3826, 5902958103587056517120
  store i128 %v219897, ptr %ss0, align 16
  %t3830 = zext i64 %v56513 to i128
  %v219899 = or disjoint i128 %t3830, 18446744073709551616
  store i128 %v219899, ptr %t1002, align 16
  %t3836 = zext i64 %v56479 to i128
  %t3837 = shl nuw i128 %t3836, 64
  %v219901 = or disjoint i128 %t3837, 320
  store i128 %v219901, ptr %t1007, align 16
  %v57499 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v56513, i64 4, i64 4, i64 %v3194)
  %t3840.not = icmp eq i64 %v57499, 0
  br i1 %t3840.not, label %edge5491, label %block33909

block33912:                                       ; preds = %edge5491, %edge5486
  %t3843.pre-phi = phi ptr [ %.pre21720, %edge5491 ], [ %t3809, %edge5486 ]
  %v57484 = phi i64 [ %v57500, %edge5491 ], [ %v56521, %edge5486 ]
  %v57486 = phi i64 [ %v57501, %edge5491 ], [ %v56513, %edge5486 ]
  %v57489 = phi i64 [ %v57502, %edge5491 ], [ %v56479, %edge5486 ]
  store i32 %v57457, ptr %t3843.pre-phi, align 1
  %t3844 = icmp eq i64 %v56479, %v57489
  %t3846.not136 = icmp eq i64 %v57489, 0
  %t3846.not = or i1 %t3844, %t3846.not136
  br i1 %t3846.not, label %edge5505, label %edge5494

block33909:                                       ; preds = %block33914
  %t3850 = insertvalue { i64, i64 } poison, i64 %v57499, 0
  %t3851 = insertvalue { i64, i64 } %t3850, i64 2881, 1
  br label %common.ret

block33934:                                       ; preds = %edge5509, %edge5506
  %t3859.not = icmp eq i64 %v56479, 0
  br i1 %t3859.not, label %edge5557, label %edge5513

block33929:                                       ; preds = %edge5516, %edge5509, %edge5523
  %v57530 = phi i64 [ %v57545, %edge5523 ], [ %v57537, %edge5509 ], [ %v57541, %edge5516 ]
  %t3866 = insertvalue { i64, i64 } poison, i64 %v57530, 0
  %t3867 = insertvalue { i64, i64 } %t3866, i64 2886, 1
  br label %common.ret

block34286:                                       ; preds = %edge5557, %edge5573
  %t3875.not22170 = phi i1 [ false, %edge5573 ], [ true, %edge5557 ]
  %v5810622167 = phi i32 [ %v5810622159, %edge5573 ], [ %v58106, %edge5557 ]
  %v5777522162 = phi i64 [ %v56479, %edge5573 ], [ 0, %edge5557 ]
  %t3880 = zext i64 %v56521 to i128
  %v219903 = or disjoint i128 %t3880, 5902958103587056517120
  store i128 %v219903, ptr %ss0, align 16
  %t3884 = zext i64 %v56513 to i128
  %v219905 = or disjoint i128 %t3884, 18446744073709551616
  store i128 %v219905, ptr %t1002, align 16
  %t3890 = zext i64 %v5777522162 to i128
  %t3891 = shl nuw i128 %t3890, 64
  %v219907 = or disjoint i128 %t3891, 320
  store i128 %v219907, ptr %t1007, align 16
  %v58166 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v56513, i64 4, i64 4, i64 %v3194)
  %t3894.not = icmp eq i64 %v58166, 0
  br i1 %t3894.not, label %block34285, label %block34281

block34285:                                       ; preds = %block34286
  %v58167 = load i64, ptr %ss1, align 8
  %v58168 = load i64, ptr %t606, align 8
  %v58169 = load i64, ptr %t607, align 8
  %.pre21721 = add i64 %v58167, 1024
  %.pre21722 = inttoptr i64 %.pre21721 to ptr
  store i32 %v5810622167, ptr %.pre21722, align 1
  %t3898 = icmp eq i64 %v5777522162, %v58169
  %t3900.not138 = icmp eq i64 %v58169, 0
  %t3900.not = or i1 %t3898, %t3900.not138
  br i1 %t3900.not, label %block34290, label %edge5581

block34290:                                       ; preds = %block34285, %edge5581
  %t3904.not = or i1 %t3875.not22170, %t3898
  br i1 %t3904.not, label %edge5595, label %edge5584

block34281:                                       ; preds = %edge5587, %block34286
  %v58133 = phi i64 [ %v58166, %block34286 ], [ %v58180, %edge5587 ]
  %t3909 = insertvalue { i64, i64 } poison, i64 %v58133, 0
  %t3910 = insertvalue { i64, i64 } %t3909, i64 2923, 1
  br label %common.ret

block2907:                                        ; preds = %block2907.backedge, %edge5647
  %v64643 = phi i64 [ 1, %edge5647 ], [ %v219164, %block2907.backedge ]
  %v64742 = phi i64 [ 0, %edge5647 ], [ %v167750, %block2907.backedge ]
  %v64745 = phi i64 [ 0, %edge5647 ], [ %v167785, %block2907.backedge ]
  %v64748 = phi i64 [ %v219208, %edge5647 ], [ %v7457922180, %block2907.backedge ]
  %v64749 = phi i64 [ %v219209, %edge5647 ], [ %v7458022184, %block2907.backedge ]
  %v64750 = phi i64 [ %v219210, %edge5647 ], [ %v7458122187, %block2907.backedge ]
  %v64751 = phi i64 [ %v219211, %edge5647 ], [ %v7458222191, %block2907.backedge ]
  %v64752 = phi i64 [ %v219212, %edge5647 ], [ %v7458322195, %block2907.backedge ]
  %v64753 = phi i64 [ %v219213, %edge5647 ], [ %v7458422199, %block2907.backedge ]
  %v64760 = phi i64 [ %v219221, %edge5647 ], [ %v219139, %block2907.backedge ]
  %v64762 = phi i64 [ %v219223, %edge5647 ], [ %v219141, %block2907.backedge ]
  %v64765 = phi i64 [ %v219226, %edge5647 ], [ %v219120, %block2907.backedge ]
  %v64772 = phi i64 [ %v219227, %edge5647 ], [ %v147323, %block2907.backedge ]
  %v64774 = phi i64 [ %v219229, %edge5647 ], [ %v147329, %block2907.backedge ]
  %v64777 = phi i64 [ %v219232, %edge5647 ], [ %v140768, %block2907.backedge ]
  %v64784 = phi i64 [ -1, %edge5647 ], [ %spec.select361079, %block2907.backedge ]
  %v64787 = phi i64 [ 10, %edge5647 ], [ %v219097, %block2907.backedge ]
  %v64790 = phi i64 [ %v58151223362235222389, %edge5647 ], [ %v80928, %block2907.backedge ]
  %v64795 = phi i64 [ %v58156223382235022393, %edge5647 ], [ %v80922, %block2907.backedge ]
  %v64802 = phi i64 [ %v58151223362235222389, %edge5647 ], [ %v132163, %block2907.backedge ]
  %v64803 = phi i64 [ 320, %edge5647 ], [ %v132164, %block2907.backedge ]
  %v64806 = phi i64 [ 320, %edge5647 ], [ %v132167, %block2907.backedge ]
  %v64807 = phi i64 [ %v58156223382235022393, %edge5647 ], [ %v132168, %block2907.backedge ]
  %v64847 = phi i64 [ 0, %edge5647 ], [ %spec.select4, %block2907.backedge ]
  %v64850 = phi i64 [ -1, %edge5647 ], [ %v219151, %block2907.backedge ]
  %v64913 = phi i64 [ %v58151223362235222389, %edge5647 ], [ %v132179, %block2907.backedge ]
  %v64918 = phi i64 [ %v58156223382235022393, %edge5647 ], [ %v132184, %block2907.backedge ]
  %v64925 = phi i64 [ %v58151223362235222389, %edge5647 ], [ %v132185, %block2907.backedge ]
  %v64926 = phi i64 [ 320, %edge5647 ], [ %v132186, %block2907.backedge ]
  %v64930 = phi i64 [ %v58156223382235022393, %edge5647 ], [ %v132190, %block2907.backedge ]
  %v64937 = phi i64 [ %v58151223362235222389, %edge5647 ], [ %v132191, %block2907.backedge ]
  %v64942 = phi i64 [ %v58156223382235022393, %edge5647 ], [ %v132196, %block2907.backedge ]
  %v64949 = phi i64 [ 0, %edge5647 ], [ %v132197, %block2907.backedge ]
  %v64952 = phi i64 [ 0, %edge5647 ], [ %v132198, %block2907.backedge ]
  %v64955 = phi i64 [ 0, %edge5647 ], [ %v132199, %block2907.backedge ]
  %v64958 = phi i64 [ 0, %edge5647 ], [ %v132200, %block2907.backedge ]
  %t3926 = icmp eq i64 %v64643, 1
  %v64969 = zext i1 %t3926 to i64
  %t3927 = icmp eq i64 %v64643, 2
  %v64985 = zext i1 %t3927 to i64
  %t3928 = icmp eq i64 %v64643, 4
  %v65001 = zext i1 %t3928 to i64
  %3 = and i64 %v64643, -2
  %v65022140 = icmp eq i64 %3, 2
  br i1 %v65022140, label %edge5744, label %edge5761

block35292:                                       ; preds = %edge5953, %edge5957, %edge5950
  %v67017 = phi i64 [ %v64750, %edge5950 ], [ %29, %edge5957 ], [ 258, %edge5953 ]
  %t4053.not = icmp eq i64 %v64753, 0
  br i1 %t4053.not, label %block35303, label %edge5970

block35303:                                       ; preds = %edge5970, %block35292
  %t4058 = zext i64 %v64748 to i128
  store i128 %t4058, ptr %ss0, align 16
  %t4062 = zext i64 %v64750 to i128
  %v219917 = or disjoint i128 %t4062, 18446744073709551616
  store i128 %v219917, ptr %t1002, align 16
  %t4068 = zext i64 %v64753 to i128
  %t4069 = shl nuw i128 %t4068, 64
  store i128 %t4069, ptr %t1007, align 16
  %v67057 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v67017, i64 8, i64 8, i64 %v3194)
  %t4072.not = icmp eq i64 %v67057, 0
  br i1 %t4072.not, label %edge5975, label %block35287

block35309:                                       ; preds = %edge5977, %edge5980
  %t4082.not = or i1 %t4053.not, %t4076
  br i1 %t4082.not, label %edge6154, label %edge5985

block35287:                                       ; preds = %edge5985, %block35303
  %v66517 = phi i64 [ %v67057, %block35303 ], [ %v67075, %edge5985 ]
  %t4084 = insertvalue { i64, i64 } poison, i64 %v66517, 0
  %t4085 = insertvalue { i64, i64 } %t4084, i64 3001, 1
  br label %common.ret

block35510:                                       ; preds = %edge6154, %edge6156
  %t4145.not = icmp eq i64 %v66494, 0
  br i1 %t4145.not, label %edge6169, label %edge6159

block2989:                                        ; preds = %block2989.backedge, %edge6169
  %v67328 = phi i64 [ %v67047, %edge6169 ], [ %v73704, %block2989.backedge ]
  %v67396 = phi i64 [ 258, %edge6169 ], [ %v73669, %block2989.backedge ]
  %v67402 = phi i64 [ %v55001, %edge6169 ], [ %v67760, %block2989.backedge ]
  %v67403 = phi i64 [ %v54961, %edge6169 ], [ %v69324, %block2989.backedge ]
  %v67404 = phi i64 [ %v67042, %edge6169 ], [ %v73699, %block2989.backedge ]
  %v67414 = phi i64 [ %v67044, %edge6169 ], [ %v73701, %block2989.backedge ]
  %t4148.not = icmp eq i64 %v67328, 0
  br i1 %t4148.not, label %edge6220, label %edge6170

block35544:                                       ; preds = %edge6180, %edge6179, %edge6181
  %t4216.pn = phi { i64, i64 } [ %t4216, %edge6181 ], [ %t4217, %edge6179 ], [ %t4218, %edge6180 ]
  %v67345 = extractvalue { i64, i64 } %t4216.pn, 0
  %t4219.not = icmp eq i64 %v67345, 0
  br i1 %t4219.not, label %edge6186, label %edge6185

block35555:                                       ; preds = %edge6190, %edge6186
  br i1 %t4145.not, label %edge5769, label %edge6196

block35536:                                       ; preds = %edge6196, %edge6190
  %v67327 = phi i64 [ %v67384, %edge6190 ], [ %v67388, %edge6196 ]
  %t4237 = insertvalue { i64, i64 } poison, i64 %v67327, 0
  %t4238 = insertvalue { i64, i64 } %t4237, i64 3009, 1
  br label %common.ret

block3108:                                        ; preds = %edge6781, %edge6331
  %v69377 = phi i64 [ %v69324, %edge6331 ], [ %v72858, %edge6781 ]
  %v69382 = phi i64 [ 0, %edge6331 ], [ %umax21633, %edge6781 ]
  %v69387 = phi i64 [ %v69265, %edge6331 ], [ %v71074, %edge6781 ]
  %v69392 = phi i64 [ %v69308, %edge6331 ], [ %v71079, %edge6781 ]
  %t4365.not = icmp eq i64 %v69377, %v67765
  br i1 %t4365.not, label %edge6805, label %edge6790

block37800:                                       ; preds = %edge6495, %edge6492
  %t4291.not = icmp eq i64 %v219198, 0
  br i1 %t4291.not, label %block37808, label %edge6501

block37808:                                       ; preds = %block37800, %edge6501
  %t4295.not = icmp eq i64 %v54920, 0
  br i1 %t4295.not, label %block37812, label %edge6513

block37812:                                       ; preds = %edge6513, %block37808
  br i1 %t3701.not, label %block37816, label %edge6519

block37816:                                       ; preds = %edge6519, %block37812
  %t4299.not = icmp eq i64 %v54944, 0
  br i1 %t4299.not, label %block37820, label %edge6525

block37820:                                       ; preds = %edge6525, %block37816
  br i1 %t3704.not, label %block37824, label %edge6531

block37824:                                       ; preds = %edge6531, %block37820
  %t4303.not = icmp eq i64 %v54956, 0
  br i1 %t4303.not, label %block37828, label %edge6537

block37828:                                       ; preds = %edge6537, %block37824
  br i1 %t7555.not, label %block37832, label %edge6543

block37832:                                       ; preds = %edge6543, %block37828
  br i1 %t7638.not, label %block37836, label %edge6549

block37836:                                       ; preds = %edge6549, %block37832
  br i1 %t3704.not, label %edge6563, label %edge6553

block37860:                                       ; preds = %edge6566, %edge6563
  %t4316.not = icmp eq i64 %v64777, 0
  br i1 %t4316.not, label %block37864, label %edge6572

block37864:                                       ; preds = %edge6572, %block37860
  %t4318.not = icmp eq i64 %v64795, 0
  br i1 %t4318.not, label %block37868, label %edge6578

block37868:                                       ; preds = %edge6578, %block37864
  %t4320.not = icmp eq i64 %v64807, 0
  br i1 %t4320.not, label %block37872, label %edge6584

block37872:                                       ; preds = %edge6584, %block37868
  br i1 %t3701.not, label %block37876, label %edge6590

block37876:                                       ; preds = %edge6590, %block37872
  br i1 %t3900.not138223402234922395, label %edge6605, label %edge6594

block37904:                                       ; preds = %edge6608, %edge6605
  %t4331.not = icmp eq i64 %v64930, 0
  br i1 %t4331.not, label %block37908, label %edge6614

block37908:                                       ; preds = %edge6614, %block37904
  %t4333.not = icmp eq i64 %v64942, 0
  br i1 %t4333.not, label %block37912, label %edge6620

block37912:                                       ; preds = %edge6620, %block37908
  br i1 %t4145.not, label %block37916, label %edge6626

block37916:                                       ; preds = %edge6626, %block37912
  br i1 %t4148.not, label %block37920, label %edge6632

block37920:                                       ; preds = %edge6632, %block37916
  br i1 %t3701.not, label %block37924, label %edge6638

block37924:                                       ; preds = %edge6638, %block37920
  br i1 %t4145.not, label %common.ret, label %edge6644

block37732:                                       ; preds = %edge6644, %edge6638, %edge6632, %edge6626, %edge6620, %edge6614, %edge6608, %edge6597, %edge6590, %edge6584, %edge6578, %edge6572, %edge6566, %edge6556, %edge6549, %edge6543, %edge6537, %edge6531, %edge6525, %edge6519, %edge6513, %edge6501, %edge6495
  %v70799 = phi i64 [ %v70993, %edge6638 ], [ %v70869, %edge6495 ], [ %v70873, %edge6501 ], [ %v70997, %edge6644 ], [ %v70881, %edge6513 ], [ %v70885, %edge6519 ], [ %v70889, %edge6525 ], [ %v70893, %edge6531 ], [ %v70897, %edge6537 ], [ %v70901, %edge6543 ], [ %v70905, %edge6549 ], [ %v70909, %edge6556 ], [ %v70929, %edge6566 ], [ %v70933, %edge6572 ], [ %v70937, %edge6578 ], [ %v70941, %edge6584 ], [ %v70945, %edge6590 ], [ %v70949, %edge6597 ], [ %v70973, %edge6608 ], [ %v70977, %edge6614 ], [ %v70981, %edge6620 ], [ %v70985, %edge6626 ], [ %v70989, %edge6632 ]
  %t4343 = insertvalue { i64, i64 } poison, i64 %v70799, 0
  %t4344 = insertvalue { i64, i64 } %t4343, i64 3236, 1
  br label %common.ret

block3215:                                        ; preds = %edge6726, %edge6652
  %v71069 = phi i64 [ %v69382, %edge6652 ], [ %v72435, %edge6726 ]
  %v71074 = phi i64 [ %v69387, %edge6652 ], [ %spec.select30, %edge6726 ]
  %v71079 = phi i64 [ %v69392, %edge6652 ], [ %v72439, %edge6726 ]
  %exitcond21634.not = icmp eq i64 %v71069, %umax21633
  br i1 %exitcond21634.not, label %edge6781, label %edge6768

block39353:                                       ; preds = %edge6820, %edge6824, %edge6833
  %v73675 = phi i64 [ %v73680, %edge6824 ], [ %v73697, %edge6833 ], [ 8, %edge6820 ]
  %t4395 = call i64 @llvm.umax.i64(i64 %v73675, i64 %v73669)
  br label %block39350

block39350:                                       ; preds = %edge6817, %block39353
  %v73674 = phi i64 [ %t4395, %block39353 ], [ %v67414, %edge6817 ]
  br i1 %t4148.not, label %block39361, label %edge6837

block39361:                                       ; preds = %edge6837, %block39350
  %t4401 = zext i64 %v67404 to i128
  %t4402 = zext i64 %v67396 to i128
  %t4403 = shl nuw i128 %t4402, 64
  %v219953 = or disjoint i128 %t4403, %t4401
  store i128 %v219953, ptr %ss0, align 16
  %t4405 = zext i64 %v67414 to i128
  %v219955 = or disjoint i128 %t4405, 18446744073709551616
  store i128 %v219955, ptr %t1002, align 16
  %t4411 = zext i64 %v67328 to i128
  %t4412 = shl nuw i128 %t4411, 64
  %v219957 = or disjoint i128 %t4412, %t4402
  store i128 %v219957, ptr %t1007, align 16
  %v73714 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v73674, i64 8, i64 8, i64 %v3194)
  %t4415.not = icmp eq i64 %v73714, 0
  br i1 %t4415.not, label %edge6842, label %block39346

block39360:                                       ; preds = %edge6837, %edge6842
  %v73699 = phi i64 [ %v73715, %edge6842 ], [ %v67404, %edge6837 ]
  %v73701 = phi i64 [ %v73716, %edge6842 ], [ %v67414, %edge6837 ]
  %v73704 = phi i64 [ %v73717, %edge6842 ], [ %v67328, %edge6837 ]
  %v73718 = shl i64 %v67396, 3
  %v73719 = add i64 %v73699, %v73718
  %t4418 = inttoptr i64 %v73719 to ptr
  store i64 %v73661, ptr %t4418, align 1
  %t4419 = icmp eq i64 %v67328, %v73704
  %t4421.not146 = icmp eq i64 %v73704, 0
  %t4421.not = or i1 %t4419, %t4421.not146
  br i1 %t4421.not, label %edge6863, label %edge6844

block39346:                                       ; preds = %block39361, %edge6824, %edge6805
  %v73663 = phi i64 [ %v73714, %block39361 ], [ 1, %edge6805 ], [ 1, %edge6824 ]
  %t4425 = insertvalue { i64, i64 } poison, i64 %v73663, 0
  %t4426 = insertvalue { i64, i64 } %t4425, i64 3396, 1
  br label %common.ret

block39399:                                       ; preds = %edge6868, %edge6865
  br i1 %t4145.not, label %edge6878, label %edge6874

block39394:                                       ; preds = %edge6882, %edge6874, %edge6868, %edge6890
  %v73782 = phi i64 [ %v73811, %edge6890 ], [ %v73799, %edge6874 ], [ %v73795, %edge6868 ], [ %v73807, %edge6882 ]
  %t4446 = insertvalue { i64, i64 } poison, i64 %v73782, 0
  %t4447 = insertvalue { i64, i64 } %t4446, i64 3407, 1
  br label %common.ret

block39892:                                       ; preds = %edge6966
  %t4483 = insertvalue { i64, i64 } poison, i64 %v75292, 0
  %t4484 = insertvalue { i64, i64 } %t4483, i64 3435, 1
  br label %common.ret

block3417:                                        ; preds = %block3417.preheader, %edge7325
  %v75597 = phi i64 [ %v79117779, %edge7325 ], [ %v75297, %block3417.preheader ]
  %v75629 = phi i64 [ %v79114778, %edge7325 ], [ %v75296, %block3417.preheader ]
  %v75631 = phi i64 [ %v79109780, %edge7325 ], [ 0, %block3417.preheader ]
  %v75637 = phi i64 [ %v79112777, %edge7325 ], [ %v75295, %block3417.preheader ]
  %t4485.not = icmp eq i64 %v75597, 0
  br i1 %t4485.not, label %edge7025, label %edge6987

block40943:                                       ; preds = %edge7099, %edge7070
  %t4579 = zext i64 %v75637 to i128
  %v219979 = or disjoint i128 %t4579, 5902958103587056517120
  store i128 %v219979, ptr %ss0, align 16
  %t4583 = zext i64 %v75629 to i128
  %v219981 = or disjoint i128 %t4583, 18446744073709551616
  store i128 %v219981, ptr %t1002, align 16
  %t4589 = zext i64 %v75597 to i128
  %t4590 = shl nuw i128 %t4589, 64
  %v219983 = or disjoint i128 %t4590, 320
  store i128 %v219983, ptr %t1007, align 16
  %v77291 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v75629, i64 4, i64 4, i64 %v3194)
  %t4593.not = icmp eq i64 %v77291, 0
  br i1 %t4593.not, label %edge7104, label %edge7103

block40942:                                       ; preds = %edge7099, %edge7104
  %t4596.pre-phi = phi ptr [ %t4567, %edge7099 ], [ %.pre21770, %edge7104 ]
  %v77276 = phi i64 [ %v75637, %edge7099 ], [ %v77292, %edge7104 ]
  %v77278 = phi i64 [ %v75629, %edge7099 ], [ %v77293, %edge7104 ]
  %v77281 = phi i64 [ %v75597, %edge7099 ], [ %v77294, %edge7104 ]
  store i32 %v77240, ptr %t4596.pre-phi, align 1
  %t4597 = icmp eq i64 %v75597, %v77281
  %t4599.not154 = icmp eq i64 %v77281, 0
  %t4599.not = or i1 %t4597, %t4599.not154
  br i1 %t4599.not, label %block40947, label %edge7107

block40947:                                       ; preds = %block40942, %edge7107
  %or.cond1305 = or i1 %t4485.not, %t4597
  br i1 %or.cond1305, label %edge7325, label %edge7305

block41444:                                       ; preds = %edge7174, %edge7173
  %t4626 = zext i64 %v75637 to i128
  %v219985 = or disjoint i128 %t4626, 5902958103587056517120
  store i128 %v219985, ptr %ss0, align 16
  %t4630 = zext i64 %v75629 to i128
  %v219987 = or disjoint i128 %t4630, 18446744073709551616
  store i128 %v219987, ptr %t1002, align 16
  %t4636 = zext i64 %v75597 to i128
  %t4637 = shl nuw i128 %t4636, 64
  %v219989 = or disjoint i128 %t4637, 320
  store i128 %v219989, ptr %t1007, align 16
  %v78174 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v75629, i64 4, i64 4, i64 %v3194)
  %t4640.not = icmp eq i64 %v78174, 0
  br i1 %t4640.not, label %edge7179, label %block41439

block41442:                                       ; preds = %edge7179, %edge7174
  %t4643.pre-phi = phi ptr [ %.pre21772, %edge7179 ], [ %t4613, %edge7174 ]
  %v78159 = phi i64 [ %v78175, %edge7179 ], [ %v75637, %edge7174 ]
  %v78161 = phi i64 [ %v78176, %edge7179 ], [ %v75629, %edge7174 ]
  %v78164 = phi i64 [ %v78177, %edge7179 ], [ %v75597, %edge7174 ]
  store i32 %42, ptr %t4643.pre-phi, align 1
  %t4644 = icmp eq i64 %v75597, %v78164
  %t4646.not151 = icmp eq i64 %v78164, 0
  %t4646.not = or i1 %t4644, %t4646.not151
  br i1 %t4646.not, label %block41448, label %edge7182

block41448:                                       ; preds = %block41442, %edge7182
  %v78184 = select i1 %t4644, i64 0, i64 %v75597
  %v78209 = sub nsw i64 256, %v76500740744753
  %v78214 = lshr i64 %v78209, 29
  %v78225 = and i64 %v78214, 7
  %v78230 = add nsw i64 %v76500740744753, -1
  %v78236 = lshr i64 %v78230, %v78225
  %t4654 = icmp ult i64 %v78236, 256
  br i1 %t4654, label %edge7201, label %edge7208

block41439:                                       ; preds = %edge7138, %block41444
  %v78141 = phi i64 [ 103425, %edge7138 ], [ %v78174, %block41444 ]
  %t4650 = insertvalue { i64, i64 } poison, i64 %v78141, 0
  %t4651 = insertvalue { i64, i64 } %t4650, i64 3575, 1
  br label %common.ret

block41938:                                       ; preds = %edge7248, %edge7245
  br i1 %t4646.not151, label %block41941, label %edge7252

block41941:                                       ; preds = %edge7252, %block41938
  %t4678 = zext i64 %v78159 to i128
  %v219991 = or disjoint i128 %t4678, 5902958103587056517120
  store i128 %v219991, ptr %ss0, align 16
  %t4682 = zext i64 %v78161 to i128
  %v219993 = or disjoint i128 %t4682, 18446744073709551616
  store i128 %v219993, ptr %t1002, align 16
  %t4688 = zext i64 %v78164 to i128
  %t4689 = shl nuw i128 %t4688, 64
  %v219995 = or disjoint i128 %t4689, 320
  store i128 %v219995, ptr %t1007, align 16
  %v79084 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v78161, i64 4, i64 4, i64 %v3194)
  %t4692.not = icmp eq i64 %v79084, 0
  br i1 %t4692.not, label %edge7257, label %block41932

block41935:                                       ; preds = %edge7257, %edge7252
  %t4695.pre-phi = phi ptr [ %.pre21774, %edge7257 ], [ %t4663, %edge7252 ]
  %v79069 = phi i64 [ %v79085, %edge7257 ], [ %v78159, %edge7252 ]
  %v79071 = phi i64 [ %v79086, %edge7257 ], [ %v78161, %edge7252 ]
  %v79074 = phi i64 [ %v79087, %edge7257 ], [ %v78164, %edge7252 ]
  store i32 %43, ptr %t4695.pre-phi, align 1
  %t4696 = icmp eq i64 %v78164, %v79074
  %t4698.not152 = icmp eq i64 %v79074, 0
  %t4698.not = or i1 %t4696, %t4698.not152
  br i1 %t4698.not, label %block41945, label %edge7260

block41945:                                       ; preds = %block41935, %edge7260
  %t4702.not = or i1 %t4646.not151, %t4696
  br i1 %t4702.not, label %edge7325, label %edge7265

block41932:                                       ; preds = %edge7208, %edge7265, %block41941, %edge7248
  %v79047 = phi i64 [ %v79084, %block41941 ], [ 103425, %edge7208 ], [ %v79068, %edge7248 ], [ %v79098, %edge7265 ]
  %t4704 = insertvalue { i64, i64 } poison, i64 %v79047, 0
  %t4705 = insertvalue { i64, i64 } %t4704, i64 3625, 1
  br label %common.ret

block41982:                                       ; preds = %edge7300, %edge7291, %edge7308
  %v79163 = phi i64 [ %v79186, %edge7308 ], [ %v79170, %edge7291 ], [ %v79182, %edge7300 ]
  %t4723 = insertvalue { i64, i64 } poison, i64 %v79163, 0
  %t4724 = insertvalue { i64, i64 } %t4723, i64 3636, 1
  br label %common.ret

block42489:                                       ; preds = %edge7344, %edge7360
  %t4731.not22246 = phi i1 [ false, %edge7360 ], [ true, %edge7344 ]
  %v7983822243 = phi i32 [ %v7983822223, %edge7360 ], [ %v79838, %edge7344 ]
  %v7950122238 = phi i64 [ %v75597, %edge7360 ], [ 0, %edge7344 ]
  %t4736 = zext i64 %v75637 to i128
  %v219997 = or disjoint i128 %t4736, 5902958103587056517120
  store i128 %v219997, ptr %ss0, align 16
  %t4740 = zext i64 %v75629 to i128
  %v219999 = or disjoint i128 %t4740, 18446744073709551616
  store i128 %v219999, ptr %t1002, align 16
  %t4746 = zext i64 %v7950122238 to i128
  %t4747 = shl nuw i128 %t4746, 64
  %v220001 = or disjoint i128 %t4747, 320
  store i128 %v220001, ptr %t1007, align 16
  %v79889 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v75629, i64 4, i64 4, i64 %v3194)
  %t4750.not = icmp eq i64 %v79889, 0
  br i1 %t4750.not, label %block42488, label %block42484

block42488:                                       ; preds = %block42489
  %v79890 = load i64, ptr %ss1, align 8
  %v79891 = load i64, ptr %t606, align 8
  %v79892 = load i64, ptr %t607, align 8
  %.pre21723 = add i64 %v79890, 1024
  %.pre21724 = inttoptr i64 %.pre21723 to ptr
  store i32 %v7983822243, ptr %.pre21724, align 1
  %t4754 = icmp eq i64 %v7950122238, %v79892
  %t4756.not155 = icmp eq i64 %v79892, 0
  %t4756.not = or i1 %t4754, %t4756.not155
  br i1 %t4756.not, label %block42493, label %edge7368

block42493:                                       ; preds = %block42488, %edge7368
  %t4760.not = or i1 %t4731.not22246, %t4754
  br i1 %t4760.not, label %edge6923, label %edge7373

block42484:                                       ; preds = %edge7373, %block42489
  %v79856 = phi i64 [ %v79889, %block42489 ], [ %v79903, %edge7373 ]
  %t4762 = insertvalue { i64, i64 } poison, i64 %v79856, 0
  %t4763 = insertvalue { i64, i64 } %t4762, i64 3672, 1
  br label %common.ret

block3658:                                        ; preds = %block3658.preheader, %edge7439
  %v81058 = phi i64 [ %v81635, %edge7439 ], [ 0, %block3658.preheader ]
  %exitcond21635.not = icmp eq i64 %v81058, 320
  br i1 %exitcond21635.not, label %edge7440, label %edge7439

block43158:                                       ; preds = %edge7443, %edge7440
  %t4790.not = icmp eq i64 %v64795, 0
  br i1 %t4790.not, label %edge11167, label %edge7447

block43154:                                       ; preds = %edge7450, %edge7443
  %v81073 = phi i64 [ %v81081, %edge7443 ], [ %v81085, %edge7450 ]
  %t4795 = insertvalue { i64, i64 } poison, i64 %v81073, 0
  %t4796 = insertvalue { i64, i64 } %t4795, i64 3706, 1
  br label %common.ret

block43821:                                       ; preds = %edge7488, %edge7485
  %t4806.not = icmp eq i64 %v64795, 0
  br i1 %t4806.not, label %block44055, label %edge7494

block43817:                                       ; preds = %edge7494, %edge7488
  %v82002 = phi i64 [ %v82009, %edge7488 ], [ %v82013, %edge7494 ]
  %t4808 = insertvalue { i64, i64 } poison, i64 %v82002, 0
  %t4809 = insertvalue { i64, i64 } %t4808, i64 3751, 1
  br label %common.ret

block44055:                                       ; preds = %edge7494, %block43821, %edge7409, %edge7406, %edge7385, %edge7381
  %t4810.not = icmp eq i64 %v64807, 0
  br i1 %t4810.not, label %block44058, label %edge7516

block44058:                                       ; preds = %edge7516, %block44055
  %t4812.not = icmp eq i64 %v64918, 0
  br i1 %t4812.not, label %block44062, label %edge7522

block44062:                                       ; preds = %edge7522, %block44058
  %t4814.not = icmp eq i64 %v64930, 0
  br i1 %t4814.not, label %block44066, label %edge7528

block44066:                                       ; preds = %edge7528, %block44062
  %t4816.not = icmp eq i64 %v64942, 0
  br i1 %t4816.not, label %edge7561, label %edge7534

block44054:                                       ; preds = %edge7534, %edge7528, %edge7522, %edge7516
  %v82338 = phi i64 [ %v82357, %edge7528 ], [ %v82349, %edge7516 ], [ %v82353, %edge7522 ], [ %v82361, %edge7534 ]
  %t4818 = insertvalue { i64, i64 } poison, i64 %v82338, 0
  %t4819 = insertvalue { i64, i64 } %t4818, i64 3773, 1
  br label %common.ret

block3784:                                        ; preds = %edge10981, %edge7629
  %v218955 = phi i64 [ 0, %edge7629 ], [ %v127608, %edge10981 ]
  %v218956 = phi i64 [ %v80928, %edge7629 ], [ %v1276671059, %edge10981 ]
  %v218957 = phi i64 [ 320, %edge7629 ], [ %v1276681060, %edge10981 ]
  %v218958 = phi i64 [ %v80922, %edge7629 ], [ %v1273071058, %edge10981 ]
  %v218959 = phi i64 [ %v80928, %edge7629 ], [ %v116175976, %edge10981 ]
  %v218960 = phi i64 [ 320, %edge7629 ], [ %v116176978, %edge10981 ]
  %v218961 = phi i64 [ %v80930, %edge7629 ], [ %v116177979, %edge10981 ]
  %v218963 = phi i64 [ 320, %edge7629 ], [ %v116179981, %edge10981 ]
  %v218964 = phi i64 [ %v80922, %edge7629 ], [ %v116180984, %edge10981 ]
  %v218965 = phi i64 [ %v80928, %edge7629 ], [ %v127204, %edge10981 ]
  %v218966 = phi i64 [ 320, %edge7629 ], [ %v127205, %edge10981 ]
  %v218967 = phi i64 [ %v80922, %edge7629 ], [ %v127209, %edge10981 ]
  %v218968 = phi i64 [ %v80928, %edge7629 ], [ %v127619, %edge10981 ]
  %v218969 = phi i64 [ %v80922, %edge7629 ], [ %v127306, %edge10981 ]
  %v218970 = phi i64 [ 0, %edge7629 ], [ %v127630, %edge10981 ]
  %v218971 = phi i64 [ 0, %edge7629 ], [ %v127633, %edge10981 ]
  %v218972 = phi i64 [ 0, %edge7629 ], [ %v127636, %edge10981 ]
  %v218973 = phi i64 [ 0, %edge7629 ], [ %v83567, %edge10981 ]
  %v218975 = phi i64 [ 288, %edge7629 ], [ %spec.select33, %edge10981 ]
  %v218977 = phi i64 [ 14, %edge7629 ], [ %spec.select34, %edge10981 ]
  %v84845 = call i64 @mech_compact_allocate(i64 %v0, i64 %v218975, i64 1, i64 1, i64 1, i64 %v3194)
  %t4854.not = icmp eq i64 %v84845, 0
  br i1 %t4854.not, label %edge7663, label %block44474

block44474:                                       ; preds = %block3784
  %t4860 = insertvalue { i64, i64 } poison, i64 %v84845, 0
  %t4861 = insertvalue { i64, i64 } %t4860, i64 3832, 1
  br label %common.ret

block44529:                                       ; preds = %edge7701
  %t4889 = insertvalue { i64, i64 } poison, i64 %v85724, 0
  %t4890 = insertvalue { i64, i64 } %t4889, i64 3845, 1
  br label %common.ret

block3810:                                        ; preds = %edge7703, %edge7704
  %t4891 = icmp eq i64 %v218973, 1
  %spec.select2 = select i1 %t4891, i64 288, i64 0
  %v86155 = and i64 %v218973, %v65824
  %t4893.not = icmp eq i64 %v86155, 0
  br i1 %t4893.not, label %block3829.preheader, label %edge7719

block3829.preheader:                              ; preds = %block3810
  %t4918 = zext nneg i64 %v218975 to i128
  %t4919 = shl nuw nsw i128 %t4918, 64
  br label %block3829

block3829:                                        ; preds = %block3829.preheader, %edge7853
  %v86753 = phi i64 [ %v88397, %edge7853 ], [ 0, %block3829.preheader ]
  %v86756 = phi i64 [ %v88402792799, %edge7853 ], [ %v85727, %block3829.preheader ]
  %v86758 = phi i64 [ %v88404793798, %edge7853 ], [ %v85728, %block3829.preheader ]
  %v86761 = phi i64 [ %v88131791800, %edge7853 ], [ %v85729, %block3829.preheader ]
  %v86768 = phi i64 [ %v88407801, %edge7853 ], [ 0, %block3829.preheader ]
  %t4899 = icmp samesign ult i64 %v86753, %v218975
  br i1 %t4899, label %edge7755, label %edge7812

block45731:                                       ; preds = %edge7797, %edge7803
  %v87772 = phi i64 [ %v87790, %edge7803 ], [ %v86756, %edge7797 ]
  %v87774 = phi i64 [ %v87791, %edge7803 ], [ %v86758, %edge7797 ]
  %v87777 = phi i64 [ %v87792, %edge7803 ], [ %v86761, %edge7797 ]
  %t4935.not = icmp eq i64 %v87777, 0
  br i1 %t4935.not, label %edge7819.thread, label %edge7806

block45728:                                       ; preds = %edge7801, %edge7795
  %v87760 = phi i64 [ 103425, %edge7795 ], [ %v87789, %edge7801 ]
  %t4938 = insertvalue { i64, i64 } poison, i64 %v87760, 0
  %t4939 = insertvalue { i64, i64 } %t4938, i64 3925, 1
  br label %common.ret

block45971:                                       ; preds = %edge7832, %edge7829
  br i1 %t4940.not, label %edge7873, label %edge7836

block45966:                                       ; preds = %edge7839, %edge7832, %edge7846
  %v88114 = phi i64 [ %v88129, %edge7846 ], [ %v88121, %edge7832 ], [ %v88125, %edge7839 ]
  %t4960 = insertvalue { i64, i64 } poison, i64 %v88114, 0
  %t4961 = insertvalue { i64, i64 } %t4960, i64 3939, 1
  br label %common.ret

block3920:                                        ; preds = %block3920.backedge, %edge7883
  %v88813 = phi i64 [ %v86756, %edge7883 ], [ %v94683, %block3920.backedge ]
  %v88815 = phi i64 [ %v86758, %edge7883 ], [ %v94685, %block3920.backedge ]
  %v88818 = phi i64 [ %v86761, %edge7883 ], [ %v94416, %block3920.backedge ]
  %v88828 = phi i64 [ %v86768, %edge7883 ], [ %spec.select329, %block3920.backedge ]
  %v88831 = phi i64 [ %v88776157, %edge7883 ], [ %v218987, %block3920.backedge ]
  %t4968 = icmp ne i64 %v88831, 0
  %t4969 = icmp ugt i64 %v88828, 1
  %v88854158 = or i1 %t4969, %t4968
  %4 = xor i1 %t4968, true
  %v88864159 = and i1 %t4969, %4
  %v89139 = add i64 %v88828, -1
  %spec.select329 = select i1 %v88864159, i64 %v89139, i64 %v88828
  %v218986 = select i1 %t4968, i64 %v88831, i64 1
  %v218987 = call i64 @llvm.usub.sat.i64(i64 %v88831, i64 1)
  br i1 %v88864159, label %edge7932, label %edge8034

block47798:                                       ; preds = %edge7992, %edge7991
  %t5006 = zext i64 %v88813 to i128
  %v220009 = or disjoint i128 %t4919, %t5006
  store i128 %v220009, ptr %ss0, align 16
  %t5010 = zext i64 %v88815 to i128
  %v220011 = or disjoint i128 %t5010, 18446744073709551616
  store i128 %v220011, ptr %t1002, align 16
  %t5016 = zext i64 %v88818 to i128
  %t5017 = shl nuw i128 %t5016, 64
  %v220013 = or disjoint i128 %t5017, %t4918
  store i128 %v220013, ptr %t1007, align 16
  %v90724 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v88815, i64 4, i64 4, i64 %v3194)
  %t5020.not = icmp eq i64 %v90724, 0
  br i1 %t5020.not, label %edge7997, label %block47793

block47797:                                       ; preds = %edge7992, %edge7997
  %t5023.pre-phi = phi ptr [ %t4997, %edge7992 ], [ %.pre21768, %edge7997 ]
  %v90709 = phi i64 [ %v88813, %edge7992 ], [ %v90725, %edge7997 ]
  %v90711 = phi i64 [ %v88815, %edge7992 ], [ %v90726, %edge7997 ]
  %v90714 = phi i64 [ %v88818, %edge7992 ], [ %v90727, %edge7997 ]
  store i32 %t4994804, ptr %t5023.pre-phi, align 1
  br label %block47796

block47796:                                       ; preds = %edge7988, %block47797
  %v90703 = phi i64 [ %v90709, %block47797 ], [ %v88813, %edge7988 ]
  %v90705 = phi i64 [ %v90711, %block47797 ], [ %v88815, %edge7988 ]
  %v90708 = phi i64 [ %v90714, %block47797 ], [ %v88818, %edge7988 ]
  %t5024 = icmp eq i64 %v88818, %v90708
  %t5026.not160 = icmp eq i64 %v90708, 0
  %t5026.not = or i1 %t5024, %t5026.not160
  br i1 %t5026.not, label %block47802, label %edge8000

block47802:                                       ; preds = %block47796, %edge8000
  %v90734 = select i1 %t5024, i64 0, i64 %v88818
  %t5033 = inttoptr i64 %v90703 to ptr
  %t5034 = load i32, ptr %t5033, align 1
  %t5035 = icmp eq i32 %t4988, %t5034
  br i1 %t5035, label %block47807, label %edge8007

block47793:                                       ; preds = %edge7932, %block47798
  %v90691 = phi i64 [ %v90724, %block47798 ], [ 103425, %edge7932 ]
  %t5030 = insertvalue { i64, i64 } poison, i64 %v90691, 0
  %t5031 = insertvalue { i64, i64 } %t5030, i64 4055, 1
  br label %common.ret

block47810:                                       ; preds = %edge8010, %edge8007
  br i1 %t5026.not160, label %block47813, label %edge8014

block47813:                                       ; preds = %edge8014, %block47810
  %t5044 = zext i64 %v90703 to i128
  %v220015 = or disjoint i128 %t4919, %t5044
  store i128 %v220015, ptr %ss0, align 16
  %t5048 = zext i64 %v90705 to i128
  %v220017 = or disjoint i128 %t5048, 18446744073709551616
  store i128 %v220017, ptr %t1002, align 16
  %t5054 = zext i64 %v90708 to i128
  %t5055 = shl nuw i128 %t5054, 64
  %v220019 = or disjoint i128 %t5055, %t4918
  store i128 %v220019, ptr %t1007, align 16
  %v90778 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v90705, i64 4, i64 4, i64 %v3194)
  %t5058.not = icmp eq i64 %v90778, 0
  br i1 %t5058.not, label %edge8019, label %block47804

block47812:                                       ; preds = %edge8014, %edge8019
  %t5061.pre-phi = phi ptr [ %t5033, %edge8014 ], [ %.pre21766, %edge8019 ]
  %v90763 = phi i64 [ %v90703, %edge8014 ], [ %v90779, %edge8019 ]
  %v90765 = phi i64 [ %v90705, %edge8014 ], [ %v90780, %edge8019 ]
  %v90768 = phi i64 [ %v90708, %edge8014 ], [ %v90781, %edge8019 ]
  store i32 %t4988, ptr %t5061.pre-phi, align 1
  br label %block47807

block47807:                                       ; preds = %block47802, %block47812
  %v90753 = phi i64 [ %v90763, %block47812 ], [ %v90703, %block47802 ]
  %v90755 = phi i64 [ %v90765, %block47812 ], [ %v90705, %block47802 ]
  %v90758 = phi i64 [ %v90768, %block47812 ], [ %v90708, %block47802 ]
  %v93550 = phi i64 [ 0, %block47812 ], [ %v90734, %block47802 ]
  %t5062 = icmp eq i64 %v90708, %v90758
  %t5064.not161 = icmp eq i64 %v90758, 0
  %t5064.not = or i1 %t5062, %t5064.not161
  br i1 %t5064.not, label %block47817, label %edge8022

block47817:                                       ; preds = %block47807, %edge8022
  %t5068.not = or i1 %t5026.not160, %t5062
  br i1 %t5068.not, label %edge7927, label %edge8027

block47804:                                       ; preds = %edge8027, %block47813, %edge8010
  %v90741 = phi i64 [ %v90778, %block47813 ], [ %v90762, %edge8010 ], [ %v90792, %edge8027 ]
  %t5070 = insertvalue { i64, i64 } poison, i64 %v90741, 0
  %t5071 = insertvalue { i64, i64 } %t5070, i64 4057, 1
  br label %common.ret

block4041:                                        ; preds = %block4041.backedge, %edge8083
  %v91460 = phi i64 [ %v90822812, %edge8083 ], [ %v93512, %block4041.backedge ]
  %v91480 = phi i64 [ %v91103814, %edge8083 ], [ %v93509, %block4041.backedge ]
  %v91483 = phi i64 [ %v218986, %edge8083 ], [ %spec.select316, %block4041.backedge ]
  %v91485 = phi i64 [ %v91101813, %edge8083 ], [ %v93507, %block4041.backedge ]
  %v93548 = phi i64 [ %v93549815, %edge8083 ], [ %v94261, %block4041.backedge ]
  %t5090.not = icmp eq i64 %v91460, 0
  br i1 %t5090.not, label %edge8105, label %edge8084

block49709:                                       ; preds = %edge8225, %edge8222
  br i1 %t5090.not, label %block49712, label %edge8229

block49712:                                       ; preds = %edge8229, %block49709
  %t5188 = zext i64 %v91485 to i128
  %v220031 = or disjoint i128 %t4919, %t5188
  store i128 %v220031, ptr %ss0, align 16
  %t5192 = zext i64 %v91480 to i128
  %v220033 = or disjoint i128 %t5192, 18446744073709551616
  store i128 %v220033, ptr %t1002, align 16
  %t5198 = zext i64 %v91460 to i128
  %t5199 = shl nuw i128 %t5198, 64
  %v220035 = or disjoint i128 %t5199, %t4918
  store i128 %v220035, ptr %t1007, align 16
  %v93532 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v91480, i64 4, i64 4, i64 %v3194)
  %t5202.not = icmp eq i64 %v93532, 0
  br i1 %t5202.not, label %edge8234, label %block49703

block49711:                                       ; preds = %edge8229, %edge8234
  %t5205.pre-phi = phi ptr [ %t5177, %edge8229 ], [ %.pre21765, %edge8234 ]
  %v93517 = phi i64 [ %v91485, %edge8229 ], [ %v93533, %edge8234 ]
  %v93519 = phi i64 [ %v91480, %edge8229 ], [ %v93534, %edge8234 ]
  %v93522 = phi i64 [ %v91460, %edge8229 ], [ %v93535, %edge8234 ]
  store i32 %t5167, ptr %t5205.pre-phi, align 1
  br label %block49706

block49706:                                       ; preds = %edge8219, %block49711
  %v93507 = phi i64 [ %v93517, %block49711 ], [ %v91485, %edge8219 ]
  %v93509 = phi i64 [ %v93519, %block49711 ], [ %v91480, %edge8219 ]
  %v93512 = phi i64 [ %v93522, %block49711 ], [ %v91460, %edge8219 ]
  %v94261 = phi i64 [ 0, %block49711 ], [ %v93548, %edge8219 ]
  %t5206 = icmp eq i64 %v91460, %v93512
  %t5208.not164 = icmp eq i64 %v93512, 0
  %t5208.not = or i1 %t5206, %t5208.not164
  br i1 %t5208.not, label %block49716, label %edge8237

block49716:                                       ; preds = %block49706, %edge8237
  %t5212.not = or i1 %t5090.not, %t5206
  br i1 %t5212.not, label %block4041.backedge, label %edge8242

block4041.backedge:                               ; preds = %block49716, %edge8242
  br label %block4041

block49703:                                       ; preds = %edge8242, %block49712, %edge8225, %edge8215
  %v93495 = phi i64 [ %v93532, %block49712 ], [ 103425, %edge8215 ], [ %v93516, %edge8225 ], [ %v93546, %edge8242 ]
  %t5214 = insertvalue { i64, i64 } poison, i64 %v93495, 0
  %t5215 = insertvalue { i64, i64 } %t5214, i64 4196, 1
  br label %common.ret

block4170:                                        ; preds = %edge8201, %edge8170, %edge8106
  %t5216.not = icmp eq i64 %v91483, 0
  br i1 %t5216.not, label %common.ret, label %edge8268

block50195:                                       ; preds = %edge8278, %edge8275
  br i1 %t5090.not, label %block50198, label %edge8282

block50198:                                       ; preds = %edge8282, %block50195
  %t5233 = zext i64 %v91485 to i128
  %v220037 = or disjoint i128 %t4919, %t5233
  store i128 %v220037, ptr %ss0, align 16
  %t5237 = zext i64 %v91480 to i128
  %v220039 = or disjoint i128 %t5237, 18446744073709551616
  store i128 %v220039, ptr %t1002, align 16
  %t5243 = zext i64 %v91460 to i128
  %t5244 = shl nuw i128 %t5243, 64
  %v220041 = or disjoint i128 %t5244, %t4918
  store i128 %v220041, ptr %t1007, align 16
  %v94343 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v91480, i64 4, i64 4, i64 %v3194)
  %t5247.not = icmp eq i64 %v94343, 0
  br i1 %t5247.not, label %edge8287, label %block50189

block50197:                                       ; preds = %edge8282, %edge8287
  %t5250.pre-phi = phi ptr [ %t5222, %edge8282 ], [ %.pre21763, %edge8287 ]
  %v94328 = phi i64 [ %v91485, %edge8282 ], [ %v94344, %edge8287 ]
  %v94330 = phi i64 [ %v91480, %edge8282 ], [ %v94345, %edge8287 ]
  %v94333 = phi i64 [ %v91460, %edge8282 ], [ %v94346, %edge8287 ]
  store i32 %45, ptr %t5250.pre-phi, align 1
  br label %block50192

block50192:                                       ; preds = %edge8272, %block50197
  %v94318 = phi i64 [ %v94328, %block50197 ], [ %v91485, %edge8272 ]
  %v94320 = phi i64 [ %v94330, %block50197 ], [ %v91480, %edge8272 ]
  %v94323 = phi i64 [ %v94333, %block50197 ], [ %v91460, %edge8272 ]
  %v94419 = phi i64 [ 0, %block50197 ], [ %v93548, %edge8272 ]
  %t5251 = icmp eq i64 %v91460, %v94323
  %t5253.not166 = icmp eq i64 %v94323, 0
  %t5253.not = or i1 %t5251, %t5253.not166
  br i1 %t5253.not, label %block50202, label %edge8290

block50202:                                       ; preds = %block50192, %edge8290
  %t5257.not = or i1 %t5090.not, %t5251
  br i1 %t5257.not, label %edge8048, label %edge8293

block50189:                                       ; preds = %edge8296, %block50198, %edge8278, %edge8268
  %v94306 = phi i64 [ %v94343, %block50198 ], [ 103425, %edge8268 ], [ %v94327, %edge8278 ], [ %v94357, %edge8296 ]
  %t5262 = insertvalue { i64, i64 } poison, i64 %v94306, 0
  %t5263 = insertvalue { i64, i64 } %t5262, i64 4225, 1
  br label %common.ret

block50232:                                       ; preds = %edge8319, %edge8327
  %v94402 = phi i64 [ %v94414, %edge8327 ], [ %v94410, %edge8319 ]
  %t5279 = insertvalue { i64, i64 } poison, i64 %v94402, 0
  %t5280 = insertvalue { i64, i64 } %t5279, i64 4233, 1
  br label %common.ret

block51376:                                       ; preds = %edge8405, %edge8404
  %t5307 = zext i64 %v218995 to i128
  %t5308 = zext nneg i64 %v218975 to i128
  %t5309 = shl nuw nsw i128 %t5308, 64
  %v220043 = or disjoint i128 %t5309, %t5307
  store i128 %v220043, ptr %ss0, align 16
  %t5311 = zext i64 %v218997 to i128
  %v220045 = or disjoint i128 %t5311, 18446744073709551616
  store i128 %v220045, ptr %t1002, align 16
  %t5317 = zext i64 %v218999 to i128
  %t5318 = shl nuw i128 %t5317, 64
  %v220047 = or disjoint i128 %t5318, %t5308
  store i128 %v220047, ptr %t1007, align 16
  %v96215 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v218997, i64 4, i64 4, i64 %v3194)
  %t5321.not = icmp eq i64 %v96215, 0
  br i1 %t5321.not, label %edge8410, label %block51371

block51375:                                       ; preds = %edge8405, %edge8410
  %t5324.pre-phi = phi ptr [ %t5298, %edge8405 ], [ %.pre21739, %edge8410 ]
  %v96200 = phi i64 [ %v218995, %edge8405 ], [ %v96216, %edge8410 ]
  %v96202 = phi i64 [ %v218997, %edge8405 ], [ %v96217, %edge8410 ]
  %v96205 = phi i64 [ %v218999, %edge8405 ], [ %v96218, %edge8410 ]
  store i32 65536, ptr %t5324.pre-phi, align 1
  br label %block51374

block51374:                                       ; preds = %edge8401, %block51375
  %v96194 = phi i64 [ %v96200, %block51375 ], [ %v218995, %edge8401 ]
  %v96196 = phi i64 [ %v96202, %block51375 ], [ %v218997, %edge8401 ]
  %v96199 = phi i64 [ %v96205, %block51375 ], [ %v218999, %edge8401 ]
  %t5325 = icmp eq i64 %v218999, %v96199
  %t5327.not196 = icmp eq i64 %v96199, 0
  %t5327.not = or i1 %t5325, %t5327.not196
  br i1 %t5327.not, label %block51380, label %edge8413

block51380:                                       ; preds = %block51374, %edge8413
  %t5331.not197 = icmp eq i64 %v218999, 0
  %t5331.not = or i1 %t5331.not197, %t5325
  br i1 %t5331.not, label %edge8424, label %edge8418

block51371:                                       ; preds = %edge8418, %block51376
  %v96182 = phi i64 [ %v96215, %block51376 ], [ %v96229, %edge8418 ]
  %t5333 = insertvalue { i64, i64 } poison, i64 %v96182, 0
  %t5334 = insertvalue { i64, i64 } %t5333, i64 4296, 1
  br label %common.ret

block51391:                                       ; preds = %edge8428, %edge8427
  %t5346 = zext i64 %v84848 to i128
  %t5347 = zext nneg i64 %v218975 to i128
  %t5348 = shl nuw nsw i128 %t5347, 64
  %v220049 = or disjoint i128 %t5348, %t5346
  store i128 %v220049, ptr %ss0, align 16
  %t5350 = zext i64 %v84849 to i128
  %v220051 = or disjoint i128 %t5350, 18446744073709551616
  store i128 %v220051, ptr %t1002, align 16
  %t5356 = zext i64 %v84850 to i128
  %t5357 = shl nuw i128 %t5356, 64
  %v220053 = or disjoint i128 %t5357, %t5347
  store i128 %v220053, ptr %t1007, align 16
  %v96390 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v84849, i64 1, i64 1, i64 %v3194)
  %t5360.not = icmp eq i64 %v96390, 0
  br i1 %t5360.not, label %block51389, label %block51386

block51389:                                       ; preds = %block51391
  %v96391 = load i64, ptr %ss1, align 8
  %v96392 = load i64, ptr %t606, align 8
  %v96393 = load i64, ptr %t607, align 8
  %t5363 = inttoptr i64 %v96391 to ptr
  store i8 1, ptr %t5363, align 1
  %t5364 = icmp eq i64 %v84850, %v96393
  %t5366.not198 = icmp eq i64 %v96393, 0
  %t5366.not = or i1 %t5364, %t5366.not198
  br i1 %t5366.not, label %block51395, label %edge8436

block51395:                                       ; preds = %block51389, %edge8436
  %t5370.not = or i1 %t4857.not, %t5364
  br i1 %t5370.not, label %edge8445, label %edge8441

block51386:                                       ; preds = %edge8441, %block51391
  %v96357 = phi i64 [ %v96390, %block51391 ], [ %v96404, %edge8441 ]
  %t5372 = insertvalue { i64, i64 } poison, i64 %v96357, 0
  %t5373 = insertvalue { i64, i64 } %t5372, i64 4298, 1
  br label %common.ret

block51406:                                       ; preds = %edge8451, %edge8450
  %t5385 = zext i64 %v96194 to i128
  %t5386 = zext nneg i64 %v218975 to i128
  %t5387 = shl nuw nsw i128 %t5386, 64
  %v220055 = or disjoint i128 %t5387, %t5385
  store i128 %v220055, ptr %ss0, align 16
  %t5389 = zext i64 %v96196 to i128
  %v220057 = or disjoint i128 %t5389, 18446744073709551616
  store i128 %v220057, ptr %t1002, align 16
  %t5395 = zext i64 %v96199 to i128
  %t5396 = shl nuw i128 %t5395, 64
  %v220059 = or disjoint i128 %t5396, %t5386
  store i128 %v220059, ptr %t1007, align 16
  %v96545 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v96196, i64 4, i64 4, i64 %v3194)
  %t5399.not = icmp eq i64 %v96545, 0
  br i1 %t5399.not, label %edge8456, label %block51401

block51405:                                       ; preds = %edge8451, %edge8456
  %t5402.pre-phi = phi ptr [ %t5376, %edge8451 ], [ %.pre21738, %edge8456 ]
  %v96530 = phi i64 [ %v96194, %edge8451 ], [ %v96546, %edge8456 ]
  %v96532 = phi i64 [ %v96196, %edge8451 ], [ %v96547, %edge8456 ]
  %v96535 = phi i64 [ %v96199, %edge8451 ], [ %v96548, %edge8456 ]
  store i32 65537, ptr %t5402.pre-phi, align 1
  br label %block51404

block51404:                                       ; preds = %edge8447, %block51405
  %v96524 = phi i64 [ %v96530, %block51405 ], [ %v96194, %edge8447 ]
  %v96526 = phi i64 [ %v96532, %block51405 ], [ %v96196, %edge8447 ]
  %v96529 = phi i64 [ %v96535, %block51405 ], [ %v96199, %edge8447 ]
  %t5403 = icmp eq i64 %v96199, %v96529
  %t5405.not200 = icmp eq i64 %v96529, 0
  %t5405.not = or i1 %t5403, %t5405.not200
  br i1 %t5405.not, label %block51410, label %edge8459

block51410:                                       ; preds = %block51404, %edge8459
  %t5409.not = or i1 %t5327.not196, %t5403
  br i1 %t5409.not, label %edge8470, label %edge8464

block51401:                                       ; preds = %edge8464, %block51406, %edge8445
  %v96512 = phi i64 [ %v96545, %block51406 ], [ 103425, %edge8445 ], [ %v96559, %edge8464 ]
  %t5411 = insertvalue { i64, i64 } poison, i64 %v96512, 0
  %t5412 = insertvalue { i64, i64 } %t5411, i64 4300, 1
  br label %common.ret

block51421:                                       ; preds = %edge8474, %edge8473
  %t5424 = zext i64 %v96369392415 to i128
  %t5425 = zext nneg i64 %v218975 to i128
  %t5426 = shl nuw nsw i128 %t5425, 64
  %v220061 = or disjoint i128 %t5426, %t5424
  store i128 %v220061, ptr %ss0, align 16
  %t5428 = zext i64 %v96371393414 to i128
  %v220063 = or disjoint i128 %t5428, 18446744073709551616
  store i128 %v220063, ptr %t1002, align 16
  %t5434 = zext i64 %v96374394413 to i128
  %t5435 = shl nuw i128 %t5434, 64
  %v220065 = or disjoint i128 %t5435, %t5425
  store i128 %v220065, ptr %t1007, align 16
  %v96720 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v96371393414, i64 1, i64 1, i64 %v3194)
  %t5438.not = icmp eq i64 %v96720, 0
  br i1 %t5438.not, label %edge8479, label %block51416

block51419:                                       ; preds = %edge8479, %edge8474
  %t5440.pre-phi = phi ptr [ %.pre21736, %edge8479 ], [ %t5415, %edge8474 ]
  %v96710 = phi i64 [ %v96723, %edge8479 ], [ %v96374394413, %edge8474 ]
  store i8 1, ptr %t5440.pre-phi, align 1
  %t5441 = icmp eq i64 %v96374394413, %v96710
  %t5443.not202 = icmp eq i64 %v96710, 0
  %t5443.not = or i1 %t5441, %t5443.not202
  br i1 %t5443.not, label %block51425, label %edge8482

block51425:                                       ; preds = %block51419, %edge8482
  %t5447.not = or i1 %t5366.not198397412, %t5441
  br i1 %t5447.not, label %block51428, label %edge8487

block51428:                                       ; preds = %edge8487, %block51425
  br i1 %t5443.not202, label %edge8362, label %edge8491

block51416:                                       ; preds = %edge8494, %edge8487, %block51421
  %v96687 = phi i64 [ %v96734, %edge8487 ], [ %v96738, %edge8494 ], [ %v96720, %block51421 ]
  %t5454 = insertvalue { i64, i64 } poison, i64 %v96687, 0
  %t5455 = insertvalue { i64, i64 } %t5454, i64 4302, 1
  br label %common.ret

block4256:                                        ; preds = %block4256.backedge, %edge8361
  %v96791 = phi i64 [ %v94792, %edge8361 ], [ %v96791.be, %block4256.backedge ]
  %v96811 = phi i64 [ %v88815, %edge8361 ], [ %v96811.be, %block4256.backedge ]
  %v96812 = phi i64 [ 0, %edge8361 ], [ %v101669, %block4256.backedge ]
  %v96813 = phi i64 [ 0, %edge8361 ], [ %spec.select335, %block4256.backedge ]
  %v96814 = phi i64 [ 0, %edge8361 ], [ %v101604872, %block4256.backedge ]
  %v96816 = phi i64 [ %v88813, %edge8361 ], [ %v96816.be, %block4256.backedge ]
  %t5456.not = icmp eq i64 %v96791, 0
  br i1 %t5456.not, label %edge8531, label %edge8510

block52884:                                       ; preds = %edge8683, %edge8682
  %t5547 = zext i64 %v96816 to i128
  %v220077 = or disjoint i128 %t4919, %t5547
  store i128 %v220077, ptr %ss0, align 16
  %t5551 = zext i64 %v96811 to i128
  %v220079 = or disjoint i128 %t5551, 18446744073709551616
  store i128 %v220079, ptr %t1002, align 16
  %t5557 = zext i64 %v96791 to i128
  %t5558 = shl nuw i128 %t5557, 64
  %v220081 = or disjoint i128 %t5558, %t4918
  store i128 %v220081, ptr %t1007, align 16
  %v99070 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v96811, i64 4, i64 4, i64 %v3194)
  %t5561.not = icmp eq i64 %v99070, 0
  br i1 %t5561.not, label %edge8688, label %block52879

block52883:                                       ; preds = %edge8683, %edge8688
  %t5564.pre-phi = phi ptr [ %t5538, %edge8683 ], [ %.pre21745, %edge8688 ]
  %v99055 = phi i64 [ %v96816, %edge8683 ], [ %v99071, %edge8688 ]
  %v99057 = phi i64 [ %v96811, %edge8683 ], [ %v99072, %edge8688 ]
  %v99060 = phi i64 [ %v96791, %edge8683 ], [ %v99073, %edge8688 ]
  %v99075 = trunc nuw i64 %v99024 to i32
  store i32 %v99075, ptr %t5564.pre-phi, align 1
  br label %block52882

block52882:                                       ; preds = %edge8679, %block52883
  %v99049 = phi i64 [ %v99055, %block52883 ], [ %v96816, %edge8679 ]
  %v99051 = phi i64 [ %v99057, %block52883 ], [ %v96811, %edge8679 ]
  %v99054 = phi i64 [ %v99060, %block52883 ], [ %v96791, %edge8679 ]
  %t5565 = icmp eq i64 %v96791, %v99054
  %t5567.not173 = icmp eq i64 %v99054, 0
  %t5567.not = or i1 %t5565, %t5567.not173
  br i1 %t5567.not, label %edge8668, label %edge8691

block52879:                                       ; preds = %block52884, %edge8672
  %v99037 = phi i64 [ 103425, %edge8672 ], [ %v99070, %block52884 ]
  %t5571 = insertvalue { i64, i64 } poison, i64 %v99037, 0
  %t5572 = insertvalue { i64, i64 } %t5571, i64 4443, 1
  br label %common.ret

block53117:                                       ; preds = %edge8716, %edge8713
  br i1 %t5567.not173, label %block53120, label %edge8720

block53120:                                       ; preds = %edge8720, %block53117
  %t5588 = zext i64 %v99049 to i128
  %v220083 = or disjoint i128 %t4919, %t5588
  store i128 %v220083, ptr %ss0, align 16
  %t5592 = zext i64 %v99051 to i128
  %v220085 = or disjoint i128 %t5592, 18446744073709551616
  store i128 %v220085, ptr %t1002, align 16
  %t5598 = zext i64 %v99054 to i128
  %t5599 = shl nuw i128 %t5598, 64
  %v220087 = or disjoint i128 %t5599, %t4918
  store i128 %v220087, ptr %t1007, align 16
  %v99436 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v99051, i64 4, i64 4, i64 %v3194)
  %t5602.not = icmp eq i64 %v99436, 0
  br i1 %t5602.not, label %edge8725, label %block53111

block53114:                                       ; preds = %edge8725, %edge8720
  %t5605.pre-phi = phi ptr [ %.pre21743, %edge8725 ], [ %t5577, %edge8720 ]
  %v99421 = phi i64 [ %v99437, %edge8725 ], [ %v99049, %edge8720 ]
  %v99423 = phi i64 [ %v99438, %edge8725 ], [ %v99051, %edge8720 ]
  %v99426 = phi i64 [ %v99439, %edge8725 ], [ %v99054, %edge8720 ]
  %v99441 = trunc nuw i64 %v99380 to i32
  store i32 %v99441, ptr %t5605.pre-phi, align 1
  %t5606 = icmp eq i64 %v99054, %v99426
  %t5608.not175 = icmp eq i64 %v99426, 0
  %t5608.not = or i1 %t5606, %t5608.not175
  br i1 %t5608.not, label %block53124, label %edge8728

block53124:                                       ; preds = %block53114, %edge8728
  %t5612.not = or i1 %t5567.not173, %t5606
  br i1 %t5612.not, label %edge8745, label %edge8733

block53111:                                       ; preds = %edge8733, %block53120, %edge8716, %edge8701
  %v99393 = phi i64 [ %v99436, %block53120 ], [ 103425, %edge8701 ], [ %v99420, %edge8716 ], [ %v99450, %edge8733 ]
  %t5614 = insertvalue { i64, i64 } poison, i64 %v99393, 0
  %t5615 = insertvalue { i64, i64 } %t5614, i64 4455, 1
  br label %common.ret

block54681:                                       ; preds = %edge8840, %edge8837
  %t5645.not = icmp eq i64 %v101236866, 0
  br i1 %t5645.not, label %block54684, label %edge8844

block54684:                                       ; preds = %edge8844, %block54681
  %t5650 = zext i64 %v101231864 to i128
  %v220089 = or disjoint i128 %t4919, %t5650
  store i128 %v220089, ptr %ss0, align 16
  %t5654 = zext i64 %v101233865 to i128
  %v220091 = or disjoint i128 %t5654, 18446744073709551616
  store i128 %v220091, ptr %t1002, align 16
  %t5660 = zext i64 %v101236866 to i128
  %t5661 = shl nuw i128 %t5660, 64
  %v220093 = or disjoint i128 %t5661, %t4918
  store i128 %v220093, ptr %t1007, align 16
  %v101643 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v101233865, i64 4, i64 4, i64 %v3194)
  %t5664.not = icmp eq i64 %v101643, 0
  br i1 %t5664.not, label %edge8849, label %block54675

block54678:                                       ; preds = %edge8849, %edge8844
  %t5667.pre-phi = phi ptr [ %.pre21741, %edge8849 ], [ %t5635, %edge8844 ]
  %v101628 = phi i64 [ %v101644, %edge8849 ], [ %v101231864, %edge8844 ]
  %v101630 = phi i64 [ %v101645, %edge8849 ], [ %v101233865, %edge8844 ]
  %v101633 = phi i64 [ %v101646, %edge8849 ], [ %v101236866, %edge8844 ]
  %v101648 = trunc nuw i64 %v101593871 to i32
  store i32 %v101648, ptr %t5667.pre-phi, align 1
  %t5668 = icmp eq i64 %v101236866, %v101633
  %t5670.not177 = icmp eq i64 %v101633, 0
  %t5670.not = or i1 %t5668, %t5670.not177
  br i1 %t5670.not, label %block54688, label %edge8852

block54688:                                       ; preds = %block54678, %edge8852
  %t5674.not = or i1 %t5645.not, %t5668
  br i1 %t5674.not, label %block4256.backedge, label %edge8857

block4256.backedge:                               ; preds = %block54688, %edge8857, %edge8880, %edge8890, %edge8893
  %v96791.be = phi i64 [ %v101236866, %edge8893 ], [ %v101236866, %edge8880 ], [ %v101236866, %edge8890 ], [ %v101633, %edge8857 ], [ %v101633, %block54688 ]
  %v96811.be = phi i64 [ %v101233865, %edge8893 ], [ %v101233865, %edge8880 ], [ %v101233865, %edge8890 ], [ %v101630, %edge8857 ], [ %v101630, %block54688 ]
  %v96816.be = phi i64 [ %v101231864, %edge8893 ], [ %v101231864, %edge8880 ], [ %v101231864, %edge8890 ], [ %v101628, %edge8857 ], [ %v101628, %block54688 ]
  br label %block4256

block54675:                                       ; preds = %edge8745, %edge8857, %block54684, %edge8840
  %v101606 = phi i64 [ %v101643, %block54684 ], [ %v101657, %edge8857 ], [ %v101627, %edge8840 ], [ 103425, %edge8745 ]
  %t5676 = insertvalue { i64, i64 } poison, i64 %v101606, 0
  %t5677 = insertvalue { i64, i64 } %t5676, i64 4541, 1
  br label %common.ret

block54723:                                       ; preds = %edge8885, %edge8893
  %v101722 = phi i64 [ %v101733, %edge8893 ], [ %v101729, %edge8885 ]
  %t5694 = insertvalue { i64, i64 } poison, i64 %v101722, 0
  %t5695 = insertvalue { i64, i64 } %t5694, i64 4553, 1
  br label %common.ret

block54988:                                       ; preds = %edge8912
  %t5727 = insertvalue { i64, i64 } poison, i64 %v102850, 0
  %t5728 = insertvalue { i64, i64 } %t5727, i64 4571, 1
  br label %common.ret

block55030:                                       ; preds = %edge8958, %edge8962
  %t5739 = zext i64 %v102853 to i128
  %t5740 = zext nneg i64 %v102058 to i128
  %t5741 = shl nuw nsw i128 %t5740, 64
  %v220095 = or disjoint i128 %t5741, %t5739
  store i128 %v220095, ptr %ss0, align 16
  %t5743 = zext i64 %v102854 to i128
  %v220097 = or disjoint i128 %t5743, 18446744073709551616
  store i128 %v220097, ptr %t1002, align 16
  %t5749 = zext i64 %v102855 to i128
  %t5750 = shl nuw i128 %t5749, 64
  %v220099 = or disjoint i128 %t5750, %t5740
  store i128 %v220099, ptr %t1007, align 16
  %v103007 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v102854, i64 8, i64 8, i64 %v3194)
  %t5753.not = icmp eq i64 %v103007, 0
  br i1 %t5753.not, label %block55028, label %block55025

block55028:                                       ; preds = %block55030
  %v103008 = load i64, ptr %ss1, align 8
  %v103009 = load i64, ptr %t606, align 8
  %v103010 = load i64, ptr %t607, align 8
  %v103011 = add i64 %v103008, 8
  %t5756 = inttoptr i64 %v103011 to ptr
  store i64 2, ptr %t5756, align 1
  %t5757 = icmp eq i64 %v102855, %v103010
  %t5759.not179 = icmp eq i64 %v103010, 0
  %t5759.not = or i1 %t5757, %t5759.not179
  br i1 %t5759.not, label %block55034, label %edge8970

block55034:                                       ; preds = %block55028, %edge8970
  %t5763.not = or i1 %t5724.not, %t5757
  br i1 %t5763.not, label %edge8988, label %edge8973

block55025:                                       ; preds = %edge8976, %block55030
  %v102974 = phi i64 [ %v103007, %block55030 ], [ %v103020, %edge8976 ]
  %t5768 = insertvalue { i64, i64 } poison, i64 %v102974, 0
  %t5769 = insertvalue { i64, i64 } %t5768, i64 4580, 1
  br label %common.ret

block55288:                                       ; preds = %edge9015, %edge9014
  %t5786 = zext i64 %v96816 to i128
  %v220101 = or disjoint i128 %t4919, %t5786
  store i128 %v220101, ptr %ss0, align 16
  %t5790 = zext i64 %v96811 to i128
  %v220103 = or disjoint i128 %t5790, 18446744073709551616
  store i128 %v220103, ptr %t1002, align 16
  %t5796 = zext i64 %v96791 to i128
  %t5797 = shl nuw i128 %t5796, 64
  %v220105 = or disjoint i128 %t5797, %t4918
  store i128 %v220105, ptr %t1007, align 16
  %v103443 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v96811, i64 4, i64 4, i64 %v3194)
  %t5800.not = icmp eq i64 %v103443, 0
  br i1 %t5800.not, label %edge9020, label %block55283

block55287:                                       ; preds = %edge9015, %edge9020
  %t5803.pre-phi = phi ptr [ %t5773, %edge9015 ], [ %.pre21761, %edge9020 ]
  %v103428 = phi i64 [ %v96816, %edge9015 ], [ %v103444, %edge9020 ]
  %v103430 = phi i64 [ %v96811, %edge9015 ], [ %v103445, %edge9020 ]
  %v103433 = phi i64 [ %v96791, %edge9015 ], [ %v103446, %edge9020 ]
  store i32 %55, ptr %t5803.pre-phi, align 1
  br label %block55286

block55286:                                       ; preds = %edge8996, %block55287
  %v103422 = phi i64 [ %v103428, %block55287 ], [ %v96816, %edge8996 ]
  %v103424 = phi i64 [ %v103430, %block55287 ], [ %v96811, %edge8996 ]
  %v103427 = phi i64 [ %v103433, %block55287 ], [ %v96791, %edge8996 ]
  %t5804 = icmp eq i64 %v96791, %v103427
  %t5806.not181 = icmp eq i64 %v103427, 0
  %t5806.not = or i1 %t5804, %t5806.not181
  br i1 %t5806.not, label %block55292, label %edge9023

block55292:                                       ; preds = %block55286, %edge9023
  %t5810.not182 = icmp eq i64 %v96791, 0
  %t5810.not = or i1 %t5810.not182, %t5804
  br i1 %t5810.not, label %edge9039, label %edge9026

block55283:                                       ; preds = %edge8988, %edge9029, %block55288
  %v103410 = phi i64 [ %v103443, %block55288 ], [ 103425, %edge8988 ], [ %v103457, %edge9029 ]
  %t5815 = insertvalue { i64, i64 } poison, i64 %v103410, 0
  %t5816 = insertvalue { i64, i64 } %t5815, i64 4606, 1
  br label %common.ret

block4548:                                        ; preds = %block4548.backedge, %edge9039
  %v103508 = phi i64 [ %v103422, %edge9039 ], [ %v104598, %block4548.backedge ]
  %v103510 = phi i64 [ %v103424, %edge9039 ], [ %v104600, %block4548.backedge ]
  %v103513 = phi i64 [ %v103427, %edge9039 ], [ %v104603, %block4548.backedge ]
  %v103520 = phi i64 [ %v102986439459, %edge9039 ], [ %v106487, %block4548.backedge ]
  %v103522 = phi i64 [ %v102988440458, %edge9039 ], [ %v106489, %block4548.backedge ]
  %v103525 = phi i64 [ %v102991441457, %edge9039 ], [ %v106492, %block4548.backedge ]
  %v103532 = phi i64 [ %v102044, %edge9039 ], [ %v103826, %block4548.backedge ]
  %t5817.not = icmp eq i64 %v103532, 0
  br i1 %t5817.not, label %edge9317, label %edge9054

block56050:                                       ; preds = %edge9129, %edge9128
  %t5848 = zext i64 %v103508 to i128
  %v220107 = or disjoint i128 %t4919, %t5848
  store i128 %v220107, ptr %ss0, align 16
  %t5852 = zext i64 %v103510 to i128
  %v220109 = or disjoint i128 %t5852, 18446744073709551616
  store i128 %v220109, ptr %t1002, align 16
  %t5858 = zext i64 %v103513 to i128
  %t5859 = shl nuw i128 %t5858, 64
  %v220111 = or disjoint i128 %t5859, %t4918
  store i128 %v220111, ptr %t1007, align 16
  %v104619 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v103510, i64 4, i64 4, i64 %v3194)
  %t5862.not = icmp eq i64 %v104619, 0
  br i1 %t5862.not, label %edge9134, label %block56045

block56049:                                       ; preds = %edge9129, %edge9134
  %t5865.pre-phi = phi ptr [ %t5839, %edge9129 ], [ %.pre21759, %edge9134 ]
  %v104604 = phi i64 [ %v103508, %edge9129 ], [ %v104620, %edge9134 ]
  %v104606 = phi i64 [ %v103510, %edge9129 ], [ %v104621, %edge9134 ]
  %v104609 = phi i64 [ %v103513, %edge9129 ], [ %v104622, %edge9134 ]
  %v104624 = trunc nuw i64 %v104573 to i32
  store i32 %v104624, ptr %t5865.pre-phi, align 1
  br label %block56048

block56048:                                       ; preds = %edge9113, %block56049
  %v104598 = phi i64 [ %v104604, %block56049 ], [ %v103508, %edge9113 ]
  %v104600 = phi i64 [ %v104606, %block56049 ], [ %v103510, %edge9113 ]
  %v104603 = phi i64 [ %v104609, %block56049 ], [ %v103513, %edge9113 ]
  %t5866 = icmp eq i64 %v103513, %v104603
  %t5868.not184 = icmp eq i64 %v104603, 0
  %t5868.not = or i1 %t5866, %t5868.not184
  br i1 %t5868.not, label %block56054, label %edge9137

block56054:                                       ; preds = %block56048, %edge9137
  %v104629 = select i1 %t5866, i64 0, i64 %v103513
  %t5874.not = icmp samesign ult i64 %v104518, %v218977
  br i1 %t5874.not, label %edge9219, label %edge9146

block56045:                                       ; preds = %block56050
  %t5872 = insertvalue { i64, i64 } poison, i64 %v104619, 0
  %t5873 = insertvalue { i64, i64 } %t5872, i64 4679, 1
  br label %common.ret

block4623:                                        ; preds = %edge9162, %edge9152
  %v104959 = phi i64 [ %v218977, %edge9152 ], [ %v104971, %edge9162 ]
  %t5880.not = icmp eq i64 %v104959, 0
  br i1 %t5880.not, label %common.ret, label %edge9162

block57036:                                       ; preds = %edge9238, %edge9231
  %t5911 = zext i64 %v103520 to i128
  %v220113 = or disjoint i128 %t5913, %t5911
  store i128 %v220113, ptr %ss0, align 16
  %t5915 = zext i64 %v103522 to i128
  %v220115 = or disjoint i128 %t5915, 18446744073709551616
  store i128 %v220115, ptr %t1002, align 16
  %t5921 = zext i64 %v103525 to i128
  %t5922 = shl nuw i128 %t5921, 64
  %v220117 = or disjoint i128 %t5922, %t5912
  store i128 %v220117, ptr %t1007, align 16
  %v106076 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v103522, i64 8, i64 8, i64 %v3194)
  %t5925.not = icmp eq i64 %v106076, 0
  br i1 %t5925.not, label %edge9243, label %edge9242

block57035:                                       ; preds = %edge9238, %edge9243
  %t5928.pre-phi = phi ptr [ %t5898, %edge9238 ], [ %.pre21755, %edge9243 ]
  %v106061 = phi i64 [ %v103520, %edge9238 ], [ %v106077, %edge9243 ]
  %v106063 = phi i64 [ %v103522, %edge9238 ], [ %v106078, %edge9243 ]
  %v106066 = phi i64 [ %v103525, %edge9238 ], [ %v106079, %edge9243 ]
  store i64 %v106036, ptr %t5928.pre-phi, align 1
  %t5929 = icmp eq i64 %v103525, %v106066
  %t5931.not185 = icmp eq i64 %v106066, 0
  %t5931.not = or i1 %t5929, %t5931.not185
  br i1 %t5931.not, label %block57040, label %edge9246

block57040:                                       ; preds = %block57035, %edge9246
  %v106085 = select i1 %t5929, i64 0, i64 %v103525
  %t5937 = icmp ult i64 %v106000, %v102058
  br i1 %t5937, label %edge9275, label %block57294

block57300:                                       ; preds = %edge9288, %edge9285
  br i1 %t5931.not185, label %block57303, label %edge9292

block57303:                                       ; preds = %edge9292, %block57300
  %t5957 = zext i64 %v106061 to i128
  %v220119 = or disjoint i128 %t5913, %t5957
  store i128 %v220119, ptr %ss0, align 16
  %t5961 = zext i64 %v106063 to i128
  %v220121 = or disjoint i128 %t5961, 18446744073709551616
  store i128 %v220121, ptr %t1002, align 16
  %t5967 = zext i64 %v106066 to i128
  %t5968 = shl nuw i128 %t5967, 64
  %v220123 = or disjoint i128 %t5968, %t5912
  store i128 %v220123, ptr %t1007, align 16
  %v106502 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v106063, i64 8, i64 8, i64 %v3194)
  %t5971.not = icmp eq i64 %v106502, 0
  br i1 %t5971.not, label %edge9297, label %block57294

block57297:                                       ; preds = %edge9297, %edge9292
  %t5974.pre-phi = phi ptr [ %t5940, %edge9292 ], [ %.pre21757, %edge9297 ]
  %v106487 = phi i64 [ %v106061, %edge9292 ], [ %v106503, %edge9297 ]
  %v106489 = phi i64 [ %v106063, %edge9292 ], [ %v106504, %edge9297 ]
  %v106492 = phi i64 [ %v106066, %edge9292 ], [ %v106505, %edge9297 ]
  store i64 %v106449, ptr %t5974.pre-phi, align 1
  %t5975 = icmp eq i64 %v106066, %v106492
  %t5977.not186 = icmp eq i64 %v106492, 0
  %t5977.not = or i1 %t5975, %t5977.not186
  br i1 %t5977.not, label %block57307, label %edge9300

block57307:                                       ; preds = %block57297, %edge9300
  %t5981.not = or i1 %t5931.not185, %t5975
  br i1 %t5981.not, label %edge9333, label %edge9303

block57294:                                       ; preds = %block57040, %edge9306, %block57303, %edge9288
  %v106465 = phi i64 [ %v106502, %block57303 ], [ %v106515, %edge9306 ], [ %v106486, %edge9288 ], [ 103425, %block57040 ]
  %t5986 = insertvalue { i64, i64 } poison, i64 %v106465, 0
  %t5987 = insertvalue { i64, i64 } %t5986, i64 4781, 1
  br label %common.ret

block57334:                                       ; preds = %edge9317, %edge9318
  %t5991.not = icmp eq i64 %v103525, 0
  br i1 %t5991.not, label %edge9334, label %edge9321

block57362:                                       ; preds = %edge9337, %edge9334
  %t5998.not = icmp eq i64 %v103525, 0
  br i1 %t5998.not, label %edge9395, label %edge9341

block57357:                                       ; preds = %edge9344, %edge9337, %edge9351
  %v106603 = phi i64 [ %v106622, %edge9351 ], [ %v106618, %edge9344 ], [ %v106614, %edge9337 ]
  %t6010 = insertvalue { i64, i64 } poison, i64 %v106603, 0
  %t6011 = insertvalue { i64, i64 } %t6010, i64 4791, 1
  br label %common.ret

block57671:                                       ; preds = %edge9395, %edge9396
  br i1 %t5998.not, label %edge9407, label %edge9399

block4742:                                        ; preds = %edge9637, %edge9407
  %v107108 = phi i64 [ %v84850, %edge9407 ], [ %v107915, %edge9637 ]
  %v107149 = phi i64 [ %v84849, %edge9407 ], [ %v107912, %edge9637 ]
  %v107150 = phi i64 [ %v218977, %edge9407 ], [ %v109110, %edge9637 ]
  %v107151 = phi i64 [ 0, %edge9407 ], [ %v107925, %edge9637 ]
  %v107156 = phi i64 [ %v84848, %edge9407 ], [ %v107910, %edge9637 ]
  %t6018.not = icmp eq i64 %v107108, 0
  br i1 %t6018.not, label %edge9449, label %edge9408

block57691:                                       ; preds = %edge9429, %edge9422
  %v107107 = phi i64 [ %v107136, %edge9422 ], [ %v107144, %edge9429 ]
  %t6079 = insertvalue { i64, i64 } poison, i64 %v107107, 0
  %t6080 = insertvalue { i64, i64 } %t6079, i64 4810, 1
  br label %common.ret

block4778:                                        ; preds = %edge9576, %edge9486
  %v107910 = phi i64 [ %v107156, %edge9486 ], [ %v108656, %edge9576 ]
  %v107912 = phi i64 [ %v107149, %edge9486 ], [ %v108658, %edge9576 ]
  %v107915 = phi i64 [ %v107108, %edge9486 ], [ %v108661, %edge9576 ]
  %v107925 = phi i64 [ %v107151, %edge9486 ], [ %v108701, %edge9576 ]
  %v107928 = phi i64 [ %v107855, %edge9486 ], [ %v108710, %edge9576 ]
  %t6091.not = icmp eq i64 %v107928, 0
  br i1 %t6091.not, label %edge9503, label %edge9522

block58746:                                       ; preds = %edge9555, %edge9548
  %t6119 = zext i64 %v107910 to i128
  %v220141 = or disjoint i128 %t4919, %t6119
  store i128 %v220141, ptr %ss0, align 16
  %t6123 = zext i64 %v107912 to i128
  %v220143 = or disjoint i128 %t6123, 18446744073709551616
  store i128 %v220143, ptr %t1002, align 16
  %t6129 = zext i64 %v107915 to i128
  %t6130 = shl nuw i128 %t6129, 64
  %v220145 = or disjoint i128 %t6130, %t4918
  store i128 %v220145, ptr %t1007, align 16
  %v108681 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v107912, i64 1, i64 1, i64 %v3194)
  %t6133.not = icmp eq i64 %v108681, 0
  br i1 %t6133.not, label %edge9560, label %block58737

block58745:                                       ; preds = %edge9555, %edge9560
  %t6136.pre-phi = phi ptr [ %t6108, %edge9555 ], [ %.pre21753, %edge9560 ]
  %v108666 = phi i64 [ %v107910, %edge9555 ], [ %v108682, %edge9560 ]
  %v108668 = phi i64 [ %v107912, %edge9555 ], [ %v108683, %edge9560 ]
  %v108671 = phi i64 [ %v107915, %edge9555 ], [ %v108684, %edge9560 ]
  store i8 %58, ptr %t6136.pre-phi, align 1
  br label %block58740

block58740:                                       ; preds = %edge9545, %block58745
  %v108656 = phi i64 [ %v108666, %block58745 ], [ %v107910, %edge9545 ]
  %v108658 = phi i64 [ %v108668, %block58745 ], [ %v107912, %edge9545 ]
  %v108661 = phi i64 [ %v108671, %block58745 ], [ %v107915, %edge9545 ]
  %t6137 = icmp eq i64 %v107915, %v108661
  %t6139.not188 = icmp eq i64 %v108661, 0
  %t6139.not = or i1 %t6137, %t6139.not188
  br i1 %t6139.not, label %block58750, label %edge9563

block58750:                                       ; preds = %block58740, %edge9563
  %t6143.not189 = icmp eq i64 %v107915, 0
  %t6143.not = or i1 %t6143.not189, %t6137
  br i1 %t6143.not, label %edge9572, label %edge9568

block58737:                                       ; preds = %edge9568, %block58746, %edge9539
  %v108644 = phi i64 [ %v108681, %block58746 ], [ 103425, %edge9539 ], [ %v108695, %edge9568 ]
  %t6145 = insertvalue { i64, i64 } poison, i64 %v108644, 0
  %t6146 = insertvalue { i64, i64 } %t6145, i64 4889, 1
  br label %common.ret

block58825:                                       ; preds = %edge9609, %edge9606
  br i1 %t5998.not, label %block58829, label %edge9615

block58829:                                       ; preds = %edge9615, %block58825
  %t6160.not = icmp eq i64 %v107108, 0
  br i1 %t6160.not, label %edge9689, label %edge9619

block58820:                                       ; preds = %edge9622, %edge9615, %edge9609
  %v108806 = phi i64 [ %v108824, %edge9622 ], [ %v108816, %edge9609 ], [ %v108820, %edge9615 ]
  %t6167 = insertvalue { i64, i64 } poison, i64 %v108806, 0
  %t6168 = insertvalue { i64, i64 } %t6167, i64 4907, 1
  br label %common.ret

block59111:                                       ; preds = %edge9689
  %t6205 = insertvalue { i64, i64 } poison, i64 %v110188, 0
  %t6206 = insertvalue { i64, i64 } %t6205, i64 4924, 1
  br label %common.ret

block4862:                                        ; preds = %block4862.backedge, %edge9703
  %v110380 = phi i64 [ %v110193, %edge9703 ], [ %v111535, %block4862.backedge ]
  %v110404 = phi i64 [ %v110192, %edge9703 ], [ %v111532, %block4862.backedge ]
  %v110406 = phi i64 [ 2, %edge9703 ], [ %v111516, %block4862.backedge ]
  %v110411 = phi i64 [ %v110191, %edge9703 ], [ %v111530, %block4862.backedge ]
  %t6207.not = icmp eq i64 %v110380, 0
  br i1 %t6207.not, label %edge9733, label %edge9704

block59927:                                       ; preds = %edge9820, %edge9819
  %t6290 = zext i64 %v110411 to i128
  %v220159 = or disjoint i128 %t6290, 313594649253062377472
  store i128 %v220159, ptr %ss0, align 16
  %t6294 = zext i64 %v110404 to i128
  %v220161 = or disjoint i128 %t6294, 18446744073709551616
  store i128 %v220161, ptr %t1002, align 16
  %t6300 = zext i64 %v110380 to i128
  %t6301 = shl nuw i128 %t6300, 64
  %v220163 = or disjoint i128 %t6301, 17
  store i128 %v220163, ptr %t1007, align 16
  %v111551 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v110404, i64 4, i64 4, i64 %v3194)
  %t6304.not = icmp eq i64 %v111551, 0
  br i1 %t6304.not, label %edge9825, label %block59922

block59926:                                       ; preds = %edge9820, %edge9825
  %t6307.pre-phi = phi ptr [ %t6281, %edge9820 ], [ %.pre21751, %edge9825 ]
  %v111536 = phi i64 [ %v110411, %edge9820 ], [ %v111552, %edge9825 ]
  %v111538 = phi i64 [ %v110404, %edge9820 ], [ %v111553, %edge9825 ]
  %v111541 = phi i64 [ %v110380, %edge9820 ], [ %v111554, %edge9825 ]
  %v111556 = trunc i64 %v111500 to i32
  store i32 %v111556, ptr %t6307.pre-phi, align 1
  br label %block59925

block59925:                                       ; preds = %edge9800, %block59926
  %v111530 = phi i64 [ %v111536, %block59926 ], [ %v110411, %edge9800 ]
  %v111532 = phi i64 [ %v111538, %block59926 ], [ %v110404, %edge9800 ]
  %v111535 = phi i64 [ %v111541, %block59926 ], [ %v110380, %edge9800 ]
  %t6308 = icmp eq i64 %v110380, %v111535
  %t6310.not190 = icmp eq i64 %v111535, 0
  %t6310.not = or i1 %t6308, %t6310.not190
  br i1 %t6310.not, label %block59931, label %edge9828

block59931:                                       ; preds = %block59925, %edge9828
  %t6314.not = or i1 %t6207.not, %t6308
  br i1 %t6314.not, label %block4862.backedge, label %edge9833

block59922:                                       ; preds = %edge9833, %block59927
  %v111518 = phi i64 [ %v111551, %block59927 ], [ %v111565, %edge9833 ]
  %t6316 = insertvalue { i64, i64 } poison, i64 %v111518, 0
  %t6317 = insertvalue { i64, i64 } %t6316, i64 5003, 1
  br label %common.ret

block4939:                                        ; preds = %block4939.preheader, %edge10135
  %v111836 = phi i64 [ %v114260, %edge10135 ], [ %v103513, %block4939.preheader ]
  %v111870 = phi i64 [ %v219024, %edge10135 ], [ %v110380, %block4939.preheader ]
  %v111872 = phi i64 [ %v114257, %edge10135 ], [ %v103510, %block4939.preheader ]
  %v111874 = phi i64 [ %v219023, %edge10135 ], [ %v110404, %block4939.preheader ]
  %v111876 = phi i64 [ %v114645, %edge10135 ], [ 0, %block4939.preheader ]
  %v111877 = phi i64 [ %v114255, %edge10135 ], [ %v103508, %block4939.preheader ]
  %v111884 = phi i64 [ %v219022, %edge10135 ], [ %v110411, %block4939.preheader ]
  %t6322.not = icmp eq i64 %v111836, 0
  br i1 %t6322.not, label %edge9910.thread, label %edge9864

block60005:                                       ; preds = %edge9886, %edge9881
  %v111835 = phi i64 [ %v111858, %edge9881 ], [ %v111862, %edge9886 ]
  %t6397 = insertvalue { i64, i64 } poison, i64 %v111835, 0
  %t6398 = insertvalue { i64, i64 } %t6397, i64 5014, 1
  br label %common.ret

block61006:                                       ; preds = %edge9980, %edge9979
  %t6430 = zext i64 %v111884 to i128
  %v220187 = or disjoint i128 %t6430, 313594649253062377472
  store i128 %v220187, ptr %ss0, align 16
  %t6434 = zext i64 %v111874 to i128
  %v220189 = or disjoint i128 %t6434, 18446744073709551616
  store i128 %v220189, ptr %t1002, align 16
  %t6440 = zext i64 %v111870 to i128
  %t6441 = shl nuw i128 %t6440, 64
  %v220191 = or disjoint i128 %t6441, 17
  store i128 %v220191, ptr %t1007, align 16
  %v113340 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v111874, i64 4, i64 4, i64 %v3194)
  %t6444.not = icmp eq i64 %v113340, 0
  br i1 %t6444.not, label %edge9985, label %block61001

block61004:                                       ; preds = %edge9985, %edge9980
  %t6447.pre-phi = phi ptr [ %t6413, %edge9980 ], [ %.pre21749, %edge9985 ]
  %v113325 = phi i64 [ %v111884, %edge9980 ], [ %v113341, %edge9985 ]
  %v113327 = phi i64 [ %v111874, %edge9980 ], [ %v113342, %edge9985 ]
  %v113330 = phi i64 [ %v111870, %edge9980 ], [ %v113343, %edge9985 ]
  store i32 %v113298, ptr %t6447.pre-phi, align 1
  %t6448 = icmp eq i64 %v111870, %v113330
  %t6450.not192 = icmp eq i64 %v113330, 0
  %t6450.not = or i1 %t6448, %t6450.not192
  br i1 %t6450.not, label %block61010, label %edge9988

block61010:                                       ; preds = %block61004, %edge9988
  %t6454.not193 = icmp eq i64 %v111870, 0
  %t6454.not = or i1 %t6454.not193, %t6448
  br i1 %t6454.not, label %edge9998, label %edge9993

block61001:                                       ; preds = %edge9955, %edge9993, %block61006
  %v113307 = phi i64 [ %v113340, %block61006 ], [ %v113354, %edge9993 ], [ 103425, %edge9955 ]
  %t6456 = insertvalue { i64, i64 } poison, i64 %v113307, 0
  %t6457 = insertvalue { i64, i64 } %t6456, i64 5083, 1
  br label %common.ret

block61540:                                       ; preds = %edge10072, %edge10071
  %t6485 = zext i64 %v111877 to i128
  %v220193 = or disjoint i128 %t4919, %t6485
  store i128 %v220193, ptr %ss0, align 16
  %t6489 = zext i64 %v111872 to i128
  %v220195 = or disjoint i128 %t6489, 18446744073709551616
  store i128 %v220195, ptr %t1002, align 16
  %t6495 = zext i64 %v111836 to i128
  %t6496 = shl nuw i128 %t6495, 64
  %v220197 = or disjoint i128 %t6496, %t4918
  store i128 %v220197, ptr %t1007, align 16
  %v114276 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v111872, i64 4, i64 4, i64 %v3194)
  %t6499.not = icmp eq i64 %v114276, 0
  br i1 %t6499.not, label %edge10077, label %block61535

block61539:                                       ; preds = %edge10072, %edge10077
  %t6502.pre-phi = phi ptr [ %t6476, %edge10072 ], [ %.pre21747, %edge10077 ]
  %v114261 = phi i64 [ %v111877, %edge10072 ], [ %v114277, %edge10077 ]
  %v114263 = phi i64 [ %v111872, %edge10072 ], [ %v114278, %edge10077 ]
  %v114266 = phi i64 [ %v111836, %edge10072 ], [ %v114279, %edge10077 ]
  %v114281 = trunc nuw nsw i64 %v219025 to i32
  store i32 %v114281, ptr %t6502.pre-phi, align 1
  br label %block61538

block61538:                                       ; preds = %edge10068, %block61539
  %v114255 = phi i64 [ %v114261, %block61539 ], [ %v111877, %edge10068 ]
  %v114257 = phi i64 [ %v114263, %block61539 ], [ %v111872, %edge10068 ]
  %v114260 = phi i64 [ %v114266, %block61539 ], [ %v111836, %edge10068 ]
  %t6503 = icmp eq i64 %v111836, %v114260
  %t6505.not194 = icmp eq i64 %v114260, 0
  %t6505.not = or i1 %t6503, %t6505.not194
  br i1 %t6505.not, label %edge10103, label %edge10080

block61535:                                       ; preds = %block61540
  %t6509 = insertvalue { i64, i64 } poison, i64 %v114276, 0
  %t6510 = insertvalue { i64, i64 } %t6509, i64 5145, 1
  br label %common.ret

block61604:                                       ; preds = %edge10114, %edge10111
  %t6520.not = icmp eq i64 %v111870, 0
  br i1 %t6520.not, label %edge8362, label %edge10118

block61591:                                       ; preds = %edge10121, %edge10114, %edge10107, %edge10128
  %v114359 = phi i64 [ %v114382, %edge10128 ], [ %v114374, %edge10114 ], [ %v114366, %edge10107 ], [ %v114378, %edge10121 ]
  %t6527 = insertvalue { i64, i64 } poison, i64 %v114359, 0
  %t6528 = insertvalue { i64, i64 } %t6527, i64 5154, 1
  br label %common.ret

block62328:                                       ; preds = %edge10195, %edge10199, %edge10208
  %v115490 = phi i64 [ %v115495, %edge10199 ], [ %v115512, %edge10208 ], [ 16, %edge10195 ]
  %t6570 = call i64 @llvm.umax.i64(i64 %v115490, i64 %v115484)
  br label %edge10212

block62341:                                       ; preds = %edge10214, %edge10219
  br i1 %t6538.not, label %edge10245, label %edge10224

block62320:                                       ; preds = %edge10224, %edge10212, %edge10199, %edge10189
  %v115354 = phi i64 [ %v115525, %edge10212 ], [ 1, %edge10189 ], [ 1, %edge10199 ], [ %v115540, %edge10224 ]
  %t6594 = insertvalue { i64, i64 } poison, i64 %v115354, 0
  %t6595 = insertvalue { i64, i64 } %t6594, i64 5185, 1
  br label %common.ret

block5118:                                        ; preds = %block5118.backedge, %edge10251
  %v116269 = phi i64 [ 288, %edge10251 ], [ %v116564, %block5118.backedge ]
  %t6604 = icmp samesign ult i64 %v116269, 258
  br i1 %t6604, label %block63058, label %edge10263

block63058:                                       ; preds = %edge10282, %block5118
  %v116269.lcssa = phi i64 [ %v116269, %edge10282 ], [ 257, %block5118 ]
  br i1 %t6601.not, label %block5173.preheader, label %edge10298

block5173.preheader:                              ; preds = %block63058, %edge10317
  br label %block5173

block5173:                                        ; preds = %block5173.backedge, %block5173.preheader
  %v117463 = phi i64 [ 320, %block5173.preheader ], [ %v117757, %block5173.backedge ]
  %t6626 = icmp samesign ult i64 %v117463, 290
  br i1 %t6626, label %block63767, label %edge10334

block63767:                                       ; preds = %edge10353, %block5173
  %v117463.lcssa = phi i64 [ %v117463, %edge10353 ], [ 289, %block5173 ]
  br i1 %t6601.not, label %edge10388, label %edge10369

block64016:                                       ; preds = %edge10425
  %t6680 = insertvalue { i64, i64 } poison, i64 %v119360, 0
  %t6681 = insertvalue { i64, i64 } %t6680, i64 5317, 1
  br label %common.ret

block64071:                                       ; preds = %edge10465
  %t6709 = insertvalue { i64, i64 } poison, i64 %v120374, 0
  %t6710 = insertvalue { i64, i64 } %t6709, i64 5330, 1
  br label %common.ret

block5254:                                        ; preds = %edge10467, %edge10468
  br i1 %t6601.not, label %edge10483, label %edge10474

block5256:                                        ; preds = %block5256.backedge, %edge10483
  %v120626 = phi i64 [ %v119363, %edge10483 ], [ %v125902, %block5256.backedge ]
  %v120628 = phi i64 [ %v119364, %edge10483 ], [ %v125904, %block5256.backedge ]
  %v120631 = phi i64 [ %v119365, %edge10483 ], [ %v125907, %block5256.backedge ]
  %v120638 = phi i64 [ %v120377, %edge10483 ], [ %v126063, %block5256.backedge ]
  %v120640 = phi i64 [ %v120378, %edge10483 ], [ %v126065, %block5256.backedge ]
  %v120643 = phi i64 [ %v120379, %edge10483 ], [ %v126068, %block5256.backedge ]
  %v120650 = phi i64 [ 0, %edge10483 ], [ %v126037, %block5256.backedge ]
  %v120653 = phi i64 [ 0, %edge10483 ], [ %v126134, %block5256.backedge ]
  %v120656 = phi i64 [ 0, %edge10483 ], [ %v219030, %block5256.backedge ]
  %v120659 = phi i64 [ 0, %edge10483 ], [ %v219032, %block5256.backedge ]
  %t6714.not = icmp eq i64 %v120653, %v118454
  br i1 %t6714.not, label %edge10810, label %edge10509

block5325:                                        ; preds = %block5325.preheader, %edge10635
  %v122309.in = phi i64 [ %v122309, %edge10635 ], [ %v120653, %block5325.preheader ]
  %v122309 = add nuw i64 %v122309.in, 1
  %t6736 = icmp ne i64 %v122309, %v118454
  %t6737 = icmp ult i64 %v122309, %v116269.lcssa
  %v122635 = select i1 %t6737, i64 0, i64 %v121553
  %v122640 = add i64 %v122635, %v122309
  %t6740 = icmp ult i64 %v122640, %v116179982
  br i1 %t6740, label %edge10607, label %edge10616

block5388:                                        ; preds = %edge10624, %edge10625, %edge10628, %edge10509
  %v219030 = phi i64 [ %v120656, %edge10509 ], [ %v122309, %edge10628 ], [ %v122309, %edge10625 ], [ %v122309, %edge10624 ]
  %v219032 = phi i64 [ %v120659, %edge10509 ], [ %v121916, %edge10628 ], [ %v121916, %edge10625 ], [ %v121916, %edge10624 ]
  %t6760.not = icmp ult i64 %v219030, %v120653
  br i1 %t6760.not, label %common.ret, label %edge10648

block67441:                                       ; preds = %edge10746, %edge10745
  %t6803 = zext i64 %v120626 to i128
  %v220205 = or disjoint i128 %t6803, 350488137400481480704
  store i128 %v220205, ptr %ss0, align 16
  %t6807 = zext i64 %v120628 to i128
  %v220207 = or disjoint i128 %t6807, 18446744073709551616
  store i128 %v220207, ptr %t1002, align 16
  %t6813 = zext i64 %v120631 to i128
  %t6814 = shl nuw i128 %t6813, 64
  %v220209 = or disjoint i128 %t6814, 19
  store i128 %v220209, ptr %t1007, align 16
  %v125917 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v120628, i64 4, i64 4, i64 %v3194)
  %t6817.not = icmp eq i64 %v125917, 0
  br i1 %t6817.not, label %edge10751, label %block67436

block67439:                                       ; preds = %edge10751, %edge10746
  %t6820.pre-phi = phi ptr [ %t6786, %edge10746 ], [ %.pre21734, %edge10751 ]
  %v125902 = phi i64 [ %v120626, %edge10746 ], [ %v125918, %edge10751 ]
  %v125904 = phi i64 [ %v120628, %edge10746 ], [ %v125919, %edge10751 ]
  %v125907 = phi i64 [ %v120631, %edge10746 ], [ %v125920, %edge10751 ]
  store i32 %v125922, ptr %t6820.pre-phi, align 1
  %t6821 = icmp eq i64 %v120631, %v125907
  %t6823.not209 = icmp eq i64 %v125907, 0
  %t6823.not = or i1 %t6821, %t6823.not209
  br i1 %t6823.not, label %block67445, label %edge10754

block67445:                                       ; preds = %block67439, %edge10754
  %v125927 = select i1 %t6821, i64 0, i64 %v120631
  %v126014 = shl nuw nsw i64 %v1254901015, 5
  %v126024 = or disjoint i64 %v12517510021012, %v126014
  %v126037 = add nuw nsw i64 %v120650, 1
  %exitcond21638.not = icmp eq i64 %v120650, 320
  br i1 %exitcond21638.not, label %block67456, label %edge10768

block67436:                                       ; preds = %edge10707, %block67441
  %v125884 = phi i64 [ %v125917, %block67441 ], [ 103425, %edge10707 ]
  %t6827 = insertvalue { i64, i64 } poison, i64 %v125884, 0
  %t6828 = insertvalue { i64, i64 } %t6827, i64 5560, 1
  br label %common.ret

block67462:                                       ; preds = %edge10774, %edge10771
  %t6838.not = icmp eq i64 %v120643, 0
  br i1 %t6838.not, label %block67465, label %edge10778

block67465:                                       ; preds = %edge10778, %block67462
  %t6843 = zext i64 %v120638 to i128
  %v220211 = or disjoint i128 %t6843, 5902958103587056517120
  store i128 %v220211, ptr %ss0, align 16
  %t6847 = zext i64 %v120640 to i128
  %v220213 = or disjoint i128 %t6847, 18446744073709551616
  store i128 %v220213, ptr %t1002, align 16
  %t6853 = zext i64 %v120643 to i128
  %t6854 = shl nuw i128 %t6853, 64
  %v220215 = or disjoint i128 %t6854, 320
  store i128 %v220215, ptr %t1007, align 16
  %v126088 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v120640, i64 4, i64 4, i64 %v3194)
  %t6857.not = icmp eq i64 %v126088, 0
  br i1 %t6857.not, label %edge10783, label %block67456

block67464:                                       ; preds = %edge10778, %edge10783
  %t6860.pre-phi = phi ptr [ %t6832, %edge10778 ], [ %.pre21732, %edge10783 ]
  %v126073 = phi i64 [ %v120638, %edge10778 ], [ %v126089, %edge10783 ]
  %v126075 = phi i64 [ %v120640, %edge10778 ], [ %v126090, %edge10783 ]
  %v126078 = phi i64 [ %v120643, %edge10778 ], [ %v126091, %edge10783 ]
  %v126093 = trunc nuw nsw i64 %v126024 to i32
  store i32 %v126093, ptr %t6860.pre-phi, align 1
  br label %block67459

block67459:                                       ; preds = %edge10768, %block67464
  %v126063 = phi i64 [ %v126073, %block67464 ], [ %v120638, %edge10768 ]
  %v126065 = phi i64 [ %v126075, %block67464 ], [ %v120640, %edge10768 ]
  %v126068 = phi i64 [ %v126078, %block67464 ], [ %v120643, %edge10768 ]
  %v126252 = phi i64 [ 0, %block67464 ], [ %v125927, %edge10768 ]
  %t6861 = icmp eq i64 %v120643, %v126068
  %t6863.not210 = icmp eq i64 %v126068, 0
  %t6863.not = or i1 %t6861, %t6863.not210
  br i1 %t6863.not, label %edge10800, label %edge10786

block67456:                                       ; preds = %block67465, %edge10774, %block67445
  %v126039 = phi i64 [ %v126072, %edge10774 ], [ 103425, %block67445 ], [ %v126088, %block67465 ]
  %t6867 = insertvalue { i64, i64 } poison, i64 %v126039, 0
  %t6868 = insertvalue { i64, i64 } %t6867, i64 5569, 1
  br label %common.ret

block67516:                                       ; preds = %edge10810, %edge10811
  %t6880.not = icmp eq i64 %v120643, 0
  br i1 %t6880.not, label %edge10830, label %edge10814

block67547:                                       ; preds = %edge10833, %edge10830
  br i1 %t6877.not, label %block67551, label %edge10839

block67551:                                       ; preds = %edge10839, %block67547
  %t6889.not = icmp eq i64 %v120643, 0
  br i1 %t6889.not, label %edge10243, label %edge10843

block67559:                                       ; preds = %edge10853, %edge10829
  %t6896.not = icmp eq i64 %v126098, 0
  br i1 %t6896.not, label %block5256.backedge, label %edge10857

block5256.backedge:                               ; preds = %block67559, %edge10857, %edge10860
  br label %block5256

block67542:                                       ; preds = %edge10846, %edge10839, %edge10833, %edge10860, %edge10853
  %v126219 = phi i64 [ %v126242, %edge10853 ], [ %v126246, %edge10860 ], [ %v126234, %edge10839 ], [ %v126230, %edge10833 ], [ %v126238, %edge10846 ]
  %t6901 = insertvalue { i64, i64 } poison, i64 %v126219, 0
  %t6902 = insertvalue { i64, i64 } %t6901, i64 5588, 1
  br label %common.ret

block67874:                                       ; preds = %edge10900, %edge10901
  %t6906.not = icmp eq i64 %v218969, 0
  br i1 %t6906.not, label %edge10243, label %edge10904

block68113:                                       ; preds = %edge10930, %edge10927
  br i1 %t6914.not, label %block68117, label %edge10936

block68117:                                       ; preds = %edge10936, %block68113
  %t6924.not = icmp eq i64 %v127306, 0
  br i1 %t6924.not, label %block68121, label %edge10942

block68121:                                       ; preds = %edge10942, %block68117
  br i1 %t6914.not, label %edge11012, label %edge10946

block68129:                                       ; preds = %edge10956, %edge10925
  %t6933.not = icmp eq i64 %v115052, 0
  br i1 %t6933.not, label %block68133, label %edge10962

block68133:                                       ; preds = %edge10962, %block68129
  %t6935.not = icmp eq i64 %v218967, 0
  br i1 %t6935.not, label %block68137, label %edge10968

block68137:                                       ; preds = %edge10968, %block68133
  %t6937.not = icmp eq i64 %v218969, 0
  br i1 %t6937.not, label %edge10981, label %edge10974

block68104:                                       ; preds = %edge10949, %edge10942, %edge10936, %edge10930, %edge10974, %edge10968, %edge10962, %edge10956
  %v127248 = phi i64 [ %v127283, %edge10968 ], [ %v127287, %edge10974 ], [ %v127279, %edge10962 ], [ %v127275, %edge10956 ], [ %v127267, %edge10942 ], [ %v127263, %edge10936 ], [ %v127259, %edge10930 ], [ %v127271, %edge10949 ]
  %t6939 = insertvalue { i64, i64 } poison, i64 %v127248, 0
  %t6940 = insertvalue { i64, i64 } %t6939, i64 5618, 1
  br label %common.ret

block5547:                                        ; preds = %block5547.backedge, %block5547.preheader
  %v130227 = phi i64 [ 19, %block5547.preheader ], [ %v130557, %block5547.backedge ]
  %t6945 = icmp samesign ult i64 %v130227, 5
  br i1 %t6945, label %block69347, label %edge11074

block69347:                                       ; preds = %edge11082, %block5547
  %v130227.lcssa = phi i64 [ %v130227, %edge11082 ], [ 4, %block5547 ]
  br i1 %t6942.not, label %edge11167, label %edge11098

block5677:                                        ; preds = %block5677.preheader, %edge11482
  %v133678 = phi i64 [ %v135928, %edge11482 ], [ 0, %block5677.preheader ]
  %v133681 = phi i64 [ %v135647, %edge11482 ], [ %v133322, %block5677.preheader ]
  %exitcond21639.not = icmp eq i64 %v133678, 19
  br i1 %exitcond21639.not, label %edge11489, label %edge11407

block71789:                                       ; preds = %edge11492, %edge11489
  br i1 %t7053.not, label %block5790.preheader, label %edge11496

block71785:                                       ; preds = %edge11499, %edge11492
  %v135651 = phi i64 [ %v135658, %edge11492 ], [ %v135662, %edge11499 ]
  %t7098 = insertvalue { i64, i64 } poison, i64 %v135651, 0
  %t7099 = insertvalue { i64, i64 } %t7098, i64 5872, 1
  br label %common.ret

block5790:                                        ; preds = %block5790.preheader, %edge11605
  %v136821 = phi i64 [ %v138998, %edge11605 ], [ 0, %block5790.preheader ]
  %v136824 = phi i64 [ %v138727, %edge11605 ], [ %v133681, %block5790.preheader ]
  %exitcond21640.not = icmp eq i64 %v136821, 320
  br i1 %exitcond21640.not, label %edge11612, label %edge11547

block73434:                                       ; preds = %edge11615, %edge11612
  br i1 %t7032.not, label %edge11188, label %edge11619

block73430:                                       ; preds = %edge11622, %edge11615
  %v138731 = phi i64 [ %v138738, %edge11615 ], [ %v138742, %edge11622 ]
  %t7140 = insertvalue { i64, i64 } poison, i64 %v138731, 0
  %t7141 = insertvalue { i64, i64 } %t7140, i64 5989, 1
  br label %common.ret

block5938:                                        ; preds = %block5938.preheader, %edge12032
  %v141572 = phi i64 [ %v143879, %edge12032 ], [ %v66494, %block5938.preheader ]
  %v141600 = phi i64 [ %v143876, %edge12032 ], [ %v67293, %block5938.preheader ]
  %v141601 = phi i64 [ %v144204, %edge12032 ], [ 3, %block5938.preheader ]
  %v141602 = phi i64 [ %v143874, %edge12032 ], [ %v67292, %block5938.preheader ]
  %t7222.not = icmp eq i64 %v141572, 0
  br i1 %t7222.not, label %edge11899, label %edge11868

block75847:                                       ; preds = %edge11987, %edge11986
  %t7320 = zext i64 %v141602 to i128
  %v220231 = or disjoint i128 %t7320, 10090369008319124733952
  store i128 %v220231, ptr %ss0, align 16
  %t7324 = zext i64 %v141600 to i128
  %v220233 = or disjoint i128 %t7324, 18446744073709551616
  store i128 %v220233, ptr %t1002, align 16
  %t7330 = zext i64 %v141572 to i128
  %t7331 = shl nuw i128 %t7330, 64
  %v220235 = or disjoint i128 %t7331, 547
  store i128 %v220235, ptr %t1007, align 16
  %v143895 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v141600, i64 4, i64 4, i64 %v3194)
  %t7334.not = icmp eq i64 %v143895, 0
  br i1 %t7334.not, label %edge11992, label %block75842

block75846:                                       ; preds = %edge11987, %edge11992
  %t7337.pre-phi = phi ptr [ %t7311, %edge11987 ], [ %.pre21726, %edge11992 ]
  %v143880 = phi i64 [ %v141602, %edge11987 ], [ %v143896, %edge11992 ]
  %v143882 = phi i64 [ %v141600, %edge11987 ], [ %v143897, %edge11992 ]
  %v143885 = phi i64 [ %v141572, %edge11987 ], [ %v143898, %edge11992 ]
  %v143900 = trunc nuw nsw i64 %v143838 to i32
  store i32 %v143900, ptr %t7337.pre-phi, align 1
  br label %block75845

block75845:                                       ; preds = %85, %block75846
  %v143874 = phi i64 [ %v143880, %block75846 ], [ %v141602, %85 ]
  %v143876 = phi i64 [ %v143882, %block75846 ], [ %v141600, %85 ]
  %v143879 = phi i64 [ %v143885, %block75846 ], [ %v141572, %85 ]
  %t7338 = icmp eq i64 %v141572, %v143879
  %t7340.not212 = icmp eq i64 %v143879, 0
  %t7340.not = or i1 %t7338, %t7340.not212
  br i1 %t7340.not, label %edge12006, label %edge11995

block75842:                                       ; preds = %block75847
  %t7344 = insertvalue { i64, i64 } poison, i64 %v143895, 0
  %t7345 = insertvalue { i64, i64 } %t7344, i64 6119, 1
  br label %common.ret

block75862:                                       ; preds = %edge12018, %edge12010, %edge12025
  %v143926 = phi i64 [ %v143945, %edge12025 ], [ %v143933, %edge12010 ], [ %v143941, %edge12018 ]
  %t7360 = insertvalue { i64, i64 } poison, i64 %v143926, 0
  %t7361 = insertvalue { i64, i64 } %t7360, i64 6124, 1
  br label %common.ret

block6057:                                        ; preds = %block6057.preheader, %edge12137
  %v145075 = phi i64 [ %v146281, %edge12137 ], [ 0, %block6057.preheader ]
  %exitcond21641.not = icmp eq i64 %v145075, 547
  br i1 %exitcond21641.not, label %edge12104, label %edge12137

block76581:                                       ; preds = %edge12107, %edge12104
  br i1 %t7382.not, label %block6130, label %edge12111

block76577:                                       ; preds = %edge12114, %edge12107
  %v145090 = phi i64 [ %v145098, %edge12107 ], [ %v145102, %edge12114 ]
  %t7395 = insertvalue { i64, i64 } poison, i64 %v145090, 0
  %t7396 = insertvalue { i64, i64 } %t7395, i64 6170, 1
  br label %common.ret

block77244:                                       ; preds = %edge12175, %edge12172
  br i1 %t7382.not, label %block6130, label %edge12179

block77240:                                       ; preds = %edge12182, %edge12175
  %v146018 = phi i64 [ %v146025, %edge12175 ], [ %v146029, %edge12182 ]
  %t7417 = insertvalue { i64, i64 } poison, i64 %v146018, 0
  %t7418 = insertvalue { i64, i64 } %t7417, i64 6215, 1
  br label %common.ret

block6130:                                        ; preds = %edge11677, %edge12073, %edge12070, %edge12069, %edge12182, %edge12179, %block77244, %block76581, %edge12111, %edge12114
  %v219046 = phi i64 [ %v144584, %edge12073 ], [ 1, %edge12182 ], [ 1, %edge12114 ], [ 1, %edge12111 ], [ 1, %block76581 ], [ 1, %block77244 ], [ 1, %edge12179 ], [ %v144584, %edge12069 ], [ %v144584, %edge12070 ], [ 0, %edge11677 ]
  %v219047 = phi i64 [ 1, %edge12073 ], [ 1, %edge12182 ], [ 0, %edge12114 ], [ 0, %edge12111 ], [ 0, %block76581 ], [ 1, %block77244 ], [ 1, %edge12179 ], [ 1, %edge12069 ], [ 1, %edge12070 ], [ 1, %edge11677 ]
  %v219060 = phi i64 [ %v141572, %edge12073 ], [ %v141572, %edge12182 ], [ %v141572, %edge12114 ], [ %v141572, %edge12111 ], [ 0, %block76581 ], [ 0, %block77244 ], [ %v141572, %edge12179 ], [ %v141572, %edge12069 ], [ %v141572, %edge12070 ], [ %v66494, %edge11677 ]
  %v219067 = phi i64 [ %v141602, %edge12073 ], [ %v141602, %edge12182 ], [ %v141602, %edge12114 ], [ %v141602, %edge12111 ], [ %v141602, %block76581 ], [ %v141602, %block77244 ], [ %v141602, %edge12179 ], [ %v141602, %edge12069 ], [ %v141602, %edge12070 ], [ %v67292, %edge11677 ]
  %v219069 = phi i64 [ %v141600, %edge12073 ], [ %v141600, %edge12182 ], [ %v141600, %edge12114 ], [ %v141600, %edge12111 ], [ %v141600, %block76581 ], [ %v141600, %block77244 ], [ %v141600, %edge12179 ], [ %v141600, %edge12069 ], [ %v141600, %edge12070 ], [ %v67293, %edge11677 ]
  %v219097 = phi i64 [ %v144576, %edge12073 ], [ %v144576, %edge12182 ], [ %v144576, %edge12114 ], [ %v144576, %edge12111 ], [ %v144576, %block76581 ], [ %v144576, %block77244 ], [ %v144576, %edge12179 ], [ %v144576, %edge12069 ], [ %v144576, %edge12070 ], [ %v64787, %edge11677 ]
  %v146359 = and i64 %v219047, %v219046
  %v146378 = xor i64 %v146359, -1
  %v146383 = and i64 %v139146, %v146378
  %t7420 = icmp ult i64 %v64847, %v64850
  %v146449 = zext i1 %t7420 to i64
  %spec.select3 = call i64 @llvm.umin.i64(i64 %v64847, i64 %v64850)
  %t7423 = icmp ult i64 %spec.select3, %spec.select361079
  %v146730 = select i1 %t7423, i64 %v146383, i64 0
  %v146741 = xor i64 %v146449, 1
  %v146746 = and i64 %v146730, %v146741
  %t7425.not = icmp eq i64 %v146730, %v146383
  br i1 %t7425.not, label %edge12236.thread, label %edge12238

edge12236.thread:                                 ; preds = %block6130
  %v1471111094 = and i64 %v146383, %v146449
  br label %edge12256

block78101:                                       ; preds = %edge12281, %edge12274
  %v147574 = phi i64 [ %v147581, %edge12274 ], [ %v147585, %edge12281 ]
  %t7447 = insertvalue { i64, i64 } poison, i64 %v147574, 0
  %t7448 = insertvalue { i64, i64 } %t7447, i64 6289, 1
  br label %common.ret

block6192:                                        ; preds = %block6192.preheader, %edge12511
  %v147883 = phi i64 [ %v149090, %edge12511 ], [ 0, %block6192.preheader ]
  %v147886 = phi i64 [ %v219105.ph, %edge12511 ], [ 0, %block6192.preheader ]
  %exitcond21642.not = icmp eq i64 %v147883, 256
  br i1 %exitcond21642.not, label %edge12500, label %edge12471

block6359:                                        ; preds = %edge12521, %edge12551
  %v219106 = phi i64 [ %v150959, %edge12551 ], [ 3, %edge12521 ]
  br i1 %t7555.not, label %block6361.preheader, label %edge12573

block6361.preheader:                              ; preds = %block6359, %edge12573
  br label %block6361

block6361:                                        ; preds = %block6361.preheader, %edge12637
  %v153853 = phi i64 [ %v154471, %edge12637 ], [ %v219106, %block6361.preheader ]
  %v153856 = phi i64 [ %v154515, %edge12637 ], [ 0, %block6361.preheader ]
  %v153859 = phi i64 [ %v154838, %edge12637 ], [ %v55001, %block6361.preheader ]
  %exitcond21643.not = icmp eq i64 %v153853, 259
  br i1 %exitcond21643.not, label %edge12592, label %edge12591

block83919:                                       ; preds = %edge12773, %edge12774
  br i1 %t7638.not, label %block6532.preheader, label %edge12777

block6532.preheader:                              ; preds = %edge12777, %block83919
  br label %block6532

block6532:                                        ; preds = %block6532.preheader, %edge12862
  %v159617 = phi i64 [ %v161382, %edge12862 ], [ 0, %block6532.preheader ]
  %v159620 = phi i64 [ %v161390, %edge12862 ], [ 0, %block6532.preheader ]
  %exitcond21644.not = icmp eq i64 %v159617, 10
  br i1 %exitcond21644.not, label %edge12845, label %edge12800

block85013:                                       ; preds = %edge12848, %edge12845
  br i1 %t7638.not, label %edge12877, label %edge12852

block85009:                                       ; preds = %edge12855, %edge12848
  %v161119 = phi i64 [ %v161126, %edge12848 ], [ %v161130, %edge12855 ]
  %t7685 = insertvalue { i64, i64 } poison, i64 %v161119, 0
  %t7686 = insertvalue { i64, i64 } %t7685, i64 6713, 1
  br label %common.ret

block6664:                                        ; preds = %edge13113, %edge12942
  %v163116 = phi i64 [ %v64765, %edge12942 ], [ %v165297, %edge13113 ]
  %v163140 = phi i64 [ %v64762, %edge12942 ], [ %v165294, %edge13113 ]
  %v163144 = phi i64 [ 3, %edge12942 ], [ %v165622, %edge13113 ]
  %v163145 = phi i64 [ %v64760, %edge12942 ], [ %v165292, %edge13113 ]
  %v165329 = phi i64 [ %v219060, %edge12942 ], [ %v165652, %edge13113 ]
  %t7752.not = icmp eq i64 %v163116, 0
  br i1 %t7752.not, label %edge12968, label %edge12943

block87369:                                       ; preds = %edge13070, %edge13067
  br i1 %t7752.not, label %block87372, label %edge13074

block87372:                                       ; preds = %edge13074, %block87369
  %t7872 = zext i64 %v163145 to i128
  %v220251 = or disjoint i128 %t7872, 10090369008319124733952
  store i128 %v220251, ptr %ss0, align 16
  %t7876 = zext i64 %v163140 to i128
  %v220253 = or disjoint i128 %t7876, 18446744073709551616
  store i128 %v220253, ptr %t1002, align 16
  %t7882 = zext i64 %v163116 to i128
  %t7883 = shl nuw i128 %t7882, 64
  %v220255 = or disjoint i128 %t7883, 547
  store i128 %v220255, ptr %t1007, align 16
  %v165317 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v163140, i64 4, i64 4, i64 %v3194)
  %t7886.not = icmp eq i64 %v165317, 0
  br i1 %t7886.not, label %edge13079, label %block87363

block87371:                                       ; preds = %edge13074, %edge13079
  %t7889.pre-phi = phi ptr [ %t7817, %edge13074 ], [ %.pre21730, %edge13079 ]
  %v165302 = phi i64 [ %v163145, %edge13074 ], [ %v165318, %edge13079 ]
  %v165304 = phi i64 [ %v163140, %edge13074 ], [ %v165319, %edge13079 ]
  %v165307 = phi i64 [ %v163116, %edge13074 ], [ %v165320, %edge13079 ]
  %v165322 = trunc nuw i64 %v164997221 to i32
  store i32 %v165322, ptr %t7889.pre-phi, align 1
  br label %block87366

block87366:                                       ; preds = %edge13022, %block87371
  %v165292 = phi i64 [ %v165302, %block87371 ], [ %v163145, %edge13022 ]
  %v165294 = phi i64 [ %v165304, %block87371 ], [ %v163140, %edge13022 ]
  %v165297 = phi i64 [ %v165307, %block87371 ], [ %v163116, %edge13022 ]
  %v165652 = phi i64 [ 0, %block87371 ], [ %v165329, %edge13022 ]
  %t7890 = icmp eq i64 %v163116, %v165297
  %t7892.not222 = icmp eq i64 %v165297, 0
  %t7892.not = or i1 %t7890, %t7892.not222
  br i1 %t7892.not, label %edge13093, label %edge13082

block87363:                                       ; preds = %block87372, %edge13070
  %v165280 = phi i64 [ %v165301, %edge13070 ], [ %v165317, %block87372 ]
  %t7896 = insertvalue { i64, i64 } poison, i64 %v165280, 0
  %t7897 = insertvalue { i64, i64 } %t7896, i64 6862, 1
  br label %common.ret

block87387:                                       ; preds = %edge13099, %edge13106
  %v165353 = phi i64 [ %v165368, %edge13106 ], [ %v165364, %edge13099 ]
  %t7910 = insertvalue { i64, i64 } poison, i64 %v165353, 0
  %t7911 = insertvalue { i64, i64 } %t7910, i64 6867, 1
  br label %common.ret

block6792:                                        ; preds = %edge13147, %edge13138, %edge12271, %edge12274, %edge13130
  %v219119 = phi i64 [ %v165329, %edge13130 ], [ %v219060, %edge12271 ], [ %v219060, %edge12274 ], [ %v165329, %edge13138 ], [ %v165329, %edge13147 ]
  %v219120 = phi i64 [ %v16311615468, %edge13130 ], [ %v147587, %edge12271 ], [ %v147587, %edge12274 ], [ %v16311615468, %edge13138 ], [ %v16311615468, %edge13147 ]
  %v219139 = phi i64 [ %v163145, %edge13130 ], [ %v165697, %edge12271 ], [ %v165697, %edge12274 ], [ %v163145, %edge13138 ], [ %v163145, %edge13147 ]
  %v219141 = phi i64 [ %v163140, %edge13130 ], [ %v165699, %edge12271 ], [ %v165699, %edge12274 ], [ %v163140, %edge13138 ], [ %v163140, %edge13147 ]
  %v219151 = phi i64 [ %v64850, %edge13130 ], [ %v64850, %edge12271 ], [ %v64850, %edge12274 ], [ 7, %edge13138 ], [ %100, %edge13147 ]
  br i1 %v147503216, label %block6816, label %edge13163

block6816:                                        ; preds = %edge13168, %edge13163, %block6792
  %v219164 = phi i64 [ 2, %block6792 ], [ %spec.select40, %edge13168 ], [ 3, %edge13163 ]
  %spec.select4 = select i1 %t3926, i64 %v139501, i64 %v64847
  %v167750 = or i64 %v146359, %v147572
  %v167755 = or i64 %v147572, %v147503
  %v167760 = or i64 %v167755, %v146359
  %v167765 = or i64 %v167760, %v146746
  %v167775 = or i64 %v167765, %v1471111098
  %v167780 = or i64 %v167775, %v1471161096
  %v167785 = or i64 %v1471161096, %v1471111098
  %t7936 = icmp eq i64 %v167780, 1
  br i1 %t7936, label %edge13194, label %edge13195

block88856:                                       ; preds = %edge13198, %edge13195
  br i1 %t7638.not, label %block88860, label %edge13204

block88860:                                       ; preds = %edge13204, %block88856
  br i1 %t3704.not, label %edge13218, label %edge13208

block88884:                                       ; preds = %edge13221, %edge13218
  br i1 %t3900.not138223402234922395, label %edge13236, label %edge13225

block88912:                                       ; preds = %edge13239, %edge13236
  %t7956.not = icmp eq i64 %v219120, 0
  br i1 %t7956.not, label %edge13432, label %edge13245

block88851:                                       ; preds = %edge13245, %edge13239, %edge13228, %edge13221, %edge13211, %edge13204, %edge13198, %edge13253
  %v167787 = phi i64 [ %v167858, %edge13253 ], [ %v167850, %edge13239 ], [ %v167826, %edge13228 ], [ %v167822, %edge13221 ], [ %v167802, %edge13211 ], [ %v167798, %edge13204 ], [ %v167794, %edge13198 ], [ %v167854, %edge13245 ]
  %t7963 = insertvalue { i64, i64 } poison, i64 %v167787, 0
  %t7964 = insertvalue { i64, i64 } %t7963, i64 6945, 1
  br label %common.ret

block91131:                                       ; preds = %edge13705, %edge13701
  br i1 %t8003.not, label %block91135, label %edge13711

block91135:                                       ; preds = %edge13711, %block91131
  %t8110.not = icmp eq i64 %v132190, 0
  br i1 %t8110.not, label %block91139, label %edge13717

block91139:                                       ; preds = %edge13717, %block91135
  %t8112.not = icmp eq i64 %v132196, 0
  br i1 %t8112.not, label %edge13730, label %edge13723

block91127:                                       ; preds = %edge13723, %edge13717, %edge13711, %edge13705
  %v176200 = phi i64 [ %v176215, %edge13717 ], [ %v176207, %edge13705 ], [ %v176211, %edge13711 ], [ %v176219, %edge13723 ]
  %t8114 = insertvalue { i64, i64 } poison, i64 %v176200, 0
  %t8115 = insertvalue { i64, i64 } %t8114, i64 7129, 1
  br label %common.ret

block7018:                                        ; preds = %block7018.preheader, %edge14014
  %v177566 = phi i64 [ %v178745, %edge14014 ], [ %v219193, %block7018.preheader ]
  %v177567 = phi i64 [ %v178715, %edge14014 ], [ %v219194, %block7018.preheader ]
  %v177568 = phi i64 [ %v178747, %edge14014 ], [ %v219195, %block7018.preheader ]
  %v177570 = phi i64 [ %v178715, %edge14014 ], [ %v219197, %block7018.preheader ]
  %v177571 = phi i64 [ %v178750, %edge14014 ], [ %v219198, %block7018.preheader ]
  %v177578 = phi i64 [ 0, %edge14014 ], [ %v219199, %block7018.preheader ]
  %v177581 = phi i64 [ 0, %edge14014 ], [ %v219200, %block7018.preheader ]
  %v177596 = phi i64 [ %v178783, %edge14014 ], [ %v54959, %block7018.preheader ]
  %t8119.not = icmp ult i64 %v54960, %v177596
  br i1 %t8119.not, label %common.ret, label %edge13755

block91576:                                       ; preds = %edge13787, %edge13791, %edge13800
  %v178011 = phi i64 [ %v178016, %edge13791 ], [ %v178033, %edge13800 ], [ 64, %edge13787 ]
  %t8158 = call i64 @llvm.umax.i64(i64 %v178011, i64 %v178005)
  br label %block91573

block91573:                                       ; preds = %edge13784, %block91576
  %v178010 = phi i64 [ %t8158, %block91576 ], [ %v177568, %edge13784 ]
  %t8159.not = icmp eq i64 %v177571, 0
  br i1 %t8159.not, label %block91584, label %edge13804

block91584:                                       ; preds = %edge13804, %block91573
  %t8164 = zext i64 %v177566 to i128
  %t8165 = zext i64 %v177567 to i128
  %t8166 = shl nuw i128 %t8165, 64
  %v220257 = or disjoint i128 %t8166, %t8164
  store i128 %v220257, ptr %ss0, align 16
  %t8168 = zext i64 %v177568 to i128
  %v220259 = or disjoint i128 %t8168, 18446744073709551616
  store i128 %v220259, ptr %t1002, align 16
  %t8173 = zext i64 %v177570 to i128
  %t8174 = zext i64 %v177571 to i128
  %t8175 = shl nuw i128 %t8174, 64
  %v220261 = or disjoint i128 %t8175, %t8173
  store i128 %v220261, ptr %t1007, align 16
  %v178050 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v178010, i64 1, i64 1, i64 %v3194)
  %t8178.not = icmp eq i64 %v178050, 0
  br i1 %t8178.not, label %edge13809, label %block91569

block91583:                                       ; preds = %edge13804, %edge13809
  %v178035 = phi i64 [ %v178051, %edge13809 ], [ %v177566, %edge13804 ]
  %v178037 = phi i64 [ %v178052, %edge13809 ], [ %v177568, %edge13804 ]
  %v178040 = phi i64 [ %v178053, %edge13809 ], [ %v177571, %edge13804 ]
  %v178055 = add i64 %v178035, %v177567
  %v178056 = trunc i64 %v177986 to i8
  %t8181 = inttoptr i64 %v178055 to ptr
  store i8 %v178056, ptr %t8181, align 1
  %t8182 = icmp eq i64 %v177571, %v178040
  %t8184.not249 = icmp eq i64 %v178040, 0
  %t8184.not = or i1 %t8182, %t8184.not249
  br i1 %t8184.not, label %block91588, label %edge13811

block91588:                                       ; preds = %block91583, %edge13811
  %t8188.not = or i1 %t8159.not, %t8182
  br i1 %t8188.not, label %edge13820, label %edge13816

block91569:                                       ; preds = %edge13816, %block91584, %edge13791, %edge13755
  %v177999 = phi i64 [ %v178050, %block91584 ], [ %v178066, %edge13816 ], [ 1, %edge13755 ], [ 1, %edge13791 ]
  %t8190 = insertvalue { i64, i64 } poison, i64 %v177999, 0
  %t8191 = insertvalue { i64, i64 } %t8190, i64 7172, 1
  br label %common.ret

block91718:                                       ; preds = %edge13831, %edge13835, %edge13844
  %v178244 = phi i64 [ %v178249, %edge13835 ], [ %v178266, %edge13844 ], [ 64, %edge13831 ]
  %t8221 = call i64 @llvm.umax.i64(i64 %v178244, i64 %v178238)
  br label %block91715

block91715:                                       ; preds = %edge13828, %block91718
  %v178243 = phi i64 [ %t8221, %block91718 ], [ %v178037, %edge13828 ]
  br i1 %t8184.not249, label %block91726, label %edge13848

block91726:                                       ; preds = %edge13848, %block91715
  %t8227 = zext i64 %v178035 to i128
  %t8228 = zext i64 %v178005 to i128
  %t8229 = shl nuw i128 %t8228, 64
  %v220263 = or disjoint i128 %t8229, %t8227
  store i128 %v220263, ptr %ss0, align 16
  %t8231 = zext i64 %v178037 to i128
  %v220265 = or disjoint i128 %t8231, 18446744073709551616
  store i128 %v220265, ptr %t1002, align 16
  %t8237 = zext i64 %v178040 to i128
  %t8238 = shl nuw i128 %t8237, 64
  %v220267 = or disjoint i128 %t8238, %t8228
  store i128 %v220267, ptr %t1007, align 16
  %v178283 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v178243, i64 1, i64 1, i64 %v3194)
  %t8241.not = icmp eq i64 %v178283, 0
  br i1 %t8241.not, label %edge13853, label %block91711

block91725:                                       ; preds = %edge13848, %edge13853
  %v178268 = phi i64 [ %v178284, %edge13853 ], [ %v178035, %edge13848 ]
  %v178270 = phi i64 [ %v178285, %edge13853 ], [ %v178037, %edge13848 ]
  %v178273 = phi i64 [ %v178286, %edge13853 ], [ %v178040, %edge13848 ]
  %v178288 = add i64 %v178268, %v178005
  %t8244 = inttoptr i64 %v178288 to ptr
  store i8 0, ptr %t8244, align 1
  %t8245 = icmp eq i64 %v178040, %v178273
  %t8247.not253 = icmp eq i64 %v178273, 0
  %t8247.not = or i1 %t8245, %t8247.not253
  br i1 %t8247.not, label %block91730, label %edge13855

block91730:                                       ; preds = %block91725, %edge13855
  %t8251.not = or i1 %t8184.not249, %t8245
  br i1 %t8251.not, label %edge13824, label %edge13860

block91711:                                       ; preds = %edge13860, %block91726, %edge13835, %edge13827
  %v178232 = phi i64 [ %v178283, %block91726 ], [ 1, %edge13827 ], [ 1, %edge13835 ], [ %v178299, %edge13860 ]
  %t8253 = insertvalue { i64, i64 } poison, i64 %v178232, 0
  %t8254 = insertvalue { i64, i64 } %t8253, i64 7178, 1
  br label %common.ret

block91755:                                       ; preds = %edge13875, %edge13879, %edge13888
  %v178342 = phi i64 [ %v178347, %edge13879 ], [ %v178364, %edge13888 ], [ 64, %edge13875 ]
  %t8281 = call i64 @llvm.umax.i64(i64 %v178342, i64 %v178336)
  br label %block91752

block91752:                                       ; preds = %edge13872, %block91755
  %v178341 = phi i64 [ %t8281, %block91755 ], [ %v178329, %edge13872 ]
  %t8282.not = icmp eq i64 %v178324, 0
  br i1 %t8282.not, label %block91763, label %edge13892

block91763:                                       ; preds = %edge13892, %block91752
  %t8287 = zext i64 %v178327 to i128
  %t8288 = zext i64 %v178328 to i128
  %t8289 = shl nuw i128 %t8288, 64
  %v220269 = or disjoint i128 %t8289, %t8287
  store i128 %v220269, ptr %ss0, align 16
  %t8291 = zext i64 %v178329 to i128
  %v220271 = or disjoint i128 %t8291, 18446744073709551616
  store i128 %v220271, ptr %t1002, align 16
  %t8297 = zext i64 %v178324 to i128
  %t8298 = shl nuw i128 %t8297, 64
  %v220273 = or disjoint i128 %t8298, %t8288
  store i128 %v220273, ptr %t1007, align 16
  %v178381 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v178341, i64 1, i64 1, i64 %v3194)
  %t8301.not = icmp eq i64 %v178381, 0
  br i1 %t8301.not, label %edge13897, label %block91747

block91783:                                       ; preds = %edge13899, %edge13909
  %t8312.not = or i1 %t8282.not, %t8306
  br i1 %t8312.not, label %edge13918, label %edge13914

block91747:                                       ; preds = %edge13914, %block91763, %edge13879, %edge13824
  %v178323 = phi i64 [ %v178381, %block91763 ], [ 1, %edge13824 ], [ 1, %edge13879 ], [ %v178433, %edge13914 ]
  %t8314 = insertvalue { i64, i64 } poison, i64 %v178323, 0
  %t8315 = insertvalue { i64, i64 } %t8314, i64 7183, 1
  br label %common.ret

block91797:                                       ; preds = %edge13925, %edge13929, %edge13938
  %v178612 = phi i64 [ %v178617, %edge13929 ], [ %v178634, %edge13938 ], [ 64, %edge13925 ]
  %t8342 = call i64 @llvm.umax.i64(i64 %v178612, i64 %v178606)
  br label %block91794

block91794:                                       ; preds = %edge13922, %block91797
  %v178611 = phi i64 [ %t8342, %block91797 ], [ %v178368, %edge13922 ]
  br i1 %t8308.not257, label %block91805, label %edge13942

block91805:                                       ; preds = %edge13942, %block91794
  %t8348 = zext i64 %v178366 to i128
  %t8349 = zext i64 %v178336 to i128
  %t8350 = shl nuw i128 %t8349, 64
  %v220275 = or disjoint i128 %t8350, %t8348
  store i128 %v220275, ptr %ss0, align 16
  %t8352 = zext i64 %v178368 to i128
  %v220277 = or disjoint i128 %t8352, 18446744073709551616
  store i128 %v220277, ptr %t1002, align 16
  %t8358 = zext i64 %v178371 to i128
  %t8359 = shl nuw i128 %t8358, 64
  %v220279 = or disjoint i128 %t8359, %t8349
  store i128 %v220279, ptr %t1007, align 16
  %v178651 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v178611, i64 1, i64 1, i64 %v3194)
  %t8362.not = icmp eq i64 %v178651, 0
  br i1 %t8362.not, label %edge13947, label %block91789

block91825:                                       ; preds = %edge13949, %edge13959
  %t8373.not = or i1 %t8308.not257, %t8367
  br i1 %t8373.not, label %edge13968, label %edge13964

block91789:                                       ; preds = %edge13964, %block91805, %edge13929, %edge13918
  %v178599 = phi i64 [ %v178651, %block91805 ], [ 1, %edge13918 ], [ 1, %edge13929 ], [ %v178703, %edge13964 ]
  %t8375 = insertvalue { i64, i64 } poison, i64 %v178599, 0
  %t8376 = insertvalue { i64, i64 } %t8375, i64 7185, 1
  br label %common.ret

block91838:                                       ; preds = %edge13975, %edge13979, %edge13988
  %v178721 = phi i64 [ %v178726, %edge13979 ], [ %v178743, %edge13988 ], [ 64, %edge13975 ]
  %t8407 = call i64 @llvm.umax.i64(i64 %v178721, i64 %v178715)
  br label %block91835

block91835:                                       ; preds = %edge13972, %block91838
  %v178720 = phi i64 [ %t8407, %block91838 ], [ %v178638, %edge13972 ]
  br i1 %t8369.not261, label %block91846, label %edge13992

block91846:                                       ; preds = %edge13992, %block91835
  %t8413 = zext i64 %v178636 to i128
  %t8414 = zext i64 %v178606 to i128
  %t8415 = shl nuw i128 %t8414, 64
  %v220281 = or disjoint i128 %t8415, %t8413
  store i128 %v220281, ptr %ss0, align 16
  %t8417 = zext i64 %v178638 to i128
  %v220283 = or disjoint i128 %t8417, 18446744073709551616
  store i128 %v220283, ptr %t1002, align 16
  %t8423 = zext i64 %v178641 to i128
  %t8424 = shl nuw i128 %t8423, 64
  %v220285 = or disjoint i128 %t8424, %t8414
  store i128 %v220285, ptr %t1007, align 16
  %v178760 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v178720, i64 1, i64 1, i64 %v3194)
  %t8427.not = icmp eq i64 %v178760, 0
  br i1 %t8427.not, label %edge13997, label %block91830

block91845:                                       ; preds = %edge13992, %edge13997
  %v178745 = phi i64 [ %v178761, %edge13997 ], [ %v178636, %edge13992 ]
  %v178747 = phi i64 [ %v178762, %edge13997 ], [ %v178638, %edge13992 ]
  %v178750 = phi i64 [ %v178763, %edge13997 ], [ %v178641, %edge13992 ]
  %t8430.not = icmp eq i64 %t8380, 0
  br i1 %t8430.not, label %block91850, label %edge13999

block91850:                                       ; preds = %block91845, %edge13999
  %t8431 = icmp eq i64 %v178641, %v178750
  %t8433.not265 = icmp eq i64 %v178750, 0
  %t8433.not = or i1 %t8431, %t8433.not265
  br i1 %t8433.not, label %block91852, label %edge14002

block91852:                                       ; preds = %block91850, %edge14002
  %t8437.not = or i1 %t8369.not261, %t8431
  br i1 %t8437.not, label %edge14011, label %edge14007

block91830:                                       ; preds = %edge14007, %block91846, %edge13979, %edge13970, %edge13968
  %v178705 = phi i64 [ %v178760, %block91846 ], [ 103427, %edge13968 ], [ 1, %edge13970 ], [ 1, %edge13979 ], [ %v178778, %edge14007 ]
  %t8439 = insertvalue { i64, i64 } poison, i64 %v178705, 0
  %t8440 = insertvalue { i64, i64 } %t8439, i64 7186, 1
  br label %common.ret

block92035:                                       ; preds = %edge14053, %edge14049
  %t8460.not = icmp eq i64 %v132196, 0
  br i1 %t8460.not, label %edge14072, label %edge14059

block92031:                                       ; preds = %edge14059, %edge14053
  %v179030 = phi i64 [ %v179037, %edge14053 ], [ %v179041, %edge14059 ]
  %t8462 = insertvalue { i64, i64 } poison, i64 %v179030, 0
  %t8463 = insertvalue { i64, i64 } %t8462, i64 7206, 1
  br label %common.ret

block92191:                                       ; preds = %edge14090, %edge14094, %edge14103
  %v179286 = phi i64 [ %v179291, %edge14094 ], [ %v179308, %edge14103 ], [ 64, %edge14090 ]
  %t8505 = call i64 @llvm.umax.i64(i64 %v179286, i64 %v179280)
  br label %block92188

block92188:                                       ; preds = %edge14087, %block92191
  %v179285 = phi i64 [ %t8505, %block92191 ], [ %v219195, %edge14087 ]
  %t8506.not = icmp eq i64 %v219198, 0
  br i1 %t8506.not, label %block92199, label %edge14107

block92199:                                       ; preds = %edge14107, %block92188
  %t8511 = zext i64 %v219193 to i128
  %t8512 = zext i64 %v219194 to i128
  %t8513 = shl nuw i128 %t8512, 64
  %v220287 = or disjoint i128 %t8513, %t8511
  store i128 %v220287, ptr %ss0, align 16
  %t8515 = zext i64 %v219195 to i128
  %v220289 = or disjoint i128 %t8515, 18446744073709551616
  store i128 %v220289, ptr %t1002, align 16
  %t8520 = zext i64 %v219197 to i128
  %t8521 = zext i64 %v219198 to i128
  %t8522 = shl nuw i128 %t8521, 64
  %v220291 = or disjoint i128 %t8522, %t8520
  store i128 %v220291, ptr %t1007, align 16
  %v179325 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v179285, i64 1, i64 1, i64 %v3194)
  %t8525.not = icmp eq i64 %v179325, 0
  br i1 %t8525.not, label %edge14112, label %block92183

block92198:                                       ; preds = %edge14107, %edge14112
  %v179310 = phi i64 [ %v179326, %edge14112 ], [ %v219193, %edge14107 ]
  %v179312 = phi i64 [ %v179327, %edge14112 ], [ %v219195, %edge14107 ]
  %v179315 = phi i64 [ %v179328, %edge14112 ], [ %v219198, %edge14107 ]
  %t8528.not = icmp eq i64 %v179260, 0
  br i1 %t8528.not, label %block92217, label %edge14114

block92217:                                       ; preds = %edge14114, %block92198
  %t8546 = icmp eq i64 %v219198, %v179315
  %t8548.not237 = icmp eq i64 %v179315, 0
  %t8548.not = or i1 %t8546, %t8548.not237
  br i1 %t8548.not, label %block92219, label %edge14134

block92219:                                       ; preds = %block92217, %edge14134
  %t8552.not = or i1 %t8506.not, %t8546
  br i1 %t8552.not, label %edge14064.thread, label %edge14139

block92183:                                       ; preds = %edge14139, %block92199, %edge14094, %edge14072
  %v179273 = phi i64 [ %v179325, %block92199 ], [ %v179377, %edge14139 ], [ 1, %edge14094 ], [ 1, %edge14072 ]
  %t8554 = insertvalue { i64, i64 } poison, i64 %v179273, 0
  %t8555 = insertvalue { i64, i64 } %t8554, i64 7221, 1
  br label %common.ret

block92615:                                       ; preds = %edge14259, %edge14263, %edge14272
  %v180420 = phi i64 [ %v180425, %edge14263 ], [ %v180442, %edge14272 ], [ 64, %edge14259 ]
  %t8639 = call i64 @llvm.umax.i64(i64 %v180420, i64 %v180414)
  br label %block92612

block92612:                                       ; preds = %edge14256, %block92615
  %v180419 = phi i64 [ %t8639, %block92615 ], [ %v219195, %edge14256 ]
  %t8640.not = icmp eq i64 %v219198, 0
  br i1 %t8640.not, label %block92623, label %edge14276

block92623:                                       ; preds = %edge14276, %block92612
  %t8645 = zext i64 %v219193 to i128
  %t8646 = zext i64 %v219194 to i128
  %t8647 = shl nuw i128 %t8646, 64
  %v220293 = or disjoint i128 %t8647, %t8645
  store i128 %v220293, ptr %ss0, align 16
  %t8649 = zext i64 %v219195 to i128
  %v220295 = or disjoint i128 %t8649, 18446744073709551616
  store i128 %v220295, ptr %t1002, align 16
  %t8654 = zext i64 %v219197 to i128
  %t8655 = zext i64 %v219198 to i128
  %t8656 = shl nuw i128 %t8655, 64
  %v220297 = or disjoint i128 %t8656, %t8654
  store i128 %v220297, ptr %t1007, align 16
  %v180459 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v180419, i64 1, i64 1, i64 %v3194)
  %t8659.not = icmp eq i64 %v180459, 0
  br i1 %t8659.not, label %edge14281, label %block92607

block92641:                                       ; preds = %edge14289, %edge14283
  %t8686 = icmp eq i64 %v219198, %v180449
  %t8688.not225 = icmp eq i64 %v180449, 0
  %t8688.not = or i1 %t8686, %t8688.not225
  br i1 %t8688.not, label %block92643, label %edge14307

block92643:                                       ; preds = %block92641, %edge14307
  %t8692.not = or i1 %t8640.not, %t8686
  br i1 %t8692.not, label %edge14316, label %edge14312

block92607:                                       ; preds = %edge14312, %block92623, %edge14263, %edge14240
  %v180407 = phi i64 [ %v180459, %block92623 ], [ %v180511, %edge14312 ], [ 1, %edge14263 ], [ 1, %edge14240 ]
  %t8694 = insertvalue { i64, i64 } poison, i64 %v180407, 0
  %t8695 = insertvalue { i64, i64 } %t8694, i64 7307, 1
  br label %common.ret

block7199:                                        ; preds = %edge14435, %edge14335
  %v180623 = phi i64 [ 1, %edge14335 ], [ %v181751, %edge14435 ]
  %v180626 = phi i64 [ %v180528, %edge14335 ], [ %v181566, %edge14435 ]
  %v180629 = phi i64 [ %v180533, %edge14335 ], [ %v181759, %edge14435 ]
  %or.cond324 = icmp samesign ult i64 %v180623, %invariant.umin
  br i1 %or.cond324, label %edge14372, label %edge14384

block93512:                                       ; preds = %edge14466, %edge14470, %edge14479
  %v181846 = phi i64 [ %v181851, %edge14470 ], [ %v181868, %edge14479 ], [ 64, %edge14466 ]
  %t8761 = call i64 @llvm.umax.i64(i64 %v181846, i64 %v181840)
  br label %block93509

block93509:                                       ; preds = %edge14463, %block93512
  %v181845 = phi i64 [ %t8761, %block93512 ], [ %v180446, %edge14463 ]
  br i1 %t8688.not225, label %block93520, label %edge14483

block93520:                                       ; preds = %edge14483, %block93509
  %t8767 = zext i64 %v180444 to i128
  %t8768 = zext i64 %v180414 to i128
  %t8769 = shl nuw i128 %t8768, 64
  %v220299 = or disjoint i128 %t8769, %t8767
  store i128 %v220299, ptr %ss0, align 16
  %t8771 = zext i64 %v180446 to i128
  %v220301 = or disjoint i128 %t8771, 18446744073709551616
  store i128 %v220301, ptr %t1002, align 16
  %t8777 = zext i64 %v180449 to i128
  %t8778 = shl nuw i128 %t8777, 64
  %v220303 = or disjoint i128 %t8778, %t8768
  store i128 %v220303, ptr %t1007, align 16
  %v181885 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v181845, i64 1, i64 1, i64 %v3194)
  %t8781.not = icmp eq i64 %v181885, 0
  br i1 %t8781.not, label %edge14488, label %block93504

block93519:                                       ; preds = %edge14483, %edge14488
  %v181870 = phi i64 [ %v181886, %edge14488 ], [ %v180444, %edge14483 ]
  %v181872 = phi i64 [ %v181887, %edge14488 ], [ %v180446, %edge14483 ]
  %v181875 = phi i64 [ %v181888, %edge14488 ], [ %v180449, %edge14483 ]
  %t8784.not = icmp eq i64 %v181825, 0
  br i1 %t8784.not, label %block93538, label %edge14490

block93538:                                       ; preds = %edge14508, %edge14505, %edge14502, %edge14499, %edge14496, %edge14493, %edge14490, %block93519, %edge14511
  %t8808 = icmp eq i64 %v180449, %v181875
  %t8810.not229 = icmp eq i64 %v181875, 0
  %t8810.not = or i1 %t8808, %t8810.not229
  br i1 %t8810.not, label %block93540, label %edge14514

block93540:                                       ; preds = %block93538, %edge14514
  %t8814.not = or i1 %t8688.not225, %t8808
  br i1 %t8814.not, label %edge14523, label %edge14519

block93504:                                       ; preds = %edge14519, %block93520, %edge14470, %edge14461, %edge14455
  %v181833 = phi i64 [ %v181885, %block93520 ], [ 103427, %edge14455 ], [ 1, %edge14461 ], [ 1, %edge14470 ], [ %v181937, %edge14519 ]
  %t8816 = insertvalue { i64, i64 } poison, i64 %v181833, 0
  %t8817 = insertvalue { i64, i64 } %t8816, i64 7414, 1
  br label %common.ret

block7305:                                        ; preds = %edge14733, %edge14523
  %v182385 = phi i64 [ %v181870, %edge14523 ], [ %v183667, %edge14733 ]
  %v182386 = phi i64 [ %v181840, %edge14523 ], [ %v183637, %edge14733 ]
  %v182387 = phi i64 [ %v181872, %edge14523 ], [ %v183669, %edge14733 ]
  %v182390 = phi i64 [ %v181875, %edge14523 ], [ %v183672, %edge14733 ]
  %v182397 = phi i64 [ %v181954, %edge14523 ], [ %v183751, %edge14733 ]
  %v182400.in = phi i64 [ %v180629, %edge14523 ], [ %v183608, %edge14733 ]
  %v182442 = phi i64 [ 0, %edge14523 ], [ %v183987, %edge14733 ]
  %v182400 = and i64 %v182400.in, 7
  %exitcond21699.not = icmp eq i64 %v182442, %v132197
  br i1 %exitcond21699.not, label %edge14572, label %edge14582

block94298:                                       ; preds = %edge14676, %edge14680, %edge14689
  %v183643 = phi i64 [ %v183648, %edge14680 ], [ %v183665, %edge14689 ], [ 64, %edge14676 ]
  %t8879 = call i64 @llvm.umax.i64(i64 %v183643, i64 %v183637)
  br label %block94295

block94295:                                       ; preds = %edge14673, %block94298
  %v183642 = phi i64 [ %t8879, %block94298 ], [ %v182387, %edge14673 ]
  %t8880.not = icmp eq i64 %v182390, 0
  br i1 %t8880.not, label %block94306, label %edge14693

block94306:                                       ; preds = %edge14693, %block94295
  %t8885 = zext i64 %v182385 to i128
  %t8886 = zext i64 %v182386 to i128
  %t8887 = shl nuw i128 %t8886, 64
  %v220305 = or disjoint i128 %t8887, %t8885
  store i128 %v220305, ptr %ss0, align 16
  %t8889 = zext i64 %v182387 to i128
  %v220307 = or disjoint i128 %t8889, 18446744073709551616
  store i128 %v220307, ptr %t1002, align 16
  %t8895 = zext i64 %v182390 to i128
  %t8896 = shl nuw i128 %t8895, 64
  %v220309 = or disjoint i128 %t8896, %t8886
  store i128 %v220309, ptr %t1007, align 16
  %v183682 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v183642, i64 1, i64 1, i64 %v3194)
  %t8899.not = icmp eq i64 %v183682, 0
  br i1 %t8899.not, label %edge14698, label %block94290

block94305:                                       ; preds = %edge14693, %edge14698
  %v183667 = phi i64 [ %v183683, %edge14698 ], [ %v182385, %edge14693 ]
  %v183669 = phi i64 [ %v183684, %edge14698 ], [ %v182387, %edge14693 ]
  %v183672 = phi i64 [ %v183685, %edge14698 ], [ %v182390, %edge14693 ]
  %t8902.not = icmp eq i64 %v183622, 0
  br i1 %t8902.not, label %block94324, label %edge14700

block94324:                                       ; preds = %edge14718, %edge14715, %edge14712, %edge14709, %edge14706, %edge14703, %edge14700, %block94305, %edge14721
  %t8926 = icmp eq i64 %v182390, %v183672
  %t8928.not233 = icmp eq i64 %v183672, 0
  %t8928.not = or i1 %t8926, %t8928.not233
  br i1 %t8928.not, label %block94326, label %edge14724

block94326:                                       ; preds = %block94324, %edge14724
  %t8932.not = or i1 %t8880.not, %t8926
  br i1 %t8932.not, label %edge14733, label %edge14729

block94290:                                       ; preds = %edge14729, %block94306, %edge14680, %edge14671, %edge14662
  %v183630 = phi i64 [ %v183682, %block94306 ], [ 103427, %edge14662 ], [ 1, %edge14671 ], [ 1, %edge14680 ], [ %v183734, %edge14729 ]
  %t8934 = insertvalue { i64, i64 } poison, i64 %v183630, 0
  %t8935 = insertvalue { i64, i64 } %t8934, i64 7525, 1
  br label %common.ret

block94361:                                       ; preds = %edge14745, %edge14572
  br i1 %t8698.not, label %edge14805, label %edge14749

block94357:                                       ; preds = %edge14752, %edge14745
  %v183792 = phi i64 [ %v183799, %edge14745 ], [ %v183803, %edge14752 ]
  %t8945 = insertvalue { i64, i64 } poison, i64 %v183792, 0
  %t8946 = insertvalue { i64, i64 } %t8945, i64 7536, 1
  br label %common.ret

block94704:                                       ; preds = %edge14805, %edge14812
  %t8961.not = icmp eq i64 %v185060, 0
  br i1 %t8961.not, label %edge14829, label %edge14815

block7441:                                        ; preds = %edge15482, %edge14829
  %v186273 = phi i64 [ %v1849871138, %edge14829 ], [ %v193128, %edge15482 ]
  %v186274 = phi i64 [ %v1849881140, %edge14829 ], [ %v193098, %edge15482 ]
  %v186275 = phi i64 [ %v1849891142, %edge14829 ], [ %v193130, %edge15482 ]
  %v186278 = phi i64 [ %v1849921144, %edge14829 ], [ %v193133, %edge15482 ]
  %v186285 = phi i64 [ %v1849991146, %edge14829 ], [ %v193212, %edge15482 ]
  %v186288 = phi i64 [ %v1850021148, %edge14829 ], [ %v193225, %edge15482 ]
  %v186324 = phi i64 [ 0, %edge14829 ], [ %v193502, %edge15482 ]
  %t8964.not = icmp eq i64 %v186324, %v55001
  br i1 %t8964.not, label %edge15411, label %edge14865

block96263:                                       ; preds = %edge15034, %edge15035
  br i1 %t8961.not, label %block7634.preheader, label %edge15038

block7634.preheader:                              ; preds = %edge15038, %block96263
  br label %block7634

block7634:                                        ; preds = %block7634.preheader, %edge15155
  %v189069 = phi i64 [ %v190089, %edge15155 ], [ 0, %block7634.preheader ]
  %v189072 = phi i64 [ %v190101, %edge15155 ], [ 0, %block7634.preheader ]
  %v189087 = phi i64 [ %v190106, %edge15155 ], [ %v186807, %block7634.preheader ]
  %v189102 = phi i64 [ %v190301, %edge15155 ], [ %v186812, %block7634.preheader ]
  %t9020.not = icmp ne i64 %v189102, 0
  %t9023 = icmp ult i64 %v189087, %v1397
  %or.cond325 = select i1 %t9020.not, i1 %t9023, i1 false
  br i1 %or.cond325, label %edge15081, label %edge15096

trap9030:                                         ; preds = %edge15085
  call void @llvm.trap()
  unreachable

ok9030:                                           ; preds = %edge15085
  %v189548 = udiv i64 %v189087, %v1399
  %v189549 = mul i64 %v189548, %v1399
  %v189550 = sub nuw i64 %v189087, %v189549
  %v189551 = mul i64 %v189550, %v1400
  %v189552 = add i64 %v189551, %v189548
  br label %block96614

block96614:                                       ; preds = %edge15081, %ok9030
  %v189547 = phi i64 [ %v189552, %ok9030 ], [ %v189087, %edge15081 ]
  %v189554 = add i64 %v189547, %v1396
  %t9031 = inttoptr i64 %v189554 to ptr
  %t9032 = load i8, ptr %t9031, align 1
  %v189555 = zext i8 %t9032 to i64
  br label %edge15096

block96964:                                       ; preds = %edge15141, %edge15138
  br i1 %t8961.not, label %edge15406, label %edge15145

block96960:                                       ; preds = %edge15148, %edge15141
  %v190108 = phi i64 [ %v190115, %edge15141 ], [ %v190119, %edge15148 ]
  %t9051 = insertvalue { i64, i64 } poison, i64 %v190108, 0
  %t9052 = insertvalue { i64, i64 } %t9051, i64 7844, 1
  br label %common.ret

block98644:                                       ; preds = %edge15425, %edge15429, %edge15438
  %v193104 = phi i64 [ %v193109, %edge15429 ], [ %v193126, %edge15438 ], [ 64, %edge15425 ]
  %t9137 = call i64 @llvm.umax.i64(i64 %v193104, i64 %v193098)
  br label %block98641

block98641:                                       ; preds = %edge15422, %block98644
  %v193103 = phi i64 [ %t9137, %block98644 ], [ %v186275, %edge15422 ]
  %t9138.not = icmp eq i64 %v186278, 0
  br i1 %t9138.not, label %block98652, label %edge15442

block98652:                                       ; preds = %edge15442, %block98641
  %t9143 = zext i64 %v186273 to i128
  %t9144 = zext i64 %v186274 to i128
  %t9145 = shl nuw i128 %t9144, 64
  %v220311 = or disjoint i128 %t9145, %t9143
  store i128 %v220311, ptr %ss0, align 16
  %t9147 = zext i64 %v186275 to i128
  %v220313 = or disjoint i128 %t9147, 18446744073709551616
  store i128 %v220313, ptr %t1002, align 16
  %t9153 = zext i64 %v186278 to i128
  %t9154 = shl nuw i128 %t9153, 64
  %v220315 = or disjoint i128 %t9154, %t9144
  store i128 %v220315, ptr %t1007, align 16
  %v193143 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v193103, i64 1, i64 1, i64 %v3194)
  %t9157.not = icmp eq i64 %v193143, 0
  br i1 %t9157.not, label %edge15447, label %block98636

block98651:                                       ; preds = %edge15442, %edge15447
  %v193128 = phi i64 [ %v193144, %edge15447 ], [ %v186273, %edge15442 ]
  %v193130 = phi i64 [ %v193145, %edge15447 ], [ %v186275, %edge15442 ]
  %v193133 = phi i64 [ %v193146, %edge15447 ], [ %v186278, %edge15442 ]
  %t9160.not = icmp eq i64 %v193035, 0
  br i1 %t9160.not, label %block98670, label %edge15449

block98670:                                       ; preds = %edge15467, %edge15464, %edge15461, %edge15458, %edge15455, %edge15452, %edge15449, %block98651, %edge15470
  %t9184 = icmp eq i64 %v186278, %v193133
  %t9186.not245 = icmp eq i64 %v193133, 0
  %t9186.not = or i1 %t9184, %t9186.not245
  br i1 %t9186.not, label %block98672, label %edge15473

block98672:                                       ; preds = %block98670, %edge15473
  %t9190.not = or i1 %t9138.not, %t9184
  br i1 %t9190.not, label %edge15482, label %edge15478

block98636:                                       ; preds = %edge15478, %block98652, %edge15429, %edge15420, %edge15413
  %v193043 = phi i64 [ %v193143, %block98652 ], [ 103427, %edge15413 ], [ 1, %edge15420 ], [ 1, %edge15429 ], [ %v193195, %edge15478 ]
  %t9192 = insertvalue { i64, i64 } poison, i64 %v193043, 0
  %t9193 = insertvalue { i64, i64 } %t9192, i64 8083, 1
  br label %common.ret

block98707:                                       ; preds = %edge15494, %edge15411
  br i1 %t8003.not, label %block98711, label %edge15500

block98711:                                       ; preds = %edge15500, %block98707
  br i1 %t8961.not, label %edge15551.thread, label %edge15504

block98703:                                       ; preds = %edge15507, %edge15500, %edge15494
  %v193260 = phi i64 [ %v193271, %edge15500 ], [ %v193267, %edge15494 ], [ %v193275, %edge15507 ]
  %t9205 = insertvalue { i64, i64 } poison, i64 %v193260, 0
  %t9206 = insertvalue { i64, i64 } %t9205, i64 8094, 1
  br label %common.ret

block99090:                                       ; preds = %edge15603, %edge15607, %edge15616
  %v194481 = phi i64 [ %v194486, %edge15607 ], [ %v194503, %edge15616 ], [ 64, %edge15603 ]
  %t9259 = call i64 @llvm.umax.i64(i64 %v194481, i64 %v194475)
  br label %block99087

block99087:                                       ; preds = %edge15600, %block99090
  %v194480 = phi i64 [ %t9259, %block99090 ], [ %v186275, %edge15600 ]
  %t9260.not = icmp eq i64 %v186278, 0
  br i1 %t9260.not, label %block99098, label %edge15620

block99098:                                       ; preds = %edge15620, %block99087
  %t9265 = zext i64 %v186273 to i128
  %t9266 = zext i64 %v186274 to i128
  %t9267 = shl nuw i128 %t9266, 64
  %v220317 = or disjoint i128 %t9267, %t9265
  store i128 %v220317, ptr %ss0, align 16
  %t9269 = zext i64 %v186275 to i128
  %v220319 = or disjoint i128 %t9269, 18446744073709551616
  store i128 %v220319, ptr %t1002, align 16
  %t9275 = zext i64 %v186278 to i128
  %t9276 = shl nuw i128 %t9275, 64
  %v220321 = or disjoint i128 %t9276, %t9266
  store i128 %v220321, ptr %t1007, align 16
  %v194520 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v194480, i64 1, i64 1, i64 %v3194)
  %t9279.not = icmp eq i64 %v194520, 0
  br i1 %t9279.not, label %edge15625, label %block99082

block99097:                                       ; preds = %edge15620, %edge15625
  %v194505 = phi i64 [ %v194521, %edge15625 ], [ %v186273, %edge15620 ]
  %v194507 = phi i64 [ %v194522, %edge15625 ], [ %v186275, %edge15620 ]
  %v194510 = phi i64 [ %v194523, %edge15625 ], [ %v186278, %edge15620 ]
  %t9282.not = icmp eq i64 %v194460, 0
  br i1 %t9282.not, label %block99116, label %edge15627

block99116:                                       ; preds = %edge15645, %edge15642, %edge15639, %edge15636, %edge15633, %edge15630, %edge15627, %block99097, %edge15648
  %t9306 = icmp eq i64 %v186278, %v194510
  %t9308.not241 = icmp eq i64 %v194510, 0
  %t9308.not = or i1 %t9306, %t9308.not241
  br i1 %t9308.not, label %block99118, label %edge15651

block99118:                                       ; preds = %block99116, %edge15651
  %t9312.not = or i1 %t9260.not, %t9306
  br i1 %t9312.not, label %edge15660, label %edge15656

block99082:                                       ; preds = %edge15656, %block99098, %edge15607, %edge15598, %edge15579
  %v194468 = phi i64 [ %v194520, %block99098 ], [ 103427, %edge15579 ], [ 1, %edge15598 ], [ 1, %edge15607 ], [ %v194572, %edge15656 ]
  %t9314 = insertvalue { i64, i64 } poison, i64 %v194468, 0
  %t9315 = insertvalue { i64, i64 } %t9314, i64 8143, 1
  br label %common.ret

block99166:                                       ; preds = %edge15689, %edge15686
  %t9324.not = icmp eq i64 %v54956, 0
  br i1 %t9324.not, label %edge15935, label %edge15695

block99162:                                       ; preds = %edge15695, %edge15689
  %v196359 = phi i64 [ %v196367, %edge15689 ], [ %v196371, %edge15695 ]
  %t9326 = insertvalue { i64, i64 } poison, i64 %v196359, 0
  %t9327 = insertvalue { i64, i64 } %t9326, i64 8163, 1
  br label %common.ret

block8027:                                        ; preds = %edge15681, %edge15794
  %v196526 = phi i64 [ %v197139, %edge15794 ], [ %v54926, %edge15681 ]
  %v196546 = phi i64 [ %v197136, %edge15794 ], [ %v54923, %edge15681 ]
  %v196549 = phi i64 [ %v197354, %edge15794 ], [ 0, %edge15681 ]
  %v196550 = phi i64 [ %v197134, %edge15794 ], [ %v54921, %edge15681 ]
  %t9328.not = icmp eq i64 %v196526, 0
  br i1 %t9328.not, label %edge15727, label %edge15707

block99677:                                       ; preds = %edge15766, %edge15765
  %t9387 = zext i64 %v196550 to i128
  %v220331 = or disjoint i128 %t1719, %t9387
  store i128 %v220331, ptr %ss0, align 16
  %t9391 = zext i64 %v196546 to i128
  %v220333 = or disjoint i128 %t9391, 18446744073709551616
  store i128 %v220333, ptr %t1002, align 16
  %t9397 = zext i64 %v196526 to i128
  %t9398 = shl nuw i128 %t9397, 64
  %v220335 = or disjoint i128 %t9398, %t1163
  store i128 %v220335, ptr %t1007, align 16
  %v197155 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v196546, i64 4, i64 4, i64 %v3194)
  %t9401.not = icmp eq i64 %v197155, 0
  br i1 %t9401.not, label %edge15771, label %block99672

block99676:                                       ; preds = %edge15766, %edge15771
  %t9404.pre-phi = phi ptr [ %t9378, %edge15766 ], [ %.pre21728, %edge15771 ]
  %v197140 = phi i64 [ %v196550, %edge15766 ], [ %v197156, %edge15771 ]
  %v197142 = phi i64 [ %v196546, %edge15766 ], [ %v197157, %edge15771 ]
  %v197145 = phi i64 [ %v196526, %edge15766 ], [ %v197158, %edge15771 ]
  %v197160 = trunc nuw i64 %v197171 to i32
  store i32 %v197160, ptr %t9404.pre-phi, align 1
  br label %block99675

block99675:                                       ; preds = %edge15762, %block99676
  %v197134 = phi i64 [ %v197140, %block99676 ], [ %v196550, %edge15762 ]
  %v197136 = phi i64 [ %v197142, %block99676 ], [ %v196546, %edge15762 ]
  %v197139 = phi i64 [ %v197145, %block99676 ], [ %v196526, %edge15762 ]
  %t9405 = icmp eq i64 %v196526, %v197139
  %t9407.not268 = icmp eq i64 %v197139, 0
  %t9407.not = or i1 %t9405, %t9407.not268
  br i1 %t9407.not, label %block99681, label %edge15774

block99681:                                       ; preds = %block99675, %edge15774
  %t9411.not = or i1 %t9328.not, %t9405
  br i1 %t9411.not, label %edge15794, label %edge15779

block99672:                                       ; preds = %edge15779, %block99677, %edge15759
  %v197122 = phi i64 [ %v197155, %block99677 ], [ 103425, %edge15759 ], [ %v197169, %edge15779 ]
  %t9413 = insertvalue { i64, i64 } poison, i64 %v197122, 0
  %t9414 = insertvalue { i64, i64 } %t9413, i64 8207, 1
  br label %common.ret

block99860:                                       ; preds = %edge15839
  %t9446 = insertvalue { i64, i64 } poison, i64 %v198512, 0
  %t9447 = insertvalue { i64, i64 } %t9446, i64 8230, 1
  br label %common.ret

block99915:                                       ; preds = %edge15882
  %t9480 = insertvalue { i64, i64 } poison, i64 %v199737, 0
  %t9481 = insertvalue { i64, i64 } %t9480, i64 8243, 1
  br label %common.ret

block100014:                                      ; preds = %edge15935
  %t9509 = insertvalue { i64, i64 } poison, i64 %v201095, 0
  %t9510 = insertvalue { i64, i64 } %t9509, i64 8260, 1
  br label %common.ret

block100069:                                      ; preds = %edge15975
  %t9538 = insertvalue { i64, i64 } poison, i64 %v202316, 0
  %t9539 = insertvalue { i64, i64 } %t9538, i64 8273, 1
  br label %common.ret

block100124:                                      ; preds = %edge16015
  %t9567 = insertvalue { i64, i64 } poison, i64 %v203553, 0
  %t9568 = insertvalue { i64, i64 } %t9567, i64 8286, 1
  br label %common.ret

block100179:                                      ; preds = %edge16058
  %t9601 = insertvalue { i64, i64 } poison, i64 %v204798, 0
  %t9602 = insertvalue { i64, i64 } %t9601, i64 8299, 1
  br label %common.ret

block100389:                                      ; preds = %edge16129, %edge16132
  %t9610.not = icmp eq i64 %v140768, 0
  br i1 %t9610.not, label %block100393, label %edge16144

block100393:                                      ; preds = %edge16144, %block100389
  %t9612.not = icmp eq i64 %v219119, 0
  br i1 %t9612.not, label %block100397, label %edge16150

block100397:                                      ; preds = %edge16150, %block100393
  %t9614.not = icmp eq i64 %v54920, 0
  br i1 %t9614.not, label %block100401, label %edge16156

block100401:                                      ; preds = %edge16156, %block100397
  %t9616.not = icmp eq i64 %v205567, 0
  br i1 %t9616.not, label %block100405, label %edge16162

block100405:                                      ; preds = %edge16162, %block100401
  br i1 %t8003.not, label %block100409, label %edge16168

block100409:                                      ; preds = %edge16168, %block100405
  %t9620.not = icmp eq i64 %v206124, 0
  br i1 %t9620.not, label %block100413, label %edge16174

block100413:                                      ; preds = %edge16174, %block100409
  %t9622.not = icmp eq i64 %v206136, 0
  br i1 %t9622.not, label %block100417, label %edge16180

block100417:                                      ; preds = %edge16180, %block100413
  br i1 %t3704.not, label %block100421, label %edge16186

block100421:                                      ; preds = %edge16186, %block100417
  %t9626.not = icmp eq i64 %v206711, 0
  br i1 %t9626.not, label %block100425, label %edge16192

block100425:                                      ; preds = %edge16192, %block100421
  %t9628.not = icmp eq i64 %v206723, 0
  br i1 %t9628.not, label %edge16248, label %edge16196

block100317:                                      ; preds = %edge16199, %edge16192, %edge16186, %edge16180, %edge16174, %edge16168, %edge16162, %edge16156, %edge16150, %edge16144, %edge16132
  %v206918 = phi i64 [ %v207042, %edge16192 ], [ %v207002, %edge16132 ], [ %v207046, %edge16199 ], [ %v207010, %edge16144 ], [ %v207014, %edge16150 ], [ %v207018, %edge16156 ], [ %v207022, %edge16162 ], [ %v207026, %edge16168 ], [ %v207030, %edge16174 ], [ %v207034, %edge16180 ], [ %v207038, %edge16186 ]
  %t9633 = insertvalue { i64, i64 } poison, i64 %v206918, 0
  %t9634 = insertvalue { i64, i64 } %t9633, i64 8333, 1
  br label %common.ret

block100716:                                      ; preds = %edge16265, %edge16269, %edge16278
  %v218638 = phi i64 [ %v218643, %edge16269 ], [ %v218660, %edge16278 ], [ 64, %edge16265 ]
  %t9665 = call i64 @llvm.umax.i64(i64 %v218638, i64 %v218632)
  br label %block100713

block100713:                                      ; preds = %edge16262, %block100716
  %v218637 = phi i64 [ %t9665, %block100716 ], [ %v206796, %edge16262 ]
  %t9666.not = icmp eq i64 %v206838, 0
  br i1 %t9666.not, label %block100724, label %edge16282

block100724:                                      ; preds = %edge16282, %block100713
  %t9671 = zext i64 %v206768 to i128
  %t9672 = zext i64 %v206782 to i128
  %t9673 = shl nuw i128 %t9672, 64
  %v220337 = or disjoint i128 %t9673, %t9671
  store i128 %v220337, ptr %ss0, align 16
  %t9675 = zext i64 %v206796 to i128
  %v220339 = or disjoint i128 %t9675, 18446744073709551616
  store i128 %v220339, ptr %t1002, align 16
  %t9681 = zext i64 %v206838 to i128
  %t9682 = shl nuw i128 %t9681, 64
  %v220341 = or disjoint i128 %t9682, %t9672
  store i128 %v220341, ptr %t1007, align 16
  %v218677 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 1, i64 %v218637, i64 1, i64 1, i64 %v3194)
  %t9685.not = icmp eq i64 %v218677, 0
  br i1 %t9685.not, label %edge16287, label %block100709

block100723:                                      ; preds = %edge16282, %edge16287
  %v218662 = phi i64 [ %v218678, %edge16287 ], [ %v206768, %edge16282 ]
  %v218664 = phi i64 [ %v218679, %edge16287 ], [ %v206796, %edge16282 ]
  %v218667 = phi i64 [ %v218680, %edge16287 ], [ %v206838, %edge16282 ]
  %v218682 = add i64 %v218662, %v206782
  %v218683 = trunc i64 %v206858 to i8
  %t9688 = inttoptr i64 %v218682 to ptr
  store i8 %v218683, ptr %t9688, align 1
  %t9689 = icmp eq i64 %v206838, %v218667
  %t9691.not272 = icmp eq i64 %v218667, 0
  %t9691.not = or i1 %t9689, %t9691.not272
  br i1 %t9691.not, label %block100728, label %edge16289

block100728:                                      ; preds = %block100723, %edge16289
  %t9695.not = or i1 %t9666.not, %t9689
  br i1 %t9695.not, label %edge16309, label %edge16294

block100709:                                      ; preds = %edge16294, %block100724, %edge16269, %edge16257
  %v218626 = phi i64 [ %v218677, %block100724 ], [ 1, %edge16257 ], [ 1, %edge16269 ], [ %v218693, %edge16294 ]
  %t9697 = insertvalue { i64, i64 } poison, i64 %v218626, 0
  %t9698 = insertvalue { i64, i64 } %t9697, i64 8357, 1
  br label %common.ret

block100765:                                      ; preds = %edge16310, %edge16309
  %t9703 = inttoptr i64 %v2 to ptr
  store i64 %v218735, ptr %t9703, align 1
  %t9705 = getelementptr i8, ptr %t9703, i64 8
  store i64 %v218736, ptr %t9705, align 1
  %t9707 = getelementptr i8, ptr %t9703, i64 16
  store i64 %v218737, ptr %t9707, align 1
  %t9709 = getelementptr i8, ptr %t9703, i64 24
  store i64 1, ptr %t9709, align 1
  %t9711 = getelementptr i8, ptr %t9703, i64 32
  store i64 %v218736, ptr %t9711, align 1
  %t9713 = getelementptr i8, ptr %t9703, i64 40
  store i64 %v218740, ptr %t9713, align 1
  br label %common.ret

edge0:                                            ; preds = %block0
  %t24 = inttoptr i64 %v1401 to ptr
  %v1588 = load i64, ptr %t24, align 1
  %v1589 = add i64 %v1588, 1
  store i64 %v1589, ptr %t24, align 1
  br label %block2

edge3:                                            ; preds = %block2
  %t27 = inttoptr i64 %v1407 to ptr
  %v1590 = load i64, ptr %t27, align 1
  %v1591 = add i64 %v1590, 1
  store i64 %v1591, ptr %t27, align 1
  br label %edge12

edge12:                                           ; preds = %edge3, %block2
  %t29 = icmp ult i64 %v1397, 8
  br i1 %t29, label %edge38, label %edge462

edge38:                                           ; preds = %edge12
  %t44 = icmp eq i64 %v1405, 1
  br i1 %t44, label %edge47, label %block8389

edge47:                                           ; preds = %edge38
  %v1827 = add nuw nsw i64 %v1397, 5
  %v1839 = tail call i64 @llvm.uadd.sat.i64(i64 %v1403, i64 %v1827)
  %t49 = icmp ugt i64 %v1839, %v1404
  br i1 %t49, label %edge49, label %edge68

edge49:                                           ; preds = %edge47
  %v1857 = ptrtoint ptr %ss1 to i64
  %v219522 = ptrtoint ptr %ss0 to i64
  %t56 = zext i64 %v1402 to i128
  %t57 = zext i64 %v1403 to i128
  %t58 = shl nuw i128 %t57, 64
  %v219523 = or disjoint i128 %t58, %t56
  store i128 %v219523, ptr %ss0, align 16
  %t60 = zext i64 %v1404 to i128
  %v219525 = or disjoint i128 %t60, 18446744073709551616
  %t64 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  store i128 %v219525, ptr %t64, align 16
  %t65 = zext i64 %v1406 to i128
  %t66 = zext i64 %v1407 to i128
  %t67 = shl nuw i128 %t66, 64
  %v219527 = or disjoint i128 %t67, %t65
  %t69 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  store i128 %v219527, ptr %t69, align 16
  %v1862 = call i64 @mech_compact_writable(i64 %v0, i64 %v219522, i64 1, i64 %v1839, i64 1, i64 1, i64 %v1857)
  %t70.not = icmp eq i64 %v1862, 0
  br i1 %t70.not, label %block8392, label %block8389

edge59:                                           ; preds = %block8392
  %t76 = inttoptr i64 %v1865 to ptr
  %v1868 = load i64, ptr %t76, align 1
  %v1869 = add i64 %v1868, 1
  store i64 %v1869, ptr %t76, align 1
  br label %block8398

edge64:                                           ; preds = %block8398
  %v1874 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1407)
  %t80.not = icmp eq i64 %v1874, 0
  br i1 %t80.not, label %edge68, label %block8389

edge68:                                           ; preds = %edge47, %edge64, %block8398
  %v1841343357 = phi i64 [ %v1863, %block8398 ], [ %v1863, %edge64 ], [ %v1402, %edge47 ]
  %v1843344356 = phi i64 [ %v1864, %block8398 ], [ %v1864, %edge64 ], [ %v1404, %edge47 ]
  %v1846345355 = phi i64 [ %v1865, %block8398 ], [ %v1865, %edge64 ], [ %v1407, %edge47 ]
  %t75.not275347354 = phi i1 [ %t75.not275, %block8398 ], [ %t75.not275, %edge64 ], [ %t26.not, %edge47 ]
  %t83 = icmp eq i64 %v1397, 0
  br i1 %t83, label %edge70, label %block8548.preheader

block8548.preheader:                              ; preds = %edge68
  %v2382 = ptrtoint ptr %ss1 to i64
  %v219540 = ptrtoint ptr %ss0 to i64
  %t269 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  %t274 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  %t276 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %t277 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %t414 = icmp eq i64 %v1399, 1
  br label %block8548

edge70:                                           ; preds = %edge68
  br i1 %t23.not, label %edge81, label %edge74

edge74:                                           ; preds = %edge70
  %v1889 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t87.not = icmp eq i64 %v1889, 0
  br i1 %t87.not, label %edge81, label %edge75

edge75:                                           ; preds = %edge74
  %t88 = insertvalue { i64, i64 } poison, i64 %v1889, 0
  %t89 = insertvalue { i64, i64 } %t88, i64 19, 1
  br label %common.ret

edge81:                                           ; preds = %edge74, %edge70
  %t90 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v1403, i64 1)
  %v1919 = extractvalue { i64, i1 } %t90, 0
  %t91 = extractvalue { i64, i1 } %t90, 1
  br i1 %t91, label %block8422, label %edge82

edge82:                                           ; preds = %edge81
  %t94.not = icmp ugt i64 %v1919, %v1843344356
  br i1 %t94.not, label %edge85, label %block8426

edge85:                                           ; preds = %edge82
  %t96.not = icmp eq i64 %v1843344356, 0
  br i1 %t96.not, label %block8429, label %edge86

edge86:                                           ; preds = %edge85
  %5 = add i64 %v1843344356, -131073
  %v1929293 = icmp ult i64 %5, -126977
  br i1 %v1929293, label %edge89, label %edge98

edge89:                                           ; preds = %edge86
  %t100 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v1843344356, i64 %v1843344356)
  %v1930 = extractvalue { i64, i1 } %t100, 0
  %t101 = extractvalue { i64, i1 } %t100, 1
  br i1 %t101, label %block8422, label %block8429

edge98:                                           ; preds = %edge86
  %t108 = mul nuw nsw i64 %v1843344356, 3
  %t112 = add nuw nsw i64 %t108, 1
  %v1947 = lshr i64 %t112, 1
  br label %block8429

edge102:                                          ; preds = %block8426
  %t118 = inttoptr i64 %v1846345355 to ptr
  %v1955 = load i64, ptr %t118, align 1
  %t119 = icmp eq i64 %v1955, 1
  %t120 = icmp ule i64 %v1924, %v1843344356
  %v1958294 = and i1 %t120, %t119
  br i1 %v1958294, label %block8436, label %block8437

edge107:                                          ; preds = %block8437
  %v1965 = load i64, ptr %ss1, align 8
  %t137 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %v1966 = load i64, ptr %t137, align 8
  %t138 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %v1967 = load i64, ptr %t138, align 8
  br label %block8436

edge109:                                          ; preds = %block8436
  %t143 = inttoptr i64 %v1954 to ptr
  %v1974 = load i64, ptr %t143, align 1
  %v1975 = add i64 %v1974, 1
  store i64 %v1975, ptr %t143, align 1
  br label %block8441

edge114:                                          ; preds = %block8441
  %v1980 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1846345355)
  %t147.not = icmp eq i64 %v1980, 0
  br i1 %t147.not, label %edge118, label %block8422

edge118:                                          ; preds = %edge114, %block8441
  %t150 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v1919, i64 4)
  %v1989 = extractvalue { i64, i1 } %t150, 0
  %t151 = extractvalue { i64, i1 } %t150, 1
  br i1 %t151, label %block8446, label %edge121

edge121:                                          ; preds = %edge118
  %t154.not = icmp ugt i64 %v1989, %v1951
  br i1 %t154.not, label %edge124, label %block8451

edge124:                                          ; preds = %edge121
  %t156.not = icmp eq i64 %v1951, 0
  br i1 %t156.not, label %block8454, label %edge125

edge125:                                          ; preds = %edge124
  %6 = add i64 %v1951, -131073
  %v1999297 = icmp ult i64 %6, -126977
  br i1 %v1999297, label %edge128, label %edge137

edge128:                                          ; preds = %edge125
  %t160 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v1951, i64 %v1951)
  %v2000 = extractvalue { i64, i1 } %t160, 0
  %t161 = extractvalue { i64, i1 } %t160, 1
  br i1 %t161, label %block8446, label %block8454

edge137:                                          ; preds = %edge125
  %t168 = mul nuw nsw i64 %v1951, 3
  %t172 = add nuw nsw i64 %t168, 1
  %v2017 = lshr i64 %t172, 1
  br label %block8454

edge141:                                          ; preds = %block8451
  %t178 = inttoptr i64 %v1954 to ptr
  %v2025 = load i64, ptr %t178, align 1
  %t179 = icmp eq i64 %v2025, 1
  %t180 = icmp ule i64 %v1994, %v1951
  %v2028298 = and i1 %t180, %t179
  br i1 %v2028298, label %edge148, label %block8462

edge146:                                          ; preds = %block8462
  %v2035 = load i64, ptr %ss1, align 8
  %t197 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %v2036 = load i64, ptr %t197, align 8
  %t198 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %v2037 = load i64, ptr %t198, align 8
  br label %edge148

edge148:                                          ; preds = %edge141, %edge146
  %v2019 = phi i64 [ %v2035, %edge146 ], [ %v1949, %edge141 ]
  %v2021 = phi i64 [ %v2036, %edge146 ], [ %v1951, %edge141 ]
  %v2024 = phi i64 [ %v2037, %edge146 ], [ %v1954, %edge141 ]
  %v2042 = add i64 %v2019, %v1919
  %t199 = inttoptr i64 %v2042 to ptr
  store i8 0, ptr %t199, align 1
  %v2046 = add nuw i64 %v1919, 1
  %v2047 = add i64 %v2046, %v2019
  %t200 = inttoptr i64 %v2047 to ptr
  store i8 0, ptr %t200, align 1
  %v2051 = add nuw i64 %v1919, 2
  %v2052 = add i64 %v2051, %v2019
  %t201 = inttoptr i64 %v2052 to ptr
  store i8 -1, ptr %t201, align 1
  %v2056 = add nuw i64 %v1919, 3
  %v2057 = add i64 %v2056, %v2019
  %t202 = inttoptr i64 %v2057 to ptr
  store i8 -1, ptr %t202, align 1
  %t203 = icmp eq i64 %v1954, %v2024
  %t205.not299 = icmp eq i64 %v2024, 0
  %t205.not = or i1 %t203, %t205.not299
  br i1 %t205.not, label %block8482, label %edge160

edge160:                                          ; preds = %edge148
  %t206 = inttoptr i64 %v2024 to ptr
  %v2080 = load i64, ptr %t206, align 1
  %v2081 = add i64 %v2080, 1
  store i64 %v2081, ptr %t206, align 1
  br label %block8482

edge163:                                          ; preds = %block8482
  %t210 = inttoptr i64 %v1954 to ptr
  %v2084 = load i64, ptr %t210, align 1
  %v2085 = add i64 %v2084, -1
  store i64 %v2085, ptr %t210, align 1
  %t212.not = icmp eq i64 %v2085, 0
  br i1 %t212.not, label %edge166, label %edge16309

edge166:                                          ; preds = %edge163
  %v2086 = call i64 @mech_compact_free(i64 %v0, i64 %v1954)
  %t213.not = icmp eq i64 %v2086, 0
  br i1 %t213.not, label %edge16309, label %block8446

edge218:                                          ; preds = %block8548
  %t233.not = icmp ugt i64 %v2342, %v219177
  br i1 %t233.not, label %edge221, label %block8595

edge221:                                          ; preds = %edge218
  %t235.not = icmp eq i64 %v219177, 0
  br i1 %t235.not, label %block8598, label %edge222

edge222:                                          ; preds = %edge221
  %7 = add i64 %v219177, -131073
  %v2352277 = icmp ult i64 %7, -126977
  br i1 %v2352277, label %edge225, label %edge234

edge225:                                          ; preds = %edge222
  %t239 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v219177, i64 %v219177)
  %v2353 = extractvalue { i64, i1 } %t239, 0
  %t240 = extractvalue { i64, i1 } %t239, 1
  br i1 %t240, label %block8591, label %block8598

edge234:                                          ; preds = %edge222
  %t247 = mul nuw nsw i64 %v219177, 3
  %t251 = add nuw nsw i64 %t247, 1
  %v2370 = lshr i64 %t251, 1
  br label %block8598

edge238:                                          ; preds = %block8595
  %t257 = inttoptr i64 %v219180 to ptr
  %v2378 = load i64, ptr %t257, align 1
  %t258 = icmp eq i64 %v2378, 1
  %t259 = icmp ule i64 %v2347, %v219177
  %v2381278 = and i1 %t259, %t258
  br i1 %v2381278, label %block8605, label %block8606

edge243:                                          ; preds = %block8606
  %v2388 = load i64, ptr %ss1, align 8
  %v2389 = load i64, ptr %t276, align 8
  %v2390 = load i64, ptr %t277, align 8
  br label %block8605

edge245:                                          ; preds = %block8605
  %t282 = inttoptr i64 %v2377 to ptr
  %v2397 = load i64, ptr %t282, align 1
  %v2398 = add i64 %v2397, 1
  store i64 %v2398, ptr %t282, align 1
  br label %block8610

edge248:                                          ; preds = %block8610
  %t286 = inttoptr i64 %v219180 to ptr
  %v2401 = load i64, ptr %t286, align 1
  %v2402 = add i64 %v2401, -1
  store i64 %v2402, ptr %t286, align 1
  %t288.not = icmp eq i64 %v2402, 0
  br i1 %t288.not, label %edge251, label %edge262

edge251:                                          ; preds = %edge248
  %v2403 = call i64 @mech_compact_free(i64 %v0, i64 %v219180)
  %t289.not = icmp eq i64 %v2403, 0
  br i1 %t289.not, label %edge262, label %block8591

edge262:                                          ; preds = %edge251, %edge248, %block8610
  %t292 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2342, i64 2)
  %v2543 = extractvalue { i64, i1 } %t292, 0
  %t293 = extractvalue { i64, i1 } %t292, 1
  br i1 %t293, label %block8665, label %edge265

edge265:                                          ; preds = %edge262
  %t296.not = icmp ugt i64 %v2543, %v2374
  br i1 %t296.not, label %edge268, label %block8670

edge268:                                          ; preds = %edge265
  %t298.not = icmp eq i64 %v2374, 0
  br i1 %t298.not, label %block8673, label %edge269

edge269:                                          ; preds = %edge268
  %8 = add i64 %v2374, -131073
  %v2553281 = icmp ult i64 %8, -126977
  br i1 %v2553281, label %edge272, label %edge281

edge272:                                          ; preds = %edge269
  %t302 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2374, i64 %v2374)
  %v2554 = extractvalue { i64, i1 } %t302, 0
  %t303 = extractvalue { i64, i1 } %t302, 1
  br i1 %t303, label %block8665, label %block8673

edge281:                                          ; preds = %edge269
  %t310 = mul nuw nsw i64 %v2374, 3
  %t314 = add nuw nsw i64 %t310, 1
  %v2571 = lshr i64 %t314, 1
  br label %block8673

edge285:                                          ; preds = %block8670
  %t320 = inttoptr i64 %v2377 to ptr
  %v2579 = load i64, ptr %t320, align 1
  %t321 = icmp eq i64 %v2579, 1
  %t322 = icmp ule i64 %v2548, %v2374
  %v2582282 = and i1 %t322, %t321
  br i1 %v2582282, label %edge292, label %block8681

edge290:                                          ; preds = %block8681
  %v2589 = load i64, ptr %ss1, align 8
  %v2590 = load i64, ptr %t276, align 8
  %v2591 = load i64, ptr %t277, align 8
  br label %edge292

edge292:                                          ; preds = %edge285, %edge290
  %v2573 = phi i64 [ %v2589, %edge290 ], [ %v2372, %edge285 ]
  %v2575 = phi i64 [ %v2590, %edge290 ], [ %v2374, %edge285 ]
  %v2578 = phi i64 [ %v2591, %edge290 ], [ %v2377, %edge285 ]
  %v2594 = trunc i64 %spec.select301 to i8
  %v2596 = add i64 %v2573, %v2342
  %t341 = inttoptr i64 %v2596 to ptr
  store i8 %v2594, ptr %t341, align 1
  %v2598 = lshr i64 %spec.select301, 8
  %v2599 = trunc nuw i64 %v2598 to i8
  %v2600 = add nuw i64 %v2342, 1
  %v2601 = add i64 %v2600, %v2573
  %t342 = inttoptr i64 %v2601 to ptr
  store i8 %v2599, ptr %t342, align 1
  %t343 = icmp eq i64 %v2377, %v2578
  %t345.not283 = icmp eq i64 %v2578, 0
  %t345.not = or i1 %t343, %t345.not283
  br i1 %t345.not, label %block8701, label %edge302

edge302:                                          ; preds = %edge292
  %t346 = inttoptr i64 %v2578 to ptr
  %v2634 = load i64, ptr %t346, align 1
  %v2635 = add i64 %v2634, 1
  store i64 %v2635, ptr %t346, align 1
  br label %block8701

edge307:                                          ; preds = %block8701
  %v2640 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v2377)
  %t350.not = icmp eq i64 %v2640, 0
  br i1 %t350.not, label %edge311, label %block8665

edge311:                                          ; preds = %edge307, %block8701
  %v2647 = xor i64 %spec.select301, -1
  %t353 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2543, i64 2)
  %v2656 = extractvalue { i64, i1 } %t353, 0
  %t354 = extractvalue { i64, i1 } %t353, 1
  br i1 %t354, label %block8707, label %edge315

edge315:                                          ; preds = %edge311
  %t357.not = icmp ugt i64 %v2656, %v2575
  br i1 %t357.not, label %edge318, label %block8712

edge318:                                          ; preds = %edge315
  %t359.not = icmp eq i64 %v2575, 0
  br i1 %t359.not, label %block8715, label %edge319

edge319:                                          ; preds = %edge318
  %9 = add i64 %v2575, -131073
  %v2666285 = icmp ult i64 %9, -126977
  br i1 %v2666285, label %edge322, label %edge331

edge322:                                          ; preds = %edge319
  %t363 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2575, i64 %v2575)
  %v2667 = extractvalue { i64, i1 } %t363, 0
  %t364 = extractvalue { i64, i1 } %t363, 1
  br i1 %t364, label %block8707, label %block8715

edge331:                                          ; preds = %edge319
  %t371 = mul nuw nsw i64 %v2575, 3
  %t375 = add nuw nsw i64 %t371, 1
  %v2684 = lshr i64 %t375, 1
  br label %block8715

edge335:                                          ; preds = %block8712
  %t381 = inttoptr i64 %v2578 to ptr
  %v2692 = load i64, ptr %t381, align 1
  %t382 = icmp eq i64 %v2692, 1
  %t383 = icmp ule i64 %v2661, %v2575
  %v2695286 = and i1 %t383, %t382
  br i1 %v2695286, label %edge342, label %block8723

edge340:                                          ; preds = %block8723
  %v2702 = load i64, ptr %ss1, align 8
  %v2703 = load i64, ptr %t276, align 8
  %v2704 = load i64, ptr %t277, align 8
  br label %edge342

edge342:                                          ; preds = %edge335, %edge340
  %v2686 = phi i64 [ %v2702, %edge340 ], [ %v2573, %edge335 ]
  %v2688 = phi i64 [ %v2703, %edge340 ], [ %v2575, %edge335 ]
  %v2691 = phi i64 [ %v2704, %edge340 ], [ %v2578, %edge335 ]
  %v2707 = trunc i64 %v2647 to i8
  %v2709 = add i64 %v2686, %v2543
  %t402 = inttoptr i64 %v2709 to ptr
  store i8 %v2707, ptr %t402, align 1
  %v2711 = lshr i64 %v2647, 8
  %v2712 = trunc i64 %v2711 to i8
  %v2713 = add nuw i64 %v2543, 1
  %v2714 = add i64 %v2713, %v2686
  %t403 = inttoptr i64 %v2714 to ptr
  store i8 %v2712, ptr %t403, align 1
  %t404 = icmp eq i64 %v2578, %v2691
  %t406.not287 = icmp eq i64 %v2691, 0
  %t406.not = or i1 %t404, %t406.not287
  br i1 %t406.not, label %block8743, label %edge352

edge352:                                          ; preds = %edge342
  %t407 = inttoptr i64 %v2691 to ptr
  %v2747 = load i64, ptr %t407, align 1
  %v2748 = add i64 %v2747, 1
  store i64 %v2748, ptr %t407, align 1
  br label %block8743

edge357:                                          ; preds = %block8743
  %v2753 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v2578)
  %t411.not = icmp eq i64 %v2753, 0
  br i1 %t411.not, label %edge361, label %block8707

edge361:                                          ; preds = %edge357, %block8743
  br i1 %t414, label %edge363, label %block8748

edge363:                                          ; preds = %edge361
  %t416 = call i64 @llvm.umin.i64(i64 %v219181, i64 %v1397)
  %v2763 = sub nsw i64 %v1397, %t416
  %t417 = call i64 @llvm.umin.i64(i64 %spec.select301, i64 %v2763)
  %t418 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2656, i64 %t417)
  %v2765 = extractvalue { i64, i1 } %t418, 0
  %t419 = extractvalue { i64, i1 } %t418, 1
  br i1 %t419, label %block8748, label %edge365

edge365:                                          ; preds = %edge363
  %t422.not = icmp ugt i64 %v2765, %v2688
  br i1 %t422.not, label %edge368, label %block8753

edge368:                                          ; preds = %edge365
  %t424.not = icmp eq i64 %v2688, 0
  br i1 %t424.not, label %block8756, label %edge369

edge369:                                          ; preds = %edge368
  %10 = add i64 %v2688, -131073
  %v2775289 = icmp ult i64 %10, -126977
  br i1 %v2775289, label %edge372, label %edge381

edge372:                                          ; preds = %edge369
  %t428 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2688, i64 %v2688)
  %v2776 = extractvalue { i64, i1 } %t428, 0
  %t429 = extractvalue { i64, i1 } %t428, 1
  br i1 %t429, label %block8748, label %block8756

edge381:                                          ; preds = %edge369
  %t436 = mul nuw nsw i64 %v2688, 3
  %t440 = add nuw nsw i64 %t436, 1
  %v2793 = lshr i64 %t440, 1
  br label %block8756

edge385:                                          ; preds = %block8753
  %t446 = inttoptr i64 %v2691 to ptr
  %v2801 = load i64, ptr %t446, align 1
  %t447 = icmp eq i64 %v2801, 1
  %t448 = icmp ule i64 %v2770, %v2688
  %v2804290 = and i1 %t448, %t447
  br i1 %v2804290, label %block8763, label %block8764

edge390:                                          ; preds = %block8764
  %v2811 = load i64, ptr %ss1, align 8
  %v2812 = load i64, ptr %t276, align 8
  %v2813 = load i64, ptr %t277, align 8
  br label %block8763

edge392:                                          ; preds = %block8763
  %v2817 = add i64 %t416, %v1396
  %v2815 = add i64 %v2795, %v2656
  %v2819 = call i64 @memmove(i64 %v2815, i64 %v2817, i64 %t417)
  br label %block8768

edge395:                                          ; preds = %block8768
  %t471 = inttoptr i64 %v2800 to ptr
  %v2822 = load i64, ptr %t471, align 1
  %v2823 = add i64 %v2822, 1
  store i64 %v2823, ptr %t471, align 1
  br label %block8770

edge400:                                          ; preds = %block8770
  %v2828 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v2691)
  %t475.not = icmp eq i64 %v2828, 0
  br i1 %t475.not, label %edge404, label %block8748

edge404:                                          ; preds = %edge400, %block8770
  %v2833 = add nuw nsw i64 %spec.select301, %v219181
  %t482 = icmp eq i64 %v2833, %v1397
  br i1 %t482, label %edge409, label %edge432

edge409:                                          ; preds = %edge404
  br i1 %t23.not, label %edge16309, label %edge411

edge411:                                          ; preds = %edge409
  %t486 = inttoptr i64 %v1401 to ptr
  %v2848 = load i64, ptr %t486, align 1
  %v2849 = add i64 %v2848, -1
  store i64 %v2849, ptr %t486, align 1
  %t488.not = icmp eq i64 %v2849, 0
  br i1 %t488.not, label %edge414, label %edge16309

edge414:                                          ; preds = %edge411
  %v2850 = call i64 @mech_compact_free(i64 %v0, i64 %v1401)
  %t489.not = icmp eq i64 %v2850, 0
  br i1 %t489.not, label %edge16309, label %edge415

edge415:                                          ; preds = %edge414
  %t490 = insertvalue { i64, i64 } poison, i64 %v2850, 0
  %t491 = insertvalue { i64, i64 } %t490, i64 86, 1
  br label %common.ret

edge432:                                          ; preds = %edge404
  %t216.not = icmp ult i64 %v1397, %v2833
  br i1 %t216.not, label %common.ret, label %block8548

edge462:                                          ; preds = %edge12
  %t523.not = icmp ugt i64 %v1397, -5000
  br i1 %t523.not, label %common.ret, label %edge477

edge477:                                          ; preds = %edge462
  %v3010 = add nuw i64 %v1397, 4999
  %v3019 = udiv i64 %v3010, 5000
  %11 = mul nuw nsw i64 %v3019, 5
  %v3164 = add i64 %11, %v1397
  %t540.not = icmp ult i64 %v3164, %11
  br i1 %t540.not, label %common.ret, label %edge482

edge482:                                          ; preds = %edge477
  %t544 = icmp eq i64 %v1405, 1
  br i1 %t544, label %edge483, label %block8962

edge483:                                          ; preds = %edge482
  %v3176 = tail call i64 @llvm.uadd.sat.i64(i64 %v1403, i64 %v3164)
  %t549 = icmp ugt i64 %v3176, %v1404
  %v3194 = ptrtoint ptr %ss1 to i64
  br i1 %t549, label %edge485, label %edge534

edge485:                                          ; preds = %edge483
  %v219570 = ptrtoint ptr %ss0 to i64
  %t556 = zext i64 %v1402 to i128
  %t557 = zext i64 %v1403 to i128
  %t558 = shl nuw i128 %t557, 64
  %v219571 = or disjoint i128 %t558, %t556
  store i128 %v219571, ptr %ss0, align 16
  %t560 = zext i64 %v1404 to i128
  %v219573 = or disjoint i128 %t560, 18446744073709551616
  %t564 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  store i128 %v219573, ptr %t564, align 16
  %t565 = zext i64 %v1406 to i128
  %t566 = zext i64 %v1407 to i128
  %t567 = shl nuw i128 %t566, 64
  %v219575 = or disjoint i128 %t567, %t565
  %t569 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  store i128 %v219575, ptr %t569, align 16
  %v3199 = call i64 @mech_compact_writable(i64 %v0, i64 %v219570, i64 1, i64 %v3176, i64 1, i64 1, i64 %v3194)
  %t570.not = icmp eq i64 %v3199, 0
  br i1 %t570.not, label %block8965, label %block8962

edge495:                                          ; preds = %block8965
  %t576 = inttoptr i64 %v3202 to ptr
  %v3205 = load i64, ptr %t576, align 1
  %v3206 = add i64 %v3205, 1
  store i64 %v3206, ptr %t576, align 1
  br label %block8971

edge498:                                          ; preds = %block8971
  %t580 = inttoptr i64 %v1407 to ptr
  %v3209 = load i64, ptr %t580, align 1
  %v3210 = add i64 %v3209, -1
  store i64 %v3210, ptr %t580, align 1
  %t582.not = icmp eq i64 %v3210, 0
  br i1 %t582.not, label %edge501, label %edge534

edge501:                                          ; preds = %edge498
  %v3211 = call i64 @mech_compact_free(i64 %v0, i64 %v1407)
  %t583.not = icmp eq i64 %v3211, 0
  br i1 %t583.not, label %edge534, label %block8962

edge514:                                          ; preds = %edge537
  %t591 = insertvalue { i64, i64 } poison, i64 %v3228, 0
  %t592 = insertvalue { i64, i64 } %t591, i64 120, 1
  br label %common.ret

edge517:                                          ; preds = %edge536, %edge537
  %spec.select6 = call i64 @llvm.umin.i64(i64 %v1397, i64 6000)
  %v3544 = mul nuw nsw i64 %spec.select6, 257
  %v3720 = call i64 @llvm.umin.i64(i64 %v3544, i64 1500513)
  %v3760 = call i64 @mech_compact_allocate(i64 %v0, i64 %v3720, i64 4, i64 4, i64 1, i64 %v3194)
  %t638.not = icmp eq i64 %v3760, 0
  br i1 %t638.not, label %edge592, label %block9231

edge534:                                          ; preds = %edge483, %block8971, %edge498, %edge501
  %v3178364375 = phi i64 [ %v3200, %edge501 ], [ %v3200, %block8971 ], [ %v3200, %edge498 ], [ %v1402, %edge483 ]
  %v3180365374 = phi i64 [ %v3201, %edge501 ], [ %v3201, %block8971 ], [ %v3201, %edge498 ], [ %v1404, %edge483 ]
  %v3183366373 = phi i64 [ %v3202, %edge501 ], [ %v3202, %block8971 ], [ %v3202, %edge498 ], [ %v1407, %edge483 ]
  %v3276 = call i64 @mech_compact_allocate(i64 %v0, i64 262144, i64 2, i64 2, i64 1, i64 %v3194)
  %t605.not = icmp eq i64 %v3276, 0
  br i1 %t605.not, label %edge536, label %block8995

edge536:                                          ; preds = %edge534
  %v3279 = load i64, ptr %ss1, align 8
  %t606 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %v3280 = load i64, ptr %t606, align 8
  %t607 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %v3281 = load i64, ptr %t607, align 8
  %cond330 = icmp eq i64 %v3281, 0
  br i1 %cond330, label %edge517, label %edge537

edge537:                                          ; preds = %edge536
  %t609 = inttoptr i64 %v3281 to ptr
  %v3282 = load i64, ptr %t609, align 1
  %v3224 = add i64 %v3282, 2
  store i64 %v3224, ptr %t609, align 1
  %v3228 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v3281)
  %t590.not = icmp eq i64 %v3228, 0
  br i1 %t590.not, label %edge517, label %edge514

edge570:                                          ; preds = %edge593
  %t624 = insertvalue { i64, i64 } poison, i64 %v3711, 0
  %t625 = insertvalue { i64, i64 } %t624, i64 152, 1
  br label %common.ret

edge573:                                          ; preds = %edge592, %edge593
  %spec.select8 = call i64 @llvm.umin.i64(i64 %v1397, i64 304999)
  %v4039 = add nuw nsw i64 %spec.select8, 258
  %v4048 = call i64 @mech_compact_allocate(i64 %v0, i64 %v4039, i64 8, i64 8, i64 0, i64 %v3194)
  %t649.not = icmp eq i64 %v4048, 0
  br i1 %t649.not, label %edge610, label %edge609

edge592:                                          ; preds = %edge517
  %v3763 = load i64, ptr %ss1, align 8
  %v3764 = load i64, ptr %t606, align 8
  %v3765 = load i64, ptr %t607, align 8
  %cond331 = icmp eq i64 %v3765, 0
  br i1 %cond331, label %edge573, label %edge593

edge593:                                          ; preds = %edge592
  %t642 = inttoptr i64 %v3765 to ptr
  %v3766 = load i64, ptr %t642, align 1
  %v3707 = add i64 %v3766, 2
  store i64 %v3707, ptr %t642, align 1
  %v3711 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v3765)
  %t623.not = icmp eq i64 %v3711, 0
  br i1 %t623.not, label %edge573, label %edge570

edge609:                                          ; preds = %edge573
  %t655 = insertvalue { i64, i64 } poison, i64 %v4048, 0
  %t656 = insertvalue { i64, i64 } %t655, i64 174, 1
  br label %common.ret

edge610:                                          ; preds = %edge573
  %v4051 = load i64, ptr %ss1, align 8
  %v4052 = load i64, ptr %t606, align 8
  %v4053 = load i64, ptr %t607, align 8
  %t652.not = icmp eq i64 %v4053, 0
  br i1 %t652.not, label %edge642, label %edge611

edge611:                                          ; preds = %edge610
  %t653 = inttoptr i64 %v4053 to ptr
  %v4054 = load i64, ptr %t653, align 1
  %v4055 = add i64 %v4054, 1
  store i64 %v4055, ptr %t653, align 1
  br label %edge642

edge621:                                          ; preds = %edge645
  %v4096 = call i64 @mech_compact_free(i64 %v0, i64 %v4153)
  %t662.not = icmp eq i64 %v4096, 0
  br i1 %t662.not, label %edge678, label %edge622

edge622:                                          ; preds = %edge621
  %t663 = insertvalue { i64, i64 } poison, i64 %v4096, 0
  %t664 = insertvalue { i64, i64 } %t663, i64 176, 1
  br label %common.ret

edge642:                                          ; preds = %edge611, %edge610
  %v4148 = call i64 @mech_compact_allocate(i64 %v0, i64 547, i64 4, i64 4, i64 1, i64 %v3194)
  %t677.not = icmp eq i64 %v4148, 0
  br i1 %t677.not, label %edge644, label %block9385

edge644:                                          ; preds = %edge642
  %v4151 = load i64, ptr %ss1, align 8
  %v4152 = load i64, ptr %t606, align 8
  %v4153 = load i64, ptr %t607, align 8
  %t680.not = icmp eq i64 %v4153, 0
  br i1 %t680.not, label %edge678, label %edge645

edge645:                                          ; preds = %edge644
  %t681 = inttoptr i64 %v4153 to ptr
  %v4154 = load i64, ptr %t681, align 1
  %v4155 = add i64 %v4154, 1
  store i64 %v4155, ptr %t681, align 1
  %t661.not = icmp eq i64 %v4155, 0
  br i1 %t661.not, label %edge621, label %edge678

edge657:                                          ; preds = %edge681
  %v4319 = call i64 @mech_compact_free(i64 %v0, i64 %v4372)
  %t690.not = icmp eq i64 %v4319, 0
  br i1 %t690.not, label %edge714, label %edge658

edge658:                                          ; preds = %edge657
  %t691 = insertvalue { i64, i64 } poison, i64 %v4319, 0
  %t692 = insertvalue { i64, i64 } %t691, i64 189, 1
  br label %common.ret

edge678:                                          ; preds = %edge644, %edge645, %edge621
  %v4367 = call i64 @mech_compact_allocate(i64 %v0, i64 547, i64 4, i64 4, i64 1, i64 %v3194)
  %t705.not = icmp eq i64 %v4367, 0
  br i1 %t705.not, label %edge680, label %block9440

edge680:                                          ; preds = %edge678
  %v4370 = load i64, ptr %ss1, align 8
  %v4371 = load i64, ptr %t606, align 8
  %v4372 = load i64, ptr %t607, align 8
  %t708.not = icmp eq i64 %v4372, 0
  br i1 %t708.not, label %edge714, label %edge681

edge681:                                          ; preds = %edge680
  %t709 = inttoptr i64 %v4372 to ptr
  %v4373 = load i64, ptr %t709, align 1
  %v4374 = add i64 %v4373, 1
  store i64 %v4374, ptr %t709, align 1
  %t689.not = icmp eq i64 %v4374, 0
  br i1 %t689.not, label %edge657, label %edge714

edge693:                                          ; preds = %edge717
  %v4550 = call i64 @mech_compact_free(i64 %v0, i64 %v4611)
  %t718.not = icmp eq i64 %v4550, 0
  br i1 %t718.not, label %edge750, label %edge694

edge694:                                          ; preds = %edge693
  %t719 = insertvalue { i64, i64 } poison, i64 %v4550, 0
  %t720 = insertvalue { i64, i64 } %t719, i64 202, 1
  br label %common.ret

edge714:                                          ; preds = %edge680, %edge681, %edge657
  %v4606 = call i64 @mech_compact_allocate(i64 %v0, i64 259, i64 4, i64 4, i64 1, i64 %v3194)
  %t733.not = icmp eq i64 %v4606, 0
  br i1 %t733.not, label %edge716, label %block9495

edge716:                                          ; preds = %edge714
  %v4609 = load i64, ptr %ss1, align 8
  %v4610 = load i64, ptr %t606, align 8
  %v4611 = load i64, ptr %t607, align 8
  %t736.not = icmp eq i64 %v4611, 0
  br i1 %t736.not, label %edge750, label %edge717

edge717:                                          ; preds = %edge716
  %t737 = inttoptr i64 %v4611 to ptr
  %v4612 = load i64, ptr %t737, align 1
  %v4613 = add i64 %v4612, 1
  store i64 %v4613, ptr %t737, align 1
  %t717.not = icmp eq i64 %v4613, 0
  br i1 %t717.not, label %edge693, label %edge750

edge729:                                          ; preds = %edge753
  %v4801 = call i64 @mech_compact_free(i64 %v0, i64 %v4854)
  %t746.not = icmp eq i64 %v4801, 0
  br i1 %t746.not, label %edge786, label %edge730

edge730:                                          ; preds = %edge729
  %t747 = insertvalue { i64, i64 } poison, i64 %v4801, 0
  %t748 = insertvalue { i64, i64 } %t747, i64 215, 1
  br label %common.ret

edge750:                                          ; preds = %edge716, %edge717, %edge693
  %v4849 = call i64 @mech_compact_allocate(i64 %v0, i64 259, i64 4, i64 4, i64 1, i64 %v3194)
  %t761.not = icmp eq i64 %v4849, 0
  br i1 %t761.not, label %edge752, label %block9550

edge752:                                          ; preds = %edge750
  %v4852 = load i64, ptr %ss1, align 8
  %v4853 = load i64, ptr %t606, align 8
  %v4854 = load i64, ptr %t607, align 8
  %t764.not = icmp eq i64 %v4854, 0
  br i1 %t764.not, label %edge786, label %edge753

edge753:                                          ; preds = %edge752
  %t765 = inttoptr i64 %v4854 to ptr
  %v4855 = load i64, ptr %t765, align 1
  %v4856 = add i64 %v4855, 1
  store i64 %v4856, ptr %t765, align 1
  %t745.not = icmp eq i64 %v4856, 0
  br i1 %t745.not, label %edge729, label %edge786

edge765:                                          ; preds = %edge789
  %v5056 = call i64 @mech_compact_free(i64 %v0, i64 %v5121)
  %t774.not = icmp eq i64 %v5056, 0
  br i1 %t774.not, label %edge822, label %edge766

edge766:                                          ; preds = %edge765
  %t775 = insertvalue { i64, i64 } poison, i64 %v5056, 0
  %t776 = insertvalue { i64, i64 } %t775, i64 228, 1
  br label %common.ret

edge786:                                          ; preds = %edge752, %edge753, %edge729
  %v5116 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3194)
  %t789.not = icmp eq i64 %v5116, 0
  br i1 %t789.not, label %edge788, label %block9605

edge788:                                          ; preds = %edge786
  %v5119 = load i64, ptr %ss1, align 8
  %v5121 = load i64, ptr %t607, align 8
  %t791.not = icmp eq i64 %v5121, 0
  br i1 %t791.not, label %edge822, label %edge789

edge789:                                          ; preds = %edge788
  %t792 = inttoptr i64 %v5121 to ptr
  %v5122 = load i64, ptr %t792, align 1
  %v5123 = add i64 %v5122, 1
  store i64 %v5123, ptr %t792, align 1
  %t773.not = icmp eq i64 %v5123, 0
  br i1 %t773.not, label %edge765, label %edge822

edge801:                                          ; preds = %edge825
  %v5335 = call i64 @mech_compact_free(i64 %v0, i64 %v5388)
  %t801.not = icmp eq i64 %v5335, 0
  br i1 %t801.not, label %edge859, label %edge802

edge802:                                          ; preds = %edge801
  %t802 = insertvalue { i64, i64 } poison, i64 %v5335, 0
  %t803 = insertvalue { i64, i64 } %t802, i64 241, 1
  br label %common.ret

edge822:                                          ; preds = %edge788, %edge789, %edge765
  %v5383 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3194)
  %t816.not = icmp eq i64 %v5383, 0
  br i1 %t816.not, label %edge824, label %block9660

edge824:                                          ; preds = %edge822
  %v5386 = load i64, ptr %ss1, align 8
  %v5387 = load i64, ptr %t606, align 8
  %v5388 = load i64, ptr %t607, align 8
  %t819.not = icmp eq i64 %v5388, 0
  br i1 %t819.not, label %edge859, label %edge825

edge825:                                          ; preds = %edge824
  %t820 = inttoptr i64 %v5388 to ptr
  %v5389 = load i64, ptr %t820, align 1
  %v5390 = add i64 %v5389, 1
  store i64 %v5390, ptr %t820, align 1
  %t800.not = icmp eq i64 %v5390, 0
  br i1 %t800.not, label %edge801, label %edge859

edge839:                                          ; preds = %edge862
  %t829 = insertvalue { i64, i64 } poison, i64 %v5614, 0
  %t830 = insertvalue { i64, i64 } %t829, i64 254, 1
  br label %common.ret

edge842:                                          ; preds = %edge861, %edge862
  %spec.select10 = call i64 @llvm.umin.i64(i64 %v1397, i64 32768)
  %t910 = inttoptr i64 %v1401 to ptr
  %t972 = icmp eq i64 %v1399, 1
  %t973 = icmp eq i64 %v1400, 1
  %v894746 = or i1 %t972, %t973
  %t975 = icmp eq i64 %v1399, 0
  %v219588 = ptrtoint ptr %ss0 to i64
  %t1002 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  %t1007 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  %v219576 = ptrtoint ptr %ss2 to i64
  %t926 = getelementptr inbounds nuw i8, ptr %ss2, i64 16
  %t931 = getelementptr inbounds nuw i8, ptr %ss2, i64 32
  %t933 = zext i64 %v1396 to i128
  %t934 = shl nuw i128 %t933, 64
  %v219583 = or disjoint i128 %t934, 256
  %t936 = getelementptr inbounds nuw i8, ptr %ss2, i64 48
  %t937 = zext i64 %v1397 to i128
  %t938 = zext i64 %v1399 to i128
  %t939 = shl nuw i128 %t938, 64
  %v219585 = or disjoint i128 %t939, %t937
  %t941 = getelementptr inbounds nuw i8, ptr %ss2, i64 64
  %t942 = zext i64 %v1400 to i128
  %t946 = getelementptr inbounds nuw i8, ptr %ss2, i64 80
  %t947 = getelementptr inbounds nuw i8, ptr %ss2, i64 96
  %v8481 = ptrtoint ptr %ss3 to i64
  %t1068 = icmp ne i64 %v1399, 1
  %t1069 = icmp ne i64 %v1400, 1
  %v1062051.not1294 = and i1 %t1068, %t1069
  %t1152 = getelementptr inbounds nuw i8, ptr %ss2, i64 112
  %t1157 = getelementptr inbounds nuw i8, ptr %ss2, i64 128
  %t1162 = getelementptr inbounds nuw i8, ptr %ss2, i64 144
  %t1163 = zext nneg i64 %v3720 to i128
  %v219615 = or disjoint i128 %t1163, 18446744073709551616
  %t1167 = getelementptr inbounds nuw i8, ptr %ss2, i64 160
  %v219617 = or disjoint i128 %t934, %t1163
  %t1172 = getelementptr inbounds nuw i8, ptr %ss2, i64 176
  %t1177 = getelementptr inbounds nuw i8, ptr %ss2, i64 192
  %t1182 = getelementptr inbounds nuw i8, ptr %ss2, i64 208
  %t1187 = getelementptr inbounds nuw i8, ptr %ss2, i64 224
  %t1192 = getelementptr inbounds nuw i8, ptr %ss2, i64 240
  %t1197 = getelementptr inbounds nuw i8, ptr %ss2, i64 256
  %t1202 = getelementptr inbounds nuw i8, ptr %ss2, i64 272
  %t1207 = getelementptr inbounds nuw i8, ptr %ss2, i64 288
  %t1212 = getelementptr inbounds nuw i8, ptr %ss2, i64 304
  %t1217 = getelementptr inbounds nuw i8, ptr %ss2, i64 320
  %t1222 = getelementptr inbounds nuw i8, ptr %ss2, i64 336
  %t1227 = getelementptr inbounds nuw i8, ptr %ss2, i64 352
  %t1232 = getelementptr inbounds nuw i8, ptr %ss2, i64 368
  %t1237 = getelementptr inbounds nuw i8, ptr %ss2, i64 384
  %t1242 = getelementptr inbounds nuw i8, ptr %ss2, i64 400
  %t1247 = getelementptr inbounds nuw i8, ptr %ss2, i64 416
  %t1248 = getelementptr inbounds nuw i8, ptr %ss2, i64 432
  %t1255 = getelementptr inbounds nuw i8, ptr %ss3, i64 8
  %t1258 = getelementptr inbounds nuw i8, ptr %ss3, i64 24
  %t1261 = getelementptr inbounds nuw i8, ptr %ss3, i64 40
  %t1264 = getelementptr inbounds nuw i8, ptr %ss3, i64 56
  %t1267 = getelementptr inbounds nuw i8, ptr %ss3, i64 72
  %t1270 = getelementptr inbounds nuw i8, ptr %ss3, i64 88
  %t1273 = getelementptr inbounds nuw i8, ptr %ss3, i64 104
  %t1275 = getelementptr inbounds nuw i8, ptr %ss3, i64 120
  %v219700 = ptrtoint ptr %ss4 to i64
  %t1383 = getelementptr inbounds nuw i8, ptr %ss4, i64 16
  %v14025 = add i64 %v1397, -4
  %t1719 = shl nuw nsw i128 %t1163, 64
  %v23159 = add i64 %v1397, -8
  %t1326 = getelementptr inbounds nuw i8, ptr %ss3, i64 248
  %t1329 = getelementptr inbounds nuw i8, ptr %ss3, i64 264
  %t1332 = getelementptr inbounds nuw i8, ptr %ss3, i64 280
  %t1335 = getelementptr inbounds nuw i8, ptr %ss3, i64 296
  %t1337 = getelementptr inbounds nuw i8, ptr %ss3, i64 320
  %v219929 = or disjoint i128 %t1719, 1
  %t4193 = zext i64 ptrtoint (ptr @mech_compact_table_18 to i64) to i128
  %v219935 = or disjoint i128 %t4193, 4722366482869645213696
  %t4225 = getelementptr inbounds nuw i8, ptr %ss3, i64 48
  %t4228 = getelementptr inbounds nuw i8, ptr %ss3, i64 64
  %t4231 = getelementptr inbounds nuw i8, ptr %ss3, i64 80
  %t4516 = zext i64 ptrtoint (ptr @mech_compact_table_7 to i64) to i128
  %t4517 = shl nuw i128 %t4516, 64
  %v219969 = or disjoint i128 %t4517, 320
  %t4527 = shl nuw i128 %t4193, 64
  %v219973 = or disjoint i128 %t4527, 259
  %t6368 = zext i64 ptrtoint (ptr @mech_compact_table_19 to i64) to i128
  %v220179 = or disjoint i128 %t6368, 4722366482869645213696
  %t7248 = zext i64 ptrtoint (ptr @mech_compact_table_11 to i64) to i128
  %t7249 = shl nuw i128 %t7248, 64
  %v220245 = or disjoint i128 %t7248, 10090369008319124733952
  br label %block10120

edge859:                                          ; preds = %edge824, %edge825, %edge801
  %v5662 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3194)
  %t843.not = icmp eq i64 %v5662, 0
  br i1 %t843.not, label %edge861, label %block9715

edge861:                                          ; preds = %edge859
  %v5665 = load i64, ptr %ss1, align 8
  %v5666 = load i64, ptr %t606, align 8
  %v5667 = load i64, ptr %t607, align 8
  %cond332 = icmp eq i64 %v5667, 0
  br i1 %cond332, label %edge842, label %edge862

edge862:                                          ; preds = %edge861
  %t847 = inttoptr i64 %v5667 to ptr
  %v5668 = load i64, ptr %t847, align 1
  %v5610 = add i64 %v5668, 2
  store i64 %v5610, ptr %t847, align 1
  %v5614 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v5667)
  %t828.not = icmp eq i64 %v5614, 0
  br i1 %t828.not, label %edge842, label %edge839

edge936:                                          ; preds = %edge940, %block10120
  %v7671 = phi i64 [ %v1397, %block10120 ], [ %v7457, %edge940 ]
  br i1 %v219253, label %edge948, label %edge952

edge940:                                          ; preds = %block10120
  %v7457 = add nuw i64 %v219217, 300000
  %t861.not = icmp ugt i64 %v219217, -300001
  br i1 %t861.not, label %common.ret, label %edge936

edge948:                                          ; preds = %edge953, %edge1200, %edge1203, %edge936
  %v10552 = phi i64 [ 259, %edge936 ], [ 3, %edge953 ], [ %v10209, %edge1203 ], [ 3, %edge1200 ]
  br i1 %t23.not, label %block475.outer.preheader, label %edge1217

edge952:                                          ; preds = %edge936
  %t868.not = icmp ult i64 %v7671, %v219217
  br i1 %t868.not, label %common.ret, label %edge953

edge953:                                          ; preds = %edge952
  %v7674 = sub nuw i64 %v7671, %v219217
  %t872 = icmp ult i64 %v7674, 512
  br i1 %t872, label %edge948, label %edge997

edge977:                                          ; preds = %edge1000
  %t884 = insertvalue { i64, i64 } poison, i64 %v8022, 0
  %t885 = insertvalue { i64, i64 } %t884, i64 339, 1
  br label %common.ret

edge980:                                          ; preds = %edge999, %edge1000
  %spec.select12 = call i64 @llvm.umin.i64(i64 %v7674, i64 4096)
  br i1 %t23.not, label %block364.preheader, label %edge1015

edge997:                                          ; preds = %edge953
  %v8090 = call i64 @mech_compact_allocate(i64 %v0, i64 256, i64 1, i64 1, i64 1, i64 %v3194)
  %t898.not = icmp eq i64 %v8090, 0
  br i1 %t898.not, label %edge999, label %block10520

edge999:                                          ; preds = %edge997
  %v8093 = load i64, ptr %ss1, align 8
  %v8094 = load i64, ptr %t606, align 8
  %v8095 = load i64, ptr %t607, align 8
  %t901.not = icmp eq i64 %v8095, 0
  br i1 %t901.not, label %edge980, label %edge1000

edge1000:                                         ; preds = %edge999
  %t902 = inttoptr i64 %v8095 to ptr
  %v8096 = load i64, ptr %t902, align 1
  %v8018 = add i64 %v8096, 2
  store i64 %v8018, ptr %t902, align 1
  %v8022 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v8095)
  %t883.not = icmp eq i64 %v8022, 0
  br i1 %t883.not, label %edge980, label %edge977

edge1015:                                         ; preds = %edge980
  %v8456 = load i64, ptr %t910, align 1
  %v8457 = add i64 %v8456, 1
  store i64 %v8457, ptr %t910, align 1
  br label %block364.preheader

block364.preheader:                               ; preds = %edge980, %edge1015
  br label %block364

edge1023:                                         ; preds = %block364
  %t913 = inttoptr i64 %v8476 to ptr
  %v8477 = load i64, ptr %t913, align 1
  %t914 = icmp eq i64 %v8477, 1
  %t916 = icmp ugt i64 %v8510, 255
  %or.cond303 = select i1 %t914, i1 %t916, i1 false
  br i1 %or.cond303, label %edge1027, label %edge1057

edge1027:                                         ; preds = %edge1023
  %t918 = zext i64 %v219217 to i128
  %t919 = zext nneg i64 %spec.select12 to i128
  %t920 = shl nuw nsw i128 %t919, 64
  %v219577 = or disjoint i128 %t920, %t918
  store i128 %v219577, ptr %ss2, align 16
  %t922 = zext nneg i64 %v8513 to i128
  %t923 = zext i64 %v8514 to i128
  %t924 = shl nuw i128 %t923, 64
  %v219579 = or disjoint i128 %t924, %t922
  store i128 %v219579, ptr %t926, align 16
  store i128 18446744073709551872, ptr %t931, align 16
  store i128 %v219583, ptr %t936, align 16
  store i128 %v219585, ptr %t941, align 16
  %t943 = zext i64 %v8476 to i128
  %t944 = shl nuw i128 %t943, 64
  %v219587 = or disjoint i128 %t944, %t942
  store i128 %v219587, ptr %t946, align 16
  store i64 %v8510, ptr %t947, align 16
  br i1 %v894746, label %edge1030, label %edge1031

edge1030:                                         ; preds = %edge1027
  %t951 = call fastcc { i64, i64 } @mech_compact_numeric_362_vector(i64 %v219576, i64 %v8481)
  br label %block10708

edge1031:                                         ; preds = %edge1027
  %t952 = call fastcc { i64, i64 } @mech_compact_numeric_362(i64 %v219576, i64 %v8481)
  br label %block10708

edge1034:                                         ; preds = %block10708
  %v8483 = extractvalue { i64, i64 } %t951.pn, 1
  %t954 = icmp eq i64 %v8482, 103169
  %v8496 = select i1 %t954, i64 103425, i64 %v8482
  %t956 = insertvalue { i64, i64 } poison, i64 %v8496, 0
  %t957 = insertvalue { i64, i64 } %t956, i64 %v8483, 1
  br label %common.ret

edge1036:                                         ; preds = %block10708
  br i1 %t23.not, label %edge1147, label %edge1039

edge1039:                                         ; preds = %edge1036
  %v8505 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t959.not = icmp eq i64 %v8505, 0
  br i1 %t959.not, label %edge1147, label %edge1040

edge1040:                                         ; preds = %edge1039
  %t960 = insertvalue { i64, i64 } poison, i64 %v8505, 0
  %t961 = insertvalue { i64, i64 } %t960, i64 362, 1
  br label %common.ret

edge1057:                                         ; preds = %block364, %edge1023
  %exitcond.not = icmp eq i64 %v8513, %spec.select12
  br i1 %exitcond.not, label %edge1117, label %edge1063

edge1063:                                         ; preds = %edge1057
  %v8747 = add i64 %v8513, %v219217
  %t965.not = icmp ult i64 %v8747, %v219217
  br i1 %t965.not, label %common.ret, label %edge1067

edge1067:                                         ; preds = %edge1063
  %t969 = icmp ult i64 %v8747, %v1397
  br i1 %t969, label %edge1073, label %edge1088

edge1073:                                         ; preds = %edge1067
  br i1 %v894746, label %block11014, label %edge1078

edge1078:                                         ; preds = %edge1073
  br i1 %t975, label %trap976, label %ok976

edge1088:                                         ; preds = %edge1067, %block11014
  %v8979 = phi i64 [ %v8956, %block11014 ], [ 0, %edge1067 ]
  %v8991 = add i64 %v8979, %v8514
  %t985 = inttoptr i64 %v8991 to ptr
  %t986 = load i8, ptr %t985, align 1
  %t987 = icmp eq i8 %t986, 1
  br i1 %t987, label %block11025, label %edge1091

edge1091:                                         ; preds = %edge1088
  br i1 %t912.not, label %block11027, label %edge1092

edge1092:                                         ; preds = %edge1091
  %t990 = inttoptr i64 %v8476 to ptr
  %v9008 = load i64, ptr %t990, align 1
  %t991 = icmp eq i64 %v9008, 1
  br i1 %t991, label %block11026, label %block11027

edge1097:                                         ; preds = %block11027
  %v9018 = load i64, ptr %ss1, align 8
  %v9019 = load i64, ptr %t606, align 8
  %v9020 = load i64, ptr %t607, align 8
  %.pre21805 = add i64 %v9018, %v8979
  %.pre21806 = inttoptr i64 %.pre21805 to ptr
  br label %block11026

edge1100:                                         ; preds = %block11025
  %t1015 = inttoptr i64 %v9001 to ptr
  %v9025 = load i64, ptr %t1015, align 1
  %v9026 = add i64 %v9025, 1
  store i64 %v9026, ptr %t1015, align 1
  br label %block11031

edge1105:                                         ; preds = %block11031
  %v9031 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v8476)
  %t1019.not = icmp eq i64 %v9031, 0
  br i1 %t1019.not, label %edge1128, label %block11022

edge1117:                                         ; preds = %edge1057
  br i1 %t23.not, label %edge1147, label %edge1118

edge1118:                                         ; preds = %edge1117
  %v9087 = load i64, ptr %t910, align 1
  %v9088 = add i64 %v9087, -1
  store i64 %v9088, ptr %t910, align 1
  %t1028.not = icmp eq i64 %v9088, 0
  br i1 %t1028.not, label %edge1121, label %edge1147

edge1121:                                         ; preds = %edge1118
  %v9089 = call i64 @mech_compact_free(i64 %v0, i64 %v1401)
  %t1029.not = icmp eq i64 %v9089, 0
  br i1 %t1029.not, label %edge1147, label %edge1122

edge1122:                                         ; preds = %edge1121
  %t1030 = insertvalue { i64, i64 } poison, i64 %v9089, 0
  %t1031 = insertvalue { i64, i64 } %t1030, i64 399, 1
  br label %common.ret

edge1128:                                         ; preds = %block11031, %edge1105
  %v9256 = add nuw nsw i64 %v8513, 1
  br label %block364

edge1147:                                         ; preds = %edge1039, %edge1036, %edge1121, %edge1118, %edge1117
  %t912.not22464 = phi i1 [ false, %edge1039 ], [ false, %edge1036 ], [ %t912.not, %edge1121 ], [ %t912.not, %edge1118 ], [ %t912.not, %edge1117 ]
  br label %edge1179

edge1160:                                         ; preds = %edge1196
  br i1 %t912.not22464, label %edge1200, label %edge1161

edge1161:                                         ; preds = %edge1160
  %t1037 = inttoptr i64 %v8476 to ptr
  %v9388 = load i64, ptr %t1037, align 1
  %v9389 = add i64 %v9388, -1
  store i64 %v9389, ptr %t1037, align 1
  %t1039.not = icmp eq i64 %v9389, 0
  br i1 %t1039.not, label %edge1164, label %edge1200

edge1164:                                         ; preds = %edge1161
  %v9390 = call i64 @mech_compact_free(i64 %v0, i64 %v8476)
  %t1040.not = icmp eq i64 %v9390, 0
  br i1 %t1040.not, label %edge1200, label %edge1165

edge1165:                                         ; preds = %edge1164
  %t1041 = insertvalue { i64, i64 } poison, i64 %v9390, 0
  %t1042 = insertvalue { i64, i64 } %t1041, i64 423, 1
  br label %common.ret

edge1179:                                         ; preds = %edge1147, %edge1196
  %v218766 = phi i64 [ 0, %edge1147 ], [ %v9553, %edge1196 ]
  %v218767 = phi i64 [ 0, %edge1147 ], [ %v9769, %edge1196 ]
  %v9744 = add i64 %v218766, %v8514
  %t1050 = inttoptr i64 %v9744 to ptr
  %t1051 = load i8, ptr %t1050, align 1
  %v9745 = zext i8 %t1051 to i64
  %v9769 = add i64 %v218767, %v9745
  %t1052.not = icmp ult i64 %v9769, %v218767
  br i1 %t1052.not, label %common.ret, label %edge1196

edge1196:                                         ; preds = %edge1179
  %v9553 = add nuw nsw i64 %v218766, 1
  %exitcond21622.not = icmp eq i64 %v9553, 256
  br i1 %exitcond21622.not, label %edge1160, label %edge1179

edge1200:                                         ; preds = %edge1160, %edge1161, %edge1164
  %t1056 = icmp ugt i64 %v9769, 79
  br i1 %t1056, label %edge948, label %edge1203

edge1203:                                         ; preds = %edge1200
  %v10208 = add i64 %v9769, ptrtoint (ptr @mech_compact_table_6 to i64)
  %t1061 = inttoptr i64 %v10208 to ptr
  %t1062 = load i8, ptr %t1061, align 1
  %v10209 = zext i8 %t1062 to i64
  br label %edge948

edge1217:                                         ; preds = %edge948
  %v10553 = load i64, ptr %t910, align 1
  %v10554 = add i64 %v10553, 1
  store i64 %v10554, ptr %t910, align 1
  br label %block475.outer.preheader

block475.outer.preheader:                         ; preds = %edge948, %edge1217
  br label %block475.outer.outer

block475.outer.outer:                             ; preds = %block2378, %block475.outer.preheader
  %v10789.ph.ph = phi i64 [ %v219204, %block475.outer.preheader ], [ %v36630646, %block2378 ]
  %v10790.ph.ph = phi i64 [ %v219207, %block475.outer.preheader ], [ %v4033512111224, %block2378 ]
  %v10791.ph.ph = phi i64 [ %v219236, %block475.outer.preheader ], [ %v46665, %block2378 ]
  %v10792.ph.ph = phi i64 [ %v219240, %block475.outer.preheader ], [ %v46677, %block2378 ]
  %v10793.ph.ph = phi i64 [ %v219247, %block475.outer.preheader ], [ %v45198, %block2378 ]
  %v10794.ph.ph = phi i64 [ %v219251, %block475.outer.preheader ], [ %v45183, %block2378 ]
  %v10796.ph.ph = phi i64 [ %v219203, %block475.outer.preheader ], [ %v36627645, %block2378 ]
  %v10798.ph.ph = phi i64 [ %v219206, %block475.outer.preheader ], [ %v4033212091225, %block2378 ]
  %v10800.ph.ph = phi i64 [ %v219234, %block475.outer.preheader ], [ %v46662, %block2378 ]
  %v10802.ph.ph = phi i64 [ %v219238, %block475.outer.preheader ], [ %v46674, %block2378 ]
  %v10804.ph.ph = phi i64 [ %v219245, %block475.outer.preheader ], [ %v45195, %block2378 ]
  %v10806.ph.ph = phi i64 [ %v219249, %block475.outer.preheader ], [ %v45180, %block2378 ]
  %v10809.ph.ph = phi i64 [ %v219214, %block475.outer.preheader ], [ %v36649651, %block2378 ]
  %v10810.ph.ph = phi i64 [ %v219215, %block475.outer.preheader ], [ %v36652652, %block2378 ]
  %v10811.ph.ph = phi i64 [ %v219216, %block475.outer.preheader ], [ %v41285, %block2378 ]
  %v10812.ph.ph = phi i64 [ %v219218, %block475.outer.preheader ], [ %spec.select1, %block2378 ]
  %v10813.ph.ph = phi i64 [ %v219219, %block475.outer.preheader ], [ %v41335, %block2378 ]
  %v10814.ph.ph = phi i64 [ %v219220, %block475.outer.preheader ], [ %v40316, %block2378 ]
  %v10815.ph.ph = phi i64 [ 0, %block475.outer.preheader ], [ %v46282, %block2378 ]
  %v10816.ph.ph = phi i64 [ %v219252, %block475.outer.preheader ], [ 0, %block2378 ]
  %v10817.ph.ph = phi i64 [ %v219216, %block475.outer.preheader ], [ %v40059, %block2378 ]
  %v10819.ph.ph = phi i64 [ %v219217, %block475.outer.preheader ], [ %v218797, %block2378 ]
  %v10820.ph.ph = phi i64 [ %v7671, %block475.outer.preheader ], [ %v218796, %block2378 ]
  %v10821.ph.ph = phi i64 [ %v10552, %block475.outer.preheader ], [ %v218795, %block2378 ]
  %v10822.ph.ph = phi i64 [ 0, %block475.outer.preheader ], [ %v41285, %block2378 ]
  %v10823.ph.ph = phi i64 [ 0, %block475.outer.preheader ], [ 1, %block2378 ]
  %v10824.ph.ph = phi i64 [ %v219201, %block475.outer.preheader ], [ %v36625644, %block2378 ]
  %v10825.ph.ph = phi i64 [ %v219205, %block475.outer.preheader ], [ %v4033012061226, %block2378 ]
  %v10828.ph.ph = phi i64 [ %v219233, %block475.outer.preheader ], [ %v46660, %block2378 ]
  %v10829.ph.ph = phi i64 [ %v219237, %block475.outer.preheader ], [ %v46672, %block2378 ]
  %v10830.ph.ph = phi i64 [ %v219244, %block475.outer.preheader ], [ %v45193, %block2378 ]
  %v10831.ph.ph = phi i64 [ %v219248, %block475.outer.preheader ], [ %v45178, %block2378 ]
  %v12013.ph.ph = phi i64 [ %v219256, %block475.outer.preheader ], [ %v218771, %block2378 ]
  %v12417.ph.ph = phi i64 [ %v219257, %block475.outer.preheader ], [ %v218773, %block2378 ]
  %t1083 = icmp ne i64 %v10791.ph.ph, 0
  %t1084 = inttoptr i64 %v10791.ph.ph to ptr
  %t1091 = icmp ne i64 %v10793.ph.ph, 0
  %t1092 = inttoptr i64 %v10793.ph.ph to ptr
  %t1103 = icmp ugt i64 %v10800.ph.ph, 258
  %t1107 = icmp ugt i64 %v10804.ph.ph, 9
  %t1179 = zext i64 %v10828.ph.ph to i128
  %t1180 = shl nuw i128 %t1179, 64
  %v219621 = or disjoint i128 %t1180, %t942
  %t1199 = zext i64 %v10830.ph.ph to i128
  %t1200 = shl nuw i128 %t1199, 64
  %v219629 = or disjoint i128 %t1200, 259
  %t1224 = zext i64 %v10791.ph.ph to i128
  %t1225 = shl nuw i128 %t1224, 64
  %t1229 = zext i64 %v10793.ph.ph to i128
  %t1230 = shl nuw i128 %t1229, 64
  %t1239 = zext i64 %v10800.ph.ph to i128
  %t1240 = shl nuw i128 %t1239, 64
  %t1244 = zext i64 %v10804.ph.ph to i128
  %t1245 = shl nuw i128 %t1244, 64
  %t3136.not = icmp eq i64 %v10793.ph.ph, 0
  br label %block475.outer

block475.outer:                                   ; preds = %edge4265, %block475.outer.outer
  %v10789.ph = phi i64 [ %v10789.ph.ph, %block475.outer.outer ], [ %v36630646, %edge4265 ]
  %v10790.ph = phi i64 [ %v10790.ph.ph, %block475.outer.outer ], [ %v4033512111224, %edge4265 ]
  %v10792.ph = phi i64 [ %v10792.ph.ph, %block475.outer.outer ], [ %v40037, %edge4265 ]
  %v10794.ph = phi i64 [ %v10794.ph.ph, %block475.outer.outer ], [ %v40049, %edge4265 ]
  %v10796.ph = phi i64 [ %v10796.ph.ph, %block475.outer.outer ], [ %v36627645, %edge4265 ]
  %v10798.ph = phi i64 [ %v10798.ph.ph, %block475.outer.outer ], [ %v4033212091225, %edge4265 ]
  %v10802.ph = phi i64 [ %v10802.ph.ph, %block475.outer.outer ], [ %v40034, %edge4265 ]
  %v10806.ph = phi i64 [ %v10806.ph.ph, %block475.outer.outer ], [ %v40046, %edge4265 ]
  %v10809.ph = phi i64 [ %v10809.ph.ph, %block475.outer.outer ], [ %v36649651, %edge4265 ]
  %v10810.ph = phi i64 [ %v10810.ph.ph, %block475.outer.outer ], [ %v36652652, %edge4265 ]
  %v10811.ph = phi i64 [ %v10811.ph.ph, %block475.outer.outer ], [ %v41285, %edge4265 ]
  %v10812.ph = phi i64 [ %v10812.ph.ph, %block475.outer.outer ], [ %spec.select1, %edge4265 ]
  %v10813.ph = phi i64 [ %v10813.ph.ph, %block475.outer.outer ], [ %v41335, %edge4265 ]
  %v10814.ph = phi i64 [ %v10814.ph.ph, %block475.outer.outer ], [ %v40316, %edge4265 ]
  %v10815.ph = phi i64 [ %v10815.ph.ph, %block475.outer.outer ], [ %v218798, %edge4265 ]
  %v10816.ph = phi i64 [ %v10816.ph.ph, %block475.outer.outer ], [ %v40056, %edge4265 ]
  %v10817.ph = phi i64 [ %v10817.ph.ph, %block475.outer.outer ], [ %v40059, %edge4265 ]
  %v10818.ph = phi i64 [ 0, %block475.outer.outer ], [ %v41267, %edge4265 ]
  %v10819.ph = phi i64 [ %v10819.ph.ph, %block475.outer.outer ], [ %v218797, %edge4265 ]
  %v10820.ph = phi i64 [ %v10820.ph.ph, %block475.outer.outer ], [ %v218796, %edge4265 ]
  %v10821.ph = phi i64 [ %v10821.ph.ph, %block475.outer.outer ], [ %v218795, %edge4265 ]
  %v10822.ph = phi i64 [ %v10822.ph.ph, %block475.outer.outer ], [ %v218803, %edge4265 ]
  %v10823.ph = phi i64 [ %v10823.ph.ph, %block475.outer.outer ], [ %v218804, %edge4265 ]
  %v10824.ph = phi i64 [ %v10824.ph.ph, %block475.outer.outer ], [ %v36625644, %edge4265 ]
  %v10825.ph = phi i64 [ %v10825.ph.ph, %block475.outer.outer ], [ %v4033012061226, %edge4265 ]
  %v10829.ph = phi i64 [ %v10829.ph.ph, %block475.outer.outer ], [ %v40032, %edge4265 ]
  %v10831.ph = phi i64 [ %v10831.ph.ph, %block475.outer.outer ], [ %v40044, %edge4265 ]
  %v12013.ph = phi i64 [ %v12013.ph.ph, %block475.outer.outer ], [ %v218771, %edge4265 ]
  %v12417.ph = phi i64 [ %v12417.ph.ph, %block475.outer.outer ], [ %v218773, %edge4265 ]
  br label %block475

edge1242:                                         ; preds = %block475
  %t1076 = inttoptr i64 %v10789 to ptr
  %v10633 = load i64, ptr %t1076, align 1
  %t1077 = icmp eq i64 %v10633, 1
  %t1079 = icmp ne i64 %v10790, 0
  %or.cond = select i1 %t1077, i1 %t1079, i1 false
  br i1 %or.cond, label %edge1246, label %edge1406

edge1246:                                         ; preds = %edge1242
  %t1080 = inttoptr i64 %v10790 to ptr
  %v10635 = load i64, ptr %t1080, align 1
  %t1081 = icmp eq i64 %v10635, 1
  %or.cond13 = select i1 %t1081, i1 %t1083, i1 false
  br i1 %or.cond13, label %edge1250, label %edge1406

edge1250:                                         ; preds = %edge1246
  %v10637 = load i64, ptr %t1084, align 1
  %t1085 = icmp eq i64 %v10637, 1
  %t1087 = icmp ne i64 %v10792, 0
  %or.cond14 = select i1 %t1085, i1 %t1087, i1 false
  br i1 %or.cond14, label %edge1254, label %edge1406

edge1254:                                         ; preds = %edge1250
  %t1088 = inttoptr i64 %v10792 to ptr
  %v10639 = load i64, ptr %t1088, align 1
  %t1089 = icmp eq i64 %v10639, 1
  %or.cond15 = select i1 %t1089, i1 %t1091, i1 false
  br i1 %or.cond15, label %edge1258, label %edge1406

edge1258:                                         ; preds = %edge1254
  %v10641 = load i64, ptr %t1092, align 1
  %t1093 = icmp eq i64 %v10641, 1
  %t1095 = icmp ne i64 %v10794, 0
  %or.cond16 = select i1 %t1093, i1 %t1095, i1 false
  br i1 %or.cond16, label %edge1262, label %edge1406

edge1262:                                         ; preds = %edge1258
  %t1096 = inttoptr i64 %v10794 to ptr
  %v10643 = load i64, ptr %t1096, align 1
  %t1097 = icmp eq i64 %v10643, 1
  %t1099 = icmp ugt i64 %v10796, 262143
  %or.cond305.not1295.not1297 = select i1 %t1097, i1 %t1099, i1 false
  %t1101.not = icmp ule i64 %v3720, %v10798
  %or.cond306.not1296 = select i1 %or.cond305.not1295.not1297, i1 %t1101.not, i1 false
  %or.cond307 = select i1 %or.cond306.not1296, i1 %t1103, i1 false
  %t1105 = icmp ugt i64 %v10802, 258
  %or.cond308 = select i1 %or.cond307, i1 %t1105, i1 false
  %or.cond309 = select i1 %or.cond308, i1 %t1107, i1 false
  %t1109 = icmp ugt i64 %v10806, 9
  %or.cond310 = select i1 %or.cond309, i1 %t1109, i1 false
  br i1 %or.cond310, label %edge1278, label %edge1406

edge1278:                                         ; preds = %edge1262
  %t1114 = zext i64 %v10809 to i128
  %t1115 = zext i64 %v10810 to i128
  %t1116 = shl nuw i128 %t1115, 64
  %v219595 = or disjoint i128 %t1116, %t1114
  store i128 %v219595, ptr %ss2, align 16
  %t1118 = zext i64 %v10811 to i128
  %t1119 = zext i64 %v10812 to i128
  %t1120 = shl nuw i128 %t1119, 64
  %v219597 = or disjoint i128 %t1120, %t1118
  store i128 %v219597, ptr %t926, align 16
  %t1123 = zext i64 %v10813 to i128
  %t1124 = zext i64 %v10814 to i128
  %t1125 = shl nuw i128 %t1124, 64
  %v219599 = or disjoint i128 %t1125, %t1123
  store i128 %v219599, ptr %t931, align 16
  %t1128 = zext i64 %v10815 to i128
  %t1129 = zext i64 %v10816 to i128
  %t1130 = shl nuw i128 %t1129, 64
  %v219601 = or disjoint i128 %t1130, %t1128
  store i128 %v219601, ptr %t936, align 16
  %t1133 = zext i64 %v10817 to i128
  %t1134 = zext i64 %v10818 to i128
  %t1135 = shl nuw i128 %t1134, 64
  %v219603 = or disjoint i128 %t1135, %t1133
  store i128 %v219603, ptr %t941, align 16
  %t1138 = zext i64 %v10819 to i128
  %t1139 = zext i64 %v10820 to i128
  %t1140 = shl nuw i128 %t1139, 64
  %v219605 = or disjoint i128 %t1140, %t1138
  store i128 %v219605, ptr %t946, align 16
  %t1143 = zext i64 %v10821 to i128
  %t1144 = zext i64 %v10822 to i128
  %t1145 = shl nuw i128 %t1144, 64
  %v219607 = or disjoint i128 %t1145, %t1143
  store i128 %v219607, ptr %t947, align 16
  %t1148 = zext i64 %v10823 to i128
  %t1149 = zext i64 %v10824 to i128
  %t1150 = shl nuw i128 %t1149, 64
  %v219609 = or disjoint i128 %t1150, %t1148
  store i128 %v219609, ptr %t1152, align 16
  store i128 18446744073709813760, ptr %t1157, align 16
  %t1159 = zext i64 %v10825 to i128
  %t1160 = shl nuw i128 %t1159, 64
  %v219613 = or disjoint i128 %t1160, 262144
  store i128 %v219613, ptr %t1162, align 16
  store i128 %v219615, ptr %t1167, align 16
  store i128 %v219617, ptr %t1172, align 16
  store i128 %v219585, ptr %t1177, align 16
  store i128 %v219621, ptr %t1182, align 16
  store i128 18446744073709551875, ptr %t1187, align 16
  %t1189 = zext i64 %v10829 to i128
  %t1190 = shl nuw i128 %t1189, 64
  %v219625 = or disjoint i128 %t1190, 259
  store i128 %v219625, ptr %t1192, align 16
  store i128 18446744073709551875, ptr %t1197, align 16
  store i128 %v219629, ptr %t1202, align 16
  store i128 18446744073709551626, ptr %t1207, align 16
  %t1209 = zext i64 %v10831 to i128
  %t1210 = shl nuw i128 %t1209, 64
  %v219633 = or disjoint i128 %t1210, 10
  store i128 %v219633, ptr %t1212, align 16
  store i128 18446744073709551626, ptr %t1217, align 16
  %t1219 = zext i64 %v10789 to i128
  %t1220 = shl nuw i128 %t1219, 64
  %v219637 = or disjoint i128 %t1220, 10
  store i128 %v219637, ptr %t1222, align 16
  %t1223 = zext i64 %v10790 to i128
  %v219639 = or disjoint i128 %t1225, %t1223
  store i128 %v219639, ptr %t1227, align 16
  %t1228 = zext i64 %v10792 to i128
  %v219641 = or disjoint i128 %t1230, %t1228
  store i128 %v219641, ptr %t1232, align 16
  %t1233 = zext i64 %v10794 to i128
  %t1234 = zext i64 %v10796 to i128
  %t1235 = shl nuw i128 %t1234, 64
  %v219643 = or disjoint i128 %t1235, %t1233
  store i128 %v219643, ptr %t1237, align 16
  %t1238 = zext i64 %v10798 to i128
  %v219645 = or disjoint i128 %t1240, %t1238
  store i128 %v219645, ptr %t1242, align 16
  %t1243 = zext i64 %v10802 to i128
  %v219647 = or disjoint i128 %t1245, %t1243
  store i128 %v219647, ptr %t1247, align 16
  store i64 %v10806, ptr %t1248, align 16
  %t1249 = call { i64, i64 } @mech_compact_numeric_475_vector(i64 %v0, i64 %v219576, i64 %v8481)
  %v10660 = extractvalue { i64, i64 } %t1249, 0
  %v10661 = extractvalue { i64, i64 } %t1249, 1
  switch i64 %v10660, label %edge1349 [
    i64 103172, label %edge1281
    i64 0, label %edge1350
  ]

edge1281:                                         ; preds = %edge1278
  %t1252 = icmp eq i64 %v10661, 510
  br i1 %t1252, label %edge1283, label %common.ret

edge1283:                                         ; preds = %edge1281
  %v219649 = load i128, ptr %t1255, align 8
  %v219650 = trunc i128 %v219649 to i64
  %t1256 = lshr i128 %v219649, 64
  %v219651 = trunc nuw i128 %t1256 to i64
  %v219653 = load i128, ptr %t1258, align 8
  %v219654 = trunc i128 %v219653 to i64
  %t1259 = lshr i128 %v219653, 64
  %v219655 = trunc nuw i128 %t1259 to i64
  %v219657 = load i128, ptr %t1261, align 8
  %v219658 = trunc i128 %v219657 to i64
  %t1262 = lshr i128 %v219657, 64
  %v219659 = trunc nuw i128 %t1262 to i64
  %v219661 = load i128, ptr %t1264, align 8
  %v219662 = trunc i128 %v219661 to i64
  %t1265 = lshr i128 %v219661, 64
  %v219663 = trunc nuw i128 %t1265 to i64
  %v219665 = load i128, ptr %t1267, align 8
  %v219666 = trunc i128 %v219665 to i64
  %t1268 = lshr i128 %v219665, 64
  %v219667 = trunc nuw i128 %t1268 to i64
  %v219669 = load i128, ptr %t1270, align 8
  %v219670 = trunc i128 %v219669 to i64
  %t1271 = lshr i128 %v219669, 64
  %v219671 = trunc nuw i128 %t1271 to i64
  %v219673 = load i128, ptr %t1273, align 8
  %v219674 = trunc i128 %v219673 to i64
  %t1274 = lshr i128 %v219673, 64
  %v219675 = trunc nuw i128 %t1274 to i64
  %v10680 = load i64, ptr %t1275, align 8
  %v10681 = load i64, ptr %t1076, align 1
  %v10682 = add i64 %v10681, 1
  store i64 %v10682, ptr %t1076, align 1
  %v10683 = load i64, ptr %t1080, align 1
  %v10684 = add i64 %v10683, 1
  store i64 %v10684, ptr %t1080, align 1
  br i1 %t23.not, label %edge1294, label %edge1291

edge1291:                                         ; preds = %edge1283
  %v10685 = load i64, ptr %t910, align 1
  %v10686 = add i64 %v10685, 1
  store i64 %v10686, ptr %t910, align 1
  br label %edge1294

edge1294:                                         ; preds = %edge1291, %edge1283
  %v10687 = load i64, ptr %t1084, align 1
  %v10688 = add i64 %v10687, 1
  store i64 %v10688, ptr %t1084, align 1
  %v10689 = load i64, ptr %t1088, align 1
  %v10690 = add i64 %v10689, 1
  store i64 %v10690, ptr %t1088, align 1
  %v10691 = load i64, ptr %t1092, align 1
  %v10692 = add i64 %v10691, 1
  store i64 %v10692, ptr %t1092, align 1
  %v10693 = load i64, ptr %t1096, align 1
  %v10694 = add i64 %v10693, 1
  store i64 %v10694, ptr %t1096, align 1
  %v10698 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10789)
  %t1298.not = icmp eq i64 %v10698, 0
  br i1 %t1298.not, label %edge1314, label %block11840

edge1314:                                         ; preds = %edge1294
  %v10702 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10790)
  %t1300.not = icmp eq i64 %v10702, 0
  br i1 %t1300.not, label %block11893, label %block11840

edge1320:                                         ; preds = %block11893
  %v10706 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t1302.not = icmp eq i64 %v10706, 0
  br i1 %t1302.not, label %edge1326, label %block11840

edge1326:                                         ; preds = %block11893, %edge1320
  %v10710 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10791.ph.ph)
  %t1304.not = icmp eq i64 %v10710, 0
  br i1 %t1304.not, label %edge1332, label %block11840

edge1332:                                         ; preds = %edge1326
  %v10714 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10792)
  %t1306.not = icmp eq i64 %v10714, 0
  br i1 %t1306.not, label %edge1338, label %block11840

edge1338:                                         ; preds = %edge1332
  %v10718 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10793.ph.ph)
  %t1308.not = icmp eq i64 %v10718, 0
  br i1 %t1308.not, label %edge1344, label %block11840

edge1344:                                         ; preds = %edge1338
  %v10722 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10794)
  %t1310.not = icmp eq i64 %v10722, 0
  br i1 %t1310.not, label %edge1438, label %block11840

edge1349:                                         ; preds = %edge1278
  %t1314 = icmp eq i64 %v10660, 103169
  %v10725 = select i1 %t1314, i64 103425, i64 %v10660
  %t1316 = insertvalue { i64, i64 } poison, i64 %v10725, 0
  %t1317 = insertvalue { i64, i64 } %t1316, i64 %v10661, 1
  br label %common.ret

edge1350:                                         ; preds = %edge1278
  %v219677 = load i128, ptr %t1267, align 8
  %v219678 = trunc i128 %v219677 to i64
  %t1320 = lshr i128 %v219677, 64
  %v219679 = trunc nuw i128 %t1320 to i64
  %v10728 = load i64, ptr %t1270, align 8
  %v219681 = load i128, ptr %t1273, align 8
  %v219682 = trunc i128 %v219681 to i64
  %t1324 = lshr i128 %v219681, 64
  %v219683 = trunc nuw i128 %t1324 to i64
  %v219685 = load i128, ptr %t1326, align 8
  %v219686 = trunc i128 %v219685 to i64
  %t1327 = lshr i128 %v219685, 64
  %v219687 = trunc nuw i128 %t1327 to i64
  %v219689 = load i128, ptr %t1329, align 8
  %v219690 = trunc i128 %v219689 to i64
  %t1330 = lshr i128 %v219689, 64
  %v219691 = trunc nuw i128 %t1330 to i64
  %v219693 = load i128, ptr %t1332, align 8
  %v219694 = trunc i128 %v219693 to i64
  %t1333 = lshr i128 %v219693, 64
  %v219695 = trunc nuw i128 %t1333 to i64
  %v219697 = load i128, ptr %t1335, align 8
  %v219698 = trunc i128 %v219697 to i64
  %t1336 = lshr i128 %v219697, 64
  %v219699 = trunc nuw i128 %t1336 to i64
  %v10741 = load i64, ptr %t1337, align 8
  br i1 %t23.not, label %edge5300, label %edge1353

edge1353:                                         ; preds = %edge1350
  %v10757 = load i64, ptr %t910, align 1
  %v10758 = add i64 %v10757, -1
  store i64 %v10758, ptr %t910, align 1
  %t1341.not = icmp eq i64 %v10758, 0
  br i1 %t1341.not, label %edge1356, label %edge5300

edge1356:                                         ; preds = %edge1353
  %v10759 = call i64 @mech_compact_free(i64 %v0, i64 %v1401)
  %t1342.not = icmp eq i64 %v10759, 0
  br i1 %t1342.not, label %edge5300, label %block11840

edge1406:                                         ; preds = %edge1262, %edge1258, %edge1254, %edge1250, %edge1246, %edge1242, %block475
  %v11039 = sub i64 %v1397, %v10811
  %t1345 = icmp eq i64 %v10811, %v10813
  br i1 %t1345, label %edge1438, label %block535

edge1423:                                         ; preds = %edge1496
  %t1355 = inttoptr i64 %v11463 to ptr
  %v11397 = load i64, ptr %t1355, align 1
  %v11398 = add i64 %v11397, 1
  store i64 %v11398, ptr %t1355, align 1
  %v11402 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v11463)
  %t1357.not = icmp eq i64 %v11402, 0
  br i1 %t1357.not, label %block535, label %block12160

edge1438:                                         ; preds = %edge1406, %edge1344
  %v11830 = phi i64 [ 1, %edge1406 ], [ %v10680, %edge1344 ]
  %v11998 = phi i64 [ %v10811, %edge1406 ], [ %v219654, %edge1344 ]
  %v12011 = phi i64 [ %v10812, %edge1406 ], [ %v12013, %edge1344 ]
  %v12211 = phi i64 [ %v11039, %edge1406 ], [ %v219675, %edge1344 ]
  %v12415 = phi i64 [ %v10811, %edge1406 ], [ %v12417, %edge1344 ]
  %v12659 = phi i64 [ %v10818, %edge1406 ], [ %v219663, %edge1344 ]
  %v14681 = phi i64 [ %v10809, %edge1406 ], [ %v219650, %edge1344 ]
  %v18190 = phi i64 [ %v10814, %edge1406 ], [ %v219655, %edge1344 ]
  %v18669 = phi i64 [ %v10810, %edge1406 ], [ %v219651, %edge1344 ]
  %v37162 = phi i64 [ %v10817, %edge1406 ], [ %v219662, %edge1344 ]
  %v37925 = phi i64 [ %v10816, %edge1406 ], [ %v219659, %edge1344 ]
  %v38009 = phi i64 [ %v10821, %edge1406 ], [ %v219670, %edge1344 ]
  %v41626 = phi i64 [ %v10820, %edge1406 ], [ %v219667, %edge1344 ]
  %v41853 = phi i64 [ %v10819, %edge1406 ], [ %v219666, %edge1344 ]
  %v42190 = phi i64 [ %v10815, %edge1406 ], [ %v219658, %edge1344 ]
  %v48120 = phi i64 [ %v10822, %edge1406 ], [ %v219671, %edge1344 ]
  %v48176 = phi i64 [ %v10823, %edge1406 ], [ %v219674, %edge1344 ]
  %v11458 = call i64 @mech_compact_allocate(i64 %v0, i64 262144, i64 2, i64 2, i64 0, i64 %v3194)
  %t1372.not = icmp eq i64 %v11458, 0
  br i1 %t1372.not, label %edge1441, label %block12170

edge1441:                                         ; preds = %edge1438
  %v11461 = load i64, ptr %ss1, align 8
  %v11462 = load i64, ptr %t606, align 8
  %v11463 = load i64, ptr %t607, align 8
  store i128 604462909807314587353088, ptr %ss4, align 16
  store i128 18446744073709551616, ptr %t1383, align 16
  %v11465 = call i64 @mech_compact_word_collection_10(i64 %v219700, i64 %v10824, i64 %v11461, i64 262144)
  %t1384 = icmp eq i64 %v11465, 0
  br i1 %t1384, label %edge1442, label %block12170

edge1442:                                         ; preds = %edge1441
  %t1386.not = icmp eq i64 %v11463, 0
  br i1 %t1386.not, label %block12171, label %edge1444

edge1444:                                         ; preds = %edge1442
  %t1387 = inttoptr i64 %v11463 to ptr
  %v11468 = load i64, ptr %t1387, align 1
  %v11469 = add i64 %v11468, 1
  store i64 %v11469, ptr %t1387, align 1
  br label %block12171

edge1453:                                         ; preds = %block12171
  %v11477 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10789)
  %t1392.not = icmp eq i64 %v11477, 0
  br i1 %t1392.not, label %edge1496, label %block12170

edge1496:                                         ; preds = %block12171, %edge1453
  br i1 %t1386.not, label %block535, label %edge1423

edge1545:                                         ; preds = %block535
  %v12940 = sub i64 %v218770, %spec.select1
  %v13659 = add i64 %spec.select18, %v218770
  %t1495.not = icmp ult i64 %v13659, %v218770
  br i1 %t1495.not, label %common.ret, label %edge1658

edge1658:                                         ; preds = %edge1545
  %t1499.not = icmp ugt i64 %v13659, %v1397
  br i1 %t1499.not, label %edge1677, label %edge1763

edge1677:                                         ; preds = %edge1658
  br i1 %t23.not, label %block13203, label %edge1680

edge1680:                                         ; preds = %edge1677
  %v13743 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t1502.not = icmp eq i64 %v13743, 0
  br i1 %t1502.not, label %block13203, label %block13135

edge1686:                                         ; preds = %block13203
  %v13747 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219198)
  %t1504.not = icmp eq i64 %v13747, 0
  br i1 %t1504.not, label %block13207, label %block13135

edge1692:                                         ; preds = %block13207
  %v13751 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219213)
  %t1506.not = icmp eq i64 %v13751, 0
  br i1 %t1506.not, label %block13211, label %block13135

edge1698:                                         ; preds = %block13211
  %v13755 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219226)
  %t1508.not = icmp eq i64 %v13755, 0
  br i1 %t1508.not, label %block13215, label %block13135

edge1704:                                         ; preds = %block13215
  %v13759 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219232)
  %t1510.not = icmp eq i64 %v13759, 0
  br i1 %t1510.not, label %block13219, label %block13135

edge1710:                                         ; preds = %block13219
  %v13763 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219242)
  %t1512.not = icmp eq i64 %v13763, 0
  br i1 %t1512.not, label %block13227, label %block13135

edge1722:                                         ; preds = %block13227
  %v13771 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10790)
  %t1516.not = icmp eq i64 %v13771, 0
  br i1 %t1516.not, label %block13231, label %block13135

edge1728:                                         ; preds = %block13231
  %v13775 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t1518.not = icmp eq i64 %v13775, 0
  br i1 %t1518.not, label %block13235, label %block13135

edge1734:                                         ; preds = %block13235
  %v13779 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10791.ph.ph)
  %t1520.not = icmp eq i64 %v13779, 0
  br i1 %t1520.not, label %block13239, label %block13135

edge1740:                                         ; preds = %block13239
  %v13783 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10792)
  %t1522.not = icmp eq i64 %v13783, 0
  br i1 %t1522.not, label %block13243, label %block13135

edge1746:                                         ; preds = %block13243
  %v13787 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10793.ph.ph)
  %t1524.not = icmp eq i64 %v13787, 0
  br i1 %t1524.not, label %block13247, label %block13135

edge1752:                                         ; preds = %block13247
  %v13791 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10794)
  %t1526.not = icmp eq i64 %v13791, 0
  br i1 %t1526.not, label %block13251, label %block13135

edge1758:                                         ; preds = %block13251
  %v13795 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v218781)
  %t1528.not = icmp eq i64 %v13795, 0
  br i1 %t1528.not, label %common.ret, label %block13135

edge1763:                                         ; preds = %edge1658
  %v13814 = add i64 %v218770, 1
  %t1533.not = icmp ugt i64 %v13814, %v14025
  br i1 %t1533.not, label %edge1789, label %edge1776

edge1776:                                         ; preds = %edge1763
  br i1 %v1062051.not1294, label %edge1781, label %edge1781.thread

edge1781.thread:                                  ; preds = %edge1776
  %v14212 = add i64 %v13814, %v1396
  %t1539 = inttoptr i64 %v14212 to ptr
  %t1540 = load i32, ptr %t1539, align 1
  %v14213 = zext i32 %t1540 to i64
  br label %edge1789

edge1781:                                         ; preds = %edge1776
  %v14215 = call i64 @mech_compact_matrix_word(i64 %v1396, i64 %v13814, i64 %v1399, i64 %v1400, i64 4)
  %t1541 = icmp ult i64 %v14215, 4294967296
  br i1 %t1541, label %edge1789, label %common.ret

edge1789:                                         ; preds = %edge1781.thread, %edge1781, %edge1763
  %v14423 = phi i64 [ %v14215, %edge1781 ], [ 0, %edge1763 ], [ %v14213, %edge1781.thread ]
  %v14477 = and i64 %v14423, 16777215
  %v14534 = mul nuw nsw i64 %v14477, 506832829
  %v14539 = lshr i64 %v14534, 16
  %v14588 = and i64 %v14539, 65535
  %v14597 = mul nuw nsw i64 %v14423, 506832829
  %v14602 = lshr i64 %v14597, 16
  %v14615 = and i64 %v14602, 65535
  %v14628 = add i64 %v12940, 32768
  %v14683 = shl i64 %v218782, 2
  %v14720 = and i64 %v14683, 262140
  %v14776 = or disjoint i64 %v14720, 262144
  %v14777 = add i64 %v14776, %v218783
  %t1549 = inttoptr i64 %v14777 to ptr
  %t1550 = load i16, ptr %t1549, align 1
  %v14778 = zext i16 %t1550 to i64
  %v14804 = or disjoint i64 %v14720, 262146
  %v14805 = add i64 %v14804, %v218783
  %t1555 = inttoptr i64 %v14805 to ptr
  %t1556 = load i16, ptr %t1555, align 1
  %v14806 = zext i16 %t1556 to i64
  %t1559.not = icmp eq i64 %v218781, 0
  br i1 %t1559.not, label %edge1835, label %edge1822

edge1822:                                         ; preds = %edge1789
  %t1561 = inttoptr i64 %v218781 to ptr
  %v14819 = load i64, ptr %t1561, align 1
  %t1562 = icmp eq i64 %v14819, 1
  %v14829 = trunc i64 %v14628 to i16
  br i1 %t1562, label %edge1826, label %edge1835.thread

edge1826:                                         ; preds = %edge1822
  store i16 %v14829, ptr %t1549, align 1
  br label %block662.sink.split

edge1835:                                         ; preds = %edge1789
  %12 = trunc i64 %v14628 to i16
  %t1578 = icmp eq i16 %t1550, %12
  br i1 %t1578, label %edge1858.thread, label %block13625

edge1835.thread:                                  ; preds = %edge1822
  %t1578470 = icmp eq i16 %t1550, %v14829
  br i1 %t1578470, label %block13623, label %block13625

edge1847:                                         ; preds = %block13623
  %t1606 = inttoptr i64 %v14880 to ptr
  %v14904 = load i64, ptr %t1606, align 1
  %v14905 = add i64 %v14904, 1
  store i64 %v14905, ptr %t1606, align 1
  br label %block13629

edge1852:                                         ; preds = %block13629
  %v14910 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v218781)
  %t1610.not = icmp eq i64 %v14910, 0
  br i1 %t1610.not, label %edge1858, label %block13600

edge1858:                                         ; preds = %block13629, %edge1852
  %v14929 = add i64 %v14875, %v14804
  %t1615 = inttoptr i64 %v14929 to ptr
  %t1616 = load i16, ptr %t1615, align 1
  %t1617 = icmp eq i16 %t1616, %t1550
  br i1 %t1617, label %block662, label %edge1861

edge1858.thread:                                  ; preds = %edge1835
  %v1492922048 = add i64 %v218783, %v14804
  %t161522049 = inttoptr i64 %v1492922048 to ptr
  %t161622050 = load i16, ptr %t161522049, align 1
  %t161722051 = icmp eq i16 %t161622050, %t1550
  br i1 %t161722051, label %block662, label %block13640

edge1861:                                         ; preds = %edge1858
  br i1 %t1605.not52, label %block13640, label %edge1862

edge1862:                                         ; preds = %edge1861
  %t1620 = inttoptr i64 %v14880 to ptr
  %v14946 = load i64, ptr %t1620, align 1
  %t1621 = icmp eq i64 %v14946, 1
  br i1 %t1621, label %block662.sink.split, label %block13640

edge1870:                                         ; preds = %block13638
  %t1645 = inttoptr i64 %v14958 to ptr
  %v14963 = load i64, ptr %t1645, align 1
  %v14964 = add i64 %v14963, 1
  store i64 %v14964, ptr %t1645, align 1
  br label %block13644

edge1875:                                         ; preds = %block13644
  %v14969 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1488021826218372205422064)
  %t1649.not = icmp eq i64 %v14969, 0
  br i1 %t1649.not, label %block662, label %block13635

edge1885:                                         ; preds = %block662
  %t1654.not = icmp ult i64 %v12940, %v14778
  br i1 %t1654.not, label %edge1896, label %edge2015

edge1889:                                         ; preds = %edge1945, %edge1994
  %v15983484.pn = phi i64 [ %v17059, %edge1994 ], [ %v15983484, %edge1945 ]
  %t1702.not.not = icmp eq i64 %v218770, %v15983484.pn
  br i1 %t1702.not.not, label %edge2015, label %edge2017

edge1896:                                         ; preds = %edge1885
  %t1657.not = icmp ugt i64 %v218770, %v14025
  br i1 %t1657.not, label %edge1917, label %edge1904

edge1904:                                         ; preds = %edge1896
  br i1 %v1062051.not1294, label %edge1909, label %edge1917.thread491

edge1909:                                         ; preds = %edge1904
  %v15727 = call i64 @mech_compact_matrix_word(i64 %v1396, i64 %v218770, i64 %v1399, i64 %v1400, i64 4)
  %t1665 = icmp ult i64 %v15727, 4294967296
  br i1 %t1665, label %edge1917.thread, label %common.ret

edge1917:                                         ; preds = %edge1896
  %v15974 = add i64 %spec.select1, -32768
  %v15983 = add i64 %v15974, %v14778
  %t1669.not = icmp ugt i64 %v15983, %v14025
  br i1 %t1669.not, label %edge1945, label %edge1932

edge1917.thread491:                               ; preds = %edge1904
  %v15724 = add i64 %v218770, %v1396
  %t1663 = inttoptr i64 %v15724 to ptr
  %t1664 = load i32, ptr %t1663, align 1
  %13 = and i32 %t1664, 16777215
  %v15953493 = zext nneg i32 %13 to i64
  %v15974494 = add i64 %spec.select1, -32768
  %v15983495 = add i64 %v15974494, %v14778
  %t1669.not496 = icmp ugt i64 %v15983495, %v14025
  br i1 %t1669.not496, label %edge1945, label %edge1937.thread

edge1917.thread:                                  ; preds = %edge1909
  %v15953477 = and i64 %v15727, 16777215
  %v15974478 = add i64 %spec.select1, -32768
  %v15983479 = add i64 %v15974478, %v14778
  %t1669.not480 = icmp ugt i64 %v15983479, %v14025
  br i1 %t1669.not480, label %edge1945, label %edge1937

edge1932:                                         ; preds = %edge1917
  br i1 %v1062051.not1294, label %edge1937, label %edge1937.thread

edge1937.thread:                                  ; preds = %edge1932, %edge1917.thread491
  %v15953481501 = phi i64 [ 0, %edge1932 ], [ %v15953493, %edge1917.thread491 ]
  %v15983483500 = phi i64 [ %v15983, %edge1932 ], [ %v15983495, %edge1917.thread491 ]
  %v16366 = add i64 %v15983483500, %v1396
  %t1675 = inttoptr i64 %v16366 to ptr
  %t1676 = load i32, ptr %t1675, align 1
  %v16367 = zext i32 %t1676 to i64
  br label %edge1945

edge1937:                                         ; preds = %edge1917.thread, %edge1932
  %v15953481490 = phi i64 [ 0, %edge1932 ], [ %v15953477, %edge1917.thread ]
  %v15983483487 = phi i64 [ %v15983, %edge1932 ], [ %v15983479, %edge1917.thread ]
  %v16369 = call i64 @mech_compact_matrix_word(i64 %v1396, i64 %v15983483487, i64 %v1399, i64 %v1400, i64 4)
  %t1677 = icmp ult i64 %v16369, 4294967296
  br i1 %t1677, label %edge1945, label %common.ret

edge1945:                                         ; preds = %edge1937.thread, %edge1917.thread491, %edge1917.thread, %edge1937, %edge1917
  %v15983484 = phi i64 [ %v15983483487, %edge1937 ], [ %v15983, %edge1917 ], [ %v15983479, %edge1917.thread ], [ %v15983495, %edge1917.thread491 ], [ %v15983483500, %edge1937.thread ]
  %v15953482 = phi i64 [ %v15953481490, %edge1937 ], [ 0, %edge1917 ], [ %v15953477, %edge1917.thread ], [ %v15953493, %edge1917.thread491 ], [ %v15953481501, %edge1937.thread ]
  %v16570 = phi i64 [ %v16369, %edge1937 ], [ 0, %edge1917 ], [ 0, %edge1917.thread ], [ 0, %edge1917.thread491 ], [ %v16367, %edge1937.thread ]
  %v16595 = and i64 %v16570, 16777215
  %t1681 = icmp eq i64 %v15953482, %v16595
  br i1 %t1681, label %edge1889, label %edge1959

edge1959:                                         ; preds = %edge1945
  %t1684.not = icmp samesign ult i64 %v12940, %v14806
  br i1 %t1684.not, label %edge1968, label %edge2015

edge1968:                                         ; preds = %edge1959
  %v17054 = add i64 %spec.select1, -32768
  %v17059 = add i64 %v17054, %v14806
  %t1687.not = icmp ugt i64 %v17059, %v14025
  br i1 %t1687.not, label %edge1994, label %edge1981

edge1981:                                         ; preds = %edge1968
  br i1 %v1062051.not1294, label %edge1986, label %edge1986.thread

edge1986.thread:                                  ; preds = %edge1981
  %v17438 = add i64 %v17059, %v1396
  %t1693 = inttoptr i64 %v17438 to ptr
  %t1694 = load i32, ptr %t1693, align 1
  %v17439 = zext i32 %t1694 to i64
  br label %edge1994

edge1986:                                         ; preds = %edge1981
  %v17441 = call i64 @mech_compact_matrix_word(i64 %v1396, i64 %v17059, i64 %v1399, i64 %v1400, i64 4)
  %t1695 = icmp ult i64 %v17441, 4294967296
  br i1 %t1695, label %edge1994, label %common.ret

edge1994:                                         ; preds = %edge1986.thread, %edge1986, %edge1968
  %v17642 = phi i64 [ %v17441, %edge1986 ], [ 0, %edge1968 ], [ %v17439, %edge1986.thread ]
  %v17667 = and i64 %v17642, 16777215
  %t1699 = icmp eq i64 %v15953482, %v17667
  br i1 %t1699, label %edge1889, label %edge2015

edge2015:                                         ; preds = %edge1959, %edge1994, %edge1885, %edge1889, %block15385, %edge2042
  %t1702.not473 = phi i64 [ 1, %block15385 ], [ 1, %edge2042 ], [ 0, %edge1889 ], [ 0, %edge1885 ], [ 0, %edge1994 ], [ 0, %edge1959 ]
  %v18606 = phi i64 [ %v18356, %block15385 ], [ %v18356, %edge2042 ], [ %v10825, %edge1889 ], [ %v10825, %edge1885 ], [ %v10825, %edge1994 ], [ %v10825, %edge1959 ]
  %v18608 = phi i64 [ %v18358, %block15385 ], [ %v18358, %edge2042 ], [ %v10798, %edge1889 ], [ %v10798, %edge1885 ], [ %v10798, %edge1994 ], [ %v10798, %edge1959 ]
  %v18611 = phi i64 [ %v18361, %block15385 ], [ %v18361, %edge2042 ], [ %v10790, %edge1889 ], [ %v10790, %edge1885 ], [ %v10790, %edge1994 ], [ %v10790, %edge1959 ]
  %v18612 = add i64 %t1702.not473, %v218785
  br label %edge2077

edge2017:                                         ; preds = %edge1889
  %v17914 = sub i64 %v218770, %v15983484.pn
  %v18149 = shl i64 %v17914, 16
  %v18154 = and i64 %v18149, 4294901760
  %v18174 = or disjoint i64 %v18154, 3
  %t1706 = icmp ult i64 %v218785, %v3720
  br i1 %t1706, label %edge2025, label %block15376

edge2025:                                         ; preds = %edge2017
  %v18350 = shl nuw nsw i64 %v218785, 2
  %v18351 = add i64 %v18350, %v10825
  %t1708 = inttoptr i64 %v18351 to ptr
  %t1709 = load i32, ptr %t1708, align 1
  %v18352 = zext i32 %t1709 to i64
  %t1710 = icmp eq i64 %v18174, %v18352
  br i1 %t1710, label %block15379, label %edge2028

edge2028:                                         ; preds = %edge2025
  %t1712.not = icmp eq i64 %v10790, 0
  br i1 %t1712.not, label %block15381, label %edge2029

edge2029:                                         ; preds = %edge2028
  %t1713 = inttoptr i64 %v10790 to ptr
  %v18368 = load i64, ptr %t1713, align 1
  %t1714 = icmp eq i64 %v18368, 1
  br i1 %t1714, label %block15380, label %block15381

edge2034:                                         ; preds = %block15381
  %v18378 = load i64, ptr %ss1, align 8
  %v18379 = load i64, ptr %t606, align 8
  %v18380 = load i64, ptr %t607, align 8
  %.pre21801 = add i64 %v18378, %v18350
  %.pre21802 = inttoptr i64 %.pre21801 to ptr
  br label %block15380

edge2037:                                         ; preds = %block15379
  %t1738 = inttoptr i64 %v18361 to ptr
  %v18385 = load i64, ptr %t1738, align 1
  %v18386 = add i64 %v18385, 1
  store i64 %v18386, ptr %t1738, align 1
  br label %block15385

edge2042:                                         ; preds = %block15385
  %v18391 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10790)
  %t1742.not = icmp eq i64 %v18391, 0
  br i1 %t1742.not, label %edge2015, label %block15376

edge2077:                                         ; preds = %edge2015, %block662
  %v18646 = phi i64 [ %v18606, %edge2015 ], [ %v10825, %block662 ]
  %v18648 = phi i64 [ %v18608, %edge2015 ], [ %v10798, %block662 ]
  %v18651 = phi i64 [ %v18611, %edge2015 ], [ %v10790, %block662 ]
  %v18658 = phi i64 [ %v18612, %edge2015 ], [ %v218785, %block662 ]
  %v18686 = shl i64 %v218788, 1
  %v18740 = and i64 %v18686, 131070
  %v18769 = or disjoint i64 %v18740, 131072
  %v18770 = add i64 %v18809, %v18769
  %t1750 = inttoptr i64 %v18770 to ptr
  %t1751 = load i16, ptr %t1750, align 1
  %v18771 = zext i16 %t1751 to i64
  %14 = trunc i64 %v14628 to i16
  %t1758 = icmp eq i16 %t1751, %14
  br i1 %t1758, label %block15570, label %edge2084

edge2084:                                         ; preds = %edge2077
  %t1760.not = icmp eq i64 %v18832, 0
  br i1 %t1760.not, label %block15572, label %edge2085

edge2085:                                         ; preds = %edge2084
  %t1761 = inttoptr i64 %v18832 to ptr
  %v18887 = load i64, ptr %t1761, align 1
  %t1762 = icmp eq i64 %v18887, 1
  br i1 %t1762, label %block15571, label %block15572

edge2090:                                         ; preds = %block15572
  %v18897 = load i64, ptr %ss1, align 8
  %v18898 = load i64, ptr %t606, align 8
  %v18899 = load i64, ptr %t607, align 8
  %.pre21799 = add i64 %v18897, %v18769
  %.pre21800 = inttoptr i64 %.pre21799 to ptr
  br label %block15571

edge2093:                                         ; preds = %block15570
  %t1786 = inttoptr i64 %v18880 to ptr
  %v18904 = load i64, ptr %t1786, align 1
  %v18905 = add i64 %v18904, 1
  store i64 %v18905, ptr %t1786, align 1
  br label %block15576

edge2098:                                         ; preds = %block15576
  %v18910 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v18832)
  %t1790.not = icmp eq i64 %v18910, 0
  br i1 %t1790.not, label %edge2102, label %block15567

edge2102:                                         ; preds = %edge2098, %block15576
  %v18988 = shl i64 %v12940, 1
  %v19006 = and i64 %v18988, 65534
  %v19011 = or disjoint i64 %v19006, 1
  %t1793.not = icmp ult i64 %v12940, %v18771
  br i1 %t1793.not, label %edge2178, label %edge2112

edge2112:                                         ; preds = %edge2102
  br i1 %t1785.not58, label %edge2127, label %edge2114

edge2114:                                         ; preds = %edge2112
  %t1798 = inttoptr i64 %v18880 to ptr
  %v19198 = load i64, ptr %t1798, align 1
  %t1799 = icmp eq i64 %v19198, 1
  %v19206 = shl nuw nsw i64 %v19006, 1
  %v19207 = add i64 %v18875, %v19206
  %t1803 = inttoptr i64 %v19207 to ptr
  br i1 %t1799, label %edge2118, label %edge2127.thread

edge2118:                                         ; preds = %edge2114
  store i16 0, ptr %t1803, align 1
  %v19228 = shl nuw nsw i64 %v19011, 1
  %v19229 = add i64 %v18875, %v19228
  %t1806 = inttoptr i64 %v19229 to ptr
  br label %edge3980.sink.split

edge2127:                                         ; preds = %edge2112
  %v19248 = shl nuw nsw i64 %v19006, 1
  %v19249 = add i64 %v18875, %v19248
  %t1813 = inttoptr i64 %v19249 to ptr
  %t1814 = load i16, ptr %t1813, align 1
  %t1815 = icmp eq i16 %t1814, 0
  br i1 %t1815, label %edge2150.thread, label %block15737

edge2127.thread:                                  ; preds = %edge2114
  %t1814511 = load i16, ptr %t1803, align 1
  %t1815512 = icmp eq i16 %t1814511, 0
  br i1 %t1815512, label %block15735, label %block15737

edge2139:                                         ; preds = %block15735
  %t1843 = inttoptr i64 %v19259 to ptr
  %v19283 = load i64, ptr %t1843, align 1
  %v19284 = add i64 %v19283, 1
  store i64 %v19284, ptr %t1843, align 1
  br label %block15741

edge2144:                                         ; preds = %block15741
  %v19289 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v18880)
  %t1847.not = icmp eq i64 %v19289, 0
  br i1 %t1847.not, label %edge2150, label %block15712

edge2150:                                         ; preds = %block15741, %edge2144
  %v19349 = shl nuw nsw i64 %v19011, 1
  %v19350 = add i64 %v19254, %v19349
  %t1852 = inttoptr i64 %v19350 to ptr
  %t1853 = load i16, ptr %t1852, align 1
  %t1854 = icmp eq i16 %t1853, 0
  br i1 %t1854, label %edge3980, label %edge2153

edge2150.thread:                                  ; preds = %edge2127
  %v1934922072 = shl nuw nsw i64 %v19011, 1
  %v1935022073 = add i64 %v18875, %v1934922072
  %t185222074 = inttoptr i64 %v1935022073 to ptr
  %t185322075 = load i16, ptr %t185222074, align 1
  %t185422076 = icmp eq i16 %t185322075, 0
  br i1 %t185422076, label %edge3980, label %block15752

edge2153:                                         ; preds = %edge2150
  br i1 %t1842.not90, label %block15752, label %edge2154

edge2154:                                         ; preds = %edge2153
  %t1857 = inttoptr i64 %v19259 to ptr
  %v19367 = load i64, ptr %t1857, align 1
  %t1858 = icmp eq i64 %v19367, 1
  br i1 %t1858, label %edge3980.sink.split, label %block15752

edge2162:                                         ; preds = %block15750
  %t1882 = inttoptr i64 %v19379 to ptr
  %v19384 = load i64, ptr %t1882, align 1
  %v19385 = add i64 %v19384, 1
  store i64 %v19385, ptr %t1882, align 1
  br label %block15756

edge2167:                                         ; preds = %block15756
  %v19390 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1925921866218772207922092)
  %t1886.not = icmp eq i64 %v19390, 0
  br i1 %t1886.not, label %edge3980, label %block15747

edge2178:                                         ; preds = %edge2102
  br i1 %t1457, label %edge3097, label %edge2183

edge2183:                                         ; preds = %edge2178
  br i1 %t23.not, label %block913.preheader, label %edge2184

edge2184:                                         ; preds = %edge2183
  %v19788 = load i64, ptr %t910, align 1
  %v19789 = add i64 %v19788, 1
  store i64 %v19789, ptr %t910, align 1
  br label %block913.preheader

block913.preheader:                               ; preds = %edge2183, %edge2184
  br label %block913

edge2227:                                         ; preds = %block913
  %v20124 = sub nsw i64 %v12940, %v19907
  %v20130 = and i64 %v20124, 32767
  %spec.select333 = call i64 @llvm.usub.sat.i64(i64 %v218770, i64 %v20130)
  %v20335 = add i64 %v19919, %spec.select333
  %t1904 = icmp ult i64 %v20335, %v1397
  br i1 %t1904, label %edge2251, label %edge2249

edge2249:                                         ; preds = %edge2227, %block16384
  %v20788 = phi i64 [ %v20540, %block16384 ], [ 0, %edge2227 ]
  %v20562 = add i64 %v19919, %v218770
  %t1914 = icmp ult i64 %v20562, %v1397
  br i1 %t1914, label %edge2270, label %edge2268

edge2251:                                         ; preds = %edge2227
  br i1 %v1062051.not1294, label %edge2256, label %block16384

edge2256:                                         ; preds = %edge2251
  br i1 %t975, label %trap1911, label %ok1911

edge2268:                                         ; preds = %edge2249, %block16527
  %v20789 = phi i64 [ %v20766, %block16527 ], [ 0, %edge2249 ]
  %t1924 = icmp eq i64 %v20788, %v20789
  br i1 %t1924, label %edge2292, label %edge2741

edge2270:                                         ; preds = %edge2249
  br i1 %v1062051.not1294, label %edge2275, label %block16527

edge2275:                                         ; preds = %edge2270
  br i1 %t975, label %trap1921, label %ok1921

edge2285:                                         ; preds = %edge2590, %edge2506, %edge2507, %edge2510, %edge2451, %edge2453, %edge2456
  %v24738 = phi i64 [ %umax, %edge2456 ], [ %v24711, %edge2590 ], [ %v22059, %edge2506 ], [ %v22059, %edge2507 ], [ %v22059, %edge2510 ], [ %umax, %edge2451 ], [ %umax, %edge2453 ]
  %t2040 = icmp ugt i64 %v24738, %v19922
  br i1 %t2040, label %edge2607, label %edge2741

edge2292:                                         ; preds = %edge2268
  %v20968 = add nuw nsw i64 %v19919, 1
  %t1932 = icmp ult i64 %spec.select333, %v218770
  br i1 %t1932, label %edge2405, label %edge2313

edge2313:                                         ; preds = %edge2292
  br i1 %t23.not, label %block16738, label %edge2316

edge2316:                                         ; preds = %edge2313
  %v21066 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t1935.not = icmp eq i64 %v21066, 0
  br i1 %t1935.not, label %block16738, label %block16670

edge2322:                                         ; preds = %block16738
  %v21070 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219198)
  %t1937.not = icmp eq i64 %v21070, 0
  br i1 %t1937.not, label %block16742, label %block16670

edge2328:                                         ; preds = %block16742
  %v21074 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219213)
  %t1939.not = icmp eq i64 %v21074, 0
  br i1 %t1939.not, label %block16746, label %block16670

edge2334:                                         ; preds = %block16746
  %v21078 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219226)
  %t1941.not = icmp eq i64 %v21078, 0
  br i1 %t1941.not, label %block16750, label %block16670

edge2340:                                         ; preds = %block16750
  %v21082 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219232)
  %t1943.not = icmp eq i64 %v21082, 0
  br i1 %t1943.not, label %block16754, label %block16670

edge2346:                                         ; preds = %block16754
  %v21086 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219242)
  %t1945.not = icmp eq i64 %v21086, 0
  br i1 %t1945.not, label %block16762, label %block16670

edge2358:                                         ; preds = %block16762
  %v21094 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t1949.not = icmp eq i64 %v21094, 0
  br i1 %t1949.not, label %block16766, label %block16670

edge2364:                                         ; preds = %block16766
  %v21098 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10791.ph.ph)
  %t1951.not = icmp eq i64 %v21098, 0
  br i1 %t1951.not, label %block16770, label %block16670

edge2370:                                         ; preds = %block16770
  %v21102 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10792)
  %t1953.not = icmp eq i64 %v21102, 0
  br i1 %t1953.not, label %block16774, label %block16670

edge2376:                                         ; preds = %block16774
  %v21106 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10793.ph.ph)
  %t1955.not = icmp eq i64 %v21106, 0
  br i1 %t1955.not, label %block16778, label %block16670

edge2382:                                         ; preds = %block16778
  %v21110 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10794)
  %t1957.not = icmp eq i64 %v21110, 0
  br i1 %t1957.not, label %block16782, label %block16670

edge2388:                                         ; preds = %block16782
  %v21114 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v19849)
  %t1959.not = icmp eq i64 %v21114, 0
  br i1 %t1959.not, label %block16786, label %block16670

edge2394:                                         ; preds = %block16786
  %v21118 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v19861)
  %t1961.not = icmp eq i64 %v21118, 0
  br i1 %t1961.not, label %block16790, label %block16670

edge2400:                                         ; preds = %block16790
  %v21122 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t1963.not = icmp eq i64 %v21122, 0
  br i1 %t1963.not, label %common.ret, label %block16670

edge2405:                                         ; preds = %edge2292
  br i1 %t23.not, label %edge2427.preheader, label %edge2407

edge2407:                                         ; preds = %edge2405
  %v21700 = load i64, ptr %t910, align 1
  %v21701 = add i64 %v21700, 1
  store i64 %v21701, ptr %t910, align 1
  br label %edge2427.preheader

edge2427.preheader:                               ; preds = %edge2405, %edge2407
  br label %edge2427

edge2427:                                         ; preds = %edge2427.preheader, %edge2571
  %v21750 = phi i64 [ %v21809, %edge2571 ], [ %v20968, %edge2427.preheader ]
  %v21809 = add nuw nsw i64 %v21750, 8
  %t1975.not = icmp ugt i64 %v21809, %spec.select18
  br i1 %t1975.not, label %block1017.preheader, label %edge2529

block1017.preheader:                              ; preds = %edge2427
  %umax = call i64 @llvm.umax.i64(i64 %spec.select18, i64 %v21750)
  br label %block1017

edge2451:                                         ; preds = %block1017
  br i1 %t23.not, label %edge2285, label %edge2453

edge2453:                                         ; preds = %edge2451
  %v22080 = load i64, ptr %t910, align 1
  %v22081 = add i64 %v22080, -1
  store i64 %v22081, ptr %t910, align 1
  %t1984.not = icmp eq i64 %v22081, 0
  br i1 %t1984.not, label %edge2456, label %edge2285

edge2456:                                         ; preds = %edge2453
  %v22082 = call i64 @mech_compact_free(i64 %v0, i64 %v1401)
  %t1985.not = icmp eq i64 %v22082, 0
  br i1 %t1985.not, label %edge2285, label %edge2457

edge2457:                                         ; preds = %edge2456
  %t1986 = insertvalue { i64, i64 } poison, i64 %v22082, 0
  %t1987 = insertvalue { i64, i64 } %t1986, i64 1032, 1
  br label %common.ret

edge2465:                                         ; preds = %block1017
  %v22270 = add i64 %v22059, %v218770
  %t1988 = icmp ult i64 %v22270, %v1397
  br i1 %t1988, label %edge2473, label %edge2471

edge2471:                                         ; preds = %edge2465, %block17271
  %v22738 = phi i64 [ %v22483, %block17271 ], [ 0, %edge2465 ]
  %v22507 = add i64 %v22059, %spec.select333
  %t1998 = icmp ult i64 %v22507, %v1397
  br i1 %t1998, label %edge2492, label %edge2490

edge2473:                                         ; preds = %edge2465
  br i1 %v1062051.not1294, label %edge2478, label %block17271

edge2478:                                         ; preds = %edge2473
  br i1 %t975, label %trap1995, label %ok1995

edge2490:                                         ; preds = %edge2471, %block17418
  %v22739 = phi i64 [ %v22716, %block17418 ], [ 0, %edge2471 ]
  %t2008 = icmp eq i64 %v22738, %v22739
  br i1 %t2008, label %edge2517, label %edge2506

edge2492:                                         ; preds = %edge2471
  br i1 %v1062051.not1294, label %edge2497, label %block17418

edge2497:                                         ; preds = %edge2492
  br i1 %t975, label %trap2005, label %ok2005

edge2506:                                         ; preds = %edge2490
  br i1 %t23.not, label %edge2285, label %edge2507

edge2507:                                         ; preds = %edge2506
  %v22748 = load i64, ptr %t910, align 1
  %v22749 = add i64 %v22748, -1
  store i64 %v22749, ptr %t910, align 1
  %t2014.not = icmp eq i64 %v22749, 0
  br i1 %t2014.not, label %edge2510, label %edge2285

edge2510:                                         ; preds = %edge2507
  %v22750 = call i64 @mech_compact_free(i64 %v0, i64 %v1401)
  %t2015.not = icmp eq i64 %v22750, 0
  br i1 %t2015.not, label %edge2285, label %edge2511

edge2511:                                         ; preds = %edge2510
  %t2016 = insertvalue { i64, i64 } poison, i64 %v22750, 0
  %t2017 = insertvalue { i64, i64 } %t2016, i64 1068, 1
  br label %common.ret

edge2517:                                         ; preds = %edge2490
  %v22922 = add nuw nsw i64 %v22059, 1
  br label %block1017

edge2529:                                         ; preds = %edge2427
  %v22967 = add i64 %v21750, %v218770
  %t2018.not = icmp ugt i64 %v22967, %v23159
  br i1 %t2018.not, label %edge2550, label %edge2541

edge2541:                                         ; preds = %edge2529
  br i1 %v1062051.not1294, label %edge2550.thread, label %edge2550.thread533

edge2550:                                         ; preds = %edge2529
  %v23757 = add i64 %v21750, %spec.select333
  %t2025.not = icmp ugt i64 %v23757, %v23159
  br i1 %t2025.not, label %edge2571, label %edge2562

edge2550.thread533:                               ; preds = %edge2541
  %v23350 = add i64 %v22967, %v1396
  %t2024 = inttoptr i64 %v23350 to ptr
  %v23351 = load i64, ptr %t2024, align 1
  %v23757535 = add i64 %v21750, %spec.select333
  %t2025.not536 = icmp ugt i64 %v23757535, %v23159
  br i1 %t2025.not536, label %edge2571, label %edge2563

edge2550.thread:                                  ; preds = %edge2541
  %v23353 = call i64 @mech_compact_matrix_word(i64 %v1396, i64 %v22967, i64 %v1399, i64 %v1400, i64 8)
  %v23757524 = add i64 %v21750, %spec.select333
  %t2025.not525 = icmp ugt i64 %v23757524, %v23159
  br i1 %t2025.not525, label %edge2571, label %edge2564

edge2562:                                         ; preds = %edge2550
  br i1 %v1062051.not1294, label %edge2564, label %edge2563

edge2563:                                         ; preds = %edge2550.thread533, %edge2562
  %v23733526541 = phi i64 [ 0, %edge2562 ], [ %v23351, %edge2550.thread533 ]
  %v23757528540 = phi i64 [ %v23757, %edge2562 ], [ %v23757535, %edge2550.thread533 ]
  %v24159 = add i64 %v23757528540, %v1396
  %t2031 = inttoptr i64 %v24159 to ptr
  %v24160 = load i64, ptr %t2031, align 1
  br label %edge2571

edge2564:                                         ; preds = %edge2550.thread, %edge2562
  %v23733526532 = phi i64 [ 0, %edge2562 ], [ %v23353, %edge2550.thread ]
  %v23757528531 = phi i64 [ %v23757, %edge2562 ], [ %v23757524, %edge2550.thread ]
  %v24162 = call i64 @mech_compact_matrix_word(i64 %v1396, i64 %v23757528531, i64 %v1399, i64 %v1400, i64 8)
  br label %edge2571

edge2571:                                         ; preds = %edge2550.thread533, %edge2550.thread, %edge2550, %edge2563, %edge2564
  %v23733527 = phi i64 [ %v23733526532, %edge2564 ], [ %v23733526541, %edge2563 ], [ 0, %edge2550 ], [ %v23353, %edge2550.thread ], [ %v23351, %edge2550.thread533 ]
  %v24546 = phi i64 [ %v24162, %edge2564 ], [ %v24160, %edge2563 ], [ 0, %edge2550 ], [ 0, %edge2550.thread ], [ 0, %edge2550.thread533 ]
  %t2032 = icmp eq i64 %v24546, %v23733527
  br i1 %t2032, label %edge2427, label %edge2574

edge2574:                                         ; preds = %edge2571
  %v24568.le = xor i64 %v24546, %v23733527
  br i1 %t23.not, label %edge2590, label %edge2577

edge2577:                                         ; preds = %edge2574
  %v24581 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t2035.not = icmp eq i64 %v24581, 0
  br i1 %t2035.not, label %edge2590, label %edge2578

edge2578:                                         ; preds = %edge2577
  %t2036 = insertvalue { i64, i64 } poison, i64 %v24581, 0
  %t2037 = insertvalue { i64, i64 } %t2036, i64 1217, 1
  br label %common.ret

edge2590:                                         ; preds = %edge2577, %edge2574
  %t2038 = call range(i64 0, 65) i64 @llvm.cttz.i64(i64 %v24568.le, i1 true)
  %v24706 = lshr i64 %t2038, 3
  %v24711 = add nuw nsw i64 %v24706, %v21750
  br label %edge2285

edge2605:                                         ; preds = %block18496, %edge2633
  %spec.select20 = add nuw nsw i64 %v19868, 1
  %t2087.not = icmp ult i64 %v24738, %spec.select18
  br i1 %t2087.not, label %edge2741, label %edge2671

edge2607:                                         ; preds = %edge2285
  %v24972 = sub i64 %v218770, %spec.select333
  %v24990 = and i64 %v24738, 65535
  %v25008 = shl i64 %v24972, 16
  %v25008.masked = and i64 %v25008, 4294901760
  %v25032 = or disjoint i64 %v24990, %v25008.masked
  %t2044 = icmp ult i64 %v19868, %v3720
  br i1 %t2044, label %edge2616, label %block18487

edge2616:                                         ; preds = %edge2607
  %v25137 = shl nuw nsw i64 %v19868, 2
  %v25138 = add i64 %v25137, %v19856
  %t2046 = inttoptr i64 %v25138 to ptr
  %t2047 = load i32, ptr %t2046, align 1
  %v25139 = zext i32 %t2047 to i64
  %t2048 = icmp eq i64 %v25032, %v25139
  br i1 %t2048, label %block18490, label %edge2619

edge2619:                                         ; preds = %edge2616
  %t2050.not = icmp eq i64 %v19861, 0
  br i1 %t2050.not, label %block18492, label %edge2620

edge2620:                                         ; preds = %edge2619
  %t2051 = inttoptr i64 %v19861 to ptr
  %v25155 = load i64, ptr %t2051, align 1
  %t2052 = icmp eq i64 %v25155, 1
  br i1 %t2052, label %block18491, label %block18492

edge2625:                                         ; preds = %block18492
  %v25165 = load i64, ptr %ss1, align 8
  %v25166 = load i64, ptr %t606, align 8
  %v25167 = load i64, ptr %t607, align 8
  %.pre21795 = add i64 %v25165, %v25137
  %.pre21796 = inttoptr i64 %.pre21795 to ptr
  br label %block18491

edge2628:                                         ; preds = %block18490
  %t2076 = inttoptr i64 %v25148 to ptr
  %v25172 = load i64, ptr %t2076, align 1
  %v25173 = add i64 %v25172, 1
  store i64 %v25173, ptr %t2076, align 1
  br label %block18496

edge2633:                                         ; preds = %block18496
  %v25178 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v19861)
  %t2080.not = icmp eq i64 %v25178, 0
  br i1 %t2080.not, label %edge2605, label %block18487

edge2671:                                         ; preds = %edge2605
  %v25802 = shl nuw nsw i64 %v19907, 2
  %v25818 = and i64 %v25802, 131068
  %v25842 = add i64 %v25818, %v19844
  %t2092 = inttoptr i64 %v25842 to ptr
  %t2093 = load i16, ptr %t2092, align 1
  %v25890 = add i64 %v19844, 2
  %v25891 = add i64 %v25890, %v25818
  %t2098 = inttoptr i64 %v25891 to ptr
  %t2099 = load i16, ptr %t2098, align 1
  %t2102.not = icmp eq i64 %v19849, 0
  br i1 %t2102.not, label %edge2692, label %edge2679

edge2679:                                         ; preds = %edge2671
  %t2104 = inttoptr i64 %v19849 to ptr
  %v25925 = load i64, ptr %t2104, align 1
  %t2105 = icmp eq i64 %v25925, 1
  %v25933 = shl nuw nsw i64 %v19901, 1
  %v25934 = add i64 %v25933, %v19844
  %t2109 = inttoptr i64 %v25934 to ptr
  br i1 %t2105, label %edge2683, label %edge2692.thread

edge2683:                                         ; preds = %edge2679
  store i16 %t2093, ptr %t2109, align 1
  %v25955 = shl nuw nsw i64 %v19904, 1
  %v25956 = add i64 %v25955, %v19844
  %t2112 = inttoptr i64 %v25956 to ptr
  store i16 %t2099, ptr %t2112, align 1
  br label %block19858

edge2692:                                         ; preds = %edge2671
  %v25975 = shl nuw nsw i64 %v19901, 1
  %v25976 = add i64 %v25975, %v19844
  %t2119 = inttoptr i64 %v25976 to ptr
  %t2120 = load i16, ptr %t2119, align 1
  %t2121 = icmp eq i16 %t2120, %t2093
  br i1 %t2121, label %block18944.thread, label %block18946

edge2692.thread:                                  ; preds = %edge2679
  %t2120561 = load i16, ptr %t2109, align 1
  %t2121562 = icmp eq i16 %t2120561, %t2093
  br i1 %t2121562, label %block18944.thread, label %block18946

edge2704:                                         ; preds = %block18944
  %t2149 = inttoptr i64 %v26005 to ptr
  %v26010 = load i64, ptr %t2149, align 1
  %v26011 = add i64 %v26010, 1
  store i64 %v26011, ptr %t2149, align 1
  br label %block18950

edge2709:                                         ; preds = %block18950
  %v26016 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v19849)
  %t2153.not = icmp eq i64 %v26016, 0
  br i1 %t2153.not, label %edge2715, label %block18921

edge2715:                                         ; preds = %block18950, %edge2709
  %v26053 = shl nuw nsw i64 %v19904, 1
  %v26054 = add i64 %v2598121906, %v26053
  %t2158 = inttoptr i64 %v26054 to ptr
  %t2159 = load i16, ptr %t2158, align 1
  %t2160 = icmp eq i16 %t2159, %t2099
  br i1 %t2160, label %block19858, label %edge2718

edge2718:                                         ; preds = %edge2715
  br i1 %t2148.not8621911, label %block18961, label %edge2719

edge2719:                                         ; preds = %edge2718
  %t2163 = inttoptr i64 %v2598621908 to ptr
  %v26071 = load i64, ptr %t2163, align 1
  %t2164 = icmp eq i64 %v26071, 1
  br i1 %t2164, label %block18959, label %block18961

edge2724:                                         ; preds = %block18961
  %v26081 = load i64, ptr %ss1, align 8
  %v26082 = load i64, ptr %t606, align 8
  %v26083 = load i64, ptr %t607, align 8
  %.pre21793 = add i64 %v26081, %v26053
  %.pre21794 = inttoptr i64 %.pre21793 to ptr
  br label %block18959

edge2727:                                         ; preds = %block18959
  %t2188 = inttoptr i64 %v26070 to ptr
  %v26088 = load i64, ptr %t2188, align 1
  %v26089 = add i64 %v26088, 1
  store i64 %v26089, ptr %t2188, align 1
  br label %block18965

edge2732:                                         ; preds = %block18965
  %v26094 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v2598621908)
  %t2192.not = icmp eq i64 %v26094, 0
  br i1 %t2192.not, label %block19858, label %block18956

edge2741:                                         ; preds = %edge2285, %edge2268, %edge2605
  %spec.select20557 = phi i64 [ %spec.select20, %edge2605 ], [ %v19868, %edge2268 ], [ %v19868, %edge2285 ]
  %v26108556 = phi i64 [ %v25148, %edge2605 ], [ %v19861, %edge2268 ], [ %v19861, %edge2285 ]
  %v26105555 = phi i64 [ %v25145, %edge2605 ], [ %v19858, %edge2268 ], [ %v19858, %edge2285 ]
  %v26103554 = phi i64 [ %v25143, %edge2605 ], [ %v19856, %edge2268 ], [ %v19856, %edge2285 ]
  %v26146553 = phi i64 [ %v24738, %edge2605 ], [ %v19922, %edge2268 ], [ %v19922, %edge2285 ]
  %v24738521552 = phi i64 [ %v24738, %edge2605 ], [ %v19919, %edge2268 ], [ %v24738, %edge2285 ]
  %v26180 = add i64 %v24738521552, %spec.select333
  %t2195 = icmp ult i64 %v26180, %v1397
  br i1 %t2195, label %edge2749, label %edge2747

edge2747:                                         ; preds = %edge2741, %block19127
  %v26682 = phi i64 [ %v26396, %block19127 ], [ 0, %edge2741 ]
  %v26456 = add i64 %v24738521552, %v218770
  %t2205 = icmp ult i64 %v26456, %v1397
  br i1 %t2205, label %edge2768, label %edge2766.thread

edge2766.thread:                                  ; preds = %edge2747
  %v26694569 = shl nuw nsw i64 %v19907, 1
  %v26699570 = and i64 %v26694569, 65534
  br label %edge2803

edge2749:                                         ; preds = %edge2741
  br i1 %v1062051.not1294, label %edge2754, label %block19127

edge2754:                                         ; preds = %edge2749
  br i1 %t975, label %trap2202, label %ok2202

edge2766:                                         ; preds = %ok2212, %edge2768
  %v26652 = phi i64 [ %v26657, %ok2212 ], [ %v26456, %edge2768 ]
  %v26659 = add i64 %v26652, %v1396
  %t2213 = inttoptr i64 %v26659 to ptr
  %t2214 = load i8, ptr %t2213, align 1
  %v26660 = zext i8 %t2214 to i64
  %15 = icmp samesign ult i64 %v26682, %v26660
  %v26694 = shl nuw nsw i64 %v19907, 1
  %v26699 = and i64 %v26694, 65534
  %v26704 = or disjoint i64 %v26699, 1
  br i1 %15, label %edge2803, label %edge2794

edge2768:                                         ; preds = %edge2747
  br i1 %v1062051.not1294, label %edge2773, label %edge2766

edge2773:                                         ; preds = %edge2768
  br i1 %t975, label %trap2212, label %ok2212

edge2794:                                         ; preds = %edge2766
  br label %edge2803

edge2803:                                         ; preds = %edge2766.thread, %edge2794, %edge2766
  %v26896 = phi i64 [ %v19901, %edge2766 ], [ %v19904, %edge2766.thread ], [ %v19904, %edge2794 ]
  %v26899 = phi i64 [ %v26704, %edge2766 ], [ %v26699570, %edge2766.thread ], [ %v26699, %edge2794 ]
  %v26902 = phi i64 [ %v26704, %edge2766 ], [ %v19901, %edge2766.thread ], [ %v19901, %edge2794 ]
  %v26905 = phi i64 [ %v19904, %edge2766 ], [ %v26699570, %edge2766.thread ], [ %v26699, %edge2794 ]
  %v26908 = phi i64 [ %v24738521552, %edge2766 ], [ %v19913, %edge2766.thread ], [ %v19913, %edge2794 ]
  %v26911 = phi i64 [ %v19916, %edge2766 ], [ %v24738521552, %edge2766.thread ], [ %v24738521552, %edge2794 ]
  %v26914 = phi i64 [ %v19916, %edge2766 ], [ %v19913, %edge2766.thread ], [ %v19913, %edge2794 ]
  %v26927 = shl nuw nsw i64 %v26896, 1
  %v26928 = add i64 %v26927, %v19844
  %t2220 = inttoptr i64 %v26928 to ptr
  %t2221 = load i16, ptr %t2220, align 1
  %v26929 = zext i16 %t2221 to i64
  %t2222 = icmp eq i64 %v19907, %v26929
  br i1 %t2222, label %block19427, label %edge2806

edge2806:                                         ; preds = %edge2803
  %t2224.not = icmp eq i64 %v19849, 0
  br i1 %t2224.not, label %block19429, label %edge2807

edge2807:                                         ; preds = %edge2806
  %t2225 = inttoptr i64 %v19849 to ptr
  %v26945 = load i64, ptr %t2225, align 1
  %t2226 = icmp eq i64 %v26945, 1
  br i1 %t2226, label %block19428, label %block19429

edge2812:                                         ; preds = %block19429
  %v26955 = load i64, ptr %ss1, align 8
  %v26956 = load i64, ptr %t606, align 8
  %v26957 = load i64, ptr %t607, align 8
  %.pre21791 = add i64 %v26955, %v26927
  %.pre21792 = inttoptr i64 %.pre21791 to ptr
  br label %block19428

edge2815:                                         ; preds = %block19427
  %t2250 = inttoptr i64 %v26938 to ptr
  %v26962 = load i64, ptr %t2250, align 1
  %v26963 = add i64 %v26962, 1
  store i64 %v26963, ptr %t2250, align 1
  br label %block19433

edge2820:                                         ; preds = %block19433
  %v26968 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v19849)
  %t2254.not = icmp eq i64 %v26968, 0
  br i1 %t2254.not, label %edge2826, label %block19424

edge2826:                                         ; preds = %block19433, %edge2820
  %v26979 = shl nuw nsw i64 %v26899, 1
  %v26980 = add i64 %v26933, %v26979
  %t2259 = inttoptr i64 %v26980 to ptr
  %t2260 = load i16, ptr %t2259, align 1
  %v26981 = zext i16 %t2260 to i64
  %spec.select21 = call i64 @llvm.umin.i64(i64 %v26914, i64 %v24738521552)
  %v27177 = add nsw i64 %v19910, -1
  %t2266 = icmp ult i64 %v12940, %v26981
  %t2267 = icmp ne i64 %v27177, 0
  %v2721081.not = and i1 %t2267, %t2266
  br i1 %v2721081.not, label %block913, label %edge2847

edge2847:                                         ; preds = %edge2826
  br i1 %t2249.not79, label %edge2862, label %edge2849

edge2849:                                         ; preds = %edge2847
  %t2272 = inttoptr i64 %v26938 to ptr
  %v27391 = load i64, ptr %t2272, align 1
  %t2273 = icmp eq i64 %v27391, 1
  %v27399 = shl nuw nsw i64 %v26902, 1
  %v27400 = add i64 %v26933, %v27399
  %t2277 = inttoptr i64 %v27400 to ptr
  br i1 %t2273, label %edge2853, label %edge2862.thread

edge2853:                                         ; preds = %edge2849
  store i16 0, ptr %t2277, align 1
  %v27421 = shl nuw nsw i64 %v26905, 1
  %v27422 = add i64 %v26933, %v27421
  %t2280 = inttoptr i64 %v27422 to ptr
  store i16 0, ptr %t2280, align 1
  br label %block19858

edge2862:                                         ; preds = %edge2847
  %v27441 = shl nuw nsw i64 %v26902, 1
  %v27442 = add i64 %v26933, %v27441
  %t2287 = inttoptr i64 %v27442 to ptr
  %t2288 = load i16, ptr %t2287, align 1
  %t2289 = icmp eq i16 %t2288, 0
  br i1 %t2289, label %edge2885.thread, label %block19735

edge2862.thread:                                  ; preds = %edge2849
  %t2288579 = load i16, ptr %t2277, align 1
  %t2289580 = icmp eq i16 %t2288579, 0
  br i1 %t2289580, label %block19733, label %block19735

edge2874:                                         ; preds = %block19733
  %t2317 = inttoptr i64 %v27452 to ptr
  %v27476 = load i64, ptr %t2317, align 1
  %v27477 = add i64 %v27476, 1
  store i64 %v27477, ptr %t2317, align 1
  br label %block19739

edge2879:                                         ; preds = %block19739
  %v27482 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v26938)
  %t2321.not = icmp eq i64 %v27482, 0
  br i1 %t2321.not, label %edge2885, label %block19710

edge2885:                                         ; preds = %block19739, %edge2879
  %v27508 = shl nuw nsw i64 %v26905, 1
  %v27509 = add i64 %v27447, %v27508
  %t2326 = inttoptr i64 %v27509 to ptr
  %t2327 = load i16, ptr %t2326, align 1
  %t2328 = icmp eq i16 %t2327, 0
  br i1 %t2328, label %block19858, label %edge2888

edge2885.thread:                                  ; preds = %edge2862
  %v2750822100 = shl nuw nsw i64 %v26905, 1
  %v2750922101 = add i64 %v26933, %v2750822100
  %t232622102 = inttoptr i64 %v2750922101 to ptr
  %t232722103 = load i16, ptr %t232622102, align 1
  %t232822104 = icmp eq i16 %t232722103, 0
  br i1 %t232822104, label %block19858, label %block19750

edge2888:                                         ; preds = %edge2885
  br i1 %t2316.not82, label %block19750, label %edge2889

edge2889:                                         ; preds = %edge2888
  %t2331 = inttoptr i64 %v27452 to ptr
  %v27526 = load i64, ptr %t2331, align 1
  %t2332 = icmp eq i64 %v27526, 1
  br i1 %t2332, label %block19754.thread22297, label %block19750

block19754.thread22297:                           ; preds = %edge2889
  store i16 0, ptr %t2326, align 1
  br label %block19858

edge2897:                                         ; preds = %block19748
  %t2356 = inttoptr i64 %v27538 to ptr
  %v27543 = load i64, ptr %t2356, align 1
  %v27544 = add i64 %v27543, 1
  store i64 %v27544, ptr %t2356, align 1
  br label %block19754

edge2902:                                         ; preds = %block19754
  %v27549 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v2745221938219492210722120)
  %t2360.not = icmp eq i64 %v27549, 0
  br i1 %t2360.not, label %block19858, label %block19745

edge2952:                                         ; preds = %block19858
  %v27753 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t2366.not = icmp eq i64 %v27753, 0
  br i1 %t2366.not, label %edge3980, label %edge2953

edge2953:                                         ; preds = %edge2952
  %t2367 = insertvalue { i64, i64 } poison, i64 %v27753, 0
  %t2368 = insertvalue { i64, i64 } %t2367, i64 1403, 1
  br label %common.ret

edge3097:                                         ; preds = %edge2178
  br i1 %t23.not, label %edge3105, label %edge3098

edge3098:                                         ; preds = %edge3097
  %v28499 = load i64, ptr %t910, align 1
  %v28500 = add i64 %v28499, 1
  store i64 %v28500, ptr %t910, align 1
  br label %edge3105

edge3105:                                         ; preds = %edge3097, %edge3098
  %v29012 = and i64 %v218774, 65535
  %v12724 = lshr i64 %v218774, 16
  %v29019 = and i64 %v12724, 65535
  %t2418 = icmp samesign ugt i64 %v29019, 1
  %v29214 = add nsw i64 %v29019, -1
  %v29408 = call i64 @llvm.umin.i64(i64 %spec.select18, i64 %v29214)
  br label %block1421

edge3132:                                         ; preds = %block1421
  %v28795 = sub nsw i64 %v12940, %v28582
  %v28801 = and i64 %v28795, 32767
  %spec.select334 = call i64 @llvm.usub.sat.i64(i64 %v218770, i64 %v28801)
  %v29006 = sub i64 %v218770, %spec.select334
  %t2417 = icmp eq i64 %v29006, %v29012
  %v2903760 = and i1 %t2418, %t2417
  %t24241298 = icmp ugt i64 %v29214, %v28594
  %t2424 = select i1 %v2903760, i1 %t24241298, i1 false
  br i1 %t2424, label %edge3233, label %edge3178

edge3178:                                         ; preds = %edge3132
  %v29614 = add i64 %v28594, %spec.select334
  %t2427 = icmp ult i64 %v29614, %v1397
  br i1 %t2427, label %edge3186, label %edge3184

edge3184:                                         ; preds = %edge3178, %block20948
  %v30066 = phi i64 [ %v29818, %block20948 ], [ 0, %edge3178 ]
  %v29840 = add i64 %v28594, %v218770
  %t2437 = icmp ult i64 %v29840, %v1397
  br i1 %t2437, label %edge3205, label %edge3203

edge3186:                                         ; preds = %edge3178
  br i1 %v1062051.not1294, label %edge3191, label %block20948

edge3191:                                         ; preds = %edge3186
  br i1 %t975, label %trap2434, label %ok2434

edge3203:                                         ; preds = %edge3184, %block21091
  %v30067 = phi i64 [ %v30044, %block21091 ], [ 0, %edge3184 ]
  %t2447.not = icmp eq i64 %v30066, %v30067
  %v30259 = add nuw nsw i64 %v28594, 1
  br i1 %t2447.not, label %edge3233, label %block1799

edge3205:                                         ; preds = %edge3184
  br i1 %v1062051.not1294, label %edge3210, label %block21091

edge3210:                                         ; preds = %edge3205
  br i1 %t975, label %trap2444, label %ok2444

edge3233:                                         ; preds = %edge3132, %edge3203
  %v218807 = phi i64 [ %v29408, %edge3132 ], [ %v30259, %edge3203 ]
  %t2455 = icmp ult i64 %spec.select334, %v218770
  br i1 %t2455, label %edge3346, label %edge3254

edge3254:                                         ; preds = %edge3233
  br i1 %t23.not, label %block21436, label %edge3257

edge3257:                                         ; preds = %edge3254
  %v30532 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t2458.not = icmp eq i64 %v30532, 0
  br i1 %t2458.not, label %block21436, label %block21368

edge3263:                                         ; preds = %block21436
  %v30536 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219198)
  %t2460.not = icmp eq i64 %v30536, 0
  br i1 %t2460.not, label %block21440, label %block21368

edge3269:                                         ; preds = %block21440
  %v30540 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219213)
  %t2462.not = icmp eq i64 %v30540, 0
  br i1 %t2462.not, label %block21444, label %block21368

edge3275:                                         ; preds = %block21444
  %v30544 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219226)
  %t2464.not = icmp eq i64 %v30544, 0
  br i1 %t2464.not, label %block21448, label %block21368

edge3281:                                         ; preds = %block21448
  %v30548 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219232)
  %t2466.not = icmp eq i64 %v30548, 0
  br i1 %t2466.not, label %block21452, label %block21368

edge3287:                                         ; preds = %block21452
  %v30552 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219242)
  %t2468.not = icmp eq i64 %v30552, 0
  br i1 %t2468.not, label %block21460, label %block21368

edge3299:                                         ; preds = %block21460
  %v30560 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t2472.not = icmp eq i64 %v30560, 0
  br i1 %t2472.not, label %block21464, label %block21368

edge3305:                                         ; preds = %block21464
  %v30564 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10791.ph.ph)
  %t2474.not = icmp eq i64 %v30564, 0
  br i1 %t2474.not, label %block21468, label %block21368

edge3311:                                         ; preds = %block21468
  %v30568 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10792)
  %t2476.not = icmp eq i64 %v30568, 0
  br i1 %t2476.not, label %block21472, label %block21368

edge3317:                                         ; preds = %block21472
  %v30572 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10793.ph.ph)
  %t2478.not = icmp eq i64 %v30572, 0
  br i1 %t2478.not, label %block21476, label %block21368

edge3323:                                         ; preds = %block21476
  %v30576 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10794)
  %t2480.not = icmp eq i64 %v30576, 0
  br i1 %t2480.not, label %block21480, label %block21368

edge3329:                                         ; preds = %block21480
  %v30580 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v18651)
  %t2482.not = icmp eq i64 %v30580, 0
  br i1 %t2482.not, label %block21484, label %block21368

edge3335:                                         ; preds = %block21484
  %v30584 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v28548)
  %t2484.not = icmp eq i64 %v30584, 0
  br i1 %t2484.not, label %block21488, label %block21368

edge3341:                                         ; preds = %block21488
  %v30588 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t2486.not = icmp eq i64 %v30588, 0
  br i1 %t2486.not, label %common.ret, label %block21368

edge3346:                                         ; preds = %edge3233
  br i1 %t23.not, label %edge3368.preheader, label %edge3348

edge3348:                                         ; preds = %edge3346
  %v30599 = load i64, ptr %t910, align 1
  %v30600 = add i64 %v30599, 1
  store i64 %v30600, ptr %t910, align 1
  br label %edge3368.preheader

edge3368.preheader:                               ; preds = %edge3346, %edge3348
  br label %edge3368

edge3368:                                         ; preds = %edge3368.preheader, %edge3512
  %v30649 = phi i64 [ %v30663, %edge3512 ], [ %v218807, %edge3368.preheader ]
  %v30663 = add nuw nsw i64 %v30649, 8
  %t2498.not = icmp ugt i64 %v30663, %spec.select18
  br i1 %t2498.not, label %block1561, label %edge3470

edge3392:                                         ; preds = %block1561
  %umax21627.le = call i64 @llvm.umax.i64(i64 %spec.select18, i64 %v30649)
  br i1 %t23.not, label %edge3550, label %edge3394

edge3394:                                         ; preds = %edge3392
  %v30925 = load i64, ptr %t910, align 1
  %v30926 = add i64 %v30925, -1
  store i64 %v30926, ptr %t910, align 1
  %t2507.not = icmp eq i64 %v30926, 0
  br i1 %t2507.not, label %edge3397, label %edge3550

edge3397:                                         ; preds = %edge3394
  %v30927 = call i64 @mech_compact_free(i64 %v0, i64 %v1401)
  %t2508.not = icmp eq i64 %v30927, 0
  br i1 %t2508.not, label %edge3550, label %edge3398

edge3398:                                         ; preds = %edge3397
  %t2509 = insertvalue { i64, i64 } poison, i64 %v30927, 0
  %t2510 = insertvalue { i64, i64 } %t2509, i64 1580, 1
  br label %common.ret

edge3406:                                         ; preds = %block1561
  %v31115 = add i64 %v30904, %v218770
  %t2511 = icmp ult i64 %v31115, %v1397
  br i1 %t2511, label %edge3414, label %edge3412

edge3412:                                         ; preds = %edge3406, %block21969
  %v31583 = phi i64 [ %v31328, %block21969 ], [ 0, %edge3406 ]
  %v31352 = add i64 %v30904, %spec.select334
  %t2521 = icmp ult i64 %v31352, %v1397
  br i1 %t2521, label %edge3433, label %edge3431

edge3414:                                         ; preds = %edge3406
  br i1 %v1062051.not1294, label %edge3419, label %block21969

edge3419:                                         ; preds = %edge3414
  br i1 %t975, label %trap2518, label %ok2518

edge3431:                                         ; preds = %edge3412, %block22116
  %v31584 = phi i64 [ %v31561, %block22116 ], [ 0, %edge3412 ]
  %t2531 = icmp eq i64 %v31583, %v31584
  br i1 %t2531, label %edge3458, label %edge3447

edge3433:                                         ; preds = %edge3412
  br i1 %v1062051.not1294, label %edge3438, label %block22116

edge3438:                                         ; preds = %edge3433
  br i1 %t975, label %trap2528, label %ok2528

edge3447:                                         ; preds = %edge3431
  br i1 %t23.not, label %edge3539, label %edge3448

edge3448:                                         ; preds = %edge3447
  %v31593 = load i64, ptr %t910, align 1
  %v31594 = add i64 %v31593, -1
  store i64 %v31594, ptr %t910, align 1
  %t2537.not = icmp eq i64 %v31594, 0
  br i1 %t2537.not, label %edge3451, label %edge3539

edge3451:                                         ; preds = %edge3448
  %v31595 = call i64 @mech_compact_free(i64 %v0, i64 %v1401)
  %t2538.not = icmp eq i64 %v31595, 0
  br i1 %t2538.not, label %edge3539, label %edge3452

edge3452:                                         ; preds = %edge3451
  %t2539 = insertvalue { i64, i64 } poison, i64 %v31595, 0
  %t2540 = insertvalue { i64, i64 } %t2539, i64 1616, 1
  br label %common.ret

edge3458:                                         ; preds = %edge3431
  %v31767 = add nuw nsw i64 %v30904, 1
  br label %block1561

edge3470:                                         ; preds = %edge3368
  %v31812 = add i64 %v30649, %v218770
  %t2541.not = icmp ugt i64 %v31812, %v23159
  br i1 %t2541.not, label %edge3491, label %edge3482

edge3482:                                         ; preds = %edge3470
  br i1 %v1062051.not1294, label %edge3491.thread, label %edge3491.thread596

edge3491:                                         ; preds = %edge3470
  %v32602 = add i64 %v30649, %spec.select334
  %t2548.not = icmp ugt i64 %v32602, %v23159
  br i1 %t2548.not, label %edge3512, label %edge3503

edge3491.thread596:                               ; preds = %edge3482
  %v32195 = add i64 %v31812, %v1396
  %t2547 = inttoptr i64 %v32195 to ptr
  %v32196 = load i64, ptr %t2547, align 1
  %v32602598 = add i64 %v30649, %spec.select334
  %t2548.not599 = icmp ugt i64 %v32602598, %v23159
  br i1 %t2548.not599, label %edge3512, label %edge3504

edge3491.thread:                                  ; preds = %edge3482
  %v32198 = call i64 @mech_compact_matrix_word(i64 %v1396, i64 %v31812, i64 %v1399, i64 %v1400, i64 8)
  %v32602587 = add i64 %v30649, %spec.select334
  %t2548.not588 = icmp ugt i64 %v32602587, %v23159
  br i1 %t2548.not588, label %edge3512, label %edge3505

edge3503:                                         ; preds = %edge3491
  br i1 %v1062051.not1294, label %edge3505, label %edge3504

edge3504:                                         ; preds = %edge3491.thread596, %edge3503
  %v32578589604 = phi i64 [ 0, %edge3503 ], [ %v32196, %edge3491.thread596 ]
  %v32602591603 = phi i64 [ %v32602, %edge3503 ], [ %v32602598, %edge3491.thread596 ]
  %v33004 = add i64 %v32602591603, %v1396
  %t2554 = inttoptr i64 %v33004 to ptr
  %v33005 = load i64, ptr %t2554, align 1
  br label %edge3512

edge3505:                                         ; preds = %edge3491.thread, %edge3503
  %v32578589595 = phi i64 [ 0, %edge3503 ], [ %v32198, %edge3491.thread ]
  %v32602591594 = phi i64 [ %v32602, %edge3503 ], [ %v32602587, %edge3491.thread ]
  %v33007 = call i64 @mech_compact_matrix_word(i64 %v1396, i64 %v32602591594, i64 %v1399, i64 %v1400, i64 8)
  br label %edge3512

edge3512:                                         ; preds = %edge3491.thread596, %edge3491.thread, %edge3491, %edge3504, %edge3505
  %v32578590 = phi i64 [ %v32578589595, %edge3505 ], [ %v32578589604, %edge3504 ], [ 0, %edge3491 ], [ %v32198, %edge3491.thread ], [ %v32196, %edge3491.thread596 ]
  %v33391 = phi i64 [ %v33007, %edge3505 ], [ %v33005, %edge3504 ], [ 0, %edge3491 ], [ 0, %edge3491.thread ], [ 0, %edge3491.thread596 ]
  %t2555 = icmp eq i64 %v33391, %v32578590
  br i1 %t2555, label %edge3368, label %edge3515

edge3515:                                         ; preds = %edge3512
  %v33413.le = xor i64 %v33391, %v32578590
  br i1 %t23.not, label %edge3531, label %edge3518

edge3518:                                         ; preds = %edge3515
  %v33426 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t2558.not = icmp eq i64 %v33426, 0
  br i1 %t2558.not, label %edge3531, label %edge3519

edge3519:                                         ; preds = %edge3518
  %t2559 = insertvalue { i64, i64 } poison, i64 %v33426, 0
  %t2560 = insertvalue { i64, i64 } %t2559, i64 1765, 1
  br label %common.ret

edge3531:                                         ; preds = %edge3518, %edge3515
  %t2561 = call range(i64 0, 65) i64 @llvm.cttz.i64(i64 %v33413.le, i1 true)
  %v33551 = lshr i64 %t2561, 3
  %v33556 = add nuw nsw i64 %v33551, %v30649
  br label %edge3539

edge3539:                                         ; preds = %edge3451, %edge3448, %edge3447, %edge3531
  %v33563 = phi i64 [ %v33556, %edge3531 ], [ %v30904, %edge3447 ], [ %v30904, %edge3448 ], [ %v30904, %edge3451 ]
  %t2563.not = icmp ult i64 %v33563, %spec.select18
  br i1 %t2563.not, label %edge3539.block1799_crit_edge, label %edge3550

edge3539.block1799_crit_edge:                     ; preds = %edge3539
  %.pre21808 = add i64 %v33563, %spec.select334
  br label %block1799

edge3550:                                         ; preds = %edge3539, %edge3397, %edge3394, %edge3392
  %v3356322126 = phi i64 [ %umax21627.le, %edge3397 ], [ %umax21627.le, %edge3392 ], [ %umax21627.le, %edge3394 ], [ %v33563, %edge3539 ]
  %v34165 = shl nuw nsw i64 %v28582, 2
  %v34181 = and i64 %v34165, 131068
  %v34207 = add i64 %v34181, %v28543
  %t2568 = inttoptr i64 %v34207 to ptr
  %t2569 = load i16, ptr %t2568, align 1
  %v34255 = add i64 %v28543, 2
  %v34256 = add i64 %v34255, %v34181
  %t2574 = inttoptr i64 %v34256 to ptr
  %t2575 = load i16, ptr %t2574, align 1
  %t2578.not = icmp eq i64 %v28548, 0
  br i1 %t2578.not, label %edge3571, label %edge3558

edge3558:                                         ; preds = %edge3550
  %t2580 = inttoptr i64 %v28548 to ptr
  %v34278 = load i64, ptr %t2580, align 1
  %t2581 = icmp eq i64 %v34278, 1
  %v34286 = shl nuw nsw i64 %v28576, 1
  %v34287 = add i64 %v34286, %v28543
  %t2585 = inttoptr i64 %v34287 to ptr
  br i1 %t2581, label %edge3562, label %edge3571.thread

edge3562:                                         ; preds = %edge3558
  store i16 %t2569, ptr %t2585, align 1
  %v34308 = shl nuw nsw i64 %v28579, 1
  %v34309 = add i64 %v34308, %v28543
  %t2588 = inttoptr i64 %v34309 to ptr
  store i16 %t2575, ptr %t2588, align 1
  br label %block1793

edge3571:                                         ; preds = %edge3550
  %v34328 = shl nuw nsw i64 %v28576, 1
  %v34329 = add i64 %v34328, %v28543
  %t2595 = inttoptr i64 %v34329 to ptr
  %t2596 = load i16, ptr %t2595, align 1
  %t2597 = icmp eq i16 %t2596, %t2569
  br i1 %t2597, label %block23210.thread, label %block23212

edge3571.thread:                                  ; preds = %edge3558
  %t2596608 = load i16, ptr %t2585, align 1
  %t2597609 = icmp eq i16 %t2596608, %t2569
  br i1 %t2597609, label %block23210.thread, label %block23212

edge3583:                                         ; preds = %block23210
  %t2625 = inttoptr i64 %v34358 to ptr
  %v34363 = load i64, ptr %t2625, align 1
  %v34364 = add i64 %v34363, 1
  store i64 %v34364, ptr %t2625, align 1
  br label %block23216

edge3588:                                         ; preds = %block23216
  %v34369 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v28548)
  %t2629.not = icmp eq i64 %v34369, 0
  br i1 %t2629.not, label %edge3594, label %block23187

edge3594:                                         ; preds = %block23216, %edge3588
  %v34406 = shl nuw nsw i64 %v28579, 1
  %v34407 = add i64 %v3433421978, %v34406
  %t2634 = inttoptr i64 %v34407 to ptr
  %t2635 = load i16, ptr %t2634, align 1
  %t2636 = icmp eq i16 %t2635, %t2575
  br i1 %t2636, label %block1793, label %edge3597

edge3597:                                         ; preds = %edge3594
  br i1 %t2624.not7021983, label %block23227, label %edge3598

edge3598:                                         ; preds = %edge3597
  %t2639 = inttoptr i64 %v3433921980 to ptr
  %v34424 = load i64, ptr %t2639, align 1
  %t2640 = icmp eq i64 %v34424, 1
  br i1 %t2640, label %block23225, label %block23227

edge3603:                                         ; preds = %block23227
  %v34434 = load i64, ptr %ss1, align 8
  %v34435 = load i64, ptr %t606, align 8
  %v34436 = load i64, ptr %t607, align 8
  %.pre21787 = add i64 %v34434, %v34406
  %.pre21788 = inttoptr i64 %.pre21787 to ptr
  br label %block23225

edge3606:                                         ; preds = %block23225
  %t2664 = inttoptr i64 %v34423 to ptr
  %v34441 = load i64, ptr %t2664, align 1
  %v34442 = add i64 %v34441, 1
  store i64 %v34442, ptr %t2664, align 1
  br label %block23231

edge3611:                                         ; preds = %block23231
  %v34447 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v3433921980)
  %t2668.not = icmp eq i64 %v34447, 0
  br i1 %t2668.not, label %block1793, label %block23222

edge3627:                                         ; preds = %block1799, %block23387
  %v35074 = phi i64 [ %v34776, %block23387 ], [ 0, %block1799 ]
  %v34848 = add i64 %v218808, %v218770
  %t2682 = icmp ult i64 %v34848, %v1397
  br i1 %t2682, label %edge3648, label %edge3646.thread

edge3646.thread:                                  ; preds = %edge3627
  %v35086616 = shl nuw nsw i64 %v28582, 1
  %v35091617 = and i64 %v35086616, 65534
  br label %edge3683

edge3629:                                         ; preds = %block1799
  br i1 %v1062051.not1294, label %edge3634, label %block23387

edge3634:                                         ; preds = %edge3629
  br i1 %t975, label %trap2679, label %ok2679

edge3646:                                         ; preds = %ok2689, %edge3648
  %v35044 = phi i64 [ %v35049, %ok2689 ], [ %v34848, %edge3648 ]
  %v35051 = add i64 %v35044, %v1396
  %t2690 = inttoptr i64 %v35051 to ptr
  %t2691 = load i8, ptr %t2690, align 1
  %v35052 = zext i8 %t2691 to i64
  %16 = icmp samesign ult i64 %v35074, %v35052
  %v35086 = shl nuw nsw i64 %v28582, 1
  %v35091 = and i64 %v35086, 65534
  %v35096 = or disjoint i64 %v35091, 1
  br i1 %16, label %edge3683, label %edge3674

edge3648:                                         ; preds = %edge3627
  br i1 %v1062051.not1294, label %edge3653, label %edge3646

edge3653:                                         ; preds = %edge3648
  br i1 %t975, label %trap2689, label %ok2689

edge3674:                                         ; preds = %edge3646
  br label %edge3683

edge3683:                                         ; preds = %edge3646.thread, %edge3674, %edge3646
  %v35288 = phi i64 [ %v28576, %edge3646 ], [ %v28579, %edge3646.thread ], [ %v28579, %edge3674 ]
  %v35291 = phi i64 [ %v35096, %edge3646 ], [ %v35091617, %edge3646.thread ], [ %v35091, %edge3674 ]
  %v35294 = phi i64 [ %v35096, %edge3646 ], [ %v28576, %edge3646.thread ], [ %v28576, %edge3674 ]
  %v35297 = phi i64 [ %v28579, %edge3646 ], [ %v35091617, %edge3646.thread ], [ %v35091, %edge3674 ]
  %v35300 = phi i64 [ %v218808, %edge3646 ], [ %v28588, %edge3646.thread ], [ %v28588, %edge3674 ]
  %v35303 = phi i64 [ %v28591, %edge3646 ], [ %v218808, %edge3646.thread ], [ %v218808, %edge3674 ]
  %v35306 = phi i64 [ %v28591, %edge3646 ], [ %v28588, %edge3646.thread ], [ %v28588, %edge3674 ]
  %v35319 = shl nuw nsw i64 %v35288, 1
  %v35320 = add i64 %v35319, %v28543
  %t2697 = inttoptr i64 %v35320 to ptr
  %t2698 = load i16, ptr %t2697, align 1
  %v35321 = zext i16 %t2698 to i64
  %t2699 = icmp eq i64 %v28582, %v35321
  br i1 %t2699, label %block23687, label %edge3686

edge3686:                                         ; preds = %edge3683
  %t2701.not = icmp eq i64 %v28548, 0
  br i1 %t2701.not, label %block23689, label %edge3687

edge3687:                                         ; preds = %edge3686
  %t2702 = inttoptr i64 %v28548 to ptr
  %v35337 = load i64, ptr %t2702, align 1
  %t2703 = icmp eq i64 %v35337, 1
  br i1 %t2703, label %block23688, label %block23689

edge3692:                                         ; preds = %block23689
  %v35347 = load i64, ptr %ss1, align 8
  %v35348 = load i64, ptr %t606, align 8
  %v35349 = load i64, ptr %t607, align 8
  %.pre21785 = add i64 %v35347, %v35319
  %.pre21786 = inttoptr i64 %.pre21785 to ptr
  br label %block23688

edge3695:                                         ; preds = %block23687
  %t2727 = inttoptr i64 %v35330 to ptr
  %v35354 = load i64, ptr %t2727, align 1
  %v35355 = add i64 %v35354, 1
  store i64 %v35355, ptr %t2727, align 1
  br label %block23693

edge3700:                                         ; preds = %block23693
  %v35360 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v28548)
  %t2731.not = icmp eq i64 %v35360, 0
  br i1 %t2731.not, label %edge3706, label %block23684

edge3706:                                         ; preds = %block23693, %edge3700
  %v35371 = shl nuw nsw i64 %v35291, 1
  %v35372 = add i64 %v35325, %v35371
  %t2736 = inttoptr i64 %v35372 to ptr
  %t2737 = load i16, ptr %t2736, align 1
  %v35373 = zext i16 %t2737 to i64
  %spec.select22 = call i64 @llvm.umin.i64(i64 %v35306, i64 %v218808)
  %v35569 = add nsw i64 %v28585, -1
  %t2743 = icmp ult i64 %v12940, %v35373
  %t2744 = icmp ne i64 %v35569, 0
  %v3560265.not = and i1 %t2744, %t2743
  br i1 %v3560265.not, label %block1421, label %edge3727

edge3727:                                         ; preds = %edge3706
  br i1 %t2726.not63, label %edge3742, label %edge3729

edge3729:                                         ; preds = %edge3727
  %t2749 = inttoptr i64 %v35330 to ptr
  %v35783 = load i64, ptr %t2749, align 1
  %t2750 = icmp eq i64 %v35783, 1
  %v35791 = shl nuw nsw i64 %v35294, 1
  %v35792 = add i64 %v35325, %v35791
  %t2754 = inttoptr i64 %v35792 to ptr
  br i1 %t2750, label %edge3733, label %edge3742.thread

edge3733:                                         ; preds = %edge3729
  store i16 0, ptr %t2754, align 1
  %v35813 = shl nuw nsw i64 %v35297, 1
  %v35814 = add i64 %v35325, %v35813
  %t2757 = inttoptr i64 %v35814 to ptr
  store i16 0, ptr %t2757, align 1
  br label %block24092

edge3742:                                         ; preds = %edge3727
  %v35833 = shl nuw nsw i64 %v35294, 1
  %v35834 = add i64 %v35325, %v35833
  %t2764 = inttoptr i64 %v35834 to ptr
  %t2765 = load i16, ptr %t2764, align 1
  %t2766 = icmp eq i16 %t2765, 0
  br i1 %t2766, label %edge3765.thread, label %block23995

edge3742.thread:                                  ; preds = %edge3729
  %t2765626 = load i16, ptr %t2754, align 1
  %t2766627 = icmp eq i16 %t2765626, 0
  br i1 %t2766627, label %block23993, label %block23995

edge3754:                                         ; preds = %block23993
  %t2794 = inttoptr i64 %v35844 to ptr
  %v35868 = load i64, ptr %t2794, align 1
  %v35869 = add i64 %v35868, 1
  store i64 %v35869, ptr %t2794, align 1
  br label %block23999

edge3759:                                         ; preds = %block23999
  %v35874 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v35330)
  %t2798.not = icmp eq i64 %v35874, 0
  br i1 %t2798.not, label %edge3765, label %block23970

edge3765:                                         ; preds = %block23999, %edge3759
  %v35900 = shl nuw nsw i64 %v35297, 1
  %v35901 = add i64 %v35839, %v35900
  %t2803 = inttoptr i64 %v35901 to ptr
  %t2804 = load i16, ptr %t2803, align 1
  %t2805 = icmp eq i16 %t2804, 0
  br i1 %t2805, label %block24092, label %edge3768

edge3765.thread:                                  ; preds = %edge3742
  %v3590022131 = shl nuw nsw i64 %v35297, 1
  %v3590122132 = add i64 %v35325, %v3590022131
  %t280322133 = inttoptr i64 %v3590122132 to ptr
  %t280422134 = load i16, ptr %t280322133, align 1
  %t280522135 = icmp eq i16 %t280422134, 0
  br i1 %t280522135, label %block24092, label %block24010

edge3768:                                         ; preds = %edge3765
  br i1 %t2793.not66, label %block24010, label %edge3769

edge3769:                                         ; preds = %edge3768
  %t2808 = inttoptr i64 %v35844 to ptr
  %v35918 = load i64, ptr %t2808, align 1
  %t2809 = icmp eq i64 %v35918, 1
  br i1 %t2809, label %block24014.thread22316, label %block24010

block24014.thread22316:                           ; preds = %edge3769
  store i16 0, ptr %t2803, align 1
  br label %block24092

edge3777:                                         ; preds = %block24008
  %t2833 = inttoptr i64 %v35930 to ptr
  %v35935 = load i64, ptr %t2833, align 1
  %v35936 = add i64 %v35935, 1
  store i64 %v35936, ptr %t2833, align 1
  br label %block24014

edge3782:                                         ; preds = %block24014
  %v35941 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v3584422010220212213822151)
  %t2837.not = icmp eq i64 %v35941, 0
  br i1 %t2837.not, label %block24092, label %block24005

edge3826:                                         ; preds = %block24092
  %v36111 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t2843.not = icmp eq i64 %v36111, 0
  br i1 %t2843.not, label %edge3980, label %edge3827

edge3827:                                         ; preds = %edge3826
  %t2844 = insertvalue { i64, i64 } poison, i64 %v36111, 0
  %t2845 = insertvalue { i64, i64 } %t2844, i64 1916, 1
  br label %common.ret

edge3980.sink.split:                              ; preds = %edge2154, %edge2118
  %t1852.sink = phi ptr [ %t1806, %edge2118 ], [ %t1852, %edge2154 ]
  %v36625.ph = phi i64 [ %v18875, %edge2118 ], [ %v19254, %edge2154 ]
  %v36627.ph = phi i64 [ %v18877, %edge2118 ], [ %v19256, %edge2154 ]
  %v36630.ph = phi i64 [ %v18880, %edge2118 ], [ %v19259, %edge2154 ]
  store i16 0, ptr %t1852.sink, align 1
  br label %edge3980

edge3980:                                         ; preds = %edge3980.sink.split, %edge2150.thread, %edge2150, %block24092, %edge3826, %block19858, %edge2952, %edge2167, %block15756
  %v36625 = phi i64 [ %v219272, %block19858 ], [ %v19377, %block15756 ], [ %v18875, %edge2150.thread ], [ %v19377, %edge2167 ], [ %v219280, %block24092 ], [ %v219272, %edge2952 ], [ %v219280, %edge3826 ], [ %v19254, %edge2150 ], [ %v36625.ph, %edge3980.sink.split ]
  %v36627 = phi i64 [ %v219273, %block19858 ], [ %v19378, %block15756 ], [ %v18877, %edge2150.thread ], [ %v19378, %edge2167 ], [ %v219281, %block24092 ], [ %v219273, %edge2952 ], [ %v219281, %edge3826 ], [ %v19256, %edge2150 ], [ %v36627.ph, %edge3980.sink.split ]
  %v36630 = phi i64 [ %v219274, %block19858 ], [ %v19379, %block15756 ], [ 0, %edge2150.thread ], [ %v19379, %edge2167 ], [ %v219282, %block24092 ], [ %v219274, %edge2952 ], [ %v219282, %edge3826 ], [ %v19259, %edge2150 ], [ %v36630.ph, %edge3980.sink.split ]
  %v36637 = phi i64 [ %v219275, %block19858 ], [ %v18646, %block15756 ], [ %v18646, %edge2150.thread ], [ %v18646, %edge2167 ], [ %v18646, %block24092 ], [ %v219275, %edge2952 ], [ %v18646, %edge3826 ], [ %v18646, %edge2150 ], [ %v18646, %edge3980.sink.split ]
  %v36639 = phi i64 [ %v219276, %block19858 ], [ %v18648, %block15756 ], [ %v18648, %edge2150.thread ], [ %v18648, %edge2167 ], [ %v18648, %block24092 ], [ %v219276, %edge2952 ], [ %v18648, %edge3826 ], [ %v18648, %edge2150 ], [ %v18648, %edge3980.sink.split ]
  %v36642 = phi i64 [ %v219277, %block19858 ], [ %v18651, %block15756 ], [ %v18651, %edge2150.thread ], [ %v18651, %edge2167 ], [ %v18651, %block24092 ], [ %v219277, %edge2952 ], [ %v18651, %edge3826 ], [ %v18651, %edge2150 ], [ %v18651, %edge3980.sink.split ]
  %v36655 = phi i64 [ %v219278, %block19858 ], [ %v18658, %block15756 ], [ %v18658, %edge2150.thread ], [ %v18658, %edge2167 ], [ %v18658, %block24092 ], [ %v219278, %edge2952 ], [ %v18658, %edge3826 ], [ %v18658, %edge2150 ], [ %v18658, %edge3980.sink.split ]
  %v41036 = phi i64 [ 0, %block19858 ], [ 0, %block15756 ], [ 0, %edge2150.thread ], [ 0, %edge2167 ], [ %v219284, %block24092 ], [ 0, %edge2952 ], [ %v219284, %edge3826 ], [ 0, %edge2150 ], [ 0, %edge3980.sink.split ]
  %t2880 = icmp ugt i64 %v36655, %v218785
  br i1 %t2880, label %edge3986, label %edge3984

edge3984:                                         ; preds = %block535, %edge3980, %edge3987
  %v41036654 = phi i64 [ %v41036, %edge3987 ], [ %v41036, %edge3980 ], [ 0, %block535 ]
  %v36655653 = phi i64 [ %v36655, %edge3987 ], [ %v36655, %edge3980 ], [ %v218785, %block535 ]
  %v36652652 = phi i64 [ %v14615, %edge3987 ], [ %v14615, %edge3980 ], [ %v218788, %block535 ]
  %v36649651 = phi i64 [ %v14588, %edge3987 ], [ %v14588, %edge3980 ], [ %v218782, %block535 ]
  %v36642649 = phi i64 [ %v36642, %edge3987 ], [ %v36642, %edge3980 ], [ %v10790, %block535 ]
  %v36639648 = phi i64 [ %v36639, %edge3987 ], [ %v36639, %edge3980 ], [ %v10798, %block535 ]
  %v36637647 = phi i64 [ %v36637, %edge3987 ], [ %v36637, %edge3980 ], [ %v10825, %block535 ]
  %v36630646 = phi i64 [ %v36630, %edge3987 ], [ %v36630, %edge3980 ], [ %v218781, %block535 ]
  %v36627645 = phi i64 [ %v36627, %edge3987 ], [ %v36627, %edge3980 ], [ %v218784, %block535 ]
  %v36625644 = phi i64 [ %v36625, %edge3987 ], [ %v36625, %edge3980 ], [ %v218783, %block535 ]
  %v36896 = phi i64 [ %v36871, %edge3987 ], [ 0, %edge3980 ], [ 0, %block535 ]
  %v36900 = and i64 %v36896, 65535
  %t2891.not = icmp ult i64 %v218770, %v218789
  %or.cond311 = select i1 %t1457, i1 true, i1 %t2891.not
  br i1 %or.cond311, label %edge4214, label %edge4012

edge3986:                                         ; preds = %edge3980
  %v36866 = add i64 %v36655, -1
  %t2883 = icmp ult i64 %v36866, %v3720
  br i1 %t2883, label %edge3987, label %common.ret

edge3987:                                         ; preds = %edge3986
  %v36869 = shl nuw nsw i64 %v36866, 2
  %v36870 = add i64 %v36869, %v36637
  %t2885 = inttoptr i64 %v36870 to ptr
  %t2886 = load i32, ptr %t2885, align 1
  %v36871 = zext i32 %t2886 to i64
  br label %edge3984

edge4012:                                         ; preds = %edge3984
  %t2894.not = icmp ult i64 %v36900, %v218795
  br i1 %t2894.not, label %edge4128, label %edge4056

edge4056:                                         ; preds = %edge4012
  %t2897 = icmp samesign ugt i64 %v36900, 8
  %v38500 = select i1 %t2897, i64 36, i64 32
  %v38691 = add i64 %v38500, %v10831
  %t2907 = inttoptr i64 %v38691 to ptr
  %t2908 = load i32, ptr %t2907, align 1
  %17 = add i32 %t2908, 1
  %t2919.not = icmp eq i64 %v10794, 0
  br i1 %t2919.not, label %block25146, label %edge4057

edge4057:                                         ; preds = %edge4056
  %t2920 = inttoptr i64 %v10794 to ptr
  %v38815 = load i64, ptr %t2920, align 1
  %t2921 = icmp eq i64 %v38815, 1
  br i1 %t2921, label %block25144, label %block25146

edge4062:                                         ; preds = %block25146
  %v38825 = load i64, ptr %ss1, align 8
  %v38826 = load i64, ptr %t606, align 8
  %v38827 = load i64, ptr %t607, align 8
  %.pre21779 = add i64 %v38825, %v38500
  %.pre21780 = inttoptr i64 %.pre21779 to ptr
  br label %block25144

edge4065:                                         ; preds = %block25144
  %t2945 = inttoptr i64 %v38814 to ptr
  %v38832 = load i64, ptr %t2945, align 1
  %v38833 = add i64 %v38832, 1
  store i64 %v38833, ptr %t2945, align 1
  br label %block25150

edge4070:                                         ; preds = %block25150
  %v38838 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10794)
  %t2949.not = icmp eq i64 %v38838, 0
  br i1 %t2949.not, label %edge4074, label %block25141

edge4074:                                         ; preds = %edge4070, %block25150
  %v38843 = add i64 %v218794, 1
  %v38848 = add i64 %v36900, %v218770
  %t2952 = icmp samesign ult i64 %v36900, 259
  br i1 %t2952, label %edge4101, label %block25299

edge4101:                                         ; preds = %edge4074
  %v39036 = shl nuw nsw i64 %v36900, 2
  %v39037 = add i64 %v39036, %v10829
  %t2955 = inttoptr i64 %v39037 to ptr
  %t2956 = load i32, ptr %t2955, align 1
  %18 = add i32 %t2956, 1
  %t2964.not = icmp eq i64 %v10792, 0
  br i1 %t2964.not, label %block25304, label %edge4102

edge4102:                                         ; preds = %edge4101
  %t2965 = inttoptr i64 %v10792 to ptr
  %v39105 = load i64, ptr %t2965, align 1
  %t2966 = icmp eq i64 %v39105, 1
  br i1 %t2966, label %block25302, label %block25304

edge4107:                                         ; preds = %block25304
  %v39115 = load i64, ptr %ss1, align 8
  %v39116 = load i64, ptr %t606, align 8
  %v39117 = load i64, ptr %t607, align 8
  %.pre21781 = add i64 %v39115, %v39036
  %.pre21782 = inttoptr i64 %.pre21781 to ptr
  br label %block25302

edge4110:                                         ; preds = %block25302
  %t2990 = inttoptr i64 %v39104 to ptr
  %v39122 = load i64, ptr %t2990, align 1
  %v39123 = add i64 %v39122, 1
  store i64 %v39123, ptr %t2990, align 1
  br label %block25308

edge4113:                                         ; preds = %block25308
  %t2994 = inttoptr i64 %v10792 to ptr
  %v39126 = load i64, ptr %t2994, align 1
  %v39127 = add i64 %v39126, -1
  store i64 %v39127, ptr %t2994, align 1
  %t2996.not = icmp eq i64 %v39127, 0
  br i1 %t2996.not, label %edge4116, label %edge4214

edge4116:                                         ; preds = %edge4113
  %v39128 = call i64 @mech_compact_free(i64 %v0, i64 %v10792)
  %t2997.not = icmp eq i64 %v39128, 0
  br i1 %t2997.not, label %edge4214, label %block25299

edge4128:                                         ; preds = %edge4012
  %t3000 = icmp ult i64 %v218770, %v1397
  br i1 %t3000, label %edge4134, label %edge4132

edge4132:                                         ; preds = %edge4128, %block25465
  %v39502 = phi i64 [ %v39387, %block25465 ], [ 0, %edge4128 ]
  %v39551 = lshr i64 %v39502, 5
  %v39632 = and i64 %v39551, 6
  %v39637 = and i64 %v39502, 1
  %v39642 = or disjoint i64 %v39632, %v39637
  %v39830 = shl nuw nsw i64 %v39642, 2
  %v39831 = add i64 %v39830, %v10831
  %t3015 = inttoptr i64 %v39831 to ptr
  %t3016 = load i32, ptr %t3015, align 1
  %v39855 = add i32 %t3016, 1
  %t3023.not = icmp eq i64 %v10794, 0
  br i1 %t3023.not, label %block25622, label %edge4170

edge4134:                                         ; preds = %edge4128
  br i1 %v1062051.not1294, label %edge4139, label %block25465

edge4139:                                         ; preds = %edge4134
  br i1 %t975, label %trap3007, label %ok3007

edge4170:                                         ; preds = %edge4132
  %t3024 = inttoptr i64 %v10794 to ptr
  %v39899 = load i64, ptr %t3024, align 1
  %t3025 = icmp eq i64 %v39899, 1
  br i1 %t3025, label %block25621, label %block25622

edge4175:                                         ; preds = %block25622
  %v39909 = load i64, ptr %ss1, align 8
  %v39910 = load i64, ptr %t606, align 8
  %v39911 = load i64, ptr %t607, align 8
  %.pre21777 = add i64 %v39909, %v39830
  %.pre21778 = inttoptr i64 %.pre21777 to ptr
  br label %block25621

edge4178:                                         ; preds = %block25621
  %t3049 = inttoptr i64 %v39898 to ptr
  %v39916 = load i64, ptr %t3049, align 1
  %v39917 = add i64 %v39916, 1
  store i64 %v39917, ptr %t3049, align 1
  br label %block25626

edge4183:                                         ; preds = %block25626
  %v39922 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10794)
  %t3053.not = icmp eq i64 %v39922, 0
  br i1 %t3053.not, label %edge4187, label %block25617

edge4187:                                         ; preds = %edge4183, %block25626
  %v39927 = add i64 %v218794, 1
  %v39932 = add i64 %v218770, 1
  br label %edge4214

edge4214:                                         ; preds = %edge4187, %edge4116, %edge4113, %block25308, %edge3984
  %v40032 = phi i64 [ %v39099, %block25308 ], [ %v10829, %edge3984 ], [ %v10829, %edge4187 ], [ %v39099, %edge4116 ], [ %v39099, %edge4113 ]
  %v40034 = phi i64 [ %v39101, %block25308 ], [ %v10802, %edge3984 ], [ %v10802, %edge4187 ], [ %v39101, %edge4116 ], [ %v39101, %edge4113 ]
  %v40037 = phi i64 [ %v39104, %block25308 ], [ %v10792, %edge3984 ], [ %v10792, %edge4187 ], [ %v39104, %edge4116 ], [ %v39104, %edge4113 ]
  %v40044 = phi i64 [ %v38809, %block25308 ], [ %v10831, %edge3984 ], [ %v39893, %edge4187 ], [ %v38809, %edge4116 ], [ %v38809, %edge4113 ]
  %v40046 = phi i64 [ %v38811, %block25308 ], [ %v10806, %edge3984 ], [ %v39895, %edge4187 ], [ %v38811, %edge4116 ], [ %v38811, %edge4113 ]
  %v40049 = phi i64 [ %v38814, %block25308 ], [ %v10794, %edge3984 ], [ %v39898, %edge4187 ], [ %v38814, %edge4116 ], [ %v38814, %edge4113 ]
  %v40056 = phi i64 [ %v38843, %block25308 ], [ %v218794, %edge3984 ], [ %v39927, %edge4187 ], [ %v38843, %edge4116 ], [ %v38843, %edge4113 ]
  %v40059 = phi i64 [ %v38848, %block25308 ], [ %v218789, %edge3984 ], [ %v39932, %edge4187 ], [ %v38848, %edge4116 ], [ %v38848, %edge4113 ]
  %v40064 = sub i64 %v36655653, %v218785
  %t3056 = icmp ult i64 %v218770, %v1397
  br i1 %t3056, label %edge4222, label %edge4220

edge4220:                                         ; preds = %edge4214, %block25834
  %v40290 = phi i64 [ %1, %block25834 ], [ 0, %edge4214 ]
  %v40285 = and i64 %v40064, 65535
  %v40300 = or disjoint i64 %v40290, %v40285
  %v40316 = add i64 %v36655653, 1
  %t3067 = icmp ult i64 %v36655653, %v3720
  br i1 %t3067, label %edge4241, label %block25847

edge4222:                                         ; preds = %edge4214
  br i1 %v1062051.not1294, label %edge4227, label %block25834

edge4227:                                         ; preds = %edge4222
  br i1 %t975, label %trap3063, label %ok3063

edge4241:                                         ; preds = %edge4220
  %v40324 = shl nuw nsw i64 %v36655653, 2
  %v40325 = add i64 %v36637647, %v40324
  %t3069 = inttoptr i64 %v40325 to ptr
  %t3070 = load i32, ptr %t3069, align 1
  %v40326 = zext i32 %t3070 to i64
  %t3071 = icmp eq i64 %v40300, %v40326
  %t3098.not1001203 = icmp eq i64 %v36642649, 0
  br i1 %t3071, label %edge4262, label %edge4244

edge4244:                                         ; preds = %edge4241
  br i1 %t3098.not1001203, label %block25852, label %edge4245

edge4245:                                         ; preds = %edge4244
  %t3074 = inttoptr i64 %v36642649 to ptr
  %v40342 = load i64, ptr %t3074, align 1
  %t3075 = icmp eq i64 %v40342, 1
  br i1 %t3075, label %block25850, label %block25852

edge4250:                                         ; preds = %block25852
  %v40352 = load i64, ptr %ss1, align 8
  %v40353 = load i64, ptr %t606, align 8
  %v40354 = load i64, ptr %t607, align 8
  %.pre21775 = add i64 %v40352, %v40324
  %.pre21776 = inttoptr i64 %.pre21775 to ptr
  br label %block25850

edge4253:                                         ; preds = %block25850
  %t3099 = inttoptr i64 %v40341 to ptr
  %v40359 = load i64, ptr %t3099, align 1
  %v40360 = add i64 %v40359, 1
  store i64 %v40360, ptr %t3099, align 1
  br label %block25856

edge4258:                                         ; preds = %block25856
  %v40365 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v36642649)
  %t3103.not = icmp eq i64 %v40365, 0
  br i1 %t3103.not, label %edge4262, label %block25847

edge4262:                                         ; preds = %edge4241, %edge4258, %block25856
  %v4033012061226 = phi i64 [ %v40336, %block25856 ], [ %v40336, %edge4258 ], [ %v36637647, %edge4241 ]
  %v4033212091225 = phi i64 [ %v40338, %block25856 ], [ %v40338, %edge4258 ], [ %v36639648, %edge4241 ]
  %v4033512111224 = phi i64 [ %v40341, %block25856 ], [ %v40341, %edge4258 ], [ %v36642649, %edge4241 ]
  %t3098.not10012141223 = phi i1 [ %t3098.not100, %block25856 ], [ %t3098.not100, %edge4258 ], [ %t3098.not1001203, %edge4241 ]
  br i1 %t1457, label %edge4281, label %edge4267

edge4265:                                         ; preds = %edge4267, %edge4281
  %v41043 = phi i64 [ %v40771, %edge4267 ], [ %v40836, %edge4281 ]
  %v40844 = shl nuw nsw i64 %v36900, 16
  %v40850 = lshr i64 %v36896, 16
  %v40855 = or disjoint i64 %v40844, %v40850
  %spec.select24 = select i1 %t1457, i64 %v41036654, i64 %v40855
  %t3116.not = icmp eq i64 %v41043, 0
  %v41261 = shl i64 %v41043, 32
  %v41267 = or i64 %v41261, %spec.select24
  %v41285 = add i64 %v218770, 1
  %t312033135 = icmp eq i64 %v41267, 0
  %t3120 = select i1 %t3116.not, i1 true, i1 %t312033135
  br i1 %t3120, label %edge4337, label %block475.outer

edge4267:                                         ; preds = %edge4262
  %t3108 = icmp samesign ugt i64 %v36900, 2
  %t3109 = icmp samesign uge i64 %v36900, %spec.select18
  %v40583102 = and i1 %t3108, %t3109
  %v40755 = add nsw i64 %v36900, -1
  %v40771 = select i1 %v40583102, i64 %v40755, i64 0
  br label %edge4265

edge4281:                                         ; preds = %edge4262
  %v40836 = add nsw i64 %v12690, -1
  br label %edge4265

edge4337:                                         ; preds = %edge4265
  %t3123 = icmp uge i64 %v41285, %v218796
  %t3124 = icmp ugt i64 %v40316, 1499999
  %v41763103.not.not33136 = or i1 %t3123, %t3124
  %t3125 = icmp ult i64 %v40056, 512
  %v41898 = sub i64 %v41285, %v218797
  %t3126 = icmp ult i64 %v41898, 5000
  %.not = or i1 %t3126, %t3125
  %v41994 = sub i64 %v1397, %v41285
  %t3127 = icmp ult i64 %v41994, 5000
  %.not33138 = or i1 %t3127, %.not
  %v42005106.not = or i1 %v41763103.not.not33136, %.not33138
  br i1 %v42005106.not, label %edge4337.block2462_crit_edge, label %edge4353

edge4337.block2462_crit_edge:                     ; preds = %edge4337
  br i1 %v41763103.not.not33136, label %edge4852, label %block475.backedge

edge4353:                                         ; preds = %edge4337
  %t3130.not = icmp eq i64 %v218798, 0
  br i1 %t3130.not, label %block2305.preheader, label %edge4360

edge4360:                                         ; preds = %edge4353
  %t3133.not = icmp eq i64 %v40049, 0
  br i1 %t3133.not, label %block26830, label %edge4361

edge4361:                                         ; preds = %edge4360
  %t3134 = inttoptr i64 %v40049 to ptr
  %v42674 = load i64, ptr %t3134, align 1
  %v42675 = add i64 %v42674, 1
  store i64 %v42675, ptr %t3134, align 1
  br label %block26830

edge4364:                                         ; preds = %block26830
  %v42676 = load i64, ptr %t1092, align 1
  %v42677 = add i64 %v42676, 1
  store i64 %v42677, ptr %t1092, align 1
  br label %block2176.preheader

edge4387:                                         ; preds = %block2176
  %v43199 = shl nuw nsw i64 %v42746, 2
  %v43200 = add i64 %v43199, %v10830.ph.ph
  %t3142 = inttoptr i64 %v43200 to ptr
  %t3143 = load i32, ptr %t3142, align 1
  %19 = zext i32 %t3143 to i64
  %umul = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %19, i64 %v40056)
  %20 = extractvalue { i64, i1 } %umul, 1
  br i1 %20, label %common.ret, label %edge4397

edge4397:                                         ; preds = %edge4387
  %v43624 = add i64 %v43199, %v40044
  %t3152 = inttoptr i64 %v43624 to ptr
  %t3153 = load i32, ptr %t3152, align 1
  %21 = zext i32 %t3153 to i64
  %umul107 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %21, i64 %v218798)
  %22 = extractvalue { i64, i1 } %umul107, 1
  br i1 %22, label %common.ret, label %edge4449

edge4432:                                         ; preds = %block2176
  br i1 %t3133.not, label %block27584, label %edge4435

edge4435:                                         ; preds = %edge4432
  %v43891 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v40049)
  %t3177.not = icmp eq i64 %v43891, 0
  br i1 %t3177.not, label %block27584, label %block27580

edge4439:                                         ; preds = %block27584
  %v43893 = load i64, ptr %t1092, align 1
  %v43894 = add i64 %v43893, -1
  store i64 %v43894, ptr %t1092, align 1
  %t3181.not = icmp eq i64 %v43894, 0
  br i1 %t3181.not, label %edge4442, label %edge4464

edge4442:                                         ; preds = %edge4439
  %v43895 = call i64 @mech_compact_free(i64 %v0, i64 %v10793.ph.ph)
  %t3182.not = icmp eq i64 %v43895, 0
  br i1 %t3182.not, label %edge4464, label %block27580

edge4449:                                         ; preds = %edge4397
  %v43228 = mul i64 %v40056, %19
  %v43651 = mul i64 %v218798, %21
  %t3162 = icmp ugt i64 %v43651, %v43228
  %v43856 = sub i64 %v43651, %v43228
  %23 = sub nuw i64 %v43228, %v43651
  %v44069 = select i1 %t3162, i64 %v43856, i64 %23
  %v44062 = add nuw nsw i64 %v42746, 1
  %v44070 = add i64 %v44069, %v42749
  %t3186.not = icmp ult i64 %v44070, %v42749
  br i1 %t3186.not, label %common.ret, label %block2176

edge4464:                                         ; preds = %block27584, %edge4439, %edge4442
  %v44114 = add i64 %v40056, %v218798
  %t3190.not = icmp ult i64 %v44114, %v218798
  br i1 %t3190.not, label %common.ret, label %edge4467

edge4467:                                         ; preds = %edge4464
  %t3198 = icmp ult i64 %v40056, 92233720368547759
  br i1 %t3198, label %edge4468, label %common.ret

edge4468:                                         ; preds = %edge4467
  %v44205 = mul nuw i64 %v40056, 200
  %v44213108 = lshr i64 %v44205, 9
  %umul109 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v44213108, i64 %v218798)
  %24 = extractvalue { i64, i1 } %umul109, 1
  br i1 %24, label %common.ret, label %edge4473

edge4473:                                         ; preds = %edge4468
  %v44221 = mul i64 %v44213108, %v218798
  %t3212 = icmp ult i64 %v41898, 10000
  %t3213 = icmp ult i64 %v44114, 8192
  %v44401110 = and i1 %t3212, %t3213
  br i1 %v44401110, label %edge4479, label %edge4495

edge4479:                                         ; preds = %edge4473
  %v44575 = sub nuw nsw i64 8192, %v44114
  %umul111 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v44221, i64 %v44575)
  %25 = extractvalue { i64, i1 } %umul111, 1
  br i1 %25, label %common.ret, label %edge4484

edge4484:                                         ; preds = %edge4479
  %v44583 = mul i64 %v44221, %v44575
  %v44590112 = lshr i64 %v44583, 13
  %v44595 = add i64 %v44590112, %v44221
  %t3226.not = icmp ult i64 %v44595, %v44221
  br i1 %t3226.not, label %common.ret, label %edge4495

edge4495:                                         ; preds = %edge4473, %edge4484
  %v44697 = phi i64 [ %v44595, %edge4484 ], [ %v44221, %edge4473 ]
  %v44677113 = lshr i64 %v41898, 12
  %umul114 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v44677113, i64 %v218798)
  %26 = extractvalue { i64, i1 } %umul114, 1
  br i1 %26, label %common.ret, label %edge4497

edge4497:                                         ; preds = %edge4495
  %v44685 = mul i64 %v44677113, %v218798
  %v44690 = add i64 %v42749, %v44685
  %t3240.not = icmp ult i64 %v44690, %v42749
  br i1 %t3240.not, label %common.ret, label %edge4502

edge4502:                                         ; preds = %edge4497
  %t3244.not = icmp ult i64 %v44690, %v44697
  br i1 %t3244.not, label %block2305.preheader, label %block2462

block2305.preheader:                              ; preds = %edge4353, %edge4502
  br label %block2305

edge4538:                                         ; preds = %block2305
  %v45569 = add nuw nsw i64 %v45205, 1
  %v45577 = shl nuw nsw i64 %v45205, 2
  %v45578 = add i64 %v45577, %v45193
  %t3250 = inttoptr i64 %v45578 to ptr
  %t3251 = load i32, ptr %t3250, align 1
  %v45579 = zext i32 %t3251 to i64
  %v45791 = add i64 %v45577, %v45178
  %t3252 = inttoptr i64 %v45791 to ptr
  %t3253 = load i32, ptr %t3252, align 1
  %v45792 = zext i32 %t3253 to i64
  %v45812 = add nuw nsw i64 %v45792, %v45579
  %t3254 = icmp samesign ult i64 %v45812, 4294967296
  br i1 %t3254, label %edge4561, label %common.ret

edge4539:                                         ; preds = %block2305
  %v46282 = add i64 %v40056, %v218798
  %t3335.not = icmp ult i64 %v46282, %v218798
  br i1 %t3335.not, label %common.ret, label %block2378

edge4561:                                         ; preds = %edge4538
  %t3260 = icmp eq i32 %t3253, 0
  br i1 %t3260, label %block28608, label %edge4566

edge4566:                                         ; preds = %edge4561
  %t3262.not = icmp eq i64 %v45198, 0
  br i1 %t3262.not, label %block28610, label %edge4567

edge4567:                                         ; preds = %edge4566
  %t3263 = inttoptr i64 %v45198 to ptr
  %v45847 = load i64, ptr %t3263, align 1
  %t3264 = icmp eq i64 %v45847, 1
  br i1 %t3264, label %block28609, label %block28610

edge4572:                                         ; preds = %block28610
  %v45857 = load i64, ptr %ss1, align 8
  %v45858 = load i64, ptr %t606, align 8
  %v45859 = load i64, ptr %t607, align 8
  %.pre21705 = add i64 %v45857, %v45577
  %.pre21706 = inttoptr i64 %.pre21705 to ptr
  br label %block28609

edge4575:                                         ; preds = %block28608
  %t3288 = inttoptr i64 %v45840 to ptr
  %v45864 = load i64, ptr %t3288, align 1
  %v45865 = add i64 %v45864, 1
  store i64 %v45865, ptr %t3288, align 1
  br label %block28614

edge4580:                                         ; preds = %block28614
  %v45870 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v45198)
  %t3292.not = icmp eq i64 %v45870, 0
  br i1 %t3292.not, label %edge4586, label %block28605

edge4586:                                         ; preds = %edge4580, %block28614
  %t3296 = load i32, ptr %t3252, align 1
  %t3297 = icmp eq i32 %t3296, 0
  br i1 %t3297, label %block28626, label %edge4591

edge4591:                                         ; preds = %edge4586
  %t3299.not = icmp eq i64 %v45183, 0
  br i1 %t3299.not, label %block28628, label %edge4592

edge4592:                                         ; preds = %edge4591
  %t3300 = inttoptr i64 %v45183 to ptr
  %v45910 = load i64, ptr %t3300, align 1
  %t3301 = icmp eq i64 %v45910, 1
  br i1 %t3301, label %block28627, label %block28628

edge4597:                                         ; preds = %block28628
  %v45920 = load i64, ptr %ss1, align 8
  %v45921 = load i64, ptr %t606, align 8
  %v45922 = load i64, ptr %t607, align 8
  %.pre = add i64 %v45920, %v45577
  %.pre21704 = inttoptr i64 %.pre to ptr
  br label %block28627

edge4600:                                         ; preds = %block28626
  %t3325 = inttoptr i64 %v45903 to ptr
  %v45927 = load i64, ptr %t3325, align 1
  %v45928 = add i64 %v45927, 1
  store i64 %v45928, ptr %t3325, align 1
  br label %block28632

edge4603:                                         ; preds = %block28632
  %t3329 = inttoptr i64 %v45183 to ptr
  %v45931 = load i64, ptr %t3329, align 1
  %v45932 = add i64 %v45931, -1
  store i64 %v45932, ptr %t3329, align 1
  %t3331.not = icmp eq i64 %v45932, 0
  br i1 %t3331.not, label %edge4606, label %block2305.backedge

edge4606:                                         ; preds = %edge4603
  %v45933 = call i64 @mech_compact_free(i64 %v0, i64 %v45183)
  %t3332.not = icmp eq i64 %v45933, 0
  br i1 %t3332.not, label %block2305.backedge, label %block28623

block2305.backedge:                               ; preds = %edge4606, %edge4603, %block28632
  br label %block2305

edge4658:                                         ; preds = %block2378
  %v47107 = shl nuw nsw i64 %v46684, 2
  %v47108 = add i64 %v47107, %v46672
  %t3344 = inttoptr i64 %v47108 to ptr
  %t3345 = load i32, ptr %t3344, align 1
  %t3346.not = icmp eq i32 %t3345, 0
  br i1 %t3346.not, label %edge4773, label %edge4678

edge4678:                                         ; preds = %edge4658
  %v47109 = zext i32 %t3345 to i64
  %v47486 = add i64 %v47107, %v46660
  %t3351 = inttoptr i64 %v47486 to ptr
  %t3352 = load i32, ptr %t3351, align 1
  %v47487 = zext i32 %t3352 to i64
  %v47506 = add nuw nsw i64 %v47487, %v47109
  %t3353 = icmp samesign ult i64 %v47506, 4294967296
  br i1 %t3353, label %edge4691, label %common.ret

edge4691:                                         ; preds = %edge4678
  %t3363.not = icmp eq i64 %v46665, 0
  br i1 %t3363.not, label %block29463, label %edge4692

edge4692:                                         ; preds = %edge4691
  %t3364 = inttoptr i64 %v46665 to ptr
  %v47541 = load i64, ptr %t3364, align 1
  %t3365 = icmp eq i64 %v47541, 1
  br i1 %t3365, label %block29462, label %block29463

edge4697:                                         ; preds = %block29463
  %v47551 = load i64, ptr %ss1, align 8
  %v47552 = load i64, ptr %t606, align 8
  %v47553 = load i64, ptr %t607, align 8
  %.pre21707 = add i64 %v47551, %v47107
  %.pre21708 = inttoptr i64 %.pre21707 to ptr
  br label %block29462

edge4700:                                         ; preds = %block29462
  %t3389 = inttoptr i64 %v47540 to ptr
  %v47558 = load i64, ptr %t3389, align 1
  %v47559 = add i64 %v47558, 1
  store i64 %v47559, ptr %t3389, align 1
  br label %block29467

edge4705:                                         ; preds = %block29467
  %v47564 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v46665)
  %t3393.not = icmp eq i64 %v47564, 0
  br i1 %t3393.not, label %edge4709, label %block29458

edge4709:                                         ; preds = %edge4705, %block29467
  %t3399 = load i32, ptr %t3344, align 1
  %t3400 = icmp eq i32 %t3399, 0
  br i1 %t3400, label %block29477, label %edge4714

edge4714:                                         ; preds = %edge4709
  %t3402.not = icmp eq i64 %v46677, 0
  br i1 %t3402.not, label %block29479, label %edge4715

edge4715:                                         ; preds = %edge4714
  %t3403 = inttoptr i64 %v46677 to ptr
  %v47597 = load i64, ptr %t3403, align 1
  %t3404 = icmp eq i64 %v47597, 1
  br i1 %t3404, label %block29478, label %block29479

edge4720:                                         ; preds = %block29479
  %v47607 = load i64, ptr %ss1, align 8
  %v47608 = load i64, ptr %t606, align 8
  %v47609 = load i64, ptr %t607, align 8
  %.pre21709 = add i64 %v47607, %v47107
  %.pre21710 = inttoptr i64 %.pre21709 to ptr
  br label %block29478

edge4723:                                         ; preds = %block29477
  %t3428 = inttoptr i64 %v47590 to ptr
  %v47614 = load i64, ptr %t3428, align 1
  %v47615 = add i64 %v47614, 1
  store i64 %v47615, ptr %t3428, align 1
  br label %block29483

edge4726:                                         ; preds = %block29483
  %t3432 = inttoptr i64 %v46677 to ptr
  %v47618 = load i64, ptr %t3432, align 1
  %v47619 = add i64 %v47618, -1
  store i64 %v47619, ptr %t3432, align 1
  %t3434.not = icmp eq i64 %v47619, 0
  br i1 %t3434.not, label %edge4729, label %edge4773

edge4729:                                         ; preds = %edge4726
  %v47620 = call i64 @mech_compact_free(i64 %v0, i64 %v46677)
  %t3435.not = icmp eq i64 %v47620, 0
  br i1 %t3435.not, label %edge4773, label %block29474

edge4773:                                         ; preds = %edge4658, %block29483, %edge4726, %edge4729
  %v47723.ph = phi i64 [ %v47535, %edge4726 ], [ %v47535, %edge4729 ], [ %v47535, %block29483 ], [ %v46660, %edge4658 ]
  %v47725.ph = phi i64 [ %v47537, %edge4726 ], [ %v47537, %edge4729 ], [ %v47537, %block29483 ], [ %v46662, %edge4658 ]
  %v47728.ph = phi i64 [ %v47540, %edge4726 ], [ %v47540, %edge4729 ], [ %v47540, %block29483 ], [ %v46665, %edge4658 ]
  %v47735.ph = phi i64 [ %v47585, %edge4726 ], [ %v47585, %edge4729 ], [ %v47585, %block29483 ], [ %v46672, %edge4658 ]
  %v47737.ph = phi i64 [ %v47587, %edge4726 ], [ %v47587, %edge4729 ], [ %v47587, %block29483 ], [ %v46674, %edge4658 ]
  %v47740.ph = phi i64 [ %v47590, %edge4726 ], [ %v47590, %edge4729 ], [ %v47590, %block29483 ], [ %v46677, %edge4658 ]
  %v47952 = add nuw nsw i64 %v46684, 1
  br label %block2378

edge4852:                                         ; preds = %edge4337.block2462_crit_edge, %block2462
  %v21882722043 = phi i64 [ 0, %edge4337.block2462_crit_edge ], [ 1, %block2462 ]
  br i1 %t23.not, label %edge4881, label %edge4853

edge4853:                                         ; preds = %edge4852
  %v48422 = load i64, ptr %t910, align 1
  %v48423 = add i64 %v48422, -1
  store i64 %v48423, ptr %t910, align 1
  %t3448.not = icmp eq i64 %v48423, 0
  br i1 %t3448.not, label %edge4856, label %edge4881

edge4856:                                         ; preds = %edge4853
  %v48424 = call i64 @mech_compact_free(i64 %v0, i64 %v1401)
  %t3449.not = icmp eq i64 %v48424, 0
  br i1 %t3449.not, label %edge4881, label %edge4857

edge4857:                                         ; preds = %edge4856
  %t3450 = insertvalue { i64, i64 } poison, i64 %v48424, 0
  %t3451 = insertvalue { i64, i64 } %t3450, i64 2494, 1
  br label %common.ret

edge4881:                                         ; preds = %edge4852, %edge4853, %edge4856
  %t3453 = icmp ne i64 %v218804, 1
  %v50059124.not = or i1 %t3453, %v42005106.not
  br i1 %v50059124.not, label %block2587.outer, label %edge4885

edge4885:                                         ; preds = %edge4881
  %t3456.not = icmp ult i64 %v41285, %v218803
  br i1 %t3456.not, label %common.ret, label %edge4894

edge4894:                                         ; preds = %edge4885
  %v50403 = sub nuw i64 %v41285, %v218803
  br i1 %t3098.not10012141223, label %edge4902, label %edge4897

edge4897:                                         ; preds = %edge4894
  %t3461 = inttoptr i64 %v4033512111224 to ptr
  %v50406 = load i64, ptr %t3461, align 1
  %v50407 = add i64 %v50406, 1
  store i64 %v50407, ptr %t3461, align 1
  br label %edge4902

edge4902:                                         ; preds = %edge4894, %edge4897
  %v51015 = add i64 %v4033012061226, -4
  br label %block2509

edge4915:                                         ; preds = %block2509
  br i1 %t3098.not10012141223, label %edge4971, label %edge4945

edge4917:                                         ; preds = %block2509
  %t3466.not = icmp eq i64 %v50450, 0
  br i1 %t3466.not, label %common.ret, label %edge4918

edge4918:                                         ; preds = %edge4917
  %v50640 = add i64 %v50450, -1
  %t3470 = icmp ult i64 %v50640, %v3720
  br i1 %t3470, label %edge4931, label %block30781

edge4931:                                         ; preds = %edge4918
  %v51012 = shl i64 %v50450, 2
  %v51016 = add i64 %v51015, %v51012
  %t3473 = inttoptr i64 %v51016 to ptr
  %t3474 = load i32, ptr %t3473, align 1
  %27 = and i32 %t3474, 65535
  %v51042 = zext nneg i32 %27 to i64
  %t3475.not = icmp ult i64 %v50640, %v51042
  br i1 %t3475.not, label %common.ret, label %block30781

edge4945:                                         ; preds = %edge4915
  %t3480 = inttoptr i64 %v4033512111224 to ptr
  %v51075 = load i64, ptr %t3480, align 1
  %v51076 = add i64 %v51075, -1
  store i64 %v51076, ptr %t3480, align 1
  %t3482.not = icmp eq i64 %v51076, 0
  br i1 %t3482.not, label %edge4948, label %edge4971

edge4948:                                         ; preds = %edge4945
  %v51077 = call i64 @mech_compact_free(i64 %v0, i64 %v4033512111224)
  %t3483.not = icmp eq i64 %v51077, 0
  br i1 %t3483.not, label %edge4971, label %edge4949

edge4949:                                         ; preds = %edge4948
  %t3484 = insertvalue { i64, i64 } poison, i64 %v51077, 0
  %t3485 = insertvalue { i64, i64 } %t3484, i64 2565, 1
  br label %common.ret

block2662.thread:                                 ; preds = %edge4971
  %v51457 = sub nuw i64 %v40316, %v50450
  br label %edge5117

edge4971:                                         ; preds = %edge4948, %edge4945, %edge4915
  %t3488.not = icmp ult i64 %v40316, %v50450
  br i1 %t3488.not, label %common.ret, label %block2662.thread

edge5008:                                         ; preds = %block2587
  %v52112 = add nuw nsw i64 %v51759, 1
  %v52120 = shl nuw nsw i64 %v51759, 2
  %v52121 = add i64 %v52120, %v51747.ph
  %t3499 = inttoptr i64 %v52121 to ptr
  %t3500 = load i32, ptr %t3499, align 1
  %t3501.not = icmp eq i32 %t3500, 0
  br i1 %t3501.not, label %block2587, label %edge5027

edge5027:                                         ; preds = %edge5008
  %v52122 = zext i32 %t3500 to i64
  %v52490 = add i64 %v52120, %v51735.ph
  %t3506 = inttoptr i64 %v52490 to ptr
  %t3507 = load i32, ptr %t3506, align 1
  %v52491 = zext i32 %t3507 to i64
  %v52510 = add nuw nsw i64 %v52491, %v52122
  %t3508 = icmp samesign ult i64 %v52510, 4294967296
  br i1 %t3508, label %edge5040, label %common.ret

edge5040:                                         ; preds = %edge5027
  %t3518.not = icmp eq i64 %v51740.ph, 0
  br i1 %t3518.not, label %block31759, label %edge5041

edge5041:                                         ; preds = %edge5040
  %t3519 = inttoptr i64 %v51740.ph to ptr
  %v52545 = load i64, ptr %t3519, align 1
  %t3520 = icmp eq i64 %v52545, 1
  br i1 %t3520, label %block31758, label %block31759

edge5046:                                         ; preds = %block31759
  %v52555 = load i64, ptr %ss1, align 8
  %v52556 = load i64, ptr %t606, align 8
  %v52557 = load i64, ptr %t607, align 8
  %.pre21711 = add i64 %v52555, %v52120
  %.pre21712 = inttoptr i64 %.pre21711 to ptr
  br label %block31758

edge5049:                                         ; preds = %block31758
  %t3544 = inttoptr i64 %v52544 to ptr
  %v52562 = load i64, ptr %t3544, align 1
  %v52563 = add i64 %v52562, 1
  store i64 %v52563, ptr %t3544, align 1
  br label %block31763

edge5054:                                         ; preds = %block31763
  %v52568 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v51740.ph)
  %t3548.not = icmp eq i64 %v52568, 0
  br i1 %t3548.not, label %edge5058, label %block31754

edge5058:                                         ; preds = %edge5054, %block31763
  %t3554 = load i32, ptr %t3499, align 1
  %t3555 = icmp eq i32 %t3554, 0
  br i1 %t3555, label %block31773, label %edge5063

edge5063:                                         ; preds = %edge5058
  %t3557.not = icmp eq i64 %v51752.ph, 0
  br i1 %t3557.not, label %block31775, label %edge5064

edge5064:                                         ; preds = %edge5063
  %t3558 = inttoptr i64 %v51752.ph to ptr
  %v52601 = load i64, ptr %t3558, align 1
  %t3559 = icmp eq i64 %v52601, 1
  br i1 %t3559, label %block31774, label %block31775

edge5069:                                         ; preds = %block31775
  %v52611 = load i64, ptr %ss1, align 8
  %v52612 = load i64, ptr %t606, align 8
  %v52613 = load i64, ptr %t607, align 8
  %.pre21713 = add i64 %v52611, %v52120
  %.pre21714 = inttoptr i64 %.pre21713 to ptr
  br label %block31774

edge5072:                                         ; preds = %block31773
  %t3583 = inttoptr i64 %v52594 to ptr
  %v52618 = load i64, ptr %t3583, align 1
  %v52619 = add i64 %v52618, 1
  store i64 %v52619, ptr %t3583, align 1
  br label %block31779

edge5075:                                         ; preds = %block31779
  %t3587 = inttoptr i64 %v51752.ph to ptr
  %v52622 = load i64, ptr %t3587, align 1
  %v52623 = add i64 %v52622, -1
  store i64 %v52623, ptr %t3587, align 1
  %t3589.not = icmp eq i64 %v52623, 0
  br i1 %t3589.not, label %edge5078, label %block2587.outer.backedge

block2587.outer:                                  ; preds = %edge4881, %block2587.outer.backedge
  %v51735.ph = phi i64 [ %v52539, %block2587.outer.backedge ], [ %v10828.ph.ph, %edge4881 ]
  %v51737.ph = phi i64 [ %v52541, %block2587.outer.backedge ], [ %v10800.ph.ph, %edge4881 ]
  %v51740.ph = phi i64 [ %v52544, %block2587.outer.backedge ], [ %v10791.ph.ph, %edge4881 ]
  %v51747.ph = phi i64 [ %v52589, %block2587.outer.backedge ], [ %v40032, %edge4881 ]
  %v51749.ph = phi i64 [ %v52591, %block2587.outer.backedge ], [ %v40034, %edge4881 ]
  %v51752.ph = phi i64 [ %v52594, %block2587.outer.backedge ], [ %v40037, %edge4881 ]
  %v51759.ph = phi i64 [ %v52112, %block2587.outer.backedge ], [ 0, %edge4881 ]
  br label %block2587

edge5078:                                         ; preds = %edge5075
  %v52624 = call i64 @mech_compact_free(i64 %v0, i64 %v51752.ph)
  %t3590.not = icmp eq i64 %v52624, 0
  br i1 %t3590.not, label %block2587.outer.backedge, label %block31770

edge5117:                                         ; preds = %block2662.thread, %edge5147
  %v54815695 = phi i64 [ %v51457, %block2662.thread ], [ 0, %edge5147 ]
  %v51454686693 = phi i64 [ %v50450, %block2662.thread ], [ %v40316, %edge5147 ]
  %v54804687691 = phi i64 [ %v218803, %block2662.thread ], [ %v41285, %edge5147 ]
  %v218879385 = phi i64 [ %v40037, %block2662.thread ], [ %v51752.ph, %edge5147 ]
  %v218877384 = phi i64 [ %v40034, %block2662.thread ], [ %v51749.ph, %edge5147 ]
  %v218876383 = phi i64 [ %v40032, %block2662.thread ], [ %v51747.ph, %edge5147 ]
  %v218875382 = phi i64 [ %v10791.ph.ph, %block2662.thread ], [ %v51740.ph, %edge5147 ]
  %v218874381 = phi i64 [ %v10828.ph.ph, %block2662.thread ], [ %v51735.ph, %edge5147 ]
  %v54666 = phi i64 [ %v10830.ph.ph, %block2662.thread ], [ %v53321, %edge5147 ]
  %v54671 = phi i64 [ %v10793.ph.ph, %block2662.thread ], [ %v53326, %edge5147 ]
  %v54678 = phi i64 [ %v40044, %block2662.thread ], [ %v53306, %edge5147 ]
  %v54680 = phi i64 [ %v40046, %block2662.thread ], [ %v53308, %edge5147 ]
  %v54683 = phi i64 [ %v40049, %block2662.thread ], [ %v53311, %edge5147 ]
  %v54690 = phi i64 [ %v218798, %block2662.thread ], [ %v54384, %edge5147 ]
  %v54693 = phi i64 [ %v40056, %block2662.thread ], [ 0, %edge5147 ]
  %t3688 = icmp eq i64 %v41285, %v1397
  %v54437133 = and i1 %t3688, %v50059124.not
  %v54818 = zext i1 %v54437133 to i64
  br label %edge5300

edge5146:                                         ; preds = %block2672
  %v53688 = add nuw nsw i64 %v53333, 1
  %v53696 = shl nuw nsw i64 %v53333, 2
  %v53697 = add i64 %v53696, %v53321
  %t3599 = inttoptr i64 %v53697 to ptr
  %t3600 = load i32, ptr %t3599, align 1
  %v53698 = zext i32 %t3600 to i64
  %v53904 = add i64 %v53696, %v53306
  %t3601 = inttoptr i64 %v53904 to ptr
  %t3602 = load i32, ptr %t3601, align 1
  %v53905 = zext i32 %t3602 to i64
  %v53925 = add nuw nsw i64 %v53905, %v53698
  %t3603 = icmp samesign ult i64 %v53925, 4294967296
  br i1 %t3603, label %edge5169, label %common.ret

edge5147:                                         ; preds = %block2672
  %v54384 = add i64 %v40056, %v218798
  %t3684.not = icmp ult i64 %v54384, %v218798
  br i1 %t3684.not, label %common.ret, label %edge5117

edge5169:                                         ; preds = %edge5146
  %t3609 = icmp eq i32 %t3602, 0
  br i1 %t3609, label %block32627, label %edge5174

edge5174:                                         ; preds = %edge5169
  %t3611.not = icmp eq i64 %v53326, 0
  br i1 %t3611.not, label %block32629, label %edge5175

edge5175:                                         ; preds = %edge5174
  %t3612 = inttoptr i64 %v53326 to ptr
  %v53960 = load i64, ptr %t3612, align 1
  %t3613 = icmp eq i64 %v53960, 1
  br i1 %t3613, label %block32628, label %block32629

edge5180:                                         ; preds = %block32629
  %v53970 = load i64, ptr %ss1, align 8
  %v53971 = load i64, ptr %t606, align 8
  %v53972 = load i64, ptr %t607, align 8
  %.pre21717 = add i64 %v53970, %v53696
  %.pre21718 = inttoptr i64 %.pre21717 to ptr
  br label %block32628

edge5183:                                         ; preds = %block32627
  %t3637 = inttoptr i64 %v53953 to ptr
  %v53977 = load i64, ptr %t3637, align 1
  %v53978 = add i64 %v53977, 1
  store i64 %v53978, ptr %t3637, align 1
  br label %block32633

edge5188:                                         ; preds = %block32633
  %v53983 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v53326)
  %t3641.not = icmp eq i64 %v53983, 0
  br i1 %t3641.not, label %edge5194, label %block32624

edge5194:                                         ; preds = %edge5188, %block32633
  %t3645 = load i32, ptr %t3601, align 1
  %t3646 = icmp eq i32 %t3645, 0
  br i1 %t3646, label %block32645, label %edge5199

edge5199:                                         ; preds = %edge5194
  %t3648.not = icmp eq i64 %v53311, 0
  br i1 %t3648.not, label %block32647, label %edge5200

edge5200:                                         ; preds = %edge5199
  %t3649 = inttoptr i64 %v53311 to ptr
  %v54023 = load i64, ptr %t3649, align 1
  %t3650 = icmp eq i64 %v54023, 1
  br i1 %t3650, label %block32646, label %block32647

edge5205:                                         ; preds = %block32647
  %v54033 = load i64, ptr %ss1, align 8
  %v54034 = load i64, ptr %t606, align 8
  %v54035 = load i64, ptr %t607, align 8
  %.pre21715 = add i64 %v54033, %v53696
  %.pre21716 = inttoptr i64 %.pre21715 to ptr
  br label %block32646

edge5208:                                         ; preds = %block32645
  %t3674 = inttoptr i64 %v54016 to ptr
  %v54040 = load i64, ptr %t3674, align 1
  %v54041 = add i64 %v54040, 1
  store i64 %v54041, ptr %t3674, align 1
  br label %block32651

edge5211:                                         ; preds = %block32651
  %t3678 = inttoptr i64 %v53311 to ptr
  %v54044 = load i64, ptr %t3678, align 1
  %v54045 = add i64 %v54044, -1
  store i64 %v54045, ptr %t3678, align 1
  %t3680.not = icmp eq i64 %v54045, 0
  br i1 %t3680.not, label %edge5214, label %block2672.backedge

edge5214:                                         ; preds = %edge5211
  %v54046 = call i64 @mech_compact_free(i64 %v0, i64 %v53311)
  %t3681.not = icmp eq i64 %v54046, 0
  br i1 %t3681.not, label %block2672.backedge, label %block32642

block2672.backedge:                               ; preds = %edge5214, %edge5211, %block32651
  br label %block2672

edge5300:                                         ; preds = %edge1350, %edge1353, %edge1356, %edge5117
  %v54915 = phi i64 [ %v36625644, %edge5117 ], [ %v10824, %edge1356 ], [ %v10824, %edge1353 ], [ %v10824, %edge1350 ]
  %v54917 = phi i64 [ %v36627645, %edge5117 ], [ %v10796, %edge1356 ], [ %v10796, %edge1353 ], [ %v10796, %edge1350 ]
  %v54920 = phi i64 [ %v36630646, %edge5117 ], [ %v10789, %edge1356 ], [ %v10789, %edge1353 ], [ %v10789, %edge1350 ]
  %v54921 = phi i64 [ %v4033012061226, %edge5117 ], [ %v10825, %edge1356 ], [ %v10825, %edge1353 ], [ %v10825, %edge1350 ]
  %v54923 = phi i64 [ %v4033212091225, %edge5117 ], [ %v10798, %edge1356 ], [ %v10798, %edge1353 ], [ %v10798, %edge1350 ]
  %v54926 = phi i64 [ %v4033512111224, %edge5117 ], [ %v10790, %edge1356 ], [ %v10790, %edge1353 ], [ %v10790, %edge1350 ]
  %v54927 = phi i64 [ %v36649651, %edge5117 ], [ %v219678, %edge1356 ], [ %v219678, %edge1353 ], [ %v219678, %edge1350 ]
  %v54928 = phi i64 [ %v36652652, %edge5117 ], [ %v219679, %edge1356 ], [ %v219679, %edge1353 ], [ %v219679, %edge1350 ]
  %v54929 = phi i64 [ %v41285, %edge5117 ], [ %v10728, %edge1356 ], [ %v10728, %edge1353 ], [ %v10728, %edge1350 ]
  %v54931 = phi i64 [ %spec.select1, %edge5117 ], [ %v219682, %edge1356 ], [ %v219682, %edge1353 ], [ %v219682, %edge1350 ]
  %v54932 = phi i64 [ %v41335, %edge5117 ], [ %v219683, %edge1356 ], [ %v219683, %edge1353 ], [ %v219683, %edge1350 ]
  %v54933 = phi i64 [ %v218874381, %edge5117 ], [ %v10828.ph.ph, %edge1356 ], [ %v10828.ph.ph, %edge1353 ], [ %v10828.ph.ph, %edge1350 ]
  %v54938 = phi i64 [ %v218875382, %edge5117 ], [ %v10791.ph.ph, %edge1356 ], [ %v10791.ph.ph, %edge1353 ], [ %v10791.ph.ph, %edge1350 ]
  %v54939 = phi i64 [ %v218876383, %edge5117 ], [ %v10829, %edge1356 ], [ %v10829, %edge1353 ], [ %v10829, %edge1350 ]
  %v54941 = phi i64 [ %v218877384, %edge5117 ], [ %v10802, %edge1356 ], [ %v10802, %edge1353 ], [ %v10802, %edge1350 ]
  %v54944 = phi i64 [ %v218879385, %edge5117 ], [ %v10792, %edge1356 ], [ %v10792, %edge1353 ], [ %v10792, %edge1350 ]
  %v54945 = phi i64 [ %v54666, %edge5117 ], [ %v10830.ph.ph, %edge1356 ], [ %v10830.ph.ph, %edge1353 ], [ %v10830.ph.ph, %edge1350 ]
  %v54950 = phi i64 [ %v54671, %edge5117 ], [ %v10793.ph.ph, %edge1356 ], [ %v10793.ph.ph, %edge1353 ], [ %v10793.ph.ph, %edge1350 ]
  %v54951 = phi i64 [ %v54678, %edge5117 ], [ %v10831, %edge1356 ], [ %v10831, %edge1353 ], [ %v10831, %edge1350 ]
  %v54953 = phi i64 [ %v54680, %edge5117 ], [ %v10806, %edge1356 ], [ %v10806, %edge1353 ], [ %v10806, %edge1350 ]
  %v54956 = phi i64 [ %v54683, %edge5117 ], [ %v10794, %edge1356 ], [ %v10794, %edge1353 ], [ %v10794, %edge1350 ]
  %v54957 = phi i64 [ %v54690, %edge5117 ], [ %v219686, %edge1356 ], [ %v219686, %edge1353 ], [ %v219686, %edge1350 ]
  %v54958 = phi i64 [ %v54693, %edge5117 ], [ %v219687, %edge1356 ], [ %v219687, %edge1353 ], [ %v219687, %edge1350 ]
  %v54959 = phi i64 [ %v218797, %edge5117 ], [ %v219690, %edge1356 ], [ %v219690, %edge1353 ], [ %v219690, %edge1350 ]
  %v54960 = phi i64 [ %v54804687691, %edge5117 ], [ %v219691, %edge1356 ], [ %v219691, %edge1353 ], [ %v219691, %edge1350 ]
  %v54961 = phi i64 [ %v51454686693, %edge5117 ], [ %v219694, %edge1356 ], [ %v219694, %edge1353 ], [ %v219694, %edge1350 ]
  %v54962 = phi i64 [ %v54815695, %edge5117 ], [ %v219695, %edge1356 ], [ %v219695, %edge1353 ], [ %v219695, %edge1350 ]
  %v54963 = phi i64 [ %v54818, %edge5117 ], [ %v219698, %edge1356 ], [ %v219698, %edge1353 ], [ %v219698, %edge1350 ]
  %v54964 = phi i64 [ %v21882722043, %edge5117 ], [ %v219699, %edge1356 ], [ %v219699, %edge1353 ], [ %v219699, %edge1350 ]
  %v54966 = phi i64 [ %v218804, %edge5117 ], [ %v10741, %edge1356 ], [ %v10741, %edge1353 ], [ %v10741, %edge1350 ]
  %v210497 = phi i64 [ %v218771, %edge5117 ], [ %v12013, %edge1356 ], [ %v12013, %edge1353 ], [ %v12013, %edge1350 ]
  %v210594 = phi i64 [ %v218773, %edge5117 ], [ %v12417, %edge1356 ], [ %v12417, %edge1353 ], [ %v12417, %edge1350 ]
  %t3691.not = icmp ult i64 %v54960, %v54959
  br i1 %t3691.not, label %common.ret, label %edge5301

edge5301:                                         ; preds = %edge5300
  %v55001 = sub nuw i64 %v54960, %v54959
  %t3695 = icmp ne i64 %v54959, 0
  %t3699 = icmp ne i64 %v54963, 1
  %t3700 = icmp ne i64 %v54961, %v55001
  %.not214 = or i1 %t3700, %t3699
  %v55220135.not = or i1 %t3695, %.not214
  %t3701.not = icmp eq i64 %v54926, 0
  br i1 %t3701.not, label %edge5328, label %edge5324

edge5324:                                         ; preds = %edge5301
  %t3702 = inttoptr i64 %v54926 to ptr
  %v55231 = load i64, ptr %t3702, align 1
  %v55232 = add i64 %v55231, 1
  store i64 %v55232, ptr %t3702, align 1
  br label %edge5328

edge5328:                                         ; preds = %edge5324, %edge5301
  %t3704.not = icmp eq i64 %v54950, 0
  br i1 %t3704.not, label %edge5365, label %edge5329

edge5329:                                         ; preds = %edge5328
  %t3705 = inttoptr i64 %v54950 to ptr
  %v55237 = load i64, ptr %t3705, align 1
  %v55238 = add i64 %v55237, 1
  store i64 %v55238, ptr %t3705, align 1
  br label %edge5365

edge5345:                                         ; preds = %edge5368
  %t3714 = insertvalue { i64, i64 } poison, i64 %v55821, 0
  %t3715 = insertvalue { i64, i64 } %t3714, i64 2807, 1
  br label %common.ret

edge5365:                                         ; preds = %edge5328, %edge5329
  %v55963 = call i64 @mech_compact_allocate(i64 %v0, i64 320, i64 4, i64 4, i64 1, i64 %v3194)
  %t3728.not = icmp eq i64 %v55963, 0
  br i1 %t3728.not, label %edge5367, label %block33335

edge5367:                                         ; preds = %edge5365
  %v55966 = load i64, ptr %ss1, align 8
  %v55967 = load i64, ptr %t606, align 8
  %v55968 = load i64, ptr %t607, align 8
  %t3731.not = icmp eq i64 %v55968, 0
  br i1 %t3731.not, label %block2804, label %edge5368

edge5368:                                         ; preds = %edge5367
  %t3732 = inttoptr i64 %v55968 to ptr
  %v55969 = load i64, ptr %t3732, align 1
  %v55817 = add i64 %v55969, 2
  store i64 %v55817, ptr %t3732, align 1
  %v55821 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v55968)
  %t3713.not = icmp eq i64 %v55821, 0
  br i1 %t3713.not, label %block2804, label %edge5345

edge5374:                                         ; preds = %block2804
  %v56459 = load i64, ptr %t910, align 1
  %v56460 = add i64 %v56459, 1
  store i64 %v56460, ptr %t910, align 1
  br label %block2806.preheader

edge5382:                                         ; preds = %block2806
  %t3740 = inttoptr i64 %v56479 to ptr
  %v56480 = load i64, ptr %t3740, align 1
  %t3741 = icmp eq i64 %v56480, 1
  %t3743 = icmp ugt i64 %v56513, 319
  %or.cond312 = select i1 %t3741, i1 %t3743, i1 false
  br i1 %or.cond312, label %edge5386, label %edge5416.thread

edge5386:                                         ; preds = %edge5382
  %t3745 = zext i64 %v54959 to i128
  %t3746 = zext i64 %v55001 to i128
  %t3747 = shl nuw i128 %t3746, 64
  %v219885 = or disjoint i128 %t3747, %t3745
  store i128 %v219885, ptr %ss2, align 16
  %t3749 = zext i64 %v56516 to i128
  %v219887 = or disjoint i128 %t934, %t3749
  store i128 %v219887, ptr %t926, align 16
  store i128 %v219585, ptr %t931, align 16
  %t3760 = zext i64 %v56521 to i128
  %t3761 = shl nuw i128 %t3760, 64
  %v219891 = or disjoint i128 %t3761, %t942
  store i128 %v219891, ptr %t936, align 16
  store i128 18446744073709551936, ptr %t941, align 16
  %t3770 = zext i64 %v56479 to i128
  %t3771 = shl nuw i128 %t3770, 64
  %v219895 = or disjoint i128 %t3771, 320
  store i128 %v219895, ptr %t946, align 16
  store i64 %v56513, ptr %t947, align 16
  br i1 %v1062051.not1294, label %edge5389, label %edge5390

edge5389:                                         ; preds = %edge5386
  %t3779 = call fastcc { i64, i64 } @mech_compact_numeric_2821(i64 %v219576, i64 %v8481)
  br label %block33393

edge5390:                                         ; preds = %edge5386
  %t3778 = call fastcc { i64, i64 } @mech_compact_numeric_2821_vector(i64 %v219576, i64 %v8481)
  br label %block33393

edge5393:                                         ; preds = %block33393
  %v56486 = extractvalue { i64, i64 } %t3778.pn, 1
  %t3781 = icmp eq i64 %v56485, 103169
  %v56499 = select i1 %t3781, i64 103425, i64 %v56485
  %t3783 = insertvalue { i64, i64 } poison, i64 %v56499, 0
  %t3784 = insertvalue { i64, i64 } %t3783, i64 %v56486, 1
  br label %common.ret

edge5394:                                         ; preds = %block33393
  br i1 %t23.not, label %edge5573, label %edge5397

edge5397:                                         ; preds = %edge5394
  %v56504 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t3786.not = icmp eq i64 %v56504, 0
  br i1 %t3786.not, label %edge5573, label %edge5398

edge5398:                                         ; preds = %edge5397
  %t3787 = insertvalue { i64, i64 } poison, i64 %v56504, 0
  %t3788 = insertvalue { i64, i64 } %t3787, i64 2821, 1
  br label %common.ret

edge5416:                                         ; preds = %block2806
  %t3789 = icmp ult i64 %v56516, %v55001
  br i1 %t3789, label %edge5422, label %edge5506

edge5416.thread:                                  ; preds = %edge5382
  %t3789703 = icmp ult i64 %v56516, %v55001
  br i1 %t3789703, label %edge5422, label %edge5501

edge5422:                                         ; preds = %edge5416.thread, %edge5416
  %v56762 = add i64 %v56516, %v54959
  %t3792.not = icmp ult i64 %v56762, %v54959
  br i1 %t3792.not, label %common.ret, label %edge5433

edge5433:                                         ; preds = %edge5422
  %t3796 = icmp ult i64 %v56762, %v1397
  br i1 %t3796, label %edge5441, label %edge5477

edge5441:                                         ; preds = %edge5433
  br i1 %v1062051.not1294, label %edge5445, label %block33729

edge5445:                                         ; preds = %edge5441
  br i1 %t975, label %trap3803, label %ok3803

edge5477:                                         ; preds = %edge5433, %block33729
  %v57101 = phi i64 [ %2, %block33729 ], [ 0, %edge5433 ]
  %v57333 = add i64 %v57101, %v56521
  %t3809 = inttoptr i64 %v57333 to ptr
  %t3810 = load i32, ptr %t3809, align 1
  %v57457 = add i32 %t3810, 1
  %t3811.not = icmp eq i32 %t3810, -1
  br i1 %t3811.not, label %common.ret, label %edge5485

edge5485:                                         ; preds = %edge5477
  br i1 %t3739.not, label %block33914, label %edge5486

edge5486:                                         ; preds = %edge5485
  %t3822 = inttoptr i64 %v56479 to ptr
  %v57490 = load i64, ptr %t3822, align 1
  %t3823 = icmp eq i64 %v57490, 1
  br i1 %t3823, label %block33912, label %block33914

edge5491:                                         ; preds = %block33914
  %v57500 = load i64, ptr %ss1, align 8
  %v57501 = load i64, ptr %t606, align 8
  %v57502 = load i64, ptr %t607, align 8
  %.pre21719 = add i64 %v57500, %v57101
  %.pre21720 = inttoptr i64 %.pre21719 to ptr
  br label %block33912

edge5494:                                         ; preds = %block33912
  %t3847 = inttoptr i64 %v57489 to ptr
  %v57507 = load i64, ptr %t3847, align 1
  %v57508 = add i64 %v57507, 1
  store i64 %v57508, ptr %t3847, align 1
  br label %edge5505

edge5501:                                         ; preds = %edge5416.thread
  %v57528 = add i64 %v56480, 1
  store i64 %v57528, ptr %t3740, align 1
  br label %edge5506

edge5505:                                         ; preds = %block33912, %edge5494
  %t3864.not = or i1 %t3739.not, %t3844
  br i1 %t3864.not, label %edge5530, label %edge5523

edge5506:                                         ; preds = %edge5416, %edge5501
  br i1 %t23.not, label %block33934, label %edge5509

edge5509:                                         ; preds = %edge5506
  %v57537 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t3858.not = icmp eq i64 %v57537, 0
  br i1 %t3858.not, label %block33934, label %block33929

edge5513:                                         ; preds = %block33934
  %t3860 = inttoptr i64 %v56479 to ptr
  %v57539 = load i64, ptr %t3860, align 1
  %v57540 = add i64 %v57539, -1
  store i64 %v57540, ptr %t3860, align 1
  %t3862.not = icmp eq i64 %v57540, 0
  br i1 %t3862.not, label %edge5516, label %edge5573

edge5516:                                         ; preds = %edge5513
  %v57541 = call i64 @mech_compact_free(i64 %v0, i64 %v56479)
  %t3863.not = icmp eq i64 %v57541, 0
  br i1 %t3863.not, label %edge5573, label %block33929

edge5523:                                         ; preds = %edge5505
  %v57545 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v56479)
  %t3865.not = icmp eq i64 %v57545, 0
  br i1 %t3865.not, label %edge5530, label %block33929

edge5530:                                         ; preds = %edge5505, %edge5523
  %v57724 = add i64 %v56516, 1
  br label %block2806

edge5557:                                         ; preds = %block33934
  %v58076 = add i64 %v56521, 1024
  %t3869 = inttoptr i64 %v58076 to ptr
  %t3870 = load i32, ptr %t3869, align 1
  %v58106 = add i32 %t3870, 1
  br label %block34286

edge5573:                                         ; preds = %edge5397, %edge5394, %edge5516, %edge5513
  %v5807622156 = add i64 %v56521, 1024
  %t386922157 = inttoptr i64 %v5807622156 to ptr
  %t387022158 = load i32, ptr %t386922157, align 1
  %v5810622159 = add i32 %t387022158, 1
  %t3876 = inttoptr i64 %v56479 to ptr
  %v58157 = load i64, ptr %t3876, align 1
  %t3877 = icmp eq i64 %v58157, 1
  br i1 %t3877, label %block34290.thread, label %block34286

block34290.thread:                                ; preds = %edge5573
  store i32 %v5810622159, ptr %t386922157, align 1
  br label %edge5596

edge5581:                                         ; preds = %block34285
  %t3901 = inttoptr i64 %v58169 to ptr
  %v58174 = load i64, ptr %t3901, align 1
  %v58175 = add i64 %v58174, 1
  store i64 %v58175, ptr %t3901, align 1
  br label %block34290

edge5584:                                         ; preds = %block34290
  %t3905 = inttoptr i64 %v5777522162 to ptr
  %v58178 = load i64, ptr %t3905, align 1
  %v58179 = add i64 %v58178, -1
  store i64 %v58179, ptr %t3905, align 1
  %t3907.not = icmp eq i64 %v58179, 0
  br i1 %t3907.not, label %edge5587, label %edge5595

edge5587:                                         ; preds = %edge5584
  %v58180 = call i64 @mech_compact_free(i64 %v0, i64 %v5777522162)
  %t3908.not = icmp eq i64 %v58180, 0
  br i1 %t3908.not, label %edge5595, label %block34281

edge5595:                                         ; preds = %edge5587, %edge5584, %block34290
  br i1 %t3900.not138, label %edge5647, label %edge5596

edge5596:                                         ; preds = %block34290.thread, %edge5595
  %v58156223382235022392 = phi i64 [ %v56479, %block34290.thread ], [ %v58169, %edge5595 ]
  %v58153223372235122390 = phi i64 [ %v56513, %block34290.thread ], [ %v58168, %edge5595 ]
  %v58151223362235222388 = phi i64 [ %v56521, %block34290.thread ], [ %v58167, %edge5595 ]
  %t3912 = inttoptr i64 %v58156223382235022392 to ptr
  %v64393 = load i64, ptr %t3912, align 1
  %v64412 = add i64 %v64393, 5
  store i64 %v64412, ptr %t3912, align 1
  br label %edge5647

edge5647:                                         ; preds = %edge5595, %edge5596
  %t3900.not138223402234922395 = phi i1 [ false, %edge5596 ], [ true, %edge5595 ]
  %v58156223382235022393 = phi i64 [ %v58156223382235022392, %edge5596 ], [ 0, %edge5595 ]
  %v58153223372235122391 = phi i64 [ %v58153223372235122390, %edge5596 ], [ %v58168, %edge5595 ]
  %v58151223362235222389 = phi i64 [ %v58151223362235222388, %edge5596 ], [ %v58167, %edge5595 ]
  %t4451.pre-phi = phi ptr [ %t3912, %edge5596 ], [ null, %edge5595 ]
  %t4143 = inttoptr i64 %v54926 to ptr
  %v68077 = add i64 %v54921, -4
  %t4173 = zext i64 %v54921 to i128
  %v219927 = or disjoint i128 %t1719, %t4173
  %v75440 = add i64 %v55001, 257
  %t4491 = zext i64 %v75440 to i128
  %t4495 = zext i64 %v55001 to i128
  %t7372 = icmp ult i64 %v55001, 16384
  %t7440 = icmp ult i64 %v55001, 10001
  %v147829 = lshr i64 %v55001, 11
  %t7555.not = icmp eq i64 %v54938, 0
  %t7556 = inttoptr i64 %v54938 to ptr
  %t7636 = inttoptr i64 %v54950 to ptr
  %t7638.not = icmp eq i64 %v219242, 0
  %t7639 = inttoptr i64 %v219242 to ptr
  %umul219 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v219243, i64 %v54957)
  %28 = extractvalue { i64, i1 } %umul219, 1
  %v161437 = mul i64 %v54957, %v219243
  %t7704 = icmp ugt i64 %v161437, 92233720368547758
  %v161513 = mul nuw i64 %v161437, 200
  %v161588220 = lshr i64 %v161513, 9
  %v161598 = mul nuw nsw i64 %v161588220, 3
  %v161892 = mul nuw nsw i64 %v161588220, 9
  %brmerge = or i1 %28, %t7704
  br label %block2907

edge5744:                                         ; preds = %block2907, %edge5762, %edge5761
  %v66494 = phi i64 [ %v64765, %edge5762 ], [ 0, %edge5761 ], [ 0, %block2907 ]
  %v67292 = phi i64 [ %v64760, %edge5762 ], [ %v64760, %edge5761 ], [ ptrtoint (ptr @mech_compact_table_2 to i64), %block2907 ]
  %v67293 = phi i64 [ %v64762, %edge5762 ], [ %v64762, %edge5761 ], [ 547, %block2907 ]
  %v65824 = or i64 %v64969, %v65001
  %t3962 = icmp ne i64 %v65824, 0
  br i1 %t3962, label %edge5769, label %edge5941

edge5761:                                         ; preds = %block2907
  %t3959.not = icmp eq i64 %v64765, 0
  br i1 %t3959.not, label %edge5744, label %edge5762

edge5762:                                         ; preds = %edge5761
  %t3960 = inttoptr i64 %v64765 to ptr
  %v65818 = load i64, ptr %t3960, align 1
  %v65819 = add i64 %v65818, 1
  store i64 %v65819, ptr %t3960, align 1
  br label %edge5744

edge5769:                                         ; preds = %edge6879, %edge6882, %edge6196, %block35555, %edge5744
  %v74579 = phi i64 [ %v64748, %edge5744 ], [ %v219942, %edge6196 ], [ %v219942, %block35555 ], [ %v67404, %edge6882 ], [ %v67404, %edge6879 ]
  %v74580 = phi i64 [ %v64749, %edge5744 ], [ %v219943, %edge6196 ], [ %v219943, %block35555 ], [ %v67396, %edge6882 ], [ %v67396, %edge6879 ]
  %v74581 = phi i64 [ %v64750, %edge5744 ], [ %v219946, %edge6196 ], [ %v219946, %block35555 ], [ %v67414, %edge6882 ], [ %v67414, %edge6879 ]
  %v74582 = phi i64 [ %v64751, %edge5744 ], [ %v219947, %edge6196 ], [ %v219947, %block35555 ], [ 1, %edge6882 ], [ 1, %edge6879 ]
  %v74583 = phi i64 [ %v64752, %edge5744 ], [ %v219950, %edge6196 ], [ %v219950, %block35555 ], [ %v67396, %edge6882 ], [ %v67396, %edge6879 ]
  %v74584 = phi i64 [ %v64753, %edge5744 ], [ %v219951, %edge6196 ], [ %v219951, %block35555 ], [ %v67328, %edge6882 ], [ %v67328, %edge6879 ]
  %v74267 = or i64 %v65824, %v64985
  %t4448.not = icmp eq i64 %v74267, 0
  br i1 %t4448.not, label %edge6930, label %edge6925

edge5769.thread:                                  ; preds = %edge6878
  br i1 %t3927, label %edge6925, label %edge6966

edge5941:                                         ; preds = %edge5744
  %t4020 = icmp eq i64 %v64751, 1
  br i1 %t4020, label %edge5950, label %common.ret

edge5950:                                         ; preds = %edge5941
  %t4030 = icmp ugt i64 %v64750, 257
  br i1 %t4030, label %block35292, label %edge5953

edge5953:                                         ; preds = %edge5950
  %t4032.not = icmp eq i64 %v64750, 0
  br i1 %t4032.not, label %block35292, label %edge5957

edge5957:                                         ; preds = %edge5953
  %t4036 = shl nuw nsw i64 %v64750, 1
  %29 = call i64 @llvm.umax.i64(i64 %t4036, i64 258)
  br label %block35292

edge5970:                                         ; preds = %block35292
  %t4054 = inttoptr i64 %v64753 to ptr
  %v67048 = load i64, ptr %t4054, align 1
  %t4055 = icmp eq i64 %v67048, 1
  %t4056 = icmp ule i64 %v67017, %v64750
  %v67051141 = and i1 %t4056, %t4055
  br i1 %v67051141, label %edge5977, label %block35303

edge5975:                                         ; preds = %block35303
  %v67058 = load i64, ptr %ss1, align 8
  %v67059 = load i64, ptr %t606, align 8
  %v67060 = load i64, ptr %t607, align 8
  br label %edge5977

edge5977:                                         ; preds = %edge5975, %edge5970
  %v67042 = phi i64 [ %v67058, %edge5975 ], [ %v64748, %edge5970 ]
  %v67044 = phi i64 [ %v67059, %edge5975 ], [ %v64750, %edge5970 ]
  %v67047 = phi i64 [ %v67060, %edge5975 ], [ %v64753, %edge5970 ]
  %v67066 = call i64 @memmove(i64 %v67042, i64 ptrtoint (ptr @mech_compact_table_8 to i64), i64 2064)
  %t4076 = icmp eq i64 %v64753, %v67047
  %t4078.not142 = icmp eq i64 %v67047, 0
  %t4078.not = or i1 %t4076, %t4078.not142
  br i1 %t4078.not, label %block35309, label %edge5980

edge5980:                                         ; preds = %edge5977
  %t4079 = inttoptr i64 %v67047 to ptr
  %v67069 = load i64, ptr %t4079, align 1
  %v67070 = add i64 %v67069, 1
  store i64 %v67070, ptr %t4079, align 1
  br label %block35309

edge5985:                                         ; preds = %block35309
  %v67075 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64753)
  %t4083.not = icmp eq i64 %v67075, 0
  br i1 %t4083.not, label %edge6154, label %block35287

edge6154:                                         ; preds = %edge5985, %block35309
  br i1 %t3701.not, label %block35510, label %edge6156

edge6156:                                         ; preds = %edge6154
  %v67296 = load i64, ptr %t4143, align 1
  %v67297 = add i64 %v67296, 1
  store i64 %v67297, ptr %t4143, align 1
  br label %block35510

edge6159:                                         ; preds = %block35510
  %t4146 = inttoptr i64 %v66494 to ptr
  %v67298 = load i64, ptr %t4146, align 1
  %v67299 = add i64 %v67298, 1
  store i64 %v67299, ptr %t4146, align 1
  br label %edge6169

edge6169:                                         ; preds = %edge6159, %block35510
  %v68871 = add i64 %v67292, 1036
  %v70415 = add i64 %v67292, 2060
  br label %block2989

edge6170:                                         ; preds = %block2989
  %t4149 = inttoptr i64 %v67328 to ptr
  %v67329 = load i64, ptr %t4149, align 1
  %t4150 = icmp eq i64 %v67329, 1
  br i1 %t4150, label %edge6176, label %edge6220.thread

edge6176:                                         ; preds = %edge6170
  %t4159 = zext i64 %v67402 to i128
  %t4160 = zext i64 %v67403 to i128
  %t4161 = shl nuw i128 %t4160, 64
  %v219921 = or disjoint i128 %t4161, %t4159
  store i128 %v219921, ptr %ss2, align 16
  %t4163 = zext i64 %v67404 to i128
  %t4164 = zext i64 %v67396 to i128
  %t4165 = shl nuw i128 %t4164, 64
  %v219923 = or disjoint i128 %t4165, %t4163
  store i128 %v219923, ptr %t926, align 16
  %v219925 = or disjoint i128 %t4165, 1
  store i128 %v219925, ptr %t931, align 16
  store i128 %v219927, ptr %t936, align 16
  store i128 %v219929, ptr %t941, align 16
  %t4183 = zext i64 %v67292 to i128
  %v219931 = or disjoint i128 %t4183, 10090369008319124733952
  store i128 %v219931, ptr %t946, align 16
  store i128 10090369008319124733953, ptr %t947, align 16
  store i128 %v219935, ptr %t1152, align 16
  store i128 4722366482869645213697, ptr %t1157, align 16
  %t4203 = zext i64 %v67414 to i128
  %t4204 = zext i64 %v67328 to i128
  %t4205 = shl nuw i128 %t4204, 64
  %v219939 = or disjoint i128 %t4205, %t4203
  store i128 %v219939, ptr %t1162, align 16
  %t4208 = icmp ule i64 %v67396, %v67414
  %v67342 = sub i64 %v67414, %v67396
  %t4209 = icmp ule i64 %v67402, %v67342
  %v67344147 = and i1 %t4208, %t4209
  br i1 %v67344147, label %edge6178, label %edge6179

edge6178:                                         ; preds = %edge6176
  %t4211 = icmp uge i64 %v67403, %v67402
  %t4212 = icmp ule i64 %v67403, %v3720
  %v67349 = sub i64 %v67403, %v67402
  %t4213 = icmp ugt i64 %v67349, 2047
  %v67352148 = and i1 %t4211, %t4212
  %v67354149 = and i1 %v67352148, %t4213
  br i1 %v67354149, label %edge6180, label %edge6181

edge6179:                                         ; preds = %edge6176
  %t4217 = call fastcc { i64, i64 } @mech_compact_numeric_3009(i64 %v0, i64 %v219576, i64 %v8481)
  br label %block35544

edge6180:                                         ; preds = %edge6178
  %t4218 = call fastcc { i64, i64 } @mech_compact_numeric_3009_no_growth_composed(i64 %v219576, i64 %v8481)
  br label %block35544

edge6181:                                         ; preds = %edge6178
  %t4216 = call fastcc { i64, i64 } @mech_compact_numeric_3009_no_growth(i64 %v219576, i64 %v8481)
  br label %block35544

edge6185:                                         ; preds = %block35544
  %v67346 = extractvalue { i64, i64 } %t4216.pn, 1
  %t4220 = icmp eq i64 %v67345, 103169
  %v67363 = select i1 %t4220, i64 103425, i64 %v67345
  %t4222 = insertvalue { i64, i64 } poison, i64 %v67363, 0
  %t4223 = insertvalue { i64, i64 } %t4222, i64 %v67346, 1
  br label %common.ret

edge6186:                                         ; preds = %block35544
  %v219941 = load i128, ptr %t4225, align 8
  %v219942 = trunc i128 %v219941 to i64
  %t4226 = lshr i128 %v219941, 64
  %v219943 = trunc nuw i128 %t4226 to i64
  %v219945 = load i128, ptr %t4228, align 8
  %v219946 = trunc i128 %v219945 to i64
  %t4229 = lshr i128 %v219945, 64
  %v219947 = trunc nuw i128 %t4229 to i64
  %v219949 = load i128, ptr %t4231, align 8
  %v219950 = trunc i128 %v219949 to i64
  %t4232 = lshr i128 %v219949, 64
  %v219951 = trunc nuw i128 %t4232 to i64
  br i1 %t3701.not, label %block35555, label %edge6190

edge6190:                                         ; preds = %edge6186
  %v67384 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54926)
  %t4234.not = icmp eq i64 %v67384, 0
  br i1 %t4234.not, label %block35555, label %block35536

edge6196:                                         ; preds = %block35555
  %v67388 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66494)
  %t4236.not = icmp eq i64 %v67388, 0
  br i1 %t4236.not, label %edge5769, label %block35536

edge6220:                                         ; preds = %block2989
  %t4239.not = icmp eq i64 %v67402, 0
  br i1 %t4239.not, label %edge6865, label %edge6226

edge6220.thread:                                  ; preds = %edge6170
  %t4239.not717 = icmp eq i64 %v67402, 0
  br i1 %t4239.not717, label %edge6854, label %edge6226

edge6226:                                         ; preds = %edge6220.thread, %edge6220
  %v67760 = add i64 %v67402, -1
  %v67765 = add i64 %v67403, -1
  %t4242 = icmp ult i64 %v67765, %v3720
  br i1 %t4242, label %edge6279, label %edge6297

edge6279:                                         ; preds = %edge6226
  %v68074 = shl i64 %v67403, 2
  %v68078 = add i64 %v68077, %v68074
  %t4245 = inttoptr i64 %v68078 to ptr
  %t4246 = load i32, ptr %t4245, align 1
  %30 = and i32 %t4246, 65535
  %31 = zext nneg i32 %30 to i64
  %32 = lshr i32 %t4246, 16
  %33 = zext nneg i32 %32 to i64
  %t4253 = icmp ult i32 %t4246, 18874368
  br i1 %t4253, label %edge6297, label %edge6331

edge6297:                                         ; preds = %edge6226, %edge6279
  %v68454713 = phi i64 [ %33, %edge6279 ], [ 0, %edge6226 ]
  %v68101705711 = phi i64 [ %31, %edge6279 ], [ 0, %edge6226 ]
  %v68559 = shl nuw nsw i64 %v68454713, 2
  %v68872 = add i64 %v68871, %v68559
  %t4256 = inttoptr i64 %v68872 to ptr
  %t4257 = load i32, ptr %t4256, align 1
  %v68873 = zext i32 %t4257 to i64
  br label %edge6331

edge6331:                                         ; preds = %edge6297, %edge6279
  %v68454712 = phi i64 [ %v68454713, %edge6297 ], [ %33, %edge6279 ]
  %v68101705710 = phi i64 [ %v68101705711, %edge6297 ], [ %31, %edge6279 ]
  %v68893 = phi i64 [ %v68873, %edge6297 ], [ 0, %edge6279 ]
  %v69209 = shl i64 %v67396, 3
  %v69212 = add i64 %v69209, -8
  %v69213 = add i64 %v69212, %v67404
  %t4261 = inttoptr i64 %v69213 to ptr
  %v69214 = load i64, ptr %t4261, align 1
  %v69260 = add i64 %v69214, %v68893
  %v69265 = and i64 %v69260, 4294967295
  %v69303 = shl nuw nsw i64 %v68454712, 9
  %v69308 = or disjoint i64 %v69303, 1
  %v69324 = sub i64 %v67765, %v68101705710
  br label %block3108

edge6385:                                         ; preds = %edge6790
  %v69707 = shl nuw nsw i64 %v69377, 2
  %v69708 = add i64 %v69707, %v54921
  %t4266 = inttoptr i64 %v69708 to ptr
  %t4267 = load i32, ptr %t4266, align 1
  %34 = lshr i32 %t4267, 16
  %35 = zext nneg i32 %34 to i64
  br label %edge6394

edge6394:                                         ; preds = %edge6385, %edge6790
  %v69729 = phi i64 [ %35, %edge6385 ], [ 0, %edge6790 ]
  %v69759 = sub nsw i64 256, %v69729
  %v69764 = lshr i64 %v69759, 29
  %v69770 = and i64 %v69764, 7
  %v69775 = add nsw i64 %v69729, -1
  %v69781 = lshr i64 %v69775, %v69770
  %t4271 = icmp ult i64 %v69781, 256
  br i1 %t4271, label %edge6411, label %edge6418

edge6411:                                         ; preds = %edge6394
  %v70068 = add i64 %v69781, ptrtoint (ptr @mech_compact_table_18 to i64)
  %t4274 = inttoptr i64 %v70068 to ptr
  %t4275 = load i8, ptr %t4274, align 1
  %v70069 = zext i8 %t4275 to i64
  br label %edge6418

edge6418:                                         ; preds = %edge6394, %edge6411
  %v70087 = phi i64 [ %v70069, %edge6411 ], [ 0, %edge6394 ]
  %v70093 = shl nuw nsw i64 %v69770, 1
  %v70098 = add nuw nsw i64 %v70087, %v70093
  %t4277 = icmp samesign ult i64 %v70098, 32
  br i1 %t4277, label %edge6437, label %edge6453

edge6437:                                         ; preds = %edge6418
  %v70103 = shl nuw nsw i64 %v70098, 2
  %v70416 = add i64 %v70415, %v70103
  %t4280 = inttoptr i64 %v70416 to ptr
  %t4281 = load i32, ptr %t4280, align 1
  %v70417 = zext i32 %t4281 to i64
  br label %edge6453

edge6453:                                         ; preds = %edge6437, %edge6418
  %v70437 = phi i64 [ %v70417, %edge6437 ], [ 0, %edge6418 ]
  br i1 %t4263, label %edge6470, label %edge6492

edge6470:                                         ; preds = %edge6453
  %v70749 = shl nuw nsw i64 %v69377, 2
  %v70750 = add i64 %v70749, %v54921
  %t4285 = inttoptr i64 %v70750 to ptr
  %t4286 = load i32, ptr %t4285, align 1
  %36 = and i32 %t4286, 65535
  %37 = add nsw i32 %36, -3
  %t4287 = icmp ult i32 %37, 256
  br i1 %t4287, label %edge6652, label %edge6492

edge6492:                                         ; preds = %edge6453, %edge6470
  br i1 %t23.not, label %block37800, label %edge6495

edge6495:                                         ; preds = %edge6492
  %v70869 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t4290.not = icmp eq i64 %v70869, 0
  br i1 %t4290.not, label %block37800, label %block37732

edge6501:                                         ; preds = %block37800
  %v70873 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219198)
  %t4292.not = icmp eq i64 %v70873, 0
  br i1 %t4292.not, label %block37808, label %block37732

edge6513:                                         ; preds = %block37808
  %v70881 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54920)
  %t4296.not = icmp eq i64 %v70881, 0
  br i1 %t4296.not, label %block37812, label %block37732

edge6519:                                         ; preds = %block37812
  %v70885 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54926)
  %t4298.not = icmp eq i64 %v70885, 0
  br i1 %t4298.not, label %block37816, label %block37732

edge6525:                                         ; preds = %block37816
  %v70889 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54944)
  %t4300.not = icmp eq i64 %v70889, 0
  br i1 %t4300.not, label %block37820, label %block37732

edge6531:                                         ; preds = %block37820
  %v70893 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54950)
  %t4302.not = icmp eq i64 %v70893, 0
  br i1 %t4302.not, label %block37824, label %block37732

edge6537:                                         ; preds = %block37824
  %v70897 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54956)
  %t4304.not = icmp eq i64 %v70897, 0
  br i1 %t4304.not, label %block37828, label %block37732

edge6543:                                         ; preds = %block37828
  %v70901 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54938)
  %t4306.not = icmp eq i64 %v70901, 0
  br i1 %t4306.not, label %block37832, label %block37732

edge6549:                                         ; preds = %block37832
  %v70905 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219242)
  %t4308.not = icmp eq i64 %v70905, 0
  br i1 %t4308.not, label %block37836, label %block37732

edge6553:                                         ; preds = %block37836
  %v70907 = load i64, ptr %t7636, align 1
  %v70908 = add i64 %v70907, -1
  store i64 %v70908, ptr %t7636, align 1
  %t4312.not = icmp eq i64 %v70908, 0
  br i1 %t4312.not, label %edge6556, label %edge6563

edge6556:                                         ; preds = %edge6553
  %v70909 = call i64 @mech_compact_free(i64 %v0, i64 %v54950)
  %t4313.not = icmp eq i64 %v70909, 0
  br i1 %t4313.not, label %edge6563, label %block37732

edge6563:                                         ; preds = %block37836, %edge6553, %edge6556
  %t4314.not = icmp eq i64 %v64765, 0
  br i1 %t4314.not, label %block37860, label %edge6566

edge6566:                                         ; preds = %edge6563
  %v70929 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64765)
  %t4315.not = icmp eq i64 %v70929, 0
  br i1 %t4315.not, label %block37860, label %block37732

edge6572:                                         ; preds = %block37860
  %v70933 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64777)
  %t4317.not = icmp eq i64 %v70933, 0
  br i1 %t4317.not, label %block37864, label %block37732

edge6578:                                         ; preds = %block37864
  %v70937 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64795)
  %t4319.not = icmp eq i64 %v70937, 0
  br i1 %t4319.not, label %block37868, label %block37732

edge6584:                                         ; preds = %block37868
  %v70941 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64807)
  %t4321.not = icmp eq i64 %v70941, 0
  br i1 %t4321.not, label %block37872, label %block37732

edge6590:                                         ; preds = %block37872
  %v70945 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54926)
  %t4323.not = icmp eq i64 %v70945, 0
  br i1 %t4323.not, label %block37876, label %block37732

edge6594:                                         ; preds = %block37876
  %v70947 = load i64, ptr %t4451.pre-phi, align 1
  %v70948 = add i64 %v70947, -1
  store i64 %v70948, ptr %t4451.pre-phi, align 1
  %t4327.not = icmp eq i64 %v70948, 0
  br i1 %t4327.not, label %edge6597, label %edge6605

edge6597:                                         ; preds = %edge6594
  %v70949 = call i64 @mech_compact_free(i64 %v0, i64 %v58156223382235022393)
  %t4328.not = icmp eq i64 %v70949, 0
  br i1 %t4328.not, label %edge6605, label %block37732

edge6605:                                         ; preds = %edge6597, %edge6594, %block37876
  %t4329.not = icmp eq i64 %v64918, 0
  br i1 %t4329.not, label %block37904, label %edge6608

edge6608:                                         ; preds = %edge6605
  %v70973 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64918)
  %t4330.not = icmp eq i64 %v70973, 0
  br i1 %t4330.not, label %block37904, label %block37732

edge6614:                                         ; preds = %block37904
  %v70977 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64930)
  %t4332.not = icmp eq i64 %v70977, 0
  br i1 %t4332.not, label %block37908, label %block37732

edge6620:                                         ; preds = %block37908
  %v70981 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64942)
  %t4334.not = icmp eq i64 %v70981, 0
  br i1 %t4334.not, label %block37912, label %block37732

edge6626:                                         ; preds = %block37912
  %v70985 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66494)
  %t4336.not = icmp eq i64 %v70985, 0
  br i1 %t4336.not, label %block37916, label %block37732

edge6632:                                         ; preds = %block37916
  %v70989 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v67328)
  %t4338.not = icmp eq i64 %v70989, 0
  br i1 %t4338.not, label %block37920, label %block37732

edge6638:                                         ; preds = %block37920
  %v70993 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54926)
  %t4340.not = icmp eq i64 %v70993, 0
  br i1 %t4340.not, label %block37924, label %block37732

edge6644:                                         ; preds = %block37924
  %v70997 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66494)
  %t4342.not = icmp eq i64 %v70997, 0
  br i1 %t4342.not, label %common.ret, label %block37732

edge6652:                                         ; preds = %edge6470
  %38 = zext nneg i32 %36 to i64
  %v71015 = add nsw i64 %38, -2
  %v72401 = shl nuw nsw i64 %v69729, 9
  %umax21633 = call i64 @llvm.umax.i64(i64 %v69382, i64 %v71015)
  br label %block3215

edge6686:                                         ; preds = %edge6768
  %v71399 = shl nuw nsw i64 %v71087, 2
  %v71400 = add i64 %v71399, %v67292
  %t4350 = inttoptr i64 %v71400 to ptr
  %t4351 = load i32, ptr %t4350, align 1
  %v71401 = zext i32 %t4351 to i64
  br label %edge6696

edge6696:                                         ; preds = %edge6768, %edge6686
  %v71421 = phi i64 [ %v71401, %edge6686 ], [ 0, %edge6768 ]
  %v71434 = add nuw nsw i64 %v71421, %v70437
  %v71452 = sub i64 %v67396, %v71087
  %t4352 = icmp ult i64 %v71452, %v67396
  br i1 %t4352, label %edge6718, label %edge6726

edge6718:                                         ; preds = %edge6696
  %v71764 = shl i64 %v71452, 3
  %v71765 = add i64 %v71764, %v67404
  %t4355 = inttoptr i64 %v71765 to ptr
  %v71766 = load i64, ptr %t4355, align 1
  br label %edge6726

edge6726:                                         ; preds = %edge6696, %edge6718
  %v71786 = phi i64 [ %v71766, %edge6718 ], [ 0, %edge6696 ]
  %v71810 = add i64 %v71434, %v71786
  %v71815 = and i64 %v71810, 4294967295
  %t4356 = icmp samesign ult i64 %v71815, %v71074
  %spec.select30 = call i64 @llvm.umin.i64(i64 %v71815, i64 %v71074)
  %v72406 = or i64 %v71087, %v72401
  %v72411 = and i64 %v72406, 4294967295
  %v72439 = select i1 %t4356, i64 %v72411, i64 %v71079
  %v72435 = add i64 %v71069, 1
  br label %block3215

edge6768:                                         ; preds = %block3215
  %v71087 = add i64 %v71069, 3
  %t4347 = icmp ult i64 %v71087, 547
  br i1 %t4347, label %edge6686, label %edge6696

edge6781:                                         ; preds = %block3215
  %v72858 = add nuw nsw i64 %v69377, 1
  br label %block3108

edge6790:                                         ; preds = %block3108
  %t4263 = icmp ult i64 %v69377, %v3720
  br i1 %t4263, label %edge6385, label %edge6394

edge6805:                                         ; preds = %block3108
  %v73656 = shl nuw i64 %v69392, 32
  %v73661 = or i64 %v73656, %v69387
  %t4369 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v67396, i64 1)
  %v73669 = extractvalue { i64, i1 } %t4369, 0
  %t4370 = extractvalue { i64, i1 } %t4369, 1
  br i1 %t4370, label %block39346, label %edge6817

edge6817:                                         ; preds = %edge6805
  %t4373.not = icmp ugt i64 %v73669, %v67414
  br i1 %t4373.not, label %edge6820, label %block39350

edge6820:                                         ; preds = %edge6817
  %t4375.not = icmp eq i64 %v67414, 0
  br i1 %t4375.not, label %block39353, label %edge6821

edge6821:                                         ; preds = %edge6820
  %39 = add i64 %v67414, -16385
  %v73679144 = icmp ult i64 %39, -15873
  br i1 %v73679144, label %edge6824, label %edge6833

edge6824:                                         ; preds = %edge6821
  %t4379 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v67414, i64 %v67414)
  %v73680 = extractvalue { i64, i1 } %t4379, 0
  %t4380 = extractvalue { i64, i1 } %t4379, 1
  br i1 %t4380, label %block39346, label %block39353

edge6833:                                         ; preds = %edge6821
  %t4387 = mul nuw nsw i64 %v67414, 3
  %t4391 = add nuw nsw i64 %t4387, 1
  %v73697 = lshr i64 %t4391, 1
  br label %block39353

edge6837:                                         ; preds = %block39350
  %t4397 = inttoptr i64 %v67328 to ptr
  %v73705 = load i64, ptr %t4397, align 1
  %t4398 = icmp eq i64 %v73705, 1
  %t4399 = icmp ule i64 %v73674, %v67414
  %v73708145 = and i1 %t4399, %t4398
  br i1 %v73708145, label %block39360, label %block39361

edge6842:                                         ; preds = %block39361
  %v73715 = load i64, ptr %ss1, align 8
  %v73716 = load i64, ptr %t606, align 8
  %v73717 = load i64, ptr %t607, align 8
  br label %block39360

edge6844:                                         ; preds = %block39360
  %t4422 = inttoptr i64 %v73704 to ptr
  %v73723 = load i64, ptr %t4422, align 1
  %v73724 = add i64 %v73723, 1
  store i64 %v73724, ptr %t4422, align 1
  br label %edge6863

edge6854:                                         ; preds = %edge6220.thread
  %v73756 = add i64 %v67329, 1
  store i64 %v73756, ptr %t4149, align 1
  br label %edge6865

edge6863:                                         ; preds = %edge6844, %block39360
  %t4441.not = or i1 %t4148.not, %t4419
  br i1 %t4441.not, label %block2989.backedge, label %edge6887

block2989.backedge:                               ; preds = %edge6863, %edge6887, %edge6890
  br label %block2989

edge6865:                                         ; preds = %edge6220, %edge6854
  br i1 %t3701.not, label %block39399, label %edge6868

edge6868:                                         ; preds = %edge6865
  %v73795 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54926)
  %t4433.not = icmp eq i64 %v73795, 0
  br i1 %t4433.not, label %block39399, label %block39394

edge6874:                                         ; preds = %block39399
  %v73799 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66494)
  %t4435.not = icmp eq i64 %v73799, 0
  br i1 %t4435.not, label %edge6878, label %block39394

edge6878:                                         ; preds = %edge6874, %block39399
  %t4436.not = icmp eq i64 %v67328, 0
  br i1 %t4436.not, label %edge5769.thread, label %edge6879

edge6879:                                         ; preds = %edge6878
  %t4437 = inttoptr i64 %v67328 to ptr
  %v73805 = load i64, ptr %t4437, align 1
  %v73806 = add i64 %v73805, -1
  store i64 %v73806, ptr %t4437, align 1
  %t4439.not = icmp eq i64 %v73806, 0
  br i1 %t4439.not, label %edge6882, label %edge5769

edge6882:                                         ; preds = %edge6879
  %v73807 = call i64 @mech_compact_free(i64 %v0, i64 %v67328)
  %t4440.not = icmp eq i64 %v73807, 0
  br i1 %t4440.not, label %edge5769, label %block39394

edge6887:                                         ; preds = %edge6863
  %t4442 = inttoptr i64 %v67328 to ptr
  %v73809 = load i64, ptr %t4442, align 1
  %v73810 = add i64 %v73809, -1
  store i64 %v73810, ptr %t4442, align 1
  %t4444.not = icmp eq i64 %v73810, 0
  br i1 %t4444.not, label %edge6890, label %block2989.backedge

edge6890:                                         ; preds = %edge6887
  %v73811 = call i64 @mech_compact_free(i64 %v0, i64 %v67328)
  %t4445.not = icmp eq i64 %v73811, 0
  br i1 %t4445.not, label %block2989.backedge, label %block39394

edge6923:                                         ; preds = %block42493.thread, %block42493, %edge7373, %edge6926, %edge6925
  %v7458422199 = phi i64 [ %v7458422200, %edge6926 ], [ %v7458422200, %edge6925 ], [ %v745842219822208, %edge7373 ], [ %v745842219822208, %block42493 ], [ %v745842219822208, %block42493.thread ]
  %v7458322195 = phi i64 [ %v7458322197, %edge6926 ], [ %v7458322197, %edge6925 ], [ %v745832219422209, %edge7373 ], [ %v745832219422209, %block42493 ], [ %v745832219422209, %block42493.thread ]
  %v7458222191 = phi i64 [ %v7458222193, %edge6926 ], [ %v7458222193, %edge6925 ], [ %v745822219022210, %edge7373 ], [ %v745822219022210, %block42493 ], [ %v745822219022210, %block42493.thread ]
  %v7458122187 = phi i64 [ %v7458122188, %edge6926 ], [ %v7458122188, %edge6925 ], [ %v745812218922211, %edge7373 ], [ %v745812218922211, %block42493 ], [ %v745812218922211, %block42493.thread ]
  %v7458022184 = phi i64 [ %v7458022186, %edge6926 ], [ %v7458022186, %edge6925 ], [ %v745802218322212, %edge7373 ], [ %v745802218322212, %block42493 ], [ %v745802218322212, %block42493.thread ]
  %v7457922180 = phi i64 [ %v7457922182, %edge6926 ], [ %v7457922182, %edge6925 ], [ %v745792217922213, %edge7373 ], [ %v745792217922213, %block42493 ], [ %v745792217922213, %block42493.thread ]
  %v80922 = phi i64 [ %v58156223382235022393, %edge6926 ], [ 0, %edge6925 ], [ %v79892, %edge7373 ], [ %v79892, %block42493 ], [ %v75597, %block42493.thread ]
  %v80928 = phi i64 [ %v58151223362235222389, %edge6926 ], [ %v58151223362235222389, %edge6925 ], [ %v79890, %edge7373 ], [ %v79890, %block42493 ], [ %v75637, %block42493.thread ]
  %v80930 = phi i64 [ %v58153223372235122391, %edge6926 ], [ %v58153223372235122391, %edge6925 ], [ %v79891, %edge7373 ], [ %v79891, %block42493 ], [ %v75629, %block42493.thread ]
  br i1 %t3962, label %edge7381, label %edge7390

edge6925:                                         ; preds = %edge5769.thread, %edge5769
  %v7458422200 = phi i64 [ 0, %edge5769.thread ], [ %v74584, %edge5769 ]
  %v7458322197 = phi i64 [ %v67396, %edge5769.thread ], [ %v74583, %edge5769 ]
  %v7458222193 = phi i64 [ 1, %edge5769.thread ], [ %v74582, %edge5769 ]
  %v7458122188 = phi i64 [ %v67414, %edge5769.thread ], [ %v74581, %edge5769 ]
  %v7458022186 = phi i64 [ %v67396, %edge5769.thread ], [ %v74580, %edge5769 ]
  %v7457922182 = phi i64 [ %v67404, %edge5769.thread ], [ %v74579, %edge5769 ]
  br i1 %t3900.not138223402234922395, label %edge6923, label %edge6926

edge6926:                                         ; preds = %edge6925
  %v74566 = load i64, ptr %t4451.pre-phi, align 1
  %v74567 = add i64 %v74566, 1
  store i64 %v74567, ptr %t4451.pre-phi, align 1
  br label %edge6923

edge6930:                                         ; preds = %edge5769
  %t4453.not = icmp eq i64 %v74584, 0
  br i1 %t4453.not, label %edge6966, label %edge6931

edge6931:                                         ; preds = %edge6930
  %t4454 = inttoptr i64 %v74584 to ptr
  %v74585 = load i64, ptr %t4454, align 1
  %v74586 = add i64 %v74585, 1
  store i64 %v74586, ptr %t4454, align 1
  br label %edge6966

edge6946:                                         ; preds = %edge6969
  %t4463 = insertvalue { i64, i64 } poison, i64 %v75233, 0
  %t4464 = insertvalue { i64, i64 } %t4463, i64 3431, 1
  br label %common.ret

edge6966:                                         ; preds = %edge5769.thread, %edge6931, %edge6930
  %t4453.not22214 = phi i1 [ true, %edge6930 ], [ false, %edge6931 ], [ true, %edge5769.thread ]
  %v745792217922213 = phi i64 [ %v74579, %edge6930 ], [ %v74579, %edge6931 ], [ %v67404, %edge5769.thread ]
  %v745802218322212 = phi i64 [ %v74580, %edge6930 ], [ %v74580, %edge6931 ], [ %v67396, %edge5769.thread ]
  %v745812218922211 = phi i64 [ %v74581, %edge6930 ], [ %v74581, %edge6931 ], [ %v67414, %edge5769.thread ]
  %v745822219022210 = phi i64 [ %v74582, %edge6930 ], [ %v74582, %edge6931 ], [ 1, %edge5769.thread ]
  %v745832219422209 = phi i64 [ %v74583, %edge6930 ], [ %v74583, %edge6931 ], [ %v67396, %edge5769.thread ]
  %v745842219822208 = phi i64 [ 0, %edge6930 ], [ %v74584, %edge6931 ], [ 0, %edge5769.thread ]
  %v75292 = call i64 @mech_compact_allocate(i64 %v0, i64 320, i64 4, i64 4, i64 1, i64 %v3194)
  %t4477.not = icmp eq i64 %v75292, 0
  br i1 %t4477.not, label %edge6968, label %block39892

edge6968:                                         ; preds = %edge6966
  %v75295 = load i64, ptr %ss1, align 8
  %v75296 = load i64, ptr %t606, align 8
  %v75297 = load i64, ptr %t607, align 8
  %t4480.not = icmp eq i64 %v75297, 0
  br i1 %t4480.not, label %block3417.preheader, label %edge6969

edge6969:                                         ; preds = %edge6968
  %t4481 = inttoptr i64 %v75297 to ptr
  %v75298 = load i64, ptr %t4481, align 1
  %v75229 = add i64 %v75298, 2
  store i64 %v75229, ptr %t4481, align 1
  %v75233 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v75297)
  %t4462.not = icmp eq i64 %v75233, 0
  br i1 %t4462.not, label %block3417.preheader, label %edge6946

block3417.preheader:                              ; preds = %edge6968, %edge6969
  br label %block3417

edge6987:                                         ; preds = %block3417
  %t4486 = inttoptr i64 %v75597 to ptr
  %v75598 = load i64, ptr %t4486, align 1
  %t4487 = icmp eq i64 %v75598, 1
  %t4489 = icmp ugt i64 %v75629, 319
  %or.cond314 = select i1 %t4487, i1 %t4489, i1 false
  br i1 %or.cond314, label %edge6991, label %edge7025.thread

edge6991:                                         ; preds = %edge6987
  %t4492 = zext i64 %v75631 to i128
  %t4493 = shl nuw i128 %t4492, 64
  %v219959 = or disjoint i128 %t4493, %t4491
  store i128 %v219959, ptr %ss2, align 16
  %t4496 = zext i64 %v745792217922213 to i128
  %t4497 = shl nuw i128 %t4496, 64
  %v219961 = or disjoint i128 %t4497, %t4495
  store i128 %v219961, ptr %t926, align 16
  %t4500 = zext i64 %v745802218322212 to i128
  %t4501 = zext i64 %v745822219022210 to i128
  %t4502 = shl nuw i128 %t4501, 64
  %v219963 = or disjoint i128 %t4502, %t4500
  store i128 %v219963, ptr %t931, align 16
  %t4505 = zext i64 %v745832219422209 to i128
  %t4506 = zext i64 %v75637 to i128
  %t4507 = shl nuw i128 %t4506, 64
  %v219965 = or disjoint i128 %t4507, %t4505
  store i128 %v219965, ptr %t936, align 16
  store i128 18446744073709551936, ptr %t941, align 16
  store i128 %v219969, ptr %t946, align 16
  store i128 18446744073709551875, ptr %t947, align 16
  store i128 %v219973, ptr %t1152, align 16
  store i128 18446744073709551872, ptr %t1157, align 16
  %t4536 = zext i64 %v75597 to i128
  %t4537 = shl nuw i128 %t4536, 64
  %v219977 = or disjoint i128 %t4537, 256
  store i128 %v219977, ptr %t1162, align 16
  store i64 %v75629, ptr %t1167, align 16
  %t4541 = call fastcc { i64, i64 } @mech_compact_numeric_3447(i64 %v219576, i64 %v8481)
  %v75603 = extractvalue { i64, i64 } %t4541, 0
  %t4542.not = icmp eq i64 %v75603, 0
  br i1 %t4542.not, label %edge6994, label %edge6993

edge6993:                                         ; preds = %edge6991
  %v75604 = extractvalue { i64, i64 } %t4541, 1
  %t4543 = icmp eq i64 %v75603, 103169
  %v75607 = select i1 %t4543, i64 103425, i64 %v75603
  %t4545 = insertvalue { i64, i64 } poison, i64 %v75607, 0
  %t4546 = insertvalue { i64, i64 } %t4545, i64 %v75604, 1
  br label %common.ret

edge6994:                                         ; preds = %edge6991
  br i1 %t4453.not22214, label %edge7360, label %edge6995

edge6995:                                         ; preds = %edge6994
  %t4548 = inttoptr i64 %v745842219822208 to ptr
  %v75610 = load i64, ptr %t4548, align 1
  %v75611 = add i64 %v75610, -1
  store i64 %v75611, ptr %t4548, align 1
  %t4550.not = icmp eq i64 %v75611, 0
  br i1 %t4550.not, label %edge6998, label %edge7360

edge6998:                                         ; preds = %edge6995
  %v75612 = call i64 @mech_compact_free(i64 %v0, i64 %v745842219822208)
  %t4551.not = icmp eq i64 %v75612, 0
  br i1 %t4551.not, label %edge7360, label %edge6999

edge6999:                                         ; preds = %edge6998
  %t4552 = insertvalue { i64, i64 } poison, i64 %v75612, 0
  %t4553 = insertvalue { i64, i64 } %t4552, i64 3447, 1
  br label %common.ret

edge7025:                                         ; preds = %block3417
  %t4554.not = icmp eq i64 %v75631, %v55001
  br i1 %t4554.not, label %edge7288, label %edge7031

edge7025.thread:                                  ; preds = %edge6987
  %t4554.not755 = icmp eq i64 %v75631, %v55001
  br i1 %t4554.not755, label %edge7278, label %edge7031

edge7031:                                         ; preds = %edge7025.thread, %edge7025
  %v76000 = sub i64 %v75440, %v75631
  %t4557 = icmp ult i64 %v76000, %v745802218322212
  br i1 %t4557, label %edge7056, label %edge7138

edge7056:                                         ; preds = %edge7031
  %v76318 = shl i64 %v76000, 3
  %v76319 = add i64 %v76318, %v745792217922213
  %t4560 = inttoptr i64 %v76319 to ptr
  %v76320 = load i64, ptr %t4560, align 1
  %40 = lshr i64 %v76320, 32
  %v76485 = and i64 %40, 511
  %v76500 = lshr i64 %v76320, 41
  %t4563 = icmp eq i64 %v76485, 1
  br i1 %t4563, label %edge7070, label %edge7121

edge7070:                                         ; preds = %edge7056
  %v76891 = shl nuw nsw i64 %v76500, 2
  %v77208 = and i64 %v76891, 1020
  %v77209 = add i64 %v77208, %v75637
  %t4567 = inttoptr i64 %v77209 to ptr
  %t4568 = load i32, ptr %t4567, align 1
  %v77240 = add i32 %t4568, 1
  br i1 %t4485.not, label %block40943, label %edge7099

edge7099:                                         ; preds = %edge7070
  %t4575 = inttoptr i64 %v75597 to ptr
  %v77282 = load i64, ptr %t4575, align 1
  %t4576 = icmp eq i64 %v77282, 1
  br i1 %t4576, label %block40942, label %block40943

edge7103:                                         ; preds = %block40943
  %t4603 = insertvalue { i64, i64 } poison, i64 %v77291, 0
  %t4604 = insertvalue { i64, i64 } %t4603, i64 3524, 1
  br label %common.ret

edge7104:                                         ; preds = %block40943
  %v77292 = load i64, ptr %ss1, align 8
  %v77293 = load i64, ptr %t606, align 8
  %v77294 = load i64, ptr %t607, align 8
  %.pre21769 = add i64 %v77292, %v77208
  %.pre21770 = inttoptr i64 %.pre21769 to ptr
  br label %block40942

edge7107:                                         ; preds = %block40942
  %t4600 = inttoptr i64 %v77281 to ptr
  %v77299 = load i64, ptr %t4600, align 1
  %v77300 = add i64 %v77299, 1
  store i64 %v77300, ptr %t4600, align 1
  br label %block40947

edge7121:                                         ; preds = %edge7056
  %t4605 = icmp samesign ult i64 %v76485, 259
  br i1 %t4605, label %edge7138, label %edge7173

edge7138:                                         ; preds = %edge7031, %edge7121
  %v76485739747 = phi i64 [ %v76485, %edge7121 ], [ 0, %edge7031 ]
  %v76500740745 = phi i64 [ %v76500, %edge7121 ], [ 0, %edge7031 ]
  %v77634 = add i64 %v76485739747, ptrtoint (ptr @mech_compact_table_7 to i64)
  %t4608 = inttoptr i64 %v77634 to ptr
  %t4609 = load i8, ptr %t4608, align 1
  %v77635 = zext nneg i8 %t4609 to i64
  %41 = add nuw nsw i64 %v77635, 257
  %t4610 = icmp ult i8 %t4609, 63
  br i1 %t4610, label %edge7173, label %block41439

edge7173:                                         ; preds = %edge7121, %edge7138
  %v77660754 = phi i64 [ %41, %edge7138 ], [ 257, %edge7121 ]
  %v76500740744753 = phi i64 [ %v76500740745, %edge7138 ], [ %v76500, %edge7121 ]
  %v76485739746752 = phi i64 [ %v76485739747, %edge7138 ], [ %v76485, %edge7121 ]
  %v78092 = shl nuw nsw i64 %v77660754, 2
  %v78093 = add i64 %v78092, %v75637
  %t4613 = inttoptr i64 %v78093 to ptr
  %t4614 = load i32, ptr %t4613, align 1
  %42 = add i32 %t4614, 1
  br i1 %t4485.not, label %block41444, label %edge7174

edge7174:                                         ; preds = %edge7173
  %t4622 = inttoptr i64 %v75597 to ptr
  %v78165 = load i64, ptr %t4622, align 1
  %t4623 = icmp eq i64 %v78165, 1
  br i1 %t4623, label %block41442, label %block41444

edge7179:                                         ; preds = %block41444
  %v78175 = load i64, ptr %ss1, align 8
  %v78176 = load i64, ptr %t606, align 8
  %v78177 = load i64, ptr %t607, align 8
  %.pre21771 = add i64 %v78175, %v78092
  %.pre21772 = inttoptr i64 %.pre21771 to ptr
  br label %block41442

edge7182:                                         ; preds = %block41442
  %t4647 = inttoptr i64 %v78164 to ptr
  %v78182 = load i64, ptr %t4647, align 1
  %v78183 = add i64 %v78182, 1
  store i64 %v78183, ptr %t4647, align 1
  br label %block41448

edge7201:                                         ; preds = %block41448
  %v78533 = add i64 %v78236, ptrtoint (ptr @mech_compact_table_18 to i64)
  %t4657 = inttoptr i64 %v78533 to ptr
  %t4658 = load i8, ptr %t4657, align 1
  %v78534 = zext i8 %t4658 to i64
  br label %edge7208

edge7208:                                         ; preds = %block41448, %edge7201
  %v78552 = phi i64 [ %v78534, %edge7201 ], [ 0, %block41448 ]
  %v78558 = shl nuw nsw i64 %v78225, 1
  %v78563 = add nuw nsw i64 %v78552, %v78558
  %t4660 = icmp samesign ult i64 %v78563, 32
  br i1 %t4660, label %edge7245, label %block41932

edge7245:                                         ; preds = %edge7208
  %v78676 = shl nuw nsw i64 %v78563, 2
  %v78998 = or disjoint i64 %v78676, 1152
  %v78999 = add i64 %v78998, %v78159
  %t4663 = inttoptr i64 %v78999 to ptr
  %t4664 = load i32, ptr %t4663, align 1
  %43 = add i32 %t4664, 1
  %t4671.not = icmp eq i64 %v78184, 0
  br i1 %t4671.not, label %block41938, label %edge7248

edge7248:                                         ; preds = %edge7245
  %v79068 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v78184)
  %t4672.not = icmp eq i64 %v79068, 0
  br i1 %t4672.not, label %block41938, label %block41932

edge7252:                                         ; preds = %block41938
  %t4674 = inttoptr i64 %v78164 to ptr
  %v79075 = load i64, ptr %t4674, align 1
  %t4675 = icmp eq i64 %v79075, 1
  br i1 %t4675, label %block41935, label %block41941

edge7257:                                         ; preds = %block41941
  %v79085 = load i64, ptr %ss1, align 8
  %v79086 = load i64, ptr %t606, align 8
  %v79087 = load i64, ptr %t607, align 8
  %.pre21773 = add i64 %v79085, %v78998
  %.pre21774 = inttoptr i64 %.pre21773 to ptr
  br label %block41935

edge7260:                                         ; preds = %block41935
  %t4699 = inttoptr i64 %v79074 to ptr
  %v79092 = load i64, ptr %t4699, align 1
  %v79093 = add i64 %v79092, 1
  store i64 %v79093, ptr %t4699, align 1
  br label %block41945

edge7265:                                         ; preds = %block41945
  %v79098 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v78164)
  %t4703.not = icmp eq i64 %v79098, 0
  br i1 %t4703.not, label %edge7325, label %block41932

edge7278:                                         ; preds = %edge7025.thread
  %v79140 = add i64 %v75598, 1
  store i64 %v79140, ptr %t4486, align 1
  br label %edge7288

edge7288:                                         ; preds = %edge7025, %edge7278
  br i1 %t4453.not22214, label %edge7296, label %edge7291

edge7291:                                         ; preds = %edge7288
  %v79170 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v745842219822208)
  %t4712.not = icmp eq i64 %v79170, 0
  br i1 %t4712.not, label %edge7296, label %block41982

edge7296:                                         ; preds = %edge7288, %edge7291
  %t4713.not = icmp eq i64 %v75597, 0
  br i1 %t4713.not, label %edge7344, label %edge7297

edge7297:                                         ; preds = %edge7296
  %t4714 = inttoptr i64 %v75597 to ptr
  %v79180 = load i64, ptr %t4714, align 1
  %v79181 = add i64 %v79180, -1
  store i64 %v79181, ptr %t4714, align 1
  %t4716.not = icmp eq i64 %v79181, 0
  br i1 %t4716.not, label %edge7300, label %edge7360

edge7300:                                         ; preds = %edge7297
  %v79182 = call i64 @mech_compact_free(i64 %v0, i64 %v75597)
  %t4717.not = icmp eq i64 %v79182, 0
  br i1 %t4717.not, label %edge7360, label %block41982

edge7305:                                         ; preds = %block40947
  %t4719 = inttoptr i64 %v75597 to ptr
  %v79184 = load i64, ptr %t4719, align 1
  %v79185 = add i64 %v79184, -1
  store i64 %v79185, ptr %t4719, align 1
  %t4721.not = icmp eq i64 %v79185, 0
  br i1 %t4721.not, label %edge7308, label %edge7325

edge7308:                                         ; preds = %edge7305
  %v79186 = call i64 @mech_compact_free(i64 %v0, i64 %v75597)
  %t4722.not = icmp eq i64 %v79186, 0
  br i1 %t4722.not, label %edge7325, label %block41982

edge7325:                                         ; preds = %block40947, %block41945, %edge7265, %edge7305, %edge7308
  %v76485738.ph.pn = phi i64 [ 1, %edge7308 ], [ 1, %block40947 ], [ 1, %edge7305 ], [ %v76485739746752, %edge7265 ], [ %v76485739746752, %block41945 ]
  %v79117779 = phi i64 [ %v77281, %edge7308 ], [ %v77281, %block40947 ], [ %v77281, %edge7305 ], [ %v79074, %edge7265 ], [ %v79074, %block41945 ]
  %v79114778 = phi i64 [ %v77278, %edge7308 ], [ %v77278, %block40947 ], [ %v77278, %edge7305 ], [ %v79071, %edge7265 ], [ %v79071, %block41945 ]
  %v79112777 = phi i64 [ %v77276, %edge7308 ], [ %v77276, %block40947 ], [ %v77276, %edge7305 ], [ %v79069, %edge7265 ], [ %v79069, %block41945 ]
  %v79109780 = add i64 %v76485738.ph.pn, %v75631
  br label %block3417

edge7344:                                         ; preds = %edge7296
  %v79808 = add i64 %v75637, 1024
  %t4725 = inttoptr i64 %v79808 to ptr
  %t4726 = load i32, ptr %t4725, align 1
  %v79838 = add i32 %t4726, 1
  br label %block42489

edge7360:                                         ; preds = %edge6998, %edge6994, %edge6995, %edge7300, %edge7297
  %v7980822220 = add i64 %v75637, 1024
  %t472522221 = inttoptr i64 %v7980822220 to ptr
  %t472622222 = load i32, ptr %t472522221, align 1
  %v7983822223 = add i32 %t472622222, 1
  %t4732 = inttoptr i64 %v75597 to ptr
  %v79880 = load i64, ptr %t4732, align 1
  %t4733 = icmp eq i64 %v79880, 1
  br i1 %t4733, label %block42493.thread, label %block42489

block42493.thread:                                ; preds = %edge7360
  store i32 %v7983822223, ptr %t472522221, align 1
  br label %edge6923

edge7368:                                         ; preds = %block42488
  %t4757 = inttoptr i64 %v79892 to ptr
  %v79897 = load i64, ptr %t4757, align 1
  %v79898 = add i64 %v79897, 1
  store i64 %v79898, ptr %t4757, align 1
  br label %block42493

edge7373:                                         ; preds = %block42493
  %v79903 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v7950122238)
  %t4761.not = icmp eq i64 %v79903, 0
  br i1 %t4761.not, label %edge6923, label %block42484

edge7381:                                         ; preds = %edge6923
  %t4766.not = icmp eq i64 %v64795, 0
  br i1 %t4766.not, label %block44055, label %edge7385

edge7385:                                         ; preds = %edge7381
  %v79930 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64795)
  %t4767.not = icmp eq i64 %v79930, 0
  br i1 %t4767.not, label %block44055, label %edge7386

edge7386:                                         ; preds = %edge7385
  %t4768 = insertvalue { i64, i64 } poison, i64 %v79930, 0
  %t4769 = insertvalue { i64, i64 } %t4768, i64 3675, 1
  br label %common.ret

edge7390:                                         ; preds = %edge6923
  br i1 %t3927, label %edge7391, label %edge7404

edge7391:                                         ; preds = %edge7390
  %t4772.not = icmp eq i64 %v64795, 0
  br i1 %t4772.not, label %edge11167, label %edge7395

edge7395:                                         ; preds = %edge7391
  %v80222 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64795)
  %t4773.not = icmp eq i64 %v80222, 0
  br i1 %t4773.not, label %edge11167, label %edge7396

edge7396:                                         ; preds = %edge7395
  %t4774 = insertvalue { i64, i64 } poison, i64 %v80222, 0
  %t4775 = insertvalue { i64, i64 } %t4774, i64 3682, 1
  br label %common.ret

edge7404:                                         ; preds = %edge7390
  %t4776 = icmp eq i64 %v64742, 1
  br i1 %t4776, label %edge7416, label %edge7406

edge7406:                                         ; preds = %edge7404
  %t4778.not = icmp eq i64 %v64795, 0
  br i1 %t4778.not, label %block44055, label %edge7409

edge7409:                                         ; preds = %edge7406
  %v80505 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64795)
  %t4779.not = icmp eq i64 %v80505, 0
  br i1 %t4779.not, label %block44055, label %edge7410

edge7410:                                         ; preds = %edge7409
  %t4780 = insertvalue { i64, i64 } poison, i64 %v80505, 0
  %t4781 = insertvalue { i64, i64 } %t4780, i64 3689, 1
  br label %common.ret

edge7416:                                         ; preds = %edge7404
  %t4782.not = icmp eq i64 %v80922, 0
  br i1 %t4782.not, label %block3658.preheader, label %edge7417

edge7417:                                         ; preds = %edge7416
  %t4783 = inttoptr i64 %v80922 to ptr
  %v80990 = load i64, ptr %t4783, align 1
  %v80991 = add i64 %v80990, 1
  store i64 %v80991, ptr %t4783, align 1
  br label %block3658.preheader

block3658.preheader:                              ; preds = %edge7416, %edge7417
  br label %block3658

edge7439:                                         ; preds = %block3658
  %v81641 = shl nuw nsw i64 %v81058, 2
  %v81642 = add i64 %v81641, %v80928
  %t4797 = inttoptr i64 %v81642 to ptr
  %t4798 = load i32, ptr %t4797, align 1
  %v81973 = add i64 %v81641, %v64790
  %t4799 = inttoptr i64 %v81973 to ptr
  %t4800 = load i32, ptr %t4799, align 1
  %t4801 = icmp eq i32 %t4798, %t4800
  %v81635 = add nuw nsw i64 %v81058, 1
  br i1 %t4801, label %block3658, label %edge7485

edge7440:                                         ; preds = %block3658
  br i1 %t4782.not, label %block43158, label %edge7443

edge7443:                                         ; preds = %edge7440
  %v81081 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v80922)
  %t4789.not = icmp eq i64 %v81081, 0
  br i1 %t4789.not, label %block43158, label %block43154

edge7447:                                         ; preds = %block43158
  %t4791 = inttoptr i64 %v64795 to ptr
  %v81083 = load i64, ptr %t4791, align 1
  %v81084 = add i64 %v81083, -1
  store i64 %v81084, ptr %t4791, align 1
  %t4793.not = icmp eq i64 %v81084, 0
  br i1 %t4793.not, label %edge7450, label %edge11167

edge7450:                                         ; preds = %edge7447
  %v81085 = call i64 @mech_compact_free(i64 %v0, i64 %v64795)
  %t4794.not = icmp eq i64 %v81085, 0
  br i1 %t4794.not, label %edge11167, label %block43154

edge7485:                                         ; preds = %edge7439
  br i1 %t4782.not, label %block43821, label %edge7488

edge7488:                                         ; preds = %edge7485
  %v82009 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v80922)
  %t4805.not = icmp eq i64 %v82009, 0
  br i1 %t4805.not, label %block43821, label %block43817

edge7494:                                         ; preds = %block43821
  %v82013 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64795)
  %t4807.not = icmp eq i64 %v82013, 0
  br i1 %t4807.not, label %block44055, label %block43817

edge7516:                                         ; preds = %block44055
  %v82349 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64807)
  %t4811.not = icmp eq i64 %v82349, 0
  br i1 %t4811.not, label %block44058, label %block44054

edge7522:                                         ; preds = %block44058
  %v82353 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64918)
  %t4813.not = icmp eq i64 %v82353, 0
  br i1 %t4813.not, label %block44062, label %block44054

edge7528:                                         ; preds = %block44062
  %v82357 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64930)
  %t4815.not = icmp eq i64 %v82357, 0
  br i1 %t4815.not, label %block44066, label %block44054

edge7534:                                         ; preds = %block44066
  %v82361 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64942)
  %t4817.not = icmp eq i64 %v82361, 0
  br i1 %t4817.not, label %edge7561, label %block44054

edge7561:                                         ; preds = %edge7534, %block44066
  %t4820.not = icmp eq i64 %v80922, 0
  br i1 %t4820.not, label %edge7629, label %edge7563

edge7563:                                         ; preds = %edge7561
  %t4821 = inttoptr i64 %v80922 to ptr
  %v83364 = load i64, ptr %t4821, align 1
  %v83371 = add i64 %v83364, 4
  store i64 %v83371, ptr %t4821, align 1
  br label %edge7629

edge7629:                                         ; preds = %edge7561, %edge7563
  %t6912.pre-phi = phi ptr [ %t4821, %edge7563 ], [ null, %edge7561 ]
  br label %block3784

edge7641:                                         ; preds = %edge7664
  %t4840 = insertvalue { i64, i64 } poison, i64 %v84796, 0
  %t4841 = insertvalue { i64, i64 } %t4840, i64 3828, 1
  br label %common.ret

edge7663:                                         ; preds = %block3784
  %v84848 = load i64, ptr %ss1, align 8
  %v84849 = load i64, ptr %t606, align 8
  %v84850 = load i64, ptr %t607, align 8
  %t4857.not = icmp eq i64 %v84850, 0
  br i1 %t4857.not, label %edge7701, label %edge7664

edge7664:                                         ; preds = %edge7663
  %t4858 = inttoptr i64 %v84850 to ptr
  %v84851 = load i64, ptr %t4858, align 1
  %v84792 = add i64 %v84851, 2
  store i64 %v84792, ptr %t4858, align 1
  %v84796 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v84850)
  %t4839.not = icmp eq i64 %v84796, 0
  br i1 %t4839.not, label %edge7701, label %edge7641

edge7681:                                         ; preds = %edge7704
  %t4869 = insertvalue { i64, i64 } poison, i64 %v85676, 0
  %t4870 = insertvalue { i64, i64 } %t4869, i64 3841, 1
  br label %common.ret

edge7701:                                         ; preds = %edge7664, %edge7663
  %v85724 = call i64 @mech_compact_allocate(i64 %v0, i64 %v218975, i64 4, i64 4, i64 1, i64 %v3194)
  %t4883.not = icmp eq i64 %v85724, 0
  br i1 %t4883.not, label %edge7703, label %block44529

edge7703:                                         ; preds = %edge7701
  %v85727 = load i64, ptr %ss1, align 8
  %v85728 = load i64, ptr %t606, align 8
  %v85729 = load i64, ptr %t607, align 8
  %t4886.not = icmp eq i64 %v85729, 0
  br i1 %t4886.not, label %block3810, label %edge7704

edge7704:                                         ; preds = %edge7703
  %t4887 = inttoptr i64 %v85729 to ptr
  %v85730 = load i64, ptr %t4887, align 1
  %v85672 = add i64 %v85730, 2
  store i64 %v85672, ptr %t4887, align 1
  %v85676 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v85729)
  %t4868.not = icmp eq i64 %v85676, 0
  br i1 %t4868.not, label %block3810, label %edge7681

edge7719:                                         ; preds = %block3810
  %t4895.not = icmp eq i64 %v218958, 0
  br i1 %t4895.not, label %edge8401, label %edge7723

edge7723:                                         ; preds = %edge7719
  %v86164 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v218958)
  %t4896.not = icmp eq i64 %v86164, 0
  br i1 %t4896.not, label %edge8401, label %edge7724

edge7724:                                         ; preds = %edge7723
  %t4897 = insertvalue { i64, i64 } poison, i64 %v86164, 0
  %t4898 = insertvalue { i64, i64 } %t4897, i64 3862, 1
  br label %common.ret

edge7755:                                         ; preds = %block3829
  %v87056 = add nuw nsw i64 %v86753, %spec.select2
  %t4902 = icmp ult i64 %v87056, %v218957
  br i1 %t4902, label %edge7784, label %edge7812

edge7784:                                         ; preds = %edge7755
  %v87378 = shl nuw nsw i64 %v87056, 2
  %v87379 = add i64 %v87378, %v218956
  %t4905 = inttoptr i64 %v87379 to ptr
  %t4906 = load i32, ptr %t4905, align 1
  %v87380 = zext i32 %t4906 to i64
  %t4907.not = icmp eq i32 %t4906, 0
  %v87454 = shl nuw nsw i64 %v87380, 10
  %v87459 = or i64 %v87454, %v86753
  br i1 %t4907.not, label %edge7812, label %edge7795

edge7795:                                         ; preds = %edge7784
  %t4911 = icmp ult i64 %v86768, %v218975
  br i1 %t4911, label %edge7797, label %block45728

edge7797:                                         ; preds = %edge7795
  %v87766 = shl nuw nsw i64 %v86768, 2
  %v87767 = add i64 %v87766, %v86756
  %t4913 = inttoptr i64 %v87767 to ptr
  %t4914 = load i32, ptr %t4913, align 1
  %44 = trunc i64 %v87459 to i32
  %t4915 = icmp eq i32 %t4914, %44
  br i1 %t4915, label %block45731, label %edge7801

edge7801:                                         ; preds = %edge7797
  %t4917 = zext i64 %v86756 to i128
  %v220003 = or disjoint i128 %t4919, %t4917
  store i128 %v220003, ptr %ss0, align 16
  %t4921 = zext i64 %v86758 to i128
  %v220005 = or disjoint i128 %t4921, 18446744073709551616
  store i128 %v220005, ptr %t1002, align 16
  %t4927 = zext i64 %v86761 to i128
  %t4928 = shl nuw i128 %t4927, 64
  %v220007 = or disjoint i128 %t4928, %t4918
  store i128 %v220007, ptr %t1007, align 16
  %v87789 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 0, i64 %v86758, i64 4, i64 4, i64 %v3194)
  %t4931.not = icmp eq i64 %v87789, 0
  br i1 %t4931.not, label %edge7803, label %block45728

edge7803:                                         ; preds = %edge7801
  %v87790 = load i64, ptr %ss1, align 8
  %v87791 = load i64, ptr %t606, align 8
  %v87792 = load i64, ptr %t607, align 8
  %v87793 = add i64 %v87790, %v87766
  %t4934 = inttoptr i64 %v87793 to ptr
  store i32 %44, ptr %t4934, align 1
  br label %block45731

edge7806:                                         ; preds = %block45731
  %t4936 = inttoptr i64 %v87777 to ptr
  %v87795 = load i64, ptr %t4936, align 1
  %v87796 = add i64 %v87795, 1
  store i64 %v87796, ptr %t4936, align 1
  br label %edge7819.thread

edge7812:                                         ; preds = %block3829, %edge7755, %edge7784
  %t4940.not = icmp eq i64 %v86761, 0
  br i1 %t4940.not, label %edge7819, label %edge7813

edge7813:                                         ; preds = %edge7812
  %t4941 = inttoptr i64 %v86761 to ptr
  %v87814 = load i64, ptr %t4941, align 1
  %v87815 = add i64 %v87814, 1
  store i64 %v87815, ptr %t4941, align 1
  br label %edge7819

edge7819:                                         ; preds = %edge7812, %edge7813
  br i1 %t4899, label %edge7828, label %edge7829

edge7819.thread:                                  ; preds = %edge7806, %block45731
  %v88097 = add nuw nsw i64 %v86768, 1
  br label %edge7828

edge7828:                                         ; preds = %edge7819.thread, %edge7819
  %v88407801 = phi i64 [ %v88097, %edge7819.thread ], [ %v86768, %edge7819 ]
  %v88131791800 = phi i64 [ %v87777, %edge7819.thread ], [ %v86761, %edge7819 ]
  %v88402792799 = phi i64 [ %v87772, %edge7819.thread ], [ %v86756, %edge7819 ]
  %v88404793798 = phi i64 [ %v87774, %edge7819.thread ], [ %v86758, %edge7819 ]
  %t4958.not = icmp eq i64 %v86761, 0
  br i1 %t4958.not, label %edge7853, label %edge7846

edge7829:                                         ; preds = %edge7819
  %t4951.not = icmp eq i64 %v218958, 0
  br i1 %t4951.not, label %block45971, label %edge7832

edge7832:                                         ; preds = %edge7829
  %v88121 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v218958)
  %t4952.not = icmp eq i64 %v88121, 0
  br i1 %t4952.not, label %block45971, label %block45966

edge7836:                                         ; preds = %block45971
  %t4954 = inttoptr i64 %v86761 to ptr
  %v88123 = load i64, ptr %t4954, align 1
  %v88124 = add i64 %v88123, -1
  store i64 %v88124, ptr %t4954, align 1
  %t4956.not = icmp eq i64 %v88124, 0
  br i1 %t4956.not, label %edge7839, label %edge7873

edge7839:                                         ; preds = %edge7836
  %v88125 = call i64 @mech_compact_free(i64 %v0, i64 %v86761)
  %t4957.not = icmp eq i64 %v88125, 0
  br i1 %t4957.not, label %edge7873, label %block45966

edge7846:                                         ; preds = %edge7828
  %v88129 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v86761)
  %t4959.not = icmp eq i64 %v88129, 0
  br i1 %t4959.not, label %edge7853, label %block45966

edge7853:                                         ; preds = %edge7828, %edge7846
  %v88397 = add nuw nsw i64 %v86753, 1
  br label %block3829

edge7873:                                         ; preds = %block45971, %edge7836, %edge7839
  %t4963 = icmp ult i64 %v86768, 2
  br i1 %t4963, label %edge8363, label %edge7883

edge7883:                                         ; preds = %edge7873
  %v88776157 = lshr i64 %v86768, 1
  %v220027 = or disjoint i128 %t4919, 1
  br label %block3920

edge7927:                                         ; preds = %edge8035, %block47817, %edge8027
  %v90822 = phi i64 [ %v90758, %block47817 ], [ %v90758, %edge8027 ], [ %v88818, %edge8035 ]
  %v91101 = phi i64 [ %v90753, %block47817 ], [ %v90753, %edge8027 ], [ %v88813, %edge8035 ]
  %v91103 = phi i64 [ %v90755, %block47817 ], [ %v90755, %edge8027 ], [ %v88815, %edge8035 ]
  %v93549 = phi i64 [ %v93550, %block47817 ], [ %v93550, %edge8027 ], [ %v88818, %edge8035 ]
  br i1 %v88854158, label %edge8053, label %edge8040

edge7927.thread:                                  ; preds = %edge8034
  br i1 %v88854158, label %edge8053, label %edge8361

edge7932:                                         ; preds = %block3920
  %t4984 = icmp ult i64 %v89139, %v218975
  br i1 %t4984, label %edge7988, label %block47793

edge7988:                                         ; preds = %edge7932
  %v90310 = shl i64 %v88828, 2
  %v90313 = add i64 %v88813, -4
  %v90314 = add i64 %v90313, %v90310
  %t4987 = inttoptr i64 %v90314 to ptr
  %t4988 = load i32, ptr %t4987, align 1
  %t4993803 = inttoptr i64 %v88813 to ptr
  %t4994804 = load i32, ptr %t4993803, align 1
  %v90697 = shl nuw nsw i64 %v89139, 2
  %v90698 = add i64 %v90697, %v88813
  %t4997 = inttoptr i64 %v90698 to ptr
  %t4998 = load i32, ptr %t4997, align 1
  %t4999 = icmp eq i32 %t4998, %t4994804
  br i1 %t4999, label %block47796, label %edge7991

edge7991:                                         ; preds = %edge7988
  %t5001.not = icmp eq i64 %v88818, 0
  br i1 %t5001.not, label %block47798, label %edge7992

edge7992:                                         ; preds = %edge7991
  %t5002 = inttoptr i64 %v88818 to ptr
  %v90715 = load i64, ptr %t5002, align 1
  %t5003 = icmp eq i64 %v90715, 1
  br i1 %t5003, label %block47797, label %block47798

edge7997:                                         ; preds = %block47798
  %v90725 = load i64, ptr %ss1, align 8
  %v90726 = load i64, ptr %t606, align 8
  %v90727 = load i64, ptr %t607, align 8
  %.pre21767 = add i64 %v90725, %v90697
  %.pre21768 = inttoptr i64 %.pre21767 to ptr
  br label %block47797

edge8000:                                         ; preds = %block47796
  %t5027 = inttoptr i64 %v90708 to ptr
  %v90732 = load i64, ptr %t5027, align 1
  %v90733 = add i64 %v90732, 1
  store i64 %v90733, ptr %t5027, align 1
  br label %block47802

edge8007:                                         ; preds = %block47802
  %t5037.not = icmp eq i64 %v90734, 0
  br i1 %t5037.not, label %block47810, label %edge8010

edge8010:                                         ; preds = %edge8007
  %v90762 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v90734)
  %t5038.not = icmp eq i64 %v90762, 0
  br i1 %t5038.not, label %block47810, label %block47804

edge8014:                                         ; preds = %block47810
  %t5040 = inttoptr i64 %v90708 to ptr
  %v90769 = load i64, ptr %t5040, align 1
  %t5041 = icmp eq i64 %v90769, 1
  br i1 %t5041, label %block47812, label %block47813

edge8019:                                         ; preds = %block47813
  %v90779 = load i64, ptr %ss1, align 8
  %v90780 = load i64, ptr %t606, align 8
  %v90781 = load i64, ptr %t607, align 8
  %.pre21766 = inttoptr i64 %v90779 to ptr
  br label %block47812

edge8022:                                         ; preds = %block47807
  %t5065 = inttoptr i64 %v90758 to ptr
  %v90786 = load i64, ptr %t5065, align 1
  %v90787 = add i64 %v90786, 1
  store i64 %v90787, ptr %t5065, align 1
  br label %block47817

edge8027:                                         ; preds = %block47817
  %v90792 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v90708)
  %t5069.not = icmp eq i64 %v90792, 0
  br i1 %t5069.not, label %edge7927, label %block47804

edge8034:                                         ; preds = %block3920
  %t5072.not = icmp eq i64 %v88818, 0
  br i1 %t5072.not, label %edge7927.thread, label %edge8035

edge8035:                                         ; preds = %edge8034
  %t5073 = inttoptr i64 %v88818 to ptr
  %v90810 = load i64, ptr %t5073, align 1
  %v90811 = add i64 %v90810, 1
  store i64 %v90811, ptr %t5073, align 1
  br label %edge7927

edge8040:                                         ; preds = %edge7927
  %t5077.not = icmp eq i64 %v90822, 0
  br i1 %t5077.not, label %edge8309, label %edge8043

edge8043:                                         ; preds = %edge8040
  %v90820 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v90822)
  %t5078.not = icmp eq i64 %v90820, 0
  br i1 %t5078.not, label %edge8309, label %edge8044

edge8044:                                         ; preds = %edge8043
  %t5079 = insertvalue { i64, i64 } poison, i64 %v90820, 0
  %t5080 = insertvalue { i64, i64 } %t5079, i64 4062, 1
  br label %common.ret

edge8048:                                         ; preds = %block50202, %edge8293, %edge8296, %edge8088
  %v94416 = phi i64 [ %v94323, %block50202 ], [ %v91460, %edge8088 ], [ %v94323, %edge8296 ], [ %v94323, %edge8293 ]
  %v94417 = phi i64 [ %v94419, %block50202 ], [ %v93548, %edge8088 ], [ %v94419, %edge8296 ], [ %v94419, %edge8293 ]
  %v94683 = phi i64 [ %v94318, %block50202 ], [ %v91485, %edge8088 ], [ %v94318, %edge8296 ], [ %v94318, %edge8293 ]
  %v94685 = phi i64 [ %v94320, %block50202 ], [ %v91480, %edge8088 ], [ %v94320, %edge8296 ], [ %v94320, %edge8293 ]
  br i1 %v88854158, label %edge8314, label %edge8315

edge8053:                                         ; preds = %edge7927.thread, %edge7927
  %v93549815 = phi i64 [ 0, %edge7927.thread ], [ %v93549, %edge7927 ]
  %v91103814 = phi i64 [ %v88815, %edge7927.thread ], [ %v91103, %edge7927 ]
  %v91101813 = phi i64 [ %v88813, %edge7927.thread ], [ %v91101, %edge7927 ]
  %v90822812 = phi i64 [ 0, %edge7927.thread ], [ %v90822, %edge7927 ]
  %v91098 = add nsw i64 %v218986, -1
  %t5085 = icmp ult i64 %v91098, %v218975
  br i1 %t5085, label %edge8071, label %edge8083

edge8071:                                         ; preds = %edge8053
  %v91417 = shl i64 %v218986, 2
  %v91420 = add i64 %v91417, -4
  %v91421 = add i64 %v91420, %v91101813
  %t5088 = inttoptr i64 %v91421 to ptr
  %t5089 = load i32, ptr %t5088, align 1
  %v91422 = zext i32 %t5089 to i64
  br label %edge8083

edge8083:                                         ; preds = %edge8053, %edge8071
  %v91442 = phi i64 [ %v91422, %edge8071 ], [ 0, %edge8053 ]
  %45 = trunc nuw i64 %v91442 to i32
  br label %block4041

edge8084:                                         ; preds = %block4041
  %t5091 = inttoptr i64 %v91460 to ptr
  %v91461 = load i64, ptr %t5091, align 1
  %t5092 = icmp ne i64 %v91461, 1
  %t5094.not = icmp ugt i64 %v218975, %v91480
  %or.cond315 = select i1 %t5092, i1 true, i1 %t5094.not
  br i1 %or.cond315, label %edge8105, label %edge8088

edge8088:                                         ; preds = %edge8084
  %t5097 = zext i64 %spec.select329 to i128
  %t5098 = shl nuw i128 %t5097, 64
  store i128 %t5098, ptr %ss2, align 16
  %t5100 = zext i64 %v91483 to i128
  %t5101 = zext nneg i64 %v91442 to i128
  %t5102 = shl nuw nsw i128 %t5101, 64
  %v220023 = or disjoint i128 %t5102, %t5100
  store i128 %v220023, ptr %t926, align 16
  %t5105 = zext i64 %v91485 to i128
  %v220025 = or disjoint i128 %t4919, %t5105
  store i128 %v220025, ptr %t931, align 16
  store i128 %v220027, ptr %t936, align 16
  %t5115 = zext i64 %v91460 to i128
  %t5116 = zext i64 %v91480 to i128
  %t5117 = shl nuw i128 %t5116, 64
  %v220029 = or disjoint i128 %t5117, %t5115
  store i128 %v220029, ptr %t941, align 16
  %t5120 = call fastcc { i64, i64 } @mech_compact_numeric_4090(i64 %v219576, i64 %v8481)
  %v91466 = extractvalue { i64, i64 } %t5120, 0
  %t5121.not = icmp eq i64 %v91466, 0
  br i1 %t5121.not, label %edge8048, label %edge8090

edge8090:                                         ; preds = %edge8088
  %v91467 = extractvalue { i64, i64 } %t5120, 1
  %t5122 = icmp eq i64 %v91466, 103169
  %v91470 = select i1 %t5122, i64 103425, i64 %v91466
  %t5124 = insertvalue { i64, i64 } poison, i64 %v91470, 0
  %t5125 = insertvalue { i64, i64 } %t5124, i64 %v91467, 1
  br label %common.ret

edge8105:                                         ; preds = %edge8084, %block4041
  %t5130 = icmp sgt i64 %v91483, -1
  br i1 %t5130, label %edge8106, label %common.ret

edge8106:                                         ; preds = %edge8105
  %v91529 = shl nuw i64 %v91483, 1
  %t5134 = icmp ugt i64 %v91529, %spec.select329
  br i1 %t5134, label %block4170, label %edge8114

edge8114:                                         ; preds = %edge8106
  %t5137 = icmp ult i64 %v91529, %spec.select329
  %t5138 = icmp ult i64 %v91529, %v218975
  br i1 %t5138, label %edge8133, label %edge8144

edge8133:                                         ; preds = %edge8114
  %v92163 = shl nuw i64 %v91483, 3
  %v92164 = add i64 %v91485, %v92163
  %t5141 = inttoptr i64 %v92164 to ptr
  %t5142 = load i32, ptr %t5141, align 1
  %v92165 = zext i32 %t5142 to i64
  br label %edge8144

edge8144:                                         ; preds = %edge8133, %edge8114
  %v92186 = phi i64 [ %v92165, %edge8133 ], [ 0, %edge8114 ]
  %v92200 = add i64 %v91529, -1
  %t5147 = icmp ult i64 %v92200, %v218975
  br i1 %t5147, label %edge8162, label %edge8170

edge8162:                                         ; preds = %edge8144
  %v92514 = shl i64 %v91483, 3
  %v92517 = add i64 %v92514, -4
  %v92518 = add i64 %v92517, %v91485
  %t5150 = inttoptr i64 %v92518 to ptr
  %t5151 = load i32, ptr %t5150, align 1
  %v92519 = zext i32 %t5151 to i64
  br label %edge8170

edge8170:                                         ; preds = %edge8144, %edge8162
  %v92539 = phi i64 [ %v92519, %edge8162 ], [ 0, %edge8144 ]
  %t5152 = icmp samesign ugt i64 %v92186, %v92539
  %v92550163 = and i1 %t5137, %t5152
  %v92827 = zext i1 %v92550163 to i64
  %spec.select316 = or disjoint i64 %v91529, %v92827
  %v92857 = add i64 %spec.select316, -1
  %t5163 = icmp ult i64 %v92857, %v218975
  br i1 %t5163, label %edge8201, label %block4170

edge8201:                                         ; preds = %edge8170
  %v93171 = shl i64 %spec.select316, 2
  %v93174 = add i64 %v91485, -4
  %v93175 = add i64 %v93174, %v93171
  %t5166 = inttoptr i64 %v93175 to ptr
  %t5167 = load i32, ptr %t5166, align 1
  %t5168 = icmp ugt i32 %t5167, %45
  br i1 %t5168, label %edge8215, label %block4170

edge8215:                                         ; preds = %edge8201
  %v93498 = add nsw i64 %v91483, -1
  %t5175 = icmp ult i64 %v93498, %v218975
  br i1 %t5175, label %edge8219, label %block49703

edge8219:                                         ; preds = %edge8215
  %v93501 = shl nuw nsw i64 %v93498, 2
  %v93502 = add i64 %v93501, %v91485
  %t5177 = inttoptr i64 %v93502 to ptr
  %t5178 = load i32, ptr %t5177, align 1
  %t5179 = icmp eq i32 %t5178, %t5167
  br i1 %t5179, label %block49706, label %edge8222

edge8222:                                         ; preds = %edge8219
  %t5181.not = icmp eq i64 %v93548, 0
  br i1 %t5181.not, label %block49709, label %edge8225

edge8225:                                         ; preds = %edge8222
  %v93516 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v93548)
  %t5182.not = icmp eq i64 %v93516, 0
  br i1 %t5182.not, label %block49709, label %block49703

edge8229:                                         ; preds = %block49709
  %t5184 = inttoptr i64 %v91460 to ptr
  %v93523 = load i64, ptr %t5184, align 1
  %t5185 = icmp eq i64 %v93523, 1
  br i1 %t5185, label %block49711, label %block49712

edge8234:                                         ; preds = %block49712
  %v93533 = load i64, ptr %ss1, align 8
  %v93534 = load i64, ptr %t606, align 8
  %v93535 = load i64, ptr %t607, align 8
  %.pre21764 = add i64 %v93533, %v93501
  %.pre21765 = inttoptr i64 %.pre21764 to ptr
  br label %block49711

edge8237:                                         ; preds = %block49706
  %t5209 = inttoptr i64 %v93512 to ptr
  %v93540 = load i64, ptr %t5209, align 1
  %v93541 = add i64 %v93540, 1
  store i64 %v93541, ptr %t5209, align 1
  br label %block49716

edge8242:                                         ; preds = %block49716
  %v93546 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v91460)
  %t5213.not = icmp eq i64 %v93546, 0
  br i1 %t5213.not, label %block4041.backedge, label %block49703

edge8268:                                         ; preds = %block4170
  %v94309 = add nsw i64 %v91483, -1
  %t5220 = icmp ult i64 %v94309, %v218975
  br i1 %t5220, label %edge8272, label %block50189

edge8272:                                         ; preds = %edge8268
  %v94312 = shl nuw nsw i64 %v94309, 2
  %v94313 = add i64 %v94312, %v91485
  %t5222 = inttoptr i64 %v94313 to ptr
  %t5223 = load i32, ptr %t5222, align 1
  %v94314 = zext i32 %t5223 to i64
  %t5224 = icmp eq i64 %v91442, %v94314
  br i1 %t5224, label %block50192, label %edge8275

edge8275:                                         ; preds = %edge8272
  %t5226.not = icmp eq i64 %v93548, 0
  br i1 %t5226.not, label %block50195, label %edge8278

edge8278:                                         ; preds = %edge8275
  %v94327 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v93548)
  %t5227.not = icmp eq i64 %v94327, 0
  br i1 %t5227.not, label %block50195, label %block50189

edge8282:                                         ; preds = %block50195
  %t5229 = inttoptr i64 %v91460 to ptr
  %v94334 = load i64, ptr %t5229, align 1
  %t5230 = icmp eq i64 %v94334, 1
  br i1 %t5230, label %block50197, label %block50198

edge8287:                                         ; preds = %block50198
  %v94344 = load i64, ptr %ss1, align 8
  %v94345 = load i64, ptr %t606, align 8
  %v94346 = load i64, ptr %t607, align 8
  %.pre21762 = add i64 %v94344, %v94312
  %.pre21763 = inttoptr i64 %.pre21762 to ptr
  br label %block50197

edge8290:                                         ; preds = %block50192
  %t5254 = inttoptr i64 %v94323 to ptr
  %v94351 = load i64, ptr %t5254, align 1
  %v94352 = add i64 %v94351, 1
  store i64 %v94352, ptr %t5254, align 1
  br label %block50202

edge8293:                                         ; preds = %block50202
  %t5258 = inttoptr i64 %v91460 to ptr
  %v94355 = load i64, ptr %t5258, align 1
  %v94356 = add i64 %v94355, -1
  store i64 %v94356, ptr %t5258, align 1
  %t5260.not = icmp eq i64 %v94356, 0
  br i1 %t5260.not, label %edge8296, label %edge8048

edge8296:                                         ; preds = %edge8293
  %v94357 = call i64 @mech_compact_free(i64 %v0, i64 %v91460)
  %t5261.not = icmp eq i64 %v94357, 0
  br i1 %t5261.not, label %edge8048, label %block50189

edge8309:                                         ; preds = %edge8043, %edge8040
  %t5264.not = icmp eq i64 %v93549, 0
  br i1 %t5264.not, label %edge8361, label %edge8315.thread1278

edge8315.thread1278:                              ; preds = %edge8309
  %t5265 = inttoptr i64 %v93549 to ptr
  %v94399 = load i64, ptr %t5265, align 1
  %v94400 = add i64 %v94399, 1
  store i64 %v94400, ptr %t5265, align 1
  br label %edge8316

edge8314:                                         ; preds = %edge8048
  %t5274.not = icmp eq i64 %v94417, 0
  br i1 %t5274.not, label %block3920.backedge, label %edge8324

block3920.backedge:                               ; preds = %edge8314, %edge8324, %edge8327
  br label %block3920

edge8315:                                         ; preds = %edge8048
  %t5269.not = icmp eq i64 %v94416, 0
  br i1 %t5269.not, label %edge8361, label %edge8315.edge8316_crit_edge

edge8315.edge8316_crit_edge:                      ; preds = %edge8315
  %t5270.phi.trans.insert = inttoptr i64 %v94416 to ptr
  %v94408.pre = load i64, ptr %t5270.phi.trans.insert, align 1
  %46 = add i64 %v94408.pre, -1
  br label %edge8316

edge8316:                                         ; preds = %edge8315.edge8316_crit_edge, %edge8315.thread1278
  %t5270.pre-phi = phi ptr [ %t5270.phi.trans.insert, %edge8315.edge8316_crit_edge ], [ %t5265, %edge8315.thread1278 ]
  %v94408 = phi i64 [ %46, %edge8315.edge8316_crit_edge ], [ %v94399, %edge8315.thread1278 ]
  %v944168251283 = phi i64 [ %v94416, %edge8315.edge8316_crit_edge ], [ %v93549, %edge8315.thread1278 ]
  %v944178261282 = phi i64 [ %v94417, %edge8315.edge8316_crit_edge ], [ %v93549, %edge8315.thread1278 ]
  store i64 %v94408, ptr %t5270.pre-phi, align 1
  %t5272.not = icmp eq i64 %v94408, 0
  br i1 %t5272.not, label %edge8319, label %edge8361

edge8319:                                         ; preds = %edge8316
  %v94410 = call i64 @mech_compact_free(i64 %v0, i64 %v944168251283)
  %t5273.not = icmp eq i64 %v94410, 0
  br i1 %t5273.not, label %edge8361, label %block50232

edge8324:                                         ; preds = %edge8314
  %t5275 = inttoptr i64 %v94417 to ptr
  %v94412 = load i64, ptr %t5275, align 1
  %v94413 = add i64 %v94412, -1
  store i64 %v94413, ptr %t5275, align 1
  %t5277.not = icmp eq i64 %v94413, 0
  br i1 %t5277.not, label %edge8327, label %block3920.backedge

edge8327:                                         ; preds = %edge8324
  %v94414 = call i64 @mech_compact_free(i64 %v0, i64 %v94417)
  %t5278.not = icmp eq i64 %v94414, 0
  br i1 %t5278.not, label %block3920.backedge, label %block50232

edge8361:                                         ; preds = %edge7927.thread, %edge8319, %edge8316, %edge8315, %edge8309
  %v94792 = phi i64 [ %v94417, %edge8315 ], [ %v944178261282, %edge8319 ], [ %v944178261282, %edge8316 ], [ 0, %edge8309 ], [ 0, %edge7927.thread ]
  %v96774 = add i64 %v86768, -1
  br label %block4256

edge8362:                                         ; preds = %block51425.thread, %edge9886, %edge10121, %edge10118, %block61604, %block51428, %edge8491, %edge8494
  %v115047 = phi i64 [ %v96524, %block51428 ], [ %v96524, %edge8494 ], [ %v96524, %edge8491 ], [ %v111877, %block61604 ], [ %v111877, %edge10118 ], [ %v111877, %edge10121 ], [ %v111877, %edge9886 ], [ %v96524, %block51425.thread ]
  %v115049 = phi i64 [ %v96526, %block51428 ], [ %v96526, %edge8494 ], [ %v96526, %edge8491 ], [ %v111872, %block61604 ], [ %v111872, %edge10118 ], [ %v111872, %edge10121 ], [ %v111872, %edge9886 ], [ %v96526, %block51425.thread ]
  %v115052 = phi i64 [ %v96529, %block51428 ], [ %v96529, %edge8494 ], [ %v96529, %edge8491 ], [ %v111836, %block61604 ], [ %v111836, %edge10118 ], [ %v111836, %edge10121 ], [ %v111836, %edge9886 ], [ %v96529, %block51425.thread ]
  %t6530 = icmp eq i64 %v218955, 0
  br i1 %t6530, label %edge10173, label %edge10180

edge8363:                                         ; preds = %edge7873
  %t5284 = icmp eq i64 %v86768, 1
  br i1 %t5284, label %edge8382, label %edge8401

edge8382:                                         ; preds = %edge8363
  %t5291 = inttoptr i64 %v86756 to ptr
  %t5292 = load i32, ptr %t5291, align 1
  %47 = and i32 %t5292, 1023
  %narrow195 = call i32 @llvm.umax.i32(i32 %47, i32 1)
  %v96509 = zext nneg i32 %narrow195 to i64
  br label %edge8401

edge8401:                                         ; preds = %edge8382, %edge7719, %edge7723, %edge8363
  %v218995 = phi i64 [ %v86756, %edge8382 ], [ %v85727, %edge7719 ], [ %v85727, %edge7723 ], [ %v86756, %edge8363 ]
  %v218997 = phi i64 [ %v86758, %edge8382 ], [ %v85728, %edge7719 ], [ %v85728, %edge7723 ], [ %v86758, %edge8363 ]
  %v218999 = phi i64 [ %v86761, %edge8382 ], [ %v85729, %edge7719 ], [ %v85729, %edge7723 ], [ %v86761, %edge8363 ]
  %v219000 = phi i64 [ %v96509, %edge8382 ], [ 1, %edge7719 ], [ 1, %edge7723 ], [ 1, %edge8363 ]
  %t5298 = inttoptr i64 %v218995 to ptr
  %t5299 = load i32, ptr %t5298, align 1
  %t5300 = icmp eq i32 %t5299, 65536
  br i1 %t5300, label %block51374, label %edge8404

edge8404:                                         ; preds = %edge8401
  %t5302.not = icmp eq i64 %v218999, 0
  br i1 %t5302.not, label %block51376, label %edge8405

edge8405:                                         ; preds = %edge8404
  %t5303 = inttoptr i64 %v218999 to ptr
  %v96206 = load i64, ptr %t5303, align 1
  %t5304 = icmp eq i64 %v96206, 1
  br i1 %t5304, label %block51375, label %block51376

edge8410:                                         ; preds = %block51376
  %v96216 = load i64, ptr %ss1, align 8
  %v96217 = load i64, ptr %t606, align 8
  %v96218 = load i64, ptr %t607, align 8
  %.pre21739 = inttoptr i64 %v96216 to ptr
  br label %block51375

edge8413:                                         ; preds = %block51374
  %t5328 = inttoptr i64 %v96199 to ptr
  %v96223 = load i64, ptr %t5328, align 1
  %v96224 = add i64 %v96223, 1
  store i64 %v96224, ptr %t5328, align 1
  br label %block51380

edge8418:                                         ; preds = %block51380
  %v96229 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v218999)
  %t5332.not = icmp eq i64 %v96229, 0
  br i1 %t5332.not, label %edge8424, label %block51371

edge8424:                                         ; preds = %edge8418, %block51380
  %t5337 = inttoptr i64 %v84848 to ptr
  %t5338 = load i8, ptr %t5337, align 1
  %t5339 = icmp eq i8 %t5338, 1
  br i1 %t5339, label %edge8445, label %edge8427

edge8427:                                         ; preds = %edge8424
  br i1 %t4857.not, label %block51391, label %edge8428

edge8428:                                         ; preds = %edge8427
  %t5342 = inttoptr i64 %v84850 to ptr
  %v96381 = load i64, ptr %t5342, align 1
  %t5343 = icmp eq i64 %v96381, 1
  br i1 %t5343, label %block51389.thread398, label %block51391

block51389.thread398:                             ; preds = %edge8428
  store i8 1, ptr %t5337, align 1
  br label %edge8445

edge8436:                                         ; preds = %block51389
  %t5367 = inttoptr i64 %v96393 to ptr
  %v96398 = load i64, ptr %t5367, align 1
  %v96399 = add i64 %v96398, 1
  store i64 %v96399, ptr %t5367, align 1
  br label %block51395

edge8441:                                         ; preds = %block51395
  %v96404 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v84850)
  %t5371.not = icmp eq i64 %v96404, 0
  br i1 %t5371.not, label %edge8445, label %block51386

edge8445:                                         ; preds = %edge8424, %block51389.thread398, %edge8441, %block51395
  %v96369392415 = phi i64 [ %v96391, %block51395 ], [ %v96391, %edge8441 ], [ %v84848, %block51389.thread398 ], [ %v84848, %edge8424 ]
  %v96371393414 = phi i64 [ %v96392, %block51395 ], [ %v96392, %edge8441 ], [ %v84849, %block51389.thread398 ], [ %v84849, %edge8424 ]
  %v96374394413 = phi i64 [ %v96393, %block51395 ], [ %v96393, %edge8441 ], [ %v84850, %block51389.thread398 ], [ %v84850, %edge8424 ]
  %t5366.not198397412 = phi i1 [ %t5366.not198, %block51395 ], [ %t5366.not198, %edge8441 ], [ false, %block51389.thread398 ], [ %t4857.not, %edge8424 ]
  %t5374 = icmp samesign ult i64 %v219000, %v218975
  br i1 %t5374, label %edge8447, label %block51401

edge8447:                                         ; preds = %edge8445
  %v96518 = shl nuw nsw i64 %v219000, 2
  %v96519 = add i64 %v96194, %v96518
  %t5376 = inttoptr i64 %v96519 to ptr
  %t5377 = load i32, ptr %t5376, align 1
  %t5378 = icmp eq i32 %t5377, 65537
  br i1 %t5378, label %block51404, label %edge8450

edge8450:                                         ; preds = %edge8447
  br i1 %t5327.not196, label %block51406, label %edge8451

edge8451:                                         ; preds = %edge8450
  %t5381 = inttoptr i64 %v96199 to ptr
  %v96536 = load i64, ptr %t5381, align 1
  %t5382 = icmp eq i64 %v96536, 1
  br i1 %t5382, label %block51405, label %block51406

edge8456:                                         ; preds = %block51406
  %v96546 = load i64, ptr %ss1, align 8
  %v96547 = load i64, ptr %t606, align 8
  %v96548 = load i64, ptr %t607, align 8
  %.pre21737 = add i64 %v96546, %v96518
  %.pre21738 = inttoptr i64 %.pre21737 to ptr
  br label %block51405

edge8459:                                         ; preds = %block51404
  %t5406 = inttoptr i64 %v96529 to ptr
  %v96553 = load i64, ptr %t5406, align 1
  %v96554 = add i64 %v96553, 1
  store i64 %v96554, ptr %t5406, align 1
  br label %block51410

edge8464:                                         ; preds = %block51410
  %v96559 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v96199)
  %t5410.not = icmp eq i64 %v96559, 0
  br i1 %t5410.not, label %edge8470, label %block51401

edge8470:                                         ; preds = %block51410, %edge8464
  %v96694 = add i64 %v96369392415, %v219000
  %t5415 = inttoptr i64 %v96694 to ptr
  %t5416 = load i8, ptr %t5415, align 1
  %t5417 = icmp eq i8 %t5416, 1
  br i1 %t5417, label %block51425.thread, label %edge8473

block51425.thread:                                ; preds = %edge8470
  %t5443.not202419 = icmp eq i64 %v96374394413, 0
  br i1 %t5443.not202419, label %edge8362, label %edge8491

edge8473:                                         ; preds = %edge8470
  br i1 %t5366.not198397412, label %block51421, label %edge8474

edge8474:                                         ; preds = %edge8473
  %t5420 = inttoptr i64 %v96374394413 to ptr
  %v96711 = load i64, ptr %t5420, align 1
  %t5421 = icmp eq i64 %v96711, 1
  br i1 %t5421, label %block51419, label %block51421

edge8479:                                         ; preds = %block51421
  %v96721 = load i64, ptr %ss1, align 8
  %v96723 = load i64, ptr %t607, align 8
  %.pre21735 = add i64 %v96721, %v219000
  %.pre21736 = inttoptr i64 %.pre21735 to ptr
  br label %block51419

edge8482:                                         ; preds = %block51419
  %t5444 = inttoptr i64 %v96710 to ptr
  %v96728 = load i64, ptr %t5444, align 1
  %v96729 = add i64 %v96728, 1
  store i64 %v96729, ptr %t5444, align 1
  br label %block51425

edge8487:                                         ; preds = %block51425
  %v96734 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v96374394413)
  %t5448.not = icmp eq i64 %v96734, 0
  br i1 %t5448.not, label %block51428, label %block51416

edge8491:                                         ; preds = %block51425.thread, %block51428
  %v96704422431432 = phi i64 [ %v96374394413, %block51425.thread ], [ %v96710, %block51428 ]
  %t5450 = inttoptr i64 %v96704422431432 to ptr
  %v96736 = load i64, ptr %t5450, align 1
  %v96737 = add i64 %v96736, -1
  store i64 %v96737, ptr %t5450, align 1
  %t5452.not = icmp eq i64 %v96737, 0
  br i1 %t5452.not, label %edge8494, label %edge8362

edge8494:                                         ; preds = %edge8491
  %v96738 = call i64 @mech_compact_free(i64 %v0, i64 %v96704422431432)
  %t5453.not = icmp eq i64 %v96738, 0
  br i1 %t5453.not, label %edge8362, label %block51416

edge8510:                                         ; preds = %block4256
  %t5457 = inttoptr i64 %v96791 to ptr
  %v96792 = load i64, ptr %t5457, align 1
  %t5458 = icmp ne i64 %v96792, 1
  %t5460.not = icmp ugt i64 %v218975, %v96811
  %or.cond317 = select i1 %t5458, i1 true, i1 %t5460.not
  br i1 %or.cond317, label %edge8531.thread, label %edge8514

edge8514:                                         ; preds = %edge8510
  %t5462 = zext i64 %v96812 to i128
  %t5463 = zext i64 %v96813 to i128
  %t5464 = shl nuw i128 %t5463, 64
  %v220067 = or disjoint i128 %t5464, %t5462
  store i128 %v220067, ptr %ss2, align 16
  %t5466 = zext nneg i64 %v96814 to i128
  %t5467 = zext i64 %v96774 to i128
  %t5468 = shl nuw i128 %t5467, 64
  %v220069 = or disjoint i128 %t5468, %t5466
  store i128 %v220069, ptr %t926, align 16
  %t5471 = zext i64 %v96816 to i128
  %v220071 = or disjoint i128 %t4919, %t5471
  store i128 %v220071, ptr %t931, align 16
  store i128 %v220027, ptr %t936, align 16
  %t5481 = zext i64 %v96791 to i128
  %t5482 = zext i64 %v96811 to i128
  %t5483 = shl nuw i128 %t5482, 64
  %v220075 = or disjoint i128 %t5483, %t5481
  store i128 %v220075, ptr %t941, align 16
  %t5486 = call fastcc { i64, i64 } @mech_compact_numeric_4310(i64 %v219576, i64 %v8481)
  %v96797 = extractvalue { i64, i64 } %t5486, 0
  %t5487.not = icmp eq i64 %v96797, 0
  br i1 %t5487.not, label %edge8912, label %edge8516

edge8516:                                         ; preds = %edge8514
  %v96798 = extractvalue { i64, i64 } %t5486, 1
  %t5488 = icmp eq i64 %v96797, 103169
  %v96801 = select i1 %t5488, i64 103425, i64 %v96797
  %t5490 = insertvalue { i64, i64 } poison, i64 %v96801, 0
  %t5491 = insertvalue { i64, i64 } %t5490, i64 %v96798, 1
  br label %common.ret

edge8531:                                         ; preds = %block4256
  %t5492 = icmp ult i64 %v96814, %v96774
  br i1 %t5492, label %edge8545, label %edge8912

edge8531.thread:                                  ; preds = %edge8510
  %t5492875 = icmp ult i64 %v96814, %v96774
  br i1 %t5492875, label %edge8545, label %edge8882

edge8545:                                         ; preds = %edge8531.thread, %edge8531
  %t5495 = icmp ult i64 %v96812, %v218975
  br i1 %t5495, label %edge8562.thread, label %edge8562

edge8562.thread:                                  ; preds = %edge8545
  %v97438 = shl nuw nsw i64 %v96812, 2
  %v97439 = add i64 %v96816, %v97438
  %t5498 = inttoptr i64 %v97439 to ptr
  %t5499 = load i32, ptr %t5498, align 1
  %48 = and i32 %t5499, -1024
  %49 = zext i32 %48 to i64
  br label %edge8574

edge8562:                                         ; preds = %edge8545
  %t5500.not = icmp eq i64 %v96812, -1
  br i1 %t5500.not, label %common.ret, label %edge8574

edge8574:                                         ; preds = %edge8562.thread, %edge8562
  %v97461838 = phi i64 [ %49, %edge8562.thread ], [ 0, %edge8562 ]
  %v97597839 = add nuw i64 %v96812, 1
  %t5504 = icmp ult i64 %v97597839, %v218975
  br i1 %t5504, label %edge8582, label %edge8591

edge8582:                                         ; preds = %edge8574
  %v97911 = shl nuw nsw i64 %v97597839, 2
  %v97912 = add i64 %v96816, %v97911
  %t5507 = inttoptr i64 %v97912 to ptr
  %t5508 = load i32, ptr %t5507, align 1
  %50 = and i32 %t5508, -1024
  %51 = zext i32 %50 to i64
  br label %edge8591

edge8591:                                         ; preds = %edge8582, %edge8574
  %v97933 = phi i64 [ %51, %edge8582 ], [ 0, %edge8574 ]
  %t5509 = icmp ult i64 %v96813, %v218975
  br i1 %t5509, label %edge8617.thread, label %edge8617

edge8617.thread:                                  ; preds = %edge8591
  %v98254 = shl nuw nsw i64 %v96813, 2
  %v98255 = add i64 %v96816, %v98254
  %t5512 = inttoptr i64 %v98255 to ptr
  %t5513 = load i32, ptr %t5512, align 1
  %v98256 = zext i32 %t5513 to i64
  br label %edge8628

edge8617:                                         ; preds = %edge8591
  %t5514.not = icmp eq i64 %v96813, -1
  br i1 %t5514.not, label %common.ret, label %edge8628

edge8628:                                         ; preds = %edge8617.thread, %edge8617
  %v98276843 = phi i64 [ %v98256, %edge8617.thread ], [ 0, %edge8617 ]
  %v98285844 = add nuw i64 %v96813, 1
  %t5518 = icmp ult i64 %v98285844, %v218975
  br i1 %t5518, label %edge8645.thread, label %edge8645

edge8645.thread:                                  ; preds = %edge8628
  %v98599 = shl nuw nsw i64 %v98285844, 2
  %v98600 = add i64 %v96816, %v98599
  %t5521 = inttoptr i64 %v98600 to ptr
  %t5522 = load i32, ptr %t5521, align 1
  %v98601 = zext i32 %t5522 to i64
  %v98635847 = and i64 %v98601, 4294966272
  %52 = and i64 %v98601, 1023
  br label %edge8658

edge8645:                                         ; preds = %edge8628
  %t5526.not = icmp eq i64 %v96813, -2
  br i1 %t5526.not, label %common.ret, label %edge8658

edge8658:                                         ; preds = %edge8645.thread, %edge8645
  %v98635852 = phi i64 [ %v98635847, %edge8645.thread ], [ 0, %edge8645 ]
  %v98621850 = phi i64 [ %52, %edge8645.thread ], [ 0, %edge8645 ]
  %v98630851 = and i64 %v98276843, 4294966272
  %v98681853 = add nuw i64 %v96813, 2
  %t5523 = icmp uge i64 %v96812, %v96774
  %t5524 = icmp ne i64 %v96813, %v96814
  %t5525 = icmp samesign ugt i64 %v97933, %v98630851
  %v98665168.not172 = and i1 %t5524, %t5525
  %v98670169.not = or i1 %t5523, %v98665168.not172
  %t5530 = icmp ule i64 %v98681853, %v96814
  %t5531 = icmp ugt i64 %v96812, %v96774
  %t5532 = icmp samesign ult i64 %v98635852, %v97461838
  %v98706170 = or i1 %t5531, %t5532
  %v98711171 = and i1 %t5530, %v98706170
  %narrow = and i1 %v98670169.not, %v98711171
  %v98726 = shl i64 %v96814, 10
  br i1 %v98670169.not, label %edge8672, label %edge8754

edge8668:                                         ; preds = %edge8691, %block52882
  %v99080 = select i1 %t5565, i64 0, i64 %v96791
  br i1 %narrow, label %edge8701, label %edge8745

edge8672:                                         ; preds = %edge8658
  %v99010 = and i64 %v98276843, 1023
  %v99024 = or disjoint i64 %v99010, %v98726
  br i1 %t5509, label %edge8679, label %block52879

edge8679:                                         ; preds = %edge8672
  %v99043 = shl nuw nsw i64 %v96813, 2
  %v99044 = add i64 %v96816, %v99043
  %t5538 = inttoptr i64 %v99044 to ptr
  %t5539 = load i32, ptr %t5538, align 1
  %v99045 = zext i32 %t5539 to i64
  %t5540 = icmp eq i64 %v99024, %v99045
  br i1 %t5540, label %block52882, label %edge8682

edge8682:                                         ; preds = %edge8679
  br i1 %t5456.not, label %block52884, label %edge8683

edge8683:                                         ; preds = %edge8682
  %t5543 = inttoptr i64 %v96791 to ptr
  %v99061 = load i64, ptr %t5543, align 1
  %t5544 = icmp eq i64 %v99061, 1
  br i1 %t5544, label %block52883, label %block52884

edge8688:                                         ; preds = %block52884
  %v99071 = load i64, ptr %ss1, align 8
  %v99072 = load i64, ptr %t606, align 8
  %v99073 = load i64, ptr %t607, align 8
  %.pre21744 = add i64 %v99071, %v99043
  %.pre21745 = inttoptr i64 %.pre21744 to ptr
  br label %block52883

edge8691:                                         ; preds = %block52882
  %t5568 = inttoptr i64 %v99054 to ptr
  %v99078 = load i64, ptr %t5568, align 1
  %v99079 = add i64 %v99078, 1
  store i64 %v99079, ptr %t5568, align 1
  br label %edge8668

edge8701:                                         ; preds = %edge8668
  %v99380 = or disjoint i64 %v98621850, %v98726
  br i1 %t5518, label %edge8710, label %block53111

edge8710:                                         ; preds = %edge8701
  %v99405 = shl nuw nsw i64 %v98285844, 2
  %v99406 = add i64 %v99049, %v99405
  %t5577 = inttoptr i64 %v99406 to ptr
  %t5578 = load i32, ptr %t5577, align 1
  %v99407 = zext i32 %t5578 to i64
  %t5579 = icmp eq i64 %v99380, %v99407
  br i1 %t5579, label %edge8745, label %edge8713

edge8713:                                         ; preds = %edge8710
  %t5581.not = icmp eq i64 %v99080, 0
  br i1 %t5581.not, label %block53117, label %edge8716

edge8716:                                         ; preds = %edge8713
  %v99420 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v99080)
  %t5582.not = icmp eq i64 %v99420, 0
  br i1 %t5582.not, label %block53117, label %block53111

edge8720:                                         ; preds = %block53117
  %t5584 = inttoptr i64 %v99054 to ptr
  %v99427 = load i64, ptr %t5584, align 1
  %t5585 = icmp eq i64 %v99427, 1
  br i1 %t5585, label %block53114, label %block53120

edge8725:                                         ; preds = %block53120
  %v99437 = load i64, ptr %ss1, align 8
  %v99438 = load i64, ptr %t606, align 8
  %v99439 = load i64, ptr %t607, align 8
  %.pre21742 = add i64 %v99437, %v99405
  %.pre21743 = inttoptr i64 %.pre21742 to ptr
  br label %block53114

edge8728:                                         ; preds = %block53114
  %t5609 = inttoptr i64 %v99426 to ptr
  %v99444 = load i64, ptr %t5609, align 1
  %v99445 = add i64 %v99444, 1
  store i64 %v99445, ptr %t5609, align 1
  br label %block53124

edge8733:                                         ; preds = %block53124
  %v99450 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v99054)
  %t5613.not = icmp eq i64 %v99450, 0
  br i1 %t5613.not, label %edge8745, label %block53111

edge8745:                                         ; preds = %edge8754, %edge8733, %block53124, %edge8668, %edge8710
  %v101659868 = phi i64 [ 0, %edge8754 ], [ %v99080, %edge8668 ], [ 0, %edge8733 ], [ 0, %block53124 ], [ %v99080, %edge8710 ]
  %v101236866 = phi i64 [ %v96791, %edge8754 ], [ %v99054, %edge8668 ], [ %v99426, %edge8733 ], [ %v99426, %block53124 ], [ %v99054, %edge8710 ]
  %v101233865 = phi i64 [ %v96811, %edge8754 ], [ %v99051, %edge8668 ], [ %v99423, %edge8733 ], [ %v99423, %block53124 ], [ %v99051, %edge8710 ]
  %v101231864 = phi i64 [ %v96816, %edge8754 ], [ %v99049, %edge8668 ], [ %v99421, %edge8733 ], [ %v99421, %block53124 ], [ %v99049, %edge8710 ]
  %v101669 = phi i64 [ %v100032, %edge8754 ], [ %v97597839, %edge8668 ], [ %v96812, %edge8733 ], [ %v96812, %block53124 ], [ %v96812, %edge8710 ]
  %v98681.v98285 = select i1 %narrow, i64 %v98681853, i64 %v98285844
  %spec.select335 = select i1 %v98670169.not, i64 %v98681.v98285, i64 %v96813
  %exitcond21636.not = icmp eq i64 %v96814, %v218975
  br i1 %exitcond21636.not, label %block54675, label %edge8834

edge8754:                                         ; preds = %edge8658
  %v100032 = add nuw i64 %v96812, 2
  br label %edge8745

edge8834:                                         ; preds = %edge8745
  %v101228.v = select i1 %v98711171, i64 %v98635852, i64 %v97461838
  %v101228 = add nuw nsw i64 %v101228.v, %v98630851
  %v100919 = add nuw nsw i64 %v97933, %v97461838
  %v101587 = select i1 %v98670169.not, i64 %v101228, i64 %v100919
  %v101551 = shl nuw nsw i64 %v96814, 2
  %v101552 = add i64 %v101231864, %v101551
  %t5635 = inttoptr i64 %v101552 to ptr
  %t5636 = load i32, ptr %t5635, align 1
  %53 = and i32 %t5636, 1023
  %54 = zext nneg i32 %53 to i64
  %v101587.masked870 = and i64 %v101587, 4294967295
  %v101593871 = or i64 %v101587.masked870, %54
  %v101604872 = add nuw nsw i64 %v96814, 1
  %v101614 = zext i32 %t5636 to i64
  %t5641 = icmp eq i64 %v101593871, %v101614
  %t5689.not = icmp eq i64 %v101659868, 0
  br i1 %t5641, label %edge8880, label %edge8837

edge8837:                                         ; preds = %edge8834
  br i1 %t5689.not, label %block54681, label %edge8840

edge8840:                                         ; preds = %edge8837
  %v101627 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v101659868)
  %t5644.not = icmp eq i64 %v101627, 0
  br i1 %t5644.not, label %block54681, label %block54675

edge8844:                                         ; preds = %block54681
  %t5646 = inttoptr i64 %v101236866 to ptr
  %v101634 = load i64, ptr %t5646, align 1
  %t5647 = icmp eq i64 %v101634, 1
  br i1 %t5647, label %block54678, label %block54684

edge8849:                                         ; preds = %block54684
  %v101644 = load i64, ptr %ss1, align 8
  %v101645 = load i64, ptr %t606, align 8
  %v101646 = load i64, ptr %t607, align 8
  %.pre21740 = add i64 %v101644, %v101551
  %.pre21741 = inttoptr i64 %.pre21740 to ptr
  br label %block54678

edge8852:                                         ; preds = %block54678
  %t5671 = inttoptr i64 %v101633 to ptr
  %v101651 = load i64, ptr %t5671, align 1
  %v101652 = add i64 %v101651, 1
  store i64 %v101652, ptr %t5671, align 1
  br label %block54688

edge8857:                                         ; preds = %block54688
  %v101657 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v101236866)
  %t5675.not = icmp eq i64 %v101657, 0
  br i1 %t5675.not, label %block4256.backedge, label %block54675

edge8880:                                         ; preds = %edge8834
  br i1 %t5689.not, label %block4256.backedge, label %edge8890

edge8882:                                         ; preds = %edge8531.thread
  %t5687.not = icmp eq i64 %v96792, 0
  br i1 %t5687.not, label %edge8885, label %edge8912

edge8885:                                         ; preds = %edge8882
  %v101729 = call i64 @mech_compact_free(i64 %v0, i64 %v96791)
  %t5688.not = icmp eq i64 %v101729, 0
  br i1 %t5688.not, label %edge8912, label %block54723

edge8890:                                         ; preds = %edge8880
  %t5690 = inttoptr i64 %v101659868 to ptr
  %v101731 = load i64, ptr %t5690, align 1
  %v101732 = add i64 %v101731, -1
  store i64 %v101732, ptr %t5690, align 1
  %t5692.not = icmp eq i64 %v101732, 0
  br i1 %t5692.not, label %edge8893, label %block4256.backedge

edge8893:                                         ; preds = %edge8890
  %v101733 = call i64 @mech_compact_free(i64 %v0, i64 %v101659868)
  %t5693.not = icmp eq i64 %v101733, 0
  br i1 %t5693.not, label %block4256.backedge, label %block54723

edge8912:                                         ; preds = %edge8531, %edge8882, %edge8885, %edge8514
  %v102044 = add i64 %v86768, -2
  %v102058 = add nuw nsw i64 %v218977, 2
  %v102850 = call i64 @mech_compact_allocate(i64 %v0, i64 %v102058, i64 8, i64 8, i64 1, i64 %v3194)
  %t5721.not = icmp eq i64 %v102850, 0
  br i1 %t5721.not, label %edge8950, label %block54988

edge8928:                                         ; preds = %edge8951
  %t5707 = insertvalue { i64, i64 } poison, i64 %v102803, 0
  %t5708 = insertvalue { i64, i64 } %t5707, i64 4567, 1
  br label %common.ret

edge8950:                                         ; preds = %edge8912
  %v102853 = load i64, ptr %ss1, align 8
  %v102854 = load i64, ptr %t606, align 8
  %v102855 = load i64, ptr %t607, align 8
  %t5724.not = icmp eq i64 %v102855, 0
  br i1 %t5724.not, label %edge8958, label %edge8951

edge8951:                                         ; preds = %edge8950
  %t5725 = inttoptr i64 %v102855 to ptr
  %v102856 = load i64, ptr %t5725, align 1
  %v102799 = add i64 %v102856, 2
  store i64 %v102799, ptr %t5725, align 1
  %v102803 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v102855)
  %t5706.not = icmp eq i64 %v102803, 0
  br i1 %t5706.not, label %edge8958.thread, label %edge8928

edge8958:                                         ; preds = %edge8950
  %v102981 = add i64 %v102853, 8
  %t5731 = inttoptr i64 %v102981 to ptr
  %v102982 = load i64, ptr %t5731, align 1
  %t5732 = icmp eq i64 %v102982, 2
  br i1 %t5732, label %edge8988, label %block55030

edge8958.thread:                                  ; preds = %edge8951
  %v102981904 = add i64 %v102853, 8
  %t5731905 = inttoptr i64 %v102981904 to ptr
  %v102982906 = load i64, ptr %t5731905, align 1
  %t5732907 = icmp eq i64 %v102982906, 2
  br i1 %t5732907, label %edge8988, label %edge8962

edge8962:                                         ; preds = %edge8958.thread
  %v102998 = load i64, ptr %t5725, align 1
  %t5736 = icmp eq i64 %v102998, 1
  br i1 %t5736, label %block55028.thread443, label %block55030

block55028.thread443:                             ; preds = %edge8962
  store i64 2, ptr %t5731905, align 1
  br label %edge8988

edge8970:                                         ; preds = %block55028
  %t5760 = inttoptr i64 %v103010 to ptr
  %v103014 = load i64, ptr %t5760, align 1
  %v103015 = add i64 %v103014, 1
  store i64 %v103015, ptr %t5760, align 1
  br label %block55034

edge8973:                                         ; preds = %block55034
  %t5764 = inttoptr i64 %v102855 to ptr
  %v103018 = load i64, ptr %t5764, align 1
  %v103019 = add i64 %v103018, -1
  store i64 %v103019, ptr %t5764, align 1
  %t5766.not = icmp eq i64 %v103019, 0
  br i1 %t5766.not, label %edge8976, label %edge8988

edge8976:                                         ; preds = %edge8973
  %v103020 = call i64 @mech_compact_free(i64 %v0, i64 %v102855)
  %t5767.not = icmp eq i64 %v103020, 0
  br i1 %t5767.not, label %edge8988, label %block55025

edge8988:                                         ; preds = %edge8958.thread, %edge8958, %block55028.thread443, %block55034, %edge8973, %edge8976
  %v102986439459 = phi i64 [ %v103008, %edge8976 ], [ %v103008, %block55034 ], [ %v103008, %edge8973 ], [ %v102853, %block55028.thread443 ], [ %v102853, %edge8958 ], [ %v102853, %edge8958.thread ]
  %v102988440458 = phi i64 [ %v103009, %edge8976 ], [ %v103009, %block55034 ], [ %v103009, %edge8973 ], [ %v102854, %block55028.thread443 ], [ %v102854, %edge8958 ], [ %v102854, %edge8958.thread ]
  %v102991441457 = phi i64 [ %v103010, %edge8976 ], [ %v103010, %block55034 ], [ %v103010, %edge8973 ], [ %v102855, %block55028.thread443 ], [ 0, %edge8958 ], [ %v102855, %edge8958.thread ]
  %t5770 = icmp ult i64 %v102044, %v218975
  br i1 %t5770, label %edge8996, label %block55283

edge8996:                                         ; preds = %edge8988
  %v103355 = shl nuw nsw i64 %v102044, 2
  %v103356 = add i64 %v96816, %v103355
  %t5773 = inttoptr i64 %v103356 to ptr
  %t5774 = load i32, ptr %t5773, align 1
  %55 = and i32 %t5774, 1023
  %t5779 = icmp ult i32 %t5774, 1024
  br i1 %t5779, label %block55286, label %edge9014

edge9014:                                         ; preds = %edge8996
  %t5781.not = icmp eq i64 %v96791, 0
  br i1 %t5781.not, label %block55288, label %edge9015

edge9015:                                         ; preds = %edge9014
  %t5782 = inttoptr i64 %v96791 to ptr
  %v103434 = load i64, ptr %t5782, align 1
  %t5783 = icmp eq i64 %v103434, 1
  br i1 %t5783, label %block55287, label %block55288

edge9020:                                         ; preds = %block55288
  %v103444 = load i64, ptr %ss1, align 8
  %v103445 = load i64, ptr %t606, align 8
  %v103446 = load i64, ptr %t607, align 8
  %.pre21760 = add i64 %v103444, %v103355
  %.pre21761 = inttoptr i64 %.pre21760 to ptr
  br label %block55287

edge9023:                                         ; preds = %block55286
  %t5807 = inttoptr i64 %v103427 to ptr
  %v103451 = load i64, ptr %t5807, align 1
  %v103452 = add i64 %v103451, 1
  store i64 %v103452, ptr %t5807, align 1
  br label %block55292

edge9026:                                         ; preds = %block55292
  %t5811 = inttoptr i64 %v96791 to ptr
  %v103455 = load i64, ptr %t5811, align 1
  %v103456 = add i64 %v103455, -1
  store i64 %v103456, ptr %t5811, align 1
  %t5813.not = icmp eq i64 %v103456, 0
  br i1 %t5813.not, label %edge9029, label %edge9039

edge9029:                                         ; preds = %edge9026
  %v103457 = call i64 @mech_compact_free(i64 %v0, i64 %v96791)
  %t5814.not = icmp eq i64 %v103457, 0
  br i1 %t5814.not, label %edge9039, label %block55283

edge9039:                                         ; preds = %edge9029, %edge9026, %block55292
  %t5912 = zext nneg i64 %v102058 to i128
  %t5913 = shl nuw nsw i128 %t5912, 64
  br label %block4548

edge9054:                                         ; preds = %block4548
  %v103826 = add nsw i64 %v103532, -1
  %v104140 = shl nuw nsw i64 %v103532, 2
  %v104143 = add i64 %v103508, -4
  %v104144 = add i64 %v104143, %v104140
  %t5823 = inttoptr i64 %v104144 to ptr
  %t5824 = load i32, ptr %t5823, align 1
  %v104145 = zext i32 %t5824 to i64
  %v104179 = lshr i64 %v104145, 10
  %t5826 = icmp samesign ult i64 %v104179, %v218975
  br i1 %t5826, label %edge9100, label %edge9113

edge9100:                                         ; preds = %edge9054
  %v104496 = shl nuw nsw i64 %v104179, 2
  %v104497 = add i64 %v104496, %v103508
  %t5829 = inttoptr i64 %v104497 to ptr
  %t5830 = load i32, ptr %t5829, align 1
  %56 = lshr i32 %t5830, 10
  %narrow183 = add nuw nsw i32 %56, 1
  %57 = zext nneg i32 %narrow183 to i64
  br label %edge9113

edge9113:                                         ; preds = %edge9054, %edge9100
  %v104518 = phi i64 [ %57, %edge9100 ], [ 1, %edge9054 ]
  %v104541 = and i64 %v104145, 1023
  %v104563 = shl nuw nsw i64 %v104518, 10
  %v104563.masked = and i64 %v104563, 4294966272
  %v104573 = or disjoint i64 %v104563.masked, %v104541
  %v104592 = shl nuw nsw i64 %v103826, 2
  %v104593 = add i64 %v104592, %v103508
  %t5839 = inttoptr i64 %v104593 to ptr
  %t5840 = load i32, ptr %t5839, align 1
  %v104594 = zext i32 %t5840 to i64
  %t5841 = icmp eq i64 %v104573, %v104594
  br i1 %t5841, label %block56048, label %edge9128

edge9128:                                         ; preds = %edge9113
  %t5843.not = icmp eq i64 %v103513, 0
  br i1 %t5843.not, label %block56050, label %edge9129

edge9129:                                         ; preds = %edge9128
  %t5844 = inttoptr i64 %v103513 to ptr
  %v104610 = load i64, ptr %t5844, align 1
  %t5845 = icmp eq i64 %v104610, 1
  br i1 %t5845, label %block56049, label %block56050

edge9134:                                         ; preds = %block56050
  %v104620 = load i64, ptr %ss1, align 8
  %v104621 = load i64, ptr %t606, align 8
  %v104622 = load i64, ptr %t607, align 8
  %.pre21758 = add i64 %v104620, %v104592
  %.pre21759 = inttoptr i64 %.pre21758 to ptr
  br label %block56049

edge9137:                                         ; preds = %block56048
  %t5869 = inttoptr i64 %v104603 to ptr
  %v104627 = load i64, ptr %t5869, align 1
  %v104628 = add i64 %v104627, 1
  store i64 %v104628, ptr %t5869, align 1
  br label %block56054

edge9146:                                         ; preds = %block56054
  %t5877.not = icmp eq i64 %v103525, 0
  br i1 %t5877.not, label %edge9152, label %edge9147

edge9147:                                         ; preds = %edge9146
  %t5878 = inttoptr i64 %v103525 to ptr
  %v104915 = load i64, ptr %t5878, align 1
  %v104916 = add i64 %v104915, 1
  store i64 %v104916, ptr %t5878, align 1
  br label %edge9152

edge9152:                                         ; preds = %edge9146, %edge9147
  %v105299 = add i64 %v103520, -8
  br label %block4623

edge9162:                                         ; preds = %block4623
  %v104971 = add nsw i64 %v104959, -1
  %v105296 = shl i64 %v104959, 3
  %v105300 = add i64 %v105299, %v105296
  %t5887 = inttoptr i64 %v105300 to ptr
  %v105301 = load i64, ptr %t5887, align 1
  %t5888.not = icmp eq i64 %v105301, 0
  br i1 %t5888.not, label %block4623, label %edge9185

edge9185:                                         ; preds = %edge9162
  br i1 %t5877.not, label %edge9219, label %edge9189

edge9189:                                         ; preds = %edge9185
  %v105338 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v103525)
  %t5892.not = icmp eq i64 %v105338, 0
  br i1 %t5892.not, label %edge9219, label %edge9190

edge9190:                                         ; preds = %edge9189
  %t5893 = insertvalue { i64, i64 } poison, i64 %v105338, 0
  %t5894 = insertvalue { i64, i64 } %t5893, i64 4717, 1
  br label %common.ret

edge9219:                                         ; preds = %edge9185, %edge9189, %block56054
  %v105684 = phi i64 [ %v104518, %block56054 ], [ %v104971, %edge9189 ], [ %v104971, %edge9185 ]
  %t5895 = icmp ult i64 %v105684, %v102058
  br i1 %t5895, label %edge9224, label %common.ret

edge9224:                                         ; preds = %edge9219
  %v106000 = add nuw nsw i64 %v105684, 1
  %v106006 = shl nuw nsw i64 %v105684, 3
  %v106007 = add i64 %v106006, %v103520
  %t5898 = inttoptr i64 %v106007 to ptr
  %v106008 = load i64, ptr %t5898, align 1
  %t5899.not = icmp eq i64 %v106008, 0
  br i1 %t5899.not, label %common.ret, label %edge9231

edge9231:                                         ; preds = %edge9224
  %v106036 = add i64 %v106008, -1
  %t5906.not = icmp eq i64 %v103525, 0
  br i1 %t5906.not, label %block57036, label %edge9238

edge9238:                                         ; preds = %edge9231
  %t5907 = inttoptr i64 %v103525 to ptr
  %v106067 = load i64, ptr %t5907, align 1
  %t5908 = icmp eq i64 %v106067, 1
  br i1 %t5908, label %block57035, label %block57036

edge9242:                                         ; preds = %block57036
  %t5935 = insertvalue { i64, i64 } poison, i64 %v106076, 0
  %t5936 = insertvalue { i64, i64 } %t5935, i64 4756, 1
  br label %common.ret

edge9243:                                         ; preds = %block57036
  %v106077 = load i64, ptr %ss1, align 8
  %v106078 = load i64, ptr %t606, align 8
  %v106079 = load i64, ptr %t607, align 8
  %.pre21754 = add i64 %v106077, %v106006
  %.pre21755 = inttoptr i64 %.pre21754 to ptr
  br label %block57035

edge9246:                                         ; preds = %block57035
  %t5932 = inttoptr i64 %v106066 to ptr
  %v106083 = load i64, ptr %t5932, align 1
  %v106084 = add i64 %v106083, 1
  store i64 %v106084, ptr %t5932, align 1
  br label %block57040

edge9275:                                         ; preds = %block57040
  %v106419 = shl nuw nsw i64 %v106000, 3
  %v106420 = add i64 %v106061, %v106419
  %t5940 = inttoptr i64 %v106420 to ptr
  %v106421 = load i64, ptr %t5940, align 1
  %v106449 = add i64 %v106421, 2
  %t5941.not = icmp ugt i64 %v106421, -3
  br i1 %t5941.not, label %common.ret, label %edge9285

edge9285:                                         ; preds = %edge9275
  %t5950.not = icmp eq i64 %v106085, 0
  br i1 %t5950.not, label %block57300, label %edge9288

edge9288:                                         ; preds = %edge9285
  %v106486 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v106085)
  %t5951.not = icmp eq i64 %v106486, 0
  br i1 %t5951.not, label %block57300, label %block57294

edge9292:                                         ; preds = %block57300
  %t5953 = inttoptr i64 %v106066 to ptr
  %v106493 = load i64, ptr %t5953, align 1
  %t5954 = icmp eq i64 %v106493, 1
  br i1 %t5954, label %block57297, label %block57303

edge9297:                                         ; preds = %block57303
  %v106503 = load i64, ptr %ss1, align 8
  %v106504 = load i64, ptr %t606, align 8
  %v106505 = load i64, ptr %t607, align 8
  %.pre21756 = add i64 %v106503, %v106419
  %.pre21757 = inttoptr i64 %.pre21756 to ptr
  br label %block57297

edge9300:                                         ; preds = %block57297
  %t5978 = inttoptr i64 %v106492 to ptr
  %v106509 = load i64, ptr %t5978, align 1
  %v106510 = add i64 %v106509, 1
  store i64 %v106510, ptr %t5978, align 1
  br label %block57307

edge9303:                                         ; preds = %block57307
  %t5982 = inttoptr i64 %v106066 to ptr
  %v106513 = load i64, ptr %t5982, align 1
  %v106514 = add i64 %v106513, -1
  store i64 %v106514, ptr %t5982, align 1
  %t5984.not = icmp eq i64 %v106514, 0
  br i1 %t5984.not, label %edge9306, label %edge9333

edge9306:                                         ; preds = %edge9303
  %v106515 = call i64 @mech_compact_free(i64 %v0, i64 %v106066)
  %t5985.not = icmp eq i64 %v106515, 0
  br i1 %t5985.not, label %edge9333, label %block57294

edge9317:                                         ; preds = %block4548
  %t5988.not = icmp eq i64 %v103513, 0
  br i1 %t5988.not, label %block57334, label %edge9318

edge9318:                                         ; preds = %edge9317
  %t5989 = inttoptr i64 %v103513 to ptr
  %v106561 = load i64, ptr %t5989, align 1
  %v106562 = add i64 %v106561, 1
  store i64 %v106562, ptr %t5989, align 1
  br label %block57334

edge9321:                                         ; preds = %block57334
  %t5992 = inttoptr i64 %v103525 to ptr
  %v106563 = load i64, ptr %t5992, align 1
  %v106564 = add i64 %v106563, 1
  store i64 %v106564, ptr %t5992, align 1
  br label %edge9334

edge9333:                                         ; preds = %edge9306, %edge9303, %block57307
  %t6003.not = icmp eq i64 %v104629, 0
  br i1 %t6003.not, label %block4548.backedge, label %edge9351

block4548.backedge:                               ; preds = %edge9333, %edge9351
  br label %block4548

edge9334:                                         ; preds = %edge9321, %block57334
  br i1 %t5988.not, label %block57362, label %edge9337

edge9337:                                         ; preds = %edge9334
  %v106614 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v103513)
  %t5997.not = icmp eq i64 %v106614, 0
  br i1 %t5997.not, label %block57362, label %block57357

edge9341:                                         ; preds = %block57362
  %t5999 = inttoptr i64 %v103525 to ptr
  %v106616 = load i64, ptr %t5999, align 1
  %v106617 = add i64 %v106616, -1
  store i64 %v106617, ptr %t5999, align 1
  %t6001.not = icmp eq i64 %v106617, 0
  br i1 %t6001.not, label %edge9344, label %edge9395

edge9344:                                         ; preds = %edge9341
  %v106618 = call i64 @mech_compact_free(i64 %v0, i64 %v103525)
  %t6002.not = icmp eq i64 %v106618, 0
  br i1 %t6002.not, label %edge9395, label %block57357

edge9351:                                         ; preds = %edge9333
  %v106622 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v104629)
  %t6004.not = icmp eq i64 %v106622, 0
  br i1 %t6004.not, label %block4548.backedge, label %block57357

edge9395:                                         ; preds = %block57362, %edge9341, %edge9344
  br i1 %t5988.not, label %block57671, label %edge9396

edge9396:                                         ; preds = %edge9395
  %t6013 = inttoptr i64 %v103513 to ptr
  %v107082 = load i64, ptr %t6013, align 1
  %v107083 = add i64 %v107082, 1
  store i64 %v107083, ptr %t6013, align 1
  br label %block57671

edge9399:                                         ; preds = %block57671
  %t6016 = inttoptr i64 %v103525 to ptr
  %v107084 = load i64, ptr %t6016, align 1
  %v107085 = add i64 %v107084, 1
  store i64 %v107085, ptr %t6016, align 1
  br label %edge9407

edge9407:                                         ; preds = %edge9399, %block57671
  %t6089 = inttoptr i64 %v103513 to ptr
  br label %block4742

edge9408:                                         ; preds = %block4742
  %t6019 = inttoptr i64 %v107108 to ptr
  %v107109 = load i64, ptr %t6019, align 1
  %t6020 = icmp ne i64 %v107109, 1
  %t6022.not = icmp ugt i64 %v218975, %v107149
  %or.cond318 = select i1 %t6020, i1 true, i1 %t6022.not
  br i1 %or.cond318, label %edge9449.thread, label %edge9412

edge9412:                                         ; preds = %edge9408
  %t6024 = zext i64 %v107150 to i128
  %t6025 = zext i64 %v107151 to i128
  %t6026 = shl nuw i128 %t6025, 64
  %v220125 = or disjoint i128 %t6026, %t6024
  store i128 %v220125, ptr %ss2, align 16
  %t6028 = zext i64 %v103508 to i128
  %v220127 = or disjoint i128 %t4919, %t6028
  store i128 %v220127, ptr %t926, align 16
  store i128 %v220027, ptr %t931, align 16
  %t6038 = zext i64 %v107156 to i128
  %v220131 = or disjoint i128 %t4919, %t6038
  store i128 %v220131, ptr %t936, align 16
  store i128 %v220027, ptr %t941, align 16
  %t6048 = zext i64 %v103520 to i128
  %v220135 = or disjoint i128 %t5913, %t6048
  store i128 %v220135, ptr %t946, align 16
  %v220137 = or disjoint i128 %t5913, 1
  store i128 %v220137, ptr %t947, align 16
  %t6058 = zext i64 %v107108 to i128
  %t6059 = zext i64 %v107149 to i128
  %t6060 = shl nuw i128 %t6059, 64
  %v220139 = or disjoint i128 %t6060, %t6058
  store i128 %v220139, ptr %t1152, align 16
  %t6069 = call fastcc { i64, i64 } @mech_compact_numeric_4810_vector(i64 %v219576, i64 %v8481)
  %v107125 = extractvalue { i64, i64 } %t6069, 0
  %t6070.not = icmp eq i64 %v107125, 0
  br i1 %t6070.not, label %edge9419, label %edge9418

edge9418:                                         ; preds = %edge9412
  %v107126 = extractvalue { i64, i64 } %t6069, 1
  %t6071 = icmp eq i64 %v107125, 103169
  %v107131 = select i1 %t6071, i64 103425, i64 %v107125
  %t6073 = insertvalue { i64, i64 } poison, i64 %v107131, 0
  %t6074 = insertvalue { i64, i64 } %t6073, i64 %v107126, 1
  br label %common.ret

edge9419:                                         ; preds = %edge9412
  br i1 %t5988.not, label %edge9426, label %edge9422

edge9422:                                         ; preds = %edge9419
  %v107136 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v103513)
  %t6076.not = icmp eq i64 %v107136, 0
  br i1 %t6076.not, label %edge9426, label %block57691

edge9426:                                         ; preds = %edge9422, %edge9419
  br i1 %t5998.not, label %edge9689, label %edge9429

edge9429:                                         ; preds = %edge9426
  %v107144 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v103525)
  %t6078.not = icmp eq i64 %v107144, 0
  br i1 %t6078.not, label %edge9689, label %block57691

edge9449:                                         ; preds = %block4742
  %t6081.not = icmp eq i64 %v107150, 0
  br i1 %t6081.not, label %edge9606, label %edge9463

edge9449.thread:                                  ; preds = %edge9408
  %t6081.not931 = icmp eq i64 %v107150, 0
  br i1 %t6081.not931, label %edge9596, label %edge9463

edge9463:                                         ; preds = %edge9449.thread, %edge9449
  %t6084 = icmp ult i64 %v107150, %v102058
  br i1 %t6084, label %edge9471, label %edge9479

edge9471:                                         ; preds = %edge9463
  %v107832 = shl nuw nsw i64 %v107150, 3
  %v107833 = add i64 %v107832, %v103520
  %t6087 = inttoptr i64 %v107833 to ptr
  %v107834 = load i64, ptr %t6087, align 1
  br label %edge9479

edge9479:                                         ; preds = %edge9463, %edge9471
  %v107855 = phi i64 [ %v107834, %edge9471 ], [ 0, %edge9463 ]
  br i1 %t5988.not, label %edge9486, label %edge9480

edge9480:                                         ; preds = %edge9479
  %v107858 = load i64, ptr %t6089, align 1
  %v107859 = add i64 %v107858, 1
  store i64 %v107859, ptr %t6089, align 1
  br label %edge9486

edge9486:                                         ; preds = %edge9480, %edge9479
  %58 = trunc i64 %v107150 to i8
  br label %block4778

edge9503:                                         ; preds = %block4778
  br i1 %t5988.not, label %edge9637, label %edge9504

edge9504:                                         ; preds = %edge9503
  %v107947 = load i64, ptr %t6089, align 1
  %v107948 = add i64 %v107947, -1
  store i64 %v107948, ptr %t6089, align 1
  %t6097.not = icmp eq i64 %v107948, 0
  br i1 %t6097.not, label %edge9507, label %edge9637

edge9507:                                         ; preds = %edge9504
  %v107949 = call i64 @mech_compact_free(i64 %v0, i64 %v103513)
  %t6098.not = icmp eq i64 %v107949, 0
  br i1 %t6098.not, label %edge9637, label %edge9508

edge9508:                                         ; preds = %edge9507
  %t6099 = insertvalue { i64, i64 } poison, i64 %v107949, 0
  %t6100 = insertvalue { i64, i64 } %t6099, i64 4860, 1
  br label %common.ret

edge9522:                                         ; preds = %block4778
  %t6101 = icmp ult i64 %v107925, %v218975
  br i1 %t6101, label %edge9539, label %edge9545

edge9539:                                         ; preds = %edge9522
  %v108569 = shl nuw nsw i64 %v107925, 2
  %v108570 = add i64 %v108569, %v103508
  %t6104 = inttoptr i64 %v108570 to ptr
  %t6105 = load i32, ptr %t6104, align 1
  %59 = and i32 %t6105, 1023
  %60 = zext nneg i32 %59 to i64
  %t6106 = icmp samesign ugt i64 %v218975, %60
  br i1 %t6106, label %edge9545, label %block58737

edge9545:                                         ; preds = %edge9522, %edge9539
  %v108593930 = phi i64 [ %60, %edge9539 ], [ 0, %edge9522 ]
  %v108651 = add i64 %v108593930, %v107910
  %t6108 = inttoptr i64 %v108651 to ptr
  %t6109 = load i8, ptr %t6108, align 1
  %t6110 = icmp eq i8 %t6109, %58
  br i1 %t6110, label %block58740, label %edge9548

edge9548:                                         ; preds = %edge9545
  %t6114.not = icmp eq i64 %v107915, 0
  br i1 %t6114.not, label %block58746, label %edge9555

edge9555:                                         ; preds = %edge9548
  %t6115 = inttoptr i64 %v107915 to ptr
  %v108672 = load i64, ptr %t6115, align 1
  %t6116 = icmp eq i64 %v108672, 1
  br i1 %t6116, label %block58745, label %block58746

edge9560:                                         ; preds = %block58746
  %v108682 = load i64, ptr %ss1, align 8
  %v108683 = load i64, ptr %t606, align 8
  %v108684 = load i64, ptr %t607, align 8
  %.pre21752 = add i64 %v108682, %v108593930
  %.pre21753 = inttoptr i64 %.pre21752 to ptr
  br label %block58745

edge9563:                                         ; preds = %block58740
  %t6140 = inttoptr i64 %v108661 to ptr
  %v108689 = load i64, ptr %t6140, align 1
  %v108690 = add i64 %v108689, 1
  store i64 %v108690, ptr %t6140, align 1
  br label %block58750

edge9568:                                         ; preds = %block58750
  %v108695 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v107915)
  %t6144.not = icmp eq i64 %v108695, 0
  br i1 %t6144.not, label %edge9572, label %block58737

edge9572:                                         ; preds = %edge9568, %block58750
  %t6147.not = icmp eq i64 %v107925, -1
  br i1 %t6147.not, label %common.ret, label %edge9576

edge9576:                                         ; preds = %edge9572
  %v108701 = add nuw i64 %v107925, 1
  %v108710 = add i64 %v107928, -1
  br label %block4778

edge9596:                                         ; preds = %edge9449.thread
  %v108783 = add i64 %v107109, 1
  store i64 %v108783, ptr %t6019, align 1
  br label %edge9606

edge9606:                                         ; preds = %edge9449, %edge9596
  br i1 %t5988.not, label %block58825, label %edge9609

edge9609:                                         ; preds = %edge9606
  %v108816 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v103513)
  %t6157.not = icmp eq i64 %v108816, 0
  br i1 %t6157.not, label %block58825, label %block58820

edge9615:                                         ; preds = %block58825
  %v108820 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v103525)
  %t6159.not = icmp eq i64 %v108820, 0
  br i1 %t6159.not, label %block58829, label %block58820

edge9619:                                         ; preds = %block58829
  %t6161 = inttoptr i64 %v107108 to ptr
  %v108822 = load i64, ptr %t6161, align 1
  %v108823 = add i64 %v108822, -1
  store i64 %v108823, ptr %t6161, align 1
  %t6163.not = icmp eq i64 %v108823, 0
  br i1 %t6163.not, label %edge9622, label %edge9689

edge9622:                                         ; preds = %edge9619
  %v108824 = call i64 @mech_compact_free(i64 %v0, i64 %v107108)
  %t6164.not = icmp eq i64 %v108824, 0
  br i1 %t6164.not, label %edge9689, label %block58820

edge9637:                                         ; preds = %edge9507, %edge9504, %edge9503
  %v109110 = add i64 %v107150, -1
  br label %block4742

edge9668:                                         ; preds = %edge9692
  %v110008 = call i64 @mech_compact_free(i64 %v0, i64 %v110193)
  %t6184.not = icmp eq i64 %v110008, 0
  br i1 %t6184.not, label %edge9703, label %edge9669

edge9669:                                         ; preds = %edge9668
  %t6185 = insertvalue { i64, i64 } poison, i64 %v110008, 0
  %t6186 = insertvalue { i64, i64 } %t6185, i64 4920, 1
  br label %common.ret

edge9689:                                         ; preds = %block58829, %edge9619, %edge9622, %edge9426, %edge9429
  %v110188 = call i64 @mech_compact_allocate(i64 %v0, i64 17, i64 4, i64 4, i64 1, i64 %v3194)
  %t6199.not = icmp eq i64 %v110188, 0
  br i1 %t6199.not, label %edge9691, label %block59111

edge9691:                                         ; preds = %edge9689
  %v110191 = load i64, ptr %ss1, align 8
  %v110192 = load i64, ptr %t606, align 8
  %v110193 = load i64, ptr %t607, align 8
  %t6202.not = icmp eq i64 %v110193, 0
  br i1 %t6202.not, label %edge9703, label %edge9692

edge9692:                                         ; preds = %edge9691
  %t6203 = inttoptr i64 %v110193 to ptr
  %v110194 = load i64, ptr %t6203, align 1
  %v110195 = add i64 %v110194, 1
  store i64 %v110195, ptr %t6203, align 1
  %t6183.not = icmp eq i64 %v110195, 0
  br i1 %t6183.not, label %edge9668, label %edge9703

edge9703:                                         ; preds = %edge9691, %edge9668, %edge9692
  %v111425 = add i64 %v103520, -8
  %61 = add nuw nsw i64 %v218977, 1
  br label %block4862

edge9704:                                         ; preds = %block4862
  %t6208 = inttoptr i64 %v110380 to ptr
  %v110381 = load i64, ptr %t6208, align 1
  %t6209 = icmp eq i64 %v110381, 1
  %t6211 = icmp ugt i64 %v110404, 16
  %or.cond319 = select i1 %t6209, i1 %t6211, i1 false
  br i1 %or.cond319, label %edge9708, label %edge9733

edge9708:                                         ; preds = %edge9704
  %t6213 = zext nneg i64 %v218977 to i128
  %t6214 = zext nneg i64 %v110406 to i128
  %t6215 = shl nuw nsw i128 %t6214, 64
  %v220147 = or disjoint i128 %t6215, %t6213
  store i128 %v220147, ptr %ss2, align 16
  %t6217 = zext i64 %v103520 to i128
  %v220149 = or disjoint i128 %t5913, %t6217
  store i128 %v220149, ptr %t926, align 16
  %v220151 = or disjoint i128 %t5913, 1
  store i128 %v220151, ptr %t931, align 16
  %t6227 = zext i64 %v110411 to i128
  %v220153 = or disjoint i128 %t6227, 313594649253062377472
  store i128 %v220153, ptr %t936, align 16
  store i128 313594649253062377473, ptr %t941, align 16
  %t6237 = zext i64 %v110380 to i128
  %t6238 = zext i64 %v110404 to i128
  %t6239 = shl nuw i128 %t6238, 64
  %v220157 = or disjoint i128 %t6239, %t6237
  store i128 %v220157, ptr %t946, align 16
  %t6242 = call fastcc { i64, i64 } @mech_compact_numeric_4934(i64 %v219576, i64 %v8481)
  %v110386 = extractvalue { i64, i64 } %t6242, 0
  %t6243.not = icmp eq i64 %v110386, 0
  br i1 %t6243.not, label %edge9711, label %edge9710

edge9710:                                         ; preds = %edge9708
  %v110387 = extractvalue { i64, i64 } %t6242, 1
  %t6244 = icmp eq i64 %v110386, 103169
  %v110390 = select i1 %t6244, i64 103425, i64 %v110386
  %t6246 = insertvalue { i64, i64 } poison, i64 %v110390, 0
  %t6247 = insertvalue { i64, i64 } %t6246, i64 %v110387, 1
  br label %common.ret

edge9711:                                         ; preds = %edge9708
  br i1 %t5998.not, label %block4939.preheader, label %edge9712

edge9712:                                         ; preds = %edge9711
  %t6249 = inttoptr i64 %v103525 to ptr
  %v110393 = load i64, ptr %t6249, align 1
  %v110394 = add i64 %v110393, -1
  store i64 %v110394, ptr %t6249, align 1
  %t6251.not = icmp eq i64 %v110394, 0
  br i1 %t6251.not, label %edge9715, label %block4939.preheader

edge9715:                                         ; preds = %edge9712
  %v110395 = call i64 @mech_compact_free(i64 %v0, i64 %v103525)
  %t6252.not = icmp eq i64 %v110395, 0
  br i1 %t6252.not, label %block4939.preheader, label %edge9716

edge9716:                                         ; preds = %edge9715
  %t6253 = insertvalue { i64, i64 } poison, i64 %v110395, 0
  %t6254 = insertvalue { i64, i64 } %t6253, i64 4934, 1
  br label %common.ret

edge9733:                                         ; preds = %edge9704, %block4862
  %exitcond21637 = icmp eq i64 %v110406, %61
  br i1 %exitcond21637, label %edge9736, label %edge9800

edge9736:                                         ; preds = %edge9733
  br i1 %t5998.not, label %block4939.preheader, label %edge9737

block4939.preheader:                              ; preds = %edge9711, %edge9712, %edge9715, %edge9740, %edge9737, %edge9736
  br label %block4939

edge9737:                                         ; preds = %edge9736
  %t6259 = inttoptr i64 %v103525 to ptr
  %v110471 = load i64, ptr %t6259, align 1
  %v110472 = add i64 %v110471, -1
  store i64 %v110472, ptr %t6259, align 1
  %t6261.not = icmp eq i64 %v110472, 0
  br i1 %t6261.not, label %edge9740, label %block4939.preheader

edge9740:                                         ; preds = %edge9737
  %v110473 = call i64 @mech_compact_free(i64 %v0, i64 %v103525)
  %t6262.not = icmp eq i64 %v110473, 0
  br i1 %t6262.not, label %block4939.preheader, label %edge9741

edge9741:                                         ; preds = %edge9740
  %t6263 = insertvalue { i64, i64 } poison, i64 %v110473, 0
  %t6264 = insertvalue { i64, i64 } %t6263, i64 4945, 1
  br label %common.ret

edge9800:                                         ; preds = %edge9733
  %v111070 = shl i64 %v110406, 2
  %v111073 = add nsw i64 %v111070, -4
  %v111074 = add i64 %v111073, %v110411
  %t6272 = inttoptr i64 %v111074 to ptr
  %t6273 = load i32, ptr %t6272, align 1
  %v111422 = shl i64 %v110406, 3
  %v111426 = add i64 %v111425, %v111422
  %t6277 = inttoptr i64 %v111426 to ptr
  %v111427 = load i64, ptr %t6277, align 1
  %62 = trunc i64 %v111427 to i32
  %v111479.narrow = add i32 %t6273, %62
  %v111484 = zext i32 %v111479.narrow to i64
  %v111500 = shl nuw nsw i64 %v111484, 1
  %v111516 = add nuw nsw i64 %v110406, 1
  %v111505 = and i64 %v111500, 4294967294
  %v111525 = add i64 %v110411, %v111070
  %t6281 = inttoptr i64 %v111525 to ptr
  %t6282 = load i32, ptr %t6281, align 1
  %v111526 = zext i32 %t6282 to i64
  %t6283 = icmp eq i64 %v111505, %v111526
  br i1 %t6283, label %block59925, label %edge9819

edge9819:                                         ; preds = %edge9800
  br i1 %t6207.not, label %block59927, label %edge9820

edge9820:                                         ; preds = %edge9819
  %t6286 = inttoptr i64 %v110380 to ptr
  %v111542 = load i64, ptr %t6286, align 1
  %t6287 = icmp eq i64 %v111542, 1
  br i1 %t6287, label %block59926, label %block59927

edge9825:                                         ; preds = %block59927
  %v111552 = load i64, ptr %ss1, align 8
  %v111553 = load i64, ptr %t606, align 8
  %v111554 = load i64, ptr %t607, align 8
  %.pre21750 = add i64 %v111552, %v111070
  %.pre21751 = inttoptr i64 %.pre21750 to ptr
  br label %block59926

edge9828:                                         ; preds = %block59925
  %t6311 = inttoptr i64 %v111535 to ptr
  %v111559 = load i64, ptr %t6311, align 1
  %v111560 = add i64 %v111559, 1
  store i64 %v111560, ptr %t6311, align 1
  br label %block59931

edge9833:                                         ; preds = %block59931
  %v111565 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v110380)
  %t6315.not = icmp eq i64 %v111565, 0
  br i1 %t6315.not, label %block4862.backedge, label %block59922

block4862.backedge:                               ; preds = %edge9833, %block59931
  br label %block4862

edge9864:                                         ; preds = %block4939
  %t6323 = inttoptr i64 %v111836 to ptr
  %v111837 = load i64, ptr %t6323, align 1
  %t6324 = icmp eq i64 %v111837, 1
  %t6326 = icmp ne i64 %v111870, 0
  %or.cond31 = select i1 %t6324, i1 %t6326, i1 false
  br i1 %or.cond31, label %edge9868, label %edge9910

edge9868:                                         ; preds = %edge9864
  %t6327 = inttoptr i64 %v111870 to ptr
  %v111839 = load i64, ptr %t6327, align 1
  %t6328 = icmp eq i64 %v111839, 1
  %t6330.not = icmp ule i64 %v218975, %v111872
  %or.cond320.not1300 = select i1 %t6328, i1 %t6330.not, i1 false
  %t6332 = icmp ugt i64 %v111874, 16
  %or.cond321 = select i1 %or.cond320.not1300, i1 %t6332, i1 false
  br i1 %or.cond321, label %edge9874, label %edge9910

edge9874:                                         ; preds = %edge9868
  %t6335 = zext nneg i64 %v111876 to i128
  %t6336 = shl nuw nsw i128 %t6335, 64
  %v220165 = or disjoint i128 %t6336, %t4918
  store i128 %v220165, ptr %ss2, align 16
  %t6338 = zext i64 %v111877 to i128
  %v220167 = or disjoint i128 %t4919, %t6338
  store i128 %v220167, ptr %t926, align 16
  store i128 %v220027, ptr %t931, align 16
  %t6348 = zext i64 %v107156 to i128
  %v220171 = or disjoint i128 %t4919, %t6348
  store i128 %v220171, ptr %t936, align 16
  store i128 %v220027, ptr %t941, align 16
  %t6358 = zext i64 %v111884 to i128
  %v220175 = or disjoint i128 %t6358, 313594649253062377472
  store i128 %v220175, ptr %t946, align 16
  store i128 313594649253062377473, ptr %t947, align 16
  store i128 %v220179, ptr %t1152, align 16
  store i128 4722366482869645213697, ptr %t1157, align 16
  %t6378 = zext i64 %v111836 to i128
  %t6379 = zext i64 %v111870 to i128
  %t6380 = shl nuw i128 %t6379, 64
  %v220183 = or disjoint i128 %t6380, %t6378
  store i128 %v220183, ptr %t1162, align 16
  %t6383 = zext i64 %v111872 to i128
  %t6384 = zext i64 %v111874 to i128
  %t6385 = shl nuw i128 %t6384, 64
  %v220185 = or disjoint i128 %t6385, %t6383
  store i128 %v220185, ptr %t1167, align 16
  %t6388 = call fastcc { i64, i64 } @mech_compact_numeric_5014(i64 %v219576, i64 %v8481)
  %v111845 = extractvalue { i64, i64 } %t6388, 0
  %t6389.not = icmp eq i64 %v111845, 0
  br i1 %t6389.not, label %edge9878, label %edge9876

edge9876:                                         ; preds = %edge9874
  %v111846 = extractvalue { i64, i64 } %t6388, 1
  %t6390 = icmp eq i64 %v111845, 103169
  %v111849 = select i1 %t6390, i64 103425, i64 %v111845
  %t6392 = insertvalue { i64, i64 } poison, i64 %v111849, 0
  %t6393 = insertvalue { i64, i64 } %t6392, i64 %v111846, 1
  br label %common.ret

edge9878:                                         ; preds = %edge9874
  %t6394.not = icmp eq i64 %v107108, 0
  br i1 %t6394.not, label %edge9886, label %edge9881

edge9881:                                         ; preds = %edge9878
  %v111858 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v107108)
  %t6395.not = icmp eq i64 %v111858, 0
  br i1 %t6395.not, label %edge9886, label %block60005

edge9886:                                         ; preds = %edge9878, %edge9881
  %v111862 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v111870)
  %t6396.not = icmp eq i64 %v111862, 0
  br i1 %t6396.not, label %edge8362, label %block60005

edge9910:                                         ; preds = %edge9868, %edge9864
  %t6399 = icmp samesign ult i64 %v111876, %v218975
  br i1 %t6399, label %edge9929, label %edge10091

edge9910.thread:                                  ; preds = %block4939
  %t6399954 = icmp samesign ult i64 %v111876, %v218975
  br i1 %t6399954, label %edge9929, label %edge10104

edge9929:                                         ; preds = %edge9910.thread, %edge9910
  %v112561 = add i64 %v111876, %v107156
  %t6405 = inttoptr i64 %v112561 to ptr
  %t6406 = load i8, ptr %t6405, align 1
  %v112562 = zext i8 %t6406 to i64
  %t6407 = icmp eq i8 %t6406, 0
  br i1 %t6407, label %edge10068, label %edge9955

edge9955:                                         ; preds = %edge9929
  %t6410 = icmp ult i8 %t6406, 17
  br i1 %t6410, label %edge9971, label %block61001

edge9971:                                         ; preds = %edge9955
  %v113233 = shl nuw nsw i64 %v112562, 2
  %v113234 = add i64 %v113233, %v111884
  %t6413 = inttoptr i64 %v113234 to ptr
  %t6414 = load i32, ptr %t6413, align 1
  %v113235 = zext i32 %t6414 to i64
  %v113298 = add i32 %t6414, 1
  %t6415.not = icmp eq i32 %t6414, -1
  br i1 %t6415.not, label %common.ret, label %edge9979

edge9979:                                         ; preds = %edge9971
  %t6425.not = icmp eq i64 %v111870, 0
  br i1 %t6425.not, label %block61006, label %edge9980

edge9980:                                         ; preds = %edge9979
  %t6426 = inttoptr i64 %v111870 to ptr
  %v113331 = load i64, ptr %t6426, align 1
  %t6427 = icmp eq i64 %v113331, 1
  br i1 %t6427, label %block61004, label %block61006

edge9985:                                         ; preds = %block61006
  %v113341 = load i64, ptr %ss1, align 8
  %v113342 = load i64, ptr %t606, align 8
  %v113343 = load i64, ptr %t607, align 8
  %.pre21748 = add i64 %v113341, %v113233
  %.pre21749 = inttoptr i64 %.pre21748 to ptr
  br label %block61004

edge9988:                                         ; preds = %block61004
  %t6451 = inttoptr i64 %v113330 to ptr
  %v113348 = load i64, ptr %t6451, align 1
  %v113349 = add i64 %v113348, 1
  store i64 %v113349, ptr %t6451, align 1
  br label %block61010

edge9993:                                         ; preds = %block61010
  %v113354 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v111870)
  %t6455.not = icmp eq i64 %v113354, 0
  br i1 %t6455.not, label %edge9998, label %block61001

edge9998:                                         ; preds = %block61010, %edge9993
  %v113367 = and i64 %v113235, 255
  %v113690 = add i64 %v113367, ptrtoint (ptr @mech_compact_table_19 to i64)
  %t6459 = inttoptr i64 %v113690 to ptr
  %t6460 = load i8, ptr %t6459, align 1
  %v113691 = zext i8 %t6460 to i64
  %t6462 = icmp ult i32 %t6414, 65536
  br i1 %t6462, label %edge10035, label %edge10047

edge10035:                                        ; preds = %edge9998
  %v113768 = lshr i64 %v113235, 8
  %v114091 = add i64 %v113768, ptrtoint (ptr @mech_compact_table_19 to i64)
  %t6465 = inttoptr i64 %v114091 to ptr
  %t6466 = load i8, ptr %t6465, align 1
  %v114092 = zext i8 %t6466 to i64
  br label %edge10047

edge10047:                                        ; preds = %edge9998, %edge10035
  %v114112 = phi i64 [ %v114092, %edge10035 ], [ 0, %edge9998 ]
  %v114122 = shl nuw nsw i64 %v113691, 8
  %v114127 = or disjoint i64 %v114112, %v114122
  %v114165 = sub nuw nsw i64 16, %v112562
  %v114171 = lshr i64 %v114127, %v114165
  %v114193 = shl nuw nsw i64 %v112562, 16
  %v114198 = or disjoint i64 %v114171, %v114193
  br label %edge10068

edge10068:                                        ; preds = %edge9929, %edge10047
  %v219022 = phi i64 [ %v111884, %edge9929 ], [ %v113325, %edge10047 ]
  %v219023 = phi i64 [ %v111874, %edge9929 ], [ %v113327, %edge10047 ]
  %v219024 = phi i64 [ %v111870, %edge9929 ], [ %v113330, %edge10047 ]
  %v219025 = phi i64 [ 0, %edge9929 ], [ %v114198, %edge10047 ]
  %v114249 = shl nuw nsw i64 %v111876, 2
  %v114250 = add i64 %v111877, %v114249
  %t6476 = inttoptr i64 %v114250 to ptr
  %t6477 = load i32, ptr %t6476, align 1
  %v114251 = zext i32 %t6477 to i64
  %t6478 = icmp eq i64 %v219025, %v114251
  br i1 %t6478, label %block61538, label %edge10071

edge10071:                                        ; preds = %edge10068
  br i1 %t6322.not, label %block61540, label %edge10072

edge10072:                                        ; preds = %edge10071
  %t6481 = inttoptr i64 %v111836 to ptr
  %v114267 = load i64, ptr %t6481, align 1
  %t6482 = icmp eq i64 %v114267, 1
  br i1 %t6482, label %block61539, label %block61540

edge10077:                                        ; preds = %block61540
  %v114277 = load i64, ptr %ss1, align 8
  %v114278 = load i64, ptr %t606, align 8
  %v114279 = load i64, ptr %t607, align 8
  %.pre21746 = add i64 %v114277, %v114249
  %.pre21747 = inttoptr i64 %.pre21746 to ptr
  br label %block61539

edge10080:                                        ; preds = %block61538
  %t6506 = inttoptr i64 %v114260 to ptr
  %v114284 = load i64, ptr %t6506, align 1
  %v114285 = add i64 %v114284, 1
  store i64 %v114285, ptr %t6506, align 1
  br label %edge10103

edge10091:                                        ; preds = %edge9910
  %v114319 = add i64 %v111837, 1
  store i64 %v114319, ptr %t6323, align 1
  br label %edge10104

edge10103:                                        ; preds = %block61538, %edge10080
  %t6525.not = or i1 %t6322.not, %t6503
  br i1 %t6525.not, label %edge10135, label %edge10128

edge10104:                                        ; preds = %edge9910.thread, %edge10091
  %t6516.not = icmp eq i64 %v107108, 0
  br i1 %t6516.not, label %edge10111, label %edge10107

edge10107:                                        ; preds = %edge10104
  %v114366 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v107108)
  %t6517.not = icmp eq i64 %v114366, 0
  br i1 %t6517.not, label %edge10111, label %block61591

edge10111:                                        ; preds = %edge10107, %edge10104
  %t6518.not = icmp eq i64 %v111836, 0
  br i1 %t6518.not, label %block61604, label %edge10114

edge10114:                                        ; preds = %edge10111
  %v114374 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v111836)
  %t6519.not = icmp eq i64 %v114374, 0
  br i1 %t6519.not, label %block61604, label %block61591

edge10118:                                        ; preds = %block61604
  %t6521 = inttoptr i64 %v111870 to ptr
  %v114376 = load i64, ptr %t6521, align 1
  %v114377 = add i64 %v114376, -1
  store i64 %v114377, ptr %t6521, align 1
  %t6523.not = icmp eq i64 %v114377, 0
  br i1 %t6523.not, label %edge10121, label %edge8362

edge10121:                                        ; preds = %edge10118
  %v114378 = call i64 @mech_compact_free(i64 %v0, i64 %v111870)
  %t6524.not = icmp eq i64 %v114378, 0
  br i1 %t6524.not, label %edge8362, label %block61591

edge10128:                                        ; preds = %edge10103
  %v114382 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v111836)
  %t6526.not = icmp eq i64 %v114382, 0
  br i1 %t6526.not, label %edge10135, label %block61591

edge10135:                                        ; preds = %edge10103, %edge10128
  %v114645 = add nuw nsw i64 %v111876, 1
  br label %block4939

edge10173:                                        ; preds = %edge8362
  %t6533.not = icmp eq i64 %v115052, 0
  br i1 %t6533.not, label %edge10240, label %edge10174

edge10174:                                        ; preds = %edge10173
  %t6534 = inttoptr i64 %v115052 to ptr
  %v115053 = load i64, ptr %t6534, align 1
  %v115054 = add i64 %v115053, 1
  store i64 %v115054, ptr %t6534, align 1
  br label %edge10240

edge10180:                                        ; preds = %edge8362
  br i1 %t4891, label %edge10185, label %edge10231

edge10185:                                        ; preds = %edge10180
  %t6538.not = icmp eq i64 %v115052, 0
  br i1 %t6538.not, label %edge10189, label %edge10186

edge10186:                                        ; preds = %edge10185
  %t6539 = inttoptr i64 %v115052 to ptr
  %v115351 = load i64, ptr %t6539, align 1
  %v115352 = add i64 %v115351, 1
  store i64 %v115352, ptr %t6539, align 1
  br label %edge10189

edge10189:                                        ; preds = %edge10185, %edge10186
  %t6543 = call i64 @llvm.umin.i64(i64 %v218975, i64 32)
  %t6544 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v218960, i64 %t6543)
  %v115484 = extractvalue { i64, i1 } %t6544, 0
  %t6545 = extractvalue { i64, i1 } %t6544, 1
  br i1 %t6545, label %block62320, label %edge10192

edge10192:                                        ; preds = %edge10189
  %t6548.not = icmp ugt i64 %v115484, %v218961
  br i1 %t6548.not, label %edge10195, label %edge10212

edge10195:                                        ; preds = %edge10192
  %t6550.not = icmp eq i64 %v218961, 0
  br i1 %t6550.not, label %block62328, label %edge10196

edge10196:                                        ; preds = %edge10195
  %63 = add i64 %v218961, -32769
  %v115494204 = icmp ult i64 %63, -31745
  br i1 %v115494204, label %edge10199, label %edge10208

edge10199:                                        ; preds = %edge10196
  %t6554 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v218961, i64 %v218961)
  %v115495 = extractvalue { i64, i1 } %t6554, 0
  %t6555 = extractvalue { i64, i1 } %t6554, 1
  br i1 %t6555, label %block62320, label %block62328

edge10208:                                        ; preds = %edge10196
  %t6562 = mul nuw nsw i64 %v218961, 3
  %t6566 = add nuw nsw i64 %t6562, 1
  %v115512 = lshr i64 %t6566, 1
  br label %block62328

edge10212:                                        ; preds = %edge10192, %block62328
  %v115489 = phi i64 [ %t6570, %block62328 ], [ %v218961, %edge10192 ]
  %t6571 = zext i64 %v218959 to i128
  %t6572 = zext i64 %v218960 to i128
  %t6573 = shl nuw i128 %t6572, 64
  %v220199 = or disjoint i128 %t6573, %t6571
  store i128 %v220199, ptr %ss0, align 16
  %t6575 = zext i64 %v218961 to i128
  %v220201 = or disjoint i128 %t6575, 18446744073709551616
  store i128 %v220201, ptr %t1002, align 16
  %t6580 = zext i64 %v218963 to i128
  %t6581 = zext i64 %v218964 to i128
  %t6582 = shl nuw i128 %t6581, 64
  %v220203 = or disjoint i128 %t6582, %t6580
  store i128 %v220203, ptr %t1007, align 16
  %v115525 = call i64 @mech_compact_writable(i64 %v0, i64 %v219588, i64 0, i64 %v115489, i64 4, i64 4, i64 %v3194)
  %t6585.not = icmp eq i64 %v115525, 0
  br i1 %t6585.not, label %edge10214, label %block62320

edge10214:                                        ; preds = %edge10212
  %v115526 = load i64, ptr %ss1, align 8
  %v115527 = load i64, ptr %t606, align 8
  %v115528 = load i64, ptr %t607, align 8
  %v115533 = shl nuw nsw i64 %t6543, 2
  %v115529 = shl i64 %v218960, 2
  %v115530 = add i64 %v115526, %v115529
  %v115534 = call i64 @memmove(i64 %v115530, i64 %v115047, i64 %v115533)
  %t6589.not = icmp eq i64 %v115528, 0
  br i1 %t6589.not, label %block62341, label %edge10219

edge10219:                                        ; preds = %edge10214
  %t6590 = inttoptr i64 %v115528 to ptr
  %v115535 = load i64, ptr %t6590, align 1
  %v115536 = add i64 %v115535, 1
  store i64 %v115536, ptr %t6590, align 1
  br label %block62341

edge10224:                                        ; preds = %block62341
  %v115540 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v115052)
  %t6593.not = icmp eq i64 %v115540, 0
  br i1 %t6593.not, label %edge10245, label %block62320

edge10231:                                        ; preds = %edge10180
  %t6596.not = icmp eq i64 %v218964, 0
  br i1 %t6596.not, label %edge10900, label %edge10232

edge10232:                                        ; preds = %edge10231
  %t6597 = inttoptr i64 %v218964 to ptr
  %v115579 = load i64, ptr %t6597, align 1
  %v115580 = add i64 %v115579, 1
  store i64 %v115580, ptr %t6597, align 1
  br label %edge10900

edge10240:                                        ; preds = %edge10174, %edge10173
  br i1 %t4891, label %edge10245, label %edge10900

edge10243:                                        ; preds = %edge10904, %block67874, %edge10846, %edge10843, %block67551
  %v116180984 = phi i64 [ %v116180983, %edge10846 ], [ %v116180983, %block67551 ], [ %v116180983, %edge10843 ], [ %v116180996, %block67874 ], [ %v116180996, %edge10904 ]
  %v116179981 = phi i64 [ %v116179982, %edge10846 ], [ %v116179982, %block67551 ], [ %v116179982, %edge10843 ], [ %v116179995, %block67874 ], [ %v116179995, %edge10904 ]
  %v116177979 = phi i64 [ %v116177980, %edge10846 ], [ %v116177980, %block67551 ], [ %v116177980, %edge10843 ], [ %v116177994, %block67874 ], [ %v116177994, %edge10904 ]
  %v116176978 = phi i64 [ %v116179982, %edge10846 ], [ %v116179982, %block67551 ], [ %v116179982, %edge10843 ], [ %v116176993, %block67874 ], [ %v116176993, %edge10904 ]
  %v116175976 = phi i64 [ %v116175975, %edge10846 ], [ %v116175975, %block67551 ], [ %v116175975, %edge10843 ], [ %v116175992, %block67874 ], [ %v116175992, %edge10904 ]
  %v127204 = phi i64 [ %v120626, %edge10846 ], [ %v120626, %block67551 ], [ %v120626, %edge10843 ], [ %v218965, %block67874 ], [ %v218965, %edge10904 ]
  %v127205 = phi i64 [ 19, %edge10846 ], [ 19, %block67551 ], [ 19, %edge10843 ], [ %v218966, %block67874 ], [ %v218966, %edge10904 ]
  %v127209 = phi i64 [ %v120631, %edge10846 ], [ %v120631, %block67551 ], [ %v120631, %edge10843 ], [ %v218967, %block67874 ], [ %v218967, %edge10904 ]
  %v127306 = phi i64 [ %v120643, %edge10846 ], [ 0, %block67551 ], [ %v120643, %edge10843 ], [ 0, %block67874 ], [ %v218969, %edge10904 ]
  %v127619 = phi i64 [ %v120638, %edge10846 ], [ %v120638, %block67551 ], [ %v120638, %edge10843 ], [ %v218968, %block67874 ], [ %v218968, %edge10904 ]
  %v127630 = phi i64 [ %v120650, %edge10846 ], [ %v120650, %block67551 ], [ %v120650, %edge10843 ], [ %v218970, %block67874 ], [ %v218970, %edge10904 ]
  %v127633 = phi i64 [ %v116269.lcssa, %edge10846 ], [ %v116269.lcssa, %block67551 ], [ %v116269.lcssa, %edge10843 ], [ %v218971, %block67874 ], [ %v218971, %edge10904 ]
  %v127636 = phi i64 [ %v118448, %edge10846 ], [ %v118448, %block67551 ], [ %v118448, %edge10843 ], [ %v218972, %block67874 ], [ %v218972, %edge10904 ]
  br i1 %t6530, label %edge10912, label %edge10917

edge10245:                                        ; preds = %edge10224, %block62341, %edge10240
  %v116180983 = phi i64 [ %v115052, %edge10240 ], [ %v115528, %block62341 ], [ %v115528, %edge10224 ]
  %v116179982 = phi i64 [ %v218975, %edge10240 ], [ %v115484, %block62341 ], [ %v115484, %edge10224 ]
  %v116177980 = phi i64 [ %v115049, %edge10240 ], [ %v115527, %block62341 ], [ %v115527, %edge10224 ]
  %v116175975 = phi i64 [ %v115047, %edge10240 ], [ %v115526, %block62341 ], [ %v115526, %edge10224 ]
  %t6601.not = icmp eq i64 %v116180983, 0
  br i1 %t6601.not, label %edge10251, label %edge10246

edge10246:                                        ; preds = %edge10245
  %t6602 = inttoptr i64 %v116180983 to ptr
  %v116225 = load i64, ptr %t6602, align 1
  %v116226 = add i64 %v116225, 1
  store i64 %v116226, ptr %t6602, align 1
  br label %edge10251

edge10251:                                        ; preds = %edge10245, %edge10246
  %v116881 = add i64 %v116175975, -4
  br label %block5118

edge10263:                                        ; preds = %block5118
  %v116564 = add nsw i64 %v116269, -1
  %t6607 = icmp ult i64 %v116564, %v116179982
  br i1 %t6607, label %edge10282, label %block5118.backedge

block5118.backedge:                               ; preds = %edge10263, %edge10282
  br label %block5118

edge10282:                                        ; preds = %edge10263
  %v116878 = shl i64 %v116269, 2
  %v116882 = add i64 %v116881, %v116878
  %t6610 = inttoptr i64 %v116882 to ptr
  %t6611 = load i32, ptr %t6610, align 1
  %64 = icmp ugt i32 %t6611, 65535
  br i1 %64, label %block63058, label %block5118.backedge

edge10298:                                        ; preds = %block63058
  %t6617 = inttoptr i64 %v116180983 to ptr
  %v116950 = load i64, ptr %t6617, align 1
  %v116951 = add i64 %v116950, -1
  store i64 %v116951, ptr %t6617, align 1
  %t6619.not = icmp eq i64 %v116951, 0
  br i1 %t6619.not, label %edge10301, label %edge10317

edge10301:                                        ; preds = %edge10298
  %v116952 = call i64 @mech_compact_free(i64 %v0, i64 %v116180983)
  %t6620.not = icmp eq i64 %v116952, 0
  br i1 %t6620.not, label %edge10301.edge10317_crit_edge, label %edge10302

edge10301.edge10317_crit_edge:                    ; preds = %edge10301
  %v117419.pre = load i64, ptr %t6617, align 1
  %65 = add i64 %v117419.pre, 1
  br label %edge10317

edge10302:                                        ; preds = %edge10301
  %t6621 = insertvalue { i64, i64 } poison, i64 %v116952, 0
  %t6622 = insertvalue { i64, i64 } %t6621, i64 5241, 1
  br label %common.ret

edge10317:                                        ; preds = %edge10301.edge10317_crit_edge, %edge10298
  %v117419 = phi i64 [ %65, %edge10301.edge10317_crit_edge ], [ %v116950, %edge10298 ]
  store i64 %v117419, ptr %t6617, align 1
  br label %block5173.preheader

edge10334:                                        ; preds = %block5173
  %v117757 = add nsw i64 %v117463, -1
  %t6629 = icmp ult i64 %v117757, %v116179982
  br i1 %t6629, label %edge10353, label %block5173.backedge

block5173.backedge:                               ; preds = %edge10334, %edge10353
  br label %block5173

edge10353:                                        ; preds = %edge10334
  %v118071 = shl i64 %v117463, 2
  %v118075 = add i64 %v116881, %v118071
  %t6632 = inttoptr i64 %v118075 to ptr
  %t6633 = load i32, ptr %t6632, align 1
  %66 = icmp ugt i32 %t6633, 65535
  br i1 %66, label %block63767, label %block5173.backedge

edge10369:                                        ; preds = %block63767
  %t6639 = inttoptr i64 %v116180983 to ptr
  %v118141 = load i64, ptr %t6639, align 1
  %v118142 = add i64 %v118141, -1
  store i64 %v118142, ptr %t6639, align 1
  %t6641.not = icmp eq i64 %v118142, 0
  br i1 %t6641.not, label %edge10372, label %edge10388

edge10372:                                        ; preds = %edge10369
  %v118143 = call i64 @mech_compact_free(i64 %v0, i64 %v116180983)
  %t6642.not = icmp eq i64 %v118143, 0
  br i1 %t6642.not, label %edge10388, label %edge10373

edge10373:                                        ; preds = %edge10372
  %t6643 = insertvalue { i64, i64 } poison, i64 %v118143, 0
  %t6644 = insertvalue { i64, i64 } %t6643, i64 5298, 1
  br label %common.ret

edge10388:                                        ; preds = %edge10372, %edge10369, %block63767
  %v118448 = add i64 %v117463.lcssa, -288
  %v118454 = add i64 %v118448, %v116269.lcssa
  %t6649.not = icmp ult i64 %v118454, %v116269.lcssa
  br i1 %t6649.not, label %common.ret, label %edge10425

edge10405:                                        ; preds = %edge10428
  %t6660 = insertvalue { i64, i64 } poison, i64 %v119289, 0
  %t6661 = insertvalue { i64, i64 } %t6660, i64 5313, 1
  br label %common.ret

edge10425:                                        ; preds = %edge10388
  %v119360 = call i64 @mech_compact_allocate(i64 %v0, i64 19, i64 4, i64 4, i64 1, i64 %v3194)
  %t6674.not = icmp eq i64 %v119360, 0
  br i1 %t6674.not, label %edge10427, label %block64016

edge10427:                                        ; preds = %edge10425
  %v119363 = load i64, ptr %ss1, align 8
  %v119364 = load i64, ptr %t606, align 8
  %v119365 = load i64, ptr %t607, align 8
  %t6677.not = icmp eq i64 %v119365, 0
  br i1 %t6677.not, label %edge10465, label %edge10428

edge10428:                                        ; preds = %edge10427
  %t6678 = inttoptr i64 %v119365 to ptr
  %v119366 = load i64, ptr %t6678, align 1
  %v119285 = add i64 %v119366, 2
  store i64 %v119285, ptr %t6678, align 1
  %v119289 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v119365)
  %t6659.not = icmp eq i64 %v119289, 0
  br i1 %t6659.not, label %edge10465, label %edge10405

edge10445:                                        ; preds = %edge10468
  %t6689 = insertvalue { i64, i64 } poison, i64 %v120323, 0
  %t6690 = insertvalue { i64, i64 } %t6689, i64 5326, 1
  br label %common.ret

edge10465:                                        ; preds = %edge10428, %edge10427
  %v120374 = call i64 @mech_compact_allocate(i64 %v0, i64 320, i64 4, i64 4, i64 1, i64 %v3194)
  %t6703.not = icmp eq i64 %v120374, 0
  br i1 %t6703.not, label %edge10467, label %block64071

edge10467:                                        ; preds = %edge10465
  %v120377 = load i64, ptr %ss1, align 8
  %v120378 = load i64, ptr %t606, align 8
  %v120379 = load i64, ptr %t607, align 8
  %t6706.not = icmp eq i64 %v120379, 0
  br i1 %t6706.not, label %block5254, label %edge10468

edge10468:                                        ; preds = %edge10467
  %t6707 = inttoptr i64 %v120379 to ptr
  %v120380 = load i64, ptr %t6707, align 1
  %v120319 = add i64 %v120380, 2
  store i64 %v120319, ptr %t6707, align 1
  %v120323 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v120379)
  %t6688.not = icmp eq i64 %v120323, 0
  br i1 %t6688.not, label %block5254, label %edge10445

edge10474:                                        ; preds = %block5254
  %t6712 = inttoptr i64 %v116180983 to ptr
  %v120552 = load i64, ptr %t6712, align 1
  %v120553 = add i64 %v120552, 1
  store i64 %v120553, ptr %t6712, align 1
  br label %edge10483

edge10483:                                        ; preds = %block5254, %edge10474
  %t6734.pre-phi = phi ptr [ %t6712, %edge10474 ], [ null, %block5254 ]
  %v121553 = sub i64 288, %v116269.lcssa
  br label %block5256

edge10509:                                        ; preds = %block5256
  %t6717 = icmp ne i64 %v120653, %v120656
  br i1 %t6717, label %block5388, label %edge10514

edge10514:                                        ; preds = %edge10509
  %t6720 = icmp ult i64 %v120653, %v116269.lcssa
  %v121558 = select i1 %t6720, i64 0, i64 %v121553
  %v121563 = add i64 %v121558, %v120653
  %t6723 = icmp ult i64 %v121563, %v116179982
  br i1 %t6723, label %edge10541, label %edge10550

edge10541:                                        ; preds = %edge10514
  %v121893 = shl i64 %v121563, 2
  %v121894 = add i64 %v121893, %v116175975
  %t6726 = inttoptr i64 %v121894 to ptr
  %t6727 = load i32, ptr %t6726, align 1
  %67 = lshr i32 %t6727, 16
  %68 = and i32 %67, 255
  %69 = zext nneg i32 %68 to i64
  br label %edge10550

edge10550:                                        ; preds = %edge10541, %edge10514
  %v121916 = phi i64 [ %69, %edge10541 ], [ 0, %edge10514 ]
  %t6729.not = icmp eq i64 %v120653, -1
  br i1 %t6729.not, label %common.ret, label %edge10561

edge10561:                                        ; preds = %edge10550
  br i1 %t6601.not, label %block5325.preheader, label %edge10562

edge10562:                                        ; preds = %edge10561
  %v122256 = load i64, ptr %t6734.pre-phi, align 1
  %v122257 = add i64 %v122256, 1
  store i64 %v122257, ptr %t6734.pre-phi, align 1
  br label %block5325.preheader

block5325.preheader:                              ; preds = %edge10561, %edge10562
  br label %block5325

edge10607:                                        ; preds = %block5325
  %v122971 = shl i64 %v122640, 2
  %v122972 = add i64 %v122971, %v116175975
  %t6743 = inttoptr i64 %v122972 to ptr
  %t6744 = load i32, ptr %t6743, align 1
  %70 = lshr i32 %t6744, 16
  %71 = and i32 %70, 255
  %72 = zext nneg i32 %71 to i64
  br label %edge10616

edge10616:                                        ; preds = %edge10607, %block5325
  %v122994 = phi i64 [ %72, %edge10607 ], [ 0, %block5325 ]
  %t6746 = icmp eq i64 %v121916, %v122994
  %v123028207 = and i1 %t6736, %t6746
  br i1 %v123028207, label %edge10635, label %edge10624

edge10624:                                        ; preds = %edge10616
  br i1 %t6601.not, label %block5388, label %edge10625

edge10625:                                        ; preds = %edge10624
  %v123035 = load i64, ptr %t6734.pre-phi, align 1
  %v123036 = add i64 %v123035, -1
  store i64 %v123036, ptr %t6734.pre-phi, align 1
  %t6752.not = icmp eq i64 %v123036, 0
  br i1 %t6752.not, label %edge10628, label %block5388

edge10628:                                        ; preds = %edge10625
  %v123037 = call i64 @mech_compact_free(i64 %v0, i64 %v116180983)
  %t6753.not = icmp eq i64 %v123037, 0
  br i1 %t6753.not, label %block5388, label %edge10629

edge10629:                                        ; preds = %edge10628
  %t6754 = insertvalue { i64, i64 } poison, i64 %v123037, 0
  %t6755 = insertvalue { i64, i64 } %t6754, i64 5460, 1
  br label %common.ret

edge10635:                                        ; preds = %edge10616
  %t6756.not = icmp eq i64 %v122309, -1
  br i1 %t6756.not, label %common.ret, label %block5325

edge10648:                                        ; preds = %block5388
  %v123371 = sub nuw i64 %v219030, %v120653
  %t6764 = icmp eq i64 %v219032, 0
  br i1 %t6764, label %edge10657, label %edge10680

edge10657:                                        ; preds = %edge10648
  %t6767 = icmp ugt i64 %v123371, 10
  br i1 %t6767, label %edge10697.thread, label %edge10666

edge10666:                                        ; preds = %edge10657
  %t6770 = icmp samesign ugt i64 %v123371, 2
  %v124777 = select i1 %t6770, i64 17, i64 0
  %v124778 = select i1 %t6770, i64 3, i64 1
  %v124779 = select i1 %t6770, i64 10, i64 1
  br label %edge10697

edge10680:                                        ; preds = %edge10648
  %t6773 = icmp ugt i64 %v123371, 2
  %v124811208 = and i1 %t6717, %t6773
  %v125170 = select i1 %v124811208, i64 16, i64 %v219032
  %v125171 = select i1 %v124811208, i64 3, i64 1
  %v125172 = select i1 %v124811208, i64 6, i64 1
  br label %edge10697

edge10697:                                        ; preds = %edge10666, %edge10680
  %v125175 = phi i64 [ %v125170, %edge10680 ], [ %v124777, %edge10666 ]
  %v125178 = phi i64 [ %v125171, %edge10680 ], [ %v124778, %edge10666 ]
  %v125181 = phi i64 [ %v125172, %edge10680 ], [ %v124779, %edge10666 ]
  %spec.select32 = call i64 @llvm.umin.i64(i64 %v123371, i64 %v125181)
  %t6779.not = icmp samesign ult i64 %spec.select32, %v125178
  br i1 %t6779.not, label %common.ret, label %edge10707

edge10697.thread:                                 ; preds = %edge10657
  %spec.select321000 = call i64 @llvm.umin.i64(i64 %v123371, i64 138)
  %v1254901008 = add nsw i64 %spec.select321000, -11
  br label %edge10737

edge10707:                                        ; preds = %edge10697
  %v125490 = sub nuw nsw i64 %spec.select32, %v125178
  %t6783 = icmp ult i64 %v125175, 19
  br i1 %t6783, label %edge10737, label %block67436

edge10737:                                        ; preds = %edge10707, %edge10697.thread
  %v1254901015 = phi i64 [ %v1254901008, %edge10697.thread ], [ %v125490, %edge10707 ]
  %v12517510021012 = phi i64 [ 18, %edge10697.thread ], [ %v125175, %edge10707 ]
  %spec.select3210041011 = phi i64 [ %spec.select321000, %edge10697.thread ], [ %spec.select32, %edge10707 ]
  %v125837 = shl nuw nsw i64 %v12517510021012, 2
  %v125838 = add i64 %v125837, %v120626
  %t6786 = inttoptr i64 %v125838 to ptr
  %t6787 = load i32, ptr %t6786, align 1
  %v125922 = add i32 %t6787, 1
  %t6788.not = icmp eq i32 %t6787, -1
  br i1 %t6788.not, label %common.ret, label %edge10745

edge10745:                                        ; preds = %edge10737
  %t6798.not = icmp eq i64 %v120631, 0
  br i1 %t6798.not, label %block67441, label %edge10746

edge10746:                                        ; preds = %edge10745
  %t6799 = inttoptr i64 %v120631 to ptr
  %v125908 = load i64, ptr %t6799, align 1
  %t6800 = icmp eq i64 %v125908, 1
  br i1 %t6800, label %block67439, label %block67441

edge10751:                                        ; preds = %block67441
  %v125918 = load i64, ptr %ss1, align 8
  %v125919 = load i64, ptr %t606, align 8
  %v125920 = load i64, ptr %t607, align 8
  %.pre21733 = add i64 %v125918, %v125837
  %.pre21734 = inttoptr i64 %.pre21733 to ptr
  br label %block67439

edge10754:                                        ; preds = %block67439
  %t6824 = inttoptr i64 %v125907 to ptr
  %v125925 = load i64, ptr %t6824, align 1
  %v125926 = add i64 %v125925, 1
  store i64 %v125926, ptr %t6824, align 1
  br label %block67445

edge10768:                                        ; preds = %block67445
  %v126057 = shl nuw nsw i64 %v120650, 2
  %v126058 = add i64 %v126057, %v120638
  %t6832 = inttoptr i64 %v126058 to ptr
  %t6833 = load i32, ptr %t6832, align 1
  %v126059 = zext i32 %t6833 to i64
  %t6834 = icmp eq i64 %v126024, %v126059
  br i1 %t6834, label %block67459, label %edge10771

edge10771:                                        ; preds = %edge10768
  %t6836.not = icmp eq i64 %v125927, 0
  br i1 %t6836.not, label %block67462, label %edge10774

edge10774:                                        ; preds = %edge10771
  %v126072 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v125927)
  %t6837.not = icmp eq i64 %v126072, 0
  br i1 %t6837.not, label %block67462, label %block67456

edge10778:                                        ; preds = %block67462
  %t6839 = inttoptr i64 %v120643 to ptr
  %v126079 = load i64, ptr %t6839, align 1
  %t6840 = icmp eq i64 %v126079, 1
  br i1 %t6840, label %block67464, label %block67465

edge10783:                                        ; preds = %block67465
  %v126089 = load i64, ptr %ss1, align 8
  %v126090 = load i64, ptr %t606, align 8
  %v126091 = load i64, ptr %t607, align 8
  %.pre21731 = add i64 %v126089, %v126057
  %.pre21732 = inttoptr i64 %.pre21731 to ptr
  br label %block67464

edge10786:                                        ; preds = %block67459
  %t6864 = inttoptr i64 %v126068 to ptr
  %v126096 = load i64, ptr %t6864, align 1
  %v126097 = add i64 %v126096, 1
  store i64 %v126097, ptr %t6864, align 1
  br label %edge10800

edge10800:                                        ; preds = %edge10786, %block67459
  %v126098 = select i1 %t6861, i64 0, i64 %v120643
  %v126134 = add i64 %spec.select3210041011, %v120653
  %t6873.not = icmp ult i64 %v126134, %v120653
  br i1 %t6873.not, label %common.ret, label %edge10829

edge10810:                                        ; preds = %block5256
  %t6877.not = icmp eq i64 %v120631, 0
  br i1 %t6877.not, label %block67516, label %edge10811

edge10811:                                        ; preds = %edge10810
  %t6878 = inttoptr i64 %v120631 to ptr
  %v126168 = load i64, ptr %t6878, align 1
  %v126169 = add i64 %v126168, 1
  store i64 %v126169, ptr %t6878, align 1
  br label %block67516

edge10814:                                        ; preds = %block67516
  %t6881 = inttoptr i64 %v120643 to ptr
  %v126170 = load i64, ptr %t6881, align 1
  %v126171 = add i64 %v126170, 1
  store i64 %v126171, ptr %t6881, align 1
  br label %edge10830

edge10829:                                        ; preds = %edge10800
  %t6894.not = icmp eq i64 %v126252, 0
  br i1 %t6894.not, label %block67559, label %edge10853

edge10830:                                        ; preds = %block67516, %edge10814
  br i1 %t6601.not, label %block67547, label %edge10833

edge10833:                                        ; preds = %edge10830
  %v126230 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v116180983)
  %t6886.not = icmp eq i64 %v126230, 0
  br i1 %t6886.not, label %block67547, label %block67542

edge10839:                                        ; preds = %block67547
  %v126234 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v120631)
  %t6888.not = icmp eq i64 %v126234, 0
  br i1 %t6888.not, label %block67551, label %block67542

edge10843:                                        ; preds = %block67551
  %t6890 = inttoptr i64 %v120643 to ptr
  %v126236 = load i64, ptr %t6890, align 1
  %v126237 = add i64 %v126236, -1
  store i64 %v126237, ptr %t6890, align 1
  %t6892.not = icmp eq i64 %v126237, 0
  br i1 %t6892.not, label %edge10846, label %edge10243

edge10846:                                        ; preds = %edge10843
  %v126238 = call i64 @mech_compact_free(i64 %v0, i64 %v120643)
  %t6893.not = icmp eq i64 %v126238, 0
  br i1 %t6893.not, label %edge10243, label %block67542

edge10853:                                        ; preds = %edge10829
  %v126242 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v126252)
  %t6895.not = icmp eq i64 %v126242, 0
  br i1 %t6895.not, label %block67559, label %block67542

edge10857:                                        ; preds = %block67559
  %t6897 = inttoptr i64 %v126098 to ptr
  %v126244 = load i64, ptr %t6897, align 1
  %v126245 = add i64 %v126244, -1
  store i64 %v126245, ptr %t6897, align 1
  %t6899.not = icmp eq i64 %v126245, 0
  br i1 %t6899.not, label %edge10860, label %block5256.backedge

edge10860:                                        ; preds = %edge10857
  %v126246 = call i64 @mech_compact_free(i64 %v0, i64 %v126098)
  %t6900.not = icmp eq i64 %v126246, 0
  br i1 %t6900.not, label %block5256.backedge, label %block67542

edge10900:                                        ; preds = %edge10231, %edge10232, %edge10240
  %v116180996 = phi i64 [ %v115052, %edge10240 ], [ %v218964, %edge10232 ], [ 0, %edge10231 ]
  %v116179995 = phi i64 [ %v218975, %edge10240 ], [ %v218963, %edge10232 ], [ %v218963, %edge10231 ]
  %v116177994 = phi i64 [ %v115049, %edge10240 ], [ %v218961, %edge10232 ], [ %v218961, %edge10231 ]
  %v116176993 = phi i64 [ %v218975, %edge10240 ], [ %v218960, %edge10232 ], [ %v218960, %edge10231 ]
  %v116175992 = phi i64 [ %v115047, %edge10240 ], [ %v218959, %edge10232 ], [ %v218959, %edge10231 ]
  %t6903.not = icmp eq i64 %v218967, 0
  br i1 %t6903.not, label %block67874, label %edge10901

edge10901:                                        ; preds = %edge10900
  %t6904 = inttoptr i64 %v218967 to ptr
  %v126710 = load i64, ptr %t6904, align 1
  %v126711 = add i64 %v126710, 1
  store i64 %v126711, ptr %t6904, align 1
  br label %block67874

edge10904:                                        ; preds = %block67874
  %t6907 = inttoptr i64 %v218969 to ptr
  %v126712 = load i64, ptr %t6907, align 1
  %v126713 = add i64 %v126712, 1
  store i64 %v126713, ptr %t6907, align 1
  br label %edge10243

edge10910:                                        ; preds = %edge10918, %edge10917
  %t6917 = icmp eq i64 %v218955, 1
  br i1 %t6917, label %edge10925, label %edge10927

edge10912:                                        ; preds = %edge10243
  br i1 %t4820.not, label %edge10925, label %edge10913

edge10913:                                        ; preds = %edge10912
  %v127191 = load i64, ptr %t6912.pre-phi, align 1
  %v127192 = add i64 %v127191, 1
  store i64 %v127192, ptr %t6912.pre-phi, align 1
  br label %edge10925

edge10917:                                        ; preds = %edge10243
  %t6914.not = icmp eq i64 %v127209, 0
  br i1 %t6914.not, label %edge10910, label %edge10918

edge10918:                                        ; preds = %edge10917
  %t6915 = inttoptr i64 %v127209 to ptr
  %v127210 = load i64, ptr %t6915, align 1
  %v127211 = add i64 %v127210, 1
  store i64 %v127211, ptr %t6915, align 1
  br label %edge10910

edge10925:                                        ; preds = %edge10912, %edge10913, %edge10910
  %v1276681060 = phi i64 [ %v127205, %edge10910 ], [ 320, %edge10913 ], [ 320, %edge10912 ]
  %v1276671059 = phi i64 [ %v127204, %edge10910 ], [ %v80928, %edge10913 ], [ %v80928, %edge10912 ]
  %v1273071058 = phi i64 [ %v127209, %edge10910 ], [ %v80922, %edge10913 ], [ 0, %edge10912 ]
  %t6931.not = icmp eq i64 %v218964, 0
  br i1 %t6931.not, label %block68129, label %edge10956

edge10927:                                        ; preds = %edge10910
  %t6920.not = icmp eq i64 %v116180984, 0
  br i1 %t6920.not, label %block68113, label %edge10930

edge10930:                                        ; preds = %edge10927
  %v127259 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v116180984)
  %t6921.not = icmp eq i64 %v127259, 0
  br i1 %t6921.not, label %block68113, label %block68104

edge10936:                                        ; preds = %block68113
  %v127263 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v127209)
  %t6923.not = icmp eq i64 %v127263, 0
  br i1 %t6923.not, label %block68117, label %block68104

edge10942:                                        ; preds = %block68117
  %v127267 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v127306)
  %t6925.not = icmp eq i64 %v127267, 0
  br i1 %t6925.not, label %block68121, label %block68104

edge10946:                                        ; preds = %block68121
  %t6927 = inttoptr i64 %v127209 to ptr
  %v127269 = load i64, ptr %t6927, align 1
  %v127270 = add i64 %v127269, -1
  store i64 %v127270, ptr %t6927, align 1
  %t6929.not = icmp eq i64 %v127270, 0
  br i1 %t6929.not, label %edge10949, label %edge11012

edge10949:                                        ; preds = %edge10946
  %v127271 = call i64 @mech_compact_free(i64 %v0, i64 %v127209)
  %t6930.not = icmp eq i64 %v127271, 0
  br i1 %t6930.not, label %edge11012, label %block68104

edge10956:                                        ; preds = %edge10925
  %v127275 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v218964)
  %t6932.not = icmp eq i64 %v127275, 0
  br i1 %t6932.not, label %block68129, label %block68104

edge10962:                                        ; preds = %block68129
  %v127279 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v115052)
  %t6934.not = icmp eq i64 %v127279, 0
  br i1 %t6934.not, label %block68133, label %block68104

edge10968:                                        ; preds = %block68133
  %v127283 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v218967)
  %t6936.not = icmp eq i64 %v127283, 0
  br i1 %t6936.not, label %block68137, label %block68104

edge10974:                                        ; preds = %block68137
  %v127287 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v218969)
  %t6938.not = icmp eq i64 %v127287, 0
  br i1 %t6938.not, label %edge10981, label %block68104

edge10981:                                        ; preds = %block68137, %edge10974
  %v127608 = add nuw nsw i64 %v218955, 1
  %v83567 = xor i64 %v218955, 1
  %spec.select33 = select i1 %t6530, i64 32, i64 19
  %spec.select34 = select i1 %t6530, i64 15, i64 7
  br label %block3784

edge11012:                                        ; preds = %block68121, %edge10946, %edge10949
  %t6942.not = icmp eq i64 %v115052, 0
  br i1 %t6942.not, label %block5547.preheader, label %edge11013

edge11013:                                        ; preds = %edge11012
  %t6943 = inttoptr i64 %v115052 to ptr
  %v130159 = load i64, ptr %t6943, align 1
  %v130160 = add i64 %v130159, 1
  store i64 %v130160, ptr %t6943, align 1
  br label %block5547.preheader

block5547.preheader:                              ; preds = %edge11012, %edge11013
  br label %block5547

edge11074:                                        ; preds = %block5547
  %v130557 = add nsw i64 %v130227, -1
  %v130865 = add i64 %v130227, add (i64 ptrtoint (ptr @mech_compact_table_15 to i64), i64 -1)
  %t6951 = inttoptr i64 %v130865 to ptr
  %t6952 = load i8, ptr %t6951, align 1
  %v130866 = zext i8 %t6952 to i64
  %t6953 = icmp samesign ugt i64 %v218975, %v130866
  br i1 %t6953, label %edge11082, label %block5547.backedge

block5547.backedge:                               ; preds = %edge11074, %edge11082
  br label %block5547

edge11082:                                        ; preds = %edge11074
  %v131474 = shl nuw nsw i64 %v130866, 2
  %v131475 = add i64 %v131474, %v115047
  %t6956 = inttoptr i64 %v131475 to ptr
  %t6957 = load i32, ptr %t6956, align 1
  %73 = icmp ugt i32 %t6957, 65535
  br i1 %73, label %block69347, label %block5547.backedge

edge11098:                                        ; preds = %block69347
  %t6963 = inttoptr i64 %v115052 to ptr
  %v131541 = load i64, ptr %t6963, align 1
  %v131542 = add i64 %v131541, -1
  store i64 %v131542, ptr %t6963, align 1
  %t6965.not = icmp eq i64 %v131542, 0
  br i1 %t6965.not, label %edge11101, label %edge11167

edge11101:                                        ; preds = %edge11098
  %v131543 = call i64 @mech_compact_free(i64 %v0, i64 %v115052)
  %t6966.not = icmp eq i64 %v131543, 0
  br i1 %t6966.not, label %edge11167, label %edge11102

edge11102:                                        ; preds = %edge11101
  %t6967 = insertvalue { i64, i64 } poison, i64 %v131543, 0
  %t6968 = insertvalue { i64, i64 } %t6967, i64 5708, 1
  br label %common.ret

edge11167:                                        ; preds = %edge11101, %edge11098, %block69347, %edge7450, %edge7447, %block43158, %edge7391, %edge7395
  %v132163 = phi i64 [ %v64802, %edge7450 ], [ %v64802, %edge7391 ], [ %v64802, %edge7395 ], [ %v64802, %block43158 ], [ %v64802, %edge7447 ], [ %v218959, %block69347 ], [ %v218959, %edge11098 ], [ %v218959, %edge11101 ]
  %v132164 = phi i64 [ %v64803, %edge7450 ], [ %v64803, %edge7391 ], [ %v64803, %edge7395 ], [ %v64803, %block43158 ], [ %v64803, %edge7447 ], [ %v218960, %block69347 ], [ %v218960, %edge11098 ], [ %v218960, %edge11101 ]
  %v132167 = phi i64 [ %v64806, %edge7450 ], [ %v64806, %edge7391 ], [ %v64806, %edge7395 ], [ %v64806, %block43158 ], [ %v64806, %edge7447 ], [ %v218963, %block69347 ], [ %v218963, %edge11098 ], [ %v218963, %edge11101 ]
  %v132168 = phi i64 [ %v64807, %edge7450 ], [ %v64807, %edge7391 ], [ %v64807, %edge7395 ], [ %v64807, %block43158 ], [ %v64807, %edge7447 ], [ %v218964, %block69347 ], [ %v218964, %edge11098 ], [ %v218964, %edge11101 ]
  %v132179 = phi i64 [ %v64913, %edge7450 ], [ %v64913, %edge7391 ], [ %v64913, %edge7395 ], [ %v64913, %block43158 ], [ %v64913, %edge7447 ], [ %v218965, %block69347 ], [ %v218965, %edge11098 ], [ %v218965, %edge11101 ]
  %v132184 = phi i64 [ %v64918, %edge7450 ], [ %v64918, %edge7391 ], [ %v64918, %edge7395 ], [ %v64918, %block43158 ], [ %v64918, %edge7447 ], [ %v218967, %block69347 ], [ %v218967, %edge11098 ], [ %v218967, %edge11101 ]
  %v132185 = phi i64 [ %v64925, %edge7450 ], [ %v64925, %edge7391 ], [ %v64925, %edge7395 ], [ %v64925, %block43158 ], [ %v64925, %edge7447 ], [ %v115047, %block69347 ], [ %v115047, %edge11098 ], [ %v115047, %edge11101 ]
  %v132186 = phi i64 [ %v64926, %edge7450 ], [ %v64926, %edge7391 ], [ %v64926, %edge7395 ], [ %v64926, %block43158 ], [ %v64926, %edge7447 ], [ %v218975, %block69347 ], [ %v218975, %edge11098 ], [ %v218975, %edge11101 ]
  %v132190 = phi i64 [ %v64930, %edge7450 ], [ %v64930, %edge7391 ], [ %v64930, %edge7395 ], [ %v64930, %block43158 ], [ %v64930, %edge7447 ], [ 0, %block69347 ], [ %v115052, %edge11098 ], [ %v115052, %edge11101 ]
  %v132191 = phi i64 [ %v64937, %edge7450 ], [ %v64937, %edge7391 ], [ %v64937, %edge7395 ], [ %v64937, %block43158 ], [ %v64937, %edge7447 ], [ %v218968, %block69347 ], [ %v218968, %edge11098 ], [ %v218968, %edge11101 ]
  %v132196 = phi i64 [ %v64942, %edge7450 ], [ %v64942, %edge7391 ], [ %v64942, %edge7395 ], [ %v64942, %block43158 ], [ %v64942, %edge7447 ], [ %v218969, %block69347 ], [ %v218969, %edge11098 ], [ %v218969, %edge11101 ]
  %v132197 = phi i64 [ %v64949, %edge7450 ], [ %v64949, %edge7391 ], [ %v64949, %edge7395 ], [ %v64949, %block43158 ], [ %v64949, %edge7447 ], [ %v218970, %block69347 ], [ %v218970, %edge11098 ], [ %v218970, %edge11101 ]
  %v132198 = phi i64 [ %v64952, %edge7450 ], [ %v64952, %edge7391 ], [ %v64952, %edge7395 ], [ %v64952, %block43158 ], [ %v64952, %edge7447 ], [ %v218971, %block69347 ], [ %v218971, %edge11098 ], [ %v218971, %edge11101 ]
  %v132199 = phi i64 [ %v64955, %edge7450 ], [ %v64955, %edge7391 ], [ %v64955, %edge7395 ], [ %v64955, %block43158 ], [ %v64955, %edge7447 ], [ %v218972, %block69347 ], [ %v218972, %edge11098 ], [ %v218972, %edge11101 ]
  %v132200 = phi i64 [ %v64958, %edge7450 ], [ %v64958, %edge7391 ], [ %v64958, %edge7395 ], [ %v64958, %block43158 ], [ %v64958, %edge7447 ], [ %v130227.lcssa, %block69347 ], [ %v130227.lcssa, %edge11098 ], [ %v130227.lcssa, %edge11101 ]
  %v132941 = phi i64 [ 1, %edge7450 ], [ 1, %edge7391 ], [ 1, %edge7395 ], [ 1, %block43158 ], [ 1, %edge7447 ], [ 0, %block69347 ], [ 0, %edge11098 ], [ 0, %edge11101 ]
  %t6969 = icmp eq i64 %v64784, -1
  %74 = select i1 %t3928, i1 true, i1 %t6969
  %v133271 = select i1 %74, i64 %v64847, i64 %v64784
  %v132976 = or i64 %v132941, %v65001
  %t6974.not = icmp eq i64 %v132976, 0
  br i1 %t6974.not, label %edge11354, label %edge11188

edge11188:                                        ; preds = %edge11622, %edge11619, %block73434, %edge11167
  %v139501 = phi i64 [ %v133271, %edge11167 ], [ %v136824, %block73434 ], [ %v136824, %edge11619 ], [ %v136824, %edge11622 ]
  %v139088 = xor i64 %v64969, 1
  %v139146 = and i64 %v139088, %v64742
  %t7142.not = icmp eq i64 %v139146, 0
  br i1 %t7142.not, label %edge11677, label %edge11652

edge11354:                                        ; preds = %edge11167
  %t7032.not = icmp eq i64 %v132168, 0
  br i1 %t7032.not, label %edge11358, label %edge11355

edge11355:                                        ; preds = %edge11354
  %t7033 = inttoptr i64 %v132168 to ptr
  %v133283 = load i64, ptr %t7033, align 1
  %v133284 = add i64 %v133283, 1
  store i64 %v133284, ptr %t7033, align 1
  br label %edge11358

edge11358:                                        ; preds = %edge11354, %edge11355
  %t7035.not = icmp eq i64 %v80922, 0
  br i1 %t7035.not, label %edge11362, label %edge11359

edge11359:                                        ; preds = %edge11358
  %t7036 = inttoptr i64 %v80922 to ptr
  %v133287 = load i64, ptr %t7036, align 1
  %v133288 = add i64 %v133287, 1
  store i64 %v133288, ptr %t7036, align 1
  br label %edge11362

edge11362:                                        ; preds = %edge11358, %edge11359
  %t7042 = icmp ult i64 %v132200, 6148914691236517206
  br i1 %t7042, label %edge11368, label %common.ret

edge11368:                                        ; preds = %edge11362
  %v133317 = mul nuw i64 %v132200, 3
  %v133322 = add i64 %v133317, 14
  %t7046 = icmp samesign ult i64 %v132200, 6148914691236517201
  br i1 %t7046, label %edge11373, label %common.ret

edge11373:                                        ; preds = %edge11368
  %t7050.not = icmp eq i64 %v132184, 0
  br i1 %t7050.not, label %edge11377, label %edge11374

edge11374:                                        ; preds = %edge11373
  %t7051 = inttoptr i64 %v132184 to ptr
  %v133327 = load i64, ptr %t7051, align 1
  %v133328 = add i64 %v133327, 1
  store i64 %v133328, ptr %t7051, align 1
  br label %edge11377

edge11377:                                        ; preds = %edge11373, %edge11374
  %t7053.not = icmp eq i64 %v132190, 0
  br i1 %t7053.not, label %block5677.preheader, label %edge11378

edge11378:                                        ; preds = %edge11377
  %t7054 = inttoptr i64 %v132190 to ptr
  %v133331 = load i64, ptr %t7054, align 1
  %v133332 = add i64 %v133331, 1
  store i64 %v133332, ptr %t7054, align 1
  br label %block5677.preheader

block5677.preheader:                              ; preds = %edge11377, %edge11378
  br label %block5677

edge11407:                                        ; preds = %block5677
  %v134297 = add nuw i64 %v133678, ptrtoint (ptr @mech_compact_table_14 to i64)
  %t7060 = inttoptr i64 %v134297 to ptr
  %t7061 = load i8, ptr %t7060, align 1
  %v134298 = zext i8 %t7061 to i64
  %v134937 = shl nuw nsw i64 %v133678, 2
  %v134938 = add i64 %v134937, %v132179
  %t7065 = inttoptr i64 %v134938 to ptr
  %t7066 = load i32, ptr %t7065, align 1
  %v134939 = zext i32 %t7066 to i64
  %t7067 = icmp ult i64 %v133678, %v132186
  br i1 %t7067, label %edge11467, label %edge11482

edge11467:                                        ; preds = %edge11407
  %v135579 = add i64 %v134937, %v132185
  %t7070 = inttoptr i64 %v135579 to ptr
  %t7071 = load i32, ptr %t7070, align 1
  %75 = lshr i32 %t7071, 16
  %76 = zext nneg i32 %75 to i64
  br label %edge11482

edge11482:                                        ; preds = %edge11407, %edge11467
  %v135600 = phi i64 [ %76, %edge11467 ], [ 0, %edge11407 ]
  %77 = add nuw nsw i64 %v135600, %v134298
  %78 = mul nuw nsw i64 %77, %v134939
  %v135647 = add i64 %78, %v133681
  %t7085.not = icmp ult i64 %v135647, %v133681
  %v135928 = add nuw nsw i64 %v133678, 1
  br i1 %t7085.not, label %common.ret, label %block5677

edge11489:                                        ; preds = %block5677
  br i1 %t7050.not, label %block71789, label %edge11492

edge11492:                                        ; preds = %edge11489
  %v135658 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v132184)
  %t7092.not = icmp eq i64 %v135658, 0
  br i1 %t7092.not, label %block71789, label %block71785

edge11496:                                        ; preds = %block71789
  %t7094 = inttoptr i64 %v132190 to ptr
  %v135660 = load i64, ptr %t7094, align 1
  %v135661 = add i64 %v135660, -1
  store i64 %v135661, ptr %t7094, align 1
  %t7096.not = icmp eq i64 %v135661, 0
  br i1 %t7096.not, label %edge11499, label %block5790.preheader

edge11499:                                        ; preds = %edge11496
  %v135662 = call i64 @mech_compact_free(i64 %v0, i64 %v132190)
  %t7097.not = icmp eq i64 %v135662, 0
  br i1 %t7097.not, label %block5790.preheader, label %block71785

block5790.preheader:                              ; preds = %edge11499, %edge11496, %block71789
  br label %block5790

edge11547:                                        ; preds = %block5790
  %v137420 = add nuw i64 %v136821, ptrtoint (ptr @mech_compact_table_21 to i64)
  %t7105 = inttoptr i64 %v137420 to ptr
  %t7106 = load i8, ptr %t7105, align 1
  %v137421 = zext i8 %t7106 to i64
  %v138039 = shl nuw nsw i64 %v136821, 2
  %v138040 = add i64 %v138039, %v80928
  %t7107 = inttoptr i64 %v138040 to ptr
  %t7108 = load i32, ptr %t7107, align 1
  %v138041 = zext i32 %t7108 to i64
  %t7109 = icmp ult i64 %v136821, %v132164
  br i1 %t7109, label %edge11590, label %edge11605

edge11590:                                        ; preds = %edge11547
  %v138660 = add i64 %v138039, %v132163
  %t7112 = inttoptr i64 %v138660 to ptr
  %t7113 = load i32, ptr %t7112, align 1
  %79 = lshr i32 %t7113, 16
  %80 = zext nneg i32 %79 to i64
  br label %edge11605

edge11605:                                        ; preds = %edge11547, %edge11590
  %v138681 = phi i64 [ %80, %edge11590 ], [ 0, %edge11547 ]
  %81 = add nuw nsw i64 %v138681, %v137421
  %82 = mul nuw nsw i64 %81, %v138041
  %v138727 = add i64 %82, %v136824
  %t7127.not = icmp ult i64 %v138727, %v136824
  %v138998 = add nuw nsw i64 %v136821, 1
  br i1 %t7127.not, label %common.ret, label %block5790

edge11612:                                        ; preds = %block5790
  br i1 %t7035.not, label %block73434, label %edge11615

edge11615:                                        ; preds = %edge11612
  %v138738 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v80922)
  %t7134.not = icmp eq i64 %v138738, 0
  br i1 %t7134.not, label %block73434, label %block73430

edge11619:                                        ; preds = %block73434
  %t7136 = inttoptr i64 %v132168 to ptr
  %v138740 = load i64, ptr %t7136, align 1
  %v138741 = add i64 %v138740, -1
  store i64 %v138741, ptr %t7136, align 1
  %t7138.not = icmp eq i64 %v138741, 0
  br i1 %t7138.not, label %edge11622, label %edge11188

edge11622:                                        ; preds = %edge11619
  %v138742 = call i64 @mech_compact_free(i64 %v0, i64 %v132168)
  %t7139.not = icmp eq i64 %v138742, 0
  br i1 %t7139.not, label %edge11188, label %block73430

edge11650:                                        ; preds = %edge11652
  %t7148.not = icmp ult i64 %v139501, %v64784
  br i1 %t7148.not, label %edge11668, label %edge11677

edge11652:                                        ; preds = %edge11188
  %t7144.not = icmp eq i64 %v139501, -1
  br i1 %t7144.not, label %common.ret, label %edge11650

edge11668:                                        ; preds = %edge11650
  %t7153.not = icmp eq i64 %v64777, 0
  br i1 %t7153.not, label %edge11679, label %edge11672

edge11672:                                        ; preds = %edge11668
  %v139756 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64777)
  %t7154.not = icmp eq i64 %v139756, 0
  br i1 %t7154.not, label %edge11679, label %edge11673

edge11673:                                        ; preds = %edge11672
  %t7155 = insertvalue { i64, i64 } poison, i64 %v139756, 0
  %t7156 = insertvalue { i64, i64 } %t7155, i64 6022, 1
  br label %common.ret

edge11677:                                        ; preds = %edge11188, %edge11650, %edge11680, %edge11679
  %spec.select361079 = phi i64 [ %v139501, %edge11680 ], [ %v139501, %edge11679 ], [ %v64784, %edge11650 ], [ %v64784, %edge11188 ]
  %t7149.not1078 = phi i1 [ false, %edge11680 ], [ false, %edge11679 ], [ true, %edge11650 ], [ true, %edge11188 ]
  %v1395321077 = phi i64 [ 1, %edge11680 ], [ 1, %edge11679 ], [ 0, %edge11650 ], [ 0, %edge11188 ]
  %v140768 = phi i64 [ %v66494, %edge11680 ], [ 0, %edge11679 ], [ %v64777, %edge11650 ], [ %v64777, %edge11188 ]
  %v147323 = phi i64 [ %v67292, %edge11680 ], [ %v67292, %edge11679 ], [ %v64772, %edge11650 ], [ %v64772, %edge11188 ]
  %v147329 = phi i64 [ %v67293, %edge11680 ], [ %v67293, %edge11679 ], [ %v64774, %edge11650 ], [ %v64774, %edge11188 ]
  %v140564 = or i64 %v1395321077, %v64745
  %t7160 = icmp eq i64 %v140564, 1
  br i1 %t7160, label %edge11853, label %block6130

edge11679:                                        ; preds = %edge11668, %edge11672
  %t7157.not = icmp eq i64 %v66494, 0
  br i1 %t7157.not, label %edge11677, label %edge11680

edge11680:                                        ; preds = %edge11679
  %t7158 = inttoptr i64 %v66494 to ptr
  %v140225 = load i64, ptr %t7158, align 1
  %v140226 = add i64 %v140225, 1
  store i64 %v140226, ptr %t7158, align 1
  br label %edge11677

edge11853:                                        ; preds = %edge11677
  %t7216.not = icmp eq i64 %v132168, 0
  br i1 %t7216.not, label %edge11857, label %edge11854

edge11854:                                        ; preds = %edge11853
  %t7217 = inttoptr i64 %v132168 to ptr
  %v140789 = load i64, ptr %t7217, align 1
  %v140790 = add i64 %v140789, 1
  store i64 %v140790, ptr %t7217, align 1
  br label %edge11857

edge11857:                                        ; preds = %edge11853, %edge11854
  %t7219.not = icmp eq i64 %v66494, 0
  br i1 %t7219.not, label %block5938.preheader, label %edge11858

edge11858:                                        ; preds = %edge11857
  %t7220 = inttoptr i64 %v66494 to ptr
  %v141546 = load i64, ptr %t7220, align 1
  %v141547 = add i64 %v141546, 1
  store i64 %v141547, ptr %t7220, align 1
  br label %block5938.preheader

block5938.preheader:                              ; preds = %edge11857, %edge11858
  br label %block5938

edge11868:                                        ; preds = %block5938
  %t7223 = inttoptr i64 %v141572 to ptr
  %v141573 = load i64, ptr %t7223, align 1
  %t7224 = icmp eq i64 %v141573, 1
  %t7226 = icmp ugt i64 %v141600, 546
  %or.cond322 = select i1 %t7224, i1 %t7226, i1 false
  br i1 %or.cond322, label %edge11872, label %edge11899.thread

edge11872:                                        ; preds = %edge11868
  %t7228 = zext nneg i64 %v141601 to i128
  %t7229 = zext i64 %v141602 to i128
  %t7230 = shl nuw i128 %t7229, 64
  %v220217 = or disjoint i128 %t7230, %t7228
  store i128 %v220217, ptr %ss2, align 16
  store i128 18446744073709552163, ptr %t926, align 16
  %t7238 = zext i64 %v132163 to i128
  %t7239 = shl nuw i128 %t7238, 64
  %v220221 = or disjoint i128 %t7239, 547
  store i128 %v220221, ptr %t931, align 16
  %t7242 = zext i64 %v132164 to i128
  %v220223 = or disjoint i128 %t7242, 18446744073709551616
  store i128 %v220223, ptr %t936, align 16
  %t7247 = zext i64 %v132167 to i128
  %v220225 = or disjoint i128 %t7249, %t7247
  store i128 %v220225, ptr %t941, align 16
  store i128 18446744073709552163, ptr %t946, align 16
  %t7258 = zext i64 %v141572 to i128
  %t7259 = shl nuw i128 %t7258, 64
  %v220229 = or disjoint i128 %t7259, 547
  store i128 %v220229, ptr %t947, align 16
  store i64 %v141600, ptr %t1152, align 16
  %t7263 = call fastcc { i64, i64 } @mech_compact_numeric_6038(i64 %v219576, i64 %v8481)
  %v141578 = extractvalue { i64, i64 } %t7263, 0
  %t7264.not = icmp eq i64 %v141578, 0
  br i1 %t7264.not, label %edge11876, label %edge11874

edge11874:                                        ; preds = %edge11872
  %v141579 = extractvalue { i64, i64 } %t7263, 1
  %t7265 = icmp eq i64 %v141578, 103169
  %v141582 = select i1 %t7265, i64 103425, i64 %v141578
  %t7267 = insertvalue { i64, i64 } poison, i64 %v141582, 0
  %t7268 = insertvalue { i64, i64 } %t7267, i64 %v141579, 1
  br label %common.ret

edge11876:                                        ; preds = %edge11872
  br i1 %t7216.not, label %edge12051, label %edge11879

edge11879:                                        ; preds = %edge11876
  %v141591 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v132168)
  %t7270.not = icmp eq i64 %v141591, 0
  br i1 %t7270.not, label %edge12051, label %edge11880

edge11880:                                        ; preds = %edge11879
  %t7271 = insertvalue { i64, i64 } poison, i64 %v141591, 0
  %t7272 = insertvalue { i64, i64 } %t7271, i64 6038, 1
  br label %common.ret

edge11899:                                        ; preds = %block5938
  %t7273 = icmp samesign ult i64 %v141601, 547
  br i1 %t7273, label %edge11905, label %edge12007

edge11899.thread:                                 ; preds = %edge11868
  %t72731092 = icmp samesign ult i64 %v141601, 547
  br i1 %t72731092, label %edge11905, label %edge12002

edge11905:                                        ; preds = %edge11899.thread, %edge11899
  %v142005 = shl nuw nsw i64 %v141601, 1
  %v142006 = add i64 %v142005, ptrtoint (ptr @mech_compact_table_11 to i64)
  %t7278 = inttoptr i64 %v142006 to ptr
  %t7279 = load i16, ptr %t7278, align 1
  %v142007 = zext i16 %t7279 to i64
  %v142138 = and i64 %v142007, 511
  %v142188 = lshr i64 %v142007, 9
  %t7283 = icmp ult i64 %v142138, %v132164
  br i1 %t7283, label %edge11972, label %edge11972.thread

edge11972.thread:                                 ; preds = %edge11905
  %t72891088 = icmp samesign ult i64 %v142138, 288
  %spec.select371089 = select i1 %t72891088, i64 13, i64 10
  br label %85

edge11972:                                        ; preds = %edge11905
  %v142767 = shl nuw nsw i64 %v142138, 2
  %v142768 = add i64 %v142767, %v132163
  %t7286 = inttoptr i64 %v142768 to ptr
  %t7287 = load i32, ptr %t7286, align 1
  %t7287.fr = freeze i32 %t7287
  %83 = lshr i32 %t7287.fr, 16
  %84 = zext nneg i32 %83 to i64
  %t7292.not = icmp eq i32 %83, 0
  %t7289 = icmp samesign ult i64 %v142138, 288
  %spec.select37 = select i1 %t7289, i64 13, i64 10
  %spec.select1292 = select i1 %t7292.not, i64 %spec.select37, i64 %84
  br label %85

85:                                               ; preds = %edge11972, %edge11972.thread
  %86 = phi i64 [ %spec.select1292, %edge11972 ], [ %spec.select371089, %edge11972.thread ]
  %v143828 = add nuw nsw i64 %86, %v142188
  %v143838 = shl nuw nsw i64 %v143828, 4
  %v143868 = shl nuw nsw i64 %v141601, 2
  %v143869 = add i64 %v141602, %v143868
  %t7311 = inttoptr i64 %v143869 to ptr
  %t7312 = load i32, ptr %t7311, align 1
  %v143870 = zext i32 %t7312 to i64
  %t7313 = icmp eq i64 %v143838, %v143870
  br i1 %t7313, label %block75845, label %edge11986

edge11986:                                        ; preds = %85
  br i1 %t7222.not, label %block75847, label %edge11987

edge11987:                                        ; preds = %edge11986
  %t7316 = inttoptr i64 %v141572 to ptr
  %v143886 = load i64, ptr %t7316, align 1
  %t7317 = icmp eq i64 %v143886, 1
  br i1 %t7317, label %block75846, label %block75847

edge11992:                                        ; preds = %block75847
  %v143896 = load i64, ptr %ss1, align 8
  %v143897 = load i64, ptr %t606, align 8
  %v143898 = load i64, ptr %t607, align 8
  %.pre21725 = add i64 %v143896, %v143868
  %.pre21726 = inttoptr i64 %.pre21725 to ptr
  br label %block75846

edge11995:                                        ; preds = %block75845
  %t7341 = inttoptr i64 %v143879 to ptr
  %v143903 = load i64, ptr %t7341, align 1
  %v143904 = add i64 %v143903, 1
  store i64 %v143904, ptr %t7341, align 1
  br label %edge12006

edge12002:                                        ; preds = %edge11899.thread
  %v143924 = add i64 %v141573, 1
  store i64 %v143924, ptr %t7223, align 1
  br label %edge12007

edge12006:                                        ; preds = %block75845, %edge11995
  %t7358.not = or i1 %t7222.not, %t7338
  br i1 %t7358.not, label %edge12032, label %edge12025

edge12007:                                        ; preds = %edge11899, %edge12002
  br i1 %t7216.not, label %edge12014, label %edge12010

edge12010:                                        ; preds = %edge12007
  %v143933 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v132168)
  %t7352.not = icmp eq i64 %v143933, 0
  br i1 %t7352.not, label %edge12014, label %block75862

edge12014:                                        ; preds = %edge12010, %edge12007
  %t7353.not = icmp eq i64 %v141572, 0
  br i1 %t7353.not, label %edge12051, label %edge12015

edge12015:                                        ; preds = %edge12014
  %t7354 = inttoptr i64 %v141572 to ptr
  %v143939 = load i64, ptr %t7354, align 1
  %v143940 = add i64 %v143939, -1
  store i64 %v143940, ptr %t7354, align 1
  %t7356.not = icmp eq i64 %v143940, 0
  br i1 %t7356.not, label %edge12018, label %edge12051

edge12018:                                        ; preds = %edge12015
  %v143941 = call i64 @mech_compact_free(i64 %v0, i64 %v141572)
  %t7357.not = icmp eq i64 %v143941, 0
  br i1 %t7357.not, label %edge12051, label %block75862

edge12025:                                        ; preds = %edge12006
  %v143945 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v141572)
  %t7359.not = icmp eq i64 %v143945, 0
  br i1 %t7359.not, label %edge12032, label %block75862

edge12032:                                        ; preds = %edge12006, %edge12025
  %v144204 = add nuw nsw i64 %v141601, 1
  br label %block5938

edge12051:                                        ; preds = %edge11876, %edge11879, %edge12018, %edge12015, %edge12014
  br i1 %t7149.not1078, label %edge12055, label %edge12057

edge12055:                                        ; preds = %edge12051, %edge12058
  %v144576 = phi i64 [ %v144558, %edge12058 ], [ %v64787, %edge12051 ]
  %t7371.not = icmp eq i64 %v144576, 0
  %v144584 = select i1 %t7371.not, i64 0, i64 %v1395321077
  %t7373.not213 = icmp eq i64 %v144584, 0
  %t7373.not = select i1 %t7372, i1 true, i1 %t7373.not213
  br i1 %t7373.not, label %edge12069, label %edge12080

edge12057:                                        ; preds = %edge12051
  %t7367.not = icmp eq i64 %v64787, 0
  br i1 %t7367.not, label %common.ret, label %edge12058

edge12058:                                        ; preds = %edge12057
  %v144558 = add i64 %v64787, -1
  br label %edge12055

edge12069:                                        ; preds = %edge12055
  br i1 %t7219.not, label %block6130, label %edge12070

edge12070:                                        ; preds = %edge12069
  %t7376 = inttoptr i64 %v66494 to ptr
  %v144795 = load i64, ptr %t7376, align 1
  %v144796 = add i64 %v144795, -1
  store i64 %v144796, ptr %t7376, align 1
  %t7378.not = icmp eq i64 %v144796, 0
  br i1 %t7378.not, label %edge12073, label %block6130

edge12073:                                        ; preds = %edge12070
  %v144797 = call i64 @mech_compact_free(i64 %v0, i64 %v66494)
  %t7379.not = icmp eq i64 %v144797, 0
  br i1 %t7379.not, label %block6130, label %edge12074

edge12074:                                        ; preds = %edge12073
  %t7380 = insertvalue { i64, i64 } poison, i64 %v144797, 0
  %t7381 = insertvalue { i64, i64 } %t7380, i64 6153, 1
  br label %common.ret

edge12080:                                        ; preds = %edge12055
  %t7382.not = icmp eq i64 %v141572, 0
  br i1 %t7382.not, label %block6057.preheader, label %edge12081

edge12081:                                        ; preds = %edge12080
  %t7383 = inttoptr i64 %v141572 to ptr
  %v145007 = load i64, ptr %t7383, align 1
  %v145008 = add i64 %v145007, 1
  store i64 %v145008, ptr %t7383, align 1
  br label %block6057.preheader

block6057.preheader:                              ; preds = %edge12080, %edge12081
  br label %block6057

edge12104:                                        ; preds = %block6057
  br i1 %t7219.not, label %block76581, label %edge12107

edge12107:                                        ; preds = %edge12104
  %v145098 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66494)
  %t7389.not = icmp eq i64 %v145098, 0
  br i1 %t7389.not, label %block76581, label %block76577

edge12111:                                        ; preds = %block76581
  %t7391 = inttoptr i64 %v141572 to ptr
  %v145100 = load i64, ptr %t7391, align 1
  %v145101 = add i64 %v145100, -1
  store i64 %v145101, ptr %t7391, align 1
  %t7393.not = icmp eq i64 %v145101, 0
  br i1 %t7393.not, label %edge12114, label %block6130

edge12114:                                        ; preds = %edge12111
  %v145102 = call i64 @mech_compact_free(i64 %v0, i64 %v141572)
  %t7394.not = icmp eq i64 %v145102, 0
  br i1 %t7394.not, label %block6130, label %block76577

edge12137:                                        ; preds = %block6057
  %v145657 = shl nuw nsw i64 %v145075, 2
  %v145658 = add i64 %v145657, %v67292
  %t7400 = inttoptr i64 %v145658 to ptr
  %t7401 = load i32, ptr %t7400, align 1
  %v145989 = add i64 %v145657, %v141602
  %t7405 = inttoptr i64 %v145989 to ptr
  %t7406 = load i32, ptr %t7405, align 1
  %t7407 = icmp eq i32 %t7401, %t7406
  %v146281 = add nuw nsw i64 %v145075, 1
  br i1 %t7407, label %block6057, label %edge12172

edge12172:                                        ; preds = %edge12137
  br i1 %t7219.not, label %block77244, label %edge12175

edge12175:                                        ; preds = %edge12172
  %v146025 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66494)
  %t7411.not = icmp eq i64 %v146025, 0
  br i1 %t7411.not, label %block77244, label %block77240

edge12179:                                        ; preds = %block77244
  %t7413 = inttoptr i64 %v141572 to ptr
  %v146027 = load i64, ptr %t7413, align 1
  %v146028 = add i64 %v146027, -1
  store i64 %v146028, ptr %t7413, align 1
  %t7415.not = icmp eq i64 %v146028, 0
  br i1 %t7415.not, label %edge12182, label %block6130

edge12182:                                        ; preds = %edge12179
  %v146029 = call i64 @mech_compact_free(i64 %v0, i64 %v141572)
  %t7416.not = icmp eq i64 %v146029, 0
  br i1 %t7416.not, label %block6130, label %block77240

edge12236:                                        ; preds = %edge12238
  %t7431.not = icmp ugt i64 %v139501, %spec.select361079
  %v147111 = and i64 %v146730, %v146449
  br i1 %t7431.not, label %edge12251, label %edge12256

edge12238:                                        ; preds = %block6130
  %t7427.not = icmp eq i64 %spec.select361079, -1
  br i1 %t7427.not, label %common.ret, label %edge12236

edge12249.sink.split:                             ; preds = %edge12256, %edge12251
  %v219060.sink = phi i64 [ %v140768, %edge12251 ], [ %v219060, %edge12256 ]
  %v1471111098.ph = phi i64 [ %v147111, %edge12251 ], [ %v1471111099, %edge12256 ]
  %v1471161096.ph = phi i64 [ 1, %edge12251 ], [ 0, %edge12256 ]
  %v165697.ph = phi i64 [ %v147323, %edge12251 ], [ %v219067, %edge12256 ]
  %v165699.ph = phi i64 [ %v147329, %edge12251 ], [ %v219069, %edge12256 ]
  %t7438 = inttoptr i64 %v219060.sink to ptr
  %v147359 = load i64, ptr %t7438, align 1
  %v147360 = add i64 %v147359, 1
  store i64 %v147360, ptr %t7438, align 1
  br label %edge12249

edge12249:                                        ; preds = %edge12249.sink.split, %edge12256, %edge12251
  %v1471111098 = phi i64 [ %v1471111099, %edge12256 ], [ %v147111, %edge12251 ], [ %v1471111098.ph, %edge12249.sink.split ]
  %v1471161096 = phi i64 [ 0, %edge12256 ], [ 1, %edge12251 ], [ %v1471161096.ph, %edge12249.sink.split ]
  %v147587 = phi i64 [ 0, %edge12256 ], [ 0, %edge12251 ], [ %v219060.sink, %edge12249.sink.split ]
  %v165697 = phi i64 [ %v219067, %edge12256 ], [ %v147323, %edge12251 ], [ %v165697.ph, %edge12249.sink.split ]
  %v165699 = phi i64 [ %v219069, %edge12256 ], [ %v147329, %edge12251 ], [ %v165699.ph, %edge12249.sink.split ]
  %v147425215 = and i1 %v55220135.not, %t3926
  %v147503216 = and i1 %t7440, %v147425215
  %v147503 = zext i1 %v147503216 to i64
  %87 = xor i64 %v147503, 1
  %v147523 = select i1 %v147425215, i64 %87, i64 0
  %v147572 = or i64 %v147523, %v64985
  %t7441.not = icmp eq i64 %v147572, 0
  br i1 %t7441.not, label %edge12271, label %edge12270

edge12251:                                        ; preds = %edge12236
  %t7434.not = icmp eq i64 %v140768, 0
  br i1 %t7434.not, label %edge12249, label %edge12249.sink.split

edge12256:                                        ; preds = %edge12236.thread, %edge12236
  %v1471111099 = phi i64 [ %v1471111094, %edge12236.thread ], [ %v147111, %edge12236 ]
  %t7437.not = icmp eq i64 %v219060, 0
  br i1 %t7437.not, label %edge12249, label %edge12249.sink.split

edge12270:                                        ; preds = %edge12249
  %t7445.not = icmp eq i64 %v147587, 0
  br i1 %t7445.not, label %edge12449, label %edge12281

edge12271:                                        ; preds = %edge12249
  %t7443.not = icmp eq i64 %v64765, 0
  br i1 %t7443.not, label %block6792, label %edge12274

edge12274:                                        ; preds = %edge12271
  %v147581 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64765)
  %t7444.not = icmp eq i64 %v147581, 0
  br i1 %t7444.not, label %block6792, label %block78101

edge12281:                                        ; preds = %edge12270
  %v147585 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v147587)
  %t7446.not = icmp eq i64 %v147585, 0
  br i1 %t7446.not, label %edge12449, label %block78101

edge12449:                                        ; preds = %edge12270, %edge12281
  %t7503.not = icmp eq i64 %v80922, 0
  br i1 %t7503.not, label %block6192.preheader, label %edge12450

edge12450:                                        ; preds = %edge12449
  %t7504 = inttoptr i64 %v80922 to ptr
  %v147815 = load i64, ptr %t7504, align 1
  %v147816 = add i64 %v147815, 1
  store i64 %v147816, ptr %t7504, align 1
  br label %block6192.preheader

block6192.preheader:                              ; preds = %edge12449, %edge12450
  br label %block6192

edge12471:                                        ; preds = %block6192
  %v148503 = shl nuw nsw i64 %v147883, 2
  %v148504 = add i64 %v148503, %v80928
  %t7511 = inttoptr i64 %v148504 to ptr
  %t7512 = load i32, ptr %t7511, align 1
  %v148505 = zext i32 %t7512 to i64
  %t7513 = icmp samesign ult i64 %v147829, %v148505
  br i1 %t7513, label %edge12492, label %edge12511

edge12492:                                        ; preds = %edge12471
  %v148814 = add nuw i64 %v147886, 1
  %t7516.not = icmp eq i64 %v147886, -1
  br i1 %t7516.not, label %common.ret, label %edge12511

edge12500:                                        ; preds = %block6192
  br i1 %t7503.not, label %edge12521, label %edge12501

edge12501:                                        ; preds = %edge12500
  %t7523 = inttoptr i64 %v80922 to ptr
  %v148836 = load i64, ptr %t7523, align 1
  %v148837 = add i64 %v148836, -1
  store i64 %v148837, ptr %t7523, align 1
  %t7525.not = icmp eq i64 %v148837, 0
  br i1 %t7525.not, label %edge12504, label %edge12521

edge12504:                                        ; preds = %edge12501
  %v148838 = call i64 @mech_compact_free(i64 %v0, i64 %v80922)
  %t7526.not = icmp eq i64 %v148838, 0
  br i1 %t7526.not, label %edge12521, label %edge12505

edge12505:                                        ; preds = %edge12504
  %t7527 = insertvalue { i64, i64 } poison, i64 %v148838, 0
  %t7528 = insertvalue { i64, i64 } %t7527, i64 6347, 1
  br label %common.ret

edge12511:                                        ; preds = %edge12471, %edge12492
  %v219105.ph = phi i64 [ %v147886, %edge12471 ], [ %v148814, %edge12492 ]
  %v149090 = add nuw nsw i64 %v147883, 1
  br label %block6192

edge12521:                                        ; preds = %edge12500, %edge12501, %edge12504
  %spec.select38 = call i64 @llvm.umax.i64(i64 %v147886, i64 1)
  %t7548 = icmp ugt i64 %v147886, 79
  br i1 %t7548, label %block6359, label %edge12551

edge12551:                                        ; preds = %edge12521
  %v150958 = add i64 %spec.select38, ptrtoint (ptr @mech_compact_table_6 to i64)
  %t7553 = inttoptr i64 %v150958 to ptr
  %t7554 = load i8, ptr %t7553, align 1
  %v150959 = zext i8 %t7554 to i64
  br label %block6359

edge12573:                                        ; preds = %block6359
  %v153809 = load i64, ptr %t7556, align 1
  %v153810 = add i64 %v153809, 1
  store i64 %v153810, ptr %t7556, align 1
  br label %block6361.preheader

edge12591:                                        ; preds = %block6361
  %v154471 = add nuw nsw i64 %v153853, 1
  %v154477 = shl nuw nsw i64 %v153853, 2
  %v154478 = add i64 %v154477, %v54933
  %t7562 = inttoptr i64 %v154478 to ptr
  %t7563 = load i32, ptr %t7562, align 1
  %v154479 = zext i32 %t7563 to i64
  %t7564 = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %v153856, i64 %v154479)
  %v154515 = extractvalue { i64, i1 } %t7564, 0
  %t7565 = extractvalue { i64, i1 } %t7564, 1
  br i1 %t7565, label %common.ret, label %edge12618

edge12592:                                        ; preds = %block6361
  br i1 %t7555.not, label %edge12650, label %edge12627

edge12618:                                        ; preds = %edge12591
  %t7570 = mul nuw nsw i64 %v153853, %v154479
  %t7576 = call { i64, i1 } @llvm.ssub.with.overflow.i64(i64 %v153859, i64 %t7570)
  %t7577 = extractvalue { i64, i1 } %t7576, 1
  br i1 %t7577, label %common.ret, label %edge12637

edge12627:                                        ; preds = %edge12592
  %v154871 = load i64, ptr %t7556, align 1
  %v154872 = add i64 %v154871, -1
  store i64 %v154872, ptr %t7556, align 1
  %t7585.not = icmp eq i64 %v154872, 0
  br i1 %t7585.not, label %edge12630, label %edge12650

edge12630:                                        ; preds = %edge12627
  %v154873 = call i64 @mech_compact_free(i64 %v0, i64 %v54938)
  %t7586.not = icmp eq i64 %v154873, 0
  br i1 %t7586.not, label %edge12650, label %edge12631

edge12631:                                        ; preds = %edge12630
  %t7587 = insertvalue { i64, i64 } poison, i64 %v154873, 0
  %t7588 = insertvalue { i64, i64 } %t7587, i64 6529, 1
  br label %common.ret

edge12637:                                        ; preds = %edge12618
  %v154838 = extractvalue { i64, i1 } %t7576, 0
  br label %block6361

edge12650:                                        ; preds = %edge12630, %edge12627, %edge12592
  %spec.select39 = call i64 @llvm.smax.i64(i64 %v153859, i64 0)
  %t7593 = icmp sgt i64 %v153856, %spec.select39
  br i1 %t7593, label %.thread, label %edge12667

edge12663:                                        ; preds = %edge12667
  %v155870 = shl nsw i64 %v153856, 2
  %t7602 = icmp sgt i64 %v155870, %spec.select39
  %cond.fr1100 = freeze i1 %t7602
  %v156148 = zext i1 %cond.fr1100 to i64
  %spec.select1306 = select i1 %cond.fr1100, i64 ptrtoint (ptr @mech_compact_table_25 to i64), i64 ptrtoint (ptr @mech_compact_table_24 to i64)
  br label %.thread

.thread:                                          ; preds = %edge12663, %edge12650
  %v15615311051111 = phi i64 [ %v156148, %edge12663 ], [ 2, %edge12650 ]
  %88 = phi i64 [ %spec.select1306, %edge12663 ], [ ptrtoint (ptr @mech_compact_table_26 to i64), %edge12650 ]
  %t7611 = icmp ult i64 %v147886, 257
  br i1 %t7611, label %edge12725, label %edge12755

edge12667:                                        ; preds = %edge12650
  %89 = add i64 %v153856, 2305843009213693952
  %t7597 = icmp ult i64 %89, 4611686018427387904
  br i1 %t7597, label %edge12663, label %common.ret

edge12725:                                        ; preds = %.thread
  %v157877 = add i64 %88, %spec.select38
  %t7621 = inttoptr i64 %v157877 to ptr
  %t7622 = load i8, ptr %t7621, align 1
  %v157878 = zext i8 %t7622 to i64
  br label %edge12755

edge12755:                                        ; preds = %edge12725, %.thread
  %v157903 = phi i64 [ %v157878, %edge12725 ], [ 0, %.thread ]
  %v158492 = add i64 %v15615311051111, ptrtoint (ptr @mech_compact_table_4 to i64)
  %t7630 = inttoptr i64 %v158492 to ptr
  %t7631 = load i8, ptr %t7630, align 1
  %v158493 = zext i8 %t7631 to i64
  br i1 %t3695, label %edge12773, label %edge12942

edge12773:                                        ; preds = %edge12755
  br i1 %t3704.not, label %block83919, label %edge12774

edge12774:                                        ; preds = %edge12773
  %v159545 = load i64, ptr %t7636, align 1
  %v159546 = add i64 %v159545, 1
  store i64 %v159546, ptr %t7636, align 1
  br label %block83919

edge12777:                                        ; preds = %block83919
  %v159547 = load i64, ptr %t7639, align 1
  %v159548 = add i64 %v159547, 1
  store i64 %v159548, ptr %t7639, align 1
  br label %block6532.preheader

edge12800:                                        ; preds = %block6532
  %v160179 = shl nuw nsw i64 %v159617, 2
  %v160180 = add i64 %v160179, %v219241
  %t7644 = inttoptr i64 %v160180 to ptr
  %t7645 = load i32, ptr %t7644, align 1
  %90 = zext i32 %t7645 to i64
  %umul217 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %90, i64 %v54957)
  %91 = extractvalue { i64, i1 } %umul217, 1
  br i1 %91, label %common.ret, label %edge12810

edge12810:                                        ; preds = %edge12800
  %v160774 = add i64 %v160179, %v54945
  %t7654 = inttoptr i64 %v160774 to ptr
  %t7655 = load i32, ptr %t7654, align 1
  %92 = zext i32 %t7655 to i64
  %umul218 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %92, i64 %v219243)
  %93 = extractvalue { i64, i1 } %umul218, 1
  br i1 %93, label %common.ret, label %edge12862

edge12845:                                        ; preds = %block6532
  br i1 %t3704.not, label %block85013, label %edge12848

edge12848:                                        ; preds = %edge12845
  %v161126 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54950)
  %t7679.not = icmp eq i64 %v161126, 0
  br i1 %t7679.not, label %block85013, label %block85009

edge12852:                                        ; preds = %block85013
  %v161128 = load i64, ptr %t7639, align 1
  %v161129 = add i64 %v161128, -1
  store i64 %v161129, ptr %t7639, align 1
  %t7683.not = icmp eq i64 %v161129, 0
  br i1 %t7683.not, label %edge12855, label %edge12877

edge12855:                                        ; preds = %edge12852
  %v161130 = call i64 @mech_compact_free(i64 %v0, i64 %v219242)
  %t7684.not = icmp eq i64 %v161130, 0
  br i1 %t7684.not, label %edge12877, label %block85009

edge12862:                                        ; preds = %edge12810
  %v160208 = mul i64 %v54957, %90
  %v160801 = mul i64 %v219243, %92
  %t7664 = icmp ugt i64 %v160801, %v160208
  %v161091 = sub i64 %v160801, %v160208
  %94 = sub nuw i64 %v160208, %v160801
  %v161389 = select i1 %t7664, i64 %v161091, i64 %94
  %v161382 = add nuw nsw i64 %v159617, 1
  %v161390 = add i64 %v161389, %v159620
  %t7688.not = icmp ult i64 %v161390, %v159620
  br i1 %t7688.not, label %common.ret, label %block6532

edge12877:                                        ; preds = %block85013, %edge12852, %edge12855
  br i1 %brmerge, label %common.ret.loopexit22434.split.loop.exit33132, label %edge12881

edge12881:                                        ; preds = %edge12877
  %t7715 = icmp ugt i64 %v159620, %v161598
  br i1 %t7715, label %edge12942, label %edge12895

edge12895:                                        ; preds = %edge12881
  %v161878 = shl nuw nsw i64 %v159620, 2
  %t7728 = icmp samesign ugt i64 %v161878, %v161892
  br i1 %t7728, label %edge12942, label %edge12907

edge12907:                                        ; preds = %edge12895
  %v162180 = shl nuw nsw i64 %v159620, 1
  %t7741 = icmp samesign ugt i64 %v162180, %v161598
  br i1 %t7741, label %edge12942, label %edge12919

edge12919:                                        ; preds = %edge12907
  %t7749 = icmp samesign ugt i64 %v162180, %v161588220
  %v162741 = select i1 %t7749, i64 4, i64 2
  br label %edge12942

edge12942:                                        ; preds = %edge12755, %edge12895, %edge12907, %edge12919, %edge12881
  %v163094 = phi i64 [ 8, %edge12755 ], [ 8, %edge12881 ], [ 6, %edge12895 ], [ %v162741, %edge12919 ], [ 5, %edge12907 ]
  %v164975 = sub nuw nsw i64 8, %v163094
  br label %block6664

edge12943:                                        ; preds = %block6664
  %t7753 = inttoptr i64 %v163116 to ptr
  %v163117 = load i64, ptr %t7753, align 1
  %t7754 = icmp eq i64 %v163117, 1
  %t7756 = icmp ugt i64 %v163140, 546
  %or.cond323 = select i1 %t7754, i1 %t7756, i1 false
  br i1 %or.cond323, label %edge12947, label %edge12968.thread

edge12947:                                        ; preds = %edge12943
  %t7758 = zext nneg i64 %v157903 to i128
  %t7759 = zext i8 %t7631 to i128
  %t7760 = shl nuw nsw i128 %t7759, 64
  %v220237 = or disjoint i128 %t7760, %t7758
  store i128 %v220237, ptr %ss2, align 16
  %t7762 = zext nneg i64 %v163094 to i128
  %t7763 = zext nneg i64 %v163144 to i128
  %t7764 = shl nuw nsw i128 %t7763, 64
  %v220239 = or disjoint i128 %t7764, %t7762
  store i128 %v220239, ptr %t926, align 16
  %t7767 = zext i64 %v163145 to i128
  %v220241 = or disjoint i128 %t7767, 10090369008319124733952
  store i128 %v220241, ptr %t931, align 16
  store i128 10090369008319124733953, ptr %t936, align 16
  store i128 %v220245, ptr %t941, align 16
  store i128 10090369008319124733953, ptr %t946, align 16
  %t7787 = zext i64 %v163116 to i128
  %t7788 = zext i64 %v163140 to i128
  %t7789 = shl nuw i128 %t7788, 64
  %v220249 = or disjoint i128 %t7789, %t7787
  store i128 %v220249, ptr %t947, align 16
  %t7792 = call fastcc { i64, i64 } @mech_compact_numeric_6776(i64 %v219576, i64 %v8481)
  %v163122 = extractvalue { i64, i64 } %t7792, 0
  %t7793.not = icmp eq i64 %v163122, 0
  br i1 %t7793.not, label %edge13130, label %edge12949

edge12949:                                        ; preds = %edge12947
  %v163123 = extractvalue { i64, i64 } %t7792, 1
  %t7794 = icmp eq i64 %v163122, 103169
  %v163126 = select i1 %t7794, i64 103425, i64 %v163122
  %t7796 = insertvalue { i64, i64 } poison, i64 %v163126, 0
  %t7797 = insertvalue { i64, i64 } %t7796, i64 %v163123, 1
  br label %common.ret

edge12968:                                        ; preds = %block6664
  %exitcond21645.not = icmp eq i64 %v163144, 547
  br i1 %exitcond21645.not, label %edge13130, label %edge13022

edge12968.thread:                                 ; preds = %edge12943
  %exitcond21645.not22247 = icmp eq i64 %v163144, 547
  br i1 %exitcond21645.not22247, label %edge13096, label %edge13022

edge13022:                                        ; preds = %edge12968.thread, %edge12968
  %v163483 = shl nuw nsw i64 %v163144, 1
  %v163484 = add nuw i64 %v163483, ptrtoint (ptr @mech_compact_table_11 to i64)
  %t7803 = inttoptr i64 %v163484 to ptr
  %t7804 = load i16, ptr %t7803, align 1
  %95 = lshr i16 %t7804, 5
  %96 = and i16 %95, 2032
  %97 = zext nneg i16 %96 to i64
  %t78061115 = icmp samesign ult i64 %v163144, 259
  %t78091116 = icmp samesign ult i64 %v163144, 515
  %v1643371117 = select i1 %t78091116, i64 %v157903, i64 78
  %v1649501118 = select i1 %t78061115, i64 %v158493, i64 %v1643371117
  %v164896 = shl nuw nsw i64 %v163144, 2
  %v164897 = add i64 %v163145, %v164896
  %t7817 = inttoptr i64 %v164897 to ptr
  %t7818 = load i32, ptr %t7817, align 1
  %v164898 = zext i32 %t7818 to i64
  %v164951 = add nuw nsw i64 %v1649501118, %97
  %v164966 = mul nuw nsw i64 %v164951, %v163094
  %v164983 = mul nuw nsw i64 %v164975, %v164898
  %v164988 = add nuw nsw i64 %v164966, %v164983
  %v164997221 = lshr i64 %v164988, 3
  %t7863 = icmp eq i64 %v164997221, %v164898
  br i1 %t7863, label %block87366, label %edge13067

edge13067:                                        ; preds = %edge13022
  %t7865.not = icmp eq i64 %v165329, 0
  br i1 %t7865.not, label %block87369, label %edge13070

edge13070:                                        ; preds = %edge13067
  %v165301 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v165329)
  %t7866.not = icmp eq i64 %v165301, 0
  br i1 %t7866.not, label %block87369, label %block87363

edge13074:                                        ; preds = %block87369
  %t7868 = inttoptr i64 %v163116 to ptr
  %v165308 = load i64, ptr %t7868, align 1
  %t7869 = icmp eq i64 %v165308, 1
  br i1 %t7869, label %block87371, label %block87372

edge13079:                                        ; preds = %block87372
  %v165318 = load i64, ptr %ss1, align 8
  %v165319 = load i64, ptr %t606, align 8
  %v165320 = load i64, ptr %t607, align 8
  %.pre21729 = add i64 %v165318, %v164896
  %.pre21730 = inttoptr i64 %.pre21729 to ptr
  br label %block87371

edge13082:                                        ; preds = %block87366
  %t7893 = inttoptr i64 %v165297 to ptr
  %v165325 = load i64, ptr %t7893, align 1
  %v165326 = add i64 %v165325, 1
  store i64 %v165326, ptr %t7893, align 1
  br label %edge13093

edge13093:                                        ; preds = %block87366, %edge13082
  %t7908.not = or i1 %t7752.not, %t7890
  br i1 %t7908.not, label %edge13113, label %edge13106

edge13096:                                        ; preds = %edge12968.thread
  %t7899 = inttoptr i64 %v163116 to ptr
  %v165350 = load i64, ptr %t7899, align 1
  %t7906.not = icmp eq i64 %v165350, 0
  br i1 %t7906.not, label %edge13099, label %edge13130

edge13099:                                        ; preds = %edge13096
  %v165364 = call i64 @mech_compact_free(i64 %v0, i64 %v163116)
  %t7907.not = icmp eq i64 %v165364, 0
  br i1 %t7907.not, label %edge13130, label %block87387

edge13106:                                        ; preds = %edge13093
  %v165368 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v163116)
  %t7909.not = icmp eq i64 %v165368, 0
  br i1 %t7909.not, label %edge13113, label %block87387

edge13113:                                        ; preds = %edge13093, %edge13106
  %v165622 = add nuw nsw i64 %v163144, 1
  br label %block6664

edge13130:                                        ; preds = %edge12968, %edge12947, %edge13099, %edge13096
  %v16311615468 = phi i64 [ %v163116, %edge13096 ], [ %v163116, %edge12947 ], [ %v163116, %edge13099 ], [ 0, %edge12968 ]
  br i1 %t3927, label %edge13138, label %block6792

edge13138:                                        ; preds = %edge13130
  %t7917.not = icmp eq i64 %v7458022184, 0
  br i1 %t7917.not, label %block6792, label %edge13147

edge13147:                                        ; preds = %edge13138
  %v166499 = shl i64 %v7458022184, 3
  %v166502 = add i64 %v7457922180, -8
  %v166503 = add i64 %v166502, %v166499
  %t7922 = inttoptr i64 %v166503 to ptr
  %v166504 = load i64, ptr %t7922, align 1
  %98 = lshr i64 %v166504, 4
  %99 = and i64 %98, 268435455
  %100 = add nuw nsw i64 %99, 7
  br label %block6792

edge13163:                                        ; preds = %block6792
  %t7930.not = icmp eq i64 %v146746, 0
  br i1 %t7930.not, label %edge13168, label %block6816

edge13168:                                        ; preds = %edge13163
  %t7932.not = icmp eq i64 %v1471111098, 0
  %spec.select40 = select i1 %t7932.not, i64 0, i64 4
  br label %block6816

edge13194:                                        ; preds = %block6816
  %t7958.not = icmp eq i64 %v219119, 0
  br i1 %t7958.not, label %block2907.backedge, label %edge13250

block2907.backedge:                               ; preds = %edge13194, %edge13250, %edge13253
  br label %block2907

edge13195:                                        ; preds = %block6816
  br i1 %t7555.not, label %block88856, label %edge13198

edge13198:                                        ; preds = %edge13195
  %v167794 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54938)
  %t7939.not = icmp eq i64 %v167794, 0
  br i1 %t7939.not, label %block88856, label %block88851

edge13204:                                        ; preds = %block88856
  %v167798 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219242)
  %t7941.not = icmp eq i64 %v167798, 0
  br i1 %t7941.not, label %block88860, label %block88851

edge13208:                                        ; preds = %block88860
  %v167800 = load i64, ptr %t7636, align 1
  %v167801 = add i64 %v167800, -1
  store i64 %v167801, ptr %t7636, align 1
  %t7945.not = icmp eq i64 %v167801, 0
  br i1 %t7945.not, label %edge13211, label %edge13218

edge13211:                                        ; preds = %edge13208
  %v167802 = call i64 @mech_compact_free(i64 %v0, i64 %v54950)
  %t7946.not = icmp eq i64 %v167802, 0
  br i1 %t7946.not, label %edge13218, label %block88851

edge13218:                                        ; preds = %block88860, %edge13208, %edge13211
  br i1 %t3701.not, label %block88884, label %edge13221

edge13221:                                        ; preds = %edge13218
  %v167822 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54926)
  %t7948.not = icmp eq i64 %v167822, 0
  br i1 %t7948.not, label %block88884, label %block88851

edge13225:                                        ; preds = %block88884
  %v167824 = load i64, ptr %t4451.pre-phi, align 1
  %v167825 = add i64 %v167824, -1
  store i64 %v167825, ptr %t4451.pre-phi, align 1
  %t7952.not = icmp eq i64 %v167825, 0
  br i1 %t7952.not, label %edge13228, label %edge13236

edge13228:                                        ; preds = %edge13225
  %v167826 = call i64 @mech_compact_free(i64 %v0, i64 %v58156223382235022393)
  %t7953.not = icmp eq i64 %v167826, 0
  br i1 %t7953.not, label %edge13236, label %block88851

edge13236:                                        ; preds = %edge13228, %edge13225, %block88884
  %t7954.not = icmp eq i64 %v132184, 0
  br i1 %t7954.not, label %block88912, label %edge13239

edge13239:                                        ; preds = %edge13236
  %v167850 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v132184)
  %t7955.not = icmp eq i64 %v167850, 0
  br i1 %t7955.not, label %block88912, label %block88851

edge13245:                                        ; preds = %block88912
  %v167854 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219120)
  %t7957.not = icmp eq i64 %v167854, 0
  br i1 %t7957.not, label %edge13432, label %block88851

edge13250:                                        ; preds = %edge13194
  %t7959 = inttoptr i64 %v219119 to ptr
  %v167856 = load i64, ptr %t7959, align 1
  %v167857 = add i64 %v167856, -1
  store i64 %v167857, ptr %t7959, align 1
  %t7961.not = icmp eq i64 %v167857, 0
  br i1 %t7961.not, label %edge13253, label %block2907.backedge

edge13253:                                        ; preds = %edge13250
  %v167858 = call i64 @mech_compact_free(i64 %v0, i64 %v219119)
  %t7962.not = icmp eq i64 %v167858, 0
  br i1 %t7962.not, label %block2907.backedge, label %block88851

edge13432:                                        ; preds = %block88912, %edge13245
  %t8003.not = icmp eq i64 %v7458422199, 0
  br i1 %t8003.not, label %edge13540, label %edge13530

edge13530:                                        ; preds = %edge13432
  %t8004 = inttoptr i64 %v7458422199 to ptr
  %v172689 = load i64, ptr %t8004, align 1
  %v172690 = add i64 %v172689, 1
  store i64 %v172690, ptr %t8004, align 1
  br label %edge13540

edge13540:                                        ; preds = %edge13530, %edge13432
  %v172778 = add i64 %v139501, 3
  %t8006.not = icmp ugt i64 %v139501, -4
  br i1 %t8006.not, label %common.ret, label %edge13550

edge13550:                                        ; preds = %edge13540
  %v173613 = call i64 @mech_compact_unsigned_dot_product(i64 %v80928, i64 ptrtoint (ptr @mech_compact_table_22 to i64), i64 0, i64 320)
  %v173614 = add i64 %v173613, 10
  %t8017.not = icmp eq i64 %v80922, 0
  br i1 %t8017.not, label %edge13630, label %edge13554

edge13554:                                        ; preds = %edge13550
  %v173618 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v80922)
  %t8018.not = icmp eq i64 %v173618, 0
  br i1 %t8018.not, label %edge13630, label %edge13555

edge13555:                                        ; preds = %edge13554
  %t8019 = insertvalue { i64, i64 } poison, i64 %v173618, 0
  %t8020 = insertvalue { i64, i64 } %t8019, i64 6994, 1
  br label %common.ret

edge13630:                                        ; preds = %edge13554, %edge13550
  %t8050.not = icmp ugt i64 %v55001, -65535
  br i1 %t8050.not, label %common.ret, label %edge13634

edge13634:                                        ; preds = %edge13630
  %v175041 = add nuw i64 %v55001, 65534
  %t8062.not = icmp eq i64 %v55001, 0
  br i1 %t8062.not, label %common.ret, label %edge13645

edge13645:                                        ; preds = %edge13634
  %t8079 = icmp ult i64 %v55001, 2305843009213693952
  br i1 %t8079, label %edge13653, label %common.ret

edge13653:                                        ; preds = %edge13645
  %v175200 = sub nsw i64 5, %v219200
  %v175205 = and i64 %v175200, 7
  %v175050 = udiv i64 %v175041, 65535
  %101 = mul nuw nsw i64 %v175050, 40
  %v175391 = add nsw i64 %v175205, -5
  %v175396 = add nsw i64 %v175391, %101
  %v175406 = shl nuw i64 %v55001, 3
  %v175411 = add i64 %v175396, %v175406
  %t8083.not = icmp ult i64 %v175411, %v175396
  br i1 %t8083.not, label %common.ret, label %edge13658

edge13658:                                        ; preds = %edge13653
  %spec.select41 = call i64 @llvm.umin.i64(i64 %v172778, i64 %v173614)
  %t8093.not.not = icmp ugt i64 %v175411, %spec.select41
  br i1 %t8093.not.not, label %edge14047, label %edge13701

edge13701:                                        ; preds = %edge13658
  %t8106.not = icmp eq i64 %v132168, 0
  br i1 %t8106.not, label %block91131, label %edge13705

edge13705:                                        ; preds = %edge13701
  %v176207 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v132168)
  %t8107.not = icmp eq i64 %v176207, 0
  br i1 %t8107.not, label %block91131, label %block91127

edge13711:                                        ; preds = %block91131
  %v176211 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v7458422199)
  %t8109.not = icmp eq i64 %v176211, 0
  br i1 %t8109.not, label %block91135, label %block91127

edge13717:                                        ; preds = %block91135
  %v176215 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v132190)
  %t8111.not = icmp eq i64 %v176215, 0
  br i1 %t8111.not, label %block91139, label %block91127

edge13723:                                        ; preds = %block91139
  %v176219 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v132196)
  %t8113.not = icmp eq i64 %v176219, 0
  br i1 %t8113.not, label %edge13730, label %block91127

edge13730:                                        ; preds = %block91139, %edge13723
  br i1 %t23.not, label %block7018.preheader, label %edge13731

edge13731:                                        ; preds = %edge13730
  %v177522 = load i64, ptr %t910, align 1
  %v177523 = add i64 %v177522, 1
  store i64 %v177523, ptr %t910, align 1
  br label %block7018.preheader

block7018.preheader:                              ; preds = %edge13730, %edge13731
  br label %block7018

edge13755:                                        ; preds = %block7018
  %v177613 = sub nuw i64 %v54960, %v177596
  %spec.select42 = call i64 @llvm.umin.i64(i64 %v177613, i64 65535)
  %t8126 = icmp ult i64 %v177613, 65536
  %v177979 = select i1 %t8126, i64 %v54963, i64 65536
  %v177981 = shl i64 %v177979, %v177581
  %v177986 = or i64 %v177981, %v177578
  %t8132 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v177567, i64 1)
  %v178005 = extractvalue { i64, i1 } %t8132, 0
  %t8133 = extractvalue { i64, i1 } %t8132, 1
  br i1 %t8133, label %block91569, label %edge13784

edge13784:                                        ; preds = %edge13755
  %t8136.not = icmp ugt i64 %v178005, %v177568
  br i1 %t8136.not, label %edge13787, label %block91573

edge13787:                                        ; preds = %edge13784
  %t8138.not = icmp eq i64 %v177568, 0
  br i1 %t8138.not, label %block91576, label %edge13788

edge13788:                                        ; preds = %edge13787
  %102 = add i64 %v177568, -131073
  %v178015247 = icmp ult i64 %102, -126977
  br i1 %v178015247, label %edge13791, label %edge13800

edge13791:                                        ; preds = %edge13788
  %t8142 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v177568, i64 %v177568)
  %v178016 = extractvalue { i64, i1 } %t8142, 0
  %t8143 = extractvalue { i64, i1 } %t8142, 1
  br i1 %t8143, label %block91569, label %block91576

edge13800:                                        ; preds = %edge13788
  %t8150 = mul nuw nsw i64 %v177568, 3
  %t8154 = add nuw nsw i64 %t8150, 1
  %v178033 = lshr i64 %t8154, 1
  br label %block91576

edge13804:                                        ; preds = %block91573
  %t8160 = inttoptr i64 %v177571 to ptr
  %v178041 = load i64, ptr %t8160, align 1
  %t8161 = icmp eq i64 %v178041, 1
  %t8162 = icmp ule i64 %v178010, %v177568
  %v178044248 = and i1 %t8162, %t8161
  br i1 %v178044248, label %block91583, label %block91584

edge13809:                                        ; preds = %block91584
  %v178051 = load i64, ptr %ss1, align 8
  %v178052 = load i64, ptr %t606, align 8
  %v178053 = load i64, ptr %t607, align 8
  br label %block91583

edge13811:                                        ; preds = %block91583
  %t8185 = inttoptr i64 %v178040 to ptr
  %v178060 = load i64, ptr %t8185, align 1
  %v178061 = add i64 %v178060, 1
  store i64 %v178061, ptr %t8185, align 1
  br label %block91588

edge13816:                                        ; preds = %block91588
  %v178066 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v177571)
  %t8189.not = icmp eq i64 %v178066, 0
  br i1 %t8189.not, label %edge13820, label %block91569

edge13820:                                        ; preds = %edge13816, %block91588
  %t8192 = icmp ugt i64 %v177581, 5
  br i1 %t8192, label %edge13827, label %edge13824

edge13824:                                        ; preds = %edge13820, %block91730, %edge13860
  %v178324 = phi i64 [ %v178273, %block91730 ], [ %v178273, %edge13860 ], [ %v178040, %edge13820 ]
  %v178327 = phi i64 [ %v178268, %block91730 ], [ %v178268, %edge13860 ], [ %v178035, %edge13820 ]
  %v178328 = phi i64 [ %v178238, %block91730 ], [ %v178238, %edge13860 ], [ %v178005, %edge13820 ]
  %v178329 = phi i64 [ %v178270, %block91730 ], [ %v178270, %edge13860 ], [ %v178037, %edge13820 ]
  %t8255 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v178328, i64 2)
  %v178336 = extractvalue { i64, i1 } %t8255, 0
  %t8256 = extractvalue { i64, i1 } %t8255, 1
  br i1 %t8256, label %block91747, label %edge13872

edge13827:                                        ; preds = %edge13820
  %t8195 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v178005, i64 1)
  %v178238 = extractvalue { i64, i1 } %t8195, 0
  %t8196 = extractvalue { i64, i1 } %t8195, 1
  br i1 %t8196, label %block91711, label %edge13828

edge13828:                                        ; preds = %edge13827
  %t8199.not = icmp ugt i64 %v178238, %v178037
  br i1 %t8199.not, label %edge13831, label %block91715

edge13831:                                        ; preds = %edge13828
  %t8201.not = icmp eq i64 %v178037, 0
  br i1 %t8201.not, label %block91718, label %edge13832

edge13832:                                        ; preds = %edge13831
  %103 = add i64 %v178037, -131073
  %v178248251 = icmp ult i64 %103, -126977
  br i1 %v178248251, label %edge13835, label %edge13844

edge13835:                                        ; preds = %edge13832
  %t8205 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v178037, i64 %v178037)
  %v178249 = extractvalue { i64, i1 } %t8205, 0
  %t8206 = extractvalue { i64, i1 } %t8205, 1
  br i1 %t8206, label %block91711, label %block91718

edge13844:                                        ; preds = %edge13832
  %t8213 = mul nuw nsw i64 %v178037, 3
  %t8217 = add nuw nsw i64 %t8213, 1
  %v178266 = lshr i64 %t8217, 1
  br label %block91718

edge13848:                                        ; preds = %block91715
  %t8223 = inttoptr i64 %v178040 to ptr
  %v178274 = load i64, ptr %t8223, align 1
  %t8224 = icmp eq i64 %v178274, 1
  %t8225 = icmp ule i64 %v178243, %v178037
  %v178277252 = and i1 %t8225, %t8224
  br i1 %v178277252, label %block91725, label %block91726

edge13853:                                        ; preds = %block91726
  %v178284 = load i64, ptr %ss1, align 8
  %v178285 = load i64, ptr %t606, align 8
  %v178286 = load i64, ptr %t607, align 8
  br label %block91725

edge13855:                                        ; preds = %block91725
  %t8248 = inttoptr i64 %v178273 to ptr
  %v178293 = load i64, ptr %t8248, align 1
  %v178294 = add i64 %v178293, 1
  store i64 %v178294, ptr %t8248, align 1
  br label %block91730

edge13860:                                        ; preds = %block91730
  %v178299 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v178040)
  %t8252.not = icmp eq i64 %v178299, 0
  br i1 %t8252.not, label %edge13824, label %block91711

edge13872:                                        ; preds = %edge13824
  %t8259.not = icmp ugt i64 %v178336, %v178329
  br i1 %t8259.not, label %edge13875, label %block91752

edge13875:                                        ; preds = %edge13872
  %t8261.not = icmp eq i64 %v178329, 0
  br i1 %t8261.not, label %block91755, label %edge13876

edge13876:                                        ; preds = %edge13875
  %104 = add i64 %v178329, -131073
  %v178346255 = icmp ult i64 %104, -126977
  br i1 %v178346255, label %edge13879, label %edge13888

edge13879:                                        ; preds = %edge13876
  %t8265 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v178329, i64 %v178329)
  %v178347 = extractvalue { i64, i1 } %t8265, 0
  %t8266 = extractvalue { i64, i1 } %t8265, 1
  br i1 %t8266, label %block91747, label %block91755

edge13888:                                        ; preds = %edge13876
  %t8273 = mul nuw nsw i64 %v178329, 3
  %t8277 = add nuw nsw i64 %t8273, 1
  %v178364 = lshr i64 %t8277, 1
  br label %block91755

edge13892:                                        ; preds = %block91752
  %t8283 = inttoptr i64 %v178324 to ptr
  %v178372 = load i64, ptr %t8283, align 1
  %t8284 = icmp eq i64 %v178372, 1
  %t8285 = icmp ule i64 %v178341, %v178329
  %v178375256 = and i1 %t8285, %t8284
  br i1 %v178375256, label %edge13899, label %block91763

edge13897:                                        ; preds = %block91763
  %v178382 = load i64, ptr %ss1, align 8
  %v178383 = load i64, ptr %t606, align 8
  %v178384 = load i64, ptr %t607, align 8
  br label %edge13899

edge13899:                                        ; preds = %edge13892, %edge13897
  %v178366 = phi i64 [ %v178382, %edge13897 ], [ %v178327, %edge13892 ]
  %v178368 = phi i64 [ %v178383, %edge13897 ], [ %v178329, %edge13892 ]
  %v178371 = phi i64 [ %v178384, %edge13897 ], [ %v178324, %edge13892 ]
  %v178387 = trunc i64 %spec.select42 to i8
  %v178389 = add i64 %v178366, %v178328
  %t8304 = inttoptr i64 %v178389 to ptr
  store i8 %v178387, ptr %t8304, align 1
  %v178391 = lshr i64 %spec.select42, 8
  %v178392 = trunc nuw i64 %v178391 to i8
  %v178393 = add nuw i64 %v178328, 1
  %v178394 = add i64 %v178393, %v178366
  %t8305 = inttoptr i64 %v178394 to ptr
  store i8 %v178392, ptr %t8305, align 1
  %t8306 = icmp eq i64 %v178324, %v178371
  %t8308.not257 = icmp eq i64 %v178371, 0
  %t8308.not = or i1 %t8306, %t8308.not257
  br i1 %t8308.not, label %block91783, label %edge13909

edge13909:                                        ; preds = %edge13899
  %t8309 = inttoptr i64 %v178371 to ptr
  %v178427 = load i64, ptr %t8309, align 1
  %v178428 = add i64 %v178427, 1
  store i64 %v178428, ptr %t8309, align 1
  br label %block91783

edge13914:                                        ; preds = %block91783
  %v178433 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v178324)
  %t8313.not = icmp eq i64 %v178433, 0
  br i1 %t8313.not, label %edge13918, label %block91747

edge13918:                                        ; preds = %edge13914, %block91783
  %v178597 = xor i64 %spec.select42, -1
  %t8316 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v178336, i64 2)
  %v178606 = extractvalue { i64, i1 } %t8316, 0
  %t8317 = extractvalue { i64, i1 } %t8316, 1
  br i1 %t8317, label %block91789, label %edge13922

edge13922:                                        ; preds = %edge13918
  %t8320.not = icmp ugt i64 %v178606, %v178368
  br i1 %t8320.not, label %edge13925, label %block91794

edge13925:                                        ; preds = %edge13922
  %t8322.not = icmp eq i64 %v178368, 0
  br i1 %t8322.not, label %block91797, label %edge13926

edge13926:                                        ; preds = %edge13925
  %105 = add i64 %v178368, -131073
  %v178616259 = icmp ult i64 %105, -126977
  br i1 %v178616259, label %edge13929, label %edge13938

edge13929:                                        ; preds = %edge13926
  %t8326 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v178368, i64 %v178368)
  %v178617 = extractvalue { i64, i1 } %t8326, 0
  %t8327 = extractvalue { i64, i1 } %t8326, 1
  br i1 %t8327, label %block91789, label %block91797

edge13938:                                        ; preds = %edge13926
  %t8334 = mul nuw nsw i64 %v178368, 3
  %t8338 = add nuw nsw i64 %t8334, 1
  %v178634 = lshr i64 %t8338, 1
  br label %block91797

edge13942:                                        ; preds = %block91794
  %t8344 = inttoptr i64 %v178371 to ptr
  %v178642 = load i64, ptr %t8344, align 1
  %t8345 = icmp eq i64 %v178642, 1
  %t8346 = icmp ule i64 %v178611, %v178368
  %v178645260 = and i1 %t8346, %t8345
  br i1 %v178645260, label %edge13949, label %block91805

edge13947:                                        ; preds = %block91805
  %v178652 = load i64, ptr %ss1, align 8
  %v178653 = load i64, ptr %t606, align 8
  %v178654 = load i64, ptr %t607, align 8
  br label %edge13949

edge13949:                                        ; preds = %edge13942, %edge13947
  %v178636 = phi i64 [ %v178652, %edge13947 ], [ %v178366, %edge13942 ]
  %v178638 = phi i64 [ %v178653, %edge13947 ], [ %v178368, %edge13942 ]
  %v178641 = phi i64 [ %v178654, %edge13947 ], [ %v178371, %edge13942 ]
  %v178657 = trunc i64 %v178597 to i8
  %v178659 = add i64 %v178636, %v178336
  %t8365 = inttoptr i64 %v178659 to ptr
  store i8 %v178657, ptr %t8365, align 1
  %v178661 = lshr i64 %v178597, 8
  %v178662 = trunc i64 %v178661 to i8
  %v178663 = add nuw i64 %v178336, 1
  %v178664 = add i64 %v178663, %v178636
  %t8366 = inttoptr i64 %v178664 to ptr
  store i8 %v178662, ptr %t8366, align 1
  %t8367 = icmp eq i64 %v178371, %v178641
  %t8369.not261 = icmp eq i64 %v178641, 0
  %t8369.not = or i1 %t8367, %t8369.not261
  br i1 %t8369.not, label %block91825, label %edge13959

edge13959:                                        ; preds = %edge13949
  %t8370 = inttoptr i64 %v178641 to ptr
  %v178697 = load i64, ptr %t8370, align 1
  %v178698 = add i64 %v178697, 1
  store i64 %v178698, ptr %t8370, align 1
  br label %block91825

edge13964:                                        ; preds = %block91825
  %v178703 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v178371)
  %t8374.not = icmp eq i64 %v178703, 0
  br i1 %t8374.not, label %edge13968, label %block91789

edge13968:                                        ; preds = %edge13964, %block91825
  br i1 %t1068, label %block91830, label %edge13970

edge13970:                                        ; preds = %edge13968
  %t8379 = call i64 @llvm.umin.i64(i64 %v177596, i64 %v1397)
  %v178713 = sub i64 %v1397, %t8379
  %t8380 = call i64 @llvm.umin.i64(i64 %spec.select42, i64 %v178713)
  %t8381 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v178606, i64 %t8380)
  %v178715 = extractvalue { i64, i1 } %t8381, 0
  %t8382 = extractvalue { i64, i1 } %t8381, 1
  br i1 %t8382, label %block91830, label %edge13972

edge13972:                                        ; preds = %edge13970
  %t8385.not = icmp ugt i64 %v178715, %v178638
  br i1 %t8385.not, label %edge13975, label %block91835

edge13975:                                        ; preds = %edge13972
  %t8387.not = icmp eq i64 %v178638, 0
  br i1 %t8387.not, label %block91838, label %edge13976

edge13976:                                        ; preds = %edge13975
  %106 = add i64 %v178638, -131073
  %v178725263 = icmp ult i64 %106, -126977
  br i1 %v178725263, label %edge13979, label %edge13988

edge13979:                                        ; preds = %edge13976
  %t8391 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v178638, i64 %v178638)
  %v178726 = extractvalue { i64, i1 } %t8391, 0
  %t8392 = extractvalue { i64, i1 } %t8391, 1
  br i1 %t8392, label %block91830, label %block91838

edge13988:                                        ; preds = %edge13976
  %t8399 = mul nuw nsw i64 %v178638, 3
  %t8403 = add nuw nsw i64 %t8399, 1
  %v178743 = lshr i64 %t8403, 1
  br label %block91838

edge13992:                                        ; preds = %block91835
  %t8409 = inttoptr i64 %v178641 to ptr
  %v178751 = load i64, ptr %t8409, align 1
  %t8410 = icmp eq i64 %v178751, 1
  %t8411 = icmp ule i64 %v178720, %v178638
  %v178754264 = and i1 %t8411, %t8410
  br i1 %v178754264, label %block91845, label %block91846

edge13997:                                        ; preds = %block91846
  %v178761 = load i64, ptr %ss1, align 8
  %v178762 = load i64, ptr %t606, align 8
  %v178763 = load i64, ptr %t607, align 8
  br label %block91845

edge13999:                                        ; preds = %block91845
  %v178767 = add i64 %t8379, %v1396
  %v178765 = add i64 %v178745, %v178606
  %v178769 = call i64 @memmove(i64 %v178765, i64 %v178767, i64 %t8380)
  br label %block91850

edge14002:                                        ; preds = %block91850
  %t8434 = inttoptr i64 %v178750 to ptr
  %v178772 = load i64, ptr %t8434, align 1
  %v178773 = add i64 %v178772, 1
  store i64 %v178773, ptr %t8434, align 1
  br label %block91852

edge14007:                                        ; preds = %block91852
  %v178778 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v178641)
  %t8438.not = icmp eq i64 %v178778, 0
  br i1 %t8438.not, label %edge14011, label %block91830

edge14011:                                        ; preds = %edge14007, %block91852
  %v178783 = add i64 %spec.select42, %v177596
  %t8441.not = icmp ult i64 %v178783, %v177596
  br i1 %t8441.not, label %common.ret, label %edge14014

edge14014:                                        ; preds = %edge14011
  %t8445 = icmp eq i64 %v178783, %v54960
  br i1 %t8445, label %edge14016, label %block7018

edge14016:                                        ; preds = %edge14014
  br i1 %t23.not, label %edge15681, label %edge14018

edge14018:                                        ; preds = %edge14016
  %v178798 = load i64, ptr %t910, align 1
  %v178799 = add i64 %v178798, -1
  store i64 %v178799, ptr %t910, align 1
  %t8451.not = icmp eq i64 %v178799, 0
  br i1 %t8451.not, label %edge14021, label %edge15681

edge14021:                                        ; preds = %edge14018
  %v178800 = call i64 @mech_compact_free(i64 %v0, i64 %v1401)
  %t8452.not = icmp eq i64 %v178800, 0
  br i1 %t8452.not, label %edge15681, label %edge14022

edge14022:                                        ; preds = %edge14021
  %t8453 = insertvalue { i64, i64 } poison, i64 %v178800, 0
  %t8454 = insertvalue { i64, i64 } %t8453, i64 7189, 1
  br label %common.ret

edge14047:                                        ; preds = %edge13658
  %t8096.not.not = icmp ugt i64 %v173614, %v172778
  br i1 %t8096.not.not, label %edge14158, label %edge14049

edge14049:                                        ; preds = %edge14047
  %t8458.not = icmp eq i64 %v132190, 0
  br i1 %t8458.not, label %block92035, label %edge14053

edge14053:                                        ; preds = %edge14049
  %v179037 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v132190)
  %t8459.not = icmp eq i64 %v179037, 0
  br i1 %t8459.not, label %block92035, label %block92031

edge14059:                                        ; preds = %block92035
  %v179041 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v132196)
  %t8461.not = icmp eq i64 %v179041, 0
  br i1 %t8461.not, label %edge14072, label %block92031

edge14072:                                        ; preds = %edge14059, %block92035
  %107 = or i64 %v54963, 2
  %108 = shl i64 %107, %v219200
  %v179247 = or i64 %108, %v219199
  %v179252 = add nuw nsw i64 %v219200, 3
  %v179260 = lshr i64 %v179252, 3
  %t8479 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v219194, i64 %v179260)
  %v179280 = extractvalue { i64, i1 } %t8479, 0
  %t8480 = extractvalue { i64, i1 } %t8479, 1
  br i1 %t8480, label %block92183, label %edge14087

edge14087:                                        ; preds = %edge14072
  %t8483.not = icmp ugt i64 %v179280, %v219195
  br i1 %t8483.not, label %edge14090, label %block92188

edge14090:                                        ; preds = %edge14087
  %t8485.not = icmp eq i64 %v219195, 0
  br i1 %t8485.not, label %block92191, label %edge14091

edge14091:                                        ; preds = %edge14090
  %109 = add i64 %v219195, -131073
  %v179290235 = icmp ult i64 %109, -126977
  br i1 %v179290235, label %edge14094, label %edge14103

edge14094:                                        ; preds = %edge14091
  %t8489 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v219195, i64 %v219195)
  %v179291 = extractvalue { i64, i1 } %t8489, 0
  %t8490 = extractvalue { i64, i1 } %t8489, 1
  br i1 %t8490, label %block92183, label %block92191

edge14103:                                        ; preds = %edge14091
  %t8497 = mul nuw nsw i64 %v219195, 3
  %t8501 = add nuw nsw i64 %t8497, 1
  %v179308 = lshr i64 %t8501, 1
  br label %block92191

edge14107:                                        ; preds = %block92188
  %t8507 = inttoptr i64 %v219198 to ptr
  %v179316 = load i64, ptr %t8507, align 1
  %t8508 = icmp eq i64 %v179316, 1
  %t8509 = icmp ule i64 %v179285, %v219195
  %v179319236 = and i1 %t8509, %t8508
  br i1 %v179319236, label %block92198, label %block92199

edge14112:                                        ; preds = %block92199
  %v179326 = load i64, ptr %ss1, align 8
  %v179327 = load i64, ptr %t606, align 8
  %v179328 = load i64, ptr %t607, align 8
  br label %block92198

edge14114:                                        ; preds = %block92198
  %v179331 = trunc i64 %v179247 to i8
  %v179333 = add i64 %v179310, %v219194
  %t8530 = inttoptr i64 %v179333 to ptr
  store i8 %v179331, ptr %t8530, align 1
  br label %block92217

edge14134:                                        ; preds = %block92217
  %t8549 = inttoptr i64 %v179315 to ptr
  %v179371 = load i64, ptr %t8549, align 1
  %v179372 = add i64 %v179371, 1
  store i64 %v179372, ptr %t8549, align 1
  br label %block92219

edge14139:                                        ; preds = %block92219
  %v179377 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219198)
  %t8553.not = icmp eq i64 %v179377, 0
  br i1 %t8553.not, label %edge14064.thread, label %block92183

edge14064.thread:                                 ; preds = %block92219, %edge14139
  %v179383 = and i64 %v179252, -8
  %v179394 = lshr i64 %v179247, %v179383
  %v179399 = and i64 %v179252, 7
  %t8950.not = icmp eq i64 %v132168, 0
  br i1 %t8950.not, label %edge14805, label %edge14790

edge14158:                                        ; preds = %edge14047
  %t8568 = icmp ugt i64 %v132198, 256
  br i1 %t8568, label %edge14166, label %common.ret

edge14166:                                        ; preds = %edge14158
  %t8577.not = icmp eq i64 %v132199, 0
  br i1 %t8577.not, label %common.ret, label %edge14176

edge14176:                                        ; preds = %edge14166
  %t8586 = icmp ugt i64 %v132200, 3
  br i1 %t8586, label %edge14240, label %common.ret

edge14240:                                        ; preds = %edge14176
  %v179528 = add nuw nsw i64 %v219200, 13
  %110 = or i64 %v54963, 4
  %111 = shl i64 %110, %v219200
  %v179475 = add i64 %v132198, -257
  %v179464 = add nuw nsw i64 %v219200, 3
  %v179486 = shl i64 %v179475, %v179464
  %v179507 = add i64 %v132199, -1
  %v179496 = add nuw nsw i64 %v219200, 8
  %v179518 = shl i64 %v179507, %v179496
  %v179539 = add i64 %v132200, -4
  %v179550 = shl i64 %v179539, %v179528
  %v179560 = add nuw nsw i64 %v219200, 17
  %v180334 = add i64 %v132185, 64
  %t8600 = inttoptr i64 %v180334 to ptr
  %t8601 = load i32, ptr %t8600, align 1
  %112 = lshr i32 %t8601, 16
  %113 = zext nneg i32 %112 to i64
  %v180376 = shl nuw nsw i64 %113, %v179560
  %114 = or i64 %111, %v179486
  %115 = or i64 %114, %v179518
  %116 = or i64 %115, %v179550
  %117 = or i64 %116, %v180376
  %v180381 = or i64 %117, %v219199
  %v180386 = add nuw nsw i64 %v219200, 20
  %v180394 = lshr i64 %v180386, 3
  %t8613 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v219194, i64 %v180394)
  %v180414 = extractvalue { i64, i1 } %t8613, 0
  %t8614 = extractvalue { i64, i1 } %t8613, 1
  br i1 %t8614, label %block92607, label %edge14256

edge14256:                                        ; preds = %edge14240
  %t8617.not = icmp ugt i64 %v180414, %v219195
  br i1 %t8617.not, label %edge14259, label %block92612

edge14259:                                        ; preds = %edge14256
  %t8619.not = icmp eq i64 %v219195, 0
  br i1 %t8619.not, label %block92615, label %edge14260

edge14260:                                        ; preds = %edge14259
  %118 = add i64 %v219195, -131073
  %v180424223 = icmp ult i64 %118, -126977
  br i1 %v180424223, label %edge14263, label %edge14272

edge14263:                                        ; preds = %edge14260
  %t8623 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v219195, i64 %v219195)
  %v180425 = extractvalue { i64, i1 } %t8623, 0
  %t8624 = extractvalue { i64, i1 } %t8623, 1
  br i1 %t8624, label %block92607, label %block92615

edge14272:                                        ; preds = %edge14260
  %t8631 = mul nuw nsw i64 %v219195, 3
  %t8635 = add nuw nsw i64 %t8631, 1
  %v180442 = lshr i64 %t8635, 1
  br label %block92615

edge14276:                                        ; preds = %block92612
  %t8641 = inttoptr i64 %v219198 to ptr
  %v180450 = load i64, ptr %t8641, align 1
  %t8642 = icmp eq i64 %v180450, 1
  %t8643 = icmp ule i64 %v180419, %v219195
  %v180453224 = and i1 %t8643, %t8642
  br i1 %v180453224, label %edge14283, label %block92623

edge14281:                                        ; preds = %block92623
  %v180460 = load i64, ptr %ss1, align 8
  %v180461 = load i64, ptr %t606, align 8
  %v180462 = load i64, ptr %t607, align 8
  br label %edge14283

edge14283:                                        ; preds = %edge14281, %edge14276
  %v180444 = phi i64 [ %v180460, %edge14281 ], [ %v219193, %edge14276 ]
  %v180446 = phi i64 [ %v180461, %edge14281 ], [ %v219195, %edge14276 ]
  %v180449 = phi i64 [ %v180462, %edge14281 ], [ %v219198, %edge14276 ]
  %v180465 = trunc i64 %v180381 to i8
  %v180467 = add i64 %v180444, %v219194
  %t8664 = inttoptr i64 %v180467 to ptr
  store i8 %v180465, ptr %t8664, align 1
  %v180469 = lshr i64 %v180381, 8
  %v180470 = trunc i64 %v180469 to i8
  %v180471 = add i64 %v219194, 1
  %v180472 = add i64 %v180471, %v180444
  %t8667 = inttoptr i64 %v180472 to ptr
  store i8 %v180470, ptr %t8667, align 1
  %t8668 = icmp ugt i64 %v219200, 3
  br i1 %t8668, label %edge14289, label %block92641

edge14289:                                        ; preds = %edge14283
  %v180474 = lshr i64 %v180381, 16
  %v180475 = trunc i64 %v180474 to i8
  %v180476 = add i64 %v219194, 2
  %v180477 = add i64 %v180476, %v180444
  %t8670 = inttoptr i64 %v180477 to ptr
  store i8 %v180475, ptr %t8670, align 1
  br label %block92641

edge14307:                                        ; preds = %block92641
  %t8689 = inttoptr i64 %v180449 to ptr
  %v180505 = load i64, ptr %t8689, align 1
  %v180506 = add i64 %v180505, 1
  store i64 %v180506, ptr %t8689, align 1
  br label %block92643

edge14312:                                        ; preds = %block92643
  %v180511 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219198)
  %t8693.not = icmp eq i64 %v180511, 0
  br i1 %t8693.not, label %edge14316, label %block92607

edge14316:                                        ; preds = %edge14312, %block92643
  %v180517 = and i64 %v180386, -8
  %v180528 = lshr i64 %v180381, %v180517
  %v180533 = and i64 %v180386, 7
  %t8698.not = icmp eq i64 %v132190, 0
  br i1 %t8698.not, label %edge14335, label %edge14323

edge14323:                                        ; preds = %edge14316
  %t8699 = inttoptr i64 %v132190 to ptr
  %v180542 = load i64, ptr %t8699, align 1
  %v180543 = add i64 %v180542, 1
  store i64 %v180543, ptr %t8699, align 1
  br label %edge14335

edge14335:                                        ; preds = %edge14323, %edge14316
  %119 = sub nuw nsw i64 -4, %v180533
  %120 = udiv i64 %119, 3
  %invariant.umin = call i64 @llvm.umin.i64(i64 %v132200, i64 19)
  %121 = add nuw nsw i64 %120, 2
  br label %block7199

edge14372:                                        ; preds = %block7199
  %v181051 = add i64 %v180623, ptrtoint (ptr @mech_compact_table_15 to i64)
  %t8707 = inttoptr i64 %v181051 to ptr
  %t8708 = load i8, ptr %t8707, align 1
  %v181052 = zext i8 %t8708 to i64
  br label %edge14384

edge14384:                                        ; preds = %block7199, %edge14372
  %v181091 = phi i64 [ 0, %block7199 ], [ %v181052, %edge14372 ]
  %exitcond21672.not = icmp eq i64 %v180623, %v132200
  br i1 %exitcond21672.not, label %edge14423, label %edge14397

edge14397:                                        ; preds = %edge14384
  %t8711 = icmp ult i64 %v181091, %v132186
  br i1 %t8711, label %edge14405, label %edge14435

edge14405:                                        ; preds = %edge14397
  %v181500 = shl nuw nsw i64 %v181091, 2
  %v181501 = add i64 %v181500, %v132185
  %t8714 = inttoptr i64 %v181501 to ptr
  %t8715 = load i32, ptr %t8714, align 1
  %122 = lshr i32 %t8715, 16
  %123 = zext nneg i32 %122 to i64
  br label %edge14435

edge14423:                                        ; preds = %edge14384
  br i1 %t8698.not, label %edge14455, label %edge14424

edge14424:                                        ; preds = %edge14423
  %t8721 = inttoptr i64 %v132190 to ptr
  %v181573 = load i64, ptr %t8721, align 1
  %v181574 = add i64 %v181573, -1
  store i64 %v181574, ptr %t8721, align 1
  %t8723.not = icmp eq i64 %v181574, 0
  br i1 %t8723.not, label %edge14427, label %edge14455

edge14427:                                        ; preds = %edge14424
  %v181575 = call i64 @mech_compact_free(i64 %v0, i64 %v132190)
  %t8724.not = icmp eq i64 %v181575, 0
  br i1 %t8724.not, label %edge14455, label %edge14428

edge14428:                                        ; preds = %edge14427
  %t8725 = insertvalue { i64, i64 } poison, i64 %v181575, 0
  %t8726 = insertvalue { i64, i64 } %t8725, i64 7394, 1
  br label %common.ret

edge14435:                                        ; preds = %edge14397, %edge14405
  %v181523 = phi i64 [ %123, %edge14405 ], [ 0, %edge14397 ]
  %v181560 = and i64 %v180629, 63
  %v181561 = shl i64 %v181523, %v181560
  %v181566 = or i64 %v181561, %v180626
  %v181751 = add nuw nsw i64 %v180623, 1
  %v181759 = add nuw i64 %v180629, 3
  %exitcond21673 = icmp eq i64 %v180623, %121
  br i1 %exitcond21673, label %common.ret, label %block7199

edge14455:                                        ; preds = %edge14423, %edge14424, %edge14427
  %v181820 = lshr i64 %v180629, 3
  %v181825 = and i64 %v181820, 255
  %t8733 = icmp samesign ult i64 %v181825, 9
  br i1 %t8733, label %edge14461, label %block93504

edge14461:                                        ; preds = %edge14455
  %t8735 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v180414, i64 %v181825)
  %v181840 = extractvalue { i64, i1 } %t8735, 0
  %t8736 = extractvalue { i64, i1 } %t8735, 1
  br i1 %t8736, label %block93504, label %edge14463

edge14463:                                        ; preds = %edge14461
  %t8739.not = icmp ugt i64 %v181840, %v180446
  br i1 %t8739.not, label %edge14466, label %block93509

edge14466:                                        ; preds = %edge14463
  %t8741.not = icmp eq i64 %v180446, 0
  br i1 %t8741.not, label %block93512, label %edge14467

edge14467:                                        ; preds = %edge14466
  %124 = add i64 %v180446, -131073
  %v181850227 = icmp ult i64 %124, -126977
  br i1 %v181850227, label %edge14470, label %edge14479

edge14470:                                        ; preds = %edge14467
  %t8745 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v180446, i64 %v180446)
  %v181851 = extractvalue { i64, i1 } %t8745, 0
  %t8746 = extractvalue { i64, i1 } %t8745, 1
  br i1 %t8746, label %block93504, label %block93512

edge14479:                                        ; preds = %edge14467
  %t8753 = mul nuw nsw i64 %v180446, 3
  %t8757 = add nuw nsw i64 %t8753, 1
  %v181868 = lshr i64 %t8757, 1
  br label %block93512

edge14483:                                        ; preds = %block93509
  %t8763 = inttoptr i64 %v180449 to ptr
  %v181876 = load i64, ptr %t8763, align 1
  %t8764 = icmp eq i64 %v181876, 1
  %t8765 = icmp ule i64 %v181845, %v180446
  %v181879228 = and i1 %t8765, %t8764
  br i1 %v181879228, label %block93519, label %block93520

edge14488:                                        ; preds = %block93520
  %v181886 = load i64, ptr %ss1, align 8
  %v181887 = load i64, ptr %t606, align 8
  %v181888 = load i64, ptr %t607, align 8
  br label %block93519

edge14490:                                        ; preds = %block93519
  %v181891 = trunc i64 %v180626 to i8
  %v181893 = add i64 %v181870, %v180414
  %t8786 = inttoptr i64 %v181893 to ptr
  store i8 %v181891, ptr %t8786, align 1
  %t8787.not = icmp eq i64 %v181825, 1
  br i1 %t8787.not, label %block93538, label %edge14493

edge14493:                                        ; preds = %edge14490
  %v181895 = lshr i64 %v180626, 8
  %v181896 = trunc i64 %v181895 to i8
  %v181897 = add i64 %v180414, 1
  %v181898 = add i64 %v181897, %v181870
  %t8789 = inttoptr i64 %v181898 to ptr
  store i8 %v181896, ptr %t8789, align 1
  %t8790 = icmp samesign ugt i64 %v181825, 2
  br i1 %t8790, label %edge14496, label %block93538

edge14496:                                        ; preds = %edge14493
  %v181900 = lshr i64 %v180626, 16
  %v181901 = trunc i64 %v181900 to i8
  %v181902 = add i64 %v180414, 2
  %v181903 = add i64 %v181902, %v181870
  %t8792 = inttoptr i64 %v181903 to ptr
  store i8 %v181901, ptr %t8792, align 1
  %t8793.not = icmp eq i64 %v181825, 3
  br i1 %t8793.not, label %block93538, label %edge14499

edge14499:                                        ; preds = %edge14496
  %v181905 = lshr i64 %v180626, 24
  %v181906 = trunc i64 %v181905 to i8
  %v181907 = add i64 %v180414, 3
  %v181908 = add i64 %v181907, %v181870
  %t8795 = inttoptr i64 %v181908 to ptr
  store i8 %v181906, ptr %t8795, align 1
  %t8796 = icmp samesign ugt i64 %v181825, 4
  br i1 %t8796, label %edge14502, label %block93538

edge14502:                                        ; preds = %edge14499
  %v181910 = lshr i64 %v180626, 32
  %v181911 = trunc i64 %v181910 to i8
  %v181912 = add i64 %v180414, 4
  %v181913 = add i64 %v181912, %v181870
  %t8798 = inttoptr i64 %v181913 to ptr
  store i8 %v181911, ptr %t8798, align 1
  %t8799.not = icmp eq i64 %v181825, 5
  br i1 %t8799.not, label %block93538, label %edge14505

edge14505:                                        ; preds = %edge14502
  %v181915 = lshr i64 %v180626, 40
  %v181916 = trunc i64 %v181915 to i8
  %v181917 = add i64 %v180414, 5
  %v181918 = add i64 %v181917, %v181870
  %t8801 = inttoptr i64 %v181918 to ptr
  store i8 %v181916, ptr %t8801, align 1
  %t8802 = icmp samesign ugt i64 %v181825, 6
  br i1 %t8802, label %edge14508, label %block93538

edge14508:                                        ; preds = %edge14505
  %v181920 = lshr i64 %v180626, 48
  %v181921 = trunc i64 %v181920 to i8
  %v181922 = add i64 %v180414, 6
  %v181923 = add i64 %v181922, %v181870
  %t8804 = inttoptr i64 %v181923 to ptr
  store i8 %v181921, ptr %t8804, align 1
  %t8805 = icmp eq i64 %v181825, 8
  br i1 %t8805, label %edge14511, label %block93538

edge14511:                                        ; preds = %edge14508
  %v181925 = lshr i64 %v180626, 56
  %v181926 = trunc nuw i64 %v181925 to i8
  %v181927 = add i64 %v180414, 7
  %v181928 = add i64 %v181927, %v181870
  %t8807 = inttoptr i64 %v181928 to ptr
  store i8 %v181926, ptr %t8807, align 1
  br label %block93538

edge14514:                                        ; preds = %block93538
  %t8811 = inttoptr i64 %v181875 to ptr
  %v181931 = load i64, ptr %t8811, align 1
  %v181932 = add i64 %v181931, 1
  store i64 %v181932, ptr %t8811, align 1
  br label %block93540

edge14519:                                        ; preds = %block93540
  %v181937 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v180449)
  %t8815.not = icmp eq i64 %v181937, 0
  br i1 %t8815.not, label %edge14523, label %block93504

edge14523:                                        ; preds = %edge14519, %block93540
  %v181953 = and i64 %v180629, 56
  %v181954 = lshr i64 %v180626, %v181953
  br label %block7305

edge14572:                                        ; preds = %block7305
  %t8938.not = icmp eq i64 %v132196, 0
  br i1 %t8938.not, label %block94361, label %edge14745

edge14582:                                        ; preds = %block7305
  %t8823 = icmp ult i64 %v182442, 320
  br i1 %t8823, label %edge14590, label %edge14599

edge14590:                                        ; preds = %edge14582
  %v182853 = shl nuw nsw i64 %v182442, 2
  %v182854 = add i64 %v182853, %v132191
  %t8826 = inttoptr i64 %v182854 to ptr
  %t8827 = load i32, ptr %t8826, align 1
  %v182855 = zext i32 %t8827 to i64
  br label %edge14599

edge14599:                                        ; preds = %edge14590, %edge14582
  %v182876 = phi i64 [ %v182855, %edge14590 ], [ 0, %edge14582 ]
  %v183044 = and i64 %v182876, 31
  %t8828 = icmp ult i64 %v183044, %v132186
  br i1 %t8828, label %edge14616, label %edge14625

edge14616:                                        ; preds = %edge14599
  %v183266 = shl nuw nsw i64 %v183044, 2
  %v183267 = add i64 %v183266, %v132185
  %t8831 = inttoptr i64 %v183267 to ptr
  %t8832 = load i32, ptr %t8831, align 1
  %v183268 = zext i32 %t8832 to i64
  br label %edge14625

edge14625:                                        ; preds = %edge14616, %edge14599
  %v183288 = phi i64 [ %v183268, %edge14616 ], [ 0, %edge14599 ]
  %v183323 = lshr i64 %v183288, 16
  %v183328 = add nuw nsw i64 %v183323, %v182400
  %v183299 = and i64 %v183288, 65535
  %v183311 = shl nuw nsw i64 %v183299, %v182400
  %v183338 = lshr i64 %v182876, 5
  %v183348 = and i64 %v183328, 63
  %v183349 = shl i64 %v183338, %v183348
  %125 = or i64 %v183311, %v183349
  %v183354 = or i64 %125, %v182397
  %t8841 = icmp samesign ult i64 %v183044, 19
  br i1 %t8841, label %edge14653, label %edge14662

edge14653:                                        ; preds = %edge14625
  %v183577 = add i64 %v183044, ptrtoint (ptr @mech_compact_table_14 to i64)
  %t8844 = inttoptr i64 %v183577 to ptr
  %t8845 = load i8, ptr %t8844, align 1
  %v183578 = zext i8 %t8845 to i64
  br label %edge14662

edge14662:                                        ; preds = %edge14653, %edge14625
  %v183599 = phi i64 [ %v183578, %edge14653 ], [ 0, %edge14625 ]
  %v183608 = add nuw nsw i64 %v183599, %v183328
  %v183617 = lshr i64 %v183608, 3
  %v183622 = and i64 %v183617, 255
  %t8851 = icmp samesign ult i64 %v183622, 9
  br i1 %t8851, label %edge14671, label %block94290

edge14671:                                        ; preds = %edge14662
  %t8853 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v182386, i64 %v183622)
  %v183637 = extractvalue { i64, i1 } %t8853, 0
  %t8854 = extractvalue { i64, i1 } %t8853, 1
  br i1 %t8854, label %block94290, label %edge14673

edge14673:                                        ; preds = %edge14671
  %t8857.not = icmp ugt i64 %v183637, %v182387
  br i1 %t8857.not, label %edge14676, label %block94295

edge14676:                                        ; preds = %edge14673
  %t8859.not = icmp eq i64 %v182387, 0
  br i1 %t8859.not, label %block94298, label %edge14677

edge14677:                                        ; preds = %edge14676
  %126 = add i64 %v182387, -131073
  %v183647231 = icmp ult i64 %126, -126977
  br i1 %v183647231, label %edge14680, label %edge14689

edge14680:                                        ; preds = %edge14677
  %t8863 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v182387, i64 %v182387)
  %v183648 = extractvalue { i64, i1 } %t8863, 0
  %t8864 = extractvalue { i64, i1 } %t8863, 1
  br i1 %t8864, label %block94290, label %block94298

edge14689:                                        ; preds = %edge14677
  %t8871 = mul nuw nsw i64 %v182387, 3
  %t8875 = add nuw nsw i64 %t8871, 1
  %v183665 = lshr i64 %t8875, 1
  br label %block94298

edge14693:                                        ; preds = %block94295
  %t8881 = inttoptr i64 %v182390 to ptr
  %v183673 = load i64, ptr %t8881, align 1
  %t8882 = icmp eq i64 %v183673, 1
  %t8883 = icmp ule i64 %v183642, %v182387
  %v183676232 = and i1 %t8883, %t8882
  br i1 %v183676232, label %block94305, label %block94306

edge14698:                                        ; preds = %block94306
  %v183683 = load i64, ptr %ss1, align 8
  %v183684 = load i64, ptr %t606, align 8
  %v183685 = load i64, ptr %t607, align 8
  br label %block94305

edge14700:                                        ; preds = %block94305
  %v183688 = trunc i64 %v183354 to i8
  %v183690 = add i64 %v183667, %v182386
  %t8904 = inttoptr i64 %v183690 to ptr
  store i8 %v183688, ptr %t8904, align 1
  %t8905.not = icmp eq i64 %v183622, 1
  br i1 %t8905.not, label %block94324, label %edge14703

edge14703:                                        ; preds = %edge14700
  %v183692 = lshr i64 %v183354, 8
  %v183693 = trunc i64 %v183692 to i8
  %v183694 = add i64 %v182386, 1
  %v183695 = add i64 %v183694, %v183667
  %t8907 = inttoptr i64 %v183695 to ptr
  store i8 %v183693, ptr %t8907, align 1
  %t8908 = icmp samesign ugt i64 %v183622, 2
  br i1 %t8908, label %edge14706, label %block94324

edge14706:                                        ; preds = %edge14703
  %v183697 = lshr i64 %v183354, 16
  %v183698 = trunc i64 %v183697 to i8
  %v183699 = add i64 %v182386, 2
  %v183700 = add i64 %v183699, %v183667
  %t8910 = inttoptr i64 %v183700 to ptr
  store i8 %v183698, ptr %t8910, align 1
  %t8911.not = icmp eq i64 %v183622, 3
  br i1 %t8911.not, label %block94324, label %edge14709

edge14709:                                        ; preds = %edge14706
  %v183702 = lshr i64 %v183354, 24
  %v183703 = trunc i64 %v183702 to i8
  %v183704 = add i64 %v182386, 3
  %v183705 = add i64 %v183704, %v183667
  %t8913 = inttoptr i64 %v183705 to ptr
  store i8 %v183703, ptr %t8913, align 1
  %t8914 = icmp samesign ugt i64 %v183622, 4
  br i1 %t8914, label %edge14712, label %block94324

edge14712:                                        ; preds = %edge14709
  %v183707 = lshr i64 %v183354, 32
  %v183708 = trunc i64 %v183707 to i8
  %v183709 = add i64 %v182386, 4
  %v183710 = add i64 %v183709, %v183667
  %t8916 = inttoptr i64 %v183710 to ptr
  store i8 %v183708, ptr %t8916, align 1
  %t8917.not = icmp eq i64 %v183622, 5
  br i1 %t8917.not, label %block94324, label %edge14715

edge14715:                                        ; preds = %edge14712
  %v183712 = lshr i64 %v183354, 40
  %v183713 = trunc i64 %v183712 to i8
  %v183714 = add i64 %v182386, 5
  %v183715 = add i64 %v183714, %v183667
  %t8919 = inttoptr i64 %v183715 to ptr
  store i8 %v183713, ptr %t8919, align 1
  %t8920 = icmp samesign ugt i64 %v183622, 6
  br i1 %t8920, label %edge14718, label %block94324

edge14718:                                        ; preds = %edge14715
  %v183717 = lshr i64 %v183354, 48
  %v183718 = trunc i64 %v183717 to i8
  %v183719 = add i64 %v182386, 6
  %v183720 = add i64 %v183719, %v183667
  %t8922 = inttoptr i64 %v183720 to ptr
  store i8 %v183718, ptr %t8922, align 1
  %t8923 = icmp eq i64 %v183622, 8
  br i1 %t8923, label %edge14721, label %block94324

edge14721:                                        ; preds = %edge14718
  %v183722 = lshr i64 %v183354, 56
  %v183723 = trunc nuw i64 %v183722 to i8
  %v183724 = add i64 %v182386, 7
  %v183725 = add i64 %v183724, %v183667
  %t8925 = inttoptr i64 %v183725 to ptr
  store i8 %v183723, ptr %t8925, align 1
  br label %block94324

edge14724:                                        ; preds = %block94324
  %t8929 = inttoptr i64 %v183672 to ptr
  %v183728 = load i64, ptr %t8929, align 1
  %v183729 = add i64 %v183728, 1
  store i64 %v183729, ptr %t8929, align 1
  br label %block94326

edge14729:                                        ; preds = %block94326
  %v183734 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v182390)
  %t8933.not = icmp eq i64 %v183734, 0
  br i1 %t8933.not, label %edge14733, label %block94290

edge14733:                                        ; preds = %edge14729, %block94326
  %v183750 = and i64 %v183608, 56
  %v183751 = lshr i64 %v183354, %v183750
  %v183987 = add i64 %v182442, 1
  br label %block7305

edge14745:                                        ; preds = %edge14572
  %v183799 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v132196)
  %t8939.not = icmp eq i64 %v183799, 0
  br i1 %t8939.not, label %block94361, label %block94357

edge14749:                                        ; preds = %block94361
  %t8941 = inttoptr i64 %v132190 to ptr
  %v183801 = load i64, ptr %t8941, align 1
  %v183802 = add i64 %v183801, -1
  store i64 %v183802, ptr %t8941, align 1
  %t8943.not = icmp eq i64 %v183802, 0
  br i1 %t8943.not, label %edge14752, label %edge14805

edge14752:                                        ; preds = %edge14749
  %v183803 = call i64 @mech_compact_free(i64 %v0, i64 %v132190)
  %t8944.not = icmp eq i64 %v183803, 0
  br i1 %t8944.not, label %edge14805, label %block94357

edge14790:                                        ; preds = %edge14064.thread
  %v184071 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v132168)
  %t8951.not = icmp eq i64 %v184071, 0
  br i1 %t8951.not, label %edge14805, label %edge14791

edge14791:                                        ; preds = %edge14790
  %t8952 = insertvalue { i64, i64 } poison, i64 %v184071, 0
  %t8953 = insertvalue { i64, i64 } %t8952, i64 7552, 1
  br label %common.ret

edge14805:                                        ; preds = %block94361, %edge14749, %edge14752, %edge14790, %edge14064.thread
  %v1850021148 = phi i64 [ %v179399, %edge14790 ], [ %v179399, %edge14064.thread ], [ %v182400, %edge14752 ], [ %v182400, %edge14749 ], [ %v182400, %block94361 ]
  %v1849991146 = phi i64 [ %v179394, %edge14790 ], [ %v179394, %edge14064.thread ], [ %v182397, %edge14752 ], [ %v182397, %edge14749 ], [ %v182397, %block94361 ]
  %v1849921144 = phi i64 [ %v179315, %edge14790 ], [ %v179315, %edge14064.thread ], [ %v182390, %edge14752 ], [ %v182390, %edge14749 ], [ %v182390, %block94361 ]
  %v1849891142 = phi i64 [ %v179312, %edge14790 ], [ %v179312, %edge14064.thread ], [ %v182387, %edge14752 ], [ %v182387, %edge14749 ], [ %v182387, %block94361 ]
  %v1849881140 = phi i64 [ %v179280, %edge14790 ], [ %v179280, %edge14064.thread ], [ %v182386, %edge14752 ], [ %v182386, %edge14749 ], [ %v182386, %block94361 ]
  %v1849871138 = phi i64 [ %v179310, %edge14790 ], [ %v179310, %edge14064.thread ], [ %v182385, %edge14752 ], [ %v182385, %edge14749 ], [ %v182385, %block94361 ]
  %v185055 = phi i64 [ ptrtoint (ptr @mech_compact_table_9 to i64), %edge14790 ], [ ptrtoint (ptr @mech_compact_table_9 to i64), %edge14064.thread ], [ %v132163, %edge14752 ], [ %v132163, %edge14749 ], [ %v132163, %block94361 ]
  %v185056 = phi i64 [ 320, %edge14790 ], [ 320, %edge14064.thread ], [ %v132164, %edge14752 ], [ %v132164, %edge14749 ], [ %v132164, %block94361 ]
  %v185060 = phi i64 [ 0, %edge14790 ], [ 0, %edge14064.thread ], [ %v132168, %edge14752 ], [ %v132168, %edge14749 ], [ %v132168, %block94361 ]
  br i1 %t23.not, label %block94704, label %edge14812

edge14812:                                        ; preds = %edge14805
  %v186187 = load i64, ptr %t910, align 1
  %v186188 = add i64 %v186187, 1
  store i64 %v186188, ptr %t910, align 1
  br label %block94704

edge14815:                                        ; preds = %block94704
  %t8962 = inttoptr i64 %v185060 to ptr
  %v186189 = load i64, ptr %t8962, align 1
  %v186190 = add i64 %v186189, 1
  store i64 %v186190, ptr %t8962, align 1
  br label %edge14829

edge14829:                                        ; preds = %block94704, %edge14815
  %t9018.pre-phi = phi ptr [ %t8962, %edge14815 ], [ null, %block94704 ]
  br label %block7441

edge14865:                                        ; preds = %block7441
  br i1 %t3962, label %edge14869, label %edge14867

edge14867:                                        ; preds = %edge14865
  %127 = xor i64 %v186324, -1
  %v190374 = add i64 %v7458022184, %127
  %t9053 = icmp ult i64 %v190374, %v7458022184
  br i1 %t9053, label %edge15199, label %edge15255

edge14869:                                        ; preds = %edge14865
  %v186807 = add i64 %v186324, %v54959
  %v186812 = sub i64 %v55001, %v186324
  %t8969 = icmp ugt i64 %v186812, 3
  br i1 %t8969, label %edge14880, label %edge15034

edge14880:                                        ; preds = %edge14869
  %t8972.not = icmp ugt i64 %v186807, %v14025
  br i1 %t8972.not, label %edge14901, label %edge14888

edge14888:                                        ; preds = %edge14880
  br i1 %v1062051.not1294, label %edge14893, label %edge14893.thread

edge14893.thread:                                 ; preds = %edge14888
  %v187397 = add i64 %v186807, %v1396
  %t8978 = inttoptr i64 %v187397 to ptr
  %t8979 = load i32, ptr %t8978, align 1
  %v187398 = zext i32 %t8979 to i64
  br label %edge14901

edge14893:                                        ; preds = %edge14888
  %v187400 = call i64 @mech_compact_matrix_word(i64 %v1396, i64 %v186807, i64 %v1399, i64 %v1400, i64 4)
  %t8980 = icmp ult i64 %v187400, 4294967296
  br i1 %t8980, label %edge14901, label %common.ret

edge14901:                                        ; preds = %edge14893.thread, %edge14893, %edge14880
  %v187599 = phi i64 [ %v187400, %edge14893 ], [ 0, %edge14880 ], [ %v187398, %edge14893.thread ]
  %v187626 = and i64 %v187599, 255
  %t8984 = icmp ult i64 %v187626, %v185056
  br i1 %t8984, label %edge14918, label %edge14927

edge14918:                                        ; preds = %edge14901
  %v187885 = shl nuw nsw i64 %v187626, 2
  %v187886 = add i64 %v187885, %v185055
  %t8987 = inttoptr i64 %v187886 to ptr
  %t8988 = load i32, ptr %t8987, align 1
  %v187887 = zext i32 %t8988 to i64
  br label %edge14927

edge14927:                                        ; preds = %edge14918, %edge14901
  %v187911 = phi i64 [ %v187887, %edge14918 ], [ 0, %edge14901 ]
  %v187926 = and i64 %v187911, 65535
  %v187937 = lshr i64 %v187911, 16
  %v187950 = lshr i64 %v187599, 8
  %v187955 = and i64 %v187950, 255
  %t8991 = icmp ult i64 %v187955, %v185056
  br i1 %t8991, label %edge14947, label %edge14956

edge14947:                                        ; preds = %edge14927
  %v188182 = shl nuw nsw i64 %v187955, 2
  %v188183 = add i64 %v188182, %v185055
  %t8994 = inttoptr i64 %v188183 to ptr
  %t8995 = load i32, ptr %t8994, align 1
  %v188184 = zext i32 %t8995 to i64
  br label %edge14956

edge14956:                                        ; preds = %edge14947, %edge14927
  %v188204 = phi i64 [ %v188184, %edge14947 ], [ 0, %edge14927 ]
  %v188213 = and i64 %v188204, 65535
  %v188223 = and i64 %v187937, 63
  %v188224 = shl i64 %v188213, %v188223
  %v188229 = or i64 %v188224, %v187926
  %v188235 = lshr i64 %v188204, 16
  %v188240 = add nuw nsw i64 %v188235, %v187937
  %v188246 = lshr i64 %v187599, 16
  %v188251 = and i64 %v188246, 255
  %t8999 = icmp ult i64 %v188251, %v185056
  br i1 %t8999, label %edge14980, label %edge14989

edge14980:                                        ; preds = %edge14956
  %v188478 = shl nuw nsw i64 %v188251, 2
  %v188479 = add i64 %v188478, %v185055
  %t9002 = inttoptr i64 %v188479 to ptr
  %t9003 = load i32, ptr %t9002, align 1
  %v188480 = zext i32 %t9003 to i64
  br label %edge14989

edge14989:                                        ; preds = %edge14980, %edge14956
  %v188500 = phi i64 [ %v188480, %edge14980 ], [ 0, %edge14956 ]
  %v188509 = and i64 %v188500, 65535
  %v188519 = and i64 %v188240, 63
  %v188520 = shl i64 %v188509, %v188519
  %v188525 = or i64 %v188229, %v188520
  %v188531 = lshr i64 %v188500, 16
  %v188536 = add nuw nsw i64 %v188531, %v188240
  %v188707 = lshr i64 %v187599, 24
  %t9007 = icmp ult i64 %v188707, %v185056
  br i1 %t9007, label %edge15012, label %edge15406.thread1181

edge15012:                                        ; preds = %edge14989
  %v188934 = shl nuw nsw i64 %v188707, 2
  %v188935 = add i64 %v188934, %v185055
  %t9010 = inttoptr i64 %v188935 to ptr
  %t9011 = load i32, ptr %t9010, align 1
  %v188936 = zext i32 %t9011 to i64
  br label %edge15406.thread1181

edge15406.thread1181:                             ; preds = %edge14989, %edge15012
  %v188956 = phi i64 [ %v188936, %edge15012 ], [ 0, %edge14989 ]
  %v188965 = and i64 %v188956, 65535
  %v188975 = and i64 %v188536, 63
  %v188976 = shl i64 %v188965, %v188975
  %v188981 = or i64 %v188525, %v188976
  %v188987 = lshr i64 %v188956, 16
  %v188992 = add nuw nsw i64 %v188987, %v188536
  br label %edge15413

edge15034:                                        ; preds = %edge14869
  br i1 %t23.not, label %block96263, label %edge15035

edge15035:                                        ; preds = %edge15034
  %v189027 = load i64, ptr %t910, align 1
  %v189028 = add i64 %v189027, 1
  store i64 %v189028, ptr %t910, align 1
  br label %block96263

edge15038:                                        ; preds = %block96263
  %v189029 = load i64, ptr %t9018.pre-phi, align 1
  %v189030 = add i64 %v189029, 1
  store i64 %v189030, ptr %t9018.pre-phi, align 1
  br label %block7634.preheader

edge15081:                                        ; preds = %block7634
  br i1 %v1062051.not1294, label %edge15085, label %block96614

edge15085:                                        ; preds = %edge15081
  br i1 %t975, label %trap9030, label %ok9030

edge15096:                                        ; preds = %block7634, %block96614
  %v189596 = phi i64 [ 0, %block7634 ], [ %v189555, %block96614 ]
  %t9035 = icmp ult i64 %v189596, %v185056
  %or.cond327 = select i1 %t9020.not, i1 %t9035, i1 false
  br i1 %or.cond327, label %edge15129.thread, label %edge15129

edge15129.thread:                                 ; preds = %edge15096
  %v190025 = shl nuw nsw i64 %v189596, 2
  %v190026 = add i64 %v190025, %v185055
  %t9038 = inttoptr i64 %v190026 to ptr
  %t9039 = load i32, ptr %t9038, align 1
  %v190027 = zext i32 %t9039 to i64
  br label %edge15155

edge15129:                                        ; preds = %edge15096
  br i1 %t9020.not, label %edge15155, label %edge15138

edge15138:                                        ; preds = %edge15129
  br i1 %t23.not, label %block96964, label %edge15141

edge15141:                                        ; preds = %edge15138
  %v190115 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t9045.not = icmp eq i64 %v190115, 0
  br i1 %t9045.not, label %block96964, label %block96960

edge15145:                                        ; preds = %block96964
  %v190117 = load i64, ptr %t9018.pre-phi, align 1
  %v190118 = add i64 %v190117, -1
  store i64 %v190118, ptr %t9018.pre-phi, align 1
  %t9049.not = icmp eq i64 %v190118, 0
  br i1 %t9049.not, label %edge15148, label %edge15406

edge15148:                                        ; preds = %edge15145
  %v190119 = call i64 @mech_compact_free(i64 %v0, i64 %v185060)
  %t9050.not = icmp eq i64 %v190119, 0
  br i1 %t9050.not, label %edge15406, label %block96960

edge15155:                                        ; preds = %edge15129.thread, %edge15129
  %v1900661153 = phi i64 [ %v190027, %edge15129.thread ], [ 0, %edge15129 ]
  %v190106 = add i64 %v189087, 1
  %v190096 = lshr i64 %v1900661153, 16
  %v190101 = add i64 %v190096, %v189072
  %v190072 = and i64 %v1900661153, 65535
  %v190083 = and i64 %v189072, 63
  %v190084 = shl i64 %v190072, %v190083
  %v190089 = or i64 %v190084, %v189069
  %v190301 = add nsw i64 %v189102, -1
  br label %block7634

edge15199:                                        ; preds = %edge14867
  %v190601 = shl i64 %v190374, 3
  %v190602 = add i64 %v190601, %v7457922180
  %t9056 = inttoptr i64 %v190602 to ptr
  %v190603 = load i64, ptr %t9056, align 1
  %128 = lshr i64 %v190603, 32
  %v190651 = and i64 %128, 511
  %v190661 = lshr i64 %v190603, 41
  %t9059 = icmp eq i64 %v190651, 1
  br i1 %t9059, label %edge15217, label %edge15247

edge15217:                                        ; preds = %edge15199
  %t9062 = icmp ult i64 %v190661, %v185056
  br i1 %t9062, label %edge15225, label %edge15234

edge15225:                                        ; preds = %edge15217
  %v191076 = shl nuw nsw i64 %v190661, 2
  %v191077 = add i64 %v191076, %v185055
  %t9065 = inttoptr i64 %v191077 to ptr
  %t9066 = load i32, ptr %t9065, align 1
  %v191078 = zext i32 %t9066 to i64
  br label %edge15234

edge15234:                                        ; preds = %edge15225, %edge15217
  %v191098 = phi i64 [ %v191078, %edge15225 ], [ 0, %edge15217 ]
  %v191107 = and i64 %v191098, 65535
  %v191113 = lshr i64 %v191098, 16
  br label %edge15413

edge15247:                                        ; preds = %edge15199
  %t9068 = icmp samesign ult i64 %v190651, 291
  br i1 %t9068, label %edge15255, label %edge15264

edge15255:                                        ; preds = %edge14867, %edge15247
  %v19065111581166 = phi i64 [ %v190651, %edge15247 ], [ 0, %edge14867 ]
  %v19066111591164 = phi i64 [ %v190661, %edge15247 ], [ 0, %edge14867 ]
  %v191353 = shl nuw nsw i64 %v19065111581166, 2
  %v191354 = add i64 %v191353, ptrtoint (ptr @mech_compact_table_20 to i64)
  %t9071 = inttoptr i64 %v191354 to ptr
  %t9072 = load i32, ptr %t9071, align 1
  %v191355 = zext i32 %t9072 to i64
  br label %edge15264

edge15264:                                        ; preds = %edge15255, %edge15247
  %v19065111581165 = phi i64 [ %v19065111581166, %edge15255 ], [ %v190651, %edge15247 ]
  %v19066111591163 = phi i64 [ %v19066111591164, %edge15255 ], [ %v190661, %edge15247 ]
  %v191375 = phi i64 [ %v191355, %edge15255 ], [ 0, %edge15247 ]
  %v191384 = and i64 %v191375, 511
  %v191551 = lshr i64 %v191375, 12
  %t9074 = icmp ult i64 %v191384, %v185056
  br i1 %t9074, label %edge15282, label %edge15291

edge15282:                                        ; preds = %edge15264
  %v191778 = shl nuw nsw i64 %v191384, 2
  %v191779 = add i64 %v191778, %v185055
  %t9077 = inttoptr i64 %v191779 to ptr
  %t9078 = load i32, ptr %t9077, align 1
  %v191780 = zext i32 %t9078 to i64
  br label %edge15291

edge15291:                                        ; preds = %edge15282, %edge15264
  %v191800 = phi i64 [ %v191780, %edge15282 ], [ 0, %edge15264 ]
  %v191810 = lshr i64 %v191800, 16
  %v191815 = and i64 %v191800, 65535
  %v191822 = and i64 %v191810, 31
  %v191828 = shl nuw nsw i64 %v191551, %v191822
  %v191828.masked = and i64 %v191828, 4294967295
  %v191838 = or i64 %v191828.masked, %v191815
  %v191850 = lshr i64 %v191375, 9
  %v191857 = and i64 %v191850, 7
  %v191862 = add nuw nsw i64 %v191810, %v191857
  %v191886 = sub nsw i64 256, %v19066111591163
  %v191891 = lshr i64 %v191886, 29
  %v191901 = and i64 %v191891, 7
  %v191906 = add nsw i64 %v19066111591163, -1
  %v191912 = lshr i64 %v191906, %v191901
  %t9084 = icmp ult i64 %v191912, 256
  br i1 %t9084, label %edge15318, label %edge15325

edge15318:                                        ; preds = %edge15291
  %v192114 = add i64 %v191912, ptrtoint (ptr @mech_compact_table_18 to i64)
  %t9087 = inttoptr i64 %v192114 to ptr
  %t9088 = load i8, ptr %t9087, align 1
  %v192115 = zext i8 %t9088 to i64
  br label %edge15325

edge15325:                                        ; preds = %edge15291, %edge15318
  %v192133 = phi i64 [ %v192115, %edge15318 ], [ 0, %edge15291 ]
  %v192139 = shl nuw nsw i64 %v191901, 1
  %v192144 = add nuw nsw i64 %v192133, %v192139
  %v192153 = add nuw nsw i64 %v192144, 288
  %t9090 = icmp ult i64 %v192153, %v185056
  br i1 %t9090, label %edge15344, label %edge15353

edge15344:                                        ; preds = %edge15325
  %v192380 = shl nuw nsw i64 %v192153, 2
  %v192381 = add i64 %v192380, %v185055
  %t9093 = inttoptr i64 %v192381 to ptr
  %t9094 = load i32, ptr %t9093, align 1
  %v192382 = zext i32 %t9094 to i64
  br label %edge15353

edge15353:                                        ; preds = %edge15344, %edge15325
  %v192402 = phi i64 [ %v192382, %edge15344 ], [ 0, %edge15325 ]
  %v192411 = and i64 %v192402, 65535
  %v192417 = lshr i64 %v192402, 16
  %v192431 = and i64 %v191862, 63
  %v192432 = shl i64 %v192411, %v192431
  %v192437 = or i64 %v191838, %v192432
  %v192442 = add nuw nsw i64 %v192417, %v191862
  %t9097 = icmp samesign ult i64 %v192144, 32
  br i1 %t9097, label %edge15377, label %edge15386

edge15377:                                        ; preds = %edge15353
  %v192451 = shl nuw nsw i64 %v192144, 2
  %v192679 = add i64 %v192451, add (i64 ptrtoint (ptr @mech_compact_table_20 to i64), i64 1036)
  %t9100 = inttoptr i64 %v192679 to ptr
  %t9101 = load i32, ptr %t9100, align 1
  %v192680 = zext i32 %t9101 to i64
  br label %edge15386

edge15386:                                        ; preds = %edge15377, %edge15353
  %v192700 = phi i64 [ %v192680, %edge15377 ], [ 0, %edge15353 ]
  %v192709 = and i64 %v192700, 65535
  %v192714 = sub nsw i64 %v19066111591163, %v192709
  %v192724 = and i64 %v192442, 63
  %v192725 = shl i64 %v192714, %v192724
  %v192730 = or i64 %v192437, %v192725
  %v192736 = lshr i64 %v192700, 16
  %v192741 = add nuw nsw i64 %v192736, %v192442
  br label %edge15413

edge15406:                                        ; preds = %block96964, %edge15145, %edge15148
  %v190356 = sub i64 %v189087, %v186807
  br label %edge15413

edge15411:                                        ; preds = %block7441
  br i1 %t23.not, label %block98707, label %edge15494

edge15413:                                        ; preds = %edge15386, %edge15234, %edge15406, %edge15406.thread1181
  %v192776.ph.pn = phi i64 [ %v188981, %edge15406.thread1181 ], [ %v189069, %edge15406 ], [ %v191107, %edge15234 ], [ %v192730, %edge15386 ]
  %v1927821173 = phi i64 [ 4, %edge15406.thread1181 ], [ %v190356, %edge15406 ], [ 1, %edge15234 ], [ %v19065111581165, %edge15386 ]
  %v1927791172 = phi i64 [ %v188992, %edge15406.thread1181 ], [ %v189072, %edge15406 ], [ %v191113, %edge15234 ], [ %v192741, %edge15386 ]
  %v1928041170.pn = shl i64 %v192776.ph.pn, %v186288
  %v1928171174 = or i64 %v1928041170.pn, %v186285
  %v193015 = add i64 %v1927791172, %v186288
  %v193030 = lshr i64 %v193015, 3
  %v193035 = and i64 %v193030, 255
  %t9109 = icmp samesign ult i64 %v193035, 9
  br i1 %t9109, label %edge15420, label %block98636

edge15420:                                        ; preds = %edge15413
  %t9111 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v186274, i64 %v193035)
  %v193098 = extractvalue { i64, i1 } %t9111, 0
  %t9112 = extractvalue { i64, i1 } %t9111, 1
  br i1 %t9112, label %block98636, label %edge15422

edge15422:                                        ; preds = %edge15420
  %t9115.not = icmp ugt i64 %v193098, %v186275
  br i1 %t9115.not, label %edge15425, label %block98641

edge15425:                                        ; preds = %edge15422
  %t9117.not = icmp eq i64 %v186275, 0
  br i1 %t9117.not, label %block98644, label %edge15426

edge15426:                                        ; preds = %edge15425
  %129 = add i64 %v186275, -131073
  %v193108243 = icmp ult i64 %129, -126977
  br i1 %v193108243, label %edge15429, label %edge15438

edge15429:                                        ; preds = %edge15426
  %t9121 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v186275, i64 %v186275)
  %v193109 = extractvalue { i64, i1 } %t9121, 0
  %t9122 = extractvalue { i64, i1 } %t9121, 1
  br i1 %t9122, label %block98636, label %block98644

edge15438:                                        ; preds = %edge15426
  %t9129 = mul nuw nsw i64 %v186275, 3
  %t9133 = add nuw nsw i64 %t9129, 1
  %v193126 = lshr i64 %t9133, 1
  br label %block98644

edge15442:                                        ; preds = %block98641
  %t9139 = inttoptr i64 %v186278 to ptr
  %v193134 = load i64, ptr %t9139, align 1
  %t9140 = icmp eq i64 %v193134, 1
  %t9141 = icmp ule i64 %v193103, %v186275
  %v193137244 = and i1 %t9141, %t9140
  br i1 %v193137244, label %block98651, label %block98652

edge15447:                                        ; preds = %block98652
  %v193144 = load i64, ptr %ss1, align 8
  %v193145 = load i64, ptr %t606, align 8
  %v193146 = load i64, ptr %t607, align 8
  br label %block98651

edge15449:                                        ; preds = %block98651
  %v193149 = trunc i64 %v1928171174 to i8
  %v193151 = add i64 %v193128, %v186274
  %t9162 = inttoptr i64 %v193151 to ptr
  store i8 %v193149, ptr %t9162, align 1
  %t9163.not = icmp eq i64 %v193035, 1
  br i1 %t9163.not, label %block98670, label %edge15452

edge15452:                                        ; preds = %edge15449
  %v193153 = lshr i64 %v1928171174, 8
  %v193154 = trunc i64 %v193153 to i8
  %v193155 = add i64 %v186274, 1
  %v193156 = add i64 %v193155, %v193128
  %t9165 = inttoptr i64 %v193156 to ptr
  store i8 %v193154, ptr %t9165, align 1
  %t9166 = icmp samesign ugt i64 %v193035, 2
  br i1 %t9166, label %edge15455, label %block98670

edge15455:                                        ; preds = %edge15452
  %v193158 = lshr i64 %v1928171174, 16
  %v193159 = trunc i64 %v193158 to i8
  %v193160 = add i64 %v186274, 2
  %v193161 = add i64 %v193160, %v193128
  %t9168 = inttoptr i64 %v193161 to ptr
  store i8 %v193159, ptr %t9168, align 1
  %t9169.not = icmp eq i64 %v193035, 3
  br i1 %t9169.not, label %block98670, label %edge15458

edge15458:                                        ; preds = %edge15455
  %v193163 = lshr i64 %v1928171174, 24
  %v193164 = trunc i64 %v193163 to i8
  %v193165 = add i64 %v186274, 3
  %v193166 = add i64 %v193165, %v193128
  %t9171 = inttoptr i64 %v193166 to ptr
  store i8 %v193164, ptr %t9171, align 1
  %t9172 = icmp samesign ugt i64 %v193035, 4
  br i1 %t9172, label %edge15461, label %block98670

edge15461:                                        ; preds = %edge15458
  %v193168 = lshr i64 %v1928171174, 32
  %v193169 = trunc i64 %v193168 to i8
  %v193170 = add i64 %v186274, 4
  %v193171 = add i64 %v193170, %v193128
  %t9174 = inttoptr i64 %v193171 to ptr
  store i8 %v193169, ptr %t9174, align 1
  %t9175.not = icmp eq i64 %v193035, 5
  br i1 %t9175.not, label %block98670, label %edge15464

edge15464:                                        ; preds = %edge15461
  %v193173 = lshr i64 %v1928171174, 40
  %v193174 = trunc i64 %v193173 to i8
  %v193175 = add i64 %v186274, 5
  %v193176 = add i64 %v193175, %v193128
  %t9177 = inttoptr i64 %v193176 to ptr
  store i8 %v193174, ptr %t9177, align 1
  %t9178 = icmp samesign ugt i64 %v193035, 6
  br i1 %t9178, label %edge15467, label %block98670

edge15467:                                        ; preds = %edge15464
  %v193178 = lshr i64 %v1928171174, 48
  %v193179 = trunc i64 %v193178 to i8
  %v193180 = add i64 %v186274, 6
  %v193181 = add i64 %v193180, %v193128
  %t9180 = inttoptr i64 %v193181 to ptr
  store i8 %v193179, ptr %t9180, align 1
  %t9181 = icmp eq i64 %v193035, 8
  br i1 %t9181, label %edge15470, label %block98670

edge15470:                                        ; preds = %edge15467
  %v193183 = lshr i64 %v1928171174, 56
  %v193184 = trunc nuw i64 %v193183 to i8
  %v193185 = add i64 %v186274, 7
  %v193186 = add i64 %v193185, %v193128
  %t9183 = inttoptr i64 %v193186 to ptr
  store i8 %v193184, ptr %t9183, align 1
  br label %block98670

edge15473:                                        ; preds = %block98670
  %t9187 = inttoptr i64 %v193133 to ptr
  %v193189 = load i64, ptr %t9187, align 1
  %v193190 = add i64 %v193189, 1
  store i64 %v193190, ptr %t9187, align 1
  br label %block98672

edge15478:                                        ; preds = %block98672
  %v193195 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v186278)
  %t9191.not = icmp eq i64 %v193195, 0
  br i1 %t9191.not, label %edge15482, label %block98636

edge15482:                                        ; preds = %edge15478, %block98672
  %v193211 = and i64 %v193015, 56
  %v193212 = lshr i64 %v1928171174, %v193211
  %v193225 = and i64 %v193015, 7
  %v193502 = add i64 %v1927821173, %v186324
  br label %block7441

edge15494:                                        ; preds = %edge15411
  %v193267 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t9197.not = icmp eq i64 %v193267, 0
  br i1 %t9197.not, label %block98707, label %block98703

edge15500:                                        ; preds = %block98707
  %v193271 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v7458422199)
  %t9199.not = icmp eq i64 %v193271, 0
  br i1 %t9199.not, label %block98711, label %block98703

edge15504:                                        ; preds = %block98711
  %v193273 = load i64, ptr %t9018.pre-phi, align 1
  %v193274 = add i64 %v193273, -1
  store i64 %v193274, ptr %t9018.pre-phi, align 1
  %t9203.not = icmp eq i64 %v193274, 0
  br i1 %t9203.not, label %edge15507, label %edge15551

edge15507:                                        ; preds = %edge15504
  %v193275 = call i64 @mech_compact_free(i64 %v0, i64 %v185060)
  %t9204.not = icmp eq i64 %v193275, 0
  br i1 %t9204.not, label %edge15551, label %block98703

edge15551:                                        ; preds = %edge15507, %edge15504
  %t9207 = icmp ugt i64 %v185056, 256
  br i1 %t9207, label %edge15570, label %edge15555

edge15551.thread:                                 ; preds = %block98711
  %t92071188 = icmp ugt i64 %v185056, 256
  br i1 %t92071188, label %edge15565.thread, label %edge15579

edge15565.thread:                                 ; preds = %edge15551.thread
  %v1943781189 = add i64 %v185055, 1024
  %t92171190 = inttoptr i64 %v1943781189 to ptr
  %t92181191 = load i32, ptr %t92171190, align 1
  br label %edge15579

edge15555:                                        ; preds = %edge15551
  %v194217 = load i64, ptr %t9018.pre-phi, align 1
  %v194218 = add i64 %v194217, -1
  store i64 %v194218, ptr %t9018.pre-phi, align 1
  %t9213.not = icmp eq i64 %v194218, 0
  br i1 %t9213.not, label %edge15558, label %edge15579

edge15558:                                        ; preds = %edge15555
  %v194219 = call i64 @mech_compact_free(i64 %v0, i64 %v185060)
  %t9214.not = icmp eq i64 %v194219, 0
  br i1 %t9214.not, label %edge15579, label %edge15559

edge15559:                                        ; preds = %edge15558
  %t9215 = insertvalue { i64, i64 } poison, i64 %v194219, 0
  %t9216 = insertvalue { i64, i64 } %t9215, i64 8118, 1
  br label %common.ret

edge15570:                                        ; preds = %edge15551
  %v194378 = add i64 %v185055, 1024
  %t9217 = inttoptr i64 %v194378 to ptr
  %t9218 = load i32, ptr %t9217, align 1
  %v194383 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v185060)
  %t9220.not = icmp eq i64 %v194383, 0
  br i1 %t9220.not, label %edge15579, label %edge15571

edge15571:                                        ; preds = %edge15570
  %t9221 = insertvalue { i64, i64 } poison, i64 %v194383, 0
  %t9222 = insertvalue { i64, i64 } %t9221, i64 8123, 1
  br label %common.ret

edge15579:                                        ; preds = %edge15551.thread, %edge15565.thread, %edge15570, %edge15555, %edge15558
  %v194399.shrunk = phi i32 [ %t9218, %edge15570 ], [ %t92181191, %edge15565.thread ], [ 0, %edge15551.thread ], [ 0, %edge15555 ], [ 0, %edge15558 ]
  %v194399 = zext i32 %v194399.shrunk to i64
  %v194408 = and i64 %v194399, 65535
  %v194423 = shl nuw nsw i64 %v194408, %v186288
  %v194428 = or i64 %v194423, %v186285
  %v194434 = lshr i64 %v194399, 16
  %v194447 = add nuw nsw i64 %v194434, %v186288
  %v194455 = lshr i64 %v194447, 3
  %v194460 = and i64 %v194455, 255
  %t9231 = icmp samesign ult i64 %v194460, 9
  br i1 %t9231, label %edge15598, label %block99082

edge15598:                                        ; preds = %edge15579
  %t9233 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v186274, i64 %v194460)
  %v194475 = extractvalue { i64, i1 } %t9233, 0
  %t9234 = extractvalue { i64, i1 } %t9233, 1
  br i1 %t9234, label %block99082, label %edge15600

edge15600:                                        ; preds = %edge15598
  %t9237.not = icmp ugt i64 %v194475, %v186275
  br i1 %t9237.not, label %edge15603, label %block99087

edge15603:                                        ; preds = %edge15600
  %t9239.not = icmp eq i64 %v186275, 0
  br i1 %t9239.not, label %block99090, label %edge15604

edge15604:                                        ; preds = %edge15603
  %130 = add i64 %v186275, -131073
  %v194485239 = icmp ult i64 %130, -126977
  br i1 %v194485239, label %edge15607, label %edge15616

edge15607:                                        ; preds = %edge15604
  %t9243 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v186275, i64 %v186275)
  %v194486 = extractvalue { i64, i1 } %t9243, 0
  %t9244 = extractvalue { i64, i1 } %t9243, 1
  br i1 %t9244, label %block99082, label %block99090

edge15616:                                        ; preds = %edge15604
  %t9251 = mul nuw nsw i64 %v186275, 3
  %t9255 = add nuw nsw i64 %t9251, 1
  %v194503 = lshr i64 %t9255, 1
  br label %block99090

edge15620:                                        ; preds = %block99087
  %t9261 = inttoptr i64 %v186278 to ptr
  %v194511 = load i64, ptr %t9261, align 1
  %t9262 = icmp eq i64 %v194511, 1
  %t9263 = icmp ule i64 %v194480, %v186275
  %v194514240 = and i1 %t9263, %t9262
  br i1 %v194514240, label %block99097, label %block99098

edge15625:                                        ; preds = %block99098
  %v194521 = load i64, ptr %ss1, align 8
  %v194522 = load i64, ptr %t606, align 8
  %v194523 = load i64, ptr %t607, align 8
  br label %block99097

edge15627:                                        ; preds = %block99097
  %v194526 = trunc i64 %v194428 to i8
  %v194528 = add i64 %v194505, %v186274
  %t9284 = inttoptr i64 %v194528 to ptr
  store i8 %v194526, ptr %t9284, align 1
  %t9285.not = icmp eq i64 %v194460, 1
  br i1 %t9285.not, label %block99116, label %edge15630

edge15630:                                        ; preds = %edge15627
  %v194530 = lshr i64 %v194428, 8
  %v194531 = trunc i64 %v194530 to i8
  %v194532 = add i64 %v186274, 1
  %v194533 = add i64 %v194532, %v194505
  %t9287 = inttoptr i64 %v194533 to ptr
  store i8 %v194531, ptr %t9287, align 1
  %t9288 = icmp samesign ugt i64 %v194460, 2
  br i1 %t9288, label %edge15633, label %block99116

edge15633:                                        ; preds = %edge15630
  %v194535 = lshr i64 %v194428, 16
  %v194536 = trunc i64 %v194535 to i8
  %v194537 = add i64 %v186274, 2
  %v194538 = add i64 %v194537, %v194505
  %t9290 = inttoptr i64 %v194538 to ptr
  store i8 %v194536, ptr %t9290, align 1
  %t9291.not = icmp eq i64 %v194460, 3
  br i1 %t9291.not, label %block99116, label %edge15636

edge15636:                                        ; preds = %edge15633
  %v194540 = lshr i64 %v194428, 24
  %v194541 = trunc i64 %v194540 to i8
  %v194542 = add i64 %v186274, 3
  %v194543 = add i64 %v194542, %v194505
  %t9293 = inttoptr i64 %v194543 to ptr
  store i8 %v194541, ptr %t9293, align 1
  %t9294 = icmp samesign ugt i64 %v194460, 4
  br i1 %t9294, label %edge15639, label %block99116

edge15639:                                        ; preds = %edge15636
  %v194545 = lshr i64 %v194428, 32
  %v194546 = trunc i64 %v194545 to i8
  %v194547 = add i64 %v186274, 4
  %v194548 = add i64 %v194547, %v194505
  %t9296 = inttoptr i64 %v194548 to ptr
  store i8 %v194546, ptr %t9296, align 1
  %t9297.not = icmp eq i64 %v194460, 5
  br i1 %t9297.not, label %block99116, label %edge15642

edge15642:                                        ; preds = %edge15639
  %v194550 = lshr i64 %v194428, 40
  %v194551 = trunc i64 %v194550 to i8
  %v194552 = add i64 %v186274, 5
  %v194553 = add i64 %v194552, %v194505
  %t9299 = inttoptr i64 %v194553 to ptr
  store i8 %v194551, ptr %t9299, align 1
  %t9300 = icmp samesign ugt i64 %v194460, 6
  br i1 %t9300, label %edge15645, label %block99116

edge15645:                                        ; preds = %edge15642
  %v194555 = lshr i64 %v194428, 48
  %v194556 = trunc i64 %v194555 to i8
  %v194557 = add i64 %v186274, 6
  %v194558 = add i64 %v194557, %v194505
  %t9302 = inttoptr i64 %v194558 to ptr
  store i8 %v194556, ptr %t9302, align 1
  %t9303 = icmp eq i64 %v194460, 8
  br i1 %t9303, label %edge15648, label %block99116

edge15648:                                        ; preds = %edge15645
  %v194560 = lshr i64 %v194428, 56
  %v194561 = trunc nuw i64 %v194560 to i8
  %v194562 = add i64 %v186274, 7
  %v194563 = add i64 %v194562, %v194505
  %t9305 = inttoptr i64 %v194563 to ptr
  store i8 %v194561, ptr %t9305, align 1
  br label %block99116

edge15651:                                        ; preds = %block99116
  %t9309 = inttoptr i64 %v194510 to ptr
  %v194566 = load i64, ptr %t9309, align 1
  %v194567 = add i64 %v194566, 1
  store i64 %v194567, ptr %t9309, align 1
  br label %block99118

edge15656:                                        ; preds = %block99118
  %v194572 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v186278)
  %t9313.not = icmp eq i64 %v194572, 0
  br i1 %t9313.not, label %edge15660, label %block99082

edge15660:                                        ; preds = %edge15656, %block99118
  %v194588 = and i64 %v194447, 56
  %v194589 = lshr i64 %v194428, %v194588
  %v194594 = and i64 %v194447, 7
  br label %edge15681

edge15681:                                        ; preds = %edge14021, %edge14018, %edge14016, %edge15660
  %v206768 = phi i64 [ %v194505, %edge15660 ], [ %v178745, %edge14016 ], [ %v178745, %edge14018 ], [ %v178745, %edge14021 ]
  %v206782 = phi i64 [ %v194475, %edge15660 ], [ %v178715, %edge14016 ], [ %v178715, %edge14018 ], [ %v178715, %edge14021 ]
  %v206796 = phi i64 [ %v194507, %edge15660 ], [ %v178747, %edge14016 ], [ %v178747, %edge14018 ], [ %v178747, %edge14021 ]
  %v206838 = phi i64 [ %v194510, %edge15660 ], [ %v178750, %edge14016 ], [ %v178750, %edge14018 ], [ %v178750, %edge14021 ]
  %v206858 = phi i64 [ %v194589, %edge15660 ], [ 0, %edge14016 ], [ 0, %edge14018 ], [ 0, %edge14021 ]
  %v206874 = phi i64 [ %v194594, %edge15660 ], [ 0, %edge14016 ], [ 0, %edge14018 ], [ 0, %edge14021 ]
  %t9318 = icmp eq i64 %v54964, 1
  %t9319 = icmp eq i64 %v54966, 1
  %v196357267 = and i1 %t9318, %t9319
  br i1 %v196357267, label %block8027, label %edge15686

edge15686:                                        ; preds = %edge15681
  %t9322.not = icmp eq i64 %v54944, 0
  br i1 %t9322.not, label %block99166, label %edge15689

edge15689:                                        ; preds = %edge15686
  %v196367 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54944)
  %t9323.not = icmp eq i64 %v196367, 0
  br i1 %t9323.not, label %block99166, label %block99162

edge15695:                                        ; preds = %block99166
  %v196371 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54956)
  %t9325.not = icmp eq i64 %v196371, 0
  br i1 %t9325.not, label %edge15935, label %block99162

edge15707:                                        ; preds = %block8027
  %t9329 = inttoptr i64 %v196526 to ptr
  %v196527 = load i64, ptr %t9329, align 1
  %t9330 = icmp ne i64 %v196527, 1
  %t9332.not = icmp ugt i64 %v3720, %v196546
  %or.cond328 = select i1 %t9330, i1 true, i1 %t9332.not
  br i1 %or.cond328, label %edge15727, label %edge15711

edge15711:                                        ; preds = %edge15707
  %t9334 = zext i64 %v54961 to i128
  %t9335 = zext i64 %v54962 to i128
  %t9336 = shl nuw i128 %t9335, 64
  %v220323 = or disjoint i128 %t9336, %t9334
  store i128 %v220323, ptr %ss2, align 16
  %t9338 = zext nneg i64 %v196549 to i128
  %t9339 = zext i64 %v196550 to i128
  %t9340 = shl nuw i128 %t9339, 64
  %v220325 = or disjoint i128 %t9340, %t9338
  store i128 %v220325, ptr %t926, align 16
  store i128 %v219615, ptr %t931, align 16
  %t9349 = zext i64 %v196526 to i128
  %t9350 = shl nuw i128 %t9349, 64
  %v220329 = or disjoint i128 %t9350, %t1163
  store i128 %v220329, ptr %t936, align 16
  store i64 %v196546, ptr %t941, align 16
  %t9354 = call fastcc { i64, i64 } @mech_compact_numeric_8169(i64 %v219576, i64 %v8481)
  %v196532 = extractvalue { i64, i64 } %t9354, 0
  %t9355.not = icmp eq i64 %v196532, 0
  br i1 %t9355.not, label %edge15839, label %edge15713

edge15713:                                        ; preds = %edge15711
  %v196533 = extractvalue { i64, i64 } %t9354, 1
  %t9356 = icmp eq i64 %v196532, 103169
  %v196536 = select i1 %t9356, i64 103425, i64 %v196532
  %t9358 = insertvalue { i64, i64 } poison, i64 %v196536, 0
  %t9359 = insertvalue { i64, i64 } %t9358, i64 %v196533, 1
  br label %common.ret

edge15727:                                        ; preds = %block8027, %edge15707
  %exitcond21700.not = icmp eq i64 %v196549, %v54962
  br i1 %exitcond21700.not, label %edge15839, label %edge15733

edge15733:                                        ; preds = %edge15727
  %v196747 = add i64 %v196549, %v54961
  %t9363.not = icmp ult i64 %v196747, %v54961
  br i1 %t9363.not, label %common.ret, label %edge15737

edge15737:                                        ; preds = %edge15733
  %t9367 = icmp ult i64 %v196747, %v3720
  br i1 %t9367, label %edge15743, label %edge15759

edge15743:                                        ; preds = %edge15737
  %v196923 = shl nuw nsw i64 %v196747, 2
  %v196924 = add i64 %v196923, %v196550
  %t9370 = inttoptr i64 %v196924 to ptr
  %t9371 = load i32, ptr %t9370, align 1
  %v196925 = zext i32 %t9371 to i64
  br label %edge15759

edge15759:                                        ; preds = %edge15737, %edge15743
  %v197171 = phi i64 [ 0, %edge15737 ], [ %v196925, %edge15743 ]
  %exitcond21701.not = icmp eq i64 %v196549, %v3720
  br i1 %exitcond21701.not, label %block99672, label %edge15762

edge15762:                                        ; preds = %edge15759
  %v197128 = shl i64 %v196549, 2
  %v197129 = add i64 %v196550, %v197128
  %t9378 = inttoptr i64 %v197129 to ptr
  %t9379 = load i32, ptr %t9378, align 1
  %v197130 = zext i32 %t9379 to i64
  %t9380 = icmp eq i64 %v197171, %v197130
  br i1 %t9380, label %block99675, label %edge15765

edge15765:                                        ; preds = %edge15762
  br i1 %t9328.not, label %block99677, label %edge15766

edge15766:                                        ; preds = %edge15765
  %t9383 = inttoptr i64 %v196526 to ptr
  %v197146 = load i64, ptr %t9383, align 1
  %t9384 = icmp eq i64 %v197146, 1
  br i1 %t9384, label %block99676, label %block99677

edge15771:                                        ; preds = %block99677
  %v197156 = load i64, ptr %ss1, align 8
  %v197157 = load i64, ptr %t606, align 8
  %v197158 = load i64, ptr %t607, align 8
  %.pre21727 = add i64 %v197156, %v197128
  %.pre21728 = inttoptr i64 %.pre21727 to ptr
  br label %block99676

edge15774:                                        ; preds = %block99675
  %t9408 = inttoptr i64 %v197139 to ptr
  %v197163 = load i64, ptr %t9408, align 1
  %v197164 = add i64 %v197163, 1
  store i64 %v197164, ptr %t9408, align 1
  br label %block99681

edge15779:                                        ; preds = %block99681
  %v197169 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v196526)
  %t9412.not = icmp eq i64 %v197169, 0
  br i1 %t9412.not, label %edge15794, label %block99672

edge15794:                                        ; preds = %block99681, %edge15779
  %v197354 = add nuw nsw i64 %v196549, 1
  br label %block8027

edge15819:                                        ; preds = %edge15842
  %t9426 = insertvalue { i64, i64 } poison, i64 %v198450, 0
  %t9427 = insertvalue { i64, i64 } %t9426, i64 8226, 1
  br label %common.ret

edge15839:                                        ; preds = %edge15727, %edge15711
  %v198512 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3194)
  %t9440.not = icmp eq i64 %v198512, 0
  br i1 %t9440.not, label %edge15841, label %block99860

edge15841:                                        ; preds = %edge15839
  %v198515 = load i64, ptr %ss1, align 8
  %v198516 = load i64, ptr %t606, align 8
  %v198517 = load i64, ptr %t607, align 8
  %t9443.not = icmp eq i64 %v198517, 0
  br i1 %t9443.not, label %edge15882, label %edge15842

edge15842:                                        ; preds = %edge15841
  %t9444 = inttoptr i64 %v198517 to ptr
  %v198518 = load i64, ptr %t9444, align 1
  %v198446 = add i64 %v198518, 2
  store i64 %v198446, ptr %t9444, align 1
  %v198450 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v198517)
  %t9425.not = icmp eq i64 %v198450, 0
  br i1 %t9425.not, label %edge15882, label %edge15819

edge15861:                                        ; preds = %edge15885
  %v199685 = call i64 @mech_compact_free(i64 %v0, i64 %v199742)
  %t9459.not = icmp eq i64 %v199685, 0
  br i1 %t9459.not, label %edge16110, label %edge15862

edge15862:                                        ; preds = %edge15861
  %t9460 = insertvalue { i64, i64 } poison, i64 %v199685, 0
  %t9461 = insertvalue { i64, i64 } %t9460, i64 8239, 1
  br label %common.ret

edge15882:                                        ; preds = %edge15842, %edge15841
  %v199737 = call i64 @mech_compact_allocate(i64 %v0, i64 259, i64 4, i64 4, i64 1, i64 %v3194)
  %t9474.not = icmp eq i64 %v199737, 0
  br i1 %t9474.not, label %edge15884, label %block99915

edge15884:                                        ; preds = %edge15882
  %v199740 = load i64, ptr %ss1, align 8
  %v199741 = load i64, ptr %t606, align 8
  %v199742 = load i64, ptr %t607, align 8
  %t9477.not = icmp eq i64 %v199742, 0
  br i1 %t9477.not, label %edge16110, label %edge15885

edge15885:                                        ; preds = %edge15884
  %t9478 = inttoptr i64 %v199742 to ptr
  %v199743 = load i64, ptr %t9478, align 1
  %v199744 = add i64 %v199743, 1
  store i64 %v199744, ptr %t9478, align 1
  %t9458.not = icmp eq i64 %v199744, 0
  br i1 %t9458.not, label %edge15861, label %edge16110

edge15915:                                        ; preds = %edge15938
  %t9489 = insertvalue { i64, i64 } poison, i64 %v201047, 0
  %t9490 = insertvalue { i64, i64 } %t9489, i64 8256, 1
  br label %common.ret

edge15935:                                        ; preds = %block99166, %edge15695
  %v201095 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3194)
  %t9503.not = icmp eq i64 %v201095, 0
  br i1 %t9503.not, label %edge15937, label %block100014

edge15937:                                        ; preds = %edge15935
  %v201098 = load i64, ptr %ss1, align 8
  %v201099 = load i64, ptr %t606, align 8
  %v201100 = load i64, ptr %t607, align 8
  %t9506.not = icmp eq i64 %v201100, 0
  br i1 %t9506.not, label %edge15975, label %edge15938

edge15938:                                        ; preds = %edge15937
  %t9507 = inttoptr i64 %v201100 to ptr
  %v201101 = load i64, ptr %t9507, align 1
  %v201043 = add i64 %v201101, 2
  store i64 %v201043, ptr %t9507, align 1
  %v201047 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v201100)
  %t9488.not = icmp eq i64 %v201047, 0
  br i1 %t9488.not, label %edge15975, label %edge15915

edge15955:                                        ; preds = %edge15978
  %t9518 = insertvalue { i64, i64 } poison, i64 %v202268, 0
  %t9519 = insertvalue { i64, i64 } %t9518, i64 8269, 1
  br label %common.ret

edge15975:                                        ; preds = %edge15938, %edge15937
  %v202316 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3194)
  %t9532.not = icmp eq i64 %v202316, 0
  br i1 %t9532.not, label %edge15977, label %block100069

edge15977:                                        ; preds = %edge15975
  %v202319 = load i64, ptr %ss1, align 8
  %v202320 = load i64, ptr %t606, align 8
  %v202321 = load i64, ptr %t607, align 8
  %t9535.not = icmp eq i64 %v202321, 0
  br i1 %t9535.not, label %edge16015, label %edge15978

edge15978:                                        ; preds = %edge15977
  %t9536 = inttoptr i64 %v202321 to ptr
  %v202322 = load i64, ptr %t9536, align 1
  %v202264 = add i64 %v202322, 2
  store i64 %v202264, ptr %t9536, align 1
  %v202268 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v202321)
  %t9517.not = icmp eq i64 %v202268, 0
  br i1 %t9517.not, label %edge16015, label %edge15955

edge15995:                                        ; preds = %edge16018
  %t9547 = insertvalue { i64, i64 } poison, i64 %v203501, 0
  %t9548 = insertvalue { i64, i64 } %t9547, i64 8282, 1
  br label %common.ret

edge16015:                                        ; preds = %edge15978, %edge15977
  %v203553 = call i64 @mech_compact_allocate(i64 %v0, i64 259, i64 4, i64 4, i64 1, i64 %v3194)
  %t9561.not = icmp eq i64 %v203553, 0
  br i1 %t9561.not, label %edge16017, label %block100124

edge16017:                                        ; preds = %edge16015
  %v203556 = load i64, ptr %ss1, align 8
  %v203557 = load i64, ptr %t606, align 8
  %v203558 = load i64, ptr %t607, align 8
  %t9564.not = icmp eq i64 %v203558, 0
  br i1 %t9564.not, label %edge16058, label %edge16018

edge16018:                                        ; preds = %edge16017
  %t9565 = inttoptr i64 %v203558 to ptr
  %v203559 = load i64, ptr %t9565, align 1
  %v203497 = add i64 %v203559, 2
  store i64 %v203497, ptr %t9565, align 1
  %v203501 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v203558)
  %t9546.not = icmp eq i64 %v203501, 0
  br i1 %t9546.not, label %edge16058, label %edge15995

edge16037:                                        ; preds = %edge16061
  %v204750 = call i64 @mech_compact_free(i64 %v0, i64 %v204803)
  %t9580.not = icmp eq i64 %v204750, 0
  br i1 %t9580.not, label %edge16110, label %edge16038

edge16038:                                        ; preds = %edge16037
  %t9581 = insertvalue { i64, i64 } poison, i64 %v204750, 0
  %t9582 = insertvalue { i64, i64 } %t9581, i64 8295, 1
  br label %common.ret

edge16058:                                        ; preds = %edge16018, %edge16017
  %v204798 = call i64 @mech_compact_allocate(i64 %v0, i64 259, i64 4, i64 4, i64 1, i64 %v3194)
  %t9595.not = icmp eq i64 %v204798, 0
  br i1 %t9595.not, label %edge16060, label %block100179

edge16060:                                        ; preds = %edge16058
  %v204801 = load i64, ptr %ss1, align 8
  %v204802 = load i64, ptr %t606, align 8
  %v204803 = load i64, ptr %t607, align 8
  %t9598.not = icmp eq i64 %v204803, 0
  br i1 %t9598.not, label %edge16110, label %edge16061

edge16061:                                        ; preds = %edge16060
  %t9599 = inttoptr i64 %v204803 to ptr
  %v204804 = load i64, ptr %t9599, align 1
  %v204805 = add i64 %v204804, 1
  store i64 %v204805, ptr %t9599, align 1
  %t9579.not = icmp eq i64 %v204805, 0
  br i1 %t9579.not, label %edge16037, label %edge16110

edge16110:                                        ; preds = %edge16060, %edge15884, %edge15861, %edge15885, %edge16037, %edge16061
  %v205562 = phi i64 [ %v54921, %edge16060 ], [ %v196550, %edge15861 ], [ %v196550, %edge15885 ], [ %v54921, %edge16037 ], [ %v54921, %edge16061 ], [ %v196550, %edge15884 ]
  %v205564 = phi i64 [ %v54923, %edge16060 ], [ %v196546, %edge15861 ], [ %v196546, %edge15885 ], [ %v54923, %edge16037 ], [ %v54923, %edge16061 ], [ %v196546, %edge15884 ]
  %v205567 = phi i64 [ %v54926, %edge16060 ], [ %v196526, %edge15861 ], [ %v196526, %edge15885 ], [ %v54926, %edge16037 ], [ %v54926, %edge16061 ], [ %v196526, %edge15884 ]
  %v205929 = phi i64 [ %v54929, %edge16060 ], [ %v54960, %edge15861 ], [ %v54960, %edge15885 ], [ %v54929, %edge16037 ], [ %v54929, %edge16061 ], [ %v54960, %edge15884 ]
  %v206116 = phi i64 [ 0, %edge16060 ], [ %v54962, %edge15861 ], [ %v54962, %edge15885 ], [ 0, %edge16037 ], [ 0, %edge16061 ], [ %v54962, %edge15884 ]
  %v206119 = phi i64 [ %v204801, %edge16060 ], [ %v199740, %edge15861 ], [ %v199740, %edge15885 ], [ %v204801, %edge16037 ], [ %v204801, %edge16061 ], [ %v199740, %edge15884 ]
  %v206121 = phi i64 [ %v204802, %edge16060 ], [ %v199741, %edge15861 ], [ %v199741, %edge15885 ], [ %v204802, %edge16037 ], [ %v204802, %edge16061 ], [ %v199741, %edge15884 ]
  %v206124 = phi i64 [ 0, %edge16060 ], [ %v199742, %edge15861 ], [ %v199742, %edge15885 ], [ %v204803, %edge16037 ], [ %v204803, %edge16061 ], [ 0, %edge15884 ]
  %v206131 = phi i64 [ %v203556, %edge16060 ], [ %v54939, %edge15861 ], [ %v54939, %edge15885 ], [ %v203556, %edge16037 ], [ %v203556, %edge16061 ], [ %v54939, %edge15884 ]
  %v206133 = phi i64 [ %v203557, %edge16060 ], [ %v54941, %edge15861 ], [ %v54941, %edge15885 ], [ %v203557, %edge16037 ], [ %v203557, %edge16061 ], [ %v54941, %edge15884 ]
  %v206136 = phi i64 [ %v203558, %edge16060 ], [ %v54944, %edge15861 ], [ %v54944, %edge15885 ], [ %v203558, %edge16037 ], [ %v203558, %edge16061 ], [ %v54944, %edge15884 ]
  %v206706 = phi i64 [ %v202319, %edge16060 ], [ %v198515, %edge15861 ], [ %v198515, %edge15885 ], [ %v202319, %edge16037 ], [ %v202319, %edge16061 ], [ %v198515, %edge15884 ]
  %v206708 = phi i64 [ %v202320, %edge16060 ], [ %v198516, %edge15861 ], [ %v198516, %edge15885 ], [ %v202320, %edge16037 ], [ %v202320, %edge16061 ], [ %v198516, %edge15884 ]
  %v206711 = phi i64 [ %v202321, %edge16060 ], [ %v198517, %edge15861 ], [ %v198517, %edge15885 ], [ %v202321, %edge16037 ], [ %v202321, %edge16061 ], [ %v198517, %edge15884 ]
  %v206718 = phi i64 [ %v201098, %edge16060 ], [ %v54951, %edge15861 ], [ %v54951, %edge15885 ], [ %v201098, %edge16037 ], [ %v201098, %edge16061 ], [ %v54951, %edge15884 ]
  %v206720 = phi i64 [ %v201099, %edge16060 ], [ %v54953, %edge15861 ], [ %v54953, %edge15885 ], [ %v201099, %edge16037 ], [ %v201099, %edge16061 ], [ %v54953, %edge15884 ]
  %v206723 = phi i64 [ %v201100, %edge16060 ], [ %v54956, %edge15861 ], [ %v54956, %edge15885 ], [ %v201100, %edge16037 ], [ %v201100, %edge16061 ], [ %v54956, %edge15884 ]
  %v206733 = phi i64 [ 0, %edge16060 ], [ %v54958, %edge15861 ], [ %v54958, %edge15885 ], [ 0, %edge16037 ], [ 0, %edge16061 ], [ %v54958, %edge15884 ]
  %t9603 = icmp eq i64 %v54929, %v1397
  br i1 %t9603, label %edge16129, label %edge16243

edge16129:                                        ; preds = %edge16110
  br i1 %t23.not, label %block100389, label %edge16132

edge16132:                                        ; preds = %edge16129
  %v207002 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1401)
  %t9607.not = icmp eq i64 %v207002, 0
  br i1 %t9607.not, label %block100389, label %block100317

edge16144:                                        ; preds = %block100389
  %v207010 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v140768)
  %t9611.not = icmp eq i64 %v207010, 0
  br i1 %t9611.not, label %block100393, label %block100317

edge16150:                                        ; preds = %block100393
  %v207014 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v219119)
  %t9613.not = icmp eq i64 %v207014, 0
  br i1 %t9613.not, label %block100397, label %block100317

edge16156:                                        ; preds = %block100397
  %v207018 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54920)
  %t9615.not = icmp eq i64 %v207018, 0
  br i1 %t9615.not, label %block100401, label %block100317

edge16162:                                        ; preds = %block100401
  %v207022 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v205567)
  %t9617.not = icmp eq i64 %v207022, 0
  br i1 %t9617.not, label %block100405, label %block100317

edge16168:                                        ; preds = %block100405
  %v207026 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v7458422199)
  %t9619.not = icmp eq i64 %v207026, 0
  br i1 %t9619.not, label %block100409, label %block100317

edge16174:                                        ; preds = %block100409
  %v207030 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v206124)
  %t9621.not = icmp eq i64 %v207030, 0
  br i1 %t9621.not, label %block100413, label %block100317

edge16180:                                        ; preds = %block100413
  %v207034 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v206136)
  %t9623.not = icmp eq i64 %v207034, 0
  br i1 %t9623.not, label %block100417, label %block100317

edge16186:                                        ; preds = %block100417
  %v207038 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54950)
  %t9625.not = icmp eq i64 %v207038, 0
  br i1 %t9625.not, label %block100421, label %block100317

edge16192:                                        ; preds = %block100421
  %v207042 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v206711)
  %t9627.not = icmp eq i64 %v207042, 0
  br i1 %t9627.not, label %block100425, label %block100317

edge16196:                                        ; preds = %block100425
  %t9629 = inttoptr i64 %v206723 to ptr
  %v207044 = load i64, ptr %t9629, align 1
  %v207045 = add i64 %v207044, -1
  store i64 %v207045, ptr %t9629, align 1
  %t9631.not = icmp eq i64 %v207045, 0
  br i1 %t9631.not, label %edge16199, label %edge16248

edge16199:                                        ; preds = %edge16196
  %v207046 = call i64 @mech_compact_free(i64 %v0, i64 %v206723)
  %t9632.not = icmp eq i64 %v207046, 0
  br i1 %t9632.not, label %edge16248, label %block100317

edge16243:                                        ; preds = %edge16110
  %t854.not = icmp ult i64 %v1397, %v205929
  br i1 %t854.not, label %common.ret, label %block10120

edge16248:                                        ; preds = %edge16199, %edge16196, %block100425
  %t9635.not = icmp eq i64 %v206874, 0
  br i1 %t9635.not, label %edge16309, label %edge16257

edge16257:                                        ; preds = %edge16248
  %t9639 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v206782, i64 1)
  %v218632 = extractvalue { i64, i1 } %t9639, 0
  %t9640 = extractvalue { i64, i1 } %t9639, 1
  br i1 %t9640, label %block100709, label %edge16262

edge16262:                                        ; preds = %edge16257
  %t9643.not = icmp ugt i64 %v218632, %v206796
  br i1 %t9643.not, label %edge16265, label %block100713

edge16265:                                        ; preds = %edge16262
  %t9645.not = icmp eq i64 %v206796, 0
  br i1 %t9645.not, label %block100716, label %edge16266

edge16266:                                        ; preds = %edge16265
  %131 = add i64 %v206796, -131073
  %v218642270 = icmp ult i64 %131, -126977
  br i1 %v218642270, label %edge16269, label %edge16278

edge16269:                                        ; preds = %edge16266
  %t9649 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v206796, i64 %v206796)
  %v218643 = extractvalue { i64, i1 } %t9649, 0
  %t9650 = extractvalue { i64, i1 } %t9649, 1
  br i1 %t9650, label %block100709, label %block100716

edge16278:                                        ; preds = %edge16266
  %t9657 = mul nuw nsw i64 %v206796, 3
  %t9661 = add nuw nsw i64 %t9657, 1
  %v218660 = lshr i64 %t9661, 1
  br label %block100716

edge16282:                                        ; preds = %block100713
  %t9667 = inttoptr i64 %v206838 to ptr
  %v218668 = load i64, ptr %t9667, align 1
  %t9668 = icmp eq i64 %v218668, 1
  %t9669 = icmp ule i64 %v218637, %v206796
  %v218671271 = and i1 %t9669, %t9668
  br i1 %v218671271, label %block100723, label %block100724

edge16287:                                        ; preds = %block100724
  %v218678 = load i64, ptr %ss1, align 8
  %v218679 = load i64, ptr %t606, align 8
  %v218680 = load i64, ptr %t607, align 8
  br label %block100723

edge16289:                                        ; preds = %block100723
  %t9692 = inttoptr i64 %v218667 to ptr
  %v218687 = load i64, ptr %t9692, align 1
  %v218688 = add i64 %v218687, 1
  store i64 %v218688, ptr %t9692, align 1
  br label %block100728

edge16294:                                        ; preds = %block100728
  %v218693 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v206838)
  %t9696.not = icmp eq i64 %v218693, 0
  br i1 %t9696.not, label %edge16309, label %block100709

edge16309:                                        ; preds = %edge166, %edge163, %block8482, %edge409, %edge411, %edge414, %edge16294, %block100728, %edge16248
  %v218735 = phi i64 [ %v2795, %edge414 ], [ %v2019, %block8482 ], [ %v2019, %edge166 ], [ %v2019, %edge163 ], [ %v2795, %edge409 ], [ %v2795, %edge411 ], [ %v218662, %block100728 ], [ %v218662, %edge16294 ], [ %v206768, %edge16248 ]
  %v218736 = phi i64 [ %v2765, %edge414 ], [ %v1989, %block8482 ], [ %v1989, %edge166 ], [ %v1989, %edge163 ], [ %v2765, %edge409 ], [ %v2765, %edge411 ], [ %v218632, %block100728 ], [ %v218632, %edge16294 ], [ %v206782, %edge16248 ]
  %v218737 = phi i64 [ %v2797, %edge414 ], [ %v2021, %block8482 ], [ %v2021, %edge166 ], [ %v2021, %edge163 ], [ %v2797, %edge409 ], [ %v2797, %edge411 ], [ %v218664, %block100728 ], [ %v218664, %edge16294 ], [ %v206796, %edge16248 ]
  %v218740 = phi i64 [ %v2800, %edge414 ], [ %v2024, %block8482 ], [ %v2024, %edge166 ], [ %v2024, %edge163 ], [ %v2800, %edge409 ], [ %v2800, %edge411 ], [ %v218667, %block100728 ], [ %v218667, %edge16294 ], [ %v206838, %edge16248 ]
  %t9699.not = icmp eq i64 %v218740, 0
  br i1 %t9699.not, label %block100765, label %edge16310

edge16310:                                        ; preds = %edge16309
  %t9700 = inttoptr i64 %v218740 to ptr
  %v218749 = load i64, ptr %t9700, align 1
  %v218750 = add i64 %v218749, 1
  store i64 %v218750, ptr %t9700, align 1
  %v218754 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v218740)
  %t9702.not = icmp eq i64 %v218754, 0
  br i1 %t9702.not, label %block100765, label %edge16315

edge16315:                                        ; preds = %edge16310
  %t9716 = insertvalue { i64, i64 } poison, i64 %v218754, 0
  %t9717 = insertvalue { i64, i64 } %t9716, i64 8365, 1
  br label %common.ret
}

; Function Attrs: cold noreturn nounwind memory(inaccessiblemem: write)
declare void @llvm.trap() #1

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.uadd.with.overflow.i64(i64, i64) #2

declare i64 @mech_compact_writable(i64, i64, i64, i64, i64, i64, i64) local_unnamed_addr #3

declare i64 @mech_compact_release_owner(i64, i64) local_unnamed_addr #3

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umax.i64(i64, i64) #2

declare i64 @mech_compact_free(i64, i64) local_unnamed_addr #3

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #2

declare i64 @memmove(i64, i64, i64) local_unnamed_addr #3

declare i64 @mech_compact_allocate(i64, i64, i64, i64, i64, i64) local_unnamed_addr #3

declare { i64, i64 } @mech_compact_numeric_475_vector(i64, i64, i64) local_unnamed_addr #3

declare i64 @mech_compact_word_collection_10(i64, i64, i64, i64) local_unnamed_addr #3

declare i64 @mech_compact_matrix_word(i64, i64, i64, i64, i64) local_unnamed_addr #3

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.cttz.i64(i64, i1 immarg) #4

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64) #2

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64) #2

declare i64 @mech_compact_unsigned_dot_product(i64, i64, i64, i64) local_unnamed_addr #3

; Function Attrs: noinline nounwind memory(readwrite, inaccessiblemem: write, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_2821(i64 %v1, i64 %v2) unnamed_addr #5 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 88
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 96
  %v6 = load i64, ptr %t3, align 1
  %v14 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v15 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v16 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v17 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 32
  %v18 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 40
  %v19 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 48
  %v20 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 56
  %v21 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 64
  %v22 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 72
  %v23 = load i64, ptr %t22, align 1
  %t24 = getelementptr i8, ptr %t0, i64 80
  %v24 = load i64, ptr %t24, align 1
  %t32 = icmp eq i64 %v19, 1
  %t33 = icmp eq i64 %v20, 1
  %v1261 = or i1 %t32, %t33
  %t35 = icmp eq i64 %v19, 0
  %umax = tail call i64 @llvm.umax.i64(i64 %v16, i64 %v15)
  br label %block6

block6:                                           ; preds = %edge66, %block0
  %v308 = phi i64 [ %v16, %block0 ], [ %v265, %edge66 ]
  %exitcond.not = icmp eq i64 %v308, %umax
  br i1 %exitcond.not, label %edge12, label %edge14

trap36:                                           ; preds = %edge34
  tail call void @llvm.trap()
  unreachable

ok36:                                             ; preds = %edge34
  %v128 = udiv i64 %v75, %v19
  %v129 = mul i64 %v128, %v19
  %v130 = sub nuw i64 %v75, %v129
  %v131 = mul i64 %v130, %v20
  %v132 = add i64 %v131, %v128
  br label %block111

block111:                                         ; preds = %edge31, %ok36
  %v127 = phi i64 [ %v132, %ok36 ], [ %v75, %edge31 ]
  %v135 = add i64 %v127, %v17
  %t37 = inttoptr i64 %v135 to ptr
  %t38 = load i8, ptr %t37, align 1
  %v136 = zext i8 %t38 to i64
  br label %edge48

common.ret:                                       ; preds = %edge81, %edge12
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge12 ], [ %t74, %edge81 ]
  ret { i64, i64 } %common.ret.op

edge12:                                           ; preds = %block6
  %t50 = inttoptr i64 %v2 to ptr
  %t51 = getelementptr i8, ptr %t50, i64 8
  store i64 %v21, ptr %t51, align 1
  %t53 = getelementptr i8, ptr %t50, i64 16
  store i64 %v22, ptr %t53, align 1
  %t55 = getelementptr i8, ptr %t50, i64 24
  store i64 %v23, ptr %t55, align 1
  %t57 = getelementptr i8, ptr %t50, i64 32
  store i64 %v24, ptr %t57, align 1
  %t59 = getelementptr i8, ptr %t50, i64 40
  store i64 %v21, ptr %t59, align 1
  %t61 = getelementptr i8, ptr %t50, i64 48
  store i64 %v22, ptr %t61, align 1
  %t63 = getelementptr i8, ptr %t50, i64 56
  store i64 %v6, ptr %t63, align 1
  %t65 = getelementptr i8, ptr %t50, i64 64
  store i64 %v23, ptr %t65, align 1
  %t67 = getelementptr i8, ptr %t50, i64 72
  store i64 %v24, ptr %t67, align 1
  %t69 = getelementptr i8, ptr %t50, i64 80
  store i64 %v5, ptr %t69, align 1
  br label %common.ret

edge14:                                           ; preds = %block6
  %v75 = add i64 %v308, %v14
  %t27.not = icmp ult i64 %v75, %v14
  br i1 %t27.not, label %edge81, label %edge25

edge25:                                           ; preds = %edge14
  %t30 = icmp ult i64 %v75, %v18
  br i1 %t30, label %edge31, label %edge48

edge31:                                           ; preds = %edge25
  br i1 %v1261, label %block111, label %edge34

edge34:                                           ; preds = %edge31
  br i1 %t35, label %trap36, label %ok36

edge48:                                           ; preds = %block111, %edge25
  %v317 = phi i64 [ %v136, %block111 ], [ 0, %edge25 ]
  %t39 = icmp ult i64 %v317, %v22
  br i1 %t39, label %edge60, label %edge81

edge60:                                           ; preds = %edge48
  %v198 = shl nuw nsw i64 %v317, 2
  %v200 = add i64 %v198, %v21
  %t41 = inttoptr i64 %v200 to ptr
  %t42 = load i32, ptr %t41, align 1
  %t43.not = icmp eq i32 %t42, -1
  br i1 %t43.not, label %edge81, label %edge66

edge66:                                           ; preds = %edge60
  %v240 = add nuw i32 %t42, 1
  store i32 %v240, ptr %t41, align 1
  %v265 = add i64 %v308, 1
  br label %block6

edge81:                                           ; preds = %edge48, %edge60, %edge14
  %.sink = phi i64 [ 2879, %edge60 ], [ 2837, %edge14 ], [ 2881, %edge48 ]
  %v27 = phi i64 [ 1, %edge60 ], [ 1, %edge14 ], [ 103169, %edge48 ]
  %t29 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t29, align 1
  %t73 = insertvalue { i64, i64 } poison, i64 %v27, 0
  %t74 = insertvalue { i64, i64 } %t73, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_2821_vector(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 88
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 96
  %v6 = load i64, ptr %t3, align 1
  %v14 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v15 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v16 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v17 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 32
  %v18 = load i64, ptr %t12, align 1
  %t18 = getelementptr i8, ptr %t0, i64 56
  %v21 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 64
  %v22 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 72
  %v23 = load i64, ptr %t22, align 1
  %t24 = getelementptr i8, ptr %t0, i64 80
  %v24 = load i64, ptr %t24, align 1
  %umax = tail call i64 @llvm.umax.i64(i64 %v16, i64 %v15)
  br label %block6

block6:                                           ; preds = %edge63, %block0
  %v299 = phi i64 [ %v16, %block0 ], [ %v256, %edge63 ]
  %exitcond.not = icmp eq i64 %v299, %umax
  br i1 %exitcond.not, label %edge12, label %edge14

common.ret:                                       ; preds = %edge78, %edge12
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge12 ], [ %t69, %edge78 ]
  ret { i64, i64 } %common.ret.op

edge12:                                           ; preds = %block6
  %t45 = inttoptr i64 %v2 to ptr
  %t46 = getelementptr i8, ptr %t45, i64 8
  store i64 %v21, ptr %t46, align 1
  %t48 = getelementptr i8, ptr %t45, i64 16
  store i64 %v22, ptr %t48, align 1
  %t50 = getelementptr i8, ptr %t45, i64 24
  store i64 %v23, ptr %t50, align 1
  %t52 = getelementptr i8, ptr %t45, i64 32
  store i64 %v24, ptr %t52, align 1
  %t54 = getelementptr i8, ptr %t45, i64 40
  store i64 %v21, ptr %t54, align 1
  %t56 = getelementptr i8, ptr %t45, i64 48
  store i64 %v22, ptr %t56, align 1
  %t58 = getelementptr i8, ptr %t45, i64 56
  store i64 %v6, ptr %t58, align 1
  %t60 = getelementptr i8, ptr %t45, i64 64
  store i64 %v23, ptr %t60, align 1
  %t62 = getelementptr i8, ptr %t45, i64 72
  store i64 %v24, ptr %t62, align 1
  %t64 = getelementptr i8, ptr %t45, i64 80
  store i64 %v5, ptr %t64, align 1
  br label %common.ret

edge14:                                           ; preds = %block6
  %v75 = add i64 %v299, %v14
  %t27.not = icmp ult i64 %v75, %v14
  br i1 %t27.not, label %edge78, label %edge25

edge25:                                           ; preds = %edge14
  %t30 = icmp ult i64 %v75, %v18
  br i1 %t30, label %edge31, label %edge45

edge31:                                           ; preds = %edge25
  %v126 = add i64 %v75, %v17
  %t32 = inttoptr i64 %v126 to ptr
  %t33 = load i8, ptr %t32, align 1
  %v127 = zext i8 %t33 to i64
  br label %edge45

edge45:                                           ; preds = %edge31, %edge25
  %v309 = phi i64 [ %v127, %edge31 ], [ 0, %edge25 ]
  %t34 = icmp ult i64 %v309, %v22
  br i1 %t34, label %edge57, label %edge78

edge57:                                           ; preds = %edge45
  %v189 = shl nuw nsw i64 %v309, 2
  %v191 = add i64 %v189, %v21
  %t36 = inttoptr i64 %v191 to ptr
  %t37 = load i32, ptr %t36, align 1
  %t38.not = icmp eq i32 %t37, -1
  br i1 %t38.not, label %edge78, label %edge63

edge63:                                           ; preds = %edge57
  %v231 = add nuw i32 %t37, 1
  store i32 %v231, ptr %t36, align 1
  %v256 = add i64 %v299, 1
  br label %block6

edge78:                                           ; preds = %edge45, %edge57, %edge14
  %.sink = phi i64 [ 2879, %edge57 ], [ 2837, %edge14 ], [ 2881, %edge45 ]
  %v27 = phi i64 [ 1, %edge57 ], [ 1, %edge14 ], [ 103169, %edge45 ]
  %t29 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t29, align 1
  %t68 = insertvalue { i64, i64 } poison, i64 %v27, 0
  %t69 = insertvalue { i64, i64 } %t68, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: noinline uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_3009(i64 %v0, i64 %v1, i64 %v2) unnamed_addr #0 align 128 {
block0:
  %ss0 = alloca [48 x i8], align 8
  %ss1 = alloca [48 x i8], align 8
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 152
  %v5 = load i64, ptr %t1, align 1
  %v25 = load i64, ptr %t0, align 1
  %t4 = getelementptr i8, ptr %t0, i64 8
  %v26 = load i64, ptr %t4, align 1
  %t6 = getelementptr i8, ptr %t0, i64 16
  %v27 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 24
  %v28 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 32
  %v29 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 40
  %v30 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 48
  %v31 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 56
  %v32 = load i64, ptr %t16, align 1
  %t22 = getelementptr i8, ptr %t0, i64 80
  %v35 = load i64, ptr %t22, align 1
  %t24 = getelementptr i8, ptr %t0, i64 88
  %v36 = load i64, ptr %t24, align 1
  %t30 = getelementptr i8, ptr %t0, i64 112
  %v39 = load i64, ptr %t30, align 1
  %t32 = getelementptr i8, ptr %t0, i64 120
  %v40 = load i64, ptr %t32, align 1
  %t38 = getelementptr i8, ptr %t0, i64 144
  %v47 = load i64, ptr %t38, align 1
  %v148 = add i64 %v31, -4
  %v899 = add i64 %v35, 12
  %v1615 = ptrtoint ptr %ss1 to i64
  %t155 = getelementptr inbounds nuw i8, ptr %ss0, i64 8
  %t156 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  %t157 = getelementptr inbounds nuw i8, ptr %ss0, i64 24
  %t158 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  %t159 = getelementptr inbounds nuw i8, ptr %ss0, i64 40
  %v1619 = ptrtoint ptr %ss0 to i64
  %t161 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %t162 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  br label %block6

block6:                                           ; preds = %edge323, %block0
  %v2120 = phi i64 [ %v25, %block0 ], [ %v96, %edge323 ]
  %v2121 = phi i64 [ %v26, %block0 ], [ %v450, %edge323 ]
  %v2181 = phi i64 [ %v28, %block0 ], [ %v1632.pre-phi, %edge323 ]
  %v2193 = phi i64 [ %v30, %block0 ], [ %v1632.pre-phi, %edge323 ]
  %v2198 = phi i64 [ %v27, %block0 ], [ %v3917, %edge323 ]
  %v2691 = phi i64 [ %v47, %block0 ], [ %v3938, %edge323 ]
  %v2694 = phi i64 [ %v5, %block0 ], [ %v3939, %edge323 ]
  %t39.not = icmp eq i64 %v2120, 0
  br i1 %t39.not, label %edge13, label %edge15

block125:                                         ; preds = %block347, %edge100
  %v2222 = phi i64 [ %v450, %edge100 ], [ %v1421, %block347 ]
  %v2223 = phi i64 [ 0, %edge100 ], [ %v4622, %block347 ]
  %v2224 = phi i64 [ %v406, %edge100 ], [ %v4620, %block347 ]
  %v2225 = phi i64 [ %v435, %edge100 ], [ %v4621, %block347 ]
  %t117.not = icmp eq i64 %v2222, %v101
  br i1 %t117.not, label %edge272, label %edge261

block755:                                         ; preds = %edge239, %edge236
  %v917 = phi i64 [ %v2223, %edge236 ], [ %v941, %edge239 ]
  %v918 = phi <4 x i32> [ %v914, %edge236 ], [ %t92, %edge239 ]
  %v919 = phi <4 x i32> [ splat (i32 -1), %edge236 ], [ %v940, %edge239 ]
  %t83 = icmp ult i64 %v917, %v913
  br i1 %t83, label %edge239, label %edge240

block752:                                         ; preds = %block752.preheader, %block752
  %v905 = phi i64 [ %v976, %block752 ], [ %v905.ph, %block752.preheader ]
  %v906 = phi i32 [ %t110, %block752 ], [ %v906.ph, %block752.preheader ]
  %v907 = phi i32 [ %v975, %block752 ], [ %v907.ph, %block752.preheader ]
  %v964 = shl i64 %v905, 2
  %v965 = add i64 %v964, %v899
  %t107 = inttoptr i64 %v965 to ptr
  %v966 = load i32, ptr %t107, align 1
  %v967 = add i32 %v966, %v2265
  %v968 = shl i64 %v905, 3
  %v969 = sub i64 %v904.reass, %v968
  %t108 = inttoptr i64 %v969 to ptr
  %v970 = load i32, ptr %t108, align 1
  %v971 = add i32 %v967, %v970
  %t109 = icmp ult i32 %v971, %v906
  %t110 = call i32 @llvm.umin.i32(i32 %v971, i32 %v906)
  %v974 = trunc i64 %v905 to i32
  %v975 = select i1 %t109, i32 %v974, i32 %v907
  %v976 = add nuw i64 %v905, 1
  %t112 = icmp ult i64 %v976, %v847
  br i1 %t112, label %block752, label %block753

block753:                                         ; preds = %block752, %edge240
  %v908 = phi i32 [ %v961, %edge240 ], [ %t110, %block752 ]
  %v909 = phi i32 [ %v962, %edge240 ], [ %v975, %block752 ]
  %t114.not = icmp eq i32 %v909, -1
  %v992 = shl nuw nsw i64 %v578, 9
  %0 = add i32 %v909, 3
  %v984.masked = zext i32 %0 to i64
  %v995 = or i64 %v992, %v984.masked
  %v978 = select i1 %t114.not, i64 %v2225, i64 %v995
  %v998 = zext i32 %v908 to i64
  br label %block347

block347:                                         ; preds = %edge229, %block753
  %v4620 = phi i64 [ %v998, %block753 ], [ %v2224, %edge229 ]
  %v4621 = phi i64 [ %v978, %block753 ], [ %v2225, %edge229 ]
  %v4622 = phi i64 [ %v847, %block753 ], [ %v2223, %edge229 ]
  %v1421 = add i64 %v2222, 1
  br label %block125

block930:                                         ; preds = %edge288, %edge292, %edge301
  %v1581 = phi i64 [ %v1586, %edge292 ], [ %v1603, %edge301 ], [ 8, %edge288 ]
  %t149 = call i64 @llvm.umax.i64(i64 %v1581, i64 %v1575)
  br label %block927

block927:                                         ; preds = %edge285, %block930
  %v1580 = phi i64 [ %t149, %block930 ], [ %v2691, %edge285 ]
  %t150.not = icmp eq i64 %v2694, 0
  br i1 %t150.not, label %block938, label %edge305

block938:                                         ; preds = %edge305, %block927
  store i64 %v2198, ptr %ss0, align 8
  store i64 %v2181, ptr %t155, align 8
  store i64 %v2691, ptr %t156, align 8
  store i64 %v29, ptr %t157, align 8
  store i64 %v2193, ptr %t158, align 8
  store i64 %v2694, ptr %t159, align 8
  %v1620 = call i64 @mech_compact_writable(i64 %v0, i64 %v1619, i64 1, i64 %v1580, i64 8, i64 8, i64 %v1615)
  %t160.not = icmp eq i64 %v1620, 0
  br i1 %t160.not, label %block937, label %edge440

block937:                                         ; preds = %block938
  %v1621 = load i64, ptr %ss1, align 8
  %v1622 = load i64, ptr %t161, align 8
  %v1623 = load i64, ptr %t162, align 8
  %t163.not = icmp eq i64 %v1623, 0
  br i1 %t163.not, label %block942, label %block937.edge312_crit_edge

block937.edge312_crit_edge:                       ; preds = %block937
  %t164.phi.trans.insert = inttoptr i64 %v1623 to ptr
  %v1624.pre = load i64, ptr %t164.phi.trans.insert, align 1
  %1 = add i64 %v1624.pre, 1
  br label %edge312

block942:                                         ; preds = %block937, %edge312
  %v161015 = phi i64 [ 0, %block937 ], [ %v161014, %edge312 ]
  %v160713 = phi i64 [ %v1622, %block937 ], [ %v160712, %edge312 ]
  %v160511 = phi i64 [ %v1621, %block937 ], [ %v160510, %edge312 ]
  br i1 %t150.not, label %edge323, label %edge315

common.ret:                                       ; preds = %edge440, %edge13
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge13 ], [ %t209, %edge440 ]
  ret { i64, i64 } %common.ret.op

edge13:                                           ; preds = %block6
  %t183 = inttoptr i64 %v2 to ptr
  %t184 = getelementptr i8, ptr %t183, i64 8
  store i64 %v2198, ptr %t184, align 1
  %t186 = getelementptr i8, ptr %t183, i64 16
  store i64 %v2181, ptr %t186, align 1
  %t188 = getelementptr i8, ptr %t183, i64 24
  store i64 %v29, ptr %t188, align 1
  %t190 = getelementptr i8, ptr %t183, i64 32
  store i64 %v2193, ptr %t190, align 1
  %t192 = getelementptr i8, ptr %t183, i64 40
  store i64 %v2691, ptr %t192, align 1
  %t194 = getelementptr i8, ptr %t183, i64 48
  store i64 %v2198, ptr %t194, align 1
  %t196 = getelementptr i8, ptr %t183, i64 56
  store i64 %v2181, ptr %t196, align 1
  %t198 = getelementptr i8, ptr %t183, i64 64
  store i64 %v2691, ptr %t198, align 1
  %t200 = getelementptr i8, ptr %t183, i64 72
  store i64 %v29, ptr %t200, align 1
  %t202 = getelementptr i8, ptr %t183, i64 80
  store i64 %v2193, ptr %t202, align 1
  %t204 = getelementptr i8, ptr %t183, i64 88
  store i64 %v2694, ptr %t204, align 1
  br label %common.ret

edge15:                                           ; preds = %block6
  %v96 = add i64 %v2120, -1
  %v101 = add i64 %v2121, -1
  %t41 = icmp ult i64 %v101, %v32
  br i1 %t41, label %edge30, label %edge37

edge30:                                           ; preds = %edge15
  %v145 = shl i64 %v2121, 2
  %v150 = add i64 %v148, %v145
  %t43 = inttoptr i64 %v150 to ptr
  %t44 = load i32, ptr %t43, align 1
  %v151 = zext i32 %t44 to i64
  br label %edge37

edge37:                                           ; preds = %edge30, %edge15
  %v2138 = phi i64 [ %v151, %edge30 ], [ 0, %edge15 ]
  %v172 = and i64 %v2138, 65535
  %v240 = lshr i64 %v2138, 16
  %v245 = add nuw nsw i64 %v240, 259
  %t48 = icmp ult i64 %v245, %v36
  br i1 %t48, label %edge73, label %edge80

edge73:                                           ; preds = %edge37
  %v292 = shl nuw nsw i64 %v245, 2
  %v294 = add i64 %v292, %v35
  %t50 = inttoptr i64 %v294 to ptr
  %t51 = load i32, ptr %t50, align 1
  %v295 = zext i32 %t51 to i64
  br label %edge80

edge80:                                           ; preds = %edge73, %edge37
  %v2177 = phi i64 [ %v295, %edge73 ], [ 0, %edge37 ]
  %t52.not = icmp eq i64 %v2181, 0
  br i1 %t52.not, label %edge100, label %edge94

edge94:                                           ; preds = %edge80
  %v361 = shl i64 %v2181, 3
  %v364 = add i64 %v361, -8
  %v366 = add i64 %v364, %v2198
  %t54 = inttoptr i64 %v366 to ptr
  %v367 = load i64, ptr %t54, align 1
  br label %edge100

edge100:                                          ; preds = %edge80, %edge94
  %v902.pre-phi = phi i64 [ %v361, %edge94 ], [ 0, %edge80 ]
  %v2203 = phi i64 [ %v367, %edge94 ], [ 0, %edge80 ]
  %v401 = add i64 %v2203, %v2177
  %v406 = and i64 %v401, 4294967295
  %v430 = shl nuw nsw i64 %v240, 9
  %v435 = or disjoint i64 %v430, 1
  %v450 = sub i64 %v101, %v172
  %invariant.op = add i64 %v2198, -24
  %v904.reass = add i64 %v902.pre-phi, %invariant.op
  %v931 = add i64 %v904.reass, -24
  br label %block125

edge147:                                          ; preds = %edge261
  %v553 = shl i64 %v2222, 2
  %v555 = add i64 %v553, %v31
  %t58 = inttoptr i64 %v555 to ptr
  %t59 = load i32, ptr %t58, align 1
  %v556 = zext i32 %t59 to i64
  br label %edge154

edge154:                                          ; preds = %edge147, %edge261
  %v2233 = phi i64 [ %v556, %edge147 ], [ 0, %edge261 ]
  %v578 = lshr i64 %v2233, 16
  %v596 = sub nsw i64 256, %v578
  %v601 = lshr i64 %v596, 29
  %v607 = and i64 %v601, 7
  %v612 = add nsw i64 %v578, -1
  %v618 = lshr i64 %v612, %v607
  %t63 = icmp ult i64 %v618, %v40
  br i1 %t63, label %edge171, label %edge177

edge171:                                          ; preds = %edge154
  %v653 = add i64 %v618, %v39
  %t65 = inttoptr i64 %v653 to ptr
  %t66 = load i8, ptr %t65, align 1
  %v654 = zext i8 %t66 to i64
  br label %edge177

edge177:                                          ; preds = %edge154, %edge171
  %v2249 = phi i64 [ %v654, %edge171 ], [ 0, %edge154 ]
  %v674 = shl nuw nsw i64 %v607, 1
  %v678 = add nuw nsw i64 %v674, 515
  %v683 = add nuw nsw i64 %v678, %v2249
  %t68 = icmp ult i64 %v683, %v36
  br i1 %t68, label %edge193, label %edge206

edge193:                                          ; preds = %edge177
  %v730 = shl nuw nsw i64 %v683, 2
  %v732 = add i64 %v730, %v35
  %t70 = inttoptr i64 %v732 to ptr
  %t71 = load i32, ptr %t70, align 1
  br label %edge206

edge206:                                          ; preds = %edge177, %edge193
  %v2265 = phi i32 [ %t71, %edge193 ], [ 0, %edge177 ]
  %v814 = and i64 %v2233, 65535
  %v4618 = add nsw i64 %v814, -3
  %t74 = icmp ult i64 %v4618, 256
  br i1 %t74, label %edge229, label %edge440

edge229:                                          ; preds = %edge206
  %v847 = add nsw i64 %v814, -2
  %t78 = icmp ult i64 %v2223, %v847
  %v888 = sub nsw i64 %v847, %v2223
  br i1 %t78, label %edge234, label %block347

edge234:                                          ; preds = %edge229
  %v893 = trunc nuw i64 %v2224 to i32
  %t80 = icmp ugt i64 %v888, 15
  br i1 %t80, label %edge236, label %block752.preheader

block752.preheader:                               ; preds = %edge240, %edge234
  %v905.ph = phi i64 [ %v913, %edge240 ], [ %v2223, %edge234 ]
  %v906.ph = phi i32 [ %v961, %edge240 ], [ %v893, %edge234 ]
  %v907.ph = phi i32 [ %v962, %edge240 ], [ -1, %edge234 ]
  br label %block752

edge236:                                          ; preds = %edge234
  %v912 = and i64 %v888, -4
  %v913 = add i64 %v912, %v2223
  %t82 = insertelement <4 x i32> poison, i32 %v893, i64 0
  %v914 = shufflevector <4 x i32> %t82, <4 x i32> poison, <4 x i32> zeroinitializer
  %t86 = insertelement <4 x i32> poison, i32 %v2265, i64 0
  %v924 = shufflevector <4 x i32> %t86, <4 x i32> poison, <4 x i32> zeroinitializer
  br label %block755

edge239:                                          ; preds = %block755
  %v921 = trunc i64 %v917 to i32
  %t85 = insertelement <4 x i32> poison, i32 %v921, i64 0
  %v922 = shufflevector <4 x i32> %t85, <4 x i32> poison, <4 x i32> zeroinitializer
  %v923 = add <4 x i32> %v922, <i32 0, i32 1, i32 2, i32 3>
  %v925 = shl i64 %v917, 2
  %v926 = add i64 %v925, %v899
  %t87 = inttoptr i64 %v926 to ptr
  %v9274 = load <4 x i32>, ptr %t87, align 1
  %v929 = add <4 x i32> %v9274, %v924
  %2 = shl i64 %v917, 3
  %v932 = sub i64 %v931, %2
  %t88 = inttoptr i64 %v932 to ptr
  %v933 = load <16 x i8>, ptr %t88, align 1
  %t90 = getelementptr i8, ptr %t88, i64 16
  %v934 = load <16 x i8>, ptr %t90, align 1
  %v935 = shufflevector <16 x i8> %v933, <16 x i8> %v934, <16 x i32> <i32 24, i32 25, i32 26, i32 27, i32 16, i32 17, i32 18, i32 19, i32 8, i32 9, i32 10, i32 11, i32 0, i32 1, i32 2, i32 3>
  %v936 = bitcast <16 x i8> %v935 to <4 x i32>
  %v937 = add <4 x i32> %v929, %v936
  %t91.not = icmp ult <4 x i32> %v937, %v918
  %t92 = call <4 x i32> @llvm.umin.v4i32(<4 x i32> %v937, <4 x i32> %v918)
  %v940 = select <4 x i1> %t91.not, <4 x i32> %v923, <4 x i32> %v919
  %v941 = add i64 %v917, 4
  br label %block755

edge240:                                          ; preds = %block755
  %v944 = shufflevector <4 x i32> %v918, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 1>
  %t96 = call <4 x i32> @llvm.umin.v4i32(<4 x i32> %v918, <4 x i32> %v944)
  %v948 = shufflevector <4 x i32> %t96, <4 x i32> poison, <4 x i32> <i32 1, i32 0, i32 3, i32 2>
  %t97 = call <4 x i32> @llvm.umin.v4i32(<4 x i32> %t96, <4 x i32> %v948)
  %t98.not = icmp eq <4 x i32> %v918, %t97
  %v952 = select <4 x i1> %t98.not, <4 x i32> %v919, <4 x i32> splat (i32 -1)
  %v955 = shufflevector <4 x i32> %v952, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 1>
  %t103 = call <4 x i32> @llvm.umin.v4i32(<4 x i32> %v952, <4 x i32> %v955)
  %v959 = shufflevector <4 x i32> %t103, <4 x i32> poison, <4 x i32> <i32 1, i32 0, i32 3, i32 2>
  %t104 = call <4 x i32> @llvm.umin.v4i32(<4 x i32> %t103, <4 x i32> %v959)
  %v961 = extractelement <4 x i32> %t97, i64 0
  %v962 = extractelement <4 x i32> %t104, i64 0
  %t105 = icmp ult i64 %v913, %v847
  br i1 %t105, label %block752.preheader, label %block753

edge261:                                          ; preds = %block125
  %t56 = icmp ult i64 %v2222, %v32
  br i1 %t56, label %edge147, label %edge154

edge272:                                          ; preds = %block125
  %v1555 = shl i64 %v2225, 32
  %v1560 = or disjoint i64 %v1555, %v2224
  %v15691 = icmp ugt i64 %v2691, %v2181
  br i1 %v15691, label %edge272.edge323_crit_edge, label %edge284

edge272.edge323_crit_edge:                        ; preds = %edge272
  %.pre42 = add nuw i64 %v2181, 1
  br label %edge323

edge284:                                          ; preds = %edge272
  %t123 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2181, i64 1)
  %v1575 = extractvalue { i64, i1 } %t123, 0
  %t124 = extractvalue { i64, i1 } %t123, 1
  br i1 %t124, label %edge440, label %edge285

edge285:                                          ; preds = %edge284
  %t127.not = icmp ugt i64 %v1575, %v2691
  br i1 %t127.not, label %edge288, label %block927

edge288:                                          ; preds = %edge285
  %t129.not = icmp eq i64 %v2691, 0
  br i1 %t129.not, label %block930, label %edge289

edge289:                                          ; preds = %edge288
  %3 = add i64 %v2691, -16385
  %v15852 = icmp ult i64 %3, -15873
  br i1 %v15852, label %edge292, label %edge301

edge292:                                          ; preds = %edge289
  %t133 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2691, i64 %v2691)
  %v1586 = extractvalue { i64, i1 } %t133, 0
  %t134 = extractvalue { i64, i1 } %t133, 1
  br i1 %t134, label %edge440, label %block930

edge301:                                          ; preds = %edge289
  %t141 = mul nuw nsw i64 %v2691, 3
  %t145 = add nuw nsw i64 %t141, 1
  %v1603 = lshr i64 %t145, 1
  br label %block930

edge305:                                          ; preds = %block927
  %t151 = inttoptr i64 %v2694 to ptr
  %v1611 = load i64, ptr %t151, align 1
  %t152 = icmp eq i64 %v1611, 1
  %t153 = icmp ule i64 %v1580, %v2691
  %v16143 = and i1 %t153, %t152
  br i1 %v16143, label %edge312, label %block938

edge312:                                          ; preds = %block937.edge312_crit_edge, %edge305
  %t164.pre-phi = phi ptr [ %t164.phi.trans.insert, %block937.edge312_crit_edge ], [ %t151, %edge305 ]
  %v1624 = phi i64 [ %1, %block937.edge312_crit_edge ], [ 2, %edge305 ]
  %v161014 = phi i64 [ %v1623, %block937.edge312_crit_edge ], [ %v2694, %edge305 ]
  %v160712 = phi i64 [ %v1622, %block937.edge312_crit_edge ], [ %v2691, %edge305 ]
  %v160510 = phi i64 [ %v1621, %block937.edge312_crit_edge ], [ %v2198, %edge305 ]
  store i64 %v1624, ptr %t164.pre-phi, align 1
  br label %block942

edge315:                                          ; preds = %block942
  %t167 = inttoptr i64 %v2694 to ptr
  %v1626 = load i64, ptr %t167, align 1
  %v1627 = add i64 %v1626, -1
  store i64 %v1627, ptr %t167, align 1
  %t169.not = icmp eq i64 %v1627, 0
  br i1 %t169.not, label %edge318, label %edge323

edge318:                                          ; preds = %edge315
  %v1628 = call i64 @mech_compact_free(i64 %v0, i64 %v2694)
  %t170.not = icmp eq i64 %v1628, 0
  br i1 %t170.not, label %edge323, label %edge440

edge323:                                          ; preds = %edge272.edge323_crit_edge, %block942, %edge315, %edge318
  %v1632.pre-phi = phi i64 [ %.pre42, %edge272.edge323_crit_edge ], [ %v1575, %block942 ], [ %v1575, %edge315 ], [ %v1575, %edge318 ]
  %v3917 = phi i64 [ %v2198, %edge272.edge323_crit_edge ], [ %v160511, %block942 ], [ %v160511, %edge315 ], [ %v160511, %edge318 ]
  %v3938 = phi i64 [ %v2691, %edge272.edge323_crit_edge ], [ %v160713, %block942 ], [ %v160713, %edge315 ], [ %v160713, %edge318 ]
  %v3939 = phi i64 [ %v2694, %edge272.edge323_crit_edge ], [ %v161015, %block942 ], [ %v161015, %edge315 ], [ %v161015, %edge318 ]
  %v1631 = add i64 %v3917, %v902.pre-phi
  %t171 = inttoptr i64 %v1631 to ptr
  store i64 %v1560, ptr %t171, align 1
  br label %block6

edge440:                                          ; preds = %edge284, %edge292, %block938, %edge318, %edge206
  %.sink = phi i64 [ 3238, %edge206 ], [ 3396, %edge318 ], [ 3396, %block938 ], [ 3396, %edge292 ], [ 3396, %edge284 ]
  %v48 = phi i64 [ 9, %edge206 ], [ 1, %edge292 ], [ %v1620, %block938 ], [ %v1628, %edge318 ], [ 1, %edge284 ]
  %t76 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t76, align 1
  %t208 = insertvalue { i64, i64 } poison, i64 %v48, 0
  %t209 = insertvalue { i64, i64 } %t208, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare <4 x i32> @llvm.umin.v4i32(<4 x i32>, <4 x i32>) #2

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umin.i32(i32, i32) #2

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_3009_no_growth(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 152
  %v25 = load i64, ptr %t0, align 1
  %t4 = getelementptr i8, ptr %t0, i64 8
  %v26 = load i64, ptr %t4, align 1
  %t6 = getelementptr i8, ptr %t0, i64 16
  %t8 = getelementptr i8, ptr %t0, i64 24
  %v28 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 32
  %t12 = getelementptr i8, ptr %t0, i64 40
  %v30 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 48
  %t16 = getelementptr i8, ptr %t0, i64 56
  %t22 = getelementptr i8, ptr %t0, i64 80
  %t24 = getelementptr i8, ptr %t0, i64 88
  %t30 = getelementptr i8, ptr %t0, i64 112
  %t32 = getelementptr i8, ptr %t0, i64 120
  %t38 = getelementptr i8, ptr %t0, i64 144
  br label %block6.outer

block6.outer.loopexit:                            ; preds = %edge406, %edge400, %block1062
  br label %block6.outer

block6.outer:                                     ; preds = %block6.outer.loopexit, %block0
  %v2079.ph = phi i64 [ %v25, %block0 ], [ %v4289, %block6.outer.loopexit ]
  %v2080.ph = phi i64 [ %v26, %block0 ], [ %v4290, %block6.outer.loopexit ]
  %v2140.ph = phi i64 [ %v28, %block0 ], [ %v4291, %block6.outer.loopexit ]
  %v2152.ph = phi i64 [ %v30, %block0 ], [ %v4291, %block6.outer.loopexit ]
  %v4288.ph = load i64, ptr %t38, align 1
  %v4287.ph = load i64, ptr %t32, align 1
  %v4286.ph = load i64, ptr %t30, align 1
  %v4285.ph = load i64, ptr %t24, align 1
  %v4284.ph = load i64, ptr %t22, align 1
  %v4283.ph = load i64, ptr %t16, align 1
  %v4282.ph = load i64, ptr %t14, align 1
  %v4281.ph = load i64, ptr %t10, align 1
  %v4280.ph = load i64, ptr %t6, align 1
  %v4279.ph = load i64, ptr %t1, align 1
  %invariant.op18 = add i64 %v4282.ph, -4
  %invariant.op19 = add i64 %v4280.ph, -8
  %invariant.op = add i64 %v4280.ph, -24
  %v899 = add i64 %v4284.ph, 12
  br label %block6

block6:                                           ; preds = %block6.backedge, %block6.outer
  %v2079 = phi i64 [ %v2079.ph, %block6.outer ], [ %v96, %block6.backedge ]
  %v2080 = phi i64 [ %v2080.ph, %block6.outer ], [ %v450, %block6.backedge ]
  %v2140 = phi i64 [ %v2140.ph, %block6.outer ], [ %v1591, %block6.backedge ]
  %v2152 = phi i64 [ %v2152.ph, %block6.outer ], [ %v1591, %block6.backedge ]
  %t39.not = icmp eq i64 %v2079, 0
  br i1 %t39.not, label %edge13, label %edge15

block125:                                         ; preds = %block347, %edge100
  %v2181 = phi i64 [ %v450, %edge100 ], [ %v1444, %block347 ]
  %v2182 = phi i64 [ 0, %edge100 ], [ %v4273, %block347 ]
  %v2183 = phi i64 [ %v401, %edge100 ], [ %v4271, %block347 ]
  %v2184 = phi i64 [ %v435, %edge100 ], [ %v4272, %block347 ]
  %t123.not = icmp eq i64 %v2181, %v101
  br i1 %t123.not, label %edge271, label %edge260

block757:                                         ; preds = %edge242, %edge236
  %v940 = phi i64 [ %v2182, %edge236 ], [ %v964, %edge242 ]
  %v941 = phi <4 x i32> [ %v937, %edge236 ], [ %t99, %edge242 ]
  %v942 = phi <4 x i32> [ splat (i32 -1), %edge236 ], [ %v963, %edge242 ]
  %t90 = icmp ult i64 %v940, %v936
  br i1 %t90, label %edge242, label %edge243

block752:                                         ; preds = %block752, %edge248
  %v906 = phi i64 [ %v4262, %edge248 ], [ %v1019, %block752 ]
  %v907 = phi i32 [ %v4263, %edge248 ], [ %t119, %block752 ]
  %v908 = phi i64 [ %v4264, %edge248 ], [ %v1018, %block752 ]
  %v909 = phi i64 [ %v4265, %edge248 ], [ %v1001, %block752 ]
  %v1001 = add i64 %v909, -8
  %v1002 = shl i64 %v906, 2
  %v1003 = add i64 %v1002, %v899
  %t116 = inttoptr i64 %v1003 to ptr
  %v1004 = load i32, ptr %t116, align 1
  %v1005 = add i32 %v1004, %v2224
  %t117 = inttoptr i64 %v1001 to ptr
  %v1006 = load i32, ptr %t117, align 1
  %v1007 = add i32 %v1005, %v1006
  %t118 = icmp ult i32 %v1007, %v907
  %t119 = tail call i32 @llvm.umin.i32(i32 %v1007, i32 %v907)
  %v1012 = add i64 %v906, 3
  %v1017 = or i64 %v1012, %v1015.pre-phi
  %v1018 = select i1 %t118, i64 %v1017, i64 %v908
  %v1019 = add nuw i64 %v906, 1
  %t121 = icmp ult i64 %v1019, %v847
  br i1 %t121, label %block752, label %block753

block753:                                         ; preds = %block752, %edge243, %edge237
  %v910 = phi i32 [ %v984, %edge243 ], [ %v931, %edge237 ], [ %t119, %block752 ]
  %v911 = phi i64 [ %v996, %edge243 ], [ %v932, %edge237 ], [ %v1018, %block752 ]
  %v1021 = zext i32 %v910 to i64
  br label %block347

block347:                                         ; preds = %edge229, %block753
  %v4271 = phi i64 [ %v1021, %block753 ], [ %v2183, %edge229 ]
  %v4272 = phi i64 [ %v911, %block753 ], [ %v2184, %edge229 ]
  %v4273 = phi i64 [ %v847, %block753 ], [ %v2182, %edge229 ]
  %v1444 = add i64 %v2181, 1
  br label %block125

common.ret:                                       ; preds = %edge225, %edge13
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge13 ], [ { i64 9, i64 3238 }, %edge225 ]
  ret { i64, i64 } %common.ret.op

block1062:                                        ; preds = %block1062.preheader, %edge408
  %v4289 = phi i64 [ %v4321, %edge408 ], [ %v96, %block1062.preheader ]
  %v4290 = phi i64 [ %v4322, %edge408 ], [ %v450, %block1062.preheader ]
  %v4291 = phi i64 [ %v4370, %edge408 ], [ %v1591, %block1062.preheader ]
  %v4292 = phi i64 [ %v4367, %edge408 ], [ %v1583, %block1062.preheader ]
  %t166.not = icmp eq i64 %v4289, 0
  br i1 %t166.not, label %block6.outer.loopexit, label %edge400

edge13:                                           ; preds = %block6
  %t139 = inttoptr i64 %v2 to ptr
  %t140 = getelementptr i8, ptr %t139, i64 8
  store i64 %v4280.ph, ptr %t140, align 1
  %t142 = getelementptr i8, ptr %t139, i64 16
  store i64 %v2140, ptr %t142, align 1
  %t144 = getelementptr i8, ptr %t139, i64 24
  store i64 %v4281.ph, ptr %t144, align 1
  %t146 = getelementptr i8, ptr %t139, i64 32
  store i64 %v2152, ptr %t146, align 1
  %t148 = getelementptr i8, ptr %t139, i64 40
  store i64 %v4288.ph, ptr %t148, align 1
  %t150 = getelementptr i8, ptr %t139, i64 48
  store i64 %v4280.ph, ptr %t150, align 1
  %t152 = getelementptr i8, ptr %t139, i64 56
  store i64 %v2140, ptr %t152, align 1
  %t154 = getelementptr i8, ptr %t139, i64 64
  store i64 %v4288.ph, ptr %t154, align 1
  %t156 = getelementptr i8, ptr %t139, i64 72
  store i64 %v4281.ph, ptr %t156, align 1
  %t158 = getelementptr i8, ptr %t139, i64 80
  store i64 %v2152, ptr %t158, align 1
  %t160 = getelementptr i8, ptr %t139, i64 88
  store i64 %v4279.ph, ptr %t160, align 1
  br label %common.ret

edge15:                                           ; preds = %block6
  %v96 = add i64 %v2079, -1
  %v101 = add i64 %v2080, -1
  %t41 = icmp ult i64 %v101, %v4283.ph
  br i1 %t41, label %edge30, label %edge37

edge30:                                           ; preds = %edge15
  %v145 = shl i64 %v2080, 2
  %v150.reass = add i64 %v145, %invariant.op18
  %t43 = inttoptr i64 %v150.reass to ptr
  %t44 = load i32, ptr %t43, align 1
  %v151 = zext i32 %t44 to i64
  br label %edge37

edge37:                                           ; preds = %edge30, %edge15
  %v2097 = phi i64 [ %v151, %edge30 ], [ 0, %edge15 ]
  %v172 = and i64 %v2097, 65535
  %v240 = lshr i64 %v2097, 16
  %v245 = add nuw nsw i64 %v240, 259
  %t48 = icmp ult i64 %v245, %v4285.ph
  br i1 %t48, label %edge73, label %edge80

edge73:                                           ; preds = %edge37
  %v292 = shl nuw nsw i64 %v245, 2
  %v294 = add i64 %v292, %v4284.ph
  %t50 = inttoptr i64 %v294 to ptr
  %t51 = load i32, ptr %t50, align 1
  br label %edge80

edge80:                                           ; preds = %edge73, %edge37
  %v2136 = phi i32 [ %t51, %edge73 ], [ 0, %edge37 ]
  %t52.not = icmp eq i64 %v2140, 0
  br i1 %t52.not, label %edge100, label %edge94

edge94:                                           ; preds = %edge80
  %v361 = shl i64 %v2140, 3
  %v366.reass = add i64 %v361, %invariant.op19
  %t54 = inttoptr i64 %v366.reass to ptr
  %v367 = load i64, ptr %t54, align 1
  %0 = trunc i64 %v367 to i32
  br label %edge100

edge100:                                          ; preds = %edge80, %edge94
  %v902.pre-phi = phi i64 [ %v361, %edge94 ], [ 0, %edge80 ]
  %v2162 = phi i32 [ %0, %edge94 ], [ 0, %edge80 ]
  %v4373 = add i32 %v2162, %v2136
  %v401 = zext i32 %v4373 to i64
  %v430 = shl nuw nsw i64 %v240, 9
  %v435 = or disjoint i64 %v430, 1
  %v450 = sub i64 %v101, %v172
  %v904.reass = add i64 %v902.pre-phi, %invariant.op
  %v954 = add i64 %v904.reass, -24
  %v999.neg = add i64 %v904.reass, 8
  br label %block125

edge147:                                          ; preds = %edge260
  %v553 = shl i64 %v2181, 2
  %v555 = add i64 %v553, %v4282.ph
  %t58 = inttoptr i64 %v555 to ptr
  %t59 = load i32, ptr %t58, align 1
  %v556 = zext i32 %t59 to i64
  br label %edge154

edge154:                                          ; preds = %edge147, %edge260
  %v2192 = phi i64 [ %v556, %edge147 ], [ 0, %edge260 ]
  %v578 = lshr i64 %v2192, 16
  %v4376 = sub nsw i64 256, %v578
  %v596 = lshr i64 %v4376, 29
  %v607 = and i64 %v596, 7
  %v612 = add nsw i64 %v578, -1
  %v618 = lshr i64 %v612, %v607
  %t63 = icmp ult i64 %v618, %v4287.ph
  br i1 %t63, label %edge171, label %edge177

edge171:                                          ; preds = %edge154
  %v653 = add i64 %v618, %v4286.ph
  %t65 = inttoptr i64 %v653 to ptr
  %t66 = load i8, ptr %t65, align 1
  %v654 = zext i8 %t66 to i64
  br label %edge177

edge177:                                          ; preds = %edge154, %edge171
  %v2208 = phi i64 [ %v654, %edge171 ], [ 0, %edge154 ]
  %v674 = shl nuw nsw i64 %v607, 1
  %v678 = add nuw nsw i64 %v674, 515
  %v683 = add nuw nsw i64 %v678, %v2208
  %t68 = icmp ult i64 %v683, %v4285.ph
  br i1 %t68, label %edge193, label %edge206

edge193:                                          ; preds = %edge177
  %v730 = shl nuw nsw i64 %v683, 2
  %v732 = add i64 %v730, %v4284.ph
  %t70 = inttoptr i64 %v732 to ptr
  %t71 = load i32, ptr %t70, align 1
  br label %edge206

edge206:                                          ; preds = %edge177, %edge193
  %v2224 = phi i32 [ %t71, %edge193 ], [ 0, %edge177 ]
  %v814 = and i64 %v2192, 65535
  %v4269 = add nsw i64 %v814, -3
  %t74 = icmp ult i64 %v4269, 256
  br i1 %t74, label %edge229, label %edge225

edge225:                                          ; preds = %edge206
  %t76 = inttoptr i64 %v2 to ptr
  store i64 3238, ptr %t76, align 1
  br label %common.ret

edge229:                                          ; preds = %edge206
  %v847 = add nsw i64 %v814, -2
  %t78 = icmp ult i64 %v2182, %v847
  %v888 = sub nsw i64 %v847, %v2182
  br i1 %t78, label %edge234, label %block347

edge234:                                          ; preds = %edge229
  %v893 = trunc nuw i64 %v2183 to i32
  %t80 = icmp ugt i64 %v888, 15
  br i1 %t80, label %edge236, label %edge237

edge236:                                          ; preds = %edge234
  %v935 = and i64 %v888, -4
  %v936 = add i64 %v935, %v2182
  %t89 = insertelement <4 x i32> poison, i32 %v893, i64 0
  %v937 = shufflevector <4 x i32> %t89, <4 x i32> poison, <4 x i32> zeroinitializer
  %t93 = insertelement <4 x i32> poison, i32 %v2224, i64 0
  %v947 = shufflevector <4 x i32> %t93, <4 x i32> poison, <4 x i32> zeroinitializer
  br label %block757

edge237:                                          ; preds = %edge234
  %v914 = shl nuw nsw i64 %v2182, 3
  %v915 = sub i64 %v904.reass, %v914
  %v916 = shl nuw nsw i64 %v2182, 2
  %v917 = add i64 %v916, %v899
  %t82 = inttoptr i64 %v917 to ptr
  %v918 = load i32, ptr %t82, align 1
  %v919 = add i32 %v918, %v2224
  %t83 = inttoptr i64 %v915 to ptr
  %v920 = load i32, ptr %t83, align 1
  %v921 = add i32 %v919, %v920
  %t84 = icmp ult i32 %v921, %v893
  %v925 = add nuw nsw i64 %v2182, 3
  %v928 = shl nuw nsw i64 %v578, 9
  %v930 = or i64 %v928, %v925
  %v931 = tail call i32 @llvm.umin.i32(i32 %v921, i32 %v893)
  %v932 = select i1 %t84, i64 %v930, i64 %v2184
  %t87 = icmp eq i64 %v888, 1
  br i1 %t87, label %block753, label %edge239

edge239:                                          ; preds = %edge237
  %v934 = add nuw nsw i64 %v2182, 1
  br label %edge248

edge242:                                          ; preds = %block757
  %v944 = trunc i64 %v940 to i32
  %t92 = insertelement <4 x i32> poison, i32 %v944, i64 0
  %v945 = shufflevector <4 x i32> %t92, <4 x i32> poison, <4 x i32> zeroinitializer
  %v946 = add <4 x i32> %v945, <i32 0, i32 1, i32 2, i32 3>
  %v948 = shl i64 %v940, 2
  %v949 = add i64 %v948, %v899
  %t94 = inttoptr i64 %v949 to ptr
  %v9504 = load <4 x i32>, ptr %t94, align 1
  %v952 = add <4 x i32> %v9504, %v947
  %1 = shl i64 %v940, 3
  %v955 = sub i64 %v954, %1
  %t95 = inttoptr i64 %v955 to ptr
  %v956 = load <16 x i8>, ptr %t95, align 1
  %t97 = getelementptr i8, ptr %t95, i64 16
  %v957 = load <16 x i8>, ptr %t97, align 1
  %v958 = shufflevector <16 x i8> %v956, <16 x i8> %v957, <16 x i32> <i32 24, i32 25, i32 26, i32 27, i32 16, i32 17, i32 18, i32 19, i32 8, i32 9, i32 10, i32 11, i32 0, i32 1, i32 2, i32 3>
  %v959 = bitcast <16 x i8> %v958 to <4 x i32>
  %v960 = add <4 x i32> %v952, %v959
  %t98.not = icmp ult <4 x i32> %v960, %v941
  %t99 = tail call <4 x i32> @llvm.umin.v4i32(<4 x i32> %v960, <4 x i32> %v941)
  %v963 = select <4 x i1> %t98.not, <4 x i32> %v946, <4 x i32> %v942
  %v964 = add i64 %v940, 4
  br label %block757

edge243:                                          ; preds = %block757
  %v967 = shufflevector <4 x i32> %v941, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 1>
  %t103 = tail call <4 x i32> @llvm.umin.v4i32(<4 x i32> %v941, <4 x i32> %v967)
  %v971 = shufflevector <4 x i32> %t103, <4 x i32> poison, <4 x i32> <i32 1, i32 0, i32 3, i32 2>
  %t104 = tail call <4 x i32> @llvm.umin.v4i32(<4 x i32> %t103, <4 x i32> %v971)
  %t105.not = icmp eq <4 x i32> %v941, %t104
  %v975 = select <4 x i1> %t105.not, <4 x i32> %v942, <4 x i32> splat (i32 -1)
  %v978 = shufflevector <4 x i32> %v975, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 1>
  %t110 = tail call <4 x i32> @llvm.umin.v4i32(<4 x i32> %v975, <4 x i32> %v978)
  %v982 = shufflevector <4 x i32> %t110, <4 x i32> poison, <4 x i32> <i32 1, i32 0, i32 3, i32 2>
  %t111 = tail call <4 x i32> @llvm.umin.v4i32(<4 x i32> %t110, <4 x i32> %v982)
  %v984 = extractelement <4 x i32> %t104, i64 0
  %v985 = extractelement <4 x i32> %t111, i64 0
  %t112.not = icmp eq i32 %v985, -1
  %v990 = add i32 %v985, 3
  %v992 = trunc nuw nsw i64 %v578 to i32
  %v993 = shl nuw nsw i32 %v992, 9
  %v994 = or i32 %v990, %v993
  %v995 = zext i32 %v994 to i64
  %v996 = select i1 %t112.not, i64 %v2184, i64 %v995
  %t114 = icmp ult i64 %v936, %v847
  br i1 %t114, label %edge245, label %block753

edge245:                                          ; preds = %edge243
  %2 = shl i64 %v936, 3
  %v1000 = sub i64 %v999.neg, %2
  %.pre = shl nuw nsw i64 %v578, 9
  br label %edge248

edge248:                                          ; preds = %edge239, %edge245
  %v1015.pre-phi = phi i64 [ %v928, %edge239 ], [ %.pre, %edge245 ]
  %v4262 = phi i64 [ %v934, %edge239 ], [ %v936, %edge245 ]
  %v4263 = phi i32 [ %v931, %edge239 ], [ %v984, %edge245 ]
  %v4264 = phi i64 [ %v932, %edge239 ], [ %v996, %edge245 ]
  %v4265 = phi i64 [ %v915, %edge239 ], [ %v1000, %edge245 ]
  br label %block752

edge260:                                          ; preds = %block125
  %t56 = icmp ult i64 %v2181, %v4283.ph
  br i1 %t56, label %edge147, label %edge154

edge271:                                          ; preds = %block125
  %v1578 = shl i64 %v2184, 32
  %v1583 = or disjoint i64 %v1578, %v2183
  %v1590 = add i64 %v4280.ph, %v902.pre-phi
  %t126 = inttoptr i64 %v1590 to ptr
  store i64 %v1583, ptr %t126, align 1
  %v1591 = add i64 %v2140, 1
  %t127 = icmp eq i64 %v172, 0
  br i1 %t127, label %edge283, label %block6.backedge

edge283:                                          ; preds = %edge271
  %v4294 = load i64, ptr %t6, align 1
  %v4296 = load i64, ptr %t16, align 1
  %v4297 = load i64, ptr %t22, align 1
  %v4298 = load i64, ptr %t24, align 1
  %t179 = icmp ule i64 %v96, %v450
  %t180 = icmp ule i64 %v450, %v4296
  %v4302 = sub i64 0, %v2079
  %v43041 = and i1 %t179, %t180
  %v43052 = icmp ult i64 %v2140, %v4302
  %v43063 = and i1 %v43052, %v43041
  br i1 %v43063, label %block1062.preheader, label %block6.backedge

block6.backedge:                                  ; preds = %edge283, %edge271
  br label %block6

block1062.preheader:                              ; preds = %edge283
  %v4295 = load i64, ptr %t14, align 1
  %v4327 = add i64 %v4295, -4
  br label %block1062

edge400:                                          ; preds = %block1062
  %v4321 = add i64 %v4289, -1
  %v4322 = add i64 %v4290, -1
  %v4326 = shl i64 %v4290, 2
  %v4328 = add i64 %v4327, %v4326
  %t205 = inttoptr i64 %v4328 to ptr
  %t206 = load i32, ptr %t205, align 1
  %v4329 = zext i32 %t206 to i64
  %v4330 = and i64 %v4329, 65535
  %t207 = icmp eq i64 %v4330, 0
  br i1 %t207, label %edge406, label %block6.outer.loopexit

edge406:                                          ; preds = %edge400
  %v4336 = lshr exact i64 %v4329, 16
  %v4337 = add nuw nsw i64 %v4336, 259
  %t211 = icmp ult i64 %v4337, %v4298
  br i1 %t211, label %edge408, label %block6.outer.loopexit

edge408:                                          ; preds = %edge406
  %v4343 = shl nuw nsw i64 %v4337, 2
  %v4344 = add i64 %v4343, %v4297
  %t214 = inttoptr i64 %v4344 to ptr
  %t215 = load i32, ptr %t214, align 1
  %v4384 = trunc i64 %v4292 to i32
  %v4385 = add i32 %t215, %v4384
  %v4354 = zext i32 %v4385 to i64
  %v4359 = shl nuw nsw i64 %v4329, 25
  %v4366 = or disjoint i64 %v4359, %v4354
  %v4367 = or disjoint i64 %v4366, 4294967296
  %v4368 = shl i64 %v4291, 3
  %v4369 = add i64 %v4368, %v4294
  %t220 = inttoptr i64 %v4369 to ptr
  store i64 %v4367, ptr %t220, align 1
  %v4370 = add i64 %v4291, 1
  br label %block1062
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_3009_no_growth_composed(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 152
  %v25 = load i64, ptr %t0, align 1
  %t4 = getelementptr i8, ptr %t0, i64 8
  %v26 = load i64, ptr %t4, align 1
  %t6 = getelementptr i8, ptr %t0, i64 16
  %t8 = getelementptr i8, ptr %t0, i64 24
  %v28 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 32
  %t12 = getelementptr i8, ptr %t0, i64 40
  %v30 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 48
  %t16 = getelementptr i8, ptr %t0, i64 56
  %t22 = getelementptr i8, ptr %t0, i64 80
  %v35 = load i64, ptr %t22, align 1
  %t24 = getelementptr i8, ptr %t0, i64 88
  %t38 = getelementptr i8, ptr %t0, i64 144
  %v4396 = add i64 %v35, 2060
  br label %block6.outer

block6.outer:                                     ; preds = %block1065, %block0
  %v2090.ph = phi i64 [ %v4297, %block1065 ], [ %v25, %block0 ]
  %v2091.in.ph = phi i64 [ %v4298, %block1065 ], [ %v26, %block0 ]
  %v2151.ph = phi i64 [ %v4299, %block1065 ], [ %v28, %block0 ]
  %v2163.ph = phi i64 [ %v4299, %block1065 ], [ %v30, %block0 ]
  %v4292.ph = phi i64 [ %v4320, %block1065 ], [ %v35, %block0 ]
  %v4296.ph = load i64, ptr %t38, align 1
  %v4293.ph = load i64, ptr %t24, align 1
  %v4291.ph = load i64, ptr %t16, align 1
  %v4290.ph = load i64, ptr %t14, align 1
  %v4289.ph = load i64, ptr %t10, align 1
  %v4288.ph = load i64, ptr %t6, align 1
  %v4287.ph = load i64, ptr %t1, align 1
  %invariant.op27 = add i64 %v4288.ph, -8
  %invariant.op = add i64 %v4288.ph, -24
  %v899 = add i64 %v4292.ph, 12
  br label %block6

block6:                                           ; preds = %block6.backedge, %block6.outer
  %v2090 = phi i64 [ %v2090.ph, %block6.outer ], [ %v96, %block6.backedge ]
  %v2091.in = phi i64 [ %v2091.in.ph, %block6.outer ], [ %v450, %block6.backedge ]
  %v2151 = phi i64 [ %v2151.ph, %block6.outer ], [ %v2163.be, %block6.backedge ]
  %v2163 = phi i64 [ %v2163.ph, %block6.outer ], [ %v2163.be, %block6.backedge ]
  %v2091 = add i64 %v2091.in, -1
  %t41.not = icmp eq i64 %v2090, 0
  br i1 %t41.not, label %edge13, label %edge15

block125:                                         ; preds = %block125.preheader, %block347
  %v2192 = phi i64 [ %v1455, %block347 ], [ %v450, %block125.preheader ]
  %v2193 = phi i64 [ %v4281, %block347 ], [ 2, %block125.preheader ]
  %v2194 = phi i64 [ %v4279, %block347 ], [ %v401, %block125.preheader ]
  %v2195 = phi i64 [ %v4280, %block347 ], [ %v4412, %block125.preheader ]
  %v4418 = add nsw i64 %v2193, -2
  %t62 = icmp ult i64 %v2192, %v4291.ph
  br i1 %t62, label %edge148, label %edge155

block757:                                         ; preds = %edge213, %edge207
  %v949 = phi i64 [ %v4418, %edge207 ], [ %v973, %edge213 ]
  %v950 = phi <4 x i32> [ %v946, %edge207 ], [ %t97, %edge213 ]
  %v951 = phi <4 x i32> [ splat (i32 -1), %edge207 ], [ %v972, %edge213 ]
  %t88 = icmp ult i64 %v949, %v945
  br i1 %t88, label %edge213, label %edge214

block752:                                         ; preds = %block752, %edge219
  %v906 = phi i64 [ %v4269, %edge219 ], [ %v1029, %block752 ]
  %v907 = phi i32 [ %v4270, %edge219 ], [ %v1021, %block752 ]
  %v908 = phi i64 [ %v4271, %edge219 ], [ %v1028, %block752 ]
  %v909 = phi i64 [ %v4272, %edge219 ], [ %v1012, %block752 ]
  %v910 = phi i64 [ %v4273, %edge219 ], [ %v1030, %block752 ]
  %v1012 = add i64 %v909, -8
  %v1014 = shl i64 %v906, 2
  %v1015 = add i64 %v1014, %v4292.ph
  %t114 = inttoptr i64 %v1015 to ptr
  %v1016 = load i32, ptr %t114, align 1
  %v1017 = add i32 %v1016, %t70
  %t115 = inttoptr i64 %v1012 to ptr
  %v1018 = load i32, ptr %t115, align 1
  %v1019 = add i32 %v1017, %v1018
  %t116 = icmp ult i32 %v1019, %v907
  %v1021 = tail call i32 @llvm.umin.i32(i32 %v1019, i32 %v907)
  %v906.masked = and i64 %v906, 4294967295
  %v1027 = or i64 %v906.masked, %v1024.pre-phi
  %v1028 = select i1 %t116, i64 %v1027, i64 %v908
  %v1029 = add i64 %v906, 1
  %v1030 = add i64 %v910, -1
  %t119.not = icmp eq i64 %v1030, 0
  br i1 %t119.not, label %block753, label %block752

block753:                                         ; preds = %block752, %edge214, %edge208
  %v911 = phi i32 [ %v993, %edge214 ], [ %v936, %edge208 ], [ %v1021, %block752 ]
  %v912 = phi i64 [ %v1005, %edge214 ], [ %v937, %edge208 ], [ %v1028, %block752 ]
  %v1032 = zext i32 %v911 to i64
  br label %block347

block347:                                         ; preds = %edge200, %block753
  %v4279 = phi i64 [ %v1032, %block753 ], [ %v2194, %edge200 ]
  %v4280 = phi i64 [ %v912, %block753 ], [ %v2195, %edge200 ]
  %v4281 = phi i64 [ %v814, %block753 ], [ %v2193, %edge200 ]
  %v1455 = add i64 %v2192, 1
  %t60 = icmp eq i64 %v1455, %v2091
  br i1 %t60, label %edge241, label %block125

common.ret:                                       ; preds = %edge196, %edge13
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge13 ], [ { i64 9, i64 3238 }, %edge196 ]
  ret { i64, i64 } %common.ret.op

block1062:                                        ; preds = %block1062.preheader, %edge378
  %v4297 = phi i64 [ %v4329, %edge378 ], [ %v96, %block1062.preheader ]
  %v4298 = phi i64 [ %v4330, %edge378 ], [ %v450, %block1062.preheader ]
  %v4299 = phi i64 [ %v4378, %edge378 ], [ %v160213, %block1062.preheader ]
  %v4300 = phi i64 [ %v4375, %edge378 ], [ %v15949, %block1062.preheader ]
  %t163.not = icmp eq i64 %v4297, 0
  br i1 %t163.not, label %block1065, label %edge370

block1065:                                        ; preds = %edge376, %edge370, %block1062
  %v4320 = load i64, ptr %t22, align 1
  br label %block6.outer

edge13:                                           ; preds = %block6
  %t136 = inttoptr i64 %v2 to ptr
  %t137 = getelementptr i8, ptr %t136, i64 8
  store i64 %v4288.ph, ptr %t137, align 1
  %t139 = getelementptr i8, ptr %t136, i64 16
  store i64 %v2151, ptr %t139, align 1
  %t141 = getelementptr i8, ptr %t136, i64 24
  store i64 %v4289.ph, ptr %t141, align 1
  %t143 = getelementptr i8, ptr %t136, i64 32
  store i64 %v2163, ptr %t143, align 1
  %t145 = getelementptr i8, ptr %t136, i64 40
  store i64 %v4296.ph, ptr %t145, align 1
  %t147 = getelementptr i8, ptr %t136, i64 48
  store i64 %v4288.ph, ptr %t147, align 1
  %t149 = getelementptr i8, ptr %t136, i64 56
  store i64 %v2151, ptr %t149, align 1
  %t151 = getelementptr i8, ptr %t136, i64 64
  store i64 %v4296.ph, ptr %t151, align 1
  %t153 = getelementptr i8, ptr %t136, i64 72
  store i64 %v4289.ph, ptr %t153, align 1
  %t155 = getelementptr i8, ptr %t136, i64 80
  store i64 %v2163, ptr %t155, align 1
  %t157 = getelementptr i8, ptr %t136, i64 88
  store i64 %v4287.ph, ptr %t157, align 1
  br label %common.ret

edge15:                                           ; preds = %block6
  %v96 = add i64 %v2090, -1
  %t43 = icmp ult i64 %v2091, %v4291.ph
  br i1 %t43, label %edge30, label %edge37

edge30:                                           ; preds = %edge15
  %v148 = shl i64 %v2091, 2
  %v150 = add i64 %v4290.ph, %v148
  %t45 = inttoptr i64 %v150 to ptr
  %t46 = load i32, ptr %t45, align 1
  %v151 = zext i32 %t46 to i64
  br label %edge37

edge37:                                           ; preds = %edge30, %edge15
  %v2108 = phi i64 [ %v151, %edge30 ], [ 0, %edge15 ]
  %v172 = and i64 %v2108, 65535
  %v240 = lshr i64 %v2108, 16
  %v245 = add nuw nsw i64 %v240, 259
  %t49 = icmp ult i64 %v245, %v4293.ph
  br i1 %t49, label %edge72, label %edge79

edge72:                                           ; preds = %edge37
  %v292 = shl nuw nsw i64 %v245, 2
  %v294 = add i64 %v292, %v4292.ph
  %t51 = inttoptr i64 %v294 to ptr
  %t52 = load i32, ptr %t51, align 1
  br label %edge79

edge79:                                           ; preds = %edge72, %edge37
  %v2147 = phi i32 [ %t52, %edge72 ], [ 0, %edge37 ]
  %t54.not = icmp eq i64 %v2151, 0
  br i1 %t54.not, label %edge99, label %edge93

edge93:                                           ; preds = %edge79
  %v4406 = shl i64 %v2151, 3
  %v4408.reass = add i64 %v4406, %invariant.op27
  %t55 = inttoptr i64 %v4408.reass to ptr
  %t56 = load i32, ptr %t55, align 1
  br label %edge99

edge99:                                           ; preds = %edge79, %edge93
  %v2173 = phi i32 [ %t56, %edge93 ], [ 0, %edge79 ]
  %v4381 = add i32 %v2173, %v2147
  %v401 = zext i32 %v4381 to i64
  %v4410 = shl nuw nsw i64 %v240, 9
  %v4412 = or disjoint i64 %v4410, 1
  %v450 = sub i64 %v2091, %v172
  %t58 = icmp eq i64 %v172, 0
  br i1 %t58, label %edge241.thread, label %block125.preheader

block125.preheader:                               ; preds = %edge99
  %v902 = shl i64 %v2151, 3
  %v904.reass = add i64 %v902, %invariant.op
  %v963 = add i64 %v904.reass, -24
  br label %block125

edge241.thread:                                   ; preds = %edge99
  %v15898 = shl nuw nsw i64 %v4412, 32
  %v15949 = or disjoint i64 %v15898, %v401
  %v160010 = shl i64 %v2151, 3
  %v160111 = add i64 %v4288.ph, %v160010
  %t12312 = inttoptr i64 %v160111 to ptr
  store i64 %v15949, ptr %t12312, align 1
  %v160213 = add i64 %v2151, 1
  %v4302 = load i64, ptr %t6, align 1
  %v4304 = load i64, ptr %t16, align 1
  %v4305 = load i64, ptr %t22, align 1
  %v4306 = load i64, ptr %t24, align 1
  %t175 = icmp ule i64 %v96, %v450
  %t176 = icmp ule i64 %v450, %v4304
  %v4310 = sub i64 0, %v2090
  %v43123 = and i1 %t175, %t176
  %v43134 = icmp ult i64 %v2151, %v4310
  %v43145 = and i1 %v43134, %v43123
  br i1 %v43145, label %block1062.preheader, label %block6.backedge

block1062.preheader:                              ; preds = %edge241.thread
  %v4303 = load i64, ptr %t14, align 1
  %v4335 = add i64 %v4303, -4
  br label %block1062

edge148:                                          ; preds = %block125
  %v553 = shl i64 %v2192, 2
  %v555 = add i64 %v553, %v4290.ph
  %t64 = inttoptr i64 %v555 to ptr
  %t65 = load i32, ptr %t64, align 1
  %v556 = zext i32 %t65 to i64
  br label %edge155

edge155:                                          ; preds = %edge148, %block125
  %v2203 = phi i64 [ %v556, %edge148 ], [ 0, %block125 ]
  %v578 = lshr i64 %v2203, 16
  %v4398 = add i64 %v578, ptrtoint (ptr @mech_compact_cost_codes_0 to i64)
  %t67 = inttoptr i64 %v4398 to ptr
  %t68 = load i8, ptr %t67, align 1
  %v4399 = zext i8 %t68 to i64
  %v4400 = shl nuw nsw i64 %v4399, 2
  %v4401 = add i64 %v4396, %v4400
  %t69 = inttoptr i64 %v4401 to ptr
  %t70 = load i32, ptr %t69, align 1
  %v814 = and i64 %v2203, 65535
  %v4277 = add nsw i64 %v814, -3
  %t72 = icmp ult i64 %v4277, 256
  br i1 %t72, label %edge200, label %edge196

edge196:                                          ; preds = %edge155
  %t74 = inttoptr i64 %v2 to ptr
  store i64 3238, ptr %t74, align 1
  br label %common.ret

edge200:                                          ; preds = %edge155
  %v847 = add nsw i64 %v814, -2
  %t76 = icmp samesign ult i64 %v2193, %v814
  %v888 = sub nsw i64 %v814, %v2193
  br i1 %t76, label %edge205, label %block347

edge205:                                          ; preds = %edge200
  %v893 = trunc nuw i64 %v2194 to i32
  %t78 = icmp ugt i64 %v888, 15
  br i1 %t78, label %edge207, label %edge208

edge207:                                          ; preds = %edge205
  %v944 = and i64 %v888, -4
  %v945 = add i64 %v944, %v4418
  %t87 = insertelement <4 x i32> poison, i32 %v893, i64 0
  %v946 = shufflevector <4 x i32> %t87, <4 x i32> poison, <4 x i32> zeroinitializer
  %t91 = insertelement <4 x i32> poison, i32 %t70, i64 0
  %v956 = shufflevector <4 x i32> %t91, <4 x i32> poison, <4 x i32> zeroinitializer
  br label %block757

edge208:                                          ; preds = %edge205
  %v915 = add nuw nsw i64 %v2193, 1
  %v917 = shl nuw nsw i64 %v915, 2
  %v918 = add i64 %v917, %v4292.ph
  %v921 = sub i64 %v2151, %v915
  %v922 = shl i64 %v921, 3
  %v923 = add i64 %v922, %v4288.ph
  %t80 = inttoptr i64 %v918 to ptr
  %v924 = load i32, ptr %t80, align 1
  %v925 = add i32 %v924, %t70
  %t81 = inttoptr i64 %v923 to ptr
  %v926 = load i64, ptr %t81, align 1
  %v927 = trunc i64 %v926 to i32
  %v928 = add i32 %v925, %v927
  %t82 = icmp ult i32 %v928, %v893
  %v932 = shl nuw nsw i64 %v578, 9
  %v934 = or i64 %v932, %v915
  %v936 = tail call i32 @llvm.umin.i32(i32 %v928, i32 %v893)
  %v937 = select i1 %t82, i64 %v934, i64 %v2195
  %t85 = icmp eq i64 %v888, 1
  br i1 %t85, label %block753, label %edge210

edge210:                                          ; preds = %edge208
  %v940 = shl nuw nsw i64 %v4418, 3
  %v942 = add nuw nsw i64 %v2193, 2
  %v943 = add nsw i64 %v888, -1
  br label %edge219

edge213:                                          ; preds = %block757
  %v953 = trunc i64 %v949 to i32
  %t90 = insertelement <4 x i32> poison, i32 %v953, i64 0
  %v954 = shufflevector <4 x i32> %t90, <4 x i32> poison, <4 x i32> zeroinitializer
  %v955 = add <4 x i32> %v954, <i32 0, i32 1, i32 2, i32 3>
  %v957 = shl i64 %v949, 2
  %v958 = add i64 %v899, %v957
  %t92 = inttoptr i64 %v958 to ptr
  %v9591 = load <4 x i32>, ptr %t92, align 1
  %v961 = add <4 x i32> %v9591, %v956
  %0 = shl i64 %v949, 3
  %v964 = sub i64 %v963, %0
  %t93 = inttoptr i64 %v964 to ptr
  %v965 = load <16 x i8>, ptr %t93, align 1
  %t95 = getelementptr i8, ptr %t93, i64 16
  %v966 = load <16 x i8>, ptr %t95, align 1
  %v967 = shufflevector <16 x i8> %v965, <16 x i8> %v966, <16 x i32> <i32 24, i32 25, i32 26, i32 27, i32 16, i32 17, i32 18, i32 19, i32 8, i32 9, i32 10, i32 11, i32 0, i32 1, i32 2, i32 3>
  %v968 = bitcast <16 x i8> %v967 to <4 x i32>
  %v969 = add <4 x i32> %v961, %v968
  %t96.not = icmp ult <4 x i32> %v969, %v950
  %t97 = tail call <4 x i32> @llvm.umin.v4i32(<4 x i32> %v969, <4 x i32> %v950)
  %v972 = select <4 x i1> %t96.not, <4 x i32> %v955, <4 x i32> %v951
  %v973 = add i64 %v949, 4
  br label %block757

edge214:                                          ; preds = %block757
  %v976 = shufflevector <4 x i32> %v950, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 1>
  %t101 = tail call <4 x i32> @llvm.umin.v4i32(<4 x i32> %v950, <4 x i32> %v976)
  %v980 = shufflevector <4 x i32> %t101, <4 x i32> poison, <4 x i32> <i32 1, i32 0, i32 3, i32 2>
  %t102 = tail call <4 x i32> @llvm.umin.v4i32(<4 x i32> %t101, <4 x i32> %v980)
  %t103.not = icmp eq <4 x i32> %v950, %t102
  %v984 = select <4 x i1> %t103.not, <4 x i32> %v951, <4 x i32> splat (i32 -1)
  %v987 = shufflevector <4 x i32> %v984, <4 x i32> poison, <4 x i32> <i32 2, i32 3, i32 0, i32 1>
  %t108 = tail call <4 x i32> @llvm.umin.v4i32(<4 x i32> %v984, <4 x i32> %v987)
  %v991 = shufflevector <4 x i32> %t108, <4 x i32> poison, <4 x i32> <i32 1, i32 0, i32 3, i32 2>
  %t109 = tail call <4 x i32> @llvm.umin.v4i32(<4 x i32> %t108, <4 x i32> %v991)
  %v993 = extractelement <4 x i32> %t102, i64 0
  %v994 = extractelement <4 x i32> %t109, i64 0
  %t110.not = icmp eq i32 %v994, -1
  %v999 = add i32 %v994, 3
  %v1001 = trunc nuw nsw i64 %v578 to i32
  %v1002 = shl nuw nsw i32 %v1001, 9
  %v1003 = or i32 %v999, %v1002
  %v1004 = zext i32 %v1003 to i64
  %v1005 = select i1 %t110.not, i64 %v2195, i64 %v1004
  %t112 = icmp ult i64 %v945, %v847
  br i1 %t112, label %edge216, label %block753

edge216:                                          ; preds = %edge214
  %v1007 = shl i64 %v945, 3
  %v1008 = add i64 %v1007, -8
  %v1010 = add i64 %v945, 3
  %v1011 = sub nuw i64 %v847, %v945
  %.pre = shl nuw nsw i64 %v578, 9
  br label %edge219

edge219:                                          ; preds = %edge210, %edge216
  %v1024.pre-phi = phi i64 [ %v932, %edge210 ], [ %.pre, %edge216 ]
  %v4269 = phi i64 [ %v942, %edge210 ], [ %v1010, %edge216 ]
  %v4270 = phi i32 [ %v936, %edge210 ], [ %v993, %edge216 ]
  %v4271 = phi i64 [ %v937, %edge210 ], [ %v1005, %edge216 ]
  %v940.pn = phi i64 [ %v940, %edge210 ], [ %v1008, %edge216 ]
  %v4273 = phi i64 [ %v943, %edge210 ], [ %v1011, %edge216 ]
  %v4272 = sub i64 %v904.reass, %v940.pn
  br label %block752

edge241:                                          ; preds = %block347
  %v1589 = shl nuw i64 %v4280, 32
  %v1594 = or disjoint i64 %v1589, %v4279
  %v1601 = add i64 %v4288.ph, %v902
  %t123 = inttoptr i64 %v1601 to ptr
  store i64 %v1594, ptr %t123, align 1
  %v1602 = add i64 %v2151, 1
  br label %block6.backedge

block6.backedge:                                  ; preds = %edge241, %edge241.thread
  %v2163.be = phi i64 [ %v1602, %edge241 ], [ %v160213, %edge241.thread ]
  br label %block6

edge370:                                          ; preds = %block1062
  %v4329 = add i64 %v4297, -1
  %v4330 = add i64 %v4298, -1
  %v4334 = shl i64 %v4298, 2
  %v4336 = add i64 %v4335, %v4334
  %t200 = inttoptr i64 %v4336 to ptr
  %t201 = load i32, ptr %t200, align 1
  %v4337 = zext i32 %t201 to i64
  %v4338 = and i64 %v4337, 65535
  %t202 = icmp eq i64 %v4338, 0
  br i1 %t202, label %edge376, label %block1065

edge376:                                          ; preds = %edge370
  %v4344 = lshr exact i64 %v4337, 16
  %v4345 = add nuw nsw i64 %v4344, 259
  %t205 = icmp ult i64 %v4345, %v4306
  br i1 %t205, label %edge378, label %block1065

edge378:                                          ; preds = %edge376
  %v4351 = shl nuw nsw i64 %v4345, 2
  %v4352 = add i64 %v4351, %v4305
  %t207 = inttoptr i64 %v4352 to ptr
  %t208 = load i32, ptr %t207, align 1
  %v4392 = trunc i64 %v4300 to i32
  %v4393 = add i32 %t208, %v4392
  %v4362 = zext i32 %v4393 to i64
  %v4416 = shl nuw nsw i64 %v4337, 25
  %v4374 = or disjoint i64 %v4416, %v4362
  %v4375 = or disjoint i64 %v4374, 4294967296
  %v4376 = shl i64 %v4299, 3
  %v4377 = add i64 %v4376, %v4302
  %t211 = inttoptr i64 %v4377 to ptr
  store i64 %v4375, ptr %t211, align 1
  %v4378 = add i64 %v4299, 1
  br label %block1062
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_3447(i64 %v1307, i64 %v1308) unnamed_addr #6 align 128 {
block409:
  %t0 = inttoptr i64 %v1307 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 24
  %v1309 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 32
  %v1310 = load i64, ptr %t3, align 1
  %t5 = getelementptr i8, ptr %t0, i64 56
  %v1311 = load i64, ptr %t5, align 1
  %t7 = getelementptr i8, ptr %t0, i64 64
  %v1312 = load i64, ptr %t7, align 1
  %t8 = icmp ult i64 %v1310, 2305843009213693952
  %v1314 = shl i64 %v1310, 3
  %v1315 = add i64 %v1314, %v1309
  %t9 = icmp uge i64 %v1315, %v1309
  %t10 = icmp ugt i64 %v1312, 255
  %v1318 = add i64 %v1311, 1024
  %t11 = icmp ult i64 %v1311, -1024
  %t12 = icmp ule i64 %v1315, %v1311
  %t13 = icmp ule i64 %v1318, %v1309
  %v13221 = or i1 %t12, %t13
  %v13232 = and i1 %t8, %t9
  %v13243 = and i1 %t11, %t10
  %v13254 = and i1 %v13232, %v13243
  %v13265 = and i1 %v13221, %v13254
  br i1 %v13265, label %edge0, label %edge1

block6:                                           ; preds = %edge50, %edge1
  %v780 = phi i64 [ %v24, %edge1 ], [ %v707, %edge50 ]
  %t56.not = icmp eq i64 %v780, %v25
  br i1 %t56.not, label %edge16, label %edge18

common.ret:                                       ; preds = %edge0, %edge197, %edge16
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge16 ], [ %t121, %edge197 ], [ %t122, %edge0 ]
  ret { i64, i64 } %common.ret.op

edge0:                                            ; preds = %block409
  %t122 = tail call fastcc { i64, i64 } @mech_compact_numeric_3447_grouped(i64 %v1307, i64 %v1308)
  br label %common.ret

edge1:                                            ; preds = %block409
  %t16 = getelementptr i8, ptr %t0, i64 152
  %v5 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 160
  %v6 = load i64, ptr %t18, align 1
  %v23 = load i64, ptr %t0, align 1
  %t21 = getelementptr i8, ptr %t0, i64 8
  %v24 = load i64, ptr %t21, align 1
  %t23 = getelementptr i8, ptr %t0, i64 16
  %v25 = load i64, ptr %t23, align 1
  %t37 = getelementptr i8, ptr %t0, i64 72
  %v32 = load i64, ptr %t37, align 1
  %t39 = getelementptr i8, ptr %t0, i64 80
  %v33 = load i64, ptr %t39, align 1
  %t41 = getelementptr i8, ptr %t0, i64 88
  %v34 = load i64, ptr %t41, align 1
  %t43 = getelementptr i8, ptr %t0, i64 96
  %v35 = load i64, ptr %t43, align 1
  %t49 = getelementptr i8, ptr %t0, i64 120
  %v38 = load i64, ptr %t49, align 1
  %t51 = getelementptr i8, ptr %t0, i64 128
  %v39 = load i64, ptr %t51, align 1
  br label %block6

edge16:                                           ; preds = %block6
  %t97 = inttoptr i64 %v1308 to ptr
  %t98 = getelementptr i8, ptr %t97, i64 8
  store i64 %v1311, ptr %t98, align 1
  %t100 = getelementptr i8, ptr %t97, i64 16
  store i64 %v1312, ptr %t100, align 1
  %t102 = getelementptr i8, ptr %t97, i64 24
  store i64 %v32, ptr %t102, align 1
  %t104 = getelementptr i8, ptr %t97, i64 32
  store i64 %v33, ptr %t104, align 1
  %t106 = getelementptr i8, ptr %t97, i64 40
  store i64 %v1311, ptr %t106, align 1
  %t108 = getelementptr i8, ptr %t97, i64 48
  store i64 %v1312, ptr %t108, align 1
  %t110 = getelementptr i8, ptr %t97, i64 56
  store i64 %v6, ptr %t110, align 1
  %t112 = getelementptr i8, ptr %t97, i64 64
  store i64 %v32, ptr %t112, align 1
  %t114 = getelementptr i8, ptr %t97, i64 72
  store i64 %v33, ptr %t114, align 1
  %t116 = getelementptr i8, ptr %t97, i64 80
  store i64 %v5, ptr %t116, align 1
  br label %common.ret

edge18:                                           ; preds = %block6
  %v98 = sub i64 %v23, %v780
  %t58 = icmp ult i64 %v98, %v1310
  br i1 %t58, label %edge38, label %edge90

edge38:                                           ; preds = %edge18
  %v145 = shl i64 %v98, 3
  %v147 = add i64 %v145, %v1309
  %t60 = inttoptr i64 %v147 to ptr
  %v148 = load i64, ptr %t60, align 1
  %0 = lshr i64 %v148, 32
  %v186 = and i64 %0, 511
  %v195 = lshr i64 %v148, 41
  %t63 = icmp eq i64 %v186, 1
  br i1 %t63, label %edge52, label %edge90

edge50:                                           ; preds = %edge154, %edge52
  %v217.sink = phi i64 [ %v217, %edge52 ], [ %v600, %edge154 ]
  %v18611 = phi i64 [ 1, %edge52 ], [ %v18610, %edge154 ]
  %v264 = shl nuw nsw i64 %v217.sink, 2
  %v266 = add i64 %v264, %v1311
  %t67 = inttoptr i64 %v266 to ptr
  %t68 = load i32, ptr %t67, align 1
  %1 = add i32 %t68, 1
  store i32 %1, ptr %t67, align 1
  %v707 = add i64 %v18611, %v780
  br label %block6

edge52:                                           ; preds = %edge38
  %v217 = and i64 %v195, 255
  %t65 = icmp ult i64 %v217, %v1312
  br i1 %t65, label %edge50, label %edge197

edge90:                                           ; preds = %edge18, %edge38
  %v19512 = phi i64 [ %v195, %edge38 ], [ 0, %edge18 ]
  %v18610 = phi i64 [ %v186, %edge38 ], [ 0, %edge18 ]
  %t72 = icmp ult i64 %v18610, %v35
  br i1 %t72, label %edge96, label %edge103

edge96:                                           ; preds = %edge90
  %v373 = add i64 %v18610, %v34
  %t74 = inttoptr i64 %v373 to ptr
  %t75 = load i8, ptr %t74, align 1
  %v374 = zext i8 %t75 to i64
  %2 = add nuw nsw i64 %v374, 257
  br label %edge103

edge103:                                          ; preds = %edge96, %edge90
  %v825 = phi i64 [ %2, %edge96 ], [ 257, %edge90 ]
  %t76 = icmp ult i64 %v825, %v1312
  br i1 %t76, label %edge116, label %edge197

edge116:                                          ; preds = %edge103
  %v442 = shl nuw nsw i64 %v825, 2
  %v444 = add i64 %v442, %v1311
  %t78 = inttoptr i64 %v444 to ptr
  %t79 = load i32, ptr %t78, align 1
  %3 = add i32 %t79, 1
  store i32 %3, ptr %t78, align 1
  %v513 = sub nsw i64 256, %v19512
  %v518 = lshr i64 %v513, 29
  %v524 = and i64 %v518, 7
  %v529 = add nsw i64 %v19512, -1
  %v535 = lshr i64 %v529, %v524
  %t85 = icmp ult i64 %v535, %v39
  br i1 %t85, label %edge148, label %edge154

edge148:                                          ; preds = %edge116
  %v570 = add i64 %v535, %v38
  %t87 = inttoptr i64 %v570 to ptr
  %t88 = load i8, ptr %t87, align 1
  %v571 = zext i8 %t88 to i64
  br label %edge154

edge154:                                          ; preds = %edge116, %edge148
  %v855 = phi i64 [ %v571, %edge148 ], [ 0, %edge116 ]
  %v591 = shl nuw nsw i64 %v524, 1
  %v595 = or disjoint i64 %v591, 288
  %v600 = add nuw nsw i64 %v595, %v855
  %t90 = icmp ult i64 %v600, %v1312
  br i1 %t90, label %edge50, label %edge197

edge197:                                          ; preds = %edge154, %edge103, %edge52
  %.sink = phi i64 [ 3575, %edge103 ], [ 3524, %edge52 ], [ 3625, %edge154 ]
  %t71 = inttoptr i64 %v1308 to ptr
  store i64 %.sink, ptr %t71, align 1
  %t121 = insertvalue { i64, i64 } { i64 103169, i64 undef }, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_3447_grouped(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 152
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 160
  %v6 = load i64, ptr %t3, align 1
  %v23 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v24 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v25 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v26 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 32
  %v27 = load i64, ptr %t12, align 1
  %t18 = getelementptr i8, ptr %t0, i64 56
  %v30 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 64
  %v31 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 72
  %v32 = load i64, ptr %t22, align 1
  %t24 = getelementptr i8, ptr %t0, i64 80
  %v33 = load i64, ptr %t24, align 1
  %t26 = getelementptr i8, ptr %t0, i64 88
  %v34 = load i64, ptr %t26, align 1
  %t28 = getelementptr i8, ptr %t0, i64 96
  %v35 = load i64, ptr %t28, align 1
  %t34 = getelementptr i8, ptr %t0, i64 120
  %v38 = load i64, ptr %t34, align 1
  %t36 = getelementptr i8, ptr %t0, i64 128
  %v39 = load i64, ptr %t36, align 1
  br label %block6

block6:                                           ; preds = %block6.backedge, %block0
  %v780 = phi i64 [ %v24, %block0 ], [ %v780.be, %block6.backedge ]
  %t41.not = icmp eq i64 %v780, %v25
  br i1 %t41.not, label %edge14, label %edge16

common.ret:                                       ; preds = %edge195, %edge14
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge14 ], [ %t106, %edge195 ]
  ret { i64, i64 } %common.ret.op

edge14:                                           ; preds = %block6
  %t82 = inttoptr i64 %v2 to ptr
  %t83 = getelementptr i8, ptr %t82, i64 8
  store i64 %v30, ptr %t83, align 1
  %t85 = getelementptr i8, ptr %t82, i64 16
  store i64 %v31, ptr %t85, align 1
  %t87 = getelementptr i8, ptr %t82, i64 24
  store i64 %v32, ptr %t87, align 1
  %t89 = getelementptr i8, ptr %t82, i64 32
  store i64 %v33, ptr %t89, align 1
  %t91 = getelementptr i8, ptr %t82, i64 40
  store i64 %v30, ptr %t91, align 1
  %t93 = getelementptr i8, ptr %t82, i64 48
  store i64 %v31, ptr %t93, align 1
  %t95 = getelementptr i8, ptr %t82, i64 56
  store i64 %v6, ptr %t95, align 1
  %t97 = getelementptr i8, ptr %t82, i64 64
  store i64 %v32, ptr %t97, align 1
  %t99 = getelementptr i8, ptr %t82, i64 72
  store i64 %v33, ptr %t99, align 1
  %t101 = getelementptr i8, ptr %t82, i64 80
  store i64 %v5, ptr %t101, align 1
  br label %common.ret

edge16:                                           ; preds = %block6
  %v98 = sub i64 %v23, %v780
  %t43 = icmp ult i64 %v98, %v27
  br i1 %t43, label %edge36, label %edge88

edge36:                                           ; preds = %edge16
  %v145 = shl i64 %v98, 3
  %v147 = add i64 %v145, %v26
  %t45 = inttoptr i64 %v147 to ptr
  %v148 = load i64, ptr %t45, align 1
  %0 = lshr i64 %v148, 32
  %v186 = and i64 %0, 511
  %v195 = lshr i64 %v148, 41
  %t48 = icmp eq i64 %v186, 1
  br i1 %t48, label %edge46, label %edge88

edge46:                                           ; preds = %edge36
  %t107 = icmp ult i64 %v780, %v25
  %v1307 = sub i64 %v25, %v780
  %t108 = icmp ugt i64 %v1307, 3
  %t109 = icmp ugt i64 %v98, 2
  %v13101 = and i1 %t107, %t108
  %v13112 = and i1 %t109, %v13101
  br i1 %v13112, label %edge196, label %edge50

edge48:                                           ; preds = %edge152, %edge50
  %v217.sink = phi i64 [ %v217, %edge50 ], [ %v600, %edge152 ]
  %v1868 = phi i64 [ 1, %edge50 ], [ %v1867, %edge152 ]
  %v264 = shl nuw nsw i64 %v217.sink, 2
  %v266 = add i64 %v264, %v30
  %t52 = inttoptr i64 %v266 to ptr
  %t53 = load i32, ptr %t52, align 1
  %1 = add i32 %t53, 1
  store i32 %1, ptr %t52, align 1
  %v707 = add i64 %v1868, %v780
  br label %block6.backedge

block6.backedge:                                  ; preds = %edge48, %edge198
  %v780.be = phi i64 [ %v707, %edge48 ], [ %v1353, %edge198 ]
  br label %block6

edge50:                                           ; preds = %edge196, %edge46
  %v217 = and i64 %v195, 255
  %t50 = icmp ult i64 %v217, %v31
  br i1 %t50, label %edge48, label %edge195

edge88:                                           ; preds = %edge16, %edge36
  %v1959 = phi i64 [ %v195, %edge36 ], [ 0, %edge16 ]
  %v1867 = phi i64 [ %v186, %edge36 ], [ 0, %edge16 ]
  %t57 = icmp ult i64 %v1867, %v35
  br i1 %t57, label %edge94, label %edge101

edge94:                                           ; preds = %edge88
  %v373 = add i64 %v1867, %v34
  %t59 = inttoptr i64 %v373 to ptr
  %t60 = load i8, ptr %t59, align 1
  %v374 = zext i8 %t60 to i64
  %2 = add nuw nsw i64 %v374, 257
  br label %edge101

edge101:                                          ; preds = %edge94, %edge88
  %v825 = phi i64 [ %2, %edge94 ], [ 257, %edge88 ]
  %t61 = icmp ult i64 %v825, %v31
  br i1 %t61, label %edge114, label %edge195

edge114:                                          ; preds = %edge101
  %v442 = shl nuw nsw i64 %v825, 2
  %v444 = add i64 %v442, %v30
  %t63 = inttoptr i64 %v444 to ptr
  %t64 = load i32, ptr %t63, align 1
  %3 = add i32 %t64, 1
  store i32 %3, ptr %t63, align 1
  %v513 = sub nsw i64 256, %v1959
  %v518 = lshr i64 %v513, 29
  %v524 = and i64 %v518, 7
  %v529 = add nsw i64 %v1959, -1
  %v535 = lshr i64 %v529, %v524
  %t70 = icmp ult i64 %v535, %v39
  br i1 %t70, label %edge146, label %edge152

edge146:                                          ; preds = %edge114
  %v570 = add i64 %v535, %v38
  %t72 = inttoptr i64 %v570 to ptr
  %t73 = load i8, ptr %t72, align 1
  %v571 = zext i8 %t73 to i64
  br label %edge152

edge152:                                          ; preds = %edge114, %edge146
  %v855 = phi i64 [ %v571, %edge146 ], [ 0, %edge114 ]
  %v591 = shl nuw nsw i64 %v524, 1
  %v595 = or disjoint i64 %v591, 288
  %v600 = add nuw nsw i64 %v595, %v855
  %t75 = icmp ult i64 %v600, %v31
  br i1 %t75, label %edge48, label %edge195

edge195:                                          ; preds = %edge152, %edge101, %edge50
  %.sink = phi i64 [ 3575, %edge101 ], [ 3524, %edge50 ], [ 3625, %edge152 ]
  %t56 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t56, align 1
  %t106 = insertvalue { i64, i64 } { i64 103169, i64 undef }, i64 %.sink, 1
  br label %common.ret

edge196:                                          ; preds = %edge46
  %t112 = getelementptr i8, ptr %t45, i64 -4
  %v1315 = load i32, ptr %t112, align 1
  %t114 = getelementptr i8, ptr %t45, i64 -12
  %v1316 = load i32, ptr %t114, align 1
  %t116 = getelementptr i8, ptr %t45, i64 -20
  %v1317 = load i32, ptr %t116, align 1
  %v1318 = xor i32 %v1315, 1
  %v1319 = xor i32 %v1316, 1
  %v1320 = xor i32 %v1317, 1
  %v1321 = or i32 %v1319, %v1318
  %v1322 = or i32 %v1321, %v1320
  %v1323 = and i32 %v1322, 511
  %t117 = icmp eq i32 %v1323, 0
  br i1 %t117, label %edge198, label %edge50

edge198:                                          ; preds = %edge196
  %v1325 = lshr i64 %v148, 39
  %v1326 = and i64 %v1325, 1020
  %v1329 = add i64 %v1326, %v30
  %t119 = inttoptr i64 %v1329 to ptr
  %v1330 = load i32, ptr %t119, align 1
  %v1331 = add i32 %v1330, 1
  store i32 %v1331, ptr %t119, align 1
  %4 = lshr i32 %v1315, 7
  %5 = and i32 %4, 1020
  %v1335 = zext nneg i32 %5 to i64
  %v1336 = add i64 %v30, %v1335
  %t121 = inttoptr i64 %v1336 to ptr
  %v1337 = load i32, ptr %t121, align 1
  %v1338 = add i32 %v1337, 1
  store i32 %v1338, ptr %t121, align 1
  %6 = lshr i32 %v1316, 7
  %7 = and i32 %6, 1020
  %v1342 = zext nneg i32 %7 to i64
  %v1343 = add i64 %v30, %v1342
  %t123 = inttoptr i64 %v1343 to ptr
  %v1344 = load i32, ptr %t123, align 1
  %v1345 = add i32 %v1344, 1
  store i32 %v1345, ptr %t123, align 1
  %8 = lshr i32 %v1317, 7
  %9 = and i32 %8, 1020
  %v1349 = zext nneg i32 %9 to i64
  %v1350 = add i64 %v30, %v1349
  %t125 = inttoptr i64 %v1350 to ptr
  %v1351 = load i32, ptr %t125, align 1
  %v1352 = add i32 %v1351, 1
  store i32 %v1352, ptr %t125, align 1
  %v1353 = add i64 %v780, 4
  br label %block6.backedge
}

; Function Attrs: noinline nounwind memory(readwrite, inaccessiblemem: write, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_362(i64 %v1, i64 %v2) unnamed_addr #5 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 88
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 96
  %v6 = load i64, ptr %t3, align 1
  %v13 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v14 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v15 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v16 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 32
  %v17 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 40
  %v18 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 48
  %v19 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 56
  %v20 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 64
  %v21 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 72
  %v22 = load i64, ptr %t22, align 1
  %t24 = getelementptr i8, ptr %t0, i64 80
  %v23 = load i64, ptr %t24, align 1
  %t32 = icmp eq i64 %v22, 1
  %t33 = icmp eq i64 %v23, 1
  %v1131 = or i1 %t32, %t33
  %t35 = icmp eq i64 %v22, 0
  %umax = tail call i64 @llvm.umax.i64(i64 %v15, i64 %v14)
  br label %block6

block6:                                           ; preds = %edge57, %block0
  %v231 = phi i64 [ %v15, %block0 ], [ %v188, %edge57 ]
  %exitcond.not = icmp eq i64 %v231, %umax
  br i1 %exitcond.not, label %edge64, label %edge16

trap36:                                           ; preds = %edge31
  tail call void @llvm.trap()
  unreachable

ok36:                                             ; preds = %edge31
  %v115 = udiv i64 %v74, %v22
  %v116 = mul i64 %v115, %v22
  %v117 = sub nuw i64 %v74, %v116
  %v118 = mul i64 %v117, %v23
  %v119 = add i64 %v118, %v115
  br label %block80

block80:                                          ; preds = %edge27, %ok36
  %v114 = phi i64 [ %v119, %ok36 ], [ %v74, %edge27 ]
  %v122 = add i64 %v114, %v20
  %t37 = inttoptr i64 %v122 to ptr
  %t38 = load i8, ptr %t37, align 1
  %v123 = zext i8 %t38 to i64
  br label %edge40

common.ret:                                       ; preds = %edge66, %edge64
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge64 ], [ %t72, %edge66 ]
  ret { i64, i64 } %common.ret.op

edge16:                                           ; preds = %block6
  %v74 = add i64 %v231, %v13
  %t27.not = icmp ult i64 %v74, %v13
  br i1 %t27.not, label %edge66, label %edge21

edge21:                                           ; preds = %edge16
  %t30 = icmp ult i64 %v74, %v21
  br i1 %t30, label %edge27, label %edge40

edge27:                                           ; preds = %edge21
  br i1 %v1131, label %block80, label %edge31

edge31:                                           ; preds = %edge27
  br i1 %t35, label %trap36, label %ok36

edge40:                                           ; preds = %block80, %edge21
  %v241 = phi i64 [ %v123, %block80 ], [ 0, %edge21 ]
  %t42 = icmp ult i64 %v241, %v17
  br i1 %t42, label %edge57, label %edge66

edge57:                                           ; preds = %edge40
  %v155 = add i64 %v241, %v16
  %t44 = inttoptr i64 %v155 to ptr
  store i8 1, ptr %t44, align 1
  %v188 = add i64 %v231, 1
  br label %block6

edge64:                                           ; preds = %block6
  %t48 = inttoptr i64 %v2 to ptr
  %t49 = getelementptr i8, ptr %t48, i64 8
  store i64 %v16, ptr %t49, align 1
  %t51 = getelementptr i8, ptr %t48, i64 16
  store i64 %v17, ptr %t51, align 1
  %t53 = getelementptr i8, ptr %t48, i64 24
  store i64 %v18, ptr %t53, align 1
  %t55 = getelementptr i8, ptr %t48, i64 32
  store i64 %v19, ptr %t55, align 1
  %t57 = getelementptr i8, ptr %t48, i64 40
  store i64 %v16, ptr %t57, align 1
  %t59 = getelementptr i8, ptr %t48, i64 48
  store i64 %v17, ptr %t59, align 1
  %t61 = getelementptr i8, ptr %t48, i64 56
  store i64 %v6, ptr %t61, align 1
  %t63 = getelementptr i8, ptr %t48, i64 64
  store i64 %v18, ptr %t63, align 1
  %t65 = getelementptr i8, ptr %t48, i64 72
  store i64 %v19, ptr %t65, align 1
  %t67 = getelementptr i8, ptr %t48, i64 80
  store i64 %v5, ptr %t67, align 1
  br label %common.ret

edge66:                                           ; preds = %edge40, %edge16
  %.sink = phi i64 [ 378, %edge16 ], [ 393, %edge40 ]
  %v26 = phi i64 [ 1, %edge16 ], [ 103169, %edge40 ]
  %t29 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t29, align 1
  %t71 = insertvalue { i64, i64 } poison, i64 %v26, 0
  %t72 = insertvalue { i64, i64 } %t71, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_362_vector(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 88
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 96
  %v6 = load i64, ptr %t3, align 1
  %v13 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v14 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v15 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v16 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 32
  %v17 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 40
  %v18 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 48
  %v19 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 56
  %v20 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 64
  %v21 = load i64, ptr %t20, align 1
  %umax = tail call i64 @llvm.umax.i64(i64 %v15, i64 %v14)
  br label %block6

block6:                                           ; preds = %edge54, %block0
  %v222 = phi i64 [ %v15, %block0 ], [ %v179, %edge54 ]
  %exitcond.not = icmp eq i64 %v222, %umax
  br i1 %exitcond.not, label %edge61, label %edge16

common.ret:                                       ; preds = %edge63, %edge61
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge61 ], [ %t67, %edge63 ]
  ret { i64, i64 } %common.ret.op

edge16:                                           ; preds = %block6
  %v74 = add i64 %v222, %v13
  %t27.not = icmp ult i64 %v74, %v13
  br i1 %t27.not, label %edge63, label %edge21

edge21:                                           ; preds = %edge16
  %t30 = icmp ult i64 %v74, %v21
  br i1 %t30, label %edge27, label %edge37

edge27:                                           ; preds = %edge21
  %v113 = add i64 %v74, %v20
  %t32 = inttoptr i64 %v113 to ptr
  %t33 = load i8, ptr %t32, align 1
  %v114 = zext i8 %t33 to i64
  br label %edge37

edge37:                                           ; preds = %edge27, %edge21
  %v233 = phi i64 [ %v114, %edge27 ], [ 0, %edge21 ]
  %t37 = icmp ult i64 %v233, %v17
  br i1 %t37, label %edge54, label %edge63

edge54:                                           ; preds = %edge37
  %v146 = add i64 %v233, %v16
  %t39 = inttoptr i64 %v146 to ptr
  store i8 1, ptr %t39, align 1
  %v179 = add i64 %v222, 1
  br label %block6

edge61:                                           ; preds = %block6
  %t43 = inttoptr i64 %v2 to ptr
  %t44 = getelementptr i8, ptr %t43, i64 8
  store i64 %v16, ptr %t44, align 1
  %t46 = getelementptr i8, ptr %t43, i64 16
  store i64 %v17, ptr %t46, align 1
  %t48 = getelementptr i8, ptr %t43, i64 24
  store i64 %v18, ptr %t48, align 1
  %t50 = getelementptr i8, ptr %t43, i64 32
  store i64 %v19, ptr %t50, align 1
  %t52 = getelementptr i8, ptr %t43, i64 40
  store i64 %v16, ptr %t52, align 1
  %t54 = getelementptr i8, ptr %t43, i64 48
  store i64 %v17, ptr %t54, align 1
  %t56 = getelementptr i8, ptr %t43, i64 56
  store i64 %v6, ptr %t56, align 1
  %t58 = getelementptr i8, ptr %t43, i64 64
  store i64 %v18, ptr %t58, align 1
  %t60 = getelementptr i8, ptr %t43, i64 72
  store i64 %v19, ptr %t60, align 1
  %t62 = getelementptr i8, ptr %t43, i64 80
  store i64 %v5, ptr %t62, align 1
  br label %common.ret

edge63:                                           ; preds = %edge37, %edge16
  %.sink = phi i64 [ 378, %edge16 ], [ 393, %edge37 ]
  %v26 = phi i64 [ 1, %edge16 ], [ 103169, %edge37 ]
  %t29 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t29, align 1
  %t66 = insertvalue { i64, i64 } poison, i64 %v26, 0
  %t67 = insertvalue { i64, i64 } %t66, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_4090(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 64
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 72
  %v6 = load i64, ptr %t3, align 1
  %v13 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v14 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v15 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v16 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 32
  %v17 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 40
  %v18 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 48
  %v19 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 56
  %v20 = load i64, ptr %t18, align 1
  %v226 = add i64 %v17, -4
  br label %block6

block6:                                           ; preds = %edge124, %block0
  %v548 = phi i64 [ %v15, %block0 ], [ %spec.select, %edge124 ]
  %t23 = icmp sgt i64 %v548, -1
  br i1 %t23, label %edge11, label %edge165

block134:                                         ; preds = %edge105, %edge90, %edge11
  %v494 = add i64 %v548, %v13
  %t73.not = icmp ult i64 %v494, %v13
  br i1 %t73.not, label %edge165, label %edge148

common.ret:                                       ; preds = %edge165, %edge155
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge155 ], [ %t107, %edge165 ]
  ret { i64, i64 } %common.ret.op

edge11:                                           ; preds = %block6
  %v58 = shl nuw i64 %v548, 1
  %t26 = icmp ugt i64 %v58, %v14
  br i1 %t26, label %block134, label %edge20

edge20:                                           ; preds = %edge11
  %t28 = icmp ult i64 %v58, %v14
  %v99 = add i64 %v58, %v13
  %t29.not = icmp ult i64 %v99, %v13
  br i1 %t29.not, label %edge165, label %edge32

edge32:                                           ; preds = %edge20
  %t32 = icmp ult i64 %v99, %v18
  br i1 %t32, label %edge38, label %edge47

edge38:                                           ; preds = %edge32
  %v148 = shl i64 %v99, 2
  %v150 = add i64 %v148, %v17
  %t34 = inttoptr i64 %v150 to ptr
  %t35 = load i32, ptr %t34, align 1
  %v151 = zext i32 %t35 to i64
  br label %edge47

edge47:                                           ; preds = %edge38, %edge32
  %v564 = phi i64 [ %v151, %edge38 ], [ 0, %edge32 ]
  %t39.not = icmp eq i64 %v99, 0
  br i1 %t39.not, label %edge165, label %edge49

edge49:                                           ; preds = %edge47
  %v179 = add i64 %v99, -1
  %t42 = icmp ult i64 %v179, %v18
  br i1 %t42, label %edge65, label %edge71

edge65:                                           ; preds = %edge49
  %v223 = shl i64 %v99, 2
  %v228 = add i64 %v226, %v223
  %t44 = inttoptr i64 %v228 to ptr
  %t45 = load i32, ptr %t44, align 1
  %v229 = zext i32 %t45 to i64
  br label %edge71

edge71:                                           ; preds = %edge49, %edge65
  %v577 = phi i64 [ %v229, %edge65 ], [ 0, %edge49 ]
  %t46 = icmp samesign ugt i64 %v564, %v577
  %v2541 = and i1 %t28, %t46
  %v270 = zext i1 %v2541 to i64
  %spec.select = or disjoint i64 %v58, %v270
  %v288 = add i64 %spec.select, %v13
  %t51.not = icmp ult i64 %v288, %v13
  br i1 %t51.not, label %edge165, label %edge88

edge88:                                           ; preds = %edge71
  %t54.not = icmp eq i64 %v288, 0
  br i1 %t54.not, label %edge165, label %edge90

edge90:                                           ; preds = %edge88
  %v297 = add i64 %v288, -1
  %t57 = icmp ult i64 %v297, %v18
  br i1 %t57, label %edge105, label %block134

edge105:                                          ; preds = %edge90
  %v341 = shl i64 %v288, 2
  %v346 = add i64 %v226, %v341
  %t59 = inttoptr i64 %v346 to ptr
  %t60 = load i32, ptr %t59, align 1
  %v347 = zext i32 %t60 to i64
  %t61 = icmp ult i64 %v16, %v347
  br i1 %t61, label %edge114, label %block134

edge114:                                          ; preds = %edge105
  %v383 = add i64 %v548, %v13
  %t63.not = icmp ult i64 %v383, %v13
  br i1 %t63.not, label %edge165, label %edge117

edge117:                                          ; preds = %edge114
  %t66.not = icmp eq i64 %v383, 0
  br i1 %t66.not, label %edge165, label %edge119

edge119:                                          ; preds = %edge117
  %v402 = add i64 %v383, -1
  %t69 = icmp ult i64 %v402, %v18
  br i1 %t69, label %edge124, label %edge165

edge124:                                          ; preds = %edge119
  %v407 = shl i64 %v402, 2
  %v409 = add i64 %v407, %v17
  %t71 = inttoptr i64 %v409 to ptr
  store i32 %t60, ptr %t71, align 1
  br label %block6

edge148:                                          ; preds = %block134
  %t76.not = icmp eq i64 %v494, 0
  br i1 %t76.not, label %edge165, label %edge150

edge150:                                          ; preds = %edge148
  %v513 = add i64 %v494, -1
  %t79 = icmp ult i64 %v513, %v18
  br i1 %t79, label %edge155, label %edge165

edge155:                                          ; preds = %edge150
  %v518 = shl i64 %v513, 2
  %v520 = add i64 %v518, %v17
  %v522 = trunc i64 %v16 to i32
  %t81 = inttoptr i64 %v520 to ptr
  store i32 %v522, ptr %t81, align 1
  %t83 = inttoptr i64 %v2 to ptr
  %t84 = getelementptr i8, ptr %t83, i64 8
  store i64 %v17, ptr %t84, align 1
  %t86 = getelementptr i8, ptr %t83, i64 16
  store i64 %v18, ptr %t86, align 1
  %t88 = getelementptr i8, ptr %t83, i64 24
  store i64 %v19, ptr %t88, align 1
  %t90 = getelementptr i8, ptr %t83, i64 32
  store i64 %v20, ptr %t90, align 1
  %t92 = getelementptr i8, ptr %t83, i64 40
  store i64 %v17, ptr %t92, align 1
  %t94 = getelementptr i8, ptr %t83, i64 48
  store i64 %v18, ptr %t94, align 1
  %t96 = getelementptr i8, ptr %t83, i64 56
  store i64 %v6, ptr %t96, align 1
  %t98 = getelementptr i8, ptr %t83, i64 64
  store i64 %v19, ptr %t98, align 1
  %t100 = getelementptr i8, ptr %t83, i64 72
  store i64 %v20, ptr %t100, align 1
  %t102 = getelementptr i8, ptr %t83, i64 80
  store i64 %v5, ptr %t102, align 1
  br label %common.ret

edge165:                                          ; preds = %edge119, %edge117, %edge114, %edge88, %edge71, %edge47, %edge20, %block6, %edge150, %edge148, %block134
  %.sink = phi i64 [ 4223, %edge148 ], [ 4222, %block134 ], [ 4225, %edge150 ], [ 4101, %block6 ], [ 4112, %edge20 ], [ 4134, %edge47 ], [ 4166, %edge71 ], [ 4167, %edge88 ], [ 4193, %edge114 ], [ 4194, %edge117 ], [ 4196, %edge119 ]
  %v22 = phi i64 [ 1, %edge148 ], [ 1, %block134 ], [ 103169, %edge150 ], [ 1, %block6 ], [ 1, %edge20 ], [ 1, %edge47 ], [ 1, %edge71 ], [ 1, %edge88 ], [ 1, %edge114 ], [ 1, %edge117 ], [ 103169, %edge119 ]
  %t25 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t25, align 1
  %t106 = insertvalue { i64, i64 } poison, i64 %v22, 0
  %t107 = insertvalue { i64, i64 } %t106, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_4310(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 64
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 72
  %v6 = load i64, ptr %t3, align 1
  %v17 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v18 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v19 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v20 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 32
  %v21 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 40
  %v22 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 48
  %v23 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 56
  %v24 = load i64, ptr %t18, align 1
  %umax = tail call i64 @llvm.umax.i64(i64 %v19, i64 %v20)
  %umax27 = tail call i64 @llvm.umax.i64(i64 %v19, i64 %v22)
  br label %block6

block6:                                           ; preds = %edge253, %block0
  %v952 = phi i64 [ %v17, %block0 ], [ %v1434, %edge253 ]
  %v953 = phi i64 [ %v18, %block0 ], [ %v1439, %edge253 ]
  %v954 = phi i64 [ %v19, %block0 ], [ %v843, %edge253 ]
  %exitcond.not = icmp eq i64 %v954, %umax
  br i1 %exitcond.not, label %edge12, label %edge21

common.ret:                                       ; preds = %edge289, %edge12
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge12 ], [ %t122, %edge289 ]
  ret { i64, i64 } %common.ret.op

edge12:                                           ; preds = %block6
  %t98 = inttoptr i64 %v2 to ptr
  %t99 = getelementptr i8, ptr %t98, i64 8
  store i64 %v21, ptr %t99, align 1
  %t101 = getelementptr i8, ptr %t98, i64 16
  store i64 %v22, ptr %t101, align 1
  %t103 = getelementptr i8, ptr %t98, i64 24
  store i64 %v23, ptr %t103, align 1
  %t105 = getelementptr i8, ptr %t98, i64 32
  store i64 %v24, ptr %t105, align 1
  %t107 = getelementptr i8, ptr %t98, i64 40
  store i64 %v21, ptr %t107, align 1
  %t109 = getelementptr i8, ptr %t98, i64 48
  store i64 %v22, ptr %t109, align 1
  %t111 = getelementptr i8, ptr %t98, i64 56
  store i64 %v6, ptr %t111, align 1
  %t113 = getelementptr i8, ptr %t98, i64 64
  store i64 %v23, ptr %t113, align 1
  %t115 = getelementptr i8, ptr %t98, i64 72
  store i64 %v24, ptr %t115, align 1
  %t117 = getelementptr i8, ptr %t98, i64 80
  store i64 %v5, ptr %t117, align 1
  br label %common.ret

edge21:                                           ; preds = %block6
  %t21 = icmp ult i64 %v952, %v22
  br i1 %t21, label %edge34.thread, label %edge34

edge34.thread:                                    ; preds = %edge21
  %v118 = shl i64 %v952, 2
  %v120 = add i64 %v118, %v21
  %t23 = inttoptr i64 %v120 to ptr
  %t24 = load i32, ptr %t23, align 1
  %0 = and i32 %t24, -1024
  %1 = zext i32 %0 to i64
  br label %edge46

edge34:                                           ; preds = %edge21
  %t25.not = icmp eq i64 %v952, -1
  br i1 %t25.not, label %edge289, label %edge46

edge46:                                           ; preds = %edge34.thread, %edge34
  %v97013 = phi i64 [ %1, %edge34.thread ], [ 0, %edge34 ]
  %v14714 = add nuw i64 %v952, 1
  %t28 = icmp ult i64 %v14714, %v22
  br i1 %t28, label %edge52, label %edge59

edge52:                                           ; preds = %edge46
  %v196 = shl i64 %v14714, 2
  %v198 = add i64 %v196, %v21
  %t30 = inttoptr i64 %v198 to ptr
  %t31 = load i32, ptr %t30, align 1
  %2 = and i32 %t31, -1024
  %3 = zext i32 %2 to i64
  br label %edge59

edge59:                                           ; preds = %edge52, %edge46
  %v983 = phi i64 [ %3, %edge52 ], [ 0, %edge46 ]
  %t32 = icmp ult i64 %v953, %v22
  br i1 %t32, label %edge80.thread, label %edge80

edge80.thread:                                    ; preds = %edge59
  %v267 = shl i64 %v953, 2
  %v269 = add i64 %v267, %v21
  %t34 = inttoptr i64 %v269 to ptr
  %t35 = load i32, ptr %t34, align 1
  %v270 = zext i32 %t35 to i64
  br label %edge91

edge80:                                           ; preds = %edge59
  %t36.not = icmp eq i64 %v953, -1
  br i1 %t36.not, label %edge289, label %edge91

edge91:                                           ; preds = %edge80.thread, %edge80
  %v99518 = phi i64 [ %v270, %edge80.thread ], [ 0, %edge80 ]
  %v29219 = add nuw i64 %v953, 1
  %t39 = icmp ult i64 %v29219, %v22
  br i1 %t39, label %edge104.thread, label %edge104

edge104.thread:                                   ; preds = %edge91
  %v341 = shl i64 %v29219, 2
  %v343 = add i64 %v341, %v21
  %t41 = inttoptr i64 %v343 to ptr
  %t42 = load i32, ptr %t41, align 1
  %v344 = zext i32 %t42 to i64
  %v36922 = and i64 %v344, 4294966272
  %4 = and i64 %v344, 1023
  br label %edge119

edge104:                                          ; preds = %edge91
  %t49.not = icmp eq i64 %v953, -2
  br i1 %t49.not, label %edge289, label %edge119

edge119:                                          ; preds = %edge104.thread, %edge104
  %v36926 = phi i64 [ %v36922, %edge104.thread ], [ 0, %edge104 ]
  %v100524 = phi i64 [ %4, %edge104.thread ], [ 0, %edge104 ]
  %v36525 = and i64 %v99518, 4294966272
  %v413 = add nuw i64 %v953, 2
  %t46 = icmp ult i64 %v952, %v20
  %t47 = icmp eq i64 %v953, %v954
  %t48 = icmp samesign ule i64 %v983, %v36525
  %v3991.not5.not9 = or i1 %t47, %t48
  %v4042.not.not8 = and i1 %t46, %v3991.not5.not9
  %t52 = icmp ugt i64 %v413, %v954
  %t53 = icmp ule i64 %v952, %v20
  %t54 = icmp samesign uge i64 %v36926, %v97013
  %v4383.not7 = and i1 %t53, %t54
  %v4434.not6 = or i1 %t52, %v4383.not7
  %narrow.not = or i1 %v4042.not.not8, %v4434.not6
  %v454 = shl i64 %v954, 10
  br i1 %v4042.not.not8, label %edge185, label %edge134

edge130:                                          ; preds = %edge134
  %v476 = and i64 %v99518, 1023
  %v486 = or disjoint i64 %v476, %v454
  %v506 = shl i64 %v953, 2
  %v508 = add i64 %v506, %v21
  %v510 = trunc i64 %v486 to i32
  %t59 = inttoptr i64 %v508 to ptr
  store i32 %v510, ptr %t59, align 1
  br i1 %narrow.not, label %edge176, label %edge156

edge134:                                          ; preds = %edge119
  br i1 %t32, label %edge130, label %edge289

edge156:                                          ; preds = %edge130
  br i1 %t39, label %edge164, label %edge289

edge164:                                          ; preds = %edge156
  %v544 = or disjoint i64 %v100524, %v454
  %v564 = shl i64 %v29219, 2
  %v566 = add i64 %v564, %v21
  %v568 = trunc i64 %v544 to i32
  %t67 = inttoptr i64 %v566 to ptr
  store i32 %v568, ptr %t67, align 1
  br label %edge176

edge176:                                          ; preds = %edge130, %edge164, %edge185
  %v1434 = phi i64 [ %v952, %edge164 ], [ %v612, %edge185 ], [ %v14714, %edge130 ]
  %exitcond28.not = icmp eq i64 %v954, %umax27
  br i1 %exitcond28.not, label %edge289, label %edge253

edge185:                                          ; preds = %edge119
  %v612 = add nuw i64 %v952, 2
  %t71.not = icmp eq i64 %v952, -2
  br i1 %t71.not, label %edge289, label %edge176

edge253:                                          ; preds = %edge176
  %v709 = add nuw nsw i64 %v983, %v97013
  %v1156.v = select i1 %v4434.not6, i64 %v97013, i64 %v36926
  %v1156 = add nuw nsw i64 %v1156.v, %v36525
  %v1271 = select i1 %v4042.not.not8, i64 %v709, i64 %v1156
  %v1093 = select i1 %narrow.not, i64 %v29219, i64 %v413
  %v1439 = select i1 %v4042.not.not8, i64 %v953, i64 %v1093
  %v799 = shl i64 %v954, 2
  %v801 = add i64 %v799, %v21
  %t89 = inttoptr i64 %v801 to ptr
  %t90 = load i32, ptr %t89, align 1
  %5 = and i32 %t90, 1023
  %v843 = add i64 %v954, 1
  %6 = trunc i64 %v1271 to i32
  %v857 = or i32 %5, %6
  store i32 %v857, ptr %t89, align 1
  br label %block6

edge289:                                          ; preds = %edge176, %edge185, %edge156, %edge134, %edge104, %edge80, %edge34
  %.sink = phi i64 [ 4471, %edge185 ], [ 4348, %edge34 ], [ 4392, %edge80 ], [ 4423, %edge104 ], [ 4443, %edge134 ], [ 4455, %edge156 ], [ 4541, %edge176 ]
  %v26 = phi i64 [ 1, %edge185 ], [ 1, %edge34 ], [ 1, %edge80 ], [ 1, %edge104 ], [ 103169, %edge134 ], [ 103169, %edge156 ], [ 103169, %edge176 ]
  %t27 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t27, align 1
  %t121 = insertvalue { i64, i64 } poison, i64 %v26, 0
  %t122 = insertvalue { i64, i64 } %t121, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_4810_vector(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %v15 = load i64, ptr %t0, align 1
  %t2 = getelementptr i8, ptr %t0, i64 8
  %v16 = load i64, ptr %t2, align 1
  %t4 = getelementptr i8, ptr %t0, i64 16
  %v17 = load i64, ptr %t4, align 1
  %t6 = getelementptr i8, ptr %t0, i64 24
  %v18 = load i64, ptr %t6, align 1
  %t12 = getelementptr i8, ptr %t0, i64 48
  %v21 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 56
  %v22 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 64
  %v23 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 72
  %v24 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 80
  %v25 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 88
  %v26 = load i64, ptr %t22, align 1
  br label %block6

block6:                                           ; preds = %edge107, %block0
  %v405 = phi i64 [ %v15, %block0 ], [ %v368, %edge107 ]
  %v406 = phi i64 [ %v16, %block0 ], [ %v597, %edge107 ]
  %t27.not = icmp eq i64 %v405, 0
  br i1 %t27.not, label %edge115, label %edge23

block42:                                          ; preds = %edge87, %edge28
  %v425 = phi i64 [ %v406, %edge28 ], [ %v295, %edge87 ]
  %v426 = phi i64 [ %v123, %edge28 ], [ %v304, %edge87 ]
  %t32.not = icmp eq i64 %v426, 0
  br i1 %t32.not, label %edge107, label %edge57

common.ret:                                       ; preds = %edge117, %edge115
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge115 ], [ %t77, %edge117 ]
  ret { i64, i64 } %common.ret.op

edge23:                                           ; preds = %block6
  %t29 = icmp ult i64 %v405, %v26
  br i1 %t29, label %edge28, label %edge107

edge28:                                           ; preds = %edge23
  %v120 = shl i64 %v405, 3
  %v122 = add i64 %v120, %v25
  %t31 = inttoptr i64 %v122 to ptr
  %v123 = load i64, ptr %t31, align 1
  %v290 = trunc i64 %v405 to i8
  br label %block42

edge57:                                           ; preds = %block42
  %t34 = icmp ult i64 %v425, %v18
  br i1 %t34, label %edge63, label %edge70

edge63:                                           ; preds = %edge57
  %v237 = shl i64 %v425, 2
  %v239 = add i64 %v237, %v17
  %t36 = inttoptr i64 %v239 to ptr
  %t37 = load i32, ptr %t36, align 1
  %0 = and i32 %t37, 1023
  %1 = zext nneg i32 %0 to i64
  br label %edge70

edge70:                                           ; preds = %edge63, %edge57
  %v461 = phi i64 [ %1, %edge63 ], [ 0, %edge57 ]
  %t38 = icmp ult i64 %v461, %v22
  br i1 %t38, label %edge76, label %edge117

edge76:                                           ; preds = %edge70
  %v288 = add i64 %v461, %v21
  %t40 = inttoptr i64 %v288 to ptr
  store i8 %v290, ptr %t40, align 1
  %t42.not = icmp eq i64 %v425, -1
  br i1 %t42.not, label %edge117, label %edge87

edge87:                                           ; preds = %edge76
  %v295 = add nuw i64 %v425, 1
  %v304 = add i64 %v426, -1
  br label %block42

edge107:                                          ; preds = %block42, %edge23
  %v597 = phi i64 [ %v406, %edge23 ], [ %v425, %block42 ]
  %v368 = add i64 %v405, -1
  br label %block6

edge115:                                          ; preds = %block6
  %t49 = inttoptr i64 %v2 to ptr
  %t50 = getelementptr i8, ptr %t49, i64 8
  store i64 %v21, ptr %t50, align 1
  %t52 = getelementptr i8, ptr %t49, i64 16
  store i64 %v22, ptr %t52, align 1
  %t54 = getelementptr i8, ptr %t49, i64 24
  store i64 %v23, ptr %t54, align 1
  %t56 = getelementptr i8, ptr %t49, i64 32
  store i64 %v24, ptr %t56, align 1
  %t58 = getelementptr i8, ptr %t49, i64 40
  store i64 %v21, ptr %t58, align 1
  %t60 = getelementptr i8, ptr %t49, i64 48
  store i64 %v22, ptr %t60, align 1
  %t62 = getelementptr i8, ptr %t0, i64 120
  %v6 = load i64, ptr %t62, align 1
  %t64 = getelementptr i8, ptr %t49, i64 56
  store i64 %v6, ptr %t64, align 1
  %t66 = getelementptr i8, ptr %t49, i64 64
  store i64 %v23, ptr %t66, align 1
  %t68 = getelementptr i8, ptr %t49, i64 72
  store i64 %v24, ptr %t68, align 1
  %t70 = getelementptr i8, ptr %t0, i64 112
  %v5 = load i64, ptr %t70, align 1
  %t72 = getelementptr i8, ptr %t49, i64 80
  store i64 %v5, ptr %t72, align 1
  br label %common.ret

edge117:                                          ; preds = %edge76, %edge70
  %.sink = phi i64 [ 4889, %edge70 ], [ 4890, %edge76 ]
  %v32 = phi i64 [ 103169, %edge70 ], [ 1, %edge76 ]
  %t41 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t41, align 1
  %t76 = insertvalue { i64, i64 } poison, i64 %v32, 0
  %t77 = insertvalue { i64, i64 } %t76, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_4934(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 80
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 88
  %v6 = load i64, ptr %t3, align 1
  %v14 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v15 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v16 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v17 = load i64, ptr %t10, align 1
  %t16 = getelementptr i8, ptr %t0, i64 48
  %v20 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 56
  %v21 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 64
  %v22 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 72
  %v23 = load i64, ptr %t22, align 1
  %v119 = add i64 %v20, -4
  %v190 = add i64 %v16, -8
  %umax = tail call i64 @llvm.umax.i64(i64 %v15, i64 %v21)
  br label %block6

block6:                                           ; preds = %edge75, %block0
  %v320 = phi i64 [ %v15, %block0 ], [ %v261, %edge75 ]
  %t23.not = icmp ugt i64 %v320, %v14
  br i1 %t23.not, label %edge90, label %edge15

common.ret:                                       ; preds = %edge92, %edge90
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge90 ], [ %t67, %edge92 ]
  ret { i64, i64 } %common.ret.op

edge15:                                           ; preds = %block6
  %t25.not = icmp eq i64 %v320, 0
  br i1 %t25.not, label %edge92, label %edge16

edge16:                                           ; preds = %edge15
  %v72 = add i64 %v320, -1
  %t28 = icmp ult i64 %v72, %v21
  br i1 %t28, label %edge32, label %edge39

edge32:                                           ; preds = %edge16
  %v116 = shl i64 %v320, 2
  %v121 = add i64 %v119, %v116
  %t30 = inttoptr i64 %v121 to ptr
  %t31 = load i32, ptr %t30, align 1
  %v122 = zext i32 %t31 to i64
  br label %edge39

edge39:                                           ; preds = %edge32, %edge16
  %v332 = phi i64 [ %v122, %edge32 ], [ 0, %edge16 ]
  %t32 = icmp ult i64 %v72, %v17
  br i1 %t32, label %edge53, label %edge59

edge53:                                           ; preds = %edge39
  %v187 = shl i64 %v320, 3
  %v192 = add i64 %v190, %v187
  %t34 = inttoptr i64 %v192 to ptr
  %v193 = load i64, ptr %t34, align 1
  br label %edge59

edge59:                                           ; preds = %edge39, %edge53
  %v343 = phi i64 [ %v193, %edge53 ], [ 0, %edge39 ]
  %exitcond.not = icmp eq i64 %v320, %umax
  br i1 %exitcond.not, label %edge92, label %edge75

edge75:                                           ; preds = %edge59
  %v261 = add i64 %v320, 1
  %v227 = add i64 %v343, %v332
  %v271 = shl i64 %v320, 2
  %v273 = add i64 %v271, %v20
  %v227.tr = trunc i64 %v227 to i32
  %v275 = shl i32 %v227.tr, 1
  %t38 = inttoptr i64 %v273 to ptr
  store i32 %v275, ptr %t38, align 1
  br label %block6

edge90:                                           ; preds = %block6
  %t43 = inttoptr i64 %v2 to ptr
  %t44 = getelementptr i8, ptr %t43, i64 8
  store i64 %v20, ptr %t44, align 1
  %t46 = getelementptr i8, ptr %t43, i64 16
  store i64 %v21, ptr %t46, align 1
  %t48 = getelementptr i8, ptr %t43, i64 24
  store i64 %v22, ptr %t48, align 1
  %t50 = getelementptr i8, ptr %t43, i64 32
  store i64 %v23, ptr %t50, align 1
  %t52 = getelementptr i8, ptr %t43, i64 40
  store i64 %v20, ptr %t52, align 1
  %t54 = getelementptr i8, ptr %t43, i64 48
  store i64 %v21, ptr %t54, align 1
  %t56 = getelementptr i8, ptr %t43, i64 56
  store i64 %v6, ptr %t56, align 1
  %t58 = getelementptr i8, ptr %t43, i64 64
  store i64 %v22, ptr %t58, align 1
  %t60 = getelementptr i8, ptr %t43, i64 72
  store i64 %v23, ptr %t60, align 1
  %t62 = getelementptr i8, ptr %t43, i64 80
  store i64 %v5, ptr %t62, align 1
  br label %common.ret

edge92:                                           ; preds = %edge59, %edge15
  %.sink = phi i64 [ 4949, %edge15 ], [ 5003, %edge59 ]
  %v26 = phi i64 [ 1, %edge15 ], [ 103169, %edge59 ]
  %t27 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t27, align 1
  %t66 = insertvalue { i64, i64 } poison, i64 %v26, 0
  %t67 = insertvalue { i64, i64 } %t66, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_5014(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 144
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 152
  %v6 = load i64, ptr %t3, align 1
  %t5 = getelementptr i8, ptr %t0, i64 160
  %v7 = load i64, ptr %t5, align 1
  %t7 = getelementptr i8, ptr %t0, i64 168
  %v8 = load i64, ptr %t7, align 1
  %v21 = load i64, ptr %t0, align 1
  %t10 = getelementptr i8, ptr %t0, i64 8
  %v22 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 16
  %v23 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 24
  %v24 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 32
  %v25 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 40
  %v26 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 48
  %v27 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 56
  %v28 = load i64, ptr %t22, align 1
  %t28 = getelementptr i8, ptr %t0, i64 80
  %v31 = load i64, ptr %t28, align 1
  %t30 = getelementptr i8, ptr %t0, i64 88
  %v32 = load i64, ptr %t30, align 1
  %t32 = getelementptr i8, ptr %t0, i64 96
  %v33 = load i64, ptr %t32, align 1
  %t34 = getelementptr i8, ptr %t0, i64 104
  %v34 = load i64, ptr %t34, align 1
  %t36 = getelementptr i8, ptr %t0, i64 112
  %v35 = load i64, ptr %t36, align 1
  %t38 = getelementptr i8, ptr %t0, i64 120
  %v36 = load i64, ptr %t38, align 1
  %umax = tail call i64 @llvm.umax.i64(i64 %v21, i64 %v22)
  %umax6 = tail call i64 @llvm.umax.i64(i64 %v22, i64 %v24)
  br label %block6

block6:                                           ; preds = %edge137, %block0
  %v585 = phi i64 [ %v22, %block0 ], [ %v483, %edge137 ]
  %exitcond.not = icmp eq i64 %v585, %umax
  br i1 %exitcond.not, label %edge13, label %edge22

block131:                                         ; preds = %edge27, %edge118, %edge22
  %v915 = phi i32 [ 0, %edge22 ], [ %2, %edge118 ], [ 0, %edge27 ]
  %exitcond7.not = icmp eq i64 %v585, %umax6
  br i1 %exitcond7.not, label %edge155, label %edge137

common.ret:                                       ; preds = %edge155, %edge13
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge13 ], [ %t117, %edge155 ]
  ret { i64, i64 } %common.ret.op

edge13:                                           ; preds = %block6
  %t81 = inttoptr i64 %v2 to ptr
  %t82 = getelementptr i8, ptr %t81, i64 8
  store i64 %v23, ptr %t82, align 1
  %t84 = getelementptr i8, ptr %t81, i64 16
  store i64 %v24, ptr %t84, align 1
  %t86 = getelementptr i8, ptr %t81, i64 24
  store i64 %v25, ptr %t86, align 1
  %t88 = getelementptr i8, ptr %t81, i64 32
  store i64 %v26, ptr %t88, align 1
  %t90 = getelementptr i8, ptr %t81, i64 40
  store i64 %v23, ptr %t90, align 1
  %t92 = getelementptr i8, ptr %t81, i64 48
  store i64 %v24, ptr %t92, align 1
  %t94 = getelementptr i8, ptr %t81, i64 56
  store i64 %v7, ptr %t94, align 1
  %t96 = getelementptr i8, ptr %t81, i64 64
  store i64 %v25, ptr %t96, align 1
  %t98 = getelementptr i8, ptr %t81, i64 72
  store i64 %v26, ptr %t98, align 1
  %t100 = getelementptr i8, ptr %t81, i64 80
  store i64 %v5, ptr %t100, align 1
  %t102 = getelementptr i8, ptr %t81, i64 88
  store i64 %v31, ptr %t102, align 1
  %t104 = getelementptr i8, ptr %t81, i64 96
  store i64 %v32, ptr %t104, align 1
  %t106 = getelementptr i8, ptr %t81, i64 104
  store i64 %v8, ptr %t106, align 1
  %t108 = getelementptr i8, ptr %t81, i64 112
  store i64 %v33, ptr %t108, align 1
  %t110 = getelementptr i8, ptr %t81, i64 120
  store i64 %v34, ptr %t110, align 1
  %t112 = getelementptr i8, ptr %t81, i64 128
  store i64 %v6, ptr %t112, align 1
  br label %common.ret

edge22:                                           ; preds = %block6
  %t45 = icmp ult i64 %v585, %v28
  br i1 %t45, label %edge27, label %block131

edge27:                                           ; preds = %edge22
  %v135 = add i64 %v585, %v27
  %t47 = inttoptr i64 %v135 to ptr
  %t48 = load i8, ptr %t47, align 1
  %v136 = zext i8 %t48 to i64
  %t49 = icmp eq i8 %t48, 0
  br i1 %t49, label %block131, label %edge48

edge48:                                           ; preds = %edge27
  %t51 = icmp ugt i64 %v32, %v136
  br i1 %t51, label %edge60, label %edge155

edge60:                                           ; preds = %edge48
  %v225 = shl nuw nsw i64 %v136, 2
  %v227 = add i64 %v225, %v31
  %t53 = inttoptr i64 %v227 to ptr
  %t54 = load i32, ptr %t53, align 1
  %v228 = zext i32 %t54 to i64
  %t55.not = icmp eq i32 %t54, -1
  br i1 %t55.not, label %edge155, label %edge66

edge66:                                           ; preds = %edge60
  %v267 = add nuw i32 %t54, 1
  store i32 %v267, ptr %t53, align 1
  %v274 = and i64 %v228, 255
  %t62 = icmp ult i64 %v274, %v36
  br i1 %t62, label %edge87, label %edge94

edge87:                                           ; preds = %edge66
  %v323 = add i64 %v274, %v35
  %t64 = inttoptr i64 %v323 to ptr
  %t65 = load i8, ptr %t64, align 1
  %v324 = zext i8 %t65 to i64
  %0 = shl nuw nsw i64 %v324, 8
  br label %edge94

edge94:                                           ; preds = %edge87, %edge66
  %v637 = phi i64 [ %0, %edge87 ], [ 0, %edge66 ]
  %v346 = lshr i64 %v228, 8
  %t67 = icmp ult i64 %v346, %v36
  br i1 %t67, label %edge108, label %edge115

edge108:                                          ; preds = %edge94
  %v395 = add i64 %v346, %v35
  %t69 = inttoptr i64 %v395 to ptr
  %t70 = load i8, ptr %t69, align 1
  %v396 = zext i8 %t70 to i64
  %1 = or disjoint i64 %v637, %v396
  br label %edge115

edge115:                                          ; preds = %edge108, %edge94
  %v647 = phi i64 [ %1, %edge108 ], [ %v637, %edge94 ]
  %t72 = icmp ult i8 %t48, 17
  br i1 %t72, label %edge118, label %edge155

edge118:                                          ; preds = %edge115
  %v428 = sub nuw nsw i64 16, %v136
  %v434 = lshr i64 %v647, %v428
  %v452 = shl nuw nsw i64 %v136, 16
  %v457 = or i64 %v434, %v452
  %2 = trunc nuw nsw i64 %v457 to i32
  br label %block131

edge137:                                          ; preds = %block131
  %v483 = add i64 %v585, 1
  %v493 = shl i64 %v585, 2
  %v495 = add i64 %v493, %v23
  %t79 = inttoptr i64 %v495 to ptr
  store i32 %v915, ptr %t79, align 1
  br label %block6

edge155:                                          ; preds = %block131, %edge115, %edge48, %edge60
  %.sink = phi i64 [ 5131, %edge115 ], [ 5081, %edge60 ], [ 5083, %edge48 ], [ 5145, %block131 ]
  %v43 = phi i64 [ 1, %edge115 ], [ 1, %edge60 ], [ 103169, %edge48 ], [ 103169, %block131 ]
  %t57 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t57, align 1
  %t116 = insertvalue { i64, i64 } poison, i64 %v43, 0
  %t117 = insertvalue { i64, i64 } %t116, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_6038(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 104
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 112
  %v6 = load i64, ptr %t3, align 1
  %v22 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v23 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v24 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v25 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 32
  %v26 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 40
  %v27 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 48
  %v28 = load i64, ptr %t16, align 1
  %t22 = getelementptr i8, ptr %t0, i64 72
  %v31 = load i64, ptr %t22, align 1
  %t24 = getelementptr i8, ptr %t0, i64 80
  %v32 = load i64, ptr %t24, align 1
  %umax = tail call i64 @llvm.umax.i64(i64 %v22, i64 547)
  %umax8 = tail call i64 @llvm.umax.i64(i64 %v22, i64 %v32)
  %umax10 = tail call i64 @llvm.umax.i64(i64 %v22, i64 %v24)
  br label %block6

block6:                                           ; preds = %edge91, %block0
  %v385 = phi i64 [ %v22, %block0 ], [ %v309, %edge91 ]
  %exitcond = icmp eq i64 %v385, %umax
  br i1 %exitcond, label %edge76, label %edge14

common.ret:                                       ; preds = %edge106, %edge76
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge76 ], [ %t86, %edge106 ]
  ret { i64, i64 } %common.ret.op

edge14:                                           ; preds = %block6
  %exitcond9.not = icmp eq i64 %v385, %umax8
  br i1 %exitcond9.not, label %edge106, label %edge39

edge39:                                           ; preds = %edge14
  %v92 = shl nuw nsw i64 %v385, 1
  %v94 = add i64 %v92, %v31
  %t33 = inttoptr i64 %v94 to ptr
  %t34 = load i16, ptr %t33, align 1
  %v95 = zext i16 %t34 to i64
  %v113 = and i64 %v95, 511
  %v118 = lshr i64 %v95, 9
  %t38 = icmp ult i64 %v113, %v28
  br i1 %t38, label %edge78, label %edge78.thread

edge78.thread:                                    ; preds = %edge39
  %t433 = icmp samesign ult i64 %v113, 288
  %spec.select4 = select i1 %t433, i64 13, i64 10
  br label %2

edge76:                                           ; preds = %block6
  %t62 = inttoptr i64 %v2 to ptr
  %t63 = getelementptr i8, ptr %t62, i64 8
  store i64 %v23, ptr %t63, align 1
  %t65 = getelementptr i8, ptr %t62, i64 16
  store i64 %v24, ptr %t65, align 1
  %t67 = getelementptr i8, ptr %t62, i64 24
  store i64 %v25, ptr %t67, align 1
  %t69 = getelementptr i8, ptr %t62, i64 32
  store i64 %v26, ptr %t69, align 1
  %t71 = getelementptr i8, ptr %t62, i64 40
  store i64 %v23, ptr %t71, align 1
  %t73 = getelementptr i8, ptr %t62, i64 48
  store i64 %v24, ptr %t73, align 1
  %t75 = getelementptr i8, ptr %t62, i64 56
  store i64 %v6, ptr %t75, align 1
  %t77 = getelementptr i8, ptr %t62, i64 64
  store i64 %v25, ptr %t77, align 1
  %t79 = getelementptr i8, ptr %t62, i64 72
  store i64 %v26, ptr %t79, align 1
  %t81 = getelementptr i8, ptr %t62, i64 80
  store i64 %v5, ptr %t81, align 1
  br label %common.ret

edge78:                                           ; preds = %edge39
  %v176 = shl nuw nsw i64 %v113, 2
  %v178 = add i64 %v176, %v27
  %t40 = inttoptr i64 %v178 to ptr
  %t41 = load i32, ptr %t40, align 1
  %t41.fr = freeze i32 %t41
  %0 = lshr i32 %t41.fr, 16
  %1 = zext nneg i32 %0 to i64
  %t45.not = icmp eq i32 %0, 0
  %t43 = icmp samesign ult i64 %v113, 288
  %spec.select = select i1 %t43, i64 13, i64 10
  %spec.select7 = select i1 %t45.not, i64 %spec.select, i64 %1
  br label %2

2:                                                ; preds = %edge78, %edge78.thread
  %3 = phi i64 [ %spec.select7, %edge78 ], [ %spec.select4, %edge78.thread ]
  %exitcond11.not = icmp eq i64 %v385, %umax10
  br i1 %exitcond11.not, label %edge106, label %edge91

edge91:                                           ; preds = %2
  %v284 = add nuw nsw i64 %3, %v118
  %v309 = add i64 %v385, 1
  %v319 = shl nuw nsw i64 %v385, 2
  %v321 = add i64 %v319, %v23
  %v284.tr = trunc nuw nsw i64 %v284 to i32
  %v323 = shl nuw nsw i32 %v284.tr, 4
  %t60 = inttoptr i64 %v321 to ptr
  store i32 %v323, ptr %t60, align 1
  br label %block6

edge106:                                          ; preds = %2, %edge14
  %.sink = phi i64 [ 6054, %edge14 ], [ 6119, %2 ]
  %t35 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t35, align 1
  %t86 = insertvalue { i64, i64 } { i64 103169, i64 poison }, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_6776(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 96
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 104
  %v6 = load i64, ptr %t3, align 1
  %v21 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v22 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v23 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v24 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 32
  %v25 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 40
  %v26 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 48
  %v27 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 56
  %v28 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 64
  %v29 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 72
  %v30 = load i64, ptr %t22, align 1
  %t61 = icmp ugt i64 %v23, 8
  %v296 = sub nuw nsw i64 8, %v23
  %umax = tail call i64 @llvm.umax.i64(i64 %v24, i64 547)
  %umax2 = tail call i64 @llvm.umax.i64(i64 %v24, i64 %v26)
  br label %block6

block6:                                           ; preds = %edge109, %block0
  %v428 = phi i64 [ %v24, %block0 ], [ %v343, %edge109 ]
  %t27 = icmp ult i64 %v428, 547
  br i1 %t27, label %edge16, label %edge74

common.ret:                                       ; preds = %edge124, %edge105
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge105 ], [ %t105, %edge124 ]
  ret { i64, i64 } %common.ret.op

edge16:                                           ; preds = %block6
  %t29 = icmp ult i64 %v428, %v30
  br i1 %t29, label %edge27, label %edge124

edge27:                                           ; preds = %edge16
  %v99 = shl nuw nsw i64 %v428, 1
  %v101 = add i64 %v99, %v29
  %t31 = inttoptr i64 %v101 to ptr
  %t32 = load i16, ptr %t31, align 1
  %0 = lshr i16 %t32, 5
  %1 = and i16 %0, 2032
  %t35 = icmp samesign ult i64 %v428, 259
  %t37 = icmp samesign ult i64 %v428, 515
  %v463 = select i1 %t37, i64 %v21, i64 78
  %v581 = select i1 %t35, i64 %v22, i64 %v463
  %t40 = icmp ult i64 %v428, %v26
  br i1 %t40, label %edge64, label %edge74

edge64:                                           ; preds = %edge27
  %v235 = shl nuw nsw i64 %v428, 2
  %v237 = add i64 %v235, %v25
  %t42 = inttoptr i64 %v237 to ptr
  %t43 = load i32, ptr %t42, align 1
  %v238 = zext i32 %t43 to i64
  %2 = mul nuw nsw i64 %v296, %v238
  br label %edge74

edge74:                                           ; preds = %block6, %edge27, %edge64
  %v58112 = phi i64 [ %v581, %edge27 ], [ %v581, %edge64 ], [ 78, %block6 ]
  %v43811.shrunk = phi i16 [ %1, %edge27 ], [ %1, %edge64 ], [ 0, %block6 ]
  %v582 = phi i64 [ 0, %edge27 ], [ %2, %edge64 ], [ 0, %block6 ]
  %v43811 = zext nneg i16 %v43811.shrunk to i64
  %v276 = add i64 %v58112, %v43811
  %t51.not = icmp ult i64 %v276, %v58112
  br i1 %t51.not, label %edge124, label %edge82

edge82:                                           ; preds = %edge74
  %umul = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v276, i64 %v23)
  %3 = extractvalue { i64, i1 } %umul, 1
  %brmerge = or i1 %3, %t61
  br i1 %brmerge, label %edge124.split.loop.exit, label %edge91

edge91:                                           ; preds = %edge82
  %v289 = mul i64 %v276, %v23
  %v308 = add i64 %v582, %v289
  %t71.not = icmp ult i64 %v308, %v289
  br i1 %t71.not, label %edge124, label %edge99

edge99:                                           ; preds = %edge91
  %v3171 = lshr i64 %v308, 3
  %exitcond.not = icmp eq i64 %v428, %umax
  br i1 %exitcond.not, label %edge105, label %edge107

edge105:                                          ; preds = %edge99
  %t81 = inttoptr i64 %v2 to ptr
  %t82 = getelementptr i8, ptr %t81, i64 8
  store i64 %v25, ptr %t82, align 1
  %t84 = getelementptr i8, ptr %t81, i64 16
  store i64 %v26, ptr %t84, align 1
  %t86 = getelementptr i8, ptr %t81, i64 24
  store i64 %v27, ptr %t86, align 1
  %t88 = getelementptr i8, ptr %t81, i64 32
  store i64 %v28, ptr %t88, align 1
  %t90 = getelementptr i8, ptr %t81, i64 40
  store i64 %v25, ptr %t90, align 1
  %t92 = getelementptr i8, ptr %t81, i64 48
  store i64 %v26, ptr %t92, align 1
  %t94 = getelementptr i8, ptr %t81, i64 56
  store i64 %v6, ptr %t94, align 1
  %t96 = getelementptr i8, ptr %t81, i64 64
  store i64 %v27, ptr %t96, align 1
  %t98 = getelementptr i8, ptr %t81, i64 72
  store i64 %v28, ptr %t98, align 1
  %t100 = getelementptr i8, ptr %t81, i64 80
  store i64 %v5, ptr %t100, align 1
  br label %common.ret

edge107:                                          ; preds = %edge99
  %exitcond3.not = icmp eq i64 %v428, %umax2
  br i1 %exitcond3.not, label %edge124, label %edge109

edge109:                                          ; preds = %edge107
  %v343 = add i64 %v428, 1
  %v353 = shl nuw nsw i64 %v428, 2
  %v355 = add i64 %v353, %v25
  %v357 = trunc i64 %v3171 to i32
  %t79 = inttoptr i64 %v355 to ptr
  store i32 %v357, ptr %t79, align 1
  br label %block6

edge124.split.loop.exit:                          ; preds = %edge82
  %.mux.le = select i1 %3, i64 6850, i64 6851
  br label %edge124

edge124:                                          ; preds = %edge16, %edge74, %edge91, %edge107, %edge124.split.loop.exit
  %.sink = phi i64 [ 6848, %edge74 ], [ 6853, %edge91 ], [ 6862, %edge107 ], [ %.mux.le, %edge124.split.loop.exit ], [ 6794, %edge16 ]
  %v35 = phi i64 [ 1, %edge74 ], [ 1, %edge91 ], [ 103169, %edge107 ], [ 1, %edge124.split.loop.exit ], [ 103169, %edge16 ]
  %t33 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t33, align 1
  %t104 = insertvalue { i64, i64 } poison, i64 %v35, 0
  %t105 = insertvalue { i64, i64 } %t104, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_8169(i64 %v1, i64 %v2) unnamed_addr #6 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 56
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 64
  %v6 = load i64, ptr %t3, align 1
  %v12 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v13 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v14 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v15 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 32
  %v16 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 40
  %v17 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 48
  %v18 = load i64, ptr %t16, align 1
  %umax = tail call i64 @llvm.umax.i64(i64 %v14, i64 %v13)
  %umax1 = tail call i64 @llvm.umax.i64(i64 %v14, i64 %v16)
  br label %block6

block6:                                           ; preds = %edge59, %block0
  %v240 = phi i64 [ %v14, %block0 ], [ %v197, %edge59 ]
  %exitcond.not = icmp eq i64 %v240, %umax
  br i1 %exitcond.not, label %edge66, label %edge15

common.ret:                                       ; preds = %edge68, %edge66
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge66 ], [ %t58, %edge68 ]
  ret { i64, i64 } %common.ret.op

edge15:                                           ; preds = %block6
  %v66 = add i64 %v240, %v12
  %t19.not = icmp ult i64 %v66, %v12
  br i1 %t19.not, label %edge68, label %edge20

edge20:                                           ; preds = %edge15
  %t22 = icmp ult i64 %v66, %v16
  br i1 %t22, label %edge26, label %edge41

edge26:                                           ; preds = %edge20
  %v103 = shl i64 %v66, 2
  %v105 = add i64 %v103, %v15
  %t24 = inttoptr i64 %v105 to ptr
  %t25 = load i32, ptr %t24, align 1
  br label %edge41

edge41:                                           ; preds = %edge20, %edge26
  %v319 = phi i32 [ 0, %edge20 ], [ %t25, %edge26 ]
  %exitcond2.not = icmp eq i64 %v240, %umax1
  br i1 %exitcond2.not, label %edge68, label %edge59

edge59:                                           ; preds = %edge41
  %v162 = shl i64 %v240, 2
  %v164 = add i64 %v162, %v15
  %t30 = inttoptr i64 %v164 to ptr
  store i32 %v319, ptr %t30, align 1
  %v197 = add i64 %v240, 1
  br label %block6

edge66:                                           ; preds = %block6
  %t34 = inttoptr i64 %v2 to ptr
  %t35 = getelementptr i8, ptr %t34, i64 8
  store i64 %v15, ptr %t35, align 1
  %t37 = getelementptr i8, ptr %t34, i64 16
  store i64 %v16, ptr %t37, align 1
  %t39 = getelementptr i8, ptr %t34, i64 24
  store i64 %v17, ptr %t39, align 1
  %t41 = getelementptr i8, ptr %t34, i64 32
  store i64 %v18, ptr %t41, align 1
  %t43 = getelementptr i8, ptr %t34, i64 40
  store i64 %v15, ptr %t43, align 1
  %t45 = getelementptr i8, ptr %t34, i64 48
  store i64 %v16, ptr %t45, align 1
  %t47 = getelementptr i8, ptr %t34, i64 56
  store i64 %v6, ptr %t47, align 1
  %t49 = getelementptr i8, ptr %t34, i64 64
  store i64 %v17, ptr %t49, align 1
  %t51 = getelementptr i8, ptr %t34, i64 72
  store i64 %v18, ptr %t51, align 1
  %t53 = getelementptr i8, ptr %t34, i64 80
  store i64 %v5, ptr %t53, align 1
  br label %common.ret

edge68:                                           ; preds = %edge41, %edge15
  %.sink = phi i64 [ 8184, %edge15 ], [ 8207, %edge41 ]
  %v20 = phi i64 [ 1, %edge15 ], [ 103169, %edge41 ]
  %t21 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t21, align 1
  %t57 = insertvalue { i64, i64 } poison, i64 %v20, 0
  %t58 = insertvalue { i64, i64 } %t57, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.uadd.sat.i64(i64, i64) #7

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.umul.with.overflow.i64(i64, i64) #7

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #7

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.umax.i32(i32, i32) #7

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smax.i64(i64, i64) #7

attributes #0 = { noinline uwtable "frame-pointer"="all" "target-cpu"="apple-m1" }
attributes #1 = { cold noreturn nounwind memory(inaccessiblemem: write) "target-cpu"="apple-m1" }
attributes #2 = { mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none) "target-cpu"="apple-m1" }
attributes #3 = { "target-cpu"="apple-m1" }
attributes #4 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) "target-cpu"="apple-m1" }
attributes #5 = { noinline nounwind memory(readwrite, inaccessiblemem: write, target_mem0: none, target_mem1: none) uwtable "frame-pointer"="all" "target-cpu"="apple-m1" }
attributes #6 = { nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable "frame-pointer"="all" "target-cpu"="apple-m1" }
attributes #7 = { nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none) }
