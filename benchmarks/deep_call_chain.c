/*
 * Targets: inline, reassociate, licm, instcombine, early-cse, gvn
 *
 * Deep function call chains (6+ levels) for inlining decisions.
 * Algebraic expressions written in suboptimal associativity for reassociate.
 * Loop-invariant computations buried inside nested calls for licm.
 * Multiple independent call chains to test selective inlining.
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

/* ---- Chain 1: Neural network simulation (6 levels) ---- */

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
    double abs_x = x > 0 ? x : -x;
    return x / (1.0 + abs_x);
}

static double activate_relu(double x) {
    return x > 0.0 ? x : 0.01 * x;  /* leaky relu */
}

static double neuron(double x, double w, double b) {
    return activate(add_bias(scale(x, w), b));
}

static double neuron_relu(double x, double w, double b) {
    return activate_relu(add_bias(scale(x, w), b));
}

static double layer(double x, double w1, double w2, double b1, double b2) {
    double h = neuron(x, w1, b1);
    return neuron(h, w2, b2);
}

static double layer_relu(double x, double w1, double w2, double b1, double b2) {
    double h = neuron_relu(x, w1, b1);
    return neuron_relu(h, w2, b2);
}

static double network(double x, const double *params) {
    double h1 = layer(x, params[0], params[1], params[2], params[3]);
    double h2 = layer(h1, params[4], params[5], params[6], params[7]);
    double h3 = layer(h2, params[8], params[9], params[10], params[11]);
    return h3;
}

/* Second network with different activation — tests if inliner treats them differently */
static double network_relu(double x, const double *params) {
    double h1 = layer_relu(x, params[0], params[1], params[2], params[3]);
    double h2 = layer_relu(h1, params[4], params[5], params[6], params[7]);
    double h3 = layer(h2, params[8], params[9], params[10], params[11]); /* mixed */
    return h3;
}

/* ---- Chain 2: Recursive-style computation flattened into chain ---- */

static double step_a(double x, double k) { return x * k + 1.0; }
static double step_b(double x, double k) { return (x + k) * 0.5; }
static double step_c(double x, double k) { return x - k * 0.1; }

static double chain_abc(double x, double k1, double k2, double k3) {
    return step_c(step_b(step_a(x, k1), k2), k3);
}

static double chain_deep(double x, double k1, double k2, double k3) {
    double r1 = chain_abc(x, k1, k2, k3);
    double r2 = chain_abc(r1, k2, k3, k1);
    return step_a(r2, k1 + k2);
}

/* ---- Suboptimal algebraic expressions for reassociate ---- */

static double bad_poly(double x, double a, double b, double c, double d) {
    double t1 = x * a + x * b;       /* could be x * (a + b) */
    double t2 = t1 + x * c + x * d;  /* could be x * (a+b+c+d) */
    double t3 = x * a + t2;          /* early-cse: x*a already computed */
    return t3;
}

/* Extended polynomial with more reassociation opportunities */
static double bad_poly2(double x, double a, double b, double c) {
    /* (x*a*b) + (x*a*c) should become x*a*(b+c) */
    double t1 = x * a * b;
    double t2 = x * a * c;
    /* And then add x*a again — CSE + reassociate */
    double t3 = x * a;
    return t1 + t2 + t3;
}

static double sum_of_products(const double *arr, int n, double k1, double k2) {
    double total = 0.0;
    int i;
    for (i = 0; i < n; i++) {
        total += (arr[i] * k1) * k2 + (arr[i] * k2) * k1;
    }
    return total;
}

/* ---- Loop-invariant computation inside call chain ---- */

static double invariant_helper(double x, double inv_a, double inv_b) {
    double factor = inv_a * inv_b + inv_a;
    return x * factor;
}

static double process_element(double x, double inv_a, double inv_b, double bias) {
    return add_bias(invariant_helper(x, inv_a, inv_b), bias);
}

/* Deeper invariant chain: 3 levels of invariant computation */
static double deep_invariant(double x, double a, double b, double c) {
    double f1 = a * b;       /* invariant */
    double f2 = f1 + c;     /* invariant */
    double f3 = f2 * a;     /* invariant */
    return x * f3 + x * f2; /* x * (f3 + f2) after reassociate */
}

/* ---- GVN target: redundant loads through call chain ---- */
static double gvn_target(const double *arr, int i, int n) {
    /* Load arr[i] multiple times through different paths */
    double v1 = arr[i];
    double v2 = arr[i];  /* GVN should eliminate */
    double v3 = (i + 1 < n) ? arr[i + 1] : 0.0;
    double v4 = arr[i];  /* GVN again */
    return v1 + v2 + v3 + v4;
}

#define N 3000
#define N_PARAMS 12

static double workload(double *arr, double *params) {
    double total = 0.0;
    int i;

    /* Chain 1a: sigmoid network */
    for (i = 0; i < N; i++) {
        total += network(arr[i], params);
    }

    /* Chain 1b: relu network */
    for (i = 0; i < N; i++) {
        total += network_relu(arr[i], params);
    }

    /* Chain 2: abc chains */
    for (i = 0; i < N; i++) {
        total += chain_deep(arr[i], params[0], params[1], params[2]);
    }

    /* Bad algebraic expressions */
    for (i = 0; i < N; i++) {
        total += bad_poly(arr[i], 1.5, 2.3, 0.7, 1.1);
        total += bad_poly2(arr[i], 1.5, 2.3, 0.7);
    }

    /* Loop-invariant buried in call chain */
    {
        double inv_a = params[0] + params[1];
        double inv_b = params[2] * params[3];
        for (i = 0; i < N; i++) {
            total += process_element(arr[i], inv_a, inv_b, params[4]);
        }
    }

    /* Deep invariant */
    {
        double a = params[5], b = params[6], c = params[7];
        for (i = 0; i < N; i++) {
            total += deep_invariant(arr[i], a, b, c);
        }
    }

    /* Sum of products with reassociation opportunity */
    total += sum_of_products(arr, N, params[5], params[6]);

    /* GVN target */
    for (i = 0; i < N; i++) {
        total += gvn_target(arr, i, N);
    }

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
    long long times[201];
    struct timespec t0, t1;
    for (i = 0; i < 201; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(arr, params);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);

    free(arr);
    return 0;
}
