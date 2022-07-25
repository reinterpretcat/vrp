init();

async function init() {
    const [{default: init, run_vrp_edge_bundling_experiment, get_bundled_edges, clear}, {main, setup}] = await Promise.all([
        import("../pkg/heuristic_research.js"),
        import("./vrp_fdeb.index.js"),
    ]);
    await init();
    setup(run_vrp_edge_bundling_experiment, get_bundled_edges, clear);
    main();
}
