use assert_cmd::Command;

#[test]
fn test_select_fields() {
    let mut cmd = Command::cargo_bin("fql").unwrap();
    let assert = cmd
       .arg("select name from tests/fixtures/test_dir")
       .assert()
       .success();
       
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(output.contains("name"));
    assert!(!output.contains("size"));
    assert!(!output.contains("path"));
    
    assert!(output.contains("file1.txt"));
    assert!(output.contains("file2.md"));
}

#[test]
fn test_select_multiple_fields() {
    let mut cmd = Command::cargo_bin("fql").unwrap();
    let assert = cmd
       .arg("select name, size from tests/fixtures/test_dir")
       .assert()
       .success();
       
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(output.contains("name"));
    assert!(output.contains("size"));
    assert!(!output.contains("path"));
    assert!(!output.contains("type"));
}

#[test]
fn test_select_where_type_f() {
    let mut cmd = Command::cargo_bin("fql").unwrap();
    let assert = cmd
       .arg("select name from tests/fixtures/test_dir where type = f")
       .assert()
       .success();
       
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(output.contains("file1.txt"));
    assert!(output.contains("file2.md"));
    assert!(!output.contains("sub_dir"));
}

#[test]
fn test_select_where_type_d() {
    let mut cmd = Command::cargo_bin("fql").unwrap();
    let assert = cmd
       .arg("select name from tests/fixtures/test_dir where type = d")
       .assert()
       .success();
       
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(output.contains("sub_dir"));
    assert!(!output.contains("file1.txt"));
}

#[test]
fn test_select_recursive() {
    let mut cmd = Command::cargo_bin("fql").unwrap();
    let assert = cmd
       .arg("select recursive name from tests/fixtures/test_dir")
       .assert()
       .success();
       
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(output.contains("file1.txt"));
    assert!(output.contains("file2.md"));
    assert!(output.contains("sub_dir"));
    assert!(output.contains("file3.rs"));
}

#[test]
fn test_select_json_output() {
    let mut cmd = Command::cargo_bin("fql").unwrap();
    let assert = cmd
       .arg("--output=json")
       .arg("select name, size from tests/fixtures/test_dir")
       .assert()
       .success();
       
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    let arr = json.as_array().unwrap();
    
    assert!(!arr.is_empty());
    
    for obj in arr {
        let map = obj.as_object().unwrap();
        assert!(map.contains_key("name"));
        assert!(map.contains_key("size"));
        assert!(!map.contains_key("path"));
        assert!(!map.contains_key("type"));
    }
}
