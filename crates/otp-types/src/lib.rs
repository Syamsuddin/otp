// SPDX-License-Identifier: GPL-3.0-only
#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Int(u8, bool),
    Real,
    Bool,
    Char,
    Str(usize),
    Array(i64, i64, Box<Type>),
    Record(Vec<(String, Type)>),
    Text,
    Void,
}
impl Type {
    pub fn encode(&self) -> String {
        match self {
            Self::Int(b, s) => format!("i{b}{}", if *s { 's' } else { 'u' }),
            Self::Real => "f".into(),
            Self::Bool => "b".into(),
            Self::Char => "c".into(),
            Self::Str(n) => format!("s{n};"),
            Self::Text => "t".into(),
            Self::Void => "v".into(),
            Self::Array(l, h, t) => format!("a{l},{h};{}", t.encode()),
            Self::Record(fs) => {
                let mut s = format!("r{};", fs.len());
                for (n, t) in fs {
                    s += &format!("{}:{n}{}", n.len(), t.encode());
                }
                s
            }
        }
    }
    pub fn decode(s: &str) -> Result<Self, String> {
        let mut p = Decoder {
            s: s.as_bytes(),
            i: 0,
        };
        let t = p.ty()?;
        if p.i != p.s.len() {
            return Err("trailing type data".into());
        }
        Ok(t)
    }
    pub fn integer() -> Self {
        Self::Int(16, true)
    }
    pub fn numeric(&self) -> bool {
        matches!(self, Self::Int(..) | Self::Real)
    }
    pub fn ordinal(&self) -> bool {
        matches!(self, Self::Int(..) | Self::Bool | Self::Char)
    }
    pub fn textlike(&self) -> bool {
        matches!(self, Self::Str(_) | Self::Char)
    }
    pub fn accepts(&self, other: &Self) -> bool {
        self == other
            || matches!(
                (self, other),
                (Self::Int(..), Self::Int(..))
                    | (Self::Real, Self::Int(..))
                    | (Self::Str(_), Self::Str(_))
                    | (Self::Str(_), Self::Char)
                    | (Self::Char, Self::Str(1))
            )
    }
}
struct Decoder<'a> {
    s: &'a [u8],
    i: usize,
}
impl Decoder<'_> {
    fn num(&mut self, end: u8) -> Result<i64, String> {
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i] != end {
            self.i += 1;
        }
        if self.i == self.s.len() {
            return Err("truncated type".into());
        }
        let n = std::str::from_utf8(&self.s[start..self.i])
            .map_err(|e| e.to_string())?
            .parse()
            .map_err(|_| "invalid type number")?;
        self.i += 1;
        Ok(n)
    }
    fn ty(&mut self) -> Result<Type, String> {
        let ch = *self.s.get(self.i).ok_or("missing type")?;
        self.i += 1;
        Ok(match ch {
            b'i' => {
                let start = self.i;
                while self.s.get(self.i).map_or(false, |c| c.is_ascii_digit()) {
                    self.i += 1;
                }
                let b = std::str::from_utf8(&self.s[start..self.i])
                    .unwrap()
                    .parse::<u8>()
                    .map_err(|_| "bad integer")?;
                let sign = *self.s.get(self.i).ok_or("missing sign")?;
                self.i += 1;
                if ![8, 16, 32].contains(&b) || ![b's', b'u'].contains(&sign) {
                    return Err("invalid integer".into());
                }
                Type::Int(b, sign == b's')
            }
            b'f' => Type::Real,
            b'b' => Type::Bool,
            b'c' => Type::Char,
            b't' => Type::Text,
            b'v' => Type::Void,
            b's' => {
                let n = self.num(b';')?;
                if !(0..=255).contains(&n) {
                    return Err("string length".into());
                }
                Type::Str(n as usize)
            }
            b'a' => {
                let l = self.num(b',')?;
                let h = self.num(b';')?;
                if h < l || h - l > 65535 {
                    return Err("array bounds".into());
                }
                Type::Array(l, h, Box::new(self.ty()?))
            }
            b'r' => {
                let n = self.num(b';')?;
                if !(0..=4096).contains(&n) {
                    return Err("record size".into());
                }
                let mut fs = vec![];
                for _ in 0..n {
                    let len = self.num(b':')? as usize;
                    let end = self.i.checked_add(len).ok_or("field length")?;
                    let name = std::str::from_utf8(self.s.get(self.i..end).ok_or("field data")?)
                        .map_err(|e| e.to_string())?
                        .to_string();
                    self.i = end;
                    fs.push((name, self.ty()?));
                }
                Type::Record(fs)
            }
            _ => return Err("unknown type".into()),
        })
    }
}
/// Turbo Pascal six-byte Real storage. Arithmetic currently uses f64 intermediates.
#[derive(Clone, Copy, Debug)]
pub struct Real48(pub [u8; 6]);
impl Real48 {
    pub fn from_f64(x: f64) -> Result<Self, String> {
        if !x.is_finite() {
            return Err("invalid real operation".into());
        }
        if x == 0.0 {
            return Ok(Self([0; 6]));
        }
        let sign = x.is_sign_negative();
        let a = x.abs();
        let mut e = a.log2().floor() as i32;
        if e < -128 {
            return Ok(Self([0; 6]));
        }
        if e > 126 {
            return Err("floating point overflow".into());
        }
        let mut mant = ((a / 2f64.powi(e) - 1.0) * 549755813888.0).round() as u64;
        if mant == (1u64 << 39) {
            e += 1;
            mant = 0;
        }
        if e > 126 {
            return Err("floating point overflow".into());
        }
        let mut b = [0; 6];
        b[0] = (e + 129) as u8;
        for i in 0..5 {
            b[i + 1] = (mant >> (8 * i)) as u8;
        }
        if sign {
            b[5] |= 128;
        }
        Ok(Self(b))
    }
    pub fn to_f64(self) -> f64 {
        let b = self.0;
        if b[0] == 0 {
            return 0.0;
        }
        let mut m = 0u64;
        for i in 0..5 {
            m |= ((if i == 4 { b[i + 1] & 127 } else { b[i + 1] }) as u64) << (8 * i);
        }
        let x = (1.0 + m as f64 / 549755813888.0) * 2f64.powi(b[0] as i32 - 129);
        if b[5] & 128 != 0 {
            -x
        } else {
            x
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layouts() {
        assert_eq!(Real48::from_f64(1.0).unwrap().0, [129, 0, 0, 0, 0, 0]);
        assert_eq!(Real48::from_f64(-1.5).unwrap().0, [129, 0, 0, 0, 0, 192]);
        for t in [
            Type::Str(255),
            Type::Array(-2, 9, Box::new(Type::integer())),
            Type::Record(vec![("x".into(), Type::Real)]),
        ] {
            assert_eq!(Type::decode(&t.encode()).unwrap(), t)
        }
    }
}
