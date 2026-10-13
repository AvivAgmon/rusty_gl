# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`rusty_gl` is an early-stage CPU software 3D renderer in Rust. It rasterizes triangle meshes (loaded from `.obj` files) and presents them through SDL2's 2D canvas. The README notes it is not yet meant for production use.

The Cargo package lives in the `rusty_gl/` subdirectory (not the repo root), so run cargo commands from there.

## Commands

Run from `rusty_gl/`:

- Build: `cargo build` (release profile uses LTO, `panic = "abort"`, strip)
- Run the demo: `cargo run --release` (must run from `rusty_gl/` since `main.rs` loads `cone.obj` by relative path)
- There are no tests or lint configs in the repo; `cargo clippy` / `cargo test` work with defaults.

System dependency: the native SDL2 library must be installed (the `sdl2` crate is only a binding).

## Architecture

- `src/lib.rs` exposes a single module, `rusty_gl`. The whole renderer lives in `src/rusty_gl.rs` (~1100 lines). `src/main.rs` is both the demo and the only usage example: it builds lights and a `Camera`, loads a mesh with `MeshLoader::load_obj_file`, and runs an SDL event loop (WASD/Space/CapsLock move the camera, Q/E and F/C rotate it).
- Render pipeline in `rusty_gl.rs`:
  1. `MeshLoader` parses `.obj` into `Mesh` (points + `Triangle`s). Normals from the file are not yet used; they are computed via `Triangle::calc_normal`.
  2. `Buffer` collects triangles from meshes each frame (`load_meshes`) and sorts them by distance from the camera (painter's algorithm via the `MeshAndBuffer` trait; there is no depth buffer).
  3. `Triangle::project` / `Point3::project` apply camera transforms (`Camera::perform_transformations`) and perspective projection to screen space, with backface culling toggled through `RustyGl`.
  4. Lighting: `Light::Point(PointLight)` shading is applied per triangle (flat, not per pixel). `Mesh::bake` can precompute lighting.
  5. Rasterization is implemented as extension traits on `sdl2::render::Canvas<Window>`: `SdlRendering` (wireframe `draw_lines_w`) and `SdlWrapper` (`draw_triangles_single_threaded` / `draw_triangles_multi_threaded`, the latter taking a thread count as its last argument).
- Coordinate note from `main.rs`: axes are relative to Blender's export, with Blender's -Y mapping to Z.
- Known unfinished work (see TODO comment in `main.rs`): textures, depth buffer, clipping fixes, shaders, per-pixel and directional lighting, OBJ normals.
