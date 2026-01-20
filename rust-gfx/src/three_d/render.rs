use super::math::{Mat4, Vec2, Vec3, Vec4};
use super::model::{Material, Mesh, Scene, Texture};
use crate::color::Rgba8888;
use crate::Rasterizer;
use alloc::vec::Vec;
use core::cmp::Ordering;
use micromath::F32Ext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadingMode {
    Wireframe,
    Lit,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub mode: ShadingMode,
    pub overlay_wireframe: bool,
    pub wireframe_color: Rgba8888,
    pub light_direction: Vec3,
    pub ambient_intensity: f32,
    pub diffuse_intensity: f32,
    pub specular_intensity: f32,
    pub shininess: f32,
    pub enable_backface_culling: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            mode: ShadingMode::Lit,
            overlay_wireframe: false,
            wireframe_color: Rgba8888::rgb(255, 255, 255),
            light_direction: Vec3::new(0.0, 0.0, -1.0),
            ambient_intensity: 0.2,
            diffuse_intensity: 0.75,
            specular_intensity: 0.1,
            shininess: 16.0,
            enable_backface_culling: true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub position: Vec3,
    pub view: Mat4,
    pub projection: Mat4,
}

impl Camera {
    pub fn look_at_perspective(
        eye: Vec3,
        target: Vec3,
        up: Vec3,
        fov_y_radians: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position: eye,
            view: Mat4::look_at(eye, target, up),
            projection: Mat4::perspective(fov_y_radians, aspect, near, far),
        }
    }
}

pub fn render_scene<R>(rasterizer: &mut R, scene: &Scene, camera: &Camera, options: &RenderOptions)
where
    R: Rasterizer,
{
    if scene.meshes.is_empty() || scene.nodes.is_empty() {
        return;
    }

    let width = rasterizer.width() as f32;
    let height = rasterizer.height() as f32;
    if width <= 1.0 || height <= 1.0 {
        return;
    }

    let light_dir = options.light_direction.normalize();
    let ambient = options.ambient_intensity.max(0.0);
    let diffuse = options.diffuse_intensity.max(0.0);
    let specular = options.specular_intensity.max(0.0);

    for node in scene.nodes.iter() {
        if let Some(mesh) = scene.meshes.get(node.mesh) {
            let material = mesh.material.and_then(|idx| scene.materials.get(idx));
            render_mesh(
                rasterizer,
                mesh,
                material,
                &scene.textures,
                &node.transform,
                camera,
                options,
                width,
                height,
                light_dir,
                ambient,
                diffuse,
                specular,
            );
        }
    }
}

struct PreparedVertex {
    clip: Vec4,
    screen: Vec3,
    world: Vec3,
    view: Vec3,
    normal: Vec3,
    uv: Vec2,
    inv_w: f32,
}

fn render_mesh<R>(
    rasterizer: &mut R,
    mesh: &Mesh,
    material: Option<&Material>,
    textures: &[Texture],
    model_matrix: &Mat4,
    camera: &Camera,
    options: &RenderOptions,
    width: f32,
    height: f32,
    light_dir: Vec3,
    ambient: f32,
    diffuse: f32,
    specular: f32,
)
where
    R: Rasterizer,
{
    if mesh.vertices.is_empty() || mesh.indices.len() < 3 {
        return;
    }

    let model_view = camera.view.mul_mat4(model_matrix);
    let mvp = camera.projection.mul_mat4(&model_view);
    let normal_matrix = compute_normal_matrix(model_matrix);

    let mut prepared: Vec<PreparedVertex> = Vec::with_capacity(mesh.vertices.len());
    for v in mesh.vertices.iter() {
        let world_pos = model_matrix.transform_point(v.position);
        let view_pos = model_view.transform_point(v.position);
        let clip = mvp.mul_vec4(Vec4::new(v.position.x, v.position.y, v.position.z, 1.0));
        if clip.w.abs() < 1.0e-5 {
            prepared.push(PreparedVertex {
                clip,
                screen: Vec3::new(0.0, 0.0, 0.0),
                world: world_pos,
                view: view_pos,
                normal: Vec3::new(0.0, 0.0, 1.0),
                uv: v.uv,
                inv_w: 0.0,
            });
            continue;
        }
        let inv_w = 1.0 / clip.w;
        let ndc_x = clip.x * inv_w;
        let ndc_y = clip.y * inv_w;
        let ndc_z = clip.z * inv_w;
        let sx = (ndc_x * 0.5 + 0.5) * (width - 1.0);
        let sy = (1.0 - (ndc_y * 0.5 + 0.5)) * (height - 1.0);
        let normal_world = transform_normal(&normal_matrix, v.normal).normalize();

        prepared.push(PreparedVertex {
            clip,
            screen: Vec3::new(sx, sy, ndc_z),
            world: world_pos,
            view: view_pos,
            normal: normal_world,
            uv: v.uv,
            inv_w,
        });
    }

    let mut triangles: Vec<(usize, usize, usize, f32)> = Vec::with_capacity(mesh.indices.len() / 3);
    for tri in mesh.indices.chunks_exact(3) {
        let a = tri[0] as usize;
        let b = tri[1] as usize;
        let c = tri[2] as usize;
        if a >= prepared.len() || b >= prepared.len() || c >= prepared.len() {
            continue;
        }
        if prepared[a].inv_w == 0.0 || prepared[b].inv_w == 0.0 || prepared[c].inv_w == 0.0 {
            continue;
        }
        let depth = (prepared[a].screen.z + prepared[b].screen.z + prepared[c].screen.z) / 3.0;
        triangles.push((a, b, c, depth));
    }

    triangles.sort_unstable_by(|lhs, rhs| rhs.3.partial_cmp(&lhs.3).unwrap_or(Ordering::Equal));

    for (ia, ib, ic, _) in triangles.into_iter() {
        let v0 = &prepared[ia];
        let v1 = &prepared[ib];
        let v2 = &prepared[ic];

        if options.enable_backface_culling {
            let edge1 = v1.world - v0.world;
            let edge2 = v2.world - v0.world;
            let face_normal = edge1.cross(edge2);
            let view_dir = v0.world - camera.position;
            if face_normal.dot(view_dir) >= 0.0 {
                continue;
            }
        }

        match options.mode {
            ShadingMode::Wireframe => {
                draw_wireframe_triangle(rasterizer, v0, v1, v2, options.wireframe_color);
            }
            ShadingMode::Lit => {
                rasterize_triangle(
                    rasterizer, v0, v1, v2, material, textures, camera, options, light_dir,
                    ambient, diffuse, specular,
                );
                if options.overlay_wireframe {
                    draw_wireframe_triangle(rasterizer, v0, v1, v2, options.wireframe_color);
                }
            }
        }
    }
}

fn rasterize_triangle<R>(
    rasterizer: &mut R,
    v0: &PreparedVertex,
    v1: &PreparedVertex,
    v2: &PreparedVertex,
    material: Option<&Material>,
    textures: &[Texture],
    camera: &Camera,
    options: &RenderOptions,
    light_dir: Vec3,
    ambient: f32,
    diffuse: f32,
    specular: f32,
)
where
    R: Rasterizer,
{
    let width = rasterizer.width() as i32;
    let height = rasterizer.height() as i32;

    let p0 = &v0.screen;
    let p1 = &v1.screen;
    let p2 = &v2.screen;

    let mut area = edge_fn(p0, p1, p2.x, p2.y);
    if area.abs() < 1.0e-5 {
        return;
    }
    let orient = if area < 0.0 { -1.0 } else { 1.0 };
    area *= orient;
    let inv_area = 1.0 / area;

    let step_x0 = (p2.y - p1.y) * orient;
    let step_x1 = (p0.y - p2.y) * orient;
    let step_x2 = (p1.y - p0.y) * orient;

    let min_x = p0.x.min(p1.x.min(p2.x)).floor() as i32;
    let max_x = p0.x.max(p1.x.max(p2.x)).ceil() as i32;
    let min_y = p0.y.min(p1.y.min(p2.y)).floor() as i32;
    let max_y = p0.y.max(p1.y.max(p2.y)).ceil() as i32;

    if max_x < 0 || max_y < 0 || min_x >= width || min_y >= height {
        return;
    }

    let x0 = min_x.max(0);
    let x1 = max_x.min(width - 1);
    let y0 = min_y.max(0);
    let y1 = max_y.min(height - 1);

    for y in y0..=y1 {
        let py = y as f32 + 0.5;
        let mut w0 = edge_fn(p1, p2, x0 as f32 + 0.5, py) * orient;
        let mut w1 = edge_fn(p2, p0, x0 as f32 + 0.5, py) * orient;
        let mut w2 = edge_fn(p0, p1, x0 as f32 + 0.5, py) * orient;

        let span_len = x1 - x0 + 1;
        if span_len <= 0 {
            continue;
        }

        rasterizer.blend_hspan_with(x0, y, span_len, |i| {
            let mut color = Rgba8888::TRANSPARENT;
            let mut coverage = 0u8;

            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let b0 = w0 * inv_area;
                let b1 = w1 * inv_area;
                let b2 = w2 * inv_area;

                let w0p = b0 * v0.inv_w;
                let w1p = b1 * v1.inv_w;
                let w2p = b2 * v2.inv_w;
                let denom = w0p + w1p + w2p;
                if denom > 0.0 {
                    let inv = 1.0 / denom;
                    let uv = (v0.uv * w0p + v1.uv * w1p + v2.uv * w2p) * inv;
                    let world_pos = (v0.world * w0p + v1.world * w1p + v2.world * w2p) * inv;
                    let mut normal = (v0.normal * w0p + v1.normal * w1p + v2.normal * w2p) * inv;
                    normal = normal.normalize();

                    let base_color = sample_material_color(material, textures, uv);
                    let lit_color = apply_lighting(
                        base_color,
                        normal,
                        world_pos,
                        camera.position,
                        light_dir,
                        ambient,
                        diffuse,
                        specular,
                        options.shininess,
                    );

                    color = lit_color;
                    coverage = 255;

                }
            }

            w0 += step_x0;
            w1 += step_x1;
            w2 += step_x2;
            (color, coverage)
        });
    }
}

fn sample_material_color(material: Option<&Material>, textures: &[Texture], uv: Vec2) -> Rgba8888 {
    if let Some(mat) = material {
        if let Some(tex_idx) = mat.base_color_texture {
            if let Some(texture) = textures.get(tex_idx) {
                return sample_texture(texture, uv, mat.base_color);
            }
        }
        return mat.base_color;
    }
    Rgba8888::WHITE
}

fn sample_texture(texture: &Texture, uv: Vec2, fallback: Rgba8888) -> Rgba8888 {
    if texture.width == 0 || texture.height == 0 || texture.data.is_empty() {
        return fallback;
    }
    let mut u = uv.x.fract();
    let mut v = uv.y.fract();
    if u < 0.0 {
        u += 1.0;
    }
    if v < 0.0 {
        v += 1.0;
    }
    let x = (u * texture.width as f32).floor() as usize % texture.width as usize;
    let y = (v * texture.height as f32).floor() as usize % texture.height as usize;
    let idx = y * texture.width as usize + x;
    texture.data.get(idx).copied().unwrap_or(fallback)
}

fn apply_lighting(
    base: Rgba8888,
    normal: Vec3,
    world_pos: Vec3,
    camera_pos: Vec3,
    light_dir: Vec3,
    ambient: f32,
    diffuse: f32,
    specular: f32,
    shininess: f32,
) -> Rgba8888 {
    let n = normal.normalize();
    let l = light_dir.normalize();
    let view_dir = (camera_pos - world_pos).normalize();
    let diff = n.dot(l).max(0.0);
    let mut spec_term = 0.0;
    if diff > 0.0 && specular > 0.0 {
        let half_vec = (l + view_dir).normalize();
        spec_term = n.dot(half_vec).max(0.0).powf(shininess.max(1.0));
    }

    let mut intensity = ambient + diffuse * diff + specular * spec_term;
    intensity = intensity.clamp(0.0, 1.0);

    let r = (base.r() as f32 * intensity).clamp(0.0, 255.0) as u8;
    let g = (base.g() as f32 * intensity).clamp(0.0, 255.0) as u8;
    let b = (base.b() as f32 * intensity).clamp(0.0, 255.0) as u8;
    Rgba8888::rgba(r, g, b, base.a())
}

fn draw_wireframe_triangle<R>(
    rasterizer: &mut R,
    v0: &PreparedVertex,
    v1: &PreparedVertex,
    v2: &PreparedVertex,
    color: Rgba8888,
)
where
    R: Rasterizer,
{
    draw_line(rasterizer, &v0.screen, &v1.screen, color);
    draw_line(rasterizer, &v1.screen, &v2.screen, color);
    draw_line(rasterizer, &v2.screen, &v0.screen, color);
}

fn draw_line<R>(rasterizer: &mut R, a: &Vec3, b: &Vec3, color: Rgba8888)
where
    R: Rasterizer,
{
    let width = rasterizer.width() as i32;
    let height = rasterizer.height() as i32;

    let mut x0 = a.x.round() as i32;
    let mut y0 = a.y.round() as i32;
    let x1 = b.x.round() as i32;
    let y1 = b.y.round() as i32;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && x0 < width && y0 >= 0 && y0 < height {
            rasterizer.blend_pixel(x0, y0, color, 255);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn compute_normal_matrix(model: &Mat4) -> [[f32; 3]; 3] {
    let a00 = model.m[0][0];
    let a01 = model.m[0][1];
    let a02 = model.m[0][2];
    let a10 = model.m[1][0];
    let a11 = model.m[1][1];
    let a12 = model.m[1][2];
    let a20 = model.m[2][0];
    let a21 = model.m[2][1];
    let a22 = model.m[2][2];

    let det = a00 * (a11 * a22 - a12 * a21) - a01 * (a10 * a22 - a12 * a20)
        + a02 * (a10 * a21 - a11 * a20);

    if det.abs() < 1.0e-8 {
        return [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    }

    let inv_det = 1.0 / det;

    let i00 = (a11 * a22 - a12 * a21) * inv_det;
    let i01 = (a02 * a21 - a01 * a22) * inv_det;
    let i02 = (a01 * a12 - a02 * a11) * inv_det;
    let i10 = (a12 * a20 - a10 * a22) * inv_det;
    let i11 = (a00 * a22 - a02 * a20) * inv_det;
    let i12 = (a02 * a10 - a00 * a12) * inv_det;
    let i20 = (a10 * a21 - a11 * a20) * inv_det;
    let i21 = (a01 * a20 - a00 * a21) * inv_det;
    let i22 = (a00 * a11 - a01 * a10) * inv_det;

    [[i00, i10, i20], [i01, i11, i21], [i02, i12, i22]]
}

fn transform_normal(matrix: &[[f32; 3]; 3], normal: Vec3) -> Vec3 {
    Vec3::new(
        matrix[0][0] * normal.x + matrix[0][1] * normal.y + matrix[0][2] * normal.z,
        matrix[1][0] * normal.x + matrix[1][1] * normal.y + matrix[1][2] * normal.z,
        matrix[2][0] * normal.x + matrix[2][1] * normal.y + matrix[2][2] * normal.z,
    )
}

fn edge_fn(a: &Vec3, b: &Vec3, x: f32, y: f32) -> f32 {
    (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
}
