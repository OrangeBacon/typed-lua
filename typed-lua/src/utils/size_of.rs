use std::fmt::Display;

/// Get the size of a structure, including all included allocations
pub trait SizeOf {
    /// get the size of self
    fn size(&self) -> usize;
}

/// Helper for pretty printing byte sizes
pub struct Size(pub usize);

impl<T: SizeOf> SizeOf for Vec<T> {
    fn size(&self) -> usize {
        let alloc = self.capacity() * std::mem::size_of::<T>();
        let inner: usize = self.iter().map(|s| s.size()).sum();
        inner + alloc
    }
}

impl<T: SizeOf> SizeOf for Option<T> {
    fn size(&self) -> usize {
        self.as_ref().map(|t| t.size()).unwrap_or_default()
    }
}

impl<A: SizeOf, B: SizeOf> SizeOf for (A, B) {
    fn size(&self) -> usize {
        self.0.size() + self.1.size()
    }
}

impl<T: SizeOf> SizeOf for Box<T> {
    fn size(&self) -> usize {
        std::mem::size_of::<T>() + self.as_ref().size()
    }
}

impl<T: SizeOf> SizeOf for &T {
    fn size(&self) -> usize {
        T::size(self)
    }
}

impl SizeOf for &str {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for str {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for bool {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for u8 {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for usize {
    fn size(&self) -> usize {
        0
    }
}

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SUFFIX: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];

        let mut num = self.0 as f64;
        for suffix in SUFFIX {
            if num <= 1024.0 {
                return write!(f, "{num:.1} {suffix}");
            }
            num /= 1024.0;
        }

        write!(f, "{num:.1} PiB")
    }
}
