# Widgets (planned)

Each widget is an independent module, so it can be built and shipped on its
own:

```
widgets/<name>/
  <Name>Widget.tsx    React component (pure UI)
  use<Name>.ts        data hook (subscribes to a native event; no polling in JS)
  <name>.css
```

Planned modules:

| Module | Data source | Notes |
|---|---|---|
| `clock` | JS `Date`, one timer aligned to the next minute boundary | no native access needed |
| `date` | JS `Intl.DateTimeFormat` | locale-aware day and date |
| `system` | Rust: `GetSystemTimes` (CPU), `GlobalMemoryStatusEx` (RAM) | sampled only while visible, ~1 Hz |
| `battery` | Rust: `GetSystemPowerStatus` + power-setting notifications | event-driven |

None of these read per-process information, window titles or anything that
leaves the machine.
