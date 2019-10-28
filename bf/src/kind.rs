use std::convert::TryFrom;

/// Enum representing possible types of BF files.
#[derive(Debug)]
#[repr(u8)]
pub enum Kind {
    Image = 0,
    Geometry = 1,
    Audio = 2,
    Material = 3,
    VirtualFileSystem = 4,
    CompiledShader = 5,
    Scene = 6,
    /* this is not a Kind but rather a trick to have number of different kinds available */
    MaxValue = 7,
}

// todo: rather derive than manually implement
impl TryFrom<u8> for Kind {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Kind::Image),
            1 => Ok(Kind::Geometry),
            2 => Ok(Kind::Audio),
            3 => Ok(Kind::Material),
            4 => Ok(Kind::VirtualFileSystem),
            5 => Ok(Kind::CompiledShader),
            6 => Ok(Kind::Scene),
            _ => Err(()),
        }
    }
}
