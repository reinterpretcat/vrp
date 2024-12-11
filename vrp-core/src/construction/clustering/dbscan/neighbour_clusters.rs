#[cfg(test)]
#[path = "../../../../tests/unit/construction/clustering/dbscan/neighbour_clusters_test.rs"]
mod neighbour_clusters_test;

use super::*;

/// Creates clusters of jobs using DBSCAN algorithm.
pub fn create_job_clusters<'a, FN, IR>(
    jobs: &[Job],
    fleet: &Fleet,
    min_points: Option<usize>,
    epsilon: Option<Float>,
    neighbour_fn: FN,
) -> GenericResult<Vec<HashSet<Job>>>
where
    FN: Fn(&Profile, &Job) -> IR + 'a,
    IR: Iterator<Item = (&'a Job, Cost)> + 'a,
{
    let min_points = min_points.unwrap_or(3).max(2);
    let epsilon = epsilon.unwrap_or_else(|| estimate_epsilon(jobs, fleet, min_points, &neighbour_fn));

    // NOTE use always first profile. It is not yet clear what would be a better way to handle multiple profiles here.
    let profile = fleet.profiles.first().ok_or_else(|| GenericError::from("cannot find any profile"))?;
    // exclude jobs without locations from clustering
    let jobs = jobs.iter().filter(|j| job_has_locations(j)).cloned().collect::<Vec<_>>();

    let neighbor_fn = move |job| {
        neighbour_fn(profile, job)
            .filter(move |(job, _)| job_has_locations(job))
            .take_while(move |(_, cost)| *cost < epsilon)
            .map(|(job, _)| job)
    };

    Ok(create_clusters(jobs.as_slice(), min_points, neighbor_fn)
        .into_iter()
        .map(|cluster| cluster.into_iter().cloned().collect::<HashSet<_>>())
        .collect())
}

/// Estimates DBSCAN epsilon parameter.
fn estimate_epsilon<'a, FN, IR>(jobs: &[Job], fleet: &Fleet, min_points: usize, neighbour_fn: &FN) -> Float
where
    FN: Fn(&Profile, &Job) -> IR + 'a,
    IR: Iterator<Item = (&'a Job, Cost)> + 'a,
{
    let costs = get_average_costs(jobs, fleet, min_points, neighbour_fn);
    let curve = costs.into_iter().enumerate().map(|(idx, cost)| Point::new(idx as Float, cost)).collect::<Vec<_>>();

    // get max curvature approximation and return it as a guess for optimal epsilon value
    get_max_curvature(curve.as_slice())
}

/// Gets average costs across all profiles.
fn get_average_costs<'a, FN, IR>(jobs: &[Job], fleet: &Fleet, min_points: usize, neighbour_fn: &FN) -> Vec<Float>
where
    FN: Fn(&Profile, &Job) -> IR + 'a,
    IR: Iterator<Item = (&'a Job, Cost)> + 'a,
{
    let mut costs = fleet.profiles.iter().fold(vec![0.; jobs.len()], |mut acc, profile| {
        jobs.iter().enumerate().for_each(|(idx, job)| {
            let (sum, count) = neighbour_fn(profile, job)
                .filter(|(j, _)| job_has_locations(j))
                .take(min_points)
                .map(|(_, cost)| cost)
                .fold((0., 1), |(sum, idx), cost| (sum + cost, idx + 1));

            acc[idx] += sum / count as Float;
        });
        acc
    });

    costs.iter_mut().for_each(|cost| *cost /= fleet.profiles.len() as Float);

    // sort all distances in ascending order
    costs.sort_unstable_by(|a, b| a.total_cmp(b));
    costs.dedup_by(|a, b| a == b);

    costs
}
