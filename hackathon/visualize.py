#!/usr/bin/env python3
"""Show a pragmatic solution GeoJSON on a map with per-truck timelines."""

from __future__ import annotations

import argparse
import json
import tempfile
import webbrowser
from collections import defaultdict
from datetime import datetime, timedelta
from html import escape
from pathlib import Path
from typing import NamedTuple

import folium

HERE = Path(__file__).resolve().parent
DEFAULT_GEOJSON = HERE / "data" / "sites.basic.solution-gjson.json"
DEFAULT_COLOR = "#3388ff"
PAGE_BG = "#0b0f14"
RELOAD_JOB = "reload"
RELOAD_ICON_SIZE = (26, 14)
DEPOT_MIN_PX = 18
JOB_BASE_PX = 14
JOB_DOT_PX = 10
KIND_STYLES = {
    "replacement": ("Emptying", "#34b36b"),
    "delivery": ("Deployment", "#4a7cf6"),
    "pickup": ("Removal", "#f2c230"),
}
UNKNOWN_KIND = ("Job", "#8b97a8")

PAGE_STYLE = """
    :root {
      --bg: #0b0f14;
      --ink: #e8edf2;
      --muted: #8b97a8;
      --line: #243042;
    }
    html, body {
      margin: 0;
      background: var(--bg);
      color: var(--ink);
      font-family: "IBM Plex Sans", sans-serif;
    }
    header {
      display: flex;
      justify-content: space-between;
      gap: 16px;
      align-items: baseline;
      padding: 18px 24px 12px;
      border-bottom: 1px solid var(--line);
    }
    header h1 {
      margin: 0;
      font-size: 20px;
      font-weight: 600;
      letter-spacing: 0.02em;
    }
    header p {
      margin: 4px 0 0;
      color: var(--muted);
      font-size: 13px;
    }
    .chips { display: flex; gap: 10px; flex-wrap: wrap; }
    .chip {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      border: 1px solid var(--line);
      padding: 6px 10px;
      font-family: "IBM Plex Mono", monospace;
      font-size: 12px;
    }
    .chip i {
      width: 8px;
      height: 8px;
      display: inline-block;
    }
    .map-wrap { height: 62vh; min-height: 420px; }
    .map-wrap .folium-map { height: 100%; width: 100%; }
    .board {
      padding: 8px 24px 28px;
    }
    .board h2 {
      margin: 18px 0 4px;
      font-size: 13px;
      font-weight: 500;
      letter-spacing: 0.14em;
      text-transform: uppercase;
      color: var(--muted);
    }
    .empty { color: var(--muted); }
    .tl-head {
      display: flex;
      align-items: baseline;
      gap: 18px;
      flex-wrap: wrap;
      padding: 14px 0 10px;
    }
    .tl-head h2 { margin: 0; }
    .key { display: flex; gap: 14px; flex-wrap: wrap; font-size: 13px; }
    .key-label { color: var(--muted); }
    .key span { display: inline-flex; align-items: center; gap: 6px; }
    .key i, .job i {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      display: inline-block;
    }
    .tz { margin-left: auto; color: var(--muted); font-size: 12px; }
    .tl-scroll { overflow-x: auto; }
    .tl { min-width: 980px; }
    .tl-row {
      display: grid;
      grid-template-columns: 330px 1fr;
      align-items: center;
      min-height: 50px;
      border-bottom: 1px solid var(--line);
    }
    .tl-axis { min-height: 34px; }
    .tl-ticks, .tl-track { position: relative; margin-right: 24px; }
    .tl-ticks { height: 34px; }
    .tl-ticks span {
      position: absolute;
      bottom: 8px;
      transform: translateX(-4px);
      font-family: "IBM Plex Mono", monospace;
      font-size: 11px;
      color: var(--muted);
    }
    .tl-ticks span:last-child { transform: translateX(-100%); }
    .tl-track {
      height: 50px;
      background: linear-gradient(to right, var(--line) 1px, transparent 1px)
        0 0 / calc(100% / var(--hours)) 100%;
    }
    .tl-truck {
      display: flex;
      align-items: center;
      gap: 10px;
      padding-right: 16px;
      font-size: 14px;
      white-space: nowrap;
    }
    .swatch {
      display: inline-grid;
      place-items: center;
      width: 22px;
      height: 22px;
      border: 1.5px solid currentColor;
      border-radius: 5px;
    }
    .badge {
      display: inline-flex;
      align-items: center;
      gap: 5px;
      padding: 3px 8px;
      border-radius: 999px;
      background: #1a2330;
      font-size: 12px;
    }
    .meta { color: var(--muted); font-size: 12px; margin-left: auto; }
    .icon-btn {
      display: inline-grid;
      place-items: center;
      width: 26px;
      height: 26px;
      padding: 0;
      border: 0;
      border-radius: 6px;
      background: transparent;
      color: var(--ink);
      cursor: pointer;
    }
    .icon-btn:hover { background: #1a2330; }
    .icon-btn:focus-visible { outline: 2px solid var(--muted); }
    .tl-row.is-hidden .tl-track { opacity: 0.3; }
    .tl-row.is-hidden [data-action="toggle"] { color: var(--muted); }
    .trip, .depot, .job {
      position: absolute;
      top: 50%;
      transform: translateY(-50%);
      box-sizing: border-box;
    }
    .trip {
      height: 30px;
      border: 1.5px solid var(--c);
      border-radius: 11px;
      background: var(--tint);
    }
    .trip::before {
      content: "";
      position: absolute;
      left: 8px;
      right: 8px;
      top: 50%;
      height: 2px;
      margin-top: -1px;
      background: var(--c);
    }
    .depot, .job {
      height: 22px;
      border: 1.5px solid var(--c);
      background: var(--bg);
    }
    .depot { border-radius: 9px; }
    .depot, .job {
      padding: 0;
      color: inherit;
      font: inherit;
      cursor: pointer;
    }
    .depot:hover, .job:hover, .depot.is-active, .job.is-active {
      box-shadow: 0 0 0 2px var(--bg), 0 0 0 4px var(--c);
      z-index: 2;
    }
    .depot:focus-visible, .job:focus-visible { outline: 2px solid var(--ink); outline-offset: 2px; }
    .stop-pop {
      position: absolute;
      z-index: 1000;
      padding: 12px 14px;
      background: #fbfaf7;
      border-radius: 4px;
      box-shadow: 0 8px 28px rgba(0, 0, 0, 0.45);
    }
    .stop-pop[hidden] { display: none; }
    .job {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 3px;
      border-radius: 7px;
      transform: translate(-50%, -50%);
    }
    .stop-card {
      font-family: "IBM Plex Sans", sans-serif;
      min-width: 220px;
      color: #1b1f24;
    }
    .stop-card .kicker {
      font-family: "IBM Plex Mono", monospace;
      font-size: 11px;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: #66707c;
    }
    .stop-card .title {
      font-size: 16px;
      font-weight: 600;
      margin: 4px 0 10px;
    }
    .stop-card .row {
      display: flex;
      justify-content: space-between;
      gap: 16px;
      font-size: 13px;
      padding: 3px 0;
      border-top: 1px solid #e4e0d7;
    }
    .stop-card span { color: #66707c; }
    .leaflet-popup-content-wrapper { border-radius: 2px; }
"""


class Stop(NamedTuple):
    tour_idx: str
    stop_idx: int
    lat: float
    lng: float
    jobs: str
    arrival: datetime | None
    departure: datetime | None
    distance: int | None
    color: str
    symbol: str
    vehicle_id: str


class Route(NamedTuple):
    tour_idx: str
    coordinates: list[list[float]]
    color: str
    vehicle_id: str


class Solution(NamedTuple):
    kinds: dict[str, str]
    vehicle_types: dict[str, str]


NO_SOLUTION = Solution({}, {})


def parse_time(value: object) -> datetime | None:
    if not value or not isinstance(value, str):
        return None
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def parse_int(value: object) -> int | None:
    if value in (None, ""):
        return None
    try:
        return int(float(str(value)))
    except ValueError:
        return None


def format_clock(moment: datetime | None) -> str:
    if moment is None:
        return "—"
    return moment.strftime("%H:%M:%S")


def format_duration(delta: timedelta) -> str:
    seconds = max(0, int(round(delta.total_seconds())))
    hours, rem = divmod(seconds, 3600)
    minutes, secs = divmod(rem, 60)
    if hours:
        return f"{hours}h {minutes:02d}m {secs:02d}s"
    if minutes:
        return f"{minutes}m {secs:02d}s"
    return f"{secs}s"


def format_distance(meters: int | None) -> str:
    if meters is None:
        return "—"
    if abs(meters) < 1000:
        return f"{meters} m"
    return f"{meters / 1000:.1f} km"


def mix_hex(color: str, onto: str = PAGE_BG, weight: float = 0.45) -> str:
    def parts(value: str) -> tuple[int, int, int]:
        value = value.lstrip("#")
        return int(value[0:2], 16), int(value[2:4], 16), int(value[4:6], 16)

    try:
        r, g, b = parts(color)
        br, bg, bb = parts(onto)
    except (ValueError, IndexError):
        return onto
    return "#{:02x}{:02x}{:02x}".format(
        int(br + (r - br) * weight),
        int(bg + (g - bg) * weight),
        int(bb + (b - bb) * weight),
    )


def readable(color: str) -> str:
    """Lighten colours that would vanish against the dark page."""
    try:
        r, g, b = (int(color.lstrip("#")[i : i + 2], 16) for i in (0, 2, 4))
    except ValueError:
        return color
    if 0.2126 * r + 0.7152 * g + 0.0722 * b >= 90:
        return color
    return mix_hex(color, onto="#ffffff", weight=0.45)


def distance_gap(stop: Stop, previous: Stop | None) -> int | None:
    if previous is None or stop.distance is None or previous.distance is None:
        return None
    return stop.distance - previous.distance


def is_depot(stop: Stop) -> bool:
    return stop.symbol == "warehouse"


def is_reload(stop: Stop) -> bool:
    return stop.jobs == RELOAD_JOB


def reload_icon(color: str) -> folium.DivIcon:
    width, height = RELOAD_ICON_SIZE
    return folium.DivIcon(
        html=(
            f'<div style="width:{width}px;height:{height}px;box-sizing:border-box;'
            f'border-radius:50%;border:2px solid #f4f1e8;background:{escape(color)}"></div>'
        ),
        icon_size=RELOAD_ICON_SIZE,
        icon_anchor=(width // 2, height // 2),
    )


def vehicle_label(tour_idx: str, vehicle_by_tour: dict[str, str]) -> str:
    return vehicle_by_tour.get(tour_idx, f"tour {tour_idx}")


def load_geojson(path: Path) -> tuple[list[Stop], list[Route]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    raw_points: list[dict] = []
    routes: list[Route] = []
    vehicle_by_tour: dict[str, str] = {}
    color_by_tour: dict[str, str] = {}

    for feature in data.get("features", []):
        geometry = feature.get("geometry") or {}
        props = feature.get("properties") or {}
        kind = geometry.get("type")
        tour_idx = str(props.get("tour_idx", "0"))
        vehicle_id = str(props.get("vehicle_id") or "")
        if vehicle_id:
            vehicle_by_tour[tour_idx] = vehicle_id

        if kind == "Point":
            lng, lat = geometry["coordinates"][:2]
            color = str(props.get("marker-color") or DEFAULT_COLOR)
            color_by_tour.setdefault(tour_idx, color)
            raw_points.append(
                {
                    "tour_idx": tour_idx,
                    "stop_idx": int(props.get("stop_idx") or 0),
                    "lat": float(lat),
                    "lng": float(lng),
                    "jobs": str(props.get("jobs_ids") or "stop"),
                    "arrival": parse_time(props.get("arrival")),
                    "departure": parse_time(props.get("departure")),
                    "distance": parse_int(props.get("distance")),
                    "color": color,
                    "symbol": str(props.get("marker-symbol") or "marker"),
                }
            )
        elif kind == "LineString":
            routes.append(
                Route(
                    tour_idx=tour_idx,
                    coordinates=geometry.get("coordinates") or [],
                    color=str(
                        props.get("stroke")
                        or color_by_tour.get(tour_idx, DEFAULT_COLOR)
                    ),
                    vehicle_id=vehicle_id or vehicle_label(tour_idx, vehicle_by_tour),
                )
            )

    stops = [
        Stop(vehicle_id=vehicle_label(point["tour_idx"], vehicle_by_tour), **point)
        for point in raw_points
    ]
    stops.sort(key=lambda stop: (stop.vehicle_id, stop.stop_idx))
    return stops, routes


def tours_of(stops: list[Stop]) -> dict[str, list[Stop]]:
    grouped: dict[str, list[Stop]] = defaultdict(list)
    for stop in stops:
        grouped[stop.vehicle_id].append(stop)
    return dict(grouped)


def _popup_row(label: str, value: str) -> str:
    return (
        f"<div class='row'><span>{label}</span><strong>{escape(value)}</strong></div>"
    )


def popup_html(stop: Stop, previous: Stop | None, kind: str = "") -> str:
    drive = ""
    if previous is not None and stop.arrival and previous.departure:
        driven = format_duration(stop.arrival - previous.departure)
        gap = format_distance(distance_gap(stop, previous))
        drive = _popup_row("Drive in", f"{driven} · {gap}")
    service = ""
    if stop.arrival and stop.departure and stop.departure > stop.arrival:
        service = _popup_row("Service", format_duration(stop.departure - stop.arrival))
    return f"""
    <div class="stop-card">
      <div class="kicker">{escape(stop.vehicle_id)} · stop {stop.stop_idx}</div>
      <div class="title">{escape(stop.jobs)}</div>
      {_popup_row("Type", kind) if kind else ""}
      {_popup_row("Arrival", format_clock(stop.arrival))}
      {_popup_row("Departure", format_clock(stop.departure))}
      {service}
      {drive}
      {_popup_row("From depot", format_distance(stop.distance))}
    </div>
    """


def build_map(
    stops: list[Stop], routes: list[Route], solution: Solution
) -> tuple[folium.Map, dict[str, str]]:
    latitudes = [stop.lat for stop in stops]
    longitudes = [stop.lng for stop in stops]
    fmap = folium.Map(
        location=(sum(latitudes) / len(latitudes), sum(longitudes) / len(longitudes)),
        tiles="Esri.WorldGrayCanvas",
        control_scale=True,
        zoom_start=10,
        height="100%",
        width="100%",
    )
    fmap.default_js = [
        item for item in folium.Map.default_js if item[0] in {"leaflet", "jquery"}
    ]
    fmap.default_css = [
        item for item in folium.Map.default_css if item[0] == "leaflet_css"
    ]
    groups: dict[str, folium.FeatureGroup] = {}

    def group_of(vehicle_id: str) -> folium.FeatureGroup:
        if vehicle_id not in groups:
            groups[vehicle_id] = folium.FeatureGroup(name=vehicle_id).add_to(fmap)
        return groups[vehicle_id]

    for route in routes:
        folium.PolyLine(
            locations=[(lat, lng) for lng, lat in route.coordinates],
            color=route.color,
            weight=4,
            opacity=0.85,
            tooltip=route.vehicle_id,
        ).add_to(group_of(route.vehicle_id))

    previous_by_vehicle: dict[str, Stop] = {}
    for stop in stops:
        previous = previous_by_vehicle.get(stop.vehicle_id)
        popup = folium.Popup(
            popup_html(stop, previous, stop_kind_label(stop, solution)), max_width=320
        )
        tooltip = f"{stop.stop_idx} · {stop.jobs}"
        if is_reload(stop):
            marker = folium.Marker(
                location=(stop.lat, stop.lng),
                icon=reload_icon(stop.color),
                popup=popup,
                tooltip=tooltip,
            )
        else:
            marker = folium.CircleMarker(
                location=(stop.lat, stop.lng),
                radius=9 if is_depot(stop) else 7,
                color="#f4f1e8",
                weight=2,
                fill=True,
                fill_color=stop.color,
                fill_opacity=1.0,
                popup=popup,
                tooltip=tooltip,
            )
        marker.add_to(group_of(stop.vehicle_id))
        previous_by_vehicle[stop.vehicle_id] = stop

    fmap.fit_bounds(
        [[min(latitudes), min(longitudes)], [max(latitudes), max(longitudes)]],
        padding=(24, 24),
    )
    return fmap, {vehicle: group.get_name() for vehicle, group in groups.items()}


def solution_path(geojson: Path) -> Path | None:
    name = geojson.name
    for suffix, replacement in (
        ("-gjson.json", ".json"),
        ("_solution.geojson", "_solution.json"),
        (".geojson", ".json"),
    ):
        if name.endswith(suffix):
            candidate = geojson.with_name(name[: -len(suffix)] + replacement)
            if candidate.exists():
                return candidate
    return None


def load_solution(path: Path | None) -> Solution:
    if path is None:
        return NO_SOLUTION
    data = json.loads(path.read_text(encoding="utf-8"))
    kinds: dict[str, str] = {}
    vehicle_types: dict[str, str] = {}
    for tour in data.get("tours", []):
        vehicle_types[str(tour.get("vehicleId"))] = str(tour.get("typeId") or "")
        for stop in tour.get("stops", []):
            for activity in stop.get("activities", []):
                kinds[str(activity.get("jobId"))] = str(activity.get("type"))
    return Solution(kinds, vehicle_types)


def job_kinds(stop: Stop, solution: Solution) -> list[str]:
    return [solution.kinds.get(job_id.strip(), "") for job_id in stop.jobs.split(",")]


def kind_style(kind: str) -> tuple[str, str]:
    return KIND_STYLES.get(kind, UNKNOWN_KIND)


def split_trips(stops: list[Stop]) -> list[tuple[int, int]]:
    splits = [index for index, stop in enumerate(stops) if is_depot(stop)]
    if not splits or splits[0] != 0:
        splits.insert(0, 0)
    if splits[-1] != len(stops) - 1:
        splits.append(len(stops) - 1)
    return list(zip(splits, splits[1:]))


def hour_floor(moment: datetime) -> datetime:
    return moment.replace(minute=0, second=0, microsecond=0)


class Scale(NamedTuple):
    start: datetime
    end: datetime

    def pct(self, moment: datetime) -> float:
        return (moment - self.start) / (self.end - self.start) * 100

    def span(self, start: datetime, end: datetime) -> float:
        return max(0.0, (end - start) / (self.end - self.start) * 100)

    @property
    def hours(self) -> int:
        return int((self.end - self.start) / timedelta(hours=1))


def timeline_scale(stops: list[Stop]) -> Scale | None:
    moments = [m for stop in stops for m in (stop.arrival, stop.departure) if m]
    if not moments:
        return None
    start = hour_floor(min(moments))
    end = hour_floor(max(moments))
    if end <= max(moments):
        end += timedelta(hours=1)
    return Scale(start, end)


def stop_kind_label(stop: Stop, solution: Solution) -> str:
    if is_depot(stop):
        return stop.jobs.capitalize()
    if not solution.kinds:
        return ""
    return ", ".join(kind_style(kind)[0] for kind in job_kinds(stop, solution))


def stop_button_attrs(stop: Stop, card: str) -> str:
    label = f"{stop.jobs}, {format_clock(stop.arrival)} to {format_clock(stop.departure)}"
    return f' type="button" aria-label="{escape(label)}" data-card="{escape(card)}"'


def depot_oval(stop: Stop, scale: Scale, anchor: str, edge: float, card: str) -> str:
    width = scale.span(stop.arrival, stop.departure) if anchor == "end" else 0.0
    side = (
        f"right:calc({100 - edge:.4f}% + 4px)"
        if anchor == "end"
        else f"left:calc({edge:.4f}% + 4px)"
    )
    return (
        f'<button class="depot" style="{side};width:max({width:.4f}%, {DEPOT_MIN_PX}px)"'
        f"{stop_button_attrs(stop, card)}></button>"
    )


def job_box(stop: Stop, scale: Scale, solution: Solution, card: str) -> str:
    kinds = job_kinds(stop, solution)
    dots = "".join(
        f'<i style="background:{kind_style(kind)[1]}"></i>' for kind in kinds
    )
    middle = stop.arrival + (stop.departure - stop.arrival) / 2
    width = scale.span(stop.arrival, stop.departure)
    min_px = JOB_BASE_PX + JOB_DOT_PX * len(kinds)
    return (
        f'<button class="job" style="left:{scale.pct(middle):.4f}%;width:max({width:.4f}%, {min_px}px)"'
        f"{stop_button_attrs(stop, card)}>{dots}</button>"
    )


def trip_html(
    stops: list[Stop], cards: list[str], first: int, last: int, scale: Scale, color: str
) -> str:
    start_stop, end_stop = stops[first], stops[last]
    start = start_stop.departure or start_stop.arrival
    end = end_stop.departure or end_stop.arrival
    left, right = scale.pct(start), scale.pct(end)
    parts = [
        f'<div class="trip" style="left:calc({left:.4f}% + 1px);width:calc({right - left:.4f}% - 2px);'
        f'--c:{color};--tint:{mix_hex(color, weight=0.2)}"></div>'
    ]
    if is_depot(start_stop):
        parts.append(depot_oval(start_stop, scale, "start", left, cards[first]))
    if is_depot(end_stop):
        parts.append(depot_oval(end_stop, scale, "end", right, cards[last]))
    return "".join(parts)


def truck_icon() -> str:
    return (
        '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor"'
        ' stroke-width="2" stroke-linecap="round" stroke-linejoin="round">'
        '<path d="M3 7h11v9H3zM14 10h4l3 3v3h-7"/><circle cx="7" cy="17.5" r="1.8"/>'
        '<circle cx="17" cy="17.5" r="1.8"/></svg>'
    )


EYE_ICON = (
    '<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor"'
    ' stroke-width="2"><path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z"/>'
    '<circle cx="12" cy="12" r="3"/></svg>'
)
FOCUS_ICON = (
    '<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor"'
    ' stroke-width="2"><path d="M4 8V4h4M16 4h4v4M20 16v4h-4M8 20H4v-4"/>'
    '<circle cx="12" cy="12" r="3"/></svg>'
)


def truck_row(
    vehicle: str,
    stops: list[Stop],
    scale: Scale,
    solution: Solution,
    group: str | None,
) -> str:
    color = readable(stops[0].color)
    trips = split_trips(stops)
    cards = [
        popup_html(stop, stops[index - 1] if index else None, stop_kind_label(stop, solution))
        for index, stop in enumerate(stops)
    ]
    track = "".join(trip_html(stops, cards, a, b, scale, color) for a, b in trips)
    track += "".join(
        job_box(stop, scale, solution, cards[index])
        for index, stop in enumerate(stops)
        if not is_depot(stop)
    )
    distance = max((stop.distance or 0) for stop in stops)
    vehicle_type = solution.vehicle_types.get(vehicle)
    badge = (
        f'<span class="badge">{truck_icon()}{escape(vehicle_type)}</span>'
        if vehicle_type
        else ""
    )
    trips_label = f"{len(trips)} trip{'' if len(trips) == 1 else 's'}"
    group_attr = f' data-group="{escape(group)}"' if group else ""
    return f"""
    <div class="tl-row"{group_attr}>
      <div class="tl-truck">
        <span class="swatch" style="color:{escape(color)}">{truck_icon()}</span>
        <strong>{escape(vehicle)}</strong>
        {badge}
        <span class="meta">{trips_label} · {round(distance / 1000)} km</span>
        <button class="icon-btn" data-action="toggle" title="Show or hide on map">{EYE_ICON}</button>
        <button class="icon-btn" data-action="focus" title="Zoom map to this truck">{FOCUS_ICON}</button>
      </div>
      <div class="tl-track" style="--hours:{scale.hours};--c:{escape(color)}">{track}</div>
    </div>"""


def timeline_html(stops: list[Stop], solution: Solution, groups: dict[str, str]) -> str:
    # unassigned jobs are points without times; they have no place on a timeline
    stops = [stop for stop in stops if stop.arrival and stop.departure]
    scale = timeline_scale(stops)
    if scale is None:
        return '<p class="empty">This GeoJSON has no arrival/departure times, so there is no timeline.</p>'
    grouped = tours_of(stops)
    present = {
        kind
        for stop in stops
        if not is_depot(stop)
        for kind in job_kinds(stop, solution)
    }
    key = "".join(
        f'<span><i style="background:{color}"></i>{label}</span>'
        for kind, (label, color) in [*KIND_STYLES.items(), ("", UNKNOWN_KIND)]
        if kind in present
    )
    ticks = "".join(
        f'<span style="left:{scale.pct(scale.start + timedelta(hours=h)):.4f}%">'
        f'{(scale.start + timedelta(hours=h)).strftime("%H:%M")}</span>'
        for h in range(scale.hours + 1)
    )
    rows = "".join(
        truck_row(vehicle, tour, scale, solution, groups.get(vehicle))
        for vehicle, tour in grouped.items()
    )
    return f"""
    <div class="tl-head">
      <h2>Routes</h2>
      <div class="key"><span class="key-label">Key:</span>{key}</div>
      <span class="tz">Times in UTC</span>
    </div>
    <div class="tl-scroll">
      <div class="tl">
        <div class="tl-row tl-axis"><div></div><div class="tl-ticks">{ticks}</div></div>
        {rows}
      </div>
    </div>"""


def page_html(
    stops: list[Stop],
    fmap: folium.Map,
    groups: dict[str, str],
    solution: Solution,
) -> str:
    grouped = tours_of(stops)
    chips = "".join(
        f'<span class="chip"><i style="background:{escape(tour[0].color)}"></i>{escape(name)}'
        f" · {len(tour)} stops</span>"
        for name, tour in grouped.items()
    )
    timeline = timeline_html(stops, solution, groups)
    root = fmap.get_root()
    root.render()
    header = root.header.render()
    body = f'<div class="folium-map" id="{fmap.get_name()}"></div>'
    script = root.script.render()
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>Route board</title>
  <link rel="preconnect" href="https://fonts.googleapis.com"/>
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
  <link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=IBM+Plex+Sans:wght@400;500;600&display=swap" rel="stylesheet"/>
  {header}
  <style>
{PAGE_STYLE}
  </style>
</head>
<body>
  <header>
    <div>
      <h1>Route board</h1>
      <p>Click a stop on the map for arrival, service, and inbound drive time.</p>
    </div>
    <div class="chips">{chips}</div>
  </header>
  <div class="map-wrap">{body}</div>
  <section class="board">
    {timeline}
  </section>
  <div class="stop-pop" role="dialog" hidden></div>
  <script>{script}</script>
  <script>
    (function () {{
      const pop = document.querySelector(".stop-pop");
      let active = null;
      const closePop = () => {{
        pop.hidden = true;
        active?.classList.remove("is-active");
        active = null;
      }};
      document.querySelectorAll(".tl-track [data-card]").forEach((button) => {{
        button.addEventListener("click", (event) => {{
          event.stopPropagation();
          if (active === button) return closePop();
          closePop();
          active = button;
          button.classList.add("is-active");
          pop.innerHTML = button.dataset.card;
          pop.hidden = false;
          const box = button.getBoundingClientRect();
          const width = pop.offsetWidth;
          const height = pop.offsetHeight;
          const left = Math.min(
            Math.max(8, box.left + box.width / 2 - width / 2),
            document.documentElement.clientWidth - width - 8,
          );
          const above = box.top - height - 10;
          const top = above >= 8 ? above : box.bottom + 10;
          pop.style.left = `${{left + window.scrollX}}px`;
          pop.style.top = `${{top + window.scrollY}}px`;
        }});
      }});
      document.addEventListener("click", (event) => {{
        if (!pop.contains(event.target)) closePop();
      }});
      document.addEventListener("keydown", (event) => {{
        if (event.key === "Escape") closePop();
      }});
      window.addEventListener("resize", closePop);
      document.querySelector(".tl-scroll")?.addEventListener("scroll", closePop);

      const map = window[{json.dumps(fmap.get_name())}];
      document.querySelectorAll(".tl-row[data-group]").forEach((row) => {{
        const group = window[row.dataset.group];
        if (!map || !group) return;
        row.querySelector('[data-action="toggle"]').addEventListener("click", () => {{
          const hidden = map.hasLayer(group);
          hidden ? map.removeLayer(group) : map.addLayer(group);
          row.classList.toggle("is-hidden", hidden);
        }});
        row.querySelector('[data-action="focus"]').addEventListener("click", () => {{
          if (!map.hasLayer(group)) {{
            map.addLayer(group);
            row.classList.remove("is-hidden");
          }}
          map.fitBounds(group.getBounds(), {{ padding: [32, 32] }});
          document.querySelector(".map-wrap").scrollIntoView({{ behavior: "smooth" }});
        }});
      }});
    }})();
  </script>
</body>
</html>
"""


def write_and_open(html: str, output: Path | None, open_browser: bool) -> Path:
    if output is None:
        tmp = tempfile.NamedTemporaryFile(suffix=".html", delete=False)
        tmp.close()
        output = Path(tmp.name)
    output.write_text(html, encoding="utf-8")
    if open_browser:
        webbrowser.open(output.as_uri())
    return output


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Visualize a pragmatic solution GeoJSON."
    )
    parser.add_argument(
        "geojson",
        nargs="?",
        type=Path,
        default=DEFAULT_GEOJSON,
        help="Path to a *.solution*.geojson / *-gjson.json file",
    )
    parser.add_argument(
        "-s",
        "--solution",
        type=Path,
        help="Pragmatic solution JSON with job types (default: next to the GeoJSON)",
    )
    parser.add_argument(
        "-o", "--output", type=Path, help="Write HTML here instead of a temp file"
    )
    parser.add_argument(
        "--no-open", action="store_true", help="Do not open the browser"
    )
    args = parser.parse_args()
    geojson = args.geojson.expanduser().resolve()
    if not geojson.exists():
        raise SystemExit(f"GeoJSON not found: {geojson}")

    stops, routes = load_geojson(geojson)
    if not stops:
        raise SystemExit(f"No stop points in {geojson}")

    solution = load_solution(args.solution or solution_path(geojson))
    fmap, groups = build_map(stops, routes, solution)
    html = page_html(stops, fmap, groups, solution)
    path = write_and_open(html, args.output, open_browser=not args.no_open)
    print(f"Wrote {path}")


if __name__ == "__main__":
    main()
