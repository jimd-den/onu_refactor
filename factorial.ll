; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [33 x i8] c"The accumulation of 5 steps is: \00", align 1
@strtmp.1 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define i64 @terminal-seed() {
bb0:
  ret i64 1
}

define i64 @calculate-accumulation(i64 %0) {
bb0:
  %v7 = alloca i64, align 8
  %v6 = alloca i64, align 8
  %v5 = alloca i64, align 8
  %v4 = alloca i64, align 8
  %v3 = alloca i64, align 8
  %v2 = alloca i64, align 8
  %v1 = alloca i64, align 8
  %current-value = alloca i64, align 8
  store i64 %0, i64* %current-value, align 4
  %v0 = load i64, i64* %current-value, align 4
  %cmptmp = icmp eq i64 %v0, 0
  %booltmp = zext i1 %cmptmp to i64
  store i64 %booltmp, i64* %v1, align 4
  %v11 = load i64, i64* %v1, align 4
  %bool_cast = icmp ne i64 %v11, 0
  br i1 %bool_cast, label %bb1, label %bb2

bb1:                                              ; preds = %bb0
  %calltmp = call i64 @terminal-seed()
  store i64 %calltmp, i64* %v2, align 4
  %v22 = load i64, i64* %v2, align 4
  ret i64 %v22

bb2:                                              ; preds = %bb0
  %v03 = load i64, i64* %current-value, align 4
  %subtmp = sub i64 %v03, 1
  store i64 %subtmp, i64* %v3, align 4
  %v34 = load i64, i64* %v3, align 4
  store i64 %v34, i64* %v4, align 4
  %v45 = load i64, i64* %v4, align 4
  %calltmp6 = call i64 @calculate-accumulation(i64 %v45)
  store i64 %calltmp6, i64* %v5, align 4
  %v57 = load i64, i64* %v5, align 4
  store i64 %v57, i64* %v6, align 4
  %v08 = load i64, i64* %current-value, align 4
  %v69 = load i64, i64* %v6, align 4
  %multmp = mul i64 %v08, %v69
  store i64 %multmp, i64* %v7, align 4
  ret i64 0
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %v8 = alloca { i64, i8*, i1 }, align 8
  %v14 = alloca i64, align 8
  %v13 = alloca i32, align 4
  %v12 = alloca i8*, align 8
  %v11 = alloca { i64, i8*, i1 }, align 8
  %v10 = alloca i8*, align 8
  %v9 = alloca i64, align 8
  %v7 = alloca i64, align 8
  %v6 = alloca i64, align 8
  %v5 = alloca { i64, i8*, i1 }, align 8
  %v4 = alloca i64, align 8
  %v3 = alloca i64, align 8
  %v2 = alloca i64, align 8
  %__argc = alloca i32, align 4
  store i32 %0, i32* %__argc, align 4
  %__argv = alloca i64, align 8
  store i64 %1, i64* %__argv, align 4
  store i64 5, i64* %v2, align 4
  %v21 = load i64, i64* %v2, align 4
  %calltmp = call i64 @calculate-accumulation(i64 %v21)
  store i64 %calltmp, i64* %v3, align 4
  %v32 = load i64, i64* %v3, align 4
  store i64 %v32, i64* %v4, align 4
  store { i64, i8*, i1 } { i64 32, i8* getelementptr inbounds ([33 x i8], [33 x i8]* @strtmp, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v5, align 8
  store i64 0, i64* %v6, align 4
  %v53 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v5, align 8
  %raw_ptr = extractvalue { i64, i8*, i1 } %v53, 1
  %emit = call i32 @puts(i8* %raw_ptr)
  store i64 0, i64* %v7, align 4
  store i64 32, i64* %v9, align 4
  %v94 = load i64, i64* %v9, align 4
  %malloc_call = call i8* @malloc(i64 %v94)
  store i8* %malloc_call, i8** %v10, align 8
  store { i64, i8*, i1 } { i64 4, i8* getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.1, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v11, align 8
  %v115 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v11, align 8
  %index_tmp = extractvalue { i64, i8*, i1 } %v115, 1
  store i8* %index_tmp, i8** %v12, align 8
  %v106 = load i8*, i8** %v10, align 8
  %v127 = load i8*, i8** %v12, align 8
  %v48 = load i64, i64* %v4, align 4
  %calltmp9 = call i32 (i8*, i8*, ...) @sprintf(i8* %v106, i8* %v127, i64 %v48)
  store i32 %calltmp9, i32* %v13, align 4
  %v1010 = load i8*, i8** %v10, align 8
  %calltmp11 = call i64 @strlen(i8* %v1010)
  store i64 %calltmp11, i64* %v14, align 4
  %v1412 = load i64, i64* %v14, align 4
  %v1013 = load i8*, i8** %v10, align 8
  %insert_0 = insertvalue { i64, i8*, i1 } undef, i64 %v1412, 0
  %insert_1 = insertvalue { i64, i8*, i1 } %insert_0, i8* %v1013, 1
  %insert_2 = insertvalue { i64, i8*, i1 } %insert_1, i1 true, 2
  store { i64, i8*, i1 } %insert_2, { i64, i8*, i1 }* %v8, align 8
  %v814 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v8, align 8
  %raw_ptr15 = extractvalue { i64, i8*, i1 } %v814, 1
  %emit16 = call i32 @puts(i8* %raw_ptr15)
  ret i32 0
}
