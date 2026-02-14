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

#define BUF_SIZE (100 * 1024)
#define COLS 10

static char csv_buf[BUF_SIZE + 1];
static int csv_len;

static void generate_csv(void) {
    int pos = 0;
    lcg_state = 12345;

    while (pos < BUF_SIZE - 200) {
        for (int col = 0; col < COLS; col++) {
            if (col > 0 && pos < BUF_SIZE) csv_buf[pos++] = ',';
            /* Generate a number: integer or decimal */
            char num[32];
            int n;
            if (lcg_rand() % 3 == 0) {
                /* Decimal */
                int whole = (int)(lcg_rand() % 10000);
                int frac = (int)(lcg_rand() % 100);
                n = sprintf(num, "%d.%02d", whole, frac);
            } else {
                /* Integer */
                int val = (int)(lcg_rand() % 100000);
                if (lcg_rand() % 4 == 0) val = -val;
                n = sprintf(num, "%d", val);
            }
            for (int i = 0; i < n && pos < BUF_SIZE; i++)
                csv_buf[pos++] = num[i];
        }
        if (pos < BUF_SIZE) csv_buf[pos++] = '\n';
    }

    csv_buf[pos] = '\0';
    csv_len = pos;
}

static volatile double total_sum;

static void do_parse(void) {
    double sum = 0.0;
    int i = 0;
    while (i < csv_len) {
        /* Parse a number field */
        char field[32];
        int fi = 0;
        while (i < csv_len && csv_buf[i] != ',' && csv_buf[i] != '\n' && fi < 31) {
            field[fi++] = csv_buf[i++];
        }
        field[fi] = '\0';
        if (fi > 0)
            sum += atof(field);

        /* Skip delimiter */
        if (i < csv_len && (csv_buf[i] == ',' || csv_buf[i] == '\n'))
            i++;
    }
    total_sum = sum;
}

int main(void) {
    generate_csv();

    /* Warmup */
    for (int w = 0; w < 5; w++)
        do_parse();

    /* Timed runs */
    long long times[50];
    for (int t = 0; t < 50; t++) {
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        do_parse();
        clock_gettime(CLOCK_MONOTONIC, &end);
        times[t] = timespec_diff_ns(&start, &end);
    }

    qsort(times, 50, sizeof(long long), cmp_ll);
    printf("%lld\n", times[25]);
    return 0;
}
