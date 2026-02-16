#include "bench_timing.h"

#define N 5000

static int data[N];
static int work[N];
static int aux[N];

static void merge(int *arr, int *tmp, int left, int mid, int right) {
    int i = left, j = mid, k = left;
    while (i < mid && j < right) {
        if (arr[i] <= arr[j])
            tmp[k++] = arr[i++];
        else
            tmp[k++] = arr[j++];
    }
    while (i < mid) tmp[k++] = arr[i++];
    while (j < right) tmp[k++] = arr[j++];
    memcpy(arr + left, tmp + left, (right - left) * sizeof(int));
}

static void mergesort_rec(int *arr, int *tmp, int left, int right) {
    if (right - left <= 1) return;
    int mid = left + (right - left) / 2;
    mergesort_rec(arr, tmp, left, mid);
    mergesort_rec(arr, tmp, mid, right);
    merge(arr, tmp, left, mid, right);
}

static void do_mergesort(void) {
    memcpy(work, data, N * sizeof(int));
    mergesort_rec(work, aux, 0, N);
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    /* Generate deterministic input */
    bench_lcg_seed(12345);
    for (int i = 0; i < N; i++)
        data[i] = (int)(bench_lcg_rand() << 16) | (int)bench_lcg_rand();

    volatile int sink;
    BENCH_TIME(niters, { do_mergesort(); sink = work[0]; });
    return 0;
}
