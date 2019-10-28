use std::fmt::{Display, Error, Formatter};
use wavefront_obj::obj::Primitive::Triangle;
use wavefront_obj::obj::{Object, TVertex, Vertex};

// todo: simd
#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Vec3<T> {
    x: T,
    y: T,
    z: T,
}

impl<T> Vec3<T> {
    fn new(x: T, y: T, z: T) -> Self {
        Vec3 { x, y, z }
    }
}

pub struct F64(f64);

impl Display for F64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_fmt(format_args!("{}", self.0))
    }
}

pub struct Geometry {
    pub positions: Vec<Vec3<F64>>,
    pub normals: Vec<Vec3<F64>>,
    pub tex_coords: Vec<Vec3<F64>>,
    pub indices: Vec<usize>,
}

impl Geometry {
    pub fn new() -> Self {
        Self {
            positions: vec![],
            normals: vec![],
            tex_coords: vec![],
            indices: vec![],
        }
    }

    pub fn push_vertex(&mut self, position: Vec3<F64>, normal: Vec3<F64>, tex_coord: Vec3<F64>) {
        self.positions.push(position);
        self.normals.push(normal);
        self.tex_coords.push(tex_coord);
        self.indices.push(self.indices.len());
    }

    pub fn dedupe_vertices(&mut self) {
        // find unique vertices

        // recreate index buffer

        // replace positions, normals, texcoords
    }

    pub fn to_obj(&self) -> String {
        let mut buff = String::with_capacity(8192);
        for x in self.positions.iter() {
            buff.push_str(&format!("v {} {} {}\n", x.x, x.y, x.z))
        }

        for x in self.tex_coords.iter() {
            buff.push_str(&format!("vt {} {} {}\n", x.x, x.y, x.z))
        }
        for x in self.normals.iter() {
            buff.push_str(&format!("vn {} {} {}\n", x.x, x.y, x.z))
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
}

impl From<&Object> for Geometry {
    fn from(obj: &Object) -> Self {
        let mut g = Self::new();

        if obj.geometry.len() != 1 {
            panic!("cannot convert .obj file with multiple or zero geometries");
        }

        for x in obj.geometry.first().unwrap().shapes.iter() {
            if let Triangle(
                (vi, Some(ti), Some(ni)),
                (vj, Some(tj), Some(nj)),
                (vk, Some(tk), Some(nk)),
            ) = x.primitive
            {
                // todo: unroll
                for (v, t, n) in [(vi, ti, ni), (vj, tj, nj), (vk, tk, nk)].iter() {
                    /* indices are guaranteed to be valid by the library */
                    let v = obj.vertices.get(*v).unwrap();
                    let t = obj.tex_vertices.get(*t).unwrap();
                    let n = obj.normals.get(*n).unwrap();

                    g.push_vertex(
                        Vec3::new(F64(v.x), F64(v.y), F64(v.z)),
                        Vec3::new(F64(n.x), F64(n.y), F64(n.z)),
                        Vec3::new(F64(t.u), F64(t.v), F64(t.w)),
                    );
                }
            } else {
                panic!("non-triangle primitives are not supported");
            }
        }
        g
    }
}
