; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [21 x i8] c"PARITY VERIFICATION:\00", align 1
@strtmp.1 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1
@strtmp.2 = private unnamed_addr constant [22 x i8] c"Is 10 even? (1=yes): \00", align 1
@strtmp.3 = private unnamed_addr constant [22 x i8] c"Is 10 even? (1=yes): \00", align 1
@strtmp.4 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@strtmp.5 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1
@strtmp.6 = private unnamed_addr constant [22 x i8] c"Is 7 even?  (1=yes): \00", align 1
@strtmp.7 = private unnamed_addr constant [22 x i8] c"Is 7 even?  (1=yes): \00", align 1
@strtmp.8 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define i64 @is-even(i64 %0) {
bb0:
  %v3 = alloca i64, align 8
  %v2 = alloca i64, align 8
  %v1 = alloca i64, align 8
  %n = alloca i64, align 8
  store i64 %0, i64* %n, align 4
  %v0 = load i64, i64* %n, align 4
  %cmptmp = icmp eq i64 %v0, 0
  %booltmp = zext i1 %cmptmp to i64
  store i64 %booltmp, i64* %v1, align 4
  %v11 = load i64, i64* %v1, align 4
  %bool_cast = icmp ne i64 %v11, 0
  br i1 %bool_cast, label %bb1, label %bb2

bb1:                                              ; preds = %bb0
  ret i64 1

bb2:                                              ; preds = %bb0
  %v02 = load i64, i64* %n, align 4
  %subtmp = sub i64 %v02, 1
  store i64 %subtmp, i64* %v2, align 4
  %v23 = load i64, i64* %v2, align 4
  %calltmp = call i64 @is-odd(i64 %v23)
  store i64 %calltmp, i64* %v3, align 4
  ret i64 0
}

define i64 @is-odd(i64 %0) {
bb0:
  %v3 = alloca i64, align 8
  %v2 = alloca i64, align 8
  %v1 = alloca i64, align 8
  %n = alloca i64, align 8
  store i64 %0, i64* %n, align 4
  %v0 = load i64, i64* %n, align 4
  %cmptmp = icmp eq i64 %v0, 0
  %booltmp = zext i1 %cmptmp to i64
  store i64 %booltmp, i64* %v1, align 4
  %v11 = load i64, i64* %v1, align 4
  %bool_cast = icmp ne i64 %v11, 0
  br i1 %bool_cast, label %bb1, label %bb2

bb1:                                              ; preds = %bb0
  ret i64 0

bb2:                                              ; preds = %bb0
  %v02 = load i64, i64* %n, align 4
  %subtmp = sub i64 %v02, 1
  store i64 %subtmp, i64* %v2, align 4
  %v23 = load i64, i64* %v2, align 4
  %calltmp = call i64 @is-even(i64 %v23)
  store i64 %calltmp, i64* %v3, align 4
  ret i64 0
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %v50 = alloca i64, align 8
  %v49 = alloca { i64, i8*, i1 }, align 8
  %v37 = alloca { i64, i8*, i1 }, align 8
  %v48 = alloca i8*, align 8
  %v47 = alloca { i64, i8*, i1 }, align 8
  %v46 = alloca i8*, align 8
  %v45 = alloca i8*, align 8
  %v44 = alloca i8*, align 8
  %v43 = alloca i64, align 8
  %v4273 = alloca i8*, align 8
  %v41 = alloca i8*, align 8
  %v40 = alloca i64, align 8
  %v39 = alloca i64, align 8
  %v38 = alloca i64, align 8
  %v30 = alloca { i64, i8*, i1 }, align 8
  %v36 = alloca i64, align 8
  %v35 = alloca i32, align 4
  %v34 = alloca i8*, align 8
  %v3352 = alloca { i64, i8*, i1 }, align 8
  %v32 = alloca i8*, align 8
  %v31 = alloca i64, align 8
  %v29 = alloca i64, align 8
  %v28 = alloca { i64, i8*, i1 }, align 8
  %v16 = alloca { i64, i8*, i1 }, align 8
  %v27 = alloca i8*, align 8
  %v26 = alloca { i64, i8*, i1 }, align 8
  %v25 = alloca i8*, align 8
  %v24 = alloca i8*, align 8
  %v23 = alloca i8*, align 8
  %v22 = alloca i64, align 8
  %v2122 = alloca i8*, align 8
  %v20 = alloca i8*, align 8
  %v19 = alloca i64, align 8
  %v18 = alloca i64, align 8
  %v17 = alloca i64, align 8
  %v9 = alloca { i64, i8*, i1 }, align 8
  %v15 = alloca i64, align 8
  %v14 = alloca i32, align 4
  %v13 = alloca i8*, align 8
  %v12 = alloca { i64, i8*, i1 }, align 8
  %v11 = alloca i8*, align 8
  %v10 = alloca i64, align 8
  %v8 = alloca i64, align 8
  %v7 = alloca i64, align 8
  %v6 = alloca i64, align 8
  %v5 = alloca i64, align 8
  %v4 = alloca i64, align 8
  %v3 = alloca i64, align 8
  %v2 = alloca i64, align 8
  %__argc = alloca i32, align 4
  store i32 %0, i32* %__argc, align 4
  %__argv = alloca i64, align 8
  store i64 %1, i64* %__argv, align 4
  store i64 10, i64* %v2, align 4
  store i64 7, i64* %v3, align 4
  %v21 = load i64, i64* %v2, align 4
  %calltmp = call i64 @is-even(i64 %v21)
  store i64 %calltmp, i64* %v4, align 4
  %v42 = load i64, i64* %v4, align 4
  store i64 %v42, i64* %v5, align 4
  %v33 = load i64, i64* %v3, align 4
  %calltmp4 = call i64 @is-even(i64 %v33)
  store i64 %calltmp4, i64* %v6, align 4
  %v65 = load i64, i64* %v6, align 4
  store i64 %v65, i64* %v7, align 4
  store i64 0, i64* %v8, align 4
  %emit = call i32 @puts(i8* getelementptr inbounds ([21 x i8], [21 x i8]* @strtmp, i32 0, i32 0))
  store i64 32, i64* %v10, align 4
  %v106 = load i64, i64* %v10, align 4
  %malloc_call = call i8* @malloc(i64 %v106)
  store i8* %malloc_call, i8** %v11, align 8
  store { i64, i8*, i1 } { i64 4, i8* getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.1, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v12, align 8
  %v127 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v12, align 8
  %index_tmp = extractvalue { i64, i8*, i1 } %v127, 1
  store i8* %index_tmp, i8** %v13, align 8
  %v118 = load i8*, i8** %v11, align 8
  %v139 = load i8*, i8** %v13, align 8
  %v510 = load i64, i64* %v5, align 4
  %calltmp11 = call i32 (i8*, i8*, ...) @sprintf(i8* %v118, i8* %v139, i64 %v510)
  store i32 %calltmp11, i32* %v14, align 4
  %v1112 = load i8*, i8** %v11, align 8
  %calltmp13 = call i64 @strlen(i8* %v1112)
  store i64 %calltmp13, i64* %v15, align 4
  %v1514 = load i64, i64* %v15, align 4
  %v1115 = load i8*, i8** %v11, align 8
  %insert_0 = insertvalue { i64, i8*, i1 } undef, i64 %v1514, 0
  %insert_1 = insertvalue { i64, i8*, i1 } %insert_0, i8* %v1115, 1
  %insert_2 = insertvalue { i64, i8*, i1 } %insert_1, i1 true, 2
  store { i64, i8*, i1 } %insert_2, { i64, i8*, i1 }* %v9, align 8
  store i64 21, i64* %v17, align 4
  %v916 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v9, align 8
  %index_tmp17 = extractvalue { i64, i8*, i1 } %v916, 0
  store i64 %index_tmp17, i64* %v18, align 4
  %v1718 = load i64, i64* %v17, align 4
  %v1819 = load i64, i64* %v18, align 4
  %addtmp = add i64 %v1718, %v1819
  store i64 %addtmp, i64* %v19, align 4
  store i8* getelementptr inbounds ([22 x i8], [22 x i8]* @strtmp.3, i32 0, i32 0), i8** %v20, align 8
  %v920 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v9, align 8
  %index_tmp21 = extractvalue { i64, i8*, i1 } %v920, 1
  store i8* %index_tmp21, i8** %v2122, align 8
  %v1923 = load i64, i64* %v19, align 4
  %addtmp24 = add i64 %v1923, 1
  store i64 %addtmp24, i64* %v22, align 4
  %v2225 = load i64, i64* %v22, align 4
  %malloc_call26 = call i8* @malloc(i64 %v2225)
  store i8* %malloc_call26, i8** %v23, align 8
  %v2327 = load i8*, i8** %v23, align 8
  %v2028 = load i8*, i8** %v20, align 8
  %v1729 = load i64, i64* %v17, align 4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v2327, i8* %v2028, i64 %v1729, i1 false)
  %v2330 = load i8*, i8** %v23, align 8
  %v1731 = load i64, i64* %v17, align 4
  %offset_ptr = getelementptr inbounds i8, i8* %v2330, i64 %v1731
  store i8* %offset_ptr, i8** %v24, align 8
  %v2432 = load i8*, i8** %v24, align 8
  %v2133 = load i8*, i8** %v2122, align 8
  %v1834 = load i64, i64* %v18, align 4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v2432, i8* %v2133, i64 %v1834, i1 false)
  %v2335 = load i8*, i8** %v23, align 8
  %v1936 = load i64, i64* %v19, align 4
  %offset_ptr37 = getelementptr inbounds i8, i8* %v2335, i64 %v1936
  store i8* %offset_ptr37, i8** %v25, align 8
  store { i64, i8*, i1 } { i64 0, i8* getelementptr inbounds ([1 x i8], [1 x i8]* @strtmp.4, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v26, align 8
  %v2638 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v26, align 8
  %index_tmp39 = extractvalue { i64, i8*, i1 } %v2638, 1
  store i8* %index_tmp39, i8** %v27, align 8
  %v2540 = load i8*, i8** %v25, align 8
  %v2741 = load i8*, i8** %v27, align 8
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v2540, i8* %v2741, i64 1, i1 false)
  %v1942 = load i64, i64* %v19, align 4
  %v2343 = load i8*, i8** %v23, align 8
  %insert_044 = insertvalue { i64, i8*, i1 } undef, i64 %v1942, 0
  %insert_145 = insertvalue { i64, i8*, i1 } %insert_044, i8* %v2343, 1
  %insert_246 = insertvalue { i64, i8*, i1 } %insert_145, i1 true, 2
  store { i64, i8*, i1 } %insert_246, { i64, i8*, i1 }* %v16, align 8
  %v1647 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v16, align 8
  store { i64, i8*, i1 } %v1647, { i64, i8*, i1 }* %v28, align 8
  store i64 0, i64* %v29, align 4
  %v2848 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v28, align 8
  %raw_ptr = extractvalue { i64, i8*, i1 } %v2848, 1
  %emit49 = call i32 @puts(i8* %raw_ptr)
  store i64 32, i64* %v31, align 4
  %v3150 = load i64, i64* %v31, align 4
  %malloc_call51 = call i8* @malloc(i64 %v3150)
  store i8* %malloc_call51, i8** %v32, align 8
  store { i64, i8*, i1 } { i64 4, i8* getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.5, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v3352, align 8
  %v3353 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v3352, align 8
  %index_tmp54 = extractvalue { i64, i8*, i1 } %v3353, 1
  store i8* %index_tmp54, i8** %v34, align 8
  %v3255 = load i8*, i8** %v32, align 8
  %v3456 = load i8*, i8** %v34, align 8
  %v757 = load i64, i64* %v7, align 4
  %calltmp58 = call i32 (i8*, i8*, ...) @sprintf(i8* %v3255, i8* %v3456, i64 %v757)
  store i32 %calltmp58, i32* %v35, align 4
  %v3259 = load i8*, i8** %v32, align 8
  %calltmp60 = call i64 @strlen(i8* %v3259)
  store i64 %calltmp60, i64* %v36, align 4
  %v3661 = load i64, i64* %v36, align 4
  %v3262 = load i8*, i8** %v32, align 8
  %insert_063 = insertvalue { i64, i8*, i1 } undef, i64 %v3661, 0
  %insert_164 = insertvalue { i64, i8*, i1 } %insert_063, i8* %v3262, 1
  %insert_265 = insertvalue { i64, i8*, i1 } %insert_164, i1 true, 2
  store { i64, i8*, i1 } %insert_265, { i64, i8*, i1 }* %v30, align 8
  store i64 21, i64* %v38, align 4
  %v3066 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v30, align 8
  %index_tmp67 = extractvalue { i64, i8*, i1 } %v3066, 0
  store i64 %index_tmp67, i64* %v39, align 4
  %v3868 = load i64, i64* %v38, align 4
  %v3969 = load i64, i64* %v39, align 4
  %addtmp70 = add i64 %v3868, %v3969
  store i64 %addtmp70, i64* %v40, align 4
  store i8* getelementptr inbounds ([22 x i8], [22 x i8]* @strtmp.7, i32 0, i32 0), i8** %v41, align 8
  %v3071 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v30, align 8
  %index_tmp72 = extractvalue { i64, i8*, i1 } %v3071, 1
  store i8* %index_tmp72, i8** %v4273, align 8
  %v4074 = load i64, i64* %v40, align 4
  %addtmp75 = add i64 %v4074, 1
  store i64 %addtmp75, i64* %v43, align 4
  %v4376 = load i64, i64* %v43, align 4
  %malloc_call77 = call i8* @malloc(i64 %v4376)
  store i8* %malloc_call77, i8** %v44, align 8
  %v4478 = load i8*, i8** %v44, align 8
  %v4179 = load i8*, i8** %v41, align 8
  %v3880 = load i64, i64* %v38, align 4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v4478, i8* %v4179, i64 %v3880, i1 false)
  %v4481 = load i8*, i8** %v44, align 8
  %v3882 = load i64, i64* %v38, align 4
  %offset_ptr83 = getelementptr inbounds i8, i8* %v4481, i64 %v3882
  store i8* %offset_ptr83, i8** %v45, align 8
  %v4584 = load i8*, i8** %v45, align 8
  %v4285 = load i8*, i8** %v4273, align 8
  %v3986 = load i64, i64* %v39, align 4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v4584, i8* %v4285, i64 %v3986, i1 false)
  %v4487 = load i8*, i8** %v44, align 8
  %v4088 = load i64, i64* %v40, align 4
  %offset_ptr89 = getelementptr inbounds i8, i8* %v4487, i64 %v4088
  store i8* %offset_ptr89, i8** %v46, align 8
  store { i64, i8*, i1 } { i64 0, i8* getelementptr inbounds ([1 x i8], [1 x i8]* @strtmp.8, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v47, align 8
  %v4790 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v47, align 8
  %index_tmp91 = extractvalue { i64, i8*, i1 } %v4790, 1
  store i8* %index_tmp91, i8** %v48, align 8
  %v4692 = load i8*, i8** %v46, align 8
  %v4893 = load i8*, i8** %v48, align 8
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v4692, i8* %v4893, i64 1, i1 false)
  %v4094 = load i64, i64* %v40, align 4
  %v4495 = load i8*, i8** %v44, align 8
  %insert_096 = insertvalue { i64, i8*, i1 } undef, i64 %v4094, 0
  %insert_197 = insertvalue { i64, i8*, i1 } %insert_096, i8* %v4495, 1
  %insert_298 = insertvalue { i64, i8*, i1 } %insert_197, i1 true, 2
  store { i64, i8*, i1 } %insert_298, { i64, i8*, i1 }* %v37, align 8
  %v3799 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v37, align 8
  store { i64, i8*, i1 } %v3799, { i64, i8*, i1 }* %v49, align 8
  store i64 0, i64* %v50, align 4
  %v49100 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v49, align 8
  %raw_ptr101 = extractvalue { i64, i8*, i1 } %v49100, 1
  %emit102 = call i32 @puts(i8* %raw_ptr101)
  ret i32 0
}

; Function Attrs: argmemonly nofree nounwind willreturn
declare void @llvm.memcpy.p0i8.p0i8.i64(i8* noalias nocapture writeonly, i8* noalias nocapture readonly, i64, i1 immarg) #0

attributes #0 = { argmemonly nofree nounwind willreturn }
