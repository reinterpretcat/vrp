# Visit windows

This example demonstrates how to use the `visitWindows` shift property and the `visitWindow` job property to keep
visits without a fixed time inside hours a shift sets aside for them.

## Overview

A shift may state two windows:

- **recurring**: the hours jobs tagged `"visitWindow": "recurring"` keep to
- **other**: the hours jobs tagged `"visitWindow": "non-recurring"` keep to

A window bounds a visit's start **and** end: service starts at or after `earliest`, and the visit is departed at or
before `latest`. The vehicle may arrive earlier and wait. A job without a `visitWindow` (for example one with its own
time window, such as a booked time) is bound only by its own time windows and the shift's `jobTimes`. Breaks, reloads
and recharges are never bound by a visit window. A job tagged with a window its shift does not state is not bound by
one either.

Both windows lie inside the shift; a window outside it, one that does not run forwards, or a tag other than
`recurring` and `nonRecurring` is refused with `E1310`.

## Overflow

With `"overflow": true`, a recurring visit that does not fit the recurring window from where the vehicle arrives may
be served in the non-recurring window instead. A recurring visit that fits its own window waits for it.

When any shift states `overflow: true` and the problem has no `objectives`, the default objectives get
`minimize-visit-window-overflow` directly after `minimize-unassigned`. It counts recurring visits outside their own
window, so overflow is used to place a visit that would otherwise stay unassigned, never to save distance or time.
A problem that lists its own `objectives` adds it there itself.

## Example

In this example, we have:
- A vehicle shift from 08:00 to 20:00 with recurring visits 08:45–15:00 and non-recurring visits 08:45–19:00
- Three jobs of 30 minutes:
  - `recurringVisit`: tagged `recurring`, no time window
  - `nonRecurringVisit`: tagged `nonRecurring`, no time window
  - `bookedVisit`: untagged, booked for 16:30–17:00

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/visit-windows.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/visit-windows.basic.solution.json}}
```

</p></details>

## Key observations

1. **The vehicle leaves late enough**: it departs at 08:42 so the recurring visit starts after 08:45
2. **Both tagged visits lie in their windows**: the recurring and the non-recurring visit are served in the morning
3. **The booked visit keeps its time**: it is served 16:30–17:00, after the recurring window closed, because it is not
   tagged
