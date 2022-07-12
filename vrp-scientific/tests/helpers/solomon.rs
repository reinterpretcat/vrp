pub type Customer = (usize, usize, usize, usize, usize, usize, usize);

pub struct SolomonBuilder {
    title: String,
    vehicle: (usize, usize),
    customers: Vec<Customer>,
}

impl Default for SolomonBuilder {
    fn default() -> Self {
        Self { title: "My Problem".to_string(), vehicle: (0, 0), customers: vec![] }
    }
}

impl SolomonBuilder {
    pub fn set_title(&mut self, title: &str) -> &mut Self {
        self.title = title.to_string();
        self
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

        data.push_str(format!("{}\n\n", self.title).as_str());

        data.push_str("VEHICLE\n NUMBER     CAPACITY\n");
        data.push_str(format!("  {}          {}\n\n", self.vehicle.0, self.vehicle.1).as_str());

        data.push_str("CUSTOMER\n");
        data.push_str("CUST NO.  XCOORD.   YCOORD.    DEMAND   READY TIME   DUE DATE   SERVICE TIME\n\n");
        self.customers.iter().for_each(|c| {
            data.push_str(format!("{} {} {} {} {} {} {} \n", c.0, c.1, c.2, c.3, c.4, c.5, c.6).as_str());
        });

        data
    }
}
