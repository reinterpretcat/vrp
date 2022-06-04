class Chart {}

const fileBrowser = document.getElementById("file-browser");
const plotPopulation = document.getElementById("plot-population");
const run = document.getElementById("run");
const generations = document.getElementById("generations");
const vrpFormat = document.getElementById("vrp-format");

/** Main entry point */
export function main() {
    setupUI();
}

/** This function is used in `vrp.bootstrap.js` to setup imports. */
export function setup(run_vrp_experiment, clear) {
    Chart.run_experiment = run_vrp_experiment;
    Chart.clear = clear;
}

function setupUI() {
    fileBrowser.addEventListener("change", openFile);
    run.addEventListener("click", runExperiment)
}

function openFile(event) {
    let input = event.target;
    let reader = new FileReader();

    reader.onload = function(){
        let content = reader.result;
        console.log(content.substring(0, 300));

        Chart.problem = content;

        run.classList.remove("hide");
        generations.classList.remove("hide");
    };
    reader.readAsText(input.files[0]);
}

/** Runs experiment. */
function runExperiment() {
    // TODO configure parameters from outside
    let max_gen = 2000
    let population_type = plotPopulation.selectedOptions[0].value;
    let format_type = vrpFormat.selectedOptions[0].value;

    Chart.run_experiment(format_type, Chart.problem, population_type, max_gen)
}