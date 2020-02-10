use crate::checker::CheckerContext;
use crate::extensions::MultiDimensionalCapacity;

/// Checks that vehicle load is assigned correctly. The following rules are checked:
/// * max vehicle's capacity is not violated
/// * load change is correct
pub fn check_vehicle_load(context: &CheckerContext) -> Result<(), String> {
    context.solution.tours.iter().try_for_each(|tour| {
        let _capacity = MultiDimensionalCapacity::new(context.get_vehicle(tour.vehicle_id.as_str())?.capacity.clone());

        tour.stops.iter().try_for_each(|stop| {
            // let load = MultiDimensionalCapacity::new(stop.load.clone());

            stop.activities.iter().fold(MultiDimensionalCapacity::default(), |_, _| unimplemented!());

            Ok(())
        })
    })
}
