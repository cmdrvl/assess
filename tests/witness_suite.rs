use assess::witness::{WitnessRecord, query::supported_modes};

#[test]
fn witness_scaffold_shapes_exist() {
    let record = WitnessRecord::scaffold(vec!["shape.json".to_owned()]);
    assert_eq!(record.tool, "assess");
    assert_eq!(supported_modes().len(), 3);
}
