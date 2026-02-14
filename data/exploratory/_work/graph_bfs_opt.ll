; ModuleID = 'data/exploratory/_work/graph_bfs.ll'
source_filename = "benchmarks/graph_bfs.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@lcg_state = internal global i32 12345, align 4
@sink = internal global i32 0, align 4
@degree = internal global [10000 x i32] zeroinitializer, align 16
@adj_offset = internal global [10001 x i32] zeroinitializer, align 16
@adj = internal global [50000 x i32] zeroinitializer, align 16
@visited = internal global [10000 x i32] zeroinitializer, align 16
@queue = internal global [10000 x i32] zeroinitializer, align 16

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
.lr.ph.preheader:
  %0 = alloca [50 x i64], align 16
  %1 = alloca %struct.timespec, align 8
  %2 = alloca %struct.timespec, align 8
  call void @run_benchmark()
  call void @run_benchmark()
  call void @run_benchmark()
  call void @run_benchmark()
  call void @run_benchmark()
  br label %.lr.ph2

.lr.ph2:                                          ; preds = %.lr.ph.preheader, %.lr.ph2
  %.0 = phi i32 [ %8, %.lr.ph2 ], [ 0, %.lr.ph.preheader ]
  %3 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %1) #7
  call void @run_benchmark()
  %4 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %2) #7
  %5 = call i64 @timespec_diff_ns(ptr noundef nonnull %1, ptr noundef nonnull %2)
  %6 = zext nneg i32 %.0 to i64
  %7 = getelementptr inbounds nuw [50 x i64], ptr %0, i64 0, i64 %6
  store i64 %5, ptr %7, align 8
  %8 = add nuw nsw i32 %.0, 1
  %9 = icmp samesign ult i32 %.0, 49
  br i1 %9, label %.lr.ph2, label %._crit_edge3, !llvm.loop !6

._crit_edge3:                                     ; preds = %.lr.ph2
  call void @qsort(ptr noundef nonnull %0, i64 noundef 50, i64 noundef 8, ptr noundef nonnull @cmp_ll) #7
  %10 = getelementptr inbounds nuw i8, ptr %0, i64 200
  %11 = load i64, ptr %10, align 8
  %12 = call i32 (ptr, ...) @printf(ptr noundef nonnull dereferenceable(1) @.str, i64 noundef %11) #7
  ret i32 0
}

; Function Attrs: noinline nounwind uwtable
define internal void @run_benchmark() #0 {
  store i32 12345, ptr @lcg_state, align 4
  call void @build_graph()
  %1 = call i32 @bfs_from(i32 noundef 0)
  store volatile i32 %1, ptr @sink, align 4
  ret void
}

; Function Attrs: nounwind
declare i32 @clock_gettime(i32 noundef, ptr noundef) #1

; Function Attrs: noinline nounwind uwtable
define internal i64 @timespec_diff_ns(ptr noundef %0, ptr noundef %1) #0 {
  %3 = load i64, ptr %1, align 8
  %4 = load i64, ptr %0, align 8
  %5 = sub nsw i64 %3, %4
  %6 = mul nsw i64 %5, 1000000000
  %7 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %8 = load i64, ptr %7, align 8
  %9 = getelementptr inbounds nuw i8, ptr %0, i64 8
  %10 = load i64, ptr %9, align 8
  %11 = sub nsw i64 %8, %10
  %12 = add nsw i64 %6, %11
  ret i64 %12
}

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #2

; Function Attrs: noinline nounwind uwtable
define internal i32 @cmp_ll(ptr noundef %0, ptr noundef %1) #0 {
  %3 = load i64, ptr %0, align 8
  %4 = load i64, ptr %1, align 8
  %5 = call i32 @llvm.scmp.i32.i64(i64 %3, i64 %4)
  ret i32 %5
}

declare i32 @printf(ptr noundef, ...) #2

; Function Attrs: noinline nounwind uwtable
define internal void @build_graph() #0 {
.lr.ph.preheader:
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 16 dereferenceable(40000) @degree, i8 0, i64 40000, i1 false)
  %0 = call noalias dereferenceable_or_null(200000) ptr @malloc(i64 noundef 200000) #8
  %1 = call noalias dereferenceable_or_null(200000) ptr @malloc(i64 noundef 200000) #8
  br label %.lr.ph

.lr.ph:                                           ; preds = %.lr.ph.preheader, %.lr.ph
  %.08 = phi i32 [ %13, %.lr.ph ], [ 0, %.lr.ph.preheader ]
  %2 = call i32 @lcg_rand()
  %3 = urem i32 %2, 10000
  %4 = zext nneg i32 %.08 to i64
  %5 = getelementptr inbounds nuw i32, ptr %0, i64 %4
  store i32 %3, ptr %5, align 4
  %6 = call i32 @lcg_rand()
  %7 = urem i32 %6, 10000
  %8 = getelementptr inbounds nuw i32, ptr %1, i64 %4
  store i32 %7, ptr %8, align 4
  %9 = sext i32 %3 to i64
  %10 = getelementptr inbounds [10000 x i32], ptr @degree, i64 0, i64 %9
  %11 = load i32, ptr %10, align 4
  %12 = add nsw i32 %11, 1
  store i32 %12, ptr %10, align 4
  %13 = add nuw nsw i32 %.08, 1
  %14 = icmp samesign ult i32 %.08, 49999
  br i1 %14, label %.lr.ph, label %._crit_edge, !llvm.loop !8

._crit_edge:                                      ; preds = %.lr.ph
  store i32 0, ptr @adj_offset, align 16
  br label %.lr.ph2

.lr.ph2:                                          ; preds = %.lr.ph2, %._crit_edge
  %15 = phi i32 [ 0, %._crit_edge ], [ %25, %.lr.ph2 ]
  %.07 = phi i32 [ 0, %._crit_edge ], [ %26, %.lr.ph2 ]
  %16 = zext nneg i32 %.07 to i64
  %17 = getelementptr inbounds nuw [10000 x i32], ptr @degree, i64 0, i64 %16
  %18 = load i32, ptr %17, align 4
  %19 = add nsw i32 %15, %18
  %20 = or disjoint i32 %.07, 1
  %21 = zext nneg i32 %20 to i64
  %22 = getelementptr inbounds nuw [10001 x i32], ptr @adj_offset, i64 0, i64 %21
  store i32 %19, ptr %22, align 4
  %23 = getelementptr inbounds nuw [10000 x i32], ptr @degree, i64 0, i64 %21
  %24 = load i32, ptr %23, align 4
  %25 = add nsw i32 %19, %24
  %26 = add nuw nsw i32 %.07, 2
  %27 = zext nneg i32 %26 to i64
  %28 = getelementptr inbounds nuw [10001 x i32], ptr @adj_offset, i64 0, i64 %27
  store i32 %25, ptr %28, align 4
  %29 = icmp samesign ult i32 %.07, 9998
  br i1 %29, label %.lr.ph2, label %._crit_edge3, !llvm.loop !9

._crit_edge3:                                     ; preds = %.lr.ph2
  %30 = call noalias dereferenceable_or_null(40000) ptr @calloc(i64 noundef 10000, i64 noundef 4) #9
  br label %.lr.ph5

.lr.ph5:                                          ; preds = %._crit_edge3, %.lr.ph5
  %.0 = phi i32 [ %45, %.lr.ph5 ], [ 0, %._crit_edge3 ]
  %31 = zext nneg i32 %.0 to i64
  %32 = getelementptr inbounds nuw i32, ptr %0, i64 %31
  %33 = load i32, ptr %32, align 4
  %34 = getelementptr inbounds nuw i32, ptr %1, i64 %31
  %35 = load i32, ptr %34, align 4
  %36 = sext i32 %33 to i64
  %37 = getelementptr inbounds [10001 x i32], ptr @adj_offset, i64 0, i64 %36
  %38 = load i32, ptr %37, align 4
  %39 = getelementptr inbounds i32, ptr %30, i64 %36
  %40 = load i32, ptr %39, align 4
  %41 = add nsw i32 %38, %40
  %42 = sext i32 %41 to i64
  %43 = getelementptr inbounds [50000 x i32], ptr @adj, i64 0, i64 %42
  store i32 %35, ptr %43, align 4
  %44 = add nsw i32 %40, 1
  store i32 %44, ptr %39, align 4
  %45 = add nuw nsw i32 %.0, 1
  %46 = icmp samesign ult i32 %.0, 49999
  br i1 %46, label %.lr.ph5, label %._crit_edge6, !llvm.loop !10

._crit_edge6:                                     ; preds = %.lr.ph5
  call void @free(ptr noundef %0) #7
  call void @free(ptr noundef %1) #7
  call void @free(ptr noundef %30) #7
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal i32 @bfs_from(i32 noundef %0) #0 {
.lr.ph2.preheader:
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 16 dereferenceable(40000) @visited, i8 0, i64 40000, i1 false)
  %1 = sext i32 %0 to i64
  %2 = getelementptr inbounds [10000 x i32], ptr @visited, i64 0, i64 %1
  store i32 1, ptr %2, align 4
  store i32 %0, ptr @queue, align 4
  br label %.lr.ph2

.lr.ph2:                                          ; preds = %.lr.ph2.preheader, %._crit_edge
  %.06 = phi i32 [ %.39, %._crit_edge ], [ 1, %.lr.ph2.preheader ]
  %.05 = phi i32 [ %.2, %._crit_edge ], [ 1, %.lr.ph2.preheader ]
  %.04 = phi i32 [ %3, %._crit_edge ], [ 0, %.lr.ph2.preheader ]
  %3 = add nuw nsw i32 %.04, 1
  %4 = zext nneg i32 %.04 to i64
  %5 = getelementptr inbounds nuw [10000 x i32], ptr @queue, i64 0, i64 %4
  %6 = load i32, ptr %5, align 4
  %7 = sext i32 %6 to i64
  %8 = getelementptr inbounds [10001 x i32], ptr @adj_offset, i64 0, i64 %7
  %9 = load i32, ptr %8, align 4
  %10 = add nsw i32 %6, 1
  %11 = sext i32 %10 to i64
  %12 = getelementptr inbounds [10001 x i32], ptr @adj_offset, i64 0, i64 %11
  %13 = load i32, ptr %12, align 4
  %14 = icmp slt i32 %9, %13
  br i1 %14, label %.lr.ph, label %._crit_edge

.lr.ph:                                           ; preds = %.lr.ph2, %26
  %.28 = phi i32 [ %.4, %26 ], [ %.06, %.lr.ph2 ]
  %.1 = phi i32 [ %.3, %26 ], [ %.05, %.lr.ph2 ]
  %.0 = phi i32 [ %27, %26 ], [ %9, %.lr.ph2 ]
  %15 = sext i32 %.0 to i64
  %16 = getelementptr inbounds [50000 x i32], ptr @adj, i64 0, i64 %15
  %17 = load i32, ptr %16, align 4
  %18 = sext i32 %17 to i64
  %19 = getelementptr inbounds [10000 x i32], ptr @visited, i64 0, i64 %18
  %20 = load i32, ptr %19, align 4
  %.not = icmp eq i32 %20, 0
  br i1 %.not, label %21, label %26

21:                                               ; preds = %.lr.ph
  store i32 1, ptr %19, align 4
  %22 = add nsw i32 %.1, 1
  %23 = sext i32 %.1 to i64
  %24 = getelementptr inbounds [10000 x i32], ptr @queue, i64 0, i64 %23
  store i32 %17, ptr %24, align 4
  %25 = add nsw i32 %.28, 1
  br label %26

26:                                               ; preds = %.lr.ph, %21
  %.4 = phi i32 [ %.28, %.lr.ph ], [ %25, %21 ]
  %.3 = phi i32 [ %.1, %.lr.ph ], [ %22, %21 ]
  %27 = add nsw i32 %.0, 1
  %28 = icmp slt i32 %27, %13
  br i1 %28, label %.lr.ph, label %._crit_edge, !llvm.loop !11

._crit_edge:                                      ; preds = %26, %.lr.ph2
  %.39 = phi i32 [ %.06, %.lr.ph2 ], [ %.4, %26 ]
  %.2 = phi i32 [ %.05, %.lr.ph2 ], [ %.3, %26 ]
  %29 = icmp slt i32 %3, %.2
  br i1 %29, label %.lr.ph2, label %._crit_edge3, !llvm.loop !12

._crit_edge3:                                     ; preds = %._crit_edge
  ret i32 %.39
}

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #3

; Function Attrs: nounwind allocsize(0)
declare noalias ptr @malloc(i64 noundef) #4

; Function Attrs: noinline nounwind uwtable
define internal i32 @lcg_rand() #0 {
  %1 = load i32, ptr @lcg_state, align 4
  %2 = mul i32 %1, 1103515245
  %3 = add i32 %2, 12345
  store i32 %3, ptr @lcg_state, align 4
  %4 = lshr i32 %3, 16
  %5 = and i32 %4, 32767
  ret i32 %5
}

; Function Attrs: nounwind allocsize(0,1)
declare noalias ptr @calloc(i64 noundef, i64 noundef) #5

; Function Attrs: nounwind
declare void @free(ptr noundef) #1

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.scmp.i32.i64(i64, i64) #6

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #4 = { nounwind allocsize(0) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #5 = { nounwind allocsize(0,1) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #6 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #7 = { nounwind }
attributes #8 = { nounwind allocsize(0) }
attributes #9 = { nounwind allocsize(0,1) }

!llvm.module.flags = !{!0, !1, !2, !3, !4}
!llvm.ident = !{!5}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{i32 7, !"PIE Level", i32 2}
!3 = !{i32 7, !"uwtable", i32 2}
!4 = !{i32 7, !"frame-pointer", i32 2}
!5 = !{!"Ubuntu clang version 20.1.2 (0ubuntu1~24.04.2)"}
!6 = distinct !{!6, !7}
!7 = !{!"llvm.loop.mustprogress"}
!8 = distinct !{!8, !7}
!9 = distinct !{!9, !7}
!10 = distinct !{!10, !7}
!11 = distinct !{!11, !7}
!12 = distinct !{!12, !7}
