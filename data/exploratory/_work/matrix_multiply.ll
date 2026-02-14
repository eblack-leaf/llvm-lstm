; ModuleID = 'benchmarks/matrix_multiply.c'
source_filename = "benchmarks/matrix_multiply.c"
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
  %10 = call noalias ptr @malloc(i64 noundef 131072) #5
  store ptr %10, ptr %2, align 8
  %11 = call noalias ptr @malloc(i64 noundef 131072) #5
  store ptr %11, ptr %3, align 8
  %12 = call noalias ptr @malloc(i64 noundef 131072) #5
  store ptr %12, ptr %4, align 8
  store i32 12345, ptr @lcg_state, align 4
  store i32 0, ptr %5, align 4
  br label %13

13:                                               ; preds = %24, %0
  %14 = load i32, ptr %5, align 4
  %15 = icmp slt i32 %14, 16384
  br i1 %15, label %16, label %27

16:                                               ; preds = %13
  %17 = call i32 @lcg_rand()
  %18 = uitofp i32 %17 to double
  %19 = fdiv double %18, 3.276800e+04
  %20 = load ptr, ptr %2, align 8
  %21 = load i32, ptr %5, align 4
  %22 = sext i32 %21 to i64
  %23 = getelementptr inbounds double, ptr %20, i64 %22
  store double %19, ptr %23, align 8
  br label %24

24:                                               ; preds = %16
  %25 = load i32, ptr %5, align 4
  %26 = add nsw i32 %25, 1
  store i32 %26, ptr %5, align 4
  br label %13, !llvm.loop !6

27:                                               ; preds = %13
  store i32 0, ptr %5, align 4
  br label %28

28:                                               ; preds = %39, %27
  %29 = load i32, ptr %5, align 4
  %30 = icmp slt i32 %29, 16384
  br i1 %30, label %31, label %42

31:                                               ; preds = %28
  %32 = call i32 @lcg_rand()
  %33 = uitofp i32 %32 to double
  %34 = fdiv double %33, 3.276800e+04
  %35 = load ptr, ptr %3, align 8
  %36 = load i32, ptr %5, align 4
  %37 = sext i32 %36 to i64
  %38 = getelementptr inbounds double, ptr %35, i64 %37
  store double %34, ptr %38, align 8
  br label %39

39:                                               ; preds = %31
  %40 = load i32, ptr %5, align 4
  %41 = add nsw i32 %40, 1
  store i32 %41, ptr %5, align 4
  br label %28, !llvm.loop !8

42:                                               ; preds = %28
  store i32 0, ptr %5, align 4
  br label %43

43:                                               ; preds = %51, %42
  %44 = load i32, ptr %5, align 4
  %45 = icmp slt i32 %44, 5
  br i1 %45, label %46, label %54

46:                                               ; preds = %43
  %47 = load ptr, ptr %2, align 8
  %48 = load ptr, ptr %3, align 8
  %49 = load ptr, ptr %4, align 8
  %50 = call double @workload(ptr noundef %47, ptr noundef %48, ptr noundef %49)
  store volatile double %50, ptr %6, align 8
  br label %51

51:                                               ; preds = %46
  %52 = load i32, ptr %5, align 4
  %53 = add nsw i32 %52, 1
  store i32 %53, ptr %5, align 4
  br label %43, !llvm.loop !9

54:                                               ; preds = %43
  store i32 0, ptr %5, align 4
  br label %55

55:                                               ; preds = %69, %54
  %56 = load i32, ptr %5, align 4
  %57 = icmp slt i32 %56, 50
  br i1 %57, label %58, label %72

58:                                               ; preds = %55
  %59 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %8) #6
  %60 = load ptr, ptr %2, align 8
  %61 = load ptr, ptr %3, align 8
  %62 = load ptr, ptr %4, align 8
  %63 = call double @workload(ptr noundef %60, ptr noundef %61, ptr noundef %62)
  store volatile double %63, ptr %6, align 8
  %64 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %9) #6
  %65 = call i64 @timespec_diff_ns(ptr noundef %8, ptr noundef %9)
  %66 = load i32, ptr %5, align 4
  %67 = sext i32 %66 to i64
  %68 = getelementptr inbounds [50 x i64], ptr %7, i64 0, i64 %67
  store i64 %65, ptr %68, align 8
  br label %69

69:                                               ; preds = %58
  %70 = load i32, ptr %5, align 4
  %71 = add nsw i32 %70, 1
  store i32 %71, ptr %5, align 4
  br label %55, !llvm.loop !10

72:                                               ; preds = %55
  %73 = getelementptr inbounds [50 x i64], ptr %7, i64 0, i64 0
  call void @qsort(ptr noundef %73, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %74 = getelementptr inbounds [50 x i64], ptr %7, i64 0, i64 25
  %75 = load i64, ptr %74, align 8
  %76 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %75)
  %77 = load ptr, ptr %2, align 8
  call void @free(ptr noundef %77) #6
  %78 = load ptr, ptr %3, align 8
  call void @free(ptr noundef %78) #6
  %79 = load ptr, ptr %4, align 8
  call void @free(ptr noundef %79) #6
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

; Function Attrs: noinline nounwind uwtable
define internal double @workload(ptr noundef %0, ptr noundef %1, ptr noundef %2) #0 {
  %4 = alloca ptr, align 8
  %5 = alloca ptr, align 8
  %6 = alloca ptr, align 8
  %7 = alloca i32, align 4
  %8 = alloca i32, align 4
  %9 = alloca i32, align 4
  %10 = alloca double, align 8
  %11 = alloca double, align 8
  store ptr %0, ptr %4, align 8
  store ptr %1, ptr %5, align 8
  store ptr %2, ptr %6, align 8
  store i32 0, ptr %7, align 4
  br label %12

12:                                               ; preds = %58, %3
  %13 = load i32, ptr %7, align 4
  %14 = icmp slt i32 %13, 128
  br i1 %14, label %15, label %61

15:                                               ; preds = %12
  store i32 0, ptr %8, align 4
  br label %16

16:                                               ; preds = %54, %15
  %17 = load i32, ptr %8, align 4
  %18 = icmp slt i32 %17, 128
  br i1 %18, label %19, label %57

19:                                               ; preds = %16
  store double 0.000000e+00, ptr %10, align 8
  store i32 0, ptr %9, align 4
  br label %20

20:                                               ; preds = %42, %19
  %21 = load i32, ptr %9, align 4
  %22 = icmp slt i32 %21, 128
  br i1 %22, label %23, label %45

23:                                               ; preds = %20
  %24 = load ptr, ptr %4, align 8
  %25 = load i32, ptr %7, align 4
  %26 = mul nsw i32 %25, 128
  %27 = load i32, ptr %9, align 4
  %28 = add nsw i32 %26, %27
  %29 = sext i32 %28 to i64
  %30 = getelementptr inbounds double, ptr %24, i64 %29
  %31 = load double, ptr %30, align 8
  %32 = load ptr, ptr %5, align 8
  %33 = load i32, ptr %9, align 4
  %34 = mul nsw i32 %33, 128
  %35 = load i32, ptr %8, align 4
  %36 = add nsw i32 %34, %35
  %37 = sext i32 %36 to i64
  %38 = getelementptr inbounds double, ptr %32, i64 %37
  %39 = load double, ptr %38, align 8
  %40 = load double, ptr %10, align 8
  %41 = call double @llvm.fmuladd.f64(double %31, double %39, double %40)
  store double %41, ptr %10, align 8
  br label %42

42:                                               ; preds = %23
  %43 = load i32, ptr %9, align 4
  %44 = add nsw i32 %43, 1
  store i32 %44, ptr %9, align 4
  br label %20, !llvm.loop !11

45:                                               ; preds = %20
  %46 = load double, ptr %10, align 8
  %47 = load ptr, ptr %6, align 8
  %48 = load i32, ptr %7, align 4
  %49 = mul nsw i32 %48, 128
  %50 = load i32, ptr %8, align 4
  %51 = add nsw i32 %49, %50
  %52 = sext i32 %51 to i64
  %53 = getelementptr inbounds double, ptr %47, i64 %52
  store double %46, ptr %53, align 8
  br label %54

54:                                               ; preds = %45
  %55 = load i32, ptr %8, align 4
  %56 = add nsw i32 %55, 1
  store i32 %56, ptr %8, align 4
  br label %16, !llvm.loop !12

57:                                               ; preds = %16
  br label %58

58:                                               ; preds = %57
  %59 = load i32, ptr %7, align 4
  %60 = add nsw i32 %59, 1
  store i32 %60, ptr %7, align 4
  br label %12, !llvm.loop !13

61:                                               ; preds = %12
  store double 0.000000e+00, ptr %11, align 8
  store i32 0, ptr %7, align 4
  br label %62

62:                                               ; preds = %73, %61
  %63 = load i32, ptr %7, align 4
  %64 = icmp slt i32 %63, 16384
  br i1 %64, label %65, label %76

65:                                               ; preds = %62
  %66 = load ptr, ptr %6, align 8
  %67 = load i32, ptr %7, align 4
  %68 = sext i32 %67 to i64
  %69 = getelementptr inbounds double, ptr %66, i64 %68
  %70 = load double, ptr %69, align 8
  %71 = load double, ptr %11, align 8
  %72 = fadd double %71, %70
  store double %72, ptr %11, align 8
  br label %73

73:                                               ; preds = %65
  %74 = load i32, ptr %7, align 4
  %75 = add nsw i32 %74, 1
  store i32 %75, ptr %7, align 4
  br label %62, !llvm.loop !14

76:                                               ; preds = %62
  %77 = load double, ptr %11, align 8
  ret double %77
}

; Function Attrs: nounwind
declare i32 @clock_gettime(i32 noundef, ptr noundef) #2

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

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #3

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
