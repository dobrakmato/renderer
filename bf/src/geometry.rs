/// Enum representing possible types geometry lists.
pub enum GeometryListType {
    Positions = 0,
    Normals = 1,
    Tangents = 2,
    Colors = 3,
    UV1 = 4,
    UV2 = 5,
    UV3 = 6,
    UV4 = 7,
    Indices = 8,
}

pub struct GeometryList<'a> {
    kind: GeometryListType,
    length: usize,
    data: &'a [u8],
}
