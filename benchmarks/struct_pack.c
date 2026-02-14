/*
 * Targets: sroa, mem2reg, instcombine, inline, gvn, early-cse
 *
 * Uses local structs and passes them by value through helper functions.
 * SROA should decompose these into scalar registers. GVN/early-cse
 * should eliminate redundant field accesses.
 */
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

typedef struct {
    double x, y, z;
} Vec3;

typedef struct {
    double real, imag;
} Complex;

typedef struct {
    int lo, hi, mid;
    int found;
} SearchState;

/* Return by value — SROA should decompose the return struct */
static Vec3 vec3_add(Vec3 a, Vec3 b) {
    Vec3 r;
    r.x = a.x + b.x;
    r.y = a.y + b.y;
    r.z = a.z + b.z;
    return r;
}

static Vec3 vec3_scale(Vec3 a, double s) {
    Vec3 r;
    r.x = a.x * s;
    r.y = a.y * s;
    r.z = a.z * s;
    return r;
}

static double vec3_dot(Vec3 a, Vec3 b) {
    return a.x * b.x + a.y * b.y + a.z * b.z;
}

static double vec3_length_sq(Vec3 a) {
    /* Redundant field access — early-cse/gvn target: a.x used twice */
    return a.x * a.x + a.y * a.y + a.z * a.z;
}

static Vec3 vec3_normalize(Vec3 a) {
    double len_sq = vec3_length_sq(a);
    double inv_len = 1.0 / (len_sq + 1e-12);
    /* sqrt approximation to avoid -lm: Newton iteration */
    double guess = inv_len;
    guess = 0.5 * (guess + inv_len / guess);
    guess = 0.5 * (guess + inv_len / guess);
    return vec3_scale(a, guess);
}

/* Complex multiply — struct by value */
static Complex complex_mul(Complex a, Complex b) {
    Complex r;
    r.real = a.real * b.real - a.imag * b.imag;
    r.imag = a.real * b.imag + a.imag * b.real;
    return r;
}

static Complex complex_add(Complex a, Complex b) {
    Complex r;
    r.real = a.real + b.real;
    r.imag = a.imag + b.imag;
    return r;
}

/* Local struct on stack — mem2reg target */
static int search_helper(const int *arr, int n, int target) {
    SearchState s;
    s.lo = 0;
    s.hi = n - 1;
    s.found = -1;
    while (s.lo <= s.hi) {
        s.mid = s.lo + (s.hi - s.lo) / 2;
        if (arr[s.mid] == target) { s.found = s.mid; return s.found; }
        else if (arr[s.mid] < target) s.lo = s.mid + 1;
        else s.hi = s.mid - 1;
    }
    return s.found;
}

#define VEC_N 2000
#define COMPLEX_ITERS 200

static long long workload(double *data, int *sorted_arr) {
    long long total = 0;
    int i;

    /* Vec3 operations — many small structs passed by value */
    Vec3 acc = {0.0, 0.0, 0.0};
    for (i = 0; i + 2 < VEC_N; i += 3) {
        Vec3 v = {data[i], data[i + 1], data[i + 2]};
        Vec3 w = {data[i + 2], data[i], data[i + 1]};
        acc = vec3_add(acc, v);
        acc = vec3_add(acc, vec3_scale(w, 0.5));
        total += (long long)(vec3_dot(v, w) * 1000.0);
    }
    Vec3 n = vec3_normalize(acc);
    total += (long long)(n.x * 1e6);

    /* Complex number iteration — struct by value */
    Complex z = {0.0, 0.0};
    Complex c = {0.3, 0.5};
    for (i = 0; i < COMPLEX_ITERS; i++) {
        z = complex_add(complex_mul(z, z), c);
        /* Clamp to prevent overflow */
        if (z.real > 1e6) z.real = 1e6;
        if (z.imag > 1e6) z.imag = 1e6;
        if (z.real < -1e6) z.real = -1e6;
        if (z.imag < -1e6) z.imag = -1e6;
    }
    total += (long long)(z.real * 1000.0);

    /* Search with struct state — mem2reg target */
    for (i = 0; i < 200; i++) {
        int target = sorted_arr[lcg_rand() % VEC_N];
        total += search_helper(sorted_arr, VEC_N, target);
    }

    return total;
}

int main(void) {
    double *data = (double *)malloc(VEC_N * sizeof(double));
    int *sorted_arr = (int *)malloc(VEC_N * sizeof(int));
    int i;

    lcg_state = 12345;
    for (i = 0; i < VEC_N; i++) {
        data[i] = (double)lcg_rand() / 32768.0;
        sorted_arr[i] = i * 3 + 1;  /* Already sorted */
    }

    /* Warmup */
    volatile long long sink;
    for (i = 0; i < 5; i++) {
        sink = workload(data, sorted_arr);
    }

    /* Timing */
    long long times[50];
    struct timespec t0, t1;
    for (i = 0; i < 50; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(data, sorted_arr);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);

    free(data);
    free(sorted_arr);
    return 0;
}
