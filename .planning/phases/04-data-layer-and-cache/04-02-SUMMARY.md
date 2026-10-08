---
phase: 04-data-layer-and-cache
plan: 02
status: done
requirements-completed: [DATA-01, DATA-02, DATA-03]
key-files:
  modified: [crates/presto-core/examples/live.rs]
---

# Phase 4 Plan 02: Live API probe

`probe` command added to `examples/live.rs`. Run once against the signed-in real engine; raw output stayed in the session scratchpad.

## SORT_SUPPORT

| kind | none | name | -name | dateAdded | -dateAdded |
|---|---|---|---|---|---|
| songs | default | orders | orders | orders | orders |
| albums | default | orders | orders | orders | orders |
| artists | default | orders | orders | error 400 (40004 Invalid Parameter) | error 400 (40004 Invalid Parameter) |
| playlists | default | orders | orders | orders | orders |

`name` returns the same first 5 as `none` (the default is name ascending); `-name` reverses it, so `name` is honored. `dateAdded` is oldest first, `-dateAdded` newest first. Artists have no date sort and a sort value Apple rejects returns HTTP 400, so 04-06 must not send one.

```rust
pub const SORT_SUPPORT: &[(LibKind, &[Sort])] = &[
    (LibKind::Songs, &[Sort::NameAsc, Sort::NameDesc, Sort::AddedOldest, Sort::AddedNewest]),
    (LibKind::Albums, &[Sort::NameAsc, Sort::NameDesc, Sort::AddedOldest, Sort::AddedNewest]),
    (LibKind::Artists, &[Sort::NameAsc, Sort::NameDesc]),
    (LibKind::Playlists, &[Sort::NameAsc, Sort::NameDesc, Sort::AddedOldest, Sort::AddedNewest]),
];
```

## Limits

- Library songs: `limit=100` returns 100 items, `limit=101` errors (400, code 40005 "Value must be an integer", source parameter `limit`). Max page is 100.
- Recently played: `limit=10` and `limit=11` both return the requested count (11 items for 11). The documented cap of 10 is not enforced here; keep 10 to be safe.
- `offset=10` on recently played returns the next 10 items (different ids). Paging works.
- `recent/played/tracks?limit=30` returns 30 `songs` items with `next` set (`?offset=30`).
- Library list responses carry `data`, `meta` (with `total`) and `next`. Recently played has `data` and `next`, no `meta`.

## Shapes

- Storefront: `data[0].id` (`us`).
- Recommendations (`/v1/me/recommendations?limit=10`): `data[]` of type `personal-recommendation`, plus `meta`, `next` (`?offset=10`). Group title: `data[].attributes.title.stringForDisplay`. Other group attributes: `kind`, `display.kind`, `hasSeeAll`, `resourceTypes[]`, `nextUpdateDate`. Contents are inline at `data[].relationships.contents.data[]` (playlists, albums, etc. with normal `attributes`).
- `recs-id` (`/v1/me/recommendations/{id}`) returns one group with the same inline `relationships.contents.data[]`; no extra paging. `recs-contents` (`/v1/me/recommendations/{id}/contents?limit=10`) returns `data[]` of the content items directly, no `next`, no `meta`. For See all: none needed, use contents from the first response (`hasSeeAll` was false on the first group).
- Search (`/v1/catalog/{sf}/search`, `types=songs,albums,artists,playlists`, `with=topResults`): `results.{songs,albums,artists,playlists}.data[]` plus `results.topResults.data[]` (mixed types, length 25), each group with `href`. `meta.results.order` and `rawOrder` list `["topResults"]`.
- Hints (`/v1/catalog/{sf}/search/hints`): `results.terms[]` (array of strings).
- Library search (`/v1/me/library/search`, `types=library-songs,library-albums,library-artists,library-playlists`): `results.{library-songs,library-albums,library-artists}.data[]`; `library-playlists` was absent for term `a`. `meta.results.order` lists the groups in display order. Items carry `attributes.name`, `playParams`, `dateAdded` (albums).

## Default order

- songs, albums, artists, playlists: name ascending (case-insensitive, Latin before non-Latin scripts).
- Recently played: most recent first (mixed playlists, stations, albums).

## Deviations

None.
