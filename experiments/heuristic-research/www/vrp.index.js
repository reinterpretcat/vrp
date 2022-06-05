class Chart {}

const fileSelector = document.getElementById("file-selector");
const plotPopulation = document.getElementById("plot-population");
const run = document.getElementById("run");
const generations = document.getElementById("generations");
const vrpFormat = document.getElementById("vrp-format");
var svg;

/** Main entry point */
export function main() {
    setupUI();
}

/** This function is used in `vrp.bootstrap.js` to setup imports. */
export function setup(run_vrp_experiment, get_data_graphs, clear) {
    Chart.run_experiment = run_vrp_experiment;
    Chart.get_data_graphs = get_data_graphs;
    Chart.clear = clear;
}

function setupUI() {
    fileSelector.addEventListener("change", openFile);
    run.addEventListener("click", runExperiment)
    generations.addEventListener("input", updatePlot);

    createSvg();
}

function createSvg() {
    d3.selectAll("svg").remove();
    svg = d3.select("#svg").append("svg")
        .attr("width", 600)
        .attr("height", 400);

    svg = svg.append('g');
    svg.append('rect').attr({
        'fill': '#111155',
        'width': 600,
        'height': 400
    });
    svg.attr('transform', 'translate(20, 20)');
}

function openFile(event) {
    let input = event.target;
    let reader = new FileReader();

    reader.onload = function(){
        let content = reader.result;
        console.log(content.substring(0, 300));

        Chart.problem = content;

        run.classList.remove("hide");
    };
    reader.readAsText(input.files[0]);
}

/** Runs experiment. */
function runExperiment() {
    let max_gen = 200
    let population_type = plotPopulation.selectedOptions[0].value;
    let format_type = vrpFormat.selectedOptions[0].value;

    Chart.run_experiment(format_type, Chart.problem, population_type, max_gen);

    generations.max = max_gen;
    generations.classList.remove("hide");

    updatePlot();
}

function updatePlot() {
    let generation_value = Number(generations.value);
    let graph = JSON.parse(Chart.get_data_graphs(generation_value));

    console.log(graph);

    let nodes = graph.nodes.reduce((acc, node, index) => Object.assign(acc, {[index]: node}), {});
    let edges = graph.edges;

    //Run the FDEB algorithm using default values on the data
    let fbundling = d3.ForceEdgeBundling().nodes(nodes).edges(edges);
    let results = fbundling();

    createSvg();

    let d3line = d3.svg.line()
        .x(function (d) {
            return d.x;
        })
        .y(function (d) {
            return d.y;
        })
        .interpolate("linear");
    //plot the data
    for (var i = 0; i < results.length; i++) {
        svg.append("path").attr("d", d3line(results[i]))
            .style("stroke-width", 1)
            .style("stroke", "#ff2222")
            .style("fill", "none")
            .style('stroke-opacity', 0.115);
    }

    //draw nodes
    svg.selectAll('.node')
        .data(d3.entries(nodes))
        .enter()
        .append('circle')
        .classed('node', true)
        .attr({
            'r': 2,
            'fill': '#ffee00'
        })
        .attr('cx', function (d) {
            return d.value.x;
        })
        .attr('cy', function (d) {
            return d.value.y;
        });
}