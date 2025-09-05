// model.rs

use crate::libs::gfx::Vec3;
pub(crate) use crate::system::kernel::config::resources::{MAX_TRIANGLES, MAX_VERTICES};
use defmt::{debug, error, info, Format};

/// A triangle in a 3D model, represented by indices into the vertex buffer.
#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    /// Indices into the `Model::vertices` array.
    pub vertices: [usize; 3],
    /// Face normal (stored per-triangle as provided by the STL file).
    pub normal: Vec3,
}

/// A 3D model with pre-allocated vertex and triangle buffers.
///
/// Important: **NOT** `Copy`. This type contains large, fixed-size arrays and
/// must not be implicitly copied (that would perform huge `memcpy`s and blow
/// MCU stacks).
#[derive(Debug, Clone)]
pub struct Model {
    pub vertices: [Vec3; MAX_VERTICES],
    pub vertex_count: usize,
    pub triangles: [Triangle; MAX_TRIANGLES],
    pub triangle_count: usize,
}

impl Model {
    /// Create an empty model (all slots zeroed, counts set to 0).
    ///
    /// This is `const`-friendly when `Vec3` implements `Copy` and the array
    /// lengths are constants.
    pub fn new() -> Self {
        Model {
            vertices: [Vec3(0.0, 0.0, 0.0); MAX_VERTICES],
            vertex_count: 0,
            triangles: [Triangle {
                vertices: [0, 0, 0],
                normal: Vec3(0.0, 0.0, 0.0),
            }; MAX_TRIANGLES],
            triangle_count: 0,
        }
    }

    /// Reset a model to empty without reallocating memory.
    pub fn clear(&mut self) {
        self.vertex_count = 0;
        self.triangle_count = 0;
    }

    /// Add a vertex into the next free slot. Returns the index of the vertex.
    /// Fails if the model is full.
    pub fn add_vertex(&mut self, vertex: Vec3) -> Result<usize, StlError> {
        if self.vertex_count >= MAX_VERTICES {
            return Err(StlError::VertexCapacityExceeded);
        }
        let idx = self.vertex_count;
        self.vertices[idx] = vertex;
        self.vertex_count += 1;
        Ok(idx)
    }

    /// Add a triangle referencing existing vertex indices.
    pub fn add_triangle(
        &mut self,
        v0: usize,
        v1: usize,
        v2: usize,
        normal: Vec3,
    ) -> Result<(), StlError> {
        if self.triangle_count >= MAX_TRIANGLES {
            return Err(StlError::TriangleCapacityExceeded);
        }
        if v0 >= self.vertex_count || v1 >= self.vertex_count || v2 >= self.vertex_count {
            return Err(StlError::InvalidVertexIndex);
        }
        self.triangles[self.triangle_count] = Triangle {
            vertices: [v0, v1, v2],
            normal,
        };
        self.triangle_count += 1;
        Ok(())
    }
}

/// Errors that may occur while parsing an STL file or manipulating a model.
#[derive(Debug, Format, PartialEq, Eq)]
pub enum StlError {
    InvalidHeader,
    BufferTooSmall,
    TooManyTriangles,
    TooManyVertices,
    VertexCapacityExceeded,
    TriangleCapacityExceeded,
    InvalidVertexIndex,
}

// -----------------------------
// Parsing helpers (safe, no panics)
// -----------------------------

/// Read a little-endian `f32` from `buffer` at `offset`.
/// Returns `Err(StlError::BufferTooSmall)` if out-of-bounds.
#[inline]
fn read_f32_le(buffer: &[u8], offset: usize) -> Result<f32, StlError> {
    let slice = buffer
        .get(offset..offset + 4)
        .ok_or(StlError::BufferTooSmall)?;
    let arr: [u8; 4] = slice.try_into().map_err(|_| StlError::BufferTooSmall)?;
    Ok(f32::from_le_bytes(arr))
}

/// Read a Vec3 (3 x f32) starting at `offset`.
#[inline]
fn read_vec3_le(buffer: &[u8], offset: usize) -> Result<Vec3, StlError> {
    let x = read_f32_le(buffer, offset)?;
    let y = read_f32_le(buffer, offset + 4)?;
    let z = read_f32_le(buffer, offset + 8)?;
    Ok(Vec3(x, y, z))
}

// -----------------------------
// Public parser
// -----------------------------

/// Parse a **binary STL** `buffer` into the provided `model` and use the
/// supplied `vertex_map_scratch` as temporary storage for deduplication.
///
/// Rationale for the signature: on constrained MCUs we **must not** allocate
/// large temporary buffers on the stack. The caller therefore provides the
/// `Model` storage and a scratch `vertex_map` (size `MAX_VERTICES`) so this
/// function never allocates large locals and remains deterministic for
/// memory usage.
///
/// `vertex_map_scratch` is expected to be zeroed (`[None; MAX_VERTICES]`) and
/// will be populated during parsing; the caller may reuse it for subsequent
/// parses by clearing its used prefix.
///
/// On success the `model` is filled and `Ok(())` is returned. No panic will
/// occur; all invalid inputs produce `StlError` values.
pub fn parse_binary_stl_into(
    buffer: &[u8],
    model: &mut Model,
    vertex_map_scratch: &mut [Option<usize>; MAX_VERTICES],
) -> Result<(), StlError> {
    // Minimum STL header size: 80 bytes header + 4 bytes triangle count
    if buffer.len() < 84 {
        return Err(StlError::BufferTooSmall);
    }

    // Read triangle count from bytes 80..84 (little endian u32)
    let count_slice = buffer.get(80..84).ok_or(StlError::BufferTooSmall)?;
    let count_arr: [u8; 4] = count_slice
        .try_into()
        .map_err(|_| StlError::BufferTooSmall)?;
    let triangle_count = u32::from_le_bytes(count_arr) as usize;

    if triangle_count > MAX_TRIANGLES {
        debug!("triangle count exceeds maximum. count: {}", triangle_count);
        return Err(StlError::TooManyTriangles);
    }

    // Each triangle record in binary STL is 50 bytes (12 bytes normal, 36 bytes vertices, 2 bytes attribute)
    let required_len = 84usize
        .checked_add(
            triangle_count
                .checked_mul(50)
                .ok_or(StlError::BufferTooSmall)?,
        )
        .ok_or(StlError::BufferTooSmall)?;
    if buffer.len() < required_len {
        return Err(StlError::BufferTooSmall);
    }

    // Clear the model (we reuse storage provided by the caller)
    model.clear();

    // Track how many unique vertices we've added (and how many entries in vertex_map_scratch are valid)
    let mut map_count: usize = 0;

    // Ensure the provided scratch is zeroed for the range we'll use; caller may reuse full array later.
    // We only need to clear up to MAX_VERTICES on first use; clearing full array is cheap and deterministic.
    for slot in vertex_map_scratch.iter_mut() {
        *slot = None;
    }

    // Iterate triangles
    for tri_index in 0..triangle_count {
        let tri_offset = 84 + tri_index * 50;

        // Parse normal
        let normal = read_vec3_le(buffer, tri_offset)?;

        // Parse 3 vertices (each 12 bytes)
        let mut tri_vertices: [Vec3; 3] = [Vec3(0.0, 0.0, 0.0); 3];
        for v in 0..3 {
            let v_offset = tri_offset + 12 + v * 12;
            tri_vertices[v] = read_vec3_le(buffer, v_offset)?;
        }

        // Deduplicate vertices (naive O(n^2) search over previously seen unique vertices).
        let mut indices: [usize; 3] = [0; 3];

        for (slot_idx, vertex) in tri_vertices.iter().enumerate() {
            let mut found = false;

            // Search previously recorded unique vertices (vertex_map_scratch[0..map_count])
            for m in 0..map_count {
                if let Some(existing_vertex_index) = vertex_map_scratch[m] {
                    let existing = model.vertices[existing_vertex_index];
                    if (existing.0 - vertex.0).abs() < 0.0001
                        && (existing.1 - vertex.1).abs() < 0.0001
                        && (existing.2 - vertex.2).abs() < 0.0001
                    {
                        indices[slot_idx] = existing_vertex_index;
                        found = true;
                        break;
                    }
                }
            }

            if !found {
                if map_count >= MAX_VERTICES || model.vertex_count >= MAX_VERTICES {
                    return Err(StlError::TooManyVertices);
                }
                let new_index = model.add_vertex(*vertex)?;
                vertex_map_scratch[map_count] = Some(new_index);
                indices[slot_idx] = new_index;
                map_count += 1;
            }
        }

        // Add triangle using the resolved indices
        model.add_triangle(indices[0], indices[1], indices[2], normal)?;
    }

    Ok(())
}

// A small convenience wrapper that creates a fresh Model and temporary map in
// static storage and fills it. This should only be used when the caller can't
// provide storage, and where a single-threaded static is acceptable.
//
// NOTE: This wrapper is purposely `cfg`-gated because it uses `static mut`.
#[cfg(feature = "single_thread_static_parser")]
pub fn parse_binary_stl(buffer: &[u8]) -> Result<Model, StlError> {
    use core::mem::MaybeUninit;

    static mut STATIC_MODEL: MaybeUninit<Model> = MaybeUninit::uninit();
    static mut STATIC_MAP: [Option<usize>; MAX_VERTICES] = [None; MAX_VERTICES];

    unsafe {
        // Initialize static model storage and pass to the fallible parser.
        STATIC_MODEL.as_mut_ptr().write(Model::new());
        let model_ptr = STATIC_MODEL.as_mut_ptr();
        let model_ref = &mut *model_ptr;

        // Clear the static scratch explicitly
        for slot in STATIC_MAP.iter_mut() {
            *slot = None;
        }

        parse_binary_stl_into(buffer, model_ref, &mut STATIC_MAP)?;

        // Move the model out of static storage to return it without double-drop
        let model = core::ptr::read(model_ptr);
        Ok(model)
    }
}
