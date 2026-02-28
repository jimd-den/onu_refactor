; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [5 x i8] c"%lld\00", align 1
@strtmp.1 = private unnamed_addr constant [42 x i8] c"COLLATZ SEQUENCE (Starting at 1,000,000):\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define void @broadcast-sequence(i64 %0, i64 %1) {
bb0:
  %v22 = alloca i64, align 8
  %v21 = alloca i64, align 8
  %v20 = alloca i64, align 8
  %v18 = alloca i64, align 8
  %v17 = alloca i64, align 8
  %v19 = alloca i64, align 8
  %v16 = alloca i64, align 8
  %v15 = alloca i64, align 8
  %v14 = alloca i64, align 8
  %v13 = alloca i64, align 8
  %v12 = alloca i64, align 8
  %v23 = alloca i64, align 8
  %v11 = alloca i64, align 8
  %v24 = alloca i64, align 8
  %v10 = alloca i64, align 8
  %v3 = alloca { i64, i8*, i1 }, align 8
  %v9 = alloca i64, align 8
  %v8 = alloca i32, align 4
  %v7 = alloca i8*, align 8
  %v6 = alloca { i64, i8*, i1 }, align 8
  %v5 = alloca i8*, align 8
  %v4 = alloca i64, align 8
  %v2 = alloca i64, align 8
  %n = alloca i64, align 8
  store i64 %0, i64* %n, align 4
  %terms-remaining = alloca i64, align 8
  store i64 %1, i64* %terms-remaining, align 4
  store i64 0, i64* %v2, align 4
  store i64 32, i64* %v4, align 4
  %v41 = load i64, i64* %v4, align 4
  %malloc_call = call i8* @malloc(i64 %v41)
  store i8* %malloc_call, i8** %v5, align 8
  store { i64, i8*, i1 } { i64 4, i8* getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v6, align 8
  %v62 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v6, align 8
  %index_tmp = extractvalue { i64, i8*, i1 } %v62, 1
  store i8* %index_tmp, i8** %v7, align 8
  %v53 = load i8*, i8** %v5, align 8
  %v74 = load i8*, i8** %v7, align 8
  %v0 = load i64, i64* %n, align 4
  %calltmp = call i32 (i8*, i8*, ...) @sprintf(i8* %v53, i8* %v74, i64 %v0)
  store i32 %calltmp, i32* %v8, align 4
  %v55 = load i8*, i8** %v5, align 8
  %calltmp6 = call i64 @strlen(i8* %v55)
  store i64 %calltmp6, i64* %v9, align 4
  %v97 = load i64, i64* %v9, align 4
  %v58 = load i8*, i8** %v5, align 8
  %insert_0 = insertvalue { i64, i8*, i1 } undef, i64 %v97, 0
  %insert_1 = insertvalue { i64, i8*, i1 } %insert_0, i8* %v58, 1
  %insert_2 = insertvalue { i64, i8*, i1 } %insert_1, i1 true, 2
  store { i64, i8*, i1 } %insert_2, { i64, i8*, i1 }* %v3, align 8
  %v39 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v3, align 8
  %raw_ptr = extractvalue { i64, i8*, i1 } %v39, 1
  %emit = call i32 @puts(i8* %raw_ptr)
  %v010 = load i64, i64* %n, align 4
  %cmptmp = icmp eq i64 %v010, 1
  %booltmp = zext i1 %cmptmp to i64
  store i64 %booltmp, i64* %v10, align 4
  %v1011 = load i64, i64* %v10, align 4
  %bool_cast = icmp ne i64 %v1011, 0
  br i1 %bool_cast, label %bb1, label %bb2

bb1:                                              ; preds = %bb0
  store i64 0, i64* %v24, align 4
  br label %bb9

bb2:                                              ; preds = %bb0
  %v1 = load i64, i64* %terms-remaining, align 4
  %cmptmp12 = icmp eq i64 %v1, 1
  %booltmp13 = zext i1 %cmptmp12 to i64
  store i64 %booltmp13, i64* %v11, align 4
  %v1114 = load i64, i64* %v11, align 4
  %bool_cast15 = icmp ne i64 %v1114, 0
  br i1 %bool_cast15, label %bb3, label %bb4

bb3:                                              ; preds = %bb2
  store i64 0, i64* %v23, align 4
  br label %bb8

bb4:                                              ; preds = %bb2
  %v016 = load i64, i64* %n, align 4
  %divtmp = sdiv i64 %v016, 2
  store i64 %divtmp, i64* %v12, align 4
  %v1217 = load i64, i64* %v12, align 4
  store i64 %v1217, i64* %v13, align 4
  %v1318 = load i64, i64* %v13, align 4
  %multmp = mul i64 %v1318, 2
  store i64 %multmp, i64* %v14, align 4
  %v1419 = load i64, i64* %v14, align 4
  %v020 = load i64, i64* %n, align 4
  %cmptmp21 = icmp eq i64 %v1419, %v020
  %booltmp22 = zext i1 %cmptmp21 to i64
  store i64 %booltmp22, i64* %v15, align 4
  %v1523 = load i64, i64* %v15, align 4
  store i64 %v1523, i64* %v16, align 4
  %v1624 = load i64, i64* %v16, align 4
  %bool_cast25 = icmp ne i64 %v1624, 0
  br i1 %bool_cast25, label %bb5, label %bb6

bb5:                                              ; preds = %bb4
  %v1326 = load i64, i64* %v13, align 4
  store i64 %v1326, i64* %v19, align 4
  br label %bb7

bb6:                                              ; preds = %bb4
  %v027 = load i64, i64* %n, align 4
  %multmp28 = mul i64 %v027, 3
  store i64 %multmp28, i64* %v17, align 4
  %v1729 = load i64, i64* %v17, align 4
  %addtmp = add i64 %v1729, 1
  store i64 %addtmp, i64* %v18, align 4
  %v1830 = load i64, i64* %v18, align 4
  store i64 %v1830, i64* %v19, align 4
  br label %bb7

bb7:                                              ; preds = %bb6, %bb5
  %v1931 = load i64, i64* %v19, align 4
  store i64 %v1931, i64* %v20, align 4
  %v132 = load i64, i64* %terms-remaining, align 4
  %subtmp = sub i64 %v132, 1
  store i64 %subtmp, i64* %v21, align 4
  %v2033 = load i64, i64* %v20, align 4
  %v2134 = load i64, i64* %v21, align 4
  call void @broadcast-sequence(i64 %v2033, i64 %v2134)
  store i64 0, i64* %v22, align 4
  %v2235 = load i64, i64* %v22, align 4
  store i64 %v2235, i64* %v23, align 4
  br label %bb8

bb8:                                              ; preds = %bb7, %bb3
  %v2336 = load i64, i64* %v23, align 4
  store i64 %v2336, i64* %v24, align 4
  br label %bb9

bb9:                                              ; preds = %bb8, %bb1
  ret void
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %v3 = alloca i64, align 8
  %v2 = alloca i64, align 8
  %__argc = alloca i32, align 4
  store i32 %0, i32* %__argc, align 4
  %__argv = alloca i64, align 8
  store i64 %1, i64* %__argv, align 4
  store i64 0, i64* %v2, align 4
  %emit = call i32 @puts(i8* getelementptr inbounds ([42 x i8], [42 x i8]* @strtmp.1, i32 0, i32 0))
  call void @broadcast-sequence(i64 1000000, i64 10)
  store i64 0, i64* %v3, align 4
  ret i32 0
}
