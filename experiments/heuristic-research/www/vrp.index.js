const fileBrowser = document.getElementById("file-browser");

/** Main entry point */
export function main() {
    setupUI();
}

/** This function is used in `vrp.bootstrap.js` to setup imports. */
export function setup(run_vrp_experiment, clear) {
    // TODO
}

function setupUI() {
    fileBrowser.addEventListener("change", openFile);
}

function openFile(event) {
    let input = event.target;
    let reader = new FileReader();

    reader.onload = function(){
        let content = reader.result;
        console.log(content.substring(0, 300));
    };
    reader.readAsText(input.files[0]);
}