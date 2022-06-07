class Chart {}

const fileSelector = document.getElementById("file-selector");
const plotPopulation = document.getElementById("plot-population");
const run = document.getElementById("run");
const generations = document.getElementById("generations");
const vrpFormat = document.getElementById("vrp-format");
const status = document.getElementById("status");
var svg;
var bundling_cache = {};

/** Main entry point */
export function main() {
    setupUI();
}

/** This function is used in `vrp.bootstrap.js` to setup imports. */
export function setup(run_vrp_experiment, get_bundled_edges, clear) {
    Chart.run_experiment = run_vrp_experiment;
    Chart.get_bundled_edges = get_bundled_edges;
    Chart.clear = clear;
}

function setupUI() {
    fileSelector.addEventListener("change", openFile);
    run.addEventListener("click", runExperiment)
    generations.addEventListener("input", updatePlot);

    createSvg([]);
}

function createSvg(nodes) {
    d3.selectAll("svg").remove();
    // set the dimensions and margins of the graph
    let margin = {top: 10, right: 40, bottom: 30, left: 30},
        width = 600 - margin.left - margin.right,
        height = 400 - margin.top - margin.bottom;

    let x_max = d3.max(nodes, node => node.x) || 100;
    let x_min = d3.min(nodes, node => node.x) || 0;

    let y_max = d3.max(nodes, node => node.y) || 100;
    let y_min = d3.min(nodes, node => node.y) || 0;

// append the svg object to the body of the page
    svg = d3.select("#svg")
        .append("svg")
        .attr("width", width + margin.left + margin.right)
        .attr("height", height + margin.top + margin.bottom)
        .append("g")
        .attr("transform", "translate(" + margin.left + "," + margin.top + ")");

    let xAxis = d3.scaleLinear()
        .domain([x_min, x_max])
        .range([0, width]);
    svg
        .append('g')
        .attr("transform", "translate(0," + height + ")")
        .call(d3.axisBottom(xAxis));

    let yAxis = d3.scaleLinear()
        .domain([y_min, y_max])
        .range([height, 0]);
    svg
        .append('g')
        .call(d3.axisLeft(yAxis));

    return [xAxis, yAxis];
}

function openFile(event) {
    let input = event.target;
    let reader = new FileReader();

    reader.onload = function () {
        let content = reader.result;
        console.log(content.substring(0, 300));

        Chart.problem = content;

        run.classList.remove("hide");
    };
    reader.readAsText(input.files[0]);
}

/** Runs experiment. */
function runExperiment() {
    let max_gen = 2000;
    let population_type = plotPopulation.selectedOptions[0].value;
    let format_type = vrpFormat.selectedOptions[0].value;

    Chart.run_experiment(format_type, Chart.problem, population_type, max_gen);

    generations.max = max_gen;
    generations.classList.remove("hide");
    bundling_cache = {};

    updatePlot();
}

function updatePlot() {
    let generation_value = Number(generations.value);
    const marker1 = performance.now();
    bundling_cache[generation_value] = bundling_cache[generation_value] || JSON.parse(Chart.get_bundled_edges(generation_value));
    let graph = bundling_cache[generation_value];
    const marker2 = performance.now();

    let [xAxis, yAxis] = createSvg(graph.nodes);

    let d3line = d3.line()
        .x(d => xAxis(d.x))
        .y(d => yAxis(d.y))
        .curve(d3.curveLinear);

    // draw edges
    for (var i = 0; i < graph.edges.length; i++) {
        svg.append("path")
            .attr("d", d3line(graph.edges[i]))
            .style("stroke-width", 1)
            .style("stroke", "#ff2222")
            .style("fill", "none")
            .style('stroke-opacity', 0.115);
    }

    // draw nodes
    svg.selectAll('.node')
        .data(Object.entries(graph.nodes))
        .enter()
        .append('circle')
        .classed('node', true)
        .attr('r', 4)
        .attr('fill', '#ffee00')
        .attr('cx', d => xAxis(d[1].x))
        .attr('cy', d => yAxis(d[1].y));

    const end = performance.now();
    status.innerText = `generation: ${generation_value}, nodes: ${graph.nodes.length}, edges: ${graph.edges.length}, bundling: ${Math.ceil(marker2 - marker1)}ms, drawing: ${Math.ceil(end - marker2)}ms`;
}