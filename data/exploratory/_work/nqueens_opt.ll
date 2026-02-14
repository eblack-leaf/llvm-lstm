; ModuleID = 'data/exploratory/_work/nqueens.ll'
source_filename = "benchmarks/nqueens.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@lcg_state = internal global i32 12345, align 4
@solution_count = internal global i32 0, align 4

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

7:                                                ; preds = %10, %0
  %8 = load i32, ptr %2, align 4
  %9 = icmp slt i32 %8, 5
  br i1 %9, label %10, label %13

10:                                               ; preds = %7
  call void @do_nqueens()
  %11 = load i32, ptr %2, align 4
  %12 = add nsw i32 %11, 1
  store i32 %12, ptr %2, align 4
  br label %7, !llvm.loop !6

13:                                               ; preds = %7
  store i32 0, ptr %4, align 4
  br label %14

14:                                               ; preds = %17, %13
  %15 = load i32, ptr %4, align 4
  %16 = icmp slt i32 %15, 50
  br i1 %16, label %17, label %26

17:                                               ; preds = %14
  %18 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %5) #3
  call void @do_nqueens()
  %19 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %6) #3
  %20 = call i64 @timespec_diff_ns(ptr noundef %5, ptr noundef %6)
  %21 = load i32, ptr %4, align 4
  %22 = sext i32 %21 to i64
  %23 = getelementptr inbounds [50 x i64], ptr %3, i64 0, i64 %22
  store i64 %20, ptr %23, align 8
  %24 = load i32, ptr %4, align 4
  %25 = add nsw i32 %24, 1
  store i32 %25, ptr %4, align 4
  br label %14, !llvm.loop !8

26:                                               ; preds = %14
  call void @qsort(ptr noundef %3, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %27 = getelementptr inbounds [50 x i64], ptr %3, i64 0, i64 25
  %28 = load i64, ptr %27, align 8
  %29 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %28)
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

; Function Attrs: noinline nounwind uwtable
define internal void @do_nqueens() #0 {
  store i32 0, ptr @solution_count, align 4
  call void @solve(i32 noundef 0, i32 noundef 0, i32 noundef 0, i32 noundef 0)
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
  %5 = load i64, ptr %1, align 8
  %6 = load ptr, ptr %3, align 8
  %7 = load i64, ptr %6, align 8
  %8 = sub nsw i64 %5, %7
  %9 = mul nsw i64 %8, 1000000000
  %10 = getelementptr inbounds nuw %struct.timespec, ptr %1, i32 0, i32 1
  %11 = load i64, ptr %10, align 8
  %12 = getelementptr inbounds nuw %struct.timespec, ptr %6, i32 0, i32 1
  %13 = load i64, ptr %12, align 8
  %14 = sub nsw i64 %11, %13
  %15 = add nsw i64 %9, %14
  ret i64 %15
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
  %12 = icmp sgt i64 %11, %10
  %13 = zext i1 %12 to i32
  %14 = icmp slt i64 %11, %10
  %15 = zext i1 %14 to i32
  %16 = sub nsw i32 %13, %15
  ret i32 %16
}

declare i32 @printf(ptr noundef, ...) #2

; Function Attrs: noinline nounwind uwtable
define internal void @solve(i32 noundef %0, i32 noundef %1, i32 noundef %2, i32 noundef %3) #0 {
  %5 = alloca i32, align 4
  %6 = alloca i32, align 4
  %7 = alloca i32, align 4
  %8 = alloca i32, align 4
  %9 = alloca i32, align 4
  %10 = alloca i32, align 4
  store i32 %0, ptr %5, align 4
  store i32 %1, ptr %6, align 4
  store i32 %2, ptr %7, align 4
  store i32 %3, ptr %8, align 4
  %11 = load i32, ptr %5, align 4
  %12 = icmp eq i32 %11, 13
  br i1 %12, label %13, label %16

13:                                               ; preds = %4
  %14 = load i32, ptr @solution_count, align 4
  %15 = add nsw i32 %14, 1
  store i32 %15, ptr @solution_count, align 4
  br label %42

16:                                               ; preds = %4
  %17 = load i32, ptr %6, align 4
  %18 = load i32, ptr %7, align 4
  %19 = or i32 %17, %18
  %20 = or i32 %19, %3
  %21 = xor i32 %20, -1
  %22 = and i32 8191, %21
  store i32 %22, ptr %9, align 4
  br label %23

23:                                               ; preds = %26, %16
  %24 = load i32, ptr %9, align 4
  %25 = icmp ne i32 %24, 0
  br i1 %25, label %26, label %42

26:                                               ; preds = %23
  %27 = sub nsw i32 0, %24
  %28 = and i32 %24, %27
  store i32 %28, ptr %10, align 4
  %29 = load i32, ptr %9, align 4
  %30 = sub nsw i32 %29, %28
  store i32 %30, ptr %9, align 4
  %31 = load i32, ptr %5, align 4
  %32 = add nsw i32 %31, 1
  %33 = load i32, ptr %6, align 4
  %34 = load i32, ptr %10, align 4
  %35 = or i32 %33, %34
  %36 = load i32, ptr %7, align 4
  %37 = or i32 %36, %34
  %38 = shl i32 %37, 1
  %39 = load i32, ptr %8, align 4
  %40 = or i32 %39, %34
  %41 = ashr i32 %40, 1
  call void @solve(i32 noundef %32, i32 noundef %35, i32 noundef %38, i32 noundef %41)
  br label %23, !llvm.loop !9

42:                                               ; preds = %23, %13
  ret void
}

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nounwind }

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
