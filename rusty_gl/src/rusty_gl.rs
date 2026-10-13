use std::error::Error;
use std::f32::consts::PI;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::str;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};
use std::sync::Mutex;
use std::thread;

use std::time::Instant;

use sdl2::{pixels::Color, rect::Point, render::Canvas, video::Window};

pub enum LightingMode {
    PerPixel,
    PerTriangle,
}

pub struct RustyGl {
    pub lighting_mode: LightingMode,
    backface_culling: AtomicBool,
    background_color: (AtomicU8, AtomicU8, AtomicU8, AtomicU8),
}
impl RustyGl {
    pub const fn init() -> RustyGl {
        RustyGl {
            lighting_mode: LightingMode::PerTriangle,
            background_color: (
                AtomicU8::new(0),
                AtomicU8::new(0),
                AtomicU8::new(0),
                AtomicU8::new(255),
            ),
            backface_culling: AtomicBool::new(true),
        }
    }

    pub fn get_backface_culling(&self) -> bool {
        self.backface_culling.load(Relaxed)
    }

    pub fn set_backface_culling(&self, val: bool) {
        self.backface_culling.store(val, Relaxed)
    }
}

pub static RUSTY_GL: RustyGl = RustyGl::init();

#[derive(Debug, Default, PartialEq, Clone, Copy, PartialOrd)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/*enum Axis {
    X(f32),
    Y(f32),
    Z(f32),
}*/

impl Point3 {
    /*fn find_point_on_line(line: [Point3; 2], known_axis: Axis) -> Result<Point3, ()> {
        let v = Point3 {x: line[0].x - line[1].x, y: line[0].y-line[1].y, z: line[0].z-line[1].z};
        match known_axis {
            Axis::X(x) => if v.z != 0.0 || v.y != 0.0 {

            },
            Axis::Y(y) => if v.x != 0.0 || v.z != 0.0 {

            },
            Axis::Z(z) => if v.x != 0.0 || v.y != 0.0 {
            },
        };
    }*/

    fn rotate_x(point_: Point3, origin_point: &Point3, degrees: f32) -> Point3 {
        let a = degrees * PI / 180.0;
        let mut point = point_;

        point.y -= origin_point.y;
        point.z -= origin_point.z;

        let y = point.y;
        let z = point.z;

        point.y = y * a.cos() - z * a.sin();
        point.z = y * a.sin() + z * a.cos();

        point.y += origin_point.y;
        point.z += origin_point.z;

        point
    }

    fn rotate_y(point_: Point3, origin_point: &Point3, degrees: f32) -> Point3 {
        let a = degrees * PI / 180.0;
        let mut point = point_;

        point.x -= origin_point.x;
        point.z -= origin_point.z;

        let x = point.x;
        let z = point.z;

        point.x = x * a.cos() + z * a.sin();
        point.z = -(x * a.sin()) + z * a.cos();

        point.x += origin_point.x;
        point.z += origin_point.z;
        point
    }

    fn rotate_z(point_: Point3, origin_point: &Point3, degrees: f32) -> Point3 {
        let a = degrees * PI / 180.0;
        let mut point = point_;

        point.x -= origin_point.x;
        point.y -= origin_point.y;

        let x = point.x;
        let y = point.y;

        point.x = x * a.cos() - y * a.sin();
        point.y = x * a.sin() + y * a.cos();

        point.x += origin_point.x;
        point.y += origin_point.y;
        point
    }

    fn project(point: Self, camera: &Camera) -> Option<Point3> {
        let width = camera.width as f32;
        let height = camera.height as f32;

        let perspective = [
            1.0 / ((width / height) * f32::tan(camera.fov_radians / 2.0)),
            1.0 / f32::tan(camera.fov_radians / 2.0),
            -((camera.far_clipping_plane + camera.near_clipping_plane)
                / (camera.far_clipping_plane - camera.near_clipping_plane)),
            -((2.0 * camera.far_clipping_plane * camera.near_clipping_plane)
                / (camera.far_clipping_plane - camera.near_clipping_plane)),
        ];

        let w = -point.z;

        if point.z < camera.near_clipping_plane || point.z > camera.far_clipping_plane {
            return None;
        }

        Some(Point3 {
            x: point.x * perspective[0] / w * camera.fov_radians * width + width / 2.0,
            y: point.y * perspective[1] / w * camera.fov_radians * height + height / 2.0,
            z: w,
        })
    }

    fn calculate_distance(point1: &Self, point2: &Self) -> f32 {
        f32::sqrt(
            (point1.x - point2.x).powi(2)
                + (point1.y - point2.y).powi(2)
                + (point1.z - point2.z).powi(2),
        )
    }

    fn get_dot_product(point1: &Point3, point2: &Point3) -> f32 {
        point1.x * point2.x + point1.y * point2.y + point1.z * point2.z
    }

    fn normalize_vector(point: Point3) -> Point3 {
        let m = (point.x.powi(2) + point.y.powi(2) + point.z.powi(2)).sqrt();

        Point3 {
            x: (point.x / m),
            y: (point.y / m),
            z: (point.z / m),
        }
    }
}

struct Texture {

}

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    point1: usize,
    point2: usize,
    point3: usize,

    origin_point: Point3,
    normal: Point3,

    color: Color,
    color_baked: Option<Color>,
}

impl Triangle {
    fn is_out_of_screen(points: [Point3; 3], width: u16, height: u16) -> bool {
        ((points[0].x > width as f32 || points[0].x < 0.0)
            && (points[1].x > width as f32 || points[1].x < 0.0)
            && (points[2].x > width as f32 || points[2].x < 0.0))
            || ((points[0].y > height as f32 || points[0].y < 0.0)
                && (points[1].y > height as f32 || points[1].y < 0.0)
                && (points[2].y > height as f32 || points[2].y < 0.0))
    }

    fn project(points: [Point3; 3], camera: &Camera) -> Option<TriangleProjected> {
        let p1 = match Point3::project(points[0], camera) {
            Some(x) => x,
            None => return None,
        };
        let p2 = match Point3::project(points[1], camera) {
            Some(x) => x,
            None => return None,
        };
        let p3 = match Point3::project(points[2], camera) {
            Some(x) => x,
            None => return None,
        };

        if Triangle::is_out_of_screen([p1, p2, p3], camera.width, camera.height) {
            return None;
        }

        Some(TriangleProjected {
            point1: p1,
            point2: p2,
            point3: p3,
            origin_point: None,
            normal: None,
            color: None,
        })
    }

    fn calc_normal(points: [Point3; 3]) -> Point3 {
        let vu = Point3 {
            x: points[1].x - points[0].x,
            y: points[1].y - points[0].y,
            z: points[1].z - points[0].z,
        };
        let vv = Point3 {
            x: points[2].x - points[0].x,
            y: points[2].y - points[0].y,
            z: points[2].z - points[0].z,
        };

        let a = Point3 {
            x: (vu.y * vv.z) - (vu.z * vv.y),
            y: (vu.z * vv.x) - (vu.x * vv.z),
            z: (vu.x * vv.y) - (vu.y * vv.x),
        };
        Point3::normalize_vector(a)
    }
}

#[allow(unused)]
pub struct TriangleProjected {
    pub point1: Point3,
    pub point2: Point3,
    pub point3: Point3,
    origin_point: Option<Point3>,
    normal: Option<Point3>,
    color: Option<Color>,
}

fn sort_far_to_near(triangles: &mut [Triangle], camera: &Camera) {
    triangles.sort_by_cached_key(|t| {
        std::cmp::Reverse(ordered_f32(
            Camera::perform_transformations(t.origin_point, camera).z,
        ))
    });
}

/// Maps an f32 to a u32 whose integer order matches `f32::total_cmp`.
fn ordered_f32(f: f32) -> u32 {
    let bits = f.to_bits();
    if bits >> 31 == 1 { !bits } else { bits | (1 << 31) }
}

pub trait MeshAndBuffer {
    fn sort_triangles_in_mesh_origin_point_z_distance_from_camera(
        &mut self,
        camera: &Camera,
    ) -> Result<(), ()>;
}

#[derive(Debug, Default, Clone)]
pub struct Mesh {
    pub name: String,

    pub points: Vec<Point3>,
    pub triangles: Vec<Triangle>,

    pub origin_point: Point3,
}

impl MeshAndBuffer for Mesh {
    fn sort_triangles_in_mesh_origin_point_z_distance_from_camera(
        &mut self,
        camera: &Camera,
    ) -> Result<(), ()> {
        if self.triangles.is_empty() {
            return Err(());
        }

        sort_far_to_near(&mut self.triangles, camera);
        Ok(())
    }
}

impl Mesh {
    pub fn init(name: String, points: &[Point3], triangles: &[Triangle]) -> Mesh {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;

        points.iter().for_each(|a| {
            x += a.x;
            y += a.y;
            z += a.z;
        });

        let p = Point3 {
            x: (x / points.len() as f32),
            y: (y / points.len() as f32),
            z: (z / points.len() as f32),
        };
        Mesh {
            name: (name),
            points: (points.to_vec()),
            triangles: (triangles.to_vec()),
            origin_point: (p),
        }
    }

    pub fn update_origin_point(&mut self) {
        let mut p = Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };

        self.points.iter().for_each(|a| {
            p.x += a.x;
            p.y += a.y;
            p.z += a.z;
        });

        p.x /= self.points.len() as f32;
        p.y /= self.points.len() as f32;
        p.z /= self.points.len() as f32;
        self.origin_point = p;
    }

    pub fn rotate_x(&mut self, mut degrees: f32) {
        degrees *= PI / 180.0;

        self.points.iter_mut().for_each(|a| {
            a.y -= self.origin_point.y;
            a.z -= self.origin_point.z;

            let y = a.y;
            let z = a.z;

            a.y = y * degrees.cos() - z * degrees.sin();
            a.z = y * degrees.sin() + z * degrees.cos();

            a.y += self.origin_point.y;
            a.z += self.origin_point.z;
        });

        self.update_normals();
    }

    pub fn rotate_y(&mut self, mut degrees: f32) {
        degrees *= PI / 180.0;

        self.points.iter_mut().for_each(|a| {
            a.x -= self.origin_point.x;
            a.z -= self.origin_point.z;

            let x = a.x;
            let z = a.z;

            a.x = x * degrees.cos() + z * degrees.sin();
            a.z = -x * degrees.sin() + z * degrees.cos();

            a.x += self.origin_point.x;
            a.z += self.origin_point.z;
        });
        self.update_normals();
    }

    pub fn rotate_z(&mut self, mut degrees: f32) {
        degrees *= PI / 180.0;

        self.points.iter_mut().for_each(|a| {
            a.x -= self.origin_point.x;
            a.y -= self.origin_point.y;

            let x = a.x;
            let y = a.y;

            a.x = x * degrees.cos() - y * degrees.sin();
            a.y = x * degrees.sin() + y * degrees.cos();

            a.x += self.origin_point.x;
            a.y += self.origin_point.y;
        });
        self.update_normals();
    }

    pub fn move_mesh(&mut self, x: f32, y: f32, z: f32) {
        self.points.iter_mut().for_each(|a| {
            a.x += x;
            a.y += y;
            a.z += z;
        });
        self.update_normals()
    }

    pub fn update_normals(&mut self) {
        self.triangles.iter_mut().for_each(|a| {
            let (p1, p2, p3) = (self.points[a.point1], self.points[a.point2], self.points[a.point3]);
            a.normal = Triangle::calc_normal([p1, p2, p3]);
            a.origin_point = Point3 {
                x: (p1.x + p2.x + p3.x) / 3.0,
                y: (p1.y + p2.y + p3.y) / 3.0,
                z: (p1.z + p2.z + p3.z) / 3.0,
            };
        });
    }

    pub fn set_color(&mut self, color: Color) {
        self.triangles.iter_mut().for_each(|a| a.color = color);
    }

    pub fn bake(&mut self, lights: &[Light]) {
        self.triangles.iter_mut().for_each(|a| {
            a.color_baked = Some(Light::apply_shading_multiple_lights(
                &a.color, &a.origin_point, &a.normal, lights,
            ))
        })
    }

    pub fn delete_bake(&mut self) {
        self.triangles.iter_mut().for_each(|a| a.color_baked = None);
    }
}

#[derive(Debug, Default, Clone)]
pub struct Buffer {
    amt_of_points: usize,

    points: Vec<Point3>,
    triangles: Vec<Triangle>,
}
impl Buffer {
    pub fn init() -> Buffer {
        Buffer {
            amt_of_points: 0,
            points: Vec::new(),
            triangles: Vec::new(),
        }
    }

    pub fn load_mesh(&mut self, mesh: &Mesh) {
        mesh.points.iter().for_each(|a| self.points.push(*a));
        mesh.triangles.iter().for_each(|a| {
            self.triangles.push(Triangle {
                point1: a.point1 + self.amt_of_points,
                point2: a.point2 + self.amt_of_points,
                point3: a.point3 + self.amt_of_points,
                origin_point: a.origin_point,
                normal: a.normal,
                color: a.color,
                color_baked: a.color_baked,
            })
        });

        self.amt_of_points += mesh.points.len();
    }

    pub fn load_meshes(&mut self, meshes: &[Mesh]) {
        meshes.iter().for_each(|a| self.load_mesh(a));
    }
}

impl MeshAndBuffer for Buffer {
    fn sort_triangles_in_mesh_origin_point_z_distance_from_camera(
        &mut self,
        camera: &Camera,
    ) -> Result<(), ()> {
        if self.triangles.is_empty() {
            return Err(());
        }

        sort_far_to_near(&mut self.triangles, camera);
        Ok(())
    }
}

pub trait Lighting {
    fn to_light(&self) -> Light;
}
impl Lighting for Point3 {
    fn to_light(&self) -> Light {
        Light::Point(PointLight {
            origin_point: Point3 {
                x: self.x,
                y: self.y,
                z: self.z,
            },
            brightness: 1.0,
            color: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            coefficients: (1.0, 1.0, 1.0),
        })
    }
}
impl Lighting for Camera {
    fn to_light(&self) -> Light {
        Light::Point(PointLight {
            origin_point: self.origin_point,
            brightness: 1.0,
            color: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            coefficients: (1.0, 1.0, 1.0),
        })
    }
}

pub trait ConvertColors {
    fn convert_255_1(color: &Color) -> (f32, f32, f32, f32) {
        (
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            color.a as f32 / 255.0,
        )
    }

    fn convert_1_255(color: &(f32, f32, f32, f32)) -> Color {
        Color {
            r: (color.0 * 255.0) as u8,
            g: (color.1 * 255.0) as u8,
            b: (color.2 * 255.0) as u8,
            a: (color.3 * 255.0) as u8,
        }
    }
}
impl ConvertColors for Color {}

#[derive(Debug, Clone, Copy)]
pub struct PointLight {
    // TODO
    pub coefficients: (f32, f32, f32),

    pub origin_point: Point3,
    pub brightness: f32,
    pub color: Color,
}
impl PointLight {
    /// Flat Lambert diffuse term for a surface at `position` facing `normal`
    /// (both in world space), as a linear RGB factor in 0..=1 per channel.
    /// `coefficients` are the attenuation terms (constant, linear, quadratic).
    fn diffuse(&self, position: &Point3, normal: &Point3) -> (f32, f32, f32) {
        let to_light = Point3 {
            x: self.origin_point.x - position.x,
            y: self.origin_point.y - position.y,
            z: self.origin_point.z - position.z,
        };
        let dist = (to_light.x.powi(2) + to_light.y.powi(2) + to_light.z.powi(2)).sqrt();
        if dist <= f32::EPSILON {
            return (0.0, 0.0, 0.0);
        }

        let n_dot_l = Point3::get_dot_product(normal, &to_light) / dist;
        if n_dot_l <= 0.0 {
            return (0.0, 0.0, 0.0);
        }

        let (c, l, q) = self.coefficients;
        let attenuation = 1.0 / (c + l * dist + q * dist * dist).max(f32::EPSILON);
        let k = n_dot_l * attenuation * self.brightness;
        let light = Color::convert_255_1(&self.color);
        (light.0 * k, light.1 * k, light.2 * k)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Light {
    Point(PointLight),
}

impl Light {
    /// Light that every surface receives, so faces turned away from all lights
    /// are dark but not pure black.
    const AMBIENT: f32 = 0.08;

    /// Shades a flat-colored triangle at `position` (world space) with `normal`
    /// (unit length, world space), summing the contribution of all lights.
    fn apply_shading_multiple_lights(
        color: &Color,
        position: &Point3,
        normal: &Point3,
        lights: &[Light],
    ) -> Color {
        let mut light = (Light::AMBIENT, Light::AMBIENT, Light::AMBIENT);
        lights.iter().for_each(|a| {
            let d = match a {
                Light::Point(x) => x.diffuse(position, normal),
            };
            light.0 += d.0;
            light.1 += d.1;
            light.2 += d.2;
        });

        let base = Color::convert_255_1(color);
        Color::convert_1_255(&(
            (base.0 * light.0).clamp(0.0, 1.0),
            (base.1 * light.1).clamp(0.0, 1.0),
            (base.2 * light.2).clamp(0.0, 1.0),
            base.3,
        ))
    }

    pub fn unwrap_point(&self) -> Option<PointLight> {
        match self {
            Light::Point(x) => Some(x.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub origin_point: Point3,

    pub width: u16,
    pub height: u16,

    fov_radians: f32,

    near_clipping_plane: f32,
    far_clipping_plane: f32,

    rotations: (f32, f32, f32),
}

impl Camera {
    pub fn init(
        origin_point: Point3,
        width: u16,
        height: u16,
        fov: f32,
        near_clipping_plane: f32,
        far_clipping_plane: f32,
    ) -> Camera {
        Camera {
            origin_point: (origin_point),
            width: (width),
            height: (height),
            fov_radians: (fov * PI / 180.0),
            near_clipping_plane: (near_clipping_plane),
            far_clipping_plane: (far_clipping_plane),
            rotations: (0.0, 0.0, 0.0),
        }
    }

    pub fn rotate_x(&mut self, degrees: f32) {
        self.rotations.0 += degrees;
        if self.rotations.0 == 360.0 {
            self.rotations.0 = 0.0
        } else if self.rotations.0 > 360.0 {
            self.rotations.0 -= 360.0
        }
    }

    pub fn rotate_y(&mut self, degrees: f32) {
        self.rotations.1 += degrees;
        if self.rotations.1 == 360.0 {
            self.rotations.1 = 0.0
        } else if self.rotations.1 > 360.0 {
            self.rotations.1 -= 360.0
        }
    }

    pub fn rotate_z(&mut self, degrees: f32) {
        self.rotations.2 += degrees;
        if self.rotations.2 == 360.0 {
            self.rotations.2 = 0.0
        } else if self.rotations.2 > 360.0 {
            self.rotations.2 -= 360.0
        }
    }

    fn perform_transformations(mut point: Point3, camera: &Camera) -> Point3 {
        point = Point3::rotate_x(point, &camera.origin_point, camera.rotations.0);
        point = Point3::rotate_y(point, &camera.origin_point, camera.rotations.1);
        point = Point3::rotate_z(point, &camera.origin_point, camera.rotations.2);
        point.x -= camera.origin_point.x;
        point.y -= camera.origin_point.y;
        point.z -= camera.origin_point.z;
        point
    }
}

#[derive(Debug)]
struct MyError;
impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "There is an error: {}", self)
    }
}
impl Error for MyError {}

pub struct MeshLoader(pub Vec<Mesh>);
impl MeshLoader {
    pub fn load_obj_file(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut normals: Vec<Point3> = Vec::new();
        let mut amt_of_points = 0;
        let mut counter = 0;

        for line in reader.lines() {
            let line_string = line?;
            let mut line_split = line_string.split(' ');
            let line_0 = match line_split.next() {
                Some(x) => x,
                None => return Err(Box::new(MyError)),
            }
            .to_string();
            let obj_len = self.0.len();

            if line_0 == "o" {
                if counter > 0 {
                    amt_of_points += self.0[obj_len - 1].points.len();
                }
                self.0
                    .push(Mesh::init(line_split.next().expect("cant read file, file is not in the correct format or has been tampered with").to_string(), &[], &[]));
                counter += 1;
            } else if line_0 == "vn" {
                let x = line_split.next().expect("cant read file, file is not in the correct format or has been tampered with").to_string();
                let y = line_split.next().expect("cant read file, file is not in the correct format or has been tampered with").to_string();
                let z = line_split.next().expect("cant read file, file is not in the correct format or has been tampered with").to_string();
                normals.push(Point3 {
                    x: x.parse()?,
                    y: y.parse()?,
                    z: z.parse()?,
                });
            } else if line_0 == "v" {
                let x = line_split.next().expect("cant read file, file is not in the correct format or has been tampered with").to_string();
                let y = line_split.next().expect("cant read file, file is not in the correct format or has been tampered with").to_string();
                let z = line_split.next().expect("cant read file, file is not in the correct format or has been tampered with").to_string();
                self.0[obj_len - 1].points.push(Point3 {
                    x: x.parse()?,
                    y: y.parse()?,
                    z: z.parse()?,
                });
            } else if line_0 == "f" {
                // face vertices can be "v", "v/vt", "v//vn" or "v/vt/vn", and a
                // face can have any number of vertices (quads, n-gons)
                let msg = "cant read file, file is not in the correct format or has been tampered with";
                let mut idx: Vec<usize> = Vec::new();
                let mut normal_idx: Option<usize> = None;
                for vertex in line_split.filter(|v| !v.is_empty()) {
                    let mut parts = vertex.split('/');
                    let v: usize = parts.next().expect(msg).parse()?;
                    idx.push(v - 1 - amt_of_points);
                    if let Some(vn) = parts.nth(1) {
                        if !vn.is_empty() {
                            normal_idx.get_or_insert(vn.parse()?);
                        }
                    }
                }
                if idx.len() < 3 {
                    return Err(Box::new(MyError));
                }
                let normal = normal_idx.map(|vn| normals[vn - 1]);
                let mesh = &mut self.0[obj_len - 1];

                // triangulate as a fan around the first vertex
                for i in 1..idx.len() - 1 {
                    let (p_1, p_2, p_3) = (idx[0], idx[i], idx[i + 1]);
                    let (a, b, c) = (mesh.points[p_1], mesh.points[p_2], mesh.points[p_3]);
                    mesh.triangles.push(Triangle {
                        point1: p_1,
                        point2: p_2,
                        point3: p_3,
                        origin_point: Point3 {
                            x: (a.x + b.x + c.x) / 3.0,
                            y: (a.y + b.y + c.y) / 3.0,
                            z: (a.z + b.z + c.z) / 3.0,
                        },
                        color: Color {
                            r: 180,
                            g: 180,
                            b: 180,
                            a: 255,
                        },
                        normal: normal.unwrap_or_else(|| Triangle::calc_normal([a, b, c])),
                        color_baked: None,
                    });
                }
            }
        }
        Ok(())
    }
}

fn backface_cull(first_point: &Point3, normal: &Point3) -> bool {
    normal.x * (first_point.x) + normal.y * (first_point.y) + normal.z * (first_point.z) >= 0.0
}

#[allow(unused_variables, unused_mut)]
pub trait SdlRendering {
    fn draw_lines_w(&mut self, buffer: &mut Buffer, camera: &Camera) -> Result<(), ()>;
    fn draw_triangle_single_threaded(&mut self, mut points_projected: [Point3; 3], color: Color) {}
}
impl SdlRendering for Canvas<Window> {
    fn draw_lines_w(&mut self, buffer: &mut Buffer, camera: &Camera) -> Result<(), ()> {
        let check = buffer.sort_triangles_in_mesh_origin_point_z_distance_from_camera(camera);
        if check == Err(()) {
            return Err(());
        }
        self.set_draw_color(Color::RGB(0, 0, 0));

        let check = buffer.triangles.iter().try_for_each(|a| {
            let p_trans1 = Camera::perform_transformations(buffer.points[a.point1], camera);
            let p_trans2 = Camera::perform_transformations(buffer.points[a.point2], camera);
            let p_trans3 = Camera::perform_transformations(buffer.points[a.point3], camera);
            let normal = Triangle::calc_normal([p_trans1, p_trans2, p_trans3]);

            if backface_cull(&p_trans1, &normal) {
                return Ok(());
            }
            let p1 = match Point3::project(buffer.points[a.point1], camera) {
                Some(x) => x,
                None => return Err(()),
            };
            let p2 = match Point3::project(buffer.points[a.point2], camera) {
                Some(x) => x,
                None => return Err(()),
            };
            let p3 = match Point3::project(buffer.points[a.point3], camera) {
                Some(x) => x,
                None => return Err(()),
            };

            let c1 = self.draw_line(
                Point::new(p1.x as i32, p1.y as i32),
                Point::new(p2.x as i32, p2.y as i32),
            );
            let c2 = self.draw_line(
                Point::new(p1.x as i32, p1.y as i32),
                Point::new(p3.x as i32, p3.y as i32),
            );
            let c3 = self.draw_line(
                Point::new(p2.x as i32, p2.y as i32),
                Point::new(p3.x as i32, p3.y as i32),
            );

            if c1 != Ok(()) || c2 != Ok(()) || c3 != Ok(()) {
                return Err(());
            }

            Ok(())
        });
        if check == Err(()) {
            return Err(());
        }
        Ok(())
    }

    fn draw_triangle_single_threaded(&mut self, mut points_projected: [Point3; 3], _color: Color) {
        // sort vertices by y: p[0] is the top, p[2] the bottom
        points_projected.sort_by(|a, b| a.y.total_cmp(&b.y));
        let [p0, p1, p2] = points_projected;
        if !(p0.y.is_finite() && p2.y.is_finite() && p0.x.is_finite() && p1.x.is_finite() && p2.x.is_finite()) {
            return;
        }

        // x on the edge a->b at scanline y (edge must not be horizontal)
        let edge_x = |a: &Point3, b: &Point3, y: f32| a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y);

        // clamp to the screen so triangles partially off-screen (or near the
        // camera, where projected coords explode) don't loop over billions of pixels
        let (w, h) = self.window().size();
        let (max_x, max_y) = (w as i32 - 1, h as i32 - 1);
        let y_start = (p0.y.ceil() as i32).max(0);
        let y_end = (p2.y.floor() as i32).min(max_y);

        for y in y_start..=y_end {
            let yf = y as f32;
            let long_x = edge_x(&p0, &p2, yf);
            // upper half uses edge 0-1, lower half uses edge 1-2
            let short_x = if yf < p1.y && p1.y > p0.y {
                edge_x(&p0, &p1, yf)
            } else if p2.y > p1.y {
                edge_x(&p1, &p2, yf)
            } else {
                long_x
            };

            let x0 = (f32::min(long_x, short_x).floor() as i32).max(0);
            let x1 = (f32::max(long_x, short_x).ceil() as i32).min(max_x);
            if x0 <= x1 {
                let _ = self.draw_line(Point::new(x0, y), Point::new(x1, y));
            }
        }
    }
}

pub trait SdlWrapper {
    fn draw_triangles_single_threaded(
        &mut self,
        buffer: Buffer,
        camera: &mut Camera,
        lights: &Vec<Light>,
    ) -> Result<(), ()>;

    fn draw_triangles_multi_threaded(
        &mut self,
        buffer: Buffer,
        camera: &mut Camera,
        lights: &Vec<Light>,
        amt_threads: usize
    ) -> Result<(), ()>;
}

impl SdlWrapper for Canvas<Window> {
    fn draw_triangles_multi_threaded(
            &mut self,
            mut buffer: Buffer,
            camera: &mut Camera,
            lights: &Vec<Light>,
            amt_threads: usize
        ) -> Result<(), ()> {
            let check = buffer.sort_triangles_in_mesh_origin_point_z_distance_from_camera(camera);
            if check == Err(()) {
                return Err(());
            }

        let triangles_mutex = Mutex::new(
            buffer
                .triangles
                .chunks((buffer.triangles.len() / amt_threads.max(1)).max(1)),
        );
        let tri_vec: Mutex<Vec<(Point3, Point3, Point3, Color)>> = Mutex::new(vec![]);
        let points = buffer.points;

        thread::scope(|scope| {
            for _ in 0..amt_threads {
                scope.spawn(|| {
                    let mut vec: Vec<(Point3, Point3, Point3, Color)> = vec![];
                    let mut lock;
                    while let Some(chunk) = {
                        lock = triangles_mutex.lock().unwrap();
                        lock.next()
                    } {
                        drop(lock);
                        chunk.iter().for_each(|tri| {
                            let p_trans1 =
                                Camera::perform_transformations(points[tri.point1], camera);
                            let p_trans2 =
                                Camera::perform_transformations(points[tri.point2], camera);
                            let p_trans3 =
                                Camera::perform_transformations(points[tri.point3], camera);
                            let normal = Triangle::calc_normal([p_trans1, p_trans2, p_trans3]);

                            if backface_cull(&p_trans1, &normal) {
                                return;
                            }
                            let triangle =
                                match Triangle::project([p_trans1, p_trans2, p_trans3], camera) {
                                    Some(x) => x,
                                    None => return,
                                };

                            if !lights.is_empty() && tri.color_baked.is_none() {
                                vec.push((
                                    triangle.point1,
                                    triangle.point2,
                                    triangle.point3,
                                    Light::apply_shading_multiple_lights(
                                        &tri.color, &tri.origin_point, &tri.normal, lights,
                                    ),
                                ))
                            } else if tri.color_baked.is_some() {
                                vec.push((
                                    triangle.point1,
                                    triangle.point2,
                                    triangle.point3,
                                    tri.color_baked.unwrap(),
                                ))
                            } else {
                                vec.push((
                                    triangle.point1,
                                    triangle.point2,
                                    triangle.point3,
                                    tri.color,
                                ))
                            }
                        });
                    }
                    tri_vec.lock().unwrap().append(&mut vec);
                });
            }
        });

        tri_vec.lock().unwrap().iter().for_each(|a| {
            self.set_draw_color(a.3);
            self.draw_triangle_single_threaded([a.0, a.1, a.2], a.3)
        });
        Ok(())
    }

    fn draw_triangles_single_threaded(
        &mut self,
        mut buffer: Buffer,
        camera: &mut Camera,
        lights: &Vec<Light>,
    ) -> Result<(), ()> {
        let start: Instant = Instant::now();
        let check = buffer.sort_triangles_in_mesh_origin_point_z_distance_from_camera(camera);
        if check == Err(()) {
            return Err(());
        }

        buffer.triangles.iter().for_each(|tri| {
            let p_trans1 = Camera::perform_transformations(buffer.points[tri.point1], camera);
            let p_trans2 = Camera::perform_transformations(buffer.points[tri.point2], camera);
            let p_trans3 = Camera::perform_transformations(buffer.points[tri.point3], camera);
            let normal = Triangle::calc_normal([p_trans1, p_trans2, p_trans3]);

            if RUSTY_GL.get_backface_culling() == true && backface_cull(&p_trans1, &normal) {
                return;
            }

            let triangle = match Triangle::project([p_trans1, p_trans2, p_trans3], camera) {
                Some(x) => x,
                None => return,
            };

            if !lights.is_empty() && tri.color_baked.is_none() {
                let color = Light::apply_shading_multiple_lights(
                    &tri.color,
                    &tri.origin_point,
                    &tri.normal,
                    lights,
                ); 
                self.set_draw_color(color);
                self.draw_triangle_single_threaded([
                    triangle.point1,
                    triangle.point2,
                    triangle.point3,
                ], color);
            } else if tri.color_baked.is_some() {
                self.set_draw_color(tri.color_baked.unwrap());
                self.draw_triangle_single_threaded([
                    triangle.point1,
                    triangle.point2,
                    triangle.point3,
                ], tri.color_baked.unwrap());
            } else {
                self.set_draw_color(tri.color);
                self.draw_triangle_single_threaded([
                    triangle.point1,
                    triangle.point2,
                    triangle.point3,],
                    tri.color
                );
            }
        });
        Ok(())
    }
}
