///! Generic functions for shape management
///
/// It has a set of methods to build programmatically 3D boxes or 2D slices
use std::array;

use crate::direction::DirectionFlat;

/// A 3D box of base BxB and height H
pub type Box3D<T, const B: usize = 3, const H: usize = 5> = [[[T; B]; B]; H];

/// A flat 2D slice of size BxB
pub type Slice2D<T, const B: usize = 3> = [[T; B]; B];

/// Build a 3D box from a function
pub fn box_from_fn<T: Copy, const B: usize, const H: usize, F>(mut func: F) -> Box3D<T, B, H>
where
    F: FnMut(usize, usize, usize) -> T,
{
    array::from_fn(|z| array::from_fn(|y| array::from_fn(|x| func(x, y, H - z - 1))))
}

/// Build a 2D slice from a function
pub fn slice_from_fn<const B: usize, T: Copy, F>(mut func: F) -> Slice2D<T, B>
where
    F: FnMut(usize, usize) -> T,
{
    array::from_fn(|y| array::from_fn(|x| func(x, y)))
}

/// Build a constant 3D box
pub const fn box_const<T: Copy, const B: usize, const H: usize>(value: T) -> Box3D<T, B, H> {
    [[[value; B]; B]; H]
}

/// Completely empty 3D box
pub const fn box_empty<const B: usize, const H: usize>() -> Box3D<bool, B, H> {
    box_const(false)
}

/// Completely full 3D box
pub const fn box_full<const B: usize, const H: usize>() -> Box3D<bool, B, H> {
    box_const(true)
}

/// Build a 3D box from levels
///
/// The input is a 2D slice of levels, and the resulting box will have
/// vertical columns of the size given by the input 2D slice.
/// A value of 0 will lead to no block in that column, a value of N will lead to a full column
pub fn box_from_levels<const B: usize, const H: usize>(
    levels: Slice2D<usize, B>,
) -> Box3D<bool, B, H> {
    box_from_fn(|x, y, z| levels[y][x] > z)
}

/// Build a constant 2D slice
pub const fn slice_const<const B: usize, T: Copy>(value: T) -> Slice2D<T, B> {
    [[value; B]; B]
}

/// Completely full 2D slice
pub const fn slice_full<const B: usize>() -> Slice2D<bool, B> {
    slice_const(true)
}

/// Empty 2D slice
pub const fn slice_empty<const B: usize>() -> Slice2D<bool, B> {
    slice_const(false)
}

/// Rotate 90° a given 2D slice
fn slice_rotated<T: Copy, const B: usize>(input: Slice2D<T, B>) -> Slice2D<T, B> {
    std::array::from_fn(|i| std::array::from_fn(|j| input[(B - 1) - j][i]))
}

/// Rotate 90° a given 3D box
fn box_rotated<T: Copy, const B: usize, const H: usize>(input: Box3D<T, B, H>) -> Box3D<T, B, H> {
    input.map(|m| slice_rotated(m))
}

pub trait Rotating {
    /// Return a copy looking at the given direction, assuming
    /// the input was looking at north
    fn looking_at(self, direction: DirectionFlat) -> Self;

    /// Return a copy rotated by amount time 90 degrees
    fn rotated_by(self, amount: usize) -> Self;
}

impl<T: Copy, const B: usize, const H: usize> Rotating for Box3D<T, B, H> {
    fn looking_at(self, direction: DirectionFlat) -> Self {
        let n = match direction {
            DirectionFlat::North => 0,
            DirectionFlat::East => 1,
            DirectionFlat::South => 2,
            DirectionFlat::West => 3,
        };
        let mut out = self;
        for _ in 0..n {
            out = box_rotated(out);
        }
        out
    }

    fn rotated_by(self, amount: usize) -> Self {
        let mut out = self;
        for _ in 0..amount {
            out = box_rotated(out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_box_from_levels() {
        #[rustfmt::skip]
        let levels = [
            [0,1,2],
            [3,1,0],
            [2,3,2]
        ];

        #[rustfmt::skip]
        let result = [
            [
                [false, false, false],
                [true, false, false],
                [false, true, false],
            ],
            [
                [false, false, true],
                [true, false, false],
                [true, true, true]
            ],
            [
                [false, true, true],
                [true, true, false],
                [true, true, true],
            ],
        ];

        assert_eq!(result, box_from_levels(levels));
    }

    #[test]
    fn test_rotate2() {
        #[rustfmt::skip]
        let a = [
            [1,2],
            [3,4]
        ];

        #[rustfmt::skip]
        let b = [
            [3,1],
            [4,2],
        ];

        assert_eq!(slice_rotated(a), b);
    }

    #[test]
    fn test_rotate() {
        #[rustfmt::skip]
        let a = [
            [1,2,3],
            [4,5,6],
            [7,8,9]
        ];

        #[rustfmt::skip]
        let b = [
            [7,4,1],
            [8,5,2],
            [9,6,3]
        ];

        assert_eq!(slice_rotated(a), b);
    }

    #[test]
    fn test_flat_rotate() {
        #[rustfmt::skip]
        let a = [
            [
                [0, 1, 0],
                [0, 2, 0],
                [1, 3, 2],
            ],
            [
                [1, 1, 1],
                [1, 2, 1],
                [1, 3, 1],
            ],
            [
                [2, 1, 2],
                [2, 2, 2],
                [2, 3, 2],
            ],
        ];

        #[rustfmt::skip]
        let b = [
            [
                [1, 0, 0],
                [3, 2, 1],
                [2, 0, 0],
            ],
            [
                [1, 1, 1],
                [3, 2, 1],
                [1, 1, 1],
            ],
            [
                [2, 2, 2],
                [3, 2, 1],
                [2, 2, 2],
            ],
        ];

        assert_eq!(b, box_rotated(a));
    }
}
