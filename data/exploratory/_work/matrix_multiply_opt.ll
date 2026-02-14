; ModuleID = 'data/exploratory/_work/matrix_multiply.ll'
source_filename = "benchmarks/matrix_multiply.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@lcg_state = internal global i32 12345, align 4
@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca double, align 8
  %2 = alloca [50 x i64], align 16
  %3 = alloca %struct.timespec, align 8
  %4 = alloca %struct.timespec, align 8
  %5 = call noalias ptr @malloc(i64 noundef 131072) #5
  %6 = call noalias ptr @malloc(i64 noundef 131072) #5
  %7 = call noalias ptr @malloc(i64 noundef 131072) #5
  store i32 12345, ptr @lcg_state, align 4
  br label %8

8:                                                ; preds = %16, %0
  %.0 = phi i32 [ 0, %0 ], [ %17, %16 ]
  %9 = icmp slt i32 %.0, 16384
  br i1 %9, label %10, label %18

10:                                               ; preds = %8
  %11 = call i32 @lcg_rand()
  %12 = uitofp i32 %11 to double
  %13 = fdiv double %12, 3.276800e+04
  %14 = sext i32 %.0 to i64
  %15 = getelementptr inbounds double, ptr %5, i64 %14
  store double %13, ptr %15, align 8
  br label %16

16:                                               ; preds = %10
  %17 = add nsw i32 %.0, 1
  br label %8, !llvm.loop !6

18:                                               ; preds = %8
  br label %19

19:                                               ; preds = %27, %18
  %.1 = phi i32 [ 0, %18 ], [ %28, %27 ]
  %20 = icmp slt i32 %.1, 16384
  br i1 %20, label %21, label %29

21:                                               ; preds = %19
  %22 = call i32 @lcg_rand()
  %23 = uitofp i32 %22 to double
  %24 = fdiv double %23, 3.276800e+04
  %25 = sext i32 %.1 to i64
  %26 = getelementptr inbounds double, ptr %6, i64 %25
  store double %24, ptr %26, align 8
  br label %27

27:                                               ; preds = %21
  %28 = add nsw i32 %.1, 1
  br label %19, !llvm.loop !8

29:                                               ; preds = %19
  br label %30

30:                                               ; preds = %34, %29
  %.2 = phi i32 [ 0, %29 ], [ %35, %34 ]
  %31 = icmp slt i32 %.2, 5
  br i1 %31, label %32, label %36

32:                                               ; preds = %30
  %33 = call double @workload(ptr noundef %5, ptr noundef %6, ptr noundef %7)
  store volatile double %33, ptr %1, align 8
  br label %34

34:                                               ; preds = %32
  %35 = add nsw i32 %.2, 1
  br label %30, !llvm.loop !9

36:                                               ; preds = %30
  br label %37

37:                                               ; preds = %46, %36
  %.3 = phi i32 [ 0, %36 ], [ %47, %46 ]
  %38 = icmp slt i32 %.3, 50
  br i1 %38, label %39, label %48

39:                                               ; preds = %37
  %40 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %3) #6
  %41 = call double @workload(ptr noundef %5, ptr noundef %6, ptr noundef %7)
  store volatile double %41, ptr %1, align 8
  %42 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %4) #6
  %43 = call i64 @timespec_diff_ns(ptr noundef %3, ptr noundef %4)
  %44 = sext i32 %.3 to i64
  %45 = getelementptr inbounds [50 x i64], ptr %2, i64 0, i64 %44
  store i64 %43, ptr %45, align 8
  br label %46

46:                                               ; preds = %39
  %47 = add nsw i32 %.3, 1
  br label %37, !llvm.loop !10

48:                                               ; preds = %37
  call void @qsort(ptr noundef %2, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %49 = getelementptr inbounds [50 x i64], ptr %2, i64 0, i64 25
  %50 = load i64, ptr %49, align 8
  %51 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %50)
  call void @free(ptr noundef %5) #6
  call void @free(ptr noundef %6) #6
  call void @free(ptr noundef %7) #6
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

; Function Attrs: noinline nounwind uwtable
define internal double @workload(ptr noundef %0, ptr noundef %1, ptr noundef %2) #0 {
  br label %4

4:                                                ; preds = %34, %3
  %.03 = phi i32 [ 0, %3 ], [ %35, %34 ]
  %5 = icmp slt i32 %.03, 128
  br i1 %5, label %6, label %36

6:                                                ; preds = %4
  br label %7

7:                                                ; preds = %31, %6
  %.04 = phi i32 [ 0, %6 ], [ %32, %31 ]
  %8 = icmp slt i32 %.04, 128
  br i1 %8, label %9, label %33

9:                                                ; preds = %7
  br label %10

10:                                               ; preds = %24, %9
  %.02 = phi i32 [ 0, %9 ], [ %25, %24 ]
  %.01 = phi double [ 0.000000e+00, %9 ], [ %23, %24 ]
  %11 = icmp slt i32 %.02, 128
  br i1 %11, label %12, label %26

12:                                               ; preds = %10
  %13 = mul nsw i32 %.03, 128
  %14 = add nsw i32 %13, %.02
  %15 = sext i32 %14 to i64
  %16 = getelementptr inbounds double, ptr %0, i64 %15
  %17 = load double, ptr %16, align 8
  %18 = mul nsw i32 %.02, 128
  %19 = add nsw i32 %18, %.04
  %20 = sext i32 %19 to i64
  %21 = getelementptr inbounds double, ptr %1, i64 %20
  %22 = load double, ptr %21, align 8
  %23 = call double @llvm.fmuladd.f64(double %17, double %22, double %.01)
  br label %24

24:                                               ; preds = %12
  %25 = add nsw i32 %.02, 1
  br label %10, !llvm.loop !11

26:                                               ; preds = %10
  %27 = mul nsw i32 %.03, 128
  %28 = add nsw i32 %27, %.04
  %29 = sext i32 %28 to i64
  %30 = getelementptr inbounds double, ptr %2, i64 %29
  store double %.01, ptr %30, align 8
  br label %31

31:                                               ; preds = %26
  %32 = add nsw i32 %.04, 1
  br label %7, !llvm.loop !12

33:                                               ; preds = %7
  br label %34

34:                                               ; preds = %33
  %35 = add nsw i32 %.03, 1
  br label %4, !llvm.loop !13

36:                                               ; preds = %4
  br label %37

37:                                               ; preds = %44, %36
  %.1 = phi i32 [ 0, %36 ], [ %45, %44 ]
  %.0 = phi double [ 0.000000e+00, %36 ], [ %43, %44 ]
  %38 = icmp slt i32 %.1, 16384
  br i1 %38, label %39, label %46

39:                                               ; preds = %37
  %40 = sext i32 %.1 to i64
  %41 = getelementptr inbounds double, ptr %2, i64 %40
  %42 = load double, ptr %41, align 8
  %43 = fadd double %.0, %42
  br label %44

44:                                               ; preds = %39
  %45 = add nsw i32 %.1, 1
  br label %37, !llvm.loop !14

46:                                               ; preds = %37
  ret double %.0
}

; Function Attrs: nounwind
declare i32 @clock_gettime(i32 noundef, ptr noundef) #2

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

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #3

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

declare i32 @printf(ptr noundef, ...) #3

; Function Attrs: nounwind
declare void @free(ptr noundef) #2

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare double @llvm.fmuladd.f64(double, double, double) #4

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind allocsize(0) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
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
!14 = distinct !{!14, !7}
