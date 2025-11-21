use std::str::FromStr as _;

use crab_nbt::{Nbt, NbtCompound};

#[test]
fn nbt_tag() {
    assert_eq!(
        Nbt::from_str("{my_tag:}").unwrap(),
        Nbt::new("my_tag".to_string(), NbtCompound::new())
    );
    assert_eq!(
        Nbt::from_str("{\"minecraft:unbreakable\":}").unwrap(),
        Nbt::new("minecraft:unbreakable".to_string(), NbtCompound::new())
    )
}
