; ModuleID = 'benchmarks/hashtable.c'
source_filename = "benchmarks/hashtable.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.Slot = type { i32, i32, i8 }
%struct.timespec = type { i64, i64 }

@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@lcg_state = internal global i32 12345, align 4
@keys = internal global [100000 x i32] zeroinitializer, align 16
@values = internal global [100000 x i32] zeroinitializer, align 16
@sink = internal global i64 0, align 8
@table = internal global [200003 x %struct.Slot] zeroinitializer, align 16

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
  %19 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %5) #4
  call void @run_benchmark()
  %20 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %6) #4
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
  store i32 0, ptr %1, align 4
  br label %5

5:                                                ; preds = %23, %0
  %6 = load i32, ptr %1, align 4
  %7 = icmp slt i32 %6, 100000
  br i1 %7, label %8, label %26

8:                                                ; preds = %5
  %9 = call i32 @lcg_rand()
  %10 = shl i32 %9, 15
  %11 = call i32 @lcg_rand()
  %12 = or i32 %10, %11
  %13 = load i32, ptr %1, align 4
  %14 = sext i32 %13 to i64
  %15 = getelementptr inbounds [100000 x i32], ptr @keys, i64 0, i64 %14
  store i32 %12, ptr %15, align 4
  %16 = call i32 @lcg_rand()
  %17 = shl i32 %16, 15
  %18 = call i32 @lcg_rand()
  %19 = or i32 %17, %18
  %20 = load i32, ptr %1, align 4
  %21 = sext i32 %20 to i64
  %22 = getelementptr inbounds [100000 x i32], ptr @values, i64 0, i64 %21
  store i32 %19, ptr %22, align 4
  br label %23

23:                                               ; preds = %8
  %24 = load i32, ptr %1, align 4
  %25 = add nsw i32 %24, 1
  store i32 %25, ptr %1, align 4
  br label %5, !llvm.loop !9

26:                                               ; preds = %5
  call void @ht_clear()
  store i32 0, ptr %2, align 4
  br label %27

27:                                               ; preds = %39, %26
  %28 = load i32, ptr %2, align 4
  %29 = icmp slt i32 %28, 100000
  br i1 %29, label %30, label %42

30:                                               ; preds = %27
  %31 = load i32, ptr %2, align 4
  %32 = sext i32 %31 to i64
  %33 = getelementptr inbounds [100000 x i32], ptr @keys, i64 0, i64 %32
  %34 = load i32, ptr %33, align 4
  %35 = load i32, ptr %2, align 4
  %36 = sext i32 %35 to i64
  %37 = getelementptr inbounds [100000 x i32], ptr @values, i64 0, i64 %36
  %38 = load i32, ptr %37, align 4
  call void @ht_insert(i32 noundef %34, i32 noundef %38)
  br label %39

39:                                               ; preds = %30
  %40 = load i32, ptr %2, align 4
  %41 = add nsw i32 %40, 1
  store i32 %41, ptr %2, align 4
  br label %27, !llvm.loop !10

42:                                               ; preds = %27
  store i64 0, ptr %3, align 8
  store i32 0, ptr %4, align 4
  br label %43

43:                                               ; preds = %55, %42
  %44 = load i32, ptr %4, align 4
  %45 = icmp slt i32 %44, 100000
  br i1 %45, label %46, label %58

46:                                               ; preds = %43
  %47 = load i32, ptr %4, align 4
  %48 = sext i32 %47 to i64
  %49 = getelementptr inbounds [100000 x i32], ptr @keys, i64 0, i64 %48
  %50 = load i32, ptr %49, align 4
  %51 = call i32 @ht_lookup(i32 noundef %50)
  %52 = zext i32 %51 to i64
  %53 = load i64, ptr %3, align 8
  %54 = add i64 %53, %52
  store i64 %54, ptr %3, align 8
  br label %55

55:                                               ; preds = %46
  %56 = load i32, ptr %4, align 4
  %57 = add nsw i32 %56, 1
  store i32 %57, ptr %4, align 4
  br label %43, !llvm.loop !11

58:                                               ; preds = %43
  %59 = load i64, ptr %3, align 8
  store volatile i64 %59, ptr @sink, align 8
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
define internal void @ht_clear() #0 {
  call void @llvm.memset.p0.i64(ptr align 16 @table, i8 0, i64 2400036, i1 false)
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal void @ht_insert(i32 noundef %0, i32 noundef %1) #0 {
  %3 = alloca i32, align 4
  %4 = alloca i32, align 4
  %5 = alloca i32, align 4
  store i32 %0, ptr %3, align 4
  store i32 %1, ptr %4, align 4
  %6 = load i32, ptr %3, align 4
  %7 = urem i32 %6, 200003
  store i32 %7, ptr %5, align 4
  br label %8

8:                                                ; preds = %30, %2
  %9 = load i32, ptr %5, align 4
  %10 = zext i32 %9 to i64
  %11 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %10
  %12 = getelementptr inbounds nuw %struct.Slot, ptr %11, i32 0, i32 2
  %13 = load i8, ptr %12, align 4
  %14 = zext i8 %13 to i32
  %15 = icmp eq i32 %14, 1
  br i1 %15, label %16, label %34

16:                                               ; preds = %8
  %17 = load i32, ptr %5, align 4
  %18 = zext i32 %17 to i64
  %19 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %18
  %20 = getelementptr inbounds nuw %struct.Slot, ptr %19, i32 0, i32 0
  %21 = load i32, ptr %20, align 4
  %22 = load i32, ptr %3, align 4
  %23 = icmp eq i32 %21, %22
  br i1 %23, label %24, label %30

24:                                               ; preds = %16
  %25 = load i32, ptr %4, align 4
  %26 = load i32, ptr %5, align 4
  %27 = zext i32 %26 to i64
  %28 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %27
  %29 = getelementptr inbounds nuw %struct.Slot, ptr %28, i32 0, i32 1
  store i32 %25, ptr %29, align 4
  br label %49

30:                                               ; preds = %16
  %31 = load i32, ptr %5, align 4
  %32 = add i32 %31, 1
  %33 = urem i32 %32, 200003
  store i32 %33, ptr %5, align 4
  br label %8, !llvm.loop !12

34:                                               ; preds = %8
  %35 = load i32, ptr %3, align 4
  %36 = load i32, ptr %5, align 4
  %37 = zext i32 %36 to i64
  %38 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %37
  %39 = getelementptr inbounds nuw %struct.Slot, ptr %38, i32 0, i32 0
  store i32 %35, ptr %39, align 4
  %40 = load i32, ptr %4, align 4
  %41 = load i32, ptr %5, align 4
  %42 = zext i32 %41 to i64
  %43 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %42
  %44 = getelementptr inbounds nuw %struct.Slot, ptr %43, i32 0, i32 1
  store i32 %40, ptr %44, align 4
  %45 = load i32, ptr %5, align 4
  %46 = zext i32 %45 to i64
  %47 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %46
  %48 = getelementptr inbounds nuw %struct.Slot, ptr %47, i32 0, i32 2
  store i8 1, ptr %48, align 4
  br label %49

49:                                               ; preds = %34, %24
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal i32 @ht_lookup(i32 noundef %0) #0 {
  %2 = alloca i32, align 4
  %3 = alloca i32, align 4
  %4 = alloca i32, align 4
  store i32 %0, ptr %3, align 4
  %5 = load i32, ptr %3, align 4
  %6 = urem i32 %5, 200003
  store i32 %6, ptr %4, align 4
  br label %7

7:                                                ; preds = %29, %1
  %8 = load i32, ptr %4, align 4
  %9 = zext i32 %8 to i64
  %10 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %9
  %11 = getelementptr inbounds nuw %struct.Slot, ptr %10, i32 0, i32 2
  %12 = load i8, ptr %11, align 4
  %13 = zext i8 %12 to i32
  %14 = icmp eq i32 %13, 1
  br i1 %14, label %15, label %33

15:                                               ; preds = %7
  %16 = load i32, ptr %4, align 4
  %17 = zext i32 %16 to i64
  %18 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %17
  %19 = getelementptr inbounds nuw %struct.Slot, ptr %18, i32 0, i32 0
  %20 = load i32, ptr %19, align 4
  %21 = load i32, ptr %3, align 4
  %22 = icmp eq i32 %20, %21
  br i1 %22, label %23, label %29

23:                                               ; preds = %15
  %24 = load i32, ptr %4, align 4
  %25 = zext i32 %24 to i64
  %26 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %25
  %27 = getelementptr inbounds nuw %struct.Slot, ptr %26, i32 0, i32 1
  %28 = load i32, ptr %27, align 4
  store i32 %28, ptr %2, align 4
  br label %34

29:                                               ; preds = %15
  %30 = load i32, ptr %4, align 4
  %31 = add i32 %30, 1
  %32 = urem i32 %31, 200003
  store i32 %32, ptr %4, align 4
  br label %7, !llvm.loop !13

33:                                               ; preds = %7
  store i32 0, ptr %2, align 4
  br label %34

34:                                               ; preds = %33, %23
  %35 = load i32, ptr %2, align 4
  ret i32 %35
}

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #3

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #4 = { nounwind }

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
