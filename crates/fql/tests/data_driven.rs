use assert_cmd::Command;
use serde::Deserialize;

#[derive(Deserialize)]
struct TestCase {
    query: String,
    expected_contains: Vec<String>,
    expected_not_contains: Vec<String>,
}

#[test]
fn run_data_driven_tests() {
    let file = std::fs::read_to_string("tests/queries.json").expect("Failed to read queries.json");
    let test_cases: Vec<TestCase> = serde_json::from_str(&file).expect("Failed to parse JSON");

    for (i, tc) in test_cases.iter().enumerate() {
        println!("Running test case {}: {}", i, tc.query);
        
        let mut cmd = Command::cargo_bin("fql").unwrap();
        let assert = cmd
           .arg(&tc.query)
           .assert()
           .success();
           
        let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
        
        for expected in &tc.expected_contains {
            if !output.contains(expected) {
                panic!("Test {} failed. Expected output to contain '{}'\nOutput:\n{}", i, expected, output);
            }
        }
        
        for unexpected in &tc.expected_not_contains {
            if output.contains(unexpected) {
                panic!("Test {} failed. Expected output NOT to contain '{}'\nOutput:\n{}", i, unexpected, output);
            }
        }
    }
}
