/*
 * Targets: instcombine (FP expressions), sroa (Vec3 structs), inline (deep
 * call chains), licm (scene constants), reassociate (dot products).
 *
 * Simple sphere raytracer: ray-sphere intersection, diffuse shading,
 * shadow rays, reflection, multiple spheres.
 */
#include "bench_timing.h"

#define IMG_W 32
#define IMG_H 32
#define NUM_SPHERES 8
#define MAX_DEPTH 3

typedef struct { double x, y, z; } Vec3;

static Vec3 v3(double x, double y, double z) {
    Vec3 r; r.x = x; r.y = y; r.z = z; return r;
}

static Vec3 v3_add(Vec3 a, Vec3 b) {
    return v3(a.x + b.x, a.y + b.y, a.z + b.z);
}

static Vec3 v3_sub(Vec3 a, Vec3 b) {
    return v3(a.x - b.x, a.y - b.y, a.z - b.z);
}

static Vec3 v3_scale(Vec3 a, double s) {
    return v3(a.x * s, a.y * s, a.z * s);
}

static double v3_dot(Vec3 a, Vec3 b) {
    return a.x * b.x + a.y * b.y + a.z * b.z;
}

static double v3_length_sq(Vec3 a) {
    return v3_dot(a, a);
}

static Vec3 v3_normalize(Vec3 a) {
    double len_sq = v3_length_sq(a);
    /* Newton's method for 1/sqrt — avoids -lm */
    double inv = 1.0 / (len_sq + 1e-12);
    double guess = inv;
    guess = 0.5 * (guess + inv / guess);
    guess = 0.5 * (guess + inv / guess);
    return v3_scale(a, guess);
}

static Vec3 v3_reflect(Vec3 dir, Vec3 normal) {
    double d = v3_dot(dir, normal);
    return v3_sub(dir, v3_scale(normal, 2.0 * d));
}

typedef struct {
    Vec3 center;
    double radius;
    Vec3 color;
    double reflectivity;
} Sphere;

typedef struct {
    Vec3 origin;
    Vec3 direction;
} Ray;

static Sphere spheres[NUM_SPHERES];
static Vec3 light_dir;

static void setup_scene(void) {
    bench_lcg_seed(42);
    for (int i = 0; i < NUM_SPHERES; i++) {
        spheres[i].center = v3(
            (double)(bench_lcg_rand() % 1000) / 100.0 - 5.0,
            (double)(bench_lcg_rand() % 1000) / 100.0 - 5.0,
            (double)(bench_lcg_rand() % 500) / 100.0 + 5.0
        );
        spheres[i].radius = 0.5 + (double)(bench_lcg_rand() % 300) / 200.0;
        spheres[i].color = v3(
            (double)(bench_lcg_rand() % 1000) / 1000.0,
            (double)(bench_lcg_rand() % 1000) / 1000.0,
            (double)(bench_lcg_rand() % 1000) / 1000.0
        );
        spheres[i].reflectivity = (double)(bench_lcg_rand() % 500) / 1000.0;
    }
    light_dir = v3_normalize(v3(1.0, -1.0, -0.5));
}

/* Returns distance to intersection, or -1 if no hit */
static double ray_sphere_intersect(Ray ray, Sphere *sphere) {
    Vec3 oc = v3_sub(ray.origin, sphere->center);
    double b = v3_dot(oc, ray.direction);
    double c = v3_dot(oc, oc) - sphere->radius * sphere->radius;
    double discriminant = b * b - c;
    if (discriminant < 0.0) return -1.0;
    /* Approximate sqrt via Newton's method */
    double sq = discriminant;
    double guess = sq * 0.5;
    if (guess > 0.0) {
        guess = 0.5 * (guess + sq / guess);
        guess = 0.5 * (guess + sq / guess);
        guess = 0.5 * (guess + sq / guess);
    }
    double t = -b - guess;
    if (t < 0.001) {
        t = -b + guess;
        if (t < 0.001) return -1.0;
    }
    return t;
}

/* Find closest sphere intersection */
static int find_closest(Ray ray, double *out_t) {
    int closest = -1;
    double min_t = 1e20;
    for (int i = 0; i < NUM_SPHERES; i++) {
        double t = ray_sphere_intersect(ray, &spheres[i]);
        if (t > 0.0 && t < min_t) {
            min_t = t;
            closest = i;
        }
    }
    *out_t = min_t;
    return closest;
}

/* Check if point is in shadow */
static int in_shadow(Vec3 point) {
    Ray shadow_ray;
    shadow_ray.origin = point;
    shadow_ray.direction = v3_scale(light_dir, -1.0);
    for (int i = 0; i < NUM_SPHERES; i++) {
        double t = ray_sphere_intersect(shadow_ray, &spheres[i]);
        if (t > 0.001) return 1;
    }
    return 0;
}

/* Shade a point on a sphere */
static Vec3 shade_point(Vec3 point, Vec3 normal, Sphere *sphere) {
    /* Ambient */
    Vec3 color = v3_scale(sphere->color, 0.1);

    /* Diffuse */
    double ndl = -v3_dot(normal, light_dir);
    if (ndl < 0.0) ndl = 0.0;
    if (!in_shadow(point)) {
        color = v3_add(color, v3_scale(sphere->color, ndl * 0.7));
    }

    /* Specular (Phong) */
    Vec3 reflected = v3_reflect(light_dir, normal);
    double spec = v3_dot(reflected, v3_normalize(v3_sub(v3(0, 0, 0), point)));
    if (spec > 0.0) {
        /* spec^8 approximation */
        spec *= spec; spec *= spec; spec *= spec;
        color = v3_add(color, v3(spec * 0.3, spec * 0.3, spec * 0.3));
    }

    return color;
}

/* Trace a ray recursively */
static Vec3 trace_ray(Ray ray, int depth) {
    if (depth >= MAX_DEPTH) return v3(0.0, 0.0, 0.0);

    double t;
    int hit = find_closest(ray, &t);
    if (hit < 0) {
        /* Sky gradient */
        double sky = 0.5 + 0.5 * ray.direction.y;
        return v3(0.4 * sky, 0.6 * sky, 0.8 + 0.2 * sky);
    }

    Vec3 point = v3_add(ray.origin, v3_scale(ray.direction, t));
    Vec3 normal = v3_normalize(v3_sub(point, spheres[hit].center));

    Vec3 color = shade_point(point, normal, &spheres[hit]);

    /* Reflection */
    if (spheres[hit].reflectivity > 0.01) {
        Ray refl_ray;
        refl_ray.origin = v3_add(point, v3_scale(normal, 0.001));
        refl_ray.direction = v3_reflect(ray.direction, normal);
        Vec3 refl_color = trace_ray(refl_ray, depth + 1);
        double r = spheres[hit].reflectivity;
        color = v3_add(v3_scale(color, 1.0 - r), v3_scale(refl_color, r));
    }

    return color;
}

/* Render a single pixel */
static Vec3 render_pixel(int px, int py) {
    /* Simple camera */
    double aspect = (double)IMG_W / IMG_H;
    double u = (2.0 * px / IMG_W - 1.0) * aspect;
    double v = 1.0 - 2.0 * py / IMG_H;

    Ray ray;
    ray.origin = v3(0.0, 0.0, 0.0);
    ray.direction = v3_normalize(v3(u, v, 1.5));

    return trace_ray(ray, 0);
}

/* Render with simple 2x2 supersampling */
static Vec3 render_pixel_aa(int px, int py) {
    Vec3 total = v3(0, 0, 0);
    for (int sy = 0; sy < 2; sy++) {
        for (int sx = 0; sx < 2; sx++) {
            double aspect = (double)IMG_W / IMG_H;
            double u = (2.0 * (px + sx * 0.5) / IMG_W - 1.0) * aspect;
            double v = 1.0 - 2.0 * (py + sy * 0.5) / IMG_H;
            Ray ray;
            ray.origin = v3(0, 0, 0);
            ray.direction = v3_normalize(v3(u, v, 1.5));
            Vec3 c = trace_ray(ray, 0);
            total = v3_add(total, c);
        }
    }
    return v3_scale(total, 0.25);
}

static double workload(void) {
    double total = 0.0;

    /* Render without AA */
    for (int y = 0; y < IMG_H; y++) {
        for (int x = 0; x < IMG_W; x++) {
            Vec3 c = render_pixel(x, y);
            total += c.x + c.y + c.z;
        }
    }

    /* Render with AA */
    for (int y = 0; y < IMG_H; y++) {
        for (int x = 0; x < IMG_W; x++) {
            Vec3 c = render_pixel_aa(x, y);
            total += c.x + c.y + c.z;
        }
    }

    return total;
}

int main(int argc, char **argv) {
    int niters = bench_parse_iters(argc, argv);
    setup_scene();

    volatile double sink;
    BENCH_TIME(niters, { sink = workload(); });
    return 0;
}
