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

#define NUM_OPS    100000
#define TABLE_SIZE 200003  /* prime, ~2x NUM_OPS for load factor ~0.5 */

#define SLOT_EMPTY   0
#define SLOT_OCCUPIED 1

typedef struct {
    unsigned int key;
    unsigned int value;
    unsigned char state;
} Slot;

static Slot table[TABLE_SIZE];
static unsigned int keys[NUM_OPS];
static unsigned int values[NUM_OPS];

static void ht_clear(void) {
    memset(table, 0, sizeof(table));
}

static void ht_insert(unsigned int key, unsigned int value) {
    unsigned int idx = key % TABLE_SIZE;
    while (table[idx].state == SLOT_OCCUPIED) {
        if (table[idx].key == key) {
            table[idx].value = value;
            return;
        }
        idx = (idx + 1) % TABLE_SIZE;
    }
    table[idx].key = key;
    table[idx].value = value;
    table[idx].state = SLOT_OCCUPIED;
}

static unsigned int ht_lookup(unsigned int key) {
    unsigned int idx = key % TABLE_SIZE;
    while (table[idx].state == SLOT_OCCUPIED) {
        if (table[idx].key == key) return table[idx].value;
        idx = (idx + 1) % TABLE_SIZE;
    }
    return 0;
}

static volatile unsigned long long sink;

static void run_benchmark(void) {
    lcg_state = 12345;

    /* Generate keys and values */
    for (int i = 0; i < NUM_OPS; i++) {
        keys[i] = (lcg_rand() << 15) | lcg_rand();
        values[i] = (lcg_rand() << 15) | lcg_rand();
    }

    ht_clear();

    /* Insert */
    for (int i = 0; i < NUM_OPS; i++) {
        ht_insert(keys[i], values[i]);
    }

    /* Lookup */
    unsigned long long sum = 0;
    for (int i = 0; i < NUM_OPS; i++) {
        sum += ht_lookup(keys[i]);
    }
    sink = sum;
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
