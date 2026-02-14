; ModuleID = 'benchmarks/binary_search.c'
source_filename = "benchmarks/binary_search.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@lcg_state = internal global i32 12345, align 4
@arr = internal global [1000000 x i32] zeroinitializer, align 16
@queries = internal global [1000000 x i32] zeroinitializer, align 16
@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@found_count = internal global i32 0, align 4

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca i32, align 4
  %2 = alloca i32, align 4
  %3 = alloca i32, align 4
  %4 = alloca i32, align 4
  %5 = alloca [50 x i64], align 16
  %6 = alloca i32, align 4
  %7 = alloca %struct.timespec, align 8
  %8 = alloca %struct.timespec, align 8
  store i32 0, ptr %1, align 4
  store i32 12345, ptr @lcg_state, align 4
  store i32 0, ptr %2, align 4
  br label %9

9:                                                ; preds = %20, %0
  %10 = load i32, ptr %2, align 4
  %11 = icmp slt i32 %10, 1000000
  br i1 %11, label %12, label %23

12:                                               ; preds = %9
  %13 = call i32 @lcg_rand()
  %14 = shl i32 %13, 16
  %15 = call i32 @lcg_rand()
  %16 = or i32 %14, %15
  %17 = load i32, ptr %2, align 4
  %18 = sext i32 %17 to i64
  %19 = getelementptr inbounds [1000000 x i32], ptr @arr, i64 0, i64 %18
  store i32 %16, ptr %19, align 4
  br label %20

20:                                               ; preds = %12
  %21 = load i32, ptr %2, align 4
  %22 = add nsw i32 %21, 1
  store i32 %22, ptr %2, align 4
  br label %9, !llvm.loop !6

23:                                               ; preds = %9
  call void @qsort(ptr noundef @arr, i64 noundef 1000000, i64 noundef 4, ptr noundef @cmp_int)
  store i32 67890, ptr @lcg_state, align 4
  store i32 0, ptr %3, align 4
  br label %24

24:                                               ; preds = %49, %23
  %25 = load i32, ptr %3, align 4
  %26 = icmp slt i32 %25, 1000000
  br i1 %26, label %27, label %52

27:                                               ; preds = %24
  %28 = call i32 @lcg_rand()
  %29 = urem i32 %28, 2
  %30 = icmp eq i32 %29, 0
  br i1 %30, label %31, label %40

31:                                               ; preds = %27
  %32 = call i32 @lcg_rand()
  %33 = urem i32 %32, 1000000
  %34 = zext i32 %33 to i64
  %35 = getelementptr inbounds nuw [1000000 x i32], ptr @arr, i64 0, i64 %34
  %36 = load i32, ptr %35, align 4
  %37 = load i32, ptr %3, align 4
  %38 = sext i32 %37 to i64
  %39 = getelementptr inbounds [1000000 x i32], ptr @queries, i64 0, i64 %38
  store i32 %36, ptr %39, align 4
  br label %48

40:                                               ; preds = %27
  %41 = call i32 @lcg_rand()
  %42 = shl i32 %41, 16
  %43 = call i32 @lcg_rand()
  %44 = or i32 %42, %43
  %45 = load i32, ptr %3, align 4
  %46 = sext i32 %45 to i64
  %47 = getelementptr inbounds [1000000 x i32], ptr @queries, i64 0, i64 %46
  store i32 %44, ptr %47, align 4
  br label %48

48:                                               ; preds = %40, %31
  br label %49

49:                                               ; preds = %48
  %50 = load i32, ptr %3, align 4
  %51 = add nsw i32 %50, 1
  store i32 %51, ptr %3, align 4
  br label %24, !llvm.loop !8

52:                                               ; preds = %24
  store i32 0, ptr %4, align 4
  br label %53

53:                                               ; preds = %57, %52
  %54 = load i32, ptr %4, align 4
  %55 = icmp slt i32 %54, 5
  br i1 %55, label %56, label %60

56:                                               ; preds = %53
  call void @do_benchmark()
  br label %57

57:                                               ; preds = %56
  %58 = load i32, ptr %4, align 4
  %59 = add nsw i32 %58, 1
  store i32 %59, ptr %4, align 4
  br label %53, !llvm.loop !9

60:                                               ; preds = %53
  store i32 0, ptr %6, align 4
  br label %61

61:                                               ; preds = %71, %60
  %62 = load i32, ptr %6, align 4
  %63 = icmp slt i32 %62, 50
  br i1 %63, label %64, label %74

64:                                               ; preds = %61
  %65 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %7) #3
  call void @do_benchmark()
  %66 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %8) #3
  %67 = call i64 @timespec_diff_ns(ptr noundef %7, ptr noundef %8)
  %68 = load i32, ptr %6, align 4
  %69 = sext i32 %68 to i64
  %70 = getelementptr inbounds [50 x i64], ptr %5, i64 0, i64 %69
  store i64 %67, ptr %70, align 8
  br label %71

71:                                               ; preds = %64
  %72 = load i32, ptr %6, align 4
  %73 = add nsw i32 %72, 1
  store i32 %73, ptr %6, align 4
  br label %61, !llvm.loop !10

74:                                               ; preds = %61
  %75 = getelementptr inbounds [50 x i64], ptr %5, i64 0, i64 0
  call void @qsort(ptr noundef %75, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %76 = getelementptr inbounds [50 x i64], ptr %5, i64 0, i64 25
  %77 = load i64, ptr %76, align 8
  %78 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %77)
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

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #1

; Function Attrs: noinline nounwind uwtable
define internal i32 @cmp_int(ptr noundef %0, ptr noundef %1) #0 {
  %3 = alloca ptr, align 8
  %4 = alloca ptr, align 8
  %5 = alloca i32, align 4
  %6 = alloca i32, align 4
  store ptr %0, ptr %3, align 8
  store ptr %1, ptr %4, align 8
  %7 = load ptr, ptr %3, align 8
  %8 = load i32, ptr %7, align 4
  store i32 %8, ptr %5, align 4
  %9 = load ptr, ptr %4, align 8
  %10 = load i32, ptr %9, align 4
  store i32 %10, ptr %6, align 4
  %11 = load i32, ptr %5, align 4
  %12 = load i32, ptr %6, align 4
  %13 = icmp sgt i32 %11, %12
  %14 = zext i1 %13 to i32
  %15 = load i32, ptr %5, align 4
  %16 = load i32, ptr %6, align 4
  %17 = icmp slt i32 %15, %16
  %18 = zext i1 %17 to i32
  %19 = sub nsw i32 %14, %18
  ret i32 %19
}

; Function Attrs: noinline nounwind uwtable
define internal void @do_benchmark() #0 {
  %1 = alloca i32, align 4
  %2 = alloca i32, align 4
  store i32 0, ptr %1, align 4
  store i32 0, ptr %2, align 4
  br label %3

3:                                                ; preds = %17, %0
  %4 = load i32, ptr %2, align 4
  %5 = icmp slt i32 %4, 1000000
  br i1 %5, label %6, label %20

6:                                                ; preds = %3
  %7 = load i32, ptr %2, align 4
  %8 = sext i32 %7 to i64
  %9 = getelementptr inbounds [1000000 x i32], ptr @queries, i64 0, i64 %8
  %10 = load i32, ptr %9, align 4
  %11 = call i32 @binary_search(ptr noundef @arr, i32 noundef 1000000, i32 noundef %10)
  %12 = icmp sge i32 %11, 0
  br i1 %12, label %13, label %16

13:                                               ; preds = %6
  %14 = load i32, ptr %1, align 4
  %15 = add nsw i32 %14, 1
  store i32 %15, ptr %1, align 4
  br label %16

16:                                               ; preds = %13, %6
  br label %17

17:                                               ; preds = %16
  %18 = load i32, ptr %2, align 4
  %19 = add nsw i32 %18, 1
  store i32 %19, ptr %2, align 4
  br label %3, !llvm.loop !11

20:                                               ; preds = %3
  %21 = load i32, ptr %1, align 4
  store volatile i32 %21, ptr @found_count, align 4
  ret void
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

declare i32 @printf(ptr noundef, ...) #1

; Function Attrs: noinline nounwind uwtable
define internal i32 @binary_search(ptr noundef %0, i32 noundef %1, i32 noundef %2) #0 {
  %4 = alloca i32, align 4
  %5 = alloca ptr, align 8
  %6 = alloca i32, align 4
  %7 = alloca i32, align 4
  %8 = alloca i32, align 4
  %9 = alloca i32, align 4
  %10 = alloca i32, align 4
  store ptr %0, ptr %5, align 8
  store i32 %1, ptr %6, align 4
  store i32 %2, ptr %7, align 4
  store i32 0, ptr %8, align 4
  %11 = load i32, ptr %6, align 4
  %12 = sub nsw i32 %11, 1
  store i32 %12, ptr %9, align 4
  br label %13

13:                                               ; preds = %48, %3
  %14 = load i32, ptr %8, align 4
  %15 = load i32, ptr %9, align 4
  %16 = icmp sle i32 %14, %15
  br i1 %16, label %17, label %49

17:                                               ; preds = %13
  %18 = load i32, ptr %8, align 4
  %19 = load i32, ptr %9, align 4
  %20 = load i32, ptr %8, align 4
  %21 = sub nsw i32 %19, %20
  %22 = sdiv i32 %21, 2
  %23 = add nsw i32 %18, %22
  store i32 %23, ptr %10, align 4
  %24 = load ptr, ptr %5, align 8
  %25 = load i32, ptr %10, align 4
  %26 = sext i32 %25 to i64
  %27 = getelementptr inbounds i32, ptr %24, i64 %26
  %28 = load i32, ptr %27, align 4
  %29 = load i32, ptr %7, align 4
  %30 = icmp eq i32 %28, %29
  br i1 %30, label %31, label %33

31:                                               ; preds = %17
  %32 = load i32, ptr %10, align 4
  store i32 %32, ptr %4, align 4
  br label %50

33:                                               ; preds = %17
  %34 = load ptr, ptr %5, align 8
  %35 = load i32, ptr %10, align 4
  %36 = sext i32 %35 to i64
  %37 = getelementptr inbounds i32, ptr %34, i64 %36
  %38 = load i32, ptr %37, align 4
  %39 = load i32, ptr %7, align 4
  %40 = icmp slt i32 %38, %39
  br i1 %40, label %41, label %44

41:                                               ; preds = %33
  %42 = load i32, ptr %10, align 4
  %43 = add nsw i32 %42, 1
  store i32 %43, ptr %8, align 4
  br label %47

44:                                               ; preds = %33
  %45 = load i32, ptr %10, align 4
  %46 = sub nsw i32 %45, 1
  store i32 %46, ptr %9, align 4
  br label %47

47:                                               ; preds = %44, %41
  br label %48

48:                                               ; preds = %47
  br label %13, !llvm.loop !12

49:                                               ; preds = %13
  store i32 -1, ptr %4, align 4
  br label %50

50:                                               ; preds = %49, %31
  %51 = load i32, ptr %4, align 4
  ret i32 %51
}

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
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
!12 = distinct !{!12, !7}
