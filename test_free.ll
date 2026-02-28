; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

declare { i64, i8* } @as-text(i64)
