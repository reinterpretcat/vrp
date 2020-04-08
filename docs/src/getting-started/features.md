# Features

The main focus of the project is to support solving multiple variations of VRP within their combination. This non-complete
list describes common VRP variations supported by the project:

 - **Capacitated VRP (CVRP)**: designs optimal delivery routes where each vehicle only travels
     one route, each vehicle has the same characteristics and there is only one central depot.

 - **Heterogeneous Fleet VRP (HFVRP)** aka Mixed Fleet VRP: extend CVRP problem by varying the capacities. Also
     [Vehicle profiles](../examples/pragmatic/basics/profiles.md) example shows how to use different routing matrix
     profiles for different vehicle types, e.g. truck and car.

 - **VRP with Time Windows (VRPTW)**: assumes that deliveries to a given customer must occur in a
     certain time interval, which varies from customer to customer.

 - **VRP with Pickup and Delivery (VRPPD)**: goods need to be picked up from a certain location and
     dropped off at their destination. The pick-up and drop-off must be done by the same vehicle,
     which is why the pick-up location and drop-off location must be included in the same route.

 - **VRP with backhauls (VRPB)**: a vehicle does deliveries as well as pick-ups in one route. Some customers
     require deliveries (referred to as linehauls) and others require pick-ups (referred to as backhauls).

 - **Multi-Depot VRP (MDVRP)**: assumes that multiple depots are geographically spread among
     the customers.

 - **Multi-Trip VRP (MTVRP)**: extends the VRP by adding the following constraint: routes have to be assigned
     to M vehicles in such a way that the total cost of the routes assigned to the same vehicle does not exceed
     a time horizon T (for instance the duration of a typical working day). See [multiple reloads](../examples/pragmatic/basics/reload.md)
     example.

 - **Multi-Objective VRP (MOVRP)**: this variant addresses a need of real life applications, where decision maker
     should consider not only one objective (for example, total cost), but multiple ones simultaneously, such as
     amount of tours, unassigned jobs, work balance, etc. See [multiple objectives](../examples/pragmatic/objectives/index.md)
     example.

 - **Open VRP (OVRP)**: usually, a route beginning at a given depot must finish at this depot, but in
     this variation vehicle ends at the last served customer.

 - **VRP with Lunch Break (VRPLB)**: this problem arises when drivers must take pauses during their shift,
     for example, for lunch breaks. The project supports different types of break: with/without location, time window,
     time period. See [Multiple breaks](../examples/pragmatic/basics/break.md) example.

 - **Periodic VRP (PVRP)**: is used when planning is made over a certain period and deliveries to the customer can be
     made in different days. In current implementation each customer is visited only once. See [multiple shifts](../examples/pragmatic/basics/multi-day.md)
     example which shows multi-day planning scenario when vehicle can be used multiple times, but on different days.

 - **Time dependent VRP (TDVRP)**: the travel time and distance between two customers or between a customer and
     the depot depends on time of day.

 - **Skill VRP (SVRP)**: associates some skill(-s) with jobs which is requirement for vehicle to have in order to serve
     the job.

 - **Traveling Salesman Problem (TSP)**: this is a specific case of VRP when there is only one vehicle.

 In general, all these variations can be combined together in one single problem definition in `pragmatic` format.
