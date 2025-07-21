use super::math::Vec3;
use defmt::Format;

// Maximum triangles supported (adjust based on MCU memory)
pub(crate) const MAX_TRIANGLES: usize = 50;
pub(crate) const MAX_VERTICES: usize = 50;

#[derive(Clone, Copy)]
pub struct Triangle {
    pub vertices: [usize; 3], // Indices into vertex array
    pub normal: Vec3,
}

#[derive(Clone, Copy)]
pub struct Model {
    pub vertices: [Vec3; MAX_VERTICES],
    pub vertex_count: usize,
    pub triangles: [Triangle; MAX_TRIANGLES],
    pub triangle_count: usize,
}

impl Model {
    pub fn new() -> Self {
        Model {
            vertices: [Vec3(0.0, 0.0, 0.0); MAX_VERTICES],
            vertex_count: 0,
            triangles: [Triangle { vertices: [0, 0, 0], normal: Vec3(0.0, 0.0, 0.0) }; MAX_TRIANGLES],
            triangle_count: 0,
        }
    }

    pub fn add_vertex(&mut self, vertex: Vec3) -> Result<usize, StlError> {
        if self.vertex_count >= MAX_VERTICES {
            return Err(StlError::VertexCapacityExceeded);
        }
        self.vertices[self.vertex_count] = vertex;
        self.vertex_count += 1;
        Ok(self.vertex_count - 1)
    }

    pub fn add_triangle(&mut self, v0: usize, v1: usize, v2: usize, normal: Vec3) -> Result<(), StlError> {
        if self.triangle_count >= MAX_TRIANGLES {
            return Err(StlError::TriangleCapacityExceeded);
        }
        if v0 >= self.vertex_count || v1 >= self.vertex_count || v2 >= self.vertex_count {
            return Err(StlError::InvalidVertexIndex);
        }
        self.triangles[self.triangle_count] = Triangle { vertices: [v0, v1, v2], normal };
        self.triangle_count += 1;
        Ok(())
    }
}

#[derive(Debug, Format)]
pub enum StlError {
    InvalidHeader,
    BufferTooSmall,
    TooManyTriangles,
    TooManyVertices,
    VertexCapacityExceeded,
    TriangleCapacityExceeded,
    InvalidVertexIndex,
}

pub fn parse_binary_stl(buffer: &[u8]) -> Result<Model, StlError> {
    if buffer.len() < 84 {
        return Err(StlError::BufferTooSmall);
    }

    // Skip 80-byte header
    let triangle_count = u32::from_le_bytes([buffer[80], buffer[81], buffer[82], buffer[83]]) as usize;
    if triangle_count > MAX_TRIANGLES {
        return Err(StlError::TooManyTriangles);
    }
    if buffer.len() < 84 + triangle_count * 50 {
        return Err(StlError::BufferTooSmall);
    }

    let mut model = Model::new();
    let mut vertex_map: [Option<usize>; MAX_VERTICES] = [None; MAX_VERTICES];
    let mut unique_vertices = 0;

    for i in 0..triangle_count {
        let offset = 84 + i * 50;
        // Read normal
        let normal = Vec3(
            f32::from_le_bytes([buffer[offset], buffer[offset + 1], buffer[offset + 2], buffer[offset + 3]]),
            f32::from_le_bytes([buffer[offset + 4], buffer[offset + 5], buffer[offset + 6], buffer[offset + 7]]),
            f32::from_le_bytes([buffer[offset + 8], buffer[offset + 9], buffer[offset + 10], buffer[offset + 11]]),
        );

        // Read vertices
        let vertices: [Vec3; 3] = [
            Vec3(
                f32::from_le_bytes([buffer[offset + 12], buffer[offset + 13], buffer[offset + 14], buffer[offset + 15]]),
                f32::from_le_bytes([buffer[offset + 16], buffer[offset + 17], buffer[offset + 18], buffer[offset + 19]]),
                f32::from_le_bytes([buffer[offset + 20], buffer[offset + 21], buffer[offset + 22], buffer[offset + 23]]),
            ),
            Vec3(
                f32::from_le_bytes([buffer[offset + 24], buffer[offset + 25], buffer[offset + 26], buffer[offset + 27]]),
                f32::from_le_bytes([buffer[offset + 28], buffer[offset + 29], buffer[offset + 30], buffer[offset + 31]]),
                f32::from_le_bytes([buffer[offset + 32], buffer[offset + 33], buffer[offset + 34], buffer[offset + 35]]),
            ),
            Vec3(
                f32::from_le_bytes([buffer[offset + 36], buffer[offset + 37], buffer[offset + 38], buffer[offset + 39]]),
                f32::from_le_bytes([buffer[offset + 40], buffer[offset + 41], buffer[offset + 42], buffer[offset + 43]]),
                f32::from_le_bytes([buffer[offset + 44], buffer[offset + 45], buffer[offset + 46], buffer[offset + 47]]),
            ),
        ];

        // Add vertices, reusing duplicates
        let mut vertex_indices = [0; 3];
        for (j, v) in vertices.iter().enumerate() {
            let mut found = false;
            for k in 0..unique_vertices {
                if let Some(idx) = vertex_map[k] {
                    if (model.vertices[idx].0 - v.0).abs() < 0.0001 &&
                        (model.vertices[idx].1 - v.1).abs() < 0.0001 &&
                        (model.vertices[idx].2 - v.2).abs() < 0.0001 {
                        vertex_indices[j] = idx;
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                if unique_vertices >= MAX_VERTICES {
                    return Err(StlError::TooManyVertices);
                }
                vertex_indices[j] = model.add_vertex(*v)?;
                vertex_map[unique_vertices] = Some(vertex_indices[j]);
                unique_vertices += 1;
            }
        }

        model.add_triangle(vertex_indices[0], vertex_indices[1], vertex_indices[2], normal)?;
    }

    Ok(model)
}