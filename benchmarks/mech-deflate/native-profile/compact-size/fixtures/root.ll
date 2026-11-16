; ModuleID = 'root.ll'
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

; Function Attrs: noinline uwtable
define { i64, i64 } @mech_compact_closed_checked(i64 %v0, i64 %v1, i64 %v2) local_unnamed_addr #0 align 128 {
block0:
  %ss0 = alloca [48 x i8], align 16
  %ss1 = alloca [48 x i8], align 8
  %ss2 = alloca [440 x i8], align 16
  %ss3 = alloca [616 x i8], align 8
  %ss4 = alloca [32 x i8], align 16
  %t0 = inttoptr i64 %v1 to ptr
  %v1394 = load i64, ptr %t0, align 1
  %t2 = getelementptr i8, ptr %t0, i64 8
  %v1395 = load i64, ptr %t2, align 1
  %t6 = getelementptr i8, ptr %t0, i64 24
  %v1397 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 32
  %v1398 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 40
  %v1399 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 48
  %v1400 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 56
  %v1401 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 64
  %v1402 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 72
  %v1403 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 80
  %v1404 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 88
  %v1405 = load i64, ptr %t22, align 1
  %t23.not = icmp eq i64 %v1399, 0
  br i1 %t23.not, label %block2, label %edge0

block2:                                           ; preds = %block0, %edge0
  %t26.not = icmp eq i64 %v1405, 0
  br i1 %t26.not, label %edge12, label %edge3

common.ret.loopexit21379.split.loop.exit31553:    ; preds = %edge12469
  %.mux10961.le = select i1 %27, { i64, i64 } { i64 1, i64 6583 }, { i64, i64 } { i64 1, i64 6584 }
  br label %common.ret

common.ret:                                       ; preds = %edge13768, %edge13758, %edge13750, %edge13245, %edge13237, %edge13226, %edge13222, %edge13132, %edge5300, %edge5147, %edge4971, %edge4885, %edge952, %edge940, %edge15835, %edge1063, %edge1179, %edge4539, %edge4931, %edge4917, %edge5027, %edge5146, %edge5477, %edge5422, %edge5941, %edge10954, %edge10960, %edge11244, %edge11649, %edge11830, %edge12259, %edge13603, %block6878, %edge14027, %edge14485, %edge15325, %edge4497, %edge4495, %edge4484, %edge4479, %edge4468, %edge4467, %edge4464, %edge3986, %edge1986, %edge1937, %edge1909, %edge1781, %edge1545, %edge1281, %edge4538, %edge4678, %edge9982, %edge8155, %edge11074, %edge11197, %edge12084, %edge12210, %edge12183, %edge12454, %edge12402, %edge12392, %edge4449, %edge4397, %edge4387, %edge8349, %edge8321, %edge8266, %edge8943, %edge8887, %edge8892, %edge9613, %edge10394, %edge10331, %edge10291, %block5249, %edge10144, %edge8057, %block4489, %edge9233, %edge10229, %edge432, %common.ret.loopexit21379.split.loop.exit31553, %edge6644, %block37784, %block21348, %edge3341, %block16650, %edge2400, %block13111, %edge1758, %edge477, %edge462, %edge15907, %block98481, %block98425, %block98033, %block97895, %edge15630, %block97840, %edge15587, %block97785, %edge15547, %block97730, %edge15507, %block97631, %edge15454, %block97576, %edge15411, %block97388, %edge15305, %block96878, %block96798, %edge15163, %edge15151, %block96419, %block96352, %block94676, %edge14383, %block92073, %block92006, %block91220, %edge14020, %block90323, %block89899, %block89747, %edge13614, %block89546, %block89505, %block89463, %block89427, %block89285, %block88843, %edge13147, %block86567, %block85103, %block85079, %edge12541, %block82725, %edge12223, %edge12097, %block75817, %block74956, %block74293, %edge11666, %block73578, %block73558, %edge11472, %edge11466, %edge11265, %block71146, %block69501, %edge10694, %block65825, %block65263, %block65177, %block65157, %edge10223, %block61792, %edge10039, %block61737, %edge9999, %edge9967, %edge9896, %block60045, %edge9748, %block59335, %block58814, %edge9521, %edge9516, %block57759, %edge9383, %edge9358, %edge9352, %block56955, %edge9309, %block56677, %block56596, %edge9174, %block55569, %edge9084, %block55233, %block55170, %edge8910, %edge8858, %block53921, %block53159, %block52901, %block52864, %edge8596, %block52563, %block51031, %block50799, %edge8222, %block49349, %block49314, %edge8132, %edge8103, %block47701, %edge7886, %edge7847, %block45594, %edge7744, %edge7739, %edge7722, %block44389, %edge7681, %block44334, %edge7641, %block43914, %block43677, %block43014, %edge7410, %edge7396, %edge7386, %block42344, %block41842, %block41792, %block41299, %edge7103, %edge6999, %edge6993, %block39752, %edge6946, %block39254, %block39206, %block37592, %block35396, %edge6185, %block35147, %block34141, %block33789, %block33769, %edge5398, %edge5393, %block33195, %edge5345, %block32502, %block32484, %block31630, %block31614, %edge4949, %edge4857, %block29334, %block29318, %block28483, %block28465, %block27440, %block25707, %block25477, %block25159, %block25001, %edge3827, %block23865, %block23830, %block23544, %block23082, %block23047, %edge3519, %edge3452, %edge3398, %block21228, %edge2953, %block19605, %block19570, %block19284, %block18816, %block18781, %block18347, %edge2578, %edge2511, %edge2457, %block16530, %block15607, %block15572, %block15427, %block15236, %block13495, %block13460, %block12995, %block12030, %block12020, %block11700, %edge1349, %edge1165, %edge1122, %block10882, %edge1040, %edge1034, %block10380, %edge977, %block9575, %edge839, %block9520, %edge802, %block9465, %edge766, %block9410, %edge730, %block9355, %edge694, %block9300, %edge658, %block9245, %edge622, %edge609, %block9091, %edge570, %block8855, %edge514, %block8822, %edge415, %block8608, %block8567, %block8525, %block8451, %block8306, %block8282, %edge75, %block8249
  %common.ret.op = phi { i64, i64 } [ %t7654, %block85103 ], [ %t82, %block8249 ], [ %t89, %edge75 ], [ %t149, %block8282 ], [ %t215, %block8306 ], [ %t6711, %edge10694 ], [ %t291, %block8451 ], [ %t352, %block8525 ], [ %t413, %block8567 ], [ %t477, %block8608 ], [ %t7271, %edge12097 ], [ %t491, %edge415 ], [ %t9460, %edge15907 ], [ { i64 9, i64 998 }, %edge2400 ], [ %t8965, %edge15163 ], [ { i64 1, i64 105 }, %edge462 ], [ %t585, %block8822 ], [ %t592, %edge514 ], [ zeroinitializer, %block98481 ], [ %t612, %block8855 ], [ %t625, %edge570 ], [ %t7429, %block82725 ], [ %t645, %block9091 ], [ %t656, %edge609 ], [ %t664, %edge622 ], [ %t9441, %block98425 ], [ %t684, %block9245 ], [ %t692, %edge658 ], [ %t9377, %block98033 ], [ %t712, %block9300 ], [ %t720, %edge694 ], [ %t9345, %block97895 ], [ %t740, %block9355 ], [ %t748, %edge730 ], [ %t6404, %edge9999 ], [ %t768, %block9410 ], [ %t776, %edge766 ], [ %t9325, %edge15630 ], [ %t795, %block9465 ], [ %t803, %edge802 ], [ { i64 1, i64 5728 }, %edge11074 ], [ %t823, %block9520 ], [ %t830, %edge839 ], [ %t9311, %block97840 ], [ %t850, %block9575 ], [ { i64 1, i64 117 }, %edge477 ], [ %t5847, %edge9084 ], [ %t5853, %block55569 ], [ %t8959, %edge15151 ], [ %t885, %edge977 ], [ %t9058, %block96798 ], [ %t905, %block10380 ], [ %t957, %edge1034 ], [ %t961, %edge1040 ], [ %t5873, %edge9174 ], [ %t9204, %edge15454 ], [ %t1021, %block10882 ], [ %t1031, %edge1122 ], [ %t1042, %edge1165 ], [ %t9170, %edge15411 ], [ { i64 1, i64 2288 }, %edge4495 ], [ { i64 1, i64 5845 }, %edge11197 ], [ %t1317, %edge1349 ], [ %t1344, %block11700 ], [ %t8949, %block96419 ], [ %t1359, %block12020 ], [ %t1394, %block12030 ], [ %t8936, %block96352 ], [ %t9291, %edge15587 ], [ %t6611, %block65177 ], [ %t8795, %block94676 ], [ %t7161, %block74956 ], [ %t1530, %block12995 ], [ %t6080, %block57759 ], [ { i64 9, i64 591 }, %block13111 ], [ { i64 1, i64 2535 }, %edge4917 ], [ %t8696, %edge14383 ], [ %t8689, %block92073 ], [ %t8678, %block92006 ], [ %t1612, %block13460 ], [ %t1651, %block13495 ], [ %t7139, %block74293 ], [ %t6146, %edge9516 ], [ %t6149, %edge9521 ], [ %t1744, %block15236 ], [ %t7707, %block86567 ], [ %t1792, %block15427 ], [ %t6424, %block61737 ], [ %t8560, %block91220 ], [ %t1849, %block15572 ], [ %t1888, %block15607 ], [ %t5918, %block56596 ], [ { i64 1, i64 4617 }, %edge8887 ], [ %t1965, %block16530 ], [ { i64 9, i64 3238 }, %block37784 ], [ { i64 9, i64 998 }, %block16650 ], [ %t1987, %edge2457 ], [ %t2017, %edge2511 ], [ %t2037, %edge2578 ], [ %t2082, %block18347 ], [ %t8469, %edge14020 ], [ %t8438, %block90323 ], [ %t6842, %block69501 ], [ %t7640, %block85079 ], [ %t2155, %block18781 ], [ %t2194, %block18816 ], [ %t2256, %block19284 ], [ %t9190, %block97576 ], [ %t7104, %block73578 ], [ %t5931, %block56677 ], [ %t2323, %block19570 ], [ %t2362, %block19605 ], [ %t2368, %edge2953 ], [ %t9282, %block97785 ], [ { i64 1, i64 7044 }, %edge13603 ], [ %t6498, %edge10223 ], [ { i64 1, i64 45 }, %edge432 ], [ %t2488, %block21228 ], [ { i64 1, i64 378 }, %edge1063 ], [ { i64 9, i64 1546 }, %block21348 ], [ %t2510, %edge3398 ], [ %t2540, %edge3452 ], [ %t2560, %edge3519 ], [ %t6884, %block71146 ], [ { i64 1, i64 2432 }, %edge4678 ], [ %t8298, %block89899 ], [ %t6899, %edge11265 ], [ %t2631, %block23047 ], [ %t2670, %block23082 ], [ %t2733, %block23544 ], [ %t9157, %block97388 ], [ %t8206, %block89747 ], [ %t8197, %edge13614 ], [ %t2800, %block23830 ], [ %t2839, %block23865 ], [ %t2845, %edge3827 ], [ %t9070, %block96878 ], [ { i64 1, i64 6199 }, %edge12084 ], [ { i64 1, i64 5322 }, %edge10229 ], [ %t8183, %block89546 ], [ %t8119, %block89505 ], [ %t2951, %block25001 ], [ %t2999, %block25159 ], [ %t3055, %block25477 ], [ %t3105, %block25707 ], [ %t6209, %block58814 ], [ { i64 1, i64 2385 }, %edge4539 ], [ %t8058, %block89463 ], [ %t7997, %block89427 ], [ %t3184, %block27440 ], [ { i64 1, i64 2726 }, %edge5146 ], [ { i64 1, i64 441 }, %edge1179 ], [ %t7763, %edge13147 ], [ %t6267, %block59335 ], [ %t6276, %edge9748 ], [ %t6338, %block60045 ], [ %t6365, %edge9896 ], [ %t6387, %edge9967 ], [ { i64 1, i64 4558 }, %block4489 ], [ %t3294, %block28465 ], [ %t3334, %block28483 ], [ { i64 1, i64 8041 }, %edge15325 ], [ %.mux10961.le, %common.ret.loopexit21379.split.loop.exit31553 ], [ %t3395, %block29318 ], [ %t3437, %block29334 ], [ %t3451, %edge4857 ], [ { i64 1, i64 4211 }, %edge8266 ], [ %t7124, %edge11666 ], [ { i64 1, i64 6362 }, %edge12183 ], [ %t3485, %edge4949 ], [ { i64 1, i64 7493 }, %edge14485 ], [ { i64 1, i64 4090 }, %edge8057 ], [ %t3550, %block31614 ], [ %t3592, %block31630 ], [ { i64 1, i64 2355 }, %edge4538 ], [ %t3643, %block32484 ], [ %t3683, %block32502 ], [ { i64 1, i64 6553 }, %edge12402 ], [ %t5949, %edge9309 ], [ %t7934, %block89285 ], [ %t3715, %edge5345 ], [ %t9262, %edge15547 ], [ %t3735, %block33195 ], [ %t3784, %edge5393 ], [ %t3788, %edge5398 ], [ { i64 1, i64 6122 }, %edge11830 ], [ { i64 1, i64 5266 }, %edge10144 ], [ %t3851, %block33769 ], [ %t3867, %block33789 ], [ %t3910, %block34141 ], [ %t5969, %block56955 ], [ %t6571, %block65157 ], [ %t7858, %block88843 ], [ { i64 1, i64 5169 }, %edge9982 ], [ %t4085, %block35147 ], [ %t9253, %block97730 ], [ %t7088, %block73558 ], [ %t4223, %edge6185 ], [ %t4238, %block35396 ], [ %t4344, %block37592 ], [ %t7540, %edge12541 ], [ %t4426, %block39206 ], [ %t4447, %block39254 ], [ %t6010, %edge9352 ], [ %t4464, %edge6946 ], [ %t9102, %edge15305 ], [ %t4484, %block39752 ], [ %t4546, %edge6993 ], [ %t4553, %edge6999 ], [ %t4604, %edge7103 ], [ %t4651, %block41299 ], [ %t4705, %block41792 ], [ %t4724, %block41842 ], [ %t4763, %block42344 ], [ %t4769, %edge7386 ], [ %t4775, %edge7396 ], [ %t4781, %edge7410 ], [ %t4796, %block43014 ], [ %t4809, %block43677 ], [ %t4819, %block43914 ], [ %t6017, %edge9358 ], [ %t4841, %edge7641 ], [ %t9233, %edge15507 ], [ %t4861, %block44334 ], [ %t7011, %edge11466 ], [ %t4870, %edge7681 ], [ %t7331, %edge12223 ], [ %t4890, %block44389 ], [ %t4901, %edge7722 ], [ %t4947, %edge7739 ], [ %t4952, %edge7744 ], [ %t5003, %block45594 ], [ { i64 9, i64 3238 }, %edge6644 ], [ %t5016, %edge7847 ], [ %t5050, %edge7886 ], [ %t6027, %edge9383 ], [ %t5118, %block47701 ], [ { i64 9, i64 1546 }, %edge3341 ], [ %t5137, %edge8103 ], [ %t7015, %edge11472 ], [ %t5154, %edge8132 ], [ { i64 1, i64 7256 }, %edge14027 ], [ { i64 1, i64 4754 }, %edge9233 ], [ %t5211, %block49314 ], [ %t5250, %block49349 ], [ %t5286, %edge8222 ], [ { i64 1, i64 2643 }, %edge5027 ], [ { i64 9, i64 591 }, %edge1758 ], [ %t6645, %block65263 ], [ %t5369, %block50799 ], [ %t5410, %block51031 ], [ %t6683, %block65825 ], [ %t5470, %block52563 ], [ %t7191, %block75817 ], [ %t6433, %edge10039 ], [ %t5484, %edge8596 ], [ %t9224, %block97631 ], [ %t5504, %block52864 ], [ %t5545, %block52901 ], [ %t5592, %block53159 ], [ { i64 1, i64 2837 }, %edge5422 ], [ %t5649, %block53921 ], [ { i64 1, i64 2215 }, %edge4387 ], [ %t5670, %edge8858 ], [ %t6453, %block61792 ], [ %t5712, %edge8910 ], [ %t5787, %block55233 ], [ %t5763, %block55170 ], [ { i64 1, i64 4950 }, %edge9613 ], [ { i64 1, i64 5433 }, %edge10394 ], [ { i64 1, i64 5416 }, %edge10331 ], [ { i64 1, i64 5391 }, %edge10291 ], [ { i64 1, i64 5334 }, %block5249 ], [ { i64 1, i64 4617 }, %edge8892 ], [ { i64 1, i64 4641 }, %edge8943 ], [ { i64 1, i64 4286 }, %edge8349 ], [ { i64 1, i64 4255 }, %edge8321 ], [ { i64 1, i64 2260 }, %edge4449 ], [ { i64 1, i64 2238 }, %edge4397 ], [ { i64 1, i64 6575 }, %edge12454 ], [ { i64 1, i64 6530 }, %edge12392 ], [ { i64 1, i64 6377 }, %edge12210 ], [ { i64 103425, i64 4166 }, %edge8155 ], [ { i64 1, i64 2280 }, %edge4479 ], [ { i64 1, i64 2282 }, %edge4484 ], [ { i64 1, i64 2289 }, %edge4497 ], [ %t1249, %edge1281 ], [ { i64 1, i64 586 }, %edge1545 ], [ { i64 1, i64 626 }, %edge1781 ], [ { i64 1, i64 709 }, %edge1909 ], [ { i64 1, i64 756 }, %edge1937 ], [ { i64 1, i64 824 }, %edge1986 ], [ { i64 103425, i64 1948 }, %edge3986 ], [ { i64 1, i64 2268 }, %edge4464 ], [ { i64 1, i64 2269 }, %edge4467 ], [ { i64 1, i64 2271 }, %edge4468 ], [ { i64 1, i64 7005 }, %block6878 ], [ { i64 1, i64 5621 }, %edge10960 ], [ { i64 1, i64 6001 }, %edge11649 ], [ { i64 1, i64 5620 }, %edge10954 ], [ { i64 103427, i64 3000 }, %edge5941 ], [ { i64 1, i64 6418 }, %edge12259 ], [ { i64 1, i64 5865 }, %edge11244 ], [ { i64 1, i64 2879 }, %edge5477 ], [ { i64 1, i64 2559 }, %edge4931 ], [ { i64 1, i64 7096 }, %edge13750 ], [ { i64 1, i64 2756 }, %edge5147 ], [ { i64 1, i64 329 }, %edge952 ], [ { i64 1, i64 2518 }, %edge4885 ], [ { i64 1, i64 6934 }, %edge13222 ], [ { i64 1, i64 2585 }, %edge4971 ], [ { i64 1, i64 6941 }, %edge13226 ], [ { i64 1, i64 319 }, %edge940 ], [ { i64 1, i64 2782 }, %edge5300 ], [ { i64 1, i64 310 }, %edge15835 ], [ { i64 1, i64 7102 }, %edge13758 ], [ { i64 1, i64 7108 }, %edge13768 ], [ { i64 1, i64 6848 }, %edge13132 ], [ { i64 1, i64 6944 }, %edge13237 ], [ { i64 1, i64 6945 }, %edge13245 ]
  ret { i64, i64 } %common.ret.op

block8252:                                        ; preds = %edge49
  %v1861 = load i64, ptr %ss1, align 8
  %t71 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %v1862 = load i64, ptr %t71, align 8
  %t72 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %v1863 = load i64, ptr %t72, align 8
  %t73 = icmp eq i64 %v1405, %v1863
  %t75.not267 = icmp eq i64 %v1863, 0
  %t75.not = or i1 %t73, %t75.not267
  br i1 %t75.not, label %block8258, label %edge59

block8258:                                        ; preds = %block8252, %edge59
  %t79.not = or i1 %t26.not, %t73
  br i1 %t79.not, label %edge68, label %edge64

block8249:                                        ; preds = %edge64, %edge49, %edge38
  %v1829 = phi i64 [ %v1860, %edge49 ], [ 103427, %edge38 ], [ %v1872, %edge64 ]
  %t81 = insertvalue { i64, i64 } poison, i64 %v1829, 0
  %t82 = insertvalue { i64, i64 } %t81, i64 17, 1
  br label %common.ret

block8289:                                        ; preds = %edge85, %edge89, %edge98
  %v1923 = phi i64 [ %v1928, %edge89 ], [ %v1945, %edge98 ], [ 64, %edge85 ]
  %t116 = call i64 @llvm.umax.i64(i64 %v1923, i64 %v1917)
  br label %block8286

block8286:                                        ; preds = %edge82, %block8289
  %v1922 = phi i64 [ %t116, %block8289 ], [ %v1841336348, %edge82 ]
  br i1 %t75.not267339346, label %block8297, label %edge102

block8297:                                        ; preds = %edge102, %block8286
  %v1957 = ptrtoint ptr %ss1 to i64
  %v218055 = ptrtoint ptr %ss0 to i64
  %t122 = zext i64 %v1839335349 to i128
  %t123 = zext i64 %v1401 to i128
  %t124 = shl nuw i128 %t123, 64
  %v218056 = or disjoint i128 %t124, %t122
  store i128 %v218056, ptr %ss0, align 16
  %t126 = zext i64 %v1841336348 to i128
  %v218058 = or disjoint i128 %t126, 18446744073709551616
  %t130 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  store i128 %v218058, ptr %t130, align 16
  %t131 = zext i64 %v1404 to i128
  %t132 = zext i64 %v1844337347 to i128
  %t133 = shl nuw i128 %t132, 64
  %v218060 = or disjoint i128 %t133, %t131
  %t135 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  store i128 %v218060, ptr %t135, align 16
  %v1962 = call i64 @mech_compact_writable(i64 %v0, i64 %v218055, i64 1, i64 %v1922, i64 1, i64 1, i64 %v1957)
  %t136.not = icmp eq i64 %v1962, 0
  br i1 %t136.not, label %edge107, label %block8282

block8296:                                        ; preds = %edge102, %edge107
  %v1947 = phi i64 [ %v1963, %edge107 ], [ %v1839335349, %edge102 ]
  %v1949 = phi i64 [ %v1964, %edge107 ], [ %v1841336348, %edge102 ]
  %v1952 = phi i64 [ %v1965, %edge107 ], [ %v1844337347, %edge102 ]
  %v1967 = add i64 %v1947, %v1401
  %t139 = inttoptr i64 %v1967 to ptr
  store i8 1, ptr %t139, align 1
  %t140 = icmp eq i64 %v1844337347, %v1952
  %t142.not287 = icmp eq i64 %v1952, 0
  %t142.not = or i1 %t140, %t142.not287
  br i1 %t142.not, label %block8301, label %edge109

block8301:                                        ; preds = %block8296, %edge109
  %t146.not = or i1 %t75.not267339346, %t140
  br i1 %t146.not, label %edge118, label %edge114

block8282:                                        ; preds = %edge114, %block8297, %edge89, %edge81
  %v1911 = phi i64 [ %v1962, %block8297 ], [ 1, %edge81 ], [ 1, %edge89 ], [ %v1978, %edge114 ]
  %t148 = insertvalue { i64, i64 } poison, i64 %v1911, 0
  %t149 = insertvalue { i64, i64 } %t148, i64 23, 1
  br label %common.ret

block8314:                                        ; preds = %edge124, %edge128, %edge137
  %v1993 = phi i64 [ %v1998, %edge128 ], [ %v2015, %edge137 ], [ 64, %edge124 ]
  %t176 = call i64 @llvm.umax.i64(i64 %v1993, i64 %v1987)
  br label %block8311

block8311:                                        ; preds = %edge121, %block8314
  %v1992 = phi i64 [ %t176, %block8314 ], [ %v1949, %edge121 ]
  br i1 %t142.not287, label %block8322, label %edge141

block8322:                                        ; preds = %edge141, %block8311
  %v2027 = ptrtoint ptr %ss1 to i64
  %v218061 = ptrtoint ptr %ss0 to i64
  %t182 = zext i64 %v1947 to i128
  %t183 = zext i64 %v1917 to i128
  %t184 = shl nuw i128 %t183, 64
  %v218062 = or disjoint i128 %t184, %t182
  store i128 %v218062, ptr %ss0, align 16
  %t186 = zext i64 %v1949 to i128
  %v218064 = or disjoint i128 %t186, 18446744073709551616
  %t190 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  store i128 %v218064, ptr %t190, align 16
  %t192 = zext i64 %v1952 to i128
  %t193 = shl nuw i128 %t192, 64
  %v218066 = or disjoint i128 %t193, %t183
  %t195 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  store i128 %v218066, ptr %t195, align 16
  %v2032 = call i64 @mech_compact_writable(i64 %v0, i64 %v218061, i64 1, i64 %v1992, i64 1, i64 1, i64 %v2027)
  %t196.not = icmp eq i64 %v2032, 0
  br i1 %t196.not, label %edge146, label %block8306

block8342:                                        ; preds = %edge148, %edge160
  %t209.not = or i1 %t142.not287, %t203
  br i1 %t209.not, label %edge15901, label %edge163

block8306:                                        ; preds = %edge166, %block8322, %edge128, %edge118
  %v1980 = phi i64 [ %v2032, %block8322 ], [ 1, %edge118 ], [ 1, %edge128 ], [ %v2084, %edge166 ]
  %t214 = insertvalue { i64, i64 } poison, i64 %v1980, 0
  %t215 = insertvalue { i64, i64 } %t214, i64 24, 1
  br label %common.ret

block8408:                                        ; preds = %block8408.preheader, %edge432
  %v217687 = phi i64 [ %v2793, %edge432 ], [ %v1839335349, %block8408.preheader ]
  %v217688 = phi i64 [ %v2763, %edge432 ], [ %v1401, %block8408.preheader ]
  %v217689 = phi i64 [ %v2795, %edge432 ], [ %v1841336348, %block8408.preheader ]
  %v217691 = phi i64 [ %v2763, %edge432 ], [ %v1404, %block8408.preheader ]
  %v217692 = phi i64 [ %v2798, %edge432 ], [ %v1844337347, %block8408.preheader ]
  %v217693 = phi i64 [ %v2831, %edge432 ], [ 0, %block8408.preheader ]
  %v2208 = sub nsw i64 %v1395, %v217693
  %t220 = icmp ult i64 %v2208, 65535
  %spec.select = zext i1 %t220 to i8
  %spec.select293 = call i64 @llvm.umin.i64(i64 %v2208, i64 65535)
  %t229 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v217688, i64 1)
  %v2340 = extractvalue { i64, i1 } %t229, 0
  %t230 = extractvalue { i64, i1 } %t229, 1
  br i1 %t230, label %block8451, label %edge218

block8458:                                        ; preds = %edge221, %edge225, %edge234
  %v2346 = phi i64 [ %v2351, %edge225 ], [ %v2368, %edge234 ], [ 64, %edge221 ]
  %t255 = call i64 @llvm.umax.i64(i64 %v2346, i64 %v2340)
  br label %block8455

block8455:                                        ; preds = %edge218, %block8458
  %v2345 = phi i64 [ %t255, %block8458 ], [ %v217689, %edge218 ]
  %t256.not = icmp eq i64 %v217692, 0
  br i1 %t256.not, label %block8466, label %edge238

block8466:                                        ; preds = %edge238, %block8455
  %t261 = zext i64 %v217687 to i128
  %t262 = zext i64 %v217688 to i128
  %t263 = shl nuw i128 %t262, 64
  %v218068 = or disjoint i128 %t263, %t261
  store i128 %v218068, ptr %ss0, align 16
  %t265 = zext i64 %v217689 to i128
  %v218070 = or disjoint i128 %t265, 18446744073709551616
  store i128 %v218070, ptr %t269, align 16
  %t270 = zext i64 %v217691 to i128
  %t271 = zext i64 %v217692 to i128
  %t272 = shl nuw i128 %t271, 64
  %v218072 = or disjoint i128 %t272, %t270
  store i128 %v218072, ptr %t274, align 16
  %v2385 = call i64 @mech_compact_writable(i64 %v0, i64 %v218067, i64 1, i64 %v2345, i64 1, i64 1, i64 %v2380)
  %t275.not = icmp eq i64 %v2385, 0
  br i1 %t275.not, label %edge243, label %block8451

block8465:                                        ; preds = %edge238, %edge243
  %v2370 = phi i64 [ %v2386, %edge243 ], [ %v217687, %edge238 ]
  %v2372 = phi i64 [ %v2387, %edge243 ], [ %v217689, %edge238 ]
  %v2375 = phi i64 [ %v2388, %edge243 ], [ %v217692, %edge238 ]
  %v2390 = add i64 %v2370, %v217688
  %t278 = inttoptr i64 %v2390 to ptr
  store i8 %spec.select, ptr %t278, align 1
  %t279 = icmp eq i64 %v217692, %v2375
  %t281.not271 = icmp eq i64 %v2375, 0
  %t281.not = or i1 %t279, %t281.not271
  br i1 %t281.not, label %block8470, label %edge245

block8470:                                        ; preds = %block8465, %edge245
  %t285.not = or i1 %t256.not, %t279
  br i1 %t285.not, label %edge262, label %edge248

block8451:                                        ; preds = %edge251, %block8466, %edge225, %block8408
  %v2334 = phi i64 [ %v2385, %block8466 ], [ %v2401, %edge251 ], [ 1, %block8408 ], [ 1, %edge225 ]
  %t290 = insertvalue { i64, i64 } poison, i64 %v2334, 0
  %t291 = insertvalue { i64, i64 } %t290, i64 69, 1
  br label %common.ret

block8533:                                        ; preds = %edge268, %edge272, %edge281
  %v2547 = phi i64 [ %v2552, %edge272 ], [ %v2569, %edge281 ], [ 64, %edge268 ]
  %t318 = call i64 @llvm.umax.i64(i64 %v2547, i64 %v2541)
  br label %block8530

block8530:                                        ; preds = %edge265, %block8533
  %v2546 = phi i64 [ %t318, %block8533 ], [ %v2372, %edge265 ]
  br i1 %t281.not271, label %block8541, label %edge285

block8541:                                        ; preds = %edge285, %block8530
  %t324 = zext i64 %v2370 to i128
  %t325 = zext i64 %v2340 to i128
  %t326 = shl nuw i128 %t325, 64
  %v218074 = or disjoint i128 %t326, %t324
  store i128 %v218074, ptr %ss0, align 16
  %t328 = zext i64 %v2372 to i128
  %v218076 = or disjoint i128 %t328, 18446744073709551616
  store i128 %v218076, ptr %t269, align 16
  %t334 = zext i64 %v2375 to i128
  %t335 = shl nuw i128 %t334, 64
  %v218078 = or disjoint i128 %t335, %t325
  store i128 %v218078, ptr %t274, align 16
  %v2586 = call i64 @mech_compact_writable(i64 %v0, i64 %v218067, i64 1, i64 %v2546, i64 1, i64 1, i64 %v2380)
  %t338.not = icmp eq i64 %v2586, 0
  br i1 %t338.not, label %edge290, label %block8525

block8561:                                        ; preds = %edge292, %edge302
  %t349.not = or i1 %t281.not271, %t343
  br i1 %t349.not, label %edge311, label %edge307

block8525:                                        ; preds = %edge307, %block8541, %edge272, %edge262
  %v2528 = phi i64 [ %v2586, %block8541 ], [ 1, %edge262 ], [ 1, %edge272 ], [ %v2638, %edge307 ]
  %t351 = insertvalue { i64, i64 } poison, i64 %v2528, 0
  %t352 = insertvalue { i64, i64 } %t351, i64 80, 1
  br label %common.ret

block8575:                                        ; preds = %edge318, %edge322, %edge331
  %v2660 = phi i64 [ %v2665, %edge322 ], [ %v2682, %edge331 ], [ 64, %edge318 ]
  %t379 = call i64 @llvm.umax.i64(i64 %v2660, i64 %v2654)
  br label %block8572

block8572:                                        ; preds = %edge315, %block8575
  %v2659 = phi i64 [ %t379, %block8575 ], [ %v2573, %edge315 ]
  br i1 %t345.not275, label %block8583, label %edge335

block8583:                                        ; preds = %edge335, %block8572
  %t385 = zext i64 %v2571 to i128
  %t386 = zext i64 %v2541 to i128
  %t387 = shl nuw i128 %t386, 64
  %v218080 = or disjoint i128 %t387, %t385
  store i128 %v218080, ptr %ss0, align 16
  %t389 = zext i64 %v2573 to i128
  %v218082 = or disjoint i128 %t389, 18446744073709551616
  store i128 %v218082, ptr %t269, align 16
  %t395 = zext i64 %v2576 to i128
  %t396 = shl nuw i128 %t395, 64
  %v218084 = or disjoint i128 %t396, %t386
  store i128 %v218084, ptr %t274, align 16
  %v2699 = call i64 @mech_compact_writable(i64 %v0, i64 %v218067, i64 1, i64 %v2659, i64 1, i64 1, i64 %v2380)
  %t399.not = icmp eq i64 %v2699, 0
  br i1 %t399.not, label %edge340, label %block8567

block8603:                                        ; preds = %edge342, %edge352
  %t410.not = or i1 %t345.not275, %t404
  br i1 %t410.not, label %edge361, label %edge357

block8567:                                        ; preds = %edge357, %block8583, %edge322, %edge311
  %v2647 = phi i64 [ %v2699, %block8583 ], [ 1, %edge311 ], [ 1, %edge322 ], [ %v2751, %edge357 ]
  %t412 = insertvalue { i64, i64 } poison, i64 %v2647, 0
  %t413 = insertvalue { i64, i64 } %t412, i64 82, 1
  br label %common.ret

block8616:                                        ; preds = %edge368, %edge372, %edge381
  %v2769 = phi i64 [ %v2774, %edge372 ], [ %v2791, %edge381 ], [ 64, %edge368 ]
  %t444 = call i64 @llvm.umax.i64(i64 %v2769, i64 %v2763)
  br label %block8613

block8613:                                        ; preds = %edge365, %block8616
  %v2768 = phi i64 [ %t444, %block8616 ], [ %v2686, %edge365 ]
  br i1 %t406.not279, label %block8624, label %edge385

block8624:                                        ; preds = %edge385, %block8613
  %t450 = zext i64 %v2684 to i128
  %t451 = zext i64 %v2654 to i128
  %t452 = shl nuw i128 %t451, 64
  %v218086 = or disjoint i128 %t452, %t450
  store i128 %v218086, ptr %ss0, align 16
  %t454 = zext i64 %v2686 to i128
  %v218088 = or disjoint i128 %t454, 18446744073709551616
  store i128 %v218088, ptr %t269, align 16
  %t460 = zext i64 %v2689 to i128
  %t461 = shl nuw i128 %t460, 64
  %v218090 = or disjoint i128 %t461, %t451
  store i128 %v218090, ptr %t274, align 16
  %v2808 = call i64 @mech_compact_writable(i64 %v0, i64 %v218067, i64 1, i64 %v2768, i64 1, i64 1, i64 %v2380)
  %t464.not = icmp eq i64 %v2808, 0
  br i1 %t464.not, label %edge390, label %block8608

block8623:                                        ; preds = %edge385, %edge390
  %v2793 = phi i64 [ %v2809, %edge390 ], [ %v2684, %edge385 ]
  %v2795 = phi i64 [ %v2810, %edge390 ], [ %v2686, %edge385 ]
  %v2798 = phi i64 [ %v2811, %edge390 ], [ %v2689, %edge385 ]
  %t467.not = icmp eq i64 %t417, 0
  br i1 %t467.not, label %block8628, label %edge392

block8628:                                        ; preds = %block8623, %edge392
  %t468 = icmp eq i64 %v2689, %v2798
  %t470.not283 = icmp eq i64 %v2798, 0
  %t470.not = or i1 %t468, %t470.not283
  br i1 %t470.not, label %block8630, label %edge395

block8630:                                        ; preds = %block8628, %edge395
  %t474.not = or i1 %t406.not279, %t468
  br i1 %t474.not, label %edge404, label %edge400

block8608:                                        ; preds = %edge400, %block8624, %edge372, %edge363, %edge361
  %v2753 = phi i64 [ %v2808, %block8624 ], [ 103427, %edge361 ], [ 1, %edge363 ], [ 1, %edge372 ], [ %v2826, %edge400 ]
  %t476 = insertvalue { i64, i64 } poison, i64 %v2753, 0
  %t477 = insertvalue { i64, i64 } %t476, i64 83, 1
  br label %common.ret

block8825:                                        ; preds = %edge485
  %v3198 = load i64, ptr %ss1, align 8
  %t571 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %v3199 = load i64, ptr %t571, align 8
  %t572 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %v3200 = load i64, ptr %t572, align 8
  %t573 = icmp eq i64 %v1405, %v3200
  %t575.not44 = icmp eq i64 %v3200, 0
  %t575.not = or i1 %t573, %t575.not44
  br i1 %t575.not, label %block8831, label %edge495

block8831:                                        ; preds = %block8825, %edge495
  %t579.not = or i1 %t26.not, %t573
  br i1 %t579.not, label %edge534, label %edge498

block8822:                                        ; preds = %edge501, %edge485, %edge482
  %v3166 = phi i64 [ %v3197, %edge485 ], [ 103427, %edge482 ], [ %v3209, %edge501 ]
  %t584 = insertvalue { i64, i64 } poison, i64 %v3166, 0
  %t585 = insertvalue { i64, i64 } %t584, i64 118, 1
  br label %common.ret

block8855:                                        ; preds = %edge534
  %t611 = insertvalue { i64, i64 } poison, i64 %v3274, 0
  %t612 = insertvalue { i64, i64 } %t611, i64 124, 1
  br label %common.ret

block9091:                                        ; preds = %edge517
  %t644 = insertvalue { i64, i64 } poison, i64 %v3758, 0
  %t645 = insertvalue { i64, i64 } %t644, i64 156, 1
  br label %common.ret

block9245:                                        ; preds = %edge642
  %t683 = insertvalue { i64, i64 } poison, i64 %v4146, 0
  %t684 = insertvalue { i64, i64 } %t683, i64 180, 1
  br label %common.ret

block9300:                                        ; preds = %edge678
  %t711 = insertvalue { i64, i64 } poison, i64 %v4365, 0
  %t712 = insertvalue { i64, i64 } %t711, i64 193, 1
  br label %common.ret

block9355:                                        ; preds = %edge714
  %t739 = insertvalue { i64, i64 } poison, i64 %v4604, 0
  %t740 = insertvalue { i64, i64 } %t739, i64 206, 1
  br label %common.ret

block9410:                                        ; preds = %edge750
  %t767 = insertvalue { i64, i64 } poison, i64 %v4847, 0
  %t768 = insertvalue { i64, i64 } %t767, i64 219, 1
  br label %common.ret

block9465:                                        ; preds = %edge786
  %t794 = insertvalue { i64, i64 } poison, i64 %v5114, 0
  %t795 = insertvalue { i64, i64 } %t794, i64 232, 1
  br label %common.ret

block9520:                                        ; preds = %edge822
  %t822 = insertvalue { i64, i64 } poison, i64 %v5381, 0
  %t823 = insertvalue { i64, i64 } %t822, i64 245, 1
  br label %common.ret

block9575:                                        ; preds = %edge859
  %t849 = insertvalue { i64, i64 } poison, i64 %v5660, 0
  %t850 = insertvalue { i64, i64 } %t849, i64 258, 1
  br label %common.ret

block9980:                                        ; preds = %edge15835, %edge842
  %v217705 = phi i64 [ %v3176356367, %edge842 ], [ %v204936, %edge15835 ]
  %v217706 = phi i64 [ %v1401, %edge842 ], [ %v204950, %edge15835 ]
  %v217707 = phi i64 [ %v3178357366, %edge842 ], [ %v204964, %edge15835 ]
  %v217709 = phi i64 [ %v1404, %edge842 ], [ %v204950, %edge15835 ]
  %v217710 = phi i64 [ %v3181358365, %edge842 ], [ %v205006, %edge15835 ]
  %v217711 = phi i64 [ 0, %edge842 ], [ %v205026, %edge15835 ]
  %v217712 = phi i64 [ 0, %edge842 ], [ %v205042, %edge15835 ]
  %v217713 = phi i64 [ %v3277, %edge842 ], [ %v54913, %edge15835 ]
  %v217715 = phi i64 [ %v3278, %edge842 ], [ %v54915, %edge15835 ]
  %v217716 = phi i64 [ %v3279, %edge842 ], [ %v54918, %edge15835 ]
  %v217717 = phi i64 [ %v3761, %edge842 ], [ %v203700, %edge15835 ]
  %v217718 = phi i64 [ %v3762, %edge842 ], [ %v203702, %edge15835 ]
  %v217719 = phi i64 [ %v3763, %edge842 ], [ %v203705, %edge15835 ]
  %v217720 = phi i64 [ %v4049, %edge842 ], [ %v7457721129, %edge15835 ]
  %v217721 = phi i64 [ 0, %edge842 ], [ %v7457821133, %edge15835 ]
  %v217722 = phi i64 [ %v4050, %edge842 ], [ %v7457921136, %edge15835 ]
  %v217723 = phi i64 [ 1, %edge842 ], [ %v7458021140, %edge15835 ]
  %v217724 = phi i64 [ 0, %edge842 ], [ %v7458121144, %edge15835 ]
  %v217725 = phi i64 [ %v4051, %edge842 ], [ %v7458221148, %edge15835 ]
  %v217726 = phi i64 [ 0, %edge842 ], [ %v54925, %edge15835 ]
  %v217727 = phi i64 [ 0, %edge842 ], [ %v54926, %edge15835 ]
  %v217728 = phi i64 [ 0, %edge842 ], [ %v54927, %edge15835 ]
  %v217729 = phi i64 [ 0, %edge842 ], [ %v204073, %edge15835 ]
  %v217730 = phi i64 [ 0, %edge842 ], [ %v54929, %edge15835 ]
  %v217731 = phi i64 [ %spec.select9, %edge842 ], [ %v54930, %edge15835 ]
  %v217732 = phi i64 [ 0, %edge842 ], [ %v204266, %edge15835 ]
  %v217733 = phi i64 [ %v4149, %edge842 ], [ %v217579, %edge15835 ]
  %v217735 = phi i64 [ %v4150, %edge842 ], [ %v217581, %edge15835 ]
  %v217738 = phi i64 [ %v4151, %edge842 ], [ %v217631, %edge15835 ]
  %v217739 = phi i64 [ %v4368, %edge842 ], [ %v144765, %edge15835 ]
  %v217741 = phi i64 [ %v4369, %edge842 ], [ %v144771, %edge15835 ]
  %v217744 = phi i64 [ %v4370, %edge842 ], [ %v138168, %edge15835 ]
  %v217745 = phi i64 [ %v4607, %edge842 ], [ %v204269, %edge15835 ]
  %v217746 = phi i64 [ %v4608, %edge842 ], [ %v204271, %edge15835 ]
  %v217748 = phi i64 [ %v4609, %edge842 ], [ %v204274, %edge15835 ]
  %v217749 = phi i64 [ %v4850, %edge842 ], [ %v204281, %edge15835 ]
  %v217750 = phi i64 [ %v4851, %edge842 ], [ %v204283, %edge15835 ]
  %v217752 = phi i64 [ %v4852, %edge842 ], [ %v204286, %edge15835 ]
  %v217753 = phi i64 [ %v5117, %edge842 ], [ %v54943, %edge15835 ]
  %v217754 = phi i64 [ %v5119, %edge842 ], [ %v54948, %edge15835 ]
  %v217755 = phi i64 [ 0, %edge842 ], [ %v54955, %edge15835 ]
  %v217756 = phi i64 [ %v5663, %edge842 ], [ %v204874, %edge15835 ]
  %v217757 = phi i64 [ %v5664, %edge842 ], [ %v204876, %edge15835 ]
  %v217759 = phi i64 [ %v5665, %edge842 ], [ %v204879, %edge15835 ]
  %v217760 = phi i64 [ %v5384, %edge842 ], [ %v204886, %edge15835 ]
  %v217761 = phi i64 [ %v5385, %edge842 ], [ %v204888, %edge15835 ]
  %v217763 = phi i64 [ %v5386, %edge842 ], [ %v204891, %edge15835 ]
  %v217764 = phi i64 [ 0, %edge842 ], [ %v204901, %edge15835 ]
  %v217765 = phi i1 [ false, %edge842 ], [ %t3962, %edge15835 ]
  %v217768 = phi i64 [ 0, %edge842 ], [ %v208852, %edge15835 ]
  %v217769 = phi i64 [ 0, %edge842 ], [ %v208952, %edge15835 ]
  %v7232 = sub i64 %v1395, %v217729
  %t858 = icmp ult i64 %v7232, 305000
  br i1 %t858, label %edge936, label %edge940

block10380:                                       ; preds = %edge997
  %t904 = insertvalue { i64, i64 } poison, i64 %v8088, 0
  %t905 = insertvalue { i64, i64 } %t904, i64 343, 1
  br label %common.ret

block364:                                         ; preds = %block364.preheader, %edge1128
  %v8474 = phi i64 [ %v8999, %edge1128 ], [ %v8093, %block364.preheader ]
  %v8508 = phi i64 [ %v8996, %edge1128 ], [ %v8092, %block364.preheader ]
  %v8511 = phi i64 [ %v9254, %edge1128 ], [ 0, %block364.preheader ]
  %v8512 = phi i64 [ %v8994, %edge1128 ], [ %v8091, %block364.preheader ]
  %t912.not = icmp eq i64 %v8474, 0
  br i1 %t912.not, label %edge1057, label %edge1023

block10568:                                       ; preds = %edge1031, %edge1030
  %t951.pn = phi { i64, i64 } [ %t951, %edge1030 ], [ %t952, %edge1031 ]
  %v8480 = extractvalue { i64, i64 } %t951.pn, 0
  %t953.not = icmp eq i64 %v8480, 0
  br i1 %t953.not, label %edge1036, label %edge1034

trap976:                                          ; preds = %edge1078
  call void @llvm.trap()
  unreachable

ok976:                                            ; preds = %edge1078
  %v8947 = udiv i64 %v8745, %v1397
  %v8948 = mul i64 %v8947, %v1397
  %v8949 = sub nuw i64 %v8745, %v8948
  %v8950 = mul i64 %v8949, %v1398
  %v8951 = add i64 %v8950, %v8947
  br label %block10874

block10874:                                       ; preds = %edge1073, %ok976
  %v8946 = phi i64 [ %v8951, %ok976 ], [ %v8745, %edge1073 ]
  %v8953 = add i64 %v8946, %v1394
  %t977 = inttoptr i64 %v8953 to ptr
  %t978 = load i8, ptr %t977, align 1
  %v8954 = zext i8 %t978 to i64
  br label %edge1088

block10887:                                       ; preds = %edge1092, %edge1091
  %t994 = zext i64 %v8512 to i128
  %v218116 = or disjoint i128 %t994, 4722366482869645213696
  store i128 %v218116, ptr %ss0, align 16
  %t998 = zext i64 %v8508 to i128
  %v218118 = or disjoint i128 %t998, 18446744073709551616
  store i128 %v218118, ptr %t1002, align 16
  %t1004 = zext i64 %v8474 to i128
  %t1005 = shl nuw i128 %t1004, 64
  %v218120 = or disjoint i128 %t1005, 256
  store i128 %v218120, ptr %t1007, align 16
  %v9015 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v8508, i64 1, i64 1, i64 %v3192)
  %t1008.not = icmp eq i64 %v9015, 0
  br i1 %t1008.not, label %edge1097, label %block10882

block10886:                                       ; preds = %edge1092, %edge1097
  %t1011.pre-phi = phi ptr [ %t985, %edge1092 ], [ %.pre20756, %edge1097 ]
  %v9000 = phi i64 [ %v8512, %edge1092 ], [ %v9016, %edge1097 ]
  %v9002 = phi i64 [ %v8508, %edge1092 ], [ %v9017, %edge1097 ]
  %v9005 = phi i64 [ %v8474, %edge1092 ], [ %v9018, %edge1097 ]
  store i8 1, ptr %t1011.pre-phi, align 1
  br label %block10885

block10885:                                       ; preds = %edge1088, %block10886
  %v8994 = phi i64 [ %v9000, %block10886 ], [ %v8512, %edge1088 ]
  %v8996 = phi i64 [ %v9002, %block10886 ], [ %v8508, %edge1088 ]
  %v8999 = phi i64 [ %v9005, %block10886 ], [ %v8474, %edge1088 ]
  %t1012 = icmp eq i64 %v8474, %v8999
  %t1014.not47 = icmp eq i64 %v8999, 0
  %t1014.not = or i1 %t1012, %t1014.not47
  br i1 %t1014.not, label %block10891, label %edge1100

block10891:                                       ; preds = %block10885, %edge1100
  %t1018.not = or i1 %t912.not, %t1012
  br i1 %t1018.not, label %edge1128, label %edge1105

block10882:                                       ; preds = %edge1105, %block10887
  %v8982 = phi i64 [ %v9015, %block10887 ], [ %v9029, %edge1105 ]
  %t1020 = insertvalue { i64, i64 } poison, i64 %v8982, 0
  %t1021 = insertvalue { i64, i64 } %t1020, i64 393, 1
  br label %common.ret

block475:                                         ; preds = %block475.backedge, %block475.outer
  %v10787 = phi i64 [ %v10787.ph, %block475.outer ], [ %v36628651, %block475.backedge ]
  %v10788 = phi i64 [ %v10788.ph, %block475.outer ], [ %v4033311241137, %block475.backedge ]
  %v10790 = phi i64 [ %v10790.ph, %block475.outer ], [ %v40035, %block475.backedge ]
  %v10792 = phi i64 [ %v10792.ph, %block475.outer ], [ %v40047, %block475.backedge ]
  %v10794 = phi i64 [ %v10794.ph, %block475.outer ], [ %v36625650, %block475.backedge ]
  %v10796 = phi i64 [ %v10796.ph, %block475.outer ], [ %v4033011221138, %block475.backedge ]
  %v10800 = phi i64 [ %v10800.ph, %block475.outer ], [ %v40032, %block475.backedge ]
  %v10804 = phi i64 [ %v10804.ph, %block475.outer ], [ %v40044, %block475.backedge ]
  %v10807 = phi i64 [ %v10807.ph, %block475.outer ], [ %v36647656, %block475.backedge ]
  %v10808 = phi i64 [ %v10808.ph, %block475.outer ], [ %v36650657, %block475.backedge ]
  %v10809 = phi i64 [ %v10809.ph, %block475.outer ], [ %v41283, %block475.backedge ]
  %v10810 = phi i64 [ %v10810.ph, %block475.outer ], [ %spec.select1, %block475.backedge ]
  %v10811 = phi i64 [ %v10811.ph, %block475.outer ], [ %v41333, %block475.backedge ]
  %v10812 = phi i64 [ %v10812.ph, %block475.outer ], [ %v40314, %block475.backedge ]
  %v10813 = phi i64 [ %v10813.ph, %block475.outer ], [ %v217321, %block475.backedge ]
  %v10814 = phi i64 [ %v10814.ph, %block475.outer ], [ %v40054, %block475.backedge ]
  %v10815 = phi i64 [ %v10815.ph, %block475.outer ], [ %v40057, %block475.backedge ]
  %v10816 = phi i64 [ %v10816.ph, %block475.outer ], [ 0, %block475.backedge ]
  %v10817 = phi i64 [ %v10817.ph, %block475.outer ], [ %v217320, %block475.backedge ]
  %v10818 = phi i64 [ %v10818.ph, %block475.outer ], [ %v217319, %block475.backedge ]
  %v10819 = phi i64 [ %v10819.ph, %block475.outer ], [ %v217318, %block475.backedge ]
  %v10820 = phi i64 [ %v10820.ph, %block475.outer ], [ %v217326, %block475.backedge ]
  %v10821 = phi i64 [ %v10821.ph, %block475.outer ], [ %v217327, %block475.backedge ]
  %v10822 = phi i64 [ %v10822.ph, %block475.outer ], [ %v36623649, %block475.backedge ]
  %v10823 = phi i64 [ %v10823.ph, %block475.outer ], [ %v4032811191139, %block475.backedge ]
  %v10827 = phi i64 [ %v10827.ph, %block475.outer ], [ %v40030, %block475.backedge ]
  %v10829 = phi i64 [ %v10829.ph, %block475.outer ], [ %v40042, %block475.backedge ]
  %v12011 = phi i64 [ %v12011.ph, %block475.outer ], [ %v217294, %block475.backedge ]
  %v12415 = phi i64 [ %v12415.ph, %block475.outer ], [ %v217296, %block475.backedge ]
  %t1075.not = icmp eq i64 %v10787, 0
  %or.cond296 = select i1 %v1061851.not1217, i1 true, i1 %t1075.not
  br i1 %or.cond296, label %edge1406, label %edge1242

block11753:                                       ; preds = %edge1314
  br i1 %t23.not, label %edge1326, label %edge1320

block11700:                                       ; preds = %edge1356, %edge1344, %edge1338, %edge1332, %edge1326, %edge1320, %edge1314, %edge1294
  %v10610 = phi i64 [ %v10720, %edge1344 ], [ %v10716, %edge1338 ], [ %v10712, %edge1332 ], [ %v10708, %edge1326 ], [ %v10704, %edge1320 ], [ %v10700, %edge1314 ], [ %v10696, %edge1294 ], [ %v10757, %edge1356 ]
  %t1343 = insertvalue { i64, i64 } poison, i64 %v10610, 0
  %t1344 = insertvalue { i64, i64 } %t1343, i64 475, 1
  br label %common.ret

block12020:                                       ; preds = %edge1423
  %t1358 = insertvalue { i64, i64 } poison, i64 %v11400, 0
  %t1359 = insertvalue { i64, i64 } %t1358, i64 511, 1
  br label %common.ret

block12031:                                       ; preds = %edge1442, %edge1444
  br i1 %t1075.not, label %edge1496, label %edge1453

block12030:                                       ; preds = %edge1453, %edge1441, %edge1438
  %v11416 = phi i64 [ 103427, %edge1441 ], [ %v11475, %edge1453 ], [ %v11456, %edge1438 ]
  %t1393 = insertvalue { i64, i64 } poison, i64 %v11416, 0
  %t1394 = insertvalue { i64, i64 } %t1393, i64 514, 1
  br label %common.ret

block535:                                         ; preds = %edge1406, %edge1423, %edge1496
  %v217292 = phi i64 [ %v11828, %edge1423 ], [ %v11828, %edge1496 ], [ 0, %edge1406 ]
  %v217293 = phi i64 [ %v11996, %edge1423 ], [ %v11996, %edge1496 ], [ %v10809, %edge1406 ]
  %v217294 = phi i64 [ %v12009, %edge1423 ], [ %v12009, %edge1496 ], [ %v10810, %edge1406 ]
  %v217295 = phi i64 [ %v12209, %edge1423 ], [ %v12209, %edge1496 ], [ %v11037, %edge1406 ]
  %v217296 = phi i64 [ %v12413, %edge1423 ], [ %v12413, %edge1496 ], [ %v10811, %edge1406 ]
  %v217297 = phi i64 [ %v12657, %edge1423 ], [ %v12657, %edge1496 ], [ %v10816, %edge1406 ]
  %v217304 = phi i64 [ %v11461, %edge1423 ], [ 0, %edge1496 ], [ %v10787, %edge1406 ]
  %v217305 = phi i64 [ %v14679, %edge1423 ], [ %v14679, %edge1496 ], [ %v10807, %edge1406 ]
  %v217306 = phi i64 [ %v11459, %edge1423 ], [ %v11459, %edge1496 ], [ %v10822, %edge1406 ]
  %v217307 = phi i64 [ %v11460, %edge1423 ], [ %v11460, %edge1496 ], [ %v10794, %edge1406 ]
  %v217308 = phi i64 [ %v18188, %edge1423 ], [ %v18188, %edge1496 ], [ %v10812, %edge1406 ]
  %v217311 = phi i64 [ %v18667, %edge1423 ], [ %v18667, %edge1496 ], [ %v10808, %edge1406 ]
  %v217312 = phi i64 [ %v37160, %edge1423 ], [ %v37160, %edge1496 ], [ %v10815, %edge1406 ]
  %v217317 = phi i64 [ %v37923, %edge1423 ], [ %v37923, %edge1496 ], [ %v10814, %edge1406 ]
  %v217318 = phi i64 [ %v38007, %edge1423 ], [ %v38007, %edge1496 ], [ %v10819, %edge1406 ]
  %v217319 = phi i64 [ %v41624, %edge1423 ], [ %v41624, %edge1496 ], [ %v10818, %edge1406 ]
  %v217320 = phi i64 [ %v41851, %edge1423 ], [ %v41851, %edge1496 ], [ %v10817, %edge1406 ]
  %v217321 = phi i64 [ %v42188, %edge1423 ], [ %v42188, %edge1496 ], [ %v10813, %edge1406 ]
  %v217326 = phi i64 [ %v48118, %edge1423 ], [ %v48118, %edge1496 ], [ %v10820, %edge1406 ]
  %v217327 = phi i64 [ %v48174, %edge1423 ], [ %v48174, %edge1496 ], [ %v10821, %edge1406 ]
  %t1446 = icmp eq i64 %v217292, 1
  %spec.select1 = select i1 %t1446, i64 %v217293, i64 %v217294
  %spec.select18 = call i64 @llvm.umin.i64(i64 %v217295, i64 32768)
  %v12398 = add i64 %spec.select18, %v217293
  %v41333 = select i1 %t1446, i64 %v12398, i64 %v217296
  %spec.select17 = call i64 @llvm.umin.i64(i64 %v217295, i64 258)
  %v12688 = lshr i64 %v217297, 32
  %t1457 = icmp ne i64 %v12688, 0
  %t1458 = icmp ugt i64 %v217295, 4
  br i1 %t1458, label %edge1545, label %edge3984

block13063:                                       ; preds = %edge1680, %edge1677
  %t1503.not = icmp eq i64 %v217710, 0
  br i1 %t1503.not, label %block13067, label %edge1686

block13067:                                       ; preds = %edge1686, %block13063
  %t1505.not = icmp eq i64 %v217725, 0
  br i1 %t1505.not, label %block13071, label %edge1692

block13071:                                       ; preds = %edge1692, %block13067
  %t1507.not = icmp eq i64 %v217738, 0
  br i1 %t1507.not, label %block13075, label %edge1698

block13075:                                       ; preds = %edge1698, %block13071
  %t1509.not = icmp eq i64 %v217744, 0
  br i1 %t1509.not, label %block13079, label %edge1704

block13079:                                       ; preds = %edge1704, %block13075
  %t1511.not = icmp eq i64 %v217754, 0
  br i1 %t1511.not, label %block13087, label %edge1710

block13087:                                       ; preds = %block13079, %edge1710
  %t1515.not = icmp eq i64 %v10788, 0
  br i1 %t1515.not, label %block13091, label %edge1722

block13091:                                       ; preds = %edge1722, %block13087
  br i1 %t23.not, label %block13095, label %edge1728

block13095:                                       ; preds = %edge1728, %block13091
  %t1519.not = icmp eq i64 %v10789.ph.ph, 0
  br i1 %t1519.not, label %block13099, label %edge1734

block13099:                                       ; preds = %edge1734, %block13095
  %t1521.not = icmp eq i64 %v10790, 0
  br i1 %t1521.not, label %block13103, label %edge1740

block13103:                                       ; preds = %edge1740, %block13099
  br i1 %t3136.not, label %block13107, label %edge1746

block13107:                                       ; preds = %edge1746, %block13103
  %t1525.not = icmp eq i64 %v10792, 0
  br i1 %t1525.not, label %block13111, label %edge1752

block13111:                                       ; preds = %edge1752, %block13107
  %t1527.not = icmp eq i64 %v217304, 0
  br i1 %t1527.not, label %common.ret, label %edge1758

block12995:                                       ; preds = %edge1758, %edge1752, %edge1746, %edge1740, %edge1734, %edge1728, %edge1722, %edge1710, %edge1704, %edge1698, %edge1692, %edge1686, %edge1680
  %v13671 = phi i64 [ %v13789, %edge1752 ], [ %v13741, %edge1680 ], [ %v13745, %edge1686 ], [ %v13749, %edge1692 ], [ %v13753, %edge1698 ], [ %v13757, %edge1704 ], [ %v13761, %edge1710 ], [ %v13793, %edge1758 ], [ %v13769, %edge1722 ], [ %v13773, %edge1728 ], [ %v13777, %edge1734 ], [ %v13781, %edge1740 ], [ %v13785, %edge1746 ]
  %t1529 = insertvalue { i64, i64 } poison, i64 %v13671, 0
  %t1530 = insertvalue { i64, i64 } %t1529, i64 589, 1
  br label %common.ret

block13485:                                       ; preds = %edge1835.thread, %edge1835
  %0 = phi i16 [ %11, %edge1835 ], [ %v14827, %edge1835.thread ]
  %t1585 = zext i64 %v217306 to i128
  %v218238 = or disjoint i128 %t1585, 4835703278458516698824704
  store i128 %v218238, ptr %ss0, align 16
  %t1589 = zext i64 %v217307 to i128
  %v218240 = or disjoint i128 %t1589, 18446744073709551616
  store i128 %v218240, ptr %t1002, align 16
  %t1595 = zext i64 %v217304 to i128
  %t1596 = shl nuw i128 %t1595, 64
  %v218242 = or disjoint i128 %t1596, 262144
  store i128 %v218242, ptr %t1007, align 16
  %v14894 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v217307, i64 2, i64 2, i64 %v3192)
  %t1599.not = icmp eq i64 %v14894, 0
  br i1 %t1599.not, label %block13484, label %block13460

block13484:                                       ; preds = %block13485
  %v14897 = load i64, ptr %t607, align 8
  %v14896 = load i64, ptr %t606, align 8
  %v14895 = load i64, ptr %ss1, align 8
  %v14898 = add i64 %v14895, %v14774
  %t1602 = inttoptr i64 %v14898 to ptr
  store i16 %0, ptr %t1602, align 1
  br label %block13483

block13483:                                       ; preds = %edge1835.thread, %block13484
  %v14873 = phi i64 [ %v14895, %block13484 ], [ %v217306, %edge1835.thread ]
  %v14875 = phi i64 [ %v14896, %block13484 ], [ %v217307, %edge1835.thread ]
  %v14878 = phi i64 [ %v14897, %block13484 ], [ %v217304, %edge1835.thread ]
  %t1603 = icmp eq i64 %v217304, %v14878
  %t1605.not52 = icmp eq i64 %v14878, 0
  %t1605.not = or i1 %t1603, %t1605.not52
  br i1 %t1605.not, label %block13489, label %edge1847

block13489:                                       ; preds = %block13483, %edge1847
  %t1609.not = or i1 %t1559.not, %t1603
  br i1 %t1609.not, label %edge1858, label %edge1852

block13460:                                       ; preds = %edge1852, %block13485
  %v14811 = phi i64 [ %v14894, %block13485 ], [ %v14908, %edge1852 ]
  %t1611 = insertvalue { i64, i64 } poison, i64 %v14811, 0
  %t1612 = insertvalue { i64, i64 } %t1611, i64 660, 1
  br label %common.ret

block13500:                                       ; preds = %edge1858.thread, %edge1862, %edge1861
  %v1487320773207882100121016 = phi i64 [ %v14873, %edge1861 ], [ %v14873, %edge1862 ], [ %v217306, %edge1858.thread ]
  %v1487520774207872100221015 = phi i64 [ %v14875, %edge1861 ], [ %v14875, %edge1862 ], [ %v217307, %edge1858.thread ]
  %v1487820775207862100321013 = phi i64 [ %v14878, %edge1861 ], [ %v14878, %edge1862 ], [ 0, %edge1858.thread ]
  %t1605.not5220778207852100421012 = phi i1 [ true, %edge1861 ], [ false, %edge1862 ], [ true, %edge1858.thread ]
  %t1624 = zext i64 %v1487320773207882100121016 to i128
  %v218244 = or disjoint i128 %t1624, 4835703278458516698824704
  store i128 %v218244, ptr %ss0, align 16
  %t1628 = zext i64 %v1487520774207872100221015 to i128
  %v218246 = or disjoint i128 %t1628, 18446744073709551616
  store i128 %v218246, ptr %t1002, align 16
  %t1634 = zext i64 %v1487820775207862100321013 to i128
  %t1635 = shl nuw i128 %t1634, 64
  %v218248 = or disjoint i128 %t1635, 262144
  store i128 %v218248, ptr %t1007, align 16
  %v14953 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v1487520774207872100221015, i64 2, i64 2, i64 %v3192)
  %t1638.not = icmp eq i64 %v14953, 0
  br i1 %t1638.not, label %block13498, label %block13495

block13498:                                       ; preds = %block13500
  %v14954 = load i64, ptr %ss1, align 8
  %v14955 = load i64, ptr %t606, align 8
  %v14956 = load i64, ptr %t607, align 8
  %.pre20753 = add i64 %v14954, %v14802
  %.pre20754 = inttoptr i64 %.pre20753 to ptr
  store i16 %t1550, ptr %.pre20754, align 1
  %t1642 = icmp eq i64 %v1487820775207862100321013, %v14956
  %t1644.not54 = icmp eq i64 %v14956, 0
  %t1644.not = or i1 %t1642, %t1644.not54
  br i1 %t1644.not, label %block13504, label %edge1870

block13504:                                       ; preds = %block13498, %edge1870
  %t1648.not = or i1 %t1605.not5220778207852100421012, %t1642
  br i1 %t1648.not, label %block662, label %edge1875

block13495:                                       ; preds = %edge1875, %block13500
  %v14920 = phi i64 [ %v14953, %block13500 ], [ %v14967, %edge1875 ]
  %t1650 = insertvalue { i64, i64 } poison, i64 %v14920, 0
  %t1651 = insertvalue { i64, i64 } %t1650, i64 662, 1
  br label %common.ret

block662.sink.split:                              ; preds = %edge1862, %edge1826
  %t1615.sink = phi ptr [ %t1555, %edge1826 ], [ %t1615, %edge1862 ]
  %v18807.ph = phi i64 [ %v217306, %edge1826 ], [ %v14873, %edge1862 ]
  %v18830.ph = phi i64 [ %v217304, %edge1826 ], [ %v14878, %edge1862 ]
  %v18849.ph = phi i64 [ %v217307, %edge1826 ], [ %v14875, %edge1862 ]
  store i16 %t1550, ptr %t1615.sink, align 1
  br label %block662

block662:                                         ; preds = %block662.sink.split, %edge1858.thread, %edge1858, %block13504, %edge1875
  %v18807 = phi i64 [ %v217306, %edge1858.thread ], [ %v14954, %edge1875 ], [ %v14954, %block13504 ], [ %v14873, %edge1858 ], [ %v18807.ph, %block662.sink.split ]
  %v18830 = phi i64 [ 0, %edge1858.thread ], [ %v14956, %edge1875 ], [ %v14956, %block13504 ], [ %v14878, %edge1858 ], [ %v18830.ph, %block662.sink.split ]
  %v18849 = phi i64 [ %v217307, %edge1858.thread ], [ %v14955, %edge1875 ], [ %v14955, %block13504 ], [ %v14875, %edge1858 ], [ %v18849.ph, %block662.sink.split ]
  br i1 %t1457, label %edge2077, label %edge1885

block15241:                                       ; preds = %edge2029, %edge2028
  %t1717 = zext i64 %v10823 to i128
  %v218250 = or disjoint i128 %t1719, %t1717
  store i128 %v218250, ptr %ss0, align 16
  %t1721 = zext i64 %v10796 to i128
  %v218252 = or disjoint i128 %t1721, 18446744073709551616
  store i128 %v218252, ptr %t1002, align 16
  %t1727 = zext i64 %v10788 to i128
  %t1728 = shl nuw i128 %t1727, 64
  %v218254 = or disjoint i128 %t1728, %t1163
  store i128 %v218254, ptr %t1007, align 16
  %v18375 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v10796, i64 4, i64 4, i64 %v3192)
  %t1731.not = icmp eq i64 %v18375, 0
  br i1 %t1731.not, label %edge2034, label %block15236

block15240:                                       ; preds = %edge2029, %edge2034
  %t1734.pre-phi = phi ptr [ %t1708, %edge2029 ], [ %.pre20752, %edge2034 ]
  %v18360 = phi i64 [ %v10823, %edge2029 ], [ %v18376, %edge2034 ]
  %v18362 = phi i64 [ %v10796, %edge2029 ], [ %v18377, %edge2034 ]
  %v18365 = phi i64 [ %v10788, %edge2029 ], [ %v18378, %edge2034 ]
  %v18380 = trunc nuw i64 %v18172 to i32
  store i32 %v18380, ptr %t1734.pre-phi, align 1
  br label %block15239

block15239:                                       ; preds = %edge2025, %block15240
  %v18354 = phi i64 [ %v18360, %block15240 ], [ %v10823, %edge2025 ]
  %v18356 = phi i64 [ %v18362, %block15240 ], [ %v10796, %edge2025 ]
  %v18359 = phi i64 [ %v18365, %block15240 ], [ %v10788, %edge2025 ]
  %t1735 = icmp eq i64 %v10788, %v18359
  %t1737.not56 = icmp eq i64 %v18359, 0
  %t1737.not = or i1 %t1735, %t1737.not56
  br i1 %t1737.not, label %block15245, label %edge2037

block15245:                                       ; preds = %block15239, %edge2037
  %t1741.not57 = icmp eq i64 %v10788, 0
  %t1741.not = or i1 %t1741.not57, %t1735
  br i1 %t1741.not, label %edge2015, label %edge2042

block15236:                                       ; preds = %edge2042, %block15241, %edge2017
  %v18218 = phi i64 [ %v18375, %block15241 ], [ 103425, %edge2017 ], [ %v18389, %edge2042 ]
  %t1743 = insertvalue { i64, i64 } poison, i64 %v18218, 0
  %t1744 = insertvalue { i64, i64 } %t1743, i64 862, 1
  br label %common.ret

block15432:                                       ; preds = %edge2085, %edge2084
  %t1765 = zext i64 %v18807 to i128
  %v218256 = or disjoint i128 %t1765, 4835703278458516698824704
  store i128 %v218256, ptr %ss0, align 16
  %t1769 = zext i64 %v18849 to i128
  %v218258 = or disjoint i128 %t1769, 18446744073709551616
  store i128 %v218258, ptr %t1002, align 16
  %t1775 = zext i64 %v18830 to i128
  %t1776 = shl nuw i128 %t1775, 64
  %v218260 = or disjoint i128 %t1776, 262144
  store i128 %v218260, ptr %t1007, align 16
  %v18894 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v18849, i64 2, i64 2, i64 %v3192)
  %t1779.not = icmp eq i64 %v18894, 0
  br i1 %t1779.not, label %edge2090, label %block15427

block15431:                                       ; preds = %edge2085, %edge2090
  %t1782.pre-phi = phi ptr [ %t1750, %edge2085 ], [ %.pre20750, %edge2090 ]
  %v18879 = phi i64 [ %v18807, %edge2085 ], [ %v18895, %edge2090 ]
  %v18881 = phi i64 [ %v18849, %edge2085 ], [ %v18896, %edge2090 ]
  %v18884 = phi i64 [ %v18830, %edge2085 ], [ %v18897, %edge2090 ]
  store i16 %13, ptr %t1782.pre-phi, align 1
  br label %block15430

block15430:                                       ; preds = %edge2077, %block15431
  %v18873 = phi i64 [ %v18879, %block15431 ], [ %v18807, %edge2077 ]
  %v18875 = phi i64 [ %v18881, %block15431 ], [ %v18849, %edge2077 ]
  %v18878 = phi i64 [ %v18884, %block15431 ], [ %v18830, %edge2077 ]
  %t1783 = icmp eq i64 %v18830, %v18878
  %t1785.not58 = icmp eq i64 %v18878, 0
  %t1785.not = or i1 %t1783, %t1785.not58
  br i1 %t1785.not, label %block15436, label %edge2093

block15436:                                       ; preds = %block15430, %edge2093
  %t1789.not59 = icmp eq i64 %v18830, 0
  %t1789.not = or i1 %t1789.not59, %t1783
  br i1 %t1789.not, label %edge2102, label %edge2098

block15427:                                       ; preds = %edge2098, %block15432
  %v18827 = phi i64 [ %v18894, %block15432 ], [ %v18908, %edge2098 ]
  %t1791 = insertvalue { i64, i64 } poison, i64 %v18827, 0
  %t1792 = insertvalue { i64, i64 } %t1791, i64 890, 1
  br label %common.ret

block15597:                                       ; preds = %edge2127.thread, %edge2127
  %v19246518520 = phi i64 [ %v19246, %edge2127 ], [ %v19204, %edge2127.thread ]
  %t1822 = zext i64 %v18873 to i128
  %v218262 = or disjoint i128 %t1822, 4835703278458516698824704
  store i128 %v218262, ptr %ss0, align 16
  %t1826 = zext i64 %v18875 to i128
  %v218264 = or disjoint i128 %t1826, 18446744073709551616
  store i128 %v218264, ptr %t1002, align 16
  %t1832 = zext i64 %v18878 to i128
  %t1833 = shl nuw i128 %t1832, 64
  %v218266 = or disjoint i128 %t1833, 262144
  store i128 %v218266, ptr %t1007, align 16
  %v19273 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v18875, i64 2, i64 2, i64 %v3192)
  %t1836.not = icmp eq i64 %v19273, 0
  br i1 %t1836.not, label %block15596, label %block15572

block15596:                                       ; preds = %block15597
  %v19276 = load i64, ptr %t607, align 8
  %v19275 = load i64, ptr %t606, align 8
  %v19274 = load i64, ptr %ss1, align 8
  %v19277 = add i64 %v19274, %v19246518520
  %t1839 = inttoptr i64 %v19277 to ptr
  store i16 0, ptr %t1839, align 1
  br label %block15595

block15595:                                       ; preds = %edge2127.thread, %block15596
  %v19252 = phi i64 [ %v19274, %block15596 ], [ %v18873, %edge2127.thread ]
  %v19254 = phi i64 [ %v19275, %block15596 ], [ %v18875, %edge2127.thread ]
  %v19257 = phi i64 [ %v19276, %block15596 ], [ %v18878, %edge2127.thread ]
  %t1840 = icmp eq i64 %v18878, %v19257
  %t1842.not90 = icmp eq i64 %v19257, 0
  %t1842.not = or i1 %t1840, %t1842.not90
  br i1 %t1842.not, label %block15601, label %edge2139

block15601:                                       ; preds = %block15595, %edge2139
  %t1846.not = or i1 %t1785.not58, %t1840
  br i1 %t1846.not, label %edge2150, label %edge2144

block15572:                                       ; preds = %edge2144, %block15597
  %v19194 = phi i64 [ %v19273, %block15597 ], [ %v19287, %edge2144 ]
  %t1848 = insertvalue { i64, i64 } poison, i64 %v19194, 0
  %t1849 = insertvalue { i64, i64 } %t1848, i64 901, 1
  br label %common.ret

block15612:                                       ; preds = %edge2150.thread, %edge2154, %edge2153
  %v1925220813208282102621044 = phi i64 [ %v19252, %edge2153 ], [ %v19252, %edge2154 ], [ %v18873, %edge2150.thread ]
  %v1925420814208272102721043 = phi i64 [ %v19254, %edge2153 ], [ %v19254, %edge2154 ], [ %v18875, %edge2150.thread ]
  %v1925720815208262102821041 = phi i64 [ %v19257, %edge2153 ], [ %v19257, %edge2154 ], [ 0, %edge2150.thread ]
  %t1842.not9020818208252102921040 = phi i1 [ true, %edge2153 ], [ false, %edge2154 ], [ true, %edge2150.thread ]
  %v193472103021038 = phi i64 [ %v19347, %edge2153 ], [ %v19347, %edge2154 ], [ %v1934721021, %edge2150.thread ]
  %t1861 = zext i64 %v1925220813208282102621044 to i128
  %v218268 = or disjoint i128 %t1861, 4835703278458516698824704
  store i128 %v218268, ptr %ss0, align 16
  %t1865 = zext i64 %v1925420814208272102721043 to i128
  %v218270 = or disjoint i128 %t1865, 18446744073709551616
  store i128 %v218270, ptr %t1002, align 16
  %t1871 = zext i64 %v1925720815208262102821041 to i128
  %t1872 = shl nuw i128 %t1871, 64
  %v218272 = or disjoint i128 %t1872, 262144
  store i128 %v218272, ptr %t1007, align 16
  %v19374 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v1925420814208272102721043, i64 2, i64 2, i64 %v3192)
  %t1875.not = icmp eq i64 %v19374, 0
  br i1 %t1875.not, label %block15610, label %block15607

block15610:                                       ; preds = %block15612
  %v19375 = load i64, ptr %ss1, align 8
  %v19376 = load i64, ptr %t606, align 8
  %v19377 = load i64, ptr %t607, align 8
  %.pre20747 = add i64 %v19375, %v193472103021038
  %.pre20748 = inttoptr i64 %.pre20747 to ptr
  store i16 0, ptr %.pre20748, align 1
  %t1879 = icmp eq i64 %v1925720815208262102821041, %v19377
  %t1881.not92 = icmp eq i64 %v19377, 0
  %t1881.not = or i1 %t1879, %t1881.not92
  br i1 %t1881.not, label %block15616, label %edge2162

block15616:                                       ; preds = %block15610, %edge2162
  %t1885.not = or i1 %t1842.not9020818208252102921040, %t1879
  br i1 %t1885.not, label %edge3980, label %edge2167

block15607:                                       ; preds = %edge2167, %block15612
  %v19341 = phi i64 [ %v19374, %block15612 ], [ %v19388, %edge2167 ]
  %t1887 = insertvalue { i64, i64 } poison, i64 %v19341, 0
  %t1888 = insertvalue { i64, i64 } %t1887, i64 903, 1
  br label %common.ret

block913:                                         ; preds = %block913.preheader, %edge2826
  %v19842 = phi i64 [ %v26931, %edge2826 ], [ %v18873, %block913.preheader ]
  %v19844 = phi i64 [ %v26933, %edge2826 ], [ %v18875, %block913.preheader ]
  %v19847 = phi i64 [ %v26936, %edge2826 ], [ %v18878, %block913.preheader ]
  %v19854 = phi i64 [ %v26101559, %edge2826 ], [ %v18644, %block913.preheader ]
  %v19856 = phi i64 [ %v26103560, %edge2826 ], [ %v18646, %block913.preheader ]
  %v19859 = phi i64 [ %v26106561, %edge2826 ], [ %v18649, %block913.preheader ]
  %v19866 = phi i64 [ %spec.select19562, %edge2826 ], [ %v18656, %block913.preheader ]
  %v19899 = phi i64 [ %v26900, %edge2826 ], [ %v19004, %block913.preheader ]
  %v19902 = phi i64 [ %v26903, %edge2826 ], [ %v19009, %block913.preheader ]
  %v19905 = phi i64 [ %v26979, %edge2826 ], [ %v18769, %block913.preheader ]
  %v19908 = phi i64 [ %v27175, %edge2826 ], [ 300, %block913.preheader ]
  %v19911 = phi i64 [ %v26906, %edge2826 ], [ 0, %block913.preheader ]
  %v19914 = phi i64 [ %v26909, %edge2826 ], [ 0, %block913.preheader ]
  %v19917 = phi i64 [ %spec.select20, %edge2826 ], [ 0, %block913.preheader ]
  %v19920 = phi i64 [ %v26144558, %edge2826 ], [ 3, %block913.preheader ]
  %t1894 = icmp ult i64 %v19917, %spec.select17
  br i1 %t1894, label %edge2227, label %block19718

trap1911:                                         ; preds = %edge2256
  call void @llvm.trap()
  unreachable

ok1911:                                           ; preds = %edge2256
  %v20531 = udiv i64 %v20333, %v1397
  %v20532 = mul i64 %v20531, %v1397
  %v20533 = sub i64 %v20333, %v20532
  %v20534 = mul i64 %v20533, %v1398
  %v20535 = add i64 %v20534, %v20531
  br label %block16244

block16244:                                       ; preds = %edge2251, %ok1911
  %v20530 = phi i64 [ %v20535, %ok1911 ], [ %v20333, %edge2251 ]
  %v20537 = add i64 %v20530, %v1394
  %t1912 = inttoptr i64 %v20537 to ptr
  %t1913 = load i8, ptr %t1912, align 1
  %v20538 = zext i8 %t1913 to i64
  br label %edge2249

trap1921:                                         ; preds = %edge2275
  call void @llvm.trap()
  unreachable

ok1921:                                           ; preds = %edge2275
  %v20757 = udiv i64 %v20560, %v1397
  %v20758 = mul i64 %v20757, %v1397
  %v20759 = sub i64 %v20560, %v20758
  %v20760 = mul i64 %v20759, %v1398
  %v20761 = add i64 %v20760, %v20757
  br label %block16387

block16387:                                       ; preds = %edge2270, %ok1921
  %v20756 = phi i64 [ %v20761, %ok1921 ], [ %v20560, %edge2270 ]
  %v20763 = add i64 %v20756, %v1394
  %t1922 = inttoptr i64 %v20763 to ptr
  %t1923 = load i8, ptr %t1922, align 1
  %v20764 = zext i8 %t1923 to i64
  br label %edge2268

block16598:                                       ; preds = %edge2316, %edge2313
  %t1936.not = icmp eq i64 %v217710, 0
  br i1 %t1936.not, label %block16602, label %edge2322

block16602:                                       ; preds = %edge2322, %block16598
  %t1938.not = icmp eq i64 %v217725, 0
  br i1 %t1938.not, label %block16606, label %edge2328

block16606:                                       ; preds = %edge2328, %block16602
  %t1940.not = icmp eq i64 %v217738, 0
  br i1 %t1940.not, label %block16610, label %edge2334

block16610:                                       ; preds = %edge2334, %block16606
  %t1942.not = icmp eq i64 %v217744, 0
  br i1 %t1942.not, label %block16614, label %edge2340

block16614:                                       ; preds = %edge2340, %block16610
  %t1944.not = icmp eq i64 %v217754, 0
  br i1 %t1944.not, label %block16622, label %edge2346

block16622:                                       ; preds = %block16614, %edge2346
  br i1 %t23.not, label %block16626, label %edge2358

block16626:                                       ; preds = %edge2358, %block16622
  %t1950.not = icmp eq i64 %v10789.ph.ph, 0
  br i1 %t1950.not, label %block16630, label %edge2364

block16630:                                       ; preds = %edge2364, %block16626
  %t1952.not = icmp eq i64 %v10790, 0
  br i1 %t1952.not, label %block16634, label %edge2370

block16634:                                       ; preds = %edge2370, %block16630
  br i1 %t3136.not, label %block16638, label %edge2376

block16638:                                       ; preds = %edge2376, %block16634
  %t1956.not = icmp eq i64 %v10792, 0
  br i1 %t1956.not, label %block16642, label %edge2382

block16642:                                       ; preds = %edge2382, %block16638
  %t1958.not = icmp eq i64 %v19847, 0
  br i1 %t1958.not, label %block16646, label %edge2388

block16646:                                       ; preds = %edge2388, %block16642
  %t1960.not = icmp eq i64 %v19859, 0
  br i1 %t1960.not, label %block16650, label %edge2394

block16650:                                       ; preds = %edge2394, %block16646
  br i1 %t23.not, label %common.ret, label %edge2400

block16530:                                       ; preds = %edge2400, %edge2394, %edge2388, %edge2382, %edge2376, %edge2370, %edge2364, %edge2358, %edge2346, %edge2340, %edge2334, %edge2328, %edge2322, %edge2316
  %v20994 = phi i64 [ %v21116, %edge2394 ], [ %v21064, %edge2316 ], [ %v21068, %edge2322 ], [ %v21072, %edge2328 ], [ %v21076, %edge2334 ], [ %v21080, %edge2340 ], [ %v21084, %edge2346 ], [ %v21120, %edge2400 ], [ %v21092, %edge2358 ], [ %v21096, %edge2364 ], [ %v21100, %edge2370 ], [ %v21104, %edge2376 ], [ %v21108, %edge2382 ], [ %v21112, %edge2388 ]
  %t1964 = insertvalue { i64, i64 } poison, i64 %v20994, 0
  %t1965 = insertvalue { i64, i64 } %t1964, i64 996, 1
  br label %common.ret

block1017:                                        ; preds = %block1017.preheader, %edge2517
  %v22057 = phi i64 [ %v22920, %edge2517 ], [ %v21748, %block1017.preheader ]
  %t1978.not = icmp ult i64 %v22057, %spec.select17
  br i1 %t1978.not, label %edge2465, label %edge2451

trap1995:                                         ; preds = %edge2478
  call void @llvm.trap()
  unreachable

ok1995:                                           ; preds = %edge2478
  %v22474 = udiv i64 %v22268, %v1397
  %v22475 = mul i64 %v22474, %v1397
  %v22476 = sub nuw i64 %v22268, %v22475
  %v22477 = mul i64 %v22476, %v1398
  %v22478 = add i64 %v22477, %v22474
  br label %block17131

block17131:                                       ; preds = %edge2473, %ok1995
  %v22473 = phi i64 [ %v22478, %ok1995 ], [ %v22268, %edge2473 ]
  %v22480 = add i64 %v22473, %v1394
  %t1996 = inttoptr i64 %v22480 to ptr
  %t1997 = load i8, ptr %t1996, align 1
  %v22481 = zext i8 %t1997 to i64
  br label %edge2471

trap2005:                                         ; preds = %edge2497
  call void @llvm.trap()
  unreachable

ok2005:                                           ; preds = %edge2497
  %v22707 = udiv i64 %v22505, %v1397
  %v22708 = mul i64 %v22707, %v1397
  %v22709 = sub nuw i64 %v22505, %v22708
  %v22710 = mul i64 %v22709, %v1398
  %v22711 = add i64 %v22710, %v22707
  br label %block17278

block17278:                                       ; preds = %edge2492, %ok2005
  %v22706 = phi i64 [ %v22711, %ok2005 ], [ %v22505, %edge2492 ]
  %v22713 = add i64 %v22706, %v1394
  %t2006 = inttoptr i64 %v22713 to ptr
  %t2007 = load i8, ptr %t2006, align 1
  %v22714 = zext i8 %t2007 to i64
  br label %edge2490

block18352:                                       ; preds = %edge2620, %edge2619
  %t2055 = zext i64 %v19854 to i128
  %v218274 = or disjoint i128 %t1719, %t2055
  store i128 %v218274, ptr %ss0, align 16
  %t2059 = zext i64 %v19856 to i128
  %v218276 = or disjoint i128 %t2059, 18446744073709551616
  store i128 %v218276, ptr %t1002, align 16
  %t2065 = zext i64 %v19859 to i128
  %t2066 = shl nuw i128 %t2065, 64
  %v218278 = or disjoint i128 %t2066, %t1163
  store i128 %v218278, ptr %t1007, align 16
  %v25162 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v19856, i64 4, i64 4, i64 %v3192)
  %t2069.not = icmp eq i64 %v25162, 0
  br i1 %t2069.not, label %edge2625, label %block18347

block18351:                                       ; preds = %edge2620, %edge2625
  %t2072.pre-phi = phi ptr [ %t2046, %edge2620 ], [ %.pre20746, %edge2625 ]
  %v25147 = phi i64 [ %v19854, %edge2620 ], [ %v25163, %edge2625 ]
  %v25149 = phi i64 [ %v19856, %edge2620 ], [ %v25164, %edge2625 ]
  %v25152 = phi i64 [ %v19859, %edge2620 ], [ %v25165, %edge2625 ]
  %v25167 = trunc nuw i64 %v25030 to i32
  store i32 %v25167, ptr %t2072.pre-phi, align 1
  br label %block18350

block18350:                                       ; preds = %edge2616, %block18351
  %v25141 = phi i64 [ %v25147, %block18351 ], [ %v19854, %edge2616 ]
  %v25143 = phi i64 [ %v25149, %block18351 ], [ %v19856, %edge2616 ]
  %v25146 = phi i64 [ %v25152, %block18351 ], [ %v19859, %edge2616 ]
  %t2073 = icmp eq i64 %v19859, %v25146
  %t2075.not76 = icmp eq i64 %v25146, 0
  %t2075.not = or i1 %t2073, %t2075.not76
  br i1 %t2075.not, label %block18356, label %edge2628

block18356:                                       ; preds = %block18350, %edge2628
  %t2079.not77 = icmp eq i64 %v19859, 0
  %t2079.not = or i1 %t2079.not77, %t2073
  br i1 %t2079.not, label %edge2605, label %edge2633

block18347:                                       ; preds = %edge2633, %block18352, %edge2607
  %v25057 = phi i64 [ %v25162, %block18352 ], [ 103425, %edge2607 ], [ %v25176, %edge2633 ]
  %t2081 = insertvalue { i64, i64 } poison, i64 %v25057, 0
  %t2082 = insertvalue { i64, i64 } %t2081, i64 1250, 1
  br label %common.ret

block18806:                                       ; preds = %edge2692.thread, %edge2692
  %v25973568570 = phi i64 [ %v25973, %edge2692 ], [ %v25931, %edge2692.thread ]
  %t2128 = zext i64 %v19842 to i128
  %v218280 = or disjoint i128 %t2128, 4835703278458516698824704
  store i128 %v218280, ptr %ss0, align 16
  %t2132 = zext i64 %v19844 to i128
  %v218282 = or disjoint i128 %t2132, 18446744073709551616
  store i128 %v218282, ptr %t1002, align 16
  %t2138 = zext i64 %v19847 to i128
  %t2139 = shl nuw i128 %t2138, 64
  %v218284 = or disjoint i128 %t2139, 262144
  store i128 %v218284, ptr %t1007, align 16
  %v26000 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v19844, i64 2, i64 2, i64 %v3192)
  %t2142.not = icmp eq i64 %v26000, 0
  br i1 %t2142.not, label %block18804, label %block18781

block18804.thread:                                ; preds = %edge2692, %edge2692.thread
  %t2148.not8620852 = icmp eq i64 %v19847, 0
  br label %block18810

block18804:                                       ; preds = %block18806
  %v26003 = load i64, ptr %t607, align 8
  %v26002 = load i64, ptr %t606, align 8
  %v26001 = load i64, ptr %ss1, align 8
  %v26004 = add i64 %v26001, %v25973568570
  %t2145 = inttoptr i64 %v26004 to ptr
  store i16 %t2093, ptr %t2145, align 1
  %t2146 = icmp eq i64 %v19847, %v26003
  %t2148.not86 = icmp eq i64 %v26003, 0
  %t2148.not = or i1 %t2146, %t2148.not86
  br i1 %t2148.not, label %block18810, label %edge2704

block18810:                                       ; preds = %block18804.thread, %block18804, %edge2704
  %t2148.not8620860 = phi i1 [ %t2148.not8620852, %block18804.thread ], [ %t2148.not86, %block18804 ], [ %t2148.not86, %edge2704 ]
  %t214620859 = phi i1 [ true, %block18804.thread ], [ %t2146, %block18804 ], [ %t2146, %edge2704 ]
  %v2598420857 = phi i64 [ %v19847, %block18804.thread ], [ %v26003, %block18804 ], [ %v26003, %edge2704 ]
  %v2598120856 = phi i64 [ %v19844, %block18804.thread ], [ %v26002, %block18804 ], [ %v26002, %edge2704 ]
  %v2597920855 = phi i64 [ %v19842, %block18804.thread ], [ %v26001, %block18804 ], [ %v26001, %edge2704 ]
  %t2152.not = or i1 %t2102.not, %t214620859
  br i1 %t2152.not, label %edge2715, label %edge2709

block18781:                                       ; preds = %edge2709, %block18806
  %v25909 = phi i64 [ %v26000, %block18806 ], [ %v26014, %edge2709 ]
  %t2154 = insertvalue { i64, i64 } poison, i64 %v25909, 0
  %t2155 = insertvalue { i64, i64 } %t2154, i64 1286, 1
  br label %common.ret

block18821:                                       ; preds = %edge2719, %edge2718
  %t2167 = zext i64 %v2597920855 to i128
  %v218286 = or disjoint i128 %t2167, 4835703278458516698824704
  store i128 %v218286, ptr %ss0, align 16
  %t2171 = zext i64 %v2598120856 to i128
  %v218288 = or disjoint i128 %t2171, 18446744073709551616
  store i128 %v218288, ptr %t1002, align 16
  %t2177 = zext i64 %v2598420857 to i128
  %t2178 = shl nuw i128 %t2177, 64
  %v218290 = or disjoint i128 %t2178, 262144
  store i128 %v218290, ptr %t1007, align 16
  %v26078 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v2598120856, i64 2, i64 2, i64 %v3192)
  %t2181.not = icmp eq i64 %v26078, 0
  br i1 %t2181.not, label %edge2724, label %block18816

block18819:                                       ; preds = %edge2724, %edge2719
  %t2184.pre-phi = phi ptr [ %t2158, %edge2719 ], [ %.pre20744, %edge2724 ]
  %v26063 = phi i64 [ %v2597920855, %edge2719 ], [ %v26079, %edge2724 ]
  %v26065 = phi i64 [ %v2598120856, %edge2719 ], [ %v26080, %edge2724 ]
  %v26068 = phi i64 [ %v2598420857, %edge2719 ], [ %v26081, %edge2724 ]
  store i16 %t2099, ptr %t2184.pre-phi, align 1
  %t2185 = icmp eq i64 %v2598420857, %v26068
  %t2187.not88 = icmp eq i64 %v26068, 0
  %t2187.not = or i1 %t2185, %t2187.not88
  br i1 %t2187.not, label %block18825, label %edge2727

block18825:                                       ; preds = %block18819, %edge2727
  %t2191.not = or i1 %t2148.not8620860, %t2185
  br i1 %t2191.not, label %block19718, label %edge2732

block18816:                                       ; preds = %edge2732, %block18821
  %v26045 = phi i64 [ %v26078, %block18821 ], [ %v26092, %edge2732 ]
  %t2193 = insertvalue { i64, i64 } poison, i64 %v26045, 0
  %t2194 = insertvalue { i64, i64 } %t2193, i64 1288, 1
  br label %common.ret

trap2202:                                         ; preds = %edge2754
  call void @llvm.trap()
  unreachable

ok2202:                                           ; preds = %edge2754
  %v26387 = udiv i64 %v26178, %v1397
  %v26388 = mul i64 %v26387, %v1397
  %v26389 = sub i64 %v26178, %v26388
  %v26390 = mul i64 %v26389, %v1398
  %v26391 = add i64 %v26390, %v26387
  br label %block18987

block18987:                                       ; preds = %edge2749, %ok2202
  %v26386 = phi i64 [ %v26391, %ok2202 ], [ %v26178, %edge2749 ]
  %v26393 = add i64 %v26386, %v1394
  %t2203 = inttoptr i64 %v26393 to ptr
  %t2204 = load i8, ptr %t2203, align 1
  %v26394 = zext i8 %t2204 to i64
  br label %edge2747

trap2212:                                         ; preds = %edge2773
  call void @llvm.trap()
  unreachable

ok2212:                                           ; preds = %edge2773
  %v26651 = udiv i64 %v26454, %v1397
  %v26652 = mul i64 %v26651, %v1397
  %v26653 = sub i64 %v26454, %v26652
  %v26654 = mul i64 %v26653, %v1398
  %v26655 = add i64 %v26654, %v26651
  br label %edge2766

block19289:                                       ; preds = %edge2807, %edge2806
  %t2229 = zext i64 %v19842 to i128
  %v218292 = or disjoint i128 %t2229, 4835703278458516698824704
  store i128 %v218292, ptr %ss0, align 16
  %t2233 = zext i64 %v19844 to i128
  %v218294 = or disjoint i128 %t2233, 18446744073709551616
  store i128 %v218294, ptr %t1002, align 16
  %t2239 = zext i64 %v19847 to i128
  %t2240 = shl nuw i128 %t2239, 64
  %v218296 = or disjoint i128 %t2240, 262144
  store i128 %v218296, ptr %t1007, align 16
  %v26952 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v19844, i64 2, i64 2, i64 %v3192)
  %t2243.not = icmp eq i64 %v26952, 0
  br i1 %t2243.not, label %edge2812, label %block19284

block19288:                                       ; preds = %edge2807, %edge2812
  %t2246.pre-phi = phi ptr [ %t2220, %edge2807 ], [ %.pre20742, %edge2812 ]
  %v26937 = phi i64 [ %v19842, %edge2807 ], [ %v26953, %edge2812 ]
  %v26939 = phi i64 [ %v19844, %edge2807 ], [ %v26954, %edge2812 ]
  %v26942 = phi i64 [ %v19847, %edge2807 ], [ %v26955, %edge2812 ]
  %v26957 = trunc nuw i64 %v19905 to i16
  store i16 %v26957, ptr %t2246.pre-phi, align 1
  br label %block19287

block19287:                                       ; preds = %edge2803, %block19288
  %v26931 = phi i64 [ %v26937, %block19288 ], [ %v19842, %edge2803 ]
  %v26933 = phi i64 [ %v26939, %block19288 ], [ %v19844, %edge2803 ]
  %v26936 = phi i64 [ %v26942, %block19288 ], [ %v19847, %edge2803 ]
  %t2247 = icmp eq i64 %v19847, %v26936
  %t2249.not79 = icmp eq i64 %v26936, 0
  %t2249.not = or i1 %t2247, %t2249.not79
  br i1 %t2249.not, label %block19293, label %edge2815

block19293:                                       ; preds = %block19287, %edge2815
  %t2253.not80 = icmp eq i64 %v19847, 0
  %t2253.not = or i1 %t2253.not80, %t2247
  br i1 %t2253.not, label %edge2826, label %edge2820

block19284:                                       ; preds = %edge2820, %block19289
  %v26919 = phi i64 [ %v26952, %block19289 ], [ %v26966, %edge2820 ]
  %t2255 = insertvalue { i64, i64 } poison, i64 %v26919, 0
  %t2256 = insertvalue { i64, i64 } %t2255, i64 1344, 1
  br label %common.ret

block19595:                                       ; preds = %edge2862.thread, %edge2862
  %v27439586588 = phi i64 [ %v27439, %edge2862 ], [ %v27397, %edge2862.thread ]
  %t2296 = zext i64 %v26931 to i128
  %v218298 = or disjoint i128 %t2296, 4835703278458516698824704
  store i128 %v218298, ptr %ss0, align 16
  %t2300 = zext i64 %v26933 to i128
  %v218300 = or disjoint i128 %t2300, 18446744073709551616
  store i128 %v218300, ptr %t1002, align 16
  %t2306 = zext i64 %v26936 to i128
  %t2307 = shl nuw i128 %t2306, 64
  %v218302 = or disjoint i128 %t2307, 262144
  store i128 %v218302, ptr %t1007, align 16
  %v27466 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v26933, i64 2, i64 2, i64 %v3192)
  %t2310.not = icmp eq i64 %v27466, 0
  br i1 %t2310.not, label %block19594, label %block19570

block19594:                                       ; preds = %block19595
  %v27469 = load i64, ptr %t607, align 8
  %v27468 = load i64, ptr %t606, align 8
  %v27467 = load i64, ptr %ss1, align 8
  %v27470 = add i64 %v27467, %v27439586588
  %t2313 = inttoptr i64 %v27470 to ptr
  store i16 0, ptr %t2313, align 1
  br label %block19593

block19593:                                       ; preds = %edge2862.thread, %block19594
  %v27445 = phi i64 [ %v27467, %block19594 ], [ %v26931, %edge2862.thread ]
  %v27447 = phi i64 [ %v27468, %block19594 ], [ %v26933, %edge2862.thread ]
  %v27450 = phi i64 [ %v27469, %block19594 ], [ %v26936, %edge2862.thread ]
  %t2314 = icmp eq i64 %v26936, %v27450
  %t2316.not82 = icmp eq i64 %v27450, 0
  %t2316.not = or i1 %t2314, %t2316.not82
  br i1 %t2316.not, label %block19599, label %edge2874

block19599:                                       ; preds = %block19593, %edge2874
  %t2320.not = or i1 %t2249.not79, %t2314
  br i1 %t2320.not, label %edge2885, label %edge2879

block19570:                                       ; preds = %edge2879, %block19595
  %v27387 = phi i64 [ %v27466, %block19595 ], [ %v27480, %edge2879 ]
  %t2322 = insertvalue { i64, i64 } poison, i64 %v27387, 0
  %t2323 = insertvalue { i64, i64 } %t2322, i64 1366, 1
  br label %common.ret

block19610:                                       ; preds = %edge2885.thread, %edge2889, %edge2888
  %v2744520885209002105421072 = phi i64 [ %v27445, %edge2888 ], [ %v27445, %edge2889 ], [ %v26931, %edge2885.thread ]
  %v2744720886208992105521071 = phi i64 [ %v27447, %edge2888 ], [ %v27447, %edge2889 ], [ %v26933, %edge2885.thread ]
  %v2745020887208982105621069 = phi i64 [ %v27450, %edge2888 ], [ %v27450, %edge2889 ], [ 0, %edge2885.thread ]
  %t2316.not8220890208972105721068 = phi i1 [ true, %edge2888 ], [ false, %edge2889 ], [ true, %edge2885.thread ]
  %v275062105821066 = phi i64 [ %v27506, %edge2888 ], [ %v27506, %edge2889 ], [ %v2750621049, %edge2885.thread ]
  %t2335 = zext i64 %v2744520885209002105421072 to i128
  %v218304 = or disjoint i128 %t2335, 4835703278458516698824704
  store i128 %v218304, ptr %ss0, align 16
  %t2339 = zext i64 %v2744720886208992105521071 to i128
  %v218306 = or disjoint i128 %t2339, 18446744073709551616
  store i128 %v218306, ptr %t1002, align 16
  %t2345 = zext i64 %v2745020887208982105621069 to i128
  %t2346 = shl nuw i128 %t2345, 64
  %v218308 = or disjoint i128 %t2346, 262144
  store i128 %v218308, ptr %t1007, align 16
  %v27533 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v2744720886208992105521071, i64 2, i64 2, i64 %v3192)
  %t2349.not = icmp eq i64 %v27533, 0
  br i1 %t2349.not, label %block19608, label %block19605

block19608:                                       ; preds = %block19610
  %v27534 = load i64, ptr %ss1, align 8
  %v27535 = load i64, ptr %t606, align 8
  %v27536 = load i64, ptr %t607, align 8
  %.pre20739 = add i64 %v27534, %v275062105821066
  %.pre20740 = inttoptr i64 %.pre20739 to ptr
  store i16 0, ptr %.pre20740, align 1
  %t2353 = icmp eq i64 %v2745020887208982105621069, %v27536
  %t2355.not84 = icmp eq i64 %v27536, 0
  %t2355.not = or i1 %t2353, %t2355.not84
  br i1 %t2355.not, label %block19614, label %edge2897

block19614:                                       ; preds = %block19608, %edge2897
  %t2359.not = or i1 %t2316.not8220890208972105721068, %t2353
  br i1 %t2359.not, label %block19718, label %edge2902

block19605:                                       ; preds = %edge2902, %block19610
  %v27500 = phi i64 [ %v27533, %block19610 ], [ %v27547, %edge2902 ]
  %t2361 = insertvalue { i64, i64 } poison, i64 %v27500, 0
  %t2362 = insertvalue { i64, i64 } %t2361, i64 1368, 1
  br label %common.ret

block19718:                                       ; preds = %block913, %block19614.thread21246, %edge2885.thread, %edge2885, %edge2715, %edge2853, %edge2902, %block19614, %edge2732, %block18825, %edge2683
  %v217784 = phi i64 [ %v26063, %edge2732 ], [ %v27534, %block19614 ], [ %v26063, %block18825 ], [ %v19842, %edge2683 ], [ %v27534, %edge2902 ], [ %v26931, %edge2853 ], [ %v2597920855, %edge2715 ], [ %v27445, %block19614.thread21246 ], [ %v27445, %edge2885 ], [ %v26931, %edge2885.thread ], [ %v19842, %block913 ]
  %v217785 = phi i64 [ %v26065, %edge2732 ], [ %v27535, %block19614 ], [ %v26065, %block18825 ], [ %v19844, %edge2683 ], [ %v27535, %edge2902 ], [ %v26933, %edge2853 ], [ %v2598120856, %edge2715 ], [ %v27447, %block19614.thread21246 ], [ %v27447, %edge2885 ], [ %v26933, %edge2885.thread ], [ %v19844, %block913 ]
  %v217786 = phi i64 [ %v26068, %edge2732 ], [ %v27536, %block19614 ], [ %v26068, %block18825 ], [ %v19847, %edge2683 ], [ %v27536, %edge2902 ], [ %v26936, %edge2853 ], [ %v2598420857, %edge2715 ], [ %v27450, %block19614.thread21246 ], [ %v27450, %edge2885 ], [ 0, %edge2885.thread ], [ %v19847, %block913 ]
  %v217787 = phi i64 [ %v25141, %edge2732 ], [ %v26101559, %block19614 ], [ %v25141, %block18825 ], [ %v25141, %edge2683 ], [ %v26101559, %edge2902 ], [ %v26101559, %edge2853 ], [ %v25141, %edge2715 ], [ %v26101559, %block19614.thread21246 ], [ %v26101559, %edge2885 ], [ %v26101559, %edge2885.thread ], [ %v19854, %block913 ]
  %v217788 = phi i64 [ %v25143, %edge2732 ], [ %v26103560, %block19614 ], [ %v25143, %block18825 ], [ %v25143, %edge2683 ], [ %v26103560, %edge2902 ], [ %v26103560, %edge2853 ], [ %v25143, %edge2715 ], [ %v26103560, %block19614.thread21246 ], [ %v26103560, %edge2885 ], [ %v26103560, %edge2885.thread ], [ %v19856, %block913 ]
  %v217789 = phi i64 [ %v25146, %edge2732 ], [ %v26106561, %block19614 ], [ %v25146, %block18825 ], [ %v25146, %edge2683 ], [ %v26106561, %edge2902 ], [ %v26106561, %edge2853 ], [ %v25146, %edge2715 ], [ %v26106561, %block19614.thread21246 ], [ %v26106561, %edge2885 ], [ %v26106561, %edge2885.thread ], [ %v19859, %block913 ]
  %v217790 = phi i64 [ %spec.select19, %edge2732 ], [ %spec.select19562, %block19614 ], [ %spec.select19, %block18825 ], [ %spec.select19, %edge2683 ], [ %spec.select19562, %edge2902 ], [ %spec.select19562, %edge2853 ], [ %spec.select19, %edge2715 ], [ %spec.select19562, %block19614.thread21246 ], [ %spec.select19562, %edge2885 ], [ %spec.select19562, %edge2885.thread ], [ %v19866, %block913 ]
  br i1 %t23.not, label %edge3980, label %edge2952

block1421:                                        ; preds = %edge3706, %edge3105
  %v28541 = phi i64 [ %v18873, %edge3105 ], [ %v35323, %edge3706 ]
  %v28543 = phi i64 [ %v18875, %edge3105 ], [ %v35325, %edge3706 ]
  %v28546 = phi i64 [ %v18878, %edge3105 ], [ %v35328, %edge3706 ]
  %v28574 = phi i64 [ %v19004, %edge3105 ], [ %v35292, %edge3706 ]
  %v28577 = phi i64 [ %v19009, %edge3105 ], [ %v35295, %edge3706 ]
  %v28580 = phi i64 [ %v18769, %edge3105 ], [ %v35371, %edge3706 ]
  %v28583 = phi i64 [ 300, %edge3105 ], [ %v35567, %edge3706 ]
  %v28586 = phi i64 [ 0, %edge3105 ], [ %v35298, %edge3706 ]
  %v28589 = phi i64 [ 0, %edge3105 ], [ %v35301, %edge3706 ]
  %v28592 = phi i64 [ 0, %edge3105 ], [ %spec.select21, %edge3706 ]
  %t2406 = icmp ult i64 %v28592, %spec.select17
  br i1 %t2406, label %edge3132, label %block23952

trap2434:                                         ; preds = %edge3191
  call void @llvm.trap()
  unreachable

ok2434:                                           ; preds = %edge3191
  %v29809 = udiv i64 %v29612, %v1397
  %v29810 = mul i64 %v29809, %v1397
  %v29811 = sub i64 %v29612, %v29810
  %v29812 = mul i64 %v29811, %v1398
  %v29813 = add i64 %v29812, %v29809
  br label %block20808

block20808:                                       ; preds = %edge3186, %ok2434
  %v29808 = phi i64 [ %v29813, %ok2434 ], [ %v29612, %edge3186 ]
  %v29815 = add i64 %v29808, %v1394
  %t2435 = inttoptr i64 %v29815 to ptr
  %t2436 = load i8, ptr %t2435, align 1
  %v29816 = zext i8 %t2436 to i64
  br label %edge3184

trap2444:                                         ; preds = %edge3210
  call void @llvm.trap()
  unreachable

ok2444:                                           ; preds = %edge3210
  %v30035 = udiv i64 %v29838, %v1397
  %v30036 = mul i64 %v30035, %v1397
  %v30037 = sub i64 %v29838, %v30036
  %v30038 = mul i64 %v30037, %v1398
  %v30039 = add i64 %v30038, %v30035
  br label %block20951

block20951:                                       ; preds = %edge3205, %ok2444
  %v30034 = phi i64 [ %v30039, %ok2444 ], [ %v29838, %edge3205 ]
  %v30041 = add i64 %v30034, %v1394
  %t2445 = inttoptr i64 %v30041 to ptr
  %t2446 = load i8, ptr %t2445, align 1
  %v30042 = zext i8 %t2446 to i64
  br label %edge3203

block21296:                                       ; preds = %edge3257, %edge3254
  %t2459.not = icmp eq i64 %v217710, 0
  br i1 %t2459.not, label %block21300, label %edge3263

block21300:                                       ; preds = %edge3263, %block21296
  %t2461.not = icmp eq i64 %v217725, 0
  br i1 %t2461.not, label %block21304, label %edge3269

block21304:                                       ; preds = %edge3269, %block21300
  %t2463.not = icmp eq i64 %v217738, 0
  br i1 %t2463.not, label %block21308, label %edge3275

block21308:                                       ; preds = %edge3275, %block21304
  %t2465.not = icmp eq i64 %v217744, 0
  br i1 %t2465.not, label %block21312, label %edge3281

block21312:                                       ; preds = %edge3281, %block21308
  %t2467.not = icmp eq i64 %v217754, 0
  br i1 %t2467.not, label %block21320, label %edge3287

block21320:                                       ; preds = %block21312, %edge3287
  br i1 %t23.not, label %block21324, label %edge3299

block21324:                                       ; preds = %edge3299, %block21320
  %t2473.not = icmp eq i64 %v10789.ph.ph, 0
  br i1 %t2473.not, label %block21328, label %edge3305

block21328:                                       ; preds = %edge3305, %block21324
  %t2475.not = icmp eq i64 %v10790, 0
  br i1 %t2475.not, label %block21332, label %edge3311

block21332:                                       ; preds = %edge3311, %block21328
  br i1 %t3136.not, label %block21336, label %edge3317

block21336:                                       ; preds = %edge3317, %block21332
  %t2479.not = icmp eq i64 %v10792, 0
  br i1 %t2479.not, label %block21340, label %edge3323

block21340:                                       ; preds = %edge3323, %block21336
  %t2481.not = icmp eq i64 %v18649, 0
  br i1 %t2481.not, label %block21344, label %edge3329

block21344:                                       ; preds = %edge3329, %block21340
  %t2483.not = icmp eq i64 %v28546, 0
  br i1 %t2483.not, label %block21348, label %edge3335

block21348:                                       ; preds = %edge3335, %block21344
  br i1 %t23.not, label %common.ret, label %edge3341

block21228:                                       ; preds = %edge3341, %edge3335, %edge3329, %edge3323, %edge3317, %edge3311, %edge3305, %edge3299, %edge3287, %edge3281, %edge3275, %edge3269, %edge3263, %edge3257
  %v30460 = phi i64 [ %v30582, %edge3335 ], [ %v30530, %edge3257 ], [ %v30534, %edge3263 ], [ %v30538, %edge3269 ], [ %v30542, %edge3275 ], [ %v30546, %edge3281 ], [ %v30550, %edge3287 ], [ %v30586, %edge3341 ], [ %v30558, %edge3299 ], [ %v30562, %edge3305 ], [ %v30566, %edge3311 ], [ %v30570, %edge3317 ], [ %v30574, %edge3323 ], [ %v30578, %edge3329 ]
  %t2487 = insertvalue { i64, i64 } poison, i64 %v30460, 0
  %t2488 = insertvalue { i64, i64 } %t2487, i64 1544, 1
  br label %common.ret

block1561:                                        ; preds = %edge3368, %edge3458
  %v30902 = phi i64 [ %v31765, %edge3458 ], [ %v30647, %edge3368 ]
  %t2501.not = icmp ult i64 %v30902, %spec.select17
  br i1 %t2501.not, label %edge3406, label %edge3392

trap2518:                                         ; preds = %edge3419
  call void @llvm.trap()
  unreachable

ok2518:                                           ; preds = %edge3419
  %v31319 = udiv i64 %v31113, %v1397
  %v31320 = mul i64 %v31319, %v1397
  %v31321 = sub nuw i64 %v31113, %v31320
  %v31322 = mul i64 %v31321, %v1398
  %v31323 = add i64 %v31322, %v31319
  br label %block21829

block21829:                                       ; preds = %edge3414, %ok2518
  %v31318 = phi i64 [ %v31323, %ok2518 ], [ %v31113, %edge3414 ]
  %v31325 = add i64 %v31318, %v1394
  %t2519 = inttoptr i64 %v31325 to ptr
  %t2520 = load i8, ptr %t2519, align 1
  %v31326 = zext i8 %t2520 to i64
  br label %edge3412

trap2528:                                         ; preds = %edge3438
  call void @llvm.trap()
  unreachable

ok2528:                                           ; preds = %edge3438
  %v31552 = udiv i64 %v31350, %v1397
  %v31553 = mul i64 %v31552, %v1397
  %v31554 = sub nuw i64 %v31350, %v31553
  %v31555 = mul i64 %v31554, %v1398
  %v31556 = add i64 %v31555, %v31552
  br label %block21976

block21976:                                       ; preds = %edge3433, %ok2528
  %v31551 = phi i64 [ %v31556, %ok2528 ], [ %v31350, %edge3433 ]
  %v31558 = add i64 %v31551, %v1394
  %t2529 = inttoptr i64 %v31558 to ptr
  %t2530 = load i8, ptr %t2529, align 1
  %v31559 = zext i8 %t2530 to i64
  br label %edge3431

block23072:                                       ; preds = %edge3571.thread, %edge3571
  %v34326615617 = phi i64 [ %v34326, %edge3571 ], [ %v34284, %edge3571.thread ]
  %t2604 = zext i64 %v28541 to i128
  %v218310 = or disjoint i128 %t2604, 4835703278458516698824704
  store i128 %v218310, ptr %ss0, align 16
  %t2608 = zext i64 %v28543 to i128
  %v218312 = or disjoint i128 %t2608, 18446744073709551616
  store i128 %v218312, ptr %t1002, align 16
  %t2614 = zext i64 %v28546 to i128
  %t2615 = shl nuw i128 %t2614, 64
  %v218314 = or disjoint i128 %t2615, 262144
  store i128 %v218314, ptr %t1007, align 16
  %v34353 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v28543, i64 2, i64 2, i64 %v3192)
  %t2618.not = icmp eq i64 %v34353, 0
  br i1 %t2618.not, label %block23070, label %block23047

block23070.thread:                                ; preds = %edge3571, %edge3571.thread
  %t2624.not7020924 = icmp eq i64 %v28546, 0
  br label %block23076

block23070:                                       ; preds = %block23072
  %v34356 = load i64, ptr %t607, align 8
  %v34355 = load i64, ptr %t606, align 8
  %v34354 = load i64, ptr %ss1, align 8
  %v34357 = add i64 %v34354, %v34326615617
  %t2621 = inttoptr i64 %v34357 to ptr
  store i16 %t2569, ptr %t2621, align 1
  %t2622 = icmp eq i64 %v28546, %v34356
  %t2624.not70 = icmp eq i64 %v34356, 0
  %t2624.not = or i1 %t2622, %t2624.not70
  br i1 %t2624.not, label %block23076, label %edge3583

block23076:                                       ; preds = %block23070.thread, %block23070, %edge3583
  %t2624.not7020932 = phi i1 [ %t2624.not7020924, %block23070.thread ], [ %t2624.not70, %block23070 ], [ %t2624.not70, %edge3583 ]
  %t262220931 = phi i1 [ true, %block23070.thread ], [ %t2622, %block23070 ], [ %t2622, %edge3583 ]
  %v3433720929 = phi i64 [ %v28546, %block23070.thread ], [ %v34356, %block23070 ], [ %v34356, %edge3583 ]
  %v3433420928 = phi i64 [ %v28543, %block23070.thread ], [ %v34355, %block23070 ], [ %v34355, %edge3583 ]
  %v3433220927 = phi i64 [ %v28541, %block23070.thread ], [ %v34354, %block23070 ], [ %v34354, %edge3583 ]
  %t2628.not = or i1 %t2578.not, %t262220931
  br i1 %t2628.not, label %edge3594, label %edge3588

block23047:                                       ; preds = %edge3588, %block23072
  %v34274 = phi i64 [ %v34353, %block23072 ], [ %v34367, %edge3588 ]
  %t2630 = insertvalue { i64, i64 } poison, i64 %v34274, 0
  %t2631 = insertvalue { i64, i64 } %t2630, i64 1799, 1
  br label %common.ret

block23087:                                       ; preds = %edge3598, %edge3597
  %t2643 = zext i64 %v3433220927 to i128
  %v218316 = or disjoint i128 %t2643, 4835703278458516698824704
  store i128 %v218316, ptr %ss0, align 16
  %t2647 = zext i64 %v3433420928 to i128
  %v218318 = or disjoint i128 %t2647, 18446744073709551616
  store i128 %v218318, ptr %t1002, align 16
  %t2653 = zext i64 %v3433720929 to i128
  %t2654 = shl nuw i128 %t2653, 64
  %v218320 = or disjoint i128 %t2654, 262144
  store i128 %v218320, ptr %t1007, align 16
  %v34431 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v3433420928, i64 2, i64 2, i64 %v3192)
  %t2657.not = icmp eq i64 %v34431, 0
  br i1 %t2657.not, label %edge3603, label %block23082

block23085:                                       ; preds = %edge3603, %edge3598
  %t2660.pre-phi = phi ptr [ %t2634, %edge3598 ], [ %.pre20738, %edge3603 ]
  %v34416 = phi i64 [ %v3433220927, %edge3598 ], [ %v34432, %edge3603 ]
  %v34418 = phi i64 [ %v3433420928, %edge3598 ], [ %v34433, %edge3603 ]
  %v34421 = phi i64 [ %v3433720929, %edge3598 ], [ %v34434, %edge3603 ]
  store i16 %t2575, ptr %t2660.pre-phi, align 1
  %t2661 = icmp eq i64 %v3433720929, %v34421
  %t2663.not72 = icmp eq i64 %v34421, 0
  %t2663.not = or i1 %t2661, %t2663.not72
  br i1 %t2663.not, label %block23091, label %edge3606

block23091:                                       ; preds = %block23085, %edge3606
  %t2667.not = or i1 %t2624.not7020932, %t2661
  br i1 %t2667.not, label %block1793, label %edge3611

block23082:                                       ; preds = %edge3611, %block23087
  %v34398 = phi i64 [ %v34431, %block23087 ], [ %v34445, %edge3611 ]
  %t2669 = insertvalue { i64, i64 } poison, i64 %v34398, 0
  %t2670 = insertvalue { i64, i64 } %t2669, i64 1801, 1
  br label %common.ret

block1793:                                        ; preds = %edge3594, %block23091, %edge3611, %edge3562
  %v34481 = phi i64 [ %v28541, %edge3562 ], [ %v34416, %edge3611 ], [ %v34416, %block23091 ], [ %v3433220927, %edge3594 ]
  %v34483 = phi i64 [ %v28543, %edge3562 ], [ %v34418, %edge3611 ], [ %v34418, %block23091 ], [ %v3433420928, %edge3594 ]
  %v34486 = phi i64 [ %v28546, %edge3562 ], [ %v34421, %edge3611 ], [ %v34421, %block23091 ], [ %v3433720929, %edge3594 ]
  %v34461 = shl nuw nsw i64 %v3356121075, 16
  %v34478 = or i64 %v34461, %v29004
  br label %block23952

block1799:                                        ; preds = %edge3539.block1799_crit_edge, %edge3203
  %v34558.pre-phi = phi i64 [ %.pre20758, %edge3539.block1799_crit_edge ], [ %v29612, %edge3203 ]
  %v217331 = phi i64 [ %v33561, %edge3539.block1799_crit_edge ], [ %v28592, %edge3203 ]
  %t2672 = icmp ult i64 %v34558.pre-phi, %v1395
  br i1 %t2672, label %edge3629, label %edge3627

trap2679:                                         ; preds = %edge3634
  call void @llvm.trap()
  unreachable

ok2679:                                           ; preds = %edge3634
  %v34767 = udiv i64 %v34558.pre-phi, %v1397
  %v34768 = mul i64 %v34767, %v1397
  %v34769 = sub i64 %v34558.pre-phi, %v34768
  %v34770 = mul i64 %v34769, %v1398
  %v34771 = add i64 %v34770, %v34767
  br label %block23247

block23247:                                       ; preds = %edge3629, %ok2679
  %v34766 = phi i64 [ %v34771, %ok2679 ], [ %v34558.pre-phi, %edge3629 ]
  %v34773 = add i64 %v34766, %v1394
  %t2680 = inttoptr i64 %v34773 to ptr
  %t2681 = load i8, ptr %t2680, align 1
  %v34774 = zext i8 %t2681 to i64
  br label %edge3627

trap2689:                                         ; preds = %edge3653
  call void @llvm.trap()
  unreachable

ok2689:                                           ; preds = %edge3653
  %v35043 = udiv i64 %v34846, %v1397
  %v35044 = mul i64 %v35043, %v1397
  %v35045 = sub i64 %v34846, %v35044
  %v35046 = mul i64 %v35045, %v1398
  %v35047 = add i64 %v35046, %v35043
  br label %edge3646

block23549:                                       ; preds = %edge3687, %edge3686
  %t2706 = zext i64 %v28541 to i128
  %v218322 = or disjoint i128 %t2706, 4835703278458516698824704
  store i128 %v218322, ptr %ss0, align 16
  %t2710 = zext i64 %v28543 to i128
  %v218324 = or disjoint i128 %t2710, 18446744073709551616
  store i128 %v218324, ptr %t1002, align 16
  %t2716 = zext i64 %v28546 to i128
  %t2717 = shl nuw i128 %t2716, 64
  %v218326 = or disjoint i128 %t2717, 262144
  store i128 %v218326, ptr %t1007, align 16
  %v35344 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v28543, i64 2, i64 2, i64 %v3192)
  %t2720.not = icmp eq i64 %v35344, 0
  br i1 %t2720.not, label %edge3692, label %block23544

block23548:                                       ; preds = %edge3687, %edge3692
  %t2723.pre-phi = phi ptr [ %t2697, %edge3687 ], [ %.pre20736, %edge3692 ]
  %v35329 = phi i64 [ %v28541, %edge3687 ], [ %v35345, %edge3692 ]
  %v35331 = phi i64 [ %v28543, %edge3687 ], [ %v35346, %edge3692 ]
  %v35334 = phi i64 [ %v28546, %edge3687 ], [ %v35347, %edge3692 ]
  %v35349 = trunc nuw i64 %v28580 to i16
  store i16 %v35349, ptr %t2723.pre-phi, align 1
  br label %block23547

block23547:                                       ; preds = %edge3683, %block23548
  %v35323 = phi i64 [ %v35329, %block23548 ], [ %v28541, %edge3683 ]
  %v35325 = phi i64 [ %v35331, %block23548 ], [ %v28543, %edge3683 ]
  %v35328 = phi i64 [ %v35334, %block23548 ], [ %v28546, %edge3683 ]
  %t2724 = icmp eq i64 %v28546, %v35328
  %t2726.not63 = icmp eq i64 %v35328, 0
  %t2726.not = or i1 %t2724, %t2726.not63
  br i1 %t2726.not, label %block23553, label %edge3695

block23553:                                       ; preds = %block23547, %edge3695
  %t2730.not64 = icmp eq i64 %v28546, 0
  %t2730.not = or i1 %t2730.not64, %t2724
  br i1 %t2730.not, label %edge3706, label %edge3700

block23544:                                       ; preds = %edge3700, %block23549
  %v35311 = phi i64 [ %v35344, %block23549 ], [ %v35358, %edge3700 ]
  %t2732 = insertvalue { i64, i64 } poison, i64 %v35311, 0
  %t2733 = insertvalue { i64, i64 } %t2732, i64 1859, 1
  br label %common.ret

block23855:                                       ; preds = %edge3742.thread, %edge3742
  %v35831633635 = phi i64 [ %v35831, %edge3742 ], [ %v35789, %edge3742.thread ]
  %t2773 = zext i64 %v35323 to i128
  %v218328 = or disjoint i128 %t2773, 4835703278458516698824704
  store i128 %v218328, ptr %ss0, align 16
  %t2777 = zext i64 %v35325 to i128
  %v218330 = or disjoint i128 %t2777, 18446744073709551616
  store i128 %v218330, ptr %t1002, align 16
  %t2783 = zext i64 %v35328 to i128
  %t2784 = shl nuw i128 %t2783, 64
  %v218332 = or disjoint i128 %t2784, 262144
  store i128 %v218332, ptr %t1007, align 16
  %v35858 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v35325, i64 2, i64 2, i64 %v3192)
  %t2787.not = icmp eq i64 %v35858, 0
  br i1 %t2787.not, label %block23854, label %block23830

block23854:                                       ; preds = %block23855
  %v35861 = load i64, ptr %t607, align 8
  %v35860 = load i64, ptr %t606, align 8
  %v35859 = load i64, ptr %ss1, align 8
  %v35862 = add i64 %v35859, %v35831633635
  %t2790 = inttoptr i64 %v35862 to ptr
  store i16 0, ptr %t2790, align 1
  br label %block23853

block23853:                                       ; preds = %edge3742.thread, %block23854
  %v35837 = phi i64 [ %v35859, %block23854 ], [ %v35323, %edge3742.thread ]
  %v35839 = phi i64 [ %v35860, %block23854 ], [ %v35325, %edge3742.thread ]
  %v35842 = phi i64 [ %v35861, %block23854 ], [ %v35328, %edge3742.thread ]
  %t2791 = icmp eq i64 %v35328, %v35842
  %t2793.not66 = icmp eq i64 %v35842, 0
  %t2793.not = or i1 %t2791, %t2793.not66
  br i1 %t2793.not, label %block23859, label %edge3754

block23859:                                       ; preds = %block23853, %edge3754
  %t2797.not = or i1 %t2726.not63, %t2791
  br i1 %t2797.not, label %edge3765, label %edge3759

block23830:                                       ; preds = %edge3759, %block23855
  %v35779 = phi i64 [ %v35858, %block23855 ], [ %v35872, %edge3759 ]
  %t2799 = insertvalue { i64, i64 } poison, i64 %v35779, 0
  %t2800 = insertvalue { i64, i64 } %t2799, i64 1881, 1
  br label %common.ret

block23870:                                       ; preds = %edge3765.thread, %edge3769, %edge3768
  %v3583720957209722108521103 = phi i64 [ %v35837, %edge3768 ], [ %v35837, %edge3769 ], [ %v35323, %edge3765.thread ]
  %v3583920958209712108621102 = phi i64 [ %v35839, %edge3768 ], [ %v35839, %edge3769 ], [ %v35325, %edge3765.thread ]
  %v3584220959209702108721100 = phi i64 [ %v35842, %edge3768 ], [ %v35842, %edge3769 ], [ 0, %edge3765.thread ]
  %t2793.not6620962209692108821099 = phi i1 [ true, %edge3768 ], [ false, %edge3769 ], [ true, %edge3765.thread ]
  %v358982108921097 = phi i64 [ %v35898, %edge3768 ], [ %v35898, %edge3769 ], [ %v3589821080, %edge3765.thread ]
  %t2812 = zext i64 %v3583720957209722108521103 to i128
  %v218334 = or disjoint i128 %t2812, 4835703278458516698824704
  store i128 %v218334, ptr %ss0, align 16
  %t2816 = zext i64 %v3583920958209712108621102 to i128
  %v218336 = or disjoint i128 %t2816, 18446744073709551616
  store i128 %v218336, ptr %t1002, align 16
  %t2822 = zext i64 %v3584220959209702108721100 to i128
  %t2823 = shl nuw i128 %t2822, 64
  %v218338 = or disjoint i128 %t2823, 262144
  store i128 %v218338, ptr %t1007, align 16
  %v35925 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v3583920958209712108621102, i64 2, i64 2, i64 %v3192)
  %t2826.not = icmp eq i64 %v35925, 0
  br i1 %t2826.not, label %block23868, label %block23865

block23868:                                       ; preds = %block23870
  %v35926 = load i64, ptr %ss1, align 8
  %v35927 = load i64, ptr %t606, align 8
  %v35928 = load i64, ptr %t607, align 8
  %.pre20733 = add i64 %v35926, %v358982108921097
  %.pre20734 = inttoptr i64 %.pre20733 to ptr
  store i16 0, ptr %.pre20734, align 1
  %t2830 = icmp eq i64 %v3584220959209702108721100, %v35928
  %t2832.not68 = icmp eq i64 %v35928, 0
  %t2832.not = or i1 %t2830, %t2832.not68
  br i1 %t2832.not, label %block23874, label %edge3777

block23874:                                       ; preds = %block23868, %edge3777
  %t2836.not = or i1 %t2793.not6620962209692108821099, %t2830
  br i1 %t2836.not, label %block23952, label %edge3782

block23865:                                       ; preds = %edge3782, %block23870
  %v35892 = phi i64 [ %v35925, %block23870 ], [ %v35939, %edge3782 ]
  %t2838 = insertvalue { i64, i64 } poison, i64 %v35892, 0
  %t2839 = insertvalue { i64, i64 } %t2838, i64 1883, 1
  br label %common.ret

block23952:                                       ; preds = %block1421, %block23874.thread21265, %edge3765.thread, %edge3765, %edge3733, %edge3782, %block23874, %block1793
  %v217792 = phi i64 [ %v35926, %block23874 ], [ %v34481, %block1793 ], [ %v35926, %edge3782 ], [ %v35323, %edge3733 ], [ %v35837, %block23874.thread21265 ], [ %v35837, %edge3765 ], [ %v35323, %edge3765.thread ], [ %v28541, %block1421 ]
  %v217793 = phi i64 [ %v35927, %block23874 ], [ %v34483, %block1793 ], [ %v35927, %edge3782 ], [ %v35325, %edge3733 ], [ %v35839, %block23874.thread21265 ], [ %v35839, %edge3765 ], [ %v35325, %edge3765.thread ], [ %v28543, %block1421 ]
  %v217794 = phi i64 [ %v35928, %block23874 ], [ %v34486, %block1793 ], [ %v35928, %edge3782 ], [ %v35328, %edge3733 ], [ %v35842, %block23874.thread21265 ], [ %v35842, %edge3765 ], [ 0, %edge3765.thread ], [ %v28546, %block1421 ]
  %v217796 = phi i64 [ 0, %block23874 ], [ %v34478, %block1793 ], [ 0, %edge3782 ], [ 0, %edge3733 ], [ 0, %block23874.thread21265 ], [ 0, %edge3765 ], [ 0, %edge3765.thread ], [ 0, %block1421 ]
  br i1 %t23.not, label %edge3980, label %edge3826

block25006:                                       ; preds = %edge4057, %edge4056
  %t2924 = zext i64 %v10829 to i128
  %v218340 = or disjoint i128 %t2924, 184467440737095516160
  store i128 %v218340, ptr %ss0, align 16
  %t2928 = zext i64 %v10804 to i128
  %v218342 = or disjoint i128 %t2928, 18446744073709551616
  store i128 %v218342, ptr %t1002, align 16
  %t2934 = zext i64 %v10792 to i128
  %t2935 = shl nuw i128 %t2934, 64
  %v218344 = or disjoint i128 %t2935, 10
  store i128 %v218344, ptr %t1007, align 16
  %v38822 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v10804, i64 4, i64 4, i64 %v3192)
  %t2938.not = icmp eq i64 %v38822, 0
  br i1 %t2938.not, label %edge4062, label %block25001

block25004:                                       ; preds = %edge4062, %edge4057
  %t2941.pre-phi = phi ptr [ %.pre20730, %edge4062 ], [ %t2907, %edge4057 ]
  %v38807 = phi i64 [ %v38823, %edge4062 ], [ %v10829, %edge4057 ]
  %v38809 = phi i64 [ %v38824, %edge4062 ], [ %v10804, %edge4057 ]
  %v38812 = phi i64 [ %v38825, %edge4062 ], [ %v10792, %edge4057 ]
  store i32 %16, ptr %t2941.pre-phi, align 1
  %t2942 = icmp eq i64 %v10792, %v38812
  %t2944.not96 = icmp eq i64 %v38812, 0
  %t2944.not = or i1 %t2942, %t2944.not96
  br i1 %t2944.not, label %block25010, label %edge4065

block25010:                                       ; preds = %block25004, %edge4065
  %t2948.not = or i1 %t2919.not, %t2942
  br i1 %t2948.not, label %edge4074, label %edge4070

block25001:                                       ; preds = %edge4070, %block25006
  %v38789 = phi i64 [ %v38822, %block25006 ], [ %v38836, %edge4070 ]
  %t2950 = insertvalue { i64, i64 } poison, i64 %v38789, 0
  %t2951 = insertvalue { i64, i64 } %t2950, i64 2000, 1
  br label %common.ret

block25164:                                       ; preds = %edge4102, %edge4101
  %t2969 = zext i64 %v10827 to i128
  %v218346 = or disjoint i128 %t2969, 4777706715090773868544
  store i128 %v218346, ptr %ss0, align 16
  %t2973 = zext i64 %v10800 to i128
  %v218348 = or disjoint i128 %t2973, 18446744073709551616
  store i128 %v218348, ptr %t1002, align 16
  %t2979 = zext i64 %v10790 to i128
  %t2980 = shl nuw i128 %t2979, 64
  %v218350 = or disjoint i128 %t2980, 259
  store i128 %v218350, ptr %t1007, align 16
  %v39112 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v10800, i64 4, i64 4, i64 %v3192)
  %t2983.not = icmp eq i64 %v39112, 0
  br i1 %t2983.not, label %edge4107, label %block25159

block25162:                                       ; preds = %edge4107, %edge4102
  %t2986.pre-phi = phi ptr [ %.pre20732, %edge4107 ], [ %t2955, %edge4102 ]
  %v39097 = phi i64 [ %v39113, %edge4107 ], [ %v10827, %edge4102 ]
  %v39099 = phi i64 [ %v39114, %edge4107 ], [ %v10800, %edge4102 ]
  %v39102 = phi i64 [ %v39115, %edge4107 ], [ %v10790, %edge4102 ]
  store i32 %17, ptr %t2986.pre-phi, align 1
  %t2987 = icmp eq i64 %v10790, %v39102
  %t2989.not98 = icmp eq i64 %v39102, 0
  %t2989.not = or i1 %t2987, %t2989.not98
  br i1 %t2989.not, label %block25168, label %edge4110

block25168:                                       ; preds = %block25162, %edge4110
  %t2993.not = or i1 %t2964.not, %t2987
  br i1 %t2993.not, label %edge4214, label %edge4113

block25159:                                       ; preds = %edge4074, %edge4116, %block25164
  %v39079 = phi i64 [ %v39112, %block25164 ], [ 103425, %edge4074 ], [ %v39126, %edge4116 ]
  %t2998 = insertvalue { i64, i64 } poison, i64 %v39079, 0
  %t2999 = insertvalue { i64, i64 } %t2998, i64 2020, 1
  br label %common.ret

trap3007:                                         ; preds = %edge4139
  call void @llvm.trap()
  unreachable

ok3007:                                           ; preds = %edge4139
  %v39378 = udiv i64 %v217293, %v1397
  %v39379 = mul i64 %v39378, %v1397
  %v39380 = sub i64 %v217293, %v39379
  %v39381 = mul i64 %v39380, %v1398
  %v39382 = add i64 %v39381, %v39378
  br label %block25325

block25325:                                       ; preds = %edge4134, %ok3007
  %v39377 = phi i64 [ %v39382, %ok3007 ], [ %v217293, %edge4134 ]
  %v39384 = add i64 %v39377, %v1394
  %t3008 = inttoptr i64 %v39384 to ptr
  %t3009 = load i8, ptr %t3008, align 1
  %v39385 = zext i8 %t3009 to i64
  br label %edge4132

block25482:                                       ; preds = %edge4170, %edge4132
  %t3028 = zext i64 %v10829 to i128
  %v218352 = or disjoint i128 %t3028, 184467440737095516160
  store i128 %v218352, ptr %ss0, align 16
  %t3032 = zext i64 %v10804 to i128
  %v218354 = or disjoint i128 %t3032, 18446744073709551616
  store i128 %v218354, ptr %t1002, align 16
  %t3038 = zext i64 %v10792 to i128
  %t3039 = shl nuw i128 %t3038, 64
  %v218356 = or disjoint i128 %t3039, 10
  store i128 %v218356, ptr %t1007, align 16
  %v39906 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v10804, i64 4, i64 4, i64 %v3192)
  %t3042.not = icmp eq i64 %v39906, 0
  br i1 %t3042.not, label %edge4175, label %block25477

block25481:                                       ; preds = %edge4170, %edge4175
  %t3045.pre-phi = phi ptr [ %t3015, %edge4170 ], [ %.pre20728, %edge4175 ]
  %v39891 = phi i64 [ %v10829, %edge4170 ], [ %v39907, %edge4175 ]
  %v39893 = phi i64 [ %v10804, %edge4170 ], [ %v39908, %edge4175 ]
  %v39896 = phi i64 [ %v10792, %edge4170 ], [ %v39909, %edge4175 ]
  store i32 %v39853, ptr %t3045.pre-phi, align 1
  %t3046 = icmp eq i64 %v10792, %v39896
  %t3048.not94 = icmp eq i64 %v39896, 0
  %t3048.not = or i1 %t3046, %t3048.not94
  br i1 %t3048.not, label %block25486, label %edge4178

block25486:                                       ; preds = %block25481, %edge4178
  %t3052.not = or i1 %t3023.not, %t3046
  br i1 %t3052.not, label %edge4187, label %edge4183

block25477:                                       ; preds = %edge4183, %block25482
  %v39873 = phi i64 [ %v39906, %block25482 ], [ %v39920, %edge4183 ]
  %t3054 = insertvalue { i64, i64 } poison, i64 %v39873, 0
  %t3055 = insertvalue { i64, i64 } %t3054, i64 2059, 1
  br label %common.ret

trap3063:                                         ; preds = %edge4227
  call void @llvm.trap()
  unreachable

ok3063:                                           ; preds = %edge4227
  %v40254 = udiv i64 %v217293, %v1397
  %v40255 = mul i64 %v40254, %v1397
  %v40256 = sub i64 %v217293, %v40255
  %v40257 = mul i64 %v40256, %v1398
  %v40258 = add i64 %v40257, %v40254
  br label %block25694

block25694:                                       ; preds = %edge4222, %ok3063
  %v40253 = phi i64 [ %v40258, %ok3063 ], [ %v217293, %edge4222 ]
  %v40260 = add i64 %v40253, %v1394
  %t3064 = inttoptr i64 %v40260 to ptr
  %t3065 = load i8, ptr %t3064, align 1
  %v40261 = zext i8 %t3065 to i64
  %1 = shl nuw nsw i64 %v40261, 16
  br label %edge4220

block25712:                                       ; preds = %edge4245, %edge4244
  %t3078 = zext i64 %v36635652 to i128
  %v218358 = or disjoint i128 %t1719, %t3078
  store i128 %v218358, ptr %ss0, align 16
  %t3082 = zext i64 %v36637653 to i128
  %v218360 = or disjoint i128 %t3082, 18446744073709551616
  store i128 %v218360, ptr %t1002, align 16
  %t3088 = zext i64 %v36640654 to i128
  %t3089 = shl nuw i128 %t3088, 64
  %v218362 = or disjoint i128 %t3089, %t1163
  store i128 %v218362, ptr %t1007, align 16
  %v40349 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v36637653, i64 4, i64 4, i64 %v3192)
  %t3092.not = icmp eq i64 %v40349, 0
  br i1 %t3092.not, label %edge4250, label %block25707

block25710:                                       ; preds = %edge4250, %edge4245
  %t3095.pre-phi = phi ptr [ %.pre20726, %edge4250 ], [ %t3069, %edge4245 ]
  %v40334 = phi i64 [ %v40350, %edge4250 ], [ %v36635652, %edge4245 ]
  %v40336 = phi i64 [ %v40351, %edge4250 ], [ %v36637653, %edge4245 ]
  %v40339 = phi i64 [ %v40352, %edge4250 ], [ %v36640654, %edge4245 ]
  %v40354 = trunc nuw nsw i64 %v40298 to i32
  store i32 %v40354, ptr %t3095.pre-phi, align 1
  %t3096 = icmp eq i64 %v36640654, %v40339
  %t3098.not100 = icmp eq i64 %v40339, 0
  %t3098.not = or i1 %t3096, %t3098.not100
  br i1 %t3098.not, label %block25716, label %edge4253

block25716:                                       ; preds = %block25710, %edge4253
  %t3102.not = or i1 %t3098.not1001116, %t3096
  br i1 %t3102.not, label %edge4262, label %edge4258

block25707:                                       ; preds = %edge4258, %block25712, %edge4220
  %v40316 = phi i64 [ %v40349, %block25712 ], [ 103425, %edge4220 ], [ %v40363, %edge4258 ]
  %t3104 = insertvalue { i64, i64 } poison, i64 %v40316, 0
  %t3105 = insertvalue { i64, i64 } %t3104, i64 2094, 1
  br label %common.ret

block26690:                                       ; preds = %edge4360, %edge4361
  br i1 %t3136.not, label %block2176.preheader, label %edge4364

block2176.preheader:                              ; preds = %edge4364, %block26690
  br label %block2176

block2176:                                        ; preds = %block2176.preheader, %edge4449
  %v42744 = phi i64 [ %v44060, %edge4449 ], [ 0, %block2176.preheader ]
  %v42747 = phi i64 [ %v44068, %edge4449 ], [ 0, %block2176.preheader ]
  %exitcond20579.not = icmp eq i64 %v42744, 10
  br i1 %exitcond20579.not, label %edge4432, label %edge4387

block27444:                                       ; preds = %edge4435, %edge4432
  br i1 %t3136.not, label %edge4464, label %edge4439

block27440:                                       ; preds = %edge4442, %edge4435
  %v43882 = phi i64 [ %v43889, %edge4435 ], [ %v43893, %edge4442 ]
  %t3183 = insertvalue { i64, i64 } poison, i64 %v43882, 0
  %t3184 = insertvalue { i64, i64 } %t3183, i64 2255, 1
  br label %common.ret

block2305:                                        ; preds = %block2305.backedge, %block2305.preheader
  %v45176 = phi i64 [ %v40042, %block2305.preheader ], [ %v45896, %block2305.backedge ]
  %v45178 = phi i64 [ %v40044, %block2305.preheader ], [ %v45898, %block2305.backedge ]
  %v45181 = phi i64 [ %v40047, %block2305.preheader ], [ %v45901, %block2305.backedge ]
  %v45191 = phi i64 [ %v10828.ph.ph, %block2305.preheader ], [ %v45833, %block2305.backedge ]
  %v45193 = phi i64 [ %v10802.ph.ph, %block2305.preheader ], [ %v45835, %block2305.backedge ]
  %v45196 = phi i64 [ %v10791.ph.ph, %block2305.preheader ], [ %v45838, %block2305.backedge ]
  %v45203 = phi i64 [ 0, %block2305.preheader ], [ %v45567, %block2305.backedge ]
  %exitcond20580.not = icmp eq i64 %v45203, 10
  br i1 %exitcond20580.not, label %edge4539, label %edge4538

block28470:                                       ; preds = %edge4567, %edge4566
  %t3267 = zext i64 %v45191 to i128
  %v218364 = or disjoint i128 %t3267, 184467440737095516160
  store i128 %v218364, ptr %ss0, align 16
  %t3271 = zext i64 %v45193 to i128
  %v218366 = or disjoint i128 %t3271, 18446744073709551616
  store i128 %v218366, ptr %t1002, align 16
  %t3277 = zext i64 %v45196 to i128
  %t3278 = shl nuw i128 %t3277, 64
  %v218368 = or disjoint i128 %t3278, 10
  store i128 %v218368, ptr %t1007, align 16
  %v45854 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v45193, i64 4, i64 4, i64 %v3192)
  %t3281.not = icmp eq i64 %v45854, 0
  br i1 %t3281.not, label %edge4572, label %block28465

block28469:                                       ; preds = %edge4567, %edge4572
  %t3284.pre-phi = phi ptr [ %t3250, %edge4567 ], [ %.pre20662, %edge4572 ]
  %v45839 = phi i64 [ %v45191, %edge4567 ], [ %v45855, %edge4572 ]
  %v45841 = phi i64 [ %v45193, %edge4567 ], [ %v45856, %edge4572 ]
  %v45844 = phi i64 [ %v45196, %edge4567 ], [ %v45857, %edge4572 ]
  %v45859 = trunc nuw i64 %v45810 to i32
  store i32 %v45859, ptr %t3284.pre-phi, align 1
  br label %block28468

block28468:                                       ; preds = %edge4561, %block28469
  %v45833 = phi i64 [ %v45839, %block28469 ], [ %v45191, %edge4561 ]
  %v45835 = phi i64 [ %v45841, %block28469 ], [ %v45193, %edge4561 ]
  %v45838 = phi i64 [ %v45844, %block28469 ], [ %v45196, %edge4561 ]
  %t3285 = icmp eq i64 %v45196, %v45838
  %t3287.not119 = icmp eq i64 %v45838, 0
  %t3287.not = or i1 %t3285, %t3287.not119
  br i1 %t3287.not, label %block28474, label %edge4575

block28474:                                       ; preds = %block28468, %edge4575
  %t3291.not120 = icmp eq i64 %v45196, 0
  %t3291.not = or i1 %t3291.not120, %t3285
  br i1 %t3291.not, label %edge4586, label %edge4580

block28465:                                       ; preds = %edge4580, %block28470
  %v45821 = phi i64 [ %v45854, %block28470 ], [ %v45868, %edge4580 ]
  %t3293 = insertvalue { i64, i64 } poison, i64 %v45821, 0
  %t3294 = insertvalue { i64, i64 } %t3293, i64 2357, 1
  br label %common.ret

block28488:                                       ; preds = %edge4592, %edge4591
  %t3304 = zext i64 %v45176 to i128
  %v218370 = or disjoint i128 %t3304, 184467440737095516160
  store i128 %v218370, ptr %ss0, align 16
  %t3308 = zext i64 %v45178 to i128
  %v218372 = or disjoint i128 %t3308, 18446744073709551616
  store i128 %v218372, ptr %t1002, align 16
  %t3314 = zext i64 %v45181 to i128
  %t3315 = shl nuw i128 %t3314, 64
  %v218374 = or disjoint i128 %t3315, 10
  store i128 %v218374, ptr %t1007, align 16
  %v45917 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v45178, i64 4, i64 4, i64 %v3192)
  %t3318.not = icmp eq i64 %v45917, 0
  br i1 %t3318.not, label %edge4597, label %block28483

block28487:                                       ; preds = %edge4592, %edge4597
  %t3321.pre-phi = phi ptr [ %t3252, %edge4592 ], [ %.pre20660, %edge4597 ]
  %v45902 = phi i64 [ %v45176, %edge4592 ], [ %v45918, %edge4597 ]
  %v45904 = phi i64 [ %v45178, %edge4592 ], [ %v45919, %edge4597 ]
  %v45907 = phi i64 [ %v45181, %edge4592 ], [ %v45920, %edge4597 ]
  store i32 0, ptr %t3321.pre-phi, align 1
  br label %block28486

block28486:                                       ; preds = %edge4586, %block28487
  %v45896 = phi i64 [ %v45902, %block28487 ], [ %v45176, %edge4586 ]
  %v45898 = phi i64 [ %v45904, %block28487 ], [ %v45178, %edge4586 ]
  %v45901 = phi i64 [ %v45907, %block28487 ], [ %v45181, %edge4586 ]
  %t3322 = icmp eq i64 %v45181, %v45901
  %t3324.not121 = icmp eq i64 %v45901, 0
  %t3324.not = or i1 %t3322, %t3324.not121
  br i1 %t3324.not, label %block28492, label %edge4600

block28492:                                       ; preds = %block28486, %edge4600
  %t3328.not122 = icmp eq i64 %v45181, 0
  %t3328.not = or i1 %t3328.not122, %t3322
  br i1 %t3328.not, label %block2305.backedge, label %edge4603

block28483:                                       ; preds = %edge4606, %block28488
  %v45884 = phi i64 [ %v45917, %block28488 ], [ %v45931, %edge4606 ]
  %t3333 = insertvalue { i64, i64 } poison, i64 %v45884, 0
  %t3334 = insertvalue { i64, i64 } %t3333, i64 2360, 1
  br label %common.ret

block2378:                                        ; preds = %edge4539, %edge4773
  %v46658 = phi i64 [ %v47721.ph, %edge4773 ], [ %v10826.ph.ph, %edge4539 ]
  %v46660 = phi i64 [ %v47723.ph, %edge4773 ], [ %v10798.ph.ph, %edge4539 ]
  %v46663 = phi i64 [ %v47726.ph, %edge4773 ], [ %v10789.ph.ph, %edge4539 ]
  %v46670 = phi i64 [ %v47733.ph, %edge4773 ], [ %v40030, %edge4539 ]
  %v46672 = phi i64 [ %v47735.ph, %edge4773 ], [ %v40032, %edge4539 ]
  %v46675 = phi i64 [ %v47738.ph, %edge4773 ], [ %v40035, %edge4539 ]
  %v46682 = phi i64 [ %v47950, %edge4773 ], [ 0, %edge4539 ]
  %exitcond20581 = icmp eq i64 %v46682, 259
  br i1 %exitcond20581, label %block475.outer.outer, label %edge4658

block29323:                                       ; preds = %edge4692, %edge4691
  %t3368 = zext i64 %v46658 to i128
  %v218376 = or disjoint i128 %t3368, 4777706715090773868544
  store i128 %v218376, ptr %ss0, align 16
  %t3372 = zext i64 %v46660 to i128
  %v218378 = or disjoint i128 %t3372, 18446744073709551616
  store i128 %v218378, ptr %t1002, align 16
  %t3378 = zext i64 %v46663 to i128
  %t3379 = shl nuw i128 %t3378, 64
  %v218380 = or disjoint i128 %t3379, 259
  store i128 %v218380, ptr %t1007, align 16
  %v47548 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v46660, i64 4, i64 4, i64 %v3192)
  %t3382.not = icmp eq i64 %v47548, 0
  br i1 %t3382.not, label %edge4697, label %block29318

block29322:                                       ; preds = %edge4692, %edge4697
  %t3385.pre-phi = phi ptr [ %t3351, %edge4692 ], [ %.pre20664, %edge4697 ]
  %v47533 = phi i64 [ %v46658, %edge4692 ], [ %v47549, %edge4697 ]
  %v47535 = phi i64 [ %v46660, %edge4692 ], [ %v47550, %edge4697 ]
  %v47538 = phi i64 [ %v46663, %edge4692 ], [ %v47551, %edge4697 ]
  %v47553 = trunc nuw i64 %v47504 to i32
  store i32 %v47553, ptr %t3385.pre-phi, align 1
  %t3386 = icmp eq i64 %v46663, %v47538
  %t3388.not115 = icmp eq i64 %v47538, 0
  %t3388.not = or i1 %t3386, %t3388.not115
  br i1 %t3388.not, label %block29327, label %edge4700

block29327:                                       ; preds = %block29322, %edge4700
  %t3392.not = or i1 %t3363.not, %t3386
  br i1 %t3392.not, label %edge4709, label %edge4705

block29318:                                       ; preds = %edge4705, %block29323
  %v47515 = phi i64 [ %v47548, %block29323 ], [ %v47562, %edge4705 ]
  %t3394 = insertvalue { i64, i64 } poison, i64 %v47515, 0
  %t3395 = insertvalue { i64, i64 } %t3394, i64 2434, 1
  br label %common.ret

block29339:                                       ; preds = %edge4715, %edge4714
  %t3407 = zext i64 %v46670 to i128
  %v218382 = or disjoint i128 %t3407, 4777706715090773868544
  store i128 %v218382, ptr %ss0, align 16
  %t3411 = zext i64 %v46672 to i128
  %v218384 = or disjoint i128 %t3411, 18446744073709551616
  store i128 %v218384, ptr %t1002, align 16
  %t3417 = zext i64 %v46675 to i128
  %t3418 = shl nuw i128 %t3417, 64
  %v218386 = or disjoint i128 %t3418, 259
  store i128 %v218386, ptr %t1007, align 16
  %v47604 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v46672, i64 4, i64 4, i64 %v3192)
  %t3421.not = icmp eq i64 %v47604, 0
  br i1 %t3421.not, label %edge4720, label %block29334

block29338:                                       ; preds = %edge4715, %edge4720
  %t3424.pre-phi = phi ptr [ %t3344, %edge4715 ], [ %.pre20666, %edge4720 ]
  %v47589 = phi i64 [ %v46670, %edge4715 ], [ %v47605, %edge4720 ]
  %v47591 = phi i64 [ %v46672, %edge4715 ], [ %v47606, %edge4720 ]
  %v47594 = phi i64 [ %v46675, %edge4715 ], [ %v47607, %edge4720 ]
  store i32 0, ptr %t3424.pre-phi, align 1
  br label %block29337

block29337:                                       ; preds = %edge4709, %block29338
  %v47583 = phi i64 [ %v47589, %block29338 ], [ %v46670, %edge4709 ]
  %v47585 = phi i64 [ %v47591, %block29338 ], [ %v46672, %edge4709 ]
  %v47588 = phi i64 [ %v47594, %block29338 ], [ %v46675, %edge4709 ]
  %t3425 = icmp eq i64 %v46675, %v47588
  %t3427.not117 = icmp eq i64 %v47588, 0
  %t3427.not = or i1 %t3425, %t3427.not117
  br i1 %t3427.not, label %block29343, label %edge4723

block29343:                                       ; preds = %block29337, %edge4723
  %t3431.not118 = icmp eq i64 %v46675, 0
  %t3431.not = or i1 %t3431.not118, %t3425
  br i1 %t3431.not, label %edge4773, label %edge4726

block29334:                                       ; preds = %edge4729, %block29339
  %v47571 = phi i64 [ %v47604, %block29339 ], [ %v47618, %edge4729 ]
  %t3436 = insertvalue { i64, i64 } poison, i64 %v47571, 0
  %t3437 = insertvalue { i64, i64 } %t3436, i64 2436, 1
  br label %common.ret

block2462:                                        ; preds = %edge4502
  br i1 %v41761103.not.not31557, label %block475.backedge, label %edge4852

block475.backedge:                                ; preds = %block2462, %edge4337.block2462_crit_edge
  br label %block475

block2509:                                        ; preds = %block30641, %edge4902
  %v50448 = phi i64 [ %v40314, %edge4902 ], [ %v51047, %block30641 ]
  %v50451 = phi i64 [ %v50401, %edge4902 ], [ %v51239, %block30641 ]
  %t3463.not = icmp eq i64 %v50451, 0
  br i1 %t3463.not, label %edge4915, label %edge4917

block30641:                                       ; preds = %edge4931, %edge4918
  %v217799 = phi i64 [ 0, %edge4918 ], [ %v51040, %edge4931 ]
  %v51047 = sub i64 %v50638, %v217799
  %v51239 = add i64 %v50451, -1
  br label %block2509

block2587:                                        ; preds = %block2587.outer, %edge5008
  %v51757 = phi i64 [ %v52110, %edge5008 ], [ %v51757.ph, %block2587.outer ]
  %exitcond20582.not = icmp eq i64 %v51757, 259
  br i1 %exitcond20582.not, label %block2672, label %edge5008

block31619:                                       ; preds = %edge5041, %edge5040
  %t3523 = zext i64 %v51733.ph to i128
  %v218388 = or disjoint i128 %t3523, 4777706715090773868544
  store i128 %v218388, ptr %ss0, align 16
  %t3527 = zext i64 %v51735.ph to i128
  %v218390 = or disjoint i128 %t3527, 18446744073709551616
  store i128 %v218390, ptr %t1002, align 16
  %t3533 = zext i64 %v51738.ph to i128
  %t3534 = shl nuw i128 %t3533, 64
  %v218392 = or disjoint i128 %t3534, 259
  store i128 %v218392, ptr %t1007, align 16
  %v52552 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v51735.ph, i64 4, i64 4, i64 %v3192)
  %t3537.not = icmp eq i64 %v52552, 0
  br i1 %t3537.not, label %edge5046, label %block31614

block31618:                                       ; preds = %edge5041, %edge5046
  %t3540.pre-phi = phi ptr [ %t3506, %edge5041 ], [ %.pre20668, %edge5046 ]
  %v52537 = phi i64 [ %v51733.ph, %edge5041 ], [ %v52553, %edge5046 ]
  %v52539 = phi i64 [ %v51735.ph, %edge5041 ], [ %v52554, %edge5046 ]
  %v52542 = phi i64 [ %v51738.ph, %edge5041 ], [ %v52555, %edge5046 ]
  %v52557 = trunc nuw i64 %v52508 to i32
  store i32 %v52557, ptr %t3540.pre-phi, align 1
  %t3541 = icmp eq i64 %v51738.ph, %v52542
  %t3543.not125 = icmp eq i64 %v52542, 0
  %t3543.not = or i1 %t3541, %t3543.not125
  br i1 %t3543.not, label %block31623, label %edge5049

block31623:                                       ; preds = %block31618, %edge5049
  %t3547.not = or i1 %t3518.not, %t3541
  br i1 %t3547.not, label %edge5058, label %edge5054

block31614:                                       ; preds = %edge5054, %block31619
  %v52519 = phi i64 [ %v52552, %block31619 ], [ %v52566, %edge5054 ]
  %t3549 = insertvalue { i64, i64 } poison, i64 %v52519, 0
  %t3550 = insertvalue { i64, i64 } %t3549, i64 2645, 1
  br label %common.ret

block31635:                                       ; preds = %edge5064, %edge5063
  %t3562 = zext i64 %v51745.ph to i128
  %v218394 = or disjoint i128 %t3562, 4777706715090773868544
  store i128 %v218394, ptr %ss0, align 16
  %t3566 = zext i64 %v51747.ph to i128
  %v218396 = or disjoint i128 %t3566, 18446744073709551616
  store i128 %v218396, ptr %t1002, align 16
  %t3572 = zext i64 %v51750.ph to i128
  %t3573 = shl nuw i128 %t3572, 64
  %v218398 = or disjoint i128 %t3573, 259
  store i128 %v218398, ptr %t1007, align 16
  %v52608 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v51747.ph, i64 4, i64 4, i64 %v3192)
  %t3576.not = icmp eq i64 %v52608, 0
  br i1 %t3576.not, label %edge5069, label %block31630

block31634:                                       ; preds = %edge5064, %edge5069
  %t3579.pre-phi = phi ptr [ %t3499, %edge5064 ], [ %.pre20670, %edge5069 ]
  %v52593 = phi i64 [ %v51745.ph, %edge5064 ], [ %v52609, %edge5069 ]
  %v52595 = phi i64 [ %v51747.ph, %edge5064 ], [ %v52610, %edge5069 ]
  %v52598 = phi i64 [ %v51750.ph, %edge5064 ], [ %v52611, %edge5069 ]
  store i32 0, ptr %t3579.pre-phi, align 1
  br label %block31633

block31633:                                       ; preds = %edge5058, %block31634
  %v52587 = phi i64 [ %v52593, %block31634 ], [ %v51745.ph, %edge5058 ]
  %v52589 = phi i64 [ %v52595, %block31634 ], [ %v51747.ph, %edge5058 ]
  %v52592 = phi i64 [ %v52598, %block31634 ], [ %v51750.ph, %edge5058 ]
  %t3580 = icmp eq i64 %v51750.ph, %v52592
  %t3582.not127 = icmp eq i64 %v52592, 0
  %t3582.not = or i1 %t3580, %t3582.not127
  br i1 %t3582.not, label %block31639, label %edge5072

block31639:                                       ; preds = %block31633, %edge5072
  %t3586.not128 = icmp eq i64 %v51750.ph, 0
  %t3586.not = or i1 %t3586.not128, %t3580
  br i1 %t3586.not, label %block2587.outer.backedge, label %edge5075

block2587.outer.backedge:                         ; preds = %block31639, %edge5078, %edge5075
  br label %block2587.outer

block31630:                                       ; preds = %edge5078, %block31635
  %v52575 = phi i64 [ %v52608, %block31635 ], [ %v52622, %edge5078 ]
  %t3591 = insertvalue { i64, i64 } poison, i64 %v52575, 0
  %t3592 = insertvalue { i64, i64 } %t3591, i64 2647, 1
  br label %common.ret

block2672:                                        ; preds = %block2587, %block2672.backedge
  %v53304 = phi i64 [ %v54009, %block2672.backedge ], [ %v40042, %block2587 ]
  %v53306 = phi i64 [ %v54011, %block2672.backedge ], [ %v40044, %block2587 ]
  %v53309 = phi i64 [ %v54014, %block2672.backedge ], [ %v40047, %block2587 ]
  %v53319 = phi i64 [ %v53946, %block2672.backedge ], [ %v10828.ph.ph, %block2587 ]
  %v53321 = phi i64 [ %v53948, %block2672.backedge ], [ %v10802.ph.ph, %block2587 ]
  %v53324 = phi i64 [ %v53951, %block2672.backedge ], [ %v10791.ph.ph, %block2587 ]
  %v53331 = phi i64 [ %v53686, %block2672.backedge ], [ 0, %block2587 ]
  %exitcond20583.not = icmp eq i64 %v53331, 10
  br i1 %exitcond20583.not, label %edge5147, label %edge5146

block32489:                                       ; preds = %edge5175, %edge5174
  %t3616 = zext i64 %v53319 to i128
  %v218400 = or disjoint i128 %t3616, 184467440737095516160
  store i128 %v218400, ptr %ss0, align 16
  %t3620 = zext i64 %v53321 to i128
  %v218402 = or disjoint i128 %t3620, 18446744073709551616
  store i128 %v218402, ptr %t1002, align 16
  %t3626 = zext i64 %v53324 to i128
  %t3627 = shl nuw i128 %t3626, 64
  %v218404 = or disjoint i128 %t3627, 10
  store i128 %v218404, ptr %t1007, align 16
  %v53967 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v53321, i64 4, i64 4, i64 %v3192)
  %t3630.not = icmp eq i64 %v53967, 0
  br i1 %t3630.not, label %edge5180, label %block32484

block32488:                                       ; preds = %edge5175, %edge5180
  %t3633.pre-phi = phi ptr [ %t3599, %edge5175 ], [ %.pre20674, %edge5180 ]
  %v53952 = phi i64 [ %v53319, %edge5175 ], [ %v53968, %edge5180 ]
  %v53954 = phi i64 [ %v53321, %edge5175 ], [ %v53969, %edge5180 ]
  %v53957 = phi i64 [ %v53324, %edge5175 ], [ %v53970, %edge5180 ]
  %v53972 = trunc nuw i64 %v53923 to i32
  store i32 %v53972, ptr %t3633.pre-phi, align 1
  br label %block32487

block32487:                                       ; preds = %edge5169, %block32488
  %v53946 = phi i64 [ %v53952, %block32488 ], [ %v53319, %edge5169 ]
  %v53948 = phi i64 [ %v53954, %block32488 ], [ %v53321, %edge5169 ]
  %v53951 = phi i64 [ %v53957, %block32488 ], [ %v53324, %edge5169 ]
  %t3634 = icmp eq i64 %v53324, %v53951
  %t3636.not129 = icmp eq i64 %v53951, 0
  %t3636.not = or i1 %t3634, %t3636.not129
  br i1 %t3636.not, label %block32493, label %edge5183

block32493:                                       ; preds = %block32487, %edge5183
  %t3640.not130 = icmp eq i64 %v53324, 0
  %t3640.not = or i1 %t3640.not130, %t3634
  br i1 %t3640.not, label %edge5194, label %edge5188

block32484:                                       ; preds = %edge5188, %block32489
  %v53934 = phi i64 [ %v53967, %block32489 ], [ %v53981, %edge5188 ]
  %t3642 = insertvalue { i64, i64 } poison, i64 %v53934, 0
  %t3643 = insertvalue { i64, i64 } %t3642, i64 2728, 1
  br label %common.ret

block32507:                                       ; preds = %edge5200, %edge5199
  %t3653 = zext i64 %v53304 to i128
  %v218406 = or disjoint i128 %t3653, 184467440737095516160
  store i128 %v218406, ptr %ss0, align 16
  %t3657 = zext i64 %v53306 to i128
  %v218408 = or disjoint i128 %t3657, 18446744073709551616
  store i128 %v218408, ptr %t1002, align 16
  %t3663 = zext i64 %v53309 to i128
  %t3664 = shl nuw i128 %t3663, 64
  %v218410 = or disjoint i128 %t3664, 10
  store i128 %v218410, ptr %t1007, align 16
  %v54030 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v53306, i64 4, i64 4, i64 %v3192)
  %t3667.not = icmp eq i64 %v54030, 0
  br i1 %t3667.not, label %edge5205, label %block32502

block32506:                                       ; preds = %edge5200, %edge5205
  %t3670.pre-phi = phi ptr [ %t3601, %edge5200 ], [ %.pre20672, %edge5205 ]
  %v54015 = phi i64 [ %v53304, %edge5200 ], [ %v54031, %edge5205 ]
  %v54017 = phi i64 [ %v53306, %edge5200 ], [ %v54032, %edge5205 ]
  %v54020 = phi i64 [ %v53309, %edge5200 ], [ %v54033, %edge5205 ]
  store i32 0, ptr %t3670.pre-phi, align 1
  br label %block32505

block32505:                                       ; preds = %edge5194, %block32506
  %v54009 = phi i64 [ %v54015, %block32506 ], [ %v53304, %edge5194 ]
  %v54011 = phi i64 [ %v54017, %block32506 ], [ %v53306, %edge5194 ]
  %v54014 = phi i64 [ %v54020, %block32506 ], [ %v53309, %edge5194 ]
  %t3671 = icmp eq i64 %v53309, %v54014
  %t3673.not131 = icmp eq i64 %v54014, 0
  %t3673.not = or i1 %t3671, %t3673.not131
  br i1 %t3673.not, label %block32511, label %edge5208

block32511:                                       ; preds = %block32505, %edge5208
  %t3677.not132 = icmp eq i64 %v53309, 0
  %t3677.not = or i1 %t3677.not132, %t3671
  br i1 %t3677.not, label %block2672.backedge, label %edge5211

block32502:                                       ; preds = %edge5214, %block32507
  %v53997 = phi i64 [ %v54030, %block32507 ], [ %v54044, %edge5214 ]
  %t3682 = insertvalue { i64, i64 } poison, i64 %v53997, 0
  %t3683 = insertvalue { i64, i64 } %t3682, i64 2731, 1
  br label %common.ret

block33195:                                       ; preds = %edge5365
  %t3734 = insertvalue { i64, i64 } poison, i64 %v55961, 0
  %t3735 = insertvalue { i64, i64 } %t3734, i64 2811, 1
  br label %common.ret

block2804:                                        ; preds = %edge5367, %edge5368
  br i1 %t23.not, label %block2806.preheader, label %edge5374

block2806.preheader:                              ; preds = %block2804, %edge5374
  br label %block2806

block2806:                                        ; preds = %block2806.preheader, %edge5530
  %v56477 = phi i64 [ %v57487, %edge5530 ], [ %v55966, %block2806.preheader ]
  %v56511 = phi i64 [ %v57484, %edge5530 ], [ %v55965, %block2806.preheader ]
  %v56514 = phi i64 [ %v57722, %edge5530 ], [ 0, %block2806.preheader ]
  %v56519 = phi i64 [ %v57482, %edge5530 ], [ %v55964, %block2806.preheader ]
  %t3739.not = icmp eq i64 %v56477, 0
  br i1 %t3739.not, label %edge5416, label %edge5382

block33253:                                       ; preds = %edge5389, %edge5390
  %t3778.pn = phi { i64, i64 } [ %t3778, %edge5390 ], [ %t3779, %edge5389 ]
  %v56483 = extractvalue { i64, i64 } %t3778.pn, 0
  %t3780.not = icmp eq i64 %v56483, 0
  br i1 %t3780.not, label %edge5394, label %edge5393

trap3803:                                         ; preds = %edge5445
  call void @llvm.trap()
  unreachable

ok3803:                                           ; preds = %edge5445
  %v56996 = udiv i64 %v56760, %v1397
  %v56997 = mul i64 %v56996, %v1397
  %v56998 = sub nuw i64 %v56760, %v56997
  %v56999 = mul i64 %v56998, %v1398
  %v57000 = add i64 %v56999, %v56996
  br label %block33589

block33589:                                       ; preds = %edge5441, %ok3803
  %v56995 = phi i64 [ %v57000, %ok3803 ], [ %v56760, %edge5441 ]
  %v57002 = add i64 %v56995, %v1394
  %t3804 = inttoptr i64 %v57002 to ptr
  %t3805 = load i8, ptr %t3804, align 1
  %v57003 = zext i8 %t3805 to i64
  %2 = shl nuw nsw i64 %v57003, 2
  br label %edge5477

block33774:                                       ; preds = %edge5486, %edge5485
  %t3826 = zext i64 %v56519 to i128
  %v218424 = or disjoint i128 %t3826, 5902958103587056517120
  store i128 %v218424, ptr %ss0, align 16
  %t3830 = zext i64 %v56511 to i128
  %v218426 = or disjoint i128 %t3830, 18446744073709551616
  store i128 %v218426, ptr %t1002, align 16
  %t3836 = zext i64 %v56477 to i128
  %t3837 = shl nuw i128 %t3836, 64
  %v218428 = or disjoint i128 %t3837, 320
  store i128 %v218428, ptr %t1007, align 16
  %v57497 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v56511, i64 4, i64 4, i64 %v3192)
  %t3840.not = icmp eq i64 %v57497, 0
  br i1 %t3840.not, label %edge5491, label %block33769

block33772:                                       ; preds = %edge5491, %edge5486
  %t3843.pre-phi = phi ptr [ %.pre20676, %edge5491 ], [ %t3809, %edge5486 ]
  %v57482 = phi i64 [ %v57498, %edge5491 ], [ %v56519, %edge5486 ]
  %v57484 = phi i64 [ %v57499, %edge5491 ], [ %v56511, %edge5486 ]
  %v57487 = phi i64 [ %v57500, %edge5491 ], [ %v56477, %edge5486 ]
  store i32 %v57455, ptr %t3843.pre-phi, align 1
  %t3844 = icmp eq i64 %v56477, %v57487
  %t3846.not136 = icmp eq i64 %v57487, 0
  %t3846.not = or i1 %t3844, %t3846.not136
  br i1 %t3846.not, label %edge5505, label %edge5494

block33769:                                       ; preds = %block33774
  %t3850 = insertvalue { i64, i64 } poison, i64 %v57497, 0
  %t3851 = insertvalue { i64, i64 } %t3850, i64 2881, 1
  br label %common.ret

block33794:                                       ; preds = %edge5509, %edge5506
  %t3859.not = icmp eq i64 %v56477, 0
  br i1 %t3859.not, label %edge5557, label %edge5513

block33789:                                       ; preds = %edge5516, %edge5509, %edge5523
  %v57528 = phi i64 [ %v57543, %edge5523 ], [ %v57535, %edge5509 ], [ %v57539, %edge5516 ]
  %t3866 = insertvalue { i64, i64 } poison, i64 %v57528, 0
  %t3867 = insertvalue { i64, i64 } %t3866, i64 2886, 1
  br label %common.ret

block34146:                                       ; preds = %edge5557, %edge5573
  %t3875.not21119 = phi i1 [ false, %edge5573 ], [ true, %edge5557 ]
  %v5810421116 = phi i32 [ %v5810421108, %edge5573 ], [ %v58104, %edge5557 ]
  %v5777321111 = phi i64 [ %v56477, %edge5573 ], [ 0, %edge5557 ]
  %t3880 = zext i64 %v56519 to i128
  %v218430 = or disjoint i128 %t3880, 5902958103587056517120
  store i128 %v218430, ptr %ss0, align 16
  %t3884 = zext i64 %v56511 to i128
  %v218432 = or disjoint i128 %t3884, 18446744073709551616
  store i128 %v218432, ptr %t1002, align 16
  %t3890 = zext i64 %v5777321111 to i128
  %t3891 = shl nuw i128 %t3890, 64
  %v218434 = or disjoint i128 %t3891, 320
  store i128 %v218434, ptr %t1007, align 16
  %v58164 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v56511, i64 4, i64 4, i64 %v3192)
  %t3894.not = icmp eq i64 %v58164, 0
  br i1 %t3894.not, label %block34145, label %block34141

block34145:                                       ; preds = %block34146
  %v58165 = load i64, ptr %ss1, align 8
  %v58166 = load i64, ptr %t606, align 8
  %v58167 = load i64, ptr %t607, align 8
  %.pre20677 = add i64 %v58165, 1024
  %.pre20678 = inttoptr i64 %.pre20677 to ptr
  store i32 %v5810421116, ptr %.pre20678, align 1
  %t3898 = icmp eq i64 %v5777321111, %v58167
  %t3900.not138 = icmp eq i64 %v58167, 0
  %t3900.not = or i1 %t3898, %t3900.not138
  br i1 %t3900.not, label %block34150, label %edge5581

block34150:                                       ; preds = %block34145, %edge5581
  %t3904.not = or i1 %t3875.not21119, %t3898
  br i1 %t3904.not, label %edge5595, label %edge5584

block34141:                                       ; preds = %edge5587, %block34146
  %v58131 = phi i64 [ %v58164, %block34146 ], [ %v58178, %edge5587 ]
  %t3909 = insertvalue { i64, i64 } poison, i64 %v58131, 0
  %t3910 = insertvalue { i64, i64 } %t3909, i64 2923, 1
  br label %common.ret

block2907:                                        ; preds = %block2907.backedge, %edge5647
  %v64641 = phi i64 [ 1, %edge5647 ], [ %v217676, %block2907.backedge ]
  %v64740 = phi i64 [ 0, %edge5647 ], [ %v165399, %block2907.backedge ]
  %v64743 = phi i64 [ 0, %edge5647 ], [ %v165434, %block2907.backedge ]
  %v64746 = phi i64 [ %v217720, %edge5647 ], [ %v7457721129, %block2907.backedge ]
  %v64747 = phi i64 [ %v217721, %edge5647 ], [ %v7457821133, %block2907.backedge ]
  %v64748 = phi i64 [ %v217722, %edge5647 ], [ %v7457921136, %block2907.backedge ]
  %v64749 = phi i64 [ %v217723, %edge5647 ], [ %v7458021140, %block2907.backedge ]
  %v64750 = phi i64 [ %v217724, %edge5647 ], [ %v7458121144, %block2907.backedge ]
  %v64751 = phi i64 [ %v217725, %edge5647 ], [ %v7458221148, %block2907.backedge ]
  %v64758 = phi i64 [ %v217733, %edge5647 ], [ %v217651, %block2907.backedge ]
  %v64760 = phi i64 [ %v217735, %edge5647 ], [ %v217653, %block2907.backedge ]
  %v64763 = phi i64 [ %v217738, %edge5647 ], [ %v217632, %block2907.backedge ]
  %v64770 = phi i64 [ %v217739, %edge5647 ], [ %v144765, %block2907.backedge ]
  %v64772 = phi i64 [ %v217741, %edge5647 ], [ %v144771, %block2907.backedge ]
  %v64775 = phi i64 [ %v217744, %edge5647 ], [ %v138168, %block2907.backedge ]
  %v64782 = phi i64 [ -1, %edge5647 ], [ %spec.select36992, %block2907.backedge ]
  %v64785 = phi i64 [ 10, %edge5647 ], [ %v217609, %block2907.backedge ]
  %v64788 = phi i64 [ %v58149212852130121338, %edge5647 ], [ %v80926, %block2907.backedge ]
  %v64793 = phi i64 [ %v58154212872129921342, %edge5647 ], [ %v80920, %block2907.backedge ]
  %v64800 = phi i64 [ %v58149212852130121338, %edge5647 ], [ %v129479, %block2907.backedge ]
  %v64801 = phi i64 [ 320, %edge5647 ], [ %v129480, %block2907.backedge ]
  %v64804 = phi i64 [ 320, %edge5647 ], [ %v129483, %block2907.backedge ]
  %v64805 = phi i64 [ %v58154212872129921342, %edge5647 ], [ %v129484, %block2907.backedge ]
  %v64845 = phi i64 [ 0, %edge5647 ], [ %spec.select3, %block2907.backedge ]
  %v64848 = phi i64 [ -1, %edge5647 ], [ %v217663, %block2907.backedge ]
  %v64911 = phi i64 [ %v58149212852130121338, %edge5647 ], [ %v129495, %block2907.backedge ]
  %v64916 = phi i64 [ %v58154212872129921342, %edge5647 ], [ %v129500, %block2907.backedge ]
  %v64923 = phi i64 [ %v58149212852130121338, %edge5647 ], [ %v129501, %block2907.backedge ]
  %v64924 = phi i64 [ 320, %edge5647 ], [ %v129502, %block2907.backedge ]
  %v64928 = phi i64 [ %v58154212872129921342, %edge5647 ], [ %v129506, %block2907.backedge ]
  %v64935 = phi i64 [ %v58149212852130121338, %edge5647 ], [ %v129507, %block2907.backedge ]
  %v64940 = phi i64 [ %v58154212872129921342, %edge5647 ], [ %v129512, %block2907.backedge ]
  %v64947 = phi i64 [ 0, %edge5647 ], [ %v129513, %block2907.backedge ]
  %v64950 = phi i64 [ 0, %edge5647 ], [ %v129514, %block2907.backedge ]
  %v64953 = phi i64 [ 0, %edge5647 ], [ %v129515, %block2907.backedge ]
  %v64956 = phi i64 [ 0, %edge5647 ], [ %v129516, %block2907.backedge ]
  %t3926 = icmp eq i64 %v64641, 1
  %v64967 = zext i1 %t3926 to i64
  %t3927 = icmp eq i64 %v64641, 2
  %v64983 = zext i1 %t3927 to i64
  %t3928 = icmp eq i64 %v64641, 4
  %v64999 = zext i1 %t3928 to i64
  %3 = and i64 %v64641, -2
  %v65020140 = icmp eq i64 %3, 2
  br i1 %v65020140, label %edge5744, label %edge5761

block35152:                                       ; preds = %edge5953, %edge5957, %edge5950
  %v67015 = phi i64 [ %v64748, %edge5950 ], [ %28, %edge5957 ], [ 258, %edge5953 ]
  %t4053.not = icmp eq i64 %v64751, 0
  br i1 %t4053.not, label %block35163, label %edge5970

block35163:                                       ; preds = %edge5970, %block35152
  %t4058 = zext i64 %v64746 to i128
  store i128 %t4058, ptr %ss0, align 16
  %t4062 = zext i64 %v64748 to i128
  %v218444 = or disjoint i128 %t4062, 18446744073709551616
  store i128 %v218444, ptr %t1002, align 16
  %t4068 = zext i64 %v64751 to i128
  %t4069 = shl nuw i128 %t4068, 64
  store i128 %t4069, ptr %t1007, align 16
  %v67055 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v67015, i64 8, i64 8, i64 %v3192)
  %t4072.not = icmp eq i64 %v67055, 0
  br i1 %t4072.not, label %edge5975, label %block35147

block35169:                                       ; preds = %edge5977, %edge5980
  %t4082.not = or i1 %t4053.not, %t4076
  br i1 %t4082.not, label %edge6154, label %edge5985

block35147:                                       ; preds = %edge5985, %block35163
  %v66515 = phi i64 [ %v67055, %block35163 ], [ %v67073, %edge5985 ]
  %t4084 = insertvalue { i64, i64 } poison, i64 %v66515, 0
  %t4085 = insertvalue { i64, i64 } %t4084, i64 3001, 1
  br label %common.ret

block35370:                                       ; preds = %edge6154, %edge6156
  %t4145.not = icmp eq i64 %v66492, 0
  br i1 %t4145.not, label %edge6169, label %edge6159

block2989:                                        ; preds = %block2989.backedge, %edge6169
  %v67326 = phi i64 [ %v67045, %edge6169 ], [ %v73702, %block2989.backedge ]
  %v67394 = phi i64 [ 258, %edge6169 ], [ %v73667, %block2989.backedge ]
  %v67400 = phi i64 [ %v54999, %edge6169 ], [ %v67758, %block2989.backedge ]
  %v67401 = phi i64 [ %v54959, %edge6169 ], [ %v69322, %block2989.backedge ]
  %v67402 = phi i64 [ %v67040, %edge6169 ], [ %v73697, %block2989.backedge ]
  %v67412 = phi i64 [ %v67042, %edge6169 ], [ %v73699, %block2989.backedge ]
  %t4148.not = icmp eq i64 %v67326, 0
  br i1 %t4148.not, label %edge6220, label %edge6170

block35404:                                       ; preds = %edge6180, %edge6179, %edge6181
  %t4216.pn = phi { i64, i64 } [ %t4216, %edge6181 ], [ %t4217, %edge6179 ], [ %t4218, %edge6180 ]
  %v67343 = extractvalue { i64, i64 } %t4216.pn, 0
  %t4219.not = icmp eq i64 %v67343, 0
  br i1 %t4219.not, label %edge6186, label %edge6185

block35415:                                       ; preds = %edge6190, %edge6186
  br i1 %t4145.not, label %edge5769, label %edge6196

block35396:                                       ; preds = %edge6196, %edge6190
  %v67325 = phi i64 [ %v67382, %edge6190 ], [ %v67386, %edge6196 ]
  %t4237 = insertvalue { i64, i64 } poison, i64 %v67325, 0
  %t4238 = insertvalue { i64, i64 } %t4237, i64 3009, 1
  br label %common.ret

block3108:                                        ; preds = %edge6781, %edge6331
  %v69375 = phi i64 [ %v69322, %edge6331 ], [ %v72856, %edge6781 ]
  %v69380 = phi i64 [ 0, %edge6331 ], [ %umax20584, %edge6781 ]
  %v69385 = phi i64 [ %v69263, %edge6331 ], [ %v71072, %edge6781 ]
  %v69390 = phi i64 [ %v69306, %edge6331 ], [ %v71077, %edge6781 ]
  %t4365.not = icmp eq i64 %v69375, %v67763
  br i1 %t4365.not, label %edge6805, label %edge6790

block37660:                                       ; preds = %edge6495, %edge6492
  %t4291.not = icmp eq i64 %v217710, 0
  br i1 %t4291.not, label %block37668, label %edge6501

block37668:                                       ; preds = %block37660, %edge6501
  %t4295.not = icmp eq i64 %v54918, 0
  br i1 %t4295.not, label %block37672, label %edge6513

block37672:                                       ; preds = %edge6513, %block37668
  br i1 %t3701.not, label %block37676, label %edge6519

block37676:                                       ; preds = %edge6519, %block37672
  %t4299.not = icmp eq i64 %v54942, 0
  br i1 %t4299.not, label %block37680, label %edge6525

block37680:                                       ; preds = %edge6525, %block37676
  br i1 %t3704.not, label %block37684, label %edge6531

block37684:                                       ; preds = %edge6531, %block37680
  %t4303.not = icmp eq i64 %v54954, 0
  br i1 %t4303.not, label %block37688, label %edge6537

block37688:                                       ; preds = %edge6537, %block37684
  br i1 %t7298.not, label %block37692, label %edge6543

block37692:                                       ; preds = %edge6543, %block37688
  br i1 %t7381.not, label %block37696, label %edge6549

block37696:                                       ; preds = %edge6549, %block37692
  br i1 %t3704.not, label %edge6563, label %edge6553

block37720:                                       ; preds = %edge6566, %edge6563
  %t4316.not = icmp eq i64 %v64775, 0
  br i1 %t4316.not, label %block37724, label %edge6572

block37724:                                       ; preds = %edge6572, %block37720
  %t4318.not = icmp eq i64 %v64793, 0
  br i1 %t4318.not, label %block37728, label %edge6578

block37728:                                       ; preds = %edge6578, %block37724
  %t4320.not = icmp eq i64 %v64805, 0
  br i1 %t4320.not, label %block37732, label %edge6584

block37732:                                       ; preds = %edge6584, %block37728
  br i1 %t3701.not, label %block37736, label %edge6590

block37736:                                       ; preds = %edge6590, %block37732
  br i1 %t3900.not138212892129821344, label %edge6605, label %edge6594

block37764:                                       ; preds = %edge6608, %edge6605
  %t4331.not = icmp eq i64 %v64928, 0
  br i1 %t4331.not, label %block37768, label %edge6614

block37768:                                       ; preds = %edge6614, %block37764
  %t4333.not = icmp eq i64 %v64940, 0
  br i1 %t4333.not, label %block37772, label %edge6620

block37772:                                       ; preds = %edge6620, %block37768
  br i1 %t4145.not, label %block37776, label %edge6626

block37776:                                       ; preds = %edge6626, %block37772
  br i1 %t4148.not, label %block37780, label %edge6632

block37780:                                       ; preds = %edge6632, %block37776
  br i1 %t3701.not, label %block37784, label %edge6638

block37784:                                       ; preds = %edge6638, %block37780
  br i1 %t4145.not, label %common.ret, label %edge6644

block37592:                                       ; preds = %edge6644, %edge6638, %edge6632, %edge6626, %edge6620, %edge6614, %edge6608, %edge6597, %edge6590, %edge6584, %edge6578, %edge6572, %edge6566, %edge6556, %edge6549, %edge6543, %edge6537, %edge6531, %edge6525, %edge6519, %edge6513, %edge6501, %edge6495
  %v70797 = phi i64 [ %v70991, %edge6638 ], [ %v70867, %edge6495 ], [ %v70871, %edge6501 ], [ %v70995, %edge6644 ], [ %v70879, %edge6513 ], [ %v70883, %edge6519 ], [ %v70887, %edge6525 ], [ %v70891, %edge6531 ], [ %v70895, %edge6537 ], [ %v70899, %edge6543 ], [ %v70903, %edge6549 ], [ %v70907, %edge6556 ], [ %v70927, %edge6566 ], [ %v70931, %edge6572 ], [ %v70935, %edge6578 ], [ %v70939, %edge6584 ], [ %v70943, %edge6590 ], [ %v70947, %edge6597 ], [ %v70971, %edge6608 ], [ %v70975, %edge6614 ], [ %v70979, %edge6620 ], [ %v70983, %edge6626 ], [ %v70987, %edge6632 ]
  %t4343 = insertvalue { i64, i64 } poison, i64 %v70797, 0
  %t4344 = insertvalue { i64, i64 } %t4343, i64 3236, 1
  br label %common.ret

block3215:                                        ; preds = %edge6726, %edge6652
  %v71067 = phi i64 [ %v69380, %edge6652 ], [ %v72433, %edge6726 ]
  %v71072 = phi i64 [ %v69385, %edge6652 ], [ %spec.select29, %edge6726 ]
  %v71077 = phi i64 [ %v69390, %edge6652 ], [ %v72437, %edge6726 ]
  %exitcond20585.not = icmp eq i64 %v71067, %umax20584
  br i1 %exitcond20585.not, label %edge6781, label %edge6768

block39213:                                       ; preds = %edge6820, %edge6824, %edge6833
  %v73673 = phi i64 [ %v73678, %edge6824 ], [ %v73695, %edge6833 ], [ 8, %edge6820 ]
  %t4395 = call i64 @llvm.umax.i64(i64 %v73673, i64 %v73667)
  br label %block39210

block39210:                                       ; preds = %edge6817, %block39213
  %v73672 = phi i64 [ %t4395, %block39213 ], [ %v67412, %edge6817 ]
  br i1 %t4148.not, label %block39221, label %edge6837

block39221:                                       ; preds = %edge6837, %block39210
  %t4401 = zext i64 %v67402 to i128
  %t4402 = zext i64 %v67394 to i128
  %t4403 = shl nuw i128 %t4402, 64
  %v218480 = or disjoint i128 %t4403, %t4401
  store i128 %v218480, ptr %ss0, align 16
  %t4405 = zext i64 %v67412 to i128
  %v218482 = or disjoint i128 %t4405, 18446744073709551616
  store i128 %v218482, ptr %t1002, align 16
  %t4411 = zext i64 %v67326 to i128
  %t4412 = shl nuw i128 %t4411, 64
  %v218484 = or disjoint i128 %t4412, %t4402
  store i128 %v218484, ptr %t1007, align 16
  %v73712 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v73672, i64 8, i64 8, i64 %v3192)
  %t4415.not = icmp eq i64 %v73712, 0
  br i1 %t4415.not, label %edge6842, label %block39206

block39220:                                       ; preds = %edge6837, %edge6842
  %v73697 = phi i64 [ %v73713, %edge6842 ], [ %v67402, %edge6837 ]
  %v73699 = phi i64 [ %v73714, %edge6842 ], [ %v67412, %edge6837 ]
  %v73702 = phi i64 [ %v73715, %edge6842 ], [ %v67326, %edge6837 ]
  %v73716 = shl i64 %v67394, 3
  %v73717 = add i64 %v73697, %v73716
  %t4418 = inttoptr i64 %v73717 to ptr
  store i64 %v73659, ptr %t4418, align 1
  %t4419 = icmp eq i64 %v67326, %v73702
  %t4421.not146 = icmp eq i64 %v73702, 0
  %t4421.not = or i1 %t4419, %t4421.not146
  br i1 %t4421.not, label %edge6863, label %edge6844

block39206:                                       ; preds = %block39221, %edge6824, %edge6805
  %v73661 = phi i64 [ %v73712, %block39221 ], [ 1, %edge6805 ], [ 1, %edge6824 ]
  %t4425 = insertvalue { i64, i64 } poison, i64 %v73661, 0
  %t4426 = insertvalue { i64, i64 } %t4425, i64 3396, 1
  br label %common.ret

block39259:                                       ; preds = %edge6868, %edge6865
  br i1 %t4145.not, label %edge6878, label %edge6874

block39254:                                       ; preds = %edge6882, %edge6874, %edge6868, %edge6890
  %v73780 = phi i64 [ %v73809, %edge6890 ], [ %v73797, %edge6874 ], [ %v73793, %edge6868 ], [ %v73805, %edge6882 ]
  %t4446 = insertvalue { i64, i64 } poison, i64 %v73780, 0
  %t4447 = insertvalue { i64, i64 } %t4446, i64 3407, 1
  br label %common.ret

block39752:                                       ; preds = %edge6966
  %t4483 = insertvalue { i64, i64 } poison, i64 %v75290, 0
  %t4484 = insertvalue { i64, i64 } %t4483, i64 3435, 1
  br label %common.ret

block3417:                                        ; preds = %block3417.preheader, %edge7325
  %v75595 = phi i64 [ %v79115784, %edge7325 ], [ %v75295, %block3417.preheader ]
  %v75627 = phi i64 [ %v79112783, %edge7325 ], [ %v75294, %block3417.preheader ]
  %v75629 = phi i64 [ %v79107785, %edge7325 ], [ 0, %block3417.preheader ]
  %v75635 = phi i64 [ %v79110782, %edge7325 ], [ %v75293, %block3417.preheader ]
  %t4485.not = icmp eq i64 %v75595, 0
  br i1 %t4485.not, label %edge7025, label %edge6987

block40803:                                       ; preds = %edge7099, %edge7070
  %t4579 = zext i64 %v75635 to i128
  %v218506 = or disjoint i128 %t4579, 5902958103587056517120
  store i128 %v218506, ptr %ss0, align 16
  %t4583 = zext i64 %v75627 to i128
  %v218508 = or disjoint i128 %t4583, 18446744073709551616
  store i128 %v218508, ptr %t1002, align 16
  %t4589 = zext i64 %v75595 to i128
  %t4590 = shl nuw i128 %t4589, 64
  %v218510 = or disjoint i128 %t4590, 320
  store i128 %v218510, ptr %t1007, align 16
  %v77289 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v75627, i64 4, i64 4, i64 %v3192)
  %t4593.not = icmp eq i64 %v77289, 0
  br i1 %t4593.not, label %edge7104, label %edge7103

block40802:                                       ; preds = %edge7099, %edge7104
  %t4596.pre-phi = phi ptr [ %t4567, %edge7099 ], [ %.pre20720, %edge7104 ]
  %v77274 = phi i64 [ %v75635, %edge7099 ], [ %v77290, %edge7104 ]
  %v77276 = phi i64 [ %v75627, %edge7099 ], [ %v77291, %edge7104 ]
  %v77279 = phi i64 [ %v75595, %edge7099 ], [ %v77292, %edge7104 ]
  store i32 %v77238, ptr %t4596.pre-phi, align 1
  %t4597 = icmp eq i64 %v75595, %v77279
  %t4599.not154 = icmp eq i64 %v77279, 0
  %t4599.not = or i1 %t4597, %t4599.not154
  br i1 %t4599.not, label %block40807, label %edge7107

block40807:                                       ; preds = %block40802, %edge7107
  %or.cond1227 = or i1 %t4485.not, %t4597
  br i1 %or.cond1227, label %edge7325, label %edge7305

block41304:                                       ; preds = %edge7174, %edge7173
  %t4626 = zext i64 %v75635 to i128
  %v218512 = or disjoint i128 %t4626, 5902958103587056517120
  store i128 %v218512, ptr %ss0, align 16
  %t4630 = zext i64 %v75627 to i128
  %v218514 = or disjoint i128 %t4630, 18446744073709551616
  store i128 %v218514, ptr %t1002, align 16
  %t4636 = zext i64 %v75595 to i128
  %t4637 = shl nuw i128 %t4636, 64
  %v218516 = or disjoint i128 %t4637, 320
  store i128 %v218516, ptr %t1007, align 16
  %v78172 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v75627, i64 4, i64 4, i64 %v3192)
  %t4640.not = icmp eq i64 %v78172, 0
  br i1 %t4640.not, label %edge7179, label %block41299

block41302:                                       ; preds = %edge7179, %edge7174
  %t4643.pre-phi = phi ptr [ %.pre20722, %edge7179 ], [ %t4613, %edge7174 ]
  %v78157 = phi i64 [ %v78173, %edge7179 ], [ %v75635, %edge7174 ]
  %v78159 = phi i64 [ %v78174, %edge7179 ], [ %v75627, %edge7174 ]
  %v78162 = phi i64 [ %v78175, %edge7179 ], [ %v75595, %edge7174 ]
  store i32 %41, ptr %t4643.pre-phi, align 1
  %t4644 = icmp eq i64 %v75595, %v78162
  %t4646.not151 = icmp eq i64 %v78162, 0
  %t4646.not = or i1 %t4644, %t4646.not151
  br i1 %t4646.not, label %block41308, label %edge7182

block41308:                                       ; preds = %block41302, %edge7182
  %v78182 = select i1 %t4644, i64 0, i64 %v75595
  %v78207 = sub nsw i64 256, %v76498745749758
  %v78212 = lshr i64 %v78207, 29
  %v78223 = and i64 %v78212, 7
  %v78228 = add nsw i64 %v76498745749758, -1
  %v78234 = lshr i64 %v78228, %v78223
  %t4654 = icmp ult i64 %v78234, 256
  br i1 %t4654, label %edge7201, label %edge7208

block41299:                                       ; preds = %edge7138, %block41304
  %v78139 = phi i64 [ 103425, %edge7138 ], [ %v78172, %block41304 ]
  %t4650 = insertvalue { i64, i64 } poison, i64 %v78139, 0
  %t4651 = insertvalue { i64, i64 } %t4650, i64 3575, 1
  br label %common.ret

block41798:                                       ; preds = %edge7248, %edge7245
  br i1 %t4646.not151, label %block41801, label %edge7252

block41801:                                       ; preds = %edge7252, %block41798
  %t4678 = zext i64 %v78157 to i128
  %v218518 = or disjoint i128 %t4678, 5902958103587056517120
  store i128 %v218518, ptr %ss0, align 16
  %t4682 = zext i64 %v78159 to i128
  %v218520 = or disjoint i128 %t4682, 18446744073709551616
  store i128 %v218520, ptr %t1002, align 16
  %t4688 = zext i64 %v78162 to i128
  %t4689 = shl nuw i128 %t4688, 64
  %v218522 = or disjoint i128 %t4689, 320
  store i128 %v218522, ptr %t1007, align 16
  %v79082 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v78159, i64 4, i64 4, i64 %v3192)
  %t4692.not = icmp eq i64 %v79082, 0
  br i1 %t4692.not, label %edge7257, label %block41792

block41795:                                       ; preds = %edge7257, %edge7252
  %t4695.pre-phi = phi ptr [ %.pre20724, %edge7257 ], [ %t4663, %edge7252 ]
  %v79067 = phi i64 [ %v79083, %edge7257 ], [ %v78157, %edge7252 ]
  %v79069 = phi i64 [ %v79084, %edge7257 ], [ %v78159, %edge7252 ]
  %v79072 = phi i64 [ %v79085, %edge7257 ], [ %v78162, %edge7252 ]
  store i32 %42, ptr %t4695.pre-phi, align 1
  %t4696 = icmp eq i64 %v78162, %v79072
  %t4698.not152 = icmp eq i64 %v79072, 0
  %t4698.not = or i1 %t4696, %t4698.not152
  br i1 %t4698.not, label %block41805, label %edge7260

block41805:                                       ; preds = %block41795, %edge7260
  %t4702.not = or i1 %t4646.not151, %t4696
  br i1 %t4702.not, label %edge7325, label %edge7265

block41792:                                       ; preds = %edge7208, %edge7265, %block41801, %edge7248
  %v79045 = phi i64 [ %v79082, %block41801 ], [ 103425, %edge7208 ], [ %v79066, %edge7248 ], [ %v79096, %edge7265 ]
  %t4704 = insertvalue { i64, i64 } poison, i64 %v79045, 0
  %t4705 = insertvalue { i64, i64 } %t4704, i64 3625, 1
  br label %common.ret

block41842:                                       ; preds = %edge7300, %edge7291, %edge7308
  %v79161 = phi i64 [ %v79184, %edge7308 ], [ %v79168, %edge7291 ], [ %v79180, %edge7300 ]
  %t4723 = insertvalue { i64, i64 } poison, i64 %v79161, 0
  %t4724 = insertvalue { i64, i64 } %t4723, i64 3636, 1
  br label %common.ret

block42349:                                       ; preds = %edge7344, %edge7360
  %t4731.not21195 = phi i1 [ false, %edge7360 ], [ true, %edge7344 ]
  %v7983621192 = phi i32 [ %v7983621172, %edge7360 ], [ %v79836, %edge7344 ]
  %v7949921187 = phi i64 [ %v75595, %edge7360 ], [ 0, %edge7344 ]
  %t4736 = zext i64 %v75635 to i128
  %v218524 = or disjoint i128 %t4736, 5902958103587056517120
  store i128 %v218524, ptr %ss0, align 16
  %t4740 = zext i64 %v75627 to i128
  %v218526 = or disjoint i128 %t4740, 18446744073709551616
  store i128 %v218526, ptr %t1002, align 16
  %t4746 = zext i64 %v7949921187 to i128
  %t4747 = shl nuw i128 %t4746, 64
  %v218528 = or disjoint i128 %t4747, 320
  store i128 %v218528, ptr %t1007, align 16
  %v79887 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v75627, i64 4, i64 4, i64 %v3192)
  %t4750.not = icmp eq i64 %v79887, 0
  br i1 %t4750.not, label %block42348, label %block42344

block42348:                                       ; preds = %block42349
  %v79888 = load i64, ptr %ss1, align 8
  %v79889 = load i64, ptr %t606, align 8
  %v79890 = load i64, ptr %t607, align 8
  %.pre20679 = add i64 %v79888, 1024
  %.pre20680 = inttoptr i64 %.pre20679 to ptr
  store i32 %v7983621192, ptr %.pre20680, align 1
  %t4754 = icmp eq i64 %v7949921187, %v79890
  %t4756.not155 = icmp eq i64 %v79890, 0
  %t4756.not = or i1 %t4754, %t4756.not155
  br i1 %t4756.not, label %block42353, label %edge7368

block42353:                                       ; preds = %block42348, %edge7368
  %t4760.not = or i1 %t4731.not21195, %t4754
  br i1 %t4760.not, label %edge6923, label %edge7373

block42344:                                       ; preds = %edge7373, %block42349
  %v79854 = phi i64 [ %v79887, %block42349 ], [ %v79901, %edge7373 ]
  %t4762 = insertvalue { i64, i64 } poison, i64 %v79854, 0
  %t4763 = insertvalue { i64, i64 } %t4762, i64 3672, 1
  br label %common.ret

block3658:                                        ; preds = %block3658.preheader, %edge7439
  %v81056 = phi i64 [ %v81633, %edge7439 ], [ 0, %block3658.preheader ]
  %exitcond20586.not = icmp eq i64 %v81056, 320
  br i1 %exitcond20586.not, label %edge7440, label %edge7439

block43018:                                       ; preds = %edge7443, %edge7440
  %t4790.not = icmp eq i64 %v64793, 0
  br i1 %t4790.not, label %edge10759, label %edge7447

block43014:                                       ; preds = %edge7450, %edge7443
  %v81071 = phi i64 [ %v81079, %edge7443 ], [ %v81083, %edge7450 ]
  %t4795 = insertvalue { i64, i64 } poison, i64 %v81071, 0
  %t4796 = insertvalue { i64, i64 } %t4795, i64 3706, 1
  br label %common.ret

block43681:                                       ; preds = %edge7488, %edge7485
  %t4806.not = icmp eq i64 %v64793, 0
  br i1 %t4806.not, label %block43915, label %edge7494

block43677:                                       ; preds = %edge7494, %edge7488
  %v82000 = phi i64 [ %v82007, %edge7488 ], [ %v82011, %edge7494 ]
  %t4808 = insertvalue { i64, i64 } poison, i64 %v82000, 0
  %t4809 = insertvalue { i64, i64 } %t4808, i64 3751, 1
  br label %common.ret

block43915:                                       ; preds = %edge7494, %block43681, %edge7409, %edge7406, %edge7385, %edge7381
  %t4810.not = icmp eq i64 %v64805, 0
  br i1 %t4810.not, label %block43918, label %edge7516

block43918:                                       ; preds = %edge7516, %block43915
  %t4812.not = icmp eq i64 %v64916, 0
  br i1 %t4812.not, label %block43922, label %edge7522

block43922:                                       ; preds = %edge7522, %block43918
  %t4814.not = icmp eq i64 %v64928, 0
  br i1 %t4814.not, label %block43926, label %edge7528

block43926:                                       ; preds = %edge7528, %block43922
  %t4816.not = icmp eq i64 %v64940, 0
  br i1 %t4816.not, label %edge7561, label %edge7534

block43914:                                       ; preds = %edge7534, %edge7528, %edge7522, %edge7516
  %v82336 = phi i64 [ %v82355, %edge7528 ], [ %v82347, %edge7516 ], [ %v82351, %edge7522 ], [ %v82359, %edge7534 ]
  %t4818 = insertvalue { i64, i64 } poison, i64 %v82336, 0
  %t4819 = insertvalue { i64, i64 } %t4818, i64 3773, 1
  br label %common.ret

block3784:                                        ; preds = %edge10575, %edge7629
  %v217478 = phi i64 [ 0, %edge7629 ], [ %v124637, %edge10575 ]
  %v217479 = phi i64 [ %v80926, %edge7629 ], [ %v124699971, %edge10575 ]
  %v217480 = phi i64 [ 320, %edge7629 ], [ %v124700972, %edge10575 ]
  %v217482 = phi i64 [ 320, %edge7629 ], [ %v124703973, %edge10575 ]
  %v217483 = phi i64 [ %v80920, %edge7629 ], [ %v124336970, %edge10575 ]
  %v217484 = phi i64 [ %v80926, %edge7629 ], [ %v113115886, %edge10575 ]
  %v217485 = phi i64 [ 320, %edge7629 ], [ %v113116888, %edge10575 ]
  %v217486 = phi i64 [ %v80928, %edge7629 ], [ %v113117889, %edge10575 ]
  %v217488 = phi i64 [ 320, %edge7629 ], [ %v113119891, %edge10575 ]
  %v217489 = phi i64 [ %v80920, %edge7629 ], [ %v113120894, %edge10575 ]
  %v217490 = phi i64 [ %v80926, %edge7629 ], [ %v124233, %edge10575 ]
  %v217491 = phi i64 [ 320, %edge7629 ], [ %v124234, %edge10575 ]
  %v217493 = phi i64 [ 320, %edge7629 ], [ %v124237, %edge10575 ]
  %v217494 = phi i64 [ %v80920, %edge7629 ], [ %v124238, %edge10575 ]
  %v217495 = phi i64 [ %v80926, %edge7629 ], [ %v124648, %edge10575 ]
  %v217496 = phi i64 [ %v80920, %edge7629 ], [ %v124335, %edge10575 ]
  %v217497 = phi i64 [ 0, %edge7629 ], [ %v124659, %edge10575 ]
  %v217498 = phi i64 [ 0, %edge7629 ], [ %v124662, %edge10575 ]
  %v217499 = phi i64 [ 0, %edge7629 ], [ %v124665, %edge10575 ]
  %v217500 = phi i1 [ false, %edge7629 ], [ %t6278, %edge10575 ]
  %v217502 = phi i64 [ 288, %edge7629 ], [ %spec.select33, %edge10575 ]
  %v217504 = phi i64 [ 14, %edge7629 ], [ %spec.select34, %edge10575 ]
  %v84843 = call i64 @mech_compact_allocate(i64 %v0, i64 %v217502, i64 4, i64 4, i64 1, i64 %v3192)
  %t4854.not = icmp eq i64 %v84843, 0
  br i1 %t4854.not, label %edge7663, label %block44334

block44334:                                       ; preds = %block3784
  %t4860 = insertvalue { i64, i64 } poison, i64 %v84843, 0
  %t4861 = insertvalue { i64, i64 } %t4860, i64 3832, 1
  br label %common.ret

block44389:                                       ; preds = %edge7701
  %t4889 = insertvalue { i64, i64 } poison, i64 %v85720, 0
  %t4890 = insertvalue { i64, i64 } %t4889, i64 3845, 1
  br label %common.ret

block3810:                                        ; preds = %edge7703, %edge7704
  %brmerge.not = and i1 %t3962, %v217500
  %.mux = select i1 %v217500, i64 288, i64 0
  br i1 %brmerge.not, label %edge7716, label %edge7732

block3829:                                        ; preds = %edge7853, %edge7732
  %v86683 = phi i64 [ %v85725, %edge7732 ], [ %v2175161146, %edge7853 ]
  %v86708 = phi i64 [ %v85724, %edge7732 ], [ %v2175151145, %edge7853 ]
  %v86711 = phi i64 [ 0, %edge7732 ], [ %v88388, %edge7853 ]
  %v86712 = phi i64 [ 0, %edge7732 ], [ %v2175171147, %edge7853 ]
  %v86717 = phi i64 [ %v85723, %edge7732 ], [ %v2175141144, %edge7853 ]
  %t4902.not = icmp eq i64 %v86683, 0
  br i1 %t4902.not, label %edge7763, label %edge7733

block45599:                                       ; preds = %edge7814, %edge7813
  %t4976 = zext i64 %v86717 to i128
  %v218544 = or disjoint i128 %t4978, %t4976
  store i128 %v218544, ptr %ss0, align 16
  %t4980 = zext i64 %v86708 to i128
  %v218546 = or disjoint i128 %t4980, 18446744073709551616
  store i128 %v218546, ptr %t1002, align 16
  %t4986 = zext i64 %v86683 to i128
  %t4987 = shl nuw i128 %t4986, 64
  %v218548 = or disjoint i128 %t4987, %t4977
  store i128 %v218548, ptr %t1007, align 16
  %v87785 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v86708, i64 4, i64 4, i64 %v3192)
  %t4990.not = icmp eq i64 %v87785, 0
  br i1 %t4990.not, label %edge7819, label %block45594

block45598:                                       ; preds = %edge7814, %edge7819
  %t4993.pre-phi = phi ptr [ %t4967, %edge7814 ], [ %.pre20694, %edge7819 ]
  %v87770 = phi i64 [ %v86717, %edge7814 ], [ %v87786, %edge7819 ]
  %v87772 = phi i64 [ %v86708, %edge7814 ], [ %v87787, %edge7819 ]
  %v87775 = phi i64 [ %v86683, %edge7814 ], [ %v87788, %edge7819 ]
  store i32 %43, ptr %t4993.pre-phi, align 1
  br label %block45597

block45597:                                       ; preds = %edge7810, %block45598
  %v87764 = phi i64 [ %v87770, %block45598 ], [ %v86717, %edge7810 ]
  %v87766 = phi i64 [ %v87772, %block45598 ], [ %v86708, %edge7810 ]
  %v87769 = phi i64 [ %v87775, %block45598 ], [ %v86683, %edge7810 ]
  %t4994 = icmp eq i64 %v86683, %v87769
  %t4996.not157 = icmp eq i64 %v87769, 0
  %t4996.not = or i1 %t4994, %t4996.not157
  br i1 %t4996.not, label %block45603, label %edge7822

block45603:                                       ; preds = %block45597, %edge7822
  %t5000.not = or i1 %t4902.not, %t4994
  br i1 %t5000.not, label %edge7853, label %edge7827

block45594:                                       ; preds = %edge7827, %block45599, %edge7808
  %v87752 = phi i64 [ %v87785, %block45599 ], [ 103425, %edge7808 ], [ %v87799, %edge7827 ]
  %t5002 = insertvalue { i64, i64 } poison, i64 %v87752, 0
  %t5003 = insertvalue { i64, i64 } %t5002, i64 3925, 1
  br label %common.ret

block3911:                                        ; preds = %edge8072, %edge7877
  %v88599 = phi i64 [ %v86683, %edge7877 ], [ %v217523, %edge8072 ]
  %v88626 = phi i64 [ %v86708, %edge7877 ], [ %v217522, %edge8072 ]
  %v88628 = phi i64 [ 4362938489, %edge7877 ], [ %v91940, %edge8072 ]
  %v88629 = phi i64 [ %v86717, %edge7877 ], [ %v217521, %edge8072 ]
  %t5018.not = icmp eq i64 %v88599, 0
  br i1 %t5018.not, label %edge7899, label %edge7878

block3929:                                        ; preds = %edge7899, %edge8057
  %v88999 = phi i64 [ %v90909, %edge8057 ], [ %v88629, %edge7899 ]
  %v89001 = phi i64 [ %v90911, %edge8057 ], [ %v88626, %edge7899 ]
  %v89004 = phi i64 [ %v90914, %edge8057 ], [ %v88599, %edge7899 ]
  %v89017 = phi i64 [ %v91550, %edge8057 ], [ %v88676, %edge7899 ]
  %exitcond20589.not = icmp eq i64 %v89017, %v88453
  br i1 %exitcond20589.not, label %block4052, label %edge7935

block3964:                                        ; preds = %edge8037, %edge7955
  %v89670 = phi i64 [ %v88999, %edge7955 ], [ %v90909, %edge8037 ]
  %v89672 = phi i64 [ %v89001, %edge7955 ], [ %v90911, %edge8037 ]
  %v89675 = phi i64 [ %v89004, %edge7955 ], [ %v90914, %edge8037 ]
  %v89685 = phi i64 [ %v89017, %edge7955 ], [ %spec.select321, %edge8037 ]
  %t5063.not = icmp ult i64 %v89685, %v88676
  %v89973 = sub i64 %v89685, %v88676
  %spec.select321 = select i1 %t5063.not, i64 %v89685, i64 %v89973
  br i1 %t5063.not, label %edge7981, label %edge7991

block4019:                                        ; preds = %edge7981, %edge7991
  %v217518 = phi i1 [ true, %edge7991 ], [ %t5077, %edge7981 ]
  %v217519 = phi i64 [ %v89633, %edge7991 ], [ %spec.select30, %edge7981 ]
  %t5080 = icmp ult i64 %v89685, %v217502
  br i1 %t5080, label %edge8016, label %block47701

block47706:                                       ; preds = %edge8020, %edge8019
  %t5091 = zext i64 %v89670 to i128
  %v218558 = or disjoint i128 %t4978, %t5091
  store i128 %v218558, ptr %ss0, align 16
  %t5095 = zext i64 %v89672 to i128
  %v218560 = or disjoint i128 %t5095, 18446744073709551616
  store i128 %v218560, ptr %t1002, align 16
  %t5101 = zext i64 %v89675 to i128
  %t5102 = shl nuw i128 %t5101, 64
  %v218562 = or disjoint i128 %t5102, %t4977
  store i128 %v218562, ptr %t1007, align 16
  %v90930 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v89672, i64 4, i64 4, i64 %v3192)
  %t5105.not = icmp eq i64 %v90930, 0
  br i1 %t5105.not, label %edge8025, label %block47701

block47705:                                       ; preds = %edge8020, %edge8025
  %t5108.pre-phi = phi ptr [ %t5082, %edge8020 ], [ %.pre20696, %edge8025 ]
  %v90915 = phi i64 [ %v89670, %edge8020 ], [ %v90931, %edge8025 ]
  %v90917 = phi i64 [ %v89672, %edge8020 ], [ %v90932, %edge8025 ]
  %v90920 = phi i64 [ %v89675, %edge8020 ], [ %v90933, %edge8025 ]
  %v90935 = trunc nuw i64 %v217519 to i32
  store i32 %v90935, ptr %t5108.pre-phi, align 1
  br label %block47704

block47704:                                       ; preds = %edge8016, %block47705
  %v90909 = phi i64 [ %v90915, %block47705 ], [ %v89670, %edge8016 ]
  %v90911 = phi i64 [ %v90917, %block47705 ], [ %v89672, %edge8016 ]
  %v90914 = phi i64 [ %v90920, %block47705 ], [ %v89675, %edge8016 ]
  %t5109 = icmp eq i64 %v89675, %v90914
  %t5111.not160 = icmp eq i64 %v90914, 0
  %t5111.not = or i1 %t5109, %t5111.not160
  br i1 %t5111.not, label %block47710, label %edge8028

block47710:                                       ; preds = %block47704, %edge8028
  %t5115.not161 = icmp eq i64 %v89675, 0
  %t5115.not = or i1 %t5115.not161, %t5109
  br i1 %t5115.not, label %edge8037, label %edge8033

block47701:                                       ; preds = %edge8033, %block47706, %block4019
  %v90897 = phi i64 [ %v90930, %block47706 ], [ 103425, %block4019 ], [ %v90944, %edge8033 ]
  %t5117 = insertvalue { i64, i64 } poison, i64 %v90897, 0
  %t5118 = insertvalue { i64, i64 } %t5117, i64 4070, 1
  br label %common.ret

block4052:                                        ; preds = %block3929, %edge7899
  %v217521 = phi i64 [ %v88629, %edge7899 ], [ %v88999, %block3929 ]
  %v217522 = phi i64 [ %v88626, %edge7899 ], [ %v89001, %block3929 ]
  %v217523 = phi i64 [ %v88599, %edge7899 ], [ %v89004, %block3929 ]
  br i1 %t5051, label %edge8072, label %edge8091

block4113:                                        ; preds = %edge8102, %edge8099, %edge8138
  %v217524 = phi i64 [ %v93651, %edge8138 ], [ 1, %edge8099 ], [ 1, %edge8102 ]
  br i1 %t4857.not, label %edge8164, label %edge8151

block49339:                                       ; preds = %edge8164.thread, %edge8164
  %t5184 = zext i64 %v84846 to i128
  %t5185 = zext nneg i64 %v217502 to i128
  %t5186 = shl nuw nsw i128 %t5185, 64
  %v218564 = or disjoint i128 %t5186, %t5184
  store i128 %v218564, ptr %ss0, align 16
  %t5188 = zext i64 %v84847 to i128
  %v218566 = or disjoint i128 %t5188, 18446744073709551616
  store i128 %v218566, ptr %t1002, align 16
  %t5194 = zext i64 %v84848 to i128
  %t5195 = shl nuw i128 %t5194, 64
  %v218568 = or disjoint i128 %t5195, %t5185
  store i128 %v218568, ptr %t1007, align 16
  %v93466 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v84847, i64 4, i64 4, i64 %v3192)
  %t5198.not = icmp eq i64 %v93466, 0
  br i1 %t5198.not, label %block49337, label %block49314

block49337:                                       ; preds = %block49339
  %v93467 = load i64, ptr %ss1, align 8
  %v93468 = load i64, ptr %t606, align 8
  %v93469 = load i64, ptr %t607, align 8
  %t5201 = inttoptr i64 %v93467 to ptr
  store i32 65536, ptr %t5201, align 1
  %t5202 = icmp eq i64 %v84848, %v93469
  %t5204.not192 = icmp eq i64 %v93469, 0
  %t5204.not = or i1 %t5202, %t5204.not192
  br i1 %t5204.not, label %block49343, label %edge8176

block49343:                                       ; preds = %block49337, %edge8176
  %t5208.not = or i1 %t4857.not, %t5202
  br i1 %t5208.not, label %edge8185, label %edge8181

block49314:                                       ; preds = %edge8181, %block49339
  %v93380 = phi i64 [ %v93466, %block49339 ], [ %v93480, %edge8181 ]
  %t5210 = insertvalue { i64, i64 } poison, i64 %v93380, 0
  %t5211 = insertvalue { i64, i64 } %t5210, i64 4164, 1
  br label %common.ret

block49354:                                       ; preds = %edge8191, %edge8190
  %t5223 = zext i64 %v93445384407 to i128
  %t5224 = zext nneg i64 %v217502 to i128
  %t5225 = shl nuw nsw i128 %t5224, 64
  %v218570 = or disjoint i128 %t5225, %t5223
  store i128 %v218570, ptr %ss0, align 16
  %t5227 = zext i64 %v93447385406 to i128
  %v218572 = or disjoint i128 %t5227, 18446744073709551616
  store i128 %v218572, ptr %t1002, align 16
  %t5233 = zext i64 %v93450386405 to i128
  %t5234 = shl nuw i128 %t5233, 64
  %v218574 = or disjoint i128 %t5234, %t5224
  store i128 %v218574, ptr %t1007, align 16
  %v93814 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v93447385406, i64 4, i64 4, i64 %v3192)
  %t5237.not = icmp eq i64 %v93814, 0
  br i1 %t5237.not, label %edge8196, label %block49349

block49352:                                       ; preds = %edge8196, %edge8191
  %t5240.pre-phi = phi ptr [ %.pre20692, %edge8196 ], [ %t5214, %edge8191 ]
  %v93799 = phi i64 [ %v93815, %edge8196 ], [ %v93445384407, %edge8191 ]
  %v93801 = phi i64 [ %v93816, %edge8196 ], [ %v93447385406, %edge8191 ]
  %v93804 = phi i64 [ %v93817, %edge8196 ], [ %v93450386405, %edge8191 ]
  store i32 65537, ptr %t5240.pre-phi, align 1
  %t5241 = icmp eq i64 %v93450386405, %v93804
  %t5243.not194 = icmp eq i64 %v93804, 0
  %t5243.not = or i1 %t5241, %t5243.not194
  br i1 %t5243.not, label %block49358, label %edge8199

block49358:                                       ; preds = %block49352, %edge8199
  %t5247.not = or i1 %t5204.not192389404, %t5241
  br i1 %t5247.not, label %edge8095, label %edge8204

block49349:                                       ; preds = %edge8204, %block49354, %edge8185
  %v93781 = phi i64 [ %v93814, %block49354 ], [ 103425, %edge8185 ], [ %v93828, %edge8204 ]
  %t5249 = insertvalue { i64, i64 } poison, i64 %v93781, 0
  %t5250 = insertvalue { i64, i64 } %t5249, i64 4166, 1
  br label %common.ret

block4122:                                        ; preds = %block4122.backedge, %edge8094
  %v93873 = phi i64 [ %v92031, %edge8094 ], [ %v93873.be, %block4122.backedge ]
  %v93893 = phi i64 [ %v92028, %edge8094 ], [ %v93893.be, %block4122.backedge ]
  %v93894 = phi i64 [ 0, %edge8094 ], [ %v98719, %block4122.backedge ]
  %v93895 = phi i64 [ 0, %edge8094 ], [ %spec.select327, %block4122.backedge ]
  %v93896 = phi i64 [ 0, %edge8094 ], [ %v98660840, %block4122.backedge ]
  %v93898 = phi i64 [ %v92026, %edge8094 ], [ %v93898.be, %block4122.backedge ]
  %t5251.not = icmp eq i64 %v93873, 0
  br i1 %t5251.not, label %edge8236, label %edge8216

block50804:                                       ; preds = %edge8388, %edge8387
  %t5342 = zext i64 %v93898 to i128
  %v218586 = or disjoint i128 %t4978, %t5342
  store i128 %v218586, ptr %ss0, align 16
  %t5346 = zext i64 %v93893 to i128
  %v218588 = or disjoint i128 %t5346, 18446744073709551616
  store i128 %v218588, ptr %t1002, align 16
  %t5352 = zext i64 %v93873 to i128
  %t5353 = shl nuw i128 %t5352, 64
  %v218590 = or disjoint i128 %t5353, %t4977
  store i128 %v218590, ptr %t1007, align 16
  %v96167 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v93893, i64 4, i64 4, i64 %v3192)
  %t5356.not = icmp eq i64 %v96167, 0
  br i1 %t5356.not, label %edge8393, label %block50799

block50803:                                       ; preds = %edge8388, %edge8393
  %t5359.pre-phi = phi ptr [ %t5333, %edge8388 ], [ %.pre20702, %edge8393 ]
  %v96152 = phi i64 [ %v93898, %edge8388 ], [ %v96168, %edge8393 ]
  %v96154 = phi i64 [ %v93893, %edge8388 ], [ %v96169, %edge8393 ]
  %v96157 = phi i64 [ %v93873, %edge8388 ], [ %v96170, %edge8393 ]
  %v96172 = trunc nuw i64 %v96121 to i32
  store i32 %v96172, ptr %t5359.pre-phi, align 1
  br label %block50802

block50802:                                       ; preds = %edge8384, %block50803
  %v96146 = phi i64 [ %v96152, %block50803 ], [ %v93898, %edge8384 ]
  %v96148 = phi i64 [ %v96154, %block50803 ], [ %v93893, %edge8384 ]
  %v96151 = phi i64 [ %v96157, %block50803 ], [ %v93873, %edge8384 ]
  %t5360 = icmp eq i64 %v93873, %v96151
  %t5362.not167 = icmp eq i64 %v96151, 0
  %t5362.not = or i1 %t5360, %t5362.not167
  br i1 %t5362.not, label %block50808, label %edge8396

block50808:                                       ; preds = %block50802, %edge8396
  %t5366.not = or i1 %t5251.not, %t5360
  br i1 %t5366.not, label %edge8372, label %edge8401

block50799:                                       ; preds = %edge8401, %block50804, %edge8377
  %v96134 = phi i64 [ %v96167, %block50804 ], [ 103425, %edge8377 ], [ %v96181, %edge8401 ]
  %t5368 = insertvalue { i64, i64 } poison, i64 %v96134, 0
  %t5369 = insertvalue { i64, i64 } %t5368, i64 4306, 1
  br label %common.ret

block51036:                                       ; preds = %edge8425, %edge8424
  %t5383 = zext i64 %v96146 to i128
  %v218592 = or disjoint i128 %t4978, %t5383
  store i128 %v218592, ptr %ss0, align 16
  %t5387 = zext i64 %v96148 to i128
  %v218594 = or disjoint i128 %t5387, 18446744073709551616
  store i128 %v218594, ptr %t1002, align 16
  %t5393 = zext i64 %v96151 to i128
  %t5394 = shl nuw i128 %t5393, 64
  %v218596 = or disjoint i128 %t5394, %t4977
  store i128 %v218596, ptr %t1007, align 16
  %v96528 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v96148, i64 4, i64 4, i64 %v3192)
  %t5397.not = icmp eq i64 %v96528, 0
  br i1 %t5397.not, label %edge8430, label %block51031

block51034:                                       ; preds = %edge8430, %edge8425
  %t5400.pre-phi = phi ptr [ %.pre20700, %edge8430 ], [ %t5374, %edge8425 ]
  %v96513 = phi i64 [ %v96529, %edge8430 ], [ %v96146, %edge8425 ]
  %v96515 = phi i64 [ %v96530, %edge8430 ], [ %v96148, %edge8425 ]
  %v96518 = phi i64 [ %v96531, %edge8430 ], [ %v96151, %edge8425 ]
  %v96533 = trunc nuw i64 %v96476 to i32
  store i32 %v96533, ptr %t5400.pre-phi, align 1
  %t5401 = icmp eq i64 %v96151, %v96518
  %t5403.not170 = icmp eq i64 %v96518, 0
  %t5403.not = or i1 %t5401, %t5403.not170
  br i1 %t5403.not, label %block51040, label %edge8433

block51040:                                       ; preds = %block51034, %edge8433
  %t5407.not = or i1 %t5362.not167, %t5401
  br i1 %t5407.not, label %edge8450, label %edge8438

block51031:                                       ; preds = %edge8438, %block51036, %edge8412
  %v96489 = phi i64 [ %v96528, %block51036 ], [ 103425, %edge8412 ], [ %v96542, %edge8438 ]
  %t5409 = insertvalue { i64, i64 } poison, i64 %v96489, 0
  %t5410 = insertvalue { i64, i64 } %t5409, i64 4318, 1
  br label %common.ret

block52568:                                       ; preds = %edge8543, %edge8542
  %t5443 = zext i64 %v98292833 to i128
  %v218598 = or disjoint i128 %t4978, %t5443
  store i128 %v218598, ptr %ss0, align 16
  %t5447 = zext i64 %v98294834 to i128
  %v218600 = or disjoint i128 %t5447, 18446744073709551616
  store i128 %v218600, ptr %t1002, align 16
  %t5453 = zext i64 %v98297835 to i128
  %t5454 = shl nuw i128 %t5453, 64
  %v218602 = or disjoint i128 %t5454, %t4977
  store i128 %v218602, ptr %t1007, align 16
  %v98695 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v98294834, i64 4, i64 4, i64 %v3192)
  %t5457.not = icmp eq i64 %v98695, 0
  br i1 %t5457.not, label %edge8548, label %block52563

block52566:                                       ; preds = %edge8548, %edge8543
  %t5460.pre-phi = phi ptr [ %.pre20698, %edge8548 ], [ %t5430, %edge8543 ]
  %v98680 = phi i64 [ %v98696, %edge8548 ], [ %v98292833, %edge8543 ]
  %v98682 = phi i64 [ %v98697, %edge8548 ], [ %v98294834, %edge8543 ]
  %v98685 = phi i64 [ %v98698, %edge8548 ], [ %v98297835, %edge8543 ]
  %v98700 = trunc nuw i64 %v98649839 to i32
  store i32 %v98700, ptr %t5460.pre-phi, align 1
  %t5461 = icmp eq i64 %v98297835, %v98685
  %t5463.not172 = icmp eq i64 %v98685, 0
  %t5463.not = or i1 %t5461, %t5463.not172
  br i1 %t5463.not, label %block52572, label %edge8551

block52572:                                       ; preds = %block52566, %edge8551
  %t5467.not = or i1 %t5438.not, %t5461
  br i1 %t5467.not, label %block4122.backedge, label %edge8556

block52563:                                       ; preds = %edge8450, %edge8556, %block52568
  %v98662 = phi i64 [ %v98695, %block52568 ], [ %v98709, %edge8556 ], [ 103425, %edge8450 ]
  %t5469 = insertvalue { i64, i64 } poison, i64 %v98662, 0
  %t5470 = insertvalue { i64, i64 } %t5469, i64 4404, 1
  br label %common.ret

block52864:                                       ; preds = %edge8580
  %t5503 = insertvalue { i64, i64 } poison, i64 %v99918, 0
  %t5504 = insertvalue { i64, i64 } %t5503, i64 4434, 1
  br label %common.ret

block52906:                                       ; preds = %edge8626, %edge8630
  %t5515 = zext i64 %v99921 to i128
  %t5516 = zext nneg i64 %v99108 to i128
  %t5517 = shl nuw nsw i128 %t5516, 64
  %v218604 = or disjoint i128 %t5517, %t5515
  store i128 %v218604, ptr %ss0, align 16
  %t5519 = zext i64 %v99922 to i128
  %v218606 = or disjoint i128 %t5519, 18446744073709551616
  store i128 %v218606, ptr %t1002, align 16
  %t5525 = zext i64 %v99923 to i128
  %t5526 = shl nuw i128 %t5525, 64
  %v218608 = or disjoint i128 %t5526, %t5516
  store i128 %v218608, ptr %t1007, align 16
  %v100075 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v99922, i64 8, i64 8, i64 %v3192)
  %t5529.not = icmp eq i64 %v100075, 0
  br i1 %t5529.not, label %block52904, label %block52901

block52904:                                       ; preds = %block52906
  %v100076 = load i64, ptr %ss1, align 8
  %v100077 = load i64, ptr %t606, align 8
  %v100078 = load i64, ptr %t607, align 8
  %v100079 = add i64 %v100076, 8
  %t5532 = inttoptr i64 %v100079 to ptr
  store i64 2, ptr %t5532, align 1
  %t5533 = icmp eq i64 %v99923, %v100078
  %t5535.not174 = icmp eq i64 %v100078, 0
  %t5535.not = or i1 %t5533, %t5535.not174
  br i1 %t5535.not, label %block52910, label %edge8638

block52910:                                       ; preds = %block52904, %edge8638
  %t5539.not = or i1 %t5500.not, %t5533
  br i1 %t5539.not, label %edge8656, label %edge8641

block52901:                                       ; preds = %edge8644, %block52906
  %v100042 = phi i64 [ %v100075, %block52906 ], [ %v100088, %edge8644 ]
  %t5544 = insertvalue { i64, i64 } poison, i64 %v100042, 0
  %t5545 = insertvalue { i64, i64 } %t5544, i64 4443, 1
  br label %common.ret

block53164:                                       ; preds = %edge8683, %edge8682
  %t5562 = zext i64 %v93898 to i128
  %v218610 = or disjoint i128 %t4978, %t5562
  store i128 %v218610, ptr %ss0, align 16
  %t5566 = zext i64 %v93893 to i128
  %v218612 = or disjoint i128 %t5566, 18446744073709551616
  store i128 %v218612, ptr %t1002, align 16
  %t5572 = zext i64 %v93873 to i128
  %t5573 = shl nuw i128 %t5572, 64
  %v218614 = or disjoint i128 %t5573, %t4977
  store i128 %v218614, ptr %t1007, align 16
  %v100511 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v93893, i64 4, i64 4, i64 %v3192)
  %t5576.not = icmp eq i64 %v100511, 0
  br i1 %t5576.not, label %edge8688, label %block53159

block53163:                                       ; preds = %edge8683, %edge8688
  %t5579.pre-phi = phi ptr [ %t5549, %edge8683 ], [ %.pre20718, %edge8688 ]
  %v100496 = phi i64 [ %v93898, %edge8683 ], [ %v100512, %edge8688 ]
  %v100498 = phi i64 [ %v93893, %edge8683 ], [ %v100513, %edge8688 ]
  %v100501 = phi i64 [ %v93873, %edge8683 ], [ %v100514, %edge8688 ]
  store i32 %52, ptr %t5579.pre-phi, align 1
  br label %block53162

block53162:                                       ; preds = %edge8664, %block53163
  %v100490 = phi i64 [ %v100496, %block53163 ], [ %v93898, %edge8664 ]
  %v100492 = phi i64 [ %v100498, %block53163 ], [ %v93893, %edge8664 ]
  %v100495 = phi i64 [ %v100501, %block53163 ], [ %v93873, %edge8664 ]
  %t5580 = icmp eq i64 %v93873, %v100495
  %t5582.not176 = icmp eq i64 %v100495, 0
  %t5582.not = or i1 %t5580, %t5582.not176
  br i1 %t5582.not, label %block53168, label %edge8691

block53168:                                       ; preds = %block53162, %edge8691
  %t5586.not = or i1 %t5251.not23858, %t5580
  br i1 %t5586.not, label %edge8707, label %edge8694

block53159:                                       ; preds = %edge8656, %edge8697, %block53164
  %v100478 = phi i64 [ %v100511, %block53164 ], [ 103425, %edge8656 ], [ %v100525, %edge8697 ]
  %t5591 = insertvalue { i64, i64 } poison, i64 %v100478, 0
  %t5592 = insertvalue { i64, i64 } %t5591, i64 4469, 1
  br label %common.ret

block4414:                                        ; preds = %block4414.backedge, %edge8707
  %v100576 = phi i64 [ %v100490, %edge8707 ], [ %v101666, %block4414.backedge ]
  %v100578 = phi i64 [ %v100492, %edge8707 ], [ %v101668, %block4414.backedge ]
  %v100581 = phi i64 [ %v100495, %edge8707 ], [ %v101671, %block4414.backedge ]
  %v100588 = phi i64 [ %v100054432452, %edge8707 ], [ %v103555, %block4414.backedge ]
  %v100590 = phi i64 [ %v100056433451, %edge8707 ], [ %v103557, %block4414.backedge ]
  %v100593 = phi i64 [ %v100059434450, %edge8707 ], [ %v103560, %block4414.backedge ]
  %v100600 = phi i64 [ %v99091, %edge8707 ], [ %v100894, %block4414.backedge ]
  %t5593.not = icmp eq i64 %v100600, 0
  br i1 %t5593.not, label %edge8985, label %edge8722

block53926:                                       ; preds = %edge8797, %edge8796
  %t5624 = zext i64 %v100576 to i128
  %v218616 = or disjoint i128 %t4978, %t5624
  store i128 %v218616, ptr %ss0, align 16
  %t5628 = zext i64 %v100578 to i128
  %v218618 = or disjoint i128 %t5628, 18446744073709551616
  store i128 %v218618, ptr %t1002, align 16
  %t5634 = zext i64 %v100581 to i128
  %t5635 = shl nuw i128 %t5634, 64
  %v218620 = or disjoint i128 %t5635, %t4977
  store i128 %v218620, ptr %t1007, align 16
  %v101687 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v100578, i64 4, i64 4, i64 %v3192)
  %t5638.not = icmp eq i64 %v101687, 0
  br i1 %t5638.not, label %edge8802, label %block53921

block53925:                                       ; preds = %edge8797, %edge8802
  %t5641.pre-phi = phi ptr [ %t5615, %edge8797 ], [ %.pre20716, %edge8802 ]
  %v101672 = phi i64 [ %v100576, %edge8797 ], [ %v101688, %edge8802 ]
  %v101674 = phi i64 [ %v100578, %edge8797 ], [ %v101689, %edge8802 ]
  %v101677 = phi i64 [ %v100581, %edge8797 ], [ %v101690, %edge8802 ]
  %v101692 = trunc nuw i64 %v101641 to i32
  store i32 %v101692, ptr %t5641.pre-phi, align 1
  br label %block53924

block53924:                                       ; preds = %edge8781, %block53925
  %v101666 = phi i64 [ %v101672, %block53925 ], [ %v100576, %edge8781 ]
  %v101668 = phi i64 [ %v101674, %block53925 ], [ %v100578, %edge8781 ]
  %v101671 = phi i64 [ %v101677, %block53925 ], [ %v100581, %edge8781 ]
  %t5642 = icmp eq i64 %v100581, %v101671
  %t5644.not179 = icmp eq i64 %v101671, 0
  %t5644.not = or i1 %t5642, %t5644.not179
  br i1 %t5644.not, label %block53930, label %edge8805

block53930:                                       ; preds = %block53924, %edge8805
  %v101697 = select i1 %t5642, i64 0, i64 %v100581
  %t5650.not = icmp samesign ult i64 %v101586, %v217504
  br i1 %t5650.not, label %edge8887, label %edge8814

block53921:                                       ; preds = %block53926
  %t5648 = insertvalue { i64, i64 } poison, i64 %v101687, 0
  %t5649 = insertvalue { i64, i64 } %t5648, i64 4542, 1
  br label %common.ret

block4489:                                        ; preds = %edge8830, %edge8820
  %v102027 = phi i64 [ %v217504, %edge8820 ], [ %v102039, %edge8830 ]
  %t5656.not = icmp eq i64 %v102027, 0
  br i1 %t5656.not, label %common.ret, label %edge8830

block54912:                                       ; preds = %edge8906, %edge8899
  %t5687 = zext i64 %v100588 to i128
  %v218622 = or disjoint i128 %t5689, %t5687
  store i128 %v218622, ptr %ss0, align 16
  %t5691 = zext i64 %v100590 to i128
  %v218624 = or disjoint i128 %t5691, 18446744073709551616
  store i128 %v218624, ptr %t1002, align 16
  %t5697 = zext i64 %v100593 to i128
  %t5698 = shl nuw i128 %t5697, 64
  %v218626 = or disjoint i128 %t5698, %t5688
  store i128 %v218626, ptr %t1007, align 16
  %v103144 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v100590, i64 8, i64 8, i64 %v3192)
  %t5701.not = icmp eq i64 %v103144, 0
  br i1 %t5701.not, label %edge8911, label %edge8910

block54911:                                       ; preds = %edge8906, %edge8911
  %t5704.pre-phi = phi ptr [ %t5674, %edge8906 ], [ %.pre20712, %edge8911 ]
  %v103129 = phi i64 [ %v100588, %edge8906 ], [ %v103145, %edge8911 ]
  %v103131 = phi i64 [ %v100590, %edge8906 ], [ %v103146, %edge8911 ]
  %v103134 = phi i64 [ %v100593, %edge8906 ], [ %v103147, %edge8911 ]
  store i64 %v103104, ptr %t5704.pre-phi, align 1
  %t5705 = icmp eq i64 %v100593, %v103134
  %t5707.not180 = icmp eq i64 %v103134, 0
  %t5707.not = or i1 %t5705, %t5707.not180
  br i1 %t5707.not, label %block54916, label %edge8914

block54916:                                       ; preds = %block54911, %edge8914
  %v103153 = select i1 %t5705, i64 0, i64 %v100593
  %t5713 = icmp ult i64 %v103068, %v99108
  br i1 %t5713, label %edge8943, label %block55170

block55176:                                       ; preds = %edge8956, %edge8953
  br i1 %t5707.not180, label %block55179, label %edge8960

block55179:                                       ; preds = %edge8960, %block55176
  %t5733 = zext i64 %v103129 to i128
  %v218628 = or disjoint i128 %t5689, %t5733
  store i128 %v218628, ptr %ss0, align 16
  %t5737 = zext i64 %v103131 to i128
  %v218630 = or disjoint i128 %t5737, 18446744073709551616
  store i128 %v218630, ptr %t1002, align 16
  %t5743 = zext i64 %v103134 to i128
  %t5744 = shl nuw i128 %t5743, 64
  %v218632 = or disjoint i128 %t5744, %t5688
  store i128 %v218632, ptr %t1007, align 16
  %v103570 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v103131, i64 8, i64 8, i64 %v3192)
  %t5747.not = icmp eq i64 %v103570, 0
  br i1 %t5747.not, label %edge8965, label %block55170

block55173:                                       ; preds = %edge8965, %edge8960
  %t5750.pre-phi = phi ptr [ %t5716, %edge8960 ], [ %.pre20714, %edge8965 ]
  %v103555 = phi i64 [ %v103129, %edge8960 ], [ %v103571, %edge8965 ]
  %v103557 = phi i64 [ %v103131, %edge8960 ], [ %v103572, %edge8965 ]
  %v103560 = phi i64 [ %v103134, %edge8960 ], [ %v103573, %edge8965 ]
  store i64 %v103517, ptr %t5750.pre-phi, align 1
  %t5751 = icmp eq i64 %v103134, %v103560
  %t5753.not181 = icmp eq i64 %v103560, 0
  %t5753.not = or i1 %t5751, %t5753.not181
  br i1 %t5753.not, label %block55183, label %edge8968

block55183:                                       ; preds = %block55173, %edge8968
  %t5757.not = or i1 %t5707.not180, %t5751
  br i1 %t5757.not, label %edge9001, label %edge8971

block55170:                                       ; preds = %block54916, %edge8974, %block55179, %edge8956
  %v103533 = phi i64 [ %v103570, %block55179 ], [ %v103583, %edge8974 ], [ %v103554, %edge8956 ], [ 103425, %block54916 ]
  %t5762 = insertvalue { i64, i64 } poison, i64 %v103533, 0
  %t5763 = insertvalue { i64, i64 } %t5762, i64 4644, 1
  br label %common.ret

block55210:                                       ; preds = %edge8985, %edge8986
  %t5767.not = icmp eq i64 %v100593, 0
  br i1 %t5767.not, label %edge9002, label %edge8989

block55238:                                       ; preds = %edge9005, %edge9002
  %t5774.not = icmp eq i64 %v100593, 0
  br i1 %t5774.not, label %edge9073, label %edge9009

block55233:                                       ; preds = %edge9012, %edge9005, %edge9019
  %v103671 = phi i64 [ %v103690, %edge9019 ], [ %v103686, %edge9012 ], [ %v103682, %edge9005 ]
  %t5786 = insertvalue { i64, i64 } poison, i64 %v103671, 0
  %t5787 = insertvalue { i64, i64 } %t5786, i64 4654, 1
  br label %common.ret

block4608:                                        ; preds = %edge9277, %edge9073
  %v104178 = phi i64 [ %v84848, %edge9073 ], [ %v104979, %edge9277 ]
  %v104219 = phi i64 [ %v84847, %edge9073 ], [ %v104976, %edge9277 ]
  %v104220 = phi i64 [ %v217504, %edge9073 ], [ %v106151, %edge9277 ]
  %v104221 = phi i64 [ 0, %edge9073 ], [ %v104989, %edge9277 ]
  %v104226 = phi i64 [ %v84846, %edge9073 ], [ %v104974, %edge9277 ]
  %t5791.not = icmp eq i64 %v104178, 0
  br i1 %t5791.not, label %edge9115, label %edge9074

block55569:                                       ; preds = %edge9095, %edge9088
  %v104177 = phi i64 [ %v104206, %edge9088 ], [ %v104214, %edge9095 ]
  %t5852 = insertvalue { i64, i64 } poison, i64 %v104177, 0
  %t5853 = insertvalue { i64, i64 } %t5852, i64 4673, 1
  br label %common.ret

block4644:                                        ; preds = %edge9237, %edge9152
  %v104974 = phi i64 [ %v104226, %edge9152 ], [ %v105715, %edge9237 ]
  %v104976 = phi i64 [ %v104219, %edge9152 ], [ %v105717, %edge9237 ]
  %v104979 = phi i64 [ %v104178, %edge9152 ], [ %v105720, %edge9237 ]
  %v104989 = phi i64 [ %v104221, %edge9152 ], [ %v105755, %edge9237 ]
  %v104992 = phi i64 [ %v104915, %edge9152 ], [ %v105764, %edge9237 ]
  %t5864.not = icmp eq i64 %v104992, 0
  br i1 %t5864.not, label %edge9169, label %edge9188

block56601:                                       ; preds = %edge9216, %edge9215
  %t5891 = zext i64 %v104974 to i128
  %v218650 = or disjoint i128 %t4978, %t5891
  store i128 %v218650, ptr %ss0, align 16
  %t5895 = zext i64 %v104976 to i128
  %v218652 = or disjoint i128 %t5895, 18446744073709551616
  store i128 %v218652, ptr %t1002, align 16
  %t5901 = zext i64 %v104979 to i128
  %t5902 = shl nuw i128 %t5901, 64
  %v218654 = or disjoint i128 %t5902, %t4977
  store i128 %v218654, ptr %t1007, align 16
  %v105736 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v104976, i64 4, i64 4, i64 %v3192)
  %t5905.not = icmp eq i64 %v105736, 0
  br i1 %t5905.not, label %edge9221, label %block56596

block56600:                                       ; preds = %edge9216, %edge9221
  %t5908.pre-phi = phi ptr [ %t5882, %edge9216 ], [ %.pre20710, %edge9221 ]
  %v105721 = phi i64 [ %v104974, %edge9216 ], [ %v105737, %edge9221 ]
  %v105723 = phi i64 [ %v104976, %edge9216 ], [ %v105738, %edge9221 ]
  %v105726 = phi i64 [ %v104979, %edge9216 ], [ %v105739, %edge9221 ]
  store i32 %v105741, ptr %t5908.pre-phi, align 1
  br label %block56599

block56599:                                       ; preds = %edge9212, %block56600
  %v105715 = phi i64 [ %v105721, %block56600 ], [ %v104974, %edge9212 ]
  %v105717 = phi i64 [ %v105723, %block56600 ], [ %v104976, %edge9212 ]
  %v105720 = phi i64 [ %v105726, %block56600 ], [ %v104979, %edge9212 ]
  %t5909 = icmp eq i64 %v104979, %v105720
  %t5911.not183 = icmp eq i64 %v105720, 0
  %t5911.not = or i1 %t5909, %t5911.not183
  br i1 %t5911.not, label %block56605, label %edge9224

block56605:                                       ; preds = %block56599, %edge9224
  %t5915.not184 = icmp eq i64 %v104979, 0
  %t5915.not = or i1 %t5915.not184, %t5909
  br i1 %t5915.not, label %edge9233, label %edge9229

block56596:                                       ; preds = %edge9229, %block56601, %edge9205
  %v105703 = phi i64 [ %v105736, %block56601 ], [ 103425, %edge9205 ], [ %v105750, %edge9229 ]
  %t5917 = insertvalue { i64, i64 } poison, i64 %v105703, 0
  %t5918 = insertvalue { i64, i64 } %t5917, i64 4753, 1
  br label %common.ret

block56681:                                       ; preds = %edge9262, %edge9118
  br i1 %t5774.not, label %edge9329, label %edge9266

block56677:                                       ; preds = %edge9269, %edge9262
  %v105861 = phi i64 [ %v105871, %edge9262 ], [ %v105875, %edge9269 ]
  %t5930 = insertvalue { i64, i64 } poison, i64 %v105861, 0
  %t5931 = insertvalue { i64, i64 } %t5930, i64 4771, 1
  br label %common.ret

block56955:                                       ; preds = %edge9329
  %t5968 = insertvalue { i64, i64 } poison, i64 %v107241, 0
  %t5969 = insertvalue { i64, i64 } %t5968, i64 4788, 1
  br label %common.ret

block4730:                                        ; preds = %block4730.backedge, %edge9345
  %v107439 = phi i64 [ %v107246, %edge9345 ], [ %v108576, %block4730.backedge ]
  %v107463 = phi i64 [ %v107245, %edge9345 ], [ %v108573, %block4730.backedge ]
  %v107465 = phi i64 [ 2, %edge9345 ], [ %v108557, %block4730.backedge ]
  %v107470 = phi i64 [ %v107244, %edge9345 ], [ %v108571, %block4730.backedge ]
  %t5970.not = icmp eq i64 %v107439, 0
  br i1 %t5970.not, label %edge9375, label %edge9346

block57764:                                       ; preds = %edge9462, %edge9461
  %t6053 = zext i64 %v107470 to i128
  %v218668 = or disjoint i128 %t6053, 313594649253062377472
  store i128 %v218668, ptr %ss0, align 16
  %t6057 = zext i64 %v107463 to i128
  %v218670 = or disjoint i128 %t6057, 18446744073709551616
  store i128 %v218670, ptr %t1002, align 16
  %t6063 = zext i64 %v107439 to i128
  %t6064 = shl nuw i128 %t6063, 64
  %v218672 = or disjoint i128 %t6064, 17
  store i128 %v218672, ptr %t1007, align 16
  %v108592 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v107463, i64 4, i64 4, i64 %v3192)
  %t6067.not = icmp eq i64 %v108592, 0
  br i1 %t6067.not, label %edge9467, label %block57759

block57763:                                       ; preds = %edge9462, %edge9467
  %t6070.pre-phi = phi ptr [ %t6044, %edge9462 ], [ %.pre20708, %edge9467 ]
  %v108577 = phi i64 [ %v107470, %edge9462 ], [ %v108593, %edge9467 ]
  %v108579 = phi i64 [ %v107463, %edge9462 ], [ %v108594, %edge9467 ]
  %v108582 = phi i64 [ %v107439, %edge9462 ], [ %v108595, %edge9467 ]
  %v108597 = trunc i64 %v108541 to i32
  store i32 %v108597, ptr %t6070.pre-phi, align 1
  br label %block57762

block57762:                                       ; preds = %edge9442, %block57763
  %v108571 = phi i64 [ %v108577, %block57763 ], [ %v107470, %edge9442 ]
  %v108573 = phi i64 [ %v108579, %block57763 ], [ %v107463, %edge9442 ]
  %v108576 = phi i64 [ %v108582, %block57763 ], [ %v107439, %edge9442 ]
  %t6071 = icmp eq i64 %v107439, %v108576
  %t6073.not185 = icmp eq i64 %v108576, 0
  %t6073.not = or i1 %t6071, %t6073.not185
  br i1 %t6073.not, label %block57768, label %edge9470

block57768:                                       ; preds = %block57762, %edge9470
  %t6077.not = or i1 %t5970.not, %t6071
  br i1 %t6077.not, label %block4730.backedge, label %edge9475

block57759:                                       ; preds = %edge9475, %block57764
  %v108559 = phi i64 [ %v108592, %block57764 ], [ %v108606, %edge9475 ]
  %t6079 = insertvalue { i64, i64 } poison, i64 %v108559, 0
  %t6080 = insertvalue { i64, i64 } %t6079, i64 4868, 1
  br label %common.ret

block4807:                                        ; preds = %block4807.preheader, %edge9754
  %v108856 = phi i64 [ %v217534.ph, %edge9754 ], [ %v104178, %block4807.preheader ]
  %v108886 = phi i64 [ %v217537.ph, %edge9754 ], [ %v107439, %block4807.preheader ]
  %v108888 = phi i64 [ %v217533.ph, %edge9754 ], [ %v104219, %block4807.preheader ]
  %v108890 = phi i64 [ %v217536.ph, %edge9754 ], [ %v107463, %block4807.preheader ]
  %v108892 = phi i64 [ %v111560, %edge9754 ], [ 0, %block4807.preheader ]
  %v108893 = phi i64 [ %v217532.ph, %edge9754 ], [ %v104226, %block4807.preheader ]
  %v108896 = phi i64 [ %v217535.ph, %edge9754 ], [ %v107470, %block4807.preheader ]
  %t6085.not = icmp eq i64 %v108856, 0
  br i1 %t6085.not, label %edge9541, label %edge9504

block58819:                                       ; preds = %edge9622, %edge9621
  %t6182 = zext i64 %v108896 to i128
  %v218692 = or disjoint i128 %t6182, 313594649253062377472
  store i128 %v218692, ptr %ss0, align 16
  %t6186 = zext i64 %v108890 to i128
  %v218694 = or disjoint i128 %t6186, 18446744073709551616
  store i128 %v218694, ptr %t1002, align 16
  %t6192 = zext i64 %v108886 to i128
  %t6193 = shl nuw i128 %t6192, 64
  %v218696 = or disjoint i128 %t6193, 17
  store i128 %v218696, ptr %t1007, align 16
  %v110336 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v108890, i64 4, i64 4, i64 %v3192)
  %t6196.not = icmp eq i64 %v110336, 0
  br i1 %t6196.not, label %edge9627, label %block58814

block58817:                                       ; preds = %edge9627, %edge9622
  %t6199.pre-phi = phi ptr [ %t6165, %edge9622 ], [ %.pre20706, %edge9627 ]
  %v110321 = phi i64 [ %v108896, %edge9622 ], [ %v110337, %edge9627 ]
  %v110323 = phi i64 [ %v108890, %edge9622 ], [ %v110338, %edge9627 ]
  %v110326 = phi i64 [ %v108886, %edge9622 ], [ %v110339, %edge9627 ]
  store i32 %v110294, ptr %t6199.pre-phi, align 1
  %t6200 = icmp eq i64 %v108886, %v110326
  %t6202.not187 = icmp eq i64 %v110326, 0
  %t6202.not = or i1 %t6200, %t6202.not187
  br i1 %t6202.not, label %block58823, label %edge9630

block58823:                                       ; preds = %block58817, %edge9630
  %t6206.not188 = icmp eq i64 %v108886, 0
  %t6206.not = or i1 %t6206.not188, %t6200
  br i1 %t6206.not, label %edge9640, label %edge9635

block58814:                                       ; preds = %edge9597, %edge9635, %block58819
  %v110303 = phi i64 [ %v110336, %block58819 ], [ %v110350, %edge9635 ], [ 103425, %edge9597 ]
  %t6208 = insertvalue { i64, i64 } poison, i64 %v110303, 0
  %t6209 = insertvalue { i64, i64 } %t6208, i64 4952, 1
  br label %common.ret

block59340:                                       ; preds = %edge9712, %edge9711
  %t6237 = zext i64 %v108893 to i128
  %v218698 = or disjoint i128 %t4978, %t6237
  store i128 %v218698, ptr %ss0, align 16
  %t6241 = zext i64 %v108888 to i128
  %v218700 = or disjoint i128 %t6241, 18446744073709551616
  store i128 %v218700, ptr %t1002, align 16
  %t6247 = zext i64 %v108856 to i128
  %t6248 = shl nuw i128 %t6247, 64
  %v218702 = or disjoint i128 %t6248, %t4977
  store i128 %v218702, ptr %t1007, align 16
  %v111227 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v108888, i64 4, i64 4, i64 %v3192)
  %t6251.not = icmp eq i64 %v111227, 0
  br i1 %t6251.not, label %edge9717, label %block59335

block59339:                                       ; preds = %edge9712, %edge9717
  %t6254.pre-phi = phi ptr [ %t6156, %edge9712 ], [ %.pre20704, %edge9717 ]
  %v111212 = phi i64 [ %v108893, %edge9712 ], [ %v111228, %edge9717 ]
  %v111214 = phi i64 [ %v108888, %edge9712 ], [ %v111229, %edge9717 ]
  %v111217 = phi i64 [ %v108856, %edge9712 ], [ %v111230, %edge9717 ]
  %v111232 = trunc nuw nsw i64 %v111162 to i32
  store i32 %v111232, ptr %t6254.pre-phi, align 1
  br label %block59338

block59338:                                       ; preds = %edge9689, %block59339
  %v111206 = phi i64 [ %v111212, %block59339 ], [ %v108893, %edge9689 ]
  %v111208 = phi i64 [ %v111214, %block59339 ], [ %v108888, %edge9689 ]
  %v111211 = phi i64 [ %v111217, %block59339 ], [ %v108856, %edge9689 ]
  %t6255 = icmp eq i64 %v108856, %v111211
  %t6257.not189 = icmp eq i64 %v111211, 0
  %t6257.not = or i1 %t6255, %t6257.not189
  br i1 %t6257.not, label %block59344, label %edge9720

block59344:                                       ; preds = %block59338, %edge9720
  %t6261.not = or i1 %t6085.not, %t6255
  br i1 %t6261.not, label %edge9754, label %edge9723

block59335:                                       ; preds = %edge9726, %block59340
  %v111194 = phi i64 [ %v111227, %block59340 ], [ %v111241, %edge9726 ]
  %t6266 = insertvalue { i64, i64 } poison, i64 %v111194, 0
  %t6267 = insertvalue { i64, i64 } %t6266, i64 5013, 1
  br label %common.ret

block60053:                                       ; preds = %edge9795, %edge9799, %edge9808
  %v112382 = phi i64 [ %v112387, %edge9799 ], [ %v112404, %edge9808 ], [ 16, %edge9795 ]
  %t6315 = call i64 @llvm.umax.i64(i64 %v112382, i64 %v112376)
  br label %edge9812

block60045:                                       ; preds = %edge9812, %edge9799, %edge9789
  %v112226 = phi i64 [ %v112417, %edge9812 ], [ 1, %edge9789 ], [ 1, %edge9799 ]
  %t6337 = insertvalue { i64, i64 } poison, i64 %v112226, 0
  %t6338 = insertvalue { i64, i64 } %t6337, i64 5043, 1
  br label %common.ret

block4979:                                        ; preds = %block4979.backedge, %edge9845
  %v113215 = phi i64 [ 288, %edge9845 ], [ %v113510, %block4979.backedge ]
  %t6347 = icmp samesign ult i64 %v113215, 258
  br i1 %t6347, label %block60779, label %edge9857

block60779:                                       ; preds = %edge9876, %block4979
  %v113215.lcssa = phi i64 [ %v113215, %edge9876 ], [ 257, %block4979 ]
  br i1 %t6344.not1201, label %block5034.preheader, label %edge9892

block5034.preheader:                              ; preds = %block60779, %edge9911
  br label %block5034

block5034:                                        ; preds = %block5034.backedge, %block5034.preheader
  %v114415 = phi i64 [ 320, %block5034.preheader ], [ %v114709, %block5034.backedge ]
  %t6369 = icmp samesign ult i64 %v114415, 290
  br i1 %t6369, label %block61488, label %edge9928

block61488:                                       ; preds = %edge9947, %block5034
  %v114415.lcssa = phi i64 [ %v114415, %edge9947 ], [ 289, %block5034 ]
  br i1 %t6344.not1201, label %edge9982, label %edge9963

block61737:                                       ; preds = %edge10019
  %t6423 = insertvalue { i64, i64 } poison, i64 %v116333, 0
  %t6424 = insertvalue { i64, i64 } %t6423, i64 5175, 1
  br label %common.ret

block61792:                                       ; preds = %edge10059
  %t6452 = insertvalue { i64, i64 } poison, i64 %v117365, 0
  %t6453 = insertvalue { i64, i64 } %t6452, i64 5188, 1
  br label %common.ret

block5115:                                        ; preds = %edge10061, %edge10062
  %.pre20761 = inttoptr i64 %v1131208931196 to ptr
  br i1 %t6344.not1201, label %edge10077, label %edge10068

block5117:                                        ; preds = %block5117.backedge, %edge10077
  %v117617 = phi i64 [ %v116336, %edge10077 ], [ %v122908, %block5117.backedge ]
  %v117619 = phi i64 [ %v116337, %edge10077 ], [ %v122910, %block5117.backedge ]
  %v117622 = phi i64 [ %v116338, %edge10077 ], [ %v122913, %block5117.backedge ]
  %v117629 = phi i64 [ %v117368, %edge10077 ], [ %v123072, %block5117.backedge ]
  %v117631 = phi i64 [ %v117369, %edge10077 ], [ %v123074, %block5117.backedge ]
  %v117634 = phi i64 [ %v117370, %edge10077 ], [ %v123077, %block5117.backedge ]
  %v117641 = phi i64 [ 0, %edge10077 ], [ %v123046, %block5117.backedge ]
  %v117644 = phi i64 [ 0, %edge10077 ], [ %v123143, %block5117.backedge ]
  %v117647 = phi i64 [ 0, %edge10077 ], [ %v217542, %block5117.backedge ]
  %v117650 = phi i64 [ 0, %edge10077 ], [ %v217544, %block5117.backedge ]
  %t6457.not = icmp eq i64 %v117644, %v115406
  br i1 %t6457.not, label %edge10404, label %edge10103

block5186:                                        ; preds = %block5186.preheader, %edge10229
  %v119300.in = phi i64 [ %v119300, %edge10229 ], [ %v117644, %block5186.preheader ]
  %v119300 = add nuw i64 %v119300.in, 1
  %t6479 = icmp ne i64 %v119300, %v115406
  %t6480 = icmp ult i64 %v119300, %v113215.lcssa
  %v119626 = select i1 %t6480, i64 0, i64 %v118544
  %v119631 = add i64 %v119626, %v119300
  %t6483 = icmp ult i64 %v119631, %v1131168871199
  br i1 %t6483, label %edge10201, label %edge10210

block5249:                                        ; preds = %edge10218, %edge10219, %edge10222, %edge10103
  %v217542 = phi i64 [ %v117647, %edge10103 ], [ %v119300, %edge10222 ], [ %v119300, %edge10219 ], [ %v119300, %edge10218 ]
  %v217544 = phi i64 [ %v117650, %edge10103 ], [ %v118907, %edge10222 ], [ %v118907, %edge10219 ], [ %v118907, %edge10218 ]
  %t6503.not = icmp ult i64 %v217542, %v117644
  br i1 %t6503.not, label %common.ret, label %edge10242

block65162:                                       ; preds = %edge10340, %edge10339
  %t6546 = zext i64 %v117617 to i128
  %v218710 = or disjoint i128 %t6546, 350488137400481480704
  store i128 %v218710, ptr %ss0, align 16
  %t6550 = zext i64 %v117619 to i128
  %v218712 = or disjoint i128 %t6550, 18446744073709551616
  store i128 %v218712, ptr %t1002, align 16
  %t6556 = zext i64 %v117622 to i128
  %t6557 = shl nuw i128 %t6556, 64
  %v218714 = or disjoint i128 %t6557, 19
  store i128 %v218714, ptr %t1007, align 16
  %v122923 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v117619, i64 4, i64 4, i64 %v3192)
  %t6560.not = icmp eq i64 %v122923, 0
  br i1 %t6560.not, label %edge10345, label %block65157

block65160:                                       ; preds = %edge10345, %edge10340
  %t6563.pre-phi = phi ptr [ %t6529, %edge10340 ], [ %.pre20690, %edge10345 ]
  %v122908 = phi i64 [ %v117617, %edge10340 ], [ %v122924, %edge10345 ]
  %v122910 = phi i64 [ %v117619, %edge10340 ], [ %v122925, %edge10345 ]
  %v122913 = phi i64 [ %v117622, %edge10340 ], [ %v122926, %edge10345 ]
  store i32 %v122928, ptr %t6563.pre-phi, align 1
  %t6564 = icmp eq i64 %v117622, %v122913
  %t6566.not201 = icmp eq i64 %v122913, 0
  %t6566.not = or i1 %t6564, %t6566.not201
  br i1 %t6566.not, label %block65166, label %edge10348

block65166:                                       ; preds = %block65160, %edge10348
  %v122933 = select i1 %t6564, i64 0, i64 %v117622
  %v123023 = shl nuw nsw i64 %v122496926, 5
  %v123033 = or disjoint i64 %v122181913923, %v123023
  %v123046 = add nuw nsw i64 %v117641, 1
  %exitcond20594.not = icmp eq i64 %v117641, 320
  br i1 %exitcond20594.not, label %block65177, label %edge10362

block65157:                                       ; preds = %edge10301, %block65162
  %v122890 = phi i64 [ %v122923, %block65162 ], [ 103425, %edge10301 ]
  %t6570 = insertvalue { i64, i64 } poison, i64 %v122890, 0
  %t6571 = insertvalue { i64, i64 } %t6570, i64 5418, 1
  br label %common.ret

block65183:                                       ; preds = %edge10368, %edge10365
  %t6581.not = icmp eq i64 %v117634, 0
  br i1 %t6581.not, label %block65186, label %edge10372

block65186:                                       ; preds = %edge10372, %block65183
  %t6586 = zext i64 %v117629 to i128
  %v218716 = or disjoint i128 %t6586, 5902958103587056517120
  store i128 %v218716, ptr %ss0, align 16
  %t6590 = zext i64 %v117631 to i128
  %v218718 = or disjoint i128 %t6590, 18446744073709551616
  store i128 %v218718, ptr %t1002, align 16
  %t6596 = zext i64 %v117634 to i128
  %t6597 = shl nuw i128 %t6596, 64
  %v218720 = or disjoint i128 %t6597, 320
  store i128 %v218720, ptr %t1007, align 16
  %v123097 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v117631, i64 4, i64 4, i64 %v3192)
  %t6600.not = icmp eq i64 %v123097, 0
  br i1 %t6600.not, label %edge10377, label %block65177

block65185:                                       ; preds = %edge10372, %edge10377
  %t6603.pre-phi = phi ptr [ %t6575, %edge10372 ], [ %.pre20688, %edge10377 ]
  %v123082 = phi i64 [ %v117629, %edge10372 ], [ %v123098, %edge10377 ]
  %v123084 = phi i64 [ %v117631, %edge10372 ], [ %v123099, %edge10377 ]
  %v123087 = phi i64 [ %v117634, %edge10372 ], [ %v123100, %edge10377 ]
  %v123102 = trunc nuw nsw i64 %v123033 to i32
  store i32 %v123102, ptr %t6603.pre-phi, align 1
  br label %block65180

block65180:                                       ; preds = %edge10362, %block65185
  %v123072 = phi i64 [ %v123082, %block65185 ], [ %v117629, %edge10362 ]
  %v123074 = phi i64 [ %v123084, %block65185 ], [ %v117631, %edge10362 ]
  %v123077 = phi i64 [ %v123087, %block65185 ], [ %v117634, %edge10362 ]
  %v123261 = phi i64 [ 0, %block65185 ], [ %v122933, %edge10362 ]
  %t6604 = icmp eq i64 %v117634, %v123077
  %t6606.not202 = icmp eq i64 %v123077, 0
  %t6606.not = or i1 %t6604, %t6606.not202
  br i1 %t6606.not, label %edge10394, label %edge10380

block65177:                                       ; preds = %block65186, %edge10368, %block65166
  %v123048 = phi i64 [ %v123081, %edge10368 ], [ 103425, %block65166 ], [ %v123097, %block65186 ]
  %t6610 = insertvalue { i64, i64 } poison, i64 %v123048, 0
  %t6611 = insertvalue { i64, i64 } %t6610, i64 5427, 1
  br label %common.ret

block65237:                                       ; preds = %edge10404, %edge10405
  %t6623.not = icmp eq i64 %v117634, 0
  br i1 %t6623.not, label %edge10424, label %edge10408

block65268:                                       ; preds = %edge10427, %edge10424
  br i1 %t6620.not, label %block65272, label %edge10433

block65272:                                       ; preds = %edge10433, %block65268
  %t6632.not = icmp eq i64 %v117634, 0
  br i1 %t6632.not, label %edge9837, label %edge10437

block65280:                                       ; preds = %edge10447, %edge10423
  %t6639.not = icmp eq i64 %v123107, 0
  br i1 %t6639.not, label %block5117.backedge, label %edge10451

block5117.backedge:                               ; preds = %block65280, %edge10451, %edge10454
  br label %block5117

block65263:                                       ; preds = %edge10440, %edge10433, %edge10427, %edge10454, %edge10447
  %v123228 = phi i64 [ %v123251, %edge10447 ], [ %v123255, %edge10454 ], [ %v123243, %edge10433 ], [ %v123239, %edge10427 ], [ %v123247, %edge10440 ]
  %t6644 = insertvalue { i64, i64 } poison, i64 %v123228, 0
  %t6645 = insertvalue { i64, i64 } %t6644, i64 5446, 1
  br label %common.ret

block65595:                                       ; preds = %edge10494, %edge10495
  %t6649.not = icmp eq i64 %v217496, 0
  br i1 %t6649.not, label %edge9837, label %edge10498

block65834:                                       ; preds = %edge10524, %edge10521
  br i1 %t6657.not, label %block65838, label %edge10530

block65838:                                       ; preds = %edge10530, %block65834
  %t6667.not = icmp eq i64 %v124335, 0
  br i1 %t6667.not, label %block65842, label %edge10536

block65842:                                       ; preds = %edge10536, %block65838
  br i1 %t6657.not, label %edge10604, label %edge10540

block65850:                                       ; preds = %edge10550, %edge10519
  %t6676.not = icmp eq i64 %v111931, 0
  br i1 %t6676.not, label %block65854, label %edge10556

block65854:                                       ; preds = %edge10556, %block65850
  %t6678.not = icmp eq i64 %v217494, 0
  br i1 %t6678.not, label %block65858, label %edge10562

block65858:                                       ; preds = %edge10562, %block65854
  %t6680.not = icmp eq i64 %v217496, 0
  br i1 %t6680.not, label %edge10575, label %edge10568

block65825:                                       ; preds = %edge10543, %edge10536, %edge10530, %edge10524, %edge10568, %edge10562, %edge10556, %edge10550
  %v124277 = phi i64 [ %v124312, %edge10562 ], [ %v124316, %edge10568 ], [ %v124308, %edge10556 ], [ %v124304, %edge10550 ], [ %v124296, %edge10536 ], [ %v124292, %edge10530 ], [ %v124288, %edge10524 ], [ %v124300, %edge10543 ]
  %t6682 = insertvalue { i64, i64 } poison, i64 %v124277, 0
  %t6683 = insertvalue { i64, i64 } %t6682, i64 5476, 1
  br label %common.ret

block5407:                                        ; preds = %block5407.backedge, %block5407.preheader
  %v127540 = phi i64 [ 19, %block5407.preheader ], [ %v127873, %block5407.backedge ]
  %t6688 = icmp samesign ult i64 %v127540, 5
  br i1 %t6688, label %block67063, label %edge10666

block67063:                                       ; preds = %edge10674, %block5407
  %v127540.lcssa = phi i64 [ %v127540, %edge10674 ], [ 4, %block5407 ]
  br i1 %t6685.not, label %edge10759, label %edge10690

block5537:                                        ; preds = %block5537.preheader, %edge11074
  %v131024 = phi i64 [ %v133274, %edge11074 ], [ 0, %block5537.preheader ]
  %v131027 = phi i64 [ %v132993, %edge11074 ], [ %v130653, %block5537.preheader ]
  %exitcond20595.not = icmp eq i64 %v131024, 19
  br i1 %exitcond20595.not, label %edge11081, label %edge10999

block69505:                                       ; preds = %edge11084, %edge11081
  br i1 %t6796.not, label %block5650.preheader, label %edge11088

block69501:                                       ; preds = %edge11091, %edge11084
  %v132997 = phi i64 [ %v133004, %edge11084 ], [ %v133008, %edge11091 ]
  %t6841 = insertvalue { i64, i64 } poison, i64 %v132997, 0
  %t6842 = insertvalue { i64, i64 } %t6841, i64 5729, 1
  br label %common.ret

block5650:                                        ; preds = %block5650.preheader, %edge11197
  %v134182 = phi i64 [ %v136359, %edge11197 ], [ 0, %block5650.preheader ]
  %v134185 = phi i64 [ %v136088, %edge11197 ], [ %v131027, %block5650.preheader ]
  %exitcond20596.not = icmp eq i64 %v134182, 320
  br i1 %exitcond20596.not, label %edge11204, label %edge11139

block71150:                                       ; preds = %edge11207, %edge11204
  br i1 %t6775.not, label %edge10780, label %edge11211

block71146:                                       ; preds = %edge11214, %edge11207
  %v136092 = phi i64 [ %v136099, %edge11207 ], [ %v136103, %edge11214 ]
  %t6883 = insertvalue { i64, i64 } poison, i64 %v136092, 0
  %t6884 = insertvalue { i64, i64 } %t6883, i64 5846, 1
  br label %common.ret

block5798:                                        ; preds = %block5798.preheader, %edge11624
  %v138987 = phi i64 [ %v141309, %edge11624 ], [ %v66492, %block5798.preheader ]
  %v139015 = phi i64 [ %v141306, %edge11624 ], [ %v67291, %block5798.preheader ]
  %v139016 = phi i64 [ %v141634, %edge11624 ], [ 3, %block5798.preheader ]
  %v139017 = phi i64 [ %v141304, %edge11624 ], [ %v67290, %block5798.preheader ]
  %t6965.not = icmp eq i64 %v138987, 0
  br i1 %t6965.not, label %edge11491, label %edge11460

block73563:                                       ; preds = %edge11579, %edge11578
  %t7063 = zext i64 %v139017 to i128
  %v218736 = or disjoint i128 %t7063, 10090369008319124733952
  store i128 %v218736, ptr %ss0, align 16
  %t7067 = zext i64 %v139015 to i128
  %v218738 = or disjoint i128 %t7067, 18446744073709551616
  store i128 %v218738, ptr %t1002, align 16
  %t7073 = zext i64 %v138987 to i128
  %t7074 = shl nuw i128 %t7073, 64
  %v218740 = or disjoint i128 %t7074, 547
  store i128 %v218740, ptr %t1007, align 16
  %v141325 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v139015, i64 4, i64 4, i64 %v3192)
  %t7077.not = icmp eq i64 %v141325, 0
  br i1 %t7077.not, label %edge11584, label %block73558

block73562:                                       ; preds = %edge11579, %edge11584
  %t7080.pre-phi = phi ptr [ %t7054, %edge11579 ], [ %.pre20682, %edge11584 ]
  %v141310 = phi i64 [ %v139017, %edge11579 ], [ %v141326, %edge11584 ]
  %v141312 = phi i64 [ %v139015, %edge11579 ], [ %v141327, %edge11584 ]
  %v141315 = phi i64 [ %v138987, %edge11579 ], [ %v141328, %edge11584 ]
  %v141330 = trunc nuw nsw i64 %v141268 to i32
  store i32 %v141330, ptr %t7080.pre-phi, align 1
  br label %block73561

block73561:                                       ; preds = %82, %block73562
  %v141304 = phi i64 [ %v141310, %block73562 ], [ %v139017, %82 ]
  %v141306 = phi i64 [ %v141312, %block73562 ], [ %v139015, %82 ]
  %v141309 = phi i64 [ %v141315, %block73562 ], [ %v138987, %82 ]
  %t7081 = icmp eq i64 %v138987, %v141309
  %t7083.not204 = icmp eq i64 %v141309, 0
  %t7083.not = or i1 %t7081, %t7083.not204
  br i1 %t7083.not, label %edge11598, label %edge11587

block73558:                                       ; preds = %block73563
  %t7087 = insertvalue { i64, i64 } poison, i64 %v141325, 0
  %t7088 = insertvalue { i64, i64 } %t7087, i64 5976, 1
  br label %common.ret

block73578:                                       ; preds = %edge11610, %edge11602, %edge11617
  %v141356 = phi i64 [ %v141375, %edge11617 ], [ %v141363, %edge11602 ], [ %v141371, %edge11610 ]
  %t7103 = insertvalue { i64, i64 } poison, i64 %v141356, 0
  %t7104 = insertvalue { i64, i64 } %t7103, i64 5981, 1
  br label %common.ret

block5917:                                        ; preds = %block5917.preheader, %edge11729
  %v142514 = phi i64 [ %v143720, %edge11729 ], [ 0, %block5917.preheader ]
  %exitcond20597.not = icmp eq i64 %v142514, 547
  br i1 %exitcond20597.not, label %edge11696, label %edge11729

block74297:                                       ; preds = %edge11699, %edge11696
  br i1 %t7125.not, label %block5990, label %edge11703

block74293:                                       ; preds = %edge11706, %edge11699
  %v142529 = phi i64 [ %v142537, %edge11699 ], [ %v142541, %edge11706 ]
  %t7138 = insertvalue { i64, i64 } poison, i64 %v142529, 0
  %t7139 = insertvalue { i64, i64 } %t7138, i64 6027, 1
  br label %common.ret

block74960:                                       ; preds = %edge11767, %edge11764
  br i1 %t7125.not, label %block5990, label %edge11771

block74956:                                       ; preds = %edge11774, %edge11767
  %v143457 = phi i64 [ %v143464, %edge11767 ], [ %v143468, %edge11774 ]
  %t7160 = insertvalue { i64, i64 } poison, i64 %v143457, 0
  %t7161 = insertvalue { i64, i64 } %t7160, i64 6072, 1
  br label %common.ret

block5990:                                        ; preds = %edge11269, %edge11665, %edge11662, %edge11661, %edge11774, %edge11771, %block74960, %block74297, %edge11703, %edge11706
  %v217558 = phi i64 [ %v142017, %edge11665 ], [ 1, %edge11774 ], [ 1, %edge11706 ], [ 1, %edge11703 ], [ 1, %block74297 ], [ 1, %block74960 ], [ 1, %edge11771 ], [ %v142017, %edge11661 ], [ %v142017, %edge11662 ], [ 0, %edge11269 ]
  %v217559 = phi i64 [ 1, %edge11665 ], [ 1, %edge11774 ], [ 0, %edge11706 ], [ 0, %edge11703 ], [ 0, %block74297 ], [ 1, %block74960 ], [ 1, %edge11771 ], [ 1, %edge11661 ], [ 1, %edge11662 ], [ 1, %edge11269 ]
  %v217572 = phi i64 [ %v138987, %edge11665 ], [ %v138987, %edge11774 ], [ %v138987, %edge11706 ], [ %v138987, %edge11703 ], [ 0, %block74297 ], [ 0, %block74960 ], [ %v138987, %edge11771 ], [ %v138987, %edge11661 ], [ %v138987, %edge11662 ], [ %v66492, %edge11269 ]
  %v217579 = phi i64 [ %v139017, %edge11665 ], [ %v139017, %edge11774 ], [ %v139017, %edge11706 ], [ %v139017, %edge11703 ], [ %v139017, %block74297 ], [ %v139017, %block74960 ], [ %v139017, %edge11771 ], [ %v139017, %edge11661 ], [ %v139017, %edge11662 ], [ %v67290, %edge11269 ]
  %v217581 = phi i64 [ %v139015, %edge11665 ], [ %v139015, %edge11774 ], [ %v139015, %edge11706 ], [ %v139015, %edge11703 ], [ %v139015, %block74297 ], [ %v139015, %block74960 ], [ %v139015, %edge11771 ], [ %v139015, %edge11661 ], [ %v139015, %edge11662 ], [ %v67291, %edge11269 ]
  %v217609 = phi i64 [ %v142009, %edge11665 ], [ %v142009, %edge11774 ], [ %v142009, %edge11706 ], [ %v142009, %edge11703 ], [ %v142009, %block74297 ], [ %v142009, %block74960 ], [ %v142009, %edge11771 ], [ %v142009, %edge11661 ], [ %v142009, %edge11662 ], [ %v64785, %edge11269 ]
  %v143798 = and i64 %v217559, %v217558
  %v143817 = xor i64 %v143798, -1
  %v143822 = and i64 %v136513, %v143817
  %t7163 = icmp ult i64 %v64845, %v64848
  %v143891 = zext i1 %t7163 to i64
  %spec.select2 = call i64 @llvm.umin.i64(i64 %v64845, i64 %v64848)
  %t7166 = icmp ult i64 %spec.select2, %spec.select36992
  %v144172 = select i1 %t7166, i64 %v143822, i64 0
  %v144183 = xor i64 %v143891, 1
  %v144188 = and i64 %v144172, %v144183
  %t7168.not = icmp eq i64 %v144172, %v143822
  br i1 %t7168.not, label %edge11828.thread, label %edge11830

edge11828.thread:                                 ; preds = %block5990
  %v1445531007 = and i64 %v143822, %v143891
  br label %edge11848

block75817:                                       ; preds = %edge11873, %edge11866
  %v145025 = phi i64 [ %v145032, %edge11866 ], [ %v145036, %edge11873 ]
  %t7190 = insertvalue { i64, i64 } poison, i64 %v145025, 0
  %t7191 = insertvalue { i64, i64 } %t7190, i64 6146, 1
  br label %common.ret

block6052:                                        ; preds = %block6052.preheader, %edge12103
  %v145334 = phi i64 [ %v146544, %edge12103 ], [ 0, %block6052.preheader ]
  %v145337 = phi i64 [ %v217617.ph, %edge12103 ], [ 0, %block6052.preheader ]
  %exitcond20598.not = icmp eq i64 %v145334, 256
  br i1 %exitcond20598.not, label %edge12092, label %edge12063

block6219:                                        ; preds = %edge12113, %edge12143
  %v217618 = phi i64 [ %v148428, %edge12143 ], [ 3, %edge12113 ]
  br i1 %t7298.not, label %block6221.preheader, label %edge12165

block6221.preheader:                              ; preds = %block6219, %edge12165
  br label %block6221

block6221:                                        ; preds = %block6221.preheader, %edge12229
  %v151352 = phi i64 [ %v151973, %edge12229 ], [ %v217618, %block6221.preheader ]
  %v151355 = phi i64 [ %v152017, %edge12229 ], [ 0, %block6221.preheader ]
  %v151358 = phi i64 [ %v152340, %edge12229 ], [ %v54999, %block6221.preheader ]
  %exitcond20599.not = icmp eq i64 %v151352, 259
  br i1 %exitcond20599.not, label %edge12184, label %edge12183

block81635:                                       ; preds = %edge12365, %edge12366
  br i1 %t7381.not, label %block6392.preheader, label %edge12369

block6392.preheader:                              ; preds = %edge12369, %block81635
  br label %block6392

block6392:                                        ; preds = %block6392.preheader, %edge12454
  %v157221 = phi i64 [ %v158986, %edge12454 ], [ 0, %block6392.preheader ]
  %v157224 = phi i64 [ %v158994, %edge12454 ], [ 0, %block6392.preheader ]
  %exitcond20600.not = icmp eq i64 %v157221, 10
  br i1 %exitcond20600.not, label %edge12437, label %edge12392

block82729:                                       ; preds = %edge12440, %edge12437
  br i1 %t7381.not, label %edge12469, label %edge12444

block82725:                                       ; preds = %edge12447, %edge12440
  %v158723 = phi i64 [ %v158730, %edge12440 ], [ %v158734, %edge12447 ]
  %t7428 = insertvalue { i64, i64 } poison, i64 %v158723, 0
  %t7429 = insertvalue { i64, i64 } %t7428, i64 6570, 1
  br label %common.ret

block6524:                                        ; preds = %edge12705, %edge12534
  %v160741 = phi i64 [ %v64763, %edge12534 ], [ %v162931, %edge12705 ]
  %v160765 = phi i64 [ %v64760, %edge12534 ], [ %v162928, %edge12705 ]
  %v160769 = phi i64 [ 3, %edge12534 ], [ %v163256, %edge12705 ]
  %v160770 = phi i64 [ %v64758, %edge12534 ], [ %v162926, %edge12705 ]
  %v162963 = phi i64 [ %v217572, %edge12534 ], [ %v163286, %edge12705 ]
  %t7495.not = icmp eq i64 %v160741, 0
  br i1 %t7495.not, label %edge12560, label %edge12535

block85085:                                       ; preds = %edge12662, %edge12659
  br i1 %t7495.not, label %block85088, label %edge12666

block85088:                                       ; preds = %edge12666, %block85085
  %t7615 = zext i64 %v160770 to i128
  %v218756 = or disjoint i128 %t7615, 10090369008319124733952
  store i128 %v218756, ptr %ss0, align 16
  %t7619 = zext i64 %v160765 to i128
  %v218758 = or disjoint i128 %t7619, 18446744073709551616
  store i128 %v218758, ptr %t1002, align 16
  %t7625 = zext i64 %v160741 to i128
  %t7626 = shl nuw i128 %t7625, 64
  %v218760 = or disjoint i128 %t7626, 547
  store i128 %v218760, ptr %t1007, align 16
  %v162951 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v160765, i64 4, i64 4, i64 %v3192)
  %t7629.not = icmp eq i64 %v162951, 0
  br i1 %t7629.not, label %edge12671, label %block85079

block85087:                                       ; preds = %edge12666, %edge12671
  %t7632.pre-phi = phi ptr [ %t7560, %edge12666 ], [ %.pre20686, %edge12671 ]
  %v162936 = phi i64 [ %v160770, %edge12666 ], [ %v162952, %edge12671 ]
  %v162938 = phi i64 [ %v160765, %edge12666 ], [ %v162953, %edge12671 ]
  %v162941 = phi i64 [ %v160741, %edge12666 ], [ %v162954, %edge12671 ]
  %v162956 = trunc nuw i64 %v162631213 to i32
  store i32 %v162956, ptr %t7632.pre-phi, align 1
  br label %block85082

block85082:                                       ; preds = %edge12614, %block85087
  %v162926 = phi i64 [ %v162936, %block85087 ], [ %v160770, %edge12614 ]
  %v162928 = phi i64 [ %v162938, %block85087 ], [ %v160765, %edge12614 ]
  %v162931 = phi i64 [ %v162941, %block85087 ], [ %v160741, %edge12614 ]
  %v163286 = phi i64 [ 0, %block85087 ], [ %v162963, %edge12614 ]
  %t7633 = icmp eq i64 %v160741, %v162931
  %t7635.not214 = icmp eq i64 %v162931, 0
  %t7635.not = or i1 %t7633, %t7635.not214
  br i1 %t7635.not, label %edge12685, label %edge12674

block85079:                                       ; preds = %block85088, %edge12662
  %v162914 = phi i64 [ %v162935, %edge12662 ], [ %v162951, %block85088 ]
  %t7639 = insertvalue { i64, i64 } poison, i64 %v162914, 0
  %t7640 = insertvalue { i64, i64 } %t7639, i64 6719, 1
  br label %common.ret

block85103:                                       ; preds = %edge12691, %edge12698
  %v162987 = phi i64 [ %v163002, %edge12698 ], [ %v162998, %edge12691 ]
  %t7653 = insertvalue { i64, i64 } poison, i64 %v162987, 0
  %t7654 = insertvalue { i64, i64 } %t7653, i64 6724, 1
  br label %common.ret

block6652:                                        ; preds = %edge12739, %edge12730, %edge11863, %edge11866, %edge12722
  %v217631 = phi i64 [ %v162963, %edge12722 ], [ %v217572, %edge11863 ], [ %v217572, %edge11866 ], [ %v162963, %edge12730 ], [ %v162963, %edge12739 ]
  %v217632 = phi i64 [ %v16074114716, %edge12722 ], [ %v145038, %edge11863 ], [ %v145038, %edge11866 ], [ %v16074114716, %edge12730 ], [ %v16074114716, %edge12739 ]
  %v217651 = phi i64 [ %v160770, %edge12722 ], [ %v163331, %edge11863 ], [ %v163331, %edge11866 ], [ %v160770, %edge12730 ], [ %v160770, %edge12739 ]
  %v217653 = phi i64 [ %v160765, %edge12722 ], [ %v163333, %edge11863 ], [ %v163333, %edge11866 ], [ %v160765, %edge12730 ], [ %v160765, %edge12739 ]
  %v217663 = phi i64 [ %v64848, %edge12722 ], [ %v64848, %edge11863 ], [ %v64848, %edge11866 ], [ 7, %edge12730 ], [ %97, %edge12739 ]
  br i1 %v144951208, label %block6676, label %edge12755

block6676:                                        ; preds = %edge12760, %edge12755, %block6652
  %v217676 = phi i64 [ 2, %block6652 ], [ %spec.select40, %edge12760 ], [ 3, %edge12755 ]
  %spec.select3 = select i1 %t3926, i64 %v136868, i64 %v64845
  %v165399 = or i64 %v143798, %v145023
  %v165404 = or i64 %v145023, %v144951
  %v165409 = or i64 %v165404, %v143798
  %v165414 = or i64 %v165409, %v144188
  %v165424 = or i64 %v165414, %v1445531011
  %v165429 = or i64 %v165424, %v1445581009
  %v165434 = or i64 %v1445581009, %v1445531011
  %t7679 = icmp eq i64 %v165429, 1
  br i1 %t7679, label %edge12786, label %edge12787

block86572:                                       ; preds = %edge12790, %edge12787
  br i1 %t7381.not, label %block86576, label %edge12796

block86576:                                       ; preds = %edge12796, %block86572
  br i1 %t3704.not, label %edge12810, label %edge12800

block86600:                                       ; preds = %edge12813, %edge12810
  br i1 %t3900.not138212892129821344, label %edge12828, label %edge12817

block86628:                                       ; preds = %edge12831, %edge12828
  %t7699.not = icmp eq i64 %v217632, 0
  br i1 %t7699.not, label %edge13024, label %edge12837

block86567:                                       ; preds = %edge12837, %edge12831, %edge12820, %edge12813, %edge12803, %edge12796, %edge12790, %edge12845
  %v165436 = phi i64 [ %v165507, %edge12845 ], [ %v165499, %edge12831 ], [ %v165475, %edge12820 ], [ %v165471, %edge12813 ], [ %v165451, %edge12803 ], [ %v165447, %edge12796 ], [ %v165443, %edge12790 ], [ %v165503, %edge12837 ]
  %t7706 = insertvalue { i64, i64 } poison, i64 %v165436, 0
  %t7707 = insertvalue { i64, i64 } %t7706, i64 6802, 1
  br label %common.ret

block88847:                                       ; preds = %edge13297, %edge13293
  br i1 %t7746.not, label %block88851, label %edge13303

block88851:                                       ; preds = %edge13303, %block88847
  %t7853.not = icmp eq i64 %v129506, 0
  br i1 %t7853.not, label %block88855, label %edge13309

block88855:                                       ; preds = %edge13309, %block88851
  %t7855.not = icmp eq i64 %v129512, 0
  br i1 %t7855.not, label %edge13322, label %edge13315

block88843:                                       ; preds = %edge13315, %edge13309, %edge13303, %edge13297
  %v174029 = phi i64 [ %v174044, %edge13309 ], [ %v174036, %edge13297 ], [ %v174040, %edge13303 ], [ %v174048, %edge13315 ]
  %t7857 = insertvalue { i64, i64 } poison, i64 %v174029, 0
  %t7858 = insertvalue { i64, i64 } %t7857, i64 6986, 1
  br label %common.ret

block6878:                                        ; preds = %block6878.preheader, %edge13606
  %v175428 = phi i64 [ %v176610, %edge13606 ], [ %v217705, %block6878.preheader ]
  %v175429 = phi i64 [ %v176580, %edge13606 ], [ %v217706, %block6878.preheader ]
  %v175430 = phi i64 [ %v176612, %edge13606 ], [ %v217707, %block6878.preheader ]
  %v175432 = phi i64 [ %v176580, %edge13606 ], [ %v217709, %block6878.preheader ]
  %v175433 = phi i64 [ %v176615, %edge13606 ], [ %v217710, %block6878.preheader ]
  %v175440 = phi i64 [ 0, %edge13606 ], [ %v217711, %block6878.preheader ]
  %v175443 = phi i64 [ 0, %edge13606 ], [ %v217712, %block6878.preheader ]
  %v175458 = phi i64 [ %v176648, %edge13606 ], [ %v54957, %block6878.preheader ]
  %t7862.not = icmp ult i64 %v54958, %v175458
  br i1 %t7862.not, label %common.ret, label %edge13347

block89292:                                       ; preds = %edge13379, %edge13383, %edge13392
  %v175873 = phi i64 [ %v175878, %edge13383 ], [ %v175895, %edge13392 ], [ 64, %edge13379 ]
  %t7901 = call i64 @llvm.umax.i64(i64 %v175873, i64 %v175867)
  br label %block89289

block89289:                                       ; preds = %edge13376, %block89292
  %v175872 = phi i64 [ %t7901, %block89292 ], [ %v175430, %edge13376 ]
  %t7902.not = icmp eq i64 %v175433, 0
  br i1 %t7902.not, label %block89300, label %edge13396

block89300:                                       ; preds = %edge13396, %block89289
  %t7907 = zext i64 %v175428 to i128
  %t7908 = zext i64 %v175429 to i128
  %t7909 = shl nuw i128 %t7908, 64
  %v218762 = or disjoint i128 %t7909, %t7907
  store i128 %v218762, ptr %ss0, align 16
  %t7911 = zext i64 %v175430 to i128
  %v218764 = or disjoint i128 %t7911, 18446744073709551616
  store i128 %v218764, ptr %t1002, align 16
  %t7916 = zext i64 %v175432 to i128
  %t7917 = zext i64 %v175433 to i128
  %t7918 = shl nuw i128 %t7917, 64
  %v218766 = or disjoint i128 %t7918, %t7916
  store i128 %v218766, ptr %t1007, align 16
  %v175912 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v175872, i64 1, i64 1, i64 %v3192)
  %t7921.not = icmp eq i64 %v175912, 0
  br i1 %t7921.not, label %edge13401, label %block89285

block89299:                                       ; preds = %edge13396, %edge13401
  %v175897 = phi i64 [ %v175913, %edge13401 ], [ %v175428, %edge13396 ]
  %v175899 = phi i64 [ %v175914, %edge13401 ], [ %v175430, %edge13396 ]
  %v175902 = phi i64 [ %v175915, %edge13401 ], [ %v175433, %edge13396 ]
  %v175917 = add i64 %v175897, %v175429
  %v175918 = trunc i64 %v175848 to i8
  %t7924 = inttoptr i64 %v175917 to ptr
  store i8 %v175918, ptr %t7924, align 1
  %t7925 = icmp eq i64 %v175433, %v175902
  %t7927.not241 = icmp eq i64 %v175902, 0
  %t7927.not = or i1 %t7925, %t7927.not241
  br i1 %t7927.not, label %block89304, label %edge13403

block89304:                                       ; preds = %block89299, %edge13403
  %t7931.not = or i1 %t7902.not, %t7925
  br i1 %t7931.not, label %edge13412, label %edge13408

block89285:                                       ; preds = %edge13408, %block89300, %edge13383, %edge13347
  %v175861 = phi i64 [ %v175912, %block89300 ], [ %v175928, %edge13408 ], [ 1, %edge13347 ], [ 1, %edge13383 ]
  %t7933 = insertvalue { i64, i64 } poison, i64 %v175861, 0
  %t7934 = insertvalue { i64, i64 } %t7933, i64 7029, 1
  br label %common.ret

block89434:                                       ; preds = %edge13423, %edge13427, %edge13436
  %v176106 = phi i64 [ %v176111, %edge13427 ], [ %v176128, %edge13436 ], [ 64, %edge13423 ]
  %t7964 = call i64 @llvm.umax.i64(i64 %v176106, i64 %v176100)
  br label %block89431

block89431:                                       ; preds = %edge13420, %block89434
  %v176105 = phi i64 [ %t7964, %block89434 ], [ %v175899, %edge13420 ]
  br i1 %t7927.not241, label %block89442, label %edge13440

block89442:                                       ; preds = %edge13440, %block89431
  %t7970 = zext i64 %v175897 to i128
  %t7971 = zext i64 %v175867 to i128
  %t7972 = shl nuw i128 %t7971, 64
  %v218768 = or disjoint i128 %t7972, %t7970
  store i128 %v218768, ptr %ss0, align 16
  %t7974 = zext i64 %v175899 to i128
  %v218770 = or disjoint i128 %t7974, 18446744073709551616
  store i128 %v218770, ptr %t1002, align 16
  %t7980 = zext i64 %v175902 to i128
  %t7981 = shl nuw i128 %t7980, 64
  %v218772 = or disjoint i128 %t7981, %t7971
  store i128 %v218772, ptr %t1007, align 16
  %v176145 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v176105, i64 1, i64 1, i64 %v3192)
  %t7984.not = icmp eq i64 %v176145, 0
  br i1 %t7984.not, label %edge13445, label %block89427

block89441:                                       ; preds = %edge13440, %edge13445
  %v176130 = phi i64 [ %v176146, %edge13445 ], [ %v175897, %edge13440 ]
  %v176132 = phi i64 [ %v176147, %edge13445 ], [ %v175899, %edge13440 ]
  %v176135 = phi i64 [ %v176148, %edge13445 ], [ %v175902, %edge13440 ]
  %v176150 = add i64 %v176130, %v175867
  %t7987 = inttoptr i64 %v176150 to ptr
  store i8 0, ptr %t7987, align 1
  %t7988 = icmp eq i64 %v175902, %v176135
  %t7990.not245 = icmp eq i64 %v176135, 0
  %t7990.not = or i1 %t7988, %t7990.not245
  br i1 %t7990.not, label %block89446, label %edge13447

block89446:                                       ; preds = %block89441, %edge13447
  %t7994.not = or i1 %t7927.not241, %t7988
  br i1 %t7994.not, label %edge13416, label %edge13452

block89427:                                       ; preds = %edge13452, %block89442, %edge13427, %edge13419
  %v176094 = phi i64 [ %v176145, %block89442 ], [ 1, %edge13419 ], [ 1, %edge13427 ], [ %v176161, %edge13452 ]
  %t7996 = insertvalue { i64, i64 } poison, i64 %v176094, 0
  %t7997 = insertvalue { i64, i64 } %t7996, i64 7035, 1
  br label %common.ret

block89471:                                       ; preds = %edge13467, %edge13471, %edge13480
  %v176204 = phi i64 [ %v176209, %edge13471 ], [ %v176226, %edge13480 ], [ 64, %edge13467 ]
  %t8024 = call i64 @llvm.umax.i64(i64 %v176204, i64 %v176198)
  br label %block89468

block89468:                                       ; preds = %edge13464, %block89471
  %v176203 = phi i64 [ %t8024, %block89471 ], [ %v176191, %edge13464 ]
  %t8025.not = icmp eq i64 %v176186, 0
  br i1 %t8025.not, label %block89479, label %edge13484

block89479:                                       ; preds = %edge13484, %block89468
  %t8030 = zext i64 %v176189 to i128
  %t8031 = zext i64 %v176190 to i128
  %t8032 = shl nuw i128 %t8031, 64
  %v218774 = or disjoint i128 %t8032, %t8030
  store i128 %v218774, ptr %ss0, align 16
  %t8034 = zext i64 %v176191 to i128
  %v218776 = or disjoint i128 %t8034, 18446744073709551616
  store i128 %v218776, ptr %t1002, align 16
  %t8040 = zext i64 %v176186 to i128
  %t8041 = shl nuw i128 %t8040, 64
  %v218778 = or disjoint i128 %t8041, %t8031
  store i128 %v218778, ptr %t1007, align 16
  %v176243 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v176203, i64 1, i64 1, i64 %v3192)
  %t8044.not = icmp eq i64 %v176243, 0
  br i1 %t8044.not, label %edge13489, label %block89463

block89499:                                       ; preds = %edge13491, %edge13501
  %t8055.not = or i1 %t8025.not, %t8049
  br i1 %t8055.not, label %edge13510, label %edge13506

block89463:                                       ; preds = %edge13506, %block89479, %edge13471, %edge13416
  %v176185 = phi i64 [ %v176243, %block89479 ], [ 1, %edge13416 ], [ 1, %edge13471 ], [ %v176295, %edge13506 ]
  %t8057 = insertvalue { i64, i64 } poison, i64 %v176185, 0
  %t8058 = insertvalue { i64, i64 } %t8057, i64 7040, 1
  br label %common.ret

block89513:                                       ; preds = %edge13517, %edge13521, %edge13530
  %v176477 = phi i64 [ %v176482, %edge13521 ], [ %v176499, %edge13530 ], [ 64, %edge13517 ]
  %t8085 = call i64 @llvm.umax.i64(i64 %v176477, i64 %v176471)
  br label %block89510

block89510:                                       ; preds = %edge13514, %block89513
  %v176476 = phi i64 [ %t8085, %block89513 ], [ %v176230, %edge13514 ]
  br i1 %t8051.not249, label %block89521, label %edge13534

block89521:                                       ; preds = %edge13534, %block89510
  %t8091 = zext i64 %v176228 to i128
  %t8092 = zext i64 %v176198 to i128
  %t8093 = shl nuw i128 %t8092, 64
  %v218780 = or disjoint i128 %t8093, %t8091
  store i128 %v218780, ptr %ss0, align 16
  %t8095 = zext i64 %v176230 to i128
  %v218782 = or disjoint i128 %t8095, 18446744073709551616
  store i128 %v218782, ptr %t1002, align 16
  %t8101 = zext i64 %v176233 to i128
  %t8102 = shl nuw i128 %t8101, 64
  %v218784 = or disjoint i128 %t8102, %t8092
  store i128 %v218784, ptr %t1007, align 16
  %v176516 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v176476, i64 1, i64 1, i64 %v3192)
  %t8105.not = icmp eq i64 %v176516, 0
  br i1 %t8105.not, label %edge13539, label %block89505

block89541:                                       ; preds = %edge13541, %edge13551
  %t8116.not = or i1 %t8051.not249, %t8110
  br i1 %t8116.not, label %edge13560, label %edge13556

block89505:                                       ; preds = %edge13556, %block89521, %edge13521, %edge13510
  %v176464 = phi i64 [ %v176516, %block89521 ], [ 1, %edge13510 ], [ 1, %edge13521 ], [ %v176568, %edge13556 ]
  %t8118 = insertvalue { i64, i64 } poison, i64 %v176464, 0
  %t8119 = insertvalue { i64, i64 } %t8118, i64 7042, 1
  br label %common.ret

block89554:                                       ; preds = %edge13567, %edge13571, %edge13580
  %v176586 = phi i64 [ %v176591, %edge13571 ], [ %v176608, %edge13580 ], [ 64, %edge13567 ]
  %t8150 = call i64 @llvm.umax.i64(i64 %v176586, i64 %v176580)
  br label %block89551

block89551:                                       ; preds = %edge13564, %block89554
  %v176585 = phi i64 [ %t8150, %block89554 ], [ %v176503, %edge13564 ]
  br i1 %t8112.not253, label %block89562, label %edge13584

block89562:                                       ; preds = %edge13584, %block89551
  %t8156 = zext i64 %v176501 to i128
  %t8157 = zext i64 %v176471 to i128
  %t8158 = shl nuw i128 %t8157, 64
  %v218786 = or disjoint i128 %t8158, %t8156
  store i128 %v218786, ptr %ss0, align 16
  %t8160 = zext i64 %v176503 to i128
  %v218788 = or disjoint i128 %t8160, 18446744073709551616
  store i128 %v218788, ptr %t1002, align 16
  %t8166 = zext i64 %v176506 to i128
  %t8167 = shl nuw i128 %t8166, 64
  %v218790 = or disjoint i128 %t8167, %t8157
  store i128 %v218790, ptr %t1007, align 16
  %v176625 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v176585, i64 1, i64 1, i64 %v3192)
  %t8170.not = icmp eq i64 %v176625, 0
  br i1 %t8170.not, label %edge13589, label %block89546

block89561:                                       ; preds = %edge13584, %edge13589
  %v176610 = phi i64 [ %v176626, %edge13589 ], [ %v176501, %edge13584 ]
  %v176612 = phi i64 [ %v176627, %edge13589 ], [ %v176503, %edge13584 ]
  %v176615 = phi i64 [ %v176628, %edge13589 ], [ %v176506, %edge13584 ]
  %t8173.not = icmp eq i64 %t8123, 0
  br i1 %t8173.not, label %block89566, label %edge13591

block89566:                                       ; preds = %block89561, %edge13591
  %t8174 = icmp eq i64 %v176506, %v176615
  %t8176.not257 = icmp eq i64 %v176615, 0
  %t8176.not = or i1 %t8174, %t8176.not257
  br i1 %t8176.not, label %block89568, label %edge13594

block89568:                                       ; preds = %block89566, %edge13594
  %t8180.not = or i1 %t8112.not253, %t8174
  br i1 %t8180.not, label %edge13603, label %edge13599

block89546:                                       ; preds = %edge13599, %block89562, %edge13571, %edge13562, %edge13560
  %v176570 = phi i64 [ %v176625, %block89562 ], [ 103427, %edge13560 ], [ 1, %edge13562 ], [ 1, %edge13571 ], [ %v176643, %edge13599 ]
  %t8182 = insertvalue { i64, i64 } poison, i64 %v176570, 0
  %t8183 = insertvalue { i64, i64 } %t8182, i64 7043, 1
  br label %common.ret

block89751:                                       ; preds = %edge13645, %edge13641
  %t8203.not = icmp eq i64 %v129512, 0
  br i1 %t8203.not, label %edge13664, label %edge13651

block89747:                                       ; preds = %edge13651, %edge13645
  %v176895 = phi i64 [ %v176902, %edge13645 ], [ %v176906, %edge13651 ]
  %t8205 = insertvalue { i64, i64 } poison, i64 %v176895, 0
  %t8206 = insertvalue { i64, i64 } %t8205, i64 7063, 1
  br label %common.ret

block89907:                                       ; preds = %edge13682, %edge13686, %edge13695
  %v177151 = phi i64 [ %v177156, %edge13686 ], [ %v177173, %edge13695 ], [ 64, %edge13682 ]
  %t8248 = call i64 @llvm.umax.i64(i64 %v177151, i64 %v177145)
  br label %block89904

block89904:                                       ; preds = %edge13679, %block89907
  %v177150 = phi i64 [ %t8248, %block89907 ], [ %v217707, %edge13679 ]
  %t8249.not = icmp eq i64 %v217710, 0
  br i1 %t8249.not, label %block89915, label %edge13699

block89915:                                       ; preds = %edge13699, %block89904
  %t8254 = zext i64 %v217705 to i128
  %t8255 = zext i64 %v217706 to i128
  %t8256 = shl nuw i128 %t8255, 64
  %v218792 = or disjoint i128 %t8256, %t8254
  store i128 %v218792, ptr %ss0, align 16
  %t8258 = zext i64 %v217707 to i128
  %v218794 = or disjoint i128 %t8258, 18446744073709551616
  store i128 %v218794, ptr %t1002, align 16
  %t8263 = zext i64 %v217709 to i128
  %t8264 = zext i64 %v217710 to i128
  %t8265 = shl nuw i128 %t8264, 64
  %v218796 = or disjoint i128 %t8265, %t8263
  store i128 %v218796, ptr %t1007, align 16
  %v177190 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v177150, i64 1, i64 1, i64 %v3192)
  %t8268.not = icmp eq i64 %v177190, 0
  br i1 %t8268.not, label %edge13704, label %block89899

block89914:                                       ; preds = %edge13699, %edge13704
  %v177175 = phi i64 [ %v177191, %edge13704 ], [ %v217705, %edge13699 ]
  %v177177 = phi i64 [ %v177192, %edge13704 ], [ %v217707, %edge13699 ]
  %v177180 = phi i64 [ %v177193, %edge13704 ], [ %v217710, %edge13699 ]
  %t8271.not = icmp eq i64 %v177125, 0
  br i1 %t8271.not, label %block89933, label %edge13706

block89933:                                       ; preds = %edge13706, %block89914
  %t8289 = icmp eq i64 %v217710, %v177180
  %t8291.not229 = icmp eq i64 %v177180, 0
  %t8291.not = or i1 %t8289, %t8291.not229
  br i1 %t8291.not, label %block89935, label %edge13726

block89935:                                       ; preds = %block89933, %edge13726
  %t8295.not = or i1 %t8249.not, %t8289
  br i1 %t8295.not, label %edge13656.thread, label %edge13731

block89899:                                       ; preds = %edge13731, %block89915, %edge13686, %edge13664
  %v177138 = phi i64 [ %v177190, %block89915 ], [ %v177242, %edge13731 ], [ 1, %edge13686 ], [ 1, %edge13664 ]
  %t8297 = insertvalue { i64, i64 } poison, i64 %v177138, 0
  %t8298 = insertvalue { i64, i64 } %t8297, i64 7078, 1
  br label %common.ret

block90331:                                       ; preds = %edge13851, %edge13855, %edge13864
  %v178300 = phi i64 [ %v178305, %edge13855 ], [ %v178322, %edge13864 ], [ 64, %edge13851 ]
  %t8382 = call i64 @llvm.umax.i64(i64 %v178300, i64 %v178294)
  br label %block90328

block90328:                                       ; preds = %edge13848, %block90331
  %v178299 = phi i64 [ %t8382, %block90331 ], [ %v217707, %edge13848 ]
  %t8383.not = icmp eq i64 %v217710, 0
  br i1 %t8383.not, label %block90339, label %edge13868

block90339:                                       ; preds = %edge13868, %block90328
  %t8388 = zext i64 %v217705 to i128
  %t8389 = zext i64 %v217706 to i128
  %t8390 = shl nuw i128 %t8389, 64
  %v218798 = or disjoint i128 %t8390, %t8388
  store i128 %v218798, ptr %ss0, align 16
  %t8392 = zext i64 %v217707 to i128
  %v218800 = or disjoint i128 %t8392, 18446744073709551616
  store i128 %v218800, ptr %t1002, align 16
  %t8397 = zext i64 %v217709 to i128
  %t8398 = zext i64 %v217710 to i128
  %t8399 = shl nuw i128 %t8398, 64
  %v218802 = or disjoint i128 %t8399, %t8397
  store i128 %v218802, ptr %t1007, align 16
  %v178339 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v178299, i64 1, i64 1, i64 %v3192)
  %t8402.not = icmp eq i64 %v178339, 0
  br i1 %t8402.not, label %edge13873, label %block90323

block90357:                                       ; preds = %edge13881, %edge13875
  %t8429 = icmp eq i64 %v217710, %v178329
  %t8431.not217 = icmp eq i64 %v178329, 0
  %t8431.not = or i1 %t8429, %t8431.not217
  br i1 %t8431.not, label %block90359, label %edge13899

block90359:                                       ; preds = %block90357, %edge13899
  %t8435.not = or i1 %t8383.not, %t8429
  br i1 %t8435.not, label %edge13908, label %edge13904

block90323:                                       ; preds = %edge13904, %block90339, %edge13855, %edge13832
  %v178287 = phi i64 [ %v178339, %block90339 ], [ %v178391, %edge13904 ], [ 1, %edge13855 ], [ 1, %edge13832 ]
  %t8437 = insertvalue { i64, i64 } poison, i64 %v178287, 0
  %t8438 = insertvalue { i64, i64 } %t8437, i64 7164, 1
  br label %common.ret

block7059:                                        ; preds = %edge14027, %edge13927
  %v178503 = phi i64 [ 1, %edge13927 ], [ %v179631, %edge14027 ]
  %v178506 = phi i64 [ %v178408, %edge13927 ], [ %v179446, %edge14027 ]
  %v178509 = phi i64 [ %v178413, %edge13927 ], [ %v179639, %edge14027 ]
  %or.cond316 = icmp samesign ult i64 %v178503, %invariant.umin
  br i1 %or.cond316, label %edge13964, label %edge13976

block91228:                                       ; preds = %edge14058, %edge14062, %edge14071
  %v179726 = phi i64 [ %v179731, %edge14062 ], [ %v179748, %edge14071 ], [ 64, %edge14058 ]
  %t8504 = call i64 @llvm.umax.i64(i64 %v179726, i64 %v179720)
  br label %block91225

block91225:                                       ; preds = %edge14055, %block91228
  %v179725 = phi i64 [ %t8504, %block91228 ], [ %v178326, %edge14055 ]
  br i1 %t8431.not217, label %block91236, label %edge14075

block91236:                                       ; preds = %edge14075, %block91225
  %t8510 = zext i64 %v178324 to i128
  %t8511 = zext i64 %v178294 to i128
  %t8512 = shl nuw i128 %t8511, 64
  %v218804 = or disjoint i128 %t8512, %t8510
  store i128 %v218804, ptr %ss0, align 16
  %t8514 = zext i64 %v178326 to i128
  %v218806 = or disjoint i128 %t8514, 18446744073709551616
  store i128 %v218806, ptr %t1002, align 16
  %t8520 = zext i64 %v178329 to i128
  %t8521 = shl nuw i128 %t8520, 64
  %v218808 = or disjoint i128 %t8521, %t8511
  store i128 %v218808, ptr %t1007, align 16
  %v179765 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v179725, i64 1, i64 1, i64 %v3192)
  %t8524.not = icmp eq i64 %v179765, 0
  br i1 %t8524.not, label %edge14080, label %block91220

block91235:                                       ; preds = %edge14075, %edge14080
  %v179750 = phi i64 [ %v179766, %edge14080 ], [ %v178324, %edge14075 ]
  %v179752 = phi i64 [ %v179767, %edge14080 ], [ %v178326, %edge14075 ]
  %v179755 = phi i64 [ %v179768, %edge14080 ], [ %v178329, %edge14075 ]
  %t8527.not = icmp eq i64 %v179705, 0
  br i1 %t8527.not, label %block91254, label %edge14082

block91254:                                       ; preds = %edge14100, %edge14097, %edge14094, %edge14091, %edge14088, %edge14085, %edge14082, %block91235, %edge14103
  %t8551 = icmp eq i64 %v178329, %v179755
  %t8553.not221 = icmp eq i64 %v179755, 0
  %t8553.not = or i1 %t8551, %t8553.not221
  br i1 %t8553.not, label %block91256, label %edge14106

block91256:                                       ; preds = %block91254, %edge14106
  %t8557.not = or i1 %t8431.not217, %t8551
  br i1 %t8557.not, label %edge14115, label %edge14111

block91220:                                       ; preds = %edge14111, %block91236, %edge14062, %edge14053, %edge14047
  %v179713 = phi i64 [ %v179765, %block91236 ], [ 103427, %edge14047 ], [ 1, %edge14053 ], [ 1, %edge14062 ], [ %v179817, %edge14111 ]
  %t8559 = insertvalue { i64, i64 } poison, i64 %v179713, 0
  %t8560 = insertvalue { i64, i64 } %t8559, i64 7271, 1
  br label %common.ret

block7165:                                        ; preds = %edge14325, %edge14115
  %v180280 = phi i64 [ %v179750, %edge14115 ], [ %v181565, %edge14325 ]
  %v180281 = phi i64 [ %v179720, %edge14115 ], [ %v181535, %edge14325 ]
  %v180282 = phi i64 [ %v179752, %edge14115 ], [ %v181567, %edge14325 ]
  %v180285 = phi i64 [ %v179755, %edge14115 ], [ %v181570, %edge14325 ]
  %v180292 = phi i64 [ %v179834, %edge14115 ], [ %v181649, %edge14325 ]
  %v180295.in = phi i64 [ %v178509, %edge14115 ], [ %v181506, %edge14325 ]
  %v180337 = phi i64 [ 0, %edge14115 ], [ %v181885, %edge14325 ]
  %v180295 = and i64 %v180295.in, 7
  %exitcond20655.not = icmp eq i64 %v180337, %v129513
  br i1 %exitcond20655.not, label %edge14164, label %edge14174

block92014:                                       ; preds = %edge14268, %edge14272, %edge14281
  %v181541 = phi i64 [ %v181546, %edge14272 ], [ %v181563, %edge14281 ], [ 64, %edge14268 ]
  %t8622 = call i64 @llvm.umax.i64(i64 %v181541, i64 %v181535)
  br label %block92011

block92011:                                       ; preds = %edge14265, %block92014
  %v181540 = phi i64 [ %t8622, %block92014 ], [ %v180282, %edge14265 ]
  %t8623.not = icmp eq i64 %v180285, 0
  br i1 %t8623.not, label %block92022, label %edge14285

block92022:                                       ; preds = %edge14285, %block92011
  %t8628 = zext i64 %v180280 to i128
  %t8629 = zext i64 %v180281 to i128
  %t8630 = shl nuw i128 %t8629, 64
  %v218810 = or disjoint i128 %t8630, %t8628
  store i128 %v218810, ptr %ss0, align 16
  %t8632 = zext i64 %v180282 to i128
  %v218812 = or disjoint i128 %t8632, 18446744073709551616
  store i128 %v218812, ptr %t1002, align 16
  %t8638 = zext i64 %v180285 to i128
  %t8639 = shl nuw i128 %t8638, 64
  %v218814 = or disjoint i128 %t8639, %t8629
  store i128 %v218814, ptr %t1007, align 16
  %v181580 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v181540, i64 1, i64 1, i64 %v3192)
  %t8642.not = icmp eq i64 %v181580, 0
  br i1 %t8642.not, label %edge14290, label %block92006

block92021:                                       ; preds = %edge14285, %edge14290
  %v181565 = phi i64 [ %v181581, %edge14290 ], [ %v180280, %edge14285 ]
  %v181567 = phi i64 [ %v181582, %edge14290 ], [ %v180282, %edge14285 ]
  %v181570 = phi i64 [ %v181583, %edge14290 ], [ %v180285, %edge14285 ]
  %t8645.not = icmp eq i64 %v181520, 0
  br i1 %t8645.not, label %block92040, label %edge14292

block92040:                                       ; preds = %edge14310, %edge14307, %edge14304, %edge14301, %edge14298, %edge14295, %edge14292, %block92021, %edge14313
  %t8669 = icmp eq i64 %v180285, %v181570
  %t8671.not225 = icmp eq i64 %v181570, 0
  %t8671.not = or i1 %t8669, %t8671.not225
  br i1 %t8671.not, label %block92042, label %edge14316

block92042:                                       ; preds = %block92040, %edge14316
  %t8675.not = or i1 %t8623.not, %t8669
  br i1 %t8675.not, label %edge14325, label %edge14321

block92006:                                       ; preds = %edge14321, %block92022, %edge14272, %edge14263, %edge14254
  %v181528 = phi i64 [ %v181580, %block92022 ], [ 103427, %edge14254 ], [ 1, %edge14263 ], [ 1, %edge14272 ], [ %v181632, %edge14321 ]
  %t8677 = insertvalue { i64, i64 } poison, i64 %v181528, 0
  %t8678 = insertvalue { i64, i64 } %t8677, i64 7382, 1
  br label %common.ret

block92077:                                       ; preds = %edge14337, %edge14164
  br i1 %t8441.not, label %edge14397, label %edge14341

block92073:                                       ; preds = %edge14344, %edge14337
  %v181690 = phi i64 [ %v181697, %edge14337 ], [ %v181701, %edge14344 ]
  %t8688 = insertvalue { i64, i64 } poison, i64 %v181690, 0
  %t8689 = insertvalue { i64, i64 } %t8688, i64 7393, 1
  br label %common.ret

block92420:                                       ; preds = %edge14397, %edge14404
  %t8704.not = icmp eq i64 %v182973, 0
  br i1 %t8704.not, label %edge14421, label %edge14407

block7301:                                        ; preds = %edge15074, %edge14421
  %v184216 = phi i64 [ %v1829001051, %edge14421 ], [ %v191077, %edge15074 ]
  %v184217 = phi i64 [ %v1829011053, %edge14421 ], [ %v191047, %edge15074 ]
  %v184218 = phi i64 [ %v1829021055, %edge14421 ], [ %v191079, %edge15074 ]
  %v184221 = phi i64 [ %v1829051057, %edge14421 ], [ %v191082, %edge15074 ]
  %v184228 = phi i64 [ %v1829121059, %edge14421 ], [ %v191161, %edge15074 ]
  %v184231 = phi i64 [ %v1829151061, %edge14421 ], [ %v191174, %edge15074 ]
  %v184267 = phi i64 [ 0, %edge14421 ], [ %v191451, %edge15074 ]
  %t8707.not = icmp eq i64 %v184267, %v54999
  br i1 %t8707.not, label %edge15003, label %edge14457

block93979:                                       ; preds = %edge14626, %edge14627
  br i1 %t8704.not, label %block7494.preheader, label %edge14630

block7494.preheader:                              ; preds = %edge14630, %block93979
  br label %block7494

block7494:                                        ; preds = %block7494.preheader, %edge14747
  %v187015 = phi i64 [ %v188035, %edge14747 ], [ 0, %block7494.preheader ]
  %v187018 = phi i64 [ %v188047, %edge14747 ], [ 0, %block7494.preheader ]
  %v187033 = phi i64 [ %v188052, %edge14747 ], [ %v184750, %block7494.preheader ]
  %v187048 = phi i64 [ %v188247, %edge14747 ], [ %v184755, %block7494.preheader ]
  %t8763.not = icmp ne i64 %v187048, 0
  %t8766 = icmp ult i64 %v187033, %v1395
  %or.cond317 = select i1 %t8763.not, i1 %t8766, i1 false
  br i1 %or.cond317, label %edge14673, label %edge14688

trap8773:                                         ; preds = %edge14677
  call void @llvm.trap()
  unreachable

ok8773:                                           ; preds = %edge14677
  %v187494 = udiv i64 %v187033, %v1397
  %v187495 = mul i64 %v187494, %v1397
  %v187496 = sub nuw i64 %v187033, %v187495
  %v187497 = mul i64 %v187496, %v1398
  %v187498 = add i64 %v187497, %v187494
  br label %block94330

block94330:                                       ; preds = %edge14673, %ok8773
  %v187493 = phi i64 [ %v187498, %ok8773 ], [ %v187033, %edge14673 ]
  %v187500 = add i64 %v187493, %v1394
  %t8774 = inttoptr i64 %v187500 to ptr
  %t8775 = load i8, ptr %t8774, align 1
  %v187501 = zext i8 %t8775 to i64
  br label %edge14688

block94680:                                       ; preds = %edge14733, %edge14730
  br i1 %t8704.not, label %edge14998, label %edge14737

block94676:                                       ; preds = %edge14740, %edge14733
  %v188054 = phi i64 [ %v188061, %edge14733 ], [ %v188065, %edge14740 ]
  %t8794 = insertvalue { i64, i64 } poison, i64 %v188054, 0
  %t8795 = insertvalue { i64, i64 } %t8794, i64 7701, 1
  br label %common.ret

block96360:                                       ; preds = %edge15017, %edge15021, %edge15030
  %v191053 = phi i64 [ %v191058, %edge15021 ], [ %v191075, %edge15030 ], [ 64, %edge15017 ]
  %t8880 = call i64 @llvm.umax.i64(i64 %v191053, i64 %v191047)
  br label %block96357

block96357:                                       ; preds = %edge15014, %block96360
  %v191052 = phi i64 [ %t8880, %block96360 ], [ %v184218, %edge15014 ]
  %t8881.not = icmp eq i64 %v184221, 0
  br i1 %t8881.not, label %block96368, label %edge15034

block96368:                                       ; preds = %edge15034, %block96357
  %t8886 = zext i64 %v184216 to i128
  %t8887 = zext i64 %v184217 to i128
  %t8888 = shl nuw i128 %t8887, 64
  %v218816 = or disjoint i128 %t8888, %t8886
  store i128 %v218816, ptr %ss0, align 16
  %t8890 = zext i64 %v184218 to i128
  %v218818 = or disjoint i128 %t8890, 18446744073709551616
  store i128 %v218818, ptr %t1002, align 16
  %t8896 = zext i64 %v184221 to i128
  %t8897 = shl nuw i128 %t8896, 64
  %v218820 = or disjoint i128 %t8897, %t8887
  store i128 %v218820, ptr %t1007, align 16
  %v191092 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v191052, i64 1, i64 1, i64 %v3192)
  %t8900.not = icmp eq i64 %v191092, 0
  br i1 %t8900.not, label %edge15039, label %block96352

block96367:                                       ; preds = %edge15034, %edge15039
  %v191077 = phi i64 [ %v191093, %edge15039 ], [ %v184216, %edge15034 ]
  %v191079 = phi i64 [ %v191094, %edge15039 ], [ %v184218, %edge15034 ]
  %v191082 = phi i64 [ %v191095, %edge15039 ], [ %v184221, %edge15034 ]
  %t8903.not = icmp eq i64 %v190984, 0
  br i1 %t8903.not, label %block96386, label %edge15041

block96386:                                       ; preds = %edge15059, %edge15056, %edge15053, %edge15050, %edge15047, %edge15044, %edge15041, %block96367, %edge15062
  %t8927 = icmp eq i64 %v184221, %v191082
  %t8929.not237 = icmp eq i64 %v191082, 0
  %t8929.not = or i1 %t8927, %t8929.not237
  br i1 %t8929.not, label %block96388, label %edge15065

block96388:                                       ; preds = %block96386, %edge15065
  %t8933.not = or i1 %t8881.not, %t8927
  br i1 %t8933.not, label %edge15074, label %edge15070

block96352:                                       ; preds = %edge15070, %block96368, %edge15021, %edge15012, %edge15005
  %v190992 = phi i64 [ %v191092, %block96368 ], [ 103427, %edge15005 ], [ 1, %edge15012 ], [ 1, %edge15021 ], [ %v191144, %edge15070 ]
  %t8935 = insertvalue { i64, i64 } poison, i64 %v190992, 0
  %t8936 = insertvalue { i64, i64 } %t8935, i64 7940, 1
  br label %common.ret

block96423:                                       ; preds = %edge15086, %edge15003
  br i1 %t7746.not, label %block96427, label %edge15092

block96427:                                       ; preds = %edge15092, %block96423
  br i1 %t8704.not, label %edge15143.thread, label %edge15096

block96419:                                       ; preds = %edge15099, %edge15092, %edge15086
  %v191209 = phi i64 [ %v191220, %edge15092 ], [ %v191216, %edge15086 ], [ %v191224, %edge15099 ]
  %t8948 = insertvalue { i64, i64 } poison, i64 %v191209, 0
  %t8949 = insertvalue { i64, i64 } %t8948, i64 7951, 1
  br label %common.ret

block96806:                                       ; preds = %edge15195, %edge15199, %edge15208
  %v192430 = phi i64 [ %v192435, %edge15199 ], [ %v192452, %edge15208 ], [ 64, %edge15195 ]
  %t9002 = call i64 @llvm.umax.i64(i64 %v192430, i64 %v192424)
  br label %block96803

block96803:                                       ; preds = %edge15192, %block96806
  %v192429 = phi i64 [ %t9002, %block96806 ], [ %v184218, %edge15192 ]
  %t9003.not = icmp eq i64 %v184221, 0
  br i1 %t9003.not, label %block96814, label %edge15212

block96814:                                       ; preds = %edge15212, %block96803
  %t9008 = zext i64 %v184216 to i128
  %t9009 = zext i64 %v184217 to i128
  %t9010 = shl nuw i128 %t9009, 64
  %v218822 = or disjoint i128 %t9010, %t9008
  store i128 %v218822, ptr %ss0, align 16
  %t9012 = zext i64 %v184218 to i128
  %v218824 = or disjoint i128 %t9012, 18446744073709551616
  store i128 %v218824, ptr %t1002, align 16
  %t9018 = zext i64 %v184221 to i128
  %t9019 = shl nuw i128 %t9018, 64
  %v218826 = or disjoint i128 %t9019, %t9009
  store i128 %v218826, ptr %t1007, align 16
  %v192469 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v192429, i64 1, i64 1, i64 %v3192)
  %t9022.not = icmp eq i64 %v192469, 0
  br i1 %t9022.not, label %edge15217, label %block96798

block96813:                                       ; preds = %edge15212, %edge15217
  %v192454 = phi i64 [ %v192470, %edge15217 ], [ %v184216, %edge15212 ]
  %v192456 = phi i64 [ %v192471, %edge15217 ], [ %v184218, %edge15212 ]
  %v192459 = phi i64 [ %v192472, %edge15217 ], [ %v184221, %edge15212 ]
  %t9025.not = icmp eq i64 %v192409, 0
  br i1 %t9025.not, label %block96832, label %edge15219

block96832:                                       ; preds = %edge15237, %edge15234, %edge15231, %edge15228, %edge15225, %edge15222, %edge15219, %block96813, %edge15240
  %t9049 = icmp eq i64 %v184221, %v192459
  %t9051.not233 = icmp eq i64 %v192459, 0
  %t9051.not = or i1 %t9049, %t9051.not233
  br i1 %t9051.not, label %block96834, label %edge15243

block96834:                                       ; preds = %block96832, %edge15243
  %t9055.not = or i1 %t9003.not, %t9049
  br i1 %t9055.not, label %edge15252, label %edge15248

block96798:                                       ; preds = %edge15248, %block96814, %edge15199, %edge15190, %edge15171
  %v192417 = phi i64 [ %v192469, %block96814 ], [ 103427, %edge15171 ], [ 1, %edge15190 ], [ 1, %edge15199 ], [ %v192521, %edge15248 ]
  %t9057 = insertvalue { i64, i64 } poison, i64 %v192417, 0
  %t9058 = insertvalue { i64, i64 } %t9057, i64 8000, 1
  br label %common.ret

block96882:                                       ; preds = %edge15281, %edge15278
  %t9067.not = icmp eq i64 %v54954, 0
  br i1 %t9067.not, label %edge15527, label %edge15287

block96878:                                       ; preds = %edge15287, %edge15281
  %v194374 = phi i64 [ %v194382, %edge15281 ], [ %v194386, %edge15287 ]
  %t9069 = insertvalue { i64, i64 } poison, i64 %v194374, 0
  %t9070 = insertvalue { i64, i64 } %t9069, i64 8020, 1
  br label %common.ret

block7887:                                        ; preds = %edge15273, %edge15386
  %v194541 = phi i64 [ %v195154, %edge15386 ], [ %v54924, %edge15273 ]
  %v194561 = phi i64 [ %v195151, %edge15386 ], [ %v54921, %edge15273 ]
  %v194564 = phi i64 [ %v195369, %edge15386 ], [ 0, %edge15273 ]
  %v194565 = phi i64 [ %v195149, %edge15386 ], [ %v54919, %edge15273 ]
  %t9071.not = icmp eq i64 %v194541, 0
  br i1 %t9071.not, label %edge15319, label %edge15299

block97393:                                       ; preds = %edge15358, %edge15357
  %t9130 = zext i64 %v194565 to i128
  %v218836 = or disjoint i128 %t1719, %t9130
  store i128 %v218836, ptr %ss0, align 16
  %t9134 = zext i64 %v194561 to i128
  %v218838 = or disjoint i128 %t9134, 18446744073709551616
  store i128 %v218838, ptr %t1002, align 16
  %t9140 = zext i64 %v194541 to i128
  %t9141 = shl nuw i128 %t9140, 64
  %v218840 = or disjoint i128 %t9141, %t1163
  store i128 %v218840, ptr %t1007, align 16
  %v195170 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v194561, i64 4, i64 4, i64 %v3192)
  %t9144.not = icmp eq i64 %v195170, 0
  br i1 %t9144.not, label %edge15363, label %block97388

block97392:                                       ; preds = %edge15358, %edge15363
  %t9147.pre-phi = phi ptr [ %t9121, %edge15358 ], [ %.pre20684, %edge15363 ]
  %v195155 = phi i64 [ %v194565, %edge15358 ], [ %v195171, %edge15363 ]
  %v195157 = phi i64 [ %v194561, %edge15358 ], [ %v195172, %edge15363 ]
  %v195160 = phi i64 [ %v194541, %edge15358 ], [ %v195173, %edge15363 ]
  %v195175 = trunc nuw i64 %v195186 to i32
  store i32 %v195175, ptr %t9147.pre-phi, align 1
  br label %block97391

block97391:                                       ; preds = %edge15354, %block97392
  %v195149 = phi i64 [ %v195155, %block97392 ], [ %v194565, %edge15354 ]
  %v195151 = phi i64 [ %v195157, %block97392 ], [ %v194561, %edge15354 ]
  %v195154 = phi i64 [ %v195160, %block97392 ], [ %v194541, %edge15354 ]
  %t9148 = icmp eq i64 %v194541, %v195154
  %t9150.not260 = icmp eq i64 %v195154, 0
  %t9150.not = or i1 %t9148, %t9150.not260
  br i1 %t9150.not, label %block97397, label %edge15366

block97397:                                       ; preds = %block97391, %edge15366
  %t9154.not = or i1 %t9071.not, %t9148
  br i1 %t9154.not, label %edge15386, label %edge15371

block97388:                                       ; preds = %edge15371, %block97393, %edge15351
  %v195137 = phi i64 [ %v195170, %block97393 ], [ 103425, %edge15351 ], [ %v195184, %edge15371 ]
  %t9156 = insertvalue { i64, i64 } poison, i64 %v195137, 0
  %t9157 = insertvalue { i64, i64 } %t9156, i64 8064, 1
  br label %common.ret

block97576:                                       ; preds = %edge15431
  %t9189 = insertvalue { i64, i64 } poison, i64 %v196545, 0
  %t9190 = insertvalue { i64, i64 } %t9189, i64 8087, 1
  br label %common.ret

block97631:                                       ; preds = %edge15474
  %t9223 = insertvalue { i64, i64 } poison, i64 %v197788, 0
  %t9224 = insertvalue { i64, i64 } %t9223, i64 8100, 1
  br label %common.ret

block97730:                                       ; preds = %edge15527
  %t9252 = insertvalue { i64, i64 } poison, i64 %v199164, 0
  %t9253 = insertvalue { i64, i64 } %t9252, i64 8117, 1
  br label %common.ret

block97785:                                       ; preds = %edge15567
  %t9281 = insertvalue { i64, i64 } poison, i64 %v200403, 0
  %t9282 = insertvalue { i64, i64 } %t9281, i64 8130, 1
  br label %common.ret

block97840:                                       ; preds = %edge15607
  %t9310 = insertvalue { i64, i64 } poison, i64 %v201658, 0
  %t9311 = insertvalue { i64, i64 } %t9310, i64 8143, 1
  br label %common.ret

block97895:                                       ; preds = %edge15650
  %t9344 = insertvalue { i64, i64 } poison, i64 %v202921, 0
  %t9345 = insertvalue { i64, i64 } %t9344, i64 8156, 1
  br label %common.ret

block98105:                                       ; preds = %edge15721, %edge15724
  %t9353.not = icmp eq i64 %v138168, 0
  br i1 %t9353.not, label %block98109, label %edge15736

block98109:                                       ; preds = %edge15736, %block98105
  %t9355.not = icmp eq i64 %v217631, 0
  br i1 %t9355.not, label %block98113, label %edge15742

block98113:                                       ; preds = %edge15742, %block98109
  %t9357.not = icmp eq i64 %v54918, 0
  br i1 %t9357.not, label %block98117, label %edge15748

block98117:                                       ; preds = %edge15748, %block98113
  %t9359.not = icmp eq i64 %v203705, 0
  br i1 %t9359.not, label %block98121, label %edge15754

block98121:                                       ; preds = %edge15754, %block98117
  br i1 %t7746.not, label %block98125, label %edge15760

block98125:                                       ; preds = %edge15760, %block98121
  %t9363.not = icmp eq i64 %v204274, 0
  br i1 %t9363.not, label %block98129, label %edge15766

block98129:                                       ; preds = %edge15766, %block98125
  %t9365.not = icmp eq i64 %v204286, 0
  br i1 %t9365.not, label %block98133, label %edge15772

block98133:                                       ; preds = %edge15772, %block98129
  br i1 %t3704.not, label %block98137, label %edge15778

block98137:                                       ; preds = %edge15778, %block98133
  %t9369.not = icmp eq i64 %v204879, 0
  br i1 %t9369.not, label %block98141, label %edge15784

block98141:                                       ; preds = %edge15784, %block98137
  %t9371.not = icmp eq i64 %v204891, 0
  br i1 %t9371.not, label %edge15840, label %edge15788

block98033:                                       ; preds = %edge15791, %edge15784, %edge15778, %edge15772, %edge15766, %edge15760, %edge15754, %edge15748, %edge15742, %edge15736, %edge15724
  %v205086 = phi i64 [ %v205210, %edge15784 ], [ %v205170, %edge15724 ], [ %v205214, %edge15791 ], [ %v205178, %edge15736 ], [ %v205182, %edge15742 ], [ %v205186, %edge15748 ], [ %v205190, %edge15754 ], [ %v205194, %edge15760 ], [ %v205198, %edge15766 ], [ %v205202, %edge15772 ], [ %v205206, %edge15778 ]
  %t9376 = insertvalue { i64, i64 } poison, i64 %v205086, 0
  %t9377 = insertvalue { i64, i64 } %t9376, i64 8190, 1
  br label %common.ret

block98432:                                       ; preds = %edge15857, %edge15861, %edge15870
  %v217161 = phi i64 [ %v217166, %edge15861 ], [ %v217183, %edge15870 ], [ 64, %edge15857 ]
  %t9408 = call i64 @llvm.umax.i64(i64 %v217161, i64 %v217155)
  br label %block98429

block98429:                                       ; preds = %edge15854, %block98432
  %v217160 = phi i64 [ %t9408, %block98432 ], [ %v204964, %edge15854 ]
  %t9409.not = icmp eq i64 %v205006, 0
  br i1 %t9409.not, label %block98440, label %edge15874

block98440:                                       ; preds = %edge15874, %block98429
  %t9414 = zext i64 %v204936 to i128
  %t9415 = zext i64 %v204950 to i128
  %t9416 = shl nuw i128 %t9415, 64
  %v218842 = or disjoint i128 %t9416, %t9414
  store i128 %v218842, ptr %ss0, align 16
  %t9418 = zext i64 %v204964 to i128
  %v218844 = or disjoint i128 %t9418, 18446744073709551616
  store i128 %v218844, ptr %t1002, align 16
  %t9424 = zext i64 %v205006 to i128
  %t9425 = shl nuw i128 %t9424, 64
  %v218846 = or disjoint i128 %t9425, %t9415
  store i128 %v218846, ptr %t1007, align 16
  %v217200 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 1, i64 %v217160, i64 1, i64 1, i64 %v3192)
  %t9428.not = icmp eq i64 %v217200, 0
  br i1 %t9428.not, label %edge15879, label %block98425

block98439:                                       ; preds = %edge15874, %edge15879
  %v217185 = phi i64 [ %v217201, %edge15879 ], [ %v204936, %edge15874 ]
  %v217187 = phi i64 [ %v217202, %edge15879 ], [ %v204964, %edge15874 ]
  %v217190 = phi i64 [ %v217203, %edge15879 ], [ %v205006, %edge15874 ]
  %v217205 = add i64 %v217185, %v204950
  %v217206 = trunc i64 %v205026 to i8
  %t9431 = inttoptr i64 %v217205 to ptr
  store i8 %v217206, ptr %t9431, align 1
  %t9432 = icmp eq i64 %v205006, %v217190
  %t9434.not264 = icmp eq i64 %v217190, 0
  %t9434.not = or i1 %t9432, %t9434.not264
  br i1 %t9434.not, label %block98444, label %edge15881

block98444:                                       ; preds = %block98439, %edge15881
  %t9438.not = or i1 %t9409.not, %t9432
  br i1 %t9438.not, label %edge15901, label %edge15886

block98425:                                       ; preds = %edge15886, %block98440, %edge15861, %edge15849
  %v217149 = phi i64 [ %v217200, %block98440 ], [ 1, %edge15849 ], [ 1, %edge15861 ], [ %v217216, %edge15886 ]
  %t9440 = insertvalue { i64, i64 } poison, i64 %v217149, 0
  %t9441 = insertvalue { i64, i64 } %t9440, i64 8214, 1
  br label %common.ret

block98481:                                       ; preds = %edge15902, %edge15901
  %t9446 = inttoptr i64 %v2 to ptr
  store i64 %v217258, ptr %t9446, align 1
  %t9448 = getelementptr i8, ptr %t9446, i64 8
  store i64 %v217259, ptr %t9448, align 1
  %t9450 = getelementptr i8, ptr %t9446, i64 16
  store i64 %v217260, ptr %t9450, align 1
  %t9452 = getelementptr i8, ptr %t9446, i64 24
  store i64 1, ptr %t9452, align 1
  %t9454 = getelementptr i8, ptr %t9446, i64 32
  store i64 %v217259, ptr %t9454, align 1
  %t9456 = getelementptr i8, ptr %t9446, i64 40
  store i64 %v217263, ptr %t9456, align 1
  br label %common.ret

edge0:                                            ; preds = %block0
  %t24 = inttoptr i64 %v1399 to ptr
  %v1586 = load i64, ptr %t24, align 1
  %v1587 = add i64 %v1586, 1
  store i64 %v1587, ptr %t24, align 1
  br label %block2

edge3:                                            ; preds = %block2
  %t27 = inttoptr i64 %v1405 to ptr
  %v1588 = load i64, ptr %t27, align 1
  %v1589 = add i64 %v1588, 1
  store i64 %v1589, ptr %t27, align 1
  br label %edge12

edge12:                                           ; preds = %edge3, %block2
  %t29 = icmp ult i64 %v1395, 8
  br i1 %t29, label %edge38, label %edge462

edge38:                                           ; preds = %edge12
  %t44 = icmp eq i64 %v1403, 1
  br i1 %t44, label %edge47, label %block8249

edge47:                                           ; preds = %edge38
  %v1825 = add nuw nsw i64 %v1395, 5
  %v1837 = tail call i64 @llvm.uadd.sat.i64(i64 %v1401, i64 %v1825)
  %t49 = icmp ugt i64 %v1837, %v1402
  br i1 %t49, label %edge49, label %edge68

edge49:                                           ; preds = %edge47
  %v1855 = ptrtoint ptr %ss1 to i64
  %v218049 = ptrtoint ptr %ss0 to i64
  %t56 = zext i64 %v1400 to i128
  %t57 = zext i64 %v1401 to i128
  %t58 = shl nuw i128 %t57, 64
  %v218050 = or disjoint i128 %t58, %t56
  store i128 %v218050, ptr %ss0, align 16
  %t60 = zext i64 %v1402 to i128
  %v218052 = or disjoint i128 %t60, 18446744073709551616
  %t64 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  store i128 %v218052, ptr %t64, align 16
  %t65 = zext i64 %v1404 to i128
  %t66 = zext i64 %v1405 to i128
  %t67 = shl nuw i128 %t66, 64
  %v218054 = or disjoint i128 %t67, %t65
  %t69 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  store i128 %v218054, ptr %t69, align 16
  %v1860 = call i64 @mech_compact_writable(i64 %v0, i64 %v218049, i64 1, i64 %v1837, i64 1, i64 1, i64 %v1855)
  %t70.not = icmp eq i64 %v1860, 0
  br i1 %t70.not, label %block8252, label %block8249

edge59:                                           ; preds = %block8252
  %t76 = inttoptr i64 %v1863 to ptr
  %v1866 = load i64, ptr %t76, align 1
  %v1867 = add i64 %v1866, 1
  store i64 %v1867, ptr %t76, align 1
  br label %block8258

edge64:                                           ; preds = %block8258
  %v1872 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1405)
  %t80.not = icmp eq i64 %v1872, 0
  br i1 %t80.not, label %edge68, label %block8249

edge68:                                           ; preds = %edge47, %edge64, %block8258
  %v1839335349 = phi i64 [ %v1861, %block8258 ], [ %v1861, %edge64 ], [ %v1400, %edge47 ]
  %v1841336348 = phi i64 [ %v1862, %block8258 ], [ %v1862, %edge64 ], [ %v1402, %edge47 ]
  %v1844337347 = phi i64 [ %v1863, %block8258 ], [ %v1863, %edge64 ], [ %v1405, %edge47 ]
  %t75.not267339346 = phi i1 [ %t75.not267, %block8258 ], [ %t75.not267, %edge64 ], [ %t26.not, %edge47 ]
  %t83 = icmp eq i64 %v1395, 0
  br i1 %t83, label %edge70, label %block8408.preheader

block8408.preheader:                              ; preds = %edge68
  %v2380 = ptrtoint ptr %ss1 to i64
  %v218067 = ptrtoint ptr %ss0 to i64
  %t269 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  %t274 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  %t276 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %t277 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %t414 = icmp eq i64 %v1397, 1
  br label %block8408

edge70:                                           ; preds = %edge68
  br i1 %t23.not, label %edge81, label %edge74

edge74:                                           ; preds = %edge70
  %v1887 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t87.not = icmp eq i64 %v1887, 0
  br i1 %t87.not, label %edge81, label %edge75

edge75:                                           ; preds = %edge74
  %t88 = insertvalue { i64, i64 } poison, i64 %v1887, 0
  %t89 = insertvalue { i64, i64 } %t88, i64 19, 1
  br label %common.ret

edge81:                                           ; preds = %edge74, %edge70
  %t90 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v1401, i64 1)
  %v1917 = extractvalue { i64, i1 } %t90, 0
  %t91 = extractvalue { i64, i1 } %t90, 1
  br i1 %t91, label %block8282, label %edge82

edge82:                                           ; preds = %edge81
  %t94.not = icmp ugt i64 %v1917, %v1841336348
  br i1 %t94.not, label %edge85, label %block8286

edge85:                                           ; preds = %edge82
  %t96.not = icmp eq i64 %v1841336348, 0
  br i1 %t96.not, label %block8289, label %edge86

edge86:                                           ; preds = %edge85
  %4 = add i64 %v1841336348, -131073
  %v1927285 = icmp ult i64 %4, -126977
  br i1 %v1927285, label %edge89, label %edge98

edge89:                                           ; preds = %edge86
  %t100 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v1841336348, i64 %v1841336348)
  %v1928 = extractvalue { i64, i1 } %t100, 0
  %t101 = extractvalue { i64, i1 } %t100, 1
  br i1 %t101, label %block8282, label %block8289

edge98:                                           ; preds = %edge86
  %t108 = mul nuw nsw i64 %v1841336348, 3
  %t112 = add nuw nsw i64 %t108, 1
  %v1945 = lshr i64 %t112, 1
  br label %block8289

edge102:                                          ; preds = %block8286
  %t118 = inttoptr i64 %v1844337347 to ptr
  %v1953 = load i64, ptr %t118, align 1
  %t119 = icmp eq i64 %v1953, 1
  %t120 = icmp ule i64 %v1922, %v1841336348
  %v1956286 = and i1 %t120, %t119
  br i1 %v1956286, label %block8296, label %block8297

edge107:                                          ; preds = %block8297
  %v1963 = load i64, ptr %ss1, align 8
  %t137 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %v1964 = load i64, ptr %t137, align 8
  %t138 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %v1965 = load i64, ptr %t138, align 8
  br label %block8296

edge109:                                          ; preds = %block8296
  %t143 = inttoptr i64 %v1952 to ptr
  %v1972 = load i64, ptr %t143, align 1
  %v1973 = add i64 %v1972, 1
  store i64 %v1973, ptr %t143, align 1
  br label %block8301

edge114:                                          ; preds = %block8301
  %v1978 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1844337347)
  %t147.not = icmp eq i64 %v1978, 0
  br i1 %t147.not, label %edge118, label %block8282

edge118:                                          ; preds = %edge114, %block8301
  %t150 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v1917, i64 4)
  %v1987 = extractvalue { i64, i1 } %t150, 0
  %t151 = extractvalue { i64, i1 } %t150, 1
  br i1 %t151, label %block8306, label %edge121

edge121:                                          ; preds = %edge118
  %t154.not = icmp ugt i64 %v1987, %v1949
  br i1 %t154.not, label %edge124, label %block8311

edge124:                                          ; preds = %edge121
  %t156.not = icmp eq i64 %v1949, 0
  br i1 %t156.not, label %block8314, label %edge125

edge125:                                          ; preds = %edge124
  %5 = add i64 %v1949, -131073
  %v1997289 = icmp ult i64 %5, -126977
  br i1 %v1997289, label %edge128, label %edge137

edge128:                                          ; preds = %edge125
  %t160 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v1949, i64 %v1949)
  %v1998 = extractvalue { i64, i1 } %t160, 0
  %t161 = extractvalue { i64, i1 } %t160, 1
  br i1 %t161, label %block8306, label %block8314

edge137:                                          ; preds = %edge125
  %t168 = mul nuw nsw i64 %v1949, 3
  %t172 = add nuw nsw i64 %t168, 1
  %v2015 = lshr i64 %t172, 1
  br label %block8314

edge141:                                          ; preds = %block8311
  %t178 = inttoptr i64 %v1952 to ptr
  %v2023 = load i64, ptr %t178, align 1
  %t179 = icmp eq i64 %v2023, 1
  %t180 = icmp ule i64 %v1992, %v1949
  %v2026290 = and i1 %t180, %t179
  br i1 %v2026290, label %edge148, label %block8322

edge146:                                          ; preds = %block8322
  %v2033 = load i64, ptr %ss1, align 8
  %t197 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %v2034 = load i64, ptr %t197, align 8
  %t198 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %v2035 = load i64, ptr %t198, align 8
  br label %edge148

edge148:                                          ; preds = %edge141, %edge146
  %v2017 = phi i64 [ %v2033, %edge146 ], [ %v1947, %edge141 ]
  %v2019 = phi i64 [ %v2034, %edge146 ], [ %v1949, %edge141 ]
  %v2022 = phi i64 [ %v2035, %edge146 ], [ %v1952, %edge141 ]
  %v2040 = add i64 %v2017, %v1917
  %t199 = inttoptr i64 %v2040 to ptr
  store i8 0, ptr %t199, align 1
  %v2044 = add nuw i64 %v1917, 1
  %v2045 = add i64 %v2044, %v2017
  %t200 = inttoptr i64 %v2045 to ptr
  store i8 0, ptr %t200, align 1
  %v2049 = add nuw i64 %v1917, 2
  %v2050 = add i64 %v2049, %v2017
  %t201 = inttoptr i64 %v2050 to ptr
  store i8 -1, ptr %t201, align 1
  %v2054 = add nuw i64 %v1917, 3
  %v2055 = add i64 %v2054, %v2017
  %t202 = inttoptr i64 %v2055 to ptr
  store i8 -1, ptr %t202, align 1
  %t203 = icmp eq i64 %v1952, %v2022
  %t205.not291 = icmp eq i64 %v2022, 0
  %t205.not = or i1 %t203, %t205.not291
  br i1 %t205.not, label %block8342, label %edge160

edge160:                                          ; preds = %edge148
  %t206 = inttoptr i64 %v2022 to ptr
  %v2078 = load i64, ptr %t206, align 1
  %v2079 = add i64 %v2078, 1
  store i64 %v2079, ptr %t206, align 1
  br label %block8342

edge163:                                          ; preds = %block8342
  %t210 = inttoptr i64 %v1952 to ptr
  %v2082 = load i64, ptr %t210, align 1
  %v2083 = add i64 %v2082, -1
  store i64 %v2083, ptr %t210, align 1
  %t212.not = icmp eq i64 %v2083, 0
  br i1 %t212.not, label %edge166, label %edge15901

edge166:                                          ; preds = %edge163
  %v2084 = call i64 @mech_compact_free(i64 %v0, i64 %v1952)
  %t213.not = icmp eq i64 %v2084, 0
  br i1 %t213.not, label %edge15901, label %block8306

edge218:                                          ; preds = %block8408
  %t233.not = icmp ugt i64 %v2340, %v217689
  br i1 %t233.not, label %edge221, label %block8455

edge221:                                          ; preds = %edge218
  %t235.not = icmp eq i64 %v217689, 0
  br i1 %t235.not, label %block8458, label %edge222

edge222:                                          ; preds = %edge221
  %6 = add i64 %v217689, -131073
  %v2350269 = icmp ult i64 %6, -126977
  br i1 %v2350269, label %edge225, label %edge234

edge225:                                          ; preds = %edge222
  %t239 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v217689, i64 %v217689)
  %v2351 = extractvalue { i64, i1 } %t239, 0
  %t240 = extractvalue { i64, i1 } %t239, 1
  br i1 %t240, label %block8451, label %block8458

edge234:                                          ; preds = %edge222
  %t247 = mul nuw nsw i64 %v217689, 3
  %t251 = add nuw nsw i64 %t247, 1
  %v2368 = lshr i64 %t251, 1
  br label %block8458

edge238:                                          ; preds = %block8455
  %t257 = inttoptr i64 %v217692 to ptr
  %v2376 = load i64, ptr %t257, align 1
  %t258 = icmp eq i64 %v2376, 1
  %t259 = icmp ule i64 %v2345, %v217689
  %v2379270 = and i1 %t259, %t258
  br i1 %v2379270, label %block8465, label %block8466

edge243:                                          ; preds = %block8466
  %v2386 = load i64, ptr %ss1, align 8
  %v2387 = load i64, ptr %t276, align 8
  %v2388 = load i64, ptr %t277, align 8
  br label %block8465

edge245:                                          ; preds = %block8465
  %t282 = inttoptr i64 %v2375 to ptr
  %v2395 = load i64, ptr %t282, align 1
  %v2396 = add i64 %v2395, 1
  store i64 %v2396, ptr %t282, align 1
  br label %block8470

edge248:                                          ; preds = %block8470
  %t286 = inttoptr i64 %v217692 to ptr
  %v2399 = load i64, ptr %t286, align 1
  %v2400 = add i64 %v2399, -1
  store i64 %v2400, ptr %t286, align 1
  %t288.not = icmp eq i64 %v2400, 0
  br i1 %t288.not, label %edge251, label %edge262

edge251:                                          ; preds = %edge248
  %v2401 = call i64 @mech_compact_free(i64 %v0, i64 %v217692)
  %t289.not = icmp eq i64 %v2401, 0
  br i1 %t289.not, label %edge262, label %block8451

edge262:                                          ; preds = %edge251, %edge248, %block8470
  %t292 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2340, i64 2)
  %v2541 = extractvalue { i64, i1 } %t292, 0
  %t293 = extractvalue { i64, i1 } %t292, 1
  br i1 %t293, label %block8525, label %edge265

edge265:                                          ; preds = %edge262
  %t296.not = icmp ugt i64 %v2541, %v2372
  br i1 %t296.not, label %edge268, label %block8530

edge268:                                          ; preds = %edge265
  %t298.not = icmp eq i64 %v2372, 0
  br i1 %t298.not, label %block8533, label %edge269

edge269:                                          ; preds = %edge268
  %7 = add i64 %v2372, -131073
  %v2551273 = icmp ult i64 %7, -126977
  br i1 %v2551273, label %edge272, label %edge281

edge272:                                          ; preds = %edge269
  %t302 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2372, i64 %v2372)
  %v2552 = extractvalue { i64, i1 } %t302, 0
  %t303 = extractvalue { i64, i1 } %t302, 1
  br i1 %t303, label %block8525, label %block8533

edge281:                                          ; preds = %edge269
  %t310 = mul nuw nsw i64 %v2372, 3
  %t314 = add nuw nsw i64 %t310, 1
  %v2569 = lshr i64 %t314, 1
  br label %block8533

edge285:                                          ; preds = %block8530
  %t320 = inttoptr i64 %v2375 to ptr
  %v2577 = load i64, ptr %t320, align 1
  %t321 = icmp eq i64 %v2577, 1
  %t322 = icmp ule i64 %v2546, %v2372
  %v2580274 = and i1 %t322, %t321
  br i1 %v2580274, label %edge292, label %block8541

edge290:                                          ; preds = %block8541
  %v2587 = load i64, ptr %ss1, align 8
  %v2588 = load i64, ptr %t276, align 8
  %v2589 = load i64, ptr %t277, align 8
  br label %edge292

edge292:                                          ; preds = %edge285, %edge290
  %v2571 = phi i64 [ %v2587, %edge290 ], [ %v2370, %edge285 ]
  %v2573 = phi i64 [ %v2588, %edge290 ], [ %v2372, %edge285 ]
  %v2576 = phi i64 [ %v2589, %edge290 ], [ %v2375, %edge285 ]
  %v2592 = trunc i64 %spec.select293 to i8
  %v2594 = add i64 %v2571, %v2340
  %t341 = inttoptr i64 %v2594 to ptr
  store i8 %v2592, ptr %t341, align 1
  %v2596 = lshr i64 %spec.select293, 8
  %v2597 = trunc nuw i64 %v2596 to i8
  %v2598 = add nuw i64 %v2340, 1
  %v2599 = add i64 %v2598, %v2571
  %t342 = inttoptr i64 %v2599 to ptr
  store i8 %v2597, ptr %t342, align 1
  %t343 = icmp eq i64 %v2375, %v2576
  %t345.not275 = icmp eq i64 %v2576, 0
  %t345.not = or i1 %t343, %t345.not275
  br i1 %t345.not, label %block8561, label %edge302

edge302:                                          ; preds = %edge292
  %t346 = inttoptr i64 %v2576 to ptr
  %v2632 = load i64, ptr %t346, align 1
  %v2633 = add i64 %v2632, 1
  store i64 %v2633, ptr %t346, align 1
  br label %block8561

edge307:                                          ; preds = %block8561
  %v2638 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v2375)
  %t350.not = icmp eq i64 %v2638, 0
  br i1 %t350.not, label %edge311, label %block8525

edge311:                                          ; preds = %edge307, %block8561
  %v2645 = xor i64 %spec.select293, -1
  %t353 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2541, i64 2)
  %v2654 = extractvalue { i64, i1 } %t353, 0
  %t354 = extractvalue { i64, i1 } %t353, 1
  br i1 %t354, label %block8567, label %edge315

edge315:                                          ; preds = %edge311
  %t357.not = icmp ugt i64 %v2654, %v2573
  br i1 %t357.not, label %edge318, label %block8572

edge318:                                          ; preds = %edge315
  %t359.not = icmp eq i64 %v2573, 0
  br i1 %t359.not, label %block8575, label %edge319

edge319:                                          ; preds = %edge318
  %8 = add i64 %v2573, -131073
  %v2664277 = icmp ult i64 %8, -126977
  br i1 %v2664277, label %edge322, label %edge331

edge322:                                          ; preds = %edge319
  %t363 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2573, i64 %v2573)
  %v2665 = extractvalue { i64, i1 } %t363, 0
  %t364 = extractvalue { i64, i1 } %t363, 1
  br i1 %t364, label %block8567, label %block8575

edge331:                                          ; preds = %edge319
  %t371 = mul nuw nsw i64 %v2573, 3
  %t375 = add nuw nsw i64 %t371, 1
  %v2682 = lshr i64 %t375, 1
  br label %block8575

edge335:                                          ; preds = %block8572
  %t381 = inttoptr i64 %v2576 to ptr
  %v2690 = load i64, ptr %t381, align 1
  %t382 = icmp eq i64 %v2690, 1
  %t383 = icmp ule i64 %v2659, %v2573
  %v2693278 = and i1 %t383, %t382
  br i1 %v2693278, label %edge342, label %block8583

edge340:                                          ; preds = %block8583
  %v2700 = load i64, ptr %ss1, align 8
  %v2701 = load i64, ptr %t276, align 8
  %v2702 = load i64, ptr %t277, align 8
  br label %edge342

edge342:                                          ; preds = %edge335, %edge340
  %v2684 = phi i64 [ %v2700, %edge340 ], [ %v2571, %edge335 ]
  %v2686 = phi i64 [ %v2701, %edge340 ], [ %v2573, %edge335 ]
  %v2689 = phi i64 [ %v2702, %edge340 ], [ %v2576, %edge335 ]
  %v2705 = trunc i64 %v2645 to i8
  %v2707 = add i64 %v2684, %v2541
  %t402 = inttoptr i64 %v2707 to ptr
  store i8 %v2705, ptr %t402, align 1
  %v2709 = lshr i64 %v2645, 8
  %v2710 = trunc i64 %v2709 to i8
  %v2711 = add nuw i64 %v2541, 1
  %v2712 = add i64 %v2711, %v2684
  %t403 = inttoptr i64 %v2712 to ptr
  store i8 %v2710, ptr %t403, align 1
  %t404 = icmp eq i64 %v2576, %v2689
  %t406.not279 = icmp eq i64 %v2689, 0
  %t406.not = or i1 %t404, %t406.not279
  br i1 %t406.not, label %block8603, label %edge352

edge352:                                          ; preds = %edge342
  %t407 = inttoptr i64 %v2689 to ptr
  %v2745 = load i64, ptr %t407, align 1
  %v2746 = add i64 %v2745, 1
  store i64 %v2746, ptr %t407, align 1
  br label %block8603

edge357:                                          ; preds = %block8603
  %v2751 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v2576)
  %t411.not = icmp eq i64 %v2751, 0
  br i1 %t411.not, label %edge361, label %block8567

edge361:                                          ; preds = %edge357, %block8603
  br i1 %t414, label %edge363, label %block8608

edge363:                                          ; preds = %edge361
  %t416 = call i64 @llvm.umin.i64(i64 %v217693, i64 %v1395)
  %v2761 = sub nsw i64 %v1395, %t416
  %t417 = call i64 @llvm.umin.i64(i64 %spec.select293, i64 %v2761)
  %t418 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2654, i64 %t417)
  %v2763 = extractvalue { i64, i1 } %t418, 0
  %t419 = extractvalue { i64, i1 } %t418, 1
  br i1 %t419, label %block8608, label %edge365

edge365:                                          ; preds = %edge363
  %t422.not = icmp ugt i64 %v2763, %v2686
  br i1 %t422.not, label %edge368, label %block8613

edge368:                                          ; preds = %edge365
  %t424.not = icmp eq i64 %v2686, 0
  br i1 %t424.not, label %block8616, label %edge369

edge369:                                          ; preds = %edge368
  %9 = add i64 %v2686, -131073
  %v2773281 = icmp ult i64 %9, -126977
  br i1 %v2773281, label %edge372, label %edge381

edge372:                                          ; preds = %edge369
  %t428 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v2686, i64 %v2686)
  %v2774 = extractvalue { i64, i1 } %t428, 0
  %t429 = extractvalue { i64, i1 } %t428, 1
  br i1 %t429, label %block8608, label %block8616

edge381:                                          ; preds = %edge369
  %t436 = mul nuw nsw i64 %v2686, 3
  %t440 = add nuw nsw i64 %t436, 1
  %v2791 = lshr i64 %t440, 1
  br label %block8616

edge385:                                          ; preds = %block8613
  %t446 = inttoptr i64 %v2689 to ptr
  %v2799 = load i64, ptr %t446, align 1
  %t447 = icmp eq i64 %v2799, 1
  %t448 = icmp ule i64 %v2768, %v2686
  %v2802282 = and i1 %t448, %t447
  br i1 %v2802282, label %block8623, label %block8624

edge390:                                          ; preds = %block8624
  %v2809 = load i64, ptr %ss1, align 8
  %v2810 = load i64, ptr %t276, align 8
  %v2811 = load i64, ptr %t277, align 8
  br label %block8623

edge392:                                          ; preds = %block8623
  %v2815 = add i64 %t416, %v1394
  %v2813 = add i64 %v2793, %v2654
  %v2817 = call i64 @memmove(i64 %v2813, i64 %v2815, i64 %t417)
  br label %block8628

edge395:                                          ; preds = %block8628
  %t471 = inttoptr i64 %v2798 to ptr
  %v2820 = load i64, ptr %t471, align 1
  %v2821 = add i64 %v2820, 1
  store i64 %v2821, ptr %t471, align 1
  br label %block8630

edge400:                                          ; preds = %block8630
  %v2826 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v2689)
  %t475.not = icmp eq i64 %v2826, 0
  br i1 %t475.not, label %edge404, label %block8608

edge404:                                          ; preds = %edge400, %block8630
  %v2831 = add nuw nsw i64 %spec.select293, %v217693
  %t482 = icmp eq i64 %v2831, %v1395
  br i1 %t482, label %edge409, label %edge432

edge409:                                          ; preds = %edge404
  br i1 %t23.not, label %edge15901, label %edge411

edge411:                                          ; preds = %edge409
  %t486 = inttoptr i64 %v1399 to ptr
  %v2846 = load i64, ptr %t486, align 1
  %v2847 = add i64 %v2846, -1
  store i64 %v2847, ptr %t486, align 1
  %t488.not = icmp eq i64 %v2847, 0
  br i1 %t488.not, label %edge414, label %edge15901

edge414:                                          ; preds = %edge411
  %v2848 = call i64 @mech_compact_free(i64 %v0, i64 %v1399)
  %t489.not = icmp eq i64 %v2848, 0
  br i1 %t489.not, label %edge15901, label %edge415

edge415:                                          ; preds = %edge414
  %t490 = insertvalue { i64, i64 } poison, i64 %v2848, 0
  %t491 = insertvalue { i64, i64 } %t490, i64 86, 1
  br label %common.ret

edge432:                                          ; preds = %edge404
  %t216.not = icmp ult i64 %v1395, %v2831
  br i1 %t216.not, label %common.ret, label %block8408

edge462:                                          ; preds = %edge12
  %t523.not = icmp ugt i64 %v1395, -5000
  br i1 %t523.not, label %common.ret, label %edge477

edge477:                                          ; preds = %edge462
  %v3008 = add nuw i64 %v1395, 4999
  %v3017 = udiv i64 %v3008, 5000
  %10 = mul nuw nsw i64 %v3017, 5
  %v3162 = add i64 %10, %v1395
  %t540.not = icmp ult i64 %v3162, %10
  br i1 %t540.not, label %common.ret, label %edge482

edge482:                                          ; preds = %edge477
  %t544 = icmp eq i64 %v1403, 1
  br i1 %t544, label %edge483, label %block8822

edge483:                                          ; preds = %edge482
  %v3174 = tail call i64 @llvm.uadd.sat.i64(i64 %v1401, i64 %v3162)
  %t549 = icmp ugt i64 %v3174, %v1402
  %v3192 = ptrtoint ptr %ss1 to i64
  br i1 %t549, label %edge485, label %edge534

edge485:                                          ; preds = %edge483
  %v218097 = ptrtoint ptr %ss0 to i64
  %t556 = zext i64 %v1400 to i128
  %t557 = zext i64 %v1401 to i128
  %t558 = shl nuw i128 %t557, 64
  %v218098 = or disjoint i128 %t558, %t556
  store i128 %v218098, ptr %ss0, align 16
  %t560 = zext i64 %v1402 to i128
  %v218100 = or disjoint i128 %t560, 18446744073709551616
  %t564 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  store i128 %v218100, ptr %t564, align 16
  %t565 = zext i64 %v1404 to i128
  %t566 = zext i64 %v1405 to i128
  %t567 = shl nuw i128 %t566, 64
  %v218102 = or disjoint i128 %t567, %t565
  %t569 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  store i128 %v218102, ptr %t569, align 16
  %v3197 = call i64 @mech_compact_writable(i64 %v0, i64 %v218097, i64 1, i64 %v3174, i64 1, i64 1, i64 %v3192)
  %t570.not = icmp eq i64 %v3197, 0
  br i1 %t570.not, label %block8825, label %block8822

edge495:                                          ; preds = %block8825
  %t576 = inttoptr i64 %v3200 to ptr
  %v3203 = load i64, ptr %t576, align 1
  %v3204 = add i64 %v3203, 1
  store i64 %v3204, ptr %t576, align 1
  br label %block8831

edge498:                                          ; preds = %block8831
  %t580 = inttoptr i64 %v1405 to ptr
  %v3207 = load i64, ptr %t580, align 1
  %v3208 = add i64 %v3207, -1
  store i64 %v3208, ptr %t580, align 1
  %t582.not = icmp eq i64 %v3208, 0
  br i1 %t582.not, label %edge501, label %edge534

edge501:                                          ; preds = %edge498
  %v3209 = call i64 @mech_compact_free(i64 %v0, i64 %v1405)
  %t583.not = icmp eq i64 %v3209, 0
  br i1 %t583.not, label %edge534, label %block8822

edge514:                                          ; preds = %edge537
  %t591 = insertvalue { i64, i64 } poison, i64 %v3226, 0
  %t592 = insertvalue { i64, i64 } %t591, i64 120, 1
  br label %common.ret

edge517:                                          ; preds = %edge536, %edge537
  %spec.select5 = call i64 @llvm.umin.i64(i64 %v1395, i64 6000)
  %v3542 = mul nuw nsw i64 %spec.select5, 257
  %v3718 = call i64 @llvm.umin.i64(i64 %v3542, i64 1500513)
  %v3758 = call i64 @mech_compact_allocate(i64 %v0, i64 %v3718, i64 4, i64 4, i64 1, i64 %v3192)
  %t638.not = icmp eq i64 %v3758, 0
  br i1 %t638.not, label %edge592, label %block9091

edge534:                                          ; preds = %edge483, %block8831, %edge498, %edge501
  %v3176356367 = phi i64 [ %v3198, %edge501 ], [ %v3198, %block8831 ], [ %v3198, %edge498 ], [ %v1400, %edge483 ]
  %v3178357366 = phi i64 [ %v3199, %edge501 ], [ %v3199, %block8831 ], [ %v3199, %edge498 ], [ %v1402, %edge483 ]
  %v3181358365 = phi i64 [ %v3200, %edge501 ], [ %v3200, %block8831 ], [ %v3200, %edge498 ], [ %v1405, %edge483 ]
  %v3274 = call i64 @mech_compact_allocate(i64 %v0, i64 262144, i64 2, i64 2, i64 1, i64 %v3192)
  %t605.not = icmp eq i64 %v3274, 0
  br i1 %t605.not, label %edge536, label %block8855

edge536:                                          ; preds = %edge534
  %v3277 = load i64, ptr %ss1, align 8
  %t606 = getelementptr inbounds nuw i8, ptr %ss1, i64 16
  %v3278 = load i64, ptr %t606, align 8
  %t607 = getelementptr inbounds nuw i8, ptr %ss1, i64 40
  %v3279 = load i64, ptr %t607, align 8
  %cond322 = icmp eq i64 %v3279, 0
  br i1 %cond322, label %edge517, label %edge537

edge537:                                          ; preds = %edge536
  %t609 = inttoptr i64 %v3279 to ptr
  %v3280 = load i64, ptr %t609, align 1
  %v3222 = add i64 %v3280, 2
  store i64 %v3222, ptr %t609, align 1
  %v3226 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v3279)
  %t590.not = icmp eq i64 %v3226, 0
  br i1 %t590.not, label %edge517, label %edge514

edge570:                                          ; preds = %edge593
  %t624 = insertvalue { i64, i64 } poison, i64 %v3709, 0
  %t625 = insertvalue { i64, i64 } %t624, i64 152, 1
  br label %common.ret

edge573:                                          ; preds = %edge592, %edge593
  %spec.select7 = call i64 @llvm.umin.i64(i64 %v1395, i64 304999)
  %v4037 = add nuw nsw i64 %spec.select7, 258
  %v4046 = call i64 @mech_compact_allocate(i64 %v0, i64 %v4037, i64 8, i64 8, i64 0, i64 %v3192)
  %t649.not = icmp eq i64 %v4046, 0
  br i1 %t649.not, label %edge610, label %edge609

edge592:                                          ; preds = %edge517
  %v3761 = load i64, ptr %ss1, align 8
  %v3762 = load i64, ptr %t606, align 8
  %v3763 = load i64, ptr %t607, align 8
  %cond323 = icmp eq i64 %v3763, 0
  br i1 %cond323, label %edge573, label %edge593

edge593:                                          ; preds = %edge592
  %t642 = inttoptr i64 %v3763 to ptr
  %v3764 = load i64, ptr %t642, align 1
  %v3705 = add i64 %v3764, 2
  store i64 %v3705, ptr %t642, align 1
  %v3709 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v3763)
  %t623.not = icmp eq i64 %v3709, 0
  br i1 %t623.not, label %edge573, label %edge570

edge609:                                          ; preds = %edge573
  %t655 = insertvalue { i64, i64 } poison, i64 %v4046, 0
  %t656 = insertvalue { i64, i64 } %t655, i64 174, 1
  br label %common.ret

edge610:                                          ; preds = %edge573
  %v4049 = load i64, ptr %ss1, align 8
  %v4050 = load i64, ptr %t606, align 8
  %v4051 = load i64, ptr %t607, align 8
  %t652.not = icmp eq i64 %v4051, 0
  br i1 %t652.not, label %edge642, label %edge611

edge611:                                          ; preds = %edge610
  %t653 = inttoptr i64 %v4051 to ptr
  %v4052 = load i64, ptr %t653, align 1
  %v4053 = add i64 %v4052, 1
  store i64 %v4053, ptr %t653, align 1
  br label %edge642

edge621:                                          ; preds = %edge645
  %v4094 = call i64 @mech_compact_free(i64 %v0, i64 %v4151)
  %t662.not = icmp eq i64 %v4094, 0
  br i1 %t662.not, label %edge678, label %edge622

edge622:                                          ; preds = %edge621
  %t663 = insertvalue { i64, i64 } poison, i64 %v4094, 0
  %t664 = insertvalue { i64, i64 } %t663, i64 176, 1
  br label %common.ret

edge642:                                          ; preds = %edge611, %edge610
  %v4146 = call i64 @mech_compact_allocate(i64 %v0, i64 547, i64 4, i64 4, i64 1, i64 %v3192)
  %t677.not = icmp eq i64 %v4146, 0
  br i1 %t677.not, label %edge644, label %block9245

edge644:                                          ; preds = %edge642
  %v4149 = load i64, ptr %ss1, align 8
  %v4150 = load i64, ptr %t606, align 8
  %v4151 = load i64, ptr %t607, align 8
  %t680.not = icmp eq i64 %v4151, 0
  br i1 %t680.not, label %edge678, label %edge645

edge645:                                          ; preds = %edge644
  %t681 = inttoptr i64 %v4151 to ptr
  %v4152 = load i64, ptr %t681, align 1
  %v4153 = add i64 %v4152, 1
  store i64 %v4153, ptr %t681, align 1
  %t661.not = icmp eq i64 %v4153, 0
  br i1 %t661.not, label %edge621, label %edge678

edge657:                                          ; preds = %edge681
  %v4317 = call i64 @mech_compact_free(i64 %v0, i64 %v4370)
  %t690.not = icmp eq i64 %v4317, 0
  br i1 %t690.not, label %edge714, label %edge658

edge658:                                          ; preds = %edge657
  %t691 = insertvalue { i64, i64 } poison, i64 %v4317, 0
  %t692 = insertvalue { i64, i64 } %t691, i64 189, 1
  br label %common.ret

edge678:                                          ; preds = %edge644, %edge645, %edge621
  %v4365 = call i64 @mech_compact_allocate(i64 %v0, i64 547, i64 4, i64 4, i64 1, i64 %v3192)
  %t705.not = icmp eq i64 %v4365, 0
  br i1 %t705.not, label %edge680, label %block9300

edge680:                                          ; preds = %edge678
  %v4368 = load i64, ptr %ss1, align 8
  %v4369 = load i64, ptr %t606, align 8
  %v4370 = load i64, ptr %t607, align 8
  %t708.not = icmp eq i64 %v4370, 0
  br i1 %t708.not, label %edge714, label %edge681

edge681:                                          ; preds = %edge680
  %t709 = inttoptr i64 %v4370 to ptr
  %v4371 = load i64, ptr %t709, align 1
  %v4372 = add i64 %v4371, 1
  store i64 %v4372, ptr %t709, align 1
  %t689.not = icmp eq i64 %v4372, 0
  br i1 %t689.not, label %edge657, label %edge714

edge693:                                          ; preds = %edge717
  %v4548 = call i64 @mech_compact_free(i64 %v0, i64 %v4609)
  %t718.not = icmp eq i64 %v4548, 0
  br i1 %t718.not, label %edge750, label %edge694

edge694:                                          ; preds = %edge693
  %t719 = insertvalue { i64, i64 } poison, i64 %v4548, 0
  %t720 = insertvalue { i64, i64 } %t719, i64 202, 1
  br label %common.ret

edge714:                                          ; preds = %edge680, %edge681, %edge657
  %v4604 = call i64 @mech_compact_allocate(i64 %v0, i64 259, i64 4, i64 4, i64 1, i64 %v3192)
  %t733.not = icmp eq i64 %v4604, 0
  br i1 %t733.not, label %edge716, label %block9355

edge716:                                          ; preds = %edge714
  %v4607 = load i64, ptr %ss1, align 8
  %v4608 = load i64, ptr %t606, align 8
  %v4609 = load i64, ptr %t607, align 8
  %t736.not = icmp eq i64 %v4609, 0
  br i1 %t736.not, label %edge750, label %edge717

edge717:                                          ; preds = %edge716
  %t737 = inttoptr i64 %v4609 to ptr
  %v4610 = load i64, ptr %t737, align 1
  %v4611 = add i64 %v4610, 1
  store i64 %v4611, ptr %t737, align 1
  %t717.not = icmp eq i64 %v4611, 0
  br i1 %t717.not, label %edge693, label %edge750

edge729:                                          ; preds = %edge753
  %v4799 = call i64 @mech_compact_free(i64 %v0, i64 %v4852)
  %t746.not = icmp eq i64 %v4799, 0
  br i1 %t746.not, label %edge786, label %edge730

edge730:                                          ; preds = %edge729
  %t747 = insertvalue { i64, i64 } poison, i64 %v4799, 0
  %t748 = insertvalue { i64, i64 } %t747, i64 215, 1
  br label %common.ret

edge750:                                          ; preds = %edge716, %edge717, %edge693
  %v4847 = call i64 @mech_compact_allocate(i64 %v0, i64 259, i64 4, i64 4, i64 1, i64 %v3192)
  %t761.not = icmp eq i64 %v4847, 0
  br i1 %t761.not, label %edge752, label %block9410

edge752:                                          ; preds = %edge750
  %v4850 = load i64, ptr %ss1, align 8
  %v4851 = load i64, ptr %t606, align 8
  %v4852 = load i64, ptr %t607, align 8
  %t764.not = icmp eq i64 %v4852, 0
  br i1 %t764.not, label %edge786, label %edge753

edge753:                                          ; preds = %edge752
  %t765 = inttoptr i64 %v4852 to ptr
  %v4853 = load i64, ptr %t765, align 1
  %v4854 = add i64 %v4853, 1
  store i64 %v4854, ptr %t765, align 1
  %t745.not = icmp eq i64 %v4854, 0
  br i1 %t745.not, label %edge729, label %edge786

edge765:                                          ; preds = %edge789
  %v5054 = call i64 @mech_compact_free(i64 %v0, i64 %v5119)
  %t774.not = icmp eq i64 %v5054, 0
  br i1 %t774.not, label %edge822, label %edge766

edge766:                                          ; preds = %edge765
  %t775 = insertvalue { i64, i64 } poison, i64 %v5054, 0
  %t776 = insertvalue { i64, i64 } %t775, i64 228, 1
  br label %common.ret

edge786:                                          ; preds = %edge752, %edge753, %edge729
  %v5114 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3192)
  %t789.not = icmp eq i64 %v5114, 0
  br i1 %t789.not, label %edge788, label %block9465

edge788:                                          ; preds = %edge786
  %v5117 = load i64, ptr %ss1, align 8
  %v5119 = load i64, ptr %t607, align 8
  %t791.not = icmp eq i64 %v5119, 0
  br i1 %t791.not, label %edge822, label %edge789

edge789:                                          ; preds = %edge788
  %t792 = inttoptr i64 %v5119 to ptr
  %v5120 = load i64, ptr %t792, align 1
  %v5121 = add i64 %v5120, 1
  store i64 %v5121, ptr %t792, align 1
  %t773.not = icmp eq i64 %v5121, 0
  br i1 %t773.not, label %edge765, label %edge822

edge801:                                          ; preds = %edge825
  %v5333 = call i64 @mech_compact_free(i64 %v0, i64 %v5386)
  %t801.not = icmp eq i64 %v5333, 0
  br i1 %t801.not, label %edge859, label %edge802

edge802:                                          ; preds = %edge801
  %t802 = insertvalue { i64, i64 } poison, i64 %v5333, 0
  %t803 = insertvalue { i64, i64 } %t802, i64 241, 1
  br label %common.ret

edge822:                                          ; preds = %edge788, %edge789, %edge765
  %v5381 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3192)
  %t816.not = icmp eq i64 %v5381, 0
  br i1 %t816.not, label %edge824, label %block9520

edge824:                                          ; preds = %edge822
  %v5384 = load i64, ptr %ss1, align 8
  %v5385 = load i64, ptr %t606, align 8
  %v5386 = load i64, ptr %t607, align 8
  %t819.not = icmp eq i64 %v5386, 0
  br i1 %t819.not, label %edge859, label %edge825

edge825:                                          ; preds = %edge824
  %t820 = inttoptr i64 %v5386 to ptr
  %v5387 = load i64, ptr %t820, align 1
  %v5388 = add i64 %v5387, 1
  store i64 %v5388, ptr %t820, align 1
  %t800.not = icmp eq i64 %v5388, 0
  br i1 %t800.not, label %edge801, label %edge859

edge839:                                          ; preds = %edge862
  %t829 = insertvalue { i64, i64 } poison, i64 %v5612, 0
  %t830 = insertvalue { i64, i64 } %t829, i64 254, 1
  br label %common.ret

edge842:                                          ; preds = %edge861, %edge862
  %spec.select9 = call i64 @llvm.umin.i64(i64 %v1395, i64 32768)
  %t910 = inttoptr i64 %v1399 to ptr
  %t972 = icmp eq i64 %v1397, 1
  %t973 = icmp eq i64 %v1398, 1
  %v894546 = or i1 %t972, %t973
  %t975 = icmp eq i64 %v1397, 0
  %v218115 = ptrtoint ptr %ss0 to i64
  %t1002 = getelementptr inbounds nuw i8, ptr %ss0, i64 16
  %t1007 = getelementptr inbounds nuw i8, ptr %ss0, i64 32
  %v218103 = ptrtoint ptr %ss2 to i64
  %t926 = getelementptr inbounds nuw i8, ptr %ss2, i64 16
  %t931 = getelementptr inbounds nuw i8, ptr %ss2, i64 32
  %t933 = zext i64 %v1394 to i128
  %t934 = shl nuw i128 %t933, 64
  %v218110 = or disjoint i128 %t934, 256
  %t936 = getelementptr inbounds nuw i8, ptr %ss2, i64 48
  %t937 = zext i64 %v1395 to i128
  %t938 = zext i64 %v1397 to i128
  %t939 = shl nuw i128 %t938, 64
  %v218112 = or disjoint i128 %t939, %t937
  %t941 = getelementptr inbounds nuw i8, ptr %ss2, i64 64
  %t942 = zext i64 %v1398 to i128
  %t946 = getelementptr inbounds nuw i8, ptr %ss2, i64 80
  %t947 = getelementptr inbounds nuw i8, ptr %ss2, i64 96
  %v8479 = ptrtoint ptr %ss3 to i64
  %t1068 = icmp ne i64 %v1397, 1
  %t1069 = icmp ne i64 %v1398, 1
  %v1061851.not1217 = and i1 %t1068, %t1069
  %t1152 = getelementptr inbounds nuw i8, ptr %ss2, i64 112
  %t1157 = getelementptr inbounds nuw i8, ptr %ss2, i64 128
  %t1162 = getelementptr inbounds nuw i8, ptr %ss2, i64 144
  %t1163 = zext nneg i64 %v3718 to i128
  %v218142 = or disjoint i128 %t1163, 18446744073709551616
  %t1167 = getelementptr inbounds nuw i8, ptr %ss2, i64 160
  %v218144 = or disjoint i128 %t934, %t1163
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
  %v218227 = ptrtoint ptr %ss4 to i64
  %t1383 = getelementptr inbounds nuw i8, ptr %ss4, i64 16
  %v14023 = add i64 %v1395, -4
  %t1719 = shl nuw nsw i128 %t1163, 64
  %v23157 = add i64 %v1395, -8
  %t1326 = getelementptr inbounds nuw i8, ptr %ss3, i64 248
  %t1329 = getelementptr inbounds nuw i8, ptr %ss3, i64 264
  %t1332 = getelementptr inbounds nuw i8, ptr %ss3, i64 280
  %t1335 = getelementptr inbounds nuw i8, ptr %ss3, i64 296
  %t1337 = getelementptr inbounds nuw i8, ptr %ss3, i64 320
  %v218456 = or disjoint i128 %t1719, 1
  %t4193 = zext i64 ptrtoint (ptr @mech_compact_table_18 to i64) to i128
  %v218462 = or disjoint i128 %t4193, 4722366482869645213696
  %t4225 = getelementptr inbounds nuw i8, ptr %ss3, i64 48
  %t4228 = getelementptr inbounds nuw i8, ptr %ss3, i64 64
  %t4231 = getelementptr inbounds nuw i8, ptr %ss3, i64 80
  %t4516 = zext i64 ptrtoint (ptr @mech_compact_table_7 to i64) to i128
  %t4517 = shl nuw i128 %t4516, 64
  %v218496 = or disjoint i128 %t4517, 320
  %t4527 = shl nuw i128 %t4193, 64
  %v218500 = or disjoint i128 %t4527, 259
  %t6121 = zext i64 ptrtoint (ptr @mech_compact_table_19 to i64) to i128
  %v218684 = or disjoint i128 %t6121, 4722366482869645213696
  %t6991 = zext i64 ptrtoint (ptr @mech_compact_table_11 to i64) to i128
  %t6992 = shl nuw i128 %t6991, 64
  %v218750 = or disjoint i128 %t6991, 10090369008319124733952
  br label %block9980

edge859:                                          ; preds = %edge824, %edge825, %edge801
  %v5660 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3192)
  %t843.not = icmp eq i64 %v5660, 0
  br i1 %t843.not, label %edge861, label %block9575

edge861:                                          ; preds = %edge859
  %v5663 = load i64, ptr %ss1, align 8
  %v5664 = load i64, ptr %t606, align 8
  %v5665 = load i64, ptr %t607, align 8
  %cond324 = icmp eq i64 %v5665, 0
  br i1 %cond324, label %edge842, label %edge862

edge862:                                          ; preds = %edge861
  %t847 = inttoptr i64 %v5665 to ptr
  %v5666 = load i64, ptr %t847, align 1
  %v5608 = add i64 %v5666, 2
  store i64 %v5608, ptr %t847, align 1
  %v5612 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v5665)
  %t828.not = icmp eq i64 %v5612, 0
  br i1 %t828.not, label %edge842, label %edge839

edge936:                                          ; preds = %edge940, %block9980
  %v7669 = phi i64 [ %v1395, %block9980 ], [ %v7455, %edge940 ]
  br i1 %v217765, label %edge948, label %edge952

edge940:                                          ; preds = %block9980
  %v7455 = add nuw i64 %v217729, 300000
  %t861.not = icmp ugt i64 %v217729, -300001
  br i1 %t861.not, label %common.ret, label %edge936

edge948:                                          ; preds = %edge953, %edge1200, %edge1203, %edge936
  %v10550 = phi i64 [ 259, %edge936 ], [ 3, %edge953 ], [ %v10207, %edge1203 ], [ 3, %edge1200 ]
  br i1 %t23.not, label %block475.outer.preheader, label %edge1217

edge952:                                          ; preds = %edge936
  %t868.not = icmp ult i64 %v7669, %v217729
  br i1 %t868.not, label %common.ret, label %edge953

edge953:                                          ; preds = %edge952
  %v7672 = sub nuw i64 %v7669, %v217729
  %t872 = icmp ult i64 %v7672, 512
  br i1 %t872, label %edge948, label %edge997

edge977:                                          ; preds = %edge1000
  %t884 = insertvalue { i64, i64 } poison, i64 %v8020, 0
  %t885 = insertvalue { i64, i64 } %t884, i64 339, 1
  br label %common.ret

edge980:                                          ; preds = %edge999, %edge1000
  %spec.select11 = call i64 @llvm.umin.i64(i64 %v7672, i64 4096)
  br i1 %t23.not, label %block364.preheader, label %edge1015

edge997:                                          ; preds = %edge953
  %v8088 = call i64 @mech_compact_allocate(i64 %v0, i64 256, i64 1, i64 1, i64 1, i64 %v3192)
  %t898.not = icmp eq i64 %v8088, 0
  br i1 %t898.not, label %edge999, label %block10380

edge999:                                          ; preds = %edge997
  %v8091 = load i64, ptr %ss1, align 8
  %v8092 = load i64, ptr %t606, align 8
  %v8093 = load i64, ptr %t607, align 8
  %t901.not = icmp eq i64 %v8093, 0
  br i1 %t901.not, label %edge980, label %edge1000

edge1000:                                         ; preds = %edge999
  %t902 = inttoptr i64 %v8093 to ptr
  %v8094 = load i64, ptr %t902, align 1
  %v8016 = add i64 %v8094, 2
  store i64 %v8016, ptr %t902, align 1
  %v8020 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v8093)
  %t883.not = icmp eq i64 %v8020, 0
  br i1 %t883.not, label %edge980, label %edge977

edge1015:                                         ; preds = %edge980
  %v8454 = load i64, ptr %t910, align 1
  %v8455 = add i64 %v8454, 1
  store i64 %v8455, ptr %t910, align 1
  br label %block364.preheader

block364.preheader:                               ; preds = %edge980, %edge1015
  br label %block364

edge1023:                                         ; preds = %block364
  %t913 = inttoptr i64 %v8474 to ptr
  %v8475 = load i64, ptr %t913, align 1
  %t914 = icmp eq i64 %v8475, 1
  %t916 = icmp ugt i64 %v8508, 255
  %or.cond295 = select i1 %t914, i1 %t916, i1 false
  br i1 %or.cond295, label %edge1027, label %edge1057

edge1027:                                         ; preds = %edge1023
  %t918 = zext i64 %v217729 to i128
  %t919 = zext nneg i64 %spec.select11 to i128
  %t920 = shl nuw nsw i128 %t919, 64
  %v218104 = or disjoint i128 %t920, %t918
  store i128 %v218104, ptr %ss2, align 16
  %t922 = zext nneg i64 %v8511 to i128
  %t923 = zext i64 %v8512 to i128
  %t924 = shl nuw i128 %t923, 64
  %v218106 = or disjoint i128 %t924, %t922
  store i128 %v218106, ptr %t926, align 16
  store i128 18446744073709551872, ptr %t931, align 16
  store i128 %v218110, ptr %t936, align 16
  store i128 %v218112, ptr %t941, align 16
  %t943 = zext i64 %v8474 to i128
  %t944 = shl nuw i128 %t943, 64
  %v218114 = or disjoint i128 %t944, %t942
  store i128 %v218114, ptr %t946, align 16
  store i64 %v8508, ptr %t947, align 16
  br i1 %v894546, label %edge1030, label %edge1031

edge1030:                                         ; preds = %edge1027
  %t951 = call fastcc { i64, i64 } @mech_compact_numeric_362_vector(i64 %v218103, i64 %v8479)
  br label %block10568

edge1031:                                         ; preds = %edge1027
  %t952 = call fastcc { i64, i64 } @mech_compact_numeric_362(i64 %v218103, i64 %v8479)
  br label %block10568

edge1034:                                         ; preds = %block10568
  %v8481 = extractvalue { i64, i64 } %t951.pn, 1
  %t954 = icmp eq i64 %v8480, 103169
  %v8494 = select i1 %t954, i64 103425, i64 %v8480
  %t956 = insertvalue { i64, i64 } poison, i64 %v8494, 0
  %t957 = insertvalue { i64, i64 } %t956, i64 %v8481, 1
  br label %common.ret

edge1036:                                         ; preds = %block10568
  br i1 %t23.not, label %edge1147, label %edge1039

edge1039:                                         ; preds = %edge1036
  %v8503 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t959.not = icmp eq i64 %v8503, 0
  br i1 %t959.not, label %edge1147, label %edge1040

edge1040:                                         ; preds = %edge1039
  %t960 = insertvalue { i64, i64 } poison, i64 %v8503, 0
  %t961 = insertvalue { i64, i64 } %t960, i64 362, 1
  br label %common.ret

edge1057:                                         ; preds = %block364, %edge1023
  %exitcond.not = icmp eq i64 %v8511, %spec.select11
  br i1 %exitcond.not, label %edge1117, label %edge1063

edge1063:                                         ; preds = %edge1057
  %v8745 = add i64 %v8511, %v217729
  %t965.not = icmp ult i64 %v8745, %v217729
  br i1 %t965.not, label %common.ret, label %edge1067

edge1067:                                         ; preds = %edge1063
  %t969 = icmp ult i64 %v8745, %v1395
  br i1 %t969, label %edge1073, label %edge1088

edge1073:                                         ; preds = %edge1067
  br i1 %v894546, label %block10874, label %edge1078

edge1078:                                         ; preds = %edge1073
  br i1 %t975, label %trap976, label %ok976

edge1088:                                         ; preds = %edge1067, %block10874
  %v8977 = phi i64 [ %v8954, %block10874 ], [ 0, %edge1067 ]
  %v8989 = add i64 %v8977, %v8512
  %t985 = inttoptr i64 %v8989 to ptr
  %t986 = load i8, ptr %t985, align 1
  %t987 = icmp eq i8 %t986, 1
  br i1 %t987, label %block10885, label %edge1091

edge1091:                                         ; preds = %edge1088
  br i1 %t912.not, label %block10887, label %edge1092

edge1092:                                         ; preds = %edge1091
  %t990 = inttoptr i64 %v8474 to ptr
  %v9006 = load i64, ptr %t990, align 1
  %t991 = icmp eq i64 %v9006, 1
  br i1 %t991, label %block10886, label %block10887

edge1097:                                         ; preds = %block10887
  %v9016 = load i64, ptr %ss1, align 8
  %v9017 = load i64, ptr %t606, align 8
  %v9018 = load i64, ptr %t607, align 8
  %.pre20755 = add i64 %v9016, %v8977
  %.pre20756 = inttoptr i64 %.pre20755 to ptr
  br label %block10886

edge1100:                                         ; preds = %block10885
  %t1015 = inttoptr i64 %v8999 to ptr
  %v9023 = load i64, ptr %t1015, align 1
  %v9024 = add i64 %v9023, 1
  store i64 %v9024, ptr %t1015, align 1
  br label %block10891

edge1105:                                         ; preds = %block10891
  %v9029 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v8474)
  %t1019.not = icmp eq i64 %v9029, 0
  br i1 %t1019.not, label %edge1128, label %block10882

edge1117:                                         ; preds = %edge1057
  br i1 %t23.not, label %edge1147, label %edge1118

edge1118:                                         ; preds = %edge1117
  %v9085 = load i64, ptr %t910, align 1
  %v9086 = add i64 %v9085, -1
  store i64 %v9086, ptr %t910, align 1
  %t1028.not = icmp eq i64 %v9086, 0
  br i1 %t1028.not, label %edge1121, label %edge1147

edge1121:                                         ; preds = %edge1118
  %v9087 = call i64 @mech_compact_free(i64 %v0, i64 %v1399)
  %t1029.not = icmp eq i64 %v9087, 0
  br i1 %t1029.not, label %edge1147, label %edge1122

edge1122:                                         ; preds = %edge1121
  %t1030 = insertvalue { i64, i64 } poison, i64 %v9087, 0
  %t1031 = insertvalue { i64, i64 } %t1030, i64 399, 1
  br label %common.ret

edge1128:                                         ; preds = %block10891, %edge1105
  %v9254 = add nuw nsw i64 %v8511, 1
  br label %block364

edge1147:                                         ; preds = %edge1039, %edge1036, %edge1121, %edge1118, %edge1117
  %t912.not21409 = phi i1 [ false, %edge1039 ], [ false, %edge1036 ], [ %t912.not, %edge1121 ], [ %t912.not, %edge1118 ], [ %t912.not, %edge1117 ]
  br label %edge1179

edge1160:                                         ; preds = %edge1196
  br i1 %t912.not21409, label %edge1200, label %edge1161

edge1161:                                         ; preds = %edge1160
  %t1037 = inttoptr i64 %v8474 to ptr
  %v9386 = load i64, ptr %t1037, align 1
  %v9387 = add i64 %v9386, -1
  store i64 %v9387, ptr %t1037, align 1
  %t1039.not = icmp eq i64 %v9387, 0
  br i1 %t1039.not, label %edge1164, label %edge1200

edge1164:                                         ; preds = %edge1161
  %v9388 = call i64 @mech_compact_free(i64 %v0, i64 %v8474)
  %t1040.not = icmp eq i64 %v9388, 0
  br i1 %t1040.not, label %edge1200, label %edge1165

edge1165:                                         ; preds = %edge1164
  %t1041 = insertvalue { i64, i64 } poison, i64 %v9388, 0
  %t1042 = insertvalue { i64, i64 } %t1041, i64 423, 1
  br label %common.ret

edge1179:                                         ; preds = %edge1147, %edge1196
  %v217289 = phi i64 [ 0, %edge1147 ], [ %v9551, %edge1196 ]
  %v217290 = phi i64 [ 0, %edge1147 ], [ %v9767, %edge1196 ]
  %v9742 = add i64 %v217289, %v8512
  %t1050 = inttoptr i64 %v9742 to ptr
  %t1051 = load i8, ptr %t1050, align 1
  %v9743 = zext i8 %t1051 to i64
  %v9767 = add i64 %v217290, %v9743
  %t1052.not = icmp ult i64 %v9767, %v217290
  br i1 %t1052.not, label %common.ret, label %edge1196

edge1196:                                         ; preds = %edge1179
  %v9551 = add nuw nsw i64 %v217289, 1
  %exitcond20573.not = icmp eq i64 %v9551, 256
  br i1 %exitcond20573.not, label %edge1160, label %edge1179

edge1200:                                         ; preds = %edge1160, %edge1161, %edge1164
  %t1056 = icmp ugt i64 %v9767, 79
  br i1 %t1056, label %edge948, label %edge1203

edge1203:                                         ; preds = %edge1200
  %v10206 = add i64 %v9767, ptrtoint (ptr @mech_compact_table_6 to i64)
  %t1061 = inttoptr i64 %v10206 to ptr
  %t1062 = load i8, ptr %t1061, align 1
  %v10207 = zext i8 %t1062 to i64
  br label %edge948

edge1217:                                         ; preds = %edge948
  %v10551 = load i64, ptr %t910, align 1
  %v10552 = add i64 %v10551, 1
  store i64 %v10552, ptr %t910, align 1
  br label %block475.outer.preheader

block475.outer.preheader:                         ; preds = %edge948, %edge1217
  br label %block475.outer.outer

block475.outer.outer:                             ; preds = %block2378, %block475.outer.preheader
  %v10787.ph.ph = phi i64 [ %v217716, %block475.outer.preheader ], [ %v36628651, %block2378 ]
  %v10788.ph.ph = phi i64 [ %v217719, %block475.outer.preheader ], [ %v4033311241137, %block2378 ]
  %v10789.ph.ph = phi i64 [ %v217748, %block475.outer.preheader ], [ %v46663, %block2378 ]
  %v10790.ph.ph = phi i64 [ %v217752, %block475.outer.preheader ], [ %v46675, %block2378 ]
  %v10791.ph.ph = phi i64 [ %v217759, %block475.outer.preheader ], [ %v45196, %block2378 ]
  %v10792.ph.ph = phi i64 [ %v217763, %block475.outer.preheader ], [ %v45181, %block2378 ]
  %v10794.ph.ph = phi i64 [ %v217715, %block475.outer.preheader ], [ %v36625650, %block2378 ]
  %v10796.ph.ph = phi i64 [ %v217718, %block475.outer.preheader ], [ %v4033011221138, %block2378 ]
  %v10798.ph.ph = phi i64 [ %v217746, %block475.outer.preheader ], [ %v46660, %block2378 ]
  %v10800.ph.ph = phi i64 [ %v217750, %block475.outer.preheader ], [ %v46672, %block2378 ]
  %v10802.ph.ph = phi i64 [ %v217757, %block475.outer.preheader ], [ %v45193, %block2378 ]
  %v10804.ph.ph = phi i64 [ %v217761, %block475.outer.preheader ], [ %v45178, %block2378 ]
  %v10807.ph.ph = phi i64 [ %v217726, %block475.outer.preheader ], [ %v36647656, %block2378 ]
  %v10808.ph.ph = phi i64 [ %v217727, %block475.outer.preheader ], [ %v36650657, %block2378 ]
  %v10809.ph.ph = phi i64 [ %v217728, %block475.outer.preheader ], [ %v41283, %block2378 ]
  %v10810.ph.ph = phi i64 [ %v217730, %block475.outer.preheader ], [ %spec.select1, %block2378 ]
  %v10811.ph.ph = phi i64 [ %v217731, %block475.outer.preheader ], [ %v41333, %block2378 ]
  %v10812.ph.ph = phi i64 [ %v217732, %block475.outer.preheader ], [ %v40314, %block2378 ]
  %v10813.ph.ph = phi i64 [ 0, %block475.outer.preheader ], [ %v46280, %block2378 ]
  %v10814.ph.ph = phi i64 [ %v217764, %block475.outer.preheader ], [ 0, %block2378 ]
  %v10815.ph.ph = phi i64 [ %v217728, %block475.outer.preheader ], [ %v40057, %block2378 ]
  %v10817.ph.ph = phi i64 [ %v217729, %block475.outer.preheader ], [ %v217320, %block2378 ]
  %v10818.ph.ph = phi i64 [ %v7669, %block475.outer.preheader ], [ %v217319, %block2378 ]
  %v10819.ph.ph = phi i64 [ %v10550, %block475.outer.preheader ], [ %v217318, %block2378 ]
  %v10820.ph.ph = phi i64 [ 0, %block475.outer.preheader ], [ %v41283, %block2378 ]
  %v10821.ph.ph = phi i64 [ 0, %block475.outer.preheader ], [ 1, %block2378 ]
  %v10822.ph.ph = phi i64 [ %v217713, %block475.outer.preheader ], [ %v36623649, %block2378 ]
  %v10823.ph.ph = phi i64 [ %v217717, %block475.outer.preheader ], [ %v4032811191139, %block2378 ]
  %v10826.ph.ph = phi i64 [ %v217745, %block475.outer.preheader ], [ %v46658, %block2378 ]
  %v10827.ph.ph = phi i64 [ %v217749, %block475.outer.preheader ], [ %v46670, %block2378 ]
  %v10828.ph.ph = phi i64 [ %v217756, %block475.outer.preheader ], [ %v45191, %block2378 ]
  %v10829.ph.ph = phi i64 [ %v217760, %block475.outer.preheader ], [ %v45176, %block2378 ]
  %v12011.ph.ph = phi i64 [ %v217768, %block475.outer.preheader ], [ %v217294, %block2378 ]
  %v12415.ph.ph = phi i64 [ %v217769, %block475.outer.preheader ], [ %v217296, %block2378 ]
  %t1083 = icmp ne i64 %v10789.ph.ph, 0
  %t1084 = inttoptr i64 %v10789.ph.ph to ptr
  %t1091 = icmp ne i64 %v10791.ph.ph, 0
  %t1092 = inttoptr i64 %v10791.ph.ph to ptr
  %t1103 = icmp ugt i64 %v10798.ph.ph, 258
  %t1107 = icmp ugt i64 %v10802.ph.ph, 9
  %t1179 = zext i64 %v10826.ph.ph to i128
  %t1180 = shl nuw i128 %t1179, 64
  %v218148 = or disjoint i128 %t1180, %t942
  %t1199 = zext i64 %v10828.ph.ph to i128
  %t1200 = shl nuw i128 %t1199, 64
  %v218156 = or disjoint i128 %t1200, 259
  %t1224 = zext i64 %v10789.ph.ph to i128
  %t1225 = shl nuw i128 %t1224, 64
  %t1229 = zext i64 %v10791.ph.ph to i128
  %t1230 = shl nuw i128 %t1229, 64
  %t1239 = zext i64 %v10798.ph.ph to i128
  %t1240 = shl nuw i128 %t1239, 64
  %t1244 = zext i64 %v10802.ph.ph to i128
  %t1245 = shl nuw i128 %t1244, 64
  %t3136.not = icmp eq i64 %v10791.ph.ph, 0
  br label %block475.outer

block475.outer:                                   ; preds = %edge4265, %block475.outer.outer
  %v10787.ph = phi i64 [ %v10787.ph.ph, %block475.outer.outer ], [ %v36628651, %edge4265 ]
  %v10788.ph = phi i64 [ %v10788.ph.ph, %block475.outer.outer ], [ %v4033311241137, %edge4265 ]
  %v10790.ph = phi i64 [ %v10790.ph.ph, %block475.outer.outer ], [ %v40035, %edge4265 ]
  %v10792.ph = phi i64 [ %v10792.ph.ph, %block475.outer.outer ], [ %v40047, %edge4265 ]
  %v10794.ph = phi i64 [ %v10794.ph.ph, %block475.outer.outer ], [ %v36625650, %edge4265 ]
  %v10796.ph = phi i64 [ %v10796.ph.ph, %block475.outer.outer ], [ %v4033011221138, %edge4265 ]
  %v10800.ph = phi i64 [ %v10800.ph.ph, %block475.outer.outer ], [ %v40032, %edge4265 ]
  %v10804.ph = phi i64 [ %v10804.ph.ph, %block475.outer.outer ], [ %v40044, %edge4265 ]
  %v10807.ph = phi i64 [ %v10807.ph.ph, %block475.outer.outer ], [ %v36647656, %edge4265 ]
  %v10808.ph = phi i64 [ %v10808.ph.ph, %block475.outer.outer ], [ %v36650657, %edge4265 ]
  %v10809.ph = phi i64 [ %v10809.ph.ph, %block475.outer.outer ], [ %v41283, %edge4265 ]
  %v10810.ph = phi i64 [ %v10810.ph.ph, %block475.outer.outer ], [ %spec.select1, %edge4265 ]
  %v10811.ph = phi i64 [ %v10811.ph.ph, %block475.outer.outer ], [ %v41333, %edge4265 ]
  %v10812.ph = phi i64 [ %v10812.ph.ph, %block475.outer.outer ], [ %v40314, %edge4265 ]
  %v10813.ph = phi i64 [ %v10813.ph.ph, %block475.outer.outer ], [ %v217321, %edge4265 ]
  %v10814.ph = phi i64 [ %v10814.ph.ph, %block475.outer.outer ], [ %v40054, %edge4265 ]
  %v10815.ph = phi i64 [ %v10815.ph.ph, %block475.outer.outer ], [ %v40057, %edge4265 ]
  %v10816.ph = phi i64 [ 0, %block475.outer.outer ], [ %v41265, %edge4265 ]
  %v10817.ph = phi i64 [ %v10817.ph.ph, %block475.outer.outer ], [ %v217320, %edge4265 ]
  %v10818.ph = phi i64 [ %v10818.ph.ph, %block475.outer.outer ], [ %v217319, %edge4265 ]
  %v10819.ph = phi i64 [ %v10819.ph.ph, %block475.outer.outer ], [ %v217318, %edge4265 ]
  %v10820.ph = phi i64 [ %v10820.ph.ph, %block475.outer.outer ], [ %v217326, %edge4265 ]
  %v10821.ph = phi i64 [ %v10821.ph.ph, %block475.outer.outer ], [ %v217327, %edge4265 ]
  %v10822.ph = phi i64 [ %v10822.ph.ph, %block475.outer.outer ], [ %v36623649, %edge4265 ]
  %v10823.ph = phi i64 [ %v10823.ph.ph, %block475.outer.outer ], [ %v4032811191139, %edge4265 ]
  %v10827.ph = phi i64 [ %v10827.ph.ph, %block475.outer.outer ], [ %v40030, %edge4265 ]
  %v10829.ph = phi i64 [ %v10829.ph.ph, %block475.outer.outer ], [ %v40042, %edge4265 ]
  %v12011.ph = phi i64 [ %v12011.ph.ph, %block475.outer.outer ], [ %v217294, %edge4265 ]
  %v12415.ph = phi i64 [ %v12415.ph.ph, %block475.outer.outer ], [ %v217296, %edge4265 ]
  br label %block475

edge1242:                                         ; preds = %block475
  %t1076 = inttoptr i64 %v10787 to ptr
  %v10631 = load i64, ptr %t1076, align 1
  %t1077 = icmp eq i64 %v10631, 1
  %t1079 = icmp ne i64 %v10788, 0
  %or.cond = select i1 %t1077, i1 %t1079, i1 false
  br i1 %or.cond, label %edge1246, label %edge1406

edge1246:                                         ; preds = %edge1242
  %t1080 = inttoptr i64 %v10788 to ptr
  %v10633 = load i64, ptr %t1080, align 1
  %t1081 = icmp eq i64 %v10633, 1
  %or.cond12 = select i1 %t1081, i1 %t1083, i1 false
  br i1 %or.cond12, label %edge1250, label %edge1406

edge1250:                                         ; preds = %edge1246
  %v10635 = load i64, ptr %t1084, align 1
  %t1085 = icmp eq i64 %v10635, 1
  %t1087 = icmp ne i64 %v10790, 0
  %or.cond13 = select i1 %t1085, i1 %t1087, i1 false
  br i1 %or.cond13, label %edge1254, label %edge1406

edge1254:                                         ; preds = %edge1250
  %t1088 = inttoptr i64 %v10790 to ptr
  %v10637 = load i64, ptr %t1088, align 1
  %t1089 = icmp eq i64 %v10637, 1
  %or.cond14 = select i1 %t1089, i1 %t1091, i1 false
  br i1 %or.cond14, label %edge1258, label %edge1406

edge1258:                                         ; preds = %edge1254
  %v10639 = load i64, ptr %t1092, align 1
  %t1093 = icmp eq i64 %v10639, 1
  %t1095 = icmp ne i64 %v10792, 0
  %or.cond15 = select i1 %t1093, i1 %t1095, i1 false
  br i1 %or.cond15, label %edge1262, label %edge1406

edge1262:                                         ; preds = %edge1258
  %t1096 = inttoptr i64 %v10792 to ptr
  %v10641 = load i64, ptr %t1096, align 1
  %t1097 = icmp eq i64 %v10641, 1
  %t1099 = icmp ugt i64 %v10794, 262143
  %or.cond297.not1218.not1220 = select i1 %t1097, i1 %t1099, i1 false
  %t1101.not = icmp ule i64 %v3718, %v10796
  %or.cond298.not1219 = select i1 %or.cond297.not1218.not1220, i1 %t1101.not, i1 false
  %or.cond299 = select i1 %or.cond298.not1219, i1 %t1103, i1 false
  %t1105 = icmp ugt i64 %v10800, 258
  %or.cond300 = select i1 %or.cond299, i1 %t1105, i1 false
  %or.cond301 = select i1 %or.cond300, i1 %t1107, i1 false
  %t1109 = icmp ugt i64 %v10804, 9
  %or.cond302 = select i1 %or.cond301, i1 %t1109, i1 false
  br i1 %or.cond302, label %edge1278, label %edge1406

edge1278:                                         ; preds = %edge1262
  %t1114 = zext i64 %v10807 to i128
  %t1115 = zext i64 %v10808 to i128
  %t1116 = shl nuw i128 %t1115, 64
  %v218122 = or disjoint i128 %t1116, %t1114
  store i128 %v218122, ptr %ss2, align 16
  %t1118 = zext i64 %v10809 to i128
  %t1119 = zext i64 %v10810 to i128
  %t1120 = shl nuw i128 %t1119, 64
  %v218124 = or disjoint i128 %t1120, %t1118
  store i128 %v218124, ptr %t926, align 16
  %t1123 = zext i64 %v10811 to i128
  %t1124 = zext i64 %v10812 to i128
  %t1125 = shl nuw i128 %t1124, 64
  %v218126 = or disjoint i128 %t1125, %t1123
  store i128 %v218126, ptr %t931, align 16
  %t1128 = zext i64 %v10813 to i128
  %t1129 = zext i64 %v10814 to i128
  %t1130 = shl nuw i128 %t1129, 64
  %v218128 = or disjoint i128 %t1130, %t1128
  store i128 %v218128, ptr %t936, align 16
  %t1133 = zext i64 %v10815 to i128
  %t1134 = zext i64 %v10816 to i128
  %t1135 = shl nuw i128 %t1134, 64
  %v218130 = or disjoint i128 %t1135, %t1133
  store i128 %v218130, ptr %t941, align 16
  %t1138 = zext i64 %v10817 to i128
  %t1139 = zext i64 %v10818 to i128
  %t1140 = shl nuw i128 %t1139, 64
  %v218132 = or disjoint i128 %t1140, %t1138
  store i128 %v218132, ptr %t946, align 16
  %t1143 = zext i64 %v10819 to i128
  %t1144 = zext i64 %v10820 to i128
  %t1145 = shl nuw i128 %t1144, 64
  %v218134 = or disjoint i128 %t1145, %t1143
  store i128 %v218134, ptr %t947, align 16
  %t1148 = zext i64 %v10821 to i128
  %t1149 = zext i64 %v10822 to i128
  %t1150 = shl nuw i128 %t1149, 64
  %v218136 = or disjoint i128 %t1150, %t1148
  store i128 %v218136, ptr %t1152, align 16
  store i128 18446744073709813760, ptr %t1157, align 16
  %t1159 = zext i64 %v10823 to i128
  %t1160 = shl nuw i128 %t1159, 64
  %v218140 = or disjoint i128 %t1160, 262144
  store i128 %v218140, ptr %t1162, align 16
  store i128 %v218142, ptr %t1167, align 16
  store i128 %v218144, ptr %t1172, align 16
  store i128 %v218112, ptr %t1177, align 16
  store i128 %v218148, ptr %t1182, align 16
  store i128 18446744073709551875, ptr %t1187, align 16
  %t1189 = zext i64 %v10827 to i128
  %t1190 = shl nuw i128 %t1189, 64
  %v218152 = or disjoint i128 %t1190, 259
  store i128 %v218152, ptr %t1192, align 16
  store i128 18446744073709551875, ptr %t1197, align 16
  store i128 %v218156, ptr %t1202, align 16
  store i128 18446744073709551626, ptr %t1207, align 16
  %t1209 = zext i64 %v10829 to i128
  %t1210 = shl nuw i128 %t1209, 64
  %v218160 = or disjoint i128 %t1210, 10
  store i128 %v218160, ptr %t1212, align 16
  store i128 18446744073709551626, ptr %t1217, align 16
  %t1219 = zext i64 %v10787 to i128
  %t1220 = shl nuw i128 %t1219, 64
  %v218164 = or disjoint i128 %t1220, 10
  store i128 %v218164, ptr %t1222, align 16
  %t1223 = zext i64 %v10788 to i128
  %v218166 = or disjoint i128 %t1225, %t1223
  store i128 %v218166, ptr %t1227, align 16
  %t1228 = zext i64 %v10790 to i128
  %v218168 = or disjoint i128 %t1230, %t1228
  store i128 %v218168, ptr %t1232, align 16
  %t1233 = zext i64 %v10792 to i128
  %t1234 = zext i64 %v10794 to i128
  %t1235 = shl nuw i128 %t1234, 64
  %v218170 = or disjoint i128 %t1235, %t1233
  store i128 %v218170, ptr %t1237, align 16
  %t1238 = zext i64 %v10796 to i128
  %v218172 = or disjoint i128 %t1240, %t1238
  store i128 %v218172, ptr %t1242, align 16
  %t1243 = zext i64 %v10800 to i128
  %v218174 = or disjoint i128 %t1245, %t1243
  store i128 %v218174, ptr %t1247, align 16
  store i64 %v10804, ptr %t1248, align 16
  %t1249 = call { i64, i64 } @mech_compact_numeric_475_vector(i64 %v0, i64 %v218103, i64 %v8479)
  %v10658 = extractvalue { i64, i64 } %t1249, 0
  %v10659 = extractvalue { i64, i64 } %t1249, 1
  switch i64 %v10658, label %edge1349 [
    i64 103172, label %edge1281
    i64 0, label %edge1350
  ]

edge1281:                                         ; preds = %edge1278
  %t1252 = icmp eq i64 %v10659, 510
  br i1 %t1252, label %edge1283, label %common.ret

edge1283:                                         ; preds = %edge1281
  %v218176 = load i128, ptr %t1255, align 8
  %v218177 = trunc i128 %v218176 to i64
  %t1256 = lshr i128 %v218176, 64
  %v218178 = trunc nuw i128 %t1256 to i64
  %v218180 = load i128, ptr %t1258, align 8
  %v218181 = trunc i128 %v218180 to i64
  %t1259 = lshr i128 %v218180, 64
  %v218182 = trunc nuw i128 %t1259 to i64
  %v218184 = load i128, ptr %t1261, align 8
  %v218185 = trunc i128 %v218184 to i64
  %t1262 = lshr i128 %v218184, 64
  %v218186 = trunc nuw i128 %t1262 to i64
  %v218188 = load i128, ptr %t1264, align 8
  %v218189 = trunc i128 %v218188 to i64
  %t1265 = lshr i128 %v218188, 64
  %v218190 = trunc nuw i128 %t1265 to i64
  %v218192 = load i128, ptr %t1267, align 8
  %v218193 = trunc i128 %v218192 to i64
  %t1268 = lshr i128 %v218192, 64
  %v218194 = trunc nuw i128 %t1268 to i64
  %v218196 = load i128, ptr %t1270, align 8
  %v218197 = trunc i128 %v218196 to i64
  %t1271 = lshr i128 %v218196, 64
  %v218198 = trunc nuw i128 %t1271 to i64
  %v218200 = load i128, ptr %t1273, align 8
  %v218201 = trunc i128 %v218200 to i64
  %t1274 = lshr i128 %v218200, 64
  %v218202 = trunc nuw i128 %t1274 to i64
  %v10678 = load i64, ptr %t1275, align 8
  %v10679 = load i64, ptr %t1076, align 1
  %v10680 = add i64 %v10679, 1
  store i64 %v10680, ptr %t1076, align 1
  %v10681 = load i64, ptr %t1080, align 1
  %v10682 = add i64 %v10681, 1
  store i64 %v10682, ptr %t1080, align 1
  br i1 %t23.not, label %edge1294, label %edge1291

edge1291:                                         ; preds = %edge1283
  %v10683 = load i64, ptr %t910, align 1
  %v10684 = add i64 %v10683, 1
  store i64 %v10684, ptr %t910, align 1
  br label %edge1294

edge1294:                                         ; preds = %edge1291, %edge1283
  %v10685 = load i64, ptr %t1084, align 1
  %v10686 = add i64 %v10685, 1
  store i64 %v10686, ptr %t1084, align 1
  %v10687 = load i64, ptr %t1088, align 1
  %v10688 = add i64 %v10687, 1
  store i64 %v10688, ptr %t1088, align 1
  %v10689 = load i64, ptr %t1092, align 1
  %v10690 = add i64 %v10689, 1
  store i64 %v10690, ptr %t1092, align 1
  %v10691 = load i64, ptr %t1096, align 1
  %v10692 = add i64 %v10691, 1
  store i64 %v10692, ptr %t1096, align 1
  %v10696 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10787)
  %t1298.not = icmp eq i64 %v10696, 0
  br i1 %t1298.not, label %edge1314, label %block11700

edge1314:                                         ; preds = %edge1294
  %v10700 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10788)
  %t1300.not = icmp eq i64 %v10700, 0
  br i1 %t1300.not, label %block11753, label %block11700

edge1320:                                         ; preds = %block11753
  %v10704 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t1302.not = icmp eq i64 %v10704, 0
  br i1 %t1302.not, label %edge1326, label %block11700

edge1326:                                         ; preds = %block11753, %edge1320
  %v10708 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10789.ph.ph)
  %t1304.not = icmp eq i64 %v10708, 0
  br i1 %t1304.not, label %edge1332, label %block11700

edge1332:                                         ; preds = %edge1326
  %v10712 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10790)
  %t1306.not = icmp eq i64 %v10712, 0
  br i1 %t1306.not, label %edge1338, label %block11700

edge1338:                                         ; preds = %edge1332
  %v10716 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10791.ph.ph)
  %t1308.not = icmp eq i64 %v10716, 0
  br i1 %t1308.not, label %edge1344, label %block11700

edge1344:                                         ; preds = %edge1338
  %v10720 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10792)
  %t1310.not = icmp eq i64 %v10720, 0
  br i1 %t1310.not, label %edge1438, label %block11700

edge1349:                                         ; preds = %edge1278
  %t1314 = icmp eq i64 %v10658, 103169
  %v10723 = select i1 %t1314, i64 103425, i64 %v10658
  %t1316 = insertvalue { i64, i64 } poison, i64 %v10723, 0
  %t1317 = insertvalue { i64, i64 } %t1316, i64 %v10659, 1
  br label %common.ret

edge1350:                                         ; preds = %edge1278
  %v218204 = load i128, ptr %t1267, align 8
  %v218205 = trunc i128 %v218204 to i64
  %t1320 = lshr i128 %v218204, 64
  %v218206 = trunc nuw i128 %t1320 to i64
  %v10726 = load i64, ptr %t1270, align 8
  %v218208 = load i128, ptr %t1273, align 8
  %v218209 = trunc i128 %v218208 to i64
  %t1324 = lshr i128 %v218208, 64
  %v218210 = trunc nuw i128 %t1324 to i64
  %v218212 = load i128, ptr %t1326, align 8
  %v218213 = trunc i128 %v218212 to i64
  %t1327 = lshr i128 %v218212, 64
  %v218214 = trunc nuw i128 %t1327 to i64
  %v218216 = load i128, ptr %t1329, align 8
  %v218217 = trunc i128 %v218216 to i64
  %t1330 = lshr i128 %v218216, 64
  %v218218 = trunc nuw i128 %t1330 to i64
  %v218220 = load i128, ptr %t1332, align 8
  %v218221 = trunc i128 %v218220 to i64
  %t1333 = lshr i128 %v218220, 64
  %v218222 = trunc nuw i128 %t1333 to i64
  %v218224 = load i128, ptr %t1335, align 8
  %v218225 = trunc i128 %v218224 to i64
  %t1336 = lshr i128 %v218224, 64
  %v218226 = trunc nuw i128 %t1336 to i64
  %v10739 = load i64, ptr %t1337, align 8
  br i1 %t23.not, label %edge5300, label %edge1353

edge1353:                                         ; preds = %edge1350
  %v10755 = load i64, ptr %t910, align 1
  %v10756 = add i64 %v10755, -1
  store i64 %v10756, ptr %t910, align 1
  %t1341.not = icmp eq i64 %v10756, 0
  br i1 %t1341.not, label %edge1356, label %edge5300

edge1356:                                         ; preds = %edge1353
  %v10757 = call i64 @mech_compact_free(i64 %v0, i64 %v1399)
  %t1342.not = icmp eq i64 %v10757, 0
  br i1 %t1342.not, label %edge5300, label %block11700

edge1406:                                         ; preds = %edge1262, %edge1258, %edge1254, %edge1250, %edge1246, %edge1242, %block475
  %v11037 = sub i64 %v1395, %v10809
  %t1345 = icmp eq i64 %v10809, %v10811
  br i1 %t1345, label %edge1438, label %block535

edge1423:                                         ; preds = %edge1496
  %t1355 = inttoptr i64 %v11461 to ptr
  %v11395 = load i64, ptr %t1355, align 1
  %v11396 = add i64 %v11395, 1
  store i64 %v11396, ptr %t1355, align 1
  %v11400 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v11461)
  %t1357.not = icmp eq i64 %v11400, 0
  br i1 %t1357.not, label %block535, label %block12020

edge1438:                                         ; preds = %edge1406, %edge1344
  %v11828 = phi i64 [ 1, %edge1406 ], [ %v10678, %edge1344 ]
  %v11996 = phi i64 [ %v10809, %edge1406 ], [ %v218181, %edge1344 ]
  %v12009 = phi i64 [ %v10810, %edge1406 ], [ %v12011, %edge1344 ]
  %v12209 = phi i64 [ %v11037, %edge1406 ], [ %v218202, %edge1344 ]
  %v12413 = phi i64 [ %v10809, %edge1406 ], [ %v12415, %edge1344 ]
  %v12657 = phi i64 [ %v10816, %edge1406 ], [ %v218190, %edge1344 ]
  %v14679 = phi i64 [ %v10807, %edge1406 ], [ %v218177, %edge1344 ]
  %v18188 = phi i64 [ %v10812, %edge1406 ], [ %v218182, %edge1344 ]
  %v18667 = phi i64 [ %v10808, %edge1406 ], [ %v218178, %edge1344 ]
  %v37160 = phi i64 [ %v10815, %edge1406 ], [ %v218189, %edge1344 ]
  %v37923 = phi i64 [ %v10814, %edge1406 ], [ %v218186, %edge1344 ]
  %v38007 = phi i64 [ %v10819, %edge1406 ], [ %v218197, %edge1344 ]
  %v41624 = phi i64 [ %v10818, %edge1406 ], [ %v218194, %edge1344 ]
  %v41851 = phi i64 [ %v10817, %edge1406 ], [ %v218193, %edge1344 ]
  %v42188 = phi i64 [ %v10813, %edge1406 ], [ %v218185, %edge1344 ]
  %v48118 = phi i64 [ %v10820, %edge1406 ], [ %v218198, %edge1344 ]
  %v48174 = phi i64 [ %v10821, %edge1406 ], [ %v218201, %edge1344 ]
  %v11456 = call i64 @mech_compact_allocate(i64 %v0, i64 262144, i64 2, i64 2, i64 0, i64 %v3192)
  %t1372.not = icmp eq i64 %v11456, 0
  br i1 %t1372.not, label %edge1441, label %block12030

edge1441:                                         ; preds = %edge1438
  %v11459 = load i64, ptr %ss1, align 8
  %v11460 = load i64, ptr %t606, align 8
  %v11461 = load i64, ptr %t607, align 8
  store i128 604462909807314587353088, ptr %ss4, align 16
  store i128 18446744073709551616, ptr %t1383, align 16
  %v11463 = call i64 @mech_compact_word_collection_10(i64 %v218227, i64 %v10822, i64 %v11459, i64 262144)
  %t1384 = icmp eq i64 %v11463, 0
  br i1 %t1384, label %edge1442, label %block12030

edge1442:                                         ; preds = %edge1441
  %t1386.not = icmp eq i64 %v11461, 0
  br i1 %t1386.not, label %block12031, label %edge1444

edge1444:                                         ; preds = %edge1442
  %t1387 = inttoptr i64 %v11461 to ptr
  %v11466 = load i64, ptr %t1387, align 1
  %v11467 = add i64 %v11466, 1
  store i64 %v11467, ptr %t1387, align 1
  br label %block12031

edge1453:                                         ; preds = %block12031
  %v11475 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10787)
  %t1392.not = icmp eq i64 %v11475, 0
  br i1 %t1392.not, label %edge1496, label %block12030

edge1496:                                         ; preds = %block12031, %edge1453
  br i1 %t1386.not, label %block535, label %edge1423

edge1545:                                         ; preds = %block535
  %v12938 = sub i64 %v217293, %spec.select1
  %v13657 = add i64 %spec.select17, %v217293
  %t1495.not = icmp ult i64 %v13657, %v217293
  br i1 %t1495.not, label %common.ret, label %edge1658

edge1658:                                         ; preds = %edge1545
  %t1499.not = icmp ugt i64 %v13657, %v1395
  br i1 %t1499.not, label %edge1677, label %edge1763

edge1677:                                         ; preds = %edge1658
  br i1 %t23.not, label %block13063, label %edge1680

edge1680:                                         ; preds = %edge1677
  %v13741 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t1502.not = icmp eq i64 %v13741, 0
  br i1 %t1502.not, label %block13063, label %block12995

edge1686:                                         ; preds = %block13063
  %v13745 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217710)
  %t1504.not = icmp eq i64 %v13745, 0
  br i1 %t1504.not, label %block13067, label %block12995

edge1692:                                         ; preds = %block13067
  %v13749 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217725)
  %t1506.not = icmp eq i64 %v13749, 0
  br i1 %t1506.not, label %block13071, label %block12995

edge1698:                                         ; preds = %block13071
  %v13753 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217738)
  %t1508.not = icmp eq i64 %v13753, 0
  br i1 %t1508.not, label %block13075, label %block12995

edge1704:                                         ; preds = %block13075
  %v13757 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217744)
  %t1510.not = icmp eq i64 %v13757, 0
  br i1 %t1510.not, label %block13079, label %block12995

edge1710:                                         ; preds = %block13079
  %v13761 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217754)
  %t1512.not = icmp eq i64 %v13761, 0
  br i1 %t1512.not, label %block13087, label %block12995

edge1722:                                         ; preds = %block13087
  %v13769 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10788)
  %t1516.not = icmp eq i64 %v13769, 0
  br i1 %t1516.not, label %block13091, label %block12995

edge1728:                                         ; preds = %block13091
  %v13773 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t1518.not = icmp eq i64 %v13773, 0
  br i1 %t1518.not, label %block13095, label %block12995

edge1734:                                         ; preds = %block13095
  %v13777 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10789.ph.ph)
  %t1520.not = icmp eq i64 %v13777, 0
  br i1 %t1520.not, label %block13099, label %block12995

edge1740:                                         ; preds = %block13099
  %v13781 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10790)
  %t1522.not = icmp eq i64 %v13781, 0
  br i1 %t1522.not, label %block13103, label %block12995

edge1746:                                         ; preds = %block13103
  %v13785 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10791.ph.ph)
  %t1524.not = icmp eq i64 %v13785, 0
  br i1 %t1524.not, label %block13107, label %block12995

edge1752:                                         ; preds = %block13107
  %v13789 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10792)
  %t1526.not = icmp eq i64 %v13789, 0
  br i1 %t1526.not, label %block13111, label %block12995

edge1758:                                         ; preds = %block13111
  %v13793 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217304)
  %t1528.not = icmp eq i64 %v13793, 0
  br i1 %t1528.not, label %common.ret, label %block12995

edge1763:                                         ; preds = %edge1658
  %v13812 = add i64 %v217293, 1
  %t1533.not = icmp ugt i64 %v13812, %v14023
  br i1 %t1533.not, label %edge1789, label %edge1776

edge1776:                                         ; preds = %edge1763
  br i1 %v1061851.not1217, label %edge1781, label %edge1781.thread

edge1781.thread:                                  ; preds = %edge1776
  %v14210 = add i64 %v13812, %v1394
  %t1539 = inttoptr i64 %v14210 to ptr
  %t1540 = load i32, ptr %t1539, align 1
  %v14211 = zext i32 %t1540 to i64
  br label %edge1789

edge1781:                                         ; preds = %edge1776
  %v14213 = call i64 @mech_compact_matrix_word(i64 %v1394, i64 %v13812, i64 %v1397, i64 %v1398, i64 4)
  %t1541 = icmp ult i64 %v14213, 4294967296
  br i1 %t1541, label %edge1789, label %common.ret

edge1789:                                         ; preds = %edge1781.thread, %edge1781, %edge1763
  %v14421 = phi i64 [ %v14213, %edge1781 ], [ 0, %edge1763 ], [ %v14211, %edge1781.thread ]
  %v14475 = and i64 %v14421, 16777215
  %v14532 = mul nuw nsw i64 %v14475, 506832829
  %v14537 = lshr i64 %v14532, 16
  %v14586 = and i64 %v14537, 65535
  %v14595 = mul nuw nsw i64 %v14421, 506832829
  %v14600 = lshr i64 %v14595, 16
  %v14613 = and i64 %v14600, 65535
  %v14626 = add i64 %v12938, 32768
  %v14681 = shl i64 %v217305, 2
  %v14718 = and i64 %v14681, 262140
  %v14774 = or disjoint i64 %v14718, 262144
  %v14775 = add i64 %v14774, %v217306
  %t1549 = inttoptr i64 %v14775 to ptr
  %t1550 = load i16, ptr %t1549, align 1
  %v14776 = zext i16 %t1550 to i64
  %v14802 = or disjoint i64 %v14718, 262146
  %v14803 = add i64 %v14802, %v217306
  %t1555 = inttoptr i64 %v14803 to ptr
  %t1556 = load i16, ptr %t1555, align 1
  %v14804 = zext i16 %t1556 to i64
  %t1559.not = icmp eq i64 %v217304, 0
  br i1 %t1559.not, label %edge1835, label %edge1822

edge1822:                                         ; preds = %edge1789
  %t1561 = inttoptr i64 %v217304 to ptr
  %v14817 = load i64, ptr %t1561, align 1
  %t1562 = icmp eq i64 %v14817, 1
  %v14827 = trunc i64 %v14626 to i16
  br i1 %t1562, label %edge1826, label %edge1835.thread

edge1826:                                         ; preds = %edge1822
  store i16 %v14827, ptr %t1549, align 1
  br label %block662.sink.split

edge1835:                                         ; preds = %edge1789
  %11 = trunc i64 %v14626 to i16
  %t1578 = icmp eq i16 %t1550, %11
  br i1 %t1578, label %edge1858.thread, label %block13485

edge1835.thread:                                  ; preds = %edge1822
  %t1578475 = icmp eq i16 %t1550, %v14827
  br i1 %t1578475, label %block13483, label %block13485

edge1847:                                         ; preds = %block13483
  %t1606 = inttoptr i64 %v14878 to ptr
  %v14902 = load i64, ptr %t1606, align 1
  %v14903 = add i64 %v14902, 1
  store i64 %v14903, ptr %t1606, align 1
  br label %block13489

edge1852:                                         ; preds = %block13489
  %v14908 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217304)
  %t1610.not = icmp eq i64 %v14908, 0
  br i1 %t1610.not, label %edge1858, label %block13460

edge1858:                                         ; preds = %block13489, %edge1852
  %v14927 = add i64 %v14873, %v14802
  %t1615 = inttoptr i64 %v14927 to ptr
  %t1616 = load i16, ptr %t1615, align 1
  %t1617 = icmp eq i16 %t1616, %t1550
  br i1 %t1617, label %block662, label %edge1861

edge1858.thread:                                  ; preds = %edge1835
  %v1492720997 = add i64 %v217306, %v14802
  %t161520998 = inttoptr i64 %v1492720997 to ptr
  %t161620999 = load i16, ptr %t161520998, align 1
  %t161721000 = icmp eq i16 %t161620999, %t1550
  br i1 %t161721000, label %block662, label %block13500

edge1861:                                         ; preds = %edge1858
  br i1 %t1605.not52, label %block13500, label %edge1862

edge1862:                                         ; preds = %edge1861
  %t1620 = inttoptr i64 %v14878 to ptr
  %v14944 = load i64, ptr %t1620, align 1
  %t1621 = icmp eq i64 %v14944, 1
  br i1 %t1621, label %block662.sink.split, label %block13500

edge1870:                                         ; preds = %block13498
  %t1645 = inttoptr i64 %v14956 to ptr
  %v14961 = load i64, ptr %t1645, align 1
  %v14962 = add i64 %v14961, 1
  store i64 %v14962, ptr %t1645, align 1
  br label %block13504

edge1875:                                         ; preds = %block13504
  %v14967 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1487820775207862100321013)
  %t1649.not = icmp eq i64 %v14967, 0
  br i1 %t1649.not, label %block662, label %block13495

edge1885:                                         ; preds = %block662
  %t1654.not = icmp ult i64 %v12938, %v14776
  br i1 %t1654.not, label %edge1896, label %edge2015

edge1889:                                         ; preds = %edge1945, %edge1994
  %v15981489.pn = phi i64 [ %v17057, %edge1994 ], [ %v15981489, %edge1945 ]
  %t1702.not.not = icmp eq i64 %v217293, %v15981489.pn
  br i1 %t1702.not.not, label %edge2015, label %edge2017

edge1896:                                         ; preds = %edge1885
  %t1657.not = icmp ugt i64 %v217293, %v14023
  br i1 %t1657.not, label %edge1917, label %edge1904

edge1904:                                         ; preds = %edge1896
  br i1 %v1061851.not1217, label %edge1909, label %edge1917.thread496

edge1909:                                         ; preds = %edge1904
  %v15725 = call i64 @mech_compact_matrix_word(i64 %v1394, i64 %v217293, i64 %v1397, i64 %v1398, i64 4)
  %t1665 = icmp ult i64 %v15725, 4294967296
  br i1 %t1665, label %edge1917.thread, label %common.ret

edge1917:                                         ; preds = %edge1896
  %v15972 = add i64 %spec.select1, -32768
  %v15981 = add i64 %v15972, %v14776
  %t1669.not = icmp ugt i64 %v15981, %v14023
  br i1 %t1669.not, label %edge1945, label %edge1932

edge1917.thread496:                               ; preds = %edge1904
  %v15722 = add i64 %v217293, %v1394
  %t1663 = inttoptr i64 %v15722 to ptr
  %t1664 = load i32, ptr %t1663, align 1
  %12 = and i32 %t1664, 16777215
  %v15951498 = zext nneg i32 %12 to i64
  %v15972499 = add i64 %spec.select1, -32768
  %v15981500 = add i64 %v15972499, %v14776
  %t1669.not501 = icmp ugt i64 %v15981500, %v14023
  br i1 %t1669.not501, label %edge1945, label %edge1937.thread

edge1917.thread:                                  ; preds = %edge1909
  %v15951482 = and i64 %v15725, 16777215
  %v15972483 = add i64 %spec.select1, -32768
  %v15981484 = add i64 %v15972483, %v14776
  %t1669.not485 = icmp ugt i64 %v15981484, %v14023
  br i1 %t1669.not485, label %edge1945, label %edge1937

edge1932:                                         ; preds = %edge1917
  br i1 %v1061851.not1217, label %edge1937, label %edge1937.thread

edge1937.thread:                                  ; preds = %edge1932, %edge1917.thread496
  %v15951486506 = phi i64 [ 0, %edge1932 ], [ %v15951498, %edge1917.thread496 ]
  %v15981488505 = phi i64 [ %v15981, %edge1932 ], [ %v15981500, %edge1917.thread496 ]
  %v16364 = add i64 %v15981488505, %v1394
  %t1675 = inttoptr i64 %v16364 to ptr
  %t1676 = load i32, ptr %t1675, align 1
  %v16365 = zext i32 %t1676 to i64
  br label %edge1945

edge1937:                                         ; preds = %edge1917.thread, %edge1932
  %v15951486495 = phi i64 [ 0, %edge1932 ], [ %v15951482, %edge1917.thread ]
  %v15981488492 = phi i64 [ %v15981, %edge1932 ], [ %v15981484, %edge1917.thread ]
  %v16367 = call i64 @mech_compact_matrix_word(i64 %v1394, i64 %v15981488492, i64 %v1397, i64 %v1398, i64 4)
  %t1677 = icmp ult i64 %v16367, 4294967296
  br i1 %t1677, label %edge1945, label %common.ret

edge1945:                                         ; preds = %edge1937.thread, %edge1917.thread496, %edge1917.thread, %edge1937, %edge1917
  %v15981489 = phi i64 [ %v15981488492, %edge1937 ], [ %v15981, %edge1917 ], [ %v15981484, %edge1917.thread ], [ %v15981500, %edge1917.thread496 ], [ %v15981488505, %edge1937.thread ]
  %v15951487 = phi i64 [ %v15951486495, %edge1937 ], [ 0, %edge1917 ], [ %v15951482, %edge1917.thread ], [ %v15951498, %edge1917.thread496 ], [ %v15951486506, %edge1937.thread ]
  %v16568 = phi i64 [ %v16367, %edge1937 ], [ 0, %edge1917 ], [ 0, %edge1917.thread ], [ 0, %edge1917.thread496 ], [ %v16365, %edge1937.thread ]
  %v16593 = and i64 %v16568, 16777215
  %t1681 = icmp eq i64 %v15951487, %v16593
  br i1 %t1681, label %edge1889, label %edge1959

edge1959:                                         ; preds = %edge1945
  %t1684.not = icmp samesign ult i64 %v12938, %v14804
  br i1 %t1684.not, label %edge1968, label %edge2015

edge1968:                                         ; preds = %edge1959
  %v17052 = add i64 %spec.select1, -32768
  %v17057 = add i64 %v17052, %v14804
  %t1687.not = icmp ugt i64 %v17057, %v14023
  br i1 %t1687.not, label %edge1994, label %edge1981

edge1981:                                         ; preds = %edge1968
  br i1 %v1061851.not1217, label %edge1986, label %edge1986.thread

edge1986.thread:                                  ; preds = %edge1981
  %v17436 = add i64 %v17057, %v1394
  %t1693 = inttoptr i64 %v17436 to ptr
  %t1694 = load i32, ptr %t1693, align 1
  %v17437 = zext i32 %t1694 to i64
  br label %edge1994

edge1986:                                         ; preds = %edge1981
  %v17439 = call i64 @mech_compact_matrix_word(i64 %v1394, i64 %v17057, i64 %v1397, i64 %v1398, i64 4)
  %t1695 = icmp ult i64 %v17439, 4294967296
  br i1 %t1695, label %edge1994, label %common.ret

edge1994:                                         ; preds = %edge1986.thread, %edge1986, %edge1968
  %v17640 = phi i64 [ %v17439, %edge1986 ], [ 0, %edge1968 ], [ %v17437, %edge1986.thread ]
  %v17665 = and i64 %v17640, 16777215
  %t1699 = icmp eq i64 %v15951487, %v17665
  br i1 %t1699, label %edge1889, label %edge2015

edge2015:                                         ; preds = %edge1959, %edge1994, %edge1885, %edge1889, %block15245, %edge2042
  %t1702.not478 = phi i64 [ 1, %block15245 ], [ 1, %edge2042 ], [ 0, %edge1889 ], [ 0, %edge1885 ], [ 0, %edge1994 ], [ 0, %edge1959 ]
  %v18604 = phi i64 [ %v18354, %block15245 ], [ %v18354, %edge2042 ], [ %v10823, %edge1889 ], [ %v10823, %edge1885 ], [ %v10823, %edge1994 ], [ %v10823, %edge1959 ]
  %v18606 = phi i64 [ %v18356, %block15245 ], [ %v18356, %edge2042 ], [ %v10796, %edge1889 ], [ %v10796, %edge1885 ], [ %v10796, %edge1994 ], [ %v10796, %edge1959 ]
  %v18609 = phi i64 [ %v18359, %block15245 ], [ %v18359, %edge2042 ], [ %v10788, %edge1889 ], [ %v10788, %edge1885 ], [ %v10788, %edge1994 ], [ %v10788, %edge1959 ]
  %v18610 = add i64 %t1702.not478, %v217308
  br label %edge2077

edge2017:                                         ; preds = %edge1889
  %v17912 = sub i64 %v217293, %v15981489.pn
  %v18147 = shl i64 %v17912, 16
  %v18152 = and i64 %v18147, 4294901760
  %v18172 = or disjoint i64 %v18152, 3
  %t1706 = icmp ult i64 %v217308, %v3718
  br i1 %t1706, label %edge2025, label %block15236

edge2025:                                         ; preds = %edge2017
  %v18348 = shl nuw nsw i64 %v217308, 2
  %v18349 = add i64 %v18348, %v10823
  %t1708 = inttoptr i64 %v18349 to ptr
  %t1709 = load i32, ptr %t1708, align 1
  %v18350 = zext i32 %t1709 to i64
  %t1710 = icmp eq i64 %v18172, %v18350
  br i1 %t1710, label %block15239, label %edge2028

edge2028:                                         ; preds = %edge2025
  %t1712.not = icmp eq i64 %v10788, 0
  br i1 %t1712.not, label %block15241, label %edge2029

edge2029:                                         ; preds = %edge2028
  %t1713 = inttoptr i64 %v10788 to ptr
  %v18366 = load i64, ptr %t1713, align 1
  %t1714 = icmp eq i64 %v18366, 1
  br i1 %t1714, label %block15240, label %block15241

edge2034:                                         ; preds = %block15241
  %v18376 = load i64, ptr %ss1, align 8
  %v18377 = load i64, ptr %t606, align 8
  %v18378 = load i64, ptr %t607, align 8
  %.pre20751 = add i64 %v18376, %v18348
  %.pre20752 = inttoptr i64 %.pre20751 to ptr
  br label %block15240

edge2037:                                         ; preds = %block15239
  %t1738 = inttoptr i64 %v18359 to ptr
  %v18383 = load i64, ptr %t1738, align 1
  %v18384 = add i64 %v18383, 1
  store i64 %v18384, ptr %t1738, align 1
  br label %block15245

edge2042:                                         ; preds = %block15245
  %v18389 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10788)
  %t1742.not = icmp eq i64 %v18389, 0
  br i1 %t1742.not, label %edge2015, label %block15236

edge2077:                                         ; preds = %edge2015, %block662
  %v18644 = phi i64 [ %v18604, %edge2015 ], [ %v10823, %block662 ]
  %v18646 = phi i64 [ %v18606, %edge2015 ], [ %v10796, %block662 ]
  %v18649 = phi i64 [ %v18609, %edge2015 ], [ %v10788, %block662 ]
  %v18656 = phi i64 [ %v18610, %edge2015 ], [ %v217308, %block662 ]
  %v18684 = shl i64 %v217311, 1
  %v18738 = and i64 %v18684, 131070
  %v18767 = or disjoint i64 %v18738, 131072
  %v18768 = add i64 %v18807, %v18767
  %t1750 = inttoptr i64 %v18768 to ptr
  %t1751 = load i16, ptr %t1750, align 1
  %v18769 = zext i16 %t1751 to i64
  %13 = trunc i64 %v14626 to i16
  %t1758 = icmp eq i16 %t1751, %13
  br i1 %t1758, label %block15430, label %edge2084

edge2084:                                         ; preds = %edge2077
  %t1760.not = icmp eq i64 %v18830, 0
  br i1 %t1760.not, label %block15432, label %edge2085

edge2085:                                         ; preds = %edge2084
  %t1761 = inttoptr i64 %v18830 to ptr
  %v18885 = load i64, ptr %t1761, align 1
  %t1762 = icmp eq i64 %v18885, 1
  br i1 %t1762, label %block15431, label %block15432

edge2090:                                         ; preds = %block15432
  %v18895 = load i64, ptr %ss1, align 8
  %v18896 = load i64, ptr %t606, align 8
  %v18897 = load i64, ptr %t607, align 8
  %.pre20749 = add i64 %v18895, %v18767
  %.pre20750 = inttoptr i64 %.pre20749 to ptr
  br label %block15431

edge2093:                                         ; preds = %block15430
  %t1786 = inttoptr i64 %v18878 to ptr
  %v18902 = load i64, ptr %t1786, align 1
  %v18903 = add i64 %v18902, 1
  store i64 %v18903, ptr %t1786, align 1
  br label %block15436

edge2098:                                         ; preds = %block15436
  %v18908 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v18830)
  %t1790.not = icmp eq i64 %v18908, 0
  br i1 %t1790.not, label %edge2102, label %block15427

edge2102:                                         ; preds = %edge2098, %block15436
  %v18986 = shl i64 %v12938, 1
  %v19004 = and i64 %v18986, 65534
  %v19009 = or disjoint i64 %v19004, 1
  %t1793.not = icmp ult i64 %v12938, %v18769
  br i1 %t1793.not, label %edge2178, label %edge2112

edge2112:                                         ; preds = %edge2102
  br i1 %t1785.not58, label %edge2127, label %edge2114

edge2114:                                         ; preds = %edge2112
  %t1798 = inttoptr i64 %v18878 to ptr
  %v19196 = load i64, ptr %t1798, align 1
  %t1799 = icmp eq i64 %v19196, 1
  %v19204 = shl nuw nsw i64 %v19004, 1
  %v19205 = add i64 %v18873, %v19204
  %t1803 = inttoptr i64 %v19205 to ptr
  br i1 %t1799, label %edge2118, label %edge2127.thread

edge2118:                                         ; preds = %edge2114
  store i16 0, ptr %t1803, align 1
  %v19226 = shl nuw nsw i64 %v19009, 1
  %v19227 = add i64 %v18873, %v19226
  %t1806 = inttoptr i64 %v19227 to ptr
  br label %edge3980.sink.split

edge2127:                                         ; preds = %edge2112
  %v19246 = shl nuw nsw i64 %v19004, 1
  %v19247 = add i64 %v18873, %v19246
  %t1813 = inttoptr i64 %v19247 to ptr
  %t1814 = load i16, ptr %t1813, align 1
  %t1815 = icmp eq i16 %t1814, 0
  br i1 %t1815, label %edge2150.thread, label %block15597

edge2127.thread:                                  ; preds = %edge2114
  %t1814516 = load i16, ptr %t1803, align 1
  %t1815517 = icmp eq i16 %t1814516, 0
  br i1 %t1815517, label %block15595, label %block15597

edge2139:                                         ; preds = %block15595
  %t1843 = inttoptr i64 %v19257 to ptr
  %v19281 = load i64, ptr %t1843, align 1
  %v19282 = add i64 %v19281, 1
  store i64 %v19282, ptr %t1843, align 1
  br label %block15601

edge2144:                                         ; preds = %block15601
  %v19287 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v18878)
  %t1847.not = icmp eq i64 %v19287, 0
  br i1 %t1847.not, label %edge2150, label %block15572

edge2150:                                         ; preds = %block15601, %edge2144
  %v19347 = shl nuw nsw i64 %v19009, 1
  %v19348 = add i64 %v19252, %v19347
  %t1852 = inttoptr i64 %v19348 to ptr
  %t1853 = load i16, ptr %t1852, align 1
  %t1854 = icmp eq i16 %t1853, 0
  br i1 %t1854, label %edge3980, label %edge2153

edge2150.thread:                                  ; preds = %edge2127
  %v1934721021 = shl nuw nsw i64 %v19009, 1
  %v1934821022 = add i64 %v18873, %v1934721021
  %t185221023 = inttoptr i64 %v1934821022 to ptr
  %t185321024 = load i16, ptr %t185221023, align 1
  %t185421025 = icmp eq i16 %t185321024, 0
  br i1 %t185421025, label %edge3980, label %block15612

edge2153:                                         ; preds = %edge2150
  br i1 %t1842.not90, label %block15612, label %edge2154

edge2154:                                         ; preds = %edge2153
  %t1857 = inttoptr i64 %v19257 to ptr
  %v19365 = load i64, ptr %t1857, align 1
  %t1858 = icmp eq i64 %v19365, 1
  br i1 %t1858, label %edge3980.sink.split, label %block15612

edge2162:                                         ; preds = %block15610
  %t1882 = inttoptr i64 %v19377 to ptr
  %v19382 = load i64, ptr %t1882, align 1
  %v19383 = add i64 %v19382, 1
  store i64 %v19383, ptr %t1882, align 1
  br label %block15616

edge2167:                                         ; preds = %block15616
  %v19388 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1925720815208262102821041)
  %t1886.not = icmp eq i64 %v19388, 0
  br i1 %t1886.not, label %edge3980, label %block15607

edge2178:                                         ; preds = %edge2102
  br i1 %t1457, label %edge3097, label %edge2183

edge2183:                                         ; preds = %edge2178
  br i1 %t23.not, label %block913.preheader, label %edge2184

edge2184:                                         ; preds = %edge2183
  %v19786 = load i64, ptr %t910, align 1
  %v19787 = add i64 %v19786, 1
  store i64 %v19787, ptr %t910, align 1
  br label %block913.preheader

block913.preheader:                               ; preds = %edge2183, %edge2184
  br label %block913

edge2227:                                         ; preds = %block913
  %v20122 = sub nsw i64 %v12938, %v19905
  %v20128 = and i64 %v20122, 32767
  %spec.select325 = call i64 @llvm.usub.sat.i64(i64 %v217293, i64 %v20128)
  %v20333 = add i64 %v19917, %spec.select325
  %t1904 = icmp ult i64 %v20333, %v1395
  br i1 %t1904, label %edge2251, label %edge2249

edge2249:                                         ; preds = %edge2227, %block16244
  %v20786 = phi i64 [ %v20538, %block16244 ], [ 0, %edge2227 ]
  %v20560 = add i64 %v19917, %v217293
  %t1914 = icmp ult i64 %v20560, %v1395
  br i1 %t1914, label %edge2270, label %edge2268

edge2251:                                         ; preds = %edge2227
  br i1 %v1061851.not1217, label %edge2256, label %block16244

edge2256:                                         ; preds = %edge2251
  br i1 %t975, label %trap1911, label %ok1911

edge2268:                                         ; preds = %edge2249, %block16387
  %v20787 = phi i64 [ %v20764, %block16387 ], [ 0, %edge2249 ]
  %t1924 = icmp eq i64 %v20786, %v20787
  br i1 %t1924, label %edge2292, label %edge2741

edge2270:                                         ; preds = %edge2249
  br i1 %v1061851.not1217, label %edge2275, label %block16387

edge2275:                                         ; preds = %edge2270
  br i1 %t975, label %trap1921, label %ok1921

edge2285:                                         ; preds = %edge2590, %edge2506, %edge2507, %edge2510, %edge2451, %edge2453, %edge2456
  %v24736 = phi i64 [ %umax, %edge2456 ], [ %v24709, %edge2590 ], [ %v22057, %edge2506 ], [ %v22057, %edge2507 ], [ %v22057, %edge2510 ], [ %umax, %edge2451 ], [ %umax, %edge2453 ]
  %t2040 = icmp ugt i64 %v24736, %v19920
  br i1 %t2040, label %edge2607, label %edge2741

edge2292:                                         ; preds = %edge2268
  %v20966 = add nuw nsw i64 %v19917, 1
  %t1932 = icmp ult i64 %spec.select325, %v217293
  br i1 %t1932, label %edge2405, label %edge2313

edge2313:                                         ; preds = %edge2292
  br i1 %t23.not, label %block16598, label %edge2316

edge2316:                                         ; preds = %edge2313
  %v21064 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t1935.not = icmp eq i64 %v21064, 0
  br i1 %t1935.not, label %block16598, label %block16530

edge2322:                                         ; preds = %block16598
  %v21068 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217710)
  %t1937.not = icmp eq i64 %v21068, 0
  br i1 %t1937.not, label %block16602, label %block16530

edge2328:                                         ; preds = %block16602
  %v21072 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217725)
  %t1939.not = icmp eq i64 %v21072, 0
  br i1 %t1939.not, label %block16606, label %block16530

edge2334:                                         ; preds = %block16606
  %v21076 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217738)
  %t1941.not = icmp eq i64 %v21076, 0
  br i1 %t1941.not, label %block16610, label %block16530

edge2340:                                         ; preds = %block16610
  %v21080 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217744)
  %t1943.not = icmp eq i64 %v21080, 0
  br i1 %t1943.not, label %block16614, label %block16530

edge2346:                                         ; preds = %block16614
  %v21084 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217754)
  %t1945.not = icmp eq i64 %v21084, 0
  br i1 %t1945.not, label %block16622, label %block16530

edge2358:                                         ; preds = %block16622
  %v21092 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t1949.not = icmp eq i64 %v21092, 0
  br i1 %t1949.not, label %block16626, label %block16530

edge2364:                                         ; preds = %block16626
  %v21096 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10789.ph.ph)
  %t1951.not = icmp eq i64 %v21096, 0
  br i1 %t1951.not, label %block16630, label %block16530

edge2370:                                         ; preds = %block16630
  %v21100 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10790)
  %t1953.not = icmp eq i64 %v21100, 0
  br i1 %t1953.not, label %block16634, label %block16530

edge2376:                                         ; preds = %block16634
  %v21104 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10791.ph.ph)
  %t1955.not = icmp eq i64 %v21104, 0
  br i1 %t1955.not, label %block16638, label %block16530

edge2382:                                         ; preds = %block16638
  %v21108 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10792)
  %t1957.not = icmp eq i64 %v21108, 0
  br i1 %t1957.not, label %block16642, label %block16530

edge2388:                                         ; preds = %block16642
  %v21112 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v19847)
  %t1959.not = icmp eq i64 %v21112, 0
  br i1 %t1959.not, label %block16646, label %block16530

edge2394:                                         ; preds = %block16646
  %v21116 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v19859)
  %t1961.not = icmp eq i64 %v21116, 0
  br i1 %t1961.not, label %block16650, label %block16530

edge2400:                                         ; preds = %block16650
  %v21120 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t1963.not = icmp eq i64 %v21120, 0
  br i1 %t1963.not, label %common.ret, label %block16530

edge2405:                                         ; preds = %edge2292
  br i1 %t23.not, label %edge2427.preheader, label %edge2407

edge2407:                                         ; preds = %edge2405
  %v21698 = load i64, ptr %t910, align 1
  %v21699 = add i64 %v21698, 1
  store i64 %v21699, ptr %t910, align 1
  br label %edge2427.preheader

edge2427.preheader:                               ; preds = %edge2405, %edge2407
  br label %edge2427

edge2427:                                         ; preds = %edge2427.preheader, %edge2571
  %v21748 = phi i64 [ %v21807, %edge2571 ], [ %v20966, %edge2427.preheader ]
  %v21807 = add nuw nsw i64 %v21748, 8
  %t1975.not = icmp ugt i64 %v21807, %spec.select17
  br i1 %t1975.not, label %block1017.preheader, label %edge2529

block1017.preheader:                              ; preds = %edge2427
  %umax = call i64 @llvm.umax.i64(i64 %spec.select17, i64 %v21748)
  br label %block1017

edge2451:                                         ; preds = %block1017
  br i1 %t23.not, label %edge2285, label %edge2453

edge2453:                                         ; preds = %edge2451
  %v22078 = load i64, ptr %t910, align 1
  %v22079 = add i64 %v22078, -1
  store i64 %v22079, ptr %t910, align 1
  %t1984.not = icmp eq i64 %v22079, 0
  br i1 %t1984.not, label %edge2456, label %edge2285

edge2456:                                         ; preds = %edge2453
  %v22080 = call i64 @mech_compact_free(i64 %v0, i64 %v1399)
  %t1985.not = icmp eq i64 %v22080, 0
  br i1 %t1985.not, label %edge2285, label %edge2457

edge2457:                                         ; preds = %edge2456
  %t1986 = insertvalue { i64, i64 } poison, i64 %v22080, 0
  %t1987 = insertvalue { i64, i64 } %t1986, i64 1032, 1
  br label %common.ret

edge2465:                                         ; preds = %block1017
  %v22268 = add i64 %v22057, %v217293
  %t1988 = icmp ult i64 %v22268, %v1395
  br i1 %t1988, label %edge2473, label %edge2471

edge2471:                                         ; preds = %edge2465, %block17131
  %v22736 = phi i64 [ %v22481, %block17131 ], [ 0, %edge2465 ]
  %v22505 = add i64 %v22057, %spec.select325
  %t1998 = icmp ult i64 %v22505, %v1395
  br i1 %t1998, label %edge2492, label %edge2490

edge2473:                                         ; preds = %edge2465
  br i1 %v1061851.not1217, label %edge2478, label %block17131

edge2478:                                         ; preds = %edge2473
  br i1 %t975, label %trap1995, label %ok1995

edge2490:                                         ; preds = %edge2471, %block17278
  %v22737 = phi i64 [ %v22714, %block17278 ], [ 0, %edge2471 ]
  %t2008 = icmp eq i64 %v22736, %v22737
  br i1 %t2008, label %edge2517, label %edge2506

edge2492:                                         ; preds = %edge2471
  br i1 %v1061851.not1217, label %edge2497, label %block17278

edge2497:                                         ; preds = %edge2492
  br i1 %t975, label %trap2005, label %ok2005

edge2506:                                         ; preds = %edge2490
  br i1 %t23.not, label %edge2285, label %edge2507

edge2507:                                         ; preds = %edge2506
  %v22746 = load i64, ptr %t910, align 1
  %v22747 = add i64 %v22746, -1
  store i64 %v22747, ptr %t910, align 1
  %t2014.not = icmp eq i64 %v22747, 0
  br i1 %t2014.not, label %edge2510, label %edge2285

edge2510:                                         ; preds = %edge2507
  %v22748 = call i64 @mech_compact_free(i64 %v0, i64 %v1399)
  %t2015.not = icmp eq i64 %v22748, 0
  br i1 %t2015.not, label %edge2285, label %edge2511

edge2511:                                         ; preds = %edge2510
  %t2016 = insertvalue { i64, i64 } poison, i64 %v22748, 0
  %t2017 = insertvalue { i64, i64 } %t2016, i64 1068, 1
  br label %common.ret

edge2517:                                         ; preds = %edge2490
  %v22920 = add nuw nsw i64 %v22057, 1
  br label %block1017

edge2529:                                         ; preds = %edge2427
  %v22965 = add i64 %v21748, %v217293
  %t2018.not = icmp ugt i64 %v22965, %v23157
  br i1 %t2018.not, label %edge2550, label %edge2541

edge2541:                                         ; preds = %edge2529
  br i1 %v1061851.not1217, label %edge2550.thread, label %edge2550.thread538

edge2550:                                         ; preds = %edge2529
  %v23755 = add i64 %v21748, %spec.select325
  %t2025.not = icmp ugt i64 %v23755, %v23157
  br i1 %t2025.not, label %edge2571, label %edge2562

edge2550.thread538:                               ; preds = %edge2541
  %v23348 = add i64 %v22965, %v1394
  %t2024 = inttoptr i64 %v23348 to ptr
  %v23349 = load i64, ptr %t2024, align 1
  %v23755540 = add i64 %v21748, %spec.select325
  %t2025.not541 = icmp ugt i64 %v23755540, %v23157
  br i1 %t2025.not541, label %edge2571, label %edge2563

edge2550.thread:                                  ; preds = %edge2541
  %v23351 = call i64 @mech_compact_matrix_word(i64 %v1394, i64 %v22965, i64 %v1397, i64 %v1398, i64 8)
  %v23755529 = add i64 %v21748, %spec.select325
  %t2025.not530 = icmp ugt i64 %v23755529, %v23157
  br i1 %t2025.not530, label %edge2571, label %edge2564

edge2562:                                         ; preds = %edge2550
  br i1 %v1061851.not1217, label %edge2564, label %edge2563

edge2563:                                         ; preds = %edge2550.thread538, %edge2562
  %v23731531546 = phi i64 [ 0, %edge2562 ], [ %v23349, %edge2550.thread538 ]
  %v23755533545 = phi i64 [ %v23755, %edge2562 ], [ %v23755540, %edge2550.thread538 ]
  %v24157 = add i64 %v23755533545, %v1394
  %t2031 = inttoptr i64 %v24157 to ptr
  %v24158 = load i64, ptr %t2031, align 1
  br label %edge2571

edge2564:                                         ; preds = %edge2550.thread, %edge2562
  %v23731531537 = phi i64 [ 0, %edge2562 ], [ %v23351, %edge2550.thread ]
  %v23755533536 = phi i64 [ %v23755, %edge2562 ], [ %v23755529, %edge2550.thread ]
  %v24160 = call i64 @mech_compact_matrix_word(i64 %v1394, i64 %v23755533536, i64 %v1397, i64 %v1398, i64 8)
  br label %edge2571

edge2571:                                         ; preds = %edge2550.thread538, %edge2550.thread, %edge2550, %edge2563, %edge2564
  %v23731532 = phi i64 [ %v23731531537, %edge2564 ], [ %v23731531546, %edge2563 ], [ 0, %edge2550 ], [ %v23351, %edge2550.thread ], [ %v23349, %edge2550.thread538 ]
  %v24544 = phi i64 [ %v24160, %edge2564 ], [ %v24158, %edge2563 ], [ 0, %edge2550 ], [ 0, %edge2550.thread ], [ 0, %edge2550.thread538 ]
  %t2032 = icmp eq i64 %v24544, %v23731532
  br i1 %t2032, label %edge2427, label %edge2574

edge2574:                                         ; preds = %edge2571
  %v24566.le = xor i64 %v24544, %v23731532
  br i1 %t23.not, label %edge2590, label %edge2577

edge2577:                                         ; preds = %edge2574
  %v24579 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t2035.not = icmp eq i64 %v24579, 0
  br i1 %t2035.not, label %edge2590, label %edge2578

edge2578:                                         ; preds = %edge2577
  %t2036 = insertvalue { i64, i64 } poison, i64 %v24579, 0
  %t2037 = insertvalue { i64, i64 } %t2036, i64 1217, 1
  br label %common.ret

edge2590:                                         ; preds = %edge2577, %edge2574
  %t2038 = call range(i64 0, 65) i64 @llvm.cttz.i64(i64 %v24566.le, i1 true)
  %v24704 = lshr i64 %t2038, 3
  %v24709 = add nuw nsw i64 %v24704, %v21748
  br label %edge2285

edge2605:                                         ; preds = %block18356, %edge2633
  %spec.select19 = add nuw nsw i64 %v19866, 1
  %t2087.not = icmp ult i64 %v24736, %spec.select17
  br i1 %t2087.not, label %edge2741, label %edge2671

edge2607:                                         ; preds = %edge2285
  %v24970 = sub i64 %v217293, %spec.select325
  %v24988 = and i64 %v24736, 65535
  %v25006 = shl i64 %v24970, 16
  %v25006.masked = and i64 %v25006, 4294901760
  %v25030 = or disjoint i64 %v24988, %v25006.masked
  %t2044 = icmp ult i64 %v19866, %v3718
  br i1 %t2044, label %edge2616, label %block18347

edge2616:                                         ; preds = %edge2607
  %v25135 = shl nuw nsw i64 %v19866, 2
  %v25136 = add i64 %v25135, %v19854
  %t2046 = inttoptr i64 %v25136 to ptr
  %t2047 = load i32, ptr %t2046, align 1
  %v25137 = zext i32 %t2047 to i64
  %t2048 = icmp eq i64 %v25030, %v25137
  br i1 %t2048, label %block18350, label %edge2619

edge2619:                                         ; preds = %edge2616
  %t2050.not = icmp eq i64 %v19859, 0
  br i1 %t2050.not, label %block18352, label %edge2620

edge2620:                                         ; preds = %edge2619
  %t2051 = inttoptr i64 %v19859 to ptr
  %v25153 = load i64, ptr %t2051, align 1
  %t2052 = icmp eq i64 %v25153, 1
  br i1 %t2052, label %block18351, label %block18352

edge2625:                                         ; preds = %block18352
  %v25163 = load i64, ptr %ss1, align 8
  %v25164 = load i64, ptr %t606, align 8
  %v25165 = load i64, ptr %t607, align 8
  %.pre20745 = add i64 %v25163, %v25135
  %.pre20746 = inttoptr i64 %.pre20745 to ptr
  br label %block18351

edge2628:                                         ; preds = %block18350
  %t2076 = inttoptr i64 %v25146 to ptr
  %v25170 = load i64, ptr %t2076, align 1
  %v25171 = add i64 %v25170, 1
  store i64 %v25171, ptr %t2076, align 1
  br label %block18356

edge2633:                                         ; preds = %block18356
  %v25176 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v19859)
  %t2080.not = icmp eq i64 %v25176, 0
  br i1 %t2080.not, label %edge2605, label %block18347

edge2671:                                         ; preds = %edge2605
  %v25800 = shl nuw nsw i64 %v19905, 2
  %v25816 = and i64 %v25800, 131068
  %v25840 = add i64 %v25816, %v19842
  %t2092 = inttoptr i64 %v25840 to ptr
  %t2093 = load i16, ptr %t2092, align 1
  %v25888 = add i64 %v19842, 2
  %v25889 = add i64 %v25888, %v25816
  %t2098 = inttoptr i64 %v25889 to ptr
  %t2099 = load i16, ptr %t2098, align 1
  %t2102.not = icmp eq i64 %v19847, 0
  br i1 %t2102.not, label %edge2692, label %edge2679

edge2679:                                         ; preds = %edge2671
  %t2104 = inttoptr i64 %v19847 to ptr
  %v25923 = load i64, ptr %t2104, align 1
  %t2105 = icmp eq i64 %v25923, 1
  %v25931 = shl nuw nsw i64 %v19899, 1
  %v25932 = add i64 %v25931, %v19842
  %t2109 = inttoptr i64 %v25932 to ptr
  br i1 %t2105, label %edge2683, label %edge2692.thread

edge2683:                                         ; preds = %edge2679
  store i16 %t2093, ptr %t2109, align 1
  %v25953 = shl nuw nsw i64 %v19902, 1
  %v25954 = add i64 %v25953, %v19842
  %t2112 = inttoptr i64 %v25954 to ptr
  store i16 %t2099, ptr %t2112, align 1
  br label %block19718

edge2692:                                         ; preds = %edge2671
  %v25973 = shl nuw nsw i64 %v19899, 1
  %v25974 = add i64 %v25973, %v19842
  %t2119 = inttoptr i64 %v25974 to ptr
  %t2120 = load i16, ptr %t2119, align 1
  %t2121 = icmp eq i16 %t2120, %t2093
  br i1 %t2121, label %block18804.thread, label %block18806

edge2692.thread:                                  ; preds = %edge2679
  %t2120566 = load i16, ptr %t2109, align 1
  %t2121567 = icmp eq i16 %t2120566, %t2093
  br i1 %t2121567, label %block18804.thread, label %block18806

edge2704:                                         ; preds = %block18804
  %t2149 = inttoptr i64 %v26003 to ptr
  %v26008 = load i64, ptr %t2149, align 1
  %v26009 = add i64 %v26008, 1
  store i64 %v26009, ptr %t2149, align 1
  br label %block18810

edge2709:                                         ; preds = %block18810
  %v26014 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v19847)
  %t2153.not = icmp eq i64 %v26014, 0
  br i1 %t2153.not, label %edge2715, label %block18781

edge2715:                                         ; preds = %block18810, %edge2709
  %v26051 = shl nuw nsw i64 %v19902, 1
  %v26052 = add i64 %v2597920855, %v26051
  %t2158 = inttoptr i64 %v26052 to ptr
  %t2159 = load i16, ptr %t2158, align 1
  %t2160 = icmp eq i16 %t2159, %t2099
  br i1 %t2160, label %block19718, label %edge2718

edge2718:                                         ; preds = %edge2715
  br i1 %t2148.not8620860, label %block18821, label %edge2719

edge2719:                                         ; preds = %edge2718
  %t2163 = inttoptr i64 %v2598420857 to ptr
  %v26069 = load i64, ptr %t2163, align 1
  %t2164 = icmp eq i64 %v26069, 1
  br i1 %t2164, label %block18819, label %block18821

edge2724:                                         ; preds = %block18821
  %v26079 = load i64, ptr %ss1, align 8
  %v26080 = load i64, ptr %t606, align 8
  %v26081 = load i64, ptr %t607, align 8
  %.pre20743 = add i64 %v26079, %v26051
  %.pre20744 = inttoptr i64 %.pre20743 to ptr
  br label %block18819

edge2727:                                         ; preds = %block18819
  %t2188 = inttoptr i64 %v26068 to ptr
  %v26086 = load i64, ptr %t2188, align 1
  %v26087 = add i64 %v26086, 1
  store i64 %v26087, ptr %t2188, align 1
  br label %block18825

edge2732:                                         ; preds = %block18825
  %v26092 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v2598420857)
  %t2192.not = icmp eq i64 %v26092, 0
  br i1 %t2192.not, label %block19718, label %block18816

edge2741:                                         ; preds = %edge2285, %edge2268, %edge2605
  %spec.select19562 = phi i64 [ %spec.select19, %edge2605 ], [ %v19866, %edge2268 ], [ %v19866, %edge2285 ]
  %v26106561 = phi i64 [ %v25146, %edge2605 ], [ %v19859, %edge2268 ], [ %v19859, %edge2285 ]
  %v26103560 = phi i64 [ %v25143, %edge2605 ], [ %v19856, %edge2268 ], [ %v19856, %edge2285 ]
  %v26101559 = phi i64 [ %v25141, %edge2605 ], [ %v19854, %edge2268 ], [ %v19854, %edge2285 ]
  %v26144558 = phi i64 [ %v24736, %edge2605 ], [ %v19920, %edge2268 ], [ %v19920, %edge2285 ]
  %v24736526557 = phi i64 [ %v24736, %edge2605 ], [ %v19917, %edge2268 ], [ %v24736, %edge2285 ]
  %v26178 = add i64 %v24736526557, %spec.select325
  %t2195 = icmp ult i64 %v26178, %v1395
  br i1 %t2195, label %edge2749, label %edge2747

edge2747:                                         ; preds = %edge2741, %block18987
  %v26680 = phi i64 [ %v26394, %block18987 ], [ 0, %edge2741 ]
  %v26454 = add i64 %v24736526557, %v217293
  %t2205 = icmp ult i64 %v26454, %v1395
  br i1 %t2205, label %edge2768, label %edge2766.thread

edge2766.thread:                                  ; preds = %edge2747
  %v26692574 = shl nuw nsw i64 %v19905, 1
  %v26697575 = and i64 %v26692574, 65534
  br label %edge2803

edge2749:                                         ; preds = %edge2741
  br i1 %v1061851.not1217, label %edge2754, label %block18987

edge2754:                                         ; preds = %edge2749
  br i1 %t975, label %trap2202, label %ok2202

edge2766:                                         ; preds = %ok2212, %edge2768
  %v26650 = phi i64 [ %v26655, %ok2212 ], [ %v26454, %edge2768 ]
  %v26657 = add i64 %v26650, %v1394
  %t2213 = inttoptr i64 %v26657 to ptr
  %t2214 = load i8, ptr %t2213, align 1
  %v26658 = zext i8 %t2214 to i64
  %14 = icmp samesign ult i64 %v26680, %v26658
  %v26692 = shl nuw nsw i64 %v19905, 1
  %v26697 = and i64 %v26692, 65534
  %v26702 = or disjoint i64 %v26697, 1
  br i1 %14, label %edge2803, label %edge2794

edge2768:                                         ; preds = %edge2747
  br i1 %v1061851.not1217, label %edge2773, label %edge2766

edge2773:                                         ; preds = %edge2768
  br i1 %t975, label %trap2212, label %ok2212

edge2794:                                         ; preds = %edge2766
  br label %edge2803

edge2803:                                         ; preds = %edge2766.thread, %edge2794, %edge2766
  %v26894 = phi i64 [ %v19899, %edge2766 ], [ %v19902, %edge2766.thread ], [ %v19902, %edge2794 ]
  %v26897 = phi i64 [ %v26702, %edge2766 ], [ %v26697575, %edge2766.thread ], [ %v26697, %edge2794 ]
  %v26900 = phi i64 [ %v26702, %edge2766 ], [ %v19899, %edge2766.thread ], [ %v19899, %edge2794 ]
  %v26903 = phi i64 [ %v19902, %edge2766 ], [ %v26697575, %edge2766.thread ], [ %v26697, %edge2794 ]
  %v26906 = phi i64 [ %v24736526557, %edge2766 ], [ %v19911, %edge2766.thread ], [ %v19911, %edge2794 ]
  %v26909 = phi i64 [ %v19914, %edge2766 ], [ %v24736526557, %edge2766.thread ], [ %v24736526557, %edge2794 ]
  %v26912 = phi i64 [ %v19914, %edge2766 ], [ %v19911, %edge2766.thread ], [ %v19911, %edge2794 ]
  %v26925 = shl nuw nsw i64 %v26894, 1
  %v26926 = add i64 %v26925, %v19842
  %t2220 = inttoptr i64 %v26926 to ptr
  %t2221 = load i16, ptr %t2220, align 1
  %v26927 = zext i16 %t2221 to i64
  %t2222 = icmp eq i64 %v19905, %v26927
  br i1 %t2222, label %block19287, label %edge2806

edge2806:                                         ; preds = %edge2803
  %t2224.not = icmp eq i64 %v19847, 0
  br i1 %t2224.not, label %block19289, label %edge2807

edge2807:                                         ; preds = %edge2806
  %t2225 = inttoptr i64 %v19847 to ptr
  %v26943 = load i64, ptr %t2225, align 1
  %t2226 = icmp eq i64 %v26943, 1
  br i1 %t2226, label %block19288, label %block19289

edge2812:                                         ; preds = %block19289
  %v26953 = load i64, ptr %ss1, align 8
  %v26954 = load i64, ptr %t606, align 8
  %v26955 = load i64, ptr %t607, align 8
  %.pre20741 = add i64 %v26953, %v26925
  %.pre20742 = inttoptr i64 %.pre20741 to ptr
  br label %block19288

edge2815:                                         ; preds = %block19287
  %t2250 = inttoptr i64 %v26936 to ptr
  %v26960 = load i64, ptr %t2250, align 1
  %v26961 = add i64 %v26960, 1
  store i64 %v26961, ptr %t2250, align 1
  br label %block19293

edge2820:                                         ; preds = %block19293
  %v26966 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v19847)
  %t2254.not = icmp eq i64 %v26966, 0
  br i1 %t2254.not, label %edge2826, label %block19284

edge2826:                                         ; preds = %block19293, %edge2820
  %v26977 = shl nuw nsw i64 %v26897, 1
  %v26978 = add i64 %v26931, %v26977
  %t2259 = inttoptr i64 %v26978 to ptr
  %t2260 = load i16, ptr %t2259, align 1
  %v26979 = zext i16 %t2260 to i64
  %spec.select20 = call i64 @llvm.umin.i64(i64 %v26912, i64 %v24736526557)
  %v27175 = add nsw i64 %v19908, -1
  %t2266 = icmp ult i64 %v12938, %v26979
  %t2267 = icmp ne i64 %v27175, 0
  %v2720881.not = and i1 %t2267, %t2266
  br i1 %v2720881.not, label %block913, label %edge2847

edge2847:                                         ; preds = %edge2826
  br i1 %t2249.not79, label %edge2862, label %edge2849

edge2849:                                         ; preds = %edge2847
  %t2272 = inttoptr i64 %v26936 to ptr
  %v27389 = load i64, ptr %t2272, align 1
  %t2273 = icmp eq i64 %v27389, 1
  %v27397 = shl nuw nsw i64 %v26900, 1
  %v27398 = add i64 %v26931, %v27397
  %t2277 = inttoptr i64 %v27398 to ptr
  br i1 %t2273, label %edge2853, label %edge2862.thread

edge2853:                                         ; preds = %edge2849
  store i16 0, ptr %t2277, align 1
  %v27419 = shl nuw nsw i64 %v26903, 1
  %v27420 = add i64 %v26931, %v27419
  %t2280 = inttoptr i64 %v27420 to ptr
  store i16 0, ptr %t2280, align 1
  br label %block19718

edge2862:                                         ; preds = %edge2847
  %v27439 = shl nuw nsw i64 %v26900, 1
  %v27440 = add i64 %v26931, %v27439
  %t2287 = inttoptr i64 %v27440 to ptr
  %t2288 = load i16, ptr %t2287, align 1
  %t2289 = icmp eq i16 %t2288, 0
  br i1 %t2289, label %edge2885.thread, label %block19595

edge2862.thread:                                  ; preds = %edge2849
  %t2288584 = load i16, ptr %t2277, align 1
  %t2289585 = icmp eq i16 %t2288584, 0
  br i1 %t2289585, label %block19593, label %block19595

edge2874:                                         ; preds = %block19593
  %t2317 = inttoptr i64 %v27450 to ptr
  %v27474 = load i64, ptr %t2317, align 1
  %v27475 = add i64 %v27474, 1
  store i64 %v27475, ptr %t2317, align 1
  br label %block19599

edge2879:                                         ; preds = %block19599
  %v27480 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v26936)
  %t2321.not = icmp eq i64 %v27480, 0
  br i1 %t2321.not, label %edge2885, label %block19570

edge2885:                                         ; preds = %block19599, %edge2879
  %v27506 = shl nuw nsw i64 %v26903, 1
  %v27507 = add i64 %v27445, %v27506
  %t2326 = inttoptr i64 %v27507 to ptr
  %t2327 = load i16, ptr %t2326, align 1
  %t2328 = icmp eq i16 %t2327, 0
  br i1 %t2328, label %block19718, label %edge2888

edge2885.thread:                                  ; preds = %edge2862
  %v2750621049 = shl nuw nsw i64 %v26903, 1
  %v2750721050 = add i64 %v26931, %v2750621049
  %t232621051 = inttoptr i64 %v2750721050 to ptr
  %t232721052 = load i16, ptr %t232621051, align 1
  %t232821053 = icmp eq i16 %t232721052, 0
  br i1 %t232821053, label %block19718, label %block19610

edge2888:                                         ; preds = %edge2885
  br i1 %t2316.not82, label %block19610, label %edge2889

edge2889:                                         ; preds = %edge2888
  %t2331 = inttoptr i64 %v27450 to ptr
  %v27524 = load i64, ptr %t2331, align 1
  %t2332 = icmp eq i64 %v27524, 1
  br i1 %t2332, label %block19614.thread21246, label %block19610

block19614.thread21246:                           ; preds = %edge2889
  store i16 0, ptr %t2326, align 1
  br label %block19718

edge2897:                                         ; preds = %block19608
  %t2356 = inttoptr i64 %v27536 to ptr
  %v27541 = load i64, ptr %t2356, align 1
  %v27542 = add i64 %v27541, 1
  store i64 %v27542, ptr %t2356, align 1
  br label %block19614

edge2902:                                         ; preds = %block19614
  %v27547 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v2745020887208982105621069)
  %t2360.not = icmp eq i64 %v27547, 0
  br i1 %t2360.not, label %block19718, label %block19605

edge2952:                                         ; preds = %block19718
  %v27751 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t2366.not = icmp eq i64 %v27751, 0
  br i1 %t2366.not, label %edge3980, label %edge2953

edge2953:                                         ; preds = %edge2952
  %t2367 = insertvalue { i64, i64 } poison, i64 %v27751, 0
  %t2368 = insertvalue { i64, i64 } %t2367, i64 1403, 1
  br label %common.ret

edge3097:                                         ; preds = %edge2178
  br i1 %t23.not, label %edge3105, label %edge3098

edge3098:                                         ; preds = %edge3097
  %v28497 = load i64, ptr %t910, align 1
  %v28498 = add i64 %v28497, 1
  store i64 %v28498, ptr %t910, align 1
  br label %edge3105

edge3105:                                         ; preds = %edge3097, %edge3098
  %v29010 = and i64 %v217297, 65535
  %v12722 = lshr i64 %v217297, 16
  %v29017 = and i64 %v12722, 65535
  %t2418 = icmp samesign ugt i64 %v29017, 1
  %v29212 = add nsw i64 %v29017, -1
  %v29406 = call i64 @llvm.umin.i64(i64 %spec.select17, i64 %v29212)
  br label %block1421

edge3132:                                         ; preds = %block1421
  %v28793 = sub nsw i64 %v12938, %v28580
  %v28799 = and i64 %v28793, 32767
  %spec.select326 = call i64 @llvm.usub.sat.i64(i64 %v217293, i64 %v28799)
  %v29004 = sub i64 %v217293, %spec.select326
  %t2417 = icmp eq i64 %v29004, %v29010
  %v2903560 = and i1 %t2418, %t2417
  %t24241221 = icmp ugt i64 %v29212, %v28592
  %t2424 = select i1 %v2903560, i1 %t24241221, i1 false
  br i1 %t2424, label %edge3233, label %edge3178

edge3178:                                         ; preds = %edge3132
  %v29612 = add i64 %v28592, %spec.select326
  %t2427 = icmp ult i64 %v29612, %v1395
  br i1 %t2427, label %edge3186, label %edge3184

edge3184:                                         ; preds = %edge3178, %block20808
  %v30064 = phi i64 [ %v29816, %block20808 ], [ 0, %edge3178 ]
  %v29838 = add i64 %v28592, %v217293
  %t2437 = icmp ult i64 %v29838, %v1395
  br i1 %t2437, label %edge3205, label %edge3203

edge3186:                                         ; preds = %edge3178
  br i1 %v1061851.not1217, label %edge3191, label %block20808

edge3191:                                         ; preds = %edge3186
  br i1 %t975, label %trap2434, label %ok2434

edge3203:                                         ; preds = %edge3184, %block20951
  %v30065 = phi i64 [ %v30042, %block20951 ], [ 0, %edge3184 ]
  %t2447.not = icmp eq i64 %v30064, %v30065
  %v30257 = add nuw nsw i64 %v28592, 1
  br i1 %t2447.not, label %edge3233, label %block1799

edge3205:                                         ; preds = %edge3184
  br i1 %v1061851.not1217, label %edge3210, label %block20951

edge3210:                                         ; preds = %edge3205
  br i1 %t975, label %trap2444, label %ok2444

edge3233:                                         ; preds = %edge3132, %edge3203
  %v217330 = phi i64 [ %v29406, %edge3132 ], [ %v30257, %edge3203 ]
  %t2455 = icmp ult i64 %spec.select326, %v217293
  br i1 %t2455, label %edge3346, label %edge3254

edge3254:                                         ; preds = %edge3233
  br i1 %t23.not, label %block21296, label %edge3257

edge3257:                                         ; preds = %edge3254
  %v30530 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t2458.not = icmp eq i64 %v30530, 0
  br i1 %t2458.not, label %block21296, label %block21228

edge3263:                                         ; preds = %block21296
  %v30534 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217710)
  %t2460.not = icmp eq i64 %v30534, 0
  br i1 %t2460.not, label %block21300, label %block21228

edge3269:                                         ; preds = %block21300
  %v30538 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217725)
  %t2462.not = icmp eq i64 %v30538, 0
  br i1 %t2462.not, label %block21304, label %block21228

edge3275:                                         ; preds = %block21304
  %v30542 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217738)
  %t2464.not = icmp eq i64 %v30542, 0
  br i1 %t2464.not, label %block21308, label %block21228

edge3281:                                         ; preds = %block21308
  %v30546 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217744)
  %t2466.not = icmp eq i64 %v30546, 0
  br i1 %t2466.not, label %block21312, label %block21228

edge3287:                                         ; preds = %block21312
  %v30550 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217754)
  %t2468.not = icmp eq i64 %v30550, 0
  br i1 %t2468.not, label %block21320, label %block21228

edge3299:                                         ; preds = %block21320
  %v30558 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t2472.not = icmp eq i64 %v30558, 0
  br i1 %t2472.not, label %block21324, label %block21228

edge3305:                                         ; preds = %block21324
  %v30562 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10789.ph.ph)
  %t2474.not = icmp eq i64 %v30562, 0
  br i1 %t2474.not, label %block21328, label %block21228

edge3311:                                         ; preds = %block21328
  %v30566 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10790)
  %t2476.not = icmp eq i64 %v30566, 0
  br i1 %t2476.not, label %block21332, label %block21228

edge3317:                                         ; preds = %block21332
  %v30570 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10791.ph.ph)
  %t2478.not = icmp eq i64 %v30570, 0
  br i1 %t2478.not, label %block21336, label %block21228

edge3323:                                         ; preds = %block21336
  %v30574 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10792)
  %t2480.not = icmp eq i64 %v30574, 0
  br i1 %t2480.not, label %block21340, label %block21228

edge3329:                                         ; preds = %block21340
  %v30578 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v18649)
  %t2482.not = icmp eq i64 %v30578, 0
  br i1 %t2482.not, label %block21344, label %block21228

edge3335:                                         ; preds = %block21344
  %v30582 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v28546)
  %t2484.not = icmp eq i64 %v30582, 0
  br i1 %t2484.not, label %block21348, label %block21228

edge3341:                                         ; preds = %block21348
  %v30586 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t2486.not = icmp eq i64 %v30586, 0
  br i1 %t2486.not, label %common.ret, label %block21228

edge3346:                                         ; preds = %edge3233
  br i1 %t23.not, label %edge3368.preheader, label %edge3348

edge3348:                                         ; preds = %edge3346
  %v30597 = load i64, ptr %t910, align 1
  %v30598 = add i64 %v30597, 1
  store i64 %v30598, ptr %t910, align 1
  br label %edge3368.preheader

edge3368.preheader:                               ; preds = %edge3346, %edge3348
  br label %edge3368

edge3368:                                         ; preds = %edge3368.preheader, %edge3512
  %v30647 = phi i64 [ %v30661, %edge3512 ], [ %v217330, %edge3368.preheader ]
  %v30661 = add nuw nsw i64 %v30647, 8
  %t2498.not = icmp ugt i64 %v30661, %spec.select17
  br i1 %t2498.not, label %block1561, label %edge3470

edge3392:                                         ; preds = %block1561
  %umax20578.le = call i64 @llvm.umax.i64(i64 %spec.select17, i64 %v30647)
  br i1 %t23.not, label %edge3550, label %edge3394

edge3394:                                         ; preds = %edge3392
  %v30923 = load i64, ptr %t910, align 1
  %v30924 = add i64 %v30923, -1
  store i64 %v30924, ptr %t910, align 1
  %t2507.not = icmp eq i64 %v30924, 0
  br i1 %t2507.not, label %edge3397, label %edge3550

edge3397:                                         ; preds = %edge3394
  %v30925 = call i64 @mech_compact_free(i64 %v0, i64 %v1399)
  %t2508.not = icmp eq i64 %v30925, 0
  br i1 %t2508.not, label %edge3550, label %edge3398

edge3398:                                         ; preds = %edge3397
  %t2509 = insertvalue { i64, i64 } poison, i64 %v30925, 0
  %t2510 = insertvalue { i64, i64 } %t2509, i64 1580, 1
  br label %common.ret

edge3406:                                         ; preds = %block1561
  %v31113 = add i64 %v30902, %v217293
  %t2511 = icmp ult i64 %v31113, %v1395
  br i1 %t2511, label %edge3414, label %edge3412

edge3412:                                         ; preds = %edge3406, %block21829
  %v31581 = phi i64 [ %v31326, %block21829 ], [ 0, %edge3406 ]
  %v31350 = add i64 %v30902, %spec.select326
  %t2521 = icmp ult i64 %v31350, %v1395
  br i1 %t2521, label %edge3433, label %edge3431

edge3414:                                         ; preds = %edge3406
  br i1 %v1061851.not1217, label %edge3419, label %block21829

edge3419:                                         ; preds = %edge3414
  br i1 %t975, label %trap2518, label %ok2518

edge3431:                                         ; preds = %edge3412, %block21976
  %v31582 = phi i64 [ %v31559, %block21976 ], [ 0, %edge3412 ]
  %t2531 = icmp eq i64 %v31581, %v31582
  br i1 %t2531, label %edge3458, label %edge3447

edge3433:                                         ; preds = %edge3412
  br i1 %v1061851.not1217, label %edge3438, label %block21976

edge3438:                                         ; preds = %edge3433
  br i1 %t975, label %trap2528, label %ok2528

edge3447:                                         ; preds = %edge3431
  br i1 %t23.not, label %edge3539, label %edge3448

edge3448:                                         ; preds = %edge3447
  %v31591 = load i64, ptr %t910, align 1
  %v31592 = add i64 %v31591, -1
  store i64 %v31592, ptr %t910, align 1
  %t2537.not = icmp eq i64 %v31592, 0
  br i1 %t2537.not, label %edge3451, label %edge3539

edge3451:                                         ; preds = %edge3448
  %v31593 = call i64 @mech_compact_free(i64 %v0, i64 %v1399)
  %t2538.not = icmp eq i64 %v31593, 0
  br i1 %t2538.not, label %edge3539, label %edge3452

edge3452:                                         ; preds = %edge3451
  %t2539 = insertvalue { i64, i64 } poison, i64 %v31593, 0
  %t2540 = insertvalue { i64, i64 } %t2539, i64 1616, 1
  br label %common.ret

edge3458:                                         ; preds = %edge3431
  %v31765 = add nuw nsw i64 %v30902, 1
  br label %block1561

edge3470:                                         ; preds = %edge3368
  %v31810 = add i64 %v30647, %v217293
  %t2541.not = icmp ugt i64 %v31810, %v23157
  br i1 %t2541.not, label %edge3491, label %edge3482

edge3482:                                         ; preds = %edge3470
  br i1 %v1061851.not1217, label %edge3491.thread, label %edge3491.thread601

edge3491:                                         ; preds = %edge3470
  %v32600 = add i64 %v30647, %spec.select326
  %t2548.not = icmp ugt i64 %v32600, %v23157
  br i1 %t2548.not, label %edge3512, label %edge3503

edge3491.thread601:                               ; preds = %edge3482
  %v32193 = add i64 %v31810, %v1394
  %t2547 = inttoptr i64 %v32193 to ptr
  %v32194 = load i64, ptr %t2547, align 1
  %v32600603 = add i64 %v30647, %spec.select326
  %t2548.not604 = icmp ugt i64 %v32600603, %v23157
  br i1 %t2548.not604, label %edge3512, label %edge3504

edge3491.thread:                                  ; preds = %edge3482
  %v32196 = call i64 @mech_compact_matrix_word(i64 %v1394, i64 %v31810, i64 %v1397, i64 %v1398, i64 8)
  %v32600592 = add i64 %v30647, %spec.select326
  %t2548.not593 = icmp ugt i64 %v32600592, %v23157
  br i1 %t2548.not593, label %edge3512, label %edge3505

edge3503:                                         ; preds = %edge3491
  br i1 %v1061851.not1217, label %edge3505, label %edge3504

edge3504:                                         ; preds = %edge3491.thread601, %edge3503
  %v32576594609 = phi i64 [ 0, %edge3503 ], [ %v32194, %edge3491.thread601 ]
  %v32600596608 = phi i64 [ %v32600, %edge3503 ], [ %v32600603, %edge3491.thread601 ]
  %v33002 = add i64 %v32600596608, %v1394
  %t2554 = inttoptr i64 %v33002 to ptr
  %v33003 = load i64, ptr %t2554, align 1
  br label %edge3512

edge3505:                                         ; preds = %edge3491.thread, %edge3503
  %v32576594600 = phi i64 [ 0, %edge3503 ], [ %v32196, %edge3491.thread ]
  %v32600596599 = phi i64 [ %v32600, %edge3503 ], [ %v32600592, %edge3491.thread ]
  %v33005 = call i64 @mech_compact_matrix_word(i64 %v1394, i64 %v32600596599, i64 %v1397, i64 %v1398, i64 8)
  br label %edge3512

edge3512:                                         ; preds = %edge3491.thread601, %edge3491.thread, %edge3491, %edge3504, %edge3505
  %v32576595 = phi i64 [ %v32576594600, %edge3505 ], [ %v32576594609, %edge3504 ], [ 0, %edge3491 ], [ %v32196, %edge3491.thread ], [ %v32194, %edge3491.thread601 ]
  %v33389 = phi i64 [ %v33005, %edge3505 ], [ %v33003, %edge3504 ], [ 0, %edge3491 ], [ 0, %edge3491.thread ], [ 0, %edge3491.thread601 ]
  %t2555 = icmp eq i64 %v33389, %v32576595
  br i1 %t2555, label %edge3368, label %edge3515

edge3515:                                         ; preds = %edge3512
  %v33411.le = xor i64 %v33389, %v32576595
  br i1 %t23.not, label %edge3531, label %edge3518

edge3518:                                         ; preds = %edge3515
  %v33424 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t2558.not = icmp eq i64 %v33424, 0
  br i1 %t2558.not, label %edge3531, label %edge3519

edge3519:                                         ; preds = %edge3518
  %t2559 = insertvalue { i64, i64 } poison, i64 %v33424, 0
  %t2560 = insertvalue { i64, i64 } %t2559, i64 1765, 1
  br label %common.ret

edge3531:                                         ; preds = %edge3518, %edge3515
  %t2561 = call range(i64 0, 65) i64 @llvm.cttz.i64(i64 %v33411.le, i1 true)
  %v33549 = lshr i64 %t2561, 3
  %v33554 = add nuw nsw i64 %v33549, %v30647
  br label %edge3539

edge3539:                                         ; preds = %edge3451, %edge3448, %edge3447, %edge3531
  %v33561 = phi i64 [ %v33554, %edge3531 ], [ %v30902, %edge3447 ], [ %v30902, %edge3448 ], [ %v30902, %edge3451 ]
  %t2563.not = icmp ult i64 %v33561, %spec.select17
  br i1 %t2563.not, label %edge3539.block1799_crit_edge, label %edge3550

edge3539.block1799_crit_edge:                     ; preds = %edge3539
  %.pre20758 = add i64 %v33561, %spec.select326
  br label %block1799

edge3550:                                         ; preds = %edge3539, %edge3397, %edge3394, %edge3392
  %v3356121075 = phi i64 [ %umax20578.le, %edge3397 ], [ %umax20578.le, %edge3392 ], [ %umax20578.le, %edge3394 ], [ %v33561, %edge3539 ]
  %v34163 = shl nuw nsw i64 %v28580, 2
  %v34179 = and i64 %v34163, 131068
  %v34205 = add i64 %v34179, %v28541
  %t2568 = inttoptr i64 %v34205 to ptr
  %t2569 = load i16, ptr %t2568, align 1
  %v34253 = add i64 %v28541, 2
  %v34254 = add i64 %v34253, %v34179
  %t2574 = inttoptr i64 %v34254 to ptr
  %t2575 = load i16, ptr %t2574, align 1
  %t2578.not = icmp eq i64 %v28546, 0
  br i1 %t2578.not, label %edge3571, label %edge3558

edge3558:                                         ; preds = %edge3550
  %t2580 = inttoptr i64 %v28546 to ptr
  %v34276 = load i64, ptr %t2580, align 1
  %t2581 = icmp eq i64 %v34276, 1
  %v34284 = shl nuw nsw i64 %v28574, 1
  %v34285 = add i64 %v34284, %v28541
  %t2585 = inttoptr i64 %v34285 to ptr
  br i1 %t2581, label %edge3562, label %edge3571.thread

edge3562:                                         ; preds = %edge3558
  store i16 %t2569, ptr %t2585, align 1
  %v34306 = shl nuw nsw i64 %v28577, 1
  %v34307 = add i64 %v34306, %v28541
  %t2588 = inttoptr i64 %v34307 to ptr
  store i16 %t2575, ptr %t2588, align 1
  br label %block1793

edge3571:                                         ; preds = %edge3550
  %v34326 = shl nuw nsw i64 %v28574, 1
  %v34327 = add i64 %v34326, %v28541
  %t2595 = inttoptr i64 %v34327 to ptr
  %t2596 = load i16, ptr %t2595, align 1
  %t2597 = icmp eq i16 %t2596, %t2569
  br i1 %t2597, label %block23070.thread, label %block23072

edge3571.thread:                                  ; preds = %edge3558
  %t2596613 = load i16, ptr %t2585, align 1
  %t2597614 = icmp eq i16 %t2596613, %t2569
  br i1 %t2597614, label %block23070.thread, label %block23072

edge3583:                                         ; preds = %block23070
  %t2625 = inttoptr i64 %v34356 to ptr
  %v34361 = load i64, ptr %t2625, align 1
  %v34362 = add i64 %v34361, 1
  store i64 %v34362, ptr %t2625, align 1
  br label %block23076

edge3588:                                         ; preds = %block23076
  %v34367 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v28546)
  %t2629.not = icmp eq i64 %v34367, 0
  br i1 %t2629.not, label %edge3594, label %block23047

edge3594:                                         ; preds = %block23076, %edge3588
  %v34404 = shl nuw nsw i64 %v28577, 1
  %v34405 = add i64 %v3433220927, %v34404
  %t2634 = inttoptr i64 %v34405 to ptr
  %t2635 = load i16, ptr %t2634, align 1
  %t2636 = icmp eq i16 %t2635, %t2575
  br i1 %t2636, label %block1793, label %edge3597

edge3597:                                         ; preds = %edge3594
  br i1 %t2624.not7020932, label %block23087, label %edge3598

edge3598:                                         ; preds = %edge3597
  %t2639 = inttoptr i64 %v3433720929 to ptr
  %v34422 = load i64, ptr %t2639, align 1
  %t2640 = icmp eq i64 %v34422, 1
  br i1 %t2640, label %block23085, label %block23087

edge3603:                                         ; preds = %block23087
  %v34432 = load i64, ptr %ss1, align 8
  %v34433 = load i64, ptr %t606, align 8
  %v34434 = load i64, ptr %t607, align 8
  %.pre20737 = add i64 %v34432, %v34404
  %.pre20738 = inttoptr i64 %.pre20737 to ptr
  br label %block23085

edge3606:                                         ; preds = %block23085
  %t2664 = inttoptr i64 %v34421 to ptr
  %v34439 = load i64, ptr %t2664, align 1
  %v34440 = add i64 %v34439, 1
  store i64 %v34440, ptr %t2664, align 1
  br label %block23091

edge3611:                                         ; preds = %block23091
  %v34445 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v3433720929)
  %t2668.not = icmp eq i64 %v34445, 0
  br i1 %t2668.not, label %block1793, label %block23082

edge3627:                                         ; preds = %block1799, %block23247
  %v35072 = phi i64 [ %v34774, %block23247 ], [ 0, %block1799 ]
  %v34846 = add i64 %v217331, %v217293
  %t2682 = icmp ult i64 %v34846, %v1395
  br i1 %t2682, label %edge3648, label %edge3646.thread

edge3646.thread:                                  ; preds = %edge3627
  %v35084621 = shl nuw nsw i64 %v28580, 1
  %v35089622 = and i64 %v35084621, 65534
  br label %edge3683

edge3629:                                         ; preds = %block1799
  br i1 %v1061851.not1217, label %edge3634, label %block23247

edge3634:                                         ; preds = %edge3629
  br i1 %t975, label %trap2679, label %ok2679

edge3646:                                         ; preds = %ok2689, %edge3648
  %v35042 = phi i64 [ %v35047, %ok2689 ], [ %v34846, %edge3648 ]
  %v35049 = add i64 %v35042, %v1394
  %t2690 = inttoptr i64 %v35049 to ptr
  %t2691 = load i8, ptr %t2690, align 1
  %v35050 = zext i8 %t2691 to i64
  %15 = icmp samesign ult i64 %v35072, %v35050
  %v35084 = shl nuw nsw i64 %v28580, 1
  %v35089 = and i64 %v35084, 65534
  %v35094 = or disjoint i64 %v35089, 1
  br i1 %15, label %edge3683, label %edge3674

edge3648:                                         ; preds = %edge3627
  br i1 %v1061851.not1217, label %edge3653, label %edge3646

edge3653:                                         ; preds = %edge3648
  br i1 %t975, label %trap2689, label %ok2689

edge3674:                                         ; preds = %edge3646
  br label %edge3683

edge3683:                                         ; preds = %edge3646.thread, %edge3674, %edge3646
  %v35286 = phi i64 [ %v28574, %edge3646 ], [ %v28577, %edge3646.thread ], [ %v28577, %edge3674 ]
  %v35289 = phi i64 [ %v35094, %edge3646 ], [ %v35089622, %edge3646.thread ], [ %v35089, %edge3674 ]
  %v35292 = phi i64 [ %v35094, %edge3646 ], [ %v28574, %edge3646.thread ], [ %v28574, %edge3674 ]
  %v35295 = phi i64 [ %v28577, %edge3646 ], [ %v35089622, %edge3646.thread ], [ %v35089, %edge3674 ]
  %v35298 = phi i64 [ %v217331, %edge3646 ], [ %v28586, %edge3646.thread ], [ %v28586, %edge3674 ]
  %v35301 = phi i64 [ %v28589, %edge3646 ], [ %v217331, %edge3646.thread ], [ %v217331, %edge3674 ]
  %v35304 = phi i64 [ %v28589, %edge3646 ], [ %v28586, %edge3646.thread ], [ %v28586, %edge3674 ]
  %v35317 = shl nuw nsw i64 %v35286, 1
  %v35318 = add i64 %v35317, %v28541
  %t2697 = inttoptr i64 %v35318 to ptr
  %t2698 = load i16, ptr %t2697, align 1
  %v35319 = zext i16 %t2698 to i64
  %t2699 = icmp eq i64 %v28580, %v35319
  br i1 %t2699, label %block23547, label %edge3686

edge3686:                                         ; preds = %edge3683
  %t2701.not = icmp eq i64 %v28546, 0
  br i1 %t2701.not, label %block23549, label %edge3687

edge3687:                                         ; preds = %edge3686
  %t2702 = inttoptr i64 %v28546 to ptr
  %v35335 = load i64, ptr %t2702, align 1
  %t2703 = icmp eq i64 %v35335, 1
  br i1 %t2703, label %block23548, label %block23549

edge3692:                                         ; preds = %block23549
  %v35345 = load i64, ptr %ss1, align 8
  %v35346 = load i64, ptr %t606, align 8
  %v35347 = load i64, ptr %t607, align 8
  %.pre20735 = add i64 %v35345, %v35317
  %.pre20736 = inttoptr i64 %.pre20735 to ptr
  br label %block23548

edge3695:                                         ; preds = %block23547
  %t2727 = inttoptr i64 %v35328 to ptr
  %v35352 = load i64, ptr %t2727, align 1
  %v35353 = add i64 %v35352, 1
  store i64 %v35353, ptr %t2727, align 1
  br label %block23553

edge3700:                                         ; preds = %block23553
  %v35358 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v28546)
  %t2731.not = icmp eq i64 %v35358, 0
  br i1 %t2731.not, label %edge3706, label %block23544

edge3706:                                         ; preds = %block23553, %edge3700
  %v35369 = shl nuw nsw i64 %v35289, 1
  %v35370 = add i64 %v35323, %v35369
  %t2736 = inttoptr i64 %v35370 to ptr
  %t2737 = load i16, ptr %t2736, align 1
  %v35371 = zext i16 %t2737 to i64
  %spec.select21 = call i64 @llvm.umin.i64(i64 %v35304, i64 %v217331)
  %v35567 = add nsw i64 %v28583, -1
  %t2743 = icmp ult i64 %v12938, %v35371
  %t2744 = icmp ne i64 %v35567, 0
  %v3560065.not = and i1 %t2744, %t2743
  br i1 %v3560065.not, label %block1421, label %edge3727

edge3727:                                         ; preds = %edge3706
  br i1 %t2726.not63, label %edge3742, label %edge3729

edge3729:                                         ; preds = %edge3727
  %t2749 = inttoptr i64 %v35328 to ptr
  %v35781 = load i64, ptr %t2749, align 1
  %t2750 = icmp eq i64 %v35781, 1
  %v35789 = shl nuw nsw i64 %v35292, 1
  %v35790 = add i64 %v35323, %v35789
  %t2754 = inttoptr i64 %v35790 to ptr
  br i1 %t2750, label %edge3733, label %edge3742.thread

edge3733:                                         ; preds = %edge3729
  store i16 0, ptr %t2754, align 1
  %v35811 = shl nuw nsw i64 %v35295, 1
  %v35812 = add i64 %v35323, %v35811
  %t2757 = inttoptr i64 %v35812 to ptr
  store i16 0, ptr %t2757, align 1
  br label %block23952

edge3742:                                         ; preds = %edge3727
  %v35831 = shl nuw nsw i64 %v35292, 1
  %v35832 = add i64 %v35323, %v35831
  %t2764 = inttoptr i64 %v35832 to ptr
  %t2765 = load i16, ptr %t2764, align 1
  %t2766 = icmp eq i16 %t2765, 0
  br i1 %t2766, label %edge3765.thread, label %block23855

edge3742.thread:                                  ; preds = %edge3729
  %t2765631 = load i16, ptr %t2754, align 1
  %t2766632 = icmp eq i16 %t2765631, 0
  br i1 %t2766632, label %block23853, label %block23855

edge3754:                                         ; preds = %block23853
  %t2794 = inttoptr i64 %v35842 to ptr
  %v35866 = load i64, ptr %t2794, align 1
  %v35867 = add i64 %v35866, 1
  store i64 %v35867, ptr %t2794, align 1
  br label %block23859

edge3759:                                         ; preds = %block23859
  %v35872 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v35328)
  %t2798.not = icmp eq i64 %v35872, 0
  br i1 %t2798.not, label %edge3765, label %block23830

edge3765:                                         ; preds = %block23859, %edge3759
  %v35898 = shl nuw nsw i64 %v35295, 1
  %v35899 = add i64 %v35837, %v35898
  %t2803 = inttoptr i64 %v35899 to ptr
  %t2804 = load i16, ptr %t2803, align 1
  %t2805 = icmp eq i16 %t2804, 0
  br i1 %t2805, label %block23952, label %edge3768

edge3765.thread:                                  ; preds = %edge3742
  %v3589821080 = shl nuw nsw i64 %v35295, 1
  %v3589921081 = add i64 %v35323, %v3589821080
  %t280321082 = inttoptr i64 %v3589921081 to ptr
  %t280421083 = load i16, ptr %t280321082, align 1
  %t280521084 = icmp eq i16 %t280421083, 0
  br i1 %t280521084, label %block23952, label %block23870

edge3768:                                         ; preds = %edge3765
  br i1 %t2793.not66, label %block23870, label %edge3769

edge3769:                                         ; preds = %edge3768
  %t2808 = inttoptr i64 %v35842 to ptr
  %v35916 = load i64, ptr %t2808, align 1
  %t2809 = icmp eq i64 %v35916, 1
  br i1 %t2809, label %block23874.thread21265, label %block23870

block23874.thread21265:                           ; preds = %edge3769
  store i16 0, ptr %t2803, align 1
  br label %block23952

edge3777:                                         ; preds = %block23868
  %t2833 = inttoptr i64 %v35928 to ptr
  %v35933 = load i64, ptr %t2833, align 1
  %v35934 = add i64 %v35933, 1
  store i64 %v35934, ptr %t2833, align 1
  br label %block23874

edge3782:                                         ; preds = %block23874
  %v35939 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v3584220959209702108721100)
  %t2837.not = icmp eq i64 %v35939, 0
  br i1 %t2837.not, label %block23952, label %block23865

edge3826:                                         ; preds = %block23952
  %v36109 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t2843.not = icmp eq i64 %v36109, 0
  br i1 %t2843.not, label %edge3980, label %edge3827

edge3827:                                         ; preds = %edge3826
  %t2844 = insertvalue { i64, i64 } poison, i64 %v36109, 0
  %t2845 = insertvalue { i64, i64 } %t2844, i64 1916, 1
  br label %common.ret

edge3980.sink.split:                              ; preds = %edge2154, %edge2118
  %t1852.sink = phi ptr [ %t1806, %edge2118 ], [ %t1852, %edge2154 ]
  %v36623.ph = phi i64 [ %v18873, %edge2118 ], [ %v19252, %edge2154 ]
  %v36625.ph = phi i64 [ %v18875, %edge2118 ], [ %v19254, %edge2154 ]
  %v36628.ph = phi i64 [ %v18878, %edge2118 ], [ %v19257, %edge2154 ]
  store i16 0, ptr %t1852.sink, align 1
  br label %edge3980

edge3980:                                         ; preds = %edge3980.sink.split, %edge2150.thread, %edge2150, %block23952, %edge3826, %block19718, %edge2952, %edge2167, %block15616
  %v36623 = phi i64 [ %v217784, %block19718 ], [ %v19375, %block15616 ], [ %v18873, %edge2150.thread ], [ %v19375, %edge2167 ], [ %v217792, %block23952 ], [ %v217784, %edge2952 ], [ %v217792, %edge3826 ], [ %v19252, %edge2150 ], [ %v36623.ph, %edge3980.sink.split ]
  %v36625 = phi i64 [ %v217785, %block19718 ], [ %v19376, %block15616 ], [ %v18875, %edge2150.thread ], [ %v19376, %edge2167 ], [ %v217793, %block23952 ], [ %v217785, %edge2952 ], [ %v217793, %edge3826 ], [ %v19254, %edge2150 ], [ %v36625.ph, %edge3980.sink.split ]
  %v36628 = phi i64 [ %v217786, %block19718 ], [ %v19377, %block15616 ], [ 0, %edge2150.thread ], [ %v19377, %edge2167 ], [ %v217794, %block23952 ], [ %v217786, %edge2952 ], [ %v217794, %edge3826 ], [ %v19257, %edge2150 ], [ %v36628.ph, %edge3980.sink.split ]
  %v36635 = phi i64 [ %v217787, %block19718 ], [ %v18644, %block15616 ], [ %v18644, %edge2150.thread ], [ %v18644, %edge2167 ], [ %v18644, %block23952 ], [ %v217787, %edge2952 ], [ %v18644, %edge3826 ], [ %v18644, %edge2150 ], [ %v18644, %edge3980.sink.split ]
  %v36637 = phi i64 [ %v217788, %block19718 ], [ %v18646, %block15616 ], [ %v18646, %edge2150.thread ], [ %v18646, %edge2167 ], [ %v18646, %block23952 ], [ %v217788, %edge2952 ], [ %v18646, %edge3826 ], [ %v18646, %edge2150 ], [ %v18646, %edge3980.sink.split ]
  %v36640 = phi i64 [ %v217789, %block19718 ], [ %v18649, %block15616 ], [ %v18649, %edge2150.thread ], [ %v18649, %edge2167 ], [ %v18649, %block23952 ], [ %v217789, %edge2952 ], [ %v18649, %edge3826 ], [ %v18649, %edge2150 ], [ %v18649, %edge3980.sink.split ]
  %v36653 = phi i64 [ %v217790, %block19718 ], [ %v18656, %block15616 ], [ %v18656, %edge2150.thread ], [ %v18656, %edge2167 ], [ %v18656, %block23952 ], [ %v217790, %edge2952 ], [ %v18656, %edge3826 ], [ %v18656, %edge2150 ], [ %v18656, %edge3980.sink.split ]
  %v41034 = phi i64 [ 0, %block19718 ], [ 0, %block15616 ], [ 0, %edge2150.thread ], [ 0, %edge2167 ], [ %v217796, %block23952 ], [ 0, %edge2952 ], [ %v217796, %edge3826 ], [ 0, %edge2150 ], [ 0, %edge3980.sink.split ]
  %t2880 = icmp ugt i64 %v36653, %v217308
  br i1 %t2880, label %edge3986, label %edge3984

edge3984:                                         ; preds = %block535, %edge3980, %edge3987
  %v41034659 = phi i64 [ %v41034, %edge3987 ], [ %v41034, %edge3980 ], [ 0, %block535 ]
  %v36653658 = phi i64 [ %v36653, %edge3987 ], [ %v36653, %edge3980 ], [ %v217308, %block535 ]
  %v36650657 = phi i64 [ %v14613, %edge3987 ], [ %v14613, %edge3980 ], [ %v217311, %block535 ]
  %v36647656 = phi i64 [ %v14586, %edge3987 ], [ %v14586, %edge3980 ], [ %v217305, %block535 ]
  %v36640654 = phi i64 [ %v36640, %edge3987 ], [ %v36640, %edge3980 ], [ %v10788, %block535 ]
  %v36637653 = phi i64 [ %v36637, %edge3987 ], [ %v36637, %edge3980 ], [ %v10796, %block535 ]
  %v36635652 = phi i64 [ %v36635, %edge3987 ], [ %v36635, %edge3980 ], [ %v10823, %block535 ]
  %v36628651 = phi i64 [ %v36628, %edge3987 ], [ %v36628, %edge3980 ], [ %v217304, %block535 ]
  %v36625650 = phi i64 [ %v36625, %edge3987 ], [ %v36625, %edge3980 ], [ %v217307, %block535 ]
  %v36623649 = phi i64 [ %v36623, %edge3987 ], [ %v36623, %edge3980 ], [ %v217306, %block535 ]
  %v36894 = phi i64 [ %v36869, %edge3987 ], [ 0, %edge3980 ], [ 0, %block535 ]
  %v36898 = and i64 %v36894, 65535
  %t2891.not = icmp ult i64 %v217293, %v217312
  %or.cond303 = select i1 %t1457, i1 true, i1 %t2891.not
  br i1 %or.cond303, label %edge4214, label %edge4012

edge3986:                                         ; preds = %edge3980
  %v36864 = add i64 %v36653, -1
  %t2883 = icmp ult i64 %v36864, %v3718
  br i1 %t2883, label %edge3987, label %common.ret

edge3987:                                         ; preds = %edge3986
  %v36867 = shl nuw nsw i64 %v36864, 2
  %v36868 = add i64 %v36867, %v36635
  %t2885 = inttoptr i64 %v36868 to ptr
  %t2886 = load i32, ptr %t2885, align 1
  %v36869 = zext i32 %t2886 to i64
  br label %edge3984

edge4012:                                         ; preds = %edge3984
  %t2894.not = icmp ult i64 %v36898, %v217318
  br i1 %t2894.not, label %edge4128, label %edge4056

edge4056:                                         ; preds = %edge4012
  %t2897 = icmp samesign ugt i64 %v36898, 8
  %v38498 = select i1 %t2897, i64 36, i64 32
  %v38689 = add i64 %v38498, %v10829
  %t2907 = inttoptr i64 %v38689 to ptr
  %t2908 = load i32, ptr %t2907, align 1
  %16 = add i32 %t2908, 1
  %t2919.not = icmp eq i64 %v10792, 0
  br i1 %t2919.not, label %block25006, label %edge4057

edge4057:                                         ; preds = %edge4056
  %t2920 = inttoptr i64 %v10792 to ptr
  %v38813 = load i64, ptr %t2920, align 1
  %t2921 = icmp eq i64 %v38813, 1
  br i1 %t2921, label %block25004, label %block25006

edge4062:                                         ; preds = %block25006
  %v38823 = load i64, ptr %ss1, align 8
  %v38824 = load i64, ptr %t606, align 8
  %v38825 = load i64, ptr %t607, align 8
  %.pre20729 = add i64 %v38823, %v38498
  %.pre20730 = inttoptr i64 %.pre20729 to ptr
  br label %block25004

edge4065:                                         ; preds = %block25004
  %t2945 = inttoptr i64 %v38812 to ptr
  %v38830 = load i64, ptr %t2945, align 1
  %v38831 = add i64 %v38830, 1
  store i64 %v38831, ptr %t2945, align 1
  br label %block25010

edge4070:                                         ; preds = %block25010
  %v38836 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10792)
  %t2949.not = icmp eq i64 %v38836, 0
  br i1 %t2949.not, label %edge4074, label %block25001

edge4074:                                         ; preds = %edge4070, %block25010
  %v38841 = add i64 %v217317, 1
  %v38846 = add i64 %v36898, %v217293
  %t2952 = icmp samesign ult i64 %v36898, 259
  br i1 %t2952, label %edge4101, label %block25159

edge4101:                                         ; preds = %edge4074
  %v39034 = shl nuw nsw i64 %v36898, 2
  %v39035 = add i64 %v39034, %v10827
  %t2955 = inttoptr i64 %v39035 to ptr
  %t2956 = load i32, ptr %t2955, align 1
  %17 = add i32 %t2956, 1
  %t2964.not = icmp eq i64 %v10790, 0
  br i1 %t2964.not, label %block25164, label %edge4102

edge4102:                                         ; preds = %edge4101
  %t2965 = inttoptr i64 %v10790 to ptr
  %v39103 = load i64, ptr %t2965, align 1
  %t2966 = icmp eq i64 %v39103, 1
  br i1 %t2966, label %block25162, label %block25164

edge4107:                                         ; preds = %block25164
  %v39113 = load i64, ptr %ss1, align 8
  %v39114 = load i64, ptr %t606, align 8
  %v39115 = load i64, ptr %t607, align 8
  %.pre20731 = add i64 %v39113, %v39034
  %.pre20732 = inttoptr i64 %.pre20731 to ptr
  br label %block25162

edge4110:                                         ; preds = %block25162
  %t2990 = inttoptr i64 %v39102 to ptr
  %v39120 = load i64, ptr %t2990, align 1
  %v39121 = add i64 %v39120, 1
  store i64 %v39121, ptr %t2990, align 1
  br label %block25168

edge4113:                                         ; preds = %block25168
  %t2994 = inttoptr i64 %v10790 to ptr
  %v39124 = load i64, ptr %t2994, align 1
  %v39125 = add i64 %v39124, -1
  store i64 %v39125, ptr %t2994, align 1
  %t2996.not = icmp eq i64 %v39125, 0
  br i1 %t2996.not, label %edge4116, label %edge4214

edge4116:                                         ; preds = %edge4113
  %v39126 = call i64 @mech_compact_free(i64 %v0, i64 %v10790)
  %t2997.not = icmp eq i64 %v39126, 0
  br i1 %t2997.not, label %edge4214, label %block25159

edge4128:                                         ; preds = %edge4012
  %t3000 = icmp ult i64 %v217293, %v1395
  br i1 %t3000, label %edge4134, label %edge4132

edge4132:                                         ; preds = %edge4128, %block25325
  %v39500 = phi i64 [ %v39385, %block25325 ], [ 0, %edge4128 ]
  %v39549 = lshr i64 %v39500, 5
  %v39630 = and i64 %v39549, 6
  %v39635 = and i64 %v39500, 1
  %v39640 = or disjoint i64 %v39630, %v39635
  %v39828 = shl nuw nsw i64 %v39640, 2
  %v39829 = add i64 %v39828, %v10829
  %t3015 = inttoptr i64 %v39829 to ptr
  %t3016 = load i32, ptr %t3015, align 1
  %v39853 = add i32 %t3016, 1
  %t3023.not = icmp eq i64 %v10792, 0
  br i1 %t3023.not, label %block25482, label %edge4170

edge4134:                                         ; preds = %edge4128
  br i1 %v1061851.not1217, label %edge4139, label %block25325

edge4139:                                         ; preds = %edge4134
  br i1 %t975, label %trap3007, label %ok3007

edge4170:                                         ; preds = %edge4132
  %t3024 = inttoptr i64 %v10792 to ptr
  %v39897 = load i64, ptr %t3024, align 1
  %t3025 = icmp eq i64 %v39897, 1
  br i1 %t3025, label %block25481, label %block25482

edge4175:                                         ; preds = %block25482
  %v39907 = load i64, ptr %ss1, align 8
  %v39908 = load i64, ptr %t606, align 8
  %v39909 = load i64, ptr %t607, align 8
  %.pre20727 = add i64 %v39907, %v39828
  %.pre20728 = inttoptr i64 %.pre20727 to ptr
  br label %block25481

edge4178:                                         ; preds = %block25481
  %t3049 = inttoptr i64 %v39896 to ptr
  %v39914 = load i64, ptr %t3049, align 1
  %v39915 = add i64 %v39914, 1
  store i64 %v39915, ptr %t3049, align 1
  br label %block25486

edge4183:                                         ; preds = %block25486
  %v39920 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v10792)
  %t3053.not = icmp eq i64 %v39920, 0
  br i1 %t3053.not, label %edge4187, label %block25477

edge4187:                                         ; preds = %edge4183, %block25486
  %v39925 = add i64 %v217317, 1
  %v39930 = add i64 %v217293, 1
  br label %edge4214

edge4214:                                         ; preds = %edge4187, %edge4116, %edge4113, %block25168, %edge3984
  %v40030 = phi i64 [ %v39097, %block25168 ], [ %v10827, %edge3984 ], [ %v10827, %edge4187 ], [ %v39097, %edge4116 ], [ %v39097, %edge4113 ]
  %v40032 = phi i64 [ %v39099, %block25168 ], [ %v10800, %edge3984 ], [ %v10800, %edge4187 ], [ %v39099, %edge4116 ], [ %v39099, %edge4113 ]
  %v40035 = phi i64 [ %v39102, %block25168 ], [ %v10790, %edge3984 ], [ %v10790, %edge4187 ], [ %v39102, %edge4116 ], [ %v39102, %edge4113 ]
  %v40042 = phi i64 [ %v38807, %block25168 ], [ %v10829, %edge3984 ], [ %v39891, %edge4187 ], [ %v38807, %edge4116 ], [ %v38807, %edge4113 ]
  %v40044 = phi i64 [ %v38809, %block25168 ], [ %v10804, %edge3984 ], [ %v39893, %edge4187 ], [ %v38809, %edge4116 ], [ %v38809, %edge4113 ]
  %v40047 = phi i64 [ %v38812, %block25168 ], [ %v10792, %edge3984 ], [ %v39896, %edge4187 ], [ %v38812, %edge4116 ], [ %v38812, %edge4113 ]
  %v40054 = phi i64 [ %v38841, %block25168 ], [ %v217317, %edge3984 ], [ %v39925, %edge4187 ], [ %v38841, %edge4116 ], [ %v38841, %edge4113 ]
  %v40057 = phi i64 [ %v38846, %block25168 ], [ %v217312, %edge3984 ], [ %v39930, %edge4187 ], [ %v38846, %edge4116 ], [ %v38846, %edge4113 ]
  %v40062 = sub i64 %v36653658, %v217308
  %t3056 = icmp ult i64 %v217293, %v1395
  br i1 %t3056, label %edge4222, label %edge4220

edge4220:                                         ; preds = %edge4214, %block25694
  %v40288 = phi i64 [ %1, %block25694 ], [ 0, %edge4214 ]
  %v40283 = and i64 %v40062, 65535
  %v40298 = or disjoint i64 %v40288, %v40283
  %v40314 = add i64 %v36653658, 1
  %t3067 = icmp ult i64 %v36653658, %v3718
  br i1 %t3067, label %edge4241, label %block25707

edge4222:                                         ; preds = %edge4214
  br i1 %v1061851.not1217, label %edge4227, label %block25694

edge4227:                                         ; preds = %edge4222
  br i1 %t975, label %trap3063, label %ok3063

edge4241:                                         ; preds = %edge4220
  %v40322 = shl nuw nsw i64 %v36653658, 2
  %v40323 = add i64 %v36635652, %v40322
  %t3069 = inttoptr i64 %v40323 to ptr
  %t3070 = load i32, ptr %t3069, align 1
  %v40324 = zext i32 %t3070 to i64
  %t3071 = icmp eq i64 %v40298, %v40324
  %t3098.not1001116 = icmp eq i64 %v36640654, 0
  br i1 %t3071, label %edge4262, label %edge4244

edge4244:                                         ; preds = %edge4241
  br i1 %t3098.not1001116, label %block25712, label %edge4245

edge4245:                                         ; preds = %edge4244
  %t3074 = inttoptr i64 %v36640654 to ptr
  %v40340 = load i64, ptr %t3074, align 1
  %t3075 = icmp eq i64 %v40340, 1
  br i1 %t3075, label %block25710, label %block25712

edge4250:                                         ; preds = %block25712
  %v40350 = load i64, ptr %ss1, align 8
  %v40351 = load i64, ptr %t606, align 8
  %v40352 = load i64, ptr %t607, align 8
  %.pre20725 = add i64 %v40350, %v40322
  %.pre20726 = inttoptr i64 %.pre20725 to ptr
  br label %block25710

edge4253:                                         ; preds = %block25710
  %t3099 = inttoptr i64 %v40339 to ptr
  %v40357 = load i64, ptr %t3099, align 1
  %v40358 = add i64 %v40357, 1
  store i64 %v40358, ptr %t3099, align 1
  br label %block25716

edge4258:                                         ; preds = %block25716
  %v40363 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v36640654)
  %t3103.not = icmp eq i64 %v40363, 0
  br i1 %t3103.not, label %edge4262, label %block25707

edge4262:                                         ; preds = %edge4241, %edge4258, %block25716
  %v4032811191139 = phi i64 [ %v40334, %block25716 ], [ %v40334, %edge4258 ], [ %v36635652, %edge4241 ]
  %v4033011221138 = phi i64 [ %v40336, %block25716 ], [ %v40336, %edge4258 ], [ %v36637653, %edge4241 ]
  %v4033311241137 = phi i64 [ %v40339, %block25716 ], [ %v40339, %edge4258 ], [ %v36640654, %edge4241 ]
  %t3098.not10011271136 = phi i1 [ %t3098.not100, %block25716 ], [ %t3098.not100, %edge4258 ], [ %t3098.not1001116, %edge4241 ]
  br i1 %t1457, label %edge4281, label %edge4267

edge4265:                                         ; preds = %edge4267, %edge4281
  %v41041 = phi i64 [ %v40769, %edge4267 ], [ %v40834, %edge4281 ]
  %v40842 = shl nuw nsw i64 %v36898, 16
  %v40848 = lshr i64 %v36894, 16
  %v40853 = or disjoint i64 %v40842, %v40848
  %spec.select23 = select i1 %t1457, i64 %v41034659, i64 %v40853
  %t3116.not = icmp eq i64 %v41041, 0
  %v41259 = shl i64 %v41041, 32
  %v41265 = or i64 %v41259, %spec.select23
  %v41283 = add i64 %v217293, 1
  %t312031556 = icmp eq i64 %v41265, 0
  %t3120 = select i1 %t3116.not, i1 true, i1 %t312031556
  br i1 %t3120, label %edge4337, label %block475.outer

edge4267:                                         ; preds = %edge4262
  %t3108 = icmp samesign ugt i64 %v36898, 2
  %t3109 = icmp samesign uge i64 %v36898, %spec.select17
  %v40581102 = and i1 %t3108, %t3109
  %v40753 = add nsw i64 %v36898, -1
  %v40769 = select i1 %v40581102, i64 %v40753, i64 0
  br label %edge4265

edge4281:                                         ; preds = %edge4262
  %v40834 = add nsw i64 %v12688, -1
  br label %edge4265

edge4337:                                         ; preds = %edge4265
  %t3123 = icmp uge i64 %v41283, %v217319
  %t3124 = icmp ugt i64 %v40314, 1499999
  %v41761103.not.not31557 = or i1 %t3123, %t3124
  %t3125 = icmp ult i64 %v40054, 512
  %v41896 = sub i64 %v41283, %v217320
  %t3126 = icmp ult i64 %v41896, 5000
  %.not = or i1 %t3126, %t3125
  %v41992 = sub i64 %v1395, %v41283
  %t3127 = icmp ult i64 %v41992, 5000
  %.not31559 = or i1 %t3127, %.not
  %v42003106.not = or i1 %v41761103.not.not31557, %.not31559
  br i1 %v42003106.not, label %edge4337.block2462_crit_edge, label %edge4353

edge4337.block2462_crit_edge:                     ; preds = %edge4337
  br i1 %v41761103.not.not31557, label %edge4852, label %block475.backedge

edge4353:                                         ; preds = %edge4337
  %t3130.not = icmp eq i64 %v217321, 0
  br i1 %t3130.not, label %block2305.preheader, label %edge4360

edge4360:                                         ; preds = %edge4353
  %t3133.not = icmp eq i64 %v40047, 0
  br i1 %t3133.not, label %block26690, label %edge4361

edge4361:                                         ; preds = %edge4360
  %t3134 = inttoptr i64 %v40047 to ptr
  %v42672 = load i64, ptr %t3134, align 1
  %v42673 = add i64 %v42672, 1
  store i64 %v42673, ptr %t3134, align 1
  br label %block26690

edge4364:                                         ; preds = %block26690
  %v42674 = load i64, ptr %t1092, align 1
  %v42675 = add i64 %v42674, 1
  store i64 %v42675, ptr %t1092, align 1
  br label %block2176.preheader

edge4387:                                         ; preds = %block2176
  %v43197 = shl nuw nsw i64 %v42744, 2
  %v43198 = add i64 %v43197, %v10828.ph.ph
  %t3142 = inttoptr i64 %v43198 to ptr
  %t3143 = load i32, ptr %t3142, align 1
  %18 = zext i32 %t3143 to i64
  %umul = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %18, i64 %v40054)
  %19 = extractvalue { i64, i1 } %umul, 1
  br i1 %19, label %common.ret, label %edge4397

edge4397:                                         ; preds = %edge4387
  %v43622 = add i64 %v43197, %v40042
  %t3152 = inttoptr i64 %v43622 to ptr
  %t3153 = load i32, ptr %t3152, align 1
  %20 = zext i32 %t3153 to i64
  %umul107 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %20, i64 %v217321)
  %21 = extractvalue { i64, i1 } %umul107, 1
  br i1 %21, label %common.ret, label %edge4449

edge4432:                                         ; preds = %block2176
  br i1 %t3133.not, label %block27444, label %edge4435

edge4435:                                         ; preds = %edge4432
  %v43889 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v40047)
  %t3177.not = icmp eq i64 %v43889, 0
  br i1 %t3177.not, label %block27444, label %block27440

edge4439:                                         ; preds = %block27444
  %v43891 = load i64, ptr %t1092, align 1
  %v43892 = add i64 %v43891, -1
  store i64 %v43892, ptr %t1092, align 1
  %t3181.not = icmp eq i64 %v43892, 0
  br i1 %t3181.not, label %edge4442, label %edge4464

edge4442:                                         ; preds = %edge4439
  %v43893 = call i64 @mech_compact_free(i64 %v0, i64 %v10791.ph.ph)
  %t3182.not = icmp eq i64 %v43893, 0
  br i1 %t3182.not, label %edge4464, label %block27440

edge4449:                                         ; preds = %edge4397
  %v43226 = mul i64 %v40054, %18
  %v43649 = mul i64 %v217321, %20
  %t3162 = icmp ugt i64 %v43649, %v43226
  %v43854 = sub i64 %v43649, %v43226
  %22 = sub nuw i64 %v43226, %v43649
  %v44067 = select i1 %t3162, i64 %v43854, i64 %22
  %v44060 = add nuw nsw i64 %v42744, 1
  %v44068 = add i64 %v44067, %v42747
  %t3186.not = icmp ult i64 %v44068, %v42747
  br i1 %t3186.not, label %common.ret, label %block2176

edge4464:                                         ; preds = %block27444, %edge4439, %edge4442
  %v44112 = add i64 %v40054, %v217321
  %t3190.not = icmp ult i64 %v44112, %v217321
  br i1 %t3190.not, label %common.ret, label %edge4467

edge4467:                                         ; preds = %edge4464
  %t3198 = icmp ult i64 %v40054, 92233720368547759
  br i1 %t3198, label %edge4468, label %common.ret

edge4468:                                         ; preds = %edge4467
  %v44203 = mul nuw i64 %v40054, 200
  %v44211108 = lshr i64 %v44203, 9
  %umul109 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v44211108, i64 %v217321)
  %23 = extractvalue { i64, i1 } %umul109, 1
  br i1 %23, label %common.ret, label %edge4473

edge4473:                                         ; preds = %edge4468
  %v44219 = mul i64 %v44211108, %v217321
  %t3212 = icmp ult i64 %v41896, 10000
  %t3213 = icmp ult i64 %v44112, 8192
  %v44399110 = and i1 %t3212, %t3213
  br i1 %v44399110, label %edge4479, label %edge4495

edge4479:                                         ; preds = %edge4473
  %v44573 = sub nuw nsw i64 8192, %v44112
  %umul111 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v44219, i64 %v44573)
  %24 = extractvalue { i64, i1 } %umul111, 1
  br i1 %24, label %common.ret, label %edge4484

edge4484:                                         ; preds = %edge4479
  %v44581 = mul i64 %v44219, %v44573
  %v44588112 = lshr i64 %v44581, 13
  %v44593 = add i64 %v44588112, %v44219
  %t3226.not = icmp ult i64 %v44593, %v44219
  br i1 %t3226.not, label %common.ret, label %edge4495

edge4495:                                         ; preds = %edge4473, %edge4484
  %v44695 = phi i64 [ %v44593, %edge4484 ], [ %v44219, %edge4473 ]
  %v44675113 = lshr i64 %v41896, 12
  %umul114 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v44675113, i64 %v217321)
  %25 = extractvalue { i64, i1 } %umul114, 1
  br i1 %25, label %common.ret, label %edge4497

edge4497:                                         ; preds = %edge4495
  %v44683 = mul i64 %v44675113, %v217321
  %v44688 = add i64 %v42747, %v44683
  %t3240.not = icmp ult i64 %v44688, %v42747
  br i1 %t3240.not, label %common.ret, label %edge4502

edge4502:                                         ; preds = %edge4497
  %t3244.not = icmp ult i64 %v44688, %v44695
  br i1 %t3244.not, label %block2305.preheader, label %block2462

block2305.preheader:                              ; preds = %edge4353, %edge4502
  br label %block2305

edge4538:                                         ; preds = %block2305
  %v45567 = add nuw nsw i64 %v45203, 1
  %v45575 = shl nuw nsw i64 %v45203, 2
  %v45576 = add i64 %v45575, %v45191
  %t3250 = inttoptr i64 %v45576 to ptr
  %t3251 = load i32, ptr %t3250, align 1
  %v45577 = zext i32 %t3251 to i64
  %v45789 = add i64 %v45575, %v45176
  %t3252 = inttoptr i64 %v45789 to ptr
  %t3253 = load i32, ptr %t3252, align 1
  %v45790 = zext i32 %t3253 to i64
  %v45810 = add nuw nsw i64 %v45790, %v45577
  %t3254 = icmp samesign ult i64 %v45810, 4294967296
  br i1 %t3254, label %edge4561, label %common.ret

edge4539:                                         ; preds = %block2305
  %v46280 = add i64 %v40054, %v217321
  %t3335.not = icmp ult i64 %v46280, %v217321
  br i1 %t3335.not, label %common.ret, label %block2378

edge4561:                                         ; preds = %edge4538
  %t3260 = icmp eq i32 %t3253, 0
  br i1 %t3260, label %block28468, label %edge4566

edge4566:                                         ; preds = %edge4561
  %t3262.not = icmp eq i64 %v45196, 0
  br i1 %t3262.not, label %block28470, label %edge4567

edge4567:                                         ; preds = %edge4566
  %t3263 = inttoptr i64 %v45196 to ptr
  %v45845 = load i64, ptr %t3263, align 1
  %t3264 = icmp eq i64 %v45845, 1
  br i1 %t3264, label %block28469, label %block28470

edge4572:                                         ; preds = %block28470
  %v45855 = load i64, ptr %ss1, align 8
  %v45856 = load i64, ptr %t606, align 8
  %v45857 = load i64, ptr %t607, align 8
  %.pre20661 = add i64 %v45855, %v45575
  %.pre20662 = inttoptr i64 %.pre20661 to ptr
  br label %block28469

edge4575:                                         ; preds = %block28468
  %t3288 = inttoptr i64 %v45838 to ptr
  %v45862 = load i64, ptr %t3288, align 1
  %v45863 = add i64 %v45862, 1
  store i64 %v45863, ptr %t3288, align 1
  br label %block28474

edge4580:                                         ; preds = %block28474
  %v45868 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v45196)
  %t3292.not = icmp eq i64 %v45868, 0
  br i1 %t3292.not, label %edge4586, label %block28465

edge4586:                                         ; preds = %edge4580, %block28474
  %t3296 = load i32, ptr %t3252, align 1
  %t3297 = icmp eq i32 %t3296, 0
  br i1 %t3297, label %block28486, label %edge4591

edge4591:                                         ; preds = %edge4586
  %t3299.not = icmp eq i64 %v45181, 0
  br i1 %t3299.not, label %block28488, label %edge4592

edge4592:                                         ; preds = %edge4591
  %t3300 = inttoptr i64 %v45181 to ptr
  %v45908 = load i64, ptr %t3300, align 1
  %t3301 = icmp eq i64 %v45908, 1
  br i1 %t3301, label %block28487, label %block28488

edge4597:                                         ; preds = %block28488
  %v45918 = load i64, ptr %ss1, align 8
  %v45919 = load i64, ptr %t606, align 8
  %v45920 = load i64, ptr %t607, align 8
  %.pre = add i64 %v45918, %v45575
  %.pre20660 = inttoptr i64 %.pre to ptr
  br label %block28487

edge4600:                                         ; preds = %block28486
  %t3325 = inttoptr i64 %v45901 to ptr
  %v45925 = load i64, ptr %t3325, align 1
  %v45926 = add i64 %v45925, 1
  store i64 %v45926, ptr %t3325, align 1
  br label %block28492

edge4603:                                         ; preds = %block28492
  %t3329 = inttoptr i64 %v45181 to ptr
  %v45929 = load i64, ptr %t3329, align 1
  %v45930 = add i64 %v45929, -1
  store i64 %v45930, ptr %t3329, align 1
  %t3331.not = icmp eq i64 %v45930, 0
  br i1 %t3331.not, label %edge4606, label %block2305.backedge

edge4606:                                         ; preds = %edge4603
  %v45931 = call i64 @mech_compact_free(i64 %v0, i64 %v45181)
  %t3332.not = icmp eq i64 %v45931, 0
  br i1 %t3332.not, label %block2305.backedge, label %block28483

block2305.backedge:                               ; preds = %edge4606, %edge4603, %block28492
  br label %block2305

edge4658:                                         ; preds = %block2378
  %v47105 = shl nuw nsw i64 %v46682, 2
  %v47106 = add i64 %v47105, %v46670
  %t3344 = inttoptr i64 %v47106 to ptr
  %t3345 = load i32, ptr %t3344, align 1
  %t3346.not = icmp eq i32 %t3345, 0
  br i1 %t3346.not, label %edge4773, label %edge4678

edge4678:                                         ; preds = %edge4658
  %v47107 = zext i32 %t3345 to i64
  %v47484 = add i64 %v47105, %v46658
  %t3351 = inttoptr i64 %v47484 to ptr
  %t3352 = load i32, ptr %t3351, align 1
  %v47485 = zext i32 %t3352 to i64
  %v47504 = add nuw nsw i64 %v47485, %v47107
  %t3353 = icmp samesign ult i64 %v47504, 4294967296
  br i1 %t3353, label %edge4691, label %common.ret

edge4691:                                         ; preds = %edge4678
  %t3363.not = icmp eq i64 %v46663, 0
  br i1 %t3363.not, label %block29323, label %edge4692

edge4692:                                         ; preds = %edge4691
  %t3364 = inttoptr i64 %v46663 to ptr
  %v47539 = load i64, ptr %t3364, align 1
  %t3365 = icmp eq i64 %v47539, 1
  br i1 %t3365, label %block29322, label %block29323

edge4697:                                         ; preds = %block29323
  %v47549 = load i64, ptr %ss1, align 8
  %v47550 = load i64, ptr %t606, align 8
  %v47551 = load i64, ptr %t607, align 8
  %.pre20663 = add i64 %v47549, %v47105
  %.pre20664 = inttoptr i64 %.pre20663 to ptr
  br label %block29322

edge4700:                                         ; preds = %block29322
  %t3389 = inttoptr i64 %v47538 to ptr
  %v47556 = load i64, ptr %t3389, align 1
  %v47557 = add i64 %v47556, 1
  store i64 %v47557, ptr %t3389, align 1
  br label %block29327

edge4705:                                         ; preds = %block29327
  %v47562 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v46663)
  %t3393.not = icmp eq i64 %v47562, 0
  br i1 %t3393.not, label %edge4709, label %block29318

edge4709:                                         ; preds = %edge4705, %block29327
  %t3399 = load i32, ptr %t3344, align 1
  %t3400 = icmp eq i32 %t3399, 0
  br i1 %t3400, label %block29337, label %edge4714

edge4714:                                         ; preds = %edge4709
  %t3402.not = icmp eq i64 %v46675, 0
  br i1 %t3402.not, label %block29339, label %edge4715

edge4715:                                         ; preds = %edge4714
  %t3403 = inttoptr i64 %v46675 to ptr
  %v47595 = load i64, ptr %t3403, align 1
  %t3404 = icmp eq i64 %v47595, 1
  br i1 %t3404, label %block29338, label %block29339

edge4720:                                         ; preds = %block29339
  %v47605 = load i64, ptr %ss1, align 8
  %v47606 = load i64, ptr %t606, align 8
  %v47607 = load i64, ptr %t607, align 8
  %.pre20665 = add i64 %v47605, %v47105
  %.pre20666 = inttoptr i64 %.pre20665 to ptr
  br label %block29338

edge4723:                                         ; preds = %block29337
  %t3428 = inttoptr i64 %v47588 to ptr
  %v47612 = load i64, ptr %t3428, align 1
  %v47613 = add i64 %v47612, 1
  store i64 %v47613, ptr %t3428, align 1
  br label %block29343

edge4726:                                         ; preds = %block29343
  %t3432 = inttoptr i64 %v46675 to ptr
  %v47616 = load i64, ptr %t3432, align 1
  %v47617 = add i64 %v47616, -1
  store i64 %v47617, ptr %t3432, align 1
  %t3434.not = icmp eq i64 %v47617, 0
  br i1 %t3434.not, label %edge4729, label %edge4773

edge4729:                                         ; preds = %edge4726
  %v47618 = call i64 @mech_compact_free(i64 %v0, i64 %v46675)
  %t3435.not = icmp eq i64 %v47618, 0
  br i1 %t3435.not, label %edge4773, label %block29334

edge4773:                                         ; preds = %edge4658, %block29343, %edge4726, %edge4729
  %v47721.ph = phi i64 [ %v47533, %edge4726 ], [ %v47533, %edge4729 ], [ %v47533, %block29343 ], [ %v46658, %edge4658 ]
  %v47723.ph = phi i64 [ %v47535, %edge4726 ], [ %v47535, %edge4729 ], [ %v47535, %block29343 ], [ %v46660, %edge4658 ]
  %v47726.ph = phi i64 [ %v47538, %edge4726 ], [ %v47538, %edge4729 ], [ %v47538, %block29343 ], [ %v46663, %edge4658 ]
  %v47733.ph = phi i64 [ %v47583, %edge4726 ], [ %v47583, %edge4729 ], [ %v47583, %block29343 ], [ %v46670, %edge4658 ]
  %v47735.ph = phi i64 [ %v47585, %edge4726 ], [ %v47585, %edge4729 ], [ %v47585, %block29343 ], [ %v46672, %edge4658 ]
  %v47738.ph = phi i64 [ %v47588, %edge4726 ], [ %v47588, %edge4729 ], [ %v47588, %block29343 ], [ %v46675, %edge4658 ]
  %v47950 = add nuw nsw i64 %v46682, 1
  br label %block2378

edge4852:                                         ; preds = %edge4337.block2462_crit_edge, %block2462
  %v21735020992 = phi i64 [ 0, %edge4337.block2462_crit_edge ], [ 1, %block2462 ]
  br i1 %t23.not, label %edge4881, label %edge4853

edge4853:                                         ; preds = %edge4852
  %v48420 = load i64, ptr %t910, align 1
  %v48421 = add i64 %v48420, -1
  store i64 %v48421, ptr %t910, align 1
  %t3448.not = icmp eq i64 %v48421, 0
  br i1 %t3448.not, label %edge4856, label %edge4881

edge4856:                                         ; preds = %edge4853
  %v48422 = call i64 @mech_compact_free(i64 %v0, i64 %v1399)
  %t3449.not = icmp eq i64 %v48422, 0
  br i1 %t3449.not, label %edge4881, label %edge4857

edge4857:                                         ; preds = %edge4856
  %t3450 = insertvalue { i64, i64 } poison, i64 %v48422, 0
  %t3451 = insertvalue { i64, i64 } %t3450, i64 2494, 1
  br label %common.ret

edge4881:                                         ; preds = %edge4852, %edge4853, %edge4856
  %t3453 = icmp ne i64 %v217327, 1
  %v50057124.not = or i1 %t3453, %v42003106.not
  br i1 %v50057124.not, label %block2587.outer, label %edge4885

edge4885:                                         ; preds = %edge4881
  %t3456.not = icmp ult i64 %v41283, %v217326
  br i1 %t3456.not, label %common.ret, label %edge4894

edge4894:                                         ; preds = %edge4885
  %v50401 = sub nuw i64 %v41283, %v217326
  br i1 %t3098.not10011271136, label %edge4902, label %edge4897

edge4897:                                         ; preds = %edge4894
  %t3461 = inttoptr i64 %v4033311241137 to ptr
  %v50404 = load i64, ptr %t3461, align 1
  %v50405 = add i64 %v50404, 1
  store i64 %v50405, ptr %t3461, align 1
  br label %edge4902

edge4902:                                         ; preds = %edge4894, %edge4897
  %v51013 = add i64 %v4032811191139, -4
  br label %block2509

edge4915:                                         ; preds = %block2509
  br i1 %t3098.not10011271136, label %edge4971, label %edge4945

edge4917:                                         ; preds = %block2509
  %t3466.not = icmp eq i64 %v50448, 0
  br i1 %t3466.not, label %common.ret, label %edge4918

edge4918:                                         ; preds = %edge4917
  %v50638 = add i64 %v50448, -1
  %t3470 = icmp ult i64 %v50638, %v3718
  br i1 %t3470, label %edge4931, label %block30641

edge4931:                                         ; preds = %edge4918
  %v51010 = shl i64 %v50448, 2
  %v51014 = add i64 %v51013, %v51010
  %t3473 = inttoptr i64 %v51014 to ptr
  %t3474 = load i32, ptr %t3473, align 1
  %26 = and i32 %t3474, 65535
  %v51040 = zext nneg i32 %26 to i64
  %t3475.not = icmp ult i64 %v50638, %v51040
  br i1 %t3475.not, label %common.ret, label %block30641

edge4945:                                         ; preds = %edge4915
  %t3480 = inttoptr i64 %v4033311241137 to ptr
  %v51073 = load i64, ptr %t3480, align 1
  %v51074 = add i64 %v51073, -1
  store i64 %v51074, ptr %t3480, align 1
  %t3482.not = icmp eq i64 %v51074, 0
  br i1 %t3482.not, label %edge4948, label %edge4971

edge4948:                                         ; preds = %edge4945
  %v51075 = call i64 @mech_compact_free(i64 %v0, i64 %v4033311241137)
  %t3483.not = icmp eq i64 %v51075, 0
  br i1 %t3483.not, label %edge4971, label %edge4949

edge4949:                                         ; preds = %edge4948
  %t3484 = insertvalue { i64, i64 } poison, i64 %v51075, 0
  %t3485 = insertvalue { i64, i64 } %t3484, i64 2565, 1
  br label %common.ret

block2662.thread:                                 ; preds = %edge4971
  %v51455 = sub nuw i64 %v40314, %v50448
  br label %edge5117

edge4971:                                         ; preds = %edge4948, %edge4945, %edge4915
  %t3488.not = icmp ult i64 %v40314, %v50448
  br i1 %t3488.not, label %common.ret, label %block2662.thread

edge5008:                                         ; preds = %block2587
  %v52110 = add nuw nsw i64 %v51757, 1
  %v52118 = shl nuw nsw i64 %v51757, 2
  %v52119 = add i64 %v52118, %v51745.ph
  %t3499 = inttoptr i64 %v52119 to ptr
  %t3500 = load i32, ptr %t3499, align 1
  %t3501.not = icmp eq i32 %t3500, 0
  br i1 %t3501.not, label %block2587, label %edge5027

edge5027:                                         ; preds = %edge5008
  %v52120 = zext i32 %t3500 to i64
  %v52488 = add i64 %v52118, %v51733.ph
  %t3506 = inttoptr i64 %v52488 to ptr
  %t3507 = load i32, ptr %t3506, align 1
  %v52489 = zext i32 %t3507 to i64
  %v52508 = add nuw nsw i64 %v52489, %v52120
  %t3508 = icmp samesign ult i64 %v52508, 4294967296
  br i1 %t3508, label %edge5040, label %common.ret

edge5040:                                         ; preds = %edge5027
  %t3518.not = icmp eq i64 %v51738.ph, 0
  br i1 %t3518.not, label %block31619, label %edge5041

edge5041:                                         ; preds = %edge5040
  %t3519 = inttoptr i64 %v51738.ph to ptr
  %v52543 = load i64, ptr %t3519, align 1
  %t3520 = icmp eq i64 %v52543, 1
  br i1 %t3520, label %block31618, label %block31619

edge5046:                                         ; preds = %block31619
  %v52553 = load i64, ptr %ss1, align 8
  %v52554 = load i64, ptr %t606, align 8
  %v52555 = load i64, ptr %t607, align 8
  %.pre20667 = add i64 %v52553, %v52118
  %.pre20668 = inttoptr i64 %.pre20667 to ptr
  br label %block31618

edge5049:                                         ; preds = %block31618
  %t3544 = inttoptr i64 %v52542 to ptr
  %v52560 = load i64, ptr %t3544, align 1
  %v52561 = add i64 %v52560, 1
  store i64 %v52561, ptr %t3544, align 1
  br label %block31623

edge5054:                                         ; preds = %block31623
  %v52566 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v51738.ph)
  %t3548.not = icmp eq i64 %v52566, 0
  br i1 %t3548.not, label %edge5058, label %block31614

edge5058:                                         ; preds = %edge5054, %block31623
  %t3554 = load i32, ptr %t3499, align 1
  %t3555 = icmp eq i32 %t3554, 0
  br i1 %t3555, label %block31633, label %edge5063

edge5063:                                         ; preds = %edge5058
  %t3557.not = icmp eq i64 %v51750.ph, 0
  br i1 %t3557.not, label %block31635, label %edge5064

edge5064:                                         ; preds = %edge5063
  %t3558 = inttoptr i64 %v51750.ph to ptr
  %v52599 = load i64, ptr %t3558, align 1
  %t3559 = icmp eq i64 %v52599, 1
  br i1 %t3559, label %block31634, label %block31635

edge5069:                                         ; preds = %block31635
  %v52609 = load i64, ptr %ss1, align 8
  %v52610 = load i64, ptr %t606, align 8
  %v52611 = load i64, ptr %t607, align 8
  %.pre20669 = add i64 %v52609, %v52118
  %.pre20670 = inttoptr i64 %.pre20669 to ptr
  br label %block31634

edge5072:                                         ; preds = %block31633
  %t3583 = inttoptr i64 %v52592 to ptr
  %v52616 = load i64, ptr %t3583, align 1
  %v52617 = add i64 %v52616, 1
  store i64 %v52617, ptr %t3583, align 1
  br label %block31639

edge5075:                                         ; preds = %block31639
  %t3587 = inttoptr i64 %v51750.ph to ptr
  %v52620 = load i64, ptr %t3587, align 1
  %v52621 = add i64 %v52620, -1
  store i64 %v52621, ptr %t3587, align 1
  %t3589.not = icmp eq i64 %v52621, 0
  br i1 %t3589.not, label %edge5078, label %block2587.outer.backedge

block2587.outer:                                  ; preds = %edge4881, %block2587.outer.backedge
  %v51733.ph = phi i64 [ %v52537, %block2587.outer.backedge ], [ %v10826.ph.ph, %edge4881 ]
  %v51735.ph = phi i64 [ %v52539, %block2587.outer.backedge ], [ %v10798.ph.ph, %edge4881 ]
  %v51738.ph = phi i64 [ %v52542, %block2587.outer.backedge ], [ %v10789.ph.ph, %edge4881 ]
  %v51745.ph = phi i64 [ %v52587, %block2587.outer.backedge ], [ %v40030, %edge4881 ]
  %v51747.ph = phi i64 [ %v52589, %block2587.outer.backedge ], [ %v40032, %edge4881 ]
  %v51750.ph = phi i64 [ %v52592, %block2587.outer.backedge ], [ %v40035, %edge4881 ]
  %v51757.ph = phi i64 [ %v52110, %block2587.outer.backedge ], [ 0, %edge4881 ]
  br label %block2587

edge5078:                                         ; preds = %edge5075
  %v52622 = call i64 @mech_compact_free(i64 %v0, i64 %v51750.ph)
  %t3590.not = icmp eq i64 %v52622, 0
  br i1 %t3590.not, label %block2587.outer.backedge, label %block31630

edge5117:                                         ; preds = %block2662.thread, %edge5147
  %v54813700 = phi i64 [ %v51455, %block2662.thread ], [ 0, %edge5147 ]
  %v51452691698 = phi i64 [ %v50448, %block2662.thread ], [ %v40314, %edge5147 ]
  %v54802692696 = phi i64 [ %v217326, %block2662.thread ], [ %v41283, %edge5147 ]
  %v217402377 = phi i64 [ %v40035, %block2662.thread ], [ %v51750.ph, %edge5147 ]
  %v217400376 = phi i64 [ %v40032, %block2662.thread ], [ %v51747.ph, %edge5147 ]
  %v217399375 = phi i64 [ %v40030, %block2662.thread ], [ %v51745.ph, %edge5147 ]
  %v217398374 = phi i64 [ %v10789.ph.ph, %block2662.thread ], [ %v51738.ph, %edge5147 ]
  %v217397373 = phi i64 [ %v10826.ph.ph, %block2662.thread ], [ %v51733.ph, %edge5147 ]
  %v54664 = phi i64 [ %v10828.ph.ph, %block2662.thread ], [ %v53319, %edge5147 ]
  %v54669 = phi i64 [ %v10791.ph.ph, %block2662.thread ], [ %v53324, %edge5147 ]
  %v54676 = phi i64 [ %v40042, %block2662.thread ], [ %v53304, %edge5147 ]
  %v54678 = phi i64 [ %v40044, %block2662.thread ], [ %v53306, %edge5147 ]
  %v54681 = phi i64 [ %v40047, %block2662.thread ], [ %v53309, %edge5147 ]
  %v54688 = phi i64 [ %v217321, %block2662.thread ], [ %v54382, %edge5147 ]
  %v54691 = phi i64 [ %v40054, %block2662.thread ], [ 0, %edge5147 ]
  %t3688 = icmp eq i64 %v41283, %v1395
  %v54435133 = and i1 %t3688, %v50057124.not
  %v54816 = zext i1 %v54435133 to i64
  br label %edge5300

edge5146:                                         ; preds = %block2672
  %v53686 = add nuw nsw i64 %v53331, 1
  %v53694 = shl nuw nsw i64 %v53331, 2
  %v53695 = add i64 %v53694, %v53319
  %t3599 = inttoptr i64 %v53695 to ptr
  %t3600 = load i32, ptr %t3599, align 1
  %v53696 = zext i32 %t3600 to i64
  %v53902 = add i64 %v53694, %v53304
  %t3601 = inttoptr i64 %v53902 to ptr
  %t3602 = load i32, ptr %t3601, align 1
  %v53903 = zext i32 %t3602 to i64
  %v53923 = add nuw nsw i64 %v53903, %v53696
  %t3603 = icmp samesign ult i64 %v53923, 4294967296
  br i1 %t3603, label %edge5169, label %common.ret

edge5147:                                         ; preds = %block2672
  %v54382 = add i64 %v40054, %v217321
  %t3684.not = icmp ult i64 %v54382, %v217321
  br i1 %t3684.not, label %common.ret, label %edge5117

edge5169:                                         ; preds = %edge5146
  %t3609 = icmp eq i32 %t3602, 0
  br i1 %t3609, label %block32487, label %edge5174

edge5174:                                         ; preds = %edge5169
  %t3611.not = icmp eq i64 %v53324, 0
  br i1 %t3611.not, label %block32489, label %edge5175

edge5175:                                         ; preds = %edge5174
  %t3612 = inttoptr i64 %v53324 to ptr
  %v53958 = load i64, ptr %t3612, align 1
  %t3613 = icmp eq i64 %v53958, 1
  br i1 %t3613, label %block32488, label %block32489

edge5180:                                         ; preds = %block32489
  %v53968 = load i64, ptr %ss1, align 8
  %v53969 = load i64, ptr %t606, align 8
  %v53970 = load i64, ptr %t607, align 8
  %.pre20673 = add i64 %v53968, %v53694
  %.pre20674 = inttoptr i64 %.pre20673 to ptr
  br label %block32488

edge5183:                                         ; preds = %block32487
  %t3637 = inttoptr i64 %v53951 to ptr
  %v53975 = load i64, ptr %t3637, align 1
  %v53976 = add i64 %v53975, 1
  store i64 %v53976, ptr %t3637, align 1
  br label %block32493

edge5188:                                         ; preds = %block32493
  %v53981 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v53324)
  %t3641.not = icmp eq i64 %v53981, 0
  br i1 %t3641.not, label %edge5194, label %block32484

edge5194:                                         ; preds = %edge5188, %block32493
  %t3645 = load i32, ptr %t3601, align 1
  %t3646 = icmp eq i32 %t3645, 0
  br i1 %t3646, label %block32505, label %edge5199

edge5199:                                         ; preds = %edge5194
  %t3648.not = icmp eq i64 %v53309, 0
  br i1 %t3648.not, label %block32507, label %edge5200

edge5200:                                         ; preds = %edge5199
  %t3649 = inttoptr i64 %v53309 to ptr
  %v54021 = load i64, ptr %t3649, align 1
  %t3650 = icmp eq i64 %v54021, 1
  br i1 %t3650, label %block32506, label %block32507

edge5205:                                         ; preds = %block32507
  %v54031 = load i64, ptr %ss1, align 8
  %v54032 = load i64, ptr %t606, align 8
  %v54033 = load i64, ptr %t607, align 8
  %.pre20671 = add i64 %v54031, %v53694
  %.pre20672 = inttoptr i64 %.pre20671 to ptr
  br label %block32506

edge5208:                                         ; preds = %block32505
  %t3674 = inttoptr i64 %v54014 to ptr
  %v54038 = load i64, ptr %t3674, align 1
  %v54039 = add i64 %v54038, 1
  store i64 %v54039, ptr %t3674, align 1
  br label %block32511

edge5211:                                         ; preds = %block32511
  %t3678 = inttoptr i64 %v53309 to ptr
  %v54042 = load i64, ptr %t3678, align 1
  %v54043 = add i64 %v54042, -1
  store i64 %v54043, ptr %t3678, align 1
  %t3680.not = icmp eq i64 %v54043, 0
  br i1 %t3680.not, label %edge5214, label %block2672.backedge

edge5214:                                         ; preds = %edge5211
  %v54044 = call i64 @mech_compact_free(i64 %v0, i64 %v53309)
  %t3681.not = icmp eq i64 %v54044, 0
  br i1 %t3681.not, label %block2672.backedge, label %block32502

block2672.backedge:                               ; preds = %edge5214, %edge5211, %block32511
  br label %block2672

edge5300:                                         ; preds = %edge1350, %edge1353, %edge1356, %edge5117
  %v54913 = phi i64 [ %v36623649, %edge5117 ], [ %v10822, %edge1356 ], [ %v10822, %edge1353 ], [ %v10822, %edge1350 ]
  %v54915 = phi i64 [ %v36625650, %edge5117 ], [ %v10794, %edge1356 ], [ %v10794, %edge1353 ], [ %v10794, %edge1350 ]
  %v54918 = phi i64 [ %v36628651, %edge5117 ], [ %v10787, %edge1356 ], [ %v10787, %edge1353 ], [ %v10787, %edge1350 ]
  %v54919 = phi i64 [ %v4032811191139, %edge5117 ], [ %v10823, %edge1356 ], [ %v10823, %edge1353 ], [ %v10823, %edge1350 ]
  %v54921 = phi i64 [ %v4033011221138, %edge5117 ], [ %v10796, %edge1356 ], [ %v10796, %edge1353 ], [ %v10796, %edge1350 ]
  %v54924 = phi i64 [ %v4033311241137, %edge5117 ], [ %v10788, %edge1356 ], [ %v10788, %edge1353 ], [ %v10788, %edge1350 ]
  %v54925 = phi i64 [ %v36647656, %edge5117 ], [ %v218205, %edge1356 ], [ %v218205, %edge1353 ], [ %v218205, %edge1350 ]
  %v54926 = phi i64 [ %v36650657, %edge5117 ], [ %v218206, %edge1356 ], [ %v218206, %edge1353 ], [ %v218206, %edge1350 ]
  %v54927 = phi i64 [ %v41283, %edge5117 ], [ %v10726, %edge1356 ], [ %v10726, %edge1353 ], [ %v10726, %edge1350 ]
  %v54929 = phi i64 [ %spec.select1, %edge5117 ], [ %v218209, %edge1356 ], [ %v218209, %edge1353 ], [ %v218209, %edge1350 ]
  %v54930 = phi i64 [ %v41333, %edge5117 ], [ %v218210, %edge1356 ], [ %v218210, %edge1353 ], [ %v218210, %edge1350 ]
  %v54931 = phi i64 [ %v217397373, %edge5117 ], [ %v10826.ph.ph, %edge1356 ], [ %v10826.ph.ph, %edge1353 ], [ %v10826.ph.ph, %edge1350 ]
  %v54936 = phi i64 [ %v217398374, %edge5117 ], [ %v10789.ph.ph, %edge1356 ], [ %v10789.ph.ph, %edge1353 ], [ %v10789.ph.ph, %edge1350 ]
  %v54937 = phi i64 [ %v217399375, %edge5117 ], [ %v10827, %edge1356 ], [ %v10827, %edge1353 ], [ %v10827, %edge1350 ]
  %v54939 = phi i64 [ %v217400376, %edge5117 ], [ %v10800, %edge1356 ], [ %v10800, %edge1353 ], [ %v10800, %edge1350 ]
  %v54942 = phi i64 [ %v217402377, %edge5117 ], [ %v10790, %edge1356 ], [ %v10790, %edge1353 ], [ %v10790, %edge1350 ]
  %v54943 = phi i64 [ %v54664, %edge5117 ], [ %v10828.ph.ph, %edge1356 ], [ %v10828.ph.ph, %edge1353 ], [ %v10828.ph.ph, %edge1350 ]
  %v54948 = phi i64 [ %v54669, %edge5117 ], [ %v10791.ph.ph, %edge1356 ], [ %v10791.ph.ph, %edge1353 ], [ %v10791.ph.ph, %edge1350 ]
  %v54949 = phi i64 [ %v54676, %edge5117 ], [ %v10829, %edge1356 ], [ %v10829, %edge1353 ], [ %v10829, %edge1350 ]
  %v54951 = phi i64 [ %v54678, %edge5117 ], [ %v10804, %edge1356 ], [ %v10804, %edge1353 ], [ %v10804, %edge1350 ]
  %v54954 = phi i64 [ %v54681, %edge5117 ], [ %v10792, %edge1356 ], [ %v10792, %edge1353 ], [ %v10792, %edge1350 ]
  %v54955 = phi i64 [ %v54688, %edge5117 ], [ %v218213, %edge1356 ], [ %v218213, %edge1353 ], [ %v218213, %edge1350 ]
  %v54956 = phi i64 [ %v54691, %edge5117 ], [ %v218214, %edge1356 ], [ %v218214, %edge1353 ], [ %v218214, %edge1350 ]
  %v54957 = phi i64 [ %v217320, %edge5117 ], [ %v218217, %edge1356 ], [ %v218217, %edge1353 ], [ %v218217, %edge1350 ]
  %v54958 = phi i64 [ %v54802692696, %edge5117 ], [ %v218218, %edge1356 ], [ %v218218, %edge1353 ], [ %v218218, %edge1350 ]
  %v54959 = phi i64 [ %v51452691698, %edge5117 ], [ %v218221, %edge1356 ], [ %v218221, %edge1353 ], [ %v218221, %edge1350 ]
  %v54960 = phi i64 [ %v54813700, %edge5117 ], [ %v218222, %edge1356 ], [ %v218222, %edge1353 ], [ %v218222, %edge1350 ]
  %v54961 = phi i64 [ %v54816, %edge5117 ], [ %v218225, %edge1356 ], [ %v218225, %edge1353 ], [ %v218225, %edge1350 ]
  %v54962 = phi i64 [ %v21735020992, %edge5117 ], [ %v218226, %edge1356 ], [ %v218226, %edge1353 ], [ %v218226, %edge1350 ]
  %v54964 = phi i64 [ %v217327, %edge5117 ], [ %v10739, %edge1356 ], [ %v10739, %edge1353 ], [ %v10739, %edge1350 ]
  %v208852 = phi i64 [ %v217294, %edge5117 ], [ %v12011, %edge1356 ], [ %v12011, %edge1353 ], [ %v12011, %edge1350 ]
  %v208952 = phi i64 [ %v217296, %edge5117 ], [ %v12415, %edge1356 ], [ %v12415, %edge1353 ], [ %v12415, %edge1350 ]
  %t3691.not = icmp ult i64 %v54958, %v54957
  br i1 %t3691.not, label %common.ret, label %edge5301

edge5301:                                         ; preds = %edge5300
  %v54999 = sub nuw i64 %v54958, %v54957
  %t3695 = icmp ne i64 %v54957, 0
  %t3699 = icmp ne i64 %v54961, 1
  %t3700 = icmp ne i64 %v54959, %v54999
  %.not206 = or i1 %t3700, %t3699
  %v55218135.not = or i1 %t3695, %.not206
  %t3701.not = icmp eq i64 %v54924, 0
  br i1 %t3701.not, label %edge5328, label %edge5324

edge5324:                                         ; preds = %edge5301
  %t3702 = inttoptr i64 %v54924 to ptr
  %v55229 = load i64, ptr %t3702, align 1
  %v55230 = add i64 %v55229, 1
  store i64 %v55230, ptr %t3702, align 1
  br label %edge5328

edge5328:                                         ; preds = %edge5324, %edge5301
  %t3704.not = icmp eq i64 %v54948, 0
  br i1 %t3704.not, label %edge5365, label %edge5329

edge5329:                                         ; preds = %edge5328
  %t3705 = inttoptr i64 %v54948 to ptr
  %v55235 = load i64, ptr %t3705, align 1
  %v55236 = add i64 %v55235, 1
  store i64 %v55236, ptr %t3705, align 1
  br label %edge5365

edge5345:                                         ; preds = %edge5368
  %t3714 = insertvalue { i64, i64 } poison, i64 %v55819, 0
  %t3715 = insertvalue { i64, i64 } %t3714, i64 2807, 1
  br label %common.ret

edge5365:                                         ; preds = %edge5328, %edge5329
  %v55961 = call i64 @mech_compact_allocate(i64 %v0, i64 320, i64 4, i64 4, i64 1, i64 %v3192)
  %t3728.not = icmp eq i64 %v55961, 0
  br i1 %t3728.not, label %edge5367, label %block33195

edge5367:                                         ; preds = %edge5365
  %v55964 = load i64, ptr %ss1, align 8
  %v55965 = load i64, ptr %t606, align 8
  %v55966 = load i64, ptr %t607, align 8
  %t3731.not = icmp eq i64 %v55966, 0
  br i1 %t3731.not, label %block2804, label %edge5368

edge5368:                                         ; preds = %edge5367
  %t3732 = inttoptr i64 %v55966 to ptr
  %v55967 = load i64, ptr %t3732, align 1
  %v55815 = add i64 %v55967, 2
  store i64 %v55815, ptr %t3732, align 1
  %v55819 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v55966)
  %t3713.not = icmp eq i64 %v55819, 0
  br i1 %t3713.not, label %block2804, label %edge5345

edge5374:                                         ; preds = %block2804
  %v56457 = load i64, ptr %t910, align 1
  %v56458 = add i64 %v56457, 1
  store i64 %v56458, ptr %t910, align 1
  br label %block2806.preheader

edge5382:                                         ; preds = %block2806
  %t3740 = inttoptr i64 %v56477 to ptr
  %v56478 = load i64, ptr %t3740, align 1
  %t3741 = icmp eq i64 %v56478, 1
  %t3743 = icmp ugt i64 %v56511, 319
  %or.cond304 = select i1 %t3741, i1 %t3743, i1 false
  br i1 %or.cond304, label %edge5386, label %edge5416.thread

edge5386:                                         ; preds = %edge5382
  %t3745 = zext i64 %v54957 to i128
  %t3746 = zext i64 %v54999 to i128
  %t3747 = shl nuw i128 %t3746, 64
  %v218412 = or disjoint i128 %t3747, %t3745
  store i128 %v218412, ptr %ss2, align 16
  %t3749 = zext i64 %v56514 to i128
  %v218414 = or disjoint i128 %t934, %t3749
  store i128 %v218414, ptr %t926, align 16
  store i128 %v218112, ptr %t931, align 16
  %t3760 = zext i64 %v56519 to i128
  %t3761 = shl nuw i128 %t3760, 64
  %v218418 = or disjoint i128 %t3761, %t942
  store i128 %v218418, ptr %t936, align 16
  store i128 18446744073709551936, ptr %t941, align 16
  %t3770 = zext i64 %v56477 to i128
  %t3771 = shl nuw i128 %t3770, 64
  %v218422 = or disjoint i128 %t3771, 320
  store i128 %v218422, ptr %t946, align 16
  store i64 %v56511, ptr %t947, align 16
  br i1 %v1061851.not1217, label %edge5389, label %edge5390

edge5389:                                         ; preds = %edge5386
  %t3779 = call fastcc { i64, i64 } @mech_compact_numeric_2821(i64 %v218103, i64 %v8479)
  br label %block33253

edge5390:                                         ; preds = %edge5386
  %t3778 = call fastcc { i64, i64 } @mech_compact_numeric_2821_vector(i64 %v218103, i64 %v8479)
  br label %block33253

edge5393:                                         ; preds = %block33253
  %v56484 = extractvalue { i64, i64 } %t3778.pn, 1
  %t3781 = icmp eq i64 %v56483, 103169
  %v56497 = select i1 %t3781, i64 103425, i64 %v56483
  %t3783 = insertvalue { i64, i64 } poison, i64 %v56497, 0
  %t3784 = insertvalue { i64, i64 } %t3783, i64 %v56484, 1
  br label %common.ret

edge5394:                                         ; preds = %block33253
  br i1 %t23.not, label %edge5573, label %edge5397

edge5397:                                         ; preds = %edge5394
  %v56502 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t3786.not = icmp eq i64 %v56502, 0
  br i1 %t3786.not, label %edge5573, label %edge5398

edge5398:                                         ; preds = %edge5397
  %t3787 = insertvalue { i64, i64 } poison, i64 %v56502, 0
  %t3788 = insertvalue { i64, i64 } %t3787, i64 2821, 1
  br label %common.ret

edge5416:                                         ; preds = %block2806
  %t3789 = icmp ult i64 %v56514, %v54999
  br i1 %t3789, label %edge5422, label %edge5506

edge5416.thread:                                  ; preds = %edge5382
  %t3789708 = icmp ult i64 %v56514, %v54999
  br i1 %t3789708, label %edge5422, label %edge5501

edge5422:                                         ; preds = %edge5416.thread, %edge5416
  %v56760 = add i64 %v56514, %v54957
  %t3792.not = icmp ult i64 %v56760, %v54957
  br i1 %t3792.not, label %common.ret, label %edge5433

edge5433:                                         ; preds = %edge5422
  %t3796 = icmp ult i64 %v56760, %v1395
  br i1 %t3796, label %edge5441, label %edge5477

edge5441:                                         ; preds = %edge5433
  br i1 %v1061851.not1217, label %edge5445, label %block33589

edge5445:                                         ; preds = %edge5441
  br i1 %t975, label %trap3803, label %ok3803

edge5477:                                         ; preds = %edge5433, %block33589
  %v57099 = phi i64 [ %2, %block33589 ], [ 0, %edge5433 ]
  %v57331 = add i64 %v57099, %v56519
  %t3809 = inttoptr i64 %v57331 to ptr
  %t3810 = load i32, ptr %t3809, align 1
  %v57455 = add i32 %t3810, 1
  %t3811.not = icmp eq i32 %t3810, -1
  br i1 %t3811.not, label %common.ret, label %edge5485

edge5485:                                         ; preds = %edge5477
  br i1 %t3739.not, label %block33774, label %edge5486

edge5486:                                         ; preds = %edge5485
  %t3822 = inttoptr i64 %v56477 to ptr
  %v57488 = load i64, ptr %t3822, align 1
  %t3823 = icmp eq i64 %v57488, 1
  br i1 %t3823, label %block33772, label %block33774

edge5491:                                         ; preds = %block33774
  %v57498 = load i64, ptr %ss1, align 8
  %v57499 = load i64, ptr %t606, align 8
  %v57500 = load i64, ptr %t607, align 8
  %.pre20675 = add i64 %v57498, %v57099
  %.pre20676 = inttoptr i64 %.pre20675 to ptr
  br label %block33772

edge5494:                                         ; preds = %block33772
  %t3847 = inttoptr i64 %v57487 to ptr
  %v57505 = load i64, ptr %t3847, align 1
  %v57506 = add i64 %v57505, 1
  store i64 %v57506, ptr %t3847, align 1
  br label %edge5505

edge5501:                                         ; preds = %edge5416.thread
  %v57526 = add i64 %v56478, 1
  store i64 %v57526, ptr %t3740, align 1
  br label %edge5506

edge5505:                                         ; preds = %block33772, %edge5494
  %t3864.not = or i1 %t3739.not, %t3844
  br i1 %t3864.not, label %edge5530, label %edge5523

edge5506:                                         ; preds = %edge5416, %edge5501
  br i1 %t23.not, label %block33794, label %edge5509

edge5509:                                         ; preds = %edge5506
  %v57535 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t3858.not = icmp eq i64 %v57535, 0
  br i1 %t3858.not, label %block33794, label %block33789

edge5513:                                         ; preds = %block33794
  %t3860 = inttoptr i64 %v56477 to ptr
  %v57537 = load i64, ptr %t3860, align 1
  %v57538 = add i64 %v57537, -1
  store i64 %v57538, ptr %t3860, align 1
  %t3862.not = icmp eq i64 %v57538, 0
  br i1 %t3862.not, label %edge5516, label %edge5573

edge5516:                                         ; preds = %edge5513
  %v57539 = call i64 @mech_compact_free(i64 %v0, i64 %v56477)
  %t3863.not = icmp eq i64 %v57539, 0
  br i1 %t3863.not, label %edge5573, label %block33789

edge5523:                                         ; preds = %edge5505
  %v57543 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v56477)
  %t3865.not = icmp eq i64 %v57543, 0
  br i1 %t3865.not, label %edge5530, label %block33789

edge5530:                                         ; preds = %edge5505, %edge5523
  %v57722 = add i64 %v56514, 1
  br label %block2806

edge5557:                                         ; preds = %block33794
  %v58074 = add i64 %v56519, 1024
  %t3869 = inttoptr i64 %v58074 to ptr
  %t3870 = load i32, ptr %t3869, align 1
  %v58104 = add i32 %t3870, 1
  br label %block34146

edge5573:                                         ; preds = %edge5397, %edge5394, %edge5516, %edge5513
  %v5807421105 = add i64 %v56519, 1024
  %t386921106 = inttoptr i64 %v5807421105 to ptr
  %t387021107 = load i32, ptr %t386921106, align 1
  %v5810421108 = add i32 %t387021107, 1
  %t3876 = inttoptr i64 %v56477 to ptr
  %v58155 = load i64, ptr %t3876, align 1
  %t3877 = icmp eq i64 %v58155, 1
  br i1 %t3877, label %block34150.thread, label %block34146

block34150.thread:                                ; preds = %edge5573
  store i32 %v5810421108, ptr %t386921106, align 1
  br label %edge5596

edge5581:                                         ; preds = %block34145
  %t3901 = inttoptr i64 %v58167 to ptr
  %v58172 = load i64, ptr %t3901, align 1
  %v58173 = add i64 %v58172, 1
  store i64 %v58173, ptr %t3901, align 1
  br label %block34150

edge5584:                                         ; preds = %block34150
  %t3905 = inttoptr i64 %v5777321111 to ptr
  %v58176 = load i64, ptr %t3905, align 1
  %v58177 = add i64 %v58176, -1
  store i64 %v58177, ptr %t3905, align 1
  %t3907.not = icmp eq i64 %v58177, 0
  br i1 %t3907.not, label %edge5587, label %edge5595

edge5587:                                         ; preds = %edge5584
  %v58178 = call i64 @mech_compact_free(i64 %v0, i64 %v5777321111)
  %t3908.not = icmp eq i64 %v58178, 0
  br i1 %t3908.not, label %edge5595, label %block34141

edge5595:                                         ; preds = %edge5587, %edge5584, %block34150
  br i1 %t3900.not138, label %edge5647, label %edge5596

edge5596:                                         ; preds = %block34150.thread, %edge5595
  %v58154212872129921341 = phi i64 [ %v56477, %block34150.thread ], [ %v58167, %edge5595 ]
  %v58151212862130021339 = phi i64 [ %v56511, %block34150.thread ], [ %v58166, %edge5595 ]
  %v58149212852130121337 = phi i64 [ %v56519, %block34150.thread ], [ %v58165, %edge5595 ]
  %t3912 = inttoptr i64 %v58154212872129921341 to ptr
  %v64391 = load i64, ptr %t3912, align 1
  %v64410 = add i64 %v64391, 5
  store i64 %v64410, ptr %t3912, align 1
  br label %edge5647

edge5647:                                         ; preds = %edge5595, %edge5596
  %t3900.not138212892129821344 = phi i1 [ false, %edge5596 ], [ true, %edge5595 ]
  %v58154212872129921342 = phi i64 [ %v58154212872129921341, %edge5596 ], [ 0, %edge5595 ]
  %v58151212862130021340 = phi i64 [ %v58151212862130021339, %edge5596 ], [ %v58166, %edge5595 ]
  %v58149212852130121338 = phi i64 [ %v58149212852130121337, %edge5596 ], [ %v58165, %edge5595 ]
  %t4451.pre-phi = phi ptr [ %t3912, %edge5596 ], [ null, %edge5595 ]
  %t4143 = inttoptr i64 %v54924 to ptr
  %v68075 = add i64 %v54919, -4
  %t4173 = zext i64 %v54919 to i128
  %v218454 = or disjoint i128 %t1719, %t4173
  %v75438 = add i64 %v54999, 257
  %t4491 = zext i64 %v75438 to i128
  %t4495 = zext i64 %v54999 to i128
  %t7115 = icmp ult i64 %v54999, 16384
  %t7183 = icmp ult i64 %v54999, 10001
  %v145280 = lshr i64 %v54999, 11
  %t7298.not = icmp eq i64 %v54936, 0
  %t7299 = inttoptr i64 %v54936 to ptr
  %t7379 = inttoptr i64 %v54948 to ptr
  %t7381.not = icmp eq i64 %v217754, 0
  %t7382 = inttoptr i64 %v217754 to ptr
  %umul211 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v217755, i64 %v54955)
  %27 = extractvalue { i64, i1 } %umul211, 1
  %v159041 = mul i64 %v54955, %v217755
  %t7447 = icmp ugt i64 %v159041, 92233720368547758
  %v159120 = mul nuw i64 %v159041, 200
  %v159198212 = lshr i64 %v159120, 9
  %v159208 = mul nuw nsw i64 %v159198212, 3
  %v159502 = mul nuw nsw i64 %v159198212, 9
  %brmerge10960 = or i1 %27, %t7447
  br label %block2907

edge5744:                                         ; preds = %block2907, %edge5762, %edge5761
  %v66492 = phi i64 [ %v64763, %edge5762 ], [ 0, %edge5761 ], [ 0, %block2907 ]
  %v67290 = phi i64 [ %v64758, %edge5762 ], [ %v64758, %edge5761 ], [ ptrtoint (ptr @mech_compact_table_2 to i64), %block2907 ]
  %v67291 = phi i64 [ %v64760, %edge5762 ], [ %v64760, %edge5761 ], [ 547, %block2907 ]
  %v65822 = or i64 %v64967, %v64999
  %t3962 = icmp ne i64 %v65822, 0
  br i1 %t3962, label %edge5769, label %edge5941

edge5761:                                         ; preds = %block2907
  %t3959.not = icmp eq i64 %v64763, 0
  br i1 %t3959.not, label %edge5744, label %edge5762

edge5762:                                         ; preds = %edge5761
  %t3960 = inttoptr i64 %v64763 to ptr
  %v65816 = load i64, ptr %t3960, align 1
  %v65817 = add i64 %v65816, 1
  store i64 %v65817, ptr %t3960, align 1
  br label %edge5744

edge5769:                                         ; preds = %edge6879, %edge6882, %edge6196, %block35415, %edge5744
  %v74577 = phi i64 [ %v64746, %edge5744 ], [ %v218469, %edge6196 ], [ %v218469, %block35415 ], [ %v67402, %edge6882 ], [ %v67402, %edge6879 ]
  %v74578 = phi i64 [ %v64747, %edge5744 ], [ %v218470, %edge6196 ], [ %v218470, %block35415 ], [ %v67394, %edge6882 ], [ %v67394, %edge6879 ]
  %v74579 = phi i64 [ %v64748, %edge5744 ], [ %v218473, %edge6196 ], [ %v218473, %block35415 ], [ %v67412, %edge6882 ], [ %v67412, %edge6879 ]
  %v74580 = phi i64 [ %v64749, %edge5744 ], [ %v218474, %edge6196 ], [ %v218474, %block35415 ], [ 1, %edge6882 ], [ 1, %edge6879 ]
  %v74581 = phi i64 [ %v64750, %edge5744 ], [ %v218477, %edge6196 ], [ %v218477, %block35415 ], [ %v67394, %edge6882 ], [ %v67394, %edge6879 ]
  %v74582 = phi i64 [ %v64751, %edge5744 ], [ %v218478, %edge6196 ], [ %v218478, %block35415 ], [ %v67326, %edge6882 ], [ %v67326, %edge6879 ]
  %v74265 = or i64 %v65822, %v64983
  %t4448.not = icmp eq i64 %v74265, 0
  br i1 %t4448.not, label %edge6930, label %edge6925

edge5769.thread:                                  ; preds = %edge6878
  br i1 %t3927, label %edge6925, label %edge6966

edge5941:                                         ; preds = %edge5744
  %t4020 = icmp eq i64 %v64749, 1
  br i1 %t4020, label %edge5950, label %common.ret

edge5950:                                         ; preds = %edge5941
  %t4030 = icmp ugt i64 %v64748, 257
  br i1 %t4030, label %block35152, label %edge5953

edge5953:                                         ; preds = %edge5950
  %t4032.not = icmp eq i64 %v64748, 0
  br i1 %t4032.not, label %block35152, label %edge5957

edge5957:                                         ; preds = %edge5953
  %t4036 = shl nuw nsw i64 %v64748, 1
  %28 = call i64 @llvm.umax.i64(i64 %t4036, i64 258)
  br label %block35152

edge5970:                                         ; preds = %block35152
  %t4054 = inttoptr i64 %v64751 to ptr
  %v67046 = load i64, ptr %t4054, align 1
  %t4055 = icmp eq i64 %v67046, 1
  %t4056 = icmp ule i64 %v67015, %v64748
  %v67049141 = and i1 %t4056, %t4055
  br i1 %v67049141, label %edge5977, label %block35163

edge5975:                                         ; preds = %block35163
  %v67056 = load i64, ptr %ss1, align 8
  %v67057 = load i64, ptr %t606, align 8
  %v67058 = load i64, ptr %t607, align 8
  br label %edge5977

edge5977:                                         ; preds = %edge5975, %edge5970
  %v67040 = phi i64 [ %v67056, %edge5975 ], [ %v64746, %edge5970 ]
  %v67042 = phi i64 [ %v67057, %edge5975 ], [ %v64748, %edge5970 ]
  %v67045 = phi i64 [ %v67058, %edge5975 ], [ %v64751, %edge5970 ]
  %v67064 = call i64 @memmove(i64 %v67040, i64 ptrtoint (ptr @mech_compact_table_8 to i64), i64 2064)
  %t4076 = icmp eq i64 %v64751, %v67045
  %t4078.not142 = icmp eq i64 %v67045, 0
  %t4078.not = or i1 %t4076, %t4078.not142
  br i1 %t4078.not, label %block35169, label %edge5980

edge5980:                                         ; preds = %edge5977
  %t4079 = inttoptr i64 %v67045 to ptr
  %v67067 = load i64, ptr %t4079, align 1
  %v67068 = add i64 %v67067, 1
  store i64 %v67068, ptr %t4079, align 1
  br label %block35169

edge5985:                                         ; preds = %block35169
  %v67073 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64751)
  %t4083.not = icmp eq i64 %v67073, 0
  br i1 %t4083.not, label %edge6154, label %block35147

edge6154:                                         ; preds = %edge5985, %block35169
  br i1 %t3701.not, label %block35370, label %edge6156

edge6156:                                         ; preds = %edge6154
  %v67294 = load i64, ptr %t4143, align 1
  %v67295 = add i64 %v67294, 1
  store i64 %v67295, ptr %t4143, align 1
  br label %block35370

edge6159:                                         ; preds = %block35370
  %t4146 = inttoptr i64 %v66492 to ptr
  %v67296 = load i64, ptr %t4146, align 1
  %v67297 = add i64 %v67296, 1
  store i64 %v67297, ptr %t4146, align 1
  br label %edge6169

edge6169:                                         ; preds = %edge6159, %block35370
  %v68869 = add i64 %v67290, 1036
  %v70413 = add i64 %v67290, 2060
  br label %block2989

edge6170:                                         ; preds = %block2989
  %t4149 = inttoptr i64 %v67326 to ptr
  %v67327 = load i64, ptr %t4149, align 1
  %t4150 = icmp eq i64 %v67327, 1
  br i1 %t4150, label %edge6176, label %edge6220.thread

edge6176:                                         ; preds = %edge6170
  %t4159 = zext i64 %v67400 to i128
  %t4160 = zext i64 %v67401 to i128
  %t4161 = shl nuw i128 %t4160, 64
  %v218448 = or disjoint i128 %t4161, %t4159
  store i128 %v218448, ptr %ss2, align 16
  %t4163 = zext i64 %v67402 to i128
  %t4164 = zext i64 %v67394 to i128
  %t4165 = shl nuw i128 %t4164, 64
  %v218450 = or disjoint i128 %t4165, %t4163
  store i128 %v218450, ptr %t926, align 16
  %v218452 = or disjoint i128 %t4165, 1
  store i128 %v218452, ptr %t931, align 16
  store i128 %v218454, ptr %t936, align 16
  store i128 %v218456, ptr %t941, align 16
  %t4183 = zext i64 %v67290 to i128
  %v218458 = or disjoint i128 %t4183, 10090369008319124733952
  store i128 %v218458, ptr %t946, align 16
  store i128 10090369008319124733953, ptr %t947, align 16
  store i128 %v218462, ptr %t1152, align 16
  store i128 4722366482869645213697, ptr %t1157, align 16
  %t4203 = zext i64 %v67412 to i128
  %t4204 = zext i64 %v67326 to i128
  %t4205 = shl nuw i128 %t4204, 64
  %v218466 = or disjoint i128 %t4205, %t4203
  store i128 %v218466, ptr %t1162, align 16
  %t4208 = icmp ule i64 %v67394, %v67412
  %v67340 = sub i64 %v67412, %v67394
  %t4209 = icmp ule i64 %v67400, %v67340
  %v67342147 = and i1 %t4208, %t4209
  br i1 %v67342147, label %edge6178, label %edge6179

edge6178:                                         ; preds = %edge6176
  %t4211 = icmp uge i64 %v67401, %v67400
  %t4212 = icmp ule i64 %v67401, %v3718
  %v67347 = sub i64 %v67401, %v67400
  %t4213 = icmp ugt i64 %v67347, 2047
  %v67350148 = and i1 %t4211, %t4212
  %v67352149 = and i1 %v67350148, %t4213
  br i1 %v67352149, label %edge6180, label %edge6181

edge6179:                                         ; preds = %edge6176
  %t4217 = call fastcc { i64, i64 } @mech_compact_numeric_3009(i64 %v0, i64 %v218103, i64 %v8479)
  br label %block35404

edge6180:                                         ; preds = %edge6178
  %t4218 = call { i64, i64 } @mech_compact_numeric_3009_no_growth_composed(i64 %v0, i64 %v218103, i64 %v8479)
  br label %block35404

edge6181:                                         ; preds = %edge6178
  %t4216 = call { i64, i64 } @mech_compact_numeric_3009_no_growth(i64 %v0, i64 %v218103, i64 %v8479)
  br label %block35404

edge6185:                                         ; preds = %block35404
  %v67344 = extractvalue { i64, i64 } %t4216.pn, 1
  %t4220 = icmp eq i64 %v67343, 103169
  %v67361 = select i1 %t4220, i64 103425, i64 %v67343
  %t4222 = insertvalue { i64, i64 } poison, i64 %v67361, 0
  %t4223 = insertvalue { i64, i64 } %t4222, i64 %v67344, 1
  br label %common.ret

edge6186:                                         ; preds = %block35404
  %v218468 = load i128, ptr %t4225, align 8
  %v218469 = trunc i128 %v218468 to i64
  %t4226 = lshr i128 %v218468, 64
  %v218470 = trunc nuw i128 %t4226 to i64
  %v218472 = load i128, ptr %t4228, align 8
  %v218473 = trunc i128 %v218472 to i64
  %t4229 = lshr i128 %v218472, 64
  %v218474 = trunc nuw i128 %t4229 to i64
  %v218476 = load i128, ptr %t4231, align 8
  %v218477 = trunc i128 %v218476 to i64
  %t4232 = lshr i128 %v218476, 64
  %v218478 = trunc nuw i128 %t4232 to i64
  br i1 %t3701.not, label %block35415, label %edge6190

edge6190:                                         ; preds = %edge6186
  %v67382 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54924)
  %t4234.not = icmp eq i64 %v67382, 0
  br i1 %t4234.not, label %block35415, label %block35396

edge6196:                                         ; preds = %block35415
  %v67386 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66492)
  %t4236.not = icmp eq i64 %v67386, 0
  br i1 %t4236.not, label %edge5769, label %block35396

edge6220:                                         ; preds = %block2989
  %t4239.not = icmp eq i64 %v67400, 0
  br i1 %t4239.not, label %edge6865, label %edge6226

edge6220.thread:                                  ; preds = %edge6170
  %t4239.not722 = icmp eq i64 %v67400, 0
  br i1 %t4239.not722, label %edge6854, label %edge6226

edge6226:                                         ; preds = %edge6220.thread, %edge6220
  %v67758 = add i64 %v67400, -1
  %v67763 = add i64 %v67401, -1
  %t4242 = icmp ult i64 %v67763, %v3718
  br i1 %t4242, label %edge6279, label %edge6297

edge6279:                                         ; preds = %edge6226
  %v68072 = shl i64 %v67401, 2
  %v68076 = add i64 %v68075, %v68072
  %t4245 = inttoptr i64 %v68076 to ptr
  %t4246 = load i32, ptr %t4245, align 1
  %29 = and i32 %t4246, 65535
  %30 = zext nneg i32 %29 to i64
  %31 = lshr i32 %t4246, 16
  %32 = zext nneg i32 %31 to i64
  %t4253 = icmp ult i32 %t4246, 18874368
  br i1 %t4253, label %edge6297, label %edge6331

edge6297:                                         ; preds = %edge6226, %edge6279
  %v68452718 = phi i64 [ %32, %edge6279 ], [ 0, %edge6226 ]
  %v68099710716 = phi i64 [ %30, %edge6279 ], [ 0, %edge6226 ]
  %v68557 = shl nuw nsw i64 %v68452718, 2
  %v68870 = add i64 %v68869, %v68557
  %t4256 = inttoptr i64 %v68870 to ptr
  %t4257 = load i32, ptr %t4256, align 1
  %v68871 = zext i32 %t4257 to i64
  br label %edge6331

edge6331:                                         ; preds = %edge6297, %edge6279
  %v68452717 = phi i64 [ %v68452718, %edge6297 ], [ %32, %edge6279 ]
  %v68099710715 = phi i64 [ %v68099710716, %edge6297 ], [ %30, %edge6279 ]
  %v68891 = phi i64 [ %v68871, %edge6297 ], [ 0, %edge6279 ]
  %v69207 = shl i64 %v67394, 3
  %v69210 = add i64 %v69207, -8
  %v69211 = add i64 %v69210, %v67402
  %t4261 = inttoptr i64 %v69211 to ptr
  %v69212 = load i64, ptr %t4261, align 1
  %v69258 = add i64 %v69212, %v68891
  %v69263 = and i64 %v69258, 4294967295
  %v69301 = shl nuw nsw i64 %v68452717, 9
  %v69306 = or disjoint i64 %v69301, 1
  %v69322 = sub i64 %v67763, %v68099710715
  br label %block3108

edge6385:                                         ; preds = %edge6790
  %v69705 = shl nuw nsw i64 %v69375, 2
  %v69706 = add i64 %v69705, %v54919
  %t4266 = inttoptr i64 %v69706 to ptr
  %t4267 = load i32, ptr %t4266, align 1
  %33 = lshr i32 %t4267, 16
  %34 = zext nneg i32 %33 to i64
  br label %edge6394

edge6394:                                         ; preds = %edge6385, %edge6790
  %v69727 = phi i64 [ %34, %edge6385 ], [ 0, %edge6790 ]
  %v69757 = sub nsw i64 256, %v69727
  %v69762 = lshr i64 %v69757, 29
  %v69768 = and i64 %v69762, 7
  %v69773 = add nsw i64 %v69727, -1
  %v69779 = lshr i64 %v69773, %v69768
  %t4271 = icmp ult i64 %v69779, 256
  br i1 %t4271, label %edge6411, label %edge6418

edge6411:                                         ; preds = %edge6394
  %v70066 = add i64 %v69779, ptrtoint (ptr @mech_compact_table_18 to i64)
  %t4274 = inttoptr i64 %v70066 to ptr
  %t4275 = load i8, ptr %t4274, align 1
  %v70067 = zext i8 %t4275 to i64
  br label %edge6418

edge6418:                                         ; preds = %edge6394, %edge6411
  %v70085 = phi i64 [ %v70067, %edge6411 ], [ 0, %edge6394 ]
  %v70091 = shl nuw nsw i64 %v69768, 1
  %v70096 = add nuw nsw i64 %v70085, %v70091
  %t4277 = icmp samesign ult i64 %v70096, 32
  br i1 %t4277, label %edge6437, label %edge6453

edge6437:                                         ; preds = %edge6418
  %v70101 = shl nuw nsw i64 %v70096, 2
  %v70414 = add i64 %v70413, %v70101
  %t4280 = inttoptr i64 %v70414 to ptr
  %t4281 = load i32, ptr %t4280, align 1
  %v70415 = zext i32 %t4281 to i64
  br label %edge6453

edge6453:                                         ; preds = %edge6437, %edge6418
  %v70435 = phi i64 [ %v70415, %edge6437 ], [ 0, %edge6418 ]
  br i1 %t4263, label %edge6470, label %edge6492

edge6470:                                         ; preds = %edge6453
  %v70747 = shl nuw nsw i64 %v69375, 2
  %v70748 = add i64 %v70747, %v54919
  %t4285 = inttoptr i64 %v70748 to ptr
  %t4286 = load i32, ptr %t4285, align 1
  %35 = and i32 %t4286, 65535
  %36 = add nsw i32 %35, -3
  %t4287 = icmp ult i32 %36, 256
  br i1 %t4287, label %edge6652, label %edge6492

edge6492:                                         ; preds = %edge6453, %edge6470
  br i1 %t23.not, label %block37660, label %edge6495

edge6495:                                         ; preds = %edge6492
  %v70867 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t4290.not = icmp eq i64 %v70867, 0
  br i1 %t4290.not, label %block37660, label %block37592

edge6501:                                         ; preds = %block37660
  %v70871 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217710)
  %t4292.not = icmp eq i64 %v70871, 0
  br i1 %t4292.not, label %block37668, label %block37592

edge6513:                                         ; preds = %block37668
  %v70879 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54918)
  %t4296.not = icmp eq i64 %v70879, 0
  br i1 %t4296.not, label %block37672, label %block37592

edge6519:                                         ; preds = %block37672
  %v70883 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54924)
  %t4298.not = icmp eq i64 %v70883, 0
  br i1 %t4298.not, label %block37676, label %block37592

edge6525:                                         ; preds = %block37676
  %v70887 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54942)
  %t4300.not = icmp eq i64 %v70887, 0
  br i1 %t4300.not, label %block37680, label %block37592

edge6531:                                         ; preds = %block37680
  %v70891 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54948)
  %t4302.not = icmp eq i64 %v70891, 0
  br i1 %t4302.not, label %block37684, label %block37592

edge6537:                                         ; preds = %block37684
  %v70895 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54954)
  %t4304.not = icmp eq i64 %v70895, 0
  br i1 %t4304.not, label %block37688, label %block37592

edge6543:                                         ; preds = %block37688
  %v70899 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54936)
  %t4306.not = icmp eq i64 %v70899, 0
  br i1 %t4306.not, label %block37692, label %block37592

edge6549:                                         ; preds = %block37692
  %v70903 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217754)
  %t4308.not = icmp eq i64 %v70903, 0
  br i1 %t4308.not, label %block37696, label %block37592

edge6553:                                         ; preds = %block37696
  %v70905 = load i64, ptr %t7379, align 1
  %v70906 = add i64 %v70905, -1
  store i64 %v70906, ptr %t7379, align 1
  %t4312.not = icmp eq i64 %v70906, 0
  br i1 %t4312.not, label %edge6556, label %edge6563

edge6556:                                         ; preds = %edge6553
  %v70907 = call i64 @mech_compact_free(i64 %v0, i64 %v54948)
  %t4313.not = icmp eq i64 %v70907, 0
  br i1 %t4313.not, label %edge6563, label %block37592

edge6563:                                         ; preds = %block37696, %edge6553, %edge6556
  %t4314.not = icmp eq i64 %v64763, 0
  br i1 %t4314.not, label %block37720, label %edge6566

edge6566:                                         ; preds = %edge6563
  %v70927 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64763)
  %t4315.not = icmp eq i64 %v70927, 0
  br i1 %t4315.not, label %block37720, label %block37592

edge6572:                                         ; preds = %block37720
  %v70931 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64775)
  %t4317.not = icmp eq i64 %v70931, 0
  br i1 %t4317.not, label %block37724, label %block37592

edge6578:                                         ; preds = %block37724
  %v70935 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64793)
  %t4319.not = icmp eq i64 %v70935, 0
  br i1 %t4319.not, label %block37728, label %block37592

edge6584:                                         ; preds = %block37728
  %v70939 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64805)
  %t4321.not = icmp eq i64 %v70939, 0
  br i1 %t4321.not, label %block37732, label %block37592

edge6590:                                         ; preds = %block37732
  %v70943 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54924)
  %t4323.not = icmp eq i64 %v70943, 0
  br i1 %t4323.not, label %block37736, label %block37592

edge6594:                                         ; preds = %block37736
  %v70945 = load i64, ptr %t4451.pre-phi, align 1
  %v70946 = add i64 %v70945, -1
  store i64 %v70946, ptr %t4451.pre-phi, align 1
  %t4327.not = icmp eq i64 %v70946, 0
  br i1 %t4327.not, label %edge6597, label %edge6605

edge6597:                                         ; preds = %edge6594
  %v70947 = call i64 @mech_compact_free(i64 %v0, i64 %v58154212872129921342)
  %t4328.not = icmp eq i64 %v70947, 0
  br i1 %t4328.not, label %edge6605, label %block37592

edge6605:                                         ; preds = %edge6597, %edge6594, %block37736
  %t4329.not = icmp eq i64 %v64916, 0
  br i1 %t4329.not, label %block37764, label %edge6608

edge6608:                                         ; preds = %edge6605
  %v70971 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64916)
  %t4330.not = icmp eq i64 %v70971, 0
  br i1 %t4330.not, label %block37764, label %block37592

edge6614:                                         ; preds = %block37764
  %v70975 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64928)
  %t4332.not = icmp eq i64 %v70975, 0
  br i1 %t4332.not, label %block37768, label %block37592

edge6620:                                         ; preds = %block37768
  %v70979 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64940)
  %t4334.not = icmp eq i64 %v70979, 0
  br i1 %t4334.not, label %block37772, label %block37592

edge6626:                                         ; preds = %block37772
  %v70983 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66492)
  %t4336.not = icmp eq i64 %v70983, 0
  br i1 %t4336.not, label %block37776, label %block37592

edge6632:                                         ; preds = %block37776
  %v70987 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v67326)
  %t4338.not = icmp eq i64 %v70987, 0
  br i1 %t4338.not, label %block37780, label %block37592

edge6638:                                         ; preds = %block37780
  %v70991 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54924)
  %t4340.not = icmp eq i64 %v70991, 0
  br i1 %t4340.not, label %block37784, label %block37592

edge6644:                                         ; preds = %block37784
  %v70995 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66492)
  %t4342.not = icmp eq i64 %v70995, 0
  br i1 %t4342.not, label %common.ret, label %block37592

edge6652:                                         ; preds = %edge6470
  %37 = zext nneg i32 %35 to i64
  %v71013 = add nsw i64 %37, -2
  %v72399 = shl nuw nsw i64 %v69727, 9
  %umax20584 = call i64 @llvm.umax.i64(i64 %v69380, i64 %v71013)
  br label %block3215

edge6686:                                         ; preds = %edge6768
  %v71397 = shl nuw nsw i64 %v71085, 2
  %v71398 = add i64 %v71397, %v67290
  %t4350 = inttoptr i64 %v71398 to ptr
  %t4351 = load i32, ptr %t4350, align 1
  %v71399 = zext i32 %t4351 to i64
  br label %edge6696

edge6696:                                         ; preds = %edge6768, %edge6686
  %v71419 = phi i64 [ %v71399, %edge6686 ], [ 0, %edge6768 ]
  %v71432 = add nuw nsw i64 %v71419, %v70435
  %v71450 = sub i64 %v67394, %v71085
  %t4352 = icmp ult i64 %v71450, %v67394
  br i1 %t4352, label %edge6718, label %edge6726

edge6718:                                         ; preds = %edge6696
  %v71762 = shl i64 %v71450, 3
  %v71763 = add i64 %v71762, %v67402
  %t4355 = inttoptr i64 %v71763 to ptr
  %v71764 = load i64, ptr %t4355, align 1
  br label %edge6726

edge6726:                                         ; preds = %edge6696, %edge6718
  %v71784 = phi i64 [ %v71764, %edge6718 ], [ 0, %edge6696 ]
  %v71808 = add i64 %v71432, %v71784
  %v71813 = and i64 %v71808, 4294967295
  %t4356 = icmp samesign ult i64 %v71813, %v71072
  %spec.select29 = call i64 @llvm.umin.i64(i64 %v71813, i64 %v71072)
  %v72404 = or i64 %v71085, %v72399
  %v72409 = and i64 %v72404, 4294967295
  %v72437 = select i1 %t4356, i64 %v72409, i64 %v71077
  %v72433 = add i64 %v71067, 1
  br label %block3215

edge6768:                                         ; preds = %block3215
  %v71085 = add i64 %v71067, 3
  %t4347 = icmp ult i64 %v71085, 547
  br i1 %t4347, label %edge6686, label %edge6696

edge6781:                                         ; preds = %block3215
  %v72856 = add nuw nsw i64 %v69375, 1
  br label %block3108

edge6790:                                         ; preds = %block3108
  %t4263 = icmp ult i64 %v69375, %v3718
  br i1 %t4263, label %edge6385, label %edge6394

edge6805:                                         ; preds = %block3108
  %v73654 = shl nuw i64 %v69390, 32
  %v73659 = or i64 %v73654, %v69385
  %t4369 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v67394, i64 1)
  %v73667 = extractvalue { i64, i1 } %t4369, 0
  %t4370 = extractvalue { i64, i1 } %t4369, 1
  br i1 %t4370, label %block39206, label %edge6817

edge6817:                                         ; preds = %edge6805
  %t4373.not = icmp ugt i64 %v73667, %v67412
  br i1 %t4373.not, label %edge6820, label %block39210

edge6820:                                         ; preds = %edge6817
  %t4375.not = icmp eq i64 %v67412, 0
  br i1 %t4375.not, label %block39213, label %edge6821

edge6821:                                         ; preds = %edge6820
  %38 = add i64 %v67412, -16385
  %v73677144 = icmp ult i64 %38, -15873
  br i1 %v73677144, label %edge6824, label %edge6833

edge6824:                                         ; preds = %edge6821
  %t4379 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v67412, i64 %v67412)
  %v73678 = extractvalue { i64, i1 } %t4379, 0
  %t4380 = extractvalue { i64, i1 } %t4379, 1
  br i1 %t4380, label %block39206, label %block39213

edge6833:                                         ; preds = %edge6821
  %t4387 = mul nuw nsw i64 %v67412, 3
  %t4391 = add nuw nsw i64 %t4387, 1
  %v73695 = lshr i64 %t4391, 1
  br label %block39213

edge6837:                                         ; preds = %block39210
  %t4397 = inttoptr i64 %v67326 to ptr
  %v73703 = load i64, ptr %t4397, align 1
  %t4398 = icmp eq i64 %v73703, 1
  %t4399 = icmp ule i64 %v73672, %v67412
  %v73706145 = and i1 %t4399, %t4398
  br i1 %v73706145, label %block39220, label %block39221

edge6842:                                         ; preds = %block39221
  %v73713 = load i64, ptr %ss1, align 8
  %v73714 = load i64, ptr %t606, align 8
  %v73715 = load i64, ptr %t607, align 8
  br label %block39220

edge6844:                                         ; preds = %block39220
  %t4422 = inttoptr i64 %v73702 to ptr
  %v73721 = load i64, ptr %t4422, align 1
  %v73722 = add i64 %v73721, 1
  store i64 %v73722, ptr %t4422, align 1
  br label %edge6863

edge6854:                                         ; preds = %edge6220.thread
  %v73754 = add i64 %v67327, 1
  store i64 %v73754, ptr %t4149, align 1
  br label %edge6865

edge6863:                                         ; preds = %edge6844, %block39220
  %t4441.not = or i1 %t4148.not, %t4419
  br i1 %t4441.not, label %block2989.backedge, label %edge6887

block2989.backedge:                               ; preds = %edge6863, %edge6887, %edge6890
  br label %block2989

edge6865:                                         ; preds = %edge6220, %edge6854
  br i1 %t3701.not, label %block39259, label %edge6868

edge6868:                                         ; preds = %edge6865
  %v73793 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54924)
  %t4433.not = icmp eq i64 %v73793, 0
  br i1 %t4433.not, label %block39259, label %block39254

edge6874:                                         ; preds = %block39259
  %v73797 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66492)
  %t4435.not = icmp eq i64 %v73797, 0
  br i1 %t4435.not, label %edge6878, label %block39254

edge6878:                                         ; preds = %edge6874, %block39259
  %t4436.not = icmp eq i64 %v67326, 0
  br i1 %t4436.not, label %edge5769.thread, label %edge6879

edge6879:                                         ; preds = %edge6878
  %t4437 = inttoptr i64 %v67326 to ptr
  %v73803 = load i64, ptr %t4437, align 1
  %v73804 = add i64 %v73803, -1
  store i64 %v73804, ptr %t4437, align 1
  %t4439.not = icmp eq i64 %v73804, 0
  br i1 %t4439.not, label %edge6882, label %edge5769

edge6882:                                         ; preds = %edge6879
  %v73805 = call i64 @mech_compact_free(i64 %v0, i64 %v67326)
  %t4440.not = icmp eq i64 %v73805, 0
  br i1 %t4440.not, label %edge5769, label %block39254

edge6887:                                         ; preds = %edge6863
  %t4442 = inttoptr i64 %v67326 to ptr
  %v73807 = load i64, ptr %t4442, align 1
  %v73808 = add i64 %v73807, -1
  store i64 %v73808, ptr %t4442, align 1
  %t4444.not = icmp eq i64 %v73808, 0
  br i1 %t4444.not, label %edge6890, label %block2989.backedge

edge6890:                                         ; preds = %edge6887
  %v73809 = call i64 @mech_compact_free(i64 %v0, i64 %v67326)
  %t4445.not = icmp eq i64 %v73809, 0
  br i1 %t4445.not, label %block2989.backedge, label %block39254

edge6923:                                         ; preds = %block42353.thread, %block42353, %edge7373, %edge6926, %edge6925
  %v7458221148 = phi i64 [ %v7458221149, %edge6926 ], [ %v7458221149, %edge6925 ], [ %v745822114721157, %edge7373 ], [ %v745822114721157, %block42353 ], [ %v745822114721157, %block42353.thread ]
  %v7458121144 = phi i64 [ %v7458121146, %edge6926 ], [ %v7458121146, %edge6925 ], [ %v745812114321158, %edge7373 ], [ %v745812114321158, %block42353 ], [ %v745812114321158, %block42353.thread ]
  %v7458021140 = phi i64 [ %v7458021142, %edge6926 ], [ %v7458021142, %edge6925 ], [ %v745802113921159, %edge7373 ], [ %v745802113921159, %block42353 ], [ %v745802113921159, %block42353.thread ]
  %v7457921136 = phi i64 [ %v7457921137, %edge6926 ], [ %v7457921137, %edge6925 ], [ %v745792113821160, %edge7373 ], [ %v745792113821160, %block42353 ], [ %v745792113821160, %block42353.thread ]
  %v7457821133 = phi i64 [ %v7457821135, %edge6926 ], [ %v7457821135, %edge6925 ], [ %v745782113221161, %edge7373 ], [ %v745782113221161, %block42353 ], [ %v745782113221161, %block42353.thread ]
  %v7457721129 = phi i64 [ %v7457721131, %edge6926 ], [ %v7457721131, %edge6925 ], [ %v745772112821162, %edge7373 ], [ %v745772112821162, %block42353 ], [ %v745772112821162, %block42353.thread ]
  %v80920 = phi i64 [ %v58154212872129921342, %edge6926 ], [ 0, %edge6925 ], [ %v79890, %edge7373 ], [ %v79890, %block42353 ], [ %v75595, %block42353.thread ]
  %v80926 = phi i64 [ %v58149212852130121338, %edge6926 ], [ %v58149212852130121338, %edge6925 ], [ %v79888, %edge7373 ], [ %v79888, %block42353 ], [ %v75635, %block42353.thread ]
  %v80928 = phi i64 [ %v58151212862130021340, %edge6926 ], [ %v58151212862130021340, %edge6925 ], [ %v79889, %edge7373 ], [ %v79889, %block42353 ], [ %v75627, %block42353.thread ]
  br i1 %t3962, label %edge7381, label %edge7390

edge6925:                                         ; preds = %edge5769.thread, %edge5769
  %v7458221149 = phi i64 [ 0, %edge5769.thread ], [ %v74582, %edge5769 ]
  %v7458121146 = phi i64 [ %v67394, %edge5769.thread ], [ %v74581, %edge5769 ]
  %v7458021142 = phi i64 [ 1, %edge5769.thread ], [ %v74580, %edge5769 ]
  %v7457921137 = phi i64 [ %v67412, %edge5769.thread ], [ %v74579, %edge5769 ]
  %v7457821135 = phi i64 [ %v67394, %edge5769.thread ], [ %v74578, %edge5769 ]
  %v7457721131 = phi i64 [ %v67402, %edge5769.thread ], [ %v74577, %edge5769 ]
  br i1 %t3900.not138212892129821344, label %edge6923, label %edge6926

edge6926:                                         ; preds = %edge6925
  %v74564 = load i64, ptr %t4451.pre-phi, align 1
  %v74565 = add i64 %v74564, 1
  store i64 %v74565, ptr %t4451.pre-phi, align 1
  br label %edge6923

edge6930:                                         ; preds = %edge5769
  %t4453.not = icmp eq i64 %v74582, 0
  br i1 %t4453.not, label %edge6966, label %edge6931

edge6931:                                         ; preds = %edge6930
  %t4454 = inttoptr i64 %v74582 to ptr
  %v74583 = load i64, ptr %t4454, align 1
  %v74584 = add i64 %v74583, 1
  store i64 %v74584, ptr %t4454, align 1
  br label %edge6966

edge6946:                                         ; preds = %edge6969
  %t4463 = insertvalue { i64, i64 } poison, i64 %v75231, 0
  %t4464 = insertvalue { i64, i64 } %t4463, i64 3431, 1
  br label %common.ret

edge6966:                                         ; preds = %edge5769.thread, %edge6931, %edge6930
  %t4453.not21163 = phi i1 [ true, %edge6930 ], [ false, %edge6931 ], [ true, %edge5769.thread ]
  %v745772112821162 = phi i64 [ %v74577, %edge6930 ], [ %v74577, %edge6931 ], [ %v67402, %edge5769.thread ]
  %v745782113221161 = phi i64 [ %v74578, %edge6930 ], [ %v74578, %edge6931 ], [ %v67394, %edge5769.thread ]
  %v745792113821160 = phi i64 [ %v74579, %edge6930 ], [ %v74579, %edge6931 ], [ %v67412, %edge5769.thread ]
  %v745802113921159 = phi i64 [ %v74580, %edge6930 ], [ %v74580, %edge6931 ], [ 1, %edge5769.thread ]
  %v745812114321158 = phi i64 [ %v74581, %edge6930 ], [ %v74581, %edge6931 ], [ %v67394, %edge5769.thread ]
  %v745822114721157 = phi i64 [ 0, %edge6930 ], [ %v74582, %edge6931 ], [ 0, %edge5769.thread ]
  %v75290 = call i64 @mech_compact_allocate(i64 %v0, i64 320, i64 4, i64 4, i64 1, i64 %v3192)
  %t4477.not = icmp eq i64 %v75290, 0
  br i1 %t4477.not, label %edge6968, label %block39752

edge6968:                                         ; preds = %edge6966
  %v75293 = load i64, ptr %ss1, align 8
  %v75294 = load i64, ptr %t606, align 8
  %v75295 = load i64, ptr %t607, align 8
  %t4480.not = icmp eq i64 %v75295, 0
  br i1 %t4480.not, label %block3417.preheader, label %edge6969

edge6969:                                         ; preds = %edge6968
  %t4481 = inttoptr i64 %v75295 to ptr
  %v75296 = load i64, ptr %t4481, align 1
  %v75227 = add i64 %v75296, 2
  store i64 %v75227, ptr %t4481, align 1
  %v75231 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v75295)
  %t4462.not = icmp eq i64 %v75231, 0
  br i1 %t4462.not, label %block3417.preheader, label %edge6946

block3417.preheader:                              ; preds = %edge6968, %edge6969
  br label %block3417

edge6987:                                         ; preds = %block3417
  %t4486 = inttoptr i64 %v75595 to ptr
  %v75596 = load i64, ptr %t4486, align 1
  %t4487 = icmp eq i64 %v75596, 1
  %t4489 = icmp ugt i64 %v75627, 319
  %or.cond306 = select i1 %t4487, i1 %t4489, i1 false
  br i1 %or.cond306, label %edge6991, label %edge7025.thread

edge6991:                                         ; preds = %edge6987
  %t4492 = zext i64 %v75629 to i128
  %t4493 = shl nuw i128 %t4492, 64
  %v218486 = or disjoint i128 %t4493, %t4491
  store i128 %v218486, ptr %ss2, align 16
  %t4496 = zext i64 %v745772112821162 to i128
  %t4497 = shl nuw i128 %t4496, 64
  %v218488 = or disjoint i128 %t4497, %t4495
  store i128 %v218488, ptr %t926, align 16
  %t4500 = zext i64 %v745782113221161 to i128
  %t4501 = zext i64 %v745802113921159 to i128
  %t4502 = shl nuw i128 %t4501, 64
  %v218490 = or disjoint i128 %t4502, %t4500
  store i128 %v218490, ptr %t931, align 16
  %t4505 = zext i64 %v745812114321158 to i128
  %t4506 = zext i64 %v75635 to i128
  %t4507 = shl nuw i128 %t4506, 64
  %v218492 = or disjoint i128 %t4507, %t4505
  store i128 %v218492, ptr %t936, align 16
  store i128 18446744073709551936, ptr %t941, align 16
  store i128 %v218496, ptr %t946, align 16
  store i128 18446744073709551875, ptr %t947, align 16
  store i128 %v218500, ptr %t1152, align 16
  store i128 18446744073709551872, ptr %t1157, align 16
  %t4536 = zext i64 %v75595 to i128
  %t4537 = shl nuw i128 %t4536, 64
  %v218504 = or disjoint i128 %t4537, 256
  store i128 %v218504, ptr %t1162, align 16
  store i64 %v75627, ptr %t1167, align 16
  %t4541 = call fastcc { i64, i64 } @mech_compact_numeric_3447(i64 %v218103, i64 %v8479)
  %v75601 = extractvalue { i64, i64 } %t4541, 0
  %t4542.not = icmp eq i64 %v75601, 0
  br i1 %t4542.not, label %edge6994, label %edge6993

edge6993:                                         ; preds = %edge6991
  %v75602 = extractvalue { i64, i64 } %t4541, 1
  %t4543 = icmp eq i64 %v75601, 103169
  %v75605 = select i1 %t4543, i64 103425, i64 %v75601
  %t4545 = insertvalue { i64, i64 } poison, i64 %v75605, 0
  %t4546 = insertvalue { i64, i64 } %t4545, i64 %v75602, 1
  br label %common.ret

edge6994:                                         ; preds = %edge6991
  br i1 %t4453.not21163, label %edge7360, label %edge6995

edge6995:                                         ; preds = %edge6994
  %t4548 = inttoptr i64 %v745822114721157 to ptr
  %v75608 = load i64, ptr %t4548, align 1
  %v75609 = add i64 %v75608, -1
  store i64 %v75609, ptr %t4548, align 1
  %t4550.not = icmp eq i64 %v75609, 0
  br i1 %t4550.not, label %edge6998, label %edge7360

edge6998:                                         ; preds = %edge6995
  %v75610 = call i64 @mech_compact_free(i64 %v0, i64 %v745822114721157)
  %t4551.not = icmp eq i64 %v75610, 0
  br i1 %t4551.not, label %edge7360, label %edge6999

edge6999:                                         ; preds = %edge6998
  %t4552 = insertvalue { i64, i64 } poison, i64 %v75610, 0
  %t4553 = insertvalue { i64, i64 } %t4552, i64 3447, 1
  br label %common.ret

edge7025:                                         ; preds = %block3417
  %t4554.not = icmp eq i64 %v75629, %v54999
  br i1 %t4554.not, label %edge7288, label %edge7031

edge7025.thread:                                  ; preds = %edge6987
  %t4554.not760 = icmp eq i64 %v75629, %v54999
  br i1 %t4554.not760, label %edge7278, label %edge7031

edge7031:                                         ; preds = %edge7025.thread, %edge7025
  %v75998 = sub i64 %v75438, %v75629
  %t4557 = icmp ult i64 %v75998, %v745782113221161
  br i1 %t4557, label %edge7056, label %edge7138

edge7056:                                         ; preds = %edge7031
  %v76316 = shl i64 %v75998, 3
  %v76317 = add i64 %v76316, %v745772112821162
  %t4560 = inttoptr i64 %v76317 to ptr
  %v76318 = load i64, ptr %t4560, align 1
  %39 = lshr i64 %v76318, 32
  %v76483 = and i64 %39, 511
  %v76498 = lshr i64 %v76318, 41
  %t4563 = icmp eq i64 %v76483, 1
  br i1 %t4563, label %edge7070, label %edge7121

edge7070:                                         ; preds = %edge7056
  %v76889 = shl nuw nsw i64 %v76498, 2
  %v77206 = and i64 %v76889, 1020
  %v77207 = add i64 %v77206, %v75635
  %t4567 = inttoptr i64 %v77207 to ptr
  %t4568 = load i32, ptr %t4567, align 1
  %v77238 = add i32 %t4568, 1
  br i1 %t4485.not, label %block40803, label %edge7099

edge7099:                                         ; preds = %edge7070
  %t4575 = inttoptr i64 %v75595 to ptr
  %v77280 = load i64, ptr %t4575, align 1
  %t4576 = icmp eq i64 %v77280, 1
  br i1 %t4576, label %block40802, label %block40803

edge7103:                                         ; preds = %block40803
  %t4603 = insertvalue { i64, i64 } poison, i64 %v77289, 0
  %t4604 = insertvalue { i64, i64 } %t4603, i64 3524, 1
  br label %common.ret

edge7104:                                         ; preds = %block40803
  %v77290 = load i64, ptr %ss1, align 8
  %v77291 = load i64, ptr %t606, align 8
  %v77292 = load i64, ptr %t607, align 8
  %.pre20719 = add i64 %v77290, %v77206
  %.pre20720 = inttoptr i64 %.pre20719 to ptr
  br label %block40802

edge7107:                                         ; preds = %block40802
  %t4600 = inttoptr i64 %v77279 to ptr
  %v77297 = load i64, ptr %t4600, align 1
  %v77298 = add i64 %v77297, 1
  store i64 %v77298, ptr %t4600, align 1
  br label %block40807

edge7121:                                         ; preds = %edge7056
  %t4605 = icmp samesign ult i64 %v76483, 259
  br i1 %t4605, label %edge7138, label %edge7173

edge7138:                                         ; preds = %edge7031, %edge7121
  %v76483744752 = phi i64 [ %v76483, %edge7121 ], [ 0, %edge7031 ]
  %v76498745750 = phi i64 [ %v76498, %edge7121 ], [ 0, %edge7031 ]
  %v77632 = add i64 %v76483744752, ptrtoint (ptr @mech_compact_table_7 to i64)
  %t4608 = inttoptr i64 %v77632 to ptr
  %t4609 = load i8, ptr %t4608, align 1
  %v77633 = zext nneg i8 %t4609 to i64
  %40 = add nuw nsw i64 %v77633, 257
  %t4610 = icmp ult i8 %t4609, 63
  br i1 %t4610, label %edge7173, label %block41299

edge7173:                                         ; preds = %edge7121, %edge7138
  %v77658759 = phi i64 [ %40, %edge7138 ], [ 257, %edge7121 ]
  %v76498745749758 = phi i64 [ %v76498745750, %edge7138 ], [ %v76498, %edge7121 ]
  %v76483744751757 = phi i64 [ %v76483744752, %edge7138 ], [ %v76483, %edge7121 ]
  %v78090 = shl nuw nsw i64 %v77658759, 2
  %v78091 = add i64 %v78090, %v75635
  %t4613 = inttoptr i64 %v78091 to ptr
  %t4614 = load i32, ptr %t4613, align 1
  %41 = add i32 %t4614, 1
  br i1 %t4485.not, label %block41304, label %edge7174

edge7174:                                         ; preds = %edge7173
  %t4622 = inttoptr i64 %v75595 to ptr
  %v78163 = load i64, ptr %t4622, align 1
  %t4623 = icmp eq i64 %v78163, 1
  br i1 %t4623, label %block41302, label %block41304

edge7179:                                         ; preds = %block41304
  %v78173 = load i64, ptr %ss1, align 8
  %v78174 = load i64, ptr %t606, align 8
  %v78175 = load i64, ptr %t607, align 8
  %.pre20721 = add i64 %v78173, %v78090
  %.pre20722 = inttoptr i64 %.pre20721 to ptr
  br label %block41302

edge7182:                                         ; preds = %block41302
  %t4647 = inttoptr i64 %v78162 to ptr
  %v78180 = load i64, ptr %t4647, align 1
  %v78181 = add i64 %v78180, 1
  store i64 %v78181, ptr %t4647, align 1
  br label %block41308

edge7201:                                         ; preds = %block41308
  %v78531 = add i64 %v78234, ptrtoint (ptr @mech_compact_table_18 to i64)
  %t4657 = inttoptr i64 %v78531 to ptr
  %t4658 = load i8, ptr %t4657, align 1
  %v78532 = zext i8 %t4658 to i64
  br label %edge7208

edge7208:                                         ; preds = %block41308, %edge7201
  %v78550 = phi i64 [ %v78532, %edge7201 ], [ 0, %block41308 ]
  %v78556 = shl nuw nsw i64 %v78223, 1
  %v78561 = add nuw nsw i64 %v78550, %v78556
  %t4660 = icmp samesign ult i64 %v78561, 32
  br i1 %t4660, label %edge7245, label %block41792

edge7245:                                         ; preds = %edge7208
  %v78674 = shl nuw nsw i64 %v78561, 2
  %v78996 = or disjoint i64 %v78674, 1152
  %v78997 = add i64 %v78996, %v78157
  %t4663 = inttoptr i64 %v78997 to ptr
  %t4664 = load i32, ptr %t4663, align 1
  %42 = add i32 %t4664, 1
  %t4671.not = icmp eq i64 %v78182, 0
  br i1 %t4671.not, label %block41798, label %edge7248

edge7248:                                         ; preds = %edge7245
  %v79066 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v78182)
  %t4672.not = icmp eq i64 %v79066, 0
  br i1 %t4672.not, label %block41798, label %block41792

edge7252:                                         ; preds = %block41798
  %t4674 = inttoptr i64 %v78162 to ptr
  %v79073 = load i64, ptr %t4674, align 1
  %t4675 = icmp eq i64 %v79073, 1
  br i1 %t4675, label %block41795, label %block41801

edge7257:                                         ; preds = %block41801
  %v79083 = load i64, ptr %ss1, align 8
  %v79084 = load i64, ptr %t606, align 8
  %v79085 = load i64, ptr %t607, align 8
  %.pre20723 = add i64 %v79083, %v78996
  %.pre20724 = inttoptr i64 %.pre20723 to ptr
  br label %block41795

edge7260:                                         ; preds = %block41795
  %t4699 = inttoptr i64 %v79072 to ptr
  %v79090 = load i64, ptr %t4699, align 1
  %v79091 = add i64 %v79090, 1
  store i64 %v79091, ptr %t4699, align 1
  br label %block41805

edge7265:                                         ; preds = %block41805
  %v79096 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v78162)
  %t4703.not = icmp eq i64 %v79096, 0
  br i1 %t4703.not, label %edge7325, label %block41792

edge7278:                                         ; preds = %edge7025.thread
  %v79138 = add i64 %v75596, 1
  store i64 %v79138, ptr %t4486, align 1
  br label %edge7288

edge7288:                                         ; preds = %edge7025, %edge7278
  br i1 %t4453.not21163, label %edge7296, label %edge7291

edge7291:                                         ; preds = %edge7288
  %v79168 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v745822114721157)
  %t4712.not = icmp eq i64 %v79168, 0
  br i1 %t4712.not, label %edge7296, label %block41842

edge7296:                                         ; preds = %edge7288, %edge7291
  %t4713.not = icmp eq i64 %v75595, 0
  br i1 %t4713.not, label %edge7344, label %edge7297

edge7297:                                         ; preds = %edge7296
  %t4714 = inttoptr i64 %v75595 to ptr
  %v79178 = load i64, ptr %t4714, align 1
  %v79179 = add i64 %v79178, -1
  store i64 %v79179, ptr %t4714, align 1
  %t4716.not = icmp eq i64 %v79179, 0
  br i1 %t4716.not, label %edge7300, label %edge7360

edge7300:                                         ; preds = %edge7297
  %v79180 = call i64 @mech_compact_free(i64 %v0, i64 %v75595)
  %t4717.not = icmp eq i64 %v79180, 0
  br i1 %t4717.not, label %edge7360, label %block41842

edge7305:                                         ; preds = %block40807
  %t4719 = inttoptr i64 %v75595 to ptr
  %v79182 = load i64, ptr %t4719, align 1
  %v79183 = add i64 %v79182, -1
  store i64 %v79183, ptr %t4719, align 1
  %t4721.not = icmp eq i64 %v79183, 0
  br i1 %t4721.not, label %edge7308, label %edge7325

edge7308:                                         ; preds = %edge7305
  %v79184 = call i64 @mech_compact_free(i64 %v0, i64 %v75595)
  %t4722.not = icmp eq i64 %v79184, 0
  br i1 %t4722.not, label %edge7325, label %block41842

edge7325:                                         ; preds = %block40807, %block41805, %edge7265, %edge7305, %edge7308
  %v76483743.ph.pn = phi i64 [ 1, %edge7308 ], [ 1, %block40807 ], [ 1, %edge7305 ], [ %v76483744751757, %edge7265 ], [ %v76483744751757, %block41805 ]
  %v79115784 = phi i64 [ %v77279, %edge7308 ], [ %v77279, %block40807 ], [ %v77279, %edge7305 ], [ %v79072, %edge7265 ], [ %v79072, %block41805 ]
  %v79112783 = phi i64 [ %v77276, %edge7308 ], [ %v77276, %block40807 ], [ %v77276, %edge7305 ], [ %v79069, %edge7265 ], [ %v79069, %block41805 ]
  %v79110782 = phi i64 [ %v77274, %edge7308 ], [ %v77274, %block40807 ], [ %v77274, %edge7305 ], [ %v79067, %edge7265 ], [ %v79067, %block41805 ]
  %v79107785 = add i64 %v76483743.ph.pn, %v75629
  br label %block3417

edge7344:                                         ; preds = %edge7296
  %v79806 = add i64 %v75635, 1024
  %t4725 = inttoptr i64 %v79806 to ptr
  %t4726 = load i32, ptr %t4725, align 1
  %v79836 = add i32 %t4726, 1
  br label %block42349

edge7360:                                         ; preds = %edge6998, %edge6994, %edge6995, %edge7300, %edge7297
  %v7980621169 = add i64 %v75635, 1024
  %t472521170 = inttoptr i64 %v7980621169 to ptr
  %t472621171 = load i32, ptr %t472521170, align 1
  %v7983621172 = add i32 %t472621171, 1
  %t4732 = inttoptr i64 %v75595 to ptr
  %v79878 = load i64, ptr %t4732, align 1
  %t4733 = icmp eq i64 %v79878, 1
  br i1 %t4733, label %block42353.thread, label %block42349

block42353.thread:                                ; preds = %edge7360
  store i32 %v7983621172, ptr %t472521170, align 1
  br label %edge6923

edge7368:                                         ; preds = %block42348
  %t4757 = inttoptr i64 %v79890 to ptr
  %v79895 = load i64, ptr %t4757, align 1
  %v79896 = add i64 %v79895, 1
  store i64 %v79896, ptr %t4757, align 1
  br label %block42353

edge7373:                                         ; preds = %block42353
  %v79901 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v7949921187)
  %t4761.not = icmp eq i64 %v79901, 0
  br i1 %t4761.not, label %edge6923, label %block42344

edge7381:                                         ; preds = %edge6923
  %t4766.not = icmp eq i64 %v64793, 0
  br i1 %t4766.not, label %block43915, label %edge7385

edge7385:                                         ; preds = %edge7381
  %v79928 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64793)
  %t4767.not = icmp eq i64 %v79928, 0
  br i1 %t4767.not, label %block43915, label %edge7386

edge7386:                                         ; preds = %edge7385
  %t4768 = insertvalue { i64, i64 } poison, i64 %v79928, 0
  %t4769 = insertvalue { i64, i64 } %t4768, i64 3675, 1
  br label %common.ret

edge7390:                                         ; preds = %edge6923
  br i1 %t3927, label %edge7391, label %edge7404

edge7391:                                         ; preds = %edge7390
  %t4772.not = icmp eq i64 %v64793, 0
  br i1 %t4772.not, label %edge10759, label %edge7395

edge7395:                                         ; preds = %edge7391
  %v80220 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64793)
  %t4773.not = icmp eq i64 %v80220, 0
  br i1 %t4773.not, label %edge10759, label %edge7396

edge7396:                                         ; preds = %edge7395
  %t4774 = insertvalue { i64, i64 } poison, i64 %v80220, 0
  %t4775 = insertvalue { i64, i64 } %t4774, i64 3682, 1
  br label %common.ret

edge7404:                                         ; preds = %edge7390
  %t4776 = icmp eq i64 %v64740, 1
  br i1 %t4776, label %edge7416, label %edge7406

edge7406:                                         ; preds = %edge7404
  %t4778.not = icmp eq i64 %v64793, 0
  br i1 %t4778.not, label %block43915, label %edge7409

edge7409:                                         ; preds = %edge7406
  %v80503 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64793)
  %t4779.not = icmp eq i64 %v80503, 0
  br i1 %t4779.not, label %block43915, label %edge7410

edge7410:                                         ; preds = %edge7409
  %t4780 = insertvalue { i64, i64 } poison, i64 %v80503, 0
  %t4781 = insertvalue { i64, i64 } %t4780, i64 3689, 1
  br label %common.ret

edge7416:                                         ; preds = %edge7404
  %t4782.not = icmp eq i64 %v80920, 0
  br i1 %t4782.not, label %block3658.preheader, label %edge7417

edge7417:                                         ; preds = %edge7416
  %t4783 = inttoptr i64 %v80920 to ptr
  %v80988 = load i64, ptr %t4783, align 1
  %v80989 = add i64 %v80988, 1
  store i64 %v80989, ptr %t4783, align 1
  br label %block3658.preheader

block3658.preheader:                              ; preds = %edge7416, %edge7417
  br label %block3658

edge7439:                                         ; preds = %block3658
  %v81639 = shl nuw nsw i64 %v81056, 2
  %v81640 = add i64 %v81639, %v80926
  %t4797 = inttoptr i64 %v81640 to ptr
  %t4798 = load i32, ptr %t4797, align 1
  %v81971 = add i64 %v81639, %v64788
  %t4799 = inttoptr i64 %v81971 to ptr
  %t4800 = load i32, ptr %t4799, align 1
  %t4801 = icmp eq i32 %t4798, %t4800
  %v81633 = add nuw nsw i64 %v81056, 1
  br i1 %t4801, label %block3658, label %edge7485

edge7440:                                         ; preds = %block3658
  br i1 %t4782.not, label %block43018, label %edge7443

edge7443:                                         ; preds = %edge7440
  %v81079 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v80920)
  %t4789.not = icmp eq i64 %v81079, 0
  br i1 %t4789.not, label %block43018, label %block43014

edge7447:                                         ; preds = %block43018
  %t4791 = inttoptr i64 %v64793 to ptr
  %v81081 = load i64, ptr %t4791, align 1
  %v81082 = add i64 %v81081, -1
  store i64 %v81082, ptr %t4791, align 1
  %t4793.not = icmp eq i64 %v81082, 0
  br i1 %t4793.not, label %edge7450, label %edge10759

edge7450:                                         ; preds = %edge7447
  %v81083 = call i64 @mech_compact_free(i64 %v0, i64 %v64793)
  %t4794.not = icmp eq i64 %v81083, 0
  br i1 %t4794.not, label %edge10759, label %block43014

edge7485:                                         ; preds = %edge7439
  br i1 %t4782.not, label %block43681, label %edge7488

edge7488:                                         ; preds = %edge7485
  %v82007 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v80920)
  %t4805.not = icmp eq i64 %v82007, 0
  br i1 %t4805.not, label %block43681, label %block43677

edge7494:                                         ; preds = %block43681
  %v82011 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64793)
  %t4807.not = icmp eq i64 %v82011, 0
  br i1 %t4807.not, label %block43915, label %block43677

edge7516:                                         ; preds = %block43915
  %v82347 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64805)
  %t4811.not = icmp eq i64 %v82347, 0
  br i1 %t4811.not, label %block43918, label %block43914

edge7522:                                         ; preds = %block43918
  %v82351 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64916)
  %t4813.not = icmp eq i64 %v82351, 0
  br i1 %t4813.not, label %block43922, label %block43914

edge7528:                                         ; preds = %block43922
  %v82355 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64928)
  %t4815.not = icmp eq i64 %v82355, 0
  br i1 %t4815.not, label %block43926, label %block43914

edge7534:                                         ; preds = %block43926
  %v82359 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64940)
  %t4817.not = icmp eq i64 %v82359, 0
  br i1 %t4817.not, label %edge7561, label %block43914

edge7561:                                         ; preds = %edge7534, %block43926
  %t4820.not = icmp eq i64 %v80920, 0
  br i1 %t4820.not, label %edge7629, label %edge7563

edge7563:                                         ; preds = %edge7561
  %t4821 = inttoptr i64 %v80920 to ptr
  %v83362 = load i64, ptr %t4821, align 1
  %v83369 = add i64 %v83362, 4
  store i64 %v83369, ptr %t4821, align 1
  br label %edge7629

edge7629:                                         ; preds = %edge7561, %edge7563
  %t6655.pre-phi = phi ptr [ %t4821, %edge7563 ], [ null, %edge7561 ]
  br label %block3784

edge7641:                                         ; preds = %edge7664
  %t4840 = insertvalue { i64, i64 } poison, i64 %v84794, 0
  %t4841 = insertvalue { i64, i64 } %t4840, i64 3828, 1
  br label %common.ret

edge7663:                                         ; preds = %block3784
  %v84846 = load i64, ptr %ss1, align 8
  %v84847 = load i64, ptr %t606, align 8
  %v84848 = load i64, ptr %t607, align 8
  %t4857.not = icmp eq i64 %v84848, 0
  br i1 %t4857.not, label %edge7701, label %edge7664

edge7664:                                         ; preds = %edge7663
  %t4858 = inttoptr i64 %v84848 to ptr
  %v84849 = load i64, ptr %t4858, align 1
  %v84790 = add i64 %v84849, 2
  store i64 %v84790, ptr %t4858, align 1
  %v84794 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v84848)
  %t4839.not = icmp eq i64 %v84794, 0
  br i1 %t4839.not, label %edge7701, label %edge7641

edge7681:                                         ; preds = %edge7704
  %t4869 = insertvalue { i64, i64 } poison, i64 %v85672, 0
  %t4870 = insertvalue { i64, i64 } %t4869, i64 3841, 1
  br label %common.ret

edge7701:                                         ; preds = %edge7664, %edge7663
  %v85720 = call i64 @mech_compact_allocate(i64 %v0, i64 %v217502, i64 4, i64 4, i64 1, i64 %v3192)
  %t4883.not = icmp eq i64 %v85720, 0
  br i1 %t4883.not, label %edge7703, label %block44389

edge7703:                                         ; preds = %edge7701
  %v85723 = load i64, ptr %ss1, align 8
  %v85724 = load i64, ptr %t606, align 8
  %v85725 = load i64, ptr %t607, align 8
  %t4886.not = icmp eq i64 %v85725, 0
  br i1 %t4886.not, label %block3810, label %edge7704

edge7704:                                         ; preds = %edge7703
  %t4887 = inttoptr i64 %v85725 to ptr
  %v85726 = load i64, ptr %t4887, align 1
  %v85668 = add i64 %v85726, 2
  store i64 %v85668, ptr %t4887, align 1
  %v85672 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v85725)
  %t4868.not = icmp eq i64 %v85672, 0
  br i1 %t4868.not, label %block3810, label %edge7681

edge7716:                                         ; preds = %block3810
  %t4895.not = icmp eq i64 %v217483, 0
  br i1 %t4895.not, label %edge8099, label %edge7718

edge7718:                                         ; preds = %edge7716
  %t4896 = inttoptr i64 %v217483 to ptr
  %v86152 = load i64, ptr %t4896, align 1
  %v86153 = add i64 %v86152, -1
  store i64 %v86153, ptr %t4896, align 1
  %t4898.not = icmp eq i64 %v86153, 0
  br i1 %t4898.not, label %edge7721, label %edge8099

edge7721:                                         ; preds = %edge7718
  %v86154 = call i64 @mech_compact_free(i64 %v0, i64 %v217483)
  %t4899.not = icmp eq i64 %v86154, 0
  br i1 %t4899.not, label %edge8099, label %edge7722

edge7722:                                         ; preds = %edge7721
  %t4900 = insertvalue { i64, i64 } poison, i64 %v86154, 0
  %t4901 = insertvalue { i64, i64 } %t4900, i64 3862, 1
  br label %common.ret

edge7732:                                         ; preds = %block3810
  %t4977 = zext nneg i64 %v217502 to i128
  %t4978 = shl nuw nsw i128 %t4977, 64
  br label %block3829

edge7733:                                         ; preds = %block3829
  %t4903 = inttoptr i64 %v86683 to ptr
  %v86684 = load i64, ptr %t4903, align 1
  %t4904 = icmp ne i64 %v86684, 1
  %t4906.not = icmp ugt i64 %v217502, %v86708
  %or.cond307 = select i1 %t4904, i1 true, i1 %t4906.not
  br i1 %or.cond307, label %edge7763, label %edge7737

edge7737:                                         ; preds = %edge7733
  %t4908 = zext nneg i64 %.mux to i128
  %v218530 = or disjoint i128 %t4978, %t4908
  store i128 %v218530, ptr %ss2, align 16
  %t4912 = zext nneg i64 %v86711 to i128
  %t4913 = zext i64 %v86712 to i128
  %t4914 = shl nuw i128 %t4913, 64
  %v218532 = or disjoint i128 %t4914, %t4912
  store i128 %v218532, ptr %t926, align 16
  %t4917 = zext i64 %v217479 to i128
  %t4918 = zext nneg i64 %v217480 to i128
  %t4919 = shl nuw nsw i128 %t4918, 64
  %v218534 = or disjoint i128 %t4919, %t4917
  store i128 %v218534, ptr %t931, align 16
  %t4923 = zext nneg i64 %v217482 to i128
  %t4924 = shl nuw nsw i128 %t4923, 64
  %v218536 = or disjoint i128 %t4924, 1
  store i128 %v218536, ptr %t936, align 16
  %t4927 = zext i64 %v86717 to i128
  %v218538 = or disjoint i128 %t4978, %t4927
  store i128 %v218538, ptr %t941, align 16
  %v218540 = or disjoint i128 %t4978, 1
  store i128 %v218540, ptr %t946, align 16
  %t4937 = zext i64 %v86683 to i128
  %t4938 = zext i64 %v86708 to i128
  %t4939 = shl nuw i128 %t4938, 64
  %v218542 = or disjoint i128 %t4939, %t4937
  store i128 %v218542, ptr %t947, align 16
  %t4942 = call fastcc { i64, i64 } @mech_compact_numeric_3872(i64 %v218103, i64 %v8479)
  %v86689 = extractvalue { i64, i64 } %t4942, 0
  %t4943.not = icmp eq i64 %v86689, 0
  br i1 %t4943.not, label %edge7740, label %edge7739

edge7739:                                         ; preds = %edge7737
  %v86690 = extractvalue { i64, i64 } %t4942, 1
  %t4944 = icmp eq i64 %v86689, 103169
  %v86693 = select i1 %t4944, i64 103425, i64 %v86689
  %t4946 = insertvalue { i64, i64 } poison, i64 %v86693, 0
  %t4947 = insertvalue { i64, i64 } %t4946, i64 %v86690, 1
  br label %common.ret

edge7740:                                         ; preds = %edge7737
  %v86694 = load i64, ptr %t1261, align 8
  %t4949.not = icmp eq i64 %v217483, 0
  br i1 %t4949.not, label %edge7877, label %edge7743

edge7743:                                         ; preds = %edge7740
  %v86699 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217483)
  %t4950.not = icmp eq i64 %v86699, 0
  br i1 %t4950.not, label %edge7877, label %edge7744

edge7744:                                         ; preds = %edge7743
  %t4951 = insertvalue { i64, i64 } poison, i64 %v86699, 0
  %t4952 = insertvalue { i64, i64 } %t4951, i64 3872, 1
  br label %common.ret

edge7763:                                         ; preds = %edge7733, %block3829
  %exitcond20587.not = icmp eq i64 %v86711, %v217502
  br i1 %exitcond20587.not, label %edge7842, label %edge7769

edge7769:                                         ; preds = %edge7763
  %v87048 = add nuw nsw i64 %v86711, %.mux
  %t4956 = icmp samesign ult i64 %v87048, %v217480
  br i1 %t4956, label %edge7798, label %edge7853

edge7798:                                         ; preds = %edge7769
  %v87370 = shl nuw nsw i64 %v87048, 2
  %v87371 = add i64 %v87370, %v217479
  %t4959 = inttoptr i64 %v87371 to ptr
  %t4960 = load i32, ptr %t4959, align 1
  %v87372 = zext i32 %t4960 to i64
  %t4961.not = icmp eq i32 %t4960, 0
  %v87446 = shl nuw nsw i64 %v87372, 10
  %v87451 = or i64 %v87446, %v86711
  br i1 %t4961.not, label %edge7853, label %edge7808

edge7808:                                         ; preds = %edge7798
  %v87750 = add i64 %v86712, 1
  %t4965 = icmp ult i64 %v86712, %v217502
  br i1 %t4965, label %edge7810, label %block45594

edge7810:                                         ; preds = %edge7808
  %v87758 = shl nuw nsw i64 %v86712, 2
  %v87759 = add i64 %v86717, %v87758
  %t4967 = inttoptr i64 %v87759 to ptr
  %t4968 = load i32, ptr %t4967, align 1
  %43 = trunc i64 %v87451 to i32
  %t4969 = icmp eq i32 %t4968, %43
  br i1 %t4969, label %block45597, label %edge7813

edge7813:                                         ; preds = %edge7810
  br i1 %t4902.not, label %block45599, label %edge7814

edge7814:                                         ; preds = %edge7813
  %t4972 = inttoptr i64 %v86683 to ptr
  %v87776 = load i64, ptr %t4972, align 1
  %t4973 = icmp eq i64 %v87776, 1
  br i1 %t4973, label %block45598, label %block45599

edge7819:                                         ; preds = %block45599
  %v87786 = load i64, ptr %ss1, align 8
  %v87787 = load i64, ptr %t606, align 8
  %v87788 = load i64, ptr %t607, align 8
  %.pre20693 = add i64 %v87786, %v87758
  %.pre20694 = inttoptr i64 %.pre20693 to ptr
  br label %block45598

edge7822:                                         ; preds = %block45597
  %t4997 = inttoptr i64 %v87769 to ptr
  %v87793 = load i64, ptr %t4997, align 1
  %v87794 = add i64 %v87793, 1
  store i64 %v87794, ptr %t4997, align 1
  br label %block45603

edge7827:                                         ; preds = %block45603
  %v87799 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v86683)
  %t5001.not = icmp eq i64 %v87799, 0
  br i1 %t5001.not, label %edge7853, label %block45594

edge7842:                                         ; preds = %edge7763
  %t5010.not = icmp eq i64 %v217483, 0
  br i1 %t5010.not, label %edge7877, label %edge7843

edge7843:                                         ; preds = %edge7842
  %t5011 = inttoptr i64 %v217483 to ptr
  %v88119 = load i64, ptr %t5011, align 1
  %v88120 = add i64 %v88119, -1
  store i64 %v88120, ptr %t5011, align 1
  %t5013.not = icmp eq i64 %v88120, 0
  br i1 %t5013.not, label %edge7846, label %edge7877

edge7846:                                         ; preds = %edge7843
  %v88121 = call i64 @mech_compact_free(i64 %v0, i64 %v217483)
  %t5014.not = icmp eq i64 %v88121, 0
  br i1 %t5014.not, label %edge7877, label %edge7847

edge7847:                                         ; preds = %edge7846
  %t5015 = insertvalue { i64, i64 } poison, i64 %v88121, 0
  %t5016 = insertvalue { i64, i64 } %t5015, i64 3939, 1
  br label %common.ret

edge7853:                                         ; preds = %edge7798, %edge7827, %block45603, %edge7769
  %v2175171147 = phi i64 [ %v86712, %edge7769 ], [ %v87750, %block45603 ], [ %v87750, %edge7827 ], [ %v86712, %edge7798 ]
  %v2175161146 = phi i64 [ %v86683, %edge7769 ], [ %v87769, %block45603 ], [ %v87769, %edge7827 ], [ %v86683, %edge7798 ]
  %v2175151145 = phi i64 [ %v86708, %edge7769 ], [ %v87766, %block45603 ], [ %v87766, %edge7827 ], [ %v86708, %edge7798 ]
  %v2175141144 = phi i64 [ %v86717, %edge7769 ], [ %v87764, %block45603 ], [ %v87764, %edge7827 ], [ %v86717, %edge7798 ]
  %v88388 = add nuw nsw i64 %v86711, 1
  br label %block3829

edge7877:                                         ; preds = %edge7842, %edge7843, %edge7846, %edge7743, %edge7740
  %v88453 = phi i64 [ %v86694, %edge7743 ], [ %v86694, %edge7740 ], [ %v86712, %edge7846 ], [ %v86712, %edge7843 ], [ %v86712, %edge7842 ]
  br label %block3911

edge7878:                                         ; preds = %block3911
  %t5019 = inttoptr i64 %v88599 to ptr
  %v88600 = load i64, ptr %t5019, align 1
  %t5020 = icmp ne i64 %v88600, 1
  %t5022.not = icmp ugt i64 %v217502, %v88626
  %or.cond308 = select i1 %t5020, i1 true, i1 %t5022.not
  br i1 %or.cond308, label %edge7899, label %edge7882

edge7882:                                         ; preds = %edge7878
  %t5024 = zext i64 %v88453 to i128
  %t5025 = zext nneg i64 %v88628 to i128
  %t5026 = shl nuw nsw i128 %t5025, 64
  %v218550 = or disjoint i128 %t5026, %t5024
  store i128 %v218550, ptr %ss2, align 16
  %t5028 = zext i64 %v88629 to i128
  %v218552 = or disjoint i128 %t4978, %t5028
  store i128 %v218552, ptr %t926, align 16
  %v218554 = or disjoint i128 %t4978, 1
  store i128 %v218554, ptr %t931, align 16
  %t5038 = zext i64 %v88599 to i128
  %t5039 = zext i64 %v88626 to i128
  %t5040 = shl nuw i128 %t5039, 64
  %v218556 = or disjoint i128 %t5040, %t5038
  store i128 %v218556, ptr %t936, align 16
  %t5045 = call fastcc { i64, i64 } @mech_compact_numeric_3956_vector(i64 %v218103, i64 %v8479)
  %v88610 = extractvalue { i64, i64 } %t5045, 0
  %t5046.not = icmp eq i64 %v88610, 0
  br i1 %t5046.not, label %edge8091, label %edge7886

edge7886:                                         ; preds = %edge7882
  %v88611 = extractvalue { i64, i64 } %t5045, 1
  %t5047 = icmp eq i64 %v88610, 103169
  %v88616 = select i1 %t5047, i64 103425, i64 %v88610
  %t5049 = insertvalue { i64, i64 } poison, i64 %v88616, 0
  %t5050 = insertvalue { i64, i64 } %t5049, i64 %v88611, 1
  br label %common.ret

edge7899:                                         ; preds = %edge7878, %block3911
  %v88676 = and i64 %v88628, 255
  %t5051 = icmp ne i64 %v88676, 0
  %t5052 = icmp ult i64 %v88676, %v88453
  %v88694159 = and i1 %t5051, %t5052
  br i1 %v88694159, label %block3929, label %block4052

edge7935:                                         ; preds = %block3929
  %t5058 = icmp ult i64 %v89017, %v217502
  br i1 %t5058, label %edge7943, label %edge7955

edge7943:                                         ; preds = %edge7935
  %v89609 = shl nuw nsw i64 %v89017, 2
  %v89610 = add i64 %v89609, %v88999
  %t5061 = inttoptr i64 %v89610 to ptr
  %t5062 = load i32, ptr %t5061, align 1
  %v89611 = zext i32 %t5062 to i64
  br label %edge7955

edge7955:                                         ; preds = %edge7935, %edge7943
  %v89633 = phi i64 [ %v89611, %edge7943 ], [ 0, %edge7935 ]
  br label %block3964

edge7981:                                         ; preds = %block3964, %edge7996
  %v90608 = phi i64 [ %v90569, %edge7996 ], [ %v89633, %block3964 ]
  %t5077 = icmp ule i64 %v90608, %v89633
  %spec.select30 = call i64 @llvm.umax.i64(i64 %v90608, i64 %v89633)
  br label %block4019

edge7991:                                         ; preds = %block3964
  %t5072 = icmp ult i64 %v89973, %v217502
  br i1 %t5072, label %edge7996, label %block4019

edge7996:                                         ; preds = %edge7991
  %v90567 = shl nuw nsw i64 %v89973, 2
  %v90568 = add i64 %v90567, %v89670
  %t5075 = inttoptr i64 %v90568 to ptr
  %t5076 = load i32, ptr %t5075, align 1
  %v90569 = zext i32 %t5076 to i64
  br label %edge7981

edge8016:                                         ; preds = %block4019
  %v90903 = shl nuw nsw i64 %v89685, 2
  %v90904 = add i64 %v90903, %v89670
  %t5082 = inttoptr i64 %v90904 to ptr
  %t5083 = load i32, ptr %t5082, align 1
  %v90905 = zext i32 %t5083 to i64
  %t5084 = icmp eq i64 %v217519, %v90905
  br i1 %t5084, label %block47704, label %edge8019

edge8019:                                         ; preds = %edge8016
  %t5086.not = icmp eq i64 %v89675, 0
  br i1 %t5086.not, label %block47706, label %edge8020

edge8020:                                         ; preds = %edge8019
  %t5087 = inttoptr i64 %v89675 to ptr
  %v90921 = load i64, ptr %t5087, align 1
  %t5088 = icmp eq i64 %v90921, 1
  br i1 %t5088, label %block47705, label %block47706

edge8025:                                         ; preds = %block47706
  %v90931 = load i64, ptr %ss1, align 8
  %v90932 = load i64, ptr %t606, align 8
  %v90933 = load i64, ptr %t607, align 8
  %.pre20695 = add i64 %v90931, %v90903
  %.pre20696 = inttoptr i64 %.pre20695 to ptr
  br label %block47705

edge8028:                                         ; preds = %block47704
  %t5112 = inttoptr i64 %v90914 to ptr
  %v90938 = load i64, ptr %t5112, align 1
  %v90939 = add i64 %v90938, 1
  store i64 %v90939, ptr %t5112, align 1
  br label %block47710

edge8033:                                         ; preds = %block47710
  %v90944 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v89675)
  %t5116.not = icmp eq i64 %v90944, 0
  br i1 %t5116.not, label %edge8037, label %block47701

edge8037:                                         ; preds = %edge8033, %block47710
  br i1 %v217518, label %edge8057, label %block3964

edge8057:                                         ; preds = %edge8037
  %v91550 = add nuw i64 %v89017, 1
  %t5121.not = icmp eq i64 %v89017, -1
  br i1 %t5121.not, label %common.ret, label %block3929

edge8072:                                         ; preds = %block4052
  %v91940 = lshr i64 %v88628, 8
  br label %block3911

edge8091:                                         ; preds = %block4052, %edge7882
  %v92026 = phi i64 [ %v88629, %edge7882 ], [ %v217521, %block4052 ]
  %v92028 = phi i64 [ %v88626, %edge7882 ], [ %v217522, %block4052 ]
  %v92031 = phi i64 [ %v88599, %edge7882 ], [ %v217523, %block4052 ]
  %t5128 = icmp ult i64 %v88453, 2
  br i1 %t5128, label %edge8096, label %edge8094

edge8094:                                         ; preds = %edge8091
  %v93856 = add i64 %v88453, -1
  br label %block4122

edge8095:                                         ; preds = %edge8187, %edge9743, %edge9744, %edge9747, %edge9520, %block49358, %edge8204, %edge8159
  %v111926 = phi i64 [ %v93799, %block49358 ], [ %v84846, %edge8159 ], [ %v93799, %edge8204 ], [ %v108893, %edge9520 ], [ %v108893, %edge9747 ], [ %v108893, %edge9744 ], [ %v108893, %edge9743 ], [ %v93445384407, %edge8187 ]
  %v111928 = phi i64 [ %v93801, %block49358 ], [ %v84847, %edge8159 ], [ %v93801, %edge8204 ], [ %v108888, %edge9520 ], [ %v108888, %edge9747 ], [ %v108888, %edge9744 ], [ %v108888, %edge9743 ], [ %v93447385406, %edge8187 ]
  %v111931 = phi i64 [ %v93804, %block49358 ], [ %v84848, %edge8159 ], [ %v93804, %edge8204 ], [ %v108856, %edge9520 ], [ %v108856, %edge9747 ], [ %v108856, %edge9744 ], [ %v108856, %edge9743 ], [ %v93450386405, %edge8187 ]
  %t6278 = icmp eq i64 %v217478, 0
  br i1 %t6278, label %edge9779, label %edge9784

edge8096:                                         ; preds = %edge8091
  %t5131 = icmp eq i64 %v88453, 1
  br i1 %t5131, label %edge8126, label %edge8099

edge8099:                                         ; preds = %edge7721, %edge7716, %edge7718, %edge8096
  %v92031796802 = phi i64 [ %v92031, %edge8096 ], [ %v85725, %edge7718 ], [ %v85725, %edge7716 ], [ %v85725, %edge7721 ]
  %t5134.not = icmp eq i64 %v92031796802, 0
  br i1 %t5134.not, label %block4113, label %edge8102

edge8102:                                         ; preds = %edge8099
  %v92351 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v92031796802)
  %t5135.not = icmp eq i64 %v92351, 0
  br i1 %t5135.not, label %block4113, label %edge8103

edge8103:                                         ; preds = %edge8102
  %t5136 = insertvalue { i64, i64 } poison, i64 %v92351, 0
  %t5137 = insertvalue { i64, i64 } %t5136, i64 4124, 1
  br label %common.ret

edge8126:                                         ; preds = %edge8096
  %t5146 = inttoptr i64 %v92026 to ptr
  %t5147 = load i32, ptr %t5146, align 1
  %t5148.not = icmp eq i64 %v92031, 0
  br i1 %t5148.not, label %edge8138, label %edge8128

edge8128:                                         ; preds = %edge8126
  %t5149 = inttoptr i64 %v92031 to ptr
  %v92925 = load i64, ptr %t5149, align 1
  %v92926 = add i64 %v92925, -1
  store i64 %v92926, ptr %t5149, align 1
  %t5151.not = icmp eq i64 %v92926, 0
  br i1 %t5151.not, label %edge8131, label %edge8138

edge8131:                                         ; preds = %edge8128
  %v92927 = call i64 @mech_compact_free(i64 %v0, i64 %v92031)
  %t5152.not = icmp eq i64 %v92927, 0
  br i1 %t5152.not, label %edge8138, label %edge8132

edge8132:                                         ; preds = %edge8131
  %t5153 = insertvalue { i64, i64 } poison, i64 %v92927, 0
  %t5154 = insertvalue { i64, i64 } %t5153, i64 4142, 1
  br label %common.ret

edge8138:                                         ; preds = %edge8126, %edge8128, %edge8131
  %44 = and i32 %t5147, 1023
  %narrow191 = call i32 @llvm.umax.i32(i32 %44, i32 1)
  %v93651 = zext nneg i32 %narrow191 to i64
  br label %block4113

edge8151:                                         ; preds = %block4113
  %t5160 = inttoptr i64 %v84848 to ptr
  %v93389 = load i64, ptr %t5160, align 1
  %t5161 = icmp eq i64 %v93389, 1
  %t5165 = inttoptr i64 %v84846 to ptr
  br i1 %t5161, label %edge8155, label %edge8164.thread

edge8155:                                         ; preds = %edge8151
  store i32 65536, ptr %t5165, align 1
  %t5166 = icmp samesign ult i64 %v217524, %v217502
  br i1 %t5166, label %edge8159, label %common.ret

edge8159:                                         ; preds = %edge8155
  %v93419 = shl nuw nsw i64 %v217524, 2
  %v93420 = add i64 %v93419, %v84846
  %t5168 = inttoptr i64 %v93420 to ptr
  store i32 65537, ptr %t5168, align 1
  br label %edge8095

edge8164:                                         ; preds = %block4113
  %t5175 = inttoptr i64 %v84846 to ptr
  %t5176 = load i32, ptr %t5175, align 1
  %t5177 = icmp eq i32 %t5176, 65536
  br i1 %t5177, label %edge8185, label %block49339

edge8164.thread:                                  ; preds = %edge8151
  %t5176804 = load i32, ptr %t5165, align 1
  %t5177805 = icmp eq i32 %t5176804, 65536
  br i1 %t5177805, label %edge8185, label %block49339

edge8176:                                         ; preds = %block49337
  %t5205 = inttoptr i64 %v93469 to ptr
  %v93474 = load i64, ptr %t5205, align 1
  %v93475 = add i64 %v93474, 1
  store i64 %v93475, ptr %t5205, align 1
  br label %block49343

edge8181:                                         ; preds = %block49343
  %v93480 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v84848)
  %t5209.not = icmp eq i64 %v93480, 0
  br i1 %t5209.not, label %edge8185, label %block49314

edge8185:                                         ; preds = %edge8164, %edge8164.thread, %edge8181, %block49343
  %v93445384407 = phi i64 [ %v93467, %block49343 ], [ %v93467, %edge8181 ], [ %v84846, %edge8164.thread ], [ %v84846, %edge8164 ]
  %v93447385406 = phi i64 [ %v93468, %block49343 ], [ %v93468, %edge8181 ], [ %v84847, %edge8164.thread ], [ %v84847, %edge8164 ]
  %v93450386405 = phi i64 [ %v93469, %block49343 ], [ %v93469, %edge8181 ], [ %v84848, %edge8164.thread ], [ 0, %edge8164 ]
  %t5204.not192389404 = phi i1 [ %t5204.not192, %block49343 ], [ %t5204.not192, %edge8181 ], [ false, %edge8164.thread ], [ true, %edge8164 ]
  %t5212 = icmp samesign ult i64 %v217524, %v217502
  br i1 %t5212, label %edge8187, label %block49349

edge8187:                                         ; preds = %edge8185
  %v93787 = shl nuw nsw i64 %v217524, 2
  %v93788 = add i64 %v93445384407, %v93787
  %t5214 = inttoptr i64 %v93788 to ptr
  %t5215 = load i32, ptr %t5214, align 1
  %t5216 = icmp eq i32 %t5215, 65537
  br i1 %t5216, label %edge8095, label %edge8190

edge8190:                                         ; preds = %edge8187
  br i1 %t5204.not192389404, label %block49354, label %edge8191

edge8191:                                         ; preds = %edge8190
  %t5219 = inttoptr i64 %v93450386405 to ptr
  %v93805 = load i64, ptr %t5219, align 1
  %t5220 = icmp eq i64 %v93805, 1
  br i1 %t5220, label %block49352, label %block49354

edge8196:                                         ; preds = %block49354
  %v93815 = load i64, ptr %ss1, align 8
  %v93816 = load i64, ptr %t606, align 8
  %v93817 = load i64, ptr %t607, align 8
  %.pre20691 = add i64 %v93815, %v93787
  %.pre20692 = inttoptr i64 %.pre20691 to ptr
  br label %block49352

edge8199:                                         ; preds = %block49352
  %t5244 = inttoptr i64 %v93804 to ptr
  %v93822 = load i64, ptr %t5244, align 1
  %v93823 = add i64 %v93822, 1
  store i64 %v93823, ptr %t5244, align 1
  br label %block49358

edge8204:                                         ; preds = %block49358
  %v93828 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v93450386405)
  %t5248.not = icmp eq i64 %v93828, 0
  br i1 %t5248.not, label %edge8095, label %block49349

edge8216:                                         ; preds = %block4122
  %t5252 = inttoptr i64 %v93873 to ptr
  %v93874 = load i64, ptr %t5252, align 1
  %t5253 = icmp ne i64 %v93874, 1
  %t5255.not = icmp ugt i64 %v217502, %v93893
  %or.cond309 = select i1 %t5253, i1 true, i1 %t5255.not
  br i1 %or.cond309, label %edge8236, label %edge8220

edge8220:                                         ; preds = %edge8216
  %t5257 = zext i64 %v93894 to i128
  %t5258 = zext i64 %v93895 to i128
  %t5259 = shl nuw i128 %t5258, 64
  %v218576 = or disjoint i128 %t5259, %t5257
  store i128 %v218576, ptr %ss2, align 16
  %t5261 = zext nneg i64 %v93896 to i128
  %t5262 = zext i64 %v93856 to i128
  %t5263 = shl nuw i128 %t5262, 64
  %v218578 = or disjoint i128 %t5263, %t5261
  store i128 %v218578, ptr %t926, align 16
  %t5266 = zext i64 %v93898 to i128
  %v218580 = or disjoint i128 %t4978, %t5266
  store i128 %v218580, ptr %t931, align 16
  %v218582 = or disjoint i128 %t4978, 1
  store i128 %v218582, ptr %t936, align 16
  %t5276 = zext i64 %v93873 to i128
  %t5277 = zext i64 %v93893 to i128
  %t5278 = shl nuw i128 %t5277, 64
  %v218584 = or disjoint i128 %t5278, %t5276
  store i128 %v218584, ptr %t941, align 16
  %t5281 = call fastcc { i64, i64 } @mech_compact_numeric_4173(i64 %v218103, i64 %v8479)
  %v93879 = extractvalue { i64, i64 } %t5281, 0
  %t5282.not = icmp eq i64 %v93879, 0
  br i1 %t5282.not, label %edge8580, label %edge8222

edge8222:                                         ; preds = %edge8220
  %v93880 = extractvalue { i64, i64 } %t5281, 1
  %t5283 = icmp eq i64 %v93879, 103169
  %v93883 = select i1 %t5283, i64 103425, i64 %v93879
  %t5285 = insertvalue { i64, i64 } poison, i64 %v93883, 0
  %t5286 = insertvalue { i64, i64 } %t5285, i64 %v93880, 1
  br label %common.ret

edge8236:                                         ; preds = %edge8216, %block4122
  %exitcond20590.not = icmp eq i64 %v93896, %v93856
  br i1 %exitcond20590.not, label %edge8580, label %edge8249

edge8249:                                         ; preds = %edge8236
  %t5290 = icmp ult i64 %v93894, %v217502
  br i1 %t5290, label %edge8266.thread, label %edge8266

edge8266.thread:                                  ; preds = %edge8249
  %v94520 = shl nuw nsw i64 %v93894, 2
  %v94521 = add i64 %v93898, %v94520
  %t5293 = inttoptr i64 %v94521 to ptr
  %t5294 = load i32, ptr %t5293, align 1
  %45 = and i32 %t5294, -1024
  %46 = zext i32 %45 to i64
  br label %edge8278

edge8266:                                         ; preds = %edge8249
  %t5295.not = icmp eq i64 %v93894, -1
  br i1 %t5295.not, label %common.ret, label %edge8278

edge8278:                                         ; preds = %edge8266.thread, %edge8266
  %v94543809 = phi i64 [ %46, %edge8266.thread ], [ 0, %edge8266 ]
  %v94682810 = add nuw i64 %v93894, 1
  %t5299 = icmp ult i64 %v94682810, %v217502
  br i1 %t5299, label %edge8286, label %edge8295

edge8286:                                         ; preds = %edge8278
  %v94996 = shl nuw nsw i64 %v94682810, 2
  %v94997 = add i64 %v93898, %v94996
  %t5302 = inttoptr i64 %v94997 to ptr
  %t5303 = load i32, ptr %t5302, align 1
  %47 = and i32 %t5303, -1024
  %48 = zext i32 %47 to i64
  br label %edge8295

edge8295:                                         ; preds = %edge8286, %edge8278
  %v95018 = phi i64 [ %48, %edge8286 ], [ 0, %edge8278 ]
  %t5304 = icmp ult i64 %v93895, %v217502
  br i1 %t5304, label %edge8321.thread, label %edge8321

edge8321.thread:                                  ; preds = %edge8295
  %v95339 = shl nuw nsw i64 %v93895, 2
  %v95340 = add i64 %v93898, %v95339
  %t5307 = inttoptr i64 %v95340 to ptr
  %t5308 = load i32, ptr %t5307, align 1
  %v95341 = zext i32 %t5308 to i64
  br label %edge8332

edge8321:                                         ; preds = %edge8295
  %t5309.not = icmp eq i64 %v93895, -1
  br i1 %t5309.not, label %common.ret, label %edge8332

edge8332:                                         ; preds = %edge8321.thread, %edge8321
  %v95361814 = phi i64 [ %v95341, %edge8321.thread ], [ 0, %edge8321 ]
  %v95370815 = add nuw i64 %v93895, 1
  %t5313 = icmp ult i64 %v95370815, %v217502
  br i1 %t5313, label %edge8349.thread, label %edge8349

edge8349.thread:                                  ; preds = %edge8332
  %v95684 = shl nuw nsw i64 %v95370815, 2
  %v95685 = add i64 %v93898, %v95684
  %t5316 = inttoptr i64 %v95685 to ptr
  %t5317 = load i32, ptr %t5316, align 1
  %v95686 = zext i32 %t5317 to i64
  %v95720818 = and i64 %v95686, 4294966272
  %49 = and i64 %v95686, 1023
  br label %edge8362

edge8349:                                         ; preds = %edge8332
  %t5321.not = icmp eq i64 %v93895, -2
  br i1 %t5321.not, label %common.ret, label %edge8362

edge8362:                                         ; preds = %edge8349.thread, %edge8349
  %v95720823 = phi i64 [ %v95720818, %edge8349.thread ], [ 0, %edge8349 ]
  %v95706821 = phi i64 [ %49, %edge8349.thread ], [ 0, %edge8349 ]
  %v95715822 = and i64 %v95361814, 4294966272
  %v95766824 = add nuw i64 %v93895, 2
  %t5318 = icmp uge i64 %v93894, %v93856
  %t5319 = icmp ne i64 %v93895, %v93896
  %t5320 = icmp samesign ugt i64 %v95018, %v95715822
  %v95750162.not166 = and i1 %t5319, %t5320
  %v95755163.not = or i1 %t5318, %v95750162.not166
  %t5325 = icmp ule i64 %v95766824, %v93896
  %t5326 = icmp ugt i64 %v93894, %v93856
  %t5327 = icmp samesign ult i64 %v95720823, %v94543809
  %v95791164 = or i1 %t5326, %t5327
  %v95796165 = and i1 %t5325, %v95791164
  %narrow = and i1 %v95755163.not, %v95796165
  %v95815 = shl i64 %v93896, 10
  br i1 %v95755163.not, label %edge8377, label %edge8459

edge8372:                                         ; preds = %block50808, %edge8401
  br i1 %narrow, label %edge8412, label %edge8450

edge8377:                                         ; preds = %edge8362
  %v96103 = and i64 %v95361814, 1023
  %v96121 = or disjoint i64 %v96103, %v95815
  br i1 %t5304, label %edge8384, label %block50799

edge8384:                                         ; preds = %edge8377
  %v96140 = shl nuw nsw i64 %v93895, 2
  %v96141 = add i64 %v93898, %v96140
  %t5333 = inttoptr i64 %v96141 to ptr
  %t5334 = load i32, ptr %t5333, align 1
  %v96142 = zext i32 %t5334 to i64
  %t5335 = icmp eq i64 %v96121, %v96142
  br i1 %t5335, label %block50802, label %edge8387

edge8387:                                         ; preds = %edge8384
  br i1 %t5251.not, label %block50804, label %edge8388

edge8388:                                         ; preds = %edge8387
  %t5338 = inttoptr i64 %v93873 to ptr
  %v96158 = load i64, ptr %t5338, align 1
  %t5339 = icmp eq i64 %v96158, 1
  br i1 %t5339, label %block50803, label %block50804

edge8393:                                         ; preds = %block50804
  %v96168 = load i64, ptr %ss1, align 8
  %v96169 = load i64, ptr %t606, align 8
  %v96170 = load i64, ptr %t607, align 8
  %.pre20701 = add i64 %v96168, %v96140
  %.pre20702 = inttoptr i64 %.pre20701 to ptr
  br label %block50803

edge8396:                                         ; preds = %block50802
  %t5363 = inttoptr i64 %v96151 to ptr
  %v96175 = load i64, ptr %t5363, align 1
  %v96176 = add i64 %v96175, 1
  store i64 %v96176, ptr %t5363, align 1
  br label %block50808

edge8401:                                         ; preds = %block50808
  %v96181 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v93873)
  %t5367.not = icmp eq i64 %v96181, 0
  br i1 %t5367.not, label %edge8372, label %block50799

edge8412:                                         ; preds = %edge8372
  %v96476 = or disjoint i64 %v95706821, %v95815
  br i1 %t5313, label %edge8421, label %block51031

edge8421:                                         ; preds = %edge8412
  %v96501 = shl nuw nsw i64 %v95370815, 2
  %v96502 = add i64 %v96146, %v96501
  %t5374 = inttoptr i64 %v96502 to ptr
  %t5375 = load i32, ptr %t5374, align 1
  %v96503 = zext i32 %t5375 to i64
  %t5376 = icmp eq i64 %v96476, %v96503
  br i1 %t5376, label %edge8450, label %edge8424

edge8424:                                         ; preds = %edge8421
  br i1 %t5362.not167, label %block51036, label %edge8425

edge8425:                                         ; preds = %edge8424
  %t5379 = inttoptr i64 %v96151 to ptr
  %v96519 = load i64, ptr %t5379, align 1
  %t5380 = icmp eq i64 %v96519, 1
  br i1 %t5380, label %block51034, label %block51036

edge8430:                                         ; preds = %block51036
  %v96529 = load i64, ptr %ss1, align 8
  %v96530 = load i64, ptr %t606, align 8
  %v96531 = load i64, ptr %t607, align 8
  %.pre20699 = add i64 %v96529, %v96501
  %.pre20700 = inttoptr i64 %.pre20699 to ptr
  br label %block51034

edge8433:                                         ; preds = %block51034
  %t5404 = inttoptr i64 %v96518 to ptr
  %v96536 = load i64, ptr %t5404, align 1
  %v96537 = add i64 %v96536, 1
  store i64 %v96537, ptr %t5404, align 1
  br label %block51040

edge8438:                                         ; preds = %block51040
  %v96542 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v96151)
  %t5408.not = icmp eq i64 %v96542, 0
  br i1 %t5408.not, label %edge8450, label %block51031

edge8450:                                         ; preds = %edge8459, %edge8438, %block51040, %edge8372, %edge8421
  %v98297835 = phi i64 [ %v93873, %edge8459 ], [ %v96151, %edge8372 ], [ %v96518, %edge8438 ], [ %v96518, %block51040 ], [ %v96151, %edge8421 ]
  %v98294834 = phi i64 [ %v93893, %edge8459 ], [ %v96148, %edge8372 ], [ %v96515, %edge8438 ], [ %v96515, %block51040 ], [ %v96148, %edge8421 ]
  %v98292833 = phi i64 [ %v93898, %edge8459 ], [ %v96146, %edge8372 ], [ %v96513, %edge8438 ], [ %v96513, %block51040 ], [ %v96146, %edge8421 ]
  %v98719 = phi i64 [ %v97113, %edge8459 ], [ %v94682810, %edge8372 ], [ %v93894, %edge8438 ], [ %v93894, %block51040 ], [ %v93894, %edge8421 ]
  %v95766.v95370 = select i1 %narrow, i64 %v95766824, i64 %v95370815
  %spec.select327 = select i1 %v95755163.not, i64 %v95766.v95370, i64 %v93895
  %exitcond20591.not = icmp eq i64 %v93896, %v217502
  br i1 %exitcond20591.not, label %block52563, label %edge8539

edge8459:                                         ; preds = %edge8362
  %v97113 = add nuw i64 %v93894, 2
  br label %edge8450

edge8539:                                         ; preds = %edge8450
  %v98289.v = select i1 %v95796165, i64 %v95720823, i64 %v94543809
  %v98289 = add nuw nsw i64 %v98289.v, %v95715822
  %v97985 = add nuw nsw i64 %v95018, %v94543809
  %v98643 = select i1 %v95755163.not, i64 %v98289, i64 %v97985
  %v98607 = shl i64 %v93896, 2
  %v98608 = add i64 %v98292833, %v98607
  %t5430 = inttoptr i64 %v98608 to ptr
  %t5431 = load i32, ptr %t5430, align 1
  %50 = and i32 %t5431, 1023
  %51 = zext nneg i32 %50 to i64
  %v98643.masked838 = and i64 %v98643, 4294967295
  %v98649839 = or i64 %v98643.masked838, %51
  %v98660840 = add nuw nsw i64 %v93896, 1
  %v98670 = zext i32 %t5431 to i64
  %t5436 = icmp eq i64 %v98649839, %v98670
  br i1 %t5436, label %block4122.backedge, label %edge8542

block4122.backedge:                               ; preds = %edge8539, %edge8556, %block52572
  %v93873.be = phi i64 [ %v98685, %block52572 ], [ %v98685, %edge8556 ], [ %v98297835, %edge8539 ]
  %v93893.be = phi i64 [ %v98682, %block52572 ], [ %v98682, %edge8556 ], [ %v98294834, %edge8539 ]
  %v93898.be = phi i64 [ %v98680, %block52572 ], [ %v98680, %edge8556 ], [ %v98292833, %edge8539 ]
  br label %block4122

edge8542:                                         ; preds = %edge8539
  %t5438.not = icmp eq i64 %v98297835, 0
  br i1 %t5438.not, label %block52568, label %edge8543

edge8543:                                         ; preds = %edge8542
  %t5439 = inttoptr i64 %v98297835 to ptr
  %v98686 = load i64, ptr %t5439, align 1
  %t5440 = icmp eq i64 %v98686, 1
  br i1 %t5440, label %block52566, label %block52568

edge8548:                                         ; preds = %block52568
  %v98696 = load i64, ptr %ss1, align 8
  %v98697 = load i64, ptr %t606, align 8
  %v98698 = load i64, ptr %t607, align 8
  %.pre20697 = add i64 %v98696, %v98607
  %.pre20698 = inttoptr i64 %.pre20697 to ptr
  br label %block52566

edge8551:                                         ; preds = %block52566
  %t5464 = inttoptr i64 %v98685 to ptr
  %v98703 = load i64, ptr %t5464, align 1
  %v98704 = add i64 %v98703, 1
  store i64 %v98704, ptr %t5464, align 1
  br label %block52572

edge8556:                                         ; preds = %block52572
  %v98709 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v98297835)
  %t5468.not = icmp eq i64 %v98709, 0
  br i1 %t5468.not, label %block4122.backedge, label %block52563

edge8580:                                         ; preds = %edge8236, %edge8220
  %t5251.not23858 = phi i1 [ false, %edge8220 ], [ %t5251.not, %edge8236 ]
  %v99091 = add i64 %v88453, -2
  %v99108 = add nuw nsw i64 %v217504, 2
  %v99918 = call i64 @mech_compact_allocate(i64 %v0, i64 %v99108, i64 8, i64 8, i64 1, i64 %v3192)
  %t5497.not = icmp eq i64 %v99918, 0
  br i1 %t5497.not, label %edge8618, label %block52864

edge8596:                                         ; preds = %edge8619
  %t5483 = insertvalue { i64, i64 } poison, i64 %v99871, 0
  %t5484 = insertvalue { i64, i64 } %t5483, i64 4430, 1
  br label %common.ret

edge8618:                                         ; preds = %edge8580
  %v99921 = load i64, ptr %ss1, align 8
  %v99922 = load i64, ptr %t606, align 8
  %v99923 = load i64, ptr %t607, align 8
  %t5500.not = icmp eq i64 %v99923, 0
  br i1 %t5500.not, label %edge8626, label %edge8619

edge8619:                                         ; preds = %edge8618
  %t5501 = inttoptr i64 %v99923 to ptr
  %v99924 = load i64, ptr %t5501, align 1
  %v99867 = add i64 %v99924, 2
  store i64 %v99867, ptr %t5501, align 1
  %v99871 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v99923)
  %t5482.not = icmp eq i64 %v99871, 0
  br i1 %t5482.not, label %edge8626.thread, label %edge8596

edge8626:                                         ; preds = %edge8618
  %v100049 = add i64 %v99921, 8
  %t5507 = inttoptr i64 %v100049 to ptr
  %v100050 = load i64, ptr %t5507, align 1
  %t5508 = icmp eq i64 %v100050, 2
  br i1 %t5508, label %edge8656, label %block52906

edge8626.thread:                                  ; preds = %edge8619
  %v100049843 = add i64 %v99921, 8
  %t5507844 = inttoptr i64 %v100049843 to ptr
  %v100050845 = load i64, ptr %t5507844, align 1
  %t5508846 = icmp eq i64 %v100050845, 2
  br i1 %t5508846, label %edge8656, label %edge8630

edge8630:                                         ; preds = %edge8626.thread
  %v100066 = load i64, ptr %t5501, align 1
  %t5512 = icmp eq i64 %v100066, 1
  br i1 %t5512, label %block52904.thread436, label %block52906

block52904.thread436:                             ; preds = %edge8630
  store i64 2, ptr %t5507844, align 1
  br label %edge8656

edge8638:                                         ; preds = %block52904
  %t5536 = inttoptr i64 %v100078 to ptr
  %v100082 = load i64, ptr %t5536, align 1
  %v100083 = add i64 %v100082, 1
  store i64 %v100083, ptr %t5536, align 1
  br label %block52910

edge8641:                                         ; preds = %block52910
  %t5540 = inttoptr i64 %v99923 to ptr
  %v100086 = load i64, ptr %t5540, align 1
  %v100087 = add i64 %v100086, -1
  store i64 %v100087, ptr %t5540, align 1
  %t5542.not = icmp eq i64 %v100087, 0
  br i1 %t5542.not, label %edge8644, label %edge8656

edge8644:                                         ; preds = %edge8641
  %v100088 = call i64 @mech_compact_free(i64 %v0, i64 %v99923)
  %t5543.not = icmp eq i64 %v100088, 0
  br i1 %t5543.not, label %edge8656, label %block52901

edge8656:                                         ; preds = %edge8626.thread, %edge8626, %block52904.thread436, %block52910, %edge8641, %edge8644
  %v100054432452 = phi i64 [ %v100076, %edge8644 ], [ %v100076, %block52910 ], [ %v100076, %edge8641 ], [ %v99921, %block52904.thread436 ], [ %v99921, %edge8626 ], [ %v99921, %edge8626.thread ]
  %v100056433451 = phi i64 [ %v100077, %edge8644 ], [ %v100077, %block52910 ], [ %v100077, %edge8641 ], [ %v99922, %block52904.thread436 ], [ %v99922, %edge8626 ], [ %v99922, %edge8626.thread ]
  %v100059434450 = phi i64 [ %v100078, %edge8644 ], [ %v100078, %block52910 ], [ %v100078, %edge8641 ], [ %v99923, %block52904.thread436 ], [ 0, %edge8626 ], [ %v99923, %edge8626.thread ]
  %t5546 = icmp ult i64 %v99091, %v217502
  br i1 %t5546, label %edge8664, label %block53159

edge8664:                                         ; preds = %edge8656
  %v100423 = shl nuw nsw i64 %v99091, 2
  %v100424 = add i64 %v93898, %v100423
  %t5549 = inttoptr i64 %v100424 to ptr
  %t5550 = load i32, ptr %t5549, align 1
  %52 = and i32 %t5550, 1023
  %t5555 = icmp ult i32 %t5550, 1024
  br i1 %t5555, label %block53162, label %edge8682

edge8682:                                         ; preds = %edge8664
  br i1 %t5251.not23858, label %block53164, label %edge8683

edge8683:                                         ; preds = %edge8682
  %t5558 = inttoptr i64 %v93873 to ptr
  %v100502 = load i64, ptr %t5558, align 1
  %t5559 = icmp eq i64 %v100502, 1
  br i1 %t5559, label %block53163, label %block53164

edge8688:                                         ; preds = %block53164
  %v100512 = load i64, ptr %ss1, align 8
  %v100513 = load i64, ptr %t606, align 8
  %v100514 = load i64, ptr %t607, align 8
  %.pre20717 = add i64 %v100512, %v100423
  %.pre20718 = inttoptr i64 %.pre20717 to ptr
  br label %block53163

edge8691:                                         ; preds = %block53162
  %t5583 = inttoptr i64 %v100495 to ptr
  %v100519 = load i64, ptr %t5583, align 1
  %v100520 = add i64 %v100519, 1
  store i64 %v100520, ptr %t5583, align 1
  br label %block53168

edge8694:                                         ; preds = %block53168
  %t5587 = inttoptr i64 %v93873 to ptr
  %v100523 = load i64, ptr %t5587, align 1
  %v100524 = add i64 %v100523, -1
  store i64 %v100524, ptr %t5587, align 1
  %t5589.not = icmp eq i64 %v100524, 0
  br i1 %t5589.not, label %edge8697, label %edge8707

edge8697:                                         ; preds = %edge8694
  %v100525 = call i64 @mech_compact_free(i64 %v0, i64 %v93873)
  %t5590.not = icmp eq i64 %v100525, 0
  br i1 %t5590.not, label %edge8707, label %block53159

edge8707:                                         ; preds = %edge8697, %edge8694, %block53168
  %t5688 = zext nneg i64 %v99108 to i128
  %t5689 = shl nuw nsw i128 %t5688, 64
  br label %block4414

edge8722:                                         ; preds = %block4414
  %v100894 = add nsw i64 %v100600, -1
  %v101208 = shl nuw nsw i64 %v100600, 2
  %v101211 = add i64 %v100576, -4
  %v101212 = add i64 %v101211, %v101208
  %t5599 = inttoptr i64 %v101212 to ptr
  %t5600 = load i32, ptr %t5599, align 1
  %v101213 = zext i32 %t5600 to i64
  %v101247 = lshr i64 %v101213, 10
  %t5602 = icmp samesign ult i64 %v101247, %v217502
  br i1 %t5602, label %edge8768, label %edge8781

edge8768:                                         ; preds = %edge8722
  %v101564 = shl nuw nsw i64 %v101247, 2
  %v101565 = add i64 %v101564, %v100576
  %t5605 = inttoptr i64 %v101565 to ptr
  %t5606 = load i32, ptr %t5605, align 1
  %53 = lshr i32 %t5606, 10
  %narrow178 = add nuw nsw i32 %53, 1
  %54 = zext nneg i32 %narrow178 to i64
  br label %edge8781

edge8781:                                         ; preds = %edge8722, %edge8768
  %v101586 = phi i64 [ %54, %edge8768 ], [ 1, %edge8722 ]
  %v101609 = and i64 %v101213, 1023
  %v101631 = shl nuw nsw i64 %v101586, 10
  %v101631.masked = and i64 %v101631, 4294966272
  %v101641 = or disjoint i64 %v101631.masked, %v101609
  %v101660 = shl nuw nsw i64 %v100894, 2
  %v101661 = add i64 %v101660, %v100576
  %t5615 = inttoptr i64 %v101661 to ptr
  %t5616 = load i32, ptr %t5615, align 1
  %v101662 = zext i32 %t5616 to i64
  %t5617 = icmp eq i64 %v101641, %v101662
  br i1 %t5617, label %block53924, label %edge8796

edge8796:                                         ; preds = %edge8781
  %t5619.not = icmp eq i64 %v100581, 0
  br i1 %t5619.not, label %block53926, label %edge8797

edge8797:                                         ; preds = %edge8796
  %t5620 = inttoptr i64 %v100581 to ptr
  %v101678 = load i64, ptr %t5620, align 1
  %t5621 = icmp eq i64 %v101678, 1
  br i1 %t5621, label %block53925, label %block53926

edge8802:                                         ; preds = %block53926
  %v101688 = load i64, ptr %ss1, align 8
  %v101689 = load i64, ptr %t606, align 8
  %v101690 = load i64, ptr %t607, align 8
  %.pre20715 = add i64 %v101688, %v101660
  %.pre20716 = inttoptr i64 %.pre20715 to ptr
  br label %block53925

edge8805:                                         ; preds = %block53924
  %t5645 = inttoptr i64 %v101671 to ptr
  %v101695 = load i64, ptr %t5645, align 1
  %v101696 = add i64 %v101695, 1
  store i64 %v101696, ptr %t5645, align 1
  br label %block53930

edge8814:                                         ; preds = %block53930
  %t5653.not = icmp eq i64 %v100593, 0
  br i1 %t5653.not, label %edge8820, label %edge8815

edge8815:                                         ; preds = %edge8814
  %t5654 = inttoptr i64 %v100593 to ptr
  %v101983 = load i64, ptr %t5654, align 1
  %v101984 = add i64 %v101983, 1
  store i64 %v101984, ptr %t5654, align 1
  br label %edge8820

edge8820:                                         ; preds = %edge8814, %edge8815
  %v102367 = add i64 %v100588, -8
  br label %block4489

edge8830:                                         ; preds = %block4489
  %v102039 = add nsw i64 %v102027, -1
  %v102364 = shl i64 %v102027, 3
  %v102368 = add i64 %v102367, %v102364
  %t5663 = inttoptr i64 %v102368 to ptr
  %v102369 = load i64, ptr %t5663, align 1
  %t5664.not = icmp eq i64 %v102369, 0
  br i1 %t5664.not, label %block4489, label %edge8853

edge8853:                                         ; preds = %edge8830
  br i1 %t5653.not, label %edge8887, label %edge8857

edge8857:                                         ; preds = %edge8853
  %v102406 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v100593)
  %t5668.not = icmp eq i64 %v102406, 0
  br i1 %t5668.not, label %edge8887, label %edge8858

edge8858:                                         ; preds = %edge8857
  %t5669 = insertvalue { i64, i64 } poison, i64 %v102406, 0
  %t5670 = insertvalue { i64, i64 } %t5669, i64 4580, 1
  br label %common.ret

edge8887:                                         ; preds = %edge8853, %edge8857, %block53930
  %v102752 = phi i64 [ %v101586, %block53930 ], [ %v102039, %edge8857 ], [ %v102039, %edge8853 ]
  %t5671 = icmp ult i64 %v102752, %v99108
  br i1 %t5671, label %edge8892, label %common.ret

edge8892:                                         ; preds = %edge8887
  %v103068 = add nuw nsw i64 %v102752, 1
  %v103074 = shl nuw nsw i64 %v102752, 3
  %v103075 = add i64 %v103074, %v100588
  %t5674 = inttoptr i64 %v103075 to ptr
  %v103076 = load i64, ptr %t5674, align 1
  %t5675.not = icmp eq i64 %v103076, 0
  br i1 %t5675.not, label %common.ret, label %edge8899

edge8899:                                         ; preds = %edge8892
  %v103104 = add i64 %v103076, -1
  %t5682.not = icmp eq i64 %v100593, 0
  br i1 %t5682.not, label %block54912, label %edge8906

edge8906:                                         ; preds = %edge8899
  %t5683 = inttoptr i64 %v100593 to ptr
  %v103135 = load i64, ptr %t5683, align 1
  %t5684 = icmp eq i64 %v103135, 1
  br i1 %t5684, label %block54911, label %block54912

edge8910:                                         ; preds = %block54912
  %t5711 = insertvalue { i64, i64 } poison, i64 %v103144, 0
  %t5712 = insertvalue { i64, i64 } %t5711, i64 4619, 1
  br label %common.ret

edge8911:                                         ; preds = %block54912
  %v103145 = load i64, ptr %ss1, align 8
  %v103146 = load i64, ptr %t606, align 8
  %v103147 = load i64, ptr %t607, align 8
  %.pre20711 = add i64 %v103145, %v103074
  %.pre20712 = inttoptr i64 %.pre20711 to ptr
  br label %block54911

edge8914:                                         ; preds = %block54911
  %t5708 = inttoptr i64 %v103134 to ptr
  %v103151 = load i64, ptr %t5708, align 1
  %v103152 = add i64 %v103151, 1
  store i64 %v103152, ptr %t5708, align 1
  br label %block54916

edge8943:                                         ; preds = %block54916
  %v103487 = shl nuw nsw i64 %v103068, 3
  %v103488 = add i64 %v103129, %v103487
  %t5716 = inttoptr i64 %v103488 to ptr
  %v103489 = load i64, ptr %t5716, align 1
  %v103517 = add i64 %v103489, 2
  %t5717.not = icmp ugt i64 %v103489, -3
  br i1 %t5717.not, label %common.ret, label %edge8953

edge8953:                                         ; preds = %edge8943
  %t5726.not = icmp eq i64 %v103153, 0
  br i1 %t5726.not, label %block55176, label %edge8956

edge8956:                                         ; preds = %edge8953
  %v103554 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v103153)
  %t5727.not = icmp eq i64 %v103554, 0
  br i1 %t5727.not, label %block55176, label %block55170

edge8960:                                         ; preds = %block55176
  %t5729 = inttoptr i64 %v103134 to ptr
  %v103561 = load i64, ptr %t5729, align 1
  %t5730 = icmp eq i64 %v103561, 1
  br i1 %t5730, label %block55173, label %block55179

edge8965:                                         ; preds = %block55179
  %v103571 = load i64, ptr %ss1, align 8
  %v103572 = load i64, ptr %t606, align 8
  %v103573 = load i64, ptr %t607, align 8
  %.pre20713 = add i64 %v103571, %v103487
  %.pre20714 = inttoptr i64 %.pre20713 to ptr
  br label %block55173

edge8968:                                         ; preds = %block55173
  %t5754 = inttoptr i64 %v103560 to ptr
  %v103577 = load i64, ptr %t5754, align 1
  %v103578 = add i64 %v103577, 1
  store i64 %v103578, ptr %t5754, align 1
  br label %block55183

edge8971:                                         ; preds = %block55183
  %t5758 = inttoptr i64 %v103134 to ptr
  %v103581 = load i64, ptr %t5758, align 1
  %v103582 = add i64 %v103581, -1
  store i64 %v103582, ptr %t5758, align 1
  %t5760.not = icmp eq i64 %v103582, 0
  br i1 %t5760.not, label %edge8974, label %edge9001

edge8974:                                         ; preds = %edge8971
  %v103583 = call i64 @mech_compact_free(i64 %v0, i64 %v103134)
  %t5761.not = icmp eq i64 %v103583, 0
  br i1 %t5761.not, label %edge9001, label %block55170

edge8985:                                         ; preds = %block4414
  %t5764.not = icmp eq i64 %v100581, 0
  br i1 %t5764.not, label %block55210, label %edge8986

edge8986:                                         ; preds = %edge8985
  %t5765 = inttoptr i64 %v100581 to ptr
  %v103629 = load i64, ptr %t5765, align 1
  %v103630 = add i64 %v103629, 1
  store i64 %v103630, ptr %t5765, align 1
  br label %block55210

edge8989:                                         ; preds = %block55210
  %t5768 = inttoptr i64 %v100593 to ptr
  %v103631 = load i64, ptr %t5768, align 1
  %v103632 = add i64 %v103631, 1
  store i64 %v103632, ptr %t5768, align 1
  br label %edge9002

edge9001:                                         ; preds = %edge8974, %edge8971, %block55183
  %t5779.not = icmp eq i64 %v101697, 0
  br i1 %t5779.not, label %block4414.backedge, label %edge9019

block4414.backedge:                               ; preds = %edge9001, %edge9019
  br label %block4414

edge9002:                                         ; preds = %edge8989, %block55210
  br i1 %t5764.not, label %block55238, label %edge9005

edge9005:                                         ; preds = %edge9002
  %v103682 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v100581)
  %t5773.not = icmp eq i64 %v103682, 0
  br i1 %t5773.not, label %block55238, label %block55233

edge9009:                                         ; preds = %block55238
  %t5775 = inttoptr i64 %v100593 to ptr
  %v103684 = load i64, ptr %t5775, align 1
  %v103685 = add i64 %v103684, -1
  store i64 %v103685, ptr %t5775, align 1
  %t5777.not = icmp eq i64 %v103685, 0
  br i1 %t5777.not, label %edge9012, label %edge9062

edge9012:                                         ; preds = %edge9009
  %v103686 = call i64 @mech_compact_free(i64 %v0, i64 %v100593)
  %t5778.not = icmp eq i64 %v103686, 0
  br i1 %t5778.not, label %edge9062, label %block55233

edge9019:                                         ; preds = %edge9001
  %v103690 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v101697)
  %t5780.not = icmp eq i64 %v103690, 0
  br i1 %t5780.not, label %block4414.backedge, label %block55233

edge9062:                                         ; preds = %edge9009, %edge9012
  %t5789 = inttoptr i64 %v100593 to ptr
  %v104104 = load i64, ptr %t5789, align 1
  %v104105 = add i64 %v104104, 1
  store i64 %v104105, ptr %t5789, align 1
  br label %edge9073

edge9073:                                         ; preds = %block55238, %edge9062
  %t5862 = inttoptr i64 %v100581 to ptr
  br label %block4608

edge9074:                                         ; preds = %block4608
  %t5792 = inttoptr i64 %v104178 to ptr
  %v104179 = load i64, ptr %t5792, align 1
  %t5793 = icmp ne i64 %v104179, 1
  %t5795.not = icmp ugt i64 %v217502, %v104219
  %or.cond310 = select i1 %t5793, i1 true, i1 %t5795.not
  br i1 %or.cond310, label %edge9115, label %edge9078

edge9078:                                         ; preds = %edge9074
  %t5797 = zext i64 %v104220 to i128
  %t5798 = zext i64 %v104221 to i128
  %t5799 = shl nuw i128 %t5798, 64
  %v218634 = or disjoint i128 %t5799, %t5797
  store i128 %v218634, ptr %ss2, align 16
  %t5801 = zext i64 %v100576 to i128
  %v218636 = or disjoint i128 %t4978, %t5801
  store i128 %v218636, ptr %t926, align 16
  %v218638 = or disjoint i128 %t4978, 1
  store i128 %v218638, ptr %t931, align 16
  %t5811 = zext i64 %v104226 to i128
  %v218640 = or disjoint i128 %t4978, %t5811
  store i128 %v218640, ptr %t936, align 16
  store i128 %v218638, ptr %t941, align 16
  %t5821 = zext i64 %v100588 to i128
  %v218644 = or disjoint i128 %t5689, %t5821
  store i128 %v218644, ptr %t946, align 16
  %v218646 = or disjoint i128 %t5689, 1
  store i128 %v218646, ptr %t947, align 16
  %t5831 = zext i64 %v104178 to i128
  %t5832 = zext i64 %v104219 to i128
  %t5833 = shl nuw i128 %t5832, 64
  %v218648 = or disjoint i128 %t5833, %t5831
  store i128 %v218648, ptr %t1152, align 16
  %t5842 = call fastcc { i64, i64 } @mech_compact_numeric_4673_vector(i64 %v218103, i64 %v8479)
  %v104195 = extractvalue { i64, i64 } %t5842, 0
  %t5843.not = icmp eq i64 %v104195, 0
  br i1 %t5843.not, label %edge9085, label %edge9084

edge9084:                                         ; preds = %edge9078
  %v104196 = extractvalue { i64, i64 } %t5842, 1
  %t5844 = icmp eq i64 %v104195, 103169
  %v104201 = select i1 %t5844, i64 103425, i64 %v104195
  %t5846 = insertvalue { i64, i64 } poison, i64 %v104201, 0
  %t5847 = insertvalue { i64, i64 } %t5846, i64 %v104196, 1
  br label %common.ret

edge9085:                                         ; preds = %edge9078
  br i1 %t5764.not, label %edge9092, label %edge9088

edge9088:                                         ; preds = %edge9085
  %v104206 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v100581)
  %t5849.not = icmp eq i64 %v104206, 0
  br i1 %t5849.not, label %edge9092, label %block55569

edge9092:                                         ; preds = %edge9088, %edge9085
  br i1 %t5774.not, label %edge9329, label %edge9095

edge9095:                                         ; preds = %edge9092
  %v104214 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v100593)
  %t5851.not = icmp eq i64 %v104214, 0
  br i1 %t5851.not, label %edge9329, label %block55569

edge9115:                                         ; preds = %edge9074, %block4608
  %t5854.not = icmp eq i64 %v104220, 0
  br i1 %t5854.not, label %edge9118, label %edge9128

edge9118:                                         ; preds = %edge9115
  br i1 %t5764.not, label %block56681, label %edge9262

edge9128:                                         ; preds = %edge9115
  %t5857 = icmp ult i64 %v104220, %v99108
  br i1 %t5857, label %edge9136, label %edge9144

edge9136:                                         ; preds = %edge9128
  %v104892 = shl nuw nsw i64 %v104220, 3
  %v104893 = add i64 %v104892, %v100588
  %t5860 = inttoptr i64 %v104893 to ptr
  %v104894 = load i64, ptr %t5860, align 1
  br label %edge9144

edge9144:                                         ; preds = %edge9128, %edge9136
  %v104915 = phi i64 [ %v104894, %edge9136 ], [ 0, %edge9128 ]
  br i1 %t5764.not, label %edge9152, label %edge9145

edge9145:                                         ; preds = %edge9144
  %v104918 = load i64, ptr %t5862, align 1
  %v104919 = add i64 %v104918, 1
  store i64 %v104919, ptr %t5862, align 1
  br label %edge9152

edge9152:                                         ; preds = %edge9144, %edge9145
  %v105683870 = shl i64 %v104220, 16
  %v105690 = and i64 %v105683870, 4294901760
  %v105741 = trunc i64 %v105683870 to i32
  br label %block4644

edge9169:                                         ; preds = %block4644
  br i1 %t5764.not, label %edge9277, label %edge9170

edge9170:                                         ; preds = %edge9169
  %v105011 = load i64, ptr %t5862, align 1
  %v105012 = add i64 %v105011, -1
  store i64 %v105012, ptr %t5862, align 1
  %t5870.not = icmp eq i64 %v105012, 0
  br i1 %t5870.not, label %edge9173, label %edge9277

edge9173:                                         ; preds = %edge9170
  %v105013 = call i64 @mech_compact_free(i64 %v0, i64 %v100581)
  %t5871.not = icmp eq i64 %v105013, 0
  br i1 %t5871.not, label %edge9277, label %edge9174

edge9174:                                         ; preds = %edge9173
  %t5872 = insertvalue { i64, i64 } poison, i64 %v105013, 0
  %t5873 = insertvalue { i64, i64 } %t5872, i64 4723, 1
  br label %common.ret

edge9188:                                         ; preds = %block4644
  %t5874 = icmp ult i64 %v104989, %v217502
  br i1 %t5874, label %edge9205, label %edge9212

edge9205:                                         ; preds = %edge9188
  %v105613 = shl nuw nsw i64 %v104989, 2
  %v105614 = add i64 %v105613, %v100576
  %t5877 = inttoptr i64 %v105614 to ptr
  %t5878 = load i32, ptr %t5877, align 1
  %55 = and i32 %t5878, 1023
  %56 = zext nneg i32 %55 to i64
  %t5880 = icmp samesign ugt i64 %v217502, %56
  br i1 %t5880, label %edge9212, label %block56596

edge9212:                                         ; preds = %edge9188, %edge9205
  %v105637869 = phi i64 [ %56, %edge9205 ], [ 0, %edge9188 ]
  %v105709 = shl nuw nsw i64 %v105637869, 2
  %v105710 = add i64 %v105709, %v104974
  %t5882 = inttoptr i64 %v105710 to ptr
  %t5883 = load i32, ptr %t5882, align 1
  %v105711 = zext i32 %t5883 to i64
  %t5884 = icmp eq i64 %v105690, %v105711
  br i1 %t5884, label %block56599, label %edge9215

edge9215:                                         ; preds = %edge9212
  %t5886.not = icmp eq i64 %v104979, 0
  br i1 %t5886.not, label %block56601, label %edge9216

edge9216:                                         ; preds = %edge9215
  %t5887 = inttoptr i64 %v104979 to ptr
  %v105727 = load i64, ptr %t5887, align 1
  %t5888 = icmp eq i64 %v105727, 1
  br i1 %t5888, label %block56600, label %block56601

edge9221:                                         ; preds = %block56601
  %v105737 = load i64, ptr %ss1, align 8
  %v105738 = load i64, ptr %t606, align 8
  %v105739 = load i64, ptr %t607, align 8
  %.pre20709 = add i64 %v105737, %v105709
  %.pre20710 = inttoptr i64 %.pre20709 to ptr
  br label %block56600

edge9224:                                         ; preds = %block56599
  %t5912 = inttoptr i64 %v105720 to ptr
  %v105744 = load i64, ptr %t5912, align 1
  %v105745 = add i64 %v105744, 1
  store i64 %v105745, ptr %t5912, align 1
  br label %block56605

edge9229:                                         ; preds = %block56605
  %v105750 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v104979)
  %t5916.not = icmp eq i64 %v105750, 0
  br i1 %t5916.not, label %edge9233, label %block56596

edge9233:                                         ; preds = %edge9229, %block56605
  %t5919.not = icmp eq i64 %v104989, -1
  br i1 %t5919.not, label %common.ret, label %edge9237

edge9237:                                         ; preds = %edge9233
  %v105755 = add nuw i64 %v104989, 1
  %v105764 = add i64 %v104992, -1
  br label %block4644

edge9262:                                         ; preds = %edge9118
  %v105871 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v100581)
  %t5924.not = icmp eq i64 %v105871, 0
  br i1 %t5924.not, label %block56681, label %block56677

edge9266:                                         ; preds = %block56681
  %t5926 = inttoptr i64 %v100593 to ptr
  %v105873 = load i64, ptr %t5926, align 1
  %v105874 = add i64 %v105873, -1
  store i64 %v105874, ptr %t5926, align 1
  %t5928.not = icmp eq i64 %v105874, 0
  br i1 %t5928.not, label %edge9269, label %edge9329

edge9269:                                         ; preds = %edge9266
  %v105875 = call i64 @mech_compact_free(i64 %v0, i64 %v100593)
  %t5929.not = icmp eq i64 %v105875, 0
  br i1 %t5929.not, label %edge9329, label %block56677

edge9277:                                         ; preds = %edge9173, %edge9170, %edge9169
  %v106151 = add nsw i64 %v104220, -1
  br label %block4608

edge9308:                                         ; preds = %edge9332
  %v107058 = call i64 @mech_compact_free(i64 %v0, i64 %v107246)
  %t5947.not = icmp eq i64 %v107058, 0
  br i1 %t5947.not, label %edge9345, label %edge9309

edge9309:                                         ; preds = %edge9308
  %t5948 = insertvalue { i64, i64 } poison, i64 %v107058, 0
  %t5949 = insertvalue { i64, i64 } %t5948, i64 4784, 1
  br label %common.ret

edge9329:                                         ; preds = %block56681, %edge9266, %edge9269, %edge9092, %edge9095
  %v107241 = call i64 @mech_compact_allocate(i64 %v0, i64 17, i64 4, i64 4, i64 1, i64 %v3192)
  %t5962.not = icmp eq i64 %v107241, 0
  br i1 %t5962.not, label %edge9331, label %block56955

edge9331:                                         ; preds = %edge9329
  %v107244 = load i64, ptr %ss1, align 8
  %v107245 = load i64, ptr %t606, align 8
  %v107246 = load i64, ptr %t607, align 8
  %t5965.not = icmp eq i64 %v107246, 0
  br i1 %t5965.not, label %edge9345, label %edge9332

edge9332:                                         ; preds = %edge9331
  %t5966 = inttoptr i64 %v107246 to ptr
  %v107247 = load i64, ptr %t5966, align 1
  %v107248 = add i64 %v107247, 1
  store i64 %v107248, ptr %t5966, align 1
  %t5946.not = icmp eq i64 %v107248, 0
  br i1 %t5946.not, label %edge9308, label %edge9345

edge9345:                                         ; preds = %edge9331, %edge9308, %edge9332
  %v108469 = add i64 %v100588, -8
  %57 = add nuw nsw i64 %v217504, 1
  br label %block4730

edge9346:                                         ; preds = %block4730
  %t5971 = inttoptr i64 %v107439 to ptr
  %v107440 = load i64, ptr %t5971, align 1
  %t5972 = icmp eq i64 %v107440, 1
  %t5974 = icmp ugt i64 %v107463, 16
  %or.cond311 = select i1 %t5972, i1 %t5974, i1 false
  br i1 %or.cond311, label %edge9350, label %edge9375

edge9350:                                         ; preds = %edge9346
  %t5976 = zext nneg i64 %v217504 to i128
  %t5977 = zext nneg i64 %v107465 to i128
  %t5978 = shl nuw nsw i128 %t5977, 64
  %v218656 = or disjoint i128 %t5978, %t5976
  store i128 %v218656, ptr %ss2, align 16
  %t5980 = zext i64 %v100588 to i128
  %v218658 = or disjoint i128 %t5689, %t5980
  store i128 %v218658, ptr %t926, align 16
  %v218660 = or disjoint i128 %t5689, 1
  store i128 %v218660, ptr %t931, align 16
  %t5990 = zext i64 %v107470 to i128
  %v218662 = or disjoint i128 %t5990, 313594649253062377472
  store i128 %v218662, ptr %t936, align 16
  store i128 313594649253062377473, ptr %t941, align 16
  %t6000 = zext i64 %v107439 to i128
  %t6001 = zext i64 %v107463 to i128
  %t6002 = shl nuw i128 %t6001, 64
  %v218666 = or disjoint i128 %t6002, %t6000
  store i128 %v218666, ptr %t946, align 16
  %t6005 = call fastcc { i64, i64 } @mech_compact_numeric_4799(i64 %v218103, i64 %v8479)
  %v107445 = extractvalue { i64, i64 } %t6005, 0
  %t6006.not = icmp eq i64 %v107445, 0
  br i1 %t6006.not, label %edge9353, label %edge9352

edge9352:                                         ; preds = %edge9350
  %v107446 = extractvalue { i64, i64 } %t6005, 1
  %t6007 = icmp eq i64 %v107445, 103169
  %v107449 = select i1 %t6007, i64 103425, i64 %v107445
  %t6009 = insertvalue { i64, i64 } poison, i64 %v107449, 0
  %t6010 = insertvalue { i64, i64 } %t6009, i64 %v107446, 1
  br label %common.ret

edge9353:                                         ; preds = %edge9350
  br i1 %t5774.not, label %block4807.preheader, label %edge9354

edge9354:                                         ; preds = %edge9353
  %t6012 = inttoptr i64 %v100593 to ptr
  %v107452 = load i64, ptr %t6012, align 1
  %v107453 = add i64 %v107452, -1
  store i64 %v107453, ptr %t6012, align 1
  %t6014.not = icmp eq i64 %v107453, 0
  br i1 %t6014.not, label %edge9357, label %block4807.preheader

edge9357:                                         ; preds = %edge9354
  %v107454 = call i64 @mech_compact_free(i64 %v0, i64 %v100593)
  %t6015.not = icmp eq i64 %v107454, 0
  br i1 %t6015.not, label %block4807.preheader, label %edge9358

edge9358:                                         ; preds = %edge9357
  %t6016 = insertvalue { i64, i64 } poison, i64 %v107454, 0
  %t6017 = insertvalue { i64, i64 } %t6016, i64 4799, 1
  br label %common.ret

edge9375:                                         ; preds = %edge9346, %block4730
  %exitcond20592 = icmp eq i64 %v107465, %57
  br i1 %exitcond20592, label %edge9378, label %edge9442

edge9378:                                         ; preds = %edge9375
  br i1 %t5774.not, label %block4807.preheader, label %edge9379

block4807.preheader:                              ; preds = %edge9353, %edge9354, %edge9357, %edge9382, %edge9379, %edge9378
  br label %block4807

edge9379:                                         ; preds = %edge9378
  %t6022 = inttoptr i64 %v100593 to ptr
  %v107530 = load i64, ptr %t6022, align 1
  %v107531 = add i64 %v107530, -1
  store i64 %v107531, ptr %t6022, align 1
  %t6024.not = icmp eq i64 %v107531, 0
  br i1 %t6024.not, label %edge9382, label %block4807.preheader

edge9382:                                         ; preds = %edge9379
  %v107532 = call i64 @mech_compact_free(i64 %v0, i64 %v100593)
  %t6025.not = icmp eq i64 %v107532, 0
  br i1 %t6025.not, label %block4807.preheader, label %edge9383

edge9383:                                         ; preds = %edge9382
  %t6026 = insertvalue { i64, i64 } poison, i64 %v107532, 0
  %t6027 = insertvalue { i64, i64 } %t6026, i64 4810, 1
  br label %common.ret

edge9442:                                         ; preds = %edge9375
  %v108119 = shl i64 %v107465, 2
  %v108122 = add nsw i64 %v108119, -4
  %v108123 = add i64 %v108122, %v107470
  %t6035 = inttoptr i64 %v108123 to ptr
  %t6036 = load i32, ptr %t6035, align 1
  %v108466 = shl i64 %v107465, 3
  %v108470 = add i64 %v108469, %v108466
  %t6040 = inttoptr i64 %v108470 to ptr
  %v108471 = load i64, ptr %t6040, align 1
  %58 = trunc i64 %v108471 to i32
  %v108520.narrow = add i32 %t6036, %58
  %v108525 = zext i32 %v108520.narrow to i64
  %v108541 = shl nuw nsw i64 %v108525, 1
  %v108557 = add nuw nsw i64 %v107465, 1
  %v108546 = and i64 %v108541, 4294967294
  %v108566 = add i64 %v107470, %v108119
  %t6044 = inttoptr i64 %v108566 to ptr
  %t6045 = load i32, ptr %t6044, align 1
  %v108567 = zext i32 %t6045 to i64
  %t6046 = icmp eq i64 %v108546, %v108567
  br i1 %t6046, label %block57762, label %edge9461

edge9461:                                         ; preds = %edge9442
  br i1 %t5970.not, label %block57764, label %edge9462

edge9462:                                         ; preds = %edge9461
  %t6049 = inttoptr i64 %v107439 to ptr
  %v108583 = load i64, ptr %t6049, align 1
  %t6050 = icmp eq i64 %v108583, 1
  br i1 %t6050, label %block57763, label %block57764

edge9467:                                         ; preds = %block57764
  %v108593 = load i64, ptr %ss1, align 8
  %v108594 = load i64, ptr %t606, align 8
  %v108595 = load i64, ptr %t607, align 8
  %.pre20707 = add i64 %v108593, %v108119
  %.pre20708 = inttoptr i64 %.pre20707 to ptr
  br label %block57763

edge9470:                                         ; preds = %block57762
  %t6074 = inttoptr i64 %v108576 to ptr
  %v108600 = load i64, ptr %t6074, align 1
  %v108601 = add i64 %v108600, 1
  store i64 %v108601, ptr %t6074, align 1
  br label %block57768

edge9475:                                         ; preds = %block57768
  %v108606 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v107439)
  %t6078.not = icmp eq i64 %v108606, 0
  br i1 %t6078.not, label %block4730.backedge, label %block57759

block4730.backedge:                               ; preds = %edge9475, %block57768
  br label %block4730

edge9504:                                         ; preds = %block4807
  %t6086 = inttoptr i64 %v108856 to ptr
  %v108857 = load i64, ptr %t6086, align 1
  %t6087 = icmp eq i64 %v108857, 1
  %t6089 = icmp ne i64 %v108886, 0
  %or.cond31 = select i1 %t6087, i1 %t6089, i1 false
  br i1 %or.cond31, label %edge9508, label %edge9541

edge9508:                                         ; preds = %edge9504
  %t6090 = inttoptr i64 %v108886 to ptr
  %v108859 = load i64, ptr %t6090, align 1
  %t6091 = icmp eq i64 %v108859, 1
  %t6093.not = icmp ule i64 %v217502, %v108888
  %or.cond312.not1223 = select i1 %t6091, i1 %t6093.not, i1 false
  %t6095 = icmp ugt i64 %v108890, 16
  %or.cond313 = select i1 %or.cond312.not1223, i1 %t6095, i1 false
  br i1 %or.cond313, label %edge9514, label %edge9541

edge9514:                                         ; preds = %edge9508
  %t6098 = zext nneg i64 %v108892 to i128
  %t6099 = shl nuw nsw i128 %t6098, 64
  %v218674 = or disjoint i128 %t6099, %t4977
  store i128 %v218674, ptr %ss2, align 16
  %t6101 = zext i64 %v108893 to i128
  %v218676 = or disjoint i128 %t4978, %t6101
  store i128 %v218676, ptr %t926, align 16
  %v218678 = or disjoint i128 %t4978, 1
  store i128 %v218678, ptr %t931, align 16
  %t6111 = zext i64 %v108896 to i128
  %v218680 = or disjoint i128 %t6111, 313594649253062377472
  store i128 %v218680, ptr %t936, align 16
  store i128 313594649253062377473, ptr %t941, align 16
  store i128 %v218684, ptr %t946, align 16
  store i128 4722366482869645213697, ptr %t947, align 16
  %t6131 = zext i64 %v108856 to i128
  %t6132 = zext i64 %v108886 to i128
  %t6133 = shl nuw i128 %t6132, 64
  %v218688 = or disjoint i128 %t6133, %t6131
  store i128 %v218688, ptr %t1152, align 16
  %t6136 = zext i64 %v108888 to i128
  %t6137 = zext i64 %v108890 to i128
  %t6138 = shl nuw i128 %t6137, 64
  %v218690 = or disjoint i128 %t6138, %t6136
  store i128 %v218690, ptr %t1157, align 16
  %t6141 = call fastcc { i64, i64 } @mech_compact_numeric_4879(i64 %v218103, i64 %v8479)
  %v108865 = extractvalue { i64, i64 } %t6141, 0
  %t6142.not = icmp eq i64 %v108865, 0
  br i1 %t6142.not, label %edge9520, label %edge9516

edge9516:                                         ; preds = %edge9514
  %v108866 = extractvalue { i64, i64 } %t6141, 1
  %t6143 = icmp eq i64 %v108865, 103169
  %v108869 = select i1 %t6143, i64 103425, i64 %v108865
  %t6145 = insertvalue { i64, i64 } poison, i64 %v108869, 0
  %t6146 = insertvalue { i64, i64 } %t6145, i64 %v108866, 1
  br label %common.ret

edge9520:                                         ; preds = %edge9514
  %v108878 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v108886)
  %t6147.not = icmp eq i64 %v108878, 0
  br i1 %t6147.not, label %edge8095, label %edge9521

edge9521:                                         ; preds = %edge9520
  %t6148 = insertvalue { i64, i64 } poison, i64 %v108878, 0
  %t6149 = insertvalue { i64, i64 } %t6148, i64 4879, 1
  br label %common.ret

edge9541:                                         ; preds = %edge9508, %edge9504, %block4807
  %exitcond20593.not = icmp eq i64 %v108892, %v217502
  br i1 %exitcond20593.not, label %edge9743, label %edge9572

edge9572:                                         ; preds = %edge9541
  %v109550 = shl nuw nsw i64 %v108892, 2
  %v109551 = add i64 %v108893, %v109550
  %t6156 = inttoptr i64 %v109551 to ptr
  %t6157 = load i32, ptr %t6156, align 1
  %59 = lshr i32 %t6157, 16
  %v109587 = zext nneg i32 %59 to i64
  %t6159 = icmp eq i32 %59, 0
  br i1 %t6159, label %edge9754, label %edge9597

edge9597:                                         ; preds = %edge9572
  %t6162 = icmp ult i32 %t6157, 1114112
  br i1 %t6162, label %edge9613, label %block58814

edge9613:                                         ; preds = %edge9597
  %v110226 = shl nuw nsw i64 %v109587, 2
  %v110227 = add i64 %v110226, %v108896
  %t6165 = inttoptr i64 %v110227 to ptr
  %t6166 = load i32, ptr %t6165, align 1
  %v110228 = zext i32 %t6166 to i64
  %v110294 = add i32 %t6166, 1
  %t6167.not = icmp eq i32 %t6166, -1
  br i1 %t6167.not, label %common.ret, label %edge9621

edge9621:                                         ; preds = %edge9613
  %t6177.not = icmp eq i64 %v108886, 0
  br i1 %t6177.not, label %block58819, label %edge9622

edge9622:                                         ; preds = %edge9621
  %t6178 = inttoptr i64 %v108886 to ptr
  %v110327 = load i64, ptr %t6178, align 1
  %t6179 = icmp eq i64 %v110327, 1
  br i1 %t6179, label %block58817, label %block58819

edge9627:                                         ; preds = %block58819
  %v110337 = load i64, ptr %ss1, align 8
  %v110338 = load i64, ptr %t606, align 8
  %v110339 = load i64, ptr %t607, align 8
  %.pre20705 = add i64 %v110337, %v110226
  %.pre20706 = inttoptr i64 %.pre20705 to ptr
  br label %block58817

edge9630:                                         ; preds = %block58817
  %t6203 = inttoptr i64 %v110326 to ptr
  %v110344 = load i64, ptr %t6203, align 1
  %v110345 = add i64 %v110344, 1
  store i64 %v110345, ptr %t6203, align 1
  br label %block58823

edge9635:                                         ; preds = %block58823
  %v110350 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v108886)
  %t6207.not = icmp eq i64 %v110350, 0
  br i1 %t6207.not, label %edge9640, label %block58814

edge9640:                                         ; preds = %block58823, %edge9635
  %v110374 = and i64 %v110228, 255
  %v110692 = add i64 %v110374, ptrtoint (ptr @mech_compact_table_19 to i64)
  %t6211 = inttoptr i64 %v110692 to ptr
  %t6212 = load i8, ptr %t6211, align 1
  %v110693 = zext i8 %t6212 to i64
  %t6214 = icmp ult i32 %t6166, 65536
  br i1 %t6214, label %edge9677, label %edge9689

edge9677:                                         ; preds = %edge9640
  %v110768 = lshr i64 %v110228, 8
  %v111086 = add i64 %v110768, ptrtoint (ptr @mech_compact_table_19 to i64)
  %t6217 = inttoptr i64 %v111086 to ptr
  %t6218 = load i8, ptr %t6217, align 1
  %v111087 = zext i8 %t6218 to i64
  br label %edge9689

edge9689:                                         ; preds = %edge9640, %edge9677
  %v111107 = phi i64 [ %v111087, %edge9677 ], [ 0, %edge9640 ]
  %v111117 = shl nuw nsw i64 %v110693, 8
  %v111122 = or disjoint i64 %v111107, %v111117
  %v111129 = sub nuw nsw i64 16, %v109587
  %v111135 = lshr i64 %v111122, %v111129
  %v111157 = shl nuw nsw i64 %v109587, 16
  %v111162 = or disjoint i64 %v111135, %v111157
  %t6229 = load i32, ptr %t6156, align 1
  %v111202 = zext i32 %t6229 to i64
  %t6230 = icmp eq i64 %v111162, %v111202
  br i1 %t6230, label %block59338, label %edge9711

edge9711:                                         ; preds = %edge9689
  br i1 %t6085.not, label %block59340, label %edge9712

edge9712:                                         ; preds = %edge9711
  %t6233 = inttoptr i64 %v108856 to ptr
  %v111218 = load i64, ptr %t6233, align 1
  %t6234 = icmp eq i64 %v111218, 1
  br i1 %t6234, label %block59339, label %block59340

edge9717:                                         ; preds = %block59340
  %v111228 = load i64, ptr %ss1, align 8
  %v111229 = load i64, ptr %t606, align 8
  %v111230 = load i64, ptr %t607, align 8
  %.pre20703 = add i64 %v111228, %v109550
  %.pre20704 = inttoptr i64 %.pre20703 to ptr
  br label %block59339

edge9720:                                         ; preds = %block59338
  %t6258 = inttoptr i64 %v111211 to ptr
  %v111235 = load i64, ptr %t6258, align 1
  %v111236 = add i64 %v111235, 1
  store i64 %v111236, ptr %t6258, align 1
  br label %block59344

edge9723:                                         ; preds = %block59344
  %t6262 = inttoptr i64 %v108856 to ptr
  %v111239 = load i64, ptr %t6262, align 1
  %v111240 = add i64 %v111239, -1
  store i64 %v111240, ptr %t6262, align 1
  %t6264.not = icmp eq i64 %v111240, 0
  br i1 %t6264.not, label %edge9726, label %edge9754

edge9726:                                         ; preds = %edge9723
  %v111241 = call i64 @mech_compact_free(i64 %v0, i64 %v108856)
  %t6265.not = icmp eq i64 %v111241, 0
  br i1 %t6265.not, label %edge9754, label %block59335

edge9743:                                         ; preds = %edge9541
  %t6270.not = icmp eq i64 %v108886, 0
  br i1 %t6270.not, label %edge8095, label %edge9744

edge9744:                                         ; preds = %edge9743
  %t6271 = inttoptr i64 %v108886 to ptr
  %v111296 = load i64, ptr %t6271, align 1
  %v111297 = add i64 %v111296, -1
  store i64 %v111297, ptr %t6271, align 1
  %t6273.not = icmp eq i64 %v111297, 0
  br i1 %t6273.not, label %edge9747, label %edge8095

edge9747:                                         ; preds = %edge9744
  %v111298 = call i64 @mech_compact_free(i64 %v0, i64 %v108886)
  %t6274.not = icmp eq i64 %v111298, 0
  br i1 %t6274.not, label %edge8095, label %edge9748

edge9748:                                         ; preds = %edge9747
  %t6275 = insertvalue { i64, i64 } poison, i64 %v111298, 0
  %t6276 = insertvalue { i64, i64 } %t6275, i64 5018, 1
  br label %common.ret

edge9754:                                         ; preds = %block59344, %edge9723, %edge9726, %edge9572
  %v217532.ph = phi i64 [ %v111206, %block59344 ], [ %v111206, %edge9723 ], [ %v111206, %edge9726 ], [ %v108893, %edge9572 ]
  %v217533.ph = phi i64 [ %v111208, %block59344 ], [ %v111208, %edge9723 ], [ %v111208, %edge9726 ], [ %v108888, %edge9572 ]
  %v217534.ph = phi i64 [ %v111211, %block59344 ], [ %v111211, %edge9723 ], [ %v111211, %edge9726 ], [ %v108856, %edge9572 ]
  %v217535.ph = phi i64 [ %v110321, %block59344 ], [ %v110321, %edge9723 ], [ %v110321, %edge9726 ], [ %v108896, %edge9572 ]
  %v217536.ph = phi i64 [ %v110323, %block59344 ], [ %v110323, %edge9723 ], [ %v110323, %edge9726 ], [ %v108890, %edge9572 ]
  %v217537.ph = phi i64 [ %v110326, %block59344 ], [ %v110326, %edge9723 ], [ %v110326, %edge9726 ], [ %v108886, %edge9572 ]
  %v111560 = add nuw nsw i64 %v108892, 1
  br label %block4807

edge9779:                                         ; preds = %edge8095
  %t6281.not = icmp eq i64 %v111931, 0
  br i1 %t6281.not, label %edge9834, label %edge9834.thread

edge9784:                                         ; preds = %edge8095
  br i1 %v217500, label %edge9789, label %edge9825

edge9789:                                         ; preds = %edge9784
  %t6288 = call i64 @llvm.umin.i64(i64 %v217502, i64 32)
  %t6289 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v217485, i64 %t6288)
  %v112376 = extractvalue { i64, i1 } %t6289, 0
  %t6290 = extractvalue { i64, i1 } %t6289, 1
  br i1 %t6290, label %block60045, label %edge9792

edge9792:                                         ; preds = %edge9789
  %t6293.not = icmp ugt i64 %v112376, %v217486
  br i1 %t6293.not, label %edge9795, label %edge9812

edge9795:                                         ; preds = %edge9792
  %t6295.not = icmp eq i64 %v217486, 0
  br i1 %t6295.not, label %block60053, label %edge9796

edge9796:                                         ; preds = %edge9795
  %60 = add i64 %v217486, -32769
  %v112386196 = icmp ult i64 %60, -31745
  br i1 %v112386196, label %edge9799, label %edge9808

edge9799:                                         ; preds = %edge9796
  %t6299 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v217486, i64 %v217486)
  %v112387 = extractvalue { i64, i1 } %t6299, 0
  %t6300 = extractvalue { i64, i1 } %t6299, 1
  br i1 %t6300, label %block60045, label %block60053

edge9808:                                         ; preds = %edge9796
  %t6307 = mul nuw nsw i64 %v217486, 3
  %t6311 = add nuw nsw i64 %t6307, 1
  %v112404 = lshr i64 %t6311, 1
  br label %block60053

edge9812:                                         ; preds = %edge9792, %block60053
  %v112381 = phi i64 [ %t6315, %block60053 ], [ %v217486, %edge9792 ]
  %t6316 = zext i64 %v217484 to i128
  %t6317 = zext i64 %v217485 to i128
  %t6318 = shl nuw i128 %t6317, 64
  %v218704 = or disjoint i128 %t6318, %t6316
  store i128 %v218704, ptr %ss0, align 16
  %t6320 = zext i64 %v217486 to i128
  %v218706 = or disjoint i128 %t6320, 18446744073709551616
  store i128 %v218706, ptr %t1002, align 16
  %t6325 = zext i64 %v217488 to i128
  %t6326 = zext i64 %v217489 to i128
  %t6327 = shl nuw i128 %t6326, 64
  %v218708 = or disjoint i128 %t6327, %t6325
  store i128 %v218708, ptr %t1007, align 16
  %v112417 = call i64 @mech_compact_writable(i64 %v0, i64 %v218115, i64 0, i64 %v112381, i64 4, i64 4, i64 %v3192)
  %t6330.not = icmp eq i64 %v112417, 0
  br i1 %t6330.not, label %edge9814, label %block60045

edge9814:                                         ; preds = %edge9812
  %v112418 = load i64, ptr %ss1, align 8
  %v112419 = load i64, ptr %t606, align 8
  %v112420 = load i64, ptr %t607, align 8
  %v112425 = shl nuw nsw i64 %t6288, 2
  %v112421 = shl i64 %v217485, 2
  %v112422 = add i64 %v112418, %v112421
  %v112426 = call i64 @memmove(i64 %v112422, i64 %v111926, i64 %v112425)
  %t6334.not = icmp eq i64 %v112420, 0
  br i1 %t6334.not, label %edge9845, label %edge9839.thread1202

edge9839.thread1202:                              ; preds = %edge9814
  %t6335 = inttoptr i64 %v112420 to ptr
  %v112427 = load i64, ptr %t6335, align 1
  %v112428 = add i64 %v112427, 1
  store i64 %v112428, ptr %t6335, align 1
  br label %edge9840

edge9825:                                         ; preds = %edge9784
  %t6339.not = icmp eq i64 %v217489, 0
  br i1 %t6339.not, label %edge10494, label %edge9826

edge9826:                                         ; preds = %edge9825
  %t6340 = inttoptr i64 %v217489 to ptr
  %v112470 = load i64, ptr %t6340, align 1
  %v112471 = add i64 %v112470, 1
  store i64 %v112471, ptr %t6340, align 1
  br label %edge10494

edge9834:                                         ; preds = %edge9779
  br i1 %v217500, label %edge9845, label %edge10494

edge9834.thread:                                  ; preds = %edge9779
  %t6282 = inttoptr i64 %v111931 to ptr
  %v111932 = load i64, ptr %t6282, align 1
  %v111933 = add i64 %v111932, 1
  store i64 %v111933, ptr %t6282, align 1
  br i1 %v217500, label %edge9839.edge9840_crit_edge, label %edge10494

edge9837:                                         ; preds = %edge10498, %block65595, %edge10440, %edge10437, %block65272
  %v113120894 = phi i64 [ %v1131208931196, %edge10440 ], [ %v1131208931196, %block65272 ], [ %v1131208931196, %edge10437 ], [ %v113120907, %block65595 ], [ %v113120907, %edge10498 ]
  %v113119891 = phi i64 [ %v1131168871199, %edge10440 ], [ %v1131168871199, %block65272 ], [ %v1131168871199, %edge10437 ], [ %v113119906, %block65595 ], [ %v113119906, %edge10498 ]
  %v113117889 = phi i64 [ %v1131178901198, %edge10440 ], [ %v1131178901198, %block65272 ], [ %v1131178901198, %edge10437 ], [ %v113117905, %block65595 ], [ %v113117905, %edge10498 ]
  %v113116888 = phi i64 [ %v1131168871199, %edge10440 ], [ %v1131168871199, %block65272 ], [ %v1131168871199, %edge10437 ], [ %v113116904, %block65595 ], [ %v113116904, %edge10498 ]
  %v113115886 = phi i64 [ %v1131158851200, %edge10440 ], [ %v1131158851200, %block65272 ], [ %v1131158851200, %edge10437 ], [ %v113115903, %block65595 ], [ %v113115903, %edge10498 ]
  %v124233 = phi i64 [ %v117617, %edge10440 ], [ %v117617, %block65272 ], [ %v117617, %edge10437 ], [ %v217490, %block65595 ], [ %v217490, %edge10498 ]
  %v124234 = phi i64 [ 19, %edge10440 ], [ 19, %block65272 ], [ 19, %edge10437 ], [ %v217491, %block65595 ], [ %v217491, %edge10498 ]
  %v124237 = phi i64 [ 19, %edge10440 ], [ 19, %block65272 ], [ 19, %edge10437 ], [ %v217493, %block65595 ], [ %v217493, %edge10498 ]
  %v124238 = phi i64 [ %v117622, %edge10440 ], [ %v117622, %block65272 ], [ %v117622, %edge10437 ], [ %v217494, %block65595 ], [ %v217494, %edge10498 ]
  %v124335 = phi i64 [ %v117634, %edge10440 ], [ 0, %block65272 ], [ %v117634, %edge10437 ], [ 0, %block65595 ], [ %v217496, %edge10498 ]
  %v124648 = phi i64 [ %v117629, %edge10440 ], [ %v117629, %block65272 ], [ %v117629, %edge10437 ], [ %v217495, %block65595 ], [ %v217495, %edge10498 ]
  %v124659 = phi i64 [ %v117641, %edge10440 ], [ %v117641, %block65272 ], [ %v117641, %edge10437 ], [ %v217497, %block65595 ], [ %v217497, %edge10498 ]
  %v124662 = phi i64 [ %v113215.lcssa, %edge10440 ], [ %v113215.lcssa, %block65272 ], [ %v113215.lcssa, %edge10437 ], [ %v217498, %block65595 ], [ %v217498, %edge10498 ]
  %v124665 = phi i64 [ %v115400, %edge10440 ], [ %v115400, %block65272 ], [ %v115400, %edge10437 ], [ %v217499, %block65595 ], [ %v217499, %edge10498 ]
  br i1 %t6278, label %edge10506, label %edge10511

edge9839.edge9840_crit_edge:                      ; preds = %edge9834.thread
  %t6345.phi.trans.insert = inttoptr i64 %v111931 to ptr
  %v113171.pre = load i64, ptr %t6345.phi.trans.insert, align 1
  br label %edge9840

edge9840:                                         ; preds = %edge9839.edge9840_crit_edge, %edge9839.thread1202
  %t6345.pre-phi = phi ptr [ %t6345.phi.trans.insert, %edge9839.edge9840_crit_edge ], [ %t6335, %edge9839.thread1202 ]
  %v113171 = phi i64 [ %v113171.pre, %edge9839.edge9840_crit_edge ], [ %v112428, %edge9839.thread1202 ]
  %v1131158851213 = phi i64 [ %v111926, %edge9839.edge9840_crit_edge ], [ %v112418, %edge9839.thread1202 ]
  %v1131168871212 = phi i64 [ %v217502, %edge9839.edge9840_crit_edge ], [ %v112376, %edge9839.thread1202 ]
  %v1131178901211 = phi i64 [ %v111928, %edge9839.edge9840_crit_edge ], [ %v112419, %edge9839.thread1202 ]
  %v1131208931209 = phi i64 [ %v111931, %edge9839.edge9840_crit_edge ], [ %v112420, %edge9839.thread1202 ]
  %v113172 = add i64 %v113171, 1
  store i64 %v113172, ptr %t6345.pre-phi, align 1
  br label %edge9845

edge9845:                                         ; preds = %edge9834, %edge9814, %edge9840
  %t6344.not1201 = phi i1 [ false, %edge9840 ], [ true, %edge9814 ], [ true, %edge9834 ]
  %v1131158851200 = phi i64 [ %v1131158851213, %edge9840 ], [ %v112418, %edge9814 ], [ %v111926, %edge9834 ]
  %v1131168871199 = phi i64 [ %v1131168871212, %edge9840 ], [ %v112376, %edge9814 ], [ %v217502, %edge9834 ]
  %v1131178901198 = phi i64 [ %v1131178901211, %edge9840 ], [ %v112419, %edge9814 ], [ %v111928, %edge9834 ]
  %v1131208931196 = phi i64 [ %v1131208931209, %edge9840 ], [ 0, %edge9814 ], [ 0, %edge9834 ]
  %v113827 = add i64 %v1131158851200, -4
  br label %block4979

edge9857:                                         ; preds = %block4979
  %v113510 = add nsw i64 %v113215, -1
  %t6350 = icmp ult i64 %v113510, %v1131168871199
  br i1 %t6350, label %edge9876, label %block4979.backedge

block4979.backedge:                               ; preds = %edge9857, %edge9876
  br label %block4979

edge9876:                                         ; preds = %edge9857
  %v113824 = shl i64 %v113215, 2
  %v113828 = add i64 %v113827, %v113824
  %t6353 = inttoptr i64 %v113828 to ptr
  %t6354 = load i32, ptr %t6353, align 1
  %61 = icmp ugt i32 %t6354, 65535
  br i1 %61, label %block60779, label %block4979.backedge

edge9892:                                         ; preds = %block60779
  %t6360 = inttoptr i64 %v1131208931196 to ptr
  %v113896 = load i64, ptr %t6360, align 1
  %v113897 = add i64 %v113896, -1
  store i64 %v113897, ptr %t6360, align 1
  %t6362.not = icmp eq i64 %v113897, 0
  br i1 %t6362.not, label %edge9895, label %edge9911

edge9895:                                         ; preds = %edge9892
  %v113898 = call i64 @mech_compact_free(i64 %v0, i64 %v1131208931196)
  %t6363.not = icmp eq i64 %v113898, 0
  br i1 %t6363.not, label %edge9895.edge9911_crit_edge, label %edge9896

edge9895.edge9911_crit_edge:                      ; preds = %edge9895
  %v114371.pre = load i64, ptr %t6360, align 1
  %62 = add i64 %v114371.pre, 1
  br label %edge9911

edge9896:                                         ; preds = %edge9895
  %t6364 = insertvalue { i64, i64 } poison, i64 %v113898, 0
  %t6365 = insertvalue { i64, i64 } %t6364, i64 5099, 1
  br label %common.ret

edge9911:                                         ; preds = %edge9895.edge9911_crit_edge, %edge9892
  %v114371 = phi i64 [ %62, %edge9895.edge9911_crit_edge ], [ %v113896, %edge9892 ]
  store i64 %v114371, ptr %t6360, align 1
  br label %block5034.preheader

edge9928:                                         ; preds = %block5034
  %v114709 = add nsw i64 %v114415, -1
  %t6372 = icmp ult i64 %v114709, %v1131168871199
  br i1 %t6372, label %edge9947, label %block5034.backedge

block5034.backedge:                               ; preds = %edge9928, %edge9947
  br label %block5034

edge9947:                                         ; preds = %edge9928
  %v115023 = shl i64 %v114415, 2
  %v115027 = add i64 %v113827, %v115023
  %t6375 = inttoptr i64 %v115027 to ptr
  %t6376 = load i32, ptr %t6375, align 1
  %63 = icmp ugt i32 %t6376, 65535
  br i1 %63, label %block61488, label %block5034.backedge

edge9963:                                         ; preds = %block61488
  %t6382 = inttoptr i64 %v1131208931196 to ptr
  %v115093 = load i64, ptr %t6382, align 1
  %v115094 = add i64 %v115093, -1
  store i64 %v115094, ptr %t6382, align 1
  %t6384.not = icmp eq i64 %v115094, 0
  br i1 %t6384.not, label %edge9966, label %edge9982

edge9966:                                         ; preds = %edge9963
  %v115095 = call i64 @mech_compact_free(i64 %v0, i64 %v1131208931196)
  %t6385.not = icmp eq i64 %v115095, 0
  br i1 %t6385.not, label %edge9982, label %edge9967

edge9967:                                         ; preds = %edge9966
  %t6386 = insertvalue { i64, i64 } poison, i64 %v115095, 0
  %t6387 = insertvalue { i64, i64 } %t6386, i64 5156, 1
  br label %common.ret

edge9982:                                         ; preds = %edge9966, %edge9963, %block61488
  %v115400 = add i64 %v114415.lcssa, -288
  %v115406 = add i64 %v115400, %v113215.lcssa
  %t6392.not = icmp ult i64 %v115406, %v113215.lcssa
  br i1 %t6392.not, label %common.ret, label %edge10019

edge9999:                                         ; preds = %edge10022
  %t6403 = insertvalue { i64, i64 } poison, i64 %v116259, 0
  %t6404 = insertvalue { i64, i64 } %t6403, i64 5171, 1
  br label %common.ret

edge10019:                                        ; preds = %edge9982
  %v116333 = call i64 @mech_compact_allocate(i64 %v0, i64 19, i64 4, i64 4, i64 1, i64 %v3192)
  %t6417.not = icmp eq i64 %v116333, 0
  br i1 %t6417.not, label %edge10021, label %block61737

edge10021:                                        ; preds = %edge10019
  %v116336 = load i64, ptr %ss1, align 8
  %v116337 = load i64, ptr %t606, align 8
  %v116338 = load i64, ptr %t607, align 8
  %t6420.not = icmp eq i64 %v116338, 0
  br i1 %t6420.not, label %edge10059, label %edge10022

edge10022:                                        ; preds = %edge10021
  %t6421 = inttoptr i64 %v116338 to ptr
  %v116339 = load i64, ptr %t6421, align 1
  %v116255 = add i64 %v116339, 2
  store i64 %v116255, ptr %t6421, align 1
  %v116259 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v116338)
  %t6402.not = icmp eq i64 %v116259, 0
  br i1 %t6402.not, label %edge10059, label %edge9999

edge10039:                                        ; preds = %edge10062
  %t6432 = insertvalue { i64, i64 } poison, i64 %v117314, 0
  %t6433 = insertvalue { i64, i64 } %t6432, i64 5184, 1
  br label %common.ret

edge10059:                                        ; preds = %edge10022, %edge10021
  %v117365 = call i64 @mech_compact_allocate(i64 %v0, i64 320, i64 4, i64 4, i64 1, i64 %v3192)
  %t6446.not = icmp eq i64 %v117365, 0
  br i1 %t6446.not, label %edge10061, label %block61792

edge10061:                                        ; preds = %edge10059
  %v117368 = load i64, ptr %ss1, align 8
  %v117369 = load i64, ptr %t606, align 8
  %v117370 = load i64, ptr %t607, align 8
  %t6449.not = icmp eq i64 %v117370, 0
  br i1 %t6449.not, label %block5115, label %edge10062

edge10062:                                        ; preds = %edge10061
  %t6450 = inttoptr i64 %v117370 to ptr
  %v117371 = load i64, ptr %t6450, align 1
  %v117310 = add i64 %v117371, 2
  store i64 %v117310, ptr %t6450, align 1
  %v117314 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v117370)
  %t6431.not = icmp eq i64 %v117314, 0
  br i1 %t6431.not, label %block5115, label %edge10039

edge10068:                                        ; preds = %block5115
  %v117543 = load i64, ptr %.pre20761, align 1
  %v117544 = add i64 %v117543, 1
  store i64 %v117544, ptr %.pre20761, align 1
  br label %edge10077

edge10077:                                        ; preds = %block5115, %edge10068
  %v118544 = sub i64 288, %v113215.lcssa
  br label %block5117

edge10103:                                        ; preds = %block5117
  %t6460 = icmp ne i64 %v117644, %v117647
  br i1 %t6460, label %block5249, label %edge10108

edge10108:                                        ; preds = %edge10103
  %t6463 = icmp ult i64 %v117644, %v113215.lcssa
  %v118549 = select i1 %t6463, i64 0, i64 %v118544
  %v118554 = add i64 %v118549, %v117644
  %t6466 = icmp ult i64 %v118554, %v1131168871199
  br i1 %t6466, label %edge10135, label %edge10144

edge10135:                                        ; preds = %edge10108
  %v118884 = shl i64 %v118554, 2
  %v118885 = add i64 %v118884, %v1131158851200
  %t6469 = inttoptr i64 %v118885 to ptr
  %t6470 = load i32, ptr %t6469, align 1
  %64 = lshr i32 %t6470, 16
  %65 = and i32 %64, 255
  %66 = zext nneg i32 %65 to i64
  br label %edge10144

edge10144:                                        ; preds = %edge10135, %edge10108
  %v118907 = phi i64 [ %66, %edge10135 ], [ 0, %edge10108 ]
  %t6472.not = icmp eq i64 %v117644, -1
  br i1 %t6472.not, label %common.ret, label %edge10155

edge10155:                                        ; preds = %edge10144
  br i1 %t6344.not1201, label %block5186.preheader, label %edge10156

edge10156:                                        ; preds = %edge10155
  %v119247 = load i64, ptr %.pre20761, align 1
  %v119248 = add i64 %v119247, 1
  store i64 %v119248, ptr %.pre20761, align 1
  br label %block5186.preheader

block5186.preheader:                              ; preds = %edge10155, %edge10156
  br label %block5186

edge10201:                                        ; preds = %block5186
  %v119962 = shl i64 %v119631, 2
  %v119963 = add i64 %v119962, %v1131158851200
  %t6486 = inttoptr i64 %v119963 to ptr
  %t6487 = load i32, ptr %t6486, align 1
  %67 = lshr i32 %t6487, 16
  %68 = and i32 %67, 255
  %69 = zext nneg i32 %68 to i64
  br label %edge10210

edge10210:                                        ; preds = %edge10201, %block5186
  %v119985 = phi i64 [ %69, %edge10201 ], [ 0, %block5186 ]
  %t6489 = icmp eq i64 %v118907, %v119985
  %v120019199 = and i1 %t6479, %t6489
  br i1 %v120019199, label %edge10229, label %edge10218

edge10218:                                        ; preds = %edge10210
  br i1 %t6344.not1201, label %block5249, label %edge10219

edge10219:                                        ; preds = %edge10218
  %v120026 = load i64, ptr %.pre20761, align 1
  %v120027 = add i64 %v120026, -1
  store i64 %v120027, ptr %.pre20761, align 1
  %t6495.not = icmp eq i64 %v120027, 0
  br i1 %t6495.not, label %edge10222, label %block5249

edge10222:                                        ; preds = %edge10219
  %v120028 = call i64 @mech_compact_free(i64 %v0, i64 %v1131208931196)
  %t6496.not = icmp eq i64 %v120028, 0
  br i1 %t6496.not, label %block5249, label %edge10223

edge10223:                                        ; preds = %edge10222
  %t6497 = insertvalue { i64, i64 } poison, i64 %v120028, 0
  %t6498 = insertvalue { i64, i64 } %t6497, i64 5318, 1
  br label %common.ret

edge10229:                                        ; preds = %edge10210
  %t6499.not = icmp eq i64 %v119300, -1
  br i1 %t6499.not, label %common.ret, label %block5186

edge10242:                                        ; preds = %block5249
  %v120362 = sub nuw i64 %v217542, %v117644
  %t6507 = icmp eq i64 %v217544, 0
  br i1 %t6507, label %edge10251, label %edge10274

edge10251:                                        ; preds = %edge10242
  %t6510 = icmp ugt i64 %v120362, 10
  br i1 %t6510, label %edge10291.thread, label %edge10260

edge10260:                                        ; preds = %edge10251
  %t6513 = icmp samesign ugt i64 %v120362, 2
  %v121780 = select i1 %t6513, i64 17, i64 0
  %v121781 = select i1 %t6513, i64 3, i64 1
  %v121782 = select i1 %t6513, i64 10, i64 1
  br label %edge10291

edge10274:                                        ; preds = %edge10242
  %t6516 = icmp ugt i64 %v120362, 2
  %v121814200 = and i1 %t6460, %t6516
  %v122176 = select i1 %v121814200, i64 16, i64 %v217544
  %v122177 = select i1 %v121814200, i64 3, i64 1
  %v122178 = select i1 %v121814200, i64 6, i64 1
  br label %edge10291

edge10291:                                        ; preds = %edge10260, %edge10274
  %v122181 = phi i64 [ %v122176, %edge10274 ], [ %v121780, %edge10260 ]
  %v122184 = phi i64 [ %v122177, %edge10274 ], [ %v121781, %edge10260 ]
  %v122187 = phi i64 [ %v122178, %edge10274 ], [ %v121782, %edge10260 ]
  %spec.select32 = call i64 @llvm.umin.i64(i64 %v120362, i64 %v122187)
  %t6522.not = icmp samesign ult i64 %spec.select32, %v122184
  br i1 %t6522.not, label %common.ret, label %edge10301

edge10291.thread:                                 ; preds = %edge10251
  %spec.select32911 = call i64 @llvm.umin.i64(i64 %v120362, i64 138)
  %v122496919 = add nsw i64 %spec.select32911, -11
  br label %edge10331

edge10301:                                        ; preds = %edge10291
  %v122496 = sub nuw nsw i64 %spec.select32, %v122184
  %t6526 = icmp ult i64 %v122181, 19
  br i1 %t6526, label %edge10331, label %block65157

edge10331:                                        ; preds = %edge10301, %edge10291.thread
  %v122496926 = phi i64 [ %v122496919, %edge10291.thread ], [ %v122496, %edge10301 ]
  %v122181913923 = phi i64 [ 18, %edge10291.thread ], [ %v122181, %edge10301 ]
  %spec.select32915922 = phi i64 [ %spec.select32911, %edge10291.thread ], [ %spec.select32, %edge10301 ]
  %v122843 = shl nuw nsw i64 %v122181913923, 2
  %v122844 = add i64 %v122843, %v117617
  %t6529 = inttoptr i64 %v122844 to ptr
  %t6530 = load i32, ptr %t6529, align 1
  %v122928 = add i32 %t6530, 1
  %t6531.not = icmp eq i32 %t6530, -1
  br i1 %t6531.not, label %common.ret, label %edge10339

edge10339:                                        ; preds = %edge10331
  %t6541.not = icmp eq i64 %v117622, 0
  br i1 %t6541.not, label %block65162, label %edge10340

edge10340:                                        ; preds = %edge10339
  %t6542 = inttoptr i64 %v117622 to ptr
  %v122914 = load i64, ptr %t6542, align 1
  %t6543 = icmp eq i64 %v122914, 1
  br i1 %t6543, label %block65160, label %block65162

edge10345:                                        ; preds = %block65162
  %v122924 = load i64, ptr %ss1, align 8
  %v122925 = load i64, ptr %t606, align 8
  %v122926 = load i64, ptr %t607, align 8
  %.pre20689 = add i64 %v122924, %v122843
  %.pre20690 = inttoptr i64 %.pre20689 to ptr
  br label %block65160

edge10348:                                        ; preds = %block65160
  %t6567 = inttoptr i64 %v122913 to ptr
  %v122931 = load i64, ptr %t6567, align 1
  %v122932 = add i64 %v122931, 1
  store i64 %v122932, ptr %t6567, align 1
  br label %block65166

edge10362:                                        ; preds = %block65166
  %v123066 = shl nuw nsw i64 %v117641, 2
  %v123067 = add i64 %v123066, %v117629
  %t6575 = inttoptr i64 %v123067 to ptr
  %t6576 = load i32, ptr %t6575, align 1
  %v123068 = zext i32 %t6576 to i64
  %t6577 = icmp eq i64 %v123033, %v123068
  br i1 %t6577, label %block65180, label %edge10365

edge10365:                                        ; preds = %edge10362
  %t6579.not = icmp eq i64 %v122933, 0
  br i1 %t6579.not, label %block65183, label %edge10368

edge10368:                                        ; preds = %edge10365
  %v123081 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v122933)
  %t6580.not = icmp eq i64 %v123081, 0
  br i1 %t6580.not, label %block65183, label %block65177

edge10372:                                        ; preds = %block65183
  %t6582 = inttoptr i64 %v117634 to ptr
  %v123088 = load i64, ptr %t6582, align 1
  %t6583 = icmp eq i64 %v123088, 1
  br i1 %t6583, label %block65185, label %block65186

edge10377:                                        ; preds = %block65186
  %v123098 = load i64, ptr %ss1, align 8
  %v123099 = load i64, ptr %t606, align 8
  %v123100 = load i64, ptr %t607, align 8
  %.pre20687 = add i64 %v123098, %v123066
  %.pre20688 = inttoptr i64 %.pre20687 to ptr
  br label %block65185

edge10380:                                        ; preds = %block65180
  %t6607 = inttoptr i64 %v123077 to ptr
  %v123105 = load i64, ptr %t6607, align 1
  %v123106 = add i64 %v123105, 1
  store i64 %v123106, ptr %t6607, align 1
  br label %edge10394

edge10394:                                        ; preds = %edge10380, %block65180
  %v123107 = select i1 %t6604, i64 0, i64 %v117634
  %v123143 = add i64 %spec.select32915922, %v117644
  %t6616.not = icmp ult i64 %v123143, %v117644
  br i1 %t6616.not, label %common.ret, label %edge10423

edge10404:                                        ; preds = %block5117
  %t6620.not = icmp eq i64 %v117622, 0
  br i1 %t6620.not, label %block65237, label %edge10405

edge10405:                                        ; preds = %edge10404
  %t6621 = inttoptr i64 %v117622 to ptr
  %v123177 = load i64, ptr %t6621, align 1
  %v123178 = add i64 %v123177, 1
  store i64 %v123178, ptr %t6621, align 1
  br label %block65237

edge10408:                                        ; preds = %block65237
  %t6624 = inttoptr i64 %v117634 to ptr
  %v123179 = load i64, ptr %t6624, align 1
  %v123180 = add i64 %v123179, 1
  store i64 %v123180, ptr %t6624, align 1
  br label %edge10424

edge10423:                                        ; preds = %edge10394
  %t6637.not = icmp eq i64 %v123261, 0
  br i1 %t6637.not, label %block65280, label %edge10447

edge10424:                                        ; preds = %block65237, %edge10408
  br i1 %t6344.not1201, label %block65268, label %edge10427

edge10427:                                        ; preds = %edge10424
  %v123239 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1131208931196)
  %t6629.not = icmp eq i64 %v123239, 0
  br i1 %t6629.not, label %block65268, label %block65263

edge10433:                                        ; preds = %block65268
  %v123243 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v117622)
  %t6631.not = icmp eq i64 %v123243, 0
  br i1 %t6631.not, label %block65272, label %block65263

edge10437:                                        ; preds = %block65272
  %t6633 = inttoptr i64 %v117634 to ptr
  %v123245 = load i64, ptr %t6633, align 1
  %v123246 = add i64 %v123245, -1
  store i64 %v123246, ptr %t6633, align 1
  %t6635.not = icmp eq i64 %v123246, 0
  br i1 %t6635.not, label %edge10440, label %edge9837

edge10440:                                        ; preds = %edge10437
  %v123247 = call i64 @mech_compact_free(i64 %v0, i64 %v117634)
  %t6636.not = icmp eq i64 %v123247, 0
  br i1 %t6636.not, label %edge9837, label %block65263

edge10447:                                        ; preds = %edge10423
  %v123251 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v123261)
  %t6638.not = icmp eq i64 %v123251, 0
  br i1 %t6638.not, label %block65280, label %block65263

edge10451:                                        ; preds = %block65280
  %t6640 = inttoptr i64 %v123107 to ptr
  %v123253 = load i64, ptr %t6640, align 1
  %v123254 = add i64 %v123253, -1
  store i64 %v123254, ptr %t6640, align 1
  %t6642.not = icmp eq i64 %v123254, 0
  br i1 %t6642.not, label %edge10454, label %block5117.backedge

edge10454:                                        ; preds = %edge10451
  %v123255 = call i64 @mech_compact_free(i64 %v0, i64 %v123107)
  %t6643.not = icmp eq i64 %v123255, 0
  br i1 %t6643.not, label %block5117.backedge, label %block65263

edge10494:                                        ; preds = %edge9834.thread, %edge9825, %edge9826, %edge9834
  %v113120907 = phi i64 [ %v111931, %edge9834 ], [ %v217489, %edge9826 ], [ 0, %edge9825 ], [ %v111931, %edge9834.thread ]
  %v113119906 = phi i64 [ %v217502, %edge9834 ], [ %v217488, %edge9826 ], [ %v217488, %edge9825 ], [ %v217502, %edge9834.thread ]
  %v113117905 = phi i64 [ %v111928, %edge9834 ], [ %v217486, %edge9826 ], [ %v217486, %edge9825 ], [ %v111928, %edge9834.thread ]
  %v113116904 = phi i64 [ %v217502, %edge9834 ], [ %v217485, %edge9826 ], [ %v217485, %edge9825 ], [ %v217502, %edge9834.thread ]
  %v113115903 = phi i64 [ %v111926, %edge9834 ], [ %v217484, %edge9826 ], [ %v217484, %edge9825 ], [ %v111926, %edge9834.thread ]
  %t6646.not = icmp eq i64 %v217494, 0
  br i1 %t6646.not, label %block65595, label %edge10495

edge10495:                                        ; preds = %edge10494
  %t6647 = inttoptr i64 %v217494 to ptr
  %v123719 = load i64, ptr %t6647, align 1
  %v123720 = add i64 %v123719, 1
  store i64 %v123720, ptr %t6647, align 1
  br label %block65595

edge10498:                                        ; preds = %block65595
  %t6650 = inttoptr i64 %v217496 to ptr
  %v123721 = load i64, ptr %t6650, align 1
  %v123722 = add i64 %v123721, 1
  store i64 %v123722, ptr %t6650, align 1
  br label %edge9837

edge10504:                                        ; preds = %edge10512, %edge10511
  %t6660 = icmp eq i64 %v217478, 1
  br i1 %t6660, label %edge10519, label %edge10521

edge10506:                                        ; preds = %edge9837
  br i1 %t4820.not, label %edge10519, label %edge10507

edge10507:                                        ; preds = %edge10506
  %v124220 = load i64, ptr %t6655.pre-phi, align 1
  %v124221 = add i64 %v124220, 1
  store i64 %v124221, ptr %t6655.pre-phi, align 1
  br label %edge10519

edge10511:                                        ; preds = %edge9837
  %t6657.not = icmp eq i64 %v124238, 0
  br i1 %t6657.not, label %edge10504, label %edge10512

edge10512:                                        ; preds = %edge10511
  %t6658 = inttoptr i64 %v124238 to ptr
  %v124239 = load i64, ptr %t6658, align 1
  %v124240 = add i64 %v124239, 1
  store i64 %v124240, ptr %t6658, align 1
  br label %edge10504

edge10519:                                        ; preds = %edge10506, %edge10507, %edge10504
  %v124703973 = phi i64 [ %v124237, %edge10504 ], [ 320, %edge10507 ], [ 320, %edge10506 ]
  %v124700972 = phi i64 [ %v124234, %edge10504 ], [ 320, %edge10507 ], [ 320, %edge10506 ]
  %v124699971 = phi i64 [ %v124233, %edge10504 ], [ %v80926, %edge10507 ], [ %v80926, %edge10506 ]
  %v124336970 = phi i64 [ %v124238, %edge10504 ], [ %v80920, %edge10507 ], [ 0, %edge10506 ]
  %t6674.not = icmp eq i64 %v217489, 0
  br i1 %t6674.not, label %block65850, label %edge10550

edge10521:                                        ; preds = %edge10504
  %t6663.not = icmp eq i64 %v113120894, 0
  br i1 %t6663.not, label %block65834, label %edge10524

edge10524:                                        ; preds = %edge10521
  %v124288 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v113120894)
  %t6664.not = icmp eq i64 %v124288, 0
  br i1 %t6664.not, label %block65834, label %block65825

edge10530:                                        ; preds = %block65834
  %v124292 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v124238)
  %t6666.not = icmp eq i64 %v124292, 0
  br i1 %t6666.not, label %block65838, label %block65825

edge10536:                                        ; preds = %block65838
  %v124296 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v124335)
  %t6668.not = icmp eq i64 %v124296, 0
  br i1 %t6668.not, label %block65842, label %block65825

edge10540:                                        ; preds = %block65842
  %t6670 = inttoptr i64 %v124238 to ptr
  %v124298 = load i64, ptr %t6670, align 1
  %v124299 = add i64 %v124298, -1
  store i64 %v124299, ptr %t6670, align 1
  %t6672.not = icmp eq i64 %v124299, 0
  br i1 %t6672.not, label %edge10543, label %edge10604

edge10543:                                        ; preds = %edge10540
  %v124300 = call i64 @mech_compact_free(i64 %v0, i64 %v124238)
  %t6673.not = icmp eq i64 %v124300, 0
  br i1 %t6673.not, label %edge10604, label %block65825

edge10550:                                        ; preds = %edge10519
  %v124304 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217489)
  %t6675.not = icmp eq i64 %v124304, 0
  br i1 %t6675.not, label %block65850, label %block65825

edge10556:                                        ; preds = %block65850
  %v124308 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v111931)
  %t6677.not = icmp eq i64 %v124308, 0
  br i1 %t6677.not, label %block65854, label %block65825

edge10562:                                        ; preds = %block65854
  %v124312 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217494)
  %t6679.not = icmp eq i64 %v124312, 0
  br i1 %t6679.not, label %block65858, label %block65825

edge10568:                                        ; preds = %block65858
  %v124316 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217496)
  %t6681.not = icmp eq i64 %v124316, 0
  br i1 %t6681.not, label %edge10575, label %block65825

edge10575:                                        ; preds = %block65858, %edge10568
  %v124637 = add nuw nsw i64 %v217478, 1
  %spec.select33 = select i1 %t6278, i64 32, i64 19
  %spec.select34 = select i1 %t6278, i64 15, i64 7
  br label %block3784

edge10604:                                        ; preds = %block65842, %edge10540, %edge10543
  %t6685.not = icmp eq i64 %v111931, 0
  br i1 %t6685.not, label %block5407.preheader, label %edge10605

edge10605:                                        ; preds = %edge10604
  %t6686 = inttoptr i64 %v111931 to ptr
  %v127472 = load i64, ptr %t6686, align 1
  %v127473 = add i64 %v127472, 1
  store i64 %v127473, ptr %t6686, align 1
  br label %block5407.preheader

block5407.preheader:                              ; preds = %edge10604, %edge10605
  br label %block5407

edge10666:                                        ; preds = %block5407
  %v127873 = add nsw i64 %v127540, -1
  %v128181 = add i64 %v127540, add (i64 ptrtoint (ptr @mech_compact_table_15 to i64), i64 -1)
  %t6694 = inttoptr i64 %v128181 to ptr
  %t6695 = load i8, ptr %t6694, align 1
  %v128182 = zext i8 %t6695 to i64
  %t6696 = icmp samesign ugt i64 %v217502, %v128182
  br i1 %t6696, label %edge10674, label %block5407.backedge

block5407.backedge:                               ; preds = %edge10666, %edge10674
  br label %block5407

edge10674:                                        ; preds = %edge10666
  %v128790 = shl nuw nsw i64 %v128182, 2
  %v128791 = add i64 %v128790, %v111926
  %t6699 = inttoptr i64 %v128791 to ptr
  %t6700 = load i32, ptr %t6699, align 1
  %70 = icmp ugt i32 %t6700, 65535
  br i1 %70, label %block67063, label %block5407.backedge

edge10690:                                        ; preds = %block67063
  %t6706 = inttoptr i64 %v111931 to ptr
  %v128857 = load i64, ptr %t6706, align 1
  %v128858 = add i64 %v128857, -1
  store i64 %v128858, ptr %t6706, align 1
  %t6708.not = icmp eq i64 %v128858, 0
  br i1 %t6708.not, label %edge10693, label %edge10759

edge10693:                                        ; preds = %edge10690
  %v128859 = call i64 @mech_compact_free(i64 %v0, i64 %v111931)
  %t6709.not = icmp eq i64 %v128859, 0
  br i1 %t6709.not, label %edge10759, label %edge10694

edge10694:                                        ; preds = %edge10693
  %t6710 = insertvalue { i64, i64 } poison, i64 %v128859, 0
  %t6711 = insertvalue { i64, i64 } %t6710, i64 5565, 1
  br label %common.ret

edge10759:                                        ; preds = %edge10693, %edge10690, %block67063, %edge7450, %edge7447, %block43018, %edge7391, %edge7395
  %v129479 = phi i64 [ %v64800, %edge7450 ], [ %v64800, %edge7391 ], [ %v64800, %edge7395 ], [ %v64800, %block43018 ], [ %v64800, %edge7447 ], [ %v217484, %block67063 ], [ %v217484, %edge10690 ], [ %v217484, %edge10693 ]
  %v129480 = phi i64 [ %v64801, %edge7450 ], [ %v64801, %edge7391 ], [ %v64801, %edge7395 ], [ %v64801, %block43018 ], [ %v64801, %edge7447 ], [ %v217485, %block67063 ], [ %v217485, %edge10690 ], [ %v217485, %edge10693 ]
  %v129483 = phi i64 [ %v64804, %edge7450 ], [ %v64804, %edge7391 ], [ %v64804, %edge7395 ], [ %v64804, %block43018 ], [ %v64804, %edge7447 ], [ %v217488, %block67063 ], [ %v217488, %edge10690 ], [ %v217488, %edge10693 ]
  %v129484 = phi i64 [ %v64805, %edge7450 ], [ %v64805, %edge7391 ], [ %v64805, %edge7395 ], [ %v64805, %block43018 ], [ %v64805, %edge7447 ], [ %v217489, %block67063 ], [ %v217489, %edge10690 ], [ %v217489, %edge10693 ]
  %v129495 = phi i64 [ %v64911, %edge7450 ], [ %v64911, %edge7391 ], [ %v64911, %edge7395 ], [ %v64911, %block43018 ], [ %v64911, %edge7447 ], [ %v217490, %block67063 ], [ %v217490, %edge10690 ], [ %v217490, %edge10693 ]
  %v129500 = phi i64 [ %v64916, %edge7450 ], [ %v64916, %edge7391 ], [ %v64916, %edge7395 ], [ %v64916, %block43018 ], [ %v64916, %edge7447 ], [ %v217494, %block67063 ], [ %v217494, %edge10690 ], [ %v217494, %edge10693 ]
  %v129501 = phi i64 [ %v64923, %edge7450 ], [ %v64923, %edge7391 ], [ %v64923, %edge7395 ], [ %v64923, %block43018 ], [ %v64923, %edge7447 ], [ %v111926, %block67063 ], [ %v111926, %edge10690 ], [ %v111926, %edge10693 ]
  %v129502 = phi i64 [ %v64924, %edge7450 ], [ %v64924, %edge7391 ], [ %v64924, %edge7395 ], [ %v64924, %block43018 ], [ %v64924, %edge7447 ], [ %v217502, %block67063 ], [ %v217502, %edge10690 ], [ %v217502, %edge10693 ]
  %v129506 = phi i64 [ %v64928, %edge7450 ], [ %v64928, %edge7391 ], [ %v64928, %edge7395 ], [ %v64928, %block43018 ], [ %v64928, %edge7447 ], [ 0, %block67063 ], [ %v111931, %edge10690 ], [ %v111931, %edge10693 ]
  %v129507 = phi i64 [ %v64935, %edge7450 ], [ %v64935, %edge7391 ], [ %v64935, %edge7395 ], [ %v64935, %block43018 ], [ %v64935, %edge7447 ], [ %v217495, %block67063 ], [ %v217495, %edge10690 ], [ %v217495, %edge10693 ]
  %v129512 = phi i64 [ %v64940, %edge7450 ], [ %v64940, %edge7391 ], [ %v64940, %edge7395 ], [ %v64940, %block43018 ], [ %v64940, %edge7447 ], [ %v217496, %block67063 ], [ %v217496, %edge10690 ], [ %v217496, %edge10693 ]
  %v129513 = phi i64 [ %v64947, %edge7450 ], [ %v64947, %edge7391 ], [ %v64947, %edge7395 ], [ %v64947, %block43018 ], [ %v64947, %edge7447 ], [ %v217497, %block67063 ], [ %v217497, %edge10690 ], [ %v217497, %edge10693 ]
  %v129514 = phi i64 [ %v64950, %edge7450 ], [ %v64950, %edge7391 ], [ %v64950, %edge7395 ], [ %v64950, %block43018 ], [ %v64950, %edge7447 ], [ %v217498, %block67063 ], [ %v217498, %edge10690 ], [ %v217498, %edge10693 ]
  %v129515 = phi i64 [ %v64953, %edge7450 ], [ %v64953, %edge7391 ], [ %v64953, %edge7395 ], [ %v64953, %block43018 ], [ %v64953, %edge7447 ], [ %v217499, %block67063 ], [ %v217499, %edge10690 ], [ %v217499, %edge10693 ]
  %v129516 = phi i64 [ %v64956, %edge7450 ], [ %v64956, %edge7391 ], [ %v64956, %edge7395 ], [ %v64956, %block43018 ], [ %v64956, %edge7447 ], [ %v127540.lcssa, %block67063 ], [ %v127540.lcssa, %edge10690 ], [ %v127540.lcssa, %edge10693 ]
  %v130269 = phi i64 [ 1, %edge7450 ], [ 1, %edge7391 ], [ 1, %edge7395 ], [ 1, %block43018 ], [ 1, %edge7447 ], [ 0, %block67063 ], [ 0, %edge10690 ], [ 0, %edge10693 ]
  %t6712 = icmp eq i64 %v64782, -1
  %71 = select i1 %t3928, i1 true, i1 %t6712
  %v130602 = select i1 %71, i64 %v64845, i64 %v64782
  %v130307 = or i64 %v130269, %v64999
  %t6717.not = icmp eq i64 %v130307, 0
  br i1 %t6717.not, label %edge10946, label %edge10780

edge10780:                                        ; preds = %edge11214, %edge11211, %block71150, %edge10759
  %v136868 = phi i64 [ %v130602, %edge10759 ], [ %v134185, %block71150 ], [ %v134185, %edge11211 ], [ %v134185, %edge11214 ]
  %v136452 = xor i64 %v64967, 1
  %v136513 = and i64 %v136452, %v64740
  %t6885.not = icmp eq i64 %v136513, 0
  br i1 %t6885.not, label %edge11269, label %edge11244

edge10946:                                        ; preds = %edge10759
  %t6775.not = icmp eq i64 %v129484, 0
  br i1 %t6775.not, label %edge10950, label %edge10947

edge10947:                                        ; preds = %edge10946
  %t6776 = inttoptr i64 %v129484 to ptr
  %v130614 = load i64, ptr %t6776, align 1
  %v130615 = add i64 %v130614, 1
  store i64 %v130615, ptr %t6776, align 1
  br label %edge10950

edge10950:                                        ; preds = %edge10946, %edge10947
  %t6778.not = icmp eq i64 %v80920, 0
  br i1 %t6778.not, label %edge10954, label %edge10951

edge10951:                                        ; preds = %edge10950
  %t6779 = inttoptr i64 %v80920 to ptr
  %v130618 = load i64, ptr %t6779, align 1
  %v130619 = add i64 %v130618, 1
  store i64 %v130619, ptr %t6779, align 1
  br label %edge10954

edge10954:                                        ; preds = %edge10950, %edge10951
  %t6785 = icmp ult i64 %v129516, 6148914691236517206
  br i1 %t6785, label %edge10960, label %common.ret

edge10960:                                        ; preds = %edge10954
  %v130648 = mul nuw i64 %v129516, 3
  %v130653 = add i64 %v130648, 14
  %t6789 = icmp samesign ult i64 %v129516, 6148914691236517201
  br i1 %t6789, label %edge10965, label %common.ret

edge10965:                                        ; preds = %edge10960
  %t6793.not = icmp eq i64 %v129500, 0
  br i1 %t6793.not, label %edge10969, label %edge10966

edge10966:                                        ; preds = %edge10965
  %t6794 = inttoptr i64 %v129500 to ptr
  %v130658 = load i64, ptr %t6794, align 1
  %v130659 = add i64 %v130658, 1
  store i64 %v130659, ptr %t6794, align 1
  br label %edge10969

edge10969:                                        ; preds = %edge10965, %edge10966
  %t6796.not = icmp eq i64 %v129506, 0
  br i1 %t6796.not, label %block5537.preheader, label %edge10970

edge10970:                                        ; preds = %edge10969
  %t6797 = inttoptr i64 %v129506 to ptr
  %v130662 = load i64, ptr %t6797, align 1
  %v130663 = add i64 %v130662, 1
  store i64 %v130663, ptr %t6797, align 1
  br label %block5537.preheader

block5537.preheader:                              ; preds = %edge10969, %edge10970
  br label %block5537

edge10999:                                        ; preds = %block5537
  %v131643 = add nuw i64 %v131024, ptrtoint (ptr @mech_compact_table_14 to i64)
  %t6803 = inttoptr i64 %v131643 to ptr
  %t6804 = load i8, ptr %t6803, align 1
  %v131644 = zext i8 %t6804 to i64
  %v132283 = shl nuw nsw i64 %v131024, 2
  %v132284 = add i64 %v132283, %v129495
  %t6808 = inttoptr i64 %v132284 to ptr
  %t6809 = load i32, ptr %t6808, align 1
  %v132285 = zext i32 %t6809 to i64
  %t6810 = icmp ult i64 %v131024, %v129502
  br i1 %t6810, label %edge11059, label %edge11074

edge11059:                                        ; preds = %edge10999
  %v132925 = add i64 %v132283, %v129501
  %t6813 = inttoptr i64 %v132925 to ptr
  %t6814 = load i32, ptr %t6813, align 1
  %72 = lshr i32 %t6814, 16
  %73 = zext nneg i32 %72 to i64
  br label %edge11074

edge11074:                                        ; preds = %edge10999, %edge11059
  %v132946 = phi i64 [ %73, %edge11059 ], [ 0, %edge10999 ]
  %74 = add nuw nsw i64 %v132946, %v131644
  %75 = mul nuw nsw i64 %74, %v132285
  %v132993 = add i64 %75, %v131027
  %t6828.not = icmp ult i64 %v132993, %v131027
  %v133274 = add nuw nsw i64 %v131024, 1
  br i1 %t6828.not, label %common.ret, label %block5537

edge11081:                                        ; preds = %block5537
  br i1 %t6793.not, label %block69505, label %edge11084

edge11084:                                        ; preds = %edge11081
  %v133004 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v129500)
  %t6835.not = icmp eq i64 %v133004, 0
  br i1 %t6835.not, label %block69505, label %block69501

edge11088:                                        ; preds = %block69505
  %t6837 = inttoptr i64 %v129506 to ptr
  %v133006 = load i64, ptr %t6837, align 1
  %v133007 = add i64 %v133006, -1
  store i64 %v133007, ptr %t6837, align 1
  %t6839.not = icmp eq i64 %v133007, 0
  br i1 %t6839.not, label %edge11091, label %block5650.preheader

edge11091:                                        ; preds = %edge11088
  %v133008 = call i64 @mech_compact_free(i64 %v0, i64 %v129506)
  %t6840.not = icmp eq i64 %v133008, 0
  br i1 %t6840.not, label %block5650.preheader, label %block69501

block5650.preheader:                              ; preds = %edge11091, %edge11088, %block69505
  br label %block5650

edge11139:                                        ; preds = %block5650
  %v134781 = add nuw i64 %v134182, ptrtoint (ptr @mech_compact_table_21 to i64)
  %t6848 = inttoptr i64 %v134781 to ptr
  %t6849 = load i8, ptr %t6848, align 1
  %v134782 = zext i8 %t6849 to i64
  %v135400 = shl nuw nsw i64 %v134182, 2
  %v135401 = add i64 %v135400, %v80926
  %t6850 = inttoptr i64 %v135401 to ptr
  %t6851 = load i32, ptr %t6850, align 1
  %v135402 = zext i32 %t6851 to i64
  %t6852 = icmp ult i64 %v134182, %v129480
  br i1 %t6852, label %edge11182, label %edge11197

edge11182:                                        ; preds = %edge11139
  %v136021 = add i64 %v135400, %v129479
  %t6855 = inttoptr i64 %v136021 to ptr
  %t6856 = load i32, ptr %t6855, align 1
  %76 = lshr i32 %t6856, 16
  %77 = zext nneg i32 %76 to i64
  br label %edge11197

edge11197:                                        ; preds = %edge11139, %edge11182
  %v136042 = phi i64 [ %77, %edge11182 ], [ 0, %edge11139 ]
  %78 = add nuw nsw i64 %v136042, %v134782
  %79 = mul nuw nsw i64 %78, %v135402
  %v136088 = add i64 %79, %v134185
  %t6870.not = icmp ult i64 %v136088, %v134185
  %v136359 = add nuw nsw i64 %v134182, 1
  br i1 %t6870.not, label %common.ret, label %block5650

edge11204:                                        ; preds = %block5650
  br i1 %t6778.not, label %block71150, label %edge11207

edge11207:                                        ; preds = %edge11204
  %v136099 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v80920)
  %t6877.not = icmp eq i64 %v136099, 0
  br i1 %t6877.not, label %block71150, label %block71146

edge11211:                                        ; preds = %block71150
  %t6879 = inttoptr i64 %v129484 to ptr
  %v136101 = load i64, ptr %t6879, align 1
  %v136102 = add i64 %v136101, -1
  store i64 %v136102, ptr %t6879, align 1
  %t6881.not = icmp eq i64 %v136102, 0
  br i1 %t6881.not, label %edge11214, label %edge10780

edge11214:                                        ; preds = %edge11211
  %v136103 = call i64 @mech_compact_free(i64 %v0, i64 %v129484)
  %t6882.not = icmp eq i64 %v136103, 0
  br i1 %t6882.not, label %edge10780, label %block71146

edge11242:                                        ; preds = %edge11244
  %t6891.not = icmp ult i64 %v136868, %v64782
  br i1 %t6891.not, label %edge11260, label %edge11269

edge11244:                                        ; preds = %edge10780
  %t6887.not = icmp eq i64 %v136868, -1
  br i1 %t6887.not, label %common.ret, label %edge11242

edge11260:                                        ; preds = %edge11242
  %t6896.not = icmp eq i64 %v64775, 0
  br i1 %t6896.not, label %edge11271, label %edge11264

edge11264:                                        ; preds = %edge11260
  %v137123 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64775)
  %t6897.not = icmp eq i64 %v137123, 0
  br i1 %t6897.not, label %edge11271, label %edge11265

edge11265:                                        ; preds = %edge11264
  %t6898 = insertvalue { i64, i64 } poison, i64 %v137123, 0
  %t6899 = insertvalue { i64, i64 } %t6898, i64 5879, 1
  br label %common.ret

edge11269:                                        ; preds = %edge10780, %edge11242, %edge11272, %edge11271
  %spec.select36992 = phi i64 [ %v136868, %edge11272 ], [ %v136868, %edge11271 ], [ %v64782, %edge11242 ], [ %v64782, %edge10780 ]
  %t6892.not991 = phi i1 [ false, %edge11272 ], [ false, %edge11271 ], [ true, %edge11242 ], [ true, %edge10780 ]
  %v136899990 = phi i64 [ 1, %edge11272 ], [ 1, %edge11271 ], [ 0, %edge11242 ], [ 0, %edge10780 ]
  %v138168 = phi i64 [ %v66492, %edge11272 ], [ 0, %edge11271 ], [ %v64775, %edge11242 ], [ %v64775, %edge10780 ]
  %v144765 = phi i64 [ %v67290, %edge11272 ], [ %v67290, %edge11271 ], [ %v64770, %edge11242 ], [ %v64770, %edge10780 ]
  %v144771 = phi i64 [ %v67291, %edge11272 ], [ %v67291, %edge11271 ], [ %v64772, %edge11242 ], [ %v64772, %edge10780 ]
  %v137964 = or i64 %v136899990, %v64743
  %t6903 = icmp eq i64 %v137964, 1
  br i1 %t6903, label %edge11445, label %block5990

edge11271:                                        ; preds = %edge11260, %edge11264
  %t6900.not = icmp eq i64 %v66492, 0
  br i1 %t6900.not, label %edge11269, label %edge11272

edge11272:                                        ; preds = %edge11271
  %t6901 = inttoptr i64 %v66492 to ptr
  %v137607 = load i64, ptr %t6901, align 1
  %v137608 = add i64 %v137607, 1
  store i64 %v137608, ptr %t6901, align 1
  br label %edge11269

edge11445:                                        ; preds = %edge11269
  %t6959.not = icmp eq i64 %v129484, 0
  br i1 %t6959.not, label %edge11449, label %edge11446

edge11446:                                        ; preds = %edge11445
  %t6960 = inttoptr i64 %v129484 to ptr
  %v138189 = load i64, ptr %t6960, align 1
  %v138190 = add i64 %v138189, 1
  store i64 %v138190, ptr %t6960, align 1
  br label %edge11449

edge11449:                                        ; preds = %edge11445, %edge11446
  %t6962.not = icmp eq i64 %v66492, 0
  br i1 %t6962.not, label %block5798.preheader, label %edge11450

edge11450:                                        ; preds = %edge11449
  %t6963 = inttoptr i64 %v66492 to ptr
  %v138961 = load i64, ptr %t6963, align 1
  %v138962 = add i64 %v138961, 1
  store i64 %v138962, ptr %t6963, align 1
  br label %block5798.preheader

block5798.preheader:                              ; preds = %edge11449, %edge11450
  br label %block5798

edge11460:                                        ; preds = %block5798
  %t6966 = inttoptr i64 %v138987 to ptr
  %v138988 = load i64, ptr %t6966, align 1
  %t6967 = icmp eq i64 %v138988, 1
  %t6969 = icmp ugt i64 %v139015, 546
  %or.cond314 = select i1 %t6967, i1 %t6969, i1 false
  br i1 %or.cond314, label %edge11464, label %edge11491.thread

edge11464:                                        ; preds = %edge11460
  %t6971 = zext nneg i64 %v139016 to i128
  %t6972 = zext i64 %v139017 to i128
  %t6973 = shl nuw i128 %t6972, 64
  %v218722 = or disjoint i128 %t6973, %t6971
  store i128 %v218722, ptr %ss2, align 16
  store i128 18446744073709552163, ptr %t926, align 16
  %t6981 = zext i64 %v129479 to i128
  %t6982 = shl nuw i128 %t6981, 64
  %v218726 = or disjoint i128 %t6982, 547
  store i128 %v218726, ptr %t931, align 16
  %t6985 = zext i64 %v129480 to i128
  %v218728 = or disjoint i128 %t6985, 18446744073709551616
  store i128 %v218728, ptr %t936, align 16
  %t6990 = zext i64 %v129483 to i128
  %v218730 = or disjoint i128 %t6992, %t6990
  store i128 %v218730, ptr %t941, align 16
  store i128 18446744073709552163, ptr %t946, align 16
  %t7001 = zext i64 %v138987 to i128
  %t7002 = shl nuw i128 %t7001, 64
  %v218734 = or disjoint i128 %t7002, 547
  store i128 %v218734, ptr %t947, align 16
  store i64 %v139015, ptr %t1152, align 16
  %t7006 = call fastcc { i64, i64 } @mech_compact_numeric_5895(i64 %v218103, i64 %v8479)
  %v138993 = extractvalue { i64, i64 } %t7006, 0
  %t7007.not = icmp eq i64 %v138993, 0
  br i1 %t7007.not, label %edge11468, label %edge11466

edge11466:                                        ; preds = %edge11464
  %v138994 = extractvalue { i64, i64 } %t7006, 1
  %t7008 = icmp eq i64 %v138993, 103169
  %v138997 = select i1 %t7008, i64 103425, i64 %v138993
  %t7010 = insertvalue { i64, i64 } poison, i64 %v138997, 0
  %t7011 = insertvalue { i64, i64 } %t7010, i64 %v138994, 1
  br label %common.ret

edge11468:                                        ; preds = %edge11464
  br i1 %t6959.not, label %edge11643, label %edge11471

edge11471:                                        ; preds = %edge11468
  %v139006 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v129484)
  %t7013.not = icmp eq i64 %v139006, 0
  br i1 %t7013.not, label %edge11643, label %edge11472

edge11472:                                        ; preds = %edge11471
  %t7014 = insertvalue { i64, i64 } poison, i64 %v139006, 0
  %t7015 = insertvalue { i64, i64 } %t7014, i64 5895, 1
  br label %common.ret

edge11491:                                        ; preds = %block5798
  %t7016 = icmp samesign ult i64 %v139016, 547
  br i1 %t7016, label %edge11497, label %edge11599

edge11491.thread:                                 ; preds = %edge11460
  %t70161005 = icmp samesign ult i64 %v139016, 547
  br i1 %t70161005, label %edge11497, label %edge11594

edge11497:                                        ; preds = %edge11491.thread, %edge11491
  %v139423 = shl nuw nsw i64 %v139016, 1
  %v139424 = add i64 %v139423, ptrtoint (ptr @mech_compact_table_11 to i64)
  %t7021 = inttoptr i64 %v139424 to ptr
  %t7022 = load i16, ptr %t7021, align 1
  %v139425 = zext i16 %t7022 to i64
  %v139562 = and i64 %v139425, 511
  %v139615 = lshr i64 %v139425, 9
  %t7026 = icmp ult i64 %v139562, %v129480
  br i1 %t7026, label %edge11564, label %edge11564.thread

edge11564.thread:                                 ; preds = %edge11497
  %t70321001 = icmp samesign ult i64 %v139562, 288
  %spec.select371002 = select i1 %t70321001, i64 13, i64 10
  br label %82

edge11564:                                        ; preds = %edge11497
  %v140194 = shl nuw nsw i64 %v139562, 2
  %v140195 = add i64 %v140194, %v129479
  %t7029 = inttoptr i64 %v140195 to ptr
  %t7030 = load i32, ptr %t7029, align 1
  %t7030.fr = freeze i32 %t7030
  %80 = lshr i32 %t7030.fr, 16
  %81 = zext nneg i32 %80 to i64
  %t7035.not = icmp eq i32 %80, 0
  %t7032 = icmp samesign ult i64 %v139562, 288
  %spec.select37 = select i1 %t7032, i64 13, i64 10
  %spec.select1215 = select i1 %t7035.not, i64 %spec.select37, i64 %81
  br label %82

82:                                               ; preds = %edge11564, %edge11564.thread
  %83 = phi i64 [ %spec.select1215, %edge11564 ], [ %spec.select371002, %edge11564.thread ]
  %v141258 = add nuw nsw i64 %83, %v139615
  %v141268 = shl nuw nsw i64 %v141258, 4
  %v141298 = shl nuw nsw i64 %v139016, 2
  %v141299 = add i64 %v139017, %v141298
  %t7054 = inttoptr i64 %v141299 to ptr
  %t7055 = load i32, ptr %t7054, align 1
  %v141300 = zext i32 %t7055 to i64
  %t7056 = icmp eq i64 %v141268, %v141300
  br i1 %t7056, label %block73561, label %edge11578

edge11578:                                        ; preds = %82
  br i1 %t6965.not, label %block73563, label %edge11579

edge11579:                                        ; preds = %edge11578
  %t7059 = inttoptr i64 %v138987 to ptr
  %v141316 = load i64, ptr %t7059, align 1
  %t7060 = icmp eq i64 %v141316, 1
  br i1 %t7060, label %block73562, label %block73563

edge11584:                                        ; preds = %block73563
  %v141326 = load i64, ptr %ss1, align 8
  %v141327 = load i64, ptr %t606, align 8
  %v141328 = load i64, ptr %t607, align 8
  %.pre20681 = add i64 %v141326, %v141298
  %.pre20682 = inttoptr i64 %.pre20681 to ptr
  br label %block73562

edge11587:                                        ; preds = %block73561
  %t7084 = inttoptr i64 %v141309 to ptr
  %v141333 = load i64, ptr %t7084, align 1
  %v141334 = add i64 %v141333, 1
  store i64 %v141334, ptr %t7084, align 1
  br label %edge11598

edge11594:                                        ; preds = %edge11491.thread
  %v141354 = add i64 %v138988, 1
  store i64 %v141354, ptr %t6966, align 1
  br label %edge11599

edge11598:                                        ; preds = %block73561, %edge11587
  %t7101.not = or i1 %t6965.not, %t7081
  br i1 %t7101.not, label %edge11624, label %edge11617

edge11599:                                        ; preds = %edge11491, %edge11594
  br i1 %t6959.not, label %edge11606, label %edge11602

edge11602:                                        ; preds = %edge11599
  %v141363 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v129484)
  %t7095.not = icmp eq i64 %v141363, 0
  br i1 %t7095.not, label %edge11606, label %block73578

edge11606:                                        ; preds = %edge11602, %edge11599
  %t7096.not = icmp eq i64 %v138987, 0
  br i1 %t7096.not, label %edge11643, label %edge11607

edge11607:                                        ; preds = %edge11606
  %t7097 = inttoptr i64 %v138987 to ptr
  %v141369 = load i64, ptr %t7097, align 1
  %v141370 = add i64 %v141369, -1
  store i64 %v141370, ptr %t7097, align 1
  %t7099.not = icmp eq i64 %v141370, 0
  br i1 %t7099.not, label %edge11610, label %edge11643

edge11610:                                        ; preds = %edge11607
  %v141371 = call i64 @mech_compact_free(i64 %v0, i64 %v138987)
  %t7100.not = icmp eq i64 %v141371, 0
  br i1 %t7100.not, label %edge11643, label %block73578

edge11617:                                        ; preds = %edge11598
  %v141375 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v138987)
  %t7102.not = icmp eq i64 %v141375, 0
  br i1 %t7102.not, label %edge11624, label %block73578

edge11624:                                        ; preds = %edge11598, %edge11617
  %v141634 = add nuw nsw i64 %v139016, 1
  br label %block5798

edge11643:                                        ; preds = %edge11468, %edge11471, %edge11610, %edge11607, %edge11606
  br i1 %t6892.not991, label %edge11647, label %edge11649

edge11647:                                        ; preds = %edge11643, %edge11650
  %v142009 = phi i64 [ %v141991, %edge11650 ], [ %v64785, %edge11643 ]
  %t7114.not = icmp eq i64 %v142009, 0
  %v142017 = select i1 %t7114.not, i64 0, i64 %v136899990
  %t7116.not205 = icmp eq i64 %v142017, 0
  %t7116.not = select i1 %t7115, i1 true, i1 %t7116.not205
  br i1 %t7116.not, label %edge11661, label %edge11672

edge11649:                                        ; preds = %edge11643
  %t7110.not = icmp eq i64 %v64785, 0
  br i1 %t7110.not, label %common.ret, label %edge11650

edge11650:                                        ; preds = %edge11649
  %v141991 = add i64 %v64785, -1
  br label %edge11647

edge11661:                                        ; preds = %edge11647
  br i1 %t6962.not, label %block5990, label %edge11662

edge11662:                                        ; preds = %edge11661
  %t7119 = inttoptr i64 %v66492 to ptr
  %v142234 = load i64, ptr %t7119, align 1
  %v142235 = add i64 %v142234, -1
  store i64 %v142235, ptr %t7119, align 1
  %t7121.not = icmp eq i64 %v142235, 0
  br i1 %t7121.not, label %edge11665, label %block5990

edge11665:                                        ; preds = %edge11662
  %v142236 = call i64 @mech_compact_free(i64 %v0, i64 %v66492)
  %t7122.not = icmp eq i64 %v142236, 0
  br i1 %t7122.not, label %block5990, label %edge11666

edge11666:                                        ; preds = %edge11665
  %t7123 = insertvalue { i64, i64 } poison, i64 %v142236, 0
  %t7124 = insertvalue { i64, i64 } %t7123, i64 6010, 1
  br label %common.ret

edge11672:                                        ; preds = %edge11647
  %t7125.not = icmp eq i64 %v138987, 0
  br i1 %t7125.not, label %block5917.preheader, label %edge11673

edge11673:                                        ; preds = %edge11672
  %t7126 = inttoptr i64 %v138987 to ptr
  %v142446 = load i64, ptr %t7126, align 1
  %v142447 = add i64 %v142446, 1
  store i64 %v142447, ptr %t7126, align 1
  br label %block5917.preheader

block5917.preheader:                              ; preds = %edge11672, %edge11673
  br label %block5917

edge11696:                                        ; preds = %block5917
  br i1 %t6962.not, label %block74297, label %edge11699

edge11699:                                        ; preds = %edge11696
  %v142537 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66492)
  %t7132.not = icmp eq i64 %v142537, 0
  br i1 %t7132.not, label %block74297, label %block74293

edge11703:                                        ; preds = %block74297
  %t7134 = inttoptr i64 %v138987 to ptr
  %v142539 = load i64, ptr %t7134, align 1
  %v142540 = add i64 %v142539, -1
  store i64 %v142540, ptr %t7134, align 1
  %t7136.not = icmp eq i64 %v142540, 0
  br i1 %t7136.not, label %edge11706, label %block5990

edge11706:                                        ; preds = %edge11703
  %v142541 = call i64 @mech_compact_free(i64 %v0, i64 %v138987)
  %t7137.not = icmp eq i64 %v142541, 0
  br i1 %t7137.not, label %block5990, label %block74293

edge11729:                                        ; preds = %block5917
  %v143096 = shl nuw nsw i64 %v142514, 2
  %v143097 = add i64 %v143096, %v67290
  %t7143 = inttoptr i64 %v143097 to ptr
  %t7144 = load i32, ptr %t7143, align 1
  %v143428 = add i64 %v143096, %v139017
  %t7148 = inttoptr i64 %v143428 to ptr
  %t7149 = load i32, ptr %t7148, align 1
  %t7150 = icmp eq i32 %t7144, %t7149
  %v143720 = add nuw nsw i64 %v142514, 1
  br i1 %t7150, label %block5917, label %edge11764

edge11764:                                        ; preds = %edge11729
  br i1 %t6962.not, label %block74960, label %edge11767

edge11767:                                        ; preds = %edge11764
  %v143464 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v66492)
  %t7154.not = icmp eq i64 %v143464, 0
  br i1 %t7154.not, label %block74960, label %block74956

edge11771:                                        ; preds = %block74960
  %t7156 = inttoptr i64 %v138987 to ptr
  %v143466 = load i64, ptr %t7156, align 1
  %v143467 = add i64 %v143466, -1
  store i64 %v143467, ptr %t7156, align 1
  %t7158.not = icmp eq i64 %v143467, 0
  br i1 %t7158.not, label %edge11774, label %block5990

edge11774:                                        ; preds = %edge11771
  %v143468 = call i64 @mech_compact_free(i64 %v0, i64 %v138987)
  %t7159.not = icmp eq i64 %v143468, 0
  br i1 %t7159.not, label %block5990, label %block74956

edge11828:                                        ; preds = %edge11830
  %t7174.not = icmp ugt i64 %v136868, %spec.select36992
  %v144553 = and i64 %v144172, %v143891
  br i1 %t7174.not, label %edge11843, label %edge11848

edge11830:                                        ; preds = %block5990
  %t7170.not = icmp eq i64 %spec.select36992, -1
  br i1 %t7170.not, label %common.ret, label %edge11828

edge11841.sink.split:                             ; preds = %edge11848, %edge11843
  %v217572.sink = phi i64 [ %v138168, %edge11843 ], [ %v217572, %edge11848 ]
  %v1445531011.ph = phi i64 [ %v144553, %edge11843 ], [ %v1445531012, %edge11848 ]
  %v1445581009.ph = phi i64 [ 1, %edge11843 ], [ 0, %edge11848 ]
  %v163331.ph = phi i64 [ %v144765, %edge11843 ], [ %v217579, %edge11848 ]
  %v163333.ph = phi i64 [ %v144771, %edge11843 ], [ %v217581, %edge11848 ]
  %t7181 = inttoptr i64 %v217572.sink to ptr
  %v144801 = load i64, ptr %t7181, align 1
  %v144802 = add i64 %v144801, 1
  store i64 %v144802, ptr %t7181, align 1
  br label %edge11841

edge11841:                                        ; preds = %edge11841.sink.split, %edge11848, %edge11843
  %v1445531011 = phi i64 [ %v1445531012, %edge11848 ], [ %v144553, %edge11843 ], [ %v1445531011.ph, %edge11841.sink.split ]
  %v1445581009 = phi i64 [ 0, %edge11848 ], [ 1, %edge11843 ], [ %v1445581009.ph, %edge11841.sink.split ]
  %v145038 = phi i64 [ 0, %edge11848 ], [ 0, %edge11843 ], [ %v217572.sink, %edge11841.sink.split ]
  %v163331 = phi i64 [ %v217579, %edge11848 ], [ %v144765, %edge11843 ], [ %v163331.ph, %edge11841.sink.split ]
  %v163333 = phi i64 [ %v217581, %edge11848 ], [ %v144771, %edge11843 ], [ %v163333.ph, %edge11841.sink.split ]
  %v144870207 = and i1 %v55218135.not, %t3926
  %v144951208 = and i1 %t7183, %v144870207
  %v144951 = zext i1 %v144951208 to i64
  %84 = xor i64 %v144951, 1
  %v144971 = select i1 %v144870207, i64 %84, i64 0
  %v145023 = or i64 %v144971, %v64983
  %t7184.not = icmp eq i64 %v145023, 0
  br i1 %t7184.not, label %edge11863, label %edge11862

edge11843:                                        ; preds = %edge11828
  %t7177.not = icmp eq i64 %v138168, 0
  br i1 %t7177.not, label %edge11841, label %edge11841.sink.split

edge11848:                                        ; preds = %edge11828.thread, %edge11828
  %v1445531012 = phi i64 [ %v1445531007, %edge11828.thread ], [ %v144553, %edge11828 ]
  %t7180.not = icmp eq i64 %v217572, 0
  br i1 %t7180.not, label %edge11841, label %edge11841.sink.split

edge11862:                                        ; preds = %edge11841
  %t7188.not = icmp eq i64 %v145038, 0
  br i1 %t7188.not, label %edge12041, label %edge11873

edge11863:                                        ; preds = %edge11841
  %t7186.not = icmp eq i64 %v64763, 0
  br i1 %t7186.not, label %block6652, label %edge11866

edge11866:                                        ; preds = %edge11863
  %v145032 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v64763)
  %t7187.not = icmp eq i64 %v145032, 0
  br i1 %t7187.not, label %block6652, label %block75817

edge11873:                                        ; preds = %edge11862
  %v145036 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v145038)
  %t7189.not = icmp eq i64 %v145036, 0
  br i1 %t7189.not, label %edge12041, label %block75817

edge12041:                                        ; preds = %edge11862, %edge11873
  %t7246.not = icmp eq i64 %v80920, 0
  br i1 %t7246.not, label %block6052.preheader, label %edge12042

edge12042:                                        ; preds = %edge12041
  %t7247 = inttoptr i64 %v80920 to ptr
  %v145266 = load i64, ptr %t7247, align 1
  %v145267 = add i64 %v145266, 1
  store i64 %v145267, ptr %t7247, align 1
  br label %block6052.preheader

block6052.preheader:                              ; preds = %edge12041, %edge12042
  br label %block6052

edge12063:                                        ; preds = %block6052
  %v145957 = shl nuw nsw i64 %v145334, 2
  %v145958 = add i64 %v145957, %v80926
  %t7254 = inttoptr i64 %v145958 to ptr
  %t7255 = load i32, ptr %t7254, align 1
  %v145959 = zext i32 %t7255 to i64
  %t7256 = icmp samesign ult i64 %v145280, %v145959
  br i1 %t7256, label %edge12084, label %edge12103

edge12084:                                        ; preds = %edge12063
  %v146268 = add nuw i64 %v145337, 1
  %t7259.not = icmp eq i64 %v145337, -1
  br i1 %t7259.not, label %common.ret, label %edge12103

edge12092:                                        ; preds = %block6052
  br i1 %t7246.not, label %edge12113, label %edge12093

edge12093:                                        ; preds = %edge12092
  %t7266 = inttoptr i64 %v80920 to ptr
  %v146290 = load i64, ptr %t7266, align 1
  %v146291 = add i64 %v146290, -1
  store i64 %v146291, ptr %t7266, align 1
  %t7268.not = icmp eq i64 %v146291, 0
  br i1 %t7268.not, label %edge12096, label %edge12113

edge12096:                                        ; preds = %edge12093
  %v146292 = call i64 @mech_compact_free(i64 %v0, i64 %v80920)
  %t7269.not = icmp eq i64 %v146292, 0
  br i1 %t7269.not, label %edge12113, label %edge12097

edge12097:                                        ; preds = %edge12096
  %t7270 = insertvalue { i64, i64 } poison, i64 %v146292, 0
  %t7271 = insertvalue { i64, i64 } %t7270, i64 6204, 1
  br label %common.ret

edge12103:                                        ; preds = %edge12063, %edge12084
  %v217617.ph = phi i64 [ %v145337, %edge12063 ], [ %v146268, %edge12084 ]
  %v146544 = add nuw nsw i64 %v145334, 1
  br label %block6052

edge12113:                                        ; preds = %edge12092, %edge12093, %edge12096
  %spec.select38 = call i64 @llvm.umax.i64(i64 %v145337, i64 1)
  %t7291 = icmp ugt i64 %v145337, 79
  br i1 %t7291, label %block6219, label %edge12143

edge12143:                                        ; preds = %edge12113
  %v148427 = add i64 %spec.select38, ptrtoint (ptr @mech_compact_table_6 to i64)
  %t7296 = inttoptr i64 %v148427 to ptr
  %t7297 = load i8, ptr %t7296, align 1
  %v148428 = zext i8 %t7297 to i64
  br label %block6219

edge12165:                                        ; preds = %block6219
  %v151308 = load i64, ptr %t7299, align 1
  %v151309 = add i64 %v151308, 1
  store i64 %v151309, ptr %t7299, align 1
  br label %block6221.preheader

edge12183:                                        ; preds = %block6221
  %v151973 = add nuw nsw i64 %v151352, 1
  %v151979 = shl nuw nsw i64 %v151352, 2
  %v151980 = add i64 %v151979, %v54931
  %t7305 = inttoptr i64 %v151980 to ptr
  %t7306 = load i32, ptr %t7305, align 1
  %v151981 = zext i32 %t7306 to i64
  %t7307 = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %v151355, i64 %v151981)
  %v152017 = extractvalue { i64, i1 } %t7307, 0
  %t7308 = extractvalue { i64, i1 } %t7307, 1
  br i1 %t7308, label %common.ret, label %edge12210

edge12184:                                        ; preds = %block6221
  br i1 %t7298.not, label %edge12242, label %edge12219

edge12210:                                        ; preds = %edge12183
  %t7313 = mul nuw nsw i64 %v151352, %v151981
  %t7319 = call { i64, i1 } @llvm.ssub.with.overflow.i64(i64 %v151358, i64 %t7313)
  %t7320 = extractvalue { i64, i1 } %t7319, 1
  br i1 %t7320, label %common.ret, label %edge12229

edge12219:                                        ; preds = %edge12184
  %v152373 = load i64, ptr %t7299, align 1
  %v152374 = add i64 %v152373, -1
  store i64 %v152374, ptr %t7299, align 1
  %t7328.not = icmp eq i64 %v152374, 0
  br i1 %t7328.not, label %edge12222, label %edge12242

edge12222:                                        ; preds = %edge12219
  %v152375 = call i64 @mech_compact_free(i64 %v0, i64 %v54936)
  %t7329.not = icmp eq i64 %v152375, 0
  br i1 %t7329.not, label %edge12242, label %edge12223

edge12223:                                        ; preds = %edge12222
  %t7330 = insertvalue { i64, i64 } poison, i64 %v152375, 0
  %t7331 = insertvalue { i64, i64 } %t7330, i64 6386, 1
  br label %common.ret

edge12229:                                        ; preds = %edge12210
  %v152340 = extractvalue { i64, i1 } %t7319, 0
  br label %block6221

edge12242:                                        ; preds = %edge12222, %edge12219, %edge12184
  %spec.select39 = call i64 @llvm.smax.i64(i64 %v151358, i64 0)
  %t7336 = icmp sgt i64 %v151355, %spec.select39
  br i1 %t7336, label %.thread, label %edge12259

edge12255:                                        ; preds = %edge12259
  %v153375 = shl nsw i64 %v151355, 2
  %t7345 = icmp sgt i64 %v153375, %spec.select39
  %cond.fr1013 = freeze i1 %t7345
  %v153653 = zext i1 %cond.fr1013 to i64
  %spec.select1228 = select i1 %cond.fr1013, i64 ptrtoint (ptr @mech_compact_table_25 to i64), i64 ptrtoint (ptr @mech_compact_table_24 to i64)
  br label %.thread

.thread:                                          ; preds = %edge12255, %edge12242
  %v15365810181024 = phi i64 [ %v153653, %edge12255 ], [ 2, %edge12242 ]
  %85 = phi i64 [ %spec.select1228, %edge12255 ], [ ptrtoint (ptr @mech_compact_table_26 to i64), %edge12242 ]
  %t7354 = icmp ult i64 %v145337, 257
  br i1 %t7354, label %edge12317, label %edge12347

edge12259:                                        ; preds = %edge12242
  %86 = add i64 %v151355, 2305843009213693952
  %t7340 = icmp ult i64 %86, 4611686018427387904
  br i1 %t7340, label %edge12255, label %common.ret

edge12317:                                        ; preds = %.thread
  %v155427 = add i64 %85, %spec.select38
  %t7364 = inttoptr i64 %v155427 to ptr
  %t7365 = load i8, ptr %t7364, align 1
  %v155428 = zext i8 %t7365 to i64
  br label %edge12347

edge12347:                                        ; preds = %edge12317, %.thread
  %v155453 = phi i64 [ %v155428, %edge12317 ], [ 0, %.thread ]
  %v156057 = add i64 %v15365810181024, ptrtoint (ptr @mech_compact_table_4 to i64)
  %t7373 = inttoptr i64 %v156057 to ptr
  %t7374 = load i8, ptr %t7373, align 1
  %v156058 = zext i8 %t7374 to i64
  br i1 %t3695, label %edge12365, label %edge12534

edge12365:                                        ; preds = %edge12347
  br i1 %t3704.not, label %block81635, label %edge12366

edge12366:                                        ; preds = %edge12365
  %v157149 = load i64, ptr %t7379, align 1
  %v157150 = add i64 %v157149, 1
  store i64 %v157150, ptr %t7379, align 1
  br label %block81635

edge12369:                                        ; preds = %block81635
  %v157151 = load i64, ptr %t7382, align 1
  %v157152 = add i64 %v157151, 1
  store i64 %v157152, ptr %t7382, align 1
  br label %block6392.preheader

edge12392:                                        ; preds = %block6392
  %v157783 = shl nuw nsw i64 %v157221, 2
  %v157784 = add i64 %v157783, %v217753
  %t7387 = inttoptr i64 %v157784 to ptr
  %t7388 = load i32, ptr %t7387, align 1
  %87 = zext i32 %t7388 to i64
  %umul209 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %87, i64 %v54955)
  %88 = extractvalue { i64, i1 } %umul209, 1
  br i1 %88, label %common.ret, label %edge12402

edge12402:                                        ; preds = %edge12392
  %v158378 = add i64 %v157783, %v54943
  %t7397 = inttoptr i64 %v158378 to ptr
  %t7398 = load i32, ptr %t7397, align 1
  %89 = zext i32 %t7398 to i64
  %umul210 = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %89, i64 %v217755)
  %90 = extractvalue { i64, i1 } %umul210, 1
  br i1 %90, label %common.ret, label %edge12454

edge12437:                                        ; preds = %block6392
  br i1 %t3704.not, label %block82729, label %edge12440

edge12440:                                        ; preds = %edge12437
  %v158730 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54948)
  %t7422.not = icmp eq i64 %v158730, 0
  br i1 %t7422.not, label %block82729, label %block82725

edge12444:                                        ; preds = %block82729
  %v158732 = load i64, ptr %t7382, align 1
  %v158733 = add i64 %v158732, -1
  store i64 %v158733, ptr %t7382, align 1
  %t7426.not = icmp eq i64 %v158733, 0
  br i1 %t7426.not, label %edge12447, label %edge12469

edge12447:                                        ; preds = %edge12444
  %v158734 = call i64 @mech_compact_free(i64 %v0, i64 %v217754)
  %t7427.not = icmp eq i64 %v158734, 0
  br i1 %t7427.not, label %edge12469, label %block82725

edge12454:                                        ; preds = %edge12402
  %v157812 = mul i64 %v54955, %87
  %v158405 = mul i64 %v217755, %89
  %t7407 = icmp ugt i64 %v158405, %v157812
  %v158695 = sub i64 %v158405, %v157812
  %91 = sub nuw i64 %v157812, %v158405
  %v158993 = select i1 %t7407, i64 %v158695, i64 %91
  %v158986 = add nuw nsw i64 %v157221, 1
  %v158994 = add i64 %v158993, %v157224
  %t7431.not = icmp ult i64 %v158994, %v157224
  br i1 %t7431.not, label %common.ret, label %block6392

edge12469:                                        ; preds = %block82729, %edge12444, %edge12447
  br i1 %brmerge10960, label %common.ret.loopexit21379.split.loop.exit31553, label %edge12473

edge12473:                                        ; preds = %edge12469
  %t7458 = icmp ugt i64 %v157224, %v159208
  br i1 %t7458, label %edge12534, label %edge12487

edge12487:                                        ; preds = %edge12473
  %v159488 = shl nuw nsw i64 %v157224, 2
  %t7471 = icmp samesign ugt i64 %v159488, %v159502
  br i1 %t7471, label %edge12534, label %edge12499

edge12499:                                        ; preds = %edge12487
  %v159790 = shl nuw nsw i64 %v157224, 1
  %t7484 = icmp samesign ugt i64 %v159790, %v159208
  br i1 %t7484, label %edge12534, label %edge12511

edge12511:                                        ; preds = %edge12499
  %t7492 = icmp samesign ugt i64 %v159790, %v159198212
  %v160351 = select i1 %t7492, i64 4, i64 2
  br label %edge12534

edge12534:                                        ; preds = %edge12347, %edge12487, %edge12499, %edge12511, %edge12473
  %v160719 = phi i64 [ 8, %edge12347 ], [ 8, %edge12473 ], [ 6, %edge12487 ], [ %v160351, %edge12511 ], [ 5, %edge12499 ]
  %v162609 = sub nuw nsw i64 8, %v160719
  br label %block6524

edge12535:                                        ; preds = %block6524
  %t7496 = inttoptr i64 %v160741 to ptr
  %v160742 = load i64, ptr %t7496, align 1
  %t7497 = icmp eq i64 %v160742, 1
  %t7499 = icmp ugt i64 %v160765, 546
  %or.cond315 = select i1 %t7497, i1 %t7499, i1 false
  br i1 %or.cond315, label %edge12539, label %edge12560.thread

edge12539:                                        ; preds = %edge12535
  %t7501 = zext nneg i64 %v155453 to i128
  %t7502 = zext i8 %t7374 to i128
  %t7503 = shl nuw nsw i128 %t7502, 64
  %v218742 = or disjoint i128 %t7503, %t7501
  store i128 %v218742, ptr %ss2, align 16
  %t7505 = zext nneg i64 %v160719 to i128
  %t7506 = zext nneg i64 %v160769 to i128
  %t7507 = shl nuw nsw i128 %t7506, 64
  %v218744 = or disjoint i128 %t7507, %t7505
  store i128 %v218744, ptr %t926, align 16
  %t7510 = zext i64 %v160770 to i128
  %v218746 = or disjoint i128 %t7510, 10090369008319124733952
  store i128 %v218746, ptr %t931, align 16
  store i128 10090369008319124733953, ptr %t936, align 16
  store i128 %v218750, ptr %t941, align 16
  store i128 10090369008319124733953, ptr %t946, align 16
  %t7530 = zext i64 %v160741 to i128
  %t7531 = zext i64 %v160765 to i128
  %t7532 = shl nuw i128 %t7531, 64
  %v218754 = or disjoint i128 %t7532, %t7530
  store i128 %v218754, ptr %t947, align 16
  %t7535 = call fastcc { i64, i64 } @mech_compact_numeric_6633(i64 %v218103, i64 %v8479)
  %v160747 = extractvalue { i64, i64 } %t7535, 0
  %t7536.not = icmp eq i64 %v160747, 0
  br i1 %t7536.not, label %edge12722, label %edge12541

edge12541:                                        ; preds = %edge12539
  %v160748 = extractvalue { i64, i64 } %t7535, 1
  %t7537 = icmp eq i64 %v160747, 103169
  %v160751 = select i1 %t7537, i64 103425, i64 %v160747
  %t7539 = insertvalue { i64, i64 } poison, i64 %v160751, 0
  %t7540 = insertvalue { i64, i64 } %t7539, i64 %v160748, 1
  br label %common.ret

edge12560:                                        ; preds = %block6524
  %exitcond20601.not = icmp eq i64 %v160769, 547
  br i1 %exitcond20601.not, label %edge12722, label %edge12614

edge12560.thread:                                 ; preds = %edge12535
  %exitcond20601.not21196 = icmp eq i64 %v160769, 547
  br i1 %exitcond20601.not21196, label %edge12688, label %edge12614

edge12614:                                        ; preds = %edge12560.thread, %edge12560
  %v161108 = shl nuw nsw i64 %v160769, 1
  %v161109 = add nuw i64 %v161108, ptrtoint (ptr @mech_compact_table_11 to i64)
  %t7546 = inttoptr i64 %v161109 to ptr
  %t7547 = load i16, ptr %t7546, align 1
  %92 = lshr i16 %t7547, 5
  %93 = and i16 %92, 2032
  %94 = zext nneg i16 %93 to i64
  %t75491028 = icmp samesign ult i64 %v160769, 259
  %t75521029 = icmp samesign ult i64 %v160769, 515
  %v1619711030 = select i1 %t75521029, i64 %v155453, i64 78
  %v1625841031 = select i1 %t75491028, i64 %v156058, i64 %v1619711030
  %v162530 = shl nuw nsw i64 %v160769, 2
  %v162531 = add i64 %v160770, %v162530
  %t7560 = inttoptr i64 %v162531 to ptr
  %t7561 = load i32, ptr %t7560, align 1
  %v162532 = zext i32 %t7561 to i64
  %v162585 = add nuw nsw i64 %v1625841031, %94
  %v162600 = mul nuw nsw i64 %v162585, %v160719
  %v162617 = mul nuw nsw i64 %v162609, %v162532
  %v162622 = add nuw nsw i64 %v162600, %v162617
  %v162631213 = lshr i64 %v162622, 3
  %t7606 = icmp eq i64 %v162631213, %v162532
  br i1 %t7606, label %block85082, label %edge12659

edge12659:                                        ; preds = %edge12614
  %t7608.not = icmp eq i64 %v162963, 0
  br i1 %t7608.not, label %block85085, label %edge12662

edge12662:                                        ; preds = %edge12659
  %v162935 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v162963)
  %t7609.not = icmp eq i64 %v162935, 0
  br i1 %t7609.not, label %block85085, label %block85079

edge12666:                                        ; preds = %block85085
  %t7611 = inttoptr i64 %v160741 to ptr
  %v162942 = load i64, ptr %t7611, align 1
  %t7612 = icmp eq i64 %v162942, 1
  br i1 %t7612, label %block85087, label %block85088

edge12671:                                        ; preds = %block85088
  %v162952 = load i64, ptr %ss1, align 8
  %v162953 = load i64, ptr %t606, align 8
  %v162954 = load i64, ptr %t607, align 8
  %.pre20685 = add i64 %v162952, %v162530
  %.pre20686 = inttoptr i64 %.pre20685 to ptr
  br label %block85087

edge12674:                                        ; preds = %block85082
  %t7636 = inttoptr i64 %v162931 to ptr
  %v162959 = load i64, ptr %t7636, align 1
  %v162960 = add i64 %v162959, 1
  store i64 %v162960, ptr %t7636, align 1
  br label %edge12685

edge12685:                                        ; preds = %block85082, %edge12674
  %t7651.not = or i1 %t7495.not, %t7633
  br i1 %t7651.not, label %edge12705, label %edge12698

edge12688:                                        ; preds = %edge12560.thread
  %t7642 = inttoptr i64 %v160741 to ptr
  %v162984 = load i64, ptr %t7642, align 1
  %t7649.not = icmp eq i64 %v162984, 0
  br i1 %t7649.not, label %edge12691, label %edge12722

edge12691:                                        ; preds = %edge12688
  %v162998 = call i64 @mech_compact_free(i64 %v0, i64 %v160741)
  %t7650.not = icmp eq i64 %v162998, 0
  br i1 %t7650.not, label %edge12722, label %block85103

edge12698:                                        ; preds = %edge12685
  %v163002 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v160741)
  %t7652.not = icmp eq i64 %v163002, 0
  br i1 %t7652.not, label %edge12705, label %block85103

edge12705:                                        ; preds = %edge12685, %edge12698
  %v163256 = add nuw nsw i64 %v160769, 1
  br label %block6524

edge12722:                                        ; preds = %edge12560, %edge12539, %edge12691, %edge12688
  %v16074114716 = phi i64 [ %v160741, %edge12688 ], [ %v160741, %edge12539 ], [ %v160741, %edge12691 ], [ 0, %edge12560 ]
  br i1 %t3927, label %edge12730, label %block6652

edge12730:                                        ; preds = %edge12722
  %t7660.not = icmp eq i64 %v7457821133, 0
  br i1 %t7660.not, label %block6652, label %edge12739

edge12739:                                        ; preds = %edge12730
  %v164148 = shl i64 %v7457821133, 3
  %v164151 = add i64 %v7457721129, -8
  %v164152 = add i64 %v164151, %v164148
  %t7665 = inttoptr i64 %v164152 to ptr
  %v164153 = load i64, ptr %t7665, align 1
  %95 = lshr i64 %v164153, 4
  %96 = and i64 %95, 268435455
  %97 = add nuw nsw i64 %96, 7
  br label %block6652

edge12755:                                        ; preds = %block6652
  %t7673.not = icmp eq i64 %v144188, 0
  br i1 %t7673.not, label %edge12760, label %block6676

edge12760:                                        ; preds = %edge12755
  %t7675.not = icmp eq i64 %v1445531011, 0
  %spec.select40 = select i1 %t7675.not, i64 0, i64 4
  br label %block6676

edge12786:                                        ; preds = %block6676
  %t7701.not = icmp eq i64 %v217631, 0
  br i1 %t7701.not, label %block2907.backedge, label %edge12842

block2907.backedge:                               ; preds = %edge12786, %edge12842, %edge12845
  br label %block2907

edge12787:                                        ; preds = %block6676
  br i1 %t7298.not, label %block86572, label %edge12790

edge12790:                                        ; preds = %edge12787
  %v165443 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54936)
  %t7682.not = icmp eq i64 %v165443, 0
  br i1 %t7682.not, label %block86572, label %block86567

edge12796:                                        ; preds = %block86572
  %v165447 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217754)
  %t7684.not = icmp eq i64 %v165447, 0
  br i1 %t7684.not, label %block86576, label %block86567

edge12800:                                        ; preds = %block86576
  %v165449 = load i64, ptr %t7379, align 1
  %v165450 = add i64 %v165449, -1
  store i64 %v165450, ptr %t7379, align 1
  %t7688.not = icmp eq i64 %v165450, 0
  br i1 %t7688.not, label %edge12803, label %edge12810

edge12803:                                        ; preds = %edge12800
  %v165451 = call i64 @mech_compact_free(i64 %v0, i64 %v54948)
  %t7689.not = icmp eq i64 %v165451, 0
  br i1 %t7689.not, label %edge12810, label %block86567

edge12810:                                        ; preds = %block86576, %edge12800, %edge12803
  br i1 %t3701.not, label %block86600, label %edge12813

edge12813:                                        ; preds = %edge12810
  %v165471 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54924)
  %t7691.not = icmp eq i64 %v165471, 0
  br i1 %t7691.not, label %block86600, label %block86567

edge12817:                                        ; preds = %block86600
  %v165473 = load i64, ptr %t4451.pre-phi, align 1
  %v165474 = add i64 %v165473, -1
  store i64 %v165474, ptr %t4451.pre-phi, align 1
  %t7695.not = icmp eq i64 %v165474, 0
  br i1 %t7695.not, label %edge12820, label %edge12828

edge12820:                                        ; preds = %edge12817
  %v165475 = call i64 @mech_compact_free(i64 %v0, i64 %v58154212872129921342)
  %t7696.not = icmp eq i64 %v165475, 0
  br i1 %t7696.not, label %edge12828, label %block86567

edge12828:                                        ; preds = %edge12820, %edge12817, %block86600
  %t7697.not = icmp eq i64 %v129500, 0
  br i1 %t7697.not, label %block86628, label %edge12831

edge12831:                                        ; preds = %edge12828
  %v165499 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v129500)
  %t7698.not = icmp eq i64 %v165499, 0
  br i1 %t7698.not, label %block86628, label %block86567

edge12837:                                        ; preds = %block86628
  %v165503 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217632)
  %t7700.not = icmp eq i64 %v165503, 0
  br i1 %t7700.not, label %edge13024, label %block86567

edge12842:                                        ; preds = %edge12786
  %t7702 = inttoptr i64 %v217631 to ptr
  %v165505 = load i64, ptr %t7702, align 1
  %v165506 = add i64 %v165505, -1
  store i64 %v165506, ptr %t7702, align 1
  %t7704.not = icmp eq i64 %v165506, 0
  br i1 %t7704.not, label %edge12845, label %block2907.backedge

edge12845:                                        ; preds = %edge12842
  %v165507 = call i64 @mech_compact_free(i64 %v0, i64 %v217631)
  %t7705.not = icmp eq i64 %v165507, 0
  br i1 %t7705.not, label %block2907.backedge, label %block86567

edge13024:                                        ; preds = %block86628, %edge12837
  %t7746.not = icmp eq i64 %v7458221148, 0
  br i1 %t7746.not, label %edge13132, label %edge13122

edge13122:                                        ; preds = %edge13024
  %t7747 = inttoptr i64 %v7458221148 to ptr
  %v170488 = load i64, ptr %t7747, align 1
  %v170489 = add i64 %v170488, 1
  store i64 %v170489, ptr %t7747, align 1
  br label %edge13132

edge13132:                                        ; preds = %edge13122, %edge13024
  %v170580 = add i64 %v136868, 3
  %t7749.not = icmp ugt i64 %v136868, -4
  br i1 %t7749.not, label %common.ret, label %edge13142

edge13142:                                        ; preds = %edge13132
  %v171430 = call i64 @mech_compact_unsigned_dot_product(i64 %v80926, i64 ptrtoint (ptr @mech_compact_table_22 to i64), i64 0, i64 320)
  %v171431 = add i64 %v171430, 10
  %t7760.not = icmp eq i64 %v80920, 0
  br i1 %t7760.not, label %edge13222, label %edge13146

edge13146:                                        ; preds = %edge13142
  %v171435 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v80920)
  %t7761.not = icmp eq i64 %v171435, 0
  br i1 %t7761.not, label %edge13222, label %edge13147

edge13147:                                        ; preds = %edge13146
  %t7762 = insertvalue { i64, i64 } poison, i64 %v171435, 0
  %t7763 = insertvalue { i64, i64 } %t7762, i64 6851, 1
  br label %common.ret

edge13222:                                        ; preds = %edge13146, %edge13142
  %t7793.not = icmp ugt i64 %v54999, -65535
  br i1 %t7793.not, label %common.ret, label %edge13226

edge13226:                                        ; preds = %edge13222
  %v172864 = add nuw i64 %v54999, 65534
  %t7805.not = icmp eq i64 %v54999, 0
  br i1 %t7805.not, label %common.ret, label %edge13237

edge13237:                                        ; preds = %edge13226
  %t7822 = icmp ult i64 %v54999, 2305843009213693952
  br i1 %t7822, label %edge13245, label %common.ret

edge13245:                                        ; preds = %edge13237
  %v173026 = sub nsw i64 5, %v217712
  %v173031 = and i64 %v173026, 7
  %v172873 = udiv i64 %v172864, 65535
  %98 = mul nuw nsw i64 %v172873, 40
  %v173220 = add nsw i64 %v173031, -5
  %v173225 = add nsw i64 %v173220, %98
  %v173235 = shl nuw i64 %v54999, 3
  %v173240 = add i64 %v173225, %v173235
  %t7826.not = icmp ult i64 %v173240, %v173225
  br i1 %t7826.not, label %common.ret, label %edge13250

edge13250:                                        ; preds = %edge13245
  %spec.select41 = call i64 @llvm.umin.i64(i64 %v170580, i64 %v171431)
  %t7836.not.not = icmp ugt i64 %v173240, %spec.select41
  br i1 %t7836.not.not, label %edge13639, label %edge13293

edge13293:                                        ; preds = %edge13250
  %t7849.not = icmp eq i64 %v129484, 0
  br i1 %t7849.not, label %block88847, label %edge13297

edge13297:                                        ; preds = %edge13293
  %v174036 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v129484)
  %t7850.not = icmp eq i64 %v174036, 0
  br i1 %t7850.not, label %block88847, label %block88843

edge13303:                                        ; preds = %block88847
  %v174040 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v7458221148)
  %t7852.not = icmp eq i64 %v174040, 0
  br i1 %t7852.not, label %block88851, label %block88843

edge13309:                                        ; preds = %block88851
  %v174044 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v129506)
  %t7854.not = icmp eq i64 %v174044, 0
  br i1 %t7854.not, label %block88855, label %block88843

edge13315:                                        ; preds = %block88855
  %v174048 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v129512)
  %t7856.not = icmp eq i64 %v174048, 0
  br i1 %t7856.not, label %edge13322, label %block88843

edge13322:                                        ; preds = %block88855, %edge13315
  br i1 %t23.not, label %block6878.preheader, label %edge13323

edge13323:                                        ; preds = %edge13322
  %v175384 = load i64, ptr %t910, align 1
  %v175385 = add i64 %v175384, 1
  store i64 %v175385, ptr %t910, align 1
  br label %block6878.preheader

block6878.preheader:                              ; preds = %edge13322, %edge13323
  br label %block6878

edge13347:                                        ; preds = %block6878
  %v175475 = sub nuw i64 %v54958, %v175458
  %spec.select42 = call i64 @llvm.umin.i64(i64 %v175475, i64 65535)
  %t7869 = icmp ult i64 %v175475, 65536
  %v175841 = select i1 %t7869, i64 %v54961, i64 65536
  %v175843 = shl i64 %v175841, %v175443
  %v175848 = or i64 %v175843, %v175440
  %t7875 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v175429, i64 1)
  %v175867 = extractvalue { i64, i1 } %t7875, 0
  %t7876 = extractvalue { i64, i1 } %t7875, 1
  br i1 %t7876, label %block89285, label %edge13376

edge13376:                                        ; preds = %edge13347
  %t7879.not = icmp ugt i64 %v175867, %v175430
  br i1 %t7879.not, label %edge13379, label %block89289

edge13379:                                        ; preds = %edge13376
  %t7881.not = icmp eq i64 %v175430, 0
  br i1 %t7881.not, label %block89292, label %edge13380

edge13380:                                        ; preds = %edge13379
  %99 = add i64 %v175430, -131073
  %v175877239 = icmp ult i64 %99, -126977
  br i1 %v175877239, label %edge13383, label %edge13392

edge13383:                                        ; preds = %edge13380
  %t7885 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v175430, i64 %v175430)
  %v175878 = extractvalue { i64, i1 } %t7885, 0
  %t7886 = extractvalue { i64, i1 } %t7885, 1
  br i1 %t7886, label %block89285, label %block89292

edge13392:                                        ; preds = %edge13380
  %t7893 = mul nuw nsw i64 %v175430, 3
  %t7897 = add nuw nsw i64 %t7893, 1
  %v175895 = lshr i64 %t7897, 1
  br label %block89292

edge13396:                                        ; preds = %block89289
  %t7903 = inttoptr i64 %v175433 to ptr
  %v175903 = load i64, ptr %t7903, align 1
  %t7904 = icmp eq i64 %v175903, 1
  %t7905 = icmp ule i64 %v175872, %v175430
  %v175906240 = and i1 %t7905, %t7904
  br i1 %v175906240, label %block89299, label %block89300

edge13401:                                        ; preds = %block89300
  %v175913 = load i64, ptr %ss1, align 8
  %v175914 = load i64, ptr %t606, align 8
  %v175915 = load i64, ptr %t607, align 8
  br label %block89299

edge13403:                                        ; preds = %block89299
  %t7928 = inttoptr i64 %v175902 to ptr
  %v175922 = load i64, ptr %t7928, align 1
  %v175923 = add i64 %v175922, 1
  store i64 %v175923, ptr %t7928, align 1
  br label %block89304

edge13408:                                        ; preds = %block89304
  %v175928 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v175433)
  %t7932.not = icmp eq i64 %v175928, 0
  br i1 %t7932.not, label %edge13412, label %block89285

edge13412:                                        ; preds = %edge13408, %block89304
  %t7935 = icmp ugt i64 %v175443, 5
  br i1 %t7935, label %edge13419, label %edge13416

edge13416:                                        ; preds = %edge13412, %block89446, %edge13452
  %v176186 = phi i64 [ %v176135, %block89446 ], [ %v176135, %edge13452 ], [ %v175902, %edge13412 ]
  %v176189 = phi i64 [ %v176130, %block89446 ], [ %v176130, %edge13452 ], [ %v175897, %edge13412 ]
  %v176190 = phi i64 [ %v176100, %block89446 ], [ %v176100, %edge13452 ], [ %v175867, %edge13412 ]
  %v176191 = phi i64 [ %v176132, %block89446 ], [ %v176132, %edge13452 ], [ %v175899, %edge13412 ]
  %t7998 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v176190, i64 2)
  %v176198 = extractvalue { i64, i1 } %t7998, 0
  %t7999 = extractvalue { i64, i1 } %t7998, 1
  br i1 %t7999, label %block89463, label %edge13464

edge13419:                                        ; preds = %edge13412
  %t7938 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v175867, i64 1)
  %v176100 = extractvalue { i64, i1 } %t7938, 0
  %t7939 = extractvalue { i64, i1 } %t7938, 1
  br i1 %t7939, label %block89427, label %edge13420

edge13420:                                        ; preds = %edge13419
  %t7942.not = icmp ugt i64 %v176100, %v175899
  br i1 %t7942.not, label %edge13423, label %block89431

edge13423:                                        ; preds = %edge13420
  %t7944.not = icmp eq i64 %v175899, 0
  br i1 %t7944.not, label %block89434, label %edge13424

edge13424:                                        ; preds = %edge13423
  %100 = add i64 %v175899, -131073
  %v176110243 = icmp ult i64 %100, -126977
  br i1 %v176110243, label %edge13427, label %edge13436

edge13427:                                        ; preds = %edge13424
  %t7948 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v175899, i64 %v175899)
  %v176111 = extractvalue { i64, i1 } %t7948, 0
  %t7949 = extractvalue { i64, i1 } %t7948, 1
  br i1 %t7949, label %block89427, label %block89434

edge13436:                                        ; preds = %edge13424
  %t7956 = mul nuw nsw i64 %v175899, 3
  %t7960 = add nuw nsw i64 %t7956, 1
  %v176128 = lshr i64 %t7960, 1
  br label %block89434

edge13440:                                        ; preds = %block89431
  %t7966 = inttoptr i64 %v175902 to ptr
  %v176136 = load i64, ptr %t7966, align 1
  %t7967 = icmp eq i64 %v176136, 1
  %t7968 = icmp ule i64 %v176105, %v175899
  %v176139244 = and i1 %t7968, %t7967
  br i1 %v176139244, label %block89441, label %block89442

edge13445:                                        ; preds = %block89442
  %v176146 = load i64, ptr %ss1, align 8
  %v176147 = load i64, ptr %t606, align 8
  %v176148 = load i64, ptr %t607, align 8
  br label %block89441

edge13447:                                        ; preds = %block89441
  %t7991 = inttoptr i64 %v176135 to ptr
  %v176155 = load i64, ptr %t7991, align 1
  %v176156 = add i64 %v176155, 1
  store i64 %v176156, ptr %t7991, align 1
  br label %block89446

edge13452:                                        ; preds = %block89446
  %v176161 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v175902)
  %t7995.not = icmp eq i64 %v176161, 0
  br i1 %t7995.not, label %edge13416, label %block89427

edge13464:                                        ; preds = %edge13416
  %t8002.not = icmp ugt i64 %v176198, %v176191
  br i1 %t8002.not, label %edge13467, label %block89468

edge13467:                                        ; preds = %edge13464
  %t8004.not = icmp eq i64 %v176191, 0
  br i1 %t8004.not, label %block89471, label %edge13468

edge13468:                                        ; preds = %edge13467
  %101 = add i64 %v176191, -131073
  %v176208247 = icmp ult i64 %101, -126977
  br i1 %v176208247, label %edge13471, label %edge13480

edge13471:                                        ; preds = %edge13468
  %t8008 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v176191, i64 %v176191)
  %v176209 = extractvalue { i64, i1 } %t8008, 0
  %t8009 = extractvalue { i64, i1 } %t8008, 1
  br i1 %t8009, label %block89463, label %block89471

edge13480:                                        ; preds = %edge13468
  %t8016 = mul nuw nsw i64 %v176191, 3
  %t8020 = add nuw nsw i64 %t8016, 1
  %v176226 = lshr i64 %t8020, 1
  br label %block89471

edge13484:                                        ; preds = %block89468
  %t8026 = inttoptr i64 %v176186 to ptr
  %v176234 = load i64, ptr %t8026, align 1
  %t8027 = icmp eq i64 %v176234, 1
  %t8028 = icmp ule i64 %v176203, %v176191
  %v176237248 = and i1 %t8028, %t8027
  br i1 %v176237248, label %edge13491, label %block89479

edge13489:                                        ; preds = %block89479
  %v176244 = load i64, ptr %ss1, align 8
  %v176245 = load i64, ptr %t606, align 8
  %v176246 = load i64, ptr %t607, align 8
  br label %edge13491

edge13491:                                        ; preds = %edge13484, %edge13489
  %v176228 = phi i64 [ %v176244, %edge13489 ], [ %v176189, %edge13484 ]
  %v176230 = phi i64 [ %v176245, %edge13489 ], [ %v176191, %edge13484 ]
  %v176233 = phi i64 [ %v176246, %edge13489 ], [ %v176186, %edge13484 ]
  %v176249 = trunc i64 %spec.select42 to i8
  %v176251 = add i64 %v176228, %v176190
  %t8047 = inttoptr i64 %v176251 to ptr
  store i8 %v176249, ptr %t8047, align 1
  %v176253 = lshr i64 %spec.select42, 8
  %v176254 = trunc nuw i64 %v176253 to i8
  %v176255 = add nuw i64 %v176190, 1
  %v176256 = add i64 %v176255, %v176228
  %t8048 = inttoptr i64 %v176256 to ptr
  store i8 %v176254, ptr %t8048, align 1
  %t8049 = icmp eq i64 %v176186, %v176233
  %t8051.not249 = icmp eq i64 %v176233, 0
  %t8051.not = or i1 %t8049, %t8051.not249
  br i1 %t8051.not, label %block89499, label %edge13501

edge13501:                                        ; preds = %edge13491
  %t8052 = inttoptr i64 %v176233 to ptr
  %v176289 = load i64, ptr %t8052, align 1
  %v176290 = add i64 %v176289, 1
  store i64 %v176290, ptr %t8052, align 1
  br label %block89499

edge13506:                                        ; preds = %block89499
  %v176295 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v176186)
  %t8056.not = icmp eq i64 %v176295, 0
  br i1 %t8056.not, label %edge13510, label %block89463

edge13510:                                        ; preds = %edge13506, %block89499
  %v176462 = xor i64 %spec.select42, -1
  %t8059 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v176198, i64 2)
  %v176471 = extractvalue { i64, i1 } %t8059, 0
  %t8060 = extractvalue { i64, i1 } %t8059, 1
  br i1 %t8060, label %block89505, label %edge13514

edge13514:                                        ; preds = %edge13510
  %t8063.not = icmp ugt i64 %v176471, %v176230
  br i1 %t8063.not, label %edge13517, label %block89510

edge13517:                                        ; preds = %edge13514
  %t8065.not = icmp eq i64 %v176230, 0
  br i1 %t8065.not, label %block89513, label %edge13518

edge13518:                                        ; preds = %edge13517
  %102 = add i64 %v176230, -131073
  %v176481251 = icmp ult i64 %102, -126977
  br i1 %v176481251, label %edge13521, label %edge13530

edge13521:                                        ; preds = %edge13518
  %t8069 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v176230, i64 %v176230)
  %v176482 = extractvalue { i64, i1 } %t8069, 0
  %t8070 = extractvalue { i64, i1 } %t8069, 1
  br i1 %t8070, label %block89505, label %block89513

edge13530:                                        ; preds = %edge13518
  %t8077 = mul nuw nsw i64 %v176230, 3
  %t8081 = add nuw nsw i64 %t8077, 1
  %v176499 = lshr i64 %t8081, 1
  br label %block89513

edge13534:                                        ; preds = %block89510
  %t8087 = inttoptr i64 %v176233 to ptr
  %v176507 = load i64, ptr %t8087, align 1
  %t8088 = icmp eq i64 %v176507, 1
  %t8089 = icmp ule i64 %v176476, %v176230
  %v176510252 = and i1 %t8089, %t8088
  br i1 %v176510252, label %edge13541, label %block89521

edge13539:                                        ; preds = %block89521
  %v176517 = load i64, ptr %ss1, align 8
  %v176518 = load i64, ptr %t606, align 8
  %v176519 = load i64, ptr %t607, align 8
  br label %edge13541

edge13541:                                        ; preds = %edge13534, %edge13539
  %v176501 = phi i64 [ %v176517, %edge13539 ], [ %v176228, %edge13534 ]
  %v176503 = phi i64 [ %v176518, %edge13539 ], [ %v176230, %edge13534 ]
  %v176506 = phi i64 [ %v176519, %edge13539 ], [ %v176233, %edge13534 ]
  %v176522 = trunc i64 %v176462 to i8
  %v176524 = add i64 %v176501, %v176198
  %t8108 = inttoptr i64 %v176524 to ptr
  store i8 %v176522, ptr %t8108, align 1
  %v176526 = lshr i64 %v176462, 8
  %v176527 = trunc i64 %v176526 to i8
  %v176528 = add nuw i64 %v176198, 1
  %v176529 = add i64 %v176528, %v176501
  %t8109 = inttoptr i64 %v176529 to ptr
  store i8 %v176527, ptr %t8109, align 1
  %t8110 = icmp eq i64 %v176233, %v176506
  %t8112.not253 = icmp eq i64 %v176506, 0
  %t8112.not = or i1 %t8110, %t8112.not253
  br i1 %t8112.not, label %block89541, label %edge13551

edge13551:                                        ; preds = %edge13541
  %t8113 = inttoptr i64 %v176506 to ptr
  %v176562 = load i64, ptr %t8113, align 1
  %v176563 = add i64 %v176562, 1
  store i64 %v176563, ptr %t8113, align 1
  br label %block89541

edge13556:                                        ; preds = %block89541
  %v176568 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v176233)
  %t8117.not = icmp eq i64 %v176568, 0
  br i1 %t8117.not, label %edge13560, label %block89505

edge13560:                                        ; preds = %edge13556, %block89541
  br i1 %t1068, label %block89546, label %edge13562

edge13562:                                        ; preds = %edge13560
  %t8122 = call i64 @llvm.umin.i64(i64 %v175458, i64 %v1395)
  %v176578 = sub i64 %v1395, %t8122
  %t8123 = call i64 @llvm.umin.i64(i64 %spec.select42, i64 %v176578)
  %t8124 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v176471, i64 %t8123)
  %v176580 = extractvalue { i64, i1 } %t8124, 0
  %t8125 = extractvalue { i64, i1 } %t8124, 1
  br i1 %t8125, label %block89546, label %edge13564

edge13564:                                        ; preds = %edge13562
  %t8128.not = icmp ugt i64 %v176580, %v176503
  br i1 %t8128.not, label %edge13567, label %block89551

edge13567:                                        ; preds = %edge13564
  %t8130.not = icmp eq i64 %v176503, 0
  br i1 %t8130.not, label %block89554, label %edge13568

edge13568:                                        ; preds = %edge13567
  %103 = add i64 %v176503, -131073
  %v176590255 = icmp ult i64 %103, -126977
  br i1 %v176590255, label %edge13571, label %edge13580

edge13571:                                        ; preds = %edge13568
  %t8134 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v176503, i64 %v176503)
  %v176591 = extractvalue { i64, i1 } %t8134, 0
  %t8135 = extractvalue { i64, i1 } %t8134, 1
  br i1 %t8135, label %block89546, label %block89554

edge13580:                                        ; preds = %edge13568
  %t8142 = mul nuw nsw i64 %v176503, 3
  %t8146 = add nuw nsw i64 %t8142, 1
  %v176608 = lshr i64 %t8146, 1
  br label %block89554

edge13584:                                        ; preds = %block89551
  %t8152 = inttoptr i64 %v176506 to ptr
  %v176616 = load i64, ptr %t8152, align 1
  %t8153 = icmp eq i64 %v176616, 1
  %t8154 = icmp ule i64 %v176585, %v176503
  %v176619256 = and i1 %t8154, %t8153
  br i1 %v176619256, label %block89561, label %block89562

edge13589:                                        ; preds = %block89562
  %v176626 = load i64, ptr %ss1, align 8
  %v176627 = load i64, ptr %t606, align 8
  %v176628 = load i64, ptr %t607, align 8
  br label %block89561

edge13591:                                        ; preds = %block89561
  %v176632 = add i64 %t8122, %v1394
  %v176630 = add i64 %v176610, %v176471
  %v176634 = call i64 @memmove(i64 %v176630, i64 %v176632, i64 %t8123)
  br label %block89566

edge13594:                                        ; preds = %block89566
  %t8177 = inttoptr i64 %v176615 to ptr
  %v176637 = load i64, ptr %t8177, align 1
  %v176638 = add i64 %v176637, 1
  store i64 %v176638, ptr %t8177, align 1
  br label %block89568

edge13599:                                        ; preds = %block89568
  %v176643 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v176506)
  %t8181.not = icmp eq i64 %v176643, 0
  br i1 %t8181.not, label %edge13603, label %block89546

edge13603:                                        ; preds = %edge13599, %block89568
  %v176648 = add i64 %spec.select42, %v175458
  %t8184.not = icmp ult i64 %v176648, %v175458
  br i1 %t8184.not, label %common.ret, label %edge13606

edge13606:                                        ; preds = %edge13603
  %t8188 = icmp eq i64 %v176648, %v54958
  br i1 %t8188, label %edge13608, label %block6878

edge13608:                                        ; preds = %edge13606
  br i1 %t23.not, label %edge15273, label %edge13610

edge13610:                                        ; preds = %edge13608
  %v176663 = load i64, ptr %t910, align 1
  %v176664 = add i64 %v176663, -1
  store i64 %v176664, ptr %t910, align 1
  %t8194.not = icmp eq i64 %v176664, 0
  br i1 %t8194.not, label %edge13613, label %edge15273

edge13613:                                        ; preds = %edge13610
  %v176665 = call i64 @mech_compact_free(i64 %v0, i64 %v1399)
  %t8195.not = icmp eq i64 %v176665, 0
  br i1 %t8195.not, label %edge15273, label %edge13614

edge13614:                                        ; preds = %edge13613
  %t8196 = insertvalue { i64, i64 } poison, i64 %v176665, 0
  %t8197 = insertvalue { i64, i64 } %t8196, i64 7046, 1
  br label %common.ret

edge13639:                                        ; preds = %edge13250
  %t7839.not.not = icmp ugt i64 %v171431, %v170580
  br i1 %t7839.not.not, label %edge13750, label %edge13641

edge13641:                                        ; preds = %edge13639
  %t8201.not = icmp eq i64 %v129506, 0
  br i1 %t8201.not, label %block89751, label %edge13645

edge13645:                                        ; preds = %edge13641
  %v176902 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v129506)
  %t8202.not = icmp eq i64 %v176902, 0
  br i1 %t8202.not, label %block89751, label %block89747

edge13651:                                        ; preds = %block89751
  %v176906 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v129512)
  %t8204.not = icmp eq i64 %v176906, 0
  br i1 %t8204.not, label %edge13664, label %block89747

edge13664:                                        ; preds = %edge13651, %block89751
  %104 = or i64 %v54961, 2
  %105 = shl i64 %104, %v217712
  %v177112 = or i64 %105, %v217711
  %v177117 = add nuw nsw i64 %v217712, 3
  %v177125 = lshr i64 %v177117, 3
  %t8222 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v217706, i64 %v177125)
  %v177145 = extractvalue { i64, i1 } %t8222, 0
  %t8223 = extractvalue { i64, i1 } %t8222, 1
  br i1 %t8223, label %block89899, label %edge13679

edge13679:                                        ; preds = %edge13664
  %t8226.not = icmp ugt i64 %v177145, %v217707
  br i1 %t8226.not, label %edge13682, label %block89904

edge13682:                                        ; preds = %edge13679
  %t8228.not = icmp eq i64 %v217707, 0
  br i1 %t8228.not, label %block89907, label %edge13683

edge13683:                                        ; preds = %edge13682
  %106 = add i64 %v217707, -131073
  %v177155227 = icmp ult i64 %106, -126977
  br i1 %v177155227, label %edge13686, label %edge13695

edge13686:                                        ; preds = %edge13683
  %t8232 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v217707, i64 %v217707)
  %v177156 = extractvalue { i64, i1 } %t8232, 0
  %t8233 = extractvalue { i64, i1 } %t8232, 1
  br i1 %t8233, label %block89899, label %block89907

edge13695:                                        ; preds = %edge13683
  %t8240 = mul nuw nsw i64 %v217707, 3
  %t8244 = add nuw nsw i64 %t8240, 1
  %v177173 = lshr i64 %t8244, 1
  br label %block89907

edge13699:                                        ; preds = %block89904
  %t8250 = inttoptr i64 %v217710 to ptr
  %v177181 = load i64, ptr %t8250, align 1
  %t8251 = icmp eq i64 %v177181, 1
  %t8252 = icmp ule i64 %v177150, %v217707
  %v177184228 = and i1 %t8252, %t8251
  br i1 %v177184228, label %block89914, label %block89915

edge13704:                                        ; preds = %block89915
  %v177191 = load i64, ptr %ss1, align 8
  %v177192 = load i64, ptr %t606, align 8
  %v177193 = load i64, ptr %t607, align 8
  br label %block89914

edge13706:                                        ; preds = %block89914
  %v177196 = trunc i64 %v177112 to i8
  %v177198 = add i64 %v177175, %v217706
  %t8273 = inttoptr i64 %v177198 to ptr
  store i8 %v177196, ptr %t8273, align 1
  br label %block89933

edge13726:                                        ; preds = %block89933
  %t8292 = inttoptr i64 %v177180 to ptr
  %v177236 = load i64, ptr %t8292, align 1
  %v177237 = add i64 %v177236, 1
  store i64 %v177237, ptr %t8292, align 1
  br label %block89935

edge13731:                                        ; preds = %block89935
  %v177242 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217710)
  %t8296.not = icmp eq i64 %v177242, 0
  br i1 %t8296.not, label %edge13656.thread, label %block89899

edge13656.thread:                                 ; preds = %block89935, %edge13731
  %v177248 = and i64 %v177117, -8
  %v177259 = lshr i64 %v177112, %v177248
  %v177264 = and i64 %v177117, 7
  %t8693.not = icmp eq i64 %v129484, 0
  br i1 %t8693.not, label %edge14397, label %edge14382

edge13750:                                        ; preds = %edge13639
  %t8311 = icmp ugt i64 %v129514, 256
  br i1 %t8311, label %edge13758, label %common.ret

edge13758:                                        ; preds = %edge13750
  %t8320.not = icmp eq i64 %v129515, 0
  br i1 %t8320.not, label %common.ret, label %edge13768

edge13768:                                        ; preds = %edge13758
  %t8329 = icmp ugt i64 %v129516, 3
  br i1 %t8329, label %edge13832, label %common.ret

edge13832:                                        ; preds = %edge13768
  %v177393 = add nuw nsw i64 %v217712, 13
  %107 = or i64 %v54961, 4
  %108 = shl i64 %107, %v217712
  %v177340 = add i64 %v129514, -257
  %v177329 = add nuw nsw i64 %v217712, 3
  %v177351 = shl i64 %v177340, %v177329
  %v177372 = add i64 %v129515, -1
  %v177361 = add nuw nsw i64 %v217712, 8
  %v177383 = shl i64 %v177372, %v177361
  %v177404 = add i64 %v129516, -4
  %v177415 = shl i64 %v177404, %v177393
  %v177425 = add nuw nsw i64 %v217712, 17
  %v178214 = add i64 %v129501, 64
  %t8343 = inttoptr i64 %v178214 to ptr
  %t8344 = load i32, ptr %t8343, align 1
  %109 = lshr i32 %t8344, 16
  %110 = zext nneg i32 %109 to i64
  %v178256 = shl nuw nsw i64 %110, %v177425
  %111 = or i64 %108, %v177351
  %112 = or i64 %111, %v177383
  %113 = or i64 %112, %v177415
  %114 = or i64 %113, %v178256
  %v178261 = or i64 %114, %v217711
  %v178266 = add nuw nsw i64 %v217712, 20
  %v178274 = lshr i64 %v178266, 3
  %t8356 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v217706, i64 %v178274)
  %v178294 = extractvalue { i64, i1 } %t8356, 0
  %t8357 = extractvalue { i64, i1 } %t8356, 1
  br i1 %t8357, label %block90323, label %edge13848

edge13848:                                        ; preds = %edge13832
  %t8360.not = icmp ugt i64 %v178294, %v217707
  br i1 %t8360.not, label %edge13851, label %block90328

edge13851:                                        ; preds = %edge13848
  %t8362.not = icmp eq i64 %v217707, 0
  br i1 %t8362.not, label %block90331, label %edge13852

edge13852:                                        ; preds = %edge13851
  %115 = add i64 %v217707, -131073
  %v178304215 = icmp ult i64 %115, -126977
  br i1 %v178304215, label %edge13855, label %edge13864

edge13855:                                        ; preds = %edge13852
  %t8366 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v217707, i64 %v217707)
  %v178305 = extractvalue { i64, i1 } %t8366, 0
  %t8367 = extractvalue { i64, i1 } %t8366, 1
  br i1 %t8367, label %block90323, label %block90331

edge13864:                                        ; preds = %edge13852
  %t8374 = mul nuw nsw i64 %v217707, 3
  %t8378 = add nuw nsw i64 %t8374, 1
  %v178322 = lshr i64 %t8378, 1
  br label %block90331

edge13868:                                        ; preds = %block90328
  %t8384 = inttoptr i64 %v217710 to ptr
  %v178330 = load i64, ptr %t8384, align 1
  %t8385 = icmp eq i64 %v178330, 1
  %t8386 = icmp ule i64 %v178299, %v217707
  %v178333216 = and i1 %t8386, %t8385
  br i1 %v178333216, label %edge13875, label %block90339

edge13873:                                        ; preds = %block90339
  %v178340 = load i64, ptr %ss1, align 8
  %v178341 = load i64, ptr %t606, align 8
  %v178342 = load i64, ptr %t607, align 8
  br label %edge13875

edge13875:                                        ; preds = %edge13873, %edge13868
  %v178324 = phi i64 [ %v178340, %edge13873 ], [ %v217705, %edge13868 ]
  %v178326 = phi i64 [ %v178341, %edge13873 ], [ %v217707, %edge13868 ]
  %v178329 = phi i64 [ %v178342, %edge13873 ], [ %v217710, %edge13868 ]
  %v178345 = trunc i64 %v178261 to i8
  %v178347 = add i64 %v178324, %v217706
  %t8407 = inttoptr i64 %v178347 to ptr
  store i8 %v178345, ptr %t8407, align 1
  %v178349 = lshr i64 %v178261, 8
  %v178350 = trunc i64 %v178349 to i8
  %v178351 = add i64 %v217706, 1
  %v178352 = add i64 %v178351, %v178324
  %t8410 = inttoptr i64 %v178352 to ptr
  store i8 %v178350, ptr %t8410, align 1
  %t8411 = icmp ugt i64 %v217712, 3
  br i1 %t8411, label %edge13881, label %block90357

edge13881:                                        ; preds = %edge13875
  %v178354 = lshr i64 %v178261, 16
  %v178355 = trunc i64 %v178354 to i8
  %v178356 = add i64 %v217706, 2
  %v178357 = add i64 %v178356, %v178324
  %t8413 = inttoptr i64 %v178357 to ptr
  store i8 %v178355, ptr %t8413, align 1
  br label %block90357

edge13899:                                        ; preds = %block90357
  %t8432 = inttoptr i64 %v178329 to ptr
  %v178385 = load i64, ptr %t8432, align 1
  %v178386 = add i64 %v178385, 1
  store i64 %v178386, ptr %t8432, align 1
  br label %block90359

edge13904:                                        ; preds = %block90359
  %v178391 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217710)
  %t8436.not = icmp eq i64 %v178391, 0
  br i1 %t8436.not, label %edge13908, label %block90323

edge13908:                                        ; preds = %edge13904, %block90359
  %v178397 = and i64 %v178266, -8
  %v178408 = lshr i64 %v178261, %v178397
  %v178413 = and i64 %v178266, 7
  %t8441.not = icmp eq i64 %v129506, 0
  br i1 %t8441.not, label %edge13927, label %edge13915

edge13915:                                        ; preds = %edge13908
  %t8442 = inttoptr i64 %v129506 to ptr
  %v178422 = load i64, ptr %t8442, align 1
  %v178423 = add i64 %v178422, 1
  store i64 %v178423, ptr %t8442, align 1
  br label %edge13927

edge13927:                                        ; preds = %edge13915, %edge13908
  %116 = sub nuw nsw i64 -4, %v178413
  %117 = udiv i64 %116, 3
  %invariant.umin = call i64 @llvm.umin.i64(i64 %v129516, i64 19)
  %118 = add nuw nsw i64 %117, 2
  br label %block7059

edge13964:                                        ; preds = %block7059
  %v178931 = add i64 %v178503, ptrtoint (ptr @mech_compact_table_15 to i64)
  %t8450 = inttoptr i64 %v178931 to ptr
  %t8451 = load i8, ptr %t8450, align 1
  %v178932 = zext i8 %t8451 to i64
  br label %edge13976

edge13976:                                        ; preds = %block7059, %edge13964
  %v178971 = phi i64 [ 0, %block7059 ], [ %v178932, %edge13964 ]
  %exitcond20628.not = icmp eq i64 %v178503, %v129516
  br i1 %exitcond20628.not, label %edge14015, label %edge13989

edge13989:                                        ; preds = %edge13976
  %t8454 = icmp ult i64 %v178971, %v129502
  br i1 %t8454, label %edge13997, label %edge14027

edge13997:                                        ; preds = %edge13989
  %v179380 = shl nuw nsw i64 %v178971, 2
  %v179381 = add i64 %v179380, %v129501
  %t8457 = inttoptr i64 %v179381 to ptr
  %t8458 = load i32, ptr %t8457, align 1
  %119 = lshr i32 %t8458, 16
  %120 = zext nneg i32 %119 to i64
  br label %edge14027

edge14015:                                        ; preds = %edge13976
  br i1 %t8441.not, label %edge14047, label %edge14016

edge14016:                                        ; preds = %edge14015
  %t8464 = inttoptr i64 %v129506 to ptr
  %v179453 = load i64, ptr %t8464, align 1
  %v179454 = add i64 %v179453, -1
  store i64 %v179454, ptr %t8464, align 1
  %t8466.not = icmp eq i64 %v179454, 0
  br i1 %t8466.not, label %edge14019, label %edge14047

edge14019:                                        ; preds = %edge14016
  %v179455 = call i64 @mech_compact_free(i64 %v0, i64 %v129506)
  %t8467.not = icmp eq i64 %v179455, 0
  br i1 %t8467.not, label %edge14047, label %edge14020

edge14020:                                        ; preds = %edge14019
  %t8468 = insertvalue { i64, i64 } poison, i64 %v179455, 0
  %t8469 = insertvalue { i64, i64 } %t8468, i64 7251, 1
  br label %common.ret

edge14027:                                        ; preds = %edge13989, %edge13997
  %v179403 = phi i64 [ %120, %edge13997 ], [ 0, %edge13989 ]
  %v179440 = and i64 %v178509, 63
  %v179441 = shl i64 %v179403, %v179440
  %v179446 = or i64 %v179441, %v178506
  %v179631 = add nuw nsw i64 %v178503, 1
  %v179639 = add nuw i64 %v178509, 3
  %exitcond20629 = icmp eq i64 %v178503, %118
  br i1 %exitcond20629, label %common.ret, label %block7059

edge14047:                                        ; preds = %edge14015, %edge14016, %edge14019
  %v179700 = lshr i64 %v178509, 3
  %v179705 = and i64 %v179700, 255
  %t8476 = icmp samesign ult i64 %v179705, 9
  br i1 %t8476, label %edge14053, label %block91220

edge14053:                                        ; preds = %edge14047
  %t8478 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v178294, i64 %v179705)
  %v179720 = extractvalue { i64, i1 } %t8478, 0
  %t8479 = extractvalue { i64, i1 } %t8478, 1
  br i1 %t8479, label %block91220, label %edge14055

edge14055:                                        ; preds = %edge14053
  %t8482.not = icmp ugt i64 %v179720, %v178326
  br i1 %t8482.not, label %edge14058, label %block91225

edge14058:                                        ; preds = %edge14055
  %t8484.not = icmp eq i64 %v178326, 0
  br i1 %t8484.not, label %block91228, label %edge14059

edge14059:                                        ; preds = %edge14058
  %121 = add i64 %v178326, -131073
  %v179730219 = icmp ult i64 %121, -126977
  br i1 %v179730219, label %edge14062, label %edge14071

edge14062:                                        ; preds = %edge14059
  %t8488 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v178326, i64 %v178326)
  %v179731 = extractvalue { i64, i1 } %t8488, 0
  %t8489 = extractvalue { i64, i1 } %t8488, 1
  br i1 %t8489, label %block91220, label %block91228

edge14071:                                        ; preds = %edge14059
  %t8496 = mul nuw nsw i64 %v178326, 3
  %t8500 = add nuw nsw i64 %t8496, 1
  %v179748 = lshr i64 %t8500, 1
  br label %block91228

edge14075:                                        ; preds = %block91225
  %t8506 = inttoptr i64 %v178329 to ptr
  %v179756 = load i64, ptr %t8506, align 1
  %t8507 = icmp eq i64 %v179756, 1
  %t8508 = icmp ule i64 %v179725, %v178326
  %v179759220 = and i1 %t8508, %t8507
  br i1 %v179759220, label %block91235, label %block91236

edge14080:                                        ; preds = %block91236
  %v179766 = load i64, ptr %ss1, align 8
  %v179767 = load i64, ptr %t606, align 8
  %v179768 = load i64, ptr %t607, align 8
  br label %block91235

edge14082:                                        ; preds = %block91235
  %v179771 = trunc i64 %v178506 to i8
  %v179773 = add i64 %v179750, %v178294
  %t8529 = inttoptr i64 %v179773 to ptr
  store i8 %v179771, ptr %t8529, align 1
  %t8530.not = icmp eq i64 %v179705, 1
  br i1 %t8530.not, label %block91254, label %edge14085

edge14085:                                        ; preds = %edge14082
  %v179775 = lshr i64 %v178506, 8
  %v179776 = trunc i64 %v179775 to i8
  %v179777 = add i64 %v178294, 1
  %v179778 = add i64 %v179777, %v179750
  %t8532 = inttoptr i64 %v179778 to ptr
  store i8 %v179776, ptr %t8532, align 1
  %t8533 = icmp samesign ugt i64 %v179705, 2
  br i1 %t8533, label %edge14088, label %block91254

edge14088:                                        ; preds = %edge14085
  %v179780 = lshr i64 %v178506, 16
  %v179781 = trunc i64 %v179780 to i8
  %v179782 = add i64 %v178294, 2
  %v179783 = add i64 %v179782, %v179750
  %t8535 = inttoptr i64 %v179783 to ptr
  store i8 %v179781, ptr %t8535, align 1
  %t8536.not = icmp eq i64 %v179705, 3
  br i1 %t8536.not, label %block91254, label %edge14091

edge14091:                                        ; preds = %edge14088
  %v179785 = lshr i64 %v178506, 24
  %v179786 = trunc i64 %v179785 to i8
  %v179787 = add i64 %v178294, 3
  %v179788 = add i64 %v179787, %v179750
  %t8538 = inttoptr i64 %v179788 to ptr
  store i8 %v179786, ptr %t8538, align 1
  %t8539 = icmp samesign ugt i64 %v179705, 4
  br i1 %t8539, label %edge14094, label %block91254

edge14094:                                        ; preds = %edge14091
  %v179790 = lshr i64 %v178506, 32
  %v179791 = trunc i64 %v179790 to i8
  %v179792 = add i64 %v178294, 4
  %v179793 = add i64 %v179792, %v179750
  %t8541 = inttoptr i64 %v179793 to ptr
  store i8 %v179791, ptr %t8541, align 1
  %t8542.not = icmp eq i64 %v179705, 5
  br i1 %t8542.not, label %block91254, label %edge14097

edge14097:                                        ; preds = %edge14094
  %v179795 = lshr i64 %v178506, 40
  %v179796 = trunc i64 %v179795 to i8
  %v179797 = add i64 %v178294, 5
  %v179798 = add i64 %v179797, %v179750
  %t8544 = inttoptr i64 %v179798 to ptr
  store i8 %v179796, ptr %t8544, align 1
  %t8545 = icmp samesign ugt i64 %v179705, 6
  br i1 %t8545, label %edge14100, label %block91254

edge14100:                                        ; preds = %edge14097
  %v179800 = lshr i64 %v178506, 48
  %v179801 = trunc i64 %v179800 to i8
  %v179802 = add i64 %v178294, 6
  %v179803 = add i64 %v179802, %v179750
  %t8547 = inttoptr i64 %v179803 to ptr
  store i8 %v179801, ptr %t8547, align 1
  %t8548 = icmp eq i64 %v179705, 8
  br i1 %t8548, label %edge14103, label %block91254

edge14103:                                        ; preds = %edge14100
  %v179805 = lshr i64 %v178506, 56
  %v179806 = trunc nuw i64 %v179805 to i8
  %v179807 = add i64 %v178294, 7
  %v179808 = add i64 %v179807, %v179750
  %t8550 = inttoptr i64 %v179808 to ptr
  store i8 %v179806, ptr %t8550, align 1
  br label %block91254

edge14106:                                        ; preds = %block91254
  %t8554 = inttoptr i64 %v179755 to ptr
  %v179811 = load i64, ptr %t8554, align 1
  %v179812 = add i64 %v179811, 1
  store i64 %v179812, ptr %t8554, align 1
  br label %block91256

edge14111:                                        ; preds = %block91256
  %v179817 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v178329)
  %t8558.not = icmp eq i64 %v179817, 0
  br i1 %t8558.not, label %edge14115, label %block91220

edge14115:                                        ; preds = %edge14111, %block91256
  %v179833 = and i64 %v178509, 56
  %v179834 = lshr i64 %v178506, %v179833
  br label %block7165

edge14164:                                        ; preds = %block7165
  %t8681.not = icmp eq i64 %v129512, 0
  br i1 %t8681.not, label %block92077, label %edge14337

edge14174:                                        ; preds = %block7165
  %t8566 = icmp ult i64 %v180337, 320
  br i1 %t8566, label %edge14182, label %edge14191

edge14182:                                        ; preds = %edge14174
  %v180748 = shl nuw nsw i64 %v180337, 2
  %v180749 = add i64 %v180748, %v129507
  %t8569 = inttoptr i64 %v180749 to ptr
  %t8570 = load i32, ptr %t8569, align 1
  %v180750 = zext i32 %t8570 to i64
  br label %edge14191

edge14191:                                        ; preds = %edge14182, %edge14174
  %v180771 = phi i64 [ %v180750, %edge14182 ], [ 0, %edge14174 ]
  %v180942 = and i64 %v180771, 31
  %t8571 = icmp ult i64 %v180942, %v129502
  br i1 %t8571, label %edge14208, label %edge14217

edge14208:                                        ; preds = %edge14191
  %v181164 = shl nuw nsw i64 %v180942, 2
  %v181165 = add i64 %v181164, %v129501
  %t8574 = inttoptr i64 %v181165 to ptr
  %t8575 = load i32, ptr %t8574, align 1
  %v181166 = zext i32 %t8575 to i64
  br label %edge14217

edge14217:                                        ; preds = %edge14208, %edge14191
  %v181186 = phi i64 [ %v181166, %edge14208 ], [ 0, %edge14191 ]
  %v181221 = lshr i64 %v181186, 16
  %v181226 = add nuw nsw i64 %v181221, %v180295
  %v181197 = and i64 %v181186, 65535
  %v181209 = shl nuw nsw i64 %v181197, %v180295
  %v181236 = lshr i64 %v180771, 5
  %v181246 = and i64 %v181226, 63
  %v181247 = shl i64 %v181236, %v181246
  %122 = or i64 %v181209, %v181247
  %v181252 = or i64 %122, %v180292
  %t8584 = icmp samesign ult i64 %v180942, 19
  br i1 %t8584, label %edge14245, label %edge14254

edge14245:                                        ; preds = %edge14217
  %v181475 = add i64 %v180942, ptrtoint (ptr @mech_compact_table_14 to i64)
  %t8587 = inttoptr i64 %v181475 to ptr
  %t8588 = load i8, ptr %t8587, align 1
  %v181476 = zext i8 %t8588 to i64
  br label %edge14254

edge14254:                                        ; preds = %edge14245, %edge14217
  %v181497 = phi i64 [ %v181476, %edge14245 ], [ 0, %edge14217 ]
  %v181506 = add nuw nsw i64 %v181497, %v181226
  %v181515 = lshr i64 %v181506, 3
  %v181520 = and i64 %v181515, 255
  %t8594 = icmp samesign ult i64 %v181520, 9
  br i1 %t8594, label %edge14263, label %block92006

edge14263:                                        ; preds = %edge14254
  %t8596 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v180281, i64 %v181520)
  %v181535 = extractvalue { i64, i1 } %t8596, 0
  %t8597 = extractvalue { i64, i1 } %t8596, 1
  br i1 %t8597, label %block92006, label %edge14265

edge14265:                                        ; preds = %edge14263
  %t8600.not = icmp ugt i64 %v181535, %v180282
  br i1 %t8600.not, label %edge14268, label %block92011

edge14268:                                        ; preds = %edge14265
  %t8602.not = icmp eq i64 %v180282, 0
  br i1 %t8602.not, label %block92014, label %edge14269

edge14269:                                        ; preds = %edge14268
  %123 = add i64 %v180282, -131073
  %v181545223 = icmp ult i64 %123, -126977
  br i1 %v181545223, label %edge14272, label %edge14281

edge14272:                                        ; preds = %edge14269
  %t8606 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v180282, i64 %v180282)
  %v181546 = extractvalue { i64, i1 } %t8606, 0
  %t8607 = extractvalue { i64, i1 } %t8606, 1
  br i1 %t8607, label %block92006, label %block92014

edge14281:                                        ; preds = %edge14269
  %t8614 = mul nuw nsw i64 %v180282, 3
  %t8618 = add nuw nsw i64 %t8614, 1
  %v181563 = lshr i64 %t8618, 1
  br label %block92014

edge14285:                                        ; preds = %block92011
  %t8624 = inttoptr i64 %v180285 to ptr
  %v181571 = load i64, ptr %t8624, align 1
  %t8625 = icmp eq i64 %v181571, 1
  %t8626 = icmp ule i64 %v181540, %v180282
  %v181574224 = and i1 %t8626, %t8625
  br i1 %v181574224, label %block92021, label %block92022

edge14290:                                        ; preds = %block92022
  %v181581 = load i64, ptr %ss1, align 8
  %v181582 = load i64, ptr %t606, align 8
  %v181583 = load i64, ptr %t607, align 8
  br label %block92021

edge14292:                                        ; preds = %block92021
  %v181586 = trunc i64 %v181252 to i8
  %v181588 = add i64 %v181565, %v180281
  %t8647 = inttoptr i64 %v181588 to ptr
  store i8 %v181586, ptr %t8647, align 1
  %t8648.not = icmp eq i64 %v181520, 1
  br i1 %t8648.not, label %block92040, label %edge14295

edge14295:                                        ; preds = %edge14292
  %v181590 = lshr i64 %v181252, 8
  %v181591 = trunc i64 %v181590 to i8
  %v181592 = add i64 %v180281, 1
  %v181593 = add i64 %v181592, %v181565
  %t8650 = inttoptr i64 %v181593 to ptr
  store i8 %v181591, ptr %t8650, align 1
  %t8651 = icmp samesign ugt i64 %v181520, 2
  br i1 %t8651, label %edge14298, label %block92040

edge14298:                                        ; preds = %edge14295
  %v181595 = lshr i64 %v181252, 16
  %v181596 = trunc i64 %v181595 to i8
  %v181597 = add i64 %v180281, 2
  %v181598 = add i64 %v181597, %v181565
  %t8653 = inttoptr i64 %v181598 to ptr
  store i8 %v181596, ptr %t8653, align 1
  %t8654.not = icmp eq i64 %v181520, 3
  br i1 %t8654.not, label %block92040, label %edge14301

edge14301:                                        ; preds = %edge14298
  %v181600 = lshr i64 %v181252, 24
  %v181601 = trunc i64 %v181600 to i8
  %v181602 = add i64 %v180281, 3
  %v181603 = add i64 %v181602, %v181565
  %t8656 = inttoptr i64 %v181603 to ptr
  store i8 %v181601, ptr %t8656, align 1
  %t8657 = icmp samesign ugt i64 %v181520, 4
  br i1 %t8657, label %edge14304, label %block92040

edge14304:                                        ; preds = %edge14301
  %v181605 = lshr i64 %v181252, 32
  %v181606 = trunc i64 %v181605 to i8
  %v181607 = add i64 %v180281, 4
  %v181608 = add i64 %v181607, %v181565
  %t8659 = inttoptr i64 %v181608 to ptr
  store i8 %v181606, ptr %t8659, align 1
  %t8660.not = icmp eq i64 %v181520, 5
  br i1 %t8660.not, label %block92040, label %edge14307

edge14307:                                        ; preds = %edge14304
  %v181610 = lshr i64 %v181252, 40
  %v181611 = trunc i64 %v181610 to i8
  %v181612 = add i64 %v180281, 5
  %v181613 = add i64 %v181612, %v181565
  %t8662 = inttoptr i64 %v181613 to ptr
  store i8 %v181611, ptr %t8662, align 1
  %t8663 = icmp samesign ugt i64 %v181520, 6
  br i1 %t8663, label %edge14310, label %block92040

edge14310:                                        ; preds = %edge14307
  %v181615 = lshr i64 %v181252, 48
  %v181616 = trunc i64 %v181615 to i8
  %v181617 = add i64 %v180281, 6
  %v181618 = add i64 %v181617, %v181565
  %t8665 = inttoptr i64 %v181618 to ptr
  store i8 %v181616, ptr %t8665, align 1
  %t8666 = icmp eq i64 %v181520, 8
  br i1 %t8666, label %edge14313, label %block92040

edge14313:                                        ; preds = %edge14310
  %v181620 = lshr i64 %v181252, 56
  %v181621 = trunc nuw i64 %v181620 to i8
  %v181622 = add i64 %v180281, 7
  %v181623 = add i64 %v181622, %v181565
  %t8668 = inttoptr i64 %v181623 to ptr
  store i8 %v181621, ptr %t8668, align 1
  br label %block92040

edge14316:                                        ; preds = %block92040
  %t8672 = inttoptr i64 %v181570 to ptr
  %v181626 = load i64, ptr %t8672, align 1
  %v181627 = add i64 %v181626, 1
  store i64 %v181627, ptr %t8672, align 1
  br label %block92042

edge14321:                                        ; preds = %block92042
  %v181632 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v180285)
  %t8676.not = icmp eq i64 %v181632, 0
  br i1 %t8676.not, label %edge14325, label %block92006

edge14325:                                        ; preds = %edge14321, %block92042
  %v181648 = and i64 %v181506, 56
  %v181649 = lshr i64 %v181252, %v181648
  %v181885 = add i64 %v180337, 1
  br label %block7165

edge14337:                                        ; preds = %edge14164
  %v181697 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v129512)
  %t8682.not = icmp eq i64 %v181697, 0
  br i1 %t8682.not, label %block92077, label %block92073

edge14341:                                        ; preds = %block92077
  %t8684 = inttoptr i64 %v129506 to ptr
  %v181699 = load i64, ptr %t8684, align 1
  %v181700 = add i64 %v181699, -1
  store i64 %v181700, ptr %t8684, align 1
  %t8686.not = icmp eq i64 %v181700, 0
  br i1 %t8686.not, label %edge14344, label %edge14397

edge14344:                                        ; preds = %edge14341
  %v181701 = call i64 @mech_compact_free(i64 %v0, i64 %v129506)
  %t8687.not = icmp eq i64 %v181701, 0
  br i1 %t8687.not, label %edge14397, label %block92073

edge14382:                                        ; preds = %edge13656.thread
  %v181969 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v129484)
  %t8694.not = icmp eq i64 %v181969, 0
  br i1 %t8694.not, label %edge14397, label %edge14383

edge14383:                                        ; preds = %edge14382
  %t8695 = insertvalue { i64, i64 } poison, i64 %v181969, 0
  %t8696 = insertvalue { i64, i64 } %t8695, i64 7409, 1
  br label %common.ret

edge14397:                                        ; preds = %block92077, %edge14341, %edge14344, %edge14382, %edge13656.thread
  %v1829151061 = phi i64 [ %v177264, %edge14382 ], [ %v177264, %edge13656.thread ], [ %v180295, %edge14344 ], [ %v180295, %edge14341 ], [ %v180295, %block92077 ]
  %v1829121059 = phi i64 [ %v177259, %edge14382 ], [ %v177259, %edge13656.thread ], [ %v180292, %edge14344 ], [ %v180292, %edge14341 ], [ %v180292, %block92077 ]
  %v1829051057 = phi i64 [ %v177180, %edge14382 ], [ %v177180, %edge13656.thread ], [ %v180285, %edge14344 ], [ %v180285, %edge14341 ], [ %v180285, %block92077 ]
  %v1829021055 = phi i64 [ %v177177, %edge14382 ], [ %v177177, %edge13656.thread ], [ %v180282, %edge14344 ], [ %v180282, %edge14341 ], [ %v180282, %block92077 ]
  %v1829011053 = phi i64 [ %v177145, %edge14382 ], [ %v177145, %edge13656.thread ], [ %v180281, %edge14344 ], [ %v180281, %edge14341 ], [ %v180281, %block92077 ]
  %v1829001051 = phi i64 [ %v177175, %edge14382 ], [ %v177175, %edge13656.thread ], [ %v180280, %edge14344 ], [ %v180280, %edge14341 ], [ %v180280, %block92077 ]
  %v182968 = phi i64 [ ptrtoint (ptr @mech_compact_table_9 to i64), %edge14382 ], [ ptrtoint (ptr @mech_compact_table_9 to i64), %edge13656.thread ], [ %v129479, %edge14344 ], [ %v129479, %edge14341 ], [ %v129479, %block92077 ]
  %v182969 = phi i64 [ 320, %edge14382 ], [ 320, %edge13656.thread ], [ %v129480, %edge14344 ], [ %v129480, %edge14341 ], [ %v129480, %block92077 ]
  %v182973 = phi i64 [ 0, %edge14382 ], [ 0, %edge13656.thread ], [ %v129484, %edge14344 ], [ %v129484, %edge14341 ], [ %v129484, %block92077 ]
  br i1 %t23.not, label %block92420, label %edge14404

edge14404:                                        ; preds = %edge14397
  %v184130 = load i64, ptr %t910, align 1
  %v184131 = add i64 %v184130, 1
  store i64 %v184131, ptr %t910, align 1
  br label %block92420

edge14407:                                        ; preds = %block92420
  %t8705 = inttoptr i64 %v182973 to ptr
  %v184132 = load i64, ptr %t8705, align 1
  %v184133 = add i64 %v184132, 1
  store i64 %v184133, ptr %t8705, align 1
  br label %edge14421

edge14421:                                        ; preds = %block92420, %edge14407
  %t8761.pre-phi = phi ptr [ %t8705, %edge14407 ], [ null, %block92420 ]
  br label %block7301

edge14457:                                        ; preds = %block7301
  br i1 %t3962, label %edge14461, label %edge14459

edge14459:                                        ; preds = %edge14457
  %124 = xor i64 %v184267, -1
  %v188320 = add i64 %v7457821133, %124
  %t8796 = icmp ult i64 %v188320, %v7457821133
  br i1 %t8796, label %edge14791, label %edge14847

edge14461:                                        ; preds = %edge14457
  %v184750 = add i64 %v184267, %v54957
  %v184755 = sub i64 %v54999, %v184267
  %t8712 = icmp ugt i64 %v184755, 3
  br i1 %t8712, label %edge14472, label %edge14626

edge14472:                                        ; preds = %edge14461
  %t8715.not = icmp ugt i64 %v184750, %v14023
  br i1 %t8715.not, label %edge14493, label %edge14480

edge14480:                                        ; preds = %edge14472
  br i1 %v1061851.not1217, label %edge14485, label %edge14485.thread

edge14485.thread:                                 ; preds = %edge14480
  %v185340 = add i64 %v184750, %v1394
  %t8721 = inttoptr i64 %v185340 to ptr
  %t8722 = load i32, ptr %t8721, align 1
  %v185341 = zext i32 %t8722 to i64
  br label %edge14493

edge14485:                                        ; preds = %edge14480
  %v185343 = call i64 @mech_compact_matrix_word(i64 %v1394, i64 %v184750, i64 %v1397, i64 %v1398, i64 4)
  %t8723 = icmp ult i64 %v185343, 4294967296
  br i1 %t8723, label %edge14493, label %common.ret

edge14493:                                        ; preds = %edge14485.thread, %edge14485, %edge14472
  %v185542 = phi i64 [ %v185343, %edge14485 ], [ 0, %edge14472 ], [ %v185341, %edge14485.thread ]
  %v185569 = and i64 %v185542, 255
  %t8727 = icmp ult i64 %v185569, %v182969
  br i1 %t8727, label %edge14510, label %edge14519

edge14510:                                        ; preds = %edge14493
  %v185828 = shl nuw nsw i64 %v185569, 2
  %v185829 = add i64 %v185828, %v182968
  %t8730 = inttoptr i64 %v185829 to ptr
  %t8731 = load i32, ptr %t8730, align 1
  %v185830 = zext i32 %t8731 to i64
  br label %edge14519

edge14519:                                        ; preds = %edge14510, %edge14493
  %v185854 = phi i64 [ %v185830, %edge14510 ], [ 0, %edge14493 ]
  %v185869 = and i64 %v185854, 65535
  %v185880 = lshr i64 %v185854, 16
  %v185893 = lshr i64 %v185542, 8
  %v185898 = and i64 %v185893, 255
  %t8734 = icmp ult i64 %v185898, %v182969
  br i1 %t8734, label %edge14539, label %edge14548

edge14539:                                        ; preds = %edge14519
  %v186125 = shl nuw nsw i64 %v185898, 2
  %v186126 = add i64 %v186125, %v182968
  %t8737 = inttoptr i64 %v186126 to ptr
  %t8738 = load i32, ptr %t8737, align 1
  %v186127 = zext i32 %t8738 to i64
  br label %edge14548

edge14548:                                        ; preds = %edge14539, %edge14519
  %v186147 = phi i64 [ %v186127, %edge14539 ], [ 0, %edge14519 ]
  %v186156 = and i64 %v186147, 65535
  %v186166 = and i64 %v185880, 63
  %v186167 = shl i64 %v186156, %v186166
  %v186172 = or i64 %v186167, %v185869
  %v186178 = lshr i64 %v186147, 16
  %v186183 = add nuw nsw i64 %v186178, %v185880
  %v186189 = lshr i64 %v185542, 16
  %v186194 = and i64 %v186189, 255
  %t8742 = icmp ult i64 %v186194, %v182969
  br i1 %t8742, label %edge14572, label %edge14581

edge14572:                                        ; preds = %edge14548
  %v186421 = shl nuw nsw i64 %v186194, 2
  %v186422 = add i64 %v186421, %v182968
  %t8745 = inttoptr i64 %v186422 to ptr
  %t8746 = load i32, ptr %t8745, align 1
  %v186423 = zext i32 %t8746 to i64
  br label %edge14581

edge14581:                                        ; preds = %edge14572, %edge14548
  %v186443 = phi i64 [ %v186423, %edge14572 ], [ 0, %edge14548 ]
  %v186452 = and i64 %v186443, 65535
  %v186462 = and i64 %v186183, 63
  %v186463 = shl i64 %v186452, %v186462
  %v186468 = or i64 %v186172, %v186463
  %v186474 = lshr i64 %v186443, 16
  %v186479 = add nuw nsw i64 %v186474, %v186183
  %v186653 = lshr i64 %v185542, 24
  %t8750 = icmp ult i64 %v186653, %v182969
  br i1 %t8750, label %edge14604, label %edge14998.thread1094

edge14604:                                        ; preds = %edge14581
  %v186880 = shl nuw nsw i64 %v186653, 2
  %v186881 = add i64 %v186880, %v182968
  %t8753 = inttoptr i64 %v186881 to ptr
  %t8754 = load i32, ptr %t8753, align 1
  %v186882 = zext i32 %t8754 to i64
  br label %edge14998.thread1094

edge14998.thread1094:                             ; preds = %edge14581, %edge14604
  %v186902 = phi i64 [ %v186882, %edge14604 ], [ 0, %edge14581 ]
  %v186911 = and i64 %v186902, 65535
  %v186921 = and i64 %v186479, 63
  %v186922 = shl i64 %v186911, %v186921
  %v186927 = or i64 %v186468, %v186922
  %v186933 = lshr i64 %v186902, 16
  %v186938 = add nuw nsw i64 %v186933, %v186479
  br label %edge15005

edge14626:                                        ; preds = %edge14461
  br i1 %t23.not, label %block93979, label %edge14627

edge14627:                                        ; preds = %edge14626
  %v186973 = load i64, ptr %t910, align 1
  %v186974 = add i64 %v186973, 1
  store i64 %v186974, ptr %t910, align 1
  br label %block93979

edge14630:                                        ; preds = %block93979
  %v186975 = load i64, ptr %t8761.pre-phi, align 1
  %v186976 = add i64 %v186975, 1
  store i64 %v186976, ptr %t8761.pre-phi, align 1
  br label %block7494.preheader

edge14673:                                        ; preds = %block7494
  br i1 %v1061851.not1217, label %edge14677, label %block94330

edge14677:                                        ; preds = %edge14673
  br i1 %t975, label %trap8773, label %ok8773

edge14688:                                        ; preds = %block7494, %block94330
  %v187542 = phi i64 [ 0, %block7494 ], [ %v187501, %block94330 ]
  %t8778 = icmp ult i64 %v187542, %v182969
  %or.cond319 = select i1 %t8763.not, i1 %t8778, i1 false
  br i1 %or.cond319, label %edge14721.thread, label %edge14721

edge14721.thread:                                 ; preds = %edge14688
  %v187971 = shl nuw nsw i64 %v187542, 2
  %v187972 = add i64 %v187971, %v182968
  %t8781 = inttoptr i64 %v187972 to ptr
  %t8782 = load i32, ptr %t8781, align 1
  %v187973 = zext i32 %t8782 to i64
  br label %edge14747

edge14721:                                        ; preds = %edge14688
  br i1 %t8763.not, label %edge14747, label %edge14730

edge14730:                                        ; preds = %edge14721
  br i1 %t23.not, label %block94680, label %edge14733

edge14733:                                        ; preds = %edge14730
  %v188061 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t8788.not = icmp eq i64 %v188061, 0
  br i1 %t8788.not, label %block94680, label %block94676

edge14737:                                        ; preds = %block94680
  %v188063 = load i64, ptr %t8761.pre-phi, align 1
  %v188064 = add i64 %v188063, -1
  store i64 %v188064, ptr %t8761.pre-phi, align 1
  %t8792.not = icmp eq i64 %v188064, 0
  br i1 %t8792.not, label %edge14740, label %edge14998

edge14740:                                        ; preds = %edge14737
  %v188065 = call i64 @mech_compact_free(i64 %v0, i64 %v182973)
  %t8793.not = icmp eq i64 %v188065, 0
  br i1 %t8793.not, label %edge14998, label %block94676

edge14747:                                        ; preds = %edge14721.thread, %edge14721
  %v1880121066 = phi i64 [ %v187973, %edge14721.thread ], [ 0, %edge14721 ]
  %v188052 = add i64 %v187033, 1
  %v188042 = lshr i64 %v1880121066, 16
  %v188047 = add i64 %v188042, %v187018
  %v188018 = and i64 %v1880121066, 65535
  %v188029 = and i64 %v187018, 63
  %v188030 = shl i64 %v188018, %v188029
  %v188035 = or i64 %v188030, %v187015
  %v188247 = add nsw i64 %v187048, -1
  br label %block7494

edge14791:                                        ; preds = %edge14459
  %v188547 = shl i64 %v188320, 3
  %v188548 = add i64 %v188547, %v7457721129
  %t8799 = inttoptr i64 %v188548 to ptr
  %v188549 = load i64, ptr %t8799, align 1
  %125 = lshr i64 %v188549, 32
  %v188597 = and i64 %125, 511
  %v188607 = lshr i64 %v188549, 41
  %t8802 = icmp eq i64 %v188597, 1
  br i1 %t8802, label %edge14809, label %edge14839

edge14809:                                        ; preds = %edge14791
  %t8805 = icmp ult i64 %v188607, %v182969
  br i1 %t8805, label %edge14817, label %edge14826

edge14817:                                        ; preds = %edge14809
  %v189022 = shl nuw nsw i64 %v188607, 2
  %v189023 = add i64 %v189022, %v182968
  %t8808 = inttoptr i64 %v189023 to ptr
  %t8809 = load i32, ptr %t8808, align 1
  %v189024 = zext i32 %t8809 to i64
  br label %edge14826

edge14826:                                        ; preds = %edge14817, %edge14809
  %v189044 = phi i64 [ %v189024, %edge14817 ], [ 0, %edge14809 ]
  %v189053 = and i64 %v189044, 65535
  %v189059 = lshr i64 %v189044, 16
  br label %edge15005

edge14839:                                        ; preds = %edge14791
  %t8811 = icmp samesign ult i64 %v188597, 291
  br i1 %t8811, label %edge14847, label %edge14856

edge14847:                                        ; preds = %edge14459, %edge14839
  %v18859710711079 = phi i64 [ %v188597, %edge14839 ], [ 0, %edge14459 ]
  %v18860710721077 = phi i64 [ %v188607, %edge14839 ], [ 0, %edge14459 ]
  %v189299 = shl nuw nsw i64 %v18859710711079, 2
  %v189300 = add i64 %v189299, ptrtoint (ptr @mech_compact_table_20 to i64)
  %t8814 = inttoptr i64 %v189300 to ptr
  %t8815 = load i32, ptr %t8814, align 1
  %v189301 = zext i32 %t8815 to i64
  br label %edge14856

edge14856:                                        ; preds = %edge14847, %edge14839
  %v18859710711078 = phi i64 [ %v18859710711079, %edge14847 ], [ %v188597, %edge14839 ]
  %v18860710721076 = phi i64 [ %v18860710721077, %edge14847 ], [ %v188607, %edge14839 ]
  %v189321 = phi i64 [ %v189301, %edge14847 ], [ 0, %edge14839 ]
  %v189330 = and i64 %v189321, 511
  %v189500 = lshr i64 %v189321, 12
  %t8817 = icmp ult i64 %v189330, %v182969
  br i1 %t8817, label %edge14874, label %edge14883

edge14874:                                        ; preds = %edge14856
  %v189727 = shl nuw nsw i64 %v189330, 2
  %v189728 = add i64 %v189727, %v182968
  %t8820 = inttoptr i64 %v189728 to ptr
  %t8821 = load i32, ptr %t8820, align 1
  %v189729 = zext i32 %t8821 to i64
  br label %edge14883

edge14883:                                        ; preds = %edge14874, %edge14856
  %v189749 = phi i64 [ %v189729, %edge14874 ], [ 0, %edge14856 ]
  %v189759 = lshr i64 %v189749, 16
  %v189764 = and i64 %v189749, 65535
  %v189771 = and i64 %v189759, 31
  %v189777 = shl nuw nsw i64 %v189500, %v189771
  %v189777.masked = and i64 %v189777, 4294967295
  %v189787 = or i64 %v189777.masked, %v189764
  %v189799 = lshr i64 %v189321, 9
  %v189806 = and i64 %v189799, 7
  %v189811 = add nuw nsw i64 %v189759, %v189806
  %v189835 = sub nsw i64 256, %v18860710721076
  %v189840 = lshr i64 %v189835, 29
  %v189850 = and i64 %v189840, 7
  %v189855 = add nsw i64 %v18860710721076, -1
  %v189861 = lshr i64 %v189855, %v189850
  %t8827 = icmp ult i64 %v189861, 256
  br i1 %t8827, label %edge14910, label %edge14917

edge14910:                                        ; preds = %edge14883
  %v190063 = add i64 %v189861, ptrtoint (ptr @mech_compact_table_18 to i64)
  %t8830 = inttoptr i64 %v190063 to ptr
  %t8831 = load i8, ptr %t8830, align 1
  %v190064 = zext i8 %t8831 to i64
  br label %edge14917

edge14917:                                        ; preds = %edge14883, %edge14910
  %v190082 = phi i64 [ %v190064, %edge14910 ], [ 0, %edge14883 ]
  %v190088 = shl nuw nsw i64 %v189850, 1
  %v190093 = add nuw nsw i64 %v190082, %v190088
  %v190102 = add nuw nsw i64 %v190093, 288
  %t8833 = icmp ult i64 %v190102, %v182969
  br i1 %t8833, label %edge14936, label %edge14945

edge14936:                                        ; preds = %edge14917
  %v190329 = shl nuw nsw i64 %v190102, 2
  %v190330 = add i64 %v190329, %v182968
  %t8836 = inttoptr i64 %v190330 to ptr
  %t8837 = load i32, ptr %t8836, align 1
  %v190331 = zext i32 %t8837 to i64
  br label %edge14945

edge14945:                                        ; preds = %edge14936, %edge14917
  %v190351 = phi i64 [ %v190331, %edge14936 ], [ 0, %edge14917 ]
  %v190360 = and i64 %v190351, 65535
  %v190366 = lshr i64 %v190351, 16
  %v190380 = and i64 %v189811, 63
  %v190381 = shl i64 %v190360, %v190380
  %v190386 = or i64 %v189787, %v190381
  %v190391 = add nuw nsw i64 %v190366, %v189811
  %t8840 = icmp samesign ult i64 %v190093, 32
  br i1 %t8840, label %edge14969, label %edge14978

edge14969:                                        ; preds = %edge14945
  %v190400 = shl nuw nsw i64 %v190093, 2
  %v190628 = add i64 %v190400, add (i64 ptrtoint (ptr @mech_compact_table_20 to i64), i64 1036)
  %t8843 = inttoptr i64 %v190628 to ptr
  %t8844 = load i32, ptr %t8843, align 1
  %v190629 = zext i32 %t8844 to i64
  br label %edge14978

edge14978:                                        ; preds = %edge14969, %edge14945
  %v190649 = phi i64 [ %v190629, %edge14969 ], [ 0, %edge14945 ]
  %v190658 = and i64 %v190649, 65535
  %v190663 = sub nsw i64 %v18860710721076, %v190658
  %v190673 = and i64 %v190391, 63
  %v190674 = shl i64 %v190663, %v190673
  %v190679 = or i64 %v190386, %v190674
  %v190685 = lshr i64 %v190649, 16
  %v190690 = add nuw nsw i64 %v190685, %v190391
  br label %edge15005

edge14998:                                        ; preds = %block94680, %edge14737, %edge14740
  %v188302 = sub i64 %v187033, %v184750
  br label %edge15005

edge15003:                                        ; preds = %block7301
  br i1 %t23.not, label %block96423, label %edge15086

edge15005:                                        ; preds = %edge14978, %edge14826, %edge14998, %edge14998.thread1094
  %v190725.ph.pn = phi i64 [ %v186927, %edge14998.thread1094 ], [ %v187015, %edge14998 ], [ %v189053, %edge14826 ], [ %v190679, %edge14978 ]
  %v1907311086 = phi i64 [ 4, %edge14998.thread1094 ], [ %v188302, %edge14998 ], [ 1, %edge14826 ], [ %v18859710711078, %edge14978 ]
  %v1907281085 = phi i64 [ %v186938, %edge14998.thread1094 ], [ %v187018, %edge14998 ], [ %v189059, %edge14826 ], [ %v190690, %edge14978 ]
  %v1907531083.pn = shl i64 %v190725.ph.pn, %v184231
  %v1907661087 = or i64 %v1907531083.pn, %v184228
  %v190964 = add i64 %v1907281085, %v184231
  %v190979 = lshr i64 %v190964, 3
  %v190984 = and i64 %v190979, 255
  %t8852 = icmp samesign ult i64 %v190984, 9
  br i1 %t8852, label %edge15012, label %block96352

edge15012:                                        ; preds = %edge15005
  %t8854 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v184217, i64 %v190984)
  %v191047 = extractvalue { i64, i1 } %t8854, 0
  %t8855 = extractvalue { i64, i1 } %t8854, 1
  br i1 %t8855, label %block96352, label %edge15014

edge15014:                                        ; preds = %edge15012
  %t8858.not = icmp ugt i64 %v191047, %v184218
  br i1 %t8858.not, label %edge15017, label %block96357

edge15017:                                        ; preds = %edge15014
  %t8860.not = icmp eq i64 %v184218, 0
  br i1 %t8860.not, label %block96360, label %edge15018

edge15018:                                        ; preds = %edge15017
  %126 = add i64 %v184218, -131073
  %v191057235 = icmp ult i64 %126, -126977
  br i1 %v191057235, label %edge15021, label %edge15030

edge15021:                                        ; preds = %edge15018
  %t8864 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v184218, i64 %v184218)
  %v191058 = extractvalue { i64, i1 } %t8864, 0
  %t8865 = extractvalue { i64, i1 } %t8864, 1
  br i1 %t8865, label %block96352, label %block96360

edge15030:                                        ; preds = %edge15018
  %t8872 = mul nuw nsw i64 %v184218, 3
  %t8876 = add nuw nsw i64 %t8872, 1
  %v191075 = lshr i64 %t8876, 1
  br label %block96360

edge15034:                                        ; preds = %block96357
  %t8882 = inttoptr i64 %v184221 to ptr
  %v191083 = load i64, ptr %t8882, align 1
  %t8883 = icmp eq i64 %v191083, 1
  %t8884 = icmp ule i64 %v191052, %v184218
  %v191086236 = and i1 %t8884, %t8883
  br i1 %v191086236, label %block96367, label %block96368

edge15039:                                        ; preds = %block96368
  %v191093 = load i64, ptr %ss1, align 8
  %v191094 = load i64, ptr %t606, align 8
  %v191095 = load i64, ptr %t607, align 8
  br label %block96367

edge15041:                                        ; preds = %block96367
  %v191098 = trunc i64 %v1907661087 to i8
  %v191100 = add i64 %v191077, %v184217
  %t8905 = inttoptr i64 %v191100 to ptr
  store i8 %v191098, ptr %t8905, align 1
  %t8906.not = icmp eq i64 %v190984, 1
  br i1 %t8906.not, label %block96386, label %edge15044

edge15044:                                        ; preds = %edge15041
  %v191102 = lshr i64 %v1907661087, 8
  %v191103 = trunc i64 %v191102 to i8
  %v191104 = add i64 %v184217, 1
  %v191105 = add i64 %v191104, %v191077
  %t8908 = inttoptr i64 %v191105 to ptr
  store i8 %v191103, ptr %t8908, align 1
  %t8909 = icmp samesign ugt i64 %v190984, 2
  br i1 %t8909, label %edge15047, label %block96386

edge15047:                                        ; preds = %edge15044
  %v191107 = lshr i64 %v1907661087, 16
  %v191108 = trunc i64 %v191107 to i8
  %v191109 = add i64 %v184217, 2
  %v191110 = add i64 %v191109, %v191077
  %t8911 = inttoptr i64 %v191110 to ptr
  store i8 %v191108, ptr %t8911, align 1
  %t8912.not = icmp eq i64 %v190984, 3
  br i1 %t8912.not, label %block96386, label %edge15050

edge15050:                                        ; preds = %edge15047
  %v191112 = lshr i64 %v1907661087, 24
  %v191113 = trunc i64 %v191112 to i8
  %v191114 = add i64 %v184217, 3
  %v191115 = add i64 %v191114, %v191077
  %t8914 = inttoptr i64 %v191115 to ptr
  store i8 %v191113, ptr %t8914, align 1
  %t8915 = icmp samesign ugt i64 %v190984, 4
  br i1 %t8915, label %edge15053, label %block96386

edge15053:                                        ; preds = %edge15050
  %v191117 = lshr i64 %v1907661087, 32
  %v191118 = trunc i64 %v191117 to i8
  %v191119 = add i64 %v184217, 4
  %v191120 = add i64 %v191119, %v191077
  %t8917 = inttoptr i64 %v191120 to ptr
  store i8 %v191118, ptr %t8917, align 1
  %t8918.not = icmp eq i64 %v190984, 5
  br i1 %t8918.not, label %block96386, label %edge15056

edge15056:                                        ; preds = %edge15053
  %v191122 = lshr i64 %v1907661087, 40
  %v191123 = trunc i64 %v191122 to i8
  %v191124 = add i64 %v184217, 5
  %v191125 = add i64 %v191124, %v191077
  %t8920 = inttoptr i64 %v191125 to ptr
  store i8 %v191123, ptr %t8920, align 1
  %t8921 = icmp samesign ugt i64 %v190984, 6
  br i1 %t8921, label %edge15059, label %block96386

edge15059:                                        ; preds = %edge15056
  %v191127 = lshr i64 %v1907661087, 48
  %v191128 = trunc i64 %v191127 to i8
  %v191129 = add i64 %v184217, 6
  %v191130 = add i64 %v191129, %v191077
  %t8923 = inttoptr i64 %v191130 to ptr
  store i8 %v191128, ptr %t8923, align 1
  %t8924 = icmp eq i64 %v190984, 8
  br i1 %t8924, label %edge15062, label %block96386

edge15062:                                        ; preds = %edge15059
  %v191132 = lshr i64 %v1907661087, 56
  %v191133 = trunc nuw i64 %v191132 to i8
  %v191134 = add i64 %v184217, 7
  %v191135 = add i64 %v191134, %v191077
  %t8926 = inttoptr i64 %v191135 to ptr
  store i8 %v191133, ptr %t8926, align 1
  br label %block96386

edge15065:                                        ; preds = %block96386
  %t8930 = inttoptr i64 %v191082 to ptr
  %v191138 = load i64, ptr %t8930, align 1
  %v191139 = add i64 %v191138, 1
  store i64 %v191139, ptr %t8930, align 1
  br label %block96388

edge15070:                                        ; preds = %block96388
  %v191144 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v184221)
  %t8934.not = icmp eq i64 %v191144, 0
  br i1 %t8934.not, label %edge15074, label %block96352

edge15074:                                        ; preds = %edge15070, %block96388
  %v191160 = and i64 %v190964, 56
  %v191161 = lshr i64 %v1907661087, %v191160
  %v191174 = and i64 %v190964, 7
  %v191451 = add i64 %v1907311086, %v184267
  br label %block7301

edge15086:                                        ; preds = %edge15003
  %v191216 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t8940.not = icmp eq i64 %v191216, 0
  br i1 %t8940.not, label %block96423, label %block96419

edge15092:                                        ; preds = %block96423
  %v191220 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v7458221148)
  %t8942.not = icmp eq i64 %v191220, 0
  br i1 %t8942.not, label %block96427, label %block96419

edge15096:                                        ; preds = %block96427
  %v191222 = load i64, ptr %t8761.pre-phi, align 1
  %v191223 = add i64 %v191222, -1
  store i64 %v191223, ptr %t8761.pre-phi, align 1
  %t8946.not = icmp eq i64 %v191223, 0
  br i1 %t8946.not, label %edge15099, label %edge15143

edge15099:                                        ; preds = %edge15096
  %v191224 = call i64 @mech_compact_free(i64 %v0, i64 %v182973)
  %t8947.not = icmp eq i64 %v191224, 0
  br i1 %t8947.not, label %edge15143, label %block96419

edge15143:                                        ; preds = %edge15099, %edge15096
  %t8950 = icmp ugt i64 %v182969, 256
  br i1 %t8950, label %edge15162, label %edge15147

edge15143.thread:                                 ; preds = %block96427
  %t89501101 = icmp ugt i64 %v182969, 256
  br i1 %t89501101, label %edge15157.thread, label %edge15171

edge15157.thread:                                 ; preds = %edge15143.thread
  %v1923271102 = add i64 %v182968, 1024
  %t89601103 = inttoptr i64 %v1923271102 to ptr
  %t89611104 = load i32, ptr %t89601103, align 1
  br label %edge15171

edge15147:                                        ; preds = %edge15143
  %v192166 = load i64, ptr %t8761.pre-phi, align 1
  %v192167 = add i64 %v192166, -1
  store i64 %v192167, ptr %t8761.pre-phi, align 1
  %t8956.not = icmp eq i64 %v192167, 0
  br i1 %t8956.not, label %edge15150, label %edge15171

edge15150:                                        ; preds = %edge15147
  %v192168 = call i64 @mech_compact_free(i64 %v0, i64 %v182973)
  %t8957.not = icmp eq i64 %v192168, 0
  br i1 %t8957.not, label %edge15171, label %edge15151

edge15151:                                        ; preds = %edge15150
  %t8958 = insertvalue { i64, i64 } poison, i64 %v192168, 0
  %t8959 = insertvalue { i64, i64 } %t8958, i64 7975, 1
  br label %common.ret

edge15162:                                        ; preds = %edge15143
  %v192327 = add i64 %v182968, 1024
  %t8960 = inttoptr i64 %v192327 to ptr
  %t8961 = load i32, ptr %t8960, align 1
  %v192332 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v182973)
  %t8963.not = icmp eq i64 %v192332, 0
  br i1 %t8963.not, label %edge15171, label %edge15163

edge15163:                                        ; preds = %edge15162
  %t8964 = insertvalue { i64, i64 } poison, i64 %v192332, 0
  %t8965 = insertvalue { i64, i64 } %t8964, i64 7980, 1
  br label %common.ret

edge15171:                                        ; preds = %edge15143.thread, %edge15157.thread, %edge15162, %edge15147, %edge15150
  %v192348.shrunk = phi i32 [ %t8961, %edge15162 ], [ %t89611104, %edge15157.thread ], [ 0, %edge15143.thread ], [ 0, %edge15147 ], [ 0, %edge15150 ]
  %v192348 = zext i32 %v192348.shrunk to i64
  %v192357 = and i64 %v192348, 65535
  %v192372 = shl nuw nsw i64 %v192357, %v184231
  %v192377 = or i64 %v192372, %v184228
  %v192383 = lshr i64 %v192348, 16
  %v192396 = add nuw nsw i64 %v192383, %v184231
  %v192404 = lshr i64 %v192396, 3
  %v192409 = and i64 %v192404, 255
  %t8974 = icmp samesign ult i64 %v192409, 9
  br i1 %t8974, label %edge15190, label %block96798

edge15190:                                        ; preds = %edge15171
  %t8976 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v184217, i64 %v192409)
  %v192424 = extractvalue { i64, i1 } %t8976, 0
  %t8977 = extractvalue { i64, i1 } %t8976, 1
  br i1 %t8977, label %block96798, label %edge15192

edge15192:                                        ; preds = %edge15190
  %t8980.not = icmp ugt i64 %v192424, %v184218
  br i1 %t8980.not, label %edge15195, label %block96803

edge15195:                                        ; preds = %edge15192
  %t8982.not = icmp eq i64 %v184218, 0
  br i1 %t8982.not, label %block96806, label %edge15196

edge15196:                                        ; preds = %edge15195
  %127 = add i64 %v184218, -131073
  %v192434231 = icmp ult i64 %127, -126977
  br i1 %v192434231, label %edge15199, label %edge15208

edge15199:                                        ; preds = %edge15196
  %t8986 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v184218, i64 %v184218)
  %v192435 = extractvalue { i64, i1 } %t8986, 0
  %t8987 = extractvalue { i64, i1 } %t8986, 1
  br i1 %t8987, label %block96798, label %block96806

edge15208:                                        ; preds = %edge15196
  %t8994 = mul nuw nsw i64 %v184218, 3
  %t8998 = add nuw nsw i64 %t8994, 1
  %v192452 = lshr i64 %t8998, 1
  br label %block96806

edge15212:                                        ; preds = %block96803
  %t9004 = inttoptr i64 %v184221 to ptr
  %v192460 = load i64, ptr %t9004, align 1
  %t9005 = icmp eq i64 %v192460, 1
  %t9006 = icmp ule i64 %v192429, %v184218
  %v192463232 = and i1 %t9006, %t9005
  br i1 %v192463232, label %block96813, label %block96814

edge15217:                                        ; preds = %block96814
  %v192470 = load i64, ptr %ss1, align 8
  %v192471 = load i64, ptr %t606, align 8
  %v192472 = load i64, ptr %t607, align 8
  br label %block96813

edge15219:                                        ; preds = %block96813
  %v192475 = trunc i64 %v192377 to i8
  %v192477 = add i64 %v192454, %v184217
  %t9027 = inttoptr i64 %v192477 to ptr
  store i8 %v192475, ptr %t9027, align 1
  %t9028.not = icmp eq i64 %v192409, 1
  br i1 %t9028.not, label %block96832, label %edge15222

edge15222:                                        ; preds = %edge15219
  %v192479 = lshr i64 %v192377, 8
  %v192480 = trunc i64 %v192479 to i8
  %v192481 = add i64 %v184217, 1
  %v192482 = add i64 %v192481, %v192454
  %t9030 = inttoptr i64 %v192482 to ptr
  store i8 %v192480, ptr %t9030, align 1
  %t9031 = icmp samesign ugt i64 %v192409, 2
  br i1 %t9031, label %edge15225, label %block96832

edge15225:                                        ; preds = %edge15222
  %v192484 = lshr i64 %v192377, 16
  %v192485 = trunc i64 %v192484 to i8
  %v192486 = add i64 %v184217, 2
  %v192487 = add i64 %v192486, %v192454
  %t9033 = inttoptr i64 %v192487 to ptr
  store i8 %v192485, ptr %t9033, align 1
  %t9034.not = icmp eq i64 %v192409, 3
  br i1 %t9034.not, label %block96832, label %edge15228

edge15228:                                        ; preds = %edge15225
  %v192489 = lshr i64 %v192377, 24
  %v192490 = trunc i64 %v192489 to i8
  %v192491 = add i64 %v184217, 3
  %v192492 = add i64 %v192491, %v192454
  %t9036 = inttoptr i64 %v192492 to ptr
  store i8 %v192490, ptr %t9036, align 1
  %t9037 = icmp samesign ugt i64 %v192409, 4
  br i1 %t9037, label %edge15231, label %block96832

edge15231:                                        ; preds = %edge15228
  %v192494 = lshr i64 %v192377, 32
  %v192495 = trunc i64 %v192494 to i8
  %v192496 = add i64 %v184217, 4
  %v192497 = add i64 %v192496, %v192454
  %t9039 = inttoptr i64 %v192497 to ptr
  store i8 %v192495, ptr %t9039, align 1
  %t9040.not = icmp eq i64 %v192409, 5
  br i1 %t9040.not, label %block96832, label %edge15234

edge15234:                                        ; preds = %edge15231
  %v192499 = lshr i64 %v192377, 40
  %v192500 = trunc i64 %v192499 to i8
  %v192501 = add i64 %v184217, 5
  %v192502 = add i64 %v192501, %v192454
  %t9042 = inttoptr i64 %v192502 to ptr
  store i8 %v192500, ptr %t9042, align 1
  %t9043 = icmp samesign ugt i64 %v192409, 6
  br i1 %t9043, label %edge15237, label %block96832

edge15237:                                        ; preds = %edge15234
  %v192504 = lshr i64 %v192377, 48
  %v192505 = trunc i64 %v192504 to i8
  %v192506 = add i64 %v184217, 6
  %v192507 = add i64 %v192506, %v192454
  %t9045 = inttoptr i64 %v192507 to ptr
  store i8 %v192505, ptr %t9045, align 1
  %t9046 = icmp eq i64 %v192409, 8
  br i1 %t9046, label %edge15240, label %block96832

edge15240:                                        ; preds = %edge15237
  %v192509 = lshr i64 %v192377, 56
  %v192510 = trunc nuw i64 %v192509 to i8
  %v192511 = add i64 %v184217, 7
  %v192512 = add i64 %v192511, %v192454
  %t9048 = inttoptr i64 %v192512 to ptr
  store i8 %v192510, ptr %t9048, align 1
  br label %block96832

edge15243:                                        ; preds = %block96832
  %t9052 = inttoptr i64 %v192459 to ptr
  %v192515 = load i64, ptr %t9052, align 1
  %v192516 = add i64 %v192515, 1
  store i64 %v192516, ptr %t9052, align 1
  br label %block96834

edge15248:                                        ; preds = %block96834
  %v192521 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v184221)
  %t9056.not = icmp eq i64 %v192521, 0
  br i1 %t9056.not, label %edge15252, label %block96798

edge15252:                                        ; preds = %edge15248, %block96834
  %v192537 = and i64 %v192396, 56
  %v192538 = lshr i64 %v192377, %v192537
  %v192543 = and i64 %v192396, 7
  br label %edge15273

edge15273:                                        ; preds = %edge13613, %edge13610, %edge13608, %edge15252
  %v204936 = phi i64 [ %v192454, %edge15252 ], [ %v176610, %edge13608 ], [ %v176610, %edge13610 ], [ %v176610, %edge13613 ]
  %v204950 = phi i64 [ %v192424, %edge15252 ], [ %v176580, %edge13608 ], [ %v176580, %edge13610 ], [ %v176580, %edge13613 ]
  %v204964 = phi i64 [ %v192456, %edge15252 ], [ %v176612, %edge13608 ], [ %v176612, %edge13610 ], [ %v176612, %edge13613 ]
  %v205006 = phi i64 [ %v192459, %edge15252 ], [ %v176615, %edge13608 ], [ %v176615, %edge13610 ], [ %v176615, %edge13613 ]
  %v205026 = phi i64 [ %v192538, %edge15252 ], [ 0, %edge13608 ], [ 0, %edge13610 ], [ 0, %edge13613 ]
  %v205042 = phi i64 [ %v192543, %edge15252 ], [ 0, %edge13608 ], [ 0, %edge13610 ], [ 0, %edge13613 ]
  %t9061 = icmp eq i64 %v54962, 1
  %t9062 = icmp eq i64 %v54964, 1
  %v194372259 = and i1 %t9061, %t9062
  br i1 %v194372259, label %block7887, label %edge15278

edge15278:                                        ; preds = %edge15273
  %t9065.not = icmp eq i64 %v54942, 0
  br i1 %t9065.not, label %block96882, label %edge15281

edge15281:                                        ; preds = %edge15278
  %v194382 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54942)
  %t9066.not = icmp eq i64 %v194382, 0
  br i1 %t9066.not, label %block96882, label %block96878

edge15287:                                        ; preds = %block96882
  %v194386 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54954)
  %t9068.not = icmp eq i64 %v194386, 0
  br i1 %t9068.not, label %edge15527, label %block96878

edge15299:                                        ; preds = %block7887
  %t9072 = inttoptr i64 %v194541 to ptr
  %v194542 = load i64, ptr %t9072, align 1
  %t9073 = icmp ne i64 %v194542, 1
  %t9075.not = icmp ugt i64 %v3718, %v194561
  %or.cond320 = select i1 %t9073, i1 true, i1 %t9075.not
  br i1 %or.cond320, label %edge15319, label %edge15303

edge15303:                                        ; preds = %edge15299
  %t9077 = zext i64 %v54959 to i128
  %t9078 = zext i64 %v54960 to i128
  %t9079 = shl nuw i128 %t9078, 64
  %v218828 = or disjoint i128 %t9079, %t9077
  store i128 %v218828, ptr %ss2, align 16
  %t9081 = zext nneg i64 %v194564 to i128
  %t9082 = zext i64 %v194565 to i128
  %t9083 = shl nuw i128 %t9082, 64
  %v218830 = or disjoint i128 %t9083, %t9081
  store i128 %v218830, ptr %t926, align 16
  store i128 %v218142, ptr %t931, align 16
  %t9092 = zext i64 %v194541 to i128
  %t9093 = shl nuw i128 %t9092, 64
  %v218834 = or disjoint i128 %t9093, %t1163
  store i128 %v218834, ptr %t936, align 16
  store i64 %v194561, ptr %t941, align 16
  %t9097 = call fastcc { i64, i64 } @mech_compact_numeric_8026(i64 %v218103, i64 %v8479)
  %v194547 = extractvalue { i64, i64 } %t9097, 0
  %t9098.not = icmp eq i64 %v194547, 0
  br i1 %t9098.not, label %edge15431, label %edge15305

edge15305:                                        ; preds = %edge15303
  %v194548 = extractvalue { i64, i64 } %t9097, 1
  %t9099 = icmp eq i64 %v194547, 103169
  %v194551 = select i1 %t9099, i64 103425, i64 %v194547
  %t9101 = insertvalue { i64, i64 } poison, i64 %v194551, 0
  %t9102 = insertvalue { i64, i64 } %t9101, i64 %v194548, 1
  br label %common.ret

edge15319:                                        ; preds = %block7887, %edge15299
  %exitcond20656.not = icmp eq i64 %v194564, %v54960
  br i1 %exitcond20656.not, label %edge15431, label %edge15325

edge15325:                                        ; preds = %edge15319
  %v194762 = add i64 %v194564, %v54959
  %t9106.not = icmp ult i64 %v194762, %v54959
  br i1 %t9106.not, label %common.ret, label %edge15329

edge15329:                                        ; preds = %edge15325
  %t9110 = icmp ult i64 %v194762, %v3718
  br i1 %t9110, label %edge15335, label %edge15351

edge15335:                                        ; preds = %edge15329
  %v194938 = shl nuw nsw i64 %v194762, 2
  %v194939 = add i64 %v194938, %v194565
  %t9113 = inttoptr i64 %v194939 to ptr
  %t9114 = load i32, ptr %t9113, align 1
  %v194940 = zext i32 %t9114 to i64
  br label %edge15351

edge15351:                                        ; preds = %edge15329, %edge15335
  %v195186 = phi i64 [ 0, %edge15329 ], [ %v194940, %edge15335 ]
  %exitcond20657.not = icmp eq i64 %v194564, %v3718
  br i1 %exitcond20657.not, label %block97388, label %edge15354

edge15354:                                        ; preds = %edge15351
  %v195143 = shl i64 %v194564, 2
  %v195144 = add i64 %v194565, %v195143
  %t9121 = inttoptr i64 %v195144 to ptr
  %t9122 = load i32, ptr %t9121, align 1
  %v195145 = zext i32 %t9122 to i64
  %t9123 = icmp eq i64 %v195186, %v195145
  br i1 %t9123, label %block97391, label %edge15357

edge15357:                                        ; preds = %edge15354
  br i1 %t9071.not, label %block97393, label %edge15358

edge15358:                                        ; preds = %edge15357
  %t9126 = inttoptr i64 %v194541 to ptr
  %v195161 = load i64, ptr %t9126, align 1
  %t9127 = icmp eq i64 %v195161, 1
  br i1 %t9127, label %block97392, label %block97393

edge15363:                                        ; preds = %block97393
  %v195171 = load i64, ptr %ss1, align 8
  %v195172 = load i64, ptr %t606, align 8
  %v195173 = load i64, ptr %t607, align 8
  %.pre20683 = add i64 %v195171, %v195143
  %.pre20684 = inttoptr i64 %.pre20683 to ptr
  br label %block97392

edge15366:                                        ; preds = %block97391
  %t9151 = inttoptr i64 %v195154 to ptr
  %v195178 = load i64, ptr %t9151, align 1
  %v195179 = add i64 %v195178, 1
  store i64 %v195179, ptr %t9151, align 1
  br label %block97397

edge15371:                                        ; preds = %block97397
  %v195184 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v194541)
  %t9155.not = icmp eq i64 %v195184, 0
  br i1 %t9155.not, label %edge15386, label %block97388

edge15386:                                        ; preds = %block97397, %edge15371
  %v195369 = add nuw nsw i64 %v194564, 1
  br label %block7887

edge15411:                                        ; preds = %edge15434
  %t9169 = insertvalue { i64, i64 } poison, i64 %v196483, 0
  %t9170 = insertvalue { i64, i64 } %t9169, i64 8083, 1
  br label %common.ret

edge15431:                                        ; preds = %edge15319, %edge15303
  %v196545 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3192)
  %t9183.not = icmp eq i64 %v196545, 0
  br i1 %t9183.not, label %edge15433, label %block97576

edge15433:                                        ; preds = %edge15431
  %v196548 = load i64, ptr %ss1, align 8
  %v196549 = load i64, ptr %t606, align 8
  %v196550 = load i64, ptr %t607, align 8
  %t9186.not = icmp eq i64 %v196550, 0
  br i1 %t9186.not, label %edge15474, label %edge15434

edge15434:                                        ; preds = %edge15433
  %t9187 = inttoptr i64 %v196550 to ptr
  %v196551 = load i64, ptr %t9187, align 1
  %v196479 = add i64 %v196551, 2
  store i64 %v196479, ptr %t9187, align 1
  %v196483 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v196550)
  %t9168.not = icmp eq i64 %v196483, 0
  br i1 %t9168.not, label %edge15474, label %edge15411

edge15453:                                        ; preds = %edge15477
  %v197736 = call i64 @mech_compact_free(i64 %v0, i64 %v197793)
  %t9202.not = icmp eq i64 %v197736, 0
  br i1 %t9202.not, label %edge15702, label %edge15454

edge15454:                                        ; preds = %edge15453
  %t9203 = insertvalue { i64, i64 } poison, i64 %v197736, 0
  %t9204 = insertvalue { i64, i64 } %t9203, i64 8096, 1
  br label %common.ret

edge15474:                                        ; preds = %edge15434, %edge15433
  %v197788 = call i64 @mech_compact_allocate(i64 %v0, i64 259, i64 4, i64 4, i64 1, i64 %v3192)
  %t9217.not = icmp eq i64 %v197788, 0
  br i1 %t9217.not, label %edge15476, label %block97631

edge15476:                                        ; preds = %edge15474
  %v197791 = load i64, ptr %ss1, align 8
  %v197792 = load i64, ptr %t606, align 8
  %v197793 = load i64, ptr %t607, align 8
  %t9220.not = icmp eq i64 %v197793, 0
  br i1 %t9220.not, label %edge15702, label %edge15477

edge15477:                                        ; preds = %edge15476
  %t9221 = inttoptr i64 %v197793 to ptr
  %v197794 = load i64, ptr %t9221, align 1
  %v197795 = add i64 %v197794, 1
  store i64 %v197795, ptr %t9221, align 1
  %t9201.not = icmp eq i64 %v197795, 0
  br i1 %t9201.not, label %edge15453, label %edge15702

edge15507:                                        ; preds = %edge15530
  %t9232 = insertvalue { i64, i64 } poison, i64 %v199116, 0
  %t9233 = insertvalue { i64, i64 } %t9232, i64 8113, 1
  br label %common.ret

edge15527:                                        ; preds = %block96882, %edge15287
  %v199164 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3192)
  %t9246.not = icmp eq i64 %v199164, 0
  br i1 %t9246.not, label %edge15529, label %block97730

edge15529:                                        ; preds = %edge15527
  %v199167 = load i64, ptr %ss1, align 8
  %v199168 = load i64, ptr %t606, align 8
  %v199169 = load i64, ptr %t607, align 8
  %t9249.not = icmp eq i64 %v199169, 0
  br i1 %t9249.not, label %edge15567, label %edge15530

edge15530:                                        ; preds = %edge15529
  %t9250 = inttoptr i64 %v199169 to ptr
  %v199170 = load i64, ptr %t9250, align 1
  %v199112 = add i64 %v199170, 2
  store i64 %v199112, ptr %t9250, align 1
  %v199116 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v199169)
  %t9231.not = icmp eq i64 %v199116, 0
  br i1 %t9231.not, label %edge15567, label %edge15507

edge15547:                                        ; preds = %edge15570
  %t9261 = insertvalue { i64, i64 } poison, i64 %v200355, 0
  %t9262 = insertvalue { i64, i64 } %t9261, i64 8126, 1
  br label %common.ret

edge15567:                                        ; preds = %edge15530, %edge15529
  %v200403 = call i64 @mech_compact_allocate(i64 %v0, i64 10, i64 4, i64 4, i64 1, i64 %v3192)
  %t9275.not = icmp eq i64 %v200403, 0
  br i1 %t9275.not, label %edge15569, label %block97785

edge15569:                                        ; preds = %edge15567
  %v200406 = load i64, ptr %ss1, align 8
  %v200407 = load i64, ptr %t606, align 8
  %v200408 = load i64, ptr %t607, align 8
  %t9278.not = icmp eq i64 %v200408, 0
  br i1 %t9278.not, label %edge15607, label %edge15570

edge15570:                                        ; preds = %edge15569
  %t9279 = inttoptr i64 %v200408 to ptr
  %v200409 = load i64, ptr %t9279, align 1
  %v200351 = add i64 %v200409, 2
  store i64 %v200351, ptr %t9279, align 1
  %v200355 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v200408)
  %t9260.not = icmp eq i64 %v200355, 0
  br i1 %t9260.not, label %edge15607, label %edge15547

edge15587:                                        ; preds = %edge15610
  %t9290 = insertvalue { i64, i64 } poison, i64 %v201606, 0
  %t9291 = insertvalue { i64, i64 } %t9290, i64 8139, 1
  br label %common.ret

edge15607:                                        ; preds = %edge15570, %edge15569
  %v201658 = call i64 @mech_compact_allocate(i64 %v0, i64 259, i64 4, i64 4, i64 1, i64 %v3192)
  %t9304.not = icmp eq i64 %v201658, 0
  br i1 %t9304.not, label %edge15609, label %block97840

edge15609:                                        ; preds = %edge15607
  %v201661 = load i64, ptr %ss1, align 8
  %v201662 = load i64, ptr %t606, align 8
  %v201663 = load i64, ptr %t607, align 8
  %t9307.not = icmp eq i64 %v201663, 0
  br i1 %t9307.not, label %edge15650, label %edge15610

edge15610:                                        ; preds = %edge15609
  %t9308 = inttoptr i64 %v201663 to ptr
  %v201664 = load i64, ptr %t9308, align 1
  %v201602 = add i64 %v201664, 2
  store i64 %v201602, ptr %t9308, align 1
  %v201606 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v201663)
  %t9289.not = icmp eq i64 %v201606, 0
  br i1 %t9289.not, label %edge15650, label %edge15587

edge15629:                                        ; preds = %edge15653
  %v202873 = call i64 @mech_compact_free(i64 %v0, i64 %v202926)
  %t9323.not = icmp eq i64 %v202873, 0
  br i1 %t9323.not, label %edge15702, label %edge15630

edge15630:                                        ; preds = %edge15629
  %t9324 = insertvalue { i64, i64 } poison, i64 %v202873, 0
  %t9325 = insertvalue { i64, i64 } %t9324, i64 8152, 1
  br label %common.ret

edge15650:                                        ; preds = %edge15610, %edge15609
  %v202921 = call i64 @mech_compact_allocate(i64 %v0, i64 259, i64 4, i64 4, i64 1, i64 %v3192)
  %t9338.not = icmp eq i64 %v202921, 0
  br i1 %t9338.not, label %edge15652, label %block97895

edge15652:                                        ; preds = %edge15650
  %v202924 = load i64, ptr %ss1, align 8
  %v202925 = load i64, ptr %t606, align 8
  %v202926 = load i64, ptr %t607, align 8
  %t9341.not = icmp eq i64 %v202926, 0
  br i1 %t9341.not, label %edge15702, label %edge15653

edge15653:                                        ; preds = %edge15652
  %t9342 = inttoptr i64 %v202926 to ptr
  %v202927 = load i64, ptr %t9342, align 1
  %v202928 = add i64 %v202927, 1
  store i64 %v202928, ptr %t9342, align 1
  %t9322.not = icmp eq i64 %v202928, 0
  br i1 %t9322.not, label %edge15629, label %edge15702

edge15702:                                        ; preds = %edge15652, %edge15476, %edge15453, %edge15477, %edge15629, %edge15653
  %v203700 = phi i64 [ %v54919, %edge15652 ], [ %v194565, %edge15453 ], [ %v194565, %edge15477 ], [ %v54919, %edge15629 ], [ %v54919, %edge15653 ], [ %v194565, %edge15476 ]
  %v203702 = phi i64 [ %v54921, %edge15652 ], [ %v194561, %edge15453 ], [ %v194561, %edge15477 ], [ %v54921, %edge15629 ], [ %v54921, %edge15653 ], [ %v194561, %edge15476 ]
  %v203705 = phi i64 [ %v54924, %edge15652 ], [ %v194541, %edge15453 ], [ %v194541, %edge15477 ], [ %v54924, %edge15629 ], [ %v54924, %edge15653 ], [ %v194541, %edge15476 ]
  %v204073 = phi i64 [ %v54927, %edge15652 ], [ %v54958, %edge15453 ], [ %v54958, %edge15477 ], [ %v54927, %edge15629 ], [ %v54927, %edge15653 ], [ %v54958, %edge15476 ]
  %v204266 = phi i64 [ 0, %edge15652 ], [ %v54960, %edge15453 ], [ %v54960, %edge15477 ], [ 0, %edge15629 ], [ 0, %edge15653 ], [ %v54960, %edge15476 ]
  %v204269 = phi i64 [ %v202924, %edge15652 ], [ %v197791, %edge15453 ], [ %v197791, %edge15477 ], [ %v202924, %edge15629 ], [ %v202924, %edge15653 ], [ %v197791, %edge15476 ]
  %v204271 = phi i64 [ %v202925, %edge15652 ], [ %v197792, %edge15453 ], [ %v197792, %edge15477 ], [ %v202925, %edge15629 ], [ %v202925, %edge15653 ], [ %v197792, %edge15476 ]
  %v204274 = phi i64 [ 0, %edge15652 ], [ %v197793, %edge15453 ], [ %v197793, %edge15477 ], [ %v202926, %edge15629 ], [ %v202926, %edge15653 ], [ 0, %edge15476 ]
  %v204281 = phi i64 [ %v201661, %edge15652 ], [ %v54937, %edge15453 ], [ %v54937, %edge15477 ], [ %v201661, %edge15629 ], [ %v201661, %edge15653 ], [ %v54937, %edge15476 ]
  %v204283 = phi i64 [ %v201662, %edge15652 ], [ %v54939, %edge15453 ], [ %v54939, %edge15477 ], [ %v201662, %edge15629 ], [ %v201662, %edge15653 ], [ %v54939, %edge15476 ]
  %v204286 = phi i64 [ %v201663, %edge15652 ], [ %v54942, %edge15453 ], [ %v54942, %edge15477 ], [ %v201663, %edge15629 ], [ %v201663, %edge15653 ], [ %v54942, %edge15476 ]
  %v204874 = phi i64 [ %v200406, %edge15652 ], [ %v196548, %edge15453 ], [ %v196548, %edge15477 ], [ %v200406, %edge15629 ], [ %v200406, %edge15653 ], [ %v196548, %edge15476 ]
  %v204876 = phi i64 [ %v200407, %edge15652 ], [ %v196549, %edge15453 ], [ %v196549, %edge15477 ], [ %v200407, %edge15629 ], [ %v200407, %edge15653 ], [ %v196549, %edge15476 ]
  %v204879 = phi i64 [ %v200408, %edge15652 ], [ %v196550, %edge15453 ], [ %v196550, %edge15477 ], [ %v200408, %edge15629 ], [ %v200408, %edge15653 ], [ %v196550, %edge15476 ]
  %v204886 = phi i64 [ %v199167, %edge15652 ], [ %v54949, %edge15453 ], [ %v54949, %edge15477 ], [ %v199167, %edge15629 ], [ %v199167, %edge15653 ], [ %v54949, %edge15476 ]
  %v204888 = phi i64 [ %v199168, %edge15652 ], [ %v54951, %edge15453 ], [ %v54951, %edge15477 ], [ %v199168, %edge15629 ], [ %v199168, %edge15653 ], [ %v54951, %edge15476 ]
  %v204891 = phi i64 [ %v199169, %edge15652 ], [ %v54954, %edge15453 ], [ %v54954, %edge15477 ], [ %v199169, %edge15629 ], [ %v199169, %edge15653 ], [ %v54954, %edge15476 ]
  %v204901 = phi i64 [ 0, %edge15652 ], [ %v54956, %edge15453 ], [ %v54956, %edge15477 ], [ 0, %edge15629 ], [ 0, %edge15653 ], [ %v54956, %edge15476 ]
  %t9346 = icmp eq i64 %v54927, %v1395
  br i1 %t9346, label %edge15721, label %edge15835

edge15721:                                        ; preds = %edge15702
  br i1 %t23.not, label %block98105, label %edge15724

edge15724:                                        ; preds = %edge15721
  %v205170 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v1399)
  %t9350.not = icmp eq i64 %v205170, 0
  br i1 %t9350.not, label %block98105, label %block98033

edge15736:                                        ; preds = %block98105
  %v205178 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v138168)
  %t9354.not = icmp eq i64 %v205178, 0
  br i1 %t9354.not, label %block98109, label %block98033

edge15742:                                        ; preds = %block98109
  %v205182 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217631)
  %t9356.not = icmp eq i64 %v205182, 0
  br i1 %t9356.not, label %block98113, label %block98033

edge15748:                                        ; preds = %block98113
  %v205186 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54918)
  %t9358.not = icmp eq i64 %v205186, 0
  br i1 %t9358.not, label %block98117, label %block98033

edge15754:                                        ; preds = %block98117
  %v205190 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v203705)
  %t9360.not = icmp eq i64 %v205190, 0
  br i1 %t9360.not, label %block98121, label %block98033

edge15760:                                        ; preds = %block98121
  %v205194 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v7458221148)
  %t9362.not = icmp eq i64 %v205194, 0
  br i1 %t9362.not, label %block98125, label %block98033

edge15766:                                        ; preds = %block98125
  %v205198 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v204274)
  %t9364.not = icmp eq i64 %v205198, 0
  br i1 %t9364.not, label %block98129, label %block98033

edge15772:                                        ; preds = %block98129
  %v205202 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v204286)
  %t9366.not = icmp eq i64 %v205202, 0
  br i1 %t9366.not, label %block98133, label %block98033

edge15778:                                        ; preds = %block98133
  %v205206 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v54948)
  %t9368.not = icmp eq i64 %v205206, 0
  br i1 %t9368.not, label %block98137, label %block98033

edge15784:                                        ; preds = %block98137
  %v205210 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v204879)
  %t9370.not = icmp eq i64 %v205210, 0
  br i1 %t9370.not, label %block98141, label %block98033

edge15788:                                        ; preds = %block98141
  %t9372 = inttoptr i64 %v204891 to ptr
  %v205212 = load i64, ptr %t9372, align 1
  %v205213 = add i64 %v205212, -1
  store i64 %v205213, ptr %t9372, align 1
  %t9374.not = icmp eq i64 %v205213, 0
  br i1 %t9374.not, label %edge15791, label %edge15840

edge15791:                                        ; preds = %edge15788
  %v205214 = call i64 @mech_compact_free(i64 %v0, i64 %v204891)
  %t9375.not = icmp eq i64 %v205214, 0
  br i1 %t9375.not, label %edge15840, label %block98033

edge15835:                                        ; preds = %edge15702
  %t854.not = icmp ult i64 %v1395, %v204073
  br i1 %t854.not, label %common.ret, label %block9980

edge15840:                                        ; preds = %edge15791, %edge15788, %block98141
  %t9378.not = icmp eq i64 %v205042, 0
  br i1 %t9378.not, label %edge15901, label %edge15849

edge15849:                                        ; preds = %edge15840
  %t9382 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v204950, i64 1)
  %v217155 = extractvalue { i64, i1 } %t9382, 0
  %t9383 = extractvalue { i64, i1 } %t9382, 1
  br i1 %t9383, label %block98425, label %edge15854

edge15854:                                        ; preds = %edge15849
  %t9386.not = icmp ugt i64 %v217155, %v204964
  br i1 %t9386.not, label %edge15857, label %block98429

edge15857:                                        ; preds = %edge15854
  %t9388.not = icmp eq i64 %v204964, 0
  br i1 %t9388.not, label %block98432, label %edge15858

edge15858:                                        ; preds = %edge15857
  %128 = add i64 %v204964, -131073
  %v217165262 = icmp ult i64 %128, -126977
  br i1 %v217165262, label %edge15861, label %edge15870

edge15861:                                        ; preds = %edge15858
  %t9392 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %v204964, i64 %v204964)
  %v217166 = extractvalue { i64, i1 } %t9392, 0
  %t9393 = extractvalue { i64, i1 } %t9392, 1
  br i1 %t9393, label %block98425, label %block98432

edge15870:                                        ; preds = %edge15858
  %t9400 = mul nuw nsw i64 %v204964, 3
  %t9404 = add nuw nsw i64 %t9400, 1
  %v217183 = lshr i64 %t9404, 1
  br label %block98432

edge15874:                                        ; preds = %block98429
  %t9410 = inttoptr i64 %v205006 to ptr
  %v217191 = load i64, ptr %t9410, align 1
  %t9411 = icmp eq i64 %v217191, 1
  %t9412 = icmp ule i64 %v217160, %v204964
  %v217194263 = and i1 %t9412, %t9411
  br i1 %v217194263, label %block98439, label %block98440

edge15879:                                        ; preds = %block98440
  %v217201 = load i64, ptr %ss1, align 8
  %v217202 = load i64, ptr %t606, align 8
  %v217203 = load i64, ptr %t607, align 8
  br label %block98439

edge15881:                                        ; preds = %block98439
  %t9435 = inttoptr i64 %v217190 to ptr
  %v217210 = load i64, ptr %t9435, align 1
  %v217211 = add i64 %v217210, 1
  store i64 %v217211, ptr %t9435, align 1
  br label %block98444

edge15886:                                        ; preds = %block98444
  %v217216 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v205006)
  %t9439.not = icmp eq i64 %v217216, 0
  br i1 %t9439.not, label %edge15901, label %block98425

edge15901:                                        ; preds = %edge166, %edge163, %block8342, %edge409, %edge411, %edge414, %edge15886, %block98444, %edge15840
  %v217258 = phi i64 [ %v2793, %edge414 ], [ %v2017, %block8342 ], [ %v2017, %edge166 ], [ %v2017, %edge163 ], [ %v2793, %edge409 ], [ %v2793, %edge411 ], [ %v217185, %block98444 ], [ %v217185, %edge15886 ], [ %v204936, %edge15840 ]
  %v217259 = phi i64 [ %v2763, %edge414 ], [ %v1987, %block8342 ], [ %v1987, %edge166 ], [ %v1987, %edge163 ], [ %v2763, %edge409 ], [ %v2763, %edge411 ], [ %v217155, %block98444 ], [ %v217155, %edge15886 ], [ %v204950, %edge15840 ]
  %v217260 = phi i64 [ %v2795, %edge414 ], [ %v2019, %block8342 ], [ %v2019, %edge166 ], [ %v2019, %edge163 ], [ %v2795, %edge409 ], [ %v2795, %edge411 ], [ %v217187, %block98444 ], [ %v217187, %edge15886 ], [ %v204964, %edge15840 ]
  %v217263 = phi i64 [ %v2798, %edge414 ], [ %v2022, %block8342 ], [ %v2022, %edge166 ], [ %v2022, %edge163 ], [ %v2798, %edge409 ], [ %v2798, %edge411 ], [ %v217190, %block98444 ], [ %v217190, %edge15886 ], [ %v205006, %edge15840 ]
  %t9442.not = icmp eq i64 %v217263, 0
  br i1 %t9442.not, label %block98481, label %edge15902

edge15902:                                        ; preds = %edge15901
  %t9443 = inttoptr i64 %v217263 to ptr
  %v217272 = load i64, ptr %t9443, align 1
  %v217273 = add i64 %v217272, 1
  store i64 %v217273, ptr %t9443, align 1
  %v217277 = call i64 @mech_compact_release_owner(i64 %v0, i64 %v217263)
  %t9445.not = icmp eq i64 %v217277, 0
  br i1 %t9445.not, label %block98481, label %edge15907

edge15907:                                        ; preds = %edge15902
  %t9459 = insertvalue { i64, i64 } poison, i64 %v217277, 0
  %t9460 = insertvalue { i64, i64 } %t9459, i64 8222, 1
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

declare { i64, i64 } @mech_compact_numeric_3009_no_growth(i64, i64, i64) local_unnamed_addr #3

declare { i64, i64 } @mech_compact_numeric_3009_no_growth_composed(i64, i64, i64) local_unnamed_addr #3

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64) #2

; Function Attrs: mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64) #2

declare i64 @mech_compact_unsigned_dot_product(i64, i64, i64, i64) local_unnamed_addr #3

; Function Attrs: noinline nounwind memory(readwrite, inaccessiblemem: write, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_2821(i64 %v1, i64 %v2) unnamed_addr #5 align 32 {
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
define internal fastcc { i64, i64 } @mech_compact_numeric_2821_vector(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
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
define internal fastcc { i64, i64 } @mech_compact_numeric_3009(i64 %v0, i64 %v1, i64 %v2) unnamed_addr #0 align 32 {
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
define internal fastcc { i64, i64 } @mech_compact_numeric_3447(i64 %v1307, i64 %v1308) unnamed_addr #6 align 32 {
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
define internal fastcc { i64, i64 } @mech_compact_numeric_3447_grouped(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
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
define internal fastcc { i64, i64 } @mech_compact_numeric_362(i64 %v1, i64 %v2) unnamed_addr #5 align 32 {
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
define internal fastcc { i64, i64 } @mech_compact_numeric_362_vector(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
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
define internal fastcc { i64, i64 } @mech_compact_numeric_3872(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 96
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 104
  %v6 = load i64, ptr %t3, align 1
  %v15 = load i64, ptr %t0, align 1
  %t6 = getelementptr i8, ptr %t0, i64 8
  %v16 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 16
  %v17 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 24
  %v18 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 32
  %v19 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 40
  %v20 = load i64, ptr %t14, align 1
  %t20 = getelementptr i8, ptr %t0, i64 64
  %v23 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 72
  %v24 = load i64, ptr %t22, align 1
  %t24 = getelementptr i8, ptr %t0, i64 80
  %v25 = load i64, ptr %t24, align 1
  %t26 = getelementptr i8, ptr %t0, i64 88
  %v26 = load i64, ptr %t26, align 1
  %umax = tail call i64 @llvm.umax.i64(i64 %v17, i64 %v16)
  br label %block6

block6:                                           ; preds = %edge73, %block0
  %v320 = phi i64 [ %v17, %block0 ], [ %v268, %edge73 ]
  %v321 = phi i64 [ %v18, %block0 ], [ %v4966, %edge73 ]
  %exitcond.not = icmp eq i64 %v320, %umax
  br i1 %exitcond.not, label %edge81, label %edge17

common.ret:                                       ; preds = %edge54, %edge81
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge81 ], [ { i64 103169, i64 3925 }, %edge54 ]
  ret { i64, i64 } %common.ret.op

edge17:                                           ; preds = %block6
  %v81 = add i64 %v320, %v15
  %t29 = icmp ult i64 %v81, %v20
  br i1 %t29, label %edge41, label %edge73

edge41:                                           ; preds = %edge17
  %v128 = shl i64 %v81, 2
  %v130 = add i64 %v128, %v19
  %t31 = inttoptr i64 %v130 to ptr
  %t32 = load i32, ptr %t31, align 1
  %v131 = zext i32 %t32 to i64
  %t33.not = icmp eq i32 %t32, 0
  %v169 = shl nuw nsw i64 %v131, 10
  %v174 = or i64 %v169, %v320
  br i1 %t33.not, label %edge73, label %edge51

edge51:                                           ; preds = %edge41
  %t36 = icmp ult i64 %v321, %v24
  br i1 %t36, label %edge53, label %edge54

edge53:                                           ; preds = %edge51
  %v200 = add nuw i64 %v321, 1
  %v210 = shl i64 %v321, 2
  %v212 = add i64 %v210, %v23
  %v214 = trunc i64 %v174 to i32
  %t38 = inttoptr i64 %v212 to ptr
  store i32 %v214, ptr %t38, align 1
  br label %edge73

edge54:                                           ; preds = %edge51
  %t39 = inttoptr i64 %v2 to ptr
  store i64 3925, ptr %t39, align 1
  br label %common.ret

edge73:                                           ; preds = %edge41, %edge53, %edge17
  %v4966 = phi i64 [ %v321, %edge17 ], [ %v200, %edge53 ], [ %v321, %edge41 ]
  %v268 = add i64 %v320, 1
  br label %block6

edge81:                                           ; preds = %block6
  %t44 = inttoptr i64 %v2 to ptr
  %t45 = getelementptr i8, ptr %t44, i64 8
  store i64 %v23, ptr %t45, align 1
  %t47 = getelementptr i8, ptr %t44, i64 16
  store i64 %v24, ptr %t47, align 1
  %t49 = getelementptr i8, ptr %t44, i64 24
  store i64 %v25, ptr %t49, align 1
  %t51 = getelementptr i8, ptr %t44, i64 32
  store i64 %v26, ptr %t51, align 1
  %t53 = getelementptr i8, ptr %t44, i64 40
  store i64 %v321, ptr %t53, align 1
  %t55 = getelementptr i8, ptr %t44, i64 48
  store i64 %v23, ptr %t55, align 1
  %t57 = getelementptr i8, ptr %t44, i64 56
  store i64 %v24, ptr %t57, align 1
  %t59 = getelementptr i8, ptr %t44, i64 64
  store i64 %v6, ptr %t59, align 1
  %t61 = getelementptr i8, ptr %t44, i64 72
  store i64 %v25, ptr %t61, align 1
  %t63 = getelementptr i8, ptr %t44, i64 80
  store i64 %v26, ptr %t63, align 1
  %t65 = getelementptr i8, ptr %t44, i64 88
  store i64 %v5, ptr %t65, align 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_3956_vector(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %v15 = load i64, ptr %t0, align 1
  %t2 = getelementptr i8, ptr %t0, i64 8
  %v16 = load i64, ptr %t2, align 1
  %t4 = getelementptr i8, ptr %t0, i64 16
  %v17 = load i64, ptr %t4, align 1
  %t6 = getelementptr i8, ptr %t0, i64 24
  %v18 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 32
  %v19 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 40
  %v20 = load i64, ptr %t10, align 1
  br label %block6

block6:                                           ; preds = %block147, %block0
  %v535 = phi i64 [ %v16, %block0 ], [ %v497, %block147 ]
  %v47 = and i64 %v535, 255
  %t11 = icmp ne i64 %v47, 0
  %t12 = icmp ult i64 %v47, %v15
  %v641 = and i1 %t11, %t12
  br i1 %v641, label %block24, label %block147

block24:                                          ; preds = %block6, %edge130
  %v544 = phi i64 [ %v448, %edge130 ], [ %v47, %block6 ]
  %exitcond.not = icmp eq i64 %v544, %v15
  br i1 %exitcond.not, label %block147, label %edge39

block59:                                          ; preds = %edge110, %edge53
  %v575 = phi i64 [ %v544, %edge53 ], [ %spec.select, %edge110 ]
  %t20.not = icmp ult i64 %v575, %v47
  %v247 = sub i64 %v575, %v47
  %spec.select = select i1 %t20.not, i64 %v575, i64 %v247
  br i1 %t20.not, label %block114, label %edge87

block114:                                         ; preds = %edge92, %block59
  %v618 = phi i64 [ %v319, %edge92 ], [ %v571, %block59 ]
  %t32 = icmp ult i64 %v575, %v18
  br i1 %t32, label %edge110, label %edge111

block114.thread:                                  ; preds = %edge87
  %t324 = icmp ult i64 %v575, %v18
  br i1 %t324, label %edge110.thread, label %edge111

edge110.thread:                                   ; preds = %block114.thread
  %v3879 = shl i64 %v575, 2
  %v38910 = add i64 %v3879, %v17
  %v39111 = trunc nuw i64 %v571 to i32
  %t3412 = inttoptr i64 %v38910 to ptr
  store i32 %v39111, ptr %t3412, align 1
  br label %edge130

block147:                                         ; preds = %block24, %block6
  %v497 = lshr i64 %v535, 8
  br i1 %t11, label %block6, label %edge147

common.ret:                                       ; preds = %edge111, %edge147
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge147 ], [ { i64 103169, i64 4070 }, %edge111 ]
  ret { i64, i64 } %common.ret.op

edge39:                                           ; preds = %block24
  %t16 = icmp ult i64 %v544, %v18
  br i1 %t16, label %edge45, label %edge53

edge45:                                           ; preds = %edge39
  %v172 = shl i64 %v544, 2
  %v174 = add i64 %v172, %v17
  %t18 = inttoptr i64 %v174 to ptr
  %t19 = load i32, ptr %t18, align 1
  %v175 = zext i32 %t19 to i64
  br label %edge53

edge53:                                           ; preds = %edge39, %edge45
  %v571 = phi i64 [ %v175, %edge45 ], [ 0, %edge39 ]
  br label %block59

edge87:                                           ; preds = %block59
  %t26 = icmp ult i64 %v247, %v18
  br i1 %t26, label %edge92, label %block114.thread

edge92:                                           ; preds = %edge87
  %v316 = shl i64 %v247, 2
  %v318 = add i64 %v316, %v17
  %t28 = inttoptr i64 %v318 to ptr
  %t29 = load i32, ptr %t28, align 1
  %v319 = zext i32 %t29 to i64
  br label %block114

edge110:                                          ; preds = %block114
  %t30.not = icmp ugt i64 %v618, %v571
  %v618.v571 = tail call i64 @llvm.umax.i64(i64 %v618, i64 %v571)
  %v387 = shl i64 %v575, 2
  %v389 = add i64 %v387, %v17
  %v391 = trunc nuw i64 %v618.v571 to i32
  %t34 = inttoptr i64 %v389 to ptr
  store i32 %v391, ptr %t34, align 1
  br i1 %t30.not, label %block59, label %edge130

edge111:                                          ; preds = %block114.thread, %block114
  %t35 = inttoptr i64 %v2 to ptr
  store i64 4070, ptr %t35, align 1
  br label %common.ret

edge130:                                          ; preds = %edge110, %edge110.thread
  %v448 = add i64 %v544, 1
  br label %block24

edge147:                                          ; preds = %block147
  %t39 = inttoptr i64 %v2 to ptr
  %t40 = getelementptr i8, ptr %t39, i64 8
  store i64 %v17, ptr %t40, align 1
  %t42 = getelementptr i8, ptr %t39, i64 16
  store i64 %v18, ptr %t42, align 1
  %t44 = getelementptr i8, ptr %t39, i64 24
  store i64 %v19, ptr %t44, align 1
  %t46 = getelementptr i8, ptr %t39, i64 32
  store i64 %v20, ptr %t46, align 1
  %t48 = getelementptr i8, ptr %t39, i64 40
  store i64 %v17, ptr %t48, align 1
  %t50 = getelementptr i8, ptr %t39, i64 48
  store i64 %v18, ptr %t50, align 1
  %t52 = getelementptr i8, ptr %t0, i64 56
  %v6 = load i64, ptr %t52, align 1
  %t54 = getelementptr i8, ptr %t39, i64 56
  store i64 %v6, ptr %t54, align 1
  %t56 = getelementptr i8, ptr %t39, i64 64
  store i64 %v19, ptr %t56, align 1
  %t58 = getelementptr i8, ptr %t39, i64 72
  store i64 %v20, ptr %t58, align 1
  %t60 = getelementptr i8, ptr %t0, i64 48
  %v5 = load i64, ptr %t60, align 1
  %t62 = getelementptr i8, ptr %t39, i64 80
  store i64 %v5, ptr %t62, align 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_4173(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
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
  %.sink = phi i64 [ 4334, %edge185 ], [ 4211, %edge34 ], [ 4255, %edge80 ], [ 4286, %edge104 ], [ 4306, %edge134 ], [ 4318, %edge156 ], [ 4404, %edge176 ]
  %v26 = phi i64 [ 1, %edge185 ], [ 1, %edge34 ], [ 1, %edge80 ], [ 1, %edge104 ], [ 103169, %edge134 ], [ 103169, %edge156 ], [ 103169, %edge176 ]
  %t27 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t27, align 1
  %t121 = insertvalue { i64, i64 } poison, i64 %v26, 0
  %t122 = insertvalue { i64, i64 } %t121, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_4673_vector(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %v16 = load i64, ptr %t0, align 1
  %t2 = getelementptr i8, ptr %t0, i64 8
  %v17 = load i64, ptr %t2, align 1
  %t4 = getelementptr i8, ptr %t0, i64 16
  %v18 = load i64, ptr %t4, align 1
  %t6 = getelementptr i8, ptr %t0, i64 24
  %v19 = load i64, ptr %t6, align 1
  %t12 = getelementptr i8, ptr %t0, i64 48
  %v22 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 56
  %v23 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 64
  %v24 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 72
  %v25 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 80
  %v26 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 88
  %v27 = load i64, ptr %t22, align 1
  br label %block6

block6:                                           ; preds = %edge108, %block0
  %v412 = phi i64 [ %v16, %block0 ], [ %v375, %edge108 ]
  %v413 = phi i64 [ %v17, %block0 ], [ %v613, %edge108 ]
  %t27.not = icmp eq i64 %v412, 0
  br i1 %t27.not, label %edge116, label %edge23

block42:                                          ; preds = %edge88, %edge28
  %v432 = phi i64 [ %v413, %edge28 ], [ %v302, %edge88 ]
  %v433 = phi i64 [ %v124, %edge28 ], [ %v311, %edge88 ]
  %t32.not = icmp eq i64 %v433, 0
  br i1 %t32.not, label %edge108, label %edge57

common.ret:                                       ; preds = %edge118, %edge116
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge116 ], [ %t78, %edge118 ]
  ret { i64, i64 } %common.ret.op

edge23:                                           ; preds = %block6
  %t29 = icmp ult i64 %v412, %v27
  br i1 %t29, label %edge28, label %edge108

edge28:                                           ; preds = %edge23
  %v121 = shl i64 %v412, 3
  %v123 = add i64 %v121, %v26
  %t31 = inttoptr i64 %v123 to ptr
  %v124 = load i64, ptr %t31, align 1
  %v412.tr = trunc i64 %v412 to i32
  %v297 = shl i32 %v412.tr, 16
  br label %block42

edge57:                                           ; preds = %block42
  %t34 = icmp ult i64 %v432, %v19
  br i1 %t34, label %edge63, label %edge70

edge63:                                           ; preds = %edge57
  %v238 = shl i64 %v432, 2
  %v240 = add i64 %v238, %v18
  %t36 = inttoptr i64 %v240 to ptr
  %t37 = load i32, ptr %t36, align 1
  %0 = and i32 %t37, 1023
  %1 = zext nneg i32 %0 to i64
  br label %edge70

edge70:                                           ; preds = %edge63, %edge57
  %v468 = phi i64 [ %1, %edge63 ], [ 0, %edge57 ]
  %t39 = icmp ult i64 %v468, %v23
  br i1 %t39, label %edge77, label %edge118

edge77:                                           ; preds = %edge70
  %v293 = shl nuw nsw i64 %v468, 2
  %v295 = add i64 %v293, %v22
  %t41 = inttoptr i64 %v295 to ptr
  store i32 %v297, ptr %t41, align 1
  %t43.not = icmp eq i64 %v432, -1
  br i1 %t43.not, label %edge118, label %edge88

edge88:                                           ; preds = %edge77
  %v302 = add nuw i64 %v432, 1
  %v311 = add i64 %v433, -1
  br label %block42

edge108:                                          ; preds = %block42, %edge23
  %v613 = phi i64 [ %v413, %edge23 ], [ %v432, %block42 ]
  %v375 = add i64 %v412, -1
  br label %block6

edge116:                                          ; preds = %block6
  %t50 = inttoptr i64 %v2 to ptr
  %t51 = getelementptr i8, ptr %t50, i64 8
  store i64 %v22, ptr %t51, align 1
  %t53 = getelementptr i8, ptr %t50, i64 16
  store i64 %v23, ptr %t53, align 1
  %t55 = getelementptr i8, ptr %t50, i64 24
  store i64 %v24, ptr %t55, align 1
  %t57 = getelementptr i8, ptr %t50, i64 32
  store i64 %v25, ptr %t57, align 1
  %t59 = getelementptr i8, ptr %t50, i64 40
  store i64 %v22, ptr %t59, align 1
  %t61 = getelementptr i8, ptr %t50, i64 48
  store i64 %v23, ptr %t61, align 1
  %t63 = getelementptr i8, ptr %t0, i64 120
  %v6 = load i64, ptr %t63, align 1
  %t65 = getelementptr i8, ptr %t50, i64 56
  store i64 %v6, ptr %t65, align 1
  %t67 = getelementptr i8, ptr %t50, i64 64
  store i64 %v24, ptr %t67, align 1
  %t69 = getelementptr i8, ptr %t50, i64 72
  store i64 %v25, ptr %t69, align 1
  %t71 = getelementptr i8, ptr %t0, i64 112
  %v5 = load i64, ptr %t71, align 1
  %t73 = getelementptr i8, ptr %t50, i64 80
  store i64 %v5, ptr %t73, align 1
  br label %common.ret

edge118:                                          ; preds = %edge77, %edge70
  %.sink = phi i64 [ 4753, %edge70 ], [ 4754, %edge77 ]
  %v33 = phi i64 [ 103169, %edge70 ], [ 1, %edge77 ]
  %t42 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t42, align 1
  %t77 = insertvalue { i64, i64 } poison, i64 %v33, 0
  %t78 = insertvalue { i64, i64 } %t77, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_4799(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
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
  %.sink = phi i64 [ 4814, %edge15 ], [ 4868, %edge59 ]
  %v26 = phi i64 [ 1, %edge15 ], [ 103169, %edge59 ]
  %t27 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t27, align 1
  %t66 = insertvalue { i64, i64 } poison, i64 %v26, 0
  %t67 = insertvalue { i64, i64 } %t66, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_4879(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %t1 = getelementptr i8, ptr %t0, i64 112
  %v5 = load i64, ptr %t1, align 1
  %t3 = getelementptr i8, ptr %t0, i64 120
  %v6 = load i64, ptr %t3, align 1
  %t5 = getelementptr i8, ptr %t0, i64 128
  %v7 = load i64, ptr %t5, align 1
  %t7 = getelementptr i8, ptr %t0, i64 136
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
  %t24 = getelementptr i8, ptr %t0, i64 64
  %v29 = load i64, ptr %t24, align 1
  %t26 = getelementptr i8, ptr %t0, i64 72
  %v30 = load i64, ptr %t26, align 1
  %t28 = getelementptr i8, ptr %t0, i64 80
  %v31 = load i64, ptr %t28, align 1
  %t30 = getelementptr i8, ptr %t0, i64 88
  %v32 = load i64, ptr %t30, align 1
  %umax = tail call i64 @llvm.umax.i64(i64 %v21, i64 %v22)
  br label %block6

block6:                                           ; preds = %edge159, %block0
  %v586 = phi i64 [ %v22, %block0 ], [ %v531, %edge159 ]
  %exitcond.not = icmp eq i64 %v586, %umax
  br i1 %exitcond.not, label %edge166, label %edge23

common.ret:                                       ; preds = %edge168, %edge166
  %common.ret.op = phi { i64, i64 } [ zeroinitializer, %edge166 ], [ %t111, %edge168 ]
  ret { i64, i64 } %common.ret.op

edge23:                                           ; preds = %block6
  %t37 = icmp ult i64 %v586, %v24
  br i1 %t37, label %edge36, label %edge159

edge36:                                           ; preds = %edge23
  %v126 = shl i64 %v586, 2
  %v128 = add i64 %v126, %v23
  %t39 = inttoptr i64 %v128 to ptr
  %t40 = load i32, ptr %t39, align 1
  %0 = lshr i32 %t40, 16
  %1 = zext nneg i32 %0 to i64
  %t42 = icmp eq i32 %0, 0
  br i1 %t42, label %edge159, label %edge56

edge56:                                           ; preds = %edge36
  %t44 = icmp ugt i64 %v28, %1
  br i1 %t44, label %edge68, label %edge168

edge68:                                           ; preds = %edge56
  %v235 = shl nuw nsw i64 %1, 2
  %v237 = add i64 %v235, %v27
  %t46 = inttoptr i64 %v237 to ptr
  %t47 = load i32, ptr %t46, align 1
  %v238 = zext i32 %t47 to i64
  %t48.not = icmp eq i32 %t47, -1
  br i1 %t48.not, label %edge168, label %edge74

edge74:                                           ; preds = %edge68
  %v278 = add nuw i32 %t47, 1
  store i32 %v278, ptr %t46, align 1
  %v285 = and i64 %v238, 255
  %t55 = icmp ult i64 %v285, %v32
  br i1 %t55, label %edge95, label %edge102

edge95:                                           ; preds = %edge74
  %v334 = add i64 %v285, %v31
  %t57 = inttoptr i64 %v334 to ptr
  %t58 = load i8, ptr %t57, align 1
  %v335 = zext i8 %t58 to i64
  %2 = shl nuw nsw i64 %v335, 8
  br label %edge102

edge102:                                          ; preds = %edge95, %edge74
  %v650 = phi i64 [ %2, %edge95 ], [ 0, %edge74 ]
  %v357 = lshr i64 %v238, 8
  %t60 = icmp ult i64 %v357, %v32
  br i1 %t60, label %edge116, label %edge123

edge116:                                          ; preds = %edge102
  %v406 = add i64 %v357, %v31
  %t62 = inttoptr i64 %v406 to ptr
  %t63 = load i8, ptr %t62, align 1
  %v407 = zext i8 %t63 to i64
  %3 = or disjoint i64 %v650, %v407
  br label %edge123

edge123:                                          ; preds = %edge116, %edge102
  %v660 = phi i64 [ %3, %edge116 ], [ %v650, %edge102 ]
  %t65 = icmp ult i32 %t40, 1114112
  br i1 %t65, label %edge144, label %edge168

edge144:                                          ; preds = %edge123
  %v440 = sub nuw nsw i64 16, %1
  %v446 = lshr i64 %v660, %v440
  %v465 = shl nuw nsw i64 %1, 16
  %v470 = or i64 %v446, %v465
  %v507 = trunc nuw nsw i64 %v470 to i32
  store i32 %v507, ptr %t39, align 1
  br label %edge159

edge159:                                          ; preds = %edge23, %edge36, %edge144
  %v531 = add i64 %v586, 1
  br label %block6

edge166:                                          ; preds = %block6
  %t75 = inttoptr i64 %v2 to ptr
  %t76 = getelementptr i8, ptr %t75, i64 8
  store i64 %v23, ptr %t76, align 1
  %t78 = getelementptr i8, ptr %t75, i64 16
  store i64 %v24, ptr %t78, align 1
  %t80 = getelementptr i8, ptr %t75, i64 24
  store i64 %v25, ptr %t80, align 1
  %t82 = getelementptr i8, ptr %t75, i64 32
  store i64 %v26, ptr %t82, align 1
  %t84 = getelementptr i8, ptr %t75, i64 40
  store i64 %v23, ptr %t84, align 1
  %t86 = getelementptr i8, ptr %t75, i64 48
  store i64 %v24, ptr %t86, align 1
  %t88 = getelementptr i8, ptr %t75, i64 56
  store i64 %v7, ptr %t88, align 1
  %t90 = getelementptr i8, ptr %t75, i64 64
  store i64 %v25, ptr %t90, align 1
  %t92 = getelementptr i8, ptr %t75, i64 72
  store i64 %v26, ptr %t92, align 1
  %t94 = getelementptr i8, ptr %t75, i64 80
  store i64 %v5, ptr %t94, align 1
  %t96 = getelementptr i8, ptr %t75, i64 88
  store i64 %v27, ptr %t96, align 1
  %t98 = getelementptr i8, ptr %t75, i64 96
  store i64 %v28, ptr %t98, align 1
  %t100 = getelementptr i8, ptr %t75, i64 104
  store i64 %v8, ptr %t100, align 1
  %t102 = getelementptr i8, ptr %t75, i64 112
  store i64 %v29, ptr %t102, align 1
  %t104 = getelementptr i8, ptr %t75, i64 120
  store i64 %v30, ptr %t104, align 1
  %t106 = getelementptr i8, ptr %t75, i64 128
  store i64 %v6, ptr %t106, align 1
  br label %common.ret

edge168:                                          ; preds = %edge123, %edge56, %edge68
  %.sink = phi i64 [ 4952, %edge56 ], [ 4950, %edge68 ], [ 5000, %edge123 ]
  %v38 = phi i64 [ 103169, %edge56 ], [ 1, %edge68 ], [ 1, %edge123 ]
  %t50 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t50, align 1
  %t110 = insertvalue { i64, i64 } poison, i64 %v38, 0
  %t111 = insertvalue { i64, i64 } %t110, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_5895(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
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
  %.sink = phi i64 [ 5911, %edge14 ], [ 5976, %2 ]
  %t35 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t35, align 1
  %t86 = insertvalue { i64, i64 } { i64 103169, i64 poison }, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_6633(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
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
  %.mux.le = select i1 %3, i64 6707, i64 6708
  br label %edge124

edge124:                                          ; preds = %edge16, %edge74, %edge91, %edge107, %edge124.split.loop.exit
  %.sink = phi i64 [ 6705, %edge74 ], [ 6710, %edge91 ], [ 6719, %edge107 ], [ %.mux.le, %edge124.split.loop.exit ], [ 6651, %edge16 ]
  %v35 = phi i64 [ 1, %edge74 ], [ 1, %edge91 ], [ 103169, %edge107 ], [ 1, %edge124.split.loop.exit ], [ 103169, %edge16 ]
  %t33 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t33, align 1
  %t104 = insertvalue { i64, i64 } poison, i64 %v35, 0
  %t105 = insertvalue { i64, i64 } %t104, i64 %.sink, 1
  br label %common.ret
}

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable
define internal fastcc { i64, i64 } @mech_compact_numeric_8026(i64 %v1, i64 %v2) unnamed_addr #6 align 32 {
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
  %.sink = phi i64 [ 8041, %edge15 ], [ 8064, %edge41 ]
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
declare i32 @llvm.umax.i32(i32, i32) #7

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.smax.i64(i64, i64) #7

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #7

attributes #0 = { noinline uwtable "frame-pointer"="all" "target-cpu"="apple-m1" }
attributes #1 = { cold noreturn nounwind memory(inaccessiblemem: write) "target-cpu"="apple-m1" }
attributes #2 = { mustprogress nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none) "target-cpu"="apple-m1" }
attributes #3 = { "target-cpu"="apple-m1" }
attributes #4 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) "target-cpu"="apple-m1" }
attributes #5 = { noinline nounwind memory(readwrite, inaccessiblemem: write, target_mem0: none, target_mem1: none) uwtable "frame-pointer"="all" "target-cpu"="apple-m1" }
attributes #6 = { nofree noinline norecurse nosync nounwind memory(readwrite, inaccessiblemem: none, target_mem0: none, target_mem1: none) uwtable "frame-pointer"="all" "target-cpu"="apple-m1" }
attributes #7 = { nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none) }
