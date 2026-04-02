#include "bench_timing.h"

/*
 * Particle physics simulation: high fcmp count + many function calls.
 * Lots of float comparisons for collision detection and boundary checks.
 */

#define NUM_PARTICLES 80
#define NUM_STEPS 20
#define BOUNDS 1000

/* Particle state */
typedef struct {
    double x, y;
    double vx, vy;
    double mass;
    double radius;
} Particle;

static double fsqrt_approx(double x) {
    /* Newton's method, 4 iterations — avoids -lm */
    if (x <= 0.0) return 0.0;
    double guess = x * 0.5;
    guess = 0.5 * (guess + x / guess);
    guess = 0.5 * (guess + x / guess);
    guess = 0.5 * (guess + x / guess);
    guess = 0.5 * (guess + x / guess);
    return guess;
}

static double fabs_d(double x) {
    return x < 0.0 ? -x : x;
}

static double fmin_d(double a, double b) {
    return a < b ? a : b;
}

static double fmax_d(double a, double b) {
    return a > b ? a : b;
}

static double distance(double x1, double y1, double x2, double y2) {
    double dx = x2 - x1;
    double dy = y2 - y1;
    return fsqrt_approx(dx * dx + dy * dy);
}

static int check_collision(const Particle *a, const Particle *b) {
    double dist = distance(a->x, a->y, b->x, b->y);
    return dist < (a->radius + b->radius);
}

static void resolve_collision(Particle *a, Particle *b) {
    double dx = b->x - a->x;
    double dy = b->y - a->y;
    double dist = fsqrt_approx(dx * dx + dy * dy);
    if (dist < 0.001) dist = 0.001;

    /* Normalize */
    double nx = dx / dist;
    double ny = dy / dist;

    /* Relative velocity along normal */
    double dvx = a->vx - b->vx;
    double dvy = a->vy - b->vy;
    double dvn = dvx * nx + dvy * ny;

    /* Don't resolve if separating */
    if (dvn < 0.0) return;

    /* Elastic collision */
    double total_mass = a->mass + b->mass;
    double impulse = 2.0 * dvn / total_mass;

    a->vx -= impulse * b->mass * nx;
    a->vy -= impulse * b->mass * ny;
    b->vx += impulse * a->mass * nx;
    b->vy += impulse * a->mass * ny;
}

static void apply_gravity(Particle *particles, int n) {
    int i, j;
    for (i = 0; i < n; i++) {
        for (j = i + 1; j < n; j++) {
            double dx = particles[j].x - particles[i].x;
            double dy = particles[j].y - particles[i].y;
            double dist_sq = dx * dx + dy * dy;
            if (dist_sq < 1.0) dist_sq = 1.0;
            double dist = fsqrt_approx(dist_sq);
            double force = particles[i].mass * particles[j].mass / dist_sq;
            double fx = force * dx / dist;
            double fy = force * dy / dist;

            particles[i].vx += fx / particles[i].mass;
            particles[i].vy += fy / particles[i].mass;
            particles[j].vx -= fx / particles[j].mass;
            particles[j].vy -= fy / particles[j].mass;
        }
    }
}

static void integrate(Particle *particles, int n, double dt) {
    int i;
    for (i = 0; i < n; i++) {
        particles[i].x += particles[i].vx * dt;
        particles[i].y += particles[i].vy * dt;

        /* Boundary reflection */
        if (particles[i].x < particles[i].radius) {
            particles[i].x = particles[i].radius;
            particles[i].vx = fabs_d(particles[i].vx);
        }
        if (particles[i].x > BOUNDS - particles[i].radius) {
            particles[i].x = BOUNDS - particles[i].radius;
            particles[i].vx = -fabs_d(particles[i].vx);
        }
        if (particles[i].y < particles[i].radius) {
            particles[i].y = particles[i].radius;
            particles[i].vy = fabs_d(particles[i].vy);
        }
        if (particles[i].y > BOUNDS - particles[i].radius) {
            particles[i].y = BOUNDS - particles[i].radius;
            particles[i].vy = -fabs_d(particles[i].vy);
        }

        /* Damping */
        particles[i].vx *= 0.999;
        particles[i].vy *= 0.999;

        /* Speed limit */
        double speed_sq = particles[i].vx * particles[i].vx +
                          particles[i].vy * particles[i].vy;
        if (speed_sq > 10000.0) {
            double speed = fsqrt_approx(speed_sq);
            particles[i].vx = particles[i].vx * 100.0 / speed;
            particles[i].vy = particles[i].vy * 100.0 / speed;
        }
    }
}

static double compute_energy(const Particle *particles, int n) {
    double energy = 0.0;
    int i;
    for (i = 0; i < n; i++) {
        energy += 0.5 * particles[i].mass *
                  (particles[i].vx * particles[i].vx +
                   particles[i].vy * particles[i].vy);
    }
    return energy;
}

static long long workload(Particle *particles, Particle *backup) {
    int step, i, j;
    double total_energy = 0.0;

    for (step = 0; step < NUM_STEPS; step++) {
        /* Save state for energy check */
        for (i = 0; i < NUM_PARTICLES; i++) backup[i] = particles[i];

        apply_gravity(particles, NUM_PARTICLES);

        /* Collision detection and response */
        for (i = 0; i < NUM_PARTICLES; i++) {
            for (j = i + 1; j < NUM_PARTICLES; j++) {
                if (check_collision(&particles[i], &particles[j])) {
                    resolve_collision(&particles[i], &particles[j]);
                }
            }
        }

        integrate(particles, NUM_PARTICLES, 0.1);
        total_energy += compute_energy(particles, NUM_PARTICLES);
    }

    return (long long)total_energy;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);

    Particle particles[NUM_PARTICLES];
    Particle backup[NUM_PARTICLES];
    int i;

    bench_lcg_seed(42);
    for (i = 0; i < NUM_PARTICLES; i++) {
        particles[i].x = (double)(bench_lcg_rand() % BOUNDS);
        particles[i].y = (double)(bench_lcg_rand() % BOUNDS);
        particles[i].vx = (double)((int)(bench_lcg_rand() % 200) - 100) * 0.1;
        particles[i].vy = (double)((int)(bench_lcg_rand() % 200) - 100) * 0.1;
        particles[i].mass = (double)(bench_lcg_rand() % 10 + 1);
        particles[i].radius = (double)(bench_lcg_rand() % 5 + 3);
    }

    volatile long long sink;
    BENCH_TIME(niters, { sink = workload(particles, backup); });

    return 0;
}
