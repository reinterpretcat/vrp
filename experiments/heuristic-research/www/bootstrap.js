init();

async function init() {
    const [{Chart, default: init, run_experiment}, {main, setup}] = await Promise.all([
        import("../pkg/heuristic_research.js"),
        import("./index.js"),
    ]);
    await init();
    setup(Chart, run_experiment);
    main();
}
