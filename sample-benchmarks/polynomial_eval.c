#include "bench_timing.h"

#define DEGREE 100
#define NPOINTS 500
#define NAIVE_LIMIT 50

/* 1. Horner evaluation: O(n) */
static double poly_horner(const double *coeffs, double x, int degree) {
    double val = coeffs[degree];
    for (int j = degree - 1; j >= 0; j--) {
        val = val * x + coeffs[j];
    }
    return val;
}

/* 2. Forward power evaluation: O(n) */
static double poly_forward(const double *coeffs, double x, int degree) {
    double sum = coeffs[0];
    double xpow = x;
    for (int j = 1; j <= degree; j++) {
        sum += coeffs[j] * xpow;
        xpow *= x;
    }
    return sum;
}

/* 3. Naive evaluation: O(n^2) — recomputes x^j from scratch each term */
static double poly_naive(const double *coeffs, double x, int degree) {
    double sum = coeffs[0];
    for (int j = 1; j <= degree; j++) {
        double term = coeffs[j];
        for (int k = 0; k < j; k++) {
            term *= x;
        }
        sum += term;
    }
    return sum;
}

/* 4. Estrin's method: pairwise combination, O(n log n) parallelism structure */
static double poly_estrin(const double *coeffs, double x, int degree) {
    double buf[DEGREE + 1];
    int len = degree + 1;
    /* Copy coefficients into working buffer */
    for (int i = 0; i < len; i++) {
        buf[i] = coeffs[i];
    }
    double xk = x; /* current power: x, x^2, x^4, ... */
    while (len > 1) {
        int newlen = 0;
        int i;
        for (i = 0; i + 1 < len; i += 2) {
            buf[newlen++] = buf[i] + buf[i + 1] * xk;
        }
        /* If odd number of elements, carry the last one through */
        if (i < len) {
            buf[newlen++] = buf[i];
        }
        len = newlen;
        xk = xk * xk;
    }
    return buf[0];
}

/* 5. Derivative evaluation via Horner on derived coefficients */
static double poly_derivative(const double *coeffs, double x, int degree) {
    if (degree < 1) return 0.0;
    double dval = (double)degree * coeffs[degree];
    for (int j = degree - 1; j >= 1; j--) {
        dval = dval * x + (double)j * coeffs[j];
    }
    return dval;
}

/* 6. Clamped Horner: two branches per iteration */
static double poly_clamped(const double *coeffs, double x, int degree) {
    double val = coeffs[degree];
    for (int j = degree - 1; j >= 0; j--) {
        val = val * x + coeffs[j];
        if (val > 1e10) val = 1e10;
        if (val < -1e10) val = -1e10;
    }
    return val;
}

/* 7. Batch evaluation wrapper — creates call edge to poly_horner */
static void poly_eval_batch(const double *coeffs, const double *xs,
                            double *out, int npoints, int degree) {
    for (int i = 0; i < npoints; i++) {
        out[i] = poly_horner(coeffs, xs[i], degree);
    }
}

static double workload(double *coeffs, double *points, double *results) {
    int i;
    double total = 0.0;

    /* Batch Horner evaluation for all points */
    poly_eval_batch(coeffs, points, results, NPOINTS, DEGREE);
    for (i = 0; i < NPOINTS; i++) {
        total += results[i];
    }

    /* Forward, derivative, and clamped for all points */
    for (i = 0; i < NPOINTS; i++) {
        double x = points[i];
        total += poly_forward(coeffs, x, DEGREE);
        total += poly_derivative(coeffs, x, DEGREE);
        total += poly_clamped(coeffs, x, DEGREE);
    }

    /* Naive on first NAIVE_LIMIT points (expensive O(n^2) per point) */
    for (i = 0; i < NAIVE_LIMIT; i++) {
        total += poly_naive(coeffs, points[i], DEGREE);
    }

    /* Estrin on all points */
    for (i = 0; i < NPOINTS; i++) {
        total += poly_estrin(coeffs, points[i], DEGREE);
    }

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    double *coeffs = (double *)malloc((DEGREE + 1) * sizeof(double));
    double *points = (double *)malloc(NPOINTS * sizeof(double));
    double *results = (double *)malloc(NPOINTS * sizeof(double));
    int i;

    bench_lcg_seed(12345);
    for (i = 0; i <= DEGREE; i++) {
        coeffs[i] = ((double)bench_lcg_rand() / 32768.0 - 0.5) * 0.001;
    }
    for (i = 0; i < NPOINTS; i++) {
        points[i] = (double)bench_lcg_rand() / 32768.0 * 2.0 - 1.0;
    }

    volatile double sink;
    BENCH_TIME(niters, { sink = workload(coeffs, points, results); });

    free(coeffs);
    free(points);
    free(results);
    return 0;
}
