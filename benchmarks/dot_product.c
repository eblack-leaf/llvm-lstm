#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <string.h>

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

#define VEC_N 2000

/* Standard dot product */
static double dot_basic(const double *a, const double *b, int n) {
    double sum = 0.0;
    int i;
    for (i = 0; i < n; i++) {
        sum += a[i] * b[i];
    }
    return sum;
}

/* Strided dot product — different memory access pattern */
static double dot_strided(const double *a, const double *b, int n, int stride) {
    double sum = 0.0;
    int i;
    for (i = 0; i + stride < n; i += stride) {
        sum += a[i] * b[i];
    }
    return sum;
}

/* Weighted dot product with per-element branch */
static double dot_weighted(const double *a, const double *b, const double *w, int n) {
    double sum = 0.0;
    int i;
    for (i = 0; i < n; i++) {
        double prod = a[i] * b[i];
        if (w[i] > 0.5)
            sum += prod * w[i];
        else
            sum += prod * (1.0 - w[i]);
    }
    return sum;
}

/* Cosine similarity: dot(a,b) / (norm(a) * norm(b)) — multiple reductions */
static double cosine_sim(const double *a, const double *b, int n) {
    double dot = 0.0, norm_a = 0.0, norm_b = 0.0;
    int i;
    for (i = 0; i < n; i++) {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    /* Avoid sqrt to not need -lm at runtime */
    return dot * dot / (norm_a * norm_b + 1e-12);
}

/* Multi-vector dot products in a small matrix-vector multiply */
static void matvec(const double *mat, const double *vec, double *out, int rows, int cols) {
    int r, c;
    for (r = 0; r < rows; r++) {
        double sum = 0.0;
        for (c = 0; c < cols; c++) {
            sum += mat[r * cols + c] * vec[c];
        }
        out[r] = sum;
    }
}

#define MAT_ROWS 50
#define MAT_COLS VEC_N

static double workload(double *a, double *b, double *w, double *mat, double *mvout) {
    double total = 0.0;

    total += dot_basic(a, b, VEC_N);
    total += dot_strided(a, b, VEC_N, 3);
    total += dot_weighted(a, b, w, VEC_N);
    total += cosine_sim(a, b, VEC_N);

    matvec(mat, b, mvout, MAT_ROWS, MAT_COLS);
    int i;
    for (i = 0; i < MAT_ROWS; i++) total += mvout[i];

    return total;
}

int main(void) {
    double *a = (double *)malloc(VEC_N * sizeof(double));
    double *b = (double *)malloc(VEC_N * sizeof(double));
    double *w = (double *)malloc(VEC_N * sizeof(double));
    double *mat = (double *)malloc(MAT_ROWS * MAT_COLS * sizeof(double));
    double *mvout = (double *)malloc(MAT_ROWS * sizeof(double));
    int i;

    lcg_state = 12345;
    for (i = 0; i < VEC_N; i++) {
        a[i] = (double)lcg_rand() / 32768.0;
        b[i] = (double)lcg_rand() / 32768.0;
        w[i] = (double)lcg_rand() / 32768.0;
    }
    for (i = 0; i < MAT_ROWS * MAT_COLS; i++) {
        mat[i] = (double)lcg_rand() / 32768.0;
    }

    /* Warmup */
    volatile double sink;
    for (i = 0; i < 5; i++) {
        sink = workload(a, b, w, mat, mvout);
    }

    /* Timing: 201 runs, 10% trimmed mean */
    long long times[201];
    struct timespec t0, t1;
    for (i = 0; i < 201; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(a, b, w, mat, mvout);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    /* Drop bottom/top 10% (20 each), average middle 161 */
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);

    free(a); free(b); free(w); free(mat); free(mvout);
    return 0;
}
