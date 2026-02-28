; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [33 x i8] c"The accumulation of 5 steps is: \00", align 1

declare void @broadcasts(i8*)

declare { i64, i8* } @as-text(i64)

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
  %v8 = alloca { i64, i8* }, align 8
  %v7 = alloca i64, align 8
  %v6 = alloca i64, align 8
  %v5 = alloca { i64, i8* }, align 8
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
  store { i64, i8* } { i64 32, i8* getelementptr inbounds ([33 x i8], [33 x i8]* @strtmp, i32 0, i32 0) }, { i64, i8* }* %v5, align 8
  store i64 0, i64* %v6, align 4
  %v53 = load { i64, i8* }, { i64, i8* }* %v5, align 8
  %raw_ptr = extractvalue { i64, i8* } %v53, 1
  call void @broadcasts(i8* %raw_ptr)
  store i64 0, i64* %v7, align 4
  %v44 = load i64, i64* %v4, align 4
  %calltmp5 = call { i64, i8* } @as-text(i64 %v44)
  store { i64, i8* } %calltmp5, { i64, i8* }* %v8, align 8
  %v86 = load { i64, i8* }, { i64, i8* }* %v8, align 8
  %raw_ptr7 = extractvalue { i64, i8* } %v86, 1
  call void @broadcasts(i8* %raw_ptr7)
  ret i32 0
}
