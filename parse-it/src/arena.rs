use std::any::Any;
use std::cell::{Cell, OnceCell};
use std::ops::Index;
use std::rc::{Rc, Weak};

pub struct Arena<const N: usize> {
    inner: Rc<ArenaInner<N>>,
}

pub struct ArenaInner<const N: usize> {
    alloc: Cell<usize>,
    slots: [OnceCell<Box<dyn Any>>; N],
}

impl<const N: usize> Default for Arena<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Arena<N> {
    pub fn new() -> Self {
        let slots = [const { OnceCell::new() }; N];
        let alloc = Cell::new(0);
        let inner = Rc::new(ArenaInner { alloc, slots });
        Self { inner }
    }

    pub fn alloc<P>(&self) -> Slot<N, P> {
        let index = self.inner.alloc.get();
        if index >= N {
            panic!("internal error: arena full");
        }
        self.inner.alloc.set(index + 1);

        Slot {
            arena: Rc::downgrade(&self.inner),
            index,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn inner(&self) -> Rc<dyn Any> {
        self.inner.clone()
    }
}

impl<const N: usize> Index<usize> for ArenaInner<N> {
    type Output = OnceCell<Box<dyn Any>>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.slots[index]
    }
}

pub struct Slot<const N: usize, P> {
    arena: Weak<ArenaInner<N>>,
    index: usize,
    _phantom: std::marker::PhantomData<P>,
}

impl<const N: usize, P> Clone for Slot<N, P> {
    fn clone(&self) -> Self {
        Slot {
            arena: self.arena.clone(),
            index: self.index,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<const N: usize, P: 'static> Slot<N, P> {
    fn arena(&self) -> Rc<ArenaInner<N>> {
        self.arena
            .upgrade()
            .expect("internal error: arena already dropped")
    }

    pub fn store(&self, parser: P) {
        self.arena()[self.index]
            .set(Box::new(parser))
            .unwrap_or_else(|_| panic!("internal error: slot already occupied"));
    }

    pub fn with<T>(&self, f: impl FnOnce(&P) -> T) -> T {
        let arena = self.arena();
        let value = arena[self.index]
            .get()
            .expect("internal error: slot not occupied")
            .downcast_ref::<P>()
            .expect("internal error: slot has wrong type");
        f(value)
    }
}
