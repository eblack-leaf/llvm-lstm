/*
 * Targets: dse, adce, instcombine, mem2reg, simplifycfg
 */
#include "bench_timing.h"

#define N 50000

static void update_with_dead_stores(int *arr, int *scratch, int n) {
    int i;
    for (i = 0; i < n; i++) {
        scratch[i] = arr[i] * 3 + 7;  /* dead store */
        scratch[i] = arr[i] * arr[i] + arr[i] + 1;
    }
}

static long long compute_with_dead_code(const int *arr, int n) {
    long long used = 0;
    long long unused = 0;
    int i;
    for (i = 0; i < n; i++) {
        used += (long long)arr[i] * arr[i];
        unused += (long long)arr[i] * 17 + arr[i] / 3;
    }
    return used;
}

static long long store_load_pattern(const int *arr, int n) {
    int tmp[4];
    long long total = 0;
    int i;
    for (i = 0; i + 3 < n; i += 4) {
        tmp[0] = arr[i] + 1;
        tmp[1] = arr[i + 1] + 2;
        tmp[2] = arr[i + 2] + 3;
        tmp[3] = arr[i + 3] + 4;
        total += tmp[0] + tmp[1] + tmp[2] + tmp[3];
    }
    return total;
}

static long long multi_phase(int *buf, const int *arr, int n) {
    int i;
    long long total = 0;
    for (i = 0; i < n; i++) {
        buf[i] = arr[i] + 42;  /* dead — overwritten below */
    }
    for (i = 0; i < n; i++) {
        buf[i] = arr[i] * arr[i];
    }
    for (i = 0; i < n; i++) {
        total += buf[i];
    }
    return total;
}

/* --- Variant 1: Conditional dead stores --- */

static long long conditional_dead_stores(int *buf, const int *arr, int n) {
    int i;
    long long total = 0;
    for (i = 0; i < n; i++) {
        /* Both branches write to buf[i], so first store is always dead */
        buf[i] = arr[i] * 5;  /* dead */
        if (arr[i] > 500) {
            buf[i] = arr[i] + 100;
        } else {
            buf[i] = arr[i] - 100;
        }
    }
    for (i = 0; i < n; i++) total += buf[i];
    return total;
}

/* --- Variant 2: Struct-like dead stores via array of pairs --- */

static long long struct_dead_stores(int *buf, const int *arr, int n) {
    int i;
    long long total = 0;
    /* buf used as pairs: buf[2*i] = x, buf[2*i+1] = y */
    int half = n / 2;
    for (i = 0; i < half; i++) {
        buf[2 * i] = arr[i] + 1;      /* dead x */
        buf[2 * i + 1] = arr[i] + 2;  /* dead y */
        /* overwrite both */
        buf[2 * i] = arr[i] * arr[i];
        buf[2 * i + 1] = arr[i] * 3;
    }
    for (i = 0; i < half; i++) {
        total += buf[2 * i] + buf[2 * i + 1];
    }
    return total;
}

/* --- Variant 3: Loop with partially-live stores --- */

static long long partial_live_stores(int *buf, const int *arr, int n) {
    int i;
    long long total = 0;
    /* Write all elements */
    for (i = 0; i < n; i++) {
        buf[i] = arr[i] * 7 + 3;
    }
    /* Overwrite only even indices — odd stores from above are live */
    for (i = 0; i < n; i += 2) {
        buf[i] = arr[i] * arr[i];
    }
    for (i = 0; i < n; i++) {
        total += buf[i];
    }
    return total;
}

static long long workload(int *arr, int *scratch) {
    long long total = 0;

    update_with_dead_stores(arr, scratch, N);
    {
        int i;
        for (i = 0; i < N; i++) total += scratch[i];
    }

    total += compute_with_dead_code(arr, N);
    total += store_load_pattern(arr, N);
    total += multi_phase(scratch, arr, N);
    total += conditional_dead_stores(scratch, arr, N);
    total += struct_dead_stores(scratch, arr, N);
    total += partial_live_stores(scratch, arr, N);

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    int *arr = (int *)malloc(N * sizeof(int));
    int *scratch = (int *)malloc(N * sizeof(int));
    int i;

    bench_lcg_seed(12345);
    for (i = 0; i < N; i++) {
        arr[i] = (int)(bench_lcg_rand() % 1000) + 1;
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(arr, scratch); });

    free(arr);
    free(scratch);
    return 0;
}
