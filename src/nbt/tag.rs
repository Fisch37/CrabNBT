use bytes::{Buf, BufMut, Bytes, BytesMut};
use crab_nbt::error::Error;
use crab_nbt::nbt::compound::NbtCompound;
use crab_nbt::nbt::utils::*;
use derive_more::From;
use std::fmt::{self, Display, Formatter};
use std::io::Cursor;
use std::mem::{Discriminant, discriminant};
use std::str::FromStr;

use crate::impl_FromStr_through_FromVisitor;
use crate::nbt::de_utils::{FromVisitor, StrVisitor, char_may_be_unquoted, consume_whitespace, expect_char, read_slice_while, read_string};
use crate::nbt::error::SnbtDeserialisationError;

/// Enum representing the different types of NBT tags.
/// Each variant corresponds to a different type of data that can be stored in an NBT tag.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, PartialOrd, From)]
pub enum NbtTag {
    End = END_ID,
    Byte(i8) = BYTE_ID,
    Short(i16) = SHORT_ID,
    Int(i32) = INT_ID,
    Long(i64) = LONG_ID,
    Float(f32) = FLOAT_ID,
    Double(f64) = DOUBLE_ID,
    ByteArray(Bytes) = BYTE_ARRAY_ID,
    String(String) = STRING_ID,
    List(Vec<NbtTag>) = LIST_ID,
    Compound(NbtCompound) = COMPOUND_ID,
    IntArray(Vec<i32>) = INT_ARRAY_ID,
    LongArray(Vec<i64>) = LONG_ARRAY_ID,
}

impl NbtTag {
    /// Returns the numeric id associated with the data type.
    pub const fn get_type_id(&self) -> u8 {
        // See https://doc.rust-lang.org/reference/items/enumerations.html#pointer-casting
        unsafe { *(self as *const Self as *const u8) }
    }

    pub fn serialize(&self) -> Bytes {
        let mut bytes = BytesMut::new();
        bytes.put_u8(self.get_type_id());
        bytes.put(self.serialize_data());
        bytes.freeze()
    }

    pub fn serialize_data(&self) -> Bytes {
        let mut bytes = BytesMut::new();
        match self {
            NbtTag::End => {}
            NbtTag::Byte(byte) => bytes.put_i8(*byte),
            NbtTag::Short(short) => bytes.put_i16(*short),
            NbtTag::Int(int) => bytes.put_i32(*int),
            NbtTag::Long(long) => bytes.put_i64(*long),
            NbtTag::Float(float) => bytes.put_f32(*float),
            NbtTag::Double(double) => bytes.put_f64(*double),
            NbtTag::ByteArray(byte_array) => {
                bytes.put_i32(byte_array.len() as i32);
                bytes.put_slice(byte_array);
            }
            NbtTag::String(string) => {
                let java_string = simd_cesu8::encode(string);
                bytes.put_u16(java_string.len() as u16);
                bytes.put_slice(&java_string);
            }
            NbtTag::List(list) => {
                bytes.put_u8(list.first().unwrap_or(&NbtTag::End).get_type_id());
                bytes.put_i32(list.len() as i32);
                for nbt_tag in list {
                    bytes.put(nbt_tag.serialize_data())
                }
            }
            NbtTag::Compound(compound) => {
                bytes.put(compound.serialize_content());
            }
            NbtTag::IntArray(int_array) => {
                bytes.put_i32(int_array.len() as i32);
                for int in int_array {
                    bytes.put_i32(*int)
                }
            }
            NbtTag::LongArray(long_array) => {
                bytes.put_i32(long_array.len() as i32);
                for long in long_array {
                    bytes.put_i64(*long)
                }
            }
        }
        bytes.freeze()
    }

    pub fn deserialize(bytes: &mut impl Buf) -> Result<NbtTag, Error> {
        let tag_id = bytes.get_u8();
        Self::deserialize_data(bytes, tag_id)
    }

    pub fn deserialize_from_cursor(cursor: &mut Cursor<&[u8]>) -> Result<NbtTag, Error> {
        Self::deserialize(cursor)
    }

    pub fn deserialize_data(bytes: &mut impl Buf, tag_id: u8) -> Result<NbtTag, Error> {
        match tag_id {
            END_ID => Ok(NbtTag::End),
            BYTE_ID => {
                let byte = bytes.get_i8();
                Ok(NbtTag::Byte(byte))
            }
            SHORT_ID => {
                let short = bytes.get_i16();
                Ok(NbtTag::Short(short))
            }
            INT_ID => {
                let int = bytes.get_i32();
                Ok(NbtTag::Int(int))
            }
            LONG_ID => {
                let long = bytes.get_i64();
                Ok(NbtTag::Long(long))
            }
            FLOAT_ID => {
                let float = bytes.get_f32();
                Ok(NbtTag::Float(float))
            }
            DOUBLE_ID => {
                let double = bytes.get_f64();
                Ok(NbtTag::Double(double))
            }
            BYTE_ARRAY_ID => {
                let len = bytes.get_i32() as usize;
                let byte_array = bytes.copy_to_bytes(len);
                Ok(NbtTag::ByteArray(byte_array))
            }
            STRING_ID => Ok(NbtTag::String(get_nbt_string(bytes).unwrap())),
            LIST_ID => {
                let tag_type_id = bytes.get_u8();
                let len = bytes.get_i32();
                let mut list = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    let tag = NbtTag::deserialize_data(bytes, tag_type_id)?;
                    assert_eq!(tag.get_type_id(), tag_type_id);
                    list.push(tag);
                }
                Ok(NbtTag::List(list))
            }
            COMPOUND_ID => Ok(NbtTag::Compound(NbtCompound::deserialize_content(bytes)?)),
            INT_ARRAY_ID => {
                const BYTES: usize = size_of::<i32>();

                let len = bytes.get_i32() as usize;
                let numbers = read_array::<i32, BYTES, _>(bytes, len, i32::from_be_bytes);
                Ok(NbtTag::IntArray(numbers))
            }
            LONG_ARRAY_ID => {
                const BYTES: usize = size_of::<i64>();

                let len = bytes.get_i32() as usize;
                let numbers = read_array::<i64, BYTES, _>(bytes, len, i64::from_be_bytes);
                Ok(NbtTag::LongArray(numbers))
            }
            _ => Err(Error::UnknownTagId(tag_id)),
        }
    }

    pub fn deserialize_data_from_cursor(
        cursor: &mut Cursor<&[u8]>,
        tag_id: u8,
    ) -> Result<NbtTag, Error> {
        Self::deserialize_data(cursor, tag_id)
    }

    pub fn extract_byte(&self) -> Option<i8> {
        match self {
            NbtTag::Byte(byte) => Some(*byte),
            _ => None,
        }
    }

    pub fn extract_short(&self) -> Option<i16> {
        match self {
            NbtTag::Short(short) => Some(*short),
            _ => None,
        }
    }

    pub fn extract_int(&self) -> Option<i32> {
        match self {
            NbtTag::Int(int) => Some(*int),
            _ => None,
        }
    }

    pub fn extract_long(&self) -> Option<i64> {
        match self {
            NbtTag::Long(long) => Some(*long),
            _ => None,
        }
    }

    pub fn extract_float(&self) -> Option<f32> {
        match self {
            NbtTag::Float(float) => Some(*float),
            _ => None,
        }
    }

    pub fn extract_double(&self) -> Option<f64> {
        match self {
            NbtTag::Double(double) => Some(*double),
            _ => None,
        }
    }

    pub fn extract_bool(&self) -> Option<bool> {
        match self {
            NbtTag::Byte(byte) => Some(*byte != 0),
            _ => None,
        }
    }

    pub fn extract_byte_array(&self) -> Option<Bytes> {
        match self {
            // Note: Bytes are free to clone, so we can hand out an owned type
            NbtTag::ByteArray(byte_array) => Some(byte_array.clone()),
            _ => None,
        }
    }

    pub fn extract_string(&self) -> Option<&String> {
        match self {
            NbtTag::String(string) => Some(string),
            _ => None,
        }
    }

    pub fn extract_list(&self) -> Option<&Vec<NbtTag>> {
        match self {
            NbtTag::List(list) => Some(list),
            _ => None,
        }
    }

    pub fn extract_compound(&self) -> Option<&NbtCompound> {
        match self {
            NbtTag::Compound(compound) => Some(compound),
            _ => None,
        }
    }

    pub fn extract_int_array(&self) -> Option<&Vec<i32>> {
        match self {
            NbtTag::IntArray(int_array) => Some(int_array),
            _ => None,
        }
    }

    pub fn extract_long_array(&self) -> Option<&Vec<i64>> {
        match self {
            NbtTag::LongArray(long_array) => Some(long_array),
            _ => None,
        }
    }
}

impl From<&str> for NbtTag {
    fn from(value: &str) -> Self {
        NbtTag::String(value.to_string())
    }
}

impl From<&[u8]> for NbtTag {
    fn from(value: &[u8]) -> Self {
        NbtTag::ByteArray(Bytes::copy_from_slice(value))
    }
}

impl From<bool> for NbtTag {
    fn from(value: bool) -> Self {
        NbtTag::Byte(value as i8)
    }
}

impl Display for NbtTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::End => Ok(()),
            Self::Byte(x) => write!(f, "{x}b"),
            Self::Short(x) => write!(f, "{x}s"),
            Self::Int(x) => write!(f, "{x}"),
            Self::Long(x) => write!(f, "{x}L"),
            // using debug here matches Minecraft on whole numbers (3.0 instead of 3)
            Self::Float(x) => write!(f, "{x:?}f"),
            Self::Double(x) => write!(f, "{x:?}d"),
            Self::ByteArray(arr) => write_listlike(f, "B; ", "B", arr.iter().map(|b| *b as i8)),
            Self::String(s) => write!(f, "{}", escape_string_value(s)),
            Self::List(list) => write_listlike(f, "", "", list),
            Self::Compound(compound) => write!(f, "{compound}"),
            Self::IntArray(arr) => write_listlike(f, "I; ", "", arr),
            Self::LongArray(arr) => write_listlike(f, "L; ", "L", arr),
        }
    }
}

impl FromVisitor for NbtTag {
    type Err = SnbtDeserialisationError;

    fn from_visitor(visitor: &mut StrVisitor) -> Result<Self, Self::Err> {
        const TRUE: NbtTag = NbtTag::Byte(1);
        const FALSE: NbtTag = NbtTag::Byte(0);

        match visitor.peek().ok_or(
            SnbtDeserialisationError::eof("any SNBT character")
        )? {
            '{' => NbtCompound::from_visitor(visitor).map(NbtTag::Compound),
            '[' => {
                _ = visitor.next();
                consume_whitespace(visitor);
                if visitor.next_if(|c| c == ']').is_some() {
                    Ok(NbtTag::List(vec![]))
                } else if visitor.as_str().chars().nth(1).filter(|c| *c == ';').is_some() {
                    // we know visitor.nth(1) will be Some and StrVisitor is fused
                    // => visitor.next must be Some
                    match visitor.next().unwrap() {
                        'I' => read_snbt_array::<i32>(visitor).map(NbtTag::IntArray),
                        'B' => read_snbt_array::<i8>(visitor).map(
                            |arr| NbtTag::ByteArray(arr.into_iter().map(|b| b as u8).collect())
                        ),
                        'L' => read_snbt_array::<i64>(visitor).map(NbtTag::LongArray),
                        c => return Err(
                            SnbtDeserialisationError::unexpected("an array type identifier", c)
                        )
                    }
                } else {
                    // NOTE: This branch also triggers if visitor is fully consumed
                    read_list(visitor).map(NbtTag::List)
                }
            },
            c if is_number_character(c) => read_number_decimal(visitor),
            _ => {
                if match_expect_constant(visitor, "true") {
                    Ok(TRUE)
                } else if match_expect_constant(visitor, "false") {
                    Ok(FALSE)
                } else if match_expect_constant(visitor, "bool(") {
                    consume_whitespace(visitor);
                    todo!("Read number or read true/false");
                    consume_whitespace(visitor);
                    expect_char(visitor, ')')?;
                } else if match_expect_constant(visitor, "uuid(") {
                    consume_whitespace(visitor);
                    let tag = uuid_from_str(&read_string(visitor)?)
                        .map(NbtTag::IntArray);
                    
                    consume_whitespace(visitor);
                    expect_char(visitor, ')')?;
                    tag
                } else {
                    read_string(visitor).map(NbtTag::String)
                }
            }
        }
    }
}
impl_FromStr_through_FromVisitor!(NbtTag);

fn write_listlike<T: Display, I: IntoIterator<Item = T>>(
    f: &mut Formatter<'_>,
    prefix: &'static str,
    affix: &'static str,
    arr: I,
) -> fmt::Result {
    write!(f, "[{prefix}")?;
    join_formatted(
        f,
        ", ",
        arr.into_iter()
            .map(|x| move |f: &mut Formatter<'_>| write!(f, "{x}{affix}")),
    )?;
    write!(f, "]")
}

fn is_number_character(c: char) -> bool {
    c.is_ascii_digit() || c == '.'
}

fn match_expect_constant(visitor: &mut StrVisitor, name: &str) -> bool {
    let mut visitor_clone = visitor.clone();
    let mut name_length = 0usize;
    for match_char in name.chars() {
        let c = match visitor_clone.next() {
            None => return false,
            Some(x) => x
        };
        if c != match_char {
            return false;
        }
        name_length += 1;
    }
    let matches = visitor.peek().filter(|c| char_may_be_unquoted(*c)).is_none();
    if matches {
        for _ in 0..name_length {
            visitor.next();
        }
    }
    matches
}

fn uuid_from_str(s: &str) -> Result<Vec<i32>, SnbtDeserialisationError> {
    let uuid_parts: Vec<i64> = s.split('-')
            .map(|part| i64::from_str_radix(part, 16))
            .map(|res| res.unwrap_or_else(|_| todo!("Better error structures")))
            .collect();
    if uuid_parts.len() != 5 {
        Err(todo!("Better error structures"))
    } else {
        // c2-70-a8-46  
        //              c9-30  4c-9f
        //                            87-f3  ba-06-
        //                                         7e-3f-7c-72
        Ok(vec![
            uuid_parts[0] as i32,
            ((uuid_parts[1] << i16::BITS) + uuid_parts[2]) as i32,
            ((uuid_parts[3] << i16::BITS) + (uuid_parts[4] >> i32::BITS)) as i32,
            uuid_parts[5] as i32
        ])
    }
}


const LIST_SEPARATOR_MSG: &'static str = ", or ]";
/// Reads an SNBT List with correction for heterogeneous lists.
/// Assumes the opening `[` character has already been consumed.
fn read_list(visitor: &mut StrVisitor) -> Result<Vec<NbtTag>, SnbtDeserialisationError> {
    let mut content = vec![];
    let mut homogeneous_content_type: Option<Option<Discriminant<NbtTag>>> = None;
    loop {
        consume_whitespace(visitor);
        let tag = NbtTag::from_visitor(visitor)?;
        homogeneous_content_type = homogeneous_content_type.map_or(
            Some(Some(discriminant(&tag))),
            |list_content| 
            Some(list_content.filter(|d| *d == discriminant(&tag)))
        );
        content.push(tag);

        consume_whitespace(visitor);

        if visitor.next_if(|c| c == ',' || c == ']')
            .ok_or_else(|| {
                // FIXME: Horrible code (duplicates peek in next_if)
                //  Must be fixed once todo!("better error structures") is finished
                match visitor.peek() {
                    None => SnbtDeserialisationError::eof(LIST_SEPARATOR_MSG),
                    Some(c) => SnbtDeserialisationError::unexpected(LIST_SEPARATOR_MSG, c)
                }
            })? == ']'
        {
            break
        }
    }
    if homogeneous_content_type.unwrap_or(Some(discriminant(&NbtTag::End))).is_none() {
        // Heterogeneous List correction
        // Nbt does not allow hetereogeneous lists
        // (lists where each element may have a different type),
        // but SNBT does (since https://www.minecraft.net/en-us/article/minecraft-snapshot-25w09a).
        // 
        // Heterogenous lists must be deserialised into lists of type NbtCompound,
        // with each element contained in a compound like so {"": element}
        content = content.into_iter().map(|elem| {
            [(String::new(), elem)].into_iter().collect::<NbtCompound>().into()
        }).collect();
    } else {
        content.shrink_to_fit();
    }
    Ok(content)
}

/// Reads an SNBT array of type Number,
/// assuming the array identifier (e.g. `[I;`) has already been consumed.
fn read_snbt_array<Number>(
    visitor: &mut StrVisitor
) -> Result<Vec<Number>, SnbtDeserialisationError>
    where Number: FromStr
{
    let mut content = vec![];
    loop {
        consume_whitespace(visitor);
        content.push(
            read_slice_while(visitor, |c| c.is_ascii_digit() && c == '-')
                .parse().map_err(|_| todo!("better error structures"))?
        );

        consume_whitespace(visitor);
        if visitor.next_if(|c| c == ',' || c == ']').ok_or_else(
            || match visitor.peek() {
                None => SnbtDeserialisationError::eof(LIST_SEPARATOR_MSG),
                Some(c) => SnbtDeserialisationError::unexpected(LIST_SEPARATOR_MSG, c)
            }
        )? == ']' {
            break
        }
    }
    Ok(content)
}

fn read_number(visitor: &mut StrVisitor) -> Result<NbtTag, SnbtDeserialisationError> {
    let mut chars = visitor.as_str().chars();
    if chars.next().map(|c| c == '0').unwrap_or(false) {
        match chars.next() {
            Some('x' | 'X') => read_number_radix(visitor, 16),
            Some('b' | 'B')
                if chars.next().map_or(false, |c| c == '0' || c == '1')
                => read_number_radix(visitor, 2),
            _ => read_number_decimal(visitor)
        }
    } else {
        read_number_decimal(visitor)
    }
}

fn read_number_decimal(visitor: &mut StrVisitor) -> Result<NbtTag, SnbtDeserialisationError> {
    let mut read_decimal_point = false;
    let number_str = read_slice_while(visitor, |c| {
        // [0-9]|-|.|e|E
        if c == '.' {
            read_decimal_point = true;
        }
        c.is_ascii_digit() || c == '-' || c == '.' || c == 'e' || c == 'E'
    });
    
    match visitor.next() {
        Some('b' | 'B') => number_from_string(number_str, NbtTag::Byte),
        Some('s' | 'S') => number_from_string(number_str, NbtTag::Short),
        Some('i' | 'I') => number_from_string(number_str, NbtTag::Int),
        Some('l' | 'L') => number_from_string(number_str, NbtTag::Long),
        Some('f' | 'F') => number_from_string(number_str, NbtTag::Float),
        Some('d' | 'D') => number_from_string(number_str, NbtTag::Double),
        Some('u' | 'U') => todo!("Unsigned suffix"),
        _ => {
            // no number type identifier, no character should have been read
            // (it's easier and faster to undo here than to use next_if)
            visitor.previous();
            if read_decimal_point {
                number_from_string(number_str, NbtTag::Double)
            } else {
                number_from_string(number_str, NbtTag::Int)
            }
        }
    }
}

fn read_number_radix(
    visitor: &mut StrVisitor,
    radix: u32
) -> Result<NbtTag, SnbtDeserialisationError> {
    todo!("Radix numbers")
}

fn number_from_string<Number, M, T>(s: &str, mapper: M) -> Result<T, SnbtDeserialisationError>
    where Number: FromStr, M: FnOnce(Number) -> T
{
    s.parse().map_err(|_| todo!("better error structures")).map(mapper)
}

#[cfg(test)]
mod tests {
    use crate::{NbtTag, nbt::{de_utils::StrVisitor, tag::read_number}};

    fn number_helper(s: &str) -> NbtTag {
        read_number(&mut StrVisitor::new(s)).unwrap()
    }

    #[test]
    fn test_numbers() {
        assert_eq!(number_helper("1.5"), NbtTag::Double(1.5));
    }
}