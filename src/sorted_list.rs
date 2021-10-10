use std::cmp::Ordering;
use std::fmt;
use std::ops::{Deref, DerefMut};

pub struct SortedList<E> {
    inner: Vec<E>,
    cmp_f: Box<dyn Fn(&E, &E) -> bool + Send + Sync + 'static>,
}

impl<E: fmt::Debug> fmt::Debug for SortedList<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<E> SortedList<E>
where
    E: PartialOrd + 'static,
{
    pub fn new() -> Self {
        let cmp = Box::new(Self::cmp_e);

        Self {
            inner: Vec::new(),
            cmp_f: cmp,
        }
    }

    fn cmp_e(o1: &E, o2: &E) -> bool {
        o1 < o2
    }
}

impl<E> SortedList<E> {
    pub fn new_with_cmp<F>(cmp: F) -> Self
    where
        F: Fn(&E, &E) -> bool + Send + Sync + 'static,
    {
        let cmp = Box::new(cmp);
        Self {
            inner: Vec::new(),
            cmp_f: cmp,
        }
    }

    pub fn new_on_field<F, T>(get_field: F) -> Self
    where
        T: PartialOrd + 'static,
        F: Fn(&E) -> T + Send + Sync + 'static,
    {
        let cmp = Box::new(move |o1: &E, o2: &E| get_field(o1) < get_field(o2));

        Self {
            inner: Vec::new(),
            cmp_f: cmp,
        }
    }

    pub fn insert(&mut self, el: E) -> usize {
        let mut min = 0;
        let mut max = self.inner.len();

        while min != max {
            let index = (min + max) / 2;

            if (self.cmp_f)(&el, &self.inner[index]) {
                max = index;
            } else if (self.cmp_f)(&self.inner[index], &el) {
                min = index + 1;
            } else {
                min = index;
                max = index;
            }
        }

        self.inner.insert(min, el);
        return min;
    }

    pub fn with_inner(mut self, mut inner: Vec<E>) -> Self {
        let cmp_f = |o1: &E, o2: &E| {
            if (self.cmp_f)(o1, o2) {
                Ordering::Less
            } else if (self.cmp_f)(o2, o1) {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        };
        inner.sort_unstable_by(cmp_f);
        self.inner = inner;

        self
    }
}

impl<E> Deref for SortedList<E> {
    type Target = Vec<E>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<E> DerefMut for SortedList<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod test {
    use super::SortedList;

    #[test]
    fn test_empty() {
        let list: SortedList<usize> = SortedList::new();

        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_single_insert() {
        let mut list: SortedList<usize> = SortedList::new();
        list.insert(1);
        assert_eq!(list.len(), 1);
        assert_eq!(list.inner, vec![1]);
    }

    #[test]
    fn test_two_insert() {
        let mut list: SortedList<usize> = SortedList::new();
        list.insert(1);
        list.insert(2);
        assert_eq!(list.len(), 2);
        assert_eq!(list.inner, vec![1, 2]);
    }

    #[test]
    fn test_two_q_insert() {
        let mut list: SortedList<usize> = SortedList::new();
        list.insert(2);
        list.insert(1);
        assert_eq!(list.len(), 2);
        assert_eq!(list.inner, vec![1, 2]);
    }

    #[test]
    fn test_four_insert() {
        let mut list: SortedList<usize> = SortedList::new();
        list.insert(2);
        list.insert(1);
        list.insert(1);
        list.insert(3);
        assert_eq!(list.len(), 4);
        assert_eq!(list.inner, vec![1, 1, 2, 3]);
    }

    #[test]
    fn test_four_q_insert() {
        let mut list: SortedList<usize> = SortedList::new();
        list.insert(2);
        list.insert(1);
        list.insert(3);
        list.insert(1);
        list.insert(0);
        list.insert(6);
        assert_eq!(list.len(), 6);
        assert_eq!(list.inner, vec![0, 1, 1, 2, 3, 6]);
    }

    #[test]
    fn test_four_inv_insert() {
        let mut list: SortedList<usize> = SortedList::new_with_cmp((|x, y| y < x));
        list.insert(2);
        list.insert(1);
        list.insert(3);
        list.insert(1);
        list.insert(0);
        list.insert(6);
        assert_eq!(list.len(), 6);
        assert_eq!(list.inner, vec![6, 3, 2, 1, 1, 0]);
    }

    #[test]
    fn test_four_on_field_insert() {
        let mut list: SortedList<(usize, usize)> =
            SortedList::new_on_field(|x: &(usize, usize)| x.1);
        list.insert((0, 2));
        list.insert((1, 1));
        list.insert((0, 3));
        list.insert((1, 1));
        list.insert((0, 0));
        list.insert((4, 6));
        assert_eq!(list.len(), 6);
        assert_eq!(
            list.inner,
            vec![(0, 0), (1, 1), (1, 1), (0, 2), (0, 3), (4, 6)]
        );
    }
}
