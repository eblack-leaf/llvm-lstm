; ModuleID = 'benchmarks/matrix_multiply_tiled.c'
source_filename = "benchmarks/matrix_multiply_tiled.c"
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
  %10 = call noalias ptr @malloc(i64 noundef 131072) #6
  store ptr %10, ptr %2, align 8
  %11 = call noalias ptr @malloc(i64 noundef 131072) #6
  store ptr %11, ptr %3, align 8
  %12 = call noalias ptr @malloc(i64 noundef 131072) #6
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
  %59 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %8) #7
  %60 = load ptr, ptr %2, align 8
  %61 = load ptr, ptr %3, align 8
  %62 = load ptr, ptr %4, align 8
  %63 = call double @workload(ptr noundef %60, ptr noundef %61, ptr noundef %62)
  store volatile double %63, ptr %6, align 8
  %64 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %9) #7
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
  call void @free(ptr noundef %77) #7
  %78 = load ptr, ptr %3, align 8
  call void @free(ptr noundef %78) #7
  %79 = load ptr, ptr %4, align 8
  call void @free(ptr noundef %79) #7
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
  %10 = alloca i32, align 4
  %11 = alloca i32, align 4
  %12 = alloca i32, align 4
  %13 = alloca double, align 8
  %14 = alloca double, align 8
  store ptr %0, ptr %4, align 8
  store ptr %1, ptr %5, align 8
  store ptr %2, ptr %6, align 8
  %15 = load ptr, ptr %6, align 8
  call void @llvm.memset.p0.i64(ptr align 8 %15, i8 0, i64 131072, i1 false)
  store i32 0, ptr %10, align 4
  br label %16

16:                                               ; preds = %103, %3
  %17 = load i32, ptr %10, align 4
  %18 = icmp slt i32 %17, 128
  br i1 %18, label %19, label %106

19:                                               ; preds = %16
  store i32 0, ptr %11, align 4
  br label %20

20:                                               ; preds = %99, %19
  %21 = load i32, ptr %11, align 4
  %22 = icmp slt i32 %21, 128
  br i1 %22, label %23, label %102

23:                                               ; preds = %20
  store i32 0, ptr %12, align 4
  br label %24

24:                                               ; preds = %95, %23
  %25 = load i32, ptr %12, align 4
  %26 = icmp slt i32 %25, 128
  br i1 %26, label %27, label %98

27:                                               ; preds = %24
  %28 = load i32, ptr %10, align 4
  store i32 %28, ptr %7, align 4
  br label %29

29:                                               ; preds = %91, %27
  %30 = load i32, ptr %7, align 4
  %31 = load i32, ptr %10, align 4
  %32 = add nsw i32 %31, 16
  %33 = icmp slt i32 %30, %32
  br i1 %33, label %34, label %94

34:                                               ; preds = %29
  %35 = load i32, ptr %11, align 4
  store i32 %35, ptr %8, align 4
  br label %36

36:                                               ; preds = %87, %34
  %37 = load i32, ptr %8, align 4
  %38 = load i32, ptr %11, align 4
  %39 = add nsw i32 %38, 16
  %40 = icmp slt i32 %37, %39
  br i1 %40, label %41, label %90

41:                                               ; preds = %36
  %42 = load ptr, ptr %6, align 8
  %43 = load i32, ptr %7, align 4
  %44 = mul nsw i32 %43, 128
  %45 = load i32, ptr %8, align 4
  %46 = add nsw i32 %44, %45
  %47 = sext i32 %46 to i64
  %48 = getelementptr inbounds double, ptr %42, i64 %47
  %49 = load double, ptr %48, align 8
  store double %49, ptr %13, align 8
  %50 = load i32, ptr %12, align 4
  store i32 %50, ptr %9, align 4
  br label %51

51:                                               ; preds = %75, %41
  %52 = load i32, ptr %9, align 4
  %53 = load i32, ptr %12, align 4
  %54 = add nsw i32 %53, 16
  %55 = icmp slt i32 %52, %54
  br i1 %55, label %56, label %78

56:                                               ; preds = %51
  %57 = load ptr, ptr %4, align 8
  %58 = load i32, ptr %7, align 4
  %59 = mul nsw i32 %58, 128
  %60 = load i32, ptr %9, align 4
  %61 = add nsw i32 %59, %60
  %62 = sext i32 %61 to i64
  %63 = getelementptr inbounds double, ptr %57, i64 %62
  %64 = load double, ptr %63, align 8
  %65 = load ptr, ptr %5, align 8
  %66 = load i32, ptr %9, align 4
  %67 = mul nsw i32 %66, 128
  %68 = load i32, ptr %8, align 4
  %69 = add nsw i32 %67, %68
  %70 = sext i32 %69 to i64
  %71 = getelementptr inbounds double, ptr %65, i64 %70
  %72 = load double, ptr %71, align 8
  %73 = load double, ptr %13, align 8
  %74 = call double @llvm.fmuladd.f64(double %64, double %72, double %73)
  store double %74, ptr %13, align 8
  br label %75

75:                                               ; preds = %56
  %76 = load i32, ptr %9, align 4
  %77 = add nsw i32 %76, 1
  store i32 %77, ptr %9, align 4
  br label %51, !llvm.loop !11

78:                                               ; preds = %51
  %79 = load double, ptr %13, align 8
  %80 = load ptr, ptr %6, align 8
  %81 = load i32, ptr %7, align 4
  %82 = mul nsw i32 %81, 128
  %83 = load i32, ptr %8, align 4
  %84 = add nsw i32 %82, %83
  %85 = sext i32 %84 to i64
  %86 = getelementptr inbounds double, ptr %80, i64 %85
  store double %79, ptr %86, align 8
  br label %87

87:                                               ; preds = %78
  %88 = load i32, ptr %8, align 4
  %89 = add nsw i32 %88, 1
  store i32 %89, ptr %8, align 4
  br label %36, !llvm.loop !12

90:                                               ; preds = %36
  br label %91

91:                                               ; preds = %90
  %92 = load i32, ptr %7, align 4
  %93 = add nsw i32 %92, 1
  store i32 %93, ptr %7, align 4
  br label %29, !llvm.loop !13

94:                                               ; preds = %29
  br label %95

95:                                               ; preds = %94
  %96 = load i32, ptr %12, align 4
  %97 = add nsw i32 %96, 16
  store i32 %97, ptr %12, align 4
  br label %24, !llvm.loop !14

98:                                               ; preds = %24
  br label %99

99:                                               ; preds = %98
  %100 = load i32, ptr %11, align 4
  %101 = add nsw i32 %100, 16
  store i32 %101, ptr %11, align 4
  br label %20, !llvm.loop !15

102:                                              ; preds = %20
  br label %103

103:                                              ; preds = %102
  %104 = load i32, ptr %10, align 4
  %105 = add nsw i32 %104, 16
  store i32 %105, ptr %10, align 4
  br label %16, !llvm.loop !16

106:                                              ; preds = %16
  store double 0.000000e+00, ptr %14, align 8
  store i32 0, ptr %7, align 4
  br label %107

107:                                              ; preds = %118, %106
  %108 = load i32, ptr %7, align 4
  %109 = icmp slt i32 %108, 16384
  br i1 %109, label %110, label %121

110:                                              ; preds = %107
  %111 = load ptr, ptr %6, align 8
  %112 = load i32, ptr %7, align 4
  %113 = sext i32 %112 to i64
  %114 = getelementptr inbounds double, ptr %111, i64 %113
  %115 = load double, ptr %114, align 8
  %116 = load double, ptr %14, align 8
  %117 = fadd double %116, %115
  store double %117, ptr %14, align 8
  br label %118

118:                                              ; preds = %110
  %119 = load i32, ptr %7, align 4
  %120 = add nsw i32 %119, 1
  store i32 %120, ptr %7, align 4
  br label %107, !llvm.loop !17

121:                                              ; preds = %107
  %122 = load double, ptr %14, align 8
  ret double %122
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

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #4

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare double @llvm.fmuladd.f64(double, double, double) #5

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind allocsize(0) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #5 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #6 = { nounwind allocsize(0) }
attributes #7 = { nounwind }

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
!15 = distinct !{!15, !7}
!16 = distinct !{!16, !7}
!17 = distinct !{!17, !7}
