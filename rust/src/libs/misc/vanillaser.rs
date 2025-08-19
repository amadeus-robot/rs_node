use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    Nil,
    Bool(bool),
    Int(i64),
    Binary(Vec<u8>),
    Atom(String),
    List(Vec<Term>),
    Map(BTreeMap<Term, Term>),
}

pub struct VanillaSer;

impl VanillaSer {
    pub fn validate(binary: &[u8]) -> Option<Term> {
        let (term, rest) = Self::decode(binary).ok()?;
        if rest.is_empty() && binary == Self::encode(&term).as_slice() {
            Some(term)
        } else {
            None
        }
    }

    pub fn encode(term: &Term) -> Vec<u8> {
        let mut acc = Vec::new();
        Self::encode_inner(term, &mut acc);
        acc
    }

    fn encode_inner(term: &Term, acc: &mut Vec<u8>) {
        match term {
            Term::Nil => acc.push(0),
            Term::Bool(true) => acc.push(1),
            Term::Bool(false) => acc.push(2),
            Term::Int(i) => {
                acc.push(3);
                Self::encode_varint(*i, acc);
            }
            Term::Binary(b) => {
                acc.push(5);
                Self::encode_varint(b.len() as i64, acc);
                acc.extend_from_slice(b);
            }
            Term::Atom(s) => {
                acc.push(5);
                Self::encode_varint(s.len() as i64, acc);
                acc.extend_from_slice(s.as_bytes());
            }
            Term::List(list) => {
                acc.push(6);
                Self::encode_varint(list.len() as i64, acc);
                for item in list {
                    Self::encode_inner(item, acc);
                }
            }
            Term::Map(map) => {
                acc.push(7);
                Self::encode_varint(map.len() as i64, acc);
                for (k, v) in map {
                    Self::encode_inner(k, acc);
                    Self::encode_inner(v, acc);
                }
            }
        }
    }

    pub fn decode(binary: &[u8]) -> Result<(Term, &[u8]), String> {
        if binary.is_empty() {
            return Err("Empty input".into());
        }
        let (first, rest) = binary.split_first().unwrap();
        match first {
            0 => Ok((Term::Nil, rest)),
            1 => Ok((Term::Bool(true), rest)),
            2 => Ok((Term::Bool(false), rest)),
            3 => {
                let (i, rest2) = Self::decode_varint(rest)?;
                Ok((Term::Int(i), rest2))
            }
            5 => {
                let (len, rest2) = Self::decode_varint(rest)?;
                let len = len as usize;
                if rest2.len() < len {
                    return Err("Binary/Atom too short".into());
                }
                let (payload, rest3) = rest2.split_at(len);
                Ok((Term::Binary(payload.to_vec()), rest3))
            }
            6 => {
                let (len, mut rest2) = Self::decode_varint(rest)?;
                let len = len as usize;
                let mut items = Vec::with_capacity(len);
                for _ in 0..len {
                    let (item, rest_next) = Self::decode(rest2)?;
                    items.push(item);
                    rest2 = rest_next;
                }
                Ok((Term::List(items), rest2))
            }
            7 => {
                let (len, mut rest2) = Self::decode_varint(rest)?;
                let len = len as usize;
                let mut map = BTreeMap::new();
                for _ in 0..len {
                    let (k, rest_next) = Self::decode(rest2)?;
                    let (v, rest_next2) = Self::decode(rest_next)?;
                    map.insert(k, v);
                    rest2 = rest_next2;
                }
                Ok((Term::Map(map), rest2))
            }
            _ => Err(format!("Unknown type byte: {}", first)),
        }
    }

    fn encode_varint(mut i: i64, acc: &mut Vec<u8>) {
        let sign = if i >= 0 { 0 } else { 1 };
        let abs_i = i.abs() as u64;
        let bytes = abs_i.to_be_bytes();
        let first_non_zero = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len() - 1);
        let payload = &bytes[first_non_zero..];
        acc.push((sign << 7 | payload.len() as u8) as u8);
        acc.extend_from_slice(payload);
    }

    fn decode_varint(binary: &[u8]) -> Result<(i64, &[u8]), String> {
        if binary.is_empty() {
            return Err("Empty for varint".into());
        }
        let (first, rest) = binary.split_first().unwrap();
        let sign = first >> 7;
        let len = (first & 0x7F) as usize;
        if rest.len() < len {
            return Err("Varint too short".into());
        }
        let (payload, rest2) = rest.split_at(len);
        let mut buf = [0u8; 8];
        buf[8 - len..].copy_from_slice(payload);
        let val = i64::from_be_bytes(buf);
        Ok((if sign == 0 { val } else { -val }, rest2))
    }
}
