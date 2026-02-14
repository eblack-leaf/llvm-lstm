; ModuleID = 'data/exploratory/_work/kmp_search.ll'
source_filename = "benchmarks/kmp_search.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@text = internal global ptr null, align 8
@stderr = external global ptr, align 8
@.str = private unnamed_addr constant [15 x i8] c"malloc failed\0A\00", align 1
@lcg_state = internal global i32 12345, align 4
@pattern = internal global [21 x i8] zeroinitializer, align 16
@fail_table = internal global [20 x i32] zeroinitializer, align 16
@.str.1 = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@match_count = internal global i32 0, align 4

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca [50 x i64], align 16
  %2 = alloca %struct.timespec, align 8
  %3 = alloca %struct.timespec, align 8
  %4 = call noalias dereferenceable_or_null(10485761) ptr @malloc(i64 noundef 10485761) #7
  store ptr %4, ptr @text, align 8
  %.not = icmp eq ptr %4, null
  br i1 %.not, label %5, label %8

5:                                                ; preds = %0
  %6 = load ptr, ptr @stderr, align 8
  %7 = call i64 @fwrite(ptr nonnull @.str, i64 14, i64 1, ptr %6) #8
  br label %39

8:                                                ; preds = %0
  store i32 12345, ptr @lcg_state, align 4
  br i1 true, label %.lr.ph, label %._crit_edge

.lr.ph:                                           ; preds = %8, %.lr.ph
  %.015 = phi i32 [ 0, %8 ], [ %16, %.lr.ph ]
  %9 = call i32 @lcg_rand()
  %10 = urem i32 %9, 26
  %11 = trunc nuw nsw i32 %10 to i8
  %12 = add nuw i8 %11, 97
  %13 = load ptr, ptr @text, align 8
  %14 = sext i32 %.015 to i64
  %15 = getelementptr inbounds i8, ptr %13, i64 %14
  store i8 %12, ptr %15, align 1
  %16 = add nsw i32 %.015, 1
  %17 = icmp slt i32 %.015, 10485759
  br i1 %17, label %.lr.ph, label %._crit_edge, !llvm.loop !6

._crit_edge:                                      ; preds = %.lr.ph, %8
  %18 = load ptr, ptr @text, align 8
  %19 = getelementptr inbounds nuw i8, ptr %18, i64 10485760
  store i8 0, ptr %19, align 1
  %20 = getelementptr inbounds nuw i8, ptr %18, i64 1000
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(20) @pattern, ptr noundef nonnull align 1 dereferenceable(20) %20, i64 20, i1 false)
  store i8 0, ptr getelementptr inbounds nuw (i8, ptr @pattern, i64 20), align 4
  br i1 true, label %.lr.ph2, label %._crit_edge3

.lr.ph2:                                          ; preds = %._crit_edge, %.lr.ph2
  %.014 = phi i32 [ 0, %._crit_edge ], [ %24, %.lr.ph2 ]
  %21 = load ptr, ptr @text, align 8
  %22 = sext i32 %.014 to i64
  %23 = getelementptr inbounds i8, ptr %21, i64 %22
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(20) %23, ptr noundef nonnull align 16 dereferenceable(20) @pattern, i64 20, i1 false)
  %24 = add nsw i32 %.014, 50000
  %25 = icmp slt i32 %.014, 10435740
  br i1 %25, label %.lr.ph2, label %._crit_edge3, !llvm.loop !8

._crit_edge3:                                     ; preds = %.lr.ph2, %._crit_edge
  call void @build_fail(ptr noundef nonnull @pattern, i32 noundef 20, ptr noundef nonnull @fail_table)
  br i1 true, label %.lr.ph5, label %._crit_edge6

.lr.ph5:                                          ; preds = %._crit_edge3, %.lr.ph5
  %.013 = phi i32 [ 0, %._crit_edge3 ], [ %26, %.lr.ph5 ]
  call void @do_kmp()
  %26 = add nsw i32 %.013, 1
  %27 = icmp slt i32 %.013, 4
  br i1 %27, label %.lr.ph5, label %._crit_edge6, !llvm.loop !9

._crit_edge6:                                     ; preds = %.lr.ph5, %._crit_edge3
  br i1 true, label %.lr.ph8, label %._crit_edge9

.lr.ph8:                                          ; preds = %._crit_edge6, %.lr.ph8
  %.0 = phi i32 [ 0, %._crit_edge6 ], [ %33, %.lr.ph8 ]
  %28 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %2) #9
  call void @do_kmp()
  %29 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %3) #9
  %30 = call i64 @timespec_diff_ns(ptr noundef nonnull %2, ptr noundef nonnull %3)
  %31 = sext i32 %.0 to i64
  %32 = getelementptr inbounds [50 x i64], ptr %1, i64 0, i64 %31
  store i64 %30, ptr %32, align 8
  %33 = add nsw i32 %.0, 1
  %34 = icmp slt i32 %.0, 49
  br i1 %34, label %.lr.ph8, label %._crit_edge9, !llvm.loop !10

._crit_edge9:                                     ; preds = %.lr.ph8, %._crit_edge6
  call void @qsort(ptr noundef nonnull %1, i64 noundef 50, i64 noundef 8, ptr noundef nonnull @cmp_ll) #9
  %35 = getelementptr inbounds nuw i8, ptr %1, i64 200
  %36 = load i64, ptr %35, align 8
  %37 = call i32 (ptr, ...) @printf(ptr noundef nonnull dereferenceable(1) @.str.1, i64 noundef %36) #9
  %38 = load ptr, ptr @text, align 8
  call void @free(ptr noundef %38) #9
  br label %39

39:                                               ; preds = %._crit_edge9, %5
  %storemerge = phi i32 [ 1, %5 ], [ 0, %._crit_edge9 ]
  ret i32 %storemerge
}

; Function Attrs: nounwind allocsize(0)
declare noalias ptr @malloc(i64 noundef) #1

; Function Attrs: nounwind
declare i32 @fprintf(ptr noundef, ptr noundef, ...) #2

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

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #3

; Function Attrs: noinline nounwind uwtable
define internal void @build_fail(ptr noundef %0, i32 noundef %1, ptr noundef %2) #0 {
  store i32 0, ptr %2, align 4
  %4 = icmp sgt i32 %1, 1
  br i1 %4, label %.lr.ph, label %._crit_edge

.lr.ph:                                           ; preds = %3, %28
  %.013 = phi i32 [ 0, %3 ], [ %.2, %28 ]
  %.0 = phi i32 [ 1, %3 ], [ %31, %28 ]
  br label %5

5:                                                ; preds = %14, %.lr.ph
  %.1 = phi i32 [ %.013, %.lr.ph ], [ %18, %14 ]
  %6 = icmp sgt i32 %.1, 0
  br i1 %6, label %7, label %.critedge

7:                                                ; preds = %5
  %8 = sext i32 %.1 to i64
  %9 = getelementptr inbounds i8, ptr %0, i64 %8
  %10 = load i8, ptr %9, align 1
  %11 = sext i32 %.0 to i64
  %12 = getelementptr inbounds i8, ptr %0, i64 %11
  %13 = load i8, ptr %12, align 1
  %.not = icmp eq i8 %10, %13
  br i1 %.not, label %.critedge, label %14

14:                                               ; preds = %7
  %15 = sext i32 %.1 to i64
  %16 = getelementptr i32, ptr %2, i64 %15
  %17 = getelementptr i8, ptr %16, i64 -4
  %18 = load i32, ptr %17, align 4
  br label %5, !llvm.loop !11

.critedge:                                        ; preds = %5, %7
  %19 = sext i32 %.1 to i64
  %20 = getelementptr inbounds i8, ptr %0, i64 %19
  %21 = load i8, ptr %20, align 1
  %22 = sext i32 %.0 to i64
  %23 = getelementptr inbounds i8, ptr %0, i64 %22
  %24 = load i8, ptr %23, align 1
  %25 = icmp eq i8 %21, %24
  br i1 %25, label %26, label %28

26:                                               ; preds = %.critedge
  %27 = add nsw i32 %.1, 1
  br label %28

28:                                               ; preds = %26, %.critedge
  %.2 = phi i32 [ %27, %26 ], [ %.1, %.critedge ]
  %29 = sext i32 %.0 to i64
  %30 = getelementptr inbounds i32, ptr %2, i64 %29
  store i32 %.2, ptr %30, align 4
  %31 = add nsw i32 %.0, 1
  %32 = icmp slt i32 %31, %1
  br i1 %32, label %.lr.ph, label %._crit_edge, !llvm.loop !12

._crit_edge:                                      ; preds = %28, %3
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal void @do_kmp() #0 {
  %1 = load ptr, ptr @text, align 8
  %2 = call i32 @kmp_count(ptr noundef %1, i32 noundef 10485760, ptr noundef nonnull @pattern, i32 noundef 20, ptr noundef nonnull @fail_table)
  store volatile i32 %2, ptr @match_count, align 4
  ret void
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

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #4

; Function Attrs: noinline nounwind uwtable
define internal i32 @cmp_ll(ptr noundef %0, ptr noundef %1) #0 {
  %3 = load i64, ptr %0, align 8
  %4 = load i64, ptr %1, align 8
  %5 = call i32 @llvm.scmp.i32.i64(i64 %3, i64 %4)
  ret i32 %5
}

declare i32 @printf(ptr noundef, ...) #4

; Function Attrs: nounwind
declare void @free(ptr noundef) #2

; Function Attrs: noinline nounwind uwtable
define internal i32 @kmp_count(ptr noundef %0, i32 noundef %1, ptr noundef %2, i32 noundef %3, ptr noundef %4) #0 {
  %6 = icmp sgt i32 %1, 0
  br i1 %6, label %.lr.ph, label %._crit_edge

.lr.ph:                                           ; preds = %5, %38
  %.021 = phi i32 [ 0, %5 ], [ %.223, %38 ]
  %.020 = phi i32 [ 0, %5 ], [ %.3, %38 ]
  %.0 = phi i32 [ 0, %5 ], [ %39, %38 ]
  br label %7

7:                                                ; preds = %16, %.lr.ph
  %.1 = phi i32 [ %.020, %.lr.ph ], [ %20, %16 ]
  %8 = icmp sgt i32 %.1, 0
  br i1 %8, label %9, label %.critedge

9:                                                ; preds = %7
  %10 = sext i32 %.1 to i64
  %11 = getelementptr inbounds i8, ptr %2, i64 %10
  %12 = load i8, ptr %11, align 1
  %13 = sext i32 %.0 to i64
  %14 = getelementptr inbounds i8, ptr %0, i64 %13
  %15 = load i8, ptr %14, align 1
  %.not = icmp eq i8 %12, %15
  br i1 %.not, label %.critedge, label %16

16:                                               ; preds = %9
  %17 = sext i32 %.1 to i64
  %18 = getelementptr i32, ptr %4, i64 %17
  %19 = getelementptr i8, ptr %18, i64 -4
  %20 = load i32, ptr %19, align 4
  br label %7, !llvm.loop !13

.critedge:                                        ; preds = %7, %9
  %21 = sext i32 %.1 to i64
  %22 = getelementptr inbounds i8, ptr %2, i64 %21
  %23 = load i8, ptr %22, align 1
  %24 = sext i32 %.0 to i64
  %25 = getelementptr inbounds i8, ptr %0, i64 %24
  %26 = load i8, ptr %25, align 1
  %27 = icmp eq i8 %23, %26
  br i1 %27, label %28, label %30

28:                                               ; preds = %.critedge
  %29 = add nsw i32 %.1, 1
  br label %30

30:                                               ; preds = %28, %.critedge
  %.2 = phi i32 [ %29, %28 ], [ %.1, %.critedge ]
  %31 = icmp eq i32 %.2, %3
  br i1 %31, label %32, label %38

32:                                               ; preds = %30
  %33 = add nsw i32 %.021, 1
  %34 = sext i32 %.2 to i64
  %35 = getelementptr i32, ptr %4, i64 %34
  %36 = getelementptr i8, ptr %35, i64 -4
  %37 = load i32, ptr %36, align 4
  br label %38

38:                                               ; preds = %30, %32
  %.223 = phi i32 [ %33, %32 ], [ %.021, %30 ]
  %.3 = phi i32 [ %37, %32 ], [ %.2, %30 ]
  %39 = add nsw i32 %.0, 1
  %40 = icmp slt i32 %39, %1
  br i1 %40, label %.lr.ph, label %._crit_edge, !llvm.loop !14

._crit_edge:                                      ; preds = %38, %5
  %.122 = phi i32 [ %.223, %38 ], [ 0, %5 ]
  ret i32 %.122
}

; Function Attrs: nofree nounwind
declare noundef i64 @fwrite(ptr nocapture noundef, i64 noundef, i64 noundef, ptr nocapture noundef) #5

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.scmp.i32.i64(i64, i64) #6

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind allocsize(0) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #4 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #5 = { nofree nounwind }
attributes #6 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #7 = { nounwind allocsize(0) }
attributes #8 = { cold }
attributes #9 = { nounwind }

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
!14 = distinct !{!14, !7}
