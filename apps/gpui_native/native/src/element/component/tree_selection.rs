//! Selection operations independent of focus and file activation.
#[derive(Default, Clone, Debug)]
pub(crate) struct TreeSelection {
    pub(crate) ids: Vec<String>,
    pub(crate) anchor: Option<String>,
}

impl TreeSelection {
    pub(crate) fn single(&mut self, id: &str) {
        self.ids = vec![id.to_string()];
        self.anchor = Some(id.to_string());
    }
    pub(crate) fn toggle(&mut self, id: &str) {
        if self.ids.iter().any(|value| value == id) {
            self.ids.retain(|value| value != id);
        } else {
            self.ids.push(id.to_string());
        }
        self.anchor = Some(id.to_string());
    }
    pub(crate) fn range(&mut self, id: &str, order: &[String], additive: bool) {
        let anchor = self
            .anchor
            .as_ref()
            .and_then(|value| order.iter().position(|item| item == value));
        let target = order.iter().position(|value| value == id);
        let (Some(anchor), Some(target)) = (anchor, target) else {
            if additive {
                self.toggle_missing(id);
            } else {
                self.single(id);
            }
            self.anchor = Some(id.to_string());
            return;
        };
        let range = &order[anchor.min(target)..=anchor.max(target)];
        if additive {
            let mut seen: std::collections::HashSet<String> = self.ids.iter().cloned().collect();
            self.ids.extend(
                range
                    .iter()
                    .filter(|value| seen.insert((*value).clone()))
                    .cloned(),
            );
        } else {
            self.ids = range.to_vec();
        }
    }
    fn toggle_missing(&mut self, id: &str) {
        if !self.ids.iter().any(|value| value == id) {
            self.ids.push(id.to_string());
        }
    }
    pub(crate) fn step(&mut self, current: &str, next: &str) {
        if self.ids.iter().any(|value| value == current)
            && self.ids.iter().any(|value| value == next)
        {
            self.ids.retain(|value| value != current);
        } else {
            self.toggle_missing(next);
        }
        if self.anchor.is_none() {
            self.anchor = Some(current.to_string());
        }
    }
    pub(crate) fn all(&mut self, order: &[String], focused: Option<String>) {
        self.ids = order.to_vec();
        if focused.is_some() {
            self.anchor = focused;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn order() -> Vec<String> {
        ["alpha", "a", "beta", "docs", "readme"]
            .map(String::from)
            .to_vec()
    }
    #[test]
    fn ranges_are_visible_ordered_and_keep_the_anchor_when_reversed() {
        let mut model = TreeSelection::default();
        model.single("alpha");
        model.range("beta", &order(), false);
        assert_eq!(model.ids, ["alpha", "a", "beta"]);
        model.range("a", &order(), false);
        assert_eq!(model.ids, ["alpha", "a"]);
        model.single("docs");
        model.range("a", &order(), false);
        assert_eq!(model.ids, ["a", "beta", "docs"]);
        assert_eq!(model.anchor.as_deref(), Some("docs"));
    }
    #[test]
    fn shift_arrows_extend_then_contract_by_removing_the_current_item() {
        let mut model = TreeSelection::default();
        model.single("alpha");
        model.range("beta", &order(), false);
        model.step("beta", "docs");
        assert_eq!(model.ids, ["alpha", "a", "beta", "docs"]);
        model.step("docs", "beta");
        assert_eq!(model.ids, ["alpha", "a", "beta"]);
        assert_eq!(model.anchor.as_deref(), Some("alpha"));
    }
    #[test]
    fn additive_ranges_and_toggle_preserve_insertion_order_without_duplicates() {
        let mut model = TreeSelection::default();
        model.single("readme");
        model.toggle("alpha");
        model.range("beta", &order(), true);
        assert_eq!(model.ids, ["readme", "alpha", "a", "beta"]);
        model.toggle("alpha");
        assert_eq!(model.ids, ["readme", "a", "beta"]);
        model.all(&order(), Some("beta".into()));
        assert_eq!(model.ids, order());
        assert_eq!(model.anchor.as_deref(), Some("beta"));
    }
}
