; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [30 x i8] c"The population at generation \00", align 1
@strtmp.1 = private unnamed_addr constant [15 x i8] c" has reached: \00", align 1

declare void @broadcasts(i8*)

declare { i64, i8* } @as-text(i64)

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
  %v11 = alloca { i64, i8* }, align 8
  %v10 = alloca i64, align 8
  %v9 = alloca { i64, i8* }, align 8
  %v8 = alloca { i64, i8* }, align 8
  %v7 = alloca { i64, i8* }, align 8
  %v6 = alloca { i64, i8* }, align 8
  %v5 = alloca { i64, i8* }, align 8
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
  %v23 = load i64, i64* %v2, align 4
  %calltmp4 = call { i64, i8* } @as-text(i64 %v23)
  store { i64, i8* } %calltmp4, { i64, i8* }* %v5, align 8
  %v55 = load { i64, i8* }, { i64, i8* }* %v5, align 8
  %calltmp6 = call { i64, i8* } @joined-with({ i64, i8* } { i64 29, i8* getelementptr inbounds ([30 x i8], [30 x i8]* @strtmp, i32 0, i32 0) }, { i64, i8* } %v55)
  store { i64, i8* } %calltmp6, { i64, i8* }* %v6, align 8
  %v67 = load { i64, i8* }, { i64, i8* }* %v6, align 8
  store { i64, i8* } %v67, { i64, i8* }* %v7, align 8
  %v78 = load { i64, i8* }, { i64, i8* }* %v7, align 8
  %calltmp9 = call { i64, i8* } @joined-with({ i64, i8* } %v78, { i64, i8* } { i64 14, i8* getelementptr inbounds ([15 x i8], [15 x i8]* @strtmp.1, i32 0, i32 0) })
  store { i64, i8* } %calltmp9, { i64, i8* }* %v8, align 8
  %v810 = load { i64, i8* }, { i64, i8* }* %v8, align 8
  store { i64, i8* } %v810, { i64, i8* }* %v9, align 8
  store i64 0, i64* %v10, align 4
  %v911 = load { i64, i8* }, { i64, i8* }* %v9, align 8
  %raw_ptr = extractvalue { i64, i8* } %v911, 1
  call void @broadcasts(i8* %raw_ptr)
  %v412 = load i64, i64* %v4, align 4
  %calltmp13 = call { i64, i8* } @as-text(i64 %v412)
  store { i64, i8* } %calltmp13, { i64, i8* }* %v11, align 8
  %v1114 = load { i64, i8* }, { i64, i8* }* %v11, align 8
  %raw_ptr15 = extractvalue { i64, i8* } %v1114, 1
  call void @broadcasts(i8* %raw_ptr15)
  ret i32 0
}

declare { i64, i8* } @joined-with({ i64, i8* }, { i64, i8* })
