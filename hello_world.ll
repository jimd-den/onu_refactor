; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [14 x i8] c"Hello, World!\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define i32 @main(i32 %0, i64 %1) {
bb0:
  %__argc = alloca i32, align 4
  store i32 %0, i32* %__argc, align 4
  %__argv = alloca i64, align 8
  store i64 %1, i64* %__argv, align 4
  %emit = call i32 @puts(i8* getelementptr inbounds ([14 x i8], [14 x i8]* @strtmp, i32 0, i32 0))
  ret i32 0
}
