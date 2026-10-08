# Phase 5 live checks

Run with `cargo run -p presto-core --example live`, then type the command at its stdin. Replace `<sf>` with your storefront (e.g. `us`). Use `get <path>` for a request that prints the response, `probe` for the D-08 probe.

| # | Check | Command | Expected | Result |
|---|-------|---------|----------|--------|
| 1 | Library song id in setQueue (Q1) | `get /v1/me/library/songs?limit=1`, then `queue i.<id>` | Track plays. If it fails, `queue_id` must use `playParams.catalogId` | PASS. `queue i.9oJZBJYSPReVxq` (library id) played "A-Punk"; library ids work in setQueue, `queue_id` stays as is. |
| 2 | Library album and playlist tracks, limit 100 (Q6) | `get /v1/me/library/albums/<l.id>/tracks?limit=100` and `get /v1/me/library/playlists/<p.id>/tracks?limit=100` | `data` returned, limit 100 accepted | PASS. Library album tracks (limit=100) returned data; library playlist tracks (limit=100) returned 55 items with no `next`. |
| 3 | Catalog playlist tracks paging (Q6) | `get /v1/catalog/<sf>/playlists/<pl.id>/tracks?limit=100` on a long playlist | `data` returned, `next` present | PASS. Catalog playlist tracks returned `next` as `/v1/catalog/us/playlists/<id>/tracks?offset=10` (checked with limit=10; the link has no `limit`, so the pager must re-add it). |
| 4 | Artist views (Q3) | `get /v1/catalog/<sf>/artists/<id>?views=top-songs,full-albums,singles` | `views` has `top-songs`, `full-albums`, `singles` | PASS. Artist 471744 `views` has `full-albums`, `singles`, `top-songs`. |
| 5 | Library artist mapping (Q3) | `get /v1/me/library/artists/<r.id>?include=catalog` | `relationships.catalog.data[0].id` present | PASS. Library artist `r.raLzGu5?include=catalog` gave `relationships.catalog.data[0].id` = 1109952091. |
| 6 | Unplayable song (Q2) | Find a catalog song with no `playParams` or region-locked, then `queue <id>` | Record: error event, skip, or stall | Silent drop. Library songs with no `playParams` (e.g. `i.vMXG6A6Sg4rKAL`) are accepted by `queue` (ok) but MusicKit leaves them out of the queue: alone, queue stays empty and player `stopped`; mixed with a playable id, only the playable track is queued. No error event, no stall. A queue index from the requested ids can shift if an item is dropped. |
