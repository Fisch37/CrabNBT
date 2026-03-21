use std::str::FromStr as _;

use crab_nbt::{NbtCompound, NbtTag, nbt};

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
fn nbt_compound() {
    assert_eq!(
        NbtCompound::from_str("{components:[]}").unwrap(),
        nbt!("", {
            "components": []
        }).into()
    )
}

#[test]
fn big_data_parser() {
    let input = include_str!("data/bigdata.snbt");
    let nbt = NbtCompound::from_str(input).unwrap();
    println!("{nbt}");
}
