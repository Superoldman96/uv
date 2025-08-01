use anyhow::Result;
use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use indoc::indoc;
use predicates::prelude::*;
use uv_python::{PYTHON_VERSION_FILENAME, PYTHON_VERSIONS_FILENAME};
use uv_static::EnvVars;

#[cfg(unix)]
use fs_err::os::unix::fs::symlink;

use crate::common::{TestContext, uv_snapshot};

#[test]
fn create_venv() {
    let context = TestContext::new_with_versions(&["3.12"]);

    // Create a virtual environment at `.venv`.
    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());

    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--python")
        .arg("3.12"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    warning: A virtual environment already exists at `.venv`. In the future, uv will require `--clear` to replace it
    Activate with: source .venv/[BIN]/activate
    "
    );

    // Create a virtual environment at the same location using `--clear`,
    // which should replace it.
    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--clear")
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());
}

#[test]
fn create_venv_313() {
    let context = TestContext::new_with_versions(&["3.13"]);

    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--python")
        .arg("3.13"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.13.[X] interpreter at: [PYTHON-3.13]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());
}

#[test]
fn create_venv_project_environment() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.12"]);

    // `uv venv` ignores `UV_PROJECT_ENVIRONMENT` when it's not a project
    uv_snapshot!(context.filters(), context.venv().env(EnvVars::UV_PROJECT_ENVIRONMENT, "foo"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());
    context
        .temp_dir
        .child("foo")
        .assert(predicates::path::missing());

    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(
        r#"
            [project]
            name = "project"
            version = "0.1.0"
            requires-python = ">=3.12"
            dependencies = ["iniconfig"]
            "#,
    )?;

    // But, if we're in a project we'll respect it
    uv_snapshot!(context.filters(), context.venv().env(EnvVars::UV_PROJECT_ENVIRONMENT, "foo"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: foo
    Activate with: source foo/[BIN]/activate
    "###
    );

    context
        .temp_dir
        .child("foo")
        .assert(predicates::path::is_dir());

    // Unless we're in a child directory
    let child = context.temp_dir.child("child");
    child.create_dir_all()?;

    uv_snapshot!(context.filters(), context.venv().env(EnvVars::UV_PROJECT_ENVIRONMENT, "foo").current_dir(child.path()), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // In which case, we'll use the default name of `.venv`
    child.child("foo").assert(predicates::path::missing());
    child.child(".venv").assert(predicates::path::is_dir());

    // Or, if a name is provided
    uv_snapshot!(context.filters(), context.venv().arg("bar"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: bar
    Activate with: source bar/[BIN]/activate
    "###
    );

    context
        .temp_dir
        .child("bar")
        .assert(predicates::path::is_dir());

    // Or, of they opt-out with `--no-workspace` or `--no-project`
    uv_snapshot!(context.filters(), context.venv().arg("--clear").arg("--no-workspace"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    uv_snapshot!(context.filters(), context.venv().arg("--clear").arg("--no-project"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    Ok(())
}

#[test]
fn create_venv_defaults_to_cwd() {
    let context = TestContext::new_with_versions(&["3.12"]);
    uv_snapshot!(context.filters(), context.venv()
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());
}

#[test]
fn create_venv_ignores_virtual_env_variable() {
    let context = TestContext::new_with_versions(&["3.12"]);
    // We shouldn't care if `VIRTUAL_ENV` is set to an non-existent directory
    // because we ignore virtual environment interpreter sources (we require a system interpreter)
    uv_snapshot!(context.filters(), context.venv()
        .env(EnvVars::VIRTUAL_ENV, context.temp_dir.child("does-not-exist").as_os_str()), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );
}

#[test]
fn create_venv_reads_request_from_python_version_file() {
    let context = TestContext::new_with_versions(&["3.11", "3.12"]);

    // Without the file, we should use the first on the PATH
    uv_snapshot!(context.filters(), context.venv(), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // With a version file, we should prefer that version
    context
        .temp_dir
        .child(PYTHON_VERSION_FILENAME)
        .write_str("3.12")
        .unwrap();

    uv_snapshot!(context.filters(), context.venv().arg("--clear"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());
}

#[test]
fn create_venv_reads_request_from_python_versions_file() {
    let context = TestContext::new_with_versions(&["3.11", "3.12"]);

    // Without the file, we should use the first on the PATH
    uv_snapshot!(context.filters(), context.venv(), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // With a versions file, we should prefer the first listed version
    context
        .temp_dir
        .child(PYTHON_VERSIONS_FILENAME)
        .write_str("3.12\n3.11")
        .unwrap();

    uv_snapshot!(context.filters(), context.venv().arg("--clear"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());
}

#[test]
fn create_venv_respects_pyproject_requires_python() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.11", "3.9", "3.10", "3.12"]);

    // Without a Python requirement, we use the first on the PATH
    uv_snapshot!(context.filters(), context.venv(), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // With `requires-python = "<3.11"`, we prefer the first available version
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        requires-python = "<3.11"
        dependencies = []
        "#
    })?;

    uv_snapshot!(context.filters(), context.venv().arg("--clear"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.9.[X] interpreter at: [PYTHON-3.9]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // With `requires-python = "==3.11.*"`, we prefer exact version (3.11)
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        requires-python = "==3.11.*"
        dependencies = []
        "#
    })?;

    uv_snapshot!(context.filters(), context.venv().arg("--clear"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // With `requires-python = ">=3.11,<3.12"`, we prefer exact version (3.11)
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        requires-python = ">=3.11,<3.12"
        dependencies = []
        "#
    })?;

    uv_snapshot!(context.filters(), context.venv().arg("--clear"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // With `requires-python = ">=3.10"`, we prefer first compatible version (3.11)
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        requires-python = ">=3.11"
        dependencies = []
        "#
    })?;

    // With `requires-python = ">=3.11"`, we prefer first compatible version (3.11)
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        requires-python = ">=3.11"
        dependencies = []
        "#
    })?;

    uv_snapshot!(context.filters(), context.venv().arg("--clear"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // With `requires-python = ">3.11"`, we prefer first compatible version (3.11)
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        requires-python = ">3.11"
        dependencies = []
        "#
    })?;

    uv_snapshot!(context.filters(), context.venv().arg("--clear"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // With `requires-python = ">=3.12"`, we prefer first compatible version (3.12)
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        requires-python = ">=3.12"
        dependencies = []
        "#
    })?;

    uv_snapshot!(context.filters(), context.venv().arg("--clear"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());

    // We warn if we receive an incompatible version
    uv_snapshot!(context.filters(), context.venv().arg("--clear").arg("--python").arg("3.11"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    warning: The requested interpreter resolved to Python 3.11.[X], which is incompatible with the project's Python requirement: `>=3.12` (from `project.requires-python`)
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "
    );

    Ok(())
}

#[test]
fn create_venv_respects_group_requires_python() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.9", "3.10", "3.11", "3.12"]);

    // Without a Python requirement, we use the first on the PATH
    uv_snapshot!(context.filters(), context.venv(), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.9.[X] interpreter at: [PYTHON-3.9]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "
    );

    // With `requires-python = ">=3.10"` on the default group, we pick 3.10
    // However non-default groups should not be consulted!
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        dependencies = []

        [dependency-groups]
        dev = ["sortedcontainers"]
        other = ["sniffio"]

        [tool.uv.dependency-groups]
        dev = {requires-python = ">=3.10"}
        other = {requires-python = ">=3.12"}
        "#
    })?;

    uv_snapshot!(context.filters(), context.venv().arg("--clear"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.10.[X] interpreter at: [PYTHON-3.10]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "
    );

    // When the top-level requires-python and default group requires-python
    // both apply, their intersection is used. However non-default groups
    // should not be consulted! (here the top-level wins)
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        requires-python = ">=3.11"
        dependencies = []

        [dependency-groups]
        dev = ["sortedcontainers"]
        other = ["sniffio"]

        [tool.uv.dependency-groups]
        dev = {requires-python = ">=3.10"}
        other = {requires-python = ">=3.12"}
        "#
    })?;

    uv_snapshot!(context.filters(), context.venv().arg("--clear"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "
    );

    // When the top-level requires-python and default group requires-python
    // both apply, their intersection is used. However non-default groups
    // should not be consulted! (here the group wins)
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        requires-python = ">=3.10"
        dependencies = []

        [dependency-groups]
        dev = ["sortedcontainers"]
        other = ["sniffio"]

        [tool.uv.dependency-groups]
        dev = {requires-python = ">=3.11"}
        other = {requires-python = ">=3.12"}
        "#
    })?;

    uv_snapshot!(context.filters(), context.venv().arg("--clear"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "
    );

    // We warn if we receive an incompatible version
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        dependencies = []

        [dependency-groups]
        dev = ["sortedcontainers"]

        [tool.uv.dependency-groups]
        dev = {requires-python = ">=3.12"}
        "#
    })?;

    uv_snapshot!(context.filters(), context.venv().arg("--clear").arg("--python").arg("3.11"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    warning: The requested interpreter resolved to Python 3.11.[X], which is incompatible with the project's Python requirement: `>=3.12` (from `tool.uv.dependency-groups.dev.requires-python`).
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "
    );

    // We error if there's no compatible version
    // non-default groups are not consulted here!
    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r#"
        [project]
        name = "foo"
        version = "1.0.0"
        requires-python = "<3.12"
        dependencies = []

        [dependency-groups]
        dev = ["sortedcontainers"]
        other = ["sniffio"]

        [tool.uv.dependency-groups]
        dev = {requires-python = ">=3.12"}
        other = {requires-python = ">=3.11"}
        "#
    })?;

    uv_snapshot!(context.filters(), context.venv().arg("--clear").arg("--python").arg("3.11"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Found conflicting Python requirements:
    - foo: <3.12
    - foo:dev: >=3.12
    "
    );

    Ok(())
}

#[test]
fn create_venv_ignores_missing_pyproject_metadata() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.12"]);

    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r"[tool.no.project.here]" })?;

    uv_snapshot!(context.filters(), context.venv(), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());

    Ok(())
}

#[test]
fn create_venv_warns_user_on_requires_python_discovery_error() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.12"]);

    let pyproject_toml = context.temp_dir.child("pyproject.toml");
    pyproject_toml.write_str(indoc! { r"invalid toml" })?;

    uv_snapshot!(context.filters(), context.venv(), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    warning: Failed to parse `pyproject.toml` during settings discovery:
      TOML parse error at line 1, column 9
        |
      1 | invalid toml
        |         ^
      key with no value, expected `=`

    warning: Failed to parse `pyproject.toml` during environment creation:
      TOML parse error at line 1, column 9
        |
      1 | invalid toml
        |         ^
      key with no value, expected `=`

    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "
    );

    context.venv.assert(predicates::path::is_dir());

    Ok(())
}

#[test]
fn create_venv_explicit_request_takes_priority_over_python_version_file() {
    let context = TestContext::new_with_versions(&["3.11", "3.12"]);

    context
        .temp_dir
        .child(PYTHON_VERSION_FILENAME)
        .write_str("3.12")
        .unwrap();

    uv_snapshot!(context.filters(), context.venv().arg("--python").arg("3.11"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());
}

#[test]
#[cfg(feature = "pypi")]
fn seed() {
    let context = TestContext::new_with_versions(&["3.12"]);
    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--seed")
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment with seed packages at: .venv
     + pip==24.0
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());
}

#[test]
#[cfg(feature = "pypi")]
fn seed_older_python_version() {
    let context = TestContext::new_with_versions(&["3.11"]);
    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--seed")
        .arg("--python")
        .arg("3.11"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment with seed packages at: .venv
     + pip==24.0
     + setuptools==69.2.0
     + wheel==0.43.0
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());
}

#[test]
fn create_venv_unknown_python_minor() {
    let context = TestContext::new_with_versions(&["3.12"]).with_filtered_python_sources();

    let mut command = context.venv();
    command
        .arg(context.venv.as_os_str())
        // Request a version we know we'll never see
        .arg("--python")
        .arg("3.100")
        // Unset this variable to force what the user would see
        .env_remove(EnvVars::UV_TEST_PYTHON_PATH);

    uv_snapshot!(context.filters(), &mut command, @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: No interpreter found for Python 3.100 in [PYTHON SOURCES]
    "
    );

    context.venv.assert(predicates::path::missing());
}

#[test]
fn create_venv_unknown_python_patch() {
    let context = TestContext::new_with_versions(&["3.12"]).with_filtered_python_sources();

    let mut command = context.venv();
    command
        .arg(context.venv.as_os_str())
        // Request a version we know we'll never see
        .arg("--python")
        .arg("3.12.100")
        // Unset this variable to force what the user would see
        .env_remove(EnvVars::UV_TEST_PYTHON_PATH);

    uv_snapshot!(context.filters(), &mut command, @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: No interpreter found for Python 3.12.[X] in [PYTHON SOURCES]
    "
    );

    context.venv.assert(predicates::path::missing());
}

#[cfg(feature = "python-patch")]
#[test]
fn create_venv_python_patch() {
    let context = TestContext::new_with_versions(&["3.12.9"]);

    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--python")
        .arg("3.12.9"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.9 interpreter at: [PYTHON-3.12.9]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "
    );

    context.venv.assert(predicates::path::is_dir());
}

#[test]
fn file_exists() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.12"]);

    // Create a file at `.venv`. Creating a virtualenv at the same path should fail.
    context.venv.touch()?;

    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--python")
        .arg("3.12"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    error: Failed to create virtual environment
      Caused by: File exists at `.venv`
    "
    );

    Ok(())
}

#[test]
fn empty_dir_exists() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.12"]);

    // Create an empty directory at `.venv`. Creating a virtualenv at the same path should succeed.
    context.venv.create_dir_all()?;
    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());

    Ok(())
}

#[test]
fn non_empty_dir_exists() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.12"]);

    // Create a non-empty directory at `.venv`. Creating a virtualenv at the same path should fail.
    context.venv.create_dir_all()?;
    context.venv.child("file").touch()?;

    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--python")
        .arg("3.12"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    warning: A directory already exists at `.venv`. In the future, uv will require `--clear` to replace it
    Activate with: source .venv/[BIN]/activate
    "
    );

    Ok(())
}

#[test]
fn non_empty_dir_exists_allow_existing() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.12"]);

    // Create a non-empty directory at `.venv`. Creating a virtualenv at the same path should
    // succeed when `--allow-existing` is specified, but fail when it is not.
    context.venv.create_dir_all()?;
    context.venv.child("file").touch()?;

    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--python")
        .arg("3.12"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    warning: A directory already exists at `.venv`. In the future, uv will require `--clear` to replace it
    Activate with: source .venv/[BIN]/activate
    "
    );

    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--allow-existing")
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // Running again should _also_ succeed, overwriting existing symlinks and respecting existing
    // directories.
    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .arg("--allow-existing")
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    Ok(())
}

/// Run `uv venv` followed by `uv venv --allow-existing`.
#[test]
fn create_venv_then_allow_existing() {
    let context = TestContext::new_with_versions(&["3.12"]);

    // Create a venv
    uv_snapshot!(context.filters(), context.venv(), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "
    );

    // Create a venv again with `--allow-existing`
    uv_snapshot!(context.filters(), context.venv()
        .arg("--allow-existing"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );
}

#[test]
#[cfg(windows)]
fn windows_shims() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.10", "3.9"]);
    let shim_path = context.temp_dir.child("shim");

    let py39 = context
        .python_versions
        .last()
        .expect("python_path_with_versions to set up the python versions");

    // We want 3.9 and the first version should be 3.10.
    // Picking the last is necessary to prove that shims work because the python version selects
    // the python version from the first path segment by default, so we take the last to prove it's not
    // returning that version.
    assert!(py39.0.to_string().contains("3.9"));

    // Write the shim script that forwards the arguments to the python3.9 installation.
    fs_err::create_dir(&shim_path)?;
    fs_err::write(
        shim_path.child("python.bat"),
        format!(
            "@echo off\r\n{}/python.exe %*",
            py39.1.parent().unwrap().display()
        ),
    )?;

    // Create a virtual environment at `.venv` with the shim
    uv_snapshot!(context.filters(), context.venv()
        .arg(context.venv.as_os_str())
        .env(EnvVars::UV_TEST_PYTHON_PATH, format!("{};{}", shim_path.display(), context.python_path().to_string_lossy())), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.9.[X] interpreter at: [PYTHON-3.9]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    context.venv.assert(predicates::path::is_dir());

    Ok(())
}

#[test]
fn verify_pyvenv_cfg() {
    let context = TestContext::new("3.12");
    let pyvenv_cfg = context.venv.child("pyvenv.cfg");

    context.venv.assert(predicates::path::is_dir());

    // Check pyvenv.cfg exists
    pyvenv_cfg.assert(predicates::path::is_file());

    // Check if "uv = version" is present in the file
    let version = env!("CARGO_PKG_VERSION").to_string();
    let search_string = format!("uv = {version}");
    pyvenv_cfg.assert(predicates::str::contains(search_string));

    // Not relocatable by default.
    pyvenv_cfg.assert(predicates::str::contains("relocatable").not());
}

#[test]
fn verify_pyvenv_cfg_relocatable() {
    let context = TestContext::new("3.12");

    // Create a virtual environment at `.venv`.
    context
        .venv()
        .arg(context.venv.as_os_str())
        .arg("--clear")
        .arg("--python")
        .arg("3.12")
        .arg("--relocatable")
        .assert()
        .success();

    let pyvenv_cfg = context.venv.child("pyvenv.cfg");

    context.venv.assert(predicates::path::is_dir());

    // Check pyvenv.cfg exists
    pyvenv_cfg.assert(predicates::path::is_file());

    // Relocatable flag is set.
    pyvenv_cfg.assert(predicates::str::contains("relocatable = true"));

    // Activate scripts contain the relocatable boilerplate
    let scripts = if cfg!(windows) {
        context.venv.child("Scripts")
    } else {
        context.venv.child("bin")
    };

    let activate_sh = scripts.child("activate");
    activate_sh.assert(predicates::path::is_file());
    activate_sh.assert(predicates::str::contains(
        r#"VIRTUAL_ENV=''"$(dirname -- "$(dirname -- "$(realpath -- "$SCRIPT_PATH")")")"''"#,
    ));

    let activate_bat = scripts.child("activate.bat");
    activate_bat.assert(predicates::path::is_file());
    activate_bat.assert(predicates::str::contains(
        r#"@for %%i in ("%~dp0..") do @set "VIRTUAL_ENV=%%~fi""#,
    ));

    let activate_fish = scripts.child("activate.fish");
    activate_fish.assert(predicates::path::is_file());
    activate_fish.assert(predicates::str::contains(r#"set -gx VIRTUAL_ENV ''"$(dirname -- "$(cd "$(dirname -- "$(status -f)")"; and pwd)")"''"#));
}

/// Ensure that a nested virtual environment uses the same `home` directory as the parent.
#[test]
fn verify_nested_pyvenv_cfg() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.12"]);

    // Create a virtual environment at `.venv`.
    context
        .venv()
        .arg(context.venv.as_os_str())
        .arg("--python")
        .arg("3.12")
        .assert()
        .success();

    let pyvenv_cfg = context.venv.child("pyvenv.cfg");

    // Check pyvenv.cfg exists
    pyvenv_cfg.assert(predicates::path::is_file());

    // Extract the "home" line from the pyvenv.cfg file.
    let contents = fs_err::read_to_string(pyvenv_cfg.path())?;
    let venv_home = contents
        .lines()
        .find(|line| line.starts_with("home"))
        .expect("home line not found");

    // Now, create a virtual environment from within the virtual environment.
    let subvenv = context.temp_dir.child(".subvenv");
    context
        .venv()
        .arg(subvenv.as_os_str())
        .arg("--python")
        .arg("3.12")
        .env(EnvVars::VIRTUAL_ENV, context.venv.as_os_str())
        .assert()
        .success();

    let sub_pyvenv_cfg = subvenv.child("pyvenv.cfg");

    // Extract the "home" line from the pyvenv.cfg file.
    let contents = fs_err::read_to_string(sub_pyvenv_cfg.path())?;
    let sub_venv_home = contents
        .lines()
        .find(|line| line.starts_with("home"))
        .expect("home line not found");

    // Check that both directories point to the same home.
    assert_eq!(sub_venv_home, venv_home);

    Ok(())
}

/// See <https://github.com/astral-sh/uv/issues/3280>
#[test]
#[cfg(windows)]
fn path_with_trailing_space_gives_proper_error() {
    let context = TestContext::new_with_versions(&["3.12"]);

    // Set a custom cache directory with a trailing space
    let path_with_trailing_slash = format!("{} ", context.cache_dir.path().display());
    let mut filters = context.filters();
    // Windows translates error messages, for example i get:
    // ": Das System kann den angegebenen Pfad nicht finden. (os error 3)"
    filters.push((
        r"CACHEDIR.TAG`: .* \(os error 3\)",
        "CACHEDIR.TAG`: The system cannot find the path specified. (os error 3)",
    ));
    uv_snapshot!(filters, std::process::Command::new(crate::common::get_bin())
        .arg("venv")
        .env(EnvVars::UV_CACHE_DIR, path_with_trailing_slash), @r###"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: failed to open file `[CACHE_DIR]/ /CACHEDIR.TAG`: The system cannot find the path specified. (os error 3)
    "###
    );
    // Note the extra trailing `/` in the snapshot is due to the filters, not the actual output.
}

/// Check that the activate script still works with the path contains an apostrophe.
#[test]
#[cfg(target_os = "linux")]
fn create_venv_apostrophe() {
    use std::env;
    use std::ffi::OsString;
    use std::io::Write;
    use std::process::Command;
    use std::process::Stdio;

    let context = TestContext::new_with_versions(&["3.12"]);

    let venv_dir = context.temp_dir.join("Testing's");

    uv_snapshot!(context.filters(), context.venv()
        .arg(&venv_dir)
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: Testing's
    Activate with: source Testing's/[BIN]/activate
    "###
    );

    // One of them should be commonly available on a linux developer machine, if not, we have to
    // extend the fallbacks.
    let shell = env::var_os(EnvVars::SHELL).unwrap_or(OsString::from("bash"));
    let mut child = Command::new(shell)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .current_dir(&venv_dir)
        .spawn()
        .expect("Failed to spawn shell script");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    std::thread::spawn(move || {
        stdin
            .write_all(". bin/activate && python -c 'import sys; print(sys.prefix)'".as_bytes())
            .expect("Failed to write to stdin");
    });

    let output = child.wait_with_output().expect("Failed to read stdout");

    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), venv_dir.to_string_lossy());
}

#[test]
fn venv_python_preference() {
    let context =
        TestContext::new_with_versions(&["3.12", "3.11"]).with_versions_as_managed(&["3.12"]);

    // Create a managed interpreter environment
    uv_snapshot!(context.filters(), context.venv(), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    ");

    uv_snapshot!(context.filters(), context.venv().arg("--no-managed-python"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    warning: A virtual environment already exists at `.venv`. In the future, uv will require `--clear` to replace it
    Activate with: source .venv/[BIN]/activate
    ");

    uv_snapshot!(context.filters(), context.venv().arg("--no-managed-python"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.11.[X] interpreter at: [PYTHON-3.11]
    Creating virtual environment at: .venv
    warning: A virtual environment already exists at `.venv`. In the future, uv will require `--clear` to replace it
    Activate with: source .venv/[BIN]/activate
    ");

    uv_snapshot!(context.filters(), context.venv(), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    warning: A virtual environment already exists at `.venv`. In the future, uv will require `--clear` to replace it
    Activate with: source .venv/[BIN]/activate
    ");

    uv_snapshot!(context.filters(), context.venv().arg("--managed-python"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    warning: A virtual environment already exists at `.venv`. In the future, uv will require `--clear` to replace it
    Activate with: source .venv/[BIN]/activate
    ");
}

#[test]
#[cfg(unix)]
fn create_venv_symlink_clear_preservation() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.12"]);

    // Create a target directory
    let target_dir = context.temp_dir.child("target");
    target_dir.create_dir_all()?;

    // Create a symlink pointing to the target directory
    let symlink_path = context.temp_dir.child(".venv");
    symlink(&target_dir, &symlink_path)?;

    // Verify symlink exists
    assert!(symlink_path.path().is_symlink());

    // Create virtual environment at symlink location
    uv_snapshot!(context.filters(), context.venv()
        .arg(symlink_path.as_os_str())
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // Verify symlink is still preserved after creation
    assert!(symlink_path.path().is_symlink());

    // Run uv venv with --clear to test symlink preservation during clear
    uv_snapshot!(context.filters(), context.venv()
        .arg(symlink_path.as_os_str())
        .arg("--clear")
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // Verify symlink is STILL preserved after --clear
    assert!(symlink_path.path().is_symlink());

    Ok(())
}

#[test]
#[cfg(unix)]
fn create_venv_symlink_recreate_preservation() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.12"]);

    // Create a target directory
    let target_dir = context.temp_dir.child("target");
    target_dir.create_dir_all()?;

    // Create a symlink pointing to the target directory
    let symlink_path = context.temp_dir.child(".venv");
    symlink(&target_dir, &symlink_path)?;

    // Verify symlink exists
    assert!(symlink_path.path().is_symlink());

    // Create virtual environment at symlink location
    uv_snapshot!(context.filters(), context.venv()
        .arg(symlink_path.as_os_str())
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // Verify symlink is preserved after first creation
    assert!(symlink_path.path().is_symlink());

    // Run uv venv again WITHOUT --clear to test recreation behavior
    uv_snapshot!(context.filters(), context.venv()
        .arg(symlink_path.as_os_str())
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    warning: A virtual environment already exists at `.venv`. In the future, uv will require `--clear` to replace it
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // Verify symlink is STILL preserved after recreation
    assert!(symlink_path.path().is_symlink());

    Ok(())
}

#[test]
#[cfg(unix)]
fn create_venv_nested_symlink_preservation() -> Result<()> {
    let context = TestContext::new_with_versions(&["3.12"]);

    // Create a target directory
    let target_dir = context.temp_dir.child("target");
    target_dir.create_dir_all()?;

    // Create first symlink level: intermediate -> target
    let intermediate_link = context.temp_dir.child("intermediate");
    symlink(&target_dir, &intermediate_link)?;

    // Create second symlink level: .venv -> intermediate (nested symlink)
    let symlink_path = context.temp_dir.child(".venv");
    symlink(&intermediate_link, &symlink_path)?;

    // Verify nested symlink exists
    assert!(symlink_path.path().is_symlink());
    assert!(intermediate_link.path().is_symlink());

    // Create virtual environment at nested symlink location
    uv_snapshot!(context.filters(), context.venv()
        .arg(symlink_path.as_os_str())
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // Verify both symlinks are preserved
    assert!(symlink_path.path().is_symlink());
    assert!(intermediate_link.path().is_symlink());

    // Run uv venv again to test nested symlink preservation during recreation
    uv_snapshot!(context.filters(), context.venv()
        .arg(symlink_path.as_os_str())
        .arg("--python")
        .arg("3.12"), @r###"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    Using CPython 3.12.[X] interpreter at: [PYTHON-3.12]
    Creating virtual environment at: .venv
    warning: A virtual environment already exists at `.venv`. In the future, uv will require `--clear` to replace it
    Activate with: source .venv/[BIN]/activate
    "###
    );

    // Verify nested symlinks are STILL preserved
    assert!(symlink_path.path().is_symlink());
    assert!(intermediate_link.path().is_symlink());

    Ok(())
}
