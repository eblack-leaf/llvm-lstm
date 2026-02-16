/*
 * Targets: jump-threading, simplifycfg, adce, instcombine, early-cse
 *
 * Chains of conditional branches, state machines, redundant checks,
 * diamond merges, correlated branches, nested switches.
 */
#include "bench_timing.h"

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

/* Extended classifier: 3rd level of correlated checks */
static int classify_deep(int val) {
    int cat = classify(val);
    int tier = cat / 100;
    int w = cat % 100;

    int bonus = 0;
    if (tier == 1) {
        if (w == 10) bonus = val & 0xF;
        else bonus = (val >> 4) & 0xF;
    } else if (tier == 2) {
        if (w <= 20) bonus = val % 7;
        else bonus = val % 11;
    } else if (tier == 3) {
        /* Redundant: we know val >= 500 && val < 2000 here */
        if (val < 1000) bonus = val % 13;
        else bonus = val % 17;
    } else {
        if (val > 4000) bonus = 100;
        else bonus = 50;
    }
    return cat + bonus;
}

/*
 * State machine with 6 states and predictable transitions.
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
                if (cmd < 3) { state = 4; score += 9; }
                else { state = 0; score += 10; }
                break;
            case 4:
                if (cmd == 0) { state = 5; score += 11; }
                else if (cmd < 4) { state = 2; score += 12; }
                else { state = 1; score += 13; }
                break;
            case 5:
                if (cmd < 6) { state = 0; score += 14; }
                else { state = 3; score += 15; }
                break;
            default:
                state = 0;
                break;
        }
    }
    return score;
}

/* Second state machine: different transition table, tests CSE across machines */
static long long state_machine2(const int *input, int n) {
    int state = 0;
    long long score = 0;
    for (int i = 0; i < n; i++) {
        int cmd = input[i] % 6;
        switch (state) {
            case 0: state = (cmd < 2) ? 1 : (cmd < 4) ? 2 : 3; score += cmd + 1; break;
            case 1: state = (cmd < 3) ? 0 : 2; score += cmd * 2; break;
            case 2: state = (cmd == 0) ? 3 : (cmd < 3) ? 1 : 0; score += cmd + 3; break;
            case 3: state = (cmd < 2) ? 0 : 1; score += cmd * 3; break;
            default: state = 0; break;
        }
    }
    return score;
}

/*
 * Redundant condition checks — same comparison multiple times.
 */
static long long redundant_checks(const int *arr, int n) {
    long long total = 0;
    int i;
    for (i = 0; i < n; i++) {
        int v = arr[i];

        if (v > 500)
            total += v;
        if (v > 500 && v > 0)  /* v>0 always true when v>500 */
            total += v * 2;
        if (v <= 500)
            total += 1;
        else
            total += v + 1;

        /* More redundancy: correlated range checks */
        if (v > 1000) {
            total += v - 1000;
            if (v > 500)  /* always true here */
                total += 1;
        }
    }
    return total;
}

/*
 * Diamond control flow with common tails.
 */
static long long diamond_merge(const int *arr, int n) {
    long long total = 0;
    int i;
    for (i = 0; i < n; i++) {
        int result;
        if (arr[i] % 2 == 0) {
            result = arr[i] * 3;
        } else {
            result = arr[i] * 5;
        }
        total += result + 1;
    }
    return total;
}

/* Nested diamond: two levels of branching with common tails at each */
static long long nested_diamond(const int *arr, int n) {
    long long total = 0;
    for (int i = 0; i < n; i++) {
        int v = arr[i];
        int r;
        if (v % 4 == 0) {
            if (v % 8 == 0) r = v + 100;
            else r = v + 200;
            r += 10;  /* common tail for inner if */
        } else if (v % 4 == 1) {
            if (v % 3 == 0) r = v * 2;
            else r = v * 3;
            r += 20;
        } else {
            r = v + 50;
        }
        total += r;  /* common tail for outer if */
    }
    return total;
}

/* Correlated branches: value checked in one branch constrains later checks */
static long long correlated_branches(const int *arr, int n) {
    long long total = 0;
    for (int i = 0; i < n; i++) {
        int v = arr[i];
        int sign = 0, magnitude = 0;

        /* First branch determines sign */
        if (v >= 2500) sign = 1;
        else sign = -1;

        /* Second branch: redundant check after sign is known */
        if (sign > 0) {
            /* We know v >= 2500 */
            if (v > 2500) magnitude = v - 2500;  /* almost always true */
            else magnitude = 0;
        } else {
            /* We know v < 2500 */
            if (v < 1000) magnitude = 1000 - v;
            else magnitude = v;
        }

        total += sign * magnitude;
    }
    return total;
}

/* Multi-way lookup table emulated with switches — simplifycfg target */
static int lookup_switch(int key) {
    int val;
    switch (key % 16) {
        case 0:  val = 42; break;
        case 1:  val = 17; break;
        case 2:  val = 93; break;
        case 3:  val = 55; break;
        case 4:  val = 31; break;
        case 5:  val = 78; break;
        case 6:  val = 64; break;
        case 7:  val = 22; break;
        case 8:  val = 89; break;
        case 9:  val = 45; break;
        case 10: val = 11; break;
        case 11: val = 73; break;
        case 12: val = 36; break;
        case 13: val = 58; break;
        case 14: val = 27; break;
        case 15: val = 91; break;
        default: val = 0; break;
    }
    return val;
}

static long long switch_workload(const int *arr, int n) {
    long long total = 0;
    for (int i = 0; i < n; i++) {
        total += lookup_switch(arr[i]);
        /* Chain two lookups: switch result feeds into classify */
        int lv = lookup_switch(arr[i]);
        total += classify(lv * 50);
    }
    return total;
}

#define N 8000

static long long workload(int *arr) {
    long long total = 0;

    /* Deep classify each element */
    for (int i = 0; i < N; i++) {
        total += classify_deep(arr[i]);
    }

    total += state_machine(arr, N);
    total += state_machine2(arr, N);
    total += redundant_checks(arr, N);
    total += diamond_merge(arr, N);
    total += nested_diamond(arr, N);
    total += correlated_branches(arr, N);
    total += switch_workload(arr, N);

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    int *arr = (int *)malloc(N * sizeof(int));
    int i;

    bench_lcg_seed(12345);
    for (i = 0; i < N; i++) {
        arr[i] = (int)(bench_lcg_rand() % 5000);
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(arr); });

    free(arr);
    return 0;
}
