use crate::storage::Storage;
use atomic_refcell::AtomicRefCell;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;

mod storage;

pub type Index = u32;

pub struct Entity(Index);

pub trait Component: Sized + Copy {
    type Storage: Storage<Self>;
}

pub trait System {
    type Data;

    fn process(data: Self::Data);
}

pub trait Resource: Any + Send + Sync + 'static {}

impl<T> Resource for T where T: Any + Send + Sync {}

#[derive(Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct ResourceId(TypeId);

impl ResourceId {
    pub fn new<T>() -> Self
    where
        T: Resource,
    {
        ResourceId(TypeId::of::<T>())
    }
}

pub struct World {
    container: HashMap<ResourceId, AtomicRefCell<Box<dyn Resource>>>,
}

impl World {
    fn get<T: Resource + Any>(&self) -> &T {
        let id = ResourceId::new::<T>();
        let cell = self.container.get(&id).unwrap();
        let s = cell.borrow().deref().deref();
        let opt = unsafe { (s as &dyn Any).downcast_ref(); }
    }
}

// ----

struct Transform {
    position: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
}

struct PhysicsObject {
    velocity: [f32; 3],
}

struct Physics;

impl System for Physics {
    type Data = (Transform, PhysicsObject);

    fn process(data: Self::Data) {
        let (mut transform, physics_object) = data;

        transform.position += physics_object.velocity;
    }
}
