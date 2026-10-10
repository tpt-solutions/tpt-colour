//! Eight-pixel batches for matrix/TRC transforms, built on `tpt-simd-vector`.
//!
//! Each lane performs exactly the operations the scalar path performs, in the same order, so
//! batched output is bit-identical to scalar output. The matrix products are vectorised. The
//! tone curves are evaluated per lane.

use tpt_simd_vector::Simd;

use crate::matrix::Mat3;

/// Lanes per batch.
pub(crate) const LANES: usize = 8;
/// One channel across a batch.
pub(crate) type Lanes = Simd<f32, LANES>;

/// `m × v` over lanes, with the same summation order as [`crate::matrix::mul_vec`].
#[must_use]
pub(crate) fn mat_lanes(m: &Mat3, v: [Lanes; 3]) -> [Lanes; 3] {
    let row = |r: usize| {
        Lanes::splat(m[r][0]) * v[0] + Lanes::splat(m[r][1]) * v[1] + Lanes::splat(m[r][2]) * v[2]
    };
    [row(0), row(1), row(2)]
}
