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

#define NUM_NODES 500
#define NUM_EDGES 2000

/* CSR (Compressed Sparse Row) representation */
static int adj[NUM_EDGES];       /* destination nodes */
static int adj_offset[NUM_NODES + 1]; /* offsets into adj[] */
static int degree[NUM_NODES];

static int visited[NUM_NODES];
static int queue[NUM_NODES];

static void build_graph(void) {
    memset(degree, 0, sizeof(degree));

    /* First pass: count degrees */
    /* We store edges temporarily, then build CSR */
    int *src = (int *)malloc(NUM_EDGES * sizeof(int));
    int *dst = (int *)malloc(NUM_EDGES * sizeof(int));

    for (int i = 0; i < NUM_EDGES; i++) {
        src[i] = lcg_rand() % NUM_NODES;
        dst[i] = lcg_rand() % NUM_NODES;
        degree[src[i]]++;
    }

    /* Build offsets */
    adj_offset[0] = 0;
    for (int i = 0; i < NUM_NODES; i++) {
        adj_offset[i + 1] = adj_offset[i] + degree[i];
    }

    /* Fill adjacency list */
    int *pos = (int *)calloc(NUM_NODES, sizeof(int));
    for (int i = 0; i < NUM_EDGES; i++) {
        int s = src[i];
        adj[adj_offset[s] + pos[s]] = dst[i];
        pos[s]++;
    }

    free(src);
    free(dst);
    free(pos);
}

static int bfs_from(int start) {
    memset(visited, 0, sizeof(visited));
    int head = 0, tail = 0;
    visited[start] = 1;
    queue[tail++] = start;
    int count = 1;

    while (head < tail) {
        int node = queue[head++];
        for (int i = adj_offset[node]; i < adj_offset[node + 1]; i++) {
            int neighbor = adj[i];
            if (!visited[neighbor]) {
                visited[neighbor] = 1;
                queue[tail++] = neighbor;
                count++;
            }
        }
    }

    return count;
}

static volatile int sink;

static void run_benchmark(void) {
    lcg_state = 12345;
    build_graph();
    sink = bfs_from(0);
}

int main(void) {
    for (int i = 0; i < 5; i++) run_benchmark();

    long long times[50];
    for (int i = 0; i < 50; i++) {
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        run_benchmark();
        clock_gettime(CLOCK_MONOTONIC, &end);
        times[i] = timespec_diff_ns(&start, &end);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);
    return 0;
}
