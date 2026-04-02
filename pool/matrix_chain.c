/*
 * Targets: inline (chain of small ops), sroa (matrix structs), gvn
 * (redundant loads across calls), licm (constants in chains),
 * instcombine (FP expressions).
 *
 * Small fixed-size matrix operations chained together. Different from
 * matrix_multiply_tiled.c: focuses on function-call patterns and
 * struct-by-value passing, not large loop nests.
 */
#include "bench_timing.h"

#define MAT_N 4   /* 4x4 matrices */
#define NUM_MATS 20

typedef struct {
    double m[MAT_N * MAT_N];
} Mat4;

/* ---- Basic operations ---- */

static Mat4 mat4_zero(void) {
    Mat4 r;
    memset(r.m, 0, sizeof(r.m));
    return r;
}

static Mat4 mat4_identity(void) {
    Mat4 r = mat4_zero();
    for (int i = 0; i < MAT_N; i++) r.m[i * MAT_N + i] = 1.0;
    return r;
}

static Mat4 mat4_add(Mat4 a, Mat4 b) {
    Mat4 r;
    for (int i = 0; i < MAT_N * MAT_N; i++)
        r.m[i] = a.m[i] + b.m[i];
    return r;
}

static Mat4 mat4_sub(Mat4 a, Mat4 b) {
    Mat4 r;
    for (int i = 0; i < MAT_N * MAT_N; i++)
        r.m[i] = a.m[i] - b.m[i];
    return r;
}

static Mat4 mat4_scale(Mat4 a, double s) {
    Mat4 r;
    for (int i = 0; i < MAT_N * MAT_N; i++)
        r.m[i] = a.m[i] * s;
    return r;
}

static Mat4 mat4_mul(Mat4 a, Mat4 b) {
    Mat4 r = mat4_zero();
    for (int i = 0; i < MAT_N; i++)
        for (int j = 0; j < MAT_N; j++)
            for (int k = 0; k < MAT_N; k++)
                r.m[i * MAT_N + j] += a.m[i * MAT_N + k] * b.m[k * MAT_N + j];
    return r;
}

static Mat4 mat4_transpose(Mat4 a) {
    Mat4 r;
    for (int i = 0; i < MAT_N; i++)
        for (int j = 0; j < MAT_N; j++)
            r.m[i * MAT_N + j] = a.m[j * MAT_N + i];
    return r;
}

static double mat4_trace(Mat4 a) {
    double t = 0.0;
    for (int i = 0; i < MAT_N; i++) t += a.m[i * MAT_N + i];
    return t;
}

static double mat4_frobenius_sq(Mat4 a) {
    double s = 0.0;
    for (int i = 0; i < MAT_N * MAT_N; i++) s += a.m[i] * a.m[i];
    return s;
}

/* Element-wise multiply (Hadamard product) */
static Mat4 mat4_hadamard(Mat4 a, Mat4 b) {
    Mat4 r;
    for (int i = 0; i < MAT_N * MAT_N; i++)
        r.m[i] = a.m[i] * b.m[i];
    return r;
}

/* ---- Higher-level operations ---- */

/* Matrix power via repeated squaring */
static Mat4 mat4_pow(Mat4 base, int n) {
    Mat4 result = mat4_identity();
    while (n > 0) {
        if (n & 1) result = mat4_mul(result, base);
        base = mat4_mul(base, base);
        n >>= 1;
    }
    return result;
}

/* Matrix exponential approximation: sum of I + A + A^2/2! + A^3/3! + ... */
static Mat4 mat4_exp_approx(Mat4 a, int terms) {
    Mat4 result = mat4_identity();
    Mat4 term = mat4_identity();
    for (int i = 1; i < terms; i++) {
        term = mat4_mul(term, a);
        term = mat4_scale(term, 1.0 / i);
        result = mat4_add(result, term);
    }
    return result;
}

/* Compute A * B^T (common in linear algebra) */
static Mat4 mat4_mul_bt(Mat4 a, Mat4 b) {
    Mat4 bt = mat4_transpose(b);
    return mat4_mul(a, bt);
}

/* Commutator [A, B] = AB - BA */
static Mat4 mat4_commutator(Mat4 a, Mat4 b) {
    Mat4 ab = mat4_mul(a, b);
    Mat4 ba = mat4_mul(b, a);
    return mat4_sub(ab, ba);
}

/* Similarity transform: P^-1 * A * P (approximate P^-1 as P^T for orthogonal-ish matrices) */
static Mat4 mat4_similarity(Mat4 a, Mat4 p) {
    Mat4 pt = mat4_transpose(p);
    Mat4 tmp = mat4_mul(pt, a);
    return mat4_mul(tmp, p);
}

/* ---- QR-like decomposition via Gram-Schmidt (simplified) ---- */

typedef struct { double v[MAT_N]; } Vec4;

static double vec4_dot(Vec4 a, Vec4 b) {
    double s = 0;
    for (int i = 0; i < MAT_N; i++) s += a.v[i] * b.v[i];
    return s;
}

static Vec4 vec4_sub_scaled(Vec4 a, Vec4 b, double s) {
    Vec4 r;
    for (int i = 0; i < MAT_N; i++) r.v[i] = a.v[i] - s * b.v[i];
    return r;
}

static Vec4 vec4_normalize(Vec4 a) {
    double len_sq = vec4_dot(a, a);
    double inv = 1.0 / (len_sq + 1e-12);
    double guess = inv;
    guess = 0.5 * (guess + inv / guess);
    guess = 0.5 * (guess + inv / guess);
    Vec4 r;
    for (int i = 0; i < MAT_N; i++) r.v[i] = a.v[i] * guess;
    return r;
}

/* Extract column j as Vec4 */
static Vec4 mat4_col(Mat4 m, int j) {
    Vec4 r;
    for (int i = 0; i < MAT_N; i++) r.v[i] = m.m[i * MAT_N + j];
    return r;
}

/* Set column j from Vec4 */
static void mat4_set_col(Mat4 *m, int j, Vec4 v) {
    for (int i = 0; i < MAT_N; i++) m->m[i * MAT_N + j] = v.v[i];
}

/* Gram-Schmidt orthogonalization of columns */
static Mat4 mat4_gram_schmidt(Mat4 a) {
    Mat4 q = mat4_zero();
    Vec4 cols[MAT_N];

    for (int j = 0; j < MAT_N; j++) {
        cols[j] = mat4_col(a, j);
        for (int k = 0; k < j; k++) {
            double proj = vec4_dot(cols[j], cols[k]);
            cols[j] = vec4_sub_scaled(cols[j], cols[k], proj);
        }
        cols[j] = vec4_normalize(cols[j]);
        mat4_set_col(&q, j, cols[j]);
    }
    return q;
}

/* ---- Eigenvalue approximation via power iteration ---- */

static double mat4_dominant_eigenvalue(Mat4 a, int iters) {
    Vec4 v;
    for (int i = 0; i < MAT_N; i++) v.v[i] = 1.0;
    v = vec4_normalize(v);

    double eigenval = 0.0;
    for (int it = 0; it < iters; it++) {
        /* v = A * v */
        Vec4 new_v;
        for (int i = 0; i < MAT_N; i++) {
            double s = 0;
            for (int j = 0; j < MAT_N; j++)
                s += a.m[i * MAT_N + j] * v.v[j];
            new_v.v[i] = s;
        }
        eigenval = vec4_dot(new_v, v);
        v = vec4_normalize(new_v);
    }
    return eigenval;
}

static double workload(Mat4 *mats) {
    double total = 0.0;

    /* Chain multiplications: M0 * M1 * M2 * ... */
    Mat4 chain = mats[0];
    for (int i = 1; i < NUM_MATS; i++) {
        chain = mat4_mul(chain, mats[i]);
        /* Periodically rescale to prevent overflow */
        double norm = mat4_frobenius_sq(chain);
        if (norm > 1e10 || norm < 1e-10) {
            chain = mat4_scale(chain, 1.0 / (norm + 1e-12));
        }
    }
    total += mat4_trace(chain);
    total += mat4_frobenius_sq(chain);

    /* Matrix powers */
    for (int i = 0; i < 5; i++) {
        Mat4 p = mat4_pow(mats[i], 6 + i);
        total += mat4_trace(p);
    }

    /* Matrix exponential */
    for (int i = 0; i < 3; i++) {
        Mat4 scaled = mat4_scale(mats[i], 0.1);
        Mat4 exp = mat4_exp_approx(scaled, 8);
        total += mat4_trace(exp);
    }

    /* Transpose, Hadamard, and combined ops */
    for (int i = 0; i < NUM_MATS - 1; i++) {
        Mat4 h = mat4_hadamard(mats[i], mats[i + 1]);
        total += mat4_frobenius_sq(h);

        Mat4 abt = mat4_mul_bt(mats[i], mats[i + 1]);
        total += mat4_trace(abt);
    }

    /* Commutators */
    for (int i = 0; i < 5; i++) {
        Mat4 comm = mat4_commutator(mats[i], mats[i + 1]);
        total += mat4_frobenius_sq(comm);
    }

    /* Similarity transforms */
    for (int i = 0; i < 5; i++) {
        Mat4 sim = mat4_similarity(mats[i], mats[(i + 3) % NUM_MATS]);
        total += mat4_trace(sim);
    }

    /* Gram-Schmidt orthogonalization */
    for (int i = 0; i < 5; i++) {
        Mat4 q = mat4_gram_schmidt(mats[i]);
        total += mat4_frobenius_sq(q);
    }

    /* Power iteration for dominant eigenvalue */
    for (int i = 0; i < 5; i++) {
        double ev = mat4_dominant_eigenvalue(mats[i], 20);
        total += ev;
    }

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    Mat4 *mats = (Mat4 *)malloc(NUM_MATS * sizeof(Mat4));

    bench_lcg_seed(12345);
    for (int m = 0; m < NUM_MATS; m++) {
        for (int i = 0; i < MAT_N * MAT_N; i++) {
            mats[m].m[i] = (double)bench_lcg_rand() / 32768.0 - 0.5;
        }
    }

    volatile double sink;
    BENCH_TIME(niters, { sink = workload(mats); });

    free(mats);
    return 0;
}
