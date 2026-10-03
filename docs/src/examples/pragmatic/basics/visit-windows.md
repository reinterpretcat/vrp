# Visit windows

This example demonstrates how to use the `visitWindows` shift property and the `visitWindow` job property to keep
visits without a fixed time inside hours a shift sets aside for them.

## Overview

A shift may state any number of **named** windows. A job names the one it keeps to with `"visitWindow": "<name>"`.
The names mean nothing to the solver: the caller decides what each window is for.

A window bounds a visit's start **and** end: service starts at or after `earliest`, and the visit is departed at or
before `latest`. The vehicle may arrive earlier and wait. A job without a `visitWindow` (for example one with its own
time window, such as a booked time) is bound only by its own time windows and the shift's `jobTimes`. Breaks, reloads
and recharges are never bound by a visit window.

Every window lies inside the shift. A window outside it, one that does not run forwards, a `fallback` that names no
window of the shift or closes a cycle, or a job tag that names no window of a shift with windows is refused with
`E1310`.

## Fallback

A window may name a `fallback`: a visit that does not fit its own window from where the vehicle arrives is served in
the first window of the chain it fits. A visit that fits its own window waits for it. A fallback need not contain the
window it backs.

When any shift window has a `fallback` and the problem has no `objectives`, the default objectives get
`minimize-visit-window-overflow` directly after `minimize-unassigned`, so a fallback is used to place a visit that would
otherwise stay unassigned, never to save distance or time. It measures `"visits"` (one per visit outside its own
window, the default) or `"minutes"` (the service time outside it):

```json
{ "type": "minimize-visit-window-overflow", "measure": "minutes" }
```

A problem that lists its own `objectives` adds it there itself.

## Bridge

With `"bridge": true`, a window reaches out to the untagged visits on the visit's route: from the earlier of its own
start and their earliest time-window start, to the later of its own end and their latest time-window end. A vehicle
that leaves early for a booked visit then fills the time before the window instead of waiting for it, while a vehicle
without such a visit keeps to the window. When ruin removes the booked visit, the visits it made room for go back for
reinsertion.

## Example

In this example, we have:
- A vehicle shift from 07:00 to 20:00 with three windows:
  - `working`: 07:00–20:00
  - `regular`: 09:00–17:00, bridged, falling back to `working`
  - `recurring`: 09:00–15:00, falling back to `regular`
- Three jobs of 30 minutes:
  - `bookedVisit`: untagged, booked for 07:30–08:00
  - `regularVisit`: tagged `regular`, no time window
  - `recurringVisit`: tagged `recurring`, no time window

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

1. **The booked visit keeps its time**: it is served at 07:51, before the regular window opens, because it is untagged
2. **The regular visit bridges**: it is served at 08:24, right after the booked visit, instead of waiting until 09:00
3. **The recurring visit keeps to its window**: it is not bridged and starts at 09:00
