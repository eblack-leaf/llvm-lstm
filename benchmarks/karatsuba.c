#include "bench_timing.h"

/* Karatsuba multiplication of big integers stored as arrays of digits (base 10000) */
#define BASE 10000
#define DIGITS 256  /* number of base-10000 digits */

static void add(const int *a, const int *b, int *c, int n) {
    int carry = 0, i;
    for (i = 0; i < n; i++) {
        int s = a[i] + b[i] + carry;
        c[i] = s % BASE;
        carry = s / BASE;
    }
}

static void sub(const int *a, const int *b, int *c, int n) {
    int borrow = 0, i;
    for (i = 0; i < n; i++) {
        int s = a[i] - b[i] - borrow;
        if (s < 0) { s += BASE; borrow = 1; }
        else borrow = 0;
        c[i] = s;
    }
}

static void karatsuba(const int *a, const int *b, int *c, int n) {
    int i;
    if (n <= 32) {
        /* Schoolbook for small sizes */
        memset(c, 0, 2 * n * sizeof(int));
        for (i = 0; i < n; i++) {
            int carry = 0, j;
            for (j = 0; j < n; j++) {
                long long p = (long long)a[i] * b[j] + c[i + j] + carry;
                c[i + j] = (int)(p % BASE);
                carry = (int)(p / BASE);
            }
            c[i + n] += carry;
        }
        return;
    }

    int half = n / 2;
    const int *a0 = a, *a1 = a + half;
    const int *b0 = b, *b1 = b + half;

    int *z0 = (int *)calloc(2 * half, sizeof(int));
    int *z2 = (int *)calloc(2 * half, sizeof(int));
    int *z1 = (int *)calloc(2 * half, sizeof(int));
    int *ta = (int *)calloc(half, sizeof(int));
    int *tb = (int *)calloc(half, sizeof(int));
    int *tm = (int *)calloc(2 * half, sizeof(int));

    /* z0 = a0 * b0 */
    karatsuba(a0, b0, z0, half);
    /* z2 = a1 * b1 */
    karatsuba(a1, b1, z2, half);
    /* z1 = (a0+a1)*(b0+b1) - z0 - z2 */
    add(a0, a1, ta, half);
    add(b0, b1, tb, half);
    karatsuba(ta, tb, tm, half);
    sub(tm, z0, z1, 2 * half);
    sub(z1, z2, z1, 2 * half);

    /* Combine: c = z0 + z1*BASE^half + z2*BASE^n */
    memset(c, 0, 2 * n * sizeof(int));
    for (i = 0; i < 2 * half; i++) {
        c[i] += z0[i];
        c[i + half] += z1[i];
        c[i + n] += z2[i];
    }
    /* Propagate carries */
    int carry = 0;
    for (i = 0; i < 2 * n; i++) {
        c[i] += carry;
        carry = c[i] / BASE;
        c[i] %= BASE;
    }

    free(z0); free(z2); free(z1);
    free(ta); free(tb); free(tm);
}

static void workload(int *a, int *b, int *c) {
    karatsuba(a, b, c, DIGITS);
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    int *a = (int *)calloc(DIGITS, sizeof(int));
    int *b = (int *)calloc(DIGITS, sizeof(int));
    int *c = (int *)calloc(2 * DIGITS, sizeof(int));
    int i;

    bench_lcg_seed(12345);
    for (i = 0; i < DIGITS; i++) {
        a[i] = bench_lcg_rand() % BASE;
        b[i] = bench_lcg_rand() % BASE;
    }

    BENCH_TIME(niters, { workload(a, b, c); });

    free(a); free(b); free(c);
    return 0;
}
