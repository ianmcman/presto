# Phase 5 live checks

Run with `cargo run -p presto-core --example live`, then type the command at its stdin. Replace `<sf>` with your storefront (e.g. `us`). Use `get <path>` for a request that prints the response, `probe` for the D-08 probe.

| # | Check | Command | Expected | Result |
|---|-------|---------|----------|--------|
| 1 | Library song id in setQueue (Q1) | `get /v1/me/library/songs?limit=1`, then `queue i.<id>` | Track plays. If it fails, `queue_id` must use `playParams.catalogId` | |
| 2 | Library album and playlist tracks, limit 100 (Q6) | `get /v1/me/library/albums/<l.id>/tracks?limit=100` and `get /v1/me/library/playlists/<p.id>/tracks?limit=100` | `data` returned, limit 100 accepted | |
| 3 | Catalog playlist tracks paging (Q6) | `get /v1/catalog/<sf>/playlists/<pl.id>/tracks?limit=100` on a long playlist | `data` returned, `next` present | |
| 4 | Artist views (Q3) | `get /v1/catalog/<sf>/artists/<id>?views=top-songs,full-albums,singles` | `views` has `top-songs`, `full-albums`, `singles` | |
| 5 | Library artist mapping (Q3) | `get /v1/me/library/artists/<r.id>?include=catalog` | `relationships.catalog.data[0].id` present | |
| 6 | Unplayable song (Q2) | Find a catalog song with no `playParams` or region-locked, then `queue <id>` | Record: error event, skip, or stall | |
