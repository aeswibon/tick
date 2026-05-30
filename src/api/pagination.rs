//! JQL search and bulk-fetch paging limits.

/// Max issues per `POST /rest/api/3/search/jql` request (id-only queries).
pub const JQL_SEARCH_PAGE_SIZE: u32 = 100;

/// Max issue ids per `POST /rest/api/3/issue/bulkfetch` request.
pub const BULK_FETCH_CHUNK: usize = 100;

/// Compute per-request JQL page size given remaining quota.
pub fn jql_page_size(remaining: u32) -> u32 {
    remaining.min(JQL_SEARCH_PAGE_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jql_page_size_caps_at_100() {
        assert_eq!(jql_page_size(250), 100);
        assert_eq!(jql_page_size(100), 100);
        assert_eq!(jql_page_size(37), 37);
        assert_eq!(jql_page_size(0), 0);
    }
}
