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

#define ARR_N 5000

static void swap(int *a, int *b) {
    int tmp = *a;
    *a = *b;
    *b = tmp;
}

static int median_of_three(int *arr, int lo, int hi) {
    int mid = lo + (hi - lo) / 2;
    if (arr[lo] > arr[mid]) swap(&arr[lo], &arr[mid]);
    if (arr[lo] > arr[hi])  swap(&arr[lo], &arr[hi]);
    if (arr[mid] > arr[hi]) swap(&arr[mid], &arr[hi]);
    /* Place pivot at hi-1 */
    swap(&arr[mid], &arr[hi - 1]);
    return arr[hi - 1];
}

static void quicksort(int *arr, int lo, int hi) {
    if (hi - lo < 2) {
        if (hi > lo && arr[lo] > arr[hi]) swap(&arr[lo], &arr[hi]);
        return;
    }
    int pivot = median_of_three(arr, lo, hi);
    int i = lo, j = hi - 1;
    for (;;) {
        while (arr[++i] < pivot) {}
        while (arr[--j] > pivot) {}
        if (i >= j) break;
        swap(&arr[i], &arr[j]);
    }
    swap(&arr[i], &arr[hi - 1]); /* Restore pivot */
    quicksort(arr, lo, i - 1);
    quicksort(arr, i + 1, hi);
}

static long long workload(int *arr, int *src) {
    memcpy(arr, src, ARR_N * sizeof(int));
    quicksort(arr, 0, ARR_N - 1);
    /* Sum to prevent optimization */
    long long sum = 0;
    int i;
    for (i = 0; i < ARR_N; i++) {
        sum += arr[i];
    }
    return sum;
}

int main(void) {
    int *src = (int *)malloc(ARR_N * sizeof(int));
    int *arr = (int *)malloc(ARR_N * sizeof(int));
    int i;

    lcg_state = 12345;
    for (i = 0; i < ARR_N; i++) {
        src[i] = (int)(lcg_rand() << 16) | (int)lcg_rand();
    }

    /* Warmup */
    volatile long long sink;
    for (i = 0; i < 5; i++) {
        sink = workload(arr, src);
    }

    /* Timing */
    long long times[201];
    struct timespec t0, t1;
    for (i = 0; i < 201; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(arr, src);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    /* Drop bottom/top 10% (20 each), average middle 161 */
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);

    free(src);
    free(arr);
    return 0;
}
