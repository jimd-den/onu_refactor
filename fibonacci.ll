; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [5 x i8] c"%lld\00", align 1
@strtmp.1 = private unnamed_addr constant [30 x i8] c"The population at generation \00", align 1
@strtmp.2 = private unnamed_addr constant [30 x i8] c"The population at generation \00", align 1
@strtmp.3 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@strtmp.4 = private unnamed_addr constant [15 x i8] c" has reached: \00", align 1
@strtmp.5 = private unnamed_addr constant [15 x i8] c" has reached: \00", align 1
@strtmp.6 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@strtmp.7 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define i64 @origin-size() {
bb0:
  ret i64 0
}

define i64 @spark-size() {
bb0:
  ret i64 1
}

define i64 @calculate-growth(i64 %0) {
bb0:
  %v14 = alloca i64, align 8
  %v13 = alloca i64, align 8
  %v12 = alloca i64, align 8
  %v1120 = alloca i64, align 8
  %v10 = alloca i64, align 8
  %v9 = alloca i64, align 8
  %v8 = alloca i64, align 8
  %v7 = alloca i64, align 8
  %v6 = alloca i64, align 8
  %v5 = alloca i64, align 8
  %v4 = alloca i64, align 8
  %v3 = alloca i64, align 8
  %v2 = alloca i64, align 8
  %v1 = alloca i64, align 8
  %generation = alloca i64, align 8
  store i64 %0, i64* %generation, align 4
  %v0 = load i64, i64* %generation, align 4
  %cmptmp = icmp eq i64 %v0, 0
  %booltmp = zext i1 %cmptmp to i64
  store i64 %booltmp, i64* %v1, align 4
  %v11 = load i64, i64* %v1, align 4
  %bool_cast = icmp ne i64 %v11, 0
  br i1 %bool_cast, label %bb1, label %bb2

bb1:                                              ; preds = %bb0
  %calltmp = call i64 @origin-size()
  store i64 %calltmp, i64* %v2, align 4
  %v22 = load i64, i64* %v2, align 4
  ret i64 %v22

bb2:                                              ; preds = %bb0
  %v03 = load i64, i64* %generation, align 4
  %subtmp = sub i64 %v03, 1
  store i64 %subtmp, i64* %v3, align 4
  %v34 = load i64, i64* %v3, align 4
  %cmptmp5 = icmp eq i64 %v34, 0
  %booltmp6 = zext i1 %cmptmp5 to i64
  store i64 %booltmp6, i64* %v4, align 4
  %v47 = load i64, i64* %v4, align 4
  %bool_cast8 = icmp ne i64 %v47, 0
  br i1 %bool_cast8, label %bb3, label %bb4

bb3:                                              ; preds = %bb2
  %calltmp9 = call i64 @spark-size()
  store i64 %calltmp9, i64* %v5, align 4
  %v510 = load i64, i64* %v5, align 4
  ret i64 %v510

bb4:                                              ; preds = %bb2
  %v011 = load i64, i64* %generation, align 4
  %subtmp12 = sub i64 %v011, 1
  store i64 %subtmp12, i64* %v6, align 4
  %v613 = load i64, i64* %v6, align 4
  store i64 %v613, i64* %v7, align 4
  %v014 = load i64, i64* %generation, align 4
  %subtmp15 = sub i64 %v014, 2
  store i64 %subtmp15, i64* %v8, align 4
  %v816 = load i64, i64* %v8, align 4
  store i64 %v816, i64* %v9, align 4
  %v717 = load i64, i64* %v7, align 4
  %calltmp18 = call i64 @calculate-growth(i64 %v717)
  store i64 %calltmp18, i64* %v10, align 4
  %v1019 = load i64, i64* %v10, align 4
  store i64 %v1019, i64* %v1120, align 4
  %v921 = load i64, i64* %v9, align 4
  %calltmp22 = call i64 @calculate-growth(i64 %v921)
  store i64 %calltmp22, i64* %v12, align 4
  %v1223 = load i64, i64* %v12, align 4
  store i64 %v1223, i64* %v13, align 4
  %v1124 = load i64, i64* %v1120, align 4
  %v1325 = load i64, i64* %v13, align 4
  %addtmp = add i64 %v1124, %v1325
  store i64 %addtmp, i64* %v14, align 4
  ret i64 0
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %v39 = alloca { i64, i8*, i1 }, align 8
  %v45 = alloca i64, align 8
  %v44 = alloca i32, align 4
  %v43 = alloca i8*, align 8
  %v42 = alloca { i64, i8*, i1 }, align 8
  %v41 = alloca i8*, align 8
  %v40 = alloca i64, align 8
  %v38 = alloca i64, align 8
  %v37 = alloca { i64, i8*, i1 }, align 8
  %v25 = alloca { i64, i8*, i1 }, align 8
  %v36 = alloca i8*, align 8
  %v35 = alloca { i64, i8*, i1 }, align 8
  %v34 = alloca i8*, align 8
  %v33 = alloca i8*, align 8
  %v3257 = alloca i8*, align 8
  %v31 = alloca i64, align 8
  %v30 = alloca i8*, align 8
  %v29 = alloca i8*, align 8
  %v28 = alloca i64, align 8
  %v2747 = alloca i64, align 8
  %v26 = alloca i64, align 8
  %v24 = alloca { i64, i8*, i1 }, align 8
  %v12 = alloca { i64, i8*, i1 }, align 8
  %v23 = alloca i8*, align 8
  %v22 = alloca { i64, i8*, i1 }, align 8
  %v2134 = alloca i8*, align 8
  %v20 = alloca i8*, align 8
  %v19 = alloca i8*, align 8
  %v18 = alloca i64, align 8
  %v17 = alloca i8*, align 8
  %v16 = alloca i8*, align 8
  %v15 = alloca i64, align 8
  %v14 = alloca i64, align 8
  %v13 = alloca i64, align 8
  %v5 = alloca { i64, i8*, i1 }, align 8
  %v11 = alloca i64, align 8
  %v10 = alloca i32, align 4
  %v9 = alloca i8*, align 8
  %v8 = alloca { i64, i8*, i1 }, align 8
  %v7 = alloca i8*, align 8
  %v6 = alloca i64, align 8
  %v4 = alloca i64, align 8
  %v3 = alloca i64, align 8
  %v2 = alloca i64, align 8
  %__argc = alloca i32, align 4
  store i32 %0, i32* %__argc, align 4
  %__argv = alloca i64, align 8
  store i64 %1, i64* %__argv, align 4
  store i64 40, i64* %v2, align 4
  %v21 = load i64, i64* %v2, align 4
  %calltmp = call i64 @calculate-growth(i64 %v21)
  store i64 %calltmp, i64* %v3, align 4
  %v32 = load i64, i64* %v3, align 4
  store i64 %v32, i64* %v4, align 4
  store i64 32, i64* %v6, align 4
  %v63 = load i64, i64* %v6, align 4
  %malloc_call = call i8* @malloc(i64 %v63)
  store i8* %malloc_call, i8** %v7, align 8
  store { i64, i8*, i1 } { i64 4, i8* getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v8, align 8
  %v84 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v8, align 8
  %index_tmp = extractvalue { i64, i8*, i1 } %v84, 1
  store i8* %index_tmp, i8** %v9, align 8
  %v75 = load i8*, i8** %v7, align 8
  %v96 = load i8*, i8** %v9, align 8
  %v27 = load i64, i64* %v2, align 4
  %calltmp8 = call i32 (i8*, i8*, ...) @sprintf(i8* %v75, i8* %v96, i64 %v27)
  store i32 %calltmp8, i32* %v10, align 4
  %v79 = load i8*, i8** %v7, align 8
  %calltmp10 = call i64 @strlen(i8* %v79)
  store i64 %calltmp10, i64* %v11, align 4
  %v1111 = load i64, i64* %v11, align 4
  %v712 = load i8*, i8** %v7, align 8
  %insert_0 = insertvalue { i64, i8*, i1 } undef, i64 %v1111, 0
  %insert_1 = insertvalue { i64, i8*, i1 } %insert_0, i8* %v712, 1
  %insert_2 = insertvalue { i64, i8*, i1 } %insert_1, i1 true, 2
  store { i64, i8*, i1 } %insert_2, { i64, i8*, i1 }* %v5, align 8
  store i64 29, i64* %v13, align 4
  %v513 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v5, align 8
  %index_tmp14 = extractvalue { i64, i8*, i1 } %v513, 0
  store i64 %index_tmp14, i64* %v14, align 4
  %v1315 = load i64, i64* %v13, align 4
  %v1416 = load i64, i64* %v14, align 4
  %addtmp = add i64 %v1315, %v1416
  store i64 %addtmp, i64* %v15, align 4
  store i8* getelementptr inbounds ([30 x i8], [30 x i8]* @strtmp.2, i32 0, i32 0), i8** %v16, align 8
  %v517 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v5, align 8
  %index_tmp18 = extractvalue { i64, i8*, i1 } %v517, 1
  store i8* %index_tmp18, i8** %v17, align 8
  %v1519 = load i64, i64* %v15, align 4
  %addtmp20 = add i64 %v1519, 1
  store i64 %addtmp20, i64* %v18, align 4
  %v1821 = load i64, i64* %v18, align 4
  %malloc_call22 = call i8* @malloc(i64 %v1821)
  store i8* %malloc_call22, i8** %v19, align 8
  %v1923 = load i8*, i8** %v19, align 8
  %v1624 = load i8*, i8** %v16, align 8
  %v1325 = load i64, i64* %v13, align 4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v1923, i8* %v1624, i64 %v1325, i1 false)
  %v1926 = load i8*, i8** %v19, align 8
  %v1327 = load i64, i64* %v13, align 4
  %offset_ptr = getelementptr inbounds i8, i8* %v1926, i64 %v1327
  store i8* %offset_ptr, i8** %v20, align 8
  %v2028 = load i8*, i8** %v20, align 8
  %v1729 = load i8*, i8** %v17, align 8
  %v1430 = load i64, i64* %v14, align 4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v2028, i8* %v1729, i64 %v1430, i1 false)
  %v1931 = load i8*, i8** %v19, align 8
  %v1532 = load i64, i64* %v15, align 4
  %offset_ptr33 = getelementptr inbounds i8, i8* %v1931, i64 %v1532
  store i8* %offset_ptr33, i8** %v2134, align 8
  store { i64, i8*, i1 } { i64 0, i8* getelementptr inbounds ([1 x i8], [1 x i8]* @strtmp.3, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v22, align 8
  %v2235 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v22, align 8
  %index_tmp36 = extractvalue { i64, i8*, i1 } %v2235, 1
  store i8* %index_tmp36, i8** %v23, align 8
  %v2137 = load i8*, i8** %v2134, align 8
  %v2338 = load i8*, i8** %v23, align 8
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v2137, i8* %v2338, i64 1, i1 false)
  %v1539 = load i64, i64* %v15, align 4
  %v1940 = load i8*, i8** %v19, align 8
  %insert_041 = insertvalue { i64, i8*, i1 } undef, i64 %v1539, 0
  %insert_142 = insertvalue { i64, i8*, i1 } %insert_041, i8* %v1940, 1
  %insert_243 = insertvalue { i64, i8*, i1 } %insert_142, i1 true, 2
  store { i64, i8*, i1 } %insert_243, { i64, i8*, i1 }* %v12, align 8
  %v1244 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v12, align 8
  store { i64, i8*, i1 } %v1244, { i64, i8*, i1 }* %v24, align 8
  %v2445 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v24, align 8
  %index_tmp46 = extractvalue { i64, i8*, i1 } %v2445, 0
  store i64 %index_tmp46, i64* %v26, align 4
  store i64 14, i64* %v2747, align 4
  %v2648 = load i64, i64* %v26, align 4
  %v2749 = load i64, i64* %v2747, align 4
  %addtmp50 = add i64 %v2648, %v2749
  store i64 %addtmp50, i64* %v28, align 4
  %v2451 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v24, align 8
  %index_tmp52 = extractvalue { i64, i8*, i1 } %v2451, 1
  store i8* %index_tmp52, i8** %v29, align 8
  store i8* getelementptr inbounds ([15 x i8], [15 x i8]* @strtmp.5, i32 0, i32 0), i8** %v30, align 8
  %v2853 = load i64, i64* %v28, align 4
  %addtmp54 = add i64 %v2853, 1
  store i64 %addtmp54, i64* %v31, align 4
  %v3155 = load i64, i64* %v31, align 4
  %malloc_call56 = call i8* @malloc(i64 %v3155)
  store i8* %malloc_call56, i8** %v3257, align 8
  %v3258 = load i8*, i8** %v3257, align 8
  %v2959 = load i8*, i8** %v29, align 8
  %v2660 = load i64, i64* %v26, align 4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v3258, i8* %v2959, i64 %v2660, i1 false)
  %v3261 = load i8*, i8** %v3257, align 8
  %v2662 = load i64, i64* %v26, align 4
  %offset_ptr63 = getelementptr inbounds i8, i8* %v3261, i64 %v2662
  store i8* %offset_ptr63, i8** %v33, align 8
  %v3364 = load i8*, i8** %v33, align 8
  %v3065 = load i8*, i8** %v30, align 8
  %v2766 = load i64, i64* %v2747, align 4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v3364, i8* %v3065, i64 %v2766, i1 false)
  %v3267 = load i8*, i8** %v3257, align 8
  %v2868 = load i64, i64* %v28, align 4
  %offset_ptr69 = getelementptr inbounds i8, i8* %v3267, i64 %v2868
  store i8* %offset_ptr69, i8** %v34, align 8
  store { i64, i8*, i1 } { i64 0, i8* getelementptr inbounds ([1 x i8], [1 x i8]* @strtmp.6, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v35, align 8
  %v3570 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v35, align 8
  %index_tmp71 = extractvalue { i64, i8*, i1 } %v3570, 1
  store i8* %index_tmp71, i8** %v36, align 8
  %v3472 = load i8*, i8** %v34, align 8
  %v3673 = load i8*, i8** %v36, align 8
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v3472, i8* %v3673, i64 1, i1 false)
  %v2874 = load i64, i64* %v28, align 4
  %v3275 = load i8*, i8** %v3257, align 8
  %insert_076 = insertvalue { i64, i8*, i1 } undef, i64 %v2874, 0
  %insert_177 = insertvalue { i64, i8*, i1 } %insert_076, i8* %v3275, 1
  %insert_278 = insertvalue { i64, i8*, i1 } %insert_177, i1 true, 2
  store { i64, i8*, i1 } %insert_278, { i64, i8*, i1 }* %v25, align 8
  %v2579 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v25, align 8
  store { i64, i8*, i1 } %v2579, { i64, i8*, i1 }* %v37, align 8
  store i64 0, i64* %v38, align 4
  %v3780 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v37, align 8
  %raw_ptr = extractvalue { i64, i8*, i1 } %v3780, 1
  %emit = call i32 @puts(i8* %raw_ptr)
  store i64 32, i64* %v40, align 4
  %v4081 = load i64, i64* %v40, align 4
  %malloc_call82 = call i8* @malloc(i64 %v4081)
  store i8* %malloc_call82, i8** %v41, align 8
  store { i64, i8*, i1 } { i64 4, i8* getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.7, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v42, align 8
  %v4283 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v42, align 8
  %index_tmp84 = extractvalue { i64, i8*, i1 } %v4283, 1
  store i8* %index_tmp84, i8** %v43, align 8
  %v4185 = load i8*, i8** %v41, align 8
  %v4386 = load i8*, i8** %v43, align 8
  %v487 = load i64, i64* %v4, align 4
  %calltmp88 = call i32 (i8*, i8*, ...) @sprintf(i8* %v4185, i8* %v4386, i64 %v487)
  store i32 %calltmp88, i32* %v44, align 4
  %v4189 = load i8*, i8** %v41, align 8
  %calltmp90 = call i64 @strlen(i8* %v4189)
  store i64 %calltmp90, i64* %v45, align 4
  %v4591 = load i64, i64* %v45, align 4
  %v4192 = load i8*, i8** %v41, align 8
  %insert_093 = insertvalue { i64, i8*, i1 } undef, i64 %v4591, 0
  %insert_194 = insertvalue { i64, i8*, i1 } %insert_093, i8* %v4192, 1
  %insert_295 = insertvalue { i64, i8*, i1 } %insert_194, i1 true, 2
  store { i64, i8*, i1 } %insert_295, { i64, i8*, i1 }* %v39, align 8
  %v3996 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v39, align 8
  %raw_ptr97 = extractvalue { i64, i8*, i1 } %v3996, 1
  %emit98 = call i32 @puts(i8* %raw_ptr97)
  ret i32 0
}

; Function Attrs: argmemonly nofree nounwind willreturn
declare void @llvm.memcpy.p0i8.p0i8.i64(i8* noalias nocapture writeonly, i8* noalias nocapture readonly, i64, i1 immarg) #0

attributes #0 = { argmemonly nofree nounwind willreturn }
