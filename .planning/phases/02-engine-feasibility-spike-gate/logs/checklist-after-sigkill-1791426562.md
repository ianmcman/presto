# presto-spike checklist
- label: after-sigkill
- unix_time: 1791426562
- engine: Some("presto-engine ecs 44.5.1") caps=["playback", "queue", "api"]
- engine_args: ["--ozone-platform=wayland"]

| check | result | detail |
|---|---|---|
| session | PASS | signed_in |
