# presto-spike checklist
- label: listen
- unix_time: 1791427310
- engine: Some("presto-engine ecs 44.5.1") caps=["playback", "queue", "api"]
- engine_args: []

| check | result | detail |
|---|---|---|
| hello | PASS | engine=Some("presto-engine ecs 44.5.1") caps=["playback", "queue", "api"] |
| musickit | PASS | auth=SignedIn |
| session | PASS | signed_in |
| api | PASS | 11 playlists, next=false |
| playback | FAIL | Connection reset by peer (os error 104) |
| events | FAIL | Connection reset by peer (os error 104) |
