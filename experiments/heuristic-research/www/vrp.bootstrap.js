init();

async function init() {
    const [{default: init, run_vrp_experiment, get_data_graphs, clear}, {main, setup}] = await Promise.all([
        import("../pkg/heuristic_research.js"),
        import("./vrp.index.js"),
        // https://github.com/upphiminn/d3.ForceBundle
        //import("./d3-ForceEdgeBundling.js"),
    ]);
    await init();
    setup(run_vrp_experiment, get_data_graphs, clear);
    main();
}
