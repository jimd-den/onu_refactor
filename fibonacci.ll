; ModuleID = 'onu_discourse'
source_filename = "onu_discourse"

@strtmp = private unnamed_addr constant [30 x i8] c"The population at generation \00", align 1
@strtmp.1 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1
@strtmp.2 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@strtmp.3 = private unnamed_addr constant [15 x i8] c" has reached: \00", align 1
@strtmp.4 = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@strtmp.5 = private unnamed_addr constant [5 x i8] c"%lld\00", align 1

declare i8* @malloc(i64)

declare void @free(i8*)

declare i32 @printf(i8*, ...)

declare i32 @puts(i8*)

declare i32 @sprintf(i8*, i8*, ...)

declare i64 @strlen(i8*)

define fastcc i64 @origin-size() {
bb0:
  ret i64 0
}

define fastcc i64 @spark-size() {
bb0:
  ret i64 1
}

define fastcc i64 @calculate-growth(i64 %0) {
bb0:
  switch i64 %0, label %bb4 [
    i64 0, label %bb1
    i64 1, label %bb3
  ]

common.ret:                                       ; preds = %bb4, %bb3, %bb1
  %common.ret.op = phi i64 [ %calltmp, %bb1 ], [ %calltmp9, %bb3 ], [ %addtmp, %bb4 ]
  ret i64 %common.ret.op

bb1:                                              ; preds = %bb0
  %calltmp = call fastcc i64 @origin-size()
  br label %common.ret

bb3:                                              ; preds = %bb0
  %calltmp9 = call fastcc i64 @spark-size()
  br label %common.ret

bb4:                                              ; preds = %bb0
  %subtmp12 = add i64 %0, -1
  %subtmp15 = add i64 %0, -2
  %calltmp18 = call fastcc i64 @calculate-growth(i64 %subtmp12)
  %calltmp22 = call fastcc i64 @calculate-growth(i64 %subtmp15)
  %addtmp = add i64 %calltmp22, %calltmp18
  br label %common.ret
}

define i32 @main(i32 %0, i64 %1) {
bb0:
  %calltmp = call fastcc i64 @calculate-growth(i64 40)
  %malloc_call = call dereferenceable_or_null(32) i8* @malloc(i64 32)
  %calltmp8 = call i32 (i8*, i8*, ...) @sprintf(i8* noundef nonnull dereferenceable(1) %malloc_call, i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.1, i64 0, i64 0), i64 40)
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
  %addtmp55 = add i64 %calltmp10, 43
  %addtmp62 = add i64 %calltmp10, 44
  %malloc_call64 = call i8* @malloc(i64 %addtmp62)
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* align 1 %malloc_call64, i8* align 1 %malloc_call26, i64 %addtmp, i1 false)
  %offset_ptr70 = getelementptr inbounds i8, i8* %malloc_call64, i64 %addtmp
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* noundef nonnull align 1 dereferenceable(14) %offset_ptr70, i8* noundef nonnull align 1 dereferenceable(14) getelementptr inbounds ([15 x i8], [15 x i8]* @strtmp.3, i64 0, i64 0), i64 14, i1 false)
  %offset_ptr76 = getelementptr inbounds i8, i8* %malloc_call64, i64 %addtmp55
  store i8 0, i8* %offset_ptr76, align 1
  call void @free(i8* %malloc_call26)
  %emit = call i32 @puts(i8* noundef nonnull dereferenceable(1) %malloc_call64)
  call void @free(i8* %malloc_call64)
  %malloc_call104 = call dereferenceable_or_null(32) i8* @malloc(i64 32)
  %calltmp110 = call i32 (i8*, i8*, ...) @sprintf(i8* noundef nonnull dereferenceable(1) %malloc_call104, i8* noundef nonnull dereferenceable(1) getelementptr inbounds ([5 x i8], [5 x i8]* @strtmp.5, i64 0, i64 0), i64 %calltmp)
  %calltmp112 = call i64 @strlen(i8* noundef nonnull dereferenceable(1) %malloc_call104)
  %emit120 = call i32 @puts(i8* noundef nonnull dereferenceable(1) %malloc_call104)
  call void @free(i8* %malloc_call104)
  ret i32 0
}

; Function Attrs: argmemonly nofree nounwind willreturn
declare void @llvm.memcpy.p0i8.p0i8.i64(i8* noalias nocapture writeonly, i8* noalias nocapture readonly, i64, i1 immarg) #0

attributes #0 = { argmemonly nofree nounwind willreturn }
