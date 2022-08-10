# Shared resources

A `fleet.resources` specifies an optional section which goal is to control distribution of limited shared resource
between different vehicles.


## Reload resource

An idea of reload resource is to put limit on amount of deliveries in total loaded to the multiple vehicles on specific
reload place. A good example is some warehouse which can be visited by multiple vehicles in the middle of their tours,
but it has only limited amount of deliveries.

The reload resource definition has the following properties:

- `type` (required): should be set to `reload`
- `id` (required): an unique resource id. Put this id in vehicle reload's `resourceId` property to trigger shared resource behavior
- `capacity` (required): total amount of resource. It has the same type as vehicle's `capacity` property.

An example of a reload resource definition:

```json
{{#include ../../../../../examples/data/pragmatic/basics/reload.resource.problem.json:174:180}}
```

An example of a vehicle reload with a reference to the resource definition:

```json
{{#include ../../../../../examples/data/pragmatic/basics/reload.resource.problem.json:152:161}}
```

The full example can be found [here](../../../examples/pragmatic/basics/reload.md#Shared-reload-resource).