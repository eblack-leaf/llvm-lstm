#include "bench_timing.h"

/*
 * Select/ternary-heavy code: phi nodes from conditional moves,
 * minimal branching. Targets branchless optimization patterns.
 */

#define N 4000

static int min3(int a, int b, int c) {
    int m = a < b ? a : b;
    return m < c ? m : c;
}

static int max3(int a, int b, int c) {
    int m = a > b ? a : b;
    return m > c ? m : c;
}

static int clamp(int x, int lo, int hi) {
    return x < lo ? lo : (x > hi ? hi : x);
}

static int median3(int a, int b, int c) {
    int ab = a < b ? a : b;
    int bc = b < c ? b : c;
    int ac = a < c ? a : c;
    return max3(ab, bc, ac);
}

/* Branchless abs */
static int iabs(int x) {
    return x < 0 ? -x : x;
}

/* Smooth step approximation via selects */
static int smoothstep(int x, int edge0, int edge1) {
    int range = edge1 - edge0;
    if (range == 0) return 0;
    int t = clamp((x - edge0) * 1000 / range, 0, 1000);
    return t * t * (3000 - 2 * t) / 1000000;
}

/* Median filter on 1D array (window=3) */
static void median_filter(const int *in, int *out, int n) {
    int i;
    out[0] = in[0];
    out[n - 1] = in[n - 1];
    for (i = 1; i < n - 1; i++) {
        out[i] = median3(in[i - 1], in[i], in[i + 1]);
    }
}

/* Branchless merge step: merge two sorted halves using min/max */
static void bitonic_merge(int *arr, int n) {
    int half = n / 2;
    int i;
    for (i = 0; i < half; i++) {
        int a = arr[i];
        int b = arr[i + half];
        arr[i] = a < b ? a : b;
        arr[i + half] = a > b ? a : b;
    }
}

/* Element-wise conditional accumulation */
static long long conditional_accum(const int *a, const int *b, const int *c, int n) {
    long long sum = 0;
    int i;
    for (i = 0; i < n; i++) {
        int val = a[i] > b[i] ? a[i] - b[i] : b[i] - a[i];
        int weight = c[i] > 500 ? 3 : 1;
        sum += (long long)val * weight;
    }
    return sum;
}

/* Pixel blending with alpha threshold */
static void alpha_blend(const int *src, const int *dst, const int *alpha,
                        int *out, int n) {
    int i;
    for (i = 0; i < n; i++) {
        int a = alpha[i] > 128 ? alpha[i] : 0;
        int inv_a = 256 - a;
        out[i] = (src[i] * a + dst[i] * inv_a) / 256;
    }
}

/* Multi-way clamp and remap */
static void remap(const int *in, int *out, int n) {
    int i;
    for (i = 0; i < n; i++) {
        int x = in[i];
        int zone = x < 250 ? 0 : (x < 500 ? 1 : (x < 750 ? 2 : 3));
        int base = zone * 250;
        int scaled = (x - base) * (zone + 1);
        out[i] = clamp(scaled, 0, 1000);
    }
}

static long long workload(int *a, int *b, int *c) {
    long long sum = 0;
    int i;

    /* Median filter passes */
    for (i = 0; i < 5; i++) {
        median_filter(a, b, N);
        median_filter(b, a, N);
    }

    /* Bitonic merge steps */
    int step;
    for (step = 2; step <= 64; step *= 2) {
        for (i = 0; i + step <= N; i += step) {
            bitonic_merge(a + i, step);
        }
    }

    /* Conditional accumulation */
    sum += conditional_accum(a, b, c, N);

    /* Alpha blending */
    alpha_blend(a, b, c, a, N);

    /* Smoothstep remap */
    for (i = 0; i < N; i++) {
        sum += smoothstep(a[i], 100, 900);
    }

    /* Multi-way remap */
    remap(a, b, N);
    for (i = 0; i < N; i++) sum += b[i];

    /* Min/max reductions */
    for (i = 0; i < N - 2; i++) {
        sum += min3(a[i], a[i + 1], a[i + 2]);
        sum += max3(a[i], a[i + 1], a[i + 2]);
    }

    return sum;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    int a[N], b[N], c[N];
    int i;

    bench_lcg_seed(55);
    for (i = 0; i < N; i++) {
        a[i] = (int)(bench_lcg_rand() % 1000);
        b[i] = (int)(bench_lcg_rand() % 1000);
        c[i] = (int)(bench_lcg_rand() % 256);
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(a, b, c); });

    return 0;
}
