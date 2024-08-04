pub(crate) const unsafe fn u16_slice_from_ptr<'p>(ptr: *const u16) -> &'p [u16] {
    let mut len = 0;
    while *ptr.add(len) != 0u16 {
        len += 1
    }
    core::slice::from_raw_parts(ptr, len + 1)
}
