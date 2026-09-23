#!/usr/bin/env python3
"""Build a pragmatic-format VRP dataset from sites.csv."""

from __future__ import annotations

import csv
import json
import os
import random
import time
import urllib.error
import urllib.parse
import urllib.request
from collections import Counter
from itertools import product
from pathlib import Path
from typing import NamedTuple

WASTE_FACILITY = {"lat": 59.93028, "lng": 10.82860}
SERVICE_DURATION = 50.0
SHIFT_START = "2019-07-04T09:00:00Z"
SHIFT_END = "2019-07-04T18:00:00Z"
TRUCK_CAPACITY = 100
TRUCK_IDS = ["truck_1", "truck_2"]
COMPATIBILITIES = ("organic", "mixed waste")
COMPATIBILITY_SEED = 0
PROFILE = "normal_car"
MAPBOX_PROFILE = "mapbox/driving"
MAPBOX_COORD_LIMIT = 25
MAPBOX_CHUNK = MAPBOX_COORD_LIMIT // 2
MAPBOX_ATTEMPTS = 6
MAPBOX_TIMEOUT = 120
MAPBOX_TILE_PAUSE = 1.05
MAPBOX_MATRIX_URL = "https://api.mapbox.com/directions-matrix/v1/{profile}/{coordinates}"

HERE = Path(__file__).resolve().parent
ENV_PATH = HERE / ".env"
SITES_CSV = HERE / "data" / "sites.csv"
PROBLEM_PATH = HERE / "data" / "sites.basic.problem.json"
LOCATIONS_PATH = HERE / "data" / "sites.basic.locations.json"
MATRIX_PATH = HERE / "data" / "sites.basic.matrix.json"


class Site(NamedTuple):
    name: str
    lat: float
    lng: float


def load_sites(path: Path) -> list[Site]:
    sites: list[Site] = []
    seen: set[tuple[str, float, float]] = set()
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            name = row["name"].strip()
            lat = float(row["latitude"])
            lng = float(row["longitude"])
            key = (name, round(lat, 6), round(lng, 6))
            if key not in seen:
                seen.add(key)
                sites.append(Site(name, lat, lng))
    return sites


def job_ids(sites: list[Site]) -> list[str]:
    totals = Counter(site.name for site in sites)
    seen: Counter[str] = Counter()
    ids: list[str] = []
    for site in sites:
        if totals[site.name] == 1:
            ids.append(site.name)
        else:
            seen[site.name] += 1
            ids.append(f"{site.name} #{seen[site.name]}")
    return ids


def job_compatibilities(count: int) -> list[str]:
    rng = random.Random(COMPATIBILITY_SEED)
    return [rng.choice(COMPATIBILITIES) for _ in range(count)]


def pickup_job(job_id: str, site: Site, compatibility: str) -> dict:
    return {
        "id": job_id,
        "compatibility": compatibility,
        "pickups": [
            {
                "places": [
                    {
                        "location": {"lat": site.lat, "lng": site.lng},
                        "duration": SERVICE_DURATION,
                    }
                ],
                "demand": [1],
            }
        ],
    }


def fleet() -> dict:
    return {
        "vehicles": [
            {
                "typeId": "truck",
                "vehicleIds": TRUCK_IDS,
                "profile": {"matrix": PROFILE},
                "costs": {
                    "fixed": 22.0,
                    "distance": 0.0002,
                    "time": 0.004806,
                },
                "shifts": [
                    {
                        "start": {
                            "earliest": SHIFT_START,
                            "location": dict(WASTE_FACILITY),
                        },
                        "end": {
                            "latest": SHIFT_END,
                            "location": dict(WASTE_FACILITY),
                        },
                    }
                ],
                "capacity": [TRUCK_CAPACITY],
            }
        ],
        "profiles": [{"name": PROFILE}],
    }


def build_problem(sites: list[Site]) -> dict:
    ids = job_ids(sites)
    jobs = [
        pickup_job(job_id, site, compatibility)
        for job_id, site, compatibility in zip(
            ids, sites, job_compatibilities(len(ids)), strict=True
        )
    ]
    return {"plan": {"jobs": jobs}, "fleet": fleet()}


def unique_locations(sites: list[Site]) -> list[dict]:
    locations: list[dict] = []
    seen: set[tuple[float, float]] = set()
    for site in sites:
        key = (site.lat, site.lng)
        if key not in seen:
            seen.add(key)
            locations.append({"lat": site.lat, "lng": site.lng})
    depot = (WASTE_FACILITY["lat"], WASTE_FACILITY["lng"])
    if depot not in seen:
        locations.append(dict(WASTE_FACILITY))
    return locations


def load_dotenv(path: Path) -> None:
    if not path.exists():
        return
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        os.environ.setdefault(key.strip(), value.strip().strip("'").strip('"'))


def mapbox_token() -> str:
    load_dotenv(ENV_PATH)
    token = os.environ.get("MAPBOX_TOKEN")
    if not token:
        raise SystemExit("MAPBOX_TOKEN is not set; add it to hackathon/.env")
    return token


def index_tiles(count: int, size: int) -> list[range]:
    return [range(start, min(start + size, count)) for start in range(0, count, size)]


def flatten(grid: list[list[int]]) -> list[int]:
    return [value for row in grid for value in row]


def request_json(url: str) -> dict:
    last_error: Exception | None = None
    for attempt in range(MAPBOX_ATTEMPTS):
        try:
            with urllib.request.urlopen(url, timeout=MAPBOX_TIMEOUT) as response:
                payload = json.loads(response.read().decode())
            if payload.get("code") != "Ok":
                raise RuntimeError(f"Mapbox Matrix API error: {payload}")
            return payload
        except urllib.error.HTTPError as err:
            body = err.read().decode(errors="replace")
            if err.code != 429:
                raise RuntimeError(f"Mapbox Matrix API HTTP {err.code}: {body}") from err
            retry_after = err.headers.get("Retry-After", "")
            delay = float(retry_after) if retry_after.isdigit() else 2 ** attempt
            last_error = RuntimeError(f"Mapbox Matrix API rate limited: {body}")
        except urllib.error.URLError as err:
            last_error = RuntimeError(f"Mapbox Matrix API request failed: {err}")
            delay = 2 ** attempt
        time.sleep(delay)
    raise last_error or RuntimeError("Mapbox Matrix API request failed")


def fetch_matrix_block(
    locations: list[dict], sources: range, dests: range, token: str
) -> tuple[list, list]:
    coord_order = list(dict.fromkeys((*sources, *dests)))
    if len(coord_order) > MAPBOX_COORD_LIMIT:
        raise RuntimeError(
            f"Mapbox request has {len(coord_order)} coordinates; limit is {MAPBOX_COORD_LIMIT}"
        )
    if len(coord_order) < 2:
        raise RuntimeError("Mapbox Matrix API needs at least two coordinates")

    index_of = {idx: pos for pos, idx in enumerate(coord_order)}
    coordinates = ";".join(f"{locations[i]['lng']},{locations[i]['lat']}" for i in coord_order)
    query = urllib.parse.urlencode(
        {
            "annotations": "duration,distance",
            "sources": ";".join(str(index_of[i]) for i in sources),
            "destinations": ";".join(str(index_of[i]) for i in dests),
            "access_token": token,
        }
    )
    path = MAPBOX_MATRIX_URL.format(profile=MAPBOX_PROFILE, coordinates=coordinates)
    payload = request_json(f"{path}?{query}")
    durations, distances = payload.get("durations"), payload.get("distances")
    if durations is None or distances is None:
        raise RuntimeError(
            f"Mapbox Matrix API response missing durations or distances: {payload}"
        )
    return durations, distances


def build_routing_matrix(locations: list[dict], token: str) -> dict:
    n = len(locations)
    travel = [[0] * n for _ in range(n)]
    dist = [[0] * n for _ in range(n)]
    errors = [[0] * n for _ in range(n)]
    tiles = index_tiles(n, MAPBOX_CHUNK)
    total = len(tiles) ** 2
    for request_idx, (sources, dests) in enumerate(product(tiles, tiles), start=1):
        print(
            f"Mapbox matrix tile {request_idx}/{total} "
            f"({sources.start}:{sources.stop} → {dests.start}:{dests.stop})"
        )
        if sources == dests and len(sources) == 1:
            continue
        durations, distances = fetch_matrix_block(locations, sources, dests, token)
        for source, duration_row, distance_row in zip(
            sources, durations, distances, strict=True
        ):
            for dest, duration, distance in zip(
                dests, duration_row, distance_row, strict=True
            ):
                if duration is None or distance is None:
                    errors[source][dest] = 1
                else:
                    travel[source][dest] = round(duration)
                    dist[source][dest] = round(distance)
        if request_idx < total:
            time.sleep(MAPBOX_TILE_PAUSE)

    matrix = {
        "profile": PROFILE,
        "travelTimes": flatten(travel),
        "distances": flatten(dist),
    }
    error_codes = flatten(errors)
    if any(error_codes):
        matrix["errorCodes"] = error_codes
    return matrix


def write_json(path: Path, data: object) -> None:
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def read_json(path: Path) -> object | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def cached_matrix(locations: list[dict]) -> dict | None:
    if read_json(LOCATIONS_PATH) != json.loads(json.dumps(locations)):
        return None
    matrix = read_json(MATRIX_PATH)
    if not isinstance(matrix, dict) or matrix.get("profile") != PROFILE:
        return None
    travel_times, distances = matrix.get("travelTimes"), matrix.get("distances")
    expected = len(locations) ** 2
    if not isinstance(travel_times, list) or len(travel_times) != expected:
        return None
    if not isinstance(distances, list) or len(distances) != expected:
        return None
    return matrix


def routing_matrix(locations: list[dict]) -> dict:
    matrix = cached_matrix(locations)
    if matrix is None:
        return build_routing_matrix(locations, mapbox_token())
    print(f"Reusing cached {len(locations)}×{len(locations)} matrix from {MATRIX_PATH}")
    return matrix


def main() -> None:
    sites = load_sites(SITES_CSV)
    locations = unique_locations(sites)
    matrix = routing_matrix(locations)
    write_json(PROBLEM_PATH, build_problem(sites))
    write_json(LOCATIONS_PATH, locations)
    write_json(MATRIX_PATH, matrix)
    print(f"Wrote {len(sites)} pickup jobs and {len(TRUCK_IDS)} trucks to {PROBLEM_PATH}")
    print(f"Wrote {len(locations)} unique locations to {LOCATIONS_PATH}")
    print(f"Wrote {len(locations)}×{len(locations)} routing matrix to {MATRIX_PATH}")


if __name__ == "__main__":
    main()
