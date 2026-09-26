// @reference: https://d3js.org/d3-scale/point

use itertools::Itertools;

use super::Scale;

/// Point scale maps discrete domain values to continuous range positions.
///
/// Points are evenly distributed across the range, with the first and last points
/// aligned to the range boundaries.
#[derive(Clone)]
pub struct ScalePoint<T> {
    domain: Vec<T>,
    range_start: f32,
    range_tick: f32,
}

impl<T> ScalePoint<T>
where
    T: PartialEq,
{
    /// Creates a new point scale with the given domain and range.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let scale = ScalePoint::new(vec![1, 2, 3], vec![0., 100.]);
    /// assert_eq!(scale.tick(&1), Some(0.));
    /// assert_eq!(scale.tick(&2), Some(50.));
    /// assert_eq!(scale.tick(&3), Some(100.));
    /// ```
    pub fn new(domain: Vec<T>, range: Vec<f32>) -> Self {
        let len = domain.len();
        let (range_start, range_tick) = if len == 0 {
            (0., 0.)
        } else {
            let (min, max) = range
                .iter()
                .minmax()
                .into_option()
                .map_or((0., 0.), |(min, max)| (*min, *max));

            let range_diff = max - min;

            if len == 1 {
                (min, range_diff)
            } else {
                (min, range_diff / (len - 1) as f32)
            }
        };

        Self {
            domain,
            range_start,
            range_tick,
        }
    }
}

impl<T> Scale<T> for ScalePoint<T>
where
    T: PartialEq,
{
    fn tick(&self, value: &T) -> Option<f32> {
        let index = self.domain.iter().position(|v| v == value)?;

        if self.domain.len() == 1 {
            Some(self.range_start + self.range_tick / 2.)
        } else {
            Some(self.range_start + index as f32 * self.range_tick)
        }
    }

    fn least_index(&self, tick: f32) -> usize {
        if self.domain.is_empty() {
            return 0;
        }

        if self.range_tick == 0. {
            return 0;
        }

        let normalized_tick = tick - self.range_start;
        let index = (normalized_tick / self.range_tick).round() as usize;
        index.min(self.domain.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_point() {
        let scale = ScalePoint::new(vec![1, 2, 3], vec![0., 100.]);
        assert_eq!(scale.tick(&1), Some(0.));
        assert_eq!(scale.tick(&2), Some(50.));
        assert_eq!(scale.tick(&3), Some(100.));
    }

    #[test]
    fn test_scale_point_range() {
        let scale = ScalePoint::new(vec![1, 2, 3], vec![40., 80.]);
        assert_eq!(scale.tick(&1), Some(40.));
        assert_eq!(scale.tick(&2), Some(60.));
        assert_eq!(scale.tick(&3), Some(80.));
    }

    #[test]
    fn test_scale_point_empty() {
        let scale = ScalePoint::new(vec![], vec![0., 100.]);
        assert_eq!(scale.tick(&1), None);
        assert_eq!(scale.tick(&2), None);
        assert_eq!(scale.tick(&3), None);

        let scale = ScalePoint::new(vec![1, 2, 3], vec![]);
        assert_eq!(scale.tick(&1), Some(0.));
        assert_eq!(scale.tick(&2), Some(0.));
        assert_eq!(scale.tick(&3), Some(0.));
    }

    #[test]
    fn test_scale_point_single() {
        let scale = ScalePoint::new(vec![1], vec![0., 100.]);
        assert_eq!(scale.tick(&1), Some(50.));
    }

    #[test]
    fn test_least_index_basic() {
        let scale = ScalePoint::new(vec![1, 2, 3], vec![0., 100.]);

        // Exact positions
        assert_eq!(scale.least_index(0.), 0);
        assert_eq!(scale.least_index(50.), 1);
        assert_eq!(scale.least_index(100.), 2);

        // Between positions (should round to nearest)
        assert_eq!(scale.least_index(24.), 0); // closer to 0
        assert_eq!(scale.least_index(25.), 1); // equidistant, rounds to 1
        assert_eq!(scale.least_index(26.), 1); // closer to 50
        assert_eq!(scale.least_index(74.), 1); // closer to 50
        assert_eq!(scale.least_index(75.), 2); // equidistant, rounds to 2
        assert_eq!(scale.least_index(76.), 2); // closer to 100

        // Outside range
        assert_eq!(scale.least_index(-10.), 0); // below min
        assert_eq!(scale.least_index(150.), 2); // above max
    }

    #[test]
    fn test_least_index_with_offset() {
        let scale = ScalePoint::new(vec![1, 2, 3], vec![40., 80.]);

        // Exact positions: 40, 60, 80
        assert_eq!(scale.least_index(40.), 0);
        assert_eq!(scale.least_index(60.), 1);
        assert_eq!(scale.least_index(80.), 2);

        // Between positions
        assert_eq!(scale.least_index(49.), 0); // closer to 40
        assert_eq!(scale.least_index(50.), 1); // equidistant, rounds to 1
        assert_eq!(scale.least_index(51.), 1); // closer to 60
        assert_eq!(scale.least_index(69.), 1); // closer to 60
        assert_eq!(scale.least_index(70.), 2); // equidistant, rounds to 2
        assert_eq!(scale.least_index(71.), 2); // closer to 80

        // Outside range
        assert_eq!(scale.least_index(30.), 0); // below min
        assert_eq!(scale.least_index(100.), 2); // above max
    }

    #[test]
    fn test_least_index_empty() {
        let scale = ScalePoint::new(Vec::<i32>::new(), vec![0., 100.]);
        assert_eq!(scale.least_index(0.), 0);
        assert_eq!(scale.least_index(50.), 0);
        assert_eq!(scale.least_index(100.), 0);
    }

    #[test]
    fn test_least_index_single() {
        let scale = ScalePoint::new(vec![1], vec![0., 100.]);
        assert_eq!(scale.least_index(0.), 0);
        assert_eq!(scale.least_index(50.), 0);
        assert_eq!(scale.least_index(100.), 0);
    }

    #[test]
    fn test_least_index_empty_range() {
        let scale = ScalePoint::new(vec![1, 2, 3], vec![]);
        assert_eq!(scale.least_index(0.), 0);
        assert_eq!(scale.least_index(50.), 0);
        assert_eq!(scale.least_index(100.), 0);
    }
}
