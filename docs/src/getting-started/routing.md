# Acquiring routing info

Once the problem is represented in `pragmatic` format, it's time to get matrix routing data.


## Routing locations

The solver does not provide routing functionality, that's why you need to get it manually using unique locations from the
problem definition. The process of getting locations for matrix routing and its usage is described
[here](../concepts/pragmatic/routing/format.md).


## Routing matrix approximation

For quick prototyping, `pragmatic` format supports distance approximation using [haversine formula](https://en.wikipedia.org/wiki/Haversine_formula)
within fixed speed for durations. This helps you quickly check how solver works on specific problem variant without
need to acquire routing matrix.

The speed is `10m/s` by default and can be tweaked by setting optional `speed` property in a each profile separately.

To use this feature, simply omit `-m` parameter.
