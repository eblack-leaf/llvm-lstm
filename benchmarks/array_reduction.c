#include <stdio.h>
#include <stdlib.h>
#include <time.h>

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

    return total;
}

int main(void) {
    float *arr = (float *)malloc(ARR_N * sizeof(float));
    float *scratch = (float *)malloc(ARR_N * sizeof(float));
    int i;

    lcg_state = 12345;
    for (i = 0; i < ARR_N; i++) {
        arr[i] = (float)lcg_rand() / 32768.0f - 0.5f;
    }

    /* Warmup */
    volatile float sink;
    for (i = 0; i < 5; i++) {
        sink = workload(arr, scratch);
    }

    /* Timing: 201 runs, 10% trimmed mean */
    long long times[201];
    struct timespec t0, t1;
    for (i = 0; i < 201; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(arr, scratch);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    /* Drop bottom/top 10% (20 each), average middle 161 */
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);

    free(arr); free(scratch);
    return 0;
}
