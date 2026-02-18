/*
 * Targets: tailcallelim, inline, instcombine, simplifycfg
 */
#include "bench_timing.h"

static long long tail_sum(const int *arr, int n, long long acc) {
    if (n <= 0) return acc;
    return tail_sum(arr + 1, n - 1, acc + arr[0]);
}

static int tail_search(const int *arr, int n, int target, int idx) {
    if (n <= 0) return -1;
    if (arr[0] == target) return idx;
    return tail_search(arr + 1, n - 1, target, idx + 1);
}

static long long tail_gcd(long long a, long long b) {
    if (b == 0) return a;
    return tail_gcd(b, a % b);
}

static int tail_partition(const int *arr, int n, int pivot, int count) {
    if (n <= 0) return count;
    int next = (arr[0] > pivot) ? count + 1 : count;
    return tail_partition(arr + 1, n - 1, pivot, next);
}

static int tail_max(const int *arr, int n, int best) {
    if (n <= 0) return best;
    int next = (arr[0] > best) ? arr[0] : best;
    return tail_max(arr + 1, n - 1, next);
}

static int tail_runs(const int *arr, int n, int prev, int count) {
    if (n <= 0) return count;
    int next = (arr[0] == prev) ? count + 1 : count;
    return tail_runs(arr + 1, n - 1, arr[0], next);
}

/* --- Variant 1: Mutual recursion (even/odd parity) --- */

static int mutual_odd(unsigned int n);

static int mutual_even(unsigned int n) {
    if (n == 0) return 1;
    return mutual_odd(n - 1);
}

static int mutual_odd(unsigned int n) {
    if (n == 0) return 0;
    return mutual_even(n - 1);
}

static long long tail_parity_classify(const int *arr, int n) {
    long long total = 0;
    int i;
    for (i = 0; i < n; i++) {
        unsigned int v = (unsigned int)(arr[i] > 0 ? arr[i] : -arr[i]);
        if (v > 500) v = 500;  /* cap recursion depth */
        total += mutual_even(v) ? arr[i] : -arr[i];
    }
    return total;
}

/* --- Variant 2: Tail-recursive power (exp by squaring) --- */

static long long tail_power(long long base, int exp, long long acc) {
    if (exp <= 0) return acc;
    if (exp % 2 == 1)
        return tail_power(base, exp - 1, acc * base);
    else
        return tail_power(base * base, exp / 2, acc);
}

static long long tail_power_sum(const int *arr, int n) {
    long long total = 0;
    int i;
    for (i = 0; i < n; i++) {
        long long base = (arr[i] % 7) + 2;
        int exp = (arr[i] % 15) + 1;
        total += tail_power(base, exp, 1);
    }
    return total;
}

/* --- Variant 3: Tail-recursive binary search --- */

static int tail_bsearch(const int *sorted, int lo, int hi, int target) {
    if (lo > hi) return -1;
    int mid = lo + (hi - lo) / 2;
    if (sorted[mid] == target) return mid;
    if (target < sorted[mid])
        return tail_bsearch(sorted, lo, mid - 1, target);
    else
        return tail_bsearch(sorted, mid + 1, hi, target);
}

static void simple_sort(int *arr, int n) {
    int i, j;
    for (i = 1; i < n; i++) {
        int key = arr[i];
        j = i - 1;
        while (j >= 0 && arr[j] > key) {
            arr[j + 1] = arr[j];
            j--;
        }
        arr[j + 1] = key;
    }
}

#define ARR_N 5000
#define SORT_N 512

static long long workload(int *arr) {
    long long total = 0;
    int i;

    total += tail_sum(arr, ARR_N, 0);

    for (i = 0; i < 100; i++) {
        int target = arr[bench_lcg_rand() % ARR_N];
        total += tail_search(arr, ARR_N, target, 0);
    }

    for (i = 0; i < ARR_N - 1; i += 2) {
        long long a = arr[i] > 0 ? arr[i] : -arr[i];
        long long b = arr[i + 1] > 0 ? arr[i + 1] : -arr[i + 1];
        if (a > 0 && b > 0) total += tail_gcd(a, b);
    }

    for (i = 0; i < 10; i++) {
        int pivot = (int)(bench_lcg_rand() % 10000);
        total += tail_partition(arr, ARR_N, pivot, 0);
    }

    for (i = 0; i < ARR_N; i += ARR_N / 20) {
        total += tail_max(arr + i, ARR_N / 20, 0);
    }

    total += tail_runs(arr, ARR_N, -1, 0);

    /* Mutual recursion parity classification */
    total += tail_parity_classify(arr, ARR_N);

    /* Power sums */
    total += tail_power_sum(arr, ARR_N);

    /* Sort a sub-buffer and do binary searches */
    {
        int sorted[SORT_N];
        memcpy(sorted, arr, SORT_N * sizeof(int));
        simple_sort(sorted, SORT_N);
        for (i = 0; i < 200; i++) {
            int target = arr[bench_lcg_rand() % ARR_N];
            total += tail_bsearch(sorted, 0, SORT_N - 1, target);
        }
    }

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
