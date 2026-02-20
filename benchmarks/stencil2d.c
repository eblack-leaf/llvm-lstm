#include "bench_timing.h"

/*
 * 2D stencil computations — heavy vectorizable array work
 * that generates vector ops for vector-combine to optimize.
 */

#define ROWS 64
#define COLS 64
#define GRID (ROWS * COLS)

/* 5-point stencil: Jacobi iteration step */
static void jacobi_step(const int *in, int *out) {
    int r, c;
    for (r = 1; r < ROWS - 1; r++) {
        for (c = 1; c < COLS - 1; c++) {
            out[r * COLS + c] = (in[(r - 1) * COLS + c] +
                                  in[(r + 1) * COLS + c] +
                                  in[r * COLS + (c - 1)] +
                                  in[r * COLS + (c + 1)]) / 4;
        }
    }
}

/* 9-point stencil with weights */
static void weighted_stencil(const int *in, int *out) {
    int r, c;
    for (r = 1; r < ROWS - 1; r++) {
        for (c = 1; c < COLS - 1; c++) {
            int center = in[r * COLS + c] * 4;
            int cross = in[(r - 1) * COLS + c] + in[(r + 1) * COLS + c] +
                        in[r * COLS + (c - 1)] + in[r * COLS + (c + 1)];
            int diag = in[(r - 1) * COLS + (c - 1)] + in[(r - 1) * COLS + (c + 1)] +
                       in[(r + 1) * COLS + (c - 1)] + in[(r + 1) * COLS + (c + 1)];
            out[r * COLS + c] = (center + cross * 2 + diag) / 16;
        }
    }
}

/* Row-wise prefix sum */
static void prefix_sum_rows(int *grid) {
    int r, c;
    for (r = 0; r < ROWS; r++) {
        for (c = 1; c < COLS; c++) {
            grid[r * COLS + c] += grid[r * COLS + (c - 1)];
        }
    }
}

/* Column-wise reduction (sum each column) */
static void col_reduce(const int *grid, int *col_sums) {
    int r, c;
    for (c = 0; c < COLS; c++) col_sums[c] = 0;
    for (r = 0; r < ROWS; r++) {
        for (c = 0; c < COLS; c++) {
            col_sums[c] += grid[r * COLS + c];
        }
    }
}

/* Element-wise blend of two grids based on mask */
static void blend_grids(const int *a, const int *b, const int *mask, int *out) {
    int i;
    for (i = 0; i < GRID; i++) {
        out[i] = (mask[i] & 1) ? a[i] : b[i];
    }
}

/* Saturating add (clamp to [0, 10000]) */
static void saturate_add(int *grid, const int *delta) {
    int i;
    for (i = 0; i < GRID; i++) {
        int val = grid[i] + delta[i];
        if (val < 0) val = 0;
        if (val > 10000) val = 10000;
        grid[i] = val;
    }
}

static long long workload(int *a, int *b, int *c, int *mask, int *col_sums) {
    long long sum = 0;
    int iter, i;

    for (iter = 0; iter < 10; iter++) {
        jacobi_step(a, b);
        weighted_stencil(b, c);
        blend_grids(b, c, mask, a);
        saturate_add(a, c);
        prefix_sum_rows(a);
        col_reduce(a, col_sums);

        for (i = 0; i < COLS; i++) {
            sum += col_sums[i];
        }

        /* Reset to prevent overflow */
        for (i = 0; i < GRID; i++) {
            a[i] = a[i] % 1000;
        }
    }

    return sum;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    int a[GRID], b[GRID], c[GRID], mask[GRID], col_sums[COLS];
    int i;

    bench_lcg_seed(77);
    for (i = 0; i < GRID; i++) {
        a[i] = (int)(bench_lcg_rand() % 1000);
        b[i] = 0;
        c[i] = 0;
        mask[i] = (int)(bench_lcg_rand() % 2);
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(a, b, c, mask, col_sums); });

    return 0;
}
