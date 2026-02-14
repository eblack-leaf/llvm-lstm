; ModuleID = 'data/exploratory/_work/binary_tree.ll'
source_filename = "benchmarks/binary_tree.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.Node = type { i32, ptr, ptr }
%struct.timespec = type { i64, i64 }

@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@lcg_state = internal global i32 12345, align 4
@pool_idx = internal global i32 0, align 4
@sink = internal global i64 0, align 8
@pool = internal global [200000 x %struct.Node] zeroinitializer, align 16

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca [50 x i64], align 16
  %2 = alloca %struct.timespec, align 8
  %3 = alloca %struct.timespec, align 8
  br i1 true, label %.lr.ph, label %._crit_edge4

._crit_edge4:                                     ; preds = %0
  br label %8

.lr.ph:                                           ; preds = %0
  br label %4

4:                                                ; preds = %.lr.ph, %4
  %5 = phi i32 [ 0, %.lr.ph ], [ %6, %4 ]
  tail call void @run_benchmark()
  %6 = add nsw i32 %5, 1
  %7 = icmp slt i32 %6, 5
  br i1 %7, label %4, label %._crit_edge, !llvm.loop !6

._crit_edge:                                      ; preds = %4
  br label %8

8:                                                ; preds = %._crit_edge4, %._crit_edge
  br i1 true, label %.lr.ph2, label %._crit_edge5

._crit_edge5:                                     ; preds = %8
  br label %18

.lr.ph2:                                          ; preds = %8
  br label %9

9:                                                ; preds = %.lr.ph2, %9
  %10 = phi i32 [ 0, %.lr.ph2 ], [ %16, %9 ]
  %11 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %2) #4
  call void @run_benchmark()
  %12 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %3) #4
  %13 = call i64 @timespec_diff_ns(ptr noundef %2, ptr noundef %3)
  %14 = sext i32 %10 to i64
  %15 = getelementptr inbounds [50 x i64], ptr %1, i64 0, i64 %14
  store i64 %13, ptr %15, align 8
  %16 = add nsw i32 %10, 1
  %17 = icmp slt i32 %16, 50
  br i1 %17, label %9, label %._crit_edge3, !llvm.loop !8

._crit_edge3:                                     ; preds = %9
  br label %18

18:                                               ; preds = %._crit_edge5, %._crit_edge3
  call void @qsort(ptr noundef %1, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %19 = getelementptr inbounds [50 x i64], ptr %1, i64 0, i64 25
  %20 = load i64, ptr %19, align 8
  %21 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %20)
  ret i32 0
}

; Function Attrs: noinline nounwind uwtable
define internal void @run_benchmark() #0 {
  store i32 12345, ptr @lcg_state, align 4
  store i32 0, ptr @pool_idx, align 4
  br i1 true, label %.lr.ph, label %._crit_edge1

._crit_edge1:                                     ; preds = %0
  br label %11

.lr.ph:                                           ; preds = %0
  br label %1

1:                                                ; preds = %.lr.ph, %1
  %2 = phi i32 [ 0, %.lr.ph ], [ %9, %1 ]
  %3 = phi ptr [ null, %.lr.ph ], [ %8, %1 ]
  %4 = tail call i32 @lcg_rand()
  %5 = shl i32 %4, 15
  %6 = tail call i32 @lcg_rand()
  %7 = or i32 %5, %6
  %8 = tail call ptr @bst_insert(ptr noundef %3, i32 noundef %7)
  %9 = add nsw i32 %2, 1
  %10 = icmp slt i32 %9, 200000
  br i1 %10, label %1, label %._crit_edge, !llvm.loop !9

._crit_edge:                                      ; preds = %1
  br label %11

11:                                               ; preds = %._crit_edge1, %._crit_edge
  %12 = phi ptr [ null, %._crit_edge1 ], [ %8, %._crit_edge ]
  %13 = tail call i64 @inorder_sum(ptr noundef %12)
  store volatile i64 %13, ptr @sink, align 8
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
  %7 = getelementptr inbounds nuw %struct.timespec, ptr %1, i32 0, i32 1
  %8 = load i64, ptr %7, align 8
  %9 = getelementptr inbounds nuw %struct.timespec, ptr %0, i32 0, i32 1
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
  %5 = icmp sgt i64 %3, %4
  %6 = zext i1 %5 to i32
  %7 = icmp slt i64 %3, %4
  %8 = zext i1 %7 to i32
  %9 = sub nsw i32 %6, %8
  ret i32 %9
}

declare i32 @printf(ptr noundef, ...) #2

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

; Function Attrs: noinline nounwind uwtable
define internal ptr @bst_insert(ptr noundef %0, i32 noundef %1) #0 {
  %3 = icmp ne ptr %0, null
  br i1 %3, label %6, label %4

4:                                                ; preds = %2
  %5 = tail call ptr @node_alloc(i32 noundef %1)
  br label %32

6:                                                ; preds = %2
  br label %7

7:                                                ; preds = %29, %6
  %8 = phi ptr [ %30, %29 ], [ %0, %6 ]
  %9 = phi i32 [ %31, %29 ], [ %1, %6 ]
  %10 = load i32, ptr %8, align 8
  %11 = icmp slt i32 %9, %10
  br i1 %11, label %12, label %19

12:                                               ; preds = %7
  %13 = getelementptr inbounds nuw %struct.Node, ptr %8, i32 0, i32 1
  %14 = load ptr, ptr %13, align 8
  %15 = icmp ne ptr %14, null
  br i1 %15, label %18, label %16

16:                                               ; preds = %12
  %17 = tail call ptr @node_alloc(i32 noundef %1)
  store ptr %17, ptr %13, align 8
  br label %32

18:                                               ; preds = %12
  br label %29

19:                                               ; preds = %7
  %20 = icmp sgt i32 %1, %10
  br i1 %20, label %21, label %28

21:                                               ; preds = %19
  %22 = getelementptr inbounds nuw %struct.Node, ptr %8, i32 0, i32 2
  %23 = load ptr, ptr %22, align 8
  %24 = icmp ne ptr %23, null
  br i1 %24, label %27, label %25

25:                                               ; preds = %21
  %26 = tail call ptr @node_alloc(i32 noundef %1)
  store ptr %26, ptr %22, align 8
  br label %32

27:                                               ; preds = %21
  br label %29

28:                                               ; preds = %19
  br label %32

29:                                               ; preds = %27, %18
  %30 = phi ptr [ %23, %27 ], [ %14, %18 ]
  %31 = phi i32 [ %1, %27 ], [ %9, %18 ]
  br label %7

32:                                               ; preds = %28, %25, %16, %4
  %33 = phi ptr [ %0, %28 ], [ %0, %25 ], [ %0, %16 ], [ %5, %4 ]
  ret ptr %33
}

; Function Attrs: noinline nounwind uwtable
define internal i64 @inorder_sum(ptr noundef %0) #0 {
  %2 = tail call noalias ptr @malloc(i64 noundef 1600000) #5
  br label %3

3:                                                ; preds = %22, %1
  %4 = phi i64 [ %30, %22 ], [ 0, %1 ]
  %5 = phi i32 [ %24, %22 ], [ 0, %1 ]
  %6 = phi ptr [ %32, %22 ], [ %0, %1 ]
  %7 = icmp ne ptr %6, null
  br i1 %7, label %10, label %8

8:                                                ; preds = %3
  %9 = icmp sgt i32 %5, 0
  br label %10

10:                                               ; preds = %8, %3
  %11 = phi i1 [ true, %3 ], [ %9, %8 ]
  br i1 %11, label %12, label %33

12:                                               ; preds = %10
  br i1 %7, label %.lr.ph, label %._crit_edge1

._crit_edge1:                                     ; preds = %12
  br label %22

.lr.ph:                                           ; preds = %12
  br label %13

13:                                               ; preds = %.lr.ph, %13
  %14 = phi i32 [ %5, %.lr.ph ], [ %16, %13 ]
  %15 = phi ptr [ %6, %.lr.ph ], [ %20, %13 ]
  %16 = add nsw i32 %14, 1
  %17 = sext i32 %14 to i64
  %18 = getelementptr inbounds ptr, ptr %2, i64 %17
  store ptr %15, ptr %18, align 8
  %19 = getelementptr inbounds nuw %struct.Node, ptr %15, i32 0, i32 1
  %20 = load ptr, ptr %19, align 8
  %21 = icmp ne ptr %20, null
  br i1 %21, label %13, label %._crit_edge, !llvm.loop !10

._crit_edge:                                      ; preds = %13
  br label %22

22:                                               ; preds = %._crit_edge1, %._crit_edge
  %23 = phi i32 [ %16, %._crit_edge ], [ %5, %._crit_edge1 ]
  %24 = add nsw i32 %23, -1
  %25 = sext i32 %24 to i64
  %26 = getelementptr inbounds ptr, ptr %2, i64 %25
  %27 = load ptr, ptr %26, align 8
  %28 = load i32, ptr %27, align 8
  %29 = sext i32 %28 to i64
  %30 = add nsw i64 %4, %29
  %31 = getelementptr inbounds nuw %struct.Node, ptr %27, i32 0, i32 2
  %32 = load ptr, ptr %31, align 8
  br label %3, !llvm.loop !11

33:                                               ; preds = %10
  tail call void @free(ptr noundef %2) #4
  ret i64 %4
}

; Function Attrs: noinline nounwind uwtable
define internal ptr @node_alloc(i32 noundef %0) #0 {
  %2 = load i32, ptr @pool_idx, align 4
  %3 = add nsw i32 %2, 1
  store i32 %3, ptr @pool_idx, align 4
  %4 = sext i32 %2 to i64
  %5 = getelementptr inbounds [200000 x %struct.Node], ptr @pool, i64 0, i64 %4
  store i32 %0, ptr %5, align 8
  %6 = getelementptr inbounds nuw %struct.Node, ptr %5, i32 0, i32 1
  store ptr null, ptr %6, align 8
  %7 = getelementptr inbounds nuw %struct.Node, ptr %5, i32 0, i32 2
  store ptr null, ptr %7, align 8
  ret ptr %5
}

; Function Attrs: nounwind allocsize(0)
declare noalias ptr @malloc(i64 noundef) #3

; Function Attrs: nounwind
declare void @free(ptr noundef) #1

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nounwind allocsize(0) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { nounwind }
attributes #5 = { nounwind allocsize(0) }

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
