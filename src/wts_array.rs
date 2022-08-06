use serde::ser::{Serialize, SerializeSeq, Serializer};
use windows::Win32::System::RemoteDesktop;

pub struct WtsArray<T> {
    data: *mut T,
    len: u32,
}

impl<T: Serialize> Serialize for WtsArray<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len as usize))?;
        for e in self.as_slice() {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

impl<T> WtsArray<T> {
    pub unsafe fn from_raw(data: *mut T, len: u32) -> Self {
        Self { data, len }
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn as_slice(&self) -> &[T] {
        if self.is_empty() {
            return &[];
        }

        unsafe { std::slice::from_raw_parts(self.data, self.len as usize) }
    }
}

impl<T> Drop for WtsArray<T> {
    fn drop(&mut self) {
        if self.is_empty() {
            return;
        }

        unsafe {
            std::ptr::drop_in_place(std::slice::from_raw_parts_mut(self.data, self.len as usize));
            RemoteDesktop::WTSFreeMemory(self.data as _);
        }
    }
}
