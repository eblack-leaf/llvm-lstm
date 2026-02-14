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

#define N 48
#define BLOCK 8

/* Tiled (blocked) matrix multiply — 6 nested loops, tests loop-unroll + licm */
static void matmul_tiled(const double *A, const double *B, double *C) {
    int i, j, k, ii, jj, kk;
    memset(C, 0, N * N * sizeof(double));
    for (ii = 0; ii < N; ii += BLOCK) {
        for (jj = 0; jj < N; jj += BLOCK) {
            for (kk = 0; kk < N; kk += BLOCK) {
                for (i = ii; i < ii + BLOCK; i++) {
                    for (j = jj; j < jj + BLOCK; j++) {
                        double sum = C[i * N + j];
                        for (k = kk; k < kk + BLOCK; k++) {
                            sum += A[i * N + k] * B[k * N + j];
                        }
                        C[i * N + j] = sum;
                    }
                }
            }
        }
    }
}

/* Naive (ijk) multiply — different loop order, different opts apply */
static void matmul_naive(const double *A, const double *B, double *C) {
    int i, j, k;
    memset(C, 0, N * N * sizeof(double));
    for (i = 0; i < N; i++) {
        for (j = 0; j < N; j++) {
            double sum = 0.0;
            for (k = 0; k < N; k++) {
                sum += A[i * N + k] * B[k * N + j];
            }
            C[i * N + j] = sum;
        }
    }
}

/* Transpose B then multiply — tests if optimizer can see the equivalence */
static void transpose(const double *src, double *dst) {
    int i, j;
    for (i = 0; i < N; i++) {
        for (j = 0; j < N; j++) {
            dst[j * N + i] = src[i * N + j];
        }
    }
}

static void matmul_transposed(const double *A, const double *B, double *C, double *Bt) {
    transpose(B, Bt);
    int i, j, k;
    memset(C, 0, N * N * sizeof(double));
    for (i = 0; i < N; i++) {
        for (j = 0; j < N; j++) {
            double sum = 0.0;
            for (k = 0; k < N; k++) {
                sum += A[i * N + k] * Bt[j * N + k];
            }
            C[i * N + j] = sum;
        }
    }
}

/* Matrix-vector multiply — smaller inner dimension, different unroll behavior */
static void matvec(const double *A, const double *x, double *y) {
    int i, j;
    for (i = 0; i < N; i++) {
        double sum = 0.0;
        for (j = 0; j < N; j++) {
            sum += A[i * N + j] * x[j];
        }
        y[i] = sum;
    }
}

/* Frobenius norm — reduction over matrix */
static double frobenius_norm_sq(const double *M) {
    double sum = 0.0;
    int i;
    for (i = 0; i < N * N; i++) {
        sum += M[i] * M[i];
    }
    return sum;
}

/* Matrix subtraction — element-wise */
static void mat_sub(const double *A, const double *B, double *C) {
    int i;
    for (i = 0; i < N * N; i++) {
        C[i] = A[i] - B[i];
    }
}

static double workload(double *A, double *B, double *C, double *D, double *Bt, double *vec, double *vout) {
    double total = 0.0;

    /* Tiled multiply */
    matmul_tiled(A, B, C);
    total += frobenius_norm_sq(C);

    /* Naive multiply into D */
    matmul_naive(A, B, D);

    /* Compute difference C - D (should be near zero, tests DSE on D) */
    mat_sub(C, D, Bt);  /* reuse Bt as temp */
    total += frobenius_norm_sq(Bt);

    /* Transposed multiply */
    matmul_transposed(A, B, D, Bt);
    total += frobenius_norm_sq(D);

    /* Matrix-vector multiply */
    matvec(A, vec, vout);
    int i;
    for (i = 0; i < N; i++) total += vout[i];

    /* Chained matvec: y = A * (A * x) */
    matvec(A, vout, vec);
    for (i = 0; i < N; i++) total += vec[i];

    return total;
}

int main(void) {
    double *A = (double *)malloc(N * N * sizeof(double));
    double *B = (double *)malloc(N * N * sizeof(double));
    double *C = (double *)malloc(N * N * sizeof(double));
    double *D = (double *)malloc(N * N * sizeof(double));
    double *Bt = (double *)malloc(N * N * sizeof(double));
    double *vec = (double *)malloc(N * sizeof(double));
    double *vout = (double *)malloc(N * sizeof(double));
    int i;

    lcg_state = 12345;
    for (i = 0; i < N * N; i++) A[i] = (double)lcg_rand() / 32768.0;
    for (i = 0; i < N * N; i++) B[i] = (double)lcg_rand() / 32768.0;
    for (i = 0; i < N; i++) vec[i] = (double)lcg_rand() / 32768.0;

    /* Warmup */
    volatile double sink;
    for (i = 0; i < 5; i++) {
        sink = workload(A, B, C, D, Bt, vec, vout);
    }

    /* Timing: 201 runs, 10% trimmed mean */
    long long times[201];
    struct timespec t0, t1;
    for (i = 0; i < 201; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(A, B, C, D, Bt, vec, vout);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);

    free(A); free(B); free(C); free(D); free(Bt); free(vec); free(vout);
    return 0;
}
