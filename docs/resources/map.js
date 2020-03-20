let geojson = document.getElementById("geojson");

if (geojson) {
    const map = L.map('map').setView([52.515, 13.413], 12);

    L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
        maxZoom: 19,
        attribution: '&copy; <a href="https://openstreetmap.org/copyright">OpenStreetMap contributors</a>'
    }).addTo(map);

    const geojsonLayer = L.geoJSON(JSON.parse(geojson.innerText), {
        weight: 2,
        style: function (feature) {
            return {
                color: feature.properties.stroke || (feature.properties["marker-symbol"] !== "marker" ? '#000000' : "#cccccc")
            };
        },
        pointToLayer: function (feature, latlng) {
            return L.circleMarker(latlng, {
                radius: 12,
                fillColor: feature.properties["marker-color"],
                fillOpacity: 0.75,
                weight: 2,
            });
        },
        onEachFeature: function (feature, layer) {
            const properties = Object.keys(feature.properties)
                .filter(key => !["marker-symbol", "marker-size", "marker-color", "stroke", "stroke-width"].includes(key))
                .reduce((obj, key) => {
                    obj[key] = feature.properties[key];
                    return obj;
                }, {});

            layer.bindPopup('<pre>' + JSON.stringify(properties, null, ' ')
                .replace(/[{}"]/g, '') + '</pre>');
        }
    });

    geojsonLayer.addTo(map);
    map.fitBounds(geojsonLayer.getBounds());
}
