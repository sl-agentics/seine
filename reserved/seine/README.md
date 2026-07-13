# seine (name reservation)

This crate name is reserved for **Seine** — a differentially certified
Drools-subset rule engine over Arrow dataframes, developed at
<https://github.com/sl-agentics/seine>.

Seine currently ships as the Python package
[`seine-rs`](https://pypi.org/project/seine-rs/) (`pip install seine-rs`,
`import seine_rs`), whose native core is this repository's Rust engine.
A public Rust crate API has not been designed yet; when it is, it will
live under this name.

Reserved rather than squatted: the project is active, the repository is
public, and the engine this name is held for already exists — it is the
`seine-engine` crate in the repository above, differentially certified
against Drools 9.44.0.Final (interrogable from the Python package via
`seine_rs.certification()`).
