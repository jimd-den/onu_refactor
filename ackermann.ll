; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [130 x i8] c"\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\00", align 1
@strtmp.1 = private unnamed_addr constant [33 x i8] c"  ACKERMANN GROWTH DEMONSTRATION\00", align 1
@strtmp.2 = private unnamed_addr constant [40 x i8] c"  Rules: Successor, Descent, and Spiral\00", align 1
@strtmp.3 = private unnamed_addr constant [130 x i8] c"\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\00", align 1
@strtmp.4 = private unnamed_addr constant [24 x i8] c"Solving Spiral(2, 2)...\00", align 1
@strtmp.5 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1
@strtmp.6 = private unnamed_addr constant [24 x i8] c"Solving Spiral(3, 2)...\00", align 1
@strtmp.7 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1
@strtmp.8 = private unnamed_addr constant [130 x i8] c"\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\E2\95\90\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define fastcc i64 @successor-of(i64 %0) {
bb0:
  %addtmp = add i64 %0, 1
  ret i64 %addtmp
}

define fastcc i64 @descend-tier(i64 %0) {
bb0:
  %subtmp = add i64 %0, -1
  %calltmp = call fastcc i64 @ackermann(i64 %subtmp, i64 1)
  ret i64 %calltmp
}

define fastcc i64 @spiral-down(i64 %0, i64 %1) {
bb0:
  %subtmp = add i64 %0, -1
  %subtmp2 = add i64 %1, -1
  %calltmp = call fastcc i64 @ackermann(i64 %0, i64 %subtmp2)
  %calltmp9 = call fastcc i64 @ackermann(i64 %subtmp, i64 %calltmp)
  ret i64 %calltmp9
}

define fastcc i64 @ackermann(i64 %0, i64 %1) {
bb0:
  %cmptmp = icmp eq i64 %0, 0
  br i1 %cmptmp, label %bb1, label %bb2

common.ret:                                       ; preds = %bb4, %bb3, %bb1
  %common.ret.op = phi i64 [ %calltmp, %bb1 ], [ %calltmp9, %bb3 ], [ %calltmp13, %bb4 ]
  ret i64 %common.ret.op

bb1:                                              ; preds = %bb0
  %calltmp = call fastcc i64 @successor-of(i64 %1)
  br label %common.ret

bb2:                                              ; preds = %bb0
  %cmptmp4 = icmp eq i64 %1, 0
  br i1 %cmptmp4, label %bb3, label %bb4

bb3:                                              ; preds = %bb2
  %calltmp9 = call fastcc i64 @descend-tier(i64 %0)
  br label %common.ret

bb4:                                              ; preds = %bb2
  %calltmp13 = call fastcc i64 @spiral-down(i64 %0, i64 %1)
  br label %common.ret
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %emit = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([130 x i8], [130 x i8]* @strtmp, i64 0, i64 0))
  %emit4 = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([33 x i8], [33 x i8]* @strtmp.1, i64 0, i64 0))
  %emit7 = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([40 x i8], [40 x i8]* @strtmp.2, i64 0, i64 0))
  %emit10 = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([130 x i8], [130 x i8]* @strtmp.3, i64 0, i64 0))
  %emit13 = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([24 x i8], [24 x i8]* @strtmp.4, i64 0, i64 0))
  %calltmp = call fastcc i64 @ackermann(i64 2, i64 2)
  %malloc_call = call dereferenceable_or_null(32) i8* @malloc(i64 32)
  %calltmp20 = call i32 (i8*, i8*, ...) @sprintf(i8* noundef nonnull dereferenceable(1) %malloc_call, i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.5, i64 0, i64 0), i64 %calltmp)
  %calltmp22 = call i64 @strlen(i8* noundef nonnull dereferenceable(1) %malloc_call)
  %emit27 = call i32 @puts(i8* noundef nonnull dereferenceable(1) %malloc_call)
  call void @free(i8* %malloc_call)
  %emit30 = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([24 x i8], [24 x i8]* @strtmp.6, i64 0, i64 0))
  %calltmp31 = call fastcc i64 @ackermann(i64 3, i64 2)
  %malloc_call34 = call dereferenceable_or_null(32) i8* @malloc(i64 32)
  %calltmp41 = call i32 (i8*, i8*, ...) @sprintf(i8* noundef nonnull dereferenceable(1) %malloc_call34, i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.7, i64 0, i64 0), i64 %calltmp31)
  %calltmp43 = call i64 @strlen(i8* noundef nonnull dereferenceable(1) %malloc_call34)
  %emit51 = call i32 @puts(i8* noundef nonnull dereferenceable(1) %malloc_call34)
  call void @free(i8* %malloc_call34)
  %emit61 = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([130 x i8], [130 x i8]* @strtmp.8, i64 0, i64 0))
  ret i32 0
}
