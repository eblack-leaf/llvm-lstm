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

/* --- Variant 1: Bottom-up iterative mergesort --- */

static void mergesort_bottomup(int *arr, int *tmp, int n) {
    int width, i;
    for (width = 1; width < n; width *= 2) {
        for (i = 0; i < n; i += 2 * width) {
            int mid = i + width;
            int right = i + 2 * width;
            if (mid > n) mid = n;
            if (right > n) right = n;
            if (mid < right) {
                merge(arr, tmp, i, mid, right);
            }
        }
    }
}

/* --- Variant 2: Hybrid with insertion sort cutoff --- */

static void insertion_sort_range(int *arr, int lo, int hi) {
    int i, j;
    for (i = lo + 1; i < hi; i++) {
        int key = arr[i];
        j = i - 1;
        while (j >= lo && arr[j] > key) {
            arr[j + 1] = arr[j];
            j--;
        }
        arr[j + 1] = key;
    }
}

static void mergesort_hybrid(int *arr, int *tmp, int left, int right) {
    if (right - left <= 32) {
        insertion_sort_range(arr, left, right);
        return;
    }
    int mid = left + (right - left) / 2;
    mergesort_hybrid(arr, tmp, left, mid);
    mergesort_hybrid(arr, tmp, mid, right);
    /* Skip merge if already sorted at boundary */
    if (arr[mid - 1] <= arr[mid]) return;
    merge(arr, tmp, left, mid, right);
}

/* --- Variant 3: Merge-based inversion count --- */

static long long merge_count(int *arr, int *tmp, int left, int mid, int right) {
    long long inv = 0;
    int i = left, j = mid, k = left;
    while (i < mid && j < right) {
        if (arr[i] <= arr[j]) {
            tmp[k++] = arr[i++];
        } else {
            inv += (long long)(mid - i);
            tmp[k++] = arr[j++];
        }
    }
    while (i < mid) tmp[k++] = arr[i++];
    while (j < right) tmp[k++] = arr[j++];
    memcpy(arr + left, tmp + left, (right - left) * sizeof(int));
    return inv;
}

static long long mergesort_inversion_count(int *arr, int *tmp, int left, int right) {
    if (right - left <= 1) return 0;
    int mid = left + (right - left) / 2;
    long long inv = 0;
    inv += mergesort_inversion_count(arr, tmp, left, mid);
    inv += mergesort_inversion_count(arr, tmp, mid, right);
    inv += merge_count(arr, tmp, left, mid, right);
    return inv;
}

static long long do_mergesort(void) {
    long long total = 0;
    int i;

    /* Original recursive mergesort */
    memcpy(work, data, N * sizeof(int));
    mergesort_rec(work, aux, 0, N);
    for (i = 0; i < N; i++) total += work[i];

    /* Bottom-up iterative */
    memcpy(work, data, N * sizeof(int));
    mergesort_bottomup(work, aux, N);
    for (i = 0; i < N; i++) total += work[i];

    /* Hybrid with insertion sort cutoff */
    memcpy(work, data, N * sizeof(int));
    mergesort_hybrid(work, aux, 0, N);
    for (i = 0; i < N; i++) total += work[i];

    /* Inversion count */
    memcpy(work, data, N * sizeof(int));
    total += mergesort_inversion_count(work, aux, 0, N);

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    /* Generate deterministic input */
    bench_lcg_seed(12345);
    for (int i = 0; i < N; i++)
        data[i] = (int)(bench_lcg_rand() << 16) | (int)bench_lcg_rand();

    volatile long long sink;
    BENCH_TIME(niters, { sink = do_mergesort(); });
    return 0;
}
