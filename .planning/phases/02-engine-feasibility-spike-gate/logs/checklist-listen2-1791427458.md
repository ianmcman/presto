# presto-spike checklist
- label: listen2
- unix_time: 1791427458
- engine: Some("presto-engine ecs 44.5.1") caps=["playback", "queue", "api"]
- engine_args: []

| check | result | detail |
|---|---|---|
| hello | PASS | engine=Some("presto-engine ecs 44.5.1") caps=["playback", "queue", "api"] |
| musickit | PASS | auth=SignedIn |
| session | PASS | signed_in |
| api | PASS | 11 playlists, next=false |
| playback | PASS | song=6781024437 duration_ms=355000 reached=155000ms seek=150000 |
| events | PASS | {"auth": 1, "playback_state": 7, "progress": 156, "queue_changed": 2, "track_changed": 1, "volume": 1} |
