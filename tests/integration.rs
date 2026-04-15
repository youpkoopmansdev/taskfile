use std::fs;
use std::process::Command;

fn task_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("task");
    path
}

fn setup_taskfile(dir: &std::path::Path, content: &str) {
    fs::write(dir.join("Taskfile"), content).unwrap();
}

#[test]
fn cli_runs_simple_task() {
    let tmp = tempfile::tempdir().unwrap();
    setup_taskfile(
        tmp.path(),
        r#"task hello {
  echo "Hello from task"
}"#,
    );

    let output = Command::new(task_bin())
        .arg("hello")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from task"));
}

#[test]
fn cli_list_shows_tasks() {
    let tmp = tempfile::tempdir().unwrap();
    setup_taskfile(
        tmp.path(),
        r#"@description Build the project
task build {
  echo "building"
}

@description Run tests
task test {
  echo "testing"
}"#,
    );

    let output = Command::new(task_bin())
        .arg("--list")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("build"));
    assert!(stdout.contains("Build the project"));
    assert!(stdout.contains("test"));
    assert!(stdout.contains("Run tests"));
}

#[test]
fn cli_task_with_params() {
    let tmp = tempfile::tempdir().unwrap();
    setup_taskfile(
        tmp.path(),
        r#"task greet [name="world"] {
  echo "Hello, $name!"
}"#,
    );

    let output = Command::new(task_bin())
        .args(["greet", "--", "--name=Rust"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello, Rust!"));
}

#[test]
fn cli_task_with_default_param() {
    let tmp = tempfile::tempdir().unwrap();
    setup_taskfile(
        tmp.path(),
        r#"task greet [name="world"] {
  echo "Hello, $name!"
}"#,
    );

    let output = Command::new(task_bin())
        .arg("greet")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello, world!"));
}

#[test]
fn cli_missing_required_param() {
    let tmp = tempfile::tempdir().unwrap();
    setup_taskfile(
        tmp.path(),
        r#"task deploy [env] {
  echo "$env"
}"#,
    );

    let output = Command::new(task_bin())
        .arg("deploy")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing required parameter"));
}

#[test]
fn cli_unknown_task_suggests() {
    let tmp = tempfile::tempdir().unwrap();
    setup_taskfile(
        tmp.path(),
        r#"task build {
  echo "building"
}

task test {
  echo "testing"
}"#,
    );

    let output = Command::new(task_bin())
        .arg("buil")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown task"));
    assert!(stderr.contains("build"));
}

#[test]
fn cli_task_with_dependencies() {
    let tmp = tempfile::tempdir().unwrap();
    setup_taskfile(
        tmp.path(),
        r#"task clean {
  echo "cleaning"
}

task build depends=[clean] {
  echo "building"
}"#,
    );

    let output = Command::new(task_bin())
        .arg("build")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cleaning"));
    assert!(stdout.contains("building"));
}

#[test]
fn cli_exports_injected() {
    let tmp = tempfile::tempdir().unwrap();
    setup_taskfile(
        tmp.path(),
        r#"export PROJECT="myapp"

task info {
  echo "Project: $PROJECT"
}"#,
    );

    let output = Command::new(task_bin())
        .arg("info")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Project: myapp"));
}

#[test]
fn cli_aliases_as_functions() {
    let tmp = tempfile::tempdir().unwrap();
    setup_taskfile(
        tmp.path(),
        r#"alias greet="echo Hello"

task hi {
  greet World
}"#,
    );

    let output = Command::new(task_bin())
        .arg("hi")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello World"));
}

#[test]
fn cli_namespaced_tasks() {
    let tmp = tempfile::tempdir().unwrap();
    let tasks_dir = tmp.path().join("tasks");
    fs::create_dir(&tasks_dir).unwrap();

    setup_taskfile(
        tmp.path(),
        r#"include "tasks/docker.Taskfile"

task build {
  echo "building"
}"#,
    );

    fs::write(
        tasks_dir.join("docker.Taskfile"),
        r#"task up {
  echo "docker up"
}"#,
    )
    .unwrap();

    // Test listing
    let output = Command::new(task_bin())
        .arg("--list")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("docker:up"));

    // Test running namespaced task
    let output = Command::new(task_bin())
        .arg("docker:up")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("docker up"));
}

#[test]
fn cli_no_taskfile_error() {
    let tmp = tempfile::tempdir().unwrap();
    let deep = tmp.path().join("a/b/c");
    fs::create_dir_all(&deep).unwrap();

    let output = Command::new(task_bin())
        .arg("build")
        .current_dir(&deep)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No Taskfile found"));
}

#[test]
fn cli_task_exit_code_propagated() {
    let tmp = tempfile::tempdir().unwrap();
    setup_taskfile(
        tmp.path(),
        r#"task fail {
  exit 42
}"#,
    );

    let output = Command::new(task_bin())
        .arg("fail")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(42));
}

#[test]
fn cli_inherited_exports_in_namespaced_tasks() {
    let tmp = tempfile::tempdir().unwrap();
    let tasks_dir = tmp.path().join("tasks");
    fs::create_dir(&tasks_dir).unwrap();

    setup_taskfile(
        tmp.path(),
        r#"export PROJECT="myapp"
include "tasks/deploy.Taskfile""#,
    );

    fs::write(
        tasks_dir.join("deploy.Taskfile"),
        r#"task staging {
  echo "Deploying $PROJECT"
}"#,
    )
    .unwrap();

    let output = Command::new(task_bin())
        .arg("deploy:staging")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Deploying myapp"));
}
