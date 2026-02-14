; ModuleID = 'data/exploratory/_work/heap_ops.ll'
source_filename = "benchmarks/heap_ops.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@lcg_state = internal global i32 12345, align 4
@sink = internal global i64 0, align 8
@heap_size = internal global i32 0, align 4
@heap = internal global [200001 x i32] zeroinitializer, align 16

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca [50 x i64], align 16
  %2 = alloca %struct.timespec, align 8
  %3 = alloca %struct.timespec, align 8
  br label %4

4:                                                ; preds = %7, %0
  %.01 = phi i32 [ 0, %0 ], [ %8, %7 ]
  %5 = icmp samesign ult i32 %.01, 5
  br i1 %5, label %6, label %9

6:                                                ; preds = %4
  call void @run_benchmark()
  br label %7

7:                                                ; preds = %6
  %8 = add nuw nsw i32 %.01, 1
  br label %4, !llvm.loop !6

9:                                                ; preds = %4
  br label %10

10:                                               ; preds = %18, %9
  %.0 = phi i32 [ 0, %9 ], [ %19, %18 ]
  %11 = icmp samesign ult i32 %.0, 50
  br i1 %11, label %12, label %20

12:                                               ; preds = %10
  %13 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %2) #4
  call void @run_benchmark()
  %14 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %3) #4
  %15 = call i64 @timespec_diff_ns(ptr noundef nonnull %2, ptr noundef nonnull %3)
  %16 = zext nneg i32 %.0 to i64
  %17 = getelementptr inbounds nuw [50 x i64], ptr %1, i64 0, i64 %16
  store i64 %15, ptr %17, align 8
  br label %18

18:                                               ; preds = %12
  %19 = add nuw nsw i32 %.0, 1
  br label %10, !llvm.loop !8

20:                                               ; preds = %10
  call void @qsort(ptr noundef nonnull %1, i64 noundef 50, i64 noundef 8, ptr noundef nonnull @cmp_ll) #4
  %21 = getelementptr inbounds nuw i8, ptr %1, i64 200
  %22 = load i64, ptr %21, align 8
  %23 = call i32 (ptr, ...) @printf(ptr noundef nonnull dereferenceable(1) @.str, i64 noundef %22) #4
  ret i32 0
}

; Function Attrs: noinline nounwind uwtable
define internal void @run_benchmark() #0 {
  store i32 12345, ptr @lcg_state, align 4
  call void @heap_init()
  br label %1

1:                                                ; preds = %8, %0
  %.0 = phi i32 [ 0, %0 ], [ %9, %8 ]
  %2 = icmp samesign ult i32 %.0, 200000
  br i1 %2, label %3, label %10

3:                                                ; preds = %1
  %4 = call i32 @lcg_rand()
  %5 = shl i32 %4, 15
  %6 = call i32 @lcg_rand()
  %7 = or i32 %5, %6
  call void @heap_push(i32 noundef %7)
  br label %8

8:                                                ; preds = %3
  %9 = add nuw nsw i32 %.0, 1
  br label %1, !llvm.loop !9

10:                                               ; preds = %1
  br label %11

11:                                               ; preds = %15, %10
  %.02 = phi i64 [ 0, %10 ], [ %17, %15 ]
  %.01 = phi i32 [ 0, %10 ], [ %18, %15 ]
  %12 = icmp samesign ult i32 %.01, 200000
  br i1 %12, label %13, label %19

13:                                               ; preds = %11
  %14 = call i32 @heap_pop()
  br label %15

15:                                               ; preds = %13
  %16 = sext i32 %14 to i64
  %17 = add nsw i64 %16, %.02
  %18 = add nuw nsw i32 %.01, 1
  br label %11, !llvm.loop !10

19:                                               ; preds = %11
  store volatile i64 %.02, ptr @sink, align 8
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
  %.neg = sub i64 0, %10
  %11 = add i64 %.neg, %8
  %12 = add nsw i64 %11, %6
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
define internal void @heap_init() #0 {
  store i32 0, ptr @heap_size, align 4
  ret void
}

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
define internal void @heap_push(i32 noundef %0) #0 {
  %2 = load i32, ptr @heap_size, align 4
  %3 = add nsw i32 %2, 1
  store i32 %3, ptr @heap_size, align 4
  %4 = sext i32 %3 to i64
  %5 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %4
  store i32 %0, ptr %5, align 4
  br label %6

6:                                                ; preds = %19, %1
  %.0 = phi i32 [ %3, %1 ], [ %23, %19 ]
  %7 = icmp sgt i32 %.0, 1
  br i1 %7, label %8, label %17

8:                                                ; preds = %6
  %9 = zext nneg i32 %.0 to i64
  %10 = getelementptr inbounds nuw [200001 x i32], ptr @heap, i64 0, i64 %9
  %11 = load i32, ptr %10, align 4
  %12 = lshr i32 %.0, 1
  %13 = zext nneg i32 %12 to i64
  %14 = getelementptr inbounds nuw [200001 x i32], ptr @heap, i64 0, i64 %13
  %15 = load i32, ptr %14, align 4
  %16 = icmp slt i32 %11, %15
  br label %17

17:                                               ; preds = %8, %6
  %18 = phi i1 [ false, %6 ], [ %16, %8 ]
  br i1 %18, label %19, label %27

19:                                               ; preds = %17
  %20 = sext i32 %.0 to i64
  %21 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %20
  %22 = load i32, ptr %21, align 4
  %23 = sdiv i32 %.0, 2
  %24 = sext i32 %23 to i64
  %25 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %24
  %26 = load i32, ptr %25, align 4
  store i32 %26, ptr %21, align 4
  store i32 %22, ptr %25, align 4
  br label %6, !llvm.loop !11

27:                                               ; preds = %17
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal i32 @heap_pop() #0 {
  %1 = load i32, ptr getelementptr inbounds nuw (i8, ptr @heap, i64 4), align 4
  %2 = load i32, ptr @heap_size, align 4
  %3 = sext i32 %2 to i64
  %4 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %3
  %5 = load i32, ptr %4, align 4
  store i32 %5, ptr getelementptr inbounds nuw (i8, ptr @heap, i64 4), align 4
  %6 = add nsw i32 %2, -1
  store i32 %6, ptr @heap_size, align 4
  br label %7

7:                                                ; preds = %32, %0
  %.01 = phi i32 [ 1, %0 ], [ %.1, %32 ]
  %8 = shl nsw i32 %.01, 1
  %9 = or disjoint i32 %8, 1
  %.not.not = icmp slt i32 %8, %2
  br i1 %.not.not, label %10, label %19

10:                                               ; preds = %7
  %11 = sext i32 %8 to i64
  %12 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %11
  %13 = load i32, ptr %12, align 4
  %14 = sext i32 %.01 to i64
  %15 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %14
  %16 = load i32, ptr %15, align 4
  %17 = icmp slt i32 %13, %16
  br i1 %17, label %18, label %19

18:                                               ; preds = %10
  br label %19

19:                                               ; preds = %18, %10, %7
  %.0 = phi i32 [ %8, %18 ], [ %.01, %10 ], [ %.01, %7 ]
  %.not.not2 = icmp slt i32 %8, %6
  br i1 %.not.not2, label %20, label %29

20:                                               ; preds = %19
  %21 = sext i32 %9 to i64
  %22 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %21
  %23 = load i32, ptr %22, align 4
  %24 = sext i32 %.0 to i64
  %25 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %24
  %26 = load i32, ptr %25, align 4
  %27 = icmp slt i32 %23, %26
  br i1 %27, label %28, label %29

28:                                               ; preds = %20
  br label %29

29:                                               ; preds = %28, %20, %19
  %.1 = phi i32 [ %9, %28 ], [ %.0, %20 ], [ %.0, %19 ]
  %30 = icmp eq i32 %.1, %.01
  br i1 %30, label %31, label %32

31:                                               ; preds = %29
  br label %39

32:                                               ; preds = %29
  %33 = sext i32 %.01 to i64
  %34 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %33
  %35 = load i32, ptr %34, align 4
  %36 = sext i32 %.1 to i64
  %37 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %36
  %38 = load i32, ptr %37, align 4
  store i32 %38, ptr %34, align 4
  store i32 %35, ptr %37, align 4
  br label %7

39:                                               ; preds = %31
  ret i32 %1
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.scmp.i32.i64(i64, i64) #3

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #4 = { nounwind }

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
