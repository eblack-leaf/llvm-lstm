/*
 * Targets: sroa, mem2reg, instcombine, inline, gvn, early-cse
 *
 * Uses local structs passed by value through helper functions.
 * SROA should decompose these into scalar registers.
 * Multiple struct types, recursive struct operations, and struct arrays.
 */
#include "bench_timing.h"

typedef struct {
    double x, y, z;
} Vec3;

typedef struct {
    double real, imag;
} Complex;

typedef struct {
    int lo, hi, mid;
    int found;
} SearchState;

typedef struct {
    double m[4]; /* 2x2 matrix stored flat */
} Mat2;

typedef struct {
    Vec3 position;
    Vec3 velocity;
    double mass;
} Particle;

/* ---- Vec3 operations ---- */

static Vec3 vec3_add(Vec3 a, Vec3 b) {
    Vec3 r;
    r.x = a.x + b.x;
    r.y = a.y + b.y;
    r.z = a.z + b.z;
    return r;
}

static Vec3 vec3_sub(Vec3 a, Vec3 b) {
    Vec3 r;
    r.x = a.x - b.x;
    r.y = a.y - b.y;
    r.z = a.z - b.z;
    return r;
}

static Vec3 vec3_scale(Vec3 a, double s) {
    Vec3 r;
    r.x = a.x * s;
    r.y = a.y * s;
    r.z = a.z * s;
    return r;
}

static double vec3_dot(Vec3 a, Vec3 b) {
    return a.x * b.x + a.y * b.y + a.z * b.z;
}

static Vec3 vec3_cross(Vec3 a, Vec3 b) {
    Vec3 r;
    r.x = a.y * b.z - a.z * b.y;
    r.y = a.z * b.x - a.x * b.z;
    r.z = a.x * b.y - a.y * b.x;
    return r;
}

static double vec3_length_sq(Vec3 a) {
    return a.x * a.x + a.y * a.y + a.z * a.z;
}

static Vec3 vec3_normalize(Vec3 a) {
    double len_sq = vec3_length_sq(a);
    double inv_len = 1.0 / (len_sq + 1e-12);
    double guess = inv_len;
    guess = 0.5 * (guess + inv_len / guess);
    guess = 0.5 * (guess + inv_len / guess);
    return vec3_scale(a, guess);
}

/* ---- Complex operations ---- */

static Complex complex_mul(Complex a, Complex b) {
    Complex r;
    r.real = a.real * b.real - a.imag * b.imag;
    r.imag = a.real * b.imag + a.imag * b.real;
    return r;
}

static Complex complex_add(Complex a, Complex b) {
    Complex r;
    r.real = a.real + b.real;
    r.imag = a.imag + b.imag;
    return r;
}

static double complex_abs_sq(Complex a) {
    return a.real * a.real + a.imag * a.imag;
}

/* Complex power: z^n by repeated squaring */
static Complex complex_pow(Complex z, int n) {
    Complex result = {1.0, 0.0};
    Complex base = z;
    while (n > 0) {
        if (n & 1) result = complex_mul(result, base);
        base = complex_mul(base, base);
        n >>= 1;
    }
    return result;
}

/* ---- Mat2 operations (2x2 matrix) ---- */

static Mat2 mat2_mul(Mat2 a, Mat2 b) {
    Mat2 r;
    r.m[0] = a.m[0] * b.m[0] + a.m[1] * b.m[2];
    r.m[1] = a.m[0] * b.m[1] + a.m[1] * b.m[3];
    r.m[2] = a.m[2] * b.m[0] + a.m[3] * b.m[2];
    r.m[3] = a.m[2] * b.m[1] + a.m[3] * b.m[3];
    return r;
}

static double mat2_det(Mat2 a) {
    return a.m[0] * a.m[3] - a.m[1] * a.m[2];
}

static double mat2_trace(Mat2 a) {
    return a.m[0] + a.m[3];
}

/* Matrix power by repeated squaring — struct by value through recursion-like loop */
static Mat2 mat2_pow(Mat2 m, int n) {
    Mat2 result = {{1.0, 0.0, 0.0, 1.0}}; /* identity */
    Mat2 base = m;
    while (n > 0) {
        if (n & 1) result = mat2_mul(result, base);
        base = mat2_mul(base, base);
        n >>= 1;
    }
    return result;
}

/* ---- Particle simulation ---- */

static Particle particle_step(Particle p, Vec3 force, double dt) {
    Vec3 accel = vec3_scale(force, 1.0 / p.mass);
    p.velocity = vec3_add(p.velocity, vec3_scale(accel, dt));
    p.position = vec3_add(p.position, vec3_scale(p.velocity, dt));
    return p;
}

static Vec3 gravity_force(Particle a, Particle b) {
    Vec3 diff = vec3_sub(b.position, a.position);
    double dist_sq = vec3_length_sq(diff) + 0.01; /* softening */
    double inv_dist = 1.0 / dist_sq;
    double force_mag = a.mass * b.mass * inv_dist;
    return vec3_scale(diff, force_mag);
}

/* ---- Binary search with struct state ---- */

static int search_helper(const int *arr, int n, int target) {
    SearchState s;
    s.lo = 0;
    s.hi = n - 1;
    s.found = -1;
    while (s.lo <= s.hi) {
        s.mid = s.lo + (s.hi - s.lo) / 2;
        if (arr[s.mid] == target) { s.found = s.mid; return s.found; }
        else if (arr[s.mid] < target) s.lo = s.mid + 1;
        else s.hi = s.mid - 1;
    }
    return s.found;
}

#define VEC_N 10000
#define COMPLEX_ITERS 1000
#define N_PARTICLES 20
#define SIM_STEPS 50

static long long workload(double *data, int *sorted_arr) {
    long long total = 0;
    int i;

    /* Vec3 operations — many small structs passed by value */
    Vec3 acc = {0.0, 0.0, 0.0};
    for (i = 0; i + 2 < VEC_N; i += 3) {
        Vec3 v = {data[i], data[i + 1], data[i + 2]};
        Vec3 w = {data[i + 2], data[i], data[i + 1]};
        acc = vec3_add(acc, v);
        acc = vec3_add(acc, vec3_scale(w, 0.5));
        total += (long long)(vec3_dot(v, w) * 1000.0);
        /* Cross product — more struct operations */
        Vec3 c = vec3_cross(v, w);
        total += (long long)(c.x * 100.0);
    }
    Vec3 n = vec3_normalize(acc);
    total += (long long)(n.x * 1e6);

    /* Complex number iteration — Mandelbrot-style */
    Complex z = {0.0, 0.0};
    Complex c = {0.3, 0.5};
    for (i = 0; i < COMPLEX_ITERS; i++) {
        z = complex_add(complex_mul(z, z), c);
        if (z.real > 1e6) z.real = 1e6;
        if (z.imag > 1e6) z.imag = 1e6;
        if (z.real < -1e6) z.real = -1e6;
        if (z.imag < -1e6) z.imag = -1e6;
    }
    total += (long long)(z.real * 1000.0);

    /* Complex power series */
    Complex base = {0.99, 0.01};
    for (i = 2; i <= 20; i++) {
        Complex p = complex_pow(base, i);
        total += (long long)(complex_abs_sq(p) * 1e6);
    }

    /* 2x2 matrix powers — Fibonacci-style computation */
    Mat2 fib = {{1.0, 1.0, 1.0, 0.0}};
    for (i = 2; i <= 30; i++) {
        Mat2 fp = mat2_pow(fib, i);
        total += (long long)(mat2_trace(fp));
        total += (long long)(mat2_det(fp) * 100.0);
    }

    /* Particle simulation — structs containing structs */
    Particle particles[N_PARTICLES];
    for (i = 0; i < N_PARTICLES; i++) {
        particles[i].position = (Vec3){data[i * 3 % VEC_N], data[(i * 3 + 1) % VEC_N], data[(i * 3 + 2) % VEC_N]};
        particles[i].velocity = (Vec3){0.0, 0.0, 0.0};
        particles[i].mass = 1.0 + data[i % VEC_N];
    }
    for (int step = 0; step < SIM_STEPS; step++) {
        for (i = 0; i < N_PARTICLES; i++) {
            Vec3 force = {0.0, 0.0, 0.0};
            for (int j = 0; j < N_PARTICLES; j++) {
                if (i != j) {
                    force = vec3_add(force, gravity_force(particles[i], particles[j]));
                }
            }
            particles[i] = particle_step(particles[i], force, 0.01);
        }
    }
    for (i = 0; i < N_PARTICLES; i++) {
        total += (long long)(vec3_length_sq(particles[i].position) * 100.0);
    }

    /* Search with struct state */
    for (i = 0; i < 200; i++) {
        int target = sorted_arr[bench_lcg_rand() % VEC_N];
        total += search_helper(sorted_arr, VEC_N, target);
    }

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    double *data = (double *)malloc(VEC_N * sizeof(double));
    int *sorted_arr = (int *)malloc(VEC_N * sizeof(int));
    int i;

    bench_lcg_seed(12345);
    for (i = 0; i < VEC_N; i++) {
        data[i] = (double)bench_lcg_rand() / 32768.0;
        sorted_arr[i] = i * 3 + 1;
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(data, sorted_arr); });

    free(data);
    free(sorted_arr);
    return 0;
}
