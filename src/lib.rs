#![doc = include_str!("../README.md")]

pub mod error;
mod macros;
mod nbt;
#[cfg(feature = "serde")]
pub mod serde;
mod utils;

pub use crab_nbt::nbt::compound::NbtCompound;
pub use crab_nbt::nbt::list::NbtList;
pub use crab_nbt::nbt::nbt_trait::NbtCompatible;
pub use crab_nbt::nbt::tag::NbtTag;
pub use crab_nbt::nbt::Nbt;
pub use crab_nbt::utils::{TryAsMut, TryAsRef};
// Trick to allow &str in nbt! macro. Must be public.
pub use crab_nbt::macros::IntoNbtCompatible;

extern crate self as crab_nbt;
