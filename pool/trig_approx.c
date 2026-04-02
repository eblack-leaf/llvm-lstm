#include "bench_timing.h"

/*
 * Trigonometric approximations using polynomial evaluation and
 * range reduction — high fcmp, high call count, float-heavy.
 * Many small helper functions create inlining opportunities.
 */

#define N 2000
#define PI_APPROX 3.14159265358979

static double fabs_d(double x) { return x < 0.0 ? -x : x; }

/* Reduce angle to [-PI, PI] via repeated subtraction */
static double reduce_angle(double x) {
    while (x > PI_APPROX) x -= 2.0 * PI_APPROX;
    while (x < -PI_APPROX) x += 2.0 * PI_APPROX;
    return x;
}

/* Horner's method polynomial evaluation */
static double poly_eval(const double *coeffs, int n, double x) {
    double result = coeffs[n - 1];
    int i;
    for (i = n - 2; i >= 0; i--) {
        result = result * x + coeffs[i];
    }
    return result;
}

/* sin(x) via Taylor series: x - x^3/6 + x^5/120 - x^7/5040 + x^9/362880 */
static double sin_approx(double x) {
    x = reduce_angle(x);
    double x2 = x * x;
    double coeffs[] = {0.0, 1.0, 0.0, -1.0/6.0, 0.0, 1.0/120.0,
                       0.0, -1.0/5040.0, 0.0, 1.0/362880.0};
    return poly_eval(coeffs, 10, x);
}

/* cos(x) via Taylor: 1 - x^2/2 + x^4/24 - x^6/720 + x^8/40320 */
static double cos_approx(double x) {
    x = reduce_angle(x);
    double coeffs[] = {1.0, 0.0, -1.0/2.0, 0.0, 1.0/24.0,
                       0.0, -1.0/720.0, 0.0, 1.0/40320.0};
    return poly_eval(coeffs, 9, x);
}

/* atan(x) for |x| <= 1 via polynomial */
static double atan_small(double x) {
    double x2 = x * x;
    return x * (1.0 - x2 * (1.0/3.0 - x2 * (1.0/5.0 - x2 * (1.0/7.0 - x2 / 9.0))));
}

/* atan2 approximation */
static double atan2_approx(double y, double x) {
    if (fabs_d(x) < 1e-10 && fabs_d(y) < 1e-10) return 0.0;
    if (fabs_d(x) > fabs_d(y)) {
        double r = y / x;
        double angle = atan_small(r);
        return x < 0.0 ? (y >= 0.0 ? angle + PI_APPROX : angle - PI_APPROX) : angle;
    } else {
        double r = x / y;
        double angle = PI_APPROX / 2.0 - atan_small(r);
        return y < 0.0 ? angle - PI_APPROX : angle;
    }
}

/* exp(x) approximation for small x via Pade */
static double exp_approx(double x) {
    /* Clamp to prevent overflow */
    if (x > 10.0) x = 10.0;
    if (x < -10.0) x = -10.0;

    /* Range reduction: exp(x) = exp(x/16)^16 */
    double r = x / 16.0;
    /* Pade(2,2) for exp(r) */
    double r2 = r * r;
    double num = 1.0 + r * 0.5 + r2 / 12.0;
    double den = 1.0 - r * 0.5 + r2 / 12.0;
    double result = num / den;

    /* Square 4 times */
    result = result * result;
    result = result * result;
    result = result * result;
    result = result * result;
    return result;
}

/* log(x) approximation via log(x) = 2 * atanh((x-1)/(x+1)) */
static double log_approx(double x) {
    if (x <= 0.0) return -1e10;

    /* Normalize to [0.5, 2) */
    int exp_adj = 0;
    while (x > 2.0) { x *= 0.5; exp_adj++; }
    while (x < 0.5) { x *= 2.0; exp_adj--; }

    double z = (x - 1.0) / (x + 1.0);
    double z2 = z * z;
    /* atanh(z) ≈ z + z^3/3 + z^5/5 + z^7/7 */
    double result = z * (1.0 + z2 * (1.0/3.0 + z2 * (1.0/5.0 + z2 / 7.0)));
    return 2.0 * result + (double)exp_adj * 0.693147180559945;
}

/* 2D rotation */
static void rotate(double *x, double *y, double angle) {
    double c = cos_approx(angle);
    double s = sin_approx(angle);
    double nx = *x * c - *y * s;
    double ny = *x * s + *y * c;
    *x = nx;
    *y = ny;
}

/* Magnitude */
static double magnitude(double x, double y) {
    double s = x * x + y * y;
    /* Newton sqrt */
    if (s <= 0.0) return 0.0;
    double g = s * 0.5;
    g = 0.5 * (g + s / g);
    g = 0.5 * (g + s / g);
    g = 0.5 * (g + s / g);
    return g;
}

static long long workload(double *xs, double *ys) {
    double sum = 0.0;
    int i;

    /* Trig evaluations */
    for (i = 0; i < N; i++) {
        double angle = xs[i];
        sum += sin_approx(angle);
        sum += cos_approx(angle);
        /* sin^2 + cos^2 should ≈ 1 */
        double s = sin_approx(angle);
        double c = cos_approx(angle);
        sum += s * s + c * c;
    }

    /* atan2 */
    for (i = 0; i < N; i++) {
        sum += atan2_approx(ys[i], xs[i]);
    }

    /* exp/log round-trip */
    for (i = 0; i < N; i++) {
        double val = fabs_d(xs[i]) + 0.01;
        double l = log_approx(val);
        double e = exp_approx(l);
        sum += fabs_d(e - val);  /* Should be near 0 */
    }

    /* 2D rotations */
    for (i = 0; i < N; i++) {
        double px = xs[i], py = ys[i];
        rotate(&px, &py, 0.1 * (double)i);
        sum += magnitude(px, py);
    }

    /* Coordinate transforms */
    for (i = 0; i < N; i++) {
        double r = magnitude(xs[i], ys[i]);
        double theta = atan2_approx(ys[i], xs[i]);
        /* Polar → Cartesian → Polar roundtrip */
        double nx = r * cos_approx(theta);
        double ny = r * sin_approx(theta);
        double r2 = magnitude(nx, ny);
        sum += fabs_d(r - r2);
    }

    return (long long)sum;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    double xs[N], ys[N];
    int i;

    bench_lcg_seed(31);
    for (i = 0; i < N; i++) {
        xs[i] = ((double)((int)(bench_lcg_rand() % 2000) - 1000)) * 0.01;
        ys[i] = ((double)((int)(bench_lcg_rand() % 2000) - 1000)) * 0.01;
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(xs, ys); });

    return 0;
}
