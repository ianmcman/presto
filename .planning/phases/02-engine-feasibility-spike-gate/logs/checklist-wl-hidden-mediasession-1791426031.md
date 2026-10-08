# presto-spike checklist
- label: wl-hidden-mediasession
- unix_time: 1791426031
- engine: Some("presto-engine ecs 44.5.1") caps=["playback", "queue", "api"]
- engine_args: ["--ozone-platform=wayland", "--keep-media-session"]

| check | result | detail |
|---|---|---|
| hello | PASS | engine=Some("presto-engine ecs 44.5.1") caps=["playback", "queue", "api"] |
| musickit | PASS | auth=SignedIn |
| session | PASS | signed_in |
| api | PASS | 11 playlists, next=false |
| playback | PASS | song=6781024437 duration_ms=355000 reached=95000ms seek=90000 |
| events | PASS | {"auth": 1, "playback_state": 7, "progress": 76, "queue_changed": 2, "track_changed": 1, "volume": 1} |
