#include "bench_timing.h"

/*
 * Float-to-int optimization target: computations that use floating point
 * but only ever produce integer results. float2int converts sitofp→math→fptosi
 * chains back to integer ops. Also includes vectorizable float array work
 * for vector-combine opportunities.
 */

#define N 2000

/* Simple: int→float→add→int — textbook float2int pattern */
static int add_via_float(int a, int b) {
    double fa = (double)a;
    double fb = (double)b;
    double sum = fa + fb;
    return (int)sum;
}

/* int→float→multiply→int */
static int mul_via_float(int a, int b) {
    double fa = (double)a;
    double fb = (double)b;
    return (int)(fa * fb);
}

/* Squared distance via float — pure int result */
static int dist_sq_float(int x1, int y1, int x2, int y2) {
    double dx = (double)(x2 - x1);
    double dy = (double)(y2 - y1);
    return (int)(dx * dx + dy * dy);
}

/* Weighted average of two ints via float */
static int weighted_avg(int a, int b, int w_num, int w_den) {
    double wa = (double)a * (double)w_num / (double)w_den;
    double wb = (double)b * (double)(w_den - w_num) / (double)w_den;
    return (int)(wa + wb);
}

/* Scale array: each element multiplied by int ratio via float */
static void scale_array(const int *in, int *out, int n, int num, int den) {
    int i;
    for (i = 0; i < n; i++) {
        double val = (double)in[i] * (double)num / (double)den;
        out[i] = (int)val;
    }
}

/* Blend two arrays with float weights (int results) */
static void blend_arrays(const int *a, const int *b, int *out, int n, int alpha_pct) {
    int i;
    double alpha = (double)alpha_pct / 100.0;
    double beta = 1.0 - alpha;
    for (i = 0; i < n; i++) {
        out[i] = (int)((double)a[i] * alpha + (double)b[i] * beta);
    }
}

/* Matrix-vector multiply via float (small, unrolled) */
static void matvec_float(const int mat[4][4], const int vec[4], int out[4]) {
    int i, j;
    for (i = 0; i < 4; i++) {
        double sum = 0.0;
        for (j = 0; j < 4; j++) {
            sum += (double)mat[i][j] * (double)vec[j];
        }
        out[i] = (int)sum;
    }
}

/* Normalize array to [0, 1000] range via float */
static void normalize_range(const int *in, int *out, int n, int lo, int hi) {
    int i;
    double range = (double)(hi - lo);
    if (range < 1.0) range = 1.0;
    for (i = 0; i < n; i++) {
        double scaled = (double)(in[i] - lo) / range * 1000.0;
        out[i] = (int)scaled;
    }
}

/* Vectorizable float reductions */
static double dot_product(const int *a, const int *b, int n) {
    double sum = 0.0;
    int i;
    for (i = 0; i < n; i++) {
        sum += (double)a[i] * (double)b[i];
    }
    return sum;
}

static long long workload(int *vals, int *buf, int *buf2) {
    long long sum = 0;
    int i;

    /* Element-wise float2int patterns */
    for (i = 0; i < N - 1; i++) {
        sum += add_via_float(vals[i], vals[i + 1]);
        sum += mul_via_float(vals[i] % 100, vals[i + 1] % 100);
    }

    /* Squared distances */
    for (i = 0; i < N - 3; i += 2) {
        sum += dist_sq_float(vals[i] % 100, vals[i + 1] % 100,
                             vals[i + 2] % 100, vals[i + 3] % 100);
    }

    /* Weighted averages */
    for (i = 0; i < N - 1; i++) {
        sum += weighted_avg(vals[i], vals[i + 1], (i % 7) + 1, 8);
    }

    /* Array scaling via float */
    scale_array(vals, buf, N, 3, 4);
    for (i = 0; i < N; i++) sum += buf[i];

    /* Array blending */
    blend_arrays(vals, buf, buf2, N, 70);
    for (i = 0; i < N; i++) sum += buf2[i];

    /* Small matvec via float */
    int mat[4][4], vec[4], mvout[4];
    for (i = 0; i < 4; i++) {
        vec[i] = vals[i];
        int j;
        for (j = 0; j < 4; j++) mat[i][j] = vals[i * 4 + j] % 50;
    }
    for (i = 0; i < 200; i++) {
        matvec_float(mat, vec, mvout);
        vec[0] = mvout[0] % 1000;
        vec[1] = mvout[1] % 1000;
        vec[2] = mvout[2] % 1000;
        vec[3] = mvout[3] % 1000;
    }
    for (i = 0; i < 4; i++) sum += mvout[i];

    /* Normalize */
    int lo = vals[0], hi = vals[0];
    for (i = 1; i < N; i++) {
        if (vals[i] < lo) lo = vals[i];
        if (vals[i] > hi) hi = vals[i];
    }
    normalize_range(vals, buf, N, lo, hi);
    for (i = 0; i < N; i++) sum += buf[i];

    /* Dot product (vectorizable) */
    sum += (long long)dot_product(vals, buf, N);

    return sum;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    int vals[N], buf[N], buf2[N];
    int i;

    bench_lcg_seed(42);
    for (i = 0; i < N; i++) {
        vals[i] = (int)(bench_lcg_rand() % 10000) + 1;
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(vals, buf, buf2); });

    return 0;
}
