init();

async function init() {
    const [{Chart, default: init, run_function_experiment, run_vrp_experiment, load_state, clear}, {main, setup}] = await Promise.all([
        import("../pkg/heuristic_research.js"),
        import("./index.js"),
    ]);
    await init();
    setup(Chart, run_function_experiment, run_vrp_experiment, load_state, clear);
    main();
}
