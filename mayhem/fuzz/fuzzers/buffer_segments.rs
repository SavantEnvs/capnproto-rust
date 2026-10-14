#![no_main]

use capnp::{message, serialize};
use libfuzzer_sys::{fuzz_target, Corpus};

// 8-byte-aligned copy of the fuzz input, so reads get past the alignment check.
struct Aligned {
    words: Vec<capnp::Word>,
    len: usize,
}

impl core::ops::Deref for Aligned {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &capnp::Word::words_to_bytes(&self.words[..])[..self.len]
    }
}

fn try_go(data: &[u8]) -> capnp::Result<()> {
    let mut words = capnp::Word::allocate_zeroed_vec(data.len().div_ceil(8));
    capnp::Word::words_to_bytes_mut(&mut words[..])[..data.len()].copy_from_slice(data);
    let buffer = Aligned {
        words,
        len: data.len(),
    };
    let segments = serialize::BufferSegments::new(buffer, Default::default())?;
    let reader = message::Reader::new(segments, Default::default());
    reader.is_canonical()?;
    reader.canonicalize()?;
    Ok(())
}

fuzz_target!(|data: &[u8]| -> Corpus {
    match try_go(data) {
        Ok(()) => Corpus::Keep,
        Err(_) => Corpus::Reject,
    }
});
