# Features

The main focus of the project is to support solving multiple variations of VRP within their combination.

 ## Supported VRP variations

 This list tries to classify supported VRP variations using common terminology and notations.

 - **Capacitated VRP (CVRP)**: designs optimal delivery routes where each vehicle only travels
     one route, each vehicle has the same characteristics and there is only one central depot.

 - **Heterogeneous Fleet VRP (HFVRP)** aka Mixed Fleet VRP: extend CVRP problem by varying the capacities.

 - **VRP with Time Windows (VRPTW)**: assumes that deliveries to a given customer must occur in a
     certain time interval, which varies from customer to customer.

 - **VRP with Pickup and Delivery (VRPPD)**: goods need to be picked up from a certain location and
     dropped off at their destination. The pick-up and drop-off must be done by the same vehicle,
     which is why the pick-up location and drop-off location must be included in the same route.

 - **VRP with backhauls (VRPB)**: a vehicle does deliveries as well as pick-ups in one route. Some customers
     require deliveries (referred to as linehauls) and others require pick-ups (referred to as backhauls).

 - **Multi-Depot VRP (MDVRP)**: assumes that multiple depots are geographically spread among
     the customers

 - **Multi-Trip VRP (MTVRP)**: extends the VRP by adding the following constraint: routes have to be assigned
     to M vehicles in such a way that the total cost of the routes assigned to the same vehicle does not exceed
     a time horizon T (for instance the duration of a typical working day

 - **Multi-Objective VRP (MOVRP)**: this variant addresses a need of real life applications, where decision maker
     should consider not only one objective (for example, total cost), but multiple ones simultaneously, such as
     amount of tours, unassigned jobs, work balance, etc.

 - **Open VRP (OVRP)**: usually, a route beginning at a given depot must finish at this depot, but in
     this variation vehicle ends at the last served customer.

 - **VRP with Lunch Break (VRPLB)**: this problem arises when drivers must take pauses during their shift,
     for example, for lunch breaks.

 - **Periodic VRP (PVRP)**: is used when planning is made over a certain period and deliveries
     to the customer can be made in different days. In current implementation each customer
     is visited only once.

 - **Traveling Salesman Problem (TSP)**: this is a specific case of VRP when there is only one vehicle.

 In general, all these variations can be combined together in one single problem definition.


 ## Featured variations

 This list describes some of supported features, exposed by `pragmatic` format, in informal way:

 - **[multiple breaks](../examples/pragmatic/break.md)** with multiple time windows or interval time and optional
     location for vehicles.

 - **[multiple shifts](../examples/pragmatic/multi-day.md)** for vehicles: this allows to define multi-day planning
     scenario when vehicle can be used multiple times, but on different days.

 - **[multiple reloads](../examples/pragmatic/reload.md)**: this allows vehicle to return back to the depot (or any
     other place) in order to unload/load goods during single tour (see MTVRP). In some VRP variations this helps to
     significantly reduce amount of used vehicles.

 - **[multi jobs](../examples/pragmatic/multi-jobs.md)**: multi job is a job which consists of multiple sub-jobs. Multi job
     is considered as assigned only when all of sub jobs are assigned. This is useful for scenarios such as multiple
     pickups, but single delivery, or other way round.

 - **[multiple objectives](../examples/pragmatic/objectives.md)**: this extends application from scientific domain to real
     life scenarios where solver should consider multiple optimization parameters simultaneously.

 - **[vehicle profiles](../examples/pragmatic/profiles.md)**: allows to use different routing matrix profiles for different
     vehicle types, e.g. truck and car.

 - **multidimensional demand**: allows you to use multiple dimensions to set different types of capacity/demand
     simultaneously.

- **[skills](../examples/pragmatic/skills.md)**: allows to specify various skills (which is simple some tag) on vehicle
     and customer. Customer with specific skills can be visited only if these skills are present on vehicle.

- **different job types**: allows to model pickups, deliveries, replacements, services, and their combinations.

- **job priorities**: allows you to force some jobs being served before others.

 - **[relations](../examples/pragmatic/relations.md)**: allows to specify relations which locks jobs to specific vehicles
     in customizable way.

 - **limits**: allows to specify limits on vehicle such as max traveling distance or time.
