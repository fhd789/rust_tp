use spin::Mutex;

pub struct Locked<T> {
    inner: Mutex<T>,
}

impl<T> Locked<T> {
    pub const fn new(inner: T) -> Self {
        Self { inner: Mutex::new(inner) }
    }

    pub fn lock(&self) -> spin::MutexGuard<'_, T> {
        self.inner.lock()
    }
}
