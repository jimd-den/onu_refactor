; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [5 x i8] c"%lld\00", align 1
@strtmp.1 = private unnamed_addr constant [42 x i8] c"COLLATZ SEQUENCE (Starting at 1,000,000):\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define fastcc void @broadcast-sequence(i64 %0, i64 %1) {
bb0:
  %malloc_call = call dereferenceable_or_null(32) i8* @malloc(i64 32)
  %calltmp = call i32 (i8*, i8*, ...) @sprintf(i8* noundef nonnull dereferenceable(1) %malloc_call, i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp, i64 0, i64 0), i64 %0)
  %calltmp6 = call i64 @strlen(i8* noundef nonnull dereferenceable(1) %malloc_call)
  %emit = call i32 @puts(i8* noundef nonnull dereferenceable(1) %malloc_call)
  call void @free(i8* %malloc_call)
  %cmptmp = icmp eq i64 %0, 1
  %cmptmp12 = icmp eq i64 %1, 1
  %or.cond = select i1 %cmptmp, i1 true, i1 %cmptmp12
  br i1 %or.cond, label %common.ret, label %bb4

common.ret:                                       ; preds = %bb0, %bb4
  ret void

bb4:                                              ; preds = %bb0
  %divtmp = sdiv i64 %0, 2
  %multmp = shl nsw i64 %divtmp, 1
  %cmptmp21 = icmp eq i64 %multmp, %0
  %multmp28 = mul i64 %0, 3
  %addtmp = add i64 %multmp28, 1
  %v19.0 = select i1 %cmptmp21, i64 %divtmp, i64 %addtmp
  %subtmp = add i64 %1, -1
  call fastcc void @broadcast-sequence(i64 %v19.0, i64 %subtmp)
  br label %common.ret
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %emit = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([42 x i8], [42 x i8]* @strtmp.1, i64 0, i64 0))
  call fastcc void @broadcast-sequence(i64 1000000, i64 10)
  ret i32 0
}
