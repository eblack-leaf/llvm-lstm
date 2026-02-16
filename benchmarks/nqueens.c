#include "bench_timing.h"

#define BOARD_SIZE 10

static int solution_count;

static void solve(int row, int cols, int diag1, int diag2) {
    if (row == BOARD_SIZE) {
        solution_count++;
        return;
    }
    int avail = ((1 << BOARD_SIZE) - 1) & ~(cols | diag1 | diag2);
    while (avail) {
        int bit = avail & (-avail);  /* lowest set bit */
        avail -= bit;
        solve(row + 1, cols | bit, (diag1 | bit) << 1, (diag2 | bit) >> 1);
    }
}

static int do_nqueens(void) {
    solution_count = 0;
    solve(0, 0, 0, 0);
    return solution_count;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    volatile int sink;
    BENCH_TIME(niters, { sink = do_nqueens(); });
    return 0;
}
