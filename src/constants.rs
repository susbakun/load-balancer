pub const AVAILABLE_ALGOS: [&str; 4] = [
    "round_robin",
    "weighted_round_robin",
    "least_connections",
    "PEWMA",
];

// used for updating the latency in
// PEWMA algorithm
pub const ALPHA: f32 = 0.2;
