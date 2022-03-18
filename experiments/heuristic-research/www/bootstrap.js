init();

async function init() {
    const [{Chart, default: init, run_experiment, get_generation}, {main, setup}] = await Promise.all([
        import("../pkg/heuristic_research.js"),
        import("./index.js"),
    ]);
    await init();
    setup(Chart, run_experiment, get_generation);
    main();
}
