use std::fmt::{Debug, Formatter};

pub trait ClonableFn<P, R>: Fn(P) -> R {
    fn clone_box<'a>(&self) -> Box<dyn 'a + ClonableFn<P, R>>
    where
        Self: 'a;
}

impl <F, P, R> ClonableFn<P, R> for F where F: Fn(P) -> R + Clone {
    fn clone_box<'a>(&self) -> Box<dyn 'a + ClonableFn<P, R>> where F: 'a {
        Box::new(self.clone())
    }
}

impl<'a, F: 'a, R: 'a> Clone for Box<dyn 'a + ClonableFn<F, R>> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

impl<F, R> Debug for Box<dyn ClonableFn<F, R>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Not the best debug output, lacks detail, but it's alright for our purposes
        write!(f, "Box<dyn ClonableFn<F, R>>")
    }
}