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

#define NUM_OPS 2000

static int heap[NUM_OPS + 1];  /* 1-indexed */
static int heap_size;

static void heap_init(void) {
    heap_size = 0;
}

static void heap_push(int val) {
    heap_size++;
    int i = heap_size;
    heap[i] = val;
    /* Sift up */
    while (i > 1 && heap[i] < heap[i / 2]) {
        int tmp = heap[i];
        heap[i] = heap[i / 2];
        heap[i / 2] = tmp;
        i /= 2;
    }
}

static int heap_pop(void) {
    int min_val = heap[1];
    heap[1] = heap[heap_size];
    heap_size--;
    /* Sift down */
    int i = 1;
    while (1) {
        int smallest = i;
        int left = 2 * i;
        int right = 2 * i + 1;
        if (left <= heap_size && heap[left] < heap[smallest])
            smallest = left;
        if (right <= heap_size && heap[right] < heap[smallest])
            smallest = right;
        if (smallest == i) break;
        int tmp = heap[i];
        heap[i] = heap[smallest];
        heap[smallest] = tmp;
        i = smallest;
    }
    return min_val;
}

static volatile long long sink;

static void run_benchmark(void) {
    lcg_state = 12345;
    heap_init();

    /* Insert NUM_OPS random integers */
    for (int i = 0; i < NUM_OPS; i++) {
        int val = (int)((lcg_rand() << 15) | lcg_rand());
        heap_push(val);
    }

    /* Extract-min NUM_OPS times */
    long long sum = 0;
    for (int i = 0; i < NUM_OPS; i++) {
        sum += heap_pop();
    }
    sink = sum;
}

int main(void) {
    for (int i = 0; i < 5; i++) run_benchmark();

    long long times[201];
    for (int i = 0; i < 201; i++) {
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        run_benchmark();
        clock_gettime(CLOCK_MONOTONIC, &end);
        times[i] = timespec_diff_ns(&start, &end);
    }

    qsort(times, 201, sizeof(long long), cmp_ll);
    /* Drop bottom/top 10% (20 each), average middle 161 */
    long long tsum = 0;
    for (int ti = 20; ti < 181; ti++) tsum += times[ti];
    printf("%lld\n", tsum / 161);
    return 0;
}
