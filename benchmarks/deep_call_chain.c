/*
 * Targets: inline, reassociate, licm, instcombine, early-cse, gvn
 *
 * Deep function call chains (6+ levels) for inlining decisions.
 * Algebraic expressions written in suboptimal associativity for reassociate.
 * Loop-invariant computations buried inside nested calls for licm.
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

/* ---- Deep call chain: 6 levels of tiny functions ---- */

static double add_bias(double x, double bias) {
    return x + bias;
}

static double scale(double x, double factor) {
    return x * factor;
}

static double clamp(double x, double lo, double hi) {
    if (x < lo) return lo;
    if (x > hi) return hi;
    return x;
}

static double activate(double x) {
    /* Approximate sigmoid without -lm: x / (1 + |x|) */
    double abs_x = x > 0 ? x : -x;
    return x / (1.0 + abs_x);
}

static double neuron(double x, double w, double b) {
    return activate(add_bias(scale(x, w), b));
}

static double layer(double x, double w1, double w2, double b1, double b2) {
    double h = neuron(x, w1, b1);
    return neuron(h, w2, b2);
}

static double network(double x, const double *params) {
    /* 3-layer deep chain: each layer calls layer->neuron->activate->scale->add_bias */
    double h1 = layer(x, params[0], params[1], params[2], params[3]);
    double h2 = layer(h1, params[4], params[5], params[6], params[7]);
    double h3 = layer(h2, params[8], params[9], params[10], params[11]);
    return h3;
}

/* ---- Suboptimal algebraic expressions for reassociate ---- */

/*
 * Computes a polynomial but with bad associativity:
 *   (((a + b) + c) + d) instead of balanced tree
 * Also has redundant common subexpressions for early-cse.
 */
static double bad_poly(double x, double a, double b, double c, double d) {
    /* reassociate target: left-leaning chain */
    double t1 = x * a + x * b;       /* could be x * (a + b) */
    double t2 = t1 + x * c + x * d;  /* could be x * (a+b+c+d) */
    /* Redundant subexpression: x*a computed again */
    double t3 = x * a + t2;          /* early-cse: x*a already computed */
    return t3;
}

/*
 * Sum of products with suboptimal ordering.
 * reassociate can reorder to enable more constant folding.
 */
static double sum_of_products(const double *arr, int n, double k1, double k2) {
    double total = 0.0;
    int i;
    for (i = 0; i < n; i++) {
        /* (arr[i] * k1) * k2 can be reassociated to arr[i] * (k1 * k2)
         * where k1*k2 is loop-invariant — licm + reassociate combo */
        total += (arr[i] * k1) * k2 + (arr[i] * k2) * k1;
    }
    return total;
}

/* ---- Loop-invariant computation inside call chain ---- */

static double invariant_helper(double x, double inv_a, double inv_b) {
    /* inv_a and inv_b don't change across loop iterations,
     * but they're passed through a call chain so licm needs
     * inlining first to see the invariance */
    double factor = inv_a * inv_b + inv_a;  /* loop-invariant after inlining */
    return x * factor;
}

static double process_element(double x, double inv_a, double inv_b, double bias) {
    return add_bias(invariant_helper(x, inv_a, inv_b), bias);
}

#define N 100000
#define N_PARAMS 12

static double workload(double *arr, double *params) {
    double total = 0.0;
    int i;

    /* Deep call chain: run network on each element */
    for (i = 0; i < N; i++) {
        total += network(arr[i], params);
    }

    /* Bad algebraic expressions */
    for (i = 0; i < N; i++) {
        total += bad_poly(arr[i], 1.5, 2.3, 0.7, 1.1);
    }

    /* Loop-invariant buried in call chain */
    {
        double inv_a = params[0] + params[1];
        double inv_b = params[2] * params[3];
        for (i = 0; i < N; i++) {
            total += process_element(arr[i], inv_a, inv_b, params[4]);
        }
    }

    /* Sum of products with reassociation opportunity */
    total += sum_of_products(arr, N, params[5], params[6]);

    return total;
}

int main(void) {
    double *arr = (double *)malloc(N * sizeof(double));
    double params[N_PARAMS];
    int i;

    lcg_state = 12345;
    for (i = 0; i < N; i++) {
        arr[i] = (double)(lcg_rand() % 1000) / 500.0 - 1.0;
    }
    for (i = 0; i < N_PARAMS; i++) {
        params[i] = (double)(lcg_rand() % 100) / 50.0 - 1.0;
    }

    /* Warmup */
    volatile double sink;
    for (i = 0; i < 5; i++) {
        sink = workload(arr, params);
    }

    /* Timing */
    long long times[50];
    struct timespec t0, t1;
    for (i = 0; i < 50; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(arr, params);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);

    free(arr);
    return 0;
}
