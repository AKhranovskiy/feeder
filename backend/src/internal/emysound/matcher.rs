use super::QueryResult;

pub fn best_results(results: Vec<QueryResult>) -> Vec<QueryResult> {
    results
        .iter()
        .filter(|r| {
            if r.coverage >= 190 {
                true
            } else {
                results
                    .iter()
                    .find(|r2| {
                        r2.id != r.id
                            && (r2.artist == r.artist || r2.title == r.title)
                            && r.coverage.checked_add(r2.coverage).unwrap_or_default() > 230
                    })
                    .map(|v| {
                        log::debug!("Result match: {r:?} - {v:?}");
                        v
                    })
                    .is_some()
            }
        })
        .cloned()
        .collect()
}
