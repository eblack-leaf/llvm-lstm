#include <stdio.h>
#include <stdlib.h>
#include <time.h>

static long long timespec_diff_ns(struct timespec *a, struct timespec *b) {
    return (long long)(b->tv_sec - a->tv_sec) * 1000000000LL + (b->tv_nsec - a->tv_nsec);
}

static int cmp_ll(const void *a, const void *b) {
    long long x = *(const long long *)a, y = *(const long long *)b;
    return (x > y) - (x < y);
}

static unsigned int lcg_state = 12345;
static unsigned int lcg_rand(void) {
    lcg_state = lcg_state * 1103515245 + 12345;
    return (lcg_state >> 16) & 0x7fff;
}

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

    return total;
}

int main(void) {
    int pixels = IMG_W * IMG_H;
    float *img = (float *)malloc(pixels * sizeof(float));
    float *tmp = (float *)malloc(pixels * sizeof(float));
    float *out = (float *)malloc(pixels * sizeof(float));
    float kern5[25], kern3[9], kern1d[5];
    int i;

    lcg_state = 12345;
    for (i = 0; i < pixels; i++) {
        img[i] = (float)lcg_rand() / 32768.0f;
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

    /* Warmup */
    volatile float sink;
    for (i = 0; i < 3; i++) {
        sink = workload(img, tmp, out, kern5, kern3, kern1d);
    }

    /* Timing: 201 runs, 10% trimmed mean */
    long long times[201];
    struct timespec t0, t1;
    for (i = 0; i < 201; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(img, tmp, out, kern5, kern3, kern1d);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);

    free(img); free(tmp); free(out);
    return 0;
}
