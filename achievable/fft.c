#include "bench_timing.h"
#include <math.h>

#define FFT_N 1024
#define LOG2N 10

static unsigned int bit_reverse(unsigned int x, int log2n) {
    unsigned int result = 0;
    int i;
    for (i = 0; i < log2n; i++) {
        result = (result << 1) | (x & 1);
        x >>= 1;
    }
    return result;
}

/* Bit-reversal permutation copy */
static void bit_reverse_copy(const double *src_re, const double *src_im,
                             double *dst_re, double *dst_im, int n, int log2n) {
    unsigned int i;
    for (i = 0; i < (unsigned int)n; i++) {
        unsigned int rev = bit_reverse(i, log2n);
        dst_re[rev] = src_re[i];
        dst_im[rev] = src_im[i];
    }
}

/* Core butterfly computation — separate function for inline decisions */
static void butterfly(double *re, double *im, int idx_even, int idx_odd,
                      double wn_re, double wn_im) {
    double t_re = wn_re * re[idx_odd] - wn_im * im[idx_odd];
    double t_im = wn_re * im[idx_odd] + wn_im * re[idx_odd];
    re[idx_odd] = re[idx_even] - t_re;
    im[idx_odd] = im[idx_even] - t_im;
    re[idx_even] = re[idx_even] + t_re;
    im[idx_even] = im[idx_even] + t_im;
}

/* Forward FFT */
static void fft_forward(const double *in_re, const double *in_im,
                        double *out_re, double *out_im) {
    int s, j, k;
    bit_reverse_copy(in_re, in_im, out_re, out_im, FFT_N, LOG2N);

    for (s = 1; s <= LOG2N; s++) {
        int m = 1 << s;
        int half = m >> 1;
        double w_re = cos(-2.0 * M_PI / m);
        double w_im = sin(-2.0 * M_PI / m);
        for (k = 0; k < FFT_N; k += m) {
            double wn_re = 1.0, wn_im = 0.0;
            for (j = 0; j < half; j++) {
                butterfly(out_re, out_im, k + j, k + j + half, wn_re, wn_im);
                double tmp = wn_re * w_re - wn_im * w_im;
                wn_im = wn_re * w_im + wn_im * w_re;
                wn_re = tmp;
            }
        }
    }
}

/* Inverse FFT — conjugate input, forward FFT, conjugate and scale output */
static void fft_inverse(const double *in_re, const double *in_im,
                        double *out_re, double *out_im,
                        double *tmp_re, double *tmp_im) {
    int i;
    /* Conjugate */
    for (i = 0; i < FFT_N; i++) {
        tmp_re[i] = in_re[i];
        tmp_im[i] = -in_im[i];
    }
    fft_forward(tmp_re, tmp_im, out_re, out_im);
    /* Conjugate and scale */
    double scale = 1.0 / FFT_N;
    for (i = 0; i < FFT_N; i++) {
        out_re[i] *= scale;
        out_im[i] = -out_im[i] * scale;
    }
}

/* Power spectrum: |X[k]|^2 */
static void power_spectrum(const double *re, const double *im, double *power, int n) {
    int i;
    for (i = 0; i < n; i++) {
        power[i] = re[i] * re[i] + im[i] * im[i];
    }
}

/* Spectral energy in bands — branchy reduction */
static void band_energy(const double *power, int n,
                        double *low, double *mid, double *high) {
    int i;
    *low = 0.0; *mid = 0.0; *high = 0.0;
    int third = n / 3;
    for (i = 0; i < n; i++) {
        if (i < third)
            *low += power[i];
        else if (i < 2 * third)
            *mid += power[i];
        else
            *high += power[i];
    }
}

static double workload(double *re, double *im,
                       double *buf1_re, double *buf1_im,
                       double *buf2_re, double *buf2_im,
                       double *power) {
    double total = 0.0;

    /* Forward FFT */
    fft_forward(re, im, buf1_re, buf1_im);

    /* Power spectrum */
    power_spectrum(buf1_re, buf1_im, power, FFT_N);
    double low, mid, high;
    band_energy(power, FFT_N, &low, &mid, &high);
    total += low + mid + high;

    /* Inverse FFT to reconstruct */
    fft_inverse(buf1_re, buf1_im, buf2_re, buf2_im, power, power);
    /* Sum reconstruction error */
    int i;
    for (i = 0; i < FFT_N; i++) {
        double err = buf2_re[i] - re[i];
        total += err * err;
    }

    /* Second forward FFT on the power spectrum itself */
    fft_forward(power, im, buf1_re, buf1_im);
    for (i = 0; i < FFT_N; i++) {
        total += buf1_re[i] * buf1_re[i] + buf1_im[i] * buf1_im[i];
    }

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    double *re = (double *)malloc(FFT_N * sizeof(double));
    double *im = (double *)malloc(FFT_N * sizeof(double));
    double *buf1_re = (double *)malloc(FFT_N * sizeof(double));
    double *buf1_im = (double *)malloc(FFT_N * sizeof(double));
    double *buf2_re = (double *)malloc(FFT_N * sizeof(double));
    double *buf2_im = (double *)malloc(FFT_N * sizeof(double));
    double *power = (double *)malloc(FFT_N * sizeof(double));
    int i;

    bench_lcg_seed(12345);
    for (i = 0; i < FFT_N; i++) {
        re[i] = (double)bench_lcg_rand() / 32768.0 - 0.5;
        im[i] = 0.0;
    }

    volatile double sink;
    BENCH_TIME(niters, { sink = workload(re, im, buf1_re, buf1_im, buf2_re, buf2_im, power); });

    free(re); free(im);
    free(buf1_re); free(buf1_im);
    free(buf2_re); free(buf2_im);
    free(power);
    return 0;
}
