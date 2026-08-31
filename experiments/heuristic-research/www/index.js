let Chart;
let api;
let experimentWorker;
let elapsedTimer;
let runStartedAt = 0;
let loadedFile;
let hasResult = false;
let activeTab = "solution";
let renderFrame;
let renderTimer;

const element = (id) => document.getElementById(id);
const canvases = {};
const chartHelp = {
    gsom: "Shows how the learned map changes over time. Read node counts, activity, learning rate, and quantization error together; open the GSOM guide below for definitions and common smoothing/compaction patterns.",
    fitness: "Best objective vector over recorded generations. Secondary objectives are rescaled only for display; optimization still uses the original lexicographic objective.",
    search: "Current Thompson selection mean for each operator. Diverse-parent labels also show parent progress × incumbent promotion, the two learned factors used for selection.",
    best: "Exact empirical incumbent-improvement rate in the selected interval. Diverse-parent labels also report parent improvements; parallel children can each beat their common pre-batch incumbent.",
    overall: "Exact calls in the selected parent bank and interval. Compare with success rate and posterior to detect starvation or monopoly.",
    duration: "Exact mean operator duration in the selected parent bank and interval. Duration is telemetry and is not part of the learned solution reward.",
};

/** Main entry point. */
export function main() {
    ["solution", "gsom", "fitness", "search", "best", "overall", "duration"].forEach((name) => {
        canvases[name] = element(`${name}Canvas`);
    });

    setupListeners();
    updateFunctionDomain();
    updateProjectionValues();
    updateChartHelp();
    updateRunAvailability();
    setStatus("Ready to run.", "success");
    element("wasmStatus").textContent = "WebAssembly ready";
    element("wasmStatus").className = "badge badge-success";
    scheduleRender();
}

/** Connects the browser controller to WebAssembly exports. */
export function setup(WasmChart, getFunctionDomain, loadState, clear, getExperimentSummary) {
    Chart = WasmChart;
    api = {getFunctionDomain, loadState, clear, getExperimentSummary};
}

function setupListeners() {
    element("benchmarkType").addEventListener("change", () => {
        const isFunction = experimentType() === "function";
        element("functionControls").classList.toggle("hide", !isFunction);
        element("vrpControls").classList.toggle("hide", isFunction);
        invalidateResult("Experiment type changed. Run the new configuration to collect data.");
        updateChartHelp();
        updateRunAvailability();
        scheduleRender();
    });
    element("plotFunction").addEventListener("change", () => {
        updateFunctionDomain();
        invalidateResult("Function changed. Run to collect a matching search trajectory.");
        scheduleRender();
    });
    element("plotPopulation").addEventListener("change", () => {
        invalidateResult("Population changed. Run to collect a matching trajectory.");
        updateChartHelp();
    });
    element("vrpFormat").addEventListener("change", () => {
        updateRunButtonLabel();
        invalidateResult("Format changed. Load or run the selected input.");
    });
    element("fileSelector").addEventListener("change", readSelectedFile);

    document.querySelectorAll('input[name="startMode"]').forEach((input) => {
        input.addEventListener("change", () => {
            element("manualPointControls").classList.toggle("hide", startMode() !== "manual");
        });
    });
    [element("pitch"), element("yaw")].forEach((input) => {
        input.addEventListener("input", () => {
            updateProjectionValues();
            if (activeTab === "solution") scheduleRender();
        });
    });
    [element("operatorState"), element("operatorWindow")].forEach((input) => {
        input.addEventListener("change", () => {
            updateChartHelp();
            scheduleRender();
        });
    });

    element("generations").addEventListener("input", (event) => setGeneration(event.target.value, false));
    element("generations").addEventListener("change", (event) => setGeneration(event.target.value, true));
    element("generationNumber").addEventListener("change", (event) => setGeneration(event.target.value, true));
    element("latestGeneration").addEventListener("click", () => setGeneration(element("generations").max, true));
    element("run").addEventListener("click", runExperiment);
    element("cancel").addEventListener("click", cancelRun);

    document.querySelectorAll(".tablinks").forEach((button) => {
        button.addEventListener("click", () => openTab(button.dataset.tab));
    });

    document.addEventListener("keydown", (event) => {
        if (element("generationControl").classList.contains("hide") || isEditable(event.target)) return;
        if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;

        const direction = event.key === "ArrowLeft" ? -1 : 1;
        const step = event.shiftKey ? 10 : 1;
        setGeneration(Number(element("generations").value) + direction * step, true);
        event.preventDefault();
    });
}

function experimentType() {
    return element("benchmarkType").value;
}

function startMode() {
    return document.querySelector('input[name="startMode"]:checked').value;
}

function updateFunctionDomain() {
    const [minX, maxX, minZ, maxZ] = api.getFunctionDomain(element("plotFunction").value);
    [[element("initX"), minX, maxX], [element("initZ"), minZ, maxZ]].forEach(([input, min, max]) => {
        input.min = min;
        input.max = max;
        const value = Number(input.value);
        if (!Number.isFinite(value) || value < min || value > max) input.value = ((min + max) / 2).toFixed(3);
    });
    element("functionDomain").textContent = `Domain: X ${formatRange(minX, maxX)} · Z ${formatRange(minZ, maxZ)}`;
}

async function readSelectedFile(event) {
    const file = event.target.files[0];
    if (!file) {
        loadedFile = undefined;
        element("fileInfo").textContent = "No file selected.";
        updateRunAvailability();
        return;
    }

    try {
        const content = await file.text();
        loadedFile = {name: file.name, size: file.size, content};
        invalidateResult("Input changed. Run it to collect a matching trajectory.");
        element("fileInfo").textContent = `${file.name} · ${formatBytes(file.size)}`;
        setStatus("Input loaded. Ready to run.", "success");
        updateRunAvailability();
    } catch (error) {
        loadedFile = undefined;
        setStatus(`Cannot read file: ${errorMessage(error)}`, "error");
        updateRunAvailability();
    }
}

function runExperiment() {
    if (experimentWorker) return;

    let config;
    try {
        config = createRunConfig();
    } catch (error) {
        setStatus(errorMessage(error), "error");
        return;
    }

    if (config.format === "state") {
        loadSavedState(config.problem);
        return;
    }

    api.clear();
    hasResult = false;
    hideResultControls();
    clearCanvases();
    element("progressPanel").classList.remove("hide");
    setRunning(true);
    runStartedAt = performance.now();
    startElapsedTimer();
    element("progressTitle").textContent = config.description;
    element("progressGeneration").textContent = `0 / ${config.generations.toLocaleString()}`;
    element("progressPhase").textContent = "initializing";
    element("progressFitness").textContent = "—";
    element("liveLog").textContent = config.initialPoint
        ? `Starting from (${config.initialPoint.x.toFixed(4)}, ${config.initialPoint.z.toFixed(4)})`
        : "Parsing problem and constructing initial solutions…";
    updateProgress(0, config.generations);

    experimentWorker = new Worker(new URL("./experiment-worker.js", import.meta.url), {type: "module"});
    experimentWorker.addEventListener("message", handleWorkerMessage);
    experimentWorker.addEventListener("error", (event) => failRun(event.message || "Experiment worker failed."));
    experimentWorker.postMessage({type: "run", config});
}

function createRunConfig() {
    const generations = parseInteger(element("maxGenerations").value, "Generation limit", 1, Number.MAX_SAFE_INTEGER);
    const population = element("plotPopulation").value;

    if (experimentType() === "vrp") {
        if (!loadedFile) throw new Error("Select a VRP problem or saved state first.");
        const format = element("vrpFormat").value;
        return {
            experimentType: "vrp",
            format,
            problem: loadedFile.content,
            population,
            generations,
            description: format === "state" ? `Load ${loadedFile.name}` : `${loadedFile.name} · ${population}`,
        };
    }

    const functionName = element("plotFunction").value;
    const [minX, maxX, minZ, maxZ] = api.getFunctionDomain(functionName);
    const initialPoint = startMode() === "manual"
        ? {
            x: parseNumberInRange(element("initX").value, "Initial X", minX, maxX),
            z: parseNumberInRange(element("initZ").value, "Initial Z", minZ, maxZ),
        }
        : createRandomPoint(minX, maxX, minZ, maxZ);

    return {
        experimentType: "function",
        functionName,
        initialPoint,
        population,
        generations,
        description: `${displayName(functionName)} · ${population}`,
    };
}

function handleWorkerMessage(event) {
    const message = event.data;
    switch (message.type) {
        case "ready":
            element("liveLog").textContent = "Solver initialized. Starting search…";
            break;
        case "progress":
            updateProgress(message.generation, message.maxGenerations);
            element("progressGeneration").textContent = `${message.generation.toLocaleString()} / ${message.maxGenerations.toLocaleString()}`;
            element("progressPhase").textContent = message.phase || "searching";
            element("progressFitness").textContent = formatFitness(message.fitness);
            break;
        case "log":
            element("liveLog").textContent = message.message;
            break;
        case "complete":
            completeRun(message);
            break;
        case "error":
            failRun(message.error);
            break;
    }
}

function completeRun(message) {
    try {
        const generation = api.loadState(message.state);
        hasResult = true;
        showResultControls(generation);
        setGeneration(generation, true);
        updateProgress(generation, generation);
        element("progressGeneration").textContent = generation.toLocaleString();
        element("progressElapsed").textContent = formatDuration(message.elapsedMs);
        element("progressTitle").textContent = "Run completed";
        element("runState").textContent = "Complete";
        element("runState").className = "badge badge-success";
        setStatus(`Completed ${generation.toLocaleString()} generations.`, "success");
        stopWorker();
        setRunning(false);
    } catch (error) {
        failRun(`The solver finished, but its state could not be loaded: ${errorMessage(error)}`);
    }
}

function loadSavedState(content) {
    try {
        api.clear();
        element("progressPanel").classList.remove("hide");
        const generation = api.loadState(content);
        hasResult = true;
        showResultControls(generation);
        setGeneration(generation, true);
        element("progressTitle").textContent = "Saved state loaded";
        element("runState").textContent = "Loaded";
        element("runState").className = "badge badge-success";
        element("progressGeneration").textContent = generation.toLocaleString();
        element("progressPhase").textContent = "recorded";
        element("progressFitness").textContent = "See charts";
        element("progressElapsed").textContent = "—";
        updateProgress(generation, generation);
        setStatus(`Loaded state through generation ${generation.toLocaleString()}.`, "success");
    } catch (error) {
        setStatus(`Cannot load state: ${errorMessage(error)}`, "error");
    }
}

function cancelRun() {
    if (!experimentWorker) return;
    stopWorker();
    setRunning(false);
    element("progressTitle").textContent = "Run cancelled";
    element("runState").textContent = "Cancelled";
    element("runState").className = "badge badge-error";
    element("liveLog").textContent = "The worker was terminated; partial state is not retained.";
    setStatus("Experiment cancelled.", "warning");
}

function failRun(error) {
    stopWorker();
    setRunning(false);
    element("progressTitle").textContent = "Run failed";
    element("runState").textContent = "Error";
    element("runState").className = "badge badge-error";
    element("liveLog").textContent = errorMessage(error);
    setStatus(errorMessage(error), "error");
}

function stopWorker() {
    if (experimentWorker) experimentWorker.terminate();
    experimentWorker = undefined;
    if (elapsedTimer) window.clearInterval(elapsedTimer);
    elapsedTimer = undefined;
}

function startElapsedTimer() {
    if (elapsedTimer) window.clearInterval(elapsedTimer);
    elapsedTimer = window.setInterval(() => {
        element("progressElapsed").textContent = formatDuration(performance.now() - runStartedAt);
    }, 250);
}

function setRunning(isRunning) {
    document.querySelectorAll(".configuration-grid input, .configuration-grid select").forEach((control) => {
        control.disabled = isRunning;
    });
    element("run").disabled = isRunning;
    element("cancel").classList.toggle("hide", !isRunning);
    if (isRunning) {
        element("runState").textContent = "Running";
        element("runState").className = "badge badge-running";
        setStatus("Search is running in a worker; the page remains interactive.", "warning");
    } else {
        updateRunAvailability();
    }
}

function updateProgress(generation, maxGenerations) {
    const ratio = maxGenerations > 0 ? Math.min(1, Math.max(0, generation / maxGenerations)) : 0;
    element("progressBar").style.width = `${(ratio * 100).toFixed(1)}%`;
}

function setGeneration(rawValue, renderImmediately) {
    const slider = element("generations");
    const value = Math.max(Number(slider.min), Math.min(Number(slider.max), Math.round(Number(rawValue) || 0)));
    slider.value = value;
    element("generationNumber").value = value;
    element("currentGen").textContent = value.toLocaleString();
    if (renderImmediately) {
        cancelScheduledRender();
        renderActivePlot();
    } else {
        scheduleRender();
    }
}

function scheduleRender() {
    cancelScheduledRender();
    renderTimer = window.setTimeout(() => {
        renderTimer = undefined;
        renderFrame = requestAnimationFrame(() => {
            renderFrame = undefined;
            renderActivePlot();
        });
    }, 80);
}

function cancelScheduledRender() {
    if (renderTimer) window.clearTimeout(renderTimer);
    if (renderFrame) cancelAnimationFrame(renderFrame);
    renderTimer = undefined;
    renderFrame = undefined;
}

function renderActivePlot() {
    const generation = Number(element("generations").value) || 0;
    const started = performance.now();

    try {
        if (activeTab === "solution") {
            const pitch = Number(element("pitch").value) / 100;
            const yaw = Number(element("yaw").value) / 100;
            if (experimentType() === "function") {
                Chart.function(canvases.solution, generation, pitch, yaw, element("plotFunction").value);
            } else if (hasResult) {
                Chart.vrp(canvases.solution, generation, pitch, yaw);
            }
        } else if (hasResult) {
            const state = element("operatorState").value;
            const window = Number(element("operatorWindow").value) || 0;
            const render = {
                gsom: () => Chart.gsom_statistics(canvases.gsom, generation),
                fitness: () => experimentType() === "function" ? Chart.fitness_func(canvases.fitness) : Chart.fitness_vrp(canvases.fitness),
                search: () => Chart.search_iteration(canvases.search, generation, state, window),
                best: () => Chart.search_best_statistics(canvases.best, generation, state, window),
                overall: () => Chart.search_overall_statistics(canvases.overall, generation, state, window),
                duration: () => Chart.search_duration_statistics(canvases.duration, generation, state, window),
            }[activeTab];
            if (render) render();
        }

        if (hasResult) updateSummary(generation);
        element("renderInfo").textContent = `Rendered ${activeTabLabel()} in ${Math.ceil(performance.now() - started)} ms.`;
    } catch (error) {
        element("renderInfo").textContent = `Cannot render ${activeTabLabel()}: ${errorMessage(error)}`;
        setStatus(`Visualization error: ${errorMessage(error)}`, "error");
    }
}

function updateSummary(generation) {
    try {
        const summary = JSON.parse(api.getExperimentSummary(generation));
        element("insights").classList.remove("hide");
        element("summaryPhase").textContent = summary.phase;
        element("summarySnapshot").textContent = `Recorded generation ${summary.snapshot_generation.toLocaleString()}`;
        element("summaryFitness").textContent = formatFitness(summary.fitness);
        element("summaryPopulation").textContent = `${summary.population_size.toLocaleString()} solutions`;
        element("summaryStorage").textContent = `${summary.snapshots} snapshots · interval ${summary.recording_interval}`;
        element("snapshotInfo").textContent = `Showing recorded generation ${summary.snapshot_generation.toLocaleString()} (retention interval ${summary.recording_interval}).`;

        if (summary.gsom_generation === null) {
            element("summaryGsom").textContent = "Not available";
            element("summaryGsomDetail").textContent = "This population has no active GSOM snapshot.";
        } else {
            element("summaryGsom").textContent = `${summary.gsom_nodes} nodes · ${summary.gsom_sink_proxies} sink proxies`;
            const stale = summary.gsom_is_stale ? `last exploration map from generation ${summary.gsom_generation}` : `${summary.gsom_active_nodes} recently hit`;
            element("summaryGsomDetail").textContent = `${summary.gsom_occupied_nodes} occupied · ${stale} · ${(summary.gsom_density * 100).toFixed(0)}% bounding-box density · MSE ${formatMetric(summary.gsom_mse)} · lr ${formatMetric(summary.gsom_learning_rate)}`;
            if (summary.gsom_is_stale) {
                element("snapshotInfo").textContent += ` GSOM is inactive in ${summary.phase}; its last exploration topology is retained from generation ${summary.gsom_generation}.`;
            }
        }
    } catch (_) {
        element("insights").classList.add("hide");
        element("snapshotInfo").textContent = "This state contains heuristic telemetry but no population snapshots.";
    }
}

function openTab(tab) {
    activeTab = tab;
    document.querySelectorAll(".tablinks").forEach((button) => button.classList.toggle("active", button.dataset.tab === tab));
    document.querySelectorAll(".tabcontent").forEach((content) => content.classList.toggle("active", content.id === `${tab}Tab`));
    updateChartHelp();
    cancelScheduledRender();
    renderActivePlot();
}

function updateChartHelp() {
    const isFunction = experimentType() === "function";
    const hasGsom = element("plotPopulation").value === "rosomaxa";

    const populationTabTitle = isFunction
        ? `Landscape${hasGsom ? " + map" : ""}`
        : `Edge footprint${hasGsom ? " + GSOM" : ""}`;
    element("solutionTabButton").textContent = populationTabTitle;
    element("solutionTabButton").classList.remove("hide");
    element("gsomTabButton").classList.toggle("hide", !hasGsom);
    if (!hasGsom && activeTab === "gsom") {
        activeTab = "fitness";
        document.querySelectorAll(".tablinks").forEach((button) => {
            button.classList.toggle("active", button.dataset.tab === activeTab);
        });
        document.querySelectorAll(".tabcontent").forEach((content) => {
            content.classList.toggle("active", content.id === `${activeTab}Tab`);
        });
    }

    const solutionHelp = isFunction
        ? hasGsom
            ? "The left side is the actual objective surface with recorded search points. The right side is GSOM feature-space topology: its grid coordinates are learned and are not coordinates from the benchmark function."
            : "The objective surface and recorded search points at the selected generation."
        : hasGsom
            ? "The compact left surface is an aggregate directed-edge footprint: peak height shows how many population members use an edge. Its axes are location indices, not geography. Rotate it with pitch and yaw. The larger right side is GSOM feature-space topology."
            : "The edge-footprint surface aggregates directed edges over the recorded population; peak height is edge frequency and its axes are location indices, not geography. Rotate it with pitch and yaw.";

    const operatorTabs = new Set(["search", "best", "overall", "duration"]);
    const isOperatorTab = operatorTabs.has(activeTab);
    const isPosterior = activeTab === "search";
    const state = element("operatorState").selectedOptions[0]?.textContent || "selected parent bank";
    const window = element("operatorWindow").selectedOptions[0]?.textContent || "selected interval";
    const help = activeTab === "solution" ? solutionHelp : chartHelp[activeTab] || "";
    const scope = isPosterior ? state.toLowerCase() : `${state.toLowerCase()}, ${window.toLowerCase()}`;
    element("chartHelp").textContent = isOperatorTab ? `${help} Showing ${scope}.` : help;
    element("operatorControls").classList.toggle("hide", !isOperatorTab);
    element("operatorGuide").classList.toggle("hide", !isOperatorTab);
    element("operatorScopeHint").textContent = isPosterior
        ? `Posterior for ${state.toLowerCase()} at the nearest recorded checkpoint. Counter window applies to success, calls, and duration.`
        : `Exact counters for ${state.toLowerCase()}; ${window.toLowerCase()}. Chart captions show the actual checkpoint interval.`;
    element("projectionTitle").textContent = isFunction ? "Objective surface view" : "Edge footprint view";
    element("projectionControls").classList.toggle("hide", activeTab !== "solution");
    element("footprintGuide").classList.toggle("hide", isFunction);
    element("gsomGuide").classList.toggle("hide", !hasGsom || (activeTab !== "solution" && activeTab !== "gsom"));
}

function invalidateResult(message) {
    if (experimentWorker) return;
    api.clear();
    hasResult = false;
    hideResultControls();
    clearCanvases();
    element("progressPanel").classList.add("hide");
    setStatus(message, "warning");
}

function showResultControls(generation) {
    const maxGeneration = Math.max(0, Number(generation) || 0);
    element("generationControl").classList.remove("hide");
    element("generations").max = maxGeneration;
    element("generationNumber").max = maxGeneration;
    element("maxGen").textContent = maxGeneration.toLocaleString();
}

function hideResultControls() {
    element("generationControl").classList.add("hide");
    element("insights").classList.add("hide");
}

function updateRunAvailability() {
    element("run").disabled = experimentType() === "vrp" && !loadedFile;
    updateRunButtonLabel();
}

function updateRunButtonLabel() {
    element("run").textContent = experimentType() === "vrp" && element("vrpFormat").value === "state"
        ? "Load state"
        : "Run experiment";
}

function updateProjectionValues() {
    element("pitchValue").textContent = (Number(element("pitch").value) / 100).toFixed(2);
    element("yawValue").textContent = (Number(element("yaw").value) / 100).toFixed(2);
}

function clearCanvases() {
    Object.values(canvases).forEach((canvas) => canvas.getContext("2d")?.clearRect(0, 0, canvas.width, canvas.height));
}

function setStatus(message, kind) {
    const status = element("status");
    status.textContent = message;
    status.className = `status-box ${kind || ""}`.trim();
}

function createRandomPoint(minX, maxX, minZ, maxZ) {
    return {x: minX + Math.random() * (maxX - minX), z: minZ + Math.random() * (maxZ - minZ)};
}

function parseInteger(value, name, min, max) {
    const parsed = Number(value);
    if (!Number.isSafeInteger(parsed) || parsed < min || parsed > max) throw new Error(`${name} must be an integer in ${formatRange(min, max)}.`);
    return parsed;
}

function parseNumberInRange(value, name, min, max) {
    const parsed = Number(value);
    if (!Number.isFinite(parsed) || parsed < min || parsed > max) throw new Error(`${name} must be in ${formatRange(min, max)}.`);
    return parsed;
}

function isEditable(target) {
    return target instanceof HTMLInputElement || target instanceof HTMLSelectElement || target instanceof HTMLTextAreaElement;
}

function displayName(value) {
    return value.split("_").map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(" ");
}

function activeTabLabel() {
    return document.querySelector(`.tablinks[data-tab="${activeTab}"]`)?.textContent || activeTab;
}

function formatFitness(values) {
    if (values === undefined || values === null || values === "") return "—";
    const items = Array.isArray(values) ? values : String(values).split(",");
    return items.length ? items.map((value) => Number.isFinite(Number(value)) ? Number(value).toLocaleString(undefined, {maximumFractionDigits: 3}) : value).join(" · ") : "—";
}

function formatMetric(value) {
    return value === null || value === undefined ? "—" : Number(value).toLocaleString(undefined, {maximumFractionDigits: 3});
}

function formatRange(min, max) {
    return `[${Number(min).toLocaleString()}, ${Number(max).toLocaleString()}]`;
}

function formatBytes(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

function formatDuration(milliseconds) {
    const seconds = Math.max(0, milliseconds) / 1000;
    if (seconds < 60) return `${seconds.toFixed(1)}s`;
    return `${Math.floor(seconds / 60)}m ${Math.floor(seconds % 60)}s`;
}

function errorMessage(error) {
    if (typeof error === "string") return error;
    return error?.message || String(error);
}
