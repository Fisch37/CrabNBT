use std::str::FromStr as _;

use bytes::Bytes;
use crab_nbt::{NbtCompound, NbtTag, nbt};

macro_rules! assert_parse {
    ($input:expr, $expected:expr) => {
        assert_eq!($input.parse(), Ok($expected))
    };
}

fn tag_helper(s: &str) -> NbtTag {
    NbtTag::from_str(s).unwrap()
}

fn wrap(tag: NbtTag) -> NbtTag {
    NbtCompound { child_tags: vec![(String::new(), tag)] }.into()
}

#[test]
fn nbt_tag() {
    assert_eq!(tag_helper("Hello"), NbtTag::String("Hello".to_owned()));
}

#[test]
fn nbt_list() {
    assert_eq!(tag_helper("[]"), NbtTag::List(vec![]));
    assert_eq!(
        tag_helper("[A,B,C ,D,      E,    F    ,   G    ]"),
        NbtTag::List("ABCDEFG".chars().map(|c| NbtTag::String(c.to_string())).collect())
    );
    assert_eq!(
        tag_helper("[A,[],B,{}]"),
        NbtTag::List(vec![
            wrap(NbtTag::String("A".to_string())),
            wrap(NbtTag::List(vec![])),
            wrap(NbtTag::String("B".to_string())),
            wrap(NbtTag::Compound(NbtCompound::new()))
        ])
    );
    assert_eq!(
        tag_helper("[\"A\", \"B\", C, \"D\", E]"),
        NbtTag::List(
            "ABCDE".chars()
                .map(|c| c.to_string())
                .map(NbtTag::String)
                .collect()
        )
    );
}

#[test]
fn nbt_uuids() {
    assert_parse!(
        "uuid(\"7c3be0c5-6abd-4fa5-b3f1-0634daa981df\")",
        NbtTag::IntArray(vec![2084298949, 1790791589, -1276049868, -626425377])
    )
}

#[test]
fn nbt_arrays() {
    assert_parse!(
        "[I; 2084298949, 1790791589, -1276049868, -626425377]",
        NbtTag::IntArray(vec![2084298949, 1790791589, -1276049868, -626425377])
    );
    assert_parse!("[B;]", NbtTag::ByteArray(Bytes::new()));
}

#[test]
fn nbt_compound() {
    assert_eq!(
        NbtCompound::from_str("{components:[]}").unwrap(),
        nbt!("", {
            "components": []
        }).into()
    )
}

#[test]
fn nbt_numbers() {
    assert_parse!("1e7f", NbtTag::Float(1e7));
    assert_parse!("1e7", NbtTag::Double(1e7));
    assert_parse!("10", NbtTag::Int(10));
    assert_parse!("-25", NbtTag::Int(-25));
    assert_parse!("5b", NbtTag::Byte(5));
    assert_parse!("-128b", NbtTag::Byte(-128));
    assert_parse!("127b", NbtTag::Byte(127));
    assert_parse!("50l", NbtTag::Long(50));
    assert_parse!("50L", NbtTag::Long(50));

    assert_parse!("255UB", NbtTag::Byte(-1));
    assert_parse!("10SB", NbtTag::Byte(10));

    // assert_parse!("0x1f", NbtTag::Int(0x1f));
    
    const TRUE: NbtTag = NbtTag::Byte(1);
    const FALSE: NbtTag = NbtTag::Byte(0);
    assert_parse!("bool(500)", TRUE);
    assert_parse!("bool(0)", FALSE);
    assert_parse!("bool(true)", TRUE);
    assert_parse!("bool(false)", FALSE);
}

#[test]
fn big_data_parser() {
    let input = include_str!("data/bigdata.snbt");
    let nbt = NbtCompound::from_str(input).unwrap();
    // assert_eq!(nbt.to_string(), input);
}
