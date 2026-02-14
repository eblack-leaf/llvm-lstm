; ModuleID = 'benchmarks/fft.c'
source_filename = "benchmarks/fft.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@lcg_state = internal global i32 12345, align 4
@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca i32, align 4
  %2 = alloca ptr, align 8
  %3 = alloca ptr, align 8
  %4 = alloca ptr, align 8
  %5 = alloca ptr, align 8
  %6 = alloca i32, align 4
  %7 = alloca double, align 8
  %8 = alloca [50 x i64], align 16
  %9 = alloca %struct.timespec, align 8
  %10 = alloca %struct.timespec, align 8
  store i32 0, ptr %1, align 4
  %11 = call noalias ptr @malloc(i64 noundef 524288) #5
  store ptr %11, ptr %2, align 8
  %12 = call noalias ptr @malloc(i64 noundef 524288) #5
  store ptr %12, ptr %3, align 8
  %13 = call noalias ptr @malloc(i64 noundef 524288) #5
  store ptr %13, ptr %4, align 8
  %14 = call noalias ptr @malloc(i64 noundef 524288) #5
  store ptr %14, ptr %5, align 8
  store i32 12345, ptr @lcg_state, align 4
  store i32 0, ptr %6, align 4
  br label %15

15:                                               ; preds = %31, %0
  %16 = load i32, ptr %6, align 4
  %17 = icmp ult i32 %16, 65536
  br i1 %17, label %18, label %34

18:                                               ; preds = %15
  %19 = call i32 @lcg_rand()
  %20 = uitofp i32 %19 to double
  %21 = fdiv double %20, 3.276800e+04
  %22 = fsub double %21, 5.000000e-01
  %23 = load ptr, ptr %2, align 8
  %24 = load i32, ptr %6, align 4
  %25 = zext i32 %24 to i64
  %26 = getelementptr inbounds nuw double, ptr %23, i64 %25
  store double %22, ptr %26, align 8
  %27 = load ptr, ptr %3, align 8
  %28 = load i32, ptr %6, align 4
  %29 = zext i32 %28 to i64
  %30 = getelementptr inbounds nuw double, ptr %27, i64 %29
  store double 0.000000e+00, ptr %30, align 8
  br label %31

31:                                               ; preds = %18
  %32 = load i32, ptr %6, align 4
  %33 = add i32 %32, 1
  store i32 %33, ptr %6, align 4
  br label %15, !llvm.loop !6

34:                                               ; preds = %15
  store i32 0, ptr %6, align 4
  br label %35

35:                                               ; preds = %44, %34
  %36 = load i32, ptr %6, align 4
  %37 = icmp ult i32 %36, 5
  br i1 %37, label %38, label %47

38:                                               ; preds = %35
  %39 = load ptr, ptr %2, align 8
  %40 = load ptr, ptr %3, align 8
  %41 = load ptr, ptr %4, align 8
  %42 = load ptr, ptr %5, align 8
  %43 = call double @workload(ptr noundef %39, ptr noundef %40, ptr noundef %41, ptr noundef %42)
  store volatile double %43, ptr %7, align 8
  br label %44

44:                                               ; preds = %38
  %45 = load i32, ptr %6, align 4
  %46 = add i32 %45, 1
  store i32 %46, ptr %6, align 4
  br label %35, !llvm.loop !8

47:                                               ; preds = %35
  store i32 0, ptr %6, align 4
  br label %48

48:                                               ; preds = %63, %47
  %49 = load i32, ptr %6, align 4
  %50 = icmp ult i32 %49, 50
  br i1 %50, label %51, label %66

51:                                               ; preds = %48
  %52 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %9) #6
  %53 = load ptr, ptr %2, align 8
  %54 = load ptr, ptr %3, align 8
  %55 = load ptr, ptr %4, align 8
  %56 = load ptr, ptr %5, align 8
  %57 = call double @workload(ptr noundef %53, ptr noundef %54, ptr noundef %55, ptr noundef %56)
  store volatile double %57, ptr %7, align 8
  %58 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %10) #6
  %59 = call i64 @timespec_diff_ns(ptr noundef %9, ptr noundef %10)
  %60 = load i32, ptr %6, align 4
  %61 = zext i32 %60 to i64
  %62 = getelementptr inbounds nuw [50 x i64], ptr %8, i64 0, i64 %61
  store i64 %59, ptr %62, align 8
  br label %63

63:                                               ; preds = %51
  %64 = load i32, ptr %6, align 4
  %65 = add i32 %64, 1
  store i32 %65, ptr %6, align 4
  br label %48, !llvm.loop !9

66:                                               ; preds = %48
  %67 = getelementptr inbounds [50 x i64], ptr %8, i64 0, i64 0
  call void @qsort(ptr noundef %67, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %68 = getelementptr inbounds [50 x i64], ptr %8, i64 0, i64 25
  %69 = load i64, ptr %68, align 8
  %70 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %69)
  %71 = load ptr, ptr %2, align 8
  call void @free(ptr noundef %71) #6
  %72 = load ptr, ptr %3, align 8
  call void @free(ptr noundef %72) #6
  %73 = load ptr, ptr %4, align 8
  call void @free(ptr noundef %73) #6
  %74 = load ptr, ptr %5, align 8
  call void @free(ptr noundef %74) #6
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
  %4 = load i32, ptr @lcg_state, align 4
  %5 = lshr i32 %4, 16
  %6 = and i32 %5, 32767
  ret i32 %6
}

; Function Attrs: noinline nounwind uwtable
define internal double @workload(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) #0 {
  %5 = alloca ptr, align 8
  %6 = alloca ptr, align 8
  %7 = alloca ptr, align 8
  %8 = alloca ptr, align 8
  %9 = alloca i32, align 4
  %10 = alloca i32, align 4
  %11 = alloca i32, align 4
  %12 = alloca i32, align 4
  %13 = alloca i32, align 4
  %14 = alloca i32, align 4
  %15 = alloca i32, align 4
  %16 = alloca i32, align 4
  %17 = alloca double, align 8
  %18 = alloca double, align 8
  %19 = alloca double, align 8
  %20 = alloca double, align 8
  %21 = alloca i32, align 4
  %22 = alloca i32, align 4
  %23 = alloca double, align 8
  %24 = alloca double, align 8
  %25 = alloca double, align 8
  %26 = alloca double, align 8
  store ptr %0, ptr %5, align 8
  store ptr %1, ptr %6, align 8
  store ptr %2, ptr %7, align 8
  store ptr %3, ptr %8, align 8
  store i32 16, ptr %9, align 4
  store i32 0, ptr %10, align 4
  br label %27

27:                                               ; preds = %52, %4
  %28 = load i32, ptr %10, align 4
  %29 = icmp ult i32 %28, 65536
  br i1 %29, label %30, label %55

30:                                               ; preds = %27
  %31 = load i32, ptr %10, align 4
  %32 = load i32, ptr %9, align 4
  %33 = call i32 @bit_reverse(i32 noundef %31, i32 noundef %32)
  store i32 %33, ptr %14, align 4
  %34 = load ptr, ptr %5, align 8
  %35 = load i32, ptr %10, align 4
  %36 = zext i32 %35 to i64
  %37 = getelementptr inbounds nuw double, ptr %34, i64 %36
  %38 = load double, ptr %37, align 8
  %39 = load ptr, ptr %7, align 8
  %40 = load i32, ptr %14, align 4
  %41 = zext i32 %40 to i64
  %42 = getelementptr inbounds nuw double, ptr %39, i64 %41
  store double %38, ptr %42, align 8
  %43 = load ptr, ptr %6, align 8
  %44 = load i32, ptr %10, align 4
  %45 = zext i32 %44 to i64
  %46 = getelementptr inbounds nuw double, ptr %43, i64 %45
  %47 = load double, ptr %46, align 8
  %48 = load ptr, ptr %8, align 8
  %49 = load i32, ptr %14, align 4
  %50 = zext i32 %49 to i64
  %51 = getelementptr inbounds nuw double, ptr %48, i64 %50
  store double %47, ptr %51, align 8
  br label %52

52:                                               ; preds = %30
  %53 = load i32, ptr %10, align 4
  %54 = add i32 %53, 1
  store i32 %54, ptr %10, align 4
  br label %27, !llvm.loop !10

55:                                               ; preds = %27
  store i32 1, ptr %11, align 4
  br label %56

56:                                               ; preds = %186, %55
  %57 = load i32, ptr %11, align 4
  %58 = load i32, ptr %9, align 4
  %59 = icmp sle i32 %57, %58
  br i1 %59, label %60, label %189

60:                                               ; preds = %56
  %61 = load i32, ptr %11, align 4
  %62 = shl i32 1, %61
  store i32 %62, ptr %15, align 4
  %63 = load i32, ptr %15, align 4
  %64 = ashr i32 %63, 1
  store i32 %64, ptr %16, align 4
  %65 = load i32, ptr %15, align 4
  %66 = sitofp i32 %65 to double
  %67 = fdiv double 0xC01921FB54442D18, %66
  %68 = call double @cos(double noundef %67) #6
  store double %68, ptr %17, align 8
  %69 = load i32, ptr %15, align 4
  %70 = sitofp i32 %69 to double
  %71 = fdiv double 0xC01921FB54442D18, %70
  %72 = call double @sin(double noundef %71) #6
  store double %72, ptr %18, align 8
  store i32 0, ptr %13, align 4
  br label %73

73:                                               ; preds = %181, %60
  %74 = load i32, ptr %13, align 4
  %75 = icmp slt i32 %74, 65536
  br i1 %75, label %76, label %185

76:                                               ; preds = %73
  store double 1.000000e+00, ptr %19, align 8
  store double 0.000000e+00, ptr %20, align 8
  store i32 0, ptr %12, align 4
  br label %77

77:                                               ; preds = %177, %76
  %78 = load i32, ptr %12, align 4
  %79 = load i32, ptr %16, align 4
  %80 = icmp slt i32 %78, %79
  br i1 %80, label %81, label %180

81:                                               ; preds = %77
  %82 = load i32, ptr %13, align 4
  %83 = load i32, ptr %12, align 4
  %84 = add nsw i32 %82, %83
  store i32 %84, ptr %21, align 4
  %85 = load i32, ptr %13, align 4
  %86 = load i32, ptr %12, align 4
  %87 = add nsw i32 %85, %86
  %88 = load i32, ptr %16, align 4
  %89 = add nsw i32 %87, %88
  store i32 %89, ptr %22, align 4
  %90 = load double, ptr %19, align 8
  %91 = load ptr, ptr %7, align 8
  %92 = load i32, ptr %22, align 4
  %93 = sext i32 %92 to i64
  %94 = getelementptr inbounds double, ptr %91, i64 %93
  %95 = load double, ptr %94, align 8
  %96 = load double, ptr %20, align 8
  %97 = load ptr, ptr %8, align 8
  %98 = load i32, ptr %22, align 4
  %99 = sext i32 %98 to i64
  %100 = getelementptr inbounds double, ptr %97, i64 %99
  %101 = load double, ptr %100, align 8
  %102 = fmul double %96, %101
  %103 = fneg double %102
  %104 = call double @llvm.fmuladd.f64(double %90, double %95, double %103)
  store double %104, ptr %23, align 8
  %105 = load double, ptr %19, align 8
  %106 = load ptr, ptr %8, align 8
  %107 = load i32, ptr %22, align 4
  %108 = sext i32 %107 to i64
  %109 = getelementptr inbounds double, ptr %106, i64 %108
  %110 = load double, ptr %109, align 8
  %111 = load double, ptr %20, align 8
  %112 = load ptr, ptr %7, align 8
  %113 = load i32, ptr %22, align 4
  %114 = sext i32 %113 to i64
  %115 = getelementptr inbounds double, ptr %112, i64 %114
  %116 = load double, ptr %115, align 8
  %117 = fmul double %111, %116
  %118 = call double @llvm.fmuladd.f64(double %105, double %110, double %117)
  store double %118, ptr %24, align 8
  %119 = load ptr, ptr %7, align 8
  %120 = load i32, ptr %21, align 4
  %121 = sext i32 %120 to i64
  %122 = getelementptr inbounds double, ptr %119, i64 %121
  %123 = load double, ptr %122, align 8
  %124 = load double, ptr %23, align 8
  %125 = fsub double %123, %124
  %126 = load ptr, ptr %7, align 8
  %127 = load i32, ptr %22, align 4
  %128 = sext i32 %127 to i64
  %129 = getelementptr inbounds double, ptr %126, i64 %128
  store double %125, ptr %129, align 8
  %130 = load ptr, ptr %8, align 8
  %131 = load i32, ptr %21, align 4
  %132 = sext i32 %131 to i64
  %133 = getelementptr inbounds double, ptr %130, i64 %132
  %134 = load double, ptr %133, align 8
  %135 = load double, ptr %24, align 8
  %136 = fsub double %134, %135
  %137 = load ptr, ptr %8, align 8
  %138 = load i32, ptr %22, align 4
  %139 = sext i32 %138 to i64
  %140 = getelementptr inbounds double, ptr %137, i64 %139
  store double %136, ptr %140, align 8
  %141 = load ptr, ptr %7, align 8
  %142 = load i32, ptr %21, align 4
  %143 = sext i32 %142 to i64
  %144 = getelementptr inbounds double, ptr %141, i64 %143
  %145 = load double, ptr %144, align 8
  %146 = load double, ptr %23, align 8
  %147 = fadd double %145, %146
  %148 = load ptr, ptr %7, align 8
  %149 = load i32, ptr %21, align 4
  %150 = sext i32 %149 to i64
  %151 = getelementptr inbounds double, ptr %148, i64 %150
  store double %147, ptr %151, align 8
  %152 = load ptr, ptr %8, align 8
  %153 = load i32, ptr %21, align 4
  %154 = sext i32 %153 to i64
  %155 = getelementptr inbounds double, ptr %152, i64 %154
  %156 = load double, ptr %155, align 8
  %157 = load double, ptr %24, align 8
  %158 = fadd double %156, %157
  %159 = load ptr, ptr %8, align 8
  %160 = load i32, ptr %21, align 4
  %161 = sext i32 %160 to i64
  %162 = getelementptr inbounds double, ptr %159, i64 %161
  store double %158, ptr %162, align 8
  %163 = load double, ptr %19, align 8
  %164 = load double, ptr %17, align 8
  %165 = load double, ptr %20, align 8
  %166 = load double, ptr %18, align 8
  %167 = fmul double %165, %166
  %168 = fneg double %167
  %169 = call double @llvm.fmuladd.f64(double %163, double %164, double %168)
  store double %169, ptr %25, align 8
  %170 = load double, ptr %19, align 8
  %171 = load double, ptr %18, align 8
  %172 = load double, ptr %20, align 8
  %173 = load double, ptr %17, align 8
  %174 = fmul double %172, %173
  %175 = call double @llvm.fmuladd.f64(double %170, double %171, double %174)
  store double %175, ptr %20, align 8
  %176 = load double, ptr %25, align 8
  store double %176, ptr %19, align 8
  br label %177

177:                                              ; preds = %81
  %178 = load i32, ptr %12, align 4
  %179 = add nsw i32 %178, 1
  store i32 %179, ptr %12, align 4
  br label %77, !llvm.loop !11

180:                                              ; preds = %77
  br label %181

181:                                              ; preds = %180
  %182 = load i32, ptr %15, align 4
  %183 = load i32, ptr %13, align 4
  %184 = add nsw i32 %183, %182
  store i32 %184, ptr %13, align 4
  br label %73, !llvm.loop !12

185:                                              ; preds = %73
  br label %186

186:                                              ; preds = %185
  %187 = load i32, ptr %11, align 4
  %188 = add nsw i32 %187, 1
  store i32 %188, ptr %11, align 4
  br label %56, !llvm.loop !13

189:                                              ; preds = %56
  store double 0.000000e+00, ptr %26, align 8
  store i32 0, ptr %10, align 4
  br label %190

190:                                              ; preds = %218, %189
  %191 = load i32, ptr %10, align 4
  %192 = icmp ult i32 %191, 65536
  br i1 %192, label %193, label %221

193:                                              ; preds = %190
  %194 = load ptr, ptr %7, align 8
  %195 = load i32, ptr %10, align 4
  %196 = zext i32 %195 to i64
  %197 = getelementptr inbounds nuw double, ptr %194, i64 %196
  %198 = load double, ptr %197, align 8
  %199 = load ptr, ptr %7, align 8
  %200 = load i32, ptr %10, align 4
  %201 = zext i32 %200 to i64
  %202 = getelementptr inbounds nuw double, ptr %199, i64 %201
  %203 = load double, ptr %202, align 8
  %204 = load ptr, ptr %8, align 8
  %205 = load i32, ptr %10, align 4
  %206 = zext i32 %205 to i64
  %207 = getelementptr inbounds nuw double, ptr %204, i64 %206
  %208 = load double, ptr %207, align 8
  %209 = load ptr, ptr %8, align 8
  %210 = load i32, ptr %10, align 4
  %211 = zext i32 %210 to i64
  %212 = getelementptr inbounds nuw double, ptr %209, i64 %211
  %213 = load double, ptr %212, align 8
  %214 = fmul double %208, %213
  %215 = call double @llvm.fmuladd.f64(double %198, double %203, double %214)
  %216 = load double, ptr %26, align 8
  %217 = fadd double %216, %215
  store double %217, ptr %26, align 8
  br label %218

218:                                              ; preds = %193
  %219 = load i32, ptr %10, align 4
  %220 = add i32 %219, 1
  store i32 %220, ptr %10, align 4
  br label %190, !llvm.loop !14

221:                                              ; preds = %190
  %222 = load double, ptr %26, align 8
  ret double %222
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

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #3

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

declare i32 @printf(ptr noundef, ...) #3

; Function Attrs: nounwind
declare void @free(ptr noundef) #2

; Function Attrs: noinline nounwind uwtable
define internal i32 @bit_reverse(i32 noundef %0, i32 noundef %1) #0 {
  %3 = alloca i32, align 4
  %4 = alloca i32, align 4
  %5 = alloca i32, align 4
  %6 = alloca i32, align 4
  store i32 %0, ptr %3, align 4
  store i32 %1, ptr %4, align 4
  store i32 0, ptr %5, align 4
  store i32 0, ptr %6, align 4
  br label %7

7:                                                ; preds = %19, %2
  %8 = load i32, ptr %6, align 4
  %9 = load i32, ptr %4, align 4
  %10 = icmp slt i32 %8, %9
  br i1 %10, label %11, label %22

11:                                               ; preds = %7
  %12 = load i32, ptr %5, align 4
  %13 = shl i32 %12, 1
  %14 = load i32, ptr %3, align 4
  %15 = and i32 %14, 1
  %16 = or i32 %13, %15
  store i32 %16, ptr %5, align 4
  %17 = load i32, ptr %3, align 4
  %18 = lshr i32 %17, 1
  store i32 %18, ptr %3, align 4
  br label %19

19:                                               ; preds = %11
  %20 = load i32, ptr %6, align 4
  %21 = add nsw i32 %20, 1
  store i32 %21, ptr %6, align 4
  br label %7, !llvm.loop !15

22:                                               ; preds = %7
  %23 = load i32, ptr %5, align 4
  ret i32 %23
}

; Function Attrs: nounwind
declare double @cos(double noundef) #2

; Function Attrs: nounwind
declare double @sin(double noundef) #2

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare double @llvm.fmuladd.f64(double, double, double) #4

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind allocsize(0) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
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
!15 = distinct !{!15, !7}
