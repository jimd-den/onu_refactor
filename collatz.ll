; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [42 x i8] c"COLLATZ SEQUENCE (Starting at 1,000,000):\00", align 1

declare void @broadcasts(i8*)

declare { i64, i8* } @as-text(i64)

define void @broadcast-sequence(i64 %0, i64 %1) {
bb0:
  %v16 = alloca i64, align 8
  %v15 = alloca i64, align 8
  %v14 = alloca i64, align 8
  %v12 = alloca i64, align 8
  %v11 = alloca i64, align 8
  %v13 = alloca i64, align 8
  %v10 = alloca i64, align 8
  %v9 = alloca i64, align 8
  %v8 = alloca i64, align 8
  %v7 = alloca i64, align 8
  %v6 = alloca i64, align 8
  %v5 = alloca i64, align 8
  %v4 = alloca i64, align 8
  %v3 = alloca { i64, i8* }, align 8
  %v2 = alloca i64, align 8
  %n = alloca i64, align 8
  store i64 %0, i64* %n, align 4
  %terms-remaining = alloca i64, align 8
  store i64 %1, i64* %terms-remaining, align 4
  store i64 0, i64* %v2, align 4
  %v0 = load i64, i64* %n, align 4
  %calltmp = call { i64, i8* } @as-text(i64 %v0)
  store { i64, i8* } %calltmp, { i64, i8* }* %v3, align 8
  %v31 = load { i64, i8* }, { i64, i8* }* %v3, align 8
  %raw_ptr = extractvalue { i64, i8* } %v31, 1
  call void @broadcasts(i8* %raw_ptr)
  %v02 = load i64, i64* %n, align 4
  %cmptmp = icmp eq i64 %v02, 1
  %booltmp = zext i1 %cmptmp to i64
  store i64 %booltmp, i64* %v4, align 4
  %v43 = load i64, i64* %v4, align 4
  %bool_cast = icmp ne i64 %v43, 0
  br i1 %bool_cast, label %bb1, label %bb2

bb1:                                              ; preds = %bb0
  ret void

bb2:                                              ; preds = %bb0
  %v1 = load i64, i64* %terms-remaining, align 4
  %cmptmp4 = icmp eq i64 %v1, 1
  %booltmp5 = zext i1 %cmptmp4 to i64
  store i64 %booltmp5, i64* %v5, align 4
  %v56 = load i64, i64* %v5, align 4
  %bool_cast7 = icmp ne i64 %v56, 0
  br i1 %bool_cast7, label %bb3, label %bb4

bb3:                                              ; preds = %bb2
  ret void

bb4:                                              ; preds = %bb2
  %v08 = load i64, i64* %n, align 4
  %divtmp = sdiv i64 %v08, 2
  store i64 %divtmp, i64* %v6, align 4
  %v69 = load i64, i64* %v6, align 4
  store i64 %v69, i64* %v7, align 4
  %v710 = load i64, i64* %v7, align 4
  %multmp = mul i64 %v710, 2
  store i64 %multmp, i64* %v8, align 4
  %v811 = load i64, i64* %v8, align 4
  %v012 = load i64, i64* %n, align 4
  %cmptmp13 = icmp eq i64 %v811, %v012
  %booltmp14 = zext i1 %cmptmp13 to i64
  store i64 %booltmp14, i64* %v9, align 4
  %v915 = load i64, i64* %v9, align 4
  store i64 %v915, i64* %v10, align 4
  %v1016 = load i64, i64* %v10, align 4
  %bool_cast17 = icmp ne i64 %v1016, 0
  br i1 %bool_cast17, label %bb5, label %bb6

bb5:                                              ; preds = %bb4
  %v718 = load i64, i64* %v7, align 4
  store i64 %v718, i64* %v13, align 4
  br label %bb7

bb6:                                              ; preds = %bb4
  %v019 = load i64, i64* %n, align 4
  %multmp20 = mul i64 %v019, 3
  store i64 %multmp20, i64* %v11, align 4
  %v1121 = load i64, i64* %v11, align 4
  %addtmp = add i64 %v1121, 1
  store i64 %addtmp, i64* %v12, align 4
  %v1222 = load i64, i64* %v12, align 4
  store i64 %v1222, i64* %v13, align 4
  br label %bb7

bb7:                                              ; preds = %bb6, %bb5
  %v1323 = load i64, i64* %v13, align 4
  store i64 %v1323, i64* %v14, align 4
  %v124 = load i64, i64* %terms-remaining, align 4
  %subtmp = sub i64 %v124, 1
  store i64 %subtmp, i64* %v15, align 4
  %v1425 = load i64, i64* %v14, align 4
  %v1526 = load i64, i64* %v15, align 4
  call void @broadcast-sequence(i64 %v1425, i64 %v1526)
  store i64 0, i64* %v16, align 4
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
  call void @broadcasts(i8* getelementptr inbounds ([42 x i8], [42 x i8]* @strtmp, i32 0, i32 0))
  call void @broadcast-sequence(i64 1000000, i64 10)
  store i64 0, i64* %v3, align 4
  ret i32 0
}
