use crate::Index;
use std::collections::HashMap;

pub trait Storage<T> {
    fn get(&self, index: Index) -> Option<&T>;
    fn get_mut(&mut self, index: Index) -> Option<&mut T>;
    fn insert(&mut self, index: Index, t: T);
    fn remove(&mut self, index: Index) -> Option<T>;
}

#[derive(Default)]
pub struct VecStorage<T>(Vec<Option<T>>);

impl<T> Storage<T> for VecStorage<T> {
    fn get(&self, index: u32) -> Option<&T> {
        self.0.get(index as usize).map(|x| x.as_ref().unwrap())
    }

    fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        self.0.get_mut(index as usize).map(|x| x.as_mut().unwrap())
    }

    fn insert(&mut self, index: u32, t: T) {
        let idx = index as usize;

        if self.0.len() <= idx {
            self.0.resize_with(idx + 1, || None)
        }

        self.0[index as usize] = Some(t);
    }

    fn remove(&mut self, index: u32) -> Option<T> {
        self.0.get_mut(index as usize).unwrap().take()
    }
}

#[derive(Default)]
pub struct DenseStorage<T> {
    sparse: Vec<u32>,
    sparse_back: Vec<u32>,
    dense: Vec<Option<T>>,
}

impl<T> Storage<T> for DenseStorage<T> {
    fn get(&self, index: u32) -> Option<&T> {
        let dense_idx = self.sparse.get(index as usize).unwrap();
        self.dense
            .get(*dense_idx as usize)
            .map(|x| x.as_ref().unwrap())
    }

    fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        let dense_idx = self.sparse.get(index as usize).unwrap();
        self.dense
            .get_mut(*dense_idx as usize)
            .map(|x| x.as_mut().unwrap())
    }

    fn insert(&mut self, index: u32, t: T) {
        let idx = index as usize;

        if self.sparse.len() <= idx {
            self.sparse.resize_with(idx + 1, u32::max_value);
        }

        self.sparse[idx] = self.dense.len() as u32;
        self.sparse_back.push(index);
        self.dense.push(Some(t));
    }

    fn remove(&mut self, index: u32) -> Option<T> {
        let last_idx = self.dense.len() - 1;
        let last = self.dense.get_mut(last_idx).unwrap().take();
        let last_sparse_idx = self.sparse_back[last_idx];

        let dense_idx = *self.sparse.get(index as usize).unwrap();
        let removed = self
            .dense
            .get_mut(dense_idx as usize)
            .unwrap()
            .replace(last.unwrap());
        self.sparse[index as usize] = u32::max_value();
        self.sparse[last_sparse_idx as usize] = dense_idx;

        self.dense.remove(self.dense.len() - 1);

        removed
    }
}

#[derive(Default)]
pub struct HashMapStorage<T>(HashMap<Index, T>);

impl<T> Storage<T> for HashMapStorage<T> {
    fn get(&self, index: u32) -> Option<&T> {
        self.0.get(&index)
    }

    fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        self.0.get_mut(&index)
    }

    fn insert(&mut self, index: u32, t: T) {
        self.0.insert(index, t);
    }

    fn remove(&mut self, index: u32) -> Option<T> {
        self.0.remove(&index)
    }
}

#[derive(Default)]
pub struct NullStorage<T>(T);

impl<T> Storage<T> for NullStorage<T> {
    fn get(&self, _: u32) -> Option<&T> {
        Some(&self.0)
    }

    fn get_mut(&mut self, _: u32) -> Option<&mut T> {
        Some(&mut self.0)
    }

    fn insert(&mut self, _: u32, _: T) {}

    fn remove(&mut self, _: u32) -> Option<T> {
        None
    }
}
