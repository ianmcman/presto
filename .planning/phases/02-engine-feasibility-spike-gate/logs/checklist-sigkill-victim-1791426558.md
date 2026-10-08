# presto-spike checklist
- label: sigkill-victim
- unix_time: 1791426558
- engine: Some("presto-engine ecs 44.5.1") caps=["playback", "queue", "api"]
- engine_args: ["--ozone-platform=wayland"]

| check | result | detail |
|---|---|---|
| hello | PASS | engine=Some("presto-engine ecs 44.5.1") caps=["playback", "queue", "api"] |
| musickit | PASS | auth=SignedIn |
| session | PASS | signed_in |
| api | PASS | 11 playlists, next=false |
| playback | FAIL | engine closed the connection |
| events | FAIL | engine closed the connection |
