init();

async function init() {
    const [{Chart, default: init, get_function_domain, load_state, clear, get_experiment_summary}, {main, setup}] = await Promise.all([
        import("../pkg/heuristic_research.js"),
        import("./index.js"),
    ]);
    await init();
    setup(Chart, get_function_domain, load_state, clear, get_experiment_summary);
    main();
}
