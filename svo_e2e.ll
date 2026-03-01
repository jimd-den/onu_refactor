; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [12 x i8] c"Greetings, \00", align 1
@strtmp.1 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@strtmp.2 = private unnamed_addr constant [10 x i8] c"Conductor\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define void @greet({ i64, i8*, i1 } %0, i64 %1, i64 %2) {
bb0:
  %v17 = alloca { i64, i8*, i1 }, align 8
  %v5 = alloca { i64, i8*, i1 }, align 8
  %v16 = alloca i8*, align 8
  %v15 = alloca { i64, i8*, i1 }, align 8
  %v14 = alloca i8*, align 8
  %v13 = alloca i8*, align 8
  %v12 = alloca i8*, align 8
  %v11 = alloca i64, align 8
  %v10 = alloca i8*, align 8
  %v9 = alloca i8*, align 8
  %v8 = alloca i64, align 8
  %v7 = alloca i64, align 8
  %v6 = alloca i64, align 8
  %v4 = alloca { i64, i8*, i1 }, align 8
  %v3 = alloca { i64, i8*, i1 }, align 8
  %name = alloca { i64, i8*, i1 }, align 8
  store { i64, i8*, i1 } %0, { i64, i8*, i1 }* %name, align 8
  %via = alloca i64, align 8
  store i64 %1, i64* %via, align 4
  %observation = alloca i64, align 8
  store i64 %2, i64* %observation, align 4
  store { i64, i8*, i1 } { i64 11, i8* getelementptr inbounds ([12 x i8], [12 x i8]* @strtmp, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v3, align 8
  %v31 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v3, align 8
  store { i64, i8*, i1 } %v31, { i64, i8*, i1 }* %v4, align 8
  %v42 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v4, align 8
  %index_tmp = extractvalue { i64, i8*, i1 } %v42, 0
  store i64 %index_tmp, i64* %v6, align 4
  %v0 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %name, align 8
  %index_tmp3 = extractvalue { i64, i8*, i1 } %v0, 0
  store i64 %index_tmp3, i64* %v7, align 4
  %v64 = load i64, i64* %v6, align 4
  %v75 = load i64, i64* %v7, align 4
  %addtmp = add i64 %v64, %v75
  store i64 %addtmp, i64* %v8, align 4
  %v46 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v4, align 8
  %index_tmp7 = extractvalue { i64, i8*, i1 } %v46, 1
  store i8* %index_tmp7, i8** %v9, align 8
  %v08 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %name, align 8
  %index_tmp9 = extractvalue { i64, i8*, i1 } %v08, 1
  store i8* %index_tmp9, i8** %v10, align 8
  %v810 = load i64, i64* %v8, align 4
  %addtmp11 = add i64 %v810, 1
  store i64 %addtmp11, i64* %v11, align 4
  %v1112 = load i64, i64* %v11, align 4
  %malloc_call = call i8* @malloc(i64 %v1112)
  store i8* %malloc_call, i8** %v12, align 8
  %v1213 = load i8*, i8** %v12, align 8
  %v914 = load i8*, i8** %v9, align 8
  %v615 = load i64, i64* %v6, align 4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v1213, i8* %v914, i64 %v615, i1 false)
  %v1216 = load i8*, i8** %v12, align 8
  %v617 = load i64, i64* %v6, align 4
  %offset_ptr = getelementptr inbounds i8, i8* %v1216, i64 %v617
  store i8* %offset_ptr, i8** %v13, align 8
  %v1318 = load i8*, i8** %v13, align 8
  %v1019 = load i8*, i8** %v10, align 8
  %v720 = load i64, i64* %v7, align 4
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v1318, i8* %v1019, i64 %v720, i1 false)
  %v1221 = load i8*, i8** %v12, align 8
  %v822 = load i64, i64* %v8, align 4
  %offset_ptr23 = getelementptr inbounds i8, i8* %v1221, i64 %v822
  store i8* %offset_ptr23, i8** %v14, align 8
  store { i64, i8*, i1 } { i64 0, i8* getelementptr inbounds ([1 x i8], [1 x i8]* @strtmp.1, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v15, align 8
  %v1524 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v15, align 8
  %index_tmp25 = extractvalue { i64, i8*, i1 } %v1524, 1
  store i8* %index_tmp25, i8** %v16, align 8
  %v1426 = load i8*, i8** %v14, align 8
  %v1627 = load i8*, i8** %v16, align 8
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* %v1426, i8* %v1627, i64 1, i1 false)
  %v828 = load i64, i64* %v8, align 4
  %v1229 = load i8*, i8** %v12, align 8
  %insert_0 = insertvalue { i64, i8*, i1 } undef, i64 %v828, 0
  %insert_1 = insertvalue { i64, i8*, i1 } %insert_0, i8* %v1229, 1
  %insert_2 = insertvalue { i64, i8*, i1 } %insert_1, i1 true, 2
  store { i64, i8*, i1 } %insert_2, { i64, i8*, i1 }* %v5, align 8
  %v530 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v5, align 8
  store { i64, i8*, i1 } %v530, { i64, i8*, i1 }* %v17, align 8
  %v1731 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v17, align 8
  %raw_ptr = extractvalue { i64, i8*, i1 } %v1731, 1
  %emit = call i32 @puts(i8* %raw_ptr)
  ret void
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %v3 = alloca i64, align 8
  %v2 = alloca { i64, i8*, i1 }, align 8
  %__argc = alloca i32, align 4
  store i32 %0, i32* %__argc, align 4
  %__argv = alloca i64, align 8
  store i64 %1, i64* %__argv, align 4
  store { i64, i8*, i1 } { i64 9, i8* getelementptr inbounds ([10 x i8], [10 x i8]* @strtmp.2, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v2, align 8
  %v21 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v2, align 8
  call void @greet({ i64, i8*, i1 } %v21)
  store i64 0, i64* %v3, align 4
  ret i32 0
}

; Function Attrs: argmemonly nofree nounwind willreturn
declare void @llvm.memcpy.p0i8.p0i8.i64(i8* noalias nocapture writeonly, i8* noalias nocapture readonly, i64, i1 immarg) #0

attributes #0 = { argmemonly nofree nounwind willreturn }
