pub const RESOLUTION_ORDER: [&str; 3] = [
    "ASSESS_POLICY_PATH",
    "builtin-policies",
    "~/.epistemic/policies/",
];

pub fn resolution_order() -> &'static [&'static str; 3] {
    &RESOLUTION_ORDER
}
