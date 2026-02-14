; ModuleID = 'benchmarks/kmp_search.c'
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
  %1 = alloca i32, align 4
  %2 = alloca i32, align 4
  %3 = alloca i32, align 4
  %4 = alloca i32, align 4
  %5 = alloca [50 x i64], align 16
  %6 = alloca i32, align 4
  %7 = alloca %struct.timespec, align 8
  %8 = alloca %struct.timespec, align 8
  store i32 0, ptr %1, align 4
  %9 = call noalias ptr @malloc(i64 noundef 10485761) #5
  store ptr %9, ptr @text, align 8
  %10 = load ptr, ptr @text, align 8
  %11 = icmp ne ptr %10, null
  br i1 %11, label %15, label %12

12:                                               ; preds = %0
  %13 = load ptr, ptr @stderr, align 8
  %14 = call i32 (ptr, ptr, ...) @fprintf(ptr noundef %13, ptr noundef @.str) #6
  store i32 1, ptr %1, align 4
  br label %77

15:                                               ; preds = %0
  store i32 12345, ptr @lcg_state, align 4
  store i32 0, ptr %2, align 4
  br label %16

16:                                               ; preds = %30, %15
  %17 = load i32, ptr %2, align 4
  %18 = icmp slt i32 %17, 10485760
  br i1 %18, label %19, label %33

19:                                               ; preds = %16
  %20 = call i32 @lcg_rand()
  %21 = urem i32 %20, 26
  %22 = trunc i32 %21 to i8
  %23 = sext i8 %22 to i32
  %24 = add nsw i32 97, %23
  %25 = trunc i32 %24 to i8
  %26 = load ptr, ptr @text, align 8
  %27 = load i32, ptr %2, align 4
  %28 = sext i32 %27 to i64
  %29 = getelementptr inbounds i8, ptr %26, i64 %28
  store i8 %25, ptr %29, align 1
  br label %30

30:                                               ; preds = %19
  %31 = load i32, ptr %2, align 4
  %32 = add nsw i32 %31, 1
  store i32 %32, ptr %2, align 4
  br label %16, !llvm.loop !6

33:                                               ; preds = %16
  %34 = load ptr, ptr @text, align 8
  %35 = getelementptr inbounds i8, ptr %34, i64 10485760
  store i8 0, ptr %35, align 1
  %36 = load ptr, ptr @text, align 8
  %37 = getelementptr inbounds i8, ptr %36, i64 1000
  call void @llvm.memcpy.p0.p0.i64(ptr align 16 @pattern, ptr align 1 %37, i64 20, i1 false)
  store i8 0, ptr getelementptr inbounds ([21 x i8], ptr @pattern, i64 0, i64 20), align 4
  store i32 0, ptr %3, align 4
  br label %38

38:                                               ; preds = %46, %33
  %39 = load i32, ptr %3, align 4
  %40 = icmp slt i32 %39, 10485740
  br i1 %40, label %41, label %49

41:                                               ; preds = %38
  %42 = load ptr, ptr @text, align 8
  %43 = load i32, ptr %3, align 4
  %44 = sext i32 %43 to i64
  %45 = getelementptr inbounds i8, ptr %42, i64 %44
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %45, ptr align 16 @pattern, i64 20, i1 false)
  br label %46

46:                                               ; preds = %41
  %47 = load i32, ptr %3, align 4
  %48 = add nsw i32 %47, 50000
  store i32 %48, ptr %3, align 4
  br label %38, !llvm.loop !8

49:                                               ; preds = %38
  call void @build_fail(ptr noundef @pattern, i32 noundef 20, ptr noundef @fail_table)
  store i32 0, ptr %4, align 4
  br label %50

50:                                               ; preds = %54, %49
  %51 = load i32, ptr %4, align 4
  %52 = icmp slt i32 %51, 5
  br i1 %52, label %53, label %57

53:                                               ; preds = %50
  call void @do_kmp()
  br label %54

54:                                               ; preds = %53
  %55 = load i32, ptr %4, align 4
  %56 = add nsw i32 %55, 1
  store i32 %56, ptr %4, align 4
  br label %50, !llvm.loop !9

57:                                               ; preds = %50
  store i32 0, ptr %6, align 4
  br label %58

58:                                               ; preds = %68, %57
  %59 = load i32, ptr %6, align 4
  %60 = icmp slt i32 %59, 50
  br i1 %60, label %61, label %71

61:                                               ; preds = %58
  %62 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %7) #6
  call void @do_kmp()
  %63 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %8) #6
  %64 = call i64 @timespec_diff_ns(ptr noundef %7, ptr noundef %8)
  %65 = load i32, ptr %6, align 4
  %66 = sext i32 %65 to i64
  %67 = getelementptr inbounds [50 x i64], ptr %5, i64 0, i64 %66
  store i64 %64, ptr %67, align 8
  br label %68

68:                                               ; preds = %61
  %69 = load i32, ptr %6, align 4
  %70 = add nsw i32 %69, 1
  store i32 %70, ptr %6, align 4
  br label %58, !llvm.loop !10

71:                                               ; preds = %58
  %72 = getelementptr inbounds [50 x i64], ptr %5, i64 0, i64 0
  call void @qsort(ptr noundef %72, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %73 = getelementptr inbounds [50 x i64], ptr %5, i64 0, i64 25
  %74 = load i64, ptr %73, align 8
  %75 = call i32 (ptr, ...) @printf(ptr noundef @.str.1, i64 noundef %74)
  %76 = load ptr, ptr @text, align 8
  call void @free(ptr noundef %76) #6
  store i32 0, ptr %1, align 4
  br label %77

77:                                               ; preds = %71, %12
  %78 = load i32, ptr %1, align 4
  ret i32 %78
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
  %4 = load i32, ptr @lcg_state, align 4
  %5 = lshr i32 %4, 16
  %6 = and i32 %5, 32767
  ret i32 %6
}

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #3

; Function Attrs: noinline nounwind uwtable
define internal void @build_fail(ptr noundef %0, i32 noundef %1, ptr noundef %2) #0 {
  %4 = alloca ptr, align 8
  %5 = alloca i32, align 4
  %6 = alloca ptr, align 8
  %7 = alloca i32, align 4
  %8 = alloca i32, align 4
  store ptr %0, ptr %4, align 8
  store i32 %1, ptr %5, align 4
  store ptr %2, ptr %6, align 8
  %9 = load ptr, ptr %6, align 8
  %10 = getelementptr inbounds i32, ptr %9, i64 0
  store i32 0, ptr %10, align 4
  store i32 0, ptr %7, align 4
  store i32 1, ptr %8, align 4
  br label %11

11:                                               ; preds = %65, %3
  %12 = load i32, ptr %8, align 4
  %13 = load i32, ptr %5, align 4
  %14 = icmp slt i32 %12, %13
  br i1 %14, label %15, label %68

15:                                               ; preds = %11
  br label %16

16:                                               ; preds = %35, %15
  %17 = load i32, ptr %7, align 4
  %18 = icmp sgt i32 %17, 0
  br i1 %18, label %19, label %33

19:                                               ; preds = %16
  %20 = load ptr, ptr %4, align 8
  %21 = load i32, ptr %7, align 4
  %22 = sext i32 %21 to i64
  %23 = getelementptr inbounds i8, ptr %20, i64 %22
  %24 = load i8, ptr %23, align 1
  %25 = sext i8 %24 to i32
  %26 = load ptr, ptr %4, align 8
  %27 = load i32, ptr %8, align 4
  %28 = sext i32 %27 to i64
  %29 = getelementptr inbounds i8, ptr %26, i64 %28
  %30 = load i8, ptr %29, align 1
  %31 = sext i8 %30 to i32
  %32 = icmp ne i32 %25, %31
  br label %33

33:                                               ; preds = %19, %16
  %34 = phi i1 [ false, %16 ], [ %32, %19 ]
  br i1 %34, label %35, label %42

35:                                               ; preds = %33
  %36 = load ptr, ptr %6, align 8
  %37 = load i32, ptr %7, align 4
  %38 = sub nsw i32 %37, 1
  %39 = sext i32 %38 to i64
  %40 = getelementptr inbounds i32, ptr %36, i64 %39
  %41 = load i32, ptr %40, align 4
  store i32 %41, ptr %7, align 4
  br label %16, !llvm.loop !11

42:                                               ; preds = %33
  %43 = load ptr, ptr %4, align 8
  %44 = load i32, ptr %7, align 4
  %45 = sext i32 %44 to i64
  %46 = getelementptr inbounds i8, ptr %43, i64 %45
  %47 = load i8, ptr %46, align 1
  %48 = sext i8 %47 to i32
  %49 = load ptr, ptr %4, align 8
  %50 = load i32, ptr %8, align 4
  %51 = sext i32 %50 to i64
  %52 = getelementptr inbounds i8, ptr %49, i64 %51
  %53 = load i8, ptr %52, align 1
  %54 = sext i8 %53 to i32
  %55 = icmp eq i32 %48, %54
  br i1 %55, label %56, label %59

56:                                               ; preds = %42
  %57 = load i32, ptr %7, align 4
  %58 = add nsw i32 %57, 1
  store i32 %58, ptr %7, align 4
  br label %59

59:                                               ; preds = %56, %42
  %60 = load i32, ptr %7, align 4
  %61 = load ptr, ptr %6, align 8
  %62 = load i32, ptr %8, align 4
  %63 = sext i32 %62 to i64
  %64 = getelementptr inbounds i32, ptr %61, i64 %63
  store i32 %60, ptr %64, align 4
  br label %65

65:                                               ; preds = %59
  %66 = load i32, ptr %8, align 4
  %67 = add nsw i32 %66, 1
  store i32 %67, ptr %8, align 4
  br label %11, !llvm.loop !12

68:                                               ; preds = %11
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal void @do_kmp() #0 {
  %1 = load ptr, ptr @text, align 8
  %2 = call i32 @kmp_count(ptr noundef %1, i32 noundef 10485760, ptr noundef @pattern, i32 noundef 20, ptr noundef @fail_table)
  store volatile i32 %2, ptr @match_count, align 4
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

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #4

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

declare i32 @printf(ptr noundef, ...) #4

; Function Attrs: nounwind
declare void @free(ptr noundef) #2

; Function Attrs: noinline nounwind uwtable
define internal i32 @kmp_count(ptr noundef %0, i32 noundef %1, ptr noundef %2, i32 noundef %3, ptr noundef %4) #0 {
  %6 = alloca ptr, align 8
  %7 = alloca i32, align 4
  %8 = alloca ptr, align 8
  %9 = alloca i32, align 4
  %10 = alloca ptr, align 8
  %11 = alloca i32, align 4
  %12 = alloca i32, align 4
  %13 = alloca i32, align 4
  store ptr %0, ptr %6, align 8
  store i32 %1, ptr %7, align 4
  store ptr %2, ptr %8, align 8
  store i32 %3, ptr %9, align 4
  store ptr %4, ptr %10, align 8
  store i32 0, ptr %11, align 4
  store i32 0, ptr %12, align 4
  store i32 0, ptr %13, align 4
  br label %14

14:                                               ; preds = %76, %5
  %15 = load i32, ptr %13, align 4
  %16 = load i32, ptr %7, align 4
  %17 = icmp slt i32 %15, %16
  br i1 %17, label %18, label %79

18:                                               ; preds = %14
  br label %19

19:                                               ; preds = %38, %18
  %20 = load i32, ptr %12, align 4
  %21 = icmp sgt i32 %20, 0
  br i1 %21, label %22, label %36

22:                                               ; preds = %19
  %23 = load ptr, ptr %8, align 8
  %24 = load i32, ptr %12, align 4
  %25 = sext i32 %24 to i64
  %26 = getelementptr inbounds i8, ptr %23, i64 %25
  %27 = load i8, ptr %26, align 1
  %28 = sext i8 %27 to i32
  %29 = load ptr, ptr %6, align 8
  %30 = load i32, ptr %13, align 4
  %31 = sext i32 %30 to i64
  %32 = getelementptr inbounds i8, ptr %29, i64 %31
  %33 = load i8, ptr %32, align 1
  %34 = sext i8 %33 to i32
  %35 = icmp ne i32 %28, %34
  br label %36

36:                                               ; preds = %22, %19
  %37 = phi i1 [ false, %19 ], [ %35, %22 ]
  br i1 %37, label %38, label %45

38:                                               ; preds = %36
  %39 = load ptr, ptr %10, align 8
  %40 = load i32, ptr %12, align 4
  %41 = sub nsw i32 %40, 1
  %42 = sext i32 %41 to i64
  %43 = getelementptr inbounds i32, ptr %39, i64 %42
  %44 = load i32, ptr %43, align 4
  store i32 %44, ptr %12, align 4
  br label %19, !llvm.loop !13

45:                                               ; preds = %36
  %46 = load ptr, ptr %8, align 8
  %47 = load i32, ptr %12, align 4
  %48 = sext i32 %47 to i64
  %49 = getelementptr inbounds i8, ptr %46, i64 %48
  %50 = load i8, ptr %49, align 1
  %51 = sext i8 %50 to i32
  %52 = load ptr, ptr %6, align 8
  %53 = load i32, ptr %13, align 4
  %54 = sext i32 %53 to i64
  %55 = getelementptr inbounds i8, ptr %52, i64 %54
  %56 = load i8, ptr %55, align 1
  %57 = sext i8 %56 to i32
  %58 = icmp eq i32 %51, %57
  br i1 %58, label %59, label %62

59:                                               ; preds = %45
  %60 = load i32, ptr %12, align 4
  %61 = add nsw i32 %60, 1
  store i32 %61, ptr %12, align 4
  br label %62

62:                                               ; preds = %59, %45
  %63 = load i32, ptr %12, align 4
  %64 = load i32, ptr %9, align 4
  %65 = icmp eq i32 %63, %64
  br i1 %65, label %66, label %75

66:                                               ; preds = %62
  %67 = load i32, ptr %11, align 4
  %68 = add nsw i32 %67, 1
  store i32 %68, ptr %11, align 4
  %69 = load ptr, ptr %10, align 8
  %70 = load i32, ptr %12, align 4
  %71 = sub nsw i32 %70, 1
  %72 = sext i32 %71 to i64
  %73 = getelementptr inbounds i32, ptr %69, i64 %72
  %74 = load i32, ptr %73, align 4
  store i32 %74, ptr %12, align 4
  br label %75

75:                                               ; preds = %66, %62
  br label %76

76:                                               ; preds = %75
  %77 = load i32, ptr %13, align 4
  %78 = add nsw i32 %77, 1
  store i32 %78, ptr %13, align 4
  br label %14, !llvm.loop !14

79:                                               ; preds = %14
  %80 = load i32, ptr %11, align 4
  ret i32 %80
}

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind allocsize(0) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #4 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #5 = { nounwind allocsize(0) }
attributes #6 = { nounwind }

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
