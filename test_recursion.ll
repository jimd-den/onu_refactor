; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [31 x i8] c"PASS: Deep recursion complete.\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define fastcc void @count-down(i64 %0) {
bb0:
  %cmptmp = icmp eq i64 %0, 0
  br i1 %cmptmp, label %bb1, label %bb2

common.ret:                                       ; preds = %bb2, %bb1
  ret void

bb1:                                              ; preds = %bb0
  %emit = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([31 x i8], [31 x i8]* @strtmp, i64 0, i64 0))
  br label %common.ret

bb2:                                              ; preds = %bb0
  %subtmp = add i64 %0, -1
  call fastcc void @count-down(i64 %subtmp)
  br label %common.ret
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  call fastcc void @count-down(i64 100000)
  ret i32 0
}
