; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [21 x i8] c"PARITY VERIFICATION:\00", align 1
@strtmp.1 = private unnamed_addr constant [22 x i8] c"Is 10 even? (1=yes): \00", align 1
@strtmp.2 = private unnamed_addr constant [22 x i8] c"Is 7 even?  (1=yes): \00", align 1

declare void @broadcasts(i8*)

declare { i64, i8* } @as-text(i64)

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
  %v16 = alloca i64, align 8
  %v15 = alloca { i64, i8* }, align 8
  %v14 = alloca { i64, i8* }, align 8
  %v13 = alloca { i64, i8* }, align 8
  %v12 = alloca i64, align 8
  %v11 = alloca { i64, i8* }, align 8
  %v10 = alloca { i64, i8* }, align 8
  %v9 = alloca { i64, i8* }, align 8
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
  call void @broadcasts(i8* getelementptr inbounds ([21 x i8], [21 x i8]* @strtmp, i32 0, i32 0))
  %v56 = load i64, i64* %v5, align 4
  %calltmp7 = call { i64, i8* } @as-text(i64 %v56)
  store { i64, i8* } %calltmp7, { i64, i8* }* %v9, align 8
  %v98 = load { i64, i8* }, { i64, i8* }* %v9, align 8
  %calltmp9 = call { i64, i8* } @joined-with({ i64, i8* } { i64 21, i8* getelementptr inbounds ([22 x i8], [22 x i8]* @strtmp.1, i32 0, i32 0) }, { i64, i8* } %v98)
  store { i64, i8* } %calltmp9, { i64, i8* }* %v10, align 8
  %v1010 = load { i64, i8* }, { i64, i8* }* %v10, align 8
  store { i64, i8* } %v1010, { i64, i8* }* %v11, align 8
  store i64 0, i64* %v12, align 4
  %v1111 = load { i64, i8* }, { i64, i8* }* %v11, align 8
  %raw_ptr = extractvalue { i64, i8* } %v1111, 1
  call void @broadcasts(i8* %raw_ptr)
  %v712 = load i64, i64* %v7, align 4
  %calltmp13 = call { i64, i8* } @as-text(i64 %v712)
  store { i64, i8* } %calltmp13, { i64, i8* }* %v13, align 8
  %v1314 = load { i64, i8* }, { i64, i8* }* %v13, align 8
  %calltmp15 = call { i64, i8* } @joined-with({ i64, i8* } { i64 21, i8* getelementptr inbounds ([22 x i8], [22 x i8]* @strtmp.2, i32 0, i32 0) }, { i64, i8* } %v1314)
  store { i64, i8* } %calltmp15, { i64, i8* }* %v14, align 8
  %v1416 = load { i64, i8* }, { i64, i8* }* %v14, align 8
  store { i64, i8* } %v1416, { i64, i8* }* %v15, align 8
  store i64 0, i64* %v16, align 4
  %v1517 = load { i64, i8* }, { i64, i8* }* %v15, align 8
  %raw_ptr18 = extractvalue { i64, i8* } %v1517, 1
  call void @broadcasts(i8* %raw_ptr18)
  ret i32 0
}

declare { i64, i8* } @joined-with({ i64, i8* }, { i64, i8* })
