; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [30 x i8] c"Total Collatz steps for 1 to \00", align 1
@strtmp.1 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1
@strtmp.2 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@strtmp.3 = private unnamed_addr constant [6 x i8] c" is: \00", align 1
@strtmp.4 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@strtmp.5 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define fastcc i64 @collatz-steps(i64 %0, i64 %1) {
bb0:
  %cmptmp = icmp eq i64 %0, 1
  br i1 %cmptmp, label %common.ret, label %bb2

common.ret:                                       ; preds = %bb0, %bb2
  %common.ret.op = phi i64 [ %calltmp, %bb2 ], [ %1, %bb0 ]
  ret i64 %common.ret.op

bb2:                                              ; preds = %bb0
  %divtmp = sdiv i64 %0, 2
  %multmp = shl nsw i64 %divtmp, 1
  %cmptmp7 = icmp eq i64 %multmp, %0
  %multmp14 = mul i64 %0, 3
  %addtmp = add i64 %multmp14, 1
  %v10.0 = select i1 %cmptmp7, i64 %divtmp, i64 %addtmp
  %addtmp19 = add i64 %1, 1
  %calltmp = call fastcc i64 @collatz-steps(i64 %v10.0, i64 %addtmp19)
  br label %common.ret
}

define fastcc i64 @collatz-range(i64 %0, i64 %1, i64 %2) {
bb0:
  %cmptmp = icmp sgt i64 %0, %1
  br i1 %cmptmp, label %common.ret, label %bb2

common.ret:                                       ; preds = %bb0, %bb2
  %common.ret.op = phi i64 [ %calltmp11, %bb2 ], [ %2, %bb0 ]
  ret i64 %common.ret.op

bb2:                                              ; preds = %bb0
  %calltmp = call fastcc i64 @collatz-steps(i64 %0, i64 0)
  %addtmp = add i64 %0, 1
  %addtmp7 = add i64 %calltmp, %2
  %calltmp11 = call fastcc i64 @collatz-range(i64 %addtmp, i64 %1, i64 %addtmp7)
  br label %common.ret
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %calltmp = call fastcc i64 @collatz-range(i64 1, i64 1000000, i64 0)
  %malloc_call = call dereferenceable_or_null(32) i8* @malloc(i64 32)
  %calltmp8 = call i32 (i8*, i8*, ...) @sprintf(i8* noundef nonnull dereferenceable(1) %malloc_call, i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.1, i64 0, i64 0), i64 1000000)
  %calltmp10 = call i64 @strlen(i8* noundef nonnull dereferenceable(1) %malloc_call)
  %addtmp = add i64 %calltmp10, 29
  %addtmp24 = add i64 %calltmp10, 30
  %malloc_call26 = call i8* @malloc(i64 %addtmp24)
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* noundef nonnull align 1 dereferenceable(29) %malloc_call26, i8* noundef nonnull align 1 dereferenceable(29) getelementptr inbounds ([30 x i8], [30 x i8]* @strtmp, i64 0, i64 0), i64 29, i1 false)
  %offset_ptr = getelementptr inbounds i8, i8* %malloc_call26, i64 29
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* nonnull align 1 %offset_ptr, i8* align 1 %malloc_call, i64 %calltmp10, i1 false)
  %offset_ptr38 = getelementptr inbounds i8, i8* %malloc_call26, i64 %addtmp
  store i8 0, i8* %offset_ptr38, align 1
  call void @free(i8* %malloc_call)
  %addtmp55 = add i64 %calltmp10, 34
  %addtmp62 = add i64 %calltmp10, 35
  %malloc_call64 = call i8* @malloc(i64 %addtmp62)
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* align 1 %malloc_call64, i8* align 1 %malloc_call26, i64 %addtmp, i1 false)
  %offset_ptr70 = getelementptr inbounds i8, i8* %malloc_call64, i64 %addtmp
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* noundef nonnull align 1 dereferenceable(5) %offset_ptr70, i8* noundef nonnull align 1 dereferenceable(5) getelementptr inbounds ([6 x i8], [6 x i8]* @strtmp.3, i64 0, i64 0), i64 5, i1 false)
  %offset_ptr76 = getelementptr inbounds i8, i8* %malloc_call64, i64 %addtmp55
  store i8 0, i8* %offset_ptr76, align 1
  %emit = call i32 @puts(i8* noundef nonnull dereferenceable(1) %malloc_call64)
  %malloc_call90 = call dereferenceable_or_null(32) i8* @malloc(i64 32)
  %calltmp96 = call i32 (i8*, i8*, ...) @sprintf(i8* noundef nonnull dereferenceable(1) %malloc_call90, i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.5, i64 0, i64 0), i64 %calltmp)
  %calltmp98 = call i64 @strlen(i8* noundef nonnull dereferenceable(1) %malloc_call90)
  %emit106 = call i32 @puts(i8* noundef nonnull dereferenceable(1) %malloc_call90)
  call void @free(i8* %malloc_call90)
  ret i32 0
}

; Function Attrs: argmemonly nofree nounwind willreturn
declare void @llvm.memcpy.p0i8.p0i8.i64(i8* noalias nocapture writeonly, i8* noalias nocapture readonly, i64, i1 immarg) #0

attributes #0 = { argmemonly nofree nounwind willreturn }
