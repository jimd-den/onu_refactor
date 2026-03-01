; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [21 x i8] c"PARITY VERIFICATION:\00", align 1
@strtmp.1 = private unnamed_addr constant [22 x i8] c"Is 10 even? (1=yes): \00", align 1
@strtmp.2 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1
@strtmp.3 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@strtmp.4 = private unnamed_addr constant [22 x i8] c"Is 7 even?  (1=yes): \00", align 1
@strtmp.5 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1
@strtmp.6 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define fastcc i64 @is-even(i64 %0) {
bb0:
  %cmptmp = icmp eq i64 %0, 0
  br i1 %cmptmp, label %common.ret, label %bb2

common.ret:                                       ; preds = %bb0, %bb2
  %common.ret.op = phi i64 [ %calltmp, %bb2 ], [ 1, %bb0 ]
  ret i64 %common.ret.op

bb2:                                              ; preds = %bb0
  %subtmp = add i64 %0, -1
  %calltmp = call fastcc i64 @is-odd(i64 %subtmp)
  br label %common.ret
}

define fastcc i64 @is-odd(i64 %0) {
bb0:
  %cmptmp = icmp eq i64 %0, 0
  br i1 %cmptmp, label %common.ret, label %bb2

common.ret:                                       ; preds = %bb0, %bb2
  %common.ret.op = phi i64 [ %calltmp, %bb2 ], [ 0, %bb0 ]
  ret i64 %common.ret.op

bb2:                                              ; preds = %bb0
  %subtmp = add i64 %0, -1
  %calltmp = call fastcc i64 @is-even(i64 %subtmp)
  br label %common.ret
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %calltmp = call fastcc i64 @is-even(i64 10)
  %calltmp4 = call fastcc i64 @is-even(i64 7)
  %emit = call i32 @puts(i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([21 x i8], [21 x i8]* @strtmp, i64 0, i64 0))
  %malloc_call = call dereferenceable_or_null(32) i8* @malloc(i64 32)
  %calltmp12 = call i32 (i8*, i8*, ...) @sprintf(i8* noundef nonnull dereferenceable(1) %malloc_call, i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.2, i64 0, i64 0), i64 %calltmp)
  %calltmp14 = call i64 @strlen(i8* noundef nonnull dereferenceable(1) %malloc_call)
  %addtmp = add i64 %calltmp14, 21
  %addtmp29 = add i64 %calltmp14, 22
  %malloc_call31 = call i8* @malloc(i64 %addtmp29)
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* noundef nonnull align 1 dereferenceable(21) %malloc_call31, i8* noundef nonnull align 1 dereferenceable(21) getelementptr inbounds ([22 x i8], [22 x i8]* @strtmp.1, i64 0, i64 0), i64 21, i1 false)
  %offset_ptr = getelementptr inbounds i8, i8* %malloc_call31, i64 21
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* nonnull align 1 %offset_ptr, i8* align 1 %malloc_call, i64 %calltmp14, i1 false)
  %offset_ptr42 = getelementptr inbounds i8, i8* %malloc_call31, i64 %addtmp
  store i8 0, i8* %offset_ptr42, align 1
  call void @free(i8* %malloc_call)
  %emit55 = call i32 @puts(i8* noundef nonnull dereferenceable(1) %malloc_call31)
  call void @free(i8* %malloc_call31)
  %malloc_call64 = call dereferenceable_or_null(32) i8* @malloc(i64 32)
  %calltmp70 = call i32 (i8*, i8*, ...) @sprintf(i8* noundef nonnull dereferenceable(1) %malloc_call64, i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.5, i64 0, i64 0), i64 %calltmp4)
  %calltmp72 = call i64 @strlen(i8* noundef nonnull dereferenceable(1) %malloc_call64)
  %addtmp86 = add i64 %calltmp72, 21
  %addtmp92 = add i64 %calltmp72, 22
  %malloc_call94 = call i8* @malloc(i64 %addtmp92)
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* noundef nonnull align 1 dereferenceable(21) %malloc_call94, i8* noundef nonnull align 1 dereferenceable(21) getelementptr inbounds ([22 x i8], [22 x i8]* @strtmp.4, i64 0, i64 0), i64 21, i1 false)
  %offset_ptr100 = getelementptr inbounds i8, i8* %malloc_call94, i64 21
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* nonnull align 1 %offset_ptr100, i8* align 1 %malloc_call64, i64 %calltmp72, i1 false)
  %offset_ptr106 = getelementptr inbounds i8, i8* %malloc_call94, i64 %addtmp86
  store i8 0, i8* %offset_ptr106, align 1
  call void @free(i8* %malloc_call64)
  %emit126 = call i32 @puts(i8* noundef nonnull dereferenceable(1) %malloc_call94)
  call void @free(i8* %malloc_call94)
  ret i32 0
}

; Function Attrs: argmemonly nofree nounwind willreturn
declare void @llvm.memcpy.p0i8.p0i8.i64(i8* noalias nocapture writeonly, i8* noalias nocapture readonly, i64, i1 immarg) #0

attributes #0 = { argmemonly nofree nounwind willreturn }
