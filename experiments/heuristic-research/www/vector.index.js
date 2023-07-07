class Chart {}

const canvas1 = document.getElementById("canvas1");
const canvas2 = document.getElementById("canvas2");
const canvas3 = document.getElementById("canvas3");

const coord = document.getElementById("coord");
const plotFunction = document.getElementById("plot-function");
const plotPopulation = document.getElementById("plot-population");
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
    plotFunction.addEventListener("change", changePlot);
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

/** Redraw currently selected plot. */
function updateDynamicPlots() {
    const selected = plotFunction.selectedOptions[0];

    let yaw_value = Number(yaw.value) / 100.0;
    let pitch_value = Number(pitch.value) / 100.0;
    let generation_value = Number(generations.value);
    let heuristic_kind = "best";

    status.innerText = `Rendering ${selected.innerText}...`;

    const start = performance.now();

    switch(selected.value) {
        case 'rosenbrock':
            Chart.rosenbrock(canvas1, generation_value, pitch_value, yaw_value);
            break;
        case 'rastrigin':
            Chart.rastrigin(canvas1, generation_value, pitch_value, yaw_value);
            break;
        case 'himmelblau':
            Chart.himmelblau(canvas1, generation_value, pitch_value, yaw_value);
            break;
        case 'ackley':
            Chart.ackley(canvas1, generation_value, pitch_value, yaw_value);
            break;
        case 'matyas':
            Chart.matyas(canvas1, generation_value, pitch_value, yaw_value);
            break;
        default:
            break;
    }

    Chart.heuristic_estimations(canvas2, generation_value, heuristic_kind);
    
    const end = performance.now();

    coord.innerText = `Pitch:${pitch_value}, Yaw:${yaw_value}`
    status.innerText = `Gen: ${generation_value}, ${selected.innerText}, ${Math.ceil(end - start)}ms`;
}

function updateStaticPlots() {
    Chart.fitness_func(canvas3)
}

/** Runs experiment. */
function runExperiment() {
    // TODO configure parameters from outside
    let max_gen = 2000
    let function_name = plotFunction.selectedOptions[0].value;
    let population_type = plotPopulation.selectedOptions[0].value;

    function getRandomInRange(min, max) {
        return (Math.random() * (max - min) + min)
    }

    var x = 0.0, z = 0.0;
    switch(function_name) {
        case 'rosenbrock':
            x = getRandomInRange(-2.0, 2.0)
            z = getRandomInRange(-2.0, 2.0)
            break;
        case 'rastrigin':
            x = getRandomInRange(-5.12, 5.12)
            z = getRandomInRange(-5.12, 5.12)
            break;
        case 'himmelblau':
            x = getRandomInRange(-5.0, 5.0)
            z = getRandomInRange(-5.0, 5.0)
            break;
        case 'ackley':
            x = getRandomInRange(-5.0, 5.0)
            z = getRandomInRange(-5.0, 5.0)
            break;
        case 'matyas':
            x = getRandomInRange(-10.0, 10.0)
            z = getRandomInRange(-10.0, 10.0)
            break;
        default:
            break;
    }

    console.log(`init point is: (${x}, ${z})`)

    // NOTE: a blocking call here
    Chart.run_experiment(function_name, population_type, x, z, max_gen);
    updateDynamicPlots();
    updateStaticPlots();
    generations.max = max_gen;
    generations.classList.remove("hide");
}