; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [33 x i8] c"The accumulation of 5 steps is: \00", align 1
@strtmp.1 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define fastcc i64 @terminal-seed() {
bb0:
  ret i64 1
}

define fastcc i64 @calculate-accumulation(i64 %0) {
bb0:
  %cmptmp = icmp eq i64 %0, 0
  br i1 %cmptmp, label %bb1, label %bb2

common.ret:                                       ; preds = %bb2, %bb1
  %common.ret.op = phi i64 [ %calltmp, %bb1 ], [ %multmp, %bb2 ]
  ret i64 %common.ret.op

bb1:                                              ; preds = %bb0
  %calltmp = call fastcc i64 @terminal-seed()
  br label %common.ret

bb2:                                              ; preds = %bb0
  %subtmp = add i64 %0, -1
  %calltmp6 = call fastcc i64 @calculate-accumulation(i64 %subtmp)
  %multmp = mul i64 %calltmp6, %0
  br label %common.ret
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %calltmp = call fastcc i64 @calculate-accumulation(i64 5)
  %emit = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([33 x i8], [33 x i8]* @strtmp, i64 0, i64 0))
  %malloc_call = call dereferenceable_or_null(32) i8* @malloc(i64 32)
  %calltmp10 = call i32 (i8*, i8*, ...) @sprintf(i8* noundef nonnull dereferenceable(1) %malloc_call, i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.1, i64 0, i64 0), i64 %calltmp)
  %calltmp12 = call i64 @strlen(i8* noundef nonnull dereferenceable(1) %malloc_call)
  %emit17 = call i32 @puts(i8* noundef nonnull dereferenceable(1) %malloc_call)
  call void @free(i8* %malloc_call)
  ret i32 0
}
