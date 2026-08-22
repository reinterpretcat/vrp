//! Specifies benchmark functions for metaheuristic testing, see `<https://en.wikipedia.org/wiki/Test_functions_for_optimization>`.

#[cfg(test)]
#[path = "../../../tests/unit/solver/vector/objectives_test.rs"]
mod objectives_test;

use rosomaxa::example::{FitnessFn, create_rosenbrock_function};
use rosomaxa::prelude::Float;
use std::f64::consts::PI;
use std::ops::Range;
use std::sync::Arc;

/// Describes the visible domain and known global minima of a two-dimensional benchmark function.
pub struct FunctionConfig {
    pub x: Range<Float>,
    pub z: Range<Float>,
    pub optima: &'static [[Float; 3]],
}

/// Returns benchmark metadata used by topology probes and visualization.
pub fn get_function_config(name: &str) -> FunctionConfig {
    const ROSENBROCK: &[[Float; 3]] = &[[1., 1., 0.]];
    const ORIGIN: &[[Float; 3]] = &[[0., 0., 0.]];
    const HIMMELBLAU: &[[Float; 3]] =
        &[[3., 2., 0.], [-2.805118, 3.131312, 0.], [-3.77931, -3.283186, 0.], [3.584428, -1.848126, 0.]];
    const BRANIN: &[[Float; 3]] =
        &[[-PI, 12.275, 0.39788735772973816], [PI, 2.275, 0.39788735772973816], [3. * PI, 2.475, 0.39788735772973816]];
    const SIX_HUMP_CAMEL: &[[Float; 3]] = &[
        [-0.08984201368301331, 0.7126564032704135, -1.031628453489877],
        [0.08984201368301331, -0.7126564032704135, -1.031628453489877],
    ];
    const GOLDSTEIN_PRICE: &[[Float; 3]] = &[[0., -1., 3.]];
    const EASOM: &[[Float; 3]] = &[[PI, PI, -1.]];
    const EGGHOLDER: &[[Float; 3]] = &[[512., 404.2319, -959.6406627106155]];
    const BUKIN6: &[[Float; 3]] = &[[-10., 1., 0.]];

    match name {
        "rosenbrock" => FunctionConfig { x: -2. ..2., z: -2. ..2., optima: ROSENBROCK },
        "rastrigin" => FunctionConfig { x: -5.12..5.12, z: -5.12..5.12, optima: ORIGIN },
        "himmelblau" => FunctionConfig { x: -5. ..5., z: -5. ..5., optima: HIMMELBLAU },
        "ackley" => FunctionConfig { x: -5. ..5., z: -5. ..5., optima: ORIGIN },
        "matyas" => FunctionConfig { x: -10. ..10., z: -10. ..10., optima: ORIGIN },
        "branin" => FunctionConfig { x: -5. ..10., z: 0. ..15., optima: BRANIN },
        "six_hump_camel" => FunctionConfig { x: -3. ..3., z: -2. ..2., optima: SIX_HUMP_CAMEL },
        "goldstein_price" => FunctionConfig { x: -2. ..2., z: -2. ..2., optima: GOLDSTEIN_PRICE },
        "easom" => FunctionConfig { x: -10. ..10., z: -10. ..10., optima: EASOM },
        "eggholder" => FunctionConfig { x: -512. ..512., z: -512. ..512., optima: EGGHOLDER },
        "bukin6" => FunctionConfig { x: -15. ..-5., z: -3. ..3., optima: BUKIN6 },
        _ => panic!("unknown objective name: `{name}`"),
    }
}

/// Returns objective function by its name.
pub fn get_fitness_fn_by_name(name: &str) -> FitnessFn {
    match name {
        "rosenbrock" => create_rosenbrock_function(),
        "rastrigin" => create_rastrigin_function(),
        "himmelblau" => create_himmelblau_function(),
        "ackley" => create_ackley_function(),
        "matyas" => create_matyas_function(),
        "branin" => create_branin_function(),
        "six_hump_camel" => create_six_hump_camel_function(),
        "goldstein_price" => create_goldstein_price_function(),
        "easom" => create_easom_function(),
        "eggholder" => create_eggholder_function(),
        "bukin6" => create_bukin6_function(),
        _ => panic!("unknown objective name: `{name}`"),
    }
}

/// Specifies [Rastrigin](https://en.wikipedia.org/wiki/Rastrigin_function) function.
/// This multimodal function is difficult to solve as it presents numerous local minima locations
/// where an optimization algorithm, with poor explorative capability, has high chances of being
/// trapped. The function’s only globally best solution 0 is found at f(i)=[0,0,…,0] within the
/// domain of [-5.12,5.12].
fn create_rastrigin_function() -> FitnessFn {
    #![allow(clippy::unnecessary_cast)]
    Arc::new(|input| {
        let a = 10.;
        input
            .iter()
            .map(|&input| input as f64)
            .fold(a * input.len() as f64, |acc, item| acc + item * item - a * (2. * std::f64::consts::PI * item).cos())
            as Float
    })
}

/// Specifies [Himmelblau](https://en.wikipedia.org/wiki/Himmelblau%27s_function) function.
/// This is a multimodal function. It is usually solved with continuous values in the domain of
/// [-5,5]. The best solution 0 can be found at four locations: f(x * )=[3.2,2.0],
/// f(x * )=[-2.805118,3.131312], f(xi)=[-3.779310,-3.283186], and f(x * )=[3.584428,-1.848126]
/// in 2 dimensional space.
fn create_himmelblau_function() -> FitnessFn {
    Arc::new(|input| {
        assert_eq!(input.len(), 2);

        let x = *input.first().unwrap();
        let y = *input.last().unwrap();

        let left = x * x + y - 11.;
        let right = x + y * y - 7.;

        left * left + right * right
    })
}

/// Specifies [Ackley](https://en.wikipedia.org/wiki/Ackley_function) function.
/// This multimodal function is one of the most commonly used test function for metaheuristic
/// algorithm evaluation. It has numerous local minima but one global optimal solution found in
/// deep narrow basin in the middle. The best solution 0 is found at f(xi)=[0,0,…,0] in domain
/// [-32,32].
fn create_ackley_function() -> FitnessFn {
    #![allow(clippy::unnecessary_cast)]
    Arc::new(|input| {
        let n = input.len() as f64;

        let square_sum = input.iter().map(|&i| i as f64).fold(0., |acc, i| acc + i * i);
        let cosine_sum = input.iter().map(|&i| i as f64).fold(0., |acc, i| acc + (2. * std::f64::consts::PI * i).cos());

        let fx = -20. * (-0.2 * (square_sum / n).sqrt()).exp();
        let fx = fx - (cosine_sum / n).exp();

        (fx + std::f64::consts::E + 20.) as Float
    })
}

/// Specifies Matyas function.
/// The best solution 0 is found at f(i)=[0,0] in domain [-10,10].
fn create_matyas_function() -> FitnessFn {
    Arc::new(|input| {
        assert_eq!(input.len(), 2);

        let x = *input.first().unwrap();
        let y = *input.last().unwrap();

        0.26 * (x * x + y * y) - 0.48 * x * y
    })
}

/// Specifies Branin's function with three equal global minima in an asymmetric domain.
fn create_branin_function() -> FitnessFn {
    Arc::new(|input| {
        let [x, y] = input else { panic!("Branin function expects two dimensions") };
        let a = 1.;
        let b = 5.1 / (4. * PI.powi(2));
        let c = 5. / PI;
        let r = 6.;
        let s = 10.;
        let t = 1. / (8. * PI);

        a * (y - b * x.powi(2) + c * x - r).powi(2) + s * (1. - t) * x.cos() + s
    })
}

/// Specifies the six-hump camel function with two global and four local minima.
fn create_six_hump_camel_function() -> FitnessFn {
    Arc::new(|input| {
        let [x, y] = input else { panic!("six-hump camel function expects two dimensions") };

        (4. - 2.1 * x.powi(2) + x.powi(4) / 3.) * x.powi(2) + x * y + (-4. + 4. * y.powi(2)) * y.powi(2)
    })
}

/// Specifies the Goldstein--Price function with several basins of very different scale.
fn create_goldstein_price_function() -> FitnessFn {
    Arc::new(|input| {
        let [x, y] = input else { panic!("Goldstein--Price function expects two dimensions") };
        let left = 1. + (x + y + 1.).powi(2) * (19. - 14. * x + 3. * x.powi(2) - 14. * y + 6. * x * y + 3. * y.powi(2));
        let right = 30.
            + (2. * x - 3. * y).powi(2) * (18. - 32. * x + 12. * x.powi(2) + 48. * y - 36. * x * y + 27. * y.powi(2));

        left * right
    })
}

/// Specifies Easom's function: an almost flat surface with one narrow optimum.
fn create_easom_function() -> FitnessFn {
    Arc::new(|input| {
        let [x, y] = input else { panic!("Easom function expects two dimensions") };

        -x.cos() * y.cos() * (-((x - PI).powi(2) + (y - PI).powi(2))).exp()
    })
}

/// Specifies the highly multimodal and asymmetric Eggholder function.
fn create_eggholder_function() -> FitnessFn {
    Arc::new(|input| {
        let [x, y] = input else { panic!("Eggholder function expects two dimensions") };

        -(y + 47.) * (x / 2. + y + 47.).abs().sqrt().sin() - x * (x - y - 47.).abs().sqrt().sin()
    })
}

/// Specifies the non-smooth Bukin N.6 function with a narrow curved valley.
fn create_bukin6_function() -> FitnessFn {
    Arc::new(|input| {
        let [x, y] = input else { panic!("Bukin N.6 function expects two dimensions") };

        100. * (y - 0.01 * x.powi(2)).abs().sqrt() + 0.01 * (x + 10.).abs()
    })
}
