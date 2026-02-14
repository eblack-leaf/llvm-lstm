; ModuleID = 'data/exploratory/_work/hashtable.ll'
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
  %1 = alloca [50 x i64], align 16
  %2 = alloca %struct.timespec, align 8
  %3 = alloca %struct.timespec, align 8
  br label %4

4:                                                ; preds = %0, %4
  %.034 = phi i32 [ 0, %0 ], [ %5, %4 ]
  call void @run_benchmark()
  %5 = add nsw i32 %.034, 1
  %6 = icmp slt i32 %5, 5
  br i1 %6, label %4, label %7, !llvm.loop !6

7:                                                ; preds = %4
  br label %8

8:                                                ; preds = %7, %8
  %.05 = phi i32 [ 0, %7 ], [ %14, %8 ]
  %9 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %2) #4
  call void @run_benchmark()
  %10 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %3) #4
  %11 = call i64 @timespec_diff_ns(ptr noundef %2, ptr noundef %3)
  %12 = sext i32 %.05 to i64
  %13 = getelementptr inbounds [50 x i64], ptr %1, i64 0, i64 %12
  store i64 %11, ptr %13, align 8
  %14 = add nsw i32 %.05, 1
  %15 = icmp slt i32 %14, 50
  br i1 %15, label %8, label %16, !llvm.loop !8

16:                                               ; preds = %8
  %17 = getelementptr inbounds [50 x i64], ptr %1, i64 0, i64 0
  call void @qsort(ptr noundef %17, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %18 = getelementptr inbounds [50 x i64], ptr %1, i64 0, i64 25
  %19 = load i64, ptr %18, align 8
  %20 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %19)
  ret i32 0
}

; Function Attrs: noinline nounwind uwtable
define internal void @run_benchmark() #0 {
  store i32 12345, ptr @lcg_state, align 4
  br label %1

1:                                                ; preds = %0, %1
  %.01213 = phi i32 [ 0, %0 ], [ %14, %1 ]
  %2 = call i32 @lcg_rand()
  %3 = shl i32 %2, 15
  %4 = call i32 @lcg_rand()
  %5 = or i32 %3, %4
  %6 = sext i32 %.01213 to i64
  %7 = getelementptr inbounds [100000 x i32], ptr @keys, i64 0, i64 %6
  store i32 %5, ptr %7, align 4
  %8 = call i32 @lcg_rand()
  %9 = shl i32 %8, 15
  %10 = call i32 @lcg_rand()
  %11 = or i32 %9, %10
  %12 = sext i32 %.01213 to i64
  %13 = getelementptr inbounds [100000 x i32], ptr @values, i64 0, i64 %12
  store i32 %11, ptr %13, align 4
  %14 = add nsw i32 %.01213, 1
  %15 = icmp slt i32 %14, 100000
  br i1 %15, label %1, label %16, !llvm.loop !9

16:                                               ; preds = %1
  call void @ht_clear()
  br label %17

17:                                               ; preds = %16, %17
  %.01114 = phi i32 [ 0, %16 ], [ %24, %17 ]
  %18 = sext i32 %.01114 to i64
  %19 = getelementptr inbounds [100000 x i32], ptr @keys, i64 0, i64 %18
  %20 = load i32, ptr %19, align 4
  %21 = sext i32 %.01114 to i64
  %22 = getelementptr inbounds [100000 x i32], ptr @values, i64 0, i64 %21
  %23 = load i32, ptr %22, align 4
  call void @ht_insert(i32 noundef %20, i32 noundef %23)
  %24 = add nsw i32 %.01114, 1
  %25 = icmp slt i32 %24, 100000
  br i1 %25, label %17, label %26, !llvm.loop !10

26:                                               ; preds = %17
  br label %27

27:                                               ; preds = %26, %27
  %.016 = phi i32 [ 0, %26 ], [ %34, %27 ]
  %.01015 = phi i64 [ 0, %26 ], [ %33, %27 ]
  %28 = sext i32 %.016 to i64
  %29 = getelementptr inbounds [100000 x i32], ptr @keys, i64 0, i64 %28
  %30 = load i32, ptr %29, align 4
  %31 = call i32 @ht_lookup(i32 noundef %30)
  %32 = zext i32 %31 to i64
  %33 = add i64 %.01015, %32
  %34 = add nsw i32 %.016, 1
  %35 = icmp slt i32 %34, 100000
  br i1 %35, label %27, label %36, !llvm.loop !11

36:                                               ; preds = %27
  %.010.lcssa = phi i64 [ %33, %27 ]
  store volatile i64 %.010.lcssa, ptr @sink, align 8
  ret void
}

; Function Attrs: nounwind
declare i32 @clock_gettime(i32 noundef, ptr noundef) #1

; Function Attrs: noinline nounwind uwtable
define internal i64 @timespec_diff_ns(ptr noundef %0, ptr noundef %1) #0 {
  %3 = getelementptr inbounds nuw %struct.timespec, ptr %1, i32 0, i32 0
  %4 = load i64, ptr %3, align 8
  %5 = getelementptr inbounds nuw %struct.timespec, ptr %0, i32 0, i32 0
  %6 = load i64, ptr %5, align 8
  %7 = sub nsw i64 %4, %6
  %8 = mul nsw i64 %7, 1000000000
  %9 = getelementptr inbounds nuw %struct.timespec, ptr %1, i32 0, i32 1
  %10 = load i64, ptr %9, align 8
  %11 = getelementptr inbounds nuw %struct.timespec, ptr %0, i32 0, i32 1
  %12 = load i64, ptr %11, align 8
  %13 = sub nsw i64 %10, %12
  %14 = add nsw i64 %8, %13
  ret i64 %14
}

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #2

; Function Attrs: noinline nounwind uwtable
define internal i32 @cmp_ll(ptr noundef %0, ptr noundef %1) #0 {
  %3 = load i64, ptr %0, align 8
  %4 = load i64, ptr %1, align 8
  %5 = icmp sgt i64 %3, %4
  %6 = zext i1 %5 to i32
  %7 = icmp slt i64 %3, %4
  %8 = zext i1 %7 to i32
  %9 = sub nsw i32 %6, %8
  ret i32 %9
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
  %3 = urem i32 %0, 200003
  %4 = zext i32 %3 to i64
  %5 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %4
  %6 = getelementptr inbounds nuw %struct.Slot, ptr %5, i32 0, i32 2
  %7 = load i8, ptr %6, align 4
  %8 = zext i8 %7 to i32
  %9 = icmp eq i32 %8, 1
  br i1 %9, label %.lr.ph, label %29

.lr.ph:                                           ; preds = %2
  br label %10

10:                                               ; preds = %.lr.ph, %20
  %.013 = phi i32 [ %3, %.lr.ph ], [ %22, %20 ]
  %11 = zext i32 %.013 to i64
  %12 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %11
  %13 = getelementptr inbounds nuw %struct.Slot, ptr %12, i32 0, i32 0
  %14 = load i32, ptr %13, align 4
  %15 = icmp eq i32 %14, %0
  br i1 %15, label %16, label %20

16:                                               ; preds = %10
  %.0.lcssa12 = phi i32 [ %.013, %10 ]
  %17 = zext i32 %.0.lcssa12 to i64
  %18 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %17
  %19 = getelementptr inbounds nuw %struct.Slot, ptr %18, i32 0, i32 1
  store i32 %1, ptr %19, align 4
  br label %39

20:                                               ; preds = %10
  %21 = add i32 %.013, 1
  %22 = urem i32 %21, 200003
  %23 = zext i32 %22 to i64
  %24 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %23
  %25 = getelementptr inbounds nuw %struct.Slot, ptr %24, i32 0, i32 2
  %26 = load i8, ptr %25, align 4
  %27 = zext i8 %26 to i32
  %28 = icmp eq i32 %27, 1
  br i1 %28, label %10, label %._crit_edge, !llvm.loop !12

._crit_edge:                                      ; preds = %20
  %split = phi i32 [ %22, %20 ]
  br label %29

29:                                               ; preds = %._crit_edge, %2
  %.0.lcssa = phi i32 [ %split, %._crit_edge ], [ %3, %2 ]
  %30 = zext i32 %.0.lcssa to i64
  %31 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %30
  %32 = getelementptr inbounds nuw %struct.Slot, ptr %31, i32 0, i32 0
  store i32 %0, ptr %32, align 4
  %33 = zext i32 %.0.lcssa to i64
  %34 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %33
  %35 = getelementptr inbounds nuw %struct.Slot, ptr %34, i32 0, i32 1
  store i32 %1, ptr %35, align 4
  %36 = zext i32 %.0.lcssa to i64
  %37 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %36
  %38 = getelementptr inbounds nuw %struct.Slot, ptr %37, i32 0, i32 2
  store i8 1, ptr %38, align 4
  br label %39

39:                                               ; preds = %29, %16
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal i32 @ht_lookup(i32 noundef %0) #0 {
  %2 = urem i32 %0, 200003
  %3 = zext i32 %2 to i64
  %4 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %3
  %5 = getelementptr inbounds nuw %struct.Slot, ptr %4, i32 0, i32 2
  %6 = load i8, ptr %5, align 4
  %7 = zext i8 %6 to i32
  %8 = icmp eq i32 %7, 1
  br i1 %8, label %.lr.ph, label %29

.lr.ph:                                           ; preds = %1
  br label %9

9:                                                ; preds = %.lr.ph, %20
  %.09 = phi i32 [ %2, %.lr.ph ], [ %22, %20 ]
  %10 = zext i32 %.09 to i64
  %11 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %10
  %12 = getelementptr inbounds nuw %struct.Slot, ptr %11, i32 0, i32 0
  %13 = load i32, ptr %12, align 4
  %14 = icmp eq i32 %13, %0
  br i1 %14, label %15, label %20

15:                                               ; preds = %9
  %.0.lcssa8 = phi i32 [ %.09, %9 ]
  %16 = zext i32 %.0.lcssa8 to i64
  %17 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %16
  %18 = getelementptr inbounds nuw %struct.Slot, ptr %17, i32 0, i32 1
  %19 = load i32, ptr %18, align 4
  br label %30

20:                                               ; preds = %9
  %21 = add i32 %.09, 1
  %22 = urem i32 %21, 200003
  %23 = zext i32 %22 to i64
  %24 = getelementptr inbounds nuw [200003 x %struct.Slot], ptr @table, i64 0, i64 %23
  %25 = getelementptr inbounds nuw %struct.Slot, ptr %24, i32 0, i32 2
  %26 = load i8, ptr %25, align 4
  %27 = zext i8 %26 to i32
  %28 = icmp eq i32 %27, 1
  br i1 %28, label %9, label %._crit_edge, !llvm.loop !13

._crit_edge:                                      ; preds = %20
  br label %29

29:                                               ; preds = %._crit_edge, %1
  br label %30

30:                                               ; preds = %29, %15
  %.07 = phi i32 [ %19, %15 ], [ 0, %29 ]
  ret i32 %.07
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
