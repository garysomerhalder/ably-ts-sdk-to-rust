# PROD-006: Fix REST Pagination Link Header Parsing

## 🎯 Objective
Implement Link header parsing for REST API pagination to enable proper iteration through large datasets.

## 📋 Task Details

**Priority:** 🟡 MEDIUM
**Effort:** 1 day
**Assignee:** Rust Engineer
**Dependencies:** PROD-001

## 🔍 Problem

Multiple TODO comments in `client/rest.rs` for Link header parsing, preventing pagination.

## ✅ Acceptance Criteria

1. [ ] Parse Link headers correctly
2. [ ] Extract first, next, current URLs
3. [ ] Support paginated iteration
4. [ ] Tests with large datasets

## 🛠️ Implementation

```rust
// client/rest.rs
fn parse_link_header(headers: &HeaderMap) -> Option<PaginationLinks> {
    let link_header = headers.get("Link")?.to_str().ok()?;

    let mut links = PaginationLinks::default();

    for link in link_header.split(',') {
        let parts: Vec<&str> = link.split(';').collect();
        if parts.len() != 2 { continue; }

        let url = parts[0].trim()
            .trim_start_matches('<')
            .trim_end_matches('>');

        let rel = parts[1].trim()
            .strip_prefix("rel=\"")?
            .strip_suffix("\"")?;

        match rel {
            "first" => links.first = Some(url.to_string()),
            "next" => links.next = Some(url.to_string()),
            "current" => links.current = Some(url.to_string()),
            _ => {}
        }
    }

    Some(links)
}

#[derive(Debug, Default)]
pub struct PaginationLinks {
    pub first: Option<String>,
    pub next: Option<String>,
    pub current: Option<String>,
}
```

## 📊 Success Metrics

- ✅ Pagination works for all REST endpoints
- ✅ Can iterate through 10,000+ items
- ✅ Link header parsing < 1ms