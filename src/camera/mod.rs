use random::Random;
use vec3::{Point3, Vec3};

use crate::{color::Color, object::hittable::HitRecord, ray::Ray, scene::Scene};

pub struct Camera {
    pub origin: Point3<f64>,
    pub direction: Vec3<f64>,
    pub focal_length: f64,
    pub image_width: u32,
    pub image_height: u32,
    pub viewport_width: f64,
}

pub struct RenderOptions {
    pub samples_per_pixel: usize,
    pub bounce_limit: u8,
}

impl Camera {
    fn calc_viewport_upper_left(&self) -> Point3<f64> {
        self.origin - self.calc_viewport_u().scalar(0.5) - self.calc_viewport_v().scalar(0.5)
            + self.direction.normalized().scalar(self.focal_length)
    }

    fn calc_viewport_u(&self) -> Vec3<f64> {
        let x = 1.0;
        let y = 0.0;
        let z = -1.0 * (x * self.direction.x) / self.direction.z;

        Vec3::new(x, y, z).normalized().scalar(self.viewport_width)
    }

    fn calc_viewport_v(&self) -> Vec3<f64> {
        let viewport_height =
            (self.viewport_width / self.image_width as f64) * self.image_height as f64;
        self.calc_viewport_u()
            .cross(self.direction)
            .normalized()
            .scalar(-1.0 * viewport_height)
    }

    fn calc_pixel_delta_u(&self) -> Vec3<f64> {
        self.calc_viewport_u().scalar(1.0 / self.image_width as f64)
    }

    fn calc_pixel_delta_v(&self) -> Vec3<f64> {
        self.calc_viewport_v()
            .scalar(1.0 / self.image_height as f64)
    }

    pub fn render(
        &self,
        scene: &Scene,
        options: &RenderOptions,
        mut cb: impl FnMut((u32, u32), Color) -> (),
    ) {
        if self.direction == Vec3::new(0.0, 0.0, 0.0) {
            panic!("Camera direction cannot be zero vector.");
        }

        let viewport_upper_left = self.calc_viewport_upper_left();
        let pixel_delta_u = self.calc_pixel_delta_u();
        let pixel_delta_v = self.calc_pixel_delta_v();

        let mut rng = Random::new(256);
        let samples_per_pixel_f64 = options.samples_per_pixel as f64;

        for y in 0..self.image_height {
            for x in 0..self.image_width {
                let mut pixel_color = Color::black();

                for _ in 0..options.samples_per_pixel {
                    // Antialiasing
                    let px = -0.5 + rng.next();
                    let py = -0.5 + rng.next();

                    let pixel_center = viewport_upper_left
                        + pixel_delta_u.scalar(x as f64 + px)
                        + pixel_delta_v.scalar(y as f64 + py)
                        + (pixel_delta_u + pixel_delta_v).scalar(0.5);

                    let ray_direction = pixel_center - self.origin;

                    let r = Ray::new(self.origin, ray_direction);
                    pixel_color =
                        pixel_color + Self::ray_color(scene, r, options.bounce_limit, &mut rng);
                }

                let color =
                    (pixel_color * Color::grey(1.0 / samples_per_pixel_f64)).linear_to_gamma();
                cb((x, y), color);
            }
        }
    }

    /// Calculate the color of a ray.
    fn ray_color(scene: &Scene, ray: Ray, depth: u8, rng: &mut Random) -> Color {
        if depth == 0 {
            return Color::black();
        }

        let mut current_hit_info: Option<HitRecord> = None;

        // First loop thorugh all objects to find the closest hit.
        for object in &scene.objects {
            if let Some(hit_info) = object.hit(&ray, 0.001, f64::INFINITY) {
                if let Some(c_hit_info) = current_hit_info {
                    if hit_info.t < c_hit_info.t {
                        current_hit_info = Some(hit_info);
                    }
                } else {
                    current_hit_info = Some(hit_info);
                }
            }
        }

        // If we have a hit, calculate the color.
        if let Some(hit_info) = current_hit_info {
            if let Some((color, ray)) = hit_info.material.scatter(&ray, &hit_info, rng) {
                let light_strength = hit_info.normal.dot(ray.direction).abs();

                let background = color
                    * Self::ray_color(scene, ray, depth - 1, rng)
                    * Color::grey(light_strength * 0.5);

                if let Some(emission) = hit_info.material.emission {
                    return emission + (Color::white() - emission) * background;
                }

                return background;
            } else {
                return Color::black();
            }
        }

        let unit_direction = ray.direction.normalized();
        let t = 0.5 * (unit_direction.y + 1.0);

        scene.sky_color.lerp(t)
    }
}