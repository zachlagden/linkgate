use std::collections::HashSet;

pub fn arrange<T>(items: Vec<T>, id_of: impl Fn(&T) -> &str, saved: &[String]) -> Vec<T> {
    let mut keyed: Vec<(usize, T)> = items
        .into_iter()
        .map(|item| {
            let rank = saved
                .iter()
                .position(|id| id == id_of(&item))
                .unwrap_or(usize::MAX);
            (rank, item)
        })
        .collect();
    keyed.sort_by_key(|(rank, _)| *rank);
    keyed.into_iter().map(|(_, item)| item).collect()
}

pub fn merge_saved(previous: &[String], installed_order: &[String]) -> Vec<String> {
    let installed: HashSet<&str> = installed_order.iter().map(String::as_str).collect();
    let mut merged: Vec<String> = installed_order.to_vec();
    let mut anchor: Option<&str> = None;
    let mut inserted_after_anchor = 0;
    for id in previous {
        if installed.contains(id.as_str()) {
            anchor = Some(id.as_str());
            inserted_after_anchor = 0;
            continue;
        }
        if merged.contains(id) {
            continue;
        }
        let position = match anchor {
            Some(anchor_id) => merged.iter().position(|m| m == anchor_id).map_or(merged.len(), |p| p + 1),
            None => 0,
        } + inserted_after_anchor;
        merged.insert(position.min(merged.len()), id.clone());
        inserted_after_anchor += 1;
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    fn arranged(default_order: &[&str], saved: &[&str]) -> Vec<String> {
        arrange(ids(default_order), |id| id.as_str(), &ids(saved))
    }

    #[test]
    fn keeps_the_default_order_with_no_saved_order() {
        assert_eq!(arranged(&["chrome", "edge", "firefox"], &[]), ids(&["chrome", "edge", "firefox"]));
    }

    #[test]
    fn follows_a_full_saved_order() {
        assert_eq!(
            arranged(&["chrome", "edge", "firefox"], &["firefox", "chrome", "edge"]),
            ids(&["firefox", "chrome", "edge"])
        );
    }

    #[test]
    fn puts_unsaved_browsers_after_the_saved_ones_in_default_order() {
        assert_eq!(
            arranged(&["chrome", "edge", "firefox", "opera"], &["opera", "edge"]),
            ids(&["opera", "edge", "chrome", "firefox"])
        );
    }

    #[test]
    fn ignores_saved_ids_that_are_not_installed() {
        assert_eq!(
            arranged(&["chrome", "firefox"], &["brave", "firefox", "chrome"]),
            ids(&["firefox", "chrome"])
        );
    }

    #[test]
    fn a_first_seen_browser_lands_last() {
        assert_eq!(
            arranged(&["chrome", "vivaldi", "firefox"], &["firefox", "chrome"]),
            ids(&["firefox", "chrome", "vivaldi"])
        );
    }

    #[test]
    fn hidden_browsers_keep_their_saved_position() {
        let order = arranged(&["chrome", "edge", "firefox"], &["edge", "firefox", "chrome"]);
        assert_eq!(order, ids(&["edge", "firefox", "chrome"]));
    }

    #[test]
    fn saving_keeps_uninstalled_ids_next_to_their_neighbour() {
        let merged = merge_saved(&ids(&["firefox", "brave", "chrome"]), &ids(&["chrome", "firefox"]));
        assert_eq!(merged, ids(&["chrome", "firefox", "brave"]));
    }

    #[test]
    fn saving_keeps_a_leading_uninstalled_id_first() {
        let merged = merge_saved(&ids(&["brave", "firefox", "chrome"]), &ids(&["chrome", "firefox"]));
        assert_eq!(merged, ids(&["brave", "chrome", "firefox"]));
    }

    #[test]
    fn saving_keeps_several_uninstalled_ids_in_order() {
        let merged = merge_saved(
            &ids(&["firefox", "brave", "opera", "chrome"]),
            &ids(&["chrome", "firefox"]),
        );
        assert_eq!(merged, ids(&["chrome", "firefox", "brave", "opera"]));
    }

    #[test]
    fn saving_with_no_previous_order_uses_the_new_order() {
        assert_eq!(merge_saved(&[], &ids(&["edge", "chrome"])), ids(&["edge", "chrome"]));
    }

    #[test]
    fn an_uninstalled_browser_returns_to_its_spot() {
        let saved = merge_saved(&ids(&["firefox", "brave", "chrome"]), &ids(&["chrome", "firefox"]));
        let order = arrange(ids(&["brave", "chrome", "firefox"]), |id| id.as_str(), &saved);
        assert_eq!(order, ids(&["chrome", "firefox", "brave"]));
    }
}
