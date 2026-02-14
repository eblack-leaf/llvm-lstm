; ModuleID = 'data/exploratory/_work/matrix_multiply_tiled.ll'
source_filename = "benchmarks/matrix_multiply_tiled.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@lcg_state = internal global i32 12345, align 4
@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca ptr, align 8
  %2 = alloca ptr, align 8
  %3 = alloca ptr, align 8
  %4 = alloca double, align 8
  %5 = alloca [50 x i64], align 16
  %6 = alloca %struct.timespec, align 8
  %7 = alloca %struct.timespec, align 8
  %8 = tail call noalias dereferenceable_or_null(131072) ptr @malloc(i64 noundef 131072) #6
  store ptr %8, ptr %1, align 8
  %9 = tail call noalias dereferenceable_or_null(131072) ptr @malloc(i64 noundef 131072) #6
  store ptr %9, ptr %2, align 8
  %10 = tail call noalias dereferenceable_or_null(131072) ptr @malloc(i64 noundef 131072) #6
  store ptr %10, ptr %3, align 8
  store i32 12345, ptr @lcg_state, align 4
  br label %11

11:                                               ; preds = %11, %0
  %storemerge4 = phi i32 [ 0, %0 ], [ %17, %11 ]
  %12 = tail call i32 @lcg_rand()
  %13 = uitofp i32 %12 to double
  %14 = fmul double %13, 0x3F00000000000000
  %15 = zext nneg i32 %storemerge4 to i64
  %16 = getelementptr inbounds nuw double, ptr %8, i64 %15
  store double %14, ptr %16, align 8
  %17 = add nuw nsw i32 %storemerge4, 1
  %18 = icmp samesign ult i32 %17, 16384
  br i1 %18, label %11, label %19, !llvm.loop !6

19:                                               ; preds = %11
  br label %20

20:                                               ; preds = %20, %19
  %storemerge15 = phi i32 [ 0, %19 ], [ %26, %20 ]
  %21 = tail call i32 @lcg_rand()
  %22 = uitofp i32 %21 to double
  %23 = fmul double %22, 0x3F00000000000000
  %24 = zext nneg i32 %storemerge15 to i64
  %25 = getelementptr inbounds nuw double, ptr %9, i64 %24
  store double %23, ptr %25, align 8
  %26 = add nuw nsw i32 %storemerge15, 1
  %27 = icmp samesign ult i32 %26, 16384
  br i1 %27, label %20, label %28, !llvm.loop !8

28:                                               ; preds = %20
  br label %29

29:                                               ; preds = %28
  %30 = tail call double @workload(ptr noundef %8, ptr noundef %9, ptr noundef %10)
  store volatile double %30, ptr %4, align 8
  %31 = tail call double @workload(ptr noundef %8, ptr noundef %9, ptr noundef %10)
  store volatile double %31, ptr %4, align 8
  %32 = tail call double @workload(ptr noundef %8, ptr noundef %9, ptr noundef %10)
  store volatile double %32, ptr %4, align 8
  %33 = tail call double @workload(ptr noundef %8, ptr noundef %9, ptr noundef %10)
  store volatile double %33, ptr %4, align 8
  %34 = tail call double @workload(ptr noundef %8, ptr noundef %9, ptr noundef %10)
  store volatile double %34, ptr %4, align 8
  br label %35

35:                                               ; preds = %35, %29
  %storemerge37 = phi i32 [ 0, %29 ], [ %42, %35 ]
  %36 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %6) #7
  %37 = call double @workload(ptr noundef %8, ptr noundef %9, ptr noundef %10)
  store volatile double %37, ptr %4, align 8
  %38 = call i32 @clock_gettime(i32 noundef 1, ptr noundef nonnull %7) #7
  %39 = call i64 @timespec_diff_ns(ptr noundef nonnull %6, ptr noundef nonnull %7)
  %40 = zext nneg i32 %storemerge37 to i64
  %41 = getelementptr inbounds nuw [50 x i64], ptr %5, i64 0, i64 %40
  store i64 %39, ptr %41, align 8
  %42 = add nuw nsw i32 %storemerge37, 1
  %43 = icmp samesign ult i32 %42, 50
  br i1 %43, label %35, label %44, !llvm.loop !9

44:                                               ; preds = %35
  call void @qsort(ptr noundef nonnull %5, i64 noundef 50, i64 noundef 8, ptr noundef nonnull @cmp_ll) #7
  %45 = getelementptr inbounds nuw i8, ptr %5, i64 200
  %46 = load i64, ptr %45, align 8
  %47 = call i32 (ptr, ...) @printf(ptr noundef nonnull dereferenceable(1) @.str, i64 noundef %46) #7
  call void @free(ptr noundef %8) #7
  call void @free(ptr noundef %9) #7
  call void @free(ptr noundef %10) #7
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
define internal double @workload(ptr noundef %0, ptr noundef %1, ptr noundef %2) #0 {
  %4 = alloca ptr, align 8
  %5 = alloca i32, align 4
  %6 = alloca i32, align 4
  %7 = alloca double, align 8
  store ptr %2, ptr %4, align 8
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(131072) %2, i8 0, i64 131072, i1 false)
  br label %8

8:                                                ; preds = %3, %207
  %storemerge50 = phi i32 [ 0, %3 ], [ %9, %207 ]
  %9 = add nuw nsw i32 %storemerge50, 16
  br label %10

10:                                               ; preds = %8, %205
  %storemerge249 = phi i32 [ 0, %8 ], [ %11, %205 ]
  %11 = add nuw nsw i32 %storemerge249, 16
  br label %12

12:                                               ; preds = %10, %203
  %storemerge348 = phi i32 [ 0, %10 ], [ %13, %203 ]
  %13 = add nuw nsw i32 %storemerge348, 16
  br label %14

14:                                               ; preds = %12, %200
  %storemerge447 = phi i32 [ %storemerge50, %12 ], [ %201, %200 ]
  %15 = shl nsw i32 %storemerge447, 7
  %16 = add nuw nsw i32 %15, %storemerge348
  %17 = zext nneg i32 %16 to i64
  %18 = getelementptr inbounds nuw double, ptr %0, i64 %17
  br label %19

19:                                               ; preds = %14, %24
  %storemerge546 = phi i32 [ %storemerge249, %14 ], [ %198, %24 ]
  %20 = add nuw nsw i32 %15, %storemerge546
  %21 = zext nneg i32 %20 to i64
  %22 = getelementptr inbounds nuw double, ptr %2, i64 %21
  %23 = load double, ptr %22, align 8
  br label %24

24:                                               ; preds = %19
  %25 = load double, ptr %18, align 8
  %26 = shl nsw i32 %storemerge348, 7
  %27 = add nuw nsw i32 %26, %storemerge546
  %28 = zext nneg i32 %27 to i64
  %29 = getelementptr inbounds nuw double, ptr %1, i64 %28
  %30 = load double, ptr %29, align 8
  %31 = tail call double @llvm.fmuladd.f64(double %25, double %30, double %23)
  %32 = add nuw nsw i32 %storemerge348, 1
  %33 = add nuw nsw i32 %15, %32
  %34 = zext nneg i32 %33 to i64
  %35 = getelementptr inbounds nuw double, ptr %0, i64 %34
  %36 = load double, ptr %35, align 8
  %37 = shl nsw i32 %32, 7
  %38 = add nuw nsw i32 %37, %storemerge546
  %39 = zext nneg i32 %38 to i64
  %40 = getelementptr inbounds nuw double, ptr %1, i64 %39
  %41 = load double, ptr %40, align 8
  %42 = tail call double @llvm.fmuladd.f64(double %36, double %41, double %31)
  %43 = add nuw nsw i32 %storemerge348, 2
  %44 = add nuw nsw i32 %15, %43
  %45 = zext nneg i32 %44 to i64
  %46 = getelementptr inbounds nuw double, ptr %0, i64 %45
  %47 = load double, ptr %46, align 8
  %48 = shl nsw i32 %43, 7
  %49 = add nuw nsw i32 %48, %storemerge546
  %50 = zext nneg i32 %49 to i64
  %51 = getelementptr inbounds nuw double, ptr %1, i64 %50
  %52 = load double, ptr %51, align 8
  %53 = tail call double @llvm.fmuladd.f64(double %47, double %52, double %42)
  %54 = add nuw nsw i32 %storemerge348, 3
  %55 = add nuw nsw i32 %15, %54
  %56 = zext nneg i32 %55 to i64
  %57 = getelementptr inbounds nuw double, ptr %0, i64 %56
  %58 = load double, ptr %57, align 8
  %59 = shl nsw i32 %54, 7
  %60 = add nuw nsw i32 %59, %storemerge546
  %61 = zext nneg i32 %60 to i64
  %62 = getelementptr inbounds nuw double, ptr %1, i64 %61
  %63 = load double, ptr %62, align 8
  %64 = tail call double @llvm.fmuladd.f64(double %58, double %63, double %53)
  %65 = add nuw nsw i32 %storemerge348, 4
  %66 = add nuw nsw i32 %15, %65
  %67 = zext nneg i32 %66 to i64
  %68 = getelementptr inbounds nuw double, ptr %0, i64 %67
  %69 = load double, ptr %68, align 8
  %70 = shl nsw i32 %65, 7
  %71 = add nuw nsw i32 %70, %storemerge546
  %72 = zext nneg i32 %71 to i64
  %73 = getelementptr inbounds nuw double, ptr %1, i64 %72
  %74 = load double, ptr %73, align 8
  %75 = tail call double @llvm.fmuladd.f64(double %69, double %74, double %64)
  %76 = add nuw nsw i32 %storemerge348, 5
  %77 = add nuw nsw i32 %15, %76
  %78 = zext nneg i32 %77 to i64
  %79 = getelementptr inbounds nuw double, ptr %0, i64 %78
  %80 = load double, ptr %79, align 8
  %81 = shl nsw i32 %76, 7
  %82 = add nuw nsw i32 %81, %storemerge546
  %83 = zext nneg i32 %82 to i64
  %84 = getelementptr inbounds nuw double, ptr %1, i64 %83
  %85 = load double, ptr %84, align 8
  %86 = tail call double @llvm.fmuladd.f64(double %80, double %85, double %75)
  %87 = add nuw nsw i32 %storemerge348, 6
  %88 = add nuw nsw i32 %15, %87
  %89 = zext nneg i32 %88 to i64
  %90 = getelementptr inbounds nuw double, ptr %0, i64 %89
  %91 = load double, ptr %90, align 8
  %92 = shl nsw i32 %87, 7
  %93 = add nuw nsw i32 %92, %storemerge546
  %94 = zext nneg i32 %93 to i64
  %95 = getelementptr inbounds nuw double, ptr %1, i64 %94
  %96 = load double, ptr %95, align 8
  %97 = tail call double @llvm.fmuladd.f64(double %91, double %96, double %86)
  %98 = add nuw nsw i32 %storemerge348, 7
  %99 = add nuw nsw i32 %15, %98
  %100 = zext nneg i32 %99 to i64
  %101 = getelementptr inbounds nuw double, ptr %0, i64 %100
  %102 = load double, ptr %101, align 8
  %103 = shl nsw i32 %98, 7
  %104 = add nuw nsw i32 %103, %storemerge546
  %105 = zext nneg i32 %104 to i64
  %106 = getelementptr inbounds nuw double, ptr %1, i64 %105
  %107 = load double, ptr %106, align 8
  %108 = tail call double @llvm.fmuladd.f64(double %102, double %107, double %97)
  %109 = add nuw nsw i32 %storemerge348, 8
  %110 = add nuw nsw i32 %15, %109
  %111 = zext nneg i32 %110 to i64
  %112 = getelementptr inbounds nuw double, ptr %0, i64 %111
  %113 = load double, ptr %112, align 8
  %114 = shl nsw i32 %109, 7
  %115 = add nuw nsw i32 %114, %storemerge546
  %116 = zext nneg i32 %115 to i64
  %117 = getelementptr inbounds nuw double, ptr %1, i64 %116
  %118 = load double, ptr %117, align 8
  %119 = tail call double @llvm.fmuladd.f64(double %113, double %118, double %108)
  %120 = add nuw nsw i32 %storemerge348, 9
  %121 = add nuw nsw i32 %15, %120
  %122 = zext nneg i32 %121 to i64
  %123 = getelementptr inbounds nuw double, ptr %0, i64 %122
  %124 = load double, ptr %123, align 8
  %125 = shl nsw i32 %120, 7
  %126 = add nuw nsw i32 %125, %storemerge546
  %127 = zext nneg i32 %126 to i64
  %128 = getelementptr inbounds nuw double, ptr %1, i64 %127
  %129 = load double, ptr %128, align 8
  %130 = tail call double @llvm.fmuladd.f64(double %124, double %129, double %119)
  %131 = add nuw nsw i32 %storemerge348, 10
  %132 = add nuw nsw i32 %15, %131
  %133 = zext nneg i32 %132 to i64
  %134 = getelementptr inbounds nuw double, ptr %0, i64 %133
  %135 = load double, ptr %134, align 8
  %136 = shl nsw i32 %131, 7
  %137 = add nuw nsw i32 %136, %storemerge546
  %138 = zext nneg i32 %137 to i64
  %139 = getelementptr inbounds nuw double, ptr %1, i64 %138
  %140 = load double, ptr %139, align 8
  %141 = tail call double @llvm.fmuladd.f64(double %135, double %140, double %130)
  %142 = add nuw nsw i32 %storemerge348, 11
  %143 = add nuw nsw i32 %15, %142
  %144 = zext nneg i32 %143 to i64
  %145 = getelementptr inbounds nuw double, ptr %0, i64 %144
  %146 = load double, ptr %145, align 8
  %147 = shl nsw i32 %142, 7
  %148 = add nuw nsw i32 %147, %storemerge546
  %149 = zext nneg i32 %148 to i64
  %150 = getelementptr inbounds nuw double, ptr %1, i64 %149
  %151 = load double, ptr %150, align 8
  %152 = tail call double @llvm.fmuladd.f64(double %146, double %151, double %141)
  %153 = add nuw nsw i32 %storemerge348, 12
  %154 = add nuw nsw i32 %15, %153
  %155 = zext nneg i32 %154 to i64
  %156 = getelementptr inbounds nuw double, ptr %0, i64 %155
  %157 = load double, ptr %156, align 8
  %158 = shl nsw i32 %153, 7
  %159 = add nuw nsw i32 %158, %storemerge546
  %160 = zext nneg i32 %159 to i64
  %161 = getelementptr inbounds nuw double, ptr %1, i64 %160
  %162 = load double, ptr %161, align 8
  %163 = tail call double @llvm.fmuladd.f64(double %157, double %162, double %152)
  %164 = add nuw nsw i32 %storemerge348, 13
  %165 = add nuw nsw i32 %15, %164
  %166 = zext nneg i32 %165 to i64
  %167 = getelementptr inbounds nuw double, ptr %0, i64 %166
  %168 = load double, ptr %167, align 8
  %169 = shl nsw i32 %164, 7
  %170 = add nuw nsw i32 %169, %storemerge546
  %171 = zext nneg i32 %170 to i64
  %172 = getelementptr inbounds nuw double, ptr %1, i64 %171
  %173 = load double, ptr %172, align 8
  %174 = tail call double @llvm.fmuladd.f64(double %168, double %173, double %163)
  %175 = add nuw nsw i32 %storemerge348, 14
  %176 = add nuw nsw i32 %15, %175
  %177 = zext nneg i32 %176 to i64
  %178 = getelementptr inbounds nuw double, ptr %0, i64 %177
  %179 = load double, ptr %178, align 8
  %180 = shl nsw i32 %175, 7
  %181 = add nuw nsw i32 %180, %storemerge546
  %182 = zext nneg i32 %181 to i64
  %183 = getelementptr inbounds nuw double, ptr %1, i64 %182
  %184 = load double, ptr %183, align 8
  %185 = tail call double @llvm.fmuladd.f64(double %179, double %184, double %174)
  %186 = add nuw nsw i32 %storemerge348, 15
  %187 = add nuw nsw i32 %15, %186
  %188 = zext nneg i32 %187 to i64
  %189 = getelementptr inbounds nuw double, ptr %0, i64 %188
  %190 = load double, ptr %189, align 8
  %191 = shl nsw i32 %186, 7
  %192 = add nuw nsw i32 %191, %storemerge546
  %193 = zext nneg i32 %192 to i64
  %194 = getelementptr inbounds nuw double, ptr %1, i64 %193
  %195 = load double, ptr %194, align 8
  %196 = tail call double @llvm.fmuladd.f64(double %190, double %195, double %185)
  %197 = add nuw nsw i32 %storemerge348, 16
  store double %196, ptr %22, align 8
  %198 = add nuw nsw i32 %storemerge546, 1
  %199 = icmp samesign ult i32 %198, %11
  br i1 %199, label %19, label %200, !llvm.loop !10

200:                                              ; preds = %24
  %.lcssa54 = phi i32 [ %198, %24 ]
  %.lcssa53.lcssa = phi double [ %196, %24 ]
  %.lcssa52.lcssa = phi i32 [ %197, %24 ]
  %201 = add nuw nsw i32 %storemerge447, 1
  %202 = icmp samesign ult i32 %201, %9
  br i1 %202, label %14, label %203, !llvm.loop !11

203:                                              ; preds = %200
  %.lcssa54.lcssa = phi i32 [ %.lcssa54, %200 ]
  %.lcssa53.lcssa.lcssa = phi double [ %.lcssa53.lcssa, %200 ]
  %.lcssa52.lcssa.lcssa = phi i32 [ %.lcssa52.lcssa, %200 ]
  %204 = icmp samesign ult i32 %13, 128
  br i1 %204, label %12, label %205, !llvm.loop !12

205:                                              ; preds = %203
  %.lcssa54.lcssa.lcssa = phi i32 [ %.lcssa54.lcssa, %203 ]
  %.lcssa53.lcssa.lcssa.lcssa = phi double [ %.lcssa53.lcssa.lcssa, %203 ]
  %.lcssa52.lcssa.lcssa.lcssa = phi i32 [ %.lcssa52.lcssa.lcssa, %203 ]
  %206 = icmp samesign ult i32 %11, 128
  br i1 %206, label %10, label %207, !llvm.loop !13

207:                                              ; preds = %205
  %.lcssa54.lcssa.lcssa.lcssa = phi i32 [ %.lcssa54.lcssa.lcssa, %205 ]
  %.lcssa53.lcssa.lcssa.lcssa.lcssa = phi double [ %.lcssa53.lcssa.lcssa.lcssa, %205 ]
  %.lcssa52.lcssa.lcssa.lcssa.lcssa = phi i32 [ %.lcssa52.lcssa.lcssa.lcssa, %205 ]
  %208 = icmp samesign ult i32 %9, 128
  br i1 %208, label %8, label %209, !llvm.loop !14

209:                                              ; preds = %207
  %.lcssa54.lcssa.lcssa.lcssa.lcssa = phi i32 [ %.lcssa54.lcssa.lcssa.lcssa, %207 ]
  %.lcssa53.lcssa.lcssa.lcssa.lcssa.lcssa = phi double [ %.lcssa53.lcssa.lcssa.lcssa.lcssa, %207 ]
  %.lcssa52.lcssa.lcssa.lcssa.lcssa.lcssa = phi i32 [ %.lcssa52.lcssa.lcssa.lcssa.lcssa, %207 ]
  store double %.lcssa53.lcssa.lcssa.lcssa.lcssa.lcssa, ptr %7, align 8
  store i32 %.lcssa52.lcssa.lcssa.lcssa.lcssa.lcssa, ptr %6, align 1
  store i32 %.lcssa54.lcssa.lcssa.lcssa.lcssa, ptr %5, align 1
  br label %210

210:                                              ; preds = %210, %209
  %storemerge151 = phi i32 [ 0, %209 ], [ %251, %210 ]
  %211 = phi double [ 0.000000e+00, %209 ], [ %250, %210 ]
  %212 = zext nneg i32 %storemerge151 to i64
  %213 = getelementptr inbounds nuw double, ptr %2, i64 %212
  %214 = load double, ptr %213, align 8
  %215 = fadd double %211, %214
  %216 = add nuw nsw i32 %storemerge151, 1
  %217 = zext nneg i32 %216 to i64
  %218 = getelementptr inbounds nuw double, ptr %2, i64 %217
  %219 = load double, ptr %218, align 8
  %220 = fadd double %215, %219
  %221 = add nuw nsw i32 %storemerge151, 2
  %222 = zext nneg i32 %221 to i64
  %223 = getelementptr inbounds nuw double, ptr %2, i64 %222
  %224 = load double, ptr %223, align 8
  %225 = fadd double %220, %224
  %226 = add nuw nsw i32 %storemerge151, 3
  %227 = zext nneg i32 %226 to i64
  %228 = getelementptr inbounds nuw double, ptr %2, i64 %227
  %229 = load double, ptr %228, align 8
  %230 = fadd double %225, %229
  %231 = add nuw nsw i32 %storemerge151, 4
  %232 = zext nneg i32 %231 to i64
  %233 = getelementptr inbounds nuw double, ptr %2, i64 %232
  %234 = load double, ptr %233, align 8
  %235 = fadd double %230, %234
  %236 = add nuw nsw i32 %storemerge151, 5
  %237 = zext nneg i32 %236 to i64
  %238 = getelementptr inbounds nuw double, ptr %2, i64 %237
  %239 = load double, ptr %238, align 8
  %240 = fadd double %235, %239
  %241 = add nuw nsw i32 %storemerge151, 6
  %242 = zext nneg i32 %241 to i64
  %243 = getelementptr inbounds nuw double, ptr %2, i64 %242
  %244 = load double, ptr %243, align 8
  %245 = fadd double %240, %244
  %246 = add nuw nsw i32 %storemerge151, 7
  %247 = zext nneg i32 %246 to i64
  %248 = getelementptr inbounds nuw double, ptr %2, i64 %247
  %249 = load double, ptr %248, align 8
  %250 = fadd double %245, %249
  %251 = add nuw nsw i32 %storemerge151, 8
  %252 = icmp samesign ult i32 %251, 16384
  br i1 %252, label %210, label %253, !llvm.loop !15

253:                                              ; preds = %210
  %.lcssa = phi double [ %250, %210 ]
  ret double %.lcssa
}

; Function Attrs: nounwind
declare i32 @clock_gettime(i32 noundef, ptr noundef) #2

; Function Attrs: noinline nounwind uwtable
define internal i64 @timespec_diff_ns(ptr noundef %0, ptr noundef %1) #0 {
  %3 = alloca ptr, align 8
  store ptr %0, ptr %3, align 8
  %4 = load i64, ptr %1, align 8
  %5 = load i64, ptr %0, align 8
  %6 = sub nsw i64 %4, %5
  %7 = mul nsw i64 %6, 1000000000
  %8 = getelementptr inbounds nuw i8, ptr %1, i64 8
  %9 = load i64, ptr %8, align 8
  %10 = getelementptr inbounds nuw i8, ptr %0, i64 8
  %11 = load i64, ptr %10, align 8
  %12 = sub nsw i64 %9, %11
  %13 = add nsw i64 %7, %12
  ret i64 %13
}

declare void @qsort(ptr noundef, i64 noundef, i64 noundef, ptr noundef) #3

; Function Attrs: noinline nounwind uwtable
define internal i32 @cmp_ll(ptr noundef %0, ptr noundef %1) #0 {
  %3 = load i64, ptr %0, align 8
  %4 = load i64, ptr %1, align 8
  %5 = tail call i32 @llvm.scmp.i32.i64(i64 %3, i64 %4)
  ret i32 %5
}

declare i32 @printf(ptr noundef, ...) #3

; Function Attrs: nounwind
declare void @free(ptr noundef) #2

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #4

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare double @llvm.fmuladd.f64(double, double, double) #5

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i32 @llvm.scmp.i32.i64(i64, i64) #5

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind allocsize(0) "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { nocallback nofree nounwind willreturn memory(argmem: write) }
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
!13 = distinct !{!13, !7}
!14 = distinct !{!14, !7}
!15 = distinct !{!15, !7}
