# Release Procedure

## Housekeeping

1) Review all changes.
    * Run rust test with `cargo test`
    * Run python test with `pytest`
    * Proof rust docs with `cargo doc --open`
    * Proof python docs with `pydoc -b npc_maker`

2) Bump npc_maker version number. All language implementations will release the
same version simultaneously.
    * Rust version number is in: `rust/Cargo.toml`
    * Python version number is in: `python/pyproject.toml`

3) Commit all changes. Only the examples directory may have local modifications.

## Rust

4) Get an API token for crates.io
    * Login to https://crates.io
    * Go to "Account Settings"
    * Go to "API Tokens"
    * Follow instructions to generate a new token
    * Run `cargo login` to activate the token

5) Publish to rust API to crates.io
    * `cargo publish --dry-run`

6) Verify the npc_maker's page on https://crates.io

## Python

See the official tutorials at: https://packaging.python.org/en/latest/

7) Get an API token for pypi.org
    * Login to https://pypi.org
    * Go to "Account Settings"
    * Go to "API Tokens"
    * Follow instructions to generate and activate a new token

8) Update python toolchain with `python3 -m pip install --upgrade pip build twine`

9) Build and upload the python package
    * Build source and wheel distributions with `python3 -m build`
    * Check the results with `python3 -m twine check dist/*`
    * Upload the results with `python3 -m twine upload dist/*`

10) Verify the npc_maker's page on https://pypi.org
