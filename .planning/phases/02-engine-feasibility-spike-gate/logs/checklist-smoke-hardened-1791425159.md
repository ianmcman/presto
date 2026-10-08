# presto-spike checklist
- label: smoke-hardened
- unix_time: 1791425159
- engine: Some("presto-engine ecs 44.5.1") caps=["playback", "queue", "api"]
- engine_args: ["--ozone-platform=wayland", "--url", "about:blank"]

| check | result | detail |
|---|---|---|
| hello | PASS | engine=Some("presto-engine ecs 44.5.1") caps=["playback", "queue", "api"] |
