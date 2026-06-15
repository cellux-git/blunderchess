use std::fmt;

pub type Bitboard = u64;

pub const MAX_DEPTH: u8 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Square(u8);

impl Square {
    pub const A1: Square = Square(0);
    pub const B1: Square = Square(1);
    pub const C1: Square = Square(2);
    pub const D1: Square = Square(3);
    pub const E1: Square = Square(4);
    pub const F1: Square = Square(5);
    pub const G1: Square = Square(6);
    pub const H1: Square = Square(7);
    pub const A8: Square = Square(56);
    pub const B8: Square = Square(57);
    pub const C8: Square = Square(58);
    pub const D8: Square = Square(59);
    pub const E8: Square = Square(60);
    pub const F8: Square = Square(61);
    pub const G8: Square = Square(62);
    pub const H8: Square = Square(63);

    #[inline]
    pub const fn new(index: u8) -> Option<Square> {
        if index < 64 { Some(Square(index)) } else { None }
    }

    #[inline]
    pub const fn from_file_rank(file: u8, rank: u8) -> Option<Square> {
        if file < 8 && rank < 8 { Some(Square(rank * 8 + file)) } else { None }
    }

    #[inline]
    pub const fn index(&self) -> u8 { self.0 }

    #[inline]
    pub const fn file(&self) -> u8 { self.0 & 7 }

    #[inline]
    pub const fn rank(&self) -> u8 { self.0 >> 3 }

    #[inline]
    pub const fn bit(&self) -> u64 { 1u64 << self.0 }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = (b'a' + self.file()) as char;
        let rank = (b'1' + self.rank()) as char;
        write!(f, "{file}{rank}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Piece {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl Piece {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    #[inline]
    pub const fn flip(self) -> Color {
        match self { Color::White => Color::Black, Color::Black => Color::White }
    }

    #[inline]
    pub const fn index(self) -> usize { self as usize }
}

// Move packed into 16 bits:
//   bits 0-5:   from square (6)
//   bits 6-11:  to square (6)
//   bits 12-13: promo (2)  — 0=Queen, 1=Knight, 2=Bishop, 3=Rook
//   bits 14-15: kind (2)   — 0=Normal, 1=Capture, 2=Castle, 3=Promotion
// En passant is detected in make_move: pawn-capture to empty ep square.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Normal = 0,
    Capture = 1,
    Castle = 2,
    Promotion = 3,
}

impl Move {
    pub const NULL: Move = Move(0);

    pub fn new(from: Square, to: Square) -> Move {
        let raw = (from.index() as u16)
            | ((to.index() as u16) << 6);
        Move(raw)
    }

    pub fn capture(from: Square, to: Square) -> Move {
        let raw = (from.index() as u16)
            | ((to.index() as u16) << 6)
            | (1u16 << 14);
        Move(raw)
    }

    pub fn castle(from: Square, to: Square) -> Move {
        let raw = (from.index() as u16)
            | ((to.index() as u16) << 6)
            | (2u16 << 14);
        Move(raw)
    }

    pub fn promotion(from: Square, to: Square, piece: Piece) -> Move {
        let promo_bits: u16 = match piece {
            Piece::Knight => 1,
            Piece::Bishop => 2,
            Piece::Rook => 3,
            _ => 0,
        };
        let raw = (from.index() as u16)
            | ((to.index() as u16) << 6)
            | (promo_bits << 12)
            | (3u16 << 14);
        Move(raw)
    }

    pub fn ep(from: Square, to: Square) -> Move {
        // stored as capture; unmake detects ep via en-passant square
        Move::capture(from, to)
    }

    #[inline]
    pub fn from(&self) -> Square { Square((self.0 & 0x3F) as u8) }

    #[inline]
    pub fn to(&self) -> Square { Square(((self.0 >> 6) & 0x3F) as u8) }

    #[inline]
    pub fn kind(&self) -> MoveKind {
        match (self.0 >> 14) & 0x3 {
            0 => MoveKind::Normal,
            1 => MoveKind::Capture,
            2 => MoveKind::Castle,
            _ => MoveKind::Promotion,
        }
    }

    #[inline]
    pub fn promotion_piece(&self) -> Option<Piece> {
        if self.kind() != MoveKind::Promotion { return None; }
        match (self.0 >> 12) & 0x3 {
            0 => Some(Piece::Queen),
            1 => Some(Piece::Knight),
            2 => Some(Piece::Bishop),
            _ => Some(Piece::Rook),
        }
    }

    #[inline]
    pub fn raw(&self) -> u16 { self.0 }

    #[inline]
    pub(crate) fn from_raw(raw: u16) -> Move { Move(raw) }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.from(), self.to())?;
        if let Some(p) = self.promotion_piece() {
            let c = match p {
                Piece::Knight => 'n',
                Piece::Bishop => 'b',
                Piece::Rook => 'r',
                Piece::Queen => 'q',
                _ => ' ',
            };
            write!(f, "{c}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastlingRights {
    pub white_kingside: bool,
    pub white_queenside: bool,
    pub black_kingside: bool,
    pub black_queenside: bool,
}

impl CastlingRights {
    pub const ALL: CastlingRights = CastlingRights {
        white_kingside: true, white_queenside: true,
        black_kingside: true, black_queenside: true,
    };

    pub const NONE: CastlingRights = CastlingRights {
        white_kingside: false, white_queenside: false,
        black_kingside: false, black_queenside: false,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_packing_normal() {
        let from = Square::from_file_rank(1, 0).unwrap();
        let to = Square::from_file_rank(3, 0).unwrap();
        let mv = Move::new(from, to);
        assert_eq!(mv.from(), from);
        assert_eq!(mv.to(), to);
        assert_eq!(mv.kind(), MoveKind::Normal);
        assert_eq!(mv.promotion_piece(), None);
    }

    #[test]
    fn test_move_packing_capture() {
        let from = Square::from_file_rank(4, 3).unwrap();
        let to = Square::from_file_rank(4, 4).unwrap();
        let mv = Move::capture(from, to);
        assert_eq!(mv.from(), from);
        assert_eq!(mv.to(), to);
        assert_eq!(mv.kind(), MoveKind::Capture);
        assert_eq!(mv.promotion_piece(), None);
    }

    #[test]
    fn test_move_packing_castle() {
        let from = Square::E1;
        let to = Square::G1;
        let mv = Move::castle(from, to);
        assert_eq!(mv.from(), from);
        assert_eq!(mv.to(), to);
        assert_eq!(mv.kind(), MoveKind::Castle);
        assert_eq!(mv.promotion_piece(), None);
    }

    #[test]
    fn test_move_packing_promotions() {
        let from = Square::from_file_rank(4, 6).unwrap();
        let to = Square::from_file_rank(4, 7).unwrap();
        let pieces = [Piece::Queen, Piece::Knight, Piece::Bishop, Piece::Rook];
        for &piece in &pieces {
            let mv = Move::promotion(from, to, piece);
            assert_eq!(mv.from(), from);
            assert_eq!(mv.to(), to);
            assert_eq!(mv.kind(), MoveKind::Promotion);
            assert_eq!(mv.promotion_piece(), Some(piece));
        }
    }

    #[test]
    fn test_square_new() {
        assert_eq!(Square::new(0), Some(Square::A1));
        assert_eq!(Square::new(63), Some(Square::H8));
        assert_eq!(Square::new(64), None);
    }

    #[test]
    fn test_square_from_file_rank() {
        assert_eq!(Square::from_file_rank(0, 0), Some(Square::A1));
        assert_eq!(Square::from_file_rank(7, 7), Some(Square::H8));
        assert_eq!(Square::from_file_rank(8, 0), None);
        assert_eq!(Square::from_file_rank(0, 8), None);
    }

    #[test]
    fn test_color_flip() {
        assert_eq!(Color::White.flip(), Color::Black);
        assert_eq!(Color::Black.flip(), Color::White);
    }

    #[test]
    fn test_move_null_sentinel() {
        let null_move = Move::NULL;
        assert_eq!(null_move.from(), Square::A1);
        assert_eq!(null_move.to(), Square::A1);
        assert_eq!(null_move.kind(), MoveKind::Normal);
        assert_eq!(null_move.raw(), 0);
    }

    #[test]
    fn test_castling_rights_all() {
        let all = CastlingRights::ALL;
        assert!(all.white_kingside);
        assert!(all.white_queenside);
        assert!(all.black_kingside);
        assert!(all.black_queenside);
    }

    #[test]
    fn test_castling_rights_none() {
        let none = CastlingRights::NONE;
        assert!(!none.white_kingside);
        assert!(!none.white_queenside);
        assert!(!none.black_kingside);
        assert!(!none.black_queenside);
    }

    #[test]
    fn test_castling_rights_remove() {
        let mut rights = CastlingRights::ALL;
        rights.white_kingside = false;
        assert!(!rights.white_kingside);
        assert!(rights.white_queenside);
        assert!(rights.black_kingside);
        assert!(rights.black_queenside);
    }

    #[test]
    fn test_display_square() {
        assert_eq!(Square::A1.to_string(), "a1");
        assert_eq!(Square::H8.to_string(), "h8");
        assert_eq!(Square::from_file_rank(4, 1).unwrap().to_string(), "e2");
    }

    #[test]
    fn test_display_move() {
        let from = Square::from_file_rank(4, 1).unwrap();
        let to = Square::from_file_rank(4, 3).unwrap();
        let mv = Move::new(from, to);
        assert_eq!(mv.to_string(), "e2e4");

        let promo = Move::promotion(
            Square::from_file_rank(4, 6).unwrap(),
            Square::from_file_rank(4, 7).unwrap(),
            Piece::Queen,
        );
        assert_eq!(promo.to_string(), "e7e8q");
    }
}
