class Chart {}

const solutionCanvas = document.getElementById("solutionCanvas");
const searchCanvas = document.getElementById("searchCanvas");
const fitnessCanvas = document.getElementById("fitnessCanvas");

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
    setupCanvas(solutionCanvas, 800);
    setupCanvas(searchCanvas, 800);
    setupCanvas(fitnessCanvas, 800);
    updateDynamicPlots();
    updateStaticPlots();
}

/** This function is used in `vector.bootstrap.js` to setup imports. */
export function setup(WasmChart, run_function_experiment, load_state, clear) {
    Chart = WasmChart;
    Chart.run_experiment = run_function_experiment;
    Chart.load_state = load_state;
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
function setupCanvas(canvas, size) {
    const aspectRatio = canvas.width / canvas.height;
    //const size = canvas.parentNode.offsetWidth * 1.2;
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

        Chart.data = content;

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

    Chart.vrp(solutionCanvas, generation_value, pitch_value, yaw_value);
    Chart.search_estimations( searchCanvas, generation_value, heuristic_kind);
    
    const end = performance.now();

    coord.innerText = `Pitch:${pitch_value}, Yaw:${yaw_value}`
    status.innerText = `Generation: ${generation_value} in ${Math.ceil(end - start)}ms`;
}

function updateStaticPlots() {
    Chart.fitness_vrp(fitnessCanvas)
}

/** Runs experiment. */
function runExperiment() {
    // TODO configure parameters from outside
    let max_gen = 2000
    let format_type = vrpFormat.selectedOptions[0].value;

    if (format_type === "state") {
        max_gen = Chart.load_state(Chart.data);
    } else {
        let population_type = plotPopulation.selectedOptions[0].value;
        Chart.run_experiment(format_type, Chart.data, population_type, max_gen);
    }


    updateDynamicPlots();
    updateStaticPlots();
    generations.max = max_gen;
    generations.classList.remove("hide");
}
