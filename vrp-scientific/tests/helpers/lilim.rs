/// id  x   y  demand early late service pickup delivery
pub type LilimCustomer = (i32, i32, i32, i32, i32, i32, i32, i32, i32);

#[derive(Default)]
pub struct LilimBuilder {
    vehicle: (usize, usize),
    customers: Vec<LilimCustomer>,
}


impl LilimBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_vehicle(&mut self, vehicle: (usize, usize)) -> &mut Self {
        self.vehicle = vehicle;
        self
    }

    pub fn add_customer(&mut self, customer: LilimCustomer) -> &mut Self {
        self.customers.push(customer);
        self
    }

    pub fn build(&self) -> String {
        let mut data = String::new();

        data.push_str(&format!("{} {} 0\n", self.vehicle.0, self.vehicle.1));

        for c in &self.customers {
            data.push_str(&format!("{} {} {} {} {} {} {} {} {}\n", c.0, c.1, c.2, c.3, c.4, c.5, c.6, c.7, c.8));
        }

        data
    }
}
