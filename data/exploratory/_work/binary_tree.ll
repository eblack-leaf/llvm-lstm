; ModuleID = 'benchmarks/binary_tree.c'
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
  %1 = alloca i32, align 4
  %2 = alloca i32, align 4
  %3 = alloca [50 x i64], align 16
  %4 = alloca i32, align 4
  %5 = alloca %struct.timespec, align 8
  %6 = alloca %struct.timespec, align 8
  store i32 0, ptr %1, align 4
  store i32 0, ptr %2, align 4
  br label %7

7:                                                ; preds = %11, %0
  %8 = load i32, ptr %2, align 4
  %9 = icmp slt i32 %8, 5
  br i1 %9, label %10, label %14

10:                                               ; preds = %7
  call void @run_benchmark()
  br label %11

11:                                               ; preds = %10
  %12 = load i32, ptr %2, align 4
  %13 = add nsw i32 %12, 1
  store i32 %13, ptr %2, align 4
  br label %7, !llvm.loop !6

14:                                               ; preds = %7
  store i32 0, ptr %4, align 4
  br label %15

15:                                               ; preds = %25, %14
  %16 = load i32, ptr %4, align 4
  %17 = icmp slt i32 %16, 50
  br i1 %17, label %18, label %28

18:                                               ; preds = %15
  %19 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %5) #4
  call void @run_benchmark()
  %20 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %6) #4
  %21 = call i64 @timespec_diff_ns(ptr noundef %5, ptr noundef %6)
  %22 = load i32, ptr %4, align 4
  %23 = sext i32 %22 to i64
  %24 = getelementptr inbounds [50 x i64], ptr %3, i64 0, i64 %23
  store i64 %21, ptr %24, align 8
  br label %25

25:                                               ; preds = %18
  %26 = load i32, ptr %4, align 4
  %27 = add nsw i32 %26, 1
  store i32 %27, ptr %4, align 4
  br label %15, !llvm.loop !8

28:                                               ; preds = %15
  %29 = getelementptr inbounds [50 x i64], ptr %3, i64 0, i64 0
  call void @qsort(ptr noundef %29, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %30 = getelementptr inbounds [50 x i64], ptr %3, i64 0, i64 25
  %31 = load i64, ptr %30, align 8
  %32 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %31)
  ret i32 0
}

; Function Attrs: noinline nounwind uwtable
define internal void @run_benchmark() #0 {
  %1 = alloca ptr, align 8
  %2 = alloca i32, align 4
  %3 = alloca i32, align 4
  store i32 12345, ptr @lcg_state, align 4
  store i32 0, ptr @pool_idx, align 4
  store ptr null, ptr %1, align 8
  store i32 0, ptr %2, align 4
  br label %4

4:                                                ; preds = %15, %0
  %5 = load i32, ptr %2, align 4
  %6 = icmp slt i32 %5, 200000
  br i1 %6, label %7, label %18

7:                                                ; preds = %4
  %8 = call i32 @lcg_rand()
  %9 = shl i32 %8, 15
  %10 = call i32 @lcg_rand()
  %11 = or i32 %9, %10
  store i32 %11, ptr %3, align 4
  %12 = load ptr, ptr %1, align 8
  %13 = load i32, ptr %3, align 4
  %14 = call ptr @bst_insert(ptr noundef %12, i32 noundef %13)
  store ptr %14, ptr %1, align 8
  br label %15

15:                                               ; preds = %7
  %16 = load i32, ptr %2, align 4
  %17 = add nsw i32 %16, 1
  store i32 %17, ptr %2, align 4
  br label %4, !llvm.loop !9

18:                                               ; preds = %4
  %19 = load ptr, ptr %1, align 8
  %20 = call i64 @inorder_sum(ptr noundef %19)
  store volatile i64 %20, ptr @sink, align 8
  ret void
}

; Function Attrs: nounwind
declare i32 @clock_gettime(i32 noundef, ptr noundef) #1

; Function Attrs: noinline nounwind uwtable
define internal i64 @timespec_diff_ns(ptr noundef %0, ptr noundef %1) #0 {
  %3 = alloca ptr, align 8
  %4 = alloca ptr, align 8
  store ptr %0, ptr %3, align 8
  store ptr %1, ptr %4, align 8
  %5 = load ptr, ptr %4, align 8
  %6 = getelementptr inbounds nuw %struct.timespec, ptr %5, i32 0, i32 0
  %7 = load i64, ptr %6, align 8
  %8 = load ptr, ptr %3, align 8
  %9 = getelementptr inbounds nuw %struct.timespec, ptr %8, i32 0, i32 0
  %10 = load i64, ptr %9, align 8
  %11 = sub nsw i64 %7, %10
  %12 = mul nsw i64 %11, 1000000000
  %13 = load ptr, ptr %4, align 8
  %14 = getelementptr inbounds nuw %struct.timespec, ptr %13, i32 0, i32 1
  %15 = load i64, ptr %14, align 8
  %16 = load ptr, ptr %3, align 8
  %17 = getelementptr inbounds nuw %struct.timespec, ptr %16, i32 0, i32 1
  %18 = load i64, ptr %17, align 8
  %19 = sub nsw i64 %15, %18
  %20 = add nsw i64 %12, %19
  ret i64 %20
}

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #2

; Function Attrs: noinline nounwind uwtable
define internal i32 @cmp_ll(ptr noundef %0, ptr noundef %1) #0 {
  %3 = alloca ptr, align 8
  %4 = alloca ptr, align 8
  %5 = alloca i64, align 8
  %6 = alloca i64, align 8
  store ptr %0, ptr %3, align 8
  store ptr %1, ptr %4, align 8
  %7 = load ptr, ptr %3, align 8
  %8 = load i64, ptr %7, align 8
  store i64 %8, ptr %5, align 8
  %9 = load ptr, ptr %4, align 8
  %10 = load i64, ptr %9, align 8
  store i64 %10, ptr %6, align 8
  %11 = load i64, ptr %5, align 8
  %12 = load i64, ptr %6, align 8
  %13 = icmp sgt i64 %11, %12
  %14 = zext i1 %13 to i32
  %15 = load i64, ptr %5, align 8
  %16 = load i64, ptr %6, align 8
  %17 = icmp slt i64 %15, %16
  %18 = zext i1 %17 to i32
  %19 = sub nsw i32 %14, %18
  ret i32 %19
}

declare i32 @printf(ptr noundef, ...) #2

; Function Attrs: noinline nounwind uwtable
define internal i32 @lcg_rand() #0 {
  %1 = load i32, ptr @lcg_state, align 4
  %2 = mul i32 %1, 1103515245
  %3 = add i32 %2, 12345
  store i32 %3, ptr @lcg_state, align 4
  %4 = load i32, ptr @lcg_state, align 4
  %5 = lshr i32 %4, 16
  %6 = and i32 %5, 32767
  ret i32 %6
}

; Function Attrs: noinline nounwind uwtable
define internal ptr @bst_insert(ptr noundef %0, i32 noundef %1) #0 {
  %3 = alloca ptr, align 8
  %4 = alloca ptr, align 8
  %5 = alloca i32, align 4
  %6 = alloca ptr, align 8
  store ptr %0, ptr %4, align 8
  store i32 %1, ptr %5, align 4
  %7 = load ptr, ptr %4, align 8
  %8 = icmp ne ptr %7, null
  br i1 %8, label %12, label %9

9:                                                ; preds = %2
  %10 = load i32, ptr %5, align 4
  %11 = call ptr @node_alloc(i32 noundef %10)
  store ptr %11, ptr %3, align 8
  br label %60

12:                                               ; preds = %2
  %13 = load ptr, ptr %4, align 8
  store ptr %13, ptr %6, align 8
  br label %14

14:                                               ; preds = %59, %12
  %15 = load i32, ptr %5, align 4
  %16 = load ptr, ptr %6, align 8
  %17 = getelementptr inbounds nuw %struct.Node, ptr %16, i32 0, i32 0
  %18 = load i32, ptr %17, align 8
  %19 = icmp slt i32 %15, %18
  br i1 %19, label %20, label %35

20:                                               ; preds = %14
  %21 = load ptr, ptr %6, align 8
  %22 = getelementptr inbounds nuw %struct.Node, ptr %21, i32 0, i32 1
  %23 = load ptr, ptr %22, align 8
  %24 = icmp ne ptr %23, null
  br i1 %24, label %31, label %25

25:                                               ; preds = %20
  %26 = load i32, ptr %5, align 4
  %27 = call ptr @node_alloc(i32 noundef %26)
  %28 = load ptr, ptr %6, align 8
  %29 = getelementptr inbounds nuw %struct.Node, ptr %28, i32 0, i32 1
  store ptr %27, ptr %29, align 8
  %30 = load ptr, ptr %4, align 8
  store ptr %30, ptr %3, align 8
  br label %60

31:                                               ; preds = %20
  %32 = load ptr, ptr %6, align 8
  %33 = getelementptr inbounds nuw %struct.Node, ptr %32, i32 0, i32 1
  %34 = load ptr, ptr %33, align 8
  store ptr %34, ptr %6, align 8
  br label %59

35:                                               ; preds = %14
  %36 = load i32, ptr %5, align 4
  %37 = load ptr, ptr %6, align 8
  %38 = getelementptr inbounds nuw %struct.Node, ptr %37, i32 0, i32 0
  %39 = load i32, ptr %38, align 8
  %40 = icmp sgt i32 %36, %39
  br i1 %40, label %41, label %56

41:                                               ; preds = %35
  %42 = load ptr, ptr %6, align 8
  %43 = getelementptr inbounds nuw %struct.Node, ptr %42, i32 0, i32 2
  %44 = load ptr, ptr %43, align 8
  %45 = icmp ne ptr %44, null
  br i1 %45, label %52, label %46

46:                                               ; preds = %41
  %47 = load i32, ptr %5, align 4
  %48 = call ptr @node_alloc(i32 noundef %47)
  %49 = load ptr, ptr %6, align 8
  %50 = getelementptr inbounds nuw %struct.Node, ptr %49, i32 0, i32 2
  store ptr %48, ptr %50, align 8
  %51 = load ptr, ptr %4, align 8
  store ptr %51, ptr %3, align 8
  br label %60

52:                                               ; preds = %41
  %53 = load ptr, ptr %6, align 8
  %54 = getelementptr inbounds nuw %struct.Node, ptr %53, i32 0, i32 2
  %55 = load ptr, ptr %54, align 8
  store ptr %55, ptr %6, align 8
  br label %58

56:                                               ; preds = %35
  %57 = load ptr, ptr %4, align 8
  store ptr %57, ptr %3, align 8
  br label %60

58:                                               ; preds = %52
  br label %59

59:                                               ; preds = %58, %31
  br label %14

60:                                               ; preds = %56, %46, %25, %9
  %61 = load ptr, ptr %3, align 8
  ret ptr %61
}

; Function Attrs: noinline nounwind uwtable
define internal i64 @inorder_sum(ptr noundef %0) #0 {
  %2 = alloca ptr, align 8
  %3 = alloca i64, align 8
  %4 = alloca ptr, align 8
  %5 = alloca i32, align 4
  %6 = alloca ptr, align 8
  store ptr %0, ptr %2, align 8
  store i64 0, ptr %3, align 8
  %7 = call noalias ptr @malloc(i64 noundef 1600000) #5
  store ptr %7, ptr %4, align 8
  store i32 0, ptr %5, align 4
  %8 = load ptr, ptr %2, align 8
  store ptr %8, ptr %6, align 8
  br label %9

9:                                                ; preds = %31, %1
  %10 = load ptr, ptr %6, align 8
  %11 = icmp ne ptr %10, null
  br i1 %11, label %15, label %12

12:                                               ; preds = %9
  %13 = load i32, ptr %5, align 4
  %14 = icmp sgt i32 %13, 0
  br label %15

15:                                               ; preds = %12, %9
  %16 = phi i1 [ true, %9 ], [ %14, %12 ]
  br i1 %16, label %17, label %47

17:                                               ; preds = %15
  br label %18

18:                                               ; preds = %21, %17
  %19 = load ptr, ptr %6, align 8
  %20 = icmp ne ptr %19, null
  br i1 %20, label %21, label %31

21:                                               ; preds = %18
  %22 = load ptr, ptr %6, align 8
  %23 = load ptr, ptr %4, align 8
  %24 = load i32, ptr %5, align 4
  %25 = add nsw i32 %24, 1
  store i32 %25, ptr %5, align 4
  %26 = sext i32 %24 to i64
  %27 = getelementptr inbounds ptr, ptr %23, i64 %26
  store ptr %22, ptr %27, align 8
  %28 = load ptr, ptr %6, align 8
  %29 = getelementptr inbounds nuw %struct.Node, ptr %28, i32 0, i32 1
  %30 = load ptr, ptr %29, align 8
  store ptr %30, ptr %6, align 8
  br label %18, !llvm.loop !10

31:                                               ; preds = %18
  %32 = load ptr, ptr %4, align 8
  %33 = load i32, ptr %5, align 4
  %34 = add nsw i32 %33, -1
  store i32 %34, ptr %5, align 4
  %35 = sext i32 %34 to i64
  %36 = getelementptr inbounds ptr, ptr %32, i64 %35
  %37 = load ptr, ptr %36, align 8
  store ptr %37, ptr %6, align 8
  %38 = load ptr, ptr %6, align 8
  %39 = getelementptr inbounds nuw %struct.Node, ptr %38, i32 0, i32 0
  %40 = load i32, ptr %39, align 8
  %41 = sext i32 %40 to i64
  %42 = load i64, ptr %3, align 8
  %43 = add nsw i64 %42, %41
  store i64 %43, ptr %3, align 8
  %44 = load ptr, ptr %6, align 8
  %45 = getelementptr inbounds nuw %struct.Node, ptr %44, i32 0, i32 2
  %46 = load ptr, ptr %45, align 8
  store ptr %46, ptr %6, align 8
  br label %9, !llvm.loop !11

47:                                               ; preds = %15
  %48 = load ptr, ptr %4, align 8
  call void @free(ptr noundef %48) #4
  %49 = load i64, ptr %3, align 8
  ret i64 %49
}

; Function Attrs: noinline nounwind uwtable
define internal ptr @node_alloc(i32 noundef %0) #0 {
  %2 = alloca i32, align 4
  %3 = alloca ptr, align 8
  store i32 %0, ptr %2, align 4
  %4 = load i32, ptr @pool_idx, align 4
  %5 = add nsw i32 %4, 1
  store i32 %5, ptr @pool_idx, align 4
  %6 = sext i32 %4 to i64
  %7 = getelementptr inbounds [200000 x %struct.Node], ptr @pool, i64 0, i64 %6
  store ptr %7, ptr %3, align 8
  %8 = load i32, ptr %2, align 4
  %9 = load ptr, ptr %3, align 8
  %10 = getelementptr inbounds nuw %struct.Node, ptr %9, i32 0, i32 0
  store i32 %8, ptr %10, align 8
  %11 = load ptr, ptr %3, align 8
  %12 = getelementptr inbounds nuw %struct.Node, ptr %11, i32 0, i32 1
  store ptr null, ptr %12, align 8
  %13 = load ptr, ptr %3, align 8
  %14 = getelementptr inbounds nuw %struct.Node, ptr %13, i32 0, i32 2
  store ptr null, ptr %14, align 8
  %15 = load ptr, ptr %3, align 8
  ret ptr %15
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
