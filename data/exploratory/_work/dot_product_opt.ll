; ModuleID = 'data/exploratory/_work/dot_product.ll'
source_filename = "benchmarks/dot_product.c"
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
  %5 = tail call noalias dereferenceable_or_null(8000000) ptr @malloc(i64 noundef 8000000) #5
  %6 = tail call noalias dereferenceable_or_null(8000000) ptr @malloc(i64 noundef 8000000) #5
  store i32 12345, ptr @lcg_state, align 4
  br label %7

7:                                                ; preds = %9, %0
  %.0 = phi i32 [ 0, %0 ], [ %15, %9 ]
  %8 = icmp samesign ult i32 %.0, 1000000
  br i1 %8, label %9, label %16

9:                                                ; preds = %7
  %10 = tail call i32 @lcg_rand()
  %11 = uitofp i32 %10 to double
  %12 = fmul double %11, 0x3F00000000000000
  %13 = zext nneg i32 %.0 to i64
  %14 = getelementptr inbounds nuw double, ptr %5, i64 %13
  store double %12, ptr %14, align 8
  %15 = add nuw nsw i32 %.0, 1
  br label %7, !llvm.loop !6

16:                                               ; preds = %7, %18
  %.1 = phi i32 [ %24, %18 ], [ 0, %7 ]
  %17 = icmp samesign ult i32 %.1, 1000000
  br i1 %17, label %18, label %25

18:                                               ; preds = %16
  %19 = tail call i32 @lcg_rand()
  %20 = uitofp i32 %19 to double
  %21 = fmul double %20, 0x3F00000000000000
  %22 = zext nneg i32 %.1 to i64
  %23 = getelementptr inbounds nuw double, ptr %6, i64 %22
  store double %21, ptr %23, align 8
  %24 = add nuw nsw i32 %.1, 1
  br label %16, !llvm.loop !8

25:                                               ; preds = %16, %27
  %.2 = phi i32 [ %29, %27 ], [ 0, %16 ]
  %26 = icmp samesign ult i32 %.2, 5
  br i1 %26, label %27, label %30

27:                                               ; preds = %25
  %28 = tail call double @workload(ptr noundef %5, ptr noundef %6)
  store volatile double %28, ptr %1, align 8
  %29 = add nuw nsw i32 %.2, 1
  br label %25, !llvm.loop !9

30:                                               ; preds = %25, %32
  %.3 = phi i32 [ %39, %32 ], [ 0, %25 ]
  %31 = icmp samesign ult i32 %.3, 50
  br i1 %31, label %32, label %40

32:                                               ; preds = %30
  %33 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %3) #6
  %34 = call double @workload(ptr noundef %5, ptr noundef %6)
  store volatile double %34, ptr %1, align 8
  %35 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %4) #6
  %36 = call i64 @timespec_diff_ns(ptr noundef nonnull %3, ptr noundef nonnull %4)
  %37 = zext nneg i32 %.3 to i64
  %38 = getelementptr inbounds nuw [50 x i64], ptr %2, i64 0, i64 %37
  store i64 %36, ptr %38, align 8
  %39 = add nuw nsw i32 %.3, 1
  br label %30, !llvm.loop !10

40:                                               ; preds = %30
  call void @qsort(ptr noundef nonnull %2, i64 noundef 50, i64 noundef 8, ptr noundef nonnull @cmp_ll) #6
  %41 = getelementptr inbounds nuw i8, ptr %2, i64 200
  %42 = load i64, ptr %41, align 8
  %43 = call i32 (ptr, ...) @printf(ptr noundef nonnull dereferenceable(1) @.str, i64 noundef %42) #6
  call void @free(ptr noundef %5) #6
  call void @free(ptr noundef %6) #6
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
define internal double @workload(ptr noundef %0, ptr noundef %1) #0 {
  br label %3

3:                                                ; preds = %5, %2
  %.07 = phi double [ 0.000000e+00, %2 ], [ %12, %5 ]
  %.0 = phi i32 [ 0, %2 ], [ %13, %5 ]
  %4 = icmp samesign ult i32 %.0, 1000000
  br i1 %4, label %5, label %14

5:                                                ; preds = %3
  %6 = zext nneg i32 %.0 to i64
  %7 = getelementptr inbounds nuw double, ptr %0, i64 %6
  %8 = load double, ptr %7, align 8
  %9 = zext nneg i32 %.0 to i64
  %10 = getelementptr inbounds nuw double, ptr %1, i64 %9
  %11 = load double, ptr %10, align 8
  %12 = tail call double @llvm.fmuladd.f64(double %8, double %11, double %.07)
  %13 = add nuw nsw i32 %.0, 1
  br label %3, !llvm.loop !11

14:                                               ; preds = %3
  ret double %.07
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

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #3

; Function Attrs: noinline nounwind uwtable
define internal i32 @cmp_ll(ptr noundef %0, ptr noundef %1) #0 {
  %3 = load i64, ptr %0, align 8
  %4 = load i64, ptr %1, align 8
  %5 = call i32 @llvm.scmp.i32.i64(i64 %3, i64 %4)
  ret i32 %5
}

declare i32 @printf(ptr noundef, ...) #3

; Function Attrs: nounwind
declare void @free(ptr noundef) #2

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare double @llvm.fmuladd.f64(double, double, double) #4

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.scmp.i32.i64(i64, i64) #4

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
