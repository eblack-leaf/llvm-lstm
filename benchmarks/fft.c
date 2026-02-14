#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <string.h>
#include <math.h>

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

#define FFT_N 65536

static unsigned int bit_reverse(unsigned int x, int log2n) {
    unsigned int result = 0;
    int i;
    for (i = 0; i < log2n; i++) {
        result = (result << 1) | (x & 1);
        x >>= 1;
    }
    return result;
}

static double workload(double *re, double *im, double *re_buf, double *im_buf) {
    int log2n = 16; /* log2(65536) */
    unsigned int i;
    int s, j, k;

    /* Copy input to buffer with bit-reversal permutation */
    for (i = 0; i < FFT_N; i++) {
        unsigned int rev = bit_reverse(i, log2n);
        re_buf[rev] = re[i];
        im_buf[rev] = im[i];
    }

    /* Butterfly stages */
    for (s = 1; s <= log2n; s++) {
        int m = 1 << s;
        int half = m >> 1;
        double w_re = cos(-2.0 * M_PI / m);
        double w_im = sin(-2.0 * M_PI / m);
        for (k = 0; k < FFT_N; k += m) {
            double wn_re = 1.0, wn_im = 0.0;
            for (j = 0; j < half; j++) {
                int idx_even = k + j;
                int idx_odd = k + j + half;
                double t_re = wn_re * re_buf[idx_odd] - wn_im * im_buf[idx_odd];
                double t_im = wn_re * im_buf[idx_odd] + wn_im * re_buf[idx_odd];
                re_buf[idx_odd] = re_buf[idx_even] - t_re;
                im_buf[idx_odd] = im_buf[idx_even] - t_im;
                re_buf[idx_even] = re_buf[idx_even] + t_re;
                im_buf[idx_even] = im_buf[idx_even] + t_im;
                double tmp = wn_re * w_re - wn_im * w_im;
                wn_im = wn_re * w_im + wn_im * w_re;
                wn_re = tmp;
            }
        }
    }

    /* Sum magnitudes to prevent optimization */
    double total = 0.0;
    for (i = 0; i < FFT_N; i++) {
        total += re_buf[i] * re_buf[i] + im_buf[i] * im_buf[i];
    }
    return total;
}

int main(void) {
    double *re = (double *)malloc(FFT_N * sizeof(double));
    double *im = (double *)malloc(FFT_N * sizeof(double));
    double *re_buf = (double *)malloc(FFT_N * sizeof(double));
    double *im_buf = (double *)malloc(FFT_N * sizeof(double));
    unsigned int i;

    lcg_state = 12345;
    for (i = 0; i < FFT_N; i++) {
        re[i] = (double)lcg_rand() / 32768.0 - 0.5;
        im[i] = 0.0;
    }

    /* Warmup */
    volatile double sink;
    for (i = 0; i < 5; i++) {
        sink = workload(re, im, re_buf, im_buf);
    }

    /* Timing */
    long long times[50];
    struct timespec t0, t1;
    for (i = 0; i < 50; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(re, im, re_buf, im_buf);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);

    free(re);
    free(im);
    free(re_buf);
    free(im_buf);
    return 0;
}
