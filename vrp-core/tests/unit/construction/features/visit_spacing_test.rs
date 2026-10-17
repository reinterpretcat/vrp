use super::*;
use crate::helpers::models::domain::{TestGoalContextBuilder, test_random};
use crate::helpers::models::problem::{FleetBuilder, TestSingleBuilder, TestVehicleBuilder, test_driver};
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder};
use crate::models::problem::{Actor, Fleet, Single, VehicleDetail, VehiclePlace};
use crate::models::solution::Registry;
use std::sync::Arc;

const VIOLATION_CODE: ViolationCode = ViolationCode(1);
const HOUR: Float = 3_600.;

fn create_test_feature(total_jobs: usize) -> Feature {
    create_visit_spacing_feature("visit_spacing", total_jobs, VIOLATION_CODE).unwrap()
}

/// One shift per entry: its day and the hour it starts at, so two shifts can start at different
/// times of their day.
fn create_test_fleet(shifts: &[(i64, Float)]) -> Fleet {
    let details = shifts
        .iter()
        .map(|&(day, hour)| {
            let start = day as Float * SECONDS_PER_DAY + hour * HOUR;
            VehicleDetail {
                start: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: Some(start), latest: None } }),
                end: Some(VehiclePlace {
                    location: 0,
                    time: TimeInterval { earliest: None, latest: Some(start + 10. * HOUR) },
                }),
            }
        })
        .collect();

    FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(TestVehicleBuilder::default().id("v1").details(details).build())
        .build()
}

fn actor_on(fleet: &Fleet, day: i64) -> Arc<Actor> {
    fleet
        .actors
        .iter()
        .find(|actor| ((actor.detail.time.start / SECONDS_PER_DAY).floor() as i64) == day)
        .cloned()
        .unwrap()
}

fn spacing(group: &str, order: usize, anchor: bool) -> VisitSpacing {
    VisitSpacing { group: group.to_string(), order, interval_days: 7, tolerance_days: 1, anchor }
}

fn visit(spacing: Option<VisitSpacing>) -> Arc<Single> {
    let mut builder = TestSingleBuilder::default();
    if let Some(spacing) = spacing {
        builder.dimens_mut().set_visit_spacing(spacing);
    }
    builder.build_shared()
}

fn route_with(actor: Arc<Actor>, visits: Vec<Arc<Single>>) -> RouteContext {
    let mut route_ctx = RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_actor(actor)
                .add_activities(
                    visits.into_iter().map(|single| ActivityBuilder::with_location(1).job(Some(single)).build()),
                )
                .build(),
        )
        .build();
    VisitSpacingState {}.accept_route_state(&mut route_ctx);

    route_ctx
}

/// `placed` holds the routes by day with the visits already on them; the first route is the one
/// the visit is offered to. `total_jobs` counts every job of the problem.
fn solution_ctx(fleet: &Fleet, total_jobs: usize, placed: Vec<(i64, Vec<Arc<Single>>)>) -> SolutionContext {
    let already_assigned = placed.iter().map(|(_, visits)| visits.len()).sum::<usize>();

    SolutionContext {
        required: (0..(total_jobs - already_assigned)).map(|_| Job::Single(visit(None))).collect(),
        ignored: vec![],
        unassigned: Default::default(),
        locked: Default::default(),
        routes: placed.into_iter().map(|(day, visits)| route_with(actor_on(fleet, day), visits)).collect(),
        registry: RegistryContext::new(&TestGoalContextBuilder::default().build(), Registry::new(fleet, test_random())),
        state: Default::default(),
    }
}

fn evaluate(ctx: &SolutionContext, total_jobs: usize, job: VisitSpacing) -> Option<ConstraintViolation> {
    let constraint = create_test_feature(total_jobs).constraint.unwrap();

    constraint.evaluate(&MoveContext::route(ctx, ctx.routes.first().unwrap(), &Job::Single(visit(Some(job)))))
}

fn refused() -> Option<ConstraintViolation> {
    Some(ConstraintViolation { code: VIOLATION_CODE, stopped: true })
}

parameterized_test! {can_keep_a_visit_its_interval_from_the_one_before, (day, expected), {
    can_keep_a_visit_its_interval_from_the_one_before_impl(day, expected);
}}

can_keep_a_visit_its_interval_from_the_one_before! {
    case01_a_day_too_early: (5, refused()),
    case02_one_day_early: (6, None),
    case03_on_the_interval: (7, None),
    case04_one_day_late: (8, None),
    case05_a_day_too_late: (9, refused()),
    case06_on_the_same_day: (0, refused()),
}

fn can_keep_a_visit_its_interval_from_the_one_before_impl(day: i64, expected: Option<ConstraintViolation>) {
    let fleet = create_test_fleet(&[(0, 7.), (day.max(1), 7.)]);
    let total = 2;
    let routes = if day == 0 {
        vec![(0, vec![visit(Some(spacing("sub-1", 1, false)))])]
    } else {
        vec![(day, vec![]), (0, vec![visit(Some(spacing("sub-1", 1, false)))])]
    };
    let ctx = solution_ctx(&fleet, total, routes);

    assert_eq!(evaluate(&ctx, total, spacing("sub-1", 2, false)), expected);
}

parameterized_test! {can_keep_a_visit_its_interval_from_the_one_after, (day, expected), {
    can_keep_a_visit_its_interval_from_the_one_after_impl(day, expected);
}}

can_keep_a_visit_its_interval_from_the_one_after! {
    case01_a_day_too_early: (3, refused()),
    case02_on_the_interval: (7, None),
    case03_a_day_too_late: (11, refused()),
}

fn can_keep_a_visit_its_interval_from_the_one_after_impl(day: i64, expected: Option<ConstraintViolation>) {
    let fleet = create_test_fleet(&[(day, 7.), (14, 7.)]);
    let total = 2;
    let ctx = solution_ctx(&fleet, total, vec![(day, vec![]), (14, vec![visit(Some(spacing("sub-1", 2, false)))])]);

    assert_eq!(evaluate(&ctx, total, spacing("sub-1", 1, false)), expected);
}

// The second visit of the rhythm is not placed: the first and the third keep two intervals apart,
// with the tolerance of both steps, so taking out a visit between two others never breaks them.
parameterized_test! {can_space_over_a_visit_not_placed, (day, expected), {
    can_space_over_a_visit_not_placed_impl(day, expected);
}}

can_space_over_a_visit_not_placed! {
    case01_too_early: (11, refused()),
    case02_two_days_early: (12, None),
    case03_two_days_late: (16, None),
    case04_too_late: (17, refused()),
}

fn can_space_over_a_visit_not_placed_impl(day: i64, expected: Option<ConstraintViolation>) {
    let fleet = create_test_fleet(&[(0, 7.), (day, 7.)]);
    let total = 3;
    let ctx = solution_ctx(&fleet, total, vec![(day, vec![]), (0, vec![visit(Some(spacing("sub-1", 1, false)))])]);

    assert_eq!(evaluate(&ctx, total, spacing("sub-1", 3, false)), expected);
}

/// Only the nearest visit on each side is measured: the third keeps to the second, which already
/// kept to the first.
#[test]
fn can_measure_against_the_nearest_visit_only() {
    let fleet = create_test_fleet(&[(0, 7.), (8, 7.), (15, 7.)]);
    let total = 3;
    let ctx = solution_ctx(
        &fleet,
        total,
        vec![
            (15, vec![]),
            (0, vec![visit(Some(spacing("sub-1", 1, false)))]),
            (8, vec![visit(Some(spacing("sub-1", 2, false)))]),
        ],
    );

    assert_eq!(evaluate(&ctx, total, spacing("sub-1", 3, false)), None);
}

#[test]
fn can_fit_a_visit_between_two_only_where_both_agree() {
    let fleet = create_test_fleet(&[(0, 7.), (6, 7.), (8, 7.), (14, 7.)]);
    let total = 3;
    let placed = |day: i64| {
        vec![
            (day, vec![]),
            (0, vec![visit(Some(spacing("sub-1", 1, false)))]),
            (14, vec![visit(Some(spacing("sub-1", 3, false)))]),
        ]
    };

    assert_eq!(evaluate(&solution_ctx(&fleet, total, placed(6)), total, spacing("sub-1", 2, false)), None);
    assert_eq!(evaluate(&solution_ctx(&fleet, total, placed(8)), total, spacing("sub-1", 2, false)), None);

    let fleet = create_test_fleet(&[(0, 7.), (6, 7.), (12, 7.)]);
    let ctx = solution_ctx(
        &fleet,
        total,
        vec![
            (6, vec![]),
            (0, vec![visit(Some(spacing("sub-1", 1, false)))]),
            (12, vec![visit(Some(spacing("sub-1", 3, false)))]),
        ],
    );

    // six days from the first, but also six days before the third, which wants seven give or take one:
    // fine; one day earlier breaks the first, one later the third.
    assert_eq!(evaluate(&ctx, total, spacing("sub-1", 2, false)), None);
}

#[test]
fn can_ignore_visits_of_another_rhythm() {
    let fleet = create_test_fleet(&[(0, 7.), (2, 7.)]);
    let total = 2;
    let ctx = solution_ctx(&fleet, total, vec![(2, vec![]), (0, vec![visit(Some(spacing("sub-2", 1, false)))])]);

    assert_eq!(evaluate(&ctx, total, spacing("sub-1", 2, false)), None);
}

/// A locked visit's day is a fact: it is never refused, and the others still keep their distance to it.
#[test]
fn can_place_an_anchor_anywhere_and_keep_the_others_to_it() {
    let fleet = create_test_fleet(&[(0, 7.), (2, 7.)]);
    let total = 2;
    let ctx = solution_ctx(&fleet, total, vec![(2, vec![]), (0, vec![visit(Some(spacing("sub-1", 1, true)))])]);

    assert_eq!(evaluate(&ctx, total, spacing("sub-1", 2, true)), None);
    assert_eq!(evaluate(&ctx, total, spacing("sub-1", 2, false)), refused());
}

#[test]
fn can_leave_a_visit_without_spacing_alone() {
    let fleet = create_test_fleet(&[(0, 7.), (2, 7.)]);
    let total = 2;
    let ctx = solution_ctx(&fleet, total, vec![(2, vec![]), (0, vec![visit(Some(spacing("sub-1", 1, false)))])]);
    let constraint = create_test_feature(total).constraint.unwrap();

    assert_eq!(
        constraint.evaluate(&MoveContext::route(&ctx, ctx.routes.first().unwrap(), &Job::Single(visit(None)))),
        None
    );
}

/// A shift that starts in the evening is still that day's shift: days are counted between the
/// shifts' starts and rounded.
#[test]
fn can_count_days_between_shifts_starting_at_different_hours() {
    let fleet = create_test_fleet(&[(0, 18.), (7, 6.)]);
    let total = 2;
    let ctx = solution_ctx(&fleet, total, vec![(7, vec![]), (0, vec![visit(Some(spacing("sub-1", 1, false)))])]);

    assert_eq!(evaluate(&ctx, total, spacing("sub-1", 2, false)), None);
}

#[test]
fn can_refuse_a_spaced_visit_in_a_sub_problem() {
    let fleet = create_test_fleet(&[(0, 7.), (7, 7.)]);
    let ctx = solution_ctx(&fleet, 2, vec![(7, vec![]), (0, vec![visit(Some(spacing("sub-1", 1, false)))])]);

    assert_eq!(evaluate(&ctx, 5, spacing("sub-1", 2, false)), refused());
}

#[test]
fn can_rebuild_a_route_state_with_its_visits() {
    let fleet = create_test_fleet(&[(0, 7.)]);
    let route_ctx = route_with(
        actor_on(&fleet, 0),
        vec![visit(Some(spacing("sub-1", 2, false))), visit(Some(spacing("sub-2", 1, false))), visit(None)],
    );
    let visits = route_ctx.state().get_current_spaced_visits().unwrap();

    assert_eq!(visits.get("sub-1"), Some(&vec![2]));
    assert_eq!(visits.get("sub-2"), Some(&vec![1]));
    assert_eq!(visits.len(), 2);
}

#[test]
fn can_merge_only_visits_without_spacing() {
    let constraint = create_test_feature(2).constraint.unwrap();
    let plain = || Job::Single(visit(None));
    let spaced = || Job::Single(visit(Some(spacing("sub-1", 1, false))));

    assert!(constraint.merge(plain(), plain()).is_ok());
    assert_eq!(constraint.merge(spaced(), plain()).err(), Some(VIOLATION_CODE));
    assert_eq!(constraint.merge(plain(), spaced()).err(), Some(VIOLATION_CODE));
}
