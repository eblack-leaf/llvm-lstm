/*
 * Targets: jump-threading, simplifycfg, adce, instcombine, early-cse
 *
 * Chains of conditional branches where the outcome of one determines
 * the next. Jump-threading can collapse multi-hop branches into direct
 * jumps. Also includes dead branches and redundant condition checks.
 */
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
static unsigned int lcg_rand(void) {
    lcg_state = lcg_state * 1103515245 + 12345;
    return (lcg_state >> 16) & 0x7fff;
}

/*
 * Classify value through chained conditions.
 * Jump-threading can collapse: if category is set in first branch,
 * subsequent checks on the same variable can be threaded.
 */
static int classify(int val) {
    int category = 0;

    if (val < 100)
        category = 1;
    else if (val < 500)
        category = 2;
    else if (val < 2000)
        category = 3;
    else
        category = 4;

    /* Second check that depends on first — jump-threading target.
     * After the first chain, we know val's range, so these checks
     * are partially redundant. */
    int weight;
    if (category <= 2) {
        if (val < 50) weight = 10;
        else if (val < 250) weight = 20;
        else weight = 30;
    } else {
        if (val < 1000) weight = 40;
        else if (val < 3000) weight = 50;
        else weight = 60;
    }

    return category * 100 + weight;
}

/*
 * State machine with predictable transitions — simplifycfg + jump-threading.
 * Many states lead to the same successor.
 */
static long long state_machine(const int *input, int n) {
    int state = 0;
    long long score = 0;
    int i;

    for (i = 0; i < n; i++) {
        int cmd = input[i] % 8;
        switch (state) {
            case 0:
                if (cmd < 3) { state = 1; score += 1; }
                else if (cmd < 6) { state = 2; score += 2; }
                else { state = 0; score += 0; }
                break;
            case 1:
                if (cmd < 2) { state = 0; score += 3; }
                else if (cmd < 5) { state = 2; score += 4; }
                else { state = 3; score += 5; }
                break;
            case 2:
                if (cmd == 0) { state = 0; score += 6; }
                else if (cmd < 4) { state = 1; score += 7; }
                else { state = 3; score += 8; }
                break;
            case 3:
                if (cmd < 3) { state = 1; score += 9; }
                else { state = 0; score += 10; }
                break;
            default:
                state = 0;
                break;
        }
    }
    return score;
}

/*
 * Redundant condition checks — the same comparison evaluated multiple times.
 * early-cse should eliminate, and jump-threading should thread through.
 */
static long long redundant_checks(const int *arr, int n) {
    long long total = 0;
    int i;
    for (i = 0; i < n; i++) {
        int v = arr[i];

        /* Check v > 500 multiple times in different ways */
        if (v > 500)
            total += v;
        /* Dead branch if we get here and v > 500 is still true */
        if (v > 500 && v > 0)  /* v>0 is always true when v>500 */
            total += v * 2;
        if (v <= 500)
            total += 1;
        else  /* Same as v > 500, redundant with first check */
            total += v + 1;
    }
    return total;
}

/*
 * Diamond control flow with common tails — simplifycfg should merge.
 */
static long long diamond_merge(const int *arr, int n) {
    long long total = 0;
    int i;
    for (i = 0; i < n; i++) {
        int result;
        if (arr[i] % 2 == 0) {
            result = arr[i] * 3;
            /* Common tail: both branches add to total */
        } else {
            result = arr[i] * 5;
        }
        total += result + 1;  /* Common code */
    }
    return total;
}

#define N 200000

static long long workload(int *arr) {
    long long total = 0;
    int i;

    /* Classify each element through chained branches */
    for (i = 0; i < N; i++) {
        total += classify(arr[i]);
    }

    total += state_machine(arr, N);
    total += redundant_checks(arr, N);
    total += diamond_merge(arr, N);

    return total;
}

int main(void) {
    int *arr = (int *)malloc(N * sizeof(int));
    int i;

    lcg_state = 12345;
    for (i = 0; i < N; i++) {
        arr[i] = (int)(lcg_rand() % 5000);
    }

    /* Warmup */
    volatile long long sink;
    for (i = 0; i < 5; i++) {
        sink = workload(arr);
    }

    /* Timing */
    long long times[50];
    struct timespec t0, t1;
    for (i = 0; i < 50; i++) {
        clock_gettime(CLOCK_MONOTONIC, &t0);
        sink = workload(arr);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        times[i] = timespec_diff_ns(&t0, &t1);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);

    free(arr);
    return 0;
}
