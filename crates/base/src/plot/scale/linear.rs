// @reference: https://d3js.org/d3-scale/linear

use itertools::Itertools;
use num_traits::{Num, ToPrimitive};

use super::{Scale, sealed::Sealed};

#[derive(Clone)]
pub struct ScaleLinear<T> {
    domain_len: usize,
    domain_start: T,
    domain_diff: T,
    range_start: f32,
    range_diff: f32,
}

impl<T> ScaleLinear<T>
where
    T: Copy + PartialOrd + Num + ToPrimitive + Sealed,
{
    pub fn new(domain: Vec<T>, range: Vec<f32>) -> Self {
        let (domain_start, domain_end) = domain
            .iter()
            .minmax()
            .into_option()
            .map_or((T::zero(), T::zero()), |(min, max)| (*min, *max));

        let (range_start, range_end) =
            range
                .iter()
                .minmax()
                .into_option()
                .map_or((0., 0.), |(min, max)| {
                    let min_pos = range.iter().position(|&x| x == *min).unwrap_or(0);
                    let max_pos = range.iter().position(|&x| x == *max).unwrap_or(0);

                    if min_pos <= max_pos {
                        (*min, *max)
                    } else {
                        (*max, *min)
                    }
                });

        Self {
            domain_len: domain.len(),
            domain_start,
            domain_diff: domain_end - domain_start,
            range_start,
            range_diff: range_end - range_start,
        }
    }
}

impl<T> Scale<T> for ScaleLinear<T>
where
    T: Copy + PartialOrd + Num + ToPrimitive + Sealed,
{
    fn tick(&self, value: &T) -> Option<f32> {
        if self.domain_diff.is_zero() {
            return None;
        }

        let ratio = ((*value - self.domain_start) / self.domain_diff).to_f32()?;

        Some(ratio * self.range_diff + self.range_start)
    }

    fn least_index_with_domain(&self, tick: f32, domain: &[T]) -> (usize, f32) {
        if self.domain_len == 0 || domain.is_empty() {
            return (0, 0.);
        }

        domain
            .iter()
            .flat_map(|v| self.tick(v))
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                ((*a) - tick)
                    .abs()
                    .partial_cmp(&((*b) - tick).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or((0, 0.))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_linear() {
        let scale = ScaleLinear::new(vec![1., 2., 3.], vec![0., 100.]);
        assert_eq!(scale.tick(&1.), Some(0.));
        assert_eq!(scale.tick(&2.), Some(50.));
        assert_eq!(scale.tick(&3.), Some(100.));

        let scale = ScaleLinear::new(vec![1., 2., 3.], vec![100., 0.]);
        assert_eq!(scale.tick(&1.), Some(100.));
        assert_eq!(scale.tick(&2.), Some(50.));
        assert_eq!(scale.tick(&3.), Some(0.));
    }

    #[test]
    fn test_scale_linear_multiple_range() {
        let scale = ScaleLinear::new(vec![1., 2., 3.], vec![0., 50., 100.]);
        assert_eq!(scale.tick(&1.), Some(0.));
        assert_eq!(scale.tick(&2.), Some(50.));
        assert_eq!(scale.tick(&3.), Some(100.));

        let scale = ScaleLinear::new(vec![1., 2., 3.], vec![100., 50., 0.]);
        assert_eq!(scale.tick(&1.), Some(100.));
        assert_eq!(scale.tick(&2.), Some(50.));
        assert_eq!(scale.tick(&3.), Some(0.));

        let scale = ScaleLinear::new(vec![1., 2., 3.], vec![100., 0., 100.]);
        assert_eq!(scale.tick(&1.), Some(100.));
        assert_eq!(scale.tick(&2.), Some(50.));
        assert_eq!(scale.tick(&3.), Some(0.));

        let scale = ScaleLinear::new(vec![1., 2., 3.], vec![0., 100., 0.]);
        assert_eq!(scale.tick(&1.), Some(0.));
        assert_eq!(scale.tick(&2.), Some(50.));
        assert_eq!(scale.tick(&3.), Some(100.));
    }

    #[test]
    fn test_scale_linear_empty() {
        let scale = ScaleLinear::new(vec![], vec![0., 100.]);
        assert_eq!(scale.tick(&1.), None);
        assert_eq!(scale.tick(&2.), None);
        assert_eq!(scale.tick(&3.), None);

        let scale = ScaleLinear::new(vec![1., 2., 3.], vec![]);
        assert_eq!(scale.tick(&1.), Some(0.));
        assert_eq!(scale.tick(&2.), Some(0.));
        assert_eq!(scale.tick(&3.), Some(0.));
    }

    #[test]
    fn test_scale_linear_least_index_with_domain() {
        let scale = ScaleLinear::new(vec![1., 2., 3.], vec![0., 100.]);
        assert_eq!(scale.least_index_with_domain(0., &[1., 2., 3.]), (0, 0.));
        assert_eq!(scale.least_index_with_domain(50., &[1., 2., 3.]), (1, 50.));
        assert_eq!(
            scale.least_index_with_domain(100., &[1., 2., 3.]),
            (2, 100.)
        );
    }
}
