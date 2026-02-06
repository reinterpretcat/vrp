class Chart {}

const solutionCanvas = document.getElementById("solutionCanvas");
const searchCanvas = document.getElementById("searchCanvas");
const overallCanvas = document.getElementById("overallCanvas");
const bestCanvas = document.getElementById("bestCanvas");
const durationCanvas = document.getElementById("durationCanvas");
const fitnessCanvas = document.getElementById("fitnessCanvas");

const benchmarkType = document.getElementById("benchmarkType");
const functionControls = document.getElementById("functionControls");
const vrpControls = document.getElementById("vrpControls");
const fileSelector = document.getElementById("fileSelector");
const plotPopulation = document.getElementById("plotPopulation");
const plotFunction = document.getElementById("plotFunction");
const vrpFormat = document.getElementById("vrpFormat");
const pitch = document.getElementById("pitch");
const yaw = document.getElementById("yaw");
const pitchValue = document.getElementById("pitchValue");
const yawValue = document.getElementById("yawValue");
const status = document.getElementById("status");
const run = document.getElementById("run");
const generations = document.getElementById("generations");
const generationControl = document.getElementById("generationControl");
const currentGen = document.getElementById("currentGen");
const maxGen = document.getElementById("maxGen");
const maxGenerations = document.getElementById("maxGenerations");
const autoInitPoint = document.getElementById("autoInitPoint");
const manualPointControls = document.getElementById("manualPointControls");
const initX = document.getElementById("initX");
const initZ = document.getElementById("initZ");

/** Main entry point */
export function main() {
    setupListeners();
    resizeAllCanvases();
    updateDynamicPlots();
    updateStaticPlots();
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
function setupListeners() {
    status.innerText = "✓ WebAssembly loaded!";
    status.classList.add('success');
    
    benchmarkType.addEventListener("change", switchBenchmarkType);
    fileSelector.addEventListener("change", openFile);
    plotFunction.addEventListener("change", changePlot);
    plotPopulation.addEventListener("change", changePlot);
    autoInitPoint.addEventListener("change", toggleInitPointMode);

    yaw.addEventListener("change", updatePlots);
    pitch.addEventListener("change", updatePlots);
    generations.addEventListener("change", updatePlots);

    yaw.addEventListener("input", (e) => {
        updateSliderValue(e.target, yawValue);
        updatePlots();
    });
    pitch.addEventListener("input", (e) => {
        updateSliderValue(e.target, pitchValue);
        updatePlots();
    });
    generations.addEventListener("input", (e) => {
        currentGen.innerText = e.target.value;
        updatePlots();
    });

    run.addEventListener("click", runExperiment);
    window.addEventListener("resize", () => {
        resizeAllCanvases();
        updatePlots();
    });

    // setup horizontal tab buttons
    ['solution', 'search', 'overall', 'best', 'duration', 'fitness'].forEach(function(type) {
        document.getElementById(type + 'TabButton').addEventListener("click", function(evt) {
            openTab(evt, 'canvasTab', type + 'Tab', '');
        });
    });

    // open default tabs
    document.getElementById("solutionTabButton").click();

    // allow to control generation range using left-right arrows
    document.addEventListener('keydown', function(event) {
        switch (event.key) {
            case "ArrowLeft":
                generations.value = Math.max(parseInt(generations.value) - 1, parseInt(generations.min));
                currentGen.innerText = generations.value;
                updatePlots();
                break;
            case "ArrowRight":
                generations.value = Math.min(parseInt(generations.value) + 1, parseInt(generations.max));
                currentGen.innerText = generations.value;
                updatePlots();
                break;
        }
    });
    
    // Initialize slider values
    updateSliderValue(pitch, pitchValue);
    updateSliderValue(yaw, yawValue);
    
    // Set initial ranges for function
    updateInitPointRanges();
}

/** Setup canvas to properly handle high DPI and redraw current plot. */
function setupCanvas(canvas) {
    if (!canvas || !canvas.style) {
        return;
    }

    const container = canvas.parentNode;
    const containerWidth = container.offsetWidth - 20; // subtract padding
    const originalAspectRatio = canvas.width / canvas.height;
    
    // Set display size (CSS pixels)
    const displayWidth = Math.min(containerWidth, 950);
    const displayHeight = displayWidth / originalAspectRatio;
    
    canvas.style.width = displayWidth + "px";
    canvas.style.height = displayHeight + "px";
}

/** Resize all canvases */
function resizeAllCanvases() {
    [solutionCanvas, searchCanvas, overallCanvas, bestCanvas, durationCanvas, fitnessCanvas].forEach(canvas => {
        setupCanvas(canvas);
    });
}

/** Update slider display value */
function updateSliderValue(slider, display) {
    const value = (Number(slider.value) / 100.0).toFixed(2);
    display.innerText = value;
}

/** Switch between Function and VRP benchmark types */
function switchBenchmarkType() {
    const type = benchmarkType.value;
    if (type === 'function') {
        functionControls.classList.remove('hide');
        vrpControls.classList.add('hide');
    } else {
        functionControls.classList.add('hide');
        vrpControls.classList.remove('hide');
        
        // Show message if no file loaded
        if (!Chart.data) {
            status.innerText = 'Please load a VRP problem file';
            status.classList.remove('success', 'loading');
        }
    }
    changePlot();
}

/** Toggle between automatic and manual initial point selection */
function toggleInitPointMode() {
    if (autoInitPoint.checked) {
        manualPointControls.classList.add('hide');
    } else {
        manualPointControls.classList.remove('hide');
    }
}

/** Update initial point input ranges based on selected function */
function updateInitPointRanges() {
    const functionName = plotFunction.value;
    let min, max;
    
    switch(functionName) {
        case 'rosenbrock':
            min = -2.0; max = 2.0;
            break;
        case 'rastrigin':
            min = -5.12; max = 5.12;
            break;
        case 'himmelblau':
            min = -5.0; max = 5.0;
            break;
        case 'ackley':
            min = -5.0; max = 5.0;
            break;
        case 'matyas':
            min = -10.0; max = 10.0;
            break;
        default:
            min = -5.0; max = 5.0;
    }
    
    initX.min = min;
    initX.max = max;
    initZ.min = min;
    initZ.max = max;
    
    // Reset to center if out of range
    if (parseFloat(initX.value) < min || parseFloat(initX.value) > max) {
        initX.value = 0;
    }
    if (parseFloat(initZ.value) < min || parseFloat(initZ.value) > max) {
        initZ.value = 0;
    }
}

/** Changes plot **/
function changePlot() {
    Chart.clear();
    generationControl.classList.add("hide");
    currentGen.innerText = "0";
    updateInitPointRanges();
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
        
        // Update status to show file loaded
        status.innerText = `✓ File loaded: ${input.files[0].name}`;
        status.classList.add('success');
        status.classList.remove('loading');
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

    // Get max generations from user input
    let max_gen = parseInt(maxGenerations.value);

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
                
                // Use manual point if checkbox is unchecked, otherwise random
                if (!autoInitPoint.checked) {
                    x = parseFloat(initX.value);
                    z = parseFloat(initZ.value);
                } else {
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
                }

                console.log(`init point is: (${x}, ${z})`)
                Chart.run_function_experiment(function_name, population_type, x, z, max_gen);
            }

            break;
        }
        case 'vrp': {
            if (run) {
                let format_type = vrpFormat.selectedOptions[0].value;
                
                // Check if data has been loaded
                if (!Chart.data) {
                    status.innerText = '⚠ Please load a VRP file first';
                    status.classList.remove('success');
                    status.classList.add('loading');
                    return;
                }
                
                if (format_type === "state") {
                    max_gen = Chart.load_state(Chart.data);
                } else {
                    Chart.run_vrp_experiment(format_type, Chart.data, population_type, max_gen);
                }
            }

            // Only render if data has been loaded
            if (Chart.data) {
                Chart.vrp(solutionCanvas, generation_value, pitch_value, yaw_value);
            }
            break;
        }
    }

    // Only render statistics if there's data to display
    if (Chart.data || getExperimentType() === 'function') {
        Chart.search_iteration(searchCanvas, generation_value, heuristic_kind);
        Chart.search_best_statistics(bestCanvas, generation_value, heuristic_kind);
        Chart.search_duration_statistics(durationCanvas, generation_value, heuristic_kind);
        Chart.search_overall_statistics(overallCanvas, generation_value, heuristic_kind);
    }

    const end = performance.now();

    if (run) {
        generations.max = max_gen;
        maxGen.innerText = max_gen;
        currentGen.innerText = "0";
        generations.value = "0";
        generationControl.classList.remove("hide");
    }

    status.innerText = `Generation: ${generation_value} | Rendered in ${Math.ceil(end - start)}ms`;
    status.classList.remove('loading');
    status.classList.add('success');
}

function updateStaticPlots() {
    switch (getExperimentType()) {
        case 'function':
            Chart.fitness_func(fitnessCanvas);
            break;
        case 'vrp':
            // Only render if data has been loaded
            if (Chart.data) {
                Chart.fitness_vrp(fitnessCanvas);
            }
            break;
        }
}

/** Runs experiment. */
function runExperiment() {
    run.disabled = true;
    run.innerHTML = '⏳ Running...';
    status.innerText = 'Running experiment...';
    status.classList.add('loading');
    status.classList.remove('success');
    
    // Use setTimeout to allow UI to update
    setTimeout(() => {
        updateDynamicPlots(true);
        updateStaticPlots(true);
        run.disabled = false;
        run.innerHTML = '▶ Run Experiment';
    }, 50);
}

function updatePlots() {
    updateDynamicPlots(false);
    updateStaticPlots(false);
}

function getExperimentType() {
    return benchmarkType.value;
}

function openTab(evt, containerId, tabId, suffix) {
    let container = document.getElementById(containerId)

    // Get all elements with class="tabcontent" and hide them
    let tabcontent = container.getElementsByClassName("tabcontent" + suffix);
    for (let i = 0; i < tabcontent.length; i++) {
        tabcontent[i].style.display = "none";
    }

    // Get all elements with class="tablinks" and remove the class "active"
    let tablinks = container.getElementsByClassName("tablinks" + suffix);
    for (let i = 0; i < tablinks.length; i++) {
        tablinks[i].className = tablinks[i].className.replace(" active", "");
    }

    // Show the current tab, and add an "active" class to the button that opened the tab
    container.querySelector("#" + tabId).style.display = "block";
    evt.currentTarget.className += " active";
}