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

#define N 500000

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

int main(void) {
    /* Generate deterministic input */
    lcg_state = 12345;
    for (int i = 0; i < N; i++)
        data[i] = (int)(lcg_rand() << 16) | (int)lcg_rand();

    /* Warmup */
    for (int w = 0; w < 5; w++)
        do_mergesort();

    /* Timed runs */
    long long times[50];
    for (int t = 0; t < 50; t++) {
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        do_mergesort();
        clock_gettime(CLOCK_MONOTONIC, &end);
        times[t] = timespec_diff_ns(&start, &end);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);
    return 0;
}
