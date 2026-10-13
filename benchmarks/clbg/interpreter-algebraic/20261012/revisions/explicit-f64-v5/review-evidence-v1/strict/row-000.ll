define internal fastcc void @_RNvNtNtNtCscPDRiDoTWel_8mech_gpu8sequence7runtime8resident10scalar_row(ptr dead_on_unwind noalias noundef nonnull writable writeonly align 8 captures(none) dereferenceable(24) %0, ptr noalias noundef nonnull readonly align 8 captures(none) dereferenceable(88) %1, ptr noalias noundef nonnull readonly align 8 captures(none) dereferenceable(88) %2, i64 noundef %3, ptr noalias noundef nonnull readonly align 8 captures(address) %4, i64 noundef range(i64 0, 1152921504606846976) %5, i1 noundef zeroext %6, i1 noundef zeroext %7) unnamed_addr #6 personality ptr @rust_eh_personality {
  %9 = alloca [24 x i8], align 8
  %10 = alloca [16 x i8], align 8
  %11 = load i64, ptr %2, align 8, !range !3, !noundef !4
  %12 = icmp eq i64 %11, -9223372036854775808
  %13 = icmp eq i64 %5, 0
  br i1 %12, label %1535, label %14

14:                                               ; preds = %8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !7257)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !7260)
  br i1 %13, label %1532, label %15

15:                                               ; preds = %14
  %16 = getelementptr inbounds nuw i8, ptr %2, i64 32
  %17 = load ptr, ptr %16, align 8, !alias.scope !7262, !noalias !7260, !nonnull !4, !noundef !4
  %18 = getelementptr inbounds nuw i8, ptr %2, i64 40
  %19 = load i64, ptr %18, align 8, !alias.scope !7262, !noalias !7260, !noundef !4
  %20 = mul nuw nsw i64 %19, 24
  %21 = getelementptr inbounds nuw i8, ptr %17, i64 %20
  %22 = icmp eq i64 %19, 0
  %23 = getelementptr inbounds nuw i8, ptr %2, i64 16
  %24 = load i64, ptr %23, align 8, !alias.scope !7257, !noalias !7260
  %25 = getelementptr inbounds nuw i8, ptr %2, i64 8
  %26 = load ptr, ptr %25, align 8, !alias.scope !7257, !noalias !7260, !nonnull !4
  %27 = getelementptr inbounds nuw i8, ptr %2, i64 80
  %28 = load i64, ptr %27, align 8, !alias.scope !7257, !noalias !7260
  %29 = uitofp i64 %3 to double
  %30 = bitcast double %29 to i64
  %31 = getelementptr inbounds nuw i8, ptr %2, i64 56
  %32 = load ptr, ptr %31, align 8, !alias.scope !7257, !noalias !7260, !nonnull !4
  %33 = getelementptr inbounds nuw i8, ptr %2, i64 64
  %34 = load i64, ptr %33, align 8, !alias.scope !7257, !noalias !7260
  %35 = shl nuw nsw i64 %34, 5
  %36 = getelementptr inbounds nuw i8, ptr %32, i64 %35
  %37 = icmp eq i64 %34, 0
  %38 = getelementptr inbounds nuw i8, ptr %2, i64 72
  %39 = load i64, ptr %38, align 8, !alias.scope !7257, !noalias !7260
  %40 = icmp ult i64 %39, %24
  %41 = getelementptr inbounds nuw [32 x i64], ptr %26, i64 %39
  %42 = getelementptr inbounds nuw i8, ptr %41, i64 16
  %43 = getelementptr inbounds nuw i8, ptr %41, i64 32
  %44 = getelementptr inbounds nuw i8, ptr %41, i64 48
  %45 = getelementptr inbounds nuw i8, ptr %41, i64 64
  %46 = getelementptr inbounds nuw i8, ptr %41, i64 80
  %47 = getelementptr inbounds nuw i8, ptr %41, i64 96
  %48 = getelementptr inbounds nuw i8, ptr %41, i64 112
  %49 = getelementptr inbounds nuw i8, ptr %41, i64 128
  %50 = getelementptr inbounds nuw i8, ptr %41, i64 144
  %51 = getelementptr inbounds nuw i8, ptr %41, i64 160
  %52 = getelementptr inbounds nuw i8, ptr %41, i64 176
  %53 = getelementptr inbounds nuw i8, ptr %41, i64 192
  %54 = getelementptr inbounds nuw i8, ptr %41, i64 208
  %55 = getelementptr inbounds nuw i8, ptr %41, i64 224
  %56 = getelementptr inbounds nuw i8, ptr %41, i64 240
  %57 = getelementptr inbounds nuw i8, ptr %41, i64 16
  %58 = getelementptr inbounds nuw i8, ptr %41, i64 32
  %59 = getelementptr inbounds nuw i8, ptr %41, i64 48
  %60 = getelementptr inbounds nuw i8, ptr %41, i64 64
  %61 = getelementptr inbounds nuw i8, ptr %41, i64 80
  %62 = getelementptr inbounds nuw i8, ptr %41, i64 96
  %63 = getelementptr inbounds nuw i8, ptr %41, i64 112
  %64 = getelementptr inbounds nuw i8, ptr %41, i64 128
  %65 = getelementptr inbounds nuw i8, ptr %41, i64 144
  %66 = getelementptr inbounds nuw i8, ptr %41, i64 160
  %67 = getelementptr inbounds nuw i8, ptr %41, i64 176
  %68 = getelementptr inbounds nuw i8, ptr %41, i64 192
  %69 = getelementptr inbounds nuw i8, ptr %41, i64 208
  %70 = getelementptr inbounds nuw i8, ptr %41, i64 224
  %71 = getelementptr inbounds nuw i8, ptr %41, i64 240
  br label %75

72:                                               ; preds = %1521, %1509, %1503, %1409
  %73 = phi double [ %1518, %1509 ], [ %1504, %1503 ], [ %1410, %1409 ], [ %1530, %1521 ]
  %74 = icmp eq i64 %82, 0
  br i1 %74, label %1532, label %75

75:                                               ; preds = %72, %15
  %76 = phi double [ 0.000000e+00, %15 ], [ %73, %72 ]
  %77 = phi ptr [ %4, %15 ], [ %81, %72 ]
  %78 = phi i64 [ %5, %15 ], [ %82, %72 ]
  %79 = phi i64 [ 0, %15 ], [ %83, %72 ]
  %80 = tail call noundef i64 @llvm.umin.i64(i64 %78, i64 32)
  %81 = getelementptr inbounds nuw double, ptr %77, i64 %80
  %82 = sub nuw nsw i64 %78, %80
  %83 = add i64 %79, 1
  %84 = shl i64 %79, 5
  tail call void @llvm.experimental.noalias.scope.decl(metadata !7265)
  br i1 %22, label %220, label %85

85:                                               ; preds = %75
  %86 = shl nuw nsw i64 %80, 3
  %87 = add nsw i64 %86, -8
  %88 = lshr exact i64 %87, 3
  %89 = add nuw nsw i64 %88, 1
  %90 = add nsw i64 %86, -8
  %91 = lshr exact i64 %90, 3
  %92 = add nuw nsw i64 %91, 1
  %93 = add nsw i64 %86, -8
  %94 = lshr exact i64 %93, 3
  %95 = add nuw nsw i64 %94, 1
  %96 = icmp ult i64 %93, 56
  %97 = and i64 %95, 4611686018427387896
  %98 = shl i64 %97, 3
  %99 = icmp eq i64 %95, %97
  %100 = icmp ult i64 %90, 56
  %101 = and i64 %92, 4611686018427387896
  %102 = shl i64 %101, 3
  %103 = insertelement <2 x i64> poison, i64 %84, i64 0
  %104 = shufflevector <2 x i64> %103, <2 x i64> poison, <2 x i32> zeroinitializer
  %105 = add <2 x i64> splat (i64 2), %104
  %106 = add <2 x i64> splat (i64 4), %104
  %107 = add <2 x i64> splat (i64 6), %104
  %108 = icmp eq i64 %92, %101
  %109 = icmp ult i64 %87, 56
  %110 = and i64 %89, 4611686018427387896
  %111 = shl i64 %110, 3
  %112 = insertelement <2 x i64> poison, i64 %84, i64 0
  %113 = shufflevector <2 x i64> %112, <2 x i64> poison, <2 x i32> zeroinitializer
  %114 = add <2 x i64> splat (i64 2), %113
  %115 = add <2 x i64> splat (i64 4), %113
  %116 = add <2 x i64> splat (i64 6), %113
  %117 = icmp eq i64 %89, %110
  br label %118

118:                                              ; preds = %136, %85
  %119 = phi ptr [ %120, %136 ], [ %17, %85 ]
  %120 = getelementptr inbounds nuw i8, ptr %119, i64 24
  %121 = load i64, ptr %119, align 8, !noalias !7262, !noundef !4
  %122 = icmp ult i64 %121, %24
  br i1 %122, label %123, label %1311

123:                                              ; preds = %118
  %124 = getelementptr inbounds nuw i8, ptr %119, i64 16
  %125 = load i64, ptr %124, align 8, !noalias !7262, !noundef !4
  %126 = getelementptr inbounds nuw i8, ptr %119, i64 8
  %127 = load i8, ptr %126, align 8, !range !3475, !noalias !7262, !noundef !4
  %128 = getelementptr inbounds nuw [32 x i64], ptr %26, i64 %121
  %129 = getelementptr inbounds nuw i8, ptr %128, i64 %86
  %130 = icmp eq i64 %125, %28
  %131 = icmp eq i8 %127, 1
  br i1 %130, label %156, label %138

132:                                              ; preds = %154, %132
  %133 = phi ptr [ %134, %132 ], [ %155, %154 ]
  %134 = getelementptr inbounds nuw i8, ptr %133, i64 8
  store i64 %139, ptr %133, align 8, !noalias !7262
  %135 = icmp eq ptr %134, %129
  br i1 %135, label %136, label %132, !llvm.loop !7266

136:                                              ; preds = %213, %205, %201, %179, %153, %132
  %137 = icmp eq ptr %120, %21
  br i1 %137, label %220, label %118

138:                                              ; preds = %123
  %139 = select i1 %131, i64 %3, i64 %30
  br i1 %96, label %154, label %140

140:                                              ; preds = %138
  %141 = getelementptr i8, ptr %128, i64 %98
  %142 = insertelement <2 x i64> poison, i64 %139, i64 0
  %143 = shufflevector <2 x i64> %142, <2 x i64> poison, <2 x i32> zeroinitializer
  br label %144

144:                                              ; preds = %144, %140
  %145 = phi i64 [ 0, %140 ], [ %151, %144 ]
  %146 = shl i64 %145, 3
  %147 = getelementptr i8, ptr %128, i64 %146
  %148 = getelementptr i8, ptr %147, i64 16
  %149 = getelementptr i8, ptr %147, i64 32
  %150 = getelementptr i8, ptr %147, i64 48
  store <2 x i64> %143, ptr %147, align 8, !noalias !7262
  store <2 x i64> %143, ptr %148, align 8, !noalias !7262
  store <2 x i64> %143, ptr %149, align 8, !noalias !7262
  store <2 x i64> %143, ptr %150, align 8, !noalias !7262
  %151 = add nuw i64 %145, 8
  %152 = icmp eq i64 %151, %97
  br i1 %152, label %153, label %144, !llvm.loop !7267

153:                                              ; preds = %144
  br i1 %99, label %136, label %154

154:                                              ; preds = %153, %138
  %155 = phi ptr [ %128, %138 ], [ %141, %153 ]
  br label %132

156:                                              ; preds = %123
  br i1 %131, label %183, label %157

157:                                              ; preds = %156
  br i1 %100, label %180, label %158

158:                                              ; preds = %157
  %159 = getelementptr i8, ptr %128, i64 %102
  br label %160

160:                                              ; preds = %160, %158
  %161 = phi i64 [ 0, %158 ], [ %176, %160 ]
  %162 = phi <2 x i64> [ <i64 0, i64 1>, %158 ], [ %177, %160 ]
  %163 = shl i64 %161, 3
  %164 = getelementptr i8, ptr %128, i64 %163
  %165 = add <2 x i64> %162, %104
  %166 = add <2 x i64> %162, %105
  %167 = add <2 x i64> %162, %106
  %168 = add <2 x i64> %162, %107
  %169 = uitofp <2 x i64> %165 to <2 x double>
  %170 = uitofp <2 x i64> %166 to <2 x double>
  %171 = uitofp <2 x i64> %167 to <2 x double>
  %172 = uitofp <2 x i64> %168 to <2 x double>
  %173 = getelementptr i8, ptr %164, i64 16
  %174 = getelementptr i8, ptr %164, i64 32
  %175 = getelementptr i8, ptr %164, i64 48
  store <2 x double> %169, ptr %164, align 8, !noalias !7262
  store <2 x double> %170, ptr %173, align 8, !noalias !7262
  store <2 x double> %171, ptr %174, align 8, !noalias !7262
  store <2 x double> %172, ptr %175, align 8, !noalias !7262
  %176 = add nuw i64 %161, 8
  %177 = add <2 x i64> %162, splat (i64 8)
  %178 = icmp eq i64 %176, %101
  br i1 %178, label %179, label %160, !llvm.loop !7268

179:                                              ; preds = %160
  br i1 %108, label %136, label %180

180:                                              ; preds = %179, %157
  %181 = phi ptr [ %128, %157 ], [ %159, %179 ]
  %182 = phi i64 [ 0, %157 ], [ %101, %179 ]
  br label %205

183:                                              ; preds = %156
  br i1 %109, label %202, label %184

184:                                              ; preds = %183
  %185 = getelementptr i8, ptr %128, i64 %111
  br label %186

186:                                              ; preds = %186, %184
  %187 = phi i64 [ 0, %184 ], [ %198, %186 ]
  %188 = phi <2 x i64> [ <i64 0, i64 1>, %184 ], [ %199, %186 ]
  %189 = shl i64 %187, 3
  %190 = getelementptr i8, ptr %128, i64 %189
  %191 = add <2 x i64> %188, %113
  %192 = add <2 x i64> %188, %114
  %193 = add <2 x i64> %188, %115
  %194 = add <2 x i64> %188, %116
  %195 = getelementptr i8, ptr %190, i64 16
  %196 = getelementptr i8, ptr %190, i64 32
  %197 = getelementptr i8, ptr %190, i64 48
  store <2 x i64> %191, ptr %190, align 8, !noalias !7262
  store <2 x i64> %192, ptr %195, align 8, !noalias !7262
  store <2 x i64> %193, ptr %196, align 8, !noalias !7262
  store <2 x i64> %194, ptr %197, align 8, !noalias !7262
  %198 = add nuw i64 %187, 8
  %199 = add <2 x i64> %188, splat (i64 8)
  %200 = icmp eq i64 %198, %110
  br i1 %200, label %201, label %186, !llvm.loop !7269

201:                                              ; preds = %186
  br i1 %117, label %136, label %202

202:                                              ; preds = %201, %183
  %203 = phi ptr [ %128, %183 ], [ %185, %201 ]
  %204 = phi i64 [ 0, %183 ], [ %110, %201 ]
  br label %213

205:                                              ; preds = %205, %180
  %206 = phi ptr [ %209, %205 ], [ %181, %180 ]
  %207 = phi i64 [ %208, %205 ], [ %182, %180 ]
  %208 = add nuw nsw i64 %207, 1
  %209 = getelementptr inbounds nuw i8, ptr %206, i64 8
  %210 = add i64 %207, %84
  %211 = uitofp i64 %210 to double
  store double %211, ptr %206, align 8, !noalias !7262
  %212 = icmp eq ptr %209, %129
  br i1 %212, label %136, label %205, !llvm.loop !7270

213:                                              ; preds = %213, %202
  %214 = phi ptr [ %217, %213 ], [ %203, %202 ]
  %215 = phi i64 [ %216, %213 ], [ %204, %202 ]
  %216 = add nuw nsw i64 %215, 1
  %217 = getelementptr inbounds nuw i8, ptr %214, i64 8
  %218 = add i64 %215, %84
  store i64 %218, ptr %214, align 8, !noalias !7262
  %219 = icmp eq ptr %217, %129
  br i1 %219, label %136, label %213, !llvm.loop !7271

220:                                              ; preds = %136, %75
  br i1 %37, label %1318, label %221

221:                                              ; preds = %220
  %222 = icmp ult i64 %78, 8
  %223 = and i64 %80, 56
  %224 = icmp eq i64 %223, 8
  %225 = icmp eq i64 %223, 16
  %226 = icmp eq i64 %223, 24
  %227 = icmp eq i64 %80, %223
  %228 = icmp ult i64 %78, 8
  %229 = and i64 %80, 56
  %230 = icmp eq i64 %229, 8
  %231 = icmp eq i64 %229, 16
  %232 = icmp eq i64 %229, 24
  %233 = icmp eq i64 %80, %229
  %234 = icmp ult i64 %78, 8
  %235 = and i64 %80, 56
  %236 = icmp eq i64 %235, 8
  %237 = icmp eq i64 %235, 16
  %238 = icmp eq i64 %235, 24
  %239 = icmp eq i64 %80, %235
  %240 = icmp ult i64 %78, 8
  %241 = and i64 %80, 56
  %242 = icmp eq i64 %241, 8
  %243 = icmp eq i64 %241, 16
  %244 = icmp eq i64 %241, 24
  %245 = icmp eq i64 %80, %241
  %246 = icmp ult i64 %78, 8
  %247 = and i64 %80, 56
  %248 = icmp eq i64 %247, 8
  %249 = icmp eq i64 %247, 16
  %250 = icmp eq i64 %247, 24
  %251 = icmp eq i64 %80, %247
  %252 = icmp ult i64 %78, 8
  %253 = and i64 %80, 56
  %254 = icmp eq i64 %253, 8
  %255 = icmp eq i64 %253, 16
  %256 = icmp eq i64 %253, 24
  %257 = icmp eq i64 %80, %253
  %258 = icmp ult i64 %78, 8
  %259 = and i64 %80, 56
  %260 = icmp eq i64 %259, 8
  %261 = icmp eq i64 %259, 16
  %262 = icmp eq i64 %259, 24
  %263 = icmp eq i64 %80, %259
  %264 = icmp ult i64 %78, 8
  %265 = and i64 %80, 56
  %266 = icmp eq i64 %265, 8
  %267 = icmp eq i64 %265, 16
  %268 = icmp eq i64 %265, 24
  %269 = icmp eq i64 %80, %265
  %270 = and i64 %80, 1
  %271 = icmp eq i64 %78, 1
  %272 = and i64 %80, 62
  %273 = icmp eq i64 %270, 0
  %274 = icmp ne i64 %270, 0
  %275 = icmp ult i64 %78, 8
  %276 = and i64 %80, 56
  %277 = icmp eq i64 %276, 8
  %278 = icmp eq i64 %276, 16
  %279 = icmp eq i64 %276, 24
  %280 = icmp eq i64 %80, %276
  br label %281

281:                                              ; preds = %1223, %221
  %282 = phi ptr [ %283, %1223 ], [ %32, %221 ]
  %283 = getelementptr inbounds nuw i8, ptr %282, i64 32
  %284 = load i32, ptr %282, align 8, !range !7272, !noalias !7262, !noundef !4
  %285 = getelementptr inbounds nuw i8, ptr %282, i64 4
  %286 = load i32, ptr %285, align 4, !noalias !7262
  %287 = getelementptr inbounds nuw i8, ptr %282, i64 8
  %288 = load i64, ptr %287, align 8, !noalias !7262, !noundef !4
  %289 = getelementptr inbounds nuw i8, ptr %282, i64 16
  %290 = load i64, ptr %289, align 8, !noalias !7262, !noundef !4
  %291 = getelementptr inbounds nuw i8, ptr %282, i64 24
  %292 = load i64, ptr %291, align 8, !noalias !7262, !noundef !4
  %293 = icmp ugt i64 %292, %24
  br i1 %293, label %1312, label %294, !prof !11

294:                                              ; preds = %281
  %295 = getelementptr inbounds nuw [32 x i64], ptr %26, i64 %292
  %296 = icmp eq i64 %24, %292
  br i1 %296, label %1313, label %297

297:                                              ; preds = %294
  %298 = icmp ult i64 %288, %292
  br i1 %298, label %299, label %1314

299:                                              ; preds = %297
  %300 = getelementptr inbounds nuw [32 x i64], ptr %26, i64 %288
  %301 = icmp ult i64 %290, %292
  br i1 %301, label %302, label %1315

302:                                              ; preds = %299
  %303 = getelementptr inbounds nuw [32 x i64], ptr %26, i64 %290
  switch i32 %284, label %1316 [
    i32 0, label %1033
    i32 1, label %304
    i32 2, label %1183
    i32 3, label %1233
    i32 4, label %924
    i32 5, label %815
    i32 6, label %706
    i32 7, label %597
    i32 8, label %488
    i32 9, label %379
    i32 10, label %305
  ]

304:                                              ; preds = %302
  br i1 %271, label %1215, label %1195

305:                                              ; preds = %302
  br i1 %222, label %377, label %306

306:                                              ; preds = %305
  %307 = sub i64 %292, %288
  %308 = and i64 %307, 72057594037927935
  %309 = icmp eq i64 %308, 0
  br i1 %309, label %377, label %310

310:                                              ; preds = %306
  %311 = getelementptr inbounds nuw i8, ptr %300, i64 16
  %312 = getelementptr inbounds nuw i8, ptr %300, i64 32
  %313 = getelementptr inbounds nuw i8, ptr %300, i64 48
  %314 = load <2 x double>, ptr %300, align 8, !noalias !7262
  %315 = load <2 x double>, ptr %311, align 8, !noalias !7262
  %316 = load <2 x double>, ptr %312, align 8, !noalias !7262
  %317 = load <2 x double>, ptr %313, align 8, !noalias !7262
  %318 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %314)
  %319 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %315)
  %320 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %316)
  %321 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %317)
  %322 = getelementptr inbounds nuw i8, ptr %295, i64 16
  %323 = getelementptr inbounds nuw i8, ptr %295, i64 32
  %324 = getelementptr inbounds nuw i8, ptr %295, i64 48
  store <2 x double> %318, ptr %295, align 8, !noalias !7262
  store <2 x double> %319, ptr %322, align 8, !noalias !7262
  store <2 x double> %320, ptr %323, align 8, !noalias !7262
  store <2 x double> %321, ptr %324, align 8, !noalias !7262
  br i1 %224, label %376, label %325, !llvm.loop !7273

325:                                              ; preds = %310
  %326 = getelementptr inbounds nuw i8, ptr %295, i64 64
  %327 = getelementptr inbounds nuw i8, ptr %300, i64 64
  %328 = getelementptr inbounds nuw i8, ptr %300, i64 80
  %329 = getelementptr inbounds nuw i8, ptr %300, i64 96
  %330 = getelementptr inbounds nuw i8, ptr %300, i64 112
  %331 = load <2 x double>, ptr %327, align 8, !noalias !7262
  %332 = load <2 x double>, ptr %328, align 8, !noalias !7262
  %333 = load <2 x double>, ptr %329, align 8, !noalias !7262
  %334 = load <2 x double>, ptr %330, align 8, !noalias !7262
  %335 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %331)
  %336 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %332)
  %337 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %333)
  %338 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %334)
  %339 = getelementptr inbounds nuw i8, ptr %295, i64 80
  %340 = getelementptr inbounds nuw i8, ptr %295, i64 96
  %341 = getelementptr inbounds nuw i8, ptr %295, i64 112
  store <2 x double> %335, ptr %326, align 8, !noalias !7262
  store <2 x double> %336, ptr %339, align 8, !noalias !7262
  store <2 x double> %337, ptr %340, align 8, !noalias !7262
  store <2 x double> %338, ptr %341, align 8, !noalias !7262
  br i1 %225, label %376, label %342, !llvm.loop !7273

342:                                              ; preds = %325
  %343 = getelementptr inbounds nuw i8, ptr %295, i64 128
  %344 = getelementptr inbounds nuw i8, ptr %300, i64 128
  %345 = getelementptr inbounds nuw i8, ptr %300, i64 144
  %346 = getelementptr inbounds nuw i8, ptr %300, i64 160
  %347 = getelementptr inbounds nuw i8, ptr %300, i64 176
  %348 = load <2 x double>, ptr %344, align 8, !noalias !7262
  %349 = load <2 x double>, ptr %345, align 8, !noalias !7262
  %350 = load <2 x double>, ptr %346, align 8, !noalias !7262
  %351 = load <2 x double>, ptr %347, align 8, !noalias !7262
  %352 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %348)
  %353 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %349)
  %354 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %350)
  %355 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %351)
  %356 = getelementptr inbounds nuw i8, ptr %295, i64 144
  %357 = getelementptr inbounds nuw i8, ptr %295, i64 160
  %358 = getelementptr inbounds nuw i8, ptr %295, i64 176
  store <2 x double> %352, ptr %343, align 8, !noalias !7262
  store <2 x double> %353, ptr %356, align 8, !noalias !7262
  store <2 x double> %354, ptr %357, align 8, !noalias !7262
  store <2 x double> %355, ptr %358, align 8, !noalias !7262
  br i1 %226, label %376, label %359, !llvm.loop !7273

359:                                              ; preds = %342
  %360 = getelementptr inbounds nuw i8, ptr %295, i64 192
  %361 = getelementptr inbounds nuw i8, ptr %300, i64 192
  %362 = getelementptr inbounds nuw i8, ptr %300, i64 208
  %363 = getelementptr inbounds nuw i8, ptr %300, i64 224
  %364 = getelementptr inbounds nuw i8, ptr %300, i64 240
  %365 = load <2 x double>, ptr %361, align 8, !noalias !7262
  %366 = load <2 x double>, ptr %362, align 8, !noalias !7262
  %367 = load <2 x double>, ptr %363, align 8, !noalias !7262
  %368 = load <2 x double>, ptr %364, align 8, !noalias !7262
  %369 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %365)
  %370 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %366)
  %371 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %367)
  %372 = tail call <2 x double> @llvm.sqrt.v2f64(<2 x double> %368)
  %373 = getelementptr inbounds nuw i8, ptr %295, i64 208
  %374 = getelementptr inbounds nuw i8, ptr %295, i64 224
  %375 = getelementptr inbounds nuw i8, ptr %295, i64 240
  store <2 x double> %369, ptr %360, align 8, !noalias !7262
  store <2 x double> %370, ptr %373, align 8, !noalias !7262
  store <2 x double> %371, ptr %374, align 8, !noalias !7262
  store <2 x double> %372, ptr %375, align 8, !noalias !7262
  br label %376

376:                                              ; preds = %359, %342, %325, %310
  br i1 %227, label %1223, label %377

377:                                              ; preds = %376, %306, %305
  %378 = phi i64 [ 0, %306 ], [ 0, %305 ], [ %223, %376 ]
  br label %1107

379:                                              ; preds = %302
  br i1 %228, label %486, label %380

380:                                              ; preds = %379
  %381 = sub i64 %292, %288
  %382 = and i64 %381, 72057594037927935
  %383 = icmp eq i64 %382, 0
  %384 = sub i64 %292, %290
  %385 = and i64 %384, 72057594037927935
  %386 = icmp eq i64 %385, 0
  %387 = or i1 %383, %386
  br i1 %387, label %486, label %388

388:                                              ; preds = %380
  %389 = getelementptr inbounds nuw i8, ptr %300, i64 16
  %390 = getelementptr inbounds nuw i8, ptr %300, i64 32
  %391 = getelementptr inbounds nuw i8, ptr %300, i64 48
  %392 = load <2 x double>, ptr %300, align 8, !noalias !7262
  %393 = load <2 x double>, ptr %389, align 8, !noalias !7262
  %394 = load <2 x double>, ptr %390, align 8, !noalias !7262
  %395 = load <2 x double>, ptr %391, align 8, !noalias !7262
  %396 = getelementptr inbounds nuw i8, ptr %303, i64 16
  %397 = getelementptr inbounds nuw i8, ptr %303, i64 32
  %398 = getelementptr inbounds nuw i8, ptr %303, i64 48
  %399 = load <2 x double>, ptr %303, align 8, !noalias !7262
  %400 = load <2 x double>, ptr %396, align 8, !noalias !7262
  %401 = load <2 x double>, ptr %397, align 8, !noalias !7262
  %402 = load <2 x double>, ptr %398, align 8, !noalias !7262
  %403 = fsub <2 x double> %392, %399
  %404 = fsub <2 x double> %393, %400
  %405 = fsub <2 x double> %394, %401
  %406 = fsub <2 x double> %395, %402
  %407 = getelementptr inbounds nuw i8, ptr %295, i64 16
  %408 = getelementptr inbounds nuw i8, ptr %295, i64 32
  %409 = getelementptr inbounds nuw i8, ptr %295, i64 48
  store <2 x double> %403, ptr %295, align 8, !noalias !7262
  store <2 x double> %404, ptr %407, align 8, !noalias !7262
  store <2 x double> %405, ptr %408, align 8, !noalias !7262
  store <2 x double> %406, ptr %409, align 8, !noalias !7262
  br i1 %230, label %485, label %410, !llvm.loop !7274

410:                                              ; preds = %388
  %411 = getelementptr inbounds nuw i8, ptr %295, i64 64
  %412 = getelementptr inbounds nuw i8, ptr %300, i64 64
  %413 = getelementptr inbounds nuw i8, ptr %303, i64 64
  %414 = getelementptr inbounds nuw i8, ptr %300, i64 80
  %415 = getelementptr inbounds nuw i8, ptr %300, i64 96
  %416 = getelementptr inbounds nuw i8, ptr %300, i64 112
  %417 = load <2 x double>, ptr %412, align 8, !noalias !7262
  %418 = load <2 x double>, ptr %414, align 8, !noalias !7262
  %419 = load <2 x double>, ptr %415, align 8, !noalias !7262
  %420 = load <2 x double>, ptr %416, align 8, !noalias !7262
  %421 = getelementptr inbounds nuw i8, ptr %303, i64 80
  %422 = getelementptr inbounds nuw i8, ptr %303, i64 96
  %423 = getelementptr inbounds nuw i8, ptr %303, i64 112
  %424 = load <2 x double>, ptr %413, align 8, !noalias !7262
  %425 = load <2 x double>, ptr %421, align 8, !noalias !7262
  %426 = load <2 x double>, ptr %422, align 8, !noalias !7262
  %427 = load <2 x double>, ptr %423, align 8, !noalias !7262
  %428 = fsub <2 x double> %417, %424
  %429 = fsub <2 x double> %418, %425
  %430 = fsub <2 x double> %419, %426
  %431 = fsub <2 x double> %420, %427
  %432 = getelementptr inbounds nuw i8, ptr %295, i64 80
  %433 = getelementptr inbounds nuw i8, ptr %295, i64 96
  %434 = getelementptr inbounds nuw i8, ptr %295, i64 112
  store <2 x double> %428, ptr %411, align 8, !noalias !7262
  store <2 x double> %429, ptr %432, align 8, !noalias !7262
  store <2 x double> %430, ptr %433, align 8, !noalias !7262
  store <2 x double> %431, ptr %434, align 8, !noalias !7262
  br i1 %231, label %485, label %435, !llvm.loop !7274

435:                                              ; preds = %410
  %436 = getelementptr inbounds nuw i8, ptr %295, i64 128
  %437 = getelementptr inbounds nuw i8, ptr %300, i64 128
  %438 = getelementptr inbounds nuw i8, ptr %303, i64 128
  %439 = getelementptr inbounds nuw i8, ptr %300, i64 144
  %440 = getelementptr inbounds nuw i8, ptr %300, i64 160
  %441 = getelementptr inbounds nuw i8, ptr %300, i64 176
  %442 = load <2 x double>, ptr %437, align 8, !noalias !7262
  %443 = load <2 x double>, ptr %439, align 8, !noalias !7262
  %444 = load <2 x double>, ptr %440, align 8, !noalias !7262
  %445 = load <2 x double>, ptr %441, align 8, !noalias !7262
  %446 = getelementptr inbounds nuw i8, ptr %303, i64 144
  %447 = getelementptr inbounds nuw i8, ptr %303, i64 160
  %448 = getelementptr inbounds nuw i8, ptr %303, i64 176
  %449 = load <2 x double>, ptr %438, align 8, !noalias !7262
  %450 = load <2 x double>, ptr %446, align 8, !noalias !7262
  %451 = load <2 x double>, ptr %447, align 8, !noalias !7262
  %452 = load <2 x double>, ptr %448, align 8, !noalias !7262
  %453 = fsub <2 x double> %442, %449
  %454 = fsub <2 x double> %443, %450
  %455 = fsub <2 x double> %444, %451
  %456 = fsub <2 x double> %445, %452
  %457 = getelementptr inbounds nuw i8, ptr %295, i64 144
  %458 = getelementptr inbounds nuw i8, ptr %295, i64 160
  %459 = getelementptr inbounds nuw i8, ptr %295, i64 176
  store <2 x double> %453, ptr %436, align 8, !noalias !7262
  store <2 x double> %454, ptr %457, align 8, !noalias !7262
  store <2 x double> %455, ptr %458, align 8, !noalias !7262
  store <2 x double> %456, ptr %459, align 8, !noalias !7262
  br i1 %232, label %485, label %460, !llvm.loop !7274

460:                                              ; preds = %435
  %461 = getelementptr inbounds nuw i8, ptr %295, i64 192
  %462 = getelementptr inbounds nuw i8, ptr %300, i64 192
  %463 = getelementptr inbounds nuw i8, ptr %303, i64 192
  %464 = getelementptr inbounds nuw i8, ptr %300, i64 208
  %465 = getelementptr inbounds nuw i8, ptr %300, i64 224
  %466 = getelementptr inbounds nuw i8, ptr %300, i64 240
  %467 = load <2 x double>, ptr %462, align 8, !noalias !7262
  %468 = load <2 x double>, ptr %464, align 8, !noalias !7262
  %469 = load <2 x double>, ptr %465, align 8, !noalias !7262
  %470 = load <2 x double>, ptr %466, align 8, !noalias !7262
  %471 = getelementptr inbounds nuw i8, ptr %303, i64 208
  %472 = getelementptr inbounds nuw i8, ptr %303, i64 224
  %473 = getelementptr inbounds nuw i8, ptr %303, i64 240
  %474 = load <2 x double>, ptr %463, align 8, !noalias !7262
  %475 = load <2 x double>, ptr %471, align 8, !noalias !7262
  %476 = load <2 x double>, ptr %472, align 8, !noalias !7262
  %477 = load <2 x double>, ptr %473, align 8, !noalias !7262
  %478 = fsub <2 x double> %467, %474
  %479 = fsub <2 x double> %468, %475
  %480 = fsub <2 x double> %469, %476
  %481 = fsub <2 x double> %470, %477
  %482 = getelementptr inbounds nuw i8, ptr %295, i64 208
  %483 = getelementptr inbounds nuw i8, ptr %295, i64 224
  %484 = getelementptr inbounds nuw i8, ptr %295, i64 240
  store <2 x double> %478, ptr %461, align 8, !noalias !7262
  store <2 x double> %479, ptr %482, align 8, !noalias !7262
  store <2 x double> %480, ptr %483, align 8, !noalias !7262
  store <2 x double> %481, ptr %484, align 8, !noalias !7262
  br label %485

485:                                              ; preds = %460, %435, %410, %388
  br i1 %233, label %1223, label %486

486:                                              ; preds = %485, %380, %379
  %487 = phi i64 [ 0, %380 ], [ 0, %379 ], [ %229, %485 ]
  br label %1115

488:                                              ; preds = %302
  br i1 %234, label %595, label %489

489:                                              ; preds = %488
  %490 = sub i64 %292, %288
  %491 = and i64 %490, 72057594037927935
  %492 = icmp eq i64 %491, 0
  %493 = sub i64 %292, %290
  %494 = and i64 %493, 72057594037927935
  %495 = icmp eq i64 %494, 0
  %496 = or i1 %492, %495
  br i1 %496, label %595, label %497

497:                                              ; preds = %489
  %498 = getelementptr inbounds nuw i8, ptr %300, i64 16
  %499 = getelementptr inbounds nuw i8, ptr %300, i64 32
  %500 = getelementptr inbounds nuw i8, ptr %300, i64 48
  %501 = load <2 x double>, ptr %300, align 8, !noalias !7262
  %502 = load <2 x double>, ptr %498, align 8, !noalias !7262
  %503 = load <2 x double>, ptr %499, align 8, !noalias !7262
  %504 = load <2 x double>, ptr %500, align 8, !noalias !7262
  %505 = getelementptr inbounds nuw i8, ptr %303, i64 16
  %506 = getelementptr inbounds nuw i8, ptr %303, i64 32
  %507 = getelementptr inbounds nuw i8, ptr %303, i64 48
  %508 = load <2 x double>, ptr %303, align 8, !noalias !7262
  %509 = load <2 x double>, ptr %505, align 8, !noalias !7262
  %510 = load <2 x double>, ptr %506, align 8, !noalias !7262
  %511 = load <2 x double>, ptr %507, align 8, !noalias !7262
  %512 = fadd <2 x double> %501, %508
  %513 = fadd <2 x double> %502, %509
  %514 = fadd <2 x double> %503, %510
  %515 = fadd <2 x double> %504, %511
  %516 = getelementptr inbounds nuw i8, ptr %295, i64 16
  %517 = getelementptr inbounds nuw i8, ptr %295, i64 32
  %518 = getelementptr inbounds nuw i8, ptr %295, i64 48
  store <2 x double> %512, ptr %295, align 8, !noalias !7262
  store <2 x double> %513, ptr %516, align 8, !noalias !7262
  store <2 x double> %514, ptr %517, align 8, !noalias !7262
  store <2 x double> %515, ptr %518, align 8, !noalias !7262
  br i1 %236, label %594, label %519, !llvm.loop !7275

519:                                              ; preds = %497
  %520 = getelementptr inbounds nuw i8, ptr %295, i64 64
  %521 = getelementptr inbounds nuw i8, ptr %300, i64 64
  %522 = getelementptr inbounds nuw i8, ptr %303, i64 64
  %523 = getelementptr inbounds nuw i8, ptr %300, i64 80
  %524 = getelementptr inbounds nuw i8, ptr %300, i64 96
  %525 = getelementptr inbounds nuw i8, ptr %300, i64 112
  %526 = load <2 x double>, ptr %521, align 8, !noalias !7262
  %527 = load <2 x double>, ptr %523, align 8, !noalias !7262
  %528 = load <2 x double>, ptr %524, align 8, !noalias !7262
  %529 = load <2 x double>, ptr %525, align 8, !noalias !7262
  %530 = getelementptr inbounds nuw i8, ptr %303, i64 80
  %531 = getelementptr inbounds nuw i8, ptr %303, i64 96
  %532 = getelementptr inbounds nuw i8, ptr %303, i64 112
  %533 = load <2 x double>, ptr %522, align 8, !noalias !7262
  %534 = load <2 x double>, ptr %530, align 8, !noalias !7262
  %535 = load <2 x double>, ptr %531, align 8, !noalias !7262
  %536 = load <2 x double>, ptr %532, align 8, !noalias !7262
  %537 = fadd <2 x double> %526, %533
  %538 = fadd <2 x double> %527, %534
  %539 = fadd <2 x double> %528, %535
  %540 = fadd <2 x double> %529, %536
  %541 = getelementptr inbounds nuw i8, ptr %295, i64 80
  %542 = getelementptr inbounds nuw i8, ptr %295, i64 96
  %543 = getelementptr inbounds nuw i8, ptr %295, i64 112
  store <2 x double> %537, ptr %520, align 8, !noalias !7262
  store <2 x double> %538, ptr %541, align 8, !noalias !7262
  store <2 x double> %539, ptr %542, align 8, !noalias !7262
  store <2 x double> %540, ptr %543, align 8, !noalias !7262
  br i1 %237, label %594, label %544, !llvm.loop !7275

544:                                              ; preds = %519
  %545 = getelementptr inbounds nuw i8, ptr %295, i64 128
  %546 = getelementptr inbounds nuw i8, ptr %300, i64 128
  %547 = getelementptr inbounds nuw i8, ptr %303, i64 128
  %548 = getelementptr inbounds nuw i8, ptr %300, i64 144
  %549 = getelementptr inbounds nuw i8, ptr %300, i64 160
  %550 = getelementptr inbounds nuw i8, ptr %300, i64 176
  %551 = load <2 x double>, ptr %546, align 8, !noalias !7262
  %552 = load <2 x double>, ptr %548, align 8, !noalias !7262
  %553 = load <2 x double>, ptr %549, align 8, !noalias !7262
  %554 = load <2 x double>, ptr %550, align 8, !noalias !7262
  %555 = getelementptr inbounds nuw i8, ptr %303, i64 144
  %556 = getelementptr inbounds nuw i8, ptr %303, i64 160
  %557 = getelementptr inbounds nuw i8, ptr %303, i64 176
  %558 = load <2 x double>, ptr %547, align 8, !noalias !7262
  %559 = load <2 x double>, ptr %555, align 8, !noalias !7262
  %560 = load <2 x double>, ptr %556, align 8, !noalias !7262
  %561 = load <2 x double>, ptr %557, align 8, !noalias !7262
  %562 = fadd <2 x double> %551, %558
  %563 = fadd <2 x double> %552, %559
  %564 = fadd <2 x double> %553, %560
  %565 = fadd <2 x double> %554, %561
  %566 = getelementptr inbounds nuw i8, ptr %295, i64 144
  %567 = getelementptr inbounds nuw i8, ptr %295, i64 160
  %568 = getelementptr inbounds nuw i8, ptr %295, i64 176
  store <2 x double> %562, ptr %545, align 8, !noalias !7262
  store <2 x double> %563, ptr %566, align 8, !noalias !7262
  store <2 x double> %564, ptr %567, align 8, !noalias !7262
  store <2 x double> %565, ptr %568, align 8, !noalias !7262
  br i1 %238, label %594, label %569, !llvm.loop !7275

569:                                              ; preds = %544
  %570 = getelementptr inbounds nuw i8, ptr %295, i64 192
  %571 = getelementptr inbounds nuw i8, ptr %300, i64 192
  %572 = getelementptr inbounds nuw i8, ptr %303, i64 192
  %573 = getelementptr inbounds nuw i8, ptr %300, i64 208
  %574 = getelementptr inbounds nuw i8, ptr %300, i64 224
  %575 = getelementptr inbounds nuw i8, ptr %300, i64 240
  %576 = load <2 x double>, ptr %571, align 8, !noalias !7262
  %577 = load <2 x double>, ptr %573, align 8, !noalias !7262
  %578 = load <2 x double>, ptr %574, align 8, !noalias !7262
  %579 = load <2 x double>, ptr %575, align 8, !noalias !7262
  %580 = getelementptr inbounds nuw i8, ptr %303, i64 208
  %581 = getelementptr inbounds nuw i8, ptr %303, i64 224
  %582 = getelementptr inbounds nuw i8, ptr %303, i64 240
  %583 = load <2 x double>, ptr %572, align 8, !noalias !7262
  %584 = load <2 x double>, ptr %580, align 8, !noalias !7262
  %585 = load <2 x double>, ptr %581, align 8, !noalias !7262
  %586 = load <2 x double>, ptr %582, align 8, !noalias !7262
  %587 = fadd <2 x double> %576, %583
  %588 = fadd <2 x double> %577, %584
  %589 = fadd <2 x double> %578, %585
  %590 = fadd <2 x double> %579, %586
  %591 = getelementptr inbounds nuw i8, ptr %295, i64 208
  %592 = getelementptr inbounds nuw i8, ptr %295, i64 224
  %593 = getelementptr inbounds nuw i8, ptr %295, i64 240
  store <2 x double> %587, ptr %570, align 8, !noalias !7262
  store <2 x double> %588, ptr %591, align 8, !noalias !7262
  store <2 x double> %589, ptr %592, align 8, !noalias !7262
  store <2 x double> %590, ptr %593, align 8, !noalias !7262
  br label %594

594:                                              ; preds = %569, %544, %519, %497
  br i1 %239, label %1223, label %595

595:                                              ; preds = %594, %489, %488
  %596 = phi i64 [ 0, %489 ], [ 0, %488 ], [ %235, %594 ]
  br label %1125

597:                                              ; preds = %302
  br i1 %240, label %704, label %598

598:                                              ; preds = %597
  %599 = sub i64 %292, %288
  %600 = and i64 %599, 72057594037927935
  %601 = icmp eq i64 %600, 0
  %602 = sub i64 %292, %290
  %603 = and i64 %602, 72057594037927935
  %604 = icmp eq i64 %603, 0
  %605 = or i1 %601, %604
  br i1 %605, label %704, label %606

606:                                              ; preds = %598
  %607 = getelementptr inbounds nuw i8, ptr %300, i64 16
  %608 = getelementptr inbounds nuw i8, ptr %300, i64 32
  %609 = getelementptr inbounds nuw i8, ptr %300, i64 48
  %610 = load <2 x double>, ptr %300, align 8, !noalias !7262
  %611 = load <2 x double>, ptr %607, align 8, !noalias !7262
  %612 = load <2 x double>, ptr %608, align 8, !noalias !7262
  %613 = load <2 x double>, ptr %609, align 8, !noalias !7262
  %614 = getelementptr inbounds nuw i8, ptr %303, i64 16
  %615 = getelementptr inbounds nuw i8, ptr %303, i64 32
  %616 = getelementptr inbounds nuw i8, ptr %303, i64 48
  %617 = load <2 x double>, ptr %303, align 8, !noalias !7262
  %618 = load <2 x double>, ptr %614, align 8, !noalias !7262
  %619 = load <2 x double>, ptr %615, align 8, !noalias !7262
  %620 = load <2 x double>, ptr %616, align 8, !noalias !7262
  %621 = fdiv <2 x double> %610, %617
  %622 = fdiv <2 x double> %611, %618
  %623 = fdiv <2 x double> %612, %619
  %624 = fdiv <2 x double> %613, %620
  %625 = getelementptr inbounds nuw i8, ptr %295, i64 16
  %626 = getelementptr inbounds nuw i8, ptr %295, i64 32
  %627 = getelementptr inbounds nuw i8, ptr %295, i64 48
  store <2 x double> %621, ptr %295, align 8, !noalias !7262
  store <2 x double> %622, ptr %625, align 8, !noalias !7262
  store <2 x double> %623, ptr %626, align 8, !noalias !7262
  store <2 x double> %624, ptr %627, align 8, !noalias !7262
  br i1 %242, label %703, label %628, !llvm.loop !7276

628:                                              ; preds = %606
  %629 = getelementptr inbounds nuw i8, ptr %295, i64 64
  %630 = getelementptr inbounds nuw i8, ptr %300, i64 64
  %631 = getelementptr inbounds nuw i8, ptr %303, i64 64
  %632 = getelementptr inbounds nuw i8, ptr %300, i64 80
  %633 = getelementptr inbounds nuw i8, ptr %300, i64 96
  %634 = getelementptr inbounds nuw i8, ptr %300, i64 112
  %635 = load <2 x double>, ptr %630, align 8, !noalias !7262
  %636 = load <2 x double>, ptr %632, align 8, !noalias !7262
  %637 = load <2 x double>, ptr %633, align 8, !noalias !7262
  %638 = load <2 x double>, ptr %634, align 8, !noalias !7262
  %639 = getelementptr inbounds nuw i8, ptr %303, i64 80
  %640 = getelementptr inbounds nuw i8, ptr %303, i64 96
  %641 = getelementptr inbounds nuw i8, ptr %303, i64 112
  %642 = load <2 x double>, ptr %631, align 8, !noalias !7262
  %643 = load <2 x double>, ptr %639, align 8, !noalias !7262
  %644 = load <2 x double>, ptr %640, align 8, !noalias !7262
  %645 = load <2 x double>, ptr %641, align 8, !noalias !7262
  %646 = fdiv <2 x double> %635, %642
  %647 = fdiv <2 x double> %636, %643
  %648 = fdiv <2 x double> %637, %644
  %649 = fdiv <2 x double> %638, %645
  %650 = getelementptr inbounds nuw i8, ptr %295, i64 80
  %651 = getelementptr inbounds nuw i8, ptr %295, i64 96
  %652 = getelementptr inbounds nuw i8, ptr %295, i64 112
  store <2 x double> %646, ptr %629, align 8, !noalias !7262
  store <2 x double> %647, ptr %650, align 8, !noalias !7262
  store <2 x double> %648, ptr %651, align 8, !noalias !7262
  store <2 x double> %649, ptr %652, align 8, !noalias !7262
  br i1 %243, label %703, label %653, !llvm.loop !7276

653:                                              ; preds = %628
  %654 = getelementptr inbounds nuw i8, ptr %295, i64 128
  %655 = getelementptr inbounds nuw i8, ptr %300, i64 128
  %656 = getelementptr inbounds nuw i8, ptr %303, i64 128
  %657 = getelementptr inbounds nuw i8, ptr %300, i64 144
  %658 = getelementptr inbounds nuw i8, ptr %300, i64 160
  %659 = getelementptr inbounds nuw i8, ptr %300, i64 176
  %660 = load <2 x double>, ptr %655, align 8, !noalias !7262
  %661 = load <2 x double>, ptr %657, align 8, !noalias !7262
  %662 = load <2 x double>, ptr %658, align 8, !noalias !7262
  %663 = load <2 x double>, ptr %659, align 8, !noalias !7262
  %664 = getelementptr inbounds nuw i8, ptr %303, i64 144
  %665 = getelementptr inbounds nuw i8, ptr %303, i64 160
  %666 = getelementptr inbounds nuw i8, ptr %303, i64 176
  %667 = load <2 x double>, ptr %656, align 8, !noalias !7262
  %668 = load <2 x double>, ptr %664, align 8, !noalias !7262
  %669 = load <2 x double>, ptr %665, align 8, !noalias !7262
  %670 = load <2 x double>, ptr %666, align 8, !noalias !7262
  %671 = fdiv <2 x double> %660, %667
  %672 = fdiv <2 x double> %661, %668
  %673 = fdiv <2 x double> %662, %669
  %674 = fdiv <2 x double> %663, %670
  %675 = getelementptr inbounds nuw i8, ptr %295, i64 144
  %676 = getelementptr inbounds nuw i8, ptr %295, i64 160
  %677 = getelementptr inbounds nuw i8, ptr %295, i64 176
  store <2 x double> %671, ptr %654, align 8, !noalias !7262
  store <2 x double> %672, ptr %675, align 8, !noalias !7262
  store <2 x double> %673, ptr %676, align 8, !noalias !7262
  store <2 x double> %674, ptr %677, align 8, !noalias !7262
  br i1 %244, label %703, label %678, !llvm.loop !7276

678:                                              ; preds = %653
  %679 = getelementptr inbounds nuw i8, ptr %295, i64 192
  %680 = getelementptr inbounds nuw i8, ptr %300, i64 192
  %681 = getelementptr inbounds nuw i8, ptr %303, i64 192
  %682 = getelementptr inbounds nuw i8, ptr %300, i64 208
  %683 = getelementptr inbounds nuw i8, ptr %300, i64 224
  %684 = getelementptr inbounds nuw i8, ptr %300, i64 240
  %685 = load <2 x double>, ptr %680, align 8, !noalias !7262
  %686 = load <2 x double>, ptr %682, align 8, !noalias !7262
  %687 = load <2 x double>, ptr %683, align 8, !noalias !7262
  %688 = load <2 x double>, ptr %684, align 8, !noalias !7262
  %689 = getelementptr inbounds nuw i8, ptr %303, i64 208
  %690 = getelementptr inbounds nuw i8, ptr %303, i64 224
  %691 = getelementptr inbounds nuw i8, ptr %303, i64 240
  %692 = load <2 x double>, ptr %681, align 8, !noalias !7262
  %693 = load <2 x double>, ptr %689, align 8, !noalias !7262
  %694 = load <2 x double>, ptr %690, align 8, !noalias !7262
  %695 = load <2 x double>, ptr %691, align 8, !noalias !7262
  %696 = fdiv <2 x double> %685, %692
  %697 = fdiv <2 x double> %686, %693
  %698 = fdiv <2 x double> %687, %694
  %699 = fdiv <2 x double> %688, %695
  %700 = getelementptr inbounds nuw i8, ptr %295, i64 208
  %701 = getelementptr inbounds nuw i8, ptr %295, i64 224
  %702 = getelementptr inbounds nuw i8, ptr %295, i64 240
  store <2 x double> %696, ptr %679, align 8, !noalias !7262
  store <2 x double> %697, ptr %700, align 8, !noalias !7262
  store <2 x double> %698, ptr %701, align 8, !noalias !7262
  store <2 x double> %699, ptr %702, align 8, !noalias !7262
  br label %703

703:                                              ; preds = %678, %653, %628, %606
  br i1 %245, label %1223, label %704

704:                                              ; preds = %703, %598, %597
  %705 = phi i64 [ 0, %598 ], [ 0, %597 ], [ %241, %703 ]
  br label %1135

706:                                              ; preds = %302
  br i1 %246, label %813, label %707

707:                                              ; preds = %706
  %708 = sub i64 %292, %288
  %709 = and i64 %708, 72057594037927935
  %710 = icmp eq i64 %709, 0
  %711 = sub i64 %292, %290
  %712 = and i64 %711, 72057594037927935
  %713 = icmp eq i64 %712, 0
  %714 = or i1 %710, %713
  br i1 %714, label %813, label %715

715:                                              ; preds = %707
  %716 = getelementptr inbounds nuw i8, ptr %300, i64 16
  %717 = getelementptr inbounds nuw i8, ptr %300, i64 32
  %718 = getelementptr inbounds nuw i8, ptr %300, i64 48
  %719 = load <2 x double>, ptr %300, align 8, !noalias !7262
  %720 = load <2 x double>, ptr %716, align 8, !noalias !7262
  %721 = load <2 x double>, ptr %717, align 8, !noalias !7262
  %722 = load <2 x double>, ptr %718, align 8, !noalias !7262
  %723 = getelementptr inbounds nuw i8, ptr %303, i64 16
  %724 = getelementptr inbounds nuw i8, ptr %303, i64 32
  %725 = getelementptr inbounds nuw i8, ptr %303, i64 48
  %726 = load <2 x double>, ptr %303, align 8, !noalias !7262
  %727 = load <2 x double>, ptr %723, align 8, !noalias !7262
  %728 = load <2 x double>, ptr %724, align 8, !noalias !7262
  %729 = load <2 x double>, ptr %725, align 8, !noalias !7262
  %730 = fmul <2 x double> %719, %726
  %731 = fmul <2 x double> %720, %727
  %732 = fmul <2 x double> %721, %728
  %733 = fmul <2 x double> %722, %729
  %734 = getelementptr inbounds nuw i8, ptr %295, i64 16
  %735 = getelementptr inbounds nuw i8, ptr %295, i64 32
  %736 = getelementptr inbounds nuw i8, ptr %295, i64 48
  store <2 x double> %730, ptr %295, align 8, !noalias !7262
  store <2 x double> %731, ptr %734, align 8, !noalias !7262
  store <2 x double> %732, ptr %735, align 8, !noalias !7262
  store <2 x double> %733, ptr %736, align 8, !noalias !7262
  br i1 %248, label %812, label %737, !llvm.loop !7277

737:                                              ; preds = %715
  %738 = getelementptr inbounds nuw i8, ptr %295, i64 64
  %739 = getelementptr inbounds nuw i8, ptr %300, i64 64
  %740 = getelementptr inbounds nuw i8, ptr %303, i64 64
  %741 = getelementptr inbounds nuw i8, ptr %300, i64 80
  %742 = getelementptr inbounds nuw i8, ptr %300, i64 96
  %743 = getelementptr inbounds nuw i8, ptr %300, i64 112
  %744 = load <2 x double>, ptr %739, align 8, !noalias !7262
  %745 = load <2 x double>, ptr %741, align 8, !noalias !7262
  %746 = load <2 x double>, ptr %742, align 8, !noalias !7262
  %747 = load <2 x double>, ptr %743, align 8, !noalias !7262
  %748 = getelementptr inbounds nuw i8, ptr %303, i64 80
  %749 = getelementptr inbounds nuw i8, ptr %303, i64 96
  %750 = getelementptr inbounds nuw i8, ptr %303, i64 112
  %751 = load <2 x double>, ptr %740, align 8, !noalias !7262
  %752 = load <2 x double>, ptr %748, align 8, !noalias !7262
  %753 = load <2 x double>, ptr %749, align 8, !noalias !7262
  %754 = load <2 x double>, ptr %750, align 8, !noalias !7262
  %755 = fmul <2 x double> %744, %751
  %756 = fmul <2 x double> %745, %752
  %757 = fmul <2 x double> %746, %753
  %758 = fmul <2 x double> %747, %754
  %759 = getelementptr inbounds nuw i8, ptr %295, i64 80
  %760 = getelementptr inbounds nuw i8, ptr %295, i64 96
  %761 = getelementptr inbounds nuw i8, ptr %295, i64 112
  store <2 x double> %755, ptr %738, align 8, !noalias !7262
  store <2 x double> %756, ptr %759, align 8, !noalias !7262
  store <2 x double> %757, ptr %760, align 8, !noalias !7262
  store <2 x double> %758, ptr %761, align 8, !noalias !7262
  br i1 %249, label %812, label %762, !llvm.loop !7277

762:                                              ; preds = %737
  %763 = getelementptr inbounds nuw i8, ptr %295, i64 128
  %764 = getelementptr inbounds nuw i8, ptr %300, i64 128
  %765 = getelementptr inbounds nuw i8, ptr %303, i64 128
  %766 = getelementptr inbounds nuw i8, ptr %300, i64 144
  %767 = getelementptr inbounds nuw i8, ptr %300, i64 160
  %768 = getelementptr inbounds nuw i8, ptr %300, i64 176
  %769 = load <2 x double>, ptr %764, align 8, !noalias !7262
  %770 = load <2 x double>, ptr %766, align 8, !noalias !7262
  %771 = load <2 x double>, ptr %767, align 8, !noalias !7262
  %772 = load <2 x double>, ptr %768, align 8, !noalias !7262
  %773 = getelementptr inbounds nuw i8, ptr %303, i64 144
  %774 = getelementptr inbounds nuw i8, ptr %303, i64 160
  %775 = getelementptr inbounds nuw i8, ptr %303, i64 176
  %776 = load <2 x double>, ptr %765, align 8, !noalias !7262
  %777 = load <2 x double>, ptr %773, align 8, !noalias !7262
  %778 = load <2 x double>, ptr %774, align 8, !noalias !7262
  %779 = load <2 x double>, ptr %775, align 8, !noalias !7262
  %780 = fmul <2 x double> %769, %776
  %781 = fmul <2 x double> %770, %777
  %782 = fmul <2 x double> %771, %778
  %783 = fmul <2 x double> %772, %779
  %784 = getelementptr inbounds nuw i8, ptr %295, i64 144
  %785 = getelementptr inbounds nuw i8, ptr %295, i64 160
  %786 = getelementptr inbounds nuw i8, ptr %295, i64 176
  store <2 x double> %780, ptr %763, align 8, !noalias !7262
  store <2 x double> %781, ptr %784, align 8, !noalias !7262
  store <2 x double> %782, ptr %785, align 8, !noalias !7262
  store <2 x double> %783, ptr %786, align 8, !noalias !7262
  br i1 %250, label %812, label %787, !llvm.loop !7277

787:                                              ; preds = %762
  %788 = getelementptr inbounds nuw i8, ptr %295, i64 192
  %789 = getelementptr inbounds nuw i8, ptr %300, i64 192
  %790 = getelementptr inbounds nuw i8, ptr %303, i64 192
  %791 = getelementptr inbounds nuw i8, ptr %300, i64 208
  %792 = getelementptr inbounds nuw i8, ptr %300, i64 224
  %793 = getelementptr inbounds nuw i8, ptr %300, i64 240
  %794 = load <2 x double>, ptr %789, align 8, !noalias !7262
  %795 = load <2 x double>, ptr %791, align 8, !noalias !7262
  %796 = load <2 x double>, ptr %792, align 8, !noalias !7262
  %797 = load <2 x double>, ptr %793, align 8, !noalias !7262
  %798 = getelementptr inbounds nuw i8, ptr %303, i64 208
  %799 = getelementptr inbounds nuw i8, ptr %303, i64 224
  %800 = getelementptr inbounds nuw i8, ptr %303, i64 240
  %801 = load <2 x double>, ptr %790, align 8, !noalias !7262
  %802 = load <2 x double>, ptr %798, align 8, !noalias !7262
  %803 = load <2 x double>, ptr %799, align 8, !noalias !7262
  %804 = load <2 x double>, ptr %800, align 8, !noalias !7262
  %805 = fmul <2 x double> %794, %801
  %806 = fmul <2 x double> %795, %802
  %807 = fmul <2 x double> %796, %803
  %808 = fmul <2 x double> %797, %804
  %809 = getelementptr inbounds nuw i8, ptr %295, i64 208
  %810 = getelementptr inbounds nuw i8, ptr %295, i64 224
  %811 = getelementptr inbounds nuw i8, ptr %295, i64 240
  store <2 x double> %805, ptr %788, align 8, !noalias !7262
  store <2 x double> %806, ptr %809, align 8, !noalias !7262
  store <2 x double> %807, ptr %810, align 8, !noalias !7262
  store <2 x double> %808, ptr %811, align 8, !noalias !7262
  br label %812

812:                                              ; preds = %787, %762, %737, %715
  br i1 %251, label %1223, label %813

813:                                              ; preds = %812, %707, %706
  %814 = phi i64 [ 0, %707 ], [ 0, %706 ], [ %247, %812 ]
  br label %1145

815:                                              ; preds = %302
  br i1 %252, label %922, label %816

816:                                              ; preds = %815
  %817 = sub i64 %292, %288
  %818 = and i64 %817, 72057594037927935
  %819 = icmp eq i64 %818, 0
  %820 = sub i64 %292, %290
  %821 = and i64 %820, 72057594037927935
  %822 = icmp eq i64 %821, 0
  %823 = or i1 %819, %822
  br i1 %823, label %922, label %824

824:                                              ; preds = %816
  %825 = getelementptr inbounds nuw i8, ptr %300, i64 16
  %826 = getelementptr inbounds nuw i8, ptr %300, i64 32
  %827 = getelementptr inbounds nuw i8, ptr %300, i64 48
  %828 = load <2 x i64>, ptr %300, align 8, !noalias !7262
  %829 = load <2 x i64>, ptr %825, align 8, !noalias !7262
  %830 = load <2 x i64>, ptr %826, align 8, !noalias !7262
  %831 = load <2 x i64>, ptr %827, align 8, !noalias !7262
  %832 = getelementptr inbounds nuw i8, ptr %303, i64 16
  %833 = getelementptr inbounds nuw i8, ptr %303, i64 32
  %834 = getelementptr inbounds nuw i8, ptr %303, i64 48
  %835 = load <2 x i64>, ptr %303, align 8, !noalias !7262
  %836 = load <2 x i64>, ptr %832, align 8, !noalias !7262
  %837 = load <2 x i64>, ptr %833, align 8, !noalias !7262
  %838 = load <2 x i64>, ptr %834, align 8, !noalias !7262
  %839 = sub <2 x i64> %828, %835
  %840 = sub <2 x i64> %829, %836
  %841 = sub <2 x i64> %830, %837
  %842 = sub <2 x i64> %831, %838
  %843 = getelementptr inbounds nuw i8, ptr %295, i64 16
  %844 = getelementptr inbounds nuw i8, ptr %295, i64 32
  %845 = getelementptr inbounds nuw i8, ptr %295, i64 48
  store <2 x i64> %839, ptr %295, align 8, !noalias !7262
  store <2 x i64> %840, ptr %843, align 8, !noalias !7262
  store <2 x i64> %841, ptr %844, align 8, !noalias !7262
  store <2 x i64> %842, ptr %845, align 8, !noalias !7262
  br i1 %254, label %921, label %846, !llvm.loop !7278

846:                                              ; preds = %824
  %847 = getelementptr inbounds nuw i8, ptr %295, i64 64
  %848 = getelementptr inbounds nuw i8, ptr %300, i64 64
  %849 = getelementptr inbounds nuw i8, ptr %303, i64 64
  %850 = getelementptr inbounds nuw i8, ptr %300, i64 80
  %851 = getelementptr inbounds nuw i8, ptr %300, i64 96
  %852 = getelementptr inbounds nuw i8, ptr %300, i64 112
  %853 = load <2 x i64>, ptr %848, align 8, !noalias !7262
  %854 = load <2 x i64>, ptr %850, align 8, !noalias !7262
  %855 = load <2 x i64>, ptr %851, align 8, !noalias !7262
  %856 = load <2 x i64>, ptr %852, align 8, !noalias !7262
  %857 = getelementptr inbounds nuw i8, ptr %303, i64 80
  %858 = getelementptr inbounds nuw i8, ptr %303, i64 96
  %859 = getelementptr inbounds nuw i8, ptr %303, i64 112
  %860 = load <2 x i64>, ptr %849, align 8, !noalias !7262
  %861 = load <2 x i64>, ptr %857, align 8, !noalias !7262
  %862 = load <2 x i64>, ptr %858, align 8, !noalias !7262
  %863 = load <2 x i64>, ptr %859, align 8, !noalias !7262
  %864 = sub <2 x i64> %853, %860
  %865 = sub <2 x i64> %854, %861
  %866 = sub <2 x i64> %855, %862
  %867 = sub <2 x i64> %856, %863
  %868 = getelementptr inbounds nuw i8, ptr %295, i64 80
  %869 = getelementptr inbounds nuw i8, ptr %295, i64 96
  %870 = getelementptr inbounds nuw i8, ptr %295, i64 112
  store <2 x i64> %864, ptr %847, align 8, !noalias !7262
  store <2 x i64> %865, ptr %868, align 8, !noalias !7262
  store <2 x i64> %866, ptr %869, align 8, !noalias !7262
  store <2 x i64> %867, ptr %870, align 8, !noalias !7262
  br i1 %255, label %921, label %871, !llvm.loop !7278

871:                                              ; preds = %846
  %872 = getelementptr inbounds nuw i8, ptr %295, i64 128
  %873 = getelementptr inbounds nuw i8, ptr %300, i64 128
  %874 = getelementptr inbounds nuw i8, ptr %303, i64 128
  %875 = getelementptr inbounds nuw i8, ptr %300, i64 144
  %876 = getelementptr inbounds nuw i8, ptr %300, i64 160
  %877 = getelementptr inbounds nuw i8, ptr %300, i64 176
  %878 = load <2 x i64>, ptr %873, align 8, !noalias !7262
  %879 = load <2 x i64>, ptr %875, align 8, !noalias !7262
  %880 = load <2 x i64>, ptr %876, align 8, !noalias !7262
  %881 = load <2 x i64>, ptr %877, align 8, !noalias !7262
  %882 = getelementptr inbounds nuw i8, ptr %303, i64 144
  %883 = getelementptr inbounds nuw i8, ptr %303, i64 160
  %884 = getelementptr inbounds nuw i8, ptr %303, i64 176
  %885 = load <2 x i64>, ptr %874, align 8, !noalias !7262
  %886 = load <2 x i64>, ptr %882, align 8, !noalias !7262
  %887 = load <2 x i64>, ptr %883, align 8, !noalias !7262
  %888 = load <2 x i64>, ptr %884, align 8, !noalias !7262
  %889 = sub <2 x i64> %878, %885
  %890 = sub <2 x i64> %879, %886
  %891 = sub <2 x i64> %880, %887
  %892 = sub <2 x i64> %881, %888
  %893 = getelementptr inbounds nuw i8, ptr %295, i64 144
  %894 = getelementptr inbounds nuw i8, ptr %295, i64 160
  %895 = getelementptr inbounds nuw i8, ptr %295, i64 176
  store <2 x i64> %889, ptr %872, align 8, !noalias !7262
  store <2 x i64> %890, ptr %893, align 8, !noalias !7262
  store <2 x i64> %891, ptr %894, align 8, !noalias !7262
  store <2 x i64> %892, ptr %895, align 8, !noalias !7262
  br i1 %256, label %921, label %896, !llvm.loop !7278

896:                                              ; preds = %871
  %897 = getelementptr inbounds nuw i8, ptr %295, i64 192
  %898 = getelementptr inbounds nuw i8, ptr %300, i64 192
  %899 = getelementptr inbounds nuw i8, ptr %303, i64 192
  %900 = getelementptr inbounds nuw i8, ptr %300, i64 208
  %901 = getelementptr inbounds nuw i8, ptr %300, i64 224
  %902 = getelementptr inbounds nuw i8, ptr %300, i64 240
  %903 = load <2 x i64>, ptr %898, align 8, !noalias !7262
  %904 = load <2 x i64>, ptr %900, align 8, !noalias !7262
  %905 = load <2 x i64>, ptr %901, align 8, !noalias !7262
  %906 = load <2 x i64>, ptr %902, align 8, !noalias !7262
  %907 = getelementptr inbounds nuw i8, ptr %303, i64 208
  %908 = getelementptr inbounds nuw i8, ptr %303, i64 224
  %909 = getelementptr inbounds nuw i8, ptr %303, i64 240
  %910 = load <2 x i64>, ptr %899, align 8, !noalias !7262
  %911 = load <2 x i64>, ptr %907, align 8, !noalias !7262
  %912 = load <2 x i64>, ptr %908, align 8, !noalias !7262
  %913 = load <2 x i64>, ptr %909, align 8, !noalias !7262
  %914 = sub <2 x i64> %903, %910
  %915 = sub <2 x i64> %904, %911
  %916 = sub <2 x i64> %905, %912
  %917 = sub <2 x i64> %906, %913
  %918 = getelementptr inbounds nuw i8, ptr %295, i64 208
  %919 = getelementptr inbounds nuw i8, ptr %295, i64 224
  %920 = getelementptr inbounds nuw i8, ptr %295, i64 240
  store <2 x i64> %914, ptr %897, align 8, !noalias !7262
  store <2 x i64> %915, ptr %918, align 8, !noalias !7262
  store <2 x i64> %916, ptr %919, align 8, !noalias !7262
  store <2 x i64> %917, ptr %920, align 8, !noalias !7262
  br label %921

921:                                              ; preds = %896, %871, %846, %824
  br i1 %257, label %1223, label %922

922:                                              ; preds = %921, %816, %815
  %923 = phi i64 [ 0, %816 ], [ 0, %815 ], [ %253, %921 ]
  br label %1155

924:                                              ; preds = %302
  br i1 %258, label %1031, label %925

925:                                              ; preds = %924
  %926 = sub i64 %292, %288
  %927 = and i64 %926, 72057594037927935
  %928 = icmp eq i64 %927, 0
  %929 = sub i64 %292, %290
  %930 = and i64 %929, 72057594037927935
  %931 = icmp eq i64 %930, 0
  %932 = or i1 %928, %931
  br i1 %932, label %1031, label %933

933:                                              ; preds = %925
  %934 = getelementptr inbounds nuw i8, ptr %300, i64 16
  %935 = getelementptr inbounds nuw i8, ptr %300, i64 32
  %936 = getelementptr inbounds nuw i8, ptr %300, i64 48
  %937 = load <2 x i64>, ptr %300, align 8, !noalias !7262
  %938 = load <2 x i64>, ptr %934, align 8, !noalias !7262
  %939 = load <2 x i64>, ptr %935, align 8, !noalias !7262
  %940 = load <2 x i64>, ptr %936, align 8, !noalias !7262
  %941 = getelementptr inbounds nuw i8, ptr %303, i64 16
  %942 = getelementptr inbounds nuw i8, ptr %303, i64 32
  %943 = getelementptr inbounds nuw i8, ptr %303, i64 48
  %944 = load <2 x i64>, ptr %303, align 8, !noalias !7262
  %945 = load <2 x i64>, ptr %941, align 8, !noalias !7262
  %946 = load <2 x i64>, ptr %942, align 8, !noalias !7262
  %947 = load <2 x i64>, ptr %943, align 8, !noalias !7262
  %948 = add <2 x i64> %944, %937
  %949 = add <2 x i64> %945, %938
  %950 = add <2 x i64> %946, %939
  %951 = add <2 x i64> %947, %940
  %952 = getelementptr inbounds nuw i8, ptr %295, i64 16
  %953 = getelementptr inbounds nuw i8, ptr %295, i64 32
  %954 = getelementptr inbounds nuw i8, ptr %295, i64 48
  store <2 x i64> %948, ptr %295, align 8, !noalias !7262
  store <2 x i64> %949, ptr %952, align 8, !noalias !7262
  store <2 x i64> %950, ptr %953, align 8, !noalias !7262
  store <2 x i64> %951, ptr %954, align 8, !noalias !7262
  br i1 %260, label %1030, label %955, !llvm.loop !7279

955:                                              ; preds = %933
  %956 = getelementptr inbounds nuw i8, ptr %295, i64 64
  %957 = getelementptr inbounds nuw i8, ptr %300, i64 64
  %958 = getelementptr inbounds nuw i8, ptr %303, i64 64
  %959 = getelementptr inbounds nuw i8, ptr %300, i64 80
  %960 = getelementptr inbounds nuw i8, ptr %300, i64 96
  %961 = getelementptr inbounds nuw i8, ptr %300, i64 112
  %962 = load <2 x i64>, ptr %957, align 8, !noalias !7262
  %963 = load <2 x i64>, ptr %959, align 8, !noalias !7262
  %964 = load <2 x i64>, ptr %960, align 8, !noalias !7262
  %965 = load <2 x i64>, ptr %961, align 8, !noalias !7262
  %966 = getelementptr inbounds nuw i8, ptr %303, i64 80
  %967 = getelementptr inbounds nuw i8, ptr %303, i64 96
  %968 = getelementptr inbounds nuw i8, ptr %303, i64 112
  %969 = load <2 x i64>, ptr %958, align 8, !noalias !7262
  %970 = load <2 x i64>, ptr %966, align 8, !noalias !7262
  %971 = load <2 x i64>, ptr %967, align 8, !noalias !7262
  %972 = load <2 x i64>, ptr %968, align 8, !noalias !7262
  %973 = add <2 x i64> %969, %962
  %974 = add <2 x i64> %970, %963
  %975 = add <2 x i64> %971, %964
  %976 = add <2 x i64> %972, %965
  %977 = getelementptr inbounds nuw i8, ptr %295, i64 80
  %978 = getelementptr inbounds nuw i8, ptr %295, i64 96
  %979 = getelementptr inbounds nuw i8, ptr %295, i64 112
  store <2 x i64> %973, ptr %956, align 8, !noalias !7262
  store <2 x i64> %974, ptr %977, align 8, !noalias !7262
  store <2 x i64> %975, ptr %978, align 8, !noalias !7262
  store <2 x i64> %976, ptr %979, align 8, !noalias !7262
  br i1 %261, label %1030, label %980, !llvm.loop !7279

980:                                              ; preds = %955
  %981 = getelementptr inbounds nuw i8, ptr %295, i64 128
  %982 = getelementptr inbounds nuw i8, ptr %300, i64 128
  %983 = getelementptr inbounds nuw i8, ptr %303, i64 128
  %984 = getelementptr inbounds nuw i8, ptr %300, i64 144
  %985 = getelementptr inbounds nuw i8, ptr %300, i64 160
  %986 = getelementptr inbounds nuw i8, ptr %300, i64 176
  %987 = load <2 x i64>, ptr %982, align 8, !noalias !7262
  %988 = load <2 x i64>, ptr %984, align 8, !noalias !7262
  %989 = load <2 x i64>, ptr %985, align 8, !noalias !7262
  %990 = load <2 x i64>, ptr %986, align 8, !noalias !7262
  %991 = getelementptr inbounds nuw i8, ptr %303, i64 144
  %992 = getelementptr inbounds nuw i8, ptr %303, i64 160
  %993 = getelementptr inbounds nuw i8, ptr %303, i64 176
  %994 = load <2 x i64>, ptr %983, align 8, !noalias !7262
  %995 = load <2 x i64>, ptr %991, align 8, !noalias !7262
  %996 = load <2 x i64>, ptr %992, align 8, !noalias !7262
  %997 = load <2 x i64>, ptr %993, align 8, !noalias !7262
  %998 = add <2 x i64> %994, %987
  %999 = add <2 x i64> %995, %988
  %1000 = add <2 x i64> %996, %989
  %1001 = add <2 x i64> %997, %990
  %1002 = getelementptr inbounds nuw i8, ptr %295, i64 144
  %1003 = getelementptr inbounds nuw i8, ptr %295, i64 160
  %1004 = getelementptr inbounds nuw i8, ptr %295, i64 176
  store <2 x i64> %998, ptr %981, align 8, !noalias !7262
  store <2 x i64> %999, ptr %1002, align 8, !noalias !7262
  store <2 x i64> %1000, ptr %1003, align 8, !noalias !7262
  store <2 x i64> %1001, ptr %1004, align 8, !noalias !7262
  br i1 %262, label %1030, label %1005, !llvm.loop !7279

1005:                                             ; preds = %980
  %1006 = getelementptr inbounds nuw i8, ptr %295, i64 192
  %1007 = getelementptr inbounds nuw i8, ptr %300, i64 192
  %1008 = getelementptr inbounds nuw i8, ptr %303, i64 192
  %1009 = getelementptr inbounds nuw i8, ptr %300, i64 208
  %1010 = getelementptr inbounds nuw i8, ptr %300, i64 224
  %1011 = getelementptr inbounds nuw i8, ptr %300, i64 240
  %1012 = load <2 x i64>, ptr %1007, align 8, !noalias !7262
  %1013 = load <2 x i64>, ptr %1009, align 8, !noalias !7262
  %1014 = load <2 x i64>, ptr %1010, align 8, !noalias !7262
  %1015 = load <2 x i64>, ptr %1011, align 8, !noalias !7262
  %1016 = getelementptr inbounds nuw i8, ptr %303, i64 208
  %1017 = getelementptr inbounds nuw i8, ptr %303, i64 224
  %1018 = getelementptr inbounds nuw i8, ptr %303, i64 240
  %1019 = load <2 x i64>, ptr %1008, align 8, !noalias !7262
  %1020 = load <2 x i64>, ptr %1016, align 8, !noalias !7262
  %1021 = load <2 x i64>, ptr %1017, align 8, !noalias !7262
  %1022 = load <2 x i64>, ptr %1018, align 8, !noalias !7262
  %1023 = add <2 x i64> %1019, %1012
  %1024 = add <2 x i64> %1020, %1013
  %1025 = add <2 x i64> %1021, %1014
  %1026 = add <2 x i64> %1022, %1015
  %1027 = getelementptr inbounds nuw i8, ptr %295, i64 208
  %1028 = getelementptr inbounds nuw i8, ptr %295, i64 224
  %1029 = getelementptr inbounds nuw i8, ptr %295, i64 240
  store <2 x i64> %1023, ptr %1006, align 8, !noalias !7262
  store <2 x i64> %1024, ptr %1027, align 8, !noalias !7262
  store <2 x i64> %1025, ptr %1028, align 8, !noalias !7262
  store <2 x i64> %1026, ptr %1029, align 8, !noalias !7262
  br label %1030

1030:                                             ; preds = %1005, %980, %955, %933
  br i1 %263, label %1223, label %1031

1031:                                             ; preds = %1030, %925, %924
  %1032 = phi i64 [ 0, %925 ], [ 0, %924 ], [ %259, %1030 ]
  br label %1165

1033:                                             ; preds = %302
  br i1 %275, label %1105, label %1034

1034:                                             ; preds = %1033
  %1035 = sub i64 %292, %288
  %1036 = and i64 %1035, 72057594037927935
  %1037 = icmp eq i64 %1036, 0
  br i1 %1037, label %1105, label %1038

1038:                                             ; preds = %1034
  %1039 = getelementptr inbounds nuw i8, ptr %300, i64 16
  %1040 = getelementptr inbounds nuw i8, ptr %300, i64 32
  %1041 = getelementptr inbounds nuw i8, ptr %300, i64 48
  %1042 = load <2 x i64>, ptr %300, align 8, !noalias !7262
  %1043 = load <2 x i64>, ptr %1039, align 8, !noalias !7262
  %1044 = load <2 x i64>, ptr %1040, align 8, !noalias !7262
  %1045 = load <2 x i64>, ptr %1041, align 8, !noalias !7262
  %1046 = uitofp <2 x i64> %1042 to <2 x double>
  %1047 = uitofp <2 x i64> %1043 to <2 x double>
  %1048 = uitofp <2 x i64> %1044 to <2 x double>
  %1049 = uitofp <2 x i64> %1045 to <2 x double>
  %1050 = getelementptr inbounds nuw i8, ptr %295, i64 16
  %1051 = getelementptr inbounds nuw i8, ptr %295, i64 32
  %1052 = getelementptr inbounds nuw i8, ptr %295, i64 48
  store <2 x double> %1046, ptr %295, align 8, !noalias !7262
  store <2 x double> %1047, ptr %1050, align 8, !noalias !7262
  store <2 x double> %1048, ptr %1051, align 8, !noalias !7262
  store <2 x double> %1049, ptr %1052, align 8, !noalias !7262
  br i1 %277, label %1104, label %1053, !llvm.loop !7280

1053:                                             ; preds = %1038
  %1054 = getelementptr inbounds nuw i8, ptr %295, i64 64
  %1055 = getelementptr inbounds nuw i8, ptr %300, i64 64
  %1056 = getelementptr inbounds nuw i8, ptr %300, i64 80
  %1057 = getelementptr inbounds nuw i8, ptr %300, i64 96
  %1058 = getelementptr inbounds nuw i8, ptr %300, i64 112
  %1059 = load <2 x i64>, ptr %1055, align 8, !noalias !7262
  %1060 = load <2 x i64>, ptr %1056, align 8, !noalias !7262
  %1061 = load <2 x i64>, ptr %1057, align 8, !noalias !7262
  %1062 = load <2 x i64>, ptr %1058, align 8, !noalias !7262
  %1063 = uitofp <2 x i64> %1059 to <2 x double>
  %1064 = uitofp <2 x i64> %1060 to <2 x double>
  %1065 = uitofp <2 x i64> %1061 to <2 x double>
  %1066 = uitofp <2 x i64> %1062 to <2 x double>
  %1067 = getelementptr inbounds nuw i8, ptr %295, i64 80
  %1068 = getelementptr inbounds nuw i8, ptr %295, i64 96
  %1069 = getelementptr inbounds nuw i8, ptr %295, i64 112
  store <2 x double> %1063, ptr %1054, align 8, !noalias !7262
  store <2 x double> %1064, ptr %1067, align 8, !noalias !7262
  store <2 x double> %1065, ptr %1068, align 8, !noalias !7262
  store <2 x double> %1066, ptr %1069, align 8, !noalias !7262
  br i1 %278, label %1104, label %1070, !llvm.loop !7280

1070:                                             ; preds = %1053
  %1071 = getelementptr inbounds nuw i8, ptr %295, i64 128
  %1072 = getelementptr inbounds nuw i8, ptr %300, i64 128
  %1073 = getelementptr inbounds nuw i8, ptr %300, i64 144
  %1074 = getelementptr inbounds nuw i8, ptr %300, i64 160
  %1075 = getelementptr inbounds nuw i8, ptr %300, i64 176
  %1076 = load <2 x i64>, ptr %1072, align 8, !noalias !7262
  %1077 = load <2 x i64>, ptr %1073, align 8, !noalias !7262
  %1078 = load <2 x i64>, ptr %1074, align 8, !noalias !7262
  %1079 = load <2 x i64>, ptr %1075, align 8, !noalias !7262
  %1080 = uitofp <2 x i64> %1076 to <2 x double>
  %1081 = uitofp <2 x i64> %1077 to <2 x double>
  %1082 = uitofp <2 x i64> %1078 to <2 x double>
  %1083 = uitofp <2 x i64> %1079 to <2 x double>
  %1084 = getelementptr inbounds nuw i8, ptr %295, i64 144
  %1085 = getelementptr inbounds nuw i8, ptr %295, i64 160
  %1086 = getelementptr inbounds nuw i8, ptr %295, i64 176
  store <2 x double> %1080, ptr %1071, align 8, !noalias !7262
  store <2 x double> %1081, ptr %1084, align 8, !noalias !7262
  store <2 x double> %1082, ptr %1085, align 8, !noalias !7262
  store <2 x double> %1083, ptr %1086, align 8, !noalias !7262
  br i1 %279, label %1104, label %1087, !llvm.loop !7280

1087:                                             ; preds = %1070
  %1088 = getelementptr inbounds nuw i8, ptr %295, i64 192
  %1089 = getelementptr inbounds nuw i8, ptr %300, i64 192
  %1090 = getelementptr inbounds nuw i8, ptr %300, i64 208
  %1091 = getelementptr inbounds nuw i8, ptr %300, i64 224
  %1092 = getelementptr inbounds nuw i8, ptr %300, i64 240
  %1093 = load <2 x i64>, ptr %1089, align 8, !noalias !7262
  %1094 = load <2 x i64>, ptr %1090, align 8, !noalias !7262
  %1095 = load <2 x i64>, ptr %1091, align 8, !noalias !7262
  %1096 = load <2 x i64>, ptr %1092, align 8, !noalias !7262
  %1097 = uitofp <2 x i64> %1093 to <2 x double>
  %1098 = uitofp <2 x i64> %1094 to <2 x double>
  %1099 = uitofp <2 x i64> %1095 to <2 x double>
  %1100 = uitofp <2 x i64> %1096 to <2 x double>
  %1101 = getelementptr inbounds nuw i8, ptr %295, i64 208
  %1102 = getelementptr inbounds nuw i8, ptr %295, i64 224
  %1103 = getelementptr inbounds nuw i8, ptr %295, i64 240
  store <2 x double> %1097, ptr %1088, align 8, !noalias !7262
  store <2 x double> %1098, ptr %1101, align 8, !noalias !7262
  store <2 x double> %1099, ptr %1102, align 8, !noalias !7262
  store <2 x double> %1100, ptr %1103, align 8, !noalias !7262
  br label %1104

1104:                                             ; preds = %1087, %1070, %1053, %1038
  br i1 %280, label %1223, label %1105

1105:                                             ; preds = %1104, %1034, %1033
  %1106 = phi i64 [ 0, %1034 ], [ 0, %1033 ], [ %276, %1104 ]
  br label %1225

1107:                                             ; preds = %1107, %377
  %1108 = phi i64 [ %1109, %1107 ], [ %378, %377 ]
  %1109 = add nuw i64 %1108, 1
  %1110 = getelementptr inbounds nuw i64, ptr %295, i64 %1108
  %1111 = getelementptr inbounds nuw i64, ptr %300, i64 %1108
  %1112 = load double, ptr %1111, align 8, !noalias !7262, !noundef !4
  %1113 = tail call double @llvm.sqrt.f64(double %1112)
  store double %1113, ptr %1110, align 8, !noalias !7262
  %1114 = icmp eq i64 %1109, %80
  br i1 %1114, label %1223, label %1107, !llvm.loop !7281

1115:                                             ; preds = %1115, %486
  %1116 = phi i64 [ %1117, %1115 ], [ %487, %486 ]
  %1117 = add nuw i64 %1116, 1
  %1118 = getelementptr inbounds nuw i64, ptr %295, i64 %1116
  %1119 = getelementptr inbounds nuw i64, ptr %300, i64 %1116
  %1120 = getelementptr inbounds nuw i64, ptr %303, i64 %1116
  %1121 = load double, ptr %1119, align 8, !noalias !7262, !noundef !4
  %1122 = load double, ptr %1120, align 8, !noalias !7262, !noundef !4
  %1123 = fsub double %1121, %1122
  store double %1123, ptr %1118, align 8, !noalias !7262
  %1124 = icmp eq i64 %1117, %80
  br i1 %1124, label %1223, label %1115, !llvm.loop !7282

1125:                                             ; preds = %1125, %595
  %1126 = phi i64 [ %1127, %1125 ], [ %596, %595 ]
  %1127 = add nuw i64 %1126, 1
  %1128 = getelementptr inbounds nuw i64, ptr %295, i64 %1126
  %1129 = getelementptr inbounds nuw i64, ptr %300, i64 %1126
  %1130 = getelementptr inbounds nuw i64, ptr %303, i64 %1126
  %1131 = load double, ptr %1129, align 8, !noalias !7262, !noundef !4
  %1132 = load double, ptr %1130, align 8, !noalias !7262, !noundef !4
  %1133 = fadd double %1131, %1132
  store double %1133, ptr %1128, align 8, !noalias !7262
  %1134 = icmp eq i64 %1127, %80
  br i1 %1134, label %1223, label %1125, !llvm.loop !7283

1135:                                             ; preds = %1135, %704
  %1136 = phi i64 [ %1137, %1135 ], [ %705, %704 ]
  %1137 = add nuw i64 %1136, 1
  %1138 = getelementptr inbounds nuw i64, ptr %295, i64 %1136
  %1139 = getelementptr inbounds nuw i64, ptr %300, i64 %1136
  %1140 = getelementptr inbounds nuw i64, ptr %303, i64 %1136
  %1141 = load double, ptr %1139, align 8, !noalias !7262, !noundef !4
  %1142 = load double, ptr %1140, align 8, !noalias !7262, !noundef !4
  %1143 = fdiv double %1141, %1142
  store double %1143, ptr %1138, align 8, !noalias !7262
  %1144 = icmp eq i64 %1137, %80
  br i1 %1144, label %1223, label %1135, !llvm.loop !7284

1145:                                             ; preds = %1145, %813
  %1146 = phi i64 [ %1147, %1145 ], [ %814, %813 ]
  %1147 = add nuw i64 %1146, 1
  %1148 = getelementptr inbounds nuw i64, ptr %295, i64 %1146
  %1149 = getelementptr inbounds nuw i64, ptr %300, i64 %1146
  %1150 = getelementptr inbounds nuw i64, ptr %303, i64 %1146
  %1151 = load double, ptr %1149, align 8, !noalias !7262, !noundef !4
  %1152 = load double, ptr %1150, align 8, !noalias !7262, !noundef !4
  %1153 = fmul double %1151, %1152
  store double %1153, ptr %1148, align 8, !noalias !7262
  %1154 = icmp eq i64 %1147, %80
  br i1 %1154, label %1223, label %1145, !llvm.loop !7285

1155:                                             ; preds = %1155, %922
  %1156 = phi i64 [ %1157, %1155 ], [ %923, %922 ]
  %1157 = add nuw i64 %1156, 1
  %1158 = getelementptr inbounds nuw i64, ptr %295, i64 %1156
  %1159 = getelementptr inbounds nuw i64, ptr %300, i64 %1156
  %1160 = getelementptr inbounds nuw i64, ptr %303, i64 %1156
  %1161 = load i64, ptr %1159, align 8, !noalias !7262, !noundef !4
  %1162 = load i64, ptr %1160, align 8, !noalias !7262, !noundef !4
  %1163 = sub i64 %1161, %1162
  store i64 %1163, ptr %1158, align 8, !noalias !7262
  %1164 = icmp eq i64 %1157, %80
  br i1 %1164, label %1223, label %1155, !llvm.loop !7286

1165:                                             ; preds = %1165, %1031
  %1166 = phi i64 [ %1167, %1165 ], [ %1032, %1031 ]
  %1167 = add nuw i64 %1166, 1
  %1168 = getelementptr inbounds nuw i64, ptr %295, i64 %1166
  %1169 = getelementptr inbounds nuw i64, ptr %300, i64 %1166
  %1170 = getelementptr inbounds nuw i64, ptr %303, i64 %1166
  %1171 = load i64, ptr %1169, align 8, !noalias !7262, !noundef !4
  %1172 = load i64, ptr %1170, align 8, !noalias !7262, !noundef !4
  %1173 = add i64 %1172, %1171
  store i64 %1173, ptr %1168, align 8, !noalias !7262
  %1174 = icmp eq i64 %1167, %80
  br i1 %1174, label %1223, label %1165, !llvm.loop !7287

1175:                                             ; preds = %1309, %1175
  %1176 = phi i64 [ %1177, %1175 ], [ %1310, %1309 ]
  %1177 = add nuw i64 %1176, 1
  %1178 = getelementptr inbounds nuw i64, ptr %295, i64 %1176
  %1179 = getelementptr inbounds nuw i64, ptr %300, i64 %1176
  %1180 = load i64, ptr %1179, align 8, !noalias !7262, !noundef !4
  %1181 = lshr i64 %1180, %1235
  store i64 %1181, ptr %1178, align 8, !noalias !7262
  %1182 = icmp eq i64 %1177, %80
  br i1 %1182, label %1223, label %1175, !llvm.loop !7288

1183:                                             ; preds = %1188, %302
  %1184 = phi i64 [ %1191, %1188 ], [ 0, %302 ]
  %1185 = getelementptr inbounds nuw i64, ptr %303, i64 %1184
  %1186 = load i64, ptr %1185, align 8, !noalias !7262, !noundef !4
  %1187 = icmp eq i64 %1186, 0
  br i1 %1187, label %1317, label %1188

1188:                                             ; preds = %1183
  %1189 = getelementptr inbounds nuw i64, ptr %300, i64 %1184
  %1190 = getelementptr inbounds nuw i64, ptr %295, i64 %1184
  %1191 = add nuw i64 %1184, 1
  %1192 = load i64, ptr %1189, align 8, !noalias !7262, !noundef !4
  %1193 = udiv i64 %1192, %1186
  store i64 %1193, ptr %1190, align 8, !noalias !7262
  %1194 = icmp eq i64 %1191, %80
  br i1 %1194, label %1223, label %1183

1195:                                             ; preds = %1195, %304
  %1196 = phi i64 [ %1205, %1195 ], [ 0, %304 ]
  %1197 = phi i64 [ %1212, %1195 ], [ 0, %304 ]
  %1198 = or disjoint i64 %1196, 1
  %1199 = getelementptr inbounds nuw i64, ptr %295, i64 %1196
  %1200 = getelementptr inbounds nuw i64, ptr %300, i64 %1196
  %1201 = getelementptr inbounds nuw i64, ptr %303, i64 %1196
  %1202 = load i64, ptr %1200, align 8, !noalias !7262, !noundef !4
  %1203 = load i64, ptr %1201, align 8, !noalias !7262, !noundef !4
  %1204 = mul i64 %1203, %1202
  store i64 %1204, ptr %1199, align 8, !noalias !7262
  %1205 = add nuw i64 %1196, 2
  %1206 = getelementptr inbounds nuw i64, ptr %295, i64 %1198
  %1207 = getelementptr inbounds nuw i64, ptr %300, i64 %1198
  %1208 = getelementptr inbounds nuw i64, ptr %303, i64 %1198
  %1209 = load i64, ptr %1207, align 8, !noalias !7262, !noundef !4
  %1210 = load i64, ptr %1208, align 8, !noalias !7262, !noundef !4
  %1211 = mul i64 %1210, %1209
  store i64 %1211, ptr %1206, align 8, !noalias !7262
  %1212 = add i64 %1197, 2
  %1213 = icmp eq i64 %1212, %272
  br i1 %1213, label %1214, label %1195

1214:                                             ; preds = %1195
  br i1 %273, label %1223, label %1215

1215:                                             ; preds = %1214, %304
  %1216 = phi i64 [ 0, %304 ], [ %1205, %1214 ]
  tail call void @llvm.assume(i1 %274)
  %1217 = getelementptr inbounds nuw i64, ptr %295, i64 %1216
  %1218 = getelementptr inbounds nuw i64, ptr %300, i64 %1216
  %1219 = getelementptr inbounds nuw i64, ptr %303, i64 %1216
  %1220 = load i64, ptr %1218, align 8, !noalias !7262, !noundef !4
  %1221 = load i64, ptr %1219, align 8, !noalias !7262, !noundef !4
  %1222 = mul i64 %1221, %1220
  store i64 %1222, ptr %1217, align 8, !noalias !7262
  br label %1223

1223:                                             ; preds = %1308, %1225, %1215, %1214, %1188, %1175, %1165, %1155, %1145, %1135, %1125, %1115, %1107, %1104, %1030, %921, %812, %703, %594, %485, %376
  %1224 = icmp eq ptr %283, %36
  br i1 %1224, label %1318, label %281

1225:                                             ; preds = %1225, %1105
  %1226 = phi i64 [ %1227, %1225 ], [ %1106, %1105 ]
  %1227 = add nuw i64 %1226, 1
  %1228 = getelementptr inbounds nuw i64, ptr %295, i64 %1226
  %1229 = getelementptr inbounds nuw i64, ptr %300, i64 %1226
  %1230 = load i64, ptr %1229, align 8, !noalias !7262, !noundef !4
  %1231 = uitofp i64 %1230 to double
  store double %1231, ptr %1228, align 8, !noalias !7262
  %1232 = icmp eq i64 %1227, %80
  br i1 %1232, label %1223, label %1225, !llvm.loop !7289

1233:                                             ; preds = %302
  %1234 = and i32 %286, 63
  %1235 = zext nneg i32 %1234 to i64
  br i1 %264, label %1309, label %1236

1236:                                             ; preds = %1233
  %1237 = sub i64 %292, %288
  %1238 = and i64 %1237, 72057594037927935
  %1239 = icmp eq i64 %1238, 0
  br i1 %1239, label %1309, label %1240

1240:                                             ; preds = %1236
  %1241 = insertelement <2 x i64> poison, i64 %1235, i64 0
  %1242 = shufflevector <2 x i64> %1241, <2 x i64> poison, <2 x i32> zeroinitializer
  %1243 = getelementptr inbounds nuw i8, ptr %300, i64 16
  %1244 = getelementptr inbounds nuw i8, ptr %300, i64 32
  %1245 = getelementptr inbounds nuw i8, ptr %300, i64 48
  %1246 = load <2 x i64>, ptr %300, align 8, !noalias !7262
  %1247 = load <2 x i64>, ptr %1243, align 8, !noalias !7262
  %1248 = load <2 x i64>, ptr %1244, align 8, !noalias !7262
  %1249 = load <2 x i64>, ptr %1245, align 8, !noalias !7262
  %1250 = lshr <2 x i64> %1246, %1242
  %1251 = lshr <2 x i64> %1247, %1242
  %1252 = lshr <2 x i64> %1248, %1242
  %1253 = lshr <2 x i64> %1249, %1242
  %1254 = getelementptr inbounds nuw i8, ptr %295, i64 16
  %1255 = getelementptr inbounds nuw i8, ptr %295, i64 32
  %1256 = getelementptr inbounds nuw i8, ptr %295, i64 48
  store <2 x i64> %1250, ptr %295, align 8, !noalias !7262
  store <2 x i64> %1251, ptr %1254, align 8, !noalias !7262
  store <2 x i64> %1252, ptr %1255, align 8, !noalias !7262
  store <2 x i64> %1253, ptr %1256, align 8, !noalias !7262
  br i1 %266, label %1308, label %1257, !llvm.loop !7290

1257:                                             ; preds = %1240
  %1258 = getelementptr inbounds nuw i8, ptr %295, i64 64
  %1259 = getelementptr inbounds nuw i8, ptr %300, i64 64
  %1260 = getelementptr inbounds nuw i8, ptr %300, i64 80
  %1261 = getelementptr inbounds nuw i8, ptr %300, i64 96
  %1262 = getelementptr inbounds nuw i8, ptr %300, i64 112
  %1263 = load <2 x i64>, ptr %1259, align 8, !noalias !7262
  %1264 = load <2 x i64>, ptr %1260, align 8, !noalias !7262
  %1265 = load <2 x i64>, ptr %1261, align 8, !noalias !7262
  %1266 = load <2 x i64>, ptr %1262, align 8, !noalias !7262
  %1267 = lshr <2 x i64> %1263, %1242
  %1268 = lshr <2 x i64> %1264, %1242
  %1269 = lshr <2 x i64> %1265, %1242
  %1270 = lshr <2 x i64> %1266, %1242
  %1271 = getelementptr inbounds nuw i8, ptr %295, i64 80
  %1272 = getelementptr inbounds nuw i8, ptr %295, i64 96
  %1273 = getelementptr inbounds nuw i8, ptr %295, i64 112
  store <2 x i64> %1267, ptr %1258, align 8, !noalias !7262
  store <2 x i64> %1268, ptr %1271, align 8, !noalias !7262
  store <2 x i64> %1269, ptr %1272, align 8, !noalias !7262
  store <2 x i64> %1270, ptr %1273, align 8, !noalias !7262
  br i1 %267, label %1308, label %1274, !llvm.loop !7290

1274:                                             ; preds = %1257
  %1275 = getelementptr inbounds nuw i8, ptr %295, i64 128
  %1276 = getelementptr inbounds nuw i8, ptr %300, i64 128
  %1277 = getelementptr inbounds nuw i8, ptr %300, i64 144
  %1278 = getelementptr inbounds nuw i8, ptr %300, i64 160
  %1279 = getelementptr inbounds nuw i8, ptr %300, i64 176
  %1280 = load <2 x i64>, ptr %1276, align 8, !noalias !7262
  %1281 = load <2 x i64>, ptr %1277, align 8, !noalias !7262
  %1282 = load <2 x i64>, ptr %1278, align 8, !noalias !7262
  %1283 = load <2 x i64>, ptr %1279, align 8, !noalias !7262
  %1284 = lshr <2 x i64> %1280, %1242
  %1285 = lshr <2 x i64> %1281, %1242
  %1286 = lshr <2 x i64> %1282, %1242
  %1287 = lshr <2 x i64> %1283, %1242
  %1288 = getelementptr inbounds nuw i8, ptr %295, i64 144
  %1289 = getelementptr inbounds nuw i8, ptr %295, i64 160
  %1290 = getelementptr inbounds nuw i8, ptr %295, i64 176
  store <2 x i64> %1284, ptr %1275, align 8, !noalias !7262
  store <2 x i64> %1285, ptr %1288, align 8, !noalias !7262
  store <2 x i64> %1286, ptr %1289, align 8, !noalias !7262
  store <2 x i64> %1287, ptr %1290, align 8, !noalias !7262
  br i1 %268, label %1308, label %1291, !llvm.loop !7290

1291:                                             ; preds = %1274
  %1292 = getelementptr inbounds nuw i8, ptr %295, i64 192
  %1293 = getelementptr inbounds nuw i8, ptr %300, i64 192
  %1294 = getelementptr inbounds nuw i8, ptr %300, i64 208
  %1295 = getelementptr inbounds nuw i8, ptr %300, i64 224
  %1296 = getelementptr inbounds nuw i8, ptr %300, i64 240
  %1297 = load <2 x i64>, ptr %1293, align 8, !noalias !7262
  %1298 = load <2 x i64>, ptr %1294, align 8, !noalias !7262
  %1299 = load <2 x i64>, ptr %1295, align 8, !noalias !7262
  %1300 = load <2 x i64>, ptr %1296, align 8, !noalias !7262
  %1301 = lshr <2 x i64> %1297, %1242
  %1302 = lshr <2 x i64> %1298, %1242
  %1303 = lshr <2 x i64> %1299, %1242
  %1304 = lshr <2 x i64> %1300, %1242
  %1305 = getelementptr inbounds nuw i8, ptr %295, i64 208
  %1306 = getelementptr inbounds nuw i8, ptr %295, i64 224
  %1307 = getelementptr inbounds nuw i8, ptr %295, i64 240
  store <2 x i64> %1301, ptr %1292, align 8, !noalias !7262
  store <2 x i64> %1302, ptr %1305, align 8, !noalias !7262
  store <2 x i64> %1303, ptr %1306, align 8, !noalias !7262
  store <2 x i64> %1304, ptr %1307, align 8, !noalias !7262
  br label %1308

1308:                                             ; preds = %1291, %1274, %1257, %1240
  br i1 %269, label %1223, label %1309

1309:                                             ; preds = %1308, %1236, %1233
  %1310 = phi i64 [ 0, %1236 ], [ 0, %1233 ], [ %265, %1308 ]
  br label %1175

1311:                                             ; preds = %118
  tail call void @_RNvNtCsgDHHaSskbIF_4core9panicking18panic_bounds_check(i64 noundef %121, i64 noundef %24, ptr noalias noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.6813bbecbd0a1b2869039a79ca820c23.288) #55, !noalias !7262
  unreachable

1312:                                             ; preds = %281
  tail call void @_RNvNtCsgDHHaSskbIF_4core9panicking9panic_fmt(ptr noundef nonnull @anon.6813bbecbd0a1b2869039a79ca820c23.70, ptr noundef nonnull inttoptr (i64 19 to ptr), ptr noalias noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.6813bbecbd0a1b2869039a79ca820c23.289) #55, !noalias !7291
  unreachable

1313:                                             ; preds = %294
  tail call void @_RNvNtCsgDHHaSskbIF_4core9panicking18panic_bounds_check(i64 noundef 0, i64 noundef 0, ptr noalias noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.6813bbecbd0a1b2869039a79ca820c23.290) #55, !noalias !7262
  unreachable

1314:                                             ; preds = %297
  tail call void @_RNvNtCsgDHHaSskbIF_4core9panicking18panic_bounds_check(i64 noundef %288, i64 noundef %292, ptr noalias noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.6813bbecbd0a1b2869039a79ca820c23.291) #55, !noalias !7262
  unreachable

1315:                                             ; preds = %299
  tail call void @_RNvNtCsgDHHaSskbIF_4core9panicking18panic_bounds_check(i64 noundef %290, i64 noundef %292, ptr noalias noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.6813bbecbd0a1b2869039a79ca820c23.292) #55, !noalias !7262
  unreachable

1316:                                             ; preds = %302
  unreachable

1317:                                             ; preds = %1183
  tail call void @_RNvNtNtCsgDHHaSskbIF_4core9panicking11panic_const23panic_const_div_by_zero(ptr noalias noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.6813bbecbd0a1b2869039a79ca820c23.293) #55, !noalias !7262
  unreachable

1318:                                             ; preds = %1223, %220
  br i1 %40, label %1319, label %1520

1319:                                             ; preds = %1318
  %1320 = icmp ult i64 %78, 8
  br i1 %7, label %1415, label %1321

1321:                                             ; preds = %1319
  br i1 %1320, label %1412, label %1322

1322:                                             ; preds = %1321
  %1323 = and i64 %80, 56
  %1324 = getelementptr inbounds nuw i8, ptr %77, i64 16
  %1325 = getelementptr inbounds nuw i8, ptr %77, i64 32
  %1326 = getelementptr inbounds nuw i8, ptr %77, i64 48
  %1327 = load <2 x double>, ptr %77, align 8, !alias.scope !7260, !noalias !7257
  %1328 = load <2 x double>, ptr %1324, align 8, !alias.scope !7260, !noalias !7257
  %1329 = load <2 x double>, ptr %1325, align 8, !alias.scope !7260, !noalias !7257
  %1330 = load <2 x double>, ptr %1326, align 8, !alias.scope !7260, !noalias !7257
  %1331 = load <2 x double>, ptr %41, align 8, !noalias !7257
  %1332 = load <2 x double>, ptr %42, align 8, !noalias !7257
  %1333 = load <2 x double>, ptr %43, align 8, !noalias !7257
  %1334 = load <2 x double>, ptr %44, align 8, !noalias !7257
  %1335 = fmul <2 x double> %1327, %1331
  %1336 = fmul <2 x double> %1328, %1332
  %1337 = fmul <2 x double> %1329, %1333
  %1338 = fmul <2 x double> %1330, %1334
  %1339 = tail call double @llvm.vector.reduce.fadd.v2f64(double %76, <2 x double> %1335)
  %1340 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1339, <2 x double> %1336)
  %1341 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1340, <2 x double> %1337)
  %1342 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1341, <2 x double> %1338)
  %1343 = icmp eq i64 %1323, 8
  br i1 %1343, label %1409, label %1344, !llvm.loop !7295

1344:                                             ; preds = %1322
  %1345 = getelementptr inbounds nuw i8, ptr %77, i64 64
  %1346 = getelementptr inbounds nuw i8, ptr %77, i64 80
  %1347 = getelementptr inbounds nuw i8, ptr %77, i64 96
  %1348 = getelementptr inbounds nuw i8, ptr %77, i64 112
  %1349 = load <2 x double>, ptr %1345, align 8, !alias.scope !7260, !noalias !7257
  %1350 = load <2 x double>, ptr %1346, align 8, !alias.scope !7260, !noalias !7257
  %1351 = load <2 x double>, ptr %1347, align 8, !alias.scope !7260, !noalias !7257
  %1352 = load <2 x double>, ptr %1348, align 8, !alias.scope !7260, !noalias !7257
  %1353 = load <2 x double>, ptr %45, align 8, !noalias !7257
  %1354 = load <2 x double>, ptr %46, align 8, !noalias !7257
  %1355 = load <2 x double>, ptr %47, align 8, !noalias !7257
  %1356 = load <2 x double>, ptr %48, align 8, !noalias !7257
  %1357 = fmul <2 x double> %1349, %1353
  %1358 = fmul <2 x double> %1350, %1354
  %1359 = fmul <2 x double> %1351, %1355
  %1360 = fmul <2 x double> %1352, %1356
  %1361 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1342, <2 x double> %1357)
  %1362 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1361, <2 x double> %1358)
  %1363 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1362, <2 x double> %1359)
  %1364 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1363, <2 x double> %1360)
  %1365 = icmp eq i64 %1323, 16
  br i1 %1365, label %1409, label %1366, !llvm.loop !7295

1366:                                             ; preds = %1344
  %1367 = getelementptr inbounds nuw i8, ptr %77, i64 128
  %1368 = getelementptr inbounds nuw i8, ptr %77, i64 144
  %1369 = getelementptr inbounds nuw i8, ptr %77, i64 160
  %1370 = getelementptr inbounds nuw i8, ptr %77, i64 176
  %1371 = load <2 x double>, ptr %1367, align 8, !alias.scope !7260, !noalias !7257
  %1372 = load <2 x double>, ptr %1368, align 8, !alias.scope !7260, !noalias !7257
  %1373 = load <2 x double>, ptr %1369, align 8, !alias.scope !7260, !noalias !7257
  %1374 = load <2 x double>, ptr %1370, align 8, !alias.scope !7260, !noalias !7257
  %1375 = load <2 x double>, ptr %49, align 8, !noalias !7257
  %1376 = load <2 x double>, ptr %50, align 8, !noalias !7257
  %1377 = load <2 x double>, ptr %51, align 8, !noalias !7257
  %1378 = load <2 x double>, ptr %52, align 8, !noalias !7257
  %1379 = fmul <2 x double> %1371, %1375
  %1380 = fmul <2 x double> %1372, %1376
  %1381 = fmul <2 x double> %1373, %1377
  %1382 = fmul <2 x double> %1374, %1378
  %1383 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1364, <2 x double> %1379)
  %1384 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1383, <2 x double> %1380)
  %1385 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1384, <2 x double> %1381)
  %1386 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1385, <2 x double> %1382)
  %1387 = icmp eq i64 %1323, 24
  br i1 %1387, label %1409, label %1388, !llvm.loop !7295

1388:                                             ; preds = %1366
  %1389 = getelementptr inbounds nuw i8, ptr %77, i64 192
  %1390 = getelementptr inbounds nuw i8, ptr %77, i64 208
  %1391 = getelementptr inbounds nuw i8, ptr %77, i64 224
  %1392 = getelementptr inbounds nuw i8, ptr %77, i64 240
  %1393 = load <2 x double>, ptr %1389, align 8, !alias.scope !7260, !noalias !7257
  %1394 = load <2 x double>, ptr %1390, align 8, !alias.scope !7260, !noalias !7257
  %1395 = load <2 x double>, ptr %1391, align 8, !alias.scope !7260, !noalias !7257
  %1396 = load <2 x double>, ptr %1392, align 8, !alias.scope !7260, !noalias !7257
  %1397 = load <2 x double>, ptr %53, align 8, !noalias !7257
  %1398 = load <2 x double>, ptr %54, align 8, !noalias !7257
  %1399 = load <2 x double>, ptr %55, align 8, !noalias !7257
  %1400 = load <2 x double>, ptr %56, align 8, !noalias !7257
  %1401 = fmul <2 x double> %1393, %1397
  %1402 = fmul <2 x double> %1394, %1398
  %1403 = fmul <2 x double> %1395, %1399
  %1404 = fmul <2 x double> %1396, %1400
  %1405 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1386, <2 x double> %1401)
  %1406 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1405, <2 x double> %1402)
  %1407 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1406, <2 x double> %1403)
  %1408 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1407, <2 x double> %1404)
  br label %1409

1409:                                             ; preds = %1388, %1366, %1344, %1322
  %1410 = phi double [ %1342, %1322 ], [ %1364, %1344 ], [ %1386, %1366 ], [ %1408, %1388 ]
  %1411 = icmp eq i64 %80, %1323
  br i1 %1411, label %72, label %1412

1412:                                             ; preds = %1409, %1321
  %1413 = phi double [ %76, %1321 ], [ %1410, %1409 ]
  %1414 = phi i64 [ 0, %1321 ], [ %1323, %1409 ]
  br label %1521

1415:                                             ; preds = %1319
  br i1 %1320, label %1506, label %1416

1416:                                             ; preds = %1415
  %1417 = and i64 %80, 56
  %1418 = getelementptr inbounds nuw i8, ptr %77, i64 16
  %1419 = getelementptr inbounds nuw i8, ptr %77, i64 32
  %1420 = getelementptr inbounds nuw i8, ptr %77, i64 48
  %1421 = load <2 x double>, ptr %77, align 8, !alias.scope !7260, !noalias !7257
  %1422 = load <2 x double>, ptr %1418, align 8, !alias.scope !7260, !noalias !7257
  %1423 = load <2 x double>, ptr %1419, align 8, !alias.scope !7260, !noalias !7257
  %1424 = load <2 x double>, ptr %1420, align 8, !alias.scope !7260, !noalias !7257
  %1425 = load <2 x double>, ptr %41, align 8, !noalias !7257
  %1426 = load <2 x double>, ptr %57, align 8, !noalias !7257
  %1427 = load <2 x double>, ptr %58, align 8, !noalias !7257
  %1428 = load <2 x double>, ptr %59, align 8, !noalias !7257
  %1429 = fdiv <2 x double> %1421, %1425
  %1430 = fdiv <2 x double> %1422, %1426
  %1431 = fdiv <2 x double> %1423, %1427
  %1432 = fdiv <2 x double> %1424, %1428
  %1433 = tail call double @llvm.vector.reduce.fadd.v2f64(double %76, <2 x double> %1429)
  %1434 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1433, <2 x double> %1430)
  %1435 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1434, <2 x double> %1431)
  %1436 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1435, <2 x double> %1432)
  %1437 = icmp eq i64 %1417, 8
  br i1 %1437, label %1503, label %1438, !llvm.loop !7296

1438:                                             ; preds = %1416
  %1439 = getelementptr inbounds nuw i8, ptr %77, i64 64
  %1440 = getelementptr inbounds nuw i8, ptr %77, i64 80
  %1441 = getelementptr inbounds nuw i8, ptr %77, i64 96
  %1442 = getelementptr inbounds nuw i8, ptr %77, i64 112
  %1443 = load <2 x double>, ptr %1439, align 8, !alias.scope !7260, !noalias !7257
  %1444 = load <2 x double>, ptr %1440, align 8, !alias.scope !7260, !noalias !7257
  %1445 = load <2 x double>, ptr %1441, align 8, !alias.scope !7260, !noalias !7257
  %1446 = load <2 x double>, ptr %1442, align 8, !alias.scope !7260, !noalias !7257
  %1447 = load <2 x double>, ptr %60, align 8, !noalias !7257
  %1448 = load <2 x double>, ptr %61, align 8, !noalias !7257
  %1449 = load <2 x double>, ptr %62, align 8, !noalias !7257
  %1450 = load <2 x double>, ptr %63, align 8, !noalias !7257
  %1451 = fdiv <2 x double> %1443, %1447
  %1452 = fdiv <2 x double> %1444, %1448
  %1453 = fdiv <2 x double> %1445, %1449
  %1454 = fdiv <2 x double> %1446, %1450
  %1455 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1436, <2 x double> %1451)
  %1456 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1455, <2 x double> %1452)
  %1457 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1456, <2 x double> %1453)
  %1458 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1457, <2 x double> %1454)
  %1459 = icmp eq i64 %1417, 16
  br i1 %1459, label %1503, label %1460, !llvm.loop !7296

1460:                                             ; preds = %1438
  %1461 = getelementptr inbounds nuw i8, ptr %77, i64 128
  %1462 = getelementptr inbounds nuw i8, ptr %77, i64 144
  %1463 = getelementptr inbounds nuw i8, ptr %77, i64 160
  %1464 = getelementptr inbounds nuw i8, ptr %77, i64 176
  %1465 = load <2 x double>, ptr %1461, align 8, !alias.scope !7260, !noalias !7257
  %1466 = load <2 x double>, ptr %1462, align 8, !alias.scope !7260, !noalias !7257
  %1467 = load <2 x double>, ptr %1463, align 8, !alias.scope !7260, !noalias !7257
  %1468 = load <2 x double>, ptr %1464, align 8, !alias.scope !7260, !noalias !7257
  %1469 = load <2 x double>, ptr %64, align 8, !noalias !7257
  %1470 = load <2 x double>, ptr %65, align 8, !noalias !7257
  %1471 = load <2 x double>, ptr %66, align 8, !noalias !7257
  %1472 = load <2 x double>, ptr %67, align 8, !noalias !7257
  %1473 = fdiv <2 x double> %1465, %1469
  %1474 = fdiv <2 x double> %1466, %1470
  %1475 = fdiv <2 x double> %1467, %1471
  %1476 = fdiv <2 x double> %1468, %1472
  %1477 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1458, <2 x double> %1473)
  %1478 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1477, <2 x double> %1474)
  %1479 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1478, <2 x double> %1475)
  %1480 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1479, <2 x double> %1476)
  %1481 = icmp eq i64 %1417, 24
  br i1 %1481, label %1503, label %1482, !llvm.loop !7296

1482:                                             ; preds = %1460
  %1483 = getelementptr inbounds nuw i8, ptr %77, i64 192
  %1484 = getelementptr inbounds nuw i8, ptr %77, i64 208
  %1485 = getelementptr inbounds nuw i8, ptr %77, i64 224
  %1486 = getelementptr inbounds nuw i8, ptr %77, i64 240
  %1487 = load <2 x double>, ptr %1483, align 8, !alias.scope !7260, !noalias !7257
  %1488 = load <2 x double>, ptr %1484, align 8, !alias.scope !7260, !noalias !7257
  %1489 = load <2 x double>, ptr %1485, align 8, !alias.scope !7260, !noalias !7257
  %1490 = load <2 x double>, ptr %1486, align 8, !alias.scope !7260, !noalias !7257
  %1491 = load <2 x double>, ptr %68, align 8, !noalias !7257
  %1492 = load <2 x double>, ptr %69, align 8, !noalias !7257
  %1493 = load <2 x double>, ptr %70, align 8, !noalias !7257
  %1494 = load <2 x double>, ptr %71, align 8, !noalias !7257
  %1495 = fdiv <2 x double> %1487, %1491
  %1496 = fdiv <2 x double> %1488, %1492
  %1497 = fdiv <2 x double> %1489, %1493
  %1498 = fdiv <2 x double> %1490, %1494
  %1499 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1480, <2 x double> %1495)
  %1500 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1499, <2 x double> %1496)
  %1501 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1500, <2 x double> %1497)
  %1502 = tail call double @llvm.vector.reduce.fadd.v2f64(double %1501, <2 x double> %1498)
  br label %1503

1503:                                             ; preds = %1482, %1460, %1438, %1416
  %1504 = phi double [ %1436, %1416 ], [ %1458, %1438 ], [ %1480, %1460 ], [ %1502, %1482 ]
  %1505 = icmp eq i64 %80, %1417
  br i1 %1505, label %72, label %1506

1506:                                             ; preds = %1503, %1415
  %1507 = phi double [ %76, %1415 ], [ %1504, %1503 ]
  %1508 = phi i64 [ 0, %1415 ], [ %1417, %1503 ]
  br label %1509

1509:                                             ; preds = %1509, %1506
  %1510 = phi double [ %1518, %1509 ], [ %1507, %1506 ]
  %1511 = phi i64 [ %1512, %1509 ], [ %1508, %1506 ]
  %1512 = add nuw i64 %1511, 1
  %1513 = getelementptr inbounds nuw double, ptr %77, i64 %1511
  %1514 = getelementptr inbounds nuw i64, ptr %41, i64 %1511
  %1515 = load double, ptr %1513, align 8, !alias.scope !7260, !noalias !7257, !noundef !4
  %1516 = load double, ptr %1514, align 8, !noalias !7257, !noundef !4
  %1517 = fdiv double %1515, %1516
  %1518 = fadd double %1510, %1517
  %1519 = icmp eq i64 %1512, %80
  br i1 %1519, label %72, label %1509, !llvm.loop !7297

1520:                                             ; preds = %1318
  tail call void @_RNvNtCsgDHHaSskbIF_4core9panicking18panic_bounds_check(i64 noundef %39, i64 noundef %24, ptr noalias noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.6813bbecbd0a1b2869039a79ca820c23.287) #55, !noalias !7257
  unreachable

1521:                                             ; preds = %1521, %1412
  %1522 = phi double [ %1530, %1521 ], [ %1413, %1412 ]
  %1523 = phi i64 [ %1524, %1521 ], [ %1414, %1412 ]
  %1524 = add nuw i64 %1523, 1
  %1525 = getelementptr inbounds nuw double, ptr %77, i64 %1523
  %1526 = getelementptr inbounds nuw i64, ptr %41, i64 %1523
  %1527 = load double, ptr %1525, align 8, !alias.scope !7260, !noalias !7257, !noundef !4
  %1528 = load double, ptr %1526, align 8, !noalias !7257, !noundef !4
  %1529 = fmul double %1527, %1528
  %1530 = fadd double %1522, %1529
  %1531 = icmp eq i64 %1524, %80
  br i1 %1531, label %72, label %1521, !llvm.loop !7298

1532:                                             ; preds = %72, %14
  %1533 = phi double [ 0.000000e+00, %14 ], [ %73, %72 ]
  %1534 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store double %1533, ptr %1534, align 8
  store i64 -9223372036854775808, ptr %0, align 8
  br label %1585

1535:                                             ; preds = %8
  %1536 = shl nuw nsw i64 %5, 3
  %1537 = getelementptr inbounds nuw i8, ptr %4, i64 %1536
  br i1 %13, label %1593, label %1538

1538:                                             ; preds = %1535
  %1539 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %1540 = getelementptr inbounds nuw i8, ptr %9, i64 8
  br i1 %7, label %1541, label %1570

1541:                                             ; preds = %1538
  br i1 %6, label %1542, label %1556

1542:                                             ; preds = %1549, %1541
  %1543 = phi double [ %1554, %1549 ], [ 0.000000e+00, %1541 ]
  %1544 = phi ptr [ %1551, %1549 ], [ %4, %1541 ]
  %1545 = phi i64 [ %1550, %1549 ], [ 0, %1541 ]
  %1546 = load double, ptr %1544, align 8, !noundef !4
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  store i64 %1545, ptr %10, align 8
  store i64 %3, ptr %1539, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %9)
  call fastcc void @_RNvMNtNtNtCscPDRiDoTWel_8mech_gpu8sequence7runtime8residentNtB2_10ScalarTape8evaluate(ptr noalias noundef align 8 captures(none) dereferenceable(24) %9, ptr noalias noundef align 8 dereferenceable(88) %1, ptr noalias noundef nonnull readonly align 8 captures(address, read_provenance) %10)
  %1547 = load i64, ptr %9, align 8, !range !3, !noundef !4
  %1548 = icmp eq i64 %1547, -9223372036854775808
  br i1 %1548, label %1549, label %1596

1549:                                             ; preds = %1542
  %1550 = add nuw nsw i64 %1545, 1
  %1551 = getelementptr inbounds nuw i8, ptr %1544, i64 8
  %1552 = load double, ptr %1540, align 8, !noundef !4
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  %1553 = fdiv double %1546, %1552
  %1554 = fadd double %1543, %1553
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1555 = icmp eq ptr %1551, %1537
  br i1 %1555, label %1593, label %1542

1556:                                             ; preds = %1563, %1541
  %1557 = phi double [ %1568, %1563 ], [ 0.000000e+00, %1541 ]
  %1558 = phi ptr [ %1565, %1563 ], [ %4, %1541 ]
  %1559 = phi i64 [ %1564, %1563 ], [ 0, %1541 ]
  %1560 = load double, ptr %1558, align 8, !noundef !4
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  store i64 %3, ptr %10, align 8
  store i64 %1559, ptr %1539, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %9)
  call fastcc void @_RNvMNtNtNtCscPDRiDoTWel_8mech_gpu8sequence7runtime8residentNtB2_10ScalarTape8evaluate(ptr noalias noundef align 8 captures(none) dereferenceable(24) %9, ptr noalias noundef align 8 dereferenceable(88) %1, ptr noalias noundef nonnull readonly align 8 captures(address, read_provenance) %10)
  %1561 = load i64, ptr %9, align 8, !range !3, !noundef !4
  %1562 = icmp eq i64 %1561, -9223372036854775808
  br i1 %1562, label %1563, label %1596

1563:                                             ; preds = %1556
  %1564 = add nuw nsw i64 %1559, 1
  %1565 = getelementptr inbounds nuw i8, ptr %1558, i64 8
  %1566 = load double, ptr %1540, align 8, !noundef !4
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  %1567 = fdiv double %1560, %1566
  %1568 = fadd double %1557, %1567
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1569 = icmp eq ptr %1565, %1537
  br i1 %1569, label %1593, label %1556

1570:                                             ; preds = %1538
  br i1 %6, label %1571, label %1586

1571:                                             ; preds = %1578, %1570
  %1572 = phi double [ %1583, %1578 ], [ 0.000000e+00, %1570 ]
  %1573 = phi ptr [ %1580, %1578 ], [ %4, %1570 ]
  %1574 = phi i64 [ %1579, %1578 ], [ 0, %1570 ]
  %1575 = load double, ptr %1573, align 8, !noundef !4
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  store i64 %1574, ptr %10, align 8
  store i64 %3, ptr %1539, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %9)
  call fastcc void @_RNvMNtNtNtCscPDRiDoTWel_8mech_gpu8sequence7runtime8residentNtB2_10ScalarTape8evaluate(ptr noalias noundef align 8 captures(none) dereferenceable(24) %9, ptr noalias noundef align 8 dereferenceable(88) %1, ptr noalias noundef nonnull readonly align 8 captures(address, read_provenance) %10)
  %1576 = load i64, ptr %9, align 8, !range !3, !noundef !4
  %1577 = icmp eq i64 %1576, -9223372036854775808
  br i1 %1577, label %1578, label %1596

1578:                                             ; preds = %1571
  %1579 = add nuw nsw i64 %1574, 1
  %1580 = getelementptr inbounds nuw i8, ptr %1573, i64 8
  %1581 = load double, ptr %1540, align 8, !noundef !4
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  %1582 = fmul double %1575, %1581
  %1583 = fadd double %1572, %1582
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1584 = icmp eq ptr %1580, %1537
  br i1 %1584, label %1593, label %1571

1585:                                             ; preds = %1596, %1593, %1532
  ret void

1586:                                             ; preds = %1603, %1570
  %1587 = phi double [ %1608, %1603 ], [ 0.000000e+00, %1570 ]
  %1588 = phi ptr [ %1605, %1603 ], [ %4, %1570 ]
  %1589 = phi i64 [ %1604, %1603 ], [ 0, %1570 ]
  %1590 = load double, ptr %1588, align 8, !noundef !4
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  store i64 %3, ptr %10, align 8
  store i64 %1589, ptr %1539, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %9)
  call fastcc void @_RNvMNtNtNtCscPDRiDoTWel_8mech_gpu8sequence7runtime8residentNtB2_10ScalarTape8evaluate(ptr noalias noundef align 8 captures(none) dereferenceable(24) %9, ptr noalias noundef align 8 dereferenceable(88) %1, ptr noalias noundef nonnull readonly align 8 captures(address, read_provenance) %10)
  %1591 = load i64, ptr %9, align 8, !range !3, !noundef !4
  %1592 = icmp eq i64 %1591, -9223372036854775808
  br i1 %1592, label %1603, label %1596

1593:                                             ; preds = %1603, %1578, %1563, %1549, %1535
  %1594 = phi double [ 0.000000e+00, %1535 ], [ %1583, %1578 ], [ %1568, %1563 ], [ %1554, %1549 ], [ %1608, %1603 ]
  %1595 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store double %1594, ptr %1595, align 8
  store i64 -9223372036854775808, ptr %0, align 8
  br label %1585

1596:                                             ; preds = %1586, %1571, %1556, %1542
  %1597 = phi i64 [ %1576, %1571 ], [ %1561, %1556 ], [ %1547, %1542 ], [ %1591, %1586 ]
  %1598 = load double, ptr %1540, align 8
  %1599 = getelementptr inbounds nuw i8, ptr %9, i64 16
  %1600 = load i64, ptr %1599, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  store i64 %1597, ptr %0, align 8
  %1601 = getelementptr inbounds nuw i8, ptr %0, i64 8
  store double %1598, ptr %1601, align 8
  %1602 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %1600, ptr %1602, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  br label %1585

1603:                                             ; preds = %1586
  %1604 = add nuw nsw i64 %1589, 1
  %1605 = getelementptr inbounds nuw i8, ptr %1588, i64 8
  %1606 = load double, ptr %1540, align 8, !noundef !4
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  %1607 = fmul double %1590, %1606
  %1608 = fadd double %1587, %1607
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1609 = icmp eq ptr %1605, %1537
  br i1 %1609, label %1593, label %1586
}
