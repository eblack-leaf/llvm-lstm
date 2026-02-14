; ModuleID = 'data/exploratory/_work/csv_parser.ll'
source_filename = "benchmarks/csv_parser.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@lcg_state = internal global i32 12345, align 4
@csv_buf = internal global [102401 x i8] zeroinitializer, align 16
@.str.1 = private unnamed_addr constant [8 x i8] c"%d.%02d\00", align 1
@.str.2 = private unnamed_addr constant [3 x i8] c"%d\00", align 1
@csv_len = internal global i32 0, align 4
@total_sum = internal global double 0.000000e+00, align 8

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca [50 x i64], align 16
  %2 = alloca %struct.timespec, align 8
  %3 = alloca %struct.timespec, align 8
  tail call void @generate_csv()
  br label %4

4:                                                ; preds = %6, %0
  %.01 = phi i32 [ 0, %0 ], [ %7, %6 ]
  %5 = icmp samesign ult i32 %.01, 5
  br i1 %5, label %6, label %8

6:                                                ; preds = %4
  tail call void @do_parse()
  %7 = add nuw nsw i32 %.01, 1
  br label %4, !llvm.loop !6

8:                                                ; preds = %4
  br label %9

9:                                                ; preds = %11, %8
  %.0 = phi i32 [ 0, %8 ], [ %17, %11 ]
  %10 = icmp samesign ult i32 %.0, 50
  br i1 %10, label %11, label %18

11:                                               ; preds = %9
  %12 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %2) #5
  call void @do_parse()
  %13 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %3) #5
  %14 = call i64 @timespec_diff_ns(ptr noundef nonnull %2, ptr noundef nonnull %3)
  %15 = zext nneg i32 %.0 to i64
  %16 = getelementptr inbounds nuw [50 x i64], ptr %1, i64 0, i64 %15
  store i64 %14, ptr %16, align 8
  %17 = add nuw nsw i32 %.0, 1
  br label %9, !llvm.loop !8

18:                                               ; preds = %9
  call void @qsort(ptr noundef nonnull %1, i64 noundef 50, i64 noundef 8, ptr noundef nonnull @cmp_ll) #5
  %19 = getelementptr inbounds nuw i8, ptr %1, i64 200
  %20 = load i64, ptr %19, align 8
  %21 = call i32 (ptr, ...) @printf(ptr noundef nonnull dereferenceable(1) @.str, i64 noundef %20) #5
  ret i32 0
}

; Function Attrs: noinline nounwind uwtable
define internal void @generate_csv() #0 {
  %1 = alloca [32 x i8], align 16
  store i32 12345, ptr @lcg_state, align 4
  br label %2

2:                                                ; preds = %53, %0
  %.0 = phi i32 [ 0, %0 ], [ %.4, %53 ]
  %3 = icmp slt i32 %.0, 102200
  br i1 %3, label %4, label %54

4:                                                ; preds = %2
  br label %5

5:                                                ; preds = %.thread, %4
  %.04 = phi i32 [ 0, %4 ], [ %46, %.thread ]
  %.1 = phi i32 [ %.0, %4 ], [ %.3, %.thread ]
  %6 = icmp samesign ult i32 %.04, 10
  br i1 %6, label %7, label %47

7:                                                ; preds = %5
  %8 = icmp ne i32 %.04, 0
  %9 = icmp slt i32 %.1, 102400
  %or.cond = select i1 %8, i1 %9, i1 false
  br i1 %or.cond, label %10, label %14

10:                                               ; preds = %7
  %11 = add nsw i32 %.1, 1
  %12 = sext i32 %.1 to i64
  %13 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %12
  store i8 44, ptr %13, align 1
  br label %14

14:                                               ; preds = %10, %7
  %.2 = phi i32 [ %11, %10 ], [ %.1, %7 ]
  %15 = call i32 @lcg_rand()
  %16 = urem i32 %15, 3
  %17 = icmp eq i32 %16, 0
  br i1 %17, label %18, label %24

18:                                               ; preds = %14
  %19 = call i32 @lcg_rand()
  %20 = urem i32 %19, 10000
  %21 = call i32 @lcg_rand()
  %22 = urem i32 %21, 100
  %23 = call i32 (ptr, ptr, ...) @sprintf(ptr noundef nonnull dereferenceable(1) %1, ptr noundef nonnull dereferenceable(1) @.str.1, i32 noundef %20, i32 noundef %22) #5
  br label %34

24:                                               ; preds = %14
  %25 = call i32 @lcg_rand()
  %26 = urem i32 %25, 100000
  %27 = call i32 @lcg_rand()
  %28 = and i32 %27, 3
  %29 = icmp eq i32 %28, 0
  br i1 %29, label %30, label %32

30:                                               ; preds = %24
  %31 = sub nsw i32 0, %26
  br label %32

32:                                               ; preds = %30, %24
  %.07 = phi i32 [ %31, %30 ], [ %26, %24 ]
  %33 = call i32 (ptr, ptr, ...) @sprintf(ptr noundef nonnull dereferenceable(1) %1, ptr noundef nonnull dereferenceable(1) @.str.2, i32 noundef %.07) #5
  br label %34

34:                                               ; preds = %32, %18
  %.05 = phi i32 [ %23, %18 ], [ %33, %32 ]
  br label %35

35:                                               ; preds = %38, %34
  %.06 = phi i32 [ 0, %34 ], [ %45, %38 ]
  %.3 = phi i32 [ %.2, %34 ], [ %42, %38 ]
  %36 = icmp slt i32 %.06, %.05
  %37 = icmp slt i32 %.3, 102400
  %or.cond3 = select i1 %36, i1 %37, i1 false
  br i1 %or.cond3, label %38, label %.thread

38:                                               ; preds = %35
  %39 = zext nneg i32 %.06 to i64
  %40 = getelementptr inbounds nuw [32 x i8], ptr %1, i64 0, i64 %39
  %41 = load i8, ptr %40, align 1
  %42 = add nsw i32 %.3, 1
  %43 = sext i32 %.3 to i64
  %44 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %43
  store i8 %41, ptr %44, align 1
  %45 = add nuw nsw i32 %.06, 1
  br label %35, !llvm.loop !9

.thread:                                          ; preds = %35
  %46 = add nuw nsw i32 %.04, 1
  br label %5, !llvm.loop !10

47:                                               ; preds = %5
  %48 = icmp slt i32 %.1, 102400
  br i1 %48, label %49, label %53

49:                                               ; preds = %47
  %50 = add nsw i32 %.1, 1
  %51 = sext i32 %.1 to i64
  %52 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %51
  store i8 10, ptr %52, align 1
  br label %53

53:                                               ; preds = %49, %47
  %.4 = phi i32 [ %50, %49 ], [ %.1, %47 ]
  br label %2, !llvm.loop !11

54:                                               ; preds = %2
  %55 = zext nneg i32 %.0 to i64
  %56 = getelementptr inbounds nuw [102401 x i8], ptr @csv_buf, i64 0, i64 %55
  store i8 0, ptr %56, align 1
  store i32 %.0, ptr @csv_len, align 4
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal void @do_parse() #0 {
  %1 = alloca [32 x i8], align 16
  br label %2

2:                                                ; preds = %50, %0
  %.02 = phi i32 [ 0, %0 ], [ %.2, %50 ]
  %.0 = phi double [ 0.000000e+00, %0 ], [ %.1, %50 ]
  %3 = load i32, ptr @csv_len, align 4
  %4 = icmp slt i32 %.02, %3
  br i1 %4, label %5, label %51

5:                                                ; preds = %2
  br label %6

6:                                                ; preds = %21, %5
  %.04 = phi i32 [ 0, %5 ], [ %26, %21 ]
  %.13 = phi i32 [ %.02, %5 ], [ %22, %21 ]
  %7 = load i32, ptr @csv_len, align 4
  %8 = icmp slt i32 %.13, %7
  br i1 %8, label %9, label %29

9:                                                ; preds = %6
  %10 = sext i32 %.13 to i64
  %11 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %10
  %12 = load i8, ptr %11, align 1
  %.not = icmp eq i8 %12, 44
  br i1 %.not, label %29, label %13

13:                                               ; preds = %9
  %14 = sext i32 %.13 to i64
  %15 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %14
  %16 = load i8, ptr %15, align 1
  %.not5 = icmp eq i8 %16, 10
  br i1 %.not5, label %29, label %17

17:                                               ; preds = %13
  %18 = icmp samesign ult i32 %.04, 31
  br i1 %18, label %21, label %.thread1

.thread1:                                         ; preds = %17
  %19 = zext nneg i32 %.04 to i64
  %20 = getelementptr inbounds nuw [32 x i8], ptr %1, i64 0, i64 %19
  store i8 0, ptr %20, align 1
  br label %32

21:                                               ; preds = %17
  %22 = add nsw i32 %.13, 1
  %23 = sext i32 %.13 to i64
  %24 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %23
  %25 = load i8, ptr %24, align 1
  %26 = add nuw nsw i32 %.04, 1
  %27 = zext nneg i32 %.04 to i64
  %28 = getelementptr inbounds nuw [32 x i8], ptr %1, i64 0, i64 %27
  store i8 %25, ptr %28, align 1
  br label %6, !llvm.loop !12

29:                                               ; preds = %13, %9, %6
  %30 = zext nneg i32 %.04 to i64
  %31 = getelementptr inbounds nuw [32 x i8], ptr %1, i64 0, i64 %30
  store i8 0, ptr %31, align 1
  %.not6 = icmp eq i32 %.04, 0
  br i1 %.not6, label %35, label %32

32:                                               ; preds = %.thread1, %29
  %33 = call double @atof(ptr noundef nonnull %1) #6
  %34 = fadd double %.0, %33
  br label %35

35:                                               ; preds = %32, %29
  %.1 = phi double [ %34, %32 ], [ %.0, %29 ]
  %36 = load i32, ptr @csv_len, align 4
  %37 = icmp slt i32 %.13, %36
  br i1 %37, label %38, label %50

38:                                               ; preds = %35
  %39 = sext i32 %.13 to i64
  %40 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %39
  %41 = load i8, ptr %40, align 1
  %42 = icmp eq i8 %41, 44
  br i1 %42, label %48, label %43

43:                                               ; preds = %38
  %44 = sext i32 %.13 to i64
  %45 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %44
  %46 = load i8, ptr %45, align 1
  %47 = icmp eq i8 %46, 10
  br i1 %47, label %48, label %50

48:                                               ; preds = %43, %38
  %49 = add nsw i32 %.13, 1
  br label %50

50:                                               ; preds = %48, %43, %35
  %.2 = phi i32 [ %49, %48 ], [ %.13, %43 ], [ %.13, %35 ]
  br label %2, !llvm.loop !13

51:                                               ; preds = %2
  store volatile double %.0, ptr @total_sum, align 8
  ret void
}

; Function Attrs: nounwind
declare i32 @clock_gettime(i32 noundef, ptr noundef) #1

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

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #2

; Function Attrs: noinline nounwind uwtable
define internal i32 @cmp_ll(ptr noundef %0, ptr noundef %1) #0 {
  %3 = load i64, ptr %0, align 8
  %4 = load i64, ptr %1, align 8
  %5 = tail call i32 @llvm.scmp.i32.i64(i64 %3, i64 %4)
  ret i32 %5
}

declare i32 @printf(ptr noundef, ...) #2

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

; Function Attrs: nounwind
declare i32 @sprintf(ptr noundef, ptr noundef, ...) #1

; Function Attrs: nounwind willreturn memory(read)
declare double @atof(ptr noundef) #3

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.scmp.i32.i64(i64, i64) #4

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nounwind willreturn memory(read) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #5 = { nounwind }
attributes #6 = { nounwind willreturn memory(read) }

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
