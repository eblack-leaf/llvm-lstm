/*
 * Targets: tailcallelim, inline, instcombine, simplifycfg
 *
 * Multiple tail-recursive patterns: summation, search, GCD, tree walk,
 * and a recursive filter. All convertible to loops by tailcallelim.
 */
#include "bench_timing.h"

/* Tail-recursive sum of array elements with accumulator */
static long long tail_sum(const int *arr, int n, long long acc) {
    if (n <= 0) return acc;
    return tail_sum(arr + 1, n - 1, acc + arr[0]);
}

/* Tail-recursive linear search — returns index or -1 */
static int tail_search(const int *arr, int n, int target, int idx) {
    if (n <= 0) return -1;
    if (arr[0] == target) return idx;
    return tail_search(arr + 1, n - 1, target, idx + 1);
}

/* Tail-recursive GCD */
static long long tail_gcd(long long a, long long b) {
    if (b == 0) return a;
    return tail_gcd(b, a % b);
}

/* Tail-recursive partition — counts elements above/below pivot */
static int tail_partition(const int *arr, int n, int pivot, int count) {
    if (n <= 0) return count;
    int next = (arr[0] > pivot) ? count + 1 : count;
    return tail_partition(arr + 1, n - 1, pivot, next);
}

/* Tail-recursive max — finds maximum element */
static int tail_max(const int *arr, int n, int best) {
    if (n <= 0) return best;
    int next = (arr[0] > best) ? arr[0] : best;
    return tail_max(arr + 1, n - 1, next);
}

/* Tail-recursive run-length counter — counts consecutive equal pairs */
static int tail_runs(const int *arr, int n, int prev, int count) {
    if (n <= 0) return count;
    int next = (arr[0] == prev) ? count + 1 : count;
    return tail_runs(arr + 1, n - 1, arr[0], next);
}

#define ARR_N 5000

static long long workload(int *arr) {
    long long total = 0;
    int i;

    /* Tail-recursive sum — tailcallelim converts this to a loop */
    total += tail_sum(arr, ARR_N, 0);

    /* Tail-recursive search for 100 targets */
    for (i = 0; i < 100; i++) {
        int target = arr[bench_lcg_rand() % ARR_N];
        total += tail_search(arr, ARR_N, target, 0);
    }

    /* Tail-recursive GCD on pairs */
    for (i = 0; i < ARR_N - 1; i += 2) {
        long long a = arr[i] > 0 ? arr[i] : -arr[i];
        long long b = arr[i + 1] > 0 ? arr[i + 1] : -arr[i + 1];
        if (a > 0 && b > 0) total += tail_gcd(a, b);
    }

    /* Tail-recursive partition with multiple pivots */
    for (i = 0; i < 10; i++) {
        int pivot = (int)(bench_lcg_rand() % 10000);
        total += tail_partition(arr, ARR_N, pivot, 0);
    }

    /* Tail-recursive max over subarrays */
    for (i = 0; i < ARR_N; i += ARR_N / 20) {
        total += tail_max(arr + i, ARR_N / 20, 0);
    }

    /* Tail-recursive run-length counting */
    total += tail_runs(arr, ARR_N, -1, 0);

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    int *arr = (int *)malloc(ARR_N * sizeof(int));
    int i;

    bench_lcg_seed(12345);
    for (i = 0; i < ARR_N; i++) {
        arr[i] = (int)(bench_lcg_rand() % 10000) + 1;
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(arr); });

    free(arr);
    return 0;
}
