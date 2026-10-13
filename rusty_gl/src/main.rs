use sdl2::event::Event;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::video::Window;
use sdl2::{self, keyboard};
use rusty_gl::rusty_gl;
use crate::rusty_gl::*;

/*
   TODO:
   add textures
   improve lightings
   add depth buffer
   fix clippings
   implement shaders
   add per pixel lighting
   add normal support in obj parser
   add directional lighting
*/
/*
   axis relative to blender:
   -y is z,

*/ 

fn main() {
    let mut lights = vec![Light::Point(PointLight {
        origin_point: Point3 {
            x: 1.0,
            y: 0.0,
            z: 1.0,
        },
        brightness: 4f32,
        color: Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        coefficients: (1.0, 1.0, 1.0),
    })];
    lights.push(Light::Point(PointLight {
        origin_point: Point3 {
            x: -1.0,
            y: 0.0,
            z: 1.0,
        },
        brightness: 4f32,
        color: Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        coefficients: (1.0, 1.0, 1.0),
    }));
    lights.push(Light::Point(PointLight {
        origin_point: Point3 {
            x: 0.5,
            y: 0.5,
            z: 7.5,
        },
        brightness: 3f32,
        color: Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        coefficients: (1.0, 1.0, 1.0),
    }));

    let mut objs: MeshLoader = MeshLoader(Vec::new());
    objs.load_obj_file("monkey.obj").unwrap();
    objs.0.iter_mut().for_each(|mesh| mesh.update_origin_point());
    println!("{:#?}", objs.0[0].origin_point);
    let mut camera = Camera::init(
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        1000,
        1000,
        80.0,
        1.0,
        100.0,
    );
    let mut points: Buffer;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window: Window = video_subsystem
        .window("rusty_gl", 3000, 3000)
        .resizable()
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        objs.0.iter_mut().for_each(|mesh| mesh.rotate_y(0.1));
        points = Buffer::init();
        points.load_meshes(&objs.0);

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,

                Event::KeyDown {
                    timestamp: _,
                    window_id: _,
                    keycode,
                    scancode: _,
                    keymod: _,
                    repeat: _,
                } => {
                    if keycode == Some(keyboard::Keycode::A) {
                        camera.origin_point.x += 0.08;
                    } else if keycode == Some(keyboard::Keycode::D) {
                        camera.origin_point.x -= 0.08;
                    } else if keycode == Some(keyboard::Keycode::W) {
                        camera.origin_point.z += 0.08;
                    } else if keycode == Some(keyboard::Keycode::S) {
                        camera.origin_point.z -= 0.08;
                    } else if keycode == Some(keyboard::Keycode::Space) {
                        camera.origin_point.y += 0.08;
                    } else if keycode == Some(keyboard::Keycode::CapsLock) {
                        camera.origin_point.y -= 0.08;
                    } else if keycode == Some(keyboard::Keycode::E) {
                        camera.rotate_y(1.5);
                    } else if keycode == Some(keyboard::Keycode::Q) {
                        camera.rotate_y(-1.5);
                    } else if keycode == Some(keyboard::Keycode::F) {
                        camera.rotate_x(1.5);
                    } else if keycode == Some(keyboard::Keycode::C) {
                        camera.rotate_x(-1.5);
                    }
                }
                _ => {}
            }
        }
        
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.draw_triangles_multi_threaded(points, &mut camera, &lights, 1).unwrap();
        canvas.present();

    }
}
