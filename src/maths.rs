use crate::direction::DirectionFlat;

fn rotate_matrix<T: Copy>(input: [[T; 3]; 3]) -> [[T; 3]; 3] {
    [0, 1, 2].map(|i| [0, 1, 2].map(|j| input[2 - j][i]))
}

fn flat_rotate<T: Copy>(input: [[[T; 3]; 3]; 3]) -> [[[T; 3]; 3]; 3] {
    input.map(|m| rotate_matrix(m))
}

/// Rotate the input to look at the given direction, assuming the input
/// looks at north
pub fn look_at<T: Copy>(input: [[[T; 3]; 3]; 3], direction: DirectionFlat) -> [[[T; 3]; 3]; 3] {
    let n = match direction {
        DirectionFlat::North => 0,
        DirectionFlat::East => 1,
        DirectionFlat::South => 2,
        DirectionFlat::West => 3,
    };
    let mut out = input;
    for _ in 0..n {
        out = flat_rotate(out);
    }
    out
}

pub trait RotatingMatrix {
    fn looking_at(self, direction: DirectionFlat) -> Self;
    fn rotated_by(self, amount: usize) -> Self;
}

impl<T: Copy> RotatingMatrix for [[[T; 3]; 3]; 3] {
    fn looking_at(self, direction: DirectionFlat) -> Self {
        look_at(self, direction)
    }

    fn rotated_by(self, amount: usize) -> Self {
        let mut out = self;
        for _ in 0..amount {
            out = flat_rotate(out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        assert_eq!(rotate_matrix(a), b);
    }

    #[test]
    fn test_flat_rotate() {
        #[rustfmt::skip]
        let a = [
            [
                [0, 1, 0],
                [0, 2, 0],
                [0, 3, 0],
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
                [0, 0, 0],
                [3, 2, 1],
                [0, 0, 0],
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

        assert_eq!(b, flat_rotate(a));
    }
}
