init();

async function init() {
    const [{Chart, default: init, run_vrp_experiment, clear}, {main, setup}] = await Promise.all([
        import("../pkg/heuristic_research.js"),
        import("./vrp_gsom.index.js"),
    ]);
    await init();
    setup(Chart, run_vrp_experiment, clear);
    main();
}
