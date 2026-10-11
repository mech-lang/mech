; ModuleID = 'results/binarytrees-swift-1.ll'
source_filename = "results/binarytrees-swift-1.ll"
target datalayout = "e-m:o-i64:64-i128:128-n32:64-S128"
target triple = "arm64-apple-macosx16.0.0"

%TSi = type <{ i64 }>
%T4main8TreeNodeCSg = type <{ [8 x i8] }>
%TSa = type <{ %Ts12_ArrayBufferV }>
%Ts12_ArrayBufferV = type <{ %Ts14_BridgeStorageV }>
%Ts14_BridgeStorageV = type <{ ptr }>
%struct._SwiftEmptyArrayStorage = type { %struct.HeapObject, %struct._SwiftArrayBodyStorage }
%struct.HeapObject = type { ptr, %struct.InlineRefCountsPlaceholder }
%struct.InlineRefCountsPlaceholder = type { i64 }
%struct._SwiftArrayBodyStorage = type { i64, i64 }
%swift.type_descriptor = type opaque
%swift.type = type { i64 }
%objc_class = type { ptr, ptr, ptr, ptr, ptr }
%swift.protocol_conformance_descriptor = type { i32, i32, i32, i32 }
%swift.full_boxmetadata = type { ptr, ptr, %swift.type, i32, ptr }
%swift.opaque = type opaque
%swift.method_descriptor = type { i32, i32 }
%swift.type_metadata_record = type { i32 }
%TSS = type <{ %Ts11_StringGutsV }>
%Ts11_StringGutsV = type <{ %Ts13_StringObjectV }>
%Ts13_StringObjectV = type <{ %Ts6UInt64V, ptr }>
%Ts6UInt64V = type <{ i64 }>
%objc_block = type { ptr, i32, i32, ptr, ptr }
%swift.function = type { ptr, ptr }
%swift.metadata_response = type { ptr, i64 }
%swift.vwtable = type { ptr, ptr, ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i32, i32 }
%Ts5UInt8V = type <{ i8 }>
%Ts28__ContiguousArrayStorageBaseC = type <{ %swift.refcounted, %Ts10_ArrayBodyV }>
%swift.refcounted = type { ptr, i64 }
%Ts10_ArrayBodyV = type <{ %TSo22_SwiftArrayBodyStorageV }>
%TSo22_SwiftArrayBodyStorageV = type <{ %TSi, %TSu }>
%TSu = type <{ i64 }>
%T4main8TreeNodeC = type <{ %swift.refcounted, %T4main8TreeNodeCSg, %T4main8TreeNodeCSg }>
%Ts16IndexingIteratorVySs8UTF8ViewVG = type <{ %TSs8UTF8ViewV, %TSS5IndexV }>
%TSs8UTF8ViewV = type <{ %Ts5SliceVySS8UTF8ViewVG }>
%Ts5SliceVySS8UTF8ViewVG = type <{ %TSS5IndexV, %TSS5IndexV, %TSS8UTF8ViewV }>
%TSS8UTF8ViewV = type <{ %Ts11_StringGutsV }>
%TSS5IndexV = type <{ %Ts6UInt64V }>

@"$s4main1nSivp" = hidden local_unnamed_addr global %TSi zeroinitializer, align 8
@"$s4main8minDepthSivp" = hidden local_unnamed_addr global %TSi zeroinitializer, align 8
@"$s4main8maxDepthSivp" = hidden local_unnamed_addr global %TSi zeroinitializer, align 8
@"$s4main4treeAA8TreeNodeCSgvp" = hidden local_unnamed_addr global %T4main8TreeNodeCSg zeroinitializer, align 8
@"$s4main5checkSivp" = hidden local_unnamed_addr global %TSi zeroinitializer, align 8
@"$s4main14longLivingTreeAA0D4NodeCSgvp" = hidden local_unnamed_addr global %T4main8TreeNodeCSg zeroinitializer, align 8
@"$s4main5groupSo012OS_dispatch_B0Cvp" = hidden local_unnamed_addr global ptr null, align 8
@"$s4main2rqSo17OS_dispatch_queueCvp" = hidden local_unnamed_addr global ptr null, align 8
@"$s4main5queueSo012OS_dispatch_B0Cvp" = hidden local_unnamed_addr global ptr null, align 8
@"$s4main7resultsSaySSGvp" = hidden local_unnamed_addr global %TSa zeroinitializer, align 8
@_swiftEmptyArrayStorage = external global %struct._SwiftEmptyArrayStorage, align 8
@"$ss23_ContiguousArrayStorageCMn" = external global %swift.type_descriptor, align 4
@"got.$ss23_ContiguousArrayStorageCMn" = private unnamed_addr constant ptr @"$ss23_ContiguousArrayStorageCMn"
@"symbolic _____yypG s23_ContiguousArrayStorageC" = linkonce_odr hidden constant <{ i8, i32, [4 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss23_ContiguousArrayStorageCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [4 x i8], i8 }>, ptr @"symbolic _____yypG s23_ContiguousArrayStorageC", i32 0, i32 1) to i64)) to i32), [4 x i8] c"yypG", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss23_ContiguousArrayStorageCyypGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____yypG s23_ContiguousArrayStorageC" to i64), i64 ptrtoint (ptr @"$ss23_ContiguousArrayStorageCyypGMD" to i64)) to i32), i32 -9 }, align 8
@".str.22.stretch tree of depth " = private unnamed_addr constant [23 x i8] c"stretch tree of depth \00"
@"$sSiN" = external global %swift.type, align 8
@"$sSis23CustomStringConvertiblesWP" = external global ptr, align 8
@"$sSSN" = external global %swift.type, align 8
@"$sSo17OS_dispatch_queueCML" = linkonce_odr hidden local_unnamed_addr global ptr null, align 8
@"OBJC_CLASS_REF_$_OS_dispatch_queue" = private externally_initialized global ptr @"OBJC_CLASS_$_OS_dispatch_queue", section "__DATA,__objc_classrefs,regular,no_dead_strip", align 8
@"OBJC_CLASS_$_OS_dispatch_queue" = external global %objc_class, align 8
@"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWL" = linkonce_odr hidden local_unnamed_addr global ptr null, align 8
@"$sSo17OS_dispatch_queueC8DispatchE10AttributesVs10SetAlgebraACMc" = external global %swift.protocol_conformance_descriptor, align 4
@"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMn" = external global %swift.type_descriptor, align 4
@"got.$sSo17OS_dispatch_queueC8DispatchE10AttributesVMn" = private unnamed_addr constant ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMn"
@"symbolic Say_____G So17OS_dispatch_queueC8DispatchE10AttributesV" = linkonce_odr hidden constant <{ [3 x i8], i8, i32, [1 x i8], i8 }> <{ [3 x i8] c"Say", i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$sSo17OS_dispatch_queueC8DispatchE10AttributesVMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ [3 x i8], i8, i32, [1 x i8], i8 }>, ptr @"symbolic Say_____G So17OS_dispatch_queueC8DispatchE10AttributesV", i32 0, i32 2) to i64)) to i32), [1 x i8] c"G", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Say_____G So17OS_dispatch_queueC8DispatchE10AttributesV" to i64), i64 ptrtoint (ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGMD" to i64)) to i32), i32 -9 }, align 8
@"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWL" = linkonce_odr hidden local_unnamed_addr global ptr null, align 8
@"$sSayxGSTsMc" = external global %swift.protocol_conformance_descriptor, align 4
@"$sSo17OS_dispatch_queueC8DispatchE20AutoreleaseFrequencyO7inherityA2EmFWC" = external local_unnamed_addr constant i32, align 4
@_NSConcreteStackBlock = external global %objc_class
@".str.5.v8@?0" = private unnamed_addr constant [6 x i8] c"v8@?0\00"
@block_descriptor = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper, ptr @block_destroy_helper, ptr @".str.5.v8@?0" }, align 8
@".str.25.long lived tree of depth " = private unnamed_addr constant [26 x i8] c"long lived tree of depth \00"
@"symbolic Si" = linkonce_odr hidden constant <{ [2 x i8], i8 }> <{ [2 x i8] c"Si", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"\01l__swift5_reflection_descriptor" = private constant { i32, i32, i32, i32 } { i32 1, i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Si" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor", i32 0, i32 3) to i64)) to i32) }, section "__TEXT,__swift5_capture, regular", no_sanitize_address, align 4
@metadata = private constant %swift.full_boxmetadata { ptr @objectdestroy, ptr null, %swift.type { i64 1024 }, i32 16, ptr @"\01l__swift5_reflection_descriptor" }, align 8
@block_descriptor.3 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.1, ptr @block_destroy_helper.2, ptr @".str.5.v8@?0" }, align 8
@"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL" = linkonce_odr hidden local_unnamed_addr global ptr null, align 8
@"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc" = external global %swift.protocol_conformance_descriptor, align 4
@"$s8Dispatch0A13WorkItemFlagsVMn" = external global %swift.type_descriptor, align 4
@"got.$s8Dispatch0A13WorkItemFlagsVMn" = private unnamed_addr constant ptr @"$s8Dispatch0A13WorkItemFlagsVMn"
@"symbolic Say_____G 8Dispatch0A13WorkItemFlagsV" = linkonce_odr hidden constant <{ [3 x i8], i8, i32, [1 x i8], i8 }> <{ [3 x i8] c"Say", i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$s8Dispatch0A13WorkItemFlagsVMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ [3 x i8], i8, i32, [1 x i8], i8 }>, ptr @"symbolic Say_____G 8Dispatch0A13WorkItemFlagsV", i32 0, i32 2) to i64)) to i32), [1 x i8] c"G", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$sSay8Dispatch0A13WorkItemFlagsVGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Say_____G 8Dispatch0A13WorkItemFlagsV" to i64), i64 ptrtoint (ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD" to i64)) to i32), i32 -9 }, align 8
@"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL" = linkonce_odr hidden local_unnamed_addr global ptr null, align 8
@"\01l_entry_point" = private constant { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @main to i64), i64 ptrtoint (ptr @"\01l_entry_point" to i64)) to i32), i32 0 }, section "__TEXT, __swift5_entry, regular, no_dead_strip", align 4
@"$s4main8TreeNodeC4leftACSgvpWvd" = hidden constant i64 16, align 8
@"$s4main8TreeNodeC5rightACSgvpWvd" = hidden constant i64 24, align 8
@"$sBoWV" = external global ptr, align 8
@"$s4main8TreeNodeCMm" = hidden global %objc_class { ptr @"OBJC_METACLASS_$__TtCs12_SwiftObject", ptr @"OBJC_METACLASS_$__TtCs12_SwiftObject", ptr @_objc_empty_cache, ptr null, ptr @_METACLASS_DATA__TtC4main8TreeNode }, align 8
@"OBJC_CLASS_$__TtCs12_SwiftObject" = external global %objc_class, align 8
@_objc_empty_cache = external global %swift.opaque
@"OBJC_METACLASS_$__TtCs12_SwiftObject" = external global %objc_class, align 8
@.str.18._TtC4main8TreeNode = private unnamed_addr constant [19 x i8] c"_TtC4main8TreeNode\00"
@_METACLASS_DATA__TtC4main8TreeNode = internal constant { i32, i32, i32, i32, ptr, ptr, ptr, ptr, ptr, ptr, ptr } { i32 129, i32 40, i32 40, i32 0, ptr null, ptr @.str.18._TtC4main8TreeNode, ptr null, ptr null, ptr null, ptr null, ptr null }, section "__DATA, __objc_const", align 8
@.str.4.left = private unnamed_addr constant [5 x i8] c"left\00"
@.str.0. = private unnamed_addr constant [1 x i8] zeroinitializer
@.str.5.right = private unnamed_addr constant [6 x i8] c"right\00"
@_IVARS__TtC4main8TreeNode = internal constant { i32, i32, [2 x { ptr, ptr, ptr, i32, i32 }] } { i32 32, i32 2, [2 x { ptr, ptr, ptr, i32, i32 }] [{ ptr, ptr, ptr, i32, i32 } { ptr @"$s4main8TreeNodeC4leftACSgvpWvd", ptr @.str.4.left, ptr @.str.0., i32 3, i32 8 }, { ptr, ptr, ptr, i32, i32 } { ptr @"$s4main8TreeNodeC5rightACSgvpWvd", ptr @.str.5.right, ptr @.str.0., i32 3, i32 8 }] }, section "__DATA, __objc_const", align 8
@_DATA__TtC4main8TreeNode = internal constant { i32, i32, i32, i32, ptr, ptr, ptr, ptr, ptr, ptr, ptr } { i32 128, i32 16, i32 32, i32 0, ptr null, ptr @.str.18._TtC4main8TreeNode, ptr null, ptr null, ptr @_IVARS__TtC4main8TreeNode, ptr null, ptr null }, section "__DATA, __objc_const", align 8
@.str.4.main = private constant [5 x i8] c"main\00"
@"$s4mainMXM" = linkonce_odr hidden constant <{ i32, i32, i32 }> <{ i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @.str.4.main to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i32, i32, i32 }>, ptr @"$s4mainMXM", i32 0, i32 2) to i64)) to i32) }>, section "__TEXT,__constg_swiftt", align 4
@.str.8.TreeNode = private constant [9 x i8] c"TreeNode\00"
@"$s4main8TreeNodeCMn" = hidden constant <{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }> <{ i32 -2147483568, i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4mainMXM" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 1) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @.str.8.TreeNode to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 2) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main8TreeNodeCMa" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 3) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main8TreeNodeCMF" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 4) to i64)) to i32), i32 0, i32 3, i32 20, i32 10, i32 2, i32 10, i32 12, i32 8, %swift.method_descriptor { i32 18, i32 0 }, %swift.method_descriptor { i32 19, i32 0 }, %swift.method_descriptor { i32 20, i32 0 }, %swift.method_descriptor { i32 18, i32 0 }, %swift.method_descriptor { i32 19, i32 0 }, %swift.method_descriptor { i32 20, i32 0 }, %swift.method_descriptor { i32 1, i32 0 }, %swift.method_descriptor { i32 16, i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main8TreeNodeC5checkSiyF" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 20, i32 1) to i64)) to i32) } }>, section "__TEXT,__constg_swiftt", align 4
@"$s4main8TreeNodeCMf" = internal global <{ ptr, ptr, ptr, i64, ptr, ptr, ptr, ptr, i32, i32, i32, i16, i16, i32, i32, ptr, ptr, i64, i64, ptr, ptr, ptr, ptr, ptr, ptr, ptr, ptr }> <{ ptr null, ptr @"$s4main8TreeNodeCfD", ptr @"$sBoWV", i64 ptrtoint (ptr @"$s4main8TreeNodeCMm" to i64), ptr @"OBJC_CLASS_$__TtCs12_SwiftObject", ptr @_objc_empty_cache, ptr null, ptr getelementptr (i8, ptr @_DATA__TtC4main8TreeNode, i64 2), i32 2, i32 0, i32 32, i16 7, i16 0, i32 184, i32 24, ptr @"$s4main8TreeNodeCMn", ptr null, i64 16, i64 24, ptr @swift_deletedMethodError, ptr @swift_deletedMethodError, ptr @swift_deletedMethodError, ptr @swift_deletedMethodError, ptr @swift_deletedMethodError, ptr @swift_deletedMethodError, ptr @swift_deletedMethodError, ptr @"$s4main8TreeNodeC5checkSiyF" }>, align 8
@"symbolic _____ 4main8TreeNodeC" = linkonce_odr hidden constant <{ i8, i32, i8 }> <{ i8 1, i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main8TreeNodeCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, i8 }>, ptr @"symbolic _____ 4main8TreeNodeC", i32 0, i32 1) to i64)) to i32), i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"symbolic _____Sg 4main8TreeNodeC" = linkonce_odr hidden constant <{ i8, i32, [2 x i8], i8 }> <{ i8 1, i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main8TreeNodeCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [2 x i8], i8 }>, ptr @"symbolic _____Sg 4main8TreeNodeC", i32 0, i32 1) to i64)) to i32), [2 x i8] c"Sg", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@0 = private constant [5 x i8] c"left\00", section "__TEXT,__swift5_reflstr, regular", no_sanitize_address
@1 = private constant [6 x i8] c"right\00", section "__TEXT,__swift5_reflstr, regular", no_sanitize_address
@"$s4main8TreeNodeCMF" = internal constant { i32, i32, i16, i16, i32, i32, i32, i32, i32, i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ 4main8TreeNodeC" to i64), i64 ptrtoint (ptr @"$s4main8TreeNodeCMF" to i64)) to i32), i32 0, i16 1, i16 12, i32 2, i32 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____Sg 4main8TreeNodeC" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i16, i16, i32, i32, i32, i32, i32, i32, i32 }, ptr @"$s4main8TreeNodeCMF", i32 0, i32 6) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @0 to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i16, i16, i32, i32, i32, i32, i32, i32, i32 }, ptr @"$s4main8TreeNodeCMF", i32 0, i32 7) to i64)) to i32), i32 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____Sg 4main8TreeNodeC" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i16, i16, i32, i32, i32, i32, i32, i32, i32 }, ptr @"$s4main8TreeNodeCMF", i32 0, i32 9) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @1 to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i16, i16, i32, i32, i32, i32, i32, i32, i32 }, ptr @"$s4main8TreeNodeCMF", i32 0, i32 10) to i64)) to i32) }, section "__TEXT,__swift5_fieldmd, regular", no_sanitize_address, align 4
@"_swift_FORCE_LOAD_$_swiftFoundation_$_main" = weak_odr hidden constant ptr @"_swift_FORCE_LOAD_$_swiftFoundation"
@"_swift_FORCE_LOAD_$_swiftObjectiveC_$_main" = weak_odr hidden constant ptr @"_swift_FORCE_LOAD_$_swiftObjectiveC"
@"_swift_FORCE_LOAD_$_swiftDarwin_$_main" = weak_odr hidden constant ptr @"_swift_FORCE_LOAD_$_swiftDarwin"
@"_swift_FORCE_LOAD_$_swiftCoreFoundation_$_main" = weak_odr hidden constant ptr @"_swift_FORCE_LOAD_$_swiftCoreFoundation"
@"_swift_FORCE_LOAD_$_swiftDispatch_$_main" = weak_odr hidden constant ptr @"_swift_FORCE_LOAD_$_swiftDispatch"
@"_swift_FORCE_LOAD_$_swiftXPC_$_main" = weak_odr hidden constant ptr @"_swift_FORCE_LOAD_$_swiftXPC"
@"_swift_FORCE_LOAD_$_swiftIOKit_$_main" = weak_odr hidden constant ptr @"_swift_FORCE_LOAD_$_swiftIOKit"
@"$sSSs25LosslessStringConvertiblesWP" = external global ptr, align 8
@"$sSSSTsWP" = external global ptr, align 8
@"$ss5UInt8VMn" = external global %swift.type_descriptor, align 4
@"got.$ss5UInt8VMn" = private unnamed_addr constant ptr @"$ss5UInt8VMn"
@"symbolic _____y_____G s23_ContiguousArrayStorageC s5UInt8V" = linkonce_odr hidden constant <{ i8, i32, [1 x i8], i8, i32, [1 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss23_ContiguousArrayStorageCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [1 x i8], i8, i32, [1 x i8], i8 }>, ptr @"symbolic _____y_____G s23_ContiguousArrayStorageC s5UInt8V", i32 0, i32 1) to i64)) to i32), [1 x i8] c"y", i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss5UInt8VMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [1 x i8], i8, i32, [1 x i8], i8 }>, ptr @"symbolic _____y_____G s23_ContiguousArrayStorageC s5UInt8V", i32 0, i32 4) to i64)) to i32), [1 x i8] c"G", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss23_ContiguousArrayStorageCys5UInt8VGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____y_____G s23_ContiguousArrayStorageC s5UInt8V" to i64), i64 ptrtoint (ptr @"$ss23_ContiguousArrayStorageCys5UInt8VGMD" to i64)) to i32), i32 -12 }, align 8
@"\01l__swift5_reflection_descriptor.4" = private constant { i32, i32, i32, i32, i32, i32, i32 } { i32 4, i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Si" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.4", i32 0, i32 3) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Si" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.4", i32 0, i32 4) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Si" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.4", i32 0, i32 5) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Si" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.4", i32 0, i32 6) to i64)) to i32) }, section "__TEXT,__swift5_capture, regular", no_sanitize_address, align 4
@metadata.6 = private constant %swift.full_boxmetadata { ptr @objectdestroy.5, ptr null, %swift.type { i64 1024 }, i32 16, ptr @"\01l__swift5_reflection_descriptor.4" }, align 8
@block_descriptor.9 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.7, ptr @block_destroy_helper.8, ptr @".str.5.v8@?0" }, align 8
@".str.17.\09 trees of depth " = private unnamed_addr constant [18 x i8] c"\09 trees of depth \00"
@"symbolic _____ySSG s23_ContiguousArrayStorageC" = linkonce_odr hidden constant <{ i8, i32, [4 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss23_ContiguousArrayStorageCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [4 x i8], i8 }>, ptr @"symbolic _____ySSG s23_ContiguousArrayStorageC", i32 0, i32 1) to i64)) to i32), [4 x i8] c"ySSG", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss23_ContiguousArrayStorageCySSGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ySSG s23_ContiguousArrayStorageC" to i64), i64 ptrtoint (ptr @"$ss23_ContiguousArrayStorageCySSGMD" to i64)) to i32), i32 -9 }, align 8
@"$s4main8TreeNodeCHn" = private constant %swift.type_metadata_record { i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main8TreeNodeCMn" to i64), i64 ptrtoint (ptr @"$s4main8TreeNodeCHn" to i64)) to i32) }, section "__TEXT, __swift5_types, regular", no_sanitize_address, align 4
@__swift_reflection_version = linkonce_odr hidden constant i16 3
@"objc_classes_$s4main8TreeNodeCN" = internal global ptr @"$s4main8TreeNodeCN", section "__DATA,__objc_classlist,regular,no_dead_strip", no_sanitize_address, align 8
@llvm.used = appending global [15 x ptr] [ptr @"\01l__swift5_reflection_descriptor", ptr @"\01l__swift5_reflection_descriptor.4", ptr @"\01l_entry_point", ptr @"$s4main8TreeNodeCHn", ptr @"$s4main8TreeNodeCMF", ptr @__swift_reflection_version, ptr @"_swift_FORCE_LOAD_$_swiftCoreFoundation_$_main", ptr @"_swift_FORCE_LOAD_$_swiftDarwin_$_main", ptr @"_swift_FORCE_LOAD_$_swiftDispatch_$_main", ptr @"_swift_FORCE_LOAD_$_swiftFoundation_$_main", ptr @"_swift_FORCE_LOAD_$_swiftIOKit_$_main", ptr @"_swift_FORCE_LOAD_$_swiftObjectiveC_$_main", ptr @"_swift_FORCE_LOAD_$_swiftXPC_$_main", ptr @main, ptr @"objc_classes_$s4main8TreeNodeCN"], section "llvm.metadata"
@llvm.compiler.used = appending global [1 x ptr] [ptr @"OBJC_CLASS_REF_$_OS_dispatch_queue"], section "llvm.metadata"

@"$s4main8TreeNodeC4leftACSgvgTq" = hidden alias %swift.method_descriptor, getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 13)
@"$s4main8TreeNodeC4leftACSgvsTq" = hidden alias %swift.method_descriptor, getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 14)
@"$s4main8TreeNodeC4leftACSgvMTq" = hidden alias %swift.method_descriptor, getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 15)
@"$s4main8TreeNodeC5rightACSgvgTq" = hidden alias %swift.method_descriptor, getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 16)
@"$s4main8TreeNodeC5rightACSgvsTq" = hidden alias %swift.method_descriptor, getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 17)
@"$s4main8TreeNodeC5rightACSgvMTq" = hidden alias %swift.method_descriptor, getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 18)
@"$s4main8TreeNodeC4left5rightA2CSg_AFtcfCTq" = hidden alias %swift.method_descriptor, getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 19)
@"$s4main8TreeNodeC5checkSiyFTq" = hidden alias %swift.method_descriptor, getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor, %swift.method_descriptor }>, ptr @"$s4main8TreeNodeCMn", i32 0, i32 20)
@"$s4main8TreeNodeCN" = hidden alias %swift.type, getelementptr inbounds (<{ ptr, ptr, ptr, i64, ptr, ptr, ptr, ptr, i32, i32, i32, i16, i16, i32, i32, ptr, ptr, i64, i64, ptr, ptr, ptr, ptr, ptr, ptr, ptr, ptr }>, ptr @"$s4main8TreeNodeCMf", i32 0, i32 3)

define i32 @main(i32 %0, ptr nocapture readnone %1) #0 {
entry:
  %2 = alloca %TSS, align 8
  %3 = alloca %Ts11_StringGutsV, align 8
  %4 = alloca %TSi, align 8
  %5 = alloca %TSi, align 8
  %6 = alloca %TSa, align 8
  %7 = alloca { %objc_block, %swift.function }, align 8
  %8 = alloca %TSS, align 8
  %9 = alloca %Ts11_StringGutsV, align 8
  %10 = alloca %TSi, align 8
  %11 = alloca %TSi, align 8
  %12 = alloca { %objc_block, %swift.function }, align 8
  %13 = alloca %TSa, align 8
  %.sroa.6 = alloca i8, align 8
  %14 = alloca <{ %Ts6UInt64V, %Ts6UInt64V }>, align 8
  %15 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64 0) #21
  %16 = extractvalue %swift.metadata_response %15, 0
  %17 = getelementptr inbounds ptr, ptr %16, i64 -1
  %.valueWitnesses = load ptr, ptr %17, align 8, !invariant.load !33, !dereferenceable !34
  %18 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses, i64 0, i32 8
  %size = load i64, ptr %18, align 8, !invariant.load !33
  %19 = alloca i8, i64 %size, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %19)
  %20 = tail call swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE20AutoreleaseFrequencyOMa"(i64 0) #21
  %21 = extractvalue %swift.metadata_response %20, 0
  %22 = getelementptr inbounds ptr, ptr %21, i64 -1
  %.valueWitnesses1 = load ptr, ptr %22, align 8, !invariant.load !33, !dereferenceable !34
  %23 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses1, i64 0, i32 8
  %size2 = load i64, ptr %23, align 8, !invariant.load !33
  %24 = alloca i8, i64 %size2, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %24)
  %25 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64 0) #21
  %26 = extractvalue %swift.metadata_response %25, 0
  %27 = getelementptr inbounds ptr, ptr %26, i64 -1
  %.valueWitnesses3 = load ptr, ptr %27, align 8, !invariant.load !33, !dereferenceable !34
  %28 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses3, i64 0, i32 8
  %size4 = load i64, ptr %28, align 8, !invariant.load !33
  %29 = alloca i8, i64 %size4, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %29)
  %30 = tail call swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMa"(i64 0) #21
  %31 = extractvalue %swift.metadata_response %30, 0
  %32 = getelementptr inbounds ptr, ptr %31, i64 -1
  %.valueWitnesses5 = load ptr, ptr %32, align 8, !invariant.load !33, !dereferenceable !34
  %33 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses5, i64 0, i32 8
  %size6 = load i64, ptr %33, align 8, !invariant.load !33
  %34 = alloca i8, i64 %size6, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %34)
  %35 = tail call swiftcc i32 @"$ss11CommandLineO4argcs5Int32VvgZ"()
  %36 = icmp sgt i32 %35, 1
  br i1 %36, label %37, label %331

37:                                               ; preds = %entry
  %38 = tail call swiftcc ptr @"$ss11CommandLineO9argumentsSaySSGvgZ"()
  %39 = getelementptr inbounds i8, ptr %38, i64 48
  %40 = load i64, ptr %39, align 8
  %._guts32._object._object = getelementptr inbounds i8, ptr %38, i64 56
  %41 = load ptr, ptr %._guts32._object._object, align 8
  %42 = tail call ptr @swift_bridgeObjectRetain(ptr returned %41) #6
  tail call void @swift_bridgeObjectRelease(ptr %38) #6
  %43 = ptrtoint ptr %41 to i64
  %44 = and i64 %43, 2305843009213693952
  %45 = icmp eq i64 %44, 0
  %46 = lshr i64 %43, 56
  %47 = and i64 %46, 15
  %48 = and i64 %40, 281474976710655
  %49 = select i1 %45, i64 %48, i64 %47
  %50 = icmp eq i64 %49, 0
  br i1 %50, label %51, label %52, !prof !35

51:                                               ; preds = %37
  tail call void @swift_bridgeObjectRelease(ptr %41) #6
  br label %331

52:                                               ; preds = %37
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %.sroa.6)
  %53 = and i64 %43, 1152921504606846976
  %54 = icmp eq i64 %53, 0
  br i1 %54, label %55, label %93, !prof !36

55:                                               ; preds = %52
  br i1 %45, label %56, label %59

56:                                               ; preds = %55
  %57 = and i64 %40, 1152921504606846976
  %58 = icmp eq i64 %57, 0
  br i1 %58, label %66, label %70, !prof !35

59:                                               ; preds = %55
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %14)
  %.elt34 = getelementptr inbounds <{ %Ts6UInt64V, %Ts6UInt64V }>, ptr %14, i64 0, i32 1
  %60 = and i64 %43, 72057594037927935
  store i64 %40, ptr %14, align 8
  store i64 %60, ptr %.elt34, align 8
  %61 = trunc i64 %40 to i8
  %62 = lshr i64 %40, 8
  %63 = trunc i64 %62 to i8
  %64 = lshr i64 %40, 16
  %65 = trunc i64 %64 to i8
  switch i8 %61, label %263 [
    i8 45, label %220
    i8 43, label %261
  ]

66:                                               ; preds = %56
  %67 = tail call swiftcc { i64, i64 } @"$ss13_StringObjectV10sharedUTF8SRys5UInt8VGvg"(i64 %40, ptr %41)
  %68 = extractvalue { i64, i64 } %67, 0
  %69 = extractvalue { i64, i64 } %67, 1
  br label %73

70:                                               ; preds = %56
  %71 = and i64 %43, 1152921504606846975
  %72 = add nuw nsw i64 %71, 32
  br label %73

73:                                               ; preds = %66, %70
  %74 = phi i64 [ %72, %70 ], [ %68, %66 ]
  %75 = phi i64 [ %48, %70 ], [ %69, %66 ]
  %76 = inttoptr i64 %74 to ptr
  %77 = load i8, ptr %76, align 1
  switch i8 %77, label %139 [
    i8 45, label %78
    i8 43, label %136
  ]

78:                                               ; preds = %73
  %79 = add i64 %75, -1
  %80 = icmp eq i64 %79, 0
  br i1 %80, label %.loopexit, label %81, !prof !35

81:                                               ; preds = %78
  %82 = icmp eq i64 %74, 0
  %83 = getelementptr inbounds %Ts5UInt8V, ptr %76, i64 1
  %84 = ptrtoint ptr %83 to i64
  %85 = select i1 %82, i64 0, i64 %84
  %86 = inttoptr i64 %85 to ptr
  br i1 %82, label %.loopexit, label %99

.loopexit.loopexit.split.loop.exit:               ; preds = %216
  %87 = extractvalue { i64, i1 } %214, 0
  br label %.loopexit

.loopexit.loopexit308.split.loop.exit:            ; preds = %132
  %88 = extractvalue { i64, i1 } %130, 0
  br label %.loopexit

.loopexit.loopexit311.split.loop.exit:            ; preds = %179
  %89 = extractvalue { i64, i1 } %177, 0
  br label %.loopexit

.loopexit:                                        ; preds = %.loopexit.loopexit311.split.loop.exit, %174, %171, %.preheader140, %.loopexit.loopexit308.split.loop.exit, %127, %124, %.preheader136, %.loopexit.loopexit.split.loop.exit, %211, %208, %.preheader, %197, %192, %189, %185, %183, %139, %160, %155, %152, %147, %141, %136, %113, %108, %104, %99, %81, %78
  %90 = phi i64 [ 0, %78 ], [ 0, %81 ], [ 0, %99 ], [ %106, %104 ], [ 0, %108 ], [ %116, %113 ], [ 0, %136 ], [ 0, %141 ], [ 0, %147 ], [ %153, %152 ], [ 0, %155 ], [ %163, %160 ], [ 0, %139 ], [ 0, %183 ], [ 0, %185 ], [ %190, %189 ], [ 0, %192 ], [ %200, %197 ], [ %87, %.loopexit.loopexit.split.loop.exit ], [ 0, %211 ], [ 0, %208 ], [ 0, %.preheader ], [ %88, %.loopexit.loopexit308.split.loop.exit ], [ 0, %127 ], [ 0, %124 ], [ 0, %.preheader136 ], [ %89, %.loopexit.loopexit311.split.loop.exit ], [ 0, %174 ], [ 0, %171 ], [ 0, %.preheader140 ]
  %91 = phi i1 [ true, %78 ], [ false, %81 ], [ true, %99 ], [ false, %104 ], [ true, %108 ], [ false, %113 ], [ true, %136 ], [ false, %141 ], [ true, %147 ], [ false, %152 ], [ true, %155 ], [ false, %160 ], [ true, %139 ], [ false, %183 ], [ true, %185 ], [ false, %189 ], [ true, %192 ], [ false, %197 ], [ false, %.loopexit.loopexit.split.loop.exit ], [ true, %211 ], [ true, %208 ], [ true, %.preheader ], [ false, %.loopexit.loopexit308.split.loop.exit ], [ true, %127 ], [ true, %124 ], [ true, %.preheader136 ], [ false, %.loopexit.loopexit311.split.loop.exit ], [ true, %174 ], [ true, %171 ], [ true, %.preheader140 ]
  store i1 %91, ptr %.sroa.6, align 8
  br label %92

92:                                               ; preds = %.loopexit, %.loopexit145
  %.sroa.0.1 = phi i64 [ %90, %.loopexit ], [ %229, %.loopexit145 ]
  %.sroa.6.0..sroa.6.0..sroa.6.0..sroa.6.8. = load i8, ptr %.sroa.6, align 8
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.6)
  call void @swift_bridgeObjectRelease(ptr %41) #6
  br label %97

93:                                               ; preds = %52
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.6)
  %94 = tail call swiftcc { i64, i8 } @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFSS_SiTg5"(i64 %40, ptr %41, i64 10)
  tail call void @swift_bridgeObjectRelease(ptr %41) #6
  %95 = extractvalue { i64, i8 } %94, 0
  %96 = extractvalue { i64, i8 } %94, 1
  br label %97

97:                                               ; preds = %92, %93
  %98 = phi i64 [ %95, %93 ], [ %.sroa.0.1, %92 ]
  %.in.in = phi i8 [ %96, %93 ], [ %.sroa.6.0..sroa.6.0..sroa.6.0..sroa.6.8., %92 ]
  %.in = and i8 %.in.in, 1
  %.not = icmp eq i8 %.in, 0
  %. = select i1 %.not, i64 %98, i64 10
  br label %331

99:                                               ; preds = %81
  %100 = getelementptr inbounds %Ts5UInt8V, ptr %86, i64 %79
  %101 = load i8, ptr %86, align 1
  %102 = add i8 %101, -48
  %103 = icmp ult i8 %102, 10
  br i1 %103, label %104, label %.loopexit, !prof !37

104:                                              ; preds = %99
  %105 = zext i8 %102 to i64
  %106 = sub nsw i64 0, %105
  %107 = icmp eq i64 %79, 1
  br i1 %107, label %.loopexit, label %108

108:                                              ; preds = %104
  %109 = getelementptr inbounds %Ts5UInt8V, ptr %86, i64 1
  %110 = load i8, ptr %109, align 1
  %111 = add i8 %110, -48
  %112 = icmp ult i8 %111, 10
  br i1 %112, label %113, label %.loopexit, !prof !37

113:                                              ; preds = %108
  %114 = mul nsw i64 %105, -10
  %115 = zext i8 %111 to i64
  %116 = sub nsw i64 %114, %115
  %117 = icmp eq i64 %79, 2
  br i1 %117, label %.loopexit, label %.preheader136.preheader

.preheader136.preheader:                          ; preds = %113
  %118 = getelementptr inbounds %Ts5UInt8V, ptr %86, i64 2
  br label %.preheader136

.preheader136:                                    ; preds = %.preheader136.preheader, %132
  %119 = phi ptr [ %134, %132 ], [ %118, %.preheader136.preheader ]
  %120 = phi i64 [ %133, %132 ], [ %116, %.preheader136.preheader ]
  %121 = load i8, ptr %119, align 1
  %122 = add i8 %121, -48
  %123 = icmp ult i8 %122, 10
  br i1 %123, label %124, label %.loopexit, !prof !37

124:                                              ; preds = %.preheader136
  %125 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %120, i64 10)
  %126 = extractvalue { i64, i1 } %125, 1
  br i1 %126, label %.loopexit, label %127

127:                                              ; preds = %124
  %128 = extractvalue { i64, i1 } %125, 0
  %129 = zext i8 %122 to i64
  %130 = tail call { i64, i1 } @llvm.ssub.with.overflow.i64(i64 %128, i64 %129)
  %131 = extractvalue { i64, i1 } %130, 1
  br i1 %131, label %.loopexit, label %132, !prof !35

132:                                              ; preds = %127
  %133 = extractvalue { i64, i1 } %130, 0
  %134 = getelementptr inbounds %Ts5UInt8V, ptr %119, i64 1
  %135 = icmp eq ptr %134, %100
  br i1 %135, label %.loopexit.loopexit308.split.loop.exit, label %.preheader136

136:                                              ; preds = %73
  %137 = add i64 %75, -1
  %138 = icmp eq i64 %137, 0
  br i1 %138, label %.loopexit, label %141, !prof !35

139:                                              ; preds = %73
  %140 = icmp eq i64 %75, 0
  br i1 %140, label %.loopexit, label %183, !prof !35

141:                                              ; preds = %136
  %142 = icmp eq i64 %74, 0
  %143 = getelementptr inbounds %Ts5UInt8V, ptr %76, i64 1
  %144 = ptrtoint ptr %143 to i64
  %145 = select i1 %142, i64 0, i64 %144
  %146 = inttoptr i64 %145 to ptr
  br i1 %142, label %.loopexit, label %147

147:                                              ; preds = %141
  %148 = getelementptr inbounds %Ts5UInt8V, ptr %146, i64 %137
  %149 = load i8, ptr %146, align 1
  %150 = add i8 %149, -48
  %151 = icmp ult i8 %150, 10
  br i1 %151, label %152, label %.loopexit, !prof !37

152:                                              ; preds = %147
  %153 = zext i8 %150 to i64
  %154 = icmp eq i64 %137, 1
  br i1 %154, label %.loopexit, label %155

155:                                              ; preds = %152
  %156 = getelementptr inbounds %Ts5UInt8V, ptr %146, i64 1
  %157 = load i8, ptr %156, align 1
  %158 = add i8 %157, -48
  %159 = icmp ult i8 %158, 10
  br i1 %159, label %160, label %.loopexit, !prof !37

160:                                              ; preds = %155
  %161 = mul nuw nsw i64 %153, 10
  %162 = zext i8 %158 to i64
  %163 = add nuw nsw i64 %161, %162
  %164 = icmp eq i64 %137, 2
  br i1 %164, label %.loopexit, label %.preheader140.preheader

.preheader140.preheader:                          ; preds = %160
  %165 = getelementptr inbounds %Ts5UInt8V, ptr %146, i64 2
  br label %.preheader140

.preheader140:                                    ; preds = %.preheader140.preheader, %179
  %166 = phi ptr [ %181, %179 ], [ %165, %.preheader140.preheader ]
  %167 = phi i64 [ %180, %179 ], [ %163, %.preheader140.preheader ]
  %168 = load i8, ptr %166, align 1
  %169 = add i8 %168, -48
  %170 = icmp ult i8 %169, 10
  br i1 %170, label %171, label %.loopexit, !prof !37

171:                                              ; preds = %.preheader140
  %172 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %167, i64 10)
  %173 = extractvalue { i64, i1 } %172, 1
  br i1 %173, label %.loopexit, label %174

174:                                              ; preds = %171
  %175 = extractvalue { i64, i1 } %172, 0
  %176 = zext i8 %169 to i64
  %177 = tail call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %175, i64 %176)
  %178 = extractvalue { i64, i1 } %177, 1
  br i1 %178, label %.loopexit, label %179, !prof !35

179:                                              ; preds = %174
  %180 = extractvalue { i64, i1 } %177, 0
  %181 = getelementptr inbounds %Ts5UInt8V, ptr %166, i64 1
  %182 = icmp eq ptr %181, %148
  br i1 %182, label %.loopexit.loopexit311.split.loop.exit, label %.preheader140

183:                                              ; preds = %139
  %184 = icmp eq i64 %74, 0
  br i1 %184, label %.loopexit, label %185

185:                                              ; preds = %183
  %186 = getelementptr inbounds %Ts5UInt8V, ptr %76, i64 %75
  %187 = add i8 %77, -48
  %188 = icmp ult i8 %187, 10
  br i1 %188, label %189, label %.loopexit, !prof !37

189:                                              ; preds = %185
  %190 = zext i8 %187 to i64
  %191 = icmp eq i64 %75, 1
  br i1 %191, label %.loopexit, label %192

192:                                              ; preds = %189
  %193 = getelementptr inbounds %Ts5UInt8V, ptr %76, i64 1
  %194 = load i8, ptr %193, align 1
  %195 = add i8 %194, -48
  %196 = icmp ult i8 %195, 10
  br i1 %196, label %197, label %.loopexit, !prof !37

197:                                              ; preds = %192
  %198 = mul nuw nsw i64 %190, 10
  %199 = zext i8 %195 to i64
  %200 = add nuw nsw i64 %198, %199
  %201 = icmp eq i64 %75, 2
  br i1 %201, label %.loopexit, label %.preheader.preheader

.preheader.preheader:                             ; preds = %197
  %202 = getelementptr inbounds %Ts5UInt8V, ptr %76, i64 2
  br label %.preheader

.preheader:                                       ; preds = %.preheader.preheader, %216
  %203 = phi ptr [ %218, %216 ], [ %202, %.preheader.preheader ]
  %204 = phi i64 [ %217, %216 ], [ %200, %.preheader.preheader ]
  %205 = load i8, ptr %203, align 1
  %206 = add i8 %205, -48
  %207 = icmp ult i8 %206, 10
  br i1 %207, label %208, label %.loopexit, !prof !37

208:                                              ; preds = %.preheader
  %209 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %204, i64 10)
  %210 = extractvalue { i64, i1 } %209, 1
  br i1 %210, label %.loopexit, label %211

211:                                              ; preds = %208
  %212 = extractvalue { i64, i1 } %209, 0
  %213 = zext i8 %206 to i64
  %214 = tail call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %212, i64 %213)
  %215 = extractvalue { i64, i1 } %214, 1
  br i1 %215, label %.loopexit, label %216, !prof !35

216:                                              ; preds = %211
  %217 = extractvalue { i64, i1 } %214, 0
  %218 = getelementptr inbounds %Ts5UInt8V, ptr %203, i64 1
  %219 = icmp eq ptr %218, %186
  br i1 %219, label %.loopexit.loopexit.split.loop.exit, label %.preheader

220:                                              ; preds = %59
  %221 = icmp eq i64 %47, 1
  br i1 %221, label %.loopexit145, label %222, !prof !35

222:                                              ; preds = %220
  %223 = getelementptr inbounds %Ts5UInt8V, ptr %14, i64 %47
  %224 = add i8 %63, -48
  %225 = icmp ult i8 %224, 10
  br i1 %225, label %231, label %.loopexit145, !prof !37

.loopexit145.loopexit.split.loop.exit:            ; preds = %327
  %226 = extractvalue { i64, i1 } %325, 0
  br label %.loopexit145

.loopexit145.loopexit316.split.loop.exit:         ; preds = %257
  %227 = extractvalue { i64, i1 } %255, 0
  br label %.loopexit145

.loopexit145.loopexit319.split.loop.exit:         ; preds = %294
  %228 = extractvalue { i64, i1 } %292, 0
  br label %.loopexit145

.loopexit145:                                     ; preds = %.loopexit145.loopexit319.split.loop.exit, %289, %286, %.preheader152, %.loopexit145.loopexit316.split.loop.exit, %252, %249, %.preheader148, %.loopexit145.loopexit.split.loop.exit, %322, %319, %.preheader144, %308, %305, %302, %298, %263, %275, %272, %269, %265, %261, %238, %235, %231, %222, %220
  %229 = phi i64 [ 0, %220 ], [ 0, %222 ], [ %233, %231 ], [ 0, %235 ], [ %241, %238 ], [ 0, %261 ], [ 0, %265 ], [ %270, %269 ], [ 0, %272 ], [ %278, %275 ], [ 0, %263 ], [ 0, %298 ], [ %303, %302 ], [ 0, %305 ], [ %311, %308 ], [ %226, %.loopexit145.loopexit.split.loop.exit ], [ 0, %322 ], [ 0, %319 ], [ 0, %.preheader144 ], [ %227, %.loopexit145.loopexit316.split.loop.exit ], [ 0, %252 ], [ 0, %249 ], [ 0, %.preheader148 ], [ %228, %.loopexit145.loopexit319.split.loop.exit ], [ 0, %289 ], [ 0, %286 ], [ 0, %.preheader152 ]
  %230 = phi i1 [ true, %220 ], [ true, %222 ], [ false, %231 ], [ true, %235 ], [ false, %238 ], [ true, %261 ], [ true, %265 ], [ false, %269 ], [ true, %272 ], [ false, %275 ], [ true, %263 ], [ true, %298 ], [ false, %302 ], [ true, %305 ], [ false, %308 ], [ false, %.loopexit145.loopexit.split.loop.exit ], [ true, %322 ], [ true, %319 ], [ true, %.preheader144 ], [ false, %.loopexit145.loopexit316.split.loop.exit ], [ true, %252 ], [ true, %249 ], [ true, %.preheader148 ], [ false, %.loopexit145.loopexit319.split.loop.exit ], [ true, %289 ], [ true, %286 ], [ true, %.preheader152 ]
  store i1 %230, ptr %.sroa.6, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %14)
  br label %92

231:                                              ; preds = %222
  %232 = zext i8 %224 to i64
  %233 = sub nsw i64 0, %232
  %234 = icmp eq i64 %47, 2
  br i1 %234, label %.loopexit145, label %235

235:                                              ; preds = %231
  %236 = add i8 %65, -48
  %237 = icmp ult i8 %236, 10
  br i1 %237, label %238, label %.loopexit145, !prof !37

238:                                              ; preds = %235
  %239 = mul nsw i64 %232, -10
  %240 = zext i8 %236 to i64
  %241 = sub nsw i64 %239, %240
  %242 = icmp eq i64 %47, 3
  br i1 %242, label %.loopexit145, label %.preheader148.preheader

.preheader148.preheader:                          ; preds = %238
  %243 = getelementptr inbounds %Ts5UInt8V, ptr %14, i64 3
  br label %.preheader148

.preheader148:                                    ; preds = %.preheader148.preheader, %257
  %244 = phi ptr [ %259, %257 ], [ %243, %.preheader148.preheader ]
  %245 = phi i64 [ %258, %257 ], [ %241, %.preheader148.preheader ]
  %246 = load i8, ptr %244, align 1
  %247 = add i8 %246, -48
  %248 = icmp ult i8 %247, 10
  br i1 %248, label %249, label %.loopexit145, !prof !37

249:                                              ; preds = %.preheader148
  %250 = call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %245, i64 10)
  %251 = extractvalue { i64, i1 } %250, 1
  br i1 %251, label %.loopexit145, label %252

252:                                              ; preds = %249
  %253 = extractvalue { i64, i1 } %250, 0
  %254 = zext i8 %247 to i64
  %255 = call { i64, i1 } @llvm.ssub.with.overflow.i64(i64 %253, i64 %254)
  %256 = extractvalue { i64, i1 } %255, 1
  br i1 %256, label %.loopexit145, label %257, !prof !35

257:                                              ; preds = %252
  %258 = extractvalue { i64, i1 } %255, 0
  %259 = getelementptr inbounds %Ts5UInt8V, ptr %244, i64 1
  %260 = icmp eq ptr %259, %223
  br i1 %260, label %.loopexit145.loopexit316.split.loop.exit, label %.preheader148

261:                                              ; preds = %59
  %262 = icmp eq i64 %47, 1
  br i1 %262, label %.loopexit145, label %265, !prof !35

263:                                              ; preds = %59
  %264 = icmp eq i64 %47, 0
  br i1 %264, label %.loopexit145, label %298, !prof !35

265:                                              ; preds = %261
  %266 = getelementptr inbounds %Ts5UInt8V, ptr %14, i64 %47
  %267 = add i8 %63, -48
  %268 = icmp ult i8 %267, 10
  br i1 %268, label %269, label %.loopexit145, !prof !37

269:                                              ; preds = %265
  %270 = zext i8 %267 to i64
  %271 = icmp eq i64 %47, 2
  br i1 %271, label %.loopexit145, label %272

272:                                              ; preds = %269
  %273 = add i8 %65, -48
  %274 = icmp ult i8 %273, 10
  br i1 %274, label %275, label %.loopexit145, !prof !37

275:                                              ; preds = %272
  %276 = mul nuw nsw i64 %270, 10
  %277 = zext i8 %273 to i64
  %278 = add nuw nsw i64 %276, %277
  %279 = icmp eq i64 %47, 3
  br i1 %279, label %.loopexit145, label %.preheader152.preheader

.preheader152.preheader:                          ; preds = %275
  %280 = getelementptr inbounds %Ts5UInt8V, ptr %14, i64 3
  br label %.preheader152

.preheader152:                                    ; preds = %.preheader152.preheader, %294
  %281 = phi ptr [ %296, %294 ], [ %280, %.preheader152.preheader ]
  %282 = phi i64 [ %295, %294 ], [ %278, %.preheader152.preheader ]
  %283 = load i8, ptr %281, align 1
  %284 = add i8 %283, -48
  %285 = icmp ult i8 %284, 10
  br i1 %285, label %286, label %.loopexit145, !prof !37

286:                                              ; preds = %.preheader152
  %287 = call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %282, i64 10)
  %288 = extractvalue { i64, i1 } %287, 1
  br i1 %288, label %.loopexit145, label %289

289:                                              ; preds = %286
  %290 = extractvalue { i64, i1 } %287, 0
  %291 = zext i8 %284 to i64
  %292 = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %290, i64 %291)
  %293 = extractvalue { i64, i1 } %292, 1
  br i1 %293, label %.loopexit145, label %294, !prof !35

294:                                              ; preds = %289
  %295 = extractvalue { i64, i1 } %292, 0
  %296 = getelementptr inbounds %Ts5UInt8V, ptr %281, i64 1
  %297 = icmp eq ptr %296, %266
  br i1 %297, label %.loopexit145.loopexit319.split.loop.exit, label %.preheader152

298:                                              ; preds = %263
  %299 = getelementptr inbounds %Ts5UInt8V, ptr %14, i64 %47
  %300 = add i8 %61, -48
  %301 = icmp ult i8 %300, 10
  br i1 %301, label %302, label %.loopexit145, !prof !37

302:                                              ; preds = %298
  %303 = zext i8 %300 to i64
  %304 = icmp eq i64 %47, 1
  br i1 %304, label %.loopexit145, label %305

305:                                              ; preds = %302
  %306 = add i8 %63, -48
  %307 = icmp ult i8 %306, 10
  br i1 %307, label %308, label %.loopexit145, !prof !37

308:                                              ; preds = %305
  %309 = mul nuw nsw i64 %303, 10
  %310 = zext i8 %306 to i64
  %311 = add nuw nsw i64 %309, %310
  %312 = icmp eq i64 %47, 2
  br i1 %312, label %.loopexit145, label %.preheader144.preheader

.preheader144.preheader:                          ; preds = %308
  %313 = getelementptr inbounds %Ts5UInt8V, ptr %14, i64 2
  br label %.preheader144

.preheader144:                                    ; preds = %.preheader144.preheader, %327
  %314 = phi ptr [ %329, %327 ], [ %313, %.preheader144.preheader ]
  %315 = phi i64 [ %328, %327 ], [ %311, %.preheader144.preheader ]
  %316 = load i8, ptr %314, align 1
  %317 = add i8 %316, -48
  %318 = icmp ult i8 %317, 10
  br i1 %318, label %319, label %.loopexit145, !prof !37

319:                                              ; preds = %.preheader144
  %320 = call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %315, i64 10)
  %321 = extractvalue { i64, i1 } %320, 1
  br i1 %321, label %.loopexit145, label %322

322:                                              ; preds = %319
  %323 = extractvalue { i64, i1 } %320, 0
  %324 = zext i8 %317 to i64
  %325 = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %323, i64 %324)
  %326 = extractvalue { i64, i1 } %325, 1
  br i1 %326, label %.loopexit145, label %327, !prof !35

327:                                              ; preds = %322
  %328 = extractvalue { i64, i1 } %325, 0
  %329 = getelementptr inbounds %Ts5UInt8V, ptr %314, i64 1
  %330 = icmp eq ptr %329, %299
  br i1 %330, label %.loopexit145.loopexit.split.loop.exit, label %.preheader144

331:                                              ; preds = %entry, %51, %97
  %storemerge = phi i64 [ 10, %51 ], [ %., %97 ], [ 10, %entry ]
  store i64 %storemerge, ptr @"$s4main1nSivp", align 8
  store i64 4, ptr @"$s4main8minDepthSivp", align 8
  %332 = call i64 @llvm.smax.i64(i64 %storemerge, i64 6)
  store i64 %332, ptr @"$s4main8maxDepthSivp", align 8
  %333 = add nuw i64 %332, 1
  %334 = call swiftcc i64 @"$s4main10createTreeyAA0C4NodeCSgSiF"(i64 %333)
  store i64 %334, ptr @"$s4main4treeAA8TreeNodeCSgvp", align 8
  %335 = inttoptr i64 %334 to ptr
  %336 = call swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr swiftself %335)
  store i64 %336, ptr @"$s4main5checkSivp", align 8
  %337 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCyypGMD") #22
  %338 = call noalias ptr @swift_allocObject(ptr %337, i64 64, i64 7) #6
  %339 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %338, i64 0, i32 1
  store <2 x i64> <i64 1, i64 2>, ptr %339, align 8
  %340 = getelementptr inbounds i8, ptr %338, i64 32
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %2)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3)
  store i64 0, ptr %3, align 8
  %._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %3, i64 0, i32 1
  store ptr inttoptr (i64 -2305843009213693952 to ptr), ptr %._object._object, align 8
  call swiftcc void @"$ss11_StringGutsV4growyySiF"(i64 35, ptr nocapture nonnull swiftself dereferenceable(16) %3)
  %341 = load ptr, ptr %._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3)
  call void @swift_bridgeObjectRelease(ptr %341) #6
  store i64 -3458764513820540906, ptr %2, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %2, i64 0, i32 1
  store ptr inttoptr (i64 or (i64 sub (i64 ptrtoint (ptr @".str.22.stretch tree of depth " to i64), i64 32), i64 -9223372036854775808) to ptr), ptr %._guts._object._object, align 8
  %342 = load i64, ptr @"$s4main8maxDepthSivp", align 8
  %343 = add i64 %342, 1
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4)
  store i64 %343, ptr %4, align 8
  %344 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %4, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %345 = extractvalue { i64, ptr } %344, 0
  %346 = extractvalue { i64, ptr } %344, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %345, ptr %346, ptr nocapture nonnull swiftself dereferenceable(16) %2)
  call void @swift_bridgeObjectRelease(ptr %346) #6
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4)
  call swiftcc void @"$sSS6appendyySSF"(i64 4209567563902033929, ptr nonnull inttoptr (i64 -1657324662872342496 to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %2)
  %347 = load i64, ptr @"$s4main5checkSivp", align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5)
  store i64 %347, ptr %5, align 8
  %348 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %5, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %349 = extractvalue { i64, ptr } %348, 0
  %350 = extractvalue { i64, ptr } %348, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %349, ptr %350, ptr nocapture nonnull swiftself dereferenceable(16) %2)
  call void @swift_bridgeObjectRelease(ptr %350) #6
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5)
  %351 = load i64, ptr %2, align 8
  %352 = load ptr, ptr %._guts._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %2)
  %353 = getelementptr inbounds i8, ptr %338, i64 56
  store ptr @"$sSSN", ptr %353, align 8
  store i64 %351, ptr %340, align 8
  %._guts12._object._object = getelementptr inbounds i8, ptr %338, i64 40
  store ptr %352, ptr %._guts12._object._object, align 8
  call swiftcc void @"$ss5print_9separator10terminatoryypd_S2StF"(ptr %338, i64 32, ptr nonnull inttoptr (i64 -2233785415175766016 to ptr), i64 10, ptr nonnull inttoptr (i64 -2233785415175766016 to ptr))
  call void @swift_release(ptr %338) #6
  %354 = load i64, ptr @"$s4main8maxDepthSivp", align 8
  %355 = call swiftcc i64 @"$s4main10createTreeyAA0C4NodeCSgSiF"(i64 %354)
  store i64 %355, ptr @"$s4main14longLivingTreeAA0D4NodeCSgvp", align 8
  %356 = call ptr @dispatch_group_create()
  store ptr %356, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  %357 = call swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueCMa"(i64 0) #21
  %358 = extractvalue %swift.metadata_response %357, 0
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6)
  store ptr @_swiftEmptyArrayStorage, ptr %6, align 8
  %359 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWL", ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMa", ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVs10SetAlgebraACMc")
  %360 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGMD") #22
  %361 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWL", ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %34, ptr noalias nocapture nonnull %6, ptr %360, ptr %361, ptr swiftself %31, ptr %31, ptr %359)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %29)
  %362 = load i32, ptr @"$sSo17OS_dispatch_queueC8DispatchE20AutoreleaseFrequencyO7inherityA2EmFWC", align 4
  %363 = getelementptr inbounds ptr, ptr %.valueWitnesses1, i64 13
  %364 = load ptr, ptr %363, align 8, !invariant.load !33
  call void %364(ptr noalias nonnull %24, i32 %362, ptr %21) #6
  %365 = call swiftcc ptr @"$sSo17OS_dispatch_queueC8DispatchE5label3qos10attributes20autoreleaseFrequency6targetABSS_AC0D3QoSVAbCE10AttributesVAbCE011AutoreleaseI0OABSgtcfC"(i64 128009175786834, ptr nonnull inttoptr (i64 -1873497444986126336 to ptr), ptr noalias nocapture nonnull %29, ptr noalias nocapture nonnull %34, ptr noalias nocapture nonnull %24, i64 0, ptr swiftself %358)
  store ptr %365, ptr @"$s4main2rqSo17OS_dispatch_queueCvp", align 8
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE10AttributesV10concurrentAEvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %34)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %29)
  call void %364(ptr noalias nonnull %24, i32 %362, ptr %21) #6
  %366 = call swiftcc ptr @"$sSo17OS_dispatch_queueC8DispatchE5label3qos10attributes20autoreleaseFrequency6targetABSS_AC0D3QoSVAbCE10AttributesVAbCE011AutoreleaseI0OABSgtcfC"(i64 125779919925079, ptr nonnull inttoptr (i64 -1873497444986126336 to ptr), ptr noalias nocapture nonnull %29, ptr noalias nocapture nonnull %34, ptr noalias nocapture nonnull %24, i64 0, ptr swiftself %358)
  store ptr %366, ptr @"$s4main5queueSo012OS_dispatch_B0Cvp", align 8
  %367 = load i64, ptr @"$s4main8maxDepthSivp", align 8
  %368 = load i64, ptr @"$s4main8minDepthSivp", align 8
  %369 = sub i64 %367, %368
  %370 = sdiv i64 %369, 2
  %371 = add nsw i64 %370, 1
  %372 = call swiftcc ptr @"$sSa9repeating5countSayxGx_SitcfCSS_Tgm5"(i64 0, ptr nonnull inttoptr (i64 -2305843009213693952 to ptr), i64 %371)
  store ptr %372, ptr @"$s4main7resultsSaySSGvp", align 8
  %373 = getelementptr inbounds { %objc_block, %swift.function }, ptr %12, i64 0, i32 1
  %.data28 = getelementptr inbounds { %objc_block, %swift.function }, ptr %12, i64 0, i32 1, i32 1
  %374 = getelementptr inbounds %objc_block, ptr %12, i64 0, i32 1
  %375 = getelementptr inbounds %objc_block, ptr %12, i64 0, i32 3
  %376 = getelementptr inbounds %objc_block, ptr %12, i64 0, i32 4
  %377 = getelementptr inbounds ptr, ptr %.valueWitnesses, i64 1
  %378 = getelementptr inbounds ptr, ptr %.valueWitnesses3, i64 1
  br label %379

379:                                              ; preds = %419, %331
  %380 = phi i1 [ false, %331 ], [ %420, %419 ]
  %381 = phi i64 [ %368, %331 ], [ %421, %419 ]
  %382 = phi i64 [ 0, %331 ], [ %422, %419 ]
  %383 = phi i1 [ false, %331 ], [ %423, %419 ]
  %384 = icmp slt i64 %381, %367
  br i1 %384, label %387, label %385

385:                                              ; preds = %379
  %386 = icmp ne i64 %381, %367
  %brmerge = select i1 %386, i1 true, i1 %380
  br i1 %brmerge, label %393, label %391

387:                                              ; preds = %379
  %388 = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %381, i64 2)
  %389 = extractvalue { i64, i1 } %388, 0
  %390 = extractvalue { i64, i1 } %388, 1
  %.55 = select i1 %390, i64 9223372036854775807, i64 %389
  %.56 = select i1 %390, i64 -9223372036854775808, i64 0
  %not. = xor i1 %390, true
  br label %419

391:                                              ; preds = %385
  %.not75 = xor i1 %383, true
  %392 = icmp eq i64 %382, -9223372036854775808
  %or.cond76 = select i1 %.not75, i1 %392, i1 false
  br i1 %or.cond76, label %393, label %419

393:                                              ; preds = %391, %385
  %394 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  call swiftcc void @"$sSo17OS_dispatch_groupC8DispatchE4waityyF"(ptr swiftself %394)
  %395 = load ptr, ptr @"$s4main2rqSo17OS_dispatch_queueCvp", align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %7)
  %396 = getelementptr inbounds { %objc_block, %swift.function }, ptr %7, i64 0, i32 1
  store ptr @"$sIg_Ieg_TR15$s4mainyyXEfU1_Tf3pf_n", ptr %396, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %7, i64 0, i32 1, i32 1
  store ptr null, ptr %.data, align 8
  store ptr @_NSConcreteStackBlock, ptr %7, align 8
  %397 = getelementptr inbounds %objc_block, ptr %7, i64 0, i32 1
  store <2 x i32> <i32 1107296256, i32 0>, ptr %397, align 8
  %398 = getelementptr inbounds %objc_block, ptr %7, i64 0, i32 3
  store ptr @"$sIeg_IyB_TR", ptr %398, align 8
  %399 = getelementptr inbounds %objc_block, ptr %7, i64 0, i32 4
  store ptr @block_descriptor, ptr %399, align 8
  %400 = call ptr @_Block_copy(ptr nonnull %7)
  %401 = load ptr, ptr %.data, align 8
  call void @swift_release(ptr %401) #6
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %7)
  call void @dispatch_sync(ptr %395, ptr %400)
  call void @_Block_release(ptr %400) #6
  %402 = call noalias ptr @swift_allocObject(ptr %337, i64 64, i64 7) #6
  %403 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %402, i64 0, i32 1
  store <2 x i64> <i64 1, i64 2>, ptr %403, align 8
  %404 = getelementptr inbounds i8, ptr %402, i64 32
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %8)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %9)
  store i64 0, ptr %9, align 8
  %._object18._object = getelementptr inbounds %Ts13_StringObjectV, ptr %9, i64 0, i32 1
  store ptr inttoptr (i64 -2305843009213693952 to ptr), ptr %._object18._object, align 8
  call swiftcc void @"$ss11_StringGutsV4growyySiF"(i64 38, ptr nocapture nonnull swiftself dereferenceable(16) %9)
  %405 = load ptr, ptr %._object18._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %9)
  call void @swift_bridgeObjectRelease(ptr %405) #6
  store i64 -3458764513820540903, ptr %8, align 8
  %._guts21._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %8, i64 0, i32 1
  store ptr inttoptr (i64 or (i64 sub (i64 ptrtoint (ptr @".str.25.long lived tree of depth " to i64), i64 32), i64 -9223372036854775808) to ptr), ptr %._guts21._object._object, align 8
  %406 = load i64, ptr @"$s4main8maxDepthSivp", align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %10)
  store i64 %406, ptr %10, align 8
  %407 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %10, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %408 = extractvalue { i64, ptr } %407, 0
  %409 = extractvalue { i64, ptr } %407, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %408, ptr %409, ptr nocapture nonnull swiftself dereferenceable(16) %8)
  call void @swift_bridgeObjectRelease(ptr %409) #6
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %10)
  call swiftcc void @"$sSS6appendyySSF"(i64 4209567563902033929, ptr nonnull inttoptr (i64 -1657324662872342496 to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %8)
  %410 = load i64, ptr @"$s4main14longLivingTreeAA0D4NodeCSgvp", align 8
  %411 = inttoptr i64 %410 to ptr
  %412 = call swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr swiftself %411)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %11)
  store i64 %412, ptr %11, align 8
  %413 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %11, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %414 = extractvalue { i64, ptr } %413, 0
  %415 = extractvalue { i64, ptr } %413, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %414, ptr %415, ptr nocapture nonnull swiftself dereferenceable(16) %8)
  call void @swift_bridgeObjectRelease(ptr %415) #6
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %11)
  %416 = load i64, ptr %8, align 8
  %417 = load ptr, ptr %._guts21._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %8)
  %418 = getelementptr inbounds i8, ptr %402, i64 56
  store ptr @"$sSSN", ptr %418, align 8
  store i64 %416, ptr %404, align 8
  %._guts25._object._object = getelementptr inbounds i8, ptr %402, i64 40
  store ptr %417, ptr %._guts25._object._object, align 8
  call swiftcc void @"$ss5print_9separator10terminatoryypd_S2StF"(ptr %402, i64 32, ptr nonnull inttoptr (i64 -2233785415175766016 to ptr), i64 10, ptr nonnull inttoptr (i64 -2233785415175766016 to ptr))
  call void @swift_release(ptr %402) #6
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %34)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %29)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %24)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %19)
  ret i32 0

419:                                              ; preds = %391, %387
  %420 = phi i1 [ %380, %387 ], [ true, %391 ]
  %421 = phi i64 [ %.55, %387 ], [ %367, %391 ]
  %422 = phi i64 [ %.56, %387 ], [ %382, %391 ]
  %423 = phi i1 [ %not., %387 ], [ %383, %391 ]
  %424 = load ptr, ptr @"$s4main5queueSo012OS_dispatch_B0Cvp", align 8
  %425 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  %426 = ptrtoint ptr %425 to i64
  %427 = call noalias ptr @swift_allocObject(ptr nonnull getelementptr inbounds (%swift.full_boxmetadata, ptr @metadata, i64 0, i32 2), i64 24, i64 7) #6
  %428 = getelementptr inbounds <{ %swift.refcounted, %TSi }>, ptr %427, i64 0, i32 1
  store i64 %381, ptr %428, align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %12)
  store ptr @"$s4mainyyYbcfU0_TA", ptr %373, align 8
  store ptr %427, ptr %.data28, align 8
  store ptr @_NSConcreteStackBlock, ptr %12, align 8
  store <2 x i32> <i32 1107296256, i32 0>, ptr %374, align 8
  store ptr @"$sIegh_IeyBh_TR", ptr %375, align 8
  store ptr @block_descriptor.3, ptr %376, align 8
  %429 = call ptr @_Block_copy(ptr nonnull %12)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %29)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %13)
  store ptr @_swiftEmptyArrayStorage, ptr %13, align 8
  %430 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL", ptr @"$s8Dispatch0A13WorkItemFlagsVMa", ptr @"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc")
  %431 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSay8Dispatch0A13WorkItemFlagsVGMD") #22
  %432 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL", ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %19, ptr noalias nocapture nonnull %13, ptr %431, ptr %432, ptr swiftself %16, ptr %16, ptr %430)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %13)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 %426, ptr noalias nocapture nonnull %29, ptr noalias nocapture nonnull %19, ptr %429, ptr swiftself %424)
  %433 = load ptr, ptr %377, align 8, !invariant.load !33
  call void %433(ptr noalias nonnull %19, ptr %16) #6
  %434 = load ptr, ptr %378, align 8, !invariant.load !33
  call void %434(ptr noalias nonnull %29, ptr %26) #6
  call void @_Block_release(ptr %429) #6
  %435 = load ptr, ptr %.data28, align 8
  call void @swift_release(ptr %435) #6
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %12)
  br label %379
}

declare swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64) local_unnamed_addr #0

; Function Attrs: argmemonly mustprogress nocallback nofree nosync nounwind willreturn
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #1

declare swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE20AutoreleaseFrequencyOMa"(i64) local_unnamed_addr #0

declare swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64) local_unnamed_addr #0

declare swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMa"(i64) local_unnamed_addr #0

declare swiftcc i32 @"$ss11CommandLineO4argcs5Int32VvgZ"() local_unnamed_addr #0

; Function Attrs: mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64) #2

; Function Attrs: nounwind
define hidden swiftcc i64 @"$s4main10createTreeyAA0C4NodeCSgSiF"(i64 %0) local_unnamed_addr #3 {
entry:
  %1 = icmp sgt i64 %0, 0
  br i1 %1, label %2, label %11

2:                                                ; preds = %entry
  %3 = add nsw i64 %0, -1
  %4 = tail call swiftcc i64 @"$s4main10createTreeyAA0C4NodeCSgSiF"(i64 %3)
  %5 = tail call swiftcc i64 @"$s4main10createTreeyAA0C4NodeCSgSiF"(i64 %3)
  %6 = tail call swiftcc %swift.metadata_response @"$s4main8TreeNodeCMa"(i64 poison) #21
  %7 = extractvalue %swift.metadata_response %6, 0
  %8 = tail call noalias ptr @swift_allocObject(ptr %7, i64 32, i64 7) #6
  %9 = getelementptr inbounds %T4main8TreeNodeC, ptr %8, i64 0, i32 1
  %10 = getelementptr inbounds %T4main8TreeNodeC, ptr %8, i64 0, i32 2
  store i64 %4, ptr %9, align 8
  store i64 %5, ptr %10, align 8
  br label %16

11:                                               ; preds = %entry
  %12 = tail call swiftcc %swift.metadata_response @"$s4main8TreeNodeCMa"(i64 poison) #21
  %13 = extractvalue %swift.metadata_response %12, 0
  %14 = tail call noalias ptr @swift_allocObject(ptr %13, i64 32, i64 7) #6
  %15 = getelementptr inbounds %T4main8TreeNodeC, ptr %14, i64 0, i32 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %15, i8 0, i64 16, i1 false)
  br label %16

16:                                               ; preds = %2, %11
  %.in = phi ptr [ %14, %11 ], [ %8, %2 ]
  %17 = ptrtoint ptr %.in to i64
  ret i64 %17
}

define linkonce_odr hidden swiftcc ptr @"$sSa9repeating5countSayxGx_SitcfCSS_Tgm5"(i64 %0, ptr %1, i64 %2) local_unnamed_addr #0 {
entry:
  %3 = icmp sgt i64 %2, 0
  br i1 %3, label %7, label %4

4:                                                ; preds = %entry
  %5 = icmp eq i64 %2, 0
  br i1 %5, label %6, label %.thread

.thread:                                          ; preds = %4
  store i64 %0, ptr getelementptr inbounds (%struct._SwiftEmptyArrayStorage, ptr @_swiftEmptyArrayStorage, i64 1, i32 0, i32 0), align 8
  store ptr %1, ptr getelementptr inbounds (%struct._SwiftEmptyArrayStorage, ptr @_swiftEmptyArrayStorage, i64 1, i32 0, i32 1, i32 0), align 8
  store i64 %0, ptr getelementptr inbounds (%struct._SwiftEmptyArrayStorage, ptr @_swiftEmptyArrayStorage, i64 1, i32 1, i32 0), align 8
  store ptr %1, ptr getelementptr inbounds (%struct._SwiftEmptyArrayStorage, ptr @_swiftEmptyArrayStorage, i64 1, i32 1, i32 1), align 8
  br label %.lr.ph.preheader

6:                                                ; preds = %4
  tail call void @swift_bridgeObjectRelease(ptr %1) #6
  br label %17

7:                                                ; preds = %entry
  %8 = tail call swiftcc ptr @"$sSa28_allocateBufferUninitialized15minimumCapacitys06_ArrayB0VyxGSi_tFZ"(i64 %2, ptr nonnull @"$sSSN")
  %9 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %8, i64 0, i32 1
  store i64 %2, ptr %9, align 8
  %10 = getelementptr inbounds i8, ptr %8, i64 32
  store i64 %0, ptr %10, align 8
  %._guts._object._object = getelementptr inbounds i8, ptr %8, i64 40
  store ptr %1, ptr %._guts._object._object, align 8
  %11 = icmp eq i64 %2, 1
  br i1 %11, label %17, label %12

12:                                               ; preds = %7
  %13 = getelementptr inbounds i8, ptr %8, i64 48
  store i64 %0, ptr %13, align 8
  %._guts1._object._object5 = getelementptr inbounds i8, ptr %8, i64 56
  store ptr %1, ptr %._guts1._object._object5, align 8
  %14 = icmp eq i64 %2, 2
  br i1 %14, label %._crit_edge, label %.lr.ph.preheader

.lr.ph.preheader:                                 ; preds = %.thread, %12
  %15 = phi ptr [ getelementptr inbounds (%struct._SwiftEmptyArrayStorage, ptr @_swiftEmptyArrayStorage, i64 1, i32 1, i32 0), %.thread ], [ %13, %12 ]
  %16 = phi ptr [ @_swiftEmptyArrayStorage, %.thread ], [ %8, %12 ]
  br label %.lr.ph

17:                                               ; preds = %7, %6, %._crit_edge
  %18 = phi ptr [ %8, %7 ], [ @_swiftEmptyArrayStorage, %6 ], [ %25, %._crit_edge ]
  ret ptr %18

.lr.ph:                                           ; preds = %.lr.ph.preheader, %.lr.ph
  %19 = phi i64 [ %22, %.lr.ph ], [ 2, %.lr.ph.preheader ]
  %20 = phi ptr [ %21, %.lr.ph ], [ %15, %.lr.ph.preheader ]
  %21 = getelementptr inbounds %TSS, ptr %20, i64 1
  %22 = add i64 %19, 1
  store i64 %0, ptr %21, align 8
  %._guts1._object._object = getelementptr inbounds %TSS, ptr %20, i64 1, i32 0, i32 0, i32 1
  store ptr %1, ptr %._guts1._object._object, align 8
  %23 = icmp eq i64 %22, %2
  %24 = tail call ptr @swift_bridgeObjectRetain(ptr returned %1) #6
  br i1 %23, label %._crit_edge, label %.lr.ph

._crit_edge:                                      ; preds = %.lr.ph, %12
  %25 = phi ptr [ %8, %12 ], [ %16, %.lr.ph ]
  %26 = tail call ptr @swift_bridgeObjectRetain(ptr returned %1) #6
  br label %17
}

define internal swiftcc void @"$s4mainyyYbcfU0_"(i64 %0) unnamed_addr #0 {
entry:
  %1 = alloca { %objc_block, %swift.function }, align 8
  %2 = alloca %TSa, align 8
  %3 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64 0) #21
  %4 = extractvalue %swift.metadata_response %3, 0
  %5 = getelementptr inbounds ptr, ptr %4, i64 -1
  %.valueWitnesses = load ptr, ptr %5, align 8, !invariant.load !33, !dereferenceable !34
  %6 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses, i64 0, i32 8
  %size = load i64, ptr %6, align 8, !invariant.load !33
  %7 = alloca i8, i64 %size, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %7)
  %8 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64 0) #21
  %9 = extractvalue %swift.metadata_response %8, 0
  %10 = getelementptr inbounds ptr, ptr %9, i64 -1
  %.valueWitnesses1 = load ptr, ptr %10, align 8, !invariant.load !33, !dereferenceable !34
  %11 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses1, i64 0, i32 8
  %size2 = load i64, ptr %11, align 8, !invariant.load !33
  %12 = alloca i8, i64 %size2, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %12)
  %13 = load i64, ptr @"$s4main8minDepthSivp", align 8
  %14 = load i64, ptr @"$s4main8maxDepthSivp", align 8
  %15 = sub i64 %14, %0
  %16 = add i64 %15, %13
  %17 = icmp ugt i64 %16, 64
  %18 = icmp ult i64 %16, 64
  %19 = shl nuw i64 1, %16
  %spec.select = select i1 %18, i64 %19, i64 0
  %20 = select i1 %17, i64 0, i64 %spec.select, !prof !38
  %21 = icmp eq i64 %20, 1
  %22 = tail call swiftcc i64 @"$s4main10createTreeyAA0C4NodeCSgSiF"(i64 %0)
  %23 = inttoptr i64 %22 to ptr
  %24 = tail call swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr swiftself %23)
  tail call void @swift_release(ptr %23) #6
  br i1 %21, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %entry, %.preheader
  %25 = phi i64 [ %27, %.preheader ], [ 1, %entry ]
  %26 = phi i64 [ %32, %.preheader ], [ %24, %entry ]
  %27 = add i64 %25, 1
  %28 = icmp eq i64 %27, %20
  %29 = tail call swiftcc i64 @"$s4main10createTreeyAA0C4NodeCSgSiF"(i64 %0)
  %30 = inttoptr i64 %29 to ptr
  %31 = tail call swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr swiftself %30)
  tail call void @swift_release(ptr %30) #6
  %32 = add i64 %31, %26
  br i1 %28, label %.loopexit, label %.preheader

.loopexit:                                        ; preds = %.preheader, %entry
  %33 = phi i64 [ %24, %entry ], [ %32, %.preheader ]
  %34 = sub i64 %0, %13
  %35 = sdiv i64 %34, 2
  %36 = load ptr, ptr @"$s4main2rqSo17OS_dispatch_queueCvp", align 8
  %37 = tail call noalias ptr @swift_allocObject(ptr nonnull getelementptr inbounds (%swift.full_boxmetadata, ptr @metadata.6, i64 0, i32 2), i64 48, i64 7) #6
  %38 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi, %TSi, %TSi }>, ptr %37, i64 0, i32 1
  store i64 %35, ptr %38, align 8
  %39 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi, %TSi, %TSi }>, ptr %37, i64 0, i32 2
  store i64 %20, ptr %39, align 8
  %40 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi, %TSi, %TSi }>, ptr %37, i64 0, i32 3
  store i64 %0, ptr %40, align 8
  %41 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi, %TSi, %TSi }>, ptr %37, i64 0, i32 4
  store i64 %33, ptr %41, align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %1)
  %42 = getelementptr inbounds { %objc_block, %swift.function }, ptr %1, i64 0, i32 1
  store ptr @"$s4mainyyYbcfU0_yyYbcfU_Tf2nnni_nTA", ptr %42, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %1, i64 0, i32 1, i32 1
  store ptr %37, ptr %.data, align 8
  store ptr @_NSConcreteStackBlock, ptr %1, align 8
  %43 = getelementptr inbounds %objc_block, ptr %1, i64 0, i32 1
  store <2 x i32> <i32 1107296256, i32 0>, ptr %43, align 8
  %44 = getelementptr inbounds %objc_block, ptr %1, i64 0, i32 3
  store ptr @"$sIegh_IeyBh_TR", ptr %44, align 8
  %45 = getelementptr inbounds %objc_block, ptr %1, i64 0, i32 4
  store ptr @block_descriptor.9, ptr %45, align 8
  %46 = call ptr @_Block_copy(ptr nonnull %1)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %12)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2)
  store ptr @_swiftEmptyArrayStorage, ptr %2, align 8
  %47 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL", ptr @"$s8Dispatch0A13WorkItemFlagsVMa", ptr @"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc")
  %48 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSay8Dispatch0A13WorkItemFlagsVGMD") #22
  %49 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL", ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %7, ptr noalias nocapture nonnull %2, ptr %48, ptr %49, ptr swiftself %4, ptr %4, ptr %47)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %12, ptr noalias nocapture nonnull %7, ptr %46, ptr swiftself %36)
  %50 = getelementptr inbounds ptr, ptr %.valueWitnesses, i64 1
  %51 = load ptr, ptr %50, align 8, !invariant.load !33
  call void %51(ptr noalias nonnull %7, ptr %4) #6
  %52 = getelementptr inbounds ptr, ptr %.valueWitnesses1, i64 1
  %53 = load ptr, ptr %52, align 8, !invariant.load !33
  call void %53(ptr noalias nonnull %12, ptr %9) #6
  call void @_Block_release(ptr %46) #6
  %54 = load ptr, ptr %.data, align 8
  call void @swift_release(ptr %54) #6
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %1)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %12)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %7)
  ret void
}

define linkonce_odr hidden void @"$sIegh_IeyBh_TR"(ptr %0) #0 {
entry:
  %1 = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1
  %2 = load ptr, ptr %1, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1, i32 1
  %3 = load ptr, ptr %.data, align 8
  %4 = tail call ptr @swift_retain(ptr returned %3) #23
  tail call swiftcc void %2(ptr swiftself %3) #24
  tail call void @swift_release(ptr %3) #6
  ret void
}

define internal swiftcc void @"$s4mainyyXEfU1_"() unnamed_addr #0 {
entry:
  %0 = load ptr, ptr @"$s4main7resultsSaySSGvp", align 8
  %1 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8, !range !39
  %3 = icmp eq i64 %2, 0
  br i1 %3, label %7, label %4

4:                                                ; preds = %entry
  %5 = getelementptr inbounds i8, ptr %0, i64 32
  %6 = tail call ptr @swift_bridgeObjectRetain(ptr nonnull returned %0) #6
  br label %8

7:                                                ; preds = %entry, %21
  ret void

8:                                                ; preds = %8, %4
  %9 = phi i64 [ 0, %4 ], [ %13, %8 ]
  %10 = getelementptr inbounds %TSS, ptr %5, i64 %9
  %11 = load i64, ptr %10, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %10, i64 0, i32 1
  %12 = load ptr, ptr %._guts._object._object, align 8
  %13 = add nuw nsw i64 %9, 1
  %14 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCyypGMD") #22
  %15 = tail call noalias ptr @swift_allocObject(ptr %14, i64 64, i64 7) #6
  %16 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %15, i64 0, i32 1
  store <2 x i64> <i64 1, i64 2>, ptr %16, align 8
  %17 = getelementptr inbounds i8, ptr %15, i64 32
  %18 = getelementptr inbounds i8, ptr %15, i64 56
  store ptr @"$sSSN", ptr %18, align 8
  store i64 %11, ptr %17, align 8
  %._guts2._object._object = getelementptr inbounds i8, ptr %15, i64 40
  store ptr %12, ptr %._guts2._object._object, align 8
  %19 = call ptr @swift_bridgeObjectRetain_n(ptr %12, i32 2)
  tail call swiftcc void @"$ss5print_9separator10terminatoryypd_S2StF"(ptr %15, i64 32, ptr nonnull inttoptr (i64 -2233785415175766016 to ptr), i64 10, ptr nonnull inttoptr (i64 -2233785415175766016 to ptr))
  tail call void @swift_bridgeObjectRelease(ptr %12) #6
  tail call void @swift_release(ptr %15) #6
  %20 = icmp eq i64 %13, %2
  br i1 %20, label %21, label %8

21:                                               ; preds = %8
  tail call void @swift_bridgeObjectRelease(ptr nonnull %0) #6
  br label %7
}

define linkonce_odr hidden swiftcc void @"$sIg_Ieg_TR15$s4mainyyXEfU1_Tf3pf_n"() #0 {
entry:
  tail call swiftcc void @"$s4mainyyXEfU1_"() #24
  ret void
}

define linkonce_odr hidden void @"$sIeg_IyB_TR"(ptr %0) #0 {
entry:
  %1 = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1
  %2 = load ptr, ptr %1, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1, i32 1
  %3 = load ptr, ptr %.data, align 8
  tail call swiftcc void %2(ptr swiftself %3) #24
  ret void
}

; Function Attrs: nounwind
define hidden swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr nocapture readonly swiftself %0) #3 {
entry:
  %1 = getelementptr inbounds %T4main8TreeNodeC, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8
  %3 = icmp eq i64 %2, 0
  br i1 %3, label %common.ret1, label %4

common.ret1:                                      ; preds = %entry, %4
  %common.ret1.op = phi i64 [ %14, %4 ], [ 1, %entry ]
  ret i64 %common.ret1.op

4:                                                ; preds = %entry
  %5 = inttoptr i64 %2 to ptr
  %6 = tail call ptr @swift_retain(ptr nonnull returned %5) #23
  %7 = tail call swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr nonnull swiftself %5)
  tail call void @swift_release(ptr nonnull %5) #6
  %8 = getelementptr inbounds %T4main8TreeNodeC, ptr %0, i64 0, i32 2
  %9 = load i64, ptr %8, align 8
  %10 = inttoptr i64 %9 to ptr
  %11 = tail call ptr @swift_retain(ptr returned %10) #23
  %12 = tail call swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr swiftself %10)
  tail call void @swift_release(ptr %10) #6
  %13 = add i64 %7, 1
  %14 = add i64 %13, %12
  br label %common.ret1
}

; Function Attrs: mustprogress nofree noinline nounwind readonly willreturn
define linkonce_odr hidden ptr @__swift_instantiateConcreteTypeFromMangledName(ptr %0) local_unnamed_addr #4 {
entry:
  %1 = load atomic i64, ptr %0 monotonic, align 8
  %2 = icmp slt i64 %1, 0
  br i1 %2, label %6, label %3, !prof !35

3:                                                ; preds = %6, %entry
  %4 = phi i64 [ %1, %entry ], [ %14, %6 ]
  %5 = inttoptr i64 %4 to ptr
  ret ptr %5

6:                                                ; preds = %entry
  %7 = ashr i64 %1, 32
  %8 = sub nsw i64 0, %7
  %sext = shl i64 %1, 32
  %9 = ashr exact i64 %sext, 32
  %10 = ptrtoint ptr %0 to i64
  %11 = add i64 %9, %10
  %12 = inttoptr i64 %11 to ptr
  %13 = tail call swiftcc ptr @swift_getTypeByMangledNameInContext2(ptr %12, i64 %8, ptr null, ptr null) #25
  %14 = ptrtoint ptr %13 to i64
  store atomic i64 %14, ptr %0 monotonic, align 8
  br label %3
}

; Function Attrs: argmemonly nounwind
declare swiftcc ptr @swift_getTypeByMangledNameInContext2(ptr, i64, ptr, ptr) local_unnamed_addr #5

; Function Attrs: nounwind
declare ptr @swift_allocObject(ptr, i64, i64) local_unnamed_addr #6

; Function Attrs: argmemonly mustprogress nocallback nofree nosync nounwind willreturn
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #1

declare swiftcc void @"$ss11_StringGutsV4growyySiF"(i64, ptr nocapture swiftself dereferenceable(16)) local_unnamed_addr #0

; Function Attrs: nounwind
declare void @swift_bridgeObjectRelease(ptr) local_unnamed_addr #6

declare swiftcc void @"$sSS6appendyySSF"(i64, ptr, ptr nocapture swiftself dereferenceable(16)) local_unnamed_addr #0

declare swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture swiftself, ptr, ptr) local_unnamed_addr #0

declare swiftcc void @"$ss5print_9separator10terminatoryypd_S2StF"(ptr, i64, ptr, i64, ptr) local_unnamed_addr #0

; Function Attrs: nounwind
declare void @swift_release(ptr) local_unnamed_addr #6

; Function Attrs: nounwind optsize
declare noalias ptr @dispatch_group_create() local_unnamed_addr #7

; Function Attrs: nofree noinline nosync nounwind readnone
define linkonce_odr hidden swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueCMa"(i64 %0) local_unnamed_addr #8 {
entry:
  %1 = load ptr, ptr @"$sSo17OS_dispatch_queueCML", align 8
  %2 = icmp eq ptr %1, null
  br i1 %2, label %cacheIsNull, label %cont

cacheIsNull:                                      ; preds = %entry
  %3 = load ptr, ptr @"OBJC_CLASS_REF_$_OS_dispatch_queue", align 8
  %4 = tail call ptr @objc_opt_self(ptr %3) #6
  %5 = tail call ptr @swift_getObjCClassMetadata(ptr %4) #26
  store atomic ptr %5, ptr @"$sSo17OS_dispatch_queueCML" release, align 8
  br label %cont

cont:                                             ; preds = %cacheIsNull, %entry
  %6 = phi ptr [ %1, %entry ], [ %5, %cacheIsNull ]
  %7 = insertvalue %swift.metadata_response undef, ptr %6, 0
  %8 = insertvalue %swift.metadata_response %7, i64 0, 1
  ret %swift.metadata_response %8
}

; Function Attrs: nounwind
declare ptr @objc_opt_self(ptr) local_unnamed_addr #6

; Function Attrs: mustprogress nofree nosync nounwind readnone willreturn
declare ptr @swift_getObjCClassMetadata(ptr) local_unnamed_addr #9

declare swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture sret(%swift.opaque), ptr noalias nocapture, ptr, ptr, ptr swiftself, ptr, ptr) local_unnamed_addr #0

; Function Attrs: nofree nounwind readonly
declare ptr @swift_getWitnessTable(ptr, ptr, ptr) local_unnamed_addr #10

; Function Attrs: mustprogress nofree noinline nounwind readonly willreturn
define linkonce_odr hidden ptr @__swift_instantiateConcreteTypeFromMangledNameAbstract(ptr %0) local_unnamed_addr #4 {
entry:
  %1 = load atomic i64, ptr %0 monotonic, align 8
  %2 = icmp slt i64 %1, 0
  br i1 %2, label %6, label %3, !prof !35

3:                                                ; preds = %6, %entry
  %4 = phi i64 [ %1, %entry ], [ %14, %6 ]
  %5 = inttoptr i64 %4 to ptr
  ret ptr %5

6:                                                ; preds = %entry
  %7 = ashr i64 %1, 32
  %8 = sub nsw i64 0, %7
  %sext = shl i64 %1, 32
  %9 = ashr exact i64 %sext, 32
  %10 = ptrtoint ptr %0 to i64
  %11 = add i64 %9, %10
  %12 = inttoptr i64 %11 to ptr
  %13 = tail call swiftcc ptr @swift_getTypeByMangledNameInContextInMetadataState2(i64 255, ptr %12, i64 %8, ptr null, ptr null) #25
  %14 = ptrtoint ptr %13 to i64
  store atomic i64 %14, ptr %0 monotonic, align 8
  br label %3
}

; Function Attrs: argmemonly nounwind
declare swiftcc ptr @swift_getTypeByMangledNameInContextInMetadataState2(i64, ptr, i64, ptr, ptr) local_unnamed_addr #5

declare swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture sret(%swift.opaque)) local_unnamed_addr #0

declare swiftcc ptr @"$sSo17OS_dispatch_queueC8DispatchE5label3qos10attributes20autoreleaseFrequency6targetABSS_AC0D3QoSVAbCE10AttributesVAbCE011AutoreleaseI0OABSgtcfC"(i64, ptr, ptr noalias nocapture, ptr noalias nocapture, ptr noalias nocapture, i64, ptr swiftself) local_unnamed_addr #0

declare swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE10AttributesV10concurrentAEvgZ"(ptr noalias nocapture sret(%swift.opaque)) local_unnamed_addr #0

; Function Attrs: mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64) #2

declare swiftcc void @"$sSo17OS_dispatch_groupC8DispatchE4waityyF"(ptr swiftself) local_unnamed_addr #0

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #11 {
entry:
  %2 = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1
  %3 = getelementptr inbounds { %objc_block, %swift.function }, ptr %1, i64 0, i32 1
  %.fn.load = load ptr, ptr %3, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %1, i64 0, i32 1, i32 1
  %4 = load ptr, ptr %.data, align 8
  store ptr %.fn.load, ptr %2, align 8
  %.data2 = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1, i32 1
  store ptr %4, ptr %.data2, align 8
  %5 = tail call ptr @swift_retain(ptr returned %4) #23
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
declare ptr @swift_retain(ptr returned) local_unnamed_addr #12

; Function Attrs: nounwind
define internal void @block_destroy_helper(ptr nocapture readonly %0) #3 {
entry:
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1, i32 1
  %1 = load ptr, ptr %.data, align 8
  tail call void @swift_release(ptr %1) #6
  ret void
}

declare ptr @_Block_copy(ptr) local_unnamed_addr

; Function Attrs: nounwind optsize
declare void @dispatch_sync(ptr noundef, ptr nocapture noundef) local_unnamed_addr #7

; Function Attrs: nounwind
declare void @_Block_release(ptr) local_unnamed_addr #6

; Function Attrs: nounwind
define private swiftcc void @objectdestroy(ptr swiftself %0) #3 {
entry:
  tail call void @swift_deallocObject(ptr %0, i64 24, i64 7) #6
  ret void
}

; Function Attrs: nounwind
declare void @swift_deallocObject(ptr, i64, i64) local_unnamed_addr #6

define internal swiftcc void @"$s4mainyyYbcfU0_TA"(ptr nocapture readonly swiftself %0) #0 {
entry:
  %1 = getelementptr inbounds <{ %swift.refcounted, %TSi }>, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8
  tail call swiftcc void @"$s4mainyyYbcfU0_"(i64 %2)
  ret void
}

; Function Attrs: nofree noinline nosync nounwind readnone
define internal ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr %0, ptr %1, ptr %2) local_unnamed_addr #8 {
entry:
  %3 = load ptr, ptr %0, align 8
  %4 = icmp eq ptr %3, null
  br i1 %4, label %cacheIsNull, label %cont

cacheIsNull:                                      ; preds = %entry
  %5 = tail call swiftcc %swift.metadata_response %1(i64 255) #21
  %6 = extractvalue %swift.metadata_response %5, 0
  %7 = tail call ptr @swift_getWitnessTable(ptr nonnull %2, ptr %6, ptr undef) #22
  store atomic ptr %7, ptr %0 release, align 8
  br label %cont

cont:                                             ; preds = %cacheIsNull, %entry
  %8 = phi ptr [ %3, %entry ], [ %7, %cacheIsNull ]
  ret ptr %8
}

; Function Attrs: nofree noinline nosync nounwind readnone
define internal ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr %0, ptr %1) local_unnamed_addr #8 {
entry:
  %2 = load ptr, ptr %0, align 8
  %3 = icmp eq ptr %2, null
  br i1 %3, label %cacheIsNull, label %cont

cacheIsNull:                                      ; preds = %entry
  %4 = tail call ptr @__swift_instantiateConcreteTypeFromMangledNameAbstract(ptr nonnull %1) #22
  %5 = tail call ptr @swift_getWitnessTable(ptr nonnull @"$sSayxGSTsMc", ptr %4, ptr undef) #22
  store atomic ptr %5, ptr %0 release, align 8
  br label %cont

cont:                                             ; preds = %cacheIsNull, %entry
  %6 = phi ptr [ %2, %entry ], [ %5, %cacheIsNull ]
  ret ptr %6
}

declare swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64, ptr noalias nocapture, ptr noalias nocapture, ptr, ptr swiftself) local_unnamed_addr #0

declare swiftcc ptr @"$ss11CommandLineO9argumentsSaySSGvgZ"() local_unnamed_addr #0

; Function Attrs: nounwind
declare ptr @swift_bridgeObjectRetain(ptr returned) local_unnamed_addr #6

; Function Attrs: nounwind
define hidden swiftcc void @"$s4main8TreeNodeCfD"(ptr swiftself %0) #3 {
entry:
  %1 = getelementptr inbounds %T4main8TreeNodeC, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8
  %3 = inttoptr i64 %2 to ptr
  tail call void @swift_release(ptr %3) #6
  %4 = getelementptr inbounds %T4main8TreeNodeC, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8
  %6 = inttoptr i64 %5 to ptr
  tail call void @swift_release(ptr %6) #6
  tail call void @swift_deallocClassInstance(ptr %0, i64 32, i64 7) #6
  ret void
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc { i64, i8 } @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFSS_SiTg5"(i64 %0, ptr %1, i64 %2) local_unnamed_addr #13 {
entry:
  %3 = alloca %TSS, align 8
  %4 = alloca <{ %Ts6UInt64V, %Ts6UInt64V }>, align 8
  %swifterror = alloca swifterror ptr, align 8
  store ptr null, ptr %swifterror, align 8
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3)
  store i64 %0, ptr %3, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %3, i64 0, i32 1
  store ptr %1, ptr %._guts._object._object, align 8
  %5 = tail call ptr @swift_bridgeObjectRetain(ptr returned %1) #6
  %6 = call swiftcc { i64, ptr } @"$sSSySSxcs25LosslessStringConvertibleRzSTRzSJ7ElementSTRtzlufC"(ptr noalias nocapture nonnull %3, ptr nonnull @"$sSSN", ptr nonnull @"$sSSs25LosslessStringConvertiblesWP", ptr nonnull @"$sSSSTsWP")
  %7 = extractvalue { i64, ptr } %6, 0
  %8 = extractvalue { i64, ptr } %6, 1
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3)
  %9 = ptrtoint ptr %8 to i64
  %10 = and i64 %9, 1152921504606846976
  %11 = icmp eq i64 %10, 0
  br i1 %11, label %16, label %12, !prof !36

12:                                               ; preds = %entry
  %13 = tail call swiftcc { i64, ptr } @"$sSS8_copyingyS2SFZ"(i64 %7, ptr %8)
  tail call void @swift_bridgeObjectRelease(ptr %8) #6
  %14 = extractvalue { i64, ptr } %13, 0
  %15 = extractvalue { i64, ptr } %13, 1
  %.pre = ptrtoint ptr %15 to i64
  br label %16

16:                                               ; preds = %entry, %12
  %.pre-phi = phi i64 [ %9, %entry ], [ %.pre, %12 ]
  %17 = phi ptr [ %8, %entry ], [ %15, %12 ]
  %18 = phi i64 [ %7, %entry ], [ %14, %12 ]
  %19 = and i64 %.pre-phi, 2305843009213693952
  %20 = icmp eq i64 %19, 0
  br i1 %20, label %21, label %24

21:                                               ; preds = %16
  %22 = and i64 %18, 1152921504606846976
  %23 = icmp eq i64 %22, 0
  br i1 %23, label %31, label %35, !prof !35

24:                                               ; preds = %16
  %25 = lshr i64 %.pre-phi, 56
  %26 = and i64 %25, 15
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %4)
  %.elt2 = getelementptr inbounds <{ %Ts6UInt64V, %Ts6UInt64V }>, ptr %4, i64 0, i32 1
  %27 = and i64 %.pre-phi, 72057594037927935
  store i64 %18, ptr %4, align 8
  store i64 %27, ptr %.elt2, align 8
  %28 = ptrtoint ptr %4 to i64
  %29 = call swiftcc { i64, i8 } @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFADSRys5UInt8VGXEfU_SS_SiTg5"(i64 %28, i64 %26, i64 %2, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %swifterror)
  %30 = load ptr, ptr %swifterror, align 8
  %.not = icmp eq ptr %30, null
  call void @llvm.assume(i1 %.not)
  call void @swift_bridgeObjectRelease(ptr %17) #6
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %4)
  br label %44

31:                                               ; preds = %21
  %32 = tail call swiftcc { i64, i64 } @"$ss13_StringObjectV10sharedUTF8SRys5UInt8VGvg"(i64 %18, ptr %17)
  %33 = extractvalue { i64, i64 } %32, 0
  %34 = extractvalue { i64, i64 } %32, 1
  br label %39

35:                                               ; preds = %21
  %36 = and i64 %.pre-phi, 1152921504606846975
  %37 = add nuw nsw i64 %36, 32
  %38 = and i64 %18, 281474976710655
  br label %39

39:                                               ; preds = %31, %35
  %40 = phi i64 [ %37, %35 ], [ %33, %31 ]
  %41 = phi i64 [ %38, %35 ], [ %34, %31 ]
  %42 = call swiftcc { i64, i8 } @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFADSRys5UInt8VGXEfU_SS_SiTg5"(i64 %40, i64 %41, i64 %2, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %swifterror)
  %43 = load ptr, ptr %swifterror, align 8
  %.not4 = icmp eq ptr %43, null
  tail call void @llvm.assume(i1 %.not4)
  tail call void @swift_bridgeObjectRelease(ptr %17) #6
  br label %44

44:                                               ; preds = %39, %24
  %.pn = phi { i64, i8 } [ %29, %24 ], [ %42, %39 ]
  %.in.in = extractvalue { i64, i8 } %.pn, 1
  %.in = and i8 %.in.in, 1
  %45 = insertvalue { i64, i8 } %.pn, i8 %.in, 1
  ret { i64, i8 } %45
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #2

; Function Attrs: noinline
declare swiftcc { i64, i64 } @"$ss13_StringObjectV10sharedUTF8SRys5UInt8VGvg"(i64, ptr) local_unnamed_addr #13

; Function Attrs: nofree noinline nosync nounwind readnone
define hidden swiftcc %swift.metadata_response @"$s4main8TreeNodeCMa"(i64 %0) #8 {
entry:
  %1 = tail call ptr @objc_opt_self(ptr nonnull getelementptr inbounds (<{ ptr, ptr, ptr, i64, ptr, ptr, ptr, ptr, i32, i32, i32, i16, i16, i32, i32, ptr, ptr, i64, i64, ptr, ptr, ptr, ptr, ptr, ptr, ptr, ptr }>, ptr @"$s4main8TreeNodeCMf", i64 0, i32 3)) #6
  %2 = insertvalue %swift.metadata_response undef, ptr %1, 0
  %3 = insertvalue %swift.metadata_response %2, i64 0, 1
  ret %swift.metadata_response %3
}

; Function Attrs: nounwind
declare void @swift_deletedMethodError() #6

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftFoundation"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftObjectiveC"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftDarwin"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftCoreFoundation"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftDispatch"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftXPC"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftIOKit"()

; Function Attrs: nounwind
declare void @swift_deallocClassInstance(ptr, i64, i64) local_unnamed_addr #6

declare swiftcc { i64, ptr } @"$sSSySSxcs25LosslessStringConvertibleRzSTRzSJ7ElementSTRtzlufC"(ptr noalias nocapture, ptr, ptr, ptr) local_unnamed_addr #0

define linkonce_odr hidden swiftcc { i64, i8 } @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFADSRys5UInt8VGXEfU_SS_SiTg5"(i64 %0, i64 %1, i64 %2, ptr swiftself %3, ptr noalias nocapture swifterror dereferenceable(8) %4) local_unnamed_addr #0 {
entry:
  %5 = inttoptr i64 %0 to ptr
  %6 = load i8, ptr %5, align 1
  switch i8 %6, label %103 [
    i8 45, label %7
    i8 43, label %59
  ]

7:                                                ; preds = %entry
  %8 = icmp eq i64 %0, 0
  %9 = getelementptr inbounds %Ts5UInt8V, ptr %5, i64 1
  %10 = ptrtoint ptr %9 to i64
  %11 = select i1 %8, i64 0, i64 %10
  %12 = add i64 %1, -1
  %13 = icmp eq i64 %12, 0
  br i1 %13, label %.loopexit, label %14, !prof !35

14:                                               ; preds = %7
  %15 = icmp sgt i64 %2, 10
  %16 = trunc i64 %2 to i8
  %17 = add i8 %16, 48
  %18 = add i8 %16, 55
  %19 = add i8 %16, 87
  %20 = select i1 %15, i8 %19, i8 97
  %21 = select i1 %15, i8 %18, i8 65
  %22 = select i1 %15, i8 58, i8 %17
  br i1 %8, label %.loopexit, label %23

23:                                               ; preds = %14
  %24 = inttoptr i64 %11 to ptr
  %25 = getelementptr inbounds %Ts5UInt8V, ptr %24, i64 %12
  br label %26

26:                                               ; preds = %23, %47
  %27 = phi ptr [ %24, %23 ], [ %49, %47 ]
  %28 = phi i64 [ 0, %23 ], [ %48, %47 ]
  %29 = load i8, ptr %27, align 1
  %30 = icmp ugt i8 %29, 47
  %31 = icmp ult i8 %29, %22
  %or.cond = select i1 %30, i1 %31, i1 false
  br i1 %or.cond, label %38, label %32, !prof !37

32:                                               ; preds = %26
  %33 = icmp ugt i8 %29, 64
  %34 = icmp ult i8 %29, %21
  %or.cond4 = select i1 %33, i1 %34, i1 false
  br i1 %or.cond4, label %38, label %35, !prof !37

35:                                               ; preds = %32
  %36 = icmp ugt i8 %29, 96
  %37 = icmp ult i8 %29, %20
  %or.cond5 = select i1 %36, i1 %37, i1 false
  br i1 %or.cond5, label %38, label %.loopexit, !prof !37

38:                                               ; preds = %35, %26, %32
  %.sink = phi i8 [ -55, %32 ], [ -48, %26 ], [ -87, %35 ]
  %39 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %28, i64 %2)
  %40 = extractvalue { i64, i1 } %39, 1
  br i1 %40, label %.loopexit, label %41

41:                                               ; preds = %38
  %42 = add i8 %29, %.sink
  %43 = extractvalue { i64, i1 } %39, 0
  %44 = zext i8 %42 to i64
  %45 = tail call { i64, i1 } @llvm.ssub.with.overflow.i64(i64 %43, i64 %44)
  %46 = extractvalue { i64, i1 } %45, 1
  br i1 %46, label %.loopexit, label %47, !prof !35

47:                                               ; preds = %41
  %48 = extractvalue { i64, i1 } %45, 0
  %49 = getelementptr inbounds %Ts5UInt8V, ptr %27, i64 1
  %50 = icmp eq ptr %49, %25
  br i1 %50, label %.loopexit.loopexit115.split.loop.exit, label %26

.loopexit.loopexit.split.loop.exit:               ; preds = %138
  %51 = extractvalue { i64, i1 } %136, 0
  br label %.loopexit

.loopexit.loopexit115.split.loop.exit:            ; preds = %47
  %52 = extractvalue { i64, i1 } %45, 0
  br label %.loopexit

.loopexit.loopexit118.split.loop.exit:            ; preds = %99
  %53 = extractvalue { i64, i1 } %97, 0
  br label %.loopexit

.loopexit:                                        ; preds = %.loopexit.loopexit118.split.loop.exit, %87, %90, %93, %.loopexit.loopexit115.split.loop.exit, %35, %38, %41, %.loopexit.loopexit.split.loop.exit, %126, %129, %132, %105, %103, %66, %59, %14, %7
  %54 = phi i64 [ 0, %7 ], [ 0, %14 ], [ 0, %59 ], [ 0, %66 ], [ 0, %103 ], [ 0, %105 ], [ %51, %.loopexit.loopexit.split.loop.exit ], [ 0, %126 ], [ 0, %129 ], [ 0, %132 ], [ %52, %.loopexit.loopexit115.split.loop.exit ], [ 0, %35 ], [ 0, %38 ], [ 0, %41 ], [ %53, %.loopexit.loopexit118.split.loop.exit ], [ 0, %87 ], [ 0, %90 ], [ 0, %93 ]
  %55 = phi i1 [ true, %7 ], [ false, %14 ], [ true, %59 ], [ false, %66 ], [ true, %103 ], [ false, %105 ], [ false, %.loopexit.loopexit.split.loop.exit ], [ true, %126 ], [ true, %129 ], [ true, %132 ], [ false, %.loopexit.loopexit115.split.loop.exit ], [ true, %35 ], [ true, %38 ], [ true, %41 ], [ false, %.loopexit.loopexit118.split.loop.exit ], [ true, %87 ], [ true, %90 ], [ true, %93 ]
  %56 = zext i1 %55 to i8
  %57 = insertvalue { i64, i8 } undef, i64 %54, 0
  %58 = insertvalue { i64, i8 } %57, i8 %56, 1
  ret { i64, i8 } %58

59:                                               ; preds = %entry
  %60 = icmp eq i64 %0, 0
  %61 = getelementptr inbounds %Ts5UInt8V, ptr %5, i64 1
  %62 = ptrtoint ptr %61 to i64
  %63 = select i1 %60, i64 0, i64 %62
  %64 = add i64 %1, -1
  %65 = icmp eq i64 %64, 0
  br i1 %65, label %.loopexit, label %66, !prof !35

66:                                               ; preds = %59
  %67 = icmp sgt i64 %2, 10
  %68 = trunc i64 %2 to i8
  %69 = add i8 %68, 48
  %70 = add i8 %68, 55
  %71 = add i8 %68, 87
  %72 = select i1 %67, i8 %71, i8 97
  %73 = select i1 %67, i8 %70, i8 65
  %74 = select i1 %67, i8 58, i8 %69
  br i1 %60, label %.loopexit, label %75

75:                                               ; preds = %66
  %76 = inttoptr i64 %63 to ptr
  %77 = getelementptr inbounds %Ts5UInt8V, ptr %76, i64 %64
  br label %78

78:                                               ; preds = %75, %99
  %79 = phi ptr [ %76, %75 ], [ %101, %99 ]
  %80 = phi i64 [ 0, %75 ], [ %100, %99 ]
  %81 = load i8, ptr %79, align 1
  %82 = icmp ugt i8 %81, 47
  %83 = icmp ult i8 %81, %74
  %or.cond6 = select i1 %82, i1 %83, i1 false
  br i1 %or.cond6, label %90, label %84, !prof !37

84:                                               ; preds = %78
  %85 = icmp ugt i8 %81, 64
  %86 = icmp ult i8 %81, %73
  %or.cond7 = select i1 %85, i1 %86, i1 false
  br i1 %or.cond7, label %90, label %87, !prof !37

87:                                               ; preds = %84
  %88 = icmp ugt i8 %81, 96
  %89 = icmp ult i8 %81, %72
  %or.cond8 = select i1 %88, i1 %89, i1 false
  br i1 %or.cond8, label %90, label %.loopexit, !prof !37

90:                                               ; preds = %87, %78, %84
  %.sink108 = phi i8 [ -55, %84 ], [ -48, %78 ], [ -87, %87 ]
  %91 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %80, i64 %2)
  %92 = extractvalue { i64, i1 } %91, 1
  br i1 %92, label %.loopexit, label %93

93:                                               ; preds = %90
  %94 = add i8 %81, %.sink108
  %95 = extractvalue { i64, i1 } %91, 0
  %96 = zext i8 %94 to i64
  %97 = tail call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %95, i64 %96)
  %98 = extractvalue { i64, i1 } %97, 1
  br i1 %98, label %.loopexit, label %99, !prof !35

99:                                               ; preds = %93
  %100 = extractvalue { i64, i1 } %97, 0
  %101 = getelementptr inbounds %Ts5UInt8V, ptr %79, i64 1
  %102 = icmp eq ptr %101, %77
  br i1 %102, label %.loopexit.loopexit118.split.loop.exit, label %78

103:                                              ; preds = %entry
  %104 = icmp eq i64 %1, 0
  br i1 %104, label %.loopexit, label %105, !prof !35

105:                                              ; preds = %103
  %106 = icmp sgt i64 %2, 10
  %107 = trunc i64 %2 to i8
  %108 = add i8 %107, 48
  %109 = add i8 %107, 55
  %110 = add i8 %107, 87
  %111 = select i1 %106, i8 %110, i8 97
  %112 = select i1 %106, i8 %109, i8 65
  %113 = select i1 %106, i8 58, i8 %108
  %114 = icmp eq i64 %0, 0
  br i1 %114, label %.loopexit, label %115

115:                                              ; preds = %105
  %116 = getelementptr inbounds %Ts5UInt8V, ptr %5, i64 %1
  br label %117

117:                                              ; preds = %115, %138
  %118 = phi ptr [ %5, %115 ], [ %140, %138 ]
  %119 = phi i64 [ 0, %115 ], [ %139, %138 ]
  %120 = load i8, ptr %118, align 1
  %121 = icmp ugt i8 %120, 47
  %122 = icmp ult i8 %120, %113
  %or.cond9 = select i1 %121, i1 %122, i1 false
  br i1 %or.cond9, label %129, label %123, !prof !37

123:                                              ; preds = %117
  %124 = icmp ugt i8 %120, 64
  %125 = icmp ult i8 %120, %112
  %or.cond10 = select i1 %124, i1 %125, i1 false
  br i1 %or.cond10, label %129, label %126, !prof !37

126:                                              ; preds = %123
  %127 = icmp ugt i8 %120, 96
  %128 = icmp ult i8 %120, %111
  %or.cond11 = select i1 %127, i1 %128, i1 false
  br i1 %or.cond11, label %129, label %.loopexit, !prof !37

129:                                              ; preds = %126, %117, %123
  %.sink109 = phi i8 [ -55, %123 ], [ -48, %117 ], [ -87, %126 ]
  %130 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %119, i64 %2)
  %131 = extractvalue { i64, i1 } %130, 1
  br i1 %131, label %.loopexit, label %132

132:                                              ; preds = %129
  %133 = add i8 %120, %.sink109
  %134 = extractvalue { i64, i1 } %130, 0
  %135 = zext i8 %133 to i64
  %136 = tail call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %134, i64 %135)
  %137 = extractvalue { i64, i1 } %136, 1
  br i1 %137, label %.loopexit, label %138, !prof !35

138:                                              ; preds = %132
  %139 = extractvalue { i64, i1 } %136, 0
  %140 = getelementptr inbounds %Ts5UInt8V, ptr %118, i64 1
  %141 = icmp eq ptr %140, %116
  br i1 %141, label %.loopexit.loopexit.split.loop.exit, label %117
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc { i64, ptr } @"$sSS8_copyingyS2SFZ"(i64 %0, ptr %1) local_unnamed_addr #13 {
entry:
  %2 = ptrtoint ptr %1 to i64
  %3 = and i64 %2, 2305843009213693952
  %4 = icmp eq i64 %3, 0
  %5 = lshr i64 %2, 56
  %6 = and i64 %5, 15
  %7 = select i1 %4, i64 %0, i64 %6
  %8 = shl i64 %7, 16
  %9 = and i64 %2, 1152921504606846976
  %10 = icmp ne i64 %9, 0
  %11 = and i64 %0, 576460752303423488
  %12 = icmp eq i64 %11, 0
  %or.cond = select i1 %10, i1 %12, i1 false
  %.v = select i1 %or.cond, i64 11, i64 7
  %13 = or i64 %8, %.v
  %14 = tail call ptr @swift_bridgeObjectRetain(ptr returned %1) #6
  %15 = tail call swiftcc { i64, i64, i64, ptr } @"$sSSySsSnySS5IndexVGcig"(i64 15, i64 %13, i64 %0, ptr %1)
  tail call void @swift_bridgeObjectRelease(ptr %1) #6
  %16 = extractvalue { i64, i64, i64, ptr } %15, 0
  %17 = extractvalue { i64, i64, i64, ptr } %15, 1
  %18 = extractvalue { i64, i64, i64, ptr } %15, 2
  %19 = extractvalue { i64, i64, i64, ptr } %15, 3
  %20 = tail call swiftcc { i64, ptr } @"$sSS8_copyingySSSsFZ"(i64 %16, i64 %17, i64 %18, ptr %19)
  tail call void @swift_bridgeObjectRelease(ptr %19) #6
  ret { i64, ptr } %20
}

declare swiftcc { i64, i64, i64, ptr } @"$sSSySsSnySS5IndexVGcig"(i64, i64, i64, ptr) local_unnamed_addr #0

; Function Attrs: noinline
define linkonce_odr hidden swiftcc { i64, ptr } @"$sSS8_copyingySSSsFZ"(i64 %0, i64 %1, i64 %2, ptr %3) local_unnamed_addr #13 {
entry:
  %4 = alloca %Ts16IndexingIteratorVySs8UTF8ViewVG, align 8
  %5 = alloca <{ %Ts6UInt64V, %Ts6UInt64V }>, align 8
  %6 = ptrtoint ptr %3 to i64
  %7 = and i64 %6, 1152921504606846976
  %8 = icmp eq i64 %7, 0
  br i1 %8, label %25, label %9, !prof !36

9:                                                ; preds = %entry
  %10 = tail call swiftcc i64 @"$sSlsE5countSivgSs8UTF8ViewV_Tgq5"(i64 %0, i64 %1, i64 %2, ptr %3)
  %11 = icmp eq i64 %10, 0
  br i1 %11, label %._crit_edge, label %12

12:                                               ; preds = %9
  %13 = tail call swiftcc ptr @"$ss22_ContiguousArrayBufferV19_uninitializedCount15minimumCapacityAByxGSi_SitcfCs5UInt8V_Tgmq5"(i64 %10, i64 0)
  %14 = getelementptr inbounds i8, ptr %13, i64 32
  %15 = ptrtoint ptr %14 to i64
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %4)
  %16 = call swiftcc i64 @"$sSTsE21_copySequenceContents12initializing8IteratorQz_SitSry7ElementQzG_tFSs8UTF8ViewV_Tgq5"(ptr noalias nocapture nonnull %4, i64 %15, i64 %10, i64 %0, i64 %1, i64 %2, ptr %3)
  %._elements._slice._base._guts._object._object = getelementptr inbounds %Ts5SliceVySS8UTF8ViewVG, ptr %4, i64 0, i32 2, i32 0, i32 0, i32 1
  %17 = load ptr, ptr %._elements._slice._base._guts._object._object, align 8
  %18 = tail call ptr @swift_bridgeObjectRetain(ptr returned %3) #6
  tail call void @swift_bridgeObjectRelease(ptr %17) #6
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %4)
  br label %._crit_edge

._crit_edge:                                      ; preds = %9, %12
  %19 = phi ptr [ %13, %12 ], [ @_swiftEmptyArrayStorage, %9 ]
  %20 = getelementptr inbounds i8, ptr %19, i64 32
  %21 = ptrtoint ptr %20 to i64
  %22 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %19, i64 0, i32 1
  %23 = load i64, ptr %22, align 8, !range !39
  %24 = tail call swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64 %21, i64 %23), !noalias !40
  tail call void @swift_release(ptr %19) #6
  br label %56

25:                                               ; preds = %entry
  %26 = lshr i64 %0, 16
  %27 = lshr i64 %1, 16
  %28 = and i64 %6, 2305843009213693952
  %29 = icmp eq i64 %28, 0
  br i1 %29, label %37, label %30

30:                                               ; preds = %25
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5)
  %.elt1 = getelementptr inbounds <{ %Ts6UInt64V, %Ts6UInt64V }>, ptr %5, i64 0, i32 1
  %31 = and i64 %6, 72057594037927935
  store i64 %2, ptr %5, align 8
  store i64 %31, ptr %.elt1, align 8
  %32 = getelementptr inbounds %Ts5UInt8V, ptr %5, i64 %26
  %33 = ptrtoint ptr %32 to i64
  %34 = sub nsw i64 %27, %26
  %35 = call ptr @swift_bridgeObjectRetain(ptr returned %3) #6
  %36 = call swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64 %33, i64 %34)
  call void @swift_bridgeObjectRelease(ptr %3) #6
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5)
  br label %56

37:                                               ; preds = %25
  %38 = and i64 %2, 1152921504606846976
  %39 = icmp eq i64 %38, 0
  br i1 %39, label %43, label %40, !prof !35

40:                                               ; preds = %37
  %41 = and i64 %6, 1152921504606846975
  %42 = add nuw nsw i64 %41, 32
  br label %47

43:                                               ; preds = %37
  %44 = tail call swiftcc { i64, i64 } @"$ss13_StringObjectV10sharedUTF8SRys5UInt8VGvg"(i64 %2, ptr %3)
  %45 = extractvalue { i64, i64 } %44, 0
  %46 = icmp eq i64 %45, 0
  br i1 %46, label %51, label %47

47:                                               ; preds = %43, %40
  %.in = phi i64 [ %42, %40 ], [ %45, %43 ]
  %48 = inttoptr i64 %.in to ptr
  %49 = getelementptr inbounds %Ts5UInt8V, ptr %48, i64 %26
  %50 = ptrtoint ptr %49 to i64
  br label %51

51:                                               ; preds = %43, %47
  %52 = phi i64 [ %50, %47 ], [ 0, %43 ]
  %53 = sub nsw i64 %27, %26
  %54 = tail call ptr @swift_bridgeObjectRetain(ptr returned %3) #6
  %55 = tail call swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64 %52, i64 %53)
  tail call void @swift_bridgeObjectRelease(ptr %3) #6
  br label %56

56:                                               ; preds = %51, %30, %._crit_edge
  %.pn = phi { i64, ptr } [ %24, %._crit_edge ], [ %36, %30 ], [ %55, %51 ]
  ret { i64, ptr } %.pn
}

define linkonce_odr hidden swiftcc i64 @"$sSlsE5countSivgSs8UTF8ViewV_Tgq5"(i64 %0, i64 %1, i64 %2, ptr %3) local_unnamed_addr #0 {
entry:
  %4 = ptrtoint ptr %3 to i64
  %5 = and i64 %4, 1152921504606846976
  %6 = icmp eq i64 %5, 0
  %7 = and i64 %2, 576460752303423488
  %8 = icmp ne i64 %7, 0
  %9 = select i1 %6, i1 true, i1 %8
  %10 = and i64 %0, 12
  %11 = zext i1 %9 to i64
  %12 = shl nuw nsw i64 4, %11
  %13 = icmp eq i64 %10, %12
  br i1 %13, label %14, label %16, !prof !35

14:                                               ; preds = %entry
  %15 = tail call swiftcc i64 @"$ss11_StringGutsV27_slowEnsureMatchingEncodingySS5IndexVAEF"(i64 %0, i64 %2, ptr %3)
  br label %16

16:                                               ; preds = %entry, %14
  %17 = phi i64 [ %15, %14 ], [ %0, %entry ]
  %18 = and i64 %1, 12
  %19 = icmp eq i64 %18, %12
  br i1 %19, label %20, label %22, !prof !35

20:                                               ; preds = %16
  %21 = tail call swiftcc i64 @"$ss11_StringGutsV27_slowEnsureMatchingEncodingySS5IndexVAEF"(i64 %1, i64 %2, ptr %3)
  br label %22

22:                                               ; preds = %16, %20
  %23 = phi i64 [ %21, %20 ], [ %1, %16 ]
  br i1 %6, label %26, label %24, !prof !36

24:                                               ; preds = %22
  %25 = tail call swiftcc i64 @"$sSS8UTF8ViewV16_foreignDistance4from2toSiSS5IndexV_AGtF"(i64 %17, i64 %23, i64 %2, ptr %3)
  br label %30

26:                                               ; preds = %22
  %27 = lshr i64 %23, 16
  %28 = lshr i64 %17, 16
  %29 = sub nsw i64 %27, %28
  br label %30

30:                                               ; preds = %26, %24
  %31 = phi i64 [ %25, %24 ], [ %29, %26 ]
  ret i64 %31
}

define linkonce_odr hidden swiftcc ptr @"$ss22_ContiguousArrayBufferV19_uninitializedCount15minimumCapacityAByxGSi_SitcfCs5UInt8V_Tgmq5"(i64 %0, i64 %1) local_unnamed_addr #14 {
entry:
  %2 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %3 = icmp eq i64 %2, 0
  br i1 %3, label %10, label %4

4:                                                ; preds = %entry
  %5 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCys5UInt8VGMD") #22
  %6 = add i64 %2, 32
  %7 = tail call noalias ptr @swift_allocObject(ptr %5, i64 %6, i64 7) #6
  %call.i = tail call i64 @malloc_size(ptr noundef %7) #27, !clang.arc.no_objc_arc_exceptions !33
  %gepdiff = shl i64 %call.i, 1
  %8 = add i64 %gepdiff, -64
  %9 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %7, i64 0, i32 1
  store i64 %0, ptr %9, align 8
  %._storage1._capacityAndFlags = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %7, i64 0, i32 1, i32 0, i32 1
  store i64 %8, ptr %._storage1._capacityAndFlags, align 8
  br label %10

10:                                               ; preds = %entry, %4
  %11 = phi ptr [ %7, %4 ], [ @_swiftEmptyArrayStorage, %entry ]
  ret ptr %11
}

define linkonce_odr hidden swiftcc i64 @"$sSTsE21_copySequenceContents12initializing8IteratorQz_SitSry7ElementQzG_tFSs8UTF8ViewV_Tgq5"(ptr noalias nocapture %0, i64 %1, i64 %2, i64 %3, i64 %4, i64 %5, ptr %6) local_unnamed_addr #0 {
entry:
  %7 = alloca <{ %Ts6UInt64V, %Ts6UInt64V }>, align 8
  %8 = icmp eq i64 %1, 0
  br i1 %8, label %.loopexit11, label %9

9:                                                ; preds = %entry
  %10 = inttoptr i64 %1 to ptr
  %11 = icmp eq i64 %2, 0
  br i1 %11, label %.loopexit11, label %12

12:                                               ; preds = %9
  %13 = lshr i64 %4, 14
  %14 = lshr i64 %3, 14
  %15 = icmp eq i64 %14, %13
  br i1 %15, label %.loopexit11, label %16

16:                                               ; preds = %12
  %17 = ptrtoint ptr %6 to i64
  %18 = and i64 %17, 1152921504606846976
  %19 = icmp eq i64 %18, 0
  %20 = and i64 %5, 576460752303423488
  %21 = icmp ne i64 %20, 0
  %22 = select i1 %19, i1 true, i1 %21
  %23 = zext i1 %22 to i64
  %24 = shl nuw nsw i64 4, %23
  %25 = and i64 %17, 2305843009213693952
  %26 = icmp eq i64 %25, 0
  %.elt5 = getelementptr inbounds <{ %Ts6UInt64V, %Ts6UInt64V }>, ptr %7, i64 0, i32 1
  %27 = and i64 %17, 72057594037927935
  %28 = and i64 %5, 1152921504606846976
  %29 = icmp eq i64 %28, 0
  %30 = and i64 %17, 1152921504606846975
  %31 = add nuw nsw i64 %30, 32
  br label %32

32:                                               ; preds = %76, %16
  %33 = phi ptr [ %10, %16 ], [ %77, %76 ]
  %34 = phi i64 [ %3, %16 ], [ %71, %76 ]
  %35 = phi i64 [ 1, %16 ], [ %78, %76 ]
  %36 = and i64 %34, 12
  %37 = icmp eq i64 %36, %24
  br i1 %37, label %38, label %40, !prof !35

38:                                               ; preds = %32
  %39 = tail call swiftcc i64 @"$ss11_StringGutsV27_slowEnsureMatchingEncodingySS5IndexVAEF"(i64 %34, i64 %5, ptr %6)
  br label %40

40:                                               ; preds = %32, %38
  %41 = phi i64 [ %39, %38 ], [ %34, %32 ]
  br i1 %19, label %42, label %43

42:                                               ; preds = %40
  br i1 %26, label %45, label %46

43:                                               ; preds = %40
  %44 = tail call swiftcc i8 @"$sSS8UTF8ViewV17_foreignSubscript8positions5UInt8VSS5IndexV_tF"(i64 %41, i64 %5, ptr %6)
  br label %59

45:                                               ; preds = %42
  br i1 %29, label %50, label %53, !prof !35

46:                                               ; preds = %42
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %7)
  store i64 %5, ptr %7, align 8
  store i64 %27, ptr %.elt5, align 8
  %47 = lshr i64 %41, 16
  %48 = getelementptr inbounds %Ts5UInt8V, ptr %7, i64 %47
  %49 = load i8, ptr %48, align 1
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %7)
  br label %59

50:                                               ; preds = %45
  %51 = tail call swiftcc { i64, i64 } @"$ss13_StringObjectV10sharedUTF8SRys5UInt8VGvg"(i64 %5, ptr %6)
  %52 = extractvalue { i64, i64 } %51, 0
  br label %53

53:                                               ; preds = %45, %50
  %54 = phi i64 [ %52, %50 ], [ %31, %45 ]
  %55 = lshr i64 %41, 16
  %56 = inttoptr i64 %54 to ptr
  %57 = getelementptr inbounds %Ts5UInt8V, ptr %56, i64 %55
  %58 = load i8, ptr %57, align 1
  br label %59

59:                                               ; preds = %53, %46, %43
  %60 = phi i8 [ %44, %43 ], [ %49, %46 ], [ %58, %53 ]
  br i1 %37, label %61, label %63, !prof !35

61:                                               ; preds = %59
  %62 = tail call swiftcc i64 @"$ss11_StringGutsV27_slowEnsureMatchingEncodingySS5IndexVAEF"(i64 %34, i64 %5, ptr %6)
  br label %63

63:                                               ; preds = %59, %61
  %64 = phi i64 [ %62, %61 ], [ %34, %59 ]
  br i1 %19, label %67, label %65

65:                                               ; preds = %63
  %66 = tail call swiftcc i64 @"$sSS8UTF8ViewV13_foreignIndex5afterSS0D0VAF_tF"(i64 %64, i64 %5, ptr %6)
  br label %70

67:                                               ; preds = %63
  %68 = and i64 %64, -65536
  %69 = add i64 %68, 65540
  br label %70

70:                                               ; preds = %67, %65
  %71 = phi i64 [ %66, %65 ], [ %69, %67 ]
  store i8 %60, ptr %33, align 1
  %72 = icmp eq i64 %35, %2
  br i1 %72, label %.loopexit11, label %73

73:                                               ; preds = %70
  %74 = lshr i64 %71, 14
  %75 = icmp eq i64 %74, %13
  br i1 %75, label %.loopexit11, label %76

76:                                               ; preds = %73
  %77 = getelementptr inbounds %Ts5UInt8V, ptr %33, i64 1
  %78 = add i64 %35, 1
  br label %32

.loopexit11:                                      ; preds = %70, %73, %9, %12, %entry
  %.sink = phi i64 [ %3, %entry ], [ %3, %12 ], [ %3, %9 ], [ %71, %73 ], [ %71, %70 ]
  %79 = phi i64 [ 0, %entry ], [ 0, %12 ], [ %2, %9 ], [ %2, %70 ], [ %35, %73 ]
  store i64 %3, ptr %0, align 8
  %._elements3._slice._endIndex = getelementptr inbounds %Ts5SliceVySS8UTF8ViewVG, ptr %0, i64 0, i32 1
  store i64 %4, ptr %._elements3._slice._endIndex, align 8
  %._elements3._slice._base = getelementptr inbounds %Ts5SliceVySS8UTF8ViewVG, ptr %0, i64 0, i32 2
  store i64 %5, ptr %._elements3._slice._base, align 8
  %._elements3._slice._base._guts._object._object = getelementptr inbounds %Ts5SliceVySS8UTF8ViewVG, ptr %0, i64 0, i32 2, i32 0, i32 0, i32 1
  store ptr %6, ptr %._elements3._slice._base._guts._object._object, align 8
  %._position4 = getelementptr inbounds %Ts16IndexingIteratorVySs8UTF8ViewVG, ptr %0, i64 0, i32 1
  store i64 %.sink, ptr %._position4, align 8
  ret i64 %79
}

declare swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64, i64) local_unnamed_addr #0

; Function Attrs: noinline
declare swiftcc i8 @"$sSS8UTF8ViewV17_foreignSubscript8positions5UInt8VSS5IndexV_tF"(i64, i64, ptr) local_unnamed_addr #13

; Function Attrs: noinline
declare swiftcc i64 @"$sSS8UTF8ViewV13_foreignIndex5afterSS0D0VAF_tF"(i64, i64, ptr) local_unnamed_addr #13

; Function Attrs: noinline
define linkonce_odr hidden swiftcc i64 @"$ss11_StringGutsV27_slowEnsureMatchingEncodingySS5IndexVAEF"(i64 %0, i64 %1, ptr %2) local_unnamed_addr #13 {
entry:
  %3 = ptrtoint ptr %2 to i64
  %4 = and i64 %3, 1152921504606846976
  %5 = icmp ne i64 %4, 0
  %6 = and i64 %1, 576460752303423488
  %7 = icmp eq i64 %6, 0
  %or.cond = select i1 %5, i1 %7, i1 false
  %8 = lshr i64 %0, 16
  %9 = lshr i64 %0, 14
  %10 = and i64 %9, 3
  %11 = icmp eq i64 %10, 0
  br i1 %or.cond, label %12, label %25

12:                                               ; preds = %entry
  %13 = tail call swiftcc i64 @"$sSS8UTF8ViewV13_foreignIndex_8offsetBySS0D0VAF_SitF"(i64 15, i64 %8, i64 %1, ptr %2)
  br i1 %11, label %18, label %14

14:                                               ; preds = %12
  %15 = shl nuw nsw i64 %10, 16
  %16 = add i64 %13, %15
  %17 = and i64 %16, -65536
  br label %22

18:                                               ; preds = %12
  %19 = and i64 %13, -4
  %20 = and i64 %0, 3
  %21 = or i64 %19, %20
  br label %22

22:                                               ; preds = %18, %14
  %23 = phi i64 [ %17, %14 ], [ %21, %18 ]
  %24 = or i64 %23, 8
  br label %38

25:                                               ; preds = %entry
  %26 = tail call swiftcc i64 @"$sSS9UTF16ViewV5index_8offsetBySS5IndexVAF_SitF"(i64 15, i64 %8, i64 %1, ptr %2)
  br i1 %11, label %31, label %27

27:                                               ; preds = %25
  %28 = shl nuw nsw i64 %10, 16
  %29 = add i64 %26, %28
  %30 = and i64 %29, -65536
  br label %35

31:                                               ; preds = %25
  %32 = and i64 %26, -4
  %33 = and i64 %0, 3
  %34 = or i64 %32, %33
  br label %35

35:                                               ; preds = %31, %27
  %36 = phi i64 [ %30, %27 ], [ %34, %31 ]
  %37 = or i64 %36, 4
  br label %38

38:                                               ; preds = %22, %35
  %39 = phi i64 [ %37, %35 ], [ %24, %22 ]
  ret i64 %39
}

declare swiftcc i64 @"$sSS9UTF16ViewV5index_8offsetBySS5IndexVAF_SitF"(i64, i64, i64, ptr) local_unnamed_addr #0

; Function Attrs: noinline
declare swiftcc i64 @"$sSS8UTF8ViewV13_foreignIndex_8offsetBySS0D0VAF_SitF"(i64, i64, i64, ptr) local_unnamed_addr #13

; Function Attrs: noinline
declare swiftcc i64 @"$sSS8UTF8ViewV16_foreignDistance4from2toSiSS5IndexV_AGtF"(i64, i64, i64, ptr) local_unnamed_addr #13

; Function Attrs: nounwind
define private swiftcc void @objectdestroy.5(ptr swiftself %0) #3 {
entry:
  tail call void @swift_deallocObject(ptr %0, i64 48, i64 7) #6
  ret void
}

define internal swiftcc void @"$s4mainyyYbcfU0_yyYbcfU_Tf2nnni_nTA"(ptr nocapture readonly swiftself %0) #0 {
entry:
  %1 = alloca %TSS, align 8
  %2 = alloca %Ts11_StringGutsV, align 8
  %3 = alloca %TSi, align 8
  %4 = alloca %TSi, align 8
  %5 = alloca %TSi, align 8
  %6 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi, %TSi, %TSi }>, ptr %0, i64 0, i32 1
  %7 = load i64, ptr %6, align 8
  %8 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi, %TSi, %TSi }>, ptr %0, i64 0, i32 2
  %9 = load i64, ptr %8, align 8
  %10 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi, %TSi, %TSi }>, ptr %0, i64 0, i32 3
  %11 = load i64, ptr %10, align 8
  %12 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi, %TSi, %TSi }>, ptr %0, i64 0, i32 4
  %13 = load i64, ptr %12, align 8
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %1)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %2)
  store i64 0, ptr %2, align 8
  %._object._object.i = getelementptr inbounds %Ts13_StringObjectV, ptr %2, i64 0, i32 1
  store ptr inttoptr (i64 -2305843009213693952 to ptr), ptr %._object._object.i, align 8
  call swiftcc void @"$ss11_StringGutsV4growyySiF"(i64 32, ptr nocapture nonnull swiftself dereferenceable(16) %2)
  %14 = load ptr, ptr %._object._object.i, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %2)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3)
  store i64 %9, ptr %3, align 8
  %15 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %3, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  tail call void @swift_bridgeObjectRelease(ptr %14) #6
  %16 = extractvalue { i64, ptr } %15, 0
  %17 = extractvalue { i64, ptr } %15, 1
  store i64 %16, ptr %1, align 8
  %._guts._object._object.i = getelementptr inbounds %Ts13_StringObjectV, ptr %1, i64 0, i32 1
  store ptr %17, ptr %._guts._object._object.i, align 8
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3)
  call swiftcc void @"$sSS6appendyySSF"(i64 -3458764513820540911, ptr nonnull inttoptr (i64 or (i64 sub (i64 ptrtoint (ptr @".str.17.\09 trees of depth " to i64), i64 32), i64 -9223372036854775808) to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %1)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4)
  store i64 %11, ptr %4, align 8
  %18 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %4, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %19 = extractvalue { i64, ptr } %18, 0
  %20 = extractvalue { i64, ptr } %18, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %19, ptr %20, ptr nocapture nonnull swiftself dereferenceable(16) %1)
  tail call void @swift_bridgeObjectRelease(ptr %20) #6
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4)
  call swiftcc void @"$sSS6appendyySSF"(i64 4209567563902033929, ptr nonnull inttoptr (i64 -1657324662872342496 to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %1)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5)
  store i64 %13, ptr %5, align 8
  %21 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %5, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %22 = extractvalue { i64, ptr } %21, 0
  %23 = extractvalue { i64, ptr } %21, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %22, ptr %23, ptr nocapture nonnull swiftself dereferenceable(16) %1)
  tail call void @swift_bridgeObjectRelease(ptr %23) #6
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5)
  %24 = load i64, ptr %1, align 8
  %25 = load ptr, ptr %._guts._object._object.i, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %1)
  %26 = load ptr, ptr @"$s4main7resultsSaySSGvp", align 8
  %27 = tail call zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr %26) #23
  store ptr %26, ptr @"$s4main7resultsSaySSGvp", align 8
  br i1 %27, label %"$s4mainyyYbcfU0_yyYbcfU_Tf2nnni_n.exit", label %28, !prof !36

28:                                               ; preds = %entry
  %29 = tail call swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNewAByxGyFSS_Tg5"(ptr %26)
  store ptr %29, ptr @"$s4main7resultsSaySSGvp", align 8
  br label %"$s4mainyyYbcfU0_yyYbcfU_Tf2nnni_n.exit"

"$s4mainyyYbcfU0_yyYbcfU_Tf2nnni_n.exit":         ; preds = %entry, %28
  %30 = phi ptr [ %29, %28 ], [ %26, %entry ]
  %31 = getelementptr inbounds i8, ptr %30, i64 32
  %32 = getelementptr inbounds %TSS, ptr %31, i64 %7
  %._guts7._object._object.i = getelementptr inbounds %Ts13_StringObjectV, ptr %32, i64 0, i32 1
  %33 = load ptr, ptr %._guts7._object._object.i, align 8
  store i64 %24, ptr %32, align 8
  store ptr %25, ptr %._guts7._object._object.i, align 8
  tail call void @swift_bridgeObjectRelease(ptr %33) #6
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
declare zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr) local_unnamed_addr #12

; Function Attrs: noinline
define linkonce_odr hidden swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNewAByxGyFSS_Tg5"(ptr %0) local_unnamed_addr #13 {
entry:
  %1 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8, !range !39
  %3 = tail call swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSS_Tg5"(i1 false, i64 %2, i1 false, ptr %0)
  ret ptr %3
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSS_Tg5"(i1 %0, i64 %1, i1 %2, ptr %3) local_unnamed_addr #15 {
entry:
  %4 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %3, i64 0, i32 1
  br i1 %2, label %5, label %12

5:                                                ; preds = %entry
  %._storage._capacityAndFlags = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %3, i64 0, i32 1, i32 0, i32 1
  %6 = load i64, ptr %._storage._capacityAndFlags, align 8
  %7 = lshr i64 %6, 1
  %8 = icmp slt i64 %7, %1
  br i1 %8, label %9, label %12

9:                                                ; preds = %5
  %10 = and i64 %6, -2
  %11 = tail call i64 @llvm.smax.i64(i64 %10, i64 %1)
  br label %12

12:                                               ; preds = %5, %9, %entry
  %13 = phi i64 [ %1, %entry ], [ %11, %9 ], [ %7, %5 ]
  %14 = load i64, ptr %4, align 8, !range !39
  %15 = tail call i64 @llvm.smax.i64(i64 %13, i64 %14)
  %16 = icmp eq i64 %15, 0
  br i1 %16, label %25, label %17

17:                                               ; preds = %12
  %18 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCySSGMD") #22
  %19 = shl i64 %15, 4
  %20 = add i64 %19, 32
  %21 = tail call noalias ptr @swift_allocObject(ptr %18, i64 %20, i64 7) #6
  %call.i = tail call i64 @malloc_size(ptr noundef %21) #27, !clang.arc.no_objc_arc_exceptions !33
  %gepdiff = add nsw i64 %call.i, -32
  %22 = sdiv i64 %gepdiff, 16
  %23 = shl nsw i64 %22, 1
  %24 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %21, i64 0, i32 1
  store i64 %14, ptr %24, align 8
  %._storage3._capacityAndFlags = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %21, i64 0, i32 1, i32 0, i32 1
  store i64 %23, ptr %._storage3._capacityAndFlags, align 8
  br label %25

25:                                               ; preds = %12, %17
  %26 = phi ptr [ %21, %17 ], [ @_swiftEmptyArrayStorage, %12 ]
  %27 = getelementptr inbounds i8, ptr %26, i64 32
  %28 = getelementptr inbounds i8, ptr %3, i64 32
  br i1 %0, label %29, label %35

29:                                               ; preds = %25
  %30 = getelementptr inbounds %TSS, ptr %28, i64 %14
  %31 = icmp ult ptr %27, %30
  %32 = icmp eq ptr %26, %3
  %or.cond9 = select i1 %32, i1 %31, i1 false
  br i1 %or.cond9, label %34, label %.sink.split

.sink.split:                                      ; preds = %29
  %33 = shl nuw i64 %14, 4
  tail call void @llvm.memmove.p0.p0.i64(ptr nonnull align 8 %27, ptr nonnull align 8 %28, i64 %33, i1 false)
  br label %34

34:                                               ; preds = %29, %.sink.split
  store i64 0, ptr %4, align 8
  br label %36

35:                                               ; preds = %25
  tail call void @swift_arrayInitWithCopy(ptr nonnull %27, ptr nonnull %28, i64 %14, ptr nonnull @"$sSSN") #23
  br label %36

36:                                               ; preds = %34, %35
  tail call void @swift_bridgeObjectRelease(ptr nonnull %3) #6
  ret ptr %26
}

; Function Attrs: mustprogress nounwind willreturn
declare void @swift_arrayInitWithCopy(ptr, ptr, i64, ptr) local_unnamed_addr #12

; Function Attrs: argmemonly mustprogress nocallback nofree nounwind willreturn
declare void @llvm.memmove.p0.p0.i64(ptr nocapture writeonly, ptr nocapture readonly, i64, i1 immarg) #16

; Function Attrs: noinline
declare swiftcc ptr @"$sSa28_allocateBufferUninitialized15minimumCapacitys06_ArrayB0VyxGSi_tFZ"(i64, ptr) local_unnamed_addr #13

; Function Attrs: optsize
declare i64 @malloc_size(ptr noundef) local_unnamed_addr #17

; Function Attrs: inaccessiblememonly nocallback nofree nosync nounwind willreturn
declare void @llvm.assume(i1 noundef) #18

; Function Attrs: nocallback nofree nosync nounwind readnone speculatable willreturn
declare i64 @llvm.smax.i64(i64, i64) #19

; Function Attrs: argmemonly nocallback nofree nounwind willreturn writeonly
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #20

; Function Attrs: nounwind
declare ptr @swift_bridgeObjectRetain_n(ptr, i32) #6

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.1(ptr nocapture writeonly %0, ptr nocapture readonly %1) #11 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #11
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.7(ptr nocapture writeonly %0, ptr nocapture readonly %1) #11 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #11
  ret void
}

; Function Attrs: nounwind
define internal void @block_destroy_helper.2(ptr nocapture readonly %0) #3 {
  tail call void @block_destroy_helper(ptr nocapture readonly %0) #3
  ret void
}

; Function Attrs: nounwind
define internal void @block_destroy_helper.8(ptr nocapture readonly %0) #3 {
  tail call void @block_destroy_helper(ptr nocapture readonly %0) #3
  ret void
}

attributes #0 = { "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #1 = { argmemonly mustprogress nocallback nofree nosync nounwind willreturn }
attributes #2 = { mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn }
attributes #3 = { nounwind "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #4 = { mustprogress nofree noinline nounwind readonly willreturn "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #5 = { argmemonly nounwind }
attributes #6 = { nounwind }
attributes #7 = { nounwind optsize "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #8 = { nofree noinline nosync nounwind readnone "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #9 = { mustprogress nofree nosync nounwind readnone willreturn }
attributes #10 = { nofree nounwind readonly }
attributes #11 = { mustprogress nounwind willreturn "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #12 = { mustprogress nounwind willreturn }
attributes #13 = { noinline "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #14 = { "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #15 = { noinline "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #16 = { argmemonly mustprogress nocallback nofree nounwind willreturn }
attributes #17 = { optsize "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #18 = { inaccessiblememonly nocallback nofree nosync nounwind willreturn }
attributes #19 = { nocallback nofree nosync nounwind readnone speculatable willreturn }
attributes #20 = { argmemonly nocallback nofree nounwind willreturn writeonly }
attributes #21 = { nounwind readnone }
attributes #22 = { nounwind readonly }
attributes #23 = { nounwind willreturn }
attributes #24 = { noinline }
attributes #25 = { argmemonly nounwind readonly }
attributes #26 = { nounwind readnone willreturn }
attributes #27 = { optsize }

!llvm.module.flags = !{!0, !1, !2, !3, !4, !5, !6, !7, !8, !9, !10, !11}
!swift.module.flags = !{!12}
!llvm.linker.options = !{!13, !14, !15, !16, !17, !18, !19, !20, !21, !22, !23, !24, !25, !26, !27, !28, !29, !30, !31, !32}

!0 = !{i32 2, !"SDK Version", [2 x i32] [i32 14, i32 4]}
!1 = !{i32 1, !"Objective-C Version", i32 2}
!2 = !{i32 1, !"Objective-C Image Info Version", i32 0}
!3 = !{i32 1, !"Objective-C Image Info Section", !"__DATA,__objc_imageinfo,regular,no_dead_strip"}
!4 = !{i32 4, !"Objective-C Garbage Collection", i32 84543232}
!5 = !{i32 1, !"Objective-C Class Properties", i32 64}
!6 = !{i32 1, !"Objective-C Enforce ClassRO Pointer Signing", i8 0}
!7 = !{i32 1, !"wchar_size", i32 4}
!8 = !{i32 8, !"PIC Level", i32 2}
!9 = !{i32 7, !"uwtable", i32 1}
!10 = !{i32 7, !"frame-pointer", i32 1}
!11 = !{i32 1, !"Swift Version", i32 7}
!12 = !{!"standard-library", i1 false}
!13 = !{!"-lswiftFoundation"}
!14 = !{!"-framework", !"Foundation"}
!15 = !{!"-lswiftCore"}
!16 = !{!"-lswift_StringProcessing"}
!17 = !{!"-lswift_Concurrency"}
!18 = !{!"-lswiftObservation"}
!19 = !{!"-lswiftObjectiveC"}
!20 = !{!"-lswiftDarwin"}
!21 = !{!"-lswiftCoreFoundation"}
!22 = !{!"-framework", !"CoreFoundation"}
!23 = !{!"-lswiftDispatch"}
!24 = !{!"-framework", !"Combine"}
!25 = !{!"-framework", !"CoreServices"}
!26 = !{!"-framework", !"Security"}
!27 = !{!"-lswiftXPC"}
!28 = !{!"-framework", !"CFNetwork"}
!29 = !{!"-framework", !"DiskArbitration"}
!30 = !{!"-lswiftIOKit"}
!31 = !{!"-framework", !"IOKit"}
!32 = !{!"-lobjc"}
!33 = !{}
!34 = !{i64 88}
!35 = !{!"branch_weights", i32 1, i32 2000}
!36 = !{!"branch_weights", i32 2000, i32 1}
!37 = !{!"branch_weights", i32 2000, i32 2002}
!38 = !{!"branch_weights", i32 2002, i32 2000}
!39 = !{i64 0, i64 9223372036854775807}
!40 = !{!41}
!41 = distinct !{!41, !42, !"$sSS8_copyingySSSsFZSSSRys5UInt8VGXEfU0_: argument 0"}
!42 = distinct !{!42, !"$sSS8_copyingySSSsFZSSSRys5UInt8VGXEfU0_"}
