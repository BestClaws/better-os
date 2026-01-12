use super::math::Vec3;

/// Configuration: Maximum allowed geometry size for models
pub const MAX_TRIANGLES: usize = 50;
pub const MAX_VERTICES: usize = 50;

/// A triangle in a 3D model, represented by indices into the vertex buffer
#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    pub vertices: [usize; 3],
    pub normal: Vec3,
}

/// A 3D model with pre-allocated vertex and triangle buffers
#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub vertices: [Vec3; MAX_VERTICES],
    pub vertex_count: usize,
    pub triangles: [Triangle; MAX_TRIANGLES],
    pub triangle_count: usize,
}

impl Model {
    /// Create a new, empty model
    pub const fn new() -> Self {
        Self {
            vertices: [Vec3(0.0, 0.0, 0.0); MAX_VERTICES],
            vertex_count: 0,
            triangles: [Triangle {
                vertices: [0, 0, 0],
                normal: Vec3(0.0, 0.0, 0.0),
            }; MAX_TRIANGLES],
            triangle_count: 0,
        }
    }

    /// Adds a vertex to the model
    pub fn add_vertex(&mut self, vertex: Vec3) -> Result<usize, StlError> {
        if self.vertex_count >= MAX_VERTICES {
            return Err(StlError::VertexCapacityExceeded);
        }
        self.vertices[self.vertex_count] = vertex;
        self.vertex_count += 1;
        Ok(self.vertex_count - 1)
    }

    /// Adds a triangle to the model using indices of existing vertices
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

/// Errors that may occur while parsing an STL file or manipulating a model
#[derive(Debug)]
pub enum StlError {
    InvalidHeader,
    BufferTooSmall,
    TooManyTriangles,
    TooManyVertices,
    VertexCapacityExceeded,
    TriangleCapacityExceeded,
    InvalidVertexIndex,
}

/// Parses a binary STL buffer into a Model structure
pub fn parse_binary_stl(buffer: &[u8]) -> Result<Model, StlError> {
    use core::convert::TryInto;

    if buffer.len() < 84 {
        return Err(StlError::BufferTooSmall);
    }

    let triangle_count = u32::from_le_bytes(
        buffer[80..84]
            .try_into()
            .map_err(|_| StlError::InvalidHeader)?,
    ) as usize;
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

        // Parse normal vector
        let normal = Vec3(
            f32::from_le_bytes(
                buffer[offset..offset + 4]
                    .try_into()
                    .map_err(|_| StlError::InvalidHeader)?,
            ),
            f32::from_le_bytes(
                buffer[offset + 4..offset + 8]
                    .try_into()
                    .map_err(|_| StlError::InvalidHeader)?,
            ),
            f32::from_le_bytes(
                buffer[offset + 8..offset + 12]
                    .try_into()
                    .map_err(|_| StlError::InvalidHeader)?,
            ),
        );

        // Parse the 3 vertices of the triangle
        let mut vertices: [Vec3; 3] = [Vec3(0.0, 0.0, 0.0); 3];
        for j in 0..3 {
            let base = offset + 12 + j * 12;
            vertices[j] = Vec3(
                f32::from_le_bytes(
                    buffer[base..base + 4]
                        .try_into()
                        .map_err(|_| StlError::InvalidHeader)?,
                ),
                f32::from_le_bytes(
                    buffer[base + 4..base + 8]
                        .try_into()
                        .map_err(|_| StlError::InvalidHeader)?,
                ),
                f32::from_le_bytes(
                    buffer[base + 8..base + 12]
                        .try_into()
                        .map_err(|_| StlError::InvalidHeader)?,
                ),
            );
        }

        // Deduplicate vertices based on position (naive O(n^2) match)
        let mut vertex_indices = [0usize; 3];
        for (j, v) in vertices.iter().enumerate() {
            let mut found = false;
            for k in 0..unique_vertices {
                if let Some(existing_idx) = vertex_map[k] {
                    let existing = model.vertices[existing_idx];
                    if (existing.0 - v.0).abs() < 0.0001
                        && (existing.1 - v.1).abs() < 0.0001
                        && (existing.2 - v.2).abs() < 0.0001
                    {
                        vertex_indices[j] = existing_idx;
                        found = true;
                        break;
                    }
                }
            }

            if !found {
                if unique_vertices >= MAX_VERTICES {
                    return Err(StlError::TooManyVertices);
                }
                let new_idx = model.add_vertex(*v)?;
                vertex_map[unique_vertices] = Some(new_idx);
                vertex_indices[j] = new_idx;
                unique_vertices += 1;
            }
        }

        // Add triangle to model
        model.add_triangle(
            vertex_indices[0],
            vertex_indices[1],
            vertex_indices[2],
            normal,
        )?;
    }

    Ok(model)
}
