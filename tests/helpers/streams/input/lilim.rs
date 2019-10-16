use crate::models::Problem;
use std::fs::File;
use std::io::BufReader;

pub type Customer = (usize, usize, usize, i32, usize, usize, usize, usize, usize);

pub struct LilimBuilder {
    vehicle: (usize, usize),
    customers: Vec<Customer>,
}

impl LilimBuilder {
    pub fn new() -> Self {
        Self { vehicle: (0, 0), customers: vec![] }
    }

    pub fn set_vehicle(&mut self, vehicle: (usize, usize)) -> &mut Self {
        self.vehicle = vehicle;
        self
    }

    pub fn add_customer(&mut self, customer: Customer) -> &mut Self {
        self.customers.push(customer);
        self
    }

    pub fn build(&self) -> String {
        let mut data = String::new();
        data.push_str(format!("{} {} 1\n", self.vehicle.0, self.vehicle.1).as_str());
        self.customers.iter().for_each(|c| {
            data.push_str(
                format!("{} {} {} {} {} {} {} {} {} \n", c.0, c.1, c.2, c.3, c.4, c.5, c.6, c.7, c.8).as_str(),
            );
        });

        data
    }
}
