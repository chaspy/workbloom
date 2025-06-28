use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct PortAllocation {
    pub frontend: u16,
    pub backend: u16,
    pub postgres: u16,
}

pub fn calculate_ports(branch_name: &str) -> PortAllocation {
    let mut hasher = DefaultHasher::new();
    branch_name.hash(&mut hasher);
    let hash = hasher.finish();
    
    let num = ((hash % 1000) + 1) as u16;
    
    PortAllocation {
        frontend: 5173 + num,
        backend: 8080 + num,
        postgres: 5432 + num,
    }
}