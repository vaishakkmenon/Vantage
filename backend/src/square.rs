use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Square(u8);

impl Square {
    pub fn from_index(idx: u8) -> Self {
        assert!(idx < 64, "Square index out of range");
        Square(idx)
    }

    pub fn index(self) -> u8 {
        self.0
    }

    /// Returns the rank (0-7, where 0 is rank 1 and 7 is rank 8)
    #[inline(always)]
    pub fn rank(self) -> u8 {
        self.0 / 8
    }

    /// Returns the file (0-7, where 0 is 'a' and 7 is 'h')
    #[inline(always)]
    pub fn file(self) -> u8 {
        self.0 % 8
    }

    /// Creates a Square from file and rank indices (both 0-7)
    #[inline(always)]
    pub fn from_file_rank(file: u8, rank: u8) -> Self {
        assert!(file < 8 && rank < 8, "File and rank must be 0-7");
        Square(rank * 8 + file)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = self.0 % 8;
        let rank = self.0 / 8;
        write!(f, "{}{}", (b'a' + file) as char, rank + 1)
    }
}

impl FromStr for Square {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        if bytes.len() != 2 {
            return Err(format!("Invalid square length: {}", s));
        }
        let mut file = bytes[0];
        let rank = bytes[1];

        if (b'A'..=b'H').contains(&file) {
            file += b'a' - b'A';
        }

        if !(b'a'..=b'h').contains(&file) {
            return Err(format!("Invalid file: {}", s));
        }

        if !(b'1'..=b'8').contains(&rank) {
            return Err(format!("Invalid rank: {}", s));
        }

        let file_idx = file - b'a';
        let rank_idx = rank - b'1';
        Ok(Square(rank_idx * 8 + file_idx))
    }
}

impl TryFrom<u8> for Square {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 64 {
            Ok(Square(value))
        } else {
            Err(format!("Invalid square index: {}", value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;
    use std::str::FromStr;

    #[test]
    fn test_parse_lowercase() {
        let sq = Square::from_str("e4").unwrap();
        assert_eq!(sq.index(), 28);
    }

    #[test]
    fn test_parse_uppercase() {
        let sq = Square::from_str("E4").unwrap();
        assert_eq!(sq.index(), 28);
    }

    #[test]
    fn test_parse_invalid_file() {
        assert!("z4".parse::<Square>().is_err());
    }

    #[test]
    fn test_parse_invalid_rank() {
        assert!("e9".parse::<Square>().is_err());
    }

    #[test]
    fn test_display() {
        let sq = Square::from_index(28);
        assert_eq!(sq.to_string(), "e4");
    }

    #[test]
    fn test_try_from_valid_index() {
        let sq = Square::try_from(28).unwrap();
        assert_eq!(sq.to_string(), "e4");
    }

    #[test]
    fn test_try_from_invalid_index() {
        let result = Square::try_from(99);
        assert!(result.is_err());
    }
}
