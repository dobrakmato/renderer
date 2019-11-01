use crate::math::Vec3;
use wavefront_obj::obj::Object;
use wavefront_obj::obj::Primitive::Triangle;

pub struct Geometry {
    pub positions: Vec<Vec3<f64>>,
    pub normals: Vec<Vec3<f64>>,
    pub tex_coords: Vec<Vec3<f64>>,
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

    pub fn push_vertex(&mut self, position: Vec3<f64>, normal: Vec3<f64>, tex_coord: Vec3<f64>) {
        self.positions.push(position);
        self.normals.push(normal);
        self.tex_coords.push(tex_coord);
        self.indices.push(self.indices.len());
    }

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

    pub fn dedup_vertices(&mut self) {
        let mut new_positions = vec![];
        let mut new_normals = vec![];
        let mut new_tex_coords = vec![];
        let mut new_indices = vec![];

        let mut tuples: Vec<(Vec3<f64>, Vec3<f64>, Vec3<f64>)> = vec![];

        for x in self.indices.iter() {
            let p = self.positions.get(*x).unwrap();
            let n = self.normals.get(*x).unwrap();
            let t = self.tex_coords.get(*x).unwrap();

            let idx = tuples
                .iter()
                .position(|(it_p, _, _)| {
                    let epsilon = 0.0001;
                    (it_p.x - p.x).abs() < epsilon
                        && (it_p.y - p.y).abs() < epsilon
                        && (it_p.z - p.z).abs() < epsilon
                })
                .unwrap_or_else(|| {
                    tuples.push((*p, *t, *n));
                    new_positions.push(*p);
                    new_normals.push(*n);
                    new_tex_coords.push(*t);
                    tuples.len() - 1
                });
            new_indices.push(idx);
        }

        std::mem::replace(&mut self.positions, new_positions);
        std::mem::replace(&mut self.normals, new_normals);
        std::mem::replace(&mut self.tex_coords, new_tex_coords);
        std::mem::replace(&mut self.indices, new_indices);
    }

    pub fn to_obj(&self) -> String {
        let mut buff = String::with_capacity(8192);

        for x in self.positions.iter() {
            buff.push_str(&format!("v {} {} {}\n", x.x, x.y, x.z))
        }

        for x in self.tex_coords.iter() {
            buff.push_str(&format!("vt {} {}\n", x.x, x.y))
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
            /* the library will automatically convert polygons to triangles */
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
                        Vec3::new(v.x, v.y, v.z),
                        Vec3::new(n.x, n.y, n.z),
                        Vec3::new(t.u, t.v, t.w),
                    );
                }
            } else {
                panic!("non-triangle primitives are not supported");
            }
        }
        g
    }
}
