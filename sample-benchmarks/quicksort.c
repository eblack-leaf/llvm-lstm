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

/* --- Variant 1: Dutch national flag 3-way partition --- */

static void dnf_quicksort(int *arr, int lo, int hi) {
    if (lo >= hi) return;
    int pivot = arr[lo + (hi - lo) / 2];
    int lt = lo, gt = hi, i = lo;
    while (i <= gt) {
        if (arr[i] < pivot) {
            swap(&arr[i], &arr[lt]);
            lt++; i++;
        } else if (arr[i] > pivot) {
            swap(&arr[i], &arr[gt]);
            gt--;
        } else {
            i++;
        }
    }
    dnf_quicksort(arr, lo, lt - 1);
    dnf_quicksort(arr, gt + 1, hi);
}

/* --- Variant 2: Hybrid with insertion sort cutoff --- */

static void insertion_sort(int *arr, int lo, int hi) {
    int i, j;
    for (i = lo + 1; i <= hi; i++) {
        int key = arr[i];
        j = i - 1;
        while (j >= lo && arr[j] > key) {
            arr[j + 1] = arr[j];
            j--;
        }
        arr[j + 1] = key;
    }
}

static void quicksort_hybrid(int *arr, int lo, int hi) {
    if (hi - lo < 16) {
        if (hi > lo) insertion_sort(arr, lo, hi);
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
    swap(&arr[i], &arr[hi - 1]);
    quicksort_hybrid(arr, lo, i - 1);
    quicksort_hybrid(arr, i + 1, hi);
}

/* --- Variant 3: Iterative with explicit stack --- */

static void quicksort_iterative(int *arr, int lo_init, int hi_init) {
    int stack[128];
    int sp = 0;
    stack[sp++] = lo_init;
    stack[sp++] = hi_init;

    while (sp > 0) {
        int hi = stack[--sp];
        int lo = stack[--sp];

        if (hi - lo < 2) {
            if (hi > lo && arr[lo] > arr[hi]) swap(&arr[lo], &arr[hi]);
            continue;
        }

        /* Lomuto partition */
        int pivot_val = arr[hi];
        int store = lo;
        int k;
        for (k = lo; k < hi; k++) {
            if (arr[k] <= pivot_val) {
                swap(&arr[store], &arr[k]);
                store++;
            }
        }
        swap(&arr[store], &arr[hi]);

        /* Push larger partition first */
        int left_size  = store - 1 - lo;
        int right_size = hi - (store + 1);
        if (left_size > right_size) {
            if (lo < store - 1 && sp + 2 <= 128) { stack[sp++] = lo; stack[sp++] = store - 1; }
            if (store + 1 < hi && sp + 2 <= 128) { stack[sp++] = store + 1; stack[sp++] = hi; }
        } else {
            if (store + 1 < hi && sp + 2 <= 128) { stack[sp++] = store + 1; stack[sp++] = hi; }
            if (lo < store - 1 && sp + 2 <= 128) { stack[sp++] = lo; stack[sp++] = store - 1; }
        }
    }
}

static long long workload(int *arr, int *src) {
    long long sum = 0;
    int i;

    /* Original quicksort */
    memcpy(arr, src, ARR_N * sizeof(int));
    quicksort(arr, 0, ARR_N - 1);
    for (i = 0; i < ARR_N; i++) sum += arr[i];

    /* DNF 3-way */
    memcpy(arr, src, ARR_N * sizeof(int));
    dnf_quicksort(arr, 0, ARR_N - 1);
    for (i = 0; i < ARR_N; i++) sum += arr[i];

    /* Hybrid */
    memcpy(arr, src, ARR_N * sizeof(int));
    quicksort_hybrid(arr, 0, ARR_N - 1);
    for (i = 0; i < ARR_N; i++) sum += arr[i];

    /* Iterative */
    memcpy(arr, src, ARR_N * sizeof(int));
    quicksort_iterative(arr, 0, ARR_N - 1);
    for (i = 0; i < ARR_N; i++) sum += arr[i];

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
