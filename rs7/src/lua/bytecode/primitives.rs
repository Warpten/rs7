use bytes::Buf;

pub fn read_cstring<R>(data: &mut R) -> Option<String>
where
    R: Buf,
{
    let mut str = vec![];
    loop {
        match data.get_u8() {
            0 => break,
            value => str.push(value),
        };
    }

    String::from_utf8(str).ok()
}

pub fn read_string<R>(data: &mut R, size: usize) -> String
where
    R: Buf,
{
    let mut s = String::with_capacity(size);

    unsafe {
        let buf = s.as_mut_vec();
        buf.set_len(size);

        data.copy_to_slice(&mut buf[..size]);
    }

    s
}
