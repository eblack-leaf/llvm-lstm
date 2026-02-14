; ModuleID = 'data/exploratory/_work/json_tokenizer.ll'
source_filename = "benchmarks/json_tokenizer.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

%struct.timespec = type { i64, i64 }

@.str = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@lcg_state = internal global i32 12345, align 4
@json_buf = internal global [102401 x i8] zeroinitializer, align 16
@.str.1 = private unnamed_addr constant [3 x i8] c"%d\00", align 1
@.str.2 = private unnamed_addr constant [5 x i8] c"null\00", align 1
@.str.3 = private unnamed_addr constant [5 x i8] c"true\00", align 1
@.str.4 = private unnamed_addr constant [6 x i8] c"false\00", align 1
@json_len = internal global i32 0, align 4
@token_count = internal global i32 0, align 4

; Function Attrs: noinline nounwind uwtable
define dso_local i32 @main() #0 {
  %1 = alloca i32, align 4
  %2 = alloca [50 x i64], align 16
  %3 = alloca i32, align 4
  %4 = alloca %struct.timespec, align 8
  %5 = alloca %struct.timespec, align 8
  call void @generate_json()
  store i32 0, ptr %1, align 4
  br label %6

6:                                                ; preds = %9, %0
  %7 = phi i32 [ %11, %9 ], [ 0, %0 ]
  %8 = icmp slt i32 %7, 5
  br i1 %8, label %9, label %12

9:                                                ; preds = %6
  call void @do_tokenize()
  %10 = load i32, ptr %1, align 4
  %11 = add nsw i32 %10, 1
  store i32 %11, ptr %1, align 4
  br label %6, !llvm.loop !6

12:                                               ; preds = %6
  store i32 0, ptr %3, align 4
  br label %13

13:                                               ; preds = %16, %12
  %14 = phi i32 [ %24, %16 ], [ 0, %12 ]
  %15 = icmp slt i32 %14, 50
  br i1 %15, label %16, label %25

16:                                               ; preds = %13
  %17 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %4) #3
  call void @do_tokenize()
  %18 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %5) #3
  %19 = call i64 @timespec_diff_ns(ptr noundef %4, ptr noundef %5)
  %20 = load i32, ptr %3, align 4
  %21 = sext i32 %20 to i64
  %22 = getelementptr inbounds [50 x i64], ptr %2, i64 0, i64 %21
  store i64 %19, ptr %22, align 8
  %23 = load i32, ptr %3, align 4
  %24 = add nsw i32 %23, 1
  store i32 %24, ptr %3, align 4
  br label %13, !llvm.loop !8

25:                                               ; preds = %13
  %26 = getelementptr inbounds [50 x i64], ptr %2, i64 0, i64 0
  call void @qsort(ptr noundef %26, i64 noundef 50, i64 noundef 8, ptr noundef @cmp_ll)
  %27 = getelementptr inbounds [50 x i64], ptr %2, i64 0, i64 25
  %28 = load i64, ptr %27, align 8
  %29 = call i32 (ptr, ...) @printf(ptr noundef @.str, i64 noundef %28)
  ret i32 0
}

; Function Attrs: noinline nounwind uwtable
define internal void @generate_json() #0 {
  %1 = alloca i32, align 4
  %2 = alloca i32, align 4
  %3 = alloca i32, align 4
  %4 = alloca i32, align 4
  %5 = alloca i32, align 4
  %6 = alloca i32, align 4
  %7 = alloca i32, align 4
  %8 = alloca i32, align 4
  %9 = alloca [16 x i8], align 16
  %10 = alloca i32, align 4
  %11 = alloca i32, align 4
  %12 = alloca i32, align 4
  %13 = alloca i32, align 4
  %14 = alloca i32, align 4
  %15 = alloca [16 x i8], align 16
  %16 = alloca i32, align 4
  %17 = alloca i32, align 4
  %18 = alloca i32, align 4
  %19 = alloca ptr, align 8
  %20 = alloca ptr, align 8
  %21 = alloca ptr, align 8
  store i32 0, ptr %1, align 4
  store i32 0, ptr %2, align 4
  store i32 12345, ptr @lcg_state, align 4
  %22 = load i32, ptr %1, align 4
  %23 = icmp slt i32 %22, 102400
  br i1 %23, label %24, label %29

24:                                               ; preds = %0
  %25 = load i32, ptr %1, align 4
  %26 = add nsw i32 %25, 1
  store i32 %26, ptr %1, align 4
  %27 = sext i32 %25 to i64
  %28 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %27
  store i8 123, ptr %28, align 1
  br label %29

29:                                               ; preds = %0, %24
  %30 = load i32, ptr %2, align 4
  %31 = add nsw i32 %30, 1
  store i32 %31, ptr %2, align 4
  store i32 0, ptr %3, align 4
  br label %32

32:                                               ; preds = %339, %29
  %33 = load i32, ptr %1, align 4
  %34 = icmp slt i32 %33, 102200
  br i1 %34, label %35, label %.thread

35:                                               ; preds = %32
  %36 = load i32, ptr %2, align 4
  %37 = icmp sgt i32 %36, 0
  br i1 %37, label %38, label %.thread

38:                                               ; preds = %35
  %39 = load i32, ptr %3, align 4
  %40 = icmp sgt i32 %39, 0
  br i1 %40, label %41, label %thread-pre-split

41:                                               ; preds = %38
  %42 = load i32, ptr %1, align 4
  %43 = icmp slt i32 %42, 102400
  br i1 %43, label %46, label %.thread1

.thread1:                                         ; preds = %41
  %44 = load i32, ptr %3, align 4
  %45 = add nsw i32 %44, 1
  store i32 %45, ptr %3, align 4
  br label %61

46:                                               ; preds = %41
  %47 = load i32, ptr %1, align 4
  %48 = add nsw i32 %47, 1
  store i32 %48, ptr %1, align 4
  %49 = sext i32 %47 to i64
  %50 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %49
  store i8 44, ptr %50, align 1
  br label %51

thread-pre-split:                                 ; preds = %38
  %.pr = load i32, ptr %1, align 4
  br label %51

51:                                               ; preds = %thread-pre-split, %46
  %52 = phi i32 [ %.pr, %thread-pre-split ], [ %48, %46 ]
  %53 = load i32, ptr %3, align 4
  %54 = add nsw i32 %53, 1
  store i32 %54, ptr %3, align 4
  %55 = icmp slt i32 %52, 102400
  br i1 %55, label %56, label %61

56:                                               ; preds = %51
  %57 = load i32, ptr %1, align 4
  %58 = add nsw i32 %57, 1
  store i32 %58, ptr %1, align 4
  %59 = sext i32 %57 to i64
  %60 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %59
  store i8 34, ptr %60, align 1
  br label %61

61:                                               ; preds = %51, %56, %.thread1
  %62 = call i32 @lcg_rand()
  %63 = urem i32 %62, 8
  %64 = add i32 3, %63
  store i32 %64, ptr %4, align 4
  store i32 0, ptr %5, align 4
  br label %65

65:                                               ; preds = %86, %61
  %66 = load i32, ptr %5, align 4
  %67 = load i32, ptr %4, align 4
  %68 = icmp slt i32 %66, %67
  br i1 %68, label %69, label %89

69:                                               ; preds = %65
  %70 = load i32, ptr %1, align 4
  %71 = icmp slt i32 %70, 102400
  br i1 %71, label %72, label %.thread5

72:                                               ; preds = %69
  %73 = load i32, ptr %1, align 4
  %74 = icmp slt i32 %73, 102400
  br i1 %74, label %75, label %86

75:                                               ; preds = %72
  %76 = call i32 @lcg_rand()
  %77 = urem i32 %76, 26
  %78 = trunc i32 %77 to i8
  %79 = sext i8 %78 to i32
  %80 = add nsw i32 97, %79
  %81 = trunc i32 %80 to i8
  %82 = load i32, ptr %1, align 4
  %83 = add nsw i32 %82, 1
  store i32 %83, ptr %1, align 4
  %84 = sext i32 %82 to i64
  %85 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %84
  store i8 %81, ptr %85, align 1
  br label %86

86:                                               ; preds = %75, %72
  %87 = load i32, ptr %5, align 4
  %88 = add nsw i32 %87, 1
  store i32 %88, ptr %5, align 4
  br label %65, !llvm.loop !9

89:                                               ; preds = %65
  %.pr3 = load i32, ptr %1, align 4
  %90 = icmp slt i32 %.pr3, 102400
  br i1 %90, label %91, label %.thread5

91:                                               ; preds = %89
  %92 = load i32, ptr %1, align 4
  %93 = add nsw i32 %92, 1
  store i32 %93, ptr %1, align 4
  %94 = sext i32 %92 to i64
  %95 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %94
  store i8 34, ptr %95, align 1
  %96 = icmp slt i32 %93, 102400
  br i1 %96, label %97, label %.thread5

97:                                               ; preds = %91
  %98 = load i32, ptr %1, align 4
  %99 = add nsw i32 %98, 1
  store i32 %99, ptr %1, align 4
  %100 = sext i32 %98 to i64
  %101 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %100
  store i8 58, ptr %101, align 1
  br label %.thread5

.thread5:                                         ; preds = %69, %89, %91, %97
  %102 = call i32 @lcg_rand()
  %103 = urem i32 %102, 10
  store i32 %103, ptr %6, align 4
  %104 = icmp slt i32 %103, 3
  br i1 %104, label %105, label %122

105:                                              ; preds = %.thread5
  %106 = load i32, ptr %2, align 4
  %107 = icmp slt i32 %106, 5
  br i1 %107, label %108, label %.thread11.thread

108:                                              ; preds = %105
  %109 = load i32, ptr %1, align 4
  %110 = icmp slt i32 %109, 101900
  br i1 %110, label %111, label %thread-pre-split6

111:                                              ; preds = %108
  %112 = load i32, ptr %1, align 4
  %113 = icmp slt i32 %112, 102400
  br i1 %113, label %114, label %119

114:                                              ; preds = %111
  %115 = load i32, ptr %1, align 4
  %116 = add nsw i32 %115, 1
  store i32 %116, ptr %1, align 4
  %117 = sext i32 %115 to i64
  %118 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %117
  store i8 123, ptr %118, align 1
  br label %119

119:                                              ; preds = %111, %114
  %120 = load i32, ptr %2, align 4
  %121 = add nsw i32 %120, 1
  store i32 %121, ptr %2, align 4
  store i32 0, ptr %3, align 4
  br label %321

thread-pre-split6:                                ; preds = %108
  %.pr7 = load i32, ptr %6, align 4
  br label %122

122:                                              ; preds = %thread-pre-split6, %.thread5
  %123 = phi i32 [ %.pr7, %thread-pre-split6 ], [ %103, %.thread5 ]
  %124 = icmp slt i32 %123, 5
  br i1 %124, label %125, label %.thread11

125:                                              ; preds = %122
  %.pr10 = load i32, ptr %2, align 4
  %126 = icmp slt i32 %.pr10, 5
  br i1 %126, label %127, label %.thread11

127:                                              ; preds = %125
  %128 = load i32, ptr %1, align 4
  %129 = icmp slt i32 %128, 101900
  br i1 %129, label %130, label %.thread11

130:                                              ; preds = %127
  %131 = load i32, ptr %1, align 4
  %132 = icmp slt i32 %131, 102400
  br i1 %132, label %133, label %138

133:                                              ; preds = %130
  %134 = load i32, ptr %1, align 4
  %135 = add nsw i32 %134, 1
  store i32 %135, ptr %1, align 4
  %136 = sext i32 %134 to i64
  %137 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %136
  store i8 91, ptr %137, align 1
  br label %138

138:                                              ; preds = %130, %133
  %139 = call i32 @lcg_rand()
  %140 = urem i32 %139, 6
  %141 = add i32 2, %140
  store i32 %141, ptr %7, align 4
  store i32 0, ptr %8, align 4
  br label %142

142:                                              ; preds = %182, %138
  %143 = load i32, ptr %8, align 4
  %144 = load i32, ptr %7, align 4
  %145 = icmp slt i32 %143, %144
  br i1 %145, label %146, label %185

146:                                              ; preds = %142
  %147 = load i32, ptr %8, align 4
  %148 = icmp sgt i32 %147, 0
  br i1 %148, label %149, label %157

149:                                              ; preds = %146
  %150 = load i32, ptr %1, align 4
  %151 = icmp slt i32 %150, 102400
  br i1 %151, label %152, label %157

152:                                              ; preds = %149
  %153 = load i32, ptr %1, align 4
  %154 = add nsw i32 %153, 1
  store i32 %154, ptr %1, align 4
  %155 = sext i32 %153 to i64
  %156 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %155
  store i8 44, ptr %156, align 1
  br label %157

157:                                              ; preds = %152, %149, %146
  %158 = call i32 @lcg_rand()
  %159 = urem i32 %158, 10000
  store i32 %159, ptr %10, align 4
  %160 = getelementptr inbounds [16 x i8], ptr %9, i64 0, i64 0
  %161 = load i32, ptr %10, align 4
  %162 = call i32 (ptr, ptr, ...) @sprintf(ptr noundef %160, ptr noundef @.str.1, i32 noundef %161) #3
  store i32 %162, ptr %11, align 4
  store i32 0, ptr %12, align 4
  br label %163

163:                                              ; preds = %179, %157
  %164 = load i32, ptr %12, align 4
  %165 = load i32, ptr %11, align 4
  %166 = icmp slt i32 %164, %165
  br i1 %166, label %167, label %182

167:                                              ; preds = %163
  %168 = load i32, ptr %1, align 4
  %169 = icmp slt i32 %168, 102400
  br i1 %169, label %170, label %179

170:                                              ; preds = %167
  %171 = load i32, ptr %12, align 4
  %172 = sext i32 %171 to i64
  %173 = getelementptr inbounds [16 x i8], ptr %9, i64 0, i64 %172
  %174 = load i8, ptr %173, align 1
  %175 = load i32, ptr %1, align 4
  %176 = add nsw i32 %175, 1
  store i32 %176, ptr %1, align 4
  %177 = sext i32 %175 to i64
  %178 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %177
  store i8 %174, ptr %178, align 1
  br label %179

179:                                              ; preds = %170, %167
  %180 = load i32, ptr %12, align 4
  %181 = add nsw i32 %180, 1
  store i32 %181, ptr %12, align 4
  br label %163, !llvm.loop !10

182:                                              ; preds = %163
  %183 = load i32, ptr %8, align 4
  %184 = add nsw i32 %183, 1
  store i32 %184, ptr %8, align 4
  br label %142, !llvm.loop !11

185:                                              ; preds = %142
  %186 = load i32, ptr %1, align 4
  %187 = icmp slt i32 %186, 102400
  br i1 %187, label %188, label %thread-pre-split15

188:                                              ; preds = %185
  %189 = load i32, ptr %1, align 4
  %190 = add nsw i32 %189, 1
  store i32 %190, ptr %1, align 4
  %191 = sext i32 %189 to i64
  %192 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %191
  store i8 93, ptr %192, align 1
  br label %thread-pre-split15

.thread11:                                        ; preds = %127, %125, %122
  %.pr18 = load i32, ptr %6, align 4
  %193 = icmp slt i32 %.pr18, 7
  br i1 %193, label %.thread11.thread, label %234

.thread11.thread:                                 ; preds = %105, %.thread11
  %194 = load i32, ptr %1, align 4
  %195 = icmp slt i32 %194, 102400
  br i1 %195, label %196, label %201

196:                                              ; preds = %.thread11.thread
  %197 = load i32, ptr %1, align 4
  %198 = add nsw i32 %197, 1
  store i32 %198, ptr %1, align 4
  %199 = sext i32 %197 to i64
  %200 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %199
  store i8 34, ptr %200, align 1
  br label %201

201:                                              ; preds = %.thread11.thread, %196
  %202 = call i32 @lcg_rand()
  %203 = urem i32 %202, 12
  %204 = add i32 3, %203
  store i32 %204, ptr %13, align 4
  store i32 0, ptr %14, align 4
  br label %205

205:                                              ; preds = %223, %201
  %206 = load i32, ptr %14, align 4
  %207 = load i32, ptr %13, align 4
  %208 = icmp slt i32 %206, %207
  br i1 %208, label %209, label %226

209:                                              ; preds = %205
  %210 = load i32, ptr %1, align 4
  %211 = icmp slt i32 %210, 102400
  br i1 %211, label %212, label %223

212:                                              ; preds = %209
  %213 = call i32 @lcg_rand()
  %214 = urem i32 %213, 26
  %215 = trunc i32 %214 to i8
  %216 = sext i8 %215 to i32
  %217 = add nsw i32 97, %216
  %218 = trunc i32 %217 to i8
  %219 = load i32, ptr %1, align 4
  %220 = add nsw i32 %219, 1
  store i32 %220, ptr %1, align 4
  %221 = sext i32 %219 to i64
  %222 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %221
  store i8 %218, ptr %222, align 1
  br label %223

223:                                              ; preds = %212, %209
  %224 = load i32, ptr %14, align 4
  %225 = add nsw i32 %224, 1
  store i32 %225, ptr %14, align 4
  br label %205, !llvm.loop !12

226:                                              ; preds = %205
  %227 = load i32, ptr %1, align 4
  %228 = icmp slt i32 %227, 102400
  br i1 %228, label %229, label %thread-pre-split15

229:                                              ; preds = %226
  %230 = load i32, ptr %1, align 4
  %231 = add nsw i32 %230, 1
  store i32 %231, ptr %1, align 4
  %232 = sext i32 %230 to i64
  %233 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %232
  store i8 34, ptr %233, align 1
  br label %thread-pre-split15

234:                                              ; preds = %.thread11
  %235 = load i32, ptr %6, align 4
  %236 = icmp slt i32 %235, 9
  br i1 %236, label %237, label %262

237:                                              ; preds = %234
  %238 = call i32 @lcg_rand()
  %239 = urem i32 %238, 100000
  store i32 %239, ptr %16, align 4
  %240 = getelementptr inbounds [16 x i8], ptr %15, i64 0, i64 0
  %241 = load i32, ptr %16, align 4
  %242 = call i32 (ptr, ptr, ...) @sprintf(ptr noundef %240, ptr noundef @.str.1, i32 noundef %241) #3
  store i32 %242, ptr %17, align 4
  store i32 0, ptr %18, align 4
  br label %243

243:                                              ; preds = %259, %237
  %244 = load i32, ptr %18, align 4
  %245 = load i32, ptr %17, align 4
  %246 = icmp slt i32 %244, %245
  br i1 %246, label %247, label %thread-pre-split15

247:                                              ; preds = %243
  %248 = load i32, ptr %1, align 4
  %249 = icmp slt i32 %248, 102400
  br i1 %249, label %250, label %259

250:                                              ; preds = %247
  %251 = load i32, ptr %18, align 4
  %252 = sext i32 %251 to i64
  %253 = getelementptr inbounds [16 x i8], ptr %15, i64 0, i64 %252
  %254 = load i8, ptr %253, align 1
  %255 = load i32, ptr %1, align 4
  %256 = add nsw i32 %255, 1
  store i32 %256, ptr %1, align 4
  %257 = sext i32 %255 to i64
  %258 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %257
  store i8 %254, ptr %258, align 1
  br label %259

259:                                              ; preds = %250, %247
  %260 = load i32, ptr %18, align 4
  %261 = add nsw i32 %260, 1
  store i32 %261, ptr %18, align 4
  br label %243, !llvm.loop !13

262:                                              ; preds = %234
  %263 = call i32 @lcg_rand()
  %264 = urem i32 %263, 3
  %265 = icmp eq i32 %264, 0
  br i1 %265, label %266, label %283

266:                                              ; preds = %262
  store ptr @.str.2, ptr %19, align 8
  br label %267

267:                                              ; preds = %275, %266
  %268 = load ptr, ptr %19, align 8
  %269 = load i8, ptr %268, align 1
  %270 = sext i8 %269 to i32
  %271 = icmp ne i32 %270, 0
  br i1 %271, label %272, label %thread-pre-split15

272:                                              ; preds = %267
  %273 = load i32, ptr %1, align 4
  %274 = icmp slt i32 %273, 102400
  br i1 %274, label %275, label %thread-pre-split15

275:                                              ; preds = %272
  %276 = load ptr, ptr %19, align 8
  %277 = getelementptr inbounds nuw i8, ptr %276, i32 1
  store ptr %277, ptr %19, align 8
  %278 = load i8, ptr %276, align 1
  %279 = load i32, ptr %1, align 4
  %280 = add nsw i32 %279, 1
  store i32 %280, ptr %1, align 4
  %281 = sext i32 %279 to i64
  %282 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %281
  store i8 %278, ptr %282, align 1
  br label %267, !llvm.loop !14

283:                                              ; preds = %262
  %284 = call i32 @lcg_rand()
  %285 = urem i32 %284, 2
  %286 = icmp ne i32 %285, 0
  br i1 %286, label %287, label %304

287:                                              ; preds = %283
  store ptr @.str.3, ptr %20, align 8
  br label %288

288:                                              ; preds = %296, %287
  %289 = load ptr, ptr %20, align 8
  %290 = load i8, ptr %289, align 1
  %291 = sext i8 %290 to i32
  %292 = icmp ne i32 %291, 0
  br i1 %292, label %293, label %thread-pre-split15

293:                                              ; preds = %288
  %294 = load i32, ptr %1, align 4
  %295 = icmp slt i32 %294, 102400
  br i1 %295, label %296, label %thread-pre-split15

296:                                              ; preds = %293
  %297 = load ptr, ptr %20, align 8
  %298 = getelementptr inbounds nuw i8, ptr %297, i32 1
  store ptr %298, ptr %20, align 8
  %299 = load i8, ptr %297, align 1
  %300 = load i32, ptr %1, align 4
  %301 = add nsw i32 %300, 1
  store i32 %301, ptr %1, align 4
  %302 = sext i32 %300 to i64
  %303 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %302
  store i8 %299, ptr %303, align 1
  br label %288, !llvm.loop !15

304:                                              ; preds = %283
  store ptr @.str.4, ptr %21, align 8
  br label %305

305:                                              ; preds = %313, %304
  %306 = load ptr, ptr %21, align 8
  %307 = load i8, ptr %306, align 1
  %308 = sext i8 %307 to i32
  %309 = icmp ne i32 %308, 0
  br i1 %309, label %310, label %thread-pre-split15

310:                                              ; preds = %305
  %311 = load i32, ptr %1, align 4
  %312 = icmp slt i32 %311, 102400
  br i1 %312, label %313, label %thread-pre-split15

313:                                              ; preds = %310
  %314 = load ptr, ptr %21, align 8
  %315 = getelementptr inbounds nuw i8, ptr %314, i32 1
  store ptr %315, ptr %21, align 8
  %316 = load i8, ptr %314, align 1
  %317 = load i32, ptr %1, align 4
  %318 = add nsw i32 %317, 1
  store i32 %318, ptr %1, align 4
  %319 = sext i32 %317 to i64
  %320 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %319
  store i8 %316, ptr %320, align 1
  br label %305, !llvm.loop !16

thread-pre-split15:                               ; preds = %305, %288, %267, %226, %229, %272, %310, %293, %243, %188, %185
  %.pr16 = load i32, ptr %2, align 4
  br label %321

321:                                              ; preds = %thread-pre-split15, %119
  %322 = phi i32 [ %.pr16, %thread-pre-split15 ], [ %121, %119 ]
  %323 = icmp sgt i32 %322, 1
  br i1 %323, label %324, label %339

324:                                              ; preds = %321
  %325 = call i32 @lcg_rand()
  %326 = urem i32 %325, 4
  %327 = icmp eq i32 %326, 0
  br i1 %327, label %328, label %339

328:                                              ; preds = %324
  %329 = load i32, ptr %1, align 4
  %330 = icmp slt i32 %329, 102400
  br i1 %330, label %331, label %336

331:                                              ; preds = %328
  %332 = load i32, ptr %1, align 4
  %333 = add nsw i32 %332, 1
  store i32 %333, ptr %1, align 4
  %334 = sext i32 %332 to i64
  %335 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %334
  store i8 125, ptr %335, align 1
  br label %336

336:                                              ; preds = %328, %331
  %337 = load i32, ptr %2, align 4
  %338 = add nsw i32 %337, -1
  store i32 %338, ptr %2, align 4
  br label %339

339:                                              ; preds = %336, %324, %321
  br label %32, !llvm.loop !17

.thread:                                          ; preds = %32, %35
  %.pr17 = load i32, ptr %2, align 4
  br label %340

340:                                              ; preds = %351, %.thread
  %341 = phi i32 [ %353, %351 ], [ %.pr17, %.thread ]
  %342 = icmp sgt i32 %341, 0
  br i1 %342, label %343, label %354

343:                                              ; preds = %340
  %344 = load i32, ptr %1, align 4
  %345 = icmp slt i32 %344, 102400
  br i1 %345, label %346, label %351

346:                                              ; preds = %343
  %347 = load i32, ptr %1, align 4
  %348 = add nsw i32 %347, 1
  store i32 %348, ptr %1, align 4
  %349 = sext i32 %347 to i64
  %350 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %349
  store i8 125, ptr %350, align 1
  br label %351

351:                                              ; preds = %343, %346
  %352 = load i32, ptr %2, align 4
  %353 = add nsw i32 %352, -1
  store i32 %353, ptr %2, align 4
  br label %340, !llvm.loop !18

354:                                              ; preds = %340
  %355 = load i32, ptr %1, align 4
  %356 = sext i32 %355 to i64
  %357 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %356
  store i8 0, ptr %357, align 1
  %358 = load i32, ptr %1, align 4
  store i32 %358, ptr @json_len, align 4
  ret void
}

; Function Attrs: noinline nounwind uwtable
define internal void @do_tokenize() #0 {
  %1 = alloca i32, align 4
  %2 = alloca i32, align 4
  %3 = alloca i8, align 1
  store i32 0, ptr %1, align 4
  store i32 0, ptr %2, align 4
  br label %4

4:                                                ; preds = %.thread2, %0
  %5 = load i32, ptr %2, align 4
  %6 = load i32, ptr @json_len, align 4
  %7 = icmp slt i32 %5, %6
  br i1 %7, label %8, label %150

8:                                                ; preds = %4
  %9 = load i32, ptr %2, align 4
  %10 = sext i32 %9 to i64
  %11 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %10
  %12 = load i8, ptr %11, align 1
  store i8 %12, ptr %3, align 1
  %13 = load i8, ptr %3, align 1
  %14 = sext i8 %13 to i32
  %15 = icmp eq i32 %14, 123
  br i1 %15, label %36, label %16

16:                                               ; preds = %8
  %17 = load i8, ptr %3, align 1
  %18 = sext i8 %17 to i32
  %19 = icmp eq i32 %18, 125
  br i1 %19, label %36, label %20

20:                                               ; preds = %16
  %21 = load i8, ptr %3, align 1
  %22 = sext i8 %21 to i32
  %23 = icmp eq i32 %22, 91
  br i1 %23, label %36, label %24

24:                                               ; preds = %20
  %25 = load i8, ptr %3, align 1
  %26 = sext i8 %25 to i32
  %27 = icmp eq i32 %26, 93
  br i1 %27, label %36, label %28

28:                                               ; preds = %24
  %29 = load i8, ptr %3, align 1
  %30 = sext i8 %29 to i32
  %31 = icmp eq i32 %30, 58
  br i1 %31, label %36, label %32

32:                                               ; preds = %28
  %33 = load i8, ptr %3, align 1
  %34 = sext i8 %33 to i32
  %35 = icmp eq i32 %34, 44
  br i1 %35, label %36, label %41

36:                                               ; preds = %32, %28, %24, %20, %16, %8
  %37 = load i32, ptr %1, align 4
  %38 = add nsw i32 %37, 1
  store i32 %38, ptr %1, align 4
  %39 = load i32, ptr %2, align 4
  %40 = add nsw i32 %39, 1
  store i32 %40, ptr %2, align 4
  br label %.thread2

41:                                               ; preds = %32
  %42 = load i8, ptr %3, align 1
  %43 = sext i8 %42 to i32
  %44 = icmp eq i32 %43, 34
  br i1 %44, label %45, label %76

45:                                               ; preds = %41
  %46 = load i32, ptr %1, align 4
  %47 = add nsw i32 %46, 1
  store i32 %47, ptr %1, align 4
  %48 = load i32, ptr %2, align 4
  %49 = add nsw i32 %48, 1
  store i32 %49, ptr %2, align 4
  br label %50

50:                                               ; preds = %71, %45
  %51 = load i32, ptr %2, align 4
  %52 = load i32, ptr @json_len, align 4
  %53 = icmp slt i32 %51, %52
  br i1 %53, label %54, label %.thread

54:                                               ; preds = %50
  %55 = load i32, ptr %2, align 4
  %56 = sext i32 %55 to i64
  %57 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %56
  %58 = load i8, ptr %57, align 1
  %59 = sext i8 %58 to i32
  %60 = icmp ne i32 %59, 34
  br i1 %60, label %61, label %.thread

61:                                               ; preds = %54
  %62 = load i32, ptr %2, align 4
  %63 = sext i32 %62 to i64
  %64 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %63
  %65 = load i8, ptr %64, align 1
  %66 = sext i8 %65 to i32
  %67 = icmp eq i32 %66, 92
  br i1 %67, label %68, label %71

68:                                               ; preds = %61
  %69 = load i32, ptr %2, align 4
  %70 = add nsw i32 %69, 1
  store i32 %70, ptr %2, align 4
  br label %71

71:                                               ; preds = %68, %61
  %72 = load i32, ptr %2, align 4
  %73 = add nsw i32 %72, 1
  store i32 %73, ptr %2, align 4
  br label %50, !llvm.loop !19

.thread:                                          ; preds = %50, %54
  %74 = load i32, ptr %2, align 4
  %75 = add nsw i32 %74, 1
  store i32 %75, ptr %2, align 4
  br label %.thread2

76:                                               ; preds = %41
  %77 = load i8, ptr %3, align 1
  %78 = sext i8 %77 to i32
  %79 = icmp sge i32 %78, 48
  br i1 %79, label %80, label %84

80:                                               ; preds = %76
  %81 = load i8, ptr %3, align 1
  %82 = sext i8 %81 to i32
  %83 = icmp sle i32 %82, 57
  br i1 %83, label %88, label %84

84:                                               ; preds = %80, %76
  %85 = load i8, ptr %3, align 1
  %86 = sext i8 %85 to i32
  %87 = icmp eq i32 %86, 45
  br i1 %87, label %88, label %120

88:                                               ; preds = %84, %80
  %89 = load i32, ptr %1, align 4
  %90 = add nsw i32 %89, 1
  store i32 %90, ptr %1, align 4
  %91 = load i32, ptr %2, align 4
  %92 = add nsw i32 %91, 1
  store i32 %92, ptr %2, align 4
  br label %93

93:                                               ; preds = %.thread1, %88
  %94 = load i32, ptr %2, align 4
  %95 = load i32, ptr @json_len, align 4
  %96 = icmp slt i32 %94, %95
  br i1 %96, label %97, label %.thread2

97:                                               ; preds = %93
  %98 = load i32, ptr %2, align 4
  %99 = sext i32 %98 to i64
  %100 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %99
  %101 = load i8, ptr %100, align 1
  %102 = sext i8 %101 to i32
  %103 = icmp sge i32 %102, 48
  br i1 %103, label %104, label %111

104:                                              ; preds = %97
  %105 = load i32, ptr %2, align 4
  %106 = sext i32 %105 to i64
  %107 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %106
  %108 = load i8, ptr %107, align 1
  %109 = sext i8 %108 to i32
  %110 = icmp sle i32 %109, 57
  br i1 %110, label %.thread1, label %111

111:                                              ; preds = %97, %104
  %112 = load i32, ptr %2, align 4
  %113 = sext i32 %112 to i64
  %114 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %113
  %115 = load i8, ptr %114, align 1
  %116 = sext i8 %115 to i32
  %117 = icmp eq i32 %116, 46
  br i1 %117, label %.thread1, label %.thread2

.thread1:                                         ; preds = %104, %111
  %118 = load i32, ptr %2, align 4
  %119 = add nsw i32 %118, 1
  store i32 %119, ptr %2, align 4
  br label %93, !llvm.loop !20

120:                                              ; preds = %84
  %121 = load i8, ptr %3, align 1
  %122 = sext i8 %121 to i32
  %123 = icmp eq i32 %122, 116
  br i1 %123, label %124, label %129

124:                                              ; preds = %120
  %125 = load i32, ptr %1, align 4
  %126 = add nsw i32 %125, 1
  store i32 %126, ptr %1, align 4
  %127 = load i32, ptr %2, align 4
  %128 = add nsw i32 %127, 4
  store i32 %128, ptr %2, align 4
  br label %.thread2

129:                                              ; preds = %120
  %130 = load i8, ptr %3, align 1
  %131 = sext i8 %130 to i32
  %132 = icmp eq i32 %131, 102
  br i1 %132, label %133, label %138

133:                                              ; preds = %129
  %134 = load i32, ptr %1, align 4
  %135 = add nsw i32 %134, 1
  store i32 %135, ptr %1, align 4
  %136 = load i32, ptr %2, align 4
  %137 = add nsw i32 %136, 5
  store i32 %137, ptr %2, align 4
  br label %.thread2

138:                                              ; preds = %129
  %139 = load i8, ptr %3, align 1
  %140 = sext i8 %139 to i32
  %141 = icmp eq i32 %140, 110
  br i1 %141, label %142, label %147

142:                                              ; preds = %138
  %143 = load i32, ptr %1, align 4
  %144 = add nsw i32 %143, 1
  store i32 %144, ptr %1, align 4
  %145 = load i32, ptr %2, align 4
  %146 = add nsw i32 %145, 4
  store i32 %146, ptr %2, align 4
  br label %.thread2

147:                                              ; preds = %138
  %148 = load i32, ptr %2, align 4
  %149 = add nsw i32 %148, 1
  store i32 %149, ptr %2, align 4
  br label %.thread2

.thread2:                                         ; preds = %93, %.thread, %124, %142, %147, %133, %111, %36
  br label %4, !llvm.loop !21

150:                                              ; preds = %4
  %151 = load i32, ptr %1, align 4
  store volatile i32 %151, ptr @token_count, align 4
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

attributes #0 = { noinline nounwind uwtable "frame-pointer"="all" "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { nounwind "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { "frame-pointer"="all" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nounwind }

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
!16 = distinct !{!16, !7}
!17 = distinct !{!17, !7}
!18 = distinct !{!18, !7}
!19 = distinct !{!19, !7}
!20 = distinct !{!20, !7}
!21 = distinct !{!21, !7}
