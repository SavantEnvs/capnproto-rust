// KAT oracle probe: drives the UNMODIFIED capnp library's public API on FIXED
// inputs and prints exact known-answer values. mayhem/test.sh greps these lines
// verbatim from bash (whitelisted by the sabotage shim), so a neutered library
// (or a patched-in no-op) fails the oracle loudly.
//
// Exercises exactly the API surface the three fuzz targets hit:
//   - serialize::write_message_to_words / read_message  (framed stream format)
//   - message canonicalization (canonicalize / is_canonical)
//   - serialize_packed round trip (the packing compression layer)
//   - schema-typed field access via the generated TestAllTypes code
//   - rejection of a malformed segment table

capnp::generated_code!(pub mod test_capnp);

use capnp::{message, serialize, serialize_packed};
use test_capnp::{test_all_types, TestEnum};

fn build_fixed_message() -> message::Builder<message::HeapAllocator> {
    let mut builder = message::Builder::new_default();
    {
        let mut root = builder.init_root::<test_all_types::Builder>();
        root.set_bool_field(true);
        root.set_int8_field(-123);
        root.set_int16_field(-12345);
        root.set_int32_field(-12345678);
        root.set_int64_field(-123456789012345);
        root.set_u_int8_field(234);
        root.set_u_int16_field(45678);
        root.set_u_int32_field(3456789012);
        root.set_u_int64_field(12345678901234567890);
        root.set_float32_field(1234.5);
        root.set_float64_field(-1.23e45);
        root.set_text_field("Hello, Cap'n Proto KAT!");
        root.set_data_field(b"\x00\x01\x02\xfe\xff");
        root.set_enum_field(TestEnum::Corge);
        {
            let mut il = root.reborrow().init_int32_list(4);
            il.set(0, 7);
            il.set(1, -8);
            il.set(2, 9000);
            il.set(3, -100000);
        }
        {
            let mut tl = root.reborrow().init_text_list(2);
            tl.set(0, "alpha");
            tl.set(1, "omega");
        }
    }
    builder
}

fn main() {
    let builder = build_fixed_message();

    // 1. Framed serialization of the fixed message: exact byte length.
    let bytes = serialize::write_message_to_words(&builder);
    println!("KAT1 serialized_len={}", bytes.len());

    // 2. Deserialize and read back exact field values through generated code.
    let reader = serialize::read_message(&mut &bytes[..], Default::default()).unwrap();
    let root = reader.get_root::<test_all_types::Reader>().unwrap();
    println!(
        "KAT2 int64={} uint32={} uint64={} enum_ok={}",
        root.get_int64_field(),
        root.get_u_int32_field(),
        root.get_u_int64_field(),
        root.get_enum_field().unwrap() == TestEnum::Corge
    );
    println!(
        "KAT3 text={} float32={} float64={}",
        root.get_text_field().unwrap().to_str().unwrap(),
        root.get_float32_field(),
        root.get_float64_field()
    );
    let il = root.get_int32_list().unwrap();
    let ints: Vec<String> = (0..il.len()).map(|i| il.get(i).to_string()).collect();
    let tl = root.get_text_list().unwrap();
    let texts: Vec<String> = (0..tl.len())
        .map(|i| tl.get(i).unwrap().to_str().unwrap().to_string())
        .collect();
    println!("KAT4 int32list={} textlist={}", ints.join(","), texts.join(","));

    // 3. Canonicalization: exact canonical word count, and canonical output
    //    re-reads as canonical (the same invariant the canonicalize fuzzer asserts).
    let canonical_words = reader.canonicalize().unwrap();
    let seg = &[capnp::Word::words_to_bytes(&canonical_words[..])];
    let canonical_reader =
        message::Reader::new(message::SegmentArray::new(seg), Default::default());
    println!(
        "KAT5 canonical_words={} canonical_stable={}",
        canonical_words.len(),
        canonical_reader.is_canonical().unwrap()
    );

    // 4. Packed round trip: exact packed length, and fields survive unpacking.
    let mut packed = Vec::new();
    serialize_packed::write_message(&mut packed, &builder).unwrap();
    let unpacked = serialize_packed::read_message(&mut &packed[..], Default::default()).unwrap();
    let uroot = unpacked.get_root::<test_all_types::Reader>().unwrap();
    println!(
        "KAT6 packed_len={} packed_text={} packed_int16={}",
        packed.len(),
        uroot.get_text_field().unwrap().to_str().unwrap(),
        uroot.get_int16_field()
    );

    // 5. A malformed segment table must be REJECTED (error, not success/crash).
    let bad: &[u8] = &[0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00];
    let rejected = serialize::read_message(&mut &bad[..], Default::default()).is_err();
    println!("KAT7 reject={rejected}");
}
