#include "bench_timing.h"

#define ARR_N 4000

/* Multiple reductions in one pass: sum, min, max, sum-of-squares */
static void reduce_basic(const float *arr, int n,
                         float *out_sum, float *out_min, float *out_max, float *out_sos) {
    float sum = 0.0f, mn = arr[0], mx = arr[0], sos = 0.0f;
    int i;
    for (i = 0; i < n; i++) {
        float v = arr[i];
        sum += v;
        if (v < mn) mn = v;
        if (v > mx) mx = v;
        sos += v * v;
    }
    *out_sum = sum;
    *out_min = mn;
    *out_max = mx;
    *out_sos = sos;
}

/* Prefix sum — serial dependency chain, tests loop opts */
static void prefix_sum(const float *arr, float *out, int n) {
    float running = 0.0f;
    int i;
    for (i = 0; i < n; i++) {
        running += arr[i];
        out[i] = running;
    }
}

/* Conditional accumulation with threshold — branch in reduction loop */
static float reduce_conditional(const float *arr, int n, float threshold) {
    float pos_sum = 0.0f, neg_sum = 0.0f;
    int pos_count = 0, neg_count = 0;
    int i;
    for (i = 0; i < n; i++) {
        if (arr[i] > threshold) {
            pos_sum += arr[i];
            pos_count++;
        } else {
            neg_sum += arr[i];
            neg_count++;
        }
    }
    float pos_avg = pos_count > 0 ? pos_sum / pos_count : 0.0f;
    float neg_avg = neg_count > 0 ? neg_sum / neg_count : 0.0f;
    return pos_avg - neg_avg;
}

/* Windowed moving average — overlapping reductions */
static void moving_avg(const float *arr, float *out, int n, int window) {
    int i, j;
    for (i = 0; i < n - window + 1; i++) {
        float sum = 0.0f;
        for (j = 0; j < window; j++) {
            sum += arr[i + j];
        }
        out[i] = sum / window;
    }
}

/* Histogram binning — reduction with computed index */
#define NUM_BINS 32
static void histogram(const float *arr, int n, int *bins) {
    int i;
    for (i = 0; i < NUM_BINS; i++) bins[i] = 0;
    for (i = 0; i < n; i++) {
        /* Map [-0.5, 0.5] to [0, NUM_BINS) */
        int bin = (int)((arr[i] + 0.5f) * NUM_BINS);
        if (bin < 0) bin = 0;
        if (bin >= NUM_BINS) bin = NUM_BINS - 1;
        bins[bin]++;
    }
}

/* Sliding window maximum — nested loops + comparison branch */
static float reduce_windowed_max(const float *arr, int n, int window) {
    float total = 0.0f;
    for (int i = 0; i <= n - window; i++) {
        float mx = arr[i];
        for (int j = 1; j < window; j++) {
            if (arr[i + j] > mx) mx = arr[i + j];
        }
        total += mx;
    }
    return total;
}

/* Exponential weighted moving average — serial dependency chain */
static float reduce_ewma(const float *arr, int n, float alpha) {
    float running = arr[0];
    float one_minus_alpha = 1.0f - alpha;
    for (int i = 1; i < n; i++) {
        running = alpha * arr[i] + one_minus_alpha * running;
    }
    return running;
}

/* L1 norm with sign branch */
static float reduce_l1_norm(const float *arr, int n) {
    float sum = 0.0f;
    for (int i = 0; i < n; i++) {
        sum += (arr[i] >= 0.0f) ? arr[i] : -arr[i];
    }
    return sum;
}

/* Two-pass variance: mean then sum of squared deviations */
static float reduce_variance(const float *arr, int n) {
    float sum = 0.0f;
    for (int i = 0; i < n; i++) {
        sum += arr[i];
    }
    float mean = sum / n;
    float var_sum = 0.0f;
    for (int i = 0; i < n; i++) {
        float d = arr[i] - mean;
        var_sum += d * d;
    }
    return var_sum / n;
}

/* 2D histogram of adjacent pairs */
static void histogram_2d(const float *arr, int n, int *bins, int nbins) {
    for (int i = 0; i < n - 1; i++) {
        int row = (int)((arr[i] + 0.5f) * nbins);
        int col = (int)((arr[i + 1] + 0.5f) * nbins);
        if (row < 0) row = 0;
        if (row >= nbins) row = nbins - 1;
        if (col < 0) col = 0;
        if (col >= nbins) col = nbins - 1;
        bins[row * nbins + col]++;
    }
}

static float workload(float *arr, float *scratch) {
    float total = 0.0f;
    float s, mn, mx, sos;
    int bins[NUM_BINS];

    reduce_basic(arr, ARR_N, &s, &mn, &mx, &sos);
    total += s + mn + mx + sos;

    prefix_sum(arr, scratch, ARR_N);
    total += scratch[ARR_N - 1];

    total += reduce_conditional(arr, ARR_N, 0.0f);
    total += reduce_conditional(arr, ARR_N, 0.2f);

    moving_avg(arr, scratch, ARR_N, 16);
    total += scratch[ARR_N / 2];

    histogram(arr, ARR_N, bins);
    int i;
    for (i = 0; i < NUM_BINS; i++) total += bins[i];

    total += reduce_windowed_max(arr, ARR_N, 8);
    total += reduce_ewma(arr, ARR_N, 0.1f);
    total += reduce_l1_norm(arr, ARR_N);
    total += reduce_variance(arr, ARR_N);

    total += reduce_conditional(arr, ARR_N, 0.1f);
    total += reduce_conditional(arr, ARR_N, 0.3f);
    total += reduce_conditional(arr, ARR_N, -0.1f);
    total += reduce_conditional(arr, ARR_N, 0.4f);

    int bins2d[16 * 16];
    memset(bins2d, 0, sizeof(bins2d));
    histogram_2d(arr, ARR_N, bins2d, 16);
    for (i = 0; i < 16 * 16; i++) total += bins2d[i];

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    float *arr = (float *)malloc(ARR_N * sizeof(float));
    float *scratch = (float *)malloc(ARR_N * sizeof(float));
    int i;

    bench_lcg_seed(12345);
    for (i = 0; i < ARR_N; i++) {
        arr[i] = (float)bench_lcg_rand() / 32768.0f - 0.5f;
    }

    volatile float sink;
    BENCH_TIME(niters, { sink = workload(arr, scratch); });

    free(arr); free(scratch);
    return 0;
}
