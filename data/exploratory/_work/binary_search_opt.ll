; ModuleID = 'data/exploratory/_work/binary_search.ll'
source_filename = "benchmarks/binary_search.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@lcg_state = internal global i32 12345, align 4
@arr = internal global [1000000 x i32] zeroinitializer, align 16
@queries = internal global [1000000 x i32] zeroinitializer, align 16
@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@found_count = internal global i32 0, align 4

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca [50 x i64], align 16
  %2 = alloca %struct.timespec, align 8
  %3 = alloca %struct.timespec, align 8
  store i32 12345, ptr @lcg_state, align 4
  br label %4

4:                                                ; preds = %7, %0
  %5 = phi i32 [ %14, %7 ], [ 0, %0 ]
  %6 = icmp samesign ult i32 %5, 1000000
  br i1 %6, label %7, label %15

7:                                                ; preds = %4
  %8 = call i32 @lcg_rand()
  %9 = shl i32 %8, 16
  %10 = call i32 @lcg_rand()
  %11 = or i32 %9, %10
  %12 = zext nneg i32 %5 to i64
  %13 = getelementptr inbounds nuw [1000000 x i32], ptr @arr, i64 0, i64 %12
  store i32 %11, ptr %13, align 4
  %14 = add nuw nsw i32 %5, 1
  br label %4, !llvm.loop !6

15:                                               ; preds = %4
  call void @qsort(ptr noundef nonnull @arr, i64 noundef 1000000, i64 noundef 4, ptr noundef nonnull @cmp_int) #4
  store i32 67890, ptr @lcg_state, align 4
  br label %16

16:                                               ; preds = %38, %15
  %17 = phi i32 [ %39, %38 ], [ 0, %15 ]
  %18 = icmp samesign ult i32 %17, 1000000
  br i1 %18, label %19, label %40

19:                                               ; preds = %16
  %20 = call i32 @lcg_rand()
  %21 = and i32 %20, 1
  %22 = icmp eq i32 %21, 0
  br i1 %22, label %23, label %31

23:                                               ; preds = %19
  %24 = call i32 @lcg_rand()
  %25 = urem i32 %24, 1000000
  %26 = zext nneg i32 %25 to i64
  %27 = getelementptr inbounds nuw [1000000 x i32], ptr @arr, i64 0, i64 %26
  %28 = load i32, ptr %27, align 4
  %29 = zext nneg i32 %17 to i64
  %30 = getelementptr inbounds nuw [1000000 x i32], ptr @queries, i64 0, i64 %29
  store i32 %28, ptr %30, align 4
  br label %38

31:                                               ; preds = %19
  %32 = call i32 @lcg_rand()
  %33 = shl i32 %32, 16
  %34 = call i32 @lcg_rand()
  %35 = or i32 %33, %34
  %36 = zext nneg i32 %17 to i64
  %37 = getelementptr inbounds nuw [1000000 x i32], ptr @queries, i64 0, i64 %36
  store i32 %35, ptr %37, align 4
  br label %38

38:                                               ; preds = %31, %23
  %39 = add nuw nsw i32 %17, 1
  br label %16, !llvm.loop !8

40:                                               ; preds = %16
  br label %41

41:                                               ; preds = %44, %40
  %42 = phi i32 [ %45, %44 ], [ 0, %40 ]
  %43 = icmp samesign ult i32 %42, 5
  br i1 %43, label %44, label %46

44:                                               ; preds = %41
  call void @do_benchmark()
  %45 = add nuw nsw i32 %42, 1
  br label %41, !llvm.loop !9

46:                                               ; preds = %41
  br label %47

47:                                               ; preds = %50, %46
  %48 = phi i32 [ %56, %50 ], [ 0, %46 ]
  %49 = icmp samesign ult i32 %48, 50
  br i1 %49, label %50, label %57

50:                                               ; preds = %47
  %51 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %2) #4
  call void @do_benchmark()
  %52 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %3) #4
  %53 = call i64 @timespec_diff_ns(ptr noundef nonnull %2, ptr noundef nonnull %3)
  %54 = zext nneg i32 %48 to i64
  %55 = getelementptr inbounds nuw [50 x i64], ptr %1, i64 0, i64 %54
  store i64 %53, ptr %55, align 8
  %56 = add nuw nsw i32 %48, 1
  br label %47, !llvm.loop !10

57:                                               ; preds = %47
  call void @qsort(ptr noundef nonnull %1, i64 noundef 50, i64 noundef 8, ptr noundef nonnull @cmp_ll) #4
  %58 = getelementptr inbounds nuw i8, ptr %1, i64 200
  %59 = load i64, ptr %58, align 8
  %60 = call i32 (ptr, ...) @printf(ptr noundef nonnull dereferenceable(1) @.str, i64 noundef %59) #4
  ret i32 0
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

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #1

; Function Attrs: noinline nounwind uwtable
define internal i32 @cmp_int(ptr noundef %0, ptr noundef %1) #0 {
  %3 = load i32, ptr %0, align 4
  %4 = load i32, ptr %1, align 4
  %5 = call i32 @llvm.scmp.i32.i32(i32 %3, i32 %4)
  ret i32 %5
}

; Function Attrs: noinline nounwind uwtable
define internal void @do_benchmark() #0 {
  br label %1

1:                                                ; preds = %13, %0
  %2 = phi i32 [ %14, %13 ], [ 0, %0 ]
  %3 = phi i32 [ %15, %13 ], [ 0, %0 ]
  %4 = icmp samesign ult i32 %3, 1000000
  br i1 %4, label %5, label %16

5:                                                ; preds = %1
  %6 = zext nneg i32 %3 to i64
  %7 = getelementptr inbounds nuw [1000000 x i32], ptr @queries, i64 0, i64 %6
  %8 = load i32, ptr %7, align 4
  %9 = call i32 @binary_search(ptr noundef nonnull @arr, i32 noundef 1000000, i32 noundef %8)
  %10 = icmp sgt i32 %9, -1
  br i1 %10, label %11, label %13

11:                                               ; preds = %5
  %12 = add nsw i32 %2, 1
  br label %13

13:                                               ; preds = %11, %5
  %14 = phi i32 [ %12, %11 ], [ %2, %5 ]
  %15 = add nuw nsw i32 %3, 1
  br label %1, !llvm.loop !11

16:                                               ; preds = %1
  store volatile i32 %2, ptr @found_count, align 4
  ret void
}

; Function Attrs: nounwind
declare i32 @clock_gettime(i32 noundef, ptr noundef) #2

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

; Function Attrs: noinline nounwind uwtable
define internal i32 @cmp_ll(ptr noundef %0, ptr noundef %1) #0 {
  %3 = load i64, ptr %0, align 8
  %4 = load i64, ptr %1, align 8
  %5 = call i32 @llvm.scmp.i32.i64(i64 %3, i64 %4)
  ret i32 %5
}

declare i32 @printf(ptr noundef, ...) #1

; Function Attrs: noinline nounwind uwtable
define internal i32 @binary_search(ptr noundef %0, i32 noundef %1, i32 noundef %2) #0 {
  %4 = add nsw i32 %1, -1
  br label %5

5:                                                ; preds = %23, %3
  %6 = phi i32 [ %24, %23 ], [ 0, %3 ]
  %7 = phi i32 [ %25, %23 ], [ %4, %3 ]
  %.not = icmp sgt i32 %6, %7
  br i1 %.not, label %26, label %8

8:                                                ; preds = %5
  %9 = sub nsw i32 %7, %6
  %10 = sdiv i32 %9, 2
  %11 = add nsw i32 %6, %10
  %12 = sext i32 %11 to i64
  %13 = getelementptr inbounds i32, ptr %0, i64 %12
  %14 = load i32, ptr %13, align 4
  %15 = icmp eq i32 %14, %2
  br i1 %15, label %16, label %17

16:                                               ; preds = %8
  br label %27

17:                                               ; preds = %8
  %18 = icmp slt i32 %14, %2
  br i1 %18, label %19, label %21

19:                                               ; preds = %17
  %20 = add nsw i32 %11, 1
  br label %23

21:                                               ; preds = %17
  %22 = add nsw i32 %11, -1
  br label %23

23:                                               ; preds = %21, %19
  %24 = phi i32 [ %6, %21 ], [ %20, %19 ]
  %25 = phi i32 [ %22, %21 ], [ %7, %19 ]
  br label %5, !llvm.loop !12

26:                                               ; preds = %5
  br label %27

27:                                               ; preds = %26, %16
  %28 = phi i32 [ -1, %26 ], [ %11, %16 ]
  ret i32 %28
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.scmp.i32.i32(i32, i32) #3

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.scmp.i32.i64(i64, i64) #3

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
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
!12 = distinct !{!12, !7}
