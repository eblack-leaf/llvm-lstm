#include "bench_timing.h"

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

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    int *src = (int *)malloc(ARR_N * sizeof(int));
    int *arr = (int *)malloc(ARR_N * sizeof(int));
    int i;

    bench_lcg_seed(12345);
    for (i = 0; i < ARR_N; i++) {
        src[i] = (int)(bench_lcg_rand() << 16) | (int)bench_lcg_rand();
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(arr, src); });

    free(src);
    free(arr);
    return 0;
}
