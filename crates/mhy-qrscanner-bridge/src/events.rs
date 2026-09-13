//! Step reporting for multi-step operations.
//!
//! The bridge does not use callbacks: every operation returns its
//! [`StepReport`] list in the result DTO. That keeps the FFI surface plain data
//! (no trait objects), which `flutter_rust_bridge` cannot express.

use crate::dto::StepReport;

/// Build a step report and append it to `out`.
pub(crate) fn push_step(
    out: &mut Vec<StepReport>,
    index: u32,
    total: u32,
    name: &str,
    detail: impl Into<String>,
    ok: bool,
) {
    out.push(StepReport {
        index,
        total,
        name: name.to_string(),
        detail: detail.into(),
        ok,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_step_appends_in_order() {
        let mut steps = Vec::new();
        push_step(&mut steps, 1, 2, "a", "first", true);
        push_step(&mut steps, 2, 2, "b", "second", false);
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].index, 1);
        assert_eq!(steps[1].name, "b");
        assert!(!steps[1].ok);
    }
}
