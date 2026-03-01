; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [30 x i8] c"PASS: 1 opposes 2 is TRUE (1)\00", align 1
@strtmp.1 = private unnamed_addr constant [31 x i8] c"FAIL: 1 opposes 2 is FALSE (0)\00", align 1
@strtmp.2 = private unnamed_addr constant [31 x i8] c"PASS: 1 opposes 1 is FALSE (0)\00", align 1
@strtmp.3 = private unnamed_addr constant [30 x i8] c"FAIL: 1 opposes 1 is TRUE (1)\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define i32 @main(i32 %0, i64 %1) {
bb0:
  %emit7 = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([31 x i8], [31 x i8]* @strtmp.1, i64 0, i64 0))
  %emit19 = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([30 x i8], [30 x i8]* @strtmp.3, i64 0, i64 0))
  ret i32 0
}
