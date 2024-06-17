use std::{
    any::Any,
    cell::{OnceCell, RefCell},
    rc::{Rc, Weak},
};

pub struct Arena {
    slots: RefCell<Vec<OnceCell<Rc<dyn Any>>>>,
}

impl Arena {
    pub fn new() -> Rc<Self> {
        Rc::new(Arena {
            slots: RefCell::new(Vec::new()),
        })
    }

    pub fn alloc<P>(self: &Rc<Self>) -> Slot<P> {
        let index = {
            let mut slots = self.slots.borrow_mut();
            slots.push(OnceCell::new());
            slots.len() - 1
        };

        Slot {
            arena: Rc::downgrade(self),
            index,
            _phantom: std::marker::PhantomData,
        }
    }
}

pub struct Slot<P> {
    arena: Weak<Arena>,
    index: usize,
    _phantom: std::marker::PhantomData<P>,
}

impl<P> Clone for Slot<P> {
    fn clone(&self) -> Self {
        Slot {
            arena: self.arena.clone(),
            index: self.index,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<P: 'static> Slot<P> {
    pub fn store(&self, parser: P) {
        self.arena.upgrade().unwrap().slots.borrow()[self.index]
            .set(Rc::new(parser))
            .unwrap_or_else(|_| panic!("internal error: slot already occupied"));
    }

    pub fn get(&self) -> Rc<P> {
        self.arena.upgrade().unwrap().slots.borrow()[self.index]
            .get()
            .expect("internal error: slot not occupied")
            .clone()
            .downcast()
            .unwrap()
    }
}
