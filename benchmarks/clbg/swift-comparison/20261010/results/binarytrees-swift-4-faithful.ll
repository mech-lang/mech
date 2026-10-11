; ModuleID = '/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/swift-comparison/20261010/results/binarytrees-swift-4-faithful.ll'
source_filename = "/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/swift-comparison/20261010/results/binarytrees-swift-4-faithful.ll"
target datalayout = "e-m:o-i64:64-i128:128-n32:64-S128"
target triple = "arm64-apple-macosx16.0.0"

%TSi = type <{ i64 }>
%TSD = type <{ %TSD8_VariantV }>
%TSD8_VariantV = type <{ %Ts14_BridgeStorageV }>
%Ts14_BridgeStorageV = type <{ ptr }>
%T4main8TreeNodeCSg = type <{ [8 x i8] }>
%struct._SwiftEmptyArrayStorage = type { %struct.HeapObject, %struct._SwiftArrayBodyStorage }
%struct.HeapObject = type { ptr, %struct.InlineRefCountsPlaceholder }
%struct.InlineRefCountsPlaceholder = type { i64 }
%struct._SwiftArrayBodyStorage = type { i64, i64 }
%struct._SwiftEmptyDictionarySingleton = type { %struct.HeapObject, %struct._SwiftDictionaryBodyStorage, i64 }
%struct._SwiftDictionaryBodyStorage = type { i64, i64, i8, i8, i16, i32, i64, ptr, ptr }
%objc_class = type { ptr, ptr, ptr, ptr, ptr }
%swift.protocol_conformance_descriptor = type { i32, i32, i32, i32 }
%swift.type_descriptor = type opaque
%swift.type = type { i64 }
%swift.opaque = type opaque
%swift.method_descriptor = type { i32, i32 }
%swift.full_boxmetadata = type { ptr, ptr, %swift.type, i32, ptr }
%swift.type_metadata_record = type { i32 }
%TSa = type <{ %Ts12_ArrayBufferV }>
%Ts12_ArrayBufferV = type <{ %Ts14_BridgeStorageV }>
%TSS = type <{ %Ts11_StringGutsV }>
%Ts11_StringGutsV = type <{ %Ts13_StringObjectV }>
%Ts13_StringObjectV = type <{ %Ts6UInt64V, ptr }>
%Ts6UInt64V = type <{ i64 }>
%objc_block = type { ptr, i32, i32, ptr, ptr }
%swift.function = type { ptr, ptr }
%Ts15ContiguousArrayV = type <{ %Ts22_ContiguousArrayBufferV }>
%Ts22_ContiguousArrayBufferV = type <{ ptr }>
%TSD8IteratorV = type <{ %TSD8IteratorV8_VariantO }>
%TSD8IteratorV8_VariantO = type <{ [40 x i8] }>
%swift.metadata_response = type { ptr, i64 }
%swift.vwtable = type { ptr, ptr, ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i32, i32 }
%Ts5UInt8V = type <{ i8 }>
%Ts22__RawDictionaryStorageC = type <{ %swift.refcounted, %TSi, %TSi, %Ts4Int8V, %Ts4Int8V, %Ts5Int16V, %Ts5Int32V, %TSi, %TSv, %TSv }>
%swift.refcounted = type { ptr, i64 }
%Ts4Int8V = type <{ i8 }>
%Ts5Int16V = type <{ i16 }>
%Ts5Int32V = type <{ i32 }>
%TSv = type <{ ptr }>
%Ts28__ContiguousArrayStorageBaseC = type <{ %swift.refcounted, %Ts10_ArrayBodyV }>
%Ts10_ArrayBodyV = type <{ %TSo22_SwiftArrayBodyStorageV }>
%TSo22_SwiftArrayBodyStorageV = type <{ %TSi, %TSu }>
%TSu = type <{ i64 }>
%T4main8TreeNodeC = type <{ %swift.refcounted, %T4main8TreeNodeCSg, %T4main8TreeNodeCSg }>
%Ts17_NativeDictionaryV = type <{ ptr }>
%Ts13_UnsafeBitsetV4WordV = type <{ %TSu }>
%TSrySi3key_SS5valuetG = type <{ %TSpySi3key_SS5valuetGSg, %TSi }>
%TSpySi3key_SS5valuetGSg = type <{ [8 x i8] }>
%TSnySiG = type <{ %TSi, %TSi }>
%Ts16IndexingIteratorVySs8UTF8ViewVG = type <{ %TSs8UTF8ViewV, %TSS5IndexV }>
%TSs8UTF8ViewV = type <{ %Ts5SliceVySS8UTF8ViewVG }>
%Ts5SliceVySS8UTF8ViewVG = type <{ %TSS5IndexV, %TSS5IndexV, %TSS8UTF8ViewV }>
%TSS8UTF8ViewV = type <{ %Ts11_StringGutsV }>
%TSS5IndexV = type <{ %Ts6UInt64V }>

@"$s4main1nSivp" = hidden local_unnamed_addr global %TSi zeroinitializer, align 8
@"$s4main8minDepthSivp" = hidden local_unnamed_addr global %TSi zeroinitializer, align 8
@"$s4main8maxDepthSivp" = hidden local_unnamed_addr global %TSi zeroinitializer, align 8
@"$s4main8messagesSDySiSSGvp" = hidden global %TSD zeroinitializer, align 8
@"$s4main5depthSivp" = hidden local_unnamed_addr global %TSi zeroinitializer, align 8
@"$s4main5groupSo012OS_dispatch_B0Cvp" = hidden local_unnamed_addr global ptr null, align 8
@"$s4main11workerQueueSo17OS_dispatch_queueCvp" = hidden local_unnamed_addr global ptr null, align 8
@"$s4main12messageQueueSo17OS_dispatch_queueCvp" = hidden local_unnamed_addr global ptr null, align 8
@"$s4main13longLivedTreeAA0D4NodeCSgvp" = hidden local_unnamed_addr global %T4main8TreeNodeCSg zeroinitializer, align 8
@_swiftEmptyArrayStorage = external global %struct._SwiftEmptyArrayStorage, align 8
@_swiftEmptyDictionarySingleton = external global %struct._SwiftEmptyDictionarySingleton, align 8
@"$sSo17OS_dispatch_queueCML" = linkonce_odr hidden local_unnamed_addr global ptr null, align 8
@"OBJC_CLASS_REF_$_OS_dispatch_queue" = private externally_initialized global ptr @"OBJC_CLASS_$_OS_dispatch_queue", section "__DATA,__objc_classrefs,regular,no_dead_strip", align 8
@"OBJC_CLASS_$_OS_dispatch_queue" = external global %objc_class, align 8
@"$sSo17OS_dispatch_queueC8DispatchE20AutoreleaseFrequencyO7inherityA2EmFWC" = external local_unnamed_addr constant i32, align 4
@"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWL" = linkonce_odr hidden local_unnamed_addr global ptr null, align 8
@"$sSo17OS_dispatch_queueC8DispatchE10AttributesVs10SetAlgebraACMc" = external global %swift.protocol_conformance_descriptor, align 4
@"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMn" = external global %swift.type_descriptor, align 4
@"got.$sSo17OS_dispatch_queueC8DispatchE10AttributesVMn" = private unnamed_addr constant ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMn"
@"symbolic Say_____G So17OS_dispatch_queueC8DispatchE10AttributesV" = linkonce_odr hidden constant <{ [3 x i8], i8, i32, [1 x i8], i8 }> <{ [3 x i8] c"Say", i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$sSo17OS_dispatch_queueC8DispatchE10AttributesVMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ [3 x i8], i8, i32, [1 x i8], i8 }>, ptr @"symbolic Say_____G So17OS_dispatch_queueC8DispatchE10AttributesV", i32 0, i32 2) to i64)) to i32), [1 x i8] c"G", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Say_____G So17OS_dispatch_queueC8DispatchE10AttributesV" to i64), i64 ptrtoint (ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGMD" to i64)) to i32), i32 -9 }, align 8
@"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWL" = linkonce_odr hidden local_unnamed_addr global ptr null, align 8
@"$sSayxGSTsMc" = external global %swift.protocol_conformance_descriptor, align 4
@".str.22.stretch tree of depth " = private unnamed_addr constant [23 x i8] c"stretch tree of depth \00"
@"$sSiN" = external global %swift.type, align 8
@"$sSis23CustomStringConvertiblesWP" = external global ptr, align 8
@_NSConcreteStackBlock = external global %objc_class
@".str.5.v8@?0" = private unnamed_addr constant [6 x i8] c"v8@?0\00"
@block_descriptor = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper, ptr @block_destroy_helper, ptr @".str.5.v8@?0" }, align 8
@"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL" = linkonce_odr hidden local_unnamed_addr global ptr null, align 8
@"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc" = external global %swift.protocol_conformance_descriptor, align 4
@"$s8Dispatch0A13WorkItemFlagsVMn" = external global %swift.type_descriptor, align 4
@"got.$s8Dispatch0A13WorkItemFlagsVMn" = private unnamed_addr constant ptr @"$s8Dispatch0A13WorkItemFlagsVMn"
@"symbolic Say_____G 8Dispatch0A13WorkItemFlagsV" = linkonce_odr hidden constant <{ [3 x i8], i8, i32, [1 x i8], i8 }> <{ [3 x i8] c"Say", i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$s8Dispatch0A13WorkItemFlagsVMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ [3 x i8], i8, i32, [1 x i8], i8 }>, ptr @"symbolic Say_____G 8Dispatch0A13WorkItemFlagsV", i32 0, i32 2) to i64)) to i32), [1 x i8] c"G", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$sSay8Dispatch0A13WorkItemFlagsVGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Say_____G 8Dispatch0A13WorkItemFlagsV" to i64), i64 ptrtoint (ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD" to i64)) to i32), i32 -9 }, align 8
@"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL" = linkonce_odr hidden local_unnamed_addr global ptr null, align 8
@".str.25.long lived tree of depth " = private unnamed_addr constant [26 x i8] c"long lived tree of depth \00"
@"$ss23_ContiguousArrayStorageCMn" = external global %swift.type_descriptor, align 4
@"got.$ss23_ContiguousArrayStorageCMn" = private unnamed_addr constant ptr @"$ss23_ContiguousArrayStorageCMn"
@"symbolic _____yypG s23_ContiguousArrayStorageC" = linkonce_odr hidden constant <{ i8, i32, [4 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss23_ContiguousArrayStorageCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [4 x i8], i8 }>, ptr @"symbolic _____yypG s23_ContiguousArrayStorageC", i32 0, i32 1) to i64)) to i32), [4 x i8] c"yypG", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss23_ContiguousArrayStorageCyypGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____yypG s23_ContiguousArrayStorageC" to i64), i64 ptrtoint (ptr @"$ss23_ContiguousArrayStorageCyypGMD" to i64)) to i32), i32 -9 }, align 8
@"$sSSN" = external global %swift.type, align 8
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
@"symbolic Si3key_SS5valuet" = linkonce_odr hidden constant <{ [16 x i8], i8 }> <{ [16 x i8] c"Si3key_SS5valuet", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$sSi3key_SS5valuetMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Si3key_SS5valuet" to i64), i64 ptrtoint (ptr @"$sSi3key_SS5valuetMD" to i64)) to i32), i32 -16 }, align 8
@"symbolic _____ySnySiGG s23_ContiguousArrayStorageC" = linkonce_odr hidden constant <{ i8, i32, [8 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss23_ContiguousArrayStorageCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [8 x i8], i8 }>, ptr @"symbolic _____ySnySiGG s23_ContiguousArrayStorageC", i32 0, i32 1) to i64)) to i32), [8 x i8] c"ySnySiGG", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss23_ContiguousArrayStorageCySnySiGGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ySnySiGG s23_ContiguousArrayStorageC" to i64), i64 ptrtoint (ptr @"$ss23_ContiguousArrayStorageCySnySiGGMD" to i64)) to i32), i32 -13 }, align 8
@"symbolic _____ySi3key_SS5valuetG s23_ContiguousArrayStorageC" = linkonce_odr hidden constant <{ i8, i32, [18 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss23_ContiguousArrayStorageCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [18 x i8], i8 }>, ptr @"symbolic _____ySi3key_SS5valuetG s23_ContiguousArrayStorageC", i32 0, i32 1) to i64)) to i32), [18 x i8] c"ySi3key_SS5valuetG", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss23_ContiguousArrayStorageCySi3key_SS5valuetGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ySi3key_SS5valuetG s23_ContiguousArrayStorageC" to i64), i64 ptrtoint (ptr @"$ss23_ContiguousArrayStorageCySi3key_SS5valuetGMD" to i64)) to i32), i32 -23 }, align 8
@"symbolic Si" = linkonce_odr hidden constant <{ [2 x i8], i8 }> <{ [2 x i8] c"Si", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"\01l__swift5_reflection_descriptor" = private constant { i32, i32, i32, i32, i32 } { i32 2, i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Si" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor", i32 0, i32 3) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Si" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor", i32 0, i32 4) to i64)) to i32) }, section "__TEXT,__swift5_capture, regular", no_sanitize_address, align 4
@metadata = private constant %swift.full_boxmetadata { ptr @objectdestroy, ptr null, %swift.type { i64 1024 }, i32 16, ptr @"\01l__swift5_reflection_descriptor" }, align 8
@block_descriptor.3 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.1, ptr @block_destroy_helper.2, ptr @".str.5.v8@?0" }, align 8
@block_descriptor.6 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.4, ptr @block_destroy_helper.5, ptr @".str.5.v8@?0" }, align 8
@"symbolic SS" = linkonce_odr hidden constant <{ [2 x i8], i8 }> <{ [2 x i8] c"SS", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"\01l__swift5_reflection_descriptor.7" = private constant { i32, i32, i32, i32, i32 } { i32 2, i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic Si" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.7", i32 0, i32 3) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic SS" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.7", i32 0, i32 4) to i64)) to i32) }, section "__TEXT,__swift5_capture, regular", no_sanitize_address, align 4
@metadata.9 = private constant %swift.full_boxmetadata { ptr @objectdestroy.8, ptr null, %swift.type { i64 1024 }, i32 16, ptr @"\01l__swift5_reflection_descriptor.7" }, align 8
@block_descriptor.12 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.10, ptr @block_destroy_helper.11, ptr @".str.5.v8@?0" }, align 8
@".str.17.\09 trees of depth " = private unnamed_addr constant [18 x i8] c"\09 trees of depth \00"
@"$ss18_DictionaryStorageCMn" = external global %swift.type_descriptor, align 4
@"got.$ss18_DictionaryStorageCMn" = private unnamed_addr constant ptr @"$ss18_DictionaryStorageCMn"
@"symbolic _____ySiSSG s18_DictionaryStorageC" = linkonce_odr hidden constant <{ i8, i32, [6 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss18_DictionaryStorageCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [6 x i8], i8 }>, ptr @"symbolic _____ySiSSG s18_DictionaryStorageC", i32 0, i32 1) to i64)) to i32), [6 x i8] c"ySiSSG", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss18_DictionaryStorageCySiSSGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ySiSSG s18_DictionaryStorageC" to i64), i64 ptrtoint (ptr @"$ss18_DictionaryStorageCySiSSGMD" to i64)) to i32), i32 -11 }, align 8
@"$s4main8TreeNodeCHn" = private constant %swift.type_metadata_record { i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main8TreeNodeCMn" to i64), i64 ptrtoint (ptr @"$s4main8TreeNodeCHn" to i64)) to i32) }, section "__TEXT, __swift5_types, regular", no_sanitize_address, align 4
@__swift_reflection_version = linkonce_odr hidden constant i16 3
@"objc_classes_$s4main8TreeNodeCN" = internal global ptr @"$s4main8TreeNodeCN", section "__DATA,__objc_classlist,regular,no_dead_strip", no_sanitize_address, align 8
@llvm.used = appending global [15 x ptr] [ptr @"\01l__swift5_reflection_descriptor", ptr @"\01l__swift5_reflection_descriptor.7", ptr @"\01l_entry_point", ptr @"$s4main8TreeNodeCHn", ptr @"$s4main8TreeNodeCMF", ptr @__swift_reflection_version, ptr @"_swift_FORCE_LOAD_$_swiftCoreFoundation_$_main", ptr @"_swift_FORCE_LOAD_$_swiftDarwin_$_main", ptr @"_swift_FORCE_LOAD_$_swiftDispatch_$_main", ptr @"_swift_FORCE_LOAD_$_swiftFoundation_$_main", ptr @"_swift_FORCE_LOAD_$_swiftIOKit_$_main", ptr @"_swift_FORCE_LOAD_$_swiftObjectiveC_$_main", ptr @"_swift_FORCE_LOAD_$_swiftXPC_$_main", ptr @main, ptr @"objc_classes_$s4main8TreeNodeCN"], section "llvm.metadata"
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
  %2 = alloca %TSa, align 8
  %3 = alloca %TSS, align 8
  %4 = alloca %Ts11_StringGutsV, align 8
  %5 = alloca %TSi, align 8
  %6 = alloca %TSi, align 8
  %7 = alloca { %objc_block, %swift.function }, align 8
  %8 = alloca %TSa, align 8
  %9 = alloca %TSS, align 8
  %10 = alloca %Ts11_StringGutsV, align 8
  %11 = alloca %TSi, align 8
  %12 = alloca %TSi, align 8
  %13 = alloca %Ts15ContiguousArrayV, align 8
  %14 = alloca %TSD8IteratorV, align 8
  %swifterror = alloca swifterror ptr, align 8
  store ptr null, ptr %swifterror, align 8
  %.sroa.6 = alloca i8, align 8
  %15 = alloca <{ %Ts6UInt64V, %Ts6UInt64V }>, align 8
  %16 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64 0) #22
  %17 = extractvalue %swift.metadata_response %16, 0
  %18 = getelementptr inbounds ptr, ptr %17, i64 -1
  %.valueWitnesses = load ptr, ptr %18, align 8, !invariant.load !33, !dereferenceable !34
  %19 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses, i64 0, i32 8
  %size = load i64, ptr %19, align 8, !invariant.load !33
  %20 = alloca i8, i64 %size, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %20)
  %21 = tail call swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE20AutoreleaseFrequencyOMa"(i64 0) #22
  %22 = extractvalue %swift.metadata_response %21, 0
  %23 = getelementptr inbounds ptr, ptr %22, i64 -1
  %.valueWitnesses1 = load ptr, ptr %23, align 8, !invariant.load !33, !dereferenceable !34
  %24 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses1, i64 0, i32 8
  %size2 = load i64, ptr %24, align 8, !invariant.load !33
  %25 = alloca i8, i64 %size2, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %25)
  %26 = tail call swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMa"(i64 0) #22
  %27 = extractvalue %swift.metadata_response %26, 0
  %28 = getelementptr inbounds ptr, ptr %27, i64 -1
  %.valueWitnesses3 = load ptr, ptr %28, align 8, !invariant.load !33, !dereferenceable !34
  %29 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses3, i64 0, i32 8
  %size4 = load i64, ptr %29, align 8, !invariant.load !33
  %30 = alloca i8, i64 %size4, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %30)
  %31 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64 0) #22
  %32 = extractvalue %swift.metadata_response %31, 0
  %33 = getelementptr inbounds ptr, ptr %32, i64 -1
  %.valueWitnesses5 = load ptr, ptr %33, align 8, !invariant.load !33, !dereferenceable !34
  %34 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses5, i64 0, i32 8
  %size6 = load i64, ptr %34, align 8, !invariant.load !33
  %35 = alloca i8, i64 %size6, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %35)
  %36 = tail call swiftcc i32 @"$ss11CommandLineO4argcs5Int32VvgZ"()
  %37 = icmp sgt i32 %36, 1
  br i1 %37, label %38, label %332

38:                                               ; preds = %entry
  %39 = tail call swiftcc ptr @"$ss11CommandLineO9argumentsSaySSGvgZ"()
  %40 = getelementptr inbounds i8, ptr %39, i64 48
  %41 = load i64, ptr %40, align 8
  %._guts29._object._object = getelementptr inbounds i8, ptr %39, i64 56
  %42 = load ptr, ptr %._guts29._object._object, align 8
  %43 = tail call ptr @swift_bridgeObjectRetain(ptr returned %42) #9
  tail call void @swift_bridgeObjectRelease(ptr %39) #9
  %44 = ptrtoint ptr %42 to i64
  %45 = and i64 %44, 2305843009213693952
  %46 = icmp eq i64 %45, 0
  %47 = lshr i64 %44, 56
  %48 = and i64 %47, 15
  %49 = and i64 %41, 281474976710655
  %50 = select i1 %46, i64 %49, i64 %48
  %51 = icmp eq i64 %50, 0
  br i1 %51, label %52, label %53, !prof !35

52:                                               ; preds = %38
  tail call void @swift_bridgeObjectRelease(ptr %42) #9
  br label %332

53:                                               ; preds = %38
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %.sroa.6)
  %54 = and i64 %44, 1152921504606846976
  %55 = icmp eq i64 %54, 0
  br i1 %55, label %56, label %94, !prof !36

56:                                               ; preds = %53
  br i1 %46, label %57, label %60

57:                                               ; preds = %56
  %58 = and i64 %41, 1152921504606846976
  %59 = icmp eq i64 %58, 0
  br i1 %59, label %67, label %71, !prof !35

60:                                               ; preds = %56
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %15)
  %.elt31 = getelementptr inbounds <{ %Ts6UInt64V, %Ts6UInt64V }>, ptr %15, i64 0, i32 1
  %61 = and i64 %44, 72057594037927935
  store i64 %41, ptr %15, align 8
  store i64 %61, ptr %.elt31, align 8
  %62 = trunc i64 %41 to i8
  %63 = lshr i64 %41, 8
  %64 = trunc i64 %63 to i8
  %65 = lshr i64 %41, 16
  %66 = trunc i64 %65 to i8
  switch i8 %62, label %264 [
    i8 45, label %221
    i8 43, label %262
  ]

67:                                               ; preds = %57
  %68 = tail call swiftcc { i64, i64 } @"$ss13_StringObjectV10sharedUTF8SRys5UInt8VGvg"(i64 %41, ptr %42)
  %69 = extractvalue { i64, i64 } %68, 0
  %70 = extractvalue { i64, i64 } %68, 1
  br label %74

71:                                               ; preds = %57
  %72 = and i64 %44, 1152921504606846975
  %73 = add nuw nsw i64 %72, 32
  br label %74

74:                                               ; preds = %67, %71
  %75 = phi i64 [ %73, %71 ], [ %69, %67 ]
  %76 = phi i64 [ %49, %71 ], [ %70, %67 ]
  %77 = inttoptr i64 %75 to ptr
  %78 = load i8, ptr %77, align 1
  switch i8 %78, label %140 [
    i8 45, label %79
    i8 43, label %137
  ]

79:                                               ; preds = %74
  %80 = add i64 %76, -1
  %81 = icmp eq i64 %80, 0
  br i1 %81, label %.loopexit, label %82, !prof !35

82:                                               ; preds = %79
  %83 = icmp eq i64 %75, 0
  %84 = getelementptr inbounds %Ts5UInt8V, ptr %77, i64 1
  %85 = ptrtoint ptr %84 to i64
  %86 = select i1 %83, i64 0, i64 %85
  %87 = inttoptr i64 %86 to ptr
  br i1 %83, label %.loopexit, label %100

.loopexit.loopexit.split.loop.exit:               ; preds = %217
  %88 = extractvalue { i64, i1 } %215, 0
  br label %.loopexit

.loopexit.loopexit302.split.loop.exit:            ; preds = %133
  %89 = extractvalue { i64, i1 } %131, 0
  br label %.loopexit

.loopexit.loopexit305.split.loop.exit:            ; preds = %180
  %90 = extractvalue { i64, i1 } %178, 0
  br label %.loopexit

.loopexit:                                        ; preds = %.loopexit.loopexit305.split.loop.exit, %175, %172, %.preheader133, %.loopexit.loopexit302.split.loop.exit, %128, %125, %.preheader129, %.loopexit.loopexit.split.loop.exit, %212, %209, %.preheader, %198, %193, %190, %186, %184, %140, %161, %156, %153, %148, %142, %137, %114, %109, %105, %100, %82, %79
  %91 = phi i64 [ 0, %79 ], [ 0, %82 ], [ 0, %100 ], [ %107, %105 ], [ 0, %109 ], [ %117, %114 ], [ 0, %137 ], [ 0, %142 ], [ 0, %148 ], [ %154, %153 ], [ 0, %156 ], [ %164, %161 ], [ 0, %140 ], [ 0, %184 ], [ 0, %186 ], [ %191, %190 ], [ 0, %193 ], [ %201, %198 ], [ %88, %.loopexit.loopexit.split.loop.exit ], [ 0, %212 ], [ 0, %209 ], [ 0, %.preheader ], [ %89, %.loopexit.loopexit302.split.loop.exit ], [ 0, %128 ], [ 0, %125 ], [ 0, %.preheader129 ], [ %90, %.loopexit.loopexit305.split.loop.exit ], [ 0, %175 ], [ 0, %172 ], [ 0, %.preheader133 ]
  %92 = phi i1 [ true, %79 ], [ false, %82 ], [ true, %100 ], [ false, %105 ], [ true, %109 ], [ false, %114 ], [ true, %137 ], [ false, %142 ], [ true, %148 ], [ false, %153 ], [ true, %156 ], [ false, %161 ], [ true, %140 ], [ false, %184 ], [ true, %186 ], [ false, %190 ], [ true, %193 ], [ false, %198 ], [ false, %.loopexit.loopexit.split.loop.exit ], [ true, %212 ], [ true, %209 ], [ true, %.preheader ], [ false, %.loopexit.loopexit302.split.loop.exit ], [ true, %128 ], [ true, %125 ], [ true, %.preheader129 ], [ false, %.loopexit.loopexit305.split.loop.exit ], [ true, %175 ], [ true, %172 ], [ true, %.preheader133 ]
  store i1 %92, ptr %.sroa.6, align 8
  br label %93

93:                                               ; preds = %.loopexit, %.loopexit138
  %.sroa.0.1 = phi i64 [ %91, %.loopexit ], [ %230, %.loopexit138 ]
  %.sroa.6.0..sroa.6.0..sroa.6.0..sroa.6.8. = load i8, ptr %.sroa.6, align 8
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.6)
  call void @swift_bridgeObjectRelease(ptr %42) #9
  br label %98

94:                                               ; preds = %53
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.6)
  %95 = tail call swiftcc { i64, i8 } @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFSS_SiTg5"(i64 %41, ptr %42, i64 10)
  tail call void @swift_bridgeObjectRelease(ptr %42) #9
  %96 = extractvalue { i64, i8 } %95, 0
  %97 = extractvalue { i64, i8 } %95, 1
  br label %98

98:                                               ; preds = %93, %94
  %99 = phi i64 [ %96, %94 ], [ %.sroa.0.1, %93 ]
  %.in.in = phi i8 [ %97, %94 ], [ %.sroa.6.0..sroa.6.0..sroa.6.0..sroa.6.8., %93 ]
  %.in = and i8 %.in.in, 1
  %.not125 = icmp eq i8 %.in, 0
  %. = select i1 %.not125, i64 %99, i64 10
  br label %332

100:                                              ; preds = %82
  %101 = getelementptr inbounds %Ts5UInt8V, ptr %87, i64 %80
  %102 = load i8, ptr %87, align 1
  %103 = add i8 %102, -48
  %104 = icmp ult i8 %103, 10
  br i1 %104, label %105, label %.loopexit, !prof !37

105:                                              ; preds = %100
  %106 = zext i8 %103 to i64
  %107 = sub nsw i64 0, %106
  %108 = icmp eq i64 %80, 1
  br i1 %108, label %.loopexit, label %109

109:                                              ; preds = %105
  %110 = getelementptr inbounds %Ts5UInt8V, ptr %87, i64 1
  %111 = load i8, ptr %110, align 1
  %112 = add i8 %111, -48
  %113 = icmp ult i8 %112, 10
  br i1 %113, label %114, label %.loopexit, !prof !37

114:                                              ; preds = %109
  %115 = mul nsw i64 %106, -10
  %116 = zext i8 %112 to i64
  %117 = sub nsw i64 %115, %116
  %118 = icmp eq i64 %80, 2
  br i1 %118, label %.loopexit, label %.preheader129.preheader

.preheader129.preheader:                          ; preds = %114
  %119 = getelementptr inbounds %Ts5UInt8V, ptr %87, i64 2
  br label %.preheader129

.preheader129:                                    ; preds = %.preheader129.preheader, %133
  %120 = phi ptr [ %135, %133 ], [ %119, %.preheader129.preheader ]
  %121 = phi i64 [ %134, %133 ], [ %117, %.preheader129.preheader ]
  %122 = load i8, ptr %120, align 1
  %123 = add i8 %122, -48
  %124 = icmp ult i8 %123, 10
  br i1 %124, label %125, label %.loopexit, !prof !37

125:                                              ; preds = %.preheader129
  %126 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %121, i64 10)
  %127 = extractvalue { i64, i1 } %126, 1
  br i1 %127, label %.loopexit, label %128

128:                                              ; preds = %125
  %129 = extractvalue { i64, i1 } %126, 0
  %130 = zext i8 %123 to i64
  %131 = tail call { i64, i1 } @llvm.ssub.with.overflow.i64(i64 %129, i64 %130)
  %132 = extractvalue { i64, i1 } %131, 1
  br i1 %132, label %.loopexit, label %133, !prof !35

133:                                              ; preds = %128
  %134 = extractvalue { i64, i1 } %131, 0
  %135 = getelementptr inbounds %Ts5UInt8V, ptr %120, i64 1
  %136 = icmp eq ptr %135, %101
  br i1 %136, label %.loopexit.loopexit302.split.loop.exit, label %.preheader129

137:                                              ; preds = %74
  %138 = add i64 %76, -1
  %139 = icmp eq i64 %138, 0
  br i1 %139, label %.loopexit, label %142, !prof !35

140:                                              ; preds = %74
  %141 = icmp eq i64 %76, 0
  br i1 %141, label %.loopexit, label %184, !prof !35

142:                                              ; preds = %137
  %143 = icmp eq i64 %75, 0
  %144 = getelementptr inbounds %Ts5UInt8V, ptr %77, i64 1
  %145 = ptrtoint ptr %144 to i64
  %146 = select i1 %143, i64 0, i64 %145
  %147 = inttoptr i64 %146 to ptr
  br i1 %143, label %.loopexit, label %148

148:                                              ; preds = %142
  %149 = getelementptr inbounds %Ts5UInt8V, ptr %147, i64 %138
  %150 = load i8, ptr %147, align 1
  %151 = add i8 %150, -48
  %152 = icmp ult i8 %151, 10
  br i1 %152, label %153, label %.loopexit, !prof !37

153:                                              ; preds = %148
  %154 = zext i8 %151 to i64
  %155 = icmp eq i64 %138, 1
  br i1 %155, label %.loopexit, label %156

156:                                              ; preds = %153
  %157 = getelementptr inbounds %Ts5UInt8V, ptr %147, i64 1
  %158 = load i8, ptr %157, align 1
  %159 = add i8 %158, -48
  %160 = icmp ult i8 %159, 10
  br i1 %160, label %161, label %.loopexit, !prof !37

161:                                              ; preds = %156
  %162 = mul nuw nsw i64 %154, 10
  %163 = zext i8 %159 to i64
  %164 = add nuw nsw i64 %162, %163
  %165 = icmp eq i64 %138, 2
  br i1 %165, label %.loopexit, label %.preheader133.preheader

.preheader133.preheader:                          ; preds = %161
  %166 = getelementptr inbounds %Ts5UInt8V, ptr %147, i64 2
  br label %.preheader133

.preheader133:                                    ; preds = %.preheader133.preheader, %180
  %167 = phi ptr [ %182, %180 ], [ %166, %.preheader133.preheader ]
  %168 = phi i64 [ %181, %180 ], [ %164, %.preheader133.preheader ]
  %169 = load i8, ptr %167, align 1
  %170 = add i8 %169, -48
  %171 = icmp ult i8 %170, 10
  br i1 %171, label %172, label %.loopexit, !prof !37

172:                                              ; preds = %.preheader133
  %173 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %168, i64 10)
  %174 = extractvalue { i64, i1 } %173, 1
  br i1 %174, label %.loopexit, label %175

175:                                              ; preds = %172
  %176 = extractvalue { i64, i1 } %173, 0
  %177 = zext i8 %170 to i64
  %178 = tail call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %176, i64 %177)
  %179 = extractvalue { i64, i1 } %178, 1
  br i1 %179, label %.loopexit, label %180, !prof !35

180:                                              ; preds = %175
  %181 = extractvalue { i64, i1 } %178, 0
  %182 = getelementptr inbounds %Ts5UInt8V, ptr %167, i64 1
  %183 = icmp eq ptr %182, %149
  br i1 %183, label %.loopexit.loopexit305.split.loop.exit, label %.preheader133

184:                                              ; preds = %140
  %185 = icmp eq i64 %75, 0
  br i1 %185, label %.loopexit, label %186

186:                                              ; preds = %184
  %187 = getelementptr inbounds %Ts5UInt8V, ptr %77, i64 %76
  %188 = add i8 %78, -48
  %189 = icmp ult i8 %188, 10
  br i1 %189, label %190, label %.loopexit, !prof !37

190:                                              ; preds = %186
  %191 = zext i8 %188 to i64
  %192 = icmp eq i64 %76, 1
  br i1 %192, label %.loopexit, label %193

193:                                              ; preds = %190
  %194 = getelementptr inbounds %Ts5UInt8V, ptr %77, i64 1
  %195 = load i8, ptr %194, align 1
  %196 = add i8 %195, -48
  %197 = icmp ult i8 %196, 10
  br i1 %197, label %198, label %.loopexit, !prof !37

198:                                              ; preds = %193
  %199 = mul nuw nsw i64 %191, 10
  %200 = zext i8 %196 to i64
  %201 = add nuw nsw i64 %199, %200
  %202 = icmp eq i64 %76, 2
  br i1 %202, label %.loopexit, label %.preheader.preheader

.preheader.preheader:                             ; preds = %198
  %203 = getelementptr inbounds %Ts5UInt8V, ptr %77, i64 2
  br label %.preheader

.preheader:                                       ; preds = %.preheader.preheader, %217
  %204 = phi ptr [ %219, %217 ], [ %203, %.preheader.preheader ]
  %205 = phi i64 [ %218, %217 ], [ %201, %.preheader.preheader ]
  %206 = load i8, ptr %204, align 1
  %207 = add i8 %206, -48
  %208 = icmp ult i8 %207, 10
  br i1 %208, label %209, label %.loopexit, !prof !37

209:                                              ; preds = %.preheader
  %210 = tail call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %205, i64 10)
  %211 = extractvalue { i64, i1 } %210, 1
  br i1 %211, label %.loopexit, label %212

212:                                              ; preds = %209
  %213 = extractvalue { i64, i1 } %210, 0
  %214 = zext i8 %207 to i64
  %215 = tail call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %213, i64 %214)
  %216 = extractvalue { i64, i1 } %215, 1
  br i1 %216, label %.loopexit, label %217, !prof !35

217:                                              ; preds = %212
  %218 = extractvalue { i64, i1 } %215, 0
  %219 = getelementptr inbounds %Ts5UInt8V, ptr %204, i64 1
  %220 = icmp eq ptr %219, %187
  br i1 %220, label %.loopexit.loopexit.split.loop.exit, label %.preheader

221:                                              ; preds = %60
  %222 = icmp eq i64 %48, 1
  br i1 %222, label %.loopexit138, label %223, !prof !35

223:                                              ; preds = %221
  %224 = getelementptr inbounds %Ts5UInt8V, ptr %15, i64 %48
  %225 = add i8 %64, -48
  %226 = icmp ult i8 %225, 10
  br i1 %226, label %232, label %.loopexit138, !prof !37

.loopexit138.loopexit.split.loop.exit:            ; preds = %328
  %227 = extractvalue { i64, i1 } %326, 0
  br label %.loopexit138

.loopexit138.loopexit310.split.loop.exit:         ; preds = %258
  %228 = extractvalue { i64, i1 } %256, 0
  br label %.loopexit138

.loopexit138.loopexit313.split.loop.exit:         ; preds = %295
  %229 = extractvalue { i64, i1 } %293, 0
  br label %.loopexit138

.loopexit138:                                     ; preds = %.loopexit138.loopexit313.split.loop.exit, %290, %287, %.preheader145, %.loopexit138.loopexit310.split.loop.exit, %253, %250, %.preheader141, %.loopexit138.loopexit.split.loop.exit, %323, %320, %.preheader137, %309, %306, %303, %299, %264, %276, %273, %270, %266, %262, %239, %236, %232, %223, %221
  %230 = phi i64 [ 0, %221 ], [ 0, %223 ], [ %234, %232 ], [ 0, %236 ], [ %242, %239 ], [ 0, %262 ], [ 0, %266 ], [ %271, %270 ], [ 0, %273 ], [ %279, %276 ], [ 0, %264 ], [ 0, %299 ], [ %304, %303 ], [ 0, %306 ], [ %312, %309 ], [ %227, %.loopexit138.loopexit.split.loop.exit ], [ 0, %323 ], [ 0, %320 ], [ 0, %.preheader137 ], [ %228, %.loopexit138.loopexit310.split.loop.exit ], [ 0, %253 ], [ 0, %250 ], [ 0, %.preheader141 ], [ %229, %.loopexit138.loopexit313.split.loop.exit ], [ 0, %290 ], [ 0, %287 ], [ 0, %.preheader145 ]
  %231 = phi i1 [ true, %221 ], [ true, %223 ], [ false, %232 ], [ true, %236 ], [ false, %239 ], [ true, %262 ], [ true, %266 ], [ false, %270 ], [ true, %273 ], [ false, %276 ], [ true, %264 ], [ true, %299 ], [ false, %303 ], [ true, %306 ], [ false, %309 ], [ false, %.loopexit138.loopexit.split.loop.exit ], [ true, %323 ], [ true, %320 ], [ true, %.preheader137 ], [ false, %.loopexit138.loopexit310.split.loop.exit ], [ true, %253 ], [ true, %250 ], [ true, %.preheader141 ], [ false, %.loopexit138.loopexit313.split.loop.exit ], [ true, %290 ], [ true, %287 ], [ true, %.preheader145 ]
  store i1 %231, ptr %.sroa.6, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %15)
  br label %93

232:                                              ; preds = %223
  %233 = zext i8 %225 to i64
  %234 = sub nsw i64 0, %233
  %235 = icmp eq i64 %48, 2
  br i1 %235, label %.loopexit138, label %236

236:                                              ; preds = %232
  %237 = add i8 %66, -48
  %238 = icmp ult i8 %237, 10
  br i1 %238, label %239, label %.loopexit138, !prof !37

239:                                              ; preds = %236
  %240 = mul nsw i64 %233, -10
  %241 = zext i8 %237 to i64
  %242 = sub nsw i64 %240, %241
  %243 = icmp eq i64 %48, 3
  br i1 %243, label %.loopexit138, label %.preheader141.preheader

.preheader141.preheader:                          ; preds = %239
  %244 = getelementptr inbounds %Ts5UInt8V, ptr %15, i64 3
  br label %.preheader141

.preheader141:                                    ; preds = %.preheader141.preheader, %258
  %245 = phi ptr [ %260, %258 ], [ %244, %.preheader141.preheader ]
  %246 = phi i64 [ %259, %258 ], [ %242, %.preheader141.preheader ]
  %247 = load i8, ptr %245, align 1
  %248 = add i8 %247, -48
  %249 = icmp ult i8 %248, 10
  br i1 %249, label %250, label %.loopexit138, !prof !37

250:                                              ; preds = %.preheader141
  %251 = call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %246, i64 10)
  %252 = extractvalue { i64, i1 } %251, 1
  br i1 %252, label %.loopexit138, label %253

253:                                              ; preds = %250
  %254 = extractvalue { i64, i1 } %251, 0
  %255 = zext i8 %248 to i64
  %256 = call { i64, i1 } @llvm.ssub.with.overflow.i64(i64 %254, i64 %255)
  %257 = extractvalue { i64, i1 } %256, 1
  br i1 %257, label %.loopexit138, label %258, !prof !35

258:                                              ; preds = %253
  %259 = extractvalue { i64, i1 } %256, 0
  %260 = getelementptr inbounds %Ts5UInt8V, ptr %245, i64 1
  %261 = icmp eq ptr %260, %224
  br i1 %261, label %.loopexit138.loopexit310.split.loop.exit, label %.preheader141

262:                                              ; preds = %60
  %263 = icmp eq i64 %48, 1
  br i1 %263, label %.loopexit138, label %266, !prof !35

264:                                              ; preds = %60
  %265 = icmp eq i64 %48, 0
  br i1 %265, label %.loopexit138, label %299, !prof !35

266:                                              ; preds = %262
  %267 = getelementptr inbounds %Ts5UInt8V, ptr %15, i64 %48
  %268 = add i8 %64, -48
  %269 = icmp ult i8 %268, 10
  br i1 %269, label %270, label %.loopexit138, !prof !37

270:                                              ; preds = %266
  %271 = zext i8 %268 to i64
  %272 = icmp eq i64 %48, 2
  br i1 %272, label %.loopexit138, label %273

273:                                              ; preds = %270
  %274 = add i8 %66, -48
  %275 = icmp ult i8 %274, 10
  br i1 %275, label %276, label %.loopexit138, !prof !37

276:                                              ; preds = %273
  %277 = mul nuw nsw i64 %271, 10
  %278 = zext i8 %274 to i64
  %279 = add nuw nsw i64 %277, %278
  %280 = icmp eq i64 %48, 3
  br i1 %280, label %.loopexit138, label %.preheader145.preheader

.preheader145.preheader:                          ; preds = %276
  %281 = getelementptr inbounds %Ts5UInt8V, ptr %15, i64 3
  br label %.preheader145

.preheader145:                                    ; preds = %.preheader145.preheader, %295
  %282 = phi ptr [ %297, %295 ], [ %281, %.preheader145.preheader ]
  %283 = phi i64 [ %296, %295 ], [ %279, %.preheader145.preheader ]
  %284 = load i8, ptr %282, align 1
  %285 = add i8 %284, -48
  %286 = icmp ult i8 %285, 10
  br i1 %286, label %287, label %.loopexit138, !prof !37

287:                                              ; preds = %.preheader145
  %288 = call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %283, i64 10)
  %289 = extractvalue { i64, i1 } %288, 1
  br i1 %289, label %.loopexit138, label %290

290:                                              ; preds = %287
  %291 = extractvalue { i64, i1 } %288, 0
  %292 = zext i8 %285 to i64
  %293 = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %291, i64 %292)
  %294 = extractvalue { i64, i1 } %293, 1
  br i1 %294, label %.loopexit138, label %295, !prof !35

295:                                              ; preds = %290
  %296 = extractvalue { i64, i1 } %293, 0
  %297 = getelementptr inbounds %Ts5UInt8V, ptr %282, i64 1
  %298 = icmp eq ptr %297, %267
  br i1 %298, label %.loopexit138.loopexit313.split.loop.exit, label %.preheader145

299:                                              ; preds = %264
  %300 = getelementptr inbounds %Ts5UInt8V, ptr %15, i64 %48
  %301 = add i8 %62, -48
  %302 = icmp ult i8 %301, 10
  br i1 %302, label %303, label %.loopexit138, !prof !37

303:                                              ; preds = %299
  %304 = zext i8 %301 to i64
  %305 = icmp eq i64 %48, 1
  br i1 %305, label %.loopexit138, label %306

306:                                              ; preds = %303
  %307 = add i8 %64, -48
  %308 = icmp ult i8 %307, 10
  br i1 %308, label %309, label %.loopexit138, !prof !37

309:                                              ; preds = %306
  %310 = mul nuw nsw i64 %304, 10
  %311 = zext i8 %307 to i64
  %312 = add nuw nsw i64 %310, %311
  %313 = icmp eq i64 %48, 2
  br i1 %313, label %.loopexit138, label %.preheader137.preheader

.preheader137.preheader:                          ; preds = %309
  %314 = getelementptr inbounds %Ts5UInt8V, ptr %15, i64 2
  br label %.preheader137

.preheader137:                                    ; preds = %.preheader137.preheader, %328
  %315 = phi ptr [ %330, %328 ], [ %314, %.preheader137.preheader ]
  %316 = phi i64 [ %329, %328 ], [ %312, %.preheader137.preheader ]
  %317 = load i8, ptr %315, align 1
  %318 = add i8 %317, -48
  %319 = icmp ult i8 %318, 10
  br i1 %319, label %320, label %.loopexit138, !prof !37

320:                                              ; preds = %.preheader137
  %321 = call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %316, i64 10)
  %322 = extractvalue { i64, i1 } %321, 1
  br i1 %322, label %.loopexit138, label %323

323:                                              ; preds = %320
  %324 = extractvalue { i64, i1 } %321, 0
  %325 = zext i8 %318 to i64
  %326 = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %324, i64 %325)
  %327 = extractvalue { i64, i1 } %326, 1
  br i1 %327, label %.loopexit138, label %328, !prof !35

328:                                              ; preds = %323
  %329 = extractvalue { i64, i1 } %326, 0
  %330 = getelementptr inbounds %Ts5UInt8V, ptr %315, i64 1
  %331 = icmp eq ptr %330, %300
  br i1 %331, label %.loopexit138.loopexit.split.loop.exit, label %.preheader137

332:                                              ; preds = %entry, %52, %98
  %storemerge = phi i64 [ 10, %52 ], [ %., %98 ], [ 10, %entry ]
  store i64 %storemerge, ptr @"$s4main1nSivp", align 8
  store i64 4, ptr @"$s4main8minDepthSivp", align 8
  %333 = call i64 @llvm.smax.i64(i64 %storemerge, i64 6)
  store i64 %333, ptr @"$s4main8maxDepthSivp", align 8
  %334 = call swiftcc ptr @"$sSD17dictionaryLiteralSDyxq_Gx_q_td_tcfCSi_SSTgm5Tf4g_n"(ptr nonnull @_swiftEmptyArrayStorage)
  store ptr %334, ptr @"$s4main8messagesSDySiSSGvp", align 8
  %335 = load i64, ptr @"$s4main8maxDepthSivp", align 8
  %336 = add i64 %335, 1
  store i64 %336, ptr @"$s4main5depthSivp", align 8
  %337 = call ptr @dispatch_group_create()
  store ptr %337, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  %338 = call swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueCMa"(i64 0) #22
  %339 = extractvalue %swift.metadata_response %338, 0
  call swiftcc void @"$s8Dispatch0A3QoSV13userInitiatedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %35)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE10AttributesV10concurrentAEvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %30)
  %340 = load i32, ptr @"$sSo17OS_dispatch_queueC8DispatchE20AutoreleaseFrequencyO7inherityA2EmFWC", align 4
  %341 = getelementptr inbounds ptr, ptr %.valueWitnesses1, i64 13
  %342 = load ptr, ptr %341, align 8, !invariant.load !33
  call void %342(ptr noalias nonnull %25, i32 %340, ptr %22) #9
  %343 = call swiftcc ptr @"$sSo17OS_dispatch_queueC8DispatchE5label3qos10attributes20autoreleaseFrequency6targetABSS_AC0D3QoSVAbCE10AttributesVAbCE011AutoreleaseI0OABSgtcfC"(i64 8453663755471056759, ptr nonnull inttoptr (i64 -1513209474789837467 to ptr), ptr noalias nocapture nonnull %35, ptr noalias nocapture nonnull %30, ptr noalias nocapture nonnull %25, i64 0, ptr swiftself %339)
  store ptr %343, ptr @"$s4main11workerQueueSo17OS_dispatch_queueCvp", align 8
  call swiftcc void @"$s8Dispatch0A3QoSV13userInitiatedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %35)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2)
  store ptr @_swiftEmptyArrayStorage, ptr %2, align 8
  %344 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWL", ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMa", ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVs10SetAlgebraACMc")
  %345 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGMD") #23
  %346 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWL", ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %30, ptr noalias nocapture nonnull %2, ptr %345, ptr %346, ptr swiftself %27, ptr %27, ptr %344)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2)
  call void %342(ptr noalias nonnull %25, i32 %340, ptr %22) #9
  %347 = call swiftcc ptr @"$sSo17OS_dispatch_queueC8DispatchE5label3qos10attributes20autoreleaseFrequency6targetABSS_AC0D3QoSVAbCE10AttributesVAbCE011AutoreleaseI0OABSgtcfC"(i64 5865207757966370157, ptr nonnull inttoptr (i64 -1441151879056366219 to ptr), ptr noalias nocapture nonnull %35, ptr noalias nocapture nonnull %30, ptr noalias nocapture nonnull %25, i64 0, ptr swiftself %339)
  store ptr %347, ptr @"$s4main12messageQueueSo17OS_dispatch_queueCvp", align 8
  %348 = load i64, ptr @"$s4main5depthSivp", align 8
  %349 = call swiftcc i64 @"$s4main12bottomUpTreeyAA0D4NodeCSgSiF"(i64 %348)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %4)
  store i64 0, ptr %4, align 8
  %._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %4, i64 0, i32 1
  store ptr inttoptr (i64 -2305843009213693952 to ptr), ptr %._object._object, align 8
  call swiftcc void @"$ss11_StringGutsV4growyySiF"(i64 35, ptr nocapture nonnull swiftself dereferenceable(16) %4)
  %350 = load ptr, ptr %._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %4)
  call void @swift_bridgeObjectRelease(ptr %350) #9
  store i64 -3458764513820540906, ptr %3, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %3, i64 0, i32 1
  store ptr inttoptr (i64 or (i64 sub (i64 ptrtoint (ptr @".str.22.stretch tree of depth " to i64), i64 32), i64 -9223372036854775808) to ptr), ptr %._guts._object._object, align 8
  %351 = load i64, ptr @"$s4main5depthSivp", align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5)
  store i64 %351, ptr %5, align 8
  %352 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %5, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %353 = extractvalue { i64, ptr } %352, 0
  %354 = extractvalue { i64, ptr } %352, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %353, ptr %354, ptr nocapture nonnull swiftself dereferenceable(16) %3)
  call void @swift_bridgeObjectRelease(ptr %354) #9
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5)
  call swiftcc void @"$sSS6appendyySSF"(i64 4209567563902033929, ptr nonnull inttoptr (i64 -1657324662872342496 to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %3)
  %355 = inttoptr i64 %349 to ptr
  %356 = call swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr swiftself %355)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6)
  store i64 %356, ptr %6, align 8
  %357 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %6, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %358 = extractvalue { i64, ptr } %357, 0
  %359 = extractvalue { i64, ptr } %357, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %358, ptr %359, ptr nocapture nonnull swiftself dereferenceable(16) %3)
  call void @swift_bridgeObjectRelease(ptr %359) #9
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6)
  %360 = load i64, ptr %3, align 8
  %361 = load ptr, ptr %._guts._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3)
  call swiftcc void @"$sSD8_VariantV8setValue_6forKeyyq_n_xtFSi_SSTg5"(i64 %360, ptr %361, i64 0, ptr nocapture nonnull swiftself dereferenceable(8) @"$s4main8messagesSDySiSSGvp")
  call void @swift_release(ptr %355) #9
  %362 = load i64, ptr @"$s4main8maxDepthSivp", align 8
  %363 = call swiftcc i64 @"$s4main12bottomUpTreeyAA0D4NodeCSgSiF"(i64 %362)
  store i64 %363, ptr @"$s4main13longLivedTreeAA0D4NodeCSgvp", align 8
  %364 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  call void @dispatch_group_enter(ptr %364)
  %365 = load ptr, ptr @"$s4main11workerQueueSo17OS_dispatch_queueCvp", align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %7)
  %366 = getelementptr inbounds { %objc_block, %swift.function }, ptr %7, i64 0, i32 1
  store ptr @"$s4mainyyYbcfU0_", ptr %366, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %7, i64 0, i32 1, i32 1
  store ptr null, ptr %.data, align 8
  store ptr @_NSConcreteStackBlock, ptr %7, align 8
  %367 = getelementptr inbounds %objc_block, ptr %7, i64 0, i32 1
  store <2 x i32> <i32 1107296256, i32 0>, ptr %367, align 8
  %368 = getelementptr inbounds %objc_block, ptr %7, i64 0, i32 3
  store ptr @"$sIegh_IeyBh_TR", ptr %368, align 8
  %369 = getelementptr inbounds %objc_block, ptr %7, i64 0, i32 4
  store ptr @block_descriptor, ptr %369, align 8
  %370 = call ptr @_Block_copy(ptr nonnull %7)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %35)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %8)
  store ptr @_swiftEmptyArrayStorage, ptr %8, align 8
  %371 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL", ptr @"$s8Dispatch0A13WorkItemFlagsVMa", ptr @"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc")
  %372 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSay8Dispatch0A13WorkItemFlagsVGMD") #23
  %373 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL", ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %20, ptr noalias nocapture nonnull %8, ptr %372, ptr %373, ptr swiftself %17, ptr %17, ptr %371)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %8)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %35, ptr noalias nocapture nonnull %20, ptr %370, ptr swiftself %365)
  %374 = getelementptr inbounds ptr, ptr %.valueWitnesses, i64 1
  %375 = load ptr, ptr %374, align 8, !invariant.load !33
  call void %375(ptr noalias nonnull %20, ptr %17) #9
  %376 = getelementptr inbounds ptr, ptr %.valueWitnesses5, i64 1
  %377 = load ptr, ptr %376, align 8, !invariant.load !33
  call void %377(ptr noalias nonnull %35, ptr %32) #9
  call void @_Block_release(ptr %370) #9
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %7)
  %378 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  call swiftcc void @"$sSo17OS_dispatch_groupC8DispatchE4waityyF"(ptr swiftself %378)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %9)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %10)
  store i64 0, ptr %10, align 8
  %._object15._object = getelementptr inbounds %Ts13_StringObjectV, ptr %10, i64 0, i32 1
  store ptr inttoptr (i64 -2305843009213693952 to ptr), ptr %._object15._object, align 8
  call swiftcc void @"$ss11_StringGutsV4growyySiF"(i64 38, ptr nocapture nonnull swiftself dereferenceable(16) %10)
  %379 = load ptr, ptr %._object15._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %10)
  call void @swift_bridgeObjectRelease(ptr %379) #9
  store i64 -3458764513820540903, ptr %9, align 8
  %._guts18._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %9, i64 0, i32 1
  store ptr inttoptr (i64 or (i64 sub (i64 ptrtoint (ptr @".str.25.long lived tree of depth " to i64), i64 32), i64 -9223372036854775808) to ptr), ptr %._guts18._object._object, align 8
  %380 = load i64, ptr @"$s4main8maxDepthSivp", align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %11)
  store i64 %380, ptr %11, align 8
  %381 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %11, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %382 = extractvalue { i64, ptr } %381, 0
  %383 = extractvalue { i64, ptr } %381, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %382, ptr %383, ptr nocapture nonnull swiftself dereferenceable(16) %9)
  call void @swift_bridgeObjectRelease(ptr %383) #9
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %11)
  call swiftcc void @"$sSS6appendyySSF"(i64 4209567563902033929, ptr nonnull inttoptr (i64 -1657324662872342496 to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %9)
  %384 = load i64, ptr @"$s4main13longLivedTreeAA0D4NodeCSgvp", align 8
  %385 = inttoptr i64 %384 to ptr
  %386 = call swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr swiftself %385)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %12)
  store i64 %386, ptr %12, align 8
  %387 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %12, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %388 = extractvalue { i64, ptr } %387, 0
  %389 = extractvalue { i64, ptr } %387, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %388, ptr %389, ptr nocapture nonnull swiftself dereferenceable(16) %9)
  call void @swift_bridgeObjectRelease(ptr %389) #9
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %12)
  %390 = load i64, ptr %9, align 8
  %391 = load ptr, ptr %._guts18._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %9)
  call swiftcc void @"$sSD8_VariantV8setValue_6forKeyyq_n_xtFSi_SSTg5"(i64 %390, ptr %391, i64 9223372036854775807, ptr nocapture nonnull swiftself dereferenceable(8) @"$s4main8messagesSDySiSSGvp")
  %392 = load ptr, ptr @"$s4main8messagesSDySiSSGvp", align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %13)
  %393 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %392, i64 0, i32 1
  %394 = load i64, ptr %393, align 8, !range !38
  %395 = icmp eq i64 %394, 0
  br i1 %395, label %396, label %398

396:                                              ; preds = %332
  %397 = call ptr @swift_bridgeObjectRetain(ptr nonnull returned %392) #9
  br label %413

398:                                              ; preds = %332
  %399 = call swiftcc ptr @"$ss22_ContiguousArrayBufferV19_uninitializedCount15minimumCapacityAByxGSi_SitcfCSi3key_SS5valuet_Tgm5"(i64 %394, i64 0)
  %400 = getelementptr inbounds i8, ptr %399, i64 32
  %401 = ptrtoint ptr %400 to i64
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %14)
  %402 = call swiftcc i64 @"$sSTsE21_copySequenceContents12initializing8IteratorQz_SitSry7ElementQzG_tFSDySiSSG_Tg5"(ptr noalias nocapture nonnull %14, i64 %401, i64 %394, ptr nonnull %392)
  %403 = load i64, ptr %14, align 8
  %404 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %14, i64 0, i32 1
  %405 = load i64, ptr %404, align 8
  %406 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %14, i64 0, i32 2
  %407 = load i64, ptr %406, align 8
  %408 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %14, i64 0, i32 3
  %409 = load i64, ptr %408, align 8
  %410 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %14, i64 0, i32 4
  %411 = load i64, ptr %410, align 8
  %412 = call ptr @swift_bridgeObjectRetain_n(ptr %392, i32 3)
  call void @"$sSD8IteratorV8_VariantOyxq___GSHRzr0_lWOe"(i64 %403, i64 %405, i64 %407, i64 %409, i64 %411)
  call void @swift_bridgeObjectRelease(ptr nonnull %392) #9
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %14)
  br label %413

413:                                              ; preds = %396, %398
  %414 = phi ptr [ %399, %398 ], [ @_swiftEmptyArrayStorage, %396 ]
  store ptr %414, ptr %13, align 8
  call swiftcc void @"$sSMsSkRzrlE4sort2byySb7ElementSTQz_ADtKXE_tKFs15ContiguousArrayVySi3key_SS5valuetG_Tg543$s4mainSbSi3key_SS5valuet_SiAB_SSACttXEfU1_Tf1cn_n"(ptr nocapture nonnull dereferenceable(8) %13, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %swifterror)
  %415 = load ptr, ptr %swifterror, align 8
  %.not = icmp eq ptr %415, null
  br i1 %.not, label %416, label %421

416:                                              ; preds = %413
  call void @swift_bridgeObjectRelease(ptr nonnull %392) #9
  %417 = load ptr, ptr %13, align 8
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %13)
  %418 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %417, i64 0, i32 1
  %419 = load i64, ptr %418, align 8, !range !38
  %420 = icmp eq i64 %419, 0
  br i1 %420, label %.loopexit291, label %423

421:                                              ; preds = %413
  %422 = load ptr, ptr %13, align 8
  call void @swift_release(ptr %422) #9
  unreachable

423:                                              ; preds = %416
  %424 = getelementptr inbounds i8, ptr %417, i64 32
  br label %425

.loopexit291:                                     ; preds = %425, %416
  call void @swift_release(ptr nonnull %417) #9
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %35)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %30)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %25)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %20)
  ret i32 0

425:                                              ; preds = %425, %423
  %426 = phi i64 [ 0, %423 ], [ %430, %425 ]
  %427 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %424, i64 %426
  %.value = getelementptr inbounds <{ %TSi, %TSS }>, ptr %427, i64 0, i32 1
  %428 = load i64, ptr %.value, align 8
  %.value._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.value, i64 0, i32 1
  %429 = load ptr, ptr %.value._guts._object._object, align 8
  %430 = add nuw nsw i64 %426, 1
  %431 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCyypGMD") #23
  %432 = call noalias ptr @swift_allocObject(ptr %431, i64 64, i64 7) #9
  %433 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %432, i64 0, i32 1
  store <2 x i64> <i64 1, i64 2>, ptr %433, align 8
  %434 = getelementptr inbounds i8, ptr %432, i64 32
  %435 = getelementptr inbounds i8, ptr %432, i64 56
  store ptr @"$sSSN", ptr %435, align 8
  store i64 %428, ptr %434, align 8
  %._guts28._object._object = getelementptr inbounds i8, ptr %432, i64 40
  store ptr %429, ptr %._guts28._object._object, align 8
  %436 = call ptr @swift_bridgeObjectRetain_n(ptr %429, i32 2)
  call swiftcc void @"$ss5print_9separator10terminatoryypd_S2StF"(ptr %432, i64 32, ptr nonnull inttoptr (i64 -2233785415175766016 to ptr), i64 10, ptr nonnull inttoptr (i64 -2233785415175766016 to ptr))
  call void @swift_bridgeObjectRelease(ptr %429) #9
  call void @swift_release(ptr %432) #9
  %437 = icmp eq i64 %430, %419
  br i1 %437, label %.loopexit291, label %425
}

declare swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64) local_unnamed_addr #0

; Function Attrs: argmemonly mustprogress nocallback nofree nosync nounwind willreturn
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #1

declare swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE20AutoreleaseFrequencyOMa"(i64) local_unnamed_addr #0

declare swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMa"(i64) local_unnamed_addr #0

declare swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64) local_unnamed_addr #0

declare swiftcc i32 @"$ss11CommandLineO4argcs5Int32VvgZ"() local_unnamed_addr #0

; Function Attrs: mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64) #2

; Function Attrs: nounwind
define hidden swiftcc i64 @"$s4main12bottomUpTreeyAA0D4NodeCSgSiF"(i64 %0) local_unnamed_addr #3 {
entry:
  %1 = icmp sgt i64 %0, 0
  br i1 %1, label %2, label %11

2:                                                ; preds = %entry
  %3 = add nsw i64 %0, -1
  %4 = tail call swiftcc i64 @"$s4main12bottomUpTreeyAA0D4NodeCSgSiF"(i64 %3)
  %5 = tail call swiftcc i64 @"$s4main12bottomUpTreeyAA0D4NodeCSgSiF"(i64 %3)
  %6 = tail call swiftcc %swift.metadata_response @"$s4main8TreeNodeCMa"(i64 poison) #22
  %7 = extractvalue %swift.metadata_response %6, 0
  %8 = tail call noalias ptr @swift_allocObject(ptr %7, i64 32, i64 7) #9
  %9 = getelementptr inbounds %T4main8TreeNodeC, ptr %8, i64 0, i32 1
  %10 = getelementptr inbounds %T4main8TreeNodeC, ptr %8, i64 0, i32 2
  store i64 %4, ptr %9, align 8
  store i64 %5, ptr %10, align 8
  br label %16

11:                                               ; preds = %entry
  %12 = tail call swiftcc %swift.metadata_response @"$s4main8TreeNodeCMa"(i64 poison) #22
  %13 = extractvalue %swift.metadata_response %12, 0
  %14 = tail call noalias ptr @swift_allocObject(ptr %13, i64 32, i64 7) #9
  %15 = getelementptr inbounds %T4main8TreeNodeC, ptr %14, i64 0, i32 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %15, i8 0, i64 16, i1 false)
  br label %16

16:                                               ; preds = %2, %11
  %.in = phi ptr [ %14, %11 ], [ %8, %2 ]
  %17 = ptrtoint ptr %.in to i64
  ret i64 %17
}

define internal swiftcc void @"$s4mainyyYbcfU0_"() #0 {
entry:
  %0 = alloca { %objc_block, %swift.function }, align 8
  %1 = alloca %TSa, align 8
  %2 = alloca { %objc_block, %swift.function }, align 8
  %3 = alloca %TSa, align 8
  %4 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64 0) #22
  %5 = extractvalue %swift.metadata_response %4, 0
  %6 = getelementptr inbounds ptr, ptr %5, i64 -1
  %.valueWitnesses = load ptr, ptr %6, align 8, !invariant.load !33, !dereferenceable !34
  %7 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses, i64 0, i32 8
  %size = load i64, ptr %7, align 8, !invariant.load !33
  %8 = alloca i8, i64 %size, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %8)
  %9 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64 0) #22
  %10 = extractvalue %swift.metadata_response %9, 0
  %11 = getelementptr inbounds ptr, ptr %10, i64 -1
  %.valueWitnesses1 = load ptr, ptr %11, align 8, !invariant.load !33, !dereferenceable !34
  %12 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses1, i64 0, i32 8
  %size2 = load i64, ptr %12, align 8, !invariant.load !33
  %13 = alloca i8, i64 %size2, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %13)
  %14 = load i64, ptr @"$s4main8minDepthSivp", align 8
  %15 = sdiv i64 %14, 2
  %16 = load i64, ptr @"$s4main8maxDepthSivp", align 8
  %17 = sdiv i64 %16, 2
  %18 = add nsw i64 %17, 1
  %19 = icmp eq i64 %15, %18
  br i1 %19, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %entry
  %20 = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1, i32 1
  %21 = getelementptr inbounds %objc_block, ptr %0, i64 0, i32 1
  %22 = getelementptr inbounds %objc_block, ptr %0, i64 0, i32 3
  %23 = getelementptr inbounds %objc_block, ptr %0, i64 0, i32 4
  %24 = getelementptr inbounds ptr, ptr %.valueWitnesses, i64 1
  %25 = getelementptr inbounds ptr, ptr %.valueWitnesses1, i64 1
  br label %39

.loopexit:                                        ; preds = %39, %entry
  %26 = load ptr, ptr @"$s4main12messageQueueSo17OS_dispatch_queueCvp", align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %2)
  %27 = getelementptr inbounds { %objc_block, %swift.function }, ptr %2, i64 0, i32 1
  store ptr @"$s4mainyyYbcfU0_yyYbcfU0_", ptr %27, align 8
  %.data7 = getelementptr inbounds { %objc_block, %swift.function }, ptr %2, i64 0, i32 1, i32 1
  store ptr null, ptr %.data7, align 8
  store ptr @_NSConcreteStackBlock, ptr %2, align 8
  %28 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 1
  store <2 x i32> <i32 1107296256, i32 0>, ptr %28, align 8
  %29 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 3
  store ptr @"$sIegh_IeyBh_TR", ptr %29, align 8
  %30 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 4
  store ptr @block_descriptor.6, ptr %30, align 8
  %31 = call ptr @_Block_copy(ptr nonnull %2)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %13)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3)
  store ptr @_swiftEmptyArrayStorage, ptr %3, align 8
  %32 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL", ptr @"$s8Dispatch0A13WorkItemFlagsVMa", ptr @"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc")
  %33 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSay8Dispatch0A13WorkItemFlagsVGMD") #23
  %34 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL", ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %8, ptr noalias nocapture nonnull %3, ptr %33, ptr %34, ptr swiftself %5, ptr %5, ptr %32)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %13, ptr noalias nocapture nonnull %8, ptr %31, ptr swiftself %26)
  %35 = getelementptr inbounds ptr, ptr %.valueWitnesses, i64 1
  %36 = load ptr, ptr %35, align 8, !invariant.load !33
  call void %36(ptr noalias nonnull %8, ptr %5) #9
  %37 = getelementptr inbounds ptr, ptr %.valueWitnesses1, i64 1
  %38 = load ptr, ptr %37, align 8, !invariant.load !33
  call void %38(ptr noalias nonnull %13, ptr %10) #9
  call void @_Block_release(ptr %31) #9
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %2)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %13)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %8)
  ret void

39:                                               ; preds = %.preheader, %39
  %40 = phi i64 [ %50, %39 ], [ %15, %.preheader ]
  %41 = shl i64 %40, 1
  %42 = load i64, ptr @"$s4main8maxDepthSivp", align 8
  %43 = sub i64 %42, %41
  %44 = load i64, ptr @"$s4main8minDepthSivp", align 8
  %45 = add i64 %43, %44
  %46 = icmp ugt i64 %45, 64
  %47 = icmp ult i64 %45, 64
  %48 = shl nuw i64 1, %45
  %spec.select = select i1 %47, i64 %48, i64 0
  %49 = select i1 %46, i64 0, i64 %spec.select, !prof !39
  %50 = add i64 %40, 1
  %51 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  call void @dispatch_group_enter(ptr %51)
  %52 = load ptr, ptr @"$s4main11workerQueueSo17OS_dispatch_queueCvp", align 8
  %53 = call noalias ptr @swift_allocObject(ptr nonnull getelementptr inbounds (%swift.full_boxmetadata, ptr @metadata, i64 0, i32 2), i64 32, i64 7) #9
  %54 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi }>, ptr %53, i64 0, i32 1
  store i64 %41, ptr %54, align 8
  %55 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi }>, ptr %53, i64 0, i32 2
  store i64 %49, ptr %55, align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %0)
  store ptr @"$s4mainyyYbcfU0_yyYbcfU_TA", ptr %20, align 8
  store ptr %53, ptr %.data, align 8
  store ptr @_NSConcreteStackBlock, ptr %0, align 8
  store <2 x i32> <i32 1107296256, i32 0>, ptr %21, align 8
  store ptr @"$sIegh_IeyBh_TR", ptr %22, align 8
  store ptr @block_descriptor.3, ptr %23, align 8
  %56 = call ptr @_Block_copy(ptr nonnull %0)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %13)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %1)
  store ptr @_swiftEmptyArrayStorage, ptr %1, align 8
  %57 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL", ptr @"$s8Dispatch0A13WorkItemFlagsVMa", ptr @"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc")
  %58 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSay8Dispatch0A13WorkItemFlagsVGMD") #23
  %59 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL", ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %8, ptr noalias nocapture nonnull %1, ptr %58, ptr %59, ptr swiftself %5, ptr %5, ptr %57)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %1)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %13, ptr noalias nocapture nonnull %8, ptr %56, ptr swiftself %52)
  %60 = load ptr, ptr %24, align 8, !invariant.load !33
  call void %60(ptr noalias nonnull %8, ptr %5) #9
  %61 = load ptr, ptr %25, align 8, !invariant.load !33
  call void %61(ptr noalias nonnull %13, ptr %10) #9
  call void @_Block_release(ptr %56) #9
  %62 = load ptr, ptr %.data, align 8
  call void @swift_release(ptr %62) #9
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %0)
  %63 = icmp eq i64 %40, %17
  br i1 %63, label %.loopexit, label %39
}

define internal swiftcc void @"$s4mainyyYbcfU0_yyYbcfU_"(i64 %0, i64 %1) unnamed_addr #0 {
entry:
  %2 = alloca { %objc_block, %swift.function }, align 8
  %3 = alloca %TSa, align 8
  %4 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64 0) #22
  %5 = extractvalue %swift.metadata_response %4, 0
  %6 = getelementptr inbounds ptr, ptr %5, i64 -1
  %.valueWitnesses = load ptr, ptr %6, align 8, !invariant.load !33, !dereferenceable !34
  %7 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses, i64 0, i32 8
  %size = load i64, ptr %7, align 8, !invariant.load !33
  %8 = alloca i8, i64 %size, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %8)
  %9 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64 0) #22
  %10 = extractvalue %swift.metadata_response %9, 0
  %11 = getelementptr inbounds ptr, ptr %10, i64 -1
  %.valueWitnesses1 = load ptr, ptr %11, align 8, !invariant.load !33, !dereferenceable !34
  %12 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses1, i64 0, i32 8
  %size2 = load i64, ptr %12, align 8, !invariant.load !33
  %13 = alloca i8, i64 %size2, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %13)
  %14 = tail call swiftcc { i64, ptr } @"$s4main5inner5depth10iterationsSSSi_SitF"(i64 %0, i64 %1)
  %15 = extractvalue { i64, ptr } %14, 0
  %16 = extractvalue { i64, ptr } %14, 1
  %17 = load ptr, ptr @"$s4main12messageQueueSo17OS_dispatch_queueCvp", align 8
  %18 = tail call noalias ptr @swift_allocObject(ptr nonnull getelementptr inbounds (%swift.full_boxmetadata, ptr @metadata.9, i64 0, i32 2), i64 40, i64 7) #9
  %19 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSS }>, ptr %18, i64 0, i32 1
  store i64 %0, ptr %19, align 8
  %20 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSS }>, ptr %18, i64 0, i32 2
  store i64 %15, ptr %20, align 8
  %._guts._object._object = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSS }>, ptr %18, i64 0, i32 2, i32 0, i32 0, i32 1
  store ptr %16, ptr %._guts._object._object, align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %2)
  %21 = getelementptr inbounds { %objc_block, %swift.function }, ptr %2, i64 0, i32 1
  store ptr @"$s4mainyyYbcfU0_yyYbcfU_yyYbcfU_TA", ptr %21, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %2, i64 0, i32 1, i32 1
  store ptr %18, ptr %.data, align 8
  store ptr @_NSConcreteStackBlock, ptr %2, align 8
  %22 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 1
  store <2 x i32> <i32 1107296256, i32 0>, ptr %22, align 8
  %23 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 3
  store ptr @"$sIegh_IeyBh_TR", ptr %23, align 8
  %24 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 4
  store ptr @block_descriptor.12, ptr %24, align 8
  %25 = call ptr @_Block_copy(ptr nonnull %2)
  %26 = call ptr @swift_bridgeObjectRetain(ptr returned %16) #9
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %13)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3)
  store ptr @_swiftEmptyArrayStorage, ptr %3, align 8
  %27 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL", ptr @"$s8Dispatch0A13WorkItemFlagsVMa", ptr @"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc")
  %28 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSay8Dispatch0A13WorkItemFlagsVGMD") #23
  %29 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL", ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %8, ptr noalias nocapture nonnull %3, ptr %28, ptr %29, ptr swiftself %5, ptr %5, ptr %27)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %13, ptr noalias nocapture nonnull %8, ptr %25, ptr swiftself %17)
  %30 = getelementptr inbounds ptr, ptr %.valueWitnesses, i64 1
  %31 = load ptr, ptr %30, align 8, !invariant.load !33
  call void %31(ptr noalias nonnull %8, ptr %5) #9
  %32 = getelementptr inbounds ptr, ptr %.valueWitnesses1, i64 1
  %33 = load ptr, ptr %32, align 8, !invariant.load !33
  call void %33(ptr noalias nonnull %13, ptr %10) #9
  call void @_Block_release(ptr %25) #9
  call void @swift_bridgeObjectRelease(ptr %16) #9
  %34 = load ptr, ptr %.data, align 8
  call void @swift_release(ptr %34) #9
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %2)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %13)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %8)
  ret void
}

define hidden swiftcc { i64, ptr } @"$s4main5inner5depth10iterationsSSSi_SitF"(i64 %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = alloca %TSS, align 8
  %3 = alloca %Ts11_StringGutsV, align 8
  %4 = alloca %TSi, align 8
  %5 = alloca %TSi, align 8
  %6 = alloca %TSi, align 8
  %7 = icmp eq i64 %1, 0
  br i1 %7, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %entry, %.preheader
  %8 = phi i64 [ %14, %.preheader ], [ 0, %entry ]
  %9 = phi i64 [ %10, %.preheader ], [ 0, %entry ]
  %10 = add nuw i64 %9, 1
  %11 = tail call swiftcc i64 @"$s4main12bottomUpTreeyAA0D4NodeCSgSiF"(i64 %0)
  %12 = inttoptr i64 %11 to ptr
  %13 = tail call swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr swiftself %12)
  tail call void @swift_release(ptr %12) #9
  %14 = add i64 %13, %8
  %15 = icmp eq i64 %10, %1
  br i1 %15, label %.loopexit, label %.preheader

.loopexit:                                        ; preds = %.preheader, %entry
  %16 = phi i64 [ 0, %entry ], [ %14, %.preheader ]
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %2)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3)
  store i64 0, ptr %3, align 8
  %._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %3, i64 0, i32 1
  store ptr inttoptr (i64 -2305843009213693952 to ptr), ptr %._object._object, align 8
  call swiftcc void @"$ss11_StringGutsV4growyySiF"(i64 32, ptr nocapture nonnull swiftself dereferenceable(16) %3)
  %17 = load ptr, ptr %._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4)
  store i64 %1, ptr %4, align 8
  %18 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %4, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  tail call void @swift_bridgeObjectRelease(ptr %17) #9
  %19 = extractvalue { i64, ptr } %18, 0
  %20 = extractvalue { i64, ptr } %18, 1
  store i64 %19, ptr %2, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %2, i64 0, i32 1
  store ptr %20, ptr %._guts._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4)
  call swiftcc void @"$sSS6appendyySSF"(i64 -3458764513820540911, ptr nonnull inttoptr (i64 or (i64 sub (i64 ptrtoint (ptr @".str.17.\09 trees of depth " to i64), i64 32), i64 -9223372036854775808) to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %2)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5)
  store i64 %0, ptr %5, align 8
  %21 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %5, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %22 = extractvalue { i64, ptr } %21, 0
  %23 = extractvalue { i64, ptr } %21, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %22, ptr %23, ptr nocapture nonnull swiftself dereferenceable(16) %2)
  tail call void @swift_bridgeObjectRelease(ptr %23) #9
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5)
  call swiftcc void @"$sSS6appendyySSF"(i64 4209567563902033929, ptr nonnull inttoptr (i64 -1657324662872342496 to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %2)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6)
  store i64 %16, ptr %6, align 8
  %24 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %6, ptr nonnull @"$sSiN", ptr nonnull @"$sSis23CustomStringConvertiblesWP")
  %25 = extractvalue { i64, ptr } %24, 0
  %26 = extractvalue { i64, ptr } %24, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %25, ptr %26, ptr nocapture nonnull swiftself dereferenceable(16) %2)
  tail call void @swift_bridgeObjectRelease(ptr %26) #9
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6)
  %27 = load i64, ptr %2, align 8
  %28 = load ptr, ptr %._guts._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %2)
  %29 = insertvalue { i64, ptr } undef, i64 %27, 0
  %30 = insertvalue { i64, ptr } %29, ptr %28, 1
  ret { i64, ptr } %30
}

define linkonce_odr hidden void @"$sIegh_IeyBh_TR"(ptr %0) #0 {
entry:
  %1 = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1
  %2 = load ptr, ptr %1, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1, i32 1
  %3 = load ptr, ptr %.data, align 8
  %4 = tail call ptr @swift_retain(ptr returned %3) #24
  tail call swiftcc void %2(ptr swiftself %3) #25
  tail call void @swift_release(ptr %3) #9
  ret void
}

; Function Attrs: nounwind
define internal swiftcc void @"$s4mainyyYbcfU0_yyYbcfU0_"() #3 {
entry:
  %0 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  tail call void @dispatch_group_leave(ptr %0)
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
  %6 = tail call ptr @swift_retain(ptr nonnull returned %5) #24
  %7 = tail call swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr nonnull swiftself %5)
  tail call void @swift_release(ptr nonnull %5) #9
  %8 = getelementptr inbounds %T4main8TreeNodeC, ptr %0, i64 0, i32 2
  %9 = load i64, ptr %8, align 8
  %10 = inttoptr i64 %9 to ptr
  %11 = tail call ptr @swift_retain(ptr returned %10) #24
  %12 = tail call swiftcc i64 @"$s4main8TreeNodeC5checkSiyF"(ptr swiftself %10)
  tail call void @swift_release(ptr %10) #9
  %13 = add i64 %7, 1
  %14 = add i64 %13, %12
  br label %common.ret1
}

; Function Attrs: nounwind
define hidden swiftcc void @"$s4main8TreeNodeCfD"(ptr swiftself %0) #3 {
entry:
  %1 = getelementptr inbounds %T4main8TreeNodeC, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8
  %3 = inttoptr i64 %2 to ptr
  tail call void @swift_release(ptr %3) #9
  %4 = getelementptr inbounds %T4main8TreeNodeC, ptr %0, i64 0, i32 2
  %5 = load i64, ptr %4, align 8
  %6 = inttoptr i64 %5 to ptr
  tail call void @swift_release(ptr %6) #9
  tail call void @swift_deallocClassInstance(ptr %0, i64 32, i64 7) #9
  ret void
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc { i64, i8 } @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFSS_SiTg5"(i64 %0, ptr %1, i64 %2) local_unnamed_addr #4 {
entry:
  %3 = alloca %TSS, align 8
  %4 = alloca <{ %Ts6UInt64V, %Ts6UInt64V }>, align 8
  %swifterror = alloca swifterror ptr, align 8
  store ptr null, ptr %swifterror, align 8
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3)
  store i64 %0, ptr %3, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %3, i64 0, i32 1
  store ptr %1, ptr %._guts._object._object, align 8
  %5 = tail call ptr @swift_bridgeObjectRetain(ptr returned %1) #9
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
  tail call void @swift_bridgeObjectRelease(ptr %8) #9
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
  call void @swift_bridgeObjectRelease(ptr %17) #9
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
  tail call void @swift_bridgeObjectRelease(ptr %17) #9
  br label %44

44:                                               ; preds = %39, %24
  %.pn = phi { i64, i8 } [ %29, %24 ], [ %42, %39 ]
  %.in.in = extractvalue { i64, i8 } %.pn, 1
  %.in = and i8 %.in.in, 1
  %45 = insertvalue { i64, i8 } %.pn, i8 %.in, 1
  ret { i64, i8 } %45
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4findys10_HashTableV6BucketV6bucket_Sb5foundtxSHRzlFSi_Tg5"(i64 %0, ptr swiftself %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %1, i64 0, i32 7
  %3 = load i64, ptr %2, align 8
  %4 = tail call swiftcc i64 @"$ss6HasherV5_hash4seed_S2i_s6UInt64VtFZ"(i64 %3, i64 %0)
  %5 = tail call swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4find_9hashValues10_HashTableV6BucketV6bucket_Sb5foundtx_SitSHRzlFSi_Tg5"(i64 %0, i64 %4, ptr swiftself %1)
  ret { i64, i1 } %5
}

define linkonce_odr hidden swiftcc void @"$sSD8_VariantV8setValue_6forKeyyq_n_xtFSi_SSTg5"(i64 %0, ptr %1, i64 %2, ptr nocapture swiftself dereferenceable(8) %3) local_unnamed_addr #0 {
entry:
  %4 = alloca %Ts17_NativeDictionaryV, align 8
  %5 = load ptr, ptr %3, align 8
  %6 = tail call zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr %5) #24
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4)
  store ptr %5, ptr %4, align 8
  store ptr inttoptr (i64 -9223372036854775808 to ptr), ptr %3, align 8
  %7 = call swiftcc { i64, i1 } @"$ss17_NativeDictionaryV12mutatingFind_8isUniques10_HashTableV6BucketV6bucket_Sb5foundtx_SbtFSi_SSTg5"(i64 %2, i1 %6, ptr nocapture nonnull swiftself dereferenceable(8) %4)
  %8 = extractvalue { i64, i1 } %7, 0
  %9 = extractvalue { i64, i1 } %7, 1
  %10 = load ptr, ptr %4, align 8
  br i1 %9, label %11, label %16

11:                                               ; preds = %entry
  %12 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %10, i64 0, i32 9
  %13 = load ptr, ptr %12, align 8
  %14 = getelementptr inbounds %TSS, ptr %13, i64 %8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %14, i64 0, i32 1
  %15 = load ptr, ptr %._guts._object._object, align 8
  tail call void @swift_bridgeObjectRelease(ptr %15) #9
  store i64 %0, ptr %14, align 8
  store ptr %1, ptr %._guts._object._object, align 8
  br label %33

16:                                               ; preds = %entry
  %17 = getelementptr inbounds i8, ptr %10, i64 64
  %18 = lshr i64 %8, 6
  %19 = and i64 %8, 63
  %20 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %17, i64 %18
  %21 = shl nuw i64 1, %19
  %22 = load i64, ptr %20, align 8
  %23 = or i64 %22, %21
  store i64 %23, ptr %20, align 8
  %24 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %10, i64 0, i32 8
  %25 = load ptr, ptr %24, align 8
  %26 = getelementptr inbounds %TSi, ptr %25, i64 %8
  store i64 %2, ptr %26, align 8
  %27 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %10, i64 0, i32 9
  %28 = load ptr, ptr %27, align 8
  %29 = getelementptr inbounds %TSS, ptr %28, i64 %8
  store i64 %0, ptr %29, align 8
  %._guts._object._object.i = getelementptr inbounds %Ts13_StringObjectV, ptr %29, i64 0, i32 1
  store ptr %1, ptr %._guts._object._object.i, align 8
  %30 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %10, i64 0, i32 1
  %31 = load i64, ptr %30, align 8
  %32 = add i64 %31, 1
  store i64 %32, ptr %30, align 8
  br label %33

33:                                               ; preds = %11, %16
  %34 = load ptr, ptr %3, align 8
  store ptr %10, ptr %3, align 8
  tail call void @swift_bridgeObjectRelease(ptr %34) #9
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4)
  ret void
}

define linkonce_odr hidden swiftcc void @"$sSMsSkRzrlE4sort2byySb7ElementSTQz_ADtKXE_tKFs15ContiguousArrayVySi3key_SS5valuetG_Tg543$s4mainSbSi3key_SS5valuet_SiAB_SSACttXEfU1_Tf1cn_n"(ptr nocapture dereferenceable(8) %0, ptr swiftself %1, ptr noalias nocapture swifterror dereferenceable(8) %2) local_unnamed_addr #0 {
entry:
  %3 = alloca %TSrySi3key_SS5valuetG, align 8
  %4 = alloca %TSi, align 8
  %5 = alloca %TSrySi3key_SS5valuetG, align 8
  %6 = load ptr, ptr %0, align 8
  %7 = tail call zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr %6) #24
  br i1 %7, label %10, label %8

8:                                                ; preds = %entry
  %9 = tail call swiftcc ptr @"$ss22_ContiguousArrayBufferV20_consumeAndCreateNewAByxGyFSi3key_SS5valuet_Tg5"(ptr %6)
  br label %10

10:                                               ; preds = %entry, %8
  %11 = phi ptr [ %9, %8 ], [ %6, %entry ]
  %12 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %11, i64 0, i32 1
  %13 = load i64, ptr %12, align 8, !range !38
  %14 = getelementptr inbounds i8, ptr %11, i64 32
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3)
  %15 = ptrtoint ptr %14 to i64
  store i64 %15, ptr %3, align 8
  %.count = getelementptr inbounds %TSrySi3key_SS5valuetG, ptr %3, i64 0, i32 1
  store i64 %13, ptr %.count, align 8
  %16 = tail call swiftcc i64 @"$ss22_minimumMergeRunLengthyS2iF"(i64 %13)
  %17 = icmp slt i64 %16, %13
  br i1 %17, label %18, label %20

18:                                               ; preds = %10
  %19 = lshr i64 %13, 1
  %.not = icmp ult i64 %13, 2
  br i1 %.not, label %25, label %21

20:                                               ; preds = %10
  %switch = icmp ult i64 %13, 2
  br i1 %switch, label %.loopexit, label %.preheader

21:                                               ; preds = %18
  %22 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSi3key_SS5valuetMD") #23
  %23 = tail call swiftcc ptr @"$sSa28_allocateBufferUninitialized15minimumCapacitys06_ArrayB0VyxGSi_tFZ"(i64 %19, ptr %22)
  %24 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %23, i64 0, i32 1
  store i64 %19, ptr %24, align 8
  br label %25

25:                                               ; preds = %18, %21
  %26 = phi ptr [ %23, %21 ], [ @_swiftEmptyArrayStorage, %18 ]
  %27 = getelementptr inbounds i8, ptr %26, i64 32
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %5)
  %28 = ptrtoint ptr %27 to i64
  store i64 %28, ptr %5, align 8
  %.count21 = getelementptr inbounds %TSrySi3key_SS5valuetG, ptr %5, i64 0, i32 1
  store i64 %19, ptr %.count21, align 8
  call swiftcc void @"$sSr15_stableSortImpl2byySbx_xtKXE_tKFySryxGz_SiztKXEfU_Si3key_SS5valuet_Tg543$s4mainSbSi3key_SS5valuet_SiAB_SSACttXEfU1_Tf1nnncn_n"(ptr nocapture nonnull dereferenceable(16) %5, ptr nocapture nonnull dereferenceable(8) %4, ptr nocapture nonnull dereferenceable(16) %3, i64 %16, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %2)
  %29 = load ptr, ptr %2, align 8
  %.not41 = icmp eq ptr %29, null
  %30 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %26, i64 0, i32 1
  store i64 0, ptr %30, align 8
  tail call void @swift_bridgeObjectRelease(ptr %26) #9
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %5)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4)
  br i1 %.not41, label %.loopexit, label %31

31:                                               ; preds = %25
  store ptr %11, ptr %0, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3)
  br label %common.ret

.loopexit:                                        ; preds = %44, %25, %20
  store ptr %11, ptr %0, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3)
  br label %common.ret

.preheader:                                       ; preds = %20, %44
  %32 = phi i64 [ %45, %44 ], [ 1, %20 ]
  %.phi.trans.insert = getelementptr inbounds <{ %TSi, %TSS }>, ptr %14, i64 %32
  %.pre = load i64, ptr %.phi.trans.insert, align 8
  br label %33

33:                                               ; preds = %39, %.preheader
  %34 = phi i64 [ %32, %.preheader ], [ %35, %39 ]
  %35 = add nsw i64 %34, -1
  %36 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %14, i64 %35
  %37 = load i64, ptr %36, align 8
  %38 = icmp slt i64 %.pre, %37
  br i1 %38, label %39, label %44

39:                                               ; preds = %33
  %40 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %14, i64 %34
  %.value16 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %14, i64 %34, i32 1
  %.value16._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.value16, i64 0, i32 1
  %41 = load i64, ptr %.value16, align 8
  %42 = load ptr, ptr %.value16._guts._object._object, align 8
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %40, ptr noundef nonnull align 8 dereferenceable(24) %36, i64 24, i1 false)
  store i64 %.pre, ptr %36, align 8
  %.value19 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %14, i64 %35, i32 1
  store i64 %41, ptr %.value19, align 8
  %.value19._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.value19, i64 0, i32 1
  store ptr %42, ptr %.value19._guts._object._object, align 8
  %43 = icmp eq i64 %35, 0
  br i1 %43, label %44, label %33

44:                                               ; preds = %39, %33
  %45 = add nuw nsw i64 %32, 1
  %46 = icmp eq i64 %45, %13
  br i1 %46, label %.loopexit, label %.preheader

common.ret:                                       ; preds = %.loopexit, %31
  ret void
}

declare swiftcc i64 @"$ss22_minimumMergeRunLengthyS2iF"(i64) local_unnamed_addr #0

define linkonce_odr hidden swiftcc void @"$sSr15_stableSortImpl2byySbx_xtKXE_tKFySryxGz_SiztKXEfU_Si3key_SS5valuet_Tg543$s4mainSbSi3key_SS5valuet_SiAB_SSACttXEfU1_Tf1nnncn_n"(ptr nocapture dereferenceable(16) %0, ptr nocapture dereferenceable(8) %1, ptr nocapture dereferenceable(16) %2, i64 %3, ptr swiftself %4, ptr noalias nocapture swifterror dereferenceable(8) %5) local_unnamed_addr #0 {
entry:
  %6 = alloca %TSa, align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6)
  store ptr @_swiftEmptyArrayStorage, ptr %6, align 8
  %7 = load i64, ptr %2, align 8
  %.count = getelementptr inbounds %TSrySi3key_SS5valuetG, ptr %2, i64 0, i32 1
  %8 = load i64, ptr %.count, align 8
  %9 = icmp sgt i64 %8, 0
  br i1 %9, label %.preheader81, label %.loopexit82

.preheader81:                                     ; preds = %entry, %85
  %10 = phi i64 [ %58, %85 ], [ 0, %entry ]
  %11 = phi i64 [ %86, %85 ], [ %7, %entry ]
  %12 = phi i64 [ %87, %85 ], [ %8, %entry ]
  %13 = add nsw i64 %10, 1
  %14 = icmp slt i64 %13, %12
  br i1 %14, label %15, label %.loopexit78

15:                                               ; preds = %.preheader81
  %16 = inttoptr i64 %11 to ptr
  %17 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %16, i64 %13
  %18 = load i64, ptr %17, align 8
  %19 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %16, i64 %10
  %20 = load i64, ptr %19, align 8
  %21 = icmp slt i64 %18, %20
  %22 = add nsw i64 %10, 2
  %23 = icmp slt i64 %22, %12
  br i1 %23, label %.preheader79, label %.loopexit80

.preheader79:                                     ; preds = %15, %30
  %24 = phi i64 [ %27, %30 ], [ %18, %15 ]
  %25 = phi i64 [ %31, %30 ], [ %22, %15 ]
  %26 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %16, i64 %25
  %27 = load i64, ptr %26, align 8
  %28 = icmp sge i64 %27, %24
  %29 = xor i1 %21, %28
  br i1 %29, label %30, label %.loopexit80

30:                                               ; preds = %.preheader79
  %31 = add nsw i64 %25, 1
  %exitcond.not = icmp eq i64 %31, %12
  br i1 %exitcond.not, label %.loopexit80, label %.preheader79

.loopexit80:                                      ; preds = %30, %.preheader79, %15
  %32 = phi i64 [ %22, %15 ], [ %12, %30 ], [ %25, %.preheader79 ]
  %33 = icmp slt i64 %10, %32
  %or.cond = select i1 %21, i1 %33, i1 false
  br i1 %or.cond, label %.preheader77, label %.loopexit78

.preheader77:                                     ; preds = %.loopexit80, %46
  %34 = phi i64 [ %36, %46 ], [ %32, %.loopexit80 ]
  %35 = phi i64 [ %47, %46 ], [ %10, %.loopexit80 ]
  %36 = add nsw i64 %34, -1
  %37 = icmp eq i64 %35, %36
  br i1 %37, label %46, label %38

38:                                               ; preds = %.preheader77
  %39 = load i64, ptr %2, align 8
  %40 = inttoptr i64 %39 to ptr
  %41 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %40, i64 %35
  %42 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %40, i64 %36
  %43 = load i64, ptr %41, align 8
  %.value31 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %40, i64 %35, i32 1
  %44 = load i64, ptr %.value31, align 8
  %.value31._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.value31, i64 0, i32 1
  %45 = load ptr, ptr %.value31._guts._object._object, align 8
  tail call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %41, ptr noundef nonnull align 8 dereferenceable(24) %42, i64 24, i1 false)
  store i64 %43, ptr %42, align 8
  %.value33 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %40, i64 %36, i32 1
  store i64 %44, ptr %.value33, align 8
  %.value33._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.value33, i64 0, i32 1
  store ptr %45, ptr %.value33._guts._object._object, align 8
  br label %46

46:                                               ; preds = %.preheader77, %38
  %47 = add nsw i64 %35, 1
  %48 = icmp slt i64 %47, %36
  br i1 %48, label %.preheader77, label %.loopexit78.loopexit

.loopexit78.loopexit:                             ; preds = %46
  %.pre = load i64, ptr %.count, align 8
  br label %.loopexit78

.loopexit78:                                      ; preds = %.loopexit78.loopexit, %.loopexit80, %.preheader81
  %49 = phi i64 [ %12, %.preheader81 ], [ %12, %.loopexit80 ], [ %.pre, %.loopexit78.loopexit ]
  %50 = phi i64 [ %13, %.preheader81 ], [ %32, %.loopexit80 ], [ %32, %.loopexit78.loopexit ]
  %51 = icmp slt i64 %50, %49
  %52 = sub i64 %50, %10
  %53 = icmp slt i64 %52, %3
  %or.cond39 = select i1 %51, i1 %53, i1 false
  br i1 %or.cond39, label %54, label %.loopexit

54:                                               ; preds = %.loopexit78
  %55 = add i64 %10, %3
  %56 = tail call i64 @llvm.smin.i64(i64 %55, i64 %49)
  %57 = icmp eq i64 %50, %56
  br i1 %57, label %.loopexit, label %.preheader

.loopexit:                                        ; preds = %145, %54, %.loopexit78
  %58 = phi i64 [ %50, %.loopexit78 ], [ %50, %54 ], [ %56, %145 ]
  %59 = load ptr, ptr %6, align 8
  %60 = tail call zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr %59) #24
  br i1 %60, label %66, label %61, !prof !36

61:                                               ; preds = %.loopexit
  %62 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %59, i64 0, i32 1
  %63 = load i64, ptr %62, align 8, !range !38
  %64 = add nuw nsw i64 %63, 1
  %65 = tail call swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSnySiG_Tg5"(i1 false, i64 %64, i1 true, ptr %59)
  br label %66

66:                                               ; preds = %.loopexit, %61
  %67 = phi ptr [ %65, %61 ], [ %59, %.loopexit ]
  %68 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %67, i64 0, i32 1
  %69 = load i64, ptr %68, align 8, !range !38
  %._storage15._capacityAndFlags = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %67, i64 0, i32 1, i32 0, i32 1
  %70 = load i64, ptr %._storage15._capacityAndFlags, align 8
  %71 = lshr i64 %70, 1
  %72 = add nuw nsw i64 %69, 1
  %.not74 = icmp ugt i64 %71, %69
  br i1 %.not74, label %76, label %73, !prof !36

73:                                               ; preds = %66
  %74 = icmp ugt i64 %70, 1
  %75 = tail call swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSnySiG_Tg5"(i1 %74, i64 %72, i1 true, ptr nonnull %67)
  br label %76

76:                                               ; preds = %66, %73
  %77 = phi ptr [ %75, %73 ], [ %67, %66 ]
  %78 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %77, i64 0, i32 1
  store i64 %72, ptr %78, align 8
  %79 = getelementptr inbounds i8, ptr %77, i64 32
  %80 = getelementptr inbounds %TSnySiG, ptr %79, i64 %69
  store i64 %10, ptr %80, align 8
  %.upperBound18 = getelementptr inbounds %TSnySiG, ptr %80, i64 0, i32 1
  store i64 %58, ptr %.upperBound18, align 8
  store ptr %77, ptr %6, align 8
  %81 = load i64, ptr %0, align 8
  %82 = inttoptr i64 %81 to ptr
  %83 = call swiftcc i1 @"$sSr13_mergeTopRuns_6buffer2bySbSaySnySiGGz_SpyxGSbx_xtKXEtKFSi3key_SS5valuet_Tg543$s4mainSbSi3key_SS5valuet_SiAB_SSACttXEfU1_Tf1nncn_n"(ptr nocapture nonnull dereferenceable(8) %6, ptr %82, ptr nocapture nonnull dereferenceable(16) %2, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %5)
  %84 = load ptr, ptr %5, align 8
  %.not75 = icmp eq ptr %84, null
  br i1 %.not75, label %85, label %89

85:                                               ; preds = %76
  %86 = load i64, ptr %2, align 8
  %87 = load i64, ptr %.count, align 8
  %88 = icmp slt i64 %58, %87
  br i1 %88, label %.preheader81, label %.loopexit82.loopexit

89:                                               ; preds = %76
  %90 = load ptr, ptr %6, align 8
  tail call void @swift_bridgeObjectRelease(ptr %90) #9
  br label %128

.loopexit82.loopexit:                             ; preds = %85
  %.pre97 = load ptr, ptr %6, align 8
  br label %.loopexit82

.loopexit82:                                      ; preds = %.loopexit82.loopexit, %entry
  %91 = phi ptr [ @_swiftEmptyArrayStorage, %entry ], [ %.pre97, %.loopexit82.loopexit ]
  %92 = phi i64 [ %7, %entry ], [ %86, %.loopexit82.loopexit ]
  %93 = load i64, ptr %0, align 8
  %94 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %91, i64 0, i32 1
  %95 = load i64, ptr %94, align 8, !range !38
  %96 = icmp ugt i64 %95, 1
  br i1 %96, label %97, label %.loopexit104

97:                                               ; preds = %.loopexit82
  %98 = tail call zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr nonnull %91) #24
  br i1 %98, label %101, label %99, !prof !36

99:                                               ; preds = %97
  %100 = tail call swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNewAByxGyFSnySiG_Tgq5"(ptr nonnull %91)
  br label %101

101:                                              ; preds = %97, %99
  %102 = phi ptr [ %100, %99 ], [ %91, %97 ]
  %103 = inttoptr i64 %93 to ptr
  %104 = inttoptr i64 %92 to ptr
  %.phi.trans.insert = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %102, i64 0, i32 1
  %.pre98 = load i64, ptr %.phi.trans.insert, align 8, !range !38
  %105 = getelementptr inbounds i8, ptr %102, i64 32
  br label %106

106:                                              ; preds = %120, %101
  %107 = phi i64 [ %.pre98, %101 ], [ %122, %120 ]
  %108 = add nsw i64 %107, -1
  %109 = add nsw i64 %107, -2
  %110 = getelementptr inbounds %TSnySiG, ptr %105, i64 %109
  %111 = load i64, ptr %110, align 8
  %112 = getelementptr inbounds %TSnySiG, ptr %105, i64 %108
  %113 = load i64, ptr %112, align 8
  %.upperBound = getelementptr inbounds %TSnySiG, ptr %112, i64 0, i32 1
  %114 = load i64, ptr %.upperBound, align 8
  %115 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %104, i64 %111
  %116 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %104, i64 %113
  %117 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %104, i64 %114
  %118 = tail call swiftcc i1 @"$ss6_merge3low3mid4high6buffer2bySbSpyxG_A3GSbx_xtKXEtKlFSi3key_SS5valuet_Tg543$s4mainSbSi3key_SS5valuet_SiAB_SSACttXEfU1_Tf1nnnnc_n"(ptr %115, ptr %116, ptr %117, ptr %103, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %5)
  %119 = load ptr, ptr %5, align 8
  %.not = icmp eq ptr %119, null
  br i1 %.not, label %120, label %127

120:                                              ; preds = %106
  store i64 %111, ptr %110, align 8
  %.upperBound6 = getelementptr inbounds %TSnySiG, ptr %110, i64 0, i32 1
  store i64 %114, ptr %.upperBound6, align 8
  store ptr %102, ptr %6, align 8
  %121 = load i64, ptr %.phi.trans.insert, align 8, !range !38
  %122 = add nsw i64 %121, -1
  %123 = sub nsw i64 %122, %108
  %124 = getelementptr inbounds %TSnySiG, ptr %112, i64 1
  %125 = shl nuw i64 %123, 4
  tail call void @llvm.memmove.p0.p0.i64(ptr nonnull align 8 %112, ptr nonnull align 8 %124, i64 %125, i1 false)
  store i64 %122, ptr %.phi.trans.insert, align 8
  %126 = icmp ugt i64 %122, 1
  br i1 %126, label %106, label %.loopexit104

127:                                              ; preds = %106
  tail call void @swift_bridgeObjectRelease(ptr nonnull %102) #9
  br label %128

common.ret:                                       ; preds = %128, %.loopexit104
  ret void

.loopexit104:                                     ; preds = %120, %.loopexit82
  %.sink = phi ptr [ %91, %.loopexit82 ], [ %102, %120 ]
  tail call void @swift_bridgeObjectRelease(ptr nonnull %.sink) #9
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6)
  br label %common.ret

128:                                              ; preds = %89, %127
  %129 = phi ptr [ %119, %127 ], [ %84, %89 ]
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6)
  store ptr %129, ptr %5, align 8
  br label %common.ret

.preheader:                                       ; preds = %54, %145
  %130 = phi i64 [ %146, %145 ], [ %50, %54 ]
  br label %131

131:                                              ; preds = %141, %.preheader
  %132 = phi i64 [ %130, %.preheader ], [ %133, %141 ]
  %133 = add i64 %132, -1
  %134 = load i64, ptr %2, align 8
  %135 = inttoptr i64 %134 to ptr
  %136 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %135, i64 %132
  %137 = load i64, ptr %136, align 8
  %138 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %135, i64 %133
  %139 = load i64, ptr %138, align 8
  %140 = icmp slt i64 %137, %139
  br i1 %140, label %141, label %145

141:                                              ; preds = %131
  %.value = getelementptr inbounds <{ %TSi, %TSS }>, ptr %135, i64 %132, i32 1
  %.value._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.value, i64 0, i32 1
  %142 = load i64, ptr %.value, align 8
  %143 = load ptr, ptr %.value._guts._object._object, align 8
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %136, ptr noundef nonnull align 8 dereferenceable(24) %138, i64 24, i1 false)
  store i64 %137, ptr %138, align 8
  %.value26 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %135, i64 %133, i32 1
  store i64 %142, ptr %.value26, align 8
  %.value26._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.value26, i64 0, i32 1
  store ptr %143, ptr %.value26._guts._object._object, align 8
  %144 = icmp eq i64 %133, %10
  br i1 %144, label %145, label %131

145:                                              ; preds = %141, %131
  %146 = add i64 %130, 1
  %147 = icmp eq i64 %146, %56
  br i1 %147, label %.loopexit, label %.preheader
}

define linkonce_odr hidden swiftcc i1 @"$sSr13_mergeTopRuns_6buffer2bySbSaySnySiGGz_SpyxGSbx_xtKXEtKFSi3key_SS5valuet_Tg543$s4mainSbSi3key_SS5valuet_SiAB_SSACttXEfU1_Tf1nncn_n"(ptr nocapture dereferenceable(8) %0, ptr %1, ptr nocapture dereferenceable(16) %2, ptr swiftself %3, ptr noalias nocapture swifterror dereferenceable(8) %4) local_unnamed_addr #0 {
entry:
  %5 = load ptr, ptr %0, align 8
  %6 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %5, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !range !38
  %8 = icmp ugt i64 %7, 1
  br i1 %8, label %9, label %common.ret

9:                                                ; preds = %entry
  %10 = tail call zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr nonnull %5) #24
  br i1 %10, label %13, label %11, !prof !36

11:                                               ; preds = %9
  %12 = tail call swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNewAByxGyFSnySiG_Tgq5"(ptr nonnull %5)
  br label %13

13:                                               ; preds = %9, %11
  %14 = phi ptr [ %12, %11 ], [ %5, %9 ]
  store ptr %14, ptr %0, align 8
  %.phi.trans.insert = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %14, i64 0, i32 1
  %.pre = load i64, ptr %.phi.trans.insert, align 8, !range !38
  br label %15

15:                                               ; preds = %75, %13
  %16 = phi i64 [ %.pre, %13 ], [ %83, %75 ]
  %17 = phi ptr [ %14, %13 ], [ %81, %75 ]
  %18 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %17, i64 0, i32 1
  %19 = add nsw i64 %16, -1
  %20 = icmp ult i64 %16, 4
  %.phi.trans.insert48 = getelementptr inbounds i8, ptr %17, i64 32
  br i1 %20, label %39, label %21

21:                                               ; preds = %15
  %22 = add nsw i64 %16, -4
  %23 = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert48, i64 %22
  %24 = load i64, ptr %23, align 8
  %.upperBound = getelementptr inbounds %TSnySiG, ptr %23, i64 0, i32 1
  %25 = load i64, ptr %.upperBound, align 8
  %26 = sub i64 %25, %24
  %27 = add nsw i64 %16, -3
  %28 = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert48, i64 %27
  %29 = load i64, ptr %28, align 8
  %.upperBound4 = getelementptr inbounds %TSnySiG, ptr %28, i64 0, i32 1
  %30 = load i64, ptr %.upperBound4, align 8
  %31 = sub i64 %30, %29
  %32 = add nsw i64 %16, -2
  %33 = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert48, i64 %32
  %34 = load i64, ptr %33, align 8
  %.upperBound6 = getelementptr inbounds %TSnySiG, ptr %33, i64 0, i32 1
  %35 = load i64, ptr %.upperBound6, align 8
  %36 = sub i64 %31, %34
  %37 = add i64 %36, %35
  %38 = icmp slt i64 %37, %26
  br i1 %38, label %.thread, label %87

39:                                               ; preds = %15
  %.not33 = icmp eq i64 %16, 3
  br i1 %.not33, label %..thread_crit_edge, label %._crit_edge

._crit_edge:                                      ; preds = %39
  %.phi.trans.insert42 = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert48, i64 %19
  %.pre43 = load i64, ptr %.phi.trans.insert42, align 8
  %.upperBound28.phi.trans.insert = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert42, i64 0, i32 1
  %.pre46 = load i64, ptr %.upperBound28.phi.trans.insert, align 8
  %.pre56 = add nsw i64 %16, -2
  %.pre58 = sub i64 %.pre46, %.pre43
  br label %54

..thread_crit_edge:                               ; preds = %39
  %.pre50 = load i64, ptr %.phi.trans.insert48, align 8
  %.phi.trans.insert54 = getelementptr inbounds i8, ptr %17, i64 48
  %.pre55 = load i64, ptr %.phi.trans.insert54, align 8
  br label %.thread

.thread:                                          ; preds = %..thread_crit_edge, %21
  %40 = phi i64 [ %.pre55, %..thread_crit_edge ], [ %34, %21 ]
  %.pre-phi52 = phi i64 [ 1, %..thread_crit_edge ], [ %32, %21 ]
  %41 = phi i64 [ %.pre50, %..thread_crit_edge ], [ %29, %21 ]
  %.pre-phi = phi i64 [ 0, %..thread_crit_edge ], [ %27, %21 ]
  %42 = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert48, i64 %.pre-phi
  %.upperBound20 = getelementptr inbounds %TSnySiG, ptr %42, i64 0, i32 1
  %43 = load i64, ptr %.upperBound20, align 8
  %44 = sub i64 %43, %41
  %45 = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert48, i64 %.pre-phi52
  %.upperBound22 = getelementptr inbounds %TSnySiG, ptr %45, i64 0, i32 1
  %46 = load i64, ptr %.upperBound22, align 8
  %47 = sub i64 %46, %40
  %48 = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert48, i64 %19
  %49 = load i64, ptr %48, align 8
  %.upperBound24 = getelementptr inbounds %TSnySiG, ptr %48, i64 0, i32 1
  %50 = load i64, ptr %.upperBound24, align 8
  %51 = sub i64 %50, %49
  %52 = add i64 %47, %51
  %53 = icmp slt i64 %52, %44
  br i1 %53, label %54, label %85

54:                                               ; preds = %._crit_edge, %.thread
  %.pre-phi59 = phi i64 [ %.pre58, %._crit_edge ], [ %51, %.thread ]
  %.pre-phi57 = phi i64 [ %.pre56, %._crit_edge ], [ %.pre-phi52, %.thread ]
  %55 = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert48, i64 %.pre-phi57
  %56 = load i64, ptr %55, align 8
  %.upperBound26 = getelementptr inbounds %TSnySiG, ptr %55, i64 0, i32 1
  %57 = load i64, ptr %.upperBound26, align 8
  %58 = sub i64 %57, %56
  %59 = icmp slt i64 %.pre-phi59, %58
  br i1 %59, label %common.ret, label %60

common.ret:                                       ; preds = %60, %75, %54, %entry
  ret i1 true

60:                                               ; preds = %87, %85, %54
  %61 = phi i64 [ %19, %54 ], [ %., %85 ], [ %.29, %87 ]
  %62 = add nsw i64 %61, -1
  %63 = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert48, i64 %62
  %64 = load i64, ptr %63, align 8
  %65 = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert48, i64 %61
  %66 = load i64, ptr %65, align 8
  %.upperBound11 = getelementptr inbounds %TSnySiG, ptr %65, i64 0, i32 1
  %67 = load i64, ptr %.upperBound11, align 8
  %68 = load i64, ptr %2, align 8
  %69 = inttoptr i64 %68 to ptr
  %70 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %69, i64 %64
  %71 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %69, i64 %66
  %72 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %69, i64 %67
  %73 = tail call swiftcc i1 @"$ss6_merge3low3mid4high6buffer2bySbSpyxG_A3GSbx_xtKXEtKlFSi3key_SS5valuet_Tg543$s4mainSbSi3key_SS5valuet_SiAB_SSACttXEfU1_Tf1nnnnc_n"(ptr %70, ptr %71, ptr %72, ptr %1, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %4)
  %74 = load ptr, ptr %4, align 8
  %.not = icmp eq ptr %74, null
  br i1 %.not, label %75, label %common.ret

75:                                               ; preds = %60
  store i64 %64, ptr %63, align 8
  %.upperBound13 = getelementptr inbounds %TSnySiG, ptr %63, i64 0, i32 1
  store i64 %67, ptr %.upperBound13, align 8
  store ptr %17, ptr %0, align 8
  %76 = load i64, ptr %18, align 8, !range !38
  %77 = add nsw i64 %76, -1
  %78 = sub nsw i64 %77, %61
  %79 = getelementptr inbounds %TSnySiG, ptr %65, i64 1
  %80 = shl nuw i64 %78, 4
  tail call void @llvm.memmove.p0.p0.i64(ptr nonnull align 8 %65, ptr nonnull align 8 %79, i64 %80, i1 false)
  store i64 %77, ptr %18, align 8
  %81 = load ptr, ptr %0, align 8
  %82 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %81, i64 0, i32 1
  %83 = load i64, ptr %82, align 8, !range !38
  %84 = icmp ugt i64 %83, 1
  br i1 %84, label %15, label %common.ret

85:                                               ; preds = %.thread
  %86 = icmp slt i64 %44, %51
  %. = select i1 %86, i64 %.pre-phi52, i64 %19
  br label %60

87:                                               ; preds = %21
  %88 = getelementptr inbounds %TSnySiG, ptr %.phi.trans.insert48, i64 %19
  %89 = load i64, ptr %88, align 8
  %.upperBound8 = getelementptr inbounds %TSnySiG, ptr %88, i64 0, i32 1
  %90 = load i64, ptr %.upperBound8, align 8
  %91 = sub i64 %90, %89
  %92 = icmp slt i64 %31, %91
  %.29 = select i1 %92, i64 %32, i64 %19
  br label %60
}

define linkonce_odr hidden swiftcc i1 @"$ss6_merge3low3mid4high6buffer2bySbSpyxG_A3GSbx_xtKXEtKlFSi3key_SS5valuet_Tg543$s4mainSbSi3key_SS5valuet_SiAB_SSACttXEfU1_Tf1nnnnc_n"(ptr %0, ptr %1, ptr %2, ptr %3, ptr swiftself %4, ptr noalias nocapture swifterror dereferenceable(8) %5) local_unnamed_addr #0 {
entry:
  %6 = ptrtoint ptr %1 to i64
  %7 = ptrtoint ptr %0 to i64
  %8 = sub i64 %6, %7
  %9 = sdiv i64 %8, 24
  %10 = ptrtoint ptr %2 to i64
  %11 = sub i64 %10, %6
  %12 = sdiv i64 %11, 24
  %13 = icmp slt i64 %9, %12
  br i1 %13, label %14, label %18

14:                                               ; preds = %entry
  %15 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %0, i64 %9
  %16 = icmp ugt ptr %15, %3
  %17 = icmp eq ptr %3, %0
  %or.cond67 = select i1 %17, i1 %16, i1 false
  br i1 %or.cond67, label %23, label %.sink.split

18:                                               ; preds = %entry
  %19 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %1, i64 %12
  %20 = icmp ugt ptr %19, %3
  %21 = icmp eq ptr %3, %1
  %or.cond70 = select i1 %21, i1 %20, i1 false
  br i1 %or.cond70, label %58, label %.sink.split58

.sink.split:                                      ; preds = %14
  %22 = mul nuw nsw i64 %9, 24
  tail call void @llvm.memmove.p0.p0.i64(ptr align 8 %3, ptr align 8 %0, i64 %22, i1 false)
  br label %23

23:                                               ; preds = %14, %.sink.split
  %24 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %3, i64 %9
  %25 = icmp sgt i64 %8, 23
  %26 = icmp ult ptr %1, %2
  %or.cond5 = select i1 %25, i1 %26, i1 false
  br i1 %or.cond5, label %.preheader, label %.loopexit

.preheader:                                       ; preds = %23, %39
  %27 = phi ptr [ %40, %39 ], [ %1, %23 ]
  %28 = phi ptr [ %41, %39 ], [ %3, %23 ]
  %29 = phi ptr [ %42, %39 ], [ %0, %23 ]
  %30 = load i64, ptr %27, align 8
  %31 = load i64, ptr %28, align 8
  %32 = icmp slt i64 %30, %31
  br i1 %32, label %33, label %36

33:                                               ; preds = %.preheader
  %34 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %27, i64 1
  %35 = icmp eq ptr %29, %27
  br i1 %35, label %39, label %.sink.split54

36:                                               ; preds = %.preheader
  %37 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %28, i64 1
  %38 = icmp eq ptr %29, %28
  br i1 %38, label %39, label %.sink.split54

.sink.split54:                                    ; preds = %33, %36
  %.sink = phi ptr [ %28, %36 ], [ %27, %33 ]
  %.ph = phi ptr [ %27, %36 ], [ %34, %33 ]
  %.ph55 = phi ptr [ %37, %36 ], [ %28, %33 ]
  tail call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %29, ptr noundef nonnull align 8 dereferenceable(24) %.sink, i64 24, i1 false)
  br label %39

39:                                               ; preds = %36, %33, %.sink.split54
  %40 = phi ptr [ %.ph, %.sink.split54 ], [ %34, %33 ], [ %27, %36 ]
  %41 = phi ptr [ %.ph55, %.sink.split54 ], [ %28, %33 ], [ %37, %36 ]
  %42 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %29, i64 1
  %43 = icmp ult ptr %41, %24
  %44 = icmp ult ptr %40, %2
  %or.cond8 = select i1 %43, i1 %44, i1 false
  br i1 %or.cond8, label %.preheader, label %.loopexit

.loopexit:                                        ; preds = %77, %39, %58, %23
  %45 = phi ptr [ %24, %23 ], [ %59, %58 ], [ %24, %39 ], [ %79, %77 ]
  %46 = phi ptr [ %3, %23 ], [ %3, %58 ], [ %41, %39 ], [ %3, %77 ]
  %47 = phi ptr [ %0, %23 ], [ %1, %58 ], [ %42, %39 ], [ %78, %77 ]
  %48 = ptrtoint ptr %45 to i64
  %49 = ptrtoint ptr %46 to i64
  %50 = sub i64 %48, %49
  %51 = sdiv i64 %50, 24
  %52 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %46, i64 %51
  %53 = icmp ult ptr %47, %52
  %54 = icmp eq ptr %47, %46
  %or.cond64 = select i1 %54, i1 %53, i1 false
  br i1 %or.cond64, label %56, label %.sink.split56

.sink.split56:                                    ; preds = %.loopexit
  %55 = mul nuw nsw i64 %51, 24
  tail call void @llvm.memmove.p0.p0.i64(ptr align 8 %47, ptr align 8 %46, i64 %55, i1 false)
  br label %56

56:                                               ; preds = %.loopexit, %.sink.split56
  ret i1 true

.sink.split58:                                    ; preds = %18
  %57 = mul nuw nsw i64 %12, 24
  tail call void @llvm.memmove.p0.p0.i64(ptr align 8 %3, ptr align 8 %1, i64 %57, i1 false)
  br label %58

58:                                               ; preds = %18, %.sink.split58
  %59 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %3, i64 %12
  %60 = icmp sgt i64 %11, 23
  %61 = icmp ult ptr %0, %1
  %or.cond15 = select i1 %60, i1 %61, i1 false
  br i1 %or.cond15, label %.preheader41, label %.loopexit

.preheader41:                                     ; preds = %58, %77
  %62 = phi ptr [ %78, %77 ], [ %1, %58 ]
  %63 = phi ptr [ %65, %77 ], [ %2, %58 ]
  %64 = phi ptr [ %79, %77 ], [ %59, %58 ]
  %65 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %63, i64 -1
  %66 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %64, i64 -1
  %67 = load i64, ptr %66, align 8
  %68 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %62, i64 -1
  %69 = load i64, ptr %68, align 8
  %70 = icmp slt i64 %67, %69
  br i1 %70, label %71, label %74

71:                                               ; preds = %.preheader41
  %72 = icmp ult ptr %65, %62
  %73 = icmp eq ptr %63, %62
  %or.cond68 = select i1 %73, i1 %72, i1 false
  br i1 %or.cond68, label %77, label %.sink.split60

74:                                               ; preds = %.preheader41
  %75 = icmp ult ptr %65, %64
  %76 = icmp eq ptr %63, %64
  %or.cond69 = select i1 %76, i1 %75, i1 false
  br i1 %or.cond69, label %77, label %.sink.split60

.sink.split60:                                    ; preds = %71, %74
  %.sink63 = phi ptr [ %66, %74 ], [ %68, %71 ]
  %.ph61 = phi ptr [ %62, %74 ], [ %68, %71 ]
  %.ph62 = phi ptr [ %66, %74 ], [ %64, %71 ]
  tail call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %65, ptr noundef nonnull align 8 dereferenceable(24) %.sink63, i64 24, i1 false)
  br label %77

77:                                               ; preds = %74, %71, %.sink.split60
  %78 = phi ptr [ %.ph61, %.sink.split60 ], [ %68, %71 ], [ %62, %74 ]
  %79 = phi ptr [ %.ph62, %.sink.split60 ], [ %64, %71 ], [ %66, %74 ]
  %80 = icmp ugt ptr %79, %3
  %81 = icmp ugt ptr %78, %0
  %or.cond18 = select i1 %80, i1 %81, i1 false
  br i1 %or.cond18, label %.preheader41, label %.loopexit
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNewAByxGyFSnySiG_Tgq5"(ptr %0) local_unnamed_addr #4 {
entry:
  %1 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8, !range !38
  %3 = tail call swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSnySiG_Tgq5"(i1 false, i64 %2, i1 false, ptr %0)
  ret ptr %3
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSnySiG_Tg5"(i1 %0, i64 %1, i1 %2, ptr %3) local_unnamed_addr #5 {
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
  %14 = load i64, ptr %4, align 8, !range !38
  %15 = tail call i64 @llvm.smax.i64(i64 %13, i64 %14)
  %16 = icmp eq i64 %15, 0
  br i1 %16, label %25, label %17

17:                                               ; preds = %12
  %18 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCySnySiGGMD") #23
  %19 = shl i64 %15, 4
  %20 = add i64 %19, 32
  %21 = tail call noalias ptr @swift_allocObject(ptr %18, i64 %20, i64 7) #9
  %call.i = tail call i64 @malloc_size(ptr noundef %21) #26, !clang.arc.no_objc_arc_exceptions !33
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
  %30 = getelementptr inbounds %TSnySiG, ptr %28, i64 %14
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
  br label %37

35:                                               ; preds = %25
  %36 = shl nuw i64 %14, 4
  tail call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 8 %27, ptr nonnull align 8 %28, i64 %36, i1 false)
  br label %37

37:                                               ; preds = %34, %35
  tail call void @swift_bridgeObjectRelease(ptr nonnull %3) #9
  ret ptr %26
}

define linkonce_odr hidden swiftcc ptr @"$ss22_ContiguousArrayBufferV19_uninitializedCount15minimumCapacityAByxGSi_SitcfCSi3key_SS5valuet_Tgm5"(i64 %0, i64 %1) local_unnamed_addr #6 {
entry:
  %2 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %3 = icmp eq i64 %2, 0
  br i1 %3, label %12, label %4

4:                                                ; preds = %entry
  %5 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCySi3key_SS5valuetGMD") #23
  %6 = mul i64 %2, 24
  %7 = add i64 %6, 32
  %8 = tail call noalias ptr @swift_allocObject(ptr %5, i64 %7, i64 7) #9
  %call.i = tail call i64 @malloc_size(ptr noundef %8) #26, !clang.arc.no_objc_arc_exceptions !33
  %gepdiff = add nsw i64 %call.i, -32
  %9 = sdiv i64 %gepdiff, 24
  %10 = shl nsw i64 %9, 1
  %11 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %8, i64 0, i32 1
  store i64 %0, ptr %11, align 8
  %._storage1._capacityAndFlags = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %8, i64 0, i32 1, i32 0, i32 1
  store i64 %10, ptr %._storage1._capacityAndFlags, align 8
  br label %12

12:                                               ; preds = %entry, %4
  %13 = phi ptr [ %8, %4 ], [ @_swiftEmptyArrayStorage, %entry ]
  ret ptr %13
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSnySiG_Tgq5"(i1 %0, i64 %1, i1 %2, ptr %3) local_unnamed_addr #5 {
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

12:                                               ; preds = %9, %5, %entry
  %13 = phi i64 [ %1, %entry ], [ %7, %5 ], [ %11, %9 ]
  %14 = load i64, ptr %4, align 8, !range !38
  %15 = tail call i64 @llvm.smax.i64(i64 %13, i64 %14)
  %16 = icmp eq i64 %15, 0
  br i1 %16, label %25, label %17

17:                                               ; preds = %12
  %18 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCySnySiGGMD") #23
  %19 = shl i64 %15, 4
  %20 = add i64 %19, 32
  %21 = tail call noalias ptr @swift_allocObject(ptr %18, i64 %20, i64 7) #9
  %call.i = tail call i64 @malloc_size(ptr noundef %21) #26, !clang.arc.no_objc_arc_exceptions !33
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
  br i1 %0, label %31, label %29

29:                                               ; preds = %25
  %30 = shl nuw i64 %14, 4
  tail call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 8 %27, ptr nonnull align 8 %28, i64 %30, i1 false)
  br label %37

31:                                               ; preds = %25
  %32 = getelementptr inbounds %TSnySiG, ptr %28, i64 %14
  %33 = icmp ult ptr %27, %32
  %34 = icmp eq ptr %26, %3
  %or.cond9 = select i1 %34, i1 %33, i1 false
  br i1 %or.cond9, label %36, label %.sink.split

.sink.split:                                      ; preds = %31
  %35 = shl nuw i64 %14, 4
  tail call void @llvm.memmove.p0.p0.i64(ptr nonnull align 8 %27, ptr nonnull align 8 %28, i64 %35, i1 false)
  br label %36

36:                                               ; preds = %31, %.sink.split
  store i64 0, ptr %4, align 8
  br label %37

37:                                               ; preds = %36, %29
  tail call void @swift_bridgeObjectRelease(ptr nonnull %3) #9
  ret ptr %26
}

define linkonce_odr hidden swiftcc { i64, i1 } @"$ss17_NativeDictionaryV12mutatingFind_8isUniques10_HashTableV6BucketV6bucket_Sb5foundtx_SbtFSi_SSTg5"(i64 %0, i1 %1, ptr nocapture swiftself dereferenceable(8) %2) local_unnamed_addr #0 {
entry:
  %3 = load ptr, ptr %2, align 8
  %4 = tail call swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4findys10_HashTableV6BucketV6bucket_Sb5foundtxSHRzlFSi_Tg5"(i64 %0, ptr swiftself %3)
  %5 = extractvalue { i64, i1 } %4, 1
  %6 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 1
  %7 = load i64, ptr %6, align 8
  %not. = xor i1 %5, true
  %. = zext i1 %not. to i64
  %8 = add i64 %7, %.
  %9 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 2
  %10 = load i64, ptr %9, align 8, !range !38
  %11 = icmp slt i64 %10, %8
  %.not = xor i1 %1, true
  %brmerge = select i1 %11, i1 true, i1 %.not
  br i1 %brmerge, label %12, label %20, !prof !39

12:                                               ; preds = %entry
  %brmerge2 = select i1 %1, i1 true, i1 %11
  br i1 %brmerge2, label %14, label %13

13:                                               ; preds = %12
  tail call swiftcc void @"$ss17_NativeDictionaryV4copyyyFSi_SSTg5"(ptr nocapture nonnull swiftself dereferenceable(8) %2)
  br label %20

14:                                               ; preds = %12
  tail call swiftcc void @"$ss17_NativeDictionaryV20_copyOrMoveAndResize8capacity12moveElementsySi_SbtFSi_SSTg5"(i64 %8, i1 %1, ptr nocapture nonnull swiftself dereferenceable(8) %2)
  %15 = load ptr, ptr %2, align 8
  %16 = tail call swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4findys10_HashTableV6BucketV6bucket_Sb5foundtxSHRzlFSi_Tg5"(i64 %0, ptr swiftself %15)
  %17 = extractvalue { i64, i1 } %16, 1
  %18 = xor i1 %5, %17
  br i1 %18, label %19, label %20

19:                                               ; preds = %14
  tail call swiftcc void @"$ss53KEY_TYPE_OF_DICTIONARY_VIOLATES_HASHABLE_REQUIREMENTSys5NeverOypXpF"(ptr nonnull @"$sSiN")
  unreachable

20:                                               ; preds = %entry, %14, %13
  %.pn = phi { i64, i1 } [ %4, %13 ], [ %16, %14 ], [ %4, %entry ]
  %21 = insertvalue { i64, i1 } %.pn, i1 %5, 1
  ret { i64, i1 } %21
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4find_9hashValues10_HashTableV6BucketV6bucket_Sb5foundtx_SitSHRzlFSi_Tg5"(i64 %0, i64 %1, ptr swiftself %2) local_unnamed_addr #4 {
entry:
  %3 = getelementptr inbounds i8, ptr %2, i64 64
  %4 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %2, i64 0, i32 3
  %5 = load i8, ptr %4, align 8
  %6 = and i8 %5, 63
  %7 = zext i8 %6 to i64
  %notmask = shl nsw i64 -1, %7
  %8 = xor i64 %notmask, -1
  %9 = and i64 %8, %1
  %10 = lshr i64 %9, 6
  %11 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %3, i64 %10
  %12 = load i64, ptr %11, align 8
  %13 = and i64 %9, 63
  %14 = shl nuw i64 1, %13
  %15 = and i64 %14, %12
  %16 = icmp eq i64 %15, 0
  br i1 %16, label %.loopexit, label %17

17:                                               ; preds = %entry
  %18 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %2, i64 0, i32 8
  %19 = load ptr, ptr %18, align 8
  %20 = getelementptr inbounds %TSi, ptr %19, i64 %9
  %21 = load i64, ptr %20, align 8
  %22 = icmp eq i64 %21, %0
  br i1 %22, label %.loopexit, label %23

23:                                               ; preds = %17
  %24 = add i64 %9, 1
  %25 = and i64 %24, %8
  %26 = lshr i64 %25, 6
  %27 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %3, i64 %26
  %28 = load i64, ptr %27, align 8
  %29 = and i64 %25, 63
  %30 = shl nuw i64 1, %29
  %31 = and i64 %28, %30
  %32 = icmp eq i64 %31, 0
  br i1 %32, label %.loopexit, label %.preheader

.loopexit:                                        ; preds = %.preheader, %41, %23, %17, %entry
  %33 = phi i1 [ false, %entry ], [ true, %17 ], [ false, %23 ], [ %40, %41 ], [ %40, %.preheader ]
  %34 = phi i64 [ %9, %entry ], [ %9, %17 ], [ %25, %23 ], [ %37, %.preheader ], [ %43, %41 ]
  %35 = insertvalue { i64, i1 } undef, i64 %34, 0
  %36 = insertvalue { i64, i1 } %35, i1 %33, 1
  ret { i64, i1 } %36

.preheader:                                       ; preds = %23, %41
  %37 = phi i64 [ %43, %41 ], [ %25, %23 ]
  %38 = getelementptr inbounds %TSi, ptr %19, i64 %37
  %39 = load i64, ptr %38, align 8
  %40 = icmp eq i64 %39, %0
  br i1 %40, label %.loopexit, label %41

41:                                               ; preds = %.preheader
  %42 = add i64 %37, 1
  %43 = and i64 %42, %8
  %44 = lshr i64 %43, 6
  %45 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %3, i64 %44
  %46 = load i64, ptr %45, align 8
  %47 = and i64 %43, 63
  %48 = shl nuw i64 1, %47
  %49 = and i64 %46, %48
  %50 = icmp eq i64 %49, 0
  br i1 %50, label %.loopexit, label %.preheader
}

define linkonce_odr hidden swiftcc void @"$ss17_NativeDictionaryV4copyyyFSi_SSTg5"(ptr nocapture swiftself dereferenceable(8) %0) local_unnamed_addr #0 {
entry:
  %1 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss18_DictionaryStorageCySiSSGMD") #23
  %2 = load ptr, ptr %0, align 8
  %3 = tail call swiftcc ptr @"$ss18_DictionaryStorageC4copy8originalAByxq_Gs05__RawaB0C_tFZ"(ptr %2, ptr swiftself %1)
  %4 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %2, i64 0, i32 1
  %5 = load i64, ptr %4, align 8, !range !38
  %.not = icmp eq i64 %5, 0
  br i1 %.not, label %.loopexit39, label %6

6:                                                ; preds = %entry
  %7 = getelementptr inbounds i8, ptr %3, i64 64
  %8 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 3
  %9 = load i8, ptr %8, align 8
  %10 = and i8 %9, 63
  %11 = zext i8 %10 to i64
  %notmask.neg = shl nuw i64 1, %11
  %12 = getelementptr inbounds i8, ptr %2, i64 64
  %13 = add nuw i64 %notmask.neg, 63
  %14 = lshr i64 %13, 6
  %15 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %12, i64 %14
  %16 = icmp ult ptr %7, %15
  %17 = icmp eq ptr %3, %2
  %or.cond38 = select i1 %17, i1 %16, i1 false
  br i1 %or.cond38, label %19, label %.sink.split

.sink.split:                                      ; preds = %6
  %18 = shl nuw nsw i64 %14, 3
  tail call void @llvm.memmove.p0.p0.i64(ptr nonnull align 8 %7, ptr nonnull align 8 %12, i64 %18, i1 false)
  br label %19

19:                                               ; preds = %6, %.sink.split
  %20 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %2, i64 0, i32 3
  %21 = load i64, ptr %4, align 8, !range !38
  %22 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 1
  store i64 %21, ptr %22, align 8
  %23 = load i8, ptr %20, align 8
  %24 = and i8 %23, 63
  %25 = zext i8 %24 to i64
  %notmask.neg31 = shl nuw i64 1, %25
  %26 = load i64, ptr %12, align 8
  %27 = icmp slt i64 %notmask.neg31, 64
  %28 = and i64 %notmask.neg31, 63
  %notmask = shl nsw i64 -1, %28
  %29 = xor i64 %notmask, -1
  %30 = select i1 %27, i64 %29, i64 -1
  %31 = and i64 %30, %26
  %32 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %2, i64 0, i32 8
  %33 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %2, i64 0, i32 9
  %34 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 8
  %35 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 9
  %36 = add nuw i64 %notmask.neg31, 63
  %37 = lshr i64 %36, 6
  %38 = add nsw i64 %37, -1
  br label %39

39:                                               ; preds = %90, %19
  %40 = phi i64 [ %31, %19 ], [ %92, %90 ]
  %41 = phi i64 [ 0, %19 ], [ %93, %90 ]
  %42 = icmp eq i64 %40, 0
  br i1 %42, label %43, label %46

43:                                               ; preds = %39
  %44 = add i64 %41, 1
  %45 = icmp slt i64 %44, %37
  br i1 %45, label %52, label %.loopexit39

46:                                               ; preds = %39
  %47 = tail call i64 @llvm.cttz.i64(i64 %40, i1 true), !range !40
  %48 = add i64 %40, -1
  %49 = and i64 %48, %40
  %50 = shl i64 %41, 6
  %51 = or i64 %50, %47
  br label %90

52:                                               ; preds = %43
  %53 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %12, i64 %44
  %54 = load i64, ptr %53, align 8
  %55 = icmp eq i64 %54, 0
  br i1 %55, label %56, label %.loopexit

56:                                               ; preds = %52
  %57 = add i64 %41, 2
  %58 = icmp slt i64 %57, %37
  br i1 %58, label %59, label %.loopexit39

59:                                               ; preds = %56
  %60 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %12, i64 %57
  %61 = load i64, ptr %60, align 8
  %62 = icmp eq i64 %61, 0
  br i1 %62, label %63, label %.loopexit

63:                                               ; preds = %59
  %64 = add i64 %41, 3
  %65 = icmp slt i64 %64, %37
  br i1 %65, label %66, label %.loopexit39

66:                                               ; preds = %63
  %67 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %12, i64 %64
  %68 = load i64, ptr %67, align 8
  %69 = icmp eq i64 %68, 0
  br i1 %69, label %70, label %.loopexit

70:                                               ; preds = %66
  %71 = add i64 %41, 4
  %72 = icmp slt i64 %71, %37
  br i1 %72, label %73, label %.loopexit39

73:                                               ; preds = %70
  %74 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %12, i64 %71
  %75 = load i64, ptr %74, align 8
  %76 = icmp eq i64 %75, 0
  br i1 %76, label %.preheader, label %.loopexit

.preheader:                                       ; preds = %73, %78
  %77 = phi i64 [ %79, %78 ], [ %71, %73 ]
  %exitcond.not = icmp eq i64 %77, %38
  br i1 %exitcond.not, label %.loopexit39, label %78

78:                                               ; preds = %.preheader
  %79 = add nsw i64 %77, 1
  %80 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %12, i64 %79
  %81 = load i64, ptr %80, align 8
  %82 = icmp eq i64 %81, 0
  br i1 %82, label %.preheader, label %.loopexit

.loopexit:                                        ; preds = %78, %73, %66, %59, %52
  %83 = phi i64 [ %44, %52 ], [ %57, %59 ], [ %64, %66 ], [ %71, %73 ], [ %79, %78 ]
  %84 = phi i64 [ %54, %52 ], [ %61, %59 ], [ %68, %66 ], [ %75, %73 ], [ %81, %78 ]
  %85 = tail call i64 @llvm.cttz.i64(i64 %84, i1 false), !range !40
  %86 = add i64 %84, -1
  %87 = and i64 %86, %84
  %88 = shl i64 %83, 6
  %89 = add i64 %85, %88
  br label %90

90:                                               ; preds = %.loopexit, %46
  %91 = phi i64 [ %51, %46 ], [ %89, %.loopexit ]
  %92 = phi i64 [ %49, %46 ], [ %87, %.loopexit ]
  %93 = phi i64 [ %41, %46 ], [ %83, %.loopexit ]
  %94 = load ptr, ptr %32, align 8
  %95 = getelementptr inbounds %TSi, ptr %94, i64 %91
  %96 = load i64, ptr %95, align 8
  %97 = load ptr, ptr %33, align 8
  %98 = getelementptr inbounds %TSS, ptr %97, i64 %91
  %99 = load i64, ptr %98, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %98, i64 0, i32 1
  %100 = load ptr, ptr %._guts._object._object, align 8
  %101 = load ptr, ptr %34, align 8
  %102 = getelementptr inbounds %TSi, ptr %101, i64 %91
  store i64 %96, ptr %102, align 8
  %103 = load ptr, ptr %35, align 8
  %104 = getelementptr inbounds %TSS, ptr %103, i64 %91
  store i64 %99, ptr %104, align 8
  %._guts10._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %104, i64 0, i32 1
  store ptr %100, ptr %._guts10._object._object, align 8
  %105 = tail call ptr @swift_bridgeObjectRetain(ptr returned %100) #9
  br label %39

.loopexit39:                                      ; preds = %63, %70, %56, %43, %.preheader, %entry
  tail call void @swift_release(ptr nonnull %2) #9
  store ptr %3, ptr %0, align 8
  ret void
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc void @"$ss17_NativeDictionaryV20_copyOrMoveAndResize8capacity12moveElementsySi_SbtFSi_SSTg5"(i64 %0, i1 %1, ptr nocapture swiftself dereferenceable(8) %2) local_unnamed_addr #4 {
entry:
  %3 = load ptr, ptr %2, align 8
  %4 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !range !38
  %6 = tail call i64 @llvm.smax.i64(i64 %5, i64 %0)
  %7 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss18_DictionaryStorageCySiSSGMD") #23
  %8 = tail call swiftcc ptr @"$ss18_DictionaryStorageC6resize8original8capacity4moveAByxq_Gs05__RawaB0C_SiSbtFZ"(ptr %3, i64 %6, i1 %1, ptr swiftself %7)
  %9 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 1
  %10 = load i64, ptr %9, align 8, !range !38
  %.not = icmp eq i64 %10, 0
  br i1 %.not, label %34, label %11

11:                                               ; preds = %entry
  %12 = getelementptr inbounds i8, ptr %3, i64 64
  %13 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 3
  %14 = load i8, ptr %13, align 8
  %15 = and i8 %14, 63
  %16 = zext i8 %15 to i64
  %notmask.neg = shl nuw i64 1, %16
  %17 = load i64, ptr %12, align 8
  %18 = icmp slt i64 %notmask.neg, 64
  %19 = and i64 %notmask.neg, 63
  %notmask48 = shl nsw i64 -1, %19
  %20 = xor i64 %notmask48, -1
  %21 = select i1 %18, i64 %20, i64 -1
  %22 = and i64 %21, %17
  %23 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 8
  %24 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 9
  %25 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %8, i64 0, i32 7
  %26 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %8, i64 0, i32 3
  %27 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %8, i64 0, i32 8
  %28 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %8, i64 0, i32 9
  %29 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %8, i64 0, i32 1
  %30 = add nuw i64 %notmask.neg, 63
  %31 = lshr i64 %30, 6
  %32 = getelementptr inbounds i8, ptr %8, i64 64
  %33 = add nsw i64 %31, -1
  br label %35

34:                                               ; preds = %entry
  tail call void @swift_release(ptr nonnull %3) #9
  br label %160

35:                                               ; preds = %134, %11
  %36 = phi i64 [ %22, %11 ], [ %88, %134 ]
  %37 = phi i64 [ 0, %11 ], [ %89, %134 ]
  %38 = icmp eq i64 %36, 0
  br i1 %38, label %39, label %42

39:                                               ; preds = %35
  %40 = add i64 %37, 1
  %41 = icmp slt i64 %40, %31
  br i1 %41, label %48, label %.loopexit49

42:                                               ; preds = %35
  %43 = tail call i64 @llvm.cttz.i64(i64 %36, i1 true), !range !40
  %44 = add i64 %36, -1
  %45 = and i64 %44, %36
  %46 = shl i64 %37, 6
  %47 = or i64 %46, %43
  br label %86

48:                                               ; preds = %39
  %49 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %12, i64 %40
  %50 = load i64, ptr %49, align 8
  %51 = icmp eq i64 %50, 0
  br i1 %51, label %52, label %.loopexit50

52:                                               ; preds = %48
  %53 = add i64 %37, 2
  %54 = icmp slt i64 %53, %31
  br i1 %54, label %55, label %.loopexit49

55:                                               ; preds = %52
  %56 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %12, i64 %53
  %57 = load i64, ptr %56, align 8
  %58 = icmp eq i64 %57, 0
  br i1 %58, label %59, label %.loopexit50

59:                                               ; preds = %55
  %60 = add i64 %37, 3
  %61 = icmp slt i64 %60, %31
  br i1 %61, label %62, label %.loopexit49

62:                                               ; preds = %59
  %63 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %12, i64 %60
  %64 = load i64, ptr %63, align 8
  %65 = icmp eq i64 %64, 0
  br i1 %65, label %66, label %.loopexit50

66:                                               ; preds = %62
  %67 = add i64 %37, 4
  %68 = icmp slt i64 %67, %31
  br i1 %68, label %69, label %.loopexit49

69:                                               ; preds = %66
  %70 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %12, i64 %67
  %71 = load i64, ptr %70, align 8
  %72 = icmp eq i64 %71, 0
  br i1 %72, label %.preheader, label %.loopexit50

.preheader:                                       ; preds = %69, %74
  %73 = phi i64 [ %75, %74 ], [ %67, %69 ]
  %exitcond.not = icmp eq i64 %73, %33
  br i1 %exitcond.not, label %.loopexit49, label %74

74:                                               ; preds = %.preheader
  %75 = add nsw i64 %73, 1
  %76 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %12, i64 %75
  %77 = load i64, ptr %76, align 8
  %78 = icmp eq i64 %77, 0
  br i1 %78, label %.preheader, label %.loopexit50

.loopexit50:                                      ; preds = %74, %69, %62, %55, %48
  %79 = phi i64 [ %40, %48 ], [ %53, %55 ], [ %60, %62 ], [ %67, %69 ], [ %75, %74 ]
  %80 = phi i64 [ %50, %48 ], [ %57, %55 ], [ %64, %62 ], [ %71, %69 ], [ %77, %74 ]
  %81 = tail call i64 @llvm.cttz.i64(i64 %80, i1 false), !range !40
  %82 = add i64 %80, -1
  %83 = and i64 %82, %80
  %84 = shl i64 %79, 6
  %85 = add i64 %81, %84
  br label %86

86:                                               ; preds = %.loopexit50, %42
  %87 = phi i64 [ %47, %42 ], [ %85, %.loopexit50 ]
  %88 = phi i64 [ %45, %42 ], [ %83, %.loopexit50 ]
  %89 = phi i64 [ %37, %42 ], [ %79, %.loopexit50 ]
  %90 = load ptr, ptr %23, align 8
  %91 = getelementptr inbounds %TSi, ptr %90, i64 %87
  %92 = load i64, ptr %91, align 8
  %93 = load ptr, ptr %24, align 8
  %94 = getelementptr inbounds %TSS, ptr %93, i64 %87
  %95 = load i64, ptr %94, align 8
  %._guts21._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %94, i64 0, i32 1
  %96 = load ptr, ptr %._guts21._object._object, align 8
  br i1 %1, label %99, label %97

97:                                               ; preds = %86
  %98 = tail call ptr @swift_bridgeObjectRetain(ptr returned %96) #9
  br label %99

99:                                               ; preds = %86, %97
  %100 = load i64, ptr %25, align 8
  %101 = tail call swiftcc i64 @"$ss6HasherV5_hash4seed_S2i_s6UInt64VtFZ"(i64 %100, i64 %92)
  %102 = load i8, ptr %26, align 8
  %103 = and i8 %102, 63
  %104 = zext i8 %103 to i64
  %notmask = shl nsw i64 -1, %104
  %105 = xor i64 %notmask, -1
  %106 = and i64 %101, %105
  %107 = lshr i64 %106, 6
  %108 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %32, i64 %107
  %109 = load i64, ptr %108, align 8
  %110 = xor i64 %109, -1
  %111 = and i64 %106, 63
  %112 = shl nsw i64 -1, %111
  %113 = and i64 %112, %110
  %114 = icmp eq i64 %113, 0
  br i1 %114, label %115, label %118

115:                                              ; preds = %99
  %116 = sub i64 63, %notmask
  %117 = lshr i64 %116, 6
  br label %122

118:                                              ; preds = %99
  %119 = tail call i64 @llvm.cttz.i64(i64 %113, i1 true), !range !40
  %120 = and i64 %106, -64
  %121 = or i64 %119, %120
  br label %134

122:                                              ; preds = %122, %115
  %123 = phi i64 [ %107, %115 ], [ %.31, %122 ]
  %124 = add i64 %123, 1
  %125 = icmp eq i64 %124, %117
  %.31 = select i1 %125, i64 0, i64 %124
  %126 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %32, i64 %.31
  %127 = load i64, ptr %126, align 8
  %128 = icmp eq i64 %127, -1
  br i1 %128, label %122, label %129

129:                                              ; preds = %122
  %130 = xor i64 %127, -1
  %131 = tail call i64 @llvm.cttz.i64(i64 %130, i1 false), !range !40
  %132 = shl i64 %.31, 6
  %133 = add i64 %132, %131
  br label %134

134:                                              ; preds = %129, %118
  %135 = phi i64 [ %121, %118 ], [ %133, %129 ]
  %136 = lshr i64 %135, 6
  %137 = and i64 %135, 63
  %138 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %32, i64 %136
  %139 = shl nuw i64 1, %137
  %140 = load i64, ptr %138, align 8
  %141 = or i64 %139, %140
  store i64 %141, ptr %138, align 8
  %142 = load ptr, ptr %27, align 8
  %143 = getelementptr inbounds %TSi, ptr %142, i64 %135
  store i64 %92, ptr %143, align 8
  %144 = load ptr, ptr %28, align 8
  %145 = getelementptr inbounds %TSS, ptr %144, i64 %135
  store i64 %95, ptr %145, align 8
  %._guts14._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %145, i64 0, i32 1
  store ptr %96, ptr %._guts14._object._object, align 8
  %146 = load i64, ptr %29, align 8
  %147 = add i64 %146, 1
  store i64 %147, ptr %29, align 8
  br label %35

.loopexit49:                                      ; preds = %66, %59, %52, %39, %.preheader
  br i1 %1, label %148, label %153

148:                                              ; preds = %.loopexit49
  %149 = load i8, ptr %13, align 8
  %150 = and i8 %149, 63
  %151 = zext i8 %150 to i64
  %notmask47.neg = shl nuw i64 1, %151
  %152 = icmp slt i64 %notmask47.neg, 64
  br i1 %152, label %154, label %.loopexit.loopexit

153:                                              ; preds = %.loopexit49
  tail call void @swift_release(ptr %3) #9
  br label %160

154:                                              ; preds = %148
  %155 = and i64 %notmask47.neg, 63
  %156 = shl nsw i64 -1, %155
  store i64 %156, ptr %12, align 8
  br label %.loopexit

.loopexit.loopexit:                               ; preds = %148
  %157 = add nuw i64 %notmask47.neg, 63
  %158 = lshr i64 %157, 3
  %159 = and i64 %158, 2305843009213693944
  tail call void @llvm.memset.p0.i64(ptr nonnull align 8 %12, i8 0, i64 %159, i1 false)
  br label %.loopexit

.loopexit:                                        ; preds = %.loopexit.loopexit, %154
  store i64 0, ptr %9, align 8
  tail call void @swift_release(ptr nonnull %3) #9
  br label %160

160:                                              ; preds = %.loopexit, %153, %34
  store ptr %8, ptr %2, align 8
  ret void
}

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
define linkonce_odr hidden swiftcc { i64, ptr } @"$sSS8_copyingyS2SFZ"(i64 %0, ptr %1) local_unnamed_addr #4 {
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
  %14 = tail call ptr @swift_bridgeObjectRetain(ptr returned %1) #9
  %15 = tail call swiftcc { i64, i64, i64, ptr } @"$sSSySsSnySS5IndexVGcig"(i64 15, i64 %13, i64 %0, ptr %1)
  tail call void @swift_bridgeObjectRelease(ptr %1) #9
  %16 = extractvalue { i64, i64, i64, ptr } %15, 0
  %17 = extractvalue { i64, i64, i64, ptr } %15, 1
  %18 = extractvalue { i64, i64, i64, ptr } %15, 2
  %19 = extractvalue { i64, i64, i64, ptr } %15, 3
  %20 = tail call swiftcc { i64, ptr } @"$sSS8_copyingySSSsFZ"(i64 %16, i64 %17, i64 %18, ptr %19)
  tail call void @swift_bridgeObjectRelease(ptr %19) #9
  ret { i64, ptr } %20
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc { i64, ptr } @"$sSS8_copyingySSSsFZ"(i64 %0, i64 %1, i64 %2, ptr %3) local_unnamed_addr #4 {
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
  %18 = tail call ptr @swift_bridgeObjectRetain(ptr returned %3) #9
  tail call void @swift_bridgeObjectRelease(ptr %17) #9
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %4)
  br label %._crit_edge

._crit_edge:                                      ; preds = %9, %12
  %19 = phi ptr [ %13, %12 ], [ @_swiftEmptyArrayStorage, %9 ]
  %20 = getelementptr inbounds i8, ptr %19, i64 32
  %21 = ptrtoint ptr %20 to i64
  %22 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %19, i64 0, i32 1
  %23 = load i64, ptr %22, align 8, !range !38
  %24 = tail call swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64 %21, i64 %23), !noalias !41
  tail call void @swift_release(ptr %19) #9
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
  %35 = call ptr @swift_bridgeObjectRetain(ptr returned %3) #9
  %36 = call swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64 %33, i64 %34)
  call void @swift_bridgeObjectRelease(ptr %3) #9
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
  %54 = tail call ptr @swift_bridgeObjectRetain(ptr returned %3) #9
  %55 = tail call swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64 %52, i64 %53)
  tail call void @swift_bridgeObjectRelease(ptr %3) #9
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

define linkonce_odr hidden swiftcc ptr @"$ss22_ContiguousArrayBufferV19_uninitializedCount15minimumCapacityAByxGSi_SitcfCs5UInt8V_Tgmq5"(i64 %0, i64 %1) local_unnamed_addr #6 {
entry:
  %2 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %3 = icmp eq i64 %2, 0
  br i1 %3, label %10, label %4

4:                                                ; preds = %entry
  %5 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCys5UInt8VGMD") #23
  %6 = add i64 %2, 32
  %7 = tail call noalias ptr @swift_allocObject(ptr %5, i64 %6, i64 7) #9
  %call.i = tail call i64 @malloc_size(ptr noundef %7) #26, !clang.arc.no_objc_arc_exceptions !33
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

; Function Attrs: noinline
define linkonce_odr hidden swiftcc i64 @"$ss11_StringGutsV27_slowEnsureMatchingEncodingySS5IndexVAEF"(i64 %0, i64 %1, ptr %2) local_unnamed_addr #4 {
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

; Function Attrs: noinline
define linkonce_odr hidden swiftcc ptr @"$ss22_ContiguousArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSi3key_SS5valuet_Tg5"(i1 %0, i64 %1, i1 %2, ptr %3) local_unnamed_addr #5 {
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
  %14 = load i64, ptr %4, align 8, !range !38
  %15 = tail call i64 @llvm.smax.i64(i64 %13, i64 %14)
  %16 = icmp eq i64 %15, 0
  br i1 %16, label %25, label %17

17:                                               ; preds = %12
  %18 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCySi3key_SS5valuetGMD") #23
  %19 = mul i64 %15, 24
  %20 = add i64 %19, 32
  %21 = tail call noalias ptr @swift_allocObject(ptr %18, i64 %20, i64 7) #9
  %call.i = tail call i64 @malloc_size(ptr noundef %21) #26, !clang.arc.no_objc_arc_exceptions !33
  %gepdiff = add nsw i64 %call.i, -32
  %22 = sdiv i64 %gepdiff, 24
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
  %30 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %28, i64 %14
  %31 = icmp ult ptr %27, %30
  %32 = icmp eq ptr %26, %3
  %or.cond9 = select i1 %32, i1 %31, i1 false
  br i1 %or.cond9, label %34, label %.sink.split

.sink.split:                                      ; preds = %29
  %33 = mul nuw i64 %14, 24
  tail call void @llvm.memmove.p0.p0.i64(ptr nonnull align 8 %27, ptr nonnull align 8 %28, i64 %33, i1 false)
  br label %34

34:                                               ; preds = %29, %.sink.split
  store i64 0, ptr %4, align 8
  br label %37

35:                                               ; preds = %25
  %36 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSi3key_SS5valuetMD") #23
  tail call void @swift_arrayInitWithCopy(ptr nonnull %27, ptr nonnull %28, i64 %14, ptr %36) #24
  br label %37

37:                                               ; preds = %34, %35
  tail call void @swift_release(ptr nonnull %3) #9
  ret ptr %26
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc ptr @"$ss22_ContiguousArrayBufferV20_consumeAndCreateNewAByxGyFSi3key_SS5valuet_Tg5"(ptr %0) local_unnamed_addr #4 {
entry:
  %1 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8, !range !38
  %3 = tail call swiftcc ptr @"$ss22_ContiguousArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSi3key_SS5valuet_Tg5"(i1 false, i64 %2, i1 false, ptr %0)
  ret ptr %3
}

define linkonce_odr hidden swiftcc i64 @"$sSTsE21_copySequenceContents12initializing8IteratorQz_SitSry7ElementQzG_tFSDySiSSG_Tg5"(ptr noalias nocapture %0, i64 %1, i64 %2, ptr %3) local_unnamed_addr #0 {
entry:
  %4 = getelementptr inbounds i8, ptr %3, i64 64
  %5 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 3
  %6 = load i8, ptr %5, align 8
  %7 = and i8 %6, 63
  %8 = zext i8 %7 to i64
  %notmask = shl nsw i64 -1, %8
  %9 = load i64, ptr %4, align 8
  %10 = sub i64 0, %notmask
  %11 = icmp slt i64 %10, 64
  %12 = and i64 %10, 63
  %notmask19 = shl nsw i64 -1, %12
  %13 = xor i64 %notmask19, -1
  %14 = select i1 %11, i64 %13, i64 -1
  %15 = and i64 %14, %9
  %16 = icmp eq i64 %1, 0
  br i1 %16, label %.loopexit20, label %17

17:                                               ; preds = %entry
  %18 = icmp eq i64 %2, 0
  br i1 %18, label %.loopexit20, label %.preheader21

.preheader21:                                     ; preds = %17
  %19 = inttoptr i64 %1 to ptr
  %20 = sub i64 63, %notmask
  %21 = lshr i64 %20, 6
  %22 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 8
  %23 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 9
  %24 = add nsw i64 %21, -1
  br label %33

.loopexit20:                                      ; preds = %45, %65, %79, %81, %17, %entry
  %.sink53 = phi i64 [ 0, %entry ], [ %50, %81 ], [ 0, %17 ], [ %24, %79 ], [ %46, %65 ], [ %36, %45 ]
  %.sink = phi i64 [ %15, %entry ], [ %51, %81 ], [ %15, %17 ], [ 0, %79 ], [ 0, %65 ], [ 0, %45 ]
  %25 = phi i64 [ 0, %entry ], [ %2, %81 ], [ %2, %17 ], [ %35, %79 ], [ %35, %65 ], [ %35, %45 ]
  %26 = ptrtoint ptr %4 to i64
  %27 = ptrtoint ptr %3 to i64
  %28 = xor i64 %notmask, -1
  store i64 %27, ptr %0, align 8
  %29 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %0, i64 0, i32 1
  store i64 %26, ptr %29, align 8
  %30 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %0, i64 0, i32 2
  store i64 %28, ptr %30, align 8
  %31 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %0, i64 0, i32 3
  store i64 %.sink53, ptr %31, align 8
  %32 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %0, i64 0, i32 4
  store i64 %.sink, ptr %32, align 8
  ret i64 %25

33:                                               ; preds = %.preheader21, %83
  %34 = phi ptr [ %84, %83 ], [ %19, %.preheader21 ]
  %35 = phi i64 [ %52, %83 ], [ 0, %.preheader21 ]
  %36 = phi i64 [ %50, %83 ], [ 0, %.preheader21 ]
  %37 = phi i64 [ %51, %83 ], [ %15, %.preheader21 ]
  %38 = icmp eq i64 %37, 0
  br i1 %38, label %45, label %39

39:                                               ; preds = %33
  %40 = tail call i64 @llvm.cttz.i64(i64 %37, i1 true), !range !40
  %41 = add i64 %37, -1
  %42 = and i64 %41, %37
  %43 = shl i64 %36, 6
  %44 = or i64 %40, %43
  br label %48

45:                                               ; preds = %33
  %46 = add i64 %36, 1
  %47 = icmp slt i64 %46, %21
  br i1 %47, label %61, label %.loopexit20

48:                                               ; preds = %.loopexit, %39
  %49 = phi i64 [ %44, %39 ], [ %74, %.loopexit ]
  %50 = phi i64 [ %36, %39 ], [ %68, %.loopexit ]
  %51 = phi i64 [ %42, %39 ], [ %72, %.loopexit ]
  %52 = add nuw i64 %35, 1
  %53 = load ptr, ptr %22, align 8
  %54 = getelementptr inbounds %TSi, ptr %53, i64 %49
  %55 = load i64, ptr %54, align 8
  %56 = load ptr, ptr %23, align 8
  %57 = getelementptr inbounds %TSS, ptr %56, i64 %49
  %58 = load i64, ptr %57, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %57, i64 0, i32 1
  %59 = load ptr, ptr %._guts._object._object, align 8
  store i64 %55, ptr %34, align 8
  %.value3 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %34, i64 0, i32 1
  store i64 %58, ptr %.value3, align 8
  %.value3._guts._object._object = getelementptr inbounds <{ %TSi, %TSS }>, ptr %34, i64 0, i32 1, i32 0, i32 0, i32 1
  store ptr %59, ptr %.value3._guts._object._object, align 8
  %60 = icmp eq i64 %52, %2
  br i1 %60, label %81, label %83

61:                                               ; preds = %45
  %62 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %4, i64 %46
  %63 = load i64, ptr %62, align 8
  %64 = icmp eq i64 %63, 0
  br i1 %64, label %65, label %.loopexit

65:                                               ; preds = %61
  %66 = add i64 %36, 2
  %67 = icmp slt i64 %66, %21
  br i1 %67, label %.preheader, label %.loopexit20

.loopexit:                                        ; preds = %.preheader, %61
  %68 = phi i64 [ %46, %61 ], [ %75, %.preheader ]
  %69 = phi i64 [ %63, %61 ], [ %77, %.preheader ]
  %70 = tail call i64 @llvm.cttz.i64(i64 %69, i1 false), !range !40
  %71 = add i64 %69, -1
  %72 = and i64 %71, %69
  %73 = shl i64 %68, 6
  %74 = add i64 %70, %73
  br label %48

.preheader:                                       ; preds = %65, %79
  %75 = phi i64 [ %80, %79 ], [ %66, %65 ]
  %76 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %4, i64 %75
  %77 = load i64, ptr %76, align 8
  %78 = icmp eq i64 %77, 0
  br i1 %78, label %79, label %.loopexit

79:                                               ; preds = %.preheader
  %80 = add i64 %75, 1
  %exitcond.not = icmp eq i64 %80, %21
  br i1 %exitcond.not, label %.loopexit20, label %.preheader

81:                                               ; preds = %48
  %82 = tail call ptr @swift_bridgeObjectRetain(ptr returned %59) #9
  br label %.loopexit20

83:                                               ; preds = %48
  %84 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %34, i64 1
  %85 = tail call ptr @swift_bridgeObjectRetain(ptr returned %59) #9
  br label %33
}

define linkonce_odr hidden swiftcc ptr @"$sSD17dictionaryLiteralSDyxq_Gx_q_td_tcfCSi_SSTgm5Tf4g_n"(ptr %0) local_unnamed_addr #0 {
entry:
  %1 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8, !range !38
  %3 = icmp eq i64 %2, 0
  br i1 %3, label %.loopexit, label %4

4:                                                ; preds = %entry
  %5 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss18_DictionaryStorageCySiSSGMD") #23
  %6 = tail call swiftcc ptr @"$ss18_DictionaryStorageC8allocate8capacityAByxq_GSi_tFZ"(i64 %2, ptr swiftself %5)
  %7 = getelementptr inbounds i8, ptr %6, i64 64
  %8 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %6, i64 0, i32 8
  %9 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %6, i64 0, i32 9
  %10 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %6, i64 0, i32 1
  %11 = getelementptr inbounds i8, ptr %0, i64 32
  br label %13

.loopexit:                                        ; preds = %13, %entry
  %12 = phi ptr [ @_swiftEmptyDictionarySingleton, %entry ], [ %6, %13 ]
  ret ptr %12

13:                                               ; preds = %13, %4
  %14 = phi i64 [ 0, %4 ], [ %19, %13 ]
  %15 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %11, i64 %14
  %16 = load i64, ptr %15, align 8
  %.elt2 = getelementptr inbounds <{ %TSi, %TSS }>, ptr %15, i64 0, i32 1
  %17 = load i64, ptr %.elt2, align 8
  %.elt2._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.elt2, i64 0, i32 1
  %18 = load ptr, ptr %.elt2._guts._object._object, align 8
  %19 = add nuw nsw i64 %14, 1
  %20 = tail call swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4findys10_HashTableV6BucketV6bucket_Sb5foundtxSHRzlFSi_Tg5"(i64 %16, ptr swiftself %6)
  %21 = extractvalue { i64, i1 } %20, 0
  %22 = lshr i64 %21, 6
  %23 = and i64 %21, 63
  %24 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %7, i64 %22
  %25 = shl nuw i64 1, %23
  %26 = load i64, ptr %24, align 8
  %27 = or i64 %25, %26
  store i64 %27, ptr %24, align 8
  %28 = load ptr, ptr %8, align 8
  %29 = getelementptr inbounds %TSi, ptr %28, i64 %21
  store i64 %16, ptr %29, align 8
  %30 = load ptr, ptr %9, align 8
  %31 = getelementptr inbounds %TSS, ptr %30, i64 %21
  store i64 %17, ptr %31, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %31, i64 0, i32 1
  store ptr %18, ptr %._guts._object._object, align 8
  %32 = load i64, ptr %10, align 8
  %33 = add i64 %32, 1
  store i64 %33, ptr %10, align 8
  %34 = icmp eq i64 %19, %2
  %35 = tail call ptr @swift_bridgeObjectRetain(ptr returned %18) #9
  br i1 %34, label %.loopexit, label %13
}

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
  %4 = tail call ptr @objc_opt_self(ptr %3) #9
  %5 = tail call ptr @swift_getObjCClassMetadata(ptr %4) #27
  store atomic ptr %5, ptr @"$sSo17OS_dispatch_queueCML" release, align 8
  br label %cont

cont:                                             ; preds = %cacheIsNull, %entry
  %6 = phi ptr [ %1, %entry ], [ %5, %cacheIsNull ]
  %7 = insertvalue %swift.metadata_response undef, ptr %6, 0
  %8 = insertvalue %swift.metadata_response %7, i64 0, 1
  ret %swift.metadata_response %8
}

; Function Attrs: nounwind
declare ptr @objc_opt_self(ptr) local_unnamed_addr #9

; Function Attrs: mustprogress nofree nosync nounwind readnone willreturn
declare ptr @swift_getObjCClassMetadata(ptr) local_unnamed_addr #10

; Function Attrs: argmemonly mustprogress nocallback nofree nosync nounwind willreturn
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #1

declare swiftcc void @"$s8Dispatch0A3QoSV13userInitiatedACvgZ"(ptr noalias nocapture sret(%swift.opaque)) local_unnamed_addr #0

declare swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE10AttributesV10concurrentAEvgZ"(ptr noalias nocapture sret(%swift.opaque)) local_unnamed_addr #0

declare swiftcc ptr @"$sSo17OS_dispatch_queueC8DispatchE5label3qos10attributes20autoreleaseFrequency6targetABSS_AC0D3QoSVAbCE10AttributesVAbCE011AutoreleaseI0OABSgtcfC"(i64, ptr, ptr noalias nocapture, ptr noalias nocapture, ptr noalias nocapture, i64, ptr swiftself) local_unnamed_addr #0

declare swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture sret(%swift.opaque), ptr noalias nocapture, ptr, ptr, ptr swiftself, ptr, ptr) local_unnamed_addr #0

; Function Attrs: nofree nounwind readonly
declare ptr @swift_getWitnessTable(ptr, ptr, ptr) local_unnamed_addr #11

; Function Attrs: mustprogress nofree noinline nounwind readonly willreturn
define linkonce_odr hidden ptr @__swift_instantiateConcreteTypeFromMangledName(ptr %0) local_unnamed_addr #12 {
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
  %13 = tail call swiftcc ptr @swift_getTypeByMangledNameInContext2(ptr %12, i64 %8, ptr null, ptr null) #28
  %14 = ptrtoint ptr %13 to i64
  store atomic i64 %14, ptr %0 monotonic, align 8
  br label %3
}

; Function Attrs: argmemonly nounwind
declare swiftcc ptr @swift_getTypeByMangledNameInContext2(ptr, i64, ptr, ptr) local_unnamed_addr #13

; Function Attrs: mustprogress nofree noinline nounwind readonly willreturn
define linkonce_odr hidden ptr @__swift_instantiateConcreteTypeFromMangledNameAbstract(ptr %0) local_unnamed_addr #12 {
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
  %13 = tail call swiftcc ptr @swift_getTypeByMangledNameInContextInMetadataState2(i64 255, ptr %12, i64 %8, ptr null, ptr null) #28
  %14 = ptrtoint ptr %13 to i64
  store atomic i64 %14, ptr %0 monotonic, align 8
  br label %3
}

; Function Attrs: argmemonly nounwind
declare swiftcc ptr @swift_getTypeByMangledNameInContextInMetadataState2(i64, ptr, i64, ptr, ptr) local_unnamed_addr #13

declare swiftcc void @"$ss11_StringGutsV4growyySiF"(i64, ptr nocapture swiftself dereferenceable(16)) local_unnamed_addr #0

; Function Attrs: nounwind
declare void @swift_bridgeObjectRelease(ptr) local_unnamed_addr #9

declare swiftcc void @"$sSS6appendyySSF"(i64, ptr, ptr nocapture swiftself dereferenceable(16)) local_unnamed_addr #0

declare swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture swiftself, ptr, ptr) local_unnamed_addr #0

; Function Attrs: nounwind
declare void @swift_release(ptr) local_unnamed_addr #9

; Function Attrs: nounwind optsize
declare void @dispatch_group_enter(ptr noundef) local_unnamed_addr #7

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #14 {
entry:
  %2 = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1
  %3 = getelementptr inbounds { %objc_block, %swift.function }, ptr %1, i64 0, i32 1
  %.fn.load = load ptr, ptr %3, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %1, i64 0, i32 1, i32 1
  %4 = load ptr, ptr %.data, align 8
  store ptr %.fn.load, ptr %2, align 8
  %.data2 = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1, i32 1
  store ptr %4, ptr %.data2, align 8
  %5 = tail call ptr @swift_retain(ptr returned %4) #24
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
declare ptr @swift_retain(ptr returned) local_unnamed_addr #15

; Function Attrs: nounwind
define internal void @block_destroy_helper(ptr nocapture readonly %0) #3 {
entry:
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1, i32 1
  %1 = load ptr, ptr %.data, align 8
  tail call void @swift_release(ptr %1) #9
  ret void
}

declare ptr @_Block_copy(ptr) local_unnamed_addr

declare swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture sret(%swift.opaque)) local_unnamed_addr #0

; Function Attrs: nofree noinline nosync nounwind readnone
define internal ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr %0, ptr %1, ptr %2) local_unnamed_addr #8 {
entry:
  %3 = load ptr, ptr %0, align 8
  %4 = icmp eq ptr %3, null
  br i1 %4, label %cacheIsNull, label %cont

cacheIsNull:                                      ; preds = %entry
  %5 = tail call swiftcc %swift.metadata_response %1(i64 255) #22
  %6 = extractvalue %swift.metadata_response %5, 0
  %7 = tail call ptr @swift_getWitnessTable(ptr nonnull %2, ptr %6, ptr undef) #23
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
  %4 = tail call ptr @__swift_instantiateConcreteTypeFromMangledNameAbstract(ptr nonnull %1) #23
  %5 = tail call ptr @swift_getWitnessTable(ptr nonnull @"$sSayxGSTsMc", ptr %4, ptr undef) #23
  store atomic ptr %5, ptr %0 release, align 8
  br label %cont

cont:                                             ; preds = %cacheIsNull, %entry
  %6 = phi ptr [ %2, %entry ], [ %5, %cacheIsNull ]
  ret ptr %6
}

declare swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64, ptr noalias nocapture, ptr noalias nocapture, ptr, ptr swiftself) local_unnamed_addr #0

; Function Attrs: nounwind
declare void @_Block_release(ptr) local_unnamed_addr #9

declare swiftcc void @"$sSo17OS_dispatch_groupC8DispatchE4waityyF"(ptr swiftself) local_unnamed_addr #0

; Function Attrs: nounwind
declare ptr @swift_bridgeObjectRetain(ptr returned) local_unnamed_addr #9

; Function Attrs: noinline nounwind
define linkonce_odr hidden void @"$sSD8IteratorV8_VariantOyxq___GSHRzr0_lWOe"(i64 %0, i64 %1, i64 %2, i64 %3, i64 %4) local_unnamed_addr #16 {
entry:
  %5 = and i64 %0, 9223372036854775807
  %6 = inttoptr i64 %5 to ptr
  tail call void @swift_release(ptr %6) #9
  ret void
}

declare swiftcc void @"$ss5print_9separator10terminatoryypd_S2StF"(ptr, i64, ptr, i64, ptr) local_unnamed_addr #0

; Function Attrs: nounwind
declare ptr @swift_allocObject(ptr, i64, i64) local_unnamed_addr #9

declare swiftcc ptr @"$ss11CommandLineO9argumentsSaySSGvgZ"() local_unnamed_addr #0

; Function Attrs: mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64) #2

; Function Attrs: mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64) #2

; Function Attrs: noinline
declare swiftcc { i64, i64 } @"$ss13_StringObjectV10sharedUTF8SRys5UInt8VGvg"(i64, ptr) local_unnamed_addr #4

; Function Attrs: nofree noinline nosync nounwind readnone
define hidden swiftcc %swift.metadata_response @"$s4main8TreeNodeCMa"(i64 %0) #8 {
entry:
  %1 = tail call ptr @objc_opt_self(ptr nonnull getelementptr inbounds (<{ ptr, ptr, ptr, i64, ptr, ptr, ptr, ptr, i32, i32, i32, i16, i16, i32, i32, ptr, ptr, i64, i64, ptr, ptr, ptr, ptr, ptr, ptr, ptr, ptr }>, ptr @"$s4main8TreeNodeCMf", i64 0, i32 3)) #9
  %2 = insertvalue %swift.metadata_response undef, ptr %1, 0
  %3 = insertvalue %swift.metadata_response %2, i64 0, 1
  ret %swift.metadata_response %3
}

; Function Attrs: nounwind
declare void @swift_deletedMethodError() #9

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftFoundation"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftObjectiveC"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftDarwin"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftCoreFoundation"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftDispatch"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftXPC"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftIOKit"()

; Function Attrs: nounwind
declare void @swift_deallocClassInstance(ptr, i64, i64) local_unnamed_addr #9

declare swiftcc { i64, ptr } @"$sSSySSxcs25LosslessStringConvertibleRzSTRzSJ7ElementSTRtzlufC"(ptr noalias nocapture, ptr, ptr, ptr) local_unnamed_addr #0

declare swiftcc { i64, i64, i64, ptr } @"$sSSySsSnySS5IndexVGcig"(i64, i64, i64, ptr) local_unnamed_addr #0

declare swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64, i64) local_unnamed_addr #0

; Function Attrs: noinline
declare swiftcc i8 @"$sSS8UTF8ViewV17_foreignSubscript8positions5UInt8VSS5IndexV_tF"(i64, i64, ptr) local_unnamed_addr #4

; Function Attrs: noinline
declare swiftcc i64 @"$sSS8UTF8ViewV13_foreignIndex5afterSS0D0VAF_tF"(i64, i64, ptr) local_unnamed_addr #4

declare swiftcc i64 @"$sSS9UTF16ViewV5index_8offsetBySS5IndexVAF_SitF"(i64, i64, i64, ptr) local_unnamed_addr #0

; Function Attrs: noinline
declare swiftcc i64 @"$sSS8UTF8ViewV13_foreignIndex_8offsetBySS0D0VAF_SitF"(i64, i64, i64, ptr) local_unnamed_addr #4

; Function Attrs: noinline
declare swiftcc i64 @"$sSS8UTF8ViewV16_foreignDistance4from2toSiSS5IndexV_AGtF"(i64, i64, i64, ptr) local_unnamed_addr #4

; Function Attrs: mustprogress nounwind willreturn
declare zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr) local_unnamed_addr #15

; Function Attrs: argmemonly mustprogress nocallback nofree nounwind willreturn
declare void @llvm.memmove.p0.p0.i64(ptr nocapture writeonly, ptr nocapture readonly, i64, i1 immarg) #17

; Function Attrs: noinline
declare swiftcc ptr @"$sSa28_allocateBufferUninitialized15minimumCapacitys06_ArrayB0VyxGSi_tFZ"(i64, ptr) local_unnamed_addr #4

; Function Attrs: argmemonly mustprogress nocallback nofree nounwind willreturn
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #17

; Function Attrs: mustprogress nounwind willreturn
declare void @swift_arrayInitWithCopy(ptr, ptr, i64, ptr) local_unnamed_addr #15

; Function Attrs: mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn
declare i64 @llvm.cttz.i64(i64, i1 immarg) #2

; Function Attrs: nounwind
define private swiftcc void @objectdestroy(ptr swiftself %0) #3 {
entry:
  tail call void @swift_deallocObject(ptr %0, i64 32, i64 7) #9
  ret void
}

; Function Attrs: nounwind
declare void @swift_deallocObject(ptr, i64, i64) local_unnamed_addr #9

define internal swiftcc void @"$s4mainyyYbcfU0_yyYbcfU_TA"(ptr nocapture readonly swiftself %0) #0 {
entry:
  %1 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi }>, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8
  %3 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSi }>, ptr %0, i64 0, i32 2
  %4 = load i64, ptr %3, align 8
  tail call swiftcc void @"$s4mainyyYbcfU0_yyYbcfU_"(i64 %2, i64 %4)
  ret void
}

; Function Attrs: nounwind optsize
declare void @dispatch_group_leave(ptr noundef) local_unnamed_addr #7

; Function Attrs: nounwind
define private swiftcc void @objectdestroy.8(ptr swiftself %0) #3 {
entry:
  %._guts._object._object = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSS }>, ptr %0, i64 0, i32 2, i32 0, i32 0, i32 1
  %toDestroy = load ptr, ptr %._guts._object._object, align 8
  tail call void @swift_bridgeObjectRelease(ptr %toDestroy) #9
  tail call void @swift_deallocObject(ptr %0, i64 40, i64 7) #9
  ret void
}

define internal swiftcc void @"$s4mainyyYbcfU0_yyYbcfU_yyYbcfU_TA"(ptr nocapture readonly swiftself %0) #0 {
entry:
  %1 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSS }>, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8
  %3 = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSS }>, ptr %0, i64 0, i32 2
  %4 = load i64, ptr %3, align 8
  %._guts._object._object = getelementptr inbounds <{ %swift.refcounted, %TSi, %TSS }>, ptr %0, i64 0, i32 2, i32 0, i32 0, i32 1
  %5 = load ptr, ptr %._guts._object._object, align 8
  %6 = tail call ptr @swift_bridgeObjectRetain(ptr returned %5) #9
  tail call swiftcc void @"$sSD8_VariantV8setValue_6forKeyyq_n_xtFSi_SSTg5"(i64 %4, ptr %5, i64 %2, ptr nocapture nonnull swiftself dereferenceable(8) @"$s4main8messagesSDySiSSGvp")
  %7 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  tail call void @dispatch_group_leave(ptr %7)
  ret void
}

; Function Attrs: noinline
declare swiftcc void @"$ss53KEY_TYPE_OF_DICTIONARY_VIOLATES_HASHABLE_REQUIREMENTSys5NeverOypXpF"(ptr) local_unnamed_addr #4

declare swiftcc ptr @"$ss18_DictionaryStorageC6resize8original8capacity4moveAByxq_Gs05__RawaB0C_SiSbtFZ"(ptr, i64, i1, ptr swiftself) local_unnamed_addr #0

declare swiftcc i64 @"$ss6HasherV5_hash4seed_S2i_s6UInt64VtFZ"(i64, i64) local_unnamed_addr #0

declare swiftcc ptr @"$ss18_DictionaryStorageC4copy8originalAByxq_Gs05__RawaB0C_tFZ"(ptr, ptr swiftself) local_unnamed_addr #0

declare swiftcc ptr @"$ss18_DictionaryStorageC8allocate8capacityAByxq_GSi_tFZ"(i64, ptr swiftself) local_unnamed_addr #0

; Function Attrs: optsize
declare i64 @malloc_size(ptr noundef) local_unnamed_addr #18

; Function Attrs: inaccessiblememonly nocallback nofree nosync nounwind willreturn
declare void @llvm.assume(i1 noundef) #19

; Function Attrs: nocallback nofree nosync nounwind readnone speculatable willreturn
declare i64 @llvm.smax.i64(i64, i64) #20

; Function Attrs: nocallback nofree nosync nounwind readnone speculatable willreturn
declare i64 @llvm.smin.i64(i64, i64) #20

; Function Attrs: argmemonly nocallback nofree nounwind willreturn writeonly
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #21

; Function Attrs: nounwind
declare ptr @swift_bridgeObjectRetain_n(ptr, i32) #9

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.1(ptr nocapture writeonly %0, ptr nocapture readonly %1) #14 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #14
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.4(ptr nocapture writeonly %0, ptr nocapture readonly %1) #14 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #14
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.10(ptr nocapture writeonly %0, ptr nocapture readonly %1) #14 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #14
  ret void
}

; Function Attrs: nounwind
define internal void @block_destroy_helper.2(ptr nocapture readonly %0) #3 {
  tail call void @block_destroy_helper(ptr nocapture readonly %0) #3
  ret void
}

; Function Attrs: nounwind
define internal void @block_destroy_helper.5(ptr nocapture readonly %0) #3 {
  tail call void @block_destroy_helper(ptr nocapture readonly %0) #3
  ret void
}

; Function Attrs: nounwind
define internal void @block_destroy_helper.11(ptr nocapture readonly %0) #3 {
  tail call void @block_destroy_helper(ptr nocapture readonly %0) #3
  ret void
}

attributes #0 = { "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #1 = { argmemonly mustprogress nocallback nofree nosync nounwind willreturn }
attributes #2 = { mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn }
attributes #3 = { nounwind "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #4 = { noinline "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #5 = { noinline "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #6 = { "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #7 = { nounwind optsize "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #8 = { nofree noinline nosync nounwind readnone "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #9 = { nounwind }
attributes #10 = { mustprogress nofree nosync nounwind readnone willreturn }
attributes #11 = { nofree nounwind readonly }
attributes #12 = { mustprogress nofree noinline nounwind readonly willreturn "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #13 = { argmemonly nounwind }
attributes #14 = { mustprogress nounwind willreturn "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #15 = { mustprogress nounwind willreturn }
attributes #16 = { noinline nounwind "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #17 = { argmemonly mustprogress nocallback nofree nounwind willreturn }
attributes #18 = { optsize "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #19 = { inaccessiblememonly nocallback nofree nosync nounwind willreturn }
attributes #20 = { nocallback nofree nosync nounwind readnone speculatable willreturn }
attributes #21 = { argmemonly nocallback nofree nounwind willreturn writeonly }
attributes #22 = { nounwind readnone }
attributes #23 = { nounwind readonly }
attributes #24 = { nounwind willreturn }
attributes #25 = { noinline }
attributes #26 = { optsize }
attributes #27 = { nounwind readnone willreturn }
attributes #28 = { argmemonly nounwind readonly }

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
!38 = !{i64 0, i64 9223372036854775807}
!39 = !{!"branch_weights", i32 2002, i32 2000}
!40 = !{i64 0, i64 65}
!41 = !{!42}
!42 = distinct !{!42, !43, !"$sSS8_copyingySSSsFZSSSRys5UInt8VGXEfU0_: argument 0"}
!43 = distinct !{!43, !"$sSS8_copyingySSSsFZSSSRys5UInt8VGXEfU0_"}
