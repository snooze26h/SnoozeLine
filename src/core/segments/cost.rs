use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

#[derive(Default)]
pub struct CostSegment;

impl CostSegment {
    pub fn new() -> Self {
        Self
    }

    fn format_cost(cost: f64) -> String {
        if cost <= 0.0 {
            "$0.00".to_string()
        } else if cost < 0.01 {
            format!("${:.4}", cost)
        } else {
            format!("${:.2}", cost)
        }
    }
}

impl Segment for CostSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let cost_data = input.cost.as_ref()?;

        // Primary display: total cost
        let primary = Self::format_cost(cost_data.total_cost_usd?);

        // Secondary display: empty for cost segment
        let secondary = String::new();

        let mut metadata = HashMap::new();
        if let Some(cost) = cost_data.total_cost_usd {
            metadata.insert("cost".to_string(), cost.to_string());
        }

        Some(SegmentData {
            primary,
            secondary,
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Cost
    }
}

#[cfg(test)]
mod tests {
    use super::CostSegment;

    #[test]
    fn preserves_sub_cent_costs() {
        assert_eq!(CostSegment::format_cost(0.0), "$0.00");
        assert_eq!(CostSegment::format_cost(0.0049), "$0.0049");
        assert_eq!(CostSegment::format_cost(0.01), "$0.01");
    }
}
