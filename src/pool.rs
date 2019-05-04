/// # Live Information.
pub struct Live {
    pub name: String
}


/// # Live Poll And Connection Poll.
pub struct Pool {
    pub lives: Vec<Live>
}


impl Pool {

    /// Creatd pool.
    /// 
    /// ## example
    /// ```
    /// Pool::new();
    /// ```
    pub fn new() -> Self {
        Pool { lives: Vec::new() }
    }
}