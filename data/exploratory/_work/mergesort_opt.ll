; ModuleID = 'data/exploratory/_work/mergesort.ll'
source_filename = "benchmarks/mergesort.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@lcg_state = internal global i32 12345, align 4
@data = internal global [500000 x i32] zeroinitializer, align 16
@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@work = internal global [500000 x i32] zeroinitializer, align 16
@aux = internal global [500000 x i32] zeroinitializer, align 16

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca i32, align 4
  %2 = alloca i32, align 4
  %3 = alloca i32, align 4
  %4 = alloca [50 x i64], align 16
  %5 = alloca i32, align 4
  %6 = alloca %struct.timespec, align 8
  %7 = alloca %struct.timespec, align 8
  store i32 0, ptr %1, align 4
  store i32 12345, ptr @lcg_state, align 4
  store i32 0, ptr %2, align 4
  br label %8

8:                                                ; preds = %18, %0
  %9 = phi i32 [ %19, %18 ], [ 0, %0 ]
  %10 = icmp slt i32 %9, 500000
  br i1 %10, label %11, label %20

11:                                               ; preds = %8
  %12 = tail call i32 @lcg_rand()
  %13 = shl i32 %12, 16
  %14 = tail call i32 @lcg_rand()
  %15 = or i32 %13, %14
  %16 = sext i32 %9 to i64
  %17 = getelementptr inbounds [500000 x i32], ptr @data, i64 0, i64 %16
  store i32 %15, ptr %17, align 4
  br label %18

18:                                               ; preds = %11
  %19 = add nsw i32 %9, 1
  br label %8, !llvm.loop !6

20:                                               ; preds = %8
  %.lcssa6 = phi i32 [ %9, %8 ]
  store i32 %.lcssa6, ptr %2, align 4
  store i32 0, ptr %3, align 4
  br label %21

21:                                               ; preds = %20
  br label %22

22:                                               ; preds = %21
  tail call void @do_mergesort()
  br label %23

23:                                               ; preds = %22
  br label %24

24:                                               ; preds = %23
  tail call void @do_mergesort()
  br label %25

25:                                               ; preds = %24
  br label %26

26:                                               ; preds = %25
  tail call void @do_mergesort()
  br label %27

27:                                               ; preds = %26
  br label %28

28:                                               ; preds = %27
  tail call void @do_mergesort()
  br label %29

29:                                               ; preds = %28
  br label %30

30:                                               ; preds = %29
  tail call void @do_mergesort()
  br label %31

31:                                               ; preds = %30
  br i1 false, label %32, label %34

32:                                               ; preds = %31
  tail call void @do_mergesort()
  br label %33

33:                                               ; preds = %32
  unreachable

34:                                               ; preds = %31
  %.lcssa5 = phi i32 [ 5, %31 ]
  store i32 %.lcssa5, ptr %3, align 4
  store i32 0, ptr %5, align 4
  br label %35

35:                                               ; preds = %44, %34
  %36 = phi i32 [ %45, %44 ], [ 0, %34 ]
  %37 = icmp slt i32 %36, 50
  br i1 %37, label %38, label %46

38:                                               ; preds = %35
  %39 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %6) #4
  call void @do_mergesort()
  %40 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %7) #4
  %41 = call i64 @timespec_diff_ns(ptr noundef %6, ptr noundef %7)
  %42 = sext i32 %36 to i64
  %43 = getelementptr inbounds [50 x i64], ptr %4, i64 0, i64 %42
  store i64 %41, ptr %43, align 8
  br label %44

44:                                               ; preds = %38
  %45 = add nsw i32 %36, 1
  br label %35, !llvm.loop !8

46:                                               ; preds = %35
  %.lcssa = phi i32 [ %36, %35 ]
  store i32 %.lcssa, ptr %5, align 4
  call void @qsort(ptr noundef %4, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %47 = getelementptr inbounds [50 x i64], ptr %4, i64 0, i64 25
  %48 = load i64, ptr %47, align 8
  %49 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %48)
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
define internal void @do_mergesort() #0 {
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 16 @work, ptr align 16 @data, i64 2000000, i1 false)
  tail call void @mergesort_rec(ptr noundef @work, ptr noundef @aux, i32 noundef 0, i32 noundef 500000)
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

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #3

; Function Attrs: noinline nounwind uwtable
define internal void @mergesort_rec(ptr noundef %0, ptr noundef %1, i32 noundef %2, i32 noundef %3) #0 {
  %5 = alloca ptr, align 8
  %6 = alloca ptr, align 8
  %7 = alloca i32, align 4
  %8 = alloca i32, align 4
  %9 = alloca i32, align 4
  store ptr %0, ptr %5, align 8
  store ptr %1, ptr %6, align 8
  store i32 %2, ptr %7, align 4
  store i32 %3, ptr %8, align 4
  %10 = load i32, ptr %7, align 4
  %11 = sub nsw i32 %3, %10
  %12 = icmp sle i32 %11, 1
  br i1 %12, label %13, label %14

13:                                               ; preds = %4
  br label %29

14:                                               ; preds = %4
  %15 = sdiv i32 %11, 2
  %16 = add nsw i32 %10, %15
  store i32 %16, ptr %9, align 4
  %17 = load ptr, ptr %5, align 8
  %18 = load ptr, ptr %6, align 8
  %19 = load i32, ptr %7, align 4
  tail call void @mergesort_rec(ptr noundef %17, ptr noundef %18, i32 noundef %19, i32 noundef %16)
  %20 = load ptr, ptr %5, align 8
  %21 = load ptr, ptr %6, align 8
  %22 = load i32, ptr %9, align 4
  %23 = load i32, ptr %8, align 4
  tail call void @mergesort_rec(ptr noundef %20, ptr noundef %21, i32 noundef %22, i32 noundef %23)
  %24 = load ptr, ptr %5, align 8
  %25 = load ptr, ptr %6, align 8
  %26 = load i32, ptr %7, align 4
  %27 = load i32, ptr %9, align 4
  %28 = load i32, ptr %8, align 4
  tail call void @merge(ptr noundef %24, ptr noundef %25, i32 noundef %26, i32 noundef %27, i32 noundef %28)
  ret void

29:                                               ; preds = %13
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal void @merge(ptr noundef %0, ptr noundef %1, i32 noundef %2, i32 noundef %3, i32 noundef %4) #0 {
  %6 = alloca ptr, align 8
  %7 = alloca ptr, align 8
  %8 = alloca i32, align 4
  %9 = alloca i32, align 4
  %10 = alloca i32, align 4
  %11 = alloca i32, align 4
  %12 = alloca i32, align 4
  %13 = alloca i32, align 4
  store ptr %0, ptr %6, align 8
  store ptr %1, ptr %7, align 8
  store i32 %2, ptr %8, align 4
  store i32 %3, ptr %9, align 4
  store i32 %4, ptr %10, align 4
  %14 = load i32, ptr %8, align 4
  store i32 %14, ptr %11, align 4
  %15 = load i32, ptr %9, align 4
  store i32 %15, ptr %12, align 4
  %16 = load i32, ptr %8, align 4
  store i32 %16, ptr %13, align 4
  %17 = load i32, ptr %9, align 4
  %18 = load i32, ptr %10, align 4
  %19 = load ptr, ptr %6, align 8
  %20 = load ptr, ptr %7, align 8
  %.promoted = load i32, ptr %11, align 4
  %.promoted1 = load i32, ptr %12, align 4
  br label %21

21:                                               ; preds = %48, %5
  %22 = phi i32 [ %49, %48 ], [ %16, %5 ]
  %23 = phi i32 [ %50, %48 ], [ %.promoted1, %5 ]
  %24 = phi i32 [ %51, %48 ], [ %.promoted, %5 ]
  %25 = icmp slt i32 %24, %17
  br i1 %25, label %26, label %28

26:                                               ; preds = %21
  %27 = icmp slt i32 %23, %18
  br label %28

28:                                               ; preds = %26, %21
  %29 = phi i1 [ false, %21 ], [ %27, %26 ]
  br i1 %29, label %30, label %52

30:                                               ; preds = %28
  %31 = sext i32 %24 to i64
  %32 = getelementptr inbounds i32, ptr %19, i64 %31
  %33 = load i32, ptr %32, align 4
  %34 = sext i32 %23 to i64
  %35 = getelementptr inbounds i32, ptr %19, i64 %34
  %36 = load i32, ptr %35, align 4
  %37 = icmp sle i32 %33, %36
  br i1 %37, label %38, label %43

38:                                               ; preds = %30
  %39 = add nsw i32 %24, 1
  %40 = add nsw i32 %22, 1
  %41 = sext i32 %22 to i64
  %42 = getelementptr inbounds i32, ptr %20, i64 %41
  store i32 %33, ptr %42, align 4
  br label %48

43:                                               ; preds = %30
  %44 = add nsw i32 %23, 1
  %45 = add nsw i32 %22, 1
  %46 = sext i32 %22 to i64
  %47 = getelementptr inbounds i32, ptr %20, i64 %46
  store i32 %36, ptr %47, align 4
  br label %48

48:                                               ; preds = %43, %38
  %49 = phi i32 [ %45, %43 ], [ %40, %38 ]
  %50 = phi i32 [ %44, %43 ], [ %23, %38 ]
  %51 = phi i32 [ %24, %43 ], [ %39, %38 ]
  br label %21, !llvm.loop !9

52:                                               ; preds = %28
  %.lcssa18 = phi i32 [ %22, %28 ]
  %.lcssa17 = phi i32 [ %23, %28 ]
  %.lcssa16 = phi i32 [ %24, %28 ]
  store i32 %.lcssa16, ptr %11, align 4
  store i32 %.lcssa17, ptr %12, align 4
  store i32 %.lcssa18, ptr %13, align 4
  %53 = load i32, ptr %9, align 4
  %54 = load ptr, ptr %6, align 8
  %55 = load ptr, ptr %7, align 8
  %.promoted5 = load i32, ptr %11, align 4
  br label %56

56:                                               ; preds = %60, %52
  %57 = phi i32 [ %65, %60 ], [ %.lcssa18, %52 ]
  %58 = phi i32 [ %61, %60 ], [ %.promoted5, %52 ]
  %59 = icmp slt i32 %58, %53
  br i1 %59, label %60, label %68

60:                                               ; preds = %56
  %61 = add nsw i32 %58, 1
  %62 = sext i32 %58 to i64
  %63 = getelementptr inbounds i32, ptr %54, i64 %62
  %64 = load i32, ptr %63, align 4
  %65 = add nsw i32 %57, 1
  %66 = sext i32 %57 to i64
  %67 = getelementptr inbounds i32, ptr %55, i64 %66
  store i32 %64, ptr %67, align 4
  br label %56, !llvm.loop !10

68:                                               ; preds = %56
  %.lcssa15 = phi i32 [ %57, %56 ]
  %.lcssa14 = phi i32 [ %58, %56 ]
  store i32 %.lcssa14, ptr %11, align 4
  store i32 %.lcssa15, ptr %13, align 4
  %69 = load i32, ptr %10, align 4
  %70 = load ptr, ptr %6, align 8
  %71 = load ptr, ptr %7, align 8
  %.promoted9 = load i32, ptr %12, align 4
  br label %72

72:                                               ; preds = %76, %68
  %73 = phi i32 [ %81, %76 ], [ %.lcssa15, %68 ]
  %74 = phi i32 [ %77, %76 ], [ %.promoted9, %68 ]
  %75 = icmp slt i32 %74, %69
  br i1 %75, label %76, label %84

76:                                               ; preds = %72
  %77 = add nsw i32 %74, 1
  %78 = sext i32 %74 to i64
  %79 = getelementptr inbounds i32, ptr %70, i64 %78
  %80 = load i32, ptr %79, align 4
  %81 = add nsw i32 %73, 1
  %82 = sext i32 %73 to i64
  %83 = getelementptr inbounds i32, ptr %71, i64 %82
  store i32 %80, ptr %83, align 4
  br label %72, !llvm.loop !11

84:                                               ; preds = %72
  %.lcssa13 = phi i32 [ %73, %72 ]
  %.lcssa = phi i32 [ %74, %72 ]
  store i32 %.lcssa, ptr %12, align 4
  store i32 %.lcssa13, ptr %13, align 4
  %85 = load ptr, ptr %6, align 8
  %86 = load i32, ptr %8, align 4
  %87 = sext i32 %86 to i64
  %88 = getelementptr inbounds i32, ptr %85, i64 %87
  %89 = load ptr, ptr %7, align 8
  %90 = getelementptr inbounds i32, ptr %89, i64 %87
  %91 = load i32, ptr %10, align 4
  %92 = sub nsw i32 %91, %86
  %93 = sext i32 %92 to i64
  %94 = mul i64 %93, 4
  tail call void @llvm.memcpy.p0.p0.i64(ptr align 4 %88, ptr align 4 %90, i64 %94, i1 false)
  ret void
}

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nocallback nofree nounwind willreturn memory(argmem: readwrite) }
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
