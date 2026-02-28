; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@fmt = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define i64 @example-derivation(i64 %0) {
bb0:
  %v1 = alloca i64, align 8
  %x = alloca i64, align 8
  store i64 %0, i64* %x, align 4
  store i64 10, i64* %v1, align 4
  %v11 = load i64, i64* %v1, align 4
  ret i64 %v11
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %v3 = alloca i64, align 8
  %v2 = alloca i64, align 8
  %__argc = alloca i32, align 4
  store i32 %0, i32* %__argc, align 4
  %__argv = alloca i64, align 8
  store i64 %1, i64* %__argv, align 4
  %calltmp = call i64 @example-derivation(i64 5)
  store i64 %calltmp, i64* %v2, align 4
  %v21 = load i64, i64* %v2, align 4
  store i64 %v21, i64* %v3, align 4
  %v32 = load i64, i64* %v3, align 4
  %printf_emit = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([6 x i8], [6 x i8]* @fmt, i32 0, i32 0), i64 %v32)
  ret i32 0
}
