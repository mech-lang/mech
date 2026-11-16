; ModuleID = 'matcher.ll'
source_filename = "matcher.ll"
target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-n32:64-S128-Fn32"
target triple = "arm64-apple-macosx26.0.0"

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.cttz.i64(i64, i1 immarg) #0

; Function Attrs: mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite, inaccessiblemem: readwrite)
declare void @llvm.prefetch.p0(ptr readonly captures(none), i32 immarg, i32 immarg, i32 immarg) #1

; Function Attrs: nofree noinline norecurse nosync nounwind memory(readwrite, target_mem0: none, target_mem1: none) uwtable
define { i64, i64 } @mech_compact_numeric_475_vector(i64 %v0, i64 %v1, i64 %v2) local_unnamed_addr #2 align 128 {
block0:
  %t0 = inttoptr i64 %v1 to ptr
  %v53 = load i64, ptr %t0, align 1
  %t2 = getelementptr i8, ptr %t0, i64 8
  %v54 = load i64, ptr %t2, align 1
  %t4 = getelementptr i8, ptr %t0, i64 16
  %v55 = load i64, ptr %t4, align 1
  %t6 = getelementptr i8, ptr %t0, i64 24
  %v56 = load i64, ptr %t6, align 1
  %t8 = getelementptr i8, ptr %t0, i64 32
  %v57 = load i64, ptr %t8, align 1
  %t10 = getelementptr i8, ptr %t0, i64 40
  %v58 = load i64, ptr %t10, align 1
  %t12 = getelementptr i8, ptr %t0, i64 48
  %v59 = load i64, ptr %t12, align 1
  %t14 = getelementptr i8, ptr %t0, i64 56
  %v60 = load i64, ptr %t14, align 1
  %t16 = getelementptr i8, ptr %t0, i64 64
  %v61 = load i64, ptr %t16, align 1
  %t18 = getelementptr i8, ptr %t0, i64 72
  %v62 = load i64, ptr %t18, align 1
  %t20 = getelementptr i8, ptr %t0, i64 80
  %v63 = load i64, ptr %t20, align 1
  %t22 = getelementptr i8, ptr %t0, i64 88
  %v64 = load i64, ptr %t22, align 1
  %t24 = getelementptr i8, ptr %t0, i64 96
  %v65 = load i64, ptr %t24, align 1
  %t26 = getelementptr i8, ptr %t0, i64 104
  %v66 = load i64, ptr %t26, align 1
  %t28 = getelementptr i8, ptr %t0, i64 112
  %v67 = load i64, ptr %t28, align 1
  %t30 = getelementptr i8, ptr %t0, i64 120
  %v68 = load i64, ptr %t30, align 1
  %t32 = getelementptr i8, ptr %t0, i64 128
  %v69 = load i64, ptr %t32, align 1
  %t34 = getelementptr i8, ptr %t0, i64 136
  %v70 = load i64, ptr %t34, align 1
  %t36 = getelementptr i8, ptr %t0, i64 144
  %v71 = load i64, ptr %t36, align 1
  %t38 = getelementptr i8, ptr %t0, i64 152
  %v72 = load i64, ptr %t38, align 1
  %t40 = getelementptr i8, ptr %t0, i64 160
  %v73 = load i64, ptr %t40, align 1
  %t42 = getelementptr i8, ptr %t0, i64 168
  %v74 = load i64, ptr %t42, align 1
  %t44 = getelementptr i8, ptr %t0, i64 176
  %v75 = load i64, ptr %t44, align 1
  %t46 = getelementptr i8, ptr %t0, i64 184
  %v76 = load i64, ptr %t46, align 1
  %t48 = getelementptr i8, ptr %t0, i64 192
  %v77 = load i64, ptr %t48, align 1
  %v26665 = tail call i64 @llvm.usub.sat.i64(i64 %v77, i64 7)
  %v26657 = tail call i64 @llvm.usub.sat.i64(i64 %v77, i64 3)
  %t58 = getelementptr i8, ptr %t0, i64 216
  %v80 = load i64, ptr %t58, align 1
  %t60 = getelementptr i8, ptr %t0, i64 224
  %v81 = load i64, ptr %t60, align 1
  %t62 = getelementptr i8, ptr %t0, i64 232
  %v82 = load i64, ptr %t62, align 1
  %t64 = getelementptr i8, ptr %t0, i64 240
  %v83 = load i64, ptr %t64, align 1
  %t66 = getelementptr i8, ptr %t0, i64 248
  %v84 = load i64, ptr %t66, align 1
  %t68 = getelementptr i8, ptr %t0, i64 256
  %v85 = load i64, ptr %t68, align 1
  %t70 = getelementptr i8, ptr %t0, i64 264
  %v86 = load i64, ptr %t70, align 1
  %t72 = getelementptr i8, ptr %t0, i64 272
  %v87 = load i64, ptr %t72, align 1
  %t74 = getelementptr i8, ptr %t0, i64 280
  %v88 = load i64, ptr %t74, align 1
  %t76 = getelementptr i8, ptr %t0, i64 288
  %v89 = load i64, ptr %t76, align 1
  %t78 = getelementptr i8, ptr %t0, i64 296
  %v90 = load i64, ptr %t78, align 1
  %t80 = getelementptr i8, ptr %t0, i64 304
  %v91 = load i64, ptr %t80, align 1
  %t82 = getelementptr i8, ptr %t0, i64 312
  %v92 = load i64, ptr %t82, align 1
  %t84 = getelementptr i8, ptr %t0, i64 320
  %v93 = load i64, ptr %t84, align 1
  %t86 = getelementptr i8, ptr %t0, i64 328
  %v94 = load i64, ptr %t86, align 1
  %t88 = getelementptr i8, ptr %t0, i64 336
  %v95 = load i64, ptr %t88, align 1
  %v611 = add i64 %v68, 262144
  %v26795 = add i64 %v68, 131072
  %v810 = add i64 %v56, -32768
  %v2471 = add i64 %v68, 2
  %0 = sub i64 %v77, %v57
  br label %block34.outer.outer

block34.outer.outer:                              ; preds = %edge1837, %block0
  %v26591.ph.ph = phi i64 [ %v53, %block0 ], [ %v1155867, %edge1837 ]
  %v26592.ph.ph = phi i64 [ %v54, %block0 ], [ %v1155968, %edge1837 ]
  %v26593.ph.ph = phi i64 [ %v55, %block0 ], [ %v5713, %edge1837 ]
  %v26594.ph.ph = phi i64 [ %v58, %block0 ], [ %v5542, %edge1837 ]
  %v26595.ph.ph = phi i64 [ %v59, %block0 ], [ %v6665, %edge1837 ]
  %v26596.ph.ph = phi i64 [ %v60, %block0 ], [ 0, %edge1837 ]
  %v26597.ph.ph = phi i64 [ %v61, %block0 ], [ %v12043, %edge1837 ]
  %v26598.ph.ph = phi i64 [ %v62, %block0 ], [ 0, %edge1837 ]
  %v26599.ph.ph = phi i64 [ %v66, %block0 ], [ %v5713, %edge1837 ]
  %v26600.ph.ph = phi i64 [ %v67, %block0 ], [ 1, %edge1837 ]
  br label %block34.outer

block34.outer:                                    ; preds = %block34.outer.backedge, %block34.outer.outer
  %v26591.ph = phi i64 [ %v26591.ph.ph, %block34.outer.outer ], [ %v1155867, %block34.outer.backedge ]
  %v26592.ph = phi i64 [ %v26592.ph.ph, %block34.outer.outer ], [ %v1155968, %block34.outer.backedge ]
  %v26593.ph = phi i64 [ %v26593.ph.ph, %block34.outer.outer ], [ %v5713, %block34.outer.backedge ]
  %v26594.ph = phi i64 [ %v26594.ph.ph, %block34.outer.outer ], [ %v5542, %block34.outer.backedge ]
  %v26596.ph = phi i64 [ %v26596.ph.ph, %block34.outer.outer ], [ %v12042, %block34.outer.backedge ]
  %v26597.ph = phi i64 [ %v26597.ph.ph, %block34.outer.outer ], [ %v12043, %block34.outer.backedge ]
  %v26598.ph = phi i64 [ %v26598.ph.ph, %block34.outer.outer ], [ 0, %block34.outer.backedge ]
  %1 = sub i64 %v77, %v26593.ph
  br label %block34

block34:                                          ; preds = %block34.outer, %edge1397
  %indvars.iv = phi i64 [ %1, %block34.outer ], [ %indvars.iv.next, %edge1397 ]
  %v26591 = phi i64 [ %v26591.ph, %block34.outer ], [ %v1155867, %edge1397 ]
  %v26592 = phi i64 [ %v26592.ph, %block34.outer ], [ %v1155968, %edge1397 ]
  %v26593 = phi i64 [ %v26593.ph, %block34.outer ], [ %v5713, %edge1397 ]
  %v26594 = phi i64 [ %v26594.ph, %block34.outer ], [ %v5542, %edge1397 ]
  %v26596 = phi i64 [ %v26596.ph, %block34.outer ], [ %v12042, %edge1397 ]
  %v26597 = phi i64 [ %v26597.ph, %block34.outer ], [ %v12043, %edge1397 ]
  %v26598 = phi i64 [ %v26598.ph, %block34.outer ], [ %v5697, %edge1397 ]
  %umin929 = tail call i64 @llvm.umin.i64(i64 %indvars.iv, i64 258)
  %v195 = sub i64 %v77, %v26593
  %t89 = icmp eq i64 %v26593, %v57
  br i1 %t89, label %edge35, label %edge47

common.ret:                                       ; preds = %edge2331, %edge2081, %edge35
  %common.ret.op = phi { i64, i64 } [ { i64 103172, i64 510 }, %edge35 ], [ zeroinitializer, %edge2081 ], [ %t876, %edge2331 ]
  ret { i64, i64 } %common.ret.op

block422:                                         ; preds = %block422.preheader, %edge682
  %v9400 = phi i64 [ %v26608, %edge682 ], [ %v9214, %block422.preheader ]
  %v9407 = phi i64 [ %v10306, %edge682 ], [ %v1305944, %block422.preheader ]
  %v9408 = phi i64 [ %v10307, %edge682 ], [ %v1310952, %block422.preheader ]
  %v9409 = phi i64 [ %v2789, %edge682 ], [ %v1278, %block422.preheader ]
  %v9410 = phi i64 [ %v2822, %edge682 ], [ 300, %block422.preheader ]
  %v9411 = phi i64 [ %v10308, %edge682 ], [ 0, %block422.preheader ]
  %v9412 = phi i64 [ %v10309, %edge682 ], [ 0, %block422.preheader ]
  %v9413 = phi i64 [ %spec.select1, %edge682 ], [ 0, %block422.preheader ]
  %v9414 = phi i64 [ %v26610, %edge682 ], [ 3, %block422.preheader ]
  %t185 = icmp ult i64 %v9413, %spec.select
  br i1 %t185, label %edge365, label %edge1188

block508:                                         ; preds = %edge422, %edge528
  %v9487 = phi i64 [ %v1793, %edge528 ], [ %v1703, %edge422 ]
  %v1793 = add i64 %v9487, 8
  %t205.not = icmp ugt i64 %v9487, -9
  br i1 %t205.not, label %edge2331, label %edge437

block797:                                         ; preds = %edge559, %edge564, %edge365
  %v26601 = phi i64 [ %v9413, %edge365 ], [ %v2266, %edge564 ], [ %v2266, %edge559 ]
  %v26608 = phi i64 [ %v9400, %edge365 ], [ %v17084, %edge564 ], [ %v9400, %edge559 ]
  %v26610 = phi i64 [ %v9414, %edge365 ], [ %v2266, %edge564 ], [ %v9414, %edge559 ]
  %v2546 = add i64 %v26601, %spec.select27
  %t245 = icmp ult i64 %v2546, %v77
  br i1 %t245, label %edge636, label %edge634

block930:                                         ; preds = %block930.preheader, %edge1106
  %v10531 = phi i64 [ %v11328, %edge1106 ], [ %v1305, %block930.preheader ]
  %v10532 = phi i64 [ %v11329, %edge1106 ], [ %v1310, %block930.preheader ]
  %v10533 = phi i64 [ %v4537, %edge1106 ], [ %v1278, %block930.preheader ]
  %v10534 = phi i64 [ %v4570, %edge1106 ], [ 300, %block930.preheader ]
  %v10535 = phi i64 [ %v11330, %edge1106 ], [ 0, %block930.preheader ]
  %v10536 = phi i64 [ %v11331, %edge1106 ], [ 0, %block930.preheader ]
  %v10537 = phi i64 [ %spec.select2, %edge1106 ], [ 0, %block930.preheader ]
  %t270 = icmp ult i64 %v10537, %spec.select
  br i1 %t270, label %edge786, label %edge1188

block1042:                                        ; preds = %edge786, %edge864
  %v26618 = phi i64 [ %v3552, %edge864 ], [ %v10568, %edge786 ]
  %t291 = icmp ult i64 %spec.select28, %v26593
  br i1 %t291, label %edge880, label %edge2331

block1685:                                        ; preds = %edge1481, %edge1545
  %v12240 = phi i64 [ %v6160, %edge1545 ], [ 0, %edge1481 ]
  %v12241 = phi i64 [ %v6167, %edge1545 ], [ 0, %edge1481 ]
  %exitcond931.not = icmp eq i64 %v12240, 10
  br i1 %exitcond931.not, label %edge1584, label %edge1507

block1883:                                        ; preds = %edge1713
  %v6665 = add i64 %v12042, %v26595.ph.ph
  %t556.not = icmp ult i64 %v6665, %v26595.ph.ph
  br i1 %t556.not, label %edge2331, label %edge1762

block1971:                                        ; preds = %edge1461
  br i1 %v580114.not.not, label %edge1964, label %block34.outer.backedge

block34.outer.backedge:                           ; preds = %block1971, %edge1467
  br label %block34.outer

block2250:                                        ; preds = %edge2148
  %v8021 = add i64 %v12042, %v26595.ph.ph
  %t675.not = icmp ult i64 %v8021, %v26595.ph.ph
  br i1 %t675.not, label %edge2331, label %edge2081

block2273:                                        ; preds = %block2273.preheader, %edge2232
  %v14842 = phi i64 [ %v8169, %edge2232 ], [ %v14842.ph, %block2273.preheader ]
  %v8169 = add i64 %v14842, 8
  %t680.not = icmp ugt i64 %v8169, %spec.select
  br i1 %t680.not, label %edge447, label %edge2232

block2440:                                        ; preds = %block2440.preheader, %edge2296
  %v15358 = phi i64 [ %v8429, %edge2296 ], [ %v15358.ph, %block2440.preheader ]
  %v8429 = add i64 %v15358, 8
  %t688.not = icmp ugt i64 %v8429, %spec.select
  br i1 %t688.not, label %edge905, label %edge2296

block5013:                                        ; preds = %edge423, %edge2335
  %v26708 = phi i64 [ %v26732, %edge2335 ], [ %v1703, %edge423 ]
  %v26714 = add i64 %v26708, %v1574
  %v26715 = add i64 %v26708, %v1630
  %t879 = inttoptr i64 %v26714 to ptr
  %v26716 = load <2 x i64>, ptr %t879, align 1
  %t880 = inttoptr i64 %v26715 to ptr
  %v26717 = load <2 x i64>, ptr %t880, align 1
  %t882 = getelementptr i8, ptr %t879, i64 16
  %v26719 = load <2 x i64>, ptr %t882, align 1
  %t884 = getelementptr i8, ptr %t880, i64 16
  %v26720 = load <2 x i64>, ptr %t884, align 1
  %t886 = getelementptr i8, ptr %t879, i64 32
  %v26723 = load <2 x i64>, ptr %t886, align 1
  %t888 = getelementptr i8, ptr %t880, i64 32
  %v26724 = load <2 x i64>, ptr %t888, align 1
  %t890 = getelementptr i8, ptr %t879, i64 48
  %v26727 = load <2 x i64>, ptr %t890, align 1
  %t892 = getelementptr i8, ptr %t880, i64 48
  %v26728 = load <2 x i64>, ptr %t892, align 1
  %2 = icmp ne <2 x i64> %v26716, %v26717
  %3 = icmp ne <2 x i64> %v26719, %v26720
  %4 = or <2 x i1> %2, %3
  %5 = icmp ne <2 x i64> %v26723, %v26724
  %6 = or <2 x i1> %4, %5
  %7 = icmp ne <2 x i64> %v26727, %v26728
  %t893 = or <2 x i1> %6, %7
  %8 = bitcast <2 x i1> %t893 to i2
  %t894.not = icmp eq i2 %8, 0
  br i1 %t894.not, label %edge2335, label %block2273.preheader

block5015.loopexit:                               ; preds = %edge2335
  %.pre940 = sub i64 %spec.select, %v26732
  br label %block5015

block5015:                                        ; preds = %block5015.loopexit, %edge423
  %v26734.pre-phi = phi i64 [ %.pre940, %block5015.loopexit ], [ %v26711, %edge423 ]
  %v26709 = phi i64 [ %v26732, %block5015.loopexit ], [ %v1703, %edge423 ]
  %t898 = icmp ugt i64 %v26734.pre-phi, 15
  br i1 %t898, label %block5016, label %block2273.preheader

block2273.preheader:                              ; preds = %block5013, %block5016, %edge2341, %block5015
  %v14842.ph = phi i64 [ %v26743, %edge2341 ], [ %v26709, %block5015 ], [ %v26710, %block5016 ], [ %v26708, %block5013 ]
  br label %block2273

block5016:                                        ; preds = %block5015, %edge2341
  %v26710 = phi i64 [ %v26743, %edge2341 ], [ %v26709, %block5015 ]
  %v26737 = add i64 %v26710, %v1574
  %v26738 = add i64 %v26710, %v1630
  %t900 = inttoptr i64 %v26737 to ptr
  %v26739 = load <2 x i64>, ptr %t900, align 1
  %t901 = inttoptr i64 %v26738 to ptr
  %v26740 = load <2 x i64>, ptr %t901, align 1
  %t902 = icmp ne <2 x i64> %v26739, %v26740
  %9 = bitcast <2 x i1> %t902 to i2
  %t903.not = icmp eq i2 %9, 0
  br i1 %t903.not, label %edge2341, label %block2273.preheader

block5019:                                        ; preds = %edge881, %edge2347
  %v26746 = phi i64 [ %v26770, %edge2347 ], [ %v26618, %edge881 ]
  %v26752 = add i64 %v26746, %v3619
  %v26753 = add i64 %v26746, %v3468
  %t909 = inttoptr i64 %v26752 to ptr
  %v26754 = load <2 x i64>, ptr %t909, align 1
  %t910 = inttoptr i64 %v26753 to ptr
  %v26755 = load <2 x i64>, ptr %t910, align 1
  %t912 = getelementptr i8, ptr %t909, i64 16
  %v26757 = load <2 x i64>, ptr %t912, align 1
  %t914 = getelementptr i8, ptr %t910, i64 16
  %v26758 = load <2 x i64>, ptr %t914, align 1
  %t916 = getelementptr i8, ptr %t909, i64 32
  %v26761 = load <2 x i64>, ptr %t916, align 1
  %t918 = getelementptr i8, ptr %t910, i64 32
  %v26762 = load <2 x i64>, ptr %t918, align 1
  %t920 = getelementptr i8, ptr %t909, i64 48
  %v26765 = load <2 x i64>, ptr %t920, align 1
  %t922 = getelementptr i8, ptr %t910, i64 48
  %v26766 = load <2 x i64>, ptr %t922, align 1
  %10 = icmp ne <2 x i64> %v26754, %v26755
  %11 = icmp ne <2 x i64> %v26757, %v26758
  %12 = or <2 x i1> %10, %11
  %13 = icmp ne <2 x i64> %v26761, %v26762
  %14 = or <2 x i1> %12, %13
  %15 = icmp ne <2 x i64> %v26765, %v26766
  %t923 = or <2 x i1> %14, %15
  %16 = bitcast <2 x i1> %t923 to i2
  %t924.not = icmp eq i2 %16, 0
  br i1 %t924.not, label %edge2347, label %block2440.preheader

block5021.loopexit:                               ; preds = %edge2347
  %.pre = sub i64 %spec.select, %v26770
  br label %block5021

block5021:                                        ; preds = %block5021.loopexit, %edge881
  %v26772.pre-phi = phi i64 [ %.pre, %block5021.loopexit ], [ %v26749, %edge881 ]
  %v26747 = phi i64 [ %v26770, %block5021.loopexit ], [ %v26618, %edge881 ]
  %t928 = icmp ugt i64 %v26772.pre-phi, 15
  br i1 %t928, label %block5022, label %block2440.preheader

block2440.preheader:                              ; preds = %block5019, %block5022, %edge2353, %block5021
  %v15358.ph = phi i64 [ %v26781, %edge2353 ], [ %v26747, %block5021 ], [ %v26748, %block5022 ], [ %v26746, %block5019 ]
  br label %block2440

block5022:                                        ; preds = %block5021, %edge2353
  %v26748 = phi i64 [ %v26781, %edge2353 ], [ %v26747, %block5021 ]
  %v26775 = add i64 %v26748, %v3619
  %v26776 = add i64 %v26748, %v3468
  %t930 = inttoptr i64 %v26775 to ptr
  %v26777 = load <2 x i64>, ptr %t930, align 1
  %t931 = inttoptr i64 %v26776 to ptr
  %v26778 = load <2 x i64>, ptr %t931, align 1
  %t932 = icmp ne <2 x i64> %v26777, %v26778
  %17 = bitcast <2 x i1> %t932 to i2
  %t933.not = icmp eq i2 %17, 0
  br i1 %t933.not, label %edge2353, label %block2440.preheader

edge35:                                           ; preds = %block34
  %t91 = inttoptr i64 %v2 to ptr
  %t92 = getelementptr i8, ptr %t91, i64 8
  store i64 %v26591, ptr %t92, align 1
  %t94 = getelementptr i8, ptr %t91, i64 16
  store i64 %v26592, ptr %t94, align 1
  %t96 = getelementptr i8, ptr %t91, i64 24
  store i64 %v57, ptr %t96, align 1
  %t98 = getelementptr i8, ptr %t91, i64 32
  store i64 %v26594, ptr %t98, align 1
  %t100 = getelementptr i8, ptr %t91, i64 40
  store i64 %v26595.ph.ph, ptr %t100, align 1
  %t102 = getelementptr i8, ptr %t91, i64 48
  store i64 %v26596, ptr %t102, align 1
  %t104 = getelementptr i8, ptr %t91, i64 56
  store i64 %v26597, ptr %t104, align 1
  %t106 = getelementptr i8, ptr %t91, i64 64
  store i64 %v26598, ptr %t106, align 1
  %t108 = getelementptr i8, ptr %t91, i64 72
  store i64 %v63, ptr %t108, align 1
  %t110 = getelementptr i8, ptr %t91, i64 80
  store i64 %v64, ptr %t110, align 1
  %t112 = getelementptr i8, ptr %t91, i64 88
  store i64 %v65, ptr %t112, align 1
  %t114 = getelementptr i8, ptr %t91, i64 96
  store i64 %v26599.ph.ph, ptr %t114, align 1
  %t116 = getelementptr i8, ptr %t91, i64 104
  store i64 %v26600.ph.ph, ptr %t116, align 1
  %t118 = getelementptr i8, ptr %t91, i64 112
  store i64 %0, ptr %t118, align 1
  %t120 = getelementptr i8, ptr %t91, i64 120
  store i64 1, ptr %t120, align 1
  br label %common.ret

edge47:                                           ; preds = %block34
  %spec.select = tail call i64 @llvm.umin.i64(i64 %v195, i64 258)
  %v347 = lshr i64 %v26598, 32
  %t126.not.not = icmp ne i64 %v347, 0
  %t127 = icmp ugt i64 %v195, 4
  br i1 %t127, label %edge65, label %edge1192

edge65:                                           ; preds = %edge47
  %v380 = sub i64 %v26593, %v56
  %v408 = add i64 %spec.select, %v26593
  %t131.not = icmp ult i64 %v408, %v26593
  br i1 %t131.not, label %edge2331, label %edge78

edge78:                                           ; preds = %edge65
  %t134.not = icmp ugt i64 %v408, %v77
  br i1 %t134.not, label %edge2331, label %edge82

edge82:                                           ; preds = %edge78
  %v433 = add i64 %v26593, 1
  %t137 = icmp ult i64 %v433, %v26657
  br i1 %t137, label %edge98, label %edge106

edge98:                                           ; preds = %edge82
  %v484 = add i64 %v433, %v76
  %t139 = inttoptr i64 %v484 to ptr
  %t140 = load i32, ptr %t139, align 1
  %v485 = zext i32 %t140 to i64
  br label %edge106

edge106:                                          ; preds = %edge82, %edge98
  %v8880 = phi i64 [ %v485, %edge98 ], [ 0, %edge82 ]
  %v519 = and i64 %v8880, 16777215
  %v537 = mul nuw nsw i64 %v519, 506832829
  %v542 = lshr i64 %v537, 16
  %v555 = and i64 %v542, 65535
  %v563 = mul nuw nsw i64 %v8880, 506832829
  %v568 = lshr i64 %v563, 16
  %v581 = and i64 %v568, 65535
  %v601 = shl i64 %v26591, 2
  %v606 = and i64 %v601, 262140
  %v626 = add i64 %v611, %v606
  %t143 = inttoptr i64 %v626 to ptr
  %t144 = load i32, ptr %t143, align 1
  %v26786 = zext i32 %t144 to i64
  %v26793 = shl i64 %v26592, 1
  %v26794 = and i64 %v26793, 131070
  %v26796 = add i64 %v26795, %v26794
  %t145 = inttoptr i64 %v26796 to ptr
  %t146 = load i16, ptr %t145, align 1
  %v1278 = zext i16 %t146 to i64
  %v627 = and i64 %v26786, 65535
  %v643 = lshr i64 %v26786, 16
  %18 = trunc i64 %v380 to i16
  %v660 = xor i16 %18, -32768
  store i16 %v660, ptr %t143, align 1
  %v677 = trunc i32 %t144 to i16
  %t149 = getelementptr i8, ptr %t143, i64 2
  store i16 %v677, ptr %t149, align 1
  br i1 %t126.not.not, label %edge300, label %edge147

edge147:                                          ; preds = %edge106
  %t151 = icmp ugt i64 %v627, %v380
  br i1 %t151, label %edge157, label %edge300.thread

edge151:                                          ; preds = %edge197, %edge243
  %v815.pn = phi i64 [ %v985, %edge243 ], [ %v815, %edge197 ]
  %v9090 = sub i64 %v26593, %v815.pn
  %t169.not.not = icmp eq i64 %v26593, %v815.pn
  br i1 %t169.not.not, label %edge300.thread, label %edge266

edge157:                                          ; preds = %edge147
  %v767 = add i64 %v26593, %v76
  %t153 = inttoptr i64 %v767 to ptr
  %t154 = load i32, ptr %t153, align 1
  %19 = and i32 %t154, 16777215
  %v802 = zext nneg i32 %19 to i64
  %v815 = add i64 %v810, %v627
  %t155 = icmp ult i64 %v815, %v26657
  br i1 %t155, label %edge189, label %edge197

edge189:                                          ; preds = %edge157
  %v866 = add i64 %v815, %v76
  %t157 = inttoptr i64 %v866 to ptr
  %t158 = load i32, ptr %t157, align 1
  %20 = and i32 %t158, 16777215
  %21 = zext nneg i32 %20 to i64
  br label %edge197

edge197:                                          ; preds = %edge157, %edge189
  %v8974 = phi i64 [ %21, %edge189 ], [ 0, %edge157 ]
  %t159 = icmp eq i64 %v8974, %v802
  br i1 %t159, label %edge151, label %edge211

edge211:                                          ; preds = %edge197
  %t161 = icmp samesign ugt i64 %v643, %v380
  br i1 %t161, label %edge220, label %edge300.thread

edge220:                                          ; preds = %edge211
  %v985 = add i64 %v810, %v643
  %t163 = icmp ult i64 %v985, %v26657
  br i1 %t163, label %edge235, label %edge243

edge235:                                          ; preds = %edge220
  %v1036 = add i64 %v985, %v76
  %t165 = inttoptr i64 %v1036 to ptr
  %t166 = load i32, ptr %t165, align 1
  %22 = and i32 %t166, 16777215
  %23 = zext nneg i32 %22 to i64
  br label %edge243

edge243:                                          ; preds = %edge220, %edge235
  %v9055 = phi i64 [ %23, %edge235 ], [ 0, %edge220 ]
  %t167 = icmp eq i64 %v9055, %v802
  br i1 %t167, label %edge151, label %edge300.thread

edge266:                                          ; preds = %edge151
  %t172 = icmp ult i64 %v26594, %v73
  br i1 %t172, label %edge274, label %edge2331

edge274:                                          ; preds = %edge266
  %v1184 = shl i64 %v26594, 2
  %v1186 = add i64 %v1184, %v72
  %v9090.tr = trunc i64 %v9090 to i32
  %24 = shl i32 %v9090.tr, 16
  %v1188 = or disjoint i32 %24, 3
  %t174 = inttoptr i64 %v1186 to ptr
  store i32 %v1188, ptr %t174, align 1
  br label %edge300.thread

edge300:                                          ; preds = %edge106
  store i16 %v660, ptr %t145, align 1
  %v1300 = shl i64 %v380, 1
  %v1305 = and i64 %v1300, 65534
  %t179.not = icmp ult i64 %v380, %v1278
  br i1 %t179.not, label %block930.preheader, label %edge318

edge300.thread:                                   ; preds = %edge274, %edge151, %edge147, %edge243, %edge211
  %t169.not41 = phi i64 [ 1, %edge274 ], [ 0, %edge151 ], [ 0, %edge147 ], [ 0, %edge243 ], [ 0, %edge211 ]
  %v9214 = add i64 %t169.not41, %v26594
  store i16 %v660, ptr %t145, align 1
  %v1300943 = shl i64 %v380, 1
  %v1305944 = and i64 %v1300943, 65534
  %t179.not945 = icmp ult i64 %v380, %v1278
  br i1 %t179.not945, label %block422.preheader, label %edge318

edge318:                                          ; preds = %edge300.thread, %edge300
  %v1305949 = phi i64 [ %v1305944, %edge300.thread ], [ %v1305, %edge300 ]
  %v9215946 = phi i64 [ %v9214, %edge300.thread ], [ %v26594, %edge300 ]
  %v1343 = shl nuw nsw i64 %v1305949, 1
  %v1345 = add i64 %v1343, %v68
  %t181 = inttoptr i64 %v1345 to ptr
  store i16 0, ptr %t181, align 1
  %t183 = getelementptr i8, ptr %t181, i64 2
  store i16 0, ptr %t183, align 1
  br label %edge1188

block422.preheader:                               ; preds = %edge300.thread
  %v1310952 = or disjoint i64 %v1305944, 1
  %v1630 = add i64 %v26593, %v76
  %t202 = icmp ugt i64 %v195, 7
  %v26712 = add nsw i64 %spec.select, -64
  %v26735 = add nsw i64 %spec.select, -16
  br label %block422

block930.preheader:                               ; preds = %edge300
  %v1310 = or disjoint i64 %v1305, 1
  %v3304 = and i64 %v26598, 65535
  %v352 = lshr i64 %v26598, 16
  %v3310 = and i64 %v352, 65535
  %t279 = icmp samesign ugt i64 %v3310, 1
  %v3343 = add nsw i64 %v3310, -1
  %v10568 = tail call i64 @llvm.umin.i64(i64 %spec.select, i64 %v3343)
  %v3468 = add i64 %v26593, %v76
  %t295 = icmp ugt i64 %v195, 7
  %v26750 = add nsw i64 %spec.select, -64
  %v26773 = add nsw i64 %spec.select, -16
  br label %block930

edge365:                                          ; preds = %block422
  %v1529 = sub nsw i64 %v380, %v9409
  %v1534 = and i64 %v1529, 32767
  %spec.select27 = tail call i64 @llvm.usub.sat.i64(i64 %v26593, i64 %v1534)
  %v1574 = add i64 %spec.select27, %v76
  %v1611 = add i64 %v1574, %v9413
  %t192 = inttoptr i64 %v1611 to ptr
  %t193 = load i8, ptr %t192, align 1
  %v1667 = add i64 %v1630, %v9413
  %t194 = inttoptr i64 %v1667 to ptr
  %t195 = load i8, ptr %t194, align 1
  %t196 = icmp eq i8 %t193, %t195
  br i1 %t196, label %edge409, label %block797

edge409:                                          ; preds = %edge365
  %v1703 = add nuw nsw i64 %v9413, 1
  %t198 = icmp ult i64 %spec.select27, %v26593
  br i1 %t198, label %edge422, label %edge2331

edge422:                                          ; preds = %edge409
  %t201 = icmp ult i64 %v9413, %spec.select
  %v175910 = and i1 %t202, %t201
  br i1 %v175910, label %edge423, label %block508

edge423:                                          ; preds = %edge422
  %v26711 = sub nsw i64 %spec.select, %v1703
  %t877 = icmp samesign ugt i64 %v26711, 63
  br i1 %t877, label %block5013, label %block5015

edge437:                                          ; preds = %block508
  %t208.not = icmp ugt i64 %v1793, %spec.select
  br i1 %t208.not, label %edge447, label %edge503

edge447:                                          ; preds = %edge437, %block2273
  %v26545 = phi i64 [ %v14842, %block2273 ], [ %v9487, %edge437 ]
  %t210514 = icmp ult i64 %v26545, %spec.select
  br i1 %t210514, label %edge465, label %edge559

edge465:                                          ; preds = %edge447, %edge493
  %v9499515 = phi i64 [ %v2011, %edge493 ], [ %v26545, %edge447 ]
  %v1919 = add i64 %v1630, %v9499515
  %t212 = inttoptr i64 %v1919 to ptr
  %t213 = load i8, ptr %t212, align 1
  %v1975 = add i64 %v1574, %v9499515
  %t214 = inttoptr i64 %v1975 to ptr
  %t215 = load i8, ptr %t214, align 1
  %t216 = icmp eq i8 %t213, %t215
  br i1 %t216, label %edge493, label %edge559

edge493:                                          ; preds = %edge465
  %v2011 = add nsw i64 %v9499515, 1
  %exitcond.not = icmp eq i64 %v2011, %umin929
  br i1 %exitcond.not, label %edge559, label %edge465

edge503:                                          ; preds = %edge437
  %v2053 = add i64 %v9487, %v26593
  %t218 = icmp ult i64 %v2053, %v26665
  br i1 %t218, label %edge517, label %edge509

edge509:                                          ; preds = %edge503, %edge517
  %v9598 = phi i64 [ %v2105, %edge517 ], [ 0, %edge503 ]
  %v2131 = add i64 %v9487, %spec.select27
  %t221 = icmp ult i64 %v2131, %v26665
  br i1 %t221, label %edge536, label %edge528

edge517:                                          ; preds = %edge503
  %v2104 = add i64 %v2053, %v76
  %t220 = inttoptr i64 %v2104 to ptr
  %v2105 = load i64, ptr %t220, align 1
  br label %edge509

edge528:                                          ; preds = %edge509, %edge536
  %v9594 = phi i64 [ %25, %edge536 ], [ %v9598, %edge509 ]
  %t224 = icmp eq i64 %v9594, 0
  br i1 %t224, label %block508, label %edge551

edge536:                                          ; preds = %edge509
  %v2182 = add i64 %v2131, %v76
  %t223 = inttoptr i64 %v2182 to ptr
  %v2183 = load i64, ptr %t223, align 1
  %25 = xor i64 %v2183, %v9598
  br label %edge528

edge551:                                          ; preds = %edge528
  %t226 = tail call range(i64 0, 65) i64 @llvm.cttz.i64(i64 %v9594, i1 true)
  %v2249 = lshr i64 %t226, 3
  %v2254 = add nuw i64 %v2249, %v9487
  br label %edge559

edge559:                                          ; preds = %edge465, %edge493, %edge447, %edge2259, %edge551
  %v2266 = phi i64 [ %v8387, %edge2259 ], [ %v2254, %edge551 ], [ %v26545, %edge447 ], [ %v9499515, %edge465 ], [ %umin929, %edge493 ]
  %t228.not = icmp ugt i64 %v2266, %v9414
  br i1 %t228.not, label %edge566, label %block797

edge564:                                          ; preds = %edge566
  %v2304 = sub i64 %v26593, %spec.select27
  %v2315 = shl i64 %v2304, 16
  %v2309 = and i64 %v2266, 65535
  %v2325 = or disjoint i64 %v2309, %v2315
  %v2345 = shl i64 %v9400, 2
  %v2347 = add i64 %v2345, %v72
  %v2349 = trunc i64 %v2325 to i32
  %t233 = inttoptr i64 %v2347 to ptr
  store i32 %v2349, ptr %t233, align 1
  %v17084 = add nuw i64 %v9400, 1
  %t237.not = icmp ult i64 %v2266, %spec.select
  br i1 %t237.not, label %block797, label %edge609

edge566:                                          ; preds = %edge559
  %t231 = icmp ult i64 %v9400, %v73
  br i1 %t231, label %edge564, label %edge2331

edge609:                                          ; preds = %edge564
  %v2437 = shl nuw nsw i64 %v9409, 2
  %v2442 = and i64 %v2437, 131068
  %v2457 = add i64 %v2442, %v68
  %t239 = inttoptr i64 %v2457 to ptr
  %t240 = load i16, ptr %t239, align 1
  %v2473 = add i64 %v2471, %v2442
  %t241 = inttoptr i64 %v2473 to ptr
  %t242 = load i16, ptr %t241, align 1
  %v2489 = shl nuw nsw i64 %v9407, 1
  %v2491 = add i64 %v2489, %v68
  %t243 = inttoptr i64 %v2491 to ptr
  store i16 %t240, ptr %t243, align 1
  %v2508 = shl nuw nsw i64 %v9408, 1
  %v2510 = add i64 %v2508, %v68
  %t244 = inttoptr i64 %v2510 to ptr
  store i16 %t242, ptr %t244, align 1
  br label %edge1188

edge634:                                          ; preds = %block797, %edge636
  %v10184 = phi i64 [ %v2586, %edge636 ], [ 0, %block797 ]
  %v2605 = add i64 %v26601, %v26593
  %t249 = icmp ult i64 %v2605, %v77
  br i1 %t249, label %edge650, label %edge650.thread

edge650.thread:                                   ; preds = %edge634
  %v267348 = shl nuw nsw i64 %v9409, 1
  %v267849 = and i64 %v267348, 65534
  br label %edge682

edge636:                                          ; preds = %block797
  %v2585 = add i64 %v2546, %v76
  %t247 = inttoptr i64 %v2585 to ptr
  %t248 = load i8, ptr %t247, align 1
  %v2586 = zext i8 %t248 to i64
  br label %edge634

edge650:                                          ; preds = %edge634
  %v2644 = add i64 %v2605, %v76
  %t251 = inttoptr i64 %v2644 to ptr
  %t252 = load i8, ptr %t251, align 1
  %v2645 = zext i8 %t252 to i64
  %26 = icmp samesign ult i64 %v10184, %v2645
  %v2673 = shl nuw nsw i64 %v9409, 1
  %v2678 = and i64 %v2673, 65534
  %v2683 = or disjoint i64 %v2678, 1
  br i1 %26, label %edge682, label %edge675

edge675:                                          ; preds = %edge650
  br label %edge682

edge682:                                          ; preds = %edge650.thread, %edge650, %edge675
  %v10304 = phi i64 [ %v9407, %edge650 ], [ %v9408, %edge650.thread ], [ %v9408, %edge675 ]
  %v10305 = phi i64 [ %v2683, %edge650 ], [ %v267849, %edge650.thread ], [ %v2678, %edge675 ]
  %v10306 = phi i64 [ %v2683, %edge650 ], [ %v9407, %edge650.thread ], [ %v9407, %edge675 ]
  %v10307 = phi i64 [ %v9408, %edge650 ], [ %v267849, %edge650.thread ], [ %v2678, %edge675 ]
  %v10308 = phi i64 [ %v26601, %edge650 ], [ %v9411, %edge650.thread ], [ %v9411, %edge675 ]
  %v10309 = phi i64 [ %v9412, %edge650 ], [ %v26601, %edge650.thread ], [ %v26601, %edge675 ]
  %v10310 = phi i64 [ %v9412, %edge650 ], [ %v9411, %edge650.thread ], [ %v9411, %edge675 ]
  %v2767 = shl nuw nsw i64 %v10304, 1
  %v2769 = add i64 %v2767, %v68
  %v2771 = trunc nuw i64 %v9409 to i16
  %t255 = inttoptr i64 %v2769 to ptr
  store i16 %v2771, ptr %t255, align 1
  %v2786 = shl nuw nsw i64 %v10305, 1
  %v2788 = add i64 %v2786, %v68
  %t256 = inttoptr i64 %v2788 to ptr
  %t257 = load i16, ptr %t256, align 1
  %v2789 = zext i16 %t257 to i64
  %spec.select1 = tail call i64 @llvm.umin.i64(i64 %v10310, i64 %v26601)
  %v2822 = add nsw i64 %v9410, -1
  %t262 = icmp ult i64 %v380, %v2789
  %t263 = icmp ne i64 %v2822, 0
  %v2678512 = and i1 %t263, %t262
  br i1 %v2678512, label %block422, label %edge708

edge708:                                          ; preds = %edge682
  %v2868 = shl nuw nsw i64 %v10306, 1
  %v2870 = add i64 %v2868, %v68
  %t265 = inttoptr i64 %v2870 to ptr
  store i16 0, ptr %t265, align 1
  %v2887 = shl nuw nsw i64 %v10307, 1
  %v2889 = add i64 %v2887, %v68
  %t266 = inttoptr i64 %v2889 to ptr
  store i16 0, ptr %t266, align 1
  br label %edge1188

edge786:                                          ; preds = %block930
  %v3254 = sub nsw i64 %v380, %v10533
  %v3259 = and i64 %v3254, 32767
  %spec.select28 = tail call i64 @llvm.usub.sat.i64(i64 %v26593, i64 %v3259)
  %v3299 = sub i64 %v26593, %spec.select28
  %t278 = icmp eq i64 %v3299, %v3304
  %v33276 = and i1 %t279, %t278
  %t283111 = icmp ugt i64 %v3343, %v10537
  %t283 = select i1 %v33276, i1 %t283111, i1 false
  br i1 %t283, label %block1042, label %edge833

edge833:                                          ; preds = %edge786
  %v3412 = add i64 %spec.select28, %v76
  %v3449 = add i64 %v3412, %v10537
  %t285 = inttoptr i64 %v3449 to ptr
  %t286 = load i8, ptr %t285, align 1
  %v3505 = add i64 %v3468, %v10537
  %t287 = inttoptr i64 %v3505 to ptr
  %t288 = load i8, ptr %t287, align 1
  %t289 = icmp eq i8 %t286, %t288
  br i1 %t289, label %edge864, label %edge1052

edge864:                                          ; preds = %edge833
  %v3552 = add nuw nsw i64 %v10537, 1
  br label %block1042

edge866:                                          ; preds = %edge955, %edge940, %edge905, %edge1013, %edge2323
  %v10814 = phi i64 [ %v4120, %edge1013 ], [ %v8647, %edge2323 ], [ %v26569, %edge905 ], [ %umin929, %edge955 ], [ %v10681518, %edge940 ]
  %t323.not = icmp ult i64 %v10814, %spec.select
  br i1 %t323.not, label %edge1052, label %edge1031

edge880:                                          ; preds = %block1042
  %v3619 = add i64 %spec.select28, %v76
  %t294 = icmp ule i64 %v26618, %spec.select
  %v36257 = and i1 %t295, %t294
  br i1 %v36257, label %edge881, label %edge895

edge881:                                          ; preds = %edge880
  %v26749 = sub nuw nsw i64 %spec.select, %v26618
  %t907 = icmp samesign ugt i64 %v26749, 63
  br i1 %t907, label %block5019, label %block5021

edge895:                                          ; preds = %edge880, %edge990
  %v10669 = phi i64 [ %v3659, %edge990 ], [ %v26618, %edge880 ]
  %v3659 = add nuw nsw i64 %v10669, 8
  %t301.not = icmp ugt i64 %v3659, %spec.select
  br i1 %t301.not, label %edge905, label %edge965

edge905:                                          ; preds = %edge895, %block2440
  %v26569 = phi i64 [ %v15358, %block2440 ], [ %v10669, %edge895 ]
  %t303517 = icmp ult i64 %v26569, %spec.select
  br i1 %t303517, label %edge923, label %edge866

edge923:                                          ; preds = %edge905, %edge955
  %v10681518 = phi i64 [ %v3877, %edge955 ], [ %v26569, %edge905 ]
  %v3785 = add i64 %v3468, %v10681518
  %t305 = inttoptr i64 %v3785 to ptr
  %t306 = load i8, ptr %t305, align 1
  %v3786 = zext i8 %t306 to i64
  %v3804 = add i64 %v10681518, %spec.select28
  %t307 = icmp ult i64 %v3804, %v77
  br i1 %t307, label %edge942, label %edge940

edge940:                                          ; preds = %edge923, %edge942
  %v10724 = phi i64 [ %v3842, %edge942 ], [ 0, %edge923 ]
  %t311 = icmp eq i64 %v10724, %v3786
  br i1 %t311, label %edge955, label %edge866

edge942:                                          ; preds = %edge923
  %v3841 = add i64 %v3804, %v76
  %t309 = inttoptr i64 %v3841 to ptr
  %t310 = load i8, ptr %t309, align 1
  %v3842 = zext i8 %t310 to i64
  br label %edge940

edge955:                                          ; preds = %edge940
  %v3877 = add nuw nsw i64 %v10681518, 1
  %exitcond930.not = icmp eq i64 %v3877, %umin929
  br i1 %exitcond930.not, label %edge866, label %edge923

edge965:                                          ; preds = %edge895
  %v3919 = add i64 %v10669, %v26593
  %t313 = icmp ult i64 %v3919, %v26665
  br i1 %t313, label %edge979, label %edge971

edge971:                                          ; preds = %edge965, %edge979
  %v10780 = phi i64 [ %v3971, %edge979 ], [ 0, %edge965 ]
  %v3997 = add i64 %v10669, %spec.select28
  %t316 = icmp ult i64 %v3997, %v26665
  br i1 %t316, label %edge998, label %edge990

edge979:                                          ; preds = %edge965
  %v3970 = add i64 %v3919, %v76
  %t315 = inttoptr i64 %v3970 to ptr
  %v3971 = load i64, ptr %t315, align 1
  br label %edge971

edge990:                                          ; preds = %edge971, %edge998
  %v10776 = phi i64 [ %27, %edge998 ], [ %v10780, %edge971 ]
  %t319 = icmp eq i64 %v10776, 0
  br i1 %t319, label %edge895, label %edge1013

edge998:                                          ; preds = %edge971
  %v4048 = add i64 %v3997, %v76
  %t318 = inttoptr i64 %v4048 to ptr
  %v4049 = load i64, ptr %t318, align 1
  %27 = xor i64 %v4049, %v10780
  br label %edge990

edge1013:                                         ; preds = %edge990
  %t321 = tail call range(i64 0, 65) i64 @llvm.cttz.i64(i64 %v10776, i1 true)
  %v4115 = lshr i64 %t321, 3
  %v4120 = add nuw nsw i64 %v4115, %v10669
  br label %edge866

edge1031:                                         ; preds = %edge866
  %v4176 = shl nuw nsw i64 %v10533, 2
  %v4181 = and i64 %v4176, 131068
  %v4196 = add i64 %v4181, %v68
  %t325 = inttoptr i64 %v4196 to ptr
  %t326 = load i16, ptr %t325, align 1
  %v4212 = add i64 %v2471, %v4181
  %t327 = inttoptr i64 %v4212 to ptr
  %t328 = load i16, ptr %t327, align 1
  %v4228 = shl nuw nsw i64 %v10531, 1
  %v4230 = add i64 %v4228, %v68
  %t329 = inttoptr i64 %v4230 to ptr
  store i16 %t326, ptr %t329, align 1
  %v4247 = shl nuw nsw i64 %v10532, 1
  %v4249 = add i64 %v4247, %v68
  %t330 = inttoptr i64 %v4249 to ptr
  store i16 %t328, ptr %t330, align 1
  %v4257 = shl i64 %v10814, 16
  %v4262 = or i64 %v4257, %v3299
  br label %edge1188

edge1052:                                         ; preds = %edge833, %edge866
  %v1081456 = phi i64 [ %v10814, %edge866 ], [ %v10537, %edge833 ]
  %v4294 = add i64 %v1081456, %spec.select28
  %t332 = icmp ult i64 %v4294, %v77
  br i1 %t332, label %edge1060, label %edge1058

edge1058:                                         ; preds = %edge1052, %edge1060
  %v11206 = phi i64 [ %v4334, %edge1060 ], [ 0, %edge1052 ]
  %v4353 = add i64 %v1081456, %v26593
  %t336 = icmp ult i64 %v4353, %v77
  br i1 %t336, label %edge1074, label %edge1074.thread

edge1074.thread:                                  ; preds = %edge1058
  %v442158 = shl nuw nsw i64 %v10533, 1
  %v442659 = and i64 %v442158, 65534
  br label %edge1106

edge1060:                                         ; preds = %edge1052
  %v4333 = add i64 %v4294, %v76
  %t334 = inttoptr i64 %v4333 to ptr
  %t335 = load i8, ptr %t334, align 1
  %v4334 = zext i8 %t335 to i64
  br label %edge1058

edge1074:                                         ; preds = %edge1058
  %v4392 = add i64 %v4353, %v76
  %t338 = inttoptr i64 %v4392 to ptr
  %t339 = load i8, ptr %t338, align 1
  %v4393 = zext i8 %t339 to i64
  %28 = icmp samesign ult i64 %v11206, %v4393
  %v4421 = shl nuw nsw i64 %v10533, 1
  %v4426 = and i64 %v4421, 65534
  %v4431 = or disjoint i64 %v4426, 1
  br i1 %28, label %edge1106, label %edge1099

edge1099:                                         ; preds = %edge1074
  br label %edge1106

edge1106:                                         ; preds = %edge1074.thread, %edge1074, %edge1099
  %v11326 = phi i64 [ %v10531, %edge1074 ], [ %v10532, %edge1074.thread ], [ %v10532, %edge1099 ]
  %v11327 = phi i64 [ %v4431, %edge1074 ], [ %v442659, %edge1074.thread ], [ %v4426, %edge1099 ]
  %v11328 = phi i64 [ %v4431, %edge1074 ], [ %v10531, %edge1074.thread ], [ %v10531, %edge1099 ]
  %v11329 = phi i64 [ %v10532, %edge1074 ], [ %v442659, %edge1074.thread ], [ %v4426, %edge1099 ]
  %v11330 = phi i64 [ %v1081456, %edge1074 ], [ %v10535, %edge1074.thread ], [ %v10535, %edge1099 ]
  %v11331 = phi i64 [ %v10536, %edge1074 ], [ %v1081456, %edge1074.thread ], [ %v1081456, %edge1099 ]
  %v11332 = phi i64 [ %v10536, %edge1074 ], [ %v10535, %edge1074.thread ], [ %v10535, %edge1099 ]
  %v4515 = shl nuw nsw i64 %v11326, 1
  %v4517 = add i64 %v4515, %v68
  %v4519 = trunc nuw i64 %v10533 to i16
  %t342 = inttoptr i64 %v4517 to ptr
  store i16 %v4519, ptr %t342, align 1
  %v4534 = shl nuw nsw i64 %v11327, 1
  %v4536 = add i64 %v4534, %v68
  %t343 = inttoptr i64 %v4536 to ptr
  %t344 = load i16, ptr %t343, align 1
  %v4537 = zext i16 %t344 to i64
  %spec.select2 = tail call i64 @llvm.umin.i64(i64 %v11332, i64 %v1081456)
  %v4570 = add nsw i64 %v10534, -1
  %t347 = icmp uge i64 %v380, %v4537
  %t348 = icmp eq i64 %v4570, 0
  %v45909 = or i1 %t348, %t347
  br i1 %v45909, label %edge1132, label %block930

edge1132:                                         ; preds = %edge1106
  %v4616 = shl nuw nsw i64 %v11328, 1
  %v4618 = add i64 %v4616, %v68
  %t350 = inttoptr i64 %v4618 to ptr
  store i16 0, ptr %t350, align 1
  %v4635 = shl nuw nsw i64 %v11329, 1
  %v4637 = add i64 %v4635, %v68
  %t351 = inttoptr i64 %v4637 to ptr
  store i16 0, ptr %t351, align 1
  br label %edge1188

edge1188:                                         ; preds = %block422, %block930, %edge318, %edge1132, %edge708, %edge609, %edge1031
  %v11544 = phi i64 [ %v9215946, %edge318 ], [ %v26608, %edge708 ], [ %v26594, %block930 ], [ %v26594, %edge1132 ], [ %v17084, %edge609 ], [ %v26594, %edge1031 ], [ %v9400, %block422 ]
  %v11545 = phi i64 [ 0, %edge318 ], [ 0, %edge708 ], [ 0, %block930 ], [ 0, %edge1132 ], [ 0, %edge609 ], [ %v4262, %edge1031 ], [ 0, %block422 ]
  %v26788 = shl nuw nsw i64 %v581, 1
  %v26789 = add i64 %v26795, %v26788
  %t355 = inttoptr i64 %v26789 to ptr
  tail call void @llvm.prefetch.p0(ptr %t355, i32 1, i32 3, i32 1)
  %v26791 = shl nuw nsw i64 %v555, 2
  %v26792 = add i64 %v611, %v26791
  %t356 = inttoptr i64 %v26792 to ptr
  tail call void @llvm.prefetch.p0(ptr %t356, i32 1, i32 3, i32 1)
  %t357 = icmp ugt i64 %v11544, %v26594
  br i1 %t357, label %edge1194, label %edge1192

edge1192:                                         ; preds = %edge47, %edge1188, %edge1195
  %v2042870 = phi i64 [ %v11545, %edge1195 ], [ %v11545, %edge1188 ], [ 0, %edge47 ]
  %v1156069 = phi i64 [ %v11544, %edge1195 ], [ %v11544, %edge1188 ], [ %v26594, %edge47 ]
  %v1155968 = phi i64 [ %v581, %edge1195 ], [ %v581, %edge1188 ], [ %v26592, %edge47 ]
  %v1155867 = phi i64 [ %v555, %edge1195 ], [ %v555, %edge1188 ], [ %v26591, %edge47 ]
  %v11616 = phi i64 [ %v4923, %edge1195 ], [ 0, %edge1188 ], [ 0, %edge47 ]
  %v4942 = and i64 %v11616, 65535
  %t365.not = icmp ult i64 %v26593, %v26597
  %or.cond29 = select i1 %t126.not.not, i1 true, i1 %t365.not
  br i1 %or.cond29, label %edge1364, label %edge1218

edge1194:                                         ; preds = %edge1188
  %v4915 = add i64 %v11544, -1
  %t359 = icmp ult i64 %v4915, %v73
  br i1 %t359, label %edge1195, label %edge2331

edge1195:                                         ; preds = %edge1194
  %v4920 = shl i64 %v4915, 2
  %v4922 = add i64 %v4920, %v72
  %t361 = inttoptr i64 %v4922 to ptr
  %t362 = load i32, ptr %t361, align 1
  %v4923 = zext i32 %t362 to i64
  br label %edge1192

edge1218:                                         ; preds = %edge1192
  %t367.not = icmp ult i64 %v4942, %v65
  br i1 %t367.not, label %edge1303, label %edge1224

edge1224:                                         ; preds = %edge1218
  %t369 = icmp samesign ugt i64 %v4942, 8
  %v5033 = select i1 %t369, i64 9, i64 8
  %t374 = icmp ult i64 %v5033, %v93
  br i1 %t374, label %edge1244, label %edge2331

edge1244:                                         ; preds = %edge1224
  %v5072 = shl nuw nsw i64 %v5033, 2
  %v5074 = add i64 %v5072, %v92
  %t376 = inttoptr i64 %v5074 to ptr
  %t377 = load i32, ptr %t376, align 1
  %29 = add i32 %t377, 1
  store i32 %29, ptr %t376, align 1
  %t385 = icmp ult i64 %v4942, %v85
  br i1 %t385, label %edge1276, label %edge2331

edge1276:                                         ; preds = %edge1244
  %v5135 = add i64 %v4942, %v26593
  %v5130 = add i64 %v26596, 1
  %v5172 = shl nuw nsw i64 %v4942, 2
  %v5174 = add i64 %v5172, %v84
  %t387 = inttoptr i64 %v5174 to ptr
  %t388 = load i32, ptr %t387, align 1
  %30 = add i32 %t388, 1
  store i32 %30, ptr %t387, align 1
  br label %edge1364

edge1303:                                         ; preds = %edge1218
  %t396 = icmp ult i64 %v26593, %v77
  br i1 %t396, label %edge1309, label %edge1307

edge1307:                                         ; preds = %edge1303, %edge1309
  %v11887 = phi i64 [ %v5278, %edge1309 ], [ 0, %edge1303 ]
  %v5298 = lshr i64 %v11887, 5
  %v5303 = and i64 %v5298, 6
  %v5308 = and i64 %v11887, 1
  %v5313 = or disjoint i64 %v5303, %v5308
  %t401 = icmp ult i64 %v5313, %v93
  br i1 %t401, label %edge1328, label %edge2331

edge1309:                                         ; preds = %edge1303
  %v5277 = add i64 %v26593, %v76
  %t398 = inttoptr i64 %v5277 to ptr
  %t399 = load i8, ptr %t398, align 1
  %v5278 = zext i8 %t399 to i64
  br label %edge1307

edge1328:                                         ; preds = %edge1307
  %v5350 = shl nuw nsw i64 %v5313, 2
  %v5352 = add i64 %v5350, %v92
  %t403 = inttoptr i64 %v5352 to ptr
  %t404 = load i32, ptr %t403, align 1
  %31 = add i32 %t404, 1
  store i32 %31, ptr %t403, align 1
  %v5408 = add i64 %v26596, 1
  %v5413 = add i64 %v26593, 1
  br label %edge1364

edge1364:                                         ; preds = %edge1192, %edge1328, %edge1276
  %v12042 = phi i64 [ %v5408, %edge1328 ], [ %v26596, %edge1192 ], [ %v5130, %edge1276 ]
  %v12043 = phi i64 [ %v5413, %edge1328 ], [ %v26597, %edge1192 ], [ %v5135, %edge1276 ]
  %v5457 = sub i64 %v1156069, %v26594
  %t412 = icmp ult i64 %v26593, %v77
  br i1 %t412, label %edge1372, label %edge1370

edge1370:                                         ; preds = %edge1364, %edge1372
  %v12091 = phi i64 [ %32, %edge1372 ], [ 0, %edge1364 ]
  %v5542 = add i64 %v1156069, 1
  %t417 = icmp ult i64 %v1156069, %v73
  br i1 %t417, label %edge1388, label %edge2331

edge1372:                                         ; preds = %edge1364
  %v5496 = add i64 %v26593, %v76
  %t414 = inttoptr i64 %v5496 to ptr
  %t415 = load i8, ptr %t414, align 1
  %v5497 = zext i8 %t415 to i64
  %32 = shl nuw nsw i64 %v5497, 16
  br label %edge1370

edge1388:                                         ; preds = %edge1370
  %v5516 = and i64 %v5457, 65535
  %v5527 = or disjoint i64 %v12091, %v5516
  %v5552 = shl i64 %v1156069, 2
  %v5554 = add i64 %v5552, %v72
  %v5556 = trunc nuw nsw i64 %v5527 to i32
  %t419 = inttoptr i64 %v5554 to ptr
  store i32 %v5556, ptr %t419, align 1
  br i1 %t126.not.not, label %edge1413, label %edge1399

edge1397:                                         ; preds = %edge1399, %edge1413
  %v12146 = phi i64 [ %spec.select5, %edge1399 ], [ %v5624, %edge1413 ]
  %v5633 = shl nuw nsw i64 %v4942, 16
  %v5639 = lshr i64 %v11616, 16
  %v5644 = or disjoint i64 %v5633, %v5639
  %spec.select4 = select i1 %t126.not.not, i64 %v2042870, i64 %v5644
  %t428.not = icmp eq i64 %v12146, 0
  %v5692 = shl i64 %v12146, 32
  %v5697 = or i64 %v5692, %spec.select4
  %v5713 = add i64 %v26593, 1
  %t431941 = icmp eq i64 %v5697, 0
  %t431 = select i1 %t428.not, i1 true, i1 %t431941
  %indvars.iv.next = add i64 %indvars.iv, -1
  br i1 %t431, label %edge1461, label %block34

edge1399:                                         ; preds = %edge1388
  %t422 = icmp samesign ugt i64 %v4942, 2
  %t423 = icmp samesign uge i64 %v4942, %spec.select
  %v558413 = and i1 %t422, %t423
  %v5600 = add nsw i64 %v4942, -1
  %spec.select5 = select i1 %v558413, i64 %v5600, i64 0
  br label %edge1397

edge1413:                                         ; preds = %edge1388
  %v5624 = add nsw i64 %v347, -1
  br label %edge1397

edge1461:                                         ; preds = %edge1397
  %t433 = icmp uge i64 %v5713, %v64
  %t434 = icmp ugt i64 %v5542, 1499999
  %v580114.not.not = or i1 %t433, %t434
  %t436 = icmp ult i64 %v12042, 512
  %or.cond.not = select i1 %v580114.not.not, i1 true, i1 %t436
  br i1 %or.cond.not, label %block1971, label %edge1467

edge1467:                                         ; preds = %edge1461
  %v5821 = sub i64 %v5713, %v63
  %t438 = icmp ugt i64 %v5821, 4999
  %v5837 = sub i64 %v77, %v5713
  %t440 = icmp ugt i64 %v5837, 4999
  %or.cond26 = select i1 %t438, i1 %t440, i1 false
  br i1 %or.cond26, label %edge1481, label %block34.outer.backedge

edge1481:                                         ; preds = %edge1467
  %t442.not = icmp eq i64 %v26595.ph.ph, 0
  br i1 %t442.not, label %edge1668.preheader, label %block1685

edge1507:                                         ; preds = %block1685
  %t446 = icmp ult i64 %v12240, %v89
  br i1 %t446, label %edge1513, label %edge1520

edge1513:                                         ; preds = %edge1507
  %v5974 = shl nuw nsw i64 %v12240, 2
  %v5976 = add i64 %v5974, %v88
  %t448 = inttoptr i64 %v5976 to ptr
  %t449 = load i32, ptr %t448, align 1
  %v5977 = zext i32 %t449 to i64
  br label %edge1520

edge1520:                                         ; preds = %edge1507, %edge1513
  %v12271 = phi i64 [ %v5977, %edge1513 ], [ 0, %edge1507 ]
  %umul = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v12271, i64 %v12042)
  %33 = extractvalue { i64, i1 } %umul, 1
  br i1 %33, label %edge2331, label %edge1521

edge1521:                                         ; preds = %edge1520
  %v5998 = mul i64 %v12271, %v12042
  %t459 = icmp ult i64 %v12240, %v93
  br i1 %t459, label %edge1537, label %edge1544

edge1537:                                         ; preds = %edge1521
  %v6061 = shl nuw nsw i64 %v12240, 2
  %v6063 = add i64 %v6061, %v92
  %t461 = inttoptr i64 %v6063 to ptr
  %t462 = load i32, ptr %t461, align 1
  %v6064 = zext i32 %t462 to i64
  br label %edge1544

edge1544:                                         ; preds = %edge1521, %edge1537
  %v12299 = phi i64 [ %v6064, %edge1537 ], [ 0, %edge1521 ]
  %umul15 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v12299, i64 %v26595.ph.ph)
  %34 = extractvalue { i64, i1 } %umul15, 1
  br i1 %34, label %edge2331, label %edge1545

edge1545:                                         ; preds = %edge1544
  %v6085 = mul i64 %v12299, %v26595.ph.ph
  %t470 = icmp ugt i64 %v6085, %v5998
  %v6120 = sub i64 %v6085, %v5998
  %v613576 = sub i64 %v5998, %v6085
  %v2445272 = select i1 %t470, i64 %v6120, i64 %v613576
  %v6160 = add nuw nsw i64 %v12240, 1
  %v6167 = add i64 %v2445272, %v12241
  %t480.not = icmp ult i64 %v6167, %v12241
  br i1 %t480.not, label %edge2331, label %block1685

edge1584:                                         ; preds = %block1685
  %v6200 = add i64 %v12042, %v26595.ph.ph
  %t483.not = icmp ult i64 %v6200, %v26595.ph.ph
  br i1 %t483.not, label %edge2331, label %edge1587

edge1587:                                         ; preds = %edge1584
  %t490 = icmp ult i64 %v12042, 92233720368547759
  br i1 %t490, label %edge1589, label %edge2331

edge1589:                                         ; preds = %edge1587
  %v6210 = mul nuw i64 %v12042, 200
  %v621716 = lshr i64 %v6210, 9
  %umul17 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v621716, i64 %v26595.ph.ph)
  %35 = extractvalue { i64, i1 } %umul17, 1
  br i1 %35, label %edge2331, label %edge1595

edge1595:                                         ; preds = %edge1589
  %v6225 = mul i64 %v621716, %v26595.ph.ph
  %t502 = icmp ult i64 %v5821, 10000
  %t503 = icmp ult i64 %v6200, 8192
  %v624218 = and i1 %t502, %t503
  br i1 %v624218, label %edge1607, label %edge1622

edge1607:                                         ; preds = %edge1595
  %v6260 = sub nuw nsw i64 8192, %v6200
  %umul19 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v6225, i64 %v6260)
  %36 = extractvalue { i64, i1 } %umul19, 1
  br i1 %36, label %edge2331, label %edge1609

edge1609:                                         ; preds = %edge1607
  %v6268 = mul i64 %v6225, %v6260
  %v627520 = lshr i64 %v6268, 13
  %v6280 = add i64 %v627520, %v6225
  %t514.not = icmp ult i64 %v6280, %v6225
  br i1 %t514.not, label %edge2331, label %edge1622

edge1622:                                         ; preds = %edge1595, %edge1609
  %v25055 = phi i64 [ %v6280, %edge1609 ], [ %v6225, %edge1595 ]
  %v630021 = lshr i64 %v5821, 12
  %umul22 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %v630021, i64 %v26595.ph.ph)
  %37 = extractvalue { i64, i1 } %umul22, 1
  br i1 %37, label %edge2331, label %edge1624

edge1624:                                         ; preds = %edge1622
  %v6308 = mul i64 %v630021, %v26595.ph.ph
  %v6313 = add i64 %v12241, %v6308
  %t526.not = icmp ult i64 %v6313, %v12241
  br i1 %t526.not, label %edge2331, label %edge1630

edge1630:                                         ; preds = %edge1624
  %t529.not = icmp ult i64 %v6313, %v25055
  br i1 %t529.not, label %edge1668.preheader, label %edge1882

edge1668.preheader:                               ; preds = %edge1481, %edge1630
  br label %edge1668

edge1668:                                         ; preds = %edge1668.preheader, %edge1713
  %v12382521 = phi i64 [ %v6550, %edge1713 ], [ 0, %edge1668.preheader ]
  %t535 = icmp ult i64 %v12382521, %v89
  br i1 %t535, label %edge1674, label %edge1681

edge1674:                                         ; preds = %edge1668
  %v6468 = shl nuw nsw i64 %v12382521, 2
  %v6470 = add i64 %v6468, %v88
  %t537 = inttoptr i64 %v6470 to ptr
  %t538 = load i32, ptr %t537, align 1
  %v6471 = zext i32 %t538 to i64
  br label %edge1681

edge1681:                                         ; preds = %edge1668, %edge1674
  %v25541 = phi i64 [ %v6471, %edge1674 ], [ 0, %edge1668 ]
  %t539 = icmp ult i64 %v12382521, %v93
  br i1 %t539, label %edge1689, label %edge1687

edge1687:                                         ; preds = %edge1681, %edge1689
  %v12427 = phi i64 [ %v6527, %edge1689 ], [ 0, %edge1681 ]
  %v6543 = add nuw nsw i64 %v12427, %v25541
  %t543 = icmp samesign ult i64 %v6543, 4294967296
  br i1 %t543, label %edge1698, label %edge2331

edge1689:                                         ; preds = %edge1681
  %v6524 = shl nuw nsw i64 %v12382521, 2
  %v6526 = add i64 %v6524, %v92
  %t541 = inttoptr i64 %v6526 to ptr
  %t542 = load i32, ptr %t541, align 1
  %v6527 = zext i32 %t542 to i64
  br label %edge1687

edge1698:                                         ; preds = %edge1687
  %v6550 = add nuw nsw i64 %v12382521, 1
  %exitcond932.not = icmp eq i64 %v12382521, %v89
  br i1 %exitcond932.not, label %edge2331, label %edge1702

edge1702:                                         ; preds = %edge1698
  %v6562 = shl nuw nsw i64 %v12382521, 2
  %v6564 = add i64 %v6562, %v88
  %v6566 = trunc nuw i64 %v6543 to i32
  %t548 = inttoptr i64 %v6564 to ptr
  store i32 %v6566, ptr %t548, align 1
  %exitcond933.not = icmp eq i64 %v12382521, %v93
  br i1 %exitcond933.not, label %edge2331, label %edge1713

edge1713:                                         ; preds = %edge1702
  %v6592 = add i64 %v6562, %v92
  %t552 = inttoptr i64 %v6592 to ptr
  store i32 0, ptr %t552, align 1
  %exitcond934.not = icmp eq i64 %v6550, 10
  br i1 %exitcond934.not, label %block1883, label %edge1668

edge1762:                                         ; preds = %block1883, %edge1837
  %v12478522 = phi i64 [ %v6935, %edge1837 ], [ 0, %block1883 ]
  %t561 = icmp ult i64 %v12478522, %v85
  br i1 %t561, label %edge1766, label %edge1837

edge1766:                                         ; preds = %edge1762
  %v6747 = shl nuw nsw i64 %v12478522, 2
  %v6749 = add i64 %v6747, %v84
  %t563 = inttoptr i64 %v6749 to ptr
  %t564 = load i32, ptr %t563, align 1
  %v6750 = zext i32 %t564 to i64
  %t565.not = icmp eq i32 %t564, 0
  br i1 %t565.not, label %edge1837, label %edge1781

edge1781:                                         ; preds = %edge1766
  %t567 = icmp ult i64 %v12478522, %v81
  br i1 %t567, label %edge1785, label %edge2331

edge1785:                                         ; preds = %edge1781
  %v6815 = add i64 %v6747, %v80
  %t569 = inttoptr i64 %v6815 to ptr
  %t570 = load i32, ptr %t569, align 1
  %v6816 = zext i32 %t570 to i64
  %v6832 = add nuw nsw i64 %v6816, %v6750
  %t571 = icmp samesign ult i64 %v6832, 4294967296
  br i1 %t571, label %edge1811, label %edge2331

edge1811:                                         ; preds = %edge1785
  %v6855 = trunc nuw i64 %v6832 to i32
  store i32 %v6855, ptr %t569, align 1
  store i32 0, ptr %t563, align 1
  br label %edge1837

edge1837:                                         ; preds = %edge1762, %edge1811, %edge1766
  %v6935 = add nuw nsw i64 %v12478522, 1
  %exitcond935.not = icmp eq i64 %v6935, 259
  br i1 %exitcond935.not, label %block34.outer.outer, label %edge1762

edge1882:                                         ; preds = %edge1630
  %t593 = icmp eq i64 %v26600.ph.ph, 1
  br i1 %t593, label %edge1898, label %edge1964

edge1898:                                         ; preds = %edge1882
  %t596.not = icmp ult i64 %v5713, %v26599.ph.ph
  br i1 %t596.not, label %edge2331, label %edge1899

edge1899:                                         ; preds = %edge1898
  %v7181 = sub nuw i64 %v5713, %v26599.ph.ph
  %t599.not523 = icmp eq i64 %v7181, 0
  br i1 %t599.not523, label %edge1964.thread, label %edge1917.lr.ph

edge1917.lr.ph:                                   ; preds = %edge1899
  %v7296 = add i64 %v72, -4
  br label %edge1917

edge1917:                                         ; preds = %edge1917.lr.ph, %edge1951
  %v13047525 = phi i64 [ %v7181, %edge1917.lr.ph ], [ %v7359, %edge1951 ]
  %v13046524 = phi i64 [ %v5542, %edge1917.lr.ph ], [ %v7324, %edge1951 ]
  %t601.not = icmp eq i64 %v13046524, 0
  br i1 %t601.not, label %edge2331, label %edge1918

edge1918:                                         ; preds = %edge1917
  %v7233 = add i64 %v13046524, -1
  %t606 = icmp ult i64 %v7233, %v73
  br i1 %t606, label %edge1933, label %edge1951

edge1933:                                         ; preds = %edge1918
  %v7293 = shl i64 %v13046524, 2
  %v7298 = add i64 %v7296, %v7293
  %t608 = inttoptr i64 %v7298 to ptr
  %t609 = load i32, ptr %t608, align 1
  %38 = and i32 %t609, 65535
  %v7317 = zext nneg i32 %38 to i64
  %t610.not = icmp ult i64 %v7233, %v7317
  br i1 %t610.not, label %edge2331, label %edge1951

edge1951:                                         ; preds = %edge1918, %edge1933
  %v26650 = phi i64 [ 0, %edge1918 ], [ %v7317, %edge1933 ]
  %v7324 = sub i64 %v7233, %v26650
  %v7359 = add i64 %v13047525, -1
  %t599.not = icmp eq i64 %v7359, 0
  br i1 %t599.not, label %edge1966, label %edge1917

edge1964:                                         ; preds = %block1971, %edge1882
  %v2663438 = phi i64 [ 1, %edge1882 ], [ 0, %block1971 ]
  br label %edge1997

edge1966:                                         ; preds = %edge1951
  %t616.not = icmp ult i64 %v5542, %v7324
  br i1 %t616.not, label %edge2331, label %edge1964.thread

edge1964.thread:                                  ; preds = %edge1899, %edge1966
  %v13046.lcssa958 = phi i64 [ %v7324, %edge1966 ], [ %v5542, %edge1899 ]
  %v7410 = sub nuw nsw i64 %v5542, %v13046.lcssa958
  br label %edge2081

edge1997:                                         ; preds = %edge1964, %edge2069
  %v13134527 = phi i64 [ 0, %edge1964 ], [ %v7705, %edge2069 ]
  %t622 = icmp ult i64 %v13134527, %v85
  br i1 %t622, label %edge2001, label %edge2069

edge2001:                                         ; preds = %edge1997
  %v7517 = shl nuw nsw i64 %v13134527, 2
  %v7519 = add i64 %v7517, %v84
  %t624 = inttoptr i64 %v7519 to ptr
  %t625 = load i32, ptr %t624, align 1
  %v7520 = zext i32 %t625 to i64
  %t626.not = icmp eq i32 %t625, 0
  br i1 %t626.not, label %edge2069, label %edge2016

edge2016:                                         ; preds = %edge2001
  %t628 = icmp ult i64 %v13134527, %v81
  br i1 %t628, label %edge2020, label %edge2331

edge2020:                                         ; preds = %edge2016
  %v7585 = add i64 %v7517, %v80
  %t630 = inttoptr i64 %v7585 to ptr
  %t631 = load i32, ptr %t630, align 1
  %v7586 = zext i32 %t631 to i64
  %v7602 = add nuw nsw i64 %v7586, %v7520
  %t632 = icmp samesign ult i64 %v7602, 4294967296
  br i1 %t632, label %edge2046, label %edge2331

edge2046:                                         ; preds = %edge2020
  %v7625 = trunc nuw i64 %v7602 to i32
  store i32 %v7625, ptr %t630, align 1
  store i32 0, ptr %t624, align 1
  br label %edge2069

edge2069:                                         ; preds = %edge1997, %edge2046, %edge2001
  %v7705 = add nuw nsw i64 %v13134527, 1
  %exitcond936.not = icmp eq i64 %v7705, 259
  br i1 %exitcond936.not, label %edge2103, label %edge1997

edge2081:                                         ; preds = %edge1964.thread, %block2250
  %v26600.ph845 = phi i64 [ %v26600.ph.ph, %block2250 ], [ 1, %edge1964.thread ]
  %v13684103 = phi i64 [ 0, %block2250 ], [ %v7410, %edge1964.thread ]
  %v2662837102 = phi i64 [ %v5542, %block2250 ], [ %v13046.lcssa958, %edge1964.thread ]
  %v2663438101 = phi i64 [ %v2663438, %block2250 ], [ 1, %edge1964.thread ]
  %v71412490100.not = phi i1 [ true, %block2250 ], [ false, %edge1964.thread ]
  %v136599299 = phi i64 [ %v5713, %block2250 ], [ %v26599.ph.ph, %edge1964.thread ]
  %v13519 = phi i64 [ %v8021, %block2250 ], [ %v26595.ph.ph, %edge1964.thread ]
  %v13521 = phi i64 [ 0, %block2250 ], [ %v12042, %edge1964.thread ]
  %t678 = icmp eq i64 %v5713, %v77
  %t679.not.not = and i1 %t678, %v71412490100.not
  %v13693 = zext i1 %t679.not.not to i64
  %t696 = inttoptr i64 %v2 to ptr
  %t697 = getelementptr i8, ptr %t696, i64 8
  store i64 %v68, ptr %t697, align 1
  %t699 = getelementptr i8, ptr %t696, i64 16
  store i64 %v69, ptr %t699, align 1
  %t701 = getelementptr i8, ptr %t696, i64 24
  store i64 %v70, ptr %t701, align 1
  %t703 = getelementptr i8, ptr %t696, i64 32
  store i64 %v71, ptr %t703, align 1
  %t705 = getelementptr i8, ptr %t696, i64 40
  store i64 %v72, ptr %t705, align 1
  %t707 = getelementptr i8, ptr %t696, i64 48
  store i64 %v73, ptr %t707, align 1
  %t709 = getelementptr i8, ptr %t696, i64 56
  store i64 %v74, ptr %t709, align 1
  %t711 = getelementptr i8, ptr %t696, i64 64
  store i64 %v75, ptr %t711, align 1
  %t713 = getelementptr i8, ptr %t696, i64 72
  store i64 %v1155867, ptr %t713, align 1
  %t715 = getelementptr i8, ptr %t696, i64 80
  store i64 %v1155968, ptr %t715, align 1
  %t717 = getelementptr i8, ptr %t696, i64 88
  store i64 %v5713, ptr %t717, align 1
  %t719 = getelementptr i8, ptr %t696, i64 96
  store i64 %v5542, ptr %t719, align 1
  %t721 = getelementptr i8, ptr %t696, i64 104
  store i64 %v56, ptr %t721, align 1
  %t723 = getelementptr i8, ptr %t696, i64 112
  store i64 %v57, ptr %t723, align 1
  %t725 = getelementptr i8, ptr %t696, i64 120
  store i64 %v80, ptr %t725, align 1
  %t727 = getelementptr i8, ptr %t696, i64 128
  store i64 %v81, ptr %t727, align 1
  %t729 = getelementptr i8, ptr %t696, i64 136
  store i64 %v82, ptr %t729, align 1
  %t731 = getelementptr i8, ptr %t696, i64 144
  store i64 %v83, ptr %t731, align 1
  %t733 = getelementptr i8, ptr %t696, i64 152
  store i64 %v84, ptr %t733, align 1
  %t735 = getelementptr i8, ptr %t696, i64 160
  store i64 %v85, ptr %t735, align 1
  %t737 = getelementptr i8, ptr %t696, i64 168
  store i64 %v86, ptr %t737, align 1
  %t739 = getelementptr i8, ptr %t696, i64 176
  store i64 %v87, ptr %t739, align 1
  %t741 = getelementptr i8, ptr %t696, i64 184
  store i64 %v88, ptr %t741, align 1
  %t743 = getelementptr i8, ptr %t696, i64 192
  store i64 %v89, ptr %t743, align 1
  %t745 = getelementptr i8, ptr %t696, i64 200
  store i64 %v90, ptr %t745, align 1
  %t747 = getelementptr i8, ptr %t696, i64 208
  store i64 %v91, ptr %t747, align 1
  %t749 = getelementptr i8, ptr %t696, i64 216
  store i64 %v92, ptr %t749, align 1
  %t751 = getelementptr i8, ptr %t696, i64 224
  store i64 %v93, ptr %t751, align 1
  %t753 = getelementptr i8, ptr %t696, i64 232
  store i64 %v94, ptr %t753, align 1
  %t755 = getelementptr i8, ptr %t696, i64 240
  store i64 %v95, ptr %t755, align 1
  %t757 = getelementptr i8, ptr %t696, i64 248
  store i64 %v13519, ptr %t757, align 1
  %t759 = getelementptr i8, ptr %t696, i64 256
  store i64 %v13521, ptr %t759, align 1
  %t761 = getelementptr i8, ptr %t696, i64 264
  store i64 %v63, ptr %t761, align 1
  %t763 = getelementptr i8, ptr %t696, i64 272
  store i64 %v136599299, ptr %t763, align 1
  %t765 = getelementptr i8, ptr %t696, i64 280
  store i64 %v2662837102, ptr %t765, align 1
  %t767 = getelementptr i8, ptr %t696, i64 288
  store i64 %v13684103, ptr %t767, align 1
  %t769 = getelementptr i8, ptr %t696, i64 296
  store i64 %v13693, ptr %t769, align 1
  %t771 = getelementptr i8, ptr %t696, i64 304
  store i64 %v2663438101, ptr %t771, align 1
  %t773 = getelementptr i8, ptr %t696, i64 312
  store i64 %v26599.ph.ph, ptr %t773, align 1
  %t775 = getelementptr i8, ptr %t696, i64 320
  store i64 %v26600.ph845, ptr %t775, align 1
  %t777 = getelementptr i8, ptr %t696, i64 328
  store i64 %v68, ptr %t777, align 1
  %t779 = getelementptr i8, ptr %t696, i64 336
  store i64 %v69, ptr %t779, align 1
  %t781 = getelementptr i8, ptr %t0, i64 392
  %v11 = load i64, ptr %t781, align 1
  %t783 = getelementptr i8, ptr %t696, i64 344
  store i64 %v11, ptr %t783, align 1
  %t785 = getelementptr i8, ptr %t696, i64 352
  store i64 %v70, ptr %t785, align 1
  %t787 = getelementptr i8, ptr %t696, i64 360
  store i64 %v71, ptr %t787, align 1
  %t789 = getelementptr i8, ptr %t0, i64 344
  %v5 = load i64, ptr %t789, align 1
  %t791 = getelementptr i8, ptr %t696, i64 368
  store i64 %v5, ptr %t791, align 1
  %t793 = getelementptr i8, ptr %t696, i64 376
  store i64 %v72, ptr %t793, align 1
  %t795 = getelementptr i8, ptr %t696, i64 384
  store i64 %v73, ptr %t795, align 1
  %t797 = getelementptr i8, ptr %t0, i64 400
  %v12 = load i64, ptr %t797, align 1
  %t799 = getelementptr i8, ptr %t696, i64 392
  store i64 %v12, ptr %t799, align 1
  %t801 = getelementptr i8, ptr %t696, i64 400
  store i64 %v74, ptr %t801, align 1
  %t803 = getelementptr i8, ptr %t696, i64 408
  store i64 %v75, ptr %t803, align 1
  %t805 = getelementptr i8, ptr %t0, i64 352
  %v6 = load i64, ptr %t805, align 1
  %t807 = getelementptr i8, ptr %t696, i64 416
  store i64 %v6, ptr %t807, align 1
  %t809 = getelementptr i8, ptr %t696, i64 424
  store i64 %v80, ptr %t809, align 1
  %t811 = getelementptr i8, ptr %t696, i64 432
  store i64 %v81, ptr %t811, align 1
  %t813 = getelementptr i8, ptr %t0, i64 408
  %v13 = load i64, ptr %t813, align 1
  %t815 = getelementptr i8, ptr %t696, i64 440
  store i64 %v13, ptr %t815, align 1
  %t817 = getelementptr i8, ptr %t696, i64 448
  store i64 %v82, ptr %t817, align 1
  %t819 = getelementptr i8, ptr %t696, i64 456
  store i64 %v83, ptr %t819, align 1
  %t821 = getelementptr i8, ptr %t0, i64 360
  %v7 = load i64, ptr %t821, align 1
  %t823 = getelementptr i8, ptr %t696, i64 464
  store i64 %v7, ptr %t823, align 1
  %t825 = getelementptr i8, ptr %t696, i64 472
  store i64 %v84, ptr %t825, align 1
  %t827 = getelementptr i8, ptr %t696, i64 480
  store i64 %v85, ptr %t827, align 1
  %t829 = getelementptr i8, ptr %t0, i64 416
  %v14 = load i64, ptr %t829, align 1
  %t831 = getelementptr i8, ptr %t696, i64 488
  store i64 %v14, ptr %t831, align 1
  %t833 = getelementptr i8, ptr %t696, i64 496
  store i64 %v86, ptr %t833, align 1
  %t835 = getelementptr i8, ptr %t696, i64 504
  store i64 %v87, ptr %t835, align 1
  %t837 = getelementptr i8, ptr %t0, i64 368
  %v8 = load i64, ptr %t837, align 1
  %t839 = getelementptr i8, ptr %t696, i64 512
  store i64 %v8, ptr %t839, align 1
  %t841 = getelementptr i8, ptr %t696, i64 520
  store i64 %v88, ptr %t841, align 1
  %t843 = getelementptr i8, ptr %t696, i64 528
  store i64 %v89, ptr %t843, align 1
  %t845 = getelementptr i8, ptr %t0, i64 424
  %v15 = load i64, ptr %t845, align 1
  %t847 = getelementptr i8, ptr %t696, i64 536
  store i64 %v15, ptr %t847, align 1
  %t849 = getelementptr i8, ptr %t696, i64 544
  store i64 %v90, ptr %t849, align 1
  %t851 = getelementptr i8, ptr %t696, i64 552
  store i64 %v91, ptr %t851, align 1
  %t853 = getelementptr i8, ptr %t0, i64 376
  %v9 = load i64, ptr %t853, align 1
  %t855 = getelementptr i8, ptr %t696, i64 560
  store i64 %v9, ptr %t855, align 1
  %t857 = getelementptr i8, ptr %t696, i64 568
  store i64 %v92, ptr %t857, align 1
  %t859 = getelementptr i8, ptr %t696, i64 576
  store i64 %v93, ptr %t859, align 1
  %t861 = getelementptr i8, ptr %t0, i64 432
  %v16 = load i64, ptr %t861, align 1
  %t863 = getelementptr i8, ptr %t696, i64 584
  store i64 %v16, ptr %t863, align 1
  %t865 = getelementptr i8, ptr %t696, i64 592
  store i64 %v94, ptr %t865, align 1
  %t867 = getelementptr i8, ptr %t696, i64 600
  store i64 %v95, ptr %t867, align 1
  %t869 = getelementptr i8, ptr %t0, i64 384
  %v10 = load i64, ptr %t869, align 1
  %t871 = getelementptr i8, ptr %t696, i64 608
  store i64 %v10, ptr %t871, align 1
  br label %common.ret

edge2103:                                         ; preds = %edge2069, %edge2148
  %v13342528 = phi i64 [ %v7906, %edge2148 ], [ 0, %edge2069 ]
  %t654 = icmp ult i64 %v13342528, %v89
  br i1 %t654, label %edge2109, label %edge2116

edge2109:                                         ; preds = %edge2103
  %v7824 = shl nuw nsw i64 %v13342528, 2
  %v7826 = add i64 %v7824, %v88
  %t656 = inttoptr i64 %v7826 to ptr
  %t657 = load i32, ptr %t656, align 1
  %v7827 = zext i32 %t657 to i64
  br label %edge2116

edge2116:                                         ; preds = %edge2103, %edge2109
  %v26204 = phi i64 [ %v7827, %edge2109 ], [ 0, %edge2103 ]
  %t658 = icmp ult i64 %v13342528, %v93
  br i1 %t658, label %edge2124, label %edge2122

edge2122:                                         ; preds = %edge2116, %edge2124
  %v13435 = phi i64 [ %v7883, %edge2124 ], [ 0, %edge2116 ]
  %v7899 = add nuw nsw i64 %v13435, %v26204
  %t662 = icmp samesign ult i64 %v7899, 4294967296
  br i1 %t662, label %edge2133, label %edge2331

edge2124:                                         ; preds = %edge2116
  %v7880 = shl nuw nsw i64 %v13342528, 2
  %v7882 = add i64 %v7880, %v92
  %t660 = inttoptr i64 %v7882 to ptr
  %t661 = load i32, ptr %t660, align 1
  %v7883 = zext i32 %t661 to i64
  br label %edge2122

edge2133:                                         ; preds = %edge2122
  %v7906 = add nuw nsw i64 %v13342528, 1
  %exitcond937.not = icmp eq i64 %v13342528, %v89
  br i1 %exitcond937.not, label %edge2331, label %edge2137

edge2137:                                         ; preds = %edge2133
  %v7918 = shl nuw nsw i64 %v13342528, 2
  %v7920 = add i64 %v7918, %v88
  %v7922 = trunc nuw i64 %v7899 to i32
  %t667 = inttoptr i64 %v7920 to ptr
  store i32 %v7922, ptr %t667, align 1
  %exitcond938.not = icmp eq i64 %v13342528, %v93
  br i1 %exitcond938.not, label %edge2331, label %edge2148

edge2148:                                         ; preds = %edge2137
  %v7948 = add i64 %v7918, %v92
  %t671 = inttoptr i64 %v7948 to ptr
  store i32 0, ptr %t671, align 1
  %exitcond939.not = icmp eq i64 %v7906, 10
  br i1 %exitcond939.not, label %block2250, label %edge2103

edge2232:                                         ; preds = %block2273
  %v8247 = add i64 %v14842, %v1630
  %t682 = inttoptr i64 %v8247 to ptr
  %v8249 = load i64, ptr %t682, align 1
  %v8314 = add i64 %v14842, %v1574
  %t683 = inttoptr i64 %v8314 to ptr
  %v8316 = load i64, ptr %t683, align 1
  %t684 = icmp eq i64 %v8316, %v8249
  br i1 %t684, label %block2273, label %edge2259

edge2259:                                         ; preds = %edge2232
  %v8342 = xor i64 %v8316, %v8249
  %t686 = tail call range(i64 0, 65) i64 @llvm.cttz.i64(i64 %v8342, i1 true)
  %v8382 = lshr i64 %t686, 3
  %v8387 = add i64 %v8382, %v14842
  br label %edge559

edge2296:                                         ; preds = %block2440
  %v8507 = add i64 %v15358, %v3468
  %t690 = inttoptr i64 %v8507 to ptr
  %v8509 = load i64, ptr %t690, align 1
  %v8574 = add i64 %v15358, %v3619
  %t691 = inttoptr i64 %v8574 to ptr
  %v8576 = load i64, ptr %t691, align 1
  %t692 = icmp eq i64 %v8576, %v8509
  br i1 %t692, label %block2440, label %edge2323

edge2323:                                         ; preds = %edge2296
  %v8602 = xor i64 %v8576, %v8509
  %t694 = tail call range(i64 0, 65) i64 @llvm.cttz.i64(i64 %v8602, i1 true)
  %v8642 = lshr i64 %t694, 3
  %v8647 = add i64 %v8642, %v15358
  br label %edge866

edge2331:                                         ; preds = %block1883, %edge1624, %edge1622, %edge1609, %edge1607, %edge1589, %edge1587, %edge1584, %edge1370, %edge1307, %edge1244, %edge1224, %edge1194, %edge266, %edge78, %edge65, %edge1545, %edge1544, %edge1520, %edge1702, %edge1698, %edge1687, %edge1781, %edge1785, %edge566, %edge409, %block1042, %block508, %edge1933, %edge1917, %edge2016, %edge2020, %edge2137, %edge2133, %edge2122, %block2250, %edge1966, %edge1898
  %.sink = phi i64 [ 2756, %block2250 ], [ 1546, %block1042 ], [ 2535, %edge1917 ], [ 2215, %edge1520 ], [ 2585, %edge1966 ], [ 586, %edge65 ], [ 2643, %edge2020 ], [ 2731, %edge2137 ], [ 2518, %edge1898 ], [ 998, %edge409 ], [ 1012, %block508 ], [ 2355, %edge1687 ], [ 2432, %edge1785 ], [ 2726, %edge2122 ], [ 2728, %edge2133 ], [ 2645, %edge2016 ], [ 2559, %edge1933 ], [ 1250, %edge566 ], [ 2434, %edge1781 ], [ 2357, %edge1698 ], [ 2360, %edge1702 ], [ 2238, %edge1544 ], [ 2260, %edge1545 ], [ 2000, %edge1224 ], [ 2020, %edge1244 ], [ 2059, %edge1307 ], [ 2094, %edge1370 ], [ 862, %edge266 ], [ 591, %edge78 ], [ 1948, %edge1194 ], [ 2280, %edge1607 ], [ 2282, %edge1609 ], [ 2288, %edge1622 ], [ 2269, %edge1587 ], [ 2268, %edge1584 ], [ 2385, %block1883 ], [ 2271, %edge1589 ], [ 2289, %edge1624 ]
  %v103 = phi i64 [ 1, %block2250 ], [ 9, %block1042 ], [ 1, %edge1933 ], [ 1, %edge1545 ], [ 1, %edge1966 ], [ 1, %edge65 ], [ 1, %edge2020 ], [ 103169, %edge2137 ], [ 1, %edge1898 ], [ 9, %edge409 ], [ 1, %block508 ], [ 1, %edge1687 ], [ 1, %edge1785 ], [ 1, %edge2122 ], [ 103169, %edge2133 ], [ 103169, %edge2016 ], [ 1, %edge1917 ], [ 103169, %edge566 ], [ 103169, %edge1781 ], [ 103169, %edge1698 ], [ 103169, %edge1702 ], [ 1, %edge1520 ], [ 1, %edge1544 ], [ 103169, %edge1224 ], [ 103169, %edge1244 ], [ 103169, %edge1307 ], [ 103169, %edge1370 ], [ 103169, %edge266 ], [ 9, %edge78 ], [ 103169, %edge1194 ], [ 1, %edge1584 ], [ 1, %edge1587 ], [ 1, %edge1589 ], [ 1, %edge1607 ], [ 1, %edge1609 ], [ 1, %edge1622 ], [ 1, %edge1624 ], [ 1, %block1883 ]
  %t133 = inttoptr i64 %v2 to ptr
  store i64 %.sink, ptr %t133, align 1
  %t875 = insertvalue { i64, i64 } poison, i64 %v103, 0
  %t876 = insertvalue { i64, i64 } %t875, i64 %.sink, 1
  br label %common.ret

edge2335:                                         ; preds = %block5013
  %v26732 = add i64 %v26708, 64
  %t896.not = icmp ugt i64 %v26732, %v26712
  br i1 %t896.not, label %block5015.loopexit, label %block5013

edge2341:                                         ; preds = %block5016
  %v26743 = add i64 %v26710, 16
  %t905.not = icmp ugt i64 %v26743, %v26735
  br i1 %t905.not, label %block2273.preheader, label %block5016

edge2347:                                         ; preds = %block5019
  %v26770 = add i64 %v26746, 64
  %t926.not = icmp ugt i64 %v26770, %v26750
  br i1 %t926.not, label %block5021.loopexit, label %block5019

edge2353:                                         ; preds = %block5022
  %v26781 = add i64 %v26748, 16
  %t935.not = icmp ugt i64 %v26781, %v26773
  br i1 %t935.not, label %block2440.preheader, label %block5022
}

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #3

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #3

; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.umul.with.overflow.i64(i64, i64) #3

attributes #0 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) "target-cpu"="apple-m1" }
attributes #1 = { mustprogress nocallback nofree nosync nounwind willreturn memory(argmem: readwrite, inaccessiblemem: readwrite) "target-cpu"="apple-m1" }
attributes #2 = { nofree noinline norecurse nosync nounwind memory(readwrite, target_mem0: none, target_mem1: none) uwtable "frame-pointer"="all" "target-cpu"="apple-m1" }
attributes #3 = { nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none) }
