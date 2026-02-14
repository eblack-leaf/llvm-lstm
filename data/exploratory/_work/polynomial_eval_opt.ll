; ModuleID = 'data/exploratory/_work/polynomial_eval.ll'
source_filename = "benchmarks/polynomial_eval.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@lcg_state = internal global i32 12345, align 4
@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca ptr, align 8
  %2 = alloca ptr, align 8
  %3 = alloca ptr, align 8
  %4 = alloca i32, align 4
  %5 = alloca double, align 8
  %6 = alloca [50 x i64], align 16
  %7 = alloca %struct.timespec, align 8
  %8 = alloca %struct.timespec, align 8
  %9 = tail call noalias dereferenceable_or_null(8008) ptr @malloc(i64 noundef 8008) #5
  store ptr %9, ptr %1, align 8
  %10 = tail call noalias dereferenceable_or_null(80000) ptr @malloc(i64 noundef 80000) #5
  store ptr %10, ptr %2, align 8
  %11 = tail call noalias dereferenceable_or_null(80000) ptr @malloc(i64 noundef 80000) #5
  store ptr %11, ptr %3, align 8
  store i32 12345, ptr @lcg_state, align 4
  %12 = load ptr, ptr %1, align 8
  br label %13

13:                                               ; preds = %23, %0
  %storemerge = phi i32 [ 0, %0 ], [ %24, %23 ]
  %14 = icmp slt i32 %storemerge, 1001
  br i1 %14, label %15, label %25

15:                                               ; preds = %13
  %16 = tail call i32 @lcg_rand()
  %17 = uitofp i32 %16 to double
  %18 = fmul double %17, 0x3F00000000000000
  %19 = fadd double %18, -5.000000e-01
  %20 = fmul double %19, 1.000000e-03
  %21 = sext i32 %storemerge to i64
  %22 = getelementptr inbounds double, ptr %12, i64 %21
  store double %20, ptr %22, align 8
  br label %23

23:                                               ; preds = %15
  %24 = add nsw i32 %storemerge, 1
  br label %13, !llvm.loop !6

25:                                               ; preds = %13
  %storemerge.lcssa = phi i32 [ %storemerge, %13 ]
  store i32 %storemerge.lcssa, ptr %4, align 4
  %26 = load ptr, ptr %2, align 8
  br label %27

27:                                               ; preds = %36, %25
  %storemerge1 = phi i32 [ 0, %25 ], [ %37, %36 ]
  %28 = icmp slt i32 %storemerge1, 10000
  br i1 %28, label %29, label %38

29:                                               ; preds = %27
  %30 = tail call i32 @lcg_rand()
  %31 = uitofp i32 %30 to double
  %32 = fmul double %31, 0x3F00000000000000
  %33 = tail call double @llvm.fmuladd.f64(double %32, double 2.000000e+00, double -1.000000e+00)
  %34 = sext i32 %storemerge1 to i64
  %35 = getelementptr inbounds double, ptr %26, i64 %34
  store double %33, ptr %35, align 8
  br label %36

36:                                               ; preds = %29
  %37 = add nsw i32 %storemerge1, 1
  br label %27, !llvm.loop !8

38:                                               ; preds = %27
  %storemerge1.lcssa = phi i32 [ %storemerge1, %27 ]
  store i32 %storemerge1.lcssa, ptr %4, align 4
  %39 = load ptr, ptr %1, align 8
  %40 = load ptr, ptr %2, align 8
  %41 = load ptr, ptr %3, align 8
  br label %42

42:                                               ; preds = %46, %38
  %storemerge2 = phi i32 [ 0, %38 ], [ %47, %46 ]
  %43 = icmp slt i32 %storemerge2, 5
  br i1 %43, label %44, label %48

44:                                               ; preds = %42
  %45 = tail call double @workload(ptr noundef %39, ptr noundef %40, ptr noundef %41)
  store volatile double %45, ptr %5, align 8
  br label %46

46:                                               ; preds = %44
  %47 = add nsw i32 %storemerge2, 1
  br label %42, !llvm.loop !9

48:                                               ; preds = %42
  %storemerge2.lcssa = phi i32 [ %storemerge2, %42 ]
  store i32 %storemerge2.lcssa, ptr %4, align 4
  %49 = load ptr, ptr %1, align 8
  %50 = load ptr, ptr %2, align 8
  %51 = load ptr, ptr %3, align 8
  br label %52

52:                                               ; preds = %61, %48
  %storemerge3 = phi i32 [ 0, %48 ], [ %62, %61 ]
  %53 = icmp slt i32 %storemerge3, 50
  br i1 %53, label %54, label %63

54:                                               ; preds = %52
  %55 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %7) #6
  %56 = call double @workload(ptr noundef %49, ptr noundef %50, ptr noundef %51)
  store volatile double %56, ptr %5, align 8
  %57 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %8) #6
  %58 = call i64 @timespec_diff_ns(ptr noundef nonnull %7, ptr noundef nonnull %8)
  %59 = sext i32 %storemerge3 to i64
  %60 = getelementptr inbounds [50 x i64], ptr %6, i64 0, i64 %59
  store i64 %58, ptr %60, align 8
  br label %61

61:                                               ; preds = %54
  %62 = add nsw i32 %storemerge3, 1
  br label %52, !llvm.loop !10

63:                                               ; preds = %52
  %storemerge3.lcssa = phi i32 [ %storemerge3, %52 ]
  store i32 %storemerge3.lcssa, ptr %4, align 4
  call void @qsort(ptr noundef nonnull %6, i64 noundef 50, i64 noundef 8, ptr noundef nonnull @cmp_ll) #6
  %64 = getelementptr inbounds nuw i8, ptr %6, i64 200
  %65 = load i64, ptr %64, align 8
  %66 = call i32 (ptr, ...) @printf(ptr noundef nonnull dereferenceable(1) @.str, i64 noundef %65) #6
  %67 = load ptr, ptr %1, align 8
  call void @free(ptr noundef %67) #6
  %68 = load ptr, ptr %2, align 8
  call void @free(ptr noundef %68) #6
  %69 = load ptr, ptr %3, align 8
  call void @free(ptr noundef %69) #6
  ret i32 0
}

; Function Attrs: nounwind allocsize(0)
declare noalias ptr @malloc(i64 noundef) #1

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
declare double @llvm.fmuladd.f64(double, double, double) #2

; Function Attrs: noinline nounwind uwtable
define internal double @workload(ptr noundef %0, ptr noundef %1, ptr noundef %2) #0 {
  %4 = alloca ptr, align 8
  %5 = alloca ptr, align 8
  %6 = alloca ptr, align 8
  %7 = alloca i32, align 4
  %8 = alloca i32, align 4
  %9 = alloca double, align 8
  %10 = alloca double, align 8
  %11 = alloca double, align 8
  store ptr %0, ptr %4, align 8
  store ptr %1, ptr %5, align 8
  store ptr %2, ptr %6, align 8
  %12 = load ptr, ptr %5, align 8
  %13 = load ptr, ptr %4, align 8
  %14 = getelementptr inbounds nuw i8, ptr %13, i64 8000
  %15 = load ptr, ptr %4, align 8
  %16 = load ptr, ptr %6, align 8
  %.promoted4 = load double, ptr %9, align 8
  %.promoted6 = load double, ptr %10, align 8
  br label %17

17:                                               ; preds = %38, %3
  %storemerge2.lcssa8 = phi i32 [ poison, %3 ], [ %storemerge2.lcssa, %38 ]
  %.lcssa7 = phi double [ %.promoted6, %3 ], [ %.lcssa, %38 ]
  %18 = phi double [ %.promoted4, %3 ], [ %23, %38 ]
  %storemerge = phi i32 [ 0, %3 ], [ %39, %38 ]
  %19 = icmp slt i32 %storemerge, 10000
  br i1 %19, label %20, label %40

20:                                               ; preds = %17
  %21 = sext i32 %storemerge to i64
  %22 = getelementptr inbounds double, ptr %12, i64 %21
  %23 = load double, ptr %22, align 8
  %24 = load double, ptr %14, align 8
  br label %25

25:                                               ; preds = %33, %20
  %26 = phi double [ %24, %20 ], [ %32, %33 ]
  %storemerge2 = phi i32 [ 999, %20 ], [ %34, %33 ]
  %27 = icmp sgt i32 %storemerge2, -1
  br i1 %27, label %28, label %35

28:                                               ; preds = %25
  %29 = sext i32 %storemerge2 to i64
  %30 = getelementptr inbounds double, ptr %15, i64 %29
  %31 = load double, ptr %30, align 8
  %32 = tail call double @llvm.fmuladd.f64(double %26, double %23, double %31)
  br label %33

33:                                               ; preds = %28
  %34 = add nsw i32 %storemerge2, -1
  br label %25, !llvm.loop !11

35:                                               ; preds = %25
  %.lcssa = phi double [ %26, %25 ]
  %storemerge2.lcssa = phi i32 [ %storemerge2, %25 ]
  %36 = sext i32 %storemerge to i64
  %37 = getelementptr inbounds double, ptr %16, i64 %36
  store double %.lcssa, ptr %37, align 8
  br label %38

38:                                               ; preds = %35
  %39 = add nsw i32 %storemerge, 1
  br label %17, !llvm.loop !12

40:                                               ; preds = %17
  %storemerge2.lcssa8.lcssa = phi i32 [ %storemerge2.lcssa8, %17 ]
  %.lcssa7.lcssa = phi double [ %.lcssa7, %17 ]
  %.lcssa5 = phi double [ %18, %17 ]
  %storemerge.lcssa = phi i32 [ %storemerge, %17 ]
  store i32 %storemerge.lcssa, ptr %7, align 4
  store double %.lcssa5, ptr %9, align 8
  store double %.lcssa7.lcssa, ptr %10, align 8
  store i32 %storemerge2.lcssa8.lcssa, ptr %8, align 4
  store double 0.000000e+00, ptr %11, align 8
  %41 = load ptr, ptr %6, align 8
  %.promoted = load double, ptr %11, align 8
  br label %42

42:                                               ; preds = %50, %40
  %43 = phi double [ %.promoted, %40 ], [ %49, %50 ]
  %storemerge1 = phi i32 [ 0, %40 ], [ %51, %50 ]
  %44 = icmp slt i32 %storemerge1, 10000
  br i1 %44, label %45, label %52

45:                                               ; preds = %42
  %46 = sext i32 %storemerge1 to i64
  %47 = getelementptr inbounds double, ptr %41, i64 %46
  %48 = load double, ptr %47, align 8
  %49 = fadd double %43, %48
  br label %50

50:                                               ; preds = %45
  %51 = add nsw i32 %storemerge1, 1
  br label %42, !llvm.loop !13

52:                                               ; preds = %42
  %.lcssa9 = phi double [ %43, %42 ]
  %storemerge1.lcssa = phi i32 [ %storemerge1, %42 ]
  store i32 %storemerge1.lcssa, ptr %7, align 4
  store double %.lcssa9, ptr %11, align 8
  %53 = load double, ptr %11, align 8
  ret double %53
}

; Function Attrs: nounwind
declare i32 @clock_gettime(i32 noundef, ptr noundef) #3

; Function Attrs: noinline nounwind uwtable
define internal i64 @timespec_diff_ns(ptr noundef %0, ptr noundef %1) #0 {
  %3 = alloca ptr, align 8
  store ptr %0, ptr %3, align 8
  %4 = load i64, ptr %1, align 8
  %5 = load i64, ptr %0, align 8
  %6 = sub nsw i64 %4, %5
  %7 = mul nsw i64 %6, 1000000000
  %8 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %9 = load i64, ptr %8, align 8
  %10 = load ptr, ptr %3, align 8
  %11 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %12 = load i64, ptr %11, align 8
  %13 = sub nsw i64 %9, %12
  %14 = add nsw i64 %7, %13
  ret i64 %14
}

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #4

; Function Attrs: noinline nounwind uwtable
define internal i32 @cmp_ll(ptr noundef %0, ptr noundef %1) #0 {
  %3 = load i64, ptr %0, align 8
  %4 = load i64, ptr %1, align 8
  %5 = tail call i32 @llvm.scmp.i32.i64(i64 %3, i64 %4)
  ret i32 %5
}

declare i32 @printf(ptr noundef, ...) #4

; Function Attrs: nounwind
declare void @free(ptr noundef) #3

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.scmp.i32.i64(i64, i64) #2

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind allocsize(0) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #3 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #5 = { nounwind allocsize(0) }
attributes #6 = { nounwind }

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
!13 = distinct !{!13, !7}
