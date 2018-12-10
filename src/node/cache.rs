use std::collections::{VecDeque, HashSet};
use std::hash::Hash;
use std::rc::Rc;

pub struct Cache<T> {
    vec: VecDeque<Rc<T>>,
    set: HashSet<Rc<T>>,
    maxsize: usize,
}

impl<T> Cache<T>
where T: Hash + Eq
{
    pub fn new(maxsize: usize) -> Self {
        assert!(maxsize > 0);
        Cache{vec: VecDeque::new(),
              set: HashSet::new(),
              maxsize: maxsize}
    }
    pub fn contains(&self, x: &T) -> bool {
        self.set.contains(x)
    }
    pub fn insert(&mut self, x: T) -> bool {
        if self.contains(&x) {
            return false;
        }

        if self.vec.len() >= self.maxsize {
            let temp = self.vec.pop_front().unwrap();
            self.set.remove(&temp);
        }
        let r = Rc::new(x);
        self.vec.push_back(r.clone());
        self.set.insert(r);
        true
    }
}
