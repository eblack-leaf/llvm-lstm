; ModuleID = 'benchmarks/heap_ops.c'
source_filename = "benchmarks/heap_ops.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@lcg_state = internal global i32 12345, align 4
@sink = internal global i64 0, align 8
@heap_size = internal global i32 0, align 4
@heap = internal global [200001 x i32] zeroinitializer, align 16

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
  call void @run_benchmark()
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
  call void @run_benchmark()
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
define internal void @run_benchmark() #0 {
  %1 = alloca i32, align 4
  %2 = alloca i32, align 4
  %3 = alloca i64, align 8
  %4 = alloca i32, align 4
  store i32 12345, ptr @lcg_state, align 4
  call void @heap_init()
  store i32 0, ptr %1, align 4
  br label %5

5:                                                ; preds = %14, %0
  %6 = load i32, ptr %1, align 4
  %7 = icmp slt i32 %6, 200000
  br i1 %7, label %8, label %17

8:                                                ; preds = %5
  %9 = call i32 @lcg_rand()
  %10 = shl i32 %9, 15
  %11 = call i32 @lcg_rand()
  %12 = or i32 %10, %11
  store i32 %12, ptr %2, align 4
  %13 = load i32, ptr %2, align 4
  call void @heap_push(i32 noundef %13)
  br label %14

14:                                               ; preds = %8
  %15 = load i32, ptr %1, align 4
  %16 = add nsw i32 %15, 1
  store i32 %16, ptr %1, align 4
  br label %5, !llvm.loop !9

17:                                               ; preds = %5
  store i64 0, ptr %3, align 8
  store i32 0, ptr %4, align 4
  br label %18

18:                                               ; preds = %26, %17
  %19 = load i32, ptr %4, align 4
  %20 = icmp slt i32 %19, 200000
  br i1 %20, label %21, label %29

21:                                               ; preds = %18
  %22 = call i32 @heap_pop()
  %23 = sext i32 %22 to i64
  %24 = load i64, ptr %3, align 8
  %25 = add nsw i64 %24, %23
  store i64 %25, ptr %3, align 8
  br label %26

26:                                               ; preds = %21
  %27 = load i32, ptr %4, align 4
  %28 = add nsw i32 %27, 1
  store i32 %28, ptr %4, align 4
  br label %18, !llvm.loop !10

29:                                               ; preds = %18
  %30 = load i64, ptr %3, align 8
  store volatile i64 %30, ptr @sink, align 8
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
define internal void @heap_init() #0 {
  store i32 0, ptr @heap_size, align 4
  ret void
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
define internal void @heap_push(i32 noundef %0) #0 {
  %2 = alloca i32, align 4
  %3 = alloca i32, align 4
  %4 = alloca i32, align 4
  store i32 %0, ptr %2, align 4
  %5 = load i32, ptr @heap_size, align 4
  %6 = add nsw i32 %5, 1
  store i32 %6, ptr @heap_size, align 4
  %7 = load i32, ptr @heap_size, align 4
  store i32 %7, ptr %3, align 4
  %8 = load i32, ptr %2, align 4
  %9 = load i32, ptr %3, align 4
  %10 = sext i32 %9 to i64
  %11 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %10
  store i32 %8, ptr %11, align 4
  br label %12

12:                                               ; preds = %28, %1
  %13 = load i32, ptr %3, align 4
  %14 = icmp sgt i32 %13, 1
  br i1 %14, label %15, label %26

15:                                               ; preds = %12
  %16 = load i32, ptr %3, align 4
  %17 = sext i32 %16 to i64
  %18 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %17
  %19 = load i32, ptr %18, align 4
  %20 = load i32, ptr %3, align 4
  %21 = sdiv i32 %20, 2
  %22 = sext i32 %21 to i64
  %23 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %22
  %24 = load i32, ptr %23, align 4
  %25 = icmp slt i32 %19, %24
  br label %26

26:                                               ; preds = %15, %12
  %27 = phi i1 [ false, %12 ], [ %25, %15 ]
  br i1 %27, label %28, label %48

28:                                               ; preds = %26
  %29 = load i32, ptr %3, align 4
  %30 = sext i32 %29 to i64
  %31 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %30
  %32 = load i32, ptr %31, align 4
  store i32 %32, ptr %4, align 4
  %33 = load i32, ptr %3, align 4
  %34 = sdiv i32 %33, 2
  %35 = sext i32 %34 to i64
  %36 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %35
  %37 = load i32, ptr %36, align 4
  %38 = load i32, ptr %3, align 4
  %39 = sext i32 %38 to i64
  %40 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %39
  store i32 %37, ptr %40, align 4
  %41 = load i32, ptr %4, align 4
  %42 = load i32, ptr %3, align 4
  %43 = sdiv i32 %42, 2
  %44 = sext i32 %43 to i64
  %45 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %44
  store i32 %41, ptr %45, align 4
  %46 = load i32, ptr %3, align 4
  %47 = sdiv i32 %46, 2
  store i32 %47, ptr %3, align 4
  br label %12, !llvm.loop !11

48:                                               ; preds = %26
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal i32 @heap_pop() #0 {
  %1 = alloca i32, align 4
  %2 = alloca i32, align 4
  %3 = alloca i32, align 4
  %4 = alloca i32, align 4
  %5 = alloca i32, align 4
  %6 = alloca i32, align 4
  %7 = load i32, ptr getelementptr inbounds ([200001 x i32], ptr @heap, i64 0, i64 1), align 4
  store i32 %7, ptr %1, align 4
  %8 = load i32, ptr @heap_size, align 4
  %9 = sext i32 %8 to i64
  %10 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %9
  %11 = load i32, ptr %10, align 4
  store i32 %11, ptr getelementptr inbounds ([200001 x i32], ptr @heap, i64 0, i64 1), align 4
  %12 = load i32, ptr @heap_size, align 4
  %13 = add nsw i32 %12, -1
  store i32 %13, ptr @heap_size, align 4
  store i32 1, ptr %2, align 4
  br label %14

14:                                               ; preds = %0, %57
  %15 = load i32, ptr %2, align 4
  store i32 %15, ptr %3, align 4
  %16 = load i32, ptr %2, align 4
  %17 = mul nsw i32 2, %16
  store i32 %17, ptr %4, align 4
  %18 = load i32, ptr %2, align 4
  %19 = mul nsw i32 2, %18
  %20 = add nsw i32 %19, 1
  store i32 %20, ptr %5, align 4
  %21 = load i32, ptr %4, align 4
  %22 = load i32, ptr @heap_size, align 4
  %23 = icmp sle i32 %21, %22
  br i1 %23, label %24, label %36

24:                                               ; preds = %14
  %25 = load i32, ptr %4, align 4
  %26 = sext i32 %25 to i64
  %27 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %26
  %28 = load i32, ptr %27, align 4
  %29 = load i32, ptr %3, align 4
  %30 = sext i32 %29 to i64
  %31 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %30
  %32 = load i32, ptr %31, align 4
  %33 = icmp slt i32 %28, %32
  br i1 %33, label %34, label %36

34:                                               ; preds = %24
  %35 = load i32, ptr %4, align 4
  store i32 %35, ptr %3, align 4
  br label %36

36:                                               ; preds = %34, %24, %14
  %37 = load i32, ptr %5, align 4
  %38 = load i32, ptr @heap_size, align 4
  %39 = icmp sle i32 %37, %38
  br i1 %39, label %40, label %52

40:                                               ; preds = %36
  %41 = load i32, ptr %5, align 4
  %42 = sext i32 %41 to i64
  %43 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %42
  %44 = load i32, ptr %43, align 4
  %45 = load i32, ptr %3, align 4
  %46 = sext i32 %45 to i64
  %47 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %46
  %48 = load i32, ptr %47, align 4
  %49 = icmp slt i32 %44, %48
  br i1 %49, label %50, label %52

50:                                               ; preds = %40
  %51 = load i32, ptr %5, align 4
  store i32 %51, ptr %3, align 4
  br label %52

52:                                               ; preds = %50, %40, %36
  %53 = load i32, ptr %3, align 4
  %54 = load i32, ptr %2, align 4
  %55 = icmp eq i32 %53, %54
  br i1 %55, label %56, label %57

56:                                               ; preds = %52
  br label %74

57:                                               ; preds = %52
  %58 = load i32, ptr %2, align 4
  %59 = sext i32 %58 to i64
  %60 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %59
  %61 = load i32, ptr %60, align 4
  store i32 %61, ptr %6, align 4
  %62 = load i32, ptr %3, align 4
  %63 = sext i32 %62 to i64
  %64 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %63
  %65 = load i32, ptr %64, align 4
  %66 = load i32, ptr %2, align 4
  %67 = sext i32 %66 to i64
  %68 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %67
  store i32 %65, ptr %68, align 4
  %69 = load i32, ptr %6, align 4
  %70 = load i32, ptr %3, align 4
  %71 = sext i32 %70 to i64
  %72 = getelementptr inbounds [200001 x i32], ptr @heap, i64 0, i64 %71
  store i32 %69, ptr %72, align 4
  %73 = load i32, ptr %3, align 4
  store i32 %73, ptr %2, align 4
  br label %14

74:                                               ; preds = %56
  %75 = load i32, ptr %1, align 4
  ret i32 %75
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
!10 = distinct !{!10, !7}
!11 = distinct !{!11, !7}
