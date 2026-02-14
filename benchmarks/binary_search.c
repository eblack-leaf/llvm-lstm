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
static unsigned int lcg_rand(void) { lcg_state = lcg_state * 1103515245 + 12345; return (lcg_state >> 16) & 0x7fff; }

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

int main(void) {
    /* Generate deterministic data */
    lcg_state = 12345;
    for (int i = 0; i < N; i++)
        arr[i] = (int)(lcg_rand() << 16) | (int)lcg_rand();

    qsort(arr, N, sizeof(int), cmp_int);

    /* Generate queries: mix of values in array and random values */
    lcg_state = 67890;
    for (int i = 0; i < QUERIES; i++) {
        if (lcg_rand() % 2 == 0)
            queries[i] = arr[lcg_rand() % N];  /* will be found */
        else
            queries[i] = (int)(lcg_rand() << 16) | (int)lcg_rand();  /* maybe found */
    }

    /* Warmup */
    for (int w = 0; w < 5; w++)
        do_benchmark();

    /* Timed runs */
    long long times[201];
    for (int t = 0; t < 201; t++) {
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        do_benchmark();
        clock_gettime(CLOCK_MONOTONIC, &end);
        times[t] = timespec_diff_ns(&start, &end);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    /* Drop bottom/top 10% (20 each), average middle 161 */
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);
    return 0;
}
