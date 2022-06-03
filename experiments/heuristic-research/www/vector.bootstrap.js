init();

async function init() {
    const [{Chart, default: init, run_function_experiment, clear}, {main, setup}] = await Promise.all([
        import("../pkg/heuristic_research.js"),
        import("./vector.index.js"),
    ]);
    await init();
    setup(Chart, run_function_experiment, clear);
    main();
}
