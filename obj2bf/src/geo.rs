use crate::math::Vec3;
use bf::mesh::{IndexType, VertexFormat};
use byteorder::{LittleEndian, WriteBytesExt};
use ordered_float::{FloatIsNan, NotNan};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::TryFrom;
use wavefront_obj::obj::Primitive::Triangle;
use wavefront_obj::obj::{Object, TVertex, Vertex};

#[derive(Default)]
pub struct Geometry {
    pub positions: Vec<Vec3<f64>>,
    pub normals: Vec<Vec3<f64>>,
    pub tex_coords: Vec<Vec3<f64>>,
    pub tangents: Vec<Vec3<f64>>,
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

    /// Recalculates the vertex tangents from position and index information.
    ///
    /// You probably should call this method every time because tangents are usually
    /// not imported from another format.
    pub fn recalculate_tangents(&mut self) {
        /* firstly we allocate large vec to store tangents */
        self.tangents = vec![Vec3::new(0.0, 0.0, 0.0); self.positions.len()];

        /* for each face we compute the tangent and add it to all vertices */
        for face in self.indices.chunks(3) {
            let edge1 = &self.positions[face[1]] - &self.positions[face[0]];
            let edge2 = &self.positions[face[2]] - &self.positions[face[0]];

            let uv0 = &self.tex_coords[face[0]];
            let uv1 = &self.tex_coords[face[1]];
            let uv2 = &self.tex_coords[face[2]];

            let d_u1 = uv1.x - uv0.x;
            let d_v1 = uv1.y - uv0.y;
            let d_u2 = uv2.x - uv0.x;
            let d_v2 = uv2.y - uv0.y;

            let f = 1.0 / (d_u1 * d_v2 - d_u2 * d_v1);

            let tangent = Vec3::new(
                f * (d_v2 * edge1.x - d_v1 * edge2.x),
                f * (d_v2 * edge1.y - d_v1 * edge2.y),
                f * (d_v2 * edge1.z - d_v1 * edge2.z),
            );

            self.tangents[face[0]] += &tangent;
            self.tangents[face[1]] += &tangent;
            self.tangents[face[2]] += &tangent;
        }

        /* we then normalize the tangents */
        self.tangents.iter_mut().for_each(|it| it.normalize());
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
    pub fn generate_vertex_data(&self, format: VertexFormat) -> Vec<u8> {
        // the only supported format
        assert_eq!(format, VertexFormat::PositionNormalUvTangent);

        let capacity = (self.positions.len() * std::mem::size_of::<f32>() * 3)
            + (self.normals.len() * std::mem::size_of::<f32>() * 3)
            + (self.tex_coords.len() * std::mem::size_of::<f32>() * 2)
            + (self.normals.len() * std::mem::size_of::<f32>() * 4); // tangents + padding
        let mut buf = Vec::with_capacity(capacity);

        assert_eq!(self.positions.len(), self.normals.len());
        assert_eq!(self.positions.len(), self.tex_coords.len());

        let pos_iter = self.positions.iter();
        let nor_iter = self.normals.iter();
        let uv_iter = self.tex_coords.iter();
        let tan_iter = self.tangents.iter();

        pos_iter
            .zip(nor_iter)
            .zip(uv_iter)
            .zip(tan_iter)
            .for_each(|(((pos, nor), uv), tan)| {
                buf.write_f32::<LittleEndian>(pos.x as f32)
                    .expect("cannot write f32");
                buf.write_f32::<LittleEndian>(pos.y as f32)
                    .expect("cannot write f32");
                buf.write_f32::<LittleEndian>(pos.z as f32)
                    .expect("cannot write f32");

                buf.write_f32::<LittleEndian>(nor.x as f32)
                    .expect("cannot write f32");
                buf.write_f32::<LittleEndian>(nor.y as f32)
                    .expect("cannot write f32");
                buf.write_f32::<LittleEndian>(nor.z as f32)
                    .expect("cannot write f32");

                buf.write_f32::<LittleEndian>(uv.x as f32)
                    .expect("cannot write f32");
                buf.write_f32::<LittleEndian>(uv.y as f32)
                    .expect("cannot write f32");

                buf.write_f32::<LittleEndian>(tan.x as f32)
                    .expect("cannot write f32");
                buf.write_f32::<LittleEndian>(tan.y as f32)
                    .expect("cannot write f32");
                buf.write_f32::<LittleEndian>(tan.z as f32)
                    .expect("cannot write f32");
                buf.write_f32::<LittleEndian>(0.0) // padding
                    .expect("cannot write f32");
            });

        buf
    }

    /// Returns the `IndexType` which is considered the best to store
    /// this geometry index data. The type returned depends on number
    /// of indices in this geometry. Current algorithm returns the
    /// smallest type that can be used to encode this geometry.
    pub fn suggest_index_type(&self) -> IndexType {
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
            IndexType::U16 => assert!(self.indices.len() <= std::u16::MAX as usize),
            IndexType::U32 => assert!(self.indices.len() <= std::u32::MAX as usize),
        }

        self.indices.iter().for_each(|x| match index_type {
            IndexType::U16 => buf.write_u16::<LittleEndian>(*x as u16).unwrap(),
            IndexType::U32 => buf.write_u32::<LittleEndian>(*x as u32).unwrap(),
        });

        buf
    }
}

#[derive(Debug)]
pub enum ObjImportError {
    InvalidGeometryIndex(usize, usize),
    NotANumber(FloatIsNan),
    UnsupportedPrimitive(wavefront_obj::obj::Primitive),
}

impl TryFrom<(&Object, usize)> for Geometry {
    type Error = ObjImportError;

    /// Converts Wavefront Object instance to Geometry. This function
    /// expects the object to have exactly one geometry inside and
    /// the geometry may not contain points or lines. If any of these
    /// constraints are violated the conversion fails.
    fn try_from(geometry_selector: (&Object, usize)) -> Result<Self, Self::Error> {
        let (obj, geo_idx) = geometry_selector;

        // try to choose geometry by index
        let geo = match obj.geometry.get(geo_idx) {
            None => {
                return Err(ObjImportError::InvalidGeometryIndex(
                    geo_idx,
                    obj.geometry.len(),
                ));
            }
            Some(t) => t,
        };

        // to find unique vertex data triplets we need to store all vertices
        // in a hashmap. because rust f32, f64 is not Hash by default we use
        // a crate `ordered-float`. before finding unique vertices we wrap them
        // to ordered variant `NotNan<f64>` which is Hash and after the uniqueness
        // we convert them back to raw f64.
        let vertex_to_vec = |v: &Vertex| {
            let x = NotNan::new(v.x).map_err(ObjImportError::NotANumber)?;
            let y = NotNan::new(v.y).map_err(ObjImportError::NotANumber)?;
            let z = NotNan::new(v.z).map_err(ObjImportError::NotANumber)?;
            Ok(Vec3::new(x, y, z))
        };
        let tvertex_to_vec = |v: &TVertex| {
            let x = NotNan::new(v.u).map_err(ObjImportError::NotANumber)?;
            let y = NotNan::new(v.v).map_err(ObjImportError::NotANumber)?;
            let z = NotNan::new(v.w).map_err(ObjImportError::NotANumber)?;
            Ok(Vec3::new(x, y, z))
        };
        let not_nan_to_f = |v: Vec3<NotNan<f64>>| Vec3::new(*v.x, *v.y, *v.z);

        let mut triplets_idx = 0;
        let mut triplets_unique = HashMap::new();
        let mut geometry = Self::default();

        for shape in geo.shapes.iter() {
            /* the library will automatically convert polygons to triangles */
            if let Triangle(
                (vi, Some(ti), Some(ni)),
                (vj, Some(tj), Some(nj)),
                (vk, Some(tk), Some(nk)),
            ) = shape.primitive
            {
                for (v, t, n) in [(vi, ti, ni), (vj, tj, nj), (vk, tk, nk)].iter() {
                    /* Safe: indices are guaranteed to be valid by the library */
                    let triplet = unsafe {
                        let v = vertex_to_vec(obj.vertices.get_unchecked(*v))?;
                        let t = tvertex_to_vec(obj.tex_vertices.get_unchecked(*t))?;
                        let n = vertex_to_vec(obj.normals.get_unchecked(*n))?;
                        (v, t, n)
                    };

                    let idx = match triplets_unique.entry(triplet) {
                        Entry::Occupied(e) => *e.get(),
                        Entry::Vacant(e) => {
                            let idx = *e.insert(triplets_idx);
                            triplets_idx += 1;

                            geometry.positions.push(not_nan_to_f(triplet.0));
                            geometry.tex_coords.push(not_nan_to_f(triplet.1));
                            geometry.normals.push(not_nan_to_f(triplet.2));

                            idx
                        }
                    };

                    geometry.indices.push(idx);
                }
            } else {
                return Err(ObjImportError::UnsupportedPrimitive(shape.primitive));
            }
        }
        Ok(geometry)
    }
}
