; ModuleID = 'results/binarytrees-swift-2.ll'
source_filename = "results/binarytrees-swift-2.ll"
target datalayout = "e-m:o-i64:64-i128:128-n32:64-S128"
target triple = "arm64-apple-macosx16.0.0"

%Ts6UInt32V = type <{ i32 }>
%TSD = type <{ %TSD8_VariantV }>
%TSD8_VariantV = type <{ %Ts14_BridgeStorageV }>
%Ts14_BridgeStorageV = type <{ ptr }>
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
%swift.full_boxmetadata = type { ptr, ptr, %swift.type, i32, ptr }
%swift.enum_vwtable = type { ptr, ptr, ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i32, i32, ptr, ptr, ptr }
%swift.type_metadata_record = type { i32 }
%TSa = type <{ %Ts12_ArrayBufferV }>
%Ts12_ArrayBufferV = type <{ %Ts14_BridgeStorageV }>
%objc_block = type { ptr, i32, i32, ptr, ptr }
%swift.function = type { ptr, ptr }
%Ts15ContiguousArrayV = type <{ %Ts22_ContiguousArrayBufferV }>
%Ts22_ContiguousArrayBufferV = type <{ ptr }>
%TSD8IteratorV = type <{ %TSD8IteratorV8_VariantO }>
%TSD8IteratorV8_VariantO = type <{ [40 x i8] }>
%Ts6UInt64V = type <{ i64 }>
%swift.metadata_response = type { ptr, i64 }
%swift.vwtable = type { ptr, ptr, ptr, ptr, ptr, ptr, ptr, ptr, i64, i64, i32, i32 }
%Ts5UInt8V = type <{ i8 }>
%swift.opaque = type opaque
%Ts22__RawDictionaryStorageC = type <{ %swift.refcounted, %TSi, %TSi, %Ts4Int8V, %Ts4Int8V, %Ts5Int16V, %Ts5Int32V, %TSi, %TSv, %TSv }>
%swift.refcounted = type { ptr, i64 }
%Ts4Int8V = type <{ i8 }>
%Ts5Int16V = type <{ i16 }>
%Ts5Int32V = type <{ i32 }>
%TSi = type <{ i64 }>
%TSv = type <{ ptr }>
%Ts28__ContiguousArrayStorageBaseC = type <{ %swift.refcounted, %Ts10_ArrayBodyV }>
%Ts10_ArrayBodyV = type <{ %TSo22_SwiftArrayBodyStorageV }>
%TSo22_SwiftArrayBodyStorageV = type <{ %TSi, %TSu }>
%TSu = type <{ i64 }>
%TSS = type <{ %Ts11_StringGutsV }>
%Ts11_StringGutsV = type <{ %Ts13_StringObjectV }>
%Ts13_StringObjectV = type <{ %Ts6UInt64V, ptr }>
%T4main4TreeO = type <{ [8 x i8] }>
%TSrys6UInt32V3key_SS5valuetG = type <{ %TSpys6UInt32V3key_SS5valuetGSg, %TSi }>
%TSpys6UInt32V3key_SS5valuetGSg = type <{ [8 x i8] }>
%TSnySiG = type <{ %TSi, %TSi }>
%Ts13_UnsafeBitsetV4WordV = type <{ %TSu }>
%Ts16IndexingIteratorVySs8UTF8ViewVG = type <{ %TSs8UTF8ViewV, %TSS5IndexV }>
%TSs8UTF8ViewV = type <{ %Ts5SliceVySS8UTF8ViewVG }>
%Ts5SliceVySS8UTF8ViewVG = type <{ %TSS5IndexV, %TSS5IndexV, %TSS8UTF8ViewV }>
%TSS8UTF8ViewV = type <{ %Ts11_StringGutsV }>
%TSS5IndexV = type <{ %Ts6UInt64V }>
%Ts17_NativeDictionaryV = type <{ ptr }>

@"$s4main1ns6UInt32Vvp" = hidden local_unnamed_addr global %Ts6UInt32V zeroinitializer, align 4
@"$s4main8minDepths6UInt32Vvp" = hidden local_unnamed_addr global %Ts6UInt32V zeroinitializer, align 4
@"$s4main8maxDepths6UInt32Vvp" = hidden local_unnamed_addr global %Ts6UInt32V zeroinitializer, align 4
@"$s4main8messagesSDys6UInt32VSSGvp" = hidden global %TSD zeroinitializer, align 8
@"$s4main5depths6UInt32Vvp" = hidden local_unnamed_addr global %Ts6UInt32V zeroinitializer, align 4
@"$s4main5groupSo012OS_dispatch_B0Cvp" = hidden local_unnamed_addr global ptr null, align 8
@"$s4main11workerQueueSo17OS_dispatch_queueCvp" = hidden local_unnamed_addr global ptr null, align 8
@"$s4main12messageQueueSo17OS_dispatch_queueCvp" = hidden local_unnamed_addr global ptr null, align 8
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
@block_descriptor.3 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.1, ptr @block_destroy_helper.2, ptr @".str.5.v8@?0" }, align 8
@block_descriptor.6 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.4, ptr @block_destroy_helper.5, ptr @".str.5.v8@?0" }, align 8
@"$ss23_ContiguousArrayStorageCMn" = external global %swift.type_descriptor, align 4
@"got.$ss23_ContiguousArrayStorageCMn" = private unnamed_addr constant ptr @"$ss23_ContiguousArrayStorageCMn"
@"symbolic _____yypG s23_ContiguousArrayStorageC" = linkonce_odr hidden constant <{ i8, i32, [4 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss23_ContiguousArrayStorageCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [4 x i8], i8 }>, ptr @"symbolic _____yypG s23_ContiguousArrayStorageC", i32 0, i32 1) to i64)) to i32), [4 x i8] c"yypG", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss23_ContiguousArrayStorageCyypGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____yypG s23_ContiguousArrayStorageC" to i64), i64 ptrtoint (ptr @"$ss23_ContiguousArrayStorageCyypGMD" to i64)) to i32), i32 -9 }, align 8
@"$sSSN" = external global %swift.type, align 8
@"\01l_entry_point" = private constant { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @main to i64), i64 ptrtoint (ptr @"\01l_entry_point" to i64)) to i32), i32 0 }, section "__TEXT, __swift5_entry, regular, no_dead_strip", align 4
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
@"$ss6UInt32VMn" = external global %swift.type_descriptor, align 4
@"got.$ss6UInt32VMn" = private unnamed_addr constant ptr @"$ss6UInt32VMn"
@"symbolic _____3key_SS5valuet s6UInt32V" = linkonce_odr hidden constant <{ i8, i32, [14 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss6UInt32VMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [14 x i8], i8 }>, ptr @"symbolic _____3key_SS5valuet s6UInt32V", i32 0, i32 1) to i64)) to i32), [14 x i8] c"3key_SS5valuet", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss6UInt32V3key_SS5valuetMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____3key_SS5valuet s6UInt32V" to i64), i64 ptrtoint (ptr @"$ss6UInt32V3key_SS5valuetMD" to i64)) to i32), i32 -19 }, align 8
@"symbolic _____ySnySiGG s23_ContiguousArrayStorageC" = linkonce_odr hidden constant <{ i8, i32, [8 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss23_ContiguousArrayStorageCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [8 x i8], i8 }>, ptr @"symbolic _____ySnySiGG s23_ContiguousArrayStorageC", i32 0, i32 1) to i64)) to i32), [8 x i8] c"ySnySiGG", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss23_ContiguousArrayStorageCySnySiGGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ySnySiGG s23_ContiguousArrayStorageC" to i64), i64 ptrtoint (ptr @"$ss23_ContiguousArrayStorageCySnySiGGMD" to i64)) to i32), i32 -13 }, align 8
@"symbolic _____y_____3key_SS5valuetG s23_ContiguousArrayStorageC s6UInt32V" = linkonce_odr hidden constant <{ i8, i32, [1 x i8], i8, i32, [15 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss23_ContiguousArrayStorageCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [1 x i8], i8, i32, [15 x i8], i8 }>, ptr @"symbolic _____y_____3key_SS5valuetG s23_ContiguousArrayStorageC s6UInt32V", i32 0, i32 1) to i64)) to i32), [1 x i8] c"y", i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss6UInt32VMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [1 x i8], i8, i32, [15 x i8], i8 }>, ptr @"symbolic _____y_____3key_SS5valuetG s23_ContiguousArrayStorageC s6UInt32V", i32 0, i32 4) to i64)) to i32), [15 x i8] c"3key_SS5valuetG", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss23_ContiguousArrayStorageCys6UInt32V3key_SS5valuetGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____y_____3key_SS5valuetG s23_ContiguousArrayStorageC s6UInt32V" to i64), i64 ptrtoint (ptr @"$ss23_ContiguousArrayStorageCys6UInt32V3key_SS5valuetGMD" to i64)) to i32), i32 -26 }, align 8
@"symbolic _____ s6UInt32V" = linkonce_odr hidden constant <{ i8, i32, i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss6UInt32VMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, i8 }>, ptr @"symbolic _____ s6UInt32V", i32 0, i32 1) to i64)) to i32), i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"\01l__swift5_reflection_descriptor" = private constant { i32, i32, i32, i32, i32 } { i32 2, i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ s6UInt32V" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor", i32 0, i32 3) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ s6UInt32V" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor", i32 0, i32 4) to i64)) to i32) }, section "__TEXT,__swift5_capture, regular", no_sanitize_address, align 4
@metadata = private constant %swift.full_boxmetadata { ptr @objectdestroy, ptr null, %swift.type { i64 1024 }, i32 16, ptr @"\01l__swift5_reflection_descriptor" }, align 8
@block_descriptor.9 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.7, ptr @block_destroy_helper.8, ptr @".str.5.v8@?0" }, align 8
@block_descriptor.12 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.10, ptr @block_destroy_helper.11, ptr @".str.5.v8@?0" }, align 8
@"symbolic SS" = linkonce_odr hidden constant <{ [2 x i8], i8 }> <{ [2 x i8] c"SS", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"\01l__swift5_reflection_descriptor.13" = private constant { i32, i32, i32, i32, i32 } { i32 2, i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ s6UInt32V" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.13", i32 0, i32 3) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic SS" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.13", i32 0, i32 4) to i64)) to i32) }, section "__TEXT,__swift5_capture, regular", no_sanitize_address, align 4
@metadata.15 = private constant %swift.full_boxmetadata { ptr @objectdestroy.14, ptr null, %swift.type { i64 1024 }, i32 16, ptr @"\01l__swift5_reflection_descriptor.13" }, align 8
@block_descriptor.18 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.16, ptr @block_destroy_helper.17, ptr @".str.5.v8@?0" }, align 8
@"$ss6UInt32VN" = external global %swift.type, align 8
@"$ss18_DictionaryStorageCMn" = external global %swift.type_descriptor, align 4
@"got.$ss18_DictionaryStorageCMn" = private unnamed_addr constant ptr @"$ss18_DictionaryStorageCMn"
@"symbolic _____y_____SSG s18_DictionaryStorageC s6UInt32V" = linkonce_odr hidden constant <{ i8, i32, [1 x i8], i8, i32, [3 x i8], i8 }> <{ i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss18_DictionaryStorageCMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [1 x i8], i8, i32, [3 x i8], i8 }>, ptr @"symbolic _____y_____SSG s18_DictionaryStorageC s6UInt32V", i32 0, i32 1) to i64)) to i32), [1 x i8] c"y", i8 2, i32 trunc (i64 sub (i64 ptrtoint (ptr @"got.$ss6UInt32VMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [1 x i8], i8, i32, [3 x i8], i8 }>, ptr @"symbolic _____y_____SSG s18_DictionaryStorageC s6UInt32V", i32 0, i32 4) to i64)) to i32), [3 x i8] c"SSG", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"$ss18_DictionaryStorageCys6UInt32VSSGMD" = linkonce_odr hidden global { i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____y_____SSG s18_DictionaryStorageC s6UInt32V" to i64), i64 ptrtoint (ptr @"$ss18_DictionaryStorageCys6UInt32VSSGMD" to i64)) to i32), i32 -14 }, align 8
@"$ss6UInt32Vs23CustomStringConvertiblesWP" = external global ptr, align 8
@".str.17.\09 trees of depth " = private unnamed_addr constant [18 x i8] c"\09 trees of depth \00"
@"symbolic _____4left_AA5rightt 4main4TreeO" = linkonce_odr hidden constant <{ i8, i32, [15 x i8], i8 }> <{ i8 1, i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main4TreeOMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, [15 x i8], i8 }>, ptr @"symbolic _____4left_AA5rightt 4main4TreeO", i32 0, i32 1) to i64)) to i32), [15 x i8] c"4left_AA5rightt", i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"\01l__swift5_reflection_descriptor.19" = private constant { i32, i32, i32, i32 } { i32 1, i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____4left_AA5rightt 4main4TreeO" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.19", i32 0, i32 3) to i64)) to i32) }, section "__TEXT,__swift5_capture, regular", no_sanitize_address, align 4
@metadata.21 = private constant %swift.full_boxmetadata { ptr @objectdestroy.20, ptr null, %swift.type { i64 1024 }, i32 16, ptr @"\01l__swift5_reflection_descriptor.19" }, align 8
@"\01l__swift5_reflection_descriptor.22" = private constant { i32, i32, i32, i32 } { i32 1, i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____4left_AA5rightt 4main4TreeO" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.22", i32 0, i32 3) to i64)) to i32) }, section "__TEXT,__swift5_capture, regular", no_sanitize_address, align 4
@"symbolic _____ 4main4TreeO" = linkonce_odr hidden constant <{ i8, i32, i8 }> <{ i8 1, i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main4TreeOMn" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i8, i32, i8 }>, ptr @"symbolic _____ 4main4TreeO", i32 0, i32 1) to i64)) to i32), i8 0 }>, section "__TEXT,__swift5_typeref, regular", no_sanitize_address, align 2
@"\01l__swift5_reflection_descriptor.23" = private constant { i32, i32, i32, i32 } { i32 1, i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ 4main4TreeO" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.23", i32 0, i32 3) to i64)) to i32) }, section "__TEXT,__swift5_capture, regular", no_sanitize_address, align 4
@metadata.25 = private constant %swift.full_boxmetadata { ptr @objectdestroy.24, ptr null, %swift.type { i64 1024 }, i32 16, ptr @"\01l__swift5_reflection_descriptor.23" }, align 8
@block_descriptor.28 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.26, ptr @block_destroy_helper.27, ptr @".str.5.v8@?0" }, align 8
@".str.25.long lived tree of depth " = private unnamed_addr constant [26 x i8] c"long lived tree of depth \00"
@"\01l__swift5_reflection_descriptor.29" = private constant { i32, i32, i32, i32 } { i32 1, i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ 4main4TreeO" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i32, i32 }, ptr @"\01l__swift5_reflection_descriptor.29", i32 0, i32 3) to i64)) to i32) }, section "__TEXT,__swift5_capture, regular", no_sanitize_address, align 4
@metadata.31 = private constant %swift.full_boxmetadata { ptr @objectdestroy.30, ptr null, %swift.type { i64 1024 }, i32 16, ptr @"\01l__swift5_reflection_descriptor.29" }, align 8
@block_descriptor.34 = internal constant { i64, i64, ptr, ptr, ptr } { i64 0, i64 48, ptr @block_copy_helper.32, ptr @block_destroy_helper.33, ptr @".str.5.v8@?0" }, align 8
@".str.22.stretch tree of depth " = private unnamed_addr constant [23 x i8] c"stretch tree of depth \00"
@.str.4.main = private constant [5 x i8] c"main\00"
@"$s4mainMXM" = linkonce_odr hidden constant <{ i32, i32, i32 }> <{ i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @.str.4.main to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i32, i32, i32 }>, ptr @"$s4mainMXM", i32 0, i32 2) to i64)) to i32) }>, section "__TEXT,__constg_swiftt", align 4
@.str.4.Tree = private constant [5 x i8] c"Tree\00"
@"$s4main4TreeOMn" = hidden constant <{ i32, i32, i32, i32, i32, i32, i32 }> <{ i32 82, i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4mainMXM" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32 }>, ptr @"$s4main4TreeOMn", i32 0, i32 1) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @.str.4.Tree to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32 }>, ptr @"$s4main4TreeOMn", i32 0, i32 2) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main4TreeOMa" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32 }>, ptr @"$s4main4TreeOMn", i32 0, i32 3) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main4TreeOMF" to i64), i64 ptrtoint (ptr getelementptr inbounds (<{ i32, i32, i32, i32, i32, i32, i32 }>, ptr @"$s4main4TreeOMn", i32 0, i32 4) to i64)) to i32), i32 1, i32 1 }>, section "__TEXT,__constg_swiftt", align 4
@0 = private constant [5 x i8] c"Node\00", section "__TEXT,__swift5_reflstr, regular", no_sanitize_address
@1 = private constant [6 x i8] c"Empty\00", section "__TEXT,__swift5_reflstr, regular", no_sanitize_address
@"$s4main4TreeOMF" = internal constant { i32, i32, i16, i16, i32, i32, i32, i32, i32, i32, i32 } { i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____ 4main4TreeO" to i64), i64 ptrtoint (ptr @"$s4main4TreeOMF" to i64)) to i32), i32 0, i16 2, i16 12, i32 2, i32 1, i32 trunc (i64 sub (i64 ptrtoint (ptr @"symbolic _____4left_AA5rightt 4main4TreeO" to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i16, i16, i32, i32, i32, i32, i32, i32, i32 }, ptr @"$s4main4TreeOMF", i32 0, i32 6) to i64)) to i32), i32 trunc (i64 sub (i64 ptrtoint (ptr @0 to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i16, i16, i32, i32, i32, i32, i32, i32, i32 }, ptr @"$s4main4TreeOMF", i32 0, i32 7) to i64)) to i32), i32 0, i32 0, i32 trunc (i64 sub (i64 ptrtoint (ptr @1 to i64), i64 ptrtoint (ptr getelementptr inbounds ({ i32, i32, i16, i16, i32, i32, i32, i32, i32, i32, i32 }, ptr @"$s4main4TreeOMF", i32 0, i32 10) to i64)) to i32) }, section "__TEXT,__swift5_fieldmd, regular", no_sanitize_address, align 4
@"$s4main4TreeOWV" = internal constant %swift.enum_vwtable { ptr @"$s4main4TreeOwCP", ptr @"$s4main4TreeOwxx", ptr @"$s4main4TreeOwcp", ptr @"$s4main4TreeOwca", ptr @__swift_memcpy8_8, ptr @"$s4main4TreeOwta", ptr @"$s4main4TreeOwet", ptr @"$s4main4TreeOwst", i64 8, i64 8, i32 2162695, i32 2147483646, ptr @"$s4main4TreeOwug", ptr @"$s4main4TreeOwup", ptr @"$s4main4TreeOwui" }, align 8
@"$s4main4TreeOMf" = internal constant <{ ptr, ptr, i64, ptr }> <{ ptr null, ptr @"$s4main4TreeOWV", i64 513, ptr @"$s4main4TreeOMn" }>, align 8
@"$s4main4TreeOHn" = private constant %swift.type_metadata_record { i32 trunc (i64 sub (i64 ptrtoint (ptr @"$s4main4TreeOMn" to i64), i64 ptrtoint (ptr @"$s4main4TreeOHn" to i64)) to i32) }, section "__TEXT, __swift5_types, regular", no_sanitize_address, align 4
@__swift_reflection_version = linkonce_odr hidden constant i16 3
@llvm.used = appending global [18 x ptr] [ptr @"\01l__swift5_reflection_descriptor", ptr @"\01l__swift5_reflection_descriptor.13", ptr @"\01l__swift5_reflection_descriptor.19", ptr @"\01l__swift5_reflection_descriptor.22", ptr @"\01l__swift5_reflection_descriptor.23", ptr @"\01l__swift5_reflection_descriptor.29", ptr @"\01l_entry_point", ptr @"$s4main4TreeOHn", ptr @"$s4main4TreeOMF", ptr @__swift_reflection_version, ptr @"_swift_FORCE_LOAD_$_swiftCoreFoundation_$_main", ptr @"_swift_FORCE_LOAD_$_swiftDarwin_$_main", ptr @"_swift_FORCE_LOAD_$_swiftDispatch_$_main", ptr @"_swift_FORCE_LOAD_$_swiftFoundation_$_main", ptr @"_swift_FORCE_LOAD_$_swiftIOKit_$_main", ptr @"_swift_FORCE_LOAD_$_swiftObjectiveC_$_main", ptr @"_swift_FORCE_LOAD_$_swiftXPC_$_main", ptr @main], section "llvm.metadata"
@llvm.compiler.used = appending global [1 x ptr] [ptr @"OBJC_CLASS_REF_$_OS_dispatch_queue"], section "llvm.metadata"

@"$s4main4TreeON" = hidden alias %swift.type, getelementptr inbounds (<{ ptr, ptr, i64, ptr }>, ptr @"$s4main4TreeOMf", i32 0, i32 2)

define i32 @main(i32 %0, ptr nocapture readnone %1) #0 {
entry:
  %2 = alloca %TSa, align 8
  %3 = alloca { %objc_block, %swift.function }, align 8
  %4 = alloca %TSa, align 8
  %5 = alloca { %objc_block, %swift.function }, align 8
  %6 = alloca %TSa, align 8
  %7 = alloca { %objc_block, %swift.function }, align 8
  %8 = alloca %TSa, align 8
  %9 = alloca %Ts15ContiguousArrayV, align 8
  %10 = alloca %TSD8IteratorV, align 8
  %swifterror = alloca swifterror ptr, align 8
  store ptr null, ptr %swifterror, align 8
  %.sroa.0.sroa.6.sroa.0 = alloca i8, align 4
  %11 = alloca <{ %Ts6UInt64V, %Ts6UInt64V }>, align 8
  %12 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64 0) #27
  %13 = extractvalue %swift.metadata_response %12, 0
  %14 = getelementptr inbounds ptr, ptr %13, i64 -1
  %.valueWitnesses = load ptr, ptr %14, align 8, !invariant.load !33, !dereferenceable !34
  %15 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses, i64 0, i32 8
  %size = load i64, ptr %15, align 8, !invariant.load !33
  %16 = alloca i8, i64 %size, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %16)
  %17 = tail call swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE20AutoreleaseFrequencyOMa"(i64 0) #27
  %18 = extractvalue %swift.metadata_response %17, 0
  %19 = getelementptr inbounds ptr, ptr %18, i64 -1
  %.valueWitnesses1 = load ptr, ptr %19, align 8, !invariant.load !33, !dereferenceable !34
  %20 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses1, i64 0, i32 8
  %size2 = load i64, ptr %20, align 8, !invariant.load !33
  %21 = alloca i8, i64 %size2, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %21)
  %22 = tail call swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMa"(i64 0) #27
  %23 = extractvalue %swift.metadata_response %22, 0
  %24 = getelementptr inbounds ptr, ptr %23, i64 -1
  %.valueWitnesses3 = load ptr, ptr %24, align 8, !invariant.load !33, !dereferenceable !34
  %25 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses3, i64 0, i32 8
  %size4 = load i64, ptr %25, align 8, !invariant.load !33
  %26 = alloca i8, i64 %size4, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %26)
  %27 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64 0) #27
  %28 = extractvalue %swift.metadata_response %27, 0
  %29 = getelementptr inbounds ptr, ptr %28, i64 -1
  %.valueWitnesses5 = load ptr, ptr %29, align 8, !invariant.load !33, !dereferenceable !34
  %30 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses5, i64 0, i32 8
  %size6 = load i64, ptr %30, align 8, !invariant.load !33
  %31 = alloca i8, i64 %size6, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %31)
  %32 = tail call swiftcc i32 @"$ss11CommandLineO4argcs5Int32VvgZ"()
  %33 = icmp sgt i32 %32, 1
  br i1 %33, label %34, label %345

34:                                               ; preds = %entry
  %35 = tail call swiftcc ptr @"$ss11CommandLineO9argumentsSaySSGvgZ"()
  %36 = getelementptr inbounds i8, ptr %35, i64 48
  %37 = load i64, ptr %36, align 8
  %._guts20._object._object = getelementptr inbounds i8, ptr %35, i64 56
  %38 = load ptr, ptr %._guts20._object._object, align 8
  %39 = tail call ptr @swift_bridgeObjectRetain(ptr returned %38) #10
  tail call void @swift_bridgeObjectRelease(ptr %35) #10
  %40 = ptrtoint ptr %38 to i64
  %41 = and i64 %40, 2305843009213693952
  %42 = icmp eq i64 %41, 0
  %43 = lshr i64 %40, 56
  %44 = and i64 %43, 15
  %45 = and i64 %37, 281474976710655
  %46 = select i1 %42, i64 %45, i64 %44
  %47 = icmp eq i64 %46, 0
  br i1 %47, label %48, label %49, !prof !35

48:                                               ; preds = %34
  tail call void @swift_bridgeObjectRelease(ptr %38) #10
  br label %344

49:                                               ; preds = %34
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %.sroa.0.sroa.6.sroa.0)
  %50 = and i64 %40, 1152921504606846976
  %51 = icmp eq i64 %50, 0
  br i1 %51, label %52, label %89, !prof !36

52:                                               ; preds = %49
  br i1 %42, label %53, label %56

53:                                               ; preds = %52
  %54 = and i64 %37, 1152921504606846976
  %55 = icmp eq i64 %54, 0
  br i1 %55, label %63, label %67, !prof !35

56:                                               ; preds = %52
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %11)
  %.elt22 = getelementptr inbounds <{ %Ts6UInt64V, %Ts6UInt64V }>, ptr %11, i64 0, i32 1
  %57 = and i64 %40, 72057594037927935
  store i64 %37, ptr %11, align 8
  store i64 %57, ptr %.elt22, align 8
  %58 = trunc i64 %37 to i8
  %59 = lshr i64 %37, 8
  %60 = trunc i64 %59 to i8
  %61 = lshr i64 %37, 16
  %62 = trunc i64 %61 to i8
  switch i8 %58, label %276 [
    i8 45, label %224
    i8 43, label %274
  ]

63:                                               ; preds = %53
  %64 = tail call swiftcc { i64, i64 } @"$ss13_StringObjectV10sharedUTF8SRys5UInt8VGvg"(i64 %37, ptr %38)
  %65 = extractvalue { i64, i64 } %64, 0
  %66 = extractvalue { i64, i64 } %64, 1
  br label %70

67:                                               ; preds = %53
  %68 = and i64 %40, 1152921504606846975
  %69 = add nuw nsw i64 %68, 32
  br label %70

70:                                               ; preds = %63, %67
  %71 = phi i64 [ %69, %67 ], [ %65, %63 ]
  %72 = phi i64 [ %45, %67 ], [ %66, %63 ]
  %73 = inttoptr i64 %71 to ptr
  %74 = load i8, ptr %73, align 1
  switch i8 %74, label %143 [
    i8 45, label %75
    i8 43, label %140
  ]

75:                                               ; preds = %70
  %76 = add i64 %72, -1
  %77 = icmp eq i64 %76, 0
  br i1 %77, label %.loopexit, label %78, !prof !35

78:                                               ; preds = %75
  %79 = icmp eq i64 %71, 0
  %80 = getelementptr inbounds %Ts5UInt8V, ptr %73, i64 1
  %81 = ptrtoint ptr %80 to i64
  %82 = select i1 %79, i64 0, i64 %81
  %83 = inttoptr i64 %82 to ptr
  br i1 %79, label %.loopexit, label %94

.loopexit.loopexit.split.loop.exit:               ; preds = %220
  %84 = extractvalue { i32, i1 } %218, 0
  br label %.loopexit

.loopexit.loopexit294.split.loop.exit:            ; preds = %136
  %85 = extractvalue { i32, i1 } %134, 0
  br label %.loopexit

.loopexit.loopexit297.split.loop.exit:            ; preds = %183
  %86 = extractvalue { i32, i1 } %181, 0
  br label %.loopexit

.loopexit:                                        ; preds = %.loopexit.loopexit297.split.loop.exit, %178, %175, %.preheader124, %.loopexit.loopexit294.split.loop.exit, %131, %128, %.preheader120, %.loopexit.loopexit.split.loop.exit, %215, %212, %.preheader, %201, %196, %193, %189, %187, %143, %164, %159, %156, %151, %145, %140, %119, %114, %111, %106, %104, %99, %94, %78, %75
  %87 = phi i32 [ 0, %75 ], [ 0, %78 ], [ 0, %94 ], [ 0, %99 ], [ %102, %104 ], [ 0, %106 ], [ 0, %111 ], [ 0, %114 ], [ %120, %119 ], [ 0, %140 ], [ 0, %145 ], [ 0, %151 ], [ %157, %156 ], [ 0, %159 ], [ %167, %164 ], [ 0, %143 ], [ 0, %187 ], [ 0, %189 ], [ %194, %193 ], [ 0, %196 ], [ %204, %201 ], [ %84, %.loopexit.loopexit.split.loop.exit ], [ 0, %215 ], [ 0, %212 ], [ 0, %.preheader ], [ %85, %.loopexit.loopexit294.split.loop.exit ], [ 0, %131 ], [ 0, %128 ], [ 0, %.preheader120 ], [ %86, %.loopexit.loopexit297.split.loop.exit ], [ 0, %178 ], [ 0, %175 ], [ 0, %.preheader124 ]
  %88 = phi i1 [ true, %75 ], [ false, %78 ], [ true, %94 ], [ true, %99 ], [ false, %104 ], [ true, %106 ], [ true, %111 ], [ true, %114 ], [ false, %119 ], [ true, %140 ], [ false, %145 ], [ true, %151 ], [ false, %156 ], [ true, %159 ], [ false, %164 ], [ true, %143 ], [ false, %187 ], [ true, %189 ], [ false, %193 ], [ true, %196 ], [ false, %201 ], [ false, %.loopexit.loopexit.split.loop.exit ], [ true, %215 ], [ true, %212 ], [ true, %.preheader ], [ false, %.loopexit.loopexit294.split.loop.exit ], [ true, %131 ], [ true, %128 ], [ true, %.preheader120 ], [ false, %.loopexit.loopexit297.split.loop.exit ], [ true, %178 ], [ true, %175 ], [ true, %.preheader124 ]
  store i1 %88, ptr %.sroa.0.sroa.6.sroa.0, align 4
  br label %92

89:                                               ; preds = %49
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.0.sroa.6.sroa.0)
  %90 = tail call swiftcc i64 @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFSS_s6UInt32VTg5"(i64 %37, ptr %38, i64 10)
  tail call void @swift_bridgeObjectRelease(ptr %38) #10
  %temp-coercion.coerced.sroa.0.0.extract.trunc = trunc i64 %90 to i32
  %91 = and i64 %90, 4294967296
  %.not208 = icmp eq i64 %91, 0
  br i1 %.not208, label %345, label %344

92:                                               ; preds = %.loopexit, %.loopexit129
  %.sroa.0.sroa.0.1 = phi i32 [ %87, %.loopexit ], [ %233, %.loopexit129 ]
  %.sroa.0.sroa.6.sroa.0.0..sroa.0.sroa.6.sroa.0.0..sroa.0.sroa.6.sroa.0.0..sroa.0.sroa.6.sroa.0.0..sroa.0.sroa.6.0..sroa.0.0. = load i8, ptr %.sroa.0.sroa.6.sroa.0, align 4
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %.sroa.0.sroa.6.sroa.0)
  call void @swift_bridgeObjectRelease(ptr %38) #10
  %93 = and i8 %.sroa.0.sroa.6.sroa.0.0..sroa.0.sroa.6.sroa.0.0..sroa.0.sroa.6.sroa.0.0..sroa.0.sroa.6.sroa.0.0..sroa.0.sroa.6.0..sroa.0.0., 1
  %.not118 = icmp eq i8 %93, 0
  br i1 %.not118, label %345, label %344

94:                                               ; preds = %78
  %95 = getelementptr inbounds %Ts5UInt8V, ptr %83, i64 %76
  %96 = load i8, ptr %83, align 1
  %97 = add i8 %96, -48
  %98 = icmp ult i8 %97, 10
  br i1 %98, label %99, label %.loopexit, !prof !37

99:                                               ; preds = %94
  %100 = zext i8 %97 to i32
  %101 = tail call { i32, i1 } @llvm.usub.with.overflow.i32(i32 0, i32 %100)
  %102 = extractvalue { i32, i1 } %101, 0
  %103 = extractvalue { i32, i1 } %101, 1
  br i1 %103, label %.loopexit, label %104, !prof !35

104:                                              ; preds = %99
  %105 = icmp eq i64 %76, 1
  br i1 %105, label %.loopexit, label %106

106:                                              ; preds = %104
  %107 = getelementptr inbounds %Ts5UInt8V, ptr %83, i64 1
  %108 = load i8, ptr %107, align 1
  %109 = add i8 %108, -48
  %110 = icmp ult i8 %109, 10
  br i1 %110, label %111, label %.loopexit, !prof !37

111:                                              ; preds = %106
  %112 = tail call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %102, i32 10)
  %113 = extractvalue { i32, i1 } %112, 1
  br i1 %113, label %.loopexit, label %114

114:                                              ; preds = %111
  %115 = extractvalue { i32, i1 } %112, 0
  %116 = zext i8 %109 to i32
  %117 = tail call { i32, i1 } @llvm.usub.with.overflow.i32(i32 %115, i32 %116)
  %118 = extractvalue { i32, i1 } %117, 1
  br i1 %118, label %.loopexit, label %119, !prof !35

119:                                              ; preds = %114
  %120 = extractvalue { i32, i1 } %117, 0
  %121 = icmp eq i64 %76, 2
  br i1 %121, label %.loopexit, label %.preheader120.preheader

.preheader120.preheader:                          ; preds = %119
  %122 = getelementptr inbounds %Ts5UInt8V, ptr %83, i64 2
  br label %.preheader120

.preheader120:                                    ; preds = %.preheader120.preheader, %136
  %123 = phi ptr [ %138, %136 ], [ %122, %.preheader120.preheader ]
  %124 = phi i32 [ %137, %136 ], [ %120, %.preheader120.preheader ]
  %125 = load i8, ptr %123, align 1
  %126 = add i8 %125, -48
  %127 = icmp ult i8 %126, 10
  br i1 %127, label %128, label %.loopexit, !prof !37

128:                                              ; preds = %.preheader120
  %129 = tail call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %124, i32 10)
  %130 = extractvalue { i32, i1 } %129, 1
  br i1 %130, label %.loopexit, label %131

131:                                              ; preds = %128
  %132 = extractvalue { i32, i1 } %129, 0
  %133 = zext i8 %126 to i32
  %134 = tail call { i32, i1 } @llvm.usub.with.overflow.i32(i32 %132, i32 %133)
  %135 = extractvalue { i32, i1 } %134, 1
  br i1 %135, label %.loopexit, label %136, !prof !35

136:                                              ; preds = %131
  %137 = extractvalue { i32, i1 } %134, 0
  %138 = getelementptr inbounds %Ts5UInt8V, ptr %123, i64 1
  %139 = icmp eq ptr %138, %95
  br i1 %139, label %.loopexit.loopexit294.split.loop.exit, label %.preheader120

140:                                              ; preds = %70
  %141 = add i64 %72, -1
  %142 = icmp eq i64 %141, 0
  br i1 %142, label %.loopexit, label %145, !prof !35

143:                                              ; preds = %70
  %144 = icmp eq i64 %72, 0
  br i1 %144, label %.loopexit, label %187, !prof !35

145:                                              ; preds = %140
  %146 = icmp eq i64 %71, 0
  %147 = getelementptr inbounds %Ts5UInt8V, ptr %73, i64 1
  %148 = ptrtoint ptr %147 to i64
  %149 = select i1 %146, i64 0, i64 %148
  %150 = inttoptr i64 %149 to ptr
  br i1 %146, label %.loopexit, label %151

151:                                              ; preds = %145
  %152 = getelementptr inbounds %Ts5UInt8V, ptr %150, i64 %141
  %153 = load i8, ptr %150, align 1
  %154 = add i8 %153, -48
  %155 = icmp ult i8 %154, 10
  br i1 %155, label %156, label %.loopexit, !prof !37

156:                                              ; preds = %151
  %157 = zext i8 %154 to i32
  %158 = icmp eq i64 %141, 1
  br i1 %158, label %.loopexit, label %159

159:                                              ; preds = %156
  %160 = getelementptr inbounds %Ts5UInt8V, ptr %150, i64 1
  %161 = load i8, ptr %160, align 1
  %162 = add i8 %161, -48
  %163 = icmp ult i8 %162, 10
  br i1 %163, label %164, label %.loopexit, !prof !37

164:                                              ; preds = %159
  %165 = mul nuw nsw i32 %157, 10
  %166 = zext i8 %162 to i32
  %167 = add nuw nsw i32 %165, %166
  %168 = icmp eq i64 %141, 2
  br i1 %168, label %.loopexit, label %.preheader124.preheader

.preheader124.preheader:                          ; preds = %164
  %169 = getelementptr inbounds %Ts5UInt8V, ptr %150, i64 2
  br label %.preheader124

.preheader124:                                    ; preds = %.preheader124.preheader, %183
  %170 = phi ptr [ %185, %183 ], [ %169, %.preheader124.preheader ]
  %171 = phi i32 [ %184, %183 ], [ %167, %.preheader124.preheader ]
  %172 = load i8, ptr %170, align 1
  %173 = add i8 %172, -48
  %174 = icmp ult i8 %173, 10
  br i1 %174, label %175, label %.loopexit, !prof !37

175:                                              ; preds = %.preheader124
  %176 = tail call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %171, i32 10)
  %177 = extractvalue { i32, i1 } %176, 1
  br i1 %177, label %.loopexit, label %178

178:                                              ; preds = %175
  %179 = extractvalue { i32, i1 } %176, 0
  %180 = zext i8 %173 to i32
  %181 = tail call { i32, i1 } @llvm.uadd.with.overflow.i32(i32 %179, i32 %180)
  %182 = extractvalue { i32, i1 } %181, 1
  br i1 %182, label %.loopexit, label %183, !prof !35

183:                                              ; preds = %178
  %184 = extractvalue { i32, i1 } %181, 0
  %185 = getelementptr inbounds %Ts5UInt8V, ptr %170, i64 1
  %186 = icmp eq ptr %185, %152
  br i1 %186, label %.loopexit.loopexit297.split.loop.exit, label %.preheader124

187:                                              ; preds = %143
  %188 = icmp eq i64 %71, 0
  br i1 %188, label %.loopexit, label %189

189:                                              ; preds = %187
  %190 = getelementptr inbounds %Ts5UInt8V, ptr %73, i64 %72
  %191 = add i8 %74, -48
  %192 = icmp ult i8 %191, 10
  br i1 %192, label %193, label %.loopexit, !prof !37

193:                                              ; preds = %189
  %194 = zext i8 %191 to i32
  %195 = icmp eq i64 %72, 1
  br i1 %195, label %.loopexit, label %196

196:                                              ; preds = %193
  %197 = getelementptr inbounds %Ts5UInt8V, ptr %73, i64 1
  %198 = load i8, ptr %197, align 1
  %199 = add i8 %198, -48
  %200 = icmp ult i8 %199, 10
  br i1 %200, label %201, label %.loopexit, !prof !37

201:                                              ; preds = %196
  %202 = mul nuw nsw i32 %194, 10
  %203 = zext i8 %199 to i32
  %204 = add nuw nsw i32 %202, %203
  %205 = icmp eq i64 %72, 2
  br i1 %205, label %.loopexit, label %.preheader.preheader

.preheader.preheader:                             ; preds = %201
  %206 = getelementptr inbounds %Ts5UInt8V, ptr %73, i64 2
  br label %.preheader

.preheader:                                       ; preds = %.preheader.preheader, %220
  %207 = phi ptr [ %222, %220 ], [ %206, %.preheader.preheader ]
  %208 = phi i32 [ %221, %220 ], [ %204, %.preheader.preheader ]
  %209 = load i8, ptr %207, align 1
  %210 = add i8 %209, -48
  %211 = icmp ult i8 %210, 10
  br i1 %211, label %212, label %.loopexit, !prof !37

212:                                              ; preds = %.preheader
  %213 = tail call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %208, i32 10)
  %214 = extractvalue { i32, i1 } %213, 1
  br i1 %214, label %.loopexit, label %215

215:                                              ; preds = %212
  %216 = extractvalue { i32, i1 } %213, 0
  %217 = zext i8 %210 to i32
  %218 = tail call { i32, i1 } @llvm.uadd.with.overflow.i32(i32 %216, i32 %217)
  %219 = extractvalue { i32, i1 } %218, 1
  br i1 %219, label %.loopexit, label %220, !prof !35

220:                                              ; preds = %215
  %221 = extractvalue { i32, i1 } %218, 0
  %222 = getelementptr inbounds %Ts5UInt8V, ptr %207, i64 1
  %223 = icmp eq ptr %222, %190
  br i1 %223, label %.loopexit.loopexit.split.loop.exit, label %.preheader

224:                                              ; preds = %56
  %225 = icmp eq i64 %44, 1
  br i1 %225, label %.loopexit129, label %226, !prof !35

226:                                              ; preds = %224
  %227 = getelementptr inbounds %Ts5UInt8V, ptr %11, i64 %44
  %228 = add i8 %60, -48
  %229 = icmp ult i8 %228, 10
  br i1 %229, label %235, label %.loopexit129, !prof !37

.loopexit129.loopexit.split.loop.exit:            ; preds = %340
  %230 = extractvalue { i32, i1 } %338, 0
  br label %.loopexit129

.loopexit129.loopexit302.split.loop.exit:         ; preds = %270
  %231 = extractvalue { i32, i1 } %268, 0
  br label %.loopexit129

.loopexit129.loopexit305.split.loop.exit:         ; preds = %307
  %232 = extractvalue { i32, i1 } %305, 0
  br label %.loopexit129

.loopexit129:                                     ; preds = %.loopexit129.loopexit305.split.loop.exit, %302, %299, %.preheader136, %.loopexit129.loopexit302.split.loop.exit, %265, %262, %.preheader132, %.loopexit129.loopexit.split.loop.exit, %335, %332, %.preheader128, %321, %318, %315, %311, %276, %288, %285, %282, %278, %274, %253, %248, %245, %242, %240, %235, %226, %224
  %233 = phi i32 [ 0, %224 ], [ 0, %226 ], [ 0, %235 ], [ %238, %240 ], [ 0, %242 ], [ 0, %245 ], [ 0, %248 ], [ %254, %253 ], [ 0, %274 ], [ 0, %278 ], [ %283, %282 ], [ 0, %285 ], [ %291, %288 ], [ 0, %276 ], [ 0, %311 ], [ %316, %315 ], [ 0, %318 ], [ %324, %321 ], [ %230, %.loopexit129.loopexit.split.loop.exit ], [ 0, %335 ], [ 0, %332 ], [ 0, %.preheader128 ], [ %231, %.loopexit129.loopexit302.split.loop.exit ], [ 0, %265 ], [ 0, %262 ], [ 0, %.preheader132 ], [ %232, %.loopexit129.loopexit305.split.loop.exit ], [ 0, %302 ], [ 0, %299 ], [ 0, %.preheader136 ]
  %234 = phi i1 [ true, %224 ], [ true, %226 ], [ true, %235 ], [ false, %240 ], [ true, %242 ], [ true, %245 ], [ true, %248 ], [ false, %253 ], [ true, %274 ], [ true, %278 ], [ false, %282 ], [ true, %285 ], [ false, %288 ], [ true, %276 ], [ true, %311 ], [ false, %315 ], [ true, %318 ], [ false, %321 ], [ false, %.loopexit129.loopexit.split.loop.exit ], [ true, %335 ], [ true, %332 ], [ true, %.preheader128 ], [ false, %.loopexit129.loopexit302.split.loop.exit ], [ true, %265 ], [ true, %262 ], [ true, %.preheader132 ], [ false, %.loopexit129.loopexit305.split.loop.exit ], [ true, %302 ], [ true, %299 ], [ true, %.preheader136 ]
  store i1 %234, ptr %.sroa.0.sroa.6.sroa.0, align 4
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %11)
  br label %92

235:                                              ; preds = %226
  %236 = zext i8 %228 to i32
  %237 = tail call { i32, i1 } @llvm.usub.with.overflow.i32(i32 0, i32 %236)
  %238 = extractvalue { i32, i1 } %237, 0
  %239 = extractvalue { i32, i1 } %237, 1
  br i1 %239, label %.loopexit129, label %240, !prof !35

240:                                              ; preds = %235
  %241 = icmp eq i64 %44, 2
  br i1 %241, label %.loopexit129, label %242

242:                                              ; preds = %240
  %243 = add i8 %62, -48
  %244 = icmp ult i8 %243, 10
  br i1 %244, label %245, label %.loopexit129, !prof !37

245:                                              ; preds = %242
  %246 = tail call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %238, i32 10)
  %247 = extractvalue { i32, i1 } %246, 1
  br i1 %247, label %.loopexit129, label %248

248:                                              ; preds = %245
  %249 = extractvalue { i32, i1 } %246, 0
  %250 = zext i8 %243 to i32
  %251 = tail call { i32, i1 } @llvm.usub.with.overflow.i32(i32 %249, i32 %250)
  %252 = extractvalue { i32, i1 } %251, 1
  br i1 %252, label %.loopexit129, label %253, !prof !35

253:                                              ; preds = %248
  %254 = extractvalue { i32, i1 } %251, 0
  %255 = icmp eq i64 %44, 3
  br i1 %255, label %.loopexit129, label %.preheader132.preheader

.preheader132.preheader:                          ; preds = %253
  %256 = getelementptr inbounds %Ts5UInt8V, ptr %11, i64 3
  br label %.preheader132

.preheader132:                                    ; preds = %.preheader132.preheader, %270
  %257 = phi ptr [ %272, %270 ], [ %256, %.preheader132.preheader ]
  %258 = phi i32 [ %271, %270 ], [ %254, %.preheader132.preheader ]
  %259 = load i8, ptr %257, align 1
  %260 = add i8 %259, -48
  %261 = icmp ult i8 %260, 10
  br i1 %261, label %262, label %.loopexit129, !prof !37

262:                                              ; preds = %.preheader132
  %263 = call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %258, i32 10)
  %264 = extractvalue { i32, i1 } %263, 1
  br i1 %264, label %.loopexit129, label %265

265:                                              ; preds = %262
  %266 = extractvalue { i32, i1 } %263, 0
  %267 = zext i8 %260 to i32
  %268 = call { i32, i1 } @llvm.usub.with.overflow.i32(i32 %266, i32 %267)
  %269 = extractvalue { i32, i1 } %268, 1
  br i1 %269, label %.loopexit129, label %270, !prof !35

270:                                              ; preds = %265
  %271 = extractvalue { i32, i1 } %268, 0
  %272 = getelementptr inbounds %Ts5UInt8V, ptr %257, i64 1
  %273 = icmp eq ptr %272, %227
  br i1 %273, label %.loopexit129.loopexit302.split.loop.exit, label %.preheader132

274:                                              ; preds = %56
  %275 = icmp eq i64 %44, 1
  br i1 %275, label %.loopexit129, label %278, !prof !35

276:                                              ; preds = %56
  %277 = icmp eq i64 %44, 0
  br i1 %277, label %.loopexit129, label %311, !prof !35

278:                                              ; preds = %274
  %279 = getelementptr inbounds %Ts5UInt8V, ptr %11, i64 %44
  %280 = add i8 %60, -48
  %281 = icmp ult i8 %280, 10
  br i1 %281, label %282, label %.loopexit129, !prof !37

282:                                              ; preds = %278
  %283 = zext i8 %280 to i32
  %284 = icmp eq i64 %44, 2
  br i1 %284, label %.loopexit129, label %285

285:                                              ; preds = %282
  %286 = add i8 %62, -48
  %287 = icmp ult i8 %286, 10
  br i1 %287, label %288, label %.loopexit129, !prof !37

288:                                              ; preds = %285
  %289 = mul nuw nsw i32 %283, 10
  %290 = zext i8 %286 to i32
  %291 = add nuw nsw i32 %289, %290
  %292 = icmp eq i64 %44, 3
  br i1 %292, label %.loopexit129, label %.preheader136.preheader

.preheader136.preheader:                          ; preds = %288
  %293 = getelementptr inbounds %Ts5UInt8V, ptr %11, i64 3
  br label %.preheader136

.preheader136:                                    ; preds = %.preheader136.preheader, %307
  %294 = phi ptr [ %309, %307 ], [ %293, %.preheader136.preheader ]
  %295 = phi i32 [ %308, %307 ], [ %291, %.preheader136.preheader ]
  %296 = load i8, ptr %294, align 1
  %297 = add i8 %296, -48
  %298 = icmp ult i8 %297, 10
  br i1 %298, label %299, label %.loopexit129, !prof !37

299:                                              ; preds = %.preheader136
  %300 = call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %295, i32 10)
  %301 = extractvalue { i32, i1 } %300, 1
  br i1 %301, label %.loopexit129, label %302

302:                                              ; preds = %299
  %303 = extractvalue { i32, i1 } %300, 0
  %304 = zext i8 %297 to i32
  %305 = call { i32, i1 } @llvm.uadd.with.overflow.i32(i32 %303, i32 %304)
  %306 = extractvalue { i32, i1 } %305, 1
  br i1 %306, label %.loopexit129, label %307, !prof !35

307:                                              ; preds = %302
  %308 = extractvalue { i32, i1 } %305, 0
  %309 = getelementptr inbounds %Ts5UInt8V, ptr %294, i64 1
  %310 = icmp eq ptr %309, %279
  br i1 %310, label %.loopexit129.loopexit305.split.loop.exit, label %.preheader136

311:                                              ; preds = %276
  %312 = getelementptr inbounds %Ts5UInt8V, ptr %11, i64 %44
  %313 = add i8 %58, -48
  %314 = icmp ult i8 %313, 10
  br i1 %314, label %315, label %.loopexit129, !prof !37

315:                                              ; preds = %311
  %316 = zext i8 %313 to i32
  %317 = icmp eq i64 %44, 1
  br i1 %317, label %.loopexit129, label %318

318:                                              ; preds = %315
  %319 = add i8 %60, -48
  %320 = icmp ult i8 %319, 10
  br i1 %320, label %321, label %.loopexit129, !prof !37

321:                                              ; preds = %318
  %322 = mul nuw nsw i32 %316, 10
  %323 = zext i8 %319 to i32
  %324 = add nuw nsw i32 %322, %323
  %325 = icmp eq i64 %44, 2
  br i1 %325, label %.loopexit129, label %.preheader128.preheader

.preheader128.preheader:                          ; preds = %321
  %326 = getelementptr inbounds %Ts5UInt8V, ptr %11, i64 2
  br label %.preheader128

.preheader128:                                    ; preds = %.preheader128.preheader, %340
  %327 = phi ptr [ %342, %340 ], [ %326, %.preheader128.preheader ]
  %328 = phi i32 [ %341, %340 ], [ %324, %.preheader128.preheader ]
  %329 = load i8, ptr %327, align 1
  %330 = add i8 %329, -48
  %331 = icmp ult i8 %330, 10
  br i1 %331, label %332, label %.loopexit129, !prof !37

332:                                              ; preds = %.preheader128
  %333 = call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %328, i32 10)
  %334 = extractvalue { i32, i1 } %333, 1
  br i1 %334, label %.loopexit129, label %335

335:                                              ; preds = %332
  %336 = extractvalue { i32, i1 } %333, 0
  %337 = zext i8 %330 to i32
  %338 = call { i32, i1 } @llvm.uadd.with.overflow.i32(i32 %336, i32 %337)
  %339 = extractvalue { i32, i1 } %338, 1
  br i1 %339, label %.loopexit129, label %340, !prof !35

340:                                              ; preds = %335
  %341 = extractvalue { i32, i1 } %338, 0
  %342 = getelementptr inbounds %Ts5UInt8V, ptr %327, i64 1
  %343 = icmp eq ptr %342, %312
  br i1 %343, label %.loopexit129.loopexit.split.loop.exit, label %.preheader128

344:                                              ; preds = %89, %92, %48
  br label %345

345:                                              ; preds = %89, %entry, %344, %92
  %storemerge = phi i32 [ 10, %344 ], [ %.sroa.0.sroa.0.1, %92 ], [ 10, %entry ], [ %temp-coercion.coerced.sroa.0.0.extract.trunc, %89 ]
  store i32 %storemerge, ptr @"$s4main1ns6UInt32Vvp", align 4
  store i32 4, ptr @"$s4main8minDepths6UInt32Vvp", align 4
  %346 = call i32 @llvm.umax.i32(i32 %storemerge, i32 6)
  store i32 %346, ptr @"$s4main8maxDepths6UInt32Vvp", align 4
  %347 = call swiftcc ptr @"$sSD17dictionaryLiteralSDyxq_Gx_q_td_tcfCs6UInt32V_SSTgm5Tf4g_n"(ptr nonnull @_swiftEmptyArrayStorage)
  store ptr %347, ptr @"$s4main8messagesSDys6UInt32VSSGvp", align 8
  %348 = load i32, ptr @"$s4main8maxDepths6UInt32Vvp", align 4
  %349 = add i32 %348, 1
  store i32 %349, ptr @"$s4main5depths6UInt32Vvp", align 4
  %350 = call ptr @dispatch_group_create()
  store ptr %350, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  %351 = call swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueCMa"(i64 0) #27
  %352 = extractvalue %swift.metadata_response %351, 0
  call swiftcc void @"$s8Dispatch0A3QoSV15userInteractiveACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %31)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE10AttributesV10concurrentAEvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %26)
  %353 = load i32, ptr @"$sSo17OS_dispatch_queueC8DispatchE20AutoreleaseFrequencyO7inherityA2EmFWC", align 4
  %354 = getelementptr inbounds ptr, ptr %.valueWitnesses1, i64 13
  %355 = load ptr, ptr %354, align 8, !invariant.load !33
  call void %355(ptr noalias nonnull %21, i32 %353, ptr %18) #10
  %356 = call swiftcc ptr @"$sSo17OS_dispatch_queueC8DispatchE5label3qos10attributes20autoreleaseFrequency6targetABSS_AC0D3QoSVAbCE10AttributesVAbCE011AutoreleaseI0OABSgtcfC"(i64 8453663755471056759, ptr nonnull inttoptr (i64 -1513209474789837467 to ptr), ptr noalias nocapture nonnull %31, ptr noalias nocapture nonnull %26, ptr noalias nocapture nonnull %21, i64 0, ptr swiftself %352)
  store ptr %356, ptr @"$s4main11workerQueueSo17OS_dispatch_queueCvp", align 8
  call swiftcc void @"$s8Dispatch0A3QoSV10backgroundACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %31)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %2)
  store ptr @_swiftEmptyArrayStorage, ptr %2, align 8
  %357 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWL", ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMa", ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVs10SetAlgebraACMc")
  %358 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGMD") #28
  %359 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWL", ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %26, ptr noalias nocapture nonnull %2, ptr %358, ptr %359, ptr swiftself %23, ptr %23, ptr %357)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %2)
  call void %355(ptr noalias nonnull %21, i32 %353, ptr %18) #10
  %360 = call swiftcc ptr @"$sSo17OS_dispatch_queueC8DispatchE5label3qos10attributes20autoreleaseFrequency6targetABSS_AC0D3QoSVAbCE10AttributesVAbCE011AutoreleaseI0OABSgtcfC"(i64 5865207757966370157, ptr nonnull inttoptr (i64 -1441151879056366219 to ptr), ptr noalias nocapture nonnull %31, ptr noalias nocapture nonnull %26, ptr noalias nocapture nonnull %21, i64 0, ptr swiftself %352)
  store ptr %360, ptr @"$s4main12messageQueueSo17OS_dispatch_queueCvp", align 8
  %361 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  call void @dispatch_group_enter(ptr %361)
  %362 = load ptr, ptr @"$s4main11workerQueueSo17OS_dispatch_queueCvp", align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %3)
  %363 = getelementptr inbounds { %objc_block, %swift.function }, ptr %3, i64 0, i32 1
  store ptr @"$s4mainyyYbcfU0_", ptr %363, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %3, i64 0, i32 1, i32 1
  store ptr null, ptr %.data, align 8
  store ptr @_NSConcreteStackBlock, ptr %3, align 8
  %364 = getelementptr inbounds %objc_block, ptr %3, i64 0, i32 1
  store <2 x i32> <i32 1107296256, i32 0>, ptr %364, align 8
  %365 = getelementptr inbounds %objc_block, ptr %3, i64 0, i32 3
  store ptr @"$sIegh_IeyBh_TR", ptr %365, align 8
  %366 = getelementptr inbounds %objc_block, ptr %3, i64 0, i32 4
  store ptr @block_descriptor, ptr %366, align 8
  %367 = call ptr @_Block_copy(ptr nonnull %3)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %31)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4)
  store ptr @_swiftEmptyArrayStorage, ptr %4, align 8
  %368 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL", ptr @"$s8Dispatch0A13WorkItemFlagsVMa", ptr @"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc")
  %369 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSay8Dispatch0A13WorkItemFlagsVGMD") #28
  %370 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL", ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %16, ptr noalias nocapture nonnull %4, ptr %369, ptr %370, ptr swiftself %13, ptr %13, ptr %368)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %31, ptr noalias nocapture nonnull %16, ptr %367, ptr swiftself %362)
  %371 = getelementptr inbounds ptr, ptr %.valueWitnesses, i64 1
  %372 = load ptr, ptr %371, align 8, !invariant.load !33
  call void %372(ptr noalias nonnull %16, ptr %13) #10
  %373 = getelementptr inbounds ptr, ptr %.valueWitnesses5, i64 1
  %374 = load ptr, ptr %373, align 8, !invariant.load !33
  call void %374(ptr noalias nonnull %31, ptr %28) #10
  call void @_Block_release(ptr %367) #10
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %3)
  %375 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  call void @dispatch_group_enter(ptr %375)
  %376 = load ptr, ptr @"$s4main11workerQueueSo17OS_dispatch_queueCvp", align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %5)
  %377 = getelementptr inbounds { %objc_block, %swift.function }, ptr %5, i64 0, i32 1
  store ptr @"$s4mainyyYbcfU1_", ptr %377, align 8
  %.data10 = getelementptr inbounds { %objc_block, %swift.function }, ptr %5, i64 0, i32 1, i32 1
  store ptr null, ptr %.data10, align 8
  store ptr @_NSConcreteStackBlock, ptr %5, align 8
  %378 = getelementptr inbounds %objc_block, ptr %5, i64 0, i32 1
  store <2 x i32> <i32 1107296256, i32 0>, ptr %378, align 8
  %379 = getelementptr inbounds %objc_block, ptr %5, i64 0, i32 3
  store ptr @"$sIegh_IeyBh_TR", ptr %379, align 8
  %380 = getelementptr inbounds %objc_block, ptr %5, i64 0, i32 4
  store ptr @block_descriptor.3, ptr %380, align 8
  %381 = call ptr @_Block_copy(ptr nonnull %5)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %31)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6)
  store ptr @_swiftEmptyArrayStorage, ptr %6, align 8
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %16, ptr noalias nocapture nonnull %6, ptr %369, ptr %370, ptr swiftself %13, ptr %13, ptr %368)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %6)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %31, ptr noalias nocapture nonnull %16, ptr %381, ptr swiftself %376)
  call void %372(ptr noalias nonnull %16, ptr %13) #10
  call void %374(ptr noalias nonnull %31, ptr %28) #10
  call void @_Block_release(ptr %381) #10
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %5)
  %382 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  call void @dispatch_group_enter(ptr %382)
  %383 = load ptr, ptr @"$s4main11workerQueueSo17OS_dispatch_queueCvp", align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %7)
  %384 = getelementptr inbounds { %objc_block, %swift.function }, ptr %7, i64 0, i32 1
  store ptr @"$s4mainyyYbcfU2_", ptr %384, align 8
  %.data13 = getelementptr inbounds { %objc_block, %swift.function }, ptr %7, i64 0, i32 1, i32 1
  store ptr null, ptr %.data13, align 8
  store ptr @_NSConcreteStackBlock, ptr %7, align 8
  %385 = getelementptr inbounds %objc_block, ptr %7, i64 0, i32 1
  store <2 x i32> <i32 1107296256, i32 0>, ptr %385, align 8
  %386 = getelementptr inbounds %objc_block, ptr %7, i64 0, i32 3
  store ptr @"$sIegh_IeyBh_TR", ptr %386, align 8
  %387 = getelementptr inbounds %objc_block, ptr %7, i64 0, i32 4
  store ptr @block_descriptor.6, ptr %387, align 8
  %388 = call ptr @_Block_copy(ptr nonnull %7)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %31)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %8)
  store ptr @_swiftEmptyArrayStorage, ptr %8, align 8
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %16, ptr noalias nocapture nonnull %8, ptr %369, ptr %370, ptr swiftself %13, ptr %13, ptr %368)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %8)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %31, ptr noalias nocapture nonnull %16, ptr %388, ptr swiftself %383)
  call void %372(ptr noalias nonnull %16, ptr %13) #10
  call void %374(ptr noalias nonnull %31, ptr %28) #10
  call void @_Block_release(ptr %388) #10
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %7)
  %389 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  call swiftcc void @"$sSo17OS_dispatch_groupC8DispatchE4waityyF"(ptr swiftself %389)
  %390 = load ptr, ptr @"$s4main8messagesSDys6UInt32VSSGvp", align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %9)
  %391 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %390, i64 0, i32 1
  %392 = load i64, ptr %391, align 8, !range !38
  %393 = icmp eq i64 %392, 0
  br i1 %393, label %394, label %396

394:                                              ; preds = %345
  %395 = call ptr @swift_bridgeObjectRetain(ptr nonnull returned %390) #10
  br label %411

396:                                              ; preds = %345
  %397 = call swiftcc ptr @"$ss22_ContiguousArrayBufferV19_uninitializedCount15minimumCapacityAByxGSi_SitcfCs6UInt32V3key_SS5valuet_Tgm5"(i64 %392, i64 0)
  %398 = getelementptr inbounds i8, ptr %397, i64 32
  %399 = ptrtoint ptr %398 to i64
  call void @llvm.lifetime.start.p0(i64 40, ptr nonnull %10)
  %400 = call swiftcc i64 @"$sSTsE21_copySequenceContents12initializing8IteratorQz_SitSry7ElementQzG_tFSDys6UInt32VSSG_Tg5"(ptr noalias nocapture nonnull %10, i64 %399, i64 %392, ptr nonnull %390)
  %401 = load i64, ptr %10, align 8
  %402 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %10, i64 0, i32 1
  %403 = load i64, ptr %402, align 8
  %404 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %10, i64 0, i32 2
  %405 = load i64, ptr %404, align 8
  %406 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %10, i64 0, i32 3
  %407 = load i64, ptr %406, align 8
  %408 = getelementptr inbounds { i64, i64, i64, i64, i64 }, ptr %10, i64 0, i32 4
  %409 = load i64, ptr %408, align 8
  %410 = call ptr @swift_bridgeObjectRetain_n(ptr %390, i32 3)
  call void @"$sSD8IteratorV8_VariantOyxq___GSHRzr0_lWOe"(i64 %401, i64 %403, i64 %405, i64 %407, i64 %409)
  call void @swift_bridgeObjectRelease(ptr nonnull %390) #10
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %10)
  br label %411

411:                                              ; preds = %394, %396
  %412 = phi ptr [ %397, %396 ], [ @_swiftEmptyArrayStorage, %394 ]
  store ptr %412, ptr %9, align 8
  call swiftcc void @"$sSMsSkRzrlE4sort2byySb7ElementSTQz_ADtKXE_tKFs15ContiguousArrayVys6UInt32V3key_SS5valuetG_Tg5011$s4mainSbs6F32V3key_SS5valuet_AcD_SSAEttXEfU3_Tf1cn_n"(ptr nocapture nonnull dereferenceable(8) %9, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %swifterror)
  %413 = load ptr, ptr %swifterror, align 8
  %.not = icmp eq ptr %413, null
  br i1 %.not, label %414, label %419

414:                                              ; preds = %411
  call void @swift_bridgeObjectRelease(ptr nonnull %390) #10
  %415 = load ptr, ptr %9, align 8
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %9)
  %416 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %415, i64 0, i32 1
  %417 = load i64, ptr %416, align 8, !range !38
  %418 = icmp eq i64 %417, 0
  br i1 %418, label %.loopexit283, label %421

419:                                              ; preds = %411
  %420 = load ptr, ptr %9, align 8
  call void @swift_release(ptr %420) #10
  unreachable

421:                                              ; preds = %414
  %422 = getelementptr inbounds i8, ptr %415, i64 32
  br label %423

.loopexit283:                                     ; preds = %423, %414
  call void @swift_release(ptr nonnull %415) #10
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %31)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %26)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %21)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %16)
  ret i32 0

423:                                              ; preds = %423, %421
  %424 = phi i64 [ 0, %421 ], [ %428, %423 ]
  %425 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %422, i64 %424
  %.value = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %425, i64 0, i32 2
  %426 = load i64, ptr %.value, align 8
  %.value._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.value, i64 0, i32 1
  %427 = load ptr, ptr %.value._guts._object._object, align 8
  %428 = add nuw nsw i64 %424, 1
  %429 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCyypGMD") #28
  %430 = call noalias ptr @swift_allocObject(ptr %429, i64 64, i64 7) #10
  %431 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %430, i64 0, i32 1
  store <2 x i64> <i64 1, i64 2>, ptr %431, align 8
  %432 = getelementptr inbounds i8, ptr %430, i64 32
  %433 = getelementptr inbounds i8, ptr %430, i64 56
  store ptr @"$sSSN", ptr %433, align 8
  store i64 %426, ptr %432, align 8
  %._guts._object._object = getelementptr inbounds i8, ptr %430, i64 40
  store ptr %427, ptr %._guts._object._object, align 8
  %434 = call ptr @swift_bridgeObjectRetain_n(ptr %427, i32 2)
  call swiftcc void @"$ss5print_9separator10terminatoryypd_S2StF"(ptr %430, i64 32, ptr nonnull inttoptr (i64 -2233785415175766016 to ptr), i64 10, ptr nonnull inttoptr (i64 -2233785415175766016 to ptr))
  call void @swift_bridgeObjectRelease(ptr %427) #10
  call void @swift_release(ptr %430) #10
  %435 = icmp eq i64 %428, %417
  br i1 %435, label %.loopexit283, label %423
}

declare swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64) local_unnamed_addr #0

; Function Attrs: argmemonly mustprogress nocallback nofree nosync nounwind willreturn
declare void @llvm.lifetime.start.p0(i64 immarg, ptr nocapture) #1

declare swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE20AutoreleaseFrequencyOMa"(i64) local_unnamed_addr #0

declare swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVMa"(i64) local_unnamed_addr #0

declare swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64) local_unnamed_addr #0

declare swiftcc i32 @"$ss11CommandLineO4argcs5Int32VvgZ"() local_unnamed_addr #0

; Function Attrs: mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i32, i1 } @llvm.uadd.with.overflow.i32(i32, i32) #2

define internal swiftcc void @"$s4mainyyYbcfU0_"() #0 {
  tail call swiftcc void @"$s4mainyyYbcfU0_Tm"(ptr @"$s4main5depths6UInt32Vvp", ptr getelementptr inbounds (%swift.full_boxmetadata, ptr @metadata.31, i64 0, i32 2), ptr @"$s4mainyyYbcfU0_yyYbcfU_TA", ptr @block_descriptor.34) #0
  ret void
}

; Function Attrs: nounwind
define hidden swiftcc i64 @"$s4main12bottomUpTreeyAA0D0Os6UInt32VF"(i32 %0) local_unnamed_addr #3 {
entry:
  %.not = icmp eq i32 %0, 0
  br i1 %.not, label %7, label %1

1:                                                ; preds = %entry
  %2 = add i32 %0, -1
  %3 = tail call swiftcc i64 @"$s4main12bottomUpTreeyAA0D0Os6UInt32VF"(i32 %2)
  %4 = tail call swiftcc i64 @"$s4main12bottomUpTreeyAA0D0Os6UInt32VF"(i32 %2)
  %5 = tail call noalias ptr @swift_allocObject(ptr nonnull getelementptr inbounds (%swift.full_boxmetadata, ptr @metadata.21, i64 0, i32 2), i64 32, i64 7) #10
  %6 = getelementptr inbounds <{ %swift.refcounted, <{ %T4main4TreeO, %T4main4TreeO }> }>, ptr %5, i64 0, i32 1
  %.right2 = getelementptr inbounds <{ %swift.refcounted, <{ %T4main4TreeO, %T4main4TreeO }> }>, ptr %5, i64 0, i32 1, i32 1
  store i64 %3, ptr %6, align 8
  store i64 %4, ptr %.right2, align 8
  br label %10

7:                                                ; preds = %entry
  %8 = tail call noalias ptr @swift_allocObject(ptr nonnull getelementptr inbounds (%swift.full_boxmetadata, ptr @metadata.21, i64 0, i32 2), i64 32, i64 7) #10
  %9 = getelementptr inbounds <{ %swift.refcounted, <{ %T4main4TreeO, %T4main4TreeO }> }>, ptr %8, i64 0, i32 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %9, i8 0, i64 16, i1 false)
  br label %10

10:                                               ; preds = %1, %7
  %.in = phi ptr [ %8, %7 ], [ %5, %1 ]
  %11 = ptrtoint ptr %.in to i64
  ret i64 %11
}

; Function Attrs: nofree nosync nounwind readonly
define hidden swiftcc i32 @"$s4main9itemCheckys6UInt32VAA4TreeOF"(i64 %0) local_unnamed_addr #4 {
entry:
  %1 = icmp eq i64 %0, 0
  br i1 %1, label %._crit_edge, label %.lr.ph

.lr.ph:                                           ; preds = %entry, %tailrecurse
  %.tr2 = phi i64 [ %5, %tailrecurse ], [ %0, %entry ]
  %accumulator.tr1 = phi i32 [ %10, %tailrecurse ], [ 0, %entry ]
  %2 = inttoptr i64 %.tr2 to ptr
  %3 = getelementptr inbounds <{ %swift.refcounted, <{ %T4main4TreeO, %T4main4TreeO }> }>, ptr %2, i64 0, i32 1
  %4 = load i64, ptr %3, align 8
  %.right = getelementptr inbounds <{ %swift.refcounted, <{ %T4main4TreeO, %T4main4TreeO }> }>, ptr %2, i64 0, i32 1, i32 1
  %5 = load i64, ptr %.right, align 8
  %6 = icmp eq i64 %4, 0
  %7 = icmp eq i64 %5, 0
  %or.cond = select i1 %6, i1 %7, i1 false
  br i1 %or.cond, label %._crit_edge, label %tailrecurse

tailrecurse:                                      ; preds = %.lr.ph
  %8 = tail call swiftcc i32 @"$s4main9itemCheckys6UInt32VAA4TreeOF"(i64 %4)
  %9 = add i32 %accumulator.tr1, 1
  %10 = add i32 %9, %8
  br i1 %7, label %._crit_edge, label %.lr.ph

._crit_edge:                                      ; preds = %tailrecurse, %.lr.ph, %entry
  %accumulator.tr.lcssa = phi i32 [ 0, %entry ], [ %accumulator.tr1, %.lr.ph ], [ %10, %tailrecurse ]
  %accumulator.ret.tr = add i32 %accumulator.tr.lcssa, 1
  ret i32 %accumulator.ret.tr
}

define linkonce_odr hidden void @"$sIegh_IeyBh_TR"(ptr %0) #0 {
entry:
  %1 = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1
  %2 = load ptr, ptr %1, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1, i32 1
  %3 = load ptr, ptr %.data, align 8
  %4 = tail call ptr @swift_retain(ptr returned %3) #29
  tail call swiftcc void %2(ptr swiftself %3) #30
  tail call void @swift_release(ptr %3) #10
  ret void
}

define internal swiftcc void @"$s4mainyyYbcfU1_"() #0 {
  tail call swiftcc void @"$s4mainyyYbcfU0_Tm"(ptr @"$s4main8maxDepths6UInt32Vvp", ptr getelementptr inbounds (%swift.full_boxmetadata, ptr @metadata.25, i64 0, i32 2), ptr @"$s4mainyyYbcfU1_yyYbcfU_TA", ptr @block_descriptor.28) #0
  ret void
}

define internal swiftcc void @"$s4mainyyYbcfU0_Tm"(ptr %0, ptr %1, ptr %2, ptr %3) #0 {
entry:
  %4 = alloca { %objc_block, %swift.function }, align 8
  %5 = alloca %TSa, align 8
  %6 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64 0) #27
  %7 = extractvalue %swift.metadata_response %6, 0
  %8 = getelementptr inbounds ptr, ptr %7, i64 -1
  %.valueWitnesses = load ptr, ptr %8, align 8, !invariant.load !33, !dereferenceable !34
  %9 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses, i64 0, i32 8
  %size = load i64, ptr %9, align 8, !invariant.load !33
  %10 = alloca i8, i64 %size, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %10)
  %11 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64 0) #27
  %12 = extractvalue %swift.metadata_response %11, 0
  %13 = getelementptr inbounds ptr, ptr %12, i64 -1
  %.valueWitnesses1 = load ptr, ptr %13, align 8, !invariant.load !33, !dereferenceable !34
  %14 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses1, i64 0, i32 8
  %size2 = load i64, ptr %14, align 8, !invariant.load !33
  %15 = alloca i8, i64 %size2, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %15)
  %16 = load i32, ptr %0, align 4
  %17 = tail call swiftcc i64 @"$s4main12bottomUpTreeyAA0D0Os6UInt32VF"(i32 %16)
  %18 = load ptr, ptr @"$s4main12messageQueueSo17OS_dispatch_queueCvp", align 8
  %19 = tail call noalias ptr @swift_allocObject(ptr nonnull %1, i64 24, i64 7) #10
  %20 = getelementptr inbounds <{ %swift.refcounted, %T4main4TreeO }>, ptr %19, i64 0, i32 1
  store i64 %17, ptr %20, align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %4)
  %21 = getelementptr inbounds { %objc_block, %swift.function }, ptr %4, i64 0, i32 1
  store ptr %2, ptr %21, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %4, i64 0, i32 1, i32 1
  store ptr %19, ptr %.data, align 8
  store ptr @_NSConcreteStackBlock, ptr %4, align 8
  %22 = getelementptr inbounds %objc_block, ptr %4, i64 0, i32 1
  store <2 x i32> <i32 1107296256, i32 0>, ptr %22, align 8
  %23 = getelementptr inbounds %objc_block, ptr %4, i64 0, i32 3
  store ptr @"$sIegh_IeyBh_TR", ptr %23, align 8
  %24 = getelementptr inbounds %objc_block, ptr %4, i64 0, i32 4
  store ptr %3, ptr %24, align 8
  %25 = call ptr @_Block_copy(ptr nonnull %4)
  %26 = inttoptr i64 %17 to ptr
  %27 = call ptr @swift_retain(ptr returned %26) #29
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %15)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %5)
  store ptr @_swiftEmptyArrayStorage, ptr %5, align 8
  %28 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL", ptr @"$s8Dispatch0A13WorkItemFlagsVMa", ptr @"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc")
  %29 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSay8Dispatch0A13WorkItemFlagsVGMD") #28
  %30 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL", ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %10, ptr noalias nocapture nonnull %5, ptr %29, ptr %30, ptr swiftself %7, ptr %7, ptr %28)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %5)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %15, ptr noalias nocapture nonnull %10, ptr %25, ptr swiftself %18)
  %31 = getelementptr inbounds ptr, ptr %.valueWitnesses, i64 1
  %32 = load ptr, ptr %31, align 8, !invariant.load !33
  call void %32(ptr noalias nonnull %10, ptr %7) #10
  %33 = getelementptr inbounds ptr, ptr %.valueWitnesses1, i64 1
  %34 = load ptr, ptr %33, align 8, !invariant.load !33
  call void %34(ptr noalias nonnull %15, ptr %12) #10
  call void @_Block_release(ptr %25) #10
  call void @swift_release(ptr %26) #10
  %35 = load ptr, ptr %.data, align 8
  call void @swift_release(ptr %35) #10
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %4)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %15)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %10)
  ret void
}

define internal swiftcc void @"$s4mainyyYbcfU2_"() #0 {
entry:
  %0 = alloca { %objc_block, %swift.function }, align 8
  %1 = alloca %TSa, align 8
  %2 = alloca { %objc_block, %swift.function }, align 8
  %3 = alloca %TSa, align 8
  %4 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64 0) #27
  %5 = extractvalue %swift.metadata_response %4, 0
  %6 = getelementptr inbounds ptr, ptr %5, i64 -1
  %.valueWitnesses = load ptr, ptr %6, align 8, !invariant.load !33, !dereferenceable !34
  %7 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses, i64 0, i32 8
  %size = load i64, ptr %7, align 8, !invariant.load !33
  %8 = alloca i8, i64 %size, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %8)
  %9 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64 0) #27
  %10 = extractvalue %swift.metadata_response %9, 0
  %11 = getelementptr inbounds ptr, ptr %10, i64 -1
  %.valueWitnesses1 = load ptr, ptr %11, align 8, !invariant.load !33, !dereferenceable !34
  %12 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses1, i64 0, i32 8
  %size2 = load i64, ptr %12, align 8, !invariant.load !33
  %13 = alloca i8, i64 %size2, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %13)
  %14 = load i32, ptr @"$s4main8minDepths6UInt32Vvp", align 4
  %15 = lshr i32 %14, 1
  %16 = load i32, ptr @"$s4main8maxDepths6UInt32Vvp", align 4
  %17 = lshr i32 %16, 1
  %18 = add nuw i32 %17, 1
  %19 = icmp eq i32 %15, %18
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
  store ptr @"$s4mainyyYbcfU2_yyYbcfU0_", ptr %27, align 8
  %.data7 = getelementptr inbounds { %objc_block, %swift.function }, ptr %2, i64 0, i32 1, i32 1
  store ptr null, ptr %.data7, align 8
  store ptr @_NSConcreteStackBlock, ptr %2, align 8
  %28 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 1
  store <2 x i32> <i32 1107296256, i32 0>, ptr %28, align 8
  %29 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 3
  store ptr @"$sIegh_IeyBh_TR", ptr %29, align 8
  %30 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 4
  store ptr @block_descriptor.12, ptr %30, align 8
  %31 = call ptr @_Block_copy(ptr nonnull %2)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %13)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3)
  store ptr @_swiftEmptyArrayStorage, ptr %3, align 8
  %32 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL", ptr @"$s8Dispatch0A13WorkItemFlagsVMa", ptr @"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc")
  %33 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSay8Dispatch0A13WorkItemFlagsVGMD") #28
  %34 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL", ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %8, ptr noalias nocapture nonnull %3, ptr %33, ptr %34, ptr swiftself %5, ptr %5, ptr %32)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %13, ptr noalias nocapture nonnull %8, ptr %31, ptr swiftself %26)
  %35 = getelementptr inbounds ptr, ptr %.valueWitnesses, i64 1
  %36 = load ptr, ptr %35, align 8, !invariant.load !33
  call void %36(ptr noalias nonnull %8, ptr %5) #10
  %37 = getelementptr inbounds ptr, ptr %.valueWitnesses1, i64 1
  %38 = load ptr, ptr %37, align 8, !invariant.load !33
  call void %38(ptr noalias nonnull %13, ptr %10) #10
  call void @_Block_release(ptr %31) #10
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %2)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %13)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %8)
  ret void

39:                                               ; preds = %.preheader, %39
  %40 = phi i32 [ %50, %39 ], [ %15, %.preheader ]
  %41 = shl i32 %40, 1
  %42 = load i32, ptr @"$s4main8maxDepths6UInt32Vvp", align 4
  %43 = sub i32 %42, %41
  %44 = load i32, ptr @"$s4main8minDepths6UInt32Vvp", align 4
  %45 = add i32 %43, %44
  %46 = icmp ult i32 %45, 64
  %47 = and i32 %45, 63
  %48 = zext i32 %47 to i64
  %49 = shl nuw i64 1, %48
  %50 = add i32 %40, 1
  %51 = trunc i64 %49 to i32
  %52 = select i1 %46, i32 %51, i32 0, !prof !37
  %53 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  call void @dispatch_group_enter(ptr %53)
  %54 = load ptr, ptr @"$s4main11workerQueueSo17OS_dispatch_queueCvp", align 8
  %55 = call noalias ptr @swift_allocObject(ptr nonnull getelementptr inbounds (%swift.full_boxmetadata, ptr @metadata, i64 0, i32 2), i64 24, i64 7) #10
  %56 = getelementptr inbounds <{ %swift.refcounted, %Ts6UInt32V, %Ts6UInt32V }>, ptr %55, i64 0, i32 1
  store i32 %41, ptr %56, align 8
  %57 = getelementptr inbounds <{ %swift.refcounted, %Ts6UInt32V, %Ts6UInt32V }>, ptr %55, i64 0, i32 2
  store i32 %52, ptr %57, align 4
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %0)
  store ptr @"$s4mainyyYbcfU2_yyYbcfU_TA", ptr %20, align 8
  store ptr %55, ptr %.data, align 8
  store ptr @_NSConcreteStackBlock, ptr %0, align 8
  store <2 x i32> <i32 1107296256, i32 0>, ptr %21, align 8
  store ptr @"$sIegh_IeyBh_TR", ptr %22, align 8
  store ptr @block_descriptor.9, ptr %23, align 8
  %58 = call ptr @_Block_copy(ptr nonnull %0)
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %13)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %1)
  store ptr @_swiftEmptyArrayStorage, ptr %1, align 8
  %59 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL", ptr @"$s8Dispatch0A13WorkItemFlagsVMa", ptr @"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc")
  %60 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSay8Dispatch0A13WorkItemFlagsVGMD") #28
  %61 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL", ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %8, ptr noalias nocapture nonnull %1, ptr %60, ptr %61, ptr swiftself %5, ptr %5, ptr %59)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %1)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %13, ptr noalias nocapture nonnull %8, ptr %58, ptr swiftself %54)
  %62 = load ptr, ptr %24, align 8, !invariant.load !33
  call void %62(ptr noalias nonnull %8, ptr %5) #10
  %63 = load ptr, ptr %25, align 8, !invariant.load !33
  call void %63(ptr noalias nonnull %13, ptr %10) #10
  call void @_Block_release(ptr %58) #10
  %64 = load ptr, ptr %.data, align 8
  call void @swift_release(ptr %64) #10
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %0)
  %65 = icmp eq i32 %40, %17
  br i1 %65, label %.loopexit, label %39
}

define internal swiftcc void @"$s4mainyyYbcfU2_yyYbcfU_"(i32 %0, i32 %1) unnamed_addr #0 {
entry:
  %2 = alloca { %objc_block, %swift.function }, align 8
  %3 = alloca %TSa, align 8
  %4 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A13WorkItemFlagsVMa"(i64 0) #27
  %5 = extractvalue %swift.metadata_response %4, 0
  %6 = getelementptr inbounds ptr, ptr %5, i64 -1
  %.valueWitnesses = load ptr, ptr %6, align 8, !invariant.load !33, !dereferenceable !34
  %7 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses, i64 0, i32 8
  %size = load i64, ptr %7, align 8, !invariant.load !33
  %8 = alloca i8, i64 %size, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %8)
  %9 = tail call swiftcc %swift.metadata_response @"$s8Dispatch0A3QoSVMa"(i64 0) #27
  %10 = extractvalue %swift.metadata_response %9, 0
  %11 = getelementptr inbounds ptr, ptr %10, i64 -1
  %.valueWitnesses1 = load ptr, ptr %11, align 8, !invariant.load !33, !dereferenceable !34
  %12 = getelementptr inbounds %swift.vwtable, ptr %.valueWitnesses1, i64 0, i32 8
  %size2 = load i64, ptr %12, align 8, !invariant.load !33
  %13 = alloca i8, i64 %size2, align 16
  call void @llvm.lifetime.start.p0(i64 -1, ptr nonnull %13)
  %14 = tail call swiftcc { i64, ptr } @"$s4main5inner5depth10iterationsSSs6UInt32V_AFtF"(i32 %0, i32 %1)
  %15 = extractvalue { i64, ptr } %14, 0
  %16 = extractvalue { i64, ptr } %14, 1
  %17 = load ptr, ptr @"$s4main12messageQueueSo17OS_dispatch_queueCvp", align 8
  %18 = tail call noalias ptr @swift_allocObject(ptr nonnull getelementptr inbounds (%swift.full_boxmetadata, ptr @metadata.15, i64 0, i32 2), i64 40, i64 7) #10
  %19 = getelementptr inbounds <{ %swift.refcounted, %Ts6UInt32V, [4 x i8], %TSS }>, ptr %18, i64 0, i32 1
  store i32 %0, ptr %19, align 8
  %20 = getelementptr inbounds <{ %swift.refcounted, %Ts6UInt32V, [4 x i8], %TSS }>, ptr %18, i64 0, i32 3
  store i64 %15, ptr %20, align 8
  %._guts._object._object = getelementptr inbounds <{ %swift.refcounted, %Ts6UInt32V, [4 x i8], %TSS }>, ptr %18, i64 0, i32 3, i32 0, i32 0, i32 1
  store ptr %16, ptr %._guts._object._object, align 8
  call void @llvm.lifetime.start.p0(i64 48, ptr nonnull %2)
  %21 = getelementptr inbounds { %objc_block, %swift.function }, ptr %2, i64 0, i32 1
  store ptr @"$s4mainyyYbcfU2_yyYbcfU_yyYbcfU_TA", ptr %21, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %2, i64 0, i32 1, i32 1
  store ptr %18, ptr %.data, align 8
  store ptr @_NSConcreteStackBlock, ptr %2, align 8
  %22 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 1
  store <2 x i32> <i32 1107296256, i32 0>, ptr %22, align 8
  %23 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 3
  store ptr @"$sIegh_IeyBh_TR", ptr %23, align 8
  %24 = getelementptr inbounds %objc_block, ptr %2, i64 0, i32 4
  store ptr @block_descriptor.18, ptr %24, align 8
  %25 = call ptr @_Block_copy(ptr nonnull %2)
  %26 = call ptr @swift_bridgeObjectRetain(ptr returned %16) #10
  call swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture nonnull sret(%swift.opaque) %13)
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %3)
  store ptr @_swiftEmptyArrayStorage, ptr %3, align 8
  %27 = call ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr @"$s8Dispatch0A13WorkItemFlagsVACs10SetAlgebraAAWL", ptr @"$s8Dispatch0A13WorkItemFlagsVMa", ptr @"$s8Dispatch0A13WorkItemFlagsVs10SetAlgebraAAMc")
  %28 = call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$sSay8Dispatch0A13WorkItemFlagsVGMD") #28
  %29 = call ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr @"$sSay8Dispatch0A13WorkItemFlagsVGSayxGSTsWL", ptr @"$sSay8Dispatch0A13WorkItemFlagsVGMD")
  call swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture nonnull sret(%swift.opaque) %8, ptr noalias nocapture nonnull %3, ptr %28, ptr %29, ptr swiftself %5, ptr %5, ptr %27)
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %3)
  call swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64 0, ptr noalias nocapture nonnull %13, ptr noalias nocapture nonnull %8, ptr %25, ptr swiftself %17)
  %30 = getelementptr inbounds ptr, ptr %.valueWitnesses, i64 1
  %31 = load ptr, ptr %30, align 8, !invariant.load !33
  call void %31(ptr noalias nonnull %8, ptr %5) #10
  %32 = getelementptr inbounds ptr, ptr %.valueWitnesses1, i64 1
  %33 = load ptr, ptr %32, align 8, !invariant.load !33
  call void %33(ptr noalias nonnull %13, ptr %10) #10
  call void @_Block_release(ptr %25) #10
  call void @swift_bridgeObjectRelease(ptr %16) #10
  %34 = load ptr, ptr %.data, align 8
  call void @swift_release(ptr %34) #10
  call void @llvm.lifetime.end.p0(i64 48, ptr nonnull %2)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %13)
  call void @llvm.lifetime.end.p0(i64 -1, ptr nonnull %8)
  ret void
}

define hidden swiftcc { i64, ptr } @"$s4main5inner5depth10iterationsSSs6UInt32V_AFtF"(i32 %0, i32 %1) local_unnamed_addr #0 {
entry:
  %2 = alloca %TSS, align 8
  %3 = alloca %Ts11_StringGutsV, align 8
  %4 = alloca %Ts6UInt32V, align 4
  %5 = alloca %Ts6UInt32V, align 4
  %6 = alloca %Ts6UInt32V, align 4
  %7 = icmp eq i32 %1, 0
  br i1 %7, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %entry, %.preheader
  %8 = phi i32 [ %14, %.preheader ], [ 0, %entry ]
  %9 = phi i32 [ %10, %.preheader ], [ 0, %entry ]
  %10 = add nuw i32 %9, 1
  %11 = tail call swiftcc i64 @"$s4main12bottomUpTreeyAA0D0Os6UInt32VF"(i32 %0)
  %12 = tail call swiftcc i32 @"$s4main9itemCheckys6UInt32VAA4TreeOF"(i64 %11)
  %13 = inttoptr i64 %11 to ptr
  tail call void @swift_release(ptr %13) #10
  %14 = add i32 %12, %8
  %15 = icmp eq i32 %10, %1
  br i1 %15, label %.loopexit, label %.preheader

.loopexit:                                        ; preds = %.preheader, %entry
  %16 = phi i32 [ 0, %entry ], [ %14, %.preheader ]
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %2)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3)
  store i64 0, ptr %3, align 8
  %._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %3, i64 0, i32 1
  store ptr inttoptr (i64 -2305843009213693952 to ptr), ptr %._object._object, align 8
  call swiftcc void @"$ss11_StringGutsV4growyySiF"(i64 32, ptr nocapture nonnull swiftself dereferenceable(16) %3)
  %17 = load ptr, ptr %._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %3)
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4)
  store i32 %1, ptr %4, align 4
  %18 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %4, ptr nonnull @"$ss6UInt32VN", ptr nonnull @"$ss6UInt32Vs23CustomStringConvertiblesWP")
  tail call void @swift_bridgeObjectRelease(ptr %17) #10
  %19 = extractvalue { i64, ptr } %18, 0
  %20 = extractvalue { i64, ptr } %18, 1
  store i64 %19, ptr %2, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %2, i64 0, i32 1
  store ptr %20, ptr %._guts._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4)
  call swiftcc void @"$sSS6appendyySSF"(i64 -3458764513820540911, ptr nonnull inttoptr (i64 or (i64 sub (i64 ptrtoint (ptr @".str.17.\09 trees of depth " to i64), i64 32), i64 -9223372036854775808) to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %2)
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %5)
  store i32 %0, ptr %5, align 4
  %21 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %5, ptr nonnull @"$ss6UInt32VN", ptr nonnull @"$ss6UInt32Vs23CustomStringConvertiblesWP")
  %22 = extractvalue { i64, ptr } %21, 0
  %23 = extractvalue { i64, ptr } %21, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %22, ptr %23, ptr nocapture nonnull swiftself dereferenceable(16) %2)
  tail call void @swift_bridgeObjectRelease(ptr %23) #10
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %5)
  call swiftcc void @"$sSS6appendyySSF"(i64 4209567563902033929, ptr nonnull inttoptr (i64 -1657324662872342496 to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %2)
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %6)
  store i32 %16, ptr %6, align 4
  %24 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %6, ptr nonnull @"$ss6UInt32VN", ptr nonnull @"$ss6UInt32Vs23CustomStringConvertiblesWP")
  %25 = extractvalue { i64, ptr } %24, 0
  %26 = extractvalue { i64, ptr } %24, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %25, ptr %26, ptr nocapture nonnull swiftself dereferenceable(16) %2)
  tail call void @swift_bridgeObjectRelease(ptr %26) #10
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %6)
  %27 = load i64, ptr %2, align 8
  %28 = load ptr, ptr %._guts._object._object, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %2)
  %29 = insertvalue { i64, ptr } undef, i64 %27, 0
  %30 = insertvalue { i64, ptr } %29, ptr %28, 1
  ret { i64, ptr } %30
}

; Function Attrs: nounwind
define internal swiftcc void @"$s4mainyyYbcfU2_yyYbcfU0_"() #3 {
entry:
  %0 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  tail call void @dispatch_group_leave(ptr %0)
  ret void
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc i64 @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFSS_s6UInt32VTg5"(i64 %0, ptr %1, i64 %2) local_unnamed_addr #5 {
entry:
  %3 = alloca %TSS, align 8
  %4 = alloca <{ %Ts6UInt64V, %Ts6UInt64V }>, align 8
  %swifterror = alloca swifterror ptr, align 8
  store ptr null, ptr %swifterror, align 8
  %temp-coercion.coerced3.sroa.4.sroa.0 = alloca i8, align 4
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3)
  store i64 %0, ptr %3, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %3, i64 0, i32 1
  store ptr %1, ptr %._guts._object._object, align 8
  %5 = tail call ptr @swift_bridgeObjectRetain(ptr returned %1) #10
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
  tail call void @swift_bridgeObjectRelease(ptr %8) #10
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
  %29 = call swiftcc i64 @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFADSRys5UInt8VGXEfU_SS_s6UInt32VTg5"(i64 %28, i64 %26, i64 %2, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %swifterror)
  %30 = load ptr, ptr %swifterror, align 8
  %.not = icmp eq ptr %30, null
  call void @llvm.assume(i1 %.not)
  call void @swift_bridgeObjectRelease(ptr %17) #10
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
  %42 = call swiftcc i64 @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFADSRys5UInt8VGXEfU_SS_s6UInt32VTg5"(i64 %40, i64 %41, i64 %2, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %swifterror)
  %43 = load ptr, ptr %swifterror, align 8
  %.not5 = icmp eq ptr %43, null
  tail call void @llvm.assume(i1 %.not5)
  tail call void @swift_bridgeObjectRelease(ptr %17) #10
  br label %44

44:                                               ; preds = %39, %24
  %.in = phi i64 [ %29, %24 ], [ %42, %39 ]
  %.in6 = and i64 %.in, 4294967296
  %45 = icmp ne i64 %.in6, 0
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %temp-coercion.coerced3.sroa.4.sroa.0)
  store i1 %45, ptr %temp-coercion.coerced3.sroa.4.sroa.0, align 4
  %temp-coercion.coerced3.sroa.4.sroa.0.0.temp-coercion.coerced3.sroa.4.sroa.0.0.temp-coercion.coerced3.sroa.4.sroa.0.0.temp-coercion.coerced3.sroa.4.sroa.0.0.temp-coercion.coerced3.sroa.4.0. = load i8, ptr %temp-coercion.coerced3.sroa.4.sroa.0, align 4
  %temp-coercion.coerced3.sroa.4.0.insert.ext = zext i8 %temp-coercion.coerced3.sroa.4.sroa.0.0.temp-coercion.coerced3.sroa.4.sroa.0.0.temp-coercion.coerced3.sroa.4.sroa.0.0.temp-coercion.coerced3.sroa.4.sroa.0.0.temp-coercion.coerced3.sroa.4.0. to i64
  %temp-coercion.coerced3.sroa.4.0.insert.shift = shl nuw nsw i64 %temp-coercion.coerced3.sroa.4.0.insert.ext, 32
  %temp-coercion.coerced3.sroa.0.0.insert.ext = and i64 %.in, 4294967295
  %temp-coercion.coerced3.sroa.0.0.insert.insert = or i64 %temp-coercion.coerced3.sroa.4.0.insert.shift, %temp-coercion.coerced3.sroa.0.0.insert.ext
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %temp-coercion.coerced3.sroa.4.sroa.0)
  ret i64 %temp-coercion.coerced3.sroa.0.0.insert.insert
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4findys10_HashTableV6BucketV6bucket_Sb5foundtxSHRzlFs6UInt32V_Tg5"(i32 %0, ptr swiftself %1) local_unnamed_addr #5 {
entry:
  %2 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %1, i64 0, i32 7
  %3 = load i64, ptr %2, align 8
  %4 = zext i32 %0 to i64
  %5 = tail call swiftcc i64 @"$ss6HasherV5_hash4seed5bytes5countS2i_s6UInt64VSitFZ"(i64 %3, i64 %4, i64 4)
  %6 = tail call swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4find_9hashValues10_HashTableV6BucketV6bucket_Sb5foundtx_SitSHRzlFs6UInt32V_Tg5"(i32 %0, i64 %5, ptr swiftself %1)
  ret { i64, i1 } %6
}

define linkonce_odr hidden swiftcc void @"$sSMsSkRzrlE4sort2byySb7ElementSTQz_ADtKXE_tKFs15ContiguousArrayVys6UInt32V3key_SS5valuetG_Tg5011$s4mainSbs6F32V3key_SS5valuet_AcD_SSAEttXEfU3_Tf1cn_n"(ptr nocapture dereferenceable(8) %0, ptr swiftself %1, ptr noalias nocapture swifterror dereferenceable(8) %2) local_unnamed_addr #0 {
entry:
  %3 = alloca %TSrys6UInt32V3key_SS5valuetG, align 8
  %4 = alloca %TSi, align 8
  %5 = alloca %TSrys6UInt32V3key_SS5valuetG, align 8
  %6 = load ptr, ptr %0, align 8
  %7 = tail call zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr %6) #29
  br i1 %7, label %10, label %8

8:                                                ; preds = %entry
  %9 = tail call swiftcc ptr @"$ss22_ContiguousArrayBufferV20_consumeAndCreateNewAByxGyFs6UInt32V3key_SS5valuet_Tg5"(ptr %6)
  br label %10

10:                                               ; preds = %entry, %8
  %11 = phi ptr [ %9, %8 ], [ %6, %entry ]
  %12 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %11, i64 0, i32 1
  %13 = load i64, ptr %12, align 8, !range !38
  %14 = getelementptr inbounds i8, ptr %11, i64 32
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %3)
  %15 = ptrtoint ptr %14 to i64
  store i64 %15, ptr %3, align 8
  %.count = getelementptr inbounds %TSrys6UInt32V3key_SS5valuetG, ptr %3, i64 0, i32 1
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
  %22 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss6UInt32V3key_SS5valuetMD") #28
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
  %.count21 = getelementptr inbounds %TSrys6UInt32V3key_SS5valuetG, ptr %5, i64 0, i32 1
  store i64 %19, ptr %.count21, align 8
  call swiftcc void @"$sSr15_stableSortImpl2byySbx_xtKXE_tKFySryxGz_SiztKXEfU_s6UInt32V3key_SS5valuet_Tg5011$s4mainSbs6E32V3key_SS5valuet_AcD_SSAEttXEfU3_Tf1nnncn_n"(ptr nocapture nonnull dereferenceable(16) %5, ptr nocapture nonnull dereferenceable(8) %4, ptr nocapture nonnull dereferenceable(16) %3, i64 %16, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %2)
  %29 = load ptr, ptr %2, align 8
  %.not41 = icmp eq ptr %29, null
  %30 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %26, i64 0, i32 1
  store i64 0, ptr %30, align 8
  tail call void @swift_bridgeObjectRelease(ptr %26) #10
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
  %.phi.trans.insert = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %14, i64 %32
  %.pre = load i32, ptr %.phi.trans.insert, align 8
  br label %33

33:                                               ; preds = %39, %.preheader
  %34 = phi i64 [ %32, %.preheader ], [ %35, %39 ]
  %35 = add nsw i64 %34, -1
  %36 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %14, i64 %35
  %37 = load i32, ptr %36, align 8
  %38 = icmp ult i32 %.pre, %37
  br i1 %38, label %39, label %44

39:                                               ; preds = %33
  %40 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %14, i64 %34
  %.value16 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %14, i64 %34, i32 2
  %.value16._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.value16, i64 0, i32 1
  %41 = load i64, ptr %.value16, align 8
  %42 = load ptr, ptr %.value16._guts._object._object, align 8
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %40, ptr noundef nonnull align 8 dereferenceable(24) %36, i64 24, i1 false)
  store i32 %.pre, ptr %36, align 8
  %.value19 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %14, i64 %35, i32 2
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

define linkonce_odr hidden swiftcc void @"$sSr15_stableSortImpl2byySbx_xtKXE_tKFySryxGz_SiztKXEfU_s6UInt32V3key_SS5valuet_Tg5011$s4mainSbs6E32V3key_SS5valuet_AcD_SSAEttXEfU3_Tf1nnncn_n"(ptr nocapture dereferenceable(16) %0, ptr nocapture dereferenceable(8) %1, ptr nocapture dereferenceable(16) %2, i64 %3, ptr swiftself %4, ptr noalias nocapture swifterror dereferenceable(8) %5) local_unnamed_addr #0 {
entry:
  %6 = alloca %TSa, align 8
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %6)
  store ptr @_swiftEmptyArrayStorage, ptr %6, align 8
  %7 = load i64, ptr %2, align 8
  %.count = getelementptr inbounds %TSrys6UInt32V3key_SS5valuetG, ptr %2, i64 0, i32 1
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
  %17 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %16, i64 %13
  %18 = load i32, ptr %17, align 8
  %19 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %16, i64 %10
  %20 = load i32, ptr %19, align 8
  %21 = icmp ult i32 %18, %20
  %22 = add nsw i64 %10, 2
  %23 = icmp slt i64 %22, %12
  br i1 %23, label %.preheader79, label %.loopexit80

.preheader79:                                     ; preds = %15, %30
  %24 = phi i32 [ %27, %30 ], [ %18, %15 ]
  %25 = phi i64 [ %31, %30 ], [ %22, %15 ]
  %26 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %16, i64 %25
  %27 = load i32, ptr %26, align 8
  %28 = icmp uge i32 %27, %24
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
  %41 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %40, i64 %35
  %42 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %40, i64 %36
  %43 = load i32, ptr %41, align 8
  %.value31 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %40, i64 %35, i32 2
  %44 = load i64, ptr %.value31, align 8
  %.value31._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.value31, i64 0, i32 1
  %45 = load ptr, ptr %.value31._guts._object._object, align 8
  tail call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %41, ptr noundef nonnull align 8 dereferenceable(24) %42, i64 24, i1 false)
  store i32 %43, ptr %42, align 8
  %.value33 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %40, i64 %36, i32 2
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
  %60 = tail call zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr %59) #29
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
  %83 = call swiftcc i1 @"$sSr13_mergeTopRuns_6buffer2bySbSaySnySiGGz_SpyxGSbx_xtKXEtKFs6UInt32V3key_SS5valuet_Tg5011$s4mainSbs6F32V3key_SS5valuet_AcD_SSAEttXEfU3_Tf1nncn_n"(ptr nocapture nonnull dereferenceable(8) %6, ptr %82, ptr nocapture nonnull dereferenceable(16) %2, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %5)
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
  tail call void @swift_bridgeObjectRelease(ptr %90) #10
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
  %98 = tail call zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr nonnull %91) #29
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
  %115 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %104, i64 %111
  %116 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %104, i64 %113
  %117 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %104, i64 %114
  %118 = tail call swiftcc i1 @"$ss6_merge3low3mid4high6buffer2bySbSpyxG_A3GSbx_xtKXEtKlFs6UInt32V3key_SS5valuet_Tg5011$s4mainSbs6G32V3key_SS5valuet_AcD_SSAEttXEfU3_Tf1nnnnc_n"(ptr %115, ptr %116, ptr %117, ptr %103, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %5)
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
  tail call void @swift_bridgeObjectRelease(ptr nonnull %102) #10
  br label %128

common.ret:                                       ; preds = %128, %.loopexit104
  ret void

.loopexit104:                                     ; preds = %120, %.loopexit82
  %.sink = phi ptr [ %91, %.loopexit82 ], [ %102, %120 ]
  tail call void @swift_bridgeObjectRelease(ptr nonnull %.sink) #10
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
  %136 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %135, i64 %132
  %137 = load i32, ptr %136, align 8
  %138 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %135, i64 %133
  %139 = load i32, ptr %138, align 8
  %140 = icmp ult i32 %137, %139
  br i1 %140, label %141, label %145

141:                                              ; preds = %131
  %.value = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %135, i64 %132, i32 2
  %.value._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.value, i64 0, i32 1
  %142 = load i64, ptr %.value, align 8
  %143 = load ptr, ptr %.value._guts._object._object, align 8
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %136, ptr noundef nonnull align 8 dereferenceable(24) %138, i64 24, i1 false)
  store i32 %137, ptr %138, align 8
  %.value26 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %135, i64 %133, i32 2
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

define linkonce_odr hidden swiftcc i1 @"$sSr13_mergeTopRuns_6buffer2bySbSaySnySiGGz_SpyxGSbx_xtKXEtKFs6UInt32V3key_SS5valuet_Tg5011$s4mainSbs6F32V3key_SS5valuet_AcD_SSAEttXEfU3_Tf1nncn_n"(ptr nocapture dereferenceable(8) %0, ptr %1, ptr nocapture dereferenceable(16) %2, ptr swiftself %3, ptr noalias nocapture swifterror dereferenceable(8) %4) local_unnamed_addr #0 {
entry:
  %5 = load ptr, ptr %0, align 8
  %6 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %5, i64 0, i32 1
  %7 = load i64, ptr %6, align 8, !range !38
  %8 = icmp ugt i64 %7, 1
  br i1 %8, label %9, label %common.ret

9:                                                ; preds = %entry
  %10 = tail call zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr nonnull %5) #29
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
  %70 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %69, i64 %64
  %71 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %69, i64 %66
  %72 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %69, i64 %67
  %73 = tail call swiftcc i1 @"$ss6_merge3low3mid4high6buffer2bySbSpyxG_A3GSbx_xtKXEtKlFs6UInt32V3key_SS5valuet_Tg5011$s4mainSbs6G32V3key_SS5valuet_AcD_SSAEttXEfU3_Tf1nnnnc_n"(ptr %70, ptr %71, ptr %72, ptr %1, ptr swiftself undef, ptr noalias nocapture nonnull swifterror dereferenceable(8) %4)
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

define linkonce_odr hidden swiftcc i1 @"$ss6_merge3low3mid4high6buffer2bySbSpyxG_A3GSbx_xtKXEtKlFs6UInt32V3key_SS5valuet_Tg5011$s4mainSbs6G32V3key_SS5valuet_AcD_SSAEttXEfU3_Tf1nnnnc_n"(ptr %0, ptr %1, ptr %2, ptr %3, ptr swiftself %4, ptr noalias nocapture swifterror dereferenceable(8) %5) local_unnamed_addr #0 {
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
  %15 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %0, i64 %9
  %16 = icmp ugt ptr %15, %3
  %17 = icmp eq ptr %3, %0
  %or.cond67 = select i1 %17, i1 %16, i1 false
  br i1 %or.cond67, label %23, label %.sink.split

18:                                               ; preds = %entry
  %19 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %1, i64 %12
  %20 = icmp ugt ptr %19, %3
  %21 = icmp eq ptr %3, %1
  %or.cond70 = select i1 %21, i1 %20, i1 false
  br i1 %or.cond70, label %58, label %.sink.split58

.sink.split:                                      ; preds = %14
  %22 = mul nuw nsw i64 %9, 24
  tail call void @llvm.memmove.p0.p0.i64(ptr align 8 %3, ptr align 8 %0, i64 %22, i1 false)
  br label %23

23:                                               ; preds = %14, %.sink.split
  %24 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %3, i64 %9
  %25 = icmp sgt i64 %8, 23
  %26 = icmp ult ptr %1, %2
  %or.cond5 = select i1 %25, i1 %26, i1 false
  br i1 %or.cond5, label %.preheader, label %.loopexit

.preheader:                                       ; preds = %23, %39
  %27 = phi ptr [ %40, %39 ], [ %1, %23 ]
  %28 = phi ptr [ %41, %39 ], [ %3, %23 ]
  %29 = phi ptr [ %42, %39 ], [ %0, %23 ]
  %30 = load i32, ptr %27, align 8
  %31 = load i32, ptr %28, align 8
  %32 = icmp ult i32 %30, %31
  br i1 %32, label %33, label %36

33:                                               ; preds = %.preheader
  %34 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %27, i64 1
  %35 = icmp eq ptr %29, %27
  br i1 %35, label %39, label %.sink.split54

36:                                               ; preds = %.preheader
  %37 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %28, i64 1
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
  %42 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %29, i64 1
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
  %52 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %46, i64 %51
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
  %59 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %3, i64 %12
  %60 = icmp sgt i64 %11, 23
  %61 = icmp ult ptr %0, %1
  %or.cond15 = select i1 %60, i1 %61, i1 false
  br i1 %or.cond15, label %.preheader41, label %.loopexit

.preheader41:                                     ; preds = %58, %77
  %62 = phi ptr [ %78, %77 ], [ %1, %58 ]
  %63 = phi ptr [ %65, %77 ], [ %2, %58 ]
  %64 = phi ptr [ %79, %77 ], [ %59, %58 ]
  %65 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %63, i64 -1
  %66 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %64, i64 -1
  %67 = load i32, ptr %66, align 8
  %68 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %62, i64 -1
  %69 = load i32, ptr %68, align 8
  %70 = icmp ult i32 %67, %69
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
define linkonce_odr hidden swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNewAByxGyFSnySiG_Tgq5"(ptr %0) local_unnamed_addr #5 {
entry:
  %1 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8, !range !38
  %3 = tail call swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSnySiG_Tgq5"(i1 false, i64 %2, i1 false, ptr %0)
  ret ptr %3
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSnySiG_Tg5"(i1 %0, i64 %1, i1 %2, ptr %3) local_unnamed_addr #6 {
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
  %18 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCySnySiGGMD") #28
  %19 = shl i64 %15, 4
  %20 = add i64 %19, 32
  %21 = tail call noalias ptr @swift_allocObject(ptr %18, i64 %20, i64 7) #10
  %call.i = tail call i64 @malloc_size(ptr noundef %21) #31, !clang.arc.no_objc_arc_exceptions !33
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
  tail call void @swift_bridgeObjectRelease(ptr nonnull %3) #10
  ret ptr %26
}

define linkonce_odr hidden swiftcc ptr @"$ss22_ContiguousArrayBufferV19_uninitializedCount15minimumCapacityAByxGSi_SitcfCs6UInt32V3key_SS5valuet_Tgm5"(i64 %0, i64 %1) local_unnamed_addr #7 {
entry:
  %2 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %3 = icmp eq i64 %2, 0
  br i1 %3, label %12, label %4

4:                                                ; preds = %entry
  %5 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCys6UInt32V3key_SS5valuetGMD") #28
  %6 = mul i64 %2, 24
  %7 = add i64 %6, 32
  %8 = tail call noalias ptr @swift_allocObject(ptr %5, i64 %7, i64 7) #10
  %call.i = tail call i64 @malloc_size(ptr noundef %8) #31, !clang.arc.no_objc_arc_exceptions !33
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
define linkonce_odr hidden swiftcc ptr @"$ss12_ArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFSnySiG_Tgq5"(i1 %0, i64 %1, i1 %2, ptr %3) local_unnamed_addr #6 {
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
  %18 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCySnySiGGMD") #28
  %19 = shl i64 %15, 4
  %20 = add i64 %19, 32
  %21 = tail call noalias ptr @swift_allocObject(ptr %18, i64 %20, i64 7) #10
  %call.i = tail call i64 @malloc_size(ptr noundef %21) #31, !clang.arc.no_objc_arc_exceptions !33
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
  tail call void @swift_bridgeObjectRelease(ptr nonnull %3) #10
  ret ptr %26
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4find_9hashValues10_HashTableV6BucketV6bucket_Sb5foundtx_SitSHRzlFs6UInt32V_Tg5"(i32 %0, i64 %1, ptr swiftself %2) local_unnamed_addr #5 {
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
  %20 = getelementptr inbounds %Ts6UInt32V, ptr %19, i64 %9
  %21 = load i32, ptr %20, align 4
  %22 = icmp eq i32 %21, %0
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
  %38 = getelementptr inbounds %Ts6UInt32V, ptr %19, i64 %37
  %39 = load i32, ptr %38, align 4
  %40 = icmp eq i32 %39, %0
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

define linkonce_odr hidden swiftcc i64 @"$ss13_parseInteger5ascii5radixq_Sgx_SitSyRzs010FixedWidthB0R_r0_lFADSRys5UInt8VGXEfU_SS_s6UInt32VTg5"(i64 %0, i64 %1, i64 %2, ptr swiftself %3, ptr noalias nocapture swifterror dereferenceable(8) %4) local_unnamed_addr #0 {
entry:
  %temp-coercion.coerced.sroa.4.sroa.0 = alloca i8, align 4
  %5 = inttoptr i64 %0 to ptr
  %6 = load i8, ptr %5, align 1
  switch i8 %6, label %94 [
    i8 45, label %7
    i8 43, label %53
  ]

7:                                                ; preds = %entry
  %8 = icmp eq i64 %1, 1
  br i1 %8, label %.loopexit, label %9, !prof !35

9:                                                ; preds = %7
  %10 = icmp eq i64 %0, 0
  %11 = icmp sgt i64 %2, 10
  %12 = trunc i64 %2 to i8
  %13 = add i8 %12, 48
  %14 = add i8 %12, 55
  %15 = add i8 %12, 87
  %16 = select i1 %11, i8 %15, i8 97
  %17 = select i1 %11, i8 %14, i8 65
  %18 = select i1 %11, i8 58, i8 %13
  br i1 %10, label %.loopexit, label %19

19:                                               ; preds = %9
  %20 = getelementptr inbounds %Ts5UInt8V, ptr %5, i64 1
  %21 = getelementptr inbounds %Ts5UInt8V, ptr %5, i64 %1
  %22 = trunc i64 %2 to i32
  br label %23

23:                                               ; preds = %44, %19
  %24 = phi ptr [ %20, %19 ], [ %46, %44 ]
  %25 = phi i32 [ 0, %19 ], [ %45, %44 ]
  %26 = load i8, ptr %24, align 1
  %27 = icmp ugt i8 %26, 47
  %28 = icmp ult i8 %26, %18
  %or.cond = select i1 %27, i1 %28, i1 false
  br i1 %or.cond, label %35, label %29, !prof !37

29:                                               ; preds = %23
  %30 = icmp ugt i8 %26, 64
  %31 = icmp ult i8 %26, %17
  %or.cond4 = select i1 %30, i1 %31, i1 false
  br i1 %or.cond4, label %35, label %32, !prof !37

32:                                               ; preds = %29
  %33 = icmp ugt i8 %26, 96
  %34 = icmp ult i8 %26, %16
  %or.cond5 = select i1 %33, i1 %34, i1 false
  br i1 %or.cond5, label %35, label %.loopexit, !prof !37

35:                                               ; preds = %32, %23, %29
  %.sink = phi i8 [ -55, %29 ], [ -48, %23 ], [ -87, %32 ]
  %36 = tail call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %25, i32 %22)
  %37 = extractvalue { i32, i1 } %36, 1
  br i1 %37, label %.loopexit, label %38

38:                                               ; preds = %35
  %39 = add i8 %26, %.sink
  %40 = extractvalue { i32, i1 } %36, 0
  %41 = zext i8 %39 to i32
  %42 = tail call { i32, i1 } @llvm.usub.with.overflow.i32(i32 %40, i32 %41)
  %43 = extractvalue { i32, i1 } %42, 1
  br i1 %43, label %.loopexit, label %44, !prof !35

44:                                               ; preds = %38
  %45 = extractvalue { i32, i1 } %42, 0
  %46 = getelementptr inbounds %Ts5UInt8V, ptr %24, i64 1
  %47 = icmp eq ptr %46, %21
  br i1 %47, label %.loopexit.loopexit118.split.loop.exit, label %23

.loopexit.loopexit.split.loop.exit:               ; preds = %130
  %48 = extractvalue { i32, i1 } %128, 0
  br label %.loopexit

.loopexit.loopexit118.split.loop.exit:            ; preds = %44
  %49 = extractvalue { i32, i1 } %42, 0
  br label %.loopexit

.loopexit.loopexit121.split.loop.exit:            ; preds = %90
  %50 = extractvalue { i32, i1 } %88, 0
  br label %.loopexit

.loopexit:                                        ; preds = %.loopexit.loopexit121.split.loop.exit, %78, %81, %84, %.loopexit.loopexit118.split.loop.exit, %32, %35, %38, %.loopexit.loopexit.split.loop.exit, %118, %121, %124, %96, %94, %55, %53, %9, %7
  %51 = phi i32 [ 0, %7 ], [ 0, %9 ], [ 0, %53 ], [ 0, %55 ], [ 0, %94 ], [ 0, %96 ], [ %48, %.loopexit.loopexit.split.loop.exit ], [ 0, %118 ], [ 0, %121 ], [ 0, %124 ], [ %49, %.loopexit.loopexit118.split.loop.exit ], [ 0, %32 ], [ 0, %35 ], [ 0, %38 ], [ %50, %.loopexit.loopexit121.split.loop.exit ], [ 0, %78 ], [ 0, %81 ], [ 0, %84 ]
  %52 = phi i1 [ true, %7 ], [ false, %9 ], [ true, %53 ], [ false, %55 ], [ true, %94 ], [ false, %96 ], [ false, %.loopexit.loopexit.split.loop.exit ], [ true, %118 ], [ true, %121 ], [ true, %124 ], [ false, %.loopexit.loopexit118.split.loop.exit ], [ true, %32 ], [ true, %35 ], [ true, %38 ], [ false, %.loopexit.loopexit121.split.loop.exit ], [ true, %78 ], [ true, %81 ], [ true, %84 ]
  call void @llvm.lifetime.start.p0(i64 1, ptr nonnull %temp-coercion.coerced.sroa.4.sroa.0)
  store i1 %52, ptr %temp-coercion.coerced.sroa.4.sroa.0, align 4
  %temp-coercion.coerced.sroa.4.sroa.0.0.temp-coercion.coerced.sroa.4.sroa.0.0.temp-coercion.coerced.sroa.4.sroa.0.0.temp-coercion.coerced.sroa.4.sroa.0.0.temp-coercion.coerced.sroa.4.0. = load i8, ptr %temp-coercion.coerced.sroa.4.sroa.0, align 4
  %temp-coercion.coerced.sroa.4.0.insert.ext = zext i8 %temp-coercion.coerced.sroa.4.sroa.0.0.temp-coercion.coerced.sroa.4.sroa.0.0.temp-coercion.coerced.sroa.4.sroa.0.0.temp-coercion.coerced.sroa.4.sroa.0.0.temp-coercion.coerced.sroa.4.0. to i64
  %temp-coercion.coerced.sroa.4.0.insert.shift = shl nuw nsw i64 %temp-coercion.coerced.sroa.4.0.insert.ext, 32
  %temp-coercion.coerced.sroa.0.0.insert.ext = zext i32 %51 to i64
  %temp-coercion.coerced.sroa.0.0.insert.insert = or i64 %temp-coercion.coerced.sroa.4.0.insert.shift, %temp-coercion.coerced.sroa.0.0.insert.ext
  call void @llvm.lifetime.end.p0(i64 1, ptr nonnull %temp-coercion.coerced.sroa.4.sroa.0)
  ret i64 %temp-coercion.coerced.sroa.0.0.insert.insert

53:                                               ; preds = %entry
  %54 = icmp eq i64 %1, 1
  br i1 %54, label %.loopexit, label %55, !prof !35

55:                                               ; preds = %53
  %56 = icmp eq i64 %0, 0
  %57 = icmp sgt i64 %2, 10
  %58 = trunc i64 %2 to i8
  %59 = add i8 %58, 48
  %60 = add i8 %58, 55
  %61 = add i8 %58, 87
  %62 = select i1 %57, i8 %61, i8 97
  %63 = select i1 %57, i8 %60, i8 65
  %64 = select i1 %57, i8 58, i8 %59
  br i1 %56, label %.loopexit, label %65

65:                                               ; preds = %55
  %66 = getelementptr inbounds %Ts5UInt8V, ptr %5, i64 1
  %67 = getelementptr inbounds %Ts5UInt8V, ptr %5, i64 %1
  %68 = trunc i64 %2 to i32
  br label %69

69:                                               ; preds = %90, %65
  %70 = phi ptr [ %66, %65 ], [ %92, %90 ]
  %71 = phi i32 [ 0, %65 ], [ %91, %90 ]
  %72 = load i8, ptr %70, align 1
  %73 = icmp ugt i8 %72, 47
  %74 = icmp ult i8 %72, %64
  %or.cond6 = select i1 %73, i1 %74, i1 false
  br i1 %or.cond6, label %81, label %75, !prof !37

75:                                               ; preds = %69
  %76 = icmp ugt i8 %72, 64
  %77 = icmp ult i8 %72, %63
  %or.cond7 = select i1 %76, i1 %77, i1 false
  br i1 %or.cond7, label %81, label %78, !prof !37

78:                                               ; preds = %75
  %79 = icmp ugt i8 %72, 96
  %80 = icmp ult i8 %72, %62
  %or.cond8 = select i1 %79, i1 %80, i1 false
  br i1 %or.cond8, label %81, label %.loopexit, !prof !37

81:                                               ; preds = %78, %69, %75
  %.sink111 = phi i8 [ -55, %75 ], [ -48, %69 ], [ -87, %78 ]
  %82 = tail call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %71, i32 %68)
  %83 = extractvalue { i32, i1 } %82, 1
  br i1 %83, label %.loopexit, label %84

84:                                               ; preds = %81
  %85 = add i8 %72, %.sink111
  %86 = extractvalue { i32, i1 } %82, 0
  %87 = zext i8 %85 to i32
  %88 = tail call { i32, i1 } @llvm.uadd.with.overflow.i32(i32 %86, i32 %87)
  %89 = extractvalue { i32, i1 } %88, 1
  br i1 %89, label %.loopexit, label %90, !prof !35

90:                                               ; preds = %84
  %91 = extractvalue { i32, i1 } %88, 0
  %92 = getelementptr inbounds %Ts5UInt8V, ptr %70, i64 1
  %93 = icmp eq ptr %92, %67
  br i1 %93, label %.loopexit.loopexit121.split.loop.exit, label %69

94:                                               ; preds = %entry
  %95 = icmp eq i64 %1, 0
  br i1 %95, label %.loopexit, label %96, !prof !35

96:                                               ; preds = %94
  %97 = icmp sgt i64 %2, 10
  %98 = trunc i64 %2 to i8
  %99 = add i8 %98, 48
  %100 = add i8 %98, 55
  %101 = add i8 %98, 87
  %102 = select i1 %97, i8 %101, i8 97
  %103 = select i1 %97, i8 %100, i8 65
  %104 = select i1 %97, i8 58, i8 %99
  %105 = icmp eq i64 %0, 0
  br i1 %105, label %.loopexit, label %106

106:                                              ; preds = %96
  %107 = getelementptr inbounds %Ts5UInt8V, ptr %5, i64 %1
  %108 = trunc i64 %2 to i32
  br label %109

109:                                              ; preds = %130, %106
  %110 = phi ptr [ %5, %106 ], [ %132, %130 ]
  %111 = phi i32 [ 0, %106 ], [ %131, %130 ]
  %112 = load i8, ptr %110, align 1
  %113 = icmp ugt i8 %112, 47
  %114 = icmp ult i8 %112, %104
  %or.cond9 = select i1 %113, i1 %114, i1 false
  br i1 %or.cond9, label %121, label %115, !prof !37

115:                                              ; preds = %109
  %116 = icmp ugt i8 %112, 64
  %117 = icmp ult i8 %112, %103
  %or.cond10 = select i1 %116, i1 %117, i1 false
  br i1 %or.cond10, label %121, label %118, !prof !37

118:                                              ; preds = %115
  %119 = icmp ugt i8 %112, 96
  %120 = icmp ult i8 %112, %102
  %or.cond11 = select i1 %119, i1 %120, i1 false
  br i1 %or.cond11, label %121, label %.loopexit, !prof !37

121:                                              ; preds = %118, %109, %115
  %.sink112 = phi i8 [ -55, %115 ], [ -48, %109 ], [ -87, %118 ]
  %122 = tail call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %111, i32 %108)
  %123 = extractvalue { i32, i1 } %122, 1
  br i1 %123, label %.loopexit, label %124

124:                                              ; preds = %121
  %125 = add i8 %112, %.sink112
  %126 = extractvalue { i32, i1 } %122, 0
  %127 = zext i8 %125 to i32
  %128 = tail call { i32, i1 } @llvm.uadd.with.overflow.i32(i32 %126, i32 %127)
  %129 = extractvalue { i32, i1 } %128, 1
  br i1 %129, label %.loopexit, label %130, !prof !35

130:                                              ; preds = %124
  %131 = extractvalue { i32, i1 } %128, 0
  %132 = getelementptr inbounds %Ts5UInt8V, ptr %110, i64 1
  %133 = icmp eq ptr %132, %107
  br i1 %133, label %.loopexit.loopexit.split.loop.exit, label %109
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc { i64, ptr } @"$sSS8_copyingyS2SFZ"(i64 %0, ptr %1) local_unnamed_addr #5 {
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
  %14 = tail call ptr @swift_bridgeObjectRetain(ptr returned %1) #10
  %15 = tail call swiftcc { i64, i64, i64, ptr } @"$sSSySsSnySS5IndexVGcig"(i64 15, i64 %13, i64 %0, ptr %1)
  tail call void @swift_bridgeObjectRelease(ptr %1) #10
  %16 = extractvalue { i64, i64, i64, ptr } %15, 0
  %17 = extractvalue { i64, i64, i64, ptr } %15, 1
  %18 = extractvalue { i64, i64, i64, ptr } %15, 2
  %19 = extractvalue { i64, i64, i64, ptr } %15, 3
  %20 = tail call swiftcc { i64, ptr } @"$sSS8_copyingySSSsFZ"(i64 %16, i64 %17, i64 %18, ptr %19)
  tail call void @swift_bridgeObjectRelease(ptr %19) #10
  ret { i64, ptr } %20
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc { i64, ptr } @"$sSS8_copyingySSSsFZ"(i64 %0, i64 %1, i64 %2, ptr %3) local_unnamed_addr #5 {
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
  %18 = tail call ptr @swift_bridgeObjectRetain(ptr returned %3) #10
  tail call void @swift_bridgeObjectRelease(ptr %17) #10
  call void @llvm.lifetime.end.p0(i64 40, ptr nonnull %4)
  br label %._crit_edge

._crit_edge:                                      ; preds = %9, %12
  %19 = phi ptr [ %13, %12 ], [ @_swiftEmptyArrayStorage, %9 ]
  %20 = getelementptr inbounds i8, ptr %19, i64 32
  %21 = ptrtoint ptr %20 to i64
  %22 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %19, i64 0, i32 1
  %23 = load i64, ptr %22, align 8, !range !38
  %24 = tail call swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64 %21, i64 %23), !noalias !39
  tail call void @swift_release(ptr %19) #10
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
  %35 = call ptr @swift_bridgeObjectRetain(ptr returned %3) #10
  %36 = call swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64 %33, i64 %34)
  call void @swift_bridgeObjectRelease(ptr %3) #10
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
  %54 = tail call ptr @swift_bridgeObjectRetain(ptr returned %3) #10
  %55 = tail call swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64 %52, i64 %53)
  tail call void @swift_bridgeObjectRelease(ptr %3) #10
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

define linkonce_odr hidden swiftcc ptr @"$ss22_ContiguousArrayBufferV19_uninitializedCount15minimumCapacityAByxGSi_SitcfCs5UInt8V_Tgmq5"(i64 %0, i64 %1) local_unnamed_addr #7 {
entry:
  %2 = tail call i64 @llvm.smax.i64(i64 %1, i64 %0)
  %3 = icmp eq i64 %2, 0
  br i1 %3, label %10, label %4

4:                                                ; preds = %entry
  %5 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCys5UInt8VGMD") #28
  %6 = add i64 %2, 32
  %7 = tail call noalias ptr @swift_allocObject(ptr %5, i64 %6, i64 7) #10
  %call.i = tail call i64 @malloc_size(ptr noundef %7) #31, !clang.arc.no_objc_arc_exceptions !33
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
define linkonce_odr hidden swiftcc i64 @"$ss11_StringGutsV27_slowEnsureMatchingEncodingySS5IndexVAEF"(i64 %0, i64 %1, ptr %2) local_unnamed_addr #5 {
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

define linkonce_odr hidden swiftcc void @"$sSD8_VariantV8setValue_6forKeyyq_n_xtFs6UInt32V_SSTg5"(i64 %0, ptr %1, i32 %2, ptr nocapture swiftself dereferenceable(8) %3) local_unnamed_addr #0 {
entry:
  %4 = alloca %Ts17_NativeDictionaryV, align 8
  %5 = load ptr, ptr %3, align 8
  %6 = tail call zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr %5) #29
  call void @llvm.lifetime.start.p0(i64 8, ptr nonnull %4)
  store ptr %5, ptr %4, align 8
  store ptr inttoptr (i64 -9223372036854775808 to ptr), ptr %3, align 8
  %7 = call swiftcc { i64, i1 } @"$ss17_NativeDictionaryV12mutatingFind_8isUniques10_HashTableV6BucketV6bucket_Sb5foundtx_SbtFs6UInt32V_SSTg5"(i32 %2, i1 %6, ptr nocapture nonnull swiftself dereferenceable(8) %4)
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
  tail call void @swift_bridgeObjectRelease(ptr %15) #10
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
  %26 = getelementptr inbounds %Ts6UInt32V, ptr %25, i64 %8
  store i32 %2, ptr %26, align 4
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
  tail call void @swift_bridgeObjectRelease(ptr %34) #10
  call void @llvm.lifetime.end.p0(i64 8, ptr nonnull %4)
  ret void
}

define linkonce_odr hidden swiftcc { i64, i1 } @"$ss17_NativeDictionaryV12mutatingFind_8isUniques10_HashTableV6BucketV6bucket_Sb5foundtx_SbtFs6UInt32V_SSTg5"(i32 %0, i1 %1, ptr nocapture swiftself dereferenceable(8) %2) local_unnamed_addr #0 {
entry:
  %3 = load ptr, ptr %2, align 8
  %4 = tail call swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4findys10_HashTableV6BucketV6bucket_Sb5foundtxSHRzlFs6UInt32V_Tg5"(i32 %0, ptr swiftself %3)
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
  br i1 %brmerge, label %12, label %20, !prof !42

12:                                               ; preds = %entry
  %brmerge2 = select i1 %1, i1 true, i1 %11
  br i1 %brmerge2, label %14, label %13

13:                                               ; preds = %12
  tail call swiftcc void @"$ss17_NativeDictionaryV4copyyyFs6UInt32V_SSTg5"(ptr nocapture nonnull swiftself dereferenceable(8) %2)
  br label %20

14:                                               ; preds = %12
  tail call swiftcc void @"$ss17_NativeDictionaryV20_copyOrMoveAndResize8capacity12moveElementsySi_SbtFs6UInt32V_SSTg5"(i64 %8, i1 %1, ptr nocapture nonnull swiftself dereferenceable(8) %2)
  %15 = load ptr, ptr %2, align 8
  %16 = tail call swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4findys10_HashTableV6BucketV6bucket_Sb5foundtxSHRzlFs6UInt32V_Tg5"(i32 %0, ptr swiftself %15)
  %17 = extractvalue { i64, i1 } %16, 1
  %18 = xor i1 %5, %17
  br i1 %18, label %19, label %20

19:                                               ; preds = %14
  tail call swiftcc void @"$ss53KEY_TYPE_OF_DICTIONARY_VIOLATES_HASHABLE_REQUIREMENTSys5NeverOypXpF"(ptr nonnull @"$ss6UInt32VN")
  unreachable

20:                                               ; preds = %entry, %14, %13
  %.pn = phi { i64, i1 } [ %4, %13 ], [ %16, %14 ], [ %4, %entry ]
  %21 = insertvalue { i64, i1 } %.pn, i1 %5, 1
  ret { i64, i1 } %21
}

define linkonce_odr hidden swiftcc void @"$ss17_NativeDictionaryV4copyyyFs6UInt32V_SSTg5"(ptr nocapture swiftself dereferenceable(8) %0) local_unnamed_addr #0 {
entry:
  %1 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss18_DictionaryStorageCys6UInt32VSSGMD") #28
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
  %47 = tail call i64 @llvm.cttz.i64(i64 %40, i1 true), !range !43
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
  %85 = tail call i64 @llvm.cttz.i64(i64 %84, i1 false), !range !43
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
  %95 = getelementptr inbounds %Ts6UInt32V, ptr %94, i64 %91
  %96 = load i32, ptr %95, align 4
  %97 = load ptr, ptr %33, align 8
  %98 = getelementptr inbounds %TSS, ptr %97, i64 %91
  %99 = load i64, ptr %98, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %98, i64 0, i32 1
  %100 = load ptr, ptr %._guts._object._object, align 8
  %101 = load ptr, ptr %34, align 8
  %102 = getelementptr inbounds %Ts6UInt32V, ptr %101, i64 %91
  store i32 %96, ptr %102, align 4
  %103 = load ptr, ptr %35, align 8
  %104 = getelementptr inbounds %TSS, ptr %103, i64 %91
  store i64 %99, ptr %104, align 8
  %._guts10._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %104, i64 0, i32 1
  store ptr %100, ptr %._guts10._object._object, align 8
  %105 = tail call ptr @swift_bridgeObjectRetain(ptr returned %100) #10
  br label %39

.loopexit39:                                      ; preds = %63, %70, %56, %43, %.preheader, %entry
  tail call void @swift_release(ptr nonnull %2) #10
  store ptr %3, ptr %0, align 8
  ret void
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc void @"$ss17_NativeDictionaryV20_copyOrMoveAndResize8capacity12moveElementsySi_SbtFs6UInt32V_SSTg5"(i64 %0, i1 %1, ptr nocapture swiftself dereferenceable(8) %2) local_unnamed_addr #5 {
entry:
  %3 = load ptr, ptr %2, align 8
  %4 = getelementptr inbounds %Ts22__RawDictionaryStorageC, ptr %3, i64 0, i32 2
  %5 = load i64, ptr %4, align 8, !range !38
  %6 = tail call i64 @llvm.smax.i64(i64 %5, i64 %0)
  %7 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss18_DictionaryStorageCys6UInt32VSSGMD") #28
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
  tail call void @swift_release(ptr nonnull %3) #10
  br label %161

35:                                               ; preds = %135, %11
  %36 = phi i64 [ %22, %11 ], [ %88, %135 ]
  %37 = phi i64 [ 0, %11 ], [ %89, %135 ]
  %38 = icmp eq i64 %36, 0
  br i1 %38, label %39, label %42

39:                                               ; preds = %35
  %40 = add i64 %37, 1
  %41 = icmp slt i64 %40, %31
  br i1 %41, label %48, label %.loopexit49

42:                                               ; preds = %35
  %43 = tail call i64 @llvm.cttz.i64(i64 %36, i1 true), !range !43
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
  %81 = tail call i64 @llvm.cttz.i64(i64 %80, i1 false), !range !43
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
  %91 = getelementptr inbounds %Ts6UInt32V, ptr %90, i64 %87
  %92 = load i32, ptr %91, align 4
  %93 = load ptr, ptr %24, align 8
  %94 = getelementptr inbounds %TSS, ptr %93, i64 %87
  %95 = load i64, ptr %94, align 8
  %._guts21._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %94, i64 0, i32 1
  %96 = load ptr, ptr %._guts21._object._object, align 8
  br i1 %1, label %99, label %97

97:                                               ; preds = %86
  %98 = tail call ptr @swift_bridgeObjectRetain(ptr returned %96) #10
  br label %99

99:                                               ; preds = %86, %97
  %100 = load i64, ptr %25, align 8
  %101 = zext i32 %92 to i64
  %102 = tail call swiftcc i64 @"$ss6HasherV5_hash4seed5bytes5countS2i_s6UInt64VSitFZ"(i64 %100, i64 %101, i64 4)
  %103 = load i8, ptr %26, align 8
  %104 = and i8 %103, 63
  %105 = zext i8 %104 to i64
  %notmask = shl nsw i64 -1, %105
  %106 = xor i64 %notmask, -1
  %107 = and i64 %102, %106
  %108 = lshr i64 %107, 6
  %109 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %32, i64 %108
  %110 = load i64, ptr %109, align 8
  %111 = xor i64 %110, -1
  %112 = and i64 %107, 63
  %113 = shl nsw i64 -1, %112
  %114 = and i64 %113, %111
  %115 = icmp eq i64 %114, 0
  br i1 %115, label %116, label %119

116:                                              ; preds = %99
  %117 = sub i64 63, %notmask
  %118 = lshr i64 %117, 6
  br label %123

119:                                              ; preds = %99
  %120 = tail call i64 @llvm.cttz.i64(i64 %114, i1 true), !range !43
  %121 = and i64 %107, -64
  %122 = or i64 %120, %121
  br label %135

123:                                              ; preds = %123, %116
  %124 = phi i64 [ %108, %116 ], [ %.31, %123 ]
  %125 = add i64 %124, 1
  %126 = icmp eq i64 %125, %118
  %.31 = select i1 %126, i64 0, i64 %125
  %127 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %32, i64 %.31
  %128 = load i64, ptr %127, align 8
  %129 = icmp eq i64 %128, -1
  br i1 %129, label %123, label %130

130:                                              ; preds = %123
  %131 = xor i64 %128, -1
  %132 = tail call i64 @llvm.cttz.i64(i64 %131, i1 false), !range !43
  %133 = shl i64 %.31, 6
  %134 = add i64 %133, %132
  br label %135

135:                                              ; preds = %130, %119
  %136 = phi i64 [ %122, %119 ], [ %134, %130 ]
  %137 = lshr i64 %136, 6
  %138 = and i64 %136, 63
  %139 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %32, i64 %137
  %140 = shl nuw i64 1, %138
  %141 = load i64, ptr %139, align 8
  %142 = or i64 %140, %141
  store i64 %142, ptr %139, align 8
  %143 = load ptr, ptr %27, align 8
  %144 = getelementptr inbounds %Ts6UInt32V, ptr %143, i64 %136
  store i32 %92, ptr %144, align 4
  %145 = load ptr, ptr %28, align 8
  %146 = getelementptr inbounds %TSS, ptr %145, i64 %136
  store i64 %95, ptr %146, align 8
  %._guts14._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %146, i64 0, i32 1
  store ptr %96, ptr %._guts14._object._object, align 8
  %147 = load i64, ptr %29, align 8
  %148 = add i64 %147, 1
  store i64 %148, ptr %29, align 8
  br label %35

.loopexit49:                                      ; preds = %66, %59, %52, %39, %.preheader
  br i1 %1, label %149, label %154

149:                                              ; preds = %.loopexit49
  %150 = load i8, ptr %13, align 8
  %151 = and i8 %150, 63
  %152 = zext i8 %151 to i64
  %notmask47.neg = shl nuw i64 1, %152
  %153 = icmp slt i64 %notmask47.neg, 64
  br i1 %153, label %155, label %.loopexit.loopexit

154:                                              ; preds = %.loopexit49
  tail call void @swift_release(ptr %3) #10
  br label %161

155:                                              ; preds = %149
  %156 = and i64 %notmask47.neg, 63
  %157 = shl nsw i64 -1, %156
  store i64 %157, ptr %12, align 8
  br label %.loopexit

.loopexit.loopexit:                               ; preds = %149
  %158 = add nuw i64 %notmask47.neg, 63
  %159 = lshr i64 %158, 3
  %160 = and i64 %159, 2305843009213693944
  tail call void @llvm.memset.p0.i64(ptr nonnull align 8 %12, i8 0, i64 %160, i1 false)
  br label %.loopexit

.loopexit:                                        ; preds = %.loopexit.loopexit, %155
  store i64 0, ptr %9, align 8
  tail call void @swift_release(ptr nonnull %3) #10
  br label %161

161:                                              ; preds = %.loopexit, %154, %34
  store ptr %8, ptr %2, align 8
  ret void
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc ptr @"$ss22_ContiguousArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFs6UInt32V3key_SS5valuet_Tg5"(i1 %0, i64 %1, i1 %2, ptr %3) local_unnamed_addr #6 {
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
  %18 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss23_ContiguousArrayStorageCys6UInt32V3key_SS5valuetGMD") #28
  %19 = mul i64 %15, 24
  %20 = add i64 %19, 32
  %21 = tail call noalias ptr @swift_allocObject(ptr %18, i64 %20, i64 7) #10
  %call.i = tail call i64 @malloc_size(ptr noundef %21) #31, !clang.arc.no_objc_arc_exceptions !33
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
  %30 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %28, i64 %14
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
  %36 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss6UInt32V3key_SS5valuetMD") #28
  tail call void @swift_arrayInitWithCopy(ptr nonnull %27, ptr nonnull %28, i64 %14, ptr %36) #29
  br label %37

37:                                               ; preds = %34, %35
  tail call void @swift_release(ptr nonnull %3) #10
  ret ptr %26
}

; Function Attrs: noinline
define linkonce_odr hidden swiftcc ptr @"$ss22_ContiguousArrayBufferV20_consumeAndCreateNewAByxGyFs6UInt32V3key_SS5valuet_Tg5"(ptr %0) local_unnamed_addr #5 {
entry:
  %1 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8, !range !38
  %3 = tail call swiftcc ptr @"$ss22_ContiguousArrayBufferV20_consumeAndCreateNew14bufferIsUnique15minimumCapacity13growForAppendAByxGSb_SiSbtFs6UInt32V3key_SS5valuet_Tg5"(i1 false, i64 %2, i1 false, ptr %0)
  ret ptr %3
}

define linkonce_odr hidden swiftcc i64 @"$sSTsE21_copySequenceContents12initializing8IteratorQz_SitSry7ElementQzG_tFSDys6UInt32VSSG_Tg5"(ptr noalias nocapture %0, i64 %1, i64 %2, ptr %3) local_unnamed_addr #0 {
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
  %40 = tail call i64 @llvm.cttz.i64(i64 %37, i1 true), !range !43
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
  %54 = getelementptr inbounds %Ts6UInt32V, ptr %53, i64 %49
  %55 = load i32, ptr %54, align 4
  %56 = load ptr, ptr %23, align 8
  %57 = getelementptr inbounds %TSS, ptr %56, i64 %49
  %58 = load i64, ptr %57, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %57, i64 0, i32 1
  %59 = load ptr, ptr %._guts._object._object, align 8
  store i32 %55, ptr %34, align 8
  %.value3 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %34, i64 0, i32 2
  store i64 %58, ptr %.value3, align 8
  %.value3._guts._object._object = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %34, i64 0, i32 2, i32 0, i32 0, i32 1
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
  %70 = tail call i64 @llvm.cttz.i64(i64 %69, i1 false), !range !43
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
  %82 = tail call ptr @swift_bridgeObjectRetain(ptr returned %59) #10
  br label %.loopexit20

83:                                               ; preds = %48
  %84 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %34, i64 1
  %85 = tail call ptr @swift_bridgeObjectRetain(ptr returned %59) #10
  br label %33
}

define linkonce_odr hidden swiftcc ptr @"$sSD17dictionaryLiteralSDyxq_Gx_q_td_tcfCs6UInt32V_SSTgm5Tf4g_n"(ptr %0) local_unnamed_addr #0 {
entry:
  %1 = getelementptr inbounds %Ts28__ContiguousArrayStorageBaseC, ptr %0, i64 0, i32 1
  %2 = load i64, ptr %1, align 8, !range !38
  %3 = icmp eq i64 %2, 0
  br i1 %3, label %.loopexit, label %4

4:                                                ; preds = %entry
  %5 = tail call ptr @__swift_instantiateConcreteTypeFromMangledName(ptr nonnull @"$ss18_DictionaryStorageCys6UInt32VSSGMD") #28
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
  %15 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %11, i64 %14
  %16 = load i32, ptr %15, align 8
  %.elt2 = getelementptr inbounds <{ %Ts6UInt32V, [4 x i8], %TSS }>, ptr %15, i64 0, i32 2
  %17 = load i64, ptr %.elt2, align 8
  %.elt2._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %.elt2, i64 0, i32 1
  %18 = load ptr, ptr %.elt2._guts._object._object, align 8
  %19 = add nuw nsw i64 %14, 1
  %20 = tail call swiftcc { i64, i1 } @"$ss22__RawDictionaryStorageC4findys10_HashTableV6BucketV6bucket_Sb5foundtxSHRzlFs6UInt32V_Tg5"(i32 %16, ptr swiftself %6)
  %21 = extractvalue { i64, i1 } %20, 0
  %22 = lshr i64 %21, 6
  %23 = and i64 %21, 63
  %24 = getelementptr inbounds %Ts13_UnsafeBitsetV4WordV, ptr %7, i64 %22
  %25 = shl nuw i64 1, %23
  %26 = load i64, ptr %24, align 8
  %27 = or i64 %25, %26
  store i64 %27, ptr %24, align 8
  %28 = load ptr, ptr %8, align 8
  %29 = getelementptr inbounds %Ts6UInt32V, ptr %28, i64 %21
  store i32 %16, ptr %29, align 4
  %30 = load ptr, ptr %9, align 8
  %31 = getelementptr inbounds %TSS, ptr %30, i64 %21
  store i64 %17, ptr %31, align 8
  %._guts._object._object = getelementptr inbounds %Ts13_StringObjectV, ptr %31, i64 0, i32 1
  store ptr %18, ptr %._guts._object._object, align 8
  %32 = load i64, ptr %10, align 8
  %33 = add i64 %32, 1
  store i64 %33, ptr %10, align 8
  %34 = icmp eq i64 %19, %2
  %35 = tail call ptr @swift_bridgeObjectRetain(ptr returned %18) #10
  br i1 %34, label %.loopexit, label %13
}

; Function Attrs: nounwind optsize
declare noalias ptr @dispatch_group_create() local_unnamed_addr #8

; Function Attrs: nofree noinline nosync nounwind readnone
define linkonce_odr hidden swiftcc %swift.metadata_response @"$sSo17OS_dispatch_queueCMa"(i64 %0) local_unnamed_addr #9 {
entry:
  %1 = load ptr, ptr @"$sSo17OS_dispatch_queueCML", align 8
  %2 = icmp eq ptr %1, null
  br i1 %2, label %cacheIsNull, label %cont

cacheIsNull:                                      ; preds = %entry
  %3 = load ptr, ptr @"OBJC_CLASS_REF_$_OS_dispatch_queue", align 8
  %4 = tail call ptr @objc_opt_self(ptr %3) #10
  %5 = tail call ptr @swift_getObjCClassMetadata(ptr %4) #32
  store atomic ptr %5, ptr @"$sSo17OS_dispatch_queueCML" release, align 8
  br label %cont

cont:                                             ; preds = %cacheIsNull, %entry
  %6 = phi ptr [ %1, %entry ], [ %5, %cacheIsNull ]
  %7 = insertvalue %swift.metadata_response undef, ptr %6, 0
  %8 = insertvalue %swift.metadata_response %7, i64 0, 1
  ret %swift.metadata_response %8
}

; Function Attrs: nounwind
declare ptr @objc_opt_self(ptr) local_unnamed_addr #10

; Function Attrs: mustprogress nofree nosync nounwind readnone willreturn
declare ptr @swift_getObjCClassMetadata(ptr) local_unnamed_addr #11

; Function Attrs: argmemonly mustprogress nocallback nofree nosync nounwind willreturn
declare void @llvm.lifetime.end.p0(i64 immarg, ptr nocapture) #1

declare swiftcc void @"$s8Dispatch0A3QoSV15userInteractiveACvgZ"(ptr noalias nocapture sret(%swift.opaque)) local_unnamed_addr #0

declare swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE10AttributesV10concurrentAEvgZ"(ptr noalias nocapture sret(%swift.opaque)) local_unnamed_addr #0

declare swiftcc ptr @"$sSo17OS_dispatch_queueC8DispatchE5label3qos10attributes20autoreleaseFrequency6targetABSS_AC0D3QoSVAbCE10AttributesVAbCE011AutoreleaseI0OABSgtcfC"(i64, ptr, ptr noalias nocapture, ptr noalias nocapture, ptr noalias nocapture, i64, ptr swiftself) local_unnamed_addr #0

declare swiftcc void @"$s8Dispatch0A3QoSV10backgroundACvgZ"(ptr noalias nocapture sret(%swift.opaque)) local_unnamed_addr #0

declare swiftcc void @"$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj"(ptr noalias nocapture sret(%swift.opaque), ptr noalias nocapture, ptr, ptr, ptr swiftself, ptr, ptr) local_unnamed_addr #0

; Function Attrs: nofree nounwind readonly
declare ptr @swift_getWitnessTable(ptr, ptr, ptr) local_unnamed_addr #12

; Function Attrs: mustprogress nofree noinline nounwind readonly willreturn
define linkonce_odr hidden ptr @__swift_instantiateConcreteTypeFromMangledName(ptr %0) local_unnamed_addr #13 {
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
  %13 = tail call swiftcc ptr @swift_getTypeByMangledNameInContext2(ptr %12, i64 %8, ptr null, ptr null) #33
  %14 = ptrtoint ptr %13 to i64
  store atomic i64 %14, ptr %0 monotonic, align 8
  br label %3
}

; Function Attrs: argmemonly nounwind
declare swiftcc ptr @swift_getTypeByMangledNameInContext2(ptr, i64, ptr, ptr) local_unnamed_addr #14

; Function Attrs: mustprogress nofree noinline nounwind readonly willreturn
define linkonce_odr hidden ptr @__swift_instantiateConcreteTypeFromMangledNameAbstract(ptr %0) local_unnamed_addr #13 {
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
  %13 = tail call swiftcc ptr @swift_getTypeByMangledNameInContextInMetadataState2(i64 255, ptr %12, i64 %8, ptr null, ptr null) #33
  %14 = ptrtoint ptr %13 to i64
  store atomic i64 %14, ptr %0 monotonic, align 8
  br label %3
}

; Function Attrs: argmemonly nounwind
declare swiftcc ptr @swift_getTypeByMangledNameInContextInMetadataState2(i64, ptr, i64, ptr, ptr) local_unnamed_addr #14

; Function Attrs: nounwind optsize
declare void @dispatch_group_enter(ptr noundef) local_unnamed_addr #8

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15 {
entry:
  %2 = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1
  %3 = getelementptr inbounds { %objc_block, %swift.function }, ptr %1, i64 0, i32 1
  %.fn.load = load ptr, ptr %3, align 8
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %1, i64 0, i32 1, i32 1
  %4 = load ptr, ptr %.data, align 8
  store ptr %.fn.load, ptr %2, align 8
  %.data2 = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1, i32 1
  store ptr %4, ptr %.data2, align 8
  %5 = tail call ptr @swift_retain(ptr returned %4) #29
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
declare ptr @swift_retain(ptr returned) local_unnamed_addr #16

; Function Attrs: nounwind
define internal void @block_destroy_helper(ptr nocapture readonly %0) #3 {
entry:
  %.data = getelementptr inbounds { %objc_block, %swift.function }, ptr %0, i64 0, i32 1, i32 1
  %1 = load ptr, ptr %.data, align 8
  tail call void @swift_release(ptr %1) #10
  ret void
}

; Function Attrs: nounwind
declare void @swift_release(ptr) local_unnamed_addr #10

declare ptr @_Block_copy(ptr) local_unnamed_addr

declare swiftcc void @"$s8Dispatch0A3QoSV11unspecifiedACvgZ"(ptr noalias nocapture sret(%swift.opaque)) local_unnamed_addr #0

; Function Attrs: nofree noinline nosync nounwind readnone
define internal ptr @"$sSo17OS_dispatch_queueC8DispatchE10AttributesVAEs10SetAlgebraACWlTm"(ptr %0, ptr %1, ptr %2) local_unnamed_addr #9 {
entry:
  %3 = load ptr, ptr %0, align 8
  %4 = icmp eq ptr %3, null
  br i1 %4, label %cacheIsNull, label %cont

cacheIsNull:                                      ; preds = %entry
  %5 = tail call swiftcc %swift.metadata_response %1(i64 255) #27
  %6 = extractvalue %swift.metadata_response %5, 0
  %7 = tail call ptr @swift_getWitnessTable(ptr nonnull %2, ptr %6, ptr undef) #28
  store atomic ptr %7, ptr %0 release, align 8
  br label %cont

cont:                                             ; preds = %cacheIsNull, %entry
  %8 = phi ptr [ %3, %entry ], [ %7, %cacheIsNull ]
  ret ptr %8
}

; Function Attrs: nofree noinline nosync nounwind readnone
define internal ptr @"$sSaySo17OS_dispatch_queueC8DispatchE10AttributesVGSayxGSTsWlTm"(ptr %0, ptr %1) local_unnamed_addr #9 {
entry:
  %2 = load ptr, ptr %0, align 8
  %3 = icmp eq ptr %2, null
  br i1 %3, label %cacheIsNull, label %cont

cacheIsNull:                                      ; preds = %entry
  %4 = tail call ptr @__swift_instantiateConcreteTypeFromMangledNameAbstract(ptr nonnull %1) #28
  %5 = tail call ptr @swift_getWitnessTable(ptr nonnull @"$sSayxGSTsMc", ptr %4, ptr undef) #28
  store atomic ptr %5, ptr %0 release, align 8
  br label %cont

cont:                                             ; preds = %cacheIsNull, %entry
  %6 = phi ptr [ %2, %entry ], [ %5, %cacheIsNull ]
  ret ptr %6
}

declare swiftcc void @"$sSo17OS_dispatch_queueC8DispatchE5async5group3qos5flags7executeySo0a1_b1_F0CSg_AC0D3QoSVAC0D13WorkItemFlagsVyyXBtF"(i64, ptr noalias nocapture, ptr noalias nocapture, ptr, ptr swiftself) local_unnamed_addr #0

; Function Attrs: nounwind
declare void @_Block_release(ptr) local_unnamed_addr #10

declare swiftcc void @"$sSo17OS_dispatch_groupC8DispatchE4waityyF"(ptr swiftself) local_unnamed_addr #0

; Function Attrs: nounwind
declare ptr @swift_bridgeObjectRetain(ptr returned) local_unnamed_addr #10

; Function Attrs: noinline nounwind
define linkonce_odr hidden void @"$sSD8IteratorV8_VariantOyxq___GSHRzr0_lWOe"(i64 %0, i64 %1, i64 %2, i64 %3, i64 %4) local_unnamed_addr #17 {
entry:
  %5 = and i64 %0, 9223372036854775807
  %6 = inttoptr i64 %5 to ptr
  tail call void @swift_release(ptr %6) #10
  ret void
}

; Function Attrs: nounwind
declare void @swift_bridgeObjectRelease(ptr) local_unnamed_addr #10

declare swiftcc void @"$ss5print_9separator10terminatoryypd_S2StF"(ptr, i64, ptr, i64, ptr) local_unnamed_addr #0

; Function Attrs: nounwind
declare ptr @swift_allocObject(ptr, i64, i64) local_unnamed_addr #10

declare swiftcc ptr @"$ss11CommandLineO9argumentsSaySSGvgZ"() local_unnamed_addr #0

; Function Attrs: mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i32, i1 } @llvm.umul.with.overflow.i32(i32, i32) #2

; Function Attrs: mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i32, i1 } @llvm.usub.with.overflow.i32(i32, i32) #2

; Function Attrs: noinline
declare swiftcc { i64, i64 } @"$ss13_StringObjectV10sharedUTF8SRys5UInt8VGvg"(i64, ptr) local_unnamed_addr #5

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftFoundation"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftObjectiveC"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftDarwin"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftCoreFoundation"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftDispatch"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftXPC"()

declare extern_weak void @"_swift_FORCE_LOAD_$_swiftIOKit"()

declare swiftcc { i64, ptr } @"$sSSySSxcs25LosslessStringConvertibleRzSTRzSJ7ElementSTRtzlufC"(ptr noalias nocapture, ptr, ptr, ptr) local_unnamed_addr #0

declare swiftcc { i64, i64, i64, ptr } @"$sSSySsSnySS5IndexVGcig"(i64, i64, i64, ptr) local_unnamed_addr #0

declare swiftcc { i64, ptr } @"$sSS18_uncheckedFromUTF8ySSSRys5UInt8VGFZ"(i64, i64) local_unnamed_addr #0

; Function Attrs: noinline
declare swiftcc i8 @"$sSS8UTF8ViewV17_foreignSubscript8positions5UInt8VSS5IndexV_tF"(i64, i64, ptr) local_unnamed_addr #5

; Function Attrs: noinline
declare swiftcc i64 @"$sSS8UTF8ViewV13_foreignIndex5afterSS0D0VAF_tF"(i64, i64, ptr) local_unnamed_addr #5

declare swiftcc i64 @"$sSS9UTF16ViewV5index_8offsetBySS5IndexVAF_SitF"(i64, i64, i64, ptr) local_unnamed_addr #0

; Function Attrs: noinline
declare swiftcc i64 @"$sSS8UTF8ViewV13_foreignIndex_8offsetBySS0D0VAF_SitF"(i64, i64, i64, ptr) local_unnamed_addr #5

; Function Attrs: noinline
declare swiftcc i64 @"$sSS8UTF8ViewV16_foreignDistance4from2toSiSS5IndexV_AGtF"(i64, i64, i64, ptr) local_unnamed_addr #5

; Function Attrs: mustprogress nounwind willreturn
declare zeroext i1 @swift_isUniquelyReferenced_nonNull_native(ptr) local_unnamed_addr #16

; Function Attrs: argmemonly mustprogress nocallback nofree nounwind willreturn
declare void @llvm.memmove.p0.p0.i64(ptr nocapture writeonly, ptr nocapture readonly, i64, i1 immarg) #18

; Function Attrs: noinline
declare swiftcc ptr @"$sSa28_allocateBufferUninitialized15minimumCapacitys06_ArrayB0VyxGSi_tFZ"(i64, ptr) local_unnamed_addr #5

; Function Attrs: argmemonly mustprogress nocallback nofree nounwind willreturn
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #18

; Function Attrs: mustprogress nounwind willreturn
declare void @swift_arrayInitWithCopy(ptr, ptr, i64, ptr) local_unnamed_addr #16

; Function Attrs: mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn
declare i64 @llvm.cttz.i64(i64, i1 immarg) #2

; Function Attrs: nounwind
define private swiftcc void @objectdestroy(ptr swiftself %0) #3 {
entry:
  tail call void @swift_deallocObject(ptr %0, i64 24, i64 7) #10
  ret void
}

; Function Attrs: nounwind
declare void @swift_deallocObject(ptr, i64, i64) local_unnamed_addr #10

define internal swiftcc void @"$s4mainyyYbcfU2_yyYbcfU_TA"(ptr nocapture readonly swiftself %0) #0 {
entry:
  %1 = getelementptr inbounds <{ %swift.refcounted, %Ts6UInt32V, %Ts6UInt32V }>, ptr %0, i64 0, i32 1
  %2 = load i32, ptr %1, align 8
  %3 = getelementptr inbounds <{ %swift.refcounted, %Ts6UInt32V, %Ts6UInt32V }>, ptr %0, i64 0, i32 2
  %4 = load i32, ptr %3, align 4
  tail call swiftcc void @"$s4mainyyYbcfU2_yyYbcfU_"(i32 %2, i32 %4)
  ret void
}

; Function Attrs: nounwind optsize
declare void @dispatch_group_leave(ptr noundef) local_unnamed_addr #8

; Function Attrs: nounwind
define private swiftcc void @objectdestroy.14(ptr swiftself %0) #3 {
entry:
  %._guts._object._object = getelementptr inbounds <{ %swift.refcounted, %Ts6UInt32V, [4 x i8], %TSS }>, ptr %0, i64 0, i32 3, i32 0, i32 0, i32 1
  %toDestroy = load ptr, ptr %._guts._object._object, align 8
  tail call void @swift_bridgeObjectRelease(ptr %toDestroy) #10
  tail call void @swift_deallocObject(ptr %0, i64 40, i64 7) #10
  ret void
}

define internal swiftcc void @"$s4mainyyYbcfU2_yyYbcfU_yyYbcfU_TA"(ptr nocapture readonly swiftself %0) #0 {
entry:
  %1 = getelementptr inbounds <{ %swift.refcounted, %Ts6UInt32V, [4 x i8], %TSS }>, ptr %0, i64 0, i32 1
  %2 = load i32, ptr %1, align 8
  %3 = getelementptr inbounds <{ %swift.refcounted, %Ts6UInt32V, [4 x i8], %TSS }>, ptr %0, i64 0, i32 3
  %4 = load i64, ptr %3, align 8
  %._guts._object._object = getelementptr inbounds <{ %swift.refcounted, %Ts6UInt32V, [4 x i8], %TSS }>, ptr %0, i64 0, i32 3, i32 0, i32 0, i32 1
  %5 = load ptr, ptr %._guts._object._object, align 8
  %6 = tail call ptr @swift_bridgeObjectRetain(ptr returned %5) #10
  tail call swiftcc void @"$sSD8_VariantV8setValue_6forKeyyq_n_xtFs6UInt32V_SSTg5"(i64 %4, ptr %5, i32 %2, ptr nocapture nonnull swiftself dereferenceable(8) @"$s4main8messagesSDys6UInt32VSSGvp")
  %7 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  tail call void @dispatch_group_leave(ptr %7)
  ret void
}

; Function Attrs: noinline
declare swiftcc void @"$ss53KEY_TYPE_OF_DICTIONARY_VIOLATES_HASHABLE_REQUIREMENTSys5NeverOypXpF"(ptr) local_unnamed_addr #5

declare swiftcc ptr @"$ss18_DictionaryStorageC6resize8original8capacity4moveAByxq_Gs05__RawaB0C_SiSbtFZ"(ptr, i64, i1, ptr swiftself) local_unnamed_addr #0

declare swiftcc i64 @"$ss6HasherV5_hash4seed5bytes5countS2i_s6UInt64VSitFZ"(i64, i64, i64) local_unnamed_addr #0

declare swiftcc ptr @"$ss18_DictionaryStorageC4copy8originalAByxq_Gs05__RawaB0C_tFZ"(ptr, ptr swiftself) local_unnamed_addr #0

declare swiftcc void @"$ss11_StringGutsV4growyySiF"(i64, ptr nocapture swiftself dereferenceable(16)) local_unnamed_addr #0

declare swiftcc void @"$sSS6appendyySSF"(i64, ptr, ptr nocapture swiftself dereferenceable(16)) local_unnamed_addr #0

declare swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture swiftself, ptr, ptr) local_unnamed_addr #0

; Function Attrs: nounwind
define private swiftcc void @objectdestroy.20(ptr swiftself %0) #3 {
entry:
  %1 = getelementptr inbounds <{ %swift.refcounted, <{ %T4main4TreeO, %T4main4TreeO }> }>, ptr %0, i64 0, i32 1
  %2 = load ptr, ptr %1, align 8
  tail call void @swift_release(ptr %2) #10
  %.right = getelementptr inbounds <{ %swift.refcounted, <{ %T4main4TreeO, %T4main4TreeO }> }>, ptr %0, i64 0, i32 1, i32 1
  %3 = load ptr, ptr %.right, align 8
  tail call void @swift_release(ptr %3) #10
  tail call void @swift_deallocObject(ptr %0, i64 32, i64 7) #10
  ret void
}

; Function Attrs: nounwind
define private swiftcc void @objectdestroy.24(ptr swiftself %0) #3 {
entry:
  %1 = getelementptr inbounds <{ %swift.refcounted, %T4main4TreeO }>, ptr %0, i64 0, i32 1
  %2 = load ptr, ptr %1, align 8
  tail call void @swift_release(ptr %2) #10
  tail call void @swift_deallocObject(ptr %0, i64 24, i64 7) #10
  ret void
}

define internal swiftcc void @"$s4mainyyYbcfU1_yyYbcfU_TA"(ptr nocapture readonly swiftself %0) #0 {
entry:
  %1 = alloca %TSS, align 8
  %2 = alloca %Ts11_StringGutsV, align 8
  %3 = alloca %Ts6UInt32V, align 4
  %4 = alloca %Ts6UInt32V, align 4
  %5 = getelementptr inbounds <{ %swift.refcounted, %T4main4TreeO }>, ptr %0, i64 0, i32 1
  %6 = load i64, ptr %5, align 8
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %1)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %2)
  store i64 0, ptr %2, align 8
  %._object._object.i = getelementptr inbounds %Ts13_StringObjectV, ptr %2, i64 0, i32 1
  store ptr inttoptr (i64 -2305843009213693952 to ptr), ptr %._object._object.i, align 8
  call swiftcc void @"$ss11_StringGutsV4growyySiF"(i64 38, ptr nocapture nonnull swiftself dereferenceable(16) %2)
  %7 = load ptr, ptr %._object._object.i, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %2)
  tail call void @swift_bridgeObjectRelease(ptr %7) #10
  store i64 -3458764513820540903, ptr %1, align 8
  %._guts._object._object.i = getelementptr inbounds %Ts13_StringObjectV, ptr %1, i64 0, i32 1
  store ptr inttoptr (i64 or (i64 sub (i64 ptrtoint (ptr @".str.25.long lived tree of depth " to i64), i64 32), i64 -9223372036854775808) to ptr), ptr %._guts._object._object.i, align 8
  %8 = load i32, ptr @"$s4main8maxDepths6UInt32Vvp", align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3)
  store i32 %8, ptr %3, align 4
  %9 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %3, ptr nonnull @"$ss6UInt32VN", ptr nonnull @"$ss6UInt32Vs23CustomStringConvertiblesWP")
  %10 = extractvalue { i64, ptr } %9, 0
  %11 = extractvalue { i64, ptr } %9, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %10, ptr %11, ptr nocapture nonnull swiftself dereferenceable(16) %1)
  tail call void @swift_bridgeObjectRelease(ptr %11) #10
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3)
  call swiftcc void @"$sSS6appendyySSF"(i64 4209567563902033929, ptr nonnull inttoptr (i64 -1657324662872342496 to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %1)
  %12 = tail call swiftcc i32 @"$s4main9itemCheckys6UInt32VAA4TreeOF"(i64 %6)
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4)
  store i32 %12, ptr %4, align 4
  %13 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %4, ptr nonnull @"$ss6UInt32VN", ptr nonnull @"$ss6UInt32Vs23CustomStringConvertiblesWP")
  %14 = extractvalue { i64, ptr } %13, 0
  %15 = extractvalue { i64, ptr } %13, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %14, ptr %15, ptr nocapture nonnull swiftself dereferenceable(16) %1)
  tail call void @swift_bridgeObjectRelease(ptr %15) #10
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4)
  %16 = load i64, ptr %1, align 8
  %17 = load ptr, ptr %._guts._object._object.i, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %1)
  tail call swiftcc void @"$sSD8_VariantV8setValue_6forKeyyq_n_xtFs6UInt32V_SSTg5"(i64 %16, ptr %17, i32 -1, ptr nocapture nonnull swiftself dereferenceable(8) @"$s4main8messagesSDys6UInt32VSSGvp")
  %18 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  tail call void @dispatch_group_leave(ptr %18)
  ret void
}

define internal swiftcc void @"$s4mainyyYbcfU0_yyYbcfU_TA"(ptr nocapture readonly swiftself %0) #0 {
entry:
  %1 = alloca %TSS, align 8
  %2 = alloca %Ts11_StringGutsV, align 8
  %3 = alloca %Ts6UInt32V, align 4
  %4 = alloca %Ts6UInt32V, align 4
  %5 = getelementptr inbounds <{ %swift.refcounted, %T4main4TreeO }>, ptr %0, i64 0, i32 1
  %6 = load i64, ptr %5, align 8
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %1)
  call void @llvm.lifetime.start.p0(i64 16, ptr nonnull %2)
  store i64 0, ptr %2, align 8
  %._object._object.i = getelementptr inbounds %Ts13_StringObjectV, ptr %2, i64 0, i32 1
  store ptr inttoptr (i64 -2305843009213693952 to ptr), ptr %._object._object.i, align 8
  call swiftcc void @"$ss11_StringGutsV4growyySiF"(i64 35, ptr nocapture nonnull swiftself dereferenceable(16) %2)
  %7 = load ptr, ptr %._object._object.i, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %2)
  tail call void @swift_bridgeObjectRelease(ptr %7) #10
  store i64 -3458764513820540906, ptr %1, align 8
  %._guts._object._object.i = getelementptr inbounds %Ts13_StringObjectV, ptr %1, i64 0, i32 1
  store ptr inttoptr (i64 or (i64 sub (i64 ptrtoint (ptr @".str.22.stretch tree of depth " to i64), i64 32), i64 -9223372036854775808) to ptr), ptr %._guts._object._object.i, align 8
  %8 = load i32, ptr @"$s4main5depths6UInt32Vvp", align 4
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %3)
  store i32 %8, ptr %3, align 4
  %9 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %3, ptr nonnull @"$ss6UInt32VN", ptr nonnull @"$ss6UInt32Vs23CustomStringConvertiblesWP")
  %10 = extractvalue { i64, ptr } %9, 0
  %11 = extractvalue { i64, ptr } %9, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %10, ptr %11, ptr nocapture nonnull swiftself dereferenceable(16) %1)
  tail call void @swift_bridgeObjectRelease(ptr %11) #10
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %3)
  call swiftcc void @"$sSS6appendyySSF"(i64 4209567563902033929, ptr nonnull inttoptr (i64 -1657324662872342496 to ptr), ptr nocapture nonnull swiftself dereferenceable(16) %1)
  %12 = tail call swiftcc i32 @"$s4main9itemCheckys6UInt32VAA4TreeOF"(i64 %6)
  call void @llvm.lifetime.start.p0(i64 4, ptr nonnull %4)
  store i32 %12, ptr %4, align 4
  %13 = call swiftcc { i64, ptr } @"$ss23CustomStringConvertibleP11descriptionSSvgTj"(ptr noalias nocapture nonnull swiftself %4, ptr nonnull @"$ss6UInt32VN", ptr nonnull @"$ss6UInt32Vs23CustomStringConvertiblesWP")
  %14 = extractvalue { i64, ptr } %13, 0
  %15 = extractvalue { i64, ptr } %13, 1
  call swiftcc void @"$sSS6appendyySSF"(i64 %14, ptr %15, ptr nocapture nonnull swiftself dereferenceable(16) %1)
  tail call void @swift_bridgeObjectRelease(ptr %15) #10
  call void @llvm.lifetime.end.p0(i64 4, ptr nonnull %4)
  %16 = load i64, ptr %1, align 8
  %17 = load ptr, ptr %._guts._object._object.i, align 8
  call void @llvm.lifetime.end.p0(i64 16, ptr nonnull %1)
  tail call swiftcc void @"$sSD8_VariantV8setValue_6forKeyyq_n_xtFs6UInt32V_SSTg5"(i64 %16, ptr %17, i32 0, ptr nocapture nonnull swiftself dereferenceable(8) @"$s4main8messagesSDys6UInt32VSSGvp")
  %18 = load ptr, ptr @"$s4main5groupSo012OS_dispatch_B0Cvp", align 8
  tail call void @dispatch_group_leave(ptr %18)
  ret void
}

declare swiftcc ptr @"$ss18_DictionaryStorageC8allocate8capacityAByxq_GSi_tFZ"(i64, ptr swiftself) local_unnamed_addr #0

; Function Attrs: mustprogress nounwind willreturn
define internal ptr @"$s4main4TreeOwCP"(ptr noalias returned writeonly %dest, ptr noalias nocapture readonly %src, ptr nocapture readnone %Tree) #15 {
entry:
  %0 = load ptr, ptr %src, align 8
  store ptr %0, ptr %dest, align 8
  %1 = tail call ptr @swift_retain(ptr returned %0) #29
  ret ptr %dest
}

; Function Attrs: nounwind
define internal void @"$s4main4TreeOwxx"(ptr noalias nocapture readonly %object, ptr nocapture readnone %Tree) #3 {
entry:
  %toDestroy = load ptr, ptr %object, align 8
  tail call void @swift_release(ptr %toDestroy) #10
  ret void
}

; Function Attrs: nounwind
define internal ptr @"$s4main4TreeOwca"(ptr returned %dest, ptr nocapture readonly %src, ptr nocapture readnone %Tree) #3 {
entry:
  %0 = load ptr, ptr %src, align 8
  %oldValue = load ptr, ptr %dest, align 8
  store ptr %0, ptr %dest, align 8
  %1 = tail call ptr @swift_retain(ptr returned %0) #29
  tail call void @swift_release(ptr %oldValue) #10
  ret ptr %dest
}

; Function Attrs: nounwind
define linkonce_odr hidden ptr @__swift_memcpy8_8(ptr %0, ptr %1, ptr %2) #3 {
entry:
  %3 = load i64, ptr %1, align 8
  store i64 %3, ptr %0, align 8
  ret ptr %0
}

; Function Attrs: nounwind
define internal ptr @"$s4main4TreeOwta"(ptr noalias returned %dest, ptr noalias nocapture readonly %src, ptr nocapture readnone %Tree) #3 {
entry:
  %0 = load ptr, ptr %src, align 8
  %oldValue = load ptr, ptr %dest, align 8
  store ptr %0, ptr %dest, align 8
  tail call void @swift_release(ptr %oldValue) #10
  ret ptr %dest
}

; Function Attrs: argmemonly mustprogress nofree nosync nounwind readonly willreturn
define internal i32 @"$s4main4TreeOwet"(ptr noalias nocapture readonly %value, i32 %numEmptyCases, ptr nocapture readnone %Tree) #19 {
entry:
  %0 = icmp eq i32 %numEmptyCases, 0
  br i1 %0, label %16, label %1

1:                                                ; preds = %entry
  %2 = icmp ugt i32 %numEmptyCases, 2147483646
  br i1 %2, label %3, label %is-valid-pointer

3:                                                ; preds = %1
  %4 = getelementptr inbounds i8, ptr %value, i64 8
  %5 = load i8, ptr %4, align 1
  %6 = icmp eq i8 %5, 0
  br i1 %6, label %is-valid-pointer, label %7

7:                                                ; preds = %3
  %8 = load i32, ptr %value, align 8
  %9 = add i32 %8, 2147483647
  br label %16

is-valid-pointer:                                 ; preds = %1, %3
  %10 = load i64, ptr %value, align 8
  %11 = tail call i64 @llvm.umin.i64(i64 %10, i64 4294967295)
  %12 = trunc i64 %11 to i32
  %13 = add i32 %12, 1
  %14 = icmp ult i32 %13, 2
  %15 = select i1 %14, i32 0, i32 %12
  br label %16

16:                                               ; preds = %entry, %is-valid-pointer, %7
  %17 = phi i32 [ %15, %is-valid-pointer ], [ %9, %7 ], [ 0, %entry ]
  ret i32 %17
}

; Function Attrs: argmemonly mustprogress nofree norecurse nosync nounwind willreturn writeonly
define internal void @"$s4main4TreeOwst"(ptr noalias nocapture writeonly %value, i32 %whichCase, i32 %numEmptyCases, ptr nocapture readnone %Tree) #20 {
entry:
  %0 = getelementptr inbounds i8, ptr %value, i64 8
  %1 = icmp ugt i32 %numEmptyCases, 2147483646
  %2 = icmp ult i32 %whichCase, 2147483647
  br i1 %2, label %3, label %9

3:                                                ; preds = %entry
  br i1 %1, label %4, label %5

4:                                                ; preds = %3
  store i8 0, ptr %0, align 1
  br label %5

5:                                                ; preds = %3, %4
  %6 = icmp eq i32 %whichCase, 0
  br i1 %6, label %12, label %7

7:                                                ; preds = %5
  %8 = zext i32 %whichCase to i64
  store i64 %8, ptr %value, align 8
  br label %12

9:                                                ; preds = %entry
  %10 = add i32 %whichCase, -2147483647
  store i64 0, ptr %value, align 8
  store i32 %10, ptr %value, align 8
  br i1 %1, label %11, label %12

11:                                               ; preds = %9
  store i8 1, ptr %0, align 1
  br label %12

12:                                               ; preds = %9, %11, %7, %5
  ret void
}

; Function Attrs: argmemonly mustprogress nofree nosync nounwind readonly willreturn
define internal i32 @"$s4main4TreeOwug"(ptr noalias nocapture readonly %value, ptr nocapture readnone %Tree) #19 {
entry:
  %0 = load i64, ptr %value, align 8
  %1 = tail call i64 @llvm.umin.i64(i64 %0, i64 4294967295)
  %2 = trunc i64 %1 to i32
  %3 = add i32 %2, 1
  ret i32 %3
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind readnone willreturn
define internal void @"$s4main4TreeOwup"(ptr noalias nocapture %value, ptr nocapture %Tree) #21 {
entry:
  ret void
}

; Function Attrs: argmemonly mustprogress nofree norecurse nosync nounwind willreturn writeonly
define internal void @"$s4main4TreeOwui"(ptr noalias nocapture writeonly %value, i32 %tag, ptr nocapture readnone %Tree) #20 {
entry:
  %0 = icmp sgt i32 %tag, -1
  br i1 %0, label %1, label %5

1:                                                ; preds = %entry
  %2 = icmp eq i32 %tag, 0
  br i1 %2, label %8, label %3

3:                                                ; preds = %1
  %4 = add nsw i32 %tag, -1
  br label %.sink.split

5:                                                ; preds = %entry
  %6 = xor i32 %tag, -2147483648
  br label %.sink.split

.sink.split:                                      ; preds = %3, %5
  %.sink1 = phi i32 [ %6, %5 ], [ %4, %3 ]
  %7 = zext i32 %.sink1 to i64
  store i64 %7, ptr %value, align 8
  br label %8

8:                                                ; preds = %.sink.split, %1
  ret void
}

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind readnone willreturn
define hidden swiftcc %swift.metadata_response @"$s4main4TreeOMa"(i64 %0) #22 {
entry:
  ret %swift.metadata_response { ptr getelementptr inbounds (<{ ptr, ptr, i64, ptr }>, ptr @"$s4main4TreeOMf", i32 0, i32 2), i64 0 }
}

; Function Attrs: optsize
declare i64 @malloc_size(ptr noundef) local_unnamed_addr #23

; Function Attrs: inaccessiblememonly nocallback nofree nosync nounwind willreturn
declare void @llvm.assume(i1 noundef) #24

; Function Attrs: nocallback nofree nosync nounwind readnone speculatable willreturn
declare i32 @llvm.umax.i32(i32, i32) #25

; Function Attrs: nocallback nofree nosync nounwind readnone speculatable willreturn
declare i64 @llvm.smin.i64(i64, i64) #25

; Function Attrs: nocallback nofree nosync nounwind readnone speculatable willreturn
declare i64 @llvm.smax.i64(i64, i64) #25

; Function Attrs: nocallback nofree nosync nounwind readnone speculatable willreturn
declare i64 @llvm.umin.i64(i64, i64) #25

; Function Attrs: argmemonly nocallback nofree nounwind willreturn writeonly
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #26

; Function Attrs: nounwind
declare ptr @swift_bridgeObjectRetain_n(ptr, i32) #10

; Function Attrs: mustprogress nounwind willreturn
define internal ptr @"$s4main4TreeOwcp"(ptr noalias returned writeonly %0, ptr noalias nocapture readonly %1, ptr nocapture readnone %2) #15 {
  %4 = tail call ptr @"$s4main4TreeOwCP"(ptr noalias returned writeonly %0, ptr noalias nocapture readonly %1, ptr nocapture readnone %2) #15
  ret ptr %4
}

; Function Attrs: nounwind
define private swiftcc void @objectdestroy.30(ptr swiftself %0) #3 {
  tail call swiftcc void @objectdestroy.24(ptr swiftself %0) #3
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.1(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.4(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.7(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.10(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.16(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.26(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define internal void @block_copy_helper.32(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15 {
  tail call void @block_copy_helper(ptr nocapture writeonly %0, ptr nocapture readonly %1) #15
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
define internal void @block_destroy_helper.8(ptr nocapture readonly %0) #3 {
  tail call void @block_destroy_helper(ptr nocapture readonly %0) #3
  ret void
}

; Function Attrs: nounwind
define internal void @block_destroy_helper.11(ptr nocapture readonly %0) #3 {
  tail call void @block_destroy_helper(ptr nocapture readonly %0) #3
  ret void
}

; Function Attrs: nounwind
define internal void @block_destroy_helper.17(ptr nocapture readonly %0) #3 {
  tail call void @block_destroy_helper(ptr nocapture readonly %0) #3
  ret void
}

; Function Attrs: nounwind
define internal void @block_destroy_helper.27(ptr nocapture readonly %0) #3 {
  tail call void @block_destroy_helper(ptr nocapture readonly %0) #3
  ret void
}

; Function Attrs: nounwind
define internal void @block_destroy_helper.33(ptr nocapture readonly %0) #3 {
  tail call void @block_destroy_helper(ptr nocapture readonly %0) #3
  ret void
}

attributes #0 = { "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #1 = { argmemonly mustprogress nocallback nofree nosync nounwind willreturn }
attributes #2 = { mustprogress nocallback nofree nosync nounwind readnone speculatable willreturn }
attributes #3 = { nounwind "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #4 = { nofree nosync nounwind readonly "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #5 = { noinline "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #6 = { noinline "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #7 = { "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #8 = { nounwind optsize "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #9 = { nofree noinline nosync nounwind readnone "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #10 = { nounwind }
attributes #11 = { mustprogress nofree nosync nounwind readnone willreturn }
attributes #12 = { nofree nounwind readonly }
attributes #13 = { mustprogress nofree noinline nounwind readonly willreturn "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #14 = { argmemonly nounwind }
attributes #15 = { mustprogress nounwind willreturn "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #16 = { mustprogress nounwind willreturn }
attributes #17 = { noinline nounwind "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #18 = { argmemonly mustprogress nocallback nofree nounwind willreturn }
attributes #19 = { argmemonly mustprogress nofree nosync nounwind readonly willreturn "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #20 = { argmemonly mustprogress nofree norecurse nosync nounwind willreturn writeonly "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #21 = { mustprogress nofree norecurse nosync nounwind readnone willreturn "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #22 = { mustprogress nofree noinline norecurse nosync nounwind readnone willreturn "frame-pointer"="non-leaf" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #23 = { optsize "frame-pointer"="non-leaf" "no-trapping-math"="true" "probe-stack"="__chkstk_darwin" "stack-protector-buffer-size"="8" "target-cpu"="apple-a12" "target-features"="+aes,+crc,+crypto,+fp-armv8,+fullfp16,+lse,+neon,+ras,+rcpc,+rdm,+sha2,+v8.1a,+v8.2a,+v8.3a,+v8a,+zcm,+zcz" }
attributes #24 = { inaccessiblememonly nocallback nofree nosync nounwind willreturn }
attributes #25 = { nocallback nofree nosync nounwind readnone speculatable willreturn }
attributes #26 = { argmemonly nocallback nofree nounwind willreturn writeonly }
attributes #27 = { nounwind readnone }
attributes #28 = { nounwind readonly }
attributes #29 = { nounwind willreturn }
attributes #30 = { noinline }
attributes #31 = { optsize }
attributes #32 = { nounwind readnone willreturn }
attributes #33 = { argmemonly nounwind readonly }

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
!39 = !{!40}
!40 = distinct !{!40, !41, !"$sSS8_copyingySSSsFZSSSRys5UInt8VGXEfU0_: argument 0"}
!41 = distinct !{!41, !"$sSS8_copyingySSSsFZSSSRys5UInt8VGXEfU0_"}
!42 = !{!"branch_weights", i32 2002, i32 2000}
!43 = !{i64 0, i64 65}
