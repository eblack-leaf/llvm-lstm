; ModuleID = 'benchmarks/polynomial_eval.c'
source_filename = "benchmarks/polynomial_eval.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@lcg_state = internal global i32 12345, align 4
@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca i32, align 4
  %2 = alloca ptr, align 8
  %3 = alloca ptr, align 8
  %4 = alloca ptr, align 8
  %5 = alloca i32, align 4
  %6 = alloca double, align 8
  %7 = alloca [50 x i64], align 16
  %8 = alloca %struct.timespec, align 8
  %9 = alloca %struct.timespec, align 8
  store i32 0, ptr %1, align 4
  %10 = call noalias ptr @malloc(i64 noundef 8008) #5
  store ptr %10, ptr %2, align 8
  %11 = call noalias ptr @malloc(i64 noundef 80000) #5
  store ptr %11, ptr %3, align 8
  %12 = call noalias ptr @malloc(i64 noundef 80000) #5
  store ptr %12, ptr %4, align 8
  store i32 12345, ptr @lcg_state, align 4
  store i32 0, ptr %5, align 4
  br label %13

13:                                               ; preds = %26, %0
  %14 = load i32, ptr %5, align 4
  %15 = icmp sle i32 %14, 1000
  br i1 %15, label %16, label %29

16:                                               ; preds = %13
  %17 = call i32 @lcg_rand()
  %18 = uitofp i32 %17 to double
  %19 = fdiv double %18, 3.276800e+04
  %20 = fsub double %19, 5.000000e-01
  %21 = fmul double %20, 1.000000e-03
  %22 = load ptr, ptr %2, align 8
  %23 = load i32, ptr %5, align 4
  %24 = sext i32 %23 to i64
  %25 = getelementptr inbounds double, ptr %22, i64 %24
  store double %21, ptr %25, align 8
  br label %26

26:                                               ; preds = %16
  %27 = load i32, ptr %5, align 4
  %28 = add nsw i32 %27, 1
  store i32 %28, ptr %5, align 4
  br label %13, !llvm.loop !6

29:                                               ; preds = %13
  store i32 0, ptr %5, align 4
  br label %30

30:                                               ; preds = %42, %29
  %31 = load i32, ptr %5, align 4
  %32 = icmp slt i32 %31, 10000
  br i1 %32, label %33, label %45

33:                                               ; preds = %30
  %34 = call i32 @lcg_rand()
  %35 = uitofp i32 %34 to double
  %36 = fdiv double %35, 3.276800e+04
  %37 = call double @llvm.fmuladd.f64(double %36, double 2.000000e+00, double -1.000000e+00)
  %38 = load ptr, ptr %3, align 8
  %39 = load i32, ptr %5, align 4
  %40 = sext i32 %39 to i64
  %41 = getelementptr inbounds double, ptr %38, i64 %40
  store double %37, ptr %41, align 8
  br label %42

42:                                               ; preds = %33
  %43 = load i32, ptr %5, align 4
  %44 = add nsw i32 %43, 1
  store i32 %44, ptr %5, align 4
  br label %30, !llvm.loop !8

45:                                               ; preds = %30
  store i32 0, ptr %5, align 4
  br label %46

46:                                               ; preds = %54, %45
  %47 = load i32, ptr %5, align 4
  %48 = icmp slt i32 %47, 5
  br i1 %48, label %49, label %57

49:                                               ; preds = %46
  %50 = load ptr, ptr %2, align 8
  %51 = load ptr, ptr %3, align 8
  %52 = load ptr, ptr %4, align 8
  %53 = call double @workload(ptr noundef %50, ptr noundef %51, ptr noundef %52)
  store volatile double %53, ptr %6, align 8
  br label %54

54:                                               ; preds = %49
  %55 = load i32, ptr %5, align 4
  %56 = add nsw i32 %55, 1
  store i32 %56, ptr %5, align 4
  br label %46, !llvm.loop !9

57:                                               ; preds = %46
  store i32 0, ptr %5, align 4
  br label %58

58:                                               ; preds = %72, %57
  %59 = load i32, ptr %5, align 4
  %60 = icmp slt i32 %59, 50
  br i1 %60, label %61, label %75

61:                                               ; preds = %58
  %62 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %8) #6
  %63 = load ptr, ptr %2, align 8
  %64 = load ptr, ptr %3, align 8
  %65 = load ptr, ptr %4, align 8
  %66 = call double @workload(ptr noundef %63, ptr noundef %64, ptr noundef %65)
  store volatile double %66, ptr %6, align 8
  %67 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %9) #6
  %68 = call i64 @timespec_diff_ns(ptr noundef %8, ptr noundef %9)
  %69 = load i32, ptr %5, align 4
  %70 = sext i32 %69 to i64
  %71 = getelementptr inbounds [50 x i64], ptr %7, i64 0, i64 %70
  store i64 %68, ptr %71, align 8
  br label %72

72:                                               ; preds = %61
  %73 = load i32, ptr %5, align 4
  %74 = add nsw i32 %73, 1
  store i32 %74, ptr %5, align 4
  br label %58, !llvm.loop !10

75:                                               ; preds = %58
  %76 = getelementptr inbounds [50 x i64], ptr %7, i64 0, i64 0
  call void @qsort(ptr noundef %76, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %77 = getelementptr inbounds [50 x i64], ptr %7, i64 0, i64 25
  %78 = load i64, ptr %77, align 8
  %79 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %78)
  %80 = load ptr, ptr %2, align 8
  call void @free(ptr noundef %80) #6
  %81 = load ptr, ptr %3, align 8
  call void @free(ptr noundef %81) #6
  %82 = load ptr, ptr %4, align 8
  call void @free(ptr noundef %82) #6
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
  %4 = load i32, ptr @lcg_state, align 4
  %5 = lshr i32 %4, 16
  %6 = and i32 %5, 32767
  ret i32 %6
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
  store i32 0, ptr %7, align 4
  br label %12

12:                                               ; preds = %45, %3
  %13 = load i32, ptr %7, align 4
  %14 = icmp slt i32 %13, 10000
  br i1 %14, label %15, label %48

15:                                               ; preds = %12
  %16 = load ptr, ptr %5, align 8
  %17 = load i32, ptr %7, align 4
  %18 = sext i32 %17 to i64
  %19 = getelementptr inbounds double, ptr %16, i64 %18
  %20 = load double, ptr %19, align 8
  store double %20, ptr %9, align 8
  %21 = load ptr, ptr %4, align 8
  %22 = getelementptr inbounds double, ptr %21, i64 1000
  %23 = load double, ptr %22, align 8
  store double %23, ptr %10, align 8
  store i32 999, ptr %8, align 4
  br label %24

24:                                               ; preds = %36, %15
  %25 = load i32, ptr %8, align 4
  %26 = icmp sge i32 %25, 0
  br i1 %26, label %27, label %39

27:                                               ; preds = %24
  %28 = load double, ptr %10, align 8
  %29 = load double, ptr %9, align 8
  %30 = load ptr, ptr %4, align 8
  %31 = load i32, ptr %8, align 4
  %32 = sext i32 %31 to i64
  %33 = getelementptr inbounds double, ptr %30, i64 %32
  %34 = load double, ptr %33, align 8
  %35 = call double @llvm.fmuladd.f64(double %28, double %29, double %34)
  store double %35, ptr %10, align 8
  br label %36

36:                                               ; preds = %27
  %37 = load i32, ptr %8, align 4
  %38 = add nsw i32 %37, -1
  store i32 %38, ptr %8, align 4
  br label %24, !llvm.loop !11

39:                                               ; preds = %24
  %40 = load double, ptr %10, align 8
  %41 = load ptr, ptr %6, align 8
  %42 = load i32, ptr %7, align 4
  %43 = sext i32 %42 to i64
  %44 = getelementptr inbounds double, ptr %41, i64 %43
  store double %40, ptr %44, align 8
  br label %45

45:                                               ; preds = %39
  %46 = load i32, ptr %7, align 4
  %47 = add nsw i32 %46, 1
  store i32 %47, ptr %7, align 4
  br label %12, !llvm.loop !12

48:                                               ; preds = %12
  store double 0.000000e+00, ptr %11, align 8
  store i32 0, ptr %7, align 4
  br label %49

49:                                               ; preds = %60, %48
  %50 = load i32, ptr %7, align 4
  %51 = icmp slt i32 %50, 10000
  br i1 %51, label %52, label %63

52:                                               ; preds = %49
  %53 = load ptr, ptr %6, align 8
  %54 = load i32, ptr %7, align 4
  %55 = sext i32 %54 to i64
  %56 = getelementptr inbounds double, ptr %53, i64 %55
  %57 = load double, ptr %56, align 8
  %58 = load double, ptr %11, align 8
  %59 = fadd double %58, %57
  store double %59, ptr %11, align 8
  br label %60

60:                                               ; preds = %52
  %61 = load i32, ptr %7, align 4
  %62 = add nsw i32 %61, 1
  store i32 %62, ptr %7, align 4
  br label %49, !llvm.loop !13

63:                                               ; preds = %49
  %64 = load double, ptr %11, align 8
  ret double %64
}

; Function Attrs: nounwind
declare i32 @clock_gettime(i32 noundef, ptr noundef) #3

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

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #4

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

declare i32 @printf(ptr noundef, ...) #4

; Function Attrs: nounwind
declare void @free(ptr noundef) #3

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
