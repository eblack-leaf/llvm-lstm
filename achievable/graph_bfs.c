/*
 * Targets: inline, loop-unroll, simplifycfg, gvn, mem2reg, sroa
 *
 * Graph algorithms: BFS, DFS, connected components, topological-ish ordering,
 * degree histogram. CSR representation with multiple traversal patterns.
 */
#include "bench_timing.h"

#define NUM_NODES 2000
#define NUM_EDGES 10000
#define MAX_DEGREE 64

/* CSR representation */
static int adj[NUM_EDGES];
static int adj_offset[NUM_NODES + 1];
static int out_degree[NUM_NODES];

/* Edge weights for weighted traversal */
static int edge_weight[NUM_EDGES];

/* Work buffers */
static int visited[NUM_NODES];
static int queue[NUM_NODES];
static int dist[NUM_NODES];
static int component[NUM_NODES];
static int order[NUM_NODES];
static int stack[NUM_NODES];

static void build_graph(void) {
    memset(out_degree, 0, sizeof(out_degree));

    int *src = (int *)malloc(NUM_EDGES * sizeof(int));
    int *dst = (int *)malloc(NUM_EDGES * sizeof(int));

    for (int i = 0; i < NUM_EDGES; i++) {
        src[i] = bench_lcg_rand() % NUM_NODES;
        dst[i] = bench_lcg_rand() % NUM_NODES;
        edge_weight[i] = 1 + bench_lcg_rand() % 20;
        out_degree[src[i]]++;
    }

    adj_offset[0] = 0;
    for (int i = 0; i < NUM_NODES; i++) {
        adj_offset[i + 1] = adj_offset[i] + out_degree[i];
    }

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

/* BFS from a start node, returns count of reachable nodes */
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

/* BFS with distance tracking */
static long long bfs_distances(int start) {
    memset(dist, -1, sizeof(dist));
    int head = 0, tail = 0;
    dist[start] = 0;
    queue[tail++] = start;
    long long total_dist = 0;

    while (head < tail) {
        int node = queue[head++];
        for (int i = adj_offset[node]; i < adj_offset[node + 1]; i++) {
            int neighbor = adj[i];
            if (dist[neighbor] == -1) {
                dist[neighbor] = dist[node] + 1;
                total_dist += dist[neighbor];
                queue[tail++] = neighbor;
            }
        }
    }
    return total_dist;
}

/* DFS iterative — different traversal order, stack-based */
static int dfs_from(int start) {
    memset(visited, 0, sizeof(visited));
    int top = 0;
    stack[top++] = start;
    int count = 0;

    while (top > 0) {
        int node = stack[--top];
        if (visited[node]) continue;
        visited[node] = 1;
        count++;
        for (int i = adj_offset[node]; i < adj_offset[node + 1]; i++) {
            int neighbor = adj[i];
            if (!visited[neighbor]) {
                stack[top++] = neighbor;
            }
        }
    }
    return count;
}

/* Connected components via repeated BFS */
static int find_components(void) {
    memset(component, -1, sizeof(component));
    int num_comp = 0;

    for (int n = 0; n < NUM_NODES; n++) {
        if (component[n] != -1) continue;
        /* BFS from n */
        int head = 0, tail = 0;
        queue[tail++] = n;
        component[n] = num_comp;
        while (head < tail) {
            int node = queue[head++];
            for (int i = adj_offset[node]; i < adj_offset[node + 1]; i++) {
                int neighbor = adj[i];
                if (component[neighbor] == -1) {
                    component[neighbor] = num_comp;
                    queue[tail++] = neighbor;
                }
            }
        }
        num_comp++;
    }
    return num_comp;
}

/* Degree histogram — tests loop opts, branchy binning */
static long long degree_histogram(void) {
    int bins[8]; /* 0, 1-2, 3-4, 5-8, 9-16, 17-32, 33-64, 65+ */
    memset(bins, 0, sizeof(bins));

    for (int i = 0; i < NUM_NODES; i++) {
        int d = adj_offset[i + 1] - adj_offset[i];
        if (d == 0) bins[0]++;
        else if (d <= 2) bins[1]++;
        else if (d <= 4) bins[2]++;
        else if (d <= 8) bins[3]++;
        else if (d <= 16) bins[4]++;
        else if (d <= 32) bins[5]++;
        else if (d <= 64) bins[6]++;
        else bins[7]++;
    }

    long long result = 0;
    for (int i = 0; i < 8; i++) {
        result += bins[i] * (long long)(i + 1);
    }
    return result;
}

/* Weighted BFS: sum of edge weights along shortest paths */
static long long weighted_bfs(int start) {
    memset(dist, -1, sizeof(dist));
    int head = 0, tail = 0;
    dist[start] = 0;
    queue[tail++] = start;
    long long total_weight = 0;

    while (head < tail) {
        int node = queue[head++];
        for (int i = adj_offset[node]; i < adj_offset[node + 1]; i++) {
            int neighbor = adj[i];
            if (dist[neighbor] == -1) {
                dist[neighbor] = dist[node] + edge_weight[i];
                total_weight += edge_weight[i];
                queue[tail++] = neighbor;
            }
        }
    }
    return total_weight;
}

/* Multi-hop neighbor count: count nodes within distance 2 */
static int two_hop_count(int start) {
    memset(dist, -1, sizeof(dist));
    int head = 0, tail = 0;
    dist[start] = 0;
    queue[tail++] = start;
    int count = 0;

    while (head < tail) {
        int node = queue[head++];
        if (dist[node] >= 2) continue;
        for (int i = adj_offset[node]; i < adj_offset[node + 1]; i++) {
            int neighbor = adj[i];
            if (dist[neighbor] == -1) {
                dist[neighbor] = dist[node] + 1;
                queue[tail++] = neighbor;
                count++;
            }
        }
    }
    return count;
}

static long long run_benchmark(void) {
    long long total = 0;

    bench_lcg_seed(12345);
    build_graph();

    /* Multiple BFS from different start nodes */
    for (int s = 0; s < NUM_NODES; s += NUM_NODES / 10) {
        total += bfs_from(s);
    }

    /* BFS with distances */
    total += bfs_distances(0);
    total += bfs_distances(NUM_NODES / 2);

    /* DFS traversals */
    for (int s = 0; s < NUM_NODES; s += NUM_NODES / 5) {
        total += dfs_from(s);
    }

    /* Connected components */
    total += find_components();

    /* Degree histogram */
    total += degree_histogram();

    /* Weighted BFS */
    total += weighted_bfs(0);

    /* Two-hop neighbor count from several nodes */
    for (int s = 0; s < NUM_NODES; s += NUM_NODES / 8) {
        total += two_hop_count(s);
    }

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    volatile long long sink;
    BENCH_TIME(niters, { sink = run_benchmark(); });

    return 0;
}
