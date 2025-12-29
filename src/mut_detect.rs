use core::ops::{Deref, DerefMut};

/// Mutation detection through [`DerefMut`] implementation.
#[derive(Default, Debug, Clone, Copy)]
pub struct MutDetect<T> {
    inner: T,
    mutated: bool,
}

impl<T> MutDetect<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            mutated: false,
        }
    }

    /// Whether the inner value has been mutated.
    pub fn mutated(&self) -> bool {
        self.mutated
    }

    // TODO: Do we need this?
    // /// Get a mutable reference to the inner value while bypassing
    // /// the mutation detection.
    // pub(crate) fn bypass_mut_detect(&mut self) -> &mut T {
    //     &mut self.inner
    // }

    /// Set mutated to `false` manually.
    pub(crate) fn reset_mutation(&mut self) {
        self.mutated = false;
    }
}

impl<T: PartialEq> MutDetect<T> {
    /// Set the inner value only if it is not the same.
    ///
    /// Mutated flag will only be turned on if values doesn't match.
    pub fn set_if_ne(&mut self, new_value: T) -> bool {
        if self.inner != new_value {
            self.inner = new_value;
            self.mutated = true;

            return true;
        }

        false
    }
}

impl<T> Deref for MutDetect<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for MutDetect<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mutated = true;
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deref_does_not_mark_mutated() {
        let m = MutDetect::new(42);

        // Deref.
        let _ = *m;
        assert!(!m.mutated());
    }

    #[test]
    fn deref_mut_marks_mutated() {
        let mut m = MutDetect::new(1);

        // DerefMut.
        *m += 1;
        assert!(m.mutated());
        assert_eq!(*m, 2);
    }

    // #[test]
    // fn bypass_mut_detect_does_not_mark_mutated() {
    //     let mut m = MutDetect::new(10);

    //     *m.bypass_mut_detect() += 5;
    //     assert!(!m.mutated());
    //     assert_eq!(*m, 15);
    // }

    #[test]
    fn reset_clears_mutated_flag() {
        let mut m = MutDetect::new(0);

        *m += 1;
        assert!(m.mutated());

        m.reset_mutation();
        assert!(!m.mutated());
    }

    #[test]
    fn multiple_mutations_stay_marked() {
        let mut m = MutDetect::new(1);

        *m += 1;
        *m += 1;

        assert!(m.mutated());
        assert_eq!(*m, 3);
    }
}
