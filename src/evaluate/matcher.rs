use crate::policy::PolicyFile;

pub fn has_default_rule(policy: &PolicyFile) -> bool {
    policy.rules.iter().any(|rule| rule.default)
}
