; ModuleID = 'benchmarks/nqueens.c'
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

7:                                                ; preds = %11, %0
  %8 = load i32, ptr %2, align 4
  %9 = icmp slt i32 %8, 5
  br i1 %9, label %10, label %14

10:                                               ; preds = %7
  call void @do_nqueens()
  br label %11

11:                                               ; preds = %10
  %12 = load i32, ptr %2, align 4
  %13 = add nsw i32 %12, 1
  store i32 %13, ptr %2, align 4
  br label %7, !llvm.loop !6

14:                                               ; preds = %7
  store i32 0, ptr %4, align 4
  br label %15

15:                                               ; preds = %25, %14
  %16 = load i32, ptr %4, align 4
  %17 = icmp slt i32 %16, 50
  br i1 %17, label %18, label %28

18:                                               ; preds = %15
  %19 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %5) #3
  call void @do_nqueens()
  %20 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %6) #3
  %21 = call i64 @timespec_diff_ns(ptr noundef %5, ptr noundef %6)
  %22 = load i32, ptr %4, align 4
  %23 = sext i32 %22 to i64
  %24 = getelementptr inbounds [50 x i64], ptr %3, i64 0, i64 %23
  store i64 %21, ptr %24, align 8
  br label %25

25:                                               ; preds = %18
  %26 = load i32, ptr %4, align 4
  %27 = add nsw i32 %26, 1
  store i32 %27, ptr %4, align 4
  br label %15, !llvm.loop !8

28:                                               ; preds = %15
  %29 = getelementptr inbounds [50 x i64], ptr %3, i64 0, i64 0
  call void @qsort(ptr noundef %29, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %30 = getelementptr inbounds [50 x i64], ptr %3, i64 0, i64 25
  %31 = load i64, ptr %30, align 8
  %32 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %31)
  ret i32 0
}

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
  br label %48

16:                                               ; preds = %4
  %17 = load i32, ptr %6, align 4
  %18 = load i32, ptr %7, align 4
  %19 = or i32 %17, %18
  %20 = load i32, ptr %8, align 4
  %21 = or i32 %19, %20
  %22 = xor i32 %21, -1
  %23 = and i32 8191, %22
  store i32 %23, ptr %9, align 4
  br label %24

24:                                               ; preds = %27, %16
  %25 = load i32, ptr %9, align 4
  %26 = icmp ne i32 %25, 0
  br i1 %26, label %27, label %48

27:                                               ; preds = %24
  %28 = load i32, ptr %9, align 4
  %29 = load i32, ptr %9, align 4
  %30 = sub nsw i32 0, %29
  %31 = and i32 %28, %30
  store i32 %31, ptr %10, align 4
  %32 = load i32, ptr %10, align 4
  %33 = load i32, ptr %9, align 4
  %34 = sub nsw i32 %33, %32
  store i32 %34, ptr %9, align 4
  %35 = load i32, ptr %5, align 4
  %36 = add nsw i32 %35, 1
  %37 = load i32, ptr %6, align 4
  %38 = load i32, ptr %10, align 4
  %39 = or i32 %37, %38
  %40 = load i32, ptr %7, align 4
  %41 = load i32, ptr %10, align 4
  %42 = or i32 %40, %41
  %43 = shl i32 %42, 1
  %44 = load i32, ptr %8, align 4
  %45 = load i32, ptr %10, align 4
  %46 = or i32 %44, %45
  %47 = ashr i32 %46, 1
  call void @solve(i32 noundef %36, i32 noundef %39, i32 noundef %43, i32 noundef %47)
  br label %24, !llvm.loop !9

48:                                               ; preds = %13, %24
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
