; ModuleID = 'data/exploratory/_work/levenshtein.ll'
source_filename = "benchmarks/levenshtein.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@lcg_state = internal global i32 12345, align 4
@sink = internal global i32 0, align 4

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca [50 x i64], align 16
  %2 = alloca %struct.timespec, align 8
  %3 = alloca %struct.timespec, align 8
  br label %4

4:                                                ; preds = %6, %0
  %storemerge = phi i32 [ 0, %0 ], [ %7, %6 ]
  %5 = icmp slt i32 %storemerge, 5
  br i1 %5, label %6, label %8

6:                                                ; preds = %4
  call void @run_benchmark()
  %7 = add nsw i32 %storemerge, 1
  br label %4, !llvm.loop !6

8:                                                ; preds = %4
  br label %9

9:                                                ; preds = %11, %8
  %storemerge1 = phi i32 [ 0, %8 ], [ %17, %11 ]
  %10 = icmp slt i32 %storemerge1, 50
  br i1 %10, label %11, label %18

11:                                               ; preds = %9
  %12 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %2) #5
  call void @run_benchmark()
  %13 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %3) #5
  %14 = call i64 @timespec_diff_ns(ptr noundef nonnull %2, ptr noundef nonnull %3)
  %15 = sext i32 %storemerge1 to i64
  %16 = getelementptr inbounds [50 x i64], ptr %1, i64 0, i64 %15
  store i64 %14, ptr %16, align 8
  %17 = add nsw i32 %storemerge1, 1
  br label %9, !llvm.loop !8

18:                                               ; preds = %9
  call void @qsort(ptr noundef nonnull %1, i64 noundef 50, i64 noundef 8, ptr noundef nonnull @cmp_ll) #5
  %19 = getelementptr inbounds nuw i8, ptr %1, i64 200
  %20 = load i64, ptr %19, align 8
  %21 = call i32 (ptr, ...) @printf(ptr noundef nonnull dereferenceable(1) @.str, i64 noundef %20) #5
  ret i32 0
}

; Function Attrs: noinline nounwind uwtable
define internal void @run_benchmark() #0 {
  store i32 12345, ptr @lcg_state, align 4
  %1 = call noalias dereferenceable_or_null(2001) ptr @malloc(i64 noundef 2001) #6
  %2 = call noalias dereferenceable_or_null(2001) ptr @malloc(i64 noundef 2001) #6
  call void @generate_random_string(ptr noundef %1, i32 noundef 2000)
  call void @generate_random_string(ptr noundef %2, i32 noundef 2000)
  %3 = call i32 @levenshtein(ptr noundef %1, i32 noundef 2000, ptr noundef %2, i32 noundef 2000)
  store volatile i32 %3, ptr @sink, align 4
  call void @free(ptr noundef %1) #5
  call void @free(ptr noundef %2) #5
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

; Function Attrs: nounwind allocsize(0)
declare noalias ptr @malloc(i64 noundef) #3

; Function Attrs: noinline nounwind uwtable
define internal void @generate_random_string(ptr noundef %0, i32 noundef %1) #0 {
  br label %3

3:                                                ; preds = %5, %2
  %storemerge = phi i32 [ 0, %2 ], [ %12, %5 ]
  %4 = icmp slt i32 %storemerge, %1
  br i1 %4, label %5, label %13

5:                                                ; preds = %3
  %6 = call i32 @lcg_rand()
  %7 = urem i32 %6, 26
  %8 = trunc nuw nsw i32 %7 to i8
  %9 = add nuw i8 %8, 97
  %10 = sext i32 %storemerge to i64
  %11 = getelementptr inbounds i8, ptr %0, i64 %10
  store i8 %9, ptr %11, align 1
  %12 = add nsw i32 %storemerge, 1
  br label %3, !llvm.loop !9

13:                                               ; preds = %3
  %14 = sext i32 %1 to i64
  %15 = getelementptr inbounds i8, ptr %0, i64 %14
  store i8 0, ptr %15, align 1
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal i32 @levenshtein(ptr noundef %0, i32 noundef %1, ptr noundef %2, i32 noundef %3) #0 {
  %5 = add nsw i32 %3, 1
  %6 = sext i32 %5 to i64
  %7 = shl nsw i64 %6, 2
  %8 = call noalias ptr @malloc(i64 noundef %7) #6
  %9 = call noalias ptr @malloc(i64 noundef %7) #6
  br label %10

10:                                               ; preds = %11, %4
  %storemerge = phi i32 [ 0, %4 ], [ %14, %11 ]
  %.not = icmp sgt i32 %storemerge, %3
  br i1 %.not, label %15, label %11

11:                                               ; preds = %10
  %12 = sext i32 %storemerge to i64
  %13 = getelementptr inbounds i32, ptr %8, i64 %12
  store i32 %storemerge, ptr %13, align 4
  %14 = add nsw i32 %storemerge, 1
  br label %10, !llvm.loop !10

15:                                               ; preds = %10
  br label %16

16:                                               ; preds = %50, %15
  %.06 = phi ptr [ %8, %15 ], [ %.0, %50 ]
  %.0 = phi ptr [ %9, %15 ], [ %.06, %50 ]
  %storemerge1 = phi i32 [ 1, %15 ], [ %51, %50 ]
  %.not2 = icmp sgt i32 %storemerge1, %1
  br i1 %.not2, label %52, label %17

17:                                               ; preds = %16
  store i32 %storemerge1, ptr %.0, align 4
  br label %18

18:                                               ; preds = %47, %17
  %storemerge3 = phi i32 [ 1, %17 ], [ %49, %47 ]
  %.not4 = icmp sgt i32 %storemerge3, %3
  br i1 %.not4, label %50, label %19

19:                                               ; preds = %18
  %20 = sext i32 %storemerge1 to i64
  %21 = getelementptr i8, ptr %0, i64 %20
  %22 = getelementptr i8, ptr %21, i64 -1
  %23 = load i8, ptr %22, align 1
  %24 = sext i32 %storemerge3 to i64
  %25 = getelementptr i8, ptr %2, i64 %24
  %26 = getelementptr i8, ptr %25, i64 -1
  %27 = load i8, ptr %26, align 1
  %.not5 = icmp ne i8 %23, %27
  %28 = zext i1 %.not5 to i32
  %29 = getelementptr i32, ptr %.06, i64 %24
  %30 = load i32, ptr %29, align 4
  %31 = add nsw i32 %30, 1
  %32 = getelementptr i32, ptr %.0, i64 %24
  %33 = getelementptr i8, ptr %32, i64 -4
  %34 = load i32, ptr %33, align 4
  %35 = add nsw i32 %34, 1
  %36 = getelementptr i8, ptr %29, i64 -4
  %37 = load i32, ptr %36, align 4
  %38 = add nsw i32 %37, %28
  %39 = icmp slt i32 %31, %35
  br i1 %39, label %40, label %41

40:                                               ; preds = %19
  br label %42

41:                                               ; preds = %19
  br label %42

42:                                               ; preds = %41, %40
  %43 = phi i32 [ %31, %40 ], [ %35, %41 ]
  %44 = icmp slt i32 %43, %38
  br i1 %44, label %45, label %46

45:                                               ; preds = %42
  br label %47

46:                                               ; preds = %42
  br label %47

47:                                               ; preds = %46, %45
  %48 = phi i32 [ %43, %45 ], [ %38, %46 ]
  store i32 %48, ptr %32, align 4
  %49 = add nsw i32 %storemerge3, 1
  br label %18, !llvm.loop !11

50:                                               ; preds = %18
  %51 = add nsw i32 %storemerge1, 1
  br label %16, !llvm.loop !12

52:                                               ; preds = %16
  %53 = sext i32 %3 to i64
  %54 = getelementptr inbounds i32, ptr %.06, i64 %53
  %55 = load i32, ptr %54, align 4
  call void @free(ptr noundef %.06) #5
  call void @free(ptr noundef %.0) #5
  ret i32 %55
}

; Function Attrs: nounwind
declare void @free(ptr noundef) #1

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

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.scmp.i32.i64(i64, i64) #4

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nounwind allocsize(0) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #5 = { nounwind }
attributes #6 = { nounwind allocsize(0) }

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
