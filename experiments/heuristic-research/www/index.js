class Chart {}

const canvas = document.getElementById("canvas");
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
    setupCanvas();
}

/** This function is used in `bootstrap.js` to setup imports. */
export function setup(WasmChart, run_experiment, clear) {
    Chart = WasmChart;
    Chart.run_experiment = run_experiment;
    Chart.clear = clear;
}

/** Add event listeners. */
function setupUI() {
    status.innerText = "WebAssembly loaded!";
    plotFunction.addEventListener("change", changePlot);
    plotPopulation.addEventListener("change", changePlot);

    yaw.addEventListener("change", updatePlot);
    pitch.addEventListener("change", updatePlot);
    generations.addEventListener("change", updatePlot);

    yaw.addEventListener("input", updatePlot);
    pitch.addEventListener("input", updatePlot);
    generations.addEventListener("input", updatePlot);

    run.addEventListener("click", runExperiment)
    window.addEventListener("resize", setupCanvas);
}

/** Setup canvas to properly handle high DPI and redraw current plot. */
function setupCanvas() {
    const aspectRatio = canvas.width / canvas.height;
    const size = canvas.parentNode.offsetWidth * 1.2;
    canvas.style.width = size + "px";
    canvas.style.height = size / aspectRatio + "px";
    canvas.width = size;
    canvas.height = size / aspectRatio;
    updatePlot();
}

/** Changes plot **/
function changePlot() {
    Chart.clear()
    generations.classList.add("hide");
    updatePlot()
}

/** Redraw currently selected plot. */
function updatePlot() {
    const selected = plotFunction.selectedOptions[0];

    let yaw_value = Number(yaw.value) / 100.0;
    let pitch_value = Number(pitch.value) / 100.0;
    let generation_value = Number(generations.value);

    status.innerText = `Rendering ${selected.innerText}...`;

    const start = performance.now();

    switch(selected.value) {
        case 'rosenbrock':
            Chart.rosenbrock(canvas, generation_value, pitch_value, yaw_value);
            break;
        case 'rastrigin':
            Chart.rastrigin(canvas, generation_value, pitch_value, yaw_value);
            break;
        case 'himmelblau':
            Chart.himmelblau(canvas, generation_value, pitch_value, yaw_value);
            break;
        case 'ackley':
            Chart.ackley(canvas, generation_value, pitch_value, yaw_value);
            break;
        case 'matyas':
            Chart.matyas(canvas, generation_value, pitch_value, yaw_value);
            break;
        default:
            break;
    }
    
    const end = performance.now();

    coord.innerText = `Pitch:${pitch_value}, Yaw:${yaw_value}`
    status.innerText = `Generation: ${generation_value}, rendered ${selected.innerText} in ${Math.ceil(end - start)}ms`;
}

/** Runs experiment. */
function runExperiment() {
    // TODO configure parameters from outside
    let max_gen = 2000
    let function_name = plotFunction.selectedOptions[0].value;
    let population_type = plotPopulation.selectedOptions[0].value;

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
    updatePlot();
    generations.max = max_gen;
    generations.classList.remove("hide");
}

function getRandomInRange(min, max) {
    return (Math.random() * (max - min) + min)
}