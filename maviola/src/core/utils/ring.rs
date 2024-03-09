use std::mem;

/// Circular contingent buffer.
pub struct RingBuffer<T> {
    buffer: Vec<Option<T>>,
    capacity: usize,
    len: usize,
    start: usize,
}

/// Iterator over [`RingBuffer`].
pub struct RingBufferIterator<'a, T> {
    ring: &'a RingBuffer<T>,
    cursor: usize,
}

impl<T> RingBuffer<T> {
    /// Creates a new [`RingBuffer`].
    pub fn new(capacity: usize) -> RingBuffer<T> {
        let buffer = Vec::with_capacity(capacity);

        RingBuffer {
            buffer,
            capacity,
            len: 0,
            start: 0,
        }
    }

    /// Returns `true` if buffer is full.
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.len == self.capacity
    }

    /// Returns `true` if buffer is empty.
    #[inline(always)]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns number of elements in a buffer.
    #[inline(always)]
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns buffer's capacity.
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Pushes new element to a buffer.
    ///
    /// Returns optional element that was pushed out from the buffer.
    ///
    /// If buffer has zero capacity, then the same element will be returned.
    pub fn push(&mut self, value: T) -> Option<T> {
        if self.capacity() == 0 {
            return Some(value);
        }

        let pos = (self.start + self.len) % self.capacity;

        if pos < self.capacity {
            self.buffer.push(None);
        }

        let mut pushed_out = Some(value);
        mem::swap(&mut pushed_out, &mut self.buffer[pos]);

        if self.is_full() {
            if self.start == self.capacity - 1 {
                self.start = 0;
            } else {
                self.start += 1;
            }
        } else {
            self.len += 1;
        }

        pushed_out
    }

    /// Pulls back the first element of a buffer.
    #[allow(dead_code)]
    pub fn pull_back(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let mut value = None;
        mem::swap(&mut value, &mut self.buffer[self.start]);

        if self.start == self.capacity - 1 {
            self.start = 0;
        } else {
            self.start += 1;
        }

        self.len -= 1;

        value
    }

    /// Changes capacity of the ring buffer, returns elements, that do not fit.
    #[allow(dead_code)]
    pub fn resize(&mut self, capacity: usize) -> impl Iterator<Item = T> {
        if capacity == self.capacity {
            return Vec::new().into_iter();
        }

        if capacity > self.capacity {
            let mut buffer = Vec::with_capacity(capacity);
            while let Some(item) = self.pull_back() {
                buffer.push(Some(item));
            }

            self.buffer = buffer;
            self.start = capacity - 1;
            self.capacity = capacity;

            return Vec::new().into_iter();
        }

        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            let item = self.pull_back();
            buffer.push(item);
        }

        let mut remaining = Vec::new();
        while let Some(item) = self.pull_back() {
            remaining.push(item);
        }

        self.buffer = buffer;
        self.capacity = capacity;
        self.len = capacity;
        self.start = 0;

        remaining.into_iter()
    }

    /// Returns an iterator over a buffer.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        RingBufferIterator::new(self)
    }
}

impl<T> Default for RingBuffer<T> {
    /// Creates an empty [`RingBuffer`] with `0` capacity.
    fn default() -> Self {
        Self::new(0)
    }
}

impl<'a, T> RingBufferIterator<'a, T> {
    /// Creates a new iterator over a [`RingBuffer`].
    pub fn new(ring: &'a RingBuffer<T>) -> Self {
        Self { ring, cursor: 0 }
    }
}

impl<'a, T> Iterator for RingBufferIterator<'a, T> {
    type Item = &'a T;

    /// Returns next element of a [`RingBuffer`] or [`None`].
    fn next(&mut self) -> Option<&'a T> {
        if self.cursor == self.ring.len {
            return None;
        }

        let pos = (self.ring.start + self.cursor) % self.ring.capacity;
        self.cursor += 1;

        let element = &self.ring.buffer[pos];
        element.as_ref()
    }
}

///////////////////////////////////////////////////////////////////////////////
//                                  Tests                                    //
///////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_basics() {
        let mut ring = RingBuffer::new(2);

        ring.push(1);
        ring.push(2);
        ring.push(3);

        assert_eq!(ring.pull_back().unwrap(), 2);
        assert_eq!(ring.pull_back().unwrap(), 3);
        assert!(ring.pull_back().is_none());
    }

    #[test]
    fn ring_zero_elements() {
        let mut ring = RingBuffer::new(0);

        assert!(matches!(ring.push(1), Some(1)));
        assert!(matches!(ring.push(2), Some(2)));
        assert!(matches!(ring.push(3), Some(3)));

        assert!(ring.pull_back().is_none());
    }

    #[test]
    fn ring_iterator() {
        let mut ring = RingBuffer::new(2);
        ring.push(1);
        ring.push(2);

        let mut iter = ring.iter();
        assert!(matches!(iter.next(), Some(1)));
        assert!(matches!(iter.next(), Some(2)));

        let mut items = 0;
        for _ in ring.iter() {
            items += 1;
        }
        assert_eq!(items, 2);
    }

    #[test]
    fn extend_ring_capacity() {
        let mut ring = RingBuffer::new(2);

        assert!(matches!(ring.push(1), None));
        assert!(matches!(ring.push(2), None));

        _ = ring.resize(3);

        assert!(matches!(ring.push(3), None));
        assert!(matches!(ring.push(4), Some(1)));
    }
}
