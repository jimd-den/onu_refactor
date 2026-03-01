; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [16 x i8] c"Linear Resource\00", align 1
@strtmp.1 = private unnamed_addr constant [16 x i8] c"Branch Resource\00", align 1
@strtmp.2 = private unnamed_addr constant [39 x i8] c"PASS: Ownership verification complete.\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define fastcc void @take-custody({ i64, i8*, i1 } %0) {
bb0:
  %raw_ptr = extractvalue { i64, i8*, i1 } %0, 1
  %emit = call i32 @puts(i8* noundef nonnull dereferenceable(1) %raw_ptr)
  ret void
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  call fastcc void @take-custody({ i64, i8*, i1 } { i64 15, i8* getelementptr inbounds ([16 x i8], [16 x i8]* @strtmp, i32 0, i32 0), i1 false })
  call fastcc void @take-custody({ i64, i8*, i1 } { i64 15, i8* getelementptr inbounds ([16 x i8], [16 x i8]* @strtmp.1, i32 0, i32 0), i1 false })
  %emit = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([39 x i8], [39 x i8]* @strtmp.2, i64 0, i64 0))
  ret i32 0
}
