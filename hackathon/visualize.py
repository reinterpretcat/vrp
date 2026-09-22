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
import plotly.graph_objects as go

HERE = Path(__file__).resolve().parent
DEFAULT_GEOJSON = HERE / "data" / "sites.basic.solution-gjson.json"
DEFAULT_COLOR = "#3388ff"
MIN_BAR = timedelta(seconds=1)
PAGE_BG = "#0b0f14"

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


class Segment(NamedTuple):
    vehicle: str
    start: datetime
    end: datetime
    color: str
    label: str
    detail: str


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


def distance_gap(stop: Stop, previous: Stop | None) -> int | None:
    if previous is None or stop.distance is None or previous.distance is None:
        return None
    return stop.distance - previous.distance


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
                    color=str(props.get("stroke") or color_by_tour.get(tour_idx, DEFAULT_COLOR)),
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
    return f"<div class='row'><span>{label}</span><strong>{escape(value)}</strong></div>"


def popup_html(stop: Stop, previous: Stop | None) -> str:
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
      {_popup_row("Arrival", format_clock(stop.arrival))}
      {_popup_row("Departure", format_clock(stop.departure))}
      {service}
      {drive}
      {_popup_row("From depot", format_distance(stop.distance))}
    </div>
    """


def build_map(stops: list[Stop], routes: list[Route]) -> folium.Map:
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
    fmap.default_js = [item for item in folium.Map.default_js if item[0] in {"leaflet", "jquery"}]
    fmap.default_css = [item for item in folium.Map.default_css if item[0] == "leaflet_css"]
    for route in routes:
        folium.PolyLine(
            locations=[(lat, lng) for lng, lat in route.coordinates],
            color=route.color,
            weight=4,
            opacity=0.85,
            tooltip=route.vehicle_id,
        ).add_to(fmap)

    previous_by_vehicle: dict[str, Stop] = {}
    for stop in stops:
        previous = previous_by_vehicle.get(stop.vehicle_id)
        depot = stop.symbol == "warehouse"
        folium.CircleMarker(
            location=(stop.lat, stop.lng),
            radius=9 if depot else 7,
            color="#f4f1e8",
            weight=2,
            fill=True,
            fill_color=stop.color,
            fill_opacity=1.0,
            popup=folium.Popup(popup_html(stop, previous), max_width=320),
            tooltip=f"{stop.stop_idx} · {stop.jobs}",
        ).add_to(fmap)
        previous_by_vehicle[stop.vehicle_id] = stop

    fmap.fit_bounds(
        [[min(latitudes), min(longitudes)], [max(latitudes), max(longitudes)]],
        padding=(24, 24),
    )
    return fmap


def timeline_segments(grouped: dict[str, list[Stop]]) -> tuple[list[Segment], list[Segment]]:
    driving: list[Segment] = []
    staying: list[Segment] = []
    for vehicle, stops in grouped.items():
        color = stops[0].color if stops else "#3cb44b"
        for index, stop in enumerate(stops):
            previous = stops[index - 1] if index else None
            if previous and previous.departure and stop.arrival and stop.arrival > previous.departure:
                driving.append(
                    Segment(
                        vehicle=vehicle,
                        start=previous.departure,
                        end=stop.arrival,
                        color=mix_hex(color, onto="#9aa7b4", weight=0.55),
                        label=f"Drive to {stop.jobs}",
                        detail=f"{format_duration(stop.arrival - previous.departure)} · {format_distance(distance_gap(stop, previous))}",
                    )
                )
            if stop.arrival and stop.departure:
                end = stop.departure if stop.departure > stop.arrival else stop.arrival + MIN_BAR
                staying.append(
                    Segment(
                        vehicle=vehicle,
                        start=stop.arrival,
                        end=end,
                        color=color,
                        label=stop.jobs,
                        detail=f"{format_clock(stop.arrival)}–{format_clock(stop.departure)} · stop {stop.stop_idx}",
                    )
                )
    return driving, staying


def add_bars(fig: go.Figure, rows: list[Segment], name: str, width: float) -> None:
    if not rows:
        return
    fig.add_trace(
        go.Bar(
            name=name,
            y=[row.vehicle for row in rows],
            x=[(row.end - row.start).total_seconds() * 1000 for row in rows],
            base=[row.start for row in rows],
            orientation="h",
            marker=dict(color=[row.color for row in rows], line=dict(width=0)),
            width=width,
            customdata=[[row.label, row.detail] for row in rows],
            hovertemplate="<b>%{customdata[0]}</b><br>%{customdata[1]}<extra></extra>",
        )
    )


def build_timeline(stops: list[Stop]) -> go.Figure | None:
    grouped = tours_of(stops)
    driving, staying = timeline_segments(grouped)
    if not driving and not staying:
        return None

    fig = go.Figure()
    add_bars(fig, driving, "Driving", 0.28)
    add_bars(fig, staying, "Stop", 0.52)
    fig.update_layout(
        barmode="overlay",
        paper_bgcolor=PAGE_BG,
        plot_bgcolor=PAGE_BG,
        font=dict(family="IBM Plex Sans, sans-serif", color="#d7dde6", size=13),
        margin=dict(l=90, r=24, t=16, b=48),
        legend=dict(orientation="h", yanchor="bottom", y=1.02, x=0, bgcolor="rgba(0,0,0,0)"),
        bargap=0.35,
        height=max(280, 140 * len(grouped) + 80),
        hoverlabel=dict(bgcolor="#161d27", font_size=12, font_family="IBM Plex Sans"),
    )
    fig.update_xaxes(
        type="date",
        tickformat="%H:%M",
        gridcolor="#243042",
        linecolor="#243042",
        zeroline=False,
        title="Time (UTC)",
    )
    fig.update_yaxes(
        type="category",
        categoryorder="array",
        categoryarray=list(grouped.keys()),
        autorange="reversed",
        gridcolor="#243042",
        linecolor="#243042",
        title="",
    )
    return fig


def page_html(stops: list[Stop], fmap: folium.Map, figure: go.Figure | None) -> str:
    grouped = tours_of(stops)
    chips = "".join(
        f'<span class="chip"><i style="background:{escape(tour[0].color)}"></i>{escape(name)}'
        f" · {len(tour)} stops</span>"
        for name, tour in grouped.items()
    )
    timeline = (
        figure.to_html(full_html=False, include_plotlyjs="cdn")
        if figure
        else '<p class="empty">This GeoJSON has no arrival/departure times, so there is no timeline.</p>'
    )
    root = fmap.get_root()
    root.render()
    header = root.header.render()
    body = f'<div class="folium-map" id="{fmap.get_name()}"></div>'
    script = root.script.render()
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
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
    <h2>Truck timeline</h2>
    {timeline}
  </section>
  <script>{script}</script>
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
    parser = argparse.ArgumentParser(description="Visualize a pragmatic solution GeoJSON.")
    parser.add_argument(
        "geojson",
        nargs="?",
        type=Path,
        default=DEFAULT_GEOJSON,
        help="Path to a *.solution*.geojson / *-gjson.json file",
    )
    parser.add_argument("-o", "--output", type=Path, help="Write HTML here instead of a temp file")
    parser.add_argument("--no-open", action="store_true", help="Do not open the browser")
    args = parser.parse_args()
    geojson = args.geojson.expanduser().resolve()
    if not geojson.exists():
        raise SystemExit(f"GeoJSON not found: {geojson}")

    stops, routes = load_geojson(geojson)
    if not stops:
        raise SystemExit(f"No stop points in {geojson}")

    fmap = build_map(stops, routes)
    figure = build_timeline(stops)
    html = page_html(stops, fmap, figure)
    path = write_and_open(html, args.output, open_browser=not args.no_open)
    print(f"Wrote {path}")


if __name__ == "__main__":
    main()
