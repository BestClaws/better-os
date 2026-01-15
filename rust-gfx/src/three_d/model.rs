use super::math::{Mat4, Quaternion, Vec2, Vec3};
use crate::color::Rgba8888;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::convert::TryInto;
use micromath::F32Ext;
use miniz_oxide::inflate::decompress_to_vec_zlib;
use serde::Deserialize;
use serde_json_core::de::from_slice;

#[derive(Clone, Debug, Default)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub name: Option<String>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct Texture {
    pub name: Option<String>,
    pub width: u32,
    pub height: u32,
    pub data: Vec<Rgba8888>,
}

#[derive(Clone, Debug, Default)]
pub struct Material {
    pub name: Option<String>,
    pub base_color: Rgba8888,
    pub base_color_texture: Option<usize>,
    pub double_sided: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub name: Option<String>,
    pub mesh: usize,
    pub transform: Mat4,
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
pub enum ModelError {
    InvalidMagic,
    UnsupportedVersion(u32),
    Truncated,
    MissingChunk(&'static str),
    Utf8(core::str::Utf8Error),
    Json(serde_json_core::de::Error),
    BufferOutOfRange,
    Unsupported(&'static str),
    Png(&'static str),
    Decompression,
}

impl From<serde_json_core::de::Error> for ModelError {
    fn from(err: serde_json_core::de::Error) -> Self {
        ModelError::Json(err)
    }
}

impl From<core::str::Utf8Error> for ModelError {
    fn from(err: core::str::Utf8Error) -> Self {
        ModelError::Utf8(err)
    }
}

#[derive(Deserialize)]
struct Buffer {
    #[serde(rename = "byteLength")]
    byte_length: u32,
}

#[derive(Deserialize)]
struct BufferView {
    buffer: u32,
    #[serde(rename = "byteOffset", default)]
    byte_offset: u32,
    #[serde(rename = "byteLength")]
    byte_length: u32,
    #[serde(rename = "byteStride", default)]
    byte_stride: Option<u32>,
}

#[derive(Deserialize)]
struct Accessor<'a> {
    #[serde(rename = "bufferView", default)]
    buffer_view: Option<u32>,
    #[serde(rename = "byteOffset", default)]
    byte_offset: u32,
    #[serde(rename = "componentType")]
    component_type: u32,
    count: u32,
    #[serde(rename = "type")]
    #[serde(borrow)]
    accessor_type: &'a str,
}

#[derive(Deserialize, Default)]
struct PrimitiveAttributes {
    #[serde(rename = "POSITION")]
    position: Option<u32>,
    #[serde(rename = "NORMAL")]
    normal: Option<u32>,
    #[serde(rename = "TEXCOORD_0")]
    texcoord0: Option<u32>,
}

#[derive(Deserialize)]
struct MeshPrimitive {
    attributes: PrimitiveAttributes,
    #[serde(default)]
    indices: Option<u32>,
    #[serde(default)]
    material: Option<u32>,
}

#[derive(Deserialize)]
struct MeshDef<'a> {
    #[serde(default)]
    #[serde(borrow)]
    name: Option<&'a str>,
    primitives: Vec<MeshPrimitive>,
}

#[derive(Deserialize, Default)]
struct NodeDef<'a> {
    #[serde(default)]
    #[serde(borrow)]
    name: Option<&'a str>,
    #[serde(default)]
    mesh: Option<u32>,
    #[serde(default)]
    translation: Option<[f32; 3]>,
    #[serde(default)]
    rotation: Option<[f32; 4]>,
    #[serde(default)]
    scale: Option<[f32; 3]>,
    #[serde(default)]
    matrix: Option<[f32; 16]>,
    #[serde(default)]
    children: Option<Vec<u32>>,
}

#[derive(Deserialize, Default)]
struct SceneDef {
    #[serde(default)]
    nodes: Vec<u32>,
}

#[derive(Deserialize, Default)]
struct TextureRef {
    index: u32,
}

#[derive(Deserialize, Default)]
struct Pbr {
    #[serde(rename = "baseColorTexture", default)]
    base_color_texture: Option<TextureRef>,
    #[serde(rename = "baseColorFactor", default)]
    base_color_factor: Option<[f32; 4]>,
}

#[derive(Deserialize, Default)]
struct MaterialDef<'a> {
    #[serde(default)]
    #[serde(borrow)]
    name: Option<&'a str>,
    #[serde(rename = "doubleSided", default)]
    double_sided: Option<bool>,
    #[serde(rename = "pbrMetallicRoughness", default)]
    pbr: Option<Pbr>,
}

#[derive(Deserialize, Default)]
struct TextureDef {
    source: u32,
}

#[derive(Deserialize, Default)]
struct ImageDef<'a> {
    #[serde(default)]
    #[serde(borrow)]
    name: Option<&'a str>,
    #[serde(rename = "bufferView")]
    buffer_view: u32,
    #[serde(rename = "mimeType")]
    #[serde(borrow)]
    mime_type: &'a str,
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
struct Gltf<'a> {
    #[serde(default)]
    buffers: Vec<Buffer>,
    #[serde(rename = "bufferViews")]
    #[serde(default)]
    buffer_views: Vec<BufferView>,
    #[serde(default)]
    accessors: Vec<Accessor<'a>>,
    #[serde(default)]
    meshes: Vec<MeshDef<'a>>,
    #[serde(default)]
    nodes: Vec<NodeDef<'a>>,
    #[serde(default)]
    scenes: Vec<SceneDef>,
    #[serde(default)]
    scene: Option<u32>,
    #[serde(default)]
    materials: Vec<MaterialDef<'a>>,
    #[serde(default)]
    textures: Vec<TextureDef>,
    #[serde(default)]
    images: Vec<ImageDef<'a>>,
}

pub fn load_glb(bytes: &[u8]) -> Result<Scene, ModelError> {
    if bytes.len() < 12 {
        return Err(ModelError::Truncated);
    }

    if &bytes[0..4] != b"glTF" {
        return Err(ModelError::InvalidMagic);
    }

    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != 2 {
        return Err(ModelError::UnsupportedVersion(version));
    }

    let length = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    if length > bytes.len() {
        return Err(ModelError::Truncated);
    }

    let mut offset = 12;
    let mut json_chunk: Option<&[u8]> = None;
    let mut bin_chunk: Option<&[u8]> = None;

    while offset + 8 <= length {
        let chunk_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let chunk_type = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());
        offset += 8;
        if offset + chunk_len > bytes.len() {
            return Err(ModelError::Truncated);
        }
        let chunk_data = &bytes[offset..offset + chunk_len];
        offset += chunk_len;

        match chunk_type {
            0x4E4F534A => json_chunk = Some(chunk_data),
            0x004E4942 => bin_chunk = Some(chunk_data),
            _ => {}
        }
    }

    let json = json_chunk.ok_or(ModelError::MissingChunk("JSON"))?;
    let binary = bin_chunk.ok_or(ModelError::MissingChunk("BIN"))?;

    let (doc, _) = from_slice::<Gltf>(json)?;

    if doc.buffers.is_empty() {
        return Err(ModelError::Unsupported("missing buffers"));
    }

    // Decode textures first so materials can reference them
    let mut textures = Vec::with_capacity(doc.textures.len());
    for tex_def in doc.textures.iter() {
        let image_idx = tex_def.source as usize;
        let image_def = doc
            .images
            .get(image_idx)
            .ok_or(ModelError::Unsupported("texture source index"))?;
        if image_def.mime_type != "image/png" {
            return Err(ModelError::Unsupported("only PNG textures are supported"));
        }
        let (w, h, data, name) = decode_png_from_view(image_def, &doc.buffer_views, binary)?;
        textures.push(Texture {
            name,
            width: w,
            height: h,
            data,
        });
    }

    // Convert materials
    let mut materials = Vec::with_capacity(doc.materials.len());
    for mat in doc.materials.iter() {
        let pbr = mat.pbr.as_ref();
        let base_color = if let Some(p) = pbr.and_then(|p| p.base_color_factor.as_ref()) {
            let r = (p[0].clamp(0.0, 1.0) * 255.0).round() as u8;
            let g = (p[1].clamp(0.0, 1.0) * 255.0).round() as u8;
            let b = (p[2].clamp(0.0, 1.0) * 255.0).round() as u8;
            let a = (p[3].clamp(0.0, 1.0) * 255.0).round() as u8;
            Rgba8888::rgba(r, g, b, a)
        } else {
            Rgba8888::WHITE
        };

        let tex_idx = pbr
            .and_then(|p| p.base_color_texture.as_ref())
            .map(|t| t.index as usize);
        if let Some(idx) = tex_idx {
            if idx >= textures.len() {
                return Err(ModelError::Unsupported("material texture index"));
            }
        }

        materials.push(Material {
            name: mat.name.map(|s| s.to_string()),
            base_color,
            base_color_texture: tex_idx,
            double_sided: mat.double_sided.unwrap_or(false),
        });
    }

    // Convert meshes
    let mut meshes = Vec::with_capacity(doc.meshes.len());
    for mesh_def in doc.meshes.iter() {
        let mut mesh = Mesh {
            name: mesh_def.name.map(|s| s.to_string()),
            ..Mesh::default()
        };

        for primitive in mesh_def.primitives.iter() {
            let position_accessor = primitive
                .attributes
                .position
                .ok_or(ModelError::Unsupported("primitive missing POSITION"))?;
            let positions = read_vec3(
                position_accessor as usize,
                &doc.accessors,
                &doc.buffer_views,
                binary,
            )?;

            let normals = primitive
                .attributes
                .normal
                .map(|idx| read_vec3(idx as usize, &doc.accessors, &doc.buffer_views, binary))
                .transpose()?;

            let uvs = primitive
                .attributes
                .texcoord0
                .map(|idx| read_vec2(idx as usize, &doc.accessors, &doc.buffer_views, binary))
                .transpose()?;

            let indices = if let Some(idx) = primitive.indices {
                read_indices(idx as usize, &doc.accessors, &doc.buffer_views, binary)?
            } else {
                (0..positions.len() as u32).collect()
            };

            let mut vertices = Vec::with_capacity(positions.len());
            for i in 0..positions.len() {
                let normal = normals
                    .as_ref()
                    .map(|n| n[i])
                    .unwrap_or(Vec3::new(0.0, 0.0, 1.0));
                let uv = uvs.as_ref().map(|uvs| uvs[i]).unwrap_or_default();
                vertices.push(Vertex {
                    position: positions[i],
                    normal,
                    uv,
                });
            }

            mesh.vertices = vertices;
            mesh.indices = indices;
            mesh.material = primitive.material.map(|m| m as usize);
        }

        meshes.push(mesh);
    }

    // Resolve scene graph
    let mut nodes_out = Vec::new();
    let scene_idx = doc.scene.unwrap_or(0) as usize;
    let scene = doc
        .scenes
        .get(scene_idx)
        .ok_or(ModelError::Unsupported("scene index"))?;

    for node_idx in scene.nodes.iter() {
        gather_nodes(
            *node_idx as usize,
            &doc.nodes,
            &mut nodes_out,
            Mat4::identity(),
        );
    }

    Ok(Scene {
        meshes,
        materials,
        textures,
        nodes: nodes_out,
    })
}

fn gather_nodes(node_idx: usize, nodes: &[NodeDef<'_>], output: &mut Vec<Node>, parent: Mat4) {
    if let Some(node_def) = nodes.get(node_idx) {
        let local = build_node_transform(node_def);
        let world = parent.mul_mat4(&local);

        if let Some(mesh_idx) = node_def.mesh {
            output.push(Node {
                name: node_def.name.map(|s| s.to_string()),
                mesh: mesh_idx as usize,
                transform: world,
            });
        }

        if let Some(children) = node_def.children.as_ref() {
            for child in children.iter() {
                gather_nodes(*child as usize, nodes, output, world);
            }
        }
    }
}

fn build_node_transform(node: &NodeDef<'_>) -> Mat4 {
    if let Some(matrix) = node.matrix.as_ref() {
        let mut m = [[0.0f32; 4]; 4];
        for row in 0..4 {
            for col in 0..4 {
                m[row][col] = matrix[col * 4 + row];
            }
        }
        return Mat4 { m };
    }

    let translation = node
        .translation
        .unwrap_or([0.0, 0.0, 0.0]);
    let rotation = node
        .rotation
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let scale = node.scale.unwrap_or([1.0, 1.0, 1.0]);

    let t = Mat4::from_translation(Vec3::new(translation[0], translation[1], translation[2]));
    let r = Mat4::from_quaternion(Quaternion::new(
        rotation[3],
        rotation[0],
        rotation[1],
        rotation[2],
    ));
    let s = Mat4::from_scale(Vec3::new(scale[0], scale[1], scale[2]));
    t.mul_mat4(&r).mul_mat4(&s)
}

fn decode_png_from_view(
    image: &ImageDef<'_>,
    views: &[BufferView],
    binary: &[u8],
) -> Result<(u32, u32, Vec<Rgba8888>, Option<String>), ModelError> {
    let view = views
        .get(image.buffer_view as usize)
        .ok_or(ModelError::Unsupported("image buffer view index"))?;
    let slice = slice_view(view, binary, 0, view.byte_length as usize)?;
    let (w, h, data) = decode_png(slice)?;
    Ok((w, h, data, image.name.map(|s| s.to_string())))
}

fn slice_view<'a>(
    view: &'a BufferView,
    binary: &'a [u8],
    extra_offset: usize,
    size: usize,
) -> Result<&'a [u8], ModelError> {
    let start = view.byte_offset as usize + extra_offset;
    let end = start + size;
    if end > view.byte_offset as usize + view.byte_length as usize {
        return Err(ModelError::BufferOutOfRange);
    }
    if end > binary.len() {
        return Err(ModelError::Truncated);
    }
    Ok(&binary[start..end])
}

fn read_vec3(
    accessor_idx: usize,
    accessors: &[Accessor<'_>],
    views: &[BufferView],
    binary: &[u8],
) -> Result<Vec<Vec3>, ModelError> {
    let accessor = accessors
        .get(accessor_idx)
        .ok_or(ModelError::Unsupported("accessor index"))?;
    if accessor.accessor_type != "VEC3" || accessor.component_type != 5126 {
        return Err(ModelError::Unsupported("expected VEC3 float accessor"));
    }
    let view_idx = accessor
        .buffer_view
        .ok_or(ModelError::Unsupported("accessor missing bufferView"))?
        as usize;
    let view = views
        .get(view_idx)
        .ok_or(ModelError::Unsupported("accessor buffer view"))?;

    let stride = view.byte_stride.unwrap_or(12) as usize;
    let base = view.byte_offset as usize + accessor.byte_offset as usize;
    let mut out = Vec::with_capacity(accessor.count as usize);
    for i in 0..accessor.count as usize {
        let offset = base + i * stride;
        if offset + 12 > binary.len() {
            return Err(ModelError::Truncated);
        }
        let slice = &binary[offset..offset + 12];
        let x = f32::from_le_bytes(slice[0..4].try_into().unwrap());
        let y = f32::from_le_bytes(slice[4..8].try_into().unwrap());
        let z = f32::from_le_bytes(slice[8..12].try_into().unwrap());
        out.push(Vec3::new(x, y, z));
    }
    Ok(out)
}

fn read_vec2(
    accessor_idx: usize,
    accessors: &[Accessor<'_>],
    views: &[BufferView],
    binary: &[u8],
) -> Result<Vec<Vec2>, ModelError> {
    let accessor = accessors
        .get(accessor_idx)
        .ok_or(ModelError::Unsupported("accessor index"))?;
    if accessor.accessor_type != "VEC2" || accessor.component_type != 5126 {
        return Err(ModelError::Unsupported("expected VEC2 float accessor"));
    }
    let view_idx = accessor
        .buffer_view
        .ok_or(ModelError::Unsupported("accessor missing bufferView"))?
        as usize;
    let view = views
        .get(view_idx)
        .ok_or(ModelError::Unsupported("accessor buffer view"))?;

    let stride = view.byte_stride.unwrap_or(8) as usize;
    let base = view.byte_offset as usize + accessor.byte_offset as usize;
    let mut out = Vec::with_capacity(accessor.count as usize);
    for i in 0..accessor.count as usize {
        let offset = base + i * stride;
        if offset + 8 > binary.len() {
            return Err(ModelError::Truncated);
        }
        let slice = &binary[offset..offset + 8];
        let x = f32::from_le_bytes(slice[0..4].try_into().unwrap());
        let y = f32::from_le_bytes(slice[4..8].try_into().unwrap());
        out.push(Vec2::new(x, y));
    }
    Ok(out)
}

fn read_indices(
    accessor_idx: usize,
    accessors: &[Accessor<'_>],
    views: &[BufferView],
    binary: &[u8],
) -> Result<Vec<u32>, ModelError> {
    let accessor = accessors
        .get(accessor_idx)
        .ok_or(ModelError::Unsupported("accessor index"))?;
    if accessor.accessor_type != "SCALAR" {
        return Err(ModelError::Unsupported("indices accessor must be SCALAR"));
    }
    let view_idx = accessor
        .buffer_view
        .ok_or(ModelError::Unsupported("accessor missing bufferView"))?
        as usize;
    let view = views
        .get(view_idx)
        .ok_or(ModelError::Unsupported("accessor buffer view"))?;

    let stride = view.byte_stride.unwrap_or_else(|| match accessor.component_type {
        5121 => 1,
        5123 => 2,
        5125 => 4,
        _ => 0,
    }) as usize;
    if stride == 0 {
        return Err(ModelError::Unsupported("unsupported index component type"));
    }

    let base = view.byte_offset as usize + accessor.byte_offset as usize;
    let mut out = Vec::with_capacity(accessor.count as usize);
    for i in 0..accessor.count as usize {
        let offset = base + i * stride;
        let value = match accessor.component_type {
            5121 => {
                if offset + 1 > binary.len() {
                    return Err(ModelError::Truncated);
                }
                binary[offset] as u32
            }
            5123 => {
                if offset + 2 > binary.len() {
                    return Err(ModelError::Truncated);
                }
                u16::from_le_bytes(binary[offset..offset + 2].try_into().unwrap()) as u32
            }
            5125 => {
                if offset + 4 > binary.len() {
                    return Err(ModelError::Truncated);
                }
                u32::from_le_bytes(binary[offset..offset + 4].try_into().unwrap())
            }
            _ => return Err(ModelError::Unsupported("unsupported index component type")),
        };
        out.push(value);
    }
    Ok(out)
}

fn decode_png(data: &[u8]) -> Result<(u32, u32, Vec<Rgba8888>), ModelError> {
    const SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if data.len() < 8 || &data[..8] != SIG {
        return Err(ModelError::Png("invalid signature"));
    }

    let mut idx = 8;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut bit_depth = 0u8;
    let mut color_type = 0u8;
    let mut compression = 0u8;
    let mut filter = 0u8;
    let mut interlace = 0u8;
    let mut idat_data: Vec<u8> = Vec::new();

    while idx + 8 <= data.len() {
        let chunk_len = u32::from_be_bytes(data[idx..idx + 4].try_into().unwrap()) as usize;
        let chunk_type = &data[idx + 4..idx + 8];
        let chunk_data = &data[idx + 8..idx + 8 + chunk_len];
        idx += 12 + chunk_len;

        match chunk_type {
            b"IHDR" => {
                width = u32::from_be_bytes(chunk_data[0..4].try_into().unwrap());
                height = u32::from_be_bytes(chunk_data[4..8].try_into().unwrap());
                bit_depth = chunk_data[8];
                color_type = chunk_data[9];
                compression = chunk_data[10];
                filter = chunk_data[11];
                interlace = chunk_data[12];
            }
            b"IDAT" => {
                idat_data.extend_from_slice(chunk_data);
            }
            b"IEND" => break,
            _ => {}
        }
    }

    if color_type != 2 || bit_depth != 8 || compression != 0 || filter != 0 || interlace != 0 {
        return Err(ModelError::Unsupported("only RGB8 non-interlaced PNG supported"));
    }

    let decompressed = decompress_to_vec_zlib(&idat_data).map_err(|_| ModelError::Decompression)?;
    let bytes_per_pixel = 3usize;
    let row_len = width as usize * bytes_per_pixel;
    let expected = height as usize * (row_len + 1);
    if decompressed.len() != expected {
        return Err(ModelError::Png("unexpected decompressed size"));
    }

    let mut output = Vec::with_capacity((width * height) as usize);
    let mut prev_row = vec![0u8; row_len];
    let mut cur_row = vec![0u8; row_len];

    let mut cursor = 0;
    for _ in 0..height as usize {
        let filter_type = decompressed[cursor];
        cursor += 1;
        let raw_row = &decompressed[cursor..cursor + row_len];
        cursor += row_len;

        match filter_type {
            0 => cur_row.copy_from_slice(raw_row),
            1 => apply_sub_filter(raw_row, &mut cur_row, bytes_per_pixel),
            2 => apply_up_filter(raw_row, &mut cur_row, &prev_row),
            3 => apply_avg_filter(raw_row, &mut cur_row, &prev_row, bytes_per_pixel),
            4 => apply_paeth_filter(raw_row, &mut cur_row, &prev_row, bytes_per_pixel),
            _ => return Err(ModelError::Png("unsupported filter")),
        }

        for px in 0..width as usize {
            let idx = px * bytes_per_pixel;
            output.push(Rgba8888::rgba(cur_row[idx], cur_row[idx + 1], cur_row[idx + 2], 255));
        }

        core::mem::swap(&mut cur_row, &mut prev_row);
    }

    Ok((width, height, output))
}

fn apply_sub_filter(raw: &[u8], out: &mut [u8], bpp: usize) {
    for i in 0..raw.len() {
        let left = if i >= bpp { out[i - bpp] } else { 0 };
        out[i] = raw[i].wrapping_add(left);
    }
}

fn apply_up_filter(raw: &[u8], out: &mut [u8], prev: &[u8]) {
    for i in 0..raw.len() {
        out[i] = raw[i].wrapping_add(prev[i]);
    }
}

fn apply_avg_filter(raw: &[u8], out: &mut [u8], prev: &[u8], bpp: usize) {
    for i in 0..raw.len() {
        let left = if i >= bpp { out[i - bpp] } else { 0 };
        let up = prev[i];
        out[i] = raw[i].wrapping_add(((left as u16 + up as u16) >> 1) as u8);
    }
}

fn apply_paeth_filter(raw: &[u8], out: &mut [u8], prev: &[u8], bpp: usize) {
    for i in 0..raw.len() {
        let a = if i >= bpp { out[i - bpp] } else { 0 };
        let b = prev[i];
        let c = if i >= bpp { prev[i - bpp] } else { 0 };
        out[i] = raw[i].wrapping_add(paeth_predictor(a, b, c));
    }
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i32;
    let b = b as i32;
    let c = c as i32;
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}
