pub mod human;
pub mod json;

use crate::evaluate::Decision;

pub fn render_stub(decision: &Decision, json_output: bool) -> String {
    if json_output {
        json::render(decision)
    } else {
        human::render(decision)
    }
}
