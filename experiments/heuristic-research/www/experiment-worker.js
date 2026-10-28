const originalLog = console.log.bind(console);
const originalError = console.error.bind(console);

console.log = (...values) => {
    const message = values.map(String).join(" ");
    reportSolverMessage(message);
    originalLog(...values);
};

console.error = (...values) => {
    const message = values.map(String).join(" ");
    if (message) self.postMessage({type: "log", message: truncate(message)});
    originalError(...values);
};

self.addEventListener("message", async (event) => {
    if (event.data?.type !== "run") return;

    const started = performance.now();
    try {
        const wasm = await loadWasm();
        const config = event.data.config;
        self.postMessage({type: "ready"});

        if (config.experimentType === "function") {
            wasm.run_function_experiment(
                config.functionName,
                config.population,
                config.initialPoint.x,
                config.initialPoint.z,
                config.generations,
            );
        } else {
            wasm.run_vrp_experiment(config.format, config.problem, config.population, config.generations);
        }

        const state = wasm.get_experiment_state();
        self.postMessage({type: "complete", state, elapsedMs: performance.now() - started});
    } catch (error) {
        self.postMessage({type: "error", error: error?.message || String(error)});
    }
});

let wasmPromise;
function loadWasm() {
    wasmPromise ??= import("../pkg/heuristic_research.js").then(async (wasm) => {
        await wasm.default();
        return wasm;
    });
    return wasmPromise;
}

function reportSolverMessage(message) {
    if (message.startsWith("EXPERIMENT_PROGRESS|")) {
        const [, generation, maxGenerations, phase, fitness] = message.split("|");
        self.postMessage({
            type: "progress",
            generation: Number(generation),
            maxGenerations: Number(maxGenerations),
            phase,
            fitness: fitness ? fitness.split(",") : [],
        });
        return;
    }

    if (message && !message.startsWith("TELEMETRY") && !message.startsWith("solution:")) {
        self.postMessage({type: "log", message: truncate(message)});
    }
}

function truncate(message) {
    const normalized = message.replace(/\s+/g, " ").trim();
    return normalized.length > 320 ? `${normalized.slice(0, 317)}…` : normalized;
}
