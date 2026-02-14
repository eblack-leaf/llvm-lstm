; ModuleID = 'data/exploratory/_work/quicksort.ll'
source_filename = "benchmarks/quicksort.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@lcg_state = internal global i32 12345, align 4
@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca i64, align 8
  %2 = alloca [50 x i64], align 16
  %3 = alloca %struct.timespec, align 8
  %4 = alloca %struct.timespec, align 8
  %5 = tail call noalias dereferenceable_or_null(2000000) ptr @malloc(i64 noundef 2000000) #6
  %6 = tail call noalias dereferenceable_or_null(2000000) ptr @malloc(i64 noundef 2000000) #6
  store i32 12345, ptr @lcg_state, align 4
  br label %7

7:                                                ; preds = %16, %0
  %.0 = phi i32 [ 0, %0 ], [ %17, %16 ]
  %8 = icmp samesign ult i32 %.0, 500000
  br i1 %8, label %9, label %18

9:                                                ; preds = %7
  %10 = tail call i32 @lcg_rand()
  %11 = shl i32 %10, 16
  %12 = tail call i32 @lcg_rand()
  %13 = or i32 %11, %12
  %14 = zext nneg i32 %.0 to i64
  %15 = getelementptr inbounds nuw i32, ptr %5, i64 %14
  store i32 %13, ptr %15, align 4
  br label %16

16:                                               ; preds = %9
  %17 = add nuw nsw i32 %.0, 1
  br label %7, !llvm.loop !6

18:                                               ; preds = %7
  br label %19

19:                                               ; preds = %23, %18
  %.1 = phi i32 [ 0, %18 ], [ %24, %23 ]
  %20 = icmp samesign ult i32 %.1, 5
  br i1 %20, label %21, label %25

21:                                               ; preds = %19
  %22 = tail call i64 @workload(ptr noundef %6, ptr noundef %5)
  store volatile i64 %22, ptr %1, align 8
  br label %23

23:                                               ; preds = %21
  %24 = add nuw nsw i32 %.1, 1
  br label %19, !llvm.loop !8

25:                                               ; preds = %19
  br label %26

26:                                               ; preds = %35, %25
  %.2 = phi i32 [ 0, %25 ], [ %36, %35 ]
  %27 = icmp samesign ult i32 %.2, 50
  br i1 %27, label %28, label %37

28:                                               ; preds = %26
  %29 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %3) #7
  %30 = call i64 @workload(ptr noundef %6, ptr noundef %5)
  store volatile i64 %30, ptr %1, align 8
  %31 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %4) #7
  %32 = call i64 @timespec_diff_ns(ptr noundef nonnull %3, ptr noundef nonnull %4)
  %33 = zext nneg i32 %.2 to i64
  %34 = getelementptr inbounds nuw [50 x i64], ptr %2, i64 0, i64 %33
  store i64 %32, ptr %34, align 8
  br label %35

35:                                               ; preds = %28
  %36 = add nuw nsw i32 %.2, 1
  br label %26, !llvm.loop !9

37:                                               ; preds = %26
  call void @qsort(ptr noundef nonnull %2, i64 noundef 50, i64 noundef 8, ptr noundef nonnull @cmp_ll) #7
  %38 = getelementptr inbounds nuw i8, ptr %2, i64 200
  %39 = load i64, ptr %38, align 8
  %40 = call i32 (ptr, ...) @printf(ptr noundef nonnull dereferenceable(1) @.str, i64 noundef %39) #7
  call void @free(ptr noundef %5) #7
  call void @free(ptr noundef %6) #7
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
define internal i64 @workload(ptr noundef %0, ptr noundef %1) #0 {
  tail call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 4 dereferenceable(2000000) %0, ptr noundef nonnull align 4 dereferenceable(2000000) %1, i64 2000000, i1 false)
  tail call void @quicksort(ptr noundef %0, i32 noundef 0, i32 noundef 499999)
  br label %3

3:                                                ; preds = %6, %2
  %.08 = phi i64 [ 0, %2 ], [ %11, %6 ]
  %.0 = phi i32 [ 0, %2 ], [ %12, %6 ]
  %4 = icmp samesign ult i32 %.0, 500000
  br i1 %4, label %5, label %13

5:                                                ; preds = %3
  br label %6

6:                                                ; preds = %5
  %7 = zext nneg i32 %.0 to i64
  %8 = getelementptr inbounds nuw i32, ptr %0, i64 %7
  %9 = load i32, ptr %8, align 4
  %10 = sext i32 %9 to i64
  %11 = add nsw i64 %.08, %10
  %12 = add nuw nsw i32 %.0, 1
  br label %3, !llvm.loop !10

13:                                               ; preds = %3
  ret i64 %.08
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

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #4

; Function Attrs: noinline nounwind uwtable
define internal void @quicksort(ptr noundef %0, i32 noundef %1, i32 noundef %2) #0 {
  br label %tailrecurse

tailrecurse:                                      ; preds = %40, %3
  %.tr40 = phi i32 [ %1, %3 ], [ %43, %40 ]
  %4 = sub nsw i32 %2, %.tr40
  %5 = icmp slt i32 %4, 2
  br i1 %5, label %6, label %18

6:                                                ; preds = %tailrecurse
  %7 = icmp sgt i32 %2, %.tr40
  br i1 %7, label %8, label %17

8:                                                ; preds = %6
  %9 = sext i32 %.tr40 to i64
  %10 = getelementptr inbounds i32, ptr %0, i64 %9
  %11 = load i32, ptr %10, align 4
  %12 = sext i32 %2 to i64
  %13 = getelementptr inbounds i32, ptr %0, i64 %12
  %14 = load i32, ptr %13, align 4
  %15 = icmp sgt i32 %11, %14
  br i1 %15, label %16, label %17

16:                                               ; preds = %8
  tail call void @swap(ptr noundef nonnull %10, ptr noundef nonnull %13)
  br label %17

17:                                               ; preds = %16, %8, %6
  br label %44

18:                                               ; preds = %tailrecurse
  %19 = tail call i32 @median_of_three(ptr noundef %0, i32 noundef %.tr40, i32 noundef %2)
  %20 = add nsw i32 %2, -1
  br label %21

21:                                               ; preds = %39, %18
  %.038 = phi i32 [ %.tr40, %18 ], [ %23, %39 ]
  %.0 = phi i32 [ %20, %18 ], [ %31, %39 ]
  br label %22

22:                                               ; preds = %28, %21
  %.139 = phi i32 [ %.038, %21 ], [ %23, %28 ]
  %23 = add nsw i32 %.139, 1
  %24 = sext i32 %23 to i64
  %25 = getelementptr inbounds i32, ptr %0, i64 %24
  %26 = load i32, ptr %25, align 4
  %27 = icmp slt i32 %26, %19
  br i1 %27, label %28, label %29

28:                                               ; preds = %22
  br label %22, !llvm.loop !11

29:                                               ; preds = %22
  br label %30

30:                                               ; preds = %36, %29
  %.1 = phi i32 [ %.0, %29 ], [ %31, %36 ]
  %31 = add nsw i32 %.1, -1
  %32 = sext i32 %31 to i64
  %33 = getelementptr inbounds i32, ptr %0, i64 %32
  %34 = load i32, ptr %33, align 4
  %35 = icmp sgt i32 %34, %19
  br i1 %35, label %36, label %37

36:                                               ; preds = %30
  br label %30, !llvm.loop !12

37:                                               ; preds = %30
  %.not = icmp slt i32 %23, %31
  br i1 %.not, label %39, label %38

38:                                               ; preds = %37
  br label %40

39:                                               ; preds = %37
  tail call void @swap(ptr noundef nonnull %25, ptr noundef nonnull %33)
  br label %21

40:                                               ; preds = %38
  %41 = sext i32 %20 to i64
  %42 = getelementptr inbounds i32, ptr %0, i64 %41
  tail call void @swap(ptr noundef nonnull %25, ptr noundef %42)
  tail call void @quicksort(ptr noundef %0, i32 noundef %.tr40, i32 noundef %.139)
  %43 = add nsw i32 %.139, 2
  br label %tailrecurse

44:                                               ; preds = %17
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal void @swap(ptr noundef %0, ptr noundef %1) #0 {
  %3 = load i32, ptr %0, align 4
  %4 = load i32, ptr %1, align 4
  store i32 %4, ptr %0, align 4
  store i32 %3, ptr %1, align 4
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal i32 @median_of_three(ptr noundef %0, i32 noundef %1, i32 noundef %2) #0 {
  %4 = sub nsw i32 %2, %1
  %5 = sdiv i32 %4, 2
  %6 = add nsw i32 %1, %5
  %7 = sext i32 %1 to i64
  %8 = getelementptr inbounds i32, ptr %0, i64 %7
  %9 = load i32, ptr %8, align 4
  %10 = sext i32 %6 to i64
  %11 = getelementptr inbounds i32, ptr %0, i64 %10
  %12 = load i32, ptr %11, align 4
  %13 = icmp sgt i32 %9, %12
  br i1 %13, label %14, label %15

14:                                               ; preds = %3
  tail call void @swap(ptr noundef nonnull %8, ptr noundef nonnull %11)
  br label %15

15:                                               ; preds = %14, %3
  %16 = load i32, ptr %8, align 4
  %17 = sext i32 %2 to i64
  %18 = getelementptr inbounds i32, ptr %0, i64 %17
  %19 = load i32, ptr %18, align 4
  %20 = icmp sgt i32 %16, %19
  br i1 %20, label %21, label %22

21:                                               ; preds = %15
  tail call void @swap(ptr noundef nonnull %8, ptr noundef nonnull %18)
  br label %22

22:                                               ; preds = %21, %15
  %23 = load i32, ptr %11, align 4
  %24 = load i32, ptr %18, align 4
  %25 = icmp sgt i32 %23, %24
  br i1 %25, label %26, label %27

26:                                               ; preds = %22
  tail call void @swap(ptr noundef nonnull %11, ptr noundef nonnull %18)
  br label %27

27:                                               ; preds = %26, %22
  %28 = sext i32 %2 to i64
  %29 = getelementptr i32, ptr %0, i64 %28
  %30 = getelementptr i8, ptr %29, i64 -4
  tail call void @swap(ptr noundef nonnull %11, ptr noundef %30)
  %31 = load i32, ptr %30, align 4
  ret i32 %31
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.scmp.i32.i64(i64, i64) #5

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind allocsize(0) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { nocallback nofree nounwind willreturn memory(argmem: readwrite) }
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
