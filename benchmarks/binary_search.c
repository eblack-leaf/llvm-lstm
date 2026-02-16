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

static volatile int found_count;

static void do_benchmark(void) {
    int count = 0;
    for (int i = 0; i < QUERIES; i++) {
        if (binary_search(arr, N, queries[i]) >= 0)
            count++;
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
