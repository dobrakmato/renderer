use crate::math::Vec3;
use bf::{IndexType, VertexDataFormat};
use byteorder::{LittleEndian, WriteBytesExt};
use std::convert::TryFrom;
use wavefront_obj::obj::Object;
use wavefront_obj::obj::Primitive::Triangle;

#[derive(Default)]
pub struct Geometry {
    pub positions: Vec<Vec3<f64>>,
    pub normals: Vec<Vec3<f64>>,
    pub tex_coords: Vec<Vec3<f64>>,
    /* 3 consecutive values represent one triangle (when correctly aligned) */
    pub indices: Vec<usize>,
}

impl Geometry {
    /// Recalculates the vertex normals by summing face normals on each
    /// vertex. Old normals are discarded.
    pub fn recalculate_normals(&mut self) {
        /* in the first step we zero the normals */
        self.normals.iter_mut().for_each(|it| *it = Vec3::default());

        /* for each face we compute the normal and add it to all vertices */
        for face in self.indices.chunks(3) {
            let v0 = &self.positions[face[0]];
            let v1 = &self.positions[face[1]];
            let v2 = &self.positions[face[2]];

            let v01 = v0 - v1;
            let v02 = v0 - v2;

            let normal = v01.cross(&v02);

            self.normals[face[0]] += &normal;
            self.normals[face[1]] += &normal;
            self.normals[face[2]] += &normal;
        }

        /* we then normalize the vertices */
        self.normals.iter_mut().for_each(|it| it.normalize());
    }

    /// Generates and .OBJ format representation of this geometry. The
    /// resulting OBJ file is returned as String.
    pub fn to_obj(&self) -> String {
        let mut buff = String::with_capacity(8192);

        for v in self.positions.iter() {
            buff.push_str(&format!("v {} {} {}\n", v.x, v.y, v.z))
        }

        for t in self.tex_coords.iter() {
            buff.push_str(&format!("vt {} {}\n", t.x, t.y))
        }

        for n in self.normals.iter() {
            buff.push_str(&format!("vn {} {} {}\n", n.x, n.y, n.z))
        }

        for face in self.indices.chunks(3) {
            buff.push_str(&format!(
                "f {} {} {}\n",
                face[0] + 1,
                face[1] + 1,
                face[2] + 1
            ))
        }
        buff
    }

    /// Encodes this geometry into byte buffer containing
    /// bytes with format (layout, padding) specified by `VertexDataFormat`
    /// parameter.
    pub fn generate_vertex_data(&self, format: VertexDataFormat) -> Vec<u8> {
        // the only supported format
        assert_eq!(format, VertexDataFormat::PositionNormalUv);

        let capacity = (self.positions.len() * std::mem::size_of::<f32>() * 3)
            + (self.normals.len() * std::mem::size_of::<f32>() * 3)
            + (self.tex_coords.len() * std::mem::size_of::<f32>() * 2);
        let mut buf = Vec::with_capacity(capacity);

        assert_eq!(self.positions.len(), self.normals.len());
        assert_eq!(self.positions.len(), self.tex_coords.len());

        let pos_iter = self.positions.iter();
        let nor_iter = self.normals.iter();
        let uv_iter = self.tex_coords.iter();

        pos_iter
            .zip(nor_iter)
            .zip(uv_iter)
            .for_each(|((pos, nor), uv)| {
                buf.write_f32::<LittleEndian>(pos.x as f32).unwrap();
                buf.write_f32::<LittleEndian>(pos.y as f32).unwrap();
                buf.write_f32::<LittleEndian>(pos.z as f32).unwrap();

                buf.write_f32::<LittleEndian>(nor.x as f32).unwrap();
                buf.write_f32::<LittleEndian>(nor.y as f32).unwrap();
                buf.write_f32::<LittleEndian>(nor.z as f32).unwrap();

                buf.write_f32::<LittleEndian>(uv.x as f32).unwrap();
                buf.write_f32::<LittleEndian>(uv.y as f32).unwrap();
            });

        buf
    }

    /// Returns the `IndexType` which is considered the best to store
    /// this geometry index data. The type returned depends on number
    /// of indices in this geometry. Current algorithm returns the
    /// smallest type that can be used to encode this geometry.
    pub fn suggest_index_type(&self) -> IndexType {
        if self.indices.len() < std::u8::MAX as usize {
            return IndexType::U8;
        }
        if self.indices.len() < std::u16::MAX as usize {
            return IndexType::U16;
        }

        IndexType::U32
    }

    /// Encodes this geometry index data into byte buffer with the
    /// index type specified by `index_type` parameter.
    ///
    /// This function expects the specified `IndexType` is valid
    /// and the index buffer can be encoded with it. It is best to
    /// use `suggest_index_type` to determine index type for geometry.
    pub fn generate_index_data(&self, index_type: IndexType) -> Vec<u8> {
        let capacity = self.indices.len() * index_type.size_of_one_index();
        let mut buf = Vec::with_capacity(capacity);

        match index_type {
            IndexType::U8 => assert!(self.indices.len() <= std::u8::MAX as usize),
            IndexType::U16 => assert!(self.indices.len() <= std::u16::MAX as usize),
            IndexType::U32 => assert!(self.indices.len() <= std::u32::MAX as usize),
        }

        self.indices.iter().for_each(|x| match index_type {
            IndexType::U8 => buf.write_u8(*x as u8).unwrap(),
            IndexType::U16 => buf.write_u16::<LittleEndian>(*x as u16).unwrap(),
            IndexType::U32 => buf.write_u32::<LittleEndian>(*x as u32).unwrap(),
        });

        buf
    }
}

// todo: add non-exhaustive annotation (when stable)
pub enum ObjImportError {
    TooManyGeometries,
    NoGeometries,
    UnsupportedPrimitive,
}

impl TryFrom<&Object> for Geometry {
    type Error = ObjImportError;

    /// Converts Wavefront Object instance to Geometry. This function
    /// expects the object to have exactly one geometry inside and
    /// the geometry may not contain points or lines. If any of these
    /// constraints are violated the conversion fails.
    fn try_from(obj: &Object) -> Result<Self, Self::Error> {
        if obj.geometry.is_empty() {
            return Err(ObjImportError::NoGeometries);
        } else if obj.geometry.len() > 1 {
            return Err(ObjImportError::TooManyGeometries);
        }

        let mut triplets: Vec<(usize, usize, usize)> = Vec::new();
        let mut g = Self::default();

        for x in obj.geometry.first().unwrap().shapes.iter() {
            /* the library will automatically convert polygons to triangles */
            if let Triangle(
                (vi, Some(ti), Some(ni)),
                (vj, Some(tj), Some(nj)),
                (vk, Some(tk), Some(nk)),
            ) = x.primitive
            {
                // todo: unroll
                for (v, t, n) in [(vi, ti, ni), (vj, tj, nj), (vk, tk, nk)].iter() {
                    let triplet = (*v, *t, *n);
                    let idx = triplets
                        .iter()
                        .position(|it| *it == triplet)
                        .unwrap_or_else(|| {
                            triplets.push(triplet);

                            /* Safe: indices are guaranteed to be valid by the library */
                            let (v, t, n) = unsafe {
                                let v = obj.vertices.get_unchecked(*v);
                                let t = obj.tex_vertices.get_unchecked(*t);
                                let n = obj.normals.get_unchecked(*n);
                                (v, t, n)
                            };

                            g.positions.push(Vec3::new(v.x, v.y, v.z));
                            g.normals.push(Vec3::new(n.x, n.y, n.z));
                            g.tex_coords.push(Vec3::new(t.u, t.v, t.w));

                            triplets.len() - 1
                        });

                    g.indices.push(idx);
                }
            } else {
                return Err(ObjImportError::UnsupportedPrimitive);
            }
        }
        Ok(g)
    }
}
