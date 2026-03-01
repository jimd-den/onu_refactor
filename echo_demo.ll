; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [23 x i8] c"No arguments provided.\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define i32 @main(i32 %0, i64 %1) {
bb0:
  %v7 = alloca { i64, i8*, i1 }, align 8
  %v8 = alloca i64, align 8
  %v6 = alloca { i64, i8* }, align 8
  %v5 = alloca { i64, i8* }, align 8
  %v4 = alloca i64, align 8
  %v3 = alloca i64, align 8
  %v2 = alloca i64, align 8
  %__argc = alloca i32, align 4
  store i32 %0, i32* %__argc, align 4
  %__argv = alloca i64, align 8
  store i64 %1, i64* %__argv, align 4
  %calltmp = call i64 @argument-count()
  store i64 %calltmp, i64* %v2, align 4
  %v21 = load i64, i64* %v2, align 4
  store i64 %v21, i64* %v3, align 4
  %v32 = load i64, i64* %v3, align 4
  %cmptmp = icmp sgt i64 %v32, 0
  %booltmp = zext i1 %cmptmp to i64
  store i64 %booltmp, i64* %v4, align 4
  %v43 = load i64, i64* %v4, align 4
  %bool_cast = icmp ne i64 %v43, 0
  br i1 %bool_cast, label %bb1, label %bb2

bb1:                                              ; preds = %bb0
  %calltmp4 = call { i64, i8* } @receives-argument(i64 0)
  store { i64, i8* } %calltmp4, { i64, i8* }* %v5, align 8
  %v55 = load { i64, i8* }, { i64, i8* }* %v5, align 8
  store { i64, i8* } %v55, { i64, i8* }* %v6, align 8
  %v66 = load { i64, i8* }, { i64, i8* }* %v6, align 8
  %raw_ptr = extractvalue { i64, i8* } %v66, 1
  %emit = call i32 @puts(i8* %raw_ptr)
  store i64 0, i64* %v8, align 4
  br label %bb3

bb2:                                              ; preds = %bb0
  store { i64, i8*, i1 } { i64 22, i8* getelementptr inbounds ([23 x i8], [23 x i8]* @strtmp, i32 0, i32 0), i1 false }, { i64, i8*, i1 }* %v7, align 8
  %v77 = load { i64, i8*, i1 }, { i64, i8*, i1 }* %v7, align 8
  %raw_ptr8 = extractvalue { i64, i8*, i1 } %v77, 1
  %emit9 = call i32 @puts(i8* %raw_ptr8)
  store i64 0, i64* %v8, align 4
  br label %bb3

bb3:                                              ; preds = %bb2, %bb1
  ret i32 0
}

declare i64 @argument-count()

declare { i64, i8* } @receives-argument(i64)
