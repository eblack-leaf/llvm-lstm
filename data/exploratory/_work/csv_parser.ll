; ModuleID = 'benchmarks/csv_parser.c'
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
  %1 = alloca i32, align 4
  %2 = alloca i32, align 4
  %3 = alloca [50 x i64], align 16
  %4 = alloca i32, align 4
  %5 = alloca %struct.timespec, align 8
  %6 = alloca %struct.timespec, align 8
  store i32 0, ptr %1, align 4
  call void @generate_csv()
  store i32 0, ptr %2, align 4
  br label %7

7:                                                ; preds = %11, %0
  %8 = load i32, ptr %2, align 4
  %9 = icmp slt i32 %8, 5
  br i1 %9, label %10, label %14

10:                                               ; preds = %7
  call void @do_parse()
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
  call void @do_parse()
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
define internal void @generate_csv() #0 {
  %1 = alloca i32, align 4
  %2 = alloca i32, align 4
  %3 = alloca [32 x i8], align 16
  %4 = alloca i32, align 4
  %5 = alloca i32, align 4
  %6 = alloca i32, align 4
  %7 = alloca i32, align 4
  %8 = alloca i32, align 4
  store i32 0, ptr %1, align 4
  store i32 12345, ptr @lcg_state, align 4
  br label %9

9:                                                ; preds = %87, %0
  %10 = load i32, ptr %1, align 4
  %11 = icmp slt i32 %10, 102200
  br i1 %11, label %12, label %88

12:                                               ; preds = %9
  store i32 0, ptr %2, align 4
  br label %13

13:                                               ; preds = %76, %12
  %14 = load i32, ptr %2, align 4
  %15 = icmp slt i32 %14, 10
  br i1 %15, label %16, label %79

16:                                               ; preds = %13
  %17 = load i32, ptr %2, align 4
  %18 = icmp sgt i32 %17, 0
  br i1 %18, label %19, label %27

19:                                               ; preds = %16
  %20 = load i32, ptr %1, align 4
  %21 = icmp slt i32 %20, 102400
  br i1 %21, label %22, label %27

22:                                               ; preds = %19
  %23 = load i32, ptr %1, align 4
  %24 = add nsw i32 %23, 1
  store i32 %24, ptr %1, align 4
  %25 = sext i32 %23 to i64
  %26 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %25
  store i8 44, ptr %26, align 1
  br label %27

27:                                               ; preds = %22, %19, %16
  %28 = call i32 @lcg_rand()
  %29 = urem i32 %28, 3
  %30 = icmp eq i32 %29, 0
  br i1 %30, label %31, label %40

31:                                               ; preds = %27
  %32 = call i32 @lcg_rand()
  %33 = urem i32 %32, 10000
  store i32 %33, ptr %5, align 4
  %34 = call i32 @lcg_rand()
  %35 = urem i32 %34, 100
  store i32 %35, ptr %6, align 4
  %36 = getelementptr inbounds [32 x i8], ptr %3, i64 0, i64 0
  %37 = load i32, ptr %5, align 4
  %38 = load i32, ptr %6, align 4
  %39 = call i32 (ptr, ptr, ...) @sprintf(ptr noundef %36, ptr noundef @.str.1, i32 noundef %37, i32 noundef %38) #4
  store i32 %39, ptr %4, align 4
  br label %53

40:                                               ; preds = %27
  %41 = call i32 @lcg_rand()
  %42 = urem i32 %41, 100000
  store i32 %42, ptr %7, align 4
  %43 = call i32 @lcg_rand()
  %44 = urem i32 %43, 4
  %45 = icmp eq i32 %44, 0
  br i1 %45, label %46, label %49

46:                                               ; preds = %40
  %47 = load i32, ptr %7, align 4
  %48 = sub nsw i32 0, %47
  store i32 %48, ptr %7, align 4
  br label %49

49:                                               ; preds = %46, %40
  %50 = getelementptr inbounds [32 x i8], ptr %3, i64 0, i64 0
  %51 = load i32, ptr %7, align 4
  %52 = call i32 (ptr, ptr, ...) @sprintf(ptr noundef %50, ptr noundef @.str.2, i32 noundef %51) #4
  store i32 %52, ptr %4, align 4
  br label %53

53:                                               ; preds = %49, %31
  store i32 0, ptr %8, align 4
  br label %54

54:                                               ; preds = %72, %53
  %55 = load i32, ptr %8, align 4
  %56 = load i32, ptr %4, align 4
  %57 = icmp slt i32 %55, %56
  br i1 %57, label %58, label %61

58:                                               ; preds = %54
  %59 = load i32, ptr %1, align 4
  %60 = icmp slt i32 %59, 102400
  br label %61

61:                                               ; preds = %58, %54
  %62 = phi i1 [ false, %54 ], [ %60, %58 ]
  br i1 %62, label %63, label %75

63:                                               ; preds = %61
  %64 = load i32, ptr %8, align 4
  %65 = sext i32 %64 to i64
  %66 = getelementptr inbounds [32 x i8], ptr %3, i64 0, i64 %65
  %67 = load i8, ptr %66, align 1
  %68 = load i32, ptr %1, align 4
  %69 = add nsw i32 %68, 1
  store i32 %69, ptr %1, align 4
  %70 = sext i32 %68 to i64
  %71 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %70
  store i8 %67, ptr %71, align 1
  br label %72

72:                                               ; preds = %63
  %73 = load i32, ptr %8, align 4
  %74 = add nsw i32 %73, 1
  store i32 %74, ptr %8, align 4
  br label %54, !llvm.loop !9

75:                                               ; preds = %61
  br label %76

76:                                               ; preds = %75
  %77 = load i32, ptr %2, align 4
  %78 = add nsw i32 %77, 1
  store i32 %78, ptr %2, align 4
  br label %13, !llvm.loop !10

79:                                               ; preds = %13
  %80 = load i32, ptr %1, align 4
  %81 = icmp slt i32 %80, 102400
  br i1 %81, label %82, label %87

82:                                               ; preds = %79
  %83 = load i32, ptr %1, align 4
  %84 = add nsw i32 %83, 1
  store i32 %84, ptr %1, align 4
  %85 = sext i32 %83 to i64
  %86 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %85
  store i8 10, ptr %86, align 1
  br label %87

87:                                               ; preds = %82, %79
  br label %9, !llvm.loop !11

88:                                               ; preds = %9
  %89 = load i32, ptr %1, align 4
  %90 = sext i32 %89 to i64
  %91 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %90
  store i8 0, ptr %91, align 1
  %92 = load i32, ptr %1, align 4
  store i32 %92, ptr @csv_len, align 4
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal void @do_parse() #0 {
  %1 = alloca double, align 8
  %2 = alloca i32, align 4
  %3 = alloca [32 x i8], align 16
  %4 = alloca i32, align 4
  store double 0.000000e+00, ptr %1, align 8
  store i32 0, ptr %2, align 4
  br label %5

5:                                                ; preds = %75, %0
  %6 = load i32, ptr %2, align 4
  %7 = load i32, ptr @csv_len, align 4
  %8 = icmp slt i32 %6, %7
  br i1 %8, label %9, label %76

9:                                                ; preds = %5
  store i32 0, ptr %4, align 4
  br label %10

10:                                               ; preds = %33, %9
  %11 = load i32, ptr %2, align 4
  %12 = load i32, ptr @csv_len, align 4
  %13 = icmp slt i32 %11, %12
  br i1 %13, label %14, label %31

14:                                               ; preds = %10
  %15 = load i32, ptr %2, align 4
  %16 = sext i32 %15 to i64
  %17 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %16
  %18 = load i8, ptr %17, align 1
  %19 = sext i8 %18 to i32
  %20 = icmp ne i32 %19, 44
  br i1 %20, label %21, label %31

21:                                               ; preds = %14
  %22 = load i32, ptr %2, align 4
  %23 = sext i32 %22 to i64
  %24 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %23
  %25 = load i8, ptr %24, align 1
  %26 = sext i8 %25 to i32
  %27 = icmp ne i32 %26, 10
  br i1 %27, label %28, label %31

28:                                               ; preds = %21
  %29 = load i32, ptr %4, align 4
  %30 = icmp slt i32 %29, 31
  br label %31

31:                                               ; preds = %28, %21, %14, %10
  %32 = phi i1 [ false, %21 ], [ false, %14 ], [ false, %10 ], [ %30, %28 ]
  br i1 %32, label %33, label %43

33:                                               ; preds = %31
  %34 = load i32, ptr %2, align 4
  %35 = add nsw i32 %34, 1
  store i32 %35, ptr %2, align 4
  %36 = sext i32 %34 to i64
  %37 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %36
  %38 = load i8, ptr %37, align 1
  %39 = load i32, ptr %4, align 4
  %40 = add nsw i32 %39, 1
  store i32 %40, ptr %4, align 4
  %41 = sext i32 %39 to i64
  %42 = getelementptr inbounds [32 x i8], ptr %3, i64 0, i64 %41
  store i8 %38, ptr %42, align 1
  br label %10, !llvm.loop !12

43:                                               ; preds = %31
  %44 = load i32, ptr %4, align 4
  %45 = sext i32 %44 to i64
  %46 = getelementptr inbounds [32 x i8], ptr %3, i64 0, i64 %45
  store i8 0, ptr %46, align 1
  %47 = load i32, ptr %4, align 4
  %48 = icmp sgt i32 %47, 0
  br i1 %48, label %49, label %54

49:                                               ; preds = %43
  %50 = getelementptr inbounds [32 x i8], ptr %3, i64 0, i64 0
  %51 = call double @atof(ptr noundef %50) #5
  %52 = load double, ptr %1, align 8
  %53 = fadd double %52, %51
  store double %53, ptr %1, align 8
  br label %54

54:                                               ; preds = %49, %43
  %55 = load i32, ptr %2, align 4
  %56 = load i32, ptr @csv_len, align 4
  %57 = icmp slt i32 %55, %56
  br i1 %57, label %58, label %75

58:                                               ; preds = %54
  %59 = load i32, ptr %2, align 4
  %60 = sext i32 %59 to i64
  %61 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %60
  %62 = load i8, ptr %61, align 1
  %63 = sext i8 %62 to i32
  %64 = icmp eq i32 %63, 44
  br i1 %64, label %72, label %65

65:                                               ; preds = %58
  %66 = load i32, ptr %2, align 4
  %67 = sext i32 %66 to i64
  %68 = getelementptr inbounds [102401 x i8], ptr @csv_buf, i64 0, i64 %67
  %69 = load i8, ptr %68, align 1
  %70 = sext i8 %69 to i32
  %71 = icmp eq i32 %70, 10
  br i1 %71, label %72, label %75

72:                                               ; preds = %65, %58
  %73 = load i32, ptr %2, align 4
  %74 = add nsw i32 %73, 1
  store i32 %74, ptr %2, align 4
  br label %75

75:                                               ; preds = %72, %65, %54
  br label %5, !llvm.loop !13

76:                                               ; preds = %5
  %77 = load double, ptr %1, align 8
  store volatile double %77, ptr @total_sum, align 8
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

; Function Attrs: nounwind
declare i32 @sprintf(ptr noundef, ptr noundef, ...) #1

; Function Attrs: nounwind willreturn memory(read)
declare double @atof(ptr noundef) #3

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nounwind willreturn memory(read) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { nounwind }
attributes #5 = { nounwind willreturn memory(read) }

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
