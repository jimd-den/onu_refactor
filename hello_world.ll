; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [14 x i8] c"Hello, World!\00", align 1

declare void @broadcasts(i8*)

declare { i64, i8* } @as-text(i64)

define i32 @main(i32 %0, i64 %1) {
bb0:
  %__argc = alloca i32, align 4
  store i32 %0, i32* %__argc, align 4
  %__argv = alloca i64, align 8
  store i64 %1, i64* %__argv, align 4
  call void @broadcasts(i8* getelementptr inbounds ([14 x i8], [14 x i8]* @strtmp, i32 0, i32 0))
  ret i32 0
}
