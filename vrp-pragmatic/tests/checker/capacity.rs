use crate::checker::CheckerContext;
use crate::extensions::MultiDimensionalCapacity;

/// Checks that vehicle load is assigned correctly. The following rules are checked:
/// * max vehicle's capacity is not violated
/// * load change is correct
pub fn check_vehicle_load(context: &CheckerContext) -> Result<(), String> {
    context.solution.tours.iter().try_for_each(|tour| {
        let capacity = MultiDimensionalCapacity::new(context.get_vehicle(tour.vehicle_id.as_str())?.capacity.clone());

        (1..).zip(tour.stops.windows(2)).try_for_each(|(idx, leg)| {
            let (from, to) = match leg {
                [from, to] => (from, to),
                _ => return Err("Unexpected leg configuration".to_owned()),
            };

            let change = to.activities.iter().try_fold::<_, _, Result<_, String>>(
                MultiDimensionalCapacity::default(),
                |acc, activity| {
                    let activity_type = context.get_activity_type(tour, to, activity)?;
                    let demand = context.visit_job(
                        activity,
                        &activity_type,
                        |job| MultiDimensionalCapacity::new(job.demand.clone()),
                        |_, place| MultiDimensionalCapacity::new(place.demand.clone()),
                        || MultiDimensionalCapacity::default(),
                    )?;

                    Ok(acc + demand)
                },
            )?;

            let old_load = MultiDimensionalCapacity::new(from.load.clone());
            let new_load = MultiDimensionalCapacity::new(to.load.clone());

            if old_load > capacity || new_load > capacity {
                return Err(format!("Load exceeds capacity in tour '{}'", tour.vehicle_id));
            }

            if new_load + change == old_load {
                Ok(())
            } else {
                Err(format!("Load mismatch at stop {} in tour '{}'", idx, tour.vehicle_id))
            }
        })
    })
}
