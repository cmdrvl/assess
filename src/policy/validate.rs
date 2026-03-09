use super::schema::PolicyFile;

pub fn default_rule_is_last(policy: &PolicyFile) -> bool {
    let Some(position) = policy.rules.iter().position(|rule| rule.default) else {
        return true;
    };

    position + 1 == policy.rules.len()
}
