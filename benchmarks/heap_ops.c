#include "bench_timing.h"

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
    bench_lcg_seed(12345);
    heap_init();

    /* Insert NUM_OPS random integers */
    for (int i = 0; i < NUM_OPS; i++) {
        int val = (int)((bench_lcg_rand() << 15) | bench_lcg_rand());
        heap_push(val);
    }

    /* Extract-min NUM_OPS times */
    long long sum = 0;
    for (int i = 0; i < NUM_OPS; i++) {
        sum += heap_pop();
    }
    sink = sum;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    BENCH_TIME(niters, { run_benchmark(); });
    return 0;
}
