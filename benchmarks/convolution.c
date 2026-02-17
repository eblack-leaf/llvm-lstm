#include "bench_timing.h"
#include <string.h>

#define IMG_W 64
#define IMG_H 64

/* Generic 2D convolution — tests loop-unroll with variable kernel size */
static void conv2d(const float *img, const float *kern, float *out,
                   int w, int h, int ks) {
    int half = ks / 2;
    int y, x, ky, kx;
    for (y = half; y < h - half; y++) {
        for (x = half; x < w - half; x++) {
            float sum = 0.0f;
            for (ky = 0; ky < ks; ky++) {
                for (kx = 0; kx < ks; kx++) {
                    sum += img[(y - half + ky) * w + (x - half + kx)]
                         * kern[ky * ks + kx];
                }
            }
            out[y * w + x] = sum;
        }
    }
}

/* Separable convolution: horizontal pass — tests LICM on kernel values */
static void conv_horizontal(const float *img, const float *kern1d, float *out,
                            int w, int h, int ks) {
    int half = ks / 2;
    int y, x, k;
    for (y = 0; y < h; y++) {
        for (x = half; x < w - half; x++) {
            float sum = 0.0f;
            for (k = 0; k < ks; k++) {
                sum += img[y * w + (x - half + k)] * kern1d[k];
            }
            out[y * w + x] = sum;
        }
    }
}

/* Separable convolution: vertical pass */
static void conv_vertical(const float *img, const float *kern1d, float *out,
                          int w, int h, int ks) {
    int half = ks / 2;
    int y, x, k;
    for (y = half; y < h - half; y++) {
        for (x = 0; x < w; x++) {
            float sum = 0.0f;
            for (k = 0; k < ks; k++) {
                sum += img[(y - half + k) * w + x] * kern1d[k];
            }
            out[y * w + x] = sum;
        }
    }
}

/* Pointwise operations between images — tests loop opts, inline */
static void pointwise_relu(float *img, int n) {
    int i;
    for (i = 0; i < n; i++) {
        if (img[i] < 0.0f) img[i] = 0.0f;
    }
}

static void pointwise_add(const float *a, const float *b, float *out, int n) {
    int i;
    for (i = 0; i < n; i++) {
        out[i] = a[i] + b[i];
    }
}

static float sum_image(const float *img, int n) {
    float s = 0.0f;
    int i;
    for (i = 0; i < n; i++) s += img[i];
    return s;
}

/* Dilated 2D convolution — larger receptive field, bounds-check branches */
static void conv2d_dilated(const float *img, const float *kern, float *out,
                           int w, int h, int ks, int dilation) {
    int half = (ks / 2) * dilation;
    int y, x, ky, kx;
    for (y = half; y < h - half; y++) {
        for (x = half; x < w - half; x++) {
            float sum = 0.0f;
            for (ky = 0; ky < ks; ky++) {
                for (kx = 0; kx < ks; kx++) {
                    int iy = y - half + ky * dilation;
                    int ix = x - half + kx * dilation;
                    if (iy >= 0 && iy < h && ix >= 0 && ix < w)
                        sum += img[iy * w + ix] * kern[ky * ks + kx];
                }
            }
            out[y * w + x] = sum;
        }
    }
}

/* Approximate tanh: x/(1+|x|) — sign branch per element */
static void pointwise_tanh_approx(float *img, int n) {
    for (int i = 0; i < n; i++) {
        float x = img[i];
        float ax = (x >= 0.0f) ? x : -x;
        img[i] = x / (1.0f + ax);
    }
}

/* Clamp to [lo, hi] — two branches per element */
static void pointwise_clamp(float *img, int n, float lo, float hi) {
    for (int i = 0; i < n; i++) {
        if (img[i] < lo) img[i] = lo;
        if (img[i] > hi) img[i] = hi;
    }
}

/* Max pooling — 4-level nested loop with comparison branch */
static void max_pool(const float *img, float *out, int w, int h, int pool) {
    int oy = 0;
    for (int y = 0; y + pool <= h; y += pool) {
        int ox = 0;
        for (int x = 0; x + pool <= w; x += pool) {
            float mx = img[y * w + x];
            for (int py = 0; py < pool; py++) {
                for (int px = 0; px < pool; px++) {
                    float v = img[(y + py) * w + (x + px)];
                    if (v > mx) mx = v;
                }
            }
            out[oy * (w / pool) + ox] = mx;
            ox++;
        }
        oy++;
    }
}

/* Average pooling — same structure as max_pool, no comparison branch */
static void average_pool(const float *img, float *out, int w, int h, int pool) {
    int oy = 0;
    float inv = 1.0f / (float)(pool * pool);
    for (int y = 0; y + pool <= h; y += pool) {
        int ox = 0;
        for (int x = 0; x + pool <= w; x += pool) {
            float sum = 0.0f;
            for (int py = 0; py < pool; py++) {
                for (int px = 0; px < pool; px++) {
                    sum += img[(y + py) * w + (x + px)];
                }
            }
            out[oy * (w / pool) + ox] = sum * inv;
            ox++;
        }
        oy++;
    }
}

static float workload(float *img, float *tmp, float *out,
                      float *kern5, float *kern3, float *kern1d) {
    int pixels = IMG_W * IMG_H;
    float total = 0.0f;

    /* 5x5 convolution */
    conv2d(img, kern5, out, IMG_W, IMG_H, 5);
    total += sum_image(out, pixels);

    /* 3x3 convolution on the result — different unroll factor */
    conv2d(out, kern3, tmp, IMG_W, IMG_H, 3);
    total += sum_image(tmp, pixels);

    /* Separable 5-tap: horizontal then vertical */
    conv_horizontal(img, kern1d, tmp, IMG_W, IMG_H, 5);
    conv_vertical(tmp, kern1d, out, IMG_W, IMG_H, 5);
    pointwise_relu(out, pixels);
    total += sum_image(out, pixels);

    /* Residual connection: add original image to filtered */
    pointwise_add(img, out, tmp, pixels);
    total += sum_image(tmp, pixels);

    /* Dilated 3x3 convolution with dilation=2 */
    conv2d_dilated(img, kern3, tmp, IMG_W, IMG_H, 3, 2);
    total += sum_image(tmp, pixels);

    /* Approximate tanh on a copy of out */
    memcpy(tmp, out, pixels * sizeof(float));
    pointwise_tanh_approx(tmp, pixels);
    total += sum_image(tmp, pixels);

    /* Clamp image copy to [0.1, 0.9] */
    memcpy(out, img, pixels * sizeof(float));
    pointwise_clamp(out, pixels, 0.1f, 0.9f);
    total += sum_image(out, pixels);

    /* Max pooling 4x4 */
    int pooled = (IMG_W / 4) * (IMG_H / 4);
    max_pool(img, tmp, IMG_W, IMG_H, 4);
    total += sum_image(tmp, pooled);

    /* Average pooling 4x4 */
    average_pool(img, out, IMG_W, IMG_H, 4);
    total += sum_image(out, pooled);

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    int pixels = IMG_W * IMG_H;
    float *img = (float *)malloc(pixels * sizeof(float));
    float *tmp = (float *)malloc(pixels * sizeof(float));
    float *out = (float *)malloc(pixels * sizeof(float));
    float kern5[25], kern3[9], kern1d[5];
    int i;

    bench_lcg_seed(12345);
    for (i = 0; i < pixels; i++) {
        img[i] = (float)bench_lcg_rand() / 32768.0f;
    }

    /* Build kernels */
    float ksum = 0.0f;
    for (i = 0; i < 25; i++) {
        int ky = i / 5 - 2, kx = i % 5 - 2;
        kern5[i] = 1.0f / (1.0f + (float)(ky * ky + kx * kx));
        ksum += kern5[i];
    }
    for (i = 0; i < 25; i++) kern5[i] /= ksum;

    ksum = 0.0f;
    for (i = 0; i < 9; i++) {
        int ky = i / 3 - 1, kx = i % 3 - 1;
        kern3[i] = 1.0f / (1.0f + (float)(ky * ky + kx * kx));
        ksum += kern3[i];
    }
    for (i = 0; i < 9; i++) kern3[i] /= ksum;

    /* 1D kernel for separable convolution */
    float weights[] = {0.1f, 0.2f, 0.4f, 0.2f, 0.1f};
    for (i = 0; i < 5; i++) kern1d[i] = weights[i];

    volatile float sink;
    BENCH_TIME(niters, { sink = workload(img, tmp, out, kern5, kern3, kern1d); });

    free(img); free(tmp); free(out);
    return 0;
}
