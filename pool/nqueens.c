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

/* --- Variant 1: Iterative with explicit stack --- */

static int nqueens_iterative(void) {
    int all = (1 << BOARD_SIZE) - 1;
    int cols_stk[BOARD_SIZE + 1], d1_stk[BOARD_SIZE + 1], d2_stk[BOARD_SIZE + 1];
    int avail_stk[BOARD_SIZE + 1];
    int row = 0, count = 0;

    cols_stk[0] = 0; d1_stk[0] = 0; d2_stk[0] = 0;
    avail_stk[0] = all & ~(cols_stk[0] | d1_stk[0] | d2_stk[0]);

    while (row >= 0) {
        if (row == BOARD_SIZE) {
            count++;
            row--;
            continue;
        }
        if (avail_stk[row] == 0) {
            row--;
            continue;
        }
        int bit = avail_stk[row] & (-avail_stk[row]);
        avail_stk[row] -= bit;
        /* Push next row */
        cols_stk[row + 1] = cols_stk[row] | bit;
        d1_stk[row + 1] = (d1_stk[row] | bit) << 1;
        d2_stk[row + 1] = (d2_stk[row] | bit) >> 1;
        row++;
        avail_stk[row] = all & ~(cols_stk[row] | d1_stk[row] | d2_stk[row]);
    }
    return count;
}

/* --- Variant 2: Array-based with conflict check function --- */

static int board_arr[BOARD_SIZE];

static int safe_to_place(int row, int col) {
    int i;
    for (i = 0; i < row; i++) {
        if (board_arr[i] == col) return 0;
        int diff = row - i;
        if (board_arr[i] - col == diff || col - board_arr[i] == diff) return 0;
    }
    return 1;
}

static int nqueens_array_count;

static void nqueens_array(int row) {
    if (row == BOARD_SIZE) {
        nqueens_array_count++;
        return;
    }
    int col;
    for (col = 0; col < BOARD_SIZE; col++) {
        if (safe_to_place(row, col)) {
            board_arr[row] = col;
            nqueens_array(row + 1);
        }
    }
}

/* --- Variant 3: Permutation-based with boolean diagonal arrays --- */

static int perm_col_used;  /* bitmask */
static int diag_pos[2 * BOARD_SIZE];  /* row + col */
static int diag_neg[2 * BOARD_SIZE];  /* row - col + BOARD_SIZE */
static int perm_count;

static void solve_perm(int row) {
    if (row == BOARD_SIZE) {
        perm_count++;
        return;
    }
    int avail = ((1 << BOARD_SIZE) - 1) & ~perm_col_used;
    while (avail) {
        int bit = avail & (-avail);
        avail -= bit;
        /* Convert bit to column index */
        int col = 0, tmp = bit;
        while (tmp > 1) { tmp >>= 1; col++; }

        int dp = row + col;
        int dn = row - col + BOARD_SIZE;
        if (diag_pos[dp] || diag_neg[dn]) continue;

        perm_col_used |= bit;
        diag_pos[dp] = 1;
        diag_neg[dn] = 1;
        solve_perm(row + 1);
        perm_col_used &= ~bit;
        diag_pos[dp] = 0;
        diag_neg[dn] = 0;
    }
}

static int do_nqueens_all(void) {
    int total = 0;

    /* Original bitmask recursive */
    total += do_nqueens();

    /* Iterative */
    total += nqueens_iterative();

    /* Array-based */
    nqueens_array_count = 0;
    nqueens_array(0);
    total += nqueens_array_count;

    /* Permutation-based */
    perm_col_used = 0;
    memset(diag_pos, 0, sizeof(diag_pos));
    memset(diag_neg, 0, sizeof(diag_neg));
    perm_count = 0;
    solve_perm(0);
    total += perm_count;

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    volatile int sink;
    BENCH_TIME(niters, { sink = do_nqueens_all(); });
    return 0;
}
