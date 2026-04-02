#include "bench_timing.h"

#define NUM_OPS 2000

/* ---- Min-heap (1-indexed) ---- */
static int heap[NUM_OPS + 1];
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

/* ---- Max-heap (1-indexed) ---- */
static int max_heap[NUM_OPS + 1];
static int max_heap_size;

static void max_heap_init(void) {
    max_heap_size = 0;
}

static void max_heap_push(int val) {
    max_heap_size++;
    int i = max_heap_size;
    max_heap[i] = val;
    while (i > 1 && max_heap[i] > max_heap[i / 2]) {
        int tmp = max_heap[i];
        max_heap[i] = max_heap[i / 2];
        max_heap[i / 2] = tmp;
        i /= 2;
    }
}

static int max_heap_pop(void) {
    int max_val = max_heap[1];
    max_heap[1] = max_heap[max_heap_size];
    max_heap_size--;
    /* Sift down */
    int i = 1;
    while (1) {
        int largest = i;
        int left = 2 * i;
        int right = 2 * i + 1;
        if (left <= max_heap_size && max_heap[left] > max_heap[largest])
            largest = left;
        if (right <= max_heap_size && max_heap[right] > max_heap[largest])
            largest = right;
        if (largest == i) break;
        int tmp = max_heap[i];
        max_heap[i] = max_heap[largest];
        max_heap[largest] = tmp;
        i = largest;
    }
    return max_val;
}

/* ---- Composite operations ---- */

static int heap_pushpop(int val) {
    if (heap_size == 0 || val <= heap[1]) return val;
    int old_min = heap[1];
    heap[1] = val;
    /* Sift down */
    int i = 1;
    while (1) {
        int smallest = i, left = 2 * i, right = 2 * i + 1;
        if (left <= heap_size && heap[left] < heap[smallest]) smallest = left;
        if (right <= heap_size && heap[right] < heap[smallest]) smallest = right;
        if (smallest == i) break;
        int tmp = heap[i];
        heap[i] = heap[smallest];
        heap[smallest] = tmp;
        i = smallest;
    }
    return old_min;
}

static void heapsort(int *out, int n) {
    for (int i = 0; i < n; i++) {
        out[i] = heap_pop();
    }
}

static int heap_median_estimate(int n) {
    max_heap_init();
    heap_init();
    bench_lcg_seed(99999);
    int median = 0;
    for (int i = 0; i < n; i++) {
        int val = (int)((bench_lcg_rand() << 15) | bench_lcg_rand());
        if (max_heap_size == 0 || val <= max_heap[1]) {
            max_heap_push(val);
        } else {
            heap_push(val);
        }
        /* Rebalance: max_heap should have equal or one more element */
        if (max_heap_size > heap_size + 1) {
            heap_push(max_heap_pop());
        } else if (heap_size > max_heap_size) {
            max_heap_push(heap_pop());
        }
        median = max_heap[1];
    }
    return median;
}

/* ---- Benchmark buffers ---- */
static int sorted_buf[NUM_OPS];
static int vals_buf[NUM_OPS];

static volatile long long sink;

static void run_benchmark(void) {
    bench_lcg_seed(12345);
    long long sum = 0;

    /* Generate random values once */
    for (int i = 0; i < NUM_OPS; i++) {
        vals_buf[i] = (int)((bench_lcg_rand() << 15) | bench_lcg_rand());
    }

    /* Phase 1: min-heap push + pop (original) */
    heap_init();
    for (int i = 0; i < NUM_OPS; i++) {
        heap_push(vals_buf[i]);
    }
    for (int i = 0; i < NUM_OPS; i++) {
        sum += heap_pop();
    }

    /* Phase 2: max-heap push + pop */
    max_heap_init();
    for (int i = 0; i < NUM_OPS; i++) {
        max_heap_push(vals_buf[i]);
    }
    for (int i = 0; i < NUM_OPS; i++) {
        sum += max_heap_pop();
    }

    /* Phase 3: min-heap pushpop with new random values */
    heap_init();
    for (int i = 0; i < NUM_OPS; i++) {
        heap_push(vals_buf[i]);
    }
    bench_lcg_seed(67890);
    for (int i = 0; i < NUM_OPS / 2; i++) {
        int val = (int)((bench_lcg_rand() << 15) | bench_lcg_rand());
        sum += heap_pushpop(val);
    }
    /* Drain remaining */
    while (heap_size > 0) {
        sum += heap_pop();
    }

    /* Phase 4: heapsort */
    heap_init();
    for (int i = 0; i < NUM_OPS; i++) {
        heap_push(vals_buf[i]);
    }
    heapsort(sorted_buf, NUM_OPS);
    for (int i = 0; i < NUM_OPS; i++) {
        sum += sorted_buf[i];
    }

    /* Phase 5: median estimate */
    sum += heap_median_estimate(NUM_OPS);

    sink = sum;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    BENCH_TIME(niters, { run_benchmark(); });
    return 0;
}
