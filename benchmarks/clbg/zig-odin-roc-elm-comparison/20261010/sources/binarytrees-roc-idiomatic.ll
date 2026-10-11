; ModuleID = 'app'
source_filename = "builtins-host"
target datalayout = "e-m:o-i64:64-i128:128-n32:64-S128"
target triple = "aarch64-unknown-macosx11.7.1-unknown"

%str.FromUtf8Result = type { i64, %str.RocStr, i1, i8, [6 x i8] }
%str.RocStr = type { ptr, i64, i64 }
%fmt.FormatOptions = type { { i64, i8, [7 x i8] }, { i64, i8, [7 x i8] }, i21, i2, [3 x i8] }
%macho.mach_header_64 = type { i32, i32, i32, i32, i32, i32, i32, i32 }
%"io.fixed_buffer_stream.FixedBufferStream([]u8)" = type { { ptr, i64 }, i64 }
%list.RocList = type { ptr, i64, i64 }
%io.Writer = type { ptr, ptr }

@__stack_chk_guard = weak_odr dso_local local_unnamed_addr global i64 -69242844270821376, align 8
@__anon_19998 = internal unnamed_addr constant [36 x i8] c"Integer raised to power overflowed!\00", align 1
@0 = private unnamed_addr constant { i16, i3, [1 x i8] } { i16 22, i3 undef, [1 x i8] undef }, align 2
@__anon_23980 = internal unnamed_addr constant [3 x i8] c"\EF\BF\BD", align 1
@1 = private unnamed_addr constant %str.FromUtf8Result { i64 0, %str.RocStr { ptr null, i64 0, i64 -9223372036854775808 }, i1 true, i8 3, [6 x i8] undef }, align 8
@2 = private unnamed_addr constant { i16, i3, [1 x i8] } { i16 0, i3 1, [1 x i8] undef }, align 4
@3 = private unnamed_addr constant { i16, i3, [1 x i8] } { i16 0, i3 2, [1 x i8] undef }, align 4
@4 = private unnamed_addr constant { i16, i3, [1 x i8] } { i16 0, i3 3, [1 x i8] undef }, align 4
@5 = private unnamed_addr constant { i16, i3, [1 x i8] } { i16 0, i3 -4, [1 x i8] undef }, align 4
@6 = private unnamed_addr constant %fmt.FormatOptions { { i64, i8, [7 x i8] } { i64 undef, i8 0, [7 x i8] undef }, { i64, i8, [7 x i8] } { i64 undef, i8 0, [7 x i8] undef }, i21 32, i2 -2, [3 x i8] undef }, align 8
@7 = private unnamed_addr constant { i64, i16, [6 x i8] } { i64 undef, i16 5, [6 x i8] undef }, align 8
@__anon_21180 = internal unnamed_addr constant [201 x i8] c"00010203040506070809101112131415161718192021222324252627282930313233343536373839404142434445464748495051525354555657585960616263646566676869707172737475767778798081828384858687888990919293949596979899\00", align 1
@8 = private unnamed_addr constant %str.RocStr { ptr null, i64 0, i64 -9223372036854775808 }, align 8
@9 = private unnamed_addr constant { i64, i16, [6 x i8] } { i64 undef, i16 6, [6 x i8] undef }, align 8
@_mh_execute_header = weak_odr dso_local local_unnamed_addr global %macho.mach_header_64 undef, align 4
@10 = private unnamed_addr constant [256 x i8] c"\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F0\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\02\13\03\03\03\03\03\03\03\03\03\03\03\03#\03\034\04\04\04D\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1\F1", align 1
@_str_literal_5625337945828202030 = private unnamed_addr constant [42 x i8] c"\00\00\00\00\00\00\00\00Integer multiplication overflowed!", align 8
@_str_literal_14520704672829006296 = private unnamed_addr constant [41 x i8] c"\00\00\00\00\00\00\00\00\0AProgram exited with error:\0A\0A\E2\9D\8C ", align 8
@_str_literal_6738018046837421198 = private unnamed_addr constant [53 x i8] c"\00\00\00\00\00\00\00\00tried to display Arg containing invalid utf-8", align 8
@_str_literal_7793802679861948314 = private unnamed_addr constant [48 x i8] c"\00\00\00\00\00\00\00\00display for utf-16 Arg not yet supported", align 8
@_str_literal_11270832565531248303 = private unnamed_addr constant [33 x i8] c"\00\00\00\00\00\00\00\00long lived tree of depth ", align 8
@"switch.table._main_for_host!_8445464738218c426e3b841f28ede2879d4391e6c33df6a616e872e199e47e" = private unnamed_addr constant [9 x i8] c"\03\00\02\04\01\00\00\00\05", align 1

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #0

; Function Attrs: nounwind uwtable
define weak_odr dso_local i128 @__divti3(i128 %0, i128 %1) local_unnamed_addr #1 {
  %3 = ashr i128 %0, 127
  %4 = ashr i128 %1, 127
  %5 = xor i128 %3, %0
  %6 = sub i128 %5, %3
  %7 = xor i128 %4, %1
  %8 = sub i128 %7, %4
  %9 = tail call fastcc i128 @compiler_rt.udivmod.udivmod__anon_4847(i128 %6, i128 %8, ptr align 16 null)
  %10 = xor i128 %4, %3
  %11 = xor i128 %9, %10
  %12 = sub i128 %11, %10
  ret i128 %12
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #2

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: write) uwtable
define private fastcc i128 @compiler_rt.udivmod.udivmod__anon_4847(i128 %0, i128 %1, ptr writeonly align 16 %2) unnamed_addr #3 {
  %4 = icmp ugt i128 %1, %0
  br i1 %4, label %5, label %6

5:                                                ; preds = %3
  %.not41 = icmp eq ptr %2, null
  br i1 %.not41, label %common.ret, label %8

6:                                                ; preds = %3
  %.sroa.0.0.extract.trunc = trunc i128 %0 to i64
  %.sroa.4.0.extract.shift = lshr i128 %0, 64
  %.sroa.4.0.extract.trunc = trunc i128 %.sroa.4.0.extract.shift to i64
  %.sroa.07.0.extract.trunc = trunc i128 %1 to i64
  %.sroa.7.0.extract.shift = lshr i128 %1, 64
  %.sroa.7.0.extract.trunc = trunc i128 %.sroa.7.0.extract.shift to i64
  %7 = icmp eq i64 %.sroa.7.0.extract.trunc, 0
  br i1 %7, label %9, label %11

common.ret:                                       ; preds = %5, %8, %142, %128
  %common.ret.op = phi i128 [ %.sroa.014.0.insert.insert, %128 ], [ %.sroa.014.0.insert.ext18, %142 ], [ 0, %8 ], [ 0, %5 ]
  ret i128 %common.ret.op

8:                                                ; preds = %5
  store i128 %0, ptr %2, align 16
  br label %common.ret

9:                                                ; preds = %6
  %10 = icmp ult i64 %.sroa.4.0.extract.trunc, %.sroa.07.0.extract.trunc
  br i1 %10, label %24, label %75

11:                                               ; preds = %6
  %12 = tail call i64 @llvm.ctlz.i64(i64 %.sroa.7.0.extract.trunc, i1 true), !range !1
  %13 = trunc i64 %12 to i7
  %14 = tail call i64 @llvm.ctlz.i64(i64 %.sroa.4.0.extract.trunc, i1 false), !range !1
  %15 = trunc i64 %14 to i7
  %16 = sub nuw i7 %13, %15
  %17 = zext nneg i7 %16 to i128
  %18 = shl i128 %1, %17
  %19 = add nuw i7 %16, 1
  %20 = zext i7 %19 to i64
  br label %130

21:                                               ; preds = %compiler_rt.udivmod.divwide__anon_7868.exit54, %compiler_rt.udivmod.divwide__anon_7868.exit
  %.sroa.013.0.i.i48.sink = phi i64 [ %.sroa.013.0.i.i48, %compiler_rt.udivmod.divwide__anon_7868.exit54 ], [ %.sroa.013.0.i.i, %compiler_rt.udivmod.divwide__anon_7868.exit ]
  %.sroa.025.0.i.i51.sink = phi i64 [ %.sroa.025.0.i.i51, %compiler_rt.udivmod.divwide__anon_7868.exit54 ], [ %.sroa.025.0.i.i, %compiler_rt.udivmod.divwide__anon_7868.exit ]
  %.sroa.0.0 = phi i64 [ %127, %compiler_rt.udivmod.divwide__anon_7868.exit54 ], [ %74, %compiler_rt.udivmod.divwide__anon_7868.exit ]
  %.sroa.10.0 = phi i64 [ %76, %compiler_rt.udivmod.divwide__anon_7868.exit54 ], [ 0, %compiler_rt.udivmod.divwide__anon_7868.exit ]
  %22 = shl i64 %.sroa.013.0.i.i48.sink, 32
  %23 = add i64 %.sroa.025.0.i.i51.sink, %22
  %.not40 = icmp eq ptr %2, null
  br i1 %.not40, label %128, label %129

24:                                               ; preds = %9
  %25 = tail call i64 @llvm.ctlz.i64(i64 %.sroa.07.0.extract.trunc, i1 true), !range !1
  %.not.i.i = icmp eq i64 %25, 0
  br i1 %.not.i.i, label %26, label %34

26:                                               ; preds = %34, %24
  %.sroa.011.0.i.i = phi i64 [ %41, %34 ], [ %.sroa.0.0.extract.trunc, %24 ]
  %.sroa.08.0.i.i = phi i64 [ %40, %34 ], [ %.sroa.4.0.extract.trunc, %24 ]
  %.sroa.0.0.i.i = phi i64 [ %35, %34 ], [ %.sroa.07.0.extract.trunc, %24 ]
  %27 = lshr i64 %.sroa.0.0.i.i, 32
  %28 = and i64 %.sroa.0.0.i.i, 4294967295
  %29 = lshr i64 %.sroa.011.0.i.i, 32
  %30 = and i64 %.sroa.011.0.i.i, 4294967295
  %31 = udiv i64 %.sroa.08.0.i.i, %27
  %32 = mul i64 %31, %27
  %33 = sub i64 %.sroa.08.0.i.i, %32
  br label %50

34:                                               ; preds = %24
  %35 = shl i64 %.sroa.07.0.extract.trunc, %25
  %36 = shl i64 %.sroa.4.0.extract.trunc, %25
  %37 = sub nsw i64 0, %25
  %38 = and i64 %37, 63
  %39 = lshr i64 %.sroa.0.0.extract.trunc, %38
  %40 = or i64 %39, %36
  %41 = shl i64 %.sroa.0.0.extract.trunc, %25
  br label %26

42:                                               ; preds = %.critedge.i.i, %52
  %.sroa.013.0.i.i = phi i64 [ %57, %.critedge.i.i ], [ %.sroa.013.1.i.i, %52 ]
  %43 = shl i64 %.sroa.08.0.i.i, 32
  %44 = or disjoint i64 %43, %29
  %45 = mul i64 %.sroa.013.0.i.i, %.sroa.0.0.i.i
  %46 = sub i64 %44, %45
  %47 = udiv i64 %46, %27
  %48 = mul i64 %47, %27
  %49 = sub i64 %46, %48
  br label %60

50:                                               ; preds = %.critedge.i.i, %26
  %.sroa.019.0.i.i = phi i64 [ %33, %26 ], [ %58, %.critedge.i.i ]
  %.sroa.013.1.i.i = phi i64 [ %31, %26 ], [ %57, %.critedge.i.i ]
  %51 = icmp ugt i64 %.sroa.013.1.i.i, 4294967295
  br i1 %51, label %.critedge.i.i, label %52

52:                                               ; preds = %50
  %53 = mul nuw i64 %.sroa.013.1.i.i, %28
  %54 = shl nuw i64 %.sroa.019.0.i.i, 32
  %55 = or disjoint i64 %54, %29
  %56 = icmp ugt i64 %53, %55
  br i1 %56, label %.critedge.i.i, label %42

.critedge.i.i:                                    ; preds = %52, %50
  %57 = add i64 %.sroa.013.1.i.i, -1
  %58 = add nuw i64 %.sroa.019.0.i.i, %27
  %59 = icmp ugt i64 %58, 4294967295
  br i1 %59, label %42, label %50

60:                                               ; preds = %.critedge2.i.i, %42
  %.sroa.025.1.i.i = phi i64 [ %47, %42 ], [ %67, %.critedge2.i.i ]
  %.sroa.019.1.i.i = phi i64 [ %49, %42 ], [ %68, %.critedge2.i.i ]
  %61 = icmp ugt i64 %.sroa.025.1.i.i, 4294967295
  br i1 %61, label %.critedge2.i.i, label %62

62:                                               ; preds = %60
  %63 = mul nuw i64 %.sroa.025.1.i.i, %28
  %64 = shl nuw i64 %.sroa.019.1.i.i, 32
  %65 = or disjoint i64 %64, %30
  %66 = icmp ugt i64 %63, %65
  br i1 %66, label %.critedge2.i.i, label %compiler_rt.udivmod.divwide__anon_7868.exit

.critedge2.i.i:                                   ; preds = %62, %60
  %67 = add i64 %.sroa.025.1.i.i, -1
  %68 = add nuw i64 %.sroa.019.1.i.i, %27
  %69 = icmp ugt i64 %68, 4294967295
  br i1 %69, label %compiler_rt.udivmod.divwide__anon_7868.exit, label %60

compiler_rt.udivmod.divwide__anon_7868.exit:      ; preds = %62, %.critedge2.i.i
  %.sroa.025.0.i.i = phi i64 [ %67, %.critedge2.i.i ], [ %.sroa.025.1.i.i, %62 ]
  %70 = shl i64 %46, 32
  %71 = or disjoint i64 %70, %30
  %72 = mul i64 %.sroa.025.0.i.i, %.sroa.0.0.i.i
  %73 = sub i64 %71, %72
  %74 = lshr i64 %73, %25
  br label %21

75:                                               ; preds = %9
  %.sroa.4.0.extract.trunc.frozen = freeze i64 %.sroa.4.0.extract.trunc
  %.sroa.07.0.extract.trunc.frozen = freeze i64 %.sroa.07.0.extract.trunc
  %76 = udiv i64 %.sroa.4.0.extract.trunc.frozen, %.sroa.07.0.extract.trunc.frozen
  %77 = mul i64 %76, %.sroa.07.0.extract.trunc.frozen
  %.decomposed = sub i64 %.sroa.4.0.extract.trunc.frozen, %77
  %78 = tail call i64 @llvm.ctlz.i64(i64 %.sroa.07.0.extract.trunc, i1 true), !range !1
  %.not.i.i42 = icmp eq i64 %78, 0
  br i1 %.not.i.i42, label %79, label %87

79:                                               ; preds = %87, %75
  %.sroa.011.0.i.i43 = phi i64 [ %94, %87 ], [ %.sroa.0.0.extract.trunc, %75 ]
  %.sroa.08.0.i.i44 = phi i64 [ %93, %87 ], [ %.decomposed, %75 ]
  %.sroa.0.0.i.i45 = phi i64 [ %88, %87 ], [ %.sroa.07.0.extract.trunc, %75 ]
  %80 = lshr i64 %.sroa.0.0.i.i45, 32
  %81 = and i64 %.sroa.0.0.i.i45, 4294967295
  %82 = lshr i64 %.sroa.011.0.i.i43, 32
  %83 = and i64 %.sroa.011.0.i.i43, 4294967295
  %84 = udiv i64 %.sroa.08.0.i.i44, %80
  %85 = mul i64 %84, %80
  %86 = sub i64 %.sroa.08.0.i.i44, %85
  br label %103

87:                                               ; preds = %75
  %88 = shl i64 %.sroa.07.0.extract.trunc, %78
  %89 = shl i64 %.decomposed, %78
  %90 = sub nsw i64 0, %78
  %91 = and i64 %90, 63
  %92 = lshr i64 %.sroa.0.0.extract.trunc, %91
  %93 = or i64 %89, %92
  %94 = shl i64 %.sroa.0.0.extract.trunc, %78
  br label %79

95:                                               ; preds = %.critedge.i.i53, %105
  %.sroa.013.0.i.i48 = phi i64 [ %110, %.critedge.i.i53 ], [ %.sroa.013.1.i.i47, %105 ]
  %96 = shl i64 %.sroa.08.0.i.i44, 32
  %97 = or disjoint i64 %96, %82
  %98 = mul i64 %.sroa.013.0.i.i48, %.sroa.0.0.i.i45
  %99 = sub i64 %97, %98
  %100 = udiv i64 %99, %80
  %101 = mul i64 %100, %80
  %102 = sub i64 %99, %101
  br label %113

103:                                              ; preds = %.critedge.i.i53, %79
  %.sroa.019.0.i.i46 = phi i64 [ %86, %79 ], [ %111, %.critedge.i.i53 ]
  %.sroa.013.1.i.i47 = phi i64 [ %84, %79 ], [ %110, %.critedge.i.i53 ]
  %104 = icmp ugt i64 %.sroa.013.1.i.i47, 4294967295
  br i1 %104, label %.critedge.i.i53, label %105

105:                                              ; preds = %103
  %106 = mul nuw i64 %.sroa.013.1.i.i47, %81
  %107 = shl nuw i64 %.sroa.019.0.i.i46, 32
  %108 = or disjoint i64 %107, %82
  %109 = icmp ugt i64 %106, %108
  br i1 %109, label %.critedge.i.i53, label %95

.critedge.i.i53:                                  ; preds = %105, %103
  %110 = add i64 %.sroa.013.1.i.i47, -1
  %111 = add nuw i64 %.sroa.019.0.i.i46, %80
  %112 = icmp ugt i64 %111, 4294967295
  br i1 %112, label %95, label %103

113:                                              ; preds = %.critedge2.i.i52, %95
  %.sroa.025.1.i.i49 = phi i64 [ %100, %95 ], [ %120, %.critedge2.i.i52 ]
  %.sroa.019.1.i.i50 = phi i64 [ %102, %95 ], [ %121, %.critedge2.i.i52 ]
  %114 = icmp ugt i64 %.sroa.025.1.i.i49, 4294967295
  br i1 %114, label %.critedge2.i.i52, label %115

115:                                              ; preds = %113
  %116 = mul nuw i64 %.sroa.025.1.i.i49, %81
  %117 = shl nuw i64 %.sroa.019.1.i.i50, 32
  %118 = or disjoint i64 %117, %83
  %119 = icmp ugt i64 %116, %118
  br i1 %119, label %.critedge2.i.i52, label %compiler_rt.udivmod.divwide__anon_7868.exit54

.critedge2.i.i52:                                 ; preds = %115, %113
  %120 = add i64 %.sroa.025.1.i.i49, -1
  %121 = add nuw i64 %.sroa.019.1.i.i50, %80
  %122 = icmp ugt i64 %121, 4294967295
  br i1 %122, label %compiler_rt.udivmod.divwide__anon_7868.exit54, label %113

compiler_rt.udivmod.divwide__anon_7868.exit54:    ; preds = %115, %.critedge2.i.i52
  %.sroa.025.0.i.i51 = phi i64 [ %120, %.critedge2.i.i52 ], [ %.sroa.025.1.i.i49, %115 ]
  %123 = shl i64 %99, 32
  %124 = or disjoint i64 %123, %83
  %125 = mul i64 %.sroa.025.0.i.i51, %.sroa.0.0.i.i45
  %126 = sub i64 %124, %125
  %127 = lshr i64 %126, %78
  br label %21

128:                                              ; preds = %21, %129
  %.sroa.10.0.insert.ext = zext i64 %.sroa.10.0 to i128
  %.sroa.10.0.insert.shift = shl nuw i128 %.sroa.10.0.insert.ext, 64
  %.sroa.014.0.insert.ext = zext i64 %23 to i128
  %.sroa.014.0.insert.insert = or disjoint i128 %.sroa.10.0.insert.shift, %.sroa.014.0.insert.ext
  br label %common.ret

129:                                              ; preds = %21
  %.sroa.0.0.insert.ext = zext i64 %.sroa.0.0 to i128
  store i128 %.sroa.0.0.insert.ext, ptr %2, align 16
  br label %128

130:                                              ; preds = %11, %130
  %.sroa.036.061 = phi i64 [ 0, %11 ], [ %140, %130 ]
  %.sroa.033.060 = phi i128 [ %18, %11 ], [ %139, %130 ]
  %.sroa.030.059 = phi i128 [ %0, %11 ], [ %138, %130 ]
  %.sroa.014.158 = phi i64 [ 0, %11 ], [ %136, %130 ]
  %131 = shl i64 %.sroa.014.158, 1
  %132 = xor i128 %.sroa.030.059, -1
  %133 = add i128 %.sroa.033.060, %132
  %134 = lshr i128 %133, 127
  %135 = trunc i128 %134 to i64
  %136 = or disjoint i64 %131, %135
  %isneg = icmp slt i128 %133, 0
  %137 = select i1 %isneg, i128 %.sroa.033.060, i128 0
  %138 = sub nuw i128 %.sroa.030.059, %137
  %139 = lshr i128 %.sroa.033.060, 1
  %140 = add nuw nsw i64 %.sroa.036.061, 1
  %exitcond.not = icmp eq i64 %140, %20
  br i1 %exitcond.not, label %141, label %130

141:                                              ; preds = %130
  %.not = icmp eq ptr %2, null
  br i1 %.not, label %142, label %143

142:                                              ; preds = %141, %143
  %.sroa.014.0.insert.ext18 = zext i64 %136 to i128
  br label %common.ret

143:                                              ; preds = %141
  store i128 %138, ptr %2, align 16
  br label %142
}

; Function Attrs: nounwind uwtable
define weak_odr dso_local i128 @__udivti3(i128 %0, i128 %1) local_unnamed_addr #1 {
  %3 = tail call fastcc i128 @compiler_rt.udivmod.udivmod__anon_4847(i128 %0, i128 %1, ptr align 16 null)
  ret i128 %3
}

; Function Attrs: nounwind uwtable
define weak_odr dso_local i128 @__modti3(i128 %0, i128 %1) local_unnamed_addr #1 {
  %3 = alloca [16 x i8], align 16
  %4 = ashr i128 %0, 127
  %5 = xor i128 %4, %0
  %6 = sub i128 %5, %4
  %7 = tail call i128 @llvm.abs.i128(i128 %1, i1 false)
  %8 = call fastcc i128 @compiler_rt.udivmod.udivmod__anon_4847(i128 %6, i128 %7, ptr nonnull align 16 %3)
  %9 = load i128, ptr %3, align 16
  %10 = xor i128 %9, %4
  %11 = sub i128 %10, %4
  ret i128 %11
}

; Function Attrs: nounwind uwtable
define weak_odr dso_local i128 @__umodti3(i128 %0, i128 %1) local_unnamed_addr #1 {
  %3 = alloca [16 x i8], align 16
  %4 = call fastcc i128 @compiler_rt.udivmod.udivmod__anon_4847(i128 %0, i128 %1, ptr nonnull align 16 %3)
  %5 = load i128, ptr %3, align 16
  ret i128 %5
}

; Function Attrs: nounwind uwtable
define weak_odr dso_local i128 @__muloti4(i128 %0, i128 %1, ptr nonnull align 4 %2) local_unnamed_addr #1 {
  %.fr = freeze i128 %1
  store i32 0, ptr %2, align 4
  %mul = tail call { i128, i1 } @llvm.smul.with.overflow.i128(i128 %0, i128 %.fr)
  %4 = icmp slt i128 %0, 0
  %5 = icmp eq i128 %.fr, -170141183460469231731687303715884105728
  %6 = and i1 %4, %5
  br i1 %6, label %.critedge, label %7

.critedge2:                                       ; preds = %7, %.critedge
  %mul.val = extractvalue { i128, i1 } %mul, 0
  ret i128 %mul.val

7:                                                ; preds = %3
  %mul.ov = extractvalue { i128, i1 } %mul, 1
  br i1 %mul.ov, label %.critedge, label %.critedge2

.critedge:                                        ; preds = %7, %3
  store i32 1, ptr %2, align 4
  br label %.critedge2
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.ctlz.i64(i64, i1 immarg) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.uadd.with.overflow.i64(i64, i64) #4

; Function Attrs: nounwind uwtable
define weak_odr dso_local float @floorf(float %0) local_unnamed_addr #1 {
  %2 = bitcast float %0 to i32
  %3 = lshr i32 %2, 23
  %4 = and i32 %3, 255
  %5 = add nuw nsw i32 %3, 1
  %6 = fcmp oeq float %0, 0.000000e+00
  %7 = icmp ugt i32 %4, 149
  %or.cond = or i1 %6, %7
  br i1 %or.cond, label %common.ret, label %8

common.ret:                                       ; preds = %10, %15, %1, %18
  %common.ret.op = phi float [ %23, %18 ], [ %0, %1 ], [ %., %15 ], [ %0, %10 ]
  ret float %common.ret.op

8:                                                ; preds = %1
  %9 = icmp ugt i32 %4, 126
  br i1 %9, label %10, label %15

10:                                               ; preds = %8
  %11 = and i32 %5, 31
  %12 = lshr i32 8388607, %11
  %13 = and i32 %12, %2
  %14 = icmp eq i32 %13, 0
  br i1 %14, label %common.ret, label %18

15:                                               ; preds = %8
  %16 = fadd float %0, 0x4770000000000000
  tail call void asm sideeffect "", "rm"(float %16) #10
  %17 = icmp sgt i32 %2, -1
  %. = select i1 %17, float 0.000000e+00, float -1.000000e+00
  br label %common.ret

18:                                               ; preds = %10
  %19 = fadd float %0, 0x4770000000000000
  tail call void asm sideeffect "", "rm"(float %19) #10
  %.not10 = icmp slt i32 %2, 0
  %20 = select i1 %.not10, i32 %12, i32 0
  %spec.select = add nuw nsw i32 %20, %2
  %21 = ashr i32 -8388608, %11
  %22 = and i32 %spec.select, %21
  %23 = bitcast i32 %22 to float
  br label %common.ret
}

; Function Attrs: nounwind uwtable
define weak_odr dso_local ptr @memcpy(ptr noalias align 1 %0, ptr noalias readonly align 1 %1, i64 %2) local_unnamed_addr #1 {
  %.not = icmp eq i64 %2, 0
  br i1 %.not, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %3
  %4 = load i8, ptr %1, align 1
  store i8 %4, ptr %0, align 1
  %5 = add i64 %2, -1
  %6 = icmp eq i64 %5, 0
  br i1 %6, label %.loopexit, label %iter.check

iter.check:                                       ; preds = %.preheader
  %min.iters.check = icmp ult i64 %2, 9
  br i1 %min.iters.check, label %.lr.ph.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check9 = icmp ult i64 %2, 33
  br i1 %min.iters.check9, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.vec = and i64 %5, -32
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %next.gep = getelementptr i8, ptr %0, i64 %index
  %next.gep11 = getelementptr i8, ptr %1, i64 %index
  %7 = getelementptr inbounds i8, ptr %next.gep, i64 1
  %8 = getelementptr inbounds i8, ptr %next.gep11, i64 1
  %9 = getelementptr inbounds i8, ptr %next.gep11, i64 17
  %wide.load = load <16 x i8>, ptr %8, align 1
  %wide.load13 = load <16 x i8>, ptr %9, align 1
  %10 = getelementptr inbounds i8, ptr %next.gep, i64 17
  store <16 x i8> %wide.load, ptr %7, align 1
  store <16 x i8> %wide.load13, ptr %10, align 1
  %index.next = add nuw i64 %index, 32
  %11 = icmp eq i64 %index.next, %n.vec
  br i1 %11, label %middle.block, label %vector.body, !llvm.loop !2

middle.block:                                     ; preds = %vector.body
  %cmp.n = icmp eq i64 %5, %n.vec
  br i1 %cmp.n, label %.loopexit, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %ind.end25 = getelementptr i8, ptr %1, i64 %n.vec
  %ind.end22 = getelementptr i8, ptr %0, i64 %n.vec
  %ind.end19 = and i64 %5, 31
  %n.vec.remaining = and i64 %5, 24
  %min.epilog.iters.check = icmp eq i64 %n.vec.remaining, 0
  br i1 %min.epilog.iters.check, label %.lr.ph.preheader, label %vec.epilog.ph

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec17 = and i64 %5, -8
  %ind.end18 = and i64 %5, 7
  %ind.end21 = getelementptr i8, ptr %0, i64 %n.vec17
  %ind.end24 = getelementptr i8, ptr %1, i64 %n.vec17
  %invariant.gep = getelementptr i8, ptr %0, i64 1
  %invariant.gep33 = getelementptr i8, ptr %1, i64 1
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index28 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next32, %vec.epilog.vector.body ]
  %gep = getelementptr i8, ptr %invariant.gep, i64 %index28
  %gep34 = getelementptr i8, ptr %invariant.gep33, i64 %index28
  %wide.load31 = load <8 x i8>, ptr %gep34, align 1
  store <8 x i8> %wide.load31, ptr %gep, align 1
  %index.next32 = add nuw i64 %index28, 8
  %12 = icmp eq i64 %index.next32, %n.vec17
  br i1 %12, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !5

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %cmp.n27 = icmp eq i64 %5, %n.vec17
  br i1 %cmp.n27, label %.loopexit, label %.lr.ph.preheader

.lr.ph.preheader:                                 ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph = phi i64 [ %5, %iter.check ], [ %ind.end19, %vec.epilog.iter.check ], [ %ind.end18, %vec.epilog.middle.block ]
  %.sroa.0.08.ph = phi ptr [ %0, %iter.check ], [ %ind.end22, %vec.epilog.iter.check ], [ %ind.end21, %vec.epilog.middle.block ]
  %.sroa.02.07.ph = phi ptr [ %1, %iter.check ], [ %ind.end25, %vec.epilog.iter.check ], [ %ind.end24, %vec.epilog.middle.block ]
  br label %.lr.ph

.loopexit:                                        ; preds = %.lr.ph, %middle.block, %vec.epilog.middle.block, %.preheader, %3
  ret ptr %0

.lr.ph:                                           ; preds = %.lr.ph.preheader, %.lr.ph
  %13 = phi i64 [ %17, %.lr.ph ], [ %.ph, %.lr.ph.preheader ]
  %.sroa.0.08 = phi ptr [ %14, %.lr.ph ], [ %.sroa.0.08.ph, %.lr.ph.preheader ]
  %.sroa.02.07 = phi ptr [ %15, %.lr.ph ], [ %.sroa.02.07.ph, %.lr.ph.preheader ]
  %14 = getelementptr inbounds i8, ptr %.sroa.0.08, i64 1
  %15 = getelementptr inbounds i8, ptr %.sroa.02.07, i64 1
  %16 = load i8, ptr %15, align 1
  store i8 %16, ptr %14, align 1
  %17 = add i64 %13, -1
  %18 = icmp eq i64 %17, 0
  br i1 %18, label %.loopexit, label %.lr.ph, !llvm.loop !6
}

; Function Attrs: nounwind uwtable
define weak_odr dso_local ptr @memset(ptr align 1 %0, i8 zeroext %1, i64 %2) local_unnamed_addr #1 {
  %.not = icmp eq i64 %2, 0
  br i1 %.not, label %.loopexit, label %iter.check

iter.check:                                       ; preds = %3
  %min.iters.check = icmp ult i64 %2, 8
  br i1 %min.iters.check, label %.preheader.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check5 = icmp ult i64 %2, 32
  br i1 %min.iters.check5, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.vec = and i64 %2, -32
  %broadcast.splatinsert = insertelement <16 x i8> poison, i8 %1, i64 0
  %broadcast.splat = shufflevector <16 x i8> %broadcast.splatinsert, <16 x i8> poison, <16 x i32> zeroinitializer
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %next.gep = getelementptr i8, ptr %0, i64 %index
  %4 = getelementptr i8, ptr %next.gep, i64 16
  store <16 x i8> %broadcast.splat, ptr %next.gep, align 1
  store <16 x i8> %broadcast.splat, ptr %4, align 1
  %index.next = add nuw i64 %index, 32
  %5 = icmp eq i64 %index.next, %n.vec
  br i1 %5, label %middle.block, label %vector.body, !llvm.loop !7

middle.block:                                     ; preds = %vector.body
  %cmp.n = icmp eq i64 %n.vec, %2
  br i1 %cmp.n, label %.loopexit, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %ind.end13 = getelementptr i8, ptr %0, i64 %n.vec
  %ind.end10 = and i64 %2, 31
  %n.vec.remaining = and i64 %2, 24
  %min.epilog.iters.check = icmp eq i64 %n.vec.remaining, 0
  br i1 %min.epilog.iters.check, label %.preheader.preheader, label %vec.epilog.ph

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec8 = and i64 %2, -8
  %ind.end9 = and i64 %2, 7
  %ind.end12 = getelementptr i8, ptr %0, i64 %n.vec8
  %broadcast.splatinsert18 = insertelement <8 x i8> poison, i8 %1, i64 0
  %broadcast.splat19 = shufflevector <8 x i8> %broadcast.splatinsert18, <8 x i8> poison, <8 x i32> zeroinitializer
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index16 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next20, %vec.epilog.vector.body ]
  %next.gep17 = getelementptr i8, ptr %0, i64 %index16
  store <8 x i8> %broadcast.splat19, ptr %next.gep17, align 1
  %index.next20 = add nuw i64 %index16, 8
  %6 = icmp eq i64 %index.next20, %n.vec8
  br i1 %6, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !8

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %cmp.n15 = icmp eq i64 %n.vec8, %2
  br i1 %cmp.n15, label %.loopexit, label %.preheader.preheader

.preheader.preheader:                             ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.sroa.02.0.ph = phi i64 [ %2, %iter.check ], [ %ind.end10, %vec.epilog.iter.check ], [ %ind.end9, %vec.epilog.middle.block ]
  %.sroa.0.0.ph = phi ptr [ %0, %iter.check ], [ %ind.end13, %vec.epilog.iter.check ], [ %ind.end12, %vec.epilog.middle.block ]
  br label %.preheader

.loopexit:                                        ; preds = %.preheader, %middle.block, %vec.epilog.middle.block, %3
  ret ptr %0

.preheader:                                       ; preds = %.preheader.preheader, %.preheader
  %.sroa.02.0 = phi i64 [ %7, %.preheader ], [ %.sroa.02.0.ph, %.preheader.preheader ]
  %.sroa.0.0 = phi ptr [ %9, %.preheader ], [ %.sroa.0.0.ph, %.preheader.preheader ]
  store i8 %1, ptr %.sroa.0.0, align 1
  %7 = add i64 %.sroa.02.0, -1
  %8 = icmp eq i64 %7, 0
  %9 = getelementptr inbounds i8, ptr %.sroa.0.0, i64 1
  br i1 %8, label %.loopexit, label %.preheader, !llvm.loop !9
}

; Function Attrs: noreturn nounwind uwtable
define private fastcc void @panic.panic_help() unnamed_addr #5 {
  %1 = alloca [24 x i8], align 8
  call fastcc void @str.RocStr.init(ptr noalias %1, ptr nonnull readonly align 1 @__anon_19998, i64 35)
  call void @roc_panic(ptr nonnull readonly align 8 %1, i32 0)
  unreachable
}

; Function Attrs: nounwind uwtable
define private fastcc void @str.RocStr.init(ptr noalias nocapture nonnull writeonly %0, ptr nocapture nonnull readonly align 1 %1, i64 %2) unnamed_addr #1 {
  %4 = alloca [24 x i8], align 8
  %5 = icmp ugt i64 %2, 23
  br i1 %5, label %str.RocStr.allocate.exit, label %str.RocStr.allocate.exit.thread

str.RocStr.allocate.exit.thread:                  ; preds = %3
  %.sroa.3.23.insert.ext = shl nuw nsw i64 %2, 56
  %.sroa.3.23.insert.shift = or disjoint i64 %.sroa.3.23.insert.ext, -9223372036854775808
  br label %str.RocStr.asU8ptrMut.exit

str.RocStr.allocate.exit:                         ; preds = %3
  %6 = add nuw i64 %2, 8
  %7 = tail call ptr @roc_alloc(i64 %6, i32 8), !noalias !10
  %8 = icmp ne ptr %7, null
  tail call void @llvm.assume(i1 %8)
  %9 = getelementptr inbounds i8, ptr %7, i64 8
  store i64 1, ptr %7, align 8, !noalias !10
  %10 = icmp slt i64 %2, 0
  %spec.select = select i1 %10, ptr %4, ptr %9
  br label %str.RocStr.asU8ptrMut.exit

str.RocStr.asU8ptrMut.exit:                       ; preds = %str.RocStr.allocate.exit, %str.RocStr.allocate.exit.thread
  %.sink7 = phi ptr [ %9, %str.RocStr.allocate.exit ], [ null, %str.RocStr.allocate.exit.thread ]
  %.sink = phi i64 [ %2, %str.RocStr.allocate.exit ], [ 0, %str.RocStr.allocate.exit.thread ]
  %.sroa.3.23.insert.shift.sink = phi i64 [ %2, %str.RocStr.allocate.exit ], [ %.sroa.3.23.insert.shift, %str.RocStr.allocate.exit.thread ]
  %common.ret.op.i = phi ptr [ %spec.select, %str.RocStr.allocate.exit ], [ %4, %str.RocStr.allocate.exit.thread ]
  store ptr %.sink7, ptr %4, align 8
  %11 = getelementptr inbounds i8, ptr %4, i64 8
  store i64 %.sink, ptr %11, align 8
  %12 = getelementptr inbounds i8, ptr %4, i64 16
  store i64 %.sroa.3.23.insert.shift.sink, ptr %12, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %common.ret.op.i, ptr nonnull align 1 %1, i64 %2, i1 false)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %4, i64 24, i1 false)
  ret void
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.uadd.sat.i64(i64, i64) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umin.i64(i64, i64) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.usub.sat.i64(i64, i64) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.usub.with.overflow.i64(i64, i64) #4

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i64 @llvm.umax.i64(i64, i64) #4

; Function Attrs: noreturn nounwind uwtable
declare void @roc_panic(ptr nonnull readonly align 8, i32) local_unnamed_addr #5

; Function Attrs: nounwind uwtable
define weak_odr dso_local i32 @__roc_force_setjmp(ptr align 4 %0) local_unnamed_addr #1 {
  %2 = tail call i32 @setjmp(ptr align 4 %0)
  ret i32 %2
}

; Function Attrs: nounwind uwtable
declare i32 @setjmp(ptr align 4) local_unnamed_addr #1

; Function Attrs: noreturn nounwind uwtable
define weak_odr dso_local void @__roc_force_longjmp(ptr align 4 %0, i32 %1) local_unnamed_addr #5 {
  tail call void @longjmp(ptr align 4 %0, i32 %1)
  unreachable
}

; Function Attrs: noreturn nounwind uwtable
declare void @longjmp(ptr align 4, i32) local_unnamed_addr #5

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.abs.i32(i32, i1 immarg) #4

; Function Attrs: nofree norecurse nosync nounwind memory(read, argmem: readwrite, inaccessiblemem: none) uwtable
define private fastcc void @fmt.parseIntWithSign__anon_8022(ptr noalias nocapture nonnull writeonly %0, ptr nocapture nonnull readonly align 1 %1, i64 %2) unnamed_addr #6 {
  %.sroa.2.i81 = alloca i8, align 8
  %.sroa.2.i = alloca i8, align 8
  %4 = icmp eq i64 %2, 0
  br i1 %4, label %5, label %8

common.ret:                                       ; preds = %71, %62, %56, %._crit_edge, %.critedge2, %5
  ret void

5:                                                ; preds = %3
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %0, ptr noundef nonnull align 8 dereferenceable(16) @7, i64 16, i1 false)
  br label %common.ret

.critedgethread-pre-split:                        ; preds = %8, %25, %22, %19, %13
  %.sroa.8.0.ph = phi i64 [ %2, %8 ], [ %21, %19 ], [ %24, %22 ], [ %27, %25 ], [ %2, %13 ]
  %.sroa.036.0.ph = phi ptr [ %1, %8 ], [ %20, %19 ], [ %23, %22 ], [ %26, %25 ], [ %1, %13 ]
  %.sroa.0.0.ph = phi i8 [ 10, %8 ], [ 2, %19 ], [ 8, %22 ], [ 16, %25 ], [ 10, %13 ]
  %.pr = load i8, ptr %.sroa.036.0.ph, align 1
  br label %.critedge

.critedge:                                        ; preds = %.critedgethread-pre-split, %10
  %6 = phi i8 [ %.pr, %.critedgethread-pre-split ], [ %11, %10 ]
  %.sroa.8.0 = phi i64 [ %.sroa.8.0.ph, %.critedgethread-pre-split ], [ %2, %10 ]
  %.sroa.036.0 = phi ptr [ %.sroa.036.0.ph, %.critedgethread-pre-split ], [ %1, %10 ]
  %.sroa.0.0 = phi i8 [ %.sroa.0.0.ph, %.critedgethread-pre-split ], [ 10, %10 ]
  %7 = icmp eq i8 %6, 95
  br i1 %7, label %.critedge2, label %28

8:                                                ; preds = %3
  %9 = icmp ugt i64 %2, 2
  br i1 %9, label %10, label %.critedgethread-pre-split

10:                                               ; preds = %8
  %11 = load i8, ptr %1, align 1
  %12 = icmp eq i8 %11, 48
  br i1 %12, label %13, label %.critedge

13:                                               ; preds = %10
  %14 = getelementptr inbounds i8, ptr %1, i64 1
  %15 = load i8, ptr %14, align 1
  %16 = add i8 %15, -65
  %17 = icmp ult i8 %16, 26
  %18 = or i8 %15, 32
  %common.ret.op.i = select i1 %17, i8 %18, i8 %15
  switch i8 %common.ret.op.i, label %.critedgethread-pre-split [
    i8 98, label %19
    i8 111, label %22
    i8 120, label %25
  ]

19:                                               ; preds = %13
  %20 = getelementptr inbounds i8, ptr %1, i64 2
  %21 = add nsw i64 %2, -2
  br label %.critedgethread-pre-split

22:                                               ; preds = %13
  %23 = getelementptr inbounds i8, ptr %1, i64 2
  %24 = add nsw i64 %2, -2
  br label %.critedgethread-pre-split

25:                                               ; preds = %13
  %26 = getelementptr inbounds i8, ptr %1, i64 2
  %27 = add nsw i64 %2, -2
  br label %.critedgethread-pre-split

28:                                               ; preds = %.critedge
  %29 = add nsw i64 %.sroa.8.0, -1
  %30 = getelementptr inbounds i8, ptr %.sroa.036.0, i64 %29
  %31 = load i8, ptr %30, align 1
  %32 = icmp eq i8 %31, 95
  br i1 %32, label %.critedge2, label %.lr.ph

.lr.ph:                                           ; preds = %28
  %33 = zext nneg i8 %.sroa.0.0 to i64
  br label %36

.critedge2:                                       ; preds = %.critedge, %28
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %0, ptr noundef nonnull align 8 dereferenceable(16) @7, i64 16, i1 false)
  br label %common.ret

34:                                               ; preds = %36, %73
  %.sroa.063.1 = phi i64 [ %.sroa.063.0110, %36 ], [ %74, %73 ]
  %35 = add nuw i64 %.sroa.067.0111, 1
  %exitcond.not = icmp eq i64 %35, %.sroa.8.0
  br i1 %exitcond.not, label %._crit_edge, label %36

36:                                               ; preds = %.lr.ph, %34
  %.sroa.067.0111 = phi i64 [ 0, %.lr.ph ], [ %35, %34 ]
  %.sroa.063.0110 = phi i64 [ 0, %.lr.ph ], [ %.sroa.063.1, %34 ]
  %37 = getelementptr inbounds i8, ptr %.sroa.036.0, i64 %.sroa.067.0111
  %38 = load i8, ptr %37, align 1
  %39 = icmp eq i8 %38, 95
  br i1 %39, label %34, label %41

._crit_edge:                                      ; preds = %34
  %40 = getelementptr inbounds { i64, i16, [6 x i8] }, ptr %0, i64 0, i32 1
  store i16 0, ptr %40, align 2
  store i64 %.sroa.063.1, ptr %0, align 8
  br label %common.ret

41:                                               ; preds = %36
  %42 = add i8 %38, -48
  %43 = icmp ult i8 %42, 10
  br i1 %43, label %44, label %46

44:                                               ; preds = %54, %49, %41
  %45 = phi i8 [ %50, %49 ], [ %55, %54 ], [ %42, %41 ]
  %.not.i = icmp ult i8 %45, %.sroa.0.0
  br i1 %.not.i, label %fmt.charToDigit.exit, label %56

46:                                               ; preds = %41
  %47 = add i8 %38, -65
  %48 = icmp ult i8 %47, 26
  br i1 %48, label %49, label %51

49:                                               ; preds = %46
  %50 = add nsw i8 %38, -55
  br label %44

51:                                               ; preds = %46
  %52 = add i8 %38, -97
  %53 = icmp ult i8 %52, 26
  br i1 %53, label %54, label %56

54:                                               ; preds = %51
  %55 = add nsw i8 %38, -87
  br label %44

56:                                               ; preds = %51, %44
  %57 = getelementptr inbounds { i64, i16, [6 x i8] }, ptr %0, i64 0, i32 1
  store i16 5, ptr %57, align 2
  br label %common.ret

fmt.charToDigit.exit:                             ; preds = %44
  %.not75 = icmp eq i64 %.sroa.063.0110, 0
  br i1 %.not75, label %66, label %58

58:                                               ; preds = %fmt.charToDigit.exit
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %.sroa.2.i)
  %59 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %.sroa.063.0110, i64 %33)
  %60 = extractvalue { i64, i1 } %59, 1
  store i1 %60, ptr %.sroa.2.i, align 8, !noalias !15
  %.sroa.2.i.0..sroa.2.i.0..sroa.2.i.0..sroa.2.0..sroa.2.0..sroa.2.0..sroa.2.8..i = load i8, ptr %.sroa.2.i, align 8, !noalias !15
  %61 = and i8 %.sroa.2.i.0..sroa.2.i.0..sroa.2.i.0..sroa.2.0..sroa.2.0..sroa.2.0..sroa.2.8..i, 1
  %.not.i80 = icmp eq i8 %61, 0
  br i1 %.not.i80, label %64, label %62

62:                                               ; preds = %58
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.2.i)
  %63 = getelementptr inbounds { i64, i16, [6 x i8] }, ptr %0, i64 0, i32 1
  store i16 1, ptr %63, align 2
  br label %common.ret

64:                                               ; preds = %58
  %65 = extractvalue { i64, i1 } %59, 0
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.2.i)
  br label %66

66:                                               ; preds = %64, %fmt.charToDigit.exit
  %.sroa.063.2 = phi i64 [ %65, %64 ], [ 0, %fmt.charToDigit.exit ]
  %67 = zext nneg i8 %45 to i64
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %.sroa.2.i81)
  %68 = tail call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %.sroa.063.2, i64 %67)
  %69 = extractvalue { i64, i1 } %68, 1
  store i1 %69, ptr %.sroa.2.i81, align 8, !noalias !18
  %.sroa.2.i81.0..sroa.2.i81.0..sroa.2.i81.0..sroa.2.0..sroa.2.0..sroa.2.0..sroa.2.8..i82 = load i8, ptr %.sroa.2.i81, align 8, !noalias !18
  %70 = and i8 %.sroa.2.i81.0..sroa.2.i81.0..sroa.2.i81.0..sroa.2.0..sroa.2.0..sroa.2.0..sroa.2.8..i82, 1
  %.not.i83 = icmp eq i8 %70, 0
  br i1 %.not.i83, label %73, label %71

71:                                               ; preds = %66
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.2.i81)
  %72 = getelementptr inbounds { i64, i16, [6 x i8] }, ptr %0, i64 0, i32 1
  store i16 1, ptr %72, align 2
  br label %common.ret

73:                                               ; preds = %66
  %74 = extractvalue { i64, i1 } %68, 0
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.2.i81)
  br label %34
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable
define private fastcc void @unicode.utf8Decode(ptr noalias nocapture nonnull writeonly %0, ptr nocapture nonnull readonly align 1 %1, i64 %2) unnamed_addr #7 {
  %.sroa.0.sroa.0.i2 = alloca i24, align 4
  %.sroa.0.sroa.0.i.i = alloca i24, align 4
  %.sroa.0.sroa.0.i = alloca i24, align 4
  %.sroa.014 = alloca i64, align 8
  %.sroa.010 = alloca i64, align 8
  %.sroa.08 = alloca i64, align 8
  %.sroa.0 = alloca i64, align 8
  switch i64 %2, label %6 [
    i64 1, label %7
    i64 2, label %10
    i64 3, label %29
    i64 4, label %63
  ]

unicode.utf8Decode3.exit:                         ; preds = %62, %61, %57, %unicode.utf8Decode4.exit, %unicode.utf8Decode2.exit, %7
  %4 = phi ptr [ %.sroa.0, %7 ], [ %.sroa.08, %unicode.utf8Decode2.exit ], [ %.sroa.014, %unicode.utf8Decode4.exit ], [ %.sroa.010, %57 ], [ %.sroa.010, %61 ], [ %.sroa.010, %62 ]
  %5 = load i64, ptr %4, align 8
  store i64 %5, ptr %0, align 4
  ret void

6:                                                ; preds = %3
  unreachable

7:                                                ; preds = %3
  %8 = load i8, ptr %1, align 1
  %9 = zext i8 %8 to i21
  %.sroa.0.4.sroa_idx17 = getelementptr inbounds i8, ptr %.sroa.0, i64 4
  store i16 0, ptr %.sroa.0.4.sroa_idx17, align 4
  store i21 %9, ptr %.sroa.0, align 8
  br label %unicode.utf8Decode3.exit

10:                                               ; preds = %3
  %.val = load i8, ptr %1, align 1
  %11 = getelementptr i8, ptr %1, i64 1
  %.val1 = load i8, ptr %11, align 1
  tail call void @llvm.experimental.noalias.scope.decl(metadata !21)
  call void @llvm.lifetime.start.p0(i64 3, ptr nonnull %.sroa.0.sroa.0.i)
  %12 = and i8 %.val, -32
  %13 = icmp eq i8 %12, -64
  tail call void @llvm.assume(i1 %13)
  %14 = and i8 %.val, 31
  %15 = zext nneg i8 %14 to i21
  store i21 %15, ptr %.sroa.0.sroa.0.i, align 4, !noalias !21
  %16 = and i8 %.val1, -64
  %.not.i = icmp eq i8 %16, -128
  br i1 %.not.i, label %18, label %17

17:                                               ; preds = %10
  store i64 103079215104, ptr %.sroa.08, align 8, !alias.scope !21
  br label %unicode.utf8Decode2.exit

18:                                               ; preds = %10
  %.sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0..i = load i24, ptr %.sroa.0.sroa.0.i, align 4, !noalias !21
  %19 = trunc i24 %.sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0..i to i21
  %20 = shl i21 %19, 6
  store i21 %20, ptr %.sroa.0.sroa.0.i, align 4, !noalias !21
  %.sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.1.i = load i24, ptr %.sroa.0.sroa.0.i, align 4, !noalias !21
  %21 = trunc i24 %.sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.1.i to i21
  %22 = and i8 %.val1, 63
  %23 = zext nneg i8 %22 to i21
  %24 = or i21 %21, %23
  store i21 %24, ptr %.sroa.0.sroa.0.i, align 4, !noalias !21
  %.sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.5.i = load i24, ptr %.sroa.0.sroa.0.i, align 4, !noalias !21
  %25 = trunc i24 %.sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.5.i to i21
  %26 = icmp ult i21 %25, 128
  br i1 %26, label %27, label %28

27:                                               ; preds = %18
  store i64 107374182400, ptr %.sroa.08, align 8, !alias.scope !21
  br label %unicode.utf8Decode2.exit

28:                                               ; preds = %18
  %.sroa.08.4.sroa_idx18 = getelementptr inbounds i8, ptr %.sroa.08, i64 4
  store i16 0, ptr %.sroa.08.4.sroa_idx18, align 4, !alias.scope !21
  store i21 %25, ptr %.sroa.08, align 8, !alias.scope !21
  br label %unicode.utf8Decode2.exit

unicode.utf8Decode2.exit:                         ; preds = %17, %27, %28
  call void @llvm.lifetime.end.p0(i64 3, ptr nonnull %.sroa.0.sroa.0.i)
  br label %unicode.utf8Decode3.exit

29:                                               ; preds = %3
  tail call void @llvm.experimental.noalias.scope.decl(metadata !24)
  call void @llvm.lifetime.start.p0(i64 3, ptr nonnull %.sroa.0.sroa.0.i.i)
  %30 = load i8, ptr %1, align 1, !noalias !27
  %31 = and i8 %30, -16
  %32 = icmp eq i8 %31, -32
  tail call void @llvm.assume(i1 %32)
  %33 = and i8 %30, 15
  %34 = zext nneg i8 %33 to i21
  store i21 %34, ptr %.sroa.0.sroa.0.i.i, align 4, !noalias !27
  %35 = getelementptr inbounds i8, ptr %1, i64 1
  %36 = load i8, ptr %35, align 1, !noalias !27
  %37 = and i8 %36, -64
  %.not.i.i = icmp eq i8 %37, -128
  br i1 %.not.i.i, label %38, label %57

38:                                               ; preds = %29
  %.sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0..i.i = load i24, ptr %.sroa.0.sroa.0.i.i, align 4, !noalias !27
  %39 = trunc i24 %.sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0..i.i to i21
  %40 = shl i21 %39, 6
  store i21 %40, ptr %.sroa.0.sroa.0.i.i, align 4, !noalias !27
  %.sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.1.i.i = load i24, ptr %.sroa.0.sroa.0.i.i, align 4, !noalias !27
  %41 = trunc i24 %.sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.1.i.i to i21
  %42 = and i8 %36, 63
  %43 = zext nneg i8 %42 to i21
  %44 = or i21 %41, %43
  store i21 %44, ptr %.sroa.0.sroa.0.i.i, align 4, !noalias !27
  %45 = getelementptr inbounds i8, ptr %1, i64 2
  %46 = load i8, ptr %45, align 1, !noalias !27
  %47 = and i8 %46, -64
  %.not46.i.i = icmp eq i8 %47, -128
  br i1 %.not46.i.i, label %48, label %57

48:                                               ; preds = %38
  %.sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.5.i.i = load i24, ptr %.sroa.0.sroa.0.i.i, align 4, !noalias !27
  %49 = trunc i24 %.sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.5.i.i to i21
  %50 = shl i21 %49, 6
  store i21 %50, ptr %.sroa.0.sroa.0.i.i, align 4, !noalias !27
  %.sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.9.i.i = load i24, ptr %.sroa.0.sroa.0.i.i, align 4, !noalias !27
  %51 = trunc i24 %.sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.9.i.i to i21
  %52 = and i8 %46, 63
  %53 = zext nneg i8 %52 to i21
  %54 = or i21 %51, %53
  store i21 %54, ptr %.sroa.0.sroa.0.i.i, align 4, !noalias !27
  %.sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.13.i.i = load i24, ptr %.sroa.0.sroa.0.i.i, align 4, !noalias !27
  %55 = trunc i24 %.sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.i.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.13.i.i to i21
  %56 = icmp ult i21 %55, 2048
  br i1 %56, label %57, label %58

57:                                               ; preds = %48, %38, %29
  %.sroa.51.0.ph.i = phi i16 [ 24, %29 ], [ 24, %38 ], [ 25, %48 ]
  call void @llvm.lifetime.end.p0(i64 3, ptr nonnull %.sroa.0.sroa.0.i.i)
  %.sroa.010.4.sroa_idx19 = getelementptr inbounds i8, ptr %.sroa.010, i64 4
  store i16 %.sroa.51.0.ph.i, ptr %.sroa.010.4.sroa_idx19, align 4, !alias.scope !24
  br label %unicode.utf8Decode3.exit

58:                                               ; preds = %48
  call void @llvm.lifetime.end.p0(i64 3, ptr nonnull %.sroa.0.sroa.0.i.i)
  %59 = and i21 %55, -2048
  %60 = icmp eq i21 %59, 55296
  br i1 %60, label %61, label %62

61:                                               ; preds = %58
  store i64 111669149696, ptr %.sroa.010, align 8, !alias.scope !24
  br label %unicode.utf8Decode3.exit

62:                                               ; preds = %58
  %.sroa.010.4.sroa_idx20 = getelementptr inbounds i8, ptr %.sroa.010, i64 4
  store i16 0, ptr %.sroa.010.4.sroa_idx20, align 4, !alias.scope !24
  store i21 %55, ptr %.sroa.010, align 8, !alias.scope !24
  br label %unicode.utf8Decode3.exit

63:                                               ; preds = %3
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30)
  call void @llvm.lifetime.start.p0(i64 3, ptr nonnull %.sroa.0.sroa.0.i2)
  %64 = load i8, ptr %1, align 1, !noalias !30
  %65 = and i8 %64, -8
  %66 = icmp eq i8 %65, -16
  tail call void @llvm.assume(i1 %66)
  %67 = and i8 %64, 7
  %68 = zext nneg i8 %67 to i21
  store i21 %68, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %69 = getelementptr inbounds i8, ptr %1, i64 1
  %70 = load i8, ptr %69, align 1, !noalias !30
  %71 = and i8 %70, -64
  %.not.i3 = icmp eq i8 %71, -128
  br i1 %.not.i3, label %73, label %72

72:                                               ; preds = %63
  store i64 103079215104, ptr %.sroa.014, align 8, !alias.scope !30
  br label %unicode.utf8Decode4.exit

73:                                               ; preds = %63
  %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0..i4 = load i24, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %74 = trunc i24 %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0..i4 to i21
  %75 = shl i21 %74, 6
  store i21 %75, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.1.i5 = load i24, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %76 = trunc i24 %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.1.i5 to i21
  %77 = and i8 %70, 63
  %78 = zext nneg i8 %77 to i21
  %79 = or i21 %76, %78
  store i21 %79, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %80 = getelementptr inbounds i8, ptr %1, i64 2
  %81 = load i8, ptr %80, align 1, !noalias !30
  %82 = and i8 %81, -64
  %.not73.i = icmp eq i8 %82, -128
  br i1 %.not73.i, label %84, label %83

83:                                               ; preds = %73
  store i64 103079215104, ptr %.sroa.014, align 8, !alias.scope !30
  br label %unicode.utf8Decode4.exit

84:                                               ; preds = %73
  %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.5.i6 = load i24, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %85 = trunc i24 %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.5.i6 to i21
  %86 = shl i21 %85, 6
  store i21 %86, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.9.i = load i24, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %87 = trunc i24 %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.9.i to i21
  %88 = and i8 %81, 63
  %89 = zext nneg i8 %88 to i21
  %90 = or i21 %87, %89
  store i21 %90, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %91 = getelementptr inbounds i8, ptr %1, i64 3
  %92 = load i8, ptr %91, align 1, !noalias !30
  %93 = and i8 %92, -64
  %.not74.i = icmp eq i8 %93, -128
  br i1 %.not74.i, label %95, label %94

94:                                               ; preds = %84
  store i64 103079215104, ptr %.sroa.014, align 8, !alias.scope !30
  br label %unicode.utf8Decode4.exit

95:                                               ; preds = %84
  %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.13.i = load i24, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %96 = trunc i24 %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.13.i to i21
  %97 = shl i21 %96, 6
  store i21 %97, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.17.i = load i24, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %98 = trunc i24 %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.17.i to i21
  %99 = and i8 %92, 63
  %100 = zext nneg i8 %99 to i21
  %101 = or i21 %98, %100
  store i21 %101, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.21.i = load i24, ptr %.sroa.0.sroa.0.i2, align 4, !noalias !30
  %102 = trunc i24 %.sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.i2.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.sroa.0.0..sroa.0.0.21.i to i21
  %103 = icmp ult i21 %102, 65536
  br i1 %103, label %104, label %105

104:                                              ; preds = %95
  store i64 107374182400, ptr %.sroa.014, align 8, !alias.scope !30
  br label %unicode.utf8Decode4.exit

105:                                              ; preds = %95
  %106 = icmp ugt i21 %102, -983041
  br i1 %106, label %107, label %108

107:                                              ; preds = %105
  store i64 115964116992, ptr %.sroa.014, align 8, !alias.scope !30
  br label %unicode.utf8Decode4.exit

108:                                              ; preds = %105
  %.sroa.014.4.sroa_idx21 = getelementptr inbounds i8, ptr %.sroa.014, i64 4
  store i16 0, ptr %.sroa.014.4.sroa_idx21, align 4, !alias.scope !30
  store i21 %102, ptr %.sroa.014, align 8, !alias.scope !30
  br label %unicode.utf8Decode4.exit

unicode.utf8Decode4.exit:                         ; preds = %72, %83, %94, %104, %107, %108
  call void @llvm.lifetime.end.p0(i64 3, ptr nonnull %.sroa.0.sroa.0.i2)
  br label %unicode.utf8Decode3.exit
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn uwtable
define private fastcc void @"io.GenericWriter(*io.fixed_buffer_stream.FixedBufferStream([]u8),error{NoSpaceLeft},(function 'write')).typeErasedWriteFn"(ptr noalias nocapture nonnull writeonly sret({ i64, i16, [6 x i8] }) %0, ptr nocapture nonnull readonly align 1 %1, ptr nocapture nonnull readonly align 1 %2, i64 %3) unnamed_addr #8 {
  %.sroa.5 = alloca [6 x i8], align 2
  %5 = load ptr, ptr %1, align 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !33)
  %6 = icmp eq i64 %3, 0
  br i1 %6, label %7, label %8

7:                                                ; preds = %4
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 2 dereferenceable(6) %.sroa.5, i8 0, i64 6, i1 false), !alias.scope !33
  br label %"io.fixed_buffer_stream.FixedBufferStream([]u8).write.exit"

8:                                                ; preds = %4
  %9 = getelementptr inbounds %"io.fixed_buffer_stream.FixedBufferStream([]u8)", ptr %5, i64 0, i32 1
  %10 = load i64, ptr %9, align 8, !noalias !33
  %.elt8.i = getelementptr inbounds { ptr, i64 }, ptr %5, i64 0, i32 1
  %.unpack9.i = load i64, ptr %.elt8.i, align 8, !noalias !33
  %.not.i = icmp ult i64 %10, %.unpack9.i
  br i1 %.not.i, label %12, label %11

11:                                               ; preds = %8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 2 dereferenceable(6) %.sroa.5, ptr noundef nonnull align 2 dereferenceable(6) getelementptr inbounds ({ i64, i16, [6 x i8] }, ptr @9, i64 0, i32 2, i64 0), i64 6, i1 false)
  br label %"io.fixed_buffer_stream.FixedBufferStream([]u8).write.exit"

12:                                               ; preds = %8
  %.unpack.i = load ptr, ptr %5, align 8, !noalias !33
  %13 = sub nuw i64 %.unpack9.i, %10
  %14 = tail call i64 @llvm.umin.i64(i64 %13, i64 %3)
  %15 = getelementptr inbounds i8, ptr %.unpack.i, i64 %10
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 1 %15, ptr nonnull align 1 %2, i64 %14, i1 false), !noalias !33
  %16 = load i64, ptr %9, align 8, !noalias !33
  %17 = add nuw i64 %16, %14
  store i64 %17, ptr %9, align 8, !noalias !33
  br label %"io.fixed_buffer_stream.FixedBufferStream([]u8).write.exit"

"io.fixed_buffer_stream.FixedBufferStream([]u8).write.exit": ; preds = %7, %11, %12
  %.sroa.0.0 = phi i64 [ 0, %7 ], [ %14, %12 ], [ undef, %11 ]
  %.sroa.4.0 = phi i16 [ 0, %7 ], [ 0, %12 ], [ 6, %11 ]
  store i64 %.sroa.0.0, ptr %0, align 8
  %.sroa.4.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 8
  store i16 %.sroa.4.0, ptr %.sroa.4.0..sroa_idx, align 8
  %.sroa.5.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 10
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 2 dereferenceable(6) %.sroa.5.0..sroa_idx, ptr noundef nonnull align 2 dereferenceable(6) %.sroa.5, i64 6, i1 false)
  ret void
}

; Function Attrs: nounwind uwtable
define private fastcc i16 @io.Writer.writeBytesNTimes(ptr nocapture nonnull readonly align 8 %0, ptr nonnull readonly align 1 %1, i64 %2, i64 %3) unnamed_addr #1 {
  %5 = alloca [16 x i8], align 8
  %.not = icmp eq i64 %3, 0
  br i1 %.not, label %common.ret, label %.lr.ph

.lr.ph:                                           ; preds = %4
  %.sroa.2.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 8
  %.not13.i = icmp eq i64 %2, 0
  %.sroa.212.0..sroa_idx.i = getelementptr inbounds i8, ptr %5, i64 8
  br i1 %.not13.i, label %common.ret, label %.lr.ph.i

common.ret:                                       ; preds = %.loopexit, %8, %.lr.ph, %4
  %common.ret.op = phi i16 [ 0, %4 ], [ 0, %.lr.ph ], [ %.sroa.212.0.copyload.i, %8 ], [ 0, %.loopexit ]
  ret i16 %common.ret.op

.lr.ph.i:                                         ; preds = %.lr.ph, %.loopexit
  %.sroa.0.06 = phi i64 [ %11, %.loopexit ], [ 0, %.lr.ph ]
  %.sroa.0.0.copyload = load ptr, ptr %0, align 8
  %.sroa.2.0.copyload = load ptr, ptr %.sroa.2.0..sroa_idx, align 8
  br label %8

6:                                                ; preds = %8
  %7 = add nuw i64 %.sroa.011.0.copyload.i, %.sroa.0.014.i
  %.not.i = icmp eq i64 %7, %2
  br i1 %.not.i, label %.loopexit, label %8

8:                                                ; preds = %6, %.lr.ph.i
  %.sroa.0.014.i = phi i64 [ 0, %.lr.ph.i ], [ %7, %6 ]
  %9 = getelementptr inbounds i8, ptr %1, i64 %.sroa.0.014.i
  %10 = sub nuw i64 %2, %.sroa.0.014.i
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5)
  call fastcc void %.sroa.2.0.copyload(ptr nonnull sret({ i64, i16, [6 x i8] }) %5, ptr nonnull readonly align 1 %.sroa.0.0.copyload, ptr nonnull readonly align 1 %9, i64 %10) #10, !noalias !36
  %.sroa.011.0.copyload.i = load i64, ptr %5, align 8
  %.sroa.212.0.copyload.i = load i16, ptr %.sroa.212.0..sroa_idx.i, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5)
  %.not7.i = icmp eq i16 %.sroa.212.0.copyload.i, 0
  br i1 %.not7.i, label %6, label %common.ret

.loopexit:                                        ; preds = %6
  %11 = add nuw i64 %.sroa.0.06, 1
  %exitcond.not = icmp eq i64 %11, %3
  br i1 %exitcond.not, label %common.ret, label %.lr.ph.i
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.umul.with.overflow.i64(i64, i64) #4

; Function Attrs: nounwind uwtable
define private fastcc void @str.RocStr.reallocateFresh(ptr noalias nocapture nonnull writeonly %0, ptr nocapture nonnull readonly align 8 %1, i64 %2) unnamed_addr #1 {
str.RocStr.len.exit:
  %3 = alloca [24 x i8], align 8
  %4 = alloca [24 x i8], align 8
  %5 = alloca [24 x i8], align 8
  %6 = alloca [24 x i8], align 8
  %.sroa.1.0..sroa_idx = getelementptr inbounds i8, ptr %1, i64 8
  %.sroa.1.0.copyload = load i64, ptr %.sroa.1.0..sroa_idx, align 8
  %.sroa.2.0..sroa_idx = getelementptr inbounds i8, ptr %1, i64 16
  %.sroa.2.0.copyload = load i64, ptr %.sroa.2.0..sroa_idx, align 8
  %7 = icmp slt i64 %.sroa.2.0.copyload, 0
  %8 = lshr i64 %.sroa.2.0.copyload, 56
  %9 = and i64 %8, 127
  %10 = and i64 %.sroa.1.0.copyload, 9223372036854775807
  %common.ret.op.i = select i1 %7, i64 %9, i64 %10
  %11 = icmp ugt i64 %2, 23
  br i1 %11, label %str.RocStr.asU8ptr.exit17, label %str.RocStr.asU8ptr.exit

common.ret.sink.split:                            ; preds = %54, %30
  %.sink50 = phi ptr [ %28, %30 ], [ %52, %54 ]
  %.sink.ph = phi ptr [ %4, %30 ], [ %6, %54 ]
  tail call void @roc_dealloc(ptr nonnull align 1 %.sink50, i32 8)
  br label %common.ret

common.ret:                                       ; preds = %common.ret.sink.split, %str.RocStr.asU8ptr.exit17, %44, %50, %54, %str.RocStr.asU8ptr.exit, %20, %26, %30
  %.sink = phi ptr [ %4, %30 ], [ %4, %26 ], [ %4, %20 ], [ %4, %str.RocStr.asU8ptr.exit ], [ %6, %54 ], [ %6, %50 ], [ %6, %44 ], [ %6, %str.RocStr.asU8ptr.exit17 ], [ %.sink.ph, %common.ret.sink.split ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %.sink, i64 24, i1 false)
  ret void

str.RocStr.asU8ptr.exit:                          ; preds = %str.RocStr.len.exit
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %4, ptr noundef nonnull align 8 dereferenceable(24) @8, i64 24, i1 false)
  %12 = getelementptr inbounds i8, ptr %4, i64 23
  %13 = trunc i64 %2 to i8
  %14 = or disjoint i8 %13, -128
  store i8 %14, ptr %12, align 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %3, ptr noundef nonnull align 8 dereferenceable(24) %1, i64 24, i1 false)
  %15 = getelementptr inbounds i8, ptr %3, i64 16
  %.val.i = load i64, ptr %15, align 8
  %16 = icmp slt i64 %.val.i, 0
  %17 = load ptr, ptr %3, align 8
  %spec.select = select i1 %16, ptr %3, ptr %17
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 8 %4, ptr align 1 %spec.select, i64 %common.ret.op.i, i1 false)
  %18 = getelementptr inbounds i8, ptr %4, i64 %common.ret.op.i
  %19 = sub nuw nsw i64 %2, %common.ret.op.i
  call void @llvm.memset.p0.i64(ptr nonnull align 1 %18, i8 0, i64 %19, i1 false)
  br i1 %7, label %common.ret, label %20

20:                                               ; preds = %str.RocStr.asU8ptr.exit
  %.sroa.031.sroa.0.0.copyload = load ptr, ptr %1, align 8
  %21 = ptrtoint ptr %.sroa.031.sroa.0.0.copyload to i64
  %22 = shl nuw i64 %.sroa.2.0.copyload, 1
  %isneg.i = icmp slt i64 %.sroa.1.0.copyload, 0
  %23 = select i1 %isneg.i, i64 %22, i64 %21
  %24 = icmp ne i64 %.sroa.2.0.copyload, 0
  %25 = icmp ne i64 %23, 0
  %or.cond.i = select i1 %24, i1 %25, i1 false
  br i1 %or.cond.i, label %26, label %common.ret

26:                                               ; preds = %20
  %27 = inttoptr i64 %23 to ptr
  %28 = getelementptr inbounds i64, ptr %27, i64 -1
  %29 = load i64, ptr %28, align 8
  %.not.i = icmp eq i64 %29, 0
  br i1 %.not.i, label %common.ret, label %30

30:                                               ; preds = %26
  %31 = atomicrmw sub ptr %28, i64 1 monotonic, align 8
  %32 = icmp eq i64 %31, 1
  br i1 %32, label %common.ret.sink.split, label %common.ret

str.RocStr.asU8ptr.exit17:                        ; preds = %str.RocStr.len.exit
  %33 = add nuw i64 %2, 8
  %34 = tail call ptr @roc_alloc(i64 %33, i32 8), !noalias !39
  %35 = icmp ne ptr %34, null
  tail call void @llvm.assume(i1 %35)
  %36 = getelementptr inbounds i8, ptr %34, i64 8
  store i64 1, ptr %34, align 8, !noalias !39
  store ptr %36, ptr %6, align 8
  %.sroa.224.0..sroa_idx = getelementptr inbounds i8, ptr %6, i64 8
  store i64 %2, ptr %.sroa.224.0..sroa_idx, align 8
  %.sroa.3.0..sroa_idx = getelementptr inbounds i8, ptr %6, i64 16
  store i64 %2, ptr %.sroa.3.0..sroa_idx, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %5, ptr noundef nonnull align 8 dereferenceable(24) %1, i64 24, i1 false)
  %37 = getelementptr inbounds i8, ptr %5, i64 16
  %.val.i15 = load i64, ptr %37, align 8
  %38 = icmp slt i64 %.val.i15, 0
  %39 = load ptr, ptr %5, align 8
  %spec.select49 = select i1 %38, ptr %5, ptr %39
  %40 = icmp slt i64 %2, 0
  %common.ret.op.i19 = select i1 %40, ptr %6, ptr %36
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %common.ret.op.i19, ptr align 1 %spec.select49, i64 %common.ret.op.i, i1 false)
  %41 = getelementptr inbounds i8, ptr %common.ret.op.i19, i64 %common.ret.op.i
  %42 = sub nuw i64 %2, %common.ret.op.i
  call void @llvm.memset.p0.i64(ptr nonnull align 1 %41, i8 0, i64 %42, i1 false)
  %.sroa.326.0.copyload = load i64, ptr %.sroa.2.0..sroa_idx, align 8
  %43 = icmp slt i64 %.sroa.326.0.copyload, 0
  br i1 %43, label %common.ret, label %44

44:                                               ; preds = %str.RocStr.asU8ptr.exit17
  %.sroa.025.sroa.3.0.copyload = load i64, ptr %.sroa.1.0..sroa_idx, align 8
  %.sroa.025.sroa.0.0.copyload = load ptr, ptr %1, align 8
  %45 = ptrtoint ptr %.sroa.025.sroa.0.0.copyload to i64
  %46 = shl nuw i64 %.sroa.326.0.copyload, 1
  %isneg.i22 = icmp slt i64 %.sroa.025.sroa.3.0.copyload, 0
  %47 = select i1 %isneg.i22, i64 %46, i64 %45
  %48 = icmp ne i64 %.sroa.326.0.copyload, 0
  %49 = icmp ne i64 %47, 0
  %or.cond.i8 = select i1 %48, i1 %49, i1 false
  br i1 %or.cond.i8, label %50, label %common.ret

50:                                               ; preds = %44
  %51 = inttoptr i64 %47 to ptr
  %52 = getelementptr inbounds i64, ptr %51, i64 -1
  %53 = load i64, ptr %52, align 8
  %.not.i9 = icmp eq i64 %53, 0
  br i1 %.not.i9, label %common.ret, label %54

54:                                               ; preds = %50
  %55 = atomicrmw sub ptr %52, i64 1 monotonic, align 8
  %56 = icmp eq i64 %55, 1
  br i1 %56, label %common.ret.sink.split, label %common.ret
}

; Function Attrs: nounwind uwtable
declare void @roc_dealloc(ptr nonnull align 1, i32) local_unnamed_addr #1

; Function Attrs: nounwind uwtable
declare ptr @roc_alloc(i64, i32) local_unnamed_addr #1

; Function Attrs: nounwind uwtable
declare ptr @roc_realloc(ptr nonnull align 1, i64, i64, i32) local_unnamed_addr #1

; Function Attrs: nounwind uwtable
define private fastcc i16 @fmt.formatBuf__anon_8817(ptr nonnull readonly align 1 %0, i64 %1, ptr nocapture nonnull readonly align 8 %2, ptr nocapture nonnull readonly align 8 %3) unnamed_addr #1 {
  %5 = alloca [16 x i8], align 8
  %6 = alloca [16 x i8], align 8
  %7 = alloca [16 x i8], align 8
  %8 = alloca [16 x i8], align 8
  %9 = alloca [16 x i8], align 8
  %10 = alloca [16 x i8], align 8
  %.sroa.4.i = alloca i8, align 2
  %11 = alloca [16 x i8], align 8
  %12 = alloca [16 x i8], align 8
  %13 = alloca [8 x i8], align 4
  %14 = alloca [16 x i8], align 8
  %15 = alloca [4 x i8], align 1
  %.sroa.2.0..sroa_idx = getelementptr inbounds %fmt.FormatOptions, ptr %2, i64 0, i32 1, i32 1
  %.sroa.2.0.copyload = load i8, ptr %.sroa.2.0..sroa_idx, align 8
  %.not = icmp eq i8 %.sroa.2.0.copyload, 0
  %.not13.i = icmp eq i64 %1, 0
  br i1 %.not, label %59, label %16

common.ret:                                       ; preds = %68, %66, %185, %191, %189, %171, %177, %152, %.loopexit.i, %159, %62, %60, %.loopexit156, %.loopexit, %65, %59, %.loopexit150
  %common.ret.op = phi i16 [ %180, %.loopexit150 ], [ 0, %59 ], [ 0, %65 ], [ 0, %.loopexit ], [ 0, %.loopexit156 ], [ 0, %60 ], [ %.sroa.212.0.copyload.i, %62 ], [ %.sroa.212.0.copyload.i.i, %159 ], [ 0, %.loopexit.i ], [ %.sroa.212.0.copyload.i40, %152 ], [ %.sroa.212.0.copyload.i75, %177 ], [ %.sroa.212.0.copyload.i.i60, %171 ], [ 0, %189 ], [ %.sroa.212.0.copyload.i106, %191 ], [ %.sroa.212.0.copyload.i.i91, %185 ], [ 0, %66 ], [ %.sroa.212.0.copyload.i27, %68 ]
  ret i16 %common.ret.op

16:                                               ; preds = %4
  %17 = getelementptr inbounds %fmt.FormatOptions, ptr %2, i64 0, i32 1
  %.sroa.0.0.copyload = load i64, ptr %17, align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %13)
  br i1 %.not13.i, label %.loopexit159, label %.preheader.lr.ph.i

.preheader.lr.ph.i:                               ; preds = %16
  %18 = getelementptr inbounds { i21, i16, [2 x i8] }, ptr %13, i64 0, i32 1
  br label %.preheader.i

.preheader.i:                                     ; preds = %28, %.preheader.lr.ph.i
  %.sroa.0.047.i = phi i64 [ 0, %.preheader.lr.ph.i ], [ %.sroa.0.2.i, %28 ]
  %.sroa.03.046.i = phi i64 [ 0, %.preheader.lr.ph.i ], [ %.sroa.03.2.i, %28 ]
  %19 = add nuw i64 %.sroa.03.046.i, 8
  %.not39.i = icmp ugt i64 %19, %1
  br i1 %.not39.i, label %._crit_edge.i, label %.lr.ph.i

._crit_edge.i:                                    ; preds = %25, %.lr.ph.i, %.preheader.i
  %.sroa.03.1.lcssa.i = phi i64 [ %.sroa.03.046.i, %.preheader.i ], [ %21, %25 ], [ %.sroa.03.140.i, %.lr.ph.i ]
  %.sroa.0.1.lcssa.i = phi i64 [ %.sroa.0.047.i, %.preheader.i ], [ %26, %25 ], [ %.sroa.0.141.i, %.lr.ph.i ]
  %20 = icmp ult i64 %.sroa.03.1.lcssa.i, %1
  br i1 %20, label %30, label %28

.lr.ph.i:                                         ; preds = %.preheader.i, %25
  %21 = phi i64 [ %27, %25 ], [ %19, %.preheader.i ]
  %.sroa.0.141.i = phi i64 [ %26, %25 ], [ %.sroa.0.047.i, %.preheader.i ]
  %.sroa.03.140.i = phi i64 [ %21, %25 ], [ %.sroa.03.046.i, %.preheader.i ]
  %22 = getelementptr inbounds i8, ptr %0, i64 %.sroa.03.140.i
  %23 = load i64, ptr %22, align 1, !noalias !42
  %24 = and i64 %23, -9187201950435737472
  %.not27.i = icmp eq i64 %24, 0
  br i1 %.not27.i, label %25, label %._crit_edge.i

25:                                               ; preds = %.lr.ph.i
  %26 = add nuw i64 %.sroa.0.141.i, 8
  %27 = add nuw i64 %21, 8
  %.not.i = icmp ugt i64 %27, %1
  br i1 %.not.i, label %._crit_edge.i, label %.lr.ph.i

28:                                               ; preds = %52, %._crit_edge.i
  %.sroa.03.2.i = phi i64 [ %49, %52 ], [ %.sroa.03.1.lcssa.i, %._crit_edge.i ]
  %.sroa.0.2.i = phi i64 [ %53, %52 ], [ %.sroa.0.1.lcssa.i, %._crit_edge.i ]
  %29 = icmp ult i64 %.sroa.03.2.i, %1
  br i1 %29, label %.preheader.i, label %.loopexit159

30:                                               ; preds = %._crit_edge.i
  %31 = getelementptr inbounds i8, ptr %0, i64 %.sroa.03.1.lcssa.i
  %32 = load i8, ptr %31, align 1, !noalias !42
  %33 = icmp sgt i8 %32, -1
  br i1 %33, label %unicode.utf8ByteSequenceLength.exit.i, label %34

34:                                               ; preds = %30
  %35 = and i8 %32, -32
  %36 = icmp eq i8 %35, -64
  br i1 %36, label %unicode.utf8ByteSequenceLength.exit.i, label %37

37:                                               ; preds = %34
  %38 = and i8 %32, -16
  %39 = icmp eq i8 %38, -32
  br i1 %39, label %unicode.utf8ByteSequenceLength.exit.i, label %40

40:                                               ; preds = %37
  %41 = and i8 %32, -8
  %42 = icmp eq i8 %41, -16
  %..i.i = select i1 %42, ptr @5, ptr @0
  br label %unicode.utf8ByteSequenceLength.exit.i

unicode.utf8ByteSequenceLength.exit.i:            ; preds = %40, %37, %34, %30
  %43 = phi ptr [ @2, %30 ], [ @3, %34 ], [ @4, %37 ], [ %..i.i, %40 ]
  %44 = load i16, ptr %43, align 2, !noalias !42
  %.sroa_idx.i = getelementptr inbounds i8, ptr %43, i64 2
  %45 = load i8, ptr %.sroa_idx.i, align 2, !noalias !42
  %.not28.i = icmp eq i16 %44, 0
  %46 = trunc i8 %45 to i3
  br i1 %.not28.i, label %47, label %unicode.utf8CountCodepoints.exit.thread

47:                                               ; preds = %unicode.utf8ByteSequenceLength.exit.i
  %.mask.i = and i8 %45, 7
  %48 = zext nneg i8 %.mask.i to i64
  %49 = add nuw i64 %.sroa.03.1.lcssa.i, %48
  %50 = icmp ugt i64 %49, %1
  br i1 %50, label %unicode.utf8CountCodepoints.exit.thread, label %51

51:                                               ; preds = %47
  %cond.i = icmp eq i3 %46, 1
  br i1 %cond.i, label %52, label %54

52:                                               ; preds = %54, %51
  %53 = add nuw i64 %.sroa.0.1.lcssa.i, 1
  br label %28

54:                                               ; preds = %51
  call fastcc void @unicode.utf8Decode(ptr noalias %13, ptr nonnull readonly align 1 %31, i64 %48), !noalias !42
  %55 = load i16, ptr %18, align 4, !noalias !42
  %.not29.i = icmp eq i16 %55, 0
  br i1 %.not29.i, label %52, label %unicode.utf8CountCodepoints.exit.thread

unicode.utf8CountCodepoints.exit.thread:          ; preds = %47, %unicode.utf8ByteSequenceLength.exit.i, %54
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %13)
  br label %56

.loopexit159:                                     ; preds = %28, %16
  %.sroa.0.0 = phi i64 [ 0, %16 ], [ %.sroa.0.2.i, %28 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %13)
  br label %56

56:                                               ; preds = %unicode.utf8CountCodepoints.exit.thread, %.loopexit159
  %57 = phi i64 [ %.sroa.0.0, %.loopexit159 ], [ %1, %unicode.utf8CountCodepoints.exit.thread ]
  %58 = tail call i64 @llvm.usub.sat.i64(i64 %.sroa.0.0.copyload, i64 %57)
  %.not12 = icmp ugt i64 %.sroa.0.0.copyload, %57
  br i1 %.not12, label %71, label %65

59:                                               ; preds = %4
  %.sroa.0129.0.copyload = load ptr, ptr %3, align 8
  %.sroa.2130.0..sroa_idx = getelementptr inbounds i8, ptr %3, i64 8
  %.sroa.2130.0.copyload = load ptr, ptr %.sroa.2130.0..sroa_idx, align 8
  br i1 %.not13.i, label %common.ret, label %.lr.ph.i17

.lr.ph.i17:                                       ; preds = %59
  %.sroa.212.0..sroa_idx.i = getelementptr inbounds i8, ptr %12, i64 8
  br label %62

60:                                               ; preds = %62
  %61 = add nuw i64 %.sroa.011.0.copyload.i, %.sroa.0.014.i
  %.not.i18 = icmp eq i64 %61, %1
  br i1 %.not.i18, label %common.ret, label %62

62:                                               ; preds = %60, %.lr.ph.i17
  %.sroa.0.014.i = phi i64 [ 0, %.lr.ph.i17 ], [ %61, %60 ]
  %63 = getelementptr inbounds i8, ptr %0, i64 %.sroa.0.014.i
  %64 = sub nuw i64 %1, %.sroa.0.014.i
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %12)
  call fastcc void %.sroa.2130.0.copyload(ptr nonnull sret({ i64, i16, [6 x i8] }) %12, ptr nonnull readonly align 1 %.sroa.0129.0.copyload, ptr nonnull readonly align 1 %63, i64 %64) #10, !noalias !45
  %.sroa.011.0.copyload.i = load i64, ptr %12, align 8
  %.sroa.212.0.copyload.i = load i16, ptr %.sroa.212.0..sroa_idx.i, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %12)
  %.not7.i = icmp eq i16 %.sroa.212.0.copyload.i, 0
  br i1 %.not7.i, label %60, label %common.ret

65:                                               ; preds = %56
  %.sroa.0112.0.copyload = load ptr, ptr %3, align 8
  %.sroa.2.0..sroa_idx113 = getelementptr inbounds i8, ptr %3, i64 8
  %.sroa.2.0.copyload114 = load ptr, ptr %.sroa.2.0..sroa_idx113, align 8
  br i1 %.not13.i, label %common.ret, label %.lr.ph.i20

.lr.ph.i20:                                       ; preds = %65
  %.sroa.212.0..sroa_idx.i22 = getelementptr inbounds i8, ptr %11, i64 8
  br label %68

66:                                               ; preds = %68
  %67 = add nuw i64 %.sroa.011.0.copyload.i26, %.sroa.0.014.i23
  %.not.i30 = icmp eq i64 %67, %1
  br i1 %.not.i30, label %common.ret, label %68

68:                                               ; preds = %66, %.lr.ph.i20
  %.sroa.0.014.i23 = phi i64 [ 0, %.lr.ph.i20 ], [ %67, %66 ]
  %69 = getelementptr inbounds i8, ptr %0, i64 %.sroa.0.014.i23
  %70 = sub nuw i64 %1, %.sroa.0.014.i23
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %11)
  call fastcc void %.sroa.2.0.copyload114(ptr nonnull sret({ i64, i16, [6 x i8] }) %11, ptr nonnull readonly align 1 %.sroa.0112.0.copyload, ptr nonnull readonly align 1 %69, i64 %70) #10, !noalias !48
  %.sroa.011.0.copyload.i26 = load i64, ptr %11, align 8
  %.sroa.212.0.copyload.i27 = load i16, ptr %.sroa.212.0..sroa_idx.i22, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %11)
  %.not7.i28 = icmp eq i16 %.sroa.212.0.copyload.i27, 0
  br i1 %.not7.i28, label %66, label %common.ret

71:                                               ; preds = %56
  %72 = getelementptr inbounds %fmt.FormatOptions, ptr %2, i64 0, i32 2
  %73 = load i32, ptr %72, align 8
  %74 = trunc i32 %73 to i21
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %.sroa.4.i)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !51)
  %75 = icmp ult i21 %74, 128
  br i1 %75, label %76, label %78

76:                                               ; preds = %71
  %77 = load i32, ptr @2, align 4, !noalias !54
  br label %unicode.utf8CodepointSequenceLength.exit.i.i

78:                                               ; preds = %71
  %79 = icmp ult i21 %74, 2048
  br i1 %79, label %80, label %82

80:                                               ; preds = %78
  %81 = load i32, ptr @3, align 4, !noalias !54
  br label %unicode.utf8CodepointSequenceLength.exit.i.i

82:                                               ; preds = %78
  %83 = icmp ult i21 %74, 65536
  br i1 %83, label %84, label %86

84:                                               ; preds = %82
  %85 = load i32, ptr @4, align 4, !noalias !54
  br label %unicode.utf8CodepointSequenceLength.exit.i.i

86:                                               ; preds = %82
  %87 = icmp ult i21 %74, -983040
  %88 = load i32, ptr @5, align 4, !noalias !54
  %spec.select.i.i.i = select i1 %87, i32 %88, i32 29
  br label %unicode.utf8CodepointSequenceLength.exit.i.i

unicode.utf8CodepointSequenceLength.exit.i.i:     ; preds = %86, %84, %80, %76
  %.sink.i.i.i = phi i32 [ %85, %84 ], [ %81, %80 ], [ %77, %76 ], [ %spec.select.i.i.i, %86 ]
  %.sroa.2.0.extract.shift.i.i = lshr i32 %.sink.i.i.i, 16
  %89 = and i32 %.sink.i.i.i, 65535
  %.not.i.i = icmp eq i32 %89, 0
  %90 = trunc i32 %.sroa.2.0.extract.shift.i.i to i3
  br i1 %.not.i.i, label %91, label %unicode.utf8Encode.exit.thread

91:                                               ; preds = %unicode.utf8CodepointSequenceLength.exit.i.i
  %92 = icmp ult i3 %90, -3
  tail call void @llvm.assume(i1 %92)
  switch i3 %90, label %93 [
    i3 1, label %94
    i3 2, label %96
    i3 3, label %104
    i3 -4, label %120
  ]

93:                                               ; preds = %91
  unreachable

94:                                               ; preds = %91
  %95 = trunc i32 %73 to i8
  store i8 %95, ptr %15, align 1, !noalias !59
  br label %138

96:                                               ; preds = %91
  %97 = lshr i32 %73, 6
  %98 = trunc i32 %97 to i8
  %99 = or i8 %98, -64
  store i8 %99, ptr %15, align 1, !noalias !59
  %100 = getelementptr inbounds i8, ptr %15, i64 1
  %101 = trunc i32 %73 to i8
  %102 = and i8 %101, 63
  %103 = or disjoint i8 %102, -128
  store i8 %103, ptr %100, align 1, !noalias !59
  br label %138

104:                                              ; preds = %91
  %105 = and i21 %74, -2048
  %106 = icmp eq i21 %105, 55296
  br i1 %106, label %unicode.utf8Encode.exit.thread, label %107

107:                                              ; preds = %104
  %108 = lshr i32 %73, 12
  %109 = trunc i32 %108 to i8
  %110 = or i8 %109, -32
  store i8 %110, ptr %15, align 1, !noalias !59
  %111 = getelementptr inbounds i8, ptr %15, i64 1
  %112 = lshr i32 %73, 6
  %113 = trunc i32 %112 to i8
  %114 = and i8 %113, 63
  %115 = or disjoint i8 %114, -128
  store i8 %115, ptr %111, align 1, !noalias !59
  %116 = getelementptr inbounds i8, ptr %15, i64 2
  %117 = trunc i32 %73 to i8
  %118 = and i8 %117, 63
  %119 = or disjoint i8 %118, -128
  store i8 %119, ptr %116, align 1, !noalias !59
  br label %138

120:                                              ; preds = %91
  %121 = lshr i21 %74, 18
  %122 = trunc i21 %121 to i8
  %123 = or disjoint i8 %122, -16
  store i8 %123, ptr %15, align 1, !noalias !59
  %124 = getelementptr inbounds i8, ptr %15, i64 1
  %125 = lshr i32 %73, 12
  %126 = trunc i32 %125 to i8
  %127 = and i8 %126, 63
  %128 = or disjoint i8 %127, -128
  store i8 %128, ptr %124, align 1, !noalias !59
  %129 = getelementptr inbounds i8, ptr %15, i64 2
  %130 = lshr i32 %73, 6
  %131 = trunc i32 %130 to i8
  %132 = and i8 %131, 63
  %133 = or disjoint i8 %132, -128
  store i8 %133, ptr %129, align 1, !noalias !59
  %134 = getelementptr inbounds i8, ptr %15, i64 3
  %135 = trunc i32 %73 to i8
  %136 = and i8 %135, 63
  %137 = or disjoint i8 %136, -128
  store i8 %137, ptr %134, align 1, !noalias !59
  br label %138

unicode.utf8Encode.exit.thread:                   ; preds = %unicode.utf8CodepointSequenceLength.exit.i.i, %104
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.4.i)
  br label %143

138:                                              ; preds = %120, %107, %96, %94
  store i3 %90, ptr %.sroa.4.i, align 2, !alias.scope !51, !noalias !60
  %.sroa.4.i.0..sroa.4.i.0..sroa.4.i.0..sroa.4.0..sroa.4.0..sroa.4.2..pre.i = load i8, ptr %.sroa.4.i, align 2, !noalias !60
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.4.i)
  %139 = insertvalue { ptr, i64 } poison, ptr %15, 0
  %140 = and i8 %.sroa.4.i.0..sroa.4.i.0..sroa.4.i.0..sroa.4.0..sroa.4.0..sroa.4.2..pre.i, 7
  %141 = zext nneg i8 %140 to i64
  %142 = insertvalue { ptr, i64 } %139, i64 %141, 1
  br label %143

143:                                              ; preds = %unicode.utf8Encode.exit.thread, %138
  %144 = phi { ptr, i64 } [ %142, %138 ], [ { ptr @__anon_23980, i64 3 }, %unicode.utf8Encode.exit.thread ]
  %145 = getelementptr inbounds %fmt.FormatOptions, ptr %2, i64 0, i32 3
  %146 = load i8, ptr %145, align 4
  %147 = trunc i8 %146 to i2
  switch i2 %147, label %148 [
    i2 0, label %149
    i2 1, label %163
    i2 -2, label %.lr.ph.i81
  ]

148:                                              ; preds = %143
  unreachable

149:                                              ; preds = %143
  %.sroa.0117.0.copyload = load ptr, ptr %3, align 8
  %.sroa.2118.0..sroa_idx = getelementptr inbounds i8, ptr %3, i64 8
  %.sroa.2118.0.copyload = load ptr, ptr %.sroa.2118.0..sroa_idx, align 8
  br i1 %.not13.i, label %.loopexit, label %.lr.ph.i33

.lr.ph.i33:                                       ; preds = %149
  %.sroa.212.0..sroa_idx.i35 = getelementptr inbounds i8, ptr %10, i64 8
  br label %152

150:                                              ; preds = %152
  %151 = add nuw i64 %.sroa.011.0.copyload.i39, %.sroa.0.014.i36
  %.not.i43 = icmp eq i64 %151, %1
  br i1 %.not.i43, label %.loopexit.loopexit, label %152

152:                                              ; preds = %150, %.lr.ph.i33
  %.sroa.0.014.i36 = phi i64 [ 0, %.lr.ph.i33 ], [ %151, %150 ]
  %153 = getelementptr inbounds i8, ptr %0, i64 %.sroa.0.014.i36
  %154 = sub nuw i64 %1, %.sroa.0.014.i36
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %10)
  call fastcc void %.sroa.2118.0.copyload(ptr nonnull sret({ i64, i16, [6 x i8] }) %10, ptr nonnull readonly align 1 %.sroa.0117.0.copyload, ptr nonnull readonly align 1 %153, i64 %154) #10, !noalias !61
  %.sroa.011.0.copyload.i39 = load i64, ptr %10, align 8
  %.sroa.212.0.copyload.i40 = load i16, ptr %.sroa.212.0..sroa_idx.i35, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %10)
  %.not7.i41 = icmp eq i16 %.sroa.212.0.copyload.i40, 0
  br i1 %.not7.i41, label %150, label %common.ret

.loopexit.loopexit:                               ; preds = %150
  %.sroa.0119.0.copyload.pre = load ptr, ptr %3, align 8
  %.sroa.2120.0.copyload.pre = load ptr, ptr %.sroa.2118.0..sroa_idx, align 8
  br label %.loopexit

.loopexit:                                        ; preds = %.loopexit.loopexit, %149
  %.sroa.2120.0.copyload = phi ptr [ %.sroa.2120.0.copyload.pre, %.loopexit.loopexit ], [ %.sroa.2118.0.copyload, %149 ]
  %.sroa.0119.0.copyload = phi ptr [ %.sroa.0119.0.copyload.pre, %.loopexit.loopexit ], [ %.sroa.0117.0.copyload, %149 ]
  %155 = extractvalue { ptr, i64 } %144, 0
  %156 = extractvalue { ptr, i64 } %144, 1
  %.not13.i.i = icmp eq i64 %156, 0
  %.sroa.212.0..sroa_idx.i.i = getelementptr inbounds i8, ptr %9, i64 8
  br i1 %.not13.i.i, label %common.ret, label %.lr.ph.i.i

.lr.ph.i.i:                                       ; preds = %.loopexit, %.loopexit.i
  %.sroa.0.06.i = phi i64 [ %162, %.loopexit.i ], [ 0, %.loopexit ]
  br label %159

157:                                              ; preds = %159
  %158 = add nuw i64 %.sroa.011.0.copyload.i.i, %.sroa.0.014.i.i
  %.not.i.i48 = icmp eq i64 %158, %156
  br i1 %.not.i.i48, label %.loopexit.i, label %159

159:                                              ; preds = %157, %.lr.ph.i.i
  %.sroa.0.014.i.i = phi i64 [ 0, %.lr.ph.i.i ], [ %158, %157 ]
  %160 = getelementptr inbounds i8, ptr %155, i64 %.sroa.0.014.i.i
  %161 = sub nuw i64 %156, %.sroa.0.014.i.i
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %9)
  call fastcc void %.sroa.2120.0.copyload(ptr nonnull sret({ i64, i16, [6 x i8] }) %9, ptr nonnull readonly align 1 %.sroa.0119.0.copyload, ptr nonnull readonly align 1 %160, i64 %161) #10, !noalias !64
  %.sroa.011.0.copyload.i.i = load i64, ptr %9, align 8
  %.sroa.212.0.copyload.i.i = load i16, ptr %.sroa.212.0..sroa_idx.i.i, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %9)
  %.not7.i.i = icmp eq i16 %.sroa.212.0.copyload.i.i, 0
  br i1 %.not7.i.i, label %157, label %common.ret

.loopexit.i:                                      ; preds = %157
  %162 = add nuw i64 %.sroa.0.06.i, 1
  %exitcond.not.i = icmp eq i64 %162, %58
  br i1 %exitcond.not.i, label %common.ret, label %.lr.ph.i.i

163:                                              ; preds = %143
  %164 = lshr i64 %58, 1
  %165 = add nuw i64 %58, 1
  %166 = lshr i64 %165, 1
  %.sroa.0121.0.copyload = load ptr, ptr %3, align 8
  %.sroa.2122.0..sroa_idx = getelementptr inbounds i8, ptr %3, i64 8
  %.sroa.2122.0.copyload = load ptr, ptr %.sroa.2122.0..sroa_idx, align 8
  %167 = extractvalue { ptr, i64 } %144, 0
  %168 = extractvalue { ptr, i64 } %144, 1
  %.not.i49 = icmp ult i64 %58, 2
  br i1 %.not.i49, label %.loopexit152, label %.lr.ph.i50

.lr.ph.i50:                                       ; preds = %163
  %.not13.i.i52 = icmp eq i64 %168, 0
  %.sroa.212.0..sroa_idx.i.i53 = getelementptr inbounds i8, ptr %8, i64 8
  br i1 %.not13.i.i52, label %.loopexit152, label %.lr.ph.i.i54

.lr.ph.i.i54:                                     ; preds = %.lr.ph.i50, %.loopexit.i64
  %.sroa.0.06.i55 = phi i64 [ %174, %.loopexit.i64 ], [ 0, %.lr.ph.i50 ]
  br label %171

169:                                              ; preds = %171
  %170 = add nuw i64 %.sroa.011.0.copyload.i.i59, %.sroa.0.014.i.i58
  %.not.i.i63 = icmp eq i64 %170, %168
  br i1 %.not.i.i63, label %.loopexit.i64, label %171

171:                                              ; preds = %169, %.lr.ph.i.i54
  %.sroa.0.014.i.i58 = phi i64 [ 0, %.lr.ph.i.i54 ], [ %170, %169 ]
  %172 = getelementptr inbounds i8, ptr %167, i64 %.sroa.0.014.i.i58
  %173 = sub nuw i64 %168, %.sroa.0.014.i.i58
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %8)
  call fastcc void %.sroa.2122.0.copyload(ptr nonnull sret({ i64, i16, [6 x i8] }) %8, ptr nonnull readonly align 1 %.sroa.0121.0.copyload, ptr nonnull readonly align 1 %172, i64 %173) #10, !noalias !67
  %.sroa.011.0.copyload.i.i59 = load i64, ptr %8, align 8
  %.sroa.212.0.copyload.i.i60 = load i16, ptr %.sroa.212.0..sroa_idx.i.i53, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %8)
  %.not7.i.i61 = icmp eq i16 %.sroa.212.0.copyload.i.i60, 0
  br i1 %.not7.i.i61, label %169, label %common.ret

.loopexit.i64:                                    ; preds = %169
  %174 = add nuw i64 %.sroa.0.06.i55, 1
  %exitcond.not.i65 = icmp eq i64 %174, %164
  br i1 %exitcond.not.i65, label %.loopexit152.loopexit, label %.lr.ph.i.i54

.loopexit152.loopexit:                            ; preds = %.loopexit.i64
  %.sroa.0123.0.copyload.pre = load ptr, ptr %3, align 8
  %.sroa.2124.0.copyload.pre = load ptr, ptr %.sroa.2122.0..sroa_idx, align 8
  br label %.loopexit152

.loopexit152:                                     ; preds = %.loopexit152.loopexit, %163, %.lr.ph.i50
  %.sroa.2124.0.copyload = phi ptr [ %.sroa.2124.0.copyload.pre, %.loopexit152.loopexit ], [ %.sroa.2122.0.copyload, %163 ], [ %.sroa.2122.0.copyload, %.lr.ph.i50 ]
  %.sroa.0123.0.copyload = phi ptr [ %.sroa.0123.0.copyload.pre, %.loopexit152.loopexit ], [ %.sroa.0121.0.copyload, %163 ], [ %.sroa.0121.0.copyload, %.lr.ph.i50 ]
  br i1 %.not13.i, label %.loopexit150, label %.lr.ph.i68

.lr.ph.i68:                                       ; preds = %.loopexit152
  %.sroa.212.0..sroa_idx.i70 = getelementptr inbounds i8, ptr %7, i64 8
  br label %177

175:                                              ; preds = %177
  %176 = add nuw i64 %.sroa.011.0.copyload.i74, %.sroa.0.014.i71
  %.not.i78 = icmp eq i64 %176, %1
  br i1 %.not.i78, label %.loopexit150, label %177

177:                                              ; preds = %175, %.lr.ph.i68
  %.sroa.0.014.i71 = phi i64 [ 0, %.lr.ph.i68 ], [ %176, %175 ]
  %178 = getelementptr inbounds i8, ptr %0, i64 %.sroa.0.014.i71
  %179 = sub nuw i64 %1, %.sroa.0.014.i71
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %7)
  call fastcc void %.sroa.2124.0.copyload(ptr nonnull sret({ i64, i16, [6 x i8] }) %7, ptr nonnull readonly align 1 %.sroa.0123.0.copyload, ptr nonnull readonly align 1 %178, i64 %179) #10, !noalias !70
  %.sroa.011.0.copyload.i74 = load i64, ptr %7, align 8
  %.sroa.212.0.copyload.i75 = load i16, ptr %.sroa.212.0..sroa_idx.i70, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %7)
  %.not7.i76 = icmp eq i16 %.sroa.212.0.copyload.i75, 0
  br i1 %.not7.i76, label %175, label %common.ret

.loopexit150:                                     ; preds = %175, %.loopexit152
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %14, ptr noundef nonnull align 8 dereferenceable(16) %3, i64 16, i1 false)
  %180 = call fastcc i16 @io.Writer.writeBytesNTimes(ptr nonnull readonly align 8 %14, ptr nonnull readonly align 1 %167, i64 %168, i64 %166)
  br label %common.ret

.lr.ph.i81:                                       ; preds = %143
  %.sroa.0125.0.copyload = load ptr, ptr %3, align 8
  %.sroa.2126.0..sroa_idx = getelementptr inbounds i8, ptr %3, i64 8
  %.sroa.2126.0.copyload = load ptr, ptr %.sroa.2126.0..sroa_idx, align 8
  %181 = extractvalue { ptr, i64 } %144, 0
  %182 = extractvalue { ptr, i64 } %144, 1
  %.not13.i.i83 = icmp eq i64 %182, 0
  %.sroa.212.0..sroa_idx.i.i84 = getelementptr inbounds i8, ptr %6, i64 8
  br i1 %.not13.i.i83, label %.loopexit156, label %.lr.ph.i.i85

.lr.ph.i.i85:                                     ; preds = %.lr.ph.i81, %.loopexit.i95
  %.sroa.0.06.i86 = phi i64 [ %188, %.loopexit.i95 ], [ 0, %.lr.ph.i81 ]
  br label %185

183:                                              ; preds = %185
  %184 = add nuw i64 %.sroa.011.0.copyload.i.i90, %.sroa.0.014.i.i89
  %.not.i.i94 = icmp eq i64 %184, %182
  br i1 %.not.i.i94, label %.loopexit.i95, label %185

185:                                              ; preds = %183, %.lr.ph.i.i85
  %.sroa.0.014.i.i89 = phi i64 [ 0, %.lr.ph.i.i85 ], [ %184, %183 ]
  %186 = getelementptr inbounds i8, ptr %181, i64 %.sroa.0.014.i.i89
  %187 = sub nuw i64 %182, %.sroa.0.014.i.i89
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %6)
  call fastcc void %.sroa.2126.0.copyload(ptr nonnull sret({ i64, i16, [6 x i8] }) %6, ptr nonnull readonly align 1 %.sroa.0125.0.copyload, ptr nonnull readonly align 1 %186, i64 %187) #10, !noalias !73
  %.sroa.011.0.copyload.i.i90 = load i64, ptr %6, align 8
  %.sroa.212.0.copyload.i.i91 = load i16, ptr %.sroa.212.0..sroa_idx.i.i84, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %6)
  %.not7.i.i92 = icmp eq i16 %.sroa.212.0.copyload.i.i91, 0
  br i1 %.not7.i.i92, label %183, label %common.ret

.loopexit.i95:                                    ; preds = %183
  %188 = add nuw i64 %.sroa.0.06.i86, 1
  %exitcond.not.i96 = icmp eq i64 %188, %58
  br i1 %exitcond.not.i96, label %.loopexit156.loopexit, label %.lr.ph.i.i85

.loopexit156.loopexit:                            ; preds = %.loopexit.i95
  %.sroa.0127.0.copyload.pre = load ptr, ptr %3, align 8
  %.sroa.2128.0.copyload.pre = load ptr, ptr %.sroa.2126.0..sroa_idx, align 8
  br label %.loopexit156

.loopexit156:                                     ; preds = %.loopexit156.loopexit, %.lr.ph.i81
  %.sroa.2128.0.copyload = phi ptr [ %.sroa.2128.0.copyload.pre, %.loopexit156.loopexit ], [ %.sroa.2126.0.copyload, %.lr.ph.i81 ]
  %.sroa.0127.0.copyload = phi ptr [ %.sroa.0127.0.copyload.pre, %.loopexit156.loopexit ], [ %.sroa.0125.0.copyload, %.lr.ph.i81 ]
  br i1 %.not13.i, label %common.ret, label %.lr.ph.i99

.lr.ph.i99:                                       ; preds = %.loopexit156
  %.sroa.212.0..sroa_idx.i101 = getelementptr inbounds i8, ptr %5, i64 8
  br label %191

189:                                              ; preds = %191
  %190 = add nuw i64 %.sroa.011.0.copyload.i105, %.sroa.0.014.i102
  %.not.i109 = icmp eq i64 %190, %1
  br i1 %.not.i109, label %common.ret, label %191

191:                                              ; preds = %189, %.lr.ph.i99
  %.sroa.0.014.i102 = phi i64 [ 0, %.lr.ph.i99 ], [ %190, %189 ]
  %192 = getelementptr inbounds i8, ptr %0, i64 %.sroa.0.014.i102
  %193 = sub nuw i64 %1, %.sroa.0.014.i102
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5)
  call fastcc void %.sroa.2128.0.copyload(ptr nonnull sret({ i64, i16, [6 x i8] }) %5, ptr nonnull readonly align 1 %.sroa.0127.0.copyload, ptr nonnull readonly align 1 %192, i64 %193) #10, !noalias !76
  %.sroa.011.0.copyload.i105 = load i64, ptr %5, align 8
  %.sroa.212.0.copyload.i106 = load i16, ptr %.sroa.212.0..sroa_idx.i101, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5)
  %.not7.i107 = icmp eq i16 %.sroa.212.0.copyload.i106, 0
  br i1 %.not7.i107, label %189, label %common.ret
}

define internal fastcc noundef i32 @"_main_for_host!_8445464738218c426e3b841f28ede2879d4391e6c33df6a616e872e199e47e"(%list.RocList %raw_args) unnamed_addr {
entry:
  %0 = alloca [24 x i8], align 8
  %1 = alloca [24 x i8], align 8
  %struct_alloca.i.i.i.i.i = alloca { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, align 8
  %result_value4.i231 = alloca %str.RocStr, align 8
  %result_value3.i232 = alloca %str.RocStr, align 8
  %struct_field2.i233 = alloca %str.RocStr, align 8
  %struct_field.i234 = alloca %str.RocStr, align 8
  %result_value4.i = alloca %str.RocStr, align 8
  %result_value3.i = alloca %str.RocStr, align 8
  %struct_field2.i = alloca %str.RocStr, align 8
  %struct_field.i = alloca %str.RocStr, align 8
  %load_element.i = alloca { %str.RocStr, %str.RocStr }, align 8
  %result_value.i215 = alloca { [6 x i64], i8 }, align 8
  %2 = alloca [24 x i8], align 8
  %const_str_store3.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value2.i3.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %load_element.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store9.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store8.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value7.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store6.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store5.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value4.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store3.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store1.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value.i2.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %load_element.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store7.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value5.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %load_element4.i.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value1.i.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value15.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %load_element13.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %3 = alloca [24 x i8], align 8
  %4 = alloca [33 x i8], align 1
  %5 = alloca [16 x i8], align 8
  %6 = alloca [8 x i8], align 8
  %7 = alloca [24 x i8], align 8
  %8 = alloca [11 x i8], align 1
  %result_value2.i.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value1.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value.i.i.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store.i.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %joinpoint_arg_alloca.i.i.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store7.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value5.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value.i.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store.i.i.i.i.i = alloca %str.RocStr, align 8
  %load_element4.i.i.i.i.i = alloca %str.RocStr, align 8
  %result_value1.i.i = alloca %str.RocStr, align 8
  %result_value.i.i.i = alloca { [4 x i64], i8 }, align 8
  %.sroa.215.i.i.i.i.i = alloca i8, align 8
  %.sroa.213.i.i.i.i.i = alloca i8, align 8
  %.sroa.2.i.i.i.i.i = alloca i8, align 8
  %result_value16.i.i = alloca %str.RocStr, align 8
  %result_value14.i.i = alloca %str.RocStr, align 8
  %result_value13.i.i = alloca %str.RocStr, align 8
  %result_value12.i.i = alloca %str.RocStr, align 8
  %result_value11.i.i = alloca %str.RocStr, align 8
  %result_value10.i.i = alloca %str.RocStr, align 8
  %const_str_store9.i.i = alloca %str.RocStr, align 8
  %result_value8.i.i = alloca %str.RocStr, align 8
  %const_str_store7.i.i = alloca %str.RocStr, align 8
  %result_value6.i.i = alloca %str.RocStr, align 8
  %const_str_store.i.i = alloca %str.RocStr, align 8
  %result_value.i.i = alloca %str.RocStr, align 8
  %joinpoint_arg_alloca.i47.i = alloca %str.RocStr, align 8
  %9 = alloca [8 x i8], align 4
  %10 = alloca [8 x i8], align 4
  %11 = alloca [4 x i8], align 4
  %.sroa.312.i.i.i.i.i.i.i.i = alloca i8, align 8
  %.sroa.3.i.i.i.i.i.i.i.i = alloca i8, align 8
  %const_str_store11.i.i.i.i = alloca %str.RocStr, align 8
  %const_str_store.i.i.i.i = alloca %str.RocStr, align 8
  %result_value.i.i.i.i = alloca { [3 x i64], i8 }, align 8
  %load_element.i.i.i = alloca { [3 x i64], i8 }, align 8
  %joinpoint_arg_alloca.i.i = alloca %str.RocStr, align 8
  %result_value27.i = alloca %str.RocStr, align 8
  %result_value26.i = alloca %str.RocStr, align 8
  %result_value25.i = alloca %str.RocStr, align 8
  %result_value24.i = alloca %str.RocStr, align 8
  %result_value23.i = alloca %str.RocStr, align 8
  %result_value22.i = alloca %str.RocStr, align 8
  %result_value21.i = alloca %str.RocStr, align 8
  %result_value20.i = alloca %str.RocStr, align 8
  %result_value19.i = alloca %str.RocStr, align 8
  %result_value18.i = alloca %str.RocStr, align 8
  %const_str_store17.i = alloca %str.RocStr, align 8
  %result_value16.i = alloca %str.RocStr, align 8
  %const_str_store15.i = alloca %str.RocStr, align 8
  %result_value14.i = alloca %str.RocStr, align 8
  %const_str_store13.i = alloca %str.RocStr, align 8
  %const_str_store12.i = alloca %str.RocStr, align 8
  %result_value11.i = alloca %str.RocStr, align 8
  %const_str_store10.i = alloca %str.RocStr, align 8
  %result_value9.i = alloca %str.RocStr, align 8
  %const_str_store7.i = alloca %str.RocStr, align 8
  %result_value.i = alloca %str.RocStr, align 8
  %result_value.i.sroa.7.i.i.i = alloca [7 x i8], align 1
  %result_value.sroa.7.i.i.i = alloca [7 x i8], align 1
  %result_value48 = alloca { [4 x i64], i8 }, align 8
  %result_value47 = alloca %str.RocStr, align 8
  %const_str_store46 = alloca %str.RocStr, align 8
  %const_str_store43 = alloca %str.RocStr, align 8
  %const_str_store40 = alloca %str.RocStr, align 8
  %const_str_store39 = alloca %str.RocStr, align 8
  %joinpoint_arg_alloca = alloca %str.RocStr, align 8
  %result_value27 = alloca %str.RocStr, align 8
  %list_len.i.i = extractvalue %list.RocList %raw_args, 1
  %.old.not.i.i.not.i.i = icmp eq i64 %list_len.i.i, 0
  br i1 %.old.not.i.i.not.i.i, label %List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i, label %12

12:                                               ; preds = %entry
  %cond.i.i = icmp eq i64 %list_len.i.i, 1
  %spec.select.i.i = select i1 %cond.i.i, i64 2, i64 %list_len.i.i
  %13 = shl nuw i64 %spec.select.i.i, 5
  %14 = or disjoint i64 %13, 16
  %15 = tail call ptr @roc_alloc(i64 %14, i32 8), !noalias !79
  %16 = icmp ne ptr %15, null
  tail call void @llvm.assume(i1 %16), !noalias !79
  %17 = getelementptr inbounds i8, ptr %15, i64 16
  %18 = getelementptr inbounds i8, ptr %15, i64 8
  store i64 1, ptr %18, align 8, !noalias !79
  br label %List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i

List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i: ; preds = %12, %entry
  %.sroa.7.0.i.i = phi i64 [ 0, %entry ], [ %spec.select.i.i, %12 ]
  %.sroa.06.0.i.i = phi ptr [ null, %entry ], [ %17, %12 ]
  %load_list.fca.0.insert.i.i = insertvalue %list.RocList poison, ptr %.sroa.06.0.i.i, 0
  %load_list.fca.1.insert.i.i = insertvalue %list.RocList %load_list.fca.0.insert.i.i, i64 0, 1
  %load_list.fca.2.insert.i.i = insertvalue %list.RocList %load_list.fca.1.insert.i.i, i64 %.sroa.7.0.i.i, 2
  %"#arg1.fca.0.extract.i.i.i.i" = extractvalue %list.RocList %raw_args, 0
  %"#arg1.fca.2.extract.i.i.i.i" = extractvalue %list.RocList %raw_args, 2
  %.not61.i.i.i.i = icmp slt i64 %"#arg1.fca.2.extract.i.i.i.i", 0
  %19 = select i1 %.not61.i.i.i.i, i64 %list_len.i.i, i64 %"#arg1.fca.2.extract.i.i.i.i"
  %20 = icmp ne i64 %19, 0
  %or.cond.i.i.i.i = or i1 %.not61.i.i.i.i, %20
  %21 = ptrtoint ptr %"#arg1.fca.0.extract.i.i.i.i" to i64
  %22 = shl i64 %"#arg1.fca.2.extract.i.i.i.i", 1
  br i1 %or.cond.i.i.i.i, label %list.RocList.refcount.exit.i.i.i.i, label %.thread.i.i.i.i

list.RocList.refcount.exit.i.i.i.i:               ; preds = %List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i
  %23 = select i1 %.not61.i.i.i.i, i64 %22, i64 %21
  %24 = inttoptr i64 %23 to ptr
  %25 = getelementptr inbounds i64, ptr %24, i64 -1
  %26 = load i64, ptr %25, align 8
  %27 = icmp ne i64 %26, 1
  %or.cond60.i.i.i.i = or i1 %.not61.i.i.i.i, %27
  br i1 %or.cond60.i.i.i.i, label %list.RocList.incref.exit.i.i.i.i, label %.thread.i.i.i.i

.thread.i.i.i.i:                                  ; preds = %list.RocList.refcount.exit.i.i.i.i, %List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i
  %.pre-phi70.i.i.i.i = phi i64 [ %23, %list.RocList.refcount.exit.i.i.i.i ], [ %21, %List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i ]
  %.not.i.i.i.i.i = icmp eq i64 %.pre-phi70.i.i.i.i, 0
  br i1 %.not.i.i.i.i.i, label %list.RocList.incref.exit.i.i.i.i, label %28

28:                                               ; preds = %.thread.i.i.i.i
  %29 = inttoptr i64 %.pre-phi70.i.i.i.i to ptr
  %30 = getelementptr inbounds i64, ptr %29, i64 -2
  store i64 %list_len.i.i, ptr %30, align 8
  br label %list.RocList.incref.exit.i.i.i.i

list.RocList.incref.exit.i.i.i.i:                 ; preds = %28, %.thread.i.i.i.i, %list.RocList.refcount.exit.i.i.i.i
  %31 = select i1 %.not61.i.i.i.i, i64 %22, i64 %21
  %.not.i1.i.i.i.i = icmp eq i64 %31, 0
  br i1 %.not.i1.i.i.i.i, label %"#Attr_#inc_19.exit.i.i.i", label %32

32:                                               ; preds = %list.RocList.incref.exit.i.i.i.i
  %33 = and i64 %31, -8
  %34 = add i64 %33, -8
  %35 = inttoptr i64 %34 to ptr
  %36 = load i64, ptr %35, align 8
  %.not.i.i.i.i.i.i = icmp eq i64 %36, 0
  br i1 %.not.i.i.i.i.i.i, label %"#Attr_#inc_19.exit.i.i.i", label %37

37:                                               ; preds = %32
  %38 = atomicrmw add ptr %35, i64 1 monotonic, align 8
  br label %"#Attr_#inc_19.exit.i.i.i"

"#Attr_#inc_19.exit.i.i.i":                       ; preds = %37, %32, %list.RocList.incref.exit.i.i.i.i
  br i1 %.old.not.i.i.not.i.i, label %else_block.i.i.i, label %then_block.i.i.i

then_block.i.i.i:                                 ; preds = %"#Attr_#inc_19.exit.i.i.i", %List_296_92df2e9c67226884f739cd53c0493c2aabaabd406877a3b72a3e676dc54e081.exit.i.i.i
  %joinpointarg324.i.i.i = phi i64 [ %add_int_wrap.i.i.i.i, %List_296_92df2e9c67226884f739cd53c0493c2aabaabd406877a3b72a3e676dc54e081.exit.i.i.i ], [ 0, %"#Attr_#inc_19.exit.i.i.i" ]
  %joinpointarg123.i.i.i = phi %list.RocList [ %load_list.fca.2.insert.i.i.i.i.i, %List_296_92df2e9c67226884f739cd53c0493c2aabaabd406877a3b72a3e676dc54e081.exit.i.i.i ], [ %load_list.fca.2.insert.i.i, %"#Attr_#inc_19.exit.i.i.i" ]
  %list_get_element.i.i.i.i = getelementptr inbounds { %list.RocList, %list.RocList, i1 }, ptr %"#arg1.fca.0.extract.i.i.i.i", i64 %joinpointarg324.i.i.i
  %result_value.sroa.0.0.copyload.i.i.i = load ptr, ptr %list_get_element.i.i.i.i, align 8
  %result_value.sroa.4.0.list_get_element.i.sroa_idx.i.i.i = getelementptr inbounds i8, ptr %list_get_element.i.i.i.i, i64 8
  %result_value.sroa.4.0.copyload.i.i.i = load i64, ptr %result_value.sroa.4.0.list_get_element.i.sroa_idx.i.i.i, align 8
  %result_value.sroa.5.0.list_get_element.i.sroa_idx.i.i.i = getelementptr inbounds i8, ptr %list_get_element.i.i.i.i, i64 16
  %result_value.sroa.5.0.copyload.i.i.i = load i64, ptr %result_value.sroa.5.0.list_get_element.i.sroa_idx.i.i.i, align 8
  %result_value.sroa.8.0.list_get_element.i.sroa_idx.i.i.i = getelementptr inbounds i8, ptr %list_get_element.i.i.i.i, i64 24
  %result_value.sroa.8.0.copyload.i.i.i = load ptr, ptr %result_value.sroa.8.0.list_get_element.i.sroa_idx.i.i.i, align 8
  %result_value.sroa.11.0.list_get_element.i.sroa_idx.i.i.i = getelementptr inbounds i8, ptr %list_get_element.i.i.i.i, i64 32
  %result_value.sroa.11.0.copyload.i.i.i = load i64, ptr %result_value.sroa.11.0.list_get_element.i.sroa_idx.i.i.i, align 8
  %result_value.sroa.12.0.list_get_element.i.sroa_idx.i.i.i = getelementptr inbounds i8, ptr %list_get_element.i.i.i.i, i64 40
  %result_value.sroa.12.0.copyload.i.i.i = load i64, ptr %result_value.sroa.12.0.list_get_element.i.sroa_idx.i.i.i, align 8
  %result_value.sroa.15.0.list_get_element.i.sroa_idx.i.i.i = getelementptr inbounds i8, ptr %list_get_element.i.i.i.i, i64 48
  %39 = load i8, ptr %result_value.sroa.15.0.list_get_element.i.sroa_idx.i.i.i, align 8
  %40 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i.i to i64
  %41 = shl i64 %result_value.sroa.5.0.copyload.i.i.i, 1
  %isneg.i.i.i.i.i = icmp slt i64 %result_value.sroa.5.0.copyload.i.i.i, 0
  %42 = select i1 %isneg.i.i.i.i.i, i64 %41, i64 %40
  %.not.i.i.i7.i.i.i = icmp eq i64 %42, 0
  %43 = and i8 %39, 1
  %.not.i.i.i = icmp eq i8 %43, 0
  br i1 %.not.i.i.i7.i.i.i, label %"#Attr_#inc_6.exit.i.i.i.i", label %44

44:                                               ; preds = %then_block.i.i.i
  %45 = and i64 %42, -8
  %46 = add i64 %45, -8
  %47 = inttoptr i64 %46 to ptr
  %48 = load i64, ptr %47, align 8
  %.not.i.i.i.i.i.i.i = icmp eq i64 %48, 0
  br i1 %.not.i.i.i.i.i.i.i, label %"#Attr_#inc_6.exit.i.i.i.i", label %49

49:                                               ; preds = %44
  %50 = atomicrmw add ptr %47, i64 1 monotonic, align 8
  br label %"#Attr_#inc_6.exit.i.i.i.i"

"#Attr_#inc_6.exit.i.i.i.i":                      ; preds = %49, %44, %then_block.i.i.i
  %51 = ptrtoint ptr %result_value.sroa.8.0.copyload.i.i.i to i64
  %52 = shl i64 %result_value.sroa.12.0.copyload.i.i.i, 1
  %isneg.i12.i.i.i.i = icmp slt i64 %result_value.sroa.12.0.copyload.i.i.i, 0
  %53 = select i1 %isneg.i12.i.i.i.i, i64 %52, i64 %51
  %.not.i.i13.i.i.i.i = icmp eq i64 %53, 0
  br i1 %.not.i.i13.i.i.i.i, label %"#Attr_#inc_20.exit.i.i.i", label %54

54:                                               ; preds = %"#Attr_#inc_6.exit.i.i.i.i"
  %55 = and i64 %53, -8
  %56 = add i64 %55, -8
  %57 = inttoptr i64 %56 to ptr
  %58 = load i64, ptr %57, align 8
  %.not.i.i.i14.i.i.i.i = icmp eq i64 %58, 0
  br i1 %.not.i.i.i14.i.i.i.i, label %"#Attr_#inc_20.exit.i.i.i", label %59

59:                                               ; preds = %54
  %60 = atomicrmw add ptr %57, i64 1 monotonic, align 8
  br label %"#Attr_#inc_20.exit.i.i.i"

"#Attr_#inc_20.exit.i.i.i":                       ; preds = %59, %54, %"#Attr_#inc_6.exit.i.i.i.i"
  br i1 %.not.i.i.i, label %then_block.i.i.i.i.i, label %else_block.i.i.i.i.i

common.ret.sink.split.i.i.i.i.i:                  ; preds = %76, %67
  %.sink34.i.i.i.i.i = phi ptr [ %65, %67 ], [ %74, %76 ]
  %struct_field4.unpack.sink.ph.i.i.i.i.i = phi ptr [ %result_value.sroa.0.0.copyload.i.i.i, %67 ], [ %result_value.sroa.8.0.copyload.i.i.i, %76 ]
  %struct_field4.unpack12.sink.ph.i.i.i.i.i = phi i64 [ %result_value.sroa.4.0.copyload.i.i.i, %67 ], [ %result_value.sroa.11.0.copyload.i.i.i, %76 ]
  %struct_field4.unpack14.sink.ph.i.i.i.i.i = phi i64 [ %result_value.sroa.5.0.copyload.i.i.i, %67 ], [ %result_value.sroa.12.0.copyload.i.i.i, %76 ]
  %.sink.ph.i.i.i.i.i = phi i8 [ 0, %67 ], [ 1, %76 ]
  tail call void @roc_dealloc(ptr nonnull align 1 %.sink34.i.i.i.i.i, i32 8)
  br label %InternalArg_to_os_raw_e26b07323a88152db96f57cffd313b88dda6eb16db54bca99d36e02da3083.exit.i.i.i.i

then_block.i.i.i.i.i:                             ; preds = %"#Attr_#inc_20.exit.i.i.i"
  %61 = icmp ne i64 %result_value.sroa.12.0.copyload.i.i.i, 0
  %62 = icmp ne i64 %53, 0
  %or.cond.i.i.i.i.i.i.i = and i1 %61, %62
  br i1 %or.cond.i.i.i.i.i.i.i, label %63, label %InternalArg_to_os_raw_e26b07323a88152db96f57cffd313b88dda6eb16db54bca99d36e02da3083.exit.i.i.i.i

63:                                               ; preds = %then_block.i.i.i.i.i
  %64 = inttoptr i64 %53 to ptr
  %65 = getelementptr inbounds i64, ptr %64, i64 -1
  %66 = load i64, ptr %65, align 8
  %.not.i.i.i.i9.i.i.i = icmp eq i64 %66, 0
  br i1 %.not.i.i.i.i9.i.i.i, label %InternalArg_to_os_raw_e26b07323a88152db96f57cffd313b88dda6eb16db54bca99d36e02da3083.exit.i.i.i.i, label %67

67:                                               ; preds = %63
  %68 = atomicrmw sub ptr %65, i64 1 monotonic, align 8
  %69 = icmp eq i64 %68, 1
  br i1 %69, label %common.ret.sink.split.i.i.i.i.i, label %InternalArg_to_os_raw_e26b07323a88152db96f57cffd313b88dda6eb16db54bca99d36e02da3083.exit.i.i.i.i

else_block.i.i.i.i.i:                             ; preds = %"#Attr_#inc_20.exit.i.i.i"
  %70 = icmp ne i64 %result_value.sroa.5.0.copyload.i.i.i, 0
  %71 = icmp ne i64 %42, 0
  %or.cond.i.i32.i.i.i.i.i = and i1 %70, %71
  br i1 %or.cond.i.i32.i.i.i.i.i, label %72, label %InternalArg_to_os_raw_e26b07323a88152db96f57cffd313b88dda6eb16db54bca99d36e02da3083.exit.i.i.i.i

72:                                               ; preds = %else_block.i.i.i.i.i
  %73 = inttoptr i64 %42 to ptr
  %74 = getelementptr inbounds i64, ptr %73, i64 -1
  %75 = load i64, ptr %74, align 8
  %.not.i.i33.i.i.i.i.i = icmp eq i64 %75, 0
  br i1 %.not.i.i33.i.i.i.i.i, label %InternalArg_to_os_raw_e26b07323a88152db96f57cffd313b88dda6eb16db54bca99d36e02da3083.exit.i.i.i.i, label %76

76:                                               ; preds = %72
  %77 = atomicrmw sub ptr %74, i64 1 monotonic, align 8
  %78 = icmp eq i64 %77, 1
  br i1 %78, label %common.ret.sink.split.i.i.i.i.i, label %InternalArg_to_os_raw_e26b07323a88152db96f57cffd313b88dda6eb16db54bca99d36e02da3083.exit.i.i.i.i

InternalArg_to_os_raw_e26b07323a88152db96f57cffd313b88dda6eb16db54bca99d36e02da3083.exit.i.i.i.i: ; preds = %76, %72, %else_block.i.i.i.i.i, %67, %63, %then_block.i.i.i.i.i, %common.ret.sink.split.i.i.i.i.i
  %struct_field4.unpack.sink.i.i.i.i.i = phi ptr [ %result_value.sroa.0.0.copyload.i.i.i, %then_block.i.i.i.i.i ], [ %result_value.sroa.0.0.copyload.i.i.i, %63 ], [ %result_value.sroa.0.0.copyload.i.i.i, %67 ], [ %result_value.sroa.8.0.copyload.i.i.i, %else_block.i.i.i.i.i ], [ %result_value.sroa.8.0.copyload.i.i.i, %72 ], [ %result_value.sroa.8.0.copyload.i.i.i, %76 ], [ %struct_field4.unpack.sink.ph.i.i.i.i.i, %common.ret.sink.split.i.i.i.i.i ]
  %struct_field4.unpack12.sink.i.i.i.i.i = phi i64 [ %result_value.sroa.4.0.copyload.i.i.i, %then_block.i.i.i.i.i ], [ %result_value.sroa.4.0.copyload.i.i.i, %63 ], [ %result_value.sroa.4.0.copyload.i.i.i, %67 ], [ %result_value.sroa.11.0.copyload.i.i.i, %else_block.i.i.i.i.i ], [ %result_value.sroa.11.0.copyload.i.i.i, %72 ], [ %result_value.sroa.11.0.copyload.i.i.i, %76 ], [ %struct_field4.unpack12.sink.ph.i.i.i.i.i, %common.ret.sink.split.i.i.i.i.i ]
  %struct_field4.unpack14.sink.i.i.i.i.i = phi i64 [ %result_value.sroa.5.0.copyload.i.i.i, %then_block.i.i.i.i.i ], [ %result_value.sroa.5.0.copyload.i.i.i, %63 ], [ %result_value.sroa.5.0.copyload.i.i.i, %67 ], [ %result_value.sroa.12.0.copyload.i.i.i, %else_block.i.i.i.i.i ], [ %result_value.sroa.12.0.copyload.i.i.i, %72 ], [ %result_value.sroa.12.0.copyload.i.i.i, %76 ], [ %struct_field4.unpack14.sink.ph.i.i.i.i.i, %common.ret.sink.split.i.i.i.i.i ]
  %.sink.i.i.i.i.i = phi i8 [ 0, %then_block.i.i.i.i.i ], [ 0, %63 ], [ 0, %67 ], [ 1, %else_block.i.i.i.i.i ], [ 1, %72 ], [ 1, %76 ], [ %.sink.ph.i.i.i.i.i, %common.ret.sink.split.i.i.i.i.i ]
  %"#arg1.fca.0.extract.i.i.i.i.i" = extractvalue %list.RocList %joinpointarg123.i.i.i, 0
  %"#arg1.fca.1.extract.i.i.i.i.i" = extractvalue %list.RocList %joinpointarg123.i.i.i, 1
  %.not.i.i8.i.i.i = icmp eq ptr %"#arg1.fca.0.extract.i.i.i.i.i", null
  br i1 %.not.i.i8.i.i.i, label %List_296_92df2e9c67226884f739cd53c0493c2aabaabd406877a3b72a3e676dc54e081.exit.i.i.i, label %79

79:                                               ; preds = %InternalArg_to_os_raw_e26b07323a88152db96f57cffd313b88dda6eb16db54bca99d36e02da3083.exit.i.i.i.i
  %80 = shl nuw i64 %"#arg1.fca.1.extract.i.i.i.i.i", 5
  %81 = getelementptr inbounds i8, ptr %"#arg1.fca.0.extract.i.i.i.i.i", i64 %80
  store ptr %struct_field4.unpack.sink.i.i.i.i.i, ptr %81, align 8
  %result_value.sroa.2.0..sroa_idx.i.i.i.i = getelementptr inbounds i8, ptr %81, i64 8
  store i64 %struct_field4.unpack12.sink.i.i.i.i.i, ptr %result_value.sroa.2.0..sroa_idx.i.i.i.i, align 8
  %result_value.sroa.3.0..sroa_idx.i.i.i.i = getelementptr inbounds i8, ptr %81, i64 16
  store i64 %struct_field4.unpack14.sink.i.i.i.i.i, ptr %result_value.sroa.3.0..sroa_idx.i.i.i.i, align 8
  %result_value.sroa.4.0..sroa_idx.i.i.i.i = getelementptr inbounds i8, ptr %81, i64 24
  store i8 %.sink.i.i.i.i.i, ptr %result_value.sroa.4.0..sroa_idx.i.i.i.i, align 8
  br label %List_296_92df2e9c67226884f739cd53c0493c2aabaabd406877a3b72a3e676dc54e081.exit.i.i.i

List_296_92df2e9c67226884f739cd53c0493c2aabaabd406877a3b72a3e676dc54e081.exit.i.i.i: ; preds = %79, %InternalArg_to_os_raw_e26b07323a88152db96f57cffd313b88dda6eb16db54bca99d36e02da3083.exit.i.i.i.i
  %"#arg1.fca.2.extract.i.i.i.i.i" = extractvalue %list.RocList %joinpointarg123.i.i.i, 2
  %82 = add nuw i64 %"#arg1.fca.1.extract.i.i.i.i.i", 1
  %load_list.fca.1.insert.i.i.i.i.i = insertvalue %list.RocList %joinpointarg123.i.i.i, i64 %82, 1
  %load_list.fca.2.insert.i.i.i.i.i = insertvalue %list.RocList %load_list.fca.1.insert.i.i.i.i.i, i64 %"#arg1.fca.2.extract.i.i.i.i.i", 2
  %add_int_wrap.i.i.i.i = add nuw i64 %joinpointarg324.i.i.i, 1
  %exitcond.not.i.i.i = icmp eq i64 %add_int_wrap.i.i.i.i, %list_len.i.i
  br i1 %exitcond.not.i.i.i, label %else_block.i.i.i, label %then_block.i.i.i

else_block.i.i.i:                                 ; preds = %List_296_92df2e9c67226884f739cd53c0493c2aabaabd406877a3b72a3e676dc54e081.exit.i.i.i, %"#Attr_#inc_19.exit.i.i.i"
  %joinpointarg1.lcssa.i.i.i = phi %list.RocList [ %load_list.fca.2.insert.i.i, %"#Attr_#inc_19.exit.i.i.i" ], [ %load_list.fca.2.insert.i.i.i.i.i, %List_296_92df2e9c67226884f739cd53c0493c2aabaabd406877a3b72a3e676dc54e081.exit.i.i.i ]
  br i1 %or.cond.i.i.i.i, label %list.RocList.refcount.exit.i21.i.i.i, label %list.RocList.refcount.exit.thread.i.i.i.i

list.RocList.refcount.exit.i21.i.i.i:             ; preds = %else_block.i.i.i
  %83 = inttoptr i64 %31 to ptr
  %84 = getelementptr inbounds i64, ptr %83, i64 -1
  %85 = load i64, ptr %84, align 8
  %86 = icmp eq i64 %85, 1
  br i1 %86, label %list.RocList.refcount.exit.thread.i.i.i.i, label %roc_builtins.list.decref.exit.i.i.i.i

list.RocList.refcount.exit.thread.i.i.i.i:        ; preds = %list.RocList.refcount.exit.i21.i.i.i, %else_block.i.i.i
  %.pre-phi89.i.i.i.i = phi ptr [ %83, %list.RocList.refcount.exit.i21.i.i.i ], [ %"#arg1.fca.0.extract.i.i.i.i", %else_block.i.i.i ]
  %.pre-phi87.i.i.i.i = phi i64 [ %31, %list.RocList.refcount.exit.i21.i.i.i ], [ %21, %else_block.i.i.i ]
  %.not.i.i.i14.i.i.i = icmp eq i64 %.pre-phi87.i.i.i.i, 0
  br i1 %.not.i.i.i14.i.i.i, label %roc_builtins.list.decref.exit.i.i.i.i, label %87

87:                                               ; preds = %list.RocList.refcount.exit.thread.i.i.i.i
  br i1 %.not61.i.i.i.i, label %88, label %list.RocList.getAllocationElementCount.exit.i.i.i.i

88:                                               ; preds = %87
  %89 = getelementptr inbounds i64, ptr %.pre-phi89.i.i.i.i, i64 -2
  %common.ret.op.in.i.sroa.speculate.load.28.i.i.i.i = load i64, ptr %89, align 8
  br label %list.RocList.getAllocationElementCount.exit.i.i.i.i

list.RocList.getAllocationElementCount.exit.i.i.i.i: ; preds = %88, %87
  %common.ret.op.in.i.sroa.speculated.i.i.i.i = phi i64 [ %common.ret.op.in.i.sroa.speculate.load.28.i.i.i.i, %88 ], [ %list_len.i.i, %87 ]
  %.not.i.i.i.i = icmp eq i64 %common.ret.op.in.i.sroa.speculated.i.i.i.i, 0
  br i1 %.not.i.i.i.i, label %roc_builtins.list.decref.exit.i.i.i.i, label %.lr.ph.i.i.i.i

.lr.ph.i.i.i.i:                                   ; preds = %list.RocList.getAllocationElementCount.exit.i.i.i.i, %"#Attr_#dec_20.exit.i.i.i.i"
  %.sroa.0.0.i.i81.i.i.i.i = phi i64 [ %118, %"#Attr_#dec_20.exit.i.i.i.i" ], [ 0, %list.RocList.getAllocationElementCount.exit.i.i.i.i ]
  %90 = mul nuw i64 %.sroa.0.0.i.i81.i.i.i.i, 56
  %91 = getelementptr inbounds i8, ptr %.pre-phi89.i.i.i.i, i64 %90
  %struct_field.unpack.i.i.i.i.i = load ptr, ptr %91, align 8
  %struct_field.elt4.i.i.i.i.i = getelementptr inbounds %list.RocList, ptr %91, i64 0, i32 2
  %struct_field.unpack5.i.i.i.i.i = load i64, ptr %struct_field.elt4.i.i.i.i.i, align 8
  %92 = ptrtoint ptr %struct_field.unpack.i.i.i.i.i to i64
  %93 = shl i64 %struct_field.unpack5.i.i.i.i.i, 1
  %isneg.i.i.i15.i.i.i = icmp slt i64 %struct_field.unpack5.i.i.i.i.i, 0
  %94 = select i1 %isneg.i.i.i15.i.i.i, i64 %93, i64 %92
  %95 = icmp ne i64 %struct_field.unpack5.i.i.i.i.i, 0
  %96 = icmp ne i64 %94, 0
  %or.cond.i.i.i.i16.i.i.i = select i1 %95, i1 %96, i1 false
  br i1 %or.cond.i.i.i.i16.i.i.i, label %97, label %"#Attr_#dec_6.exit.i.i.i.i.i"

97:                                               ; preds = %.lr.ph.i.i.i.i
  %98 = inttoptr i64 %94 to ptr
  %99 = getelementptr inbounds i64, ptr %98, i64 -1
  %100 = load i64, ptr %99, align 8
  %.not.i.i.i.i20.i.i.i = icmp eq i64 %100, 0
  br i1 %.not.i.i.i.i20.i.i.i, label %"#Attr_#dec_6.exit.i.i.i.i.i", label %101

101:                                              ; preds = %97
  %102 = atomicrmw sub ptr %99, i64 1 monotonic, align 8
  %103 = icmp eq i64 %102, 1
  br i1 %103, label %104, label %"#Attr_#dec_6.exit.i.i.i.i.i"

104:                                              ; preds = %101
  tail call void @roc_dealloc(ptr nonnull align 1 %99, i32 8)
  br label %"#Attr_#dec_6.exit.i.i.i.i.i"

"#Attr_#dec_6.exit.i.i.i.i.i":                    ; preds = %104, %101, %97, %.lr.ph.i.i.i.i
  %struct_field_access_record_1.i.i17.i.i.i = getelementptr inbounds { %list.RocList, %list.RocList, i1 }, ptr %91, i64 0, i32 1
  %struct_field1.unpack.i.i18.i.i.i = load ptr, ptr %struct_field_access_record_1.i.i17.i.i.i, align 8
  %struct_field1.elt9.i.i.i.i.i = getelementptr inbounds { %list.RocList, %list.RocList, i1 }, ptr %91, i64 0, i32 1, i32 2
  %struct_field1.unpack10.i.i.i.i.i = load i64, ptr %struct_field1.elt9.i.i.i.i.i, align 8
  %105 = ptrtoint ptr %struct_field1.unpack.i.i18.i.i.i to i64
  %106 = shl i64 %struct_field1.unpack10.i.i.i.i.i, 1
  %isneg.i12.i.i.i.i.i = icmp slt i64 %struct_field1.unpack10.i.i.i.i.i, 0
  %107 = select i1 %isneg.i12.i.i.i.i.i, i64 %106, i64 %105
  %108 = icmp ne i64 %struct_field1.unpack10.i.i.i.i.i, 0
  %109 = icmp ne i64 %107, 0
  %or.cond.i.i13.i.i.i.i.i = select i1 %108, i1 %109, i1 false
  br i1 %or.cond.i.i13.i.i.i.i.i, label %110, label %"#Attr_#dec_20.exit.i.i.i.i"

110:                                              ; preds = %"#Attr_#dec_6.exit.i.i.i.i.i"
  %111 = inttoptr i64 %107 to ptr
  %112 = getelementptr inbounds i64, ptr %111, i64 -1
  %113 = load i64, ptr %112, align 8
  %.not.i.i14.i.i.i.i.i = icmp eq i64 %113, 0
  br i1 %.not.i.i14.i.i.i.i.i, label %"#Attr_#dec_20.exit.i.i.i.i", label %114

114:                                              ; preds = %110
  %115 = atomicrmw sub ptr %112, i64 1 monotonic, align 8
  %116 = icmp eq i64 %115, 1
  br i1 %116, label %117, label %"#Attr_#dec_20.exit.i.i.i.i"

117:                                              ; preds = %114
  tail call void @roc_dealloc(ptr nonnull align 1 %112, i32 8)
  br label %"#Attr_#dec_20.exit.i.i.i.i"

"#Attr_#dec_20.exit.i.i.i.i":                     ; preds = %117, %114, %110, %"#Attr_#dec_6.exit.i.i.i.i.i"
  %118 = add nuw i64 %.sroa.0.0.i.i81.i.i.i.i, 1
  %exitcond.not.i.i.i.i = icmp eq i64 %118, %common.ret.op.in.i.sroa.speculated.i.i.i.i
  br i1 %exitcond.not.i.i.i.i, label %roc_builtins.list.decref.exit.i.i.i.i, label %.lr.ph.i.i.i.i

roc_builtins.list.decref.exit.i.i.i.i:            ; preds = %"#Attr_#dec_20.exit.i.i.i.i", %list.RocList.getAllocationElementCount.exit.i.i.i.i, %list.RocList.refcount.exit.thread.i.i.i.i, %list.RocList.refcount.exit.i21.i.i.i
  %119 = icmp ne i64 %"#arg1.fca.2.extract.i.i.i.i", 0
  %120 = icmp ne i64 %31, 0
  %or.cond.i.i.i.i.i = and i1 %119, %120
  br i1 %or.cond.i.i.i.i.i, label %121, label %List_map_11922f5e717257e3c76632973ee406cbf106889cd4e80e37d14c1d9c194671.exit

121:                                              ; preds = %roc_builtins.list.decref.exit.i.i.i.i
  %122 = inttoptr i64 %31 to ptr
  %123 = getelementptr inbounds i64, ptr %122, i64 -1
  %124 = load i64, ptr %123, align 8
  %.not.i.i19.i.i.i = icmp eq i64 %124, 0
  br i1 %.not.i.i19.i.i.i, label %List_map_11922f5e717257e3c76632973ee406cbf106889cd4e80e37d14c1d9c194671.exit, label %125

125:                                              ; preds = %121
  %126 = atomicrmw sub ptr %123, i64 1 monotonic, align 8
  %127 = icmp eq i64 %126, 1
  br i1 %127, label %128, label %List_map_11922f5e717257e3c76632973ee406cbf106889cd4e80e37d14c1d9c194671.exit

128:                                              ; preds = %125
  %129 = getelementptr inbounds i64, ptr %122, i64 -2
  tail call void @roc_dealloc(ptr nonnull align 1 %129, i32 8)
  br label %List_map_11922f5e717257e3c76632973ee406cbf106889cd4e80e37d14c1d9c194671.exit

List_map_11922f5e717257e3c76632973ee406cbf106889cd4e80e37d14c1d9c194671.exit: ; preds = %roc_builtins.list.decref.exit.i.i.i.i, %121, %125, %128
  %list_len.i.i68 = extractvalue %list.RocList %joinpointarg1.lcssa.i.i.i, 1
  %.old.not.i.i.not.i.i69 = icmp eq i64 %list_len.i.i68, 0
  br i1 %.old.not.i.i.not.i.i69, label %List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i72, label %130

130:                                              ; preds = %List_map_11922f5e717257e3c76632973ee406cbf106889cd4e80e37d14c1d9c194671.exit
  %cond.i.i70 = icmp eq i64 %list_len.i.i68, 1
  %spec.select.i.i71 = select i1 %cond.i.i70, i64 2, i64 %list_len.i.i68
  %131 = shl nuw i64 %spec.select.i.i71, 5
  %132 = or disjoint i64 %131, 16
  %133 = tail call ptr @roc_alloc(i64 %132, i32 8), !noalias !88
  %134 = icmp ne ptr %133, null
  tail call void @llvm.assume(i1 %134), !noalias !88
  %135 = getelementptr inbounds i8, ptr %133, i64 16
  %136 = getelementptr inbounds i8, ptr %133, i64 8
  store i64 1, ptr %136, align 8, !noalias !88
  br label %List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i72

List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i72: ; preds = %130, %List_map_11922f5e717257e3c76632973ee406cbf106889cd4e80e37d14c1d9c194671.exit
  %.sroa.7.0.i.i73 = phi i64 [ 0, %List_map_11922f5e717257e3c76632973ee406cbf106889cd4e80e37d14c1d9c194671.exit ], [ %spec.select.i.i71, %130 ]
  %.sroa.06.0.i.i74 = phi ptr [ null, %List_map_11922f5e717257e3c76632973ee406cbf106889cd4e80e37d14c1d9c194671.exit ], [ %135, %130 ]
  %load_list.fca.0.insert.i.i75 = insertvalue %list.RocList poison, ptr %.sroa.06.0.i.i74, 0
  %load_list.fca.1.insert.i.i76 = insertvalue %list.RocList %load_list.fca.0.insert.i.i75, i64 0, 1
  %load_list.fca.2.insert.i.i77 = insertvalue %list.RocList %load_list.fca.1.insert.i.i76, i64 %.sroa.7.0.i.i73, 2
  call void @llvm.lifetime.start.p0(i64 7, ptr nonnull %result_value.sroa.7.i.i.i)
  %"#arg1.fca.0.extract.i.i.i.i78" = extractvalue %list.RocList %joinpointarg1.lcssa.i.i.i, 0
  %"#arg1.fca.2.extract.i.i.i.i79" = extractvalue %list.RocList %joinpointarg1.lcssa.i.i.i, 2
  %.not61.i.i.i.i80 = icmp slt i64 %"#arg1.fca.2.extract.i.i.i.i79", 0
  %137 = select i1 %.not61.i.i.i.i80, i64 %list_len.i.i68, i64 %"#arg1.fca.2.extract.i.i.i.i79"
  %138 = icmp ne i64 %137, 0
  %or.cond.i.i.i.i81 = or i1 %.not61.i.i.i.i80, %138
  %139 = ptrtoint ptr %"#arg1.fca.0.extract.i.i.i.i78" to i64
  %140 = shl i64 %"#arg1.fca.2.extract.i.i.i.i79", 1
  br i1 %or.cond.i.i.i.i81, label %list.RocList.refcount.exit.i.i.i.i105, label %.thread.i.i.i.i82

list.RocList.refcount.exit.i.i.i.i105:            ; preds = %List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i72
  %141 = select i1 %.not61.i.i.i.i80, i64 %140, i64 %139
  %142 = inttoptr i64 %141 to ptr
  %143 = getelementptr inbounds i64, ptr %142, i64 -1
  %144 = load i64, ptr %143, align 8
  %145 = icmp ne i64 %144, 1
  %or.cond60.i.i.i.i106 = or i1 %.not61.i.i.i.i80, %145
  br i1 %or.cond60.i.i.i.i106, label %list.RocList.incref.exit.i.i.i.i85, label %.thread.i.i.i.i82

.thread.i.i.i.i82:                                ; preds = %list.RocList.refcount.exit.i.i.i.i105, %List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i72
  %.pre-phi70.i.i.i.i83 = phi i64 [ %141, %list.RocList.refcount.exit.i.i.i.i105 ], [ %139, %List_with_capacity_5fea3a382f6b6c4a2af77ea4365b5abbdda8b93d1f0b9b895dc2a48489fb2.exit.i72 ]
  %.not.i.i.i.i.i84 = icmp eq i64 %.pre-phi70.i.i.i.i83, 0
  br i1 %.not.i.i.i.i.i84, label %list.RocList.incref.exit.i.i.i.i85, label %146

146:                                              ; preds = %.thread.i.i.i.i82
  %147 = inttoptr i64 %.pre-phi70.i.i.i.i83 to ptr
  %148 = getelementptr inbounds i64, ptr %147, i64 -2
  store i64 %list_len.i.i68, ptr %148, align 8
  br label %list.RocList.incref.exit.i.i.i.i85

list.RocList.incref.exit.i.i.i.i85:               ; preds = %146, %.thread.i.i.i.i82, %list.RocList.refcount.exit.i.i.i.i105
  %149 = select i1 %.not61.i.i.i.i80, i64 %140, i64 %139
  %.not.i1.i.i.i.i86 = icmp eq i64 %149, 0
  br i1 %.not.i1.i.i.i.i86, label %"#Attr_#inc_4.exit.i.i.i", label %150

150:                                              ; preds = %list.RocList.incref.exit.i.i.i.i85
  %151 = and i64 %149, -8
  %152 = add i64 %151, -8
  %153 = inttoptr i64 %152 to ptr
  %154 = load i64, ptr %153, align 8
  %.not.i.i.i.i.i.i87 = icmp eq i64 %154, 0
  br i1 %.not.i.i.i.i.i.i87, label %"#Attr_#inc_4.exit.i.i.i", label %155

155:                                              ; preds = %150
  %156 = atomicrmw add ptr %153, i64 1 monotonic, align 8
  br label %"#Attr_#inc_4.exit.i.i.i"

"#Attr_#inc_4.exit.i.i.i":                        ; preds = %155, %150, %list.RocList.incref.exit.i.i.i.i85
  br i1 %.old.not.i.i.not.i.i69, label %List_map_b7c59c3aec44645db91b229c81990d288c86aeb49f116d1eae85e2b9a39999f.exit, label %then_block.i.i.i88

then_block.i.i.i88:                               ; preds = %"#Attr_#inc_4.exit.i.i.i", %List_296_7f8c4a473141d41efa7657e3f378539f18179e0b2dff0f626f6dce25d295f.exit.i.i.i
  %joinpointarg316.i.i.i = phi i64 [ %add_int_wrap.i.i.i.i99, %List_296_7f8c4a473141d41efa7657e3f378539f18179e0b2dff0f626f6dce25d295f.exit.i.i.i ], [ 0, %"#Attr_#inc_4.exit.i.i.i" ]
  %joinpointarg115.i.i.i = phi %list.RocList [ %load_list.fca.2.insert.i.i.i.i.i98, %List_296_7f8c4a473141d41efa7657e3f378539f18179e0b2dff0f626f6dce25d295f.exit.i.i.i ], [ %load_list.fca.2.insert.i.i77, %"#Attr_#inc_4.exit.i.i.i" ]
  %list_get_element.i.i.i.i89 = getelementptr inbounds { [3 x i64], i8 }, ptr %"#arg1.fca.0.extract.i.i.i.i78", i64 %joinpointarg316.i.i.i
  %result_value.sroa.0.0.copyload.i.i.i90 = load ptr, ptr %list_get_element.i.i.i.i89, align 8
  %result_value.sroa.4.0.list_get_element.i.sroa_idx.i.i.i91 = getelementptr inbounds i8, ptr %list_get_element.i.i.i.i89, i64 8
  %result_value.sroa.4.0.copyload.i.i.i92 = load i64, ptr %result_value.sroa.4.0.list_get_element.i.sroa_idx.i.i.i91, align 8
  %result_value.sroa.411.0.list_get_element.i.sroa_idx.i.i.i = getelementptr inbounds i8, ptr %list_get_element.i.i.i.i89, i64 16
  %result_value.sroa.411.0.copyload.i.i.i = load i64, ptr %result_value.sroa.411.0.list_get_element.i.sroa_idx.i.i.i, align 8
  %result_value.sroa.6.0.list_get_element.i.sroa_idx.i.i.i = getelementptr inbounds i8, ptr %list_get_element.i.i.i.i89, i64 24
  %result_value.sroa.6.0.copyload.i.i.i = load i8, ptr %result_value.sroa.6.0.list_get_element.i.sroa_idx.i.i.i, align 8
  %result_value.sroa.7.0.list_get_element.i.sroa_idx.i.i.i = getelementptr inbounds i8, ptr %list_get_element.i.i.i.i89, i64 25
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(7) %result_value.sroa.7.i.i.i, ptr noundef nonnull align 1 dereferenceable(7) %result_value.sroa.7.0.list_get_element.i.sroa_idx.i.i.i, i64 7, i1 false)
  switch i8 %result_value.sroa.6.0.copyload.i.i.i, label %"#Attr_#inc_5.exit.i.i.i" [
    i8 0, label %tag_id_modify.i.i.i.i
    i8 1, label %tag_id_modify1.i.i.i.i
  ]

modify_rc_union_merge.sink.split.i.i.i.i:         ; preds = %169, %161
  %.sink.i.i.i.i = phi ptr [ %164, %161 ], [ %172, %169 ]
  %157 = atomicrmw add ptr %.sink.i.i.i.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_5.exit.i.i.i"

tag_id_modify.i.i.i.i:                            ; preds = %then_block.i.i.i88
  %158 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i.i90 to i64
  %159 = shl i64 %result_value.sroa.411.0.copyload.i.i.i, 1
  %isneg.i.i.i.i.i102 = icmp slt i64 %result_value.sroa.411.0.copyload.i.i.i, 0
  %160 = select i1 %isneg.i.i.i.i.i102, i64 %159, i64 %158
  %.not.i.i.i7.i.i.i103 = icmp eq i64 %160, 0
  br i1 %.not.i.i.i7.i.i.i103, label %"#Attr_#inc_5.exit.i.i.i", label %161

161:                                              ; preds = %tag_id_modify.i.i.i.i
  %162 = and i64 %160, -8
  %163 = add i64 %162, -8
  %164 = inttoptr i64 %163 to ptr
  %165 = load i64, ptr %164, align 8
  %.not.i.i.i.i.i.i.i104 = icmp eq i64 %165, 0
  br i1 %.not.i.i.i.i.i.i.i104, label %"#Attr_#inc_5.exit.i.i.i", label %modify_rc_union_merge.sink.split.i.i.i.i

tag_id_modify1.i.i.i.i:                           ; preds = %then_block.i.i.i88
  %166 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i.i90 to i64
  %167 = shl i64 %result_value.sroa.411.0.copyload.i.i.i, 1
  %isneg.i15.i.i.i.i = icmp slt i64 %result_value.sroa.411.0.copyload.i.i.i, 0
  %168 = select i1 %isneg.i15.i.i.i.i, i64 %167, i64 %166
  %.not.i.i16.i.i.i.i = icmp eq i64 %168, 0
  br i1 %.not.i.i16.i.i.i.i, label %"#Attr_#inc_5.exit.i.i.i", label %169

169:                                              ; preds = %tag_id_modify1.i.i.i.i
  %170 = and i64 %168, -8
  %171 = add i64 %170, -8
  %172 = inttoptr i64 %171 to ptr
  %173 = load i64, ptr %172, align 8
  %.not.i.i.i17.i.i.i.i = icmp eq i64 %173, 0
  br i1 %.not.i.i.i17.i.i.i.i, label %"#Attr_#inc_5.exit.i.i.i", label %modify_rc_union_merge.sink.split.i.i.i.i

"#Attr_#inc_5.exit.i.i.i":                        ; preds = %169, %tag_id_modify1.i.i.i.i, %161, %tag_id_modify.i.i.i.i, %modify_rc_union_merge.sink.split.i.i.i.i, %then_block.i.i.i88
  call void @llvm.lifetime.start.p0(i64 7, ptr nonnull %result_value.i.sroa.7.i.i.i)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(7) %result_value.i.sroa.7.i.i.i, ptr noundef nonnull align 1 dereferenceable(7) %result_value.sroa.7.i.i.i, i64 7, i1 false)
  %"#arg1.fca.0.extract.i.i.i.i.i93" = extractvalue %list.RocList %joinpointarg115.i.i.i, 0
  %"#arg1.fca.1.extract.i.i.i.i.i94" = extractvalue %list.RocList %joinpointarg115.i.i.i, 1
  %.not.i.i8.i.i.i95 = icmp eq ptr %"#arg1.fca.0.extract.i.i.i.i.i93", null
  br i1 %.not.i.i8.i.i.i95, label %List_296_7f8c4a473141d41efa7657e3f378539f18179e0b2dff0f626f6dce25d295f.exit.i.i.i, label %174

174:                                              ; preds = %"#Attr_#inc_5.exit.i.i.i"
  %175 = shl nuw i64 %"#arg1.fca.1.extract.i.i.i.i.i94", 5
  %176 = getelementptr inbounds i8, ptr %"#arg1.fca.0.extract.i.i.i.i.i93", i64 %175
  store ptr %result_value.sroa.0.0.copyload.i.i.i90, ptr %176, align 8
  %result_value.i.sroa.4.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %176, i64 8
  store i64 %result_value.sroa.4.0.copyload.i.i.i92, ptr %result_value.i.sroa.4.0..sroa_idx.i.i.i, align 8
  %result_value.i.sroa.5.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %176, i64 16
  store i64 %result_value.sroa.411.0.copyload.i.i.i, ptr %result_value.i.sroa.5.0..sroa_idx.i.i.i, align 8
  %result_value.i.sroa.6.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %176, i64 24
  store i8 %result_value.sroa.6.0.copyload.i.i.i, ptr %result_value.i.sroa.6.0..sroa_idx.i.i.i, align 8
  %result_value.i.sroa.7.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %176, i64 25
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(7) %result_value.i.sroa.7.0..sroa_idx.i.i.i, ptr noundef nonnull align 1 dereferenceable(7) %result_value.i.sroa.7.i.i.i, i64 7, i1 false)
  br label %List_296_7f8c4a473141d41efa7657e3f378539f18179e0b2dff0f626f6dce25d295f.exit.i.i.i

List_296_7f8c4a473141d41efa7657e3f378539f18179e0b2dff0f626f6dce25d295f.exit.i.i.i: ; preds = %174, %"#Attr_#inc_5.exit.i.i.i"
  %"#arg1.fca.2.extract.i.i.i.i.i96" = extractvalue %list.RocList %joinpointarg115.i.i.i, 2
  %177 = add nuw i64 %"#arg1.fca.1.extract.i.i.i.i.i94", 1
  %load_list.fca.1.insert.i.i.i.i.i97 = insertvalue %list.RocList %joinpointarg115.i.i.i, i64 %177, 1
  %load_list.fca.2.insert.i.i.i.i.i98 = insertvalue %list.RocList %load_list.fca.1.insert.i.i.i.i.i97, i64 %"#arg1.fca.2.extract.i.i.i.i.i96", 2
  call void @llvm.lifetime.end.p0(i64 7, ptr nonnull %result_value.i.sroa.7.i.i.i)
  %add_int_wrap.i.i.i.i99 = add nuw i64 %joinpointarg316.i.i.i, 1
  %exitcond.not.i.i.i100 = icmp eq i64 %add_int_wrap.i.i.i.i99, %list_len.i.i68
  br i1 %exitcond.not.i.i.i100, label %List_map_b7c59c3aec44645db91b229c81990d288c86aeb49f116d1eae85e2b9a39999f.exit, label %then_block.i.i.i88

List_map_b7c59c3aec44645db91b229c81990d288c86aeb49f116d1eae85e2b9a39999f.exit: ; preds = %List_296_7f8c4a473141d41efa7657e3f378539f18179e0b2dff0f626f6dce25d295f.exit.i.i.i, %"#Attr_#inc_4.exit.i.i.i"
  %joinpointarg1.lcssa.i.i.i101 = phi %list.RocList [ %load_list.fca.2.insert.i.i77, %"#Attr_#inc_4.exit.i.i.i" ], [ %load_list.fca.2.insert.i.i.i.i.i98, %List_296_7f8c4a473141d41efa7657e3f378539f18179e0b2dff0f626f6dce25d295f.exit.i.i.i ]
  tail call fastcc void @"#Attr_#dec_4"(%list.RocList %joinpointarg1.lcssa.i.i.i)
  call void @llvm.lifetime.end.p0(i64 7, ptr nonnull %result_value.sroa.7.i.i.i)
  tail call fastcc void @"#Attr_#dec_4"(%list.RocList %joinpointarg1.lcssa.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value27.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value26.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value25.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value24.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value23.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value22.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value21.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value20.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value19.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value18.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store17.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value16.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store15.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value14.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store13.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store12.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value11.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store10.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value9.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store7.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %joinpoint_arg_alloca.i.i)
  %list_len.i.i.i.i = extractvalue %list.RocList %joinpointarg1.lcssa.i.i.i101, 1
  %lt_uint.i.i.i.i = icmp ugt i64 %list_len.i.i.i.i, 1
  br i1 %lt_uint.i.i.i.i, label %then_block.i.i.i108, label %joinpointcont.thread.i.i

then_block.i.i.i108:                              ; preds = %List_map_b7c59c3aec44645db91b229c81990d288c86aeb49f116d1eae85e2b9a39999f.exit
  %read_list_ptr.i.i.i.i = extractvalue %list.RocList %joinpointarg1.lcssa.i.i.i101, 0
  %list_get_element.i.i.i.i109 = getelementptr inbounds { [3 x i64], i8 }, ptr %read_list_ptr.i.i.i.i, i64 1
  %result_value.sroa.0.0.copyload.i.i.i110 = load i64, ptr %list_get_element.i.i.i.i109, align 8
  %result_value.sroa.4.0.list_get_element.i.sroa_idx.i.i.i111 = getelementptr inbounds { [3 x i64], i8 }, ptr %read_list_ptr.i.i.i.i, i64 1, i32 0, i64 1
  %result_value.sroa.4.0.copyload.i.i.i112 = load i64, ptr %result_value.sroa.4.0.list_get_element.i.sroa_idx.i.i.i111, align 8
  %result_value.sroa.5.0.list_get_element.i.sroa_idx.i.i.i113 = getelementptr inbounds { [3 x i64], i8 }, ptr %read_list_ptr.i.i.i.i, i64 1, i32 0, i64 2
  %result_value.sroa.5.0.copyload.i.i.i114 = load i64, ptr %result_value.sroa.5.0.list_get_element.i.sroa_idx.i.i.i113, align 8
  %result_value.sroa.8.0.list_get_element.i.sroa_idx.i.i.i115 = getelementptr inbounds { [3 x i64], i8 }, ptr %read_list_ptr.i.i.i.i, i64 1, i32 1
  %result_value.sroa.8.0.copyload.i.i.i116 = load i8, ptr %result_value.sroa.8.0.list_get_element.i.sroa_idx.i.i.i115, align 8
  switch i8 %result_value.sroa.8.0.copyload.i.i.i116, label %then_block.i14.thread.i.i [
    i8 0, label %tag_id_modify.i.i.i.i121
    i8 1, label %tag_id_modify1.i.i.i.i117
  ]

tag_id_modify.i.i.i.i121:                         ; preds = %then_block.i.i.i108
  %178 = shl i64 %result_value.sroa.5.0.copyload.i.i.i114, 1
  %isneg.i.i.i.i.i122 = icmp slt i64 %result_value.sroa.5.0.copyload.i.i.i114, 0
  %179 = select i1 %isneg.i.i.i.i.i122, i64 %178, i64 %result_value.sroa.0.0.copyload.i.i.i110
  %.not.i.i.i.i.i.i123 = icmp eq i64 %179, 0
  br i1 %.not.i.i.i.i.i.i123, label %then_block.i14.thread73.i.i, label %180

180:                                              ; preds = %tag_id_modify.i.i.i.i121
  %181 = and i64 %179, -8
  %182 = add i64 %181, -8
  %183 = inttoptr i64 %182 to ptr
  %184 = load i64, ptr %183, align 8
  %.not.i.i.i.i.i.i.i124 = icmp eq i64 %184, 0
  br i1 %.not.i.i.i.i.i.i.i124, label %then_block.i14.thread73.i.i, label %then_block.i14.thread78.i.i

then_block.i14.thread78.i.i:                      ; preds = %180
  %185 = atomicrmw add ptr %183, i64 1 monotonic, align 8
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %load_element.i.i.i)
  store i64 %result_value.sroa.0.0.copyload.i.i.i110, ptr %load_element.i.i.i, align 8
  %result_value.sroa.2.0.load_element.i.sroa_idx80.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 8
  store i64 %result_value.sroa.4.0.copyload.i.i.i112, ptr %result_value.sroa.2.0.load_element.i.sroa_idx80.i.i, align 8
  %result_value.sroa.3.0.load_element.i.sroa_idx81.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 16
  store i64 %result_value.sroa.5.0.copyload.i.i.i114, ptr %result_value.sroa.3.0.load_element.i.sroa_idx81.i.i, align 8
  %result_value.sroa.4.0.load_element.i.sroa_idx82.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 24
  store i8 0, ptr %result_value.sroa.4.0.load_element.i.sroa_idx82.i.i, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store11.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %result_value.i.i.i.i)
  br label %then_block.i.i.i.i

tag_id_modify1.i.i.i.i117:                        ; preds = %then_block.i.i.i108
  %186 = shl i64 %result_value.sroa.5.0.copyload.i.i.i114, 1
  %isneg.i15.i.i.i.i118 = icmp slt i64 %result_value.sroa.5.0.copyload.i.i.i114, 0
  %187 = select i1 %isneg.i15.i.i.i.i118, i64 %186, i64 %result_value.sroa.0.0.copyload.i.i.i110
  %.not.i.i16.i.i.i.i119 = icmp eq i64 %187, 0
  br i1 %.not.i.i16.i.i.i.i119, label %then_block.i14.thread.i.i, label %188

188:                                              ; preds = %tag_id_modify1.i.i.i.i117
  %189 = and i64 %187, -8
  %190 = add i64 %189, -8
  %191 = inttoptr i64 %190 to ptr
  %192 = load i64, ptr %191, align 8
  %.not.i.i.i17.i.i.i.i120 = icmp eq i64 %192, 0
  br i1 %.not.i.i.i17.i.i.i.i120, label %then_block.i14.thread.i.i, label %then_block.i14.i.i

then_block.i14.thread.i.i:                        ; preds = %188, %tag_id_modify1.i.i.i.i117, %then_block.i.i.i108
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %load_element.i.i.i)
  store i64 %result_value.sroa.0.0.copyload.i.i.i110, ptr %load_element.i.i.i, align 8
  %result_value.sroa.2.0.load_element.i.sroa_idx68.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 8
  store i64 %result_value.sroa.4.0.copyload.i.i.i112, ptr %result_value.sroa.2.0.load_element.i.sroa_idx68.i.i, align 8
  %result_value.sroa.3.0.load_element.i.sroa_idx69.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 16
  store i64 %result_value.sroa.5.0.copyload.i.i.i114, ptr %result_value.sroa.3.0.load_element.i.sroa_idx69.i.i, align 8
  %result_value.sroa.4.0.load_element.i.sroa_idx70.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 24
  store i8 %result_value.sroa.8.0.copyload.i.i.i116, ptr %result_value.sroa.4.0.load_element.i.sroa_idx70.i.i, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store11.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %result_value.i.i.i.i)
  br label %else_block.i.i.i.i

then_block.i14.thread73.i.i:                      ; preds = %180, %tag_id_modify.i.i.i.i121
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %load_element.i.i.i)
  store i64 %result_value.sroa.0.0.copyload.i.i.i110, ptr %load_element.i.i.i, align 8
  %result_value.sroa.2.0.load_element.i.sroa_idx74.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 8
  store i64 %result_value.sroa.4.0.copyload.i.i.i112, ptr %result_value.sroa.2.0.load_element.i.sroa_idx74.i.i, align 8
  %result_value.sroa.3.0.load_element.i.sroa_idx75.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 16
  store i64 %result_value.sroa.5.0.copyload.i.i.i114, ptr %result_value.sroa.3.0.load_element.i.sroa_idx75.i.i, align 8
  %result_value.sroa.4.0.load_element.i.sroa_idx76.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 24
  store i8 0, ptr %result_value.sroa.4.0.load_element.i.sroa_idx76.i.i, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store11.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %result_value.i.i.i.i)
  br label %then_block.i.i.i.i

then_block.i14.i.i:                               ; preds = %188
  %193 = atomicrmw add ptr %191, i64 1 monotonic, align 8
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %load_element.i.i.i)
  store i64 %result_value.sroa.0.0.copyload.i.i.i110, ptr %load_element.i.i.i, align 8
  %result_value.sroa.2.0.load_element.i.sroa_idx.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 8
  store i64 %result_value.sroa.4.0.copyload.i.i.i112, ptr %result_value.sroa.2.0.load_element.i.sroa_idx.i.i, align 8
  %result_value.sroa.3.0.load_element.i.sroa_idx.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 16
  store i64 %result_value.sroa.5.0.copyload.i.i.i114, ptr %result_value.sroa.3.0.load_element.i.sroa_idx.i.i, align 8
  %result_value.sroa.4.0.load_element.i.sroa_idx.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i, i64 24
  store i8 1, ptr %result_value.sroa.4.0.load_element.i.sroa_idx.i.i, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store11.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 32, ptr nonnull %result_value.i.i.i.i)
  br label %else_block.i.i.i.i

then_block.i.i.i.i:                               ; preds = %then_block.i14.thread73.i.i, %then_block.i14.thread78.i.i
  %194 = inttoptr i64 %result_value.sroa.0.0.copyload.i.i.i110 to ptr
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %.sroa.312.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %.sroa.3.i.i.i.i.i.i.i.i)
  %195 = icmp eq i64 %result_value.sroa.4.0.copyload.i.i.i112, 0
  br i1 %195, label %196, label %207

196:                                              ; preds = %then_block.i.i.i.i
  %197 = icmp ne i64 %result_value.sroa.5.0.copyload.i.i.i114, 0
  %198 = icmp ne i64 %179, 0
  %or.cond.i.i.i.i.i.i.i.i.i = and i1 %197, %198
  br i1 %or.cond.i.i.i.i.i.i.i.i.i, label %199, label %Str_from_utf8_lowlevel_653de62d66328bd02e166b818cc28fc7f76ad909358c5f3f340dd1c827546c9.exit.i.i.i.i.i

199:                                              ; preds = %196
  %200 = inttoptr i64 %179 to ptr
  %201 = getelementptr inbounds i64, ptr %200, i64 -1
  %202 = load i64, ptr %201, align 8, !noalias !97
  %.not.i.i.i.i.i.i.i.i.i = icmp eq i64 %202, 0
  br i1 %.not.i.i.i.i.i.i.i.i.i, label %Str_from_utf8_lowlevel_653de62d66328bd02e166b818cc28fc7f76ad909358c5f3f340dd1c827546c9.exit.i.i.i.i.i, label %203

203:                                              ; preds = %199
  %204 = atomicrmw sub ptr %201, i64 1 monotonic, align 8, !noalias !97
  %205 = icmp eq i64 %204, 1
  br i1 %205, label %206, label %Str_from_utf8_lowlevel_653de62d66328bd02e166b818cc28fc7f76ad909358c5f3f340dd1c827546c9.exit.i.i.i.i.i

206:                                              ; preds = %203
  tail call void @roc_dealloc(ptr nonnull align 1 %201, i32 8), !noalias !97
  br label %Str_from_utf8_lowlevel_653de62d66328bd02e166b818cc28fc7f76ad909358c5f3f340dd1c827546c9.exit.i.i.i.i.i

207:                                              ; preds = %then_block.i.i.i.i
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %10), !noalias !97
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %11), !noalias !97
  %208 = icmp ugt i64 %result_value.sroa.4.0.copyload.i.i.i112, 8
  br i1 %208, label %.lr.ph70.i.i.i.i.i.i.i.i.i, label %._crit_edge.i.i.i.i.i.i.i.i.i

.lr.ph70.i.i.i.i.i.i.i.i.i:                       ; preds = %207
  %209 = getelementptr inbounds i8, ptr %10, i64 4
  br label %210

210:                                              ; preds = %.loopexit.i.i.i.i.i.i.i.i.i, %.lr.ph70.i.i.i.i.i.i.i.i.i
  %211 = phi i64 [ 8, %.lr.ph70.i.i.i.i.i.i.i.i.i ], [ %216, %.loopexit.i.i.i.i.i.i.i.i.i ]
  %.sroa.0.069.i.i.i.i.i.i.i.i.i = phi i64 [ 0, %.lr.ph70.i.i.i.i.i.i.i.i.i ], [ %.sroa.0.1.i.i.i.i.i.i.i.i.i, %.loopexit.i.i.i.i.i.i.i.i.i ]
  %212 = getelementptr inbounds i8, ptr %194, i64 %.sroa.0.069.i.i.i.i.i.i.i.i.i
  %.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i = load i64, ptr %212, align 1, !noalias !97
  %213 = and i64 %.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i, -9187201950435737472
  %214 = icmp eq i64 %213, 0
  br i1 %214, label %.loopexit.i.i.i.i.i.i.i.i.i, label %.preheader61.i.i.i.i.i.i.i.i.i

._crit_edge.i.i.i.i.i.i.i.i.i:                    ; preds = %.loopexit.i.i.i.i.i.i.i.i.i, %207
  %.sroa.0.0.lcssa.i.i.i.i.i.i.i.i.i = phi i64 [ 0, %207 ], [ %.sroa.0.1.i.i.i.i.i.i.i.i.i, %.loopexit.i.i.i.i.i.i.i.i.i ]
  %215 = icmp eq i64 %.sroa.0.0.lcssa.i.i.i.i.i.i.i.i.i, %result_value.sroa.4.0.copyload.i.i.i112
  br i1 %215, label %.loopexit54.i.i.i.i.i.i.i.i, label %.preheader.i.i.i.i.i.i.i.i.i

.loopexit.i.i.i.i.i.i.i.i.i:                      ; preds = %224, %.preheader59.i.i.i.i.i.i.i.i.i, %210
  %.sroa.0.1.i.i.i.i.i.i.i.i.i = phi i64 [ %211, %210 ], [ %.sroa.0.2.i.i.i.i.i.i.i.i.i, %.preheader59.i.i.i.i.i.i.i.i.i ], [ %225, %224 ]
  %216 = add nuw i64 %.sroa.0.1.i.i.i.i.i.i.i.i.i, 8
  %217 = icmp ult i64 %216, %result_value.sroa.4.0.copyload.i.i.i112
  br i1 %217, label %210, label %._crit_edge.i.i.i.i.i.i.i.i.i

.preheader61.i.i.i.i.i.i.i.i.i:                   ; preds = %210, %.preheader61.i.i.i.i.i.i.i.i.i
  %.sroa.0.2.i.i.i.i.i.i.i.i.i = phi i64 [ %221, %.preheader61.i.i.i.i.i.i.i.i.i ], [ %.sroa.0.069.i.i.i.i.i.i.i.i.i, %210 ]
  %218 = getelementptr inbounds i8, ptr %194, i64 %.sroa.0.2.i.i.i.i.i.i.i.i.i
  %219 = load i8, ptr %218, align 1, !noalias !97
  %220 = icmp sgt i8 %219, -1
  %221 = add nuw i64 %.sroa.0.2.i.i.i.i.i.i.i.i.i, 1
  br i1 %220, label %.preheader61.i.i.i.i.i.i.i.i.i, label %.preheader59.i.i.i.i.i.i.i.i.i

.preheader59.i.i.i.i.i.i.i.i.i:                   ; preds = %.preheader61.i.i.i.i.i.i.i.i.i
  %222 = add nuw i64 %.sroa.0.2.i.i.i.i.i.i.i.i.i, 4
  %.not66.i.i.i.i.i.i.i.i.i = icmp ult i64 %222, %result_value.sroa.4.0.copyload.i.i.i112
  br i1 %.not66.i.i.i.i.i.i.i.i.i, label %.lr.ph.i.i.i.i.i.i.i.i.i.preheader, label %.loopexit.i.i.i.i.i.i.i.i.i

.lr.ph.i.i.i.i.i.i.i.i.i.preheader:               ; preds = %.preheader59.i.i.i.i.i.i.i.i.i
  %223 = getelementptr inbounds i8, ptr %194, i64 %.sroa.0.2.i.i.i.i.i.i.i.i.i
  br label %.lr.ph.i.i.i.i.i.i.i.i.i

224:                                              ; preds = %246
  %225 = add nuw i64 %.sroa.0.368.i.i.i.i.i.i.i.i.i, %249
  %226 = getelementptr inbounds i8, ptr %194, i64 %225
  %227 = load i8, ptr %226, align 1, !noalias !97
  %228 = icmp slt i8 %227, 0
  %229 = add nuw i64 %225, 4
  %.not.i18.i.i.i.i.i.i.i.i = icmp ult i64 %229, %result_value.sroa.4.0.copyload.i.i.i112
  %or.cond.i19.i.i.i.i.i.i.i.i = select i1 %228, i1 %.not.i18.i.i.i.i.i.i.i.i, i1 false
  br i1 %or.cond.i19.i.i.i.i.i.i.i.i, label %.lr.ph.i.i.i.i.i.i.i.i.i, label %.loopexit.i.i.i.i.i.i.i.i.i

.lr.ph.i.i.i.i.i.i.i.i.i:                         ; preds = %.lr.ph.i.i.i.i.i.i.i.i.i.preheader, %224
  %230 = phi ptr [ %226, %224 ], [ %223, %.lr.ph.i.i.i.i.i.i.i.i.i.preheader ]
  %.sroa.0.368.i.i.i.i.i.i.i.i.i = phi i64 [ %225, %224 ], [ %.sroa.0.2.i.i.i.i.i.i.i.i.i, %.lr.ph.i.i.i.i.i.i.i.i.i.preheader ]
  %231 = load i32, ptr %230, align 1, !noalias !97
  store i32 %231, ptr %11, align 4, !noalias !97
  %232 = trunc i32 %231 to i8
  %233 = icmp sgt i8 %232, -1
  br i1 %233, label %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i, label %234

234:                                              ; preds = %.lr.ph.i.i.i.i.i.i.i.i.i
  %235 = and i8 %232, -32
  %236 = icmp eq i8 %235, -64
  br i1 %236, label %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i, label %237

237:                                              ; preds = %234
  %238 = and i8 %232, -16
  %239 = icmp eq i8 %238, -32
  br i1 %239, label %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i, label %240

240:                                              ; preds = %237
  %241 = and i8 %232, -8
  %242 = icmp eq i8 %241, -16
  %..i.i.i.i.i.i.i.i.i.i = select i1 %242, ptr @5, ptr @0
  br label %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i

unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i: ; preds = %240, %237, %234, %.lr.ph.i.i.i.i.i.i.i.i.i
  %243 = phi ptr [ @2, %.lr.ph.i.i.i.i.i.i.i.i.i ], [ @3, %234 ], [ @4, %237 ], [ %..i.i.i.i.i.i.i.i.i.i, %240 ]
  %244 = load i16, ptr %243, align 2, !noalias !97
  %245 = icmp eq i16 %244, 0
  br i1 %245, label %246, label %.loopexit.i.i.i.i.i.i.i.i

246:                                              ; preds = %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i
  %.sroa_idx.i.i.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %243, i64 2
  %247 = load i8, ptr %.sroa_idx.i.i.i.i.i.i.i.i.i, align 2, !noalias !97
  %248 = and i8 %247, 7
  %249 = zext nneg i8 %248 to i64
  call fastcc void @unicode.utf8Decode(ptr noalias %10, ptr nonnull readonly align 1 %11, i64 %249), !noalias !97
  %.val.i.i.i.i.i.i.i.i.i = load i16, ptr %209, align 4, !noalias !97
  %.not58.i.i.i.i.i.i.i.i.i = icmp eq i16 %.val.i.i.i.i.i.i.i.i.i, 0
  br i1 %.not58.i.i.i.i.i.i.i.i.i, label %224, label %.loopexit.i.i.i.i.i.i.i.i

.preheader.i.i.i.i.i.i.i.i.i:                     ; preds = %._crit_edge.i.i.i.i.i.i.i.i.i, %253
  %.sroa.0.4.i.i.i.i.i.i.i.i.i = phi i64 [ %254, %253 ], [ %.sroa.0.0.lcssa.i.i.i.i.i.i.i.i.i, %._crit_edge.i.i.i.i.i.i.i.i.i ]
  %250 = getelementptr inbounds i8, ptr %194, i64 %.sroa.0.4.i.i.i.i.i.i.i.i.i
  %251 = load i8, ptr %250, align 1, !noalias !97
  %252 = icmp sgt i8 %251, -1
  br i1 %252, label %253, label %256

253:                                              ; preds = %.preheader.i.i.i.i.i.i.i.i.i
  %254 = add nuw i64 %.sroa.0.4.i.i.i.i.i.i.i.i.i, 1
  %255 = icmp eq i64 %254, %result_value.sroa.4.0.copyload.i.i.i112
  br i1 %255, label %.loopexit54.i.i.i.i.i.i.i.i, label %.preheader.i.i.i.i.i.i.i.i.i

256:                                              ; preds = %.preheader.i.i.i.i.i.i.i.i.i
  %257 = getelementptr inbounds i8, ptr %194, i64 %.sroa.0.4.i.i.i.i.i.i.i.i.i
  %258 = sub nuw i64 %result_value.sroa.4.0.copyload.i.i.i112, %.sroa.0.4.i.i.i.i.i.i.i.i.i
  %259 = icmp ugt i64 %258, 15
  br i1 %259, label %.lr.ph.i24.i.i.i.i.i.i.i.i, label %._crit_edge.i21.i.i.i.i.i.i.i.i

._crit_edge.i21.i.i.i.i.i.i.i.i:                  ; preds = %264, %256
  %.sroa.11.0.lcssa.i.i.i.i.i.i.i.i.i = phi i64 [ %258, %256 ], [ %266, %264 ]
  %.sroa.0.0.lcssa.i22.i.i.i.i.i.i.i.i = phi ptr [ %257, %256 ], [ %265, %264 ]
  %260 = icmp eq i64 %.sroa.11.0.lcssa.i.i.i.i.i.i.i.i.i, 0
  br i1 %260, label %.loopexit54.i.i.i.i.i.i.i.i, label %.lr.ph87.preheader.i.i.i.i.i.i.i.i.i

.lr.ph87.preheader.i.i.i.i.i.i.i.i.i:             ; preds = %.lr.ph.i24.i.i.i.i.i.i.i.i, %._crit_edge.i21.i.i.i.i.i.i.i.i
  %.sroa.0.0.lcssa105.i.i.i.i.i.i.i.i.i = phi ptr [ %.sroa.0.0.lcssa.i22.i.i.i.i.i.i.i.i, %._crit_edge.i21.i.i.i.i.i.i.i.i ], [ %.sroa.0.080.i.i.i.i.i.i.i.i.i, %.lr.ph.i24.i.i.i.i.i.i.i.i ]
  %.sroa.11.0.lcssa104.i.i.i.i.i.i.i.i.i = phi i64 [ %.sroa.11.0.lcssa.i.i.i.i.i.i.i.i.i, %._crit_edge.i21.i.i.i.i.i.i.i.i ], [ %.sroa.11.079.i.i.i.i.i.i.i.i.i, %.lr.ph.i24.i.i.i.i.i.i.i.i ]
  br label %.lr.ph87.i.i.i.i.i.i.i.i.i

.lr.ph.i24.i.i.i.i.i.i.i.i:                       ; preds = %256, %264
  %.sroa.0.080.i.i.i.i.i.i.i.i.i = phi ptr [ %265, %264 ], [ %257, %256 ]
  %.sroa.11.079.i.i.i.i.i.i.i.i.i = phi i64 [ %266, %264 ], [ %258, %256 ]
  %261 = load <16 x i8>, ptr %.sroa.0.080.i.i.i.i.i.i.i.i.i, align 1, !noalias !97
  %262 = icmp slt <16 x i8> %261, zeroinitializer
  %263 = bitcast <16 x i1> %262 to i16
  %.not.i25.i.i.i.i.i.i.i.i = icmp eq i16 %263, 0
  br i1 %.not.i25.i.i.i.i.i.i.i.i, label %264, label %.lr.ph87.preheader.i.i.i.i.i.i.i.i.i

264:                                              ; preds = %.lr.ph.i24.i.i.i.i.i.i.i.i
  %265 = getelementptr inbounds i8, ptr %.sroa.0.080.i.i.i.i.i.i.i.i.i, i64 16
  %266 = add i64 %.sroa.11.079.i.i.i.i.i.i.i.i.i, -16
  %267 = icmp ugt i64 %266, 15
  br i1 %267, label %.lr.ph.i24.i.i.i.i.i.i.i.i, label %._crit_edge.i21.i.i.i.i.i.i.i.i

.lr.ph87.i.i.i.i.i.i.i.i.i:                       ; preds = %271, %.lr.ph87.preheader.i.i.i.i.i.i.i.i.i
  %.sroa.063.085.i.i.i.i.i.i.i.i.i = phi i64 [ %272, %271 ], [ 0, %.lr.ph87.preheader.i.i.i.i.i.i.i.i.i ]
  %268 = getelementptr inbounds i8, ptr %.sroa.0.0.lcssa105.i.i.i.i.i.i.i.i.i, i64 %.sroa.063.085.i.i.i.i.i.i.i.i.i
  %269 = load i8, ptr %268, align 1, !noalias !97
  %270 = icmp sgt i8 %269, -1
  br i1 %270, label %271, label %273

271:                                              ; preds = %304, %294, %292, %.lr.ph87.i.i.i.i.i.i.i.i.i
  %.sink.i23.i.i.i.i.i.i.i.i = phi i64 [ 1, %.lr.ph87.i.i.i.i.i.i.i.i.i ], [ 2, %292 ], [ 3, %294 ], [ 4, %304 ]
  %272 = add nuw i64 %.sink.i23.i.i.i.i.i.i.i.i, %.sroa.063.085.i.i.i.i.i.i.i.i.i
  %.not101.i.i.i.i.i.i.i.i.i = icmp ult i64 %272, %.sroa.11.0.lcssa104.i.i.i.i.i.i.i.i.i
  br i1 %.not101.i.i.i.i.i.i.i.i.i, label %.lr.ph87.i.i.i.i.i.i.i.i.i, label %.loopexit54.i.i.i.i.i.i.i.i

273:                                              ; preds = %.lr.ph87.i.i.i.i.i.i.i.i.i
  %274 = zext i8 %269 to i64
  %275 = getelementptr inbounds [256 x i8], ptr @10, i64 0, i64 %274
  %276 = load i8, ptr %275, align 1, !noalias !97
  %277 = icmp eq i8 %276, -15
  br i1 %277, label %.loopexit.i.i.i.i.i.i.i.i, label %278

278:                                              ; preds = %273
  %279 = and i8 %276, 7
  %280 = zext nneg i8 %279 to i64
  %281 = add nuw i64 %.sroa.063.085.i.i.i.i.i.i.i.i.i, %280
  %282 = icmp ugt i64 %281, %.sroa.11.0.lcssa104.i.i.i.i.i.i.i.i.i
  br i1 %282, label %.loopexit.i.i.i.i.i.i.i.i, label %switch.lookup

switch.lookup:                                    ; preds = %278
  %283 = lshr i8 %276, 4
  %284 = shl nuw nsw i8 %283, 3
  %switch.shiftamt = zext nneg i8 %284 to i40
  %switch.downshift = lshr i40 -482116386881, %switch.shiftamt
  %switch.masked = trunc i40 %switch.downshift to i8
  %285 = shl nuw nsw i8 %283, 3
  %switch.shiftamt603 = zext nneg i8 %285 to i40
  %switch.downshift604 = lshr i40 -547331465088, %switch.shiftamt603
  %switch.masked605 = trunc i40 %switch.downshift604 to i8
  %286 = add nuw i64 %.sroa.063.085.i.i.i.i.i.i.i.i.i, 1
  %287 = getelementptr inbounds i8, ptr %.sroa.0.0.lcssa105.i.i.i.i.i.i.i.i.i, i64 %286
  %288 = load i8, ptr %287, align 1, !noalias !97
  %289 = icmp ult i8 %288, %switch.masked605
  %290 = icmp ugt i8 %288, %switch.masked
  %291 = or i1 %289, %290
  br i1 %291, label %.loopexit.i.i.i.i.i.i.i.i, label %292

292:                                              ; preds = %switch.lookup
  switch i8 %279, label %293 [
    i8 2, label %271
    i8 3, label %294
    i8 4, label %299
  ]

293:                                              ; preds = %292
  unreachable

294:                                              ; preds = %292
  %295 = add nuw i64 %.sroa.063.085.i.i.i.i.i.i.i.i.i, 2
  %296 = getelementptr inbounds i8, ptr %.sroa.0.0.lcssa105.i.i.i.i.i.i.i.i.i, i64 %295
  %297 = load i8, ptr %296, align 1, !noalias !97
  %298 = icmp sgt i8 %297, -65
  br i1 %298, label %.loopexit.i.i.i.i.i.i.i.i, label %271

299:                                              ; preds = %292
  %300 = add nuw i64 %.sroa.063.085.i.i.i.i.i.i.i.i.i, 2
  %301 = getelementptr inbounds i8, ptr %.sroa.0.0.lcssa105.i.i.i.i.i.i.i.i.i, i64 %300
  %302 = load i8, ptr %301, align 1, !noalias !97
  %303 = icmp sgt i8 %302, -65
  br i1 %303, label %.loopexit.i.i.i.i.i.i.i.i, label %304

304:                                              ; preds = %299
  %305 = add nuw i64 %.sroa.063.085.i.i.i.i.i.i.i.i.i, 3
  %306 = getelementptr inbounds i8, ptr %.sroa.0.0.lcssa105.i.i.i.i.i.i.i.i.i, i64 %305
  %307 = load i8, ptr %306, align 1, !noalias !97
  %308 = icmp sgt i8 %307, -65
  br i1 %308, label %.loopexit.i.i.i.i.i.i.i.i, label %271

.loopexit54.i.i.i.i.i.i.i.i:                      ; preds = %253, %271, %._crit_edge.i21.i.i.i.i.i.i.i.i, %._crit_edge.i.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %10), !noalias !97
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %11), !noalias !97
  br i1 %isneg.i.i.i.i.i122, label %309, label %312

309:                                              ; preds = %.loopexit54.i.i.i.i.i.i.i.i
  %310 = or i64 %result_value.sroa.4.0.copyload.i.i.i112, -9223372036854775808
  %311 = and i64 %result_value.sroa.5.0.copyload.i.i.i114, 9223372036854775807
  br label %str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i

312:                                              ; preds = %.loopexit54.i.i.i.i.i.i.i.i
  %313 = icmp eq i64 %result_value.sroa.5.0.copyload.i.i.i114, 0
  br i1 %313, label %str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i, label %list.RocList.refcount.exit.i.i.i.i.i.i.i.i.i

list.RocList.refcount.exit.i.i.i.i.i.i.i.i.i:     ; preds = %312
  %314 = getelementptr inbounds i64, ptr %194, i64 -1
  %315 = load i64, ptr %314, align 8, !noalias !102
  %316 = icmp eq i64 %315, 1
  br i1 %316, label %str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i, label %.critedge.i.i.i.i.i.i.i.i.i

.critedge.i.i.i.i.i.i.i.i.i:                      ; preds = %list.RocList.refcount.exit.i.i.i.i.i.i.i.i.i
  %317 = or i64 %result_value.sroa.4.0.copyload.i.i.i112, -9223372036854775808
  %318 = lshr i64 %result_value.sroa.0.0.copyload.i.i.i110, 1
  br label %str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i

str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i: ; preds = %.critedge.i.i.i.i.i.i.i.i.i, %list.RocList.refcount.exit.i.i.i.i.i.i.i.i.i, %312, %309
  %.sink42.i.i.i.i.i.i.i.i.i = phi i64 [ %317, %.critedge.i.i.i.i.i.i.i.i.i ], [ %310, %309 ], [ %result_value.sroa.4.0.copyload.i.i.i112, %312 ], [ %result_value.sroa.4.0.copyload.i.i.i112, %list.RocList.refcount.exit.i.i.i.i.i.i.i.i.i ]
  %.sink.i.i.i.i.i.i.i.i.i = phi i64 [ %318, %.critedge.i.i.i.i.i.i.i.i.i ], [ %311, %309 ], [ 0, %312 ], [ %result_value.sroa.5.0.copyload.i.i.i114, %list.RocList.refcount.exit.i.i.i.i.i.i.i.i.i ]
  store i1 true, ptr %.sroa.3.i.i.i.i.i.i.i.i, align 8, !noalias !97
  br label %Str_from_utf8_lowlevel_653de62d66328bd02e166b818cc28fc7f76ad909358c5f3f340dd1c827546c9.exit.i.i.i.i.i

.loopexit.i.i.i.i.i.i.i.i:                        ; preds = %246, %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i, %304, %299, %294, %switch.lookup, %278, %273
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %10), !noalias !97
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %11), !noalias !97
  %319 = getelementptr inbounds { i21, i16, [2 x i8] }, ptr %9, i64 0, i32 1
  br label %320

320:                                              ; preds = %str.numberOfNextCodepointBytes.exit.thread.i.i.i.i.i.i.i.i.i, %.loopexit.i.i.i.i.i.i.i.i
  %.sroa.0.0.i.i.i.i.i.i.i.i.i = phi i64 [ 0, %.loopexit.i.i.i.i.i.i.i.i ], [ %340, %str.numberOfNextCodepointBytes.exit.thread.i.i.i.i.i.i.i.i.i ]
  %321 = icmp ult i64 %.sroa.0.0.i.i.i.i.i.i.i.i.i, %result_value.sroa.4.0.copyload.i.i.i112
  tail call void @llvm.assume(i1 %321)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %9), !noalias !105
  %322 = getelementptr inbounds i8, ptr %194, i64 %.sroa.0.0.i.i.i.i.i.i.i.i.i
  %323 = load i8, ptr %322, align 1, !noalias !108
  %324 = icmp sgt i8 %323, -1
  br i1 %324, label %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i.i, label %325

325:                                              ; preds = %320
  %326 = and i8 %323, -32
  %327 = icmp eq i8 %326, -64
  br i1 %327, label %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i.i, label %328

328:                                              ; preds = %325
  %329 = and i8 %323, -16
  %330 = icmp eq i8 %329, -32
  br i1 %330, label %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i.i, label %331

331:                                              ; preds = %328
  %332 = and i8 %323, -8
  %333 = icmp eq i8 %332, -16
  %..i.i.i.i.i.i.i.i.i.i.i = select i1 %333, ptr @5, ptr @0
  br label %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i.i

unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i.i: ; preds = %331, %328, %325, %320
  %334 = phi ptr [ @2, %320 ], [ @3, %325 ], [ @4, %328 ], [ %..i.i.i.i.i.i.i.i.i.i.i, %331 ]
  %335 = load i16, ptr %334, align 2, !noalias !108
  %.not.i.i.i.i.i.i.i.i.i.i = icmp eq i16 %335, 0
  br i1 %.not.i.i.i.i.i.i.i.i.i.i, label %336, label %str.numberOfNextCodepointBytes.exit.i.i.i.i.i.i.i.i.i

336:                                              ; preds = %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i.i
  %.sroa_idx.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %334, i64 2
  %337 = load i8, ptr %.sroa_idx.i.i.i.i.i.i.i.i.i.i, align 2, !noalias !108
  %338 = and i8 %337, 7
  %339 = zext nneg i8 %338 to i64
  %340 = add nuw i64 %.sroa.0.0.i.i.i.i.i.i.i.i.i, %339
  %341 = icmp ugt i64 %340, %result_value.sroa.4.0.copyload.i.i.i112
  br i1 %341, label %str.numberOfNextCodepointBytes.exit.thread13.i.i.i.i.i.i.i.i.i, label %342

str.numberOfNextCodepointBytes.exit.thread13.i.i.i.i.i.i.i.i.i: ; preds = %336
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %9), !noalias !105
  br label %str.errorToProblem.exit.i.i.i.i.i.i.i.i

342:                                              ; preds = %336
  call fastcc void @unicode.utf8Decode(ptr noalias %9, ptr nonnull readonly align 1 %322, i64 %339), !noalias !108
  %343 = load i16, ptr %319, align 4, !noalias !108
  %.not4.i.i.i.i.i.i.i.i.i.i = icmp eq i16 %343, 0
  br i1 %.not4.i.i.i.i.i.i.i.i.i.i, label %str.numberOfNextCodepointBytes.exit.thread.i.i.i.i.i.i.i.i.i, label %str.numberOfNextCodepointBytes.exit.i.i.i.i.i.i.i.i.i

str.numberOfNextCodepointBytes.exit.thread.i.i.i.i.i.i.i.i.i: ; preds = %342
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %9), !noalias !105
  br label %320

str.numberOfNextCodepointBytes.exit.i.i.i.i.i.i.i.i.i: ; preds = %342, %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i.i
  %.sroa.3.0.i.i.i.i.i.i.i.i.i = phi i16 [ %335, %unicode.utf8ByteSequenceLength.exit.i.i.i.i.i.i.i.i.i.i ], [ %343, %342 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %9), !noalias !105
  %switch.tableidx = add nsw i16 %.sroa.3.0.i.i.i.i.i.i.i.i.i, -22
  %switch.shifted = lshr i16 317, %switch.tableidx
  %344 = and i16 %switch.shifted, 1
  %switch.lobit = icmp ne i16 %344, 0
  tail call void @llvm.assume(i1 %switch.lobit)
  %345 = sext i16 %switch.tableidx to i64
  %switch.gep = getelementptr inbounds [9 x i8], ptr @"switch.table._main_for_host!_8445464738218c426e3b841f28ede2879d4391e6c33df6a616e872e199e47e", i64 0, i64 %345
  %switch.load = load i8, ptr %switch.gep, align 1
  br label %str.errorToProblem.exit.i.i.i.i.i.i.i.i

str.errorToProblem.exit.i.i.i.i.i.i.i.i:          ; preds = %str.numberOfNextCodepointBytes.exit.i.i.i.i.i.i.i.i.i, %str.numberOfNextCodepointBytes.exit.thread13.i.i.i.i.i.i.i.i.i
  %.sink.i20.i.i.i.i.i.i.i.i = phi i8 [ 5, %str.numberOfNextCodepointBytes.exit.thread13.i.i.i.i.i.i.i.i.i ], [ %switch.load, %str.numberOfNextCodepointBytes.exit.i.i.i.i.i.i.i.i.i ]
  %346 = icmp ne i64 %result_value.sroa.5.0.copyload.i.i.i114, 0
  %347 = icmp ne i64 %179, 0
  %or.cond.i15.i.i.i.i.i.i.i.i = and i1 %346, %347
  br i1 %or.cond.i15.i.i.i.i.i.i.i.i, label %348, label %utils.decref.exit17.i.i.i.i.i.i.i.i

348:                                              ; preds = %str.errorToProblem.exit.i.i.i.i.i.i.i.i
  %349 = inttoptr i64 %179 to ptr
  %350 = getelementptr inbounds i64, ptr %349, i64 -1
  %351 = load i64, ptr %350, align 8, !noalias !97
  %.not.i16.i.i.i.i.i.i.i.i = icmp eq i64 %351, 0
  br i1 %.not.i16.i.i.i.i.i.i.i.i, label %utils.decref.exit17.i.i.i.i.i.i.i.i, label %352

352:                                              ; preds = %348
  %353 = atomicrmw sub ptr %350, i64 1 monotonic, align 8, !noalias !97
  %354 = icmp eq i64 %353, 1
  br i1 %354, label %355, label %utils.decref.exit17.i.i.i.i.i.i.i.i

355:                                              ; preds = %352
  tail call void @roc_dealloc(ptr nonnull align 1 %350, i32 8), !noalias !97
  br label %utils.decref.exit17.i.i.i.i.i.i.i.i

utils.decref.exit17.i.i.i.i.i.i.i.i:              ; preds = %355, %352, %348, %str.errorToProblem.exit.i.i.i.i.i.i.i.i
  store i1 false, ptr %.sroa.312.i.i.i.i.i.i.i.i, align 8, !noalias !97
  br label %Str_from_utf8_lowlevel_653de62d66328bd02e166b818cc28fc7f76ad909358c5f3f340dd1c827546c9.exit.i.i.i.i.i

Str_from_utf8_lowlevel_653de62d66328bd02e166b818cc28fc7f76ad909358c5f3f340dd1c827546c9.exit.i.i.i.i.i: ; preds = %utils.decref.exit17.i.i.i.i.i.i.i.i, %str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i, %206, %203, %199, %196
  %.sroa.10.0.i.i.i.i.i.i.i = phi i8 [ 3, %str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i ], [ %.sink.i20.i.i.i.i.i.i.i.i, %utils.decref.exit17.i.i.i.i.i.i.i.i ], [ 3, %206 ], [ 3, %203 ], [ 3, %199 ], [ 3, %196 ]
  %.sroa.8.0.in.i.i.i.i.i.i.i = phi ptr [ %.sroa.3.i.i.i.i.i.i.i.i, %str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i ], [ %.sroa.312.i.i.i.i.i.i.i.i, %utils.decref.exit17.i.i.i.i.i.i.i.i ], [ getelementptr inbounds (%str.FromUtf8Result, ptr @1, i64 0, i32 2), %206 ], [ getelementptr inbounds (%str.FromUtf8Result, ptr @1, i64 0, i32 2), %203 ], [ getelementptr inbounds (%str.FromUtf8Result, ptr @1, i64 0, i32 2), %199 ], [ getelementptr inbounds (%str.FromUtf8Result, ptr @1, i64 0, i32 2), %196 ]
  %.sroa.7.0.i.i.i.i.i.i.i = phi i64 [ %.sink.i.i.i.i.i.i.i.i.i, %str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i ], [ -9223372036854775808, %utils.decref.exit17.i.i.i.i.i.i.i.i ], [ -9223372036854775808, %206 ], [ -9223372036854775808, %203 ], [ -9223372036854775808, %199 ], [ -9223372036854775808, %196 ]
  %.sroa.6.0.i.i.i.i.i.i.i = phi i64 [ %.sink42.i.i.i.i.i.i.i.i.i, %str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i ], [ 0, %utils.decref.exit17.i.i.i.i.i.i.i.i ], [ 0, %206 ], [ 0, %203 ], [ 0, %199 ], [ 0, %196 ]
  %.sroa.4.0.i.i.i.i.i.i.i = phi ptr [ %194, %str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i ], [ null, %utils.decref.exit17.i.i.i.i.i.i.i.i ], [ null, %206 ], [ null, %203 ], [ null, %199 ], [ null, %196 ]
  %.sroa.0.0.i.i.i.i.i.i.i = phi i64 [ 0, %str.RocStr.fromSubListUnsafe.exit.i.i.i.i.i.i.i.i ], [ %.sroa.0.0.i.i.i.i.i.i.i.i.i, %utils.decref.exit17.i.i.i.i.i.i.i.i ], [ 0, %206 ], [ 0, %203 ], [ 0, %199 ], [ 0, %196 ]
  %.sroa.8.0.i.i.i.i.i.i.i = load i8, ptr %.sroa.8.0.in.i.i.i.i.i.i.i, align 8, !noalias !111
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.312.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.3.i.i.i.i.i.i.i.i)
  %356 = and i8 %.sroa.8.0.i.i.i.i.i.i.i, 1
  %.not.i.i.i.i.i125 = icmp eq i8 %356, 0
  br i1 %.not.i.i.i.i.i125, label %else_block.i.i.i.i.i127, label %joinpointcont.i.i

else_block.i.i.i.i.i127:                          ; preds = %Str_from_utf8_lowlevel_653de62d66328bd02e166b818cc28fc7f76ad909358c5f3f340dd1c827546c9.exit.i.i.i.i.i
  %is_big_str.i.i.i.i.i.i = icmp sgt i64 %.sroa.7.0.i.i.i.i.i.i.i, 0
  br i1 %is_big_str.i.i.i.i.i.i, label %modify_rc.i.i.i.i.i.i, label %else_block7.i.i.i.i

modify_rc.i.i.i.i.i.i:                            ; preds = %else_block.i.i.i.i.i127
  %357 = ptrtoint ptr %.sroa.4.0.i.i.i.i.i.i.i to i64
  %358 = shl nuw i64 %.sroa.7.0.i.i.i.i.i.i.i, 1
  %isneg.i.i.i12.i.i.i.i.i = icmp slt i64 %.sroa.6.0.i.i.i.i.i.i.i, 0
  %359 = select i1 %isneg.i.i.i12.i.i.i.i.i, i64 %358, i64 %357
  %360 = inttoptr i64 %359 to ptr
  %get_rc_ptr.i.i.i.i.i.i = getelementptr inbounds i64, ptr %360, i64 -1
  %361 = load i64, ptr %get_rc_ptr.i.i.i.i.i.i, align 8
  %.not.i.i.i.i.i17.i.i = icmp eq i64 %361, 0
  br i1 %.not.i.i.i.i.i17.i.i, label %else_block7.i.i.i.i, label %362

362:                                              ; preds = %modify_rc.i.i.i.i.i.i
  %363 = atomicrmw sub ptr %get_rc_ptr.i.i.i.i.i.i, i64 1 monotonic, align 8
  %364 = icmp eq i64 %363, 1
  br i1 %364, label %365, label %else_block7.i.i.i.i

365:                                              ; preds = %362
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i.i.i.i, i32 8)
  br label %else_block7.i.i.i.i

else_block.i.i.i.i:                               ; preds = %then_block.i14.i.i, %then_block.i14.thread.i.i
  call fastcc void @"#Attr_#dec_5"(ptr nonnull %load_element.i.i.i)
  store ptr getelementptr inbounds ([48 x i8], ptr @_str_literal_7793802679861948314, i64 0, i64 8), ptr %const_str_store11.i.i.i.i, align 8
  %.fca.1.gep13.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store11.i.i.i.i, i64 0, i32 1
  store <2 x i64> <i64 40, i64 40>, ptr %.fca.1.gep13.i.i.i.i, align 8
  call void @roc_panic(ptr %const_str_store11.i.i.i.i, i32 1)
  unreachable

else_block7.i.i.i.i:                              ; preds = %365, %362, %modify_rc.i.i.i.i.i.i, %else_block.i.i.i.i.i127
  store i64 %.sroa.0.0.i.i.i.i.i.i.i, ptr %result_value.i.i.i.i, align 8
  %tag_alloca9.sroa.2.0..sroa_idx.i.i.i.i.i = getelementptr inbounds i8, ptr %result_value.i.i.i.i, i64 8
  store i8 %.sroa.10.0.i.i.i.i.i.i.i, ptr %tag_alloca9.sroa.2.0..sroa_idx.i.i.i.i.i, align 8
  %tag_alloca9.sroa.316.0..sroa_idx.i.i.i.i.i = getelementptr inbounds i8, ptr %result_value.i.i.i.i, i64 24
  store i8 0, ptr %tag_alloca9.sroa.316.0..sroa_idx.i.i.i.i.i, align 8
  call fastcc void @"#Attr_#dec_17"(ptr nonnull %result_value.i.i.i.i)
  store ptr getelementptr inbounds ([53 x i8], ptr @_str_literal_6738018046837421198, i64 0, i64 8), ptr %const_str_store.i.i.i.i, align 8
  %.fca.1.gep.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store.i.i.i.i, i64 0, i32 1
  store <2 x i64> <i64 45, i64 45>, ptr %.fca.1.gep.i.i.i.i, align 8
  call void @roc_panic(ptr %const_str_store.i.i.i.i, i32 1)
  unreachable

joinpointcont.thread.i.i:                         ; preds = %List_map_b7c59c3aec44645db91b229c81990d288c86aeb49f116d1eae85e2b9a39999f.exit
  store ptr inttoptr (i64 48 to ptr), ptr %joinpoint_arg_alloca.i.i, align 8
  %tmp_output_for_jmp3.sroa.2.0.joinpoint_arg_alloca.sroa_idx.i.i = getelementptr inbounds i8, ptr %joinpoint_arg_alloca.i.i, i64 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %tmp_output_for_jmp3.sroa.2.0.joinpoint_arg_alloca.sroa_idx.i.i, align 8
  %call.i.i87.i.i = call fastcc { i64, i8 } @Str_str_to_num_926c4e1deae44cb32fa91b0fc2f966fdf98af98ee562517f2d5df6cc1b8bf0(ptr nonnull %joinpoint_arg_alloca.i.i)
  br label %"#UserApp_argument_55d628abedfcc0e78784ef0cac7a0c47c774ce17d4fb8ba44f37721d7f3d98.exit.i"

joinpointcont.i.i:                                ; preds = %Str_from_utf8_lowlevel_653de62d66328bd02e166b818cc28fc7f76ad909358c5f3f340dd1c827546c9.exit.i.i.i.i.i
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store11.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 32, ptr nonnull %result_value.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 32, ptr nonnull %load_element.i.i.i)
  store ptr %.sroa.4.0.i.i.i.i.i.i.i, ptr %joinpoint_arg_alloca.i.i, align 8
  %tmp_output_for_jmp.sroa.2.0.joinpoint_arg_alloca.sroa_idx.i.i = getelementptr inbounds i8, ptr %joinpoint_arg_alloca.i.i, i64 8
  store i64 %.sroa.6.0.i.i.i.i.i.i.i, ptr %tmp_output_for_jmp.sroa.2.0.joinpoint_arg_alloca.sroa_idx.i.i, align 8
  %tmp_output_for_jmp.sroa.3.0.joinpoint_arg_alloca.sroa_idx.i.i = getelementptr inbounds i8, ptr %joinpoint_arg_alloca.i.i, i64 16
  store i64 %.sroa.7.0.i.i.i.i.i.i.i, ptr %tmp_output_for_jmp.sroa.3.0.joinpoint_arg_alloca.sroa_idx.i.i, align 8
  %call.i.i.i.i = call fastcc { i64, i8 } @Str_str_to_num_926c4e1deae44cb32fa91b0fc2f966fdf98af98ee562517f2d5df6cc1b8bf0(ptr nonnull %joinpoint_arg_alloca.i.i)
  %is_big_str.i.i.i = icmp sgt i64 %.sroa.7.0.i.i.i.i.i.i.i, 0
  br i1 %is_big_str.i.i.i, label %modify_rc.i.i.i, label %"#UserApp_argument_55d628abedfcc0e78784ef0cac7a0c47c774ce17d4fb8ba44f37721d7f3d98.exit.i"

modify_rc.i.i.i:                                  ; preds = %joinpointcont.i.i
  %366 = icmp slt i64 %.sroa.6.0.i.i.i.i.i.i.i, 0
  %367 = ptrtoint ptr %.sroa.4.0.i.i.i.i.i.i.i to i64
  %368 = shl nuw i64 %.sroa.7.0.i.i.i.i.i.i.i, 1
  %369 = select i1 %366, i64 %368, i64 %367
  %370 = inttoptr i64 %369 to ptr
  %get_rc_ptr.i.i.i = getelementptr inbounds i64, ptr %370, i64 -1
  %371 = load i64, ptr %get_rc_ptr.i.i.i, align 8
  %.not.i.i.i.i126 = icmp eq i64 %371, 0
  br i1 %.not.i.i.i.i126, label %"#UserApp_argument_55d628abedfcc0e78784ef0cac7a0c47c774ce17d4fb8ba44f37721d7f3d98.exit.i", label %372

372:                                              ; preds = %modify_rc.i.i.i
  %373 = atomicrmw sub ptr %get_rc_ptr.i.i.i, i64 1 monotonic, align 8
  %374 = icmp eq i64 %373, 1
  br i1 %374, label %375, label %"#UserApp_argument_55d628abedfcc0e78784ef0cac7a0c47c774ce17d4fb8ba44f37721d7f3d98.exit.i"

375:                                              ; preds = %372
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i, i32 8)
  br label %"#UserApp_argument_55d628abedfcc0e78784ef0cac7a0c47c774ce17d4fb8ba44f37721d7f3d98.exit.i"

"#UserApp_argument_55d628abedfcc0e78784ef0cac7a0c47c774ce17d4fb8ba44f37721d7f3d98.exit.i": ; preds = %375, %372, %modify_rc.i.i.i, %joinpointcont.i.i, %joinpointcont.thread.i.i
  %call.i.i89.i.i = phi { i64, i8 } [ %call.i.i87.i.i, %joinpointcont.thread.i.i ], [ %call.i.i.i.i, %joinpointcont.i.i ], [ %call.i.i.i.i, %modify_rc.i.i.i ], [ %call.i.i.i.i, %372 ], [ %call.i.i.i.i, %375 ]
  %struct_field_access_record_1.i.i.i.i = extractvalue { i64, i8 } %call.i.i89.i.i, 1
  %eq_u8.i.i.i.i.i = icmp eq i8 %struct_field_access_record_1.i.i.i.i, 0
  %struct_field_access_record_0.i.i.i.i = extractvalue { i64, i8 } %call.i.i89.i.i, 0
  %376 = tail call i64 @llvm.umax.i64(i64 %struct_field_access_record_0.i.i.i.i, i64 6)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %joinpoint_arg_alloca.i.i)
  %a.b.i.i = select i1 %eq_u8.i.i.i.i.i, i64 %376, i64 6
  %call.i.i = tail call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %a.b.i.i, i64 1)
  %has_overflowed.i.i = extractvalue { i64, i1 } %call.i.i, 1
  br i1 %has_overflowed.i.i, label %throw_block.i.i, label %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i

throw_block.i.i:                                  ; preds = %"#UserApp_argument_55d628abedfcc0e78784ef0cac7a0c47c774ce17d4fb8ba44f37721d7f3d98.exit.i"
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i: ; preds = %"#UserApp_argument_55d628abedfcc0e78784ef0cac7a0c47c774ce17d4fb8ba44f37721d7f3d98.exit.i"
  %operation_result.i.i = extractvalue { i64, i1 } %call.i.i, 0
  %call3.i = tail call fastcc ptr @"#UserApp_build_1e4d2f1e6b4984301a1489b71481ade3a818d1fae80b8f87ea525c7bff923"(i64 %operation_result.i.i, i64 1)
  %call4.i = tail call fastcc i64 @"#UserApp_check_b7aa9f7d377b2692ada596045493ead6d491b934dc9015fcbdd1a8e01477d"(ptr %call3.i)
  %call5.i = tail call fastcc ptr @"#UserApp_build_1e4d2f1e6b4984301a1489b71481ade3a818d1fae80b8f87ea525c7bff923"(i64 %a.b.i.i, i64 1)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value16.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value14.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value13.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value12.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value11.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value10.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store9.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value8.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store7.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value6.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %joinpoint_arg_alloca.i47.i)
  %const_str_store.sroa.3.0.joinpoint_arg_alloca.i47.sroa_idx.i = getelementptr inbounds i8, ptr %joinpoint_arg_alloca.i47.i, i64 16
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %joinpoint_arg_alloca.i47.i, i8 0, i64 16, i1 false)
  store i64 -9223372036854775808, ptr %const_str_store.sroa.3.0.joinpoint_arg_alloca.i47.sroa_idx.i, align 8
  %.fca.1.gep.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store.i.i, i64 0, i32 1
  %.fca.1.gep19.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store7.i.i, i64 0, i32 1
  %.fca.1.gep22.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store9.i.i, i64 0, i32 1
  %load_str_to_stack.elt3.i28.i.i = getelementptr inbounds %str.RocStr, ptr %result_value10.i.i, i64 0, i32 2
  %.sroa.2.0..sroa_idx.i.i33.i.i = getelementptr inbounds i8, ptr %result_value10.i.i, i64 8
  %load_str_to_stack.elt3.i39.i.i = getelementptr inbounds %str.RocStr, ptr %result_value11.i.i, i64 0, i32 2
  %.sroa.2.0..sroa_idx.i.i44.i.i = getelementptr inbounds i8, ptr %result_value11.i.i, i64 8
  %load_str_to_stack.elt3.i50.i.i = getelementptr inbounds %str.RocStr, ptr %result_value12.i.i, i64 0, i32 2
  %.sroa.2.0..sroa_idx.i.i55.i.i = getelementptr inbounds i8, ptr %result_value12.i.i, i64 8
  %load_str_to_stack.elt3.i61.i.i = getelementptr inbounds %str.RocStr, ptr %result_value13.i.i, i64 0, i32 2
  %.sroa.2.0..sroa_idx.i.i66.i.i = getelementptr inbounds i8, ptr %result_value13.i.i, i64 8
  %load_str_to_stack.elt3.i77.i.i = getelementptr inbounds %str.RocStr, ptr %result_value14.i.i, i64 0, i32 2
  %.sroa.2.0..sroa_idx.i.i82.i.i = getelementptr inbounds i8, ptr %result_value14.i.i, i64 8
  br label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i

Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i: ; preds = %"#Attr_#dec_1.exit87.i.i", %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i
  %joinpointarg90.i.i = phi i64 [ 4, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i ], [ %operation_result.i74.i.i, %"#Attr_#dec_1.exit87.i.i" ]
  %call.i.i.i = sub nuw i64 %a.b.i.i, %joinpointarg90.i.i
  %call.i24.i.i = tail call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %call.i.i.i, i64 4)
  %has_overflowed.i25.i.i = extractvalue { i64, i1 } %call.i24.i.i, 1
  br i1 %has_overflowed.i25.i.i, label %throw_block.i27.i.i, label %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i

throw_block.i27.i.i:                              ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i: ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i
  %operation_result.i26.i.i = extractvalue { i64, i1 } %call.i24.i.i, 0
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %.sroa.215.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %.sroa.213.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %.sroa.2.i.i.i.i.i)
  %377 = icmp ugt i64 %operation_result.i26.i.i, 63
  br i1 %377, label %.loopexit.i.i.i.i, label %.lr.ph.i.i.i.i.i

.lr.ph.i.i.i.i.i:                                 ; preds = %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i, %389
  %.sroa.0.026.i.i.i.i.i = phi i64 [ %390, %389 ], [ 2, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i ]
  %.sroa.04.025.i.i.i.i.i = phi i64 [ %391, %389 ], [ %operation_result.i26.i.i, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i ]
  %.sroa.08.024.i.i.i.i.i = phi i64 [ %.sroa.08.1.i.i.i.i.i, %389 ], [ 1, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i ]
  %378 = and i64 %.sroa.04.025.i.i.i.i.i, 1
  %.not17.i.i.i.i.i = icmp eq i64 %378, 0
  br i1 %.not17.i.i.i.i.i, label %379, label %383

379:                                              ; preds = %387, %.lr.ph.i.i.i.i.i
  %.sroa.08.1.i.i.i.i.i = phi i64 [ %388, %387 ], [ %.sroa.08.024.i.i.i.i.i, %.lr.ph.i.i.i.i.i ]
  %380 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %.sroa.0.026.i.i.i.i.i, i64 %.sroa.0.026.i.i.i.i.i)
  %381 = extractvalue { i64, i1 } %380, 1
  store i1 %381, ptr %.sroa.213.i.i.i.i.i, align 8, !noalias !112
  %.sroa.213.i.i.i.i.i.0..sroa.213.i.i.i.i.i.0..sroa.213.i.i.i.i.i.0..sroa.213.i.i.i.i.0..sroa.213.i.i.i.i.0..sroa.213.i.i.i.0..sroa.213.i.i.i.0..sroa.213.i.i.0..sroa.213.i.i.0..sroa.213.i.0..sroa.213.i.0..sroa.213.0..sroa.213.0..sroa.213.0..sroa.213.8..i.i.i.i.i = load i8, ptr %.sroa.213.i.i.i.i.i, align 8, !noalias !112
  %382 = and i8 %.sroa.213.i.i.i.i.i.0..sroa.213.i.i.i.i.i.0..sroa.213.i.i.i.i.i.0..sroa.213.i.i.i.i.0..sroa.213.i.i.i.i.0..sroa.213.i.i.i.0..sroa.213.i.i.i.0..sroa.213.i.i.0..sroa.213.i.i.0..sroa.213.i.0..sroa.213.i.0..sroa.213.0..sroa.213.0..sroa.213.0..sroa.213.8..i.i.i.i.i, 1
  %.not19.i.i.i.i.i = icmp eq i8 %382, 0
  br i1 %.not19.i.i.i.i.i, label %389, label %.loopexit.i.i.i.i

383:                                              ; preds = %.lr.ph.i.i.i.i.i
  %384 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %.sroa.08.024.i.i.i.i.i, i64 %.sroa.0.026.i.i.i.i.i)
  %385 = extractvalue { i64, i1 } %384, 1
  store i1 %385, ptr %.sroa.2.i.i.i.i.i, align 8, !noalias !112
  %.sroa.2.i.i.i.i.i.0..sroa.2.i.i.i.i.i.0..sroa.2.i.i.i.i.i.0..sroa.2.i.i.i.i.0..sroa.2.i.i.i.i.0..sroa.2.i.i.i.0..sroa.2.i.i.i.0..sroa.2.i.i.0..sroa.2.i.i.0..sroa.2.i.0..sroa.2.i.0..sroa.2.0..sroa.2.0..sroa.2.0..sroa.2.8..i.i.i.i.i = load i8, ptr %.sroa.2.i.i.i.i.i, align 8, !noalias !112
  %386 = and i8 %.sroa.2.i.i.i.i.i.0..sroa.2.i.i.i.i.i.0..sroa.2.i.i.i.i.i.0..sroa.2.i.i.i.i.0..sroa.2.i.i.i.i.0..sroa.2.i.i.i.0..sroa.2.i.i.i.0..sroa.2.i.i.0..sroa.2.i.i.0..sroa.2.i.0..sroa.2.i.0..sroa.2.0..sroa.2.0..sroa.2.0..sroa.2.8..i.i.i.i.i, 1
  %.not18.i.i.i.i.i = icmp eq i8 %386, 0
  br i1 %.not18.i.i.i.i.i, label %387, label %.loopexit.i.i.i.i

387:                                              ; preds = %383
  %388 = extractvalue { i64, i1 } %384, 0
  br label %379

389:                                              ; preds = %379
  %390 = extractvalue { i64, i1 } %380, 0
  %391 = lshr i64 %.sroa.04.025.i.i.i.i.i, 1
  %392 = icmp ugt i64 %.sroa.04.025.i.i.i.i.i, 3
  br i1 %392, label %.lr.ph.i.i.i.i.i, label %._crit_edge.i.i.i.i.i

._crit_edge.i.i.i.i.i:                            ; preds = %389
  %393 = tail call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %.sroa.08.1.i.i.i.i.i, i64 %390)
  %394 = extractvalue { i64, i1 } %393, 1
  store i1 %394, ptr %.sroa.215.i.i.i.i.i, align 8, !noalias !112
  %.sroa.215.i.i.i.i.i.0..sroa.215.i.i.i.i.i.0..sroa.215.i.i.i.i.i.0..sroa.215.i.i.i.i.0..sroa.215.i.i.i.i.0..sroa.215.i.i.i.0..sroa.215.i.i.i.0..sroa.215.i.i.0..sroa.215.i.i.0..sroa.215.i.0..sroa.215.i.0..sroa.215.0..sroa.215.0..sroa.215.0..sroa.215.8..i.i.i.i.i = load i8, ptr %.sroa.215.i.i.i.i.i, align 8, !noalias !112
  %395 = and i8 %.sroa.215.i.i.i.i.i.0..sroa.215.i.i.i.i.i.0..sroa.215.i.i.i.i.i.0..sroa.215.i.i.i.i.0..sroa.215.i.i.i.i.0..sroa.215.i.i.i.0..sroa.215.i.i.i.0..sroa.215.i.i.0..sroa.215.i.i.0..sroa.215.i.0..sroa.215.i.0..sroa.215.0..sroa.215.0..sroa.215.0..sroa.215.8..i.i.i.i.i, 1
  %.not.i.i.i.i48.i = icmp eq i8 %395, 0
  br i1 %.not.i.i.i.i48.i, label %Num_pow_int_76e6e4fef22a778f22804a4a55d5f106b42fb9eadb9eb1f662982e2379174e.exit.i.i, label %.loopexit.i.i.i.i

Num_pow_int_76e6e4fef22a778f22804a4a55d5f106b42fb9eadb9eb1f662982e2379174e.exit.i.i: ; preds = %._crit_edge.i.i.i.i.i
  %396 = extractvalue { i64, i1 } %393, 0
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.215.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.213.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.2.i.i.i.i.i)
  %eq_u64.i13.i.i.i = icmp eq i64 %396, 0
  br i1 %eq_u64.i13.i.i.i, label %"#Attr_#dec_1.exit.i.i", label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i.i

.loopexit.i.i.i.i:                                ; preds = %._crit_edge.i.i.i.i.i, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i, %383, %379
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.215.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.213.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.2.i.i.i.i.i)
  tail call fastcc void @panic.panic_help()
  unreachable

Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i.i: ; preds = %Num_pow_int_76e6e4fef22a778f22804a4a55d5f106b42fb9eadb9eb1f662982e2379174e.exit.i.i, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i.i
  %joinpointarg215.i.i.i = phi i64 [ %operation_result.i9.i.i.i, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i.i ], [ 0, %Num_pow_int_76e6e4fef22a778f22804a4a55d5f106b42fb9eadb9eb1f662982e2379174e.exit.i.i ]
  %joinpointarg14.i.i.i = phi i64 [ %call.i11.i.i.i, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i.i ], [ %396, %Num_pow_int_76e6e4fef22a778f22804a4a55d5f106b42fb9eadb9eb1f662982e2379174e.exit.i.i ]
  %call4.i.i.i = tail call fastcc ptr @"#UserApp_build_9b7306a2e571f3d11e34b901a57455c3e32e69a2f8fde813ed1c4300e12712"(i64 %joinpointarg90.i.i, i64 %joinpointarg14.i.i.i)
  %call5.i.i.i = tail call fastcc i64 @"#UserApp_check_99c9f773566b1d5d689233ef7949cf16c8797c970a4668678361d8c89d24f20"(ptr %call4.i.i.i)
  %call.i7.i.i.i = tail call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %joinpointarg215.i.i.i, i64 %call5.i.i.i)
  %has_overflowed.i8.i.i.i = extractvalue { i64, i1 } %call.i7.i.i.i, 1
  br i1 %has_overflowed.i8.i.i.i, label %throw_block.i10.i.i.i, label %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i.i

throw_block.i10.i.i.i:                            ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i.i
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i.i: ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i.i
  %call.i11.i.i.i = add i64 %joinpointarg14.i.i.i, -1
  %operation_result.i9.i.i.i = extractvalue { i64, i1 } %call.i7.i.i.i, 0
  %eq_u64.i.i.i.i = icmp eq i64 %call.i11.i.i.i, 0
  br i1 %eq_u64.i.i.i.i, label %"#Attr_#dec_1.exit.i.i", label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i.i

"#Attr_#dec_1.exit.i.i":                          ; preds = %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i.i, %Num_pow_int_76e6e4fef22a778f22804a4a55d5f106b42fb9eadb9eb1f662982e2379174e.exit.i.i
  %joinpointarg2.lcssa.i.i.i = phi i64 [ 0, %Num_pow_int_76e6e4fef22a778f22804a4a55d5f106b42fb9eadb9eb1f662982e2379174e.exit.i.i ], [ %operation_result.i9.i.i.i, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit.i.i.i ]
  call fastcc void @Num_to_str_68697e959be5e5da06cc73b6f998e193cbf2d9b22efd0355a3d37129951b(i64 %396, ptr nonnull %result_value.i.i)
  store ptr inttoptr (i64 2338324117921734665 to ptr), ptr %const_str_store.i.i, align 8
  store <2 x i64> <i64 7526764458016794223, i64 -7998392938210000864>, ptr %.fca.1.gep.i.i, align 8
  call fastcc void @Num_to_str_68697e959be5e5da06cc73b6f998e193cbf2d9b22efd0355a3d37129951b(i64 %joinpointarg90.i.i, ptr nonnull %result_value6.i.i)
  store ptr inttoptr (i64 4209567563902033929 to ptr), ptr %const_str_store7.i.i, align 8
  store <2 x i64> <i64 32, i64 -8574853690513424384>, ptr %.fca.1.gep19.i.i, align 8
  call fastcc void @Num_to_str_68697e959be5e5da06cc73b6f998e193cbf2d9b22efd0355a3d37129951b(i64 %joinpointarg2.lcssa.i.i.i, ptr nonnull %result_value8.i.i)
  store ptr inttoptr (i64 10 to ptr), ptr %const_str_store9.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep22.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value8.i.i, ptr nonnull %const_str_store9.i.i, ptr nonnull %result_value10.i.i)
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %const_str_store7.i.i, ptr nonnull %result_value10.i.i, ptr nonnull %result_value11.i.i)
  %load_str_to_stack.unpack4.i29.i.i = load i64, ptr %load_str_to_stack.elt3.i28.i.i, align 8
  %is_big_str.i30.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i29.i.i, 0
  br i1 %is_big_str.i30.i.i, label %modify_rc.i31.i.i, label %"#Attr_#dec_1.exit38.i.i"

modify_rc.i31.i.i:                                ; preds = %"#Attr_#dec_1.exit.i.i"
  %.sroa.0.0.copyload.i.i32.i.i = load ptr, ptr %result_value10.i.i, align 8
  %.sroa.2.0.copyload.i.i34.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i33.i.i, align 8
  %397 = ptrtoint ptr %.sroa.0.0.copyload.i.i32.i.i to i64
  %398 = shl nuw i64 %load_str_to_stack.unpack4.i29.i.i, 1
  %isneg.i.i.i35.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i34.i.i, 0
  %399 = select i1 %isneg.i.i.i35.i.i, i64 %398, i64 %397
  %400 = inttoptr i64 %399 to ptr
  %get_rc_ptr.i36.i.i = getelementptr inbounds i64, ptr %400, i64 -1
  %401 = load i64, ptr %get_rc_ptr.i36.i.i, align 8
  %.not.i.i37.i.i = icmp eq i64 %401, 0
  br i1 %.not.i.i37.i.i, label %"#Attr_#dec_1.exit38.i.i", label %402

402:                                              ; preds = %modify_rc.i31.i.i
  %403 = atomicrmw sub ptr %get_rc_ptr.i36.i.i, i64 1 monotonic, align 8
  %404 = icmp eq i64 %403, 1
  br i1 %404, label %405, label %"#Attr_#dec_1.exit38.i.i"

405:                                              ; preds = %402
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i36.i.i, i32 8)
  br label %"#Attr_#dec_1.exit38.i.i"

"#Attr_#dec_1.exit38.i.i":                        ; preds = %405, %402, %modify_rc.i31.i.i, %"#Attr_#dec_1.exit.i.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value6.i.i, ptr nonnull %result_value11.i.i, ptr nonnull %result_value12.i.i)
  %load_str_to_stack.unpack4.i40.i.i = load i64, ptr %load_str_to_stack.elt3.i39.i.i, align 8
  %is_big_str.i41.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i40.i.i, 0
  br i1 %is_big_str.i41.i.i, label %modify_rc.i42.i.i, label %"#Attr_#dec_1.exit49.i.i"

modify_rc.i42.i.i:                                ; preds = %"#Attr_#dec_1.exit38.i.i"
  %.sroa.0.0.copyload.i.i43.i.i = load ptr, ptr %result_value11.i.i, align 8
  %.sroa.2.0.copyload.i.i45.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i44.i.i, align 8
  %406 = ptrtoint ptr %.sroa.0.0.copyload.i.i43.i.i to i64
  %407 = shl nuw i64 %load_str_to_stack.unpack4.i40.i.i, 1
  %isneg.i.i.i46.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i45.i.i, 0
  %408 = select i1 %isneg.i.i.i46.i.i, i64 %407, i64 %406
  %409 = inttoptr i64 %408 to ptr
  %get_rc_ptr.i47.i.i = getelementptr inbounds i64, ptr %409, i64 -1
  %410 = load i64, ptr %get_rc_ptr.i47.i.i, align 8
  %.not.i.i48.i.i = icmp eq i64 %410, 0
  br i1 %.not.i.i48.i.i, label %"#Attr_#dec_1.exit49.i.i", label %411

411:                                              ; preds = %modify_rc.i42.i.i
  %412 = atomicrmw sub ptr %get_rc_ptr.i47.i.i, i64 1 monotonic, align 8
  %413 = icmp eq i64 %412, 1
  br i1 %413, label %414, label %"#Attr_#dec_1.exit49.i.i"

414:                                              ; preds = %411
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i47.i.i, i32 8)
  br label %"#Attr_#dec_1.exit49.i.i"

"#Attr_#dec_1.exit49.i.i":                        ; preds = %414, %411, %modify_rc.i42.i.i, %"#Attr_#dec_1.exit38.i.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %const_str_store.i.i, ptr nonnull %result_value12.i.i, ptr nonnull %result_value13.i.i)
  %load_str_to_stack.unpack4.i51.i.i = load i64, ptr %load_str_to_stack.elt3.i50.i.i, align 8
  %is_big_str.i52.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i51.i.i, 0
  br i1 %is_big_str.i52.i.i, label %modify_rc.i53.i.i, label %"#Attr_#dec_1.exit60.i.i"

modify_rc.i53.i.i:                                ; preds = %"#Attr_#dec_1.exit49.i.i"
  %.sroa.0.0.copyload.i.i54.i.i = load ptr, ptr %result_value12.i.i, align 8
  %.sroa.2.0.copyload.i.i56.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i55.i.i, align 8
  %415 = ptrtoint ptr %.sroa.0.0.copyload.i.i54.i.i to i64
  %416 = shl nuw i64 %load_str_to_stack.unpack4.i51.i.i, 1
  %isneg.i.i.i57.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i56.i.i, 0
  %417 = select i1 %isneg.i.i.i57.i.i, i64 %416, i64 %415
  %418 = inttoptr i64 %417 to ptr
  %get_rc_ptr.i58.i.i = getelementptr inbounds i64, ptr %418, i64 -1
  %419 = load i64, ptr %get_rc_ptr.i58.i.i, align 8
  %.not.i.i59.i.i = icmp eq i64 %419, 0
  br i1 %.not.i.i59.i.i, label %"#Attr_#dec_1.exit60.i.i", label %420

420:                                              ; preds = %modify_rc.i53.i.i
  %421 = atomicrmw sub ptr %get_rc_ptr.i58.i.i, i64 1 monotonic, align 8
  %422 = icmp eq i64 %421, 1
  br i1 %422, label %423, label %"#Attr_#dec_1.exit60.i.i"

423:                                              ; preds = %420
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i58.i.i, i32 8)
  br label %"#Attr_#dec_1.exit60.i.i"

"#Attr_#dec_1.exit60.i.i":                        ; preds = %423, %420, %modify_rc.i53.i.i, %"#Attr_#dec_1.exit49.i.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value.i.i, ptr nonnull %result_value13.i.i, ptr nonnull %result_value14.i.i)
  %load_str_to_stack.unpack4.i62.i.i = load i64, ptr %load_str_to_stack.elt3.i61.i.i, align 8
  %is_big_str.i63.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i62.i.i, 0
  br i1 %is_big_str.i63.i.i, label %modify_rc.i64.i.i, label %"#Attr_#dec_1.exit71.i.i"

modify_rc.i64.i.i:                                ; preds = %"#Attr_#dec_1.exit60.i.i"
  %.sroa.0.0.copyload.i.i65.i.i = load ptr, ptr %result_value13.i.i, align 8
  %.sroa.2.0.copyload.i.i67.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i66.i.i, align 8
  %424 = ptrtoint ptr %.sroa.0.0.copyload.i.i65.i.i to i64
  %425 = shl nuw i64 %load_str_to_stack.unpack4.i62.i.i, 1
  %isneg.i.i.i68.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i67.i.i, 0
  %426 = select i1 %isneg.i.i.i68.i.i, i64 %425, i64 %424
  %427 = inttoptr i64 %426 to ptr
  %get_rc_ptr.i69.i.i = getelementptr inbounds i64, ptr %427, i64 -1
  %428 = load i64, ptr %get_rc_ptr.i69.i.i, align 8
  %.not.i.i70.i.i = icmp eq i64 %428, 0
  br i1 %.not.i.i70.i.i, label %"#Attr_#dec_1.exit71.i.i", label %429

429:                                              ; preds = %modify_rc.i64.i.i
  %430 = atomicrmw sub ptr %get_rc_ptr.i69.i.i, i64 1 monotonic, align 8
  %431 = icmp eq i64 %430, 1
  br i1 %431, label %432, label %"#Attr_#dec_1.exit71.i.i"

432:                                              ; preds = %429
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i69.i.i, i32 8)
  br label %"#Attr_#dec_1.exit71.i.i"

"#Attr_#dec_1.exit71.i.i":                        ; preds = %432, %429, %modify_rc.i64.i.i, %"#Attr_#dec_1.exit60.i.i"
  %call.i72.i.i = tail call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %joinpointarg90.i.i, i64 2)
  %has_overflowed.i73.i.i = extractvalue { i64, i1 } %call.i72.i.i, 1
  br i1 %has_overflowed.i73.i.i, label %throw_block.i75.i.i, label %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit76.i.i

throw_block.i75.i.i:                              ; preds = %"#Attr_#dec_1.exit71.i.i"
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit76.i.i: ; preds = %"#Attr_#dec_1.exit71.i.i"
  %operation_result.i74.i.i = extractvalue { i64, i1 } %call.i72.i.i, 0
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %joinpoint_arg_alloca.i47.i, ptr nonnull %result_value14.i.i, ptr nonnull %result_value16.i.i)
  %load_str_to_stack.unpack4.i78.i.i = load i64, ptr %load_str_to_stack.elt3.i77.i.i, align 8
  %is_big_str.i79.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i78.i.i, 0
  br i1 %is_big_str.i79.i.i, label %modify_rc.i80.i.i, label %"#Attr_#dec_1.exit87.i.i"

modify_rc.i80.i.i:                                ; preds = %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit76.i.i
  %.sroa.0.0.copyload.i.i81.i.i = load ptr, ptr %result_value14.i.i, align 8
  %.sroa.2.0.copyload.i.i83.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i82.i.i, align 8
  %433 = ptrtoint ptr %.sroa.0.0.copyload.i.i81.i.i to i64
  %434 = shl nuw i64 %load_str_to_stack.unpack4.i78.i.i, 1
  %isneg.i.i.i84.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i83.i.i, 0
  %435 = select i1 %isneg.i.i.i84.i.i, i64 %434, i64 %433
  %436 = inttoptr i64 %435 to ptr
  %get_rc_ptr.i85.i.i = getelementptr inbounds i64, ptr %436, i64 -1
  %437 = load i64, ptr %get_rc_ptr.i85.i.i, align 8
  %.not.i.i86.i.i = icmp eq i64 %437, 0
  br i1 %.not.i.i86.i.i, label %"#Attr_#dec_1.exit87.i.i", label %438

438:                                              ; preds = %modify_rc.i80.i.i
  %439 = atomicrmw sub ptr %get_rc_ptr.i85.i.i, i64 1 monotonic, align 8
  %440 = icmp eq i64 %439, 1
  br i1 %440, label %441, label %"#Attr_#dec_1.exit87.i.i"

441:                                              ; preds = %438
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i85.i.i, i32 8)
  br label %"#Attr_#dec_1.exit87.i.i"

"#Attr_#dec_1.exit87.i.i":                        ; preds = %441, %438, %modify_rc.i80.i.i, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit76.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %joinpoint_arg_alloca.i47.i, ptr noundef nonnull align 8 dereferenceable(24) %result_value16.i.i, i64 24, i1 false)
  %gt_uint.i.i.i = icmp ugt i64 %operation_result.i74.i.i, %a.b.i.i
  br i1 %gt_uint.i.i.i, label %"#Attr_#dec_1.exit.i", label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i

"#Attr_#dec_1.exit.i":                            ; preds = %"#Attr_#dec_1.exit87.i.i"
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %result_value.i, ptr noundef nonnull align 8 dereferenceable(24) %result_value16.i.i, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value16.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value14.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value13.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value12.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value11.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value10.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store9.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value8.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store7.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value6.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %joinpoint_arg_alloca.i47.i)
  %call6.i = tail call fastcc i64 @"#UserApp_check_b7aa9f7d377b2692ada596045493ead6d491b934dc9015fcbdd1a8e01477d"(ptr %call5.i)
  store ptr inttoptr (i64 2335225758360958067 to ptr), ptr %const_str_store7.i, align 8
  %.fca.1.gep30.i = getelementptr inbounds %str.RocStr, ptr %const_str_store7.i, i64 0, i32 1
  store <2 x i64> <i64 2334675641768964724, i64 -7638069335018150556>, ptr %.fca.1.gep30.i, align 8
  call fastcc void @Num_to_str_68697e959be5e5da06cc73b6f998e193cbf2d9b22efd0355a3d37129951b(i64 %operation_result.i.i, ptr nonnull %result_value9.i)
  store ptr inttoptr (i64 4209567563902033929 to ptr), ptr %const_str_store10.i, align 8
  %.fca.1.gep33.i = getelementptr inbounds %str.RocStr, ptr %const_str_store10.i, i64 0, i32 1
  store <2 x i64> <i64 32, i64 -8574853690513424384>, ptr %.fca.1.gep33.i, align 8
  call fastcc void @Num_to_str_68697e959be5e5da06cc73b6f998e193cbf2d9b22efd0355a3d37129951b(i64 %call4.i, ptr nonnull %result_value11.i)
  store ptr inttoptr (i64 10 to ptr), ptr %const_str_store12.i, align 8
  %.fca.1.gep36.i = getelementptr inbounds %str.RocStr, ptr %const_str_store12.i, i64 0, i32 1
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep36.i, align 8
  store ptr getelementptr inbounds ([33 x i8], ptr @_str_literal_11270832565531248303, i64 0, i64 8), ptr %const_str_store13.i, align 8
  %.fca.1.gep39.i = getelementptr inbounds %str.RocStr, ptr %const_str_store13.i, i64 0, i32 1
  store <2 x i64> <i64 25, i64 25>, ptr %.fca.1.gep39.i, align 8
  call fastcc void @Num_to_str_68697e959be5e5da06cc73b6f998e193cbf2d9b22efd0355a3d37129951b(i64 %a.b.i.i, ptr nonnull %result_value14.i)
  store ptr inttoptr (i64 4209567563902033929 to ptr), ptr %const_str_store15.i, align 8
  %.fca.1.gep42.i = getelementptr inbounds %str.RocStr, ptr %const_str_store15.i, i64 0, i32 1
  store <2 x i64> <i64 32, i64 -8574853690513424384>, ptr %.fca.1.gep42.i, align 8
  call fastcc void @Num_to_str_68697e959be5e5da06cc73b6f998e193cbf2d9b22efd0355a3d37129951b(i64 %call6.i, ptr nonnull %result_value16.i)
  store ptr inttoptr (i64 10 to ptr), ptr %const_str_store17.i, align 8
  %.fca.1.gep45.i = getelementptr inbounds %str.RocStr, ptr %const_str_store17.i, i64 0, i32 1
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep45.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value16.i, ptr nonnull %const_str_store17.i, ptr nonnull %result_value18.i)
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %const_str_store15.i, ptr nonnull %result_value18.i, ptr nonnull %result_value19.i)
  %load_str_to_stack.elt3.i54.i = getelementptr inbounds %str.RocStr, ptr %result_value18.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i55.i = load i64, ptr %load_str_to_stack.elt3.i54.i, align 8
  %is_big_str.i56.i = icmp sgt i64 %load_str_to_stack.unpack4.i55.i, 0
  br i1 %is_big_str.i56.i, label %modify_rc.i57.i, label %"#Attr_#dec_1.exit64.i"

modify_rc.i57.i:                                  ; preds = %"#Attr_#dec_1.exit.i"
  %.sroa.0.0.copyload.i.i58.i = load ptr, ptr %result_value18.i, align 8
  %.sroa.2.0..sroa_idx.i.i59.i = getelementptr inbounds i8, ptr %result_value18.i, i64 8
  %.sroa.2.0.copyload.i.i60.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i59.i, align 8
  %442 = ptrtoint ptr %.sroa.0.0.copyload.i.i58.i to i64
  %443 = shl nuw i64 %load_str_to_stack.unpack4.i55.i, 1
  %isneg.i.i.i61.i = icmp slt i64 %.sroa.2.0.copyload.i.i60.i, 0
  %444 = select i1 %isneg.i.i.i61.i, i64 %443, i64 %442
  %445 = inttoptr i64 %444 to ptr
  %get_rc_ptr.i62.i = getelementptr inbounds i64, ptr %445, i64 -1
  %446 = load i64, ptr %get_rc_ptr.i62.i, align 8
  %.not.i.i63.i = icmp eq i64 %446, 0
  br i1 %.not.i.i63.i, label %"#Attr_#dec_1.exit64.i", label %447

447:                                              ; preds = %modify_rc.i57.i
  %448 = atomicrmw sub ptr %get_rc_ptr.i62.i, i64 1 monotonic, align 8
  %449 = icmp eq i64 %448, 1
  br i1 %449, label %450, label %"#Attr_#dec_1.exit64.i"

450:                                              ; preds = %447
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i62.i, i32 8)
  br label %"#Attr_#dec_1.exit64.i"

"#Attr_#dec_1.exit64.i":                          ; preds = %450, %447, %modify_rc.i57.i, %"#Attr_#dec_1.exit.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value14.i, ptr nonnull %result_value19.i, ptr nonnull %result_value20.i)
  %load_str_to_stack.elt3.i65.i = getelementptr inbounds %str.RocStr, ptr %result_value19.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i66.i = load i64, ptr %load_str_to_stack.elt3.i65.i, align 8
  %is_big_str.i67.i = icmp sgt i64 %load_str_to_stack.unpack4.i66.i, 0
  br i1 %is_big_str.i67.i, label %modify_rc.i68.i, label %"#Attr_#dec_1.exit75.i"

modify_rc.i68.i:                                  ; preds = %"#Attr_#dec_1.exit64.i"
  %.sroa.0.0.copyload.i.i69.i = load ptr, ptr %result_value19.i, align 8
  %.sroa.2.0..sroa_idx.i.i70.i = getelementptr inbounds i8, ptr %result_value19.i, i64 8
  %.sroa.2.0.copyload.i.i71.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i70.i, align 8
  %451 = ptrtoint ptr %.sroa.0.0.copyload.i.i69.i to i64
  %452 = shl nuw i64 %load_str_to_stack.unpack4.i66.i, 1
  %isneg.i.i.i72.i = icmp slt i64 %.sroa.2.0.copyload.i.i71.i, 0
  %453 = select i1 %isneg.i.i.i72.i, i64 %452, i64 %451
  %454 = inttoptr i64 %453 to ptr
  %get_rc_ptr.i73.i = getelementptr inbounds i64, ptr %454, i64 -1
  %455 = load i64, ptr %get_rc_ptr.i73.i, align 8
  %.not.i.i74.i = icmp eq i64 %455, 0
  br i1 %.not.i.i74.i, label %"#Attr_#dec_1.exit75.i", label %456

456:                                              ; preds = %modify_rc.i68.i
  %457 = atomicrmw sub ptr %get_rc_ptr.i73.i, i64 1 monotonic, align 8
  %458 = icmp eq i64 %457, 1
  br i1 %458, label %459, label %"#Attr_#dec_1.exit75.i"

459:                                              ; preds = %456
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i73.i, i32 8)
  br label %"#Attr_#dec_1.exit75.i"

"#Attr_#dec_1.exit75.i":                          ; preds = %459, %456, %modify_rc.i68.i, %"#Attr_#dec_1.exit64.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %const_str_store13.i, ptr nonnull %result_value20.i, ptr nonnull %result_value21.i)
  %load_str_to_stack.elt3.i76.i = getelementptr inbounds %str.RocStr, ptr %result_value20.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i77.i = load i64, ptr %load_str_to_stack.elt3.i76.i, align 8
  %is_big_str.i78.i = icmp sgt i64 %load_str_to_stack.unpack4.i77.i, 0
  br i1 %is_big_str.i78.i, label %modify_rc.i79.i, label %"#Attr_#dec_1.exit86.i"

modify_rc.i79.i:                                  ; preds = %"#Attr_#dec_1.exit75.i"
  %.sroa.0.0.copyload.i.i80.i = load ptr, ptr %result_value20.i, align 8
  %.sroa.2.0..sroa_idx.i.i81.i = getelementptr inbounds i8, ptr %result_value20.i, i64 8
  %.sroa.2.0.copyload.i.i82.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i81.i, align 8
  %460 = ptrtoint ptr %.sroa.0.0.copyload.i.i80.i to i64
  %461 = shl nuw i64 %load_str_to_stack.unpack4.i77.i, 1
  %isneg.i.i.i83.i = icmp slt i64 %.sroa.2.0.copyload.i.i82.i, 0
  %462 = select i1 %isneg.i.i.i83.i, i64 %461, i64 %460
  %463 = inttoptr i64 %462 to ptr
  %get_rc_ptr.i84.i = getelementptr inbounds i64, ptr %463, i64 -1
  %464 = load i64, ptr %get_rc_ptr.i84.i, align 8
  %.not.i.i85.i = icmp eq i64 %464, 0
  br i1 %.not.i.i85.i, label %"#Attr_#dec_1.exit86.i", label %465

465:                                              ; preds = %modify_rc.i79.i
  %466 = atomicrmw sub ptr %get_rc_ptr.i84.i, i64 1 monotonic, align 8
  %467 = icmp eq i64 %466, 1
  br i1 %467, label %468, label %"#Attr_#dec_1.exit86.i"

468:                                              ; preds = %465
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i84.i, i32 8)
  br label %"#Attr_#dec_1.exit86.i"

"#Attr_#dec_1.exit86.i":                          ; preds = %468, %465, %modify_rc.i79.i, %"#Attr_#dec_1.exit75.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value.i, ptr nonnull %result_value21.i, ptr nonnull %result_value22.i)
  %load_str_to_stack.elt3.i87.i = getelementptr inbounds %str.RocStr, ptr %result_value21.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i88.i = load i64, ptr %load_str_to_stack.elt3.i87.i, align 8
  %is_big_str.i89.i = icmp sgt i64 %load_str_to_stack.unpack4.i88.i, 0
  br i1 %is_big_str.i89.i, label %modify_rc.i90.i, label %"#Attr_#dec_1.exit97.i"

modify_rc.i90.i:                                  ; preds = %"#Attr_#dec_1.exit86.i"
  %.sroa.0.0.copyload.i.i91.i = load ptr, ptr %result_value21.i, align 8
  %.sroa.2.0..sroa_idx.i.i92.i = getelementptr inbounds i8, ptr %result_value21.i, i64 8
  %.sroa.2.0.copyload.i.i93.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i92.i, align 8
  %469 = ptrtoint ptr %.sroa.0.0.copyload.i.i91.i to i64
  %470 = shl nuw i64 %load_str_to_stack.unpack4.i88.i, 1
  %isneg.i.i.i94.i = icmp slt i64 %.sroa.2.0.copyload.i.i93.i, 0
  %471 = select i1 %isneg.i.i.i94.i, i64 %470, i64 %469
  %472 = inttoptr i64 %471 to ptr
  %get_rc_ptr.i95.i = getelementptr inbounds i64, ptr %472, i64 -1
  %473 = load i64, ptr %get_rc_ptr.i95.i, align 8
  %.not.i.i96.i = icmp eq i64 %473, 0
  br i1 %.not.i.i96.i, label %"#Attr_#dec_1.exit97.i", label %474

474:                                              ; preds = %modify_rc.i90.i
  %475 = atomicrmw sub ptr %get_rc_ptr.i95.i, i64 1 monotonic, align 8
  %476 = icmp eq i64 %475, 1
  br i1 %476, label %477, label %"#Attr_#dec_1.exit97.i"

477:                                              ; preds = %474
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i95.i, i32 8)
  br label %"#Attr_#dec_1.exit97.i"

"#Attr_#dec_1.exit97.i":                          ; preds = %477, %474, %modify_rc.i90.i, %"#Attr_#dec_1.exit86.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %const_str_store12.i, ptr nonnull %result_value22.i, ptr nonnull %result_value23.i)
  %load_str_to_stack.elt3.i98.i = getelementptr inbounds %str.RocStr, ptr %result_value22.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i99.i = load i64, ptr %load_str_to_stack.elt3.i98.i, align 8
  %is_big_str.i100.i = icmp sgt i64 %load_str_to_stack.unpack4.i99.i, 0
  br i1 %is_big_str.i100.i, label %modify_rc.i101.i, label %"#Attr_#dec_1.exit108.i"

modify_rc.i101.i:                                 ; preds = %"#Attr_#dec_1.exit97.i"
  %.sroa.0.0.copyload.i.i102.i = load ptr, ptr %result_value22.i, align 8
  %.sroa.2.0..sroa_idx.i.i103.i = getelementptr inbounds i8, ptr %result_value22.i, i64 8
  %.sroa.2.0.copyload.i.i104.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i103.i, align 8
  %478 = ptrtoint ptr %.sroa.0.0.copyload.i.i102.i to i64
  %479 = shl nuw i64 %load_str_to_stack.unpack4.i99.i, 1
  %isneg.i.i.i105.i = icmp slt i64 %.sroa.2.0.copyload.i.i104.i, 0
  %480 = select i1 %isneg.i.i.i105.i, i64 %479, i64 %478
  %481 = inttoptr i64 %480 to ptr
  %get_rc_ptr.i106.i = getelementptr inbounds i64, ptr %481, i64 -1
  %482 = load i64, ptr %get_rc_ptr.i106.i, align 8
  %.not.i.i107.i = icmp eq i64 %482, 0
  br i1 %.not.i.i107.i, label %"#Attr_#dec_1.exit108.i", label %483

483:                                              ; preds = %modify_rc.i101.i
  %484 = atomicrmw sub ptr %get_rc_ptr.i106.i, i64 1 monotonic, align 8
  %485 = icmp eq i64 %484, 1
  br i1 %485, label %486, label %"#Attr_#dec_1.exit108.i"

486:                                              ; preds = %483
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i106.i, i32 8)
  br label %"#Attr_#dec_1.exit108.i"

"#Attr_#dec_1.exit108.i":                         ; preds = %486, %483, %modify_rc.i101.i, %"#Attr_#dec_1.exit97.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value11.i, ptr nonnull %result_value23.i, ptr nonnull %result_value24.i)
  %load_str_to_stack.elt3.i109.i = getelementptr inbounds %str.RocStr, ptr %result_value23.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i110.i = load i64, ptr %load_str_to_stack.elt3.i109.i, align 8
  %is_big_str.i111.i = icmp sgt i64 %load_str_to_stack.unpack4.i110.i, 0
  br i1 %is_big_str.i111.i, label %modify_rc.i112.i, label %"#Attr_#dec_1.exit119.i"

modify_rc.i112.i:                                 ; preds = %"#Attr_#dec_1.exit108.i"
  %.sroa.0.0.copyload.i.i113.i = load ptr, ptr %result_value23.i, align 8
  %.sroa.2.0..sroa_idx.i.i114.i = getelementptr inbounds i8, ptr %result_value23.i, i64 8
  %.sroa.2.0.copyload.i.i115.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i114.i, align 8
  %487 = ptrtoint ptr %.sroa.0.0.copyload.i.i113.i to i64
  %488 = shl nuw i64 %load_str_to_stack.unpack4.i110.i, 1
  %isneg.i.i.i116.i = icmp slt i64 %.sroa.2.0.copyload.i.i115.i, 0
  %489 = select i1 %isneg.i.i.i116.i, i64 %488, i64 %487
  %490 = inttoptr i64 %489 to ptr
  %get_rc_ptr.i117.i = getelementptr inbounds i64, ptr %490, i64 -1
  %491 = load i64, ptr %get_rc_ptr.i117.i, align 8
  %.not.i.i118.i = icmp eq i64 %491, 0
  br i1 %.not.i.i118.i, label %"#Attr_#dec_1.exit119.i", label %492

492:                                              ; preds = %modify_rc.i112.i
  %493 = atomicrmw sub ptr %get_rc_ptr.i117.i, i64 1 monotonic, align 8
  %494 = icmp eq i64 %493, 1
  br i1 %494, label %495, label %"#Attr_#dec_1.exit119.i"

495:                                              ; preds = %492
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i117.i, i32 8)
  br label %"#Attr_#dec_1.exit119.i"

"#Attr_#dec_1.exit119.i":                         ; preds = %495, %492, %modify_rc.i112.i, %"#Attr_#dec_1.exit108.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %const_str_store10.i, ptr nonnull %result_value24.i, ptr nonnull %result_value25.i)
  %load_str_to_stack.elt3.i120.i = getelementptr inbounds %str.RocStr, ptr %result_value24.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i121.i = load i64, ptr %load_str_to_stack.elt3.i120.i, align 8
  %is_big_str.i122.i = icmp sgt i64 %load_str_to_stack.unpack4.i121.i, 0
  br i1 %is_big_str.i122.i, label %modify_rc.i123.i, label %"#Attr_#dec_1.exit130.i"

modify_rc.i123.i:                                 ; preds = %"#Attr_#dec_1.exit119.i"
  %.sroa.0.0.copyload.i.i124.i = load ptr, ptr %result_value24.i, align 8
  %.sroa.2.0..sroa_idx.i.i125.i = getelementptr inbounds i8, ptr %result_value24.i, i64 8
  %.sroa.2.0.copyload.i.i126.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i125.i, align 8
  %496 = ptrtoint ptr %.sroa.0.0.copyload.i.i124.i to i64
  %497 = shl nuw i64 %load_str_to_stack.unpack4.i121.i, 1
  %isneg.i.i.i127.i = icmp slt i64 %.sroa.2.0.copyload.i.i126.i, 0
  %498 = select i1 %isneg.i.i.i127.i, i64 %497, i64 %496
  %499 = inttoptr i64 %498 to ptr
  %get_rc_ptr.i128.i = getelementptr inbounds i64, ptr %499, i64 -1
  %500 = load i64, ptr %get_rc_ptr.i128.i, align 8
  %.not.i.i129.i = icmp eq i64 %500, 0
  br i1 %.not.i.i129.i, label %"#Attr_#dec_1.exit130.i", label %501

501:                                              ; preds = %modify_rc.i123.i
  %502 = atomicrmw sub ptr %get_rc_ptr.i128.i, i64 1 monotonic, align 8
  %503 = icmp eq i64 %502, 1
  br i1 %503, label %504, label %"#Attr_#dec_1.exit130.i"

504:                                              ; preds = %501
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i128.i, i32 8)
  br label %"#Attr_#dec_1.exit130.i"

"#Attr_#dec_1.exit130.i":                         ; preds = %504, %501, %modify_rc.i123.i, %"#Attr_#dec_1.exit119.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value9.i, ptr nonnull %result_value25.i, ptr nonnull %result_value26.i)
  %load_str_to_stack.elt3.i131.i = getelementptr inbounds %str.RocStr, ptr %result_value25.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i132.i = load i64, ptr %load_str_to_stack.elt3.i131.i, align 8
  %is_big_str.i133.i = icmp sgt i64 %load_str_to_stack.unpack4.i132.i, 0
  br i1 %is_big_str.i133.i, label %modify_rc.i134.i, label %"#Attr_#dec_1.exit141.i"

modify_rc.i134.i:                                 ; preds = %"#Attr_#dec_1.exit130.i"
  %.sroa.0.0.copyload.i.i135.i = load ptr, ptr %result_value25.i, align 8
  %.sroa.2.0..sroa_idx.i.i136.i = getelementptr inbounds i8, ptr %result_value25.i, i64 8
  %.sroa.2.0.copyload.i.i137.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i136.i, align 8
  %505 = ptrtoint ptr %.sroa.0.0.copyload.i.i135.i to i64
  %506 = shl nuw i64 %load_str_to_stack.unpack4.i132.i, 1
  %isneg.i.i.i138.i = icmp slt i64 %.sroa.2.0.copyload.i.i137.i, 0
  %507 = select i1 %isneg.i.i.i138.i, i64 %506, i64 %505
  %508 = inttoptr i64 %507 to ptr
  %get_rc_ptr.i139.i = getelementptr inbounds i64, ptr %508, i64 -1
  %509 = load i64, ptr %get_rc_ptr.i139.i, align 8
  %.not.i.i140.i = icmp eq i64 %509, 0
  br i1 %.not.i.i140.i, label %"#Attr_#dec_1.exit141.i", label %510

510:                                              ; preds = %modify_rc.i134.i
  %511 = atomicrmw sub ptr %get_rc_ptr.i139.i, i64 1 monotonic, align 8
  %512 = icmp eq i64 %511, 1
  br i1 %512, label %513, label %"#Attr_#dec_1.exit141.i"

513:                                              ; preds = %510
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i139.i, i32 8)
  br label %"#Attr_#dec_1.exit141.i"

"#Attr_#dec_1.exit141.i":                         ; preds = %513, %510, %modify_rc.i134.i, %"#Attr_#dec_1.exit130.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %const_str_store7.i, ptr nonnull %result_value26.i, ptr nonnull %result_value27.i)
  %load_str_to_stack.elt3.i142.i = getelementptr inbounds %str.RocStr, ptr %result_value26.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i143.i = load i64, ptr %load_str_to_stack.elt3.i142.i, align 8
  %is_big_str.i144.i = icmp sgt i64 %load_str_to_stack.unpack4.i143.i, 0
  br i1 %is_big_str.i144.i, label %modify_rc.i145.i, label %"#Attr_#dec_1.exit152.i"

modify_rc.i145.i:                                 ; preds = %"#Attr_#dec_1.exit141.i"
  %.sroa.0.0.copyload.i.i146.i = load ptr, ptr %result_value26.i, align 8
  %.sroa.2.0..sroa_idx.i.i147.i = getelementptr inbounds i8, ptr %result_value26.i, i64 8
  %.sroa.2.0.copyload.i.i148.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i147.i, align 8
  %514 = ptrtoint ptr %.sroa.0.0.copyload.i.i146.i to i64
  %515 = shl nuw i64 %load_str_to_stack.unpack4.i143.i, 1
  %isneg.i.i.i149.i = icmp slt i64 %.sroa.2.0.copyload.i.i148.i, 0
  %516 = select i1 %isneg.i.i.i149.i, i64 %515, i64 %514
  %517 = inttoptr i64 %516 to ptr
  %get_rc_ptr.i150.i = getelementptr inbounds i64, ptr %517, i64 -1
  %518 = load i64, ptr %get_rc_ptr.i150.i, align 8
  %.not.i.i151.i = icmp eq i64 %518, 0
  br i1 %.not.i.i151.i, label %"#Attr_#dec_1.exit152.i", label %519

519:                                              ; preds = %modify_rc.i145.i
  %520 = atomicrmw sub ptr %get_rc_ptr.i150.i, i64 1 monotonic, align 8
  %521 = icmp eq i64 %520, 1
  br i1 %521, label %522, label %"#Attr_#dec_1.exit152.i"

522:                                              ; preds = %519
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i150.i, i32 8)
  br label %"#Attr_#dec_1.exit152.i"

"#Attr_#dec_1.exit152.i":                         ; preds = %522, %519, %modify_rc.i145.i, %"#Attr_#dec_1.exit141.i"
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %result_value.i.i.i)
  call void @roc_fx_stdout_write(ptr nonnull %result_value.i.i.i, ptr nonnull %result_value27.i)
  %load_str_to_stack.elt3.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %result_value27.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i.i.i.i = load i64, ptr %load_str_to_stack.elt3.i.i.i.i, align 8
  %is_big_str.i.i.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i.i.i.i, 0
  br i1 %is_big_str.i.i.i.i, label %modify_rc.i.i.i.i, label %"Host_stdout_write!_ab6bf482529beb271842e34a3922217532b2f57565d72aea510e9edbdcef6.exit.i.i"

modify_rc.i.i.i.i:                                ; preds = %"#Attr_#dec_1.exit152.i"
  %.sroa.0.0.copyload.i.i.i.i.i = load ptr, ptr %result_value27.i, align 8
  %.sroa.2.0..sroa_idx.i.i.i.i.i = getelementptr inbounds i8, ptr %result_value27.i, i64 8
  %.sroa.2.0.copyload.i.i.i.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i.i.i.i, align 8
  %523 = ptrtoint ptr %.sroa.0.0.copyload.i.i.i.i.i to i64
  %524 = shl nuw i64 %load_str_to_stack.unpack4.i.i.i.i, 1
  %isneg.i.i.i.i.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i.i.i.i, 0
  %525 = select i1 %isneg.i.i.i.i.i.i, i64 %524, i64 %523
  %526 = inttoptr i64 %525 to ptr
  %get_rc_ptr.i.i.i.i = getelementptr inbounds i64, ptr %526, i64 -1
  %527 = load i64, ptr %get_rc_ptr.i.i.i.i, align 8
  %.not.i.i.i.i154.i = icmp eq i64 %527, 0
  br i1 %.not.i.i.i.i154.i, label %"Host_stdout_write!_ab6bf482529beb271842e34a3922217532b2f57565d72aea510e9edbdcef6.exit.i.i", label %528

528:                                              ; preds = %modify_rc.i.i.i.i
  %529 = atomicrmw sub ptr %get_rc_ptr.i.i.i.i, i64 1 monotonic, align 8
  %530 = icmp eq i64 %529, 1
  br i1 %530, label %531, label %"Host_stdout_write!_ab6bf482529beb271842e34a3922217532b2f57565d72aea510e9edbdcef6.exit.i.i"

531:                                              ; preds = %528
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i.i, i32 8)
  br label %"Host_stdout_write!_ab6bf482529beb271842e34a3922217532b2f57565d72aea510e9edbdcef6.exit.i.i"

"Host_stdout_write!_ab6bf482529beb271842e34a3922217532b2f57565d72aea510e9edbdcef6.exit.i.i": ; preds = %531, %528, %modify_rc.i.i.i.i, %"#Attr_#dec_1.exit152.i"
  %result_value.sroa.0.0.copyload.i.i = load ptr, ptr %result_value.i.i.i, align 8
  %result_value.sroa.2.0.result_value.i.sroa_idx.i.i = getelementptr inbounds i8, ptr %result_value.i.i.i, i64 8
  %result_value.sroa.2.0.copyload.i.i = load i64, ptr %result_value.sroa.2.0.result_value.i.sroa_idx.i.i, align 8
  %result_value.sroa.3.0.result_value.i.sroa_idx.i.i = getelementptr inbounds i8, ptr %result_value.i.i.i, i64 16
  %result_value.sroa.3.0.copyload.i.i = load i64, ptr %result_value.sroa.3.0.result_value.i.sroa_idx.i.i, align 8
  %result_value.sroa.4.0.result_value.i.sroa_idx.i.i = getelementptr inbounds i8, ptr %result_value.i.i.i, i64 24
  %result_value.sroa.4.0.copyload.i.i = load i8, ptr %result_value.sroa.4.0.result_value.i.sroa_idx.i.i, align 8
  %result_value.sroa.53.0.result_value.i.sroa_idx.i.i = getelementptr inbounds i8, ptr %result_value.i.i.i, i64 32
  %result_value.sroa.53.0.copyload.i.i = load i8, ptr %result_value.sroa.53.0.result_value.i.sroa_idx.i.i, align 8
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %result_value.i.i.i)
  %eq_u8.i.i.i = icmp eq i8 %result_value.sroa.53.0.copyload.i.i, 1
  br i1 %eq_u8.i.i.i, label %then_block, label %else_block.i.i.i107

else_block.i.i.i107:                              ; preds = %"Host_stdout_write!_ab6bf482529beb271842e34a3922217532b2f57565d72aea510e9edbdcef6.exit.i.i"
  switch i8 %result_value.sroa.4.0.copyload.i.i, label %default.i.i.i.i [
    i8 4, label %branch4.i.i.i.i
    i8 7, label %branch7.i.i.i.i
    i8 1, label %branch1.i.i.i.i
    i8 0, label %branch0.i.i.i.i
    i8 3, label %branch3.i.i.i.i
    i8 8, label %branch8.i.i.i.i
    i8 6, label %branch6.i.i.i.i
    i8 5, label %branch5.i.i.i.i
  ]

default.i.i.i.i:                                  ; preds = %else_block.i.i.i107
  %532 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i to i64
  br label %list.RocList.incref.exit.i.i28.i.i.i

branch4.i.i.i.i:                                  ; preds = %else_block.i.i.i107
  %is_big_str.i.i.i.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload.i.i, 0
  br i1 %is_big_str.i.i.i.i.i, label %modify_rc.i.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i

modify_rc.i.i.i.i.i:                              ; preds = %branch4.i.i.i.i
  %533 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i to i64
  %534 = shl nuw i64 %result_value.sroa.3.0.copyload.i.i, 1
  %isneg.i.i.i.i.i.i.i = icmp slt i64 %result_value.sroa.2.0.copyload.i.i, 0
  %535 = select i1 %isneg.i.i.i.i.i.i.i, i64 %534, i64 %533
  %536 = inttoptr i64 %535 to ptr
  %get_rc_ptr.i.i.i.i.i = getelementptr inbounds i64, ptr %536, i64 -1
  %537 = load i64, ptr %get_rc_ptr.i.i.i.i.i, align 8
  %.not.i.i.i.i.i153.i = icmp eq i64 %537, 0
  br i1 %.not.i.i.i.i.i153.i, label %list.RocList.incref.exit.i.i28.i.i.i, label %538

538:                                              ; preds = %modify_rc.i.i.i.i.i
  %539 = atomicrmw sub ptr %get_rc_ptr.i.i.i.i.i, i64 1 monotonic, align 8
  %540 = icmp eq i64 %539, 1
  br i1 %540, label %541, label %list.RocList.incref.exit.i.i28.i.i.i

541:                                              ; preds = %538
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i.i.i, i32 8)
  br label %list.RocList.incref.exit.i.i28.i.i.i

branch7.i.i.i.i:                                  ; preds = %else_block.i.i.i107
  %is_big_str.i74.i.i.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload.i.i, 0
  br i1 %is_big_str.i74.i.i.i.i, label %modify_rc.i75.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i

modify_rc.i75.i.i.i.i:                            ; preds = %branch7.i.i.i.i
  %542 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i to i64
  %543 = shl nuw i64 %result_value.sroa.3.0.copyload.i.i, 1
  %isneg.i.i.i79.i.i.i.i = icmp slt i64 %result_value.sroa.2.0.copyload.i.i, 0
  %544 = select i1 %isneg.i.i.i79.i.i.i.i, i64 %543, i64 %542
  %545 = inttoptr i64 %544 to ptr
  %get_rc_ptr.i80.i.i.i.i = getelementptr inbounds i64, ptr %545, i64 -1
  %546 = load i64, ptr %get_rc_ptr.i80.i.i.i.i, align 8
  %.not.i.i81.i.i.i.i = icmp eq i64 %546, 0
  br i1 %.not.i.i81.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i, label %547

547:                                              ; preds = %modify_rc.i75.i.i.i.i
  %548 = atomicrmw sub ptr %get_rc_ptr.i80.i.i.i.i, i64 1 monotonic, align 8
  %549 = icmp eq i64 %548, 1
  br i1 %549, label %550, label %list.RocList.incref.exit.i.i28.i.i.i

550:                                              ; preds = %547
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i80.i.i.i.i, i32 8)
  br label %list.RocList.incref.exit.i.i28.i.i.i

branch1.i.i.i.i:                                  ; preds = %else_block.i.i.i107
  %is_big_str.i85.i.i.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload.i.i, 0
  br i1 %is_big_str.i85.i.i.i.i, label %modify_rc.i86.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i

modify_rc.i86.i.i.i.i:                            ; preds = %branch1.i.i.i.i
  %551 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i to i64
  %552 = shl nuw i64 %result_value.sroa.3.0.copyload.i.i, 1
  %isneg.i.i.i90.i.i.i.i = icmp slt i64 %result_value.sroa.2.0.copyload.i.i, 0
  %553 = select i1 %isneg.i.i.i90.i.i.i.i, i64 %552, i64 %551
  %554 = inttoptr i64 %553 to ptr
  %get_rc_ptr.i91.i.i.i.i = getelementptr inbounds i64, ptr %554, i64 -1
  %555 = load i64, ptr %get_rc_ptr.i91.i.i.i.i, align 8
  %.not.i.i92.i.i.i.i = icmp eq i64 %555, 0
  br i1 %.not.i.i92.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i, label %556

556:                                              ; preds = %modify_rc.i86.i.i.i.i
  %557 = atomicrmw sub ptr %get_rc_ptr.i91.i.i.i.i, i64 1 monotonic, align 8
  %558 = icmp eq i64 %557, 1
  br i1 %558, label %559, label %list.RocList.incref.exit.i.i28.i.i.i

559:                                              ; preds = %556
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i91.i.i.i.i, i32 8)
  br label %list.RocList.incref.exit.i.i28.i.i.i

branch0.i.i.i.i:                                  ; preds = %else_block.i.i.i107
  %is_big_str.i96.i.i.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload.i.i, 0
  br i1 %is_big_str.i96.i.i.i.i, label %modify_rc.i97.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i

modify_rc.i97.i.i.i.i:                            ; preds = %branch0.i.i.i.i
  %560 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i to i64
  %561 = shl nuw i64 %result_value.sroa.3.0.copyload.i.i, 1
  %isneg.i.i.i101.i.i.i.i = icmp slt i64 %result_value.sroa.2.0.copyload.i.i, 0
  %562 = select i1 %isneg.i.i.i101.i.i.i.i, i64 %561, i64 %560
  %563 = inttoptr i64 %562 to ptr
  %get_rc_ptr.i102.i.i.i.i = getelementptr inbounds i64, ptr %563, i64 -1
  %564 = load i64, ptr %get_rc_ptr.i102.i.i.i.i, align 8
  %.not.i.i103.i.i.i.i = icmp eq i64 %564, 0
  br i1 %.not.i.i103.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i, label %565

565:                                              ; preds = %modify_rc.i97.i.i.i.i
  %566 = atomicrmw sub ptr %get_rc_ptr.i102.i.i.i.i, i64 1 monotonic, align 8
  %567 = icmp eq i64 %566, 1
  br i1 %567, label %568, label %list.RocList.incref.exit.i.i28.i.i.i

568:                                              ; preds = %565
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i102.i.i.i.i, i32 8)
  br label %list.RocList.incref.exit.i.i28.i.i.i

branch3.i.i.i.i:                                  ; preds = %else_block.i.i.i107
  %is_big_str.i107.i.i.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload.i.i, 0
  br i1 %is_big_str.i107.i.i.i.i, label %modify_rc.i108.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i

modify_rc.i108.i.i.i.i:                           ; preds = %branch3.i.i.i.i
  %569 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i to i64
  %570 = shl nuw i64 %result_value.sroa.3.0.copyload.i.i, 1
  %isneg.i.i.i112.i.i.i.i = icmp slt i64 %result_value.sroa.2.0.copyload.i.i, 0
  %571 = select i1 %isneg.i.i.i112.i.i.i.i, i64 %570, i64 %569
  %572 = inttoptr i64 %571 to ptr
  %get_rc_ptr.i113.i.i.i.i = getelementptr inbounds i64, ptr %572, i64 -1
  %573 = load i64, ptr %get_rc_ptr.i113.i.i.i.i, align 8
  %.not.i.i114.i.i.i.i = icmp eq i64 %573, 0
  br i1 %.not.i.i114.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i, label %574

574:                                              ; preds = %modify_rc.i108.i.i.i.i
  %575 = atomicrmw sub ptr %get_rc_ptr.i113.i.i.i.i, i64 1 monotonic, align 8
  %576 = icmp eq i64 %575, 1
  br i1 %576, label %577, label %list.RocList.incref.exit.i.i28.i.i.i

577:                                              ; preds = %574
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i113.i.i.i.i, i32 8)
  br label %list.RocList.incref.exit.i.i28.i.i.i

branch8.i.i.i.i:                                  ; preds = %else_block.i.i.i107
  %is_big_str.i118.i.i.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload.i.i, 0
  br i1 %is_big_str.i118.i.i.i.i, label %modify_rc.i119.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i

modify_rc.i119.i.i.i.i:                           ; preds = %branch8.i.i.i.i
  %578 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i to i64
  %579 = shl nuw i64 %result_value.sroa.3.0.copyload.i.i, 1
  %isneg.i.i.i123.i.i.i.i = icmp slt i64 %result_value.sroa.2.0.copyload.i.i, 0
  %580 = select i1 %isneg.i.i.i123.i.i.i.i, i64 %579, i64 %578
  %581 = inttoptr i64 %580 to ptr
  %get_rc_ptr.i124.i.i.i.i = getelementptr inbounds i64, ptr %581, i64 -1
  %582 = load i64, ptr %get_rc_ptr.i124.i.i.i.i, align 8
  %.not.i.i125.i.i.i.i = icmp eq i64 %582, 0
  br i1 %.not.i.i125.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i, label %583

583:                                              ; preds = %modify_rc.i119.i.i.i.i
  %584 = atomicrmw sub ptr %get_rc_ptr.i124.i.i.i.i, i64 1 monotonic, align 8
  %585 = icmp eq i64 %584, 1
  br i1 %585, label %586, label %list.RocList.incref.exit.i.i28.i.i.i

586:                                              ; preds = %583
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i124.i.i.i.i, i32 8)
  br label %list.RocList.incref.exit.i.i28.i.i.i

branch6.i.i.i.i:                                  ; preds = %else_block.i.i.i107
  %is_big_str.i129.i.i.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload.i.i, 0
  br i1 %is_big_str.i129.i.i.i.i, label %modify_rc.i130.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i

modify_rc.i130.i.i.i.i:                           ; preds = %branch6.i.i.i.i
  %587 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i to i64
  %588 = shl nuw i64 %result_value.sroa.3.0.copyload.i.i, 1
  %isneg.i.i.i134.i.i.i.i = icmp slt i64 %result_value.sroa.2.0.copyload.i.i, 0
  %589 = select i1 %isneg.i.i.i134.i.i.i.i, i64 %588, i64 %587
  %590 = inttoptr i64 %589 to ptr
  %get_rc_ptr.i135.i.i.i.i = getelementptr inbounds i64, ptr %590, i64 -1
  %591 = load i64, ptr %get_rc_ptr.i135.i.i.i.i, align 8
  %.not.i.i136.i.i.i.i = icmp eq i64 %591, 0
  br i1 %.not.i.i136.i.i.i.i, label %list.RocList.incref.exit.i.i28.i.i.i, label %592

592:                                              ; preds = %modify_rc.i130.i.i.i.i
  %593 = atomicrmw sub ptr %get_rc_ptr.i135.i.i.i.i, i64 1 monotonic, align 8
  %594 = icmp eq i64 %593, 1
  br i1 %594, label %595, label %list.RocList.incref.exit.i.i28.i.i.i

595:                                              ; preds = %592
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i135.i.i.i.i, i32 8)
  br label %list.RocList.incref.exit.i.i28.i.i.i

branch5.i.i.i.i:                                  ; preds = %else_block.i.i.i107
  %596 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i to i64
  br label %list.RocList.incref.exit.i.i28.i.i.i

common.ret:                                       ; preds = %870, %867, %modify_rc.i.i.i344, %tag_id_modify.i.i340, %"#Attr_#dec_1.exit332", %then_block
  %common.ret.op = phi i32 [ 0, %then_block ], [ 1, %"#Attr_#dec_1.exit332" ], [ 1, %tag_id_modify.i.i340 ], [ 1, %modify_rc.i.i.i344 ], [ 1, %867 ], [ 1, %870 ]
  ret i32 %common.ret.op

then_block:                                       ; preds = %"Host_stdout_write!_ab6bf482529beb271842e34a3922217532b2f57565d72aea510e9edbdcef6.exit.i.i"
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value27.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value26.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value25.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value24.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value23.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value22.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value21.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value20.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value19.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value18.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store17.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value16.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store15.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value14.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store13.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store12.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value11.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store10.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value9.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store7.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i)
  call fastcc void @"#Attr_#dec_4"(%list.RocList %joinpointarg1.lcssa.i.i.i101)
  br label %common.ret

list.RocList.incref.exit.i.i28.i.i.i:             ; preds = %default.i.i.i.i, %branch5.i.i.i.i, %modify_rc.i130.i.i.i.i, %595, %592, %branch6.i.i.i.i, %modify_rc.i119.i.i.i.i, %586, %583, %branch8.i.i.i.i, %modify_rc.i108.i.i.i.i, %577, %574, %branch3.i.i.i.i, %modify_rc.i97.i.i.i.i, %568, %565, %branch0.i.i.i.i, %modify_rc.i86.i.i.i.i, %559, %556, %branch1.i.i.i.i, %modify_rc.i75.i.i.i.i, %550, %547, %branch7.i.i.i.i, %modify_rc.i.i.i.i.i, %541, %538, %branch4.i.i.i.i
  %result_value1.sroa.0.0.i.i.ph = phi i64 [ undef, %branch4.i.i.i.i ], [ undef, %538 ], [ undef, %541 ], [ undef, %modify_rc.i.i.i.i.i ], [ undef, %branch7.i.i.i.i ], [ undef, %547 ], [ undef, %550 ], [ undef, %modify_rc.i75.i.i.i.i ], [ undef, %branch1.i.i.i.i ], [ undef, %556 ], [ undef, %559 ], [ undef, %modify_rc.i86.i.i.i.i ], [ undef, %branch0.i.i.i.i ], [ undef, %565 ], [ undef, %568 ], [ undef, %modify_rc.i97.i.i.i.i ], [ undef, %branch3.i.i.i.i ], [ undef, %574 ], [ undef, %577 ], [ undef, %modify_rc.i108.i.i.i.i ], [ undef, %branch8.i.i.i.i ], [ undef, %583 ], [ undef, %586 ], [ undef, %modify_rc.i119.i.i.i.i ], [ undef, %branch6.i.i.i.i ], [ undef, %592 ], [ undef, %595 ], [ undef, %modify_rc.i130.i.i.i.i ], [ %596, %branch5.i.i.i.i ], [ %532, %default.i.i.i.i ]
  %result_value1.sroa.4.0.i.i.ph = phi i8 [ 3, %branch4.i.i.i.i ], [ 3, %538 ], [ 3, %541 ], [ 3, %modify_rc.i.i.i.i.i ], [ 6, %branch7.i.i.i.i ], [ 6, %547 ], [ 6, %550 ], [ 6, %modify_rc.i75.i.i.i.i ], [ 1, %branch1.i.i.i.i ], [ 1, %556 ], [ 1, %559 ], [ 1, %modify_rc.i86.i.i.i.i ], [ 0, %branch0.i.i.i.i ], [ 0, %565 ], [ 0, %568 ], [ 0, %modify_rc.i97.i.i.i.i ], [ 2, %branch3.i.i.i.i ], [ 2, %574 ], [ 2, %577 ], [ 2, %modify_rc.i108.i.i.i.i ], [ 7, %branch8.i.i.i.i ], [ 7, %583 ], [ 7, %586 ], [ 7, %modify_rc.i119.i.i.i.i ], [ 5, %branch6.i.i.i.i ], [ 5, %592 ], [ 5, %595 ], [ 5, %modify_rc.i130.i.i.i.i ], [ 4, %branch5.i.i.i.i ], [ 4, %default.i.i.i.i ]
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value27.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value26.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value25.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value24.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value23.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value22.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value21.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value20.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value19.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value18.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store17.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value16.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store15.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value14.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store13.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store12.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value11.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store10.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value9.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store7.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i)
  call fastcc void @"#Attr_#dec_4"(%list.RocList %joinpointarg1.lcssa.i.i.i101)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value1.i.i)
  %const_str_store.sroa.3.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %result_value1.i.i, i64 16
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %result_value1.i.i, i8 0, i64 16, i1 false)
  store i64 -9223372036854775808, ptr %const_str_store.sroa.3.0..sroa_idx.i.i.i, align 8
  %597 = call ptr @roc_alloc(i64 56, i32 8)
  %598 = icmp ne ptr %597, null
  call void @llvm.assume(i1 %598)
  %599 = getelementptr inbounds i8, ptr %597, i64 16
  %600 = getelementptr inbounds i8, ptr %597, i64 8
  store i64 1, ptr %600, align 8
  store i64 %result_value1.sroa.0.0.i.i.ph, ptr %599, align 8
  %result_value14.sroa.2.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %597, i64 24
  store i64 %result_value.sroa.2.0.copyload.i.i, ptr %result_value14.sroa.2.0..sroa_idx.i.i.i, align 8
  %result_value14.sroa.3.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %597, i64 32
  store i64 %result_value.sroa.3.0.copyload.i.i, ptr %result_value14.sroa.3.0..sroa_idx.i.i.i, align 8
  %result_value14.sroa.4.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %597, i64 40
  store i8 %result_value1.sroa.4.0.i.i.ph, ptr %result_value14.sroa.4.0..sroa_idx.i.i.i, align 8
  %result_value14.sroa.572.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %597, i64 48
  store i8 0, ptr %result_value14.sroa.572.0..sroa_idx.i.i.i, align 8
  %insert_record_field17.i.i.i = insertvalue %list.RocList zeroinitializer, ptr %599, 0
  %insert_record_field18.i.i.i = insertvalue %list.RocList %insert_record_field17.i.i.i, i64 1, 1
  %insert_record_field19.i.i.i = insertvalue %list.RocList %insert_record_field18.i.i.i, i64 1, 2
  %601 = ptrtoint ptr %599 to i64
  store i64 1, ptr %597, align 8
  %602 = and i64 %601, -8
  %603 = add i64 %602, -8
  %604 = inttoptr i64 %603 to ptr
  %605 = load i64, ptr %604, align 8
  %.not.i.i.i.i29.i.i.i = icmp eq i64 %605, 0
  br i1 %.not.i.i.i.i29.i.i.i, label %list.RocList.refcount.exit.i.i.i.i.i.i.i.i, label %list.RocList.refcount.exit.i.i.i.i.i.sink.split.i.i.i

list.RocList.refcount.exit.i.i.i.i.i.sink.split.i.i.i: ; preds = %list.RocList.incref.exit.i.i28.i.i.i
  %606 = atomicrmw add ptr %604, i64 1 monotonic, align 8
  br label %list.RocList.refcount.exit.i.i.i.i.i.i.i.i

list.RocList.refcount.exit.i.i.i.i.i.i.i.i:       ; preds = %list.RocList.refcount.exit.i.i.i.i.i.sink.split.i.i.i, %list.RocList.incref.exit.i.i28.i.i.i
  call fastcc void @"#Attr_#dec_12"(%list.RocList %insert_record_field19.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %joinpoint_arg_alloca.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store7.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value5.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %load_element4.i.i.i.i.i)
  store ptr inttoptr (i64 8234115541445669971 to ptr), ptr %load_element4.i.i.i.i.i, align 8
  %joinpoint_arg_alloca.sroa.0.sroa.4.0.load_element4.i.i.sroa_idx.i.i.i = getelementptr inbounds i8, ptr %load_element4.i.i.i.i.i, i64 8
  store <2 x i64> <i64 114, i64 -8574853690513424384>, ptr %joinpoint_arg_alloca.sroa.0.sroa.4.0.load_element4.i.i.sroa_idx.i.i.i, align 8
  store ptr inttoptr (i64 40 to ptr), ptr %const_str_store.i.i.i.i.i, align 8
  %.fca.1.gep.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store.i.i.i.i.i, i64 0, i32 1
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value1.i.i, ptr nonnull %const_str_store.i.i.i.i.i, ptr nonnull %result_value.i.i.i.i.i)
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value.i.i.i.i.i, ptr nonnull %load_element4.i.i.i.i.i, ptr nonnull %result_value5.i.i.i.i.i)
  %607 = load i64, ptr %600, align 8
  %.not.i.i.i151 = icmp eq i64 %607, 1
  br i1 %.not.i.i.i151, label %.thread.i.i.i.i.i.i.i.i, label %list.RocList.incref.exit.i.i.i.i.i.i.i.i

.thread.i.i.i.i.i.i.i.i:                          ; preds = %list.RocList.refcount.exit.i.i.i.i.i.i.i.i
  store i64 1, ptr %597, align 8
  br label %list.RocList.incref.exit.i.i.i.i.i.i.i.i

list.RocList.incref.exit.i.i.i.i.i.i.i.i:         ; preds = %.thread.i.i.i.i.i.i.i.i, %list.RocList.refcount.exit.i.i.i.i.i.i.i.i
  %608 = load i64, ptr %604, align 8
  %.not.i.i.i.i.i.i.i.i.i.i152 = icmp eq i64 %608, 0
  br i1 %.not.i.i.i.i.i.i.i.i.i.i152, label %then_block.lr.ph.i.i.i.i.i.i.i, label %609

609:                                              ; preds = %list.RocList.incref.exit.i.i.i.i.i.i.i.i
  %610 = atomicrmw add ptr %604, i64 1 monotonic, align 8
  br label %then_block.lr.ph.i.i.i.i.i.i.i

then_block.lr.ph.i.i.i.i.i.i.i:                   ; preds = %609, %list.RocList.incref.exit.i.i.i.i.i.i.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %joinpoint_arg_alloca.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %result_value5.i.i.i.i.i, i64 24, i1 false)
  %.fca.1.gep.i17.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store.i.i.i.i.i.i.i.i, i64 0, i32 1
  %result_value.sroa.6.0.load_element.i.i.i.i.sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %result_value.sroa.9.0.load_element.i.i.i.i.sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i.i.i.i.i.i.i.i.i, i64 16
  %.fca.1.gep.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %.fca.1.gep6.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store3.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %load_element.sroa.9.0.load_element13.sroa_idx.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %load_element13.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %load_element.sroa.16.0.load_element13.sroa_idx.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %load_element13.i.i.i.i.i.i.i.i.i.i.i, i64 16
  %.fca.1.gep.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %.fca.1.gep13.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store1.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %.fca.1.gep16.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %.fca.1.gep19.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store3.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %.fca.1.gep22.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store5.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %.fca.1.gep25.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store6.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %.fca.1.gep28.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store8.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %.fca.1.gep31.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store9.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %joinpoint_arg_alloca.sroa.0.sroa.10.0.load_element4.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %load_element4.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %.fca.1.gep.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %load_str_to_stack.elt3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 2
  %.sroa.2.0..sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %.fca.1.gep.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %.fca.1.gep.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %.fca.1.gep5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %.fca.1.gep10.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store7.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 0, i32 1
  %joinpoint_arg_alloca.sroa.0.sroa.10.0.load_element.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %load_element.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 8
  %.sroa.2.0..sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %7, i64 8
  %.sroa.3.0..sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %7, i64 16
  %611 = ptrtoint ptr %7 to i64
  %612 = getelementptr inbounds %io.Writer, ptr %5, i64 0, i32 1
  %613 = getelementptr inbounds i8, ptr %3, i64 8
  %614 = getelementptr inbounds i8, ptr %3, i64 16
  %result_value.sroa.0.0.copyload.i.i.i.i.i.i.i = load ptr, ptr %599, align 8
  %result_value.sroa.6.0.list_get_element.i.sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %597, i64 24
  %result_value.sroa.6.0.copyload.i.i.i.i.i.i.i = load i64, ptr %result_value.sroa.6.0.list_get_element.i.sroa_idx.i.i.i.i.i.i.i, align 8
  %result_value.sroa.9.0.list_get_element.i.sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %597, i64 32
  %result_value.sroa.9.0.copyload.i.i.i.i.i.i.i = load i64, ptr %result_value.sroa.9.0.list_get_element.i.sroa_idx.i.i.i.i.i.i.i, align 8
  %result_value.sroa.12.0.list_get_element.i.sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %597, i64 40
  %result_value.sroa.12.0.copyload.i.i.i.i.i.i.i = load i8, ptr %result_value.sroa.12.0.list_get_element.i.sroa_idx.i.i.i.i.i.i.i, align 8
  %result_value.sroa.1412.0.list_get_element.i.sroa_idx.i.i.i.i.i.i.i = getelementptr inbounds i8, ptr %597, i64 48
  %result_value.sroa.1412.0.copyload.i.i.i.i.i.i.i = load i8, ptr %result_value.sroa.1412.0.list_get_element.i.sroa_idx.i.i.i.i.i.i.i, align 8
  switch i8 %result_value.sroa.1412.0.copyload.i.i.i.i.i.i.i, label %"#Attr_#inc_13.exit.i.i.i.i.i.i.i" [
    i8 0, label %tag_id_modify.i.i.i.i.i.i.i.i
    i8 1, label %tag_id_modify1.i.i.i.i.i.i.i.i
  ]

modify_rc_union_merge.sink.split.i.i.i.i.i.i.i.i: ; preds = %modify_rc.i.i.i.i.i.i.i.i.i, %modify_rc.i.i.i.i.i.i.i.i.i.i
  %get_rc_ptr.i.sink.i.i.i.i.i.i.i.i = phi ptr [ %get_rc_ptr.i.i.i.i.i.i.i.i.i.i, %modify_rc.i.i.i.i.i.i.i.i.i.i ], [ %get_rc_ptr.i.i.i.i.i.i.i.i.i, %modify_rc.i.i.i.i.i.i.i.i.i ]
  %615 = atomicrmw add ptr %get_rc_ptr.i.sink.i.i.i.i.i.i.i.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_13.exit.i.i.i.i.i.i.i"

tag_id_modify.i.i.i.i.i.i.i.i:                    ; preds = %then_block.lr.ph.i.i.i.i.i.i.i
  %cond.i.i.i.i.i.i.i.i.i = icmp eq i8 %result_value.sroa.12.0.copyload.i.i.i.i.i.i.i, 4
  %is_big_str.i.i.i.i.i.i.i.i.i.i = icmp sgt i64 %result_value.sroa.9.0.copyload.i.i.i.i.i.i.i, 0
  %or.cond.i.i.i.i.i.i.i153 = select i1 %cond.i.i.i.i.i.i.i.i.i, i1 %is_big_str.i.i.i.i.i.i.i.i.i.i, i1 false
  br i1 %or.cond.i.i.i.i.i.i.i153, label %modify_rc.i.i.i.i.i.i.i.i.i.i, label %"#Attr_#inc_13.exit.thread.i.i.i.i.i.i.i"

modify_rc.i.i.i.i.i.i.i.i.i.i:                    ; preds = %tag_id_modify.i.i.i.i.i.i.i.i
  %616 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i.i.i.i.i.i to i64
  %617 = shl nuw i64 %result_value.sroa.9.0.copyload.i.i.i.i.i.i.i, 1
  %isneg.i.i.i.i.i.i.i.i.i.i.i.i = icmp slt i64 %result_value.sroa.6.0.copyload.i.i.i.i.i.i.i, 0
  %618 = select i1 %isneg.i.i.i.i.i.i.i.i.i.i.i.i, i64 %617, i64 %616
  %619 = inttoptr i64 %618 to ptr
  %get_rc_ptr.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i64, ptr %619, i64 -1
  %620 = load i64, ptr %get_rc_ptr.i.i.i.i.i.i.i.i.i.i, align 8
  %.not.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %620, 0
  br i1 %.not.i.i.i.i.i.i.i.i.i.i.i, label %"#Attr_#inc_13.exit.thread.i.i.i.i.i.i.i", label %modify_rc_union_merge.sink.split.i.i.i.i.i.i.i.i

tag_id_modify1.i.i.i.i.i.i.i.i:                   ; preds = %then_block.lr.ph.i.i.i.i.i.i.i
  %is_big_str.i.i.i.i.i.i.i.i.i = icmp sgt i64 %result_value.sroa.9.0.copyload.i.i.i.i.i.i.i, 0
  br i1 %is_big_str.i.i.i.i.i.i.i.i.i, label %modify_rc.i.i.i.i.i.i.i.i.i, label %"#Attr_#inc_13.exit.thread16.i.i.i.i.i.i.i"

modify_rc.i.i.i.i.i.i.i.i.i:                      ; preds = %tag_id_modify1.i.i.i.i.i.i.i.i
  %621 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i.i.i.i.i.i to i64
  %622 = shl nuw i64 %result_value.sroa.9.0.copyload.i.i.i.i.i.i.i, 1
  %isneg.i.i.i.i.i.i.i.i.i.i.i = icmp slt i64 %result_value.sroa.6.0.copyload.i.i.i.i.i.i.i, 0
  %623 = select i1 %isneg.i.i.i.i.i.i.i.i.i.i.i, i64 %622, i64 %621
  %624 = inttoptr i64 %623 to ptr
  %get_rc_ptr.i.i.i.i.i.i.i.i.i = getelementptr inbounds i64, ptr %624, i64 -1
  %625 = load i64, ptr %get_rc_ptr.i.i.i.i.i.i.i.i.i, align 8
  %.not.i.i.i7.i.i.i.i.i.i.i = icmp eq i64 %625, 0
  br i1 %.not.i.i.i7.i.i.i.i.i.i.i, label %"#Attr_#inc_13.exit.thread16.i.i.i.i.i.i.i", label %modify_rc_union_merge.sink.split.i.i.i.i.i.i.i.i

"#Attr_#inc_13.exit.thread.i.i.i.i.i.i.i":        ; preds = %modify_rc.i.i.i.i.i.i.i.i.i.i, %tag_id_modify.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 32 to ptr), ptr %const_str_store.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep.i17.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %joinpoint_arg_alloca.i.i.i.i.i.i.i, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i, ptr nonnull %result_value.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value2.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i)
  br label %branch0.i.i.i.i.i.i.i.i.i.i

"#Attr_#inc_13.exit.thread16.i.i.i.i.i.i.i":      ; preds = %modify_rc.i.i.i.i.i.i.i.i.i, %tag_id_modify1.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 32 to ptr), ptr %const_str_store.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep.i17.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %joinpoint_arg_alloca.i.i.i.i.i.i.i, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i, ptr nonnull %result_value.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value2.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i)
  br label %branch1.i.i.i.i.i.i.i.i.i.i

"#Attr_#inc_13.exit.i.i.i.i.i.i.i":               ; preds = %modify_rc_union_merge.sink.split.i.i.i.i.i.i.i.i, %then_block.lr.ph.i.i.i.i.i.i.i
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 32 to ptr), ptr %const_str_store.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep.i17.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %joinpoint_arg_alloca.i.i.i.i.i.i.i, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i, ptr nonnull %result_value.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value2.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i)
  switch i8 %result_value.sroa.1412.0.copyload.i.i.i.i.i.i.i, label %default.i.i.i.i.i.i.i.i.i.i [
    i8 0, label %branch0.i.i.i.i.i.i.i.i.i.i
    i8 1, label %branch1.i.i.i.i.i.i.i.i.i.i
  ]

default.i.i.i.i.i.i.i.i.i.i:                      ; preds = %"#Attr_#inc_13.exit.i.i.i.i.i.i.i"
  %626 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i.i.i.i.i.i to i64
  %result_value.sroa.0.0.extract.trunc.i.i.i.i.i.i.i = trunc i64 %626 to i32
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3)
  call void @llvm.lifetime.start.p0(i64 11, ptr nonnull %8)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5), !noalias !115
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6), !noalias !115
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %7), !noalias !115
  store ptr %8, ptr %7, align 8, !noalias !118
  store <2 x i64> <i64 11, i64 0>, ptr %.sroa.2.0..sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !118
  store i64 %611, ptr %6, align 8, !noalias !118
  store ptr %6, ptr %5, align 8, !noalias !118
  store ptr @"io.GenericWriter(*io.fixed_buffer_stream.FixedBufferStream([]u8),error{NoSpaceLeft},(function 'write')).typeErasedWriteFn", ptr %612, align 8, !noalias !118
  call void @llvm.lifetime.start.p0(i64 33, ptr nonnull %4), !noalias !118
  %627 = call i32 @llvm.abs.i32(i32 %result_value.sroa.0.0.extract.trunc.i.i.i.i.i.i.i, i1 false)
  %628 = icmp ugt i32 %627, 99
  br i1 %628, label %.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %._crit_edge.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

629:                                              ; preds = %646, %641
  %.sroa.09.0.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi i64 [ %642, %641 ], [ %647, %646 ]
  %630 = icmp slt i32 %result_value.sroa.0.0.extract.trunc.i.i.i.i.i.i.i, 0
  br i1 %630, label %653, label %fmt.format__anon_8070.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i

.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:     ; preds = %default.i.i.i.i.i.i.i.i.i.i, %.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %.sroa.0.03.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi i32 [ %633, %.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %627, %default.i.i.i.i.i.i.i.i.i.i ]
  %.sroa.09.12.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi i64 [ %631, %.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ 33, %default.i.i.i.i.i.i.i.i.i.i ]
  %631 = add i64 %.sroa.09.12.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, -2
  %632 = getelementptr inbounds i8, ptr %4, i64 %631
  %.sroa.0.03.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.frozen = freeze i32 %.sroa.0.03.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %633 = udiv i32 %.sroa.0.03.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.frozen, 100
  %634 = mul i32 %633, 100
  %.decomposed = sub i32 %.sroa.0.03.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.frozen, %634
  %635 = shl nuw nsw i32 %.decomposed, 1
  %636 = zext nneg i32 %635 to i64
  %637 = getelementptr inbounds i8, ptr @__anon_21180, i64 %636
  %638 = load i16, ptr %637, align 1, !noalias !121
  store i16 %638, ptr %632, align 1, !noalias !118
  %639 = icmp ugt i32 %.sroa.0.03.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 9999
  br i1 %639, label %.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %._crit_edge.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

._crit_edge.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %default.i.i.i.i.i.i.i.i.i.i
  %.sroa.09.1.lcssa.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi i64 [ 33, %default.i.i.i.i.i.i.i.i.i.i ], [ %631, %.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %.sroa.0.0.lcssa.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi i32 [ %627, %default.i.i.i.i.i.i.i.i.i.i ], [ %633, %.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %640 = icmp ult i32 %.sroa.0.0.lcssa.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 10
  br i1 %640, label %641, label %646

641:                                              ; preds = %._crit_edge.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %642 = add i64 %.sroa.09.1.lcssa.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, -1
  %643 = getelementptr inbounds [33 x i8], ptr %4, i64 0, i64 %642
  %644 = trunc i32 %.sroa.0.0.lcssa.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i to i8
  %645 = or disjoint i8 %644, 48
  store i8 %645, ptr %643, align 1, !noalias !118
  br label %629

646:                                              ; preds = %._crit_edge.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %647 = add i64 %.sroa.09.1.lcssa.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, -2
  %648 = getelementptr inbounds i8, ptr %4, i64 %647
  %649 = shl nuw nsw i32 %.sroa.0.0.lcssa.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 1
  %650 = zext nneg i32 %649 to i64
  %651 = getelementptr inbounds i8, ptr @__anon_21180, i64 %650
  %652 = load i16, ptr %651, align 1, !noalias !124
  store i16 %652, ptr %648, align 1, !noalias !118
  br label %629

653:                                              ; preds = %629
  %654 = add i64 %.sroa.09.0.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, -1
  %655 = getelementptr inbounds [33 x i8], ptr %4, i64 0, i64 %654
  store i8 45, ptr %655, align 1, !noalias !118
  br label %fmt.format__anon_8070.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i

fmt.format__anon_8070.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %653, %629
  %.sroa.09.3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi i64 [ %654, %653 ], [ %.sroa.09.0.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %629 ]
  %656 = getelementptr inbounds i8, ptr %4, i64 %.sroa.09.3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %657 = sub nuw i64 33, %.sroa.09.3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %658 = call fastcc i16 @fmt.formatBuf__anon_8817(ptr nonnull readonly align 1 %656, i64 %657, ptr nonnull readonly align 8 @6, ptr nonnull readonly align 8 %5), !noalias !118
  call void @llvm.lifetime.end.p0(i64 33, ptr nonnull %4), !noalias !118
  %659 = icmp eq i16 %658, 0
  call void @llvm.assume(i1 %659)
  %.val3.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %.sroa.3.0..sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8, !noalias !118
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5), !noalias !115
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6), !noalias !115
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %7), !noalias !115
  %660 = icmp ugt i64 %.val3.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 23
  br i1 %660, label %str.RocStr.allocate.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %str.RocStr.allocate.exit.thread.i.i.i.i.i.i.i.i.i.i.i.i.i.i

str.RocStr.allocate.exit.thread.i.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %fmt.format__anon_8070.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %.sroa.3.23.insert.ext.i.i.i.i.i.i.i.i.i.i.i.i.i.i = shl nuw nsw i64 %.val3.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 56
  %.sroa.3.23.insert.shift.i.i.i.i.i.i.i.i.i.i.i.i.i.i = or disjoint i64 %.sroa.3.23.insert.ext.i.i.i.i.i.i.i.i.i.i.i.i.i.i, -9223372036854775808
  br label %Num_to_str_4e123451c288c52798d3df0fc84811d2d957f324242982575c70dfd6d338df.exit.i.i.i.i.i.i.i.i.i.i.i

str.RocStr.allocate.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %fmt.format__anon_8070.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %661 = add nuw i64 %.val3.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 8
  %662 = call ptr @roc_alloc(i64 %661, i32 8), !noalias !127
  %663 = icmp ne ptr %662, null
  call void @llvm.assume(i1 %663)
  %664 = getelementptr inbounds i8, ptr %662, i64 8
  store i64 1, ptr %662, align 8, !noalias !127
  %665 = icmp slt i64 %.val3.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  %spec.select.i.i.i.i.i.i.i.i.i.i.i.i.i.i = select i1 %665, ptr %3, ptr %664
  br label %Num_to_str_4e123451c288c52798d3df0fc84811d2d957f324242982575c70dfd6d338df.exit.i.i.i.i.i.i.i.i.i.i.i

Num_to_str_4e123451c288c52798d3df0fc84811d2d957f324242982575c70dfd6d338df.exit.i.i.i.i.i.i.i.i.i.i.i: ; preds = %str.RocStr.allocate.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %str.RocStr.allocate.exit.thread.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %.sink7.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi ptr [ %664, %str.RocStr.allocate.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ null, %str.RocStr.allocate.exit.thread.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %.sink.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi i64 [ %.val3.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %str.RocStr.allocate.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ 0, %str.RocStr.allocate.exit.thread.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %.sroa.3.23.insert.shift.sink.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi i64 [ %.val3.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %str.RocStr.allocate.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %.sroa.3.23.insert.shift.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %str.RocStr.allocate.exit.thread.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  %common.ret.op.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = phi ptr [ %spec.select.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %str.RocStr.allocate.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i ], [ %3, %str.RocStr.allocate.exit.thread.i.i.i.i.i.i.i.i.i.i.i.i.i.i ]
  store ptr %.sink7.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr %3, align 8
  store i64 %.sink.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr %613, align 8
  store i64 %.sroa.3.23.insert.shift.sink.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr %614, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %common.ret.op.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull align 1 %8, i64 %.val3.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i1 false)
  call void @llvm.lifetime.end.p0(i64 11, ptr nonnull %8)
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value.i.i.i.i.i.i.i.i, ptr nonnull %3, ptr nonnull %result_value2.i.i.i.i.i.i.i.i.i.i.i)
  %load_str_to_stack.unpack4.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %614, align 8
  %is_big_str.i.i.i.i.i.i.i.i.i.i.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %is_big_str.i.i.i.i.i.i.i.i.i.i.i.i, label %modify_rc.i.i.i.i.i.i.i.i.i.i.i.i, label %Inspect_267_ca8af06be175497c1986028dec9ff9bde775267ac5c92d6992ca796b5e797c.exit.i.i.i.i.i.i.i.i.i.i

modify_rc.i.i.i.i.i.i.i.i.i.i.i.i:                ; preds = %Num_to_str_4e123451c288c52798d3df0fc84811d2d957f324242982575c70dfd6d338df.exit.i.i.i.i.i.i.i.i.i.i.i
  %.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i.i.i.i.i = load ptr, ptr %3, align 8
  %.sroa.2.0.copyload.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %613, align 8
  %666 = ptrtoint ptr %.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i.i.i.i.i to i64
  %667 = shl nuw i64 %load_str_to_stack.unpack4.i.i.i.i.i.i.i.i.i.i.i.i, 1
  %isneg.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  %668 = select i1 %isneg.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 %667, i64 %666
  %669 = inttoptr i64 %668 to ptr
  %get_rc_ptr.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i64, ptr %669, i64 -1
  %670 = load i64, ptr %get_rc_ptr.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  %.not.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %670, 0
  br i1 %.not.i.i.i.i.i.i.i.i.i.i.i.i.i, label %Inspect_267_ca8af06be175497c1986028dec9ff9bde775267ac5c92d6992ca796b5e797c.exit.i.i.i.i.i.i.i.i.i.i, label %671

671:                                              ; preds = %modify_rc.i.i.i.i.i.i.i.i.i.i.i.i
  %672 = atomicrmw sub ptr %get_rc_ptr.i.i.i.i.i.i.i.i.i.i.i.i, i64 1 monotonic, align 8
  %673 = icmp eq i64 %672, 1
  br i1 %673, label %674, label %Inspect_267_ca8af06be175497c1986028dec9ff9bde775267ac5c92d6992ca796b5e797c.exit.i.i.i.i.i.i.i.i.i.i

674:                                              ; preds = %671
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i.i.i.i.i.i.i.i.i.i, i32 8)
  br label %Inspect_267_ca8af06be175497c1986028dec9ff9bde775267ac5c92d6992ca796b5e797c.exit.i.i.i.i.i.i.i.i.i.i

Inspect_267_ca8af06be175497c1986028dec9ff9bde775267ac5c92d6992ca796b5e797c.exit.i.i.i.i.i.i.i.i.i.i: ; preds = %674, %671, %modify_rc.i.i.i.i.i.i.i.i.i.i.i.i, %Num_to_str_4e123451c288c52798d3df0fc84811d2d957f324242982575c70dfd6d338df.exit.i.i.i.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3)
  br label %Inspect_207_46df57811ab9544a1f9313aab57b6ce51825f299caf5a1ffe2569412f3ed972.exit.i.i.i.i.i.i.i

branch0.i.i.i.i.i.i.i.i.i.i:                      ; preds = %"#Attr_#inc_13.exit.i.i.i.i.i.i.i", %"#Attr_#inc_13.exit.thread.i.i.i.i.i.i.i"
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value15.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %load_element13.i.i.i.i.i.i.i.i.i.i.i)
  switch i8 %result_value.sroa.12.0.copyload.i.i.i.i.i.i.i, label %Inspect_201_4dcdd9fc1c563c9592918682f5bb9bfbff249c75cdcf934a994231c5c3a018.exit.i.i.i.i.i.i.i.i.i.i.i.i [
    i8 0, label %"#Attr_#dec_1.exit118.i.i.i.i.i.i.i.i.i.i.i"
    i8 1, label %"#Attr_#dec_1.exit184.i.i.i.i.i.i.i.i.i.i.i"
    i8 2, label %"#Attr_#dec_1.exit230.i.i.i.i.i.i.i.i.i.i.i"
    i8 3, label %"#Attr_#dec_1.exit296.i.i.i.i.i.i.i.i.i.i.i"
    i8 4, label %list.RocList.incref.exit.i.i.i.i.i.i.i.i.i.i.i.i.i
    i8 5, label %"#Attr_#dec_1.exit368.i.i.i.i.i.i.i.i.i.i.i"
    i8 6, label %"#Attr_#dec_1.exit414.i.i.i.i.i.i.i.i.i.i.i"
  ]

list.RocList.refcount.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %707, %list.RocList.incref.exit.i.i.i.i.i.i.i.i.i.i.i.i.i
  call fastcc void @"#Attr_#dec_14"(%list.RocList %insert_record_field17.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i2.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store7.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value5.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %load_element4.i.i.i.i.i.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 491327616079 to ptr), ptr %load_element4.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -8863084066665136128>, ptr %joinpoint_arg_alloca.sroa.0.sroa.10.0.load_element4.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 8
  store ptr inttoptr (i64 40 to ptr), ptr %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %load_element4.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value5.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  %675 = load i64, ptr %701, align 8
  %.not.i.i.i.i8.i.i.i.i.i.i.i = icmp eq i64 %675, 1
  br i1 %.not.i.i.i.i8.i.i.i.i.i.i.i, label %list.RocList.incref.exit.i.i.i.i.i.thread526.i.i.i.i.i.i.i.i.i.i.i, label %list.RocList.incref.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

list.RocList.incref.exit.i.i.i.i.i.thread526.i.i.i.i.i.i.i.i.i.i.i: ; preds = %list.RocList.refcount.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  store i64 1, ptr %698, align 8
  br label %list.RocList.incref.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

list.RocList.incref.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %list.RocList.incref.exit.i.i.i.i.i.thread526.i.i.i.i.i.i.i.i.i.i.i, %list.RocList.refcount.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %676 = load i64, ptr %705, align 8
  %.not.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %676, 0
  br i1 %.not.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %then_block.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %677

677:                                              ; preds = %list.RocList.incref.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %678 = atomicrmw add ptr %705, i64 1 monotonic, align 8
  br label %then_block.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

then_block.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:   ; preds = %677, %list.RocList.incref.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %700, i64 24, i1 false)
  %load_str_to_stack.unpack4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %load_str_to_stack.elt3.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  %is_big_str.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  br i1 %is_big_str.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %modify_rc.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %"#Attr_#inc_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i"

modify_rc.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:        ; preds = %then_block.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load ptr, ptr %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  %.sroa.2.0.copyload.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  %679 = ptrtoint ptr %.sroa.0.0.copyload.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i to i64
  %680 = shl nuw i64 %load_str_to_stack.unpack4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 1
  %isneg.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  %681 = select i1 %isneg.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 %680, i64 %679
  %682 = inttoptr i64 %681 to ptr
  %get_rc_ptr.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i64, ptr %682, i64 -1
  %683 = load i64, ptr %get_rc_ptr.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  %.not.i.i7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %683, 0
  br i1 %.not.i.i7.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %"#Attr_#inc_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i", label %684

684:                                              ; preds = %modify_rc.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %685 = atomicrmw add ptr %get_rc_ptr.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i"

"#Attr_#inc_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i": ; preds = %684, %modify_rc.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %then_block.lr.ph.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 32 to ptr), ptr %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value5.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 34 to ptr), ptr %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 34 to ptr), ptr %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep5.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  br i1 %is_big_str.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %modify_rc.i11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %"#Attr_#dec_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i"

modify_rc.i11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i:      ; preds = %"#Attr_#inc_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i"
  %.sroa.0.0.copyload.i.i12.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load ptr, ptr %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  %.sroa.2.0.copyload.i.i14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  %686 = ptrtoint ptr %.sroa.0.0.copyload.i.i12.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i to i64
  %687 = shl nuw i64 %load_str_to_stack.unpack4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 1
  %isneg.i.i.i15.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i14.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, 0
  %688 = select i1 %isneg.i.i.i15.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 %687, i64 %686
  %689 = inttoptr i64 %688 to ptr
  %get_rc_ptr.i16.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i64, ptr %689, i64 -1
  %690 = load i64, ptr %get_rc_ptr.i16.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  %.not.i.i17.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %690, 0
  br i1 %.not.i.i17.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %"#Attr_#dec_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i", label %691

691:                                              ; preds = %modify_rc.i11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i
  %692 = atomicrmw sub ptr %get_rc_ptr.i16.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 1 monotonic, align 8
  %693 = icmp eq i64 %692, 1
  br i1 %693, label %694, label %"#Attr_#dec_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i"

694:                                              ; preds = %691
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i16.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i32 8)
  br label %"#Attr_#dec_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i"

"#Attr_#dec_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i": ; preds = %694, %691, %modify_rc.i11.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, %"#Attr_#inc_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i"
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %result_value5.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %result_value4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, i64 24, i1 false)
  %695 = insertvalue %list.RocList poison, ptr %700, 0
  %696 = insertvalue %list.RocList %695, i64 1, 1
  %load_element16.i.i.i.i.i.i.i.i.i.i.i.i.i = insertvalue %list.RocList %696, i64 1, 2
  call fastcc void @"#Attr_#dec_14"(%list.RocList %load_element16.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value4.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call fastcc void @"#Attr_#dec_14"(%list.RocList %load_element16.i.i.i.i.i.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 41 to ptr), ptr %const_str_store7.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep10.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value5.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store7.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store7.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value5.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %load_element4.i.i.i.i.i.i.i.i.i.i.i.i.i)
  br label %"#Derived_custom3_beb22fad19423347b2aa99b33212e862ded3f83df5d6238acb1a6a9ade3e.exit.i.i.i.i.i.i.i.i.i.i"

Inspect_201_4dcdd9fc1c563c9592918682f5bb9bfbff249c75cdcf934a994231c5c3a018.exit.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %"#Attr_#dec_1.exit414.i.i.i.i.i.i.i.i.i.i.i", %"#Attr_#dec_1.exit368.i.i.i.i.i.i.i.i.i.i.i", %"#Attr_#dec_1.exit296.i.i.i.i.i.i.i.i.i.i.i", %"#Attr_#dec_1.exit230.i.i.i.i.i.i.i.i.i.i.i", %"#Attr_#dec_1.exit184.i.i.i.i.i.i.i.i.i.i.i", %"#Attr_#dec_1.exit118.i.i.i.i.i.i.i.i.i.i.i", %branch0.i.i.i.i.i.i.i.i.i.i
  %joinpoint_arg_alloca.sroa.0.sroa.0.0.ph.i.i.i.i.i.i.i.i.i.i.i = phi ptr [ inttoptr (i64 5006142830275619905 to ptr), %"#Attr_#dec_1.exit118.i.i.i.i.i.i.i.i.i.i.i" ], [ inttoptr (i64 7588686753992503874 to ptr), %"#Attr_#dec_1.exit184.i.i.i.i.i.i.i.i.i.i.i" ], [ inttoptr (i64 8103508940177043017 to ptr), %"#Attr_#dec_1.exit230.i.i.i.i.i.i.i.i.i.i.i" ], [ inttoptr (i64 7236850772014821198 to ptr), %"#Attr_#dec_1.exit296.i.i.i.i.i.i.i.i.i.i.i" ], [ inttoptr (i64 7882791824596956495 to ptr), %"#Attr_#dec_1.exit368.i.i.i.i.i.i.i.i.i.i.i" ], [ inttoptr (i64 7598543892949132624 to ptr), %"#Attr_#dec_1.exit414.i.i.i.i.i.i.i.i.i.i.i" ], [ inttoptr (i64 8245933071047814741 to ptr), %branch0.i.i.i.i.i.i.i.i.i.i ]
  %697 = phi <2 x i64> [ <i64 495874959736, i64 -8286623314361712640>, %"#Attr_#dec_1.exit118.i.i.i.i.i.i.i.i.i.i.i" ], [ <i64 25968, i64 -8502796096475496448>, %"#Attr_#dec_1.exit184.i.i.i.i.i.i.i.i.i.i.i" ], [ <i64 6579572, i64 -8430738502437568512>, %"#Attr_#dec_1.exit230.i.i.i.i.i.i.i.i.i.i.i" ], [ <i64 0, i64 -8646911284551352320>, %"#Attr_#dec_1.exit296.i.i.i.i.i.i.i.i.i.i.i" ], [ <i64 7959151, i64 -8430738502437568512>, %"#Attr_#dec_1.exit368.i.i.i.i.i.i.i.i.i.i.i" ], [ <i64 7234304299306872431, i64 -8070450532247928832>, %"#Attr_#dec_1.exit414.i.i.i.i.i.i.i.i.i.i.i" ], [ <i64 6579572, i64 -8430738502437568512>, %branch0.i.i.i.i.i.i.i.i.i.i ]
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i2.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %load_element.i.i.i.i.i.i.i.i.i.i.i.i.i)
  store ptr %joinpoint_arg_alloca.sroa.0.sroa.0.0.ph.i.i.i.i.i.i.i.i.i.i.i, ptr %load_element.i.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> %697, ptr %joinpoint_arg_alloca.sroa.0.sroa.10.0.load_element.i.i.sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value.i.i.i.i.i.i.i.i, ptr nonnull %load_element.i.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value.i2.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %load_element.i.i.i.i.i.i.i.i.i.i.i.i.i)
  br label %"#Derived_custom3_beb22fad19423347b2aa99b33212e862ded3f83df5d6238acb1a6a9ade3e.exit.i.i.i.i.i.i.i.i.i.i"

"#Attr_#dec_1.exit118.i.i.i.i.i.i.i.i.i.i.i":     ; preds = %branch0.i.i.i.i.i.i.i.i.i.i
  br label %Inspect_201_4dcdd9fc1c563c9592918682f5bb9bfbff249c75cdcf934a994231c5c3a018.exit.i.i.i.i.i.i.i.i.i.i.i.i

"#Attr_#dec_1.exit184.i.i.i.i.i.i.i.i.i.i.i":     ; preds = %branch0.i.i.i.i.i.i.i.i.i.i
  br label %Inspect_201_4dcdd9fc1c563c9592918682f5bb9bfbff249c75cdcf934a994231c5c3a018.exit.i.i.i.i.i.i.i.i.i.i.i.i

"#Attr_#dec_1.exit230.i.i.i.i.i.i.i.i.i.i.i":     ; preds = %branch0.i.i.i.i.i.i.i.i.i.i
  br label %Inspect_201_4dcdd9fc1c563c9592918682f5bb9bfbff249c75cdcf934a994231c5c3a018.exit.i.i.i.i.i.i.i.i.i.i.i.i

"#Attr_#dec_1.exit296.i.i.i.i.i.i.i.i.i.i.i":     ; preds = %branch0.i.i.i.i.i.i.i.i.i.i
  br label %Inspect_201_4dcdd9fc1c563c9592918682f5bb9bfbff249c75cdcf934a994231c5c3a018.exit.i.i.i.i.i.i.i.i.i.i.i.i

list.RocList.incref.exit.i.i.i.i.i.i.i.i.i.i.i.i.i: ; preds = %branch0.i.i.i.i.i.i.i.i.i.i
  store ptr %result_value.sroa.0.0.copyload.i.i.i.i.i.i.i, ptr %load_element13.i.i.i.i.i.i.i.i.i.i.i, align 8
  store i64 %result_value.sroa.6.0.copyload.i.i.i.i.i.i.i, ptr %load_element.sroa.9.0.load_element13.sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 8
  store i64 %result_value.sroa.9.0.copyload.i.i.i.i.i.i.i, ptr %load_element.sroa.16.0.load_element13.sroa_idx.i.i.i.i.i.i.i.i.i.i.i, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store9.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store8.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value7.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store6.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store5.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value4.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store3.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store1.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 12565487 to ptr), ptr %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9007199254740992000>, ptr %.fca.1.gep.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store ptr inttoptr (i64 2983184388871189852 to ptr), ptr %const_str_store1.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -8646911284551352320>, ptr %.fca.1.gep13.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_replace_each_f0adb8f180253d489b50ac5199522556362f583929ee5e65c919bd9ed2bc82f(ptr nonnull %load_element13.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store1.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 9142498 to ptr), ptr %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9007199254740992000>, ptr %.fca.1.gep16.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store ptr inttoptr (i64 2981998886830765404 to ptr), ptr %const_str_store3.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -8646911284551352320>, ptr %.fca.1.gep19.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_replace_each_f0adb8f180253d489b50ac5199522556362f583929ee5e65c919bd9ed2bc82f(ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store3.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value4.i.i.i.i.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 9208034 to ptr), ptr %const_str_store5.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9007199254740992000>, ptr %.fca.1.gep22.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store ptr inttoptr (i64 2982280361807476060 to ptr), ptr %const_str_store6.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -8646911284551352320>, ptr %.fca.1.gep25.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_replace_each_f0adb8f180253d489b50ac5199522556362f583929ee5e65c919bd9ed2bc82f(ptr nonnull %result_value4.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store5.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store6.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value7.i.i.i.i.i.i.i.i.i.i.i.i)
  store ptr inttoptr (i64 9273570 to ptr), ptr %const_str_store8.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9007199254740992000>, ptr %.fca.1.gep28.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store ptr inttoptr (i64 2982561836784186716 to ptr), ptr %const_str_store9.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -8646911284551352320>, ptr %.fca.1.gep31.i.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_replace_each_f0adb8f180253d489b50ac5199522556362f583929ee5e65c919bd9ed2bc82f(ptr nonnull %result_value7.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store8.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store9.i.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value15.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store9.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store8.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value7.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store6.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store5.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value4.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store3.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store2.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store1.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i.i)
  %698 = call ptr @roc_alloc(i64 40, i32 8)
  %699 = icmp ne ptr %698, null
  call void @llvm.assume(i1 %699)
  %700 = getelementptr inbounds i8, ptr %698, i64 16
  %701 = getelementptr inbounds i8, ptr %698, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %700, ptr noundef nonnull align 8 dereferenceable(24) %result_value15.i.i.i.i.i.i.i.i.i.i.i, i64 24, i1 false)
  %insert_record_field.i.i.i.i.i.i.i.i.i.i.i = insertvalue %list.RocList zeroinitializer, ptr %700, 0
  %insert_record_field16.i.i.i.i.i.i.i.i.i.i.i = insertvalue %list.RocList %insert_record_field.i.i.i.i.i.i.i.i.i.i.i, i64 1, 1
  %insert_record_field17.i.i.i.i.i.i.i.i.i.i.i = insertvalue %list.RocList %insert_record_field16.i.i.i.i.i.i.i.i.i.i.i, i64 1, 2
  %702 = ptrtoint ptr %700 to i64
  store <2 x i64> <i64 1, i64 1>, ptr %698, align 8
  %703 = and i64 %702, -8
  %704 = add i64 %703, -8
  %705 = inttoptr i64 %704 to ptr
  %706 = load i64, ptr %705, align 8
  %.not.i.i.i.i297.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %706, 0
  br i1 %.not.i.i.i.i297.i.i.i.i.i.i.i.i.i.i.i, label %list.RocList.refcount.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i, label %707

707:                                              ; preds = %list.RocList.incref.exit.i.i.i.i.i.i.i.i.i.i.i.i.i
  %708 = atomicrmw add ptr %705, i64 1 monotonic, align 8
  br label %list.RocList.refcount.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i

"#Attr_#dec_1.exit368.i.i.i.i.i.i.i.i.i.i.i":     ; preds = %branch0.i.i.i.i.i.i.i.i.i.i
  br label %Inspect_201_4dcdd9fc1c563c9592918682f5bb9bfbff249c75cdcf934a994231c5c3a018.exit.i.i.i.i.i.i.i.i.i.i.i.i

"#Attr_#dec_1.exit414.i.i.i.i.i.i.i.i.i.i.i":     ; preds = %branch0.i.i.i.i.i.i.i.i.i.i
  br label %Inspect_201_4dcdd9fc1c563c9592918682f5bb9bfbff249c75cdcf934a994231c5c3a018.exit.i.i.i.i.i.i.i.i.i.i.i.i

"#Derived_custom3_beb22fad19423347b2aa99b33212e862ded3f83df5d6238acb1a6a9ade3e.exit.i.i.i.i.i.i.i.i.i.i": ; preds = %Inspect_201_4dcdd9fc1c563c9592918682f5bb9bfbff249c75cdcf934a994231c5c3a018.exit.i.i.i.i.i.i.i.i.i.i.i.i, %"#Attr_#dec_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i"
  %result_value.i2.sink.i.i.i.i.i.i.i.i.i.i.i.i = phi ptr [ %result_value.i2.i.i.i.i.i.i.i.i.i.i.i.i, %Inspect_201_4dcdd9fc1c563c9592918682f5bb9bfbff249c75cdcf934a994231c5c3a018.exit.i.i.i.i.i.i.i.i.i.i.i.i ], [ %result_value1.i.i.i.i.i.i.i.i.i.i.i.i, %"#Attr_#dec_1.exit.i.i.i.i.i.i.i.i.i.i.i.i.i.i.i" ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %result_value.i.i.i.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %result_value.i2.sink.i.i.i.i.i.i.i.i.i.i.i.i, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i2.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value15.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %load_element13.i.i.i.i.i.i.i.i.i.i.i)
  br label %Inspect_207_46df57811ab9544a1f9313aab57b6ce51825f299caf5a1ffe2569412f3ed972.exit.i.i.i.i.i.i.i

branch1.i.i.i.i.i.i.i.i.i.i:                      ; preds = %"#Attr_#inc_13.exit.i.i.i.i.i.i.i", %"#Attr_#inc_13.exit.thread16.i.i.i.i.i.i.i"
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store3.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value2.i3.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %load_element.i.i.i.i.i.i.i.i.i.i.i)
  store ptr %result_value.sroa.0.0.copyload.i.i.i.i.i.i.i, ptr %load_element.i.i.i.i.i.i.i.i.i.i.i, align 8
  store i64 %result_value.sroa.6.0.copyload.i.i.i.i.i.i.i, ptr %result_value.sroa.6.0.load_element.i.i.i.i.sroa_idx.i.i.i.i.i.i.i, align 8
  store i64 %result_value.sroa.9.0.copyload.i.i.i.i.i.i.i, ptr %result_value.sroa.9.0.load_element.i.i.i.i.sroa_idx.i.i.i.i.i.i.i, align 8
  store ptr inttoptr (i64 34 to ptr), ptr %const_str_store.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i)
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %load_element.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value2.i3.i.i.i.i.i.i.i.i.i.i)
  %is_big_str.i11.i.i.i.i.i.i.i.i.i.i.i = icmp sgt i64 %result_value.sroa.9.0.copyload.i.i.i.i.i.i.i, 0
  br i1 %is_big_str.i11.i.i.i.i.i.i.i.i.i.i.i, label %modify_rc.i12.i.i.i.i.i.i.i.i.i.i.i, label %Inspect_custom_dbg_str_a4f1bd72ba9ffa9bc6b14a41321324cb50df1636d7bfebf57c16d684c9fbc2.exit.i.i.i.i.i.i.i.i.i.i

modify_rc.i12.i.i.i.i.i.i.i.i.i.i.i:              ; preds = %branch1.i.i.i.i.i.i.i.i.i.i
  %709 = ptrtoint ptr %result_value.sroa.0.0.copyload.i.i.i.i.i.i.i to i64
  %710 = shl nuw i64 %result_value.sroa.9.0.copyload.i.i.i.i.i.i.i, 1
  %isneg.i.i.i16.i.i.i.i.i.i.i.i.i.i.i = icmp slt i64 %result_value.sroa.6.0.copyload.i.i.i.i.i.i.i, 0
  %711 = select i1 %isneg.i.i.i16.i.i.i.i.i.i.i.i.i.i.i, i64 %710, i64 %709
  %712 = inttoptr i64 %711 to ptr
  %get_rc_ptr.i17.i.i.i.i.i.i.i.i.i.i.i = getelementptr inbounds i64, ptr %712, i64 -1
  %713 = load i64, ptr %get_rc_ptr.i17.i.i.i.i.i.i.i.i.i.i.i, align 8
  %.not.i.i18.i.i.i.i.i.i.i.i.i.i.i = icmp eq i64 %713, 0
  br i1 %.not.i.i18.i.i.i.i.i.i.i.i.i.i.i, label %Inspect_custom_dbg_str_a4f1bd72ba9ffa9bc6b14a41321324cb50df1636d7bfebf57c16d684c9fbc2.exit.i.i.i.i.i.i.i.i.i.i, label %714

714:                                              ; preds = %modify_rc.i12.i.i.i.i.i.i.i.i.i.i.i
  %715 = atomicrmw sub ptr %get_rc_ptr.i17.i.i.i.i.i.i.i.i.i.i.i, i64 1 monotonic, align 8
  %716 = icmp eq i64 %715, 1
  br i1 %716, label %717, label %Inspect_custom_dbg_str_a4f1bd72ba9ffa9bc6b14a41321324cb50df1636d7bfebf57c16d684c9fbc2.exit.i.i.i.i.i.i.i.i.i.i

717:                                              ; preds = %714
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i17.i.i.i.i.i.i.i.i.i.i.i, i32 8)
  br label %Inspect_custom_dbg_str_a4f1bd72ba9ffa9bc6b14a41321324cb50df1636d7bfebf57c16d684c9fbc2.exit.i.i.i.i.i.i.i.i.i.i

Inspect_custom_dbg_str_a4f1bd72ba9ffa9bc6b14a41321324cb50df1636d7bfebf57c16d684c9fbc2.exit.i.i.i.i.i.i.i.i.i.i: ; preds = %717, %714, %modify_rc.i12.i.i.i.i.i.i.i.i.i.i.i, %branch1.i.i.i.i.i.i.i.i.i.i
  store ptr inttoptr (i64 34 to ptr), ptr %const_str_store3.i.i.i.i.i.i.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep6.i.i.i.i.i.i.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value2.i3.i.i.i.i.i.i.i.i.i.i, ptr nonnull %const_str_store3.i.i.i.i.i.i.i.i.i.i.i, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store3.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value2.i3.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %load_element.i.i.i.i.i.i.i.i.i.i.i)
  br label %Inspect_207_46df57811ab9544a1f9313aab57b6ce51825f299caf5a1ffe2569412f3ed972.exit.i.i.i.i.i.i.i

Inspect_207_46df57811ab9544a1f9313aab57b6ce51825f299caf5a1ffe2569412f3ed972.exit.i.i.i.i.i.i.i: ; preds = %Inspect_custom_dbg_str_a4f1bd72ba9ffa9bc6b14a41321324cb50df1636d7bfebf57c16d684c9fbc2.exit.i.i.i.i.i.i.i.i.i.i, %"#Derived_custom3_beb22fad19423347b2aa99b33212e862ded3f83df5d6238acb1a6a9ade3e.exit.i.i.i.i.i.i.i.i.i.i", %Inspect_267_ca8af06be175497c1986028dec9ff9bde775267ac5c92d6992ca796b5e797c.exit.i.i.i.i.i.i.i.i.i.i
  %result_value1.sink.i.i.i.i.i.i.i.i.i.i = phi ptr [ %result_value1.i.i.i.i.i.i.i.i.i.i, %Inspect_custom_dbg_str_a4f1bd72ba9ffa9bc6b14a41321324cb50df1636d7bfebf57c16d684c9fbc2.exit.i.i.i.i.i.i.i.i.i.i ], [ %result_value.i.i.i.i.i.i.i.i.i.i, %"#Derived_custom3_beb22fad19423347b2aa99b33212e862ded3f83df5d6238acb1a6a9ade3e.exit.i.i.i.i.i.i.i.i.i.i" ], [ %result_value2.i.i.i.i.i.i.i.i.i.i.i, %Inspect_267_ca8af06be175497c1986028dec9ff9bde775267ac5c92d6992ca796b5e797c.exit.i.i.i.i.i.i.i.i.i.i ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %joinpoint_arg_alloca.i.i.i.i.i.i.i, ptr noundef nonnull align 8 dereferenceable(24) %result_value1.sink.i.i.i.i.i.i.i.i.i.i, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value2.i.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value1.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i.i.i.i)
  %718 = insertvalue %list.RocList poison, ptr %599, 0
  %719 = insertvalue %list.RocList %718, i64 1, 1
  %load_element16.i.i.i.i.i = insertvalue %list.RocList %719, i64 1, 2
  call fastcc void @"#Attr_#dec_12"(%list.RocList %load_element16.i.i.i.i.i)
  call fastcc void @"#Attr_#dec_12"(%list.RocList %load_element16.i.i.i.i.i)
  store ptr inttoptr (i64 41 to ptr), ptr %const_str_store7.i.i.i.i.i, align 8
  %.fca.1.gep10.i.i.i.i.i = getelementptr inbounds %str.RocStr, ptr %const_str_store7.i.i.i.i.i, i64 0, i32 1
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep10.i.i.i.i.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %joinpoint_arg_alloca.i.i.i.i.i.i.i, ptr nonnull %const_str_store7.i.i.i.i.i, ptr nonnull %result_value27)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %joinpoint_arg_alloca.i.i.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store7.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value5.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %const_str_store.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %load_element4.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value1.i.i)
  %.sroa.1.0..sroa_idx.i.i = getelementptr inbounds i8, ptr %result_value27, i64 8
  %.sroa.1.0.copyload.i.i = load i64, ptr %.sroa.1.0..sroa_idx.i.i, align 8
  %.sroa.2.0..sroa_idx.i.i = getelementptr inbounds i8, ptr %result_value27, i64 16
  %.sroa.2.0.copyload.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i, align 8
  %720 = icmp slt i64 %.sroa.2.0.copyload.i.i, 0
  %721 = lshr i64 %.sroa.2.0.copyload.i.i, 56
  %722 = and i64 %721, 127
  %723 = and i64 %.sroa.1.0.copyload.i.i, 9223372036854775807
  %common.ret.op.i.i.i161 = select i1 %720, i64 %722, i64 %723
  %.sroa.0.0.copyload = load ptr, ptr %result_value27, align 8
  %724 = icmp eq i64 %common.ret.op.i.i.i161, 0
  br i1 %724, label %else_block38, label %.lr.ph.i.i.preheader

.lr.ph.i.i.preheader:                             ; preds = %Inspect_207_46df57811ab9544a1f9313aab57b6ce51825f299caf5a1ffe2569412f3ed972.exit.i.i.i.i.i.i.i
  %725 = ptrtoint ptr %.sroa.0.0.copyload to i64
  %.sroa.0.0.extract.trunc = trunc i64 %725 to i8
  br i1 %720, label %.lr.ph.i.i.cont, label %.lr.ph.i.i.else

then_block31:                                     ; preds = %.lr.ph.i.i.cont
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %2)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2, ptr noundef nonnull align 8 dereferenceable(24) %result_value27, i64 24, i1 false)
  %726 = getelementptr inbounds i8, ptr %2, i64 16
  %.val.i.i.i199 = load i64, ptr %726, align 8
  %727 = load ptr, ptr %2, align 8
  %728 = icmp slt i64 %.val.i.i.i199, 0
  %common.ret.op.i5.i.i = select i1 %728, ptr %2, ptr %727
  %729 = getelementptr i8, ptr %common.ret.op.i5.i.i, i64 %common.ret.op.i.i.i161
  %invariant.gep.i.i = getelementptr i8, ptr %729, i64 -1
  %730 = load i8, ptr %invariant.gep.i.i, align 1
  %.not.i.i201 = icmp eq i8 %730, 41
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %2)
  br i1 %.not.i.i201, label %then_block37, label %else_block38

.lr.ph.i.i.else:                                  ; preds = %.lr.ph.i.i.preheader
  %.else.val = load i8, ptr %.sroa.0.0.copyload, align 1
  br label %.lr.ph.i.i.cont

.lr.ph.i.i.cont:                                  ; preds = %.lr.ph.i.i.preheader, %.lr.ph.i.i.else
  %731 = phi i8 [ %.sroa.0.0.extract.trunc, %.lr.ph.i.i.preheader ], [ %.else.val, %.lr.ph.i.i.else ]
  %.not.i.i = icmp eq i8 %731, 40
  br i1 %.not.i.i, label %then_block31, label %else_block38

then_block37:                                     ; preds = %then_block31
  store ptr inttoptr (i64 40 to ptr), ptr %const_str_store39, align 8
  %.fca.1.gep53 = getelementptr inbounds %str.RocStr, ptr %const_str_store39, i64 0, i32 1
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %.fca.1.gep53, align 8
  %.fca.2.gep57 = getelementptr inbounds %str.RocStr, ptr %const_str_store40, i64 0, i32 2
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %const_str_store40, i8 0, i64 16, i1 false)
  store i64 -9223372036854775808, ptr %.fca.2.gep57, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value4.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value3.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %struct_field2.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %struct_field.i)
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %load_element.i)
  call void @llvm.lifetime.start.p0(i64 56, ptr nonnull %result_value.i215)
  %is_big_str.i.i = icmp sgt i64 %.sroa.2.0.copyload.i.i, 0
  br i1 %is_big_str.i.i, label %modify_rc.i.i, label %"#Attr_#inc_1.exit.i.thread"

modify_rc.i.i:                                    ; preds = %then_block37
  %732 = shl nuw i64 %.sroa.2.0.copyload.i.i, 1
  %isneg.i.i.i.i = icmp slt i64 %.sroa.1.0.copyload.i.i, 0
  %733 = select i1 %isneg.i.i.i.i, i64 %732, i64 %725
  %734 = inttoptr i64 %733 to ptr
  %get_rc_ptr.i.i = getelementptr inbounds i64, ptr %734, i64 -1
  %735 = load i64, ptr %get_rc_ptr.i.i, align 8
  %.not.i.i.i219 = icmp eq i64 %735, 0
  br i1 %.not.i.i.i219, label %"#Attr_#inc_1.exit.i", label %736

736:                                              ; preds = %modify_rc.i.i
  %737 = atomicrmw add ptr %get_rc_ptr.i.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit.i"

"#Attr_#inc_1.exit.i":                            ; preds = %736, %modify_rc.i.i
  call fastcc void @Str_split_first_bfa1d47a221bdaf089999196bed323c433d1a6b8c78ec612e6fa7b3e3d811(ptr nonnull %result_value27, ptr nonnull %const_str_store39, ptr nonnull %result_value.i215)
  %tag_id_ptr.i216 = getelementptr inbounds { [6 x i64], i8 }, ptr %result_value.i215, i64 0, i32 1
  %load_tag_id.i217 = load i8, ptr %tag_id_ptr.i216, align 8
  %eq_u8.i = icmp eq i8 %load_tag_id.i217, 1
  br i1 %eq_u8.i, label %modify_rc.i8.i, label %"#Attr_#dec_1.exit230.thread"

"#Attr_#dec_1.exit230.thread":                    ; preds = %"#Attr_#inc_1.exit.i"
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value4.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value3.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %struct_field2.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %struct_field.i)
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %load_element.i)
  call void @llvm.lifetime.end.p0(i64 56, ptr nonnull %result_value.i215)
  %.fca.2.gep63540 = getelementptr inbounds %str.RocStr, ptr %const_str_store43, i64 0, i32 2
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %const_str_store43, i8 0, i64 16, i1 false)
  store i64 -9223372036854775808, ptr %.fca.2.gep63540, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value4.i231)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value3.i232)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %struct_field2.i233)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %struct_field.i234)
  br label %modify_rc.i.i304

"#Attr_#inc_1.exit.i.thread":                     ; preds = %then_block37
  call fastcc void @Str_split_first_bfa1d47a221bdaf089999196bed323c433d1a6b8c78ec612e6fa7b3e3d811(ptr nonnull %result_value27, ptr nonnull %const_str_store39, ptr nonnull %result_value.i215)
  %tag_id_ptr.i216412 = getelementptr inbounds { [6 x i64], i8 }, ptr %result_value.i215, i64 0, i32 1
  %load_tag_id.i217413 = load i8, ptr %tag_id_ptr.i216412, align 8
  %eq_u8.i414 = icmp eq i8 %load_tag_id.i217413, 1
  br i1 %eq_u8.i414, label %"#Attr_#dec_1.exit.i218", label %"#Attr_#dec_1.exit230.thread550"

"#Attr_#dec_1.exit230.thread550":                 ; preds = %"#Attr_#inc_1.exit.i.thread"
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value4.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value3.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %struct_field2.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %struct_field.i)
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %load_element.i)
  call void @llvm.lifetime.end.p0(i64 56, ptr nonnull %result_value.i215)
  %.fca.2.gep63555 = getelementptr inbounds %str.RocStr, ptr %const_str_store43, i64 0, i32 2
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %const_str_store43, i8 0, i64 16, i1 false)
  store i64 -9223372036854775808, ptr %.fca.2.gep63555, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value4.i231)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value3.i232)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %struct_field2.i233)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %struct_field.i234)
  br label %"#Attr_#inc_1.exit.i.i.i.i"

modify_rc.i8.i:                                   ; preds = %"#Attr_#inc_1.exit.i"
  %738 = load i64, ptr %get_rc_ptr.i.i, align 8
  %.not.i.i14.i = icmp eq i64 %738, 0
  br i1 %.not.i.i14.i, label %"#Attr_#dec_1.exit.i218", label %739

739:                                              ; preds = %modify_rc.i8.i
  %740 = atomicrmw sub ptr %get_rc_ptr.i.i, i64 1 monotonic, align 8
  %741 = icmp eq i64 %740, 1
  br i1 %741, label %742, label %"#Attr_#dec_1.exit.i218"

742:                                              ; preds = %739
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i, i32 8)
  br label %"#Attr_#dec_1.exit.i218"

"#Attr_#dec_1.exit.i218":                         ; preds = %"#Attr_#inc_1.exit.i.thread", %742, %739, %modify_rc.i8.i
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %load_element.i, ptr noundef nonnull align 8 dereferenceable(48) %result_value.i215, i64 48, i1 false)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %struct_field.i, ptr noundef nonnull align 8 dereferenceable(24) %result_value.i215, i64 24, i1 false)
  %load_element.24.sroa_idx.i = getelementptr inbounds i8, ptr %load_element.i, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %struct_field2.i, ptr noundef nonnull align 8 dereferenceable(24) %load_element.24.sroa_idx.i, i64 24, i1 false)
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %const_str_store40, ptr nonnull %struct_field.i, ptr nonnull %result_value3.i)
  %load_str_to_stack.elt3.i15.i = getelementptr inbounds %str.RocStr, ptr %struct_field.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i16.i = load i64, ptr %load_str_to_stack.elt3.i15.i, align 8
  %is_big_str.i17.i = icmp sgt i64 %load_str_to_stack.unpack4.i16.i, 0
  br i1 %is_big_str.i17.i, label %modify_rc.i18.i, label %"#Attr_#dec_1.exit25.i"

modify_rc.i18.i:                                  ; preds = %"#Attr_#dec_1.exit.i218"
  %.sroa.0.0.copyload.i.i19.i = load ptr, ptr %struct_field.i, align 8
  %.sroa.2.0..sroa_idx.i.i20.i = getelementptr inbounds i8, ptr %struct_field.i, i64 8
  %.sroa.2.0.copyload.i.i21.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i20.i, align 8
  %743 = ptrtoint ptr %.sroa.0.0.copyload.i.i19.i to i64
  %744 = shl nuw i64 %load_str_to_stack.unpack4.i16.i, 1
  %isneg.i.i.i22.i = icmp slt i64 %.sroa.2.0.copyload.i.i21.i, 0
  %745 = select i1 %isneg.i.i.i22.i, i64 %744, i64 %743
  %746 = inttoptr i64 %745 to ptr
  %get_rc_ptr.i23.i = getelementptr inbounds i64, ptr %746, i64 -1
  %747 = load i64, ptr %get_rc_ptr.i23.i, align 8
  %.not.i.i24.i = icmp eq i64 %747, 0
  br i1 %.not.i.i24.i, label %"#Attr_#dec_1.exit25.i", label %748

748:                                              ; preds = %modify_rc.i18.i
  %749 = atomicrmw sub ptr %get_rc_ptr.i23.i, i64 1 monotonic, align 8
  %750 = icmp eq i64 %749, 1
  br i1 %750, label %751, label %"#Attr_#dec_1.exit25.i"

751:                                              ; preds = %748
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i23.i, i32 8)
  br label %"#Attr_#dec_1.exit25.i"

"#Attr_#dec_1.exit25.i":                          ; preds = %751, %748, %modify_rc.i18.i, %"#Attr_#dec_1.exit.i218"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %struct_field2.i, ptr nonnull %result_value3.i, ptr nonnull %result_value4.i)
  %load_str_to_stack.elt3.i26.i = getelementptr inbounds %str.RocStr, ptr %result_value3.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i27.i = load i64, ptr %load_str_to_stack.elt3.i26.i, align 8
  %is_big_str.i28.i = icmp sgt i64 %load_str_to_stack.unpack4.i27.i, 0
  br i1 %is_big_str.i28.i, label %modify_rc.i29.i, label %"#Attr_#dec_1.exit230"

modify_rc.i29.i:                                  ; preds = %"#Attr_#dec_1.exit25.i"
  %.sroa.0.0.copyload.i.i30.i = load ptr, ptr %result_value3.i, align 8
  %.sroa.2.0..sroa_idx.i.i31.i = getelementptr inbounds i8, ptr %result_value3.i, i64 8
  %.sroa.2.0.copyload.i.i32.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i31.i, align 8
  %752 = ptrtoint ptr %.sroa.0.0.copyload.i.i30.i to i64
  %753 = shl nuw i64 %load_str_to_stack.unpack4.i27.i, 1
  %isneg.i.i.i33.i = icmp slt i64 %.sroa.2.0.copyload.i.i32.i, 0
  %754 = select i1 %isneg.i.i.i33.i, i64 %753, i64 %752
  %755 = inttoptr i64 %754 to ptr
  %get_rc_ptr.i34.i = getelementptr inbounds i64, ptr %755, i64 -1
  %756 = load i64, ptr %get_rc_ptr.i34.i, align 8
  %.not.i.i35.i = icmp eq i64 %756, 0
  br i1 %.not.i.i35.i, label %"#Attr_#dec_1.exit230", label %757

757:                                              ; preds = %modify_rc.i29.i
  %758 = atomicrmw sub ptr %get_rc_ptr.i34.i, i64 1 monotonic, align 8
  %759 = icmp eq i64 %758, 1
  br i1 %759, label %760, label %"#Attr_#dec_1.exit230"

760:                                              ; preds = %757
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i34.i, i32 8)
  br label %"#Attr_#dec_1.exit230"

"#Attr_#dec_1.exit230":                           ; preds = %"#Attr_#dec_1.exit25.i", %modify_rc.i29.i, %757, %760
  %result_value41.sroa.14.0.result_value4.i.sroa_idx = getelementptr inbounds i8, ptr %result_value4.i, i64 8
  %result_value41.sroa.14.0.copyload381 = load i64, ptr %result_value41.sroa.14.0.result_value4.i.sroa_idx, align 8
  %result_value41.sroa.25.0.result_value4.i.sroa_idx = getelementptr inbounds i8, ptr %result_value4.i, i64 16
  %result_value41.sroa.25.0.copyload384 = load i64, ptr %result_value41.sroa.25.0.result_value4.i.sroa_idx, align 8
  %result_value41.sroa.0.0.pre = load ptr, ptr %result_value4.i, align 8
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value4.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value3.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %struct_field2.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %struct_field.i)
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %load_element.i)
  call void @llvm.lifetime.end.p0(i64 56, ptr nonnull %result_value.i215)
  %.fca.2.gep63 = getelementptr inbounds %str.RocStr, ptr %const_str_store43, i64 0, i32 2
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %const_str_store43, i8 0, i64 16, i1 false)
  store i64 -9223372036854775808, ptr %.fca.2.gep63, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value4.i231)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value3.i232)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %struct_field2.i233)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %struct_field.i234)
  %is_big_str.i.i237 = icmp sgt i64 %result_value41.sroa.25.0.copyload384, 0
  br i1 %is_big_str.i.i237, label %modify_rc.i.i304, label %"#Attr_#inc_1.exit.i.i.i.i"

modify_rc.i.i304:                                 ; preds = %"#Attr_#dec_1.exit230.thread", %"#Attr_#dec_1.exit230"
  %result_value41.sroa.25.0546 = phi i64 [ %.sroa.2.0.copyload.i.i, %"#Attr_#dec_1.exit230.thread" ], [ %result_value41.sroa.25.0.copyload384, %"#Attr_#dec_1.exit230" ]
  %result_value41.sroa.14.0544 = phi i64 [ %.sroa.1.0.copyload.i.i, %"#Attr_#dec_1.exit230.thread" ], [ %result_value41.sroa.14.0.copyload381, %"#Attr_#dec_1.exit230" ]
  %result_value41.sroa.0.0543 = phi ptr [ %.sroa.0.0.copyload, %"#Attr_#dec_1.exit230.thread" ], [ %result_value41.sroa.0.0.pre, %"#Attr_#dec_1.exit230" ]
  %761 = ptrtoint ptr %result_value41.sroa.0.0543 to i64
  %762 = shl nuw i64 %result_value41.sroa.25.0546, 1
  %isneg.i.i.i.i308 = icmp slt i64 %result_value41.sroa.14.0544, 0
  %763 = select i1 %isneg.i.i.i.i308, i64 %762, i64 %761
  %764 = inttoptr i64 %763 to ptr
  %get_rc_ptr.i.i309 = getelementptr inbounds i64, ptr %764, i64 -1
  %765 = load i64, ptr %get_rc_ptr.i.i309, align 8
  %.not.i.i.i310 = icmp eq i64 %765, 0
  br i1 %.not.i.i.i310, label %"#Attr_#inc_1.exit.i.i.i.i", label %766

766:                                              ; preds = %modify_rc.i.i304
  %767 = atomicrmw add ptr %get_rc_ptr.i.i309, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit.i.i.i.i"

"#Attr_#inc_1.exit.i.i.i.i":                      ; preds = %"#Attr_#dec_1.exit230.thread550", %"#Attr_#dec_1.exit230", %modify_rc.i.i304, %766
  %is_big_str.i.i237548 = phi i1 [ false, %"#Attr_#dec_1.exit230" ], [ true, %modify_rc.i.i304 ], [ true, %766 ], [ false, %"#Attr_#dec_1.exit230.thread550" ]
  %result_value41.sroa.25.0547 = phi i64 [ %result_value41.sroa.25.0.copyload384, %"#Attr_#dec_1.exit230" ], [ %result_value41.sroa.25.0546, %modify_rc.i.i304 ], [ %result_value41.sroa.25.0546, %766 ], [ %.sroa.2.0.copyload.i.i, %"#Attr_#dec_1.exit230.thread550" ]
  %result_value41.sroa.14.0545 = phi i64 [ %result_value41.sroa.14.0.copyload381, %"#Attr_#dec_1.exit230" ], [ %result_value41.sroa.14.0544, %modify_rc.i.i304 ], [ %result_value41.sroa.14.0544, %766 ], [ %.sroa.1.0.copyload.i.i, %"#Attr_#dec_1.exit230.thread550" ]
  %result_value41.sroa.0.0542 = phi ptr [ %result_value41.sroa.0.0.pre, %"#Attr_#dec_1.exit230" ], [ %result_value41.sroa.0.0543, %modify_rc.i.i304 ], [ %result_value41.sroa.0.0543, %766 ], [ %.sroa.0.0.copyload, %"#Attr_#dec_1.exit230.thread550" ]
  %768 = icmp slt i64 %result_value41.sroa.25.0547, 0
  %769 = lshr i64 %result_value41.sroa.25.0547, 56
  %770 = and i64 %769, 127
  %771 = and i64 %result_value41.sroa.14.0545, 9223372036854775807
  %common.ret.op.i.i.i.i.i.i = select i1 %768, i64 %770, i64 %771
  %call.i.i.i.i239 = call i64 @llvm.usub.sat.i64(i64 %common.ret.op.i.i.i.i.i.i, i64 1)
  br i1 %is_big_str.i.i237548, label %modify_rc.i16.i.i.i.i, label %"#Attr_#inc_1.exit23.split.i.i.i.i"

modify_rc.i16.i.i.i.i:                            ; preds = %"#Attr_#inc_1.exit.i.i.i.i"
  %772 = ptrtoint ptr %result_value41.sroa.0.0542 to i64
  %773 = shl nuw i64 %result_value41.sroa.25.0547, 1
  %isneg.i.i.i20.i.i.i.i = icmp slt i64 %result_value41.sroa.14.0545, 0
  %774 = select i1 %isneg.i.i.i20.i.i.i.i, i64 %773, i64 %772
  %775 = inttoptr i64 %774 to ptr
  %get_rc_ptr.i21.i.i.i.i = getelementptr inbounds i64, ptr %775, i64 -1
  %776 = load i64, ptr %get_rc_ptr.i21.i.i.i.i, align 8
  %.not.i.i22.i.i.i.i = icmp eq i64 %776, 0
  br i1 %.not.i.i22.i.i.i.i, label %"#Attr_#inc_1.exit23.split.us.i.i.i.i", label %777

777:                                              ; preds = %modify_rc.i16.i.i.i.i
  %778 = atomicrmw add ptr %get_rc_ptr.i21.i.i.i.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit23.split.us.i.i.i.i"

"#Attr_#inc_1.exit23.split.us.i.i.i.i":           ; preds = %777, %modify_rc.i16.i.i.i.i
  %struct_field_gep4.i.i.i.i.i415 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 1
  %joinpoint_arg_alloca.sroa.7.0.struct_field_gep4.i.sroa_idx.i.i.i.i416 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 1, i32 1
  %joinpoint_arg_alloca.sroa.10.0.struct_field_gep4.i.sroa_idx.i.i.i.i417 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 1, i32 2
  %struct_field_gep5.i.i.i.i.i418 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 2
  %struct_field_gep6.i.i.i.i.i419 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 3
  %joinpoint_arg_alloca1.sroa.7.0.struct_field_gep6.i.sroa_idx.i.i.i.i420 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 3, i32 1
  %struct_field_gep7.i.i.i.i.i422 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 4
  br label %joinpointcont.us.i.i.i.i

joinpointcont.us.i.i.i.i:                         ; preds = %"#Attr_#inc_1.exit23.split.us.i.i.i.i", %then_block5.us.i.i.i.i
  %joinpointarg.us.i.i.i.i = phi i64 [ %struct_field_access_record_0.i.us.i.i.i.i, %then_block5.us.i.i.i.i ], [ %call.i.i.i.i239, %"#Attr_#inc_1.exit23.split.us.i.i.i.i" ]
  call void @llvm.lifetime.start.p0(i64 80, ptr nonnull %struct_alloca.i.i.i.i.i)
  %779 = load i64, ptr %get_rc_ptr.i21.i.i.i.i, align 8
  %.not.i.i.i.us.i.i.i.i = icmp eq i64 %779, 0
  br i1 %.not.i.i.i.us.i.i.i.i, label %"#Attr_#inc_1.exit.i.us.i.i.i.i", label %780

780:                                              ; preds = %joinpointcont.us.i.i.i.i
  %781 = atomicrmw add ptr %get_rc_ptr.i21.i.i.i.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit.i.us.i.i.i.i"

"#Attr_#inc_1.exit.i.us.i.i.i.i":                 ; preds = %780, %joinpointcont.us.i.i.i.i
  %call.i.i.us.i.i.i.i564 = add nuw i64 %joinpointarg.us.i.i.i.i, 1
  %x.y.i.i.us.i.i.i.i = call i64 @llvm.umin.i64(i64 %call.i.i.us.i.i.i.i564, i64 %common.ret.op.i.i.i.i.i.i)
  store i64 %x.y.i.i.us.i.i.i.i, ptr %struct_alloca.i.i.i.i.i, align 8
  store ptr %result_value41.sroa.0.0542, ptr %struct_field_gep4.i.i.i.i.i415, align 8
  store i64 %result_value41.sroa.14.0545, ptr %joinpoint_arg_alloca.sroa.7.0.struct_field_gep4.i.sroa_idx.i.i.i.i416, align 8
  store i64 %result_value41.sroa.25.0547, ptr %joinpoint_arg_alloca.sroa.10.0.struct_field_gep4.i.sroa_idx.i.i.i.i417, align 8
  store i64 %joinpointarg.us.i.i.i.i, ptr %struct_field_gep5.i.i.i.i.i418, align 8
  store ptr inttoptr (i64 41 to ptr), ptr %struct_field_gep6.i.i.i.i.i419, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %joinpoint_arg_alloca1.sroa.7.0.struct_field_gep6.i.sroa_idx.i.i.i.i420, align 8
  store <2 x i64> <i64 0, i64 1>, ptr %struct_field_gep7.i.i.i.i.i422, align 8
  %call9.i.us.i.i.i.i = call fastcc i1 @Str_matches_at_help_e3211b24ebda6e959f98c8dfabbb8bde232b75ae94930caa03d2bdb8e7b5674(ptr nonnull %struct_alloca.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 80, ptr nonnull %struct_alloca.i.i.i.i.i)
  br i1 %call9.i.us.i.i.i.i, label %"#Attr_#dec_1.exit.i.i.i.i", label %else_block.us.i.i.i.i

else_block.us.i.i.i.i:                            ; preds = %"#Attr_#inc_1.exit.i.us.i.i.i.i"
  %call.i.i45.us.i.i.i.i = call { i64, i1 } @llvm.usub.with.overflow.i64(i64 %joinpointarg.us.i.i.i.i, i64 1)
  %struct_field_access_record_1.i.us.i.i.i.i = extractvalue { i64, i1 } %call.i.i45.us.i.i.i.i, 1
  br i1 %struct_field_access_record_1.i.us.i.i.i.i, label %"#Attr_#dec_1.exit56.i.i.i.i", label %then_block5.us.i.i.i.i

then_block5.us.i.i.i.i:                           ; preds = %else_block.us.i.i.i.i
  %struct_field_access_record_0.i.us.i.i.i.i = extractvalue { i64, i1 } %call.i.i45.us.i.i.i.i, 0
  br label %joinpointcont.us.i.i.i.i

"#Attr_#inc_1.exit23.split.i.i.i.i":              ; preds = %"#Attr_#inc_1.exit.i.i.i.i"
  %struct_field_gep4.i.i.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 1
  %joinpoint_arg_alloca.sroa.7.0.struct_field_gep4.i.sroa_idx.i.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 1, i32 1
  %joinpoint_arg_alloca.sroa.10.0.struct_field_gep4.i.sroa_idx.i.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 1, i32 2
  %struct_field_gep5.i.i.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 2
  %struct_field_gep6.i.i.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 3
  %joinpoint_arg_alloca1.sroa.7.0.struct_field_gep6.i.sroa_idx.i.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 3, i32 1
  %struct_field_gep7.i.i.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i.i.i, i64 0, i32 4
  %782 = ptrtoint ptr %result_value41.sroa.0.0542 to i64
  %783 = shl nuw i64 %result_value41.sroa.25.0547, 1
  %isneg.i.i.i.i.i.i.i.i = icmp slt i64 %result_value41.sroa.14.0545, 0
  %784 = select i1 %isneg.i.i.i.i.i.i.i.i, i64 %783, i64 %782
  %785 = inttoptr i64 %784 to ptr
  %get_rc_ptr.i.i.i.i.i.i242 = getelementptr inbounds i64, ptr %785, i64 -1
  call void @llvm.lifetime.start.p0(i64 80, ptr nonnull %struct_alloca.i.i.i.i.i)
  %call.i.i.i.i.i.i485508 = add nuw nsw i64 %call.i.i.i.i239, 1
  %x.y.i.i.i.i.i.i509 = call i64 @llvm.umin.i64(i64 %call.i.i.i.i.i.i485508, i64 %common.ret.op.i.i.i.i.i.i)
  store i64 %x.y.i.i.i.i.i.i509, ptr %struct_alloca.i.i.i.i.i, align 8
  store ptr %result_value41.sroa.0.0542, ptr %struct_field_gep4.i.i.i.i.i, align 8
  store i64 %result_value41.sroa.14.0545, ptr %joinpoint_arg_alloca.sroa.7.0.struct_field_gep4.i.sroa_idx.i.i.i.i, align 8
  store i64 %result_value41.sroa.25.0547, ptr %joinpoint_arg_alloca.sroa.10.0.struct_field_gep4.i.sroa_idx.i.i.i.i, align 8
  store i64 %call.i.i.i.i239, ptr %struct_field_gep5.i.i.i.i.i, align 8
  store ptr inttoptr (i64 41 to ptr), ptr %struct_field_gep6.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %joinpoint_arg_alloca1.sroa.7.0.struct_field_gep6.i.sroa_idx.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 1>, ptr %struct_field_gep7.i.i.i.i.i, align 8
  %call9.i.i.i.i.i510 = call fastcc i1 @Str_matches_at_help_e3211b24ebda6e959f98c8dfabbb8bde232b75ae94930caa03d2bdb8e7b5674(ptr nonnull %struct_alloca.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 80, ptr nonnull %struct_alloca.i.i.i.i.i)
  br i1 %call9.i.i.i.i.i510, label %then_block.i.i, label %else_block.i.i.i.i243

"#Attr_#dec_1.exit.i.i.i.i":                      ; preds = %then_block5.i.i.i.i, %"#Attr_#inc_1.exit.i.us.i.i.i.i"
  %get_rc_ptr.i.i.i.i.i.i242442 = phi ptr [ %get_rc_ptr.i21.i.i.i.i, %"#Attr_#inc_1.exit.i.us.i.i.i.i" ], [ %get_rc_ptr.i.i.i.i.i.i242, %then_block5.i.i.i.i ]
  %isneg.i.i.i.i.i.i.i.i435 = phi i1 [ %isneg.i.i.i20.i.i.i.i, %"#Attr_#inc_1.exit.i.us.i.i.i.i" ], [ %isneg.i.i.i.i.i.i.i.i, %then_block5.i.i.i.i ]
  %786 = phi i64 [ %772, %"#Attr_#inc_1.exit.i.us.i.i.i.i" ], [ %782, %then_block5.i.i.i.i ]
  %.us-phi102150.i.i.i.i = phi i64 [ %joinpointarg.us.i.i.i.i, %"#Attr_#inc_1.exit.i.us.i.i.i.i" ], [ %struct_field_access_record_0.i.i.i.i.i, %then_block5.i.i.i.i ]
  br i1 %is_big_str.i.i237548, label %modify_rc.i37.i.i.i.i, label %then_block.i.i

modify_rc.i37.i.i.i.i:                            ; preds = %"#Attr_#dec_1.exit.i.i.i.i"
  %787 = load i64, ptr %get_rc_ptr.i.i.i.i.i.i242442, align 8
  %.not.i.i43.i.i.i.i = icmp eq i64 %787, 0
  br i1 %.not.i.i43.i.i.i.i, label %then_block.i.i, label %788

788:                                              ; preds = %modify_rc.i37.i.i.i.i
  %789 = atomicrmw sub ptr %get_rc_ptr.i.i.i.i.i.i242442, i64 1 monotonic, align 8
  %790 = icmp eq i64 %789, 1
  br i1 %790, label %Str_last_match_3bbacd33228bca14fe5573efe7278cde33c78fe9028ba98810cff368dece.exit.thread47.i.i, label %then_block.i.i

Str_last_match_3bbacd33228bca14fe5573efe7278cde33c78fe9028ba98810cff368dece.exit.thread47.i.i: ; preds = %788
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i.i.i.i242442, i32 8)
  br label %then_block.i.i

else_block.i.i.i.i243:                            ; preds = %"#Attr_#inc_1.exit23.split.i.i.i.i", %then_block5.i.i.i.i
  %joinpointarg.i.i.i.i511 = phi i64 [ %struct_field_access_record_0.i.i.i.i.i, %then_block5.i.i.i.i ], [ %call.i.i.i.i239, %"#Attr_#inc_1.exit23.split.i.i.i.i" ]
  %call.i.i45.i.i.i.i = call { i64, i1 } @llvm.usub.with.overflow.i64(i64 %joinpointarg.i.i.i.i511, i64 1)
  %struct_field_access_record_1.i.i.i.i.i = extractvalue { i64, i1 } %call.i.i45.i.i.i.i, 1
  br i1 %struct_field_access_record_1.i.i.i.i.i, label %"#Attr_#dec_1.exit56.i.i.i.i", label %then_block5.i.i.i.i

then_block5.i.i.i.i:                              ; preds = %else_block.i.i.i.i243
  %struct_field_access_record_0.i.i.i.i.i = extractvalue { i64, i1 } %call.i.i45.i.i.i.i, 0
  call void @llvm.lifetime.start.p0(i64 80, ptr nonnull %struct_alloca.i.i.i.i.i)
  %call.i.i.i.i.i.i485 = add nuw i64 %struct_field_access_record_0.i.i.i.i.i, 1
  %x.y.i.i.i.i.i.i = call i64 @llvm.umin.i64(i64 %call.i.i.i.i.i.i485, i64 %common.ret.op.i.i.i.i.i.i)
  store i64 %x.y.i.i.i.i.i.i, ptr %struct_alloca.i.i.i.i.i, align 8
  store ptr %result_value41.sroa.0.0542, ptr %struct_field_gep4.i.i.i.i.i, align 8
  store i64 %result_value41.sroa.14.0545, ptr %joinpoint_arg_alloca.sroa.7.0.struct_field_gep4.i.sroa_idx.i.i.i.i, align 8
  store i64 %result_value41.sroa.25.0547, ptr %joinpoint_arg_alloca.sroa.10.0.struct_field_gep4.i.sroa_idx.i.i.i.i, align 8
  store i64 %struct_field_access_record_0.i.i.i.i.i, ptr %struct_field_gep5.i.i.i.i.i, align 8
  store ptr inttoptr (i64 41 to ptr), ptr %struct_field_gep6.i.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 -9151314442816847872>, ptr %joinpoint_arg_alloca1.sroa.7.0.struct_field_gep6.i.sroa_idx.i.i.i.i, align 8
  store <2 x i64> <i64 0, i64 1>, ptr %struct_field_gep7.i.i.i.i.i, align 8
  %call9.i.i.i.i.i = call fastcc i1 @Str_matches_at_help_e3211b24ebda6e959f98c8dfabbb8bde232b75ae94930caa03d2bdb8e7b5674(ptr nonnull %struct_alloca.i.i.i.i.i)
  call void @llvm.lifetime.end.p0(i64 80, ptr nonnull %struct_alloca.i.i.i.i.i)
  br i1 %call9.i.i.i.i.i, label %"#Attr_#dec_1.exit.i.i.i.i", label %else_block.i.i.i.i243

"#Attr_#dec_1.exit56.i.i.i.i":                    ; preds = %else_block.i.i.i.i243, %else_block.us.i.i.i.i
  %get_rc_ptr.i.i.i.i.i.i242439 = phi ptr [ %get_rc_ptr.i21.i.i.i.i, %else_block.us.i.i.i.i ], [ %get_rc_ptr.i.i.i.i.i.i242, %else_block.i.i.i.i243 ]
  br i1 %is_big_str.i.i237548, label %modify_rc.i60.i.i.i.i, label %"#Attr_#dec_1.exit321"

modify_rc.i60.i.i.i.i:                            ; preds = %"#Attr_#dec_1.exit56.i.i.i.i"
  %791 = load i64, ptr %get_rc_ptr.i.i.i.i.i.i242439, align 8
  %.not.i.i66.i.i.i.i = icmp eq i64 %791, 0
  br i1 %.not.i.i66.i.i.i.i, label %"#Attr_#dec_1.exit321", label %792

792:                                              ; preds = %modify_rc.i60.i.i.i.i
  %793 = atomicrmw sub ptr %get_rc_ptr.i.i.i.i.i.i242439, i64 1 monotonic, align 8
  %794 = icmp eq i64 %793, 1
  br i1 %794, label %Str_last_match_3bbacd33228bca14fe5573efe7278cde33c78fe9028ba98810cff368dece.exit.i.i, label %modify_rc.i31.i.i257

Str_last_match_3bbacd33228bca14fe5573efe7278cde33c78fe9028ba98810cff368dece.exit.i.i: ; preds = %792
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i.i.i.i242439, i32 8)
  br label %modify_rc.i31.i.i257

then_block.i.i:                                   ; preds = %"#Attr_#inc_1.exit23.split.i.i.i.i", %Str_last_match_3bbacd33228bca14fe5573efe7278cde33c78fe9028ba98810cff368dece.exit.thread47.i.i, %788, %modify_rc.i37.i.i.i.i, %"#Attr_#dec_1.exit.i.i.i.i"
  %.us-phi102150.i.i.i.i563 = phi i64 [ %.us-phi102150.i.i.i.i, %Str_last_match_3bbacd33228bca14fe5573efe7278cde33c78fe9028ba98810cff368dece.exit.thread47.i.i ], [ %.us-phi102150.i.i.i.i, %788 ], [ %.us-phi102150.i.i.i.i, %modify_rc.i37.i.i.i.i ], [ %.us-phi102150.i.i.i.i, %"#Attr_#dec_1.exit.i.i.i.i" ], [ %call.i.i.i.i239, %"#Attr_#inc_1.exit23.split.i.i.i.i" ]
  %795 = phi i64 [ %786, %Str_last_match_3bbacd33228bca14fe5573efe7278cde33c78fe9028ba98810cff368dece.exit.thread47.i.i ], [ %786, %788 ], [ %786, %modify_rc.i37.i.i.i.i ], [ %786, %"#Attr_#dec_1.exit.i.i.i.i" ], [ %782, %"#Attr_#inc_1.exit23.split.i.i.i.i" ]
  %isneg.i.i.i.i.i.i.i.i435561 = phi i1 [ %isneg.i.i.i.i.i.i.i.i435, %Str_last_match_3bbacd33228bca14fe5573efe7278cde33c78fe9028ba98810cff368dece.exit.thread47.i.i ], [ %isneg.i.i.i.i.i.i.i.i435, %788 ], [ %isneg.i.i.i.i.i.i.i.i435, %modify_rc.i37.i.i.i.i ], [ %isneg.i.i.i.i.i.i.i.i435, %"#Attr_#dec_1.exit.i.i.i.i" ], [ %isneg.i.i.i.i.i.i.i.i, %"#Attr_#inc_1.exit23.split.i.i.i.i" ]
  %get_rc_ptr.i.i.i.i.i.i242442560 = phi ptr [ %get_rc_ptr.i.i.i.i.i.i242442, %Str_last_match_3bbacd33228bca14fe5573efe7278cde33c78fe9028ba98810cff368dece.exit.thread47.i.i ], [ %get_rc_ptr.i.i.i.i.i.i242442, %788 ], [ %get_rc_ptr.i.i.i.i.i.i242442, %modify_rc.i37.i.i.i.i ], [ %get_rc_ptr.i.i.i.i.i.i242442, %"#Attr_#dec_1.exit.i.i.i.i" ], [ %get_rc_ptr.i.i.i.i.i.i242, %"#Attr_#inc_1.exit23.split.i.i.i.i" ]
  %call.i.i.i263 = call { i64, i1 } @llvm.usub.with.overflow.i64(i64 %common.ret.op.i.i.i.i.i.i, i64 1)
  %has_overflowed.i.i.i = extractvalue { i64, i1 } %call.i.i.i263, 1
  br i1 %has_overflowed.i.i.i, label %throw_block.i.i.i, label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i264

throw_block.i.i.i:                                ; preds = %then_block.i.i
  call fastcc void @throw_on_overflow()
  unreachable

Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i264: ; preds = %then_block.i.i
  %operation_result.i.i.i = extractvalue { i64, i1 } %call.i.i.i263, 0
  %call.i21.i.i = call { i64, i1 } @llvm.usub.with.overflow.i64(i64 %operation_result.i.i.i, i64 %.us-phi102150.i.i.i.i563)
  %has_overflowed.i22.i.i = extractvalue { i64, i1 } %call.i21.i.i, 1
  br i1 %has_overflowed.i22.i.i, label %throw_block.i24.i.i, label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit25.i.i

throw_block.i24.i.i:                              ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i264
  call fastcc void @throw_on_overflow()
  unreachable

Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit25.i.i: ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit.i.i264
  %operation_result.i23.i.i = extractvalue { i64, i1 } %call.i21.i.i, 0
  br i1 %is_big_str.i.i237548, label %modify_rc.i.i.i295, label %"#Attr_#inc_1.exit.i.i"

modify_rc.i.i.i295:                               ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit25.i.i
  %796 = load i64, ptr %get_rc_ptr.i.i.i.i.i.i242442560, align 8
  %.not.i.i.i.i299 = icmp eq i64 %796, 0
  br i1 %.not.i.i.i.i299, label %"#Attr_#inc_1.exit.thread.i.i", label %"#Attr_#inc_1.exit.i.i.thread"

"#Attr_#inc_1.exit.i.i.thread":                   ; preds = %modify_rc.i.i.i295
  %797 = atomicrmw add ptr %get_rc_ptr.i.i.i.i.i.i242442560, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit.thread.i.i"

"#Attr_#inc_1.exit.i.i":                          ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit25.i.i
  br i1 %768, label %str.RocStr.asSlice.exit.i.i.i, label %"#Attr_#inc_1.exit.thread.i.i"

"#Attr_#inc_1.exit.thread.i.i":                   ; preds = %"#Attr_#inc_1.exit.i.i", %"#Attr_#inc_1.exit.i.i.thread", %modify_rc.i.i.i295
  %.sroa.1.0.copyload.i.i.i69.i.i = phi i64 [ %result_value41.sroa.25.0547, %modify_rc.i.i.i295 ], [ %result_value41.sroa.25.0547, %"#Attr_#inc_1.exit.i.i.thread" ], [ 0, %"#Attr_#inc_1.exit.i.i" ]
  %.not.i.i.i.i.i266 = icmp eq ptr %result_value41.sroa.0.0542, null
  br i1 %.not.i.i.i.i.i266, label %813, label %798

798:                                              ; preds = %"#Attr_#inc_1.exit.thread.i.i"
  %spec.select.i.i.i.i.i.i.i.i = select i1 %isneg.i.i.i.i.i.i.i.i435561, i64 %771, i64 %.sroa.1.0.copyload.i.i.i69.i.i
  %799 = icmp ne i64 %spec.select.i.i.i.i.i.i.i.i, 0
  %.not1.i.i.i.i.i.i.i = or i1 %isneg.i.i.i.i.i.i.i.i435561, %799
  br i1 %.not1.i.i.i.i.i.i.i, label %str.RocStr.isUnique.exit.i.i.i.i.i, label %str.RocStr.setLen.exit9.i.i.i.i.i

str.RocStr.isUnique.exit.i.i.i.i.i:               ; preds = %798
  %800 = shl nuw i64 %.sroa.1.0.copyload.i.i.i69.i.i, 1
  %801 = inttoptr i64 %800 to ptr
  %802 = select i1 %isneg.i.i.i.i.i.i.i.i435561, ptr %801, ptr %result_value41.sroa.0.0542
  %803 = getelementptr inbounds i64, ptr %802, i64 -1
  %804 = load i64, ptr %803, align 8, !noalias !134
  %805 = icmp eq i64 %804, 1
  br i1 %805, label %str.RocStr.setLen.exit9.i.i.i.i.i, label %.critedge.i.i.i.i.i

str.RocStr.setLen.exit9.i.i.i.i.i:                ; preds = %str.RocStr.isUnique.exit.i.i.i.i.i, %798
  %806 = and i64 %result_value41.sroa.14.0545, -9223372036854775808
  br label %.critedge.i.i.i

.critedge.i.i.i.i.i:                              ; preds = %str.RocStr.isUnique.exit.i.i.i.i.i
  %807 = lshr i64 %795, 1
  %808 = select i1 %isneg.i.i.i.i.i.i.i.i435561, i64 %.sroa.1.0.copyload.i.i.i69.i.i, i64 %807
  br label %.critedge.i.i.i

modify_rc.i31.i.i257:                             ; preds = %Str_last_match_3bbacd33228bca14fe5573efe7278cde33c78fe9028ba98810cff368dece.exit.i.i, %792
  %.pr = load i64, ptr %get_rc_ptr.i.i.i.i.i.i242439, align 8
  %.not.i.i37.i.i262 = icmp eq i64 %.pr, 0
  br i1 %.not.i.i37.i.i262, label %"#Attr_#dec_1.exit321", label %809

809:                                              ; preds = %modify_rc.i31.i.i257
  %810 = atomicrmw sub ptr %get_rc_ptr.i.i.i.i.i.i242439, i64 1 monotonic, align 8
  %811 = icmp eq i64 %810, 1
  br i1 %811, label %812, label %"#Attr_#dec_1.exit321"

812:                                              ; preds = %809
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i.i.i.i242439, i32 8)
  br label %"#Attr_#dec_1.exit321"

813:                                              ; preds = %"#Attr_#inc_1.exit.thread.i.i"
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %0), !noalias !139
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1), !noalias !139
  br label %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit

str.RocStr.asSlice.exit.i.i.i:                    ; preds = %"#Attr_#inc_1.exit.i.i"
  %814 = shl i64 %.us-phi102150.i.i.i.i563, 56
  %.sroa.4.23.insert.mask.i.i.i.i.i = and i64 %result_value41.sroa.25.0547, 72057594037927935
  %.sroa.4.23.insert.shift.i.i.i.i.i = or disjoint i64 %814, %.sroa.4.23.insert.mask.i.i.i.i.i
  %.sroa.4.23.insert.insert.i.i.i.i.i = or i64 %.sroa.4.23.insert.shift.i.i.i.i.i, -9223372036854775808
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %0), !noalias !139
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1), !noalias !139
  store ptr %result_value41.sroa.0.0542, ptr %1, align 8, !noalias !142
  %result_value41.sroa.14.0..sroa_idx = getelementptr inbounds i8, ptr %1, i64 8
  store i64 %result_value41.sroa.14.0545, ptr %result_value41.sroa.14.0..sroa_idx, align 8, !noalias !142
  %result_value41.sroa.25.0..sroa_idx = getelementptr inbounds i8, ptr %1, i64 16
  store i64 %result_value41.sroa.25.0547, ptr %result_value41.sroa.25.0..sroa_idx, align 8, !noalias !142
  %815 = getelementptr i8, ptr %1, i64 %.us-phi102150.i.i.i.i563
  %816 = getelementptr i8, ptr %815, i64 1
  %817 = icmp ugt i64 %operation_result.i23.i.i, 23
  br i1 %817, label %str.RocStr.allocate.exit.i.i.i.i.i, label %str.RocStr.allocate.exit.thread.i.i.i.i.i

str.RocStr.allocate.exit.thread.i.i.i.i.i:        ; preds = %str.RocStr.asSlice.exit.i.i.i
  %.sroa.3.23.insert.ext.i.i.i.i.i = shl nuw nsw i64 %operation_result.i23.i.i, 56
  %.sroa.3.23.insert.shift.i.i.i.i.i = or disjoint i64 %.sroa.3.23.insert.ext.i.i.i.i.i, -9223372036854775808
  br label %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit.thread

str.RocStr.allocate.exit.i.i.i.i.i:               ; preds = %str.RocStr.asSlice.exit.i.i.i
  %818 = add nuw i64 %operation_result.i23.i.i, 8
  %819 = call ptr @roc_alloc(i64 %818, i32 8), !noalias !145
  %820 = icmp ne ptr %819, null
  call void @llvm.assume(i1 %820)
  %821 = getelementptr inbounds i8, ptr %819, i64 8
  store i64 1, ptr %819, align 8, !noalias !145
  %822 = icmp slt i64 %operation_result.i23.i.i, 0
  %spec.select.i.i.i.i.i = select i1 %822, ptr %0, ptr %821
  br label %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit.thread

.critedge.i.i.i:                                  ; preds = %str.RocStr.setLen.exit9.i.i.i.i.i, %.critedge.i.i.i.i.i
  %.sroa.9.0.i.i.i.i.ph.ph = phi i64 [ %.sroa.1.0.copyload.i.i.i69.i.i, %str.RocStr.setLen.exit9.i.i.i.i.i ], [ %808, %.critedge.i.i.i.i.i ]
  %.pn = phi i64 [ %806, %str.RocStr.setLen.exit9.i.i.i.i.i ], [ -9223372036854775808, %.critedge.i.i.i.i.i ]
  %.sroa.6.0.i.i.i.i.ph.ph = or i64 %.us-phi102150.i.i.i.i563, %.pn
  %add_int_wrap.i.i.i452466 = add nuw i64 %.us-phi102150.i.i.i.i563, 1
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %0), !noalias !139
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1), !noalias !139
  %823 = lshr i64 %795, 1
  %824 = select i1 %isneg.i.i.i.i.i.i.i.i435561, i64 %result_value41.sroa.25.0547, i64 %823
  %825 = getelementptr inbounds i8, ptr %result_value41.sroa.0.0542, i64 %add_int_wrap.i.i.i452466
  %826 = or i64 %operation_result.i23.i.i, -9223372036854775808
  %827 = ptrtoint ptr %825 to i64
  br label %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit

Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit.thread: ; preds = %str.RocStr.allocate.exit.thread.i.i.i.i.i, %str.RocStr.allocate.exit.i.i.i.i.i
  %.sink7.i.i.i.i.i = phi ptr [ %821, %str.RocStr.allocate.exit.i.i.i.i.i ], [ null, %str.RocStr.allocate.exit.thread.i.i.i.i.i ]
  %.sink.i.i.i.i.i354 = phi i64 [ %operation_result.i23.i.i, %str.RocStr.allocate.exit.i.i.i.i.i ], [ 0, %str.RocStr.allocate.exit.thread.i.i.i.i.i ]
  %.sroa.3.23.insert.shift.sink.i.i.i.i.i = phi i64 [ %operation_result.i23.i.i, %str.RocStr.allocate.exit.i.i.i.i.i ], [ %.sroa.3.23.insert.shift.i.i.i.i.i, %str.RocStr.allocate.exit.thread.i.i.i.i.i ]
  %common.ret.op.i.i.i.i.i.i355 = phi ptr [ %spec.select.i.i.i.i.i, %str.RocStr.allocate.exit.i.i.i.i.i ], [ %0, %str.RocStr.allocate.exit.thread.i.i.i.i.i ]
  store ptr %.sink7.i.i.i.i.i, ptr %0, align 8, !noalias !142
  %828 = getelementptr inbounds i8, ptr %0, i64 8
  store i64 %.sink.i.i.i.i.i354, ptr %828, align 8, !noalias !142
  %829 = getelementptr inbounds i8, ptr %0, i64 16
  store i64 %.sroa.3.23.insert.shift.sink.i.i.i.i.i, ptr %829, align 8, !noalias !142
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %common.ret.op.i.i.i.i.i.i355, ptr nonnull align 1 %816, i64 %operation_result.i23.i.i, i1 false), !noalias !142
  %.sroa.0.0.copyload2.i.i = load i64, ptr %0, align 8, !noalias !139
  %.sroa.6.0.copyload5.i.i = load i64, ptr %828, align 8, !noalias !139
  %.sroa.9.0.copyload8.i.i = load i64, ptr %829, align 8, !noalias !139
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %0), !noalias !139
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1), !noalias !139
  br label %"#Attr_#dec_1.exit.i270"

Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit: ; preds = %813, %.critedge.i.i.i
  %.sroa.0.0.i.i.i.i460 = phi i64 [ %795, %.critedge.i.i.i ], [ 0, %813 ]
  %.sroa.6.0.i.i.i.i458 = phi i64 [ %.sroa.6.0.i.i.i.i.ph.ph, %.critedge.i.i.i ], [ 0, %813 ]
  %.sroa.9.0.i.i.i.i456 = phi i64 [ %.sroa.9.0.i.i.i.i.ph.ph, %.critedge.i.i.i ], [ -9223372036854775808, %813 ]
  %.sroa.9.0.i.i = phi i64 [ %824, %.critedge.i.i.i ], [ -9223372036854775808, %813 ]
  %.sroa.6.0.i.i = phi i64 [ %826, %.critedge.i.i.i ], [ 0, %813 ]
  %.sroa.0.0.i.i = phi i64 [ %827, %.critedge.i.i.i ], [ 0, %813 ]
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %0), !noalias !139
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1), !noalias !139
  br i1 %is_big_str.i.i237548, label %modify_rc.i8.i288, label %"#Attr_#dec_1.exit.i270"

modify_rc.i8.i288:                                ; preds = %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit
  %830 = load i64, ptr %get_rc_ptr.i.i.i.i.i.i242442560, align 8
  %.not.i.i14.i293 = icmp eq i64 %830, 0
  br i1 %.not.i.i14.i293, label %"#Attr_#dec_1.exit.i270", label %831

831:                                              ; preds = %modify_rc.i8.i288
  %832 = atomicrmw sub ptr %get_rc_ptr.i.i.i.i.i.i242442560, i64 1 monotonic, align 8
  %833 = icmp eq i64 %832, 1
  br i1 %833, label %834, label %"#Attr_#dec_1.exit.i270"

834:                                              ; preds = %831
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i.i.i.i242442560, i32 8)
  br label %"#Attr_#dec_1.exit.i270"

"#Attr_#dec_1.exit.i270":                         ; preds = %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit.thread, %834, %831, %modify_rc.i8.i288, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit
  %.sroa.0.0.i.i484 = phi i64 [ %.sroa.0.0.copyload2.i.i, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit.thread ], [ %.sroa.0.0.i.i, %834 ], [ %.sroa.0.0.i.i, %831 ], [ %.sroa.0.0.i.i, %modify_rc.i8.i288 ], [ %.sroa.0.0.i.i, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit ]
  %.sroa.6.0.i.i483 = phi i64 [ %.sroa.6.0.copyload5.i.i, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit.thread ], [ %.sroa.6.0.i.i, %834 ], [ %.sroa.6.0.i.i, %831 ], [ %.sroa.6.0.i.i, %modify_rc.i8.i288 ], [ %.sroa.6.0.i.i, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit ]
  %.sroa.9.0.i.i482 = phi i64 [ %.sroa.9.0.copyload8.i.i, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit.thread ], [ %.sroa.9.0.i.i, %834 ], [ %.sroa.9.0.i.i, %831 ], [ %.sroa.9.0.i.i, %modify_rc.i8.i288 ], [ %.sroa.9.0.i.i, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit ]
  %.sroa.9.0.i.i.i.i456481 = phi i64 [ %.sroa.4.23.insert.insert.i.i.i.i.i, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit.thread ], [ %.sroa.9.0.i.i.i.i456, %834 ], [ %.sroa.9.0.i.i.i.i456, %831 ], [ %.sroa.9.0.i.i.i.i456, %modify_rc.i8.i288 ], [ %.sroa.9.0.i.i.i.i456, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit ]
  %.sroa.6.0.i.i.i.i458480 = phi i64 [ %result_value41.sroa.14.0545, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit.thread ], [ %.sroa.6.0.i.i.i.i458, %834 ], [ %.sroa.6.0.i.i.i.i458, %831 ], [ %.sroa.6.0.i.i.i.i458, %modify_rc.i8.i288 ], [ %.sroa.6.0.i.i.i.i458, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit ]
  %.sroa.0.0.i.i.i.i460479 = phi i64 [ %795, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit.thread ], [ %.sroa.0.0.i.i.i.i460, %834 ], [ %.sroa.0.0.i.i.i.i460, %831 ], [ %.sroa.0.0.i.i.i.i460, %modify_rc.i8.i288 ], [ %.sroa.0.0.i.i.i.i460, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit ]
  %835 = inttoptr i64 %.sroa.0.0.i.i484 to ptr
  store ptr %835, ptr %struct_field.i234, align 8
  %load_element.sroa.2.0.struct_field.sroa_idx.i = getelementptr inbounds i8, ptr %struct_field.i234, i64 8
  store i64 %.sroa.6.0.i.i483, ptr %load_element.sroa.2.0.struct_field.sroa_idx.i, align 8
  %load_element.sroa.3.0.struct_field.sroa_idx.i = getelementptr inbounds i8, ptr %struct_field.i234, i64 16
  store i64 %.sroa.9.0.i.i482, ptr %load_element.sroa.3.0.struct_field.sroa_idx.i, align 8
  store i64 %.sroa.0.0.i.i.i.i460479, ptr %struct_field2.i233, align 8
  %load_element.sroa.6.24.struct_field2.sroa_idx.i = getelementptr inbounds i8, ptr %struct_field2.i233, i64 8
  store i64 %.sroa.6.0.i.i.i.i458480, ptr %load_element.sroa.6.24.struct_field2.sroa_idx.i, align 8
  %load_element.sroa.7.24.struct_field2.sroa_idx.i = getelementptr inbounds i8, ptr %struct_field2.i233, i64 16
  store i64 %.sroa.9.0.i.i.i.i456481, ptr %load_element.sroa.7.24.struct_field2.sroa_idx.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %const_str_store43, ptr nonnull %struct_field.i234, ptr nonnull %result_value3.i232)
  %is_big_str.i17.i271 = icmp sgt i64 %.sroa.9.0.i.i482, 0
  br i1 %is_big_str.i17.i271, label %modify_rc.i18.i284, label %"#Attr_#dec_1.exit25.i272"

modify_rc.i18.i284:                               ; preds = %"#Attr_#dec_1.exit.i270"
  %836 = shl nuw i64 %.sroa.9.0.i.i482, 1
  %isneg.i.i.i22.i285 = icmp slt i64 %.sroa.6.0.i.i483, 0
  %837 = select i1 %isneg.i.i.i22.i285, i64 %836, i64 %.sroa.0.0.i.i484
  %838 = inttoptr i64 %837 to ptr
  %get_rc_ptr.i23.i286 = getelementptr inbounds i64, ptr %838, i64 -1
  %839 = load i64, ptr %get_rc_ptr.i23.i286, align 8
  %.not.i.i24.i287 = icmp eq i64 %839, 0
  br i1 %.not.i.i24.i287, label %"#Attr_#dec_1.exit25.i272", label %840

840:                                              ; preds = %modify_rc.i18.i284
  %841 = atomicrmw sub ptr %get_rc_ptr.i23.i286, i64 1 monotonic, align 8
  %842 = icmp eq i64 %841, 1
  br i1 %842, label %843, label %"#Attr_#dec_1.exit25.i272"

843:                                              ; preds = %840
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i23.i286, i32 8)
  br label %"#Attr_#dec_1.exit25.i272"

"#Attr_#dec_1.exit25.i272":                       ; preds = %843, %840, %modify_rc.i18.i284, %"#Attr_#dec_1.exit.i270"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %struct_field2.i233, ptr nonnull %result_value3.i232, ptr nonnull %result_value4.i231)
  %load_str_to_stack.elt3.i26.i273 = getelementptr inbounds %str.RocStr, ptr %result_value3.i232, i64 0, i32 2
  %load_str_to_stack.unpack4.i27.i274 = load i64, ptr %load_str_to_stack.elt3.i26.i273, align 8
  %is_big_str.i28.i275 = icmp sgt i64 %load_str_to_stack.unpack4.i27.i274, 0
  br i1 %is_big_str.i28.i275, label %modify_rc.i29.i277, label %"#Attr_#dec_1.exit36.i276"

modify_rc.i29.i277:                               ; preds = %"#Attr_#dec_1.exit25.i272"
  %.sroa.0.0.copyload.i.i30.i278 = load ptr, ptr %result_value3.i232, align 8
  %.sroa.2.0..sroa_idx.i.i31.i279 = getelementptr inbounds i8, ptr %result_value3.i232, i64 8
  %.sroa.2.0.copyload.i.i32.i280 = load i64, ptr %.sroa.2.0..sroa_idx.i.i31.i279, align 8
  %844 = ptrtoint ptr %.sroa.0.0.copyload.i.i30.i278 to i64
  %845 = shl nuw i64 %load_str_to_stack.unpack4.i27.i274, 1
  %isneg.i.i.i33.i281 = icmp slt i64 %.sroa.2.0.copyload.i.i32.i280, 0
  %846 = select i1 %isneg.i.i.i33.i281, i64 %845, i64 %844
  %847 = inttoptr i64 %846 to ptr
  %get_rc_ptr.i34.i282 = getelementptr inbounds i64, ptr %847, i64 -1
  %848 = load i64, ptr %get_rc_ptr.i34.i282, align 8
  %.not.i.i35.i283 = icmp eq i64 %848, 0
  br i1 %.not.i.i35.i283, label %"#Attr_#dec_1.exit36.i276", label %849

849:                                              ; preds = %modify_rc.i29.i277
  %850 = atomicrmw sub ptr %get_rc_ptr.i34.i282, i64 1 monotonic, align 8
  %851 = icmp eq i64 %850, 1
  br i1 %851, label %852, label %"#Attr_#dec_1.exit36.i276"

852:                                              ; preds = %849
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i34.i282, i32 8)
  br label %"#Attr_#dec_1.exit36.i276"

"#Attr_#dec_1.exit36.i276":                       ; preds = %852, %849, %modify_rc.i29.i277, %"#Attr_#dec_1.exit25.i272"
  %result_value44.sroa.0.0.copyload401 = load ptr, ptr %result_value4.i231, align 8
  %result_value44.sroa.3.0.result_value4.i231.sroa_idx = getelementptr inbounds i8, ptr %result_value4.i231, i64 8
  %result_value44.sroa.3.0.copyload402 = load i64, ptr %result_value44.sroa.3.0.result_value4.i231.sroa_idx, align 8
  %result_value44.sroa.4.0.result_value4.i231.sroa_idx = getelementptr inbounds i8, ptr %result_value4.i231, i64 16
  %result_value44.sroa.4.0.copyload403 = load i64, ptr %result_value44.sroa.4.0.result_value4.i231.sroa_idx, align 8
  br label %"#Attr_#dec_1.exit321"

"#Attr_#dec_1.exit321":                           ; preds = %modify_rc.i60.i.i.i.i, %"#Attr_#dec_1.exit36.i276", %modify_rc.i31.i.i257, %809, %812, %"#Attr_#dec_1.exit56.i.i.i.i"
  %result_value44.sroa.0.0 = phi ptr [ %result_value44.sroa.0.0.copyload401, %"#Attr_#dec_1.exit36.i276" ], [ %result_value41.sroa.0.0542, %modify_rc.i31.i.i257 ], [ %result_value41.sroa.0.0542, %809 ], [ %result_value41.sroa.0.0542, %812 ], [ %result_value41.sroa.0.0542, %"#Attr_#dec_1.exit56.i.i.i.i" ], [ %result_value41.sroa.0.0542, %modify_rc.i60.i.i.i.i ]
  %result_value44.sroa.3.0 = phi i64 [ %result_value44.sroa.3.0.copyload402, %"#Attr_#dec_1.exit36.i276" ], [ %result_value41.sroa.14.0545, %modify_rc.i31.i.i257 ], [ %result_value41.sroa.14.0545, %809 ], [ %result_value41.sroa.14.0545, %812 ], [ %result_value41.sroa.14.0545, %"#Attr_#dec_1.exit56.i.i.i.i" ], [ %result_value41.sroa.14.0545, %modify_rc.i60.i.i.i.i ]
  %result_value44.sroa.4.0 = phi i64 [ %result_value44.sroa.4.0.copyload403, %"#Attr_#dec_1.exit36.i276" ], [ %result_value41.sroa.25.0547, %modify_rc.i31.i.i257 ], [ %result_value41.sroa.25.0547, %809 ], [ %result_value41.sroa.25.0547, %812 ], [ %result_value41.sroa.25.0547, %"#Attr_#dec_1.exit56.i.i.i.i" ], [ %result_value41.sroa.25.0547, %modify_rc.i60.i.i.i.i ]
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value4.i231)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value3.i232)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %struct_field2.i233)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %struct_field.i234)
  store ptr %result_value44.sroa.0.0, ptr %joinpoint_arg_alloca, align 8
  %tmp_output_for_jmp.sroa.2.0.joinpoint_arg_alloca.sroa_idx = getelementptr inbounds i8, ptr %joinpoint_arg_alloca, i64 8
  store i64 %result_value44.sroa.3.0, ptr %tmp_output_for_jmp.sroa.2.0.joinpoint_arg_alloca.sroa_idx, align 8
  %tmp_output_for_jmp.sroa.3.0.joinpoint_arg_alloca.sroa_idx = getelementptr inbounds i8, ptr %joinpoint_arg_alloca, i64 16
  store i64 %result_value44.sroa.4.0, ptr %tmp_output_for_jmp.sroa.3.0.joinpoint_arg_alloca.sroa_idx, align 8
  br label %joinpointcont

else_block38:                                     ; preds = %then_block31, %Inspect_207_46df57811ab9544a1f9313aab57b6ce51825f299caf5a1ffe2569412f3ed972.exit.i.i.i.i.i.i.i, %.lr.ph.i.i.cont
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %joinpoint_arg_alloca, ptr noundef nonnull align 8 dereferenceable(24) %result_value27, i64 24, i1 false)
  %load_str_to_stack.elt3.i322.phi.trans.insert = getelementptr inbounds %str.RocStr, ptr %joinpoint_arg_alloca, i64 0, i32 2
  %load_str_to_stack.unpack4.i323.pre = load i64, ptr %load_str_to_stack.elt3.i322.phi.trans.insert, align 8
  br label %joinpointcont

joinpointcont:                                    ; preds = %else_block38, %"#Attr_#dec_1.exit321"
  %load_str_to_stack.unpack4.i323 = phi i64 [ %load_str_to_stack.unpack4.i323.pre, %else_block38 ], [ %result_value44.sroa.4.0, %"#Attr_#dec_1.exit321" ]
  store ptr getelementptr inbounds ([41 x i8], ptr @_str_literal_14520704672829006296, i64 0, i64 8), ptr %const_str_store46, align 8
  %.fca.1.gep65 = getelementptr inbounds %str.RocStr, ptr %const_str_store46, i64 0, i32 1
  store <2 x i64> <i64 33, i64 33>, ptr %.fca.1.gep65, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %const_str_store46, ptr nonnull %joinpoint_arg_alloca, ptr nonnull %result_value47)
  %is_big_str.i324 = icmp sgt i64 %load_str_to_stack.unpack4.i323, 0
  br i1 %is_big_str.i324, label %modify_rc.i325, label %"#Attr_#dec_1.exit332"

modify_rc.i325:                                   ; preds = %joinpointcont
  %.sroa.0.0.copyload.i.i326 = load ptr, ptr %joinpoint_arg_alloca, align 8
  %.sroa.2.0..sroa_idx.i.i327 = getelementptr inbounds i8, ptr %joinpoint_arg_alloca, i64 8
  %.sroa.2.0.copyload.i.i328 = load i64, ptr %.sroa.2.0..sroa_idx.i.i327, align 8
  %853 = ptrtoint ptr %.sroa.0.0.copyload.i.i326 to i64
  %854 = shl nuw i64 %load_str_to_stack.unpack4.i323, 1
  %isneg.i.i.i329 = icmp slt i64 %.sroa.2.0.copyload.i.i328, 0
  %855 = select i1 %isneg.i.i.i329, i64 %854, i64 %853
  %856 = inttoptr i64 %855 to ptr
  %get_rc_ptr.i330 = getelementptr inbounds i64, ptr %856, i64 -1
  %857 = load i64, ptr %get_rc_ptr.i330, align 8
  %.not.i.i331 = icmp eq i64 %857, 0
  br i1 %.not.i.i331, label %"#Attr_#dec_1.exit332", label %858

858:                                              ; preds = %modify_rc.i325
  %859 = atomicrmw sub ptr %get_rc_ptr.i330, i64 1 monotonic, align 8
  %860 = icmp eq i64 %859, 1
  br i1 %860, label %861, label %"#Attr_#dec_1.exit332"

861:                                              ; preds = %858
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i330, i32 8)
  br label %"#Attr_#dec_1.exit332"

"#Attr_#dec_1.exit332":                           ; preds = %joinpointcont, %modify_rc.i325, %858, %861
  call fastcc void @"Stderr_line!_9d55cb5018ba494bcf5765953355c83e4ade148e5c95b280aacd826c59bea86"(ptr nonnull %result_value47, ptr nonnull %result_value48)
  %tag_id_ptr.i333 = getelementptr inbounds { [4 x i64], i8 }, ptr %result_value48, i64 0, i32 1
  %load_tag_id.i334 = load i8, ptr %tag_id_ptr.i333, align 8
  %cond.i335 = icmp eq i8 %load_tag_id.i334, 0
  %tag_id_ptr.i.i337 = getelementptr inbounds { [3 x i64], i8 }, ptr %result_value48, i64 0, i32 1
  %load_tag_id.i.i338 = load i8, ptr %tag_id_ptr.i.i337, align 8
  %cond.i.i339 = icmp eq i8 %load_tag_id.i.i338, 4
  %or.cond = select i1 %cond.i335, i1 %cond.i.i339, i1 false
  br i1 %or.cond, label %tag_id_modify.i.i340, label %common.ret

tag_id_modify.i.i340:                             ; preds = %"#Attr_#dec_1.exit332"
  %load_str_to_stack.elt3.i.i.i341 = getelementptr inbounds %str.RocStr, ptr %result_value48, i64 0, i32 2
  %load_str_to_stack.unpack4.i.i.i342 = load i64, ptr %load_str_to_stack.elt3.i.i.i341, align 8
  %is_big_str.i.i.i343 = icmp sgt i64 %load_str_to_stack.unpack4.i.i.i342, 0
  br i1 %is_big_str.i.i.i343, label %modify_rc.i.i.i344, label %common.ret

modify_rc.i.i.i344:                               ; preds = %tag_id_modify.i.i340
  %.sroa.0.0.copyload.i.i.i.i345 = load ptr, ptr %result_value48, align 8
  %.sroa.2.0..sroa_idx.i.i.i.i346 = getelementptr inbounds i8, ptr %result_value48, i64 8
  %.sroa.2.0.copyload.i.i.i.i347 = load i64, ptr %.sroa.2.0..sroa_idx.i.i.i.i346, align 8
  %862 = ptrtoint ptr %.sroa.0.0.copyload.i.i.i.i345 to i64
  %863 = shl nuw i64 %load_str_to_stack.unpack4.i.i.i342, 1
  %isneg.i.i.i.i.i348 = icmp slt i64 %.sroa.2.0.copyload.i.i.i.i347, 0
  %864 = select i1 %isneg.i.i.i.i.i348, i64 %863, i64 %862
  %865 = inttoptr i64 %864 to ptr
  %get_rc_ptr.i.i.i349 = getelementptr inbounds i64, ptr %865, i64 -1
  %866 = load i64, ptr %get_rc_ptr.i.i.i349, align 8
  %.not.i.i.i.i350 = icmp eq i64 %866, 0
  br i1 %.not.i.i.i.i350, label %common.ret, label %867

867:                                              ; preds = %modify_rc.i.i.i344
  %868 = atomicrmw sub ptr %get_rc_ptr.i.i.i349, i64 1 monotonic, align 8
  %869 = icmp eq i64 %868, 1
  br i1 %869, label %870, label %common.ret

870:                                              ; preds = %867
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i349, i32 8)
  br label %common.ret
}

define void @roc__main_for_host_1_exposed_generic(ptr nocapture writeonly %0, ptr nocapture readonly %1) local_unnamed_addr {
entry:
  %load_arg.unpack = load ptr, ptr %1, align 8
  %2 = insertvalue %list.RocList poison, ptr %load_arg.unpack, 0
  %load_arg.elt1 = getelementptr inbounds %list.RocList, ptr %1, i64 0, i32 1
  %load_arg.unpack2 = load i64, ptr %load_arg.elt1, align 8
  %3 = insertvalue %list.RocList %2, i64 %load_arg.unpack2, 1
  %load_arg.elt3 = getelementptr inbounds %list.RocList, ptr %1, i64 0, i32 2
  %load_arg.unpack4 = load i64, ptr %load_arg.elt3, align 8
  %load_arg5 = insertvalue %list.RocList %3, i64 %load_arg.unpack4, 2
  %call = tail call fastcc i32 @"_main_for_host!_8445464738218c426e3b841f28ede2879d4391e6c33df6a616e872e199e47e"(%list.RocList %load_arg5)
  store i32 %call, ptr %0, align 4
  ret void
}

define noundef i32 @roc__main_for_host_1_exposed(ptr nocapture readonly %0) local_unnamed_addr {
entry:
  %load_arg.unpack = load ptr, ptr %0, align 8
  %1 = insertvalue %list.RocList poison, ptr %load_arg.unpack, 0
  %load_arg.elt1 = getelementptr inbounds %list.RocList, ptr %0, i64 0, i32 1
  %load_arg.unpack2 = load i64, ptr %load_arg.elt1, align 8
  %2 = insertvalue %list.RocList %1, i64 %load_arg.unpack2, 1
  %load_arg.elt3 = getelementptr inbounds %list.RocList, ptr %0, i64 0, i32 2
  %load_arg.unpack4 = load i64, ptr %load_arg.elt3, align 8
  %load_arg5 = insertvalue %list.RocList %2, i64 %load_arg.unpack4, 2
  %call = tail call fastcc i32 @"_main_for_host!_8445464738218c426e3b841f28ede2879d4391e6c33df6a616e872e199e47e"(%list.RocList %load_arg5)
  ret i32 %call
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none)
define noundef i64 @roc__main_for_host_1_exposed_size() local_unnamed_addr #9 {
entry:
  ret i64 4
}

define internal fastcc void @"Stderr_line!_9d55cb5018ba494bcf5765953355c83e4ade148e5c95b280aacd826c59bea86"(ptr %str, ptr nocapture writeonly %0) unnamed_addr {
entry:
  %result_value.i = alloca { [4 x i64], i8 }, align 8
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %result_value.i)
  call void @roc_fx_stderr_line(ptr nonnull %result_value.i, ptr %str)
  %load_str_to_stack.elt3.i.i = getelementptr inbounds %str.RocStr, ptr %str, i64 0, i32 2
  %load_str_to_stack.unpack4.i.i = load i64, ptr %load_str_to_stack.elt3.i.i, align 8
  %is_big_str.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i.i, 0
  br i1 %is_big_str.i.i, label %modify_rc.i.i, label %"Host_stderr_line!_dbefccae6de790f8e3497ad3c6c1c58a12a744edb0d65ec1ec4ade8b1151a59b.exit"

modify_rc.i.i:                                    ; preds = %entry
  %.sroa.0.0.copyload.i.i.i = load ptr, ptr %str, align 8
  %.sroa.2.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %str, i64 8
  %.sroa.2.0.copyload.i.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i.i, align 8
  %1 = ptrtoint ptr %.sroa.0.0.copyload.i.i.i to i64
  %2 = shl nuw i64 %load_str_to_stack.unpack4.i.i, 1
  %isneg.i.i.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i.i, 0
  %3 = select i1 %isneg.i.i.i.i, i64 %2, i64 %1
  %4 = inttoptr i64 %3 to ptr
  %get_rc_ptr.i.i = getelementptr inbounds i64, ptr %4, i64 -1
  %5 = load i64, ptr %get_rc_ptr.i.i, align 8
  %.not.i.i.i = icmp eq i64 %5, 0
  br i1 %.not.i.i.i, label %"Host_stderr_line!_dbefccae6de790f8e3497ad3c6c1c58a12a744edb0d65ec1ec4ade8b1151a59b.exit", label %6

6:                                                ; preds = %modify_rc.i.i
  %7 = atomicrmw sub ptr %get_rc_ptr.i.i, i64 1 monotonic, align 8
  %8 = icmp eq i64 %7, 1
  br i1 %8, label %9, label %"Host_stderr_line!_dbefccae6de790f8e3497ad3c6c1c58a12a744edb0d65ec1ec4ade8b1151a59b.exit"

9:                                                ; preds = %6
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i, i32 8)
  br label %"Host_stderr_line!_dbefccae6de790f8e3497ad3c6c1c58a12a744edb0d65ec1ec4ade8b1151a59b.exit"

"Host_stderr_line!_dbefccae6de790f8e3497ad3c6c1c58a12a744edb0d65ec1ec4ade8b1151a59b.exit": ; preds = %entry, %modify_rc.i.i, %6, %9
  %result_value.sroa.0.0.copyload = load ptr, ptr %result_value.i, align 8
  %result_value.sroa.2.0.result_value.i.sroa_idx = getelementptr inbounds i8, ptr %result_value.i, i64 8
  %result_value.sroa.2.0.copyload = load i64, ptr %result_value.sroa.2.0.result_value.i.sroa_idx, align 8
  %result_value.sroa.3.0.result_value.i.sroa_idx = getelementptr inbounds i8, ptr %result_value.i, i64 16
  %result_value.sroa.3.0.copyload = load i64, ptr %result_value.sroa.3.0.result_value.i.sroa_idx, align 8
  %result_value.sroa.4.0.result_value.i.sroa_idx = getelementptr inbounds i8, ptr %result_value.i, i64 24
  %result_value.sroa.4.0.copyload = load i8, ptr %result_value.sroa.4.0.result_value.i.sroa_idx, align 8
  %result_value.sroa.53.0.result_value.i.sroa_idx = getelementptr inbounds i8, ptr %result_value.i, i64 32
  %result_value.sroa.53.0.copyload = load i8, ptr %result_value.sroa.53.0.result_value.i.sroa_idx, align 8
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %result_value.i)
  %eq_u8.i = icmp eq i8 %result_value.sroa.53.0.copyload, 1
  br i1 %eq_u8.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit, label %else_block.i

else_block.i:                                     ; preds = %"Host_stderr_line!_dbefccae6de790f8e3497ad3c6c1c58a12a744edb0d65ec1ec4ade8b1151a59b.exit"
  switch i8 %result_value.sroa.4.0.copyload, label %default.i.i [
    i8 4, label %branch4.i.i
    i8 7, label %branch7.i.i
    i8 1, label %branch1.i.i
    i8 0, label %branch0.i.i
    i8 3, label %branch3.i.i
    i8 8, label %branch8.i.i
    i8 6, label %branch6.i.i
    i8 5, label %branch5.i.i
  ]

default.i.i:                                      ; preds = %else_block.i
  %10 = ptrtoint ptr %result_value.sroa.0.0.copyload to i64
  br label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

branch4.i.i:                                      ; preds = %else_block.i
  %is_big_str.i.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload, 0
  br i1 %is_big_str.i.i.i, label %modify_rc.i.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

modify_rc.i.i.i:                                  ; preds = %branch4.i.i
  %11 = ptrtoint ptr %result_value.sroa.0.0.copyload to i64
  %12 = shl nuw i64 %result_value.sroa.3.0.copyload, 1
  %isneg.i.i.i.i.i = icmp slt i64 %result_value.sroa.2.0.copyload, 0
  %13 = select i1 %isneg.i.i.i.i.i, i64 %12, i64 %11
  %14 = inttoptr i64 %13 to ptr
  %get_rc_ptr.i.i.i = getelementptr inbounds i64, ptr %14, i64 -1
  %15 = load i64, ptr %get_rc_ptr.i.i.i, align 8
  %.not.i.i.i.i = icmp eq i64 %15, 0
  br i1 %.not.i.i.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit, label %16

16:                                               ; preds = %modify_rc.i.i.i
  %17 = atomicrmw sub ptr %get_rc_ptr.i.i.i, i64 1 monotonic, align 8
  %18 = icmp eq i64 %17, 1
  br i1 %18, label %19, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

19:                                               ; preds = %16
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i, i32 8)
  br label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

branch7.i.i:                                      ; preds = %else_block.i
  %is_big_str.i29.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload, 0
  br i1 %is_big_str.i29.i.i, label %modify_rc.i30.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

modify_rc.i30.i.i:                                ; preds = %branch7.i.i
  %20 = ptrtoint ptr %result_value.sroa.0.0.copyload to i64
  %21 = shl nuw i64 %result_value.sroa.3.0.copyload, 1
  %isneg.i.i.i34.i.i = icmp slt i64 %result_value.sroa.2.0.copyload, 0
  %22 = select i1 %isneg.i.i.i34.i.i, i64 %21, i64 %20
  %23 = inttoptr i64 %22 to ptr
  %get_rc_ptr.i35.i.i = getelementptr inbounds i64, ptr %23, i64 -1
  %24 = load i64, ptr %get_rc_ptr.i35.i.i, align 8
  %.not.i.i36.i.i = icmp eq i64 %24, 0
  br i1 %.not.i.i36.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit, label %25

25:                                               ; preds = %modify_rc.i30.i.i
  %26 = atomicrmw sub ptr %get_rc_ptr.i35.i.i, i64 1 monotonic, align 8
  %27 = icmp eq i64 %26, 1
  br i1 %27, label %28, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

28:                                               ; preds = %25
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i35.i.i, i32 8)
  br label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

branch1.i.i:                                      ; preds = %else_block.i
  %is_big_str.i40.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload, 0
  br i1 %is_big_str.i40.i.i, label %modify_rc.i41.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

modify_rc.i41.i.i:                                ; preds = %branch1.i.i
  %29 = ptrtoint ptr %result_value.sroa.0.0.copyload to i64
  %30 = shl nuw i64 %result_value.sroa.3.0.copyload, 1
  %isneg.i.i.i45.i.i = icmp slt i64 %result_value.sroa.2.0.copyload, 0
  %31 = select i1 %isneg.i.i.i45.i.i, i64 %30, i64 %29
  %32 = inttoptr i64 %31 to ptr
  %get_rc_ptr.i46.i.i = getelementptr inbounds i64, ptr %32, i64 -1
  %33 = load i64, ptr %get_rc_ptr.i46.i.i, align 8
  %.not.i.i47.i.i = icmp eq i64 %33, 0
  br i1 %.not.i.i47.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit, label %34

34:                                               ; preds = %modify_rc.i41.i.i
  %35 = atomicrmw sub ptr %get_rc_ptr.i46.i.i, i64 1 monotonic, align 8
  %36 = icmp eq i64 %35, 1
  br i1 %36, label %37, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

37:                                               ; preds = %34
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i46.i.i, i32 8)
  br label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

branch0.i.i:                                      ; preds = %else_block.i
  %is_big_str.i51.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload, 0
  br i1 %is_big_str.i51.i.i, label %modify_rc.i52.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

modify_rc.i52.i.i:                                ; preds = %branch0.i.i
  %38 = ptrtoint ptr %result_value.sroa.0.0.copyload to i64
  %39 = shl nuw i64 %result_value.sroa.3.0.copyload, 1
  %isneg.i.i.i56.i.i = icmp slt i64 %result_value.sroa.2.0.copyload, 0
  %40 = select i1 %isneg.i.i.i56.i.i, i64 %39, i64 %38
  %41 = inttoptr i64 %40 to ptr
  %get_rc_ptr.i57.i.i = getelementptr inbounds i64, ptr %41, i64 -1
  %42 = load i64, ptr %get_rc_ptr.i57.i.i, align 8
  %.not.i.i58.i.i = icmp eq i64 %42, 0
  br i1 %.not.i.i58.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit, label %43

43:                                               ; preds = %modify_rc.i52.i.i
  %44 = atomicrmw sub ptr %get_rc_ptr.i57.i.i, i64 1 monotonic, align 8
  %45 = icmp eq i64 %44, 1
  br i1 %45, label %46, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

46:                                               ; preds = %43
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i57.i.i, i32 8)
  br label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

branch3.i.i:                                      ; preds = %else_block.i
  %is_big_str.i62.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload, 0
  br i1 %is_big_str.i62.i.i, label %modify_rc.i63.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

modify_rc.i63.i.i:                                ; preds = %branch3.i.i
  %47 = ptrtoint ptr %result_value.sroa.0.0.copyload to i64
  %48 = shl nuw i64 %result_value.sroa.3.0.copyload, 1
  %isneg.i.i.i67.i.i = icmp slt i64 %result_value.sroa.2.0.copyload, 0
  %49 = select i1 %isneg.i.i.i67.i.i, i64 %48, i64 %47
  %50 = inttoptr i64 %49 to ptr
  %get_rc_ptr.i68.i.i = getelementptr inbounds i64, ptr %50, i64 -1
  %51 = load i64, ptr %get_rc_ptr.i68.i.i, align 8
  %.not.i.i69.i.i = icmp eq i64 %51, 0
  br i1 %.not.i.i69.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit, label %52

52:                                               ; preds = %modify_rc.i63.i.i
  %53 = atomicrmw sub ptr %get_rc_ptr.i68.i.i, i64 1 monotonic, align 8
  %54 = icmp eq i64 %53, 1
  br i1 %54, label %55, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

55:                                               ; preds = %52
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i68.i.i, i32 8)
  br label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

branch8.i.i:                                      ; preds = %else_block.i
  %is_big_str.i73.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload, 0
  br i1 %is_big_str.i73.i.i, label %modify_rc.i74.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

modify_rc.i74.i.i:                                ; preds = %branch8.i.i
  %56 = ptrtoint ptr %result_value.sroa.0.0.copyload to i64
  %57 = shl nuw i64 %result_value.sroa.3.0.copyload, 1
  %isneg.i.i.i78.i.i = icmp slt i64 %result_value.sroa.2.0.copyload, 0
  %58 = select i1 %isneg.i.i.i78.i.i, i64 %57, i64 %56
  %59 = inttoptr i64 %58 to ptr
  %get_rc_ptr.i79.i.i = getelementptr inbounds i64, ptr %59, i64 -1
  %60 = load i64, ptr %get_rc_ptr.i79.i.i, align 8
  %.not.i.i80.i.i = icmp eq i64 %60, 0
  br i1 %.not.i.i80.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit, label %61

61:                                               ; preds = %modify_rc.i74.i.i
  %62 = atomicrmw sub ptr %get_rc_ptr.i79.i.i, i64 1 monotonic, align 8
  %63 = icmp eq i64 %62, 1
  br i1 %63, label %64, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

64:                                               ; preds = %61
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i79.i.i, i32 8)
  br label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

branch6.i.i:                                      ; preds = %else_block.i
  %is_big_str.i84.i.i = icmp sgt i64 %result_value.sroa.3.0.copyload, 0
  br i1 %is_big_str.i84.i.i, label %modify_rc.i85.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

modify_rc.i85.i.i:                                ; preds = %branch6.i.i
  %65 = ptrtoint ptr %result_value.sroa.0.0.copyload to i64
  %66 = shl nuw i64 %result_value.sroa.3.0.copyload, 1
  %isneg.i.i.i89.i.i = icmp slt i64 %result_value.sroa.2.0.copyload, 0
  %67 = select i1 %isneg.i.i.i89.i.i, i64 %66, i64 %65
  %68 = inttoptr i64 %67 to ptr
  %get_rc_ptr.i90.i.i = getelementptr inbounds i64, ptr %68, i64 -1
  %69 = load i64, ptr %get_rc_ptr.i90.i.i, align 8
  %.not.i.i91.i.i = icmp eq i64 %69, 0
  br i1 %.not.i.i91.i.i, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit, label %70

70:                                               ; preds = %modify_rc.i85.i.i
  %71 = atomicrmw sub ptr %get_rc_ptr.i90.i.i, i64 1 monotonic, align 8
  %72 = icmp eq i64 %71, 1
  br i1 %72, label %73, label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

73:                                               ; preds = %70
  call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i90.i.i, i32 8)
  br label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

branch5.i.i:                                      ; preds = %else_block.i
  %74 = ptrtoint ptr %result_value.sroa.0.0.copyload to i64
  br label %Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit

Result_map_err_1dc6f9836252db90e110ad1b5e28c952e1292787deeab71a2a18c14f1d18ec0.exit: ; preds = %default.i.i, %branch4.i.i, %modify_rc.i.i.i, %16, %19, %branch7.i.i, %modify_rc.i30.i.i, %25, %28, %branch1.i.i, %modify_rc.i41.i.i, %34, %37, %branch0.i.i, %modify_rc.i52.i.i, %43, %46, %branch3.i.i, %modify_rc.i63.i.i, %52, %55, %branch8.i.i, %modify_rc.i74.i.i, %61, %64, %branch6.i.i, %modify_rc.i85.i.i, %70, %73, %branch5.i.i, %"Host_stderr_line!_dbefccae6de790f8e3497ad3c6c1c58a12a744edb0d65ec1ec4ade8b1151a59b.exit"
  %result_value1.sroa.0.0 = phi i64 [ undef, %"Host_stderr_line!_dbefccae6de790f8e3497ad3c6c1c58a12a744edb0d65ec1ec4ade8b1151a59b.exit" ], [ %10, %default.i.i ], [ %74, %branch5.i.i ], [ undef, %modify_rc.i85.i.i ], [ undef, %73 ], [ undef, %70 ], [ undef, %branch6.i.i ], [ undef, %modify_rc.i74.i.i ], [ undef, %64 ], [ undef, %61 ], [ undef, %branch8.i.i ], [ undef, %modify_rc.i63.i.i ], [ undef, %55 ], [ undef, %52 ], [ undef, %branch3.i.i ], [ undef, %modify_rc.i52.i.i ], [ undef, %46 ], [ undef, %43 ], [ undef, %branch0.i.i ], [ undef, %modify_rc.i41.i.i ], [ undef, %37 ], [ undef, %34 ], [ undef, %branch1.i.i ], [ undef, %modify_rc.i30.i.i ], [ undef, %28 ], [ undef, %25 ], [ undef, %branch7.i.i ], [ undef, %modify_rc.i.i.i ], [ undef, %19 ], [ undef, %16 ], [ undef, %branch4.i.i ]
  %result_value1.sroa.4.0 = phi i8 [ undef, %"Host_stderr_line!_dbefccae6de790f8e3497ad3c6c1c58a12a744edb0d65ec1ec4ade8b1151a59b.exit" ], [ 4, %default.i.i ], [ 4, %branch5.i.i ], [ 5, %modify_rc.i85.i.i ], [ 5, %73 ], [ 5, %70 ], [ 5, %branch6.i.i ], [ 7, %modify_rc.i74.i.i ], [ 7, %64 ], [ 7, %61 ], [ 7, %branch8.i.i ], [ 2, %modify_rc.i63.i.i ], [ 2, %55 ], [ 2, %52 ], [ 2, %branch3.i.i ], [ 0, %modify_rc.i52.i.i ], [ 0, %46 ], [ 0, %43 ], [ 0, %branch0.i.i ], [ 1, %modify_rc.i41.i.i ], [ 1, %37 ], [ 1, %34 ], [ 1, %branch1.i.i ], [ 6, %modify_rc.i30.i.i ], [ 6, %28 ], [ 6, %25 ], [ 6, %branch7.i.i ], [ 3, %modify_rc.i.i.i ], [ 3, %19 ], [ 3, %16 ], [ 3, %branch4.i.i ]
  %.sink.i = phi i8 [ 1, %"Host_stderr_line!_dbefccae6de790f8e3497ad3c6c1c58a12a744edb0d65ec1ec4ade8b1151a59b.exit" ], [ 0, %default.i.i ], [ 0, %branch5.i.i ], [ 0, %modify_rc.i85.i.i ], [ 0, %73 ], [ 0, %70 ], [ 0, %branch6.i.i ], [ 0, %modify_rc.i74.i.i ], [ 0, %64 ], [ 0, %61 ], [ 0, %branch8.i.i ], [ 0, %modify_rc.i63.i.i ], [ 0, %55 ], [ 0, %52 ], [ 0, %branch3.i.i ], [ 0, %modify_rc.i52.i.i ], [ 0, %46 ], [ 0, %43 ], [ 0, %branch0.i.i ], [ 0, %modify_rc.i41.i.i ], [ 0, %37 ], [ 0, %34 ], [ 0, %branch1.i.i ], [ 0, %modify_rc.i30.i.i ], [ 0, %28 ], [ 0, %25 ], [ 0, %branch7.i.i ], [ 0, %modify_rc.i.i.i ], [ 0, %19 ], [ 0, %16 ], [ 0, %branch4.i.i ]
  store i64 %result_value1.sroa.0.0, ptr %0, align 8
  %result_value1.sroa.2.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 8
  store i64 %result_value.sroa.2.0.copyload, ptr %result_value1.sroa.2.0..sroa_idx, align 8
  %result_value1.sroa.3.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 16
  store i64 %result_value.sroa.3.0.copyload, ptr %result_value1.sroa.3.0..sroa_idx, align 8
  %result_value1.sroa.4.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 24
  store i8 %result_value1.sroa.4.0, ptr %result_value1.sroa.4.0..sroa_idx, align 8
  %result_value1.sroa.54.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 32
  store i8 %.sink.i, ptr %result_value1.sroa.54.0..sroa_idx, align 8
  ret void
}

; Function Attrs: nounwind
define internal fastcc ptr @"#UserApp_build_9b7306a2e571f3d11e34b901a57455c3e32e69a2f8fde813ed1c4300e12712"(i64 %arg_0, i64 %arg_1) unnamed_addr #10 {
entry:
  %stack_value = alloca ptr, align 8
  store ptr null, ptr %stack_value, align 8
  %eq_u64.i45 = icmp eq i64 %arg_0, 0
  br i1 %eq_u64.i45, label %then_block, label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit

then_block:                                       ; preds = %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit, %entry
  %joinpointarg1.lcssa = phi i64 [ %arg_1, %entry ], [ %operation_result.i26, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit ]
  %joinpointarg2.lcssa = phi ptr [ %stack_value, %entry ], [ %call_builtin12.repack17, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit ]
  %0 = tail call ptr @roc_alloc(i64 32, i32 8)
  %1 = icmp ne ptr %0, null
  tail call void @llvm.assume(i1 %1)
  %2 = getelementptr inbounds i8, ptr %0, i64 8
  store i64 1, ptr %0, align 8
  store i64 %joinpointarg1.lcssa, ptr %2, align 8
  %call_builtin.repack21 = getelementptr inbounds i8, ptr %0, i64 16
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %call_builtin.repack21, i8 0, i64 16, i1 false)
  store ptr %2, ptr %joinpointarg2.lcssa, align 8
  %stack_value.0.stack_value.0.stack_value.0.stack_value.0.ptr_load = load ptr, ptr %stack_value, align 8
  ret ptr %stack_value.0.stack_value.0.stack_value.0.stack_value.0.ptr_load

Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit: ; preds = %entry, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit
  %joinpointarg248 = phi ptr [ %call_builtin12.repack17, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit ], [ %stack_value, %entry ]
  %joinpointarg147 = phi i64 [ %operation_result.i26, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit ], [ %arg_1, %entry ]
  %joinpointarg46 = phi i64 [ %call.i41, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit ], [ %arg_0, %entry ]
  %has_overflowed.i25 = icmp slt i64 %joinpointarg147, 0
  br i1 %has_overflowed.i25, label %throw_block.i27, label %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit

throw_block.i27:                                  ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit: ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit
  %call.i41 = add i64 %joinpointarg46, -1
  %operation_result.i26 = shl nuw i64 %joinpointarg147, 1
  %call.i3742 = or disjoint i64 %operation_result.i26, 1
  %call11 = tail call fastcc ptr @"#UserApp_build_9b7306a2e571f3d11e34b901a57455c3e32e69a2f8fde813ed1c4300e12712"(i64 %call.i41, i64 %call.i3742)
  %3 = tail call ptr @roc_alloc(i64 32, i32 8)
  %4 = icmp ne ptr %3, null
  tail call void @llvm.assume(i1 %4)
  %5 = getelementptr inbounds i8, ptr %3, i64 8
  store i64 1, ptr %3, align 8
  store i64 %joinpointarg147, ptr %5, align 8
  %call_builtin12.repack17 = getelementptr inbounds i8, ptr %3, i64 16
  store ptr null, ptr %call_builtin12.repack17, align 8
  %call_builtin12.repack19 = getelementptr inbounds i8, ptr %3, i64 24
  store ptr %call11, ptr %call_builtin12.repack19, align 8
  store ptr %5, ptr %joinpointarg248, align 8
  %eq_u64.i = icmp eq i64 %call.i41, 0
  br i1 %eq_u64.i, label %then_block, label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit
}

; Function Attrs: nofree norecurse nosync nounwind memory(readwrite, inaccessiblemem: none)
define internal fastcc { i64, i8 } @Str_str_to_num_926c4e1deae44cb32fa91b0fc2f966fdf98af98ee562517f2d5df6cc1b8bf0(ptr nocapture readonly %"#arg1") unnamed_addr #11 {
entry:
  %0 = alloca [16 x i8], align 8
  %.sroa.gep6 = getelementptr inbounds i8, ptr %0, i64 8
  %1 = alloca [16 x i8], align 8
  %.sroa.gep = getelementptr inbounds i8, ptr %1, i64 8
  %2 = alloca [24 x i8], align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %2)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2, ptr noundef nonnull align 8 dereferenceable(24) %"#arg1", i64 24, i1 false)
  %3 = getelementptr inbounds i8, ptr %2, i64 16
  %.val.i.i.i = load i64, ptr %3, align 8
  %4 = icmp slt i64 %.val.i.i.i, 0
  br i1 %4, label %5, label %8

5:                                                ; preds = %entry
  %6 = lshr i64 %.val.i.i.i, 56
  %7 = and i64 %6, 127
  br label %str.RocStr.asSlice.exit.i

8:                                                ; preds = %entry
  %9 = load ptr, ptr %2, align 8
  %10 = getelementptr inbounds %str.RocStr, ptr %2, i64 0, i32 1
  %11 = load i64, ptr %10, align 8
  %12 = and i64 %11, 9223372036854775807
  br label %str.RocStr.asSlice.exit.i

str.RocStr.asSlice.exit.i:                        ; preds = %8, %5
  %common.ret.op.i5.i.i = phi ptr [ %2, %5 ], [ %9, %8 ]
  %common.ret.op.i3.i.i = phi i64 [ %7, %5 ], [ %12, %8 ]
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %0), !noalias !154
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %1), !noalias !154
  %13 = icmp eq i64 %common.ret.op.i3.i.i, 0
  br i1 %13, label %fmt.parseInt__anon_6494.exit.thread.i, label %14

14:                                               ; preds = %str.RocStr.asSlice.exit.i
  %15 = load i8, ptr %common.ret.op.i5.i.i, align 1, !noalias !157
  switch i8 %15, label %54 [
    i8 43, label %16
    i8 45, label %19
  ]

16:                                               ; preds = %14
  %17 = getelementptr inbounds i8, ptr %common.ret.op.i5.i.i, i64 1
  %18 = add nsw i64 %common.ret.op.i3.i.i, -1
  call fastcc void @fmt.parseIntWithSign__anon_8022(ptr noalias %1, ptr nonnull readonly align 1 %17, i64 %18)
  br label %fmt.parseInt__anon_6494.exit.i

19:                                               ; preds = %14
  %20 = getelementptr inbounds i8, ptr %common.ret.op.i5.i.i, i64 1
  %21 = add nsw i64 %common.ret.op.i3.i.i, -1
  %22 = icmp eq i64 %21, 0
  br i1 %22, label %fmt.parseInt__anon_6494.exit.thread.i, label %25

.critedgethread-pre-split.i.i.i.i:                ; preds = %42, %39, %36, %30, %25
  %.sroa.8.0.ph.i.i.i.i = phi i64 [ %21, %25 ], [ %38, %36 ], [ %41, %39 ], [ %44, %42 ], [ %21, %30 ]
  %.sroa.036.0.ph.i.i.i.i = phi ptr [ %20, %25 ], [ %37, %36 ], [ %40, %39 ], [ %43, %42 ], [ %20, %30 ]
  %.pr.i.i.i.i = load i8, ptr %.sroa.036.0.ph.i.i.i.i, align 1, !noalias !160
  br label %.critedge.i.i.i.i

.critedge.i.i.i.i:                                ; preds = %27, %.critedgethread-pre-split.i.i.i.i
  %23 = phi i8 [ %.pr.i.i.i.i, %.critedgethread-pre-split.i.i.i.i ], [ %28, %27 ]
  %.sroa.8.0.i.i.i.i = phi i64 [ %.sroa.8.0.ph.i.i.i.i, %.critedgethread-pre-split.i.i.i.i ], [ %21, %27 ]
  %.sroa.036.0.i.i.i.i = phi ptr [ %.sroa.036.0.ph.i.i.i.i, %.critedgethread-pre-split.i.i.i.i ], [ %20, %27 ]
  %24 = icmp eq i8 %23, 95
  br i1 %24, label %fmt.parseInt__anon_6494.exit.thread.i, label %45

25:                                               ; preds = %19
  %26 = icmp ugt i64 %21, 2
  br i1 %26, label %27, label %.critedgethread-pre-split.i.i.i.i

27:                                               ; preds = %25
  %28 = load i8, ptr %20, align 1, !noalias !160
  %29 = icmp eq i8 %28, 48
  br i1 %29, label %30, label %.critedge.i.i.i.i

30:                                               ; preds = %27
  %31 = getelementptr inbounds i8, ptr %common.ret.op.i5.i.i, i64 2
  %32 = load i8, ptr %31, align 1, !noalias !160
  %33 = add i8 %32, -65
  %34 = icmp ult i8 %33, 26
  %35 = or i8 %32, 32
  %common.ret.op.i.i.i.i.i = select i1 %34, i8 %35, i8 %32
  switch i8 %common.ret.op.i.i.i.i.i, label %.critedgethread-pre-split.i.i.i.i [
    i8 98, label %36
    i8 111, label %39
    i8 120, label %42
  ]

36:                                               ; preds = %30
  %37 = getelementptr inbounds i8, ptr %common.ret.op.i5.i.i, i64 3
  %38 = add nsw i64 %common.ret.op.i3.i.i, -3
  br label %.critedgethread-pre-split.i.i.i.i

39:                                               ; preds = %30
  %40 = getelementptr inbounds i8, ptr %common.ret.op.i5.i.i, i64 3
  %41 = add nsw i64 %common.ret.op.i3.i.i, -3
  br label %.critedgethread-pre-split.i.i.i.i

42:                                               ; preds = %30
  %43 = getelementptr inbounds i8, ptr %common.ret.op.i5.i.i, i64 3
  %44 = add nsw i64 %common.ret.op.i3.i.i, -3
  br label %.critedgethread-pre-split.i.i.i.i

45:                                               ; preds = %.critedge.i.i.i.i
  %46 = add nsw i64 %.sroa.8.0.i.i.i.i, -1
  %47 = getelementptr inbounds i8, ptr %.sroa.036.0.i.i.i.i, i64 %46
  %48 = load i8, ptr %47, align 1, !noalias !160
  %49 = icmp eq i8 %48, 95
  br i1 %49, label %fmt.parseInt__anon_6494.exit.thread.i, label %.lr.ph.i.i.i.i

50:                                               ; preds = %.lr.ph.i.i.i.i, %.lr.ph.i.i.i.i
  %51 = add nuw i64 %.sroa.067.0120.i.i.i.i, 1
  %exitcond.not.i.i.i.i = icmp eq i64 %51, %.sroa.8.0.i.i.i.i
  br i1 %exitcond.not.i.i.i.i, label %fmt.parseInt__anon_6494.exit.thread10.i, label %.lr.ph.i.i.i.i

fmt.parseInt__anon_6494.exit.thread10.i:          ; preds = %50
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %0), !noalias !154
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %1), !noalias !154
  br label %roc_builtins.str.to_int.u64.exit

.lr.ph.i.i.i.i:                                   ; preds = %45, %50
  %.sroa.067.0120.i.i.i.i = phi i64 [ %51, %50 ], [ 0, %45 ]
  %52 = getelementptr inbounds i8, ptr %.sroa.036.0.i.i.i.i, i64 %.sroa.067.0120.i.i.i.i
  %53 = load i8, ptr %52, align 1, !noalias !160
  switch i8 %53, label %fmt.parseInt__anon_6494.exit.thread.i [
    i8 95, label %50
    i8 48, label %50
  ]

54:                                               ; preds = %14
  call fastcc void @fmt.parseIntWithSign__anon_8022(ptr noalias %0, ptr nonnull readonly align 1 %common.ret.op.i5.i.i, i64 %common.ret.op.i3.i.i)
  br label %fmt.parseInt__anon_6494.exit.i

fmt.parseInt__anon_6494.exit.thread.i:            ; preds = %.lr.ph.i.i.i.i, %45, %.critedge.i.i.i.i, %19, %str.RocStr.asSlice.exit.i
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %0), !noalias !154
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %1), !noalias !154
  br label %roc_builtins.str.to_int.u64.exit

fmt.parseInt__anon_6494.exit.i:                   ; preds = %54, %16
  %.sink.i = phi ptr [ %1, %16 ], [ %0, %54 ]
  %.sink.i.sroa.phi = phi ptr [ %.sroa.gep, %16 ], [ %.sroa.gep6, %54 ]
  %.sroa.5.0.copyload5.i.i = load i16, ptr %.sink.i.sroa.phi, align 8, !noalias !154
  %.sroa.0.0.i.i = load i64, ptr %.sink.i, align 8, !noalias !154
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %0), !noalias !154
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %1), !noalias !154
  %.sroa.5.0.i.fr.i = freeze i16 %.sroa.5.0.copyload5.i.i
  %55 = icmp eq i16 %.sroa.5.0.i.fr.i, 0
  %.fca.0.insert.i = insertvalue [2 x i64] poison, i64 %.sroa.0.0.i.i, 0
  %.fca.1.insert.i = insertvalue [2 x i64] %.fca.0.insert.i, i64 0, 1
  %spec.select.i = select i1 %55, [2 x i64] %.fca.1.insert.i, [2 x i64] [i64 0, i64 1]
  br label %roc_builtins.str.to_int.u64.exit

roc_builtins.str.to_int.u64.exit:                 ; preds = %fmt.parseInt__anon_6494.exit.thread10.i, %fmt.parseInt__anon_6494.exit.thread.i, %fmt.parseInt__anon_6494.exit.i
  %56 = phi [2 x i64] [ [i64 0, i64 1], %fmt.parseInt__anon_6494.exit.thread.i ], [ zeroinitializer, %fmt.parseInt__anon_6494.exit.thread10.i ], [ %spec.select.i, %fmt.parseInt__anon_6494.exit.i ]
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %2)
  %dec_alloca3 = alloca [2 x i64], align 16
  %call_builtin.elt = extractvalue [2 x i64] %56, 0
  store i64 %call_builtin.elt, ptr %dec_alloca3, align 16
  %dec_alloca3.repack4 = getelementptr inbounds [2 x i64], ptr %dec_alloca3, i64 0, i64 1
  %call_builtin.elt5 = extractvalue [2 x i64] %56, 1
  store i64 %call_builtin.elt5, ptr %dec_alloca3.repack4, align 8
  %return_i128 = load i128, ptr %dec_alloca3, align 16
  %cast_alloca1.sroa.0.0.extract.trunc = trunc i128 %return_i128 to i64
  %cast_alloca1.sroa.2.0.extract.shift = lshr i128 %return_i128, 64
  %cast_alloca1.sroa.2.0.extract.trunc = trunc i128 %cast_alloca1.sroa.2.0.extract.shift to i8
  %cast_value2.fca.0.insert = insertvalue { i64, i8 } poison, i64 %cast_alloca1.sroa.0.0.extract.trunc, 0
  %cast_value2.fca.1.insert = insertvalue { i64, i8 } %cast_value2.fca.0.insert, i8 %cast_alloca1.sroa.2.0.extract.trunc, 1
  ret { i64, i8 } %cast_value2.fca.1.insert
}

; Function Attrs: nounwind
define internal fastcc void @Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133(ptr nocapture readonly %"#arg1", i64 %"#arg2", i64 %"#arg3", ptr nocapture writeonly %0) unnamed_addr #10 {
entry:
  %1 = alloca [24 x i8], align 8
  %2 = alloca [24 x i8], align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1), !noalias !163
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %2), !noalias !163
  %.sroa.1.0..sroa_idx.i.i = getelementptr inbounds i8, ptr %"#arg1", i64 16
  %.sroa.1.0.copyload.i.i = load i64, ptr %.sroa.1.0..sroa_idx.i.i, align 8, !noalias !166
  %3 = icmp slt i64 %.sroa.1.0.copyload.i.i, 0
  br i1 %3, label %4, label %6

4:                                                ; preds = %entry
  %5 = icmp eq i64 %"#arg2", 0
  br i1 %5, label %str.RocStr.setLen.exit.i.i, label %str.RocStr.asSlice.exit.i.i

6:                                                ; preds = %entry
  %7 = load ptr, ptr %"#arg1", align 8, !noalias !166
  %.not.i.i = icmp eq ptr %7, null
  %8 = ptrtoint ptr %7 to i64
  br i1 %.not.i.i, label %roc_builtins.str.substring_unsafe.exit, label %22

str.RocStr.setLen.exit.i.i:                       ; preds = %4
  %.sroa.010.0.copyload.i.i = load i64, ptr %"#arg1", align 8, !noalias !166
  %.sroa.2.0..sroa_idx.i.i = getelementptr inbounds i8, ptr %"#arg1", i64 8
  %.sroa.2.0.copyload.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i, align 8, !noalias !166
  %9 = shl i64 %"#arg3", 56
  %.sroa.4.23.insert.mask.i.i = and i64 %.sroa.1.0.copyload.i.i, 72057594037927935
  %.sroa.4.23.insert.shift.i.i = or disjoint i64 %.sroa.4.23.insert.mask.i.i, %9
  %.sroa.4.23.insert.insert.i.i = or i64 %.sroa.4.23.insert.shift.i.i, -9223372036854775808
  br label %roc_builtins.str.substring_unsafe.exit

str.RocStr.asSlice.exit.i.i:                      ; preds = %4
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %2, ptr noundef nonnull align 8 dereferenceable(24) %"#arg1", i64 24, i1 false), !noalias !166
  %10 = getelementptr inbounds i8, ptr %2, i64 16
  %.val.i.i.i.i = load i64, ptr %10, align 8, !noalias !166
  %11 = icmp slt i64 %.val.i.i.i.i, 0
  %12 = load ptr, ptr %2, align 8, !noalias !166
  %common.ret.op.i5.i.i.i = select i1 %11, ptr %2, ptr %12
  %13 = getelementptr inbounds i8, ptr %common.ret.op.i5.i.i.i, i64 %"#arg2"
  %14 = icmp ugt i64 %"#arg3", 23
  br i1 %14, label %str.RocStr.allocate.exit.i.i.i.i, label %str.RocStr.allocate.exit.thread.i.i.i.i

str.RocStr.allocate.exit.thread.i.i.i.i:          ; preds = %str.RocStr.asSlice.exit.i.i
  %.sroa.3.23.insert.ext.i.i.i.i = shl nuw nsw i64 %"#arg3", 56
  %.sroa.3.23.insert.shift.i.i.i.i = or disjoint i64 %.sroa.3.23.insert.ext.i.i.i.i, -9223372036854775808
  br label %str.RocStr.fromSlice.exit.i.i

str.RocStr.allocate.exit.i.i.i.i:                 ; preds = %str.RocStr.asSlice.exit.i.i
  %15 = add nuw i64 %"#arg3", 8
  %16 = tail call ptr @roc_alloc(i64 %15, i32 8), !noalias !169
  %17 = icmp ne ptr %16, null
  tail call void @llvm.assume(i1 %17)
  %18 = getelementptr inbounds i8, ptr %16, i64 8
  store i64 1, ptr %16, align 8, !noalias !169
  %19 = icmp slt i64 %"#arg3", 0
  %spec.select.i.i.i.i = select i1 %19, ptr %1, ptr %18
  br label %str.RocStr.fromSlice.exit.i.i

str.RocStr.fromSlice.exit.i.i:                    ; preds = %str.RocStr.allocate.exit.i.i.i.i, %str.RocStr.allocate.exit.thread.i.i.i.i
  %.sink7.i.i.i.i = phi ptr [ %18, %str.RocStr.allocate.exit.i.i.i.i ], [ null, %str.RocStr.allocate.exit.thread.i.i.i.i ]
  %.sink.i.i.i.i = phi i64 [ %"#arg3", %str.RocStr.allocate.exit.i.i.i.i ], [ 0, %str.RocStr.allocate.exit.thread.i.i.i.i ]
  %.sroa.3.23.insert.shift.sink.i.i.i.i = phi i64 [ %"#arg3", %str.RocStr.allocate.exit.i.i.i.i ], [ %.sroa.3.23.insert.shift.i.i.i.i, %str.RocStr.allocate.exit.thread.i.i.i.i ]
  %common.ret.op.i.i.i.i.i = phi ptr [ %spec.select.i.i.i.i, %str.RocStr.allocate.exit.i.i.i.i ], [ %1, %str.RocStr.allocate.exit.thread.i.i.i.i ]
  store ptr %.sink7.i.i.i.i, ptr %1, align 8, !noalias !166
  %20 = getelementptr inbounds i8, ptr %1, i64 8
  store i64 %.sink.i.i.i.i, ptr %20, align 8, !noalias !166
  %21 = getelementptr inbounds i8, ptr %1, i64 16
  store i64 %.sroa.3.23.insert.shift.sink.i.i.i.i, ptr %21, align 8, !noalias !166
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %common.ret.op.i.i.i.i.i, ptr nonnull align 1 %13, i64 %"#arg3", i1 false), !noalias !166
  %.sroa.0.0.copyload2.i = load i64, ptr %1, align 8, !noalias !163
  %.sroa.6.0.copyload5.i = load i64, ptr %20, align 8, !noalias !163
  %.sroa.9.0.copyload8.i = load i64, ptr %21, align 8, !noalias !163
  br label %roc_builtins.str.substring_unsafe.exit

22:                                               ; preds = %6
  %23 = icmp eq i64 %"#arg2", 0
  %.sroa.217.0..sroa_idx.i.i = getelementptr inbounds i8, ptr %"#arg1", i64 8
  %.sroa.217.0.copyload.i.i = load i64, ptr %.sroa.217.0..sroa_idx.i.i, align 8, !noalias !166
  br i1 %23, label %str.RocStr.getCapacity.exit.i.i.i.i, label %.critedge.i.i

str.RocStr.getCapacity.exit.i.i.i.i:              ; preds = %22
  %24 = icmp slt i64 %.sroa.217.0.copyload.i.i, 0
  %25 = and i64 %.sroa.217.0.copyload.i.i, 9223372036854775807
  %spec.select.i.i.i.i.i = select i1 %24, i64 %25, i64 %.sroa.1.0.copyload.i.i
  %26 = icmp ne i64 %spec.select.i.i.i.i.i, 0
  %.not1.i.i.i.i = or i1 %24, %26
  br i1 %.not1.i.i.i.i, label %str.RocStr.isUnique.exit.i.i, label %str.RocStr.setLen.exit9.i.i

str.RocStr.isUnique.exit.i.i:                     ; preds = %str.RocStr.getCapacity.exit.i.i.i.i
  %27 = shl nuw i64 %.sroa.1.0.copyload.i.i, 1
  %28 = inttoptr i64 %27 to ptr
  %29 = select i1 %24, ptr %28, ptr %7
  %30 = getelementptr inbounds i64, ptr %29, i64 -1
  %31 = load i64, ptr %30, align 8, !noalias !166
  %32 = icmp eq i64 %31, 1
  br i1 %32, label %str.RocStr.setLen.exit9.i.i, label %.critedge.i.i

str.RocStr.setLen.exit9.i.i:                      ; preds = %str.RocStr.isUnique.exit.i.i, %str.RocStr.getCapacity.exit.i.i.i.i
  %33 = and i64 %.sroa.217.0.copyload.i.i, -9223372036854775808
  %34 = or i64 %33, %"#arg3"
  br label %roc_builtins.str.substring_unsafe.exit

.critedge.i.i:                                    ; preds = %str.RocStr.isUnique.exit.i.i, %22
  %35 = lshr i64 %8, 1
  %isneg.i.i = icmp slt i64 %.sroa.217.0.copyload.i.i, 0
  %36 = select i1 %isneg.i.i, i64 %.sroa.1.0.copyload.i.i, i64 %35
  %37 = getelementptr inbounds i8, ptr %7, i64 %"#arg2"
  %38 = or i64 %"#arg3", -9223372036854775808
  %39 = ptrtoint ptr %37 to i64
  br label %roc_builtins.str.substring_unsafe.exit

roc_builtins.str.substring_unsafe.exit:           ; preds = %6, %str.RocStr.setLen.exit.i.i, %str.RocStr.fromSlice.exit.i.i, %str.RocStr.setLen.exit9.i.i, %.critedge.i.i
  %.sroa.9.0.i = phi i64 [ %.sroa.4.23.insert.insert.i.i, %str.RocStr.setLen.exit.i.i ], [ %.sroa.9.0.copyload8.i, %str.RocStr.fromSlice.exit.i.i ], [ %.sroa.1.0.copyload.i.i, %str.RocStr.setLen.exit9.i.i ], [ %36, %.critedge.i.i ], [ -9223372036854775808, %6 ]
  %.sroa.6.0.i = phi i64 [ %.sroa.2.0.copyload.i.i, %str.RocStr.setLen.exit.i.i ], [ %.sroa.6.0.copyload5.i, %str.RocStr.fromSlice.exit.i.i ], [ %34, %str.RocStr.setLen.exit9.i.i ], [ %38, %.critedge.i.i ], [ 0, %6 ]
  %.sroa.0.0.i = phi i64 [ %.sroa.010.0.copyload.i.i, %str.RocStr.setLen.exit.i.i ], [ %.sroa.0.0.copyload2.i, %str.RocStr.fromSlice.exit.i.i ], [ %8, %str.RocStr.setLen.exit9.i.i ], [ %39, %.critedge.i.i ], [ 0, %6 ]
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1), !noalias !163
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %2), !noalias !163
  store i64 %.sroa.0.0.i, ptr %0, align 8
  %str_alloca.sroa.2.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 8
  store i64 %.sroa.6.0.i, ptr %str_alloca.sroa.2.0..sroa_idx, align 8
  %str_alloca.sroa.3.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 16
  store i64 %.sroa.9.0.i, ptr %str_alloca.sroa.3.0..sroa_idx, align 8
  ret void
}

; Function Attrs: nounwind
define internal fastcc void @Str_split_first_bfa1d47a221bdaf089999196bed323c433d1a6b8c78ec612e6fa7b3e3d811(ptr nocapture readonly %haystack, ptr nocapture readonly %needle, ptr nocapture writeonly %0) unnamed_addr #10 {
entry:
  %struct_alloca.i.i.i = alloca { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, align 8
  %tag_alloca.sroa.0 = alloca [3 x i64], align 8
  %1 = getelementptr i8, ptr %haystack, i64 8
  %haystack.val.i = load i64, ptr %1, align 8
  %2 = getelementptr i8, ptr %haystack, i64 16
  %haystack.val4.i = load i64, ptr %2, align 8
  %3 = icmp slt i64 %haystack.val4.i, 0
  %4 = lshr i64 %haystack.val4.i, 56
  %5 = and i64 %4, 127
  %6 = and i64 %haystack.val.i, 9223372036854775807
  %common.ret.op.i.i.i.i = select i1 %3, i64 %5, i64 %6
  %7 = getelementptr i8, ptr %needle, i64 8
  %needle.val.i = load i64, ptr %7, align 8
  %8 = getelementptr i8, ptr %needle, i64 16
  %needle.val3.i = load i64, ptr %8, align 8
  %9 = icmp slt i64 %needle.val3.i, 0
  %10 = lshr i64 %needle.val3.i, 56
  %11 = and i64 %10, 127
  %12 = and i64 %needle.val.i, 9223372036854775807
  %common.ret.op.i.i.i5.i = select i1 %9, i64 %11, i64 %12
  %call.i.i = tail call i64 @llvm.usub.sat.i64(i64 %common.ret.op.i.i.i.i, i64 %common.ret.op.i.i.i5.i)
  %is_big_str.i.i.i = icmp sgt i64 %needle.val3.i, 0
  br i1 %is_big_str.i.i.i, label %modify_rc.i.i.i, label %"#Attr_#inc_1.exit.i.i"

modify_rc.i.i.i:                                  ; preds = %entry
  %.sroa.0.0.copyload.i.i.i.i = load ptr, ptr %needle, align 8
  %13 = ptrtoint ptr %.sroa.0.0.copyload.i.i.i.i to i64
  %14 = shl nuw i64 %needle.val3.i, 1
  %isneg.i.i.i.i.i = icmp slt i64 %needle.val.i, 0
  %15 = select i1 %isneg.i.i.i.i.i, i64 %14, i64 %13
  %16 = inttoptr i64 %15 to ptr
  %get_rc_ptr.i.i.i = getelementptr inbounds i64, ptr %16, i64 -1
  %17 = load i64, ptr %get_rc_ptr.i.i.i, align 8
  %.not.i.i.i.i = icmp eq i64 %17, 0
  br i1 %.not.i.i.i.i, label %"#Attr_#inc_1.exit.i.i", label %18

18:                                               ; preds = %modify_rc.i.i.i
  %19 = atomicrmw add ptr %get_rc_ptr.i.i.i, i64 1 monotonic, align 8
  %load_str_to_stack.unpack4.i15.i.pre.i = load i64, ptr %2, align 8
  %tmp_output_for_jmp.sroa.2.0.copyload.i.pre25.pre.i = load i64, ptr %1, align 8
  br label %"#Attr_#inc_1.exit.i.i"

"#Attr_#inc_1.exit.i.i":                          ; preds = %18, %modify_rc.i.i.i, %entry
  %tmp_output_for_jmp.sroa.2.0.copyload.i.pre25.i = phi i64 [ %tmp_output_for_jmp.sroa.2.0.copyload.i.pre25.pre.i, %18 ], [ %haystack.val.i, %modify_rc.i.i.i ], [ %haystack.val.i, %entry ]
  %load_str_to_stack.unpack4.i15.i.i = phi i64 [ %load_str_to_stack.unpack4.i15.i.pre.i, %18 ], [ %haystack.val4.i, %modify_rc.i.i.i ], [ %haystack.val4.i, %entry ]
  %is_big_str.i16.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i15.i.i, 0
  %tmp_output_for_jmp.sroa.0.0.copyload.pre130.i.i = load ptr, ptr %haystack, align 8
  br i1 %is_big_str.i16.i.i, label %modify_rc.i17.i.i, label %"#Attr_#inc_1.exit24.i.i"

modify_rc.i17.i.i:                                ; preds = %"#Attr_#inc_1.exit.i.i"
  %20 = ptrtoint ptr %tmp_output_for_jmp.sroa.0.0.copyload.pre130.i.i to i64
  %21 = shl nuw i64 %load_str_to_stack.unpack4.i15.i.i, 1
  %isneg.i.i.i21.i.i = icmp slt i64 %tmp_output_for_jmp.sroa.2.0.copyload.i.pre25.i, 0
  %22 = select i1 %isneg.i.i.i21.i.i, i64 %21, i64 %20
  %23 = inttoptr i64 %22 to ptr
  %get_rc_ptr.i22.i.i = getelementptr inbounds i64, ptr %23, i64 -1
  %24 = load i64, ptr %get_rc_ptr.i22.i.i, align 8
  %.not.i.i23.i.i = icmp eq i64 %24, 0
  br i1 %.not.i.i23.i.i, label %"#Attr_#inc_1.exit24.i.i", label %25

25:                                               ; preds = %modify_rc.i17.i.i
  %26 = atomicrmw add ptr %get_rc_ptr.i22.i.i, i64 1 monotonic, align 8
  %tmp_output_for_jmp.sroa.3.0.copyload.pr.pre.i.i = load i64, ptr %2, align 8
  %tmp_output_for_jmp.sroa.0.0.copyload.pre.pre.i.i = load ptr, ptr %haystack, align 8
  %tmp_output_for_jmp.sroa.2.0.copyload.i.pre.i = load i64, ptr %1, align 8
  br label %"#Attr_#inc_1.exit24.i.i"

"#Attr_#inc_1.exit24.i.i":                        ; preds = %25, %modify_rc.i17.i.i, %"#Attr_#inc_1.exit.i.i"
  %tmp_output_for_jmp.sroa.2.0.copyload.i.i = phi i64 [ %tmp_output_for_jmp.sroa.2.0.copyload.i.pre25.i, %"#Attr_#inc_1.exit.i.i" ], [ %tmp_output_for_jmp.sroa.2.0.copyload.i.pre.i, %25 ], [ %tmp_output_for_jmp.sroa.2.0.copyload.i.pre25.i, %modify_rc.i17.i.i ]
  %tmp_output_for_jmp.sroa.0.0.copyload.i.i = phi ptr [ %tmp_output_for_jmp.sroa.0.0.copyload.pre130.i.i, %"#Attr_#inc_1.exit.i.i" ], [ %tmp_output_for_jmp.sroa.0.0.copyload.pre.pre.i.i, %25 ], [ %tmp_output_for_jmp.sroa.0.0.copyload.pre130.i.i, %modify_rc.i17.i.i ]
  %tmp_output_for_jmp.sroa.3.0.copyload.i.i = phi i64 [ %load_str_to_stack.unpack4.i15.i.i, %"#Attr_#inc_1.exit.i.i" ], [ %tmp_output_for_jmp.sroa.3.0.copyload.pr.pre.i.i, %25 ], [ %load_str_to_stack.unpack4.i15.i.i, %modify_rc.i17.i.i ]
  %tmp_output_for_jmp3.sroa.0.0.copyload.i.i = load ptr, ptr %needle, align 8
  %tmp_output_for_jmp3.sroa.2.0.copyload.i.i = load i64, ptr %7, align 8
  %tmp_output_for_jmp3.sroa.3.0.copyload.i.i = load i64, ptr %8, align 8
  %is_big_str.i.i.i.i = icmp sgt i64 %tmp_output_for_jmp.sroa.3.0.copyload.i.i, 0
  %is_big_str.i15.i.i.i = icmp sgt i64 %tmp_output_for_jmp3.sroa.3.0.copyload.i.i, 0
  %27 = icmp slt i64 %tmp_output_for_jmp3.sroa.3.0.copyload.i.i, 0
  %28 = lshr i64 %tmp_output_for_jmp3.sroa.3.0.copyload.i.i, 56
  %29 = and i64 %28, 127
  %30 = and i64 %tmp_output_for_jmp3.sroa.2.0.copyload.i.i, 9223372036854775807
  %common.ret.op.i.i.i12.i.i.i = select i1 %27, i64 %29, i64 %30
  %31 = icmp slt i64 %tmp_output_for_jmp.sroa.3.0.copyload.i.i, 0
  %32 = lshr i64 %tmp_output_for_jmp.sroa.3.0.copyload.i.i, 56
  %33 = and i64 %32, 127
  %34 = and i64 %tmp_output_for_jmp.sroa.2.0.copyload.i.i, 9223372036854775807
  %common.ret.op.i.i.i.i.i.i = select i1 %31, i64 %33, i64 %34
  %struct_field_gep4.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i, i64 0, i32 1
  %joinpoint_arg_alloca.sroa.7.0.struct_field_gep4.i.sroa_idx.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i, i64 0, i32 1, i32 1
  %joinpoint_arg_alloca.sroa.10.0.struct_field_gep4.i.sroa_idx.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i, i64 0, i32 1, i32 2
  %struct_field_gep5.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i, i64 0, i32 2
  %struct_field_gep6.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i, i64 0, i32 3
  %joinpoint_arg_alloca1.sroa.7.0.struct_field_gep6.i.sroa_idx.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i, i64 0, i32 3, i32 1
  %joinpoint_arg_alloca1.sroa.11.0.struct_field_gep6.i.sroa_idx.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i, i64 0, i32 3, i32 2
  %struct_field_gep7.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i, i64 0, i32 4
  %struct_field_gep8.i.i.i = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca.i.i.i, i64 0, i32 5
  %35 = ptrtoint ptr %tmp_output_for_jmp.sroa.0.0.copyload.i.i to i64
  %36 = shl nuw i64 %tmp_output_for_jmp.sroa.3.0.copyload.i.i, 1
  %isneg.i.i.i.i.i.i = icmp slt i64 %tmp_output_for_jmp.sroa.2.0.copyload.i.i, 0
  %37 = select i1 %isneg.i.i.i.i.i.i, i64 %36, i64 %35
  %38 = inttoptr i64 %37 to ptr
  %get_rc_ptr.i.i.i.i = getelementptr inbounds i64, ptr %38, i64 -1
  %39 = ptrtoint ptr %tmp_output_for_jmp3.sroa.0.0.copyload.i.i to i64
  %40 = shl nuw i64 %tmp_output_for_jmp3.sroa.3.0.copyload.i.i, 1
  %isneg.i.i.i20.i.i.i = icmp slt i64 %tmp_output_for_jmp3.sroa.2.0.copyload.i.i, 0
  %41 = select i1 %isneg.i.i.i20.i.i.i, i64 %40, i64 %39
  %42 = inttoptr i64 %41 to ptr
  %get_rc_ptr.i21.i.i.i = getelementptr inbounds i64, ptr %42, i64 -1
  br i1 %is_big_str.i.i.i.i, label %"#Attr_#inc_1.exit24.split.us.i.i", label %"#Attr_#inc_1.exit24.split.i.i"

"#Attr_#inc_1.exit24.split.us.i.i":               ; preds = %"#Attr_#inc_1.exit24.i.i"
  br i1 %is_big_str.i15.i.i.i, label %then_block.us.us.i.i, label %then_block.us.i.i

then_block.us.us.i.i:                             ; preds = %"#Attr_#inc_1.exit24.split.us.i.i", %else_block7.us.us.i.i
  %joinpointarg96.us.us.i.i = phi i64 [ %add_int_wrap.i.us.us.i.i, %else_block7.us.us.i.i ], [ 0, %"#Attr_#inc_1.exit24.split.us.i.i" ]
  call void @llvm.lifetime.start.p0(i64 80, ptr nonnull %struct_alloca.i.i.i)
  %43 = load i64, ptr %get_rc_ptr.i.i.i.i, align 8
  %.not.i.i.i.us.us.i.i = icmp eq i64 %43, 0
  br i1 %.not.i.i.i.us.us.i.i, label %"#Attr_#inc_1.exit.i.us.us.i.i", label %44

44:                                               ; preds = %then_block.us.us.i.i
  %45 = atomicrmw add ptr %get_rc_ptr.i.i.i.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit.i.us.us.i.i"

"#Attr_#inc_1.exit.i.us.us.i.i":                  ; preds = %44, %then_block.us.us.i.i
  %46 = load i64, ptr %get_rc_ptr.i21.i.i.i, align 8
  %.not.i.i22.i.us.us.i.i = icmp eq i64 %46, 0
  br i1 %.not.i.i22.i.us.us.i.i, label %Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us.us.i.i, label %47

47:                                               ; preds = %"#Attr_#inc_1.exit.i.us.us.i.i"
  %48 = atomicrmw add ptr %get_rc_ptr.i21.i.i.i, i64 1 monotonic, align 8
  br label %Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us.us.i.i

Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us.us.i.i: ; preds = %47, %"#Attr_#inc_1.exit.i.us.us.i.i"
  %call.i.i.us.us.i.i = tail call i64 @llvm.uadd.sat.i64(i64 %joinpointarg96.us.us.i.i, i64 %common.ret.op.i.i.i12.i.i.i)
  %x.y.i.i.us.us.i.i = tail call i64 @llvm.umin.i64(i64 %call.i.i.us.us.i.i, i64 %common.ret.op.i.i.i.i.i.i)
  store i64 %x.y.i.i.us.us.i.i, ptr %struct_alloca.i.i.i, align 8
  store ptr %tmp_output_for_jmp.sroa.0.0.copyload.i.i, ptr %struct_field_gep4.i.i.i, align 8
  store i64 %tmp_output_for_jmp.sroa.2.0.copyload.i.i, ptr %joinpoint_arg_alloca.sroa.7.0.struct_field_gep4.i.sroa_idx.i.i, align 8
  store i64 %tmp_output_for_jmp.sroa.3.0.copyload.i.i, ptr %joinpoint_arg_alloca.sroa.10.0.struct_field_gep4.i.sroa_idx.i.i, align 8
  store i64 %joinpointarg96.us.us.i.i, ptr %struct_field_gep5.i.i.i, align 8
  store ptr %tmp_output_for_jmp3.sroa.0.0.copyload.i.i, ptr %struct_field_gep6.i.i.i, align 8
  store i64 %tmp_output_for_jmp3.sroa.2.0.copyload.i.i, ptr %joinpoint_arg_alloca1.sroa.7.0.struct_field_gep6.i.sroa_idx.i.i, align 8
  store i64 %tmp_output_for_jmp3.sroa.3.0.copyload.i.i, ptr %joinpoint_arg_alloca1.sroa.11.0.struct_field_gep6.i.sroa_idx.i.i, align 8
  store i64 0, ptr %struct_field_gep7.i.i.i, align 8
  store i64 %common.ret.op.i.i.i12.i.i.i, ptr %struct_field_gep8.i.i.i, align 8
  %call9.i.us.us.i.i = call fastcc i1 @Str_matches_at_help_e3211b24ebda6e959f98c8dfabbb8bde232b75ae94930caa03d2bdb8e7b5674(ptr nonnull %struct_alloca.i.i.i)
  call void @llvm.lifetime.end.p0(i64 80, ptr nonnull %struct_alloca.i.i.i)
  br i1 %call9.i.us.us.i.i, label %modify_rc.i49.i.i, label %else_block7.us.us.i.i

else_block7.us.us.i.i:                            ; preds = %Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us.us.i.i
  %add_int_wrap.i.us.us.i.i = add i64 %joinpointarg96.us.us.i.i, 1
  %lte_uint.i.not.us.us.i.i = icmp ugt i64 %add_int_wrap.i.us.us.i.i, %call.i.i
  br i1 %lte_uint.i.not.us.us.i.i, label %else_block.i.i, label %then_block.us.us.i.i

then_block.us.i.i:                                ; preds = %"#Attr_#inc_1.exit24.split.us.i.i", %else_block7.us.i.i
  %joinpointarg96.us.i.i = phi i64 [ %add_int_wrap.i.us.i.i, %else_block7.us.i.i ], [ 0, %"#Attr_#inc_1.exit24.split.us.i.i" ]
  call void @llvm.lifetime.start.p0(i64 80, ptr nonnull %struct_alloca.i.i.i)
  %49 = load i64, ptr %get_rc_ptr.i.i.i.i, align 8
  %.not.i.i.i.us.i.i = icmp eq i64 %49, 0
  br i1 %.not.i.i.i.us.i.i, label %"#Attr_#inc_1.exit.i.us.i.i", label %50

50:                                               ; preds = %then_block.us.i.i
  %51 = atomicrmw add ptr %get_rc_ptr.i.i.i.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit.i.us.i.i"

"#Attr_#inc_1.exit.i.us.i.i":                     ; preds = %50, %then_block.us.i.i
  %call.i.i.us.i.i = tail call i64 @llvm.uadd.sat.i64(i64 %joinpointarg96.us.i.i, i64 %common.ret.op.i.i.i12.i.i.i)
  %x.y.i.i.us.i.i = tail call i64 @llvm.umin.i64(i64 %call.i.i.us.i.i, i64 %common.ret.op.i.i.i.i.i.i)
  store i64 %x.y.i.i.us.i.i, ptr %struct_alloca.i.i.i, align 8
  store ptr %tmp_output_for_jmp.sroa.0.0.copyload.i.i, ptr %struct_field_gep4.i.i.i, align 8
  store i64 %tmp_output_for_jmp.sroa.2.0.copyload.i.i, ptr %joinpoint_arg_alloca.sroa.7.0.struct_field_gep4.i.sroa_idx.i.i, align 8
  store i64 %tmp_output_for_jmp.sroa.3.0.copyload.i.i, ptr %joinpoint_arg_alloca.sroa.10.0.struct_field_gep4.i.sroa_idx.i.i, align 8
  store i64 %joinpointarg96.us.i.i, ptr %struct_field_gep5.i.i.i, align 8
  store ptr %tmp_output_for_jmp3.sroa.0.0.copyload.i.i, ptr %struct_field_gep6.i.i.i, align 8
  store i64 %tmp_output_for_jmp3.sroa.2.0.copyload.i.i, ptr %joinpoint_arg_alloca1.sroa.7.0.struct_field_gep6.i.sroa_idx.i.i, align 8
  store i64 %tmp_output_for_jmp3.sroa.3.0.copyload.i.i, ptr %joinpoint_arg_alloca1.sroa.11.0.struct_field_gep6.i.sroa_idx.i.i, align 8
  store i64 0, ptr %struct_field_gep7.i.i.i, align 8
  store i64 %common.ret.op.i.i.i12.i.i.i, ptr %struct_field_gep8.i.i.i, align 8
  %call9.i.us.i.i = call fastcc i1 @Str_matches_at_help_e3211b24ebda6e959f98c8dfabbb8bde232b75ae94930caa03d2bdb8e7b5674(ptr nonnull %struct_alloca.i.i.i)
  call void @llvm.lifetime.end.p0(i64 80, ptr nonnull %struct_alloca.i.i.i)
  br i1 %call9.i.us.i.i, label %"#Attr_#dec_1.exit56.i.i", label %else_block7.us.i.i

else_block7.us.i.i:                               ; preds = %"#Attr_#inc_1.exit.i.us.i.i"
  %add_int_wrap.i.us.i.i = add i64 %joinpointarg96.us.i.i, 1
  %lte_uint.i.not.us.i.i = icmp ugt i64 %add_int_wrap.i.us.i.i, %call.i.i
  br i1 %lte_uint.i.not.us.i.i, label %else_block.i.i, label %then_block.us.i.i

"#Attr_#inc_1.exit24.split.i.i":                  ; preds = %"#Attr_#inc_1.exit24.i.i"
  br i1 %is_big_str.i15.i.i.i, label %then_block.us99.i.i, label %then_block.i.i

then_block.us99.i.i:                              ; preds = %"#Attr_#inc_1.exit24.split.i.i", %else_block7.us107.i.i
  %joinpointarg96.us100.i.i = phi i64 [ %add_int_wrap.i.us108.i.i, %else_block7.us107.i.i ], [ 0, %"#Attr_#inc_1.exit24.split.i.i" ]
  call void @llvm.lifetime.start.p0(i64 80, ptr nonnull %struct_alloca.i.i.i)
  %52 = load i64, ptr %get_rc_ptr.i21.i.i.i, align 8
  %.not.i.i22.i.us102.i.i = icmp eq i64 %52, 0
  br i1 %.not.i.i22.i.us102.i.i, label %Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us103.i.i, label %53

53:                                               ; preds = %then_block.us99.i.i
  %54 = atomicrmw add ptr %get_rc_ptr.i21.i.i.i, i64 1 monotonic, align 8
  br label %Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us103.i.i

Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us103.i.i: ; preds = %53, %then_block.us99.i.i
  %call.i.i.us104.i.i = tail call i64 @llvm.uadd.sat.i64(i64 %joinpointarg96.us100.i.i, i64 %common.ret.op.i.i.i12.i.i.i)
  %x.y.i.i.us105.i.i = tail call i64 @llvm.umin.i64(i64 %call.i.i.us104.i.i, i64 %common.ret.op.i.i.i.i.i.i)
  store i64 %x.y.i.i.us105.i.i, ptr %struct_alloca.i.i.i, align 8
  store ptr %tmp_output_for_jmp.sroa.0.0.copyload.i.i, ptr %struct_field_gep4.i.i.i, align 8
  store i64 %tmp_output_for_jmp.sroa.2.0.copyload.i.i, ptr %joinpoint_arg_alloca.sroa.7.0.struct_field_gep4.i.sroa_idx.i.i, align 8
  store i64 %tmp_output_for_jmp.sroa.3.0.copyload.i.i, ptr %joinpoint_arg_alloca.sroa.10.0.struct_field_gep4.i.sroa_idx.i.i, align 8
  store i64 %joinpointarg96.us100.i.i, ptr %struct_field_gep5.i.i.i, align 8
  store ptr %tmp_output_for_jmp3.sroa.0.0.copyload.i.i, ptr %struct_field_gep6.i.i.i, align 8
  store i64 %tmp_output_for_jmp3.sroa.2.0.copyload.i.i, ptr %joinpoint_arg_alloca1.sroa.7.0.struct_field_gep6.i.sroa_idx.i.i, align 8
  store i64 %tmp_output_for_jmp3.sroa.3.0.copyload.i.i, ptr %joinpoint_arg_alloca1.sroa.11.0.struct_field_gep6.i.sroa_idx.i.i, align 8
  store i64 0, ptr %struct_field_gep7.i.i.i, align 8
  store i64 %common.ret.op.i.i.i12.i.i.i, ptr %struct_field_gep8.i.i.i, align 8
  %call9.i.us106.i.i = call fastcc i1 @Str_matches_at_help_e3211b24ebda6e959f98c8dfabbb8bde232b75ae94930caa03d2bdb8e7b5674(ptr nonnull %struct_alloca.i.i.i)
  call void @llvm.lifetime.end.p0(i64 80, ptr nonnull %struct_alloca.i.i.i)
  br i1 %call9.i.us106.i.i, label %modify_rc.i49.i.i, label %else_block7.us107.i.i

else_block7.us107.i.i:                            ; preds = %Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us103.i.i
  %add_int_wrap.i.us108.i.i = add i64 %joinpointarg96.us100.i.i, 1
  %lte_uint.i.not.us109.i.i = icmp ugt i64 %add_int_wrap.i.us108.i.i, %call.i.i
  br i1 %lte_uint.i.not.us109.i.i, label %else_block.i.i, label %then_block.us99.i.i

then_block.i.i:                                   ; preds = %"#Attr_#inc_1.exit24.split.i.i", %else_block7.i.i
  %joinpointarg96.i.i = phi i64 [ %add_int_wrap.i.i.i, %else_block7.i.i ], [ 0, %"#Attr_#inc_1.exit24.split.i.i" ]
  call void @llvm.lifetime.start.p0(i64 80, ptr nonnull %struct_alloca.i.i.i)
  %call.i.i.i.i = tail call i64 @llvm.uadd.sat.i64(i64 %joinpointarg96.i.i, i64 %common.ret.op.i.i.i12.i.i.i)
  %x.y.i.i.i.i = tail call i64 @llvm.umin.i64(i64 %call.i.i.i.i, i64 %common.ret.op.i.i.i.i.i.i)
  store i64 %x.y.i.i.i.i, ptr %struct_alloca.i.i.i, align 8
  store ptr %tmp_output_for_jmp.sroa.0.0.copyload.i.i, ptr %struct_field_gep4.i.i.i, align 8
  store i64 %tmp_output_for_jmp.sroa.2.0.copyload.i.i, ptr %joinpoint_arg_alloca.sroa.7.0.struct_field_gep4.i.sroa_idx.i.i, align 8
  store i64 %tmp_output_for_jmp.sroa.3.0.copyload.i.i, ptr %joinpoint_arg_alloca.sroa.10.0.struct_field_gep4.i.sroa_idx.i.i, align 8
  store i64 %joinpointarg96.i.i, ptr %struct_field_gep5.i.i.i, align 8
  store ptr %tmp_output_for_jmp3.sroa.0.0.copyload.i.i, ptr %struct_field_gep6.i.i.i, align 8
  store i64 %tmp_output_for_jmp3.sroa.2.0.copyload.i.i, ptr %joinpoint_arg_alloca1.sroa.7.0.struct_field_gep6.i.sroa_idx.i.i, align 8
  store i64 %tmp_output_for_jmp3.sroa.3.0.copyload.i.i, ptr %joinpoint_arg_alloca1.sroa.11.0.struct_field_gep6.i.sroa_idx.i.i, align 8
  store i64 0, ptr %struct_field_gep7.i.i.i, align 8
  store i64 %common.ret.op.i.i.i12.i.i.i, ptr %struct_field_gep8.i.i.i, align 8
  %call9.i.i.i = call fastcc i1 @Str_matches_at_help_e3211b24ebda6e959f98c8dfabbb8bde232b75ae94930caa03d2bdb8e7b5674(ptr nonnull %struct_alloca.i.i.i)
  call void @llvm.lifetime.end.p0(i64 80, ptr nonnull %struct_alloca.i.i.i)
  br i1 %call9.i.i.i, label %"#Attr_#dec_1.exit56.i.i", label %else_block7.i.i

else_block.i.i:                                   ; preds = %else_block7.i.i, %else_block7.us107.i.i, %else_block7.us.i.i, %else_block7.us.us.i.i
  br i1 %is_big_str.i15.i.i.i, label %modify_rc.i28.i.i, label %"#Attr_#dec_1.exit.i.i"

modify_rc.i28.i.i:                                ; preds = %else_block.i.i
  %55 = load i64, ptr %get_rc_ptr.i21.i.i.i, align 8
  %.not.i.i34.i.i = icmp eq i64 %55, 0
  br i1 %.not.i.i34.i.i, label %"#Attr_#dec_1.exit.i.i", label %56

56:                                               ; preds = %modify_rc.i28.i.i
  %57 = atomicrmw sub ptr %get_rc_ptr.i21.i.i.i, i64 1 monotonic, align 8
  %58 = icmp eq i64 %57, 1
  br i1 %58, label %59, label %"#Attr_#dec_1.exit.i.i"

59:                                               ; preds = %56
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i21.i.i.i, i32 8)
  br label %"#Attr_#dec_1.exit.i.i"

"#Attr_#dec_1.exit.i.i":                          ; preds = %59, %56, %modify_rc.i28.i.i, %else_block.i.i
  br i1 %is_big_str.i.i.i.i, label %modify_rc.i38.i.i, label %else_block

modify_rc.i38.i.i:                                ; preds = %"#Attr_#dec_1.exit.i.i"
  %60 = load i64, ptr %get_rc_ptr.i.i.i.i, align 8
  %.not.i.i44.i.i = icmp eq i64 %60, 0
  br i1 %.not.i.i44.i.i, label %else_block, label %61

61:                                               ; preds = %modify_rc.i38.i.i
  %62 = atomicrmw sub ptr %get_rc_ptr.i.i.i.i, i64 1 monotonic, align 8
  %63 = icmp eq i64 %62, 1
  br i1 %63, label %Str_first_match_4afec3b1b615e34b46f852dc4576722a03d82d96cc27deb38d7b350ecaf31.exit, label %else_block

modify_rc.i49.i.i:                                ; preds = %Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us103.i.i, %Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us.us.i.i
  %.us-phi98.ph.i.i = phi i64 [ %joinpointarg96.us.us.i.i, %Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us.us.i.i ], [ %joinpointarg96.us100.i.i, %Str_matches_at_d394208415ac8fe0ce8aa0ddf6a845c7cc74d818698e3d25c85705ce311f5ec.exit.us103.i.i ]
  %64 = load i64, ptr %get_rc_ptr.i21.i.i.i, align 8
  %.not.i.i55.i.i = icmp eq i64 %64, 0
  br i1 %.not.i.i55.i.i, label %"#Attr_#dec_1.exit56.i.i", label %65

65:                                               ; preds = %modify_rc.i49.i.i
  %66 = atomicrmw sub ptr %get_rc_ptr.i21.i.i.i, i64 1 monotonic, align 8
  %67 = icmp eq i64 %66, 1
  br i1 %67, label %68, label %"#Attr_#dec_1.exit56.i.i"

68:                                               ; preds = %65
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i21.i.i.i, i32 8)
  br label %"#Attr_#dec_1.exit56.i.i"

"#Attr_#dec_1.exit56.i.i":                        ; preds = %then_block.i.i, %"#Attr_#inc_1.exit.i.us.i.i", %68, %65, %modify_rc.i49.i.i
  %.us-phi98135.i.i = phi i64 [ %.us-phi98.ph.i.i, %modify_rc.i49.i.i ], [ %.us-phi98.ph.i.i, %65 ], [ %.us-phi98.ph.i.i, %68 ], [ %joinpointarg96.us.i.i, %"#Attr_#inc_1.exit.i.us.i.i" ], [ %joinpointarg96.i.i, %then_block.i.i ]
  br i1 %is_big_str.i.i.i.i, label %modify_rc.i60.i.i, label %then_block

modify_rc.i60.i.i:                                ; preds = %"#Attr_#dec_1.exit56.i.i"
  %69 = load i64, ptr %get_rc_ptr.i.i.i.i, align 8
  %.not.i.i66.i.i = icmp eq i64 %69, 0
  br i1 %.not.i.i66.i.i, label %then_block, label %70

70:                                               ; preds = %modify_rc.i60.i.i
  %71 = atomicrmw sub ptr %get_rc_ptr.i.i.i.i, i64 1 monotonic, align 8
  %72 = icmp eq i64 %71, 1
  br i1 %72, label %Str_first_match_4afec3b1b615e34b46f852dc4576722a03d82d96cc27deb38d7b350ecaf31.exit.thread47, label %then_block

Str_first_match_4afec3b1b615e34b46f852dc4576722a03d82d96cc27deb38d7b350ecaf31.exit.thread47: ; preds = %70
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i.i, i32 8)
  br label %then_block

else_block7.i.i:                                  ; preds = %then_block.i.i
  %add_int_wrap.i.i.i = add i64 %joinpointarg96.i.i, 1
  %lte_uint.i.not.i.i = icmp ugt i64 %add_int_wrap.i.i.i, %call.i.i
  br i1 %lte_uint.i.not.i.i, label %else_block.i.i, label %then_block.i.i

Str_first_match_4afec3b1b615e34b46f852dc4576722a03d82d96cc27deb38d7b350ecaf31.exit: ; preds = %61
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.i.i.i, i32 8)
  br label %else_block

common.ret:                                       ; preds = %120, %117, %modify_rc.i31, %else_block, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit
  %.sink = phi i8 [ 1, %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit ], [ 0, %else_block ], [ 0, %modify_rc.i31 ], [ 0, %117 ], [ 0, %120 ]
  %tag_alloca13.sroa.1.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 48
  store i8 %.sink, ptr %tag_alloca13.sroa.1.0..sroa_idx, align 8
  ret void

then_block:                                       ; preds = %"#Attr_#dec_1.exit56.i.i", %modify_rc.i60.i.i, %70, %Str_first_match_4afec3b1b615e34b46f852dc4576722a03d82d96cc27deb38d7b350ecaf31.exit.thread47
  %haystack.val = load i64, ptr %1, align 8
  %haystack.val19 = load i64, ptr %2, align 8
  %73 = icmp slt i64 %haystack.val19, 0
  %74 = lshr i64 %haystack.val19, 56
  %75 = and i64 %74, 127
  %76 = and i64 %haystack.val, 9223372036854775807
  %common.ret.op.i.i.i = select i1 %73, i64 %75, i64 %76
  %needle.val17 = load i64, ptr %7, align 8
  %needle.val18 = load i64, ptr %8, align 8
  %77 = icmp slt i64 %needle.val18, 0
  %78 = lshr i64 %needle.val18, 56
  %79 = and i64 %78, 127
  %80 = and i64 %needle.val17, 9223372036854775807
  %common.ret.op.i.i.i20 = select i1 %77, i64 %79, i64 %80
  %call.i = tail call { i64, i1 } @llvm.usub.with.overflow.i64(i64 %common.ret.op.i.i.i, i64 %common.ret.op.i.i.i20)
  %has_overflowed.i = extractvalue { i64, i1 } %call.i, 1
  br i1 %has_overflowed.i, label %throw_block.i, label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit

throw_block.i:                                    ; preds = %then_block
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit: ; preds = %then_block
  %operation_result.i = extractvalue { i64, i1 } %call.i, 0
  %call.i21 = tail call { i64, i1 } @llvm.usub.with.overflow.i64(i64 %operation_result.i, i64 %.us-phi98135.i.i)
  %has_overflowed.i22 = extractvalue { i64, i1 } %call.i21, 1
  br i1 %has_overflowed.i22, label %throw_block.i24, label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit25

throw_block.i24:                                  ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit25: ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit
  %operation_result.i23 = extractvalue { i64, i1 } %call.i21, 0
  %is_big_str.i = icmp sgt i64 %haystack.val19, 0
  br i1 %is_big_str.i, label %modify_rc.i, label %"#Attr_#inc_1.exit"

modify_rc.i:                                      ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit25
  %.sroa.0.0.copyload.i.i = load ptr, ptr %haystack, align 8
  %81 = ptrtoint ptr %.sroa.0.0.copyload.i.i to i64
  %82 = shl nuw i64 %haystack.val19, 1
  %isneg.i.i.i = icmp slt i64 %haystack.val, 0
  %83 = select i1 %isneg.i.i.i, i64 %82, i64 %81
  %84 = inttoptr i64 %83 to ptr
  %get_rc_ptr.i = getelementptr inbounds i64, ptr %84, i64 -1
  %85 = load i64, ptr %get_rc_ptr.i, align 8
  %.not.i.i = icmp eq i64 %85, 0
  br i1 %.not.i.i, label %"#Attr_#inc_1.exit.thread", label %86

86:                                               ; preds = %modify_rc.i
  %87 = atomicrmw add ptr %get_rc_ptr.i, i64 1 monotonic, align 8
  %.sroa.1.0.copyload.i.i.i.pre = load i64, ptr %2, align 8, !noalias !178
  br label %"#Attr_#inc_1.exit"

"#Attr_#inc_1.exit":                              ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit25, %86
  %.sroa.1.0.copyload.i.i.i = phi i64 [ %haystack.val19, %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit25 ], [ %.sroa.1.0.copyload.i.i.i.pre, %86 ]
  %88 = icmp slt i64 %.sroa.1.0.copyload.i.i.i, 0
  br i1 %88, label %89, label %"#Attr_#inc_1.exit.thread"

89:                                               ; preds = %"#Attr_#inc_1.exit"
  %.sroa.010.0.copyload.i.i.i = load i64, ptr %haystack, align 8, !noalias !178
  %.sroa.2.0.copyload.i.i.i = load i64, ptr %1, align 8, !noalias !178
  %90 = shl i64 %.us-phi98135.i.i, 56
  %.sroa.4.23.insert.mask.i.i.i = and i64 %.sroa.1.0.copyload.i.i.i, 72057594037927935
  %.sroa.4.23.insert.shift.i.i.i = or disjoint i64 %.sroa.4.23.insert.mask.i.i.i, %90
  %.sroa.4.23.insert.insert.i.i.i = or i64 %.sroa.4.23.insert.shift.i.i.i, -9223372036854775808
  br label %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit

"#Attr_#inc_1.exit.thread":                       ; preds = %modify_rc.i, %"#Attr_#inc_1.exit"
  %.sroa.1.0.copyload.i.i.i71 = phi i64 [ %.sroa.1.0.copyload.i.i.i, %"#Attr_#inc_1.exit" ], [ %haystack.val19, %modify_rc.i ]
  %91 = load ptr, ptr %haystack, align 8, !noalias !178
  %.not.i.i.i = icmp eq ptr %91, null
  %92 = ptrtoint ptr %91 to i64
  br i1 %.not.i.i.i, label %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit, label %93

93:                                               ; preds = %"#Attr_#inc_1.exit.thread"
  %.sroa.217.0.copyload.i.i.i = load i64, ptr %1, align 8, !noalias !178
  %94 = icmp slt i64 %.sroa.217.0.copyload.i.i.i, 0
  %95 = and i64 %.sroa.217.0.copyload.i.i.i, 9223372036854775807
  %spec.select.i.i.i.i.i.i = select i1 %94, i64 %95, i64 %.sroa.1.0.copyload.i.i.i71
  %96 = icmp ne i64 %spec.select.i.i.i.i.i.i, 0
  %.not1.i.i.i.i.i = or i1 %94, %96
  br i1 %.not1.i.i.i.i.i, label %str.RocStr.isUnique.exit.i.i.i, label %str.RocStr.setLen.exit9.i.i.i

str.RocStr.isUnique.exit.i.i.i:                   ; preds = %93
  %97 = shl nuw i64 %.sroa.1.0.copyload.i.i.i71, 1
  %98 = inttoptr i64 %97 to ptr
  %99 = select i1 %94, ptr %98, ptr %91
  %100 = getelementptr inbounds i64, ptr %99, i64 -1
  %101 = load i64, ptr %100, align 8, !noalias !178
  %102 = icmp eq i64 %101, 1
  br i1 %102, label %str.RocStr.setLen.exit9.i.i.i, label %.critedge.i.i.i

str.RocStr.setLen.exit9.i.i.i:                    ; preds = %str.RocStr.isUnique.exit.i.i.i, %93
  %103 = and i64 %.sroa.217.0.copyload.i.i.i, -9223372036854775808
  %104 = or i64 %103, %.us-phi98135.i.i
  br label %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit

.critedge.i.i.i:                                  ; preds = %str.RocStr.isUnique.exit.i.i.i
  %105 = lshr i64 %92, 1
  %106 = select i1 %94, i64 %.sroa.1.0.copyload.i.i.i71, i64 %105
  %107 = or i64 %.us-phi98135.i.i, -9223372036854775808
  br label %Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit

Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133.exit: ; preds = %"#Attr_#inc_1.exit.thread", %89, %str.RocStr.setLen.exit9.i.i.i, %.critedge.i.i.i
  %.sroa.9.0.i.i = phi i64 [ %.sroa.4.23.insert.insert.i.i.i, %89 ], [ %.sroa.1.0.copyload.i.i.i71, %str.RocStr.setLen.exit9.i.i.i ], [ %106, %.critedge.i.i.i ], [ -9223372036854775808, %"#Attr_#inc_1.exit.thread" ]
  %.sroa.6.0.i.i = phi i64 [ %.sroa.2.0.copyload.i.i.i, %89 ], [ %104, %str.RocStr.setLen.exit9.i.i.i ], [ %107, %.critedge.i.i.i ], [ 0, %"#Attr_#inc_1.exit.thread" ]
  %.sroa.0.0.i.i = phi i64 [ %.sroa.010.0.copyload.i.i.i, %89 ], [ %92, %str.RocStr.setLen.exit9.i.i.i ], [ %92, %.critedge.i.i.i ], [ 0, %"#Attr_#inc_1.exit.thread" ]
  %needle.val = load i64, ptr %7, align 8
  %needle.val16 = load i64, ptr %8, align 8
  %108 = icmp slt i64 %needle.val16, 0
  %109 = lshr i64 %needle.val16, 56
  %110 = and i64 %109, 127
  %111 = and i64 %needle.val, 9223372036854775807
  %common.ret.op.i.i.i27 = select i1 %108, i64 %110, i64 %111
  %add_int_wrap.i = add i64 %common.ret.op.i.i.i27, %.us-phi98135.i.i
  call fastcc void @Str_substring_unsafe_4fe2c0cee861629d2ef04c3f725dba5813b563598f88e6fe57cefd4dd1a133(ptr nonnull %haystack, i64 %add_int_wrap.i, i64 %operation_result.i23, ptr nonnull %tag_alloca.sroa.0)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %tag_alloca.sroa.0, i64 24, i1 false)
  %tag_alloca.sroa.2.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 24
  store i64 %.sroa.0.0.i.i, ptr %tag_alloca.sroa.2.0..sroa_idx, align 8
  %tag_alloca.sroa.3.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 32
  store i64 %.sroa.6.0.i.i, ptr %tag_alloca.sroa.3.0..sroa_idx, align 8
  %tag_alloca.sroa.4.0..sroa_idx = getelementptr inbounds i8, ptr %0, i64 40
  store i64 %.sroa.9.0.i.i, ptr %tag_alloca.sroa.4.0..sroa_idx, align 8
  br label %common.ret

else_block:                                       ; preds = %"#Attr_#dec_1.exit.i.i", %61, %modify_rc.i38.i.i, %Str_first_match_4afec3b1b615e34b46f852dc4576722a03d82d96cc27deb38d7b350ecaf31.exit
  %load_str_to_stack.unpack4.i29 = load i64, ptr %2, align 8
  %is_big_str.i30 = icmp sgt i64 %load_str_to_stack.unpack4.i29, 0
  br i1 %is_big_str.i30, label %modify_rc.i31, label %common.ret

modify_rc.i31:                                    ; preds = %else_block
  %.sroa.0.0.copyload.i.i32 = load ptr, ptr %haystack, align 8
  %.sroa.2.0.copyload.i.i34 = load i64, ptr %1, align 8
  %112 = ptrtoint ptr %.sroa.0.0.copyload.i.i32 to i64
  %113 = shl nuw i64 %load_str_to_stack.unpack4.i29, 1
  %isneg.i.i.i35 = icmp slt i64 %.sroa.2.0.copyload.i.i34, 0
  %114 = select i1 %isneg.i.i.i35, i64 %113, i64 %112
  %115 = inttoptr i64 %114 to ptr
  %get_rc_ptr.i36 = getelementptr inbounds i64, ptr %115, i64 -1
  %116 = load i64, ptr %get_rc_ptr.i36, align 8
  %.not.i.i37 = icmp eq i64 %116, 0
  br i1 %.not.i.i37, label %common.ret, label %117

117:                                              ; preds = %modify_rc.i31
  %118 = atomicrmw sub ptr %get_rc_ptr.i36, i64 1 monotonic, align 8
  %119 = icmp eq i64 %118, 1
  br i1 %119, label %120, label %common.ret

120:                                              ; preds = %117
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i36, i32 8)
  br label %common.ret
}

; Function Attrs: nounwind
define internal fastcc i64 @"#UserApp_check_b7aa9f7d377b2692ada596045493ead6d491b934dc9015fcbdd1a8e01477d"(ptr %tree) unnamed_addr #10 {
entry:
  %is_null = icmp eq ptr %tree, null
  br i1 %is_null, label %common.ret, label %else_block

common.ret:                                       ; preds = %entry, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit20
  %common.ret.op = phi i64 [ %operation_result.i18, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit20 ], [ 0, %entry ]
  ret i64 %common.ret.op

else_block:                                       ; preds = %entry
  %at_index_struct_gep_data = getelementptr inbounds { i64, ptr, ptr }, ptr %tree, i64 0, i32 1
  %load_at_index_ptr_old = load ptr, ptr %at_index_struct_gep_data, align 8
  %at_index_struct_gep_data1 = getelementptr inbounds { i64, ptr, ptr }, ptr %tree, i64 0, i32 2
  %load_at_index_ptr_old2 = load ptr, ptr %at_index_struct_gep_data1, align 8
  %get_rc_ptr = getelementptr inbounds i64, ptr %tree, i64 -1
  %get_refcount = load i64, ptr %get_rc_ptr, align 8
  %is_one = icmp eq i64 %get_refcount, 1
  br i1 %is_one, label %joinpointcont.sink.split, label %else_block5

else_block5:                                      ; preds = %else_block
  %is_null.i = icmp eq ptr %load_at_index_ptr_old, null
  br i1 %is_null.i, label %"#Attr_#inc_16.exit", label %should_recurse.i

should_recurse.i:                                 ; preds = %else_block5
  %get_rc_ptr.i = getelementptr inbounds i64, ptr %load_at_index_ptr_old, i64 -1
  %0 = load i64, ptr %get_rc_ptr.i, align 8
  %.not.i.i = icmp eq i64 %0, 0
  br i1 %.not.i.i, label %"#Attr_#inc_16.exit", label %1

1:                                                ; preds = %should_recurse.i
  %2 = atomicrmw add ptr %get_rc_ptr.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_16.exit"

"#Attr_#inc_16.exit":                             ; preds = %else_block5, %should_recurse.i, %1
  %is_null.i11 = icmp eq ptr %load_at_index_ptr_old2, null
  br i1 %is_null.i11, label %"#Attr_#inc_16.exit15", label %should_recurse.i12

should_recurse.i12:                               ; preds = %"#Attr_#inc_16.exit"
  %get_rc_ptr.i13 = getelementptr inbounds i64, ptr %load_at_index_ptr_old2, i64 -1
  %3 = load i64, ptr %get_rc_ptr.i13, align 8
  %.not.i.i14 = icmp eq i64 %3, 0
  br i1 %.not.i.i14, label %"#Attr_#inc_16.exit15", label %4

4:                                                ; preds = %should_recurse.i12
  %5 = atomicrmw add ptr %get_rc_ptr.i13, i64 1 monotonic, align 8
  br label %"#Attr_#inc_16.exit15"

"#Attr_#inc_16.exit15":                           ; preds = %"#Attr_#inc_16.exit", %should_recurse.i12, %4
  %6 = load i64, ptr %get_rc_ptr, align 8
  %.not.i = icmp eq i64 %6, 0
  br i1 %.not.i, label %joinpointcont, label %7

7:                                                ; preds = %"#Attr_#inc_16.exit15"
  %8 = atomicrmw sub ptr %get_rc_ptr, i64 1 monotonic, align 8
  %9 = icmp eq i64 %8, 1
  br i1 %9, label %joinpointcont.sink.split, label %joinpointcont

joinpointcont.sink.split:                         ; preds = %7, %else_block
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr, i32 8)
  br label %joinpointcont

joinpointcont:                                    ; preds = %joinpointcont.sink.split, %7, %"#Attr_#inc_16.exit15"
  %call = tail call fastcc i64 @"#UserApp_check_b7aa9f7d377b2692ada596045493ead6d491b934dc9015fcbdd1a8e01477d"(ptr %load_at_index_ptr_old)
  %call.i = tail call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %call, i64 1)
  %has_overflowed.i = extractvalue { i64, i1 } %call.i, 1
  br i1 %has_overflowed.i, label %throw_block.i, label %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit

throw_block.i:                                    ; preds = %joinpointcont
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit: ; preds = %joinpointcont
  %operation_result.i = extractvalue { i64, i1 } %call.i, 0
  %call9 = tail call fastcc i64 @"#UserApp_check_b7aa9f7d377b2692ada596045493ead6d491b934dc9015fcbdd1a8e01477d"(ptr %load_at_index_ptr_old2)
  %call.i16 = tail call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %operation_result.i, i64 %call9)
  %has_overflowed.i17 = extractvalue { i64, i1 } %call.i16, 1
  br i1 %has_overflowed.i17, label %throw_block.i19, label %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit20

throw_block.i19:                                  ; preds = %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit20: ; preds = %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit
  %operation_result.i18 = extractvalue { i64, i1 } %call.i16, 0
  br label %common.ret
}

; Function Attrs: nounwind
define internal fastcc i1 @Str_matches_at_help_e3211b24ebda6e959f98c8dfabbb8bde232b75ae94930caa03d2bdb8e7b5674(ptr nocapture readonly %state) unnamed_addr #10 {
entry:
  %0 = alloca [24 x i8], align 8
  %1 = alloca [24 x i8], align 8
  %struct_alloca = alloca { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, align 8
  %struct_field = load i64, ptr %state, align 8
  %struct_field_access_record_1 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %state, i64 0, i32 1
  %struct_field1.sroa.0.0.copyload = load ptr, ptr %struct_field_access_record_1, align 8
  %struct_field1.sroa.3.0.struct_field_access_record_1.sroa_idx = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %state, i64 0, i32 1, i32 1
  %struct_field1.sroa.3.0.copyload = load i64, ptr %struct_field1.sroa.3.0.struct_field_access_record_1.sroa_idx, align 8
  %struct_field1.sroa.5.0.struct_field_access_record_1.sroa_idx = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %state, i64 0, i32 1, i32 2
  %struct_field1.sroa.5.0.copyload = load i64, ptr %struct_field1.sroa.5.0.struct_field_access_record_1.sroa_idx, align 8
  %struct_field_access_record_2 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %state, i64 0, i32 2
  %struct_field2 = load i64, ptr %struct_field_access_record_2, align 8
  %struct_field_access_record_3 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %state, i64 0, i32 3
  %struct_field3.sroa.0.0.copyload = load ptr, ptr %struct_field_access_record_3, align 8
  %struct_field3.sroa.3.0.struct_field_access_record_3.sroa_idx = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %state, i64 0, i32 3, i32 1
  %struct_field3.sroa.3.0.copyload = load i64, ptr %struct_field3.sroa.3.0.struct_field_access_record_3.sroa_idx, align 8
  %struct_field3.sroa.5.0.struct_field_access_record_3.sroa_idx = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %state, i64 0, i32 3, i32 2
  %struct_field3.sroa.5.0.copyload = load i64, ptr %struct_field3.sroa.5.0.struct_field_access_record_3.sroa_idx, align 8
  %struct_field_access_record_4 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %state, i64 0, i32 4
  %struct_field4 = load i64, ptr %struct_field_access_record_4, align 8
  %struct_field_access_record_5 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %state, i64 0, i32 5
  %struct_field5 = load i64, ptr %struct_field_access_record_5, align 8
  %gte_uint.i.not = icmp ult i64 %struct_field2, %struct_field
  br i1 %gte_uint.i.not, label %else_block, label %then_block

common.ret:                                       ; preds = %Str_get_unsafe_127cb22736133e34b265d61ea1d591a8834a13a1d4a2cb161a40b74f7c37b3.exit51, %"#Attr_#dec_1.exit40"
  %common.ret.op = phi i1 [ %eq_u64.i, %"#Attr_#dec_1.exit40" ], [ %spec.select, %Str_get_unsafe_127cb22736133e34b265d61ea1d591a8834a13a1d4a2cb161a40b74f7c37b3.exit51 ]
  ret i1 %common.ret.op

then_block:                                       ; preds = %entry
  %is_big_str.i = icmp sgt i64 %struct_field3.sroa.5.0.copyload, 0
  br i1 %is_big_str.i, label %modify_rc.i, label %"#Attr_#dec_1.exit"

modify_rc.i:                                      ; preds = %then_block
  %2 = ptrtoint ptr %struct_field3.sroa.0.0.copyload to i64
  %3 = shl nuw i64 %struct_field3.sroa.5.0.copyload, 1
  %isneg.i.i.i = icmp slt i64 %struct_field3.sroa.3.0.copyload, 0
  %4 = select i1 %isneg.i.i.i, i64 %3, i64 %2
  %5 = inttoptr i64 %4 to ptr
  %get_rc_ptr.i = getelementptr inbounds i64, ptr %5, i64 -1
  %6 = load i64, ptr %get_rc_ptr.i, align 8
  %.not.i.i = icmp eq i64 %6, 0
  br i1 %.not.i.i, label %"#Attr_#dec_1.exit", label %7

7:                                                ; preds = %modify_rc.i
  %8 = atomicrmw sub ptr %get_rc_ptr.i, i64 1 monotonic, align 8
  %9 = icmp eq i64 %8, 1
  br i1 %9, label %10, label %"#Attr_#dec_1.exit"

10:                                               ; preds = %7
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i, i32 8)
  br label %"#Attr_#dec_1.exit"

"#Attr_#dec_1.exit":                              ; preds = %then_block, %modify_rc.i, %7, %10
  %is_big_str.i32 = icmp sgt i64 %struct_field1.sroa.5.0.copyload, 0
  br i1 %is_big_str.i32, label %modify_rc.i33, label %"#Attr_#dec_1.exit40"

modify_rc.i33:                                    ; preds = %"#Attr_#dec_1.exit"
  %11 = ptrtoint ptr %struct_field1.sroa.0.0.copyload to i64
  %12 = shl nuw i64 %struct_field1.sroa.5.0.copyload, 1
  %isneg.i.i.i37 = icmp slt i64 %struct_field1.sroa.3.0.copyload, 0
  %13 = select i1 %isneg.i.i.i37, i64 %12, i64 %11
  %14 = inttoptr i64 %13 to ptr
  %get_rc_ptr.i38 = getelementptr inbounds i64, ptr %14, i64 -1
  %15 = load i64, ptr %get_rc_ptr.i38, align 8
  %.not.i.i39 = icmp eq i64 %15, 0
  br i1 %.not.i.i39, label %"#Attr_#dec_1.exit40", label %16

16:                                               ; preds = %modify_rc.i33
  %17 = atomicrmw sub ptr %get_rc_ptr.i38, i64 1 monotonic, align 8
  %18 = icmp eq i64 %17, 1
  br i1 %18, label %19, label %"#Attr_#dec_1.exit40"

19:                                               ; preds = %16
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i38, i32 8)
  br label %"#Attr_#dec_1.exit40"

"#Attr_#dec_1.exit40":                            ; preds = %"#Attr_#dec_1.exit", %modify_rc.i33, %16, %19
  %eq_u64.i = icmp eq i64 %struct_field4, %struct_field5
  br label %common.ret

else_block:                                       ; preds = %entry
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1)
  %20 = icmp slt i64 %struct_field1.sroa.5.0.copyload, 0
  br i1 %20, label %21, label %23

21:                                               ; preds = %else_block
  store ptr %struct_field1.sroa.0.0.copyload, ptr %1, align 8
  %.sroa.3.0..sroa_idx2.i.i = getelementptr inbounds i8, ptr %1, i64 8
  store i64 %struct_field1.sroa.3.0.copyload, ptr %.sroa.3.0..sroa_idx2.i.i, align 8
  %.sroa.34.0..sroa_idx5.i.i = getelementptr inbounds i8, ptr %1, i64 16
  store i64 %struct_field1.sroa.5.0.copyload, ptr %.sroa.34.0..sroa_idx5.i.i, align 8
  %22 = getelementptr inbounds [24 x i8], ptr %1, i64 0, i64 %struct_field2
  br label %Str_get_unsafe_127cb22736133e34b265d61ea1d591a8834a13a1d4a2cb161a40b74f7c37b3.exit

23:                                               ; preds = %else_block
  %24 = icmp ne ptr %struct_field1.sroa.0.0.copyload, null
  tail call void @llvm.assume(i1 %24)
  %25 = getelementptr inbounds i8, ptr %struct_field1.sroa.0.0.copyload, i64 %struct_field2
  br label %Str_get_unsafe_127cb22736133e34b265d61ea1d591a8834a13a1d4a2cb161a40b74f7c37b3.exit

Str_get_unsafe_127cb22736133e34b265d61ea1d591a8834a13a1d4a2cb161a40b74f7c37b3.exit: ; preds = %21, %23
  %common.ret.op.in.i.i.i = phi ptr [ %22, %21 ], [ %25, %23 ]
  %common.ret.op.i.i.i = load i8, ptr %common.ret.op.in.i.i.i, align 1
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %0)
  %26 = icmp slt i64 %struct_field3.sroa.5.0.copyload, 0
  br i1 %26, label %27, label %29

27:                                               ; preds = %Str_get_unsafe_127cb22736133e34b265d61ea1d591a8834a13a1d4a2cb161a40b74f7c37b3.exit
  store ptr %struct_field3.sroa.0.0.copyload, ptr %0, align 8
  %.sroa.3.0..sroa_idx2.i.i49 = getelementptr inbounds i8, ptr %0, i64 8
  store i64 %struct_field3.sroa.3.0.copyload, ptr %.sroa.3.0..sroa_idx2.i.i49, align 8
  %.sroa.34.0..sroa_idx5.i.i50 = getelementptr inbounds i8, ptr %0, i64 16
  store i64 %struct_field3.sroa.5.0.copyload, ptr %.sroa.34.0..sroa_idx5.i.i50, align 8
  %28 = getelementptr inbounds [24 x i8], ptr %0, i64 0, i64 %struct_field4
  br label %Str_get_unsafe_127cb22736133e34b265d61ea1d591a8834a13a1d4a2cb161a40b74f7c37b3.exit51

29:                                               ; preds = %Str_get_unsafe_127cb22736133e34b265d61ea1d591a8834a13a1d4a2cb161a40b74f7c37b3.exit
  %30 = icmp ne ptr %struct_field3.sroa.0.0.copyload, null
  tail call void @llvm.assume(i1 %30)
  %31 = getelementptr inbounds i8, ptr %struct_field3.sroa.0.0.copyload, i64 %struct_field4
  br label %Str_get_unsafe_127cb22736133e34b265d61ea1d591a8834a13a1d4a2cb161a40b74f7c37b3.exit51

Str_get_unsafe_127cb22736133e34b265d61ea1d591a8834a13a1d4a2cb161a40b74f7c37b3.exit51: ; preds = %27, %29
  %common.ret.op.in.i.i.i47 = phi ptr [ %28, %27 ], [ %31, %29 ]
  %common.ret.op.i.i.i48 = load i8, ptr %common.ret.op.in.i.i.i47, align 1
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %0)
  %eq_u8.i = icmp eq i8 %common.ret.op.i.i.i, %common.ret.op.i.i.i48
  %struct_field_gep20 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca, i64 0, i32 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %struct_field_gep20, ptr noundef nonnull align 8 dereferenceable(24) %struct_field_access_record_1, i64 24, i1 false)
  %struct_field_gep22 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca, i64 0, i32 3
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %struct_field_gep22, ptr noundef nonnull align 8 dereferenceable(24) %struct_field_access_record_3, i64 24, i1 false)
  %add_int_wrap.i = add i64 %struct_field4, 1
  %add_int_wrap.i52 = add nuw i64 %struct_field2, 1
  store i64 %struct_field, ptr %struct_alloca, align 8
  %struct_field_gep21 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca, i64 0, i32 2
  store i64 %add_int_wrap.i52, ptr %struct_field_gep21, align 8
  %struct_field_gep23 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca, i64 0, i32 4
  store i64 %add_int_wrap.i, ptr %struct_field_gep23, align 8
  %struct_field_gep24 = getelementptr inbounds { i64, %str.RocStr, i64, %str.RocStr, i64, i64 }, ptr %struct_alloca, i64 0, i32 5
  store i64 %struct_field5, ptr %struct_field_gep24, align 8
  %call25 = call fastcc i1 @Str_matches_at_help_e3211b24ebda6e959f98c8dfabbb8bde232b75ae94930caa03d2bdb8e7b5674(ptr nonnull %struct_alloca)
  %spec.select = select i1 %eq_u8.i, i1 %call25, i1 false
  br label %common.ret
}

; Function Attrs: nounwind
define internal fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nocapture readonly %"#arg1", ptr nocapture readonly %"#arg2", ptr nocapture writeonly %0) unnamed_addr #10 {
entry:
  %1 = alloca [24 x i8], align 8
  %2 = alloca [24 x i8], align 8
  %3 = alloca [24 x i8], align 8
  %4 = alloca [24 x i8], align 8
  %5 = alloca [24 x i8], align 8
  %6 = alloca [24 x i8], align 8
  %7 = alloca [24 x i8], align 8
  %str_alloca = alloca %str.RocStr, align 8
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %5)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %6)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %7)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %7, ptr noundef nonnull align 8 dereferenceable(24) %"#arg1", i64 24, i1 false), !noalias !183
  %.sroa.2.sroa.0.0.copyload.i = load i64, ptr %"#arg2", align 8, !noalias !183
  %.sroa.2.sroa.2.0..sroa_idx.i = getelementptr inbounds i8, ptr %"#arg2", i64 8
  %.sroa.2.sroa.2.0.copyload.i = load i64, ptr %.sroa.2.sroa.2.0..sroa_idx.i, align 8, !noalias !183
  %.sroa.2.sroa.3.0..sroa_idx.i = getelementptr inbounds i8, ptr %"#arg2", i64 16
  %.sroa.2.sroa.3.0.copyload.i = load i64, ptr %.sroa.2.sroa.3.0..sroa_idx.i, align 8, !noalias !183
  %8 = icmp slt i64 %.sroa.2.sroa.3.0.copyload.i, 0
  %9 = lshr i64 %.sroa.2.sroa.3.0.copyload.i, 56
  %10 = and i64 %9, 127
  %11 = and i64 %.sroa.2.sroa.2.0.copyload.i, 9223372036854775807
  %common.ret.op.i.i.i = select i1 %8, i64 %10, i64 %11
  %12 = icmp eq i64 %common.ret.op.i.i.i, 0
  br i1 %12, label %roc_builtins.str.concat.exit, label %str.RocStr.len.exit.i

str.RocStr.len.exit.i:                            ; preds = %entry
  %.sroa.141.0..sroa_idx.i = getelementptr inbounds i8, ptr %7, i64 8
  %.sroa.141.0.copyload.i = load i64, ptr %.sroa.141.0..sroa_idx.i, align 8, !noalias !183
  %.sroa.2.0..sroa_idx.i = getelementptr inbounds i8, ptr %7, i64 16
  %.sroa.2.0.copyload.i = load i64, ptr %.sroa.2.0..sroa_idx.i, align 8, !noalias !183
  %13 = icmp slt i64 %.sroa.2.0.copyload.i, 0
  %14 = lshr i64 %.sroa.2.0.copyload.i, 56
  %15 = and i64 %14, 127
  %16 = and i64 %.sroa.141.0.copyload.i, 9223372036854775807
  %common.ret.op.i.i = select i1 %13, i64 %15, i64 %16
  %17 = add nuw i64 %common.ret.op.i.i, %common.ret.op.i.i.i
  %.sroa.044.0.copyload.i = load ptr, ptr %7, align 8, !noalias !183
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %1), !noalias !183
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %2), !noalias !183
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %3), !noalias !183
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %4), !noalias !183
  %18 = icmp slt i64 %.sroa.141.0.copyload.i, 0
  %or.cond.i = select i1 %13, i1 true, i1 %18
  br i1 %or.cond.i, label %.critedge.i.i, label %str.RocStr.getCapacity.exit.i.i.i.i

str.RocStr.getCapacity.exit.i.i.i.i:              ; preds = %str.RocStr.len.exit.i
  %.not32.i.i = icmp eq i64 %.sroa.2.0.copyload.i, 0
  %19 = ptrtoint ptr %.sroa.044.0.copyload.i to i64
  br i1 %.not32.i.i, label %str.RocStr.isUnique.exit.thread.i.i, label %str.RocStr.isUnique.exit.i.i

str.RocStr.isUnique.exit.i.i:                     ; preds = %str.RocStr.getCapacity.exit.i.i.i.i
  %20 = getelementptr inbounds i64, ptr %.sroa.044.0.copyload.i, i64 -1
  %21 = load i64, ptr %20, align 8, !noalias !186
  %22 = icmp eq i64 %21, 1
  br i1 %22, label %str.RocStr.isUnique.exit.thread.thread.i.i, label %.critedge.i.i

.critedge.i.i:                                    ; preds = %str.RocStr.isUnique.exit.i.i, %str.RocStr.len.exit.i
  store ptr %.sroa.044.0.copyload.i, ptr %4, align 8, !noalias !186
  %.sroa.4.0..sroa_idx47.i = getelementptr inbounds i8, ptr %4, i64 8
  store i64 %.sroa.141.0.copyload.i, ptr %.sroa.4.0..sroa_idx47.i, align 8, !noalias !186
  %.sroa.551.0..sroa_idx52.i = getelementptr inbounds i8, ptr %4, i64 16
  store i64 %.sroa.2.0.copyload.i, ptr %.sroa.551.0..sroa_idx52.i, align 8, !noalias !186
  call fastcc void @str.RocStr.reallocateFresh(ptr noalias %3, ptr nonnull readonly align 8 %4, i64 %17), !noalias !186
  %.sroa.056.0.copyload.i = load i64, ptr %3, align 8, !noalias !183
  %.sroa.559.0..sroa_idx.i = getelementptr inbounds i8, ptr %3, i64 8
  %.sroa.559.0.copyload.i = load i64, ptr %.sroa.559.0..sroa_idx.i, align 8, !noalias !183
  %.sroa.7.0..sroa_idx.i = getelementptr inbounds i8, ptr %3, i64 16
  %.sroa.7.0.copyload.i = load i64, ptr %.sroa.7.0..sroa_idx.i, align 8, !noalias !183
  br label %str.RocStr.reallocate.exit.i

str.RocStr.isUnique.exit.thread.i.i:              ; preds = %str.RocStr.getCapacity.exit.i.i.i.i
  %.not.i.i = icmp eq ptr %.sroa.044.0.copyload.i, null
  br i1 %.not.i.i, label %24, label %str.RocStr.isUnique.exit.thread.thread.i.i

str.RocStr.isUnique.exit.thread.thread.i.i:       ; preds = %str.RocStr.isUnique.exit.thread.i.i, %str.RocStr.isUnique.exit.i.i
  %23 = icmp ugt i64 %.sroa.2.0.copyload.i, %17
  br i1 %23, label %str.RocStr.reallocate.exit.thread.i, label %25

str.RocStr.reallocate.exit.thread.i:              ; preds = %str.RocStr.isUnique.exit.thread.thread.i.i
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1), !noalias !183
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %2), !noalias !183
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3), !noalias !183
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4), !noalias !183
  br label %str.RocStr.asU8ptrMut.exit.sink.split.i

24:                                               ; preds = %str.RocStr.isUnique.exit.thread.i.i
  store ptr null, ptr %2, align 8, !noalias !186
  %.sroa.4.0..sroa_idx49.i = getelementptr inbounds i8, ptr %2, i64 8
  store i64 %.sroa.141.0.copyload.i, ptr %.sroa.4.0..sroa_idx49.i, align 8, !noalias !186
  %.sroa.551.0..sroa_idx54.i = getelementptr inbounds i8, ptr %2, i64 16
  store i64 0, ptr %.sroa.551.0..sroa_idx54.i, align 8, !noalias !186
  call fastcc void @str.RocStr.reallocateFresh(ptr noalias %1, ptr nonnull readonly align 8 %2, i64 %17), !noalias !186
  %.sroa.056.0.copyload57.i = load i64, ptr %1, align 8, !noalias !183
  %.sroa.559.0..sroa_idx60.i = getelementptr inbounds i8, ptr %1, i64 8
  %.sroa.559.0.copyload61.i = load i64, ptr %.sroa.559.0..sroa_idx60.i, align 8, !noalias !183
  %.sroa.7.0..sroa_idx64.i = getelementptr inbounds i8, ptr %1, i64 16
  %.sroa.7.0.copyload65.i = load i64, ptr %.sroa.7.0..sroa_idx64.i, align 8, !noalias !183
  br label %str.RocStr.reallocate.exit.i

25:                                               ; preds = %str.RocStr.isUnique.exit.thread.thread.i.i
  %26 = add nuw i64 %.sroa.2.0.copyload.i, 1
  %.not6.i.i = icmp eq i64 %26, %17
  br i1 %.not6.i.i, label %35, label %27

27:                                               ; preds = %36, %25
  %28 = phi i64 [ %37, %36 ], [ %17, %25 ]
  %.not.i.i.i = icmp ult i64 %.sroa.2.0.copyload.i, %28
  br i1 %.not.i.i.i, label %29, label %str.RocStr.reallocate.exit.thread82.i

str.RocStr.reallocate.exit.thread82.i:            ; preds = %27
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1), !noalias !183
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %2), !noalias !183
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3), !noalias !183
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4), !noalias !183
  br label %str.RocStr.asU8ptrMut.exit.sink.split.i

29:                                               ; preds = %27
  %30 = add nuw i64 %28, 8
  %31 = add nuw i64 %.sroa.2.0.copyload.i, 8
  %32 = getelementptr inbounds i8, ptr %.sroa.044.0.copyload.i, i64 -8
  %33 = tail call ptr @roc_realloc(ptr nonnull align 1 %32, i64 %30, i64 %31, i32 8), !noalias !186
  %34 = getelementptr inbounds i8, ptr %33, i64 8
  %.pre.i = ptrtoint ptr %34 to i64
  br label %str.RocStr.reallocate.exit.i

35:                                               ; preds = %25
  br i1 %.not32.i.i, label %36, label %38

36:                                               ; preds = %46, %44, %40, %35
  %.sroa.0.0.i.i = phi i64 [ %41, %40 ], [ %45, %44 ], [ %49, %46 ], [ 64, %35 ]
  %37 = tail call i64 @llvm.umax.i64(i64 %.sroa.0.0.i.i, i64 %17)
  br label %27

38:                                               ; preds = %35
  %39 = icmp ult i64 %.sroa.2.0.copyload.i, 4096
  br i1 %39, label %40, label %42

40:                                               ; preds = %38
  %41 = shl nuw nsw i64 %.sroa.2.0.copyload.i, 1
  br label %36

42:                                               ; preds = %38
  %43 = icmp ugt i64 %.sroa.2.0.copyload.i, 131072
  br i1 %43, label %44, label %46

44:                                               ; preds = %42
  %45 = shl nuw i64 %.sroa.2.0.copyload.i, 1
  br label %36

46:                                               ; preds = %42
  %47 = mul nuw nsw i64 %.sroa.2.0.copyload.i, 3
  %48 = add nuw nsw i64 %47, 1
  %49 = lshr i64 %48, 1
  br label %36

str.RocStr.reallocate.exit.i:                     ; preds = %29, %24, %.critedge.i.i
  %.sroa.056.0.i = phi i64 [ %.sroa.056.0.copyload.i, %.critedge.i.i ], [ %.sroa.056.0.copyload57.i, %24 ], [ %.pre.i, %29 ]
  %.sroa.559.0.i = phi i64 [ %.sroa.559.0.copyload.i, %.critedge.i.i ], [ %.sroa.559.0.copyload61.i, %24 ], [ %17, %29 ]
  %.sroa.7.0.i = phi i64 [ %.sroa.7.0.copyload.i, %.critedge.i.i ], [ %.sroa.7.0.copyload65.i, %24 ], [ %28, %29 ]
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %1), !noalias !183
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %2), !noalias !183
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %3), !noalias !183
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %4), !noalias !183
  store i64 %.sroa.056.0.i, ptr %6, align 8, !noalias !183
  %.sroa.559.0..sroa_idx62.i = getelementptr inbounds i8, ptr %6, i64 8
  store i64 %.sroa.559.0.i, ptr %.sroa.559.0..sroa_idx62.i, align 8, !noalias !183
  %.sroa.7.0..sroa_idx66.i = getelementptr inbounds i8, ptr %6, i64 16
  store i64 %.sroa.7.0.i, ptr %.sroa.7.0..sroa_idx66.i, align 8, !noalias !183
  %50 = icmp slt i64 %.sroa.7.0.i, 0
  %51 = inttoptr i64 %.sroa.056.0.i to ptr
  %spec.select.i = select i1 %50, ptr %6, ptr %51
  br label %str.RocStr.asU8ptrMut.exit.i

str.RocStr.asU8ptrMut.exit.sink.split.i:          ; preds = %str.RocStr.reallocate.exit.thread82.i, %str.RocStr.reallocate.exit.thread.i
  %.sroa.2.0.copyload.sink.i = phi i64 [ %.sroa.2.0.copyload.i, %str.RocStr.reallocate.exit.thread.i ], [ %28, %str.RocStr.reallocate.exit.thread82.i ]
  store i64 %19, ptr %6, align 8, !noalias !183
  %.sroa.559.0..sroa_idx6277.i = getelementptr inbounds i8, ptr %6, i64 8
  store i64 %17, ptr %.sroa.559.0..sroa_idx6277.i, align 8, !noalias !183
  %.sroa.7.0..sroa_idx6678.i = getelementptr inbounds i8, ptr %6, i64 16
  store i64 %.sroa.2.0.copyload.sink.i, ptr %.sroa.7.0..sroa_idx6678.i, align 8, !noalias !183
  br label %str.RocStr.asU8ptrMut.exit.i

str.RocStr.asU8ptrMut.exit.i:                     ; preds = %str.RocStr.asU8ptrMut.exit.sink.split.i, %str.RocStr.reallocate.exit.i
  %common.ret.op.i13.i = phi ptr [ %spec.select.i, %str.RocStr.reallocate.exit.i ], [ %.sroa.044.0.copyload.i, %str.RocStr.asU8ptrMut.exit.sink.split.i ]
  %52 = getelementptr inbounds i8, ptr %common.ret.op.i13.i, i64 %common.ret.op.i.i
  store i64 %.sroa.2.sroa.0.0.copyload.i, ptr %5, align 8, !noalias !183
  %.sroa.5.0..sroa_idx30.i = getelementptr inbounds i8, ptr %5, i64 8
  store i64 %.sroa.2.sroa.2.0.copyload.i, ptr %.sroa.5.0..sroa_idx30.i, align 8, !noalias !183
  %.sroa.6.0..sroa_idx35.i = getelementptr inbounds i8, ptr %5, i64 16
  store i64 %.sroa.2.sroa.3.0.copyload.i, ptr %.sroa.6.0..sroa_idx35.i, align 8, !noalias !183
  %.cast.i = inttoptr i64 %.sroa.2.sroa.0.0.copyload.i to ptr
  %common.ret.op.i1980.i = select i1 %8, ptr %5, ptr %.cast.i
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %52, ptr align 1 %common.ret.op.i1980.i, i64 %common.ret.op.i.i.i, i1 false), !noalias !183
  br label %roc_builtins.str.concat.exit

roc_builtins.str.concat.exit:                     ; preds = %entry, %str.RocStr.asU8ptrMut.exit.i
  %53 = phi ptr [ %6, %str.RocStr.asU8ptrMut.exit.i ], [ %7, %entry ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %str_alloca, ptr noundef nonnull align 8 dereferenceable(24) %53, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %5)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %6)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %7)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %str_alloca, i64 24, i1 false)
  ret void
}

; Function Attrs: nounwind
define internal fastcc void @Str_replace_each_f0adb8f180253d489b50ac5199522556362f583929ee5e65c919bd9ed2bc82f(ptr nocapture readonly %haystack, ptr nocapture readonly %needle, ptr nocapture readonly %flower, ptr nocapture writeonly %0) unnamed_addr #10 {
entry:
  %result_value15.i = alloca %str.RocStr, align 8
  %result_value9.i = alloca %str.RocStr, align 8
  %struct_field8.i = alloca %str.RocStr, align 8
  %load_element.i = alloca { %str.RocStr, %str.RocStr }, align 8
  %result_value.i = alloca { [6 x i64], i8 }, align 8
  %joinpoint_arg_alloca3.i = alloca %str.RocStr, align 8
  %joinpoint_arg_alloca2.i = alloca %str.RocStr, align 8
  %joinpoint_arg_alloca1.i = alloca %str.RocStr, align 8
  %result_value5 = alloca %str.RocStr, align 8
  %result_value4 = alloca %str.RocStr, align 8
  %result_value3 = alloca %str.RocStr, align 8
  %struct_field2 = alloca %str.RocStr, align 8
  %load_element = alloca { %str.RocStr, %str.RocStr }, align 8
  %result_value = alloca { [6 x i64], i8 }, align 8
  %load_str_to_stack.elt3.i = getelementptr inbounds %str.RocStr, ptr %haystack, i64 0, i32 2
  %load_str_to_stack.unpack4.i = load i64, ptr %load_str_to_stack.elt3.i, align 8
  %is_big_str.i = icmp sgt i64 %load_str_to_stack.unpack4.i, 0
  br i1 %is_big_str.i, label %modify_rc.i, label %"#Attr_#inc_1.exit"

modify_rc.i:                                      ; preds = %entry
  %.sroa.0.0.copyload.i.i = load ptr, ptr %haystack, align 8
  %.sroa.2.0..sroa_idx.i.i = getelementptr inbounds i8, ptr %haystack, i64 8
  %.sroa.2.0.copyload.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i, align 8
  %1 = ptrtoint ptr %.sroa.0.0.copyload.i.i to i64
  %2 = shl nuw i64 %load_str_to_stack.unpack4.i, 1
  %isneg.i.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i, 0
  %3 = select i1 %isneg.i.i.i, i64 %2, i64 %1
  %4 = inttoptr i64 %3 to ptr
  %get_rc_ptr.i = getelementptr inbounds i64, ptr %4, i64 -1
  %5 = load i64, ptr %get_rc_ptr.i, align 8
  %.not.i.i = icmp eq i64 %5, 0
  br i1 %.not.i.i, label %"#Attr_#inc_1.exit", label %6

6:                                                ; preds = %modify_rc.i
  %7 = atomicrmw add ptr %get_rc_ptr.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit"

"#Attr_#inc_1.exit":                              ; preds = %entry, %modify_rc.i, %6
  call fastcc void @Str_split_first_bfa1d47a221bdaf089999196bed323c433d1a6b8c78ec612e6fa7b3e3d811(ptr nonnull %haystack, ptr %needle, ptr nonnull %result_value)
  %tag_id_ptr = getelementptr inbounds { [6 x i64], i8 }, ptr %result_value, i64 0, i32 1
  %load_tag_id = load i8, ptr %tag_id_ptr, align 8
  %eq_u8 = icmp eq i8 %load_tag_id, 1
  br i1 %eq_u8, label %then_block, label %"#Attr_#dec_2.exit"

common.ret:                                       ; preds = %"#Attr_#dec_2.exit", %Str_replace_each_help_95dbc324453309f26dee9436b39568cc8bcbe17ef409e9273c4edb58653fd.exit
  ret void

then_block:                                       ; preds = %"#Attr_#inc_1.exit"
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %load_element, ptr noundef nonnull align 8 dereferenceable(48) %result_value, i64 48, i1 false)
  %load_element.24.sroa_idx = getelementptr inbounds i8, ptr %load_element, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %struct_field2, ptr noundef nonnull align 8 dereferenceable(24) %load_element.24.sroa_idx, i64 24, i1 false)
  %8 = getelementptr i8, ptr %haystack, i64 8
  %haystack.val = load i64, ptr %8, align 8
  %haystack.val7 = load i64, ptr %load_str_to_stack.elt3.i, align 8
  %9 = icmp slt i64 %haystack.val7, 0
  %10 = lshr i64 %haystack.val7, 56
  %11 = and i64 %10, 127
  %12 = and i64 %haystack.val, 9223372036854775807
  %common.ret.op.i.i.i = select i1 %9, i64 %11, i64 %12
  %is_big_str.i10 = icmp sgt i64 %haystack.val7, 0
  br i1 %is_big_str.i10, label %modify_rc.i11, label %"#Attr_#dec_1.exit"

modify_rc.i11:                                    ; preds = %then_block
  %.sroa.0.0.copyload.i.i12 = load ptr, ptr %haystack, align 8
  %13 = ptrtoint ptr %.sroa.0.0.copyload.i.i12 to i64
  %14 = shl nuw i64 %haystack.val7, 1
  %isneg.i.i.i15 = icmp slt i64 %haystack.val, 0
  %15 = select i1 %isneg.i.i.i15, i64 %14, i64 %13
  %16 = inttoptr i64 %15 to ptr
  %get_rc_ptr.i16 = getelementptr inbounds i64, ptr %16, i64 -1
  %17 = load i64, ptr %get_rc_ptr.i16, align 8
  %.not.i.i17 = icmp eq i64 %17, 0
  br i1 %.not.i.i17, label %"#Attr_#dec_1.exit", label %18

18:                                               ; preds = %modify_rc.i11
  %19 = atomicrmw sub ptr %get_rc_ptr.i16, i64 1 monotonic, align 8
  %20 = icmp eq i64 %19, 1
  br i1 %20, label %21, label %"#Attr_#dec_1.exit"

21:                                               ; preds = %18
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i16, i32 8)
  br label %"#Attr_#dec_1.exit"

"#Attr_#dec_1.exit":                              ; preds = %then_block, %modify_rc.i11, %18, %21
  %22 = icmp ugt i64 %common.ret.op.i.i.i, 23
  br i1 %22, label %24, label %str.RocStr.asU8ptrMut.exit.i.i.i

str.RocStr.asU8ptrMut.exit.i.i.i:                 ; preds = %"#Attr_#dec_1.exit"
  %23 = or disjoint i64 %common.ret.op.i.i.i, 128
  br label %Str_with_capacity_839d816e30d259b7a113479095a2bf4a37efc9c09ed13ea6ca0fa9b9a5ff9.exit

24:                                               ; preds = %"#Attr_#dec_1.exit"
  %25 = add nuw i64 %common.ret.op.i.i.i, 8
  %26 = tail call ptr @roc_alloc(i64 %25, i32 8), !noalias !189
  %27 = icmp ne ptr %26, null
  tail call void @llvm.assume(i1 %27)
  %28 = getelementptr inbounds i8, ptr %26, i64 8
  store i64 1, ptr %26, align 8, !noalias !189
  %.sroa.7.16.extract.shift.i.i = lshr i64 %common.ret.op.i.i.i, 56
  %29 = ptrtoint ptr %28 to i64
  %30 = and i64 %common.ret.op.i.i.i, 72057594037927935
  br label %Str_with_capacity_839d816e30d259b7a113479095a2bf4a37efc9c09ed13ea6ca0fa9b9a5ff9.exit

Str_with_capacity_839d816e30d259b7a113479095a2bf4a37efc9c09ed13ea6ca0fa9b9a5ff9.exit: ; preds = %str.RocStr.asU8ptrMut.exit.i.i.i, %24
  %.sroa.49.0.i.i = phi i64 [ %30, %24 ], [ 0, %str.RocStr.asU8ptrMut.exit.i.i.i ]
  %.sroa.7.0.i.i = phi i64 [ %.sroa.7.16.extract.shift.i.i, %24 ], [ %23, %str.RocStr.asU8ptrMut.exit.i.i.i ]
  %.sink13.i.i.i = phi i64 [ %29, %24 ], [ 0, %str.RocStr.asU8ptrMut.exit.i.i.i ]
  %.sink.i.i.i = phi i64 [ %common.ret.op.i.i.i, %24 ], [ 0, %str.RocStr.asU8ptrMut.exit.i.i.i ]
  %.sroa.7.16.insert.shift.i.i = shl nuw i64 %.sroa.7.0.i.i, 56
  %.sroa.49.16.insert.insert.i.i = or disjoint i64 %.sroa.7.16.insert.shift.i.i, %.sroa.49.0.i.i
  %31 = icmp slt i64 %.sroa.49.16.insert.insert.i.i, 0
  %.sroa.4.23.insert.insert.i.i = or disjoint i64 %.sroa.49.0.i.i, -9223372036854775808
  %.sroa.4.0.i.i = select i1 %31, i64 %.sroa.4.23.insert.insert.i.i, i64 %.sroa.49.16.insert.insert.i.i
  %.sroa.2.0.i.i = select i1 %31, i64 %.sink.i.i.i, i64 0
  store i64 %.sink13.i.i.i, ptr %result_value3, align 8
  %str_alloca.sroa.2.0..sroa_idx.i = getelementptr inbounds i8, ptr %result_value3, i64 8
  store i64 %.sroa.2.0.i.i, ptr %str_alloca.sroa.2.0..sroa_idx.i, align 8
  %str_alloca.sroa.3.0..sroa_idx.i = getelementptr inbounds i8, ptr %result_value3, i64 16
  store i64 %.sroa.4.0.i.i, ptr %str_alloca.sroa.3.0..sroa_idx.i, align 8
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value3, ptr nonnull %struct_field2, ptr nonnull %result_value4)
  %load_str_to_stack.elt3.i18 = getelementptr inbounds %str.RocStr, ptr %struct_field2, i64 0, i32 2
  %load_str_to_stack.unpack4.i19 = load i64, ptr %load_str_to_stack.elt3.i18, align 8
  %is_big_str.i20 = icmp sgt i64 %load_str_to_stack.unpack4.i19, 0
  br i1 %is_big_str.i20, label %modify_rc.i21, label %"#Attr_#dec_1.exit28"

modify_rc.i21:                                    ; preds = %Str_with_capacity_839d816e30d259b7a113479095a2bf4a37efc9c09ed13ea6ca0fa9b9a5ff9.exit
  %.sroa.0.0.copyload.i.i22 = load ptr, ptr %struct_field2, align 8
  %.sroa.2.0..sroa_idx.i.i23 = getelementptr inbounds i8, ptr %struct_field2, i64 8
  %.sroa.2.0.copyload.i.i24 = load i64, ptr %.sroa.2.0..sroa_idx.i.i23, align 8
  %32 = ptrtoint ptr %.sroa.0.0.copyload.i.i22 to i64
  %33 = shl nuw i64 %load_str_to_stack.unpack4.i19, 1
  %isneg.i.i.i25 = icmp slt i64 %.sroa.2.0.copyload.i.i24, 0
  %34 = select i1 %isneg.i.i.i25, i64 %33, i64 %32
  %35 = inttoptr i64 %34 to ptr
  %get_rc_ptr.i26 = getelementptr inbounds i64, ptr %35, i64 -1
  %36 = load i64, ptr %get_rc_ptr.i26, align 8
  %.not.i.i27 = icmp eq i64 %36, 0
  br i1 %.not.i.i27, label %"#Attr_#dec_1.exit28", label %37

37:                                               ; preds = %modify_rc.i21
  %38 = atomicrmw sub ptr %get_rc_ptr.i26, i64 1 monotonic, align 8
  %39 = icmp eq i64 %38, 1
  br i1 %39, label %40, label %"#Attr_#dec_1.exit28"

40:                                               ; preds = %37
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i26, i32 8)
  br label %"#Attr_#dec_1.exit28"

"#Attr_#dec_1.exit28":                            ; preds = %Str_with_capacity_839d816e30d259b7a113479095a2bf4a37efc9c09ed13ea6ca0fa9b9a5ff9.exit, %modify_rc.i21, %37, %40
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value4, ptr %flower, ptr nonnull %result_value5)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %result_value9.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %struct_field8.i)
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %load_element.i)
  call void @llvm.lifetime.start.p0(i64 56, ptr nonnull %result_value.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %joinpoint_arg_alloca3.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %joinpoint_arg_alloca2.i)
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %joinpoint_arg_alloca1.i)
  %load_str_to_stack.elt3.i.i = getelementptr inbounds %str.RocStr, ptr %flower, i64 0, i32 2
  %load_str_to_stack.unpack4.i.i = load i64, ptr %load_str_to_stack.elt3.i.i, align 8
  %is_big_str.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i.i, 0
  br i1 %is_big_str.i.i, label %modify_rc.i.i, label %"#Attr_#inc_1.exit.i"

modify_rc.i.i:                                    ; preds = %"#Attr_#dec_1.exit28"
  %.sroa.0.0.copyload.i.i.i = load ptr, ptr %flower, align 8
  %.sroa.2.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %flower, i64 8
  %.sroa.2.0.copyload.i.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i.i, align 8
  %41 = ptrtoint ptr %.sroa.0.0.copyload.i.i.i to i64
  %42 = shl nuw i64 %load_str_to_stack.unpack4.i.i, 1
  %isneg.i.i.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i.i, 0
  %43 = select i1 %isneg.i.i.i.i, i64 %42, i64 %41
  %44 = inttoptr i64 %43 to ptr
  %get_rc_ptr.i.i = getelementptr inbounds i64, ptr %44, i64 -1
  %45 = load i64, ptr %get_rc_ptr.i.i, align 8
  %.not.i.i.i = icmp eq i64 %45, 0
  br i1 %.not.i.i.i, label %"#Attr_#inc_1.exit.i", label %46

46:                                               ; preds = %modify_rc.i.i
  %47 = atomicrmw add ptr %get_rc_ptr.i.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit.i"

"#Attr_#inc_1.exit.i":                            ; preds = %46, %modify_rc.i.i, %"#Attr_#dec_1.exit28"
  %load_str_to_stack.elt3.i16.i = getelementptr inbounds %str.RocStr, ptr %needle, i64 0, i32 2
  %load_str_to_stack.unpack4.i17.i = load i64, ptr %load_str_to_stack.elt3.i16.i, align 8
  %is_big_str.i18.i = icmp sgt i64 %load_str_to_stack.unpack4.i17.i, 0
  br i1 %is_big_str.i18.i, label %modify_rc.i19.i, label %"#Attr_#inc_1.exit26.i"

modify_rc.i19.i:                                  ; preds = %"#Attr_#inc_1.exit.i"
  %.sroa.0.0.copyload.i.i20.i = load ptr, ptr %needle, align 8
  %.sroa.2.0..sroa_idx.i.i21.i = getelementptr inbounds i8, ptr %needle, i64 8
  %.sroa.2.0.copyload.i.i22.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i21.i, align 8
  %48 = ptrtoint ptr %.sroa.0.0.copyload.i.i20.i to i64
  %49 = shl nuw i64 %load_str_to_stack.unpack4.i17.i, 1
  %isneg.i.i.i23.i = icmp slt i64 %.sroa.2.0.copyload.i.i22.i, 0
  %50 = select i1 %isneg.i.i.i23.i, i64 %49, i64 %48
  %51 = inttoptr i64 %50 to ptr
  %get_rc_ptr.i24.i = getelementptr inbounds i64, ptr %51, i64 -1
  %52 = load i64, ptr %get_rc_ptr.i24.i, align 8
  %.not.i.i25.i = icmp eq i64 %52, 0
  br i1 %.not.i.i25.i, label %"#Attr_#inc_1.exit26.i", label %53

53:                                               ; preds = %modify_rc.i19.i
  %54 = atomicrmw add ptr %get_rc_ptr.i24.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit26.i"

"#Attr_#inc_1.exit26.i":                          ; preds = %53, %modify_rc.i19.i, %"#Attr_#inc_1.exit.i"
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %joinpoint_arg_alloca1.i, ptr noundef nonnull align 8 dereferenceable(24) %result_value, i64 24, i1 false)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %joinpoint_arg_alloca2.i, ptr noundef nonnull align 8 dereferenceable(24) %needle, i64 24, i1 false)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %joinpoint_arg_alloca3.i, ptr noundef nonnull align 8 dereferenceable(24) %flower, i64 24, i1 false)
  %load_str_to_stack.elt3.i27.i = getelementptr inbounds %str.RocStr, ptr %joinpoint_arg_alloca1.i, i64 0, i32 2
  %tag_id_ptr92.i = getelementptr inbounds { [6 x i64], i8 }, ptr %result_value.i, i64 0, i32 1
  %.sroa.2.0..sroa_idx.i.i32.i = getelementptr inbounds i8, ptr %joinpoint_arg_alloca1.i, i64 8
  %load_element.24.sroa_idx.i = getelementptr inbounds i8, ptr %load_element.i, i64 24
  %load_str_to_stack.elt3.i48.i = getelementptr inbounds %str.RocStr, ptr %struct_field8.i, i64 0, i32 2
  %.sroa.2.0..sroa_idx.i.i53.i = getelementptr inbounds i8, ptr %struct_field8.i, i64 8
  br label %joinpointcont.i

joinpointcont.i:                                  ; preds = %"#Attr_#dec_1.exit58.i", %"#Attr_#inc_1.exit26.i"
  %load_str_to_stack.unpack4.i28.i = load i64, ptr %load_str_to_stack.elt3.i27.i, align 8
  %is_big_str.i29.i = icmp sgt i64 %load_str_to_stack.unpack4.i28.i, 0
  br i1 %is_big_str.i29.i, label %modify_rc.i30.i, label %"#Attr_#inc_1.exit37.thread.i"

modify_rc.i30.i:                                  ; preds = %joinpointcont.i
  %.sroa.0.0.copyload.i.i31.i = load ptr, ptr %joinpoint_arg_alloca1.i, align 8
  %.sroa.2.0.copyload.i.i33.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i32.i, align 8
  %55 = ptrtoint ptr %.sroa.0.0.copyload.i.i31.i to i64
  %56 = shl nuw i64 %load_str_to_stack.unpack4.i28.i, 1
  %isneg.i.i.i34.i = icmp slt i64 %.sroa.2.0.copyload.i.i33.i, 0
  %57 = select i1 %isneg.i.i.i34.i, i64 %56, i64 %55
  %58 = inttoptr i64 %57 to ptr
  %get_rc_ptr.i35.i = getelementptr inbounds i64, ptr %58, i64 -1
  %59 = load i64, ptr %get_rc_ptr.i35.i, align 8
  %.not.i.i36.i = icmp eq i64 %59, 0
  br i1 %.not.i.i36.i, label %"#Attr_#inc_1.exit37.i", label %60

60:                                               ; preds = %modify_rc.i30.i
  %61 = atomicrmw add ptr %get_rc_ptr.i35.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_1.exit37.i"

"#Attr_#inc_1.exit37.i":                          ; preds = %60, %modify_rc.i30.i
  call fastcc void @Str_split_first_bfa1d47a221bdaf089999196bed323c433d1a6b8c78ec612e6fa7b3e3d811(ptr nonnull %joinpoint_arg_alloca1.i, ptr nonnull %joinpoint_arg_alloca2.i, ptr nonnull %result_value.i)
  %load_tag_id.i = load i8, ptr %tag_id_ptr92.i, align 8
  %eq_u8.i = icmp eq i8 %load_tag_id.i, 1
  br i1 %eq_u8.i, label %then_block.i, label %"#Attr_#dec_2.exit.i"

"#Attr_#inc_1.exit37.thread.i":                   ; preds = %joinpointcont.i
  call fastcc void @Str_split_first_bfa1d47a221bdaf089999196bed323c433d1a6b8c78ec612e6fa7b3e3d811(ptr nonnull %joinpoint_arg_alloca1.i, ptr nonnull %joinpoint_arg_alloca2.i, ptr nonnull %result_value.i)
  %load_tag_id93.i = load i8, ptr %tag_id_ptr92.i, align 8
  %eq_u894.i = icmp eq i8 %load_tag_id93.i, 1
  br i1 %eq_u894.i, label %"#Attr_#dec_1.exit.i", label %"#Attr_#dec_2.exit.i"

then_block.i:                                     ; preds = %"#Attr_#inc_1.exit37.i"
  %62 = load i64, ptr %get_rc_ptr.i35.i, align 8
  %.not.i.i47.i = icmp eq i64 %62, 0
  br i1 %.not.i.i47.i, label %"#Attr_#dec_1.exit.i", label %63

63:                                               ; preds = %then_block.i
  %64 = atomicrmw sub ptr %get_rc_ptr.i35.i, i64 1 monotonic, align 8
  %65 = icmp eq i64 %64, 1
  br i1 %65, label %66, label %"#Attr_#dec_1.exit.i"

66:                                               ; preds = %63
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i35.i, i32 8)
  br label %"#Attr_#dec_1.exit.i"

"#Attr_#dec_1.exit.i":                            ; preds = %66, %63, %then_block.i, %"#Attr_#inc_1.exit37.thread.i"
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %load_element.i, ptr noundef nonnull align 8 dereferenceable(48) %result_value.i, i64 48, i1 false)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %struct_field8.i, ptr noundef nonnull align 8 dereferenceable(24) %load_element.24.sroa_idx.i, i64 24, i1 false)
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value5, ptr nonnull %struct_field8.i, ptr nonnull %result_value9.i)
  %load_str_to_stack.unpack4.i49.i = load i64, ptr %load_str_to_stack.elt3.i48.i, align 8
  %is_big_str.i50.i = icmp sgt i64 %load_str_to_stack.unpack4.i49.i, 0
  br i1 %is_big_str.i50.i, label %modify_rc.i51.i, label %"#Attr_#dec_1.exit58.i"

modify_rc.i51.i:                                  ; preds = %"#Attr_#dec_1.exit.i"
  %.sroa.0.0.copyload.i.i52.i = load ptr, ptr %struct_field8.i, align 8
  %.sroa.2.0.copyload.i.i54.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i53.i, align 8
  %67 = ptrtoint ptr %.sroa.0.0.copyload.i.i52.i to i64
  %68 = shl nuw i64 %load_str_to_stack.unpack4.i49.i, 1
  %isneg.i.i.i55.i = icmp slt i64 %.sroa.2.0.copyload.i.i54.i, 0
  %69 = select i1 %isneg.i.i.i55.i, i64 %68, i64 %67
  %70 = inttoptr i64 %69 to ptr
  %get_rc_ptr.i56.i = getelementptr inbounds i64, ptr %70, i64 -1
  %71 = load i64, ptr %get_rc_ptr.i56.i, align 8
  %.not.i.i57.i = icmp eq i64 %71, 0
  br i1 %.not.i.i57.i, label %"#Attr_#dec_1.exit58.i", label %72

72:                                               ; preds = %modify_rc.i51.i
  %73 = atomicrmw sub ptr %get_rc_ptr.i56.i, i64 1 monotonic, align 8
  %74 = icmp eq i64 %73, 1
  br i1 %74, label %75, label %"#Attr_#dec_1.exit58.i"

75:                                               ; preds = %72
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i56.i, i32 8)
  br label %"#Attr_#dec_1.exit58.i"

"#Attr_#dec_1.exit58.i":                          ; preds = %75, %72, %modify_rc.i51.i, %"#Attr_#dec_1.exit.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value9.i, ptr nonnull %joinpoint_arg_alloca3.i, ptr nonnull %result_value5)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %joinpoint_arg_alloca1.i, ptr noundef nonnull align 8 dereferenceable(24) %result_value.i, i64 24, i1 false)
  br label %joinpointcont.i

"#Attr_#dec_2.exit.i":                            ; preds = %"#Attr_#inc_1.exit37.thread.i", %"#Attr_#inc_1.exit37.i"
  %load_str_to_stack.elt3.i59.i = getelementptr inbounds %str.RocStr, ptr %joinpoint_arg_alloca2.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i60.i = load i64, ptr %load_str_to_stack.elt3.i59.i, align 8
  %is_big_str.i61.i = icmp sgt i64 %load_str_to_stack.unpack4.i60.i, 0
  br i1 %is_big_str.i61.i, label %modify_rc.i62.i, label %"#Attr_#dec_1.exit69.i"

modify_rc.i62.i:                                  ; preds = %"#Attr_#dec_2.exit.i"
  %.sroa.0.0.copyload.i.i63.i = load ptr, ptr %joinpoint_arg_alloca2.i, align 8
  %.sroa.2.0..sroa_idx.i.i64.i = getelementptr inbounds i8, ptr %joinpoint_arg_alloca2.i, i64 8
  %.sroa.2.0.copyload.i.i65.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i64.i, align 8
  %76 = ptrtoint ptr %.sroa.0.0.copyload.i.i63.i to i64
  %77 = shl nuw i64 %load_str_to_stack.unpack4.i60.i, 1
  %isneg.i.i.i66.i = icmp slt i64 %.sroa.2.0.copyload.i.i65.i, 0
  %78 = select i1 %isneg.i.i.i66.i, i64 %77, i64 %76
  %79 = inttoptr i64 %78 to ptr
  %get_rc_ptr.i67.i = getelementptr inbounds i64, ptr %79, i64 -1
  %80 = load i64, ptr %get_rc_ptr.i67.i, align 8
  %.not.i.i68.i = icmp eq i64 %80, 0
  br i1 %.not.i.i68.i, label %"#Attr_#dec_1.exit69.i", label %81

81:                                               ; preds = %modify_rc.i62.i
  %82 = atomicrmw sub ptr %get_rc_ptr.i67.i, i64 1 monotonic, align 8
  %83 = icmp eq i64 %82, 1
  br i1 %83, label %84, label %"#Attr_#dec_1.exit69.i"

84:                                               ; preds = %81
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i67.i, i32 8)
  br label %"#Attr_#dec_1.exit69.i"

"#Attr_#dec_1.exit69.i":                          ; preds = %84, %81, %modify_rc.i62.i, %"#Attr_#dec_2.exit.i"
  %load_str_to_stack.elt3.i70.i = getelementptr inbounds %str.RocStr, ptr %joinpoint_arg_alloca3.i, i64 0, i32 2
  %load_str_to_stack.unpack4.i71.i = load i64, ptr %load_str_to_stack.elt3.i70.i, align 8
  %is_big_str.i72.i = icmp sgt i64 %load_str_to_stack.unpack4.i71.i, 0
  br i1 %is_big_str.i72.i, label %modify_rc.i73.i, label %"#Attr_#dec_1.exit80.i"

modify_rc.i73.i:                                  ; preds = %"#Attr_#dec_1.exit69.i"
  %.sroa.0.0.copyload.i.i74.i = load ptr, ptr %joinpoint_arg_alloca3.i, align 8
  %.sroa.2.0..sroa_idx.i.i75.i = getelementptr inbounds i8, ptr %joinpoint_arg_alloca3.i, i64 8
  %.sroa.2.0.copyload.i.i76.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i75.i, align 8
  %85 = ptrtoint ptr %.sroa.0.0.copyload.i.i74.i to i64
  %86 = shl nuw i64 %load_str_to_stack.unpack4.i71.i, 1
  %isneg.i.i.i77.i = icmp slt i64 %.sroa.2.0.copyload.i.i76.i, 0
  %87 = select i1 %isneg.i.i.i77.i, i64 %86, i64 %85
  %88 = inttoptr i64 %87 to ptr
  %get_rc_ptr.i78.i = getelementptr inbounds i64, ptr %88, i64 -1
  %89 = load i64, ptr %get_rc_ptr.i78.i, align 8
  %.not.i.i79.i = icmp eq i64 %89, 0
  br i1 %.not.i.i79.i, label %"#Attr_#dec_1.exit80.i", label %90

90:                                               ; preds = %modify_rc.i73.i
  %91 = atomicrmw sub ptr %get_rc_ptr.i78.i, i64 1 monotonic, align 8
  %92 = icmp eq i64 %91, 1
  br i1 %92, label %93, label %"#Attr_#dec_1.exit80.i"

93:                                               ; preds = %90
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i78.i, i32 8)
  br label %"#Attr_#dec_1.exit80.i"

"#Attr_#dec_1.exit80.i":                          ; preds = %93, %90, %modify_rc.i73.i, %"#Attr_#dec_1.exit69.i"
  call fastcc void @Str_concat_99aa979e4a9cadd6dbe48ea878ec84acb7696eb93470c375f6893f1da46c3772(ptr nonnull %result_value5, ptr nonnull %joinpoint_arg_alloca1.i, ptr nonnull %result_value15.i)
  br i1 %is_big_str.i29.i, label %modify_rc.i84.i, label %Str_replace_each_help_95dbc324453309f26dee9436b39568cc8bcbe17ef409e9273c4edb58653fd.exit

modify_rc.i84.i:                                  ; preds = %"#Attr_#dec_1.exit80.i"
  %.sroa.0.0.copyload.i.i85.i = load ptr, ptr %joinpoint_arg_alloca1.i, align 8
  %.sroa.2.0.copyload.i.i87.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i32.i, align 8
  %94 = ptrtoint ptr %.sroa.0.0.copyload.i.i85.i to i64
  %95 = shl nuw i64 %load_str_to_stack.unpack4.i28.i, 1
  %isneg.i.i.i88.i = icmp slt i64 %.sroa.2.0.copyload.i.i87.i, 0
  %96 = select i1 %isneg.i.i.i88.i, i64 %95, i64 %94
  %97 = inttoptr i64 %96 to ptr
  %get_rc_ptr.i89.i = getelementptr inbounds i64, ptr %97, i64 -1
  %98 = load i64, ptr %get_rc_ptr.i89.i, align 8
  %.not.i.i90.i = icmp eq i64 %98, 0
  br i1 %.not.i.i90.i, label %Str_replace_each_help_95dbc324453309f26dee9436b39568cc8bcbe17ef409e9273c4edb58653fd.exit, label %99

99:                                               ; preds = %modify_rc.i84.i
  %100 = atomicrmw sub ptr %get_rc_ptr.i89.i, i64 1 monotonic, align 8
  %101 = icmp eq i64 %100, 1
  br i1 %101, label %102, label %Str_replace_each_help_95dbc324453309f26dee9436b39568cc8bcbe17ef409e9273c4edb58653fd.exit

102:                                              ; preds = %99
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i89.i, i32 8)
  br label %Str_replace_each_help_95dbc324453309f26dee9436b39568cc8bcbe17ef409e9273c4edb58653fd.exit

Str_replace_each_help_95dbc324453309f26dee9436b39568cc8bcbe17ef409e9273c4edb58653fd.exit: ; preds = %"#Attr_#dec_1.exit80.i", %modify_rc.i84.i, %99, %102
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %result_value9.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %struct_field8.i)
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %load_element.i)
  call void @llvm.lifetime.end.p0(i64 56, ptr nonnull %result_value.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %joinpoint_arg_alloca3.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %joinpoint_arg_alloca2.i)
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %joinpoint_arg_alloca1.i)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %result_value15.i, i64 24, i1 false)
  br label %common.ret

"#Attr_#dec_2.exit":                              ; preds = %"#Attr_#inc_1.exit"
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %haystack, i64 24, i1 false)
  br label %common.ret
}

; Function Attrs: nounwind
define internal fastcc i64 @"#UserApp_check_99c9f773566b1d5d689233ef7949cf16c8797c970a4668678361d8c89d24f20"(ptr %tree) unnamed_addr #10 {
entry:
  %is_null = icmp eq ptr %tree, null
  br i1 %is_null, label %common.ret, label %else_block

common.ret:                                       ; preds = %entry, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit20
  %common.ret.op = phi i64 [ %operation_result.i18, %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit20 ], [ 0, %entry ]
  ret i64 %common.ret.op

else_block:                                       ; preds = %entry
  %at_index_struct_gep_data = getelementptr inbounds { i64, ptr, ptr }, ptr %tree, i64 0, i32 1
  %load_at_index_ptr_old = load ptr, ptr %at_index_struct_gep_data, align 8
  %at_index_struct_gep_data1 = getelementptr inbounds { i64, ptr, ptr }, ptr %tree, i64 0, i32 2
  %load_at_index_ptr_old2 = load ptr, ptr %at_index_struct_gep_data1, align 8
  %get_rc_ptr = getelementptr inbounds i64, ptr %tree, i64 -1
  %get_refcount = load i64, ptr %get_rc_ptr, align 8
  %is_one = icmp eq i64 %get_refcount, 1
  br i1 %is_one, label %joinpointcont.sink.split, label %else_block5

else_block5:                                      ; preds = %else_block
  %is_null.i = icmp eq ptr %load_at_index_ptr_old, null
  br i1 %is_null.i, label %"#Attr_#inc_18.exit", label %should_recurse.i

should_recurse.i:                                 ; preds = %else_block5
  %get_rc_ptr.i = getelementptr inbounds i64, ptr %load_at_index_ptr_old, i64 -1
  %0 = load i64, ptr %get_rc_ptr.i, align 8
  %.not.i.i = icmp eq i64 %0, 0
  br i1 %.not.i.i, label %"#Attr_#inc_18.exit", label %1

1:                                                ; preds = %should_recurse.i
  %2 = atomicrmw add ptr %get_rc_ptr.i, i64 1 monotonic, align 8
  br label %"#Attr_#inc_18.exit"

"#Attr_#inc_18.exit":                             ; preds = %else_block5, %should_recurse.i, %1
  %is_null.i11 = icmp eq ptr %load_at_index_ptr_old2, null
  br i1 %is_null.i11, label %"#Attr_#inc_18.exit15", label %should_recurse.i12

should_recurse.i12:                               ; preds = %"#Attr_#inc_18.exit"
  %get_rc_ptr.i13 = getelementptr inbounds i64, ptr %load_at_index_ptr_old2, i64 -1
  %3 = load i64, ptr %get_rc_ptr.i13, align 8
  %.not.i.i14 = icmp eq i64 %3, 0
  br i1 %.not.i.i14, label %"#Attr_#inc_18.exit15", label %4

4:                                                ; preds = %should_recurse.i12
  %5 = atomicrmw add ptr %get_rc_ptr.i13, i64 1 monotonic, align 8
  br label %"#Attr_#inc_18.exit15"

"#Attr_#inc_18.exit15":                           ; preds = %"#Attr_#inc_18.exit", %should_recurse.i12, %4
  %6 = load i64, ptr %get_rc_ptr, align 8
  %.not.i = icmp eq i64 %6, 0
  br i1 %.not.i, label %joinpointcont, label %7

7:                                                ; preds = %"#Attr_#inc_18.exit15"
  %8 = atomicrmw sub ptr %get_rc_ptr, i64 1 monotonic, align 8
  %9 = icmp eq i64 %8, 1
  br i1 %9, label %joinpointcont.sink.split, label %joinpointcont

joinpointcont.sink.split:                         ; preds = %7, %else_block
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr, i32 8)
  br label %joinpointcont

joinpointcont:                                    ; preds = %joinpointcont.sink.split, %7, %"#Attr_#inc_18.exit15"
  %call = tail call fastcc i64 @"#UserApp_check_99c9f773566b1d5d689233ef7949cf16c8797c970a4668678361d8c89d24f20"(ptr %load_at_index_ptr_old)
  %call.i = tail call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %call, i64 1)
  %has_overflowed.i = extractvalue { i64, i1 } %call.i, 1
  br i1 %has_overflowed.i, label %throw_block.i, label %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit

throw_block.i:                                    ; preds = %joinpointcont
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit: ; preds = %joinpointcont
  %operation_result.i = extractvalue { i64, i1 } %call.i, 0
  %call9 = tail call fastcc i64 @"#UserApp_check_99c9f773566b1d5d689233ef7949cf16c8797c970a4668678361d8c89d24f20"(ptr %load_at_index_ptr_old2)
  %call.i16 = tail call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %operation_result.i, i64 %call9)
  %has_overflowed.i17 = extractvalue { i64, i1 } %call.i16, 1
  br i1 %has_overflowed.i17, label %throw_block.i19, label %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit20

throw_block.i19:                                  ; preds = %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit20: ; preds = %Num_add_669c1355a3e727bb53dd458f2e96e48571aa45dfabcfb4b7de1689484f11.exit
  %operation_result.i18 = extractvalue { i64, i1 } %call.i16, 0
  br label %common.ret
}

; Function Attrs: nounwind
define internal fastcc void @Num_to_str_68697e959be5e5da06cc73b6f998e193cbf2d9b22efd0355a3d37129951b(i64 %"#arg1", ptr nocapture writeonly %0) unnamed_addr #10 {
entry:
  %1 = alloca [24 x i8], align 8
  %2 = alloca [65 x i8], align 1
  %3 = alloca [16 x i8], align 8
  %4 = alloca [8 x i8], align 8
  %5 = alloca [24 x i8], align 8
  %6 = alloca [20 x i8], align 1
  call void @llvm.lifetime.start.p0(i64 20, ptr nonnull %6)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3), !noalias !196
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4), !noalias !196
  call void @llvm.lifetime.start.p0(i64 24, ptr nonnull %5), !noalias !196
  store ptr %6, ptr %5, align 8, !noalias !199
  %.sroa.2.0..sroa_idx.i.i = getelementptr inbounds i8, ptr %5, i64 8
  %.sroa.3.0..sroa_idx.i.i = getelementptr inbounds i8, ptr %5, i64 16
  store <2 x i64> <i64 20, i64 0>, ptr %.sroa.2.0..sroa_idx.i.i, align 8, !noalias !199
  %7 = ptrtoint ptr %5 to i64
  store i64 %7, ptr %4, align 8, !noalias !199
  store ptr %4, ptr %3, align 8, !noalias !199
  %8 = getelementptr inbounds %io.Writer, ptr %3, i64 0, i32 1
  store ptr @"io.GenericWriter(*io.fixed_buffer_stream.FixedBufferStream([]u8),error{NoSpaceLeft},(function 'write')).typeErasedWriteFn", ptr %8, align 8, !noalias !199
  call void @llvm.lifetime.start.p0(i64 65, ptr nonnull %2), !noalias !199
  %9 = icmp ugt i64 %"#arg1", 99
  br i1 %9, label %.lr.ph.i.i.i.i.i.i.i, label %._crit_edge.i.i.i.i.i.i.i

.lr.ph.i.i.i.i.i.i.i:                             ; preds = %entry, %.lr.ph.i.i.i.i.i.i.i
  %.sroa.0.03.i.i.i.i.i.i.i = phi i64 [ %12, %.lr.ph.i.i.i.i.i.i.i ], [ %"#arg1", %entry ]
  %.sroa.09.12.i.i.i.i.i.i.i = phi i64 [ %10, %.lr.ph.i.i.i.i.i.i.i ], [ 65, %entry ]
  %10 = add i64 %.sroa.09.12.i.i.i.i.i.i.i, -2
  %11 = getelementptr inbounds i8, ptr %2, i64 %10
  %.sroa.0.03.i.i.i.i.i.i.i.frozen = freeze i64 %.sroa.0.03.i.i.i.i.i.i.i
  %12 = udiv i64 %.sroa.0.03.i.i.i.i.i.i.i.frozen, 100
  %13 = mul i64 %12, 100
  %.decomposed = sub i64 %.sroa.0.03.i.i.i.i.i.i.i.frozen, %13
  %14 = shl nuw nsw i64 %.decomposed, 1
  %15 = getelementptr inbounds i8, ptr @__anon_21180, i64 %14
  %16 = load i16, ptr %15, align 1, !noalias !202
  store i16 %16, ptr %11, align 1, !noalias !199
  %17 = icmp ugt i64 %.sroa.0.03.i.i.i.i.i.i.i, 9999
  br i1 %17, label %.lr.ph.i.i.i.i.i.i.i, label %._crit_edge.i.i.i.i.i.i.i

._crit_edge.i.i.i.i.i.i.i:                        ; preds = %.lr.ph.i.i.i.i.i.i.i, %entry
  %.sroa.09.1.lcssa.i.i.i.i.i.i.i = phi i64 [ 65, %entry ], [ %10, %.lr.ph.i.i.i.i.i.i.i ]
  %.sroa.0.0.lcssa.i.i.i.i.i.i.i = phi i64 [ %"#arg1", %entry ], [ %12, %.lr.ph.i.i.i.i.i.i.i ]
  %18 = icmp ult i64 %.sroa.0.0.lcssa.i.i.i.i.i.i.i, 10
  br i1 %18, label %19, label %24

19:                                               ; preds = %._crit_edge.i.i.i.i.i.i.i
  %20 = add i64 %.sroa.09.1.lcssa.i.i.i.i.i.i.i, -1
  %21 = getelementptr inbounds [65 x i8], ptr %2, i64 0, i64 %20
  %22 = trunc i64 %.sroa.0.0.lcssa.i.i.i.i.i.i.i to i8
  %23 = or disjoint i8 %22, 48
  store i8 %23, ptr %21, align 1, !noalias !199
  br label %fmt.format__anon_8088.exit.i.i

24:                                               ; preds = %._crit_edge.i.i.i.i.i.i.i
  %25 = add i64 %.sroa.09.1.lcssa.i.i.i.i.i.i.i, -2
  %26 = getelementptr inbounds i8, ptr %2, i64 %25
  %27 = shl nuw nsw i64 %.sroa.0.0.lcssa.i.i.i.i.i.i.i, 1
  %28 = getelementptr inbounds i8, ptr @__anon_21180, i64 %27
  %29 = load i16, ptr %28, align 1, !noalias !205
  store i16 %29, ptr %26, align 1, !noalias !199
  br label %fmt.format__anon_8088.exit.i.i

fmt.format__anon_8088.exit.i.i:                   ; preds = %24, %19
  %.sroa.09.0.i.i.i.i.i.i.i = phi i64 [ %20, %19 ], [ %25, %24 ]
  %30 = getelementptr inbounds i8, ptr %2, i64 %.sroa.09.0.i.i.i.i.i.i.i
  %31 = sub nuw i64 65, %.sroa.09.0.i.i.i.i.i.i.i
  %32 = call fastcc i16 @fmt.formatBuf__anon_8817(ptr nonnull readonly align 1 %30, i64 %31, ptr nonnull readonly align 8 @6, ptr nonnull readonly align 8 %3), !noalias !199
  call void @llvm.lifetime.end.p0(i64 65, ptr nonnull %2), !noalias !199
  %33 = icmp eq i16 %32, 0
  call void @llvm.assume(i1 %33)
  %.val4.i.i = load i64, ptr %.sroa.3.0..sroa_idx.i.i, align 8, !noalias !199
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3), !noalias !196
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4), !noalias !196
  call void @llvm.lifetime.end.p0(i64 24, ptr nonnull %5), !noalias !196
  %34 = icmp ugt i64 %.val4.i.i, 23
  br i1 %34, label %str.RocStr.allocate.exit.i.i, label %str.RocStr.allocate.exit.thread.i.i

str.RocStr.allocate.exit.thread.i.i:              ; preds = %fmt.format__anon_8088.exit.i.i
  %.sroa.3.23.insert.ext.i.i = shl nuw nsw i64 %.val4.i.i, 56
  %.sroa.3.23.insert.shift.i.i = or disjoint i64 %.sroa.3.23.insert.ext.i.i, -9223372036854775808
  br label %roc_builtins.str.from_int.u64.exit

str.RocStr.allocate.exit.i.i:                     ; preds = %fmt.format__anon_8088.exit.i.i
  %35 = add nuw i64 %.val4.i.i, 8
  %36 = call ptr @roc_alloc(i64 %35, i32 8), !noalias !208
  %37 = icmp ne ptr %36, null
  call void @llvm.assume(i1 %37)
  %38 = getelementptr inbounds i8, ptr %36, i64 8
  store i64 1, ptr %36, align 8, !noalias !208
  %39 = icmp slt i64 %.val4.i.i, 0
  %spec.select.i.i = select i1 %39, ptr %1, ptr %38
  br label %roc_builtins.str.from_int.u64.exit

roc_builtins.str.from_int.u64.exit:               ; preds = %str.RocStr.allocate.exit.thread.i.i, %str.RocStr.allocate.exit.i.i
  %.sink7.i.i = phi ptr [ %38, %str.RocStr.allocate.exit.i.i ], [ null, %str.RocStr.allocate.exit.thread.i.i ]
  %.sink.i.i = phi i64 [ %.val4.i.i, %str.RocStr.allocate.exit.i.i ], [ 0, %str.RocStr.allocate.exit.thread.i.i ]
  %.sroa.3.23.insert.shift.sink.i.i = phi i64 [ %.val4.i.i, %str.RocStr.allocate.exit.i.i ], [ %.sroa.3.23.insert.shift.i.i, %str.RocStr.allocate.exit.thread.i.i ]
  %common.ret.op.i.i.i = phi ptr [ %spec.select.i.i, %str.RocStr.allocate.exit.i.i ], [ %1, %str.RocStr.allocate.exit.thread.i.i ]
  store ptr %.sink7.i.i, ptr %1, align 8
  %40 = getelementptr inbounds i8, ptr %1, i64 8
  store i64 %.sink.i.i, ptr %40, align 8
  %41 = getelementptr inbounds i8, ptr %1, i64 16
  store i64 %.sroa.3.23.insert.shift.sink.i.i, ptr %41, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %common.ret.op.i.i.i, ptr nonnull align 1 %6, i64 %.val4.i.i, i1 false)
  call void @llvm.lifetime.end.p0(i64 20, ptr nonnull %6)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %1, i64 24, i1 false)
  ret void
}

; Function Attrs: nounwind
define internal fastcc ptr @"#UserApp_build_1e4d2f1e6b4984301a1489b71481ade3a818d1fae80b8f87ea525c7bff923"(i64 %arg_0, i64 %arg_1) unnamed_addr #10 {
entry:
  %stack_value = alloca ptr, align 8
  store ptr null, ptr %stack_value, align 8
  %eq_u64.i44 = icmp eq i64 %arg_0, 0
  br i1 %eq_u64.i44, label %then_block, label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit

then_block:                                       ; preds = %Num_add_e84248fb50d0833361d0417df114b0b3b3448fff97c39cdde963b09a9aebb8.exit, %entry
  %joinpointarg1.lcssa = phi i64 [ %arg_1, %entry ], [ %operation_result.i26, %Num_add_e84248fb50d0833361d0417df114b0b3b3448fff97c39cdde963b09a9aebb8.exit ]
  %joinpointarg2.lcssa = phi ptr [ %stack_value, %entry ], [ %call_builtin12.repack17, %Num_add_e84248fb50d0833361d0417df114b0b3b3448fff97c39cdde963b09a9aebb8.exit ]
  %0 = tail call ptr @roc_alloc(i64 32, i32 8)
  %1 = icmp ne ptr %0, null
  tail call void @llvm.assume(i1 %1)
  %2 = getelementptr inbounds i8, ptr %0, i64 8
  store i64 1, ptr %0, align 8
  store i64 %joinpointarg1.lcssa, ptr %2, align 8
  %call_builtin.repack21 = getelementptr inbounds i8, ptr %0, i64 16
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %call_builtin.repack21, i8 0, i64 16, i1 false)
  store ptr %2, ptr %joinpointarg2.lcssa, align 8
  %stack_value.0.stack_value.0.stack_value.0.stack_value.0.ptr_load = load ptr, ptr %stack_value, align 8
  ret ptr %stack_value.0.stack_value.0.stack_value.0.stack_value.0.ptr_load

Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit: ; preds = %entry, %Num_add_e84248fb50d0833361d0417df114b0b3b3448fff97c39cdde963b09a9aebb8.exit
  %joinpointarg247 = phi ptr [ %call_builtin12.repack17, %Num_add_e84248fb50d0833361d0417df114b0b3b3448fff97c39cdde963b09a9aebb8.exit ], [ %stack_value, %entry ]
  %joinpointarg146 = phi i64 [ %operation_result.i26, %Num_add_e84248fb50d0833361d0417df114b0b3b3448fff97c39cdde963b09a9aebb8.exit ], [ %arg_1, %entry ]
  %joinpointarg45 = phi i64 [ %call.i41, %Num_add_e84248fb50d0833361d0417df114b0b3b3448fff97c39cdde963b09a9aebb8.exit ], [ %arg_0, %entry ]
  %3 = add i64 %joinpointarg146, 4611686018427387904
  %has_overflowed.i25 = icmp slt i64 %3, 0
  br i1 %has_overflowed.i25, label %throw_block.i27, label %Num_add_e84248fb50d0833361d0417df114b0b3b3448fff97c39cdde963b09a9aebb8.exit

throw_block.i27:                                  ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit
  tail call fastcc void @throw_on_overflow()
  unreachable

Num_add_e84248fb50d0833361d0417df114b0b3b3448fff97c39cdde963b09a9aebb8.exit: ; preds = %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit
  %call.i41 = add i64 %joinpointarg45, -1
  %operation_result.i26 = shl nsw i64 %joinpointarg146, 1
  %call.i37 = or disjoint i64 %operation_result.i26, 1
  %call11 = tail call fastcc ptr @"#UserApp_build_1e4d2f1e6b4984301a1489b71481ade3a818d1fae80b8f87ea525c7bff923"(i64 %call.i41, i64 %call.i37)
  %4 = tail call ptr @roc_alloc(i64 32, i32 8)
  %5 = icmp ne ptr %4, null
  tail call void @llvm.assume(i1 %5)
  %6 = getelementptr inbounds i8, ptr %4, i64 8
  store i64 1, ptr %4, align 8
  store i64 %joinpointarg146, ptr %6, align 8
  %call_builtin12.repack17 = getelementptr inbounds i8, ptr %4, i64 16
  store ptr null, ptr %call_builtin12.repack17, align 8
  %call_builtin12.repack19 = getelementptr inbounds i8, ptr %4, i64 24
  store ptr %call11, ptr %call_builtin12.repack19, align 8
  store ptr %6, ptr %joinpointarg247, align 8
  %eq_u64.i = icmp eq i64 %call.i41, 0
  br i1 %eq_u64.i, label %then_block, label %Num_sub_edaf1bd3d1c2ffcc44df55829c02f262426de2ffbea9be2cdf075ec12c528d.exit
}

; Function Attrs: cold noinline noreturn nounwind
define internal fastcc void @throw_on_overflow() unnamed_addr #12 {
entry:
  %const_str_store = alloca %str.RocStr, align 8
  store ptr getelementptr inbounds ([42 x i8], ptr @_str_literal_5625337945828202030, i64 0, i64 8), ptr %const_str_store, align 8
  %.fca.1.gep = getelementptr inbounds %str.RocStr, ptr %const_str_store, i64 0, i32 1
  store <2 x i64> <i64 34, i64 34>, ptr %.fca.1.gep, align 8
  call void @roc_panic(ptr %const_str_store, i32 0)
  unreachable
}

; Function Attrs: nounwind
define private fastcc void @"#Attr_#dec_4"(%list.RocList %"#arg1") unnamed_addr #10 {
entry:
  %"#arg1.fca.0.extract" = extractvalue %list.RocList %"#arg1", 0
  %"#arg1.fca.1.extract" = extractvalue %list.RocList %"#arg1", 1
  %"#arg1.fca.2.extract" = extractvalue %list.RocList %"#arg1", 2
  %.not77 = icmp slt i64 %"#arg1.fca.2.extract", 0
  %0 = select i1 %.not77, i64 %"#arg1.fca.1.extract", i64 %"#arg1.fca.2.extract"
  %1 = icmp ne i64 %0, 0
  %or.cond = or i1 %.not77, %1
  %2 = ptrtoint ptr %"#arg1.fca.0.extract" to i64
  %3 = shl i64 %"#arg1.fca.2.extract", 1
  br i1 %or.cond, label %list.RocList.refcount.exit, label %list.RocList.refcount.exit.thread

list.RocList.refcount.exit:                       ; preds = %entry
  %4 = select i1 %.not77, i64 %3, i64 %2
  %5 = inttoptr i64 %4 to ptr
  %6 = getelementptr inbounds i64, ptr %5, i64 -1
  %7 = load i64, ptr %6, align 8
  %8 = icmp eq i64 %7, 1
  br i1 %8, label %list.RocList.refcount.exit.thread, label %roc_builtins.list.decref.exit

list.RocList.refcount.exit.thread:                ; preds = %entry, %list.RocList.refcount.exit
  %.pre-phi89 = phi ptr [ %5, %list.RocList.refcount.exit ], [ %"#arg1.fca.0.extract", %entry ]
  %.pre-phi87 = phi i64 [ %4, %list.RocList.refcount.exit ], [ %2, %entry ]
  %.not.i.i = icmp eq i64 %.pre-phi87, 0
  br i1 %.not.i.i, label %roc_builtins.list.decref.exit, label %9

9:                                                ; preds = %list.RocList.refcount.exit.thread
  br i1 %.not77, label %10, label %list.RocList.getAllocationElementCount.exit

10:                                               ; preds = %9
  %11 = getelementptr inbounds i64, ptr %.pre-phi89, i64 -2
  %common.ret.op.in.i.sroa.speculate.load.28 = load i64, ptr %11, align 8
  br label %list.RocList.getAllocationElementCount.exit

list.RocList.getAllocationElementCount.exit:      ; preds = %9, %10
  %common.ret.op.in.i.sroa.speculated = phi i64 [ %common.ret.op.in.i.sroa.speculate.load.28, %10 ], [ %"#arg1.fca.1.extract", %9 ]
  %.not = icmp eq i64 %common.ret.op.in.i.sroa.speculated, 0
  br i1 %.not, label %roc_builtins.list.decref.exit, label %.lr.ph

.lr.ph:                                           ; preds = %list.RocList.getAllocationElementCount.exit, %"#Attr_#dec_5.exit"
  %.sroa.0.0.i.i81 = phi i64 [ %38, %"#Attr_#dec_5.exit" ], [ 0, %list.RocList.getAllocationElementCount.exit ]
  %12 = shl nuw i64 %.sroa.0.0.i.i81, 5
  %13 = getelementptr inbounds i8, ptr %.pre-phi89, i64 %12
  %tag_id_ptr.i = getelementptr inbounds { [3 x i64], i8 }, ptr %13, i64 0, i32 1
  %load_tag_id.i = load i8, ptr %tag_id_ptr.i, align 1
  switch i8 %load_tag_id.i, label %"#Attr_#dec_5.exit" [
    i8 0, label %tag_id_modify.i
    i8 1, label %tag_id_modify1.i
  ]

modify_rc_union_merge.sink.split.i:               ; preds = %35, %23
  %.sink.i = phi ptr [ %21, %23 ], [ %33, %35 ]
  tail call void @roc_dealloc(ptr nonnull align 1 %.sink.i, i32 8)
  br label %"#Attr_#dec_5.exit"

tag_id_modify.i:                                  ; preds = %.lr.ph
  %field_value.unpack.i = load ptr, ptr %13, align 8
  %field_value.elt12.i = getelementptr inbounds %list.RocList, ptr %13, i64 0, i32 2
  %field_value.unpack13.i = load i64, ptr %field_value.elt12.i, align 8
  %14 = ptrtoint ptr %field_value.unpack.i to i64
  %15 = shl i64 %field_value.unpack13.i, 1
  %isneg.i.i = icmp slt i64 %field_value.unpack13.i, 0
  %16 = select i1 %isneg.i.i, i64 %15, i64 %14
  %17 = icmp ne i64 %field_value.unpack13.i, 0
  %18 = icmp ne i64 %16, 0
  %or.cond.i.i.i = select i1 %17, i1 %18, i1 false
  br i1 %or.cond.i.i.i, label %19, label %"#Attr_#dec_5.exit"

19:                                               ; preds = %tag_id_modify.i
  %20 = inttoptr i64 %16 to ptr
  %21 = getelementptr inbounds i64, ptr %20, i64 -1
  %22 = load i64, ptr %21, align 8
  %.not.i.i.i = icmp eq i64 %22, 0
  br i1 %.not.i.i.i, label %"#Attr_#dec_5.exit", label %23

23:                                               ; preds = %19
  %24 = atomicrmw sub ptr %21, i64 1 monotonic, align 8
  %25 = icmp eq i64 %24, 1
  br i1 %25, label %modify_rc_union_merge.sink.split.i, label %"#Attr_#dec_5.exit"

tag_id_modify1.i:                                 ; preds = %.lr.ph
  %field_value4.unpack.i = load ptr, ptr %13, align 8
  %field_value4.elt7.i = getelementptr inbounds %list.RocList, ptr %13, i64 0, i32 2
  %field_value4.unpack8.i = load i64, ptr %field_value4.elt7.i, align 8
  %26 = ptrtoint ptr %field_value4.unpack.i to i64
  %27 = shl i64 %field_value4.unpack8.i, 1
  %isneg.i15.i = icmp slt i64 %field_value4.unpack8.i, 0
  %28 = select i1 %isneg.i15.i, i64 %27, i64 %26
  %29 = icmp ne i64 %field_value4.unpack8.i, 0
  %30 = icmp ne i64 %28, 0
  %or.cond.i.i16.i = select i1 %29, i1 %30, i1 false
  br i1 %or.cond.i.i16.i, label %31, label %"#Attr_#dec_5.exit"

31:                                               ; preds = %tag_id_modify1.i
  %32 = inttoptr i64 %28 to ptr
  %33 = getelementptr inbounds i64, ptr %32, i64 -1
  %34 = load i64, ptr %33, align 8
  %.not.i.i17.i = icmp eq i64 %34, 0
  br i1 %.not.i.i17.i, label %"#Attr_#dec_5.exit", label %35

35:                                               ; preds = %31
  %36 = atomicrmw sub ptr %33, i64 1 monotonic, align 8
  %37 = icmp eq i64 %36, 1
  br i1 %37, label %modify_rc_union_merge.sink.split.i, label %"#Attr_#dec_5.exit"

"#Attr_#dec_5.exit":                              ; preds = %.lr.ph, %modify_rc_union_merge.sink.split.i, %tag_id_modify.i, %19, %23, %tag_id_modify1.i, %31, %35
  %38 = add nuw i64 %.sroa.0.0.i.i81, 1
  %exitcond.not = icmp eq i64 %38, %common.ret.op.in.i.sroa.speculated
  br i1 %exitcond.not, label %roc_builtins.list.decref.exit, label %.lr.ph

roc_builtins.list.decref.exit:                    ; preds = %"#Attr_#dec_5.exit", %list.RocList.getAllocationElementCount.exit, %list.RocList.refcount.exit, %list.RocList.refcount.exit.thread
  %39 = select i1 %.not77, i64 %3, i64 %2
  %40 = icmp ne i64 %"#arg1.fca.2.extract", 0
  %41 = icmp ne i64 %39, 0
  %or.cond.i = select i1 %40, i1 %41, i1 false
  br i1 %or.cond.i, label %42, label %utils.decref.exit

42:                                               ; preds = %roc_builtins.list.decref.exit
  %43 = inttoptr i64 %39 to ptr
  %44 = getelementptr inbounds i64, ptr %43, i64 -1
  %45 = load i64, ptr %44, align 8
  %.not.i = icmp eq i64 %45, 0
  br i1 %.not.i, label %utils.decref.exit, label %46

46:                                               ; preds = %42
  %47 = atomicrmw sub ptr %44, i64 1 monotonic, align 8
  %48 = icmp eq i64 %47, 1
  br i1 %48, label %49, label %utils.decref.exit

49:                                               ; preds = %46
  %50 = getelementptr inbounds i64, ptr %43, i64 -2
  tail call void @roc_dealloc(ptr nonnull align 1 %50, i32 8)
  br label %utils.decref.exit

utils.decref.exit:                                ; preds = %roc_builtins.list.decref.exit, %42, %46, %49
  ret void
}

; Function Attrs: nounwind
define private fastcc void @"#Attr_#dec_5"(ptr nocapture readonly %"#arg1") unnamed_addr #10 {
entry:
  %tag_id_ptr = getelementptr inbounds { [3 x i64], i8 }, ptr %"#arg1", i64 0, i32 1
  %load_tag_id = load i8, ptr %tag_id_ptr, align 1
  switch i8 %load_tag_id, label %modify_rc_union_merge [
    i8 0, label %tag_id_modify
    i8 1, label %tag_id_modify1
  ]

modify_rc_union_merge.sink.split:                 ; preds = %21, %9
  %.sink = phi ptr [ %7, %9 ], [ %19, %21 ]
  tail call void @roc_dealloc(ptr nonnull align 1 %.sink, i32 8)
  br label %modify_rc_union_merge

modify_rc_union_merge:                            ; preds = %modify_rc_union_merge.sink.split, %21, %17, %tag_id_modify1, %9, %5, %tag_id_modify, %entry
  ret void

tag_id_modify:                                    ; preds = %entry
  %field_value.unpack = load ptr, ptr %"#arg1", align 8
  %field_value.elt12 = getelementptr inbounds %list.RocList, ptr %"#arg1", i64 0, i32 2
  %field_value.unpack13 = load i64, ptr %field_value.elt12, align 8
  %0 = ptrtoint ptr %field_value.unpack to i64
  %1 = shl i64 %field_value.unpack13, 1
  %isneg.i = icmp slt i64 %field_value.unpack13, 0
  %2 = select i1 %isneg.i, i64 %1, i64 %0
  %3 = icmp ne i64 %field_value.unpack13, 0
  %4 = icmp ne i64 %2, 0
  %or.cond.i.i = select i1 %3, i1 %4, i1 false
  br i1 %or.cond.i.i, label %5, label %modify_rc_union_merge

5:                                                ; preds = %tag_id_modify
  %6 = inttoptr i64 %2 to ptr
  %7 = getelementptr inbounds i64, ptr %6, i64 -1
  %8 = load i64, ptr %7, align 8
  %.not.i.i = icmp eq i64 %8, 0
  br i1 %.not.i.i, label %modify_rc_union_merge, label %9

9:                                                ; preds = %5
  %10 = atomicrmw sub ptr %7, i64 1 monotonic, align 8
  %11 = icmp eq i64 %10, 1
  br i1 %11, label %modify_rc_union_merge.sink.split, label %modify_rc_union_merge

tag_id_modify1:                                   ; preds = %entry
  %field_value4.unpack = load ptr, ptr %"#arg1", align 8
  %field_value4.elt7 = getelementptr inbounds %list.RocList, ptr %"#arg1", i64 0, i32 2
  %field_value4.unpack8 = load i64, ptr %field_value4.elt7, align 8
  %12 = ptrtoint ptr %field_value4.unpack to i64
  %13 = shl i64 %field_value4.unpack8, 1
  %isneg.i15 = icmp slt i64 %field_value4.unpack8, 0
  %14 = select i1 %isneg.i15, i64 %13, i64 %12
  %15 = icmp ne i64 %field_value4.unpack8, 0
  %16 = icmp ne i64 %14, 0
  %or.cond.i.i16 = select i1 %15, i1 %16, i1 false
  br i1 %or.cond.i.i16, label %17, label %modify_rc_union_merge

17:                                               ; preds = %tag_id_modify1
  %18 = inttoptr i64 %14 to ptr
  %19 = getelementptr inbounds i64, ptr %18, i64 -1
  %20 = load i64, ptr %19, align 8
  %.not.i.i17 = icmp eq i64 %20, 0
  br i1 %.not.i.i17, label %modify_rc_union_merge, label %21

21:                                               ; preds = %17
  %22 = atomicrmw sub ptr %19, i64 1 monotonic, align 8
  %23 = icmp eq i64 %22, 1
  br i1 %23, label %modify_rc_union_merge.sink.split, label %modify_rc_union_merge
}

; Function Attrs: nounwind
define private fastcc void @"#Attr_#dec_12"(%list.RocList %"#arg1") unnamed_addr #10 {
entry:
  %"#arg1.fca.0.extract" = extractvalue %list.RocList %"#arg1", 0
  %"#arg1.fca.1.extract" = extractvalue %list.RocList %"#arg1", 1
  %"#arg1.fca.2.extract" = extractvalue %list.RocList %"#arg1", 2
  %.not77 = icmp slt i64 %"#arg1.fca.2.extract", 0
  %0 = select i1 %.not77, i64 %"#arg1.fca.1.extract", i64 %"#arg1.fca.2.extract"
  %1 = icmp ne i64 %0, 0
  %or.cond = or i1 %.not77, %1
  %2 = ptrtoint ptr %"#arg1.fca.0.extract" to i64
  %3 = shl i64 %"#arg1.fca.2.extract", 1
  br i1 %or.cond, label %list.RocList.refcount.exit, label %list.RocList.refcount.exit.thread

list.RocList.refcount.exit:                       ; preds = %entry
  %4 = select i1 %.not77, i64 %3, i64 %2
  %5 = inttoptr i64 %4 to ptr
  %6 = getelementptr inbounds i64, ptr %5, i64 -1
  %7 = load i64, ptr %6, align 8
  %8 = icmp eq i64 %7, 1
  br i1 %8, label %list.RocList.refcount.exit.thread, label %roc_builtins.list.decref.exit

list.RocList.refcount.exit.thread:                ; preds = %entry, %list.RocList.refcount.exit
  %.pre-phi89 = phi ptr [ %5, %list.RocList.refcount.exit ], [ %"#arg1.fca.0.extract", %entry ]
  %.pre-phi87 = phi i64 [ %4, %list.RocList.refcount.exit ], [ %2, %entry ]
  %.not.i.i = icmp eq i64 %.pre-phi87, 0
  br i1 %.not.i.i, label %roc_builtins.list.decref.exit, label %9

9:                                                ; preds = %list.RocList.refcount.exit.thread
  br i1 %.not77, label %10, label %list.RocList.getAllocationElementCount.exit

10:                                               ; preds = %9
  %11 = getelementptr inbounds i64, ptr %.pre-phi89, i64 -2
  %common.ret.op.in.i.sroa.speculate.load.28 = load i64, ptr %11, align 8
  br label %list.RocList.getAllocationElementCount.exit

list.RocList.getAllocationElementCount.exit:      ; preds = %9, %10
  %common.ret.op.in.i.sroa.speculated = phi i64 [ %common.ret.op.in.i.sroa.speculate.load.28, %10 ], [ %"#arg1.fca.1.extract", %9 ]
  %.not = icmp eq i64 %common.ret.op.in.i.sroa.speculated, 0
  br i1 %.not, label %roc_builtins.list.decref.exit, label %.lr.ph

.lr.ph:                                           ; preds = %list.RocList.getAllocationElementCount.exit, %"#Attr_#dec_13.exit"
  %.sroa.0.0.i.i81 = phi i64 [ %30, %"#Attr_#dec_13.exit" ], [ 0, %list.RocList.getAllocationElementCount.exit ]
  %12 = mul nuw i64 %.sroa.0.0.i.i81, 40
  %13 = getelementptr inbounds i8, ptr %.pre-phi89, i64 %12
  %tag_id_ptr.i = getelementptr inbounds { [4 x i64], i8 }, ptr %13, i64 0, i32 1
  %load_tag_id.i = load i8, ptr %tag_id_ptr.i, align 1
  switch i8 %load_tag_id.i, label %"#Attr_#dec_13.exit" [
    i8 0, label %tag_id_modify.i
    i8 1, label %tag_id_modify1.i
  ]

modify_rc_union_merge.sink.split.i:               ; preds = %27, %19
  %get_rc_ptr.i.sink.i = phi ptr [ %get_rc_ptr.i.i.i, %19 ], [ %get_rc_ptr.i.i, %27 ]
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i.sink.i, i32 8)
  br label %"#Attr_#dec_13.exit"

tag_id_modify.i:                                  ; preds = %.lr.ph
  %tag_id_ptr.i.i = getelementptr inbounds { [3 x i64], i8 }, ptr %13, i64 0, i32 1
  %load_tag_id.i.i = load i8, ptr %tag_id_ptr.i.i, align 1
  %cond.i.i = icmp eq i8 %load_tag_id.i.i, 4
  br i1 %cond.i.i, label %tag_id_modify.i.i, label %"#Attr_#dec_13.exit"

tag_id_modify.i.i:                                ; preds = %tag_id_modify.i
  %load_str_to_stack.elt3.i.i.i = getelementptr inbounds %str.RocStr, ptr %13, i64 0, i32 2
  %load_str_to_stack.unpack4.i.i.i = load i64, ptr %load_str_to_stack.elt3.i.i.i, align 8
  %is_big_str.i.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i.i.i, 0
  br i1 %is_big_str.i.i.i, label %modify_rc.i.i.i, label %"#Attr_#dec_13.exit"

modify_rc.i.i.i:                                  ; preds = %tag_id_modify.i.i
  %.sroa.0.0.copyload.i.i.i.i = load ptr, ptr %13, align 8
  %.sroa.2.0..sroa_idx.i.i.i.i = getelementptr inbounds i8, ptr %13, i64 8
  %.sroa.2.0.copyload.i.i.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i.i.i, align 8
  %14 = ptrtoint ptr %.sroa.0.0.copyload.i.i.i.i to i64
  %15 = shl nuw i64 %load_str_to_stack.unpack4.i.i.i, 1
  %isneg.i.i.i.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i.i.i, 0
  %16 = select i1 %isneg.i.i.i.i.i, i64 %15, i64 %14
  %17 = inttoptr i64 %16 to ptr
  %get_rc_ptr.i.i.i = getelementptr inbounds i64, ptr %17, i64 -1
  %18 = load i64, ptr %get_rc_ptr.i.i.i, align 8
  %.not.i.i.i.i = icmp eq i64 %18, 0
  br i1 %.not.i.i.i.i, label %"#Attr_#dec_13.exit", label %19

19:                                               ; preds = %modify_rc.i.i.i
  %20 = atomicrmw sub ptr %get_rc_ptr.i.i.i, i64 1 monotonic, align 8
  %21 = icmp eq i64 %20, 1
  br i1 %21, label %modify_rc_union_merge.sink.split.i, label %"#Attr_#dec_13.exit"

tag_id_modify1.i:                                 ; preds = %.lr.ph
  %load_str_to_stack.elt3.i.i = getelementptr inbounds %str.RocStr, ptr %13, i64 0, i32 2
  %load_str_to_stack.unpack4.i.i = load i64, ptr %load_str_to_stack.elt3.i.i, align 8
  %is_big_str.i.i = icmp sgt i64 %load_str_to_stack.unpack4.i.i, 0
  br i1 %is_big_str.i.i, label %modify_rc.i.i, label %"#Attr_#dec_13.exit"

modify_rc.i.i:                                    ; preds = %tag_id_modify1.i
  %.sroa.0.0.copyload.i.i.i = load ptr, ptr %13, align 8
  %.sroa.2.0..sroa_idx.i.i.i = getelementptr inbounds i8, ptr %13, i64 8
  %.sroa.2.0.copyload.i.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i.i, align 8
  %22 = ptrtoint ptr %.sroa.0.0.copyload.i.i.i to i64
  %23 = shl nuw i64 %load_str_to_stack.unpack4.i.i, 1
  %isneg.i.i.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i.i, 0
  %24 = select i1 %isneg.i.i.i.i, i64 %23, i64 %22
  %25 = inttoptr i64 %24 to ptr
  %get_rc_ptr.i.i = getelementptr inbounds i64, ptr %25, i64 -1
  %26 = load i64, ptr %get_rc_ptr.i.i, align 8
  %.not.i.i.i = icmp eq i64 %26, 0
  br i1 %.not.i.i.i, label %"#Attr_#dec_13.exit", label %27

27:                                               ; preds = %modify_rc.i.i
  %28 = atomicrmw sub ptr %get_rc_ptr.i.i, i64 1 monotonic, align 8
  %29 = icmp eq i64 %28, 1
  br i1 %29, label %modify_rc_union_merge.sink.split.i, label %"#Attr_#dec_13.exit"

"#Attr_#dec_13.exit":                             ; preds = %.lr.ph, %modify_rc_union_merge.sink.split.i, %tag_id_modify.i, %tag_id_modify.i.i, %modify_rc.i.i.i, %19, %tag_id_modify1.i, %modify_rc.i.i, %27
  %30 = add nuw i64 %.sroa.0.0.i.i81, 1
  %exitcond.not = icmp eq i64 %30, %common.ret.op.in.i.sroa.speculated
  br i1 %exitcond.not, label %roc_builtins.list.decref.exit, label %.lr.ph

roc_builtins.list.decref.exit:                    ; preds = %"#Attr_#dec_13.exit", %list.RocList.getAllocationElementCount.exit, %list.RocList.refcount.exit, %list.RocList.refcount.exit.thread
  %31 = select i1 %.not77, i64 %3, i64 %2
  %32 = icmp ne i64 %"#arg1.fca.2.extract", 0
  %33 = icmp ne i64 %31, 0
  %or.cond.i = select i1 %32, i1 %33, i1 false
  br i1 %or.cond.i, label %34, label %utils.decref.exit

34:                                               ; preds = %roc_builtins.list.decref.exit
  %35 = inttoptr i64 %31 to ptr
  %36 = getelementptr inbounds i64, ptr %35, i64 -1
  %37 = load i64, ptr %36, align 8
  %.not.i = icmp eq i64 %37, 0
  br i1 %.not.i, label %utils.decref.exit, label %38

38:                                               ; preds = %34
  %39 = atomicrmw sub ptr %36, i64 1 monotonic, align 8
  %40 = icmp eq i64 %39, 1
  br i1 %40, label %41, label %utils.decref.exit

41:                                               ; preds = %38
  %42 = getelementptr inbounds i64, ptr %35, i64 -2
  tail call void @roc_dealloc(ptr nonnull align 1 %42, i32 8)
  br label %utils.decref.exit

utils.decref.exit:                                ; preds = %roc_builtins.list.decref.exit, %34, %38, %41
  ret void
}

; Function Attrs: nounwind
define private fastcc void @"#Attr_#dec_14"(%list.RocList %"#arg1") unnamed_addr #10 {
entry:
  %"#arg1.fca.0.extract" = extractvalue %list.RocList %"#arg1", 0
  %"#arg1.fca.1.extract" = extractvalue %list.RocList %"#arg1", 1
  %"#arg1.fca.2.extract" = extractvalue %list.RocList %"#arg1", 2
  %.not78 = icmp slt i64 %"#arg1.fca.2.extract", 0
  %0 = select i1 %.not78, i64 %"#arg1.fca.1.extract", i64 %"#arg1.fca.2.extract"
  %1 = icmp ne i64 %0, 0
  %or.cond = or i1 %.not78, %1
  %2 = ptrtoint ptr %"#arg1.fca.0.extract" to i64
  %3 = shl i64 %"#arg1.fca.2.extract", 1
  br i1 %or.cond, label %list.RocList.refcount.exit, label %list.RocList.refcount.exit.thread

list.RocList.refcount.exit:                       ; preds = %entry
  %4 = select i1 %.not78, i64 %3, i64 %2
  %5 = inttoptr i64 %4 to ptr
  %6 = getelementptr inbounds i64, ptr %5, i64 -1
  %7 = load i64, ptr %6, align 8
  %8 = icmp eq i64 %7, 1
  br i1 %8, label %list.RocList.refcount.exit.thread, label %roc_builtins.list.decref.exit

list.RocList.refcount.exit.thread:                ; preds = %entry, %list.RocList.refcount.exit
  %.pre-phi90 = phi ptr [ %5, %list.RocList.refcount.exit ], [ %"#arg1.fca.0.extract", %entry ]
  %.pre-phi88 = phi i64 [ %4, %list.RocList.refcount.exit ], [ %2, %entry ]
  %.not.i.i = icmp eq i64 %.pre-phi88, 0
  br i1 %.not.i.i, label %roc_builtins.list.decref.exit, label %9

9:                                                ; preds = %list.RocList.refcount.exit.thread
  br i1 %.not78, label %10, label %list.RocList.getAllocationElementCount.exit

10:                                               ; preds = %9
  %11 = getelementptr inbounds i64, ptr %.pre-phi90, i64 -2
  %common.ret.op.in.i.sroa.speculate.load.29 = load i64, ptr %11, align 8
  br label %list.RocList.getAllocationElementCount.exit

list.RocList.getAllocationElementCount.exit:      ; preds = %9, %10
  %common.ret.op.in.i.sroa.speculated = phi i64 [ %common.ret.op.in.i.sroa.speculate.load.29, %10 ], [ %"#arg1.fca.1.extract", %9 ]
  %.not = icmp eq i64 %common.ret.op.in.i.sroa.speculated, 0
  br i1 %.not, label %roc_builtins.list.decref.exit, label %.lr.ph

.lr.ph:                                           ; preds = %list.RocList.getAllocationElementCount.exit, %"#Attr_#dec_1.exit"
  %.sroa.0.0.i.i82 = phi i64 [ %23, %"#Attr_#dec_1.exit" ], [ 0, %list.RocList.getAllocationElementCount.exit ]
  %12 = mul nuw i64 %.sroa.0.0.i.i82, 24
  %13 = getelementptr inbounds i8, ptr %.pre-phi90, i64 %12
  %load_str_to_stack.elt3.i = getelementptr inbounds %str.RocStr, ptr %13, i64 0, i32 2
  %load_str_to_stack.unpack4.i = load i64, ptr %load_str_to_stack.elt3.i, align 8
  %is_big_str.i = icmp sgt i64 %load_str_to_stack.unpack4.i, 0
  br i1 %is_big_str.i, label %modify_rc.i, label %"#Attr_#dec_1.exit"

modify_rc.i:                                      ; preds = %.lr.ph
  %.sroa.0.0.copyload.i.i = load ptr, ptr %13, align 8
  %.sroa.2.0..sroa_idx.i.i = getelementptr inbounds i8, ptr %13, i64 8
  %.sroa.2.0.copyload.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i, align 8
  %14 = ptrtoint ptr %.sroa.0.0.copyload.i.i to i64
  %15 = shl nuw i64 %load_str_to_stack.unpack4.i, 1
  %isneg.i.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i, 0
  %16 = select i1 %isneg.i.i.i, i64 %15, i64 %14
  %17 = inttoptr i64 %16 to ptr
  %get_rc_ptr.i = getelementptr inbounds i64, ptr %17, i64 -1
  %18 = load i64, ptr %get_rc_ptr.i, align 8
  %.not.i.i2 = icmp eq i64 %18, 0
  br i1 %.not.i.i2, label %"#Attr_#dec_1.exit", label %19

19:                                               ; preds = %modify_rc.i
  %20 = atomicrmw sub ptr %get_rc_ptr.i, i64 1 monotonic, align 8
  %21 = icmp eq i64 %20, 1
  br i1 %21, label %22, label %"#Attr_#dec_1.exit"

22:                                               ; preds = %19
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i, i32 8)
  br label %"#Attr_#dec_1.exit"

"#Attr_#dec_1.exit":                              ; preds = %.lr.ph, %modify_rc.i, %19, %22
  %23 = add nuw i64 %.sroa.0.0.i.i82, 1
  %exitcond.not = icmp eq i64 %23, %common.ret.op.in.i.sroa.speculated
  br i1 %exitcond.not, label %roc_builtins.list.decref.exit, label %.lr.ph

roc_builtins.list.decref.exit:                    ; preds = %"#Attr_#dec_1.exit", %list.RocList.getAllocationElementCount.exit, %list.RocList.refcount.exit, %list.RocList.refcount.exit.thread
  %24 = select i1 %.not78, i64 %3, i64 %2
  %25 = icmp ne i64 %"#arg1.fca.2.extract", 0
  %26 = icmp ne i64 %24, 0
  %or.cond.i = select i1 %25, i1 %26, i1 false
  br i1 %or.cond.i, label %27, label %utils.decref.exit

27:                                               ; preds = %roc_builtins.list.decref.exit
  %28 = inttoptr i64 %24 to ptr
  %29 = getelementptr inbounds i64, ptr %28, i64 -1
  %30 = load i64, ptr %29, align 8
  %.not.i = icmp eq i64 %30, 0
  br i1 %.not.i, label %utils.decref.exit, label %31

31:                                               ; preds = %27
  %32 = atomicrmw sub ptr %29, i64 1 monotonic, align 8
  %33 = icmp eq i64 %32, 1
  br i1 %33, label %34, label %utils.decref.exit

34:                                               ; preds = %31
  %35 = getelementptr inbounds i64, ptr %28, i64 -2
  tail call void @roc_dealloc(ptr nonnull align 1 %35, i32 8)
  br label %utils.decref.exit

utils.decref.exit:                                ; preds = %roc_builtins.list.decref.exit, %27, %31, %34
  ret void
}

declare void @roc_fx_stderr_line(ptr sret({ [4 x i64], i8 }), ptr) local_unnamed_addr

; Function Attrs: nounwind
define private fastcc void @"#Attr_#dec_17"(ptr nocapture readonly %"#arg1") unnamed_addr #10 {
entry:
  %tag_id_ptr = getelementptr inbounds { [3 x i64], i8 }, ptr %"#arg1", i64 0, i32 1
  %load_tag_id = load i8, ptr %tag_id_ptr, align 1
  %cond = icmp eq i8 %load_tag_id, 1
  br i1 %cond, label %tag_id_modify, label %modify_rc_union_merge

modify_rc_union_merge:                            ; preds = %8, %5, %modify_rc.i, %tag_id_modify, %entry
  ret void

tag_id_modify:                                    ; preds = %entry
  %load_str_to_stack.elt3.i = getelementptr inbounds %str.RocStr, ptr %"#arg1", i64 0, i32 2
  %load_str_to_stack.unpack4.i = load i64, ptr %load_str_to_stack.elt3.i, align 8
  %is_big_str.i = icmp sgt i64 %load_str_to_stack.unpack4.i, 0
  br i1 %is_big_str.i, label %modify_rc.i, label %modify_rc_union_merge

modify_rc.i:                                      ; preds = %tag_id_modify
  %.sroa.0.0.copyload.i.i = load ptr, ptr %"#arg1", align 8
  %.sroa.2.0..sroa_idx.i.i = getelementptr inbounds i8, ptr %"#arg1", i64 8
  %.sroa.2.0.copyload.i.i = load i64, ptr %.sroa.2.0..sroa_idx.i.i, align 8
  %0 = ptrtoint ptr %.sroa.0.0.copyload.i.i to i64
  %1 = shl nuw i64 %load_str_to_stack.unpack4.i, 1
  %isneg.i.i.i = icmp slt i64 %.sroa.2.0.copyload.i.i, 0
  %2 = select i1 %isneg.i.i.i, i64 %1, i64 %0
  %3 = inttoptr i64 %2 to ptr
  %get_rc_ptr.i = getelementptr inbounds i64, ptr %3, i64 -1
  %4 = load i64, ptr %get_rc_ptr.i, align 8
  %.not.i.i = icmp eq i64 %4, 0
  br i1 %.not.i.i, label %modify_rc_union_merge, label %5

5:                                                ; preds = %modify_rc.i
  %6 = atomicrmw sub ptr %get_rc_ptr.i, i64 1 monotonic, align 8
  %7 = icmp eq i64 %6, 1
  br i1 %7, label %8, label %modify_rc_union_merge

8:                                                ; preds = %5
  tail call void @roc_dealloc(ptr nonnull align 1 %get_rc_ptr.i, i32 8)
  br label %modify_rc_union_merge
}

declare void @roc_fx_stdout_write(ptr sret({ [4 x i64], i8 }), ptr) local_unnamed_addr

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write)
declare void @llvm.assume(i1 noundef) #13

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i128 @llvm.abs.i128(i128, i1 immarg) #14

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i128, i1 } @llvm.smul.with.overflow.i128(i128, i128) #14

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite)
declare void @llvm.experimental.noalias.scope.decl(metadata) #15

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #16

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #16

attributes #0 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #1 = { nounwind uwtable "frame-pointer"="none" "target-cpu"="generic" "target-features"="-a510,-a520,-a65,-a710,-a720,-a76,-a78,-a78c,-addr-lsl-fast,-aes,-aggressive-fma,-alternate-sextload-cvt-f32-pattern,-altnzcv,-alu-lsl-fast,-am,-amvs,-arith-bcc-fusion,-arith-cbz-fusion,-ascend-store-address,-b16b16,-balance-fp-ops,-bf16,-brbe,-bti,-call-saved-x10,-call-saved-x11,-call-saved-x12,-call-saved-x13,-call-saved-x14,-call-saved-x15,-call-saved-x18,-call-saved-x8,-call-saved-x9,-ccdp,-ccidx,-ccpp,-chk,-clrbhb,-cmp-bcc-fusion,-complxnum,-CONTEXTIDREL2,-cortex-r82,-cpa,-crc,-crypto,-cssc,-d128,-disable-latency-sched-heuristic,-disable-ldp,-disable-stp,-dit,-dotprod,-ecv,-el2vmsa,-el3,+enable-select-opt,+ete,-exynos-cheap-as-move,-f32mm,-f64mm,-faminmax,-fgt,-fix-cortex-a53-835769,-flagm,-fmv,-force-32bit-jump-tables,-fp16fml,-fp8,-fp8dot2,-fp8dot4,-fp8fma,+fp-armv8,-fpmr,-fptoint,-fullfp16,-fuse-address,-fuse-addsub-2reg-const1,+fuse-adrp-add,+fuse-aes,-fuse-arith-logic,-fuse-crypto-eor,-fuse-csel,-fuse-literals,-gcs,-harden-sls-blr,-harden-sls-nocomdat,-harden-sls-retbr,-hbc,-hcx,-i8mm,-ite,-jsconv,-ldp-aligned-only,-lor,-ls64,-lse,-lse128,-lse2,-lut,-mec,-mops,-mpam,-mte,+neon,-nmi,-no-bti-at-return-twice,-no-neg-immediates,-no-sve-fp-ld1r,-no-zcz-fp,-nv,-outline-atomics,-pan,-pan-rwv,-pauth,-pauth-lr,-perfmon,-predictable-select-expensive,-predres,-prfm-slc-target,-rand,-ras,-rasv2,-rcpc,-rcpc3,-rcpc-immo,-rdm,-reserve-x1,-reserve-x10,-reserve-x11,-reserve-x12,-reserve-x13,-reserve-x14,-reserve-x15,-reserve-x18,-reserve-x2,-reserve-x20,-reserve-x21,-reserve-x22,-reserve-x23,-reserve-x24,-reserve-x25,-reserve-x26,-reserve-x27,-reserve-x28,-reserve-x3,-reserve-x30,-reserve-x4,-reserve-x5,-reserve-x6,-reserve-x7,-reserve-x9,-rme,-sb,-sel2,-sha2,-sha3,-slow-misaligned-128store,-slow-paired-128,-slow-strqro-store,-sm4,-sme,-sme2,-sme2p1,-sme-f16f16,-sme-f64f64,-sme-f8f16,-sme-f8f32,-sme-fa64,-sme-i16i64,-sme-lutv2,-spe,-spe-eef,-specres2,-specrestrict,-ssbs,-ssve-fp8dot2,-ssve-fp8dot4,-ssve-fp8fma,-store-pair-suppress,-stp-aligned-only,-strict-align,-sve,-sve2,-sve2-aes,-sve2-bitperm,-sve2-sha3,-sve2-sm4,-sve2p1,-tagged-globals,-the,-tlb-rmi,-tlbiw,-tme,-tpidr-el1,-tpidr-el2,-tpidr-el3,-tpidrro-el0,-tracev8.4,+trbe,-uaops,-use-experimental-zeroing-pseudos,+use-postra-scheduler,-use-reciprocal-square-root,-use-scalar-inc-vl,-v8.1a,-v8.2a,-v8.3a,-v8.4a,-v8.5a,-v8.6a,-v8.7a,-v8.8a,-v8.9a,-v8a,-v8r,-v9.1a,-v9.2a,-v9.3a,-v9.4a,-v9.5a,-v9a,-vh,-wfxt,-xs,-zcm,-zcz,-zcz-fp-workaround,-zcz-gp" }
attributes #2 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #3 = { nofree norecurse nosync nounwind memory(argmem: write) uwtable "frame-pointer"="none" "target-cpu"="generic" "target-features"="-a510,-a520,-a65,-a710,-a720,-a76,-a78,-a78c,-addr-lsl-fast,-aes,-aggressive-fma,-alternate-sextload-cvt-f32-pattern,-altnzcv,-alu-lsl-fast,-am,-amvs,-arith-bcc-fusion,-arith-cbz-fusion,-ascend-store-address,-b16b16,-balance-fp-ops,-bf16,-brbe,-bti,-call-saved-x10,-call-saved-x11,-call-saved-x12,-call-saved-x13,-call-saved-x14,-call-saved-x15,-call-saved-x18,-call-saved-x8,-call-saved-x9,-ccdp,-ccidx,-ccpp,-chk,-clrbhb,-cmp-bcc-fusion,-complxnum,-CONTEXTIDREL2,-cortex-r82,-cpa,-crc,-crypto,-cssc,-d128,-disable-latency-sched-heuristic,-disable-ldp,-disable-stp,-dit,-dotprod,-ecv,-el2vmsa,-el3,+enable-select-opt,+ete,-exynos-cheap-as-move,-f32mm,-f64mm,-faminmax,-fgt,-fix-cortex-a53-835769,-flagm,-fmv,-force-32bit-jump-tables,-fp16fml,-fp8,-fp8dot2,-fp8dot4,-fp8fma,+fp-armv8,-fpmr,-fptoint,-fullfp16,-fuse-address,-fuse-addsub-2reg-const1,+fuse-adrp-add,+fuse-aes,-fuse-arith-logic,-fuse-crypto-eor,-fuse-csel,-fuse-literals,-gcs,-harden-sls-blr,-harden-sls-nocomdat,-harden-sls-retbr,-hbc,-hcx,-i8mm,-ite,-jsconv,-ldp-aligned-only,-lor,-ls64,-lse,-lse128,-lse2,-lut,-mec,-mops,-mpam,-mte,+neon,-nmi,-no-bti-at-return-twice,-no-neg-immediates,-no-sve-fp-ld1r,-no-zcz-fp,-nv,-outline-atomics,-pan,-pan-rwv,-pauth,-pauth-lr,-perfmon,-predictable-select-expensive,-predres,-prfm-slc-target,-rand,-ras,-rasv2,-rcpc,-rcpc3,-rcpc-immo,-rdm,-reserve-x1,-reserve-x10,-reserve-x11,-reserve-x12,-reserve-x13,-reserve-x14,-reserve-x15,-reserve-x18,-reserve-x2,-reserve-x20,-reserve-x21,-reserve-x22,-reserve-x23,-reserve-x24,-reserve-x25,-reserve-x26,-reserve-x27,-reserve-x28,-reserve-x3,-reserve-x30,-reserve-x4,-reserve-x5,-reserve-x6,-reserve-x7,-reserve-x9,-rme,-sb,-sel2,-sha2,-sha3,-slow-misaligned-128store,-slow-paired-128,-slow-strqro-store,-sm4,-sme,-sme2,-sme2p1,-sme-f16f16,-sme-f64f64,-sme-f8f16,-sme-f8f32,-sme-fa64,-sme-i16i64,-sme-lutv2,-spe,-spe-eef,-specres2,-specrestrict,-ssbs,-ssve-fp8dot2,-ssve-fp8dot4,-ssve-fp8fma,-store-pair-suppress,-stp-aligned-only,-strict-align,-sve,-sve2,-sve2-aes,-sve2-bitperm,-sve2-sha3,-sve2-sm4,-sve2p1,-tagged-globals,-the,-tlb-rmi,-tlbiw,-tme,-tpidr-el1,-tpidr-el2,-tpidr-el3,-tpidrro-el0,-tracev8.4,+trbe,-uaops,-use-experimental-zeroing-pseudos,+use-postra-scheduler,-use-reciprocal-square-root,-use-scalar-inc-vl,-v8.1a,-v8.2a,-v8.3a,-v8.4a,-v8.5a,-v8.6a,-v8.7a,-v8.8a,-v8.9a,-v8a,-v8r,-v9.1a,-v9.2a,-v9.3a,-v9.4a,-v9.5a,-v9a,-vh,-wfxt,-xs,-zcm,-zcz,-zcz-fp-workaround,-zcz-gp" }
attributes #4 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #5 = { noreturn nounwind uwtable "frame-pointer"="none" "target-cpu"="generic" "target-features"="-a510,-a520,-a65,-a710,-a720,-a76,-a78,-a78c,-addr-lsl-fast,-aes,-aggressive-fma,-alternate-sextload-cvt-f32-pattern,-altnzcv,-alu-lsl-fast,-am,-amvs,-arith-bcc-fusion,-arith-cbz-fusion,-ascend-store-address,-b16b16,-balance-fp-ops,-bf16,-brbe,-bti,-call-saved-x10,-call-saved-x11,-call-saved-x12,-call-saved-x13,-call-saved-x14,-call-saved-x15,-call-saved-x18,-call-saved-x8,-call-saved-x9,-ccdp,-ccidx,-ccpp,-chk,-clrbhb,-cmp-bcc-fusion,-complxnum,-CONTEXTIDREL2,-cortex-r82,-cpa,-crc,-crypto,-cssc,-d128,-disable-latency-sched-heuristic,-disable-ldp,-disable-stp,-dit,-dotprod,-ecv,-el2vmsa,-el3,+enable-select-opt,+ete,-exynos-cheap-as-move,-f32mm,-f64mm,-faminmax,-fgt,-fix-cortex-a53-835769,-flagm,-fmv,-force-32bit-jump-tables,-fp16fml,-fp8,-fp8dot2,-fp8dot4,-fp8fma,+fp-armv8,-fpmr,-fptoint,-fullfp16,-fuse-address,-fuse-addsub-2reg-const1,+fuse-adrp-add,+fuse-aes,-fuse-arith-logic,-fuse-crypto-eor,-fuse-csel,-fuse-literals,-gcs,-harden-sls-blr,-harden-sls-nocomdat,-harden-sls-retbr,-hbc,-hcx,-i8mm,-ite,-jsconv,-ldp-aligned-only,-lor,-ls64,-lse,-lse128,-lse2,-lut,-mec,-mops,-mpam,-mte,+neon,-nmi,-no-bti-at-return-twice,-no-neg-immediates,-no-sve-fp-ld1r,-no-zcz-fp,-nv,-outline-atomics,-pan,-pan-rwv,-pauth,-pauth-lr,-perfmon,-predictable-select-expensive,-predres,-prfm-slc-target,-rand,-ras,-rasv2,-rcpc,-rcpc3,-rcpc-immo,-rdm,-reserve-x1,-reserve-x10,-reserve-x11,-reserve-x12,-reserve-x13,-reserve-x14,-reserve-x15,-reserve-x18,-reserve-x2,-reserve-x20,-reserve-x21,-reserve-x22,-reserve-x23,-reserve-x24,-reserve-x25,-reserve-x26,-reserve-x27,-reserve-x28,-reserve-x3,-reserve-x30,-reserve-x4,-reserve-x5,-reserve-x6,-reserve-x7,-reserve-x9,-rme,-sb,-sel2,-sha2,-sha3,-slow-misaligned-128store,-slow-paired-128,-slow-strqro-store,-sm4,-sme,-sme2,-sme2p1,-sme-f16f16,-sme-f64f64,-sme-f8f16,-sme-f8f32,-sme-fa64,-sme-i16i64,-sme-lutv2,-spe,-spe-eef,-specres2,-specrestrict,-ssbs,-ssve-fp8dot2,-ssve-fp8dot4,-ssve-fp8fma,-store-pair-suppress,-stp-aligned-only,-strict-align,-sve,-sve2,-sve2-aes,-sve2-bitperm,-sve2-sha3,-sve2-sm4,-sve2p1,-tagged-globals,-the,-tlb-rmi,-tlbiw,-tme,-tpidr-el1,-tpidr-el2,-tpidr-el3,-tpidrro-el0,-tracev8.4,+trbe,-uaops,-use-experimental-zeroing-pseudos,+use-postra-scheduler,-use-reciprocal-square-root,-use-scalar-inc-vl,-v8.1a,-v8.2a,-v8.3a,-v8.4a,-v8.5a,-v8.6a,-v8.7a,-v8.8a,-v8.9a,-v8a,-v8r,-v9.1a,-v9.2a,-v9.3a,-v9.4a,-v9.5a,-v9a,-vh,-wfxt,-xs,-zcm,-zcz,-zcz-fp-workaround,-zcz-gp" }
attributes #6 = { nofree norecurse nosync nounwind memory(read, argmem: readwrite, inaccessiblemem: none) uwtable "frame-pointer"="none" "target-cpu"="generic" "target-features"="-a510,-a520,-a65,-a710,-a720,-a76,-a78,-a78c,-addr-lsl-fast,-aes,-aggressive-fma,-alternate-sextload-cvt-f32-pattern,-altnzcv,-alu-lsl-fast,-am,-amvs,-arith-bcc-fusion,-arith-cbz-fusion,-ascend-store-address,-b16b16,-balance-fp-ops,-bf16,-brbe,-bti,-call-saved-x10,-call-saved-x11,-call-saved-x12,-call-saved-x13,-call-saved-x14,-call-saved-x15,-call-saved-x18,-call-saved-x8,-call-saved-x9,-ccdp,-ccidx,-ccpp,-chk,-clrbhb,-cmp-bcc-fusion,-complxnum,-CONTEXTIDREL2,-cortex-r82,-cpa,-crc,-crypto,-cssc,-d128,-disable-latency-sched-heuristic,-disable-ldp,-disable-stp,-dit,-dotprod,-ecv,-el2vmsa,-el3,+enable-select-opt,+ete,-exynos-cheap-as-move,-f32mm,-f64mm,-faminmax,-fgt,-fix-cortex-a53-835769,-flagm,-fmv,-force-32bit-jump-tables,-fp16fml,-fp8,-fp8dot2,-fp8dot4,-fp8fma,+fp-armv8,-fpmr,-fptoint,-fullfp16,-fuse-address,-fuse-addsub-2reg-const1,+fuse-adrp-add,+fuse-aes,-fuse-arith-logic,-fuse-crypto-eor,-fuse-csel,-fuse-literals,-gcs,-harden-sls-blr,-harden-sls-nocomdat,-harden-sls-retbr,-hbc,-hcx,-i8mm,-ite,-jsconv,-ldp-aligned-only,-lor,-ls64,-lse,-lse128,-lse2,-lut,-mec,-mops,-mpam,-mte,+neon,-nmi,-no-bti-at-return-twice,-no-neg-immediates,-no-sve-fp-ld1r,-no-zcz-fp,-nv,-outline-atomics,-pan,-pan-rwv,-pauth,-pauth-lr,-perfmon,-predictable-select-expensive,-predres,-prfm-slc-target,-rand,-ras,-rasv2,-rcpc,-rcpc3,-rcpc-immo,-rdm,-reserve-x1,-reserve-x10,-reserve-x11,-reserve-x12,-reserve-x13,-reserve-x14,-reserve-x15,-reserve-x18,-reserve-x2,-reserve-x20,-reserve-x21,-reserve-x22,-reserve-x23,-reserve-x24,-reserve-x25,-reserve-x26,-reserve-x27,-reserve-x28,-reserve-x3,-reserve-x30,-reserve-x4,-reserve-x5,-reserve-x6,-reserve-x7,-reserve-x9,-rme,-sb,-sel2,-sha2,-sha3,-slow-misaligned-128store,-slow-paired-128,-slow-strqro-store,-sm4,-sme,-sme2,-sme2p1,-sme-f16f16,-sme-f64f64,-sme-f8f16,-sme-f8f32,-sme-fa64,-sme-i16i64,-sme-lutv2,-spe,-spe-eef,-specres2,-specrestrict,-ssbs,-ssve-fp8dot2,-ssve-fp8dot4,-ssve-fp8fma,-store-pair-suppress,-stp-aligned-only,-strict-align,-sve,-sve2,-sve2-aes,-sve2-bitperm,-sve2-sha3,-sve2-sm4,-sve2p1,-tagged-globals,-the,-tlb-rmi,-tlbiw,-tme,-tpidr-el1,-tpidr-el2,-tpidr-el3,-tpidrro-el0,-tracev8.4,+trbe,-uaops,-use-experimental-zeroing-pseudos,+use-postra-scheduler,-use-reciprocal-square-root,-use-scalar-inc-vl,-v8.1a,-v8.2a,-v8.3a,-v8.4a,-v8.5a,-v8.6a,-v8.7a,-v8.8a,-v8.9a,-v8a,-v8r,-v9.1a,-v9.2a,-v9.3a,-v9.4a,-v9.5a,-v9a,-vh,-wfxt,-xs,-zcm,-zcz,-zcz-fp-workaround,-zcz-gp" }
attributes #7 = { mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite, inaccessiblemem: readwrite) uwtable "frame-pointer"="none" "target-cpu"="generic" "target-features"="-a510,-a520,-a65,-a710,-a720,-a76,-a78,-a78c,-addr-lsl-fast,-aes,-aggressive-fma,-alternate-sextload-cvt-f32-pattern,-altnzcv,-alu-lsl-fast,-am,-amvs,-arith-bcc-fusion,-arith-cbz-fusion,-ascend-store-address,-b16b16,-balance-fp-ops,-bf16,-brbe,-bti,-call-saved-x10,-call-saved-x11,-call-saved-x12,-call-saved-x13,-call-saved-x14,-call-saved-x15,-call-saved-x18,-call-saved-x8,-call-saved-x9,-ccdp,-ccidx,-ccpp,-chk,-clrbhb,-cmp-bcc-fusion,-complxnum,-CONTEXTIDREL2,-cortex-r82,-cpa,-crc,-crypto,-cssc,-d128,-disable-latency-sched-heuristic,-disable-ldp,-disable-stp,-dit,-dotprod,-ecv,-el2vmsa,-el3,+enable-select-opt,+ete,-exynos-cheap-as-move,-f32mm,-f64mm,-faminmax,-fgt,-fix-cortex-a53-835769,-flagm,-fmv,-force-32bit-jump-tables,-fp16fml,-fp8,-fp8dot2,-fp8dot4,-fp8fma,+fp-armv8,-fpmr,-fptoint,-fullfp16,-fuse-address,-fuse-addsub-2reg-const1,+fuse-adrp-add,+fuse-aes,-fuse-arith-logic,-fuse-crypto-eor,-fuse-csel,-fuse-literals,-gcs,-harden-sls-blr,-harden-sls-nocomdat,-harden-sls-retbr,-hbc,-hcx,-i8mm,-ite,-jsconv,-ldp-aligned-only,-lor,-ls64,-lse,-lse128,-lse2,-lut,-mec,-mops,-mpam,-mte,+neon,-nmi,-no-bti-at-return-twice,-no-neg-immediates,-no-sve-fp-ld1r,-no-zcz-fp,-nv,-outline-atomics,-pan,-pan-rwv,-pauth,-pauth-lr,-perfmon,-predictable-select-expensive,-predres,-prfm-slc-target,-rand,-ras,-rasv2,-rcpc,-rcpc3,-rcpc-immo,-rdm,-reserve-x1,-reserve-x10,-reserve-x11,-reserve-x12,-reserve-x13,-reserve-x14,-reserve-x15,-reserve-x18,-reserve-x2,-reserve-x20,-reserve-x21,-reserve-x22,-reserve-x23,-reserve-x24,-reserve-x25,-reserve-x26,-reserve-x27,-reserve-x28,-reserve-x3,-reserve-x30,-reserve-x4,-reserve-x5,-reserve-x6,-reserve-x7,-reserve-x9,-rme,-sb,-sel2,-sha2,-sha3,-slow-misaligned-128store,-slow-paired-128,-slow-strqro-store,-sm4,-sme,-sme2,-sme2p1,-sme-f16f16,-sme-f64f64,-sme-f8f16,-sme-f8f32,-sme-fa64,-sme-i16i64,-sme-lutv2,-spe,-spe-eef,-specres2,-specrestrict,-ssbs,-ssve-fp8dot2,-ssve-fp8dot4,-ssve-fp8fma,-store-pair-suppress,-stp-aligned-only,-strict-align,-sve,-sve2,-sve2-aes,-sve2-bitperm,-sve2-sha3,-sve2-sm4,-sve2p1,-tagged-globals,-the,-tlb-rmi,-tlbiw,-tme,-tpidr-el1,-tpidr-el2,-tpidr-el3,-tpidrro-el0,-tracev8.4,+trbe,-uaops,-use-experimental-zeroing-pseudos,+use-postra-scheduler,-use-reciprocal-square-root,-use-scalar-inc-vl,-v8.1a,-v8.2a,-v8.3a,-v8.4a,-v8.5a,-v8.6a,-v8.7a,-v8.8a,-v8.9a,-v8a,-v8r,-v9.1a,-v9.2a,-v9.3a,-v9.4a,-v9.5a,-v9a,-vh,-wfxt,-xs,-zcm,-zcz,-zcz-fp-workaround,-zcz-gp" }
attributes #8 = { mustprogress nofree norecurse nosync nounwind willreturn uwtable "frame-pointer"="none" "target-cpu"="generic" "target-features"="-a510,-a520,-a65,-a710,-a720,-a76,-a78,-a78c,-addr-lsl-fast,-aes,-aggressive-fma,-alternate-sextload-cvt-f32-pattern,-altnzcv,-alu-lsl-fast,-am,-amvs,-arith-bcc-fusion,-arith-cbz-fusion,-ascend-store-address,-b16b16,-balance-fp-ops,-bf16,-brbe,-bti,-call-saved-x10,-call-saved-x11,-call-saved-x12,-call-saved-x13,-call-saved-x14,-call-saved-x15,-call-saved-x18,-call-saved-x8,-call-saved-x9,-ccdp,-ccidx,-ccpp,-chk,-clrbhb,-cmp-bcc-fusion,-complxnum,-CONTEXTIDREL2,-cortex-r82,-cpa,-crc,-crypto,-cssc,-d128,-disable-latency-sched-heuristic,-disable-ldp,-disable-stp,-dit,-dotprod,-ecv,-el2vmsa,-el3,+enable-select-opt,+ete,-exynos-cheap-as-move,-f32mm,-f64mm,-faminmax,-fgt,-fix-cortex-a53-835769,-flagm,-fmv,-force-32bit-jump-tables,-fp16fml,-fp8,-fp8dot2,-fp8dot4,-fp8fma,+fp-armv8,-fpmr,-fptoint,-fullfp16,-fuse-address,-fuse-addsub-2reg-const1,+fuse-adrp-add,+fuse-aes,-fuse-arith-logic,-fuse-crypto-eor,-fuse-csel,-fuse-literals,-gcs,-harden-sls-blr,-harden-sls-nocomdat,-harden-sls-retbr,-hbc,-hcx,-i8mm,-ite,-jsconv,-ldp-aligned-only,-lor,-ls64,-lse,-lse128,-lse2,-lut,-mec,-mops,-mpam,-mte,+neon,-nmi,-no-bti-at-return-twice,-no-neg-immediates,-no-sve-fp-ld1r,-no-zcz-fp,-nv,-outline-atomics,-pan,-pan-rwv,-pauth,-pauth-lr,-perfmon,-predictable-select-expensive,-predres,-prfm-slc-target,-rand,-ras,-rasv2,-rcpc,-rcpc3,-rcpc-immo,-rdm,-reserve-x1,-reserve-x10,-reserve-x11,-reserve-x12,-reserve-x13,-reserve-x14,-reserve-x15,-reserve-x18,-reserve-x2,-reserve-x20,-reserve-x21,-reserve-x22,-reserve-x23,-reserve-x24,-reserve-x25,-reserve-x26,-reserve-x27,-reserve-x28,-reserve-x3,-reserve-x30,-reserve-x4,-reserve-x5,-reserve-x6,-reserve-x7,-reserve-x9,-rme,-sb,-sel2,-sha2,-sha3,-slow-misaligned-128store,-slow-paired-128,-slow-strqro-store,-sm4,-sme,-sme2,-sme2p1,-sme-f16f16,-sme-f64f64,-sme-f8f16,-sme-f8f32,-sme-fa64,-sme-i16i64,-sme-lutv2,-spe,-spe-eef,-specres2,-specrestrict,-ssbs,-ssve-fp8dot2,-ssve-fp8dot4,-ssve-fp8fma,-store-pair-suppress,-stp-aligned-only,-strict-align,-sve,-sve2,-sve2-aes,-sve2-bitperm,-sve2-sha3,-sve2-sm4,-sve2p1,-tagged-globals,-the,-tlb-rmi,-tlbiw,-tme,-tpidr-el1,-tpidr-el2,-tpidr-el3,-tpidrro-el0,-tracev8.4,+trbe,-uaops,-use-experimental-zeroing-pseudos,+use-postra-scheduler,-use-reciprocal-square-root,-use-scalar-inc-vl,-v8.1a,-v8.2a,-v8.3a,-v8.4a,-v8.5a,-v8.6a,-v8.7a,-v8.8a,-v8.9a,-v8a,-v8r,-v9.1a,-v9.2a,-v9.3a,-v9.4a,-v9.5a,-v9a,-vh,-wfxt,-xs,-zcm,-zcz,-zcz-fp-workaround,-zcz-gp" }
attributes #9 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) }
attributes #10 = { nounwind }
attributes #11 = { nofree norecurse nosync nounwind memory(readwrite, inaccessiblemem: none) }
attributes #12 = { cold noinline noreturn nounwind }
attributes #13 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: write) }
attributes #14 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #15 = { nocallback nofree nosync nounwind willreturn memory(inaccessiblemem: readwrite) }
attributes #16 = { nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }

!llvm.module.flags = !{!0}

!0 = !{i32 2, !"Debug Info Version", i32 3}
!1 = !{i64 0, i64 65}
!2 = distinct !{!2, !3, !4}
!3 = !{!"llvm.loop.isvectorized", i32 1}
!4 = !{!"llvm.loop.unroll.runtime.disable"}
!5 = distinct !{!5, !3, !4}
!6 = distinct !{!6, !4, !3}
!7 = distinct !{!7, !3, !4}
!8 = distinct !{!8, !3, !4}
!9 = distinct !{!9, !4, !3}
!10 = !{!11, !13}
!11 = distinct !{!11, !12, !"str.RocStr.allocateBig: argument 0"}
!12 = distinct !{!12, !"str.RocStr.allocateBig"}
!13 = distinct !{!13, !14, !"str.RocStr.allocate: argument 0"}
!14 = distinct !{!14, !"str.RocStr.allocate"}
!15 = !{!16}
!16 = distinct !{!16, !17, !"math.mul__anon_8575: argument 0"}
!17 = distinct !{!17, !"math.mul__anon_8575"}
!18 = !{!19}
!19 = distinct !{!19, !20, !"math.add__anon_8577: argument 0"}
!20 = distinct !{!20, !"math.add__anon_8577"}
!21 = !{!22}
!22 = distinct !{!22, !23, !"unicode.utf8Decode2: argument 0"}
!23 = distinct !{!23, !"unicode.utf8Decode2"}
!24 = !{!25}
!25 = distinct !{!25, !26, !"unicode.utf8Decode3: argument 0"}
!26 = distinct !{!26, !"unicode.utf8Decode3"}
!27 = !{!28, !25}
!28 = distinct !{!28, !29, !"unicode.utf8Decode3AllowSurrogateHalf: argument 0"}
!29 = distinct !{!29, !"unicode.utf8Decode3AllowSurrogateHalf"}
!30 = !{!31}
!31 = distinct !{!31, !32, !"unicode.utf8Decode4: argument 0"}
!32 = distinct !{!32, !"unicode.utf8Decode4"}
!33 = !{!34}
!34 = distinct !{!34, !35, !"io.fixed_buffer_stream.FixedBufferStream([]u8).write: argument 0"}
!35 = distinct !{!35, !"io.fixed_buffer_stream.FixedBufferStream([]u8).write"}
!36 = !{!37}
!37 = distinct !{!37, !38, !"io.Writer.write: argument 0"}
!38 = distinct !{!38, !"io.Writer.write"}
!39 = !{!40}
!40 = distinct !{!40, !41, !"str.RocStr.allocateBig: argument 0"}
!41 = distinct !{!41, !"str.RocStr.allocateBig"}
!42 = !{!43}
!43 = distinct !{!43, !44, !"unicode.utf8CountCodepoints: argument 0"}
!44 = distinct !{!44, !"unicode.utf8CountCodepoints"}
!45 = !{!46}
!46 = distinct !{!46, !47, !"io.Writer.write: argument 0"}
!47 = distinct !{!47, !"io.Writer.write"}
!48 = !{!49}
!49 = distinct !{!49, !50, !"io.Writer.write: argument 0"}
!50 = distinct !{!50, !"io.Writer.write"}
!51 = !{!52}
!52 = distinct !{!52, !53, !"unicode.utf8EncodeImpl__anon_8608: argument 0"}
!53 = distinct !{!53, !"unicode.utf8EncodeImpl__anon_8608"}
!54 = !{!55, !52, !57}
!55 = distinct !{!55, !56, !"unicode.utf8CodepointSequenceLength: argument 0"}
!56 = distinct !{!56, !"unicode.utf8CodepointSequenceLength"}
!57 = distinct !{!57, !58, !"unicode.utf8Encode: argument 0"}
!58 = distinct !{!58, !"unicode.utf8Encode"}
!59 = !{!52, !57}
!60 = !{!57}
!61 = !{!62}
!62 = distinct !{!62, !63, !"io.Writer.write: argument 0"}
!63 = distinct !{!63, !"io.Writer.write"}
!64 = !{!65}
!65 = distinct !{!65, !66, !"io.Writer.write: argument 0"}
!66 = distinct !{!66, !"io.Writer.write"}
!67 = !{!68}
!68 = distinct !{!68, !69, !"io.Writer.write: argument 0"}
!69 = distinct !{!69, !"io.Writer.write"}
!70 = !{!71}
!71 = distinct !{!71, !72, !"io.Writer.write: argument 0"}
!72 = distinct !{!72, !"io.Writer.write"}
!73 = !{!74}
!74 = distinct !{!74, !75, !"io.Writer.write: argument 0"}
!75 = distinct !{!75, !"io.Writer.write"}
!76 = !{!77}
!77 = distinct !{!77, !78, !"io.Writer.write: argument 0"}
!78 = distinct !{!78, !"io.Writer.write"}
!79 = !{!80, !82, !84, !86}
!80 = distinct !{!80, !81, !"list.RocList.allocate: argument 0"}
!81 = distinct !{!81, !"list.RocList.allocate"}
!82 = distinct !{!82, !83, !"list.RocList.reallocate: argument 0"}
!83 = distinct !{!83, !"list.RocList.reallocate"}
!84 = distinct !{!84, !85, !"roc_builtins.list.reserve: argument 0"}
!85 = distinct !{!85, !"roc_builtins.list.reserve"}
!86 = distinct !{!86, !87, !"roc_builtins.list.with_capacity: argument 0"}
!87 = distinct !{!87, !"roc_builtins.list.with_capacity"}
!88 = !{!89, !91, !93, !95}
!89 = distinct !{!89, !90, !"list.RocList.allocate: argument 0"}
!90 = distinct !{!90, !"list.RocList.allocate"}
!91 = distinct !{!91, !92, !"list.RocList.reallocate: argument 0"}
!92 = distinct !{!92, !"list.RocList.reallocate"}
!93 = distinct !{!93, !94, !"roc_builtins.list.reserve: argument 0"}
!94 = distinct !{!94, !"roc_builtins.list.reserve"}
!95 = distinct !{!95, !96, !"roc_builtins.list.with_capacity: argument 0"}
!96 = distinct !{!96, !"roc_builtins.list.with_capacity"}
!97 = !{!98, !100}
!98 = distinct !{!98, !99, !"str.fromUtf8: argument 0"}
!99 = distinct !{!99, !"str.fromUtf8"}
!100 = distinct !{!100, !101, !"roc_builtins.str.from_utf8: argument 0"}
!101 = distinct !{!101, !"roc_builtins.str.from_utf8"}
!102 = !{!103, !98, !100}
!103 = distinct !{!103, !104, !"str.RocStr.fromSubListUnsafe: argument 0"}
!104 = distinct !{!104, !"str.RocStr.fromSubListUnsafe"}
!105 = !{!106, !98, !100}
!106 = distinct !{!106, !107, !"str.errorToProblem: argument 0"}
!107 = distinct !{!107, !"str.errorToProblem"}
!108 = !{!109, !106, !98, !100}
!109 = distinct !{!109, !110, !"str.numberOfNextCodepointBytes: argument 0"}
!110 = distinct !{!110, !"str.numberOfNextCodepointBytes"}
!111 = !{!100}
!112 = !{!113}
!113 = distinct !{!113, !114, !"math.powi.powi__anon_5786: argument 0"}
!114 = distinct !{!114, !"math.powi.powi__anon_5786"}
!115 = !{!116}
!116 = distinct !{!116, !117, !"roc_builtins.str.from_int.i32: argument 0"}
!117 = distinct !{!117, !"roc_builtins.str.from_int.i32"}
!118 = !{!119, !116}
!119 = distinct !{!119, !120, !"fmt.bufPrint__anon_6481: argument 0"}
!120 = distinct !{!120, !"fmt.bufPrint__anon_6481"}
!121 = !{!122, !119, !116}
!122 = distinct !{!122, !123, !"fmt.digits2: argument 0"}
!123 = distinct !{!123, !"fmt.digits2"}
!124 = !{!125, !119, !116}
!125 = distinct !{!125, !126, !"fmt.digits2: argument 0"}
!126 = distinct !{!126, !"fmt.digits2"}
!127 = !{!128, !130, !132, !116}
!128 = distinct !{!128, !129, !"str.RocStr.allocateBig: argument 0"}
!129 = distinct !{!129, !"str.RocStr.allocateBig"}
!130 = distinct !{!130, !131, !"str.RocStr.allocate: argument 0"}
!131 = distinct !{!131, !"str.RocStr.allocate"}
!132 = distinct !{!132, !133, !"str.RocStr.init: argument 0"}
!133 = distinct !{!133, !"str.RocStr.init"}
!134 = !{!135, !137}
!135 = distinct !{!135, !136, !"str.substringUnsafe: argument 0"}
!136 = distinct !{!136, !"str.substringUnsafe"}
!137 = distinct !{!137, !138, !"roc_builtins.str.substring_unsafe: argument 0"}
!138 = distinct !{!138, !"roc_builtins.str.substring_unsafe"}
!139 = !{!140}
!140 = distinct !{!140, !141, !"roc_builtins.str.substring_unsafe: argument 0"}
!141 = distinct !{!141, !"roc_builtins.str.substring_unsafe"}
!142 = !{!143, !140}
!143 = distinct !{!143, !144, !"str.substringUnsafe: argument 0"}
!144 = distinct !{!144, !"str.substringUnsafe"}
!145 = !{!146, !148, !150, !152, !143, !140}
!146 = distinct !{!146, !147, !"str.RocStr.allocateBig: argument 0"}
!147 = distinct !{!147, !"str.RocStr.allocateBig"}
!148 = distinct !{!148, !149, !"str.RocStr.allocate: argument 0"}
!149 = distinct !{!149, !"str.RocStr.allocate"}
!150 = distinct !{!150, !151, !"str.RocStr.init: argument 0"}
!151 = distinct !{!151, !"str.RocStr.init"}
!152 = distinct !{!152, !153, !"str.RocStr.fromSlice: argument 0"}
!153 = distinct !{!153, !"str.RocStr.fromSlice"}
!154 = !{!155}
!155 = distinct !{!155, !156, !"fmt.parseInt__anon_6494: argument 0"}
!156 = distinct !{!156, !"fmt.parseInt__anon_6494"}
!157 = !{!158, !155}
!158 = distinct !{!158, !159, !"fmt.parseIntWithGenericCharacter__anon_8090: argument 0"}
!159 = distinct !{!159, !"fmt.parseIntWithGenericCharacter__anon_8090"}
!160 = !{!161, !158, !155}
!161 = distinct !{!161, !162, !"fmt.parseIntWithSign__anon_8649: argument 0"}
!162 = distinct !{!162, !"fmt.parseIntWithSign__anon_8649"}
!163 = !{!164}
!164 = distinct !{!164, !165, !"roc_builtins.str.substring_unsafe: argument 0"}
!165 = distinct !{!165, !"roc_builtins.str.substring_unsafe"}
!166 = !{!167, !164}
!167 = distinct !{!167, !168, !"str.substringUnsafe: argument 0"}
!168 = distinct !{!168, !"str.substringUnsafe"}
!169 = !{!170, !172, !174, !176, !167, !164}
!170 = distinct !{!170, !171, !"str.RocStr.allocateBig: argument 0"}
!171 = distinct !{!171, !"str.RocStr.allocateBig"}
!172 = distinct !{!172, !173, !"str.RocStr.allocate: argument 0"}
!173 = distinct !{!173, !"str.RocStr.allocate"}
!174 = distinct !{!174, !175, !"str.RocStr.init: argument 0"}
!175 = distinct !{!175, !"str.RocStr.init"}
!176 = distinct !{!176, !177, !"str.RocStr.fromSlice: argument 0"}
!177 = distinct !{!177, !"str.RocStr.fromSlice"}
!178 = !{!179, !181}
!179 = distinct !{!179, !180, !"str.substringUnsafe: argument 0"}
!180 = distinct !{!180, !"str.substringUnsafe"}
!181 = distinct !{!181, !182, !"roc_builtins.str.substring_unsafe: argument 0"}
!182 = distinct !{!182, !"roc_builtins.str.substring_unsafe"}
!183 = !{!184}
!184 = distinct !{!184, !185, !"roc_builtins.str.concat: argument 0"}
!185 = distinct !{!185, !"roc_builtins.str.concat"}
!186 = !{!187, !184}
!187 = distinct !{!187, !188, !"str.RocStr.reallocate: argument 0"}
!188 = distinct !{!188, !"str.RocStr.reallocate"}
!189 = !{!190, !192, !194}
!190 = distinct !{!190, !191, !"str.RocStr.allocateBig: argument 0"}
!191 = distinct !{!191, !"str.RocStr.allocateBig"}
!192 = distinct !{!192, !193, !"str.RocStr.allocate: argument 0"}
!193 = distinct !{!193, !"str.RocStr.allocate"}
!194 = distinct !{!194, !195, !"roc_builtins.str.with_capacity: argument 0"}
!195 = distinct !{!195, !"roc_builtins.str.with_capacity"}
!196 = !{!197}
!197 = distinct !{!197, !198, !"roc_builtins.str.from_int.u64: argument 0"}
!198 = distinct !{!198, !"roc_builtins.str.from_int.u64"}
!199 = !{!200, !197}
!200 = distinct !{!200, !201, !"fmt.bufPrint__anon_6493: argument 0"}
!201 = distinct !{!201, !"fmt.bufPrint__anon_6493"}
!202 = !{!203, !200, !197}
!203 = distinct !{!203, !204, !"fmt.digits2: argument 0"}
!204 = distinct !{!204, !"fmt.digits2"}
!205 = !{!206, !200, !197}
!206 = distinct !{!206, !207, !"fmt.digits2: argument 0"}
!207 = distinct !{!207, !"fmt.digits2"}
!208 = !{!209, !211, !213, !197}
!209 = distinct !{!209, !210, !"str.RocStr.allocateBig: argument 0"}
!210 = distinct !{!210, !"str.RocStr.allocateBig"}
!211 = distinct !{!211, !212, !"str.RocStr.allocate: argument 0"}
!212 = distinct !{!212, !"str.RocStr.allocate"}
!213 = distinct !{!213, !214, !"str.RocStr.init: argument 0"}
!214 = distinct !{!214, !"str.RocStr.init"}
