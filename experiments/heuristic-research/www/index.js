class Chart {}

const solutionCanvas = document.getElementById("solutionCanvas");
const searchCanvas = document.getElementById("searchCanvas");
const overallCanvas = document.getElementById("overallCanvas");
const bestCanvas = document.getElementById("bestCanvas");
const durationCanvas = document.getElementById("durationCanvas");
const fitnessCanvas = document.getElementById("fitnessCanvas");

const coordLabel = document.getElementById("coordLabel");
const fileSelector = document.getElementById("fileSelector");
const plotPopulation = document.getElementById("plotPopulation");
const plotFunction = document.getElementById("plotFunction");
const vrpFormat = document.getElementById("vrpFormat");
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
    setupCanvas(overallCanvas, 800);
    setupCanvas(bestCanvas, 800);
    setupCanvas(durationCanvas, 800);
    setupCanvas(fitnessCanvas, 800);

    updateDynamicPlots();
    updateStaticPlots();

    document.addEventListener('keydown', function(event) {
        switch (event.key) {
            case "ArrowLeft":
                generations.value = Math.max(parseInt(generations.value) - 1, parseInt(generations.min));
                updatePlots();
                break;
            case "ArrowRight":
                generations.value = Math.min(parseInt(generations.value) + 1, parseInt(generations.max));
                updatePlots();
                break;
        }
    });
}

/** This function is used in `vector.bootstrap.js` to setup imports. */
export function setup(WasmChart, run_function_experiment, run_vrp_experiment, load_state, clear) {
    Chart = WasmChart;
    Chart.run_function_experiment = run_function_experiment;
    Chart.run_vrp_experiment = run_vrp_experiment;
    Chart.load_state = load_state;
    Chart.clear = clear;
}

/** Add event listeners. */
function setupUI() {
    status.innerText = "WebAssembly loaded!";
    fileSelector.addEventListener("change", openFile);
    plotFunction.addEventListener("change", changePlot);
    plotPopulation.addEventListener("change", changePlot);

    yaw.addEventListener("change", updatePlots);
    pitch.addEventListener("change", updatePlots);
    generations.addEventListener("change", updatePlots);

    yaw.addEventListener("input", updatePlots);
    pitch.addEventListener("input", updatePlots);
    generations.addEventListener("input", updatePlots);

    run.addEventListener("click", runExperiment)
    window.addEventListener("resize", setupCanvas);

    // setup vertical tabs buttons
    ['function', 'vrp'].forEach(function(type) {
        document.getElementById(type + 'TabButton').addEventListener("click", function(evt) {
            openTab(evt, 'controlTab', type + 'Tab', '-vert');
        });
    });

    // setup horizontal tab buttons
    ['solution', 'search', 'overall', 'best', 'duration', 'fitness'].forEach(function(type) {
        document.getElementById(type + 'TabButton').addEventListener("click", function(evt) {
            openTab(evt, 'canvasTab', type + 'Tab', '');
        });
    });

    // open default tabs
    document.getElementById("functionTabButton").click();
    document.getElementById("solutionTabButton").click();
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
    Chart.clear();
    generations.classList.add("hide");
    updatePlots();
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

function getRandomInRange(min, max) {
    return (Math.random() * (max - min) + min)
}

/** Redraw currently selected plot. */
function updateDynamicPlots(run) {
    let yaw_value = Number(yaw.value) / 100.0;
    let pitch_value = Number(pitch.value) / 100.0;
    let generation_value = Number(generations.value);
    let population_type = plotPopulation.selectedOptions[0].value;
    let heuristic_kind = "best";

    coordLabel.innerText = `Rotation: pitch=${pitch_value}, yaw=${yaw_value}`

    // TODO configure parameters from outside
    let max_gen = 2000

    const start = performance.now();
    switch (getExperimentType()) {
        case 'function': {
            // apply solution space visualization
            const selected = plotFunction.selectedOptions[0];
            switch(selected.value) {
                case 'rosenbrock':
                    Chart.rosenbrock(solutionCanvas, generation_value, pitch_value, yaw_value);
                    break;
                case 'rastrigin':
                    Chart.rastrigin(solutionCanvas, generation_value, pitch_value, yaw_value);
                    break;
                case 'himmelblau':
                    Chart.himmelblau(solutionCanvas, generation_value, pitch_value, yaw_value);
                    break;
                case 'ackley':
                    Chart.ackley(solutionCanvas, generation_value, pitch_value, yaw_value);
                    break;
                case 'matyas':
                    Chart.matyas(solutionCanvas, generation_value, pitch_value, yaw_value);
                    break;
                default:
                    break;
            }

            if (run) {
                let function_name = plotFunction.selectedOptions[0].value;
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
                Chart.run_function_experiment(function_name, population_type, x, z, max_gen);
            }

            break;
        }
        case 'vrp': {
            if (run) {
                let format_type = vrpFormat.selectedOptions[0].value;
                if (format_type === "state") {
                    max_gen = Chart.load_state(Chart.data);
                } else {
                    Chart.run_vrp_experiment(format_type, Chart.data, population_type, max_gen);
                }
            }

            Chart.vrp(solutionCanvas, generation_value, pitch_value, yaw_value);
            break;
        }
    }

    Chart.search_iteration(searchCanvas, generation_value, heuristic_kind);
    Chart.search_best_statistics(bestCanvas, generation_value, heuristic_kind);
    Chart.search_duration_statistics(durationCanvas, generation_value, heuristic_kind);
    Chart.search_overall_statistics(overallCanvas, generation_value, heuristic_kind);

    const end = performance.now();

    if (run) {
        generations.max = max_gen;
        generations.classList.remove("hide");
    }

    status.innerText = `Generation: ${generation_value} rendered in ${Math.ceil(end - start)}ms`;
}

function updateStaticPlots() {
    switch (getExperimentType()) {
        case 'function':
            Chart.fitness_func(fitnessCanvas);
            break;
        case 'vrp':
            Chart.fitness_vrp(fitnessCanvas)
            break;
        }
}

/** Runs experiment. */
function runExperiment() {
    updateDynamicPlots(true);
    updateStaticPlots(true);
}

function updatePlots() {
    updateDynamicPlots(false);
    updateStaticPlots(false);
}

function getExperimentType() {
    const buttons = document.querySelectorAll('.tablinks-vert');
    for (const button of buttons) {
        if (button.classList.contains('active')) {
            switch (button.textContent) {
                case 'Function Bench':
                    return 'function'
                case 'VRP Bench':
                    return 'vrp'
                default:
                    console.error("unknown experiment type: '" + button.textContent + "'");
                    return 'function'
            }
        }
    }

    console.error("no active tab detected");

    return 'function';
}

function openTab(evt, containerId, tabId, suffix) {
    var i, tabcontent, tablinks;

    const container = document.getElementById(containerId)

    // Get all elements with class="tabcontent" and hide them
    tabcontent = container.getElementsByClassName("tabcontent" + suffix);
    for (i = 0; i < tabcontent.length; i++) {
        tabcontent[i].style.display = "none";
    }

    // Get all elements with class="tablinks" and remove the class "active"
    tablinks = container.getElementsByClassName("tablinks" + suffix);
    for (i = 0; i < tablinks.length; i++) {
        tablinks[i].className = tablinks[i].className.replace(" active", "");
    }

    // Show the current tab, and add an "active" class to the button that opened the tab
    container.querySelector("#" + tabId).style.display = "block";
    evt.currentTarget.className += " active";
}