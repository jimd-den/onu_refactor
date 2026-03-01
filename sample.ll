; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@fmt = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define fastcc i64 @example-derivation(i64 %0) {
bb0:
  ret i64 10
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %calltmp = call fastcc i64 @example-derivation(i64 5)
  %printf_emit = call i32 (i8*, ...) @printf(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([6 x i8], [6 x i8]* @fmt, i64 0, i64 0), i64 %calltmp)
  ret i32 0
}
