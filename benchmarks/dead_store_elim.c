/*
 * Targets: dse, adce, instcombine, mem2reg, simplifycfg
 *
 * Deliberately includes dead stores, dead computations, and redundant
 * writes to exercise DSE and ADCE. The workload does real work too
 * so timing differences are measurable.
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

#define N 50000

/*
 * Simulate a "sloppy" update loop: each element gets written twice,
 * first with a provisional value then with the real one. DSE should
 * eliminate the first store.
 */
static void update_with_dead_stores(int *arr, int *scratch, int n) {
    int i;
    for (i = 0; i < n; i++) {
        /* Dead store — overwritten below */
        scratch[i] = arr[i] * 3 + 7;
        /* Real computation */
        scratch[i] = arr[i] * arr[i] + arr[i] + 1;
    }
}

/*
 * Compute values that are never used — ADCE should eliminate these.
 * The function returns only `used`, but computes `unused` too.
 */
static long long compute_with_dead_code(const int *arr, int n) {
    long long used = 0;
    long long unused = 0;  /* ADCE target */
    int i;
    for (i = 0; i < n; i++) {
        used += (long long)arr[i] * arr[i];
        /* Dead computation — result is never read */
        unused += (long long)arr[i] * 17 + arr[i] / 3;
    }
    return used;
}

/*
 * Store-then-load pattern — mem2reg + DSE can simplify.
 * Writes to a local buffer, immediately reads back.
 */
static long long store_load_pattern(const int *arr, int n) {
    int tmp[4];
    long long total = 0;
    int i;
    for (i = 0; i + 3 < n; i += 4) {
        /* Store into local buffer */
        tmp[0] = arr[i] + 1;
        tmp[1] = arr[i + 1] + 2;
        tmp[2] = arr[i + 2] + 3;
        tmp[3] = arr[i + 3] + 4;
        /* Immediately load back */
        total += tmp[0] + tmp[1] + tmp[2] + tmp[3];
    }
    return total;
}

/*
 * Multi-phase computation where intermediate arrays are overwritten.
 * First pass writes, second pass overwrites completely — DSE on first.
 */
static long long multi_phase(int *buf, const int *arr, int n) {
    int i;
    long long total = 0;

    /* Phase 1: provisional values (dead — overwritten in phase 2) */
    for (i = 0; i < n; i++) {
        buf[i] = arr[i] + 42;
    }
    /* Phase 2: real values overwrite phase 1 */
    for (i = 0; i < n; i++) {
        buf[i] = arr[i] * arr[i];
    }
    /* Only phase 2 results are used */
    for (i = 0; i < n; i++) {
        total += buf[i];
    }
    return total;
}

static long long workload(int *arr, int *scratch) {
    long long total = 0;

    update_with_dead_stores(arr, scratch, N);
    /* Sum scratch to make the non-dead stores live */
    {
        int i;
        for (i = 0; i < N; i++) total += scratch[i];
    }

    total += compute_with_dead_code(arr, N);
    total += store_load_pattern(arr, N);
    total += multi_phase(scratch, arr, N);

    return total;
}

int main(void) {
    int *arr = (int *)malloc(N * sizeof(int));
    int *scratch = (int *)malloc(N * sizeof(int));
    int i;

    lcg_state = 12345;
    for (i = 0; i < N; i++) {
        arr[i] = (int)(lcg_rand() % 1000) + 1;
    }

    /* Warmup */
    volatile long long sink;
    for (i = 0; i < 5; i++) {
        sink = workload(arr, scratch);
    }

    /* Timing */
    long long times[50];
    struct timespec t0, t1;
    for (i = 0; i < 50; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(arr, scratch);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);

    free(arr);
    free(scratch);
    return 0;
}
