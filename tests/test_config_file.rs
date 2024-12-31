
use std::fs::write;
use tempfile::NamedTempFile;



use rattomail::{
                read_config_ini,
                Config,
               };


#[test]
fn test_read_config_ini_success() {
  let temp_file = NamedTempFile::new().unwrap();
  let file_path = temp_file.path();
  let conts = r#"
mailDir = /home/user/Maildir/new
userName = user
"#;

  // Write some test content to the file
  write(file_path, conts).unwrap();

  // Call the function you're testing
  let config = read_config_ini(file_path).unwrap();
  let expected = Config {
    mailDir: "/home/user/Maildir/new".to_string(),
    userName: "user".to_string(),
  };

  assert_eq!(expected, config, "config file conts does not equal what was written");
}

#[test]
fn test_read_config_ini_no_such_file() {
  let invalid_path = "non_existent_file.ini";
  let result = read_config_ini(invalid_path);

  assert!(result.is_err(), "Expected an error, but got: {:?}", result);
}

#[test]
fn test_read_config_ini_malformed_file() {
  let temp_file = NamedTempFile::new().unwrap();
  let file_path = temp_file.path();
  let conts = r#"
mailDir /home/user/Maildir/new
userName = user
"#;

  write(file_path, conts).unwrap();

  // Call the function you're testing
  let result = read_config_ini(file_path);

  assert!(result.is_err(), "Expected an error, but got: {:?}", result);
}
