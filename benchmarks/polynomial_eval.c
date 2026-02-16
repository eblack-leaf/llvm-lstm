#include "bench_timing.h"

#define DEGREE 100
#define NPOINTS 500

static double workload(double *coeffs, double *points, double *results) {
    int i, j;
    for (i = 0; i < NPOINTS; i++) {
        double x = points[i];
        double val = coeffs[DEGREE];
        for (j = DEGREE - 1; j >= 0; j--) {
            val = val * x + coeffs[j];
        }
        results[i] = val;
    }
    double total = 0.0;
    for (i = 0; i < NPOINTS; i++) {
        total += results[i];
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
