#include "bench_timing.h"

#define N 1000
#define QUERIES 5000

static int arr[N];
static int queries[QUERIES];

static int cmp_int(const void *a, const void *b) {
    int x = *(const int *)a, y = *(const int *)b;
    return (x > y) - (x < y);
}

static int binary_search(const int *a, int n, int key) {
    int lo = 0, hi = n - 1;
    while (lo <= hi) {
        int mid = lo + (hi - lo) / 2;
        if (a[mid] == key) return mid;
        else if (a[mid] < key) lo = mid + 1;
        else hi = mid - 1;
    }
    return -1;
}

/* --- Variant 1: Interpolation search --- */

static int interpolation_search(const int *a, int n, int key) {
    int lo = 0, hi = n - 1;
    while (lo <= hi && key >= a[lo] && key <= a[hi]) {
        if (a[hi] == a[lo]) {
            if (a[lo] == key) return lo;
            break;
        }
        long long num = (long long)(key - a[lo]) * (hi - lo);
        int pos = lo + (int)(num / (a[hi] - a[lo]));
        if (pos < lo) pos = lo;
        if (pos > hi) pos = hi;
        if (a[pos] == key) return pos;
        else if (a[pos] < key) lo = pos + 1;
        else hi = pos - 1;
    }
    return -1;
}

/* --- Variant 2: Exponential search --- */

static int exponential_search(const int *a, int n, int key) {
    if (n == 0) return -1;
    if (a[0] == key) return 0;
    int bound = 1;
    while (bound < n && a[bound] <= key) {
        bound *= 2;
    }
    /* Binary search in [bound/2, min(bound, n-1)] */
    int lo = bound / 2;
    int hi = bound < n ? bound : n - 1;
    while (lo <= hi) {
        int mid = lo + (hi - lo) / 2;
        if (a[mid] == key) return mid;
        else if (a[mid] < key) lo = mid + 1;
        else hi = mid - 1;
    }
    return -1;
}

/* --- Variant 3: Ternary search --- */

static int ternary_search(const int *a, int n, int key) {
    int lo = 0, hi = n - 1;
    while (lo <= hi) {
        int third = (hi - lo) / 3;
        int m1 = lo + third;
        int m2 = hi - third;
        if (a[m1] == key) return m1;
        if (a[m2] == key) return m2;
        if (key < a[m1]) hi = m1 - 1;
        else if (key > a[m2]) lo = m2 + 1;
        else { lo = m1 + 1; hi = m2 - 1; }
    }
    return -1;
}

static volatile int found_count;

static void do_benchmark(void) {
    int count = 0;
    int i;
    for (i = 0; i < QUERIES; i++) {
        int key = queries[i];
        if (binary_search(arr, N, key) >= 0) count++;
        if (interpolation_search(arr, N, key) >= 0) count++;
        if (exponential_search(arr, N, key) >= 0) count++;
        if (ternary_search(arr, N, key) >= 0) count++;
    }
    found_count = count;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    /* Generate deterministic data */
    bench_lcg_seed(12345);
    for (int i = 0; i < N; i++)
        arr[i] = (int)(bench_lcg_rand() << 16) | (int)bench_lcg_rand();

    qsort(arr, N, sizeof(int), cmp_int);

    /* Generate queries: mix of values in array and random values */
    bench_lcg_seed(67890);
    for (int i = 0; i < QUERIES; i++) {
        if (bench_lcg_rand() % 2 == 0)
            queries[i] = arr[bench_lcg_rand() % N];  /* will be found */
        else
            queries[i] = (int)(bench_lcg_rand() << 16) | (int)bench_lcg_rand();  /* maybe found */
    }

    volatile int sink;
    BENCH_TIME(niters, { do_benchmark(); sink = found_count; });

    return 0;
}
