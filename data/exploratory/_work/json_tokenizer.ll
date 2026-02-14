; ModuleID = 'benchmarks/json_tokenizer.c'
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
  %2 = alloca i32, align 4
  %3 = alloca [50 x i64], align 16
  %4 = alloca i32, align 4
  %5 = alloca %struct.timespec, align 8
  %6 = alloca %struct.timespec, align 8
  store i32 0, ptr %1, align 4
  call void @generate_json()
  store i32 0, ptr %2, align 4
  br label %7

7:                                                ; preds = %11, %0
  %8 = load i32, ptr %2, align 4
  %9 = icmp slt i32 %8, 5
  br i1 %9, label %10, label %14

10:                                               ; preds = %7
  call void @do_tokenize()
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
  %19 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %5) #3
  call void @do_tokenize()
  %20 = call i32 @clock_gettime(i32 noundef 1, ptr noundef %6) #3
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
  br label %22

22:                                               ; preds = %0
  %23 = load i32, ptr %1, align 4
  %24 = icmp slt i32 %23, 102400
  br i1 %24, label %25, label %30

25:                                               ; preds = %22
  %26 = load i32, ptr %1, align 4
  %27 = add nsw i32 %26, 1
  store i32 %27, ptr %1, align 4
  %28 = sext i32 %26 to i64
  %29 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %28
  store i8 123, ptr %29, align 1
  br label %30

30:                                               ; preds = %25, %22
  br label %31

31:                                               ; preds = %30
  %32 = load i32, ptr %2, align 4
  %33 = add nsw i32 %32, 1
  store i32 %33, ptr %2, align 4
  store i32 0, ptr %3, align 4
  br label %34

34:                                               ; preds = %412, %31
  %35 = load i32, ptr %1, align 4
  %36 = icmp slt i32 %35, 102200
  br i1 %36, label %37, label %40

37:                                               ; preds = %34
  %38 = load i32, ptr %2, align 4
  %39 = icmp sgt i32 %38, 0
  br label %40

40:                                               ; preds = %37, %34
  %41 = phi i1 [ false, %34 ], [ %39, %37 ]
  br i1 %41, label %42, label %413

42:                                               ; preds = %40
  %43 = load i32, ptr %3, align 4
  %44 = icmp sgt i32 %43, 0
  br i1 %44, label %45, label %56

45:                                               ; preds = %42
  br label %46

46:                                               ; preds = %45
  %47 = load i32, ptr %1, align 4
  %48 = icmp slt i32 %47, 102400
  br i1 %48, label %49, label %54

49:                                               ; preds = %46
  %50 = load i32, ptr %1, align 4
  %51 = add nsw i32 %50, 1
  store i32 %51, ptr %1, align 4
  %52 = sext i32 %50 to i64
  %53 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %52
  store i8 44, ptr %53, align 1
  br label %54

54:                                               ; preds = %49, %46
  br label %55

55:                                               ; preds = %54
  br label %56

56:                                               ; preds = %55, %42
  %57 = load i32, ptr %3, align 4
  %58 = add nsw i32 %57, 1
  store i32 %58, ptr %3, align 4
  br label %59

59:                                               ; preds = %56
  %60 = load i32, ptr %1, align 4
  %61 = icmp slt i32 %60, 102400
  br i1 %61, label %62, label %67

62:                                               ; preds = %59
  %63 = load i32, ptr %1, align 4
  %64 = add nsw i32 %63, 1
  store i32 %64, ptr %1, align 4
  %65 = sext i32 %63 to i64
  %66 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %65
  store i8 34, ptr %66, align 1
  br label %67

67:                                               ; preds = %62, %59
  br label %68

68:                                               ; preds = %67
  %69 = call i32 @lcg_rand()
  %70 = urem i32 %69, 8
  %71 = add i32 3, %70
  store i32 %71, ptr %4, align 4
  store i32 0, ptr %5, align 4
  br label %72

72:                                               ; preds = %98, %68
  %73 = load i32, ptr %5, align 4
  %74 = load i32, ptr %4, align 4
  %75 = icmp slt i32 %73, %74
  br i1 %75, label %76, label %79

76:                                               ; preds = %72
  %77 = load i32, ptr %1, align 4
  %78 = icmp slt i32 %77, 102400
  br label %79

79:                                               ; preds = %76, %72
  %80 = phi i1 [ false, %72 ], [ %78, %76 ]
  br i1 %80, label %81, label %101

81:                                               ; preds = %79
  br label %82

82:                                               ; preds = %81
  %83 = load i32, ptr %1, align 4
  %84 = icmp slt i32 %83, 102400
  br i1 %84, label %85, label %96

85:                                               ; preds = %82
  %86 = call i32 @lcg_rand()
  %87 = urem i32 %86, 26
  %88 = trunc i32 %87 to i8
  %89 = sext i8 %88 to i32
  %90 = add nsw i32 97, %89
  %91 = trunc i32 %90 to i8
  %92 = load i32, ptr %1, align 4
  %93 = add nsw i32 %92, 1
  store i32 %93, ptr %1, align 4
  %94 = sext i32 %92 to i64
  %95 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %94
  store i8 %91, ptr %95, align 1
  br label %96

96:                                               ; preds = %85, %82
  br label %97

97:                                               ; preds = %96
  br label %98

98:                                               ; preds = %97
  %99 = load i32, ptr %5, align 4
  %100 = add nsw i32 %99, 1
  store i32 %100, ptr %5, align 4
  br label %72, !llvm.loop !9

101:                                              ; preds = %79
  br label %102

102:                                              ; preds = %101
  %103 = load i32, ptr %1, align 4
  %104 = icmp slt i32 %103, 102400
  br i1 %104, label %105, label %110

105:                                              ; preds = %102
  %106 = load i32, ptr %1, align 4
  %107 = add nsw i32 %106, 1
  store i32 %107, ptr %1, align 4
  %108 = sext i32 %106 to i64
  %109 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %108
  store i8 34, ptr %109, align 1
  br label %110

110:                                              ; preds = %105, %102
  br label %111

111:                                              ; preds = %110
  br label %112

112:                                              ; preds = %111
  %113 = load i32, ptr %1, align 4
  %114 = icmp slt i32 %113, 102400
  br i1 %114, label %115, label %120

115:                                              ; preds = %112
  %116 = load i32, ptr %1, align 4
  %117 = add nsw i32 %116, 1
  store i32 %117, ptr %1, align 4
  %118 = sext i32 %116 to i64
  %119 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %118
  store i8 58, ptr %119, align 1
  br label %120

120:                                              ; preds = %115, %112
  br label %121

121:                                              ; preds = %120
  %122 = call i32 @lcg_rand()
  %123 = urem i32 %122, 10
  store i32 %123, ptr %6, align 4
  %124 = load i32, ptr %6, align 4
  %125 = icmp slt i32 %124, 3
  br i1 %125, label %126, label %145

126:                                              ; preds = %121
  %127 = load i32, ptr %2, align 4
  %128 = icmp slt i32 %127, 5
  br i1 %128, label %129, label %145

129:                                              ; preds = %126
  %130 = load i32, ptr %1, align 4
  %131 = icmp slt i32 %130, 101900
  br i1 %131, label %132, label %145

132:                                              ; preds = %129
  br label %133

133:                                              ; preds = %132
  %134 = load i32, ptr %1, align 4
  %135 = icmp slt i32 %134, 102400
  br i1 %135, label %136, label %141

136:                                              ; preds = %133
  %137 = load i32, ptr %1, align 4
  %138 = add nsw i32 %137, 1
  store i32 %138, ptr %1, align 4
  %139 = sext i32 %137 to i64
  %140 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %139
  store i8 123, ptr %140, align 1
  br label %141

141:                                              ; preds = %136, %133
  br label %142

142:                                              ; preds = %141
  %143 = load i32, ptr %2, align 4
  %144 = add nsw i32 %143, 1
  store i32 %144, ptr %2, align 4
  store i32 0, ptr %3, align 4
  br label %392

145:                                              ; preds = %129, %126, %121
  %146 = load i32, ptr %6, align 4
  %147 = icmp slt i32 %146, 5
  br i1 %147, label %148, label %229

148:                                              ; preds = %145
  %149 = load i32, ptr %2, align 4
  %150 = icmp slt i32 %149, 5
  br i1 %150, label %151, label %229

151:                                              ; preds = %148
  %152 = load i32, ptr %1, align 4
  %153 = icmp slt i32 %152, 101900
  br i1 %153, label %154, label %229

154:                                              ; preds = %151
  br label %155

155:                                              ; preds = %154
  %156 = load i32, ptr %1, align 4
  %157 = icmp slt i32 %156, 102400
  br i1 %157, label %158, label %163

158:                                              ; preds = %155
  %159 = load i32, ptr %1, align 4
  %160 = add nsw i32 %159, 1
  store i32 %160, ptr %1, align 4
  %161 = sext i32 %159 to i64
  %162 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %161
  store i8 91, ptr %162, align 1
  br label %163

163:                                              ; preds = %158, %155
  br label %164

164:                                              ; preds = %163
  %165 = call i32 @lcg_rand()
  %166 = urem i32 %165, 6
  %167 = add i32 2, %166
  store i32 %167, ptr %7, align 4
  store i32 0, ptr %8, align 4
  br label %168

168:                                              ; preds = %215, %164
  %169 = load i32, ptr %8, align 4
  %170 = load i32, ptr %7, align 4
  %171 = icmp slt i32 %169, %170
  br i1 %171, label %172, label %218

172:                                              ; preds = %168
  %173 = load i32, ptr %8, align 4
  %174 = icmp sgt i32 %173, 0
  br i1 %174, label %175, label %186

175:                                              ; preds = %172
  br label %176

176:                                              ; preds = %175
  %177 = load i32, ptr %1, align 4
  %178 = icmp slt i32 %177, 102400
  br i1 %178, label %179, label %184

179:                                              ; preds = %176
  %180 = load i32, ptr %1, align 4
  %181 = add nsw i32 %180, 1
  store i32 %181, ptr %1, align 4
  %182 = sext i32 %180 to i64
  %183 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %182
  store i8 44, ptr %183, align 1
  br label %184

184:                                              ; preds = %179, %176
  br label %185

185:                                              ; preds = %184
  br label %186

186:                                              ; preds = %185, %172
  %187 = call i32 @lcg_rand()
  %188 = urem i32 %187, 10000
  store i32 %188, ptr %10, align 4
  %189 = getelementptr inbounds [16 x i8], ptr %9, i64 0, i64 0
  %190 = load i32, ptr %10, align 4
  %191 = call i32 (ptr, ptr, ...) @sprintf(ptr noundef %189, ptr noundef @.str.1, i32 noundef %190) #3
  store i32 %191, ptr %11, align 4
  store i32 0, ptr %12, align 4
  br label %192

192:                                              ; preds = %211, %186
  %193 = load i32, ptr %12, align 4
  %194 = load i32, ptr %11, align 4
  %195 = icmp slt i32 %193, %194
  br i1 %195, label %196, label %214

196:                                              ; preds = %192
  br label %197

197:                                              ; preds = %196
  %198 = load i32, ptr %1, align 4
  %199 = icmp slt i32 %198, 102400
  br i1 %199, label %200, label %209

200:                                              ; preds = %197
  %201 = load i32, ptr %12, align 4
  %202 = sext i32 %201 to i64
  %203 = getelementptr inbounds [16 x i8], ptr %9, i64 0, i64 %202
  %204 = load i8, ptr %203, align 1
  %205 = load i32, ptr %1, align 4
  %206 = add nsw i32 %205, 1
  store i32 %206, ptr %1, align 4
  %207 = sext i32 %205 to i64
  %208 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %207
  store i8 %204, ptr %208, align 1
  br label %209

209:                                              ; preds = %200, %197
  br label %210

210:                                              ; preds = %209
  br label %211

211:                                              ; preds = %210
  %212 = load i32, ptr %12, align 4
  %213 = add nsw i32 %212, 1
  store i32 %213, ptr %12, align 4
  br label %192, !llvm.loop !10

214:                                              ; preds = %192
  br label %215

215:                                              ; preds = %214
  %216 = load i32, ptr %8, align 4
  %217 = add nsw i32 %216, 1
  store i32 %217, ptr %8, align 4
  br label %168, !llvm.loop !11

218:                                              ; preds = %168
  br label %219

219:                                              ; preds = %218
  %220 = load i32, ptr %1, align 4
  %221 = icmp slt i32 %220, 102400
  br i1 %221, label %222, label %227

222:                                              ; preds = %219
  %223 = load i32, ptr %1, align 4
  %224 = add nsw i32 %223, 1
  store i32 %224, ptr %1, align 4
  %225 = sext i32 %223 to i64
  %226 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %225
  store i8 93, ptr %226, align 1
  br label %227

227:                                              ; preds = %222, %219
  br label %228

228:                                              ; preds = %227
  br label %391

229:                                              ; preds = %151, %148, %145
  %230 = load i32, ptr %6, align 4
  %231 = icmp slt i32 %230, 7
  br i1 %231, label %232, label %281

232:                                              ; preds = %229
  br label %233

233:                                              ; preds = %232
  %234 = load i32, ptr %1, align 4
  %235 = icmp slt i32 %234, 102400
  br i1 %235, label %236, label %241

236:                                              ; preds = %233
  %237 = load i32, ptr %1, align 4
  %238 = add nsw i32 %237, 1
  store i32 %238, ptr %1, align 4
  %239 = sext i32 %237 to i64
  %240 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %239
  store i8 34, ptr %240, align 1
  br label %241

241:                                              ; preds = %236, %233
  br label %242

242:                                              ; preds = %241
  %243 = call i32 @lcg_rand()
  %244 = urem i32 %243, 12
  %245 = add i32 3, %244
  store i32 %245, ptr %13, align 4
  store i32 0, ptr %14, align 4
  br label %246

246:                                              ; preds = %267, %242
  %247 = load i32, ptr %14, align 4
  %248 = load i32, ptr %13, align 4
  %249 = icmp slt i32 %247, %248
  br i1 %249, label %250, label %270

250:                                              ; preds = %246
  br label %251

251:                                              ; preds = %250
  %252 = load i32, ptr %1, align 4
  %253 = icmp slt i32 %252, 102400
  br i1 %253, label %254, label %265

254:                                              ; preds = %251
  %255 = call i32 @lcg_rand()
  %256 = urem i32 %255, 26
  %257 = trunc i32 %256 to i8
  %258 = sext i8 %257 to i32
  %259 = add nsw i32 97, %258
  %260 = trunc i32 %259 to i8
  %261 = load i32, ptr %1, align 4
  %262 = add nsw i32 %261, 1
  store i32 %262, ptr %1, align 4
  %263 = sext i32 %261 to i64
  %264 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %263
  store i8 %260, ptr %264, align 1
  br label %265

265:                                              ; preds = %254, %251
  br label %266

266:                                              ; preds = %265
  br label %267

267:                                              ; preds = %266
  %268 = load i32, ptr %14, align 4
  %269 = add nsw i32 %268, 1
  store i32 %269, ptr %14, align 4
  br label %246, !llvm.loop !12

270:                                              ; preds = %246
  br label %271

271:                                              ; preds = %270
  %272 = load i32, ptr %1, align 4
  %273 = icmp slt i32 %272, 102400
  br i1 %273, label %274, label %279

274:                                              ; preds = %271
  %275 = load i32, ptr %1, align 4
  %276 = add nsw i32 %275, 1
  store i32 %276, ptr %1, align 4
  %277 = sext i32 %275 to i64
  %278 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %277
  store i8 34, ptr %278, align 1
  br label %279

279:                                              ; preds = %274, %271
  br label %280

280:                                              ; preds = %279
  br label %390

281:                                              ; preds = %229
  %282 = load i32, ptr %6, align 4
  %283 = icmp slt i32 %282, 9
  br i1 %283, label %284, label %313

284:                                              ; preds = %281
  %285 = call i32 @lcg_rand()
  %286 = urem i32 %285, 100000
  store i32 %286, ptr %16, align 4
  %287 = getelementptr inbounds [16 x i8], ptr %15, i64 0, i64 0
  %288 = load i32, ptr %16, align 4
  %289 = call i32 (ptr, ptr, ...) @sprintf(ptr noundef %287, ptr noundef @.str.1, i32 noundef %288) #3
  store i32 %289, ptr %17, align 4
  store i32 0, ptr %18, align 4
  br label %290

290:                                              ; preds = %309, %284
  %291 = load i32, ptr %18, align 4
  %292 = load i32, ptr %17, align 4
  %293 = icmp slt i32 %291, %292
  br i1 %293, label %294, label %312

294:                                              ; preds = %290
  br label %295

295:                                              ; preds = %294
  %296 = load i32, ptr %1, align 4
  %297 = icmp slt i32 %296, 102400
  br i1 %297, label %298, label %307

298:                                              ; preds = %295
  %299 = load i32, ptr %18, align 4
  %300 = sext i32 %299 to i64
  %301 = getelementptr inbounds [16 x i8], ptr %15, i64 0, i64 %300
  %302 = load i8, ptr %301, align 1
  %303 = load i32, ptr %1, align 4
  %304 = add nsw i32 %303, 1
  store i32 %304, ptr %1, align 4
  %305 = sext i32 %303 to i64
  %306 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %305
  store i8 %302, ptr %306, align 1
  br label %307

307:                                              ; preds = %298, %295
  br label %308

308:                                              ; preds = %307
  br label %309

309:                                              ; preds = %308
  %310 = load i32, ptr %18, align 4
  %311 = add nsw i32 %310, 1
  store i32 %311, ptr %18, align 4
  br label %290, !llvm.loop !13

312:                                              ; preds = %290
  br label %389

313:                                              ; preds = %281
  %314 = call i32 @lcg_rand()
  %315 = urem i32 %314, 3
  %316 = icmp eq i32 %315, 0
  br i1 %316, label %317, label %339

317:                                              ; preds = %313
  br label %318

318:                                              ; preds = %317
  store ptr @.str.2, ptr %19, align 8
  br label %319

319:                                              ; preds = %329, %318
  %320 = load ptr, ptr %19, align 8
  %321 = load i8, ptr %320, align 1
  %322 = sext i8 %321 to i32
  %323 = icmp ne i32 %322, 0
  br i1 %323, label %324, label %327

324:                                              ; preds = %319
  %325 = load i32, ptr %1, align 4
  %326 = icmp slt i32 %325, 102400
  br label %327

327:                                              ; preds = %324, %319
  %328 = phi i1 [ false, %319 ], [ %326, %324 ]
  br i1 %328, label %329, label %337

329:                                              ; preds = %327
  %330 = load ptr, ptr %19, align 8
  %331 = getelementptr inbounds nuw i8, ptr %330, i32 1
  store ptr %331, ptr %19, align 8
  %332 = load i8, ptr %330, align 1
  %333 = load i32, ptr %1, align 4
  %334 = add nsw i32 %333, 1
  store i32 %334, ptr %1, align 4
  %335 = sext i32 %333 to i64
  %336 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %335
  store i8 %332, ptr %336, align 1
  br label %319, !llvm.loop !14

337:                                              ; preds = %327
  br label %338

338:                                              ; preds = %337
  br label %388

339:                                              ; preds = %313
  %340 = call i32 @lcg_rand()
  %341 = urem i32 %340, 2
  %342 = icmp ne i32 %341, 0
  br i1 %342, label %343, label %365

343:                                              ; preds = %339
  br label %344

344:                                              ; preds = %343
  store ptr @.str.3, ptr %20, align 8
  br label %345

345:                                              ; preds = %355, %344
  %346 = load ptr, ptr %20, align 8
  %347 = load i8, ptr %346, align 1
  %348 = sext i8 %347 to i32
  %349 = icmp ne i32 %348, 0
  br i1 %349, label %350, label %353

350:                                              ; preds = %345
  %351 = load i32, ptr %1, align 4
  %352 = icmp slt i32 %351, 102400
  br label %353

353:                                              ; preds = %350, %345
  %354 = phi i1 [ false, %345 ], [ %352, %350 ]
  br i1 %354, label %355, label %363

355:                                              ; preds = %353
  %356 = load ptr, ptr %20, align 8
  %357 = getelementptr inbounds nuw i8, ptr %356, i32 1
  store ptr %357, ptr %20, align 8
  %358 = load i8, ptr %356, align 1
  %359 = load i32, ptr %1, align 4
  %360 = add nsw i32 %359, 1
  store i32 %360, ptr %1, align 4
  %361 = sext i32 %359 to i64
  %362 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %361
  store i8 %358, ptr %362, align 1
  br label %345, !llvm.loop !15

363:                                              ; preds = %353
  br label %364

364:                                              ; preds = %363
  br label %387

365:                                              ; preds = %339
  br label %366

366:                                              ; preds = %365
  store ptr @.str.4, ptr %21, align 8
  br label %367

367:                                              ; preds = %377, %366
  %368 = load ptr, ptr %21, align 8
  %369 = load i8, ptr %368, align 1
  %370 = sext i8 %369 to i32
  %371 = icmp ne i32 %370, 0
  br i1 %371, label %372, label %375

372:                                              ; preds = %367
  %373 = load i32, ptr %1, align 4
  %374 = icmp slt i32 %373, 102400
  br label %375

375:                                              ; preds = %372, %367
  %376 = phi i1 [ false, %367 ], [ %374, %372 ]
  br i1 %376, label %377, label %385

377:                                              ; preds = %375
  %378 = load ptr, ptr %21, align 8
  %379 = getelementptr inbounds nuw i8, ptr %378, i32 1
  store ptr %379, ptr %21, align 8
  %380 = load i8, ptr %378, align 1
  %381 = load i32, ptr %1, align 4
  %382 = add nsw i32 %381, 1
  store i32 %382, ptr %1, align 4
  %383 = sext i32 %381 to i64
  %384 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %383
  store i8 %380, ptr %384, align 1
  br label %367, !llvm.loop !16

385:                                              ; preds = %375
  br label %386

386:                                              ; preds = %385
  br label %387

387:                                              ; preds = %386, %364
  br label %388

388:                                              ; preds = %387, %338
  br label %389

389:                                              ; preds = %388, %312
  br label %390

390:                                              ; preds = %389, %280
  br label %391

391:                                              ; preds = %390, %228
  br label %392

392:                                              ; preds = %391, %142
  %393 = load i32, ptr %2, align 4
  %394 = icmp sgt i32 %393, 1
  br i1 %394, label %395, label %412

395:                                              ; preds = %392
  %396 = call i32 @lcg_rand()
  %397 = urem i32 %396, 4
  %398 = icmp eq i32 %397, 0
  br i1 %398, label %399, label %412

399:                                              ; preds = %395
  br label %400

400:                                              ; preds = %399
  %401 = load i32, ptr %1, align 4
  %402 = icmp slt i32 %401, 102400
  br i1 %402, label %403, label %408

403:                                              ; preds = %400
  %404 = load i32, ptr %1, align 4
  %405 = add nsw i32 %404, 1
  store i32 %405, ptr %1, align 4
  %406 = sext i32 %404 to i64
  %407 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %406
  store i8 125, ptr %407, align 1
  br label %408

408:                                              ; preds = %403, %400
  br label %409

409:                                              ; preds = %408
  %410 = load i32, ptr %2, align 4
  %411 = add nsw i32 %410, -1
  store i32 %411, ptr %2, align 4
  br label %412

412:                                              ; preds = %409, %395, %392
  br label %34, !llvm.loop !17

413:                                              ; preds = %40
  br label %414

414:                                              ; preds = %427, %413
  %415 = load i32, ptr %2, align 4
  %416 = icmp sgt i32 %415, 0
  br i1 %416, label %417, label %430

417:                                              ; preds = %414
  br label %418

418:                                              ; preds = %417
  %419 = load i32, ptr %1, align 4
  %420 = icmp slt i32 %419, 102400
  br i1 %420, label %421, label %426

421:                                              ; preds = %418
  %422 = load i32, ptr %1, align 4
  %423 = add nsw i32 %422, 1
  store i32 %423, ptr %1, align 4
  %424 = sext i32 %422 to i64
  %425 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %424
  store i8 125, ptr %425, align 1
  br label %426

426:                                              ; preds = %421, %418
  br label %427

427:                                              ; preds = %426
  %428 = load i32, ptr %2, align 4
  %429 = add nsw i32 %428, -1
  store i32 %429, ptr %2, align 4
  br label %414, !llvm.loop !18

430:                                              ; preds = %414
  %431 = load i32, ptr %1, align 4
  %432 = sext i32 %431 to i64
  %433 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %432
  store i8 0, ptr %433, align 1
  %434 = load i32, ptr %1, align 4
  store i32 %434, ptr @json_len, align 4
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

4:                                                ; preds = %164, %0
  %5 = load i32, ptr %2, align 4
  %6 = load i32, ptr @json_len, align 4
  %7 = icmp slt i32 %5, %6
  br i1 %7, label %8, label %165

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
  br label %164

41:                                               ; preds = %32
  %42 = load i8, ptr %3, align 1
  %43 = sext i8 %42 to i32
  %44 = icmp eq i32 %43, 34
  br i1 %44, label %45, label %79

45:                                               ; preds = %41
  %46 = load i32, ptr %1, align 4
  %47 = add nsw i32 %46, 1
  store i32 %47, ptr %1, align 4
  %48 = load i32, ptr %2, align 4
  %49 = add nsw i32 %48, 1
  store i32 %49, ptr %2, align 4
  br label %50

50:                                               ; preds = %73, %45
  %51 = load i32, ptr %2, align 4
  %52 = load i32, ptr @json_len, align 4
  %53 = icmp slt i32 %51, %52
  br i1 %53, label %54, label %61

54:                                               ; preds = %50
  %55 = load i32, ptr %2, align 4
  %56 = sext i32 %55 to i64
  %57 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %56
  %58 = load i8, ptr %57, align 1
  %59 = sext i8 %58 to i32
  %60 = icmp ne i32 %59, 34
  br label %61

61:                                               ; preds = %54, %50
  %62 = phi i1 [ false, %50 ], [ %60, %54 ]
  br i1 %62, label %63, label %76

63:                                               ; preds = %61
  %64 = load i32, ptr %2, align 4
  %65 = sext i32 %64 to i64
  %66 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %65
  %67 = load i8, ptr %66, align 1
  %68 = sext i8 %67 to i32
  %69 = icmp eq i32 %68, 92
  br i1 %69, label %70, label %73

70:                                               ; preds = %63
  %71 = load i32, ptr %2, align 4
  %72 = add nsw i32 %71, 1
  store i32 %72, ptr %2, align 4
  br label %73

73:                                               ; preds = %70, %63
  %74 = load i32, ptr %2, align 4
  %75 = add nsw i32 %74, 1
  store i32 %75, ptr %2, align 4
  br label %50, !llvm.loop !19

76:                                               ; preds = %61
  %77 = load i32, ptr %2, align 4
  %78 = add nsw i32 %77, 1
  store i32 %78, ptr %2, align 4
  br label %163

79:                                               ; preds = %41
  %80 = load i8, ptr %3, align 1
  %81 = sext i8 %80 to i32
  %82 = icmp sge i32 %81, 48
  br i1 %82, label %83, label %87

83:                                               ; preds = %79
  %84 = load i8, ptr %3, align 1
  %85 = sext i8 %84 to i32
  %86 = icmp sle i32 %85, 57
  br i1 %86, label %91, label %87

87:                                               ; preds = %83, %79
  %88 = load i8, ptr %3, align 1
  %89 = sext i8 %88 to i32
  %90 = icmp eq i32 %89, 45
  br i1 %90, label %91, label %129

91:                                               ; preds = %87, %83
  %92 = load i32, ptr %1, align 4
  %93 = add nsw i32 %92, 1
  store i32 %93, ptr %1, align 4
  %94 = load i32, ptr %2, align 4
  %95 = add nsw i32 %94, 1
  store i32 %95, ptr %2, align 4
  br label %96

96:                                               ; preds = %125, %91
  %97 = load i32, ptr %2, align 4
  %98 = load i32, ptr @json_len, align 4
  %99 = icmp slt i32 %97, %98
  br i1 %99, label %100, label %123

100:                                              ; preds = %96
  %101 = load i32, ptr %2, align 4
  %102 = sext i32 %101 to i64
  %103 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %102
  %104 = load i8, ptr %103, align 1
  %105 = sext i8 %104 to i32
  %106 = icmp sge i32 %105, 48
  br i1 %106, label %107, label %114

107:                                              ; preds = %100
  %108 = load i32, ptr %2, align 4
  %109 = sext i32 %108 to i64
  %110 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %109
  %111 = load i8, ptr %110, align 1
  %112 = sext i8 %111 to i32
  %113 = icmp sle i32 %112, 57
  br i1 %113, label %121, label %114

114:                                              ; preds = %107, %100
  %115 = load i32, ptr %2, align 4
  %116 = sext i32 %115 to i64
  %117 = getelementptr inbounds [102401 x i8], ptr @json_buf, i64 0, i64 %116
  %118 = load i8, ptr %117, align 1
  %119 = sext i8 %118 to i32
  %120 = icmp eq i32 %119, 46
  br label %121

121:                                              ; preds = %114, %107
  %122 = phi i1 [ true, %107 ], [ %120, %114 ]
  br label %123

123:                                              ; preds = %121, %96
  %124 = phi i1 [ false, %96 ], [ %122, %121 ]
  br i1 %124, label %125, label %128

125:                                              ; preds = %123
  %126 = load i32, ptr %2, align 4
  %127 = add nsw i32 %126, 1
  store i32 %127, ptr %2, align 4
  br label %96, !llvm.loop !20

128:                                              ; preds = %123
  br label %162

129:                                              ; preds = %87
  %130 = load i8, ptr %3, align 1
  %131 = sext i8 %130 to i32
  %132 = icmp eq i32 %131, 116
  br i1 %132, label %133, label %138

133:                                              ; preds = %129
  %134 = load i32, ptr %1, align 4
  %135 = add nsw i32 %134, 1
  store i32 %135, ptr %1, align 4
  %136 = load i32, ptr %2, align 4
  %137 = add nsw i32 %136, 4
  store i32 %137, ptr %2, align 4
  br label %161

138:                                              ; preds = %129
  %139 = load i8, ptr %3, align 1
  %140 = sext i8 %139 to i32
  %141 = icmp eq i32 %140, 102
  br i1 %141, label %142, label %147

142:                                              ; preds = %138
  %143 = load i32, ptr %1, align 4
  %144 = add nsw i32 %143, 1
  store i32 %144, ptr %1, align 4
  %145 = load i32, ptr %2, align 4
  %146 = add nsw i32 %145, 5
  store i32 %146, ptr %2, align 4
  br label %160

147:                                              ; preds = %138
  %148 = load i8, ptr %3, align 1
  %149 = sext i8 %148 to i32
  %150 = icmp eq i32 %149, 110
  br i1 %150, label %151, label %156

151:                                              ; preds = %147
  %152 = load i32, ptr %1, align 4
  %153 = add nsw i32 %152, 1
  store i32 %153, ptr %1, align 4
  %154 = load i32, ptr %2, align 4
  %155 = add nsw i32 %154, 4
  store i32 %155, ptr %2, align 4
  br label %159

156:                                              ; preds = %147
  %157 = load i32, ptr %2, align 4
  %158 = add nsw i32 %157, 1
  store i32 %158, ptr %2, align 4
  br label %159

159:                                              ; preds = %156, %151
  br label %160

160:                                              ; preds = %159, %142
  br label %161

161:                                              ; preds = %160, %133
  br label %162

162:                                              ; preds = %161, %128
  br label %163

163:                                              ; preds = %162, %76
  br label %164

164:                                              ; preds = %163, %36
  br label %4, !llvm.loop !21

165:                                              ; preds = %4
  %166 = load i32, ptr %1, align 4
  store volatile i32 %166, ptr @token_count, align 4
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
