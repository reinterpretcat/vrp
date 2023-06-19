class Chart {}

const canvas1 = document.getElementById("canvas1");
const canvas2 = document.getElementById("canvas2");
const canvas3 = document.getElementById("canvas3");

const coord = document.getElementById("coord");
const fileSelector = document.getElementById("file-selector");
const plotPopulation = document.getElementById("plot-population");
const vrpFormat = document.getElementById("vrp-format");
const pitch = document.getElementById("pitch");
const yaw = document.getElementById("yaw");
const status = document.getElementById("status");
const run = document.getElementById("run");
const generations = document.getElementById("generations");

/** Main entry point */
export function main() {
    setupUI();
    setupCanvas(canvas1);
    setupCanvas(canvas2);
    setupCanvas(canvas3);
    updateDynamicPlots();
    updateStaticPlots();
}

/** This function is used in `vector.bootstrap.js` to setup imports. */
export function setup(WasmChart, run_function_experiment, clear) {
    Chart = WasmChart;
    Chart.run_experiment = run_function_experiment;
    Chart.clear = clear;
}

/** Add event listeners. */
function setupUI() {
    status.innerText = "WebAssembly loaded!";
    fileSelector.addEventListener("change", openFile);
    plotPopulation.addEventListener("change", changePlot);

    yaw.addEventListener("change", updateDynamicPlots);
    pitch.addEventListener("change", updateDynamicPlots);
    generations.addEventListener("change", updateDynamicPlots);

    yaw.addEventListener("input", updateDynamicPlots);
    pitch.addEventListener("input", updateDynamicPlots);
    generations.addEventListener("input", updateDynamicPlots);

    run.addEventListener("click", runExperiment)
    window.addEventListener("resize", setupCanvas);
}

/** Setup canvas to properly handle high DPI and redraw current plot. */
function setupCanvas(canvas) {
    const aspectRatio = canvas.width / canvas.height;
    const size = canvas.parentNode.offsetWidth * 1.2;
    canvas.style.width = size + "px";
    canvas.style.height = size / aspectRatio + "px";
    canvas.width = size;
    canvas.height = size / aspectRatio;
}

/** Changes plot **/
function changePlot() {
    Chart.clear()
    generations.classList.add("hide");
    updateDynamicPlots()
    updateStaticPlots();
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

/** Redraw currently selected plot. */
function updateDynamicPlots() {
    let yaw_value = Number(yaw.value) / 100.0;
    let pitch_value = Number(pitch.value) / 100.0;
    let generation_value = Number(generations.value);
    let heuristic_kind = "best";

    const start = performance.now();

    Chart.vrp(canvas1, generation_value, pitch_value, yaw_value);
    Chart.heuristic_estimations(canvas2, generation_value, heuristic_kind);
    
    const end = performance.now();

    coord.innerText = `Pitch:${pitch_value}, Yaw:${yaw_value}`
    status.innerText = `Generation: ${generation_value} in ${Math.ceil(end - start)}ms`;
}

function updateStaticPlots() {
    Chart.fitness_vrp(canvas3)
}

/** Runs experiment. */
function runExperiment() {
    // TODO configure parameters from outside
    let max_gen = 2000
    let population_type = plotPopulation.selectedOptions[0].value;
    let format_type = vrpFormat.selectedOptions[0].value;

    Chart.run_experiment(format_type, Chart.problem, population_type, max_gen);

    updateDynamicPlots();
    updateStaticPlots();
    generations.max = max_gen;
    generations.classList.remove("hide");
}
